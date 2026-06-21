import { describe, expect, it } from 'vitest';

import {
    SNAPSHOT_SCHEMA_VERSION,
    exportDataIslandSnapshotForEdge,
    generateScopeUid,
    type DataIslandSnapshot,
} from './cem-elements.js';
import {
    InMemoryEdgeRenderStateStore,
    assertProcessingBoundaryValue,
    createEdgeRenderStateRecord,
    diffRenderPlansToPatchFrames,
    projectTemplate,
    readEdgeRenderStateContents,
    renderPlanIdentity,
    renderPlansHaveDomChanges,
    renderInstanceScopeUid,
    scopeCssText,
    scopeRenderPlan,
    validateRenderPlanGeneratedIds,
    type PatchFrame,
    type RenderPlan,
    type TemplateProjectionInput,
    type TemplateSourceNode,
} from './projection.js';

class HostClassInstance {
    constructor(readonly value: string) {}
}

class BrowserHandleLike {
    readonly elementId = 'handle-1';
}

const TEMPLATE_SOURCE: TemplateSourceNode[] = [
    {
        kind: 'element',
        namespace: null,
        tag: 'article',
        attributes: [{ name: 'data-label', value: '{$label}' }],
        sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0' },
        children: [
            {
                kind: 'element',
                namespace: null,
                tag: 'slot',
                attributes: [{ name: 'name', value: 'detail' }],
                sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0' },
                children: [{ kind: 'text', text: 'fallback', sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0/0' } }],
            },
        ],
    },
];

describe('host processing boundary contracts', () => {
    it('keeps snapshots, render plans, patch frames, and edge records plain structured-clone data', () => {
        const snapshot = snapshotFixture();
        const exported = exportDataIslandSnapshotForEdge(snapshot, {
            privacyPolicyStamp: 'edge-policy-v1',
            fields: {
                hostAttributes: 'allow',
                dataset: 'allow',
                payload: 'allow',
                slices: 'allow',
                formData: 'allow',
                validationState: 'allow',
                eventPayloads: 'allow',
            },
        });
        expectPlainBoundaryValue(exported);
        expect((exported.formData as Record<string, unknown>).signin).toEqual({ username: 'ada' });
        expect((exported.slices as Record<string, unknown>).date).toBe('2026-06-17T00:00:00.000Z');
        expect((exported.slices as Record<string, unknown>).klass).toEqual({ value: 'class-value' });
        expect((exported.eventPayloads as Record<string, unknown>).fn).toBeUndefined();

        const projection: TemplateProjectionInput = {
            snapshot,
            values: { label: 'Projected' },
        };
        const plan = projectTemplate(TEMPLATE_SOURCE, projection);
        expectPlainBoundaryValue(plan);
        expect(plan.nodes[0]?.kind).toBe('element');

        const nextPlan: RenderPlan = {
            ...plan,
            dataRevision: '2',
            nodes: [{
                ...plan.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>,
                attributes: [{ name: 'data-label', value: 'Updated' }],
                children: [{ kind: 'text', text: 'Updated', sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0' } }],
            }],
        };
        const frames = diffRenderPlansToPatchFrames(plan, nextPlan, { transactionId: 'boundary-tx', batchSize: 1 });
        expectPlainBoundaryValue(frames);
        expect(frames.map((frame) => frame.type)).toContain('ops');

        const store = new InMemoryEdgeRenderStateStore();
        const write = store.writeRenderState({
            renderPlan: nextPlan,
            templateArtifact: TEMPLATE_SOURCE,
            sanitizedSnapshot: exported,
            renderedHtml: '<article>Updated</article>',
            privacyPolicyStamp: exported.privacyPolicyStamp,
        });
        expect(write.ok).toBe(true);
        if (!write.ok) return;
        expectPlainBoundaryValue(write.record);

        const contents = readEdgeRenderStateContents(store, write.record);
        expect(contents.ok).toBe(true);
        if (!contents.ok) return;
        expectPlainBoundaryValue(contents.contents);
        expect(renderPlanIdentity(contents.contents.renderPlan)).toEqual(renderPlanIdentity(nextPlan));

        const record = createEdgeRenderStateRecord({
            renderPlan: nextPlan,
            templateArtifact: TEMPLATE_SOURCE,
            sanitizedSnapshot: exported,
            renderedHtml: '<article>Updated</article>',
        });
        expectPlainBoundaryValue(record);
    });

    it('rejects live or host-owned values at the edge content boundary', () => {
        const unsafeValues: Record<string, unknown> = {
            function: () => 'nope',
            classInstance: new HostClassInstance('nope'),
            map: new Map([['x', 1]]),
            set: new Set(['x']),
            date: new Date('2026-06-17T00:00:00.000Z'),
            browserHandle: new BrowserHandleLike(),
        };
        const eventCtor = globalThis.Event;
        if (typeof eventCtor === 'function') {
            unsafeValues.event = new eventCtor('input');
        }

        for (const [name, value] of Object.entries(unsafeValues)) {
            expect(() => assertProcessingBoundaryValue({ value }, name)).toThrow(/non-|function/);
        }

        const store = new InMemoryEdgeRenderStateStore();
        const plan = projectTemplate(TEMPLATE_SOURCE, {
            snapshot: snapshotFixture(),
            values: { label: 'Projected' },
        });
        expect(() =>
            store.writeRenderState({
                renderPlan: plan,
                sanitizedSnapshot: { unsafe: new Date('2026-06-17T00:00:00.000Z') },
            })
        ).toThrow(/non-plain object Date/);
    });

    it('generates scope UIDs from explicit seeds, blank seeds, and dynamic fallback seeds', () => {
        expect(generateScopeUid({
            producedTag: 'story-card',
            uidSeed: 'demo/scoped-css/card',
            occurrencePath: '0.2',
        })).toBe('cem-scope-story-card-udemoz2fscoped-cssz2fcard-p0-2');

        expect(generateScopeUid({
            producedTag: 'story-card',
            uidSeed: '',
            occurrencePath: '0.2',
            runtimeSeed: 'ignored',
        })).toBe('cem-scope-story-card-p0-2');

        expect(generateScopeUid({
            producedTag: 'story-card',
            uidSeed: null,
            occurrencePath: '0.2',
            runtimeSeed: 'worker-0-counter-7',
        })).toBe('cem-scope-story-card-uworker-0-counter-7-p0-2');
    });

    it('covers the accepted UID matrix for stable persisted output', () => {
        const stableInput = {
            producedTag: 'story-card',
            uidSeed: 'docs/public seed: card',
            occurrencePath: '0.2.1',
        };
        const first = generateScopeUid(stableInput);
        const repeated = generateScopeUid(stableInput);
        expect(repeated).toBe(first);
        expect(first).toBe('cem-scope-story-card-udocsz2fpublicz20seedz3az20card-p0-2-1');
        expect(first).not.toMatch(/[ /:@]/);

        expect(generateScopeUid({
            ...stableInput,
            occurrencePath: '0.2.2',
        })).not.toBe(first);

        expect(generateScopeUid({
            ...stableInput,
            runtimeSeed: 'worker-17-counter-99',
        })).toBe(first);

        const scheduledOutOfOrder = [
            generateScopeUid({ ...stableInput, occurrencePath: '0.3' }),
            generateScopeUid({ ...stableInput, occurrencePath: '0.1' }),
            generateScopeUid({ ...stableInput, occurrencePath: '0.2' }),
        ].sort();
        const scheduledInOrder = ['0.1', '0.2', '0.3']
            .map((occurrencePath) => generateScopeUid({ ...stableInput, occurrencePath }))
            .sort();
        expect(scheduledOutOfOrder).toEqual(scheduledInOrder);
    });

    it('rewrites scoped CSS with nesting, host aliases, keyframes, and suppressed globals', () => {
        const result = scopeCssText(
            [
                '@import url("./theme.css");',
                '@font-face { font-family: Demo; src: url("./demo.woff2"); }',
                ':host { display: block; }',
                ':global(.legacy) button, :root { color: red; }',
                '@keyframes pulse { from { opacity: 0; } to { opacity: 1; } }',
                'button { animation: pulse 1s ease; animation-name: pulse; }',
            ].join('\n'),
            'cem-scope-story-card-useed-p0'
        );

        expect(result.css).toContain('[data-cem-scope="cem-scope-story-card-useed-p0"] {');
        expect(result.css).toContain('& { display: block; }');
        expect(result.css).toContain('&.legacy button, & { color: red; }');
        expect(result.css).toContain('@keyframes pulse-cem-scope-story-card-useed-p0');
        expect(result.css).toContain('animation: pulse-cem-scope-story-card-useed-p0 1s ease');
        expect(result.css).toContain('animation-name: pulse-cem-scope-story-card-useed-p0');
        expect(result.css).not.toContain('@import');
        expect(result.css).not.toContain('@font-face');
        expect(result.diagnostics.map((diagnostic) => diagnostic.code)).toEqual([
            'cem.scoped_css.import_unsupported',
            'cem.scoped_css.global_construct_unsupported',
            'cem.scoped_css.global_alias',
        ]);
    });

    it('stamps scoped render roots and rewrites style nodes in render plans', () => {
        const plan = projectTemplate(
            [{
                kind: 'element',
                namespace: null,
                tag: 'section',
                attributes: [],
                children: [
                    {
                        kind: 'element',
                        namespace: null,
                        tag: 'style',
                        attributes: [],
                        children: [{ kind: 'text', text: 'button { color: green; }' }],
                    },
                    {
                        kind: 'element',
                        namespace: null,
                        tag: 'button',
                        attributes: [],
                        children: [{ kind: 'text', text: 'Save' }],
                    },
                ],
            }],
            { snapshot: snapshotFixture(), values: {} }
        );
        const scoped = scopeRenderPlan(plan, 'cem-scope-story-card-useed-p0');
        const root = scoped.renderPlan.nodes[0];
        expect(root.kind).toBe('element');
        if (root.kind !== 'element') return;

        expect(root.attributes).toContainEqual({
            name: 'data-cem-scope',
            value: 'cem-scope-story-card-useed-p0',
        });
        const style = root.children[0];
        expect(style.kind).toBe('element');
        if (style.kind !== 'element') return;
        expect(style.children[0]).toEqual({
            kind: 'text',
            text: '[data-cem-scope="cem-scope-story-card-useed-p0"] {\n    button { color: green; }\n}',
            sourceMapRef: undefined,
        });
    });

    it('rewrites payload style nodes against an instance scope', () => {
        const plan: RenderPlan = {
            producedTag: 'story-card',
            instanceId: 'cem-instance-7',
            templateArtifactId: 'template-artifact-payload-style',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'boundary-scope',
            nodes: [{
                kind: 'element',
                namespace: null,
                tag: 'button',
                renderNodeId: 'story-card-1',
                attributes: [],
                children: [{
                    kind: 'element',
                    namespace: null,
                    tag: 'style',
                    renderNodeId: 'payload-0',
                    attributes: [],
                    children: [{ kind: 'text', text: 'button { border-color: red; }' }],
                }],
            }],
        };
        const scopeUid = 'cem-scope-story-card-useed-p0';
        const instanceScopeUid = renderInstanceScopeUid(scopeUid, plan.instanceId);
        const scoped = scopeRenderPlan(plan, scopeUid, { instanceScopeUid });
        const root = scoped.renderPlan.nodes[0];
        expect(root.kind).toBe('element');
        if (root.kind !== 'element') return;
        const style = root.children[0];
        expect(style.kind).toBe('element');
        if (style.kind !== 'element') return;
        expect(style.children[0]).toEqual({
            kind: 'text',
            text: `[data-cem-instance-scope="${instanceScopeUid}"] {\n    button { border-color: red; }\n}`,
            sourceMapRef: undefined,
        });
    });

    it('diagnoses duplicate generated render-plan and stylesheet IDs', () => {
        const plan: RenderPlan = {
            producedTag: 'story-card',
            instanceId: 'cem-instance-1',
            templateArtifactId: 'template-artifact-1',
            dataRevision: '1',
            outputTarget: 'light-dom',
            scopePolicyStamp: 'boundary-scope',
            nodes: [
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'style',
                    renderNodeId: 'story-card-style',
                    attributes: [],
                    children: [{ kind: 'text', text: 'button { color: red; }' }],
                },
                {
                    kind: 'element',
                    namespace: null,
                    tag: 'section',
                    renderNodeId: 'story-card-section',
                    attributes: [],
                    children: [{
                        kind: 'element',
                        namespace: null,
                        tag: 'style',
                        renderNodeId: 'story-card-style',
                        attributes: [],
                        children: [{ kind: 'text', text: 'button { color: blue; }' }],
                    }],
                },
            ],
        };

        expect(validateRenderPlanGeneratedIds(plan).map((diagnostic) => diagnostic.code)).toEqual([
            'cem.render_plan.generated_render_node_id_duplicate',
            'cem.render_plan.generated_stylesheet_id_duplicate',
        ]);
    });

    it('detects visible render-plan DOM changes while ignoring revision-only changes', () => {
        const first = projectTemplate(TEMPLATE_SOURCE, {
            snapshot: snapshotFixture(),
            values: { label: 'Projected' },
        });
        const revisionOnly: RenderPlan = {
            ...first,
            dataRevision: '2',
        };
        expect(renderPlansHaveDomChanges(first, revisionOnly)).toBe(false);

        const changedAttribute = projectTemplate(TEMPLATE_SOURCE, {
            snapshot: snapshotFixture(),
            values: { label: 'Updated' },
        });
        expect(renderPlansHaveDomChanges(first, changedAttribute)).toBe(true);

        const changedText: RenderPlan = {
            ...first,
            nodes: [{
                ...(first.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>),
                children: [{ kind: 'text', text: 'Updated' }],
            }],
        };
        expect(renderPlansHaveDomChanges(first, changedText)).toBe(true);
    });
});

function expectPlainBoundaryValue(value: unknown): void {
    assertProcessingBoundaryValue(value);
    const cloned = structuredClone(value);
    expect(cloned).toEqual(value);
}

function snapshotFixture(): DataIslandSnapshot {
    return {
        version: SNAPSHOT_SCHEMA_VERSION,
        instanceId: 'boundary-instance-1',
        producedTag: 'boundary-card',
        declarationTag: 'cem-element-boundary',
        templateArtifactId: 'boundary-template-1',
        dataRevision: '1',
        outputTarget: 'light-dom',
        sourceMapMode: 'dev',
        scopePolicyStamp: 'boundary-scope',
        privacyPolicyStamp: 'boundary-privacy',
        hostAttributes: { label: 'Projected' },
        dataset: { flavor: 'plain' },
        payload: {
            text: 'Detail',
            childCount: 1,
            nodes: [{
                kind: 'element',
                key: 'payload-0',
                tag: 'span',
                namespace: null,
                attributes: { slot: 'detail' },
                slot: 'detail',
                children: [{ kind: 'text', key: 'payload-0/0', text: 'Detail' }],
            }],
            slots: {
                detail: [{
                    kind: 'element',
                    key: 'payload-0',
                    tag: 'span',
                    namespace: null,
                    attributes: { slot: 'detail' },
                    slot: 'detail',
                    children: [{ kind: 'text', key: 'payload-0/0', text: 'Detail' }],
                }],
            },
            data: [],
            options: [],
            dataByValue: {},
            optionsByValue: {},
        },
        slices: {
            date: new Date('2026-06-17T00:00:00.000Z') as unknown,
            klass: new HostClassInstance('class-value') as unknown,
            primitive: 'ok',
        },
        formData: { signin: { username: 'ada' } },
        validationState: {},
        eventPayloads: {
            fn: (() => 'dropped') as unknown,
            detail: { ok: true },
        },
    };
}
