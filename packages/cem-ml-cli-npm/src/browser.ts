import * as runtime from '@epa-wg/cem-ml/wasm';
import runtimeMetadata from '@epa-wg/cem-ml/runtime.json' with { type: 'json' };

import {
    CemMlOperationHandle,
    OperationHostController,
    type ResumableOperationOptions,
    type ResumableRuntime,
} from './operation.js';
import {
    DEFAULT_MAX_BROWSER_WORKERS,
    DEFAULT_STARTUP_TIMEOUT_MS,
    MAX_HARD_CANCEL_GRACE_MS,
    MAX_COORDINATED_WORKERS,
    MAX_STARTUP_TIMEOUT_MS,
    MIN_HARD_CANCEL_GRACE_MS,
    OPERATION_PROTOCOL_VERSION,
    WORKER_PROTOCOL_VERSION,
} from './protocol.js';
import type {
    BrowserMainThreadCapabilityManifest,
    BrowserWorkerBootstrap,
    BrowserWorkerCapabilityManifest,
    BrowserWorkerInitializePayload,
    WorkerAddress,
    WorkerProtocolDescriptor,
    OperationWorkPacket,
    OperationWorkResult,
    ResumableOperationRunRequest,
} from './protocol.js';

export {
    BrowserCommandServiceClient,
    BrowserCommandServiceError,
    BrowserCommandServiceHandle,
    createBrowserCommandServiceClient,
} from './browser-command.js';
export type {
    BrowserCommandArtifactReadOptions,
    BrowserCommandArtifactReadResult,
    BrowserCommandServiceClientOptions,
    BrowserCommandServiceExecuteOptions,
    BrowserCommandWorkerDescriptor,
    BrowserCommandWorkerFailure,
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
} from './browser-command.js';

export {
    buildBrowserCommandInvocation,
    BrowserCommandInvocationError,
    projectBrowserCommandPresentation,
} from './browser-invocation.js';
export type {
    BrowserCommandInvocationOptions,
    BrowserCommandInvocationResolver,
    BrowserCommandInvocationResource,
} from './browser-invocation.js';
export type {
    CommandInvocationBuildResponseV1,
    CommandInvocationEnvironmentV1,
    CommandInvocationResourceRequirementKindV1,
    CommandInvocationResourceRequirementV1,
    CommandInvocationV1,
    CommandPresentationPlanV1,
    CommandPresentationRouteV1,
    CommandPresentationTargetKindV1,
    CommandPresentationV1,
    CommandPresentationWriteV1,
} from '@epa-wg/cem-ml/wasm';

export {
    CemMlCommandError,
    commandSchema,
    parseCemMlCommand,
    parseCemMlCommandText,
    serializeCemMlCommand,
    serializeCemMlCommandText,
} from './command.js';
export type {
    CommandRuntime,
    ParsedCemMlCommand,
    ParseCemMlCommandOptions,
    SharedCommandSchema,
} from './command.js';

export type BrowserWorkerPoolMode =
    | 'pool'
    | 'single-worker'
    | 'single-worker-fallback'
    | 'main-thread-fallback';

export type BrowserWorkerPoolFallbackReason =
    | 'workers-unavailable'
    | 'pool-initialization-failed'
    | 'worker-initialization-failed';

export interface BrowserWorkerFailure {
    readonly worker: WorkerAddress;
    readonly code: 'worker-error' | 'message-error';
    readonly message: string;
}

export interface BrowserWorkerPoolOptions {
    readonly workerCount?: number;
    readonly maxWorkers?: number;
    readonly startupTimeoutMs?: number;
    readonly onWorkerFailure?: (failure: BrowserWorkerFailure) => void;
    readonly hardCancelGraceMs?: number;
}

export interface BrowserWorkerDescriptor extends WorkerAddress {
    readonly runtimeInstanceId: string;
    readonly commonVersion: string;
}

export interface BrowserMainThreadDescriptor {
    readonly runtimeInstanceId: 'browser-main-thread';
    readonly commonVersion: string;
}

interface WorkerRecord {
    worker: Worker;
    descriptor: BrowserWorkerDescriptor;
    initialization: BrowserWorkerInitializePayload;
    nextSequence: number;
    readonly pending: Map<string, PendingWork>;
}

interface PendingWork {
    readonly packet: OperationWorkPacket;
    readonly resolve: (result: OperationWorkResult) => void;
    readonly reject: (error: Error) => void;
}

interface WorkerCandidate {
    readonly worker: Worker;
    readonly initialized: Promise<WorkerRecord>;
    readonly cancel: (error: Error) => void;
}

interface WorkerPlan {
    readonly requested: number;
    readonly startupTimeoutMs: number;
}

interface RuntimeMetadata {
    readonly abi: {
        readonly identity: string;
    };
}

let nextRuntimeHostId = 1;
const initializeRuntime = (runtime as unknown as { readonly default: () => Promise<unknown> }).default;

export class BrowserWorkerPool {
    readonly mode: BrowserWorkerPoolMode;
    readonly fallbackReason: BrowserWorkerPoolFallbackReason | undefined;
    readonly capability: BrowserWorkerCapabilityManifest | BrowserMainThreadCapabilityManifest;
    readonly mainThread: BrowserMainThreadDescriptor | undefined;

    #records: readonly WorkerRecord[];
    #controller: OperationHostController;
    #mainWorkerAddress: WorkerAddress;
    #closing = false;
    #startupTimeoutMs: number;
    #runtimeHostId: string;
    #abiIdentity: string;
    #onWorkerFailure: ((failure: BrowserWorkerFailure) => void) | undefined;

    private constructor(
        records: readonly WorkerRecord[],
        mainThread: BrowserMainThreadDescriptor | undefined,
        capability: BrowserWorkerCapabilityManifest | BrowserMainThreadCapabilityManifest,
        mode: BrowserWorkerPoolMode,
        fallbackReason: BrowserWorkerPoolFallbackReason | undefined,
        onWorkerFailure: ((failure: BrowserWorkerFailure) => void) | undefined,
        startupTimeoutMs: number,
        runtimeHostId: string,
        abiIdentity: string,
        operationOptions: ResumableOperationOptions,
    ) {
        this.#records = records;
        this.#startupTimeoutMs = startupTimeoutMs;
        this.#runtimeHostId = runtimeHostId;
        this.#abiIdentity = abiIdentity;
        this.#onWorkerFailure = onWorkerFailure;
        this.mode = mode;
        this.fallbackReason = fallbackReason;
        this.capability = capability;
        this.mainThread = mainThread === undefined ? undefined : Object.freeze(mainThread);
        this.#mainWorkerAddress = records[0]?.descriptor ?? { slot: 1, generation: 1 };
        for (const record of records) {
            record.worker.addEventListener('message', (event: MessageEvent<unknown>) => {
                this.#acceptWorkerMessage(record, event.data);
            });
            record.worker.addEventListener('error', (event) => {
                if (!this.#closing) {
                    rejectPending(record, new Error(event.message || 'CEM-ML browser worker failed'));
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'worker-error',
                        message: event.message || 'CEM-ML browser worker failed',
                    });
                }
            });
            record.worker.addEventListener('messageerror', () => {
                if (!this.#closing) {
                    rejectPending(record, new Error('CEM-ML browser worker emitted an invalid message'));
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'message-error',
                        message: 'CEM-ML browser worker emitted an invalid structured-clone message',
                    });
                }
            });
        }
        this.#controller = new OperationHostController(
            runtime as unknown as ResumableRuntime,
            {
                workerAddresses: () =>
                    this.#records.length === 0
                        ? [this.#mainWorkerAddress]
                        : this.#records.map(({ descriptor }) => descriptor),
                execute: (packet) => this.#executeWork(packet),
                replace: (previous, replacement) => this.#replacePhysicalWorker(previous, replacement),
            },
            operationOptions,
        );
    }

    get workers(): readonly BrowserWorkerDescriptor[] {
        return Object.freeze(this.#records.map(({ descriptor }) => Object.freeze({ ...descriptor })));
    }

    private static async startMainThreadFallback(
        reason: BrowserWorkerPoolFallbackReason,
        onWorkerFailure: ((failure: BrowserWorkerFailure) => void) | undefined,
        operationOptions: ResumableOperationOptions,
    ): Promise<BrowserWorkerPool> {
        await initializeRuntime();
        const commonVersion = runtime.version();
        const request = JSON.stringify({
            runtime: 'wasm-browser-worker',
            targetIdentity: 'wasm32-unknown-unknown:web-main-thread',
            abiIdentity: runtimeAbiIdentity(),
            debugControlActive: false,
        });
        const capability = JSON.parse(runtime.capabilityManifest(request)) as BrowserMainThreadCapabilityManifest & {
            readonly error?: unknown;
        };
        if (!validMainThreadCapability(capability, commonVersion)) {
            throw new Error('CEM-ML browser main-thread capability projection is invalid');
        }
        return new BrowserWorkerPool(
            [],
            { runtimeInstanceId: 'browser-main-thread', commonVersion },
            capability,
            'main-thread-fallback',
            reason,
            onWorkerFailure,
            DEFAULT_STARTUP_TIMEOUT_MS,
            `browser-main-${nextRuntimeHostId++}`,
            runtimeAbiIdentity(),
            operationOptions,
        );
    }

    get size(): number {
        return this.#records.length === 0 ? 1 : this.#records.length;
    }

    run(request: ResumableOperationRunRequest): CemMlOperationHandle {
        return this.#controller.start(request);
    }

    async close(): Promise<void> {
        if (this.#closing) return;
        this.#closing = true;
        this.#controller.close();
        for (const record of this.#records) {
            record.worker.postMessage({ type: 'cem-worker-close' });
            record.worker.terminate();
        }
    }

    static async create(options: BrowserWorkerPoolOptions = {}): Promise<BrowserWorkerPool> {
        const plan = workerPlan(options);
        await initializeRuntime();
        const abiIdentity = runtimeAbiIdentity();
        const runtimeHostId = `browser-host-${nextRuntimeHostId++}`;
        if (typeof Worker !== 'function') {
            return BrowserWorkerPool.startMainThreadFallback(
                'workers-unavailable',
                options.onWorkerFailure,
                resumableOptions(options.hardCancelGraceMs),
            );
        }

        try {
            const records = await startWorkerBatch(
                plan.requested,
                plan.startupTimeoutMs,
                runtimeHostId,
                abiIdentity,
            );
            return new BrowserWorkerPool(
                records,
                undefined,
                records[0].initialization.capability,
                plan.requested === 1 ? 'single-worker' : 'pool',
                undefined,
                options.onWorkerFailure,
                plan.startupTimeoutMs,
                runtimeHostId,
                abiIdentity,
                resumableOptions(options.hardCancelGraceMs),
            );
        } catch {
            if (plan.requested > 1) {
                try {
                    const records = await startWorkerBatch(1, plan.startupTimeoutMs, runtimeHostId, abiIdentity);
                    return new BrowserWorkerPool(
                        records,
                        undefined,
                        records[0].initialization.capability,
                        'single-worker-fallback',
                        'pool-initialization-failed',
                        options.onWorkerFailure,
                        plan.startupTimeoutMs,
                        runtimeHostId,
                        abiIdentity,
                        resumableOptions(options.hardCancelGraceMs),
                    );
                } catch {
                    return BrowserWorkerPool.startMainThreadFallback(
                        'worker-initialization-failed',
                        options.onWorkerFailure,
                        resumableOptions(options.hardCancelGraceMs),
                    );
                }
            }
            return BrowserWorkerPool.startMainThreadFallback(
                'worker-initialization-failed',
                options.onWorkerFailure,
                resumableOptions(options.hardCancelGraceMs),
            );
        }
    }

    #executeWork(packet: OperationWorkPacket): Promise<OperationWorkResult> {
        if (this.#records.length === 0) {
            if (!sameWorker(packet.worker, this.#mainWorkerAddress)) {
                return Promise.reject(new Error('CEM-ML main-thread work targets a stale generation'));
            }
            return new Promise((resolve, reject) => {
                setTimeout(() => {
                    try {
                        resolve(parseRuntimeWorkResult((runtime as unknown as ResumableRuntime).executeOperationWork(JSON.stringify(packet))));
                    } catch (error) {
                        reject(error instanceof Error ? error : new Error(String(error)));
                    }
                });
            });
        }
        const record = this.#records.find(({ descriptor }) => sameWorker(descriptor, packet.worker));
        if (record === undefined) {
            return Promise.reject(new Error('CEM-ML work packet targets an unavailable browser worker generation'));
        }
        const key = packetKey(packet);
        if (record.pending.has(key)) {
            return Promise.reject(new Error(`CEM-ML browser work packet ${key} is already pending`));
        }
        return new Promise((resolve, reject) => {
            record.pending.set(key, { packet, resolve, reject });
            record.worker.postMessage({ type: 'cem-operation-work', packet });
        });
    }

    #acceptWorkerMessage(record: WorkerRecord, message: unknown): void {
        try {
            const result = validateWorkResult(message, record);
            const key = packetKey(result);
            const pending = record.pending.get(key);
            if (pending === undefined) throw new Error(`CEM-ML browser worker result ${key} is unexpected`);
            record.pending.delete(key);
            pending.resolve(result);
        } catch (error) {
            rejectPending(record, error instanceof Error ? error : new Error(String(error)));
        }
    }

    async #replacePhysicalWorker(previous: WorkerAddress, replacement: WorkerAddress): Promise<void> {
        if (this.#records.length === 0) {
            this.#mainWorkerAddress = replacement;
            return;
        }
        const index = this.#records.findIndex(({ descriptor }) => sameWorker(descriptor, previous));
        if (index < 0) throw new Error('CEM-ML cannot replace an unknown browser worker generation');
        const current = this.#records[index];
        rejectPending(current, new Error('CEM-ML browser worker was replaced'));
        current.worker.terminate();
        const candidate = startWorker(
            replacement,
            this.size,
            this.#startupTimeoutMs,
            this.#runtimeHostId,
            this.#abiIdentity,
        );
        const next = await candidate.initialized;
        next.worker.addEventListener('message', (event: MessageEvent<unknown>) => {
            this.#acceptWorkerMessage(next, event.data);
        });
        next.worker.addEventListener('error', (event) => {
            if (!this.#closing) {
                rejectPending(next, new Error(event.message || 'CEM-ML browser worker failed'));
                this.#onWorkerFailure?.({
                    worker: next.descriptor,
                    code: 'worker-error',
                    message: event.message || 'CEM-ML browser worker failed',
                });
            }
        });
        next.worker.addEventListener('messageerror', () => {
            if (!this.#closing) {
                rejectPending(next, new Error('CEM-ML browser worker emitted an invalid message'));
                this.#onWorkerFailure?.({
                    worker: next.descriptor,
                    code: 'message-error',
                    message: 'CEM-ML browser worker emitted an invalid structured-clone message',
                });
            }
        });
        (this.#records as WorkerRecord[])[index] = next;
    }
}

export async function createBrowserWorkerPool(options: BrowserWorkerPoolOptions = {}): Promise<BrowserWorkerPool> {
    return BrowserWorkerPool.create(options);
}

function workerPlan(options: BrowserWorkerPoolOptions): WorkerPlan {
    const maxWorkers = options.maxWorkers ?? DEFAULT_MAX_BROWSER_WORKERS;
    requireBoundedInteger('maxWorkers', maxWorkers, 1, MAX_COORDINATED_WORKERS);
    const availableWorkers = Math.max(1, Math.floor(globalThis.navigator?.hardwareConcurrency ?? 1));
    const requested = options.workerCount ?? Math.min(availableWorkers, maxWorkers);
    requireBoundedInteger('workerCount', requested, 1, maxWorkers);
    const startupTimeoutMs = options.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS;
    requireBoundedInteger('startupTimeoutMs', startupTimeoutMs, 1, MAX_STARTUP_TIMEOUT_MS);
    if (options.hardCancelGraceMs !== undefined) {
        requireBoundedInteger(
            'hardCancelGraceMs',
            options.hardCancelGraceMs,
            MIN_HARD_CANCEL_GRACE_MS,
            MAX_HARD_CANCEL_GRACE_MS,
        );
    }
    return { requested, startupTimeoutMs };
}

async function startWorkerBatch(
    workerCount: number,
    startupTimeoutMs: number,
    runtimeHostId: string,
    abiIdentity: string,
): Promise<readonly WorkerRecord[]> {
    const pending: WorkerCandidate[] = [];
    try {
        for (let index = 0; index < workerCount; index += 1) {
            pending.push(
                startWorker(
                    {
                        slot: index + 1,
                        generation: 1,
                    },
                    workerCount,
                    startupTimeoutMs,
                    runtimeHostId,
                    abiIdentity,
                ),
            );
        }
        const records = await Promise.all(pending.map((entry) => entry.initialized));
        validateBatch(records, workerCount);
        return records;
    } catch (error) {
        const failure = error instanceof Error ? error : new Error(String(error));
        for (const entry of pending) {
            entry.cancel(failure);
            entry.worker.terminate();
        }
        throw failure;
    }
}

function startWorker(
    address: WorkerAddress,
    effectiveWorkers: number,
    startupTimeoutMs: number,
    runtimeHostId: string,
    abiIdentity: string,
): WorkerCandidate {
    const worker = new Worker(new URL('./browser-worker.js', import.meta.url), {
        name: `cem-ml-${address.slot}-${address.generation}`,
        type: 'module',
    });
    let cancelInitialization: (error: Error) => void = () => undefined;
    const initialized = new Promise<WorkerRecord>((resolve, reject) => {
        let settled = false;
        const timeout = setTimeout(() => {
            fail(new Error(`CEM-ML worker ${address.slot}:${address.generation} startup timed out`));
        }, startupTimeoutMs);
        const cleanup = () => {
            clearTimeout(timeout);
            worker.removeEventListener('message', onMessage);
            worker.removeEventListener('error', onError);
            worker.removeEventListener('messageerror', onMessageError);
        };
        const fail = (error: Error) => {
            if (settled) return;
            settled = true;
            cleanup();
            reject(error);
        };
        cancelInitialization = fail;
        const onError = (event: ErrorEvent) => {
            fail(new Error(event.message || `CEM-ML worker ${address.slot}:${address.generation} failed`));
        };
        const onMessageError = () => {
            fail(new Error(`CEM-ML worker ${address.slot}:${address.generation} initialization was not cloneable`));
        };
        const onMessage = (event: MessageEvent<unknown>) => {
            if (settled) return;
            try {
                const initialization = validateInitialization(event.data, address, effectiveWorkers);
                settled = true;
                cleanup();
                resolve({
                    worker,
                    initialization,
                    nextSequence: 2,
                    pending: new Map(),
                    descriptor: {
                        ...address,
                        runtimeInstanceId: initialization.runtimeInstanceId,
                        commonVersion: initialization.commonVersion,
                    },
                });
            } catch (error) {
                fail(error instanceof Error ? error : new Error(String(error)));
            }
        };
        worker.addEventListener('message', onMessage);
        worker.addEventListener('error', onError);
        worker.addEventListener('messageerror', onMessageError);

        const bootstrap: BrowserWorkerBootstrap = {
            type: 'cem-worker-initialize',
            worker: address,
            effectiveWorkers,
            runtimeHostId,
            abiIdentity,
        };
        worker.postMessage(bootstrap);
    });
    return { worker, initialized, cancel: (error) => cancelInitialization(error) };
}

function validateInitialization(
    value: unknown,
    expectedWorker: WorkerAddress,
    effectiveWorkers: number,
): BrowserWorkerInitializePayload {
    if (!isRecord(value)) throw new Error('CEM-ML worker initialization must be an object');
    if (value.workerProtocolVersion !== WORKER_PROTOCOL_VERSION) {
        throw new Error('CEM-ML worker protocol version mismatch');
    }
    if (!sameWorker(value.worker, expectedWorker) || value.sequence !== 1) {
        throw new Error('CEM-ML worker initialization address or sequence mismatch');
    }
    if (!Array.isArray(value.transfers) || value.transfers.length !== 0) {
        throw new Error('CEM-ML worker initialization must not contain transfers');
    }
    if (!isRecord(value.operation)) throw new Error('CEM-ML operation envelope is missing');
    if (
        value.operation.protocolVersion !== OPERATION_PROTOCOL_VERSION ||
        value.operation.kind !== 'initialize' ||
        !isRecord(value.operation.payload)
    ) {
        throw new Error('CEM-ML operation initialization envelope is invalid');
    }
    const payload = value.operation.payload;
    if (
        typeof payload.runtimeInstanceId !== 'string' ||
        payload.runtimeInstanceId.length === 0 ||
        typeof payload.commonVersion !== 'string' ||
        payload.commonVersion.length === 0 ||
        !validProtocol(payload.protocol) ||
        !validWorkerCapability(payload.capability, payload.commonVersion, effectiveWorkers)
    ) {
        throw new Error('CEM-ML browser worker initialization payload is invalid');
    }
    return payload as unknown as BrowserWorkerInitializePayload;
}

function validateBatch(records: readonly WorkerRecord[], effectiveWorkers: number): void {
    const first = records[0].initialization;
    const runtimeInstances = new Set<string>();
    for (const record of records) {
        const current = record.initialization;
        if (
            current.commonVersion !== first.commonVersion ||
            current.capability.abiIdentity !== first.capability.abiIdentity ||
            current.capability.effectiveMaxWorkers !== effectiveWorkers
        ) {
            throw new Error('CEM-ML browser workers initialized with inconsistent runtime identities');
        }
        if (!runtimeInstances.add(current.runtimeInstanceId)) {
            throw new Error('CEM-ML browser worker runtime instances must be unique');
        }
    }
}

function validateWorkResult(value: unknown, record: WorkerRecord): OperationWorkResult {
    if (!isRecord(value)) throw new Error('CEM-ML browser worker result must be an object');
    if (value.workerProtocolVersion !== WORKER_PROTOCOL_VERSION) {
        throw new Error('CEM-ML browser worker result protocol version mismatch');
    }
    if (!sameWorker(value.worker, record.descriptor) || value.sequence !== record.nextSequence) {
        throw new Error('CEM-ML browser worker result address or sequence mismatch');
    }
    if (!Array.isArray(value.transfers) || value.transfers.length !== 0) {
        throw new Error('CEM-ML browser worker result unexpectedly contains transfers');
    }
    if (!isRecord(value.operation)) throw new Error('CEM-ML browser worker result envelope is missing');
    if (
        value.operation.protocolVersion !== OPERATION_PROTOCOL_VERSION ||
        value.operation.kind !== 'result' ||
        !isPositiveInteger(value.operation.operationId) ||
        !isPositiveInteger(value.operation.sequence) ||
        !isRecord(value.operation.payload)
    ) {
        throw new Error('CEM-ML browser worker result envelope is invalid');
    }
    const result = value.operation.payload;
    if (
        result.workProtocolVersion !== 1 ||
        result.operationId !== value.operation.operationId ||
        result.taskId !== value.operation.sequence ||
        !sameWorker(result.worker, record.descriptor) ||
        !isPositiveInteger(result.attempt) ||
        !isPositiveInteger(result.commitSequence) ||
        !isRecord(result.stage) ||
        !['succeeded', 'failed', 'cancelled'].includes(String(result.status))
    ) {
        throw new Error('CEM-ML browser worker result payload metadata is invalid');
    }
    record.nextSequence += 1;
    return result as unknown as OperationWorkResult;
}

function parseRuntimeWorkResult(json: string): OperationWorkResult {
    const value = JSON.parse(json) as unknown;
    if (isRecord(value) && isRecord(value.error)) {
        throw new Error(
            typeof value.error.message === 'string' ? value.error.message : 'CEM-ML main-thread work failed',
        );
    }
    return value as OperationWorkResult;
}

function rejectPending(record: WorkerRecord, error: Error): void {
    for (const pending of record.pending.values()) pending.reject(error);
    record.pending.clear();
}

function packetKey(packet: Pick<OperationWorkPacket, 'operationId' | 'taskId' | 'attempt'>): string {
    return `${packet.operationId}:${packet.taskId}:${packet.attempt}`;
}

function resumableOptions(hardCancelGraceMs: number | undefined): ResumableOperationOptions {
    return hardCancelGraceMs === undefined ? {} : { hardCancelGraceMs };
}

function validProtocol(value: unknown): value is WorkerProtocolDescriptor {
    return (
        isRecord(value) &&
        value.workerProtocolVersion === WORKER_PROTOCOL_VERSION &&
        value.operationProtocolVersion === OPERATION_PROTOCOL_VERSION &&
        isRecord(value.limits) &&
        value.limits.maxWorkers === MAX_COORDINATED_WORKERS &&
        isPositiveInteger(value.limits.maxTransferBuffersPerMessage) &&
        isPositiveInteger(value.limits.maxTransferBytesPerMessage)
    );
}

function validWorkerCapability(value: unknown, commonVersion: string, effectiveWorkers: number): boolean {
    return (
        isRecord(value) &&
        value.runtime === 'wasm-browser-worker' &&
        value.commonVersion === commonVersion &&
        value.executorTopology === 'browser-worker-pool' &&
        value.effectiveMaxWorkers === effectiveWorkers &&
        typeof value.abiIdentity === 'string' &&
        value.abiIdentity.length > 0
    );
}

function validMainThreadCapability(value: unknown, commonVersion: string): value is BrowserMainThreadCapabilityManifest {
    return (
        isRecord(value) &&
        value.error === undefined &&
        value.runtime === 'wasm-browser-worker' &&
        value.commonVersion === commonVersion &&
        value.executorTopology === 'sequential' &&
        value.effectiveMaxWorkers === 1 &&
        typeof value.abiIdentity === 'string' &&
        value.abiIdentity.length > 0
    );
}

function runtimeAbiIdentity(): string {
    const metadata = runtimeMetadata as RuntimeMetadata;
    const identity = metadata.abi?.identity;
    if (typeof identity !== 'string' || identity.length === 0) {
        throw new Error('CEM-ML runtime metadata does not provide an ABI identity');
    }
    return identity;
}

function sameWorker(value: unknown, expected: WorkerAddress): boolean {
    return isRecord(value) && value.slot === expected.slot && value.generation === expected.generation;
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isPositiveInteger(value: unknown): value is number {
    return typeof value === 'number' && Number.isSafeInteger(value) && value > 0;
}

function requireBoundedInteger(field: string, value: number, minimum: number, maximum: number): void {
    if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        throw new RangeError(`${field}=${value} is outside ${minimum}..=${maximum}`);
    }
}
