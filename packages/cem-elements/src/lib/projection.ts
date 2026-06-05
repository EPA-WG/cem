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

export interface RenderRevision {
    instanceId: string;
    dataRevision: string;
    templateArtifactId: string;
    scopePolicyStamp: string;
    outputTarget: 'light-dom';
}

export interface RenderPlanIdentity extends RenderRevision {
    producedTag: string;
}

export type DomPatchTarget = { kind: 'render-node'; id: string };

export type SerializedNode =
    | {
          kind: 'element';
          renderNodeId: string;
          tagName: string;
          attributes: Record<string, string>;
          children: SerializedNode[];
          sourceMapRef?: SourceMapRef;
      }
    | { kind: 'text'; renderNodeId: string; text: string; sourceMapRef?: SourceMapRef }
    | { kind: 'comment'; renderNodeId: string; text: string; sourceMapRef?: SourceMapRef };

export type PatchNodePayload = {
    encoding: 'structured-node-v1';
    node: SerializedNode;
};

export type DomPatchOp =
    | { op: 'replace'; target: DomPatchTarget; node: PatchNodePayload }
    | { op: 'setText'; target: DomPatchTarget; value: string }
    | { op: 'setAttribute'; target: DomPatchTarget; name: string; value: string | null }
    | {
          op: 'replaceScope';
          scopeId: string;
          node: PatchNodePayload;
          reason: 'first-render' | 'fallback' | 'policy' | 'recovery';
      };

export type PatchFrame =
    | { type: 'begin'; transactionId: string; revision: RenderRevision }
    | { type: 'ops'; transactionId: string; batchIndex: number; ops: DomPatchOp[] }
    | { type: 'commit'; transactionId: string; nextRenderPlan: RenderPlanIdentity };

export interface EdgePatchOptions {
    batchSize?: number;
    transactionId?: string;
}

export type EdgeContentKind = 'template-artifact' | 'render-plan' | 'rendered-html' | 'sanitized-snapshot';

export interface EdgeContentAddress {
    kind: EdgeContentKind;
    algorithm: 'stable-json-fnv1a64-v1';
    digest: string;
    key: string;
}

export interface EdgeRenderStateRecord {
    storageModel: 'content-addressed-cache-with-revision-pointer-v1';
    stateKey: string;
    producedTag: string;
    instanceId: string;
    templateArtifactId: string;
    scopePolicyStamp: string;
    privacyPolicyStamp?: string;
    renderRevision: RenderRevision;
    currentRenderPlan: EdgeContentAddress;
    currentSnapshot?: EdgeContentAddress;
    currentHtml?: EdgeContentAddress;
    etag: string;
}

export interface EdgeRenderStateInput {
    renderPlan: RenderPlan;
    sanitizedSnapshot?: unknown;
    renderedHtml?: string;
    privacyPolicyStamp?: string;
    stateKey?: string;
}

export type EdgeContentReadResult<T = unknown> =
    | { ok: true; address: EdgeContentAddress; value: T }
    | { ok: false; reason: 'missing-content'; address: EdgeContentAddress }
    | {
          ok: false;
          reason: 'content-address-mismatch';
          expected: EdgeContentAddress;
          actual: EdgeContentAddress;
      };

export type EdgeRenderStateWriteResult =
    | { ok: true; record: EdgeRenderStateRecord }
    | { ok: false; reason: 'etag-mismatch'; current: EdgeRenderStateRecord | undefined };

export interface EdgeRenderStateWriteOptions {
    expectedEtag?: string;
}

export interface EdgeRenderStateAdvanceOptions extends EdgeRenderStateWriteOptions {
    patchOptions?: EdgePatchOptions;
}

export interface EdgeProjectionAdvanceInput {
    source: readonly TemplateSourceNode[];
    projection: TemplateProjectionInput;
    sanitizedSnapshot?: unknown;
    renderedHtml?: string;
    privacyPolicyStamp?: string;
    stateKey?: string;
}

export interface EdgeRenderStateStore {
    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress;
    getContent<T = unknown>(address: EdgeContentAddress): T | undefined;
    readRecord(stateKey: string): EdgeRenderStateRecord | undefined;
    writeRecord(record: EdgeRenderStateRecord, options?: EdgeRenderStateWriteOptions): EdgeRenderStateWriteResult;
    writeRenderState(input: EdgeRenderStateInput, options?: EdgeRenderStateWriteOptions): EdgeRenderStateWriteResult;
}

export type EdgeRenderStateAdvanceResult =
    | {
          ok: true;
          previousRenderPlan: RenderPlan | null;
          frames: PatchFrame[];
          record: EdgeRenderStateRecord;
      }
    | { ok: false; reason: 'etag-mismatch'; current: EdgeRenderStateRecord | undefined }
    | {
          ok: false;
          reason: 'missing-render-plan';
          current: EdgeRenderStateRecord;
          address: EdgeContentAddress;
      }
    | {
          ok: false;
          reason: 'content-address-mismatch';
          current: EdgeRenderStateRecord;
          expected: EdgeContentAddress;
          actual: EdgeContentAddress;
      };

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

export function renderPlanIdentity(plan: RenderPlan): RenderPlanIdentity {
    return {
        producedTag: plan.producedTag,
        instanceId: plan.instanceId,
        dataRevision: plan.dataRevision,
        templateArtifactId: plan.templateArtifactId,
        scopePolicyStamp: plan.scopePolicyStamp,
        outputTarget: plan.outputTarget,
    };
}

export function diffRenderPlansToPatchFrames(
    previous: RenderPlan | null,
    next: RenderPlan,
    options: EdgePatchOptions = {}
): PatchFrame[] {
    const batchSize = options.batchSize ?? 16;
    const transactionId = options.transactionId ?? patchTransactionId(next);
    const ops = diffRenderPlans(previous, next);
    const frames: PatchFrame[] = [{ type: 'begin', transactionId, revision: renderPlanIdentity(next) }];

    for (let index = 0; index < ops.length; index += batchSize) {
        frames.push({
            type: 'ops',
            transactionId,
            batchIndex: index / batchSize,
            ops: ops.slice(index, index + batchSize),
        });
    }

    frames.push({ type: 'commit', transactionId, nextRenderPlan: renderPlanIdentity(next) });
    return frames;
}

export function edgeContentAddress(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
    const digest = stableJsonDigest(value);
    const algorithm = 'stable-json-fnv1a64-v1';
    return {
        kind,
        algorithm,
        digest,
        key: `${kind}:${algorithm}:${digest}`,
    };
}

export function createEdgeRenderStateRecord(input: EdgeRenderStateInput): EdgeRenderStateRecord {
    const identity = renderPlanIdentity(input.renderPlan);
    const currentRenderPlan = edgeContentAddress('render-plan', input.renderPlan);
    const recordWithoutEtag = {
        storageModel: 'content-addressed-cache-with-revision-pointer-v1' as const,
        stateKey: input.stateKey ?? edgeRenderStateKey(identity),
        producedTag: input.renderPlan.producedTag,
        instanceId: input.renderPlan.instanceId,
        templateArtifactId: input.renderPlan.templateArtifactId,
        scopePolicyStamp: input.renderPlan.scopePolicyStamp,
        privacyPolicyStamp: input.privacyPolicyStamp,
        renderRevision: identity,
        currentRenderPlan,
        currentSnapshot: input.sanitizedSnapshot
            ? edgeContentAddress('sanitized-snapshot', input.sanitizedSnapshot)
            : undefined,
        currentHtml: input.renderedHtml ? edgeContentAddress('rendered-html', input.renderedHtml) : undefined,
    };
    return {
        ...recordWithoutEtag,
        etag: edgeContentAddress('render-plan', recordWithoutEtag).digest,
    };
}

export function edgeRenderStateRevisionMatches(
    record: EdgeRenderStateRecord,
    expectedRevision: RenderRevision
): boolean {
    return renderRevisionKey(record.renderRevision) === renderRevisionKey(expectedRevision);
}

export function readEdgeContent<T = unknown>(
    store: EdgeRenderStateStore,
    address: EdgeContentAddress
): EdgeContentReadResult<T> {
    const value = store.getContent<T>(address);
    if (value === undefined) {
        return { ok: false, reason: 'missing-content', address };
    }
    const actual = edgeContentAddress(address.kind, value);
    if (actual.key !== address.key) {
        return { ok: false, reason: 'content-address-mismatch', expected: address, actual };
    }
    return { ok: true, address, value };
}

export function advanceEdgeRenderState(
    store: EdgeRenderStateStore,
    input: EdgeRenderStateInput,
    options: EdgeRenderStateAdvanceOptions = {}
): EdgeRenderStateAdvanceResult {
    const stateKey = input.stateKey ?? edgeRenderStateKey(renderPlanIdentity(input.renderPlan));
    const current = store.readRecord(stateKey);
    let previousRenderPlan: RenderPlan | null = null;
    if (current) {
        const storedPreviousPlan = readEdgeContent<RenderPlan>(store, current.currentRenderPlan);
        if (!storedPreviousPlan.ok && storedPreviousPlan.reason === 'missing-content') {
            return {
                ok: false,
                reason: 'missing-render-plan',
                current,
                address: current.currentRenderPlan,
            };
        }
        if (!storedPreviousPlan.ok) {
            return {
                ok: false,
                reason: 'content-address-mismatch',
                current,
                expected: storedPreviousPlan.expected,
                actual: storedPreviousPlan.actual,
            };
        }
        previousRenderPlan = storedPreviousPlan.value;
    }
    const expectedEtag = options.expectedEtag ?? current?.etag;
    const write = store.writeRenderState(
        { ...input, stateKey },
        expectedEtag === undefined ? {} : { expectedEtag }
    );
    if (!write.ok) {
        return write;
    }
    return {
        ok: true,
        previousRenderPlan,
        frames: diffRenderPlansToPatchFrames(previousRenderPlan, input.renderPlan, options.patchOptions),
        record: write.record,
    };
}

export function projectAndAdvanceEdgeRenderState(
    store: EdgeRenderStateStore,
    input: EdgeProjectionAdvanceInput,
    options: EdgeRenderStateAdvanceOptions = {}
): EdgeRenderStateAdvanceResult {
    return advanceEdgeRenderState(
        store,
        {
            renderPlan: projectTemplate(input.source, input.projection),
            sanitizedSnapshot: input.sanitizedSnapshot,
            renderedHtml: input.renderedHtml,
            privacyPolicyStamp: input.privacyPolicyStamp,
            stateKey: input.stateKey,
        },
        options
    );
}

export class InMemoryEdgeRenderStateStore implements EdgeRenderStateStore {
    private readonly contents = new Map<string, unknown>();
    private readonly records = new Map<string, EdgeRenderStateRecord>();

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        const address = edgeContentAddress(kind, value);
        this.contents.set(address.key, cloneStableJsonValue(value));
        return address;
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        const value = this.contents.get(address.key);
        return value === undefined ? undefined : cloneStableJsonValue(value) as T;
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        const record = this.records.get(stateKey);
        return record ? cloneStableJsonValue(record) as EdgeRenderStateRecord : undefined;
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options: EdgeRenderStateWriteOptions = {}
    ): EdgeRenderStateWriteResult {
        const current = this.records.get(record.stateKey);
        if (options.expectedEtag !== undefined && current?.etag !== options.expectedEtag) {
            return {
                ok: false,
                reason: 'etag-mismatch',
                current: current ? cloneStableJsonValue(current) as EdgeRenderStateRecord : undefined,
            };
        }
        const stored = cloneStableJsonValue(record) as EdgeRenderStateRecord;
        this.records.set(record.stateKey, stored);
        return { ok: true, record: cloneStableJsonValue(stored) as EdgeRenderStateRecord };
    }

    writeRenderState(
        input: EdgeRenderStateInput,
        options: EdgeRenderStateWriteOptions = {}
    ): EdgeRenderStateWriteResult {
        this.putContent('render-plan', input.renderPlan);
        if (input.sanitizedSnapshot !== undefined) {
            this.putContent('sanitized-snapshot', input.sanitizedSnapshot);
        }
        if (input.renderedHtml !== undefined) {
            this.putContent('rendered-html', input.renderedHtml);
        }
        return this.writeRecord(createEdgeRenderStateRecord(input), options);
    }
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

function diffRenderPlans(previous: RenderPlan | null, next: RenderPlan): DomPatchOp[] {
    if (!previous) {
        return next.nodes.map((node) => ({
            op: 'replaceScope',
            scopeId: next.producedTag,
            node: structuredPatchNode(node),
            reason: 'first-render',
        }));
    }

    if (
        previous.producedTag !== next.producedTag ||
        previous.templateArtifactId !== next.templateArtifactId ||
        previous.outputTarget !== next.outputTarget ||
        previous.nodes.length !== next.nodes.length
    ) {
        return next.nodes.map((node) => ({
            op: 'replaceScope',
            scopeId: next.producedTag,
            node: structuredPatchNode(node),
            reason: 'fallback',
        }));
    }

    const ops: DomPatchOp[] = [];
    for (let index = 0; index < next.nodes.length; index += 1) {
        diffRenderNode(previous.nodes[index], next.nodes[index], ops);
    }
    return ops;
}

function diffRenderNode(previous: RenderPlanNode, next: RenderPlanNode, ops: DomPatchOp[]): void {
    if (previous.kind !== next.kind || renderNodeId(previous) !== renderNodeId(next)) {
        ops.push({ op: 'replace', target: renderNodeTarget(previous), node: structuredPatchNode(next) });
        return;
    }

    if (previous.kind === 'text' && next.kind === 'text') {
        if (previous.text !== next.text) {
            ops.push({ op: 'setText', target: renderNodeTarget(previous), value: next.text });
        }
        return;
    }

    if (previous.kind === 'comment' && next.kind === 'comment') {
        if (previous.text !== next.text) {
            ops.push({ op: 'setText', target: renderNodeTarget(previous), value: next.text });
        }
        return;
    }

    if (previous.kind === 'element' && next.kind === 'element') {
        if (
            previous.tag !== next.tag ||
            previous.namespace !== next.namespace ||
            previous.children.length !== next.children.length
        ) {
            ops.push({ op: 'replace', target: renderNodeTarget(previous), node: structuredPatchNode(next) });
            return;
        }

        diffAttributes(previous, next, ops);
        for (let index = 0; index < next.children.length; index += 1) {
            diffRenderNode(previous.children[index], next.children[index], ops);
        }
        return;
    }

    ops.push({ op: 'replace', target: renderNodeTarget(previous), node: structuredPatchNode(next) });
}

function diffAttributes(previous: Extract<RenderPlanNode, { kind: 'element' }>, next: Extract<RenderPlanNode, { kind: 'element' }>, ops: DomPatchOp[]): void {
    const previousAttributes = attributeRecord(previous.attributes);
    const nextAttributes = attributeRecord(next.attributes);
    const target = renderNodeTarget(previous);
    for (const name of Object.keys(previousAttributes).sort()) {
        if (!(name in nextAttributes)) {
            ops.push({ op: 'setAttribute', target, name, value: null });
        }
    }
    for (const name of Object.keys(nextAttributes).sort()) {
        if (previousAttributes[name] !== nextAttributes[name]) {
            ops.push({ op: 'setAttribute', target, name, value: nextAttributes[name] });
        }
    }
}

function attributeRecord(attributes: readonly RenderPlanAttribute[]): Record<string, string> {
    return Object.fromEntries(attributes.map((attribute) => [attribute.name, attribute.value]));
}

function renderNodeId(node: RenderPlanNode): string {
    return node.kind === 'element' ? node.renderNodeId : textNodePatchId(node);
}

function renderNodeTarget(node: RenderPlanNode): DomPatchTarget {
    return { kind: 'render-node', id: renderNodeId(node) };
}

function structuredPatchNode(node: RenderPlanNode): PatchNodePayload {
    return { encoding: 'structured-node-v1', node: serializeRenderNode(node) };
}

function serializeRenderNode(node: RenderPlanNode): SerializedNode {
    if (node.kind === 'text' || node.kind === 'comment') {
        return {
            kind: node.kind,
            renderNodeId: textNodePatchId(node),
            text: node.text,
            sourceMapRef: node.sourceMapRef,
        };
    }

    return {
        kind: 'element',
        renderNodeId: node.renderNodeId,
        tagName: node.tag,
        attributes: attributeRecord(node.attributes),
        children: node.children.map(serializeRenderNode),
        sourceMapRef: node.sourceMapRef,
    };
}

function textNodePatchId(node: Extract<RenderPlanNode, { kind: 'text' | 'comment' }>): string {
    return node.sourceMapRef?.frame ? `text:${node.sourceMapRef.frame}` : `text:${stableTextHash(node.text)}`;
}

function stableTextHash(text: string): string {
    let hash = 0;
    for (let index = 0; index < text.length; index += 1) {
        hash = (hash * 31 + text.charCodeAt(index)) >>> 0;
    }
    return hash.toString(16);
}

function stableJsonDigest(value: unknown): string {
    const canonical = stableJsonStringify(value);
    let hash = 0xcbf29ce484222325n;
    const prime = 0x100000001b3n;
    const mask = 0xffffffffffffffffn;
    for (let index = 0; index < canonical.length; index += 1) {
        hash ^= BigInt(canonical.charCodeAt(index));
        hash = (hash * prime) & mask;
    }
    return hash.toString(16).padStart(16, '0');
}

function cloneStableJsonValue(value: unknown): unknown {
    return JSON.parse(stableJsonStringify(value)) as unknown;
}

function stableJsonStringify(value: unknown): string {
    if (value === null || typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
        return JSON.stringify(value);
    }
    if (Array.isArray(value)) {
        return `[${value.map((item) => stableJsonStringify(item === undefined ? null : item)).join(',')}]`;
    }
    if (value && typeof value === 'object') {
        const record = value as Record<string, unknown>;
        const entries = Object.keys(record)
            .filter((key) => record[key] !== undefined)
            .sort()
            .map((key) => `${JSON.stringify(key)}:${stableJsonStringify(record[key])}`);
        return `{${entries.join(',')}}`;
    }
    throw new TypeError(`Edge render-state content is not JSON-serializable: ${String(value)}`);
}

function edgeRenderStateKey(revision: RenderRevision): string {
    return ['edge-state', revision.scopePolicyStamp, revision.instanceId].join(':');
}

function renderRevisionKey(revision: RenderRevision): string {
    return [
        revision.instanceId,
        revision.dataRevision,
        revision.templateArtifactId,
        revision.scopePolicyStamp,
        revision.outputTarget,
    ].join(':');
}

function patchTransactionId(plan: RenderPlan): string {
    return [
        'patch',
        plan.instanceId,
        plan.templateArtifactId,
        plan.dataRevision,
        plan.scopePolicyStamp,
    ].join(':');
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
