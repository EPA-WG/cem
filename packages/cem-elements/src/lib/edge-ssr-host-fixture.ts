import type {
    DataIslandSnapshot,
    SerializedPayloadElement,
} from './cem-elements.js';
import {
    projectTemplate,
    readEdgeRenderStateContents,
    renderPlanIdentity,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type EdgeRenderStateStore,
    type RenderPlan,
    type RenderPlanIdentity,
    type RenderPlanNode,
    type RenderRevision,
    type TemplateSourceNode,
    type TemplateValue,
} from './projection.js';
import {
    assertCemEdgeSsrHostEnvelope,
    createCemEdgeSsrHostFailureEnvelope,
    createCemEdgeSsrHostSuccessEnvelope,
    type CemEdgeSsrHostFailureEnvelope,
    type CemEdgeSsrHostRequestEnvelope,
    type CemEdgeSsrHostSuccessEnvelope,
} from './edge-ssr-host.js';

const RENDER_NODE_ID_ATTR = 'data-cem-render-node-id';
const TEMPLATE_ARTIFACT_ID_ATTR = 'data-cem-template-artifact-id';
const DATA_REVISION_ATTR = 'data-cem-data-revision';
const SOURCE_FIDELITY_ATTR = 'data-cem-source-fidelity';
const SOURCE_FRAME_ATTR = 'data-cem-source-frame';
const HTML_NAME = /^[A-Za-z][A-Za-z0-9._:-]*$/;
const HTML_ATTRIBUTE_NAME_FORBIDDEN = new Set(['"', "'", '<', '>', '/', '=']);
const HTML_VOID_ELEMENTS = new Set([
    'area',
    'base',
    'br',
    'col',
    'embed',
    'hr',
    'img',
    'input',
    'link',
    'meta',
    'param',
    'source',
    'track',
    'wbr',
]);
const UNSUPPORTED_RAW_TEXT_ELEMENTS = new Set([
    'iframe',
    'noembed',
    'noframes',
    'noscript',
    'plaintext',
    'script',
    'xmp',
]);

export type NonBrowserSsrInitialRenderFixtureResult =
    | CemEdgeSsrHostSuccessEnvelope<'render-initial'>
    | CemEdgeSsrHostFailureEnvelope<'render-initial'>;

/**
 * Node-only Phase 3.5 evidence host. It intentionally supports serialized
 * template source only; artifact resolution belongs to later deployment adapters.
 */
export function executeNonBrowserSsrInitialRenderFixture(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    store: EdgeRenderStateStore
): NonBrowserSsrInitialRenderFixtureResult {
    assertCemEdgeSsrHostEnvelope(request);

    const identityFailure = initialRenderIdentityFailure(request);
    if (identityFailure) {
        return fixtureFailure(request, 'invalid-request', 'cem.edge_ssr.initial_identity_invalid', identityFailure);
    }
    if (!isCompleteRenderSnapshot(request.payload.snapshot)) {
        return fixtureFailure(
            request,
            'privacy-policy-rejected',
            'cem.edge_ssr.snapshot_fields_unavailable',
            'the sanitized snapshot omits fields required to produce hydration-safe initial HTML'
        );
    }
    if (request.payload.template.kind !== 'serialized-template-source-v1') {
        return fixtureFailure(
            request,
            'content-unavailable',
            'cem.edge_ssr.template_source_unavailable',
            'the non-browser SSR fixture accepts serialized template source only'
        );
    }

    try {
        const snapshot = request.payload.snapshot;
        const projected = projectTemplate(request.payload.template.source, {
            snapshot,
            values: templateValues(snapshot, request.payload.template.source),
        });
        const sourceMapped = request.payload.sourceMapMode === 'dev'
            ? projected
            : stripRenderPlanSourceMaps(projected);
        const scoped = scopeRenderPlan(sourceMapped, request.payload.scopeUid, {
            instanceScopeUid: request.payload.instanceScopeUid,
        });
        const plan = scoped.renderPlan;
        const identity = renderPlanIdentity(plan);
        if (!sameRenderPlanIdentity(identity, {
            producedTag: snapshot.producedTag,
            ...request.payload.revision,
        })) {
            return fixtureFailure(
                request,
                'render-failed',
                'cem.edge_ssr.render_plan_identity_invalid',
                'the projected render-plan identity did not match the validated initial-render request'
            );
        }

        const renderedHtml = serializeRenderPlanToHtmlFixture(plan);
        const stateInput = {
            renderPlan: plan,
            templateArtifact: request.payload.template.source,
            sanitizedSnapshot: snapshot,
            renderedHtml,
            privacyPolicyStamp: snapshot.privacyPolicyStamp,
        };
        const write = store.writeRenderState(stateInput, { ifAbsent: true });
        if (!write.ok) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'render-state-conflict',
                [fixtureDiagnostic(
                    'cem.edge_ssr.initial_state_exists',
                    'initial render refused to replace an existing render-state pointer'
                )],
                write.current
            );
        }
        const retained = readEdgeRenderStateContents(store, write.record);
        if (!retained.ok) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'content-unavailable',
                [fixtureDiagnostic(
                    'cem.edge_ssr.retained_render_state_unavailable',
                    `the committed render state could not be verified: ${retained.reason}`
                )],
                write.record
            );
        }
        if (!sameRenderPlanIdentity(renderPlanIdentity(retained.contents.renderPlan), identity)) {
            return createCemEdgeSsrHostFailureEnvelope(
                request,
                'failure',
                'render-failed',
                [fixtureDiagnostic(
                    'cem.edge_ssr.retained_render_plan_identity_invalid',
                    'the retained render plan did not preserve the initial-render identity'
                )],
                write.record
            );
        }

        return createCemEdgeSsrHostSuccessEnvelope(request, {
            kind: 'initial-render',
            renderedHtml,
            hydrationMetadata: {
                kind: 'cem-ssr-hydration-v1',
                snapshot,
                revision: request.payload.revision,
                renderPlanIdentity: identity,
                sourceMapMode: request.payload.sourceMapMode,
            },
            renderState: write.record,
            diagnostics: [
                ...scoped.diagnostics.map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
                ...validateRenderPlanGeneratedIds(plan).map((diagnostic) => ({
                    code: diagnostic.code,
                    severity: diagnostic.severity,
                    message: diagnostic.message,
                })),
            ],
        });
    } catch (error) {
        return fixtureFailure(
            request,
            'render-failed',
            'cem.edge_ssr.initial_render_failed',
            error instanceof Error ? error.message : String(error)
        );
    }
}

/** Serialize the owned render range without constructing or reading browser DOM. */
export function serializeRenderPlanToHtmlFixture(plan: RenderPlan): string {
    return plan.nodes.map((node) => serializeRenderNode(node, plan)).join('');
}

function initialRenderIdentityFailure(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>
): string | undefined {
    const { revision, snapshot, sourceMapMode, template, scopeUid, instanceScopeUid } = request.payload;
    if (
        !isPlainRecord(revision)
        || !isPlainRecord(snapshot)
        || !isPlainRecord(template)
        || typeof template.templateArtifactId !== 'string'
        || typeof snapshot.templateArtifactId !== 'string'
        || typeof snapshot.instanceId !== 'string'
        || typeof snapshot.dataRevision !== 'string'
        || typeof snapshot.scopePolicyStamp !== 'string'
        || typeof snapshot.producedTag !== 'string'
        || typeof snapshot.privacyPolicyStamp !== 'string'
        || typeof revision.templateArtifactId !== 'string'
        || typeof revision.instanceId !== 'string'
        || typeof revision.dataRevision !== 'string'
        || typeof revision.scopePolicyStamp !== 'string'
        || revision.outputTarget !== 'light-dom'
        || snapshot.outputTarget !== 'light-dom'
        || (sourceMapMode !== 'dev' && sourceMapMode !== 'prod')
        || typeof scopeUid !== 'string'
        || typeof instanceScopeUid !== 'string'
    ) {
        return 'the initial-render request is missing required identity fields';
    }
    if (
        template.kind !== 'serialized-template-source-v1'
        && template.kind !== 'compiled-template-artifact-v1'
        && template.kind !== 'content-addressed-template-artifact-v1'
    ) {
        return 'the initial-render request names an unsupported template input kind';
    }
    if (template.templateArtifactId !== snapshot.templateArtifactId) {
        return 'template input and snapshot template artifact identities differ';
    }
    if (revision.templateArtifactId !== snapshot.templateArtifactId) {
        return 'render revision and snapshot template artifact identities differ';
    }
    if (revision.instanceId !== snapshot.instanceId) {
        return 'render revision and snapshot instance identities differ';
    }
    if (revision.dataRevision !== snapshot.dataRevision) {
        return 'render revision and snapshot data revisions differ';
    }
    if (revision.scopePolicyStamp !== snapshot.scopePolicyStamp) {
        return 'render revision and snapshot scope-policy stamps differ';
    }
    if (revision.outputTarget !== snapshot.outputTarget) {
        return 'render revision and snapshot output targets differ';
    }
    if ((revision.renderAttempt ?? undefined) !== (snapshot.renderAttempt ?? undefined)) {
        return 'render revision and snapshot render attempts differ';
    }
    if (snapshot.sourceMapMode !== sourceMapMode) {
        return 'request and snapshot source-map modes differ or are incomplete';
    }
    if (
        snapshot.producedTag.length === 0
        || snapshot.privacyPolicyStamp.length === 0
        || scopeUid.length === 0
        || instanceScopeUid.length === 0
    ) {
        return 'produced-tag, privacy-policy, and scope identities must be non-empty';
    }
    const snapshotScopeUid = snapshot.hostAttributes?.['data-cem-scope'];
    if (typeof snapshotScopeUid === 'string' && snapshotScopeUid !== scopeUid) {
        return 'snapshot and request scope identities differ';
    }
    return undefined;
}

function isCompleteRenderSnapshot(value: unknown): value is DataIslandSnapshot {
    if (!isPlainRecord(value)) {
        return false;
    }
    const payload = value.payload;
    return (
        isPlainRecord(value.hostAttributes)
        && isPlainRecord(value.dataset)
        && isPlainRecord(payload)
        && typeof payload.text === 'string'
        && typeof payload.childCount === 'number'
        && Array.isArray(payload.nodes)
        && isPlainRecord(payload.slots)
        && isPlainRecord(payload.elementsByAttribute)
        && Array.isArray(payload.data)
        && Array.isArray(payload.options)
        && isPlainRecord(payload.dataByValue)
        && isPlainRecord(payload.optionsByValue)
        && isPlainRecord(value.slices)
        && (value.formData === undefined || isPlainRecord(value.formData))
        && isPlainRecord(value.validationState)
        && isPlainRecord(value.eventPayloads)
    );
}

function templateValues(
    snapshot: DataIslandSnapshot,
    source: readonly TemplateSourceNode[]
): Record<string, TemplateValue> {
    const values: Record<string, TemplateValue> = {};
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
        values[name] = text.length > 0 ? text : null;
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

function dataDocumentFromSnapshot(snapshot: DataIslandSnapshot): Record<string, unknown> {
    return {
        attributes: snapshot.hostAttributes,
        dataset: snapshot.dataset,
        elementsByAttribute: dataDocumentElementsByAttribute(snapshot),
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

function addTemplateValuePaths(
    values: Record<string, TemplateValue>,
    prefix: string,
    value: unknown
): void {
    if (
        value === null
        || typeof value === 'string'
        || typeof value === 'boolean'
        || typeof value === 'number'
        || typeof value === 'undefined'
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

function toTemplateValue(value: unknown): TemplateValue {
    if (value === null || typeof value === 'string' || typeof value === 'boolean') {
        return value;
    }
    if (value === undefined) {
        return null;
    }
    return String(value);
}

function stripRenderPlanSourceMaps(plan: RenderPlan): RenderPlan {
    return {
        ...plan,
        nodes: plan.nodes.map(stripRenderNodeSourceMaps),
    };
}

function stripRenderNodeSourceMaps(node: RenderPlanNode): RenderPlanNode {
    if (node.kind === 'text' || node.kind === 'comment') {
        const { sourceMapRef: _sourceMapRef, ...plain } = node;
        return plain;
    }
    const { sourceMapRef: _sourceMapRef, ...plain } = node;
    return {
        ...plain,
        children: node.children.map(stripRenderNodeSourceMaps),
    };
}

function serializeRenderNode(node: RenderPlanNode, plan: RenderPlan): string {
    if (node.kind === 'text') {
        assertNoNullCharacter(node.text, 'rendered text');
        return escapeHtmlText(node.text);
    }
    if (node.kind === 'comment') {
        assertNoNullCharacter(node.text, 'rendered comment');
        if (node.text.includes('--') || node.text.endsWith('-')) {
            throw new TypeError('rendered comments may not contain `--` or end with `-`');
        }
        return `<!--${node.text}-->`;
    }

    if (!HTML_NAME.test(node.tag)) {
        throw new TypeError(`rendered element name is not safely serializable: ${node.tag}`);
    }
    const lowerTag = node.tag.toLowerCase();
    if (node.namespace === null && UNSUPPORTED_RAW_TEXT_ELEMENTS.has(lowerTag)) {
        throw new TypeError(`rendered ${lowerTag} elements are not supported by the SSR fixture`);
    }
    const attributes = new Map(node.attributes.map((attribute) => [attribute.name, attribute.value]));
    attributes.set(RENDER_NODE_ID_ATTR, node.renderNodeId);
    attributes.set(TEMPLATE_ARTIFACT_ID_ATTR, plan.templateArtifactId);
    attributes.set(DATA_REVISION_ATTR, plan.dataRevision);
    if (node.sourceMapRef) {
        attributes.set(SOURCE_FIDELITY_ATTR, node.sourceMapRef.fidelity);
        attributes.set(SOURCE_FRAME_ATTR, node.sourceMapRef.frame);
    }
    const serializedAttributes = Array.from(attributes, ([name, value]) => {
        if (!isSafeHtmlAttributeName(name)) {
            throw new TypeError(`rendered attribute name is not safely serializable: ${name}`);
        }
        assertNoNullCharacter(value, `rendered attribute ${name}`);
        return ` ${name}="${escapeHtmlAttribute(value)}"`;
    }).join('');
    if (node.namespace === null && HTML_VOID_ELEMENTS.has(lowerTag)) {
        if (node.children.length > 0) {
            throw new TypeError(`rendered void element ${lowerTag} may not contain children`);
        }
        return `<${node.tag}${serializedAttributes}>`;
    }
    const children = node.namespace === null && lowerTag === 'style'
        ? serializeStyleChildren(node.children)
        : node.children.map((child) => serializeRenderNode(child, plan)).join('');
    return `<${node.tag}${serializedAttributes}>${children}</${node.tag}>`;
}

function serializeStyleChildren(children: readonly RenderPlanNode[]): string {
    return children.map((child) => {
        if (child.kind !== 'text') {
            throw new TypeError('rendered style elements may contain text only in the SSR fixture');
        }
        assertNoNullCharacter(child.text, 'rendered style text');
        if (/<\/style/i.test(child.text)) {
            throw new TypeError('rendered style text may not contain a closing style tag');
        }
        return child.text;
    }).join('');
}

function escapeHtmlText(value: string): string {
    return value.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}

function isSafeHtmlAttributeName(value: string): boolean {
    return value.length > 0 && Array.from(value).every(
        (character) => character.charCodeAt(0) > 0x20 && !HTML_ATTRIBUTE_NAME_FORBIDDEN.has(character)
    );
}

function escapeHtmlAttribute(value: string): string {
    return escapeHtmlText(value).replaceAll('"', '&quot;');
}

function assertNoNullCharacter(value: string, label: string): void {
    if (value.includes('\0')) {
        throw new TypeError(`${label} may not contain a null character`);
    }
}

function sameRenderPlanIdentity(left: RenderPlanIdentity, right: RenderPlanIdentity): boolean {
    return (
        left.producedTag === right.producedTag
        && sameRenderRevision(left, right)
    );
}

function sameRenderRevision(left: RenderRevision, right: RenderRevision): boolean {
    return (
        left.instanceId === right.instanceId
        && left.dataRevision === right.dataRevision
        && left.templateArtifactId === right.templateArtifactId
        && left.scopePolicyStamp === right.scopePolicyStamp
        && left.outputTarget === right.outputTarget
        && (left.renderAttempt ?? undefined) === (right.renderAttempt ?? undefined)
    );
}

function fixtureFailure(
    request: CemEdgeSsrHostRequestEnvelope<'render-initial'>,
    reason: Exclude<CemEdgeSsrHostFailureEnvelope<'render-initial'>['reason'], 'cancelled'>,
    code: string,
    message: string
): CemEdgeSsrHostFailureEnvelope<'render-initial'> {
    return createCemEdgeSsrHostFailureEnvelope(
        request,
        'failure',
        reason,
        [fixtureDiagnostic(code, message)]
    );
}

function fixtureDiagnostic(code: string, message: string) {
    return {
        code,
        severity: 'error' as const,
        message,
    };
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
    return Boolean(value && typeof value === 'object' && !Array.isArray(value));
}
