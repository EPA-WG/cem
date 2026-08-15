import runtimeMetadata from '@epa-wg/cem-ml/runtime.json' with { type: 'json' };

import {
    DEFAULT_STARTUP_TIMEOUT_MS,
    MAX_STARTUP_TIMEOUT_MS,
    OPERATION_PROTOCOL_VERSION,
    WORKER_PROTOCOL_VERSION,
} from './protocol.js';
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
    BrowserCommandSerializedError,
    BrowserWorkerBootstrap,
    BrowserWorkerCapabilityManifest,
    BrowserWorkerInitializePayload,
} from './protocol.js';
import type {
    CommandArtifactHandleV1,
    CommandServiceArtifactDisposeAckV1,
    CommandServiceArtifactReadV1,
    CommandServiceControlAckV1,
    CommandServiceHostCapabilitiesV1,
    CommandServiceProgressCallbackV1,
    CommandServiceProgressV1,
    CommandServiceRequestV1,
    CommandServiceResultV1,
} from '@epa-wg/cem-ml/wasm';

export type {
    CommandArtifactHandleV1,
    CommandPreparedWriteTokenV1,
    CommandResolvedResourceV1,
    CommandResolvedWriteV1,
    CommandResourceReadRequestV1,
    CommandResourceWriteRequestV1,
    CommandRevisionLedgerRequestV1,
    CommandRevisionLedgerV1,
    CommandServiceArtifactDisposeAckV1,
    CommandServiceArtifactReadV1,
    CommandServiceControlAckV1,
    CommandServiceHostCapabilitiesV1,
    CommandServiceProgressCallbackV1,
    CommandServiceProgressV1,
    CommandServiceRequestV1,
    CommandServiceResultV1,
} from '@epa-wg/cem-ml/wasm';

const DEFAULT_ARTIFACT_READ_BYTES = 1024 * 1024;

interface RuntimeMetadata {
    readonly abi: { readonly identity: string };
}

interface PendingAction {
    readonly resolve: (value: BrowserCommandActionResponse) => void;
    readonly reject: (error: Error) => void;
}

interface BrowserCommandActionResponse {
    readonly result: unknown;
    readonly bytes?: Uint8Array;
}

export interface BrowserCommandServiceClientOptions {
    readonly host: CommandServiceHostCapabilitiesV1;
    readonly startupTimeoutMs?: number;
    readonly onWorkerFailure?: (failure: BrowserCommandWorkerFailure) => void;
}

export interface BrowserCommandServiceExecuteOptions {
    readonly signal?: AbortSignal;
    readonly onProgress?: CommandServiceProgressCallbackV1;
}

export interface BrowserCommandArtifactReadOptions {
    readonly offset?: number;
    readonly maxBytes?: number;
}

export interface BrowserCommandArtifactReadResult {
    readonly metadata: CommandServiceArtifactReadV1;
    readonly bytes: Uint8Array;
}

export interface BrowserCommandWorkerDescriptor {
    readonly slot: 1;
    readonly generation: 1;
    readonly runtimeInstanceId: string;
    readonly commonVersion: string;
}

export interface BrowserCommandWorkerFailure {
    readonly worker: BrowserCommandWorkerDescriptor;
    readonly code: 'worker-error' | 'message-error';
    readonly message: string;
}

export class BrowserCommandServiceError extends Error {
    readonly code: string;
    readonly details: unknown;

    constructor(code: string, message: string, details?: unknown) {
        super(message);
        this.name = 'BrowserCommandServiceError';
        this.code = code;
        this.details = details;
    }
}

export class BrowserCommandServiceHandle implements PromiseLike<CommandServiceResultV1> {
    readonly requestId: string;

    #client: BrowserCommandServiceClient;
    #executionId: number;
    #promise: Promise<CommandServiceResultV1>;
    #resolve!: (result: CommandServiceResultV1) => void;
    #reject!: (error: Error) => void;
    #listeners = new Set<CommandServiceProgressCallbackV1>();
    #abortSignal: AbortSignal | undefined;
    #abortListener: (() => void) | undefined;
    #settled = false;

    constructor(
        client: BrowserCommandServiceClient,
        executionId: number,
        requestId: string,
        options: BrowserCommandServiceExecuteOptions,
    ) {
        this.#client = client;
        this.#executionId = executionId;
        this.requestId = requestId;
        this.#promise = new Promise((resolve, reject) => {
            this.#resolve = resolve;
            this.#reject = reject;
        });
        if (options.onProgress !== undefined) this.#listeners.add(options.onProgress);
        if (options.signal !== undefined) {
            this.#abortSignal = options.signal;
            this.#abortListener = () => {
                void this.cancel(abortReason(options.signal)).catch(() => undefined);
            };
            options.signal.addEventListener('abort', this.#abortListener, { once: true });
        }
    }

    result(): Promise<CommandServiceResultV1> {
        return this.#promise;
    }

    subscribe(listener: CommandServiceProgressCallbackV1): () => void {
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    }

    cancel(reason?: string): Promise<CommandServiceControlAckV1> {
        return this.#client.cancel(this.requestId, reason);
    }

    readArtifact(
        handle: CommandArtifactHandleV1,
        options: BrowserCommandArtifactReadOptions = {},
    ): Promise<BrowserCommandArtifactReadResult> {
        return this.#client.readArtifact(this.requestId, handle, options);
    }

    disposeArtifact(handle: CommandArtifactHandleV1): Promise<CommandServiceArtifactDisposeAckV1> {
        return this.#client.disposeArtifact(this.requestId, handle);
    }

    dispose(): Promise<CommandServiceArtifactDisposeAckV1> {
        return this.#client.disposeArtifacts(this.requestId);
    }

    then<TResult1 = CommandServiceResultV1, TResult2 = never>(
        onfulfilled?: ((value: CommandServiceResultV1) => TResult1 | PromiseLike<TResult1>) | null,
        onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
    ): Promise<TResult1 | TResult2> {
        return this.#promise.then(onfulfilled, onrejected);
    }

    acceptProgress(progress: CommandServiceProgressV1): void {
        if (this.#settled) return;
        for (const listener of this.#listeners) {
            try {
                listener(progress);
            } catch {
                // Progress is observational and cannot alter command execution.
            }
        }
    }

    settle(result: CommandServiceResultV1): void {
        if (this.#settled) return;
        this.#settled = true;
        this.#cleanupAbort();
        this.#resolve(result);
    }

    fail(error: Error): void {
        if (this.#settled) return;
        this.#settled = true;
        this.#cleanupAbort();
        this.#reject(error);
    }

    get executionId(): number {
        return this.#executionId;
    }

    startAbort(): void {
        if (this.#abortSignal?.aborted) this.#abortListener?.();
    }

    #cleanupAbort(): void {
        if (this.#abortSignal !== undefined && this.#abortListener !== undefined) {
            this.#abortSignal.removeEventListener('abort', this.#abortListener);
        }
        this.#listeners.clear();
    }
}

export class BrowserCommandServiceClient {
    readonly capability: BrowserWorkerCapabilityManifest;
    readonly worker: BrowserCommandWorkerDescriptor;

    #physicalWorker: Worker;
    #host: CommandServiceHostCapabilitiesV1;
    #onWorkerFailure: ((failure: BrowserCommandWorkerFailure) => void) | undefined;
    #executions = new Map<number, BrowserCommandServiceHandle>();
    #actions = new Map<number, PendingAction>();
    #nextExecutionId = 1;
    #nextActionId = 1;
    #closed = false;

    private constructor(
        physicalWorker: Worker,
        initialization: BrowserWorkerInitializePayload,
        host: CommandServiceHostCapabilitiesV1,
        onWorkerFailure: ((failure: BrowserCommandWorkerFailure) => void) | undefined,
    ) {
        this.#physicalWorker = physicalWorker;
        this.#host = host;
        this.#onWorkerFailure = onWorkerFailure;
        this.capability = Object.freeze(initialization.capability);
        this.worker = Object.freeze({
            slot: 1,
            generation: 1,
            runtimeInstanceId: initialization.runtimeInstanceId,
            commonVersion: initialization.commonVersion,
        });
        physicalWorker.addEventListener('message', this.#onMessage);
        physicalWorker.addEventListener('error', this.#onError);
        physicalWorker.addEventListener('messageerror', this.#onMessageError);
    }

    static async create(options: BrowserCommandServiceClientOptions): Promise<BrowserCommandServiceClient> {
        validateOptions(options);
        if (typeof Worker !== 'function') {
            throw new BrowserCommandServiceError(
                'cem.browser_command.worker_unavailable',
                'CEM-ML browser command service requires a dedicated Worker',
            );
        }
        const startupTimeoutMs = options.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS;
        requireBoundedInteger('startupTimeoutMs', startupTimeoutMs, 1, MAX_STARTUP_TIMEOUT_MS);
        const runtimeHostId = `browser-command-${nextBrowserCommandHostId++}`;
        const physicalWorker = new Worker(new URL('./browser-worker.js', import.meta.url), {
            name: runtimeHostId,
            type: 'module',
        });
        try {
            const initialization = await initializeWorker(physicalWorker, startupTimeoutMs, runtimeHostId);
            return new BrowserCommandServiceClient(
                physicalWorker,
                initialization,
                options.host,
                options.onWorkerFailure,
            );
        } catch (error) {
            physicalWorker.terminate();
            throw asError(error);
        }
    }

    execute(
        request: CommandServiceRequestV1,
        options: BrowserCommandServiceExecuteOptions = {},
    ): BrowserCommandServiceHandle {
        this.#requireOpen();
        const executionId = this.#nextExecutionId++;
        const handle = new BrowserCommandServiceHandle(this, executionId, request.requestId, options);
        this.#executions.set(executionId, handle);
        const message: BrowserCommandExecuteRequest = {
            type: 'cem-command-execute',
            executionId,
            request,
        };
        try {
            this.#physicalWorker.postMessage(message);
        } catch (error) {
            this.#executions.delete(executionId);
            handle.fail(asError(error));
        }
        handle.startAbort();
        return handle;
    }

    async close(): Promise<void> {
        if (this.#closed) return;
        this.#closed = true;
        this.#physicalWorker.removeEventListener('message', this.#onMessage);
        this.#physicalWorker.removeEventListener('error', this.#onError);
        this.#physicalWorker.removeEventListener('messageerror', this.#onMessageError);
        try {
            this.#physicalWorker.postMessage({ type: 'cem-worker-close' });
        } catch {
            // Termination and pending-request cleanup remain authoritative.
        } finally {
            this.#physicalWorker.terminate();
            this.#rejectPending(
                new BrowserCommandServiceError(
                    'cem.browser_command.client_closed',
                    'CEM-ML browser command-service client was closed',
                ),
            );
        }
    }

    cancel(requestId: string, reason?: string): Promise<CommandServiceControlAckV1> {
        const actionId = this.#nextAction();
        const message: BrowserCommandCancelRequest = {
            type: 'cem-command-cancel',
            actionId,
            requestId,
            ...(reason === undefined ? {} : { reason }),
        };
        return this.#action<CommandServiceControlAckV1>(actionId, message).then(({ value }) => value);
    }

    async readArtifact(
        requestId: string,
        handle: CommandArtifactHandleV1,
        options: BrowserCommandArtifactReadOptions,
    ): Promise<BrowserCommandArtifactReadResult> {
        const actionId = this.#nextAction();
        const message: BrowserCommandArtifactReadRequest = {
            type: 'cem-command-artifact-read',
            actionId,
            requestId,
            handleId: handle.handleId,
            offset: options.offset ?? 0,
            maxBytes: options.maxBytes ?? DEFAULT_ARTIFACT_READ_BYTES,
        };
        const response = await this.#action<CommandServiceArtifactReadV1>(actionId, message);
        if (!(response.bytes instanceof Uint8Array)) {
            throw new BrowserCommandServiceError(
                'cem.browser_command.artifact_bytes_missing',
                'CEM-ML browser command artifact response did not include copied bytes',
            );
        }
        return { metadata: response.value, bytes: response.bytes };
    }

    disposeArtifact(
        requestId: string,
        handle: CommandArtifactHandleV1,
    ): Promise<CommandServiceArtifactDisposeAckV1> {
        const actionId = this.#nextAction();
        const message: BrowserCommandArtifactDisposeRequest = {
            type: 'cem-command-artifact-dispose',
            actionId,
            requestId,
            handleId: handle.handleId,
        };
        return this.#action<CommandServiceArtifactDisposeAckV1>(actionId, message).then(({ value }) => value);
    }

    disposeArtifacts(requestId: string): Promise<CommandServiceArtifactDisposeAckV1> {
        const actionId = this.#nextAction();
        const message: BrowserCommandArtifactsDisposeRequest = {
            type: 'cem-command-artifacts-dispose',
            actionId,
            requestId,
        };
        return this.#action<CommandServiceArtifactDisposeAckV1>(actionId, message).then(({ value }) => value);
    }

    #onMessage = (event: MessageEvent<unknown>): void => {
        try {
            const message = event.data;
            if (!isRecord(message) || typeof message.type !== 'string') {
                throw new Error('CEM-ML browser command worker emitted an invalid message');
            }
            switch (message.type) {
                case 'cem-command-progress':
                    this.#acceptProgress(message as unknown as BrowserCommandProgressMessage);
                    return;
                case 'cem-command-result':
                    this.#acceptResult(message as unknown as BrowserCommandResultMessage);
                    return;
                case 'cem-command-action-result':
                    this.#acceptAction(message as unknown as BrowserCommandActionResultMessage);
                    return;
                case 'cem-command-capability-request':
                    void this.#serveCapability(message as unknown as BrowserCommandCapabilityRequest).catch((error) => {
                        this.#failTransport(asError(error), 'message-error');
                    });
                    return;
                default:
                    throw new Error(`CEM-ML browser command worker emitted unexpected ${message.type}`);
            }
        } catch (error) {
            this.#failTransport(asError(error), 'message-error');
        }
    };

    #onError = (event: ErrorEvent): void => {
        this.#failTransport(new Error(event.message || 'CEM-ML browser command worker failed'), 'worker-error');
    };

    #onMessageError = (): void => {
        this.#failTransport(new Error('CEM-ML browser command worker emitted an invalid message'), 'message-error');
    };

    #acceptProgress(message: BrowserCommandProgressMessage): void {
        const handle = this.#executions.get(message.executionId);
        if (handle === undefined) throw new Error('CEM-ML browser command progress targets an unknown execution');
        handle.acceptProgress(message.progress as CommandServiceProgressV1);
    }

    #acceptResult(message: BrowserCommandResultMessage): void {
        const handle = this.#executions.get(message.executionId);
        if (handle === undefined) throw new Error('CEM-ML browser command result targets an unknown execution');
        this.#executions.delete(message.executionId);
        try {
            handle.settle(unwrapResponse<CommandServiceResultV1>(message.result));
        } catch (error) {
            handle.fail(asError(error));
        }
    }

    #acceptAction(message: BrowserCommandActionResultMessage): void {
        const pending = this.#actions.get(message.actionId);
        if (pending === undefined) throw new Error('CEM-ML browser command action result is unexpected');
        this.#actions.delete(message.actionId);
        try {
            unwrapResponse(message.result);
            pending.resolve({ result: message.result, ...(message.bytes === undefined ? {} : { bytes: message.bytes }) });
        } catch (error) {
            pending.reject(asError(error));
        }
    }

    async #serveCapability(message: BrowserCommandCapabilityRequest): Promise<void> {
        const response: BrowserCommandCapabilityResponse = await this.#invokeCapability(message).then(
            (value) => ({
                type: 'cem-command-capability-response',
                callbackId: message.callbackId,
                ok: true,
                value,
            }),
            (error: unknown) => ({
                type: 'cem-command-capability-response',
                callbackId: message.callbackId,
                ok: false,
                error: serializeError(error),
            }),
        );
        if (this.#closed) return;
        try {
            this.#physicalWorker.postMessage(response);
        } catch (error) {
            const cloneFailure: BrowserCommandCapabilityResponse = {
                type: 'cem-command-capability-response',
                callbackId: message.callbackId,
                ok: false,
                error: serializeError(error),
            };
            this.#physicalWorker.postMessage(cloneFailure);
        }
    }

    #invokeCapability(message: BrowserCommandCapabilityRequest): Promise<unknown> {
        if (!this.#executions.has(message.executionId)) {
            return Promise.reject(new Error('capability request targets an inactive command execution'));
        }
        const capability: BrowserCommandCapabilityName = message.capability;
        switch (capability) {
            case 'currentRevision':
                return Promise.resolve(this.#host.currentRevision(message.argument as never));
            case 'readResource':
                return Promise.resolve(this.#host.readResource(message.argument as never));
            case 'prepareWrite':
                if (!(message.bytes instanceof Uint8Array)) {
                    return Promise.reject(new Error('prepareWrite capability did not include bytes'));
                }
                return Promise.resolve(this.#host.prepareWrite(message.argument as never, message.bytes));
            case 'commitWrite':
                return Promise.resolve(this.#host.commitWrite(String(message.argument)));
            case 'rollbackWrite':
                return Promise.resolve(this.#host.rollbackWrite(String(message.argument)));
        }
    }

    #nextAction(): number {
        this.#requireOpen();
        return this.#nextActionId++;
    }

    #action<T>(actionId: number, message: object): Promise<{ readonly value: T; readonly bytes?: Uint8Array }> {
        return new Promise<BrowserCommandActionResponse>((resolve, reject) => {
            this.#actions.set(actionId, { resolve, reject });
            try {
                this.#physicalWorker.postMessage(message);
            } catch (error) {
                this.#actions.delete(actionId);
                reject(asError(error));
            }
        }).then(({ result, bytes }) => ({
            value: unwrapResponse<T>(result),
            ...(bytes === undefined ? {} : { bytes }),
        }));
    }

    #requireOpen(): void {
        if (this.#closed) {
            throw new BrowserCommandServiceError(
                'cem.browser_command.client_closed',
                'CEM-ML browser command-service client is closed',
            );
        }
    }

    #failTransport(error: Error, code: BrowserCommandWorkerFailure['code']): void {
        if (this.#closed) return;
        this.#closed = true;
        this.#physicalWorker.terminate();
        try {
            this.#onWorkerFailure?.({ worker: this.worker, code, message: error.message });
        } catch {
            // Failure reporting is observational and cannot alter cleanup.
        }
        this.#rejectPending(
            new BrowserCommandServiceError('cem.browser_command.worker_failed', error.message, error),
        );
    }

    #rejectPending(error: Error): void {
        for (const handle of this.#executions.values()) handle.fail(error);
        this.#executions.clear();
        for (const action of this.#actions.values()) action.reject(error);
        this.#actions.clear();
    }
}

let nextBrowserCommandHostId = 1;

export function createBrowserCommandServiceClient(
    options: BrowserCommandServiceClientOptions,
): Promise<BrowserCommandServiceClient> {
    return BrowserCommandServiceClient.create(options);
}

async function initializeWorker(
    worker: Worker,
    startupTimeoutMs: number,
    runtimeHostId: string,
): Promise<BrowserWorkerInitializePayload> {
    return new Promise((resolve, reject) => {
        let settled = false;
        const finish = (callback: () => void): void => {
            if (settled) return;
            settled = true;
            clearTimeout(timeout);
            worker.removeEventListener('message', onMessage);
            worker.removeEventListener('error', onError);
            worker.removeEventListener('messageerror', onMessageError);
            callback();
        };
        const onMessage = (event: MessageEvent<unknown>): void => {
            finish(() => {
                try {
                    resolve(validateInitialization(event.data));
                } catch (error) {
                    reject(asError(error));
                }
            });
        };
        const onError = (event: ErrorEvent): void => {
            finish(() => reject(new Error(event.message || 'CEM-ML browser command worker failed')));
        };
        const onMessageError = (): void => {
            finish(() => reject(new Error('CEM-ML browser command worker initialization was not cloneable')));
        };
        const timeout = setTimeout(() => {
            finish(() => reject(new Error('CEM-ML browser command worker startup timed out')));
        }, startupTimeoutMs);
        worker.addEventListener('message', onMessage);
        worker.addEventListener('error', onError);
        worker.addEventListener('messageerror', onMessageError);
        const bootstrap: BrowserWorkerBootstrap = {
            type: 'cem-worker-initialize',
            worker: { slot: 1, generation: 1 },
            effectiveWorkers: 1,
            runtimeHostId,
            abiIdentity: runtimeAbiIdentity(),
        };
        worker.postMessage(bootstrap);
    });
}

function validateInitialization(value: unknown): BrowserWorkerInitializePayload {
    if (
        !isRecord(value) ||
        value.workerProtocolVersion !== WORKER_PROTOCOL_VERSION ||
        !isRecord(value.worker) ||
        value.worker.slot !== 1 ||
        value.worker.generation !== 1 ||
        value.sequence !== 1 ||
        !Array.isArray(value.transfers) ||
        value.transfers.length !== 0 ||
        !isRecord(value.operation) ||
        value.operation.protocolVersion !== OPERATION_PROTOCOL_VERSION ||
        value.operation.kind !== 'initialize' ||
        !isRecord(value.operation.payload)
    ) {
        throw new Error('CEM-ML browser command worker initialization envelope is invalid');
    }
    const payload = value.operation.payload;
    if (
        typeof payload.runtimeInstanceId !== 'string' ||
        payload.runtimeInstanceId.length === 0 ||
        typeof payload.commonVersion !== 'string' ||
        payload.commonVersion.length === 0 ||
        !isRecord(payload.capability) ||
        payload.capability.runtime !== 'wasm-browser-worker' ||
        payload.capability.executorTopology !== 'browser-worker-pool' ||
        payload.capability.effectiveMaxWorkers !== 1 ||
        payload.capability.abiIdentity !== runtimeAbiIdentity()
    ) {
        throw new Error('CEM-ML browser command worker capability identity is invalid');
    }
    return payload as unknown as BrowserWorkerInitializePayload;
}

function validateOptions(options: BrowserCommandServiceClientOptions): void {
    if (!isRecord(options) || !isRecord(options.host)) {
        throw new TypeError('browser command-service options.host is required');
    }
    for (const capability of [
        'currentRevision',
        'readResource',
        'prepareWrite',
        'commitWrite',
        'rollbackWrite',
    ] as const) {
        if (typeof options.host[capability] !== 'function') {
            throw new TypeError(`browser command-service host.${capability} must be a function`);
        }
    }
}

function unwrapResponse<T>(value: unknown): T {
    if (isRecord(value) && isRecord(value.error)) {
        const code = typeof value.error.code === 'string' ? value.error.code : 'cem.browser_command.unknown';
        const message = typeof value.error.message === 'string' ? value.error.message : 'CEM-ML command failed';
        throw new BrowserCommandServiceError(code, message, value.error);
    }
    return value as T;
}

function serializeError(error: unknown): BrowserCommandSerializedError {
    if (error instanceof BrowserCommandServiceError) {
        return { name: error.name, message: error.message, code: error.code };
    }
    if (error instanceof Error) return { name: error.name, message: error.message };
    return { name: 'Error', message: String(error) };
}

function abortReason(signal: AbortSignal | undefined): string {
    if (signal?.reason instanceof Error && signal.reason.message.length > 0) return signal.reason.message;
    if (typeof signal?.reason === 'string' && signal.reason.length > 0) return signal.reason;
    return 'browser command aborted';
}

function runtimeAbiIdentity(): string {
    const identity = (runtimeMetadata as RuntimeMetadata).abi?.identity;
    if (typeof identity !== 'string' || identity.length === 0) {
        throw new Error('CEM-ML runtime metadata does not provide an ABI identity');
    }
    return identity;
}

function requireBoundedInteger(field: string, value: number, minimum: number, maximum: number): void {
    if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        throw new RangeError(`${field}=${value} is outside ${minimum}..=${maximum}`);
    }
}

function asError(error: unknown): Error {
    return error instanceof Error ? error : new Error(String(error));
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
