import type {
    OperationWorkPacket,
    OperationWorkResult,
    ResumableOperationEvent,
    ResumableOperationRunRequest,
    ResumableOperationTerminal,
    ResumableWorkerReplacement,
    WorkerAddress,
} from './protocol.js';
import {
    DEFAULT_HARD_CANCEL_GRACE_MS,
    MAX_HARD_CANCEL_GRACE_MS,
    MIN_HARD_CANCEL_GRACE_MS,
} from './protocol.js';

export interface ResumableRuntime {
    readonly initializeResumableOperationHost: (workerCount: number) => string;
    readonly disposeResumableOperationHost: (hostId: number) => string;
    readonly startResumableOperation: (hostId: number, requestJson: string) => string;
    readonly pollResumableOperation: (hostId: number, operationId: string, maxPackets: number) => string;
    readonly acceptResumableOperationResult: (hostId: number, resultJson: string) => string;
    readonly cancelResumableOperation: (hostId: number, operationId: string, reason?: string) => string;
    readonly replaceResumableOperationWorker: (hostId: number, slot: number) => string;
    readonly executeOperationWork: (packetJson: string) => string;
    readonly pauseResumableOperation: (hostId: number, operationId: string, generation: number) => string;
    readonly acknowledgeResumableOperationStop: (
        hostId: number,
        operationId: string,
        stopGeneration: number,
        workerSlot: number,
        workerGeneration: number,
        externalWait: boolean,
    ) => string;
    readonly continueResumableOperation: (hostId: number, operationId: string, generation: number) => string;
    readonly stepResumableOperation: (
        hostId: number,
        operationId: string,
        currentGeneration: number,
        nextGeneration: number,
    ) => string;
}

export interface OperationExecutor {
    readonly workerAddresses: () => readonly WorkerAddress[];
    readonly execute: (packet: OperationWorkPacket) => Promise<OperationWorkResult>;
    readonly replace: (previous: WorkerAddress, replacement: WorkerAddress) => Promise<void>;
}

export interface ResumableOperationOptions {
    readonly hardCancelGraceMs?: number;
}

export type ResumableOperationListener = (event: ResumableOperationEvent) => void;
type OperationHandleState = 'running' | 'pause-requested' | 'paused' | 'stepping' | 'cancelling' | 'terminal';

export class CemMlOperationError extends Error {
    readonly code: string;

    constructor(code: string, message: string) {
        super(message);
        this.name = 'CemMlOperationError';
        this.code = code;
    }
}

export class CemMlOperationHandle implements PromiseLike<unknown> {
    readonly operationId: string;

    #host: OperationHostController;
    #terminal: Promise<ResumableOperationTerminal>;
    #resolveTerminal!: (terminal: ResumableOperationTerminal) => void;
    #rejectTerminal!: (error: unknown) => void;
    #state: OperationHandleState = 'running';
    #inflight = new Map<string, { readonly packet: OperationWorkPacket; readonly promise: Promise<void> }>();
    #listeners = new Set<ResumableOperationListener>();
    #stopGeneration = 0;
    #pumping = false;
    #pumpRequested = false;
    #disposed = false;

    constructor(host: OperationHostController, operationId: string) {
        this.#host = host;
        this.operationId = operationId;
        this.#terminal = new Promise((resolve, reject) => {
            this.#resolveTerminal = resolve;
            this.#rejectTerminal = reject;
        });
        void this.#pump();
    }

    then<TResult1 = unknown, TResult2 = never>(
        onfulfilled?: ((value: unknown) => TResult1 | PromiseLike<TResult1>) | null,
        onrejected?: ((reason: unknown) => TResult2 | PromiseLike<TResult2>) | null,
    ): PromiseLike<TResult1 | TResult2> {
        return this.#terminal.then(terminalValue, terminalError).then(onfulfilled, onrejected);
    }

    result(): Promise<ResumableOperationTerminal> {
        return this.#terminal;
    }

    subscribe(listener: ResumableOperationListener): () => void {
        if (this.#disposed) throw new CemMlOperationError('cem.operation.disposed', 'operation handle is disposed');
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    }

    async cancel(reason?: string): Promise<ResumableOperationTerminal> {
        if (this.#state === 'terminal') return this.#terminal;
        if (this.#state === 'cancelling') return this.#terminal;
        this.#state = 'cancelling';
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
        const running = [...this.#inflight.values()];
        if (running.length > 0) {
            const settled = Promise.allSettled(running.map(({ promise }) => promise));
            const completed = await Promise.race([
                settled.then(() => true),
                delay(this.#host.hardCancelGraceMs).then(() => false),
            ]);
            if (!completed) {
                const workers = new Map(
                    running.map(({ packet }) => [packet.worker.slot, packet.worker] as const),
                );
                for (const worker of workers.values()) await this.#hardReplace(worker, true);
            }
        }
        const terminal = this.#host.runtimeCall<ResumableOperationTerminal>('cancelResumableOperation',
            this.#host.hostId,
            this.operationId,
            reason,
        );
        this.#settle(terminal);
        return terminal;
    }

    async pause(): Promise<void> {
        this.#requireState('running', 'pause');
        this.#state = 'pause-requested';
        this.#stopGeneration += 1;
        this.#host.runtimeCall('pauseResumableOperation', this.#host.hostId, this.operationId, this.#stopGeneration);
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
        await this.#settleInflight();
        this.#acknowledgeAllStop(this.#stopGeneration);
        this.#state = 'paused';
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
    }

    async continue(): Promise<void> {
        this.#requireState('paused', 'continue');
        this.#host.runtimeCall(
            'continueResumableOperation',
            this.#host.hostId,
            this.operationId,
            this.#stopGeneration,
        );
        this.#state = 'running';
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
        await this.#pump();
    }

    async step(): Promise<void> {
        this.#requireState('paused', 'step');
        const previousGeneration = this.#stopGeneration;
        this.#stopGeneration += 1;
        this.#host.runtimeCall(
            'stepResumableOperation',
            this.#host.hostId,
            this.operationId,
            previousGeneration,
            this.#stopGeneration,
        );
        this.#state = 'stepping';
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
        await this.#pump();
        await this.#settleInflight();
        if (this.#isTerminal()) return;
        this.#acknowledgeAllStop(this.#stopGeneration);
        this.#state = 'paused';
        this.#emit({ kind: 'state', operationId: this.operationId, state: this.#state });
    }

    dispose(): void {
        this.#listeners.clear();
        this.#disposed = true;
    }

    async #pump(): Promise<void> {
        if (this.#pumping) {
            this.#pumpRequested = true;
            return;
        }
        if (!['running', 'stepping'].includes(this.#state)) return;
        this.#pumping = true;
        try {
            do {
                this.#pumpRequested = false;
                const poll = this.#host.runtimeCall<{
                    readonly state: string;
                    readonly packets: readonly OperationWorkPacket[];
                    readonly terminal?: ResumableOperationTerminal;
                }>('pollResumableOperation', this.#host.hostId, this.operationId, 64);
                if (poll.terminal !== undefined) {
                    this.#settle(poll.terminal);
                    return;
                }
                for (const packet of poll.packets) this.#dispatch(packet);
            } while (this.#pumpRequested && ['running', 'stepping'].includes(this.#state));
        } catch (error) {
            this.#fail(error);
        } finally {
            this.#pumping = false;
        }
    }

    #dispatch(packet: OperationWorkPacket): void {
        const key = packetKey(packet);
        this.#emit({
            kind: 'dispatch',
            operationId: this.operationId,
            taskId: String(packet.taskId),
            stage: packet.stage,
            worker: packet.worker,
        });
        const promise = this.#host.executor
            .execute(packet)
            .then(
                async (result) => this.#accept(result),
                async (error) => this.#replaceFailedPacket(packet, error),
            )
            .catch((error) => this.#fail(error))
            .finally(() => {
                this.#inflight.delete(key);
            });
        this.#inflight.set(key, { packet, promise });
    }

    async #accept(result: OperationWorkResult): Promise<void> {
        if (this.#state === 'cancelling' || this.#state === 'terminal') return;
        const acceptance = this.#host.runtimeCall<{
            readonly state: string;
            readonly committedTaskIds: readonly number[];
            readonly terminal?: ResumableOperationTerminal;
        }>('acceptResumableOperationResult', this.#host.hostId, JSON.stringify(result));
        this.#emit({
            kind: 'commit',
            operationId: this.operationId,
            taskIds: acceptance.committedTaskIds.map(String),
        });
        if (acceptance.terminal !== undefined) {
            this.#settle(acceptance.terminal);
            return;
        }
        if (acceptance.state === 'pause-requested' && this.#state === 'stepping') return;
        await this.#pump();
    }

    async #replaceFailedPacket(packet: OperationWorkPacket, error: unknown): Promise<void> {
        if (this.#state === 'cancelling' || this.#state === 'terminal') return;
        try {
            await this.#hardReplace(packet.worker, false);
        } catch (replacementError) {
            this.#fail(replacementError instanceof Error ? replacementError : error);
        }
    }

    async #hardReplace(worker: WorkerAddress, suppressCurrent: boolean): Promise<ResumableWorkerReplacement> {
        const replacement = await this.#host.replaceWorker(
            worker,
            suppressCurrent ? this.operationId : undefined,
        );
        this.#emit({
            kind: 'worker-replaced',
            operationId: this.operationId,
            previous: replacement.previous,
            replacement: replacement.replacement,
        });
        return replacement;
    }

    dispatchRetry(packet: OperationWorkPacket): void {
        if (this.#state === 'cancelling' || this.#state === 'terminal') return;
        this.#dispatch(packet);
    }

    #acknowledgeAllStop(generation: number): void {
        for (const worker of this.#host.executor.workerAddresses()) {
            this.#host.runtimeCall(
                'acknowledgeResumableOperationStop',
                this.#host.hostId,
                this.operationId,
                generation,
                worker.slot,
                worker.generation,
                true,
            );
        }
    }

    async #settleInflight(): Promise<void> {
        while (this.#inflight.size > 0) {
            await Promise.allSettled([...this.#inflight.values()].map(({ promise }) => promise));
        }
    }

    #settle(terminal: ResumableOperationTerminal): void {
        if (this.#state === 'terminal') return;
        this.#state = 'terminal';
        this.#resolveTerminal(terminal);
        this.#emit({ kind: 'terminal', operationId: this.operationId, terminal });
    }

    #fail(error: unknown): void {
        if (this.#state === 'terminal') return;
        this.#state = 'terminal';
        this.#rejectTerminal(error);
    }

    #emit(event: ResumableOperationEvent): void {
        if (this.#disposed) return;
        for (const listener of this.#listeners) listener(event);
    }

    #requireState(expected: OperationHandleState, action: string): void {
        if (this.#state !== expected) {
            throw new CemMlOperationError(
                'cem.operation.state',
                `operation ${this.operationId} cannot ${action} while ${this.#state}`,
            );
        }
    }

    #isTerminal(): boolean {
        return this.#state === 'terminal';
    }
}

export class OperationHostController {
    readonly runtime: ResumableRuntime;
    readonly executor: OperationExecutor;
    readonly hostId: number;
    readonly hardCancelGraceMs: number;

    #closed = false;
    #handles = new Map<string, CemMlOperationHandle>();
    #replacements = new Map<number, Promise<ResumableWorkerReplacement>>();

    constructor(runtime: ResumableRuntime, executor: OperationExecutor, options: ResumableOperationOptions = {}) {
        this.runtime = runtime;
        this.executor = executor;
        this.hardCancelGraceMs = options.hardCancelGraceMs ?? DEFAULT_HARD_CANCEL_GRACE_MS;
        requireBoundedInteger(
            'hardCancelGraceMs',
            this.hardCancelGraceMs,
            MIN_HARD_CANCEL_GRACE_MS,
            MAX_HARD_CANCEL_GRACE_MS,
        );
        const initialized = this.runtimeCall<{ readonly hostId: number }>(
            'initializeResumableOperationHost',
            executor.workerAddresses().length,
        );
        this.hostId = initialized.hostId;
    }

    start(request: ResumableOperationRunRequest): CemMlOperationHandle {
        if (this.#closed) throw new CemMlOperationError('cem.operation.host_closed', 'operation host is closed');
        const started = this.runtimeCall<{ readonly operationId: number }>(
            'startResumableOperation',
            this.hostId,
            JSON.stringify(request),
        );
        const handle = new CemMlOperationHandle(this, String(started.operationId));
        this.#handles.set(handle.operationId, handle);
        return handle;
    }

    close(): void {
        if (this.#closed) return;
        this.#closed = true;
        this.runtimeCall('disposeResumableOperationHost', this.hostId);
    }

    runtimeCall<T = unknown>(method: keyof ResumableRuntime, ...args: readonly unknown[]): T {
        const fn = this.runtime[method] as (...runtimeArgs: readonly unknown[]) => string;
        const value = JSON.parse(fn(...args)) as unknown;
        if (isRecord(value) && isRecord(value.error)) {
            throw new CemMlOperationError(
                typeof value.error.code === 'string' ? value.error.code : 'cem.operation.unknown',
                typeof value.error.message === 'string' ? value.error.message : 'CEM-ML operation failed',
            );
        }
        return value as T;
    }

    replaceWorker(
        expected: WorkerAddress,
        suppressedOperationId?: string,
    ): Promise<ResumableWorkerReplacement> {
        const existing = this.#replacements.get(expected.slot);
        if (existing !== undefined) return existing;
        const current = this.executor.workerAddresses().find(({ slot }) => slot === expected.slot);
        if (current === undefined) {
            return Promise.reject(new Error(`CEM-ML worker slot ${expected.slot} is unavailable`));
        }
        if (current.generation !== expected.generation) {
            return Promise.reject(
                new Error(
                    `CEM-ML worker ${expected.slot}:${expected.generation} was already replaced by generation ${current.generation}`,
                ),
            );
        }
        const pending = (async () => {
            const replacement = this.runtimeCall<ResumableWorkerReplacement>(
                'replaceResumableOperationWorker',
                this.hostId,
                expected.slot,
            );
            await this.executor.replace(replacement.previous, replacement.replacement);
            for (const packet of replacement.retryPackets) {
                const operationId = String(packet.operationId);
                if (operationId !== suppressedOperationId) this.#handles.get(operationId)?.dispatchRetry(packet);
            }
            return replacement;
        })();
        this.#replacements.set(expected.slot, pending);
        void pending.then(
            () => this.#replacements.delete(expected.slot),
            () => this.#replacements.delete(expected.slot),
        );
        return pending;
    }
}

function terminalValue(terminal: ResumableOperationTerminal): unknown {
    if (terminal.status === 'succeeded') return terminal.result;
    throw terminalError(terminal);
}

function terminalError(terminal: ResumableOperationTerminal): Error {
    if (terminal.status === 'cancelled') {
        return new DOMException(terminal.reason ?? 'CEM-ML operation cancelled', 'AbortError');
    }
    return new CemMlOperationError(
        terminal.error?.code ?? `cem.operation.${terminal.status}`,
        terminal.error?.message ?? `CEM-ML operation ${terminal.status}`,
    );
}

function packetKey(packet: OperationWorkPacket): string {
    return `${packet.operationId}:${packet.taskId}:${packet.attempt}`;
}

function delay(milliseconds: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function requireBoundedInteger(field: string, value: number, minimum: number, maximum: number): void {
    if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
        throw new RangeError(`${field}=${value} is outside ${minimum}..=${maximum}`);
    }
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}
