import { CemNativeCheckpointDomain, cemWorkerCheckpointDomain, cemMainThreadCheckpointDomain, nativeCheckpointAdmission, nativeCheckpointRequirements, nativeCheckpointChanged, nativeCheckpointRefuse, nativeCheckpointIdentity, nativeCheckpointGeneration, nativeCheckpointFailureFence, nativeCheckpointWitnessSource, type CemCheckpointNativeClient } from './checkpoint-domain.js';
import { nativeCheckpointRetirementCleanup, nativeCheckpointRetirementStarted, nativeCheckpointSubscribe, nativeCheckpointAssertAdmission, nativeCheckpointDrainRequirements } from './checkpoint-domain.js';
import {
    onCemDeclarationScopeDispose,
    type CemDeclarationScope,
} from '../../declaration-scope.js';
import { CemProcessingEngine } from './processing-engine.js';
import {
    CEM_RENDERING_RETIREMENT_PROTOCOL, isCemRenderingRetirementControl, type CemRenderingRetirementAction,
    CemProcessingCancellationRegistry,
    CemProcessingDiagnosticError,
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
    type CemProcessingStylesheetInput,
    type CemProcessingStylesheetResult,
    type CemProcessingNativeSessionInput,
    type CemProcessingNativeSessionResult,
    type CemProcessingValueInput,
    type CemProcessingValueResult,
    type CemProcessingDocumentInput,
    type CemProcessingDocumentResult,
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
    readonly checkpointDomain: CemNativeCheckpointDomain;
    readonly ready: Promise<CemProcessingReadyEnvelope>;

    private readonly worker: Worker;
    private readonly pending = new Map<number, PendingWorkerRequest>();
    private controlSequence = 0;
    private readonly controls = new Map<number, { action: CemRenderingRetirementAction; resolve(value: unknown): void; reject(error: unknown): void }>();
    private terminated = false;
    private readySettled = false;
    private failed = false;
    private readonly failureListeners = new Set<() => void>();
    private resolveReady!: (ready: CemProcessingReadyEnvelope) => void;
    private rejectReady!: (error: unknown) => void;

    constructor(options: CemProcessingHostRuntimeOptions, name: string) {
        const factory = options.workerFactory ?? defaultCemProcessingWorkerFactory;
        this.ready = new Promise<CemProcessingReadyEnvelope>((resolve, reject) => {
            this.resolveReady = resolve;
            this.rejectReady = reject;
        });
        this.worker = factory({ scriptUrl: options.workerScriptUrl, name, type: 'module' });
        this.checkpointDomain = cemWorkerCheckpointDomain(this.worker);
        this.checkpointDomain.retirementEndpoint({
            close: async () => { await this.control('close'); },
            prepare: () => this.control('prepare'),
            abort: async () => { await this.control('abort'); },
            terminate: () => this.terminate(),
        });
        this.worker.addEventListener('message', this.onMessage);
        this.worker.addEventListener('error', this.onError);
        this.worker.addEventListener('messageerror', this.onMessageError);
    }

    onFailure(listener: () => void): () => void {
        if (this.failed) { queueMicrotask(listener); return () => undefined; }
        this.failureListeners.add(listener); return () => { this.failureListeners.delete(listener); };
    }

    async request(request: CemProcessingRequestEnvelope): Promise<CemProcessingSuccessEnvelope> {
        await this.ready;
        if (this.failed) {
            throw new CemProcessingWorkerTransportError('execution', 'the CEM processing worker transport failed');
        }
        if (this.terminated) throw new Error('cem.runtime.generation.retired');
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

    private async control(action: CemRenderingRetirementAction): Promise<unknown> {
        await this.ready;
        if (this.failed || this.terminated || this.controls.size >= 2 || !Number.isSafeInteger(++this.controlSequence))
            throw new Error('cem.runtime.retirement.transport');
        const id = this.controlSequence;
        return new Promise((resolve, reject) => {
            this.controls.set(id, { action, resolve, reject });
            try { this.worker.postMessage({ protocol: CEM_RENDERING_RETIREMENT_PROTOCOL, direction: 'request', id, action }); }
            catch (error) { this.controls.delete(id); reject(error); }
        });
    }

    terminate(): void {
        if (this.terminated) return;
        this.terminated = true;
        for (const pending of this.controls.values()) pending.reject(new Error('cem.runtime.generation.retired'));
        this.controls.clear();
        this.detach();
        this.worker.terminate();
    }

    private readonly onMessage = (event: MessageEvent<unknown>): void => {
        try {
            if (isCemRenderingRetirementControl(event.data)) {
                const response = event.data, pending = this.controls.get(response.id);
                if (response.direction !== 'response' || !pending || pending.action !== response.action) throw new Error('cem.runtime.retirement.response');
                this.controls.delete(response.id);
                if (response.ok) pending.resolve(response.confirmation);
                else pending.reject(new Error(`cem.runtime.retirement.native-refused: ${response.error ?? 'unavailable'}`));
                return;
            }
            assertCemProcessingEnvelope(event.data);
            const envelope = event.data;
            if (envelope.direction === 'pressure') {
                if (!this.readySettled) throw new TypeError('pressure signal before worker readiness');
                this.checkpointDomain.closeAdmission(envelope.reason);
                return;
            }
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
        for (const pending of this.controls.values()) pending.reject(error);
        this.controls.clear();
        for (const listener of [...this.failureListeners]) { try { listener(); } catch { /* Finish revoking all owners. */ } }
        this.failureListeners.clear();
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
    readonly checkpointDomain: CemNativeCheckpointDomain;
    onFailure?(listener: () => void): () => void;
    readonly ready: Promise<CemProcessingReadyEnvelope>;
    request(request: CemProcessingRequestEnvelope): Promise<CemProcessingSuccessEnvelope>;
    terminate(): void;
}

class FailedCemProcessingWorkerTransport implements CemProcessingWorkerConnection {
    readonly checkpointDomain = new CemNativeCheckpointDomain('worker');
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

class CemProcessingOperationError extends CemProcessingDiagnosticError {
    constructor(readonly response: CemProcessingResponseEnvelope) {
        super(response.outcome === 'success' ? [] : response.diagnostics,
            response.outcome === 'success' ? 'the CEM processing operation unexpectedly failed' : undefined);
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
    checkpointClient: CemCheckpointNativeClient;
}

interface CemProcessingWorkerLease {
    readonly checkpointClient: CemCheckpointNativeClient;
    onFailure(listener: () => void): () => void;
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
    get checkpointDomain(): CemNativeCheckpointDomain { return this.transport.checkpointDomain; }
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
        this.transport.onFailure?.(() => this.checkpointDomain.revoke());
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
        // Exact-generation CSS release is cleanup control, just like cancel.
        // Bulk disconnect must not fill the work queue or reject new renders.
        if (pending.request.operation === 'cancel' || pending.request.operation === 'native-session' && pending.request.payload.action === 'release' || pending.request.operation === 'stylesheet'
            && pending.request.payload.action === 'release') {
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
        this.checkpointDomain.remove(owner.checkpointClient);
        for (const entry of this.scheduler.removeOwner(owner.id)) {
            entry.value.reject(reason);
        }
    }

    terminate(): void {
        this.checkpointDomain.revoke();
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
            checkpointClient: slot.checkpointDomain.client(),
        };
        this.owners.set(owner.id, owner);
        slot.register(owner);
        let released = false;
        return {
            checkpointClient: owner.checkpointClient,
            sequence: this.sequence,
            ready: slot.transport.ready,
            onFailure: listener => slot.transport.onFailure?.(listener) ?? (() => undefined),
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

interface NativeProcessingJob<Result> extends CemProcessingJob<Result> { nativeCurrent(): boolean }

class RootCemProcessingHost implements CemProcessingHost {
    private readonly sequence: CemProcessingJobSequence;
    private readonly jobs = new CemProcessingCancellationRegistry();
    private readonly engine: CemProcessingEngine;
    readonly checkpointClient: CemCheckpointNativeClient;
    private readonly stylesheetOwner = crypto.randomUUID();
    private readonly stylesheetConsumers = new Map<string, Pick<CemProcessingStylesheetInput, 'artifact' | 'consumer'>>();
    private readonly pendingStylesheetLoads = new Map<string, string>();
    private readonly stylesheetJobs = new Set<Promise<unknown>>();
    private readonly nativeSessionOwner = crypto.randomUUID();
    private readonly nativeSessionHandles = new Map<string, { handle: CemProcessingNativeSessionInput['handle']; mode: 'worker' | 'main-thread'; generation?: string }>();
    private readonly nativeAuthorityListeners = new Set<() => void>();
    private readonly stopNativeOwnerLoss: () => void;
    private readonly nativeSessionJobs = new Set<Promise<unknown>>();
    private readonly documentInputs = new Map<string, Extract<CemProcessingDocumentInput, { action: 'retain' }>>();
    private readonly compileInputs = new Map<string, CemProcessingCompileInput>();
    private readonly checkpointInstances = new Map<string, number>();
    private readonly hostJobs = new Set<Promise<unknown>>();
    private readonly jobPolicies = new Map<number, string>();
    private readonly initialReady: Promise<CemProcessingReadyEnvelope>;
    private readonly lease: CemProcessingWorkerLease;
    private workerFailed = false;
    private readonly stopGeneration: () => void;
    private generationRetired = false;
    private fallbackSelected = false;
    private fallbackTracePending = false;
    private pendingTransitionDiagnostic?: CemProcessingDiagnostic;
    private disposed = false;
    private disposalJob?: CemProcessingJob<CemProcessingDisposeResult>;
    private removeDisposeListener: () => void;

    constructor(
        readonly ownerScope: CemDeclarationScope,
        pool: CemProcessingWorkerPool,
        observer?: (event: CemProcessingSchedulingTraceEvent) => void
    ) {
        this.lease = pool.acquire(observer);
        this.sequence = this.lease.sequence;
        this.checkpointClient = this.lease.checkpointClient;
        this.engine = new CemProcessingEngine({ checkpointClient: this.checkpointClient });
        nativeCheckpointRetirementCleanup(this.checkpointClient, () => {
            if (this.mode !== 'main-thread' || this.disposed || this.generationRetired || this.hostJobs.size
                || this.nativeSessionJobs.size || this.stylesheetJobs.size || this.pendingStylesheetLoads.size)
                throw new Error('cem.runtime.retirement.cleanup');
            this.engine.dispose({ reason: 'runtime-disposed' });
        });
        this.stopGeneration = nativeCheckpointSubscribe(this.checkpointClient, () => {
            if (!this.generationRetired && !nativeCheckpointIdentity(this.checkpointClient)) { this.generationRetired = true; this.revokeNativeAuthority(); }
        });
        nativeCheckpointRequirements(this.checkpointClient, sources => {
            if (this.pendingStylesheetLoads.size || [...this.nativeSessionHandles.keys()].some(key => !sources.has(`session:${key}`))
                || [...this.documentInputs.keys()].some(key => !sources.has(`document:${key}`))) return undefined;
            return [...this.checkpointInstances.keys()];
        });
        nativeCheckpointDrainRequirements(this.checkpointClient, () => !this.pendingStylesheetLoads.size);
        this.stopNativeOwnerLoss = this.lease.onFailure(() => {
            this.workerFailed = true;
            if (!this.generationRetired) { this.generationRetired = true; this.revokeNativeAuthority(); }
        });
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

    witnessCheckpointSource(handle: CemProcessingNativeSessionInput['handle'], source: object): void {
        const key = JSON.stringify([this.nativeSessionOwner, handle.sessionKey]);
        const known = this.nativeSessionHandles.get(key);
        if (!known || known.mode !== this.mode || known.generation !== nativeCheckpointGeneration(this.checkpointClient) || known.handle.instanceId !== handle.instanceId
            || known.handle.scopePolicyStamp !== handle.scopePolicyStamp || known.handle.sourceRevision !== handle.sourceRevision) return;
        const mode = this.mode, generation = nativeCheckpointIdentity(this.checkpointClient);
        nativeCheckpointWitnessSource(source, this.checkpointClient, `session:${key}`, () => !!generation && nativeCheckpointIdentity(this.checkpointClient) === generation && !this.disposed && this.mode === mode && this.nativeSessionHandles.get(key) === known);
    }

    witnessCheckpointDocument(handle: CemProcessingDocumentInput['handle'], source: object): void {
        const key = JSON.stringify(handle), known = this.documentInputs.get(key), mode = this.mode, generation = nativeCheckpointIdentity(this.checkpointClient);
        if (!known) return;
        nativeCheckpointWitnessSource(source, this.checkpointClient, `document:${key}`,
            () => !!generation && nativeCheckpointIdentity(this.checkpointClient) === generation && !this.disposed && this.mode === mode && this.documentInputs.get(key) === known);
    }

    checkpointInstance(instanceId: string): () => void {
        if (!this.checkpointInstances.has(instanceId) && this.checkpointInstances.size >= 256) { nativeCheckpointRefuse(this.checkpointClient); return () => undefined; }
        this.checkpointInstances.set(instanceId, (this.checkpointInstances.get(instanceId) ?? 0) + 1);
        nativeCheckpointChanged(this.checkpointClient);
        let removed = false;
        return () => {
            if (removed) return;
            removed = true;
            const count = (this.checkpointInstances.get(instanceId) ?? 1) - 1;
            if (count) this.checkpointInstances.set(instanceId, count); else this.checkpointInstances.delete(instanceId);
            nativeCheckpointChanged(this.checkpointClient);
        };
    }

    onNativeAuthorityLost(invalidate: () => void): () => void {
        if (this.disposed || this.generationRetired || this.workerFailed && !this.fallbackSelected) {
            let active = true; queueMicrotask(() => { if (active) invalidate(); });
            return () => { active = false; };
        }
        this.nativeAuthorityListeners.add(invalidate); return () => { this.nativeAuthorityListeners.delete(invalidate); };
    }
    private revokeNativeAuthority(): void {
        for (const listener of [...this.nativeAuthorityListeners]) { try { listener(); } catch { /* Continue revoking other source sessions. */ } }
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

    nativeSession(input: CemProcessingNativeSessionInput): CemProcessingJob<CemProcessingNativeSessionResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingNativeSessionResult>(input.action === 'release' || input.action === 'release-suggestions'); if (refused) return refused;
        const failCleanup = nativeCheckpointFailureFence(this.checkpointClient);
        input = structuredClone(input);
        const publicHandle = { ...input.handle };
        input = { ...input, handle: { ...publicHandle, sessionKey: JSON.stringify([this.nativeSessionOwner, publicHandle.sessionKey]) } };
        const key = input.handle.sessionKey;
        const known = this.nativeSessionHandles.get(key);
        if (known && (known.handle.instanceId !== input.handle.instanceId || known.handle.scopePolicyStamp !== input.handle.scopePolicyStamp
            || known.handle.sourceRevision !== input.handle.sourceRevision)) {
            return { jobId: this.sequence.next(), result: Promise.reject(new Error('Native session revision or owner mismatch')) };
        }
        if (input.action === 'prepare' && known) {
            return { jobId: this.sequence.next(), result: Promise.reject(new Error('Native session identity was already issued')) };
        }
        if (input.action !== 'prepare' && input.action !== 'release' && (!known || known.mode !== this.mode || known.generation !== nativeCheckpointGeneration(this.checkpointClient))) {
            return { jobId: this.sequence.next(), result: Promise.reject(new Error('Native session worker owner was lost; prepare a fresh session')) };
        }
        if (input.action === 'prepare') this.nativeSessionHandles.set(key, { handle: input.handle, mode: this.mode });
        const issued = this.nativeSessionHandles.get(key);
        const payload = input;
        const job = this.submit('native-session', payload);
        const result = job.result.then(async result => {
            if (this.disposed && payload.action !== 'release') throw new Error('Native session host was disposed');
            if (payload.action !== 'release' && !job.nativeCurrent()) throw new Error('Native session execution generation changed; prepare a fresh session');
            if (payload.action === 'prepare') {
                const current = this.nativeSessionHandles.get(key);
                if (!current || current !== issued) throw new Error('Native session preparation was released');
                current.mode = this.mode; current.generation = nativeCheckpointGeneration(this.checkpointClient);
            }
            if (payload.action === 'release') this.nativeSessionHandles.delete(key);
            return { ...result, handle: publicHandle };
        }).catch(async (error: unknown) => {
            if (payload.action === 'release' && !this.disposed) failCleanup();
            if (payload.action === 'prepare' && !this.disposed && !job.nativeCurrent()) {
                if (this.nativeSessionHandles.get(key) === issued) this.nativeSessionHandles.delete(key);
            } else if (payload.action === 'prepare' && !this.disposed) {
                await this.nativeSession({ action: 'release', handle: publicHandle }).result.catch(() => undefined);
            }
            throw error;
        }).finally(() => this.nativeSessionJobs.delete(result));
        this.nativeSessionJobs.add(result);
        return { jobId: job.jobId, result, nativeCurrent: () => job.nativeCurrent()
            && (payload.action !== 'prepare' || this.nativeSessionHandles.get(key) === issued) };
    }

    value(input: CemProcessingValueInput): CemProcessingJob<CemProcessingValueResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingValueResult>(); if (refused) return refused;
        return this.submit('value', input);
    }

    stylesheet(input: CemProcessingStylesheetInput): CemProcessingJob<CemProcessingStylesheetResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingStylesheetResult>(input.action !== 'begin'); if (refused) return refused;
        input = { ...input, consumer: JSON.stringify([this.stylesheetOwner, input.consumer]) };
        const key = JSON.stringify([input.artifact, input.consumer]);
        if (input.action !== 'release') this.stylesheetConsumers.set(key, { artifact: input.artifact, consumer: input.consumer });
        // Like render-diff, a new load may begin after the bounded worker cache
        // evicted its artifact. Reacquire only at begin; delivery/release must
        // keep their original generation and reject stale load handles.
        const compileInput = input.action === 'begin'
            ? this.compileInputs.get(compileInputKey(input.artifact.scopePolicyStamp, input.artifact.artifactId))
            : undefined;
        const preflight = compileInput
            ? createCemProcessingRequestEnvelope(this.sequence, 'compile', compileInput)
            : undefined;
        const job = this.submit('stylesheet', input, false, preflight);
        const result = job.result.then((result) => {
            if (input.action !== 'release' && !job.nativeCurrent()) throw new Error('Stylesheet execution generation changed; restart the load');
            if (result.status === 'pending') {
                const load = JSON.stringify([key, result.loadId]);
                if (!this.pendingStylesheetLoads.has(load) && this.pendingStylesheetLoads.size >= 256) nativeCheckpointRefuse(this.checkpointClient);
                else this.pendingStylesheetLoads.set(load, key);
            } else if (result.status === 'ready') this.pendingStylesheetLoads.delete(JSON.stringify([key, result.loadId]));
            else if (input.action === 'release') {
                if (input.loadId === undefined) {
                    this.stylesheetConsumers.delete(key);
                    for (const [load, consumer] of this.pendingStylesheetLoads) if (consumer === key) this.pendingStylesheetLoads.delete(load);
                } else this.pendingStylesheetLoads.delete(JSON.stringify([key, input.loadId]));
            }
            nativeCheckpointChanged(this.checkpointClient);
            return result;
        }).finally(() => this.stylesheetJobs.delete(result));
        this.stylesheetJobs.add(result);
        return { jobId: job.jobId, result };
    }

    document(input: CemProcessingDocumentInput): CemProcessingJob<CemProcessingDocumentResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingDocumentResult>(input.action === 'release'); if (refused) return refused;
        const failCleanup = nativeCheckpointFailureFence(this.checkpointClient);
        input = structuredClone(input);
        const key = JSON.stringify(input.handle);
        if (input.action === 'retain') this.documentInputs.set(key, input);
        const job = this.submit('document', input);
        return { jobId: job.jobId, result: job.result.then(result => {
            if (input.action === 'retain' && !job.nativeCurrent()) throw new Error('Document execution generation changed; reacquire source');
            if (input.action === 'release') this.documentInputs.delete(key);
            return result;
        }).catch((error: unknown) => {
            if (input.action === 'release' && !this.disposed) failCleanup();
            if (input.action === 'retain' && !this.disposed && this.documentInputs.get(key) === input) {
                // Retain the cleanup intent until native release acknowledges it.
                // A cancellation can follow native import before the reply arrives.
                if (!this.disposed) void this.document({ action: 'release', handle: input.handle }).result.catch(() => undefined);
            }
            throw error;
        }) };
    }

    compile(input: CemProcessingCompileInput): CemProcessingJob<CemProcessingCompileResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingCompileResult>(); if (refused) return refused;
        this.compileInputs.set(compileInputKey(input.scopePolicyStamp, input.templateArtifactId), input);
        return this.submit('compile', input);
    }

    renderDiff(input: CemProcessingRenderDiffInput): CemProcessingJob<CemProcessingRenderDiffResult> {
        if (this.disposed) return this.disposedJob();
        const refused = this.admissionRefusal<CemProcessingRenderDiffResult>(); if (refused) return refused;
        // Direct host render clients must also be represented in coverage.
        if (!this.checkpointInstances.has(input.revision.instanceId)) {
            if (this.checkpointInstances.size >= 256) nativeCheckpointRefuse(this.checkpointClient);
            else this.checkpointInstances.set(input.revision.instanceId, 0);
        }
        // Pin the request before async dispatch/preflight in either execution mode.
        input = structuredClone(input);
        if (input.nativeSuggestions) {
            const live = input.nativeSuggestions;
            if (!live) throw new Error('Missing live publication');
            const handle = { ...live.handle, sessionKey: JSON.stringify([this.nativeSessionOwner, live.handle.sessionKey]) };
            const known = this.nativeSessionHandles.get(handle.sessionKey);
            if (!known || known.mode !== this.mode || known.generation !== nativeCheckpointGeneration(this.checkpointClient) || known.handle.instanceId !== handle.instanceId
                || known.handle.scopePolicyStamp !== handle.scopePolicyStamp || known.handle.sourceRevision !== handle.sourceRevision) {
                return { jobId: this.sequence.next(), result: Promise.reject(new Error('Native frame owner was lost or differs; reacquire authority')) };
            }
            input.nativeSuggestions = { ...live, handle };
        }
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
        if (this.disposalJob) return this.disposalJob;
        if (this.disposed) {
            const jobId = this.sequence.next();
            return { jobId, result: Promise.resolve({ disposed: true }) };
        }
        this.disposed = true;
        const releaseCheckpoint = nativeCheckpointAdmission(this.checkpointClient, true);
        this.stopNativeOwnerLoss(); this.revokeNativeAuthority(); this.nativeAuthorityListeners.clear();
        this.removeDisposeListener?.();
        const request = createCemProcessingRequestEnvelope(this.sequence, 'dispose', input);
        this.jobs.start(request.jobId);
        const result = Promise.resolve().then(async () => {
            await Promise.allSettled([...this.hostJobs, ...this.stylesheetJobs, ...this.nativeSessionJobs]);
            if (!this.fallbackSelected && !this.generationRetired) {
                // The worker may also serve other roots. Release this root's owners first.
                if (this.nativeSessionHandles.size) await Promise.all([...this.nativeSessionHandles.values()].filter(s => s.mode === 'worker').map(s =>
                    this.lease.request(createCemProcessingRequestEnvelope(this.sequence, 'native-session',
                        { action: 'release', handle: s.handle }), s.handle.scopePolicyStamp).catch(() => nativeCheckpointRefuse(this.checkpointClient))));

                await Promise.all([...this.stylesheetConsumers.values()].map((consumer) =>
                    this.lease.request(createCemProcessingRequestEnvelope(this.sequence, 'stylesheet',
                        { ...consumer, action: 'release' }), consumer.artifact.scopePolicyStamp).catch(() => nativeCheckpointRefuse(this.checkpointClient))));

                await Promise.all([...this.documentInputs.values()].map((document) =>
                    this.lease.request(createCemProcessingRequestEnvelope(this.sequence, 'document',
                        { action: 'release', handle: document.handle }), document.handle.scopePolicyStamp)
                        .catch(() => nativeCheckpointRefuse(this.checkpointClient))));
            }
            this.nativeSessionHandles.clear();
            this.stylesheetConsumers.clear();
            this.pendingStylesheetLoads.clear();
            this.documentInputs.clear();
            this.lease.release();
            this.compileInputs.clear();
            this.jobPolicies.clear();
            return this.engine.dispose(input);
        }).catch(error => { nativeCheckpointRefuse(this.checkpointClient); throw error; }).finally(() => {
            releaseCheckpoint(); this.stopGeneration();
            cemMainThreadCheckpointDomain.remove(this.checkpointClient);
            this.jobs.finish(request.jobId);
        });
        this.disposalJob = { jobId: request.jobId, result };
        return this.disposalJob;
    }

    private disposedJob<Result>(): CemProcessingJob<Result> {
        return { jobId: this.sequence.next(), result: Promise.reject(new Error('the CEM processing host is disposed')) };
    }

    private admissionRefusal<Result>(cleanup = false): CemProcessingJob<Result> | undefined {
        try { nativeCheckpointAssertAdmission(this.checkpointClient, cleanup); }
        catch (error) { return { jobId: this.sequence.next(), result: Promise.reject(error) }; }
        return undefined;
    }

    private submit<TOperation extends CemProcessingOperation>(
        operation: TOperation,
        payload: Parameters<typeof createCemProcessingRequestEnvelope<TOperation>>[2],
        cancellationAccepted = false,
        preflight?: CemProcessingRequestEnvelope<'compile'>
    ): NativeProcessingJob<OperationResult<TOperation>> {
        let execution: { generation: string | undefined; mode: 'worker' | 'main-thread' } | undefined;
        const entering = () => { execution = { generation: nativeCheckpointGeneration(this.checkpointClient), mode: this.mode }; };
        const request = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
        const cleanup = operation === 'cancel' || operation === 'dispose'
            || (operation === 'stylesheet' && (payload as CemProcessingStylesheetInput).action !== 'begin')
            || operation === 'native-session' && ['release', 'release-suggestions'].includes((payload as CemProcessingNativeSessionInput).action)
            || operation === 'document' && (payload as CemProcessingDocumentInput).action === 'release';
        let releaseCheckpoint: () => void;
        try { releaseCheckpoint = nativeCheckpointAdmission(this.checkpointClient, cleanup); }
        catch (error) { return { jobId: request.jobId, result: Promise.reject(error), nativeCurrent: () => false }; }
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
                entering();
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
                entering();
                const response = await this.lease.request(request, scopePolicyStamp);
                if (request.operation === 'stylesheet' && response.operation === 'stylesheet'
                    && 'loadId' in response.result && (this.disposed || this.jobs.isCancelled(request.jobId))) {
                    await this.lease.request(createCemProcessingRequestEnvelope(this.sequence, 'stylesheet', {
                        action: 'release', artifact: request.payload.artifact, consumer: request.payload.consumer,
                        loadId: response.result.loadId,
                    }), scopePolicyStamp).catch(() => undefined);
                }
                if (request.operation === 'native-session' && request.payload.action === 'prepare'
                    && (this.disposed || this.jobs.isCancelled(request.jobId))) {
                    await this.lease.request(createCemProcessingRequestEnvelope(this.sequence, 'native-session', {
                        action: 'release', handle: request.payload.handle,
                    }), scopePolicyStamp).catch(() => undefined);
                }
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
                if (request.operation === 'native-session' && request.payload.action !== 'prepare') {
                    throw new Error('Native session worker owner was lost; prepare a fresh session', { cause: error });
                }
                if (request.operation === 'stylesheet' && request.payload.action !== 'begin') {
                    throw new Error('stylesheet worker owner was lost; restart the load', { cause: error });
                }
                if (request.operation === 'cancel') {
                    return {
                        targetJobId: request.payload.targetJobId,
                        accepted: cancellationAccepted,
                    } as OperationResult<TOperation>;
                }
                const retry = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
                entering();
                const retried = await this.executeMainThreadWithArtifact(retry, cancellationAccepted);
                this.assertNotCancelled(request);
                return this.addPendingTransitionDiagnostic(retried);
            }
        }).finally(() => {
            this.jobs.finish(request.jobId);
            this.jobPolicies.delete(request.jobId);
            releaseCheckpoint();
        });
        this.hostJobs.add(result);
        void result.then(() => this.hostJobs.delete(result), () => this.hostJobs.delete(result));
        return { jobId: request.jobId, result, nativeCurrent: () => !this.generationRetired && !!execution?.generation
            && execution.generation === nativeCheckpointGeneration(this.checkpointClient)
            && (execution.mode !== 'worker' || !this.workerFailed) };
    }

    private async executeMainThreadWithArtifact<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>,
        cancellationAccepted = false
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'render-diff' || request.operation === 'stylesheet' && request.payload.action === 'begin') {
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
            for (const binding of request.operation === 'render-diff' ? request.payload.documents ?? [] : []) {
                const document = this.documentInputs.get(JSON.stringify(binding.handle));
                if (!document) throw new Error('the main-thread fallback is missing the retained document bytes');
                await this.engine.document(document);
            }
        }
        return this.executeMainThread(request, cancellationAccepted);
    }

    private async executeMainThread<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>,
        cancellationAccepted = false
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'native-session') {
            const result = await this.engine.nativeSession(request.payload);
            try { this.assertNotCancelled(request); }
            catch (error) {
                if (request.payload.action === 'prepare') await this.engine.nativeSession({ action: 'release', handle: request.payload.handle });
                throw error;
            }
            return result as OperationResult<TOperation>;
        }
        if (request.operation === 'value') {
            const result = await this.engine.value(request.payload);
            this.assertNotCancelled(request);
            return result as OperationResult<TOperation>;
        }
        if (request.operation === 'stylesheet') {
            const result = await this.engine.stylesheet(request.payload);
            try {
                this.assertNotCancelled(request);
            } catch (error) {
                if ('loadId' in result) await this.engine.stylesheet({ action: 'release',
                    artifact: request.payload.artifact, consumer: request.payload.consumer, loadId: result.loadId });
                throw error;
            }
            return result as OperationResult<TOperation>;
        }
        if (request.operation === 'document') {
            const result = await this.engine.document(request.payload);
            try {
                this.assertNotCancelled(request);
            } catch (error) {
                if (request.payload.action === 'retain') {
                    await this.engine.document({ action: 'release', handle: request.payload.handle });
                }
                throw error;
            }
            return result as OperationResult<TOperation>;
        }
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
        if (request.operation === 'stylesheet' && this.disposed) throw new Error('the CEM processing host is disposed');
        if (request.operation !== 'cancel' && this.jobs.isCancelled(request.jobId)) {
            throw new CemProcessingJobCancelledError(request.jobId);
        }
    }

    private selectFallback(
        error: unknown,
        request?: CemProcessingRequestEnvelope
    ): { ready: Promise<CemProcessingReadyEnvelope> } {
        if (nativeCheckpointRetirementStarted(this.checkpointClient)) throw new Error('cem.runtime.generation.retired');
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
            : request?.operation === 'stylesheet'
                ? decideCemProcessingWorkerFailure({ phase, operation: 'stylesheet', action: request.payload.action })
            : request?.operation === 'native-session'
                ? decideCemProcessingWorkerFailure({ phase, operation: 'native-session', action: request.payload.action })
            : decideCemProcessingWorkerFailure({
                phase,
                operation: request?.operation ?? 'compile',
            });
        this.fallbackSelected = true;
        // Confirmed worker termination retires committed worker models. Pending
        // preparations still own their input and may complete a fallback retry.
        for (const [key, session] of this.nativeSessionHandles) if (session.mode === 'worker' && session.generation) this.nativeSessionHandles.delete(key);
        cemMainThreadCheckpointDomain.move(this.checkpointClient);
        this.generationRetired = false;
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
    if (request.operation === 'native-session') return request.payload.handle.scopePolicyStamp;
    if (request.operation === 'value') return request.payload.scopePolicyStamp;
    if (request.operation === 'document') return request.payload.handle.scopePolicyStamp;
    if (request.operation === 'compile') {
        return request.payload.scopePolicyStamp;
    }
    if (request.operation === 'render-diff' || request.operation === 'stylesheet') {
        return request.payload.artifact.scopePolicyStamp;
    }
    if (request.operation === 'cancel') {
        return jobPolicies.get(request.payload.targetJobId) ?? 'host-control';
    }
    return 'host-control';
}

type OperationResult<TOperation extends CemProcessingOperation> =
    TOperation extends 'native-session' ? CemProcessingNativeSessionResult
        : TOperation extends 'stylesheet' ? CemProcessingStylesheetResult
        : TOperation extends 'value' ? CemProcessingValueResult
        : TOperation extends 'document' ? CemProcessingDocumentResult
        : TOperation extends 'compile' ? CemProcessingCompileResult
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
