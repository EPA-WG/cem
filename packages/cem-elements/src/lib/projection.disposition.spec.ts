import { describe, expect, it } from 'vitest';

import {
    EDGE_RENDER_STATE_VERSION,
    InMemoryEdgeRenderStateStore,
    readEdgeRenderStateContents,
    type EdgeRenderStateRecord,
    type RenderPlan,
} from './projection.js';

// A minimal light-DOM render plan; enough for writeRenderState to store the
// render-plan content that readEdgeRenderStateContents reads back.
const PLAN: RenderPlan = {
    producedTag: 'disp-card',
    instanceId: 'disp-instance-1',
    templateArtifactId: 'disp-artifact-1',
    dataRevision: '1',
    outputTarget: 'light-dom',
    scopePolicyStamp: 'disp-scope',
    nodes: [
        {
            kind: 'element',
            namespace: null,
            tag: 'span',
            attributes: [],
            renderNodeId: 'disp-1',
            children: [{ kind: 'text', text: 'ok' }],
        },
    ],
};

function storedRecord(): { store: InMemoryEdgeRenderStateStore; record: EdgeRenderStateRecord } {
    const store = new InMemoryEdgeRenderStateStore();
    const write = store.writeRenderState({ renderPlan: PLAN });
    if (!write.ok) {
        throw new Error('fixture: writeRenderState failed');
    }
    return { store, record: write.record };
}

describe('readEdgeRenderStateContents — BR-VC-9 disposition on a data/security contract', () => {
    it('accepts a record stamped at the build version (understood)', () => {
        const { store, record } = storedRecord();
        expect(record.schemaVersion).toBe(EDGE_RENDER_STATE_VERSION);
        const result = readEdgeRenderStateContents(store, record);
        expect(result.ok).toBe(true);
    });

    it('accepts a version-less record (BR-EV-5 expand-phase optional)', () => {
        const { store, record } = storedRecord();
        const result = readEdgeRenderStateContents(store, { ...record, schemaVersion: undefined });
        expect(result.ok).toBe(true);
    });

    it('rejects a higher-MINOR record in an application run (data/security → strict)', () => {
        const { store, record } = storedRecord();
        const bumped = { ...record, schemaVersion: bumpMinor(EDGE_RENDER_STATE_VERSION) };
        const result = readEdgeRenderStateContents(store, bumped, 'application');
        expect(result.ok).toBe(false);
        if (!result.ok) {
            expect(result.reason).toBe('schema-version-unsupported');
            expect(result.decision?.disposition).toBe('reject');
        }
    });

    it('rejects a higher-MINOR record in build/SSR', () => {
        const { store, record } = storedRecord();
        const bumped = { ...record, schemaVersion: bumpMinor(EDGE_RENDER_STATE_VERSION) };
        const result = readEdgeRenderStateContents(store, bumped, 'build-ssr');
        expect(result.ok).toBe(false);
    });

    it('tolerates a higher-MINOR record in development (degrade)', () => {
        const { store, record } = storedRecord();
        const bumped = { ...record, schemaVersion: bumpMinor(EDGE_RENDER_STATE_VERSION) };
        const result = readEdgeRenderStateContents(store, bumped, 'development');
        expect(result.ok).toBe(true);
    });

    it('rejects a MAJOR mismatch as must-understand in every mode', () => {
        const { store, record } = storedRecord();
        const bumped = { ...record, schemaVersion: bumpMajor(EDGE_RENDER_STATE_VERSION) };
        for (const mode of ['application', 'build-ssr', 'development'] as const) {
            const result = readEdgeRenderStateContents(store, bumped, mode);
            expect(result.ok).toBe(false);
            if (!result.ok && result.reason === 'schema-version-unsupported') {
                expect(result.decision?.mustUnderstand).toBe(true);
            }
        }
    });
});

function bumpMinor(version: string): string {
    const [major, minor, patch] = version.split('.').map((n) => Number.parseInt(n, 10));
    return `${major}.${minor + 1}.${patch}`;
}

function bumpMajor(version: string): string {
    const [major, minor, patch] = version.split('.').map((n) => Number.parseInt(n, 10));
    return `${major + 1}.${minor}.${patch}`;
}
