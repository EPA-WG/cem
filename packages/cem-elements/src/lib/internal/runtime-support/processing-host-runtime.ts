import {
    onCemDeclarationScopeDispose,
    type CemDeclarationScope,
} from '../../declaration-scope.js';
import { CemProcessingEngine } from './processing-engine.js';
import {
    CemProcessingCancellationRegistry,
    CemProcessingJobSequence,
    assertCemProcessingEnvelope,
    cemProcessingHostOwnerScope,
    createCemProcessingReadyEnvelope,
    createCemProcessingRequestEnvelope,
    decideCemProcessingWorkerFailure,
    defaultCemProcessingWorkerFactory,
    type CemProcessingCancelInput,
    type CemProcessingCancelResult,
    type CemProcessingCompileInput,
    type CemProcessingCompileResult,
    type CemProcessingDiagnostic,
    type CemProcessingDisposeInput,
    type CemProcessingDisposeResult,
    type CemProcessingHost,
    type CemProcessingJob,
    type CemProcessingOperation,
    type CemProcessingReadyEnvelope,
    type CemProcessingRenderDiffInput,
    type CemProcessingRenderDiffResult,
    type CemProcessingRequestEnvelope,
    type CemProcessingResponseEnvelope,
    type CemProcessingSuccessEnvelope,
    type CemProcessingWorkerFactory,
} from './processing-host.js';
import {
    CemProcessingFairScheduler,
    CemProcessingSchedulingTrace,
    resolveCemProcessingPoolPolicy,
    type CemProcessingPoolPolicy,
    type CemProcessingSchedulingTraceEvent,
    type CemProcessingSchedulingTraceKind,
    type ResolvedCemProcessingPoolPolicy,
} from './processing-scheduler.js';

export interface CemProcessingHostRuntimeOptions {
    workerScriptUrl: string | URL;
    workerFactory?: CemProcessingWorkerFactory;
    poolPolicy?: CemProcessingPoolPolicy;
    onTrace?: (event: CemProcessingSchedulingTraceEvent) => void;
}

interface PendingWorkerRequest {
    operation: CemProcessingOperation;
    resolve(response: CemProcessingSuccessEnvelope): void;
    reject(error: unknown): void;
}

class CemProcessingWorkerTransport {
    readonly ready: Promise<CemProcessingReadyEnvelope>;

    private readonly worker: Worker;
    private readonly pending = new Map<number, PendingWorkerRequest>();
    private readySettled = false;
    private failed = false;
    private resolveReady!: (ready: CemProcessingReadyEnvelope) => void;
    private rejectReady!: (error: unknown) => void;

    constructor(options: CemProcessingHostRuntimeOptions, name: string) {
        const factory = options.workerFactory ?? defaultCemProcessingWorkerFactory;
        this.ready = new Promise<CemProcessingReadyEnvelope>((resolve, reject) => {
            this.resolveReady = resolve;
            this.rejectReady = reject;
        });
        this.worker = factory({ scriptUrl: options.workerScriptUrl, name, type: 'module' });
        this.worker.addEventListener('message', this.onMessage);
        this.worker.addEventListener('error', this.onError);
        this.worker.addEventListener('messageerror', this.onMessageError);
    }

    async request(request: CemProcessingRequestEnvelope): Promise<CemProcessingSuccessEnvelope> {
        await this.ready;
        if (this.failed) {
            throw new CemProcessingWorkerTransportError('execution', 'the CEM processing worker transport failed');
        }
        return new Promise<CemProcessingSuccessEnvelope>((resolve, reject) => {
            this.pending.set(request.jobId, { operation: request.operation, resolve, reject });
            try {
                this.worker.postMessage(request);
            } catch (error) {
                this.pending.delete(request.jobId);
                reject(this.fail('execution', error));
            }
        });
    }

    terminate(): void {
        this.detach();
        this.worker.terminate();
    }

    private readonly onMessage = (event: MessageEvent<unknown>): void => {
        try {
            assertCemProcessingEnvelope(event.data);
            const envelope = event.data;
            if (envelope.direction === 'ready') {
                if (this.readySettled || envelope.mode !== 'worker') {
                    throw new TypeError('the CEM processing worker sent an invalid ready transition');
                }
                this.readySettled = true;
                this.resolveReady(envelope);
                return;
            }
            if (envelope.direction !== 'response') {
                throw new TypeError('the CEM processing worker sent a non-response envelope after startup');
            }
            const pending = this.pending.get(envelope.jobId);
            if (!pending || pending.operation !== envelope.operation) {
                return;
            }
            this.pending.delete(envelope.jobId);
            if (envelope.outcome === 'success') {
                pending.resolve(envelope);
            } else {
                pending.reject(new CemProcessingOperationError(envelope));
            }
        } catch (error) {
            this.fail(this.readySettled ? 'execution' : 'startup', error);
        }
    };

    private readonly onError = (event: ErrorEvent): void => {
        event.preventDefault();
        this.fail(this.readySettled ? 'execution' : 'startup', event.error ?? new Error(event.message));
    };

    private readonly onMessageError = (): void => {
        this.fail(this.readySettled ? 'execution' : 'startup', new Error('worker message deserialization failed'));
    };

    private fail(phase: 'startup' | 'execution', cause: unknown): CemProcessingWorkerTransportError {
        const error = cause instanceof CemProcessingWorkerTransportError
            ? cause
            : new CemProcessingWorkerTransportError(
                phase,
                cause instanceof Error ? cause.message : 'the CEM processing worker failed'
            );
        if (this.failed) {
            return error;
        }
        this.failed = true;
        if (!this.readySettled) {
            this.readySettled = true;
            this.rejectReady(error);
        }
        for (const pending of this.pending.values()) {
            pending.reject(error);
        }
        this.pending.clear();
        this.detach();
        this.worker.terminate();
        return error;
    }

    private detach(): void {
        this.worker.removeEventListener('message', this.onMessage);
        this.worker.removeEventListener('error', this.onError);
        this.worker.removeEventListener('messageerror', this.onMessageError);
    }
}

class CemProcessingWorkerTransportError extends Error {
    constructor(readonly phase: 'startup' | 'execution', message: string) {
        super(message);
        this.name = 'CemProcessingWorkerTransportError';
    }
}

interface CemProcessingWorkerConnection {
    readonly ready: Promise<CemProcessingReadyEnvelope>;
    request(request: CemProcessingRequestEnvelope): Promise<CemProcessingSuccessEnvelope>;
    terminate(): void;
}

class FailedCemProcessingWorkerTransport implements CemProcessingWorkerConnection {
    readonly ready: Promise<CemProcessingReadyEnvelope>;

    private readonly error: CemProcessingWorkerTransportError;

    constructor(cause: unknown) {
        this.error = new CemProcessingWorkerTransportError(
            'startup',
            cause instanceof Error ? cause.message : 'the CEM processing worker could not be constructed'
        );
        this.ready = Promise.reject(this.error);
    }

    request(_request: CemProcessingRequestEnvelope): Promise<CemProcessingSuccessEnvelope> {
        return Promise.reject(this.error);
    }

    terminate(): void {
        // No worker was constructed.
    }
}

class CemProcessingOperationError extends Error {
    constructor(readonly response: CemProcessingResponseEnvelope) {
        super(
            response.outcome === 'success'
                ? 'the CEM processing operation unexpectedly failed'
                : response.diagnostics.map((diagnostic) => diagnostic.message).join('; ')
        );
        this.name = 'CemProcessingOperationError';
    }
}

class CemProcessingJobCancelledError extends Error {
    constructor(readonly jobId: number) {
        super(`CEM processing job ${jobId} was cancelled`);
        this.name = 'CemProcessingJobCancelledError';
    }
}

interface PooledWorkerRequest {
    request: CemProcessingRequestEnvelope;
    scopePolicyStamp: string;
    resolve(response: CemProcessingSuccessEnvelope): void;
    reject(error: unknown): void;
}

interface PooledRootOwner {
    id: number;
    slot: CemProcessingWorkerSlot;
    observers: Set<(event: CemProcessingSchedulingTraceEvent) => void>;
    released: boolean;
}

interface CemProcessingWorkerLease {
    readonly sequence: CemProcessingJobSequence;
    readonly ready: Promise<CemProcessingReadyEnvelope>;
    addTraceObserver(observer: ((event: CemProcessingSchedulingTraceEvent) => void) | undefined): void;
    request(
        request: CemProcessingRequestEnvelope,
        scopePolicyStamp: string
    ): Promise<CemProcessingSuccessEnvelope>;
    cancelQueued(targetJobId: number, scopePolicyStamp: string): void;
    record(
        kind: CemProcessingSchedulingTraceKind,
        request: CemProcessingRequestEnvelope,
        scopePolicyStamp: string
    ): void;
    release(reason?: unknown): void;
}

class CemProcessingWorkerSlot {
    readonly transport: CemProcessingWorkerConnection;
    private readonly scheduler: CemProcessingFairScheduler<PooledWorkerRequest>;
    private readonly owners = new Set<number>();
    private draining = false;

    constructor(
        readonly slotId: number,
        options: CemProcessingHostRuntimeOptions,
        policy: ResolvedCemProcessingPoolPolicy,
        private readonly pool: CemProcessingWorkerPool
    ) {
        try {
            this.transport = new CemProcessingWorkerTransport(
                options,
                `cem-processing-pool-${pool.poolId}-slot-${slotId}`
            );
        } catch (error) {
            this.transport = new FailedCemProcessingWorkerTransport(error);
        }
        this.scheduler = new CemProcessingFairScheduler(policy.queueSize);
    }

    get ownerCount(): number {
        return this.owners.size;
    }

    register(owner: PooledRootOwner): void {
        this.owners.add(owner.id);
        this.scheduler.registerOwner(owner.id);
    }

    enqueue(owner: PooledRootOwner, pending: PooledWorkerRequest): void {
        if (pending.request.operation === 'cancel') {
            this.pool.record(owner, 'enqueue', pending.request, pending.scopePolicyStamp);
            this.pool.record(owner, 'dispatch', pending.request, pending.scopePolicyStamp);
            void this.transport.request(pending.request).then(pending.resolve, pending.reject);
            return;
        }
        try {
            this.scheduler.enqueue(owner.id, pending);
        } catch (error) {
            this.pool.record(owner, 'overflow', pending.request, pending.scopePolicyStamp);
            pending.reject(error);
            return;
        }
        this.pool.record(owner, 'enqueue', pending.request, pending.scopePolicyStamp);
        this.drain();
    }

    cancelQueued(owner: PooledRootOwner, targetJobId: number, scopePolicyStamp: string): void {
        const removed = this.scheduler.cancel(
            (entry) => entry.ownerId === owner.id && entry.value.request.jobId === targetJobId
        );
        for (const entry of removed) {
            entry.value.reject(new CemProcessingJobCancelledError(targetJobId));
        }
        if (removed.length > 0) {
            const cancelled = removed[0].value.request;
            this.pool.record(owner, 'cancel', cancelled, scopePolicyStamp);
        }
    }

    release(
        owner: PooledRootOwner,
        reason: unknown = new Error('the CEM processing root was released')
    ): void {
        this.owners.delete(owner.id);
        for (const entry of this.scheduler.removeOwner(owner.id)) {
            entry.value.reject(reason);
        }
    }

    terminate(): void {
        this.transport.terminate();
    }

    private drain(): void {
        if (this.draining) {
            return;
        }
        this.draining = true;
        void this.drainQueued();
    }

    private async drainQueued(): Promise<void> {
        try {
            for (let entry = this.scheduler.dequeue(); entry; entry = this.scheduler.dequeue()) {
                const owner = this.pool.owner(entry.ownerId);
                if (!owner || owner.released) {
                    entry.value.reject(new Error('the CEM processing root was released'));
                    continue;
                }
                this.pool.record(owner, 'dispatch', entry.value.request, entry.value.scopePolicyStamp);
                try {
                    entry.value.resolve(await this.transport.request(entry.value.request));
                } catch (error) {
                    entry.value.reject(error);
                }
            }
        } finally {
            this.draining = false;
        }
    }
}

class CemProcessingWorkerPool {
    readonly sequence = new CemProcessingJobSequence();
    readonly poolId: number;

    private readonly trace = new CemProcessingSchedulingTrace();
    private readonly slots: CemProcessingWorkerSlot[] = [];
    private readonly owners = new Map<number, PooledRootOwner>();
    private ownerSequence = 0;
    private closed = false;

    constructor(
        private readonly options: CemProcessingHostRuntimeOptions,
        private readonly policy: ResolvedCemProcessingPoolPolicy,
        poolId: number,
        private readonly onEmpty: () => void
    ) {
        this.poolId = poolId;
    }

    acquire(observer?: (event: CemProcessingSchedulingTraceEvent) => void): CemProcessingWorkerLease {
        if (this.closed) {
            throw new Error('the CEM processing worker pool is closed');
        }
        const slot = this.selectSlot();
        const owner: PooledRootOwner = {
            id: ++this.ownerSequence,
            slot,
            observers: new Set(observer ? [observer] : []),
            released: false,
        };
        this.owners.set(owner.id, owner);
        slot.register(owner);
        let released = false;
        return {
            sequence: this.sequence,
            ready: slot.transport.ready,
            addTraceObserver: (next) => {
                if (next) {
                    owner.observers.add(next);
                }
            },
            request: (request, scopePolicyStamp) => new Promise((resolve, reject) => {
                if (released || owner.released) {
                    reject(new Error('the CEM processing root was released'));
                    return;
                }
                slot.enqueue(owner, { request, scopePolicyStamp, resolve, reject });
            }),
            cancelQueued: (targetJobId, scopePolicyStamp) => {
                slot.cancelQueued(owner, targetJobId, scopePolicyStamp);
            },
            record: (kind, request, scopePolicyStamp) => {
                this.record(owner, kind, request, scopePolicyStamp);
            },
            release: (reason) => {
                if (released) {
                    return;
                }
                released = true;
                this.release(owner, reason);
            },
        };
    }

    owner(ownerId: number): PooledRootOwner | undefined {
        return this.owners.get(ownerId);
    }

    record(
        owner: PooledRootOwner,
        kind: CemProcessingSchedulingTraceKind,
        request: CemProcessingRequestEnvelope,
        scopePolicyStamp: string
    ): void {
        const event = this.trace.record({
            kind,
            ownerScopeId: owner.id,
            scopePolicyStamp,
            workerSlot: owner.slot.slotId,
            jobId: request.jobId,
            operation: request.operation,
        });
        for (const observer of owner.observers) {
            try {
                observer(event);
            } catch {
                // Observability is never allowed to perturb scheduling semantics.
            }
        }
    }

    private selectSlot(): CemProcessingWorkerSlot {
        if (this.slots.length < this.policy.workerCount) {
            const slot = new CemProcessingWorkerSlot(
                this.slots.length + 1,
                this.options,
                this.policy,
                this
            );
            this.slots.push(slot);
            return slot;
        }
        return this.slots.reduce((selected, candidate) =>
            candidate.ownerCount < selected.ownerCount ? candidate : selected
        );
    }

    private release(owner: PooledRootOwner, reason?: unknown): void {
        owner.released = true;
        owner.slot.release(owner, reason);
        this.owners.delete(owner.id);
        if (this.owners.size !== 0 || this.closed) {
            return;
        }
        this.closed = true;
        for (const slot of this.slots) {
            slot.terminate();
        }
        this.onEmpty();
    }
}

class RootCemProcessingHost implements CemProcessingHost {
    private readonly sequence: CemProcessingJobSequence;
    private readonly jobs = new CemProcessingCancellationRegistry();
    private readonly engine = new CemProcessingEngine();
    private readonly compileInputs = new Map<string, CemProcessingCompileInput>();
    private readonly jobPolicies = new Map<number, string>();
    private readonly initialReady: Promise<CemProcessingReadyEnvelope>;
    private readonly lease: CemProcessingWorkerLease;
    private fallbackSelected = false;
    private fallbackTracePending = false;
    private pendingTransitionDiagnostic?: CemProcessingDiagnostic;
    private disposed = false;
    private removeDisposeListener: () => void;

    constructor(
        readonly ownerScope: CemDeclarationScope,
        pool: CemProcessingWorkerPool,
        observer?: (event: CemProcessingSchedulingTraceEvent) => void
    ) {
        this.lease = pool.acquire(observer);
        this.sequence = this.lease.sequence;
        try {
            this.initialReady = this.lease.ready.catch((error) => this.selectFallback(error).ready);
        } catch (error) {
            this.initialReady = this.selectFallback(
                new CemProcessingWorkerTransportError(
                    'startup',
                    error instanceof Error ? error.message : 'the CEM processing worker could not be constructed'
                )
            ).ready;
        }
        this.removeDisposeListener = onCemDeclarationScopeDispose(ownerScope, () => {
            void this.dispose({ reason: 'scope-disposed' }).result.catch(() => undefined);
        });
    }

    addTraceObserver(observer: ((event: CemProcessingSchedulingTraceEvent) => void) | undefined): void {
        this.lease.addTraceObserver(observer);
    }

    get mode(): 'worker' | 'main-thread' {
        return this.fallbackSelected ? 'main-thread' : 'worker';
    }

    get ready(): Promise<CemProcessingReadyEnvelope> {
        return this.initialReady;
    }

    compile(input: CemProcessingCompileInput): CemProcessingJob<CemProcessingCompileResult> {
        this.compileInputs.set(compileInputKey(input.scopePolicyStamp, input.templateArtifactId), input);
        return this.submit('compile', input);
    }

    renderDiff(input: CemProcessingRenderDiffInput): CemProcessingJob<CemProcessingRenderDiffResult> {
        const compileInput = this.compileInputs.get(
            compileInputKey(input.artifact.scopePolicyStamp, input.artifact.artifactId)
        );
        const preflight = compileInput
            ? createCemProcessingRequestEnvelope(this.sequence, 'compile', compileInput)
            : undefined;
        return this.submit('render-diff', input, false, preflight);
    }

    cancel(input: CemProcessingCancelInput): CemProcessingJob<CemProcessingCancelResult> {
        const scopePolicyStamp = this.jobPolicies.get(input.targetJobId) ?? 'host-control';
        const accepted = this.jobs.cancel(input.targetJobId);
        if (accepted) {
            this.lease.cancelQueued(input.targetJobId, scopePolicyStamp);
        }
        return this.submit('cancel', input, accepted);
    }

    dispose(input: CemProcessingDisposeInput): CemProcessingJob<CemProcessingDisposeResult> {
        if (this.disposed) {
            const jobId = this.sequence.next();
            return { jobId, result: Promise.resolve({ disposed: true }) };
        }
        this.disposed = true;
        this.removeDisposeListener?.();
        const request = createCemProcessingRequestEnvelope(this.sequence, 'dispose', input);
        this.jobs.start(request.jobId);
        const result = Promise.resolve().then(() => {
            this.lease.release();
            this.compileInputs.clear();
            this.jobPolicies.clear();
            return this.engine.dispose(input);
        }).finally(() => this.jobs.finish(request.jobId));
        return { jobId: request.jobId, result };
    }

    private submit<TOperation extends CemProcessingOperation>(
        operation: TOperation,
        payload: Parameters<typeof createCemProcessingRequestEnvelope<TOperation>>[2],
        cancellationAccepted = false,
        preflight?: CemProcessingRequestEnvelope<'compile'>
    ): CemProcessingJob<OperationResult<TOperation>> {
        const request = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
        const scopePolicyStamp = requestScopePolicyStamp(request, this.jobPolicies);
        this.jobs.start(request.jobId);
        this.jobPolicies.set(request.jobId, scopePolicyStamp);
        const result = Promise.resolve().then(async () => {
            if (this.disposed) {
                throw new Error('the CEM processing host is disposed');
            }
            await this.ready;
            this.assertNotCancelled(request);
            if (this.fallbackSelected) {
                this.recordPendingFallback(request, scopePolicyStamp);
                return this.addPendingTransitionDiagnostic(
                    await this.executeMainThreadWithArtifact(request, cancellationAccepted)
                );
            }
            try {
                if (preflight) {
                    await this.lease.request(preflight, preflight.payload.scopePolicyStamp);
                    this.assertNotCancelled(request);
                }
                const response = await this.lease.request(request, scopePolicyStamp);
                this.assertNotCancelled(request);
                if (request.operation === 'cancel' && cancellationAccepted) {
                    return {
                        targetJobId: request.payload.targetJobId,
                        accepted: true,
                    } as OperationResult<TOperation>;
                }
                return responseResult<TOperation>(response, operation);
            } catch (error) {
                this.assertNotCancelled(request);
                if (!(error instanceof CemProcessingWorkerTransportError)) {
                    throw error;
                }
                this.selectFallback(error, request);
                if (request.operation === 'cancel') {
                    return {
                        targetJobId: request.payload.targetJobId,
                        accepted: cancellationAccepted,
                    } as OperationResult<TOperation>;
                }
                const retry = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
                const retried = await this.executeMainThreadWithArtifact(retry, cancellationAccepted);
                this.assertNotCancelled(request);
                return this.addPendingTransitionDiagnostic(retried);
            }
        }).finally(() => {
            this.jobs.finish(request.jobId);
            this.jobPolicies.delete(request.jobId);
        });
        return { jobId: request.jobId, result };
    }

    private async executeMainThreadWithArtifact<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>,
        cancellationAccepted = false
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'render-diff') {
            const input = this.compileInputs.get(
                compileInputKey(
                    request.payload.artifact.scopePolicyStamp,
                    request.payload.artifact.artifactId
                )
            );
            if (!input) {
                throw new Error('the main-thread fallback is missing the worker template source');
            }
            await this.engine.compile(input);
        }
        return this.executeMainThread(request, cancellationAccepted);
    }

    private async executeMainThread<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>,
        cancellationAccepted = false
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'compile') {
            const result = await this.engine.compile(request.payload);
            this.assertNotCancelled(request);
            return result as OperationResult<TOperation>;
        }
        if (request.operation === 'render-diff') {
            const result = await this.engine.renderDiff(request.payload);
            this.assertNotCancelled(request);
            return result as OperationResult<TOperation>;
        }
        if (request.operation === 'cancel') {
            return {
                targetJobId: request.payload.targetJobId,
                accepted: cancellationAccepted,
            } as OperationResult<TOperation>;
        }
        return this.engine.dispose(request.payload) as OperationResult<TOperation>;
    }

    private assertNotCancelled(request: CemProcessingRequestEnvelope): void {
        if (request.operation !== 'cancel' && this.jobs.isCancelled(request.jobId)) {
            throw new CemProcessingJobCancelledError(request.jobId);
        }
    }

    private selectFallback(
        error: unknown,
        request?: CemProcessingRequestEnvelope
    ): { ready: Promise<CemProcessingReadyEnvelope> } {
        if (this.fallbackSelected) {
            return { ready: Promise.resolve(createCemProcessingReadyEnvelope('main-thread')) };
        }
        const phase = error instanceof CemProcessingWorkerTransportError ? error.phase : 'startup';
        const decision = request?.operation === 'render-diff'
            ? decideCemProcessingWorkerFailure({
                phase,
                operation: 'render-diff',
                transactionState: 'not-started',
                revision: request.payload.revision,
            })
            : decideCemProcessingWorkerFailure({
                phase,
                operation: request?.operation ?? 'compile',
            });
        this.fallbackSelected = true;
        this.lease.release(error);
        this.fallbackTracePending = request === undefined;
        if (request) {
            this.lease.record('fallback', request, requestScopePolicyStamp(request, this.jobPolicies));
        }
        this.pendingTransitionDiagnostic = 'diagnostic' in decision ? decision.diagnostic : {
            code: 'cem.processing_host.worker_startup_fallback',
            severity: 'warning',
            message: 'the dedicated worker failed; processing moved to the main-thread fallback',
        };
        return {
            ready: Promise.resolve(createCemProcessingReadyEnvelope('main-thread')),
        };
    }

    private recordPendingFallback(
        request: CemProcessingRequestEnvelope,
        scopePolicyStamp: string
    ): void {
        if (!this.fallbackTracePending) {
            return;
        }
        this.fallbackTracePending = false;
        this.lease.record('fallback', request, scopePolicyStamp);
    }

    private addPendingTransitionDiagnostic<TOperation extends CemProcessingOperation>(
        result: OperationResult<TOperation>
    ): OperationResult<TOperation> {
        const diagnostic = this.pendingTransitionDiagnostic;
        this.pendingTransitionDiagnostic = undefined;
        return diagnostic ? addDiagnostic(result, diagnostic) : result;
    }
}

function compileInputKey(scopePolicyStamp: string, artifactId: string): string {
    return `${scopePolicyStamp}\u0000${artifactId}`;
}

function requestScopePolicyStamp(
    request: CemProcessingRequestEnvelope,
    jobPolicies: ReadonlyMap<number, string>
): string {
    if (request.operation === 'compile') {
        return request.payload.scopePolicyStamp;
    }
    if (request.operation === 'render-diff') {
        return request.payload.artifact.scopePolicyStamp;
    }
    if (request.operation === 'cancel') {
        return jobPolicies.get(request.payload.targetJobId) ?? 'host-control';
    }
    return 'host-control';
}

type OperationResult<TOperation extends CemProcessingOperation> =
    TOperation extends 'compile' ? CemProcessingCompileResult
        : TOperation extends 'render-diff' ? CemProcessingRenderDiffResult
            : TOperation extends 'cancel' ? CemProcessingCancelResult
                : CemProcessingDisposeResult;

function responseResult<TOperation extends CemProcessingOperation>(
    response: CemProcessingSuccessEnvelope,
    operation: TOperation
): OperationResult<TOperation> {
    if (response.operation !== operation) {
        throw new Error('the CEM processing response operation did not match its request');
    }
    return response.result as OperationResult<TOperation>;
}

function addDiagnostic<TOperation extends CemProcessingOperation>(
    result: OperationResult<TOperation>,
    diagnostic: CemProcessingDiagnostic
): OperationResult<TOperation> {
    if ('diagnostics' in result) {
        return { ...result, diagnostics: [diagnostic, ...result.diagnostics] };
    }
    return result;
}

const hostsByRoot = new WeakMap<CemDeclarationScope, RootCemProcessingHost>();
const poolsByDocument = new WeakMap<
    Document,
    Map<CemProcessingWorkerFactory, Map<string, CemProcessingWorkerPool>>
>();
let poolSequence = 0;

/** Return the one lazy worker/fallback host owned by a logical root declaration scope. */
export function cemProcessingHostForScope(
    scope: CemDeclarationScope,
    options: CemProcessingHostRuntimeOptions
): CemProcessingHost {
    const ownerScope = cemProcessingHostOwnerScope(scope);
    const existing = hostsByRoot.get(ownerScope);
    if (existing) {
        existing.addTraceObserver(options.onTrace);
        return existing;
    }
    const host = new RootCemProcessingHost(
        ownerScope,
        workerPoolForDocument(ownerScope.document, options),
        options.onTrace
    );
    hostsByRoot.set(ownerScope, host);
    return host;
}

function workerPoolForDocument(
    document: Document,
    options: CemProcessingHostRuntimeOptions
): CemProcessingWorkerPool {
    const workerFactory = options.workerFactory ?? defaultCemProcessingWorkerFactory;
    const policy = resolveCemProcessingPoolPolicy(options.poolPolicy);
    let byFactory = poolsByDocument.get(document);
    if (!byFactory) {
        byFactory = new Map();
        poolsByDocument.set(document, byFactory);
    }
    let byPolicy = byFactory.get(workerFactory);
    if (!byPolicy) {
        byPolicy = new Map();
        byFactory.set(workerFactory, byPolicy);
    }
    const key = `${String(options.workerScriptUrl)}\u0000${policy.workerCount}\u0000${policy.maxWorkers}\u0000${policy.queueSize}`;
    const existing = byPolicy.get(key);
    if (existing) {
        return existing;
    }
    const normalizedOptions = { ...options, workerFactory };
    const pool = new CemProcessingWorkerPool(normalizedOptions, policy, ++poolSequence, () => {
        byPolicy?.delete(key);
        if (byPolicy?.size === 0) {
            byFactory?.delete(workerFactory);
        }
    });
    byPolicy.set(key, pool);
    return pool;
}
