import { readFile } from 'node:fs/promises';
import { parentPort, threadId, workerData } from 'node:worker_threads';

import * as runtime from '@epa-wg/cem-ml/wasm';

import type {
    BrowserCommandActionResultMessage,
    BrowserCommandArtifactDisposeRequest,
    BrowserCommandArtifactReadRequest,
    BrowserCommandArtifactsDisposeRequest,
    BrowserCommandCancelRequest,
    BrowserCommandCapabilityName,
    BrowserCommandCapabilityRequest,
    BrowserCommandCapabilityResponse,
    BrowserCommandExecuteRequest,
    BrowserCommandProgressMessage,
    BrowserCommandResultMessage,
    NodeWorkerBootstrap,
    NodeWorkerCapabilityManifest,
    NodeWorkerInitializeEnvelope,
    WorkerProtocolDescriptor,
    WorkerWorkRequest,
    WorkerWorkResultEnvelope,
} from './protocol.js';
import type { ResumableRuntime } from './operation.js';

if (parentPort === null) {
    throw new Error('CEM-ML Node worker host requires a worker-thread parent port');
}
const port = parentPort;

const bootstrap = parseBootstrap(workerData as unknown);
const runtimeMetadataUrl = new URL(import.meta.resolve('@epa-wg/cem-ml/runtime.json'));
const runtimeMetadata = JSON.parse(await readFile(runtimeMetadataUrl, 'utf8')) as unknown;
if (!isRecord(runtimeMetadata) || !isRecord(runtimeMetadata.abi)) {
    throw new Error('CEM-ML runtime metadata is invalid');
}
const abiIdentity = runtimeMetadata.abi.identity;
if (typeof abiIdentity !== 'string' || abiIdentity.length === 0) {
    throw new Error('CEM-ML runtime metadata does not provide an ABI identity');
}

const protocol = JSON.parse(runtime.workerProtocolDescriptor()) as WorkerProtocolDescriptor;
const capabilityRequest = JSON.stringify({
    runtime: 'wasm-node',
    targetIdentity: 'wasm32-unknown-unknown:nodejs',
    abiIdentity,
    debugControlActive: false,
});
const capability = JSON.parse(
    runtime.nodeWorkerCapabilityManifest(capabilityRequest, bootstrap.effectiveWorkers),
) as NodeWorkerCapabilityManifest & { readonly error?: unknown };
if (capability.error !== undefined) {
    throw new Error('CEM-ML Node worker capability projection failed');
}

const commonVersion = runtime.version();
const envelope: NodeWorkerInitializeEnvelope = {
    workerProtocolVersion: protocol.workerProtocolVersion,
    worker: bootstrap.worker,
    sequence: 1,
    operation: {
        protocolVersion: protocol.operationProtocolVersion,
        kind: 'initialize',
        payload: {
            runtimeInstanceId: `node-thread-${threadId}:slot-${bootstrap.worker.slot}:generation-${bootstrap.worker.generation}`,
            threadId,
            commonVersion,
            protocol,
            capability,
        },
    },
    transfers: [],
};

port.postMessage(envelope);
let nextSequence = 2;
let nextCallbackId = 1;
const pendingCapabilities = new Map<
    number,
    { readonly resolve: (value: unknown) => void; readonly reject: (error: Error) => void }
>();
port.on('message', (message: unknown) => {
    if (isRecord(message) && message.type === 'cem-worker-close') {
        port.close();
        return;
    }
    if (isWorkRequest(message)) {
        const packet = message.packet;
        if (
            packet.worker.slot !== bootstrap.worker.slot ||
            packet.worker.generation !== bootstrap.worker.generation
        ) {
            throw new Error('CEM-ML work packet targets a different worker generation');
        }
        const result = parseRuntimeResult(
            (runtime as unknown as ResumableRuntime).executeOperationWork(JSON.stringify(packet)),
        );
        const response: WorkerWorkResultEnvelope = {
            workerProtocolVersion: protocol.workerProtocolVersion,
            worker: bootstrap.worker,
            sequence: nextSequence++,
            operation: {
                protocolVersion: protocol.operationProtocolVersion,
                kind: 'result',
                operationId: packet.operationId,
                sequence: packet.taskId,
                payload: result,
            },
            transfers: [],
        };
        port.postMessage(response);
        return;
    }
    if (isCommandExecuteRequest(message)) {
        void executeCommand(message).catch(reportFailure);
        return;
    }
    if (isCommandActionRequest(message)) {
        executeCommandAction(message);
        return;
    }
    if (isCommandCapabilityResponse(message)) {
        acceptCapabilityResponse(message);
    }
});

async function executeCommand(message: BrowserCommandExecuteRequest): Promise<void> {
    const request = JSON.stringify({
        runtime: capability.runtime,
        targetIdentity: capability.targetIdentity,
        abiIdentity: capability.abiIdentity,
        debugControlActive: false,
    });
    const bridge = (name: BrowserCommandCapabilityName, argument: unknown, bytes?: Uint8Array) =>
        requestCapability(message.executionId, name, argument, bytes);
    const resultJson = await runtime.executeCommandServiceV1(
        JSON.stringify(message.request),
        request,
        async (json) => JSON.stringify(await bridge('currentRevision', JSON.parse(json))),
        async (json) => JSON.stringify(await bridge('readResource', JSON.parse(json))),
        async (json, bytes) => JSON.stringify(await bridge('prepareWrite', JSON.parse(json), bytes)),
        async (token) => JSON.stringify(await bridge('commitWrite', token)),
        async (token) => {
            const value = await bridge('rollbackWrite', token);
            return value === undefined ? undefined : JSON.stringify(value);
        },
        (json) => {
            const progress: BrowserCommandProgressMessage = {
                type: 'cem-command-progress',
                executionId: message.executionId,
                progress: JSON.parse(json),
            };
            port.postMessage(progress);
        },
    );
    const result: BrowserCommandResultMessage = {
        type: 'cem-command-result',
        executionId: message.executionId,
        result: JSON.parse(resultJson),
    };
    port.postMessage(result);
}

function executeCommandAction(
    message:
        | BrowserCommandCancelRequest
        | BrowserCommandArtifactReadRequest
        | BrowserCommandArtifactDisposeRequest
        | BrowserCommandArtifactsDisposeRequest,
): void {
    if (message.type === 'cem-command-artifact-read') {
        const read = runtime.readCommandArtifactV1(
            message.requestId,
            message.handleId,
            message.offset,
            message.maxBytes,
        );
        const bytes = read.bytes === undefined ? undefined : new Uint8Array(read.bytes);
        const response: BrowserCommandActionResultMessage = {
            type: 'cem-command-action-result',
            actionId: message.actionId,
            result: JSON.parse(read.json),
            ...(bytes === undefined ? {} : { bytes }),
        };
        port.postMessage(response, bytes === undefined ? [] : [bytes.buffer]);
        return;
    }
    const json =
        message.type === 'cem-command-cancel'
            ? runtime.cancelCommandServiceV1(message.requestId, message.reason)
            : message.type === 'cem-command-artifact-dispose'
              ? runtime.disposeCommandArtifactV1(message.requestId, message.handleId)
              : runtime.disposeCommandArtifactsV1(message.requestId);
    const response: BrowserCommandActionResultMessage = {
        type: 'cem-command-action-result',
        actionId: message.actionId,
        result: JSON.parse(json),
    };
    port.postMessage(response);
}

function requestCapability(
    executionId: number,
    name: BrowserCommandCapabilityName,
    argument: unknown,
    bytes?: Uint8Array,
): Promise<unknown> {
    const callbackId = nextCallbackId++;
    const copiedBytes = bytes === undefined ? undefined : new Uint8Array(bytes);
    const request: BrowserCommandCapabilityRequest = {
        type: 'cem-command-capability-request',
        executionId,
        callbackId,
        capability: name,
        argument,
        ...(copiedBytes === undefined ? {} : { bytes: copiedBytes }),
    };
    const pending = new Promise<unknown>((resolve, reject) => {
        pendingCapabilities.set(callbackId, { resolve, reject });
    });
    port.postMessage(request, copiedBytes === undefined ? [] : [copiedBytes.buffer]);
    return pending;
}

function acceptCapabilityResponse(message: BrowserCommandCapabilityResponse): void {
    const pending = pendingCapabilities.get(message.callbackId);
    if (pending === undefined) throw new Error('CEM-ML Node command capability response is unexpected');
    pendingCapabilities.delete(message.callbackId);
    if (message.ok) {
        pending.resolve(message.value);
        return;
    }
    const serialized = message.error;
    const error = new Error(
        `${serialized?.code === undefined ? '' : `${serialized.code}: `}${serialized?.message ?? 'Node host capability failed'}`,
    );
    error.name = serialized?.name ?? 'Error';
    pending.reject(error);
}

function isCommandExecuteRequest(value: unknown): value is BrowserCommandExecuteRequest {
    return isRecord(value) && value.type === 'cem-command-execute' && isPositiveInteger(value.executionId) && isRecord(value.request);
}

function isCommandActionRequest(
    value: unknown,
): value is
    | BrowserCommandCancelRequest
    | BrowserCommandArtifactReadRequest
    | BrowserCommandArtifactDisposeRequest
    | BrowserCommandArtifactsDisposeRequest {
    return (
        isRecord(value) &&
        [
            'cem-command-cancel',
            'cem-command-artifact-read',
            'cem-command-artifact-dispose',
            'cem-command-artifacts-dispose',
        ].includes(String(value.type)) &&
        isPositiveInteger(value.actionId) &&
        typeof value.requestId === 'string'
    );
}

function isCommandCapabilityResponse(value: unknown): value is BrowserCommandCapabilityResponse {
    return isRecord(value) && value.type === 'cem-command-capability-response' && isPositiveInteger(value.callbackId) && typeof value.ok === 'boolean';
}

function reportFailure(error: unknown): void {
    queueMicrotask(() => {
        throw error instanceof Error ? error : new Error(String(error));
    });
}

function parseRuntimeResult(json: string): WorkerWorkResultEnvelope['operation']['payload'] {
    const value = JSON.parse(json) as unknown;
    if (isRecord(value) && isRecord(value.error)) {
        throw new Error(
            typeof value.error.message === 'string' ? value.error.message : 'CEM-ML worker execution failed',
        );
    }
    return value as WorkerWorkResultEnvelope['operation']['payload'];
}

function isWorkRequest(value: unknown): value is WorkerWorkRequest {
    return isRecord(value) && value.type === 'cem-operation-work' && isRecord(value.packet);
}

function parseBootstrap(value: unknown): NodeWorkerBootstrap {
    if (
        !isRecord(value) ||
        !isRecord(value.worker) ||
        !isPositiveInteger(value.worker.slot) ||
        !isPositiveInteger(value.worker.generation) ||
        !isPositiveInteger(value.effectiveWorkers)
    ) {
        throw new Error('CEM-ML Node worker bootstrap is invalid');
    }
    return {
        worker: {
            slot: value.worker.slot,
            generation: value.worker.generation,
        },
        effectiveWorkers: value.effectiveWorkers,
    };
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}
