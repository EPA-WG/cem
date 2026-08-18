import {
    onCemDeclarationScopeDispose,
    type CemDeclarationScope,
} from '../../declaration-scope.js';
import { CemProcessingEngine } from './processing-engine.js';
import {
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

export interface CemProcessingHostRuntimeOptions {
    workerScriptUrl: string | URL;
    workerFactory?: CemProcessingWorkerFactory;
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

class RootCemProcessingHost implements CemProcessingHost {
    private readonly sequence = new CemProcessingJobSequence();
    private readonly engine = new CemProcessingEngine();
    private readonly compileInputs = new Map<string, CemProcessingCompileInput>();
    private readonly initialReady: Promise<CemProcessingReadyEnvelope>;
    private worker?: CemProcessingWorkerTransport;
    private fallbackSelected = false;
    private pendingTransitionDiagnostic?: CemProcessingDiagnostic;
    private disposed = false;
    private removeDisposeListener: () => void;

    constructor(
        readonly ownerScope: CemDeclarationScope,
        options: CemProcessingHostRuntimeOptions,
        workerName: string
    ) {
        try {
            this.worker = new CemProcessingWorkerTransport(options, workerName);
            this.initialReady = this.worker.ready.catch((error) => this.selectFallback(error).ready);
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

    get mode(): 'worker' | 'main-thread' {
        return this.fallbackSelected ? 'main-thread' : 'worker';
    }

    get ready(): Promise<CemProcessingReadyEnvelope> {
        return this.initialReady;
    }

    compile(input: CemProcessingCompileInput): CemProcessingJob<CemProcessingCompileResult> {
        this.compileInputs.set(input.templateArtifactId, input);
        return this.submit('compile', input);
    }

    renderDiff(input: CemProcessingRenderDiffInput): CemProcessingJob<CemProcessingRenderDiffResult> {
        return this.submit('render-diff', input);
    }

    cancel(input: CemProcessingCancelInput): CemProcessingJob<CemProcessingCancelResult> {
        return this.submit('cancel', input);
    }

    dispose(input: CemProcessingDisposeInput): CemProcessingJob<CemProcessingDisposeResult> {
        if (this.disposed) {
            const jobId = this.sequence.next();
            return { jobId, result: Promise.resolve({ disposed: true }) };
        }
        this.disposed = true;
        this.removeDisposeListener?.();
        const job = this.submit('dispose', input, true);
        void job.result.then(
            () => this.worker?.terminate(),
            () => this.worker?.terminate()
        );
        return job;
    }

    private submit<TOperation extends CemProcessingOperation>(
        operation: TOperation,
        payload: Parameters<typeof createCemProcessingRequestEnvelope<TOperation>>[2],
        allowDisposed = false
    ): CemProcessingJob<OperationResult<TOperation>> {
        const request = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
        const result = Promise.resolve().then(async () => {
            if (this.disposed && !allowDisposed) {
                throw new Error('the CEM processing host is disposed');
            }
            await this.ready;
            if (this.fallbackSelected || !this.worker) {
                return this.addPendingTransitionDiagnostic(await this.executeMainThread(request));
            }
            try {
                const response = await this.worker.request(request);
                return responseResult<TOperation>(response, operation);
            } catch (error) {
                if (!(error instanceof CemProcessingWorkerTransportError)) {
                    throw error;
                }
                this.selectFallback(error, request);
                if (request.operation === 'cancel') {
                    return {
                        targetJobId: request.payload.targetJobId,
                        accepted: false,
                    } as OperationResult<TOperation>;
                }
                if (request.operation === 'dispose') {
                    return { disposed: true } as OperationResult<TOperation>;
                }
                const retry = createCemProcessingRequestEnvelope(this.sequence, operation, payload);
                const retried = await this.executeMainThreadWithArtifact(retry);
                return this.addPendingTransitionDiagnostic(retried);
            }
        });
        return { jobId: request.jobId, result };
    }

    private async executeMainThreadWithArtifact<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'render-diff') {
            const input = this.compileInputs.get(request.payload.artifact.artifactId);
            if (!input) {
                throw new Error('the main-thread fallback is missing the worker template source');
            }
            await this.engine.compile(input);
        }
        return this.executeMainThread(request);
    }

    private async executeMainThread<TOperation extends CemProcessingOperation>(
        request: CemProcessingRequestEnvelope<TOperation>
    ): Promise<OperationResult<TOperation>> {
        if (request.operation === 'compile') {
            return await this.engine.compile(request.payload) as OperationResult<TOperation>;
        }
        if (request.operation === 'render-diff') {
            return await this.engine.renderDiff(request.payload) as OperationResult<TOperation>;
        }
        if (request.operation === 'cancel') {
            return {
                targetJobId: request.payload.targetJobId,
                accepted: true,
            } as OperationResult<TOperation>;
        }
        return this.engine.dispose(request.payload) as OperationResult<TOperation>;
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
        this.worker?.terminate();
        this.pendingTransitionDiagnostic = 'diagnostic' in decision ? decision.diagnostic : {
            code: 'cem.processing_host.worker_startup_fallback',
            severity: 'warning',
            message: 'the dedicated worker failed; processing moved to the main-thread fallback',
        };
        return {
            ready: Promise.resolve(createCemProcessingReadyEnvelope('main-thread')),
        };
    }

    private addPendingTransitionDiagnostic<TOperation extends CemProcessingOperation>(
        result: OperationResult<TOperation>
    ): OperationResult<TOperation> {
        const diagnostic = this.pendingTransitionDiagnostic;
        this.pendingTransitionDiagnostic = undefined;
        return diagnostic ? addDiagnostic(result, diagnostic) : result;
    }
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
let workerSequence = 0;

/** Return the one lazy worker/fallback host owned by a logical root declaration scope. */
export function cemProcessingHostForScope(
    scope: CemDeclarationScope,
    options: CemProcessingHostRuntimeOptions
): CemProcessingHost {
    const ownerScope = cemProcessingHostOwnerScope(scope);
    const existing = hostsByRoot.get(ownerScope);
    if (existing) {
        return existing;
    }
    const host = new RootCemProcessingHost(ownerScope, options, `cem-processing-root-${++workerSequence}`);
    hostsByRoot.set(ownerScope, host);
    return host;
}
