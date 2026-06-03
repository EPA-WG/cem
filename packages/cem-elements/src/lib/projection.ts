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
 * CEM-ML curly templates lower through the same `projectTemplate` shape once the cem_ml
 * WASM boundary is wired; Phase 3A only ships the XML/HTML parity (DOM) parser here.
 */

const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
const ATTRIBUTE_DECLARATION_TAG = 'attribute';

export type TemplateValue = string | boolean | null;

export interface TemplateSourceAttribute {
    name: string;
    value: string;
}

export type TemplateSourceNode =
    | { kind: 'text'; text: string }
    | { kind: 'comment'; text: string }
    | {
          kind: 'element';
          namespace: string | null;
          tag: string;
          attributes: TemplateSourceAttribute[];
          children: TemplateSourceNode[];
      };

export interface RenderPlanAttribute {
    name: string;
    value: string;
}

export type RenderPlanNode =
    | { kind: 'text'; text: string }
    | { kind: 'comment'; text: string }
    | {
          kind: 'element';
          namespace: string | null;
          tag: string;
          attributes: RenderPlanAttribute[];
          renderNodeId: string;
          children: RenderPlanNode[];
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
    for (const child of Array.from(content.childNodes)) {
        const node = readSourceNode(child);
        if (node) {
            nodes.push(node);
        }
    }
    return nodes;
}

function readSourceNode(source: Node): TemplateSourceNode | undefined {
    if (source.nodeType === 3) {
        return { kind: 'text', text: source.textContent ?? '' };
    }
    if (source.nodeType === 8) {
        return { kind: 'comment', text: source.textContent ?? '' };
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
            .map((child) => readSourceNode(child))
            .filter((node): node is TemplateSourceNode => node !== undefined),
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
    let renderNodeSequence = 0;
    const nextRenderNodeId = (): string => {
        renderNodeSequence += 1;
        return `${input.snapshot.producedTag}-${renderNodeSequence}`;
    };

    const nodes: RenderPlanNode[] = [];
    for (const sourceNode of source) {
        if (isTopLevelNonOutputNode(sourceNode)) {
            continue;
        }
        const planned = projectNode(sourceNode, input.values, nextRenderNodeId);
        if (planned) {
            nodes.push(planned);
        }
    }
    return {
        producedTag: input.snapshot.producedTag,
        instanceId: input.snapshot.instanceId,
        templateArtifactId: input.snapshot.templateArtifactId,
        dataRevision: input.snapshot.dataRevision,
        outputTarget: input.snapshot.outputTarget,
        scopePolicyStamp: input.snapshot.scopePolicyStamp,
        nodes,
    };
}

function projectNode(
    source: TemplateSourceNode,
    values: Record<string, TemplateValue>,
    nextRenderNodeId: () => string
): RenderPlanNode | undefined {
    if (source.kind === 'text') {
        return { kind: 'text', text: interpolateText(source.text, values) };
    }
    if (source.kind === 'comment') {
        return { kind: 'comment', text: source.text };
    }

    const attributes: RenderPlanAttribute[] = [];
    for (const attribute of source.attributes) {
        const resolved = resolveAttribute(attribute.name, attribute.value, values);
        if (resolved) {
            attributes.push(resolved);
        }
    }

    return {
        kind: 'element',
        namespace: source.namespace,
        tag: source.tag,
        attributes,
        renderNodeId: nextRenderNodeId(),
        children: source.children
            .map((child) => projectNode(child, values, nextRenderNodeId))
            .filter((node): node is RenderPlanNode => node !== undefined),
    };
}

/**
 * Materialize a render plan into a live light-DOM fragment. UI-adapter side: this is the
 * only place the projection boundary touches live DOM on the way out.
 */
export function materializeRenderPlan(plan: RenderPlan, document: Document): DocumentFragment {
    const fragment = document.createDocumentFragment();
    for (const node of plan.nodes) {
        fragment.appendChild(materializeNode(node, document));
    }
    return fragment;
}

const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';

function materializeNode(node: RenderPlanNode, document: Document): Node {
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
    for (const child of node.children) {
        element.appendChild(materializeNode(child, document));
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

function isTopLevelNonOutputNode(node: TemplateSourceNode): boolean {
    if (node.kind === 'element') {
        return node.tag === ATTRIBUTE_DECLARATION_TAG;
    }
    return node.kind === 'text' && node.text.trim().length === 0;
}
