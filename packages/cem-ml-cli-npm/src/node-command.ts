import { Worker } from 'node:worker_threads';

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
    NodeWorkerInitializeEnvelope,
    NodeWorkerInitializePayload,
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

const DEFAULT_ARTIFACT_READ_BYTES = 1024 * 1024;

interface PendingAction {
    readonly resolve: (value: NodeCommandActionResponse) => void;
    readonly reject: (error: Error) => void;
}

interface NodeCommandActionResponse {
    readonly result: unknown;
    readonly bytes?: Uint8Array;
}

export interface NodeCommandServiceClientOptions {
    readonly host: CommandServiceHostCapabilitiesV1;
    readonly startupTimeoutMs?: number;
    readonly onWorkerFailure?: (error: Error) => void;
}

export interface NodeCommandServiceExecuteOptions {
    readonly signal?: AbortSignal;
    readonly onProgress?: CommandServiceProgressCallbackV1;
}

export interface NodeCommandArtifactReadOptions {
    readonly offset?: number;
    readonly maxBytes?: number;
}

export interface NodeCommandArtifactReadResult {
    readonly metadata: CommandServiceArtifactReadV1;
    readonly bytes: Uint8Array;
}

export class NodeCommandServiceError extends Error {
    readonly code: string;
    readonly details: unknown;

    constructor(code: string, message: string, details?: unknown) {
        super(message);
        this.name = 'NodeCommandServiceError';
        this.code = code;
        this.details = details;
    }
}

export class NodeCommandServiceHandle implements PromiseLike<CommandServiceResultV1> {
    readonly requestId: string;

    #client: NodeCommandServiceClient;
    #promise: Promise<CommandServiceResultV1>;
    #resolve!: (result: CommandServiceResultV1) => void;
    #reject!: (error: Error) => void;
    #listeners = new Set<CommandServiceProgressCallbackV1>();
    #signal: AbortSignal | undefined;
    #abortListener: (() => void) | undefined;
    #settled = false;

    constructor(
        client: NodeCommandServiceClient,
        requestId: string,
        options: NodeCommandServiceExecuteOptions,
    ) {
        this.#client = client;
        this.requestId = requestId;
        this.#promise = new Promise((resolve, reject) => {
            this.#resolve = resolve;
            this.#reject = reject;
        });
        if (options.onProgress !== undefined) this.#listeners.add(options.onProgress);
        if (options.signal !== undefined) {
            this.#signal = options.signal;
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
        options: NodeCommandArtifactReadOptions = {},
    ): Promise<NodeCommandArtifactReadResult> {
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
                // Progress is observational.
            }
        }
    }

    settle(result: CommandServiceResultV1): void {
        if (this.#settled) return;
        this.#settled = true;
        this.#cleanup();
        this.#resolve(result);
    }

    fail(error: Error): void {
        if (this.#settled) return;
        this.#settled = true;
        this.#cleanup();
        this.#reject(error);
    }

    startAbort(): void {
        if (this.#signal?.aborted) this.#abortListener?.();
    }

    #cleanup(): void {
        if (this.#signal !== undefined && this.#abortListener !== undefined) {
            this.#signal.removeEventListener('abort', this.#abortListener);
        }
        this.#listeners.clear();
    }
}

export class NodeCommandServiceClient {
    readonly capability: NodeWorkerInitializePayload['capability'];
    readonly runtimeInstanceId: string;

    #worker: Worker;
    #host: CommandServiceHostCapabilitiesV1;
    #executions = new Map<number, NodeCommandServiceHandle>();
    #actions = new Map<number, PendingAction>();
    #nextExecutionId = 1;
    #nextActionId = 1;
    #closed = false;

    private constructor(
        worker: Worker,
        initialization: NodeWorkerInitializePayload,
        host: CommandServiceHostCapabilitiesV1,
        onWorkerFailure: ((error: Error) => void) | undefined,
    ) {
        this.#worker = worker;
        this.#host = host;
        this.capability = Object.freeze(initialization.capability);
        this.runtimeInstanceId = initialization.runtimeInstanceId;
        worker.on('message', this.#onMessage);
        worker.on('error', (error) => {
            if (this.#closed) return;
            const failure = asError(error);
            onWorkerFailure?.(failure);
            this.#failTransport(failure);
        });
        worker.on('exit', (code) => {
            if (this.#closed) return;
            const error = new Error(`CEM-ML Node command worker exited with code ${code}`);
            onWorkerFailure?.(error);
            this.#failTransport(error);
        });
    }

    static async create(options: NodeCommandServiceClientOptions): Promise<NodeCommandServiceClient> {
        validateOptions(options);
        const timeout = options.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS;
        requireBoundedInteger('startupTimeoutMs', timeout, 1, MAX_STARTUP_TIMEOUT_MS);
        const worker = new Worker(new URL('./node-worker.js', import.meta.url), {
            workerData: { worker: { slot: 1, generation: 1 }, effectiveWorkers: 1 },
        });
        try {
            const initialization = await initializeWorker(worker, timeout);
            return new NodeCommandServiceClient(
                worker,
                initialization,
                options.host,
                options.onWorkerFailure,
            );
        } catch (error) {
            await worker.terminate();
            throw asError(error);
        }
    }

    execute(
        request: CommandServiceRequestV1,
        options: NodeCommandServiceExecuteOptions = {},
    ): NodeCommandServiceHandle {
        this.#requireOpen();
        const executionId = this.#nextExecutionId++;
        const handle = new NodeCommandServiceHandle(this, request.requestId, options);
        this.#executions.set(executionId, handle);
        const message: BrowserCommandExecuteRequest = {
            type: 'cem-command-execute',
            executionId,
            request,
        };
        try {
            this.#worker.postMessage(message);
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
        this.#worker.off('message', this.#onMessage);
        try {
            this.#worker.postMessage({ type: 'cem-worker-close' });
        } finally {
            await this.#worker.terminate();
            this.#rejectPending(
                new NodeCommandServiceError(
                    'cem.node_command.client_closed',
                    'CEM-ML Node command-service client was closed',
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
        options: NodeCommandArtifactReadOptions,
    ): Promise<NodeCommandArtifactReadResult> {
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
            throw new NodeCommandServiceError(
                'cem.node_command.artifact_bytes_missing',
                'CEM-ML Node command artifact response did not include copied bytes',
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

    #onMessage = (message: unknown): void => {
        try {
            if (!isRecord(message) || typeof message.type !== 'string') {
                throw new Error('CEM-ML Node command worker emitted an invalid message');
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
                    void this.#serveCapability(message as unknown as BrowserCommandCapabilityRequest);
                    return;
                default:
                    throw new Error(`CEM-ML Node command worker emitted unexpected ${message.type}`);
            }
        } catch (error) {
            this.#failTransport(asError(error));
        }
    };

    #acceptProgress(message: BrowserCommandProgressMessage): void {
        const handle = this.#executions.get(message.executionId);
        if (handle === undefined) throw new Error('Node command progress targets an unknown execution');
        handle.acceptProgress(message.progress as CommandServiceProgressV1);
    }

    #acceptResult(message: BrowserCommandResultMessage): void {
        const handle = this.#executions.get(message.executionId);
        if (handle === undefined) throw new Error('Node command result targets an unknown execution');
        this.#executions.delete(message.executionId);
        try {
            handle.settle(unwrapResponse<CommandServiceResultV1>(message.result));
        } catch (error) {
            handle.fail(asError(error));
        }
    }

    #acceptAction(message: BrowserCommandActionResultMessage): void {
        const pending = this.#actions.get(message.actionId);
        if (pending === undefined) throw new Error('Node command action result is unexpected');
        this.#actions.delete(message.actionId);
        try {
            unwrapResponse(message.result);
            pending.resolve({
                result: message.result,
                ...(message.bytes === undefined ? {} : { bytes: message.bytes }),
            });
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
        if (!this.#closed) this.#worker.postMessage(response);
    }

    #invokeCapability(message: BrowserCommandCapabilityRequest): Promise<unknown> {
        if (!this.#executions.has(message.executionId)) {
            return Promise.reject(new Error('capability request targets an inactive Node command'));
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

    #action<T>(
        actionId: number,
        message: object,
    ): Promise<{ readonly value: T; readonly bytes?: Uint8Array }> {
        return new Promise<NodeCommandActionResponse>((resolve, reject) => {
            this.#actions.set(actionId, { resolve, reject });
            try {
                this.#worker.postMessage(message);
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
            throw new NodeCommandServiceError(
                'cem.node_command.client_closed',
                'CEM-ML Node command-service client is closed',
            );
        }
    }

    #failTransport(error: Error): void {
        if (this.#closed) return;
        this.#closed = true;
        void this.#worker.terminate();
        this.#rejectPending(error);
    }

    #rejectPending(error: Error): void {
        for (const handle of this.#executions.values()) handle.fail(error);
        this.#executions.clear();
        for (const pending of this.#actions.values()) pending.reject(error);
        this.#actions.clear();
    }
}

export function createNodeCommandServiceClient(
    options: NodeCommandServiceClientOptions,
): Promise<NodeCommandServiceClient> {
    return NodeCommandServiceClient.create(options);
}

function initializeWorker(worker: Worker, timeoutMs: number): Promise<NodeWorkerInitializePayload> {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
            cleanup();
            reject(new Error(`CEM-ML Node command worker initialization exceeded ${timeoutMs}ms`));
        }, timeoutMs);
        const cleanup = (): void => {
            clearTimeout(timer);
            worker.off('message', onMessage);
            worker.off('error', onError);
            worker.off('exit', onExit);
        };
        const onError = (error: Error): void => {
            cleanup();
            reject(error);
        };
        const onExit = (code: number): void => {
            cleanup();
            reject(new Error(`CEM-ML Node command worker exited during initialization with code ${code}`));
        };
        const onMessage = (value: unknown): void => {
            try {
                const payload = validateInitialization(value);
                cleanup();
                resolve(payload);
            } catch (error) {
                cleanup();
                reject(error);
            }
        };
        worker.once('message', onMessage);
        worker.once('error', onError);
        worker.once('exit', onExit);
    });
}

function validateInitialization(value: unknown): NodeWorkerInitializePayload {
    if (
        !isRecord(value) ||
        value.workerProtocolVersion !== WORKER_PROTOCOL_VERSION ||
        !isRecord(value.worker) ||
        value.worker.slot !== 1 ||
        value.worker.generation !== 1 ||
        !isRecord(value.operation) ||
        value.operation.protocolVersion !== OPERATION_PROTOCOL_VERSION ||
        value.operation.kind !== 'initialize' ||
        !isRecord(value.operation.payload)
    ) {
        throw new Error('CEM-ML Node command worker initialization envelope is invalid');
    }
    const payload = value.operation.payload;
    if (
        typeof payload.runtimeInstanceId !== 'string' ||
        typeof payload.commonVersion !== 'string' ||
        !isRecord(payload.capability) ||
        payload.capability.runtime !== 'wasm-node' ||
        payload.capability.executorTopology !== 'node-worker-pool' ||
        payload.capability.effectiveMaxWorkers !== 1
    ) {
        throw new Error('CEM-ML Node command worker capability identity is invalid');
    }
    return payload as unknown as NodeWorkerInitializeEnvelope['operation']['payload'];
}

function validateOptions(options: NodeCommandServiceClientOptions): void {
    if (!isRecord(options) || !isRecord(options.host)) {
        throw new TypeError('Node command-service options.host is required');
    }
    for (const capability of [
        'currentRevision',
        'readResource',
        'prepareWrite',
        'commitWrite',
        'rollbackWrite',
    ] as const) {
        if (typeof options.host[capability] !== 'function') {
            throw new TypeError(`Node command-service host.${capability} must be a function`);
        }
    }
}

function unwrapResponse<T>(value: unknown): T {
    if (isRecord(value) && isRecord(value.error)) {
        const code = typeof value.error.code === 'string' ? value.error.code : 'cem.node_command.unknown';
        const message = typeof value.error.message === 'string' ? value.error.message : 'CEM-ML command failed';
        throw new NodeCommandServiceError(code, message, value.error);
    }
    return value as T;
}

function serializeError(error: unknown): BrowserCommandSerializedError {
    if (error instanceof NodeCommandServiceError) {
        return { name: error.name, message: error.message, code: error.code };
    }
    if (error instanceof Error) return { name: error.name, message: error.message };
    return { name: 'Error', message: String(error) };
}

function abortReason(signal: AbortSignal | undefined): string {
    if (signal?.reason instanceof Error && signal.reason.message.length > 0) return signal.reason.message;
    if (typeof signal?.reason === 'string' && signal.reason.length > 0) return signal.reason;
    return 'Node command aborted';
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
