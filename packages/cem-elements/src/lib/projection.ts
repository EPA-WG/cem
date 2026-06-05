/**
 * Processing-layer projection boundary for `<cem-element>` (design §4.1/§4.2).
 *
 * The UI adapter (runtime) never lowers templates inline. It crosses this boundary in
 * three steps:
 *
 *   1. {@link readTemplateSource} — the "available parser": the browser DOM parser has
 *      already lowered the declaration `<template>`; this reads its `content` into a
 *      serializable {@link TemplateSourceNode} tree. Live `Node` references stop here.
 *   2. {@link projectTemplate} — the processing layer proper: a pure function that takes
 *      the serializable source plus a serializable data-island snapshot/revision input
 *      and emits a serializable {@link RenderPlan}. It touches no live DOM,
 *      `customElements`, or browser handles, so the same call can later run in a
 *      worker/WASM/edge host unchanged.
 *   3. {@link materializeRenderPlan} — the UI adapter again: turns the render plan into a
 *      live light-DOM `DocumentFragment` for the runtime to commit.
 *
 * CEM-ML curly templates lower through the cem_ql WASM render boundary before entering
 * this same render-plan materialization path.
 */

const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
const ATTRIBUTE_DECLARATION_TAG = 'attribute';
const SLICE_DECLARATION_TAG = 'slice';
const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';
const TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const SOURCE_FRAME_ATTR = 'data-cem-source-frame';

export type TemplateValue = string | boolean | null;
export type SourceMapFidelity = 'author-byte-exact' | 'dom-canonical' | 'declaration-only';

export interface SourceMapRef {
    fidelity: SourceMapFidelity;
    frame: string;
}

export interface TemplateSourceAttribute {
    name: string;
    value: string;
}

export type TemplateSourceNode =
    | { kind: 'text'; text: string; sourceMapRef?: SourceMapRef }
    | { kind: 'comment'; text: string; sourceMapRef?: SourceMapRef }
    | {
          kind: 'element';
          namespace: string | null;
          tag: string;
          attributes: TemplateSourceAttribute[];
          children: TemplateSourceNode[];
          sourceMapRef?: SourceMapRef;
      };

export interface RenderPlanAttribute {
    name: string;
    value: string;
}

export type RenderPlanNode =
    | { kind: 'text'; text: string; sourceMapRef?: SourceMapRef }
    | { kind: 'comment'; text: string; sourceMapRef?: SourceMapRef }
    | {
          kind: 'element';
          namespace: string | null;
          tag: string;
          attributes: RenderPlanAttribute[];
          renderNodeId: string;
          children: RenderPlanNode[];
          sourceMapRef?: SourceMapRef;
      };

export interface RenderPlan {
    producedTag: string;
    instanceId: string;
    templateArtifactId: string;
    dataRevision: string;
    outputTarget: 'light-dom';
    scopePolicyStamp: string;
    nodes: RenderPlanNode[];
}

export interface ProjectionPayload {
    slots?: Record<string, ProjectionPayloadNode[]>;
}

export type ProjectionPayloadNode =
    | { kind: 'text'; key: string; text: string }
    | { kind: 'comment'; key: string; text: string }
    | {
          kind: 'element';
          key: string;
          tag: string;
          namespace: string | null;
          attributes: Record<string, string>;
          children: ProjectionPayloadNode[];
      };

export interface TemplateProjectionSnapshot {
    instanceId: string;
    producedTag: string;
    templateArtifactId: string;
    dataRevision: string;
    outputTarget: 'light-dom';
    scopePolicyStamp: string;
    hostAttributes: Record<string, string | boolean | null>;
    dataset: Record<string, string>;
    payload: unknown;
    slices: Record<string, unknown>;
    validationState: Record<string, unknown>;
    eventPayloads: Record<string, unknown>;
}

export interface TemplateProjectionInput {
    snapshot: TemplateProjectionSnapshot;
    values: Record<string, TemplateValue>;
}

/**
 * Read a declaration template's already-parsed `content` into a serializable source
 * tree. This is the only place the projection boundary touches live DOM on the way in.
 */
export function readTemplateSource(content: ParentNode): TemplateSourceNode[] {
    const nodes: TemplateSourceNode[] = [];
    for (const [index, child] of Array.from(content.childNodes).entries()) {
        const node = readSourceNode(child, `dom:${index}`);
        if (node) {
            nodes.push(node);
        }
    }
    return nodes;
}

function readSourceNode(source: Node, frame: string): TemplateSourceNode | undefined {
    const sourceMapRef: SourceMapRef = { fidelity: 'dom-canonical', frame };
    if (source.nodeType === 3) {
        return { kind: 'text', text: source.textContent ?? '', sourceMapRef };
    }
    if (source.nodeType === 8) {
        return { kind: 'comment', text: source.textContent ?? '', sourceMapRef };
    }
    if (source.nodeType !== 1) {
        return undefined;
    }

    const element = source as Element;
    return {
        kind: 'element',
        namespace: element.namespaceURI && element.namespaceURI !== XHTML_NAMESPACE ? element.namespaceURI : null,
        tag: element.localName,
        attributes: Array.from(element.attributes).map((attribute) => ({
            name: attribute.name,
            value: attribute.value,
        })),
        children: Array.from(element.childNodes)
            .map((child, index) => readSourceNode(child, `${frame}/${index}`))
            .filter((node): node is TemplateSourceNode => node !== undefined),
        sourceMapRef,
    };
}

/**
 * Pure processing-layer projection: serializable source + data snapshot → a
 * serializable render plan. No live DOM, no browser handles.
 *
 * Top-level `<attribute>` declaration nodes are dropped — they configure the produced
 * element rather than producing visible output.
 */
export function projectTemplate(
    source: readonly TemplateSourceNode[],
    input: TemplateProjectionInput
): RenderPlan {
    return projectTemplateWith(source, input, projectNode, isTopLevelNonOutputNode);
}

export function projectLegacyTemplate(
    source: readonly TemplateSourceNode[],
    input: TemplateProjectionInput
): RenderPlan {
    return projectTemplateWith(source, input, projectLegacyNode, isTopLevelLegacyNonOutputNode);
}

function projectTemplateWith(
    source: readonly TemplateSourceNode[],
    input: TemplateProjectionInput,
    project: (
        source: TemplateSourceNode,
        input: TemplateProjectionInput,
        nextRenderNodeId: () => string
    ) => RenderPlanNode[],
    isTopLevelNonOutput: (node: TemplateSourceNode) => boolean
): RenderPlan {
    let renderNodeSequence = 0;
    const nextRenderNodeId = (): string => {
        renderNodeSequence += 1;
        return `${input.snapshot.producedTag}-${renderNodeSequence}`;
    };

    const nodes: RenderPlanNode[] = [];
    for (const sourceNode of source) {
        if (isTopLevelNonOutput(sourceNode)) {
            continue;
        }
        nodes.push(...project(sourceNode, input, nextRenderNodeId));
    }
    const plan: RenderPlan = {
        producedTag: input.snapshot.producedTag,
        instanceId: input.snapshot.instanceId,
        templateArtifactId: input.snapshot.templateArtifactId,
        dataRevision: input.snapshot.dataRevision,
        outputTarget: input.snapshot.outputTarget,
        scopePolicyStamp: input.snapshot.scopePolicyStamp,
        nodes,
    };
    return projectSlotsInRenderPlan(plan, input.snapshot.payload);
}

function projectNode(
    source: TemplateSourceNode,
    input: TemplateProjectionInput,
    nextRenderNodeId: () => string
): RenderPlanNode[] {
    if (source.kind === 'text') {
        return [{ kind: 'text', text: interpolateText(source.text, input.values), sourceMapRef: source.sourceMapRef }];
    }
    if (source.kind === 'comment') {
        return [{ kind: 'comment', text: source.text, sourceMapRef: source.sourceMapRef }];
    }

    const attributes: RenderPlanAttribute[] = [];
    for (const attribute of source.attributes) {
        const resolved = resolveAttribute(attribute.name, attribute.value, input.values);
        if (resolved) {
            attributes.push(resolved);
        }
    }

    return [{
        kind: 'element',
        namespace: source.namespace,
        tag: source.tag,
        attributes,
        renderNodeId: nextRenderNodeId(),
        children: source.children
            .flatMap((child) => projectNode(child, input, nextRenderNodeId)),
        sourceMapRef: source.sourceMapRef,
    }];
}

function projectLegacyNode(
    source: TemplateSourceNode,
    input: TemplateProjectionInput,
    nextRenderNodeId: () => string
): RenderPlanNode[] {
    if (source.kind === 'text') {
        return [{ kind: 'text', text: interpolateLegacy(source.text, input), sourceMapRef: source.sourceMapRef }];
    }
    if (source.kind === 'comment') {
        return [{ kind: 'comment', text: source.text, sourceMapRef: source.sourceMapRef }];
    }
    if (source.tag === 'if') {
        return legacyTestIsTruthy(source.attributes, input)
            ? source.children.flatMap((child) => projectLegacyNode(child, input, nextRenderNodeId))
            : [];
    }
    if (source.tag === 'choose') {
        for (const child of source.children) {
            if (child.kind !== 'element') {
                continue;
            }
            if (child.tag === 'when' && legacyTestIsTruthy(child.attributes, input)) {
                return child.children.flatMap((branch) => projectLegacyNode(branch, input, nextRenderNodeId));
            }
            if (child.tag === 'otherwise') {
                return child.children.flatMap((branch) => projectLegacyNode(branch, input, nextRenderNodeId));
            }
        }
        return [];
    }

    const attributes: RenderPlanAttribute[] = [];
    for (const attribute of source.attributes) {
        if (source.tag === 'attribute' && attribute.name === 'select') {
            continue;
        }
        const value = interpolateLegacy(attribute.value, input);
        if (isWholeLegacyExpression(attribute.value) && (value === '' || value === 'false')) {
            continue;
        }
        attributes.push({ name: attribute.name, value });
    }

    return [{
        kind: 'element',
        namespace: source.namespace,
        tag: source.tag,
        attributes,
        renderNodeId: nextRenderNodeId(),
        children: source.children.flatMap((child) => projectLegacyNode(child, input, nextRenderNodeId)),
        sourceMapRef: source.sourceMapRef,
    }];
}

function legacyTestIsTruthy(attributes: readonly TemplateSourceAttribute[], input: TemplateProjectionInput): boolean {
    const test = attributes.find((attribute) => attribute.name === 'test')?.value ?? '';
    return legacyValueIsTruthy(evaluateLegacyExpression(test, input));
}

/**
 * Pure render-plan lowering for declarative slots. It replaces rendered `<slot>`
 * elements with serialized payload nodes assigned to that slot, or with the
 * slot's already-rendered fallback children when no payload is assigned.
 */
export function projectSlotsInRenderPlan(plan: RenderPlan, payload: unknown): RenderPlan {
    const slotPayload = coerceProjectionPayload(payload);
    if (!slotPayload) {
        return plan;
    }
    const consumed = new Set<string>();
    return {
        ...plan,
        nodes: projectSlotNodes(plan.nodes, slotPayload, consumed),
    };
}

function projectSlotNodes(
    nodes: readonly RenderPlanNode[],
    payload: ProjectionPayload,
    consumed: Set<string>
): RenderPlanNode[] {
    const out: RenderPlanNode[] = [];
    for (const node of nodes) {
        if (node.kind !== 'element') {
            out.push(node);
            continue;
        }
        if (node.tag === 'slot') {
            const name = node.attributes.find((attribute) => attribute.name === 'name')?.value ?? '';
            const projected = collectProjectedSlotPayload(payload, name, consumed);
            out.push(...(projected.length > 0 ? projected : node.children));
            continue;
        }
        out.push({
            ...node,
            children: projectSlotNodes(node.children, payload, consumed),
        });
    }
    return out;
}

function collectProjectedSlotPayload(
    payload: ProjectionPayload,
    name: string,
    consumed: Set<string>
): RenderPlanNode[] {
    const projected: RenderPlanNode[] = [];
    for (const node of payload.slots?.[name] ?? []) {
        if (consumed.has(node.key)) {
            continue;
        }
        projected.push(payloadNodeToRenderNode(node));
        consumed.add(node.key);
    }
    return projected;
}

function payloadNodeToRenderNode(node: ProjectionPayloadNode): RenderPlanNode {
    if (node.kind === 'text') {
        return { kind: 'text', text: node.text };
    }
    if (node.kind === 'comment') {
        return { kind: 'comment', text: node.text };
    }
    return {
        kind: 'element',
        namespace: node.namespace,
        tag: node.tag,
        attributes: Object.entries(node.attributes).map(([name, value]) => ({ name, value })),
        renderNodeId: `payload-${node.key}`,
        children: node.children.map(payloadNodeToRenderNode),
    };
}

function coerceProjectionPayload(payload: unknown): ProjectionPayload | null {
    if (!payload || typeof payload !== 'object') {
        return null;
    }
    const slots = (payload as ProjectionPayload).slots;
    return slots && typeof slots === 'object' ? { slots } : null;
}

/**
 * Materialize a render plan into a live light-DOM fragment. UI-adapter side: this is the
 * only place the projection boundary touches live DOM on the way out.
 */
export function materializeRenderPlan(plan: RenderPlan, document: Document): DocumentFragment {
    const fragment = document.createDocumentFragment();
    for (const node of plan.nodes) {
        fragment.appendChild(materializeNode(node, plan, document));
    }
    return fragment;
}

function materializeNode(node: RenderPlanNode, plan: RenderPlan, document: Document): Node {
    if (node.kind === 'text') {
        return document.createTextNode(node.text);
    }
    if (node.kind === 'comment') {
        return document.createComment(node.text);
    }

    const element = node.namespace
        ? document.createElementNS(node.namespace, node.tag)
        : document.createElement(node.tag);
    for (const attribute of node.attributes) {
        element.setAttribute(attribute.name, attribute.value);
    }
    element.setAttribute(RENDER_NODE_ID_ATTR, node.renderNodeId);
    element.setAttribute(TEMPLATE_ARTIFACT_ID_ATTR, plan.templateArtifactId);
    element.setAttribute(DATA_REVISION_ATTR, plan.dataRevision);
    if (node.sourceMapRef) {
        element.setAttribute(SOURCE_FIDELITY_ATTR, node.sourceMapRef.fidelity);
        element.setAttribute(SOURCE_FRAME_ATTR, node.sourceMapRef.frame);
    }
    for (const child of node.children) {
        element.appendChild(materializeNode(child, plan, document));
    }
    return element;
}

function resolveAttribute(
    name: string,
    value: string,
    values: Record<string, TemplateValue>
): RenderPlanAttribute | undefined {
    const wholeExpression = value.match(/^\{\s*\$([A-Za-z_][\w.-]*)\s*\}$/);
    if (wholeExpression) {
        const resolved = values[wholeExpression[1]] ?? null;
        if (resolved === null || resolved === false) {
            return undefined;
        }
        return { name, value: resolved === true ? '' : resolved };
    }
    return { name, value: interpolateAttribute(value, values) };
}

function interpolateText(text: string, values: Record<string, TemplateValue>): string {
    return text.replace(/\$\{\s*\$([A-Za-z_][\w.-]*)\s*\}/g, (_, name: string) => valueToText(values[name] ?? null));
}

function interpolateAttribute(value: string, values: Record<string, TemplateValue>): string {
    return value.replace(/\{\s*\$([A-Za-z_][\w.-]*)\s*\}/g, (_, name: string) => valueToText(values[name] ?? null));
}

function valueToText(value: TemplateValue): string {
    return value === null ? '' : String(value);
}

function interpolateLegacy(text: string, input: TemplateProjectionInput): string {
    return text.replace(/\{([^{}]+)\}/g, (_, expression: string) =>
        valueToText(evaluateLegacyExpression(expression, input))
    );
}

function evaluateLegacyExpression(expression: string, input: TemplateProjectionInput): TemplateValue {
    const trimmed = expression.trim();
    if (trimmed === '') {
        return null;
    }
    const coalesce = splitLegacyCoalesce(trimmed);
    if (coalesce) {
        const left = evaluateLegacyExpression(coalesce[0], input);
        return legacyValueIsTruthy(left) ? left : evaluateLegacyExpression(coalesce[1], input);
    }
    const quoted = trimmed.match(/^(['"])(.*)\1$/);
    if (quoted) {
        return quoted[2];
    }
    if (trimmed === 'true') {
        return true;
    }
    if (trimmed === 'false') {
        return false;
    }
    const path = legacyExpressionPath(trimmed);
    if (path.length > 0) {
        return legacyPathValue(path, input);
    }
    return null;
}

function splitLegacyCoalesce(expression: string): [string, string] | null {
    const index = expression.indexOf('??');
    return index < 0 ? null : [expression.slice(0, index), expression.slice(index + 2)];
}

function legacyExpressionPath(expression: string): string[] {
    if (expression.startsWith('$')) {
        return expression.slice(1).split('.').filter(Boolean);
    }
    if (expression.startsWith('/datadom/')) {
        return expression.slice('/datadom/'.length).split('/').filter(Boolean);
    }
    if (expression.startsWith('//')) {
        return expression.slice(2).split('/').filter(Boolean);
    }
    if (/^[A-Za-z_][\w.-]*$/.test(expression)) {
        return [expression];
    }
    return [];
}

function legacyPathValue(path: readonly string[], input: TemplateProjectionInput): TemplateValue {
    const [first, ...rest] = path;
    if (!first) {
        return null;
    }
    if (first === 'attributes') {
        return toTemplateValue((input.snapshot.hostAttributes as Record<string, unknown>)[rest.join('.')]);
    }
    if (first === 'dataset') {
        return toTemplateValue((input.snapshot.dataset as Record<string, unknown>)[rest.join('.')]);
    }
    if (first === 'slice' || first === 'slices') {
        return toTemplateValue((input.snapshot.slices as Record<string, unknown>)[rest.join('.')]);
    }
    if (first === 'payload') {
        return readUnknownPath(input.snapshot.payload, rest);
    }
    return (
        input.values[first] ??
        toTemplateValue((input.snapshot.hostAttributes as Record<string, unknown>)[first]) ??
        toTemplateValue((input.snapshot.dataset as Record<string, unknown>)[first]) ??
        toTemplateValue((input.snapshot.slices as Record<string, unknown>)[first]) ??
        null
    );
}

function readUnknownPath(value: unknown, path: readonly string[]): TemplateValue {
    let current = value;
    for (const segment of path) {
        if (!current || typeof current !== 'object' || Array.isArray(current)) {
            return null;
        }
        current = (current as Record<string, unknown>)[segment];
    }
    return toTemplateValue(current);
}

function toTemplateValue(value: unknown): TemplateValue {
    if (value === null || value === undefined) {
        return null;
    }
    if (typeof value === 'string' || typeof value === 'boolean') {
        return value;
    }
    if (typeof value === 'number') {
        return String(value);
    }
    if (typeof value === 'object' && 'text' in value && typeof (value as { text?: unknown }).text === 'string') {
        return (value as { text: string }).text;
    }
    return null;
}

function legacyValueIsTruthy(value: TemplateValue): boolean {
    return value !== null && value !== false && value !== '' && value !== 'false' && value !== '0';
}

function isWholeLegacyExpression(value: string): boolean {
    return /^\{\s*[^{}]+\s*\}$/.test(value);
}

function isTopLevelNonOutputNode(node: TemplateSourceNode): boolean {
    if (node.kind === 'element') {
        return node.tag === ATTRIBUTE_DECLARATION_TAG || node.tag === SLICE_DECLARATION_TAG;
    }
    return node.kind === 'text' && node.text.trim().length === 0;
}

function isTopLevelLegacyNonOutputNode(node: TemplateSourceNode): boolean {
    if (node.kind === 'element') {
        return node.tag === ATTRIBUTE_DECLARATION_TAG || node.tag === SLICE_DECLARATION_TAG;
    }
    return node.kind === 'text' && node.text.trim().length === 0;
}
