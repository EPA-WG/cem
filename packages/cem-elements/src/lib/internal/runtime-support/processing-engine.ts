import {
    RENDER_ENGINE_VERSION,
    diffRenderPlansToPatchFrames,
    edgeContentAddress,
    renderPlanIdentity,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type RenderPlan,
} from '../../projection.js';
import {
    compileCemMlTemplate,
    processCemMlTemplate,
} from './cem-ql-render.js';
import type {
    CemProcessingArtifactHandle,
    CemProcessingCompileInput,
    CemProcessingCompileResult,
    CemProcessingDiagnostic,
    CemProcessingDisposeInput,
    CemProcessingDisposeResult,
    CemProcessingRenderDiffInput,
    CemProcessingRenderDiffResult,
    CemProcessingRenderPlanHandle,
} from './processing-host.js';

interface RetainedTemplateArtifact {
    input: CemProcessingCompileInput;
    handle: CemProcessingArtifactHandle;
    diagnostics: CemProcessingDiagnostic[];
}

/** Shared semantic implementation used inside the worker and by main-thread fallback. */
export class CemProcessingEngine {
    private readonly artifacts = new Map<string, RetainedTemplateArtifact>();
    private readonly renderPlans = new Map<string, RenderPlan>();
    private disposed = false;

    async compile(input: CemProcessingCompileInput): Promise<CemProcessingCompileResult> {
        this.assertActive();
        const retained = this.artifacts.get(input.templateArtifactId);
        if (retained) {
            if (!sameCompileIdentity(retained.input, input)) {
                throw new Error(`template artifact \`${input.templateArtifactId}\` was already retained with another identity`);
            }
            return compileResult(retained);
        }

        const diagnostics = await compileCemMlTemplate(input.source);
        this.assertActive();
        const handle: CemProcessingArtifactHandle = {
            kind: 'template-artifact-handle',
            artifactId: input.templateArtifactId,
            cacheKey: edgeContentAddress('template-artifact', {
                language: input.language,
                source: input.source,
                sourceRef: input.sourceRef,
                resolverIdentity: input.resolverIdentity,
                scopePolicyStamp: input.scopePolicyStamp,
                sourceMapMode: input.sourceMapMode,
            }).key,
            registrationIdentity: input.registrationIdentity,
            scopePolicyStamp: input.scopePolicyStamp,
            sourceMapMode: input.sourceMapMode,
        };
        const artifact = { input, handle, diagnostics };
        this.artifacts.set(handle.artifactId, artifact);
        return compileResult(artifact);
    }

    async renderDiff(input: CemProcessingRenderDiffInput): Promise<CemProcessingRenderDiffResult> {
        this.assertActive();
        const artifact = this.artifacts.get(input.artifact.artifactId);
        if (!artifact || !sameArtifactHandle(artifact.handle, input.artifact)) {
            throw new Error(`template artifact \`${input.artifact.artifactId}\` is not retained by this processing host`);
        }
        assertRenderRevision(input);
        const previous = retainedPreviousPlan(this.renderPlans, input.previousRenderPlan, input.artifact);
        const processed = await processCemMlTemplate({
            source: artifact.input.source,
            data: input.data,
            payload: input.snapshot.payload,
            identity: {
                producedTag: artifact.input.producedTag,
                ...input.revision,
            },
            renderNodeIdPrefix: artifact.input.producedTag,
        });
        this.assertActive();
        const scoped = scopeRenderPlan(processed.renderPlan, input.scopeUid, {
            instanceScopeUid: input.instanceScopeUid,
        });
        const frames = diffRenderPlansToPatchFrames(previous, scoped.renderPlan, {
            batchSize: input.patchBatchSize,
        });
        const renderPlanId = edgeContentAddress('render-plan', scoped.renderPlan).key;
        const nextRenderPlan: CemProcessingRenderPlanHandle = {
            kind: 'render-plan-handle',
            renderPlanId,
            templateArtifactId: input.artifact.artifactId,
            revision: renderPlanIdentity(scoped.renderPlan),
            renderEngineVersion: RENDER_ENGINE_VERSION,
            sourceMapMode: input.artifact.sourceMapMode,
        };
        this.renderPlans.set(renderPlanId, scoped.renderPlan);
        const generatedIdDiagnostics = validateRenderPlanGeneratedIds(scoped.renderPlan);
        return {
            revision: input.revision,
            nextRenderPlan,
            frames,
            diagnostics: [
                ...processed.diagnostics,
                ...scoped.diagnostics.map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
                ...generatedIdDiagnostics.map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
            ],
        };
    }

    dispose(_input: CemProcessingDisposeInput): CemProcessingDisposeResult {
        this.artifacts.clear();
        this.renderPlans.clear();
        this.disposed = true;
        return { disposed: true };
    }

    private assertActive(): void {
        if (this.disposed) {
            throw new Error('the CEM processing engine is disposed');
        }
    }
}

function compileResult(artifact: RetainedTemplateArtifact): CemProcessingCompileResult {
    return {
        artifact: artifact.handle,
        declaredAttributes: [],
        observedAttributes: [],
        invalidationScopes: ['host-attributes', 'payload', 'slices', 'forms', 'events'],
        diagnostics: artifact.diagnostics,
    };
}

function sameCompileIdentity(left: CemProcessingCompileInput, right: CemProcessingCompileInput): boolean {
    return left.registrationIdentity === right.registrationIdentity
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.sourceMapMode === right.sourceMapMode
        && left.source === right.source;
}

function sameArtifactHandle(left: CemProcessingArtifactHandle, right: CemProcessingArtifactHandle): boolean {
    return left.kind === right.kind
        && left.artifactId === right.artifactId
        && left.cacheKey === right.cacheKey
        && left.registrationIdentity === right.registrationIdentity
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.sourceMapMode === right.sourceMapMode;
}

function retainedPreviousPlan(
    renderPlans: Map<string, RenderPlan>,
    handle: CemProcessingRenderPlanHandle | null | undefined,
    artifact: CemProcessingArtifactHandle
): RenderPlan | null {
    if (!handle) {
        return null;
    }
    if (handle.templateArtifactId !== artifact.artifactId) {
        throw new Error('the retained previous render plan belongs to another template artifact');
    }
    const plan = renderPlans.get(handle.renderPlanId);
    if (!plan) {
        throw new Error(`render plan \`${handle.renderPlanId}\` is not retained by this processing host`);
    }
    return plan;
}

function assertRenderRevision(input: CemProcessingRenderDiffInput): void {
    const { revision, snapshot, artifact } = input;
    if (
        revision.instanceId !== snapshot.instanceId
        || revision.dataRevision !== snapshot.dataRevision
        || revision.templateArtifactId !== snapshot.templateArtifactId
        || revision.templateArtifactId !== artifact.artifactId
        || revision.scopePolicyStamp !== snapshot.scopePolicyStamp
        || revision.outputTarget !== snapshot.outputTarget
        || revision.renderAttempt !== snapshot.renderAttempt
    ) {
        throw new Error('the CEM processing render revision does not match its snapshot and artifact');
    }
}
