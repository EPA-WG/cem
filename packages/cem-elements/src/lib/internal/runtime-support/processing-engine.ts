import {
    RENDER_ENGINE_VERSION,
    diffRenderPlansToPatchFrames,
    edgeContentAddress,
    renderPlanIdentity,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type RenderPlan,
    type RenderPlanAttribute,
    type RenderPlanNode,
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
    CemProcessingResourceControl,
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

        const source = processingSourceText(input);
        const diagnostics = await compileCemMlTemplate(source);
        this.assertActive();
        const handle: CemProcessingArtifactHandle = {
            kind: 'template-artifact-handle',
            artifactId: input.templateArtifactId,
            cacheKey: edgeContentAddress('template-artifact', {
                language: input.language,
                source,
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
            source: processingSourceText(artifact.input),
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
        const lowered = lowerResourceControls(scoped.renderPlan);
        const frames = diffRenderPlansToPatchFrames(previous, lowered.renderPlan, {
            batchSize: input.patchBatchSize,
        });
        const renderPlanId = edgeContentAddress('render-plan', lowered.renderPlan).key;
        const nextRenderPlan: CemProcessingRenderPlanHandle = {
            kind: 'render-plan-handle',
            renderPlanId,
            templateArtifactId: input.artifact.artifactId,
            revision: renderPlanIdentity(scoped.renderPlan),
            renderEngineVersion: RENDER_ENGINE_VERSION,
            sourceMapMode: input.artifact.sourceMapMode,
        };
        this.renderPlans.set(renderPlanId, lowered.renderPlan);
        const generatedIdDiagnostics = validateRenderPlanGeneratedIds(lowered.renderPlan);
        return {
            revision: input.revision,
            nextRenderPlan,
            frames,
            resourceControls: lowered.resourceControls,
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
        && processingSourceText(left) === processingSourceText(right)
        && left.sourceRef.kind === right.sourceRef.kind
        && left.sourceRef.value === right.sourceRef.value
        && left.resolverIdentity === right.resolverIdentity;
}

function processingSourceText(input: CemProcessingCompileInput): string {
    if (input.source.kind !== 'text-chunks-v1') {
        throw new TypeError(`unsupported CEM processing source kind ${String(input.source.kind)}`);
    }
    return input.source.chunks.join('');
}

function lowerResourceControls(plan: RenderPlan): {
    renderPlan: RenderPlan;
    resourceControls: CemProcessingResourceControl[];
} {
    const resourceControls: CemProcessingResourceControl[] = [];
    const lowerNodes = (nodes: RenderPlanNode[]): RenderPlanNode[] => {
        const retained: RenderPlanNode[] = [];
        for (const node of nodes) {
            if (node.kind !== 'element') {
                retained.push(node);
                continue;
            }
            if (node.tag === 'http-request') {
                const control = lowerHttpRequestControl(node);
                if (control) {
                    resourceControls.push(control);
                }
                continue;
            }
            retained.push({ ...node, children: lowerNodes(node.children) });
        }
        return retained;
    };
    return {
        renderPlan: { ...plan, nodes: lowerNodes(plan.nodes) },
        resourceControls,
    };
}

function lowerHttpRequestControl(
    node: Extract<RenderPlanNode, { kind: 'element' }>
): CemProcessingResourceControl | null {
    const attributes = renderAttributeRecord(node.attributes);
    const sliceName = attributes.slice?.trim() ?? '';
    const authoredUrl = attributes.url?.trim() ?? '';
    if (!sliceName || !authoredUrl) {
        return null;
    }
    const headers: Record<string, string> = {};
    for (const [name, value] of Object.entries(attributes)) {
        if (name.startsWith('header-') && name.length > 'header-'.length) {
            headers[name.slice('header-'.length).trim().toLowerCase()] = value;
        }
    }
    const expectedContentType = optionalControlAttribute(attributes, 'content-type');
    const credentials = optionalControlAttribute(attributes, 'credentials');
    const cache = optionalControlAttribute(attributes, 'cache');
    return {
        kind: 'http-request',
        renderNodeId: node.renderNodeId,
        sliceName,
        authoredUrl,
        method: (attributes.method?.trim() || 'GET').toUpperCase(),
        headers,
        ...(expectedContentType === undefined ? {} : { expectedContentType }),
        ...(credentials === undefined ? {} : { credentials }),
        ...(cache === undefined ? {} : { cache }),
        ...(node.sourceMapRef === undefined ? {} : { sourceMapRef: node.sourceMapRef }),
    };
}

function renderAttributeRecord(attributes: RenderPlanAttribute[]): Record<string, string> {
    return Object.fromEntries(attributes.map((attribute) => [attribute.name.toLowerCase(), attribute.value]));
}

function optionalControlAttribute(
    attributes: Record<string, string>,
    name: string
): string | undefined {
    const value = attributes[name]?.trim();
    return value ? value : undefined;
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
