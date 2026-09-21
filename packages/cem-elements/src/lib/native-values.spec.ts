import { describe, expect, it } from 'vitest';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, exportNativeCemAttributes, importNativeCemAttributes, type NativeCemValue } from './native-values.js';
import { createCemDeclarationScope, resolveCemValueArtifactLimits } from './declaration-scope.js';
import { diffRenderPlansToPatchFrames, edgeContentAddress, InMemoryEdgeRenderStateStore, type RenderPlan } from './projection.js';

const value = (byte: number): NativeCemValue => ({ kind: 'cem-native-value-v1', artifact: new Uint8Array([67, 69, 77, 86, byte]).buffer, contentHash: `fixture-${byte}`, index: 0 });
const plan = (nativeValue: NativeCemValue): RenderPlan => ({
    instanceId: 'parent', producedTag: 'parent-value', templateArtifactId: 'parent-template', dataRevision: '1', scopePolicyStamp: 'scope', outputTarget: 'light-dom',
    nodes: [{ kind: 'element', tag: 'child-value', namespace: null, renderNodeId: 'child', attributes: [{ name: 'count', value: '2', nativeValue }], children: [] }],
});

describe('CEMT-VALUE-TRANSPORT portable binary control boundary', () => {
    it('retains artifact sharing across named saved-envelope round trips', () => {
        const first = value(1);
        const envelope = JSON.parse(JSON.stringify(exportNativeCemAttributes([{ name: 'a', value: first }, { name: 'b', value: { ...first, index: 1 } }])));
        const restored = importNativeCemAttributes(envelope);
        expect(restored[0].value.artifact).toBe(restored[1].value.artifact);
        expect(restored[1].value.index).toBe(1);
        expect(new Uint8Array(restored[0].value.artifact)).toEqual(new Uint8Array(first.artifact));
        expect(() => importNativeCemAttributes(envelope, { ...DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, maxBytes: 1 })).toThrow();
    });

    it('patches native changes when browser strings are equal and clears removed metadata', () => {
        const before = plan(value(1));
        const after = plan(value(2));
        const frames = diffRenderPlansToPatchFrames(before, after);
        const ops = frames.flatMap(frame => frame.type === 'ops' ? frame.ops : []);
        expect(ops).toEqual([expect.objectContaining({ op: 'setAttribute', name: 'count', value: '2', nativeValue: expect.objectContaining({ contentHash: 'fixture-2' }) })]);
        const plain = plan(value(2));
        if (plain.nodes[0].kind === 'element') delete plain.nodes[0].attributes[0].nativeValue;
        const cleared = diffRenderPlansToPatchFrames(after, plain).flatMap(frame => frame.type === 'ops' ? frame.ops : []);
        expect(cleared).toEqual([{ op: 'setAttribute', target: { kind: 'render-node', id: 'child' }, name: 'count', value: '2' }]);
    });

    it('hashes artifact bytes and preserves them in saved render plans', () => {
        const before = plan(value(1));
        const after = plan({ ...value(2), contentHash: 'fixture-1' });
        expect(edgeContentAddress('render-plan', before).key).not.toBe(edgeContentAddress('render-plan', after).key);
        const store = new InMemoryEdgeRenderStateStore();
        const address = store.putContent('render-plan', before);
        const restored = store.getContent<RenderPlan>(address);
        expect(restored).toEqual(before);
        expect(restored).not.toBe(before);
    });

    it('takes environment ceilings and allows only lower CEM scope overrides', () => {
        const document = {} as Document;
        const root = createCemDeclarationScope({ document, nativeValueLimits: { maxBytes: 4000 } });
        const child = createCemDeclarationScope({ document, parent: root, nativeValueLimits: { maxBytes: 2000, maxValues: 20 } });
        expect(resolveCemValueArtifactLimits({ maxBytes: 8000 }, child)).toEqual({ maxBytes: 2000, maxValues: 20, maxDepth: 128 });
        expect(() => createCemDeclarationScope({ document, parent: child, nativeValueLimits: { maxBytes: 3000 } })).toThrow(/cap_relaxation_denied/);
        expect(() => resolveCemValueArtifactLimits({ maxBytes: 1000 }, root)).toThrow(/cap_relaxation_denied/);
        root.dispose();
    });
});
