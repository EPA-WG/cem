import { describe, expect, it } from 'vitest';

import { exportDataIslandSnapshotForEdge } from './cem-elements.js';
import {
    InMemoryEdgeRenderStateStore,
    assertProcessingBoundaryValue,
    createEdgeRenderStateRecord,
    diffRenderPlansToPatchFrames,
    projectTemplate,
    readEdgeRenderStateContents,
    renderPlanIdentity,
    type RenderPlan,
    type TemplateProjectionInput,
} from './projection.js';
import {
    PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
    processingBoundarySnapshotFixture,
} from './processing-boundary.fixtures.js';

class HostClassInstance {
    constructor(readonly value: string) {}
}

class BrowserHandleLike {
    readonly elementId = 'handle-1';
}

describe('Edge/SSR processing boundary contracts', () => {
    it('keeps snapshots, render plans, patch frames, and edge records plain structured-clone data', () => {
        const snapshot = processingBoundarySnapshotFixture();
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
        expect(exported.renderAttempt).toBe(2);
        expect((exported.formData as Record<string, unknown>).signin).toEqual({ username: 'ada' });
        expect((exported.slices as Record<string, unknown>).date).toBe('2026-06-17T00:00:00.000Z');
        expect((exported.slices as Record<string, unknown>).klass).toEqual({ value: 'class-value' });
        expect((exported.eventPayloads as Record<string, unknown>).fn).toBeUndefined();

        const projection: TemplateProjectionInput = {
            snapshot,
            values: { label: 'Projected' },
        };
        const plan = projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, projection);
        expectPlainBoundaryValue(plan);
        expect(plan.nodes[0]?.kind).toBe('element');

        const nextPlan: RenderPlan = {
            ...plan,
            dataRevision: '2',
            nodes: [{
                ...plan.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>,
                attributes: [{ name: 'data-label', value: 'Updated' }],
                children: [{
                    kind: 'text',
                    text: 'Updated',
                    sourceMapRef: { fidelity: 'dom-canonical', frame: 'dom:0/0' },
                }],
            }],
        };
        const frames = diffRenderPlansToPatchFrames(plan, nextPlan, {
            transactionId: 'boundary-tx',
            batchSize: 1,
        });
        expectPlainBoundaryValue(frames);
        expect(frames.map((frame) => frame.type)).toContain('ops');

        const store = new InMemoryEdgeRenderStateStore();
        const write = store.writeRenderState({
            renderPlan: nextPlan,
            templateArtifact: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
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
            templateArtifact: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            sanitizedSnapshot: exported,
            renderedHtml: '<article>Updated</article>',
        });
        expectPlainBoundaryValue(record);
    });

    it('applies default-deny and redaction policy before exporting snapshots to edge hosts', () => {
        const snapshot = processingBoundarySnapshotFixture();
        snapshot.dataset = { analyticsId: 'visitor-42' };
        snapshot.slices = { draftInput: 'browser-local draft' };
        snapshot.formData = { signin: { username: 'ada', password: 'secret' } };
        snapshot.validationState = { valid: false, message: 'private validation detail' };
        snapshot.eventPayloads = {
            focus: { targetId: 'email' },
            input: { isComposing: true, selectionStart: 3, value: 'raw browser input' },
        };
        snapshot.payload = {
            ...snapshot.payload,
            text: 'Sensitive detail',
            data: [{
                kind: 'data',
                key: 'data-0',
                value: 'secret',
                label: 'Secret',
                text: 'Sensitive data',
                attributes: { value: 'secret' },
                group: null,
            }],
            dataByValue: {
                secret: {
                    kind: 'data',
                    key: 'data-0',
                    value: 'secret',
                    label: 'Secret',
                    text: 'Sensitive data',
                    attributes: { value: 'secret' },
                    group: null,
                },
            },
        };

        const defaultExport = exportDataIslandSnapshotForEdge(snapshot);
        expect(defaultExport.privacyPolicyStamp).toBe('boundary-privacy');
        expect(defaultExport).not.toHaveProperty('hostAttributes');
        expect(defaultExport).not.toHaveProperty('dataset');
        expect(defaultExport).not.toHaveProperty('payload');
        expect(defaultExport).not.toHaveProperty('slices');
        expect(defaultExport).not.toHaveProperty('formData');
        expect(defaultExport).not.toHaveProperty('validationState');
        expect(defaultExport).not.toHaveProperty('eventPayloads');

        const exported = exportDataIslandSnapshotForEdge(snapshot, {
            privacyPolicyStamp: 'edge-export-policy-v1',
            fields: {
                dataset: 'omit',
                eventPayloads: 'omit',
                formData: 'redact',
                hostAttributes: 'allow',
                payload: 'redact',
                slices: 'omit',
                validationState: 'redact',
            },
        });
        expect(exported.privacyPolicyStamp).toBe('edge-export-policy-v1');
        expect(exported.hostAttributes).toEqual({ label: 'Projected' });
        expect(exported).not.toHaveProperty('dataset');
        expect(exported).not.toHaveProperty('slices');
        expect(exported).not.toHaveProperty('eventPayloads');
        expect(exported.payload?.text).toBe('');
        expect(exported.payload?.childCount).toBe(0);
        expect(exported.payload?.data).toEqual([]);
        expect(exported.payload?.dataByValue).toEqual({});
        expect(exported.formData).toEqual({});
        expect(exported.validationState).toEqual({});

        snapshot.hostAttributes.label = 'Mutated After Export';
        expect(exported.hostAttributes?.label).toBe('Projected');
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
        const plan = projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, {
            snapshot: processingBoundarySnapshotFixture(),
            values: { label: 'Projected' },
        });
        expect(() =>
            store.writeRenderState({
                renderPlan: plan,
                sanitizedSnapshot: { unsafe: new Date('2026-06-17T00:00:00.000Z') },
            })
        ).toThrow(/non-plain object Date/);
    });
});

function expectPlainBoundaryValue(value: unknown): void {
    assertProcessingBoundaryValue(value);
    const cloned = structuredClone(value);
    expect(cloned).toEqual(value);
}
