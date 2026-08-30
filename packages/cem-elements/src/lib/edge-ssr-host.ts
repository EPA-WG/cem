import type {
    ExportedDataIslandSnapshot,
    SourceMapMode,
} from './cem-elements.js';
import {
    assertProcessingBoundaryValue,
    type EdgeContentAddress,
    type EdgeRenderStateRecord,
    type PatchFrame,
    type RenderPlanIdentity,
    type RenderRevision,
    type TemplateSourceNode,
} from './projection.js';
import {
    CEM_PROCESSING_HOST_PROTOCOL_VERSION,
    CemProcessingJobSequence,
    type CemProcessingArtifactBinaryTransfer,
    type CemProcessingDiagnostic,
    type CemProcessingRequestEnvelope,
    type CemProcessingSuccessEnvelope,
} from './internal/runtime-support/processing-host.js';

/** Public Edge/SSR profile of the shared CEM processing-host envelope. */
export const CEM_EDGE_SSR_HOST_PROTOCOL_VERSION = CEM_PROCESSING_HOST_PROTOCOL_VERSION;

export const CEM_EDGE_SSR_HOST_OPERATIONS = [
    'render-initial',
    'render-update',
] as const;

export const CEM_EDGE_SSR_HOST_FAILURE_REASONS = [
    'invalid-request',
    'privacy-policy-rejected',
    'render-state-not-found',
    'render-state-conflict',
    'content-unavailable',
    'render-failed',
    'cancelled',
] as const;

export type CemEdgeSsrHostOperation = typeof CEM_EDGE_SSR_HOST_OPERATIONS[number];
export type CemEdgeSsrHostFailureReason = typeof CEM_EDGE_SSR_HOST_FAILURE_REASONS[number];

export type CemEdgeSsrTemplateInput =
    | {
          kind: 'serialized-template-source-v1';
          templateArtifactId: string;
          source: TemplateSourceNode[];
      }
    | {
          kind: 'compiled-template-artifact-v1';
          templateArtifactId: string;
          artifact: CemProcessingArtifactBinaryTransfer;
      }
    | {
          kind: 'content-addressed-template-artifact-v1';
          templateArtifactId: string;
          address: EdgeContentAddress & { kind: 'template-artifact' };
      };

export interface CemEdgeSsrRenderInput {
    template: CemEdgeSsrTemplateInput;
    snapshot: ExportedDataIslandSnapshot;
    revision: RenderRevision;
    sourceMapMode: SourceMapMode;
    scopeUid: string;
}

export type CemEdgeSsrInitialRenderInput = CemEdgeSsrRenderInput;

export interface CemEdgeSsrPreviousRenderPlan {
    stateKey: string;
    expectedEtag: string;
    identity: RenderPlanIdentity;
    address: EdgeContentAddress & { kind: 'render-plan' };
}

export interface CemEdgeSsrRenderUpdateInput extends CemEdgeSsrRenderInput {
    previousRenderPlan: CemEdgeSsrPreviousRenderPlan;
}

export interface CemSsrHydrationData {
    kind: 'cem-ssr-hydration-v1';
    snapshot: ExportedDataIslandSnapshot;
    revision: RenderRevision;
    renderPlanIdentity: RenderPlanIdentity;
    sourceMapMode: SourceMapMode;
}

export interface CemEdgeSsrInitialRenderResult {
    kind: 'initial-render';
    /** HTML for the owned light-DOM render range; host adapters own wrapper and hydration-data escaping. */
    renderedHtml: string;
    hydrationData: CemSsrHydrationData;
    renderState: EdgeRenderStateRecord;
    diagnostics: CemProcessingDiagnostic[];
}

export interface CemEdgeSsrPatchFrameProgress {
    kind: 'patch-frame';
    frame: PatchFrame;
}

export interface CemEdgeSsrRenderUpdateResult {
    kind: 'render-update-complete';
    renderPlanIdentity: RenderPlanIdentity;
    renderState: EdgeRenderStateRecord;
    diagnostics: CemProcessingDiagnostic[];
}

interface CemEdgeSsrHostRequestPayloads {
    'render-initial': CemEdgeSsrInitialRenderInput;
    'render-update': CemEdgeSsrRenderUpdateInput;
}

interface CemEdgeSsrHostSuccessResults {
    'render-initial': CemEdgeSsrInitialRenderResult;
    'render-update': CemEdgeSsrRenderUpdateResult;
}

type CemProcessingRequestHeader = Pick<
    CemProcessingRequestEnvelope<'compile'>,
    'protocolVersion' | 'direction' | 'jobId'
>;

type CemProcessingResponseHeader = Pick<
    CemProcessingSuccessEnvelope<'compile'>,
    'protocolVersion' | 'direction' | 'jobId'
>;

export type CemEdgeSsrHostRequestEnvelope<
    TOperation extends CemEdgeSsrHostOperation = CemEdgeSsrHostOperation,
> = TOperation extends CemEdgeSsrHostOperation
    ? CemProcessingRequestHeader & {
          operation: TOperation;
          payload: CemEdgeSsrHostRequestPayloads[TOperation];
      }
    : never;

export type CemEdgeSsrHostProgressEnvelope = CemProcessingResponseHeader & {
    operation: 'render-update';
    outcome: 'progress';
    result: CemEdgeSsrPatchFrameProgress;
};

export type CemEdgeSsrHostSuccessEnvelope<
    TOperation extends CemEdgeSsrHostOperation = CemEdgeSsrHostOperation,
> = TOperation extends CemEdgeSsrHostOperation
    ? CemProcessingResponseHeader & {
          operation: TOperation;
          outcome: 'success';
          result: CemEdgeSsrHostSuccessResults[TOperation];
      }
    : never;

export type CemEdgeSsrHostFailureEnvelope<
    TOperation extends CemEdgeSsrHostOperation = CemEdgeSsrHostOperation,
> = TOperation extends CemEdgeSsrHostOperation
    ? CemProcessingResponseHeader & {
          operation: TOperation;
          outcome: 'failure' | 'cancelled';
          reason: CemEdgeSsrHostFailureReason;
          diagnostics: CemProcessingDiagnostic[];
          currentRenderState?: EdgeRenderStateRecord;
      }
    : never;

export type CemEdgeSsrHostResponseEnvelope =
    | CemEdgeSsrHostProgressEnvelope
    | CemEdgeSsrHostSuccessEnvelope
    | CemEdgeSsrHostFailureEnvelope;

export type CemEdgeSsrHostEnvelope =
    | CemEdgeSsrHostRequestEnvelope
    | CemEdgeSsrHostResponseEnvelope;

/** Uses the same positive, monotonic job-ID lifecycle as the browser processing host. */
export class CemEdgeSsrJobSequence extends CemProcessingJobSequence {}

export function createCemEdgeSsrHostRequestEnvelope<TOperation extends CemEdgeSsrHostOperation>(
    sequence: CemEdgeSsrJobSequence,
    operation: TOperation,
    payload: CemEdgeSsrHostRequestPayloads[TOperation]
): CemEdgeSsrHostRequestEnvelope<TOperation> {
    const envelope = {
        protocolVersion: CEM_EDGE_SSR_HOST_PROTOCOL_VERSION,
        direction: 'request' as const,
        jobId: sequence.next(),
        operation,
        payload,
    } as CemEdgeSsrHostRequestEnvelope<TOperation>;
    assertCemEdgeSsrHostEnvelope(envelope);
    return envelope;
}

export function createCemEdgeSsrHostProgressEnvelope(
    request: CemEdgeSsrHostRequestEnvelope<'render-update'>,
    frame: PatchFrame
): CemEdgeSsrHostProgressEnvelope {
    const envelope: CemEdgeSsrHostProgressEnvelope = {
        protocolVersion: CEM_EDGE_SSR_HOST_PROTOCOL_VERSION,
        direction: 'response',
        jobId: request.jobId,
        operation: request.operation,
        outcome: 'progress',
        result: { kind: 'patch-frame', frame },
    };
    assertCemEdgeSsrHostEnvelope(envelope);
    return envelope;
}

export function createCemEdgeSsrHostSuccessEnvelope<TOperation extends CemEdgeSsrHostOperation>(
    request: CemEdgeSsrHostRequestEnvelope<TOperation>,
    result: CemEdgeSsrHostSuccessResults[TOperation]
): CemEdgeSsrHostSuccessEnvelope<TOperation> {
    const envelope = {
        protocolVersion: CEM_EDGE_SSR_HOST_PROTOCOL_VERSION,
        direction: 'response' as const,
        jobId: request.jobId,
        operation: request.operation,
        outcome: 'success' as const,
        result,
    } as CemEdgeSsrHostSuccessEnvelope<TOperation>;
    assertCemEdgeSsrHostEnvelope(envelope);
    return envelope;
}

export function createCemEdgeSsrHostFailureEnvelope<TOperation extends CemEdgeSsrHostOperation>(
    request: CemEdgeSsrHostRequestEnvelope<TOperation>,
    outcome: CemEdgeSsrHostFailureEnvelope<TOperation>['outcome'],
    reason: CemEdgeSsrHostFailureReason,
    diagnostics: CemProcessingDiagnostic[],
    currentRenderState?: EdgeRenderStateRecord
): CemEdgeSsrHostFailureEnvelope<TOperation> {
    const envelope = {
        protocolVersion: CEM_EDGE_SSR_HOST_PROTOCOL_VERSION,
        direction: 'response' as const,
        jobId: request.jobId,
        operation: request.operation,
        outcome,
        reason,
        diagnostics,
        ...(currentRenderState ? { currentRenderState } : {}),
    } as CemEdgeSsrHostFailureEnvelope<TOperation>;
    assertCemEdgeSsrHostEnvelope(envelope);
    return envelope;
}

/** Reject incompatible envelope variants and non-plain structured-clone values. */
export function assertCemEdgeSsrHostEnvelope(envelope: unknown): asserts envelope is CemEdgeSsrHostEnvelope {
    assertProcessingBoundaryValue(envelope, 'CEM Edge/SSR host envelope');
    if (!isPlainRecord(envelope)) {
        throw new TypeError('a CEM Edge/SSR host envelope must be a plain record');
    }
    if (envelope.protocolVersion !== CEM_EDGE_SSR_HOST_PROTOCOL_VERSION) {
        throw new TypeError(`unsupported CEM Edge/SSR host protocol ${String(envelope.protocolVersion)}`);
    }
    if (envelope.direction !== 'request' && envelope.direction !== 'response') {
        throw new TypeError(`unsupported CEM Edge/SSR host direction ${String(envelope.direction)}`);
    }
    if (!CEM_EDGE_SSR_HOST_OPERATIONS.includes(envelope.operation as CemEdgeSsrHostOperation)) {
        throw new TypeError(`unsupported CEM Edge/SSR host operation ${String(envelope.operation)}`);
    }
    if (!Number.isSafeInteger(envelope.jobId) || (envelope.jobId as number) < 1) {
        throw new TypeError('a CEM Edge/SSR host envelope requires a positive safe-integer job ID');
    }
    if (envelope.direction === 'request') {
        if (!isPlainRecord(envelope.payload)) {
            throw new TypeError('a CEM Edge/SSR host request requires a plain payload record');
        }
        return;
    }
    if (
        envelope.outcome !== 'progress'
        && envelope.outcome !== 'success'
        && envelope.outcome !== 'failure'
        && envelope.outcome !== 'cancelled'
    ) {
        throw new TypeError(`unsupported CEM Edge/SSR host outcome ${String(envelope.outcome)}`);
    }
    if (envelope.outcome === 'progress') {
        if (
            envelope.operation !== 'render-update'
            || !isPlainRecord(envelope.result)
            || envelope.result.kind !== 'patch-frame'
            || !isPlainRecord(envelope.result.frame)
        ) {
            throw new TypeError('only render-update patch-frame results may use a progress envelope');
        }
        return;
    }
    if (envelope.outcome === 'success') {
        const expectedKind = envelope.operation === 'render-initial'
            ? 'initial-render'
            : 'render-update-complete';
        if (!isPlainRecord(envelope.result) || envelope.result.kind !== expectedKind) {
            throw new TypeError(`a successful ${String(envelope.operation)} response requires ${expectedKind}`);
        }
        return;
    }
    if (
        !CEM_EDGE_SSR_HOST_FAILURE_REASONS.includes(envelope.reason as CemEdgeSsrHostFailureReason)
        || !Array.isArray(envelope.diagnostics)
    ) {
        throw new TypeError('a failed CEM Edge/SSR host response requires a typed reason and diagnostics');
    }
    if (
        (envelope.outcome === 'cancelled' && envelope.reason !== 'cancelled')
        || (envelope.outcome === 'failure' && envelope.reason === 'cancelled')
    ) {
        throw new TypeError('the cancelled Edge/SSR outcome and reason must be used together');
    }
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}
