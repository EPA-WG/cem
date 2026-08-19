import type { CemProcessingOperation } from './processing-host.js';

export const DEFAULT_MAX_CEM_PROCESSING_WORKERS = 8;
export const DEFAULT_CEM_PROCESSING_QUEUE_SIZE = 64;
const MAX_CEM_PROCESSING_WORKERS = 256;
const MAX_CEM_PROCESSING_QUEUE_SIZE = 65_536;

export interface CemProcessingPoolPolicy {
    /** Exact upper bound of lazily created worker slots. */
    workerCount?: number;
    /** Host policy cap applied to the requested/default worker count. */
    maxWorkers?: number;
    /** Maximum queued operations across one worker slot. */
    queueSize?: number;
}

export interface ResolvedCemProcessingPoolPolicy {
    workerCount: number;
    maxWorkers: number;
    queueSize: number;
}

export type CemProcessingSchedulingTraceKind =
    | 'enqueue'
    | 'dispatch'
    | 'cancel'
    | 'fallback'
    | 'overflow';

export interface CemProcessingSchedulingTraceEvent {
    version: 'cem-processing-schedule-v1';
    sequence: number;
    kind: CemProcessingSchedulingTraceKind;
    ownerScopeId: number;
    scopePolicyStamp: string;
    workerSlot: number;
    jobId: number;
    operation: CemProcessingOperation;
}

export type CemProcessingSchedulingTraceEventInput = Omit<
    CemProcessingSchedulingTraceEvent,
    'version' | 'sequence'
>;

export function resolveCemProcessingPoolPolicy(
    policy: CemProcessingPoolPolicy = {},
    availableWorkers = Math.max(1, Math.floor(globalThis.navigator?.hardwareConcurrency ?? 1))
): ResolvedCemProcessingPoolPolicy {
    const maxWorkers = policy.maxWorkers ?? DEFAULT_MAX_CEM_PROCESSING_WORKERS;
    requireBoundedInteger('maxWorkers', maxWorkers, 1, MAX_CEM_PROCESSING_WORKERS);
    const workerCount = policy.workerCount ?? Math.min(Math.max(1, availableWorkers), maxWorkers);
    requireBoundedInteger('workerCount', workerCount, 1, maxWorkers);
    const queueSize = policy.queueSize ?? DEFAULT_CEM_PROCESSING_QUEUE_SIZE;
    requireBoundedInteger('queueSize', queueSize, 1, MAX_CEM_PROCESSING_QUEUE_SIZE);
    return { workerCount, maxWorkers, queueSize };
}

export interface CemProcessingScheduledEntry<TValue> {
    ownerId: number;
    value: TValue;
}

/** Bounded FIFO queues drained round-robin across their registered root owners. */
export class CemProcessingFairScheduler<TValue> {
    private readonly queues = new Map<number, TValue[]>();
    private readonly ownerOrder: number[] = [];
    private queued = 0;
    private nextOwnerIndex = 0;

    constructor(readonly capacity: number) {
        if (!Number.isSafeInteger(capacity) || capacity < 1) {
            throw new RangeError(`a CEM processing scheduler capacity must be a positive safe integer; received ${capacity}`);
        }
    }

    registerOwner(ownerId: number): void {
        if (this.queues.has(ownerId)) {
            return;
        }
        this.queues.set(ownerId, []);
        this.ownerOrder.push(ownerId);
    }

    enqueue(ownerId: number, value: TValue): void {
        const queue = this.queues.get(ownerId);
        if (!queue) {
            throw new Error(`CEM processing scheduler owner ${ownerId} is not registered`);
        }
        if (this.queued >= this.capacity) {
            throw new Error(`CEM processing scheduler queue capacity ${this.capacity} was exceeded`);
        }
        queue.push(value);
        this.queued += 1;
    }

    dequeue(): CemProcessingScheduledEntry<TValue> | undefined {
        if (this.queued === 0 || this.ownerOrder.length === 0) {
            return undefined;
        }
        for (let checked = 0; checked < this.ownerOrder.length; checked += 1) {
            const index = (this.nextOwnerIndex + checked) % this.ownerOrder.length;
            const ownerId = this.ownerOrder[index];
            const value = this.queues.get(ownerId)?.shift();
            if (value === undefined) {
                continue;
            }
            this.queued -= 1;
            this.nextOwnerIndex = (index + 1) % this.ownerOrder.length;
            return { ownerId, value };
        }
        return undefined;
    }

    cancel(predicate: (entry: CemProcessingScheduledEntry<TValue>) => boolean): CemProcessingScheduledEntry<TValue>[] {
        const removed: CemProcessingScheduledEntry<TValue>[] = [];
        for (const ownerId of this.ownerOrder) {
            const queue = this.queues.get(ownerId);
            if (!queue) {
                continue;
            }
            for (let index = queue.length - 1; index >= 0; index -= 1) {
                const entry = { ownerId, value: queue[index] };
                if (predicate(entry)) {
                    queue.splice(index, 1);
                    this.queued -= 1;
                    removed.unshift(entry);
                }
            }
        }
        return removed;
    }

    removeOwner(ownerId: number): CemProcessingScheduledEntry<TValue>[] {
        const queue = this.queues.get(ownerId);
        if (!queue) {
            return [];
        }
        const index = this.ownerOrder.indexOf(ownerId);
        if (index >= 0) {
            this.ownerOrder.splice(index, 1);
            if (index < this.nextOwnerIndex) {
                this.nextOwnerIndex -= 1;
            }
            if (this.nextOwnerIndex >= this.ownerOrder.length) {
                this.nextOwnerIndex = 0;
            }
        }
        this.queues.delete(ownerId);
        this.queued -= queue.length;
        return queue.map((value) => ({ ownerId, value }));
    }
}

/** In-memory deterministic decision trace; it deliberately contains no clock fields. */
export class CemProcessingSchedulingTrace {
    private readonly events: CemProcessingSchedulingTraceEvent[] = [];
    private sequence = 0;

    record(input: CemProcessingSchedulingTraceEventInput): CemProcessingSchedulingTraceEvent {
        const event = Object.freeze({
            version: 'cem-processing-schedule-v1' as const,
            sequence: ++this.sequence,
            ...input,
        });
        this.events.push(event);
        return event;
    }

    snapshot(): CemProcessingSchedulingTraceEvent[] {
        return this.events.map((event) => ({ ...event }));
    }
}

function requireBoundedInteger(name: string, value: number, minimum: number, maximum: number): void {
    if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        throw new RangeError(`${name}=${value} must be an integer in ${minimum}..${maximum}`);
    }
}
