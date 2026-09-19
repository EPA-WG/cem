/**
 * Host runtime-support boundary for the `cem_ql` WASM render engine
 * (design [`cem-element-wasm-proposal.md` §5/§6](../../../../../../docs/cem-element-wasm-proposal.md)).
 *
 * This module is the Phase 3A internal form of the reusable runtime-support layer.
 * It is authored as if it will be extracted to `@epa-wg/cem-runtime-support`: it
 * knows nothing about `customElements`, declaration discovery, produced-element
 * lifecycle, or that its caller is `<cem-element>`. It only turns a canonical
 * CEM-ML source string plus host metadata and explicit native document bindings into a serializable
 * {@link RenderPlanNode} list, by calling the `cem_ql` WASM render boundary
 * (C2.2 exports) and mapping its JSON plan into the projection-layer shape so the
 * existing `materializeRenderPlan` can commit it unchanged.
 *
 * Topology: the Phase 3A processing engine calls this boundary inside one dedicated
 * pooled module worker selected for each logical root. The same module is instantiated on the main thread
 * only after the processing host selects its deterministic fallback (design §4.3).
 */

// eslint-disable-next-line @nx/enforce-module-boundaries -- generated WASM bindings are the Phase 3A internal runtime boundary.
import initCemQlWasm, {
    cemQlVersion,
    compileTemplate,
    compileTemplateArtifact,
    compileTemplateModuleClosure,
    convertLegacyCustomElementTemplate,
    disposeTemplate,
    importTemplateArtifact,
    renderTemplate,
    renderTemplateWithCemDocuments,
    retainCemDocument,
    disposeCemDocument,
    renderTemplateSource,
    resolveModuleUrl as resolveModuleUrlWasm,
    templateArtifactPayloadKey,
    templateModuleImports,
    retainXsltComponent,
    renderXsltComponent,
    disposeXsltComponent,
    xsltStylesheetImports,
} from '../../../../../cem_ql/dist/wasm/cem_ql.js';
import {
    assertProcessingBoundaryValue,
    diffRenderPlansToPatchFrames,
    projectSlotsInRenderPlan,
    type EdgePatchOptions,
    type PatchFrame,
    type RenderPlan,
    type RenderPlanNode,
    type SourceMapRef,
} from '../../projection.js';
import type { CemBrowserModuleUrlMap } from './module-url-resolution.js';
import type { CemControlInputPolicy } from '../../declaration-scope.js';

export interface RuntimeSupportDiagnostic {
    code: string;
    severity: 'info' | 'warning' | 'error' | 'fatal';
    message: string;
    byteOffset?: number;
    sourceMapRef?: SourceMapRef;
}

export interface CemQlHostAttributeUpdate {
    name: string;
    value: string;
}

export interface CemQlRenderResult {
    nodes: RenderPlanNode[];
    hostAttributeUpdates: CemQlHostAttributeUpdate[];
    diagnostics: RuntimeSupportDiagnostic[];
}

export interface CemQlStylesheetArtifact {
    css: string;
    scope: string | null;
}

export interface CemMlTemplateCompileResult {
    stylesheets: CemQlStylesheetArtifact[];
    moduleMap: CemBrowserModuleUrlMap | null;
    diagnostics: RuntimeSupportDiagnostic[];
}

export interface CemQlRenderOptions {
    /** Prefix for deterministic, pre-order render-node ids (typically the produced tag). */
    renderNodeIdPrefix?: string;
    /** Resolver/loader for static CEMT imports. Omit when the source has no module imports. */
    moduleLoader?: CemMlTemplateModuleLoader;
}

export interface CemMlTemplateModuleLoader {
    /** Absolute URL of the root inline or external template; relative imports resolve from it. */
    rootUrl: string;
    /** Stable identity of the active scope/import-map policy. */
    resolverPolicyStamp: string;
    /** Resolve one authored import through the host's scope-aware module URL service. */
    resolve(
        specifier: string,
        referrerUrl: string,
        referrerModuleMap: CemBrowserModuleUrlMap | null
    ): string | Promise<string>;
    /** Load one resolved module URL. Defaults to an HTTP `fetch()` when omitted. */
    load?(resolvedUrl: string): string | Promise<string>;
}

export interface CemMlTemplateModuleSource {
    alias: string;
    parentUri?: string;
    uri: string;
    contentHash: string;
    source: string;
}

export interface CemXsltComponentOptions {
    entrypoint?: string;
    parameters: Array<{ name: string; select: string }>;
    modules?: Array<{ parentUri: string; href: string; uri: string; source: string; contentHash: string }>;
}

// Shared by engines using this WASM instance, including main-thread fallback.
// Leases survive native cache eviction; their next render recompiles the same
// immutable authoring inputs. A synchronous WASM render cannot be evicted midway.
const xsltResidents = new Map<RetainedXsltComponent, number>();

export class RetainedXsltComponent {
    readonly stylesheets: CemQlStylesheetArtifact[];
    readonly diagnostics: RuntimeSupportDiagnostic[];
    private disposed = false;

    constructor(private readonly source: string, private readonly uri: string,
        private readonly optionsJson: string, private readonly hostBindingsJson: string,
        private readonly controlPolicyJson?: string) {
        const compiled = this.ensureNative();
        this.stylesheets = (compiled.stylesheets ?? []).map(mapStylesheet);
        this.diagnostics = (compiled.diagnostics ?? []).map(mapDiagnostic);
    }

    private ensureNative(): { artifactId: number; stylesheets?: WasmStylesheetArtifact[]; diagnostics?: WasmDiagnostic[] } {
        if (this.disposed) throw new Error('XSLT component lease is disposed');
        const retained = xsltResidents.get(this);
        if (retained !== undefined) {
            xsltResidents.delete(this);
            xsltResidents.set(this, retained);
            return { artifactId: retained };
        }
        const evict = (): boolean => {
            const oldest = xsltResidents.entries().next().value;
            if (!oldest) return false;
            xsltResidents.delete(oldest[0]);
            disposeXsltComponent(oldest[1]);
            return true;
        };
        if (xsltResidents.size >= 16) evict();
        for (;;) {
            try {
                const compiled = JSON.parse(retainXsltComponent(this.source, this.uri, this.optionsJson, this.hostBindingsJson, this.controlPolicyJson)) as {
                    artifactId: number; stylesheets?: WasmStylesheetArtifact[]; diagnostics?: WasmDiagnostic[];
                };
                xsltResidents.set(this, compiled.artifactId);
                return compiled;
            } catch (error) {
                // Native byte capacity may be reached before the handle bound.
                const message = String(error);
                if ((message === 'XSLT component byte limit exceeded' || message === 'XSLT component handle limit exceeded') && evict()) continue;
                throw error;
            }
        }
    }

    async render(data: Record<string, unknown>, options: CemQlRenderOptions & { documents?: { slice: string; documentId: number }[] }): Promise<CemQlRenderResult> {
        assertProcessingBoundaryValue(data, 'XSLT component control data');
        await ensureRuntimeReady();
        const { artifactId } = this.ensureNative();
        return mapWasmRenderPlan(renderXsltComponent(artifactId, JSON.stringify(data), JSON.stringify(options.documents ?? [])), options);
    }

    dispose(): void {
        const id = xsltResidents.get(this);
        if (id !== undefined) disposeXsltComponent(id);
        xsltResidents.delete(this);
        this.disposed = true;
    }
}

export async function retainXsltComponentSource(source: string, uri: string, options: CemXsltComponentOptions,
    hostBindings: readonly string[], controlPolicy?: CemControlInputPolicy): Promise<RetainedXsltComponent> {
    const optionsJson = JSON.stringify(options);
    const hostBindingsJson = JSON.stringify(hostBindings);
    const controlPolicyJson = controlPolicy === undefined ? undefined : JSON.stringify(controlPolicy);
    await ensureRuntimeReady();
    return new RetainedXsltComponent(source, uri, optionsJson, hostBindingsJson, controlPolicyJson);
}

/** Host URL resolution over typed authoring import/include edges. */
export async function preflightXsltModules(source: string, loader: CemMlTemplateModuleLoader): Promise<NonNullable<CemXsltComponentOptions['modules']>> {
    await ensureRuntimeReady();
    const root = absoluteModuleUrl(loader.rootUrl, 'root stylesheet');
    const modules: NonNullable<CemXsltComponentOptions['modules']> = [];
    const loaded = new Map<string, { source: string; contentHash: string }>();
    const expanded = new Set<string>();
    const active = new Set<string>();
    const visit = async (source: string, uri: string, depth: number): Promise<void> => {
        if (active.has(uri)) throw new Error(`cyclic XSLT import: ${uri}`);
        if (expanded.has(uri)) return;
        if (depth > 32 || expanded.size >= 64) throw new Error('XSLT import closure exceeds limits');
        active.add(uri);
        const imports = JSON.parse(xsltStylesheetImports(source, uri)) as string[];
        for (const href of new Set(imports)) {
            if (modules.length >= 128) throw new Error('XSLT import edge limit exceeded');
            const resolved = absoluteModuleUrl(await loader.resolve(href, uri, null), 'imported stylesheet');
            if (active.has(resolved)) throw new Error(`cyclic XSLT import: ${resolved}`);
            let member = loaded.get(resolved);
            if (!member) {
                if (loaded.size >= 63) throw new Error('XSLT source count exceeds limits');
                const text = loader.load ? await loader.load(resolved) : await fetchModuleSource(resolved);
                if (new TextEncoder().encode(text).byteLength > 128 * 1024) throw new Error('XSLT source exceeds byte limit');
                member = { source: text, contentHash: (await cemMlTemplateArtifactPayloadKey(text, 'dev')).sourceHash };
                loaded.set(resolved, member);
            }
            modules.push({ parentUri: uri, href, uri: resolved, ...member });
            await visit(member.source, resolved, depth + 1);
        }
        active.delete(uri);
        expanded.add(uri);
    };
    await visit(source, root, 0);
    return modules;
}

export interface CemMlTemplateModuleClosure {
    rootUri: string;
    rootContentHash: string;
    resolverPolicyStamp: string;
    entrypoint: 'body';
    parameterContract: string[];
    cemMlVersion: string;
    cemQlVersion: string;
    modules: CemMlTemplateModuleSource[];
}

export interface CemMlTemplateProcessingIdentity {
    producedTag: string;
    instanceId: string;
    templateArtifactId: string;
    dataRevision: string;
    renderAttempt?: number;
    outputTarget: 'light-dom';
    scopePolicyStamp: string;
}

export interface CemMlTemplateProcessingInput {
    source: string;
    data: Record<string, unknown>;
    identity: CemMlTemplateProcessingIdentity;
    payload?: unknown;
    previousRenderPlan?: RenderPlan | null;
    patchOptions?: EdgePatchOptions;
    renderNodeIdPrefix?: string;
    moduleClosure?: CemMlTemplateModuleClosure;
}

export interface CemMlTemplateArtifactPayloadKey {
    contentType: 'cem-template-artifact';
    sourceHash: string;
    cemMlVersion: string;
    cemQlVersion: string;
    sourceMapMode: 'dev' | 'prod';
}

export interface RetainedCemMlTemplate {
    artifactId: number;
    stylesheets: CemQlStylesheetArtifact[];
    moduleMap: CemBrowserModuleUrlMap | null;
    diagnostics: RuntimeSupportDiagnostic[];
    contentHash?: string;
    formatVersion?: string;
}

export interface CemMlTemplateProcessingResult {
    renderPlan: RenderPlan;
    hostAttributeUpdates: CemQlHostAttributeUpdate[];
    diagnostics: RuntimeSupportDiagnostic[];
    patchFrames?: PatchFrame[];
    stylesheets?: CemQlStylesheetArtifact[];
}

let initPromise: Promise<void> | undefined;
let ready = false;

/**
 * Lazily instantiate the `cem_ql` WASM module once per host. Safe to call
 * repeatedly; concurrent callers share one initialization promise.
 */
export function ensureRuntimeReady(): Promise<void> {
    if (!initPromise) {
        initPromise = Promise.resolve()
            .then(() => initCemQlWasm())
            .then(() => {
                ready = true;
            });
    }
    return initPromise;
}

/** Synchronous readiness probe so callers can choose a fallback before awaiting. */
export function isRuntimeReady(): boolean {
    return ready;
}

/** The `cem_ql` engine version; only meaningful after {@link ensureRuntimeReady}. */
export function runtimeVersion(): string {
    return cemQlVersion();
}

/**
 * Resolve one immutable module-context request through the Rust-owned resolver
 * compiled into the shared CEM-QL WASM runtime.
 */
export async function resolveCemModuleUrl(request: unknown): Promise<unknown> {
    await ensureRuntimeReady();
    return JSON.parse(resolveModuleUrlWasm(JSON.stringify(request))) as unknown;
}

/**
 * Compile a CEM-ML template through the `cem_ql` WASM boundary to surface
 * **declaration-time** diagnostics, returning only structural (`cem.tokenizer.*`)
 * well-formedness errors. cem-ql expression diagnostics (`cem.ql.*`) are intentionally
 * dropped here: host bindings are unknown at declaration time, so every `{$x}` would
 * otherwise report `unknown_variable`. Those surface at render instead. Awaits WASM init.
 */
export async function compileCemMlTemplate(source: string): Promise<CemMlTemplateCompileResult> {
    await ensureRuntimeReady();
    const result = JSON.parse(compileTemplate(source, '[]')) as {
        artifactId?: number;
        stylesheets?: WasmStylesheetArtifact[];
        moduleMap?: CemBrowserModuleUrlMap | null;
        diagnostics?: WasmDiagnostic[];
    };
    if (Number.isSafeInteger(result.artifactId) && (result.artifactId ?? 0) > 0) {
        disposeTemplate(result.artifactId as number);
    }
    return {
        stylesheets: (result.stylesheets ?? []).map(mapStylesheet),
        moduleMap: result.moduleMap ?? null,
        diagnostics: (result.diagnostics ?? [])
            .filter((diagnostic) => {
                const code = diagnostic.code ?? '';
                return code.startsWith('cem.tokenizer.')
                    || code === 'cem.ql.template.stylesheet_dynamic_unsupported'
                    || code.startsWith('cem.ql.template.module_map_');
            })
            .map(mapDiagnostic),
    };
}

/** Compute the portable registry key without compiling the template. */
export async function cemMlTemplateArtifactPayloadKey(
    source: string,
    sourceMapMode: 'dev' | 'prod'
): Promise<CemMlTemplateArtifactPayloadKey> {
    await ensureRuntimeReady();
    const parsed = JSON.parse(templateArtifactPayloadKey(source, sourceMapMode)) as
        Partial<CemMlTemplateArtifactPayloadKey> & { diagnostics?: WasmDiagnostic[] };
    if (
        parsed.contentType !== 'cem-template-artifact'
        || typeof parsed.sourceHash !== 'string'
        || typeof parsed.cemMlVersion !== 'string'
        || typeof parsed.cemQlVersion !== 'string'
        || (parsed.sourceMapMode !== 'dev' && parsed.sourceMapMode !== 'prod')
    ) {
        throw new Error(parsed.diagnostics?.[0]?.message ?? 'CEM template artifact key creation failed');
    }
    return parsed as CemMlTemplateArtifactPayloadKey;
}

/** Build portable binary template IR for build pipelines and registry write-through. */
export async function compileCemMlTemplateArtifact(
    source: string,
    hostBindings: readonly string[] = [],
    sourceMapMode: 'dev' | 'prod' = 'dev'
): Promise<Uint8Array> {
    await ensureRuntimeReady();
    return compileTemplateArtifact(source, JSON.stringify([...hostBindings]), sourceMapMode);
}

/** Retain a source-compiled template for compile-once/render-many execution. */
export async function retainCemMlTemplateSource(
    source: string,
    hostBindings: readonly string[] = []
): Promise<RetainedCemMlTemplate> {
    await ensureRuntimeReady();
    const result = JSON.parse(compileTemplate(source, JSON.stringify([...hostBindings]))) as {
        artifactId?: number;
        stylesheets?: WasmStylesheetArtifact[];
        moduleMap?: CemBrowserModuleUrlMap | null;
        diagnostics?: WasmDiagnostic[];
    };
    if (!Number.isSafeInteger(result.artifactId) || (result.artifactId ?? 0) < 1) {
        throw new Error(result.diagnostics?.[0]?.message ?? 'CEM template compilation did not retain an artifact');
    }
    return {
        artifactId: result.artifactId as number,
        stylesheets: (result.stylesheets ?? []).map(mapStylesheet),
        moduleMap: result.moduleMap ?? null,
        diagnostics: (result.diagnostics ?? []).map(mapDiagnostic),
    };
}

/**
 * Resolve, load, hash, and inspect the complete static CEMT import graph. I/O and URL policy stay
 * in the host; the returned immutable closure is the same JSON boundary consumed by WASM and SSR.
 */
export async function preflightCemMlTemplateModules(
    source: string,
    loader: CemMlTemplateModuleLoader,
    hostBindings: readonly string[] = []
): Promise<CemMlTemplateModuleClosure> {
    await ensureRuntimeReady();
    const rootUri = absoluteModuleUrl(loader.rootUrl, 'root template');
    const rootIdentity = await cemMlTemplateArtifactPayloadKey(source, 'dev');
    const modules: CemMlTemplateModuleSource[] = [];
    const loaded = new Map<string, { source: string; contentHash: string }>();
    const expanded = new Set<string>();
    const active = new Set<string>();

    const visit = async (moduleSource: string, moduleUri: string, depth: number): Promise<void> => {
        const inspected = JSON.parse(templateModuleImports(moduleSource, moduleUri)) as {
            imports?: Array<{ alias?: string; uri?: string }>;
            maxImportDepth?: number;
            moduleMap?: CemBrowserModuleUrlMap | null;
            diagnostics?: WasmDiagnostic[];
        };
        const fatal = (inspected.diagnostics ?? []).find(
            (diagnostic) => diagnostic.severity === 'error' || diagnostic.severity === 'fatal'
        );
        if (fatal) {
            throw new Error(`${fatal.code ?? 'cem.transform_template.module_invalid'}: ${fatal.message ?? ''}`);
        }
        const maxDepth = inspected.maxImportDepth ?? 32;
        if (depth > maxDepth) {
            throw new Error(
                `cem.transform_template.import_depth: module import depth ${depth} exceeds ${maxDepth} at ${moduleUri}`
            );
        }
        for (const imported of inspected.imports ?? []) {
            const alias = imported.alias?.trim() ?? '';
            const specifier = imported.uri?.trim() ?? '';
            if (!alias || !specifier) {
                throw new Error(`cem.transform_template.import_invalid: ${moduleUri} has an invalid static import`);
            }
            const resolvedUrl = absoluteModuleUrl(
                await loader.resolve(specifier, moduleUri, inspected.moduleMap ?? null),
                `resolved module ${specifier}`
            );
            if (active.has(resolvedUrl)) {
                throw new Error(
                    `cem.transform_template.import_cycle: ${moduleUri} imports active module ${resolvedUrl}`
                );
            }
            let loadedModule = loaded.get(resolvedUrl);
            if (!loadedModule) {
                const importedSource = loader.load
                    ? await loader.load(resolvedUrl)
                    : await fetchModuleSource(resolvedUrl);
                const contentHash = (await cemMlTemplateArtifactPayloadKey(importedSource, 'dev')).sourceHash;
                loadedModule = { source: importedSource, contentHash };
                loaded.set(resolvedUrl, loadedModule);
            }
            modules.push({
                alias,
                ...(moduleUri === rootUri ? {} : { parentUri: moduleUri }),
                uri: resolvedUrl,
                contentHash: loadedModule.contentHash,
                source: loadedModule.source,
            });
            if (!expanded.has(resolvedUrl)) {
                active.add(resolvedUrl);
                await visit(loadedModule.source, resolvedUrl, depth + 1);
                active.delete(resolvedUrl);
                expanded.add(resolvedUrl);
            }
        }
    };

    active.add(rootUri);
    await visit(source, rootUri, 0);
    active.delete(rootUri);
    return {
        rootUri,
        rootContentHash: rootIdentity.sourceHash,
        resolverPolicyStamp: loader.resolverPolicyStamp,
        entrypoint: 'body',
        parameterContract: [...new Set(hostBindings)].sort(),
        cemMlVersion: rootIdentity.cemMlVersion,
        cemQlVersion: rootIdentity.cemQlVersion,
        modules,
    };
}

/** Compile and retain a resolver-preflighted CEMT module closure. */
export async function retainCemMlTemplateModuleClosure(
    source: string,
    closure: CemMlTemplateModuleClosure,
    hostBindings: readonly string[] = []
): Promise<RetainedCemMlTemplate> {
    await ensureRuntimeReady();
    const result = JSON.parse(
        compileTemplateModuleClosure(source, JSON.stringify(closure), JSON.stringify([...hostBindings]))
    ) as {
        artifactId?: number;
        stylesheets?: WasmStylesheetArtifact[];
        moduleMap?: CemBrowserModuleUrlMap | null;
        diagnostics?: WasmDiagnostic[];
    };
    if (!Number.isSafeInteger(result.artifactId) || (result.artifactId ?? 0) < 1) {
        throw new Error(result.diagnostics?.[0]?.message ?? 'CEM template module closure did not retain an artifact');
    }
    return {
        artifactId: result.artifactId as number,
        stylesheets: (result.stylesheets ?? []).map(mapStylesheet),
        moduleMap: result.moduleMap ?? null,
        diagnostics: (result.diagnostics ?? []).map(mapDiagnostic),
    };
}

/** Validate binary identity against the active source/context and retain it. */
export async function retainCemMlTemplateArtifact(
    bytes: ArrayBuffer,
    expectedContentHash: string,
    source: string,
    hostBindings: readonly string[] = [],
    sourceMapMode: 'dev' | 'prod' = 'dev'
): Promise<RetainedCemMlTemplate> {
    await ensureRuntimeReady();
    const result = JSON.parse(importTemplateArtifact(
        new Uint8Array(bytes),
        expectedContentHash,
        source,
        JSON.stringify([...hostBindings]),
        sourceMapMode
    )) as {
        artifactId?: number | null;
        contentHash?: string;
        formatVersion?: string;
        stylesheets?: WasmStylesheetArtifact[];
        moduleMap?: CemBrowserModuleUrlMap | null;
        diagnostics?: WasmDiagnostic[];
    };
    if (!Number.isSafeInteger(result.artifactId) || (result.artifactId ?? 0) < 1) {
        const diagnostic = result.diagnostics?.[0];
        throw new Error(`${diagnostic?.code ?? 'cem.ql.template_artifact_unsupported'}: ${
            diagnostic?.message ?? 'CEM template artifact import failed'
        }`);
    }
    return {
        artifactId: result.artifactId as number,
        stylesheets: (result.stylesheets ?? []).map(mapStylesheet),
        moduleMap: result.moduleMap ?? null,
        contentHash: result.contentHash,
        formatVersion: result.formatVersion,
        diagnostics: (result.diagnostics ?? []).map(mapDiagnostic),
    };
}

export function disposeRetainedCemMlTemplate(artifactId: number): boolean {
    return disposeTemplate(artifactId);
}

export interface LegacyConvertResult {
    /** Canonical CEM-ML source text for the cem_ql render boundary. */
    source: string;
    diagnostics: RuntimeSupportDiagnostic[];
}

/**
 * Lower a legacy `<custom-element>` HTML+XSLT template to canonical CEM-ML through the CEM-owned
 * engine (`cem_ml::legacy_custom_element`, exposed on the cem_ql WASM module). This is the single
 * legacy compiler shared by the browser runtime, CLI, and tests. Awaits WASM init on first use.
 */
export async function convertLegacyTemplate(source: string): Promise<LegacyConvertResult> {
    await ensureRuntimeReady();
    const result = JSON.parse(convertLegacyCustomElementTemplate(source)) as {
        source?: string;
        diagnostics?: Array<{ code?: string; message?: string }>;
    };
    return {
        source: result.source ?? '',
        // Engine conversion diagnostics carry no severity; they are advisory (unsupported
        // function / Tier-3 construct) — surface them as warnings.
        diagnostics: (result.diagnostics ?? []).map((diagnostic) => ({
            code: diagnostic.code ?? 'legacy_xslt.unknown',
            severity: 'warning' as const,
            message: diagnostic.message ?? '',
        })),
    };
}

/**
 * Render a canonical CEM-ML template against host/data bindings through the
 * `cem_ql` WASM render boundary, returning a serializable render plan plus
 * diagnostics. Awaits WASM initialization on first use.
 */
export async function renderCemMlTemplate(
    source: string,
    data: Record<string, unknown>,
    options: CemQlRenderOptions = {}
): Promise<CemQlRenderResult> {
    assertProcessingBoundaryValue(data, 'CEM-ML render data');
    await ensureRuntimeReady();
    if (options.moduleLoader) {
        const hostBindings = Object.keys(data ?? {});
        const closure = await preflightCemMlTemplateModules(source, options.moduleLoader, hostBindings);
        const retained = await retainCemMlTemplateModuleClosure(source, closure, hostBindings);
        try {
            const rendered = mapWasmRenderPlan(
                renderTemplate(retained.artifactId, JSON.stringify(data ?? {})),
                options
            );
            return {
                ...rendered,
                diagnostics: [...retained.diagnostics, ...rendered.diagnostics],
            };
        } finally {
            disposeTemplate(retained.artifactId);
        }
    }
    const planJson = renderTemplateSource(source, JSON.stringify(data ?? {}));
    return mapWasmRenderPlan(planJson, options);
}

function absoluteModuleUrl(value: string, label: string): string {
    try {
        return new URL(value).href;
    } catch (error) {
        throw new TypeError(`${label} URL \`${value}\` is not absolute`, { cause: error });
    }
}

async function fetchModuleSource(resolvedUrl: string): Promise<string> {
    const response = await fetch(resolvedUrl);
    if (!response.ok) {
        throw new Error(`template module fetch failed (${response.status}) for ${resolvedUrl}`);
    }
    return response.text();
}

/** Render already-compiled template IR without parsing source again. */
export async function renderRetainedCemMlTemplate(
    artifactId: number,
    data: Record<string, unknown>,
    // A host-local capability belongs only to the explicit retained render path.
    options: CemQlRenderOptions & { xpathCompanionId?: number; documents?: { slice: string; documentId: number }[] } = {}
): Promise<CemQlRenderResult> {
    assertProcessingBoundaryValue(data, 'CEM-ML render data');
    await ensureRuntimeReady();
    return mapWasmRenderPlan(options.xpathCompanionId === undefined && !options.documents?.length
        ? renderTemplate(artifactId, JSON.stringify(data ?? {}))
        : renderTemplateWithCemDocuments(artifactId, options.xpathCompanionId ?? 0,
            JSON.stringify(data ?? {}), JSON.stringify(options.documents ?? [])), options);
}

function mapWasmRenderPlan(planJson: string, options: CemQlRenderOptions): CemQlRenderResult {
    const plan = JSON.parse(planJson) as WasmRenderPlan;

    const prefix = options.renderNodeIdPrefix ?? 'cem-node';
    let sequence = 0;
    const nextRenderNodeId = (): string => {
        sequence += 1;
        return `${prefix}-${sequence}`;
    };

    return {
        nodes: (plan.nodes ?? []).map((node, index) => mapNode(node, nextRenderNodeId, `${prefix}:root:${index}`)),
        hostAttributeUpdates: (plan.hostAttributeUpdates ?? []).map((update) => ({
            name: update.name,
            value: update.value,
        })),
        diagnostics: (plan.diagnostics ?? []).map(mapDiagnostic),
    };
}

/**
 * First explicit template-processing path: compile/render CEM-ML through WASM,
 * return a serializable light-DOM render plan, and optionally produce patch frames.
 * DOM materialization/application remains the caller's main-thread UI-adapter work.
 */
export async function processCemMlTemplate(
    input: CemMlTemplateProcessingInput
): Promise<CemMlTemplateProcessingResult> {
    if (input.moduleClosure) {
        const retained = await retainCemMlTemplateModuleClosure(input.source, input.moduleClosure, Object.keys(input.data));
        try {
            const result = await processRetainedCemMlTemplate(retained.artifactId, input);
            return { ...result, diagnostics: [...retained.diagnostics, ...result.diagnostics], stylesheets: retained.stylesheets };
        } finally {
            disposeRetainedCemMlTemplate(retained.artifactId);
        }
    }
    assertProcessingBoundaryValue(input.data, 'CEM-ML processing data');
    assertProcessingBoundaryValue(input.identity, 'CEM-ML processing identity');
    if (input.payload !== undefined) {
        assertProcessingBoundaryValue(input.payload, 'CEM-ML processing payload');
    }
    if (input.previousRenderPlan !== undefined && input.previousRenderPlan !== null) {
        assertProcessingBoundaryValue(input.previousRenderPlan, 'previous render plan');
    }

    const declaration = await compileCemMlTemplate(input.source);
    const rendered = await renderCemMlTemplate(input.source, input.data, {
        renderNodeIdPrefix: input.renderNodeIdPrefix ?? input.identity.producedTag,
    });
    const renderPlan = projectSlotsInRenderPlan(
        {
            ...input.identity,
            nodes: rendered.nodes,
        },
        input.payload
    );
    assertProcessingBoundaryValue(renderPlan, 'CEM-ML render plan');

    return {
        renderPlan,
        hostAttributeUpdates: rendered.hostAttributeUpdates,
        diagnostics: [...declaration.diagnostics, ...rendered.diagnostics],
        patchFrames:
            input.previousRenderPlan === undefined
                ? undefined
                : diffRenderPlansToPatchFrames(input.previousRenderPlan, renderPlan, input.patchOptions),
    };
}

/** Processing path for a validated, retained component-template artifact. */
export async function processRetainedCemMlTemplate(
    artifactId: number,
    input: CemMlTemplateProcessingInput & { xslt?: RetainedXsltComponent; xpathCompanionId?: number; documents?: { slice: string; documentId: number }[] }
): Promise<CemMlTemplateProcessingResult> {
    assertProcessingBoundaryValue(input.data, 'CEM-ML processing data');
    assertProcessingBoundaryValue(input.identity, 'CEM-ML processing identity');
    if (input.payload !== undefined) {
        assertProcessingBoundaryValue(input.payload, 'CEM-ML processing payload');
    }
    if (input.previousRenderPlan !== undefined && input.previousRenderPlan !== null) {
        assertProcessingBoundaryValue(input.previousRenderPlan, 'previous render plan');
    }

    const renderOptions = {
        xpathCompanionId: input.xpathCompanionId,
        documents: input.documents,
        renderNodeIdPrefix: input.renderNodeIdPrefix ?? input.identity.producedTag,
    };
    const rendered = input.xslt ? await input.xslt.render(input.data, renderOptions)
        : await renderRetainedCemMlTemplate(artifactId, input.data, renderOptions);
    const renderPlan = projectSlotsInRenderPlan(
        {
            ...input.identity,
            nodes: rendered.nodes,
        },
        input.payload
    );
    assertProcessingBoundaryValue(renderPlan, 'CEM-ML render plan');
    return {
        renderPlan,
        hostAttributeUpdates: rendered.hostAttributeUpdates,
        diagnostics: rendered.diagnostics,
        patchFrames:
            input.previousRenderPlan === undefined
                ? undefined
                : diffRenderPlansToPatchFrames(input.previousRenderPlan, renderPlan, input.patchOptions),
    };
}

interface WasmRenderPlan {
    nodes?: WasmRenderNode[];
    hostAttributeUpdates?: CemQlHostAttributeUpdate[];
    diagnostics?: WasmDiagnostic[];
}

interface WasmStylesheetArtifact {
    css?: string;
    scope?: string | null;
}

type WasmRenderNode =
    | { kind: 'text'; text: string; byteOffset?: number | null }
    | { kind: 'comment'; text: string; byteOffset?: number | null }
    | {
          kind: 'element';
          tag: string;
          namespace?: string | null;
          attributes?: WasmRenderAttribute[];
          children?: WasmRenderNode[];
          byteOffset?: number | null;
      };

interface WasmRenderAttribute {
    name: string;
    value: string;
}

interface WasmDiagnostic {
    code?: string;
    severity?: string;
    message?: string;
    byteOffset?: number | null;
}

function mapStylesheet(stylesheet: WasmStylesheetArtifact): CemQlStylesheetArtifact {
    return {
        css: stylesheet.css ?? '',
        scope: typeof stylesheet.scope === 'string' ? stylesheet.scope : null,
    };
}

function mapNode(node: WasmRenderNode, nextRenderNodeId: () => string, occurrence: string): RenderPlanNode {
    if (node.kind === 'text') {
        return { kind: 'text', text: node.text, renderNodeId: `text:${occurrence}`, sourceMapRef: frameFrom(node.byteOffset) };
    }
    if (node.kind === 'comment') {
        return { kind: 'comment', text: node.text, renderNodeId: `comment:${occurrence}`, sourceMapRef: frameFrom(node.byteOffset) };
    }
    // Assign the render-node id before recursing so ids follow a deterministic
    // pre-order sequence, matching the DOM/projection path.
    const renderNodeId = nextRenderNodeId();
    return {
        kind: 'element',
        namespace: node.namespace ?? null,
        tag: node.tag,
        attributes: (node.attributes ?? []).map((attribute) => ({
            name: attribute.name,
            value: attribute.value,
        })),
        renderNodeId,
        children: (node.children ?? []).map((child, index) => mapNode(child, nextRenderNodeId, `${renderNodeId}:child:${index}`)),
        sourceMapRef: frameFrom(node.byteOffset),
    };
}

function frameFrom(byteOffset: number | null | undefined): SourceMapRef | undefined {
    if (typeof byteOffset !== 'number') {
        return undefined;
    }
    return { fidelity: 'author-byte-exact', frame: `cem:${byteOffset}` };
}

function mapDiagnostic(diagnostic: WasmDiagnostic): RuntimeSupportDiagnostic {
    const byteOffset = typeof diagnostic.byteOffset === 'number' ? diagnostic.byteOffset : undefined;
    return {
        code: diagnostic.code ?? 'cem.ql.wasm.diagnostic',
        severity: coerceSeverity(diagnostic.severity),
        message: diagnostic.message ?? 'cem_ql render diagnostic',
        byteOffset,
        sourceMapRef: frameFrom(byteOffset),
    };
}

function coerceSeverity(severity: string | undefined): RuntimeSupportDiagnostic['severity'] {
    switch (severity) {
        case 'fatal':
        case 'error':
        case 'warning':
        case 'info':
            return severity;
        default:
            return 'error';
    }
}

/** Byte ingress into cem-ml. The response document never enters JavaScript. */
export async function retainLoadedCemDocument(bytes: ArrayBuffer, contentType: string, uri: string): Promise<number> {
    await ensureRuntimeReady();
    return retainCemDocument(new Uint8Array(bytes), contentType, uri);
}
export function disposeLoadedCemDocument(id: number): void {
    disposeCemDocument(id);
}
