import { describe, expect, it } from 'vitest';

import {
    exportDataIslandSnapshotForEdge,
    type ExportedDataIslandSnapshot,
} from './cem-elements.js';
import {
    CemEdgeSsrJobSequence,
    createCemEdgeSsrHostRequestEnvelope,
    type CemEdgeSsrInitialRenderInput,
    type CemEdgeSsrTemplateInput,
} from './edge-ssr-host.js';
import { executeNonBrowserSsrInitialRenderFixture } from './edge-ssr-host-fixture.js';
import {
    InMemoryEdgeRenderStateStore,
    edgeContentAddress,
    readEdgeRenderStateContents,
    renderPlanIdentity,
} from './projection.js';
import {
    PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
    processingBoundarySnapshotFixture,
} from './processing-boundary.fixtures.js';

describe('non-browser SSR initial-render host fixture', () => {
    it('emits escaped identity-bearing HTML, hydration data, and verified retained state without DOM globals', () => {
        const snapshot = processingBoundarySnapshotFixture();
        snapshot.hostAttributes = {
            ...snapshot.hostAttributes,
            label: 'Server <Card> & "safe"',
            'data-cem-render-scope': 'boundary-server-scope',
        };
        const exported = exportCompleteSnapshot(snapshot);
        const request = initialRequest(exported);
        const store = new InMemoryEdgeRenderStateStore();

        expect(typeof (globalThis as { document?: unknown }).document).toBe('undefined');
        const response = executeNonBrowserSsrInitialRenderFixture(request, store);

        expect(response.outcome).toBe('success');
        if (response.outcome !== 'success') return;
        expect(response.jobId).toBe(request.jobId);
        expect(response.result.renderedHtml).toContain(
            'data-label="Server &lt;Card&gt; &amp; &quot;safe&quot;"'
        );
        expect(response.result.renderedHtml).toContain('data-cem-render-scope="boundary-server-scope"');
        expect(response.result.renderedHtml).toContain('data-cem-render-node-id="boundary-card-1"');
        expect(response.result.renderedHtml).toContain('data-cem-template-artifact-id="boundary-template-1"');
        expect(response.result.renderedHtml).toContain('data-cem-data-revision="1"');
        expect(response.result.renderedHtml).toContain('data-cem-source-fidelity="dom-canonical"');
        expect(response.result.hydrationData).toEqual({
            kind: 'cem-ssr-hydration-v1',
            snapshot: exported,
            revision: request.payload.revision,
            renderPlanIdentity: expect.objectContaining(request.payload.revision),
            sourceMapMode: 'dev',
        });

        const retained = readEdgeRenderStateContents(store, response.result.renderState);
        expect(retained.ok).toBe(true);
        if (!retained.ok) return;
        expect(retained.contents.renderedHtml).toBe(response.result.renderedHtml);
        expect(retained.contents.sanitizedSnapshot).toEqual(exported);
        expect(renderPlanIdentity(retained.contents.renderPlan)).toEqual(
            response.result.hydrationData.renderPlanIdentity
        );
        expect(structuredClone(response)).toEqual(response);

        const duplicate = executeNonBrowserSsrInitialRenderFixture(initialRequest(exported), store);
        expect(duplicate.outcome).toBe('failure');
        if (duplicate.outcome === 'failure') {
            expect(duplicate.reason).toBe('render-state-conflict');
            expect(duplicate.currentRenderState?.etag).toBe(response.result.renderState.etag);
        }
    });

    it('rejects template, revision, source-map, and scope identity mismatches before storing state', () => {
        const cases: Array<[string, (input: CemEdgeSsrInitialRenderInput) => CemEdgeSsrInitialRenderInput]> = [
            [
                'template artifact',
                (input) => ({
                    ...input,
                    template: { ...input.template, templateArtifactId: 'mismatched-template' },
                }),
            ],
            [
                'render revision',
                (input) => ({
                    ...input,
                    revision: { ...input.revision, dataRevision: 'mismatched-revision' },
                }),
            ],
            [
                'source-map mode',
                (input) => ({
                    ...input,
                    snapshot: { ...input.snapshot, sourceMapMode: 'prod' },
                }),
            ],
            [
                'scope identity',
                (input) => ({
                    ...input,
                    snapshot: {
                        ...input.snapshot,
                        hostAttributes: {
                            ...input.snapshot.hostAttributes,
                            'data-cem-render-scope': 'mismatched-scope',
                        },
                    },
                }),
            ],
        ];

        for (const [label, transform] of cases) {
            const snapshot = processingBoundarySnapshotFixture();
            snapshot.hostAttributes['data-cem-render-scope'] = 'boundary-server-scope';
            const exported = exportCompleteSnapshot(snapshot);
            const request = initialRequest(exported, undefined, transform);
            const store = new InMemoryEdgeRenderStateStore();
            const response = executeNonBrowserSsrInitialRenderFixture(request, store);

            expect(response.outcome, label).toBe('failure');
            if (response.outcome !== 'failure') continue;
            expect(response.reason, label).toBe('invalid-request');
            expect(response.diagnostics[0]?.code, label).toBe('cem.edge_ssr.initial_identity_invalid');
            expect(store.readRecord(expectedStateKey(exported)), label).toBeUndefined();
        }
    });

    it('fails closed for policy-omitted fields, unresolved artifacts, and unsafe raw HTML', () => {
        const snapshot = processingBoundarySnapshotFixture();
        const defaultDenied = exportDataIslandSnapshotForEdge(snapshot);
        const missingFieldsStore = new InMemoryEdgeRenderStateStore();
        const missingFields = executeNonBrowserSsrInitialRenderFixture(
            initialRequest(defaultDenied),
            missingFieldsStore
        );
        expect(missingFields.outcome).toBe('failure');
        if (missingFields.outcome === 'failure') {
            expect(missingFields.reason).toBe('privacy-policy-rejected');
        }
        expect(missingFieldsStore.readRecord(expectedStateKey(defaultDenied))).toBeUndefined();

        const exported = exportCompleteSnapshot(snapshot);
        const addressedTemplate: CemEdgeSsrTemplateInput = {
            kind: 'content-addressed-template-artifact-v1',
            templateArtifactId: exported.templateArtifactId,
            address: {
                ...edgeContentAddress('template-artifact', PROCESSING_BOUNDARY_TEMPLATE_SOURCE),
                kind: 'template-artifact',
            },
        };
        const unresolvedStore = new InMemoryEdgeRenderStateStore();
        const unresolved = executeNonBrowserSsrInitialRenderFixture(
            initialRequest(exported, addressedTemplate),
            unresolvedStore
        );
        expect(unresolved.outcome).toBe('failure');
        if (unresolved.outcome === 'failure') {
            expect(unresolved.reason).toBe('content-unavailable');
        }
        expect(unresolvedStore.readRecord(expectedStateKey(exported))).toBeUndefined();

        const unsafeStore = new InMemoryEdgeRenderStateStore();
        const unsafe = executeNonBrowserSsrInitialRenderFixture(
            initialRequest(exported, {
                kind: 'serialized-template-source-v1',
                templateArtifactId: exported.templateArtifactId,
                source: [{
                    kind: 'element',
                    namespace: null,
                    tag: 'script',
                    attributes: [],
                    children: [{ kind: 'text', text: 'alert(1)' }],
                }],
            }),
            unsafeStore
        );
        expect(unsafe.outcome).toBe('failure');
        if (unsafe.outcome === 'failure') {
            expect(unsafe.reason).toBe('render-failed');
            expect(unsafe.diagnostics[0]?.message).toMatch(/script elements are not supported/);
        }
        expect(unsafeStore.readRecord(expectedStateKey(exported))).toBeUndefined();
    });
});

function exportCompleteSnapshot(
    snapshot = processingBoundarySnapshotFixture()
): ExportedDataIslandSnapshot {
    return exportDataIslandSnapshotForEdge(snapshot, {
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
}

function initialRequest(
    snapshot: ExportedDataIslandSnapshot,
    template: CemEdgeSsrTemplateInput = {
        kind: 'serialized-template-source-v1',
        templateArtifactId: snapshot.templateArtifactId,
        source: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
    },
    transform: (input: CemEdgeSsrInitialRenderInput) => CemEdgeSsrInitialRenderInput = (input) => input
) {
    const input = transform({
        template,
        snapshot,
        revision: {
            instanceId: snapshot.instanceId,
            dataRevision: snapshot.dataRevision,
            templateArtifactId: snapshot.templateArtifactId,
            scopePolicyStamp: snapshot.scopePolicyStamp,
            outputTarget: snapshot.outputTarget,
            ...(snapshot.renderAttempt === undefined ? {} : { renderAttempt: snapshot.renderAttempt }),
        },
        sourceMapMode: snapshot.sourceMapMode ?? 'dev',
        scopeUid: 'boundary-server-scope',
    });
    return createCemEdgeSsrHostRequestEnvelope(
        new CemEdgeSsrJobSequence(),
        'render-initial',
        input
    );
}

function expectedStateKey(snapshot: ExportedDataIslandSnapshot): string {
    return `edge-state:${snapshot.scopePolicyStamp}:${snapshot.instanceId}`;
}
