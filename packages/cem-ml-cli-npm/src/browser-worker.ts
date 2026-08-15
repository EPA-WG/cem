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
    BrowserWorkerBootstrap,
    BrowserWorkerCapabilityManifest,
    BrowserWorkerInitializeEnvelope,
    WorkerProtocolDescriptor,
    WorkerWorkRequest,
    WorkerWorkResultEnvelope,
} from './protocol.js';
import type { ResumableRuntime } from './operation.js';

const workerScope = globalThis as unknown as DedicatedWorkerGlobalScope;
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;
let lifecycle: 'initializing' | 'loading' | 'ready' = 'initializing';
let bootstrapState: BrowserWorkerBootstrap | undefined;
let protocolState: WorkerProtocolDescriptor | undefined;
let capabilityState: BrowserWorkerCapabilityManifest | undefined;
let nextSequence = 2;
let nextCallbackId = 1;
const pendingCapabilities = new Map<
    number,
    { readonly resolve: (value: unknown) => void; readonly reject: (error: Error) => void }
>();

workerScope.addEventListener('message', (event: MessageEvent<unknown>) => {
    if (isRecord(event.data) && event.data.type === 'cem-worker-close') {
        workerScope.close();
        return;
    }
    if (lifecycle === 'ready' && isWorkRequest(event.data)) {
        executeWork(event.data);
        return;
    }
    if (lifecycle === 'ready' && isCommandExecuteRequest(event.data)) {
        void executeCommand(event.data).catch(reportFailure);
        return;
    }
    if (lifecycle === 'ready' && isCommandActionRequest(event.data)) {
        executeCommandAction(event.data);
        return;
    }
    if (lifecycle === 'ready' && isCommandCapabilityResponse(event.data)) {
        acceptCapabilityResponse(event.data);
        return;
    }
    if (lifecycle !== 'initializing') {
        reportFailure(new Error('CEM-ML browser worker accepts exactly one initialization message'));
        return;
    }
    lifecycle = 'loading';
    void initialize(parseBootstrap(event.data)).catch(reportFailure);
});

async function initialize(bootstrap: BrowserWorkerBootstrap): Promise<void> {
    await initializeRuntime();
    const protocol = JSON.parse(runtime.workerProtocolDescriptor()) as WorkerProtocolDescriptor;
    const capabilityRequest = JSON.stringify({
        runtime: 'wasm-browser-worker',
        targetIdentity: 'wasm32-unknown-unknown:web',
        abiIdentity: bootstrap.abiIdentity,
        debugControlActive: false,
    });
    const capability = JSON.parse(
        runtime.browserWorkerCapabilityManifest(capabilityRequest, bootstrap.effectiveWorkers),
    ) as BrowserWorkerCapabilityManifest & { readonly error?: unknown };
    if (capability.error !== undefined) {
        throw new Error('CEM-ML browser worker capability projection failed');
    }

    const envelope: BrowserWorkerInitializeEnvelope = {
        workerProtocolVersion: protocol.workerProtocolVersion,
        worker: bootstrap.worker,
        sequence: 1,
        operation: {
            protocolVersion: protocol.operationProtocolVersion,
            kind: 'initialize',
            payload: {
                runtimeInstanceId: `${bootstrap.runtimeHostId}:slot-${bootstrap.worker.slot}:generation-${bootstrap.worker.generation}`,
                commonVersion: runtime.version(),
                protocol,
                capability,
            },
        },
        transfers: [],
    };

    lifecycle = 'ready';
    bootstrapState = bootstrap;
    protocolState = protocol;
    capabilityState = capability;
    workerScope.postMessage(envelope);
}

function executeWork(message: WorkerWorkRequest): void {
    const bootstrap = bootstrapState;
    const protocol = protocolState;
    if (bootstrap === undefined || protocol === undefined) {
        throw new Error('CEM-ML browser worker received work before initialization');
    }
    const packet = message.packet;
    if (
        packet.worker.slot !== bootstrap.worker.slot ||
        packet.worker.generation !== bootstrap.worker.generation
    ) {
        throw new Error('CEM-ML work packet targets a different browser worker generation');
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
    workerScope.postMessage(response);
}

async function executeCommand(message: BrowserCommandExecuteRequest): Promise<void> {
    const capability = requireCapability();
    const capabilityRequest = JSON.stringify({
        runtime: capability.runtime,
        targetIdentity: capability.targetIdentity,
        abiIdentity: capability.abiIdentity,
        debugControlActive: false,
    });
    const bridge = (name: BrowserCommandCapabilityName, argument: unknown, bytes?: Uint8Array) =>
        requestCapability(message.executionId, name, argument, bytes);
    const resultJson = await runtime.executeCommandServiceV1(
        JSON.stringify(message.request),
        capabilityRequest,
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
            workerScope.postMessage(progress);
        },
    );
    const result: BrowserCommandResultMessage = {
        type: 'cem-command-result',
        executionId: message.executionId,
        result: JSON.parse(resultJson),
    };
    workerScope.postMessage(result);
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
        workerScope.postMessage(response, bytes === undefined ? [] : [bytes.buffer]);
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
    workerScope.postMessage(response);
}

function requestCapability(
    executionId: number,
    capability: BrowserCommandCapabilityName,
    argument: unknown,
    bytes?: Uint8Array,
): Promise<unknown> {
    const callbackId = nextCallbackId++;
    const copiedBytes = bytes === undefined ? undefined : new Uint8Array(bytes);
    const request: BrowserCommandCapabilityRequest = {
        type: 'cem-command-capability-request',
        executionId,
        callbackId,
        capability,
        argument,
        ...(copiedBytes === undefined ? {} : { bytes: copiedBytes }),
    };
    const pending = new Promise<unknown>((resolve, reject) => {
        pendingCapabilities.set(callbackId, { resolve, reject });
    });
    workerScope.postMessage(request, copiedBytes === undefined ? [] : [copiedBytes.buffer]);
    return pending;
}

function acceptCapabilityResponse(message: BrowserCommandCapabilityResponse): void {
    const pending = pendingCapabilities.get(message.callbackId);
    if (pending === undefined) throw new Error('CEM-ML browser command capability response is unexpected');
    pendingCapabilities.delete(message.callbackId);
    if (message.ok) {
        pending.resolve(message.value);
        return;
    }
    const serialized = message.error;
    const code = serialized?.code;
    const error = new Error(
        `${code === undefined ? '' : `${code}: `}${serialized?.message ?? 'browser host capability failed'}`,
    );
    error.name = serialized?.name ?? 'Error';
    pending.reject(error);
}

function requireCapability(): BrowserWorkerCapabilityManifest {
    if (bootstrapState === undefined || protocolState === undefined || capabilityState === undefined) {
        throw new Error('CEM-ML browser command execution started before initialization');
    }
    return capabilityState;
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

function isCommandExecuteRequest(value: unknown): value is BrowserCommandExecuteRequest {
    return (
        isRecord(value) &&
        value.type === 'cem-command-execute' &&
        isPositiveInteger(value.executionId) &&
        isRecord(value.request)
    );
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
    return (
        isRecord(value) &&
        value.type === 'cem-command-capability-response' &&
        isPositiveInteger(value.callbackId) &&
        typeof value.ok === 'boolean'
    );
}

function parseBootstrap(value: unknown): BrowserWorkerBootstrap {
    if (
        !isRecord(value) ||
        value.type !== 'cem-worker-initialize' ||
        !isRecord(value.worker) ||
        !isPositiveInteger(value.worker.slot) ||
        !isPositiveInteger(value.worker.generation) ||
        !isPositiveInteger(value.effectiveWorkers) ||
        typeof value.runtimeHostId !== 'string' ||
        value.runtimeHostId.length === 0 ||
        typeof value.abiIdentity !== 'string' ||
        value.abiIdentity.length === 0
    ) {
        throw new Error('CEM-ML browser worker bootstrap is invalid');
    }
    return {
        type: 'cem-worker-initialize',
        worker: {
            slot: value.worker.slot,
            generation: value.worker.generation,
        },
        effectiveWorkers: value.effectiveWorkers,
        runtimeHostId: value.runtimeHostId,
        abiIdentity: value.abiIdentity,
    };
}

function reportFailure(error: unknown): void {
    const failure = error instanceof Error ? error : new Error(String(error));
    setTimeout(() => {
        throw failure;
    });
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}
