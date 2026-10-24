import { parseBrowserCommandPackageRequest as parsePackageRequest } from './protocol.js';
import { CommandNativeAdmissionError, isCommandNativeAdmissionError } from './native-admission.js';
import { assertCommandNativeHeadroom, prepareCommandNativeRetirement, abortCommandNativeRetirement, commandNativeAdmission, closeCommandNativeAdmission, validateCommandNativeClosure, readCommandNativeInventory, commandNativeRuntime as runtime } from './managed-native-runtime.js';

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
    if (lifecycle === 'ready' && isRecord(event.data) && event.data.type === 'cem-command-native-retirement') {
        retireCommandNative(event.data); return;
    }
    if (lifecycle === 'ready' && isRecord(event.data) && event.data.type === 'cem-command-native-checkpoint') {
        const message = event.data;
        void checkpointCommandNative(message).catch(reportFailure);
        return;
    }
    if (lifecycle === 'ready' && isRecord(event.data) && event.data.type === 'cem-command-package') {
        executePackage(event.data); return;
    }
    if (lifecycle === 'ready' && isWorkRequest(event.data)) {
        executeWork(event.data);
        return;
    }
    if (lifecycle === 'ready' && isCommandExecuteRequest(event.data)) {
        void executeTrackedCommand(event.data).catch(reportFailure);
        return;
    }
    if (lifecycle === 'ready' && isCommandActionRequest(event.data)) {
        executeManagedAction(event.data);
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
    assertCommandNativeHeadroom();
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
        executeManagedWork(packet),
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
        message.hostConfigurationJson,
    );
    const result: BrowserCommandResultMessage = {
        type: 'cem-command-result',
        ...(commandNativeAdmission.closure === undefined ? {} : { nativeAdmissionClosed: true as const }),
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
            ...(commandNativeAdmission.closure === undefined ? {} : { nativeAdmissionClosed: true as const }),
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
        ...(commandNativeAdmission.closure === undefined ? {} : { nativeAdmissionClosed: true as const }),
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
        ...(typeof value.packageHostConfigurationJson === 'string' ? { packageHostConfigurationJson: value.packageHostConfigurationJson } : {}),
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

function executeManagedWork(packet: WorkerWorkRequest['packet']): string {
    try { return (runtime as unknown as ResumableRuntime).executeOperationWork(JSON.stringify(packet)); }
    catch (error) {
        if (!isCommandNativeAdmissionError(error)) throw error;
        return JSON.stringify({ ...packet, status: 'failed', payload: { error: { code: error.code, message: error.message } } });
    }
}

function executeManagedAction(message: Parameters<typeof executeCommandAction>[0]): void {
    try { executeCommandAction(message); }
    catch (error) {
        if (!isCommandNativeAdmissionError(error)) throw error;
        workerScope.postMessage({ type: 'cem-command-action-result', actionId: message.actionId,
            nativeAdmissionClosed: true, result: { error: { code: error.code, message: error.message } } });
    }
}

async function checkpointCommandNative(message: Record<string, unknown>): Promise<void> {
    if (!isPositiveInteger(message.actionId) || !isPositiveInteger(message.timeoutMs) || message.timeoutMs > 60000 || Object.keys(message).length !== (message.nativeInventory === true ? 4 : 3) + (message.nativeClosure === undefined ? 0 : 1) ||
        (message.nativeClosure !== undefined && (message.nativeInventory !== true || (message.nativeClosure !== 'close' && !isPositiveInteger(message.nativeClosure))))) {
        throw new Error('Invalid managed command checkpoint request');
    }
    commandNativeAdmission.close();
    let result: unknown;
    try {
        if (message.nativeClosure === 'close') closeCommandNativeAdmission();
        const drain = commandNativeAdmission.checkpoint(await commandNativeAdmission.drain(message.timeoutMs));
        if (message.nativeClosure !== undefined) {
            result = { ...drain, ...validateCommandNativeClosure(message.nativeClosure === 'close' ? undefined : message.nativeClosure as number) };
        } else if (message.nativeInventory === true) {
            const inventory = readCommandNativeInventory();
            if (inventory.blockingResources !== 0) throw new CommandNativeAdmissionError('cem.command.inventory.blocked', 'Native resources are retained outside the CLI ledger');
            result = { ...drain, nativeInventory: inventory };
        } else result = drain;
    }
    catch (error) {
        if (!isCommandNativeAdmissionError(error)) throw error;
        result = { error: { code: error.code, message: error.message } };
    }
    workerScope.postMessage({ type: 'cem-command-action-result', actionId: message.actionId, nativeAdmissionClosed: true, result });
}

async function executeTrackedCommand(message: BrowserCommandExecuteRequest): Promise<void> {
    let retire: (() => void) | undefined;
    try {
        retire = commandNativeAdmission.admit();
        await executeCommand(message);
    } catch (error) {
        if (!isCommandNativeAdmissionError(error)) throw error;
        workerScope.postMessage({ type: 'cem-command-result', executionId: message.executionId,
            nativeAdmissionClosed: true, result: { error: { code: error.code, message: error.message } } });
    } finally { retire?.(); }
}

function retireCommandNative(message: Record<string, unknown>): void {
    if (!isPositiveInteger(message.actionId) || !isPositiveInteger(message.preparationId) ||
        Object.keys(message).length !== (message.expectedRevision === undefined ? 3 : 4) ||
        (message.expectedRevision !== undefined && !isPositiveInteger(message.expectedRevision))) {
        throw new Error('Invalid native retirement request');
    }
    let result: unknown;
    try {
        if (message.expectedRevision === undefined) {
            abortCommandNativeRetirement(message.preparationId); result = { aborted: true };
        } else {
            // No await or callback between managed closure and the native barrier.
            commandNativeAdmission.close();
            result = prepareCommandNativeRetirement(message.expectedRevision, message.preparationId);
        }
    } catch (error) {
        if (!isCommandNativeAdmissionError(error)) throw error;
        result = { error: { code: error.code, message: error.message } };
    }
    workerScope.postMessage({ type: 'cem-command-action-result', actionId: message.actionId, result });
}


// Optional ABI is absent from default-profile generated declarations. The managed
// proxy still refuses missing methods; this structural type cannot add exports.
const packageRuntime = runtime as unknown as {
    schemaAssetPackBootstrapV1(): string;
    schemaAssetPackSummaryV1(index: Uint8Array, pin: string): string;
    schemaAssetPackImportV1(index: Uint8Array, pin: string, id: string, version: string, path: string, bytes: Uint8Array): string;
    schemaAssetOwnerOpenV1(index: Uint8Array, pin: string, configuration: string): number;
    schemaAssetOwnerAdmitV1(owner: number, id: string, version: string, path: string, bytes: Uint8Array): void;
    schemaAssetOwnerActivateV1(owner: number, selection: string): string;
    schemaAssetOwnerDisposeV1(owner: number): boolean;
};
// One owner on this exact worker heap, never encoded in transferable replies.
let packageOwner: number | undefined;
function executePackage(value: Record<string, unknown>): void {
    if (!isPositiveInteger(value.actionId)) throw Error('Invalid package action');
    let result: unknown;
    try {
        if (value.operation === 'close') {
            if (Object.keys(value).length !== 3) throw Error('Invalid package closure');
            commandNativeAdmission.close(); closeCommandNativeAdmission();
            workerScope.postMessage({ type: 'cem-command-action-result', actionId: value.actionId, result: true }); return;
        }
        const message = parsePackageRequest(value);
        switch (message.operation) {
            case 'bootstrap': result = packageRuntime.schemaAssetPackBootstrapV1(); break;
            case 'summary': result = packageRuntime.schemaAssetPackSummaryV1(message.index, message.pin); break;
            case 'import': {
                const source = message.source;
                result = packageRuntime.schemaAssetPackImportV1(message.index, message.pin, source.packageId, source.version, source.path, source.bytes); break;
            }
            case 'activate': {
                const configuration = bootstrapState?.packageHostConfigurationJson;
                if (configuration === undefined) throw Error('cem.schema_asset.policy');
                packageOwner ??= packageRuntime.schemaAssetOwnerOpenV1(message.index, message.pin, configuration);
                for (const source of message.sources) packageRuntime.schemaAssetOwnerAdmitV1(packageOwner, source.packageId, source.version, source.path, source.bytes);
                result = packageRuntime.schemaAssetOwnerActivateV1(packageOwner, message.selection); break;
            }
            case 'release':
                if (packageOwner !== undefined && packageRuntime.schemaAssetOwnerDisposeV1(packageOwner) !== true) throw Error('cem.schema_asset.owner');
                packageOwner = undefined; result = true; break;
        }
    } catch (error) {
        result = { error: { code: typeof error === 'string' && /^cem\.[a-z0-9_.]+$/u.test(error) ? error
            : isCommandNativeAdmissionError(error) ? error.code : 'cem.schema_asset.transport', message: 'Worker package operation refused' } };
    }
    workerScope.postMessage({ type: 'cem-command-action-result', actionId: value.actionId, result,
        ...(commandNativeAdmission.closure === undefined ? {} : { nativeAdmissionClosed: true }) });
}
