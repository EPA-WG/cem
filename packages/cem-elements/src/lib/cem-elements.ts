import {
    DATA_CEM_SCOPE_ATTR,
    applyRenderPlanToRange,
    edgeContentAddress,
    materializeRenderPlan,
    projectTemplate,
    readTemplateSource,
    renderPlansHaveDomChanges,
    scopeRenderPlan,
    type RenderPlan,
    type RenderPlanApplyDiagnostic,
    type ScopedCssRewriteDiagnostic,
    type SourceMapFidelity,
    type SourceMapRef,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
import {
    compileCemMlTemplate,
    convertLegacyTemplate,
    ensureRuntimeReady,
    processCemMlTemplate,
    type RuntimeSupportDiagnostic,
} from './internal/runtime-support/cem-ql-render.js';
import { ingestContractVersion, type RunMode } from './disposition.js';
import { LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG } from './legacy-xslt/contract.js';

export type CemElementDiagnosticSeverity = 'info' | 'warning' | 'error' | 'fatal';

export interface CemElementDiagnostic {
    code: string;
    severity: CemElementDiagnosticSeverity;
    message: string;
    source: 'declaration' | 'instance' | 'render';
    tag?: string;
    sourceMapRef?: SourceMapRef;
}

export interface DeclarationShapeInput {
    tag: string | null;
    src: string | null;
    directTemplateCount: number;
    directLiveNodeCount: number;
}

export interface DeclarationShapeResult {
    ok: boolean;
    tag: string | null;
    src: string | null;
    diagnostics: CemElementDiagnostic[];
}

export interface SerializedPayload {
    text: string;
    childCount: number;
    nodes: SerializedPayloadNode[];
    slots: Record<string, SerializedPayloadNode[]>;
    elementsByAttribute: Record<string, SerializedPayloadElement[]>;
    data: SerializedPayloadChoice[];
    options: SerializedPayloadChoice[];
    dataByValue: Record<string, SerializedPayloadChoice>;
    optionsByValue: Record<string, SerializedPayloadChoice>;
}

export interface SerializedPayloadElement {
    key: string;
    tag: string;
    namespace: string | null;
    text: string;
    attributes: Record<string, string>;
    slot: string;
}

export interface SerializedPayloadChoice {
    kind: 'data' | 'option';
    key: string;
    value: string;
    label: string;
    text: string;
    attributes: Record<string, string>;
    group: string | null;
}

export type SerializedPayloadNode =
    | { kind: 'text'; key: string; text: string }
    | { kind: 'comment'; key: string; text: string }
    | {
          kind: 'element';
          key: string;
          tag: string;
          namespace: string | null;
          attributes: Record<string, string>;
          slot: string;
          children: SerializedPayloadNode[];
      };

export interface SerializedEventTarget {
    tag: string;
    id: string | null;
    name: string | null;
    type: string | null;
    value: string | null;
    checked: boolean | null;
    dataset: Record<string, string>;
}

export interface SerializedEventPayload {
    type: string;
    bubbles: boolean;
    cancelable: boolean;
    composed: boolean;
    target: SerializedEventTarget | null;
    currentTarget: SerializedEventTarget | null;
    sliceValue: TemplateValue;
    detail?: unknown;
}

/** Schema version of the DataIslandSnapshot / datadom governed contract (FF-6 SemVer axis, BR-VC-5). */
export const SNAPSHOT_SCHEMA_VERSION = '1.1.0';

export type SourceMapMode = 'dev' | 'prod';

export interface DataIslandSnapshot {
    /** Snapshot schema version; see {@link SNAPSHOT_SCHEMA_VERSION}. Optional during the expand phase (BR-EV-5). */
    version?: string;
    instanceId: string;
    producedTag: string;
    declarationTag: string;
    templateArtifactId: string;
    dataRevision: string;
    outputTarget: 'light-dom';
    /** Optional during the expand phase: older SSR snapshots predate source-map-mode hydration checks. */
    sourceMapMode?: SourceMapMode;
    scopePolicyStamp: string;
    privacyPolicyStamp: string;
    hostAttributes: Record<string, string | boolean | null>;
    dataset: Record<string, string>;
    payload: SerializedPayload;
    slices: Record<string, unknown>;
    validationState: Record<string, unknown>;
    eventPayloads: Record<string, unknown>;
}

export type DataIslandSnapshotExportField =
    | 'hostAttributes'
    | 'dataset'
    | 'payload'
    | 'slices'
    | 'validationState'
    | 'eventPayloads';

export type DataIslandSnapshotExportDecision = 'allow' | 'omit' | 'redact';

export type ExportedDataIslandSnapshot = Pick<
    DataIslandSnapshot,
    | 'version'
    | 'instanceId'
    | 'producedTag'
    | 'declarationTag'
    | 'templateArtifactId'
    | 'dataRevision'
    | 'outputTarget'
    | 'sourceMapMode'
    | 'scopePolicyStamp'
    | 'privacyPolicyStamp'
> &
    Partial<Pick<DataIslandSnapshot, DataIslandSnapshotExportField>>;

export interface DataIslandSnapshotExportPolicy {
    fields?: Partial<Record<DataIslandSnapshotExportField, DataIslandSnapshotExportDecision>>;
    privacyPolicyStamp?: string;
}

export interface CemResourceResolutionRequest {
    kind: 'http-request';
    authoredUrl: string;
    baseUrl: string;
    declarationScopeId: string;
    method: string;
    headers: Record<string, string>;
    expectedContentType?: string;
}

export interface CemResourceResolution {
    authoredUrl: string;
    resolvedUrl: string;
    resolverIdentity: string;
    resourcePolicyStamp: string;
    contentTypeHint?: string;
    integrity?: string;
}

export type CemResourceResolutionResult = string | CemResourceResolution;

export interface CemHttpRequest {
    authoredUrl: string;
    baseUrl: string;
    resolvedUrl: string;
    resolverIdentity: string;
    resourcePolicyStamp: string;
    method: 'GET' | 'HEAD';
    headers: Record<string, string>;
    credentials?: string;
    cache?: string;
    expectedContentType?: string;
    policy: CemHttpResourcePolicy;
    signal: AbortSignal;
}

export interface CemHttpResponseHead {
    url: string;
    status: number;
    statusText: string;
    ok: boolean;
    redirected: boolean;
    headers: Record<string, string>;
    contentType: string | null;
}

export interface CemHttpResourceSourceId {
    kind: 'http-response';
    id: string;
    authoredUrl: string;
    resolvedUrl: string;
    finalUrl: string;
    resolverIdentity: string;
    resourcePolicyStamp: string;
    method: string;
    contentType: string | null;
    responseIdentityHash?: string;
    redacted: boolean;
}

export interface CemHttpResourceLoadResult {
    response: CemHttpResponseHead;
    body: AsyncIterable<Uint8Array>;
}

export type CemHttpResourceState = 'pending' | 'headers' | 'complete' | 'error' | 'aborted';

export interface CemHttpResourcePolicy {
    allowCrossOrigin: boolean;
    maxResponseBytes: number;
    timeoutMs: number;
    redirect: RequestRedirect;
}

export interface CemHttpResourceEnvelope {
    kind: 'http-request';
    state: CemHttpResourceState;
    resourceRevision: number;
    request: {
        authoredUrl: string;
        url: string;
        resolvedUrl: string;
        resolverIdentity: string;
        resourcePolicyStamp: string;
        method: string;
        headers: Record<string, string>;
    };
    response?: CemHttpResponseHead;
    sourceId?: CemHttpResourceSourceId;
    data: unknown;
    diagnostics: CemElementDiagnostic[];
}

export interface CemElementRuntimeOptions {
    declarationTag?: string;
    scopePolicyStamp?: string;
    privacyPolicyStamp?: string;
    logger?: Pick<Console, 'warn' | 'error'>;
    /**
     * Load the HTML document an external `src` declaration references, given the `src`
     * path (the part before `#`) and the declaring document. Lets a host control module-map
     * resolution, fetching, and scope-URL policy (and makes external `src` testable). The
     * default resolves the path against the declaring document's base URL and `fetch`es it.
     */
    loadSrcDocument?: (specifier: string, baseDocument: Document) => Promise<string>;
    /**
     * Resolve a `module-url` resource slice specifier to the URL exposed under
     * `datadom.slices.<slice>`. Relative/absolute URLs resolve by default; bare
     * package/module specifiers should be supplied by the host module-map resolver.
     */
    resolveModuleUrl?: (specifier: string, baseDocument: Document) => string | Promise<string>;
    /**
     * Resolve an `http-request @url` resource specifier in the declaration scope before
     * the request loader opens it. Hosts use this for module/import-map aliases, fixture
     * resources, CDN manifests, and policy stamps.
     */
    resolveResourceUrl?: (
        request: CemResourceResolutionRequest,
        baseDocument: Document
    ) => CemResourceResolutionResult | Promise<CemResourceResolutionResult>;
    /**
     * Open an authorized HTTP resource. The stream-shaped body is required at the host
     * boundary even while Phase 1 materializes JSON responses before rerendering.
     */
    loadHttpResource?: (request: CemHttpRequest) => Promise<CemHttpResourceLoadResult>;
    /**
     * Conservative browser defaults for direct HTTP resource loading. Hosts can
     * override these when they provide their own resolver/loader policy.
     */
    httpResourcePolicy?: Partial<CemHttpResourcePolicy>;
    /**
     * Effective run mode for the BR-VC-9 unknown-optional-feature disposition
     * applied when ingesting a versioned governed-contract payload (e.g. a
     * server-rendered hydration snapshot whose schema MINOR is ahead of this
     * build). Defaults to `application` — the correct conservative default for a
     * client runtime; build/SSR pipelines pass `build-ssr`, dev tooling
     * `development`. See {@link ingestContractVersion}.
     */
    runMode?: RunMode;
    /**
     * Host/build-provided deterministic seed used when a declaration does not
     * carry `uid-seed`. A resolver can scope seeds by source URI, fragment,
     * produced tag, or other host-owned public identity.
     */
    uidSeed?: string | ((input: CemElementUidSeedInput) => string | null | undefined);
    /**
     * Seed fallback after explicit declaration and host seeds. Defaults to
     * `source-hash` in build/SSR mode and `runtime` in normal browser runtime.
     */
    uidSeedFallback?: 'runtime' | 'source-hash';
    /**
     * Debug/validation switch for generated public IDs. Normal ephemeral browser
     * runtime can leave this off and rely on host-provided `uid-seed` uniqueness.
     */
    validateGeneratedIds?: boolean;
}

export interface CemElementUidSeedInput {
    declarationElement: HTMLElement;
    declarationTag: string;
    producedTag: string;
    template: HTMLTemplateElement;
    mode: 'dom' | 'cem-ml' | 'legacy-xslt';
    occurrencePath: string;
    sourceText: string;
    sourceHash: string;
}

type CemElementWindow = Window &
    typeof globalThis & {
        HTMLElement: typeof HTMLElement;
        customElements: CustomElementRegistry;
    };

interface AttributeDeclaration {
    name: string;
    defaultValue: TemplateValue;
}

interface SliceDeclaration {
    name: string;
    defaultValue: TemplateValue;
}

interface CompiledDeclaration {
    declarationElement: HTMLElement;
    declarationTag: string;
    producedTag: string;
    uidSeed: string | null;
    uidSeedSource: 'declaration' | 'host' | 'source-hash' | 'runtime';
    occurrencePath: string;
    sourceHash: string;
    scopeUid: string;
    artifactId: string;
    template: HTMLTemplateElement;
    templateSource: TemplateSourceNode[];
    mode: 'dom' | 'cem-ml' | 'legacy-xslt';
    /**
     * Raw canonical CEM-ML source text for the `cem_ql` WASM render boundary. For legacy-xslt this
     * starts null and is filled by the async engine conversion of {@link legacySource} on first render.
     */
    cemMlSource: string | null;
    /** Raw legacy HTML+XSLT markup, lowered to {@link cemMlSource} by the engine on first render. */
    legacySource: string | null;
    /** Whether this declaration renders through the canonical CEM-ML WASM boundary. */
    wasmEligible: boolean;
    declaredAttributes: AttributeDeclaration[];
    declaredSlices: SliceDeclaration[];
    diagnostics: CemElementDiagnostic[];
}

interface RenderBounds {
    start: Comment;
    end: Comment;
}

interface InstanceState {
    slices: Record<string, unknown>;
    eventPayloads: Record<string, unknown>;
    httpResources: Record<string, ActiveHttpResource>;
    localStorageResources: Record<string, ActiveLocalStorageResource>;
    resourceRevisions: Record<string, number>;
    observer?: MutationObserver;
}

interface SliceEventBinding {
    instance: HTMLElement;
    sliceName: string;
    eventName: string;
    expression: string;
    listener: EventListener;
}

interface HttpRequestDeclaration {
    sliceName: string;
    authoredUrl: string;
    method: string;
    headers: Record<string, string>;
    expectedContentType?: string;
    credentials?: string;
    cache?: string;
}

interface LocalStorageDeclaration {
    sliceName: string;
    key: string;
    storageType: string;
    live: boolean;
    initialValue?: string;
}

interface ActiveHttpResource {
    key: string;
    revision: number;
    controller: AbortController;
    settled: Promise<void>;
}

interface ActiveLocalStorageResource {
    key: string;
    storageType: string;
    live: boolean;
    lastValue: unknown;
    lastRawValue: string | null;
    destroy?: () => void;
}

type RenderedResourceResult =
    { kind: 'module-url'; sliceName: string; specifier: string; value: string; error?: unknown };

const DEFAULT_DECLARATION_TAG = 'cem-element';
const DEFAULT_SCOPE_POLICY_STAMP = 'phase-3a-local-default';
const DEFAULT_PRIVACY_POLICY_STAMP = 'local-only';
const DEFAULT_HTTP_RESOURCE_POLICY: CemHttpResourcePolicy = {
    allowCrossOrigin: false,
    maxResponseBytes: 1_048_576,
    timeoutMs: 15_000,
    redirect: 'error',
};
const DATA_ISLAND_ATTR = 'data-cem-island';
const DATA_ISLAND_VALUE = 'instance';
const HYDRATION_METADATA_ATTR = 'data-cem-hydration';
const HYDRATION_METADATA_VALUE = 'snapshot';
const UID_SEED_ATTR = 'uid-seed';
const LOCAL_STORAGE_EVENT = 'cem-local-storage';
const RENDER_TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const RENDER_DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
const DATA_ISLAND_EXPORT_FIELDS: readonly DataIslandSnapshotExportField[] = [
    'hostAttributes',
    'dataset',
    'payload',
    'slices',
    'validationState',
    'eventPayloads',
];
const RESERVED_CUSTOM_ELEMENT_NAMES = new Set([
    'annotation-xml',
    'color-profile',
    'font-face',
    'font-face-src',
    'font-face-uri',
    'font-face-format',
    'font-face-name',
    'missing-glyph',
]);

let artifactSequence = 0;
let runtimeUidSeedSequence = 0;
const localStorageTrackers = new WeakSet<Window>();

export interface ScopeUidInput {
    producedTag: string | null;
    uidSeed: string | null;
    occurrencePath: string;
    runtimeSeed?: string;
}

export function cemElements(): string {
    return '@epa-wg/cem-elements';
}

export function isValidCustomElementName(tag: string): boolean {
    return /^[a-z][.0-9_a-z-]*-[.0-9_a-z-]*$/.test(tag) && !RESERVED_CUSTOM_ELEMENT_NAMES.has(tag);
}

export function generateScopeUid(input: ScopeUidInput): string {
    const tagPrefix = uidTagPrefix(input.producedTag);
    const seed = input.uidSeed !== null ? input.uidSeed : input.runtimeSeed ?? nextRuntimeUidSeed();
    const seedPart = seed.length > 0 ? `-u${encodeUidComponent(seed)}` : '';
    const pathPart = `-p${encodeUidComponent(input.occurrencePath) || '0'}`;
    return `cem-scope${tagPrefix}${seedPart}${pathPart}`;
}

function nextRuntimeUidSeed(): string {
    runtimeUidSeedSequence += 1;
    return `runtime-${runtimeUidSeedSequence}`;
}

function uidTagPrefix(tag: string | null): string {
    if (!tag) {
        return '';
    }
    const prefix = tag.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    return prefix.length > 0 ? `-${prefix}` : '';
}

function encodeUidComponent(value: string): string {
    return encodeURIComponent(value)
        .replace(/%/g, 'z')
        .replace(/[^A-Za-z0-9]+/g, '-')
        .replace(/^-+|-+$/g, '')
        .toLowerCase();
}

export function analyzeDeclarationShape(input: DeclarationShapeInput): DeclarationShapeResult {
    const diagnostics: CemElementDiagnostic[] = [];
    const tag = input.tag?.trim() || null;
    const src = input.src?.trim() || null;

    if (!tag) {
        diagnostics.push(declarationDiagnostic('cem-element.tag_missing', 'declaration requires a `tag` attribute'));
    } else if (!isValidCustomElementName(tag)) {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.tag_invalid',
                `declaration tag \`${tag}\` is not a valid custom-element name`,
                tag
            )
        );
    }

    if (src && input.directTemplateCount > 0) {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.src_inline_template_conflict',
                '`src` declarations must not also include an inline declaration template',
                tag ?? undefined
            )
        );
    }

    if (!src && input.directTemplateCount > 1) {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.inline_template_count',
                'inline declarations must contain at most one direct-child `<template>`',
                tag ?? undefined
            )
        );
    }

    if (input.directLiveNodeCount > 0 && (src || input.directTemplateCount > 0)) {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.declaration_live_content',
                'declaration content outside the associated `<template>` would be live page content',
                tag ?? undefined
            )
        );
    }

    return {
        ok: !diagnostics.some((diagnostic) => diagnostic.severity === 'error' || diagnostic.severity === 'fatal'),
        tag,
        src,
        diagnostics,
    };
}

export function installCemElementRuntime(
    host: CemElementWindow = globalThis as CemElementWindow,
    options: CemElementRuntimeOptions = {}
): CemElementRuntime {
    const runtime = new CemElementRuntime(options);
    runtime.install(host);
    return runtime;
}

export function exportDataIslandSnapshotForEdge(
    snapshot: DataIslandSnapshot,
    policy: DataIslandSnapshotExportPolicy = {}
): ExportedDataIslandSnapshot {
    const exported: ExportedDataIslandSnapshot = {
        instanceId: snapshot.instanceId,
        producedTag: snapshot.producedTag,
        declarationTag: snapshot.declarationTag,
        templateArtifactId: snapshot.templateArtifactId,
        dataRevision: snapshot.dataRevision,
        outputTarget: snapshot.outputTarget,
        scopePolicyStamp: snapshot.scopePolicyStamp,
        privacyPolicyStamp: policy.privacyPolicyStamp ?? snapshot.privacyPolicyStamp,
    };
    if (snapshot.version !== undefined) exported.version = snapshot.version;
    if (snapshot.sourceMapMode !== undefined) exported.sourceMapMode = snapshot.sourceMapMode;
    for (const field of DATA_ISLAND_EXPORT_FIELDS) {
        const decision = policy.fields?.[field] ?? 'omit';
        if (decision === 'allow') {
            exported[field] = cloneJsonSnapshotField(snapshot[field]) as never;
        } else if (decision === 'redact') {
            exported[field] = redactedSnapshotField(field) as never;
        }
    }
    return exported;
}

export class CemElementRuntime {
    readonly declarationTag: string;
    readonly scopePolicyStamp: string;
    readonly privacyPolicyStamp: string;

    private readonly logger?: Pick<Console, 'warn' | 'error'>;
    private readonly declarations = new Map<string, CompiledDeclaration>();
    private readonly diagnostics = new WeakMap<object, CemElementDiagnostic[]>();
    private readonly initializedInstances = new WeakSet<HTMLElement>();
    private readonly registeredDeclarationElements = new WeakSet<object>();
    private readonly hydratedServerRenders = new WeakSet<HTMLElement>();
    private readonly hydrationSnapshots = new WeakMap<HTMLElement, DataIslandSnapshot>();
    private readonly instanceIds = new WeakMap<HTMLElement, string>();
    private readonly dataRevisions = new WeakMap<HTMLElement, number>();
    private readonly renderBounds = new WeakMap<HTMLElement, RenderBounds>();
    private readonly committedRenderPlans = new WeakMap<HTMLElement, RenderPlan>();
    private readonly instanceStates = new WeakMap<HTMLElement, InstanceState>();
    private readonly sliceEventBindings = new WeakMap<Element, SliceEventBinding>();
    private readonly renderTokens = new WeakMap<HTMLElement, number>();
    private readonly renderSettled = new WeakMap<HTMLElement, Promise<void>>();
    private readonly declarationSettled = new WeakMap<object, Promise<void>>();
    /** Dedupes the async engine lowering of a legacy-xslt declaration across its instances. */
    private readonly legacyConversions = new WeakMap<CompiledDeclaration, Promise<void>>();
    private readonly srcDocuments = new Map<string, Promise<Document>>();
    private readonly moduleUrls = new Map<string, Promise<string>>();
    private readonly loadSrcDocumentOption?: CemElementRuntimeOptions['loadSrcDocument'];
    private readonly resolveModuleUrlOption?: CemElementRuntimeOptions['resolveModuleUrl'];
    private readonly resolveResourceUrlOption?: CemElementRuntimeOptions['resolveResourceUrl'];
    private readonly loadHttpResourceOption?: CemElementRuntimeOptions['loadHttpResource'];
    private readonly httpResourcePolicy: CemHttpResourcePolicy;
    private readonly runMode: RunMode;
    private readonly uidSeedOption?: CemElementRuntimeOptions['uidSeed'];
    private readonly uidSeedFallback: NonNullable<CemElementRuntimeOptions['uidSeedFallback']>;
    private readonly validateGeneratedIds: boolean;
    private readonly generatedScopeOwners = new Map<string, HTMLElement>();
    private instanceSequence = 0;

    constructor(options: CemElementRuntimeOptions = {}) {
        this.declarationTag = options.declarationTag ?? DEFAULT_DECLARATION_TAG;
        this.scopePolicyStamp = options.scopePolicyStamp ?? DEFAULT_SCOPE_POLICY_STAMP;
        this.privacyPolicyStamp = options.privacyPolicyStamp ?? DEFAULT_PRIVACY_POLICY_STAMP;
        this.logger = options.logger;
        this.loadSrcDocumentOption = options.loadSrcDocument;
        this.resolveModuleUrlOption = options.resolveModuleUrl;
        this.resolveResourceUrlOption = options.resolveResourceUrl;
        this.loadHttpResourceOption = options.loadHttpResource;
        this.httpResourcePolicy = { ...DEFAULT_HTTP_RESOURCE_POLICY, ...(options.httpResourcePolicy ?? {}) };
        this.runMode = options.runMode ?? 'application';
        this.uidSeedOption = options.uidSeed;
        this.uidSeedFallback = options.uidSeedFallback ?? (this.runMode === 'build-ssr' ? 'source-hash' : 'runtime');
        this.validateGeneratedIds = options.validateGeneratedIds ?? false;
        // Eagerly warm the cem_ql WASM engine so canonical CEM-ML instances can render
        // through the authoritative boundary as soon as possible. Failures surface
        // per-instance at render time.
        void ensureRuntimeReady().catch(() => undefined);
    }

    /**
     * Resolves once the most recent render for an instance has settled, including the
     * asynchronous `cem_ql` WASM render boundary for canonical CEM-ML. Synchronous
     * (DOM / legacy) renders resolve immediately.
     */
    whenRenderSettled(instance: HTMLElement): Promise<void> {
        return this.renderSettled.get(instance) ?? Promise.resolve();
    }

    install(host: CemElementWindow): void {
        if (host.customElements.get(this.declarationTag)) {
            return;
        }

        const registerDeclaration = this.registerDeclaration.bind(this);
        const BaseElement = host.HTMLElement;
        class CemElementDeclarationElement extends BaseElement {
            connectedCallback(): void {
                registerDeclaration(this);
            }
        }

        host.customElements.define(this.declarationTag, CemElementDeclarationElement);
    }

    registerDeclaration(declarationElement: HTMLElement): boolean {
        if (this.registeredDeclarationElements.has(declarationElement)) {
            return true;
        }

        const shape = analyzeDeclarationElement(declarationElement);
        if (!shape.ok || !shape.tag) {
            this.recordDiagnostics(declarationElement, shape.diagnostics);
            return false;
        }

        if (shape.src) {
            const reference = parseSrcReference(shape.src);
            if (!reference.local) {
                // External `src="./file#tag"`: fetch, parse, and register asynchronously.
                this.registeredDeclarationElements.add(declarationElement);
                this.declarationSettled.set(
                    declarationElement,
                    this.registerExternalDeclaration(declarationElement, shape.tag, shape.src, reference)
                );
                return true;
            }
            const localTemplate = this.resolveLocalSrcTemplate(declarationElement, shape.src, reference, shape.tag);
            if (!localTemplate) {
                return false;
            }
            this.registeredDeclarationElements.add(declarationElement);
            this.declarationSettled.set(
                declarationElement,
                this.registerResolvedDeclaration(declarationElement, shape.tag, localTemplate, shape.diagnostics)
            );
            return true;
        }

        const template = directTemplateChildren(declarationElement)[0] ?? implicitCemMlTemplate(declarationElement);
        this.registeredDeclarationElements.add(declarationElement);
        this.declarationSettled.set(
            declarationElement,
            this.registerResolvedDeclaration(declarationElement, shape.tag, template, shape.diagnostics)
        );
        return true;
    }

    /** Compile a resolved template, register the produced tag, and surface declaration diagnostics. */
    private registerResolvedDeclaration(
        declarationElement: HTMLElement,
        tag: string,
        template: HTMLTemplateElement,
        shapeDiagnostics: CemElementDiagnostic[]
    ): Promise<void> {
        const registry = declarationElement.ownerDocument.defaultView?.customElements;
        if (this.declarations.has(tag) || registry?.get(tag)) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.tag_already_defined',
                    `custom element \`${tag}\` is already defined`,
                    tag
                ),
            ]);
            return Promise.resolve();
        }

        const compiled = compileInlineDeclaration(declarationElement, tag, template, {
            declarationTag: this.declarationTag,
            uidSeed: this.uidSeedOption,
            uidSeedFallback: this.uidSeedFallback,
        });
        if (!this.validateGeneratedScopeUid(compiled)) {
            return Promise.resolve();
        }
        this.recordDiagnostics(declarationElement, [...shapeDiagnostics, ...compiled.diagnostics]);
        this.declarations.set(tag, compiled);
        this.defineProducedElement(declarationElement, compiled);
        // CEM-ML declaration parse diagnostics (structural well-formedness) come from the async
        // cem_ql WASM compile; cem-ql expression errors surface at render instead. Legacy-XSLT
        // declarations have no cemMlSource until the engine lowers them on first render, where their
        // conversion diagnostics surface — so they are not compiled here.
        if (compiled.mode === 'cem-ml' && compiled.cemMlSource !== null) {
            return this.surfaceDeclarationDiagnostics(declarationElement, compiled);
        }
        return Promise.resolve();
    }

    /**
     * Load and register an external `src="./file#tag"` declaration: fetch the referenced
     * document (through the host loader / module-map resolver), parse it, resolve the
     * `#fragment` to its `<template>`, and register the produced tag from it.
     */
    private async registerExternalDeclaration(
        declarationElement: HTMLElement,
        tag: string,
        src: string,
        reference: SrcReference
    ): Promise<void> {
        let document: Document;
        try {
            document = await this.loadSrcDocumentParsed(declarationElement, reference.path);
        } catch (error) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.src_load_failed',
                    `loading \`${src}\` failed: ${error instanceof Error ? error.message : String(error)}`,
                    tag
                ),
            ]);
            return;
        }
        const sourceTemplate = templateFromTarget(document.getElementById(reference.id));
        if (!sourceTemplate) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.src_target_missing',
                    `external \`src\` reference \`${src}\` did not resolve to a <template> for \`#${reference.id}\``,
                    tag
                ),
            ]);
            return;
        }
        const template = declarationElement.ownerDocument.importNode(sourceTemplate, true) as HTMLTemplateElement;
        await this.registerResolvedDeclaration(declarationElement, tag, template, []);
    }

    /** Resolve a same-document `src="#id"` reference to its `<template>`, or diagnose a miss. */
    private resolveLocalSrcTemplate(
        declarationElement: HTMLElement,
        src: string,
        reference: SrcReference,
        tag: string
    ): HTMLTemplateElement | undefined {
        const template = templateFromTarget(declarationElement.ownerDocument.getElementById(reference.id));
        if (!template) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.src_local_target_missing',
                    `local \`src\` reference \`${src}\` did not resolve to a same-document <template>`,
                    tag
                ),
            ]);
        }
        return template;
    }

    /** Fetch + parse the document an external `src` references, cached per declaring document and path. */
    private loadSrcDocumentParsed(declarationElement: HTMLElement, path: string): Promise<Document> {
        const baseDocument = declarationElement.ownerDocument;
        const key = `${baseDocument.baseURI}\n${path}`;
        const cached = this.srcDocuments.get(key);
        if (cached) {
            return cached;
        }
        const parsed = this.loadSrcDocument(path, baseDocument).then((html) =>
            new DOMParser().parseFromString(html, 'text/html')
        );
        this.srcDocuments.set(key, parsed);
        return parsed;
    }

    private loadSrcDocument(path: string, baseDocument: Document): Promise<string> {
        return this.loadSrcDocumentOption
            ? this.loadSrcDocumentOption(path, baseDocument)
            : defaultLoadSrcDocument(path, baseDocument);
    }

    diagnosticsFor(target: object): readonly CemElementDiagnostic[] {
        return this.diagnostics.get(target) ?? [];
    }

    /**
     * Resolves once a declaration's asynchronous parse diagnostics (from the cem_ql WASM
     * compile) have been recorded. Synchronous (DOM / legacy) declarations resolve
     * immediately.
     */
    whenDeclarationSettled(declaration: object): Promise<void> {
        return this.declarationSettled.get(declaration) ?? Promise.resolve();
    }

    private async surfaceDeclarationDiagnostics(
        declarationElement: HTMLElement,
        compiled: CompiledDeclaration
    ): Promise<void> {
        try {
            const diagnostics = await compileCemMlTemplate(compiled.cemMlSource ?? '');
            if (diagnostics.length > 0) {
                this.recordDiagnostics(
                    declarationElement,
                    diagnostics.map((diagnostic) => declarationRuntimeSupportDiagnostic(diagnostic, compiled.producedTag))
                );
            }
        } catch {
            // WASM unavailable — declaration diagnostics are best-effort.
        }
    }

    snapshotInstance(instance: HTMLElement): DataIslandSnapshot {
        const declaration = this.declarationForInstance(instance);
        if (!declaration) {
            throw new Error(`No <${this.declarationTag}> declaration registered for <${instance.localName}>`);
        }
        const island = this.ensureDataIsland(instance);
        return this.createSnapshot(instance, declaration, island);
    }

    private defineProducedElement(declarationElement: HTMLElement, compiled: CompiledDeclaration): void {
        const registry = declarationElement.ownerDocument.defaultView?.customElements;
        const baseElement = declarationElement.ownerDocument.defaultView?.HTMLElement;
        if (!registry || !baseElement) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.registry_unavailable',
                    'customElements registry is unavailable for this declaration document',
                    compiled.producedTag
                ),
            ]);
            return;
        }

        if (registry.get(compiled.producedTag)) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.tag_already_defined',
                    `custom element \`${compiled.producedTag}\` is already defined`,
                    compiled.producedTag
                ),
            ]);
            return;
        }

        const connectProducedInstance = this.connectProducedInstance.bind(this);
        const disconnectProducedInstance = this.disconnectProducedInstance.bind(this);
        // No `observedAttributes`/`attributeChangedCallback`: the declared-attribute list
        // is only known after the async WASM compile, but `observedAttributes` is read once
        // at definition time. Instead a per-instance MutationObserver (set up on connect)
        // watches every host attribute and schedules an async re-render — see
        // `observeInstance`. This keeps the element defined synchronously and observes
        // attributes the synchronous path could not have known.
        class ProducedCemElement extends baseElement {
            connectedCallback(): void {
                connectProducedInstance(this, compiled);
            }

            disconnectedCallback(): void {
                disconnectProducedInstance(this);
            }
        }

        registry.define(compiled.producedTag, ProducedCemElement);
    }

    private connectProducedInstance(instance: HTMLElement, compiled: CompiledDeclaration): void {
        const island = this.ensureDataIsland(instance);
        this.ensureInstanceScope(instance, compiled);
        const state = this.ensureInstanceState(instance, compiled, island);
        this.observeInstance(instance, island, state);
        if (this.hydratedServerRenders.has(instance)) {
            this.renderSettled.set(instance, Promise.resolve());
            return;
        }
        this.renderInstance(instance, compiled);
    }

    private disconnectProducedInstance(instance: HTMLElement): void {
        const state = this.instanceStates.get(instance);
        state?.observer?.disconnect();
        if (state) {
            for (const active of Object.values(state.httpResources)) {
                active.controller.abort();
            }
            for (const active of Object.values(state.localStorageResources)) {
                active.destroy?.();
            }
        }
    }

    /**
     * Establish per-instance mutation observation. The single observer watches two
     * targets: the host element's attributes (replacing `observedAttributes` /
     * `attributeChangedCallback`) and the inert data-island content. Either kind of
     * mutation invalidates the instance and schedules an async re-render that reads the
     * live attributes/state fresh. Observing every attribute means a change to any
     * attribute — declared or not, even ones only resolvable after the async render —
     * reliably re-renders. Idempotent, so it also re-attaches on reconnect.
     *
     * Re-entrancy is structurally precluded: the runtime never mutates an observed target
     * during render (render output is written to the light DOM between render-boundary
     * comments, not to host attributes or to `island.content`), so a render cannot
     * self-trigger this observer. A future host-attribute write would need to drain its
     * own record via `observer.takeRecords()`.
     */
    private observeInstance(instance: HTMLElement, island: HTMLTemplateElement, state: InstanceState): void {
        const observer = state.observer;
        if (!observer) {
            return;
        }
        observer.disconnect();
        observer.observe(instance, { attributes: true });
        observer.observe(island.content, {
            childList: true,
            subtree: true,
            characterData: true,
            attributes: true,
        });
    }

    private invalidateProducedInstance(instance: HTMLElement, compiled: CompiledDeclaration): void {
        if (!this.initializedInstances.has(instance) || !instance.isConnected) {
            return;
        }
        this.renderInstance(instance, compiled);
    }

    private renderInstance(instance: HTMLElement, compiled: CompiledDeclaration): void {
        const island = this.ensureDataIsland(instance);
        this.ensureInstanceState(instance, compiled, island);
        const snapshot = this.createSnapshot(instance, compiled, island);
        const token = this.nextRenderToken(instance);

        if (compiled.wasmEligible && (compiled.cemMlSource !== null || compiled.legacySource !== null)) {
            // Canonical CEM-ML — and legacy HTML+XSLT (lowered by the engine on first render) —
            // render through the authoritative `cem_ql` WASM boundary.
            this.renderSettled.set(instance, this.renderViaWasm(instance, compiled, snapshot, token));
            return;
        }

        // DOM parity and legacy bridge templates render synchronously through the
        // projection path.
        const renderPlan = this.renderFromDeclaration(instance, compiled, snapshot);
        this.renderSettled.set(
            instance,
            renderPlan ? this.commitRenderPlan(instance, compiled, island, renderPlan, token) : Promise.resolve()
        );
    }

    /**
     * Lower a legacy HTML+XSLT declaration to canonical CEM-ML through the CEM-owned engine, once per
     * declaration (shared across its instances). Fills {@link CompiledDeclaration.cemMlSource} and
     * surfaces conversion diagnostics on the declaration element. No-op for CEM-ML declarations.
     */
    private ensureLegacyConverted(compiled: CompiledDeclaration): Promise<void> {
        if (compiled.cemMlSource !== null || compiled.legacySource === null) {
            return Promise.resolve();
        }
        let conversion = this.legacyConversions.get(compiled);
        if (!conversion) {
            conversion = convertLegacyTemplate(compiled.legacySource).then((converted) => {
                compiled.cemMlSource = converted.source;
                if (converted.diagnostics.length > 0) {
                    this.recordDiagnostics(
                        compiled.declarationElement,
                        converted.diagnostics.map((diagnostic) =>
                            runtimeSupportDiagnostic(diagnostic, compiled.producedTag)
                        )
                    );
                }
            });
            this.legacyConversions.set(compiled, conversion);
        }
        return conversion;
    }

    private async renderViaWasm(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        snapshot: DataIslandSnapshot,
        token: number
    ): Promise<void> {
        try {
            // Legacy HTML+XSLT is lowered to CEM-ML by the engine on first render (cached).
            await this.ensureLegacyConverted(compiled);
            if (this.renderTokens.get(instance) !== token) {
                return; // a newer render superseded this one mid-flight
            }
            const source = compiled.cemMlSource ?? '';
            const data = wasmTemplateData(snapshot, compiled.declaredAttributes);
            const result = await processCemMlTemplate({
                source,
                data,
                payload: snapshot.payload,
                identity: {
                    producedTag: compiled.producedTag,
                    instanceId: snapshot.instanceId,
                    templateArtifactId: compiled.artifactId,
                    dataRevision: snapshot.dataRevision,
                    outputTarget: snapshot.outputTarget,
                    scopePolicyStamp: snapshot.scopePolicyStamp,
                },
                renderNodeIdPrefix: compiled.producedTag,
            });
            if (this.renderTokens.get(instance) !== token) {
                return; // a newer render superseded this one mid-flight
            }
            if (result.diagnostics.length > 0) {
                this.recordDiagnostics(
                    instance,
                    result.diagnostics.map((diagnostic) =>
                        runtimeSupportDiagnostic(diagnostic, compiled.producedTag)
                    )
                );
            }
            const scoped = scopeRenderPlan(result.renderPlan, this.currentScopeUid(instance, compiled));
            this.recordDiagnostics(
                instance,
                scoped.diagnostics.map((diagnostic) => scopedCssDiagnostic(diagnostic, compiled.producedTag))
            );
            const island = this.ensureDataIsland(instance);
            await this.commitRenderPlan(instance, compiled, island, scoped.renderPlan, token);
        } catch (error) {
            if (this.renderTokens.get(instance) !== token) {
                return;
            }
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.wasm_render_failed',
                    error instanceof Error ? error.message : 'cem_ql WASM render failed',
                    compiled.producedTag
                ),
            ]);
        }
    }

    private nextRenderToken(instance: HTMLElement): number {
        const token = (this.renderTokens.get(instance) ?? 0) + 1;
        this.renderTokens.set(instance, token);
        return token;
    }

    private renderFromDeclaration(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        snapshot: DataIslandSnapshot
    ): RenderPlan | null {
        // UI adapter → processing layer → UI adapter: project the serializable template
        // source against a serializable data-island snapshot, then hand the scoped plan
        // to the DOM commit helper.
        try {
            const values = templateValues(snapshot, compiled.declaredAttributes);
            const input = { snapshot, values };
            const plan = projectTemplate(compiled.templateSource, input);
            const scoped = scopeRenderPlan(plan, this.currentScopeUid(instance, compiled));
            this.recordDiagnostics(
                instance,
                scoped.diagnostics.map((diagnostic) => scopedCssDiagnostic(diagnostic, compiled.producedTag))
            );
            return scoped.renderPlan;
        } catch (error) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.render_failed',
                    error instanceof Error ? error.message : 'render failed',
                    compiled.producedTag
                ),
            ]);
            return null;
        }
    }

    private ensureDataIsland(instance: HTMLElement): HTMLTemplateElement {
        const existing = directDataIsland(instance);
        if (existing) {
            if (!this.initializedInstances.has(instance)) {
                if (this.adoptServerRenderedInstance(instance, existing)) {
                    return existing;
                }
                for (const child of Array.from(instance.childNodes)) {
                    if (child !== existing && !isRenderBoundary(child)) {
                        existing.content.appendChild(child);
                    }
                }
                this.initializedInstances.add(instance);
            }
            return existing;
        }

        const island = instance.ownerDocument.createElement('template') as HTMLTemplateElement;
        island.setAttribute(DATA_ISLAND_ATTR, DATA_ISLAND_VALUE);
        while (instance.firstChild) {
            island.content.appendChild(instance.firstChild);
        }
        instance.appendChild(island);
        this.initializedInstances.add(instance);
        return island;
    }

    private adoptServerRenderedInstance(instance: HTMLElement, island: HTMLTemplateElement): boolean {
        const metadata = directHydrationMetadata(instance);
        const bounds = directRenderBounds(instance);
        if (!metadata && !bounds) {
            return false;
        }
        if (!metadata) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.hydration_metadata_missing',
                    'SSR hydration render boundaries were present but hydration metadata was missing',
                    instance.localName
                ),
            ]);
            return false;
        }
        if (!bounds) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.hydration_boundaries_missing',
                    'SSR hydration metadata was present but render boundaries were missing',
                    instance.localName
                ),
            ]);
            return false;
        }

        const parsed = parseHydrationSnapshot(metadata);
        if (!parsed.ok) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    parsed.code,
                    parsed.message,
                    instance.localName
                ),
            ]);
            return false;
        }
        const snapshot = parsed.snapshot;
        if (snapshot.producedTag !== instance.localName || snapshot.outputTarget !== 'light-dom') {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.hydration_metadata_invalid',
                    'SSR hydration metadata did not match the produced element',
                    instance.localName
                ),
            ]);
            return false;
        }
        const identityDiagnostics = hydrationRenderIdentityDiagnostics(instance, bounds, snapshot);
        if (identityDiagnostics.length > 0) {
            this.recordDiagnostics(instance, identityDiagnostics);
            return false;
        }

        // BR-VC-9: the snapshot/`datadom` is a data/security contract. If the
        // persisted snapshot declares a schema version this build does not fully
        // understand (a higher MINOR = unknown optional features, or a MAJOR
        // mismatch = must-understand), apply the run-mode disposition. In an
        // application run the data/security disposition is reject — refuse to
        // trust the un-understood snapshot and fall back to a fresh render rather
        // than silently honoring or dropping unknown fields.
        const ingest = ingestContractVersion(
            snapshot.version,
            SNAPSHOT_SCHEMA_VERSION,
            this.runMode,
            'data-snapshot'
        );
        if (!ingest.accept) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.snapshot_version_rejected',
                    ingest.decision?.rationale ??
                        `SSR hydration snapshot version ${String(snapshot.version)} is not understood by build ${SNAPSHOT_SCHEMA_VERSION} (${ingest.reason})`,
                    instance.localName
                ),
            ]);
            return false;
        }

        this.initializedInstances.add(instance);
        this.hydratedServerRenders.add(instance);
        this.hydrationSnapshots.set(instance, snapshot);
        this.renderBounds.set(instance, bounds);
        this.instanceIds.set(instance, snapshot.instanceId);
        this.dataRevisions.set(instance, parseDataRevision(snapshot.dataRevision));
        island.setAttribute(DATA_ISLAND_ATTR, DATA_ISLAND_VALUE);
        return true;
    }

    private ensureInstanceState(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        island: HTMLTemplateElement
    ): InstanceState {
        const existing = this.instanceStates.get(instance);
        if (existing) {
            return existing;
        }

        const hydrationSnapshot = this.hydrationSnapshots.get(instance);
        const state: InstanceState = {
            slices: hydrationSnapshot
                ? templateValueRecord(hydrationSnapshot.slices)
                : Object.fromEntries(compiled.declaredSlices.map((slice) => [slice.name, slice.defaultValue])),
            eventPayloads: hydrationSnapshot?.eventPayloads ?? {},
            httpResources: {},
            localStorageResources: {},
            resourceRevisions: {},
        };
        const observer = island.ownerDocument.defaultView?.MutationObserver;
        if (observer) {
            // Observation targets are attached in `observeInstance` (on connect), so the
            // observer can be torn down on disconnect and re-attached on reconnect.
            state.observer = new observer(() => this.invalidateProducedInstance(instance, compiled));
        }
        this.instanceStates.set(instance, state);
        return state;
    }

    private bindRenderedSliceEvents(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        rendered: ParentNode
    ): void {
        for (const element of Array.from(rendered.querySelectorAll('[slice][slice-event]'))) {
            this.bindRenderedSliceEventElement(instance, compiled, element);
        }
    }

    private bindRenderedSliceEventsInRange(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        bounds: RenderBounds
    ): void {
        for (const element of renderedElementsBetween(bounds, '[slice][slice-event]')) {
            this.bindRenderedSliceEventElement(instance, compiled, element);
        }
    }

    private bindRenderedSliceEventElement(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        element: Element
    ): void {
        const sliceName = element.getAttribute('slice')?.trim();
        const eventName = element.getAttribute('slice-event')?.trim();
        if (!sliceName || !eventName) {
            return;
        }
        const expression = element.getAttribute('slice-value') ?? '{$target.value}';
        element.removeAttribute('slice');
        element.removeAttribute('slice-event');
        element.removeAttribute('slice-value');

        const existing = this.sliceEventBindings.get(element);
        if (
            existing &&
            existing.instance === instance &&
            existing.sliceName === sliceName &&
            existing.eventName === eventName &&
            existing.expression === expression
        ) {
            return;
        }
        if (existing) {
            element.removeEventListener(existing.eventName, existing.listener);
        }

        const listener: EventListener = (event) => {
            this.writeSliceFromEvent(instance, compiled, sliceName, expression, event);
        };
        element.addEventListener(eventName, listener);
        this.sliceEventBindings.set(element, { instance, sliceName, eventName, expression, listener });
    }

    private bindRenderedResourceSlices(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        rendered: ParentNode,
        token: number
    ): Promise<void> {
        return this.bindRenderedResourceSliceElements(
            instance,
            compiled,
            Array.from(rendered.querySelectorAll('module-url,http-request,local-storage')),
            token
        );
    }

    private bindRenderedResourceSlicesInRange(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        bounds: RenderBounds,
        token: number
    ): Promise<void> {
        return this.bindRenderedResourceSliceElements(
            instance,
            compiled,
            renderedElementsBetween(bounds, 'module-url,http-request,local-storage'),
            token
        );
    }

    private bindRenderedResourceSliceElements(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        resourceElements: Element[],
        token: number
    ): Promise<void> {
        if (resourceElements.length === 0) {
            return Promise.resolve();
        }

        const moduleTasks: Promise<RenderedResourceResult>[] = [];
        const httpSettled: Promise<void>[] = [];
        for (const element of resourceElements) {
            const localName = element.localName;
            const sliceName = element.getAttribute('slice')?.trim() ?? '';
            const specifier = element.getAttribute('src')?.trim() ?? '';
            const httpRequest = localName === 'http-request' ? readHttpRequestDeclaration(element) : null;
            const localStorage = localName === 'local-storage' ? readLocalStorageDeclaration(element) : null;
            element.remove();
            if (localName === 'module-url') {
                if (!sliceName || !specifier) {
                    continue;
                }
                moduleTasks.push(
                    this.resolveModuleUrl(specifier, instance.ownerDocument)
                        .then((value) => ({ kind: 'module-url' as const, sliceName, specifier, value }))
                        .catch((error: unknown) => ({
                            kind: 'module-url' as const,
                            sliceName,
                            specifier,
                            value: specifier,
                            error,
                        }))
                );
                continue;
            }
            if (httpRequest) {
                httpSettled.push(this.startHttpRequestResource(instance, compiled, httpRequest));
                continue;
            }
            if (localStorage) {
                this.bindLocalStorageResource(instance, compiled, localStorage);
            }
        }
        if (moduleTasks.length === 0 && httpSettled.length === 0) {
            return Promise.resolve();
        }

        const modulesSettled = Promise.all(moduleTasks).then(async (resolved) => {
            if (this.renderTokens.get(instance) !== token || !instance.isConnected) {
                return;
            }
            const island = this.ensureDataIsland(instance);
            const state = this.ensureInstanceState(instance, compiled, island);
            let changed = false;
            const diagnostics: CemElementDiagnostic[] = [];
            for (const result of resolved) {
                if (result.kind === 'module-url') {
                    if (state.slices[result.sliceName] !== result.value) {
                        state.slices[result.sliceName] = result.value;
                        changed = true;
                    }
                    state.eventPayloads[result.sliceName] = {
                        type: 'module-url',
                        src: result.specifier,
                        value: result.value,
                    };
                    if (result.error) {
                        diagnostics.push(
                            resourceDiagnostic(
                                'cem-element.module_url_resolve_failed',
                                `module-url \`${result.specifier}\` could not be resolved: ${
                                    result.error instanceof Error ? result.error.message : String(result.error)
                                }`,
                                compiled.producedTag
                            )
                        );
                    }
                    continue;
                }
            }
            this.recordDiagnostics(instance, diagnostics);
            if (changed) {
                this.renderInstance(instance, compiled);
                await this.whenRenderSettled(instance);
            }
        });
        return Promise.all([modulesSettled, ...httpSettled]).then(() => undefined);
    }

    private resolveModuleUrl(specifier: string, baseDocument: Document): Promise<string> {
        const key = `${baseDocument.baseURI}\n${specifier}`;
        const cached = this.moduleUrls.get(key);
        if (cached) {
            return cached;
        }
        const resolved = Promise.resolve(
            this.resolveModuleUrlOption
                ? this.resolveModuleUrlOption(specifier, baseDocument)
                : defaultResolveModuleUrl(specifier, baseDocument)
        ).then((value) => String(value));
        this.moduleUrls.set(key, resolved);
        return resolved;
    }

    private bindLocalStorageResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        declaration: LocalStorageDeclaration
    ): void {
        const window = instance.ownerDocument.defaultView;
        const storage = localStorageForWindow(window);
        if (!storage || !window) {
            this.recordDiagnostics(instance, [
                resourceDiagnostic(
                    'cem-element.local_storage_unavailable',
                    `local-storage key \`${declaration.key}\` cannot be read in this browser context`,
                    compiled.producedTag,
                    'warning'
                ),
            ]);
            return;
        }

        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        let active: ActiveLocalStorageResource | undefined = state.localStorageResources[declaration.sliceName];
        if (active && !sameLocalStorageDeclaration(active, declaration)) {
            active.destroy?.();
            delete state.localStorageResources[declaration.sliceName];
            active = undefined;
        }

        let source: LocalStorageSliceSource = 'retained';
        let nextValue: unknown;
        let nextRawValue: string | null;
        let needsRerender = false;

        if (declaration.initialValue !== undefined) {
            nextValue = localStorageStringToValue(declaration.storageType, declaration.initialValue, instance.ownerDocument);
            nextRawValue = localStorageValueToString(declaration.storageType, nextValue);
            writeLocalStorageRaw(storage, declaration.key, nextRawValue);
            source = 'value-attribute';
            needsRerender = this.writeLocalStorageSlice(
                state,
                declaration,
                nextValue,
                nextRawValue,
                source,
                active
            );
        } else if (!active) {
            nextRawValue = storage.getItem(declaration.key);
            nextValue = localStorageStringToValue(declaration.storageType, nextRawValue, instance.ownerDocument);
            source = 'initial-read';
            needsRerender = this.writeLocalStorageSlice(
                state,
                declaration,
                nextValue,
                nextRawValue,
                source,
                active
            );
        } else {
            const sliceValue = state.slices[declaration.sliceName];
            if (!localStorageValuesEqual(sliceValue, active.lastValue)) {
                nextValue = sliceValue;
                nextRawValue = localStorageValueToString(declaration.storageType, nextValue);
                writeLocalStorageRaw(storage, declaration.key, nextRawValue);
                source = 'slice-write';
                this.writeLocalStorageSlice(state, declaration, nextValue, nextRawValue, source, active);
            } else {
                nextValue = active.lastValue;
                nextRawValue = active.lastRawValue;
            }
        }

        active = this.ensureActiveLocalStorageResource(instance, compiled, state, declaration, nextValue, nextRawValue);
        if (source !== 'retained') {
            this.writeLocalStorageEventPayload(state, declaration, nextValue, nextRawValue, source);
            active.lastValue = nextValue;
            active.lastRawValue = nextRawValue;
        }
        if (needsRerender) {
            queueMicrotask(() => {
                if (instance.isConnected) {
                    this.renderInstance(instance, compiled);
                }
            });
        }
    }

    private writeLocalStorageSlice(
        state: InstanceState,
        declaration: LocalStorageDeclaration,
        value: unknown,
        rawValue: string | null,
        source: LocalStorageSliceSource,
        active: ActiveLocalStorageResource | undefined
    ): boolean {
        const changed = !localStorageValuesEqual(state.slices[declaration.sliceName], value);
        if (changed) {
            state.slices[declaration.sliceName] = value;
        }
        this.writeLocalStorageEventPayload(state, declaration, value, rawValue, source);
        if (active) {
            active.lastValue = value;
            active.lastRawValue = rawValue;
        }
        return changed;
    }

    private writeLocalStorageEventPayload(
        state: InstanceState,
        declaration: LocalStorageDeclaration,
        value: unknown,
        rawValue: string | null,
        source: LocalStorageSliceSource
    ): void {
        state.eventPayloads[declaration.sliceName] = {
            type: 'local-storage',
            key: declaration.key,
            storageType: declaration.storageType,
            live: declaration.live,
            source,
            value,
            rawValue,
        };
    }

    private ensureActiveLocalStorageResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        state: InstanceState,
        declaration: LocalStorageDeclaration,
        value: unknown,
        rawValue: string | null
    ): ActiveLocalStorageResource {
        const existing = state.localStorageResources[declaration.sliceName];
        if (existing) {
            return existing;
        }
        const active: ActiveLocalStorageResource = {
            key: declaration.key,
            storageType: declaration.storageType,
            live: declaration.live,
            lastValue: value,
            lastRawValue: rawValue,
        };
        if (declaration.live) {
            active.destroy = this.bindLocalStorageLiveListener(instance, compiled, state, declaration, active);
        }
        state.localStorageResources[declaration.sliceName] = active;
        return active;
    }

    private bindLocalStorageLiveListener(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        state: InstanceState,
        declaration: LocalStorageDeclaration,
        active: ActiveLocalStorageResource
    ): () => void {
        const window = instance.ownerDocument.defaultView;
        const storage = localStorageForWindow(window);
        if (!window || !storage) {
            return () => undefined;
        }
        ensureTrackedLocalStorage(window);
        const listener = (event: Event) => {
            const changedKey = localStorageChangedKey(event);
            if (changedKey !== null && changedKey !== declaration.key) {
                return;
            }
            const rawValue = storage.getItem(declaration.key);
            const value = localStorageStringToValue(declaration.storageType, rawValue, instance.ownerDocument);
            active.lastValue = value;
            active.lastRawValue = rawValue;
            if (this.writeLocalStorageSlice(state, declaration, value, rawValue, 'storage-event', active)) {
                this.renderInstance(instance, compiled);
            }
        };
        window.addEventListener('storage', listener);
        window.addEventListener(LOCAL_STORAGE_EVENT, listener);
        return () => {
            window.removeEventListener('storage', listener);
            window.removeEventListener(LOCAL_STORAGE_EVENT, listener);
        };
    }

    private startHttpRequestResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        declaration: HttpRequestDeclaration
    ): Promise<void> {
        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        const key = httpRequestCacheKey({
            baseUrl: instance.ownerDocument.baseURI,
            scope: this.currentScopeUid(instance, compiled),
            declaration,
            policy: this.httpResourcePolicy,
        });
        const active = state.httpResources[declaration.sliceName];
        if (active?.key === key) {
            return active.settled;
        }
        if (active) {
            active.controller.abort();
        }

        const revision = (state.resourceRevisions[declaration.sliceName] ?? 0) + 1;
        state.resourceRevisions[declaration.sliceName] = revision;
        const controller = new AbortController();
        const pending = this.httpRequestEnvelope({
            declaration,
            revision,
            state: 'pending',
            request: unresolvedHttpRequestMetadata(declaration, this.scopePolicyStamp),
            data: null,
            diagnostics: [],
        });
        this.writeHttpResourceEnvelope(instance, compiled, state, declaration.sliceName, pending);
        this.scheduleResourceRerender(instance, compiled, declaration.sliceName, revision);

        const settled = this.runHttpRequestResource(instance, compiled, declaration, key, revision, controller);
        state.httpResources[declaration.sliceName] = { key, revision, controller, settled };
        return settled;
    }

    private async resolveHttpResourceUrl(
        request: CemResourceResolutionRequest,
        baseDocument: Document
    ): Promise<CemResourceResolution> {
        const result = await Promise.resolve(
            this.resolveResourceUrlOption
                ? this.resolveResourceUrlOption(request, baseDocument)
                : defaultResolveResourceUrl(request, baseDocument, this.scopePolicyStamp, this.httpResourcePolicy)
        );
        if (typeof result === 'string') {
            return {
                authoredUrl: request.authoredUrl,
                resolvedUrl: result,
                resolverIdentity: `host:${baseDocument.baseURI}`,
                resourcePolicyStamp: this.scopePolicyStamp,
            };
        }
        return result;
    }

    private async runHttpRequestResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        declaration: HttpRequestDeclaration,
        key: string,
        revision: number,
        controller: AbortController
    ): Promise<void> {
        let timeout: ReturnType<typeof setTimeout> | undefined;
        try {
            if (declaration.method !== 'GET' && declaration.method !== 'HEAD') {
                throw new HttpResourceError(
                    'cem-element.http_request_method_unsupported',
                    `method ${declaration.method} is not supported; use GET or HEAD`
                );
            }
            const method = declaration.method;
            const resolutionRequest: CemResourceResolutionRequest = {
                kind: 'http-request',
                authoredUrl: declaration.authoredUrl,
                baseUrl: instance.ownerDocument.baseURI,
                declarationScopeId: this.currentScopeUid(instance, compiled),
                method,
                headers: declaration.headers,
                expectedContentType: declaration.expectedContentType,
            };
            const resolution = await this.resolveHttpResourceUrl(resolutionRequest, instance.ownerDocument);
            if (!this.isActiveHttpResource(instance, declaration.sliceName, key, revision)) {
                return;
            }
            if (this.httpResourcePolicy.timeoutMs > 0) {
                timeout = setTimeout(() => controller.abort(), this.httpResourcePolicy.timeoutMs);
            }
            const request: CemHttpRequest = {
                authoredUrl: resolution.authoredUrl,
                baseUrl: instance.ownerDocument.baseURI,
                resolvedUrl: resolution.resolvedUrl,
                resolverIdentity: resolution.resolverIdentity,
                resourcePolicyStamp: resolution.resourcePolicyStamp,
                method,
                headers: declaration.headers,
                credentials: declaration.credentials,
                cache: declaration.cache,
                expectedContentType: declaration.expectedContentType ?? resolution.contentTypeHint,
                policy: this.httpResourcePolicy,
                signal: controller.signal,
            };
            const requestMetadata = httpRequestMetadata(request);
            const loaded = await (this.loadHttpResourceOption
                ? this.loadHttpResourceOption(request)
                : defaultLoadHttpResource(request));
            if (!this.isActiveHttpResource(instance, declaration.sliceName, key, revision)) {
                return;
            }
            this.updateHttpResourceAndRerender(instance, compiled, declaration.sliceName, revision, {
                declaration,
                revision,
                state: 'headers',
                request: requestMetadata,
                response: loaded.response,
                data: null,
                diagnostics: [],
            });
            const parse = await parseHttpResourceData(
                request,
                loaded.response,
                loaded.body,
                request.expectedContentType,
                request.policy.maxResponseBytes,
                request.signal,
                compiled.producedTag
            );
            if (!this.isActiveHttpResource(instance, declaration.sliceName, key, revision)) {
                return;
            }
            this.updateHttpResourceAndRerender(instance, compiled, declaration.sliceName, revision, {
                declaration,
                revision,
                state: parse.ok ? 'complete' : 'error',
                request: requestMetadata,
                response: loaded.response,
                sourceId: parse.sourceId,
                data: parse.data,
                diagnostics: parse.diagnostics,
            });
        } catch (error) {
            if (!this.isActiveHttpResource(instance, declaration.sliceName, key, revision)) {
                return;
            }
            const aborted = controller.signal.aborted;
            const diagnosticCode =
                error instanceof HttpResourceError
                    ? error.code
                    : aborted
                      ? 'cem-element.http_request_aborted'
                      : 'cem-element.http_request_load_failed';
            this.updateHttpResourceAndRerender(instance, compiled, declaration.sliceName, revision, {
                declaration,
                revision,
                state: aborted ? 'aborted' : 'error',
                request: unresolvedHttpRequestMetadata(declaration, this.scopePolicyStamp),
                data: null,
                diagnostics: [
                    resourceDiagnostic(
                        diagnosticCode,
                        `http-request \`${declaration.authoredUrl}\` failed: ${
                            error instanceof Error ? error.message : String(error)
                        }`,
                        compiled.producedTag,
                        aborted ? 'warning' : 'error'
                    ),
                ],
            });
        } finally {
            if (timeout) {
                clearTimeout(timeout);
            }
        }
    }

    private httpRequestEnvelope(input: {
        declaration: HttpRequestDeclaration;
        revision: number;
        state: CemHttpResourceState;
        request: CemHttpResourceEnvelope['request'];
        response?: CemHttpResponseHead;
        sourceId?: CemHttpResourceSourceId;
        data: unknown;
        diagnostics: CemElementDiagnostic[];
    }): CemHttpResourceEnvelope {
        return {
            kind: 'http-request',
            state: input.state,
            resourceRevision: input.revision,
            request: input.request,
            response: input.response,
            sourceId: input.sourceId,
            data: input.data,
            diagnostics: input.diagnostics,
        };
    }

    private writeHttpResourceEnvelope(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        state: InstanceState,
        sliceName: string,
        envelope: CemHttpResourceEnvelope
    ): void {
        state.slices[sliceName] = envelope;
        state.eventPayloads[sliceName] = {
            type: 'http-request',
            state: envelope.state,
            resourceRevision: envelope.resourceRevision,
            request: envelope.request,
            response: envelope.response,
            sourceId: envelope.sourceId,
            diagnostics: envelope.diagnostics,
        };
        this.recordDiagnostics(instance, envelope.diagnostics);
    }

    private updateHttpResourceAndRerender(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        sliceName: string,
        revision: number,
        input: Parameters<CemElementRuntime['httpRequestEnvelope']>[0]
    ): void {
        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        if (state.resourceRevisions[sliceName] !== revision || !instance.isConnected) {
            return;
        }
        this.writeHttpResourceEnvelope(instance, compiled, state, sliceName, this.httpRequestEnvelope(input));
        this.scheduleResourceRerender(instance, compiled, sliceName, revision);
    }

    private scheduleResourceRerender(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        sliceName: string,
        revision: number
    ): void {
        queueMicrotask(() => {
            if (
                instance.isConnected &&
                this.instanceStates.get(instance)?.resourceRevisions[sliceName] === revision
            ) {
                this.renderInstance(instance, compiled);
            }
        });
    }

    private isActiveHttpResource(
        instance: HTMLElement,
        sliceName: string,
        key: string,
        revision: number
    ): boolean {
        const active = this.instanceStates.get(instance)?.httpResources[sliceName];
        return Boolean(active && active.key === key && active.revision === revision && instance.isConnected);
    }

    private writeSliceFromEvent(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        sliceName: string,
        expression: string,
        event: Event
    ): void {
        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        const sliceValue = evaluateSliceValue(expression, event, state.slices);
        state.eventPayloads[sliceName] = serializeEventPayload(event, sliceValue);
        if (state.slices[sliceName] !== sliceValue) {
            state.slices[sliceName] = sliceValue;
            this.renderInstance(instance, compiled);
        }
    }

    private ensureInstanceScope(instance: HTMLElement, compiled: CompiledDeclaration): string {
        const existing = instance.getAttribute(DATA_CEM_SCOPE_ATTR) ?? this.retainedRenderedScope(instance);
        const scopeUid = existing && existing.length > 0 ? existing : compiled.scopeUid;
        if (instance.getAttribute(DATA_CEM_SCOPE_ATTR) !== scopeUid) {
            instance.setAttribute(DATA_CEM_SCOPE_ATTR, scopeUid);
        }
        return scopeUid;
    }

    private currentScopeUid(instance: HTMLElement, compiled: CompiledDeclaration): string {
        return instance.getAttribute(DATA_CEM_SCOPE_ATTR) || compiled.scopeUid;
    }

    private retainedRenderedScope(instance: HTMLElement): string | null {
        const bounds = this.renderBounds.get(instance);
        if (!bounds) {
            return null;
        }
        return firstRenderedElementBetween(bounds)?.getAttribute(DATA_CEM_SCOPE_ATTR) ?? null;
    }

    private commitRenderPlan(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        island: HTMLTemplateElement,
        renderPlan: RenderPlan,
        token: number
    ): Promise<void> {
        const previous = this.committedRenderPlans.get(instance) ?? null;
        if (
            previous &&
            !renderPlansHaveDomChanges(previous, renderPlan) &&
            !renderPlanHasRuntimeResourceNodes(renderPlan)
        ) {
            this.committedRenderPlans.set(instance, renderPlan);
            return Promise.resolve();
        }

        const mergeOptions = {
            preserveElementChildren: (current: Element) =>
                this.declarations.has(current.localName) && directDataIsland(current) !== undefined,
            transientElementTags: ['module-url', 'http-request', 'local-storage'],
        };
        if (previous) {
            const bounds = this.ensureRenderBounds(instance, island);
            const result = applyRenderPlanToRange(bounds, renderPlan, instance.ownerDocument, mergeOptions);
            this.recordDiagnostics(
                instance,
                result.diagnostics.map((diagnostic) => renderPlanApplyDiagnostic(diagnostic, compiled.producedTag))
            );
            this.bindRenderedSliceEventsInRange(instance, compiled, bounds);
            const resourcesSettled = this.bindRenderedResourceSlicesInRange(instance, compiled, bounds, token);
            this.committedRenderPlans.set(instance, renderPlan);
            return resourcesSettled;
        }

        const fragment = materializeRenderPlan(renderPlan, instance.ownerDocument);
        this.bindRenderedSliceEvents(instance, compiled, fragment);
        const resourcesSettled = this.bindRenderedResourceSlices(instance, compiled, fragment, token);
        this.replaceRenderedContent(instance, island, fragment);
        this.committedRenderPlans.set(instance, renderPlan);
        return resourcesSettled;
    }

    private replaceRenderedContent(instance: HTMLElement, island: HTMLTemplateElement, rendered: DocumentFragment): void {
        const bounds = this.ensureRenderBounds(instance, island);
        let current = bounds.start.nextSibling;
        while (current && current !== bounds.end) {
            const next = current.nextSibling;
            current.parentNode?.removeChild(current);
            current = next;
        }
        instance.insertBefore(rendered, bounds.end);
    }

    private ensureRenderBounds(instance: HTMLElement, island: HTMLTemplateElement): RenderBounds {
        const existing = this.renderBounds.get(instance);
        if (existing?.start.parentNode === instance && existing.end.parentNode === instance) {
            return existing;
        }

        const start = instance.ownerDocument.createComment('cem-render-start');
        const end = instance.ownerDocument.createComment('cem-render-end');
        const insertBefore = island.nextSibling;
        instance.insertBefore(start, insertBefore);
        instance.insertBefore(end, insertBefore);
        const bounds = { start, end };
        this.renderBounds.set(instance, bounds);
        return bounds;
    }

    private createSnapshot(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        island: HTMLTemplateElement
    ): DataIslandSnapshot {
        return {
            version: SNAPSHOT_SCHEMA_VERSION,
            instanceId: this.instanceId(instance),
            producedTag: compiled.producedTag,
            declarationTag: compiled.declarationTag,
            templateArtifactId: compiled.artifactId,
            dataRevision: this.nextDataRevision(instance),
            outputTarget: 'light-dom',
            sourceMapMode: 'dev',
            scopePolicyStamp: this.scopePolicyStamp,
            privacyPolicyStamp: this.privacyPolicyStamp,
            hostAttributes: hostAttributes(instance),
            dataset: datasetEntries(instance),
            payload: serializePayload(island),
            slices: { ...(this.instanceStates.get(instance)?.slices ?? {}) },
            validationState: {},
            eventPayloads: { ...(this.instanceStates.get(instance)?.eventPayloads ?? {}) },
        };
    }

    private instanceId(instance: HTMLElement): string {
        const existing = this.instanceIds.get(instance);
        if (existing) {
            return existing;
        }
        this.instanceSequence += 1;
        const id = `cem-instance-${this.instanceSequence}`;
        this.instanceIds.set(instance, id);
        return id;
    }

    private nextDataRevision(instance: HTMLElement): string {
        const revision = (this.dataRevisions.get(instance) ?? 0) + 1;
        this.dataRevisions.set(instance, revision);
        return String(revision);
    }

    private declarationForInstance(instance: HTMLElement): CompiledDeclaration | undefined {
        return this.declarations.get(instance.localName);
    }

    private validateGeneratedScopeUid(compiled: CompiledDeclaration): boolean {
        if (!this.validateGeneratedIds) {
            return true;
        }
        const owner = this.generatedScopeOwners.get(compiled.scopeUid);
        if (owner && owner !== compiled.declarationElement) {
            this.recordDiagnostics(compiled.declarationElement, [
                declarationDiagnostic(
                    'cem-element.scope_uid_duplicate',
                    `generated scope UID \`${compiled.scopeUid}\` is already used in this runtime output scope`,
                    compiled.producedTag
                ),
            ]);
            return false;
        }
        this.generatedScopeOwners.set(compiled.scopeUid, compiled.declarationElement);
        return true;
    }

    private recordDiagnostics(target: object, diagnostics: CemElementDiagnostic[]): void {
        if (diagnostics.length === 0) {
            return;
        }
        const current = this.diagnostics.get(target) ?? [];
        current.push(...diagnostics);
        this.diagnostics.set(target, current);
        for (const diagnostic of diagnostics) {
            if (diagnostic.severity === 'fatal' || diagnostic.severity === 'error') {
                this.logger?.error?.(diagnostic.message);
            } else {
                this.logger?.warn?.(diagnostic.message);
            }
        }
    }
}

function analyzeDeclarationElement(element: HTMLElement): DeclarationShapeResult {
    return analyzeDeclarationShape({
        tag: element.getAttribute('tag'),
        src: element.getAttribute('src'),
        directTemplateCount: directTemplateChildren(element).length,
        directLiveNodeCount: directLiveNodeCount(element),
    });
}

function compileInlineDeclaration(
    declarationElement: HTMLElement,
    producedTag: string,
    template: HTMLTemplateElement,
    options: InlineDeclarationCompileOptions
): CompiledDeclaration {
    const mode = templateMode(template);
    const diagnostics: CemElementDiagnostic[] = [];

    const templateSource = readInlineTemplateSource(template, mode);
    // DOM-parity templates extract their declarations here for the synchronous projection path.
    // CEM-ML and legacy-XSLT templates render through the cem_ql WASM boundary,
    // which owns declared attributes/defaults (seed_declaration_defaults binds even unset ones).
    const scanDeclarations = mode === 'dom';
    const declaredAttributes = scanDeclarations ? extractAttributeDeclarationsFromSource(templateSource) : [];
    const declaredSlices = scanDeclarations ? extractSliceDeclarationsFromSource(templateSource) : [];

    // Legacy HTML+XSLT templates are transpiled to canonical CEM-ML by the CEM-owned engine
    // (`convertLegacyTemplate`, cem_ml via the cem_ql WASM module) and then ride the same WASM render
    // path as hand-migrated templates. The conversion is async (WASM), so the raw markup is retained
    // and lowered lazily on first render (see {@link renderViaWasm}) — `cemMlSource` starts null.
    const cemMlSource = mode === 'cem-ml' ? templateSourceText(template) : null;
    const legacySource =
        mode === 'legacy-xslt' ? (template.innerHTML.trim().length > 0 ? template.innerHTML : templateSourceText(template)) : null;
    const wasmEligible = mode === 'cem-ml' || mode === 'legacy-xslt';
    const occurrencePath = declarationOccurrencePath(declarationElement);
    const sourceText = sourceTextForUidSeed(template, mode, cemMlSource, legacySource);
    const sourceHash = sourceHashSeedDigest({
        declarationTag: options.declarationTag,
        producedTag,
        mode,
        sourceText,
    });
    const uidSeedResolution = resolveDeclarationUidSeed({
        declarationElement,
        declarationTag: options.declarationTag,
        producedTag,
        template,
        mode,
        occurrencePath,
        sourceText,
        sourceHash,
    }, options);
    return {
        declarationElement,
        declarationTag: options.declarationTag,
        producedTag,
        uidSeed: uidSeedResolution.seed,
        uidSeedSource: uidSeedResolution.source,
        occurrencePath,
        sourceHash,
        scopeUid: generateScopeUid({ producedTag, uidSeed: uidSeedResolution.seed, occurrencePath }),
        artifactId: `template-artifact-${++artifactSequence}`,
        template,
        templateSource,
        mode,
        cemMlSource,
        legacySource,
        wasmEligible,
        declaredAttributes,
        declaredSlices,
        diagnostics,
    };
}

interface InlineDeclarationCompileOptions {
    declarationTag: string;
    uidSeed?: CemElementRuntimeOptions['uidSeed'];
    uidSeedFallback: NonNullable<CemElementRuntimeOptions['uidSeedFallback']>;
}

interface ResolvedUidSeed {
    seed: string | null;
    source: CompiledDeclaration['uidSeedSource'];
}

function resolveDeclarationUidSeed(
    input: CemElementUidSeedInput,
    options: InlineDeclarationCompileOptions
): ResolvedUidSeed {
    if (input.declarationElement.hasAttribute(UID_SEED_ATTR)) {
        return {
            seed: input.declarationElement.getAttribute(UID_SEED_ATTR) ?? '',
            source: 'declaration',
        };
    }

    const hostSeed = resolveHostUidSeed(input, options.uidSeed);
    if (hostSeed !== null) {
        return {
            seed: hostSeed,
            source: 'host',
        };
    }

    if (options.uidSeedFallback === 'source-hash') {
        return {
            seed: `source-${input.sourceHash}`,
            source: 'source-hash',
        };
    }

    return {
        seed: null,
        source: 'runtime',
    };
}

function resolveHostUidSeed(
    input: CemElementUidSeedInput,
    option: CemElementRuntimeOptions['uidSeed']
): string | null {
    if (option === undefined) {
        return null;
    }
    const value = typeof option === 'function' ? option(input) : option;
    return value === undefined || value === null ? null : value;
}

function sourceTextForUidSeed(
    template: HTMLTemplateElement,
    mode: CompiledDeclaration['mode'],
    cemMlSource: string | null,
    legacySource: string | null
): string {
    if (mode === 'cem-ml') {
        return cemMlSource ?? '';
    }
    if (mode === 'legacy-xslt') {
        return legacySource ?? '';
    }
    return template.innerHTML;
}

function sourceHashSeedDigest(input: {
    declarationTag: string;
    producedTag: string;
    mode: CompiledDeclaration['mode'];
    sourceText: string;
}): string {
    return edgeContentAddress('template-artifact', input).digest;
}

/**
 * Read the synchronous template source for a declaration. DOM-parity templates lower
 * through the browser DOM parser into a serializable source tree. CEM-ML templates render
 * through the cem_ql WASM boundary — which owns parsing, declaration metadata, defaults,
 * and diagnostics — so no synchronous source is read for them.
 */
function readInlineTemplateSource(
    template: HTMLTemplateElement,
    mode: CompiledDeclaration['mode']
): TemplateSourceNode[] {
    // Legacy-XSLT templates are parsed + lowered by the engine from raw markup (see
    // compileInlineDeclaration), so no synchronous source tree is read for them here.
    return mode === 'dom' ? readTemplateSource(template.content) : [];
}

function templateMode(template: HTMLTemplateElement): CompiledDeclaration['mode'] {
    const type = template.getAttribute('type');
    if (type === 'text/cem-ml' || type === 'application/cem-ml') {
        return 'cem-ml';
    }
    if (
        template.getAttribute('lang') === 'custom-element-v0' ||
        template.getAttribute('lang') === LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG ||
        containsLegacyXsltConstructs(template)
    ) {
        return 'legacy-xslt';
    }
    const source = templateSourceText(template).trim();
    if (source.startsWith('@doc') || source.startsWith('{')) {
        return 'cem-ml';
    }
    return 'dom';
}

/**
 * Detect whether an untyped template is authored as legacy HTML+XSLT: the `xsl:` namespace prefix or
 * the bare XSLT control-flow spellings (`for-each`/`value-of`/`choose`/`when`/`otherwise`/`variable`,
 * and `<if>`). These tags do not exist in HTML, so their presence unambiguously marks the legacy
 * dialect. Explicit CEM-ML templates are decided first; `custom-element-v0` is accepted as a
 * deprecated alias for the shared engine-backed legacy-XSLT adapter.
 */
function containsLegacyXsltConstructs(template: HTMLTemplateElement): boolean {
    const raw = (template.innerHTML || templateSourceText(template)).toLowerCase();
    return (
        raw.includes('<xsl:') ||
        /<\/?(?:for-each|value-of|choose|when|otherwise|variable)[\s/>]/.test(raw) ||
        /<if[\s/>]/.test(raw)
    );
}

/**
 * The raw CEM-ML source text of a template. Inline templates carry it as set `textContent`;
 * templates parsed via the DOM/DOMParser (e.g. external `src` documents) hold it in
 * `.content`, where `textContent` is empty.
 */
function templateSourceText(template: HTMLTemplateElement): string {
    const content = template.content.textContent ?? '';
    return content.length > 0 ? content : template.textContent ?? '';
}

function extractAttributeDeclarationsFromSource(source: readonly TemplateSourceNode[]): AttributeDeclaration[] {
    const declarations: AttributeDeclaration[] = [];
    for (const child of source) {
        if (child.kind !== 'element' || child.tag !== 'attribute') {
            continue;
        }
        const name = child.attributes.find((attribute) => attribute.name === 'name')?.value.trim();
        if (!name) {
            continue;
        }
        const text = child.children
            .map((node) => (node.kind === 'text' ? node.text : ''))
            .join('')
            .trim();
        declarations.push({
            name,
            defaultValue: text.length > 0 ? text : null,
        });
    }
    return declarations;
}

function extractSliceDeclarationsFromSource(source: readonly TemplateSourceNode[]): SliceDeclaration[] {
    const declarations: SliceDeclaration[] = [];
    for (const child of source) {
        if (child.kind !== 'element' || child.tag !== 'slice') {
            continue;
        }
        const name = child.attributes.find((attribute) => attribute.name === 'name')?.value.trim();
        if (!name) {
            continue;
        }
        const text = child.children
            .map((node) => (node.kind === 'text' ? node.text : ''))
            .join('')
            .trim();
        declarations.push({
            name,
            defaultValue: parseLiteralValue(text),
        });
    }
    return declarations;
}

function directTemplateChildren(element: Element): HTMLTemplateElement[] {
    return Array.from(element.children).filter(
        (child): child is HTMLTemplateElement => child.localName === 'template'
    );
}

function declarationOccurrencePath(element: Element): string {
    const indexes: number[] = [];
    let current: Element | null = element;
    while (current) {
        const parent: Element | null = current.parentElement;
        if (!parent) {
            indexes.unshift(0);
            break;
        }
        indexes.unshift(Array.from(parent.children).indexOf(current));
        current = parent;
    }
    return indexes.join('.');
}

function implicitCemMlTemplate(element: HTMLElement): HTMLTemplateElement {
    const template = element.ownerDocument.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    while (element.firstChild) {
        template.content.appendChild(element.firstChild);
    }
    element.appendChild(template);
    return template;
}

interface SrcReference {
    local: boolean;
    path: string;
    id: string;
}

/**
 * Split a declaration `src` into its document path and fragment id. A reference with an
 * empty path (`src="#id"`) targets the same document; anything else (`./file.html#tag`)
 * is an external reference.
 */
function parseSrcReference(src: string): SrcReference {
    const hashIndex = src.indexOf('#');
    if (hashIndex < 0) {
        return { local: false, path: src, id: '' };
    }
    const path = src.slice(0, hashIndex);
    return { local: path === '', path, id: src.slice(hashIndex + 1) };
}

/**
 * Default external `src` loader: resolve the path against the declaring document's base URL
 * and `fetch` it. Bare module specifiers (`@scope/pkg`) require a host `loadSrcDocument`
 * (the shared module-map resolver).
 */
function defaultLoadSrcDocument(path: string, baseDocument: Document): Promise<string> {
    let url: string;
    try {
        url = new URL(path, baseDocument.baseURI).href;
    } catch {
        return Promise.reject(
            new Error(`cannot resolve \`${path}\`; bare module specifiers need a host \`loadSrcDocument\``)
        );
    }
    return fetch(url).then((response) => {
        if (!response.ok) {
            throw new Error(`HTTP ${response.status} for ${url}`);
        }
        return response.text();
    });
}

function defaultResolveModuleUrl(specifier: string, baseDocument: Document): string {
    const trimmed = specifier.trim();
    if (trimmed === '') {
        return '';
    }
    if (isUrlLikeSpecifier(trimmed)) {
        return new URL(trimmed, baseDocument.baseURI).href;
    }
    const importMeta = import.meta as ImportMeta & { resolve?: (specifier: string) => string };
    if (typeof importMeta.resolve === 'function') {
        return importMeta.resolve(trimmed);
    }
    throw new Error(`cannot resolve \`${specifier}\`; bare module specifiers need a host \`resolveModuleUrl\``);
}

function readHttpRequestDeclaration(element: Element): HttpRequestDeclaration | null {
    const sliceName = element.getAttribute('slice')?.trim();
    const authoredUrl = element.getAttribute('url')?.trim();
    if (!sliceName || !authoredUrl) {
        return null;
    }
    return {
        sliceName,
        authoredUrl,
        method: (element.getAttribute('method')?.trim() || 'GET').toUpperCase(),
        headers: httpRequestHeaders(element),
        expectedContentType: optionalAttribute(element, 'content-type'),
        credentials: optionalAttribute(element, 'credentials'),
        cache: optionalAttribute(element, 'cache'),
    };
}

function readLocalStorageDeclaration(element: Element): LocalStorageDeclaration | null {
    const sliceName = element.getAttribute('slice')?.trim();
    const key = element.getAttribute('key')?.trim();
    if (!sliceName || !key) {
        return null;
    }
    return {
        sliceName,
        key,
        storageType: element.getAttribute('type')?.trim() || 'text',
        live: booleanAttribute(element, 'live'),
        initialValue: element.hasAttribute('value') ? element.getAttribute('value') ?? '' : undefined,
    };
}

function httpRequestHeaders(element: Element): Record<string, string> {
    const headers: Record<string, string> = {};
    for (const attribute of Array.from(element.attributes)) {
        if (attribute.name.startsWith('header-')) {
            const headerName = attribute.name.slice('header-'.length).trim().toLowerCase();
            if (headerName) {
                headers[headerName] = attribute.value;
            }
        }
    }
    return headers;
}

function booleanAttribute(element: Element, name: string): boolean {
    if (!element.hasAttribute(name)) {
        return false;
    }
    const value = element.getAttribute(name)?.trim().toLowerCase();
    return value !== 'false' && value !== '0';
}

function optionalAttribute(element: Element, name: string): string | undefined {
    const value = element.getAttribute(name)?.trim();
    return value && value.length > 0 ? value : undefined;
}

type LocalStorageSliceSource = 'initial-read' | 'value-attribute' | 'slice-write' | 'storage-event' | 'retained';

function sameLocalStorageDeclaration(active: ActiveLocalStorageResource, declaration: LocalStorageDeclaration): boolean {
    return active.key === declaration.key && active.storageType === declaration.storageType && active.live === declaration.live;
}

function localStorageForWindow(window: Window | null | undefined): Storage | null {
    if (!window) {
        return null;
    }
    try {
        return window.localStorage;
    } catch {
        return null;
    }
}

function ensureTrackedLocalStorage(window: Window): void {
    if (localStorageTrackers.has(window)) {
        return;
    }
    const storage = localStorageForWindow(window);
    if (!storage) {
        return;
    }
    const originalSetItem = storage.setItem;
    const originalRemoveItem = storage.removeItem;
    const originalClear = storage.clear;
    try {
        storage.setItem = function setItem(key: string, value: string): void {
            originalSetItem.call(storage, key, value);
            dispatchLocalStorageChange(window, String(key), String(value));
        };
        storage.removeItem = function removeItem(key: string): void {
            originalRemoveItem.call(storage, key);
            dispatchLocalStorageChange(window, String(key), null);
        };
        storage.clear = function clear(): void {
            originalClear.call(storage);
            dispatchLocalStorageChange(window, null, null);
        };
        localStorageTrackers.add(window);
    } catch {
        // Some hosts expose immutable Storage methods; cross-document `storage` events still work.
    }
}

function dispatchLocalStorageChange(window: Window, key: string | null, value: string | null): void {
    window.dispatchEvent(new CustomEvent(LOCAL_STORAGE_EVENT, { detail: { key, value } }));
}

function localStorageChangedKey(event: Event): string | null {
    if (event.type === 'storage') {
        return (event as StorageEvent).key;
    }
    const detail = (event as CustomEvent<{ key?: string | null }>).detail;
    return detail?.key ?? null;
}

function localStorageStringToValue(type: string, rawValue: string | null, document: Document): unknown {
    const storageType = type || 'text';
    if (rawValue === null) {
        return null;
    }
    if (storageType === 'text') {
        return rawValue;
    }
    if (storageType === 'json') {
        try {
            return JSON.parse(rawValue) as unknown;
        } catch {
            return null;
        }
    }
    const input = document.createElement('input');
    input.setAttribute('type', storageType);
    if (storageType === 'number') {
        input.value = rawValue;
        return Number.isNaN(input.valueAsNumber) ? null : input.valueAsNumber;
    }
    if (storageType === 'date') {
        const date = new Date(rawValue);
        if (Number.isNaN(date.getTime())) {
            return null;
        }
        input.valueAsDate = date;
        return input.value || null;
    }
    input.value = rawValue;
    return input.value || null;
}

function localStorageValueToString(type: string, value: unknown): string | null {
    if (value === undefined || value === null) {
        return null;
    }
    if (type === 'json') {
        try {
            return JSON.stringify(value);
        } catch {
            return null;
        }
    }
    if (type === 'number') {
        const number = typeof value === 'number' ? value : Number(value);
        return Number.isNaN(number) ? null : String(number);
    }
    return String(value);
}

function writeLocalStorageRaw(storage: Storage, key: string, rawValue: string | null): void {
    if (rawValue === null) {
        storage.removeItem(key);
    } else {
        storage.setItem(key, rawValue);
    }
}

function localStorageValuesEqual(left: unknown, right: unknown): boolean {
    if (Object.is(left, right)) {
        return true;
    }
    if (isPlainRecord(left) || Array.isArray(left) || isPlainRecord(right) || Array.isArray(right)) {
        return stableJson(left) === stableJson(right);
    }
    return false;
}

function defaultResolveResourceUrl(
    request: CemResourceResolutionRequest,
    baseDocument: Document,
    resourcePolicyStamp: string,
    policy: CemHttpResourcePolicy
): CemResourceResolution {
    const trimmed = request.authoredUrl.trim();
    if (!isUrlLikeSpecifier(trimmed)) {
        throw new Error(`cannot resolve \`${request.authoredUrl}\`; bare resource specifiers need a host resolver`);
    }
    const resolvedUrl = new URL(trimmed, baseDocument.baseURI).href;
    if (!policy.allowCrossOrigin) {
        const baseOrigin = new URL(baseDocument.baseURI).origin;
        const resolvedOrigin = new URL(resolvedUrl).origin;
        if (baseOrigin !== resolvedOrigin) {
            throw new Error(`cross-origin http-request \`${request.authoredUrl}\` requires host policy`);
        }
    }
    return {
        authoredUrl: request.authoredUrl,
        resolvedUrl,
        resolverIdentity: `document-base:${baseDocument.baseURI}`,
        resourcePolicyStamp,
    };
}

function unresolvedHttpRequestMetadata(
    declaration: HttpRequestDeclaration,
    resourcePolicyStamp: string
): CemHttpResourceEnvelope['request'] {
    return {
        authoredUrl: declaration.authoredUrl,
        url: declaration.authoredUrl,
        resolvedUrl: declaration.authoredUrl,
        resolverIdentity: 'unresolved',
        resourcePolicyStamp,
        method: declaration.method,
        headers: declaration.headers,
    };
}

function httpRequestMetadata(request: CemHttpRequest): CemHttpResourceEnvelope['request'] {
    return {
        authoredUrl: request.authoredUrl,
        url: request.resolvedUrl,
        resolvedUrl: request.resolvedUrl,
        resolverIdentity: request.resolverIdentity,
        resourcePolicyStamp: request.resourcePolicyStamp,
        method: request.method,
        headers: { ...request.headers },
    };
}

async function defaultLoadHttpResource(request: CemHttpRequest): Promise<CemHttpResourceLoadResult> {
    if (!request.policy.allowCrossOrigin) {
        const baseOrigin = new URL(request.baseUrl).origin;
        const resolvedOrigin = new URL(request.resolvedUrl).origin;
        if (baseOrigin !== resolvedOrigin) {
            throw new Error(`cross-origin http-request \`${request.resolvedUrl}\` requires host policy`);
        }
    }
    const response = await fetch(request.resolvedUrl, {
        method: request.method,
        headers: request.headers,
        credentials: request.credentials as RequestCredentials | undefined,
        cache: request.cache as RequestCache | undefined,
        redirect: request.policy.redirect,
        signal: request.signal,
    });
    if (!response.ok) {
        throw new Error(`HTTP ${response.status} for ${request.resolvedUrl}`);
    }
    return {
        response: {
            url: response.url,
            status: response.status,
            statusText: response.statusText,
            ok: response.ok,
            redirected: response.redirected,
            headers: headersToRecord(response.headers),
            contentType: response.headers.get('content-type'),
        },
        body: responseBody(response),
    };
}

function headersToRecord(headers: Headers): Record<string, string> {
    const record: Record<string, string> = {};
    headers.forEach((value, name) => {
        record[name.toLowerCase()] = value;
    });
    return record;
}

async function* responseBody(response: Response): AsyncIterable<Uint8Array> {
    if (response.body) {
        const reader = response.body.getReader();
        try {
            for (;;) {
                const chunk = await reader.read();
                if (chunk.done) {
                    break;
                }
                yield chunk.value;
            }
        } finally {
            reader.releaseLock();
        }
        return;
    }
    yield new Uint8Array(await response.arrayBuffer());
}

async function parseHttpResourceData(
    request: CemHttpRequest,
    response: CemHttpResponseHead,
    body: AsyncIterable<Uint8Array>,
    expectedContentType: string | undefined,
    maxResponseBytes: number,
    signal: AbortSignal,
    tag: string
): Promise<{ ok: boolean; data: unknown; diagnostics: CemElementDiagnostic[]; sourceId: CemHttpResourceSourceId }> {
    const contentType = recognizedContentType(response.contentType, expectedContentType);
    const fallbackSourceId = httpResourceSourceId(request, response, contentType.ok ? contentType.contentType : null);
    if (!contentType.ok) {
        return {
            ok: false,
            data: null,
            sourceId: fallbackSourceId,
            diagnostics: [
                resourceDiagnostic(
                    'cem-element.http_request_unsupported_content_type',
                    contentType.message,
                    tag,
                    'error',
                    httpSourceMapRef(fallbackSourceId)
                ),
            ],
        };
    }
    const bytes = await readByteStream(body, maxResponseBytes, signal);
    const text = new TextDecoder('utf-8').decode(bytes);
    const sourceId = httpResourceSourceId(request, response, contentType.contentType, text);
    if (contentType.kind === 'json') {
        try {
            return { ok: true, data: JSON.parse(text) as unknown, diagnostics: [], sourceId };
        } catch (error) {
            return {
                ok: false,
                data: null,
                sourceId,
                diagnostics: [
                    resourceDiagnostic(
                        'cem-element.http_request_parse_failed',
                        `JSON response could not be parsed: ${
                            error instanceof Error ? error.message : String(error)
                        }`,
                        tag,
                        'error',
                        httpSourceMapRef(sourceId)
                    ),
                ],
            };
        }
    }
    if (contentType.kind === 'xml') {
        return parseXmlHttpResourceData(text, contentType.contentType, sourceId, tag);
    }
    return { ok: true, data: { text }, diagnostics: [], sourceId };
}

function recognizedContentType(
    responseContentType: string | null,
    expectedContentType: string | undefined
):
    | { ok: true; kind: 'json' | 'xml' | 'text'; contentType: string }
    | { ok: false; message: string } {
    const contentType = mediaType(responseContentType) ?? mediaType(expectedContentType);
    if (!contentType) {
        return { ok: false, message: 'http-request response did not provide a Content-Type' };
    }
    if (contentType === 'application/json' || contentType === 'text/json' || contentType.endsWith('+json')) {
        return { ok: true, kind: 'json', contentType };
    }
    if (
        contentType === 'application/xml' ||
        contentType === 'text/xml' ||
        contentType === 'application/xhtml+xml' ||
        contentType.endsWith('+xml')
    ) {
        return { ok: true, kind: 'xml', contentType };
    }
    if (contentType === 'text/plain') {
        return { ok: true, kind: 'text', contentType };
    }
    return { ok: false, message: `unsupported http-request content type \`${contentType}\`` };
}

function mediaType(contentType: string | null | undefined): string | null {
    const trimmed = contentType?.split(';', 1)[0]?.trim().toLowerCase() ?? '';
    return trimmed.length > 0 ? trimmed : null;
}

function parseXmlHttpResourceData(
    text: string,
    contentType: string,
    sourceId: CemHttpResourceSourceId,
    tag: string
): { ok: boolean; data: unknown; diagnostics: CemElementDiagnostic[]; sourceId: CemHttpResourceSourceId } {
    const parser = new DOMParser();
    const parsed = parser.parseFromString(text, xmlDomParserContentType(contentType));
    const parserError = parsed.getElementsByTagName('parsererror')[0];
    if (parserError) {
        return {
            ok: false,
            data: null,
            sourceId,
            diagnostics: [
                resourceDiagnostic(
                    'cem-element.http_request_parse_failed',
                    normalizeTextContent(parserError.textContent ?? 'XML response could not be parsed'),
                    tag,
                    'error',
                    httpSourceMapRef(sourceId)
                ),
            ],
        };
    }
    if (!parsed.documentElement) {
        return {
            ok: false,
            data: null,
            sourceId,
            diagnostics: [
                resourceDiagnostic(
                    'cem-element.http_request_parse_failed',
                    'XML response did not contain a document element',
                    tag,
                    'error',
                    httpSourceMapRef(sourceId)
                ),
            ],
        };
    }
    return { ok: true, data: xmlElementToRecord(parsed.documentElement), diagnostics: [], sourceId };
}

function xmlDomParserContentType(contentType: string): DOMParserSupportedType {
    return contentType === 'application/xhtml+xml' ? 'application/xhtml+xml' : 'application/xml';
}

function xmlElementToRecord(element: Element): {
    tag: string;
    namespace: string | null;
    attributes: Record<string, string>;
    text: string;
    children: ReturnType<typeof xmlElementToRecord>[];
} {
    const attributes: Record<string, string> = {};
    for (const attribute of Array.from(element.attributes)) {
        attributes[attribute.name] = attribute.value;
    }
    return {
        tag: element.localName,
        namespace: element.namespaceURI,
        attributes,
        text: normalizeTextContent(element.textContent ?? ''),
        children: Array.from(element.children).map(xmlElementToRecord),
    };
}

function normalizeTextContent(value: string): string {
    return value
        .split(/\s+/)
        .filter((part) => part.length > 0)
        .join(' ');
}

function httpResourceSourceId(
    request: CemHttpRequest,
    response: CemHttpResponseHead,
    contentType: string | null,
    bodyText?: string
): CemHttpResourceSourceId {
    const responseIdentityHash =
        bodyText === undefined
            ? undefined
            : edgeContentAddress('sanitized-snapshot', {
                  url: response.url,
                  status: response.status,
                  contentType,
                  bodyText,
              }).digest;
    const id = edgeContentAddress('sanitized-snapshot', {
        authoredUrl: request.authoredUrl,
        resolvedUrl: request.resolvedUrl,
        finalUrl: response.url,
        resolverIdentity: request.resolverIdentity,
        resourcePolicyStamp: request.resourcePolicyStamp,
        method: request.method,
        contentType,
        responseIdentityHash,
    }).digest;
    return {
        kind: 'http-response',
        id: `http-source-${id}`,
        authoredUrl: request.authoredUrl,
        resolvedUrl: request.resolvedUrl,
        finalUrl: response.url,
        resolverIdentity: request.resolverIdentity,
        resourcePolicyStamp: request.resourcePolicyStamp,
        method: request.method,
        contentType,
        responseIdentityHash,
        redacted: false,
    };
}

function httpSourceMapRef(sourceId: CemHttpResourceSourceId): SourceMapRef {
    return { fidelity: 'declaration-only', frame: `http:${sourceId.id}` };
}

async function readByteStream(
    body: AsyncIterable<Uint8Array>,
    maxResponseBytes: number,
    signal: AbortSignal
): Promise<Uint8Array> {
    const chunks: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of body) {
        if (signal.aborted) {
            throw new HttpResourceError('cem-element.http_request_aborted', 'http-request was aborted');
        }
        chunks.push(chunk);
        size += chunk.byteLength;
        if (size > maxResponseBytes) {
            throw new HttpResourceError(
                'cem-element.http_request_response_too_large',
                `http-request response exceeded ${maxResponseBytes} bytes`
            );
        }
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }
    return bytes;
}

function httpRequestCacheKey(input: {
    baseUrl: string;
    scope: string;
    declaration: HttpRequestDeclaration;
    policy: CemHttpResourcePolicy;
}): string {
    return stableJson({
        baseUrl: input.baseUrl,
        scope: input.scope,
        url: input.declaration.authoredUrl,
        method: input.declaration.method,
        headers: input.declaration.headers,
        contentType: input.declaration.expectedContentType,
        credentials: input.declaration.credentials,
        cache: input.declaration.cache,
        policy: input.policy,
    });
}

function stableJson(value: unknown): string {
    if (Array.isArray(value)) {
        return `[${value.map(stableJson).join(',')}]`;
    }
    if (value && typeof value === 'object') {
        const record = value as Record<string, unknown>;
        return `{${Object.keys(record)
            .sort()
            .map((key) => `${JSON.stringify(key)}:${stableJson(record[key])}`)
            .join(',')}}`;
    }
    return JSON.stringify(value);
}

class HttpResourceError extends Error {
    readonly code: string;

    constructor(code: string, message: string) {
        super(message);
        this.name = 'HttpResourceError';
        this.code = code;
    }
}

function isUrlLikeSpecifier(specifier: string): boolean {
    return (
        specifier.startsWith('.') ||
        specifier.startsWith('/') ||
        specifier.startsWith('#') ||
        /^[A-Za-z][A-Za-z0-9+.-]*:/.test(specifier)
    );
}

/** The `<template>` a local `src` reference loads: the target itself, or its first template child. */
function templateFromTarget(target: Element | null): HTMLTemplateElement | undefined {
    if (!target) {
        return undefined;
    }
    if (target.localName === 'template') {
        return target as HTMLTemplateElement;
    }
    return directTemplateChildren(target)[0];
}

function directDataIsland(element: Element): HTMLTemplateElement | undefined {
    return Array.from(element.children).find(
        (child): child is HTMLTemplateElement =>
            child.localName === 'template' && child.getAttribute(DATA_ISLAND_ATTR) === DATA_ISLAND_VALUE
    );
}

function directHydrationMetadata(element: Element): HTMLScriptElement | undefined {
    return Array.from(element.children).find(
        (child): child is HTMLScriptElement =>
            child.localName === 'script' &&
            child.getAttribute('type') === 'application/json' &&
            child.getAttribute(HYDRATION_METADATA_ATTR) === HYDRATION_METADATA_VALUE
    );
}

function directRenderBounds(element: Element): RenderBounds | undefined {
    let start: Comment | undefined;
    for (const child of Array.from(element.childNodes)) {
        if (isRenderStartBoundary(child)) {
            start = child;
            continue;
        }
        if (start && isRenderEndBoundary(child)) {
            return { start, end: child };
        }
    }
    return undefined;
}

type HydrationSnapshotParseResult =
    | { ok: true; snapshot: DataIslandSnapshot }
    | { ok: false; code: string; message: string };

function parseHydrationSnapshot(metadata: HTMLScriptElement): HydrationSnapshotParseResult {
    const raw = metadata.textContent ?? '';
    if (raw.trim().length === 0) {
        return {
            ok: false,
            code: 'cem-element.hydration_snapshot_missing',
            message: 'SSR hydration metadata did not contain a serialized DataIslandSnapshot',
        };
    }
    try {
        const value: unknown = JSON.parse(raw);
        if (!isDataIslandSnapshot(value)) {
            return {
                ok: false,
                code: 'cem-element.hydration_snapshot_invalid',
                message: 'SSR hydration metadata JSON was not a valid DataIslandSnapshot',
            };
        }
        return { ok: true, snapshot: value };
    } catch (error) {
        return {
            ok: false,
            code: 'cem-element.hydration_json_invalid',
            message: `SSR hydration metadata JSON could not be parsed: ${
                error instanceof Error ? error.message : String(error)
            }`,
        };
    }
}

function hydrationRenderIdentityDiagnostics(
    instance: HTMLElement,
    bounds: RenderBounds,
    snapshot: DataIslandSnapshot
): CemElementDiagnostic[] {
    const firstRenderedElement = firstRenderedElementBetween(bounds);
    if (!firstRenderedElement) {
        return [
            renderDiagnostic(
                'cem-element.hydration_render_plan_missing',
                'SSR hydration render boundaries did not contain a retained render-plan root element',
                instance.localName
            ),
        ];
    }
    const diagnostics: CemElementDiagnostic[] = [];
    const artifactId = firstRenderedElement.getAttribute(RENDER_TEMPLATE_ARTIFACT_ID_ATTR);
    if (!artifactId) {
        diagnostics.push(
            renderDiagnostic(
                'cem-element.hydration_render_plan_identity_missing',
                'SSR hydration retained render root was missing template artifact identity',
                instance.localName
            )
        );
    } else if (artifactId !== snapshot.templateArtifactId) {
        diagnostics.push(
            renderDiagnostic(
                'cem-element.hydration_template_artifact_mismatch',
                `SSR hydration retained template artifact \`${artifactId}\` did not match snapshot artifact \`${snapshot.templateArtifactId}\``,
                instance.localName
            )
        );
    }
    const dataRevision = firstRenderedElement.getAttribute(RENDER_DATA_REVISION_ATTR);
    if (!dataRevision) {
        diagnostics.push(
            renderDiagnostic(
                'cem-element.hydration_render_revision_missing',
                'SSR hydration retained render root was missing data revision identity',
                instance.localName
            )
        );
    } else if (dataRevision !== snapshot.dataRevision) {
        diagnostics.push(
            renderDiagnostic(
                'cem-element.hydration_render_revision_mismatch',
                `SSR hydration retained data revision \`${dataRevision}\` did not match snapshot revision \`${snapshot.dataRevision}\``,
                instance.localName
            )
        );
    }
    const sourceMapModeDiagnostic = hydrationSourceMapModeDiagnostic(instance, firstRenderedElement, snapshot);
    if (sourceMapModeDiagnostic) {
        diagnostics.push(sourceMapModeDiagnostic);
    }
    return diagnostics;
}

function hydrationSourceMapModeDiagnostic(
    instance: HTMLElement,
    firstRenderedElement: Element,
    snapshot: DataIslandSnapshot
): CemElementDiagnostic | undefined {
    if (!snapshot.sourceMapMode) {
        return undefined;
    }
    const retainedFidelity = firstRenderedElement.getAttribute(SOURCE_FIDELITY_ATTR);
    if (snapshot.sourceMapMode === 'dev') {
        if (!isSourceMapFidelity(retainedFidelity)) {
            return renderDiagnostic(
                'cem-element.hydration_source_map_mode_mismatch',
                'SSR hydration snapshot expected dev source metadata but the retained render root did not carry source fidelity',
                instance.localName
            );
        }
        return undefined;
    }
    if (retainedFidelity !== null) {
        return renderDiagnostic(
            'cem-element.hydration_source_map_mode_mismatch',
            'SSR hydration snapshot expected prod source metadata policy but the retained render root carried source fidelity',
            instance.localName
        );
    }
    return undefined;
}

function firstRenderedElementBetween(bounds: RenderBounds): Element | undefined {
    let current = bounds.start.nextSibling;
    while (current && current !== bounds.end) {
        if (current.nodeType === 1) {
            return current as Element;
        }
        current = current.nextSibling;
    }
    return undefined;
}

function renderedElementsBetween(bounds: RenderBounds, selector: string): Element[] {
    const elements: Element[] = [];
    let current = bounds.start.nextSibling;
    while (current && current !== bounds.end) {
        if (current.nodeType === 1) {
            const element = current as Element;
            if (element.matches(selector)) {
                elements.push(element);
            }
            elements.push(...Array.from(element.querySelectorAll(selector)));
        }
        current = current.nextSibling;
    }
    return elements;
}

function isDataIslandSnapshot(value: unknown): value is DataIslandSnapshot {
    if (!value || typeof value !== 'object') {
        return false;
    }
    const record = value as Partial<DataIslandSnapshot>;
    return (
        typeof record.instanceId === 'string' &&
        typeof record.producedTag === 'string' &&
        typeof record.declarationTag === 'string' &&
        typeof record.templateArtifactId === 'string' &&
        typeof record.dataRevision === 'string' &&
        record.outputTarget === 'light-dom' &&
        (record.sourceMapMode === undefined || isSourceMapMode(record.sourceMapMode)) &&
        typeof record.scopePolicyStamp === 'string' &&
        typeof record.privacyPolicyStamp === 'string' &&
        isPlainRecord(record.hostAttributes) &&
        isPlainRecord(record.dataset) &&
        isPlainRecord(record.slices) &&
        isPlainRecord(record.validationState) &&
        isPlainRecord(record.eventPayloads)
    );
}

function isSourceMapMode(value: unknown): value is SourceMapMode {
    return value === 'dev' || value === 'prod';
}

function isSourceMapFidelity(value: unknown): value is SourceMapFidelity {
    return value === 'author-byte-exact' || value === 'dom-canonical' || value === 'declaration-only';
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}

function parseDataRevision(value: string): number {
    const revision = Number.parseInt(value, 10);
    return Number.isFinite(revision) && revision > 0 ? revision : 0;
}

function directLiveNodeCount(element: Element): number {
    return Array.from(element.childNodes).filter((node) => {
        if (node.nodeType === 1) {
            return (node as Element).localName !== 'template';
        }
        if (node.nodeType === 3) {
            return (node.textContent?.trim() ?? '').length > 0;
        }
        return node.nodeType !== 8;
    }).length;
}

function declarationDiagnostic(code: string, message: string, tag?: string): CemElementDiagnostic {
    return {
        code,
        severity: 'error',
        source: 'declaration',
        message,
        tag,
        sourceMapRef: { fidelity: 'declaration-only', frame: `decl:${tag ?? 'unknown'}` },
    };
}

function renderDiagnostic(code: string, message: string, tag?: string): CemElementDiagnostic {
    return {
        code,
        severity: 'error',
        source: 'render',
        message,
        tag,
    };
}

function resourceDiagnostic(
    code: string,
    message: string,
    tag?: string,
    severity: CemElementDiagnosticSeverity = 'warning',
    sourceMapRef?: SourceMapRef
): CemElementDiagnostic {
    return {
        code,
        severity,
        source: 'render',
        message,
        tag,
        sourceMapRef,
    };
}

function scopedCssDiagnostic(diagnostic: ScopedCssRewriteDiagnostic, tag: string): CemElementDiagnostic {
    return {
        code: diagnostic.code,
        severity: diagnostic.severity,
        source: 'render',
        message: diagnostic.message,
        tag,
    };
}

function renderPlanApplyDiagnostic(diagnostic: RenderPlanApplyDiagnostic, tag: string): CemElementDiagnostic {
    return {
        code: diagnostic.code,
        severity: diagnostic.severity,
        source: 'render',
        message: diagnostic.message,
        tag,
    };
}

function renderPlanHasRuntimeResourceNodes(plan: RenderPlan): boolean {
    const visit = (node: RenderPlan['nodes'][number]): boolean => {
        if (node.kind !== 'element') {
            return false;
        }
        return (
            node.tag === 'module-url' ||
            node.tag === 'http-request' ||
            node.tag === 'local-storage' ||
            node.children.some(visit)
        );
    };
    return plan.nodes.some(visit);
}

function templateValues(
    snapshot: DataIslandSnapshot,
    declarations: AttributeDeclaration[]
): Record<string, TemplateValue> {
    const values: Record<string, TemplateValue> = {};
    for (const declaration of declarations) {
        values[declaration.name] = declaration.defaultValue;
    }
    for (const [name, value] of Object.entries(snapshot.hostAttributes)) {
        values[name] = value;
    }
    for (const [name, value] of Object.entries(snapshot.slices)) {
        values[name] = toTemplateValue(value);
    }
    addTemplateValuePaths(values, 'datadom', dataDocumentFromSnapshot(snapshot));
    return values;
}

function wasmTemplateData(snapshot: DataIslandSnapshot, declarations: AttributeDeclaration[]): Record<string, unknown> {
    return {
        ...templateValues(snapshot, declarations),
        datadom: dataDocumentFromSnapshot(snapshot),
    };
}

function dataDocumentFromSnapshot(snapshot: DataIslandSnapshot): Record<string, unknown> {
    const elementsByAttribute = dataDocumentElementsByAttribute(snapshot);
    return {
        attributes: snapshot.hostAttributes,
        dataset: snapshot.dataset,
        elementsByAttribute,
        payload: snapshot.payload,
        slots: snapshot.payload.slots,
        data: snapshot.payload.dataByValue,
        options: snapshot.payload.optionsByValue,
        dataItems: snapshot.payload.data,
        optionItems: snapshot.payload.options,
        slices: snapshot.slices,
        validationState: snapshot.validationState,
        eventPayloads: snapshot.eventPayloads,
    };
}

function dataDocumentElementsByAttribute(
    snapshot: DataIslandSnapshot
): Record<string, SerializedPayloadElement[]> {
    const byAttribute: Record<string, SerializedPayloadElement[]> = {};
    for (const [name, elements] of Object.entries(snapshot.payload.elementsByAttribute)) {
        byAttribute[name] = [...elements];
    }
    const hostElement: SerializedPayloadElement = {
        key: 'host',
        tag: snapshot.producedTag,
        namespace: null,
        text: '',
        attributes: Object.fromEntries(
            Object.entries(snapshot.hostAttributes)
                .filter((entry): entry is [string, string | boolean] => entry[1] !== null)
                .map(([name, value]) => [name, value === true ? '' : value === false ? 'false' : value])
        ),
        slot: '',
    };
    for (const name of Object.keys(hostElement.attributes)) {
        byAttribute[name] = [...(byAttribute[name] ?? []), hostElement];
    }
    return byAttribute;
}

function cloneJsonSnapshotField(value: unknown): unknown {
    return JSON.parse(JSON.stringify(value)) as unknown;
}

function redactedSnapshotField(field: DataIslandSnapshotExportField): unknown {
    return field === 'payload' ? emptySerializedPayload() : {};
}

function emptySerializedPayload(): SerializedPayload {
    return {
        text: '',
        childCount: 0,
        nodes: [],
        slots: {},
        elementsByAttribute: {},
        data: [],
        options: [],
        dataByValue: {},
        optionsByValue: {},
    };
}

function addTemplateValuePaths(values: Record<string, TemplateValue>, prefix: string, value: unknown): void {
    if (
        value === null ||
        typeof value === 'string' ||
        typeof value === 'boolean' ||
        typeof value === 'number' ||
        typeof value === 'undefined'
    ) {
        values[prefix] = toTemplateValue(value);
        return;
    }
    if (Array.isArray(value)) {
        return;
    }
    if (typeof value !== 'object') {
        values[prefix] = toTemplateValue(value);
        return;
    }
    for (const [name, child] of Object.entries(value)) {
        addTemplateValuePaths(values, `${prefix}.${name}`, child);
    }
}

function runtimeSupportDiagnostic(diagnostic: RuntimeSupportDiagnostic, tag: string): CemElementDiagnostic {
    return {
        code: diagnostic.code,
        severity: diagnostic.severity,
        source: 'render',
        message: diagnostic.message,
        tag,
        sourceMapRef: runtimeSupportSourceMapRef(diagnostic, tag),
    };
}

function declarationRuntimeSupportDiagnostic(
    diagnostic: RuntimeSupportDiagnostic,
    tag: string
): CemElementDiagnostic {
    return {
        code: diagnostic.code,
        severity: diagnostic.severity,
        source: 'declaration',
        message: diagnostic.message,
        tag,
        sourceMapRef: runtimeSupportSourceMapRef(diagnostic, tag),
    };
}

function runtimeSupportSourceMapRef(diagnostic: RuntimeSupportDiagnostic, tag: string): SourceMapRef {
    return diagnostic.sourceMapRef ?? { fidelity: 'declaration-only', frame: `decl:${tag}` };
}

function evaluateSliceValue(
    expression: string,
    event: Event,
    slices: Record<string, unknown>
): TemplateValue {
    const body = unwrapExpression(expression);
    const target = event.target;
    if (body === '$event.type') {
        return event.type;
    }
    if (body === '$target.checked') {
        return target instanceof HTMLInputElement ? target.checked : null;
    }
    if (body === '$target.value') {
        return target instanceof HTMLInputElement ||
            target instanceof HTMLTextAreaElement ||
            target instanceof HTMLSelectElement
            ? target.value
            : null;
    }
    if (/^\$[A-Za-z_][\w.-]*$/.test(body)) {
        return toTemplateValue(slices[body.slice(1)]);
    }
    return parseLiteralValue(body);
}

function serializeEventPayload(event: Event, sliceValue: TemplateValue): SerializedEventPayload {
    const payload: SerializedEventPayload = {
        type: event.type,
        bubbles: event.bubbles,
        cancelable: event.cancelable,
        composed: event.composed,
        target: serializeEventTarget(event.target),
        currentTarget: serializeEventTarget(event.currentTarget),
        sliceValue,
    };
    if (event instanceof CustomEvent) {
        const detail = cloneJsonSafe(event.detail);
        if (detail !== undefined) {
            payload.detail = detail;
        }
    }
    return payload;
}

function serializeEventTarget(target: EventTarget | null): SerializedEventTarget | null {
    if (!(target instanceof Element)) {
        return null;
    }
    return {
        tag: target.localName,
        id: target.getAttribute('id'),
        name: target.getAttribute('name'),
        type: target instanceof HTMLInputElement ? target.type : target.getAttribute('type'),
        value:
            target instanceof HTMLInputElement ||
            target instanceof HTMLTextAreaElement ||
            target instanceof HTMLSelectElement
                ? target.value
                : null,
        checked: target instanceof HTMLInputElement ? target.checked : null,
        dataset: target instanceof HTMLElement ? datasetEntries(target) : {},
    };
}

function cloneJsonSafe(value: unknown): unknown {
    if (value === undefined) {
        return undefined;
    }
    try {
        return JSON.parse(JSON.stringify(value)) as unknown;
    } catch {
        return undefined;
    }
}

function unwrapExpression(expression: string): string {
    const trimmed = expression.trim();
    const wrapped = trimmed.match(/^\{\s*(.*?)\s*\}$/);
    return (wrapped?.[1] ?? trimmed).trim();
}

function parseLiteralValue(value: string): TemplateValue {
    const trimmed = value.trim();
    if (trimmed === '') {
        return null;
    }
    if (trimmed === 'true') {
        return true;
    }
    if (trimmed === 'false') {
        return false;
    }
    const quoted = trimmed.match(/^(['"])(.*)\1$/);
    if (quoted) {
        return quoted[2];
    }
    return trimmed;
}

function toTemplateValue(value: unknown): TemplateValue {
    if (value === null || typeof value === 'string' || typeof value === 'boolean') {
        return value;
    }
    if (value === undefined) {
        return null;
    }
    return String(value);
}

function templateValueRecord(value: Record<string, unknown>): Record<string, unknown> {
    return cloneJsonSnapshotField(value) as Record<string, unknown>;
}

function hostAttributes(instance: HTMLElement): Record<string, string | boolean | null> {
    const attributes: Record<string, string | boolean | null> = {};
    for (const attribute of Array.from(instance.attributes)) {
        attributes[attribute.name] = attribute.value === '' ? true : attribute.value;
    }
    return attributes;
}

function datasetEntries(instance: HTMLElement): Record<string, string> {
    const dataset: Record<string, string> = {};
    for (const [key, value] of Object.entries(instance.dataset)) {
        if (value !== undefined) {
            dataset[key] = value;
        }
    }
    return dataset;
}

function serializePayload(island: HTMLTemplateElement): SerializedPayload {
    const nodes = Array.from(island.content.childNodes)
        .map((node, index) => serializePayloadNode(node, String(index)))
        .filter((node): node is SerializedPayloadNode => node !== undefined);
    const slots: Record<string, SerializedPayloadNode[]> = {};
    for (const node of nodes) {
        const slot = payloadSlotName(node);
        if (slot === null) {
            continue;
        }
        slots[slot] = [...(slots[slot] ?? []), node];
    }
    const data = collectPayloadChoices(nodes, 'data');
    const options = collectPayloadChoices(nodes, 'option');
    return {
        text: island.content.textContent ?? '',
        childCount: island.content.childNodes.length,
        nodes,
        slots,
        elementsByAttribute: payloadElementsByAttribute(nodes),
        data,
        options,
        dataByValue: choicesByValue(data),
        optionsByValue: choicesByValue(options),
    };
}

function serializePayloadNode(node: Node, key: string): SerializedPayloadNode | undefined {
    if (node.nodeType === 3) {
        const text = node.textContent ?? '';
        return text.trim().length > 0 ? { kind: 'text', key, text } : undefined;
    }
    if (node.nodeType === 8) {
        return { kind: 'comment', key, text: node.textContent ?? '' };
    }
    if (node.nodeType !== 1) {
        return undefined;
    }

    const element = node as Element;
    return {
        kind: 'element',
        key,
        tag: element.localName,
        namespace: element.namespaceURI === XHTML_NAMESPACE ? null : element.namespaceURI,
        attributes: Object.fromEntries(Array.from(element.attributes).map((attribute) => [attribute.name, attribute.value])),
        slot: element.getAttribute('slot') ?? '',
        children: Array.from(element.childNodes)
            .map((child, index) => serializePayloadNode(child, `${key}/${index}`))
            .filter((child): child is SerializedPayloadNode => child !== undefined),
    };
}

function payloadSlotName(node: SerializedPayloadNode): string | null {
    if (node.kind === 'element') {
        return node.slot;
    }
    if (node.kind === 'text') {
        return '';
    }
    return null;
}

function collectPayloadChoices(
    nodes: readonly SerializedPayloadNode[],
    kind: SerializedPayloadChoice['kind'],
    group: string | null = null
): SerializedPayloadChoice[] {
    const choices: SerializedPayloadChoice[] = [];
    for (const node of nodes) {
        if (node.kind !== 'element') {
            continue;
        }
        const nextGroup = node.tag === 'optgroup' ? node.attributes.label ?? null : group;
        if (node.tag === kind) {
            const text = nodeText(node).trim();
            choices.push({
                kind,
                key: node.key,
                value: node.attributes.value ?? text,
                label: node.attributes.label ?? text,
                text,
                attributes: node.attributes,
                group,
            });
        }
        choices.push(...collectPayloadChoices(node.children, kind, nextGroup));
    }
    return choices;
}

function choicesByValue(choices: readonly SerializedPayloadChoice[]): Record<string, SerializedPayloadChoice> {
    const byValue: Record<string, SerializedPayloadChoice> = {};
    for (const choice of choices) {
        if (isTemplatePathSegment(choice.value)) {
            byValue[choice.value] = choice;
        }
    }
    return byValue;
}

function payloadElementsByAttribute(
    nodes: readonly SerializedPayloadNode[]
): Record<string, SerializedPayloadElement[]> {
    const byAttribute: Record<string, SerializedPayloadElement[]> = {};
    for (const element of collectPayloadElements(nodes)) {
        for (const name of Object.keys(element.attributes)) {
            byAttribute[name] = [...(byAttribute[name] ?? []), element];
        }
    }
    return byAttribute;
}

function collectPayloadElements(nodes: readonly SerializedPayloadNode[]): SerializedPayloadElement[] {
    const elements: SerializedPayloadElement[] = [];
    for (const node of nodes) {
        if (node.kind !== 'element') {
            continue;
        }
        elements.push({
            key: node.key,
            tag: node.tag,
            namespace: node.namespace,
            text: nodeText(node),
            attributes: node.attributes,
            slot: node.slot,
        });
        elements.push(...collectPayloadElements(node.children));
    }
    return elements;
}

function isTemplatePathSegment(value: string): boolean {
    return /^[A-Za-z_][\w.-]*$/.test(value);
}

function nodeText(node: SerializedPayloadNode): string {
    if (node.kind === 'text' || node.kind === 'comment') {
        return node.text;
    }
    return node.children.map(nodeText).join('');
}

function isRenderBoundary(node: Node): boolean {
    return isRenderStartBoundary(node) || isRenderEndBoundary(node);
}

function isRenderStartBoundary(node: Node): node is Comment {
    return node.nodeType === 8 && node.textContent === 'cem-render-start';
}

function isRenderEndBoundary(node: Node): node is Comment {
    return node.nodeType === 8 && node.textContent === 'cem-render-end';
}
