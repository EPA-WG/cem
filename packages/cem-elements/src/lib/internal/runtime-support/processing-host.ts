import type { DataIslandSnapshot, SourceMapMode } from '../../cem-elements.js';
import {
    assertProcessingBoundaryValue,
    type PatchFrame,
    type RenderRevision,
    type SourceMapRef,
} from '../../projection.js';
import {
    assertCemDeclarationScopeActive,
    type CemDeclarationScope,
} from '../../declaration-scope.js';

/** @internal Phase 3A worker/main-thread protocol. Not a public package export. */
export const CEM_PROCESSING_HOST_PROTOCOL_VERSION = 'cem-processing-host-v1' as const;

export const CEM_PROCESSING_HOST_CAPABILITIES = [
    'compile',
    'render-diff',
    'cancel',
    'dispose',
] as const;

export type CemProcessingHostMode = 'worker' | 'main-thread';
export type CemProcessingOperation = typeof CEM_PROCESSING_HOST_CAPABILITIES[number];
export type CemProcessingJobId = number;

export interface CemProcessingDiagnostic {
    code: string;
    severity: 'info' | 'warning' | 'error' | 'fatal';
    message: string;
    sourceMapRef?: SourceMapRef;
}

export interface CemProcessingSourceRef {
    kind: 'inline' | 'url' | 'specifier' | 'fragment';
    value: string;
}

/** Clone-safe text chunks consumed as one Phase 3A compile source. */
export interface CemProcessingTextSource {
    kind: 'text-chunks-v1';
    chunks: string[];
}

export interface CemTemplateArtifactPayloadKey {
    contentType: 'cem-template-artifact';
    sourceHash: string;
    cemMlVersion: string;
    cemQlVersion: string;
    sourceMapMode: SourceMapMode;
}

export interface CemProcessingArtifactBinaryTransfer {
    kind: 'template-artifact';
    payloadKey: CemTemplateArtifactPayloadKey;
    /** CEM-Hash of the complete immutable artifact envelope. */
    cacheKey: string;
    formatVersion: string;
    policyStamp: string;
    bytes: ArrayBuffer;
    sourceMapSidecarHash?: string;
}

export interface CemArtifactRegistryNamespace {
    namespace: 'cem-template-artifacts';
    registryContractVersion: 'cem-artifact-registry-v1';
    artifactFormatVersion: string;
}

export interface CemArtifactRegistryHooks {
    getArtifact?(
        namespace: CemArtifactRegistryNamespace,
        key: CemTemplateArtifactPayloadKey
    ): Promise<CemProcessingArtifactBinaryTransfer | undefined>;
    putArtifact?(
        namespace: CemArtifactRegistryNamespace,
        artifact: CemProcessingArtifactBinaryTransfer
    ): Promise<void>;
    invalidateNamespace?(namespace: CemArtifactRegistryNamespace): Promise<void>;
}

/**
 * Adapt local text or a materialized Phase 1 remote stream to the same worker
 * source shape. Chunk boundaries are transport details and do not participate in
 * artifact identity.
 */
export function createCemProcessingTextSource(
    source: string,
    chunkSize = 16_384
): CemProcessingTextSource {
    if (!Number.isSafeInteger(chunkSize) || chunkSize < 1) {
        throw new RangeError('a CEM processing source chunk size must be a positive safe integer');
    }
    const chunks: string[] = [];
    for (let offset = 0; offset < source.length; offset += chunkSize) {
        chunks.push(source.slice(offset, offset + chunkSize));
    }
    if (chunks.length === 0) {
        chunks.push('');
    }
    return { kind: 'text-chunks-v1', chunks };
}

export interface CemProcessingCompileInput {
    language: 'cem-ml';
    producedTag: string;
    templateArtifactId: string;
    registrationIdentity: string;
    source: CemProcessingTextSource;
    sourceRef: CemProcessingSourceRef;
    resolverIdentity: string;
    scopePolicyStamp: string;
    sourceMapMode: SourceMapMode;
    hostBindings?: string[];
    precompiledArtifact?: CemProcessingArtifactBinaryTransfer;
    /** Request binary write-through after a registry miss. Omitted on the normal source path. */
    exportCompiledArtifact?: true;
}

/** A stable reference to an artifact retained by one root-scope processing host. */
export interface CemProcessingArtifactHandle {
    kind: 'template-artifact-handle';
    artifactId: string;
    cacheKey: string;
    registrationIdentity: string;
    scopePolicyStamp: string;
    sourceMapMode: SourceMapMode;
}

/** A stable reference to a render plan retained by one root-scope processing host. */
export interface CemProcessingRenderPlanHandle {
    kind: 'render-plan-handle';
    renderPlanId: string;
    templateArtifactId: string;
    revision: RenderRevision;
    renderEngineVersion: string;
    sourceMapMode: SourceMapMode;
}

export interface CemProcessingCompileResult {
    artifact: CemProcessingArtifactHandle;
    declaredAttributes: string[];
    observedAttributes: string[];
    invalidationScopes: string[];
    diagnostics: CemProcessingDiagnostic[];
    /** Present only after source compilation so a host registry can write through. */
    compiledArtifact?: CemProcessingArtifactBinaryTransfer;
}

export interface CemProcessingRenderDiffInput {
    artifact: CemProcessingArtifactHandle;
    revision: RenderRevision;
    snapshot: DataIslandSnapshot;
    /** Host-neutral bindings derived from the complete data-island snapshot. */
    data: Record<string, unknown>;
    /** Deterministic render/keyframe identity applied before the retained-plan diff. */
    scopeUid: string;
    previousRenderPlan?: CemProcessingRenderPlanHandle | null;
    patchBatchSize?: number;
}

/**
 * Browser-owned resource work lowered out of a worker render plan before DOM
 * diffing. It contains only interpolated, clone-safe declaration data; URL
 * resolution, policy, transport streams, and AbortSignals stay on the host.
 */
export interface CemProcessingHttpRequestControl {
    kind: 'http-request';
    renderNodeId: string;
    sliceName: string;
    authoredUrl: string;
    method: string;
    headers: Record<string, string>;
    expectedContentType?: string;
    credentials?: string;
    cache?: string;
    sourceMapRef?: SourceMapRef;
}

export interface CemProcessingModuleUrlControl {
    kind: 'module-url';
    renderNodeId: string;
    sliceName: string;
    authoredSpecifier: string;
    referrer?: string;
    referrerSelector?: string;
    sourceMapRef?: SourceMapRef;
}

export interface CemProcessingRepositoryQueryControl {
    kind: 'repository-query';
    renderNodeId: string;
    sliceName: string;
    repository: string;
    operation: string;
    parameters?: string;
    live: boolean;
    cursor?: string;
    sourceMapRef?: SourceMapRef;
}

export interface CemProcessingStorageStatusControl {
    kind: 'storage-status';
    renderNodeId: string;
    sliceName: string;
    repository: string;
    live: boolean;
    cursor?: string;
    sourceMapRef?: SourceMapRef;
}

export type CemProcessingResourceControl =
    | CemProcessingModuleUrlControl
    | CemProcessingHttpRequestControl
    | CemProcessingRepositoryQueryControl
    | CemProcessingStorageStatusControl;

export interface CemProcessingRenderDiffResult {
    revision: RenderRevision;
    nextRenderPlan: CemProcessingRenderPlanHandle;
    frames: PatchFrame[];
    hostAttributeUpdates: CemProcessingHostAttributeUpdate[];
    resourceControls: CemProcessingResourceControl[];
    diagnostics: CemProcessingDiagnostic[];
}

export interface CemProcessingHostAttributeUpdate {
    name: string;
    value: string;
}

export interface CemProcessingCancelInput {
    targetJobId: CemProcessingJobId;
    reason: 'superseded' | 'declaration-disposed' | 'scope-disposed' | 'host-disposed';
}

export interface CemProcessingCancelResult {
    targetJobId: CemProcessingJobId;
    accepted: boolean;
}

export interface CemProcessingDisposeInput {
    reason: 'scope-disposed' | 'runtime-disposed' | 'worker-failed';
}

export interface CemProcessingDisposeResult {
    disposed: true;
}

interface CemProcessingRequestPayloads {
    compile: CemProcessingCompileInput;
    'render-diff': CemProcessingRenderDiffInput;
    cancel: CemProcessingCancelInput;
    dispose: CemProcessingDisposeInput;
}

interface CemProcessingSuccessResults {
    compile: CemProcessingCompileResult;
    'render-diff': CemProcessingRenderDiffResult;
    cancel: CemProcessingCancelResult;
    dispose: CemProcessingDisposeResult;
}

export type CemProcessingRequestEnvelope<
    TOperation extends CemProcessingOperation = CemProcessingOperation,
> = TOperation extends CemProcessingOperation
    ? {
          protocolVersion: typeof CEM_PROCESSING_HOST_PROTOCOL_VERSION;
          direction: 'request';
          jobId: CemProcessingJobId;
          operation: TOperation;
          payload: CemProcessingRequestPayloads[TOperation];
      }
    : never;

export type CemProcessingSuccessEnvelope<
    TOperation extends CemProcessingOperation = CemProcessingOperation,
> = TOperation extends CemProcessingOperation
    ? {
          protocolVersion: typeof CEM_PROCESSING_HOST_PROTOCOL_VERSION;
          direction: 'response';
          jobId: CemProcessingJobId;
          operation: TOperation;
          outcome: 'success';
          result: CemProcessingSuccessResults[TOperation];
      }
    : never;

export interface CemProcessingFailureEnvelope {
    protocolVersion: typeof CEM_PROCESSING_HOST_PROTOCOL_VERSION;
    direction: 'response';
    jobId: CemProcessingJobId;
    operation: CemProcessingOperation;
    outcome: 'failure' | 'cancelled';
    diagnostics: CemProcessingDiagnostic[];
}

export type CemProcessingResponseEnvelope =
    | CemProcessingSuccessEnvelope
    | CemProcessingFailureEnvelope;

export interface CemProcessingReadyEnvelope {
    protocolVersion: typeof CEM_PROCESSING_HOST_PROTOCOL_VERSION;
    direction: 'ready';
    mode: CemProcessingHostMode;
    capabilities: typeof CEM_PROCESSING_HOST_CAPABILITIES;
}

export type CemProcessingEnvelope =
    | CemProcessingRequestEnvelope
    | CemProcessingResponseEnvelope
    | CemProcessingReadyEnvelope;

/** Host-owned positive safe-integer IDs. Response IDs echo their request ID. */
export class CemProcessingJobSequence {
    private nextJobId = 1;

    next(): CemProcessingJobId {
        if (!Number.isSafeInteger(this.nextJobId)) {
            throw new RangeError('the CEM processing-host job sequence is exhausted');
        }
        const jobId = this.nextJobId;
        this.nextJobId += 1;
        return jobId;
    }
}

/**
 * Tracks the bounded lifetime of host jobs so worker and fallback cancellation
 * have the same acceptance and late-result suppression semantics.
 */
export class CemProcessingCancellationRegistry {
    private readonly activeJobs = new Set<CemProcessingJobId>();
    private readonly cancelledJobs = new Set<CemProcessingJobId>();

    start(jobId: CemProcessingJobId): void {
        this.activeJobs.add(jobId);
    }

    cancel(jobId: CemProcessingJobId): boolean {
        if (!this.activeJobs.has(jobId)) {
            return false;
        }
        this.cancelledJobs.add(jobId);
        return true;
    }

    isCancelled(jobId: CemProcessingJobId): boolean {
        return this.cancelledJobs.has(jobId);
    }

    finish(jobId: CemProcessingJobId): void {
        this.activeJobs.delete(jobId);
        this.cancelledJobs.delete(jobId);
    }
}

export function createCemProcessingReadyEnvelope(
    mode: CemProcessingHostMode
): CemProcessingReadyEnvelope {
    const envelope: CemProcessingReadyEnvelope = {
        protocolVersion: CEM_PROCESSING_HOST_PROTOCOL_VERSION,
        direction: 'ready',
        mode,
        capabilities: CEM_PROCESSING_HOST_CAPABILITIES,
    };
    assertCemProcessingEnvelope(envelope);
    return envelope;
}

export function createCemProcessingRequestEnvelope<TOperation extends CemProcessingOperation>(
    sequence: CemProcessingJobSequence,
    operation: TOperation,
    payload: CemProcessingRequestPayloads[TOperation]
): CemProcessingRequestEnvelope<TOperation> {
    const envelope = {
        protocolVersion: CEM_PROCESSING_HOST_PROTOCOL_VERSION,
        direction: 'request' as const,
        jobId: sequence.next(),
        operation,
        payload,
    } as CemProcessingRequestEnvelope<TOperation>;
    assertCemProcessingEnvelope(envelope);
    return envelope;
}

export function createCemProcessingSuccessEnvelope<TOperation extends CemProcessingOperation>(
    request: CemProcessingRequestEnvelope<TOperation>,
    result: CemProcessingSuccessResults[TOperation]
): CemProcessingSuccessEnvelope<TOperation> {
    const envelope = {
        protocolVersion: CEM_PROCESSING_HOST_PROTOCOL_VERSION,
        direction: 'response' as const,
        jobId: request.jobId,
        operation: request.operation,
        outcome: 'success' as const,
        result,
    } as CemProcessingSuccessEnvelope<TOperation>;
    assertCemProcessingEnvelope(envelope);
    return envelope;
}

export function createCemProcessingFailureEnvelope(
    request: CemProcessingRequestEnvelope,
    outcome: CemProcessingFailureEnvelope['outcome'],
    diagnostics: CemProcessingDiagnostic[]
): CemProcessingFailureEnvelope {
    const envelope: CemProcessingFailureEnvelope = {
        protocolVersion: CEM_PROCESSING_HOST_PROTOCOL_VERSION,
        direction: 'response',
        jobId: request.jobId,
        operation: request.operation,
        outcome,
        diagnostics,
    };
    assertCemProcessingEnvelope(envelope);
    return envelope;
}

/** Reject malformed versions/IDs and all non-plain structured-clone values at either host. */
export function assertCemProcessingEnvelope(envelope: unknown): asserts envelope is CemProcessingEnvelope {
    assertProcessingBoundaryValue(envelope, 'CEM processing-host envelope');
    if (!envelope || typeof envelope !== 'object' || Array.isArray(envelope)) {
        throw new TypeError('a CEM processing-host envelope must be a plain record');
    }
    const candidate = envelope as Record<string, unknown>;
    if (candidate.protocolVersion !== CEM_PROCESSING_HOST_PROTOCOL_VERSION) {
        throw new TypeError(`unsupported CEM processing-host protocol ${String(candidate.protocolVersion)}`);
    }
    if (candidate.direction !== 'request' && candidate.direction !== 'response' && candidate.direction !== 'ready') {
        throw new TypeError(`unsupported CEM processing-host envelope direction ${String(candidate.direction)}`);
    }
    if (candidate.direction === 'ready') {
        if (candidate.mode !== 'worker' && candidate.mode !== 'main-thread') {
            throw new TypeError(`unsupported CEM processing-host ready mode ${String(candidate.mode)}`);
        }
        if (
            !Array.isArray(candidate.capabilities)
            || candidate.capabilities.length !== CEM_PROCESSING_HOST_CAPABILITIES.length
            || candidate.capabilities.some(
                (capability, index) => capability !== CEM_PROCESSING_HOST_CAPABILITIES[index]
            )
        ) {
            throw new TypeError('the CEM processing-host ready envelope has incompatible capabilities');
        }
        return;
    }
    if (!CEM_PROCESSING_HOST_CAPABILITIES.includes(candidate.operation as CemProcessingOperation)) {
        throw new TypeError(`unsupported CEM processing-host operation ${String(candidate.operation)}`);
    }
    if (!Number.isSafeInteger(candidate.jobId) || (candidate.jobId as number) < 1) {
        throw new TypeError('a CEM processing-host request/response requires a positive safe-integer job ID');
    }
}

/** Resolve the one owner key shared by an explicit root and every logical child scope. */
export function cemProcessingHostOwnerScope(scope: CemDeclarationScope): CemDeclarationScope {
    assertCemDeclarationScopeActive(scope);
    let owner = scope;
    while (owner.parent) {
        owner = owner.parent;
    }
    return owner;
}

export interface CemProcessingJob<TResult> {
    readonly jobId: CemProcessingJobId;
    readonly result: Promise<TResult>;
}

/**
 * One semantic interface implemented by the worker primary and main-thread fallback.
 * A provider MUST return one host per {@link cemProcessingHostOwnerScope} identity.
 */
export interface CemProcessingHost {
    readonly mode: CemProcessingHostMode;
    readonly ownerScope: CemDeclarationScope;
    readonly ready: Promise<CemProcessingReadyEnvelope>;
    compile(input: CemProcessingCompileInput): CemProcessingJob<CemProcessingCompileResult>;
    renderDiff(input: CemProcessingRenderDiffInput): CemProcessingJob<CemProcessingRenderDiffResult>;
    cancel(input: CemProcessingCancelInput): CemProcessingJob<CemProcessingCancelResult>;
    dispose(input: CemProcessingDisposeInput): CemProcessingJob<CemProcessingDisposeResult>;
}

export interface CemProcessingHostProvider {
    forScope(scope: CemDeclarationScope): CemProcessingHost;
}

export interface CemProcessingWorkerFactoryInput {
    scriptUrl: string | URL;
    name: string;
    type: 'module';
}

/** Package-private injection seam; CSP/bundler/test hosts inject construction, not a worker instance. */
export type CemProcessingWorkerFactory = (input: CemProcessingWorkerFactoryInput) => Worker;

/** Package default. The processing host supplies its bundler-resolved module URL. */
export const defaultCemProcessingWorkerFactory: CemProcessingWorkerFactory = ({ scriptUrl, name }) =>
    new Worker(scriptUrl, { name, type: 'module' });

export type CemProcessingWorkerFailurePhase = 'startup' | 'execution';
export type CemProcessingPatchTransactionState = 'not-started' | 'begun' | 'committed';

interface CemProcessingWorkerFailureBase {
    phase: CemProcessingWorkerFailurePhase;
    fallbackAlreadySelected?: boolean;
}

export type CemProcessingWorkerFailure =
    | (CemProcessingWorkerFailureBase & { operation: 'compile' })
    | (CemProcessingWorkerFailureBase & {
          operation: 'render-diff';
          transactionState: CemProcessingPatchTransactionState;
          revision: RenderRevision;
      })
    | (CemProcessingWorkerFailureBase & { operation: 'cancel' | 'dispose' });

export type CemProcessingWorkerFailureDecision =
    | {
          action: 'retry-main-thread';
          nextMode: 'main-thread';
          allocateNewJobId: true;
          ignoreLateWorkerResult: true;
          abortTransaction: false;
          retryRevision?: RenderRevision;
          diagnostic: CemProcessingDiagnostic;
      }
    | {
          action: 'abort-and-retry-main-thread';
          nextMode: 'main-thread';
          allocateNewJobId: true;
          ignoreLateWorkerResult: true;
          abortTransaction: true;
          retryRevision: RenderRevision;
          diagnostic: CemProcessingDiagnostic;
      }
    | {
          action: 'preserve-committed-result';
          nextMode: 'main-thread';
          allocateNewJobId: false;
          ignoreLateWorkerResult: true;
          abortTransaction: false;
          diagnostic: CemProcessingDiagnostic;
      }
    | {
          action: 'complete-control-without-retry';
          nextMode: 'main-thread' | 'disposed';
          allocateNewJobId: false;
          ignoreLateWorkerResult: true;
          abortTransaction: false;
          diagnostic: CemProcessingDiagnostic;
      }
    | {
          action: 'ignore-duplicate-worker-failure';
          nextMode: 'main-thread';
          allocateNewJobId: false;
          ignoreLateWorkerResult: true;
          abortTransaction: false;
      };

/** Pure transition core used by both startup and post-handshake worker failure paths. */
export function decideCemProcessingWorkerFailure(
    failure: CemProcessingWorkerFailure
): CemProcessingWorkerFailureDecision {
    if (failure.fallbackAlreadySelected) {
        return {
            action: 'ignore-duplicate-worker-failure',
            nextMode: 'main-thread',
            allocateNewJobId: false,
            ignoreLateWorkerResult: true,
            abortTransaction: false,
        };
    }

    const diagnostic = workerFallbackDiagnostic(failure.phase);
    if (failure.operation === 'compile') {
        return {
            action: 'retry-main-thread',
            nextMode: 'main-thread',
            allocateNewJobId: true,
            ignoreLateWorkerResult: true,
            abortTransaction: false,
            diagnostic,
        };
    }
    if (failure.operation !== 'render-diff') {
        return {
            action: 'complete-control-without-retry',
            nextMode: failure.operation === 'dispose' ? 'disposed' : 'main-thread',
            allocateNewJobId: false,
            ignoreLateWorkerResult: true,
            abortTransaction: false,
            diagnostic,
        };
    }
    if (failure.transactionState === 'committed') {
        return {
            action: 'preserve-committed-result',
            nextMode: 'main-thread',
            allocateNewJobId: false,
            ignoreLateWorkerResult: true,
            abortTransaction: false,
            diagnostic,
        };
    }
    if (failure.transactionState === 'begun') {
        return {
            action: 'abort-and-retry-main-thread',
            nextMode: 'main-thread',
            allocateNewJobId: true,
            ignoreLateWorkerResult: true,
            abortTransaction: true,
            retryRevision: nextRenderAttempt(failure.revision),
            diagnostic,
        };
    }
    return {
        action: 'retry-main-thread',
        nextMode: 'main-thread',
        allocateNewJobId: true,
        ignoreLateWorkerResult: true,
        abortTransaction: false,
        retryRevision: failure.revision,
        diagnostic,
    };
}

function nextRenderAttempt(revision: RenderRevision): RenderRevision {
    return {
        ...revision,
        renderAttempt: (revision.renderAttempt ?? 0) + 1,
    };
}

function workerFallbackDiagnostic(phase: CemProcessingWorkerFailurePhase): CemProcessingDiagnostic {
    return {
        code: phase === 'startup'
            ? 'cem.processing_host.worker_startup_fallback'
            : 'cem.processing_host.worker_execution_fallback',
        severity: 'warning',
        message: phase === 'startup'
            ? 'the dedicated worker failed during startup; processing moved to the main-thread fallback'
            : 'the dedicated worker failed after startup; processing moved to the main-thread fallback',
    };
}
