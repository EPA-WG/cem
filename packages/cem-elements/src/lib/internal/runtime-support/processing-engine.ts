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
    compileCemMlTemplateArtifact,
    cemMlTemplateArtifactPayloadKey,
    disposeRetainedCemMlTemplate,
    processRetainedCemMlTemplate,
    retainCemMlTemplateArtifact,
    retainCemMlTemplateSource,
    type RetainedCemMlTemplate,
} from './cem-ql-render.js';
import type {
    CemProcessingArtifactBinaryTransfer,
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
import { CemProcessingLruCache } from './processing-cache.js';

interface RetainedTemplateArtifact {
    input: CemProcessingCompileInput;
    handle: CemProcessingArtifactHandle;
    diagnostics: CemProcessingDiagnostic[];
    wasmArtifactId: number;
    compiledArtifact?: CemProcessingArtifactBinaryTransfer;
}

interface CachedTemplateCompilation {
    diagnostics: CemProcessingDiagnostic[];
    wasmArtifactId: number;
    compiledArtifact?: CemProcessingArtifactBinaryTransfer;
}

export interface CemProcessingEngineOptions {
    maxArtifactEntries?: number;
    maxRenderPlanEntries?: number;
}

const DEFAULT_ARTIFACT_CACHE_ENTRIES = 64;
const DEFAULT_RENDER_PLAN_CACHE_ENTRIES = 64;

/** Shared semantic implementation used inside the worker and by main-thread fallback. */
export class CemProcessingEngine {
    private readonly artifacts: CemProcessingLruCache<string, RetainedTemplateArtifact>;
    private readonly compiledArtifacts: CemProcessingLruCache<string, CachedTemplateCompilation>;
    private readonly renderPlans: CemProcessingLruCache<string, RenderPlan>;
    private readonly wasmArtifactIds = new Set<number>();
    private disposed = false;

    constructor(options: CemProcessingEngineOptions = {}) {
        const maxArtifactEntries = options.maxArtifactEntries ?? DEFAULT_ARTIFACT_CACHE_ENTRIES;
        this.artifacts = new CemProcessingLruCache(maxArtifactEntries);
        this.compiledArtifacts = new CemProcessingLruCache(maxArtifactEntries);
        this.renderPlans = new CemProcessingLruCache(
            options.maxRenderPlanEntries ?? DEFAULT_RENDER_PLAN_CACHE_ENTRIES
        );
    }

    async compile(input: CemProcessingCompileInput): Promise<CemProcessingCompileResult> {
        this.assertActive();
        const artifactKey = retainedArtifactKey(input.scopePolicyStamp, input.templateArtifactId);
        const retained = this.artifacts.get(artifactKey);
        if (retained) {
            if (!sameCompileIdentity(retained.input, input)) {
                throw new Error(`template artifact \`${input.templateArtifactId}\` was already retained with another identity`);
            }
            return compileResult(retained);
        }

        const source = processingSourceText(input);
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
                hostBindings: [...new Set(input.hostBindings ?? [])].sort(),
            }).key,
            registrationIdentity: input.registrationIdentity,
            scopePolicyStamp: input.scopePolicyStamp,
            sourceMapMode: input.sourceMapMode,
        };
        let compilation = this.compiledArtifacts.get(handle.cacheKey);
        if (!compilation) {
            const loaded = await this.compileOrImportTemplate(input, source);
            this.assertActive();
            compilation = loaded;
            this.wasmArtifactIds.add(loaded.wasmArtifactId);
            this.compiledArtifacts.set(handle.cacheKey, compilation);
        }
        const artifact = {
            input,
            handle,
            diagnostics: compilation.diagnostics,
            wasmArtifactId: compilation.wasmArtifactId,
            compiledArtifact: compilation.compiledArtifact,
        };
        this.artifacts.set(artifactKey, artifact);
        return compileResult(artifact);
    }

    async renderDiff(input: CemProcessingRenderDiffInput): Promise<CemProcessingRenderDiffResult> {
        this.assertActive();
        const artifact = this.artifacts.get(
            retainedArtifactKey(input.artifact.scopePolicyStamp, input.artifact.artifactId)
        );
        if (!artifact || !sameArtifactHandle(artifact.handle, input.artifact)) {
            throw new Error(`template artifact \`${input.artifact.artifactId}\` is not retained by this processing host`);
        }
        assertRenderRevision(input);
        const previous = retainedPreviousPlan(this.renderPlans, input.previousRenderPlan, input.artifact);
        const processed = await processRetainedCemMlTemplate(artifact.wasmArtifactId, {
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
        for (const artifactId of this.wasmArtifactIds) {
            disposeRetainedCemMlTemplate(artifactId);
        }
        this.wasmArtifactIds.clear();
        this.artifacts.clear();
        this.compiledArtifacts.clear();
        this.renderPlans.clear();
        this.disposed = true;
        return { disposed: true };
    }

    private assertActive(): void {
        if (this.disposed) {
            throw new Error('the CEM processing engine is disposed');
        }
    }

    private async compileOrImportTemplate(
        input: CemProcessingCompileInput,
        source: string
    ): Promise<CachedTemplateCompilation> {
        const hostBindings = input.hostBindings ?? [];
        const payloadKey = await cemMlTemplateArtifactPayloadKey(source, input.sourceMapMode);
        let rejectionDiagnostic: CemProcessingDiagnostic | undefined;
        if (input.precompiledArtifact) {
            try {
                assertPrecompiledTransfer(
                    input.precompiledArtifact,
                    input.scopePolicyStamp,
                    payloadKey
                );
                const retained = await retainCemMlTemplateArtifact(
                    input.precompiledArtifact.bytes,
                    input.precompiledArtifact.cacheKey,
                    source,
                    hostBindings,
                    input.sourceMapMode
                );
                return retainedCompilation(retained);
            } catch (error) {
                rejectionDiagnostic = {
                    code: 'cem.processing_host.precompiled_artifact_rejected',
                    severity: 'warning',
                    message: `${error instanceof Error ? error.message : 'precompiled template artifact was rejected'}; source compilation was used`,
                };
            }
        }

        if (!input.exportCompiledArtifact) {
            const retained = await retainCemMlTemplateSource(source, hostBindings);
            const compilation = retainedCompilation(retained);
            if (rejectionDiagnostic) {
                compilation.diagnostics.unshift(rejectionDiagnostic);
            }
            return compilation;
        }

        const bytes = await compileCemMlTemplateArtifact(source, hostBindings, input.sourceMapMode);
        const artifactBytes = exactArrayBuffer(bytes);
        const retained = await retainCemMlTemplateArtifact(
            artifactBytes,
            '',
            source,
            hostBindings,
            input.sourceMapMode
        );
        if (!retained.contentHash || !retained.formatVersion) {
            throw new Error('source-compiled template artifact did not return stable binary identity');
        }
        const compiledArtifact: CemProcessingArtifactBinaryTransfer = {
            kind: 'template-artifact',
            payloadKey,
            cacheKey: retained.contentHash,
            formatVersion: retained.formatVersion,
            policyStamp: input.scopePolicyStamp,
            bytes: artifactBytes,
        };
        const compilation = retainedCompilation(retained, compiledArtifact);
        if (rejectionDiagnostic) {
            compilation.diagnostics.unshift(rejectionDiagnostic);
        }
        return compilation;
    }
}

function retainedArtifactKey(scopePolicyStamp: string, artifactId: string): string {
    return `${scopePolicyStamp}\u0000${artifactId}`;
}

function compileResult(artifact: RetainedTemplateArtifact): CemProcessingCompileResult {
    return {
        artifact: artifact.handle,
        declaredAttributes: [],
        observedAttributes: [],
        invalidationScopes: ['host-attributes', 'payload', 'slices', 'forms', 'events'],
        diagnostics: artifact.diagnostics,
        ...(artifact.compiledArtifact === undefined
            ? {}
            : { compiledArtifact: cloneArtifactTransfer(artifact.compiledArtifact) }),
    };
}

function sameCompileIdentity(left: CemProcessingCompileInput, right: CemProcessingCompileInput): boolean {
    return left.registrationIdentity === right.registrationIdentity
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.sourceMapMode === right.sourceMapMode
        && left.exportCompiledArtifact === right.exportCompiledArtifact
        && sameStrings(left.hostBindings ?? [], right.hostBindings ?? [])
        && processingSourceText(left) === processingSourceText(right)
        && left.sourceRef.kind === right.sourceRef.kind
        && left.sourceRef.value === right.sourceRef.value
        && left.resolverIdentity === right.resolverIdentity;
}

function retainedCompilation(
    retained: RetainedCemMlTemplate,
    compiledArtifact?: CemProcessingArtifactBinaryTransfer
): CachedTemplateCompilation {
    return {
        wasmArtifactId: retained.artifactId,
        diagnostics: retained.diagnostics.filter((diagnostic) =>
            diagnostic.code.startsWith('cem.tokenizer.')
        ),
        compiledArtifact,
    };
}

function assertPrecompiledTransfer(
    artifact: CemProcessingArtifactBinaryTransfer,
    scopePolicyStamp: string,
    payloadKey: CemProcessingArtifactBinaryTransfer['payloadKey']
): void {
    if (artifact.formatVersion !== 'cem-template-artifact/1') {
        throw new Error(`unsupported template artifact format ${artifact.formatVersion}`);
    }
    if (artifact.policyStamp !== scopePolicyStamp) {
        throw new Error('cem.cc.policy_mismatch: template artifact policy stamp does not match the active scope');
    }
    if (JSON.stringify(artifact.payloadKey) !== JSON.stringify(payloadKey)) {
        throw new Error('component-template artifact payload key does not match the active source or runtime versions');
    }
    if (!(artifact.bytes instanceof ArrayBuffer)) {
        throw new TypeError('a precompiled template artifact requires ArrayBuffer bytes');
    }
}

function exactArrayBuffer(bytes: Uint8Array): ArrayBuffer {
    return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength) as ArrayBuffer;
}

function cloneArtifactTransfer(
    artifact: CemProcessingArtifactBinaryTransfer
): CemProcessingArtifactBinaryTransfer {
    return {
        ...artifact,
        bytes: artifact.bytes.slice(0),
    };
}

function sameStrings(left: readonly string[], right: readonly string[]): boolean {
    const canonical = (values: readonly string[]) => [...new Set(values.filter(Boolean))].sort();
    const leftCanonical = canonical(left);
    const rightCanonical = canonical(right);
    return leftCanonical.length === rightCanonical.length
        && leftCanonical.every((value, index) => value === rightCanonical[index]);
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
            if (node.tag === 'repository-query') {
                const control = lowerRepositoryQueryControl(node);
                if (control) {
                    resourceControls.push(control);
                }
                continue;
            }
            if (node.tag === 'storage-status') {
                const control = lowerStorageStatusControl(node);
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

function lowerRepositoryQueryControl(
    node: Extract<RenderPlanNode, { kind: 'element' }>
): CemProcessingResourceControl | null {
    const attributes = renderAttributeRecord(node.attributes);
    const sliceName = attributes.slice?.trim() ?? '';
    const repository = attributes.repository?.trim() ?? '';
    const operation = attributes.operation?.trim() ?? '';
    if (!sliceName || !repository || !operation) {
        return null;
    }
    const parameters = optionalControlAttribute(attributes, 'parameters');
    const cursor = optionalControlAttribute(attributes, 'cursor');
    return {
        kind: 'repository-query',
        renderNodeId: node.renderNodeId,
        sliceName,
        repository,
        operation,
        ...(parameters === undefined ? {} : { parameters }),
        live: controlBooleanAttribute(attributes, 'live'),
        ...(cursor === undefined ? {} : { cursor }),
        ...(node.sourceMapRef === undefined ? {} : { sourceMapRef: node.sourceMapRef })
    };
}

function lowerStorageStatusControl(
    node: Extract<RenderPlanNode, { kind: 'element' }>
): CemProcessingResourceControl | null {
    const attributes = renderAttributeRecord(node.attributes);
    const sliceName = attributes.slice?.trim() ?? '';
    const repository = attributes.repository?.trim() ?? '';
    if (!sliceName || !repository) {
        return null;
    }
    const cursor = optionalControlAttribute(attributes, 'cursor');
    return {
        kind: 'storage-status',
        renderNodeId: node.renderNodeId,
        sliceName,
        repository,
        live: controlBooleanAttribute(attributes, 'live'),
        ...(cursor === undefined ? {} : { cursor }),
        ...(node.sourceMapRef === undefined ? {} : { sourceMapRef: node.sourceMapRef })
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

function controlBooleanAttribute(attributes: Record<string, string>, name: string): boolean {
    if (!(name in attributes)) {
        return false;
    }
    const value = attributes[name]?.trim().toLowerCase();
    return value !== 'false' && value !== '0';
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
    renderPlans: CemProcessingLruCache<string, RenderPlan>,
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
        return null;
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
