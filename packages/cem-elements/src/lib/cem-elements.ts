import {
    materializeRenderPlan,
    projectTemplate,
    projectSlotsInRenderPlan,
    readTemplateSource,
    type RenderPlan,
    type RenderPlanNode,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
import {
    compileCemMlTemplate,
    ensureRuntimeReady,
    renderCemMlTemplate,
    type RuntimeSupportDiagnostic,
} from './internal/runtime-support/cem-ql-render.js';

export type CemElementDiagnosticSeverity = 'info' | 'warning' | 'error' | 'fatal';

export interface CemElementDiagnostic {
    code: string;
    severity: CemElementDiagnosticSeverity;
    message: string;
    source: 'declaration' | 'instance' | 'render';
    tag?: string;
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
    data: SerializedPayloadChoice[];
    options: SerializedPayloadChoice[];
    dataByValue: Record<string, SerializedPayloadChoice>;
    optionsByValue: Record<string, SerializedPayloadChoice>;
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

export interface DataIslandSnapshot {
    instanceId: string;
    producedTag: string;
    declarationTag: string;
    templateArtifactId: string;
    dataRevision: string;
    outputTarget: 'light-dom';
    scopePolicyStamp: string;
    privacyPolicyStamp: string;
    hostAttributes: Record<string, string | boolean | null>;
    dataset: Record<string, string>;
    payload: SerializedPayload;
    slices: Record<string, unknown>;
    validationState: Record<string, unknown>;
    eventPayloads: Record<string, unknown>;
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
    artifactId: string;
    template: HTMLTemplateElement;
    templateSource: TemplateSourceNode[];
    mode: 'dom' | 'cem-ml' | 'legacy-v0';
    /** Raw canonical CEM-ML source text, retained for the `cem_ql` WASM render boundary. */
    cemMlSource: string | null;
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
    slices: Record<string, TemplateValue>;
    eventPayloads: Record<string, unknown>;
    observer?: MutationObserver;
}

const DEFAULT_DECLARATION_TAG = 'cem-element';
const DEFAULT_SCOPE_POLICY_STAMP = 'phase-3a-local-default';
const DEFAULT_PRIVACY_POLICY_STAMP = 'local-only';
const DATA_ISLAND_ATTR = 'data-cem-island';
const DATA_ISLAND_VALUE = 'instance';
const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
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

export function cemElements(): string {
    return '@epa-wg/cem-elements';
}

export function isValidCustomElementName(tag: string): boolean {
    return /^[a-z][.0-9_a-z-]*-[.0-9_a-z-]*$/.test(tag) && !RESERVED_CUSTOM_ELEMENT_NAMES.has(tag);
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

    if (!src && input.directTemplateCount !== 1) {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.inline_template_count',
                'inline declarations must contain exactly one direct-child `<template>`',
                tag ?? undefined
            )
        );
    }

    if (input.directLiveNodeCount > 0) {
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

export class CemElementRuntime {
    readonly declarationTag: string;
    readonly scopePolicyStamp: string;
    readonly privacyPolicyStamp: string;

    private readonly logger?: Pick<Console, 'warn' | 'error'>;
    private readonly declarations = new Map<string, CompiledDeclaration>();
    private readonly diagnostics = new WeakMap<object, CemElementDiagnostic[]>();
    private readonly initializedInstances = new WeakSet<HTMLElement>();
    private readonly instanceIds = new WeakMap<HTMLElement, string>();
    private readonly dataRevisions = new WeakMap<HTMLElement, number>();
    private readonly renderBounds = new WeakMap<HTMLElement, RenderBounds>();
    private readonly instanceStates = new WeakMap<HTMLElement, InstanceState>();
    private readonly renderTokens = new WeakMap<HTMLElement, number>();
    private readonly renderSettled = new WeakMap<HTMLElement, Promise<void>>();
    private readonly declarationSettled = new WeakMap<object, Promise<void>>();
    private readonly srcDocuments = new Map<string, Promise<Document>>();
    private readonly moduleUrls = new Map<string, Promise<string>>();
    private readonly loadSrcDocumentOption?: CemElementRuntimeOptions['loadSrcDocument'];
    private readonly resolveModuleUrlOption?: CemElementRuntimeOptions['resolveModuleUrl'];
    private instanceSequence = 0;

    constructor(options: CemElementRuntimeOptions = {}) {
        this.declarationTag = options.declarationTag ?? DEFAULT_DECLARATION_TAG;
        this.scopePolicyStamp = options.scopePolicyStamp ?? DEFAULT_SCOPE_POLICY_STAMP;
        this.privacyPolicyStamp = options.privacyPolicyStamp ?? DEFAULT_PRIVACY_POLICY_STAMP;
        this.logger = options.logger;
        this.loadSrcDocumentOption = options.loadSrcDocument;
        this.resolveModuleUrlOption = options.resolveModuleUrl;
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
        const shape = analyzeDeclarationElement(declarationElement);
        if (!shape.ok || !shape.tag) {
            this.recordDiagnostics(declarationElement, shape.diagnostics);
            return false;
        }

        if (shape.src) {
            const reference = parseSrcReference(shape.src);
            if (!reference.local) {
                // External `src="./file#tag"`: fetch, parse, and register asynchronously.
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
            this.declarationSettled.set(
                declarationElement,
                this.registerResolvedDeclaration(declarationElement, shape.tag, localTemplate, shape.diagnostics)
            );
            return true;
        }

        const template = directTemplateChildren(declarationElement)[0];
        if (!template) {
            this.recordDiagnostics(declarationElement, shape.diagnostics);
            return false;
        }
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
        const compiled = compileInlineDeclaration(declarationElement, tag, template, this.declarationTag);
        this.recordDiagnostics(declarationElement, [...shapeDiagnostics, ...compiled.diagnostics]);
        this.declarations.set(tag, compiled);
        this.defineProducedElement(declarationElement, compiled);
        // CEM-ML declaration parse diagnostics (structural well-formedness) come from the
        // async cem_ql WASM compile; cem-ql expression errors surface at render instead.
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

    /** Fetch + parse the document an external `src` references, cached per resolved path. */
    private loadSrcDocumentParsed(declarationElement: HTMLElement, path: string): Promise<Document> {
        const cached = this.srcDocuments.get(path);
        if (cached) {
            return cached;
        }
        const baseDocument = declarationElement.ownerDocument;
        const parsed = this.loadSrcDocument(path, baseDocument).then((html) =>
            new DOMParser().parseFromString(html, 'text/html')
        );
        this.srcDocuments.set(path, parsed);
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
                    diagnostics.map((diagnostic) => ({
                        code: diagnostic.code,
                        severity: diagnostic.severity,
                        source: 'declaration' as const,
                        message: diagnostic.message,
                        tag: compiled.producedTag,
                    }))
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
        const state = this.ensureInstanceState(instance, compiled, island);
        this.observeInstance(instance, island, state);
        this.renderInstance(instance, compiled);
    }

    private disconnectProducedInstance(instance: HTMLElement): void {
        this.instanceStates.get(instance)?.observer?.disconnect();
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

        if (compiled.wasmEligible && compiled.cemMlSource !== null) {
            // Canonical CEM-ML renders through the authoritative `cem_ql` WASM boundary.
            this.renderSettled.set(instance, this.renderViaWasm(instance, compiled, snapshot, token));
            return;
        }

        // DOM parity and legacy bridge templates render synchronously through the
        // projection path.
        const rendered = this.renderFromDeclaration(instance, compiled, snapshot);
        this.bindRenderedSliceEvents(instance, compiled, rendered);
        const resourcesSettled = this.bindRenderedResourceSlices(instance, compiled, rendered, token);
        this.replaceRenderedContent(instance, island, rendered);
        this.renderSettled.set(instance, resourcesSettled);
    }

    private async renderViaWasm(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        snapshot: DataIslandSnapshot,
        token: number
    ): Promise<void> {
        const source = compiled.cemMlSource ?? '';
        try {
            const data = wasmTemplateData(snapshot, compiled.declaredAttributes);
            const result = await renderCemMlTemplate(source, data, {
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
            const plan = projectSlotsInRenderPlan(planFromNodes(result.nodes, snapshot, compiled), snapshot.payload);
            const fragment = materializeRenderPlan(plan, instance.ownerDocument);
            const island = this.ensureDataIsland(instance);
            this.bindRenderedSliceEvents(instance, compiled, fragment);
            const resourcesSettled = this.bindRenderedResourceSlices(instance, compiled, fragment, token);
            this.replaceRenderedContent(instance, island, fragment);
            await resourcesSettled;
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
    ): DocumentFragment {
        if (compiled.mode === 'legacy-v0') {
            this.recordDiagnostics(instance, [
                {
                    code: 'cem-element.legacy_template_not_implemented',
                    severity: 'error',
                    source: 'render',
                    tag: compiled.producedTag,
                    message: '`custom-element-v0` bridge templates are reserved for the bridge-support slice',
                },
            ]);
            return instance.ownerDocument.createDocumentFragment();
        }

        // UI adapter → processing layer → UI adapter: project the serializable template
        // source against a serializable data-island snapshot, then materialize the plan
        // into live light DOM.
        try {
            const values = templateValues(snapshot, compiled.declaredAttributes);
            const plan = projectTemplate(compiled.templateSource, { snapshot, values });
            return materializeRenderPlan(plan, instance.ownerDocument);
        } catch (error) {
            this.recordDiagnostics(instance, [
                renderDiagnostic(
                    'cem-element.render_failed',
                    error instanceof Error ? error.message : 'render failed',
                    compiled.producedTag
                ),
            ]);
            return instance.ownerDocument.createDocumentFragment();
        }
    }

    private ensureDataIsland(instance: HTMLElement): HTMLTemplateElement {
        const existing = directDataIsland(instance);
        if (existing) {
            if (!this.initializedInstances.has(instance)) {
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

    private ensureInstanceState(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        island: HTMLTemplateElement
    ): InstanceState {
        const existing = this.instanceStates.get(instance);
        if (existing) {
            return existing;
        }

        const state: InstanceState = {
            slices: Object.fromEntries(
                compiled.declaredSlices.map((slice) => [slice.name, slice.defaultValue])
            ),
            eventPayloads: {},
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
        rendered: DocumentFragment
    ): void {
        for (const element of Array.from(rendered.querySelectorAll('[slice][slice-event]'))) {
            const sliceName = element.getAttribute('slice')?.trim();
            const eventName = element.getAttribute('slice-event')?.trim();
            if (!sliceName || !eventName) {
                continue;
            }
            const expression = element.getAttribute('slice-value') ?? '{$target.value}';
            element.removeAttribute('slice');
            element.removeAttribute('slice-event');
            element.removeAttribute('slice-value');
            element.addEventListener(eventName, (event) => {
                this.writeSliceFromEvent(instance, compiled, sliceName, expression, event);
            });
        }
    }

    private bindRenderedResourceSlices(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        rendered: DocumentFragment,
        token: number
    ): Promise<void> {
        const resourceElements = Array.from(rendered.querySelectorAll('module-url'));
        if (resourceElements.length === 0) {
            return Promise.resolve();
        }

        const tasks: Promise<{ sliceName: string; specifier: string; value: string; error?: unknown }>[] = [];
        for (const element of resourceElements) {
            element.remove();
            const sliceName = element.getAttribute('slice')?.trim();
            const specifier = element.getAttribute('src')?.trim();
            if (!sliceName || !specifier) {
                continue;
            }
            tasks.push(
                this.resolveModuleUrl(specifier, instance.ownerDocument)
                    .then((value) => ({ sliceName, specifier, value }))
                    .catch((error: unknown) => ({ sliceName, specifier, value: specifier, error }))
            );
        }
        if (tasks.length === 0) {
            return Promise.resolve();
        }

        return Promise.all(tasks).then(async (resolved) => {
            if (this.renderTokens.get(instance) !== token || !instance.isConnected) {
                return;
            }
            const island = this.ensureDataIsland(instance);
            const state = this.ensureInstanceState(instance, compiled, island);
            let changed = false;
            const diagnostics: CemElementDiagnostic[] = [];
            for (const result of resolved) {
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
            }
            this.recordDiagnostics(instance, diagnostics);
            if (changed) {
                this.renderInstance(instance, compiled);
                await this.whenRenderSettled(instance);
            }
        });
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

    private writeSliceFromEvent(
        instance: HTMLElement,
        compiled: CompiledDeclaration,
        sliceName: string,
        expression: string,
        event: Event
    ): void {
        const island = this.ensureDataIsland(instance);
        const state = this.ensureInstanceState(instance, compiled, island);
        state.eventPayloads[sliceName] = {
            type: event.type,
        };
        state.slices[sliceName] = evaluateSliceValue(expression, event, state.slices);
        this.renderInstance(instance, compiled);
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
            instanceId: this.instanceId(instance),
            producedTag: compiled.producedTag,
            declarationTag: compiled.declarationTag,
            templateArtifactId: compiled.artifactId,
            dataRevision: this.nextDataRevision(instance),
            outputTarget: 'light-dom',
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
    declarationTag: string
): CompiledDeclaration {
    const mode = templateMode(template);
    const diagnostics: CemElementDiagnostic[] = [];
    if (mode === 'legacy-v0') {
        diagnostics.push(
            declarationDiagnostic(
                'cem-element.legacy_template_not_implemented',
                '`custom-element-v0` templates are reserved for the bridge-support slice',
                producedTag
            )
        );
    }

    const templateSource = readInlineTemplateSource(template, mode);
    // DOM-parity templates extract their declarations here for the synchronous projection
    // path. CEM-ML templates render through the cem_ql WASM boundary, which owns declared
    // attributes/slices and their defaults, so nothing is scanned synchronously for them.
    const declaredAttributes = mode === 'dom' ? extractAttributeDeclarationsFromSource(templateSource) : [];
    const declaredSlices = mode === 'dom' ? extractSliceDeclarationsFromSource(templateSource) : [];
    const cemMlSource = mode === 'cem-ml' ? templateSourceText(template) : null;
    return {
        declarationElement,
        declarationTag,
        producedTag,
        artifactId: `template-artifact-${++artifactSequence}`,
        template,
        templateSource,
        mode,
        cemMlSource,
        wasmEligible: mode === 'cem-ml',
        declaredAttributes,
        declaredSlices,
        diagnostics,
    };
}

/**
 * Read the synchronous template source for a declaration. DOM-parity templates lower
 * through the browser DOM parser into a serializable source tree. CEM-ML templates render
 * through the cem_ql WASM boundary — which owns parsing, declaration metadata, defaults,
 * and diagnostics — so no synchronous source is read for them. Legacy bridge templates are
 * inert until the bridge-support slice.
 */
function readInlineTemplateSource(
    template: HTMLTemplateElement,
    mode: CompiledDeclaration['mode']
): TemplateSourceNode[] {
    return mode === 'dom' ? readTemplateSource(template.content) : [];
}

function templateMode(template: HTMLTemplateElement): CompiledDeclaration['mode'] {
    if (template.getAttribute('lang') === 'custom-element-v0') {
        return 'legacy-v0';
    }
    const type = template.getAttribute('type');
    if (type === 'text/cem-ml' || type === 'application/cem-ml') {
        return 'cem-ml';
    }
    const source = templateSourceText(template).trim();
    if (source.startsWith('@doc') || source.startsWith('{')) {
        return 'cem-ml';
    }
    return 'dom';
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

function resourceDiagnostic(code: string, message: string, tag?: string): CemElementDiagnostic {
    return {
        code,
        severity: 'warning',
        source: 'render',
        message,
        tag,
    };
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
    return {
        attributes: snapshot.hostAttributes,
        dataset: snapshot.dataset,
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

/** Wrap WASM-produced render-plan nodes in a render plan carrying snapshot identity. */
function planFromNodes(
    nodes: RenderPlanNode[],
    snapshot: DataIslandSnapshot,
    compiled: CompiledDeclaration
): RenderPlan {
    return {
        producedTag: compiled.producedTag,
        instanceId: snapshot.instanceId,
        templateArtifactId: compiled.artifactId,
        dataRevision: snapshot.dataRevision,
        outputTarget: 'light-dom',
        scopePolicyStamp: snapshot.scopePolicyStamp,
        nodes,
    };
}

function runtimeSupportDiagnostic(diagnostic: RuntimeSupportDiagnostic, tag: string): CemElementDiagnostic {
    return {
        code: diagnostic.code,
        severity: diagnostic.severity,
        source: 'render',
        message: diagnostic.message,
        tag,
    };
}

function evaluateSliceValue(
    expression: string,
    event: Event,
    slices: Record<string, TemplateValue>
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
        return slices[body.slice(1)] ?? null;
    }
    return parseLiteralValue(body);
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
    return node.nodeType === 8 && /^cem-render-(start|end)$/.test(node.textContent ?? '');
}
