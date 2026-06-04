import {
    materializeRenderPlan,
    projectTemplate,
    readTemplateSource,
    type RenderPlan,
    type RenderPlanNode,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
import {
    ensureRuntimeReady,
    renderCemMlTemplate,
    type RuntimeSupportDiagnostic,
} from './internal/runtime-support/cem-ql-render.js';
import { parseCemMlTemplateSource } from './runtime-support/cem-ml-template.js';

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
    /**
     * Whether this declaration's template is within the canonical CEM-ML subset the
     * `cem_ql` WASM engine renders today (no `<attribute>`/`<slice>` declarations, no
     * `${}` C1.5 text interpolation, at least one renderable element). When false, the
     * C1.5 TypeScript adapter remains the renderer until later C2 slices extend WASM.
     */
    wasmEligible: boolean;
    declaredAttributes: AttributeDeclaration[];
    declaredSlices: SliceDeclaration[];
    observedAttributes: string[];
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
    private instanceSequence = 0;

    constructor(options: CemElementRuntimeOptions = {}) {
        this.declarationTag = options.declarationTag ?? DEFAULT_DECLARATION_TAG;
        this.scopePolicyStamp = options.scopePolicyStamp ?? DEFAULT_SCOPE_POLICY_STAMP;
        this.privacyPolicyStamp = options.privacyPolicyStamp ?? DEFAULT_PRIVACY_POLICY_STAMP;
        this.logger = options.logger;
        // Eagerly warm the cem_ql WASM engine so canonical CEM-ML instances can upgrade
        // to the authoritative render boundary as soon as possible. Failures fall back
        // to the C1.5 path and surface per-instance at render time.
        void ensureRuntimeReady().catch(() => undefined);
    }

    /**
     * Resolves once the most recent render for an instance has settled, including the
     * asynchronous `cem_ql` WASM render boundary for canonical CEM-ML. Synchronous
     * (DOM / C1.5 / legacy) renders resolve immediately.
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
            const diagnostics = [
                ...shape.diagnostics,
                declarationDiagnostic(
                    'cem-element.src_not_implemented',
                    '`src` declaration loading is reserved for the URI/source-streaming slice',
                    shape.tag
                ),
            ];
            this.recordDiagnostics(declarationElement, diagnostics);
            return false;
        }

        const template = directTemplateChildren(declarationElement)[0];
        if (!template) {
            this.recordDiagnostics(declarationElement, shape.diagnostics);
            return false;
        }

        const compiled = compileInlineDeclaration(declarationElement, shape.tag, template, this.declarationTag);
        this.recordDiagnostics(declarationElement, [...shape.diagnostics, ...compiled.diagnostics]);
        this.declarations.set(shape.tag, compiled);
        this.defineProducedElement(declarationElement, compiled);
        return true;
    }

    diagnosticsFor(target: object): readonly CemElementDiagnostic[] {
        return this.diagnostics.get(target) ?? [];
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
        const invalidateProducedInstance = this.invalidateProducedInstance.bind(this);
        class ProducedCemElement extends baseElement {
            static get observedAttributes(): string[] {
                return compiled.observedAttributes;
            }

            connectedCallback(): void {
                connectProducedInstance(this, compiled);
            }

            attributeChangedCallback(): void {
                invalidateProducedInstance(this, compiled);
            }
        }

        registry.define(compiled.producedTag, ProducedCemElement);
    }

    private connectProducedInstance(instance: HTMLElement, compiled: CompiledDeclaration): void {
        const island = this.ensureDataIsland(instance);
        this.ensureInstanceState(instance, compiled, island);
        this.renderInstance(instance, compiled);
    }

    private invalidateProducedInstance(instance: HTMLElement, compiled: CompiledDeclaration): void {
        if (!this.initializedInstances.has(instance)) {
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
            // The C1.5 TypeScript adapter cannot lower canonical `{$x}` content, so here it
            // is only the unavailability fallback; the async WASM render owns this output.
            this.renderSettled.set(instance, this.renderViaWasm(instance, compiled, snapshot, token));
            return;
        }

        // DOM parity, the C1.5 bespoke CEM-ML subset, and legacy bridge templates render
        // synchronously through the projection / TS-adapter path.
        const rendered = this.renderFromDeclaration(instance, compiled, snapshot);
        this.projectSlots(snapshot, rendered, instance.ownerDocument);
        this.bindRenderedSliceEvents(instance, compiled, rendered);
        this.replaceRenderedContent(instance, island, rendered);
        this.renderSettled.set(instance, Promise.resolve());
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
            const plan = planFromNodes(result.nodes, snapshot, compiled);
            const fragment = materializeRenderPlan(plan, instance.ownerDocument);
            const island = this.ensureDataIsland(instance);
            this.projectSlots(snapshot, fragment, instance.ownerDocument);
            this.bindRenderedSliceEvents(instance, compiled, fragment);
            this.replaceRenderedContent(instance, island, fragment);
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

    /**
     * Project the produced instance's serialized payload into the `<slot>` positions of a
     * rendered fragment. The live data island is not consulted here; browser, worker, SSR,
     * and edge hosts can reproduce the same projection from `DataIslandSnapshot.payload`.
     */
    private projectSlots(snapshot: DataIslandSnapshot, fragment: DocumentFragment, document: Document): void {
        const slots = Array.from(fragment.querySelectorAll('slot'));
        if (slots.length === 0) {
            return;
        }
        const consumed = new Set<string>();
        for (const slot of slots) {
            const name = slot.getAttribute('name') ?? '';
            const projected = this.collectSlotPayload(snapshot.payload, name, consumed);
            const replacement =
                projected.length > 0
                    ? projected.map((node) => materializePayloadNode(node, document))
                    : Array.from(slot.childNodes).map((node) => node.cloneNode(true));
            slot.replaceWith(...replacement);
        }
    }

    private collectSlotPayload(
        payload: SerializedPayload,
        name: string,
        consumed: Set<string>
    ): SerializedPayloadNode[] {
        const projected: SerializedPayloadNode[] = [];
        for (const node of payload.slots[name] ?? []) {
            if (consumed.has(node.key)) {
                continue;
            }
            projected.push(node);
            consumed.add(node.key);
        }
        return projected;
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
            state.observer = new observer(() => this.invalidateProducedInstance(instance, compiled));
            state.observer.observe(island.content, {
                childList: true,
                subtree: true,
                characterData: true,
                attributes: true,
            });
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

    const templateSource = readInlineTemplateSource(template, mode, producedTag, diagnostics);
    const declaredAttributes =
        mode === 'legacy-v0' ? [] : extractAttributeDeclarationsFromSource(templateSource);
    const declaredSlices = mode === 'legacy-v0' ? [] : extractSliceDeclarationsFromSource(templateSource);
    const cemMlSource = mode === 'cem-ml' ? template.textContent ?? '' : null;
    return {
        declarationElement,
        declarationTag,
        producedTag,
        artifactId: `template-artifact-${++artifactSequence}`,
        template,
        templateSource,
        mode,
        cemMlSource,
        wasmEligible: isCanonicalWasmSubset(mode, cemMlSource, templateSource, declaredAttributes, declaredSlices),
        declaredAttributes,
        declaredSlices,
        observedAttributes: declaredAttributes.map((attribute) => attribute.name),
        diagnostics,
    };
}

/**
 * Whether a CEM-ML declaration falls inside the canonical subset the `cem_ql` WASM
 * engine renders today. It must be CEM-ML mode with no `<attribute>`/`<slice>`
 * declarations (deferred to a later C2 slice), no `${}` C1.5 text interpolation, and
 * at least one renderable element (so degenerate expression-only templates such as
 * `{$ | name}` stay on the C1.5 failure path the diagnostics surface relies on).
 */
function isCanonicalWasmSubset(
    mode: CompiledDeclaration['mode'],
    cemMlSource: string | null,
    templateSource: readonly TemplateSourceNode[],
    declaredAttributes: AttributeDeclaration[],
    declaredSlices: SliceDeclaration[]
): boolean {
    return (
        mode === 'cem-ml' &&
        cemMlSource !== null &&
        declaredAttributes.length === 0 &&
        declaredSlices.length === 0 &&
        !cemMlSource.includes('${') &&
        templateSource.some(
            (node) =>
                node.kind === 'element' &&
                node.tag !== 'attribute' &&
                node.tag !== 'slice' &&
                /^[a-z][a-z0-9]*(-[a-z0-9]+)*$/.test(node.tag)
        )
    );
}

function readInlineTemplateSource(
    template: HTMLTemplateElement,
    mode: CompiledDeclaration['mode'],
    producedTag: string,
    diagnostics: CemElementDiagnostic[]
): TemplateSourceNode[] {
    if (mode === 'dom') {
        return readTemplateSource(template.content);
    }
    if (mode === 'legacy-v0') {
        return [];
    }

    const parsed = parseCemMlTemplateSource(template.textContent ?? '');
    diagnostics.push(
        ...parsed.diagnostics.map((diagnostic) =>
            declarationDiagnostic(diagnostic.code, diagnostic.message, producedTag)
        )
    );
    return parsed.source;
}

function templateMode(template: HTMLTemplateElement): CompiledDeclaration['mode'] {
    if (template.getAttribute('lang') === 'custom-element-v0') {
        return 'legacy-v0';
    }
    const type = template.getAttribute('type');
    if (type === 'text/cem-ml' || type === 'application/cem-ml') {
        return 'cem-ml';
    }
    const source = template.textContent?.trim() ?? '';
    if (source.startsWith('@doc') || source.startsWith('{')) {
        return 'cem-ml';
    }
    return 'dom';
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
    return {
        text: island.content.textContent ?? '',
        childCount: island.content.childNodes.length,
        nodes,
        slots,
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

function materializePayloadNode(node: SerializedPayloadNode, document: Document): Node {
    if (node.kind === 'text') {
        return document.createTextNode(node.text);
    }
    if (node.kind === 'comment') {
        return document.createComment(node.text);
    }

    const element = node.namespace ? document.createElementNS(node.namespace, node.tag) : document.createElement(node.tag);
    for (const [name, value] of Object.entries(node.attributes)) {
        element.setAttribute(name, value);
    }
    for (const child of node.children) {
        element.appendChild(materializePayloadNode(child, document));
    }
    return element;
}

function isRenderBoundary(node: Node): boolean {
    return node.nodeType === 8 && /^cem-render-(start|end)$/.test(node.textContent ?? '');
}
