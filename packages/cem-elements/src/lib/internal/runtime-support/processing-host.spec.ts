import { describe, expect, it, vi } from 'vitest';

import { createCemDeclarationScope } from '../../declaration-scope.js';
import type { DataIslandSnapshot } from '../../cem-elements.js';
import type { RenderRevision } from '../../projection.js';
import {
    CEM_PROCESSING_HOST_PROTOCOL_VERSION,
    CemProcessingJobSequence,
    assertCemProcessingEnvelope,
    cemProcessingHostOwnerScope,
    createCemProcessingTextSource,
    createCemProcessingReadyEnvelope,
    createCemProcessingRequestEnvelope,
    createCemProcessingSuccessEnvelope,
    decideCemProcessingWorkerFailure,
    type CemProcessingArtifactHandle,
    type CemProcessingWorkerFactory,
} from './processing-host.js';

const REVISION: RenderRevision = {
    instanceId: 'cem-card-1',
    dataRevision: 'data-7',
    templateArtifactId: 'artifact-card-v1',
    scopePolicyStamp: 'scope-policy-v2',
    outputTarget: 'light-dom',
    renderAttempt: 3,
};

describe('Phase 3A processing-host contract', () => {
    it('owns one host at the logical root while keeping independent roots isolated', () => {
        const document = {} as Document;
        const root = createCemDeclarationScope({ document });
        const child = createCemDeclarationScope({ document, parent: root });
        const grandchild = createCemDeclarationScope({ document, parent: child });
        const independentRoot = createCemDeclarationScope({ document });

        expect(cemProcessingHostOwnerScope(root)).toBe(root);
        expect(cemProcessingHostOwnerScope(child)).toBe(root);
        expect(cemProcessingHostOwnerScope(grandchild)).toBe(root);
        expect(cemProcessingHostOwnerScope(independentRoot)).toBe(independentRoot);
    });

    it('assigns monotonic job IDs to every operation in versioned clone-safe envelopes', () => {
        const sequence = new CemProcessingJobSequence();
        const ready = createCemProcessingReadyEnvelope('worker');
        const compile = createCemProcessingRequestEnvelope(sequence, 'compile', {
            language: 'cem-ml',
            producedTag: 'cem-card',
            templateArtifactId: REVISION.templateArtifactId,
            registrationIdentity: 'cem-registration-v1:card',
            source: createCemProcessingTextSource('<template><p>{$title}</p></template>', 8),
            sourceRef: { kind: 'inline', value: 'cem-card' },
            resolverIdentity: 'document:https://example.test/components/',
            scopePolicyStamp: REVISION.scopePolicyStamp,
            sourceMapMode: 'dev',
        });
        const cancel = createCemProcessingRequestEnvelope(sequence, 'cancel', {
            targetJobId: compile.jobId,
            reason: 'superseded',
        });
        const dispose = createCemProcessingRequestEnvelope(sequence, 'dispose', {
            reason: 'scope-disposed',
        });

        expect([compile.jobId, cancel.jobId, dispose.jobId]).toEqual([1, 2, 3]);
        expect(structuredClone(ready)).toEqual(ready);
        expect(compile.protocolVersion).toBe(CEM_PROCESSING_HOST_PROTOCOL_VERSION);
        expect(structuredClone(compile)).toEqual(compile);
        expect(structuredClone(cancel)).toEqual(cancel);
        expect(structuredClone(dispose)).toEqual(dispose);
        expect(() => assertCemProcessingEnvelope({ ...compile, protocolVersion: 'future-v2' })).toThrow(
            /unsupported CEM processing-host protocol/
        );
        expect(() => assertCemProcessingEnvelope({ ...compile, jobId: 0 })).toThrow(/positive safe-integer job ID/);
    });

    it('carries the complete render revision and retained artifact/plan handles', () => {
        const sequence = new CemProcessingJobSequence();
        const artifact = artifactHandle();
        const previousRenderPlan = {
            kind: 'render-plan-handle' as const,
            renderPlanId: 'plan-card-6',
            templateArtifactId: artifact.artifactId,
            revision: { ...REVISION, dataRevision: 'data-6', renderAttempt: 0 },
            renderEngineVersion: '1.0.0',
            sourceMapMode: 'dev' as const,
        };
        const request = createCemProcessingRequestEnvelope(sequence, 'render-diff', {
            artifact,
            revision: REVISION,
            snapshot: snapshotFixture(),
            data: { title: 'Card' },
            scopeUid: 'card-scope',
            instanceScopeUid: 'card-instance-scope',
            previousRenderPlan,
            patchBatchSize: 16,
        });
        const nextRenderPlan = {
            ...previousRenderPlan,
            renderPlanId: 'plan-card-7',
            revision: REVISION,
        };
        const response = createCemProcessingSuccessEnvelope(request, {
            revision: REVISION,
            nextRenderPlan,
            frames: [
                { type: 'begin', transactionId: 'render-7-attempt-3', revision: REVISION },
                {
                    type: 'commit',
                    transactionId: 'render-7-attempt-3',
                    nextRenderPlan: { producedTag: 'cem-card', ...REVISION },
                },
            ],
            resourceControls: [],
            diagnostics: [],
        });

        expect(request.payload.revision).toEqual(REVISION);
        expect(request.payload.previousRenderPlan?.templateArtifactId).toBe(artifact.artifactId);
        expect(response.jobId).toBe(request.jobId);
        expect(response.result.revision).toEqual(REVISION);
        expect(response.result.nextRenderPlan).toEqual(nextRenderPlan);
        expect(structuredClone(request)).toEqual(request);
        expect(structuredClone(response)).toEqual(response);
    });

    it('constructs a module worker through an injected factory seam', () => {
        const worker = {} as Worker;
        const factory: CemProcessingWorkerFactory = vi.fn(() => worker);
        const scriptUrl = new URL('https://example.test/assets/cem-processing.worker.js');

        expect(factory({ scriptUrl, name: 'cem-processing-root-1', type: 'module' })).toBe(worker);
        expect(factory).toHaveBeenCalledWith({ scriptUrl, name: 'cem-processing-root-1', type: 'module' });
    });

    it.each(['startup', 'execution'] as const)(
        'retries a pure compile once through fallback after %s failure',
        (phase) => {
            expect(decideCemProcessingWorkerFailure({ phase, operation: 'compile' })).toEqual({
                action: 'retry-main-thread',
                nextMode: 'main-thread',
                allocateNewJobId: true,
                ignoreLateWorkerResult: true,
                abortTransaction: false,
                diagnostic: expect.objectContaining({
                    code: `cem.processing_host.worker_${phase === 'startup' ? 'startup' : 'execution'}_fallback`,
                    severity: 'warning',
                }),
            });
        }
    );

    it('retries an uncommitted render and starts a fresh attempt after frames began', () => {
        const notStarted = decideCemProcessingWorkerFailure({
            phase: 'startup',
            operation: 'render-diff',
            transactionState: 'not-started',
            revision: REVISION,
        });
        const begun = decideCemProcessingWorkerFailure({
            phase: 'execution',
            operation: 'render-diff',
            transactionState: 'begun',
            revision: REVISION,
        });

        expect(notStarted).toEqual(expect.objectContaining({
            action: 'retry-main-thread',
            allocateNewJobId: true,
            abortTransaction: false,
            retryRevision: REVISION,
        }));
        expect(begun).toEqual(expect.objectContaining({
            action: 'abort-and-retry-main-thread',
            allocateNewJobId: true,
            abortTransaction: true,
            retryRevision: { ...REVISION, renderAttempt: 4 },
        }));
    });

    it('never replays committed renders or control operations', () => {
        const committed = decideCemProcessingWorkerFailure({
            phase: 'execution',
            operation: 'render-diff',
            transactionState: 'committed',
            revision: REVISION,
        });
        const cancel = decideCemProcessingWorkerFailure({ phase: 'execution', operation: 'cancel' });
        const dispose = decideCemProcessingWorkerFailure({ phase: 'execution', operation: 'dispose' });

        expect(committed).toEqual(expect.objectContaining({
            action: 'preserve-committed-result',
            allocateNewJobId: false,
        }));
        expect(cancel).toEqual(expect.objectContaining({
            action: 'complete-control-without-retry',
            nextMode: 'main-thread',
            allocateNewJobId: false,
        }));
        expect(dispose).toEqual(expect.objectContaining({
            action: 'complete-control-without-retry',
            nextMode: 'disposed',
            allocateNewJobId: false,
        }));
    });

    it('ignores late worker failures after the one fallback transition', () => {
        expect(decideCemProcessingWorkerFailure({
            phase: 'execution',
            operation: 'compile',
            fallbackAlreadySelected: true,
        })).toEqual({
            action: 'ignore-duplicate-worker-failure',
            nextMode: 'main-thread',
            allocateNewJobId: false,
            ignoreLateWorkerResult: true,
            abortTransaction: false,
        });
    });
});

function artifactHandle(): CemProcessingArtifactHandle {
    return {
        kind: 'template-artifact-handle',
        artifactId: REVISION.templateArtifactId,
        cacheKey: 'cem-template-artifact:card-v1',
        registrationIdentity: 'cem-registration-v1:card',
        scopePolicyStamp: REVISION.scopePolicyStamp,
        sourceMapMode: 'dev',
    };
}

function snapshotFixture(): DataIslandSnapshot {
    return {
        version: '1.2.0',
        instanceId: REVISION.instanceId,
        producedTag: 'cem-card',
        declarationTag: 'cem-element',
        templateArtifactId: REVISION.templateArtifactId,
        dataRevision: REVISION.dataRevision,
        renderAttempt: REVISION.renderAttempt,
        outputTarget: REVISION.outputTarget,
        scopePolicyStamp: REVISION.scopePolicyStamp,
        privacyPolicyStamp: 'privacy-v1',
        hostAttributes: {},
        dataset: {},
        payload: { roots: [], byKey: {}, choices: [], data: [], dataByValue: {}, text: '' },
        slices: {},
        validationState: {},
        eventPayloads: {},
    };
}
