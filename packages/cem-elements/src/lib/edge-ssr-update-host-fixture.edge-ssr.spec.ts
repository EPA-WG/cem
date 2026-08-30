import { describe, expect, it } from 'vitest';

import {
    exportDataIslandSnapshotForEdge,
    type DataIslandSnapshot,
    type ExportedDataIslandSnapshot,
} from './cem-elements.js';
import {
    CemEdgeSsrJobSequence,
    createCemEdgeSsrHostRequestEnvelope,
    type CemEdgeSsrHostResponseEnvelope,
    type CemEdgeSsrRenderUpdateInput,
} from './edge-ssr-host.js';
import {
    executeNonBrowserEdgeRenderUpdateFixture,
    executeNonBrowserSsrInitialRenderFixture,
    type NonBrowserEdgeRenderUpdateFixtureResponse,
} from './edge-ssr-host-fixture.js';
import {
    InMemoryEdgeRenderStateStore,
    diffRenderPlansToPatchFrames,
    projectTemplate,
    scopeRenderPlan,
    type EdgeContentAddress,
    type EdgeContentKind,
    type EdgeRenderStateInput,
    type EdgeRenderStateRecord,
    type EdgeRenderStateStore,
    type EdgeRenderStateWriteOptions,
    type EdgeRenderStateWriteResult,
    type RenderPlan,
} from './projection.js';
import {
    PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
    processingBoundarySnapshotFixture,
} from './processing-boundary.fixtures.js';

describe('non-browser edge render-update host fixture', () => {
    it('streams the browser-reference patch frames before a terminal committed state without DOM globals', async () => {
        const store = new InMemoryEdgeRenderStateStore();
        const seeded = seedInitialRender(store);
        const nextSnapshot = renderSnapshot('After', '2');
        const nextExported = exportCompleteSnapshot(nextSnapshot);
        const request = updateRequest(seeded, nextExported);

        expect(typeof (globalThis as { document?: unknown }).document).toBe('undefined');
        const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));
        const progress = responses.filter((response) => response.outcome === 'progress');
        const terminal = responses.at(-1);

        expect(progress.map((response) => response.result.frame.type)).toEqual(['begin', 'ops', 'commit']);
        expect(terminal?.outcome).toBe('success');
        if (!terminal || terminal.outcome !== 'success') return;
        expect(responses.every((response) => response.jobId === request.jobId)).toBe(true);
        expect(terminal.result.renderState.etag).not.toBe(seeded.result.renderState.etag);
        expect(terminal.result.renderState.renderRevision.dataRevision).toBe('2');

        const previousPlan = store.getContent<RenderPlan>(seeded.result.renderState.currentRenderPlan);
        expect(previousPlan).toBeDefined();
        if (!previousPlan) return;
        const referenceNext = scopeRenderPlan(
            projectTemplate(PROCESSING_BOUNDARY_TEMPLATE_SOURCE, {
                snapshot: nextSnapshot,
                values: { label: 'After' },
            }),
            'boundary-server-scope',
            { payload: nextSnapshot.payload }
        ).renderPlan;
        const referenceFrames = diffRenderPlansToPatchFrames(previousPlan, referenceNext);
        const referenceCommit = referenceFrames.at(-1);
        expect(progress.map((response) => response.result.frame)).toEqual(referenceFrames);
        expect(referenceCommit?.type).toBe('commit');
        if (referenceCommit?.type !== 'commit') return;
        expect(terminal.result.renderPlanIdentity).toEqual(referenceCommit.nextRenderPlan);
        expect(store.readRecord(seeded.result.renderState.stateKey)?.etag).toBe(
            terminal.result.renderState.etag
        );
        for (const response of responses) {
            expect(structuredClone(response)).toEqual(response);
        }
    });

    it('returns one conflict and no patch progress for a stale expected ETag', async () => {
        const store = new InMemoryEdgeRenderStateStore();
        const seeded = seedInitialRender(store);
        const nextExported = exportCompleteSnapshot(renderSnapshot('After', '2'));
        const request = updateRequest(seeded, nextExported, (input) => ({
            ...input,
            previousRenderPlan: {
                ...input.previousRenderPlan,
                expectedEtag: 'stale-etag',
            },
        }));

        const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));

        expect(responses).toHaveLength(1);
        expect(responses[0]?.outcome).toBe('failure');
        if (responses[0]?.outcome === 'failure') {
            expect(responses[0].reason).toBe('render-state-conflict');
            expect(responses[0].currentRenderState?.etag).toBe(seeded.result.renderState.etag);
        }
        expect(responses.some((response) => response.outcome === 'progress')).toBe(false);
        expect(store.readRecord(seeded.result.renderState.stateKey)?.etag).toBe(
            seeded.result.renderState.etag
        );
    });

    it('rejects missing state, mismatched addresses, and mismatched identities without a commit', async () => {
        const cases: Array<[
            string,
            'render-state-not-found' | 'invalid-request',
            (input: CemEdgeSsrRenderUpdateInput) => CemEdgeSsrRenderUpdateInput,
        ]> = [
            [
                'missing state',
                'render-state-not-found',
                (input) => ({
                    ...input,
                    previousRenderPlan: { ...input.previousRenderPlan, stateKey: 'missing-state' },
                }),
            ],
            [
                'mismatched address',
                'invalid-request',
                (input) => ({
                    ...input,
                    previousRenderPlan: {
                        ...input.previousRenderPlan,
                        address: {
                            ...input.previousRenderPlan.address,
                            digest: 'mismatched-digest',
                        },
                    },
                }),
            ],
            [
                'mismatched identity',
                'invalid-request',
                (input) => ({
                    ...input,
                    previousRenderPlan: {
                        ...input.previousRenderPlan,
                        identity: {
                            ...input.previousRenderPlan.identity,
                            dataRevision: 'mismatched-revision',
                        },
                    },
                }),
            ],
        ];

        for (const [label, reason, transform] of cases) {
            const store = new InMemoryEdgeRenderStateStore();
            const seeded = seedInitialRender(store);
            const request = updateRequest(
                seeded,
                exportCompleteSnapshot(renderSnapshot('After', '2')),
                transform
            );
            const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));

            expect(responses, label).toHaveLength(1);
            expect(responses[0]?.outcome, label).toBe('failure');
            if (responses[0]?.outcome === 'failure') {
                expect(responses[0].reason, label).toBe(reason);
            }
            expect(
                responses.some(
                    (response) => response.outcome === 'progress' && response.result.frame.type === 'commit'
                ),
                label
            ).toBe(false);
            expect(store.readRecord(seeded.result.renderState.stateKey)?.etag, label).toBe(
                seeded.result.renderState.etag
            );
        }
    });

    it('fails before progress when retained render-plan content is unavailable', async () => {
        const source = new InMemoryEdgeRenderStateStore();
        const seeded = seedInitialRender(source);
        const store = new MissingRenderPlanStore(source);
        const request = updateRequest(
            seeded,
            exportCompleteSnapshot(renderSnapshot('After', '2'))
        );

        const responses = await collectResponses(executeNonBrowserEdgeRenderUpdateFixture(request, store));

        expect(responses).toHaveLength(1);
        expect(responses[0]?.outcome).toBe('failure');
        if (responses[0]?.outcome === 'failure') {
            expect(responses[0].reason).toBe('content-unavailable');
        }
        expect(responses.some((response) => response.outcome === 'progress')).toBe(false);
        expect(source.readRecord(seeded.result.renderState.stateKey)?.etag).toBe(
            seeded.result.renderState.etag
        );
    });
});

type SeededInitialRender = Extract<CemEdgeSsrHostResponseEnvelope, {
    operation: 'render-initial';
    outcome: 'success';
}>;

function seedInitialRender(store: InMemoryEdgeRenderStateStore): SeededInitialRender {
    const snapshot = renderSnapshot('Before', '1');
    const exported = exportCompleteSnapshot(snapshot);
    const request = createCemEdgeSsrHostRequestEnvelope(
        new CemEdgeSsrJobSequence(),
        'render-initial',
        {
            template: {
                kind: 'serialized-template-source-v1',
                templateArtifactId: exported.templateArtifactId,
                source: PROCESSING_BOUNDARY_TEMPLATE_SOURCE,
            },
            snapshot: exported,
            revision: revisionFromSnapshot(exported),
            sourceMapMode: 'dev',
            scopeUid: 'boundary-server-scope',
        }
    );
    const response = executeNonBrowserSsrInitialRenderFixture(request, store);
    if (response.outcome !== 'success') {
        throw new Error(`failed to seed edge-update fixture: ${response.diagnostics[0]?.message ?? response.reason}`);
    }
    return response;
}

function updateRequest(
    seeded: SeededInitialRender,
    snapshot: ExportedDataIslandSnapshot,
    transform: (input: CemEdgeSsrRenderUpdateInput) => CemEdgeSsrRenderUpdateInput = (input) => input
) {
    const templateAddress = seeded.result.renderState.currentTemplateArtifact;
    if (!templateAddress || templateAddress.kind !== 'template-artifact') {
        throw new Error('seeded fixture did not retain a template artifact');
    }
    const renderPlanAddress = seeded.result.renderState.currentRenderPlan;
    if (renderPlanAddress.kind !== 'render-plan') {
        throw new Error('seeded fixture did not retain a render plan');
    }
    const input = transform({
        template: {
            kind: 'content-addressed-template-artifact-v1',
            templateArtifactId: snapshot.templateArtifactId,
            address: { ...templateAddress, kind: 'template-artifact' },
        },
        snapshot,
        revision: revisionFromSnapshot(snapshot),
        sourceMapMode: 'dev',
        scopeUid: 'boundary-server-scope',
        previousRenderPlan: {
            stateKey: seeded.result.renderState.stateKey,
            expectedEtag: seeded.result.renderState.etag,
            identity: seeded.result.hydrationData.renderPlanIdentity,
            address: { ...renderPlanAddress, kind: 'render-plan' },
        },
    });
    return createCemEdgeSsrHostRequestEnvelope(
        new CemEdgeSsrJobSequence(),
        'render-update',
        input
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

function exportCompleteSnapshot(snapshot: DataIslandSnapshot): ExportedDataIslandSnapshot {
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

function revisionFromSnapshot(snapshot: ExportedDataIslandSnapshot) {
    return {
        instanceId: snapshot.instanceId,
        dataRevision: snapshot.dataRevision,
        templateArtifactId: snapshot.templateArtifactId,
        scopePolicyStamp: snapshot.scopePolicyStamp,
        outputTarget: snapshot.outputTarget,
        ...(snapshot.renderAttempt === undefined ? {} : { renderAttempt: snapshot.renderAttempt }),
    };
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

class MissingRenderPlanStore implements EdgeRenderStateStore {
    constructor(private readonly source: EdgeRenderStateStore) {}

    putContent(kind: EdgeContentKind, value: unknown): EdgeContentAddress {
        return this.source.putContent(kind, value);
    }

    getContent<T = unknown>(address: EdgeContentAddress): T | undefined {
        return address.kind === 'render-plan' ? undefined : this.source.getContent<T>(address);
    }

    readRecord(stateKey: string): EdgeRenderStateRecord | undefined {
        return this.source.readRecord(stateKey);
    }

    writeRecord(
        record: EdgeRenderStateRecord,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.source.writeRecord(record, options);
    }

    writeRenderState(
        input: EdgeRenderStateInput,
        options?: EdgeRenderStateWriteOptions
    ): EdgeRenderStateWriteResult {
        return this.source.writeRenderState(input, options);
    }
}
