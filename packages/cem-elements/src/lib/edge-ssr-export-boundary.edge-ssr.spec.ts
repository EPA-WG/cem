import { describe, expect, it } from 'vitest';

import {
    createCemEdgeSsrBrowserRequestEnvelope,
    type DataIslandSnapshot,
    type DataIslandSnapshotExportPolicy,
} from './cem-elements.js';
import {
    CemEdgeSsrJobSequence,
    type CemEdgeSsrHostResponseEnvelope,
} from './edge-ssr-host.js';
import {
    executeNonBrowserEdgeRenderUpdateFixture,
    executeNonBrowserSsrInitialRenderFixture,
    type NonBrowserEdgeRenderUpdateFixtureResponse,
} from './edge-ssr-host-fixture.js';
import {
    InMemoryEdgeRenderStateStore,
    readEdgeRenderStateContents,
} from './projection.js';
import {
    PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
    processingBoundarySnapshotFixture,
} from './processing-boundary.fixtures.js';

const COMPLETE_EXPORT_POLICY: DataIslandSnapshotExportPolicy = {
    privacyPolicyStamp: 'edge-complete-export-v1',
    fields: {
        hostAttributes: 'allow',
        dataset: 'allow',
        payload: 'allow',
        slices: 'allow',
        formData: 'allow',
        validationState: 'allow',
        eventPayloads: 'allow',
    },
};

const REDACTED_EXPORT_POLICY: DataIslandSnapshotExportPolicy = {
    privacyPolicyStamp: 'edge-redacted-export-v1',
    fields: {
        hostAttributes: 'allow',
        dataset: 'redact',
        payload: 'redact',
        slices: 'redact',
        formData: 'redact',
        validationState: 'redact',
        eventPayloads: 'redact',
    },
};

describe('browser-to-edge snapshot export boundary', () => {
    it('omits default-denied fields before creating an initial-render host request', () => {
        const secret = 'initial-default-deny-secret';
        const snapshot = privateSnapshot('Before', '1', secret);
        const request = initialBrowserRequest(snapshot);
        const store = new InMemoryEdgeRenderStateStore();

        expect(request.payload).not.toHaveProperty('exportPolicy');
        expect(request.payload.snapshot).not.toHaveProperty('hostAttributes');
        expect(request.payload.snapshot).not.toHaveProperty('dataset');
        expect(request.payload.snapshot).not.toHaveProperty('payload');
        expect(request.payload.snapshot).not.toHaveProperty('slices');
        expect(request.payload.snapshot).not.toHaveProperty('formData');
        expect(request.payload.snapshot).not.toHaveProperty('validationState');
        expect(request.payload.snapshot).not.toHaveProperty('eventPayloads');
        expect(JSON.stringify(request)).not.toContain(secret);

        const response = executeNonBrowserSsrInitialRenderFixture(request, store);
        expect(response.outcome).toBe('failure');
        if (response.outcome === 'failure') {
            expect(response.reason).toBe('privacy-policy-rejected');
        }
        expect(store.readRecord(stateKey(snapshot))).toBeUndefined();
    });

    it('redacts initial-render fields before transport and retains exactly the owned export', () => {
        const secret = 'initial-redaction-secret';
        const snapshot = privateSnapshot('Before', '1', secret);
        const request = initialBrowserRequest(snapshot, REDACTED_EXPORT_POLICY);
        const exported = request.payload.snapshot;
        const store = new InMemoryEdgeRenderStateStore();

        expect(exported.privacyPolicyStamp).toBe('edge-redacted-export-v1');
        expect(exported.hostAttributes?.label).toBe('Before');
        expect(exported.dataset).toEqual({});
        expect(exported.payload?.text).toBe('');
        expect(exported.payload?.nodes).toEqual([]);
        expect(exported.slices).toEqual({});
        expect(exported.formData).toEqual({});
        expect(exported.validationState).toEqual({});
        expect(exported.eventPayloads).toEqual({});
        expect(JSON.stringify(request)).not.toContain(secret);

        snapshot.hostAttributes.label = 'Mutated After Export';
        snapshot.payload.text = 'Mutated After Export';
        snapshot.formData = { password: 'Mutated After Export' };
        expect(exported.hostAttributes?.label).toBe('Before');
        expect(exported.payload?.text).toBe('');
        expect(exported.formData).toEqual({});

        const response = executeNonBrowserSsrInitialRenderFixture(request, store);
        expect(response.outcome).toBe('success');
        if (response.outcome !== 'success') return;
        expect(response.result.hydrationMetadata.snapshot).toEqual(exported);
        const retained = readEdgeRenderStateContents(store, response.result.renderState);
        expect(retained.ok).toBe(true);
        if (!retained.ok) return;
        expect(retained.contents.sanitizedSnapshot).toEqual(exported);
        expect(JSON.stringify(retained.contents)).not.toContain(secret);
    });

    it('omits default-denied fields before an update request and leaves the pointer unchanged', async () => {
        const store = new InMemoryEdgeRenderStateStore();
        const seeded = seedInitialRender(store);
        const secret = 'update-default-deny-secret';
        const snapshot = privateSnapshot('After', '2', secret);
        const request = updateBrowserRequest(seeded, snapshot);

        expect(request.payload).not.toHaveProperty('exportPolicy');
        expect(request.payload.snapshot).not.toHaveProperty('hostAttributes');
        expect(request.payload.snapshot).not.toHaveProperty('payload');
        expect(JSON.stringify(request)).not.toContain(secret);

        const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));
        expect(responses).toHaveLength(1);
        expect(responses[0]?.outcome).toBe('failure');
        if (responses[0]?.outcome === 'failure') {
            expect(responses[0].reason).toBe('privacy-policy-rejected');
        }
        expect(responses.some((response) => response.outcome === 'progress')).toBe(false);
        expect(store.readRecord(seeded.result.renderState.stateKey)?.etag).toBe(
            seeded.result.renderState.etag
        );
    });

    it('redacts update fields before transport and commits only the owned export', async () => {
        const store = new InMemoryEdgeRenderStateStore();
        const seeded = seedInitialRender(store);
        const secret = 'update-redaction-secret';
        const snapshot = privateSnapshot('After', '2', secret);
        const request = updateBrowserRequest(seeded, snapshot, REDACTED_EXPORT_POLICY);
        const exported = request.payload.snapshot;

        expect(exported.hostAttributes?.label).toBe('After');
        expect(exported.dataset).toEqual({});
        expect(exported.payload?.text).toBe('');
        expect(exported.slices).toEqual({});
        expect(exported.formData).toEqual({});
        expect(exported.validationState).toEqual({});
        expect(exported.eventPayloads).toEqual({});
        expect(JSON.stringify(request)).not.toContain(secret);

        snapshot.hostAttributes.label = 'Mutated After Export';
        snapshot.eventPayloads = { input: { value: 'Mutated After Export' } };
        expect(exported.hostAttributes?.label).toBe('After');
        expect(exported.eventPayloads).toEqual({});

        const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));
        expect(responses.filter((response) => response.outcome === 'progress')
            .map((response) => response.result.frame.type)).toEqual(['begin', 'ops', 'commit']);
        const terminal = responses.at(-1);
        expect(terminal?.outcome).toBe('success');
        if (!terminal || terminal.outcome !== 'success') return;
        const retained = readEdgeRenderStateContents(store, terminal.result.renderState);
        expect(retained.ok).toBe(true);
        if (!retained.ok) return;
        expect(retained.contents.sanitizedSnapshot).toEqual(exported);
        expect(JSON.stringify(retained.contents)).not.toContain(secret);
    });
});

type SeededInitialRender = Extract<CemEdgeSsrHostResponseEnvelope, {
    operation: 'render-initial';
    outcome: 'success';
}>;

function seedInitialRender(store: InMemoryEdgeRenderStateStore): SeededInitialRender {
    const request = initialBrowserRequest(
        renderSnapshot('Before', '1'),
        COMPLETE_EXPORT_POLICY
    );
    const response = executeNonBrowserSsrInitialRenderFixture(request, store);
    if (response.outcome !== 'success') {
        throw new Error(`failed to seed privacy fixture: ${response.diagnostics[0]?.message ?? response.reason}`);
    }
    return response;
}

function initialBrowserRequest(
    snapshot: DataIslandSnapshot,
    exportPolicy?: DataIslandSnapshotExportPolicy
) {
    return createCemEdgeSsrBrowserRequestEnvelope(
        new CemEdgeSsrJobSequence(),
        'render-initial',
        {
            template: {
                kind: 'serialized-template-source-v1',
                templateArtifactId: snapshot.templateArtifactId,
                source: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            },
            snapshot,
            ...(exportPolicy ? { exportPolicy } : {}),
            revision: revisionFromSnapshot(snapshot),
            sourceMapMode: snapshot.sourceMapMode ?? 'dev',
            scopeUid: 'boundary-server-scope',
        }
    );
}

function updateBrowserRequest(
    seeded: SeededInitialRender,
    snapshot: DataIslandSnapshot,
    exportPolicy?: DataIslandSnapshotExportPolicy
) {
    const templateAddress = seeded.result.renderState.currentTemplateArtifact;
    if (!templateAddress || templateAddress.kind !== 'template-artifact') {
        throw new Error('seeded privacy fixture did not retain a template artifact');
    }
    const renderPlanAddress = seeded.result.renderState.currentRenderPlan;
    if (renderPlanAddress.kind !== 'render-plan') {
        throw new Error('seeded privacy fixture did not retain a render plan');
    }
    return createCemEdgeSsrBrowserRequestEnvelope(
        new CemEdgeSsrJobSequence(),
        'render-update',
        {
            template: {
                kind: 'content-addressed-template-artifact-v1',
                templateArtifactId: snapshot.templateArtifactId,
                address: { ...templateAddress, kind: 'template-artifact' },
            },
            snapshot,
            ...(exportPolicy ? { exportPolicy } : {}),
            revision: revisionFromSnapshot(snapshot),
            sourceMapMode: snapshot.sourceMapMode ?? 'dev',
            scopeUid: 'boundary-server-scope',
            previousRenderPlan: {
                stateKey: seeded.result.renderState.stateKey,
                expectedEtag: seeded.result.renderState.etag,
                identity: seeded.result.hydrationMetadata.renderPlanIdentity,
                address: { ...renderPlanAddress, kind: 'render-plan' },
            },
        }
    );
}

function renderSnapshot(label: string, dataRevision: string): DataIslandSnapshot {
    const snapshot = processingBoundarySnapshotFixture();
    snapshot.dataRevision = dataRevision;
    snapshot.hostAttributes = {
        ...snapshot.hostAttributes,
        label,
        'data-cem-render-scope': 'boundary-server-scope',
    };
    return snapshot;
}

function privateSnapshot(label: string, dataRevision: string, secret: string): DataIslandSnapshot {
    const snapshot = renderSnapshot(label, dataRevision);
    snapshot.dataset = { analyticsId: secret };
    snapshot.payload = { ...snapshot.payload, text: secret };
    snapshot.slices = { draft: secret };
    snapshot.formData = { signin: { password: secret } };
    snapshot.validationState = { message: secret };
    snapshot.eventPayloads = { input: { value: secret, isComposing: true } };
    return snapshot;
}

function revisionFromSnapshot(snapshot: DataIslandSnapshot) {
    return {
        instanceId: snapshot.instanceId,
        dataRevision: snapshot.dataRevision,
        templateArtifactId: snapshot.templateArtifactId,
        scopePolicyStamp: snapshot.scopePolicyStamp,
        outputTarget: snapshot.outputTarget,
        ...(snapshot.renderAttempt === undefined ? {} : { renderAttempt: snapshot.renderAttempt }),
    };
}

function stateKey(snapshot: DataIslandSnapshot): string {
    return `edge-state:${snapshot.scopePolicyStamp}:${snapshot.instanceId}`;
}

async function collectResponses(
    responses: AsyncIterable<NonBrowserEdgeRenderUpdateFixtureResponse>
): Promise<NonBrowserEdgeRenderUpdateFixtureResponse[]> {
    const collected: NonBrowserEdgeRenderUpdateFixtureResponse[] = [];
    for await (const response of responses) {
        collected.push(response);
    }
    return collected;
}
