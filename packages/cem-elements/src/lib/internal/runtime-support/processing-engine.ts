import { CemXPathFunctionLibraries, type CemXPathFunctionLibraryLease } from './xpath-function-library.js';
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
    retainLoadedCemDocument,
    disposeLoadedCemDocument,
    compileCemMlTemplateArtifact,
    cemMlTemplateArtifactPayloadKey,
    disposeRetainedCemMlTemplate,
    processRetainedCemMlTemplate,
    retainCemMlTemplateArtifact,
    retainCemMlTemplateSource,
    retainCemMlTemplateModuleClosure,
    retainXsltComponentSource,
    type RetainedXsltComponent,
    type CemQlStylesheetArtifact,
    type RetainedCemMlTemplate,
} from './cem-ql-render.js';
import type {
    CemProcessingArtifactBinaryTransfer,
    CemProcessingArtifactHandle,
    CemProcessingCompileInput,
    CemProcessingCompileResult,
    CemProcessingDocumentInput,
    CemProcessingDocumentResult,
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
    compilation: CachedTemplateCompilation;
    input: CemProcessingCompileInput;
    handle: CemProcessingArtifactHandle;
    diagnostics: CemProcessingDiagnostic[];
    wasmArtifactId: number;
    compiledArtifact?: CemProcessingArtifactBinaryTransfer;
    stylesheets?: CemQlStylesheetArtifact[];
}

interface CachedTemplateCompilation {
    xslt?: RetainedXsltComponent;
    xpathLibrary?: CemXPathFunctionLibraryLease;
    diagnostics: CemProcessingDiagnostic[];
    wasmArtifactId: number;
    compiledArtifact?: CemProcessingArtifactBinaryTransfer;
    stylesheets?: CemQlStylesheetArtifact[];
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
    private readonly compilations = new Map<CachedTemplateCompilation, number>();
    private readonly xpathLibraries = new CemXPathFunctionLibraries();
    private readonly documents = new Map<string, { input: Extract<CemProcessingDocumentInput, { action: 'retain' }>; id: number }>();
    private readonly documentOperations = new Map<string, object>();
    private disposed = false;

    constructor(options: CemProcessingEngineOptions = {}) {
        const maxArtifactEntries = options.maxArtifactEntries ?? DEFAULT_ARTIFACT_CACHE_ENTRIES;
        this.artifacts = new CemProcessingLruCache(maxArtifactEntries);
        this.compiledArtifacts = new CemProcessingLruCache(maxArtifactEntries);
        this.renderPlans = new CemProcessingLruCache(
            options.maxRenderPlanEntries ?? DEFAULT_RENDER_PLAN_CACHE_ENTRIES
        );
    }

    async document(input: CemProcessingDocumentInput): Promise<CemProcessingDocumentResult> {
        this.assertActive();
        const key = JSON.stringify(input.handle);
        if (input.action === 'release') {
            this.documentOperations.delete(key);
            const retained = this.documents.get(key);
            if (retained) disposeLoadedCemDocument(retained.id);
            this.documents.delete(key);
            return { handle: input.handle, retained: false };
        }
        const existing = this.documents.get(key);
        if (existing) {
            if (existing.input.contentType !== input.contentType || existing.input.sourceUri !== input.sourceUri
                || !sameDocumentBytes(existing.input.bytes, input.bytes)) {
                throw new Error('CEM document handle was reused with different source bytes or metadata');
            }
            return { handle: input.handle, retained: true };
        }
        const operation = {};
        this.documentOperations.set(key, operation);
        let id: number;
        try {
            id = await retainLoadedCemDocument(input.bytes, input.contentType, input.sourceUri);
        } catch (error) {
            if (this.documentOperations.get(key) === operation) this.documentOperations.delete(key);
            throw error;
        }
        if (this.disposed || this.documentOperations.get(key) !== operation) {
            disposeLoadedCemDocument(id);
            throw new Error('CEM document import was superseded or released');
        }
        this.documentOperations.delete(key);
        this.documents.set(key, { input, id });
        return { handle: input.handle, retained: true };
    }

    async compile(input: CemProcessingCompileInput): Promise<CemProcessingCompileResult> {
        this.assertActive();
        if (input.language === 'xslt') {
            if (!input.xslt || input.moduleClosure || input.xpathFunctionLibrary || input.precompiledArtifact || input.exportCompiledArtifact) {
                throw new Error('XSLT compilation requires its explicit source/options contract');
            }
        } else if (input.language !== 'cem-ml' || input.xslt) {
            throw new Error('invalid processing language or XSLT options');
        }
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
                moduleClosure: input.moduleClosure ?? null,
                xpathFunctionLibrary: input.xpathFunctionLibrary ?? null,
                xslt: input.xslt ?? null,
            }).key,
            registrationIdentity: input.registrationIdentity,
            scopePolicyStamp: input.scopePolicyStamp,
            sourceMapMode: input.sourceMapMode,
        };
        let compilation = this.compiledArtifacts.get(handle.cacheKey);
        if (!compilation) {
            const xpathLibrary = input.xpathFunctionLibrary
                ? await this.xpathLibraries.acquire(input.xpathFunctionLibrary) : undefined;
            let loaded: CachedTemplateCompilation | undefined;
            try {
                this.assertActive();
                loaded = await this.compileOrImportTemplate(input, source);
                this.assertActive();
                // A concurrent compile may have retained the same identity while
                // this request awaited WASM. Reuse it and release the extra lease.
                compilation = this.compiledArtifacts.get(handle.cacheKey);
                if (compilation) {
                    disposeCompilation(loaded);
                    xpathLibrary?.release();
                } else {
                    compilation = { ...loaded, xpathLibrary };
                    this.compilations.set(compilation, 1);
                    const evicted = this.compiledArtifacts.set(handle.cacheKey, compilation);
                    if (evicted) this.releaseCompilation(evicted.value);
                }
            } catch (error) {
                if (loaded) disposeCompilation(loaded);
                xpathLibrary?.release();
                throw error;
            }
        }
        const concurrentlyRetained = this.artifacts.get(artifactKey);
        if (concurrentlyRetained) {
            if (!sameCompileIdentity(concurrentlyRetained.input, input)) {
                throw new Error(`template artifact \`${input.templateArtifactId}\` was already retained with another identity`);
            }
            return compileResult(concurrentlyRetained);
        }
        this.compilations.set(compilation, (this.compilations.get(compilation) ?? 0) + 1);
        const artifact = {
            compilation,
            input,
            handle,
            diagnostics: compilation.diagnostics,
            wasmArtifactId: compilation.wasmArtifactId,
            compiledArtifact: compilation.compiledArtifact,
            stylesheets: compilation.stylesheets,
        };
        const evicted = this.artifacts.set(artifactKey, artifact);
        if (evicted) this.releaseCompilation(evicted.value.compilation);
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
            xslt: artifact.compilation.xslt,
            xpathCompanionId: artifact.compilation.xpathLibrary?.companionId,
            documents: (input.documents ?? []).map(({ slice, handle }) => {
                if (handle.instanceId !== input.revision.instanceId || handle.scopePolicyStamp !== input.revision.scopePolicyStamp) {
                    throw new Error('CEM document binding belongs to another instance or scope');
                }
                const document = this.documents.get(JSON.stringify(handle));
                if (!document) throw new Error('CEM document is not retained by this processing host');
                return { slice, documentId: document.id };
            }),
            nativeAttributes: input.nativeAttributes,
            nativeValueLimits: input.nativeValueLimits,
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
            payload: input.snapshot.payload,
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
            hostAttributeUpdates: processed.hostAttributeUpdates,
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
        for (const document of this.documents.values()) disposeLoadedCemDocument(document.id);
        this.documents.clear();
        this.documentOperations.clear();
        for (const compilation of this.compilations.keys()) {
            disposeCompilation(compilation);
            compilation.xpathLibrary?.release();
        }
        this.compilations.clear();
        this.xpathLibraries.dispose();
        this.artifacts.clear();
        this.compiledArtifacts.clear();
        this.renderPlans.clear();
        this.disposed = true;
        return { disposed: true };
    }

    private releaseCompilation(compilation: CachedTemplateCompilation): void {
        const references = (this.compilations.get(compilation) ?? 0) - 1;
        if (references > 0) {
            this.compilations.set(compilation, references);
        } else {
            this.compilations.delete(compilation);
            disposeCompilation(compilation);
            compilation.xpathLibrary?.release();
        }
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
        if (input.xslt) {
            const xslt = await retainXsltComponentSource(source, input.xslt.sourceUri, input.xslt.options, hostBindings, input.xslt.controlPolicy);
            return { xslt, wasmArtifactId: 0, diagnostics: xslt.diagnostics, stylesheets: xslt.stylesheets };
        }
        if (input.moduleClosure) {
            // A source-only binary is not a dependency-closure artifact. Keep the
            // closure content/version/resolver contract in the native compilation.
            const retained = await retainCemMlTemplateModuleClosure(source, input.moduleClosure, hostBindings);
            return {
                wasmArtifactId: retained.artifactId,
                diagnostics: retained.diagnostics,
                stylesheets: retained.stylesheets,
            };
        }
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
        ...(artifact.stylesheets === undefined ? {} : { stylesheets: artifact.stylesheets }),
        ...(artifact.compiledArtifact === undefined
            ? {}
            : { compiledArtifact: cloneArtifactTransfer(artifact.compiledArtifact) }),
    };
}

function sameCompileIdentity(left: CemProcessingCompileInput, right: CemProcessingCompileInput): boolean {
    return left.language === right.language
        && left.registrationIdentity === right.registrationIdentity
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.sourceMapMode === right.sourceMapMode
        && left.exportCompiledArtifact === right.exportCompiledArtifact
        && JSON.stringify(left.moduleClosure ?? null) === JSON.stringify(right.moduleClosure ?? null)
        && JSON.stringify(left.xpathFunctionLibrary ?? null) === JSON.stringify(right.xpathFunctionLibrary ?? null)
        && JSON.stringify(left.xslt ?? null) === JSON.stringify(right.xslt ?? null)
        && sameStrings(left.hostBindings ?? [], right.hostBindings ?? [])
        && processingSourceText(left) === processingSourceText(right)
        && left.sourceRef.kind === right.sourceRef.kind
        && left.sourceRef.value === right.sourceRef.value
        && left.resolverIdentity === right.resolverIdentity;
}

function disposeCompilation(compilation: CachedTemplateCompilation): void {
    if (compilation.xslt) compilation.xslt.dispose();
    else disposeRetainedCemMlTemplate(compilation.wasmArtifactId);
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
            if (node.tag === 'cem-module-url' || node.tag === 'module-url') {
                const control = lowerModuleUrlControl(node);
                if (control) {
                    resourceControls.push(control);
                }
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

function lowerModuleUrlControl(
    node: Extract<RenderPlanNode, { kind: 'element' }>
): CemProcessingResourceControl | null {
    const attributes = renderAttributeRecord(node.attributes);
    const sliceName = attributes.slice?.trim() ?? '';
    const authoredSpecifier = attributes.src?.trim() ?? '';
    if (!sliceName || !authoredSpecifier) {
        return null;
    }
    const referrer = optionalControlAttribute(attributes, 'referrer');
    const referrerSelector = optionalControlAttribute(attributes, 'referrer-selector');
    return {
        kind: 'module-url',
        renderNodeId: node.renderNodeId,
        sliceName,
        authoredSpecifier,
        ...(referrer === undefined ? {} : { referrer }),
        ...(referrerSelector === undefined ? {} : { referrerSelector }),
        ...(node.sourceMapRef === undefined ? {} : { sourceMapRef: node.sourceMapRef }),
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

function sameDocumentBytes(left: ArrayBuffer, right: ArrayBuffer): boolean {
    const a = new Uint8Array(left), b = new Uint8Array(right);
    return a.length === b.length && a.every((byte, index) => byte === b[index]);
}
