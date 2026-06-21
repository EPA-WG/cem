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

import {
    ingestContractVersion,
    type DispositionDecision,
    type RunMode,
} from './disposition.js';

const XHTML_NAMESPACE = 'http://www.w3.org/1999/xhtml';
const ATTRIBUTE_DECLARATION_TAG = 'attribute';
const SLICE_DECLARATION_TAG = 'slice';
const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';
const TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const SOURCE_FRAME_ATTR = 'data-cem-source-frame';
export const DATA_CEM_SCOPE_ATTR = 'data-cem-scope';
const STYLE_TAG = 'style';
const KEYFRAMES_AT_RULE = /@(-webkit-)?keyframes\s+([A-Za-z_][\w-]*)/g;
const UNSUPPORTED_SCOPED_CSS_AT_RULES = [
    'font-face',
    'property',
    'counter-style',
    'font-palette-values',
    'page',
    'namespace',
] as const;

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

/** Render-engine / patch-transport schema version (FF-6 SemVer axis, BR-VC-5). */
export const RENDER_ENGINE_VERSION = '1.0.0';

/** Edge render-state record schema version (FF-6 SemVer axis, BR-VC-5). */
export const EDGE_RENDER_STATE_VERSION = '1.0.0';

export interface RenderPlan {
    producedTag: string;
    instanceId: string;
    templateArtifactId: string;
    dataRevision: string;
    outputTarget: 'light-dom';
    scopePolicyStamp: string;
    nodes: RenderPlanNode[];
}

export interface RenderPlanDomRange {
    start: Comment;
    end: Comment;
}

export interface RenderedFragmentMergeOptions {
    preserveElementChildren?: (current: Element, desired: Element) => boolean;
}

export interface RenderPlanApplyOptions extends RenderedFragmentMergeOptions {
    dynamicTextRanges?: boolean;
    transientElementTags?: readonly string[];
}

export interface RenderPlanApplyDiagnostic {
    code: string;
    severity: 'info' | 'warning';
    reason: 'first-render' | 'recovery';
    message: string;
}

export interface RenderPlanApplyResult {
    mode: 'patch' | 'replaceScope';
    diagnostics: RenderPlanApplyDiagnostic[];
}

export interface ScopedCssRewriteDiagnostic {
    code: string;
    severity: 'warning';
    message: string;
}

export interface ScopedCssRewriteResult {
    css: string;
    diagnostics: ScopedCssRewriteDiagnostic[];
}

export interface ScopedRenderPlanResult {
    renderPlan: RenderPlan;
    diagnostics: ScopedCssRewriteDiagnostic[];
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
    | { type: 'begin'; transactionId: string; revision: RenderRevision; renderEngineVersion?: string }
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
    schemaVersion?: string;
    stateKey: string;
    producedTag: string;
    instanceId: string;
    templateArtifactId: string;
    scopePolicyStamp: string;
    privacyPolicyStamp?: string;
    renderRevision: RenderRevision;
    currentTemplateArtifact?: EdgeContentAddress;
    currentRenderPlan: EdgeContentAddress;
    currentSnapshot?: EdgeContentAddress;
    currentHtml?: EdgeContentAddress;
    etag: string;
}

export interface EdgeRenderStateInput {
    renderPlan: RenderPlan;
    templateArtifact?: unknown;
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

export type EdgeRenderStateContentField =
    | 'currentTemplateArtifact'
    | 'currentRenderPlan'
    | 'currentSnapshot'
    | 'currentHtml';

export interface EdgeRenderStateContents {
    record: EdgeRenderStateRecord;
    templateArtifact?: unknown;
    renderPlan: RenderPlan;
    sanitizedSnapshot?: unknown;
    renderedHtml?: string;
}

export type EdgeRenderStateContentsReadResult =
    | { ok: true; contents: EdgeRenderStateContents }
    | {
          ok: false;
          reason: 'schema-version-unsupported';
          record: EdgeRenderStateRecord;
          decision?: DispositionDecision;
      }
    | {
          ok: false;
          reason: 'missing-content';
          record: EdgeRenderStateRecord;
          field: EdgeRenderStateContentField;
          address: EdgeContentAddress;
      }
    | {
          ok: false;
          reason: 'content-address-mismatch';
          record: EdgeRenderStateRecord;
          field: EdgeRenderStateContentField;
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
      }
    | {
          ok: false;
          reason: 'render-revision-mismatch';
          current: EdgeRenderStateRecord;
          actual: RenderPlanIdentity;
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
    formData?: Record<string, unknown>;
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
    const frames: PatchFrame[] = [
        { type: 'begin', transactionId, revision: renderPlanIdentity(next), renderEngineVersion: RENDER_ENGINE_VERSION },
    ];

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

export function renderPlansHaveDomChanges(previous: RenderPlan | null, next: RenderPlan): boolean {
    return diffRenderPlans(previous, next).length > 0;
}

export function edgeContentAddress(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
    assertProcessingBoundaryValue(value, `${kind} content`);
    const digest = stableJsonDigest(value);
    const algorithm = 'stable-json-fnv1a64-v1';
    return {
        kind,
        algorithm,
        digest,
        key: `${kind}:${algorithm}:${digest}`,
    };
}

export function assertProcessingBoundaryValue(value: unknown, label = 'processing boundary value'): void {
    const failure = findProcessingBoundaryViolation(value, label);
    if (failure) {
        throw new TypeError(failure);
    }
}

function findProcessingBoundaryViolation(value: unknown, path: string): string | null {
    if (
        value === null ||
        value === undefined ||
        typeof value === 'string' ||
        typeof value === 'number' ||
        typeof value === 'boolean'
    ) {
        return null;
    }
    if (typeof value === 'function' || typeof value === 'symbol' || typeof value === 'bigint') {
        return `${path} contains non-transport value ${typeof value}`;
    }
    if (Array.isArray(value)) {
        for (const [index, item] of value.entries()) {
            const failure = findProcessingBoundaryViolation(item, `${path}[${index}]`);
            if (failure) {
                return failure;
            }
        }
        return null;
    }
    if (typeof value === 'object') {
        const prototype = Object.getPrototypeOf(value);
        if (prototype !== Object.prototype && prototype !== null) {
            const name = prototype?.constructor?.name ?? 'unknown';
            return `${path} contains non-plain object ${name}`;
        }
        for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
            const failure = findProcessingBoundaryViolation(item, `${path}.${key}`);
            if (failure) {
                return failure;
            }
        }
        return null;
    }
    return `${path} contains unsupported value`;
}

export function createEdgeRenderStateRecord(input: EdgeRenderStateInput): EdgeRenderStateRecord {
    const identity = renderPlanIdentity(input.renderPlan);
    const currentRenderPlan = edgeContentAddress('render-plan', input.renderPlan);
    const recordWithoutEtag = {
        storageModel: 'content-addressed-cache-with-revision-pointer-v1' as const,
        schemaVersion: EDGE_RENDER_STATE_VERSION,
        stateKey: input.stateKey ?? edgeRenderStateKey(identity),
        producedTag: input.renderPlan.producedTag,
        instanceId: input.renderPlan.instanceId,
        templateArtifactId: input.renderPlan.templateArtifactId,
        scopePolicyStamp: input.renderPlan.scopePolicyStamp,
        privacyPolicyStamp: input.privacyPolicyStamp,
        renderRevision: identity,
        currentTemplateArtifact: input.templateArtifact !== undefined
            ? edgeContentAddress('template-artifact', input.templateArtifact)
            : undefined,
        currentRenderPlan,
        currentSnapshot: input.sanitizedSnapshot !== undefined
            ? edgeContentAddress('sanitized-snapshot', input.sanitizedSnapshot)
            : undefined,
        currentHtml: input.renderedHtml !== undefined ? edgeContentAddress('rendered-html', input.renderedHtml) : undefined,
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

export function readEdgeRenderStateContents(
    store: EdgeRenderStateStore,
    record: EdgeRenderStateRecord,
    mode: RunMode = 'application'
): EdgeRenderStateContentsReadResult {
    // BR-VC-9: the edge render-state record is a data/security contract. If the
    // persisted record declares a schema version this build does not fully
    // understand (higher MINOR = unknown optional features, or a MAJOR mismatch
    // = must-understand), apply the run-mode disposition before trusting it. An
    // application/build-SSR run rejects rather than honoring/dropping unknown
    // fields from a record written by a newer engine.
    const ingest = ingestContractVersion(
        record.schemaVersion,
        EDGE_RENDER_STATE_VERSION,
        mode,
        'edge-render-state'
    );
    if (!ingest.accept) {
        return { ok: false, reason: 'schema-version-unsupported', record, decision: ingest.decision };
    }

    const renderPlan = readEdgeContent<RenderPlan>(store, record.currentRenderPlan);
    if (!renderPlan.ok) {
        return edgeContentFailureToRecordFailure(record, 'currentRenderPlan', renderPlan);
    }

    const contents: EdgeRenderStateContents = {
        record,
        renderPlan: renderPlan.value,
    };

    if (record.currentTemplateArtifact) {
        const templateArtifact = readEdgeContent(store, record.currentTemplateArtifact);
        if (!templateArtifact.ok) {
            return edgeContentFailureToRecordFailure(record, 'currentTemplateArtifact', templateArtifact);
        }
        contents.templateArtifact = templateArtifact.value;
    }
    if (record.currentSnapshot) {
        const sanitizedSnapshot = readEdgeContent(store, record.currentSnapshot);
        if (!sanitizedSnapshot.ok) {
            return edgeContentFailureToRecordFailure(record, 'currentSnapshot', sanitizedSnapshot);
        }
        contents.sanitizedSnapshot = sanitizedSnapshot.value;
    }
    if (record.currentHtml) {
        const renderedHtml = readEdgeContent<string>(store, record.currentHtml);
        if (!renderedHtml.ok) {
            return edgeContentFailureToRecordFailure(record, 'currentHtml', renderedHtml);
        }
        contents.renderedHtml = renderedHtml.value;
    }

    return { ok: true, contents };
}

function edgeContentFailureToRecordFailure(
    record: EdgeRenderStateRecord,
    field: EdgeRenderStateContentField,
    failure: Exclude<EdgeContentReadResult, { ok: true }>
): EdgeRenderStateContentsReadResult {
    if (failure.reason === 'missing-content') {
        return {
            ok: false,
            reason: 'missing-content',
            record,
            field,
            address: failure.address,
        };
    }
    return {
        ok: false,
        reason: 'content-address-mismatch',
        record,
        field,
        expected: failure.expected,
        actual: failure.actual,
    };
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
        const actualRevision = renderPlanIdentity(storedPreviousPlan.value);
        if (renderRevisionKey(actualRevision) !== renderRevisionKey(current.renderRevision)) {
            return {
                ok: false,
                reason: 'render-revision-mismatch',
                current,
                actual: actualRevision,
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
            templateArtifact: input.source,
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
        if (input.templateArtifact !== undefined) {
            this.putContent('template-artifact', input.templateArtifact);
        }
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
    return {
        ...plan,
        nodes: projectSlotNodes(plan.nodes, slotPayload),
    };
}

/**
 * Stamp a render plan with a generated scope identity and rewrite template-local
 * `<style>` nodes so light-DOM rendering gets the same containment model in the
 * browser, SSR, and edge render paths.
 */
export function scopeRenderPlan(plan: RenderPlan, scopeUid: string): ScopedRenderPlanResult {
    const diagnostics: ScopedCssRewriteDiagnostic[] = [];
    return {
        renderPlan: {
            ...plan,
            nodes: scopeRenderNodes(plan.nodes, scopeUid, diagnostics, true),
        },
        diagnostics,
    };
}

export function scopeCssText(css: string, scopeUid: string): ScopedCssRewriteResult {
    const diagnostics: ScopedCssRewriteDiagnostic[] = [];
    let scoped = css;

    scoped = scoped.replace(/@import\b[^;]*;?/gi, (statement) => {
        diagnostics.push({
            code: 'cem.scoped_css.import_unsupported',
            severity: 'warning',
            message: `scoped CSS suppresses unsupported @import statement: ${statement.trim()}`,
        });
        return '';
    });

    for (const atRule of UNSUPPORTED_SCOPED_CSS_AT_RULES) {
        const before = scoped;
        scoped = stripUnsupportedAtRule(scoped, atRule);
        if (scoped !== before) {
            diagnostics.push({
                code: 'cem.scoped_css.global_construct_unsupported',
                severity: 'warning',
                message: `scoped CSS suppresses unsupported global @${atRule} construct`,
            });
        }
    }

    const keyframeRenames = new Map<string, string>();
    scoped = scoped.replace(KEYFRAMES_AT_RULE, (_statement: string, vendor: string | undefined, name: string) => {
        const scopedName = `${name}-${cssIdentifier(scopeUid)}`;
        keyframeRenames.set(name, scopedName);
        return `@${vendor ?? ''}keyframes ${scopedName}`;
    });
    scoped = rewriteAnimationReferences(scoped, keyframeRenames);

    let globalAliasDiagnostic = false;
    scoped = scoped
        .replace(/:host\(([^)]*)\)/g, '&$1')
        .replace(/:host(?![-_A-Za-z0-9])/g, '&')
        .replace(/:global\(([^)]*)\)/g, (_match, selector: string) => {
            globalAliasDiagnostic = true;
            return `&${selector}`;
        })
        .replace(/:global(?![-_A-Za-z0-9])/g, () => {
            globalAliasDiagnostic = true;
            return '&';
        })
        .replace(/:root(?![-_A-Za-z0-9])/g, () => {
            globalAliasDiagnostic = true;
            return '&';
        });

    if (globalAliasDiagnostic) {
        diagnostics.push({
            code: 'cem.scoped_css.global_alias',
            severity: 'warning',
            message: 'scoped CSS treats :global and :root as :host aliases',
        });
    }

    const body = scoped.trim();
    return {
        css: body.length > 0 ? `[${DATA_CEM_SCOPE_ATTR}="${cssString(scopeUid)}"] {\n${indentCss(body)}\n}` : '',
        diagnostics,
    };
}

function projectSlotNodes(
    nodes: readonly RenderPlanNode[],
    payload: ProjectionPayload
): RenderPlanNode[] {
    const out: RenderPlanNode[] = [];
    for (const node of nodes) {
        if (node.kind !== 'element') {
            out.push(node);
            continue;
        }
        if (node.tag === 'slot') {
            const name = node.attributes.find((attribute) => attribute.name === 'name')?.value ?? '';
            const projected = collectProjectedSlotPayload(payload, name);
            out.push(...(projected.length > 0 ? projected : node.children));
            continue;
        }
        out.push({
            ...node,
            children: projectSlotNodes(node.children, payload),
        });
    }
    return out;
}

function collectProjectedSlotPayload(
    payload: ProjectionPayload,
    name: string
): RenderPlanNode[] {
    const projected: RenderPlanNode[] = [];
    for (const node of payload.slots?.[name] ?? []) {
        projected.push(payloadNodeToRenderNode(node));
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

function scopeRenderNodes(
    nodes: readonly RenderPlanNode[],
    scopeUid: string,
    diagnostics: ScopedCssRewriteDiagnostic[],
    stampScope: boolean
): RenderPlanNode[] {
    return nodes.map((node) => {
        if (node.kind !== 'element') {
            return node;
        }

        const attributes = stampScope
            ? withRenderPlanAttribute(node.attributes, DATA_CEM_SCOPE_ATTR, scopeUid)
            : node.attributes;
        if (node.tag === STYLE_TAG && node.namespace === null) {
            const rewritten = scopeStyleNode(node, scopeUid);
            diagnostics.push(...rewritten.diagnostics);
            return {
                ...node,
                attributes,
                children: [{
                    kind: 'text',
                    text: rewritten.css,
                    sourceMapRef: firstTextSourceMapRef(node.children) ?? node.sourceMapRef,
                }],
            };
        }

        return {
            ...node,
            attributes,
            children: scopeRenderNodes(node.children, scopeUid, diagnostics, false),
        };
    });
}

function scopeStyleNode(
    node: Extract<RenderPlanNode, { kind: 'element' }>,
    scopeUid: string
): ScopedCssRewriteResult {
    const css = node.children
        .map((child) => {
            if (child.kind === 'text') {
                return child.text;
            }
            return child.kind === 'comment' ? `/*${child.text}*/` : '';
        })
        .join('');
    return scopeCssText(css, scopeUid);
}

function firstTextSourceMapRef(nodes: readonly RenderPlanNode[]): SourceMapRef | undefined {
    for (const node of nodes) {
        if ((node.kind === 'text' || node.kind === 'comment') && node.sourceMapRef) {
            return node.sourceMapRef;
        }
    }
    return undefined;
}

function withRenderPlanAttribute(
    attributes: readonly RenderPlanAttribute[],
    name: string,
    value: string
): RenderPlanAttribute[] {
    let replaced = false;
    const next = attributes.map((attribute) => {
        if (attribute.name !== name) {
            return attribute;
        }
        replaced = true;
        return { name, value };
    });
    return replaced ? next : [...next, { name, value }];
}

function rewriteAnimationReferences(css: string, renames: ReadonlyMap<string, string>): string {
    if (renames.size === 0) {
        return css;
    }
    return css
        .replace(/((?:-webkit-)?animation-name\s*:\s*)([^;{}]+)/gi, (_match, prefix: string, value: string) =>
            `${prefix}${replaceCssValueNames(value, renames)}`
        )
        .replace(/((?:-webkit-)?animation\s*:\s*)([^;{}]+)/gi, (_match, prefix: string, value: string) =>
            `${prefix}${replaceCssValueNames(value, renames)}`
        );
}

function replaceCssValueNames(value: string, renames: ReadonlyMap<string, string>): string {
    let rewritten = value;
    for (const [name, scopedName] of renames) {
        rewritten = rewritten.replace(
            new RegExp(`(^|[^-_A-Za-z0-9])(${escapeRegExp(name)})(?=$|[^-_A-Za-z0-9])`, 'g'),
            (_match, prefix: string) => `${prefix}${scopedName}`
        );
    }
    return rewritten;
}

function stripUnsupportedAtRule(css: string, atRule: string): string {
    if (atRule === 'namespace') {
        return css.replace(/@namespace\b[^;]*;?/gi, '');
    }

    const atRulePattern = new RegExp(`@${escapeRegExp(atRule)}\\b`, 'gi');
    let output = '';
    let cursor = 0;
    let match: RegExpExecArray | null;
    while ((match = atRulePattern.exec(css)) !== null) {
        output += css.slice(cursor, match.index);
        const blockStart = css.indexOf('{', atRulePattern.lastIndex);
        const statementEnd = css.indexOf(';', atRulePattern.lastIndex);
        if (blockStart < 0 || (statementEnd >= 0 && statementEnd < blockStart)) {
            cursor = statementEnd >= 0 ? statementEnd + 1 : css.length;
            atRulePattern.lastIndex = cursor;
            continue;
        }
        const blockEnd = matchingBraceIndex(css, blockStart);
        cursor = blockEnd >= 0 ? blockEnd + 1 : css.length;
        atRulePattern.lastIndex = cursor;
    }
    return output + css.slice(cursor);
}

function matchingBraceIndex(css: string, openIndex: number): number {
    let depth = 0;
    for (let index = openIndex; index < css.length; index += 1) {
        const char = css[index];
        if (char === '{') {
            depth += 1;
        } else if (char === '}') {
            depth -= 1;
            if (depth === 0) {
                return index;
            }
        }
    }
    return -1;
}

function indentCss(css: string): string {
    return css.split(/\r?\n/).map((line) => (line.length > 0 ? `    ${line}` : '')).join('\n');
}

function cssIdentifier(value: string): string {
    const sanitized = value.toLowerCase().replace(/[^-_a-z0-9]+/g, '-').replace(/^-+|-+$/g, '');
    return sanitized.length > 0 ? sanitized : 'scope';
}

function cssString(value: string): string {
    return value.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
}

function escapeRegExp(value: string): string {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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

export function mergeRenderedFragmentIntoRange(
    bounds: RenderPlanDomRange,
    rendered: DocumentFragment,
    options: RenderedFragmentMergeOptions = {}
): void {
    const parent = bounds.start.parentNode;
    if (!parent || bounds.end.parentNode !== parent) {
        throw new Error('cem-element render bounds are not attached to the same parent');
    }
    mergeChildNodes(parent, bounds.start.nextSibling as ChildNode | null, bounds.end, Array.from(rendered.childNodes), options);
}

export function applyRenderPlanToRange(
    bounds: RenderPlanDomRange,
    plan: RenderPlan,
    document: Document,
    options: RenderPlanApplyOptions = {}
): RenderPlanApplyResult {
    const parent = bounds.start.parentNode;
    if (!parent || bounds.end.parentNode !== parent) {
        throw new Error('cem-element render bounds are not attached to the same parent');
    }

    const recovery = renderScopeRecoveryReason(bounds, plan, options);
    if (recovery) {
        replaceRangeWithRenderPlan(bounds, plan, document, options);
        return {
            mode: 'replaceScope',
            diagnostics: [{
                code: 'cem.render_plan_apply.replace_scope',
                severity: recovery.reason === 'first-render' ? 'info' : 'warning',
                reason: recovery.reason,
                message: recovery.message,
            }],
        };
    }

    const context: RenderPlanApplyContext = { plan, document, options };
    mergeRenderPlanChildNodes(parent, bounds.start.nextSibling as ChildNode | null, bounds.end, plan.nodes, context);
    return { mode: 'patch', diagnostics: [] };
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
    (element as Element & { cemRenderNodeId?: string }).cemRenderNodeId = node.renderNodeId;
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

interface RenderPlanApplyContext {
    plan: RenderPlan;
    document: Document;
    options: RenderPlanApplyOptions;
}

interface RenderPlanNodeMatch {
    first: ChildNode;
    after: ChildNode | null;
    rangeEnd?: Comment;
}

interface RenderScopeRecoveryReason {
    reason: 'first-render' | 'recovery';
    message: string;
}

function replaceRangeWithRenderPlan(
    bounds: RenderPlanDomRange,
    plan: RenderPlan,
    document: Document,
    options: RenderPlanApplyOptions
): void {
    let current = bounds.start.nextSibling;
    while (current && current !== bounds.end) {
        const next = current.nextSibling;
        current.parentNode?.removeChild(current);
        current = next;
    }
    const nodes = plan.nodes.flatMap((node) => createRenderPlanDomNodes(node, { plan, document, options }));
    for (const node of nodes) {
        bounds.end.parentNode?.insertBefore(node, bounds.end);
    }
}

function renderScopeRecoveryReason(
    bounds: RenderPlanDomRange,
    plan: RenderPlan,
    options: RenderPlanApplyOptions
): RenderScopeRecoveryReason | undefined {
    const currentIds = elementRenderIdentitiesBetween(bounds.start.nextSibling, bounds.end);
    const desiredIds = plan.nodes.flatMap((node) =>
        node.kind === 'element' && !isTransientRenderPlanElement(node, options) ? [node.renderNodeId] : []
    );
    if (currentIds.length === 0) {
        return undefined;
    }
    if (desiredIds.length === 0) {
        return {
            reason: 'recovery',
            message: 'retained render scope had element identities but the next render plan has no element roots',
        };
    }
    const current = new Set(currentIds);
    const desired = new Set(desiredIds);
    if (desiredIds.some((id) => !current.has(id)) || currentIds.some((id) => !desired.has(id))) {
        return {
            reason: 'recovery',
            message: 'retained render scope root identities did not match the next render plan; replaced the scope',
        };
    }
    return undefined;
}

function isTransientRenderPlanElement(
    node: Extract<RenderPlanNode, { kind: 'element' }>,
    options: RenderPlanApplyOptions
): boolean {
    return options.transientElementTags?.includes(node.tag) ?? false;
}

function elementRenderIdentitiesBetween(first: ChildNode | null, end: Node): string[] {
    const ids: string[] = [];
    let current: ChildNode | null = first;
    while (current && current !== end) {
        if (current.nodeType === 1) {
            const id = renderIdentity(current);
            if (id) {
                ids.push(id);
            }
        }
        current = current.nextSibling as ChildNode | null;
    }
    return ids;
}

function mergeRenderPlanChildNodes(
    parent: Node,
    firstCurrent: ChildNode | null,
    end: Node | null,
    desiredNodes: readonly RenderPlanNode[],
    context: RenderPlanApplyContext
): void {
    let current: ChildNode | null = firstCurrent;
    for (const desired of desiredNodes) {
        const match = matchRenderPlanNode(current, end, desired, context);
        if (match) {
            if (match.first !== current) {
                parent.insertBefore(match.first, current ?? end);
            }
            mergeRenderPlanNode(match, desired, context);
            current = match.after;
            continue;
        }

        const created = createRenderPlanDomNodes(desired, context);
        for (const node of created) {
            parent.insertBefore(node, current ?? end);
        }
    }

    while (current && current !== end) {
        const next = current.nextSibling as ChildNode | null;
        parent.removeChild(current);
        current = next;
    }
}

function matchRenderPlanNode(
    current: ChildNode | null,
    end: Node | null,
    desired: RenderPlanNode,
    context: RenderPlanApplyContext
): RenderPlanNodeMatch | null {
    if (!current || current === end) {
        return null;
    }
    const direct = matchRenderPlanNodeAt(current, desired, context);
    if (direct) {
        return direct;
    }

    const desiredId = desired.kind === 'element' ? desired.renderNodeId : dynamicRangeId(desired);
    let sibling = current.nextSibling as ChildNode | null;
    while (sibling && sibling !== end) {
        const match = matchRenderPlanNodeAt(sibling, desired, context);
        if (match && (desired.kind !== 'element' || renderIdentity(sibling) === desiredId)) {
            return match;
        }
        sibling = sibling.nextSibling as ChildNode | null;
    }
    return null;
}

function matchRenderPlanNodeAt(
    current: ChildNode,
    desired: RenderPlanNode,
    context: RenderPlanApplyContext
): RenderPlanNodeMatch | null {
    if (desired.kind === 'text' || desired.kind === 'comment') {
        if (context.options.dynamicTextRanges) {
            const rangeEnd = matchDynamicRange(current, dynamicRangeId(desired));
            return rangeEnd ? { first: current, after: rangeEnd.nextSibling as ChildNode | null, rangeEnd } : null;
        }
        const desiredType = desired.kind === 'text' ? 3 : 8;
        return current.nodeType === desiredType ? { first: current, after: current.nextSibling as ChildNode | null } : null;
    }

    if (current.nodeType !== 1) {
        return null;
    }
    const element = current as Element;
    if (
        element.localName !== desired.tag ||
        !renderPlanNamespaceMatches(element.namespaceURI, desired.namespace) ||
        renderIdentity(element) !== desired.renderNodeId
    ) {
        return null;
    }
    return { first: current, after: current.nextSibling as ChildNode | null };
}

function mergeRenderPlanNode(match: RenderPlanNodeMatch, desired: RenderPlanNode, context: RenderPlanApplyContext): void {
    if (desired.kind === 'text' || desired.kind === 'comment') {
        if (match.rangeEnd) {
            mergeDynamicRange(match.first as Comment, match.rangeEnd, desired, context);
            return;
        }
        const value = desired.kind === 'text' ? desired.text : desired.text;
        if (match.first.nodeValue !== value) {
            match.first.nodeValue = value;
        }
        return;
    }

    const element = match.first as Element;
    mirrorRenderIdentity(element, desired.renderNodeId);
    syncAttributes(element, renderPlanElementAttributes(desired, context.plan));
    if (context.options.preserveElementChildren?.(element, renderPlanElementPreview(desired, context))) {
        return;
    }
    mergeRenderPlanChildNodes(element, element.firstChild as ChildNode | null, null, desired.children, context);
}

function mergeDynamicRange(start: Comment, end: Comment, desired: Extract<RenderPlanNode, { kind: 'text' | 'comment' }>, context: RenderPlanApplyContext): void {
    const desiredType = desired.kind === 'text' ? 3 : 8;
    let current = start.nextSibling as ChildNode | null;
    if (current && current !== end && current.nodeType === desiredType) {
        if (current.nodeValue !== desired.text) {
            current.nodeValue = desired.text;
        }
        current = current.nextSibling as ChildNode | null;
    } else {
        start.parentNode?.insertBefore(createTextLikeNode(desired, context.document), end);
    }
    while (current && current !== end) {
        const next = current.nextSibling as ChildNode | null;
        current.parentNode?.removeChild(current);
        current = next;
    }
}

function createRenderPlanDomNodes(node: RenderPlanNode, context: RenderPlanApplyContext): Node[] {
    if (node.kind === 'text' || node.kind === 'comment') {
        if (!context.options.dynamicTextRanges) {
            return [createTextLikeNode(node, context.document)];
        }
        const id = dynamicRangeId(node);
        return [
            context.document.createComment(`cem-start:${id}`),
            createTextLikeNode(node, context.document),
            context.document.createComment(`cem-end:${id}`),
        ];
    }

    const element = createRenderPlanElement(node, context.plan, context.document);
    for (const child of node.children) {
        for (const childNode of createRenderPlanDomNodes(child, context)) {
            element.appendChild(childNode);
        }
    }
    return [element];
}

function createTextLikeNode(node: Extract<RenderPlanNode, { kind: 'text' | 'comment' }>, document: Document): Node {
    return node.kind === 'text' ? document.createTextNode(node.text) : document.createComment(node.text);
}

function createRenderPlanElement(
    node: Extract<RenderPlanNode, { kind: 'element' }>,
    plan: RenderPlan,
    document: Document
): Element {
    const element = node.namespace
        ? document.createElementNS(node.namespace, node.tag)
        : document.createElement(node.tag);
    syncAttributes(element, renderPlanElementAttributes(node, plan));
    mirrorRenderIdentity(element, node.renderNodeId);
    return element;
}

function renderPlanElementAttributes(
    node: Extract<RenderPlanNode, { kind: 'element' }>,
    plan: RenderPlan
): Map<string, string> {
    const attributes = new Map(node.attributes.map((attribute) => [attribute.name, attribute.value]));
    attributes.set(RENDER_NODE_ID_ATTR, node.renderNodeId);
    attributes.set(TEMPLATE_ARTIFACT_ID_ATTR, plan.templateArtifactId);
    attributes.set(DATA_REVISION_ATTR, plan.dataRevision);
    if (node.sourceMapRef) {
        attributes.set(SOURCE_FIDELITY_ATTR, node.sourceMapRef.fidelity);
        attributes.set(SOURCE_FRAME_ATTR, node.sourceMapRef.frame);
    }
    return attributes;
}

function renderPlanElementPreview(
    node: Extract<RenderPlanNode, { kind: 'element' }>,
    context: RenderPlanApplyContext
): Element {
    return createRenderPlanElement(node, context.plan, context.document);
}

function matchDynamicRange(current: ChildNode, id: string): Comment | null {
    if (!isDynamicRangeBoundary(current, 'start', id)) {
        return null;
    }
    let sibling = current.nextSibling as ChildNode | null;
    while (sibling) {
        if (isDynamicRangeBoundary(sibling, 'end', id)) {
            return sibling as Comment;
        }
        sibling = sibling.nextSibling as ChildNode | null;
    }
    return null;
}

function isDynamicRangeBoundary(node: Node, kind: 'start' | 'end', id: string): boolean {
    return node.nodeType === 8 && node.nodeValue === `cem-${kind}:${id}`;
}

function dynamicRangeId(node: Extract<RenderPlanNode, { kind: 'text' | 'comment' }>): string {
    return textNodePatchId(node);
}

function renderPlanNamespaceMatches(actual: string | null, expected: string | null): boolean {
    return actual === expected || (expected === null && actual === XHTML_NAMESPACE);
}

function mergeChildNodes(
    parent: Node,
    firstCurrent: ChildNode | null,
    end: Node | null,
    desiredNodes: readonly Node[],
    options: RenderedFragmentMergeOptions
): void {
    let current: ChildNode | null = firstCurrent;
    for (const desired of desiredNodes) {
        const matched = matchMergeNode(current, end, desired);
        if (matched) {
            if (matched !== current) {
                parent.insertBefore(matched, current ?? end);
            }
            mergeNode(matched, desired, options);
            current = matched.nextSibling as ChildNode | null;
            continue;
        }

        parent.insertBefore(desired, current ?? end);
    }

    while (current && current !== end) {
        const next = current.nextSibling as ChildNode | null;
        parent.removeChild(current);
        current = next;
    }
}

function matchMergeNode(current: ChildNode | null, end: Node | null, desired: Node): ChildNode | null {
    if (!current || current === end) {
        return null;
    }
    if (canMergeNode(current, desired)) {
        return current;
    }

    const desiredId = renderIdentity(desired);
    if (!desiredId) {
        return null;
    }

    let sibling = current.nextSibling as ChildNode | null;
    while (sibling && sibling !== end) {
        if (renderIdentity(sibling) === desiredId && canMergeNode(sibling, desired)) {
            return sibling;
        }
        sibling = sibling.nextSibling as ChildNode | null;
    }
    return null;
}

function canMergeNode(current: Node, desired: Node): boolean {
    if (current.nodeType !== desired.nodeType) {
        return false;
    }
    if (current.nodeType !== 1) {
        return true;
    }

    const currentElement = current as Element;
    const desiredElement = desired as Element;
    if (
        currentElement.localName !== desiredElement.localName ||
        currentElement.namespaceURI !== desiredElement.namespaceURI
    ) {
        return false;
    }

    const desiredId = renderIdentity(desiredElement);
    return !desiredId || renderIdentity(currentElement) === desiredId;
}

function mergeNode(current: Node, desired: Node, options: RenderedFragmentMergeOptions): void {
    if (current.nodeType === 3 || current.nodeType === 8) {
        if (current.nodeValue !== desired.nodeValue) {
            current.nodeValue = desired.nodeValue;
        }
        return;
    }
    if (current.nodeType !== 1 || desired.nodeType !== 1) {
        current.parentNode?.replaceChild(desired, current);
        return;
    }

    const currentElement = current as Element;
    const desiredElement = desired as Element;
    const desiredId = renderIdentity(desiredElement);
    if (desiredId) {
        mirrorRenderIdentity(currentElement, desiredId);
    }
    syncAttributes(currentElement, desiredElement);
    if (options.preserveElementChildren?.(currentElement, desiredElement)) {
        return;
    }
    mergeChildNodes(
        currentElement,
        currentElement.firstChild as ChildNode | null,
        null,
        Array.from(desiredElement.childNodes),
        options
    );
}

function syncAttributes(current: Element, desired: Element | ReadonlyMap<string, string>): void {
    const desiredAttributes = isAttributeElement(desired)
        ? new Map(Array.from(desired.attributes).map((attribute) => [attribute.name, attribute.value]))
        : desired;
    for (const attribute of Array.from(current.attributes)) {
        if (!desiredAttributes.has(attribute.name)) {
            current.removeAttribute(attribute.name);
        }
    }
    for (const [name, value] of desiredAttributes) {
        if (current.getAttribute(name) !== value) {
            current.setAttribute(name, value);
        }
    }
}

function isAttributeElement(value: Element | ReadonlyMap<string, string>): value is Element {
    return 'attributes' in value && typeof value.getAttribute === 'function';
}

function renderIdentity(node: Node): string | null {
    if (node.nodeType !== 1) {
        return null;
    }
    const element = node as Element & { cemRenderNodeId?: string };
    if (element.cemRenderNodeId) {
        return element.cemRenderNodeId;
    }
    const serialized = element.getAttribute(RENDER_NODE_ID_ATTR);
    if (serialized) {
        element.cemRenderNodeId = serialized;
        return serialized;
    }
    return null;
}

function mirrorRenderIdentity(element: Element, id: string): void {
    (element as Element & { cemRenderNodeId?: string }).cemRenderNodeId = id;
    if (element.getAttribute(RENDER_NODE_ID_ATTR) !== id) {
        element.setAttribute(RENDER_NODE_ID_ATTR, id);
    }
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
    assertProcessingBoundaryValue(value);
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

function isTopLevelNonOutputNode(node: TemplateSourceNode): boolean {
    if (node.kind === 'element') {
        return node.tag === ATTRIBUTE_DECLARATION_TAG || node.tag === SLICE_DECLARATION_TAG;
    }
    return node.kind === 'text' && node.text.trim().length === 0;
}
