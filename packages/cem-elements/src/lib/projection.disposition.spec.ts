import { describe, expect, it, vi } from 'vitest';

import {
    EDGE_RENDER_STATE_VERSION,
    edgeContentAddress,
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


describe('retained Edge stylesheet state', () => {
    it('stores an optional content-addressed stylesheet record and includes it in the ETag', () => {
        const store = new InMemoryEdgeRenderStateStore();
        const stylesheetState = { kind: 'native-ssr-stylesheets-v1', inputKey: 'inputs', instanceStylesheetHtml: '<style>p{}</style>' };
        const write = store.writeRenderState({ renderPlan: PLAN, stylesheetState });
        if (!write.ok) throw new Error('failed state write');
        expect(write.record.currentStylesheets).toEqual(edgeContentAddress('stylesheets', stylesheetState));
        expect(readEdgeRenderStateContents(store, write.record)).toMatchObject({ ok: true, contents: { stylesheetState } });
        const changed = store.writeRenderState({ renderPlan: PLAN, stylesheetState: { ...stylesheetState, inputKey: 'changed' } });
        if (!changed.ok) throw new Error('failed changed state write');
        expect(changed.record.etag).not.toBe(write.record.etag);
    });

    it('rejects unavailable or corrupted stylesheet content', () => {
        const { store, record } = storedRecord();
        const address = edgeContentAddress('stylesheets', { inputKey: 'missing' });
        expect(readEdgeRenderStateContents(store, { ...record, currentStylesheets: address })).toMatchObject({
            ok: false, reason: 'missing-content', field: 'currentStylesheets',
        });
        const actual = store.putContent('stylesheets', { inputKey: 'present' });
        const getContent = store.getContent.bind(store);
        vi.spyOn(store, 'getContent').mockImplementation(address => address.kind === 'stylesheets' ? { inputKey: 'corrupt' } : getContent(address));
        expect(readEdgeRenderStateContents(store, { ...record, currentStylesheets: actual })).toMatchObject({
            ok: false, reason: 'content-address-mismatch', field: 'currentStylesheets',
        });
    });
});
