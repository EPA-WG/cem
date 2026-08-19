import { describe, expect, it } from 'vitest';

import { exportDataIslandSnapshotForEdge } from './cem-elements.js';
import {
    CemEdgeSsrJobSequence,
    assertCemEdgeSsrHostEnvelope,
    createCemEdgeSsrHostFailureEnvelope,
    createCemEdgeSsrHostProgressEnvelope,
    createCemEdgeSsrHostRequestEnvelope,
    createCemEdgeSsrHostSuccessEnvelope,
} from './edge-ssr-host.js';
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
    it('locks correlated initial-render and streamed edge-update host envelopes', () => {
        const snapshot = processingBoundarySnapshotFixture();
        const exported = exportDataIslandSnapshotForEdge(snapshot, {
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
        const revision = {
            instanceId: snapshot.instanceId,
            dataRevision: snapshot.dataRevision,
            templateArtifactId: snapshot.templateArtifactId,
            scopePolicyStamp: snapshot.scopePolicyStamp,
            outputTarget: snapshot.outputTarget,
            renderAttempt: snapshot.renderAttempt,
        };
        const previousPlan = projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, {
            snapshot,
            values: { label: 'Before' },
        });
        const store = new InMemoryEdgeRenderStateStore();
        const seeded = store.writeRenderState({
            renderPlan: previousPlan,
            templateArtifact: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            sanitizedSnapshot: exported,
            renderedHtml: '<article data-label="Before"></article>',
            privacyPolicyStamp: exported.privacyPolicyStamp,
        });
        expect(seeded.ok).toBe(true);
        if (!seeded.ok || !seeded.record.currentTemplateArtifact) return;

        const sequence = new CemEdgeSsrJobSequence();
        const initialRequest = createCemEdgeSsrHostRequestEnvelope(sequence, 'render-initial', {
            template: {
                kind: 'serialized-template-source-v1',
                templateArtifactId: snapshot.templateArtifactId,
                source: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            },
            snapshot: exported,
            revision,
            sourceMapMode: 'dev',
            scopeUid: 'boundary-scope-uid',
            instanceScopeUid: 'boundary-instance-scope-uid',
        });
        const initialResponse = createCemEdgeSsrHostSuccessEnvelope(initialRequest, {
            kind: 'initial-render',
            renderedHtml: '<article data-label="Before"></article>',
            hydrationMetadata: {
                kind: 'cem-ssr-hydration-v1',
                snapshot: exported,
                revision,
                renderPlanIdentity: renderPlanIdentity(previousPlan),
                sourceMapMode: 'dev',
            },
            renderState: seeded.record,
            diagnostics: [],
        });

        const nextSnapshot = { ...snapshot, dataRevision: '2' };
        const nextExported = { ...exported, dataRevision: '2' };
        const nextPlan = projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, {
            snapshot: nextSnapshot,
            values: { label: 'After' },
        });
        const updateRequest = createCemEdgeSsrHostRequestEnvelope(sequence, 'render-update', {
            template: {
                kind: 'content-addressed-template-artifact-v1',
                templateArtifactId: snapshot.templateArtifactId,
                address: seeded.record.currentTemplateArtifact,
            },
            snapshot: nextExported,
            revision: { ...revision, dataRevision: '2' },
            sourceMapMode: 'dev',
            scopeUid: 'boundary-scope-uid',
            instanceScopeUid: 'boundary-instance-scope-uid',
            previousRenderPlan: {
                stateKey: seeded.record.stateKey,
                expectedEtag: seeded.record.etag,
                identity: renderPlanIdentity(previousPlan),
                address: seeded.record.currentRenderPlan,
            },
        });
        const frames = diffRenderPlansToPatchFrames(previousPlan, nextPlan, {
            transactionId: 'edge-update-2',
            batchSize: 1,
        });
        const advanced = store.writeRenderState(
            {
                renderPlan: nextPlan,
                templateArtifact: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
                sanitizedSnapshot: nextExported,
                renderedHtml: '<article data-label="After"></article>',
                privacyPolicyStamp: nextExported.privacyPolicyStamp,
                stateKey: seeded.record.stateKey,
            },
            { expectedEtag: seeded.record.etag }
        );
        expect(advanced.ok).toBe(true);
        if (!advanced.ok) return;
        const progress = frames.map((frame) => createCemEdgeSsrHostProgressEnvelope(updateRequest, frame));
        const complete = createCemEdgeSsrHostSuccessEnvelope(updateRequest, {
            kind: 'render-update-complete',
            renderPlanIdentity: renderPlanIdentity(nextPlan),
            renderState: advanced.record,
            diagnostics: [],
        });
        const conflictRequest = createCemEdgeSsrHostRequestEnvelope(sequence, 'render-update', {
            ...updateRequest.payload,
            previousRenderPlan: {
                ...updateRequest.payload.previousRenderPlan,
                expectedEtag: seeded.record.etag,
            },
        });
        const conflict = createCemEdgeSsrHostFailureEnvelope(
            conflictRequest,
            'failure',
            'render-state-conflict',
            [{
                code: 'cem.edge_ssr.render_state_conflict',
                severity: 'error',
                message: 'the expected render-state ETag is stale',
            }],
            advanced.record
        );

        expect(initialRequest.jobId).toBe(1);
        expect(updateRequest.jobId).toBe(2);
        expect(conflictRequest.jobId).toBe(3);
        expect(initialResponse.jobId).toBe(initialRequest.jobId);
        expect(progress.every((envelope) => envelope.jobId === updateRequest.jobId)).toBe(true);
        expect(progress.map((envelope) => envelope.result.frame.type)).toEqual(frames.map((frame) => frame.type));
        expect(progress.at(-1)?.result.frame.type).toBe('commit');
        expect(complete.result.renderState.etag).toBe(advanced.record.etag);
        expect(conflict.currentRenderState?.etag).toBe(advanced.record.etag);
        for (const envelope of [
            initialRequest,
            initialResponse,
            updateRequest,
            ...progress,
            complete,
            conflictRequest,
            conflict,
        ]) {
            assertCemEdgeSsrHostEnvelope(envelope);
            expectPlainBoundaryValue(envelope);
        }
        expect(() => assertCemEdgeSsrHostEnvelope({ ...initialRequest, jobId: 0 })).toThrow(
            /positive safe-integer job ID/
        );
        expect(() => assertCemEdgeSsrHostEnvelope({
            ...initialResponse,
            outcome: 'progress',
        })).toThrow(/only render-update patch-frame results/);
    });

    it('locks hybrid content-addressed blobs behind an optimistic revision pointer', () => {
        const previousPlan = projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, {
            snapshot: processingBoundarySnapshotFixture(),
            values: { label: 'Before' },
        });
        const nextPlan: RenderPlan = {
            ...previousPlan,
            dataRevision: '2',
            nodes: [{
                ...previousPlan.nodes[0] as Extract<RenderPlan['nodes'][number], { kind: 'element' }>,
                attributes: [{ name: 'data-label', value: 'After' }],
            }],
        };
        const store = new InMemoryEdgeRenderStateStore();
        const initial = store.writeRenderState({
            renderPlan: previousPlan,
            templateArtifact: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            sanitizedSnapshot: { label: 'Before' },
            renderedHtml: '<article data-label="Before"></article>',
            privacyPolicyStamp: 'edge-policy-v1',
        });
        expect(initial.ok).toBe(true);
        if (!initial.ok) return;

        expect(initial.record.storageModel).toBe('content-addressed-cache-with-revision-pointer-v1');
        expect(initial.record.currentTemplateArtifact?.kind).toBe('template-artifact');
        expect(initial.record.currentRenderPlan.kind).toBe('render-plan');
        expect(initial.record.currentSnapshot?.kind).toBe('sanitized-snapshot');
        expect(initial.record.currentHtml?.kind).toBe('rendered-html');

        const stale = store.writeRenderState(
            {
                renderPlan: nextPlan,
                stateKey: initial.record.stateKey,
                privacyPolicyStamp: 'edge-policy-v1',
            },
            { expectedEtag: 'stale-etag' }
        );
        expect(stale.ok).toBe(false);
        if (stale.ok) return;
        expect(stale.reason).toBe('etag-mismatch');
        expect(stale.current?.etag).toBe(initial.record.etag);
        expect(store.readRecord(initial.record.stateKey)?.etag).toBe(initial.record.etag);

        const advanced = store.writeRenderState(
            {
                renderPlan: nextPlan,
                stateKey: initial.record.stateKey,
                privacyPolicyStamp: 'edge-policy-v1',
            },
            { expectedEtag: initial.record.etag }
        );
        expect(advanced.ok).toBe(true);
        if (!advanced.ok) return;
        expect(advanced.record.stateKey).toBe(initial.record.stateKey);
        expect(advanced.record.etag).not.toBe(initial.record.etag);
        expect(store.readRecord(initial.record.stateKey)?.etag).toBe(advanced.record.etag);
        expect(store.getContent<RenderPlan>(initial.record.currentRenderPlan)).toEqual(previousPlan);
        expect(store.getContent<RenderPlan>(advanced.record.currentRenderPlan)).toEqual(nextPlan);
    });

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
