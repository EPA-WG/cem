import * as runtime from '@epa-wg/cem-ml/wasm';
import runtimeMetadata from '@epa-wg/cem-ml/runtime.json' with { type: 'json' };

import {
    DEFAULT_MAX_BROWSER_WORKERS,
    DEFAULT_STARTUP_TIMEOUT_MS,
    MAX_COORDINATED_WORKERS,
    MAX_STARTUP_TIMEOUT_MS,
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
} from './protocol.js';

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
    readonly worker: Worker;
    readonly descriptor: BrowserWorkerDescriptor;
    readonly initialization: BrowserWorkerInitializePayload;
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
    readonly workers: readonly BrowserWorkerDescriptor[];
    readonly mainThread: BrowserMainThreadDescriptor | undefined;

    #records: readonly WorkerRecord[];
    #closing = false;

    private constructor(
        records: readonly WorkerRecord[],
        mainThread: BrowserMainThreadDescriptor | undefined,
        capability: BrowserWorkerCapabilityManifest | BrowserMainThreadCapabilityManifest,
        mode: BrowserWorkerPoolMode,
        fallbackReason: BrowserWorkerPoolFallbackReason | undefined,
        onWorkerFailure: ((failure: BrowserWorkerFailure) => void) | undefined,
    ) {
        this.#records = records;
        this.mode = mode;
        this.fallbackReason = fallbackReason;
        this.capability = capability;
        this.workers = Object.freeze(records.map((record) => Object.freeze(record.descriptor)));
        this.mainThread = mainThread === undefined ? undefined : Object.freeze(mainThread);
        for (const record of records) {
            record.worker.addEventListener('error', (event) => {
                if (!this.#closing) {
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'worker-error',
                        message: event.message || 'CEM-ML browser worker failed',
                    });
                }
            });
            record.worker.addEventListener('messageerror', () => {
                if (!this.#closing) {
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'message-error',
                        message: 'CEM-ML browser worker emitted an invalid structured-clone message',
                    });
                }
            });
        }
    }

    private static async startMainThreadFallback(
        reason: BrowserWorkerPoolFallbackReason,
        onWorkerFailure: ((failure: BrowserWorkerFailure) => void) | undefined,
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
        );
    }

    get size(): number {
        return this.#records.length === 0 ? 1 : this.#records.length;
    }

    async close(): Promise<void> {
        if (this.#closing) return;
        this.#closing = true;
        for (const record of this.#records) {
            record.worker.postMessage({ type: 'cem-worker-close' });
            record.worker.terminate();
        }
    }

    static async create(options: BrowserWorkerPoolOptions = {}): Promise<BrowserWorkerPool> {
        const plan = workerPlan(options);
        const abiIdentity = runtimeAbiIdentity();
        const runtimeHostId = `browser-host-${nextRuntimeHostId++}`;
        if (typeof Worker !== 'function') {
            return BrowserWorkerPool.startMainThreadFallback('workers-unavailable', options.onWorkerFailure);
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
                    );
                } catch {
                    return BrowserWorkerPool.startMainThreadFallback(
                        'worker-initialization-failed',
                        options.onWorkerFailure,
                    );
                }
            }
            return BrowserWorkerPool.startMainThreadFallback('worker-initialization-failed', options.onWorkerFailure);
        }
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
