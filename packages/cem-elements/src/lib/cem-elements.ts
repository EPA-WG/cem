import {
    DATA_CEM_INSTANCE_SCOPE_ATTR,
    DATA_CEM_SCOPE_ATTR,
    applyRenderPlanToRange,
    edgeContentAddress,
    materializeRenderPlan,
    projectTemplate,
    readTemplateSource,
    renderInstanceScopeUid,
    renderPlansHaveDomChanges,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type GeneratedRenderPlanIdDiagnostic,
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
export const SNAPSHOT_SCHEMA_VERSION = '1.2.0';

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
    formData?: Record<string, unknown>;
    validationState: Record<string, unknown>;
    eventPayloads: Record<string, unknown>;
}

export type DataIslandSnapshotExportField =
    | 'hostAttributes'
    | 'dataset'
    | 'payload'
    | 'slices'
    | 'formData'
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
    locationResources: Record<string, ActiveLocationResource>;
    resourceRevisions: Record<string, number>;
    observer?: MutationObserver;
}

interface SliceEventBinding {
    instance: HTMLElement;
    sliceNames: string[];
    eventNames: string[];
    expression: string;
    listener: EventListener;
}

interface FormEventBinding {
    instance: HTMLElement;
    eventNames: string[];
    listener: EventListener;
}

interface CapturedRenderedForms {
    formData: Record<string, SerializedFormData>;
    validationState: Record<string, SerializedFormValidation>;
    sliceMirrors: Record<string, Record<string, unknown>>;
}

type SerializedFormData = Record<string, string | string[]>;

interface SerializedFormValidation {
    valid: boolean;
    validationMessage: string;
    controls: Record<string, SerializedControlValidation>;
}

interface SerializedControlValidation {
    tag: string;
    name: string | null;
    type: string | null;
    value: string | null;
    checked: boolean | null;
    disabled: boolean;
    required: boolean;
    willValidate: boolean;
    valid: boolean;
    validationMessage: string;
    validity: Record<string, boolean>;
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

interface LocationElementDeclaration {
    sliceName?: string;
    href?: string;
    live: boolean;
    method?: string;
    src?: string;
}

type LocationReadDeclaration = LocationElementDeclaration & { sliceName: string };

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

interface ActiveLocationResource {
    key: string;
    live: boolean;
    lastValue: unknown;
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
const LOCATION_EVENT = 'cem-location';
const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';
const RENDER_TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const RENDER_DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const SOURCE_FRAME_ATTR = 'data-cem-source-frame';
const RUNTIME_PAYLOAD_ATTRIBUTE_NAMES = new Set([
    DATA_ISLAND_ATTR,
    HYDRATION_METADATA_ATTR,
    DATA_CEM_SCOPE_ATTR,
    DATA_CEM_INSTANCE_SCOPE_ATTR,
    RENDER_NODE_ID_ATTR,
    RENDER_TEMPLATE_ARTIFACT_ID_ATTR,
    RENDER_DATA_REVISION_ATTR,
    SOURCE_FIDELITY_ATTR,
    SOURCE_FRAME_ATTR,
]);
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
const DATA_ISLAND_EXPORT_FIELDS: readonly DataIslandSnapshotExportField[] = [
    'hostAttributes',
    'dataset',
    'payload',
    'slices',
    'formData',
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
const locationTrackers = new WeakSet<Window>();

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
    private readonly formSliceNames = new WeakMap<Element, string[]>();
    private readonly formEventBindings = new WeakMap<Element, FormEventBinding>();
    private readonly customValidityExpressions = new WeakMap<Element, string>();
    private readonly customValidationMessages = new WeakMap<Element, string>();
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
    private readonly generatedIdOwners = new Map<string, HTMLElement>();
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
        if (!this.validateGeneratedDeclarationIds(compiled)) {
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
     * Load and register an external `src` declaration: fetch the referenced document
     * (through the host loader / module-map resolver), then use either the full loaded
     * document (`src="./file.html"`) or the referenced subtree (`src="./file.html#id"`)
     * as the declaration template.
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
        const sourceTemplate =
            reference.id.length > 0
                ? templateFromTarget(document.getElementById(reference.id), declarationElement.ownerDocument)
                : templateFromDocument(document, declarationElement.ownerDocument);
        if (!sourceTemplate) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.src_target_missing',
                    reference.id.length > 0
                        ? `external \`src\` reference \`${src}\` did not resolve to a template or subtree for \`#${reference.id}\``
                        : `external \`src\` reference \`${src}\` did not resolve to a usable document template`,
                    tag
                ),
            ]);
            return;
        }
        await this.registerResolvedDeclaration(declarationElement, tag, sourceTemplate, []);
    }

    /** Resolve a same-document `src="#id"` reference to its `<template>`, or diagnose a miss. */
    private resolveLocalSrcTemplate(
        declarationElement: HTMLElement,
        src: string,
        reference: SrcReference,
        tag: string
    ): HTMLTemplateElement | undefined {
        const template = templateFromTarget(
            declarationElement.ownerDocument.getElementById(reference.id),
            declarationElement.ownerDocument
        );
        if (!template) {
            this.recordDiagnostics(declarationElement, [
                declarationDiagnostic(
                    'cem-element.src_local_target_missing',
                    `local \`src\` reference \`${src}\` did not resolve to a same-document template or subtree`,
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
            state.localStorageResources = {};
            for (const active of Object.values(state.locationResources)) {
                active.destroy?.();
            }
            state.locationResources = {};
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
            const scoped = scopeRenderPlan(result.renderPlan, this.currentScopeUid(instance, compiled), {
                instanceScopeUid: this.currentInstanceScopeUid(instance, compiled),
            });
            this.recordDiagnostics(
                instance,
                scoped.diagnostics.map((diagnostic) => scopedCssDiagnostic(diagnostic, compiled.producedTag))
            );
            this.recordGeneratedRenderPlanDiagnostics(instance, scoped.renderPlan, compiled.producedTag);
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
            const scoped = scopeRenderPlan(plan, this.currentScopeUid(instance, compiled), {
                instanceScopeUid: this.currentInstanceScopeUid(instance, compiled),
            });
            this.recordDiagnostics(
                instance,
                scoped.diagnostics.map((diagnostic) => scopedCssDiagnostic(diagnostic, compiled.producedTag))
            );
            this.recordGeneratedRenderPlanDiagnostics(instance, scoped.renderPlan, compiled.producedTag);
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

    private recordGeneratedRenderPlanDiagnostics(instance: HTMLElement, plan: RenderPlan, tag: string): void {
        if (!this.validateGeneratedIds) {
            return;
        }
        this.recordDiagnostics(
            instance,
            validateRenderPlanGeneratedIds(plan).map((diagnostic) =>
                generatedRenderPlanIdDiagnostic(diagnostic, tag)
            )
        );
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
        instance.appendChild(island);
        for (const child of Array.from(instance.childNodes)) {
            if (child !== island) {
                island.content.appendChild(child);
            }
        }
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
        const identityDiagnostics = hydrationRenderIdentityDiagnostics(instance, bounds, snapshot, this.validateGeneratedIds);
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
            locationResources: {},
            resourceRevisions: {},
        };
        const observer = island.ownerDocument.defaultView?.MutationObserver;
        if (observer) {
            // Observation targets are attached in `observeInstance` (on connect), so the
            // observer can be torn down on disconnect and re-attached on reconnect.
            state.observer = new observer((records) => {
                if (records.some((record) => mutationInvalidatesInstance(record, instance, island))) {
                    this.invalidateProducedInstance(instance, compiled);
                }
            });
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
        const sliceNames = parseSliceTargets(element.getAttribute('slice') ?? '');
        const eventNames = parseSliceEventNames(element.getAttribute('slice-event') ?? '');
        if (sliceNames.length === 0 || eventNames.length === 0) {
            return;
        }
        const expression = element.getAttribute('slice-value') ?? '{$target.value}';
        if (element.localName === 'form') {
            this.formSliceNames.set(element, sliceNames);
        }
        element.removeAttribute('slice');
        element.removeAttribute('slice-event');
        element.removeAttribute('slice-value');

        const existing = this.sliceEventBindings.get(element);
        if (
            existing &&
            existing.instance === instance &&
            stringArraysEqual(existing.sliceNames, sliceNames) &&
            stringArraysEqual(existing.eventNames, eventNames) &&
            existing.expression === expression
        ) {
            return;
        }
        if (existing) {
            for (const eventName of existing.eventNames) {
                element.removeEventListener(eventName, existing.listener);
            }
        }

        const listener: EventListener = (event) => {
            this.writeSlicesFromEvent(instance, compiled, sliceNames, expression, event);
        };
        for (const eventName of eventNames) {
            element.addEventListener(eventName, listener);
        }
        this.sliceEventBindings.set(element, { instance, sliceNames, eventNames, expression, listener });
        if (eventNames.includes('init')) {
            element.dispatchEvent(new Event('init', { bubbles: false }));
        }
    }

    private bindRenderedCustomValidity(rendered: ParentNode): void {
        for (const element of Array.from(rendered.querySelectorAll('[custom-validity]'))) {
            this.bindRenderedCustomValidityElement(element);
        }
    }

    private bindRenderedCustomValidityInRange(bounds: RenderBounds): void {
        for (const element of renderedElementsBetween(bounds, '[custom-validity]')) {
            this.bindRenderedCustomValidityElement(element);
        }
    }

    private bindRenderedCustomValidityElement(element: Element): void {
        const expression = element.getAttribute('custom-validity');
        if (expression === null) {
            return;
        }
        this.customValidityExpressions.set(element, expression);
        element.removeAttribute('custom-validity');
    }

    private bindRenderedFormEvents(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        rendered: ParentNode
    ): void {
        for (const element of Array.from(rendered.querySelectorAll('form'))) {
            this.bindRenderedFormEventElement(instance, compiled, element as HTMLFormElement);
        }
    }

    private bindRenderedFormEventsInRange(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        bounds: RenderBounds
    ): void {
        for (const element of renderedElementsBetween(bounds, 'form')) {
            this.bindRenderedFormEventElement(instance, compiled, element as HTMLFormElement);
        }
    }

    private bindRenderedFormEventElement(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        form: HTMLFormElement
    ): void {
        const sliceNames = parseSliceTargets(form.getAttribute('slice') ?? '');
        if (sliceNames.length > 0) {
            this.formSliceNames.set(form, sliceNames);
            form.removeAttribute('slice');
        }
        if (!this.formNeedsRuntimeBinding(form)) {
            return;
        }

        const eventNames = ['input', 'change', 'submit'];
        const existing = this.formEventBindings.get(form);
        if (existing && existing.instance === instance && stringArraysEqual(existing.eventNames, eventNames)) {
            return;
        }
        if (existing) {
            for (const eventName of existing.eventNames) {
                form.removeEventListener(eventName, existing.listener);
            }
        }

        const listener: EventListener = (event) => {
            if (event.type === 'submit' && !this.isFormCurrentlyValid(form)) {
                event.preventDefault();
            }
            this.renderInstance(instance, compiled);
        };
        for (const eventName of eventNames) {
            form.addEventListener(eventName, listener);
        }
        this.formEventBindings.set(form, { instance, eventNames, listener });
    }

    private formNeedsRuntimeBinding(form: HTMLFormElement): boolean {
        return (
            this.formSliceNames.has(form) ||
            this.customValidityExpressions.has(form) ||
            Array.from(form.querySelectorAll('input,select,textarea,button,fieldset')).some((element) =>
                this.customValidityExpressions.has(element)
            )
        );
    }

    private isFormCurrentlyValid(form: HTMLFormElement): boolean {
        const formMessage = this.customValidationMessages.get(form) ?? '';
        if (formMessage.length > 0) {
            return false;
        }
        return typeof form.checkValidity === 'function' ? form.checkValidity() : true;
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
            Array.from(rendered.querySelectorAll('module-url,http-request,local-storage,location-element')),
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
            renderedElementsBetween(bounds, 'module-url,http-request,local-storage,location-element'),
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
            const locationElement = localName === 'location-element' ? readLocationElementDeclaration(element) : null;
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
                continue;
            }
            if (locationElement) {
                this.bindLocationResource(instance, compiled, locationElement);
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
            if (!resourceValuesEqual(sliceValue, active.lastValue)) {
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
        const changed = !resourceValuesEqual(state.slices[declaration.sliceName], value);
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

    private bindLocationResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        declaration: LocationElementDeclaration
    ): void {
        const window = instance.ownerDocument.defaultView;
        if (!window) {
            this.recordDiagnostics(instance, [
                resourceDiagnostic(
                    'cem-element.location_unavailable',
                    'location-element cannot read a URL in this browser context',
                    compiled.producedTag,
                    'warning'
                ),
            ]);
            return;
        }

        const writeDiagnostics = writeLocationTarget(window, instance.ownerDocument, declaration, compiled.producedTag);
        this.recordDiagnostics(instance, writeDiagnostics);
        if (!declaration.sliceName) {
            return;
        }
        const readDeclaration: LocationReadDeclaration = { ...declaration, sliceName: declaration.sliceName };

        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        const key = locationResourceKey(window, instance.ownerDocument, readDeclaration);
        let active: ActiveLocationResource | undefined = state.locationResources[readDeclaration.sliceName];
        if (active && active.key !== key) {
            active.destroy?.();
            delete state.locationResources[readDeclaration.sliceName];
            active = undefined;
        }

        if (active) {
            return;
        }

        const value = readLocationValue(window, instance.ownerDocument, readDeclaration);
        const changed = this.writeLocationSlice(state, readDeclaration, value, 'initial-read', active);
        active = this.ensureActiveLocationResource(instance, compiled, state, readDeclaration, key, value);
        active.lastValue = value;
        if (changed) {
            queueMicrotask(() => {
                if (instance.isConnected) {
                    this.renderInstance(instance, compiled);
                }
            });
        }
    }

    private writeLocationSlice(
        state: InstanceState,
        declaration: LocationElementDeclaration,
        value: unknown,
        source: LocationSliceSource,
        active: ActiveLocationResource | undefined
    ): boolean {
        if (!declaration.sliceName) {
            return false;
        }
        const changed = !resourceValuesEqual(state.slices[declaration.sliceName], value);
        if (changed) {
            state.slices[declaration.sliceName] = value;
        }
        state.eventPayloads[declaration.sliceName] = {
            type: 'location-element',
            href: declaration.href ?? null,
            live: declaration.live,
            source,
            value,
        };
        if (active) {
            active.lastValue = value;
        }
        return changed;
    }

    private ensureActiveLocationResource(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        state: InstanceState,
        declaration: LocationReadDeclaration,
        key: string,
        value: unknown
    ): ActiveLocationResource {
        const existing = state.locationResources[declaration.sliceName];
        if (existing) {
            return existing;
        }
        const active: ActiveLocationResource = {
            key,
            live: declaration.live,
            lastValue: value,
        };
        if (declaration.live && declaration.href === undefined) {
            active.destroy = this.bindLocationLiveListener(instance, compiled, state, declaration, active);
        }
        state.locationResources[declaration.sliceName] = active;
        return active;
    }

    private bindLocationLiveListener(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        state: InstanceState,
        declaration: LocationReadDeclaration,
        active: ActiveLocationResource
    ): () => void {
        const window = instance.ownerDocument.defaultView;
        if (!window) {
            return () => undefined;
        }
        ensureTrackedLocation(window);
        const listener = () => {
            const value = readLocationValue(window, instance.ownerDocument, declaration);
            if (this.writeLocationSlice(state, declaration, value, 'location-event', active)) {
                this.renderInstance(instance, compiled);
            }
        };
        window.addEventListener('popstate', listener);
        window.addEventListener('hashchange', listener);
        window.addEventListener(LOCATION_EVENT, listener);
        const navigation = (window as Window & { navigation?: EventTarget }).navigation;
        navigation?.addEventListener('navigate', listener);
        return () => {
            window.removeEventListener('popstate', listener);
            window.removeEventListener('hashchange', listener);
            window.removeEventListener(LOCATION_EVENT, listener);
            navigation?.removeEventListener('navigate', listener);
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

    private writeSlicesFromEvent(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        sliceNames: string[],
        expression: string,
        event: Event
    ): void {
        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        const sliceValue = evaluateSliceValue(expression, event, state.slices);
        let changed = false;
        for (const sliceName of sliceNames) {
            state.eventPayloads[sliceName] = serializeEventPayload(event, sliceValue);
            if (state.slices[sliceName] !== sliceValue) {
                state.slices[sliceName] = sliceValue;
                changed = true;
            }
        }
        if (changed) {
            this.renderInstance(instance, compiled);
        }
    }

    private ensureInstanceScope(instance: HTMLElement, compiled: CompiledDeclaration): string {
        const existing = instance.getAttribute(DATA_CEM_SCOPE_ATTR) ?? this.retainedRenderedScope(instance);
        const scopeUid = existing && existing.length > 0 ? existing : compiled.scopeUid;
        if (instance.getAttribute(DATA_CEM_SCOPE_ATTR) !== scopeUid) {
            instance.setAttribute(DATA_CEM_SCOPE_ATTR, scopeUid);
        }
        const existingInstanceScope = instance.getAttribute(DATA_CEM_INSTANCE_SCOPE_ATTR);
        if (!existingInstanceScope || existingInstanceScope.length === 0) {
            instance.setAttribute(DATA_CEM_INSTANCE_SCOPE_ATTR, renderInstanceScopeUid(scopeUid, this.instanceId(instance)));
        }
        return scopeUid;
    }

    private currentScopeUid(instance: HTMLElement, compiled: CompiledDeclaration): string {
        return instance.getAttribute(DATA_CEM_SCOPE_ATTR) || compiled.scopeUid;
    }

    private currentInstanceScopeUid(instance: HTMLElement, compiled: CompiledDeclaration): string {
        return instance.getAttribute(DATA_CEM_INSTANCE_SCOPE_ATTR)
            || renderInstanceScopeUid(this.currentScopeUid(instance, compiled), this.instanceId(instance));
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
            transientElementTags: ['module-url', 'http-request', 'local-storage', 'location-element'],
        };
        if (previous) {
            const bounds = this.ensureRenderBounds(instance, island);
            const result = applyRenderPlanToRange(bounds, renderPlan, instance.ownerDocument, mergeOptions);
            this.recordDiagnostics(
                instance,
                result.diagnostics.map((diagnostic) => renderPlanApplyDiagnostic(diagnostic, compiled.producedTag))
            );
            this.bindRenderedSliceEventsInRange(instance, compiled, bounds);
            this.bindRenderedCustomValidityInRange(bounds);
            this.bindRenderedFormEventsInRange(instance, compiled, bounds);
            const resourcesSettled = this.bindRenderedResourceSlicesInRange(instance, compiled, bounds, token);
            this.committedRenderPlans.set(instance, renderPlan);
            return resourcesSettled;
        }

        const fragment = materializeRenderPlan(renderPlan, instance.ownerDocument);
        this.bindRenderedSliceEvents(instance, compiled, fragment);
        this.bindRenderedCustomValidity(fragment);
        this.bindRenderedFormEvents(instance, compiled, fragment);
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
        const dataRevision = this.nextDataRevision(instance);
        const state = this.instanceStates.get(instance);
        const baseForms = this.captureRenderedForms(instance);
        const baseSnapshot = this.snapshotFromCapturedForms(
            instance,
            compiled,
            island,
            dataRevision,
            state,
            baseForms
        );
        if (!this.applyRenderedCustomValidity(instance, baseSnapshot)) {
            return baseSnapshot;
        }
        return this.snapshotFromCapturedForms(
            instance,
            compiled,
            island,
            dataRevision,
            state,
            this.captureRenderedForms(instance)
        );
    }

    private snapshotFromCapturedForms(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        island: HTMLTemplateElement,
        dataRevision: string,
        state: InstanceState | undefined,
        forms: CapturedRenderedForms
    ): DataIslandSnapshot {
        const slices = { ...(state?.slices ?? {}) };
        for (const [name, mirror] of Object.entries(forms.sliceMirrors)) {
            slices[name] = isPlainRecord(slices[name])
                ? { ...(slices[name] as Record<string, unknown>), ...mirror }
                : mirror;
        }
        return {
            version: SNAPSHOT_SCHEMA_VERSION,
            instanceId: this.instanceId(instance),
            producedTag: compiled.producedTag,
            declarationTag: compiled.declarationTag,
            templateArtifactId: compiled.artifactId,
            dataRevision,
            outputTarget: 'light-dom',
            sourceMapMode: 'dev',
            scopePolicyStamp: this.scopePolicyStamp,
            privacyPolicyStamp: this.privacyPolicyStamp,
            hostAttributes: hostAttributes(instance),
            dataset: datasetEntries(instance),
            payload: serializePayload(island),
            slices,
            formData: forms.formData,
            validationState: forms.validationState,
            eventPayloads: { ...(state?.eventPayloads ?? {}) },
        };
    }

    private applyRenderedCustomValidity(instance: HTMLElement, snapshot: DataIslandSnapshot): boolean {
        const bounds = this.renderBounds.get(instance);
        if (!bounds) {
            return false;
        }
        let applied = false;
        for (const element of renderedElementsBetween(bounds, 'form,input,select,textarea,button,fieldset')) {
            const expression = this.customValidityExpressions.get(element) ?? element.getAttribute('custom-validity');
            if (expression === null || expression === undefined) {
                continue;
            }
            const result = evaluateCustomValidityExpression(
                expression,
                snapshot,
                element,
                this.formKeyForElement(instance, element)
            );
            const message = customValidityMessage(result);
            this.customValidationMessages.set(element, message);
            setElementCustomValidity(element, message);
            applied = true;
        }
        return applied;
    }

    private formKeyForElement(instance: HTMLElement, element: Element): string | null {
        const form = element.localName === 'form'
            ? (element as HTMLFormElement)
            : ((element as HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement | HTMLButtonElement).form ?? null);
        if (!form) {
            return null;
        }
        const bounds = this.renderBounds.get(instance);
        const forms = bounds ? renderedElementsBetween(bounds, 'form') : [];
        const index = Math.max(0, forms.indexOf(form));
        return renderedFormKey(form, this.formSliceNames.get(form), index);
    }

    private captureRenderedForms(instance: HTMLElement): CapturedRenderedForms {
        const bounds = this.renderBounds.get(instance);
        const captured: CapturedRenderedForms = { formData: {}, validationState: {}, sliceMirrors: {} };
        if (!bounds) {
            return captured;
        }
        const forms = renderedElementsBetween(bounds, 'form').filter((element) => element.localName === 'form');
        for (const [index, element] of forms.entries()) {
            const form = element as HTMLFormElement;
            const names = this.formSliceNames.get(form);
            const key = renderedFormKey(form, names, index);
            const formData = serializeRenderedFormData(form);
            const validation = serializeRenderedFormValidation(form, this.customValidationMessages);
            captured.formData[key] = formData;
            captured.validationState[key] = validation;
            captured.sliceMirrors[key] = {
                formData,
                valid: validation.valid,
                validationMessage: validation.validationMessage,
            };
        }
        return captured;
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

    private validateGeneratedDeclarationIds(compiled: CompiledDeclaration): boolean {
        if (!this.validateGeneratedIds) {
            return true;
        }
        let valid = true;
        for (const generated of generatedDeclarationIds(compiled)) {
            const key = `${generated.kind}:${generated.id}`;
            const owner = this.generatedIdOwners.get(key);
            if (owner && owner !== compiled.declarationElement) {
                this.recordDiagnostics(compiled.declarationElement, [
                    declarationDiagnostic(
                        generated.code,
                        `generated ${generated.label} \`${generated.id}\` is already used in this runtime output scope`,
                        compiled.producedTag
                    ),
                ]);
                valid = false;
                continue;
            }
            this.generatedIdOwners.set(key, compiled.declarationElement);
        }
        return valid;
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

interface GeneratedDeclarationId {
    kind: 'scope' | 'template-artifact' | 'anonymous-custom-element-name';
    id: string;
    code: string;
    label: string;
}

function generatedDeclarationIds(compiled: CompiledDeclaration): GeneratedDeclarationId[] {
    return [
        {
            kind: 'scope',
            id: compiled.scopeUid,
            code: 'cem-element.scope_uid_duplicate',
            label: 'scope UID',
        },
        {
            kind: 'template-artifact',
            id: compiled.artifactId,
            code: 'cem-element.template_artifact_id_duplicate',
            label: 'template artifact ID',
        },
    ];
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

function readLocationElementDeclaration(element: Element): LocationElementDeclaration | null {
    const sliceName = element.getAttribute('slice')?.trim();
    const method = element.getAttribute('method')?.trim();
    const src = element.getAttribute('src')?.trim();
    if (!sliceName && (!method || !src)) {
        return null;
    }
    const href = element.getAttribute('href')?.trim();
    return {
        ...(sliceName ? { sliceName } : {}),
        href: href && href.length > 0 ? href : undefined,
        live: booleanAttribute(element, 'live'),
        ...(method && method.length > 0 ? { method } : {}),
        ...(src && src.length > 0 ? { src } : {}),
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
type LocationSliceSource = 'initial-read' | 'location-event';

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

function locationResourceKey(
    window: Window,
    document: Document,
    declaration: LocationElementDeclaration
): string {
    return stableJson({
        scope: declaration.href === undefined ? 'window.location' : 'href',
        href: declaration.href ?? null,
        baseUrl: document.baseURI,
        live: declaration.href === undefined ? declaration.live : false,
        origin: declaration.href === undefined ? window.location.origin : null,
    });
}

function readLocationValue(
    window: Window,
    document: Document,
    declaration: LocationElementDeclaration
): Record<string, unknown> {
    try {
        const url = declaration.href === undefined
            ? new URL(window.location.href)
            : new URL(declaration.href, document.baseURI);
        return locationUrlToRecord(url, declaration.href === undefined ? 'window' : 'href');
    } catch (error) {
        return {
            kind: 'location',
            source: declaration.href === undefined ? 'window' : 'href',
            href: declaration.href ?? null,
            params: {},
            paramEntries: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

function locationUrlToRecord(url: URL, source: 'window' | 'href'): Record<string, unknown> {
    const params: Record<string, string[]> = {};
    const paramEntries: { name: string; value: string; values: string[]; text: string }[] = [];
    const seen = new Set<string>();
    for (const name of url.searchParams.keys()) {
        if (seen.has(name)) {
            continue;
        }
        seen.add(name);
        const values = url.searchParams.getAll(name);
        params[name] = values;
        paramEntries.push({
            name,
            value: values[0] ?? '',
            values,
            text: values.join(','),
        });
    }
    return {
        kind: 'location',
        source,
        href: url.href,
        origin: url.origin,
        protocol: url.protocol,
        username: url.username,
        password: url.password,
        host: url.host,
        hostname: url.hostname,
        port: url.port,
        pathname: url.pathname,
        search: url.search,
        hash: url.hash,
        params,
        paramEntries,
    };
}

function writeLocationTarget(
    window: Window,
    document: Document,
    declaration: LocationElementDeclaration,
    tag: string
): CemElementDiagnostic[] {
    const method = declaration.method?.trim();
    const src = declaration.src?.trim();
    if (!method || !src) {
        return [];
    }
    if (!isSupportedLocationWriteMethod(method)) {
        return [
            resourceDiagnostic(
                'cem-element.location_method_unsupported',
                `location-element method \`${method}\` is not supported`,
                tag,
                'error'
            ),
        ];
    }
    try {
        ensureTrackedLocation(window);
        const currentHref = window.location.href;
        if (method === 'location.hash') {
            const nextHash = src.startsWith('#') || src === '' ? src : `#${src}`;
            if (window.location.hash !== nextHash) {
                window.location.hash = nextHash;
                scheduleLocationChange(window, method);
            }
            return [];
        }

        const nextUrl = new URL(src, currentHref || document.baseURI);
        if (currentHref === nextUrl.href) {
            return [];
        }
        switch (method) {
            case 'location.href':
                window.location.href = src;
                scheduleLocationChange(window, method);
                break;
            case 'location.assign':
                window.location.assign(src);
                scheduleLocationChange(window, method);
                break;
            case 'location.replace':
                window.location.replace(src);
                scheduleLocationChange(window, method);
                break;
            case 'history.pushState':
                window.history.pushState({}, '', nextUrl.href);
                break;
            case 'history.replaceState':
                window.history.replaceState({}, '', nextUrl.href);
                break;
        }
    } catch (error) {
        return [
            resourceDiagnostic(
                'cem-element.location_write_failed',
                `location-element method \`${method}\` failed for \`${src}\`: ${
                    error instanceof Error ? error.message : String(error)
                }`,
                tag,
                'error'
            ),
        ];
    }
    return [];
}

function isSupportedLocationWriteMethod(method: string): boolean {
    return (
        method === 'location.href' ||
        method === 'location.hash' ||
        method === 'location.assign' ||
        method === 'location.replace' ||
        method === 'history.pushState' ||
        method === 'history.replaceState'
    );
}

function ensureTrackedLocation(window: Window): void {
    if (locationTrackers.has(window)) {
        return;
    }
    const historyRecord = window.history as unknown as Record<string, (...args: unknown[]) => unknown>;
    for (const method of ['back', 'forward', 'go', 'pushState', 'replaceState']) {
        const original = historyRecord[method];
        if (typeof original !== 'function') {
            continue;
        }
        try {
            historyRecord[method] = function trackedHistoryMethod(...args: unknown[]): unknown {
                const result = original.apply(window.history, args);
                scheduleLocationChange(window, method);
                return result;
            };
        } catch {
            // Some hosts expose immutable History methods; native location events still work.
        }
    }
    locationTrackers.add(window);
}

function scheduleLocationChange(window: Window, method: string): void {
    queueMicrotask(() => dispatchLocationChange(window, method));
}

function dispatchLocationChange(window: Window, method: string): void {
    const CustomEventCtor = ((window as Window & typeof globalThis).CustomEvent ?? CustomEvent) as typeof CustomEvent;
    window.dispatchEvent(new CustomEventCtor(LOCATION_EVENT, { detail: { method } }));
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

function resourceValuesEqual(left: unknown, right: unknown): boolean {
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

/** The declaration template a `src` reference loads: the target itself, its first template child, or the target subtree. */
function templateFromTarget(target: Element | null, document: Document): HTMLTemplateElement | undefined {
    if (!target) {
        return undefined;
    }
    if (target.localName === 'template') {
        return document.importNode(target, true) as HTMLTemplateElement;
    }
    const childTemplate = directTemplateChildren(target)[0];
    if (childTemplate) {
        return document.importNode(childTemplate, true) as HTMLTemplateElement;
    }
    return templateFromNodes([target], document);
}

function templateFromDocument(sourceDocument: Document, document: Document): HTMLTemplateElement | undefined {
    const bodyNodes = sourceDocument.body ? Array.from(sourceDocument.body.childNodes) : [];
    const sourceNodes = bodyNodes.length > 0 ? bodyNodes : sourceDocument.documentElement ? [sourceDocument.documentElement] : [];
    if (sourceNodes.length === 0) {
        return undefined;
    }
    const meaningfulNodes = sourceNodes.filter((node) => node.nodeType !== 3 || (node.textContent?.trim() ?? '').length > 0);
    if (meaningfulNodes.length === 1) {
        const only = meaningfulNodes[0];
        if (only.nodeType === 1 && (only as Element).localName === 'template') {
            return document.importNode(only, true) as HTMLTemplateElement;
        }
    }
    return templateFromNodes(sourceNodes, document);
}

function templateFromNodes(nodes: readonly Node[], document: Document): HTMLTemplateElement {
    const template = document.createElement('template') as HTMLTemplateElement;
    for (const node of nodes) {
        template.content.append(document.importNode(node, true));
    }
    return template;
}

function directDataIsland(element: Element): HTMLTemplateElement | undefined {
    return Array.from(element.children).find(
        (child): child is HTMLTemplateElement =>
            child.localName === 'template' && child.getAttribute(DATA_ISLAND_ATTR) === DATA_ISLAND_VALUE
    );
}

function mutationInvalidatesInstance(
    record: MutationRecord,
    instance: HTMLElement,
    island: HTMLTemplateElement
): boolean {
    if (record.target === instance) {
        return true;
    }
    return !isNestedRuntimePayloadMutation(record.target, island);
}

function isNestedRuntimePayloadMutation(target: Node, island: HTMLTemplateElement): boolean {
    let current: Node | null = target.nodeType === 1 ? target : target.parentNode;
    while (current && current !== island.content) {
        if (current.nodeType === 1) {
            const element = current as Element;
            if (directDataIsland(element)) {
                return true;
            }
        }
        current = current.parentNode;
    }
    return false;
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
    snapshot: DataIslandSnapshot,
    validateGeneratedIds = false
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
    if (validateGeneratedIds) {
        diagnostics.push(...hydrationGeneratedIdDiagnostics(instance, bounds));
    }
    return diagnostics;
}

function hydrationGeneratedIdDiagnostics(instance: HTMLElement, bounds: RenderBounds): CemElementDiagnostic[] {
    const diagnostics: CemElementDiagnostic[] = [];
    const renderNodeIds = new Map<string, Element>();
    const stylesheetIds = new Map<string, Element>();
    for (const element of renderedElementsBetween(bounds, '*')) {
        const renderNodeId = element.getAttribute(RENDER_NODE_ID_ATTR);
        if (!renderNodeId) {
            continue;
        }
        const existing = renderNodeIds.get(renderNodeId);
        if (existing && existing !== element) {
            diagnostics.push(
                renderDiagnostic(
                    'cem-element.hydration_render_node_id_duplicate',
                    `SSR hydration retained duplicate render-node ID \`${renderNodeId}\``,
                    instance.localName
                )
            );
        } else {
            renderNodeIds.set(renderNodeId, element);
        }
        if (element.localName !== 'style') {
            continue;
        }
        const existingStylesheet = stylesheetIds.get(renderNodeId);
        if (existingStylesheet && existingStylesheet !== element) {
            diagnostics.push(
                renderDiagnostic(
                    'cem-element.hydration_stylesheet_id_duplicate',
                    `SSR hydration retained duplicate stylesheet ID \`${renderNodeId}\``,
                    instance.localName
                )
            );
        } else {
            stylesheetIds.set(renderNodeId, element);
        }
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

function renderedFormKey(form: HTMLFormElement, sliceNames: string[] | undefined, index: number): string {
    const firstSliceName = sliceNames?.find((name) => name.length > 0);
    return (
        firstSliceName ??
        form.getAttribute('slice')?.trim() ??
        form.getAttribute('name')?.trim() ??
        form.id.trim() ??
        `form${index + 1}`
    );
}

function serializeRenderedFormData(form: HTMLFormElement): SerializedFormData {
    const data: SerializedFormData = {};
    const FormDataCtor = form.ownerDocument.defaultView?.FormData;
    if (!FormDataCtor) {
        return data;
    }
    try {
        for (const [name, value] of new FormDataCtor(form).entries()) {
            appendSerializedFormDataValue(data, name, serializeFormDataValue(value));
        }
    } catch {
        // Some browser-hosted forms can throw while controls are mid-mutation; keep the
        // snapshot transport-safe and let the next event/render capture a stable form.
    }
    return data;
}

function appendSerializedFormDataValue(data: SerializedFormData, name: string, value: string): void {
    const existing = data[name];
    if (existing === undefined) {
        data[name] = value;
    } else if (Array.isArray(existing)) {
        existing.push(value);
    } else {
        data[name] = [existing, value];
    }
}

function serializeFormDataValue(value: FormDataEntryValue): string {
    return typeof value === 'string' ? value : value.name;
}

function serializeRenderedFormValidation(
    form: HTMLFormElement,
    customMessages: WeakMap<Element, string>
): SerializedFormValidation {
    const controls: Record<string, SerializedControlValidation> = {};
    const formMessage = customMessages.get(form) ?? '';
    let valid = formMessage.length === 0;
    let validationMessage = formMessage;
    for (const [index, control] of renderedFormControls(form).entries()) {
        const serialized = serializeRenderedControlValidation(control, customMessages);
        controls[uniqueFormControlKey(controls, control, index)] = serialized;
        if (!serialized.valid) {
            valid = false;
            validationMessage ||= serialized.validationMessage;
        }
    }
    return { valid, validationMessage, controls };
}

function renderedFormControls(form: HTMLFormElement): Element[] {
    return Array.from(form.elements).filter(
        (control): control is Element => (control as Element | undefined)?.nodeType === 1
    );
}

function uniqueFormControlKey(
    controls: Record<string, SerializedControlValidation>,
    control: Element,
    index: number
): string {
    const base =
        control.getAttribute('name')?.trim() ||
        control.id.trim() ||
        control.getAttribute('data-role')?.trim() ||
        `${control.localName}${index + 1}`;
    if (!(base in controls)) {
        return base;
    }
    let suffix = 2;
    while (`${base}-${suffix}` in controls) {
        suffix += 1;
    }
    return `${base}-${suffix}`;
}

function serializeRenderedControlValidation(
    control: Element,
    customMessages: WeakMap<Element, string>
): SerializedControlValidation {
    const controlRecord = control as Element & {
        checked?: boolean;
        disabled?: boolean;
        required?: boolean;
        type?: string;
        value?: string;
        validationMessage?: string;
        validity?: ValidityState;
        willValidate?: boolean;
    };
    const validity = serializeValidityState(controlRecord.validity);
    const customMessage = customMessages.get(control) ?? '';
    return {
        tag: control.localName,
        name: control.getAttribute('name'),
        type: controlRecord.type ?? control.getAttribute('type'),
        value: typeof controlRecord.value === 'string' ? controlRecord.value : control.getAttribute('value'),
        checked: typeof controlRecord.checked === 'boolean' ? controlRecord.checked : null,
        disabled: controlRecord.disabled === true,
        required: controlRecord.required === true || control.hasAttribute('required'),
        willValidate: controlRecord.willValidate === true,
        valid: customMessage.length > 0 ? false : validity.valid ?? true,
        validationMessage: customMessage || controlRecord.validationMessage || '',
        validity,
    };
}

function serializeValidityState(validity: ValidityState | undefined): Record<string, boolean> {
    if (!validity) {
        return { valid: true };
    }
    return {
        badInput: validity.badInput,
        customError: validity.customError,
        patternMismatch: validity.patternMismatch,
        rangeOverflow: validity.rangeOverflow,
        rangeUnderflow: validity.rangeUnderflow,
        stepMismatch: validity.stepMismatch,
        tooLong: validity.tooLong,
        tooShort: validity.tooShort,
        typeMismatch: validity.typeMismatch,
        valid: validity.valid,
        valueMissing: validity.valueMissing,
    };
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
        (record.formData === undefined || isPlainRecord(record.formData)) &&
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

function generatedRenderPlanIdDiagnostic(
    diagnostic: GeneratedRenderPlanIdDiagnostic,
    tag: string
): CemElementDiagnostic {
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
            node.tag === 'location-element' ||
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
        formData: snapshot.formData ?? {},
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
    if (value === undefined) {
        return {};
    }
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

interface CustomValidityEvalContext {
    snapshot: DataIslandSnapshot;
    datadom: Record<string, unknown>;
    formKey: string | null;
}

function evaluateCustomValidityExpression(
    expression: string,
    snapshot: DataIslandSnapshot,
    element: Element,
    formKey: string | null
): unknown {
    void element;
    return evaluateCustomValidityValue(unwrapExpression(expression), {
        snapshot,
        datadom: dataDocumentFromSnapshot(snapshot),
        formKey,
    });
}

function customValidityMessage(value: unknown): string {
    if (value === true || value === null || value === undefined) {
        return '';
    }
    if (value === false) {
        return 'invalid';
    }
    return String(value);
}

function setElementCustomValidity(element: Element, message: string): void {
    const control = element as Element & { setCustomValidity?: (message: string) => void };
    if (typeof control.setCustomValidity === 'function') {
        control.setCustomValidity(message);
    }
}

function evaluateCustomValidityValue(expression: string, context: CustomValidityEvalContext): unknown {
    const body = stripBalancedOuterParens(expression.trim());
    const fallback = splitTopLevelOperator(body, '??');
    if (fallback) {
        return truthyCustomValidityValue(evaluateCustomValidityValue(fallback.left, context))
            ? true
            : evaluateCustomValidityValue(fallback.right, context);
    }

    const orParts = splitTopLevelWord(body, 'or');
    if (orParts.length > 1) {
        return orParts.some((part) => truthyCustomValidityValue(evaluateCustomValidityValue(part, context)));
    }
    const andParts = splitTopLevelWord(body, 'and');
    if (andParts.length > 1) {
        return andParts.every((part) => truthyCustomValidityValue(evaluateCustomValidityValue(part, context)));
    }

    const notArg = functionArgument(body, 'not');
    if (notArg !== null) {
        return !truthyCustomValidityValue(evaluateCustomValidityValue(notArg, context));
    }

    const comparison = splitTopLevelComparison(body);
    if (comparison) {
        const left = evaluateCustomValidityValue(comparison.left, context);
        const right = evaluateCustomValidityValue(comparison.right, context);
        return compareCustomValidityValues(left, comparison.operator, right);
    }

    const lengthArg = functionArgument(body, 'string-length') ?? functionArgument(body, 'str:length');
    if (lengthArg !== null) {
        return String(evaluateCustomValidityValue(lengthArg, context) ?? '').length;
    }

    const concatArgs = parseConcatArguments(body);
    if (concatArgs) {
        return concatArgs.map((part) => String(evaluateCustomValidityValue(part, context) ?? '')).join('');
    }

    const literal = quotedLiteral(body);
    if (literal) {
        return body.trim().slice(1, -1);
    }
    if (body === 'true') {
        return true;
    }
    if (body === 'false') {
        return false;
    }
    if (/^-?\d+(?:\.\d+)?$/.test(body)) {
        return Number(body);
    }

    const pathValue = resolveCustomValidityPath(body, context);
    return pathValue !== undefined ? pathValue : body;
}

function truthyCustomValidityValue(value: unknown): boolean {
    return value !== false && value !== null && value !== undefined && value !== '' && value !== 0 && value !== 'false';
}

function compareCustomValidityValues(left: unknown, operator: string, right: unknown): boolean {
    const leftNumber = Number(left);
    const rightNumber = Number(right);
    const numeric = Number.isFinite(leftNumber) && Number.isFinite(rightNumber);
    const leftComparable = numeric ? leftNumber : String(left ?? '');
    const rightComparable = numeric ? rightNumber : String(right ?? '');
    switch (operator) {
        case '=':
        case '==':
            return leftComparable === rightComparable;
        case '!=':
            return leftComparable !== rightComparable;
        case '>':
            return leftComparable > rightComparable;
        case '<':
            return leftComparable < rightComparable;
        case '>=':
            return leftComparable >= rightComparable;
        case '<=':
            return leftComparable <= rightComparable;
        default:
            return false;
    }
}

function resolveCustomValidityPath(path: string, context: CustomValidityEvalContext): unknown {
    const body = path.trim();
    if (body.startsWith('/datadom/slice/')) {
        const parts = body.slice('/datadom/slice/'.length).split('/').filter(Boolean);
        return resolveLegacySlicePath(parts, context);
    }
    if (body.startsWith('/datadom/slices/')) {
        const parts = body.slice('/datadom/slices/'.length).split('/').filter(Boolean);
        return resolveObjectPath(context.snapshot.slices, parts.map(legacyPathSegment));
    }
    if (body.startsWith('/datadom/form-data/') || body.startsWith('/datadom/formData/')) {
        const prefix = body.startsWith('/datadom/form-data/') ? '/datadom/form-data/' : '/datadom/formData/';
        const parts = body.slice(prefix.length).split('/').filter(Boolean).map(legacyPathSegment);
        return resolveObjectPath(context.snapshot.formData ?? {}, parts);
    }
    if (body.startsWith('datadom.')) {
        return resolveObjectPath(context.datadom, body.slice('datadom.'.length).split('.'));
    }
    if (body.startsWith('//form-data/') || body.startsWith('//formData/')) {
        const prefix = body.startsWith('//form-data/') ? '//form-data/' : '//formData/';
        return resolveCurrentFormData(body.slice(prefix.length).split('/').filter(Boolean), context);
    }
    if (body.startsWith('//slice/')) {
        return resolveObjectPath(context.snapshot.slices, body.slice('//slice/'.length).split('/').filter(Boolean));
    }
    if (body.startsWith('//')) {
        const parts = body.slice(2).split('/').filter(Boolean);
        return resolveShorthandCustomValidityPath(parts, context);
    }
    return undefined;
}

function resolveLegacySlicePath(parts: string[], context: CustomValidityEvalContext): unknown {
    if (parts.length === 0) {
        return undefined;
    }
    const [sliceName, next, ...rest] = parts.map(legacyPathSegment);
    if (next === 'form-data' || next === 'formData') {
        return resolveObjectPath(context.snapshot.formData?.[sliceName] ?? {}, rest);
    }
    return resolveObjectPath(context.snapshot.slices, [sliceName, next, ...rest].filter(Boolean));
}

function resolveCurrentFormData(parts: string[], context: CustomValidityEvalContext): unknown {
    const normalized = parts.map(legacyPathSegment);
    if (context.formKey) {
        const value = resolveObjectPath(context.snapshot.formData?.[context.formKey] ?? {}, normalized);
        if (value !== undefined) {
            return value;
        }
    }
    for (const formData of Object.values(context.snapshot.formData ?? {})) {
        const value = resolveObjectPath(formData, normalized);
        if (value !== undefined) {
            return value;
        }
    }
    return undefined;
}

function resolveShorthandCustomValidityPath(parts: string[], context: CustomValidityEvalContext): unknown {
    if (parts.length === 0) {
        return undefined;
    }
    const normalized = parts.map(legacyPathSegment);
    const [first, ...rest] = normalized;
    const sliceValue = resolveObjectPath(context.snapshot.slices, normalized);
    if (sliceValue !== undefined) {
        return sliceValue;
    }
    if (context.formKey) {
        const value = resolveObjectPath(context.snapshot.formData?.[context.formKey] ?? {}, normalized);
        if (value !== undefined) {
            return value;
        }
    }
    for (const formData of Object.values(context.snapshot.formData ?? {})) {
        const value = resolveObjectPath(formData, normalized);
        if (value !== undefined) {
            return value;
        }
    }
    return rest.length === 0 ? context.snapshot.slices[first] : undefined;
}

function resolveObjectPath(root: unknown, parts: readonly string[]): unknown {
    let current = root;
    for (const part of parts) {
        if (!isPlainRecord(current)) {
            return undefined;
        }
        current = current[part];
    }
    return current;
}

function legacyPathSegment(segment: string): string {
    return segment === 'form-data' ? 'formData' : segment.replace(/^@/, '');
}

function functionArgument(value: string, name: string): string | null {
    const prefix = `${name}(`;
    if (!value.startsWith(prefix) || !value.endsWith(')')) {
        return null;
    }
    return stripBalancedOuterParens(value.slice(name.length));
}

function splitTopLevelComparison(value: string): { left: string; operator: string; right: string } | null {
    for (const operator of ['>=', '<=', '!=', '==', '=', '>', '<']) {
        const split = splitTopLevelOperator(value, operator);
        if (split) {
            return { ...split, operator };
        }
    }
    return null;
}

function splitTopLevelOperator(value: string, operator: string): { left: string; right: string } | null {
    const index = topLevelOperatorIndex(value, operator);
    return index >= 0
        ? { left: value.slice(0, index).trim(), right: value.slice(index + operator.length).trim() }
        : null;
}

function splitTopLevelWord(value: string, word: string): string[] {
    const parts: string[] = [];
    let start = 0;
    let depth = 0;
    let quote: string | null = null;
    for (let index = 0; index < value.length; index += 1) {
        const char = value[index];
        if ((char === '"' || char === "'") && quote === null) {
            quote = char;
            continue;
        }
        if (char === quote) {
            quote = null;
            continue;
        }
        if (quote !== null) {
            continue;
        }
        if (char === '(') depth += 1;
        if (char === ')') depth = Math.max(0, depth - 1);
        if (
            depth === 0 &&
            value.slice(index, index + word.length) === word &&
            isWordBoundary(value[index - 1]) &&
            isWordBoundary(value[index + word.length])
        ) {
            parts.push(value.slice(start, index).trim());
            start = index + word.length;
            index += word.length - 1;
        }
    }
    if (start === 0) {
        return [value];
    }
    parts.push(value.slice(start).trim());
    return parts;
}

function topLevelOperatorIndex(value: string, operator: string): number {
    let depth = 0;
    let quote: string | null = null;
    for (let index = 0; index <= value.length - operator.length; index += 1) {
        const char = value[index];
        if ((char === '"' || char === "'") && quote === null) {
            quote = char;
            continue;
        }
        if (char === quote) {
            quote = null;
            continue;
        }
        if (quote !== null) {
            continue;
        }
        if (char === '(') depth += 1;
        if (char === ')') depth = Math.max(0, depth - 1);
        if (depth === 0 && value.slice(index, index + operator.length) === operator) {
            return index;
        }
    }
    return -1;
}

function stripBalancedOuterParens(value: string): string {
    let current = value.trim();
    while (current.startsWith('(') && current.endsWith(')') && enclosesWholeExpression(current)) {
        current = current.slice(1, -1).trim();
    }
    return current;
}

function enclosesWholeExpression(value: string): boolean {
    let depth = 0;
    let quote: string | null = null;
    for (let index = 0; index < value.length; index += 1) {
        const char = value[index];
        if ((char === '"' || char === "'") && quote === null) {
            quote = char;
            continue;
        }
        if (char === quote) {
            quote = null;
            continue;
        }
        if (quote !== null) {
            continue;
        }
        if (char === '(') depth += 1;
        if (char === ')') depth -= 1;
        if (depth === 0 && index < value.length - 1) {
            return false;
        }
    }
    return depth === 0;
}

function isWordBoundary(value: string | undefined): boolean {
    return value === undefined || !/[A-Za-z0-9_-]/.test(value);
}

function evaluateSliceValue(
    expression: string,
    event: Event,
    slices: Record<string, unknown>
): TemplateValue {
    const body = unwrapExpression(expression);
    const concatArgs = parseConcatArguments(body);
    if (concatArgs) {
        return concatArgs.map((part) => toTemplateValue(evaluateSliceAtom(part, event, slices)) ?? '').join('');
    }
    const arithmetic = parseSliceArithmetic(body);
    if (arithmetic) {
        const left = sliceNumberValue(evaluateSliceAtom(arithmetic.left, event, slices));
        const right = sliceNumberValue(evaluateSliceAtom(arithmetic.right, event, slices));
        return String(arithmetic.operator === '+' ? left + right : left - right);
    }
    return toTemplateValue(evaluateSliceAtom(body, event, slices));
}

function evaluateSliceAtom(expression: string, event: Event, slices: Record<string, unknown>): unknown {
    const body = unwrapExpression(expression);
    const target = event.target ?? event.currentTarget;
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
    const eventAlias = body.match(/^\/\/@([A-Za-z_][\w.-]*)$/) ?? body.match(/^@([A-Za-z_][\w.-]*)$/);
    if (eventAlias) {
        return eventAliasValue(eventAlias[1], event, target);
    }
    const sliceReference = sliceReferenceName(body);
    if (sliceReference) {
        return toTemplateValue(slices[sliceReference]);
    }
    return parseLiteralValue(body);
}

function parseSliceTargets(value: string): string[] {
    return value
        .split('|')
        .map((part) => part.trim())
        .filter((part) => part.length > 0 && !part.startsWith('/datadom/attributes/'));
}

function parseSliceEventNames(value: string): string[] {
    return Array.from(
        new Set(
            value
                .split(/\s+/)
                .map((part) => part.trim())
                .filter((part) => part.length > 0)
        )
    );
}

function stringArraysEqual(left: readonly string[], right: readonly string[]): boolean {
    return left.length === right.length && left.every((value, index) => value === right[index]);
}

function parseSliceArithmetic(value: string): { left: string; operator: '+' | '-'; right: string } | null {
    if (quotedLiteral(value)) {
        return null;
    }
    const spacedMatch = value.match(/^(.+?)\s+([+-])\s+(.+)$/);
    if (spacedMatch) {
        return { left: spacedMatch[1].trim(), operator: spacedMatch[2] as '+' | '-', right: spacedMatch[3].trim() };
    }
    const compactMatch = value.match(/^(\/\/(?:slice\/)?[A-Za-z_][\w.]*)([+-])(.+)$/);
    if (!compactMatch) {
        return null;
    }
    return { left: compactMatch[1].trim(), operator: compactMatch[2] as '+' | '-', right: compactMatch[3].trim() };
}

function parseConcatArguments(value: string): string[] | null {
    const match = value.match(/^concat\((.*)\)$/);
    if (!match) {
        return null;
    }
    const args: string[] = [];
    let current = '';
    let quote: string | null = null;
    for (const char of match[1]) {
        if ((char === '"' || char === "'") && quote === null) {
            quote = char;
            current += char;
            continue;
        }
        if (char === quote) {
            quote = null;
            current += char;
            continue;
        }
        if (char === ',' && quote === null) {
            args.push(current.trim());
            current = '';
            continue;
        }
        current += char;
    }
    if (current.trim().length > 0) {
        args.push(current.trim());
    }
    return args;
}

function eventAliasValue(name: string, event: Event, target: EventTarget | null): unknown {
    if (name === 'value') {
        return target instanceof HTMLInputElement ||
            target instanceof HTMLTextAreaElement ||
            target instanceof HTMLSelectElement
            ? target.value
            : target instanceof Element
              ? target.getAttribute('value')
              : null;
    }
    if (name === 'checked') {
        return target instanceof HTMLInputElement ? target.checked : null;
    }
    const record = event as unknown as Record<string, unknown>;
    const value = record[name];
    return typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean' ? value : null;
}

function sliceReferenceName(value: string): string | null {
    const normalized = value.trim();
    const match = normalized.match(/^\/\/(?:slice\/)?([A-Za-z_][\w.-]*)(?:\/text\(\))?$/);
    return match?.[1] ?? null;
}

function sliceNumberValue(value: unknown): number {
    if (value === null || value === undefined || value === '') {
        return 0;
    }
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : 0;
}

function quotedLiteral(value: string): boolean {
    return /^(['"]).*\1$/.test(value.trim());
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
        text: nodes.map(nodeText).join(''),
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
    const childNodes = payloadChildNodes(element);
    return {
        kind: 'element',
        key,
        tag: element.localName,
        namespace: element.namespaceURI === XHTML_NAMESPACE ? null : element.namespaceURI,
        attributes: payloadAttributes(element),
        slot: element.getAttribute('slot') ?? '',
        children: childNodes
            .map((child, index) => serializePayloadNode(child, `${key}/${index}`))
            .filter((child): child is SerializedPayloadNode => child !== undefined),
    };
}

function payloadChildNodes(element: Element): Node[] {
    const island = directDataIsland(element);
    if (island) {
        return Array.from(island.content.childNodes);
    }
    return Array.from(element.childNodes).filter((child) => {
        if (isRenderBoundary(child)) {
            return false;
        }
        if (child.nodeType !== 1) {
            return true;
        }
        const childElement = child as Element;
        return !(
            childElement.localName === 'template'
            && childElement.getAttribute(DATA_ISLAND_ATTR) === DATA_ISLAND_VALUE
        ) && !(
            childElement.localName === 'script'
            && childElement.getAttribute(HYDRATION_METADATA_ATTR) === HYDRATION_METADATA_VALUE
        );
    });
}

function payloadAttributes(element: Element): Record<string, string> {
    return Object.fromEntries(
        Array.from(element.attributes)
            .filter((attribute) => !RUNTIME_PAYLOAD_ATTRIBUTE_NAMES.has(attribute.name))
            .map((attribute) => [attribute.name, attribute.value])
    );
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
