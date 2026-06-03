import {
    materializeRenderPlan,
    projectTemplate,
    readTemplateSource,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
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
}

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

interface CompiledDeclaration {
    declarationElement: HTMLElement;
    declarationTag: string;
    producedTag: string;
    artifactId: string;
    template: HTMLTemplateElement;
    templateSource: TemplateSourceNode[];
    mode: 'dom' | 'cem-ml' | 'legacy-v0';
    declaredAttributes: AttributeDeclaration[];
    observedAttributes: string[];
    diagnostics: CemElementDiagnostic[];
}

interface RenderBounds {
    start: Comment;
    end: Comment;
}

const DEFAULT_DECLARATION_TAG = 'cem-element';
const DEFAULT_SCOPE_POLICY_STAMP = 'phase-3a-local-default';
const DEFAULT_PRIVACY_POLICY_STAMP = 'local-only';
const DATA_ISLAND_ATTR = 'data-cem-island';
const DATA_ISLAND_VALUE = 'instance';
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
    private instanceSequence = 0;

    constructor(options: CemElementRuntimeOptions = {}) {
        this.declarationTag = options.declarationTag ?? DEFAULT_DECLARATION_TAG;
        this.scopePolicyStamp = options.scopePolicyStamp ?? DEFAULT_SCOPE_POLICY_STAMP;
        this.privacyPolicyStamp = options.privacyPolicyStamp ?? DEFAULT_PRIVACY_POLICY_STAMP;
        this.logger = options.logger;
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
        this.ensureDataIsland(instance);
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
        const snapshot = this.createSnapshot(instance, compiled, island);
        const rendered = this.renderFromDeclaration(instance, compiled, snapshot);
        this.replaceRenderedContent(instance, island, rendered);
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
        const values = templateValues(snapshot, compiled.declaredAttributes);
        const plan = projectTemplate(compiled.templateSource, { snapshot, values });
        return materializeRenderPlan(plan, instance.ownerDocument);
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
            slices: {},
            validationState: {},
            eventPayloads: {},
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
    return {
        declarationElement,
        declarationTag,
        producedTag,
        artifactId: `template-artifact-${++artifactSequence}`,
        template,
        templateSource,
        mode,
        declaredAttributes,
        observedAttributes: declaredAttributes.map((attribute) => attribute.name),
        diagnostics,
    };
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
    return values;
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
    return {
        text: island.content.textContent ?? '',
        childCount: island.content.childNodes.length,
    };
}

function isRenderBoundary(node: Node): boolean {
    return node.nodeType === 8 && /^cem-render-(start|end)$/.test(node.textContent ?? '');
}
