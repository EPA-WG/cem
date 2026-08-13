import { availableParallelism } from 'node:os';
import { Worker } from 'node:worker_threads';

import {
    DEFAULT_MAX_NODE_WORKERS,
    DEFAULT_STARTUP_TIMEOUT_MS,
    MAX_COORDINATED_WORKERS,
    MAX_STARTUP_TIMEOUT_MS,
    OPERATION_PROTOCOL_VERSION,
    WORKER_PROTOCOL_VERSION,
} from './protocol.js';
import type { NodeWorkerCapabilityManifest, NodeWorkerInitializePayload, WorkerAddress } from './protocol.js';

export type NodeWorkerPoolMode = 'pool' | 'single-worker' | 'single-worker-fallback';

export interface NodeWorkerFailure {
    readonly worker: WorkerAddress;
    readonly code: 'worker-error' | 'worker-exit';
    readonly message: string;
}

export interface NodeWorkerPoolOptions {
    readonly workerCount?: number;
    readonly maxWorkers?: number;
    readonly startupTimeoutMs?: number;
    readonly onWorkerFailure?: (failure: NodeWorkerFailure) => void;
}

export interface NodeWorkerDescriptor extends WorkerAddress {
    readonly runtimeInstanceId: string;
    readonly threadId: number;
    readonly commonVersion: string;
}

interface WorkerRecord {
    readonly worker: Worker;
    readonly descriptor: NodeWorkerDescriptor;
    readonly initialization: NodeWorkerInitializePayload;
}

interface WorkerPlan {
    readonly requested: number;
    readonly startupTimeoutMs: number;
}

export class NodeWorkerPool {
    readonly mode: NodeWorkerPoolMode;
    readonly fallbackReason: string | undefined;
    readonly capability: NodeWorkerCapabilityManifest;
    readonly workers: readonly NodeWorkerDescriptor[];

    #records: readonly WorkerRecord[];
    #closing = false;

    private constructor(
        records: readonly WorkerRecord[],
        mode: NodeWorkerPoolMode,
        fallbackReason: string | undefined,
        onWorkerFailure: ((failure: NodeWorkerFailure) => void) | undefined,
    ) {
        this.#records = records;
        this.mode = mode;
        this.fallbackReason = fallbackReason;
        this.capability = records[0].initialization.capability;
        this.workers = Object.freeze(records.map((record) => Object.freeze(record.descriptor)));
        for (const record of records) {
            record.worker.on('error', (error) => {
                if (!this.#closing) {
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'worker-error',
                        message: error instanceof Error ? error.message : String(error),
                    });
                }
            });
            record.worker.on('exit', (code) => {
                if (!this.#closing) {
                    onWorkerFailure?.({
                        worker: record.descriptor,
                        code: 'worker-exit',
                        message: `CEM-ML worker exited with code ${code}`,
                    });
                }
            });
        }
    }

    get size(): number {
        return this.#records.length;
    }

    async close(): Promise<void> {
        if (this.#closing) return;
        this.#closing = true;
        for (const record of this.#records) {
            record.worker.postMessage({ type: 'cem-worker-close' });
        }
        await Promise.all(this.#records.map(async (record) => record.worker.terminate()));
    }

    static async create(options: NodeWorkerPoolOptions = {}): Promise<NodeWorkerPool> {
        const plan = workerPlan(options);
        try {
            const records = await startWorkerBatch(plan.requested, plan.startupTimeoutMs);
            return new NodeWorkerPool(
                records,
                plan.requested === 1 ? 'single-worker' : 'pool',
                undefined,
                options.onWorkerFailure,
            );
        } catch (error) {
            if (plan.requested === 1) throw error;
            const records = await startWorkerBatch(1, plan.startupTimeoutMs);
            return new NodeWorkerPool(
                records,
                'single-worker-fallback',
                'pool-initialization-failed',
                options.onWorkerFailure,
            );
        }
    }
}

export async function createNodeWorkerPool(options: NodeWorkerPoolOptions = {}): Promise<NodeWorkerPool> {
    return NodeWorkerPool.create(options);
}

function workerPlan(options: NodeWorkerPoolOptions): WorkerPlan {
    const maxWorkers = options.maxWorkers ?? DEFAULT_MAX_NODE_WORKERS;
    requireBoundedInteger('maxWorkers', maxWorkers, 1, MAX_COORDINATED_WORKERS);
    const requested = options.workerCount ?? Math.min(availableParallelism(), maxWorkers);
    requireBoundedInteger('workerCount', requested, 1, maxWorkers);
    const startupTimeoutMs = options.startupTimeoutMs ?? DEFAULT_STARTUP_TIMEOUT_MS;
    requireBoundedInteger('startupTimeoutMs', startupTimeoutMs, 1, MAX_STARTUP_TIMEOUT_MS);
    return { requested, startupTimeoutMs };
}

async function startWorkerBatch(workerCount: number, startupTimeoutMs: number): Promise<readonly WorkerRecord[]> {
    const pending = Array.from({ length: workerCount }, (_, index) =>
        startWorker(
            {
                slot: index + 1,
                generation: 1,
            },
            workerCount,
            startupTimeoutMs,
        ),
    );
    try {
        const records = await Promise.all(pending.map((entry) => entry.initialized));
        validateBatch(records, workerCount);
        return records;
    } catch (error) {
        await Promise.all(pending.map(async (entry) => entry.worker.terminate()));
        throw error;
    }
}

function startWorker(
    address: WorkerAddress,
    effectiveWorkers: number,
    startupTimeoutMs: number,
): { readonly worker: Worker; readonly initialized: Promise<WorkerRecord> } {
    const worker = new Worker(new URL('./node-worker.js', import.meta.url), {
        workerData: { worker: address, effectiveWorkers },
    });
    const initialized = new Promise<WorkerRecord>((resolve, reject) => {
        let settled = false;
        const timeout = setTimeout(() => {
            fail(new Error(`CEM-ML worker ${address.slot}:${address.generation} startup timed out`));
        }, startupTimeoutMs);
        const cleanup = () => {
            clearTimeout(timeout);
            worker.off('message', onMessage);
            worker.off('error', fail);
            worker.off('exit', onExit);
        };
        const fail = (error: Error) => {
            if (settled) return;
            settled = true;
            cleanup();
            reject(error);
        };
        const onExit = (code: number) => {
            fail(new Error(`CEM-ML worker ${address.slot}:${address.generation} exited with code ${code}`));
        };
        const onMessage = (message: unknown) => {
            if (settled) return;
            try {
                const initialization = validateInitialization(message, address, effectiveWorkers);
                settled = true;
                cleanup();
                resolve({
                    worker,
                    initialization,
                    descriptor: {
                        ...address,
                        runtimeInstanceId: initialization.runtimeInstanceId,
                        threadId: initialization.threadId,
                        commonVersion: initialization.commonVersion,
                    },
                });
            } catch (error) {
                fail(error instanceof Error ? error : new Error(String(error)));
            }
        };
        worker.on('message', onMessage);
        worker.on('error', fail);
        worker.on('exit', onExit);
    });
    return { worker, initialized };
}

function validateInitialization(
    value: unknown,
    expectedWorker: WorkerAddress,
    effectiveWorkers: number,
): NodeWorkerInitializePayload {
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
        !isPositiveInteger(payload.threadId) ||
        typeof payload.commonVersion !== 'string' ||
        payload.commonVersion.length === 0 ||
        !validProtocol(payload.protocol) ||
        !validCapability(payload.capability, payload.commonVersion, effectiveWorkers)
    ) {
        throw new Error('CEM-ML worker initialization payload is invalid');
    }
    return payload as unknown as NodeWorkerInitializePayload;
}

function validateBatch(records: readonly WorkerRecord[], effectiveWorkers: number): void {
    const first = records[0].initialization;
    const runtimeInstances = new Set<string>();
    const threadIds = new Set<number>();
    for (const record of records) {
        const current = record.initialization;
        if (
            current.commonVersion !== first.commonVersion ||
            current.capability.abiIdentity !== first.capability.abiIdentity ||
            current.capability.effectiveMaxWorkers !== effectiveWorkers
        ) {
            throw new Error('CEM-ML workers initialized with inconsistent runtime identities');
        }
        if (!runtimeInstances.add(current.runtimeInstanceId) || !threadIds.add(current.threadId)) {
            throw new Error('CEM-ML worker runtime instances must be unique');
        }
    }
}

function validProtocol(value: unknown): boolean {
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

function validCapability(value: unknown, commonVersion: string, effectiveWorkers: number): boolean {
    return (
        isRecord(value) &&
        value.runtime === 'wasm-node' &&
        value.commonVersion === commonVersion &&
        value.executorTopology === 'node-worker-pool' &&
        value.effectiveMaxWorkers === effectiveWorkers &&
        typeof value.abiIdentity === 'string' &&
        value.abiIdentity.length > 0
    );
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
