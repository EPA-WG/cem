import { expect, it } from 'vitest';
import { edgeContentAddress, diffRenderPlansToPatchFrames, type RenderPlan, type RenderPlanNode } from './projection.js';

const text = (value: string): RenderPlanNode => ({ kind: 'text', text: value, renderNodeId: 'text-1' });
const element = (children: RenderPlanNode[]): RenderPlanNode => ({ kind: 'element', namespace: null,
    tag: 'p', attributes: [], renderNodeId: 'paragraph', children });
const plan = (nodes: RenderPlanNode[]): RenderPlan => ({ producedTag: 'ssr-card', instanceId: 'instance',
    templateArtifactId: 'template', dataRevision: '1', outputTarget: 'light-dom', scopePolicyStamp: 'policy', nodes });
function ops(before: RenderPlan, after: RenderPlan, textNodeIdsAvailable = false) {
    return diffRenderPlansToPatchFrames(before, after, { textNodeIdsAvailable })
        .flatMap(frame => frame.type === 'ops' ? frame.ops : []);
}

it('reconciles children under serialized element IDs instead of targeting unretained text IDs', () => {
    const before = plan([element([text('Before')])]);
    const after = plan([element([text('After')])]);
    expect(ops(before, after)).toMatchObject([{ op: 'reconcileChildren', target: { id: 'paragraph' },
        children: [{ node: { kind: 'text', text: 'After' } }] }]);
    expect(ops(before, after, true)).toMatchObject([{ op: 'setText', target: { id: 'text-1' }, value: 'After' }]);
});

it('reconciles only the nearest element containing changed text and comments', () => {
    const before = plan([element([{ kind: 'element', namespace: null, tag: 'span', attributes: [], renderNodeId: 'inner',
        children: [text('Before'), { kind: 'comment', text: 'before', renderNodeId: 'comment' }] }])]);
    const after = structuredClone(before);
    const root = after.nodes[0];
    if (root.kind !== 'element' || root.children[0].kind !== 'element') throw new Error('bad fixture');
    root.children[0].children = [text('After'), { kind: 'comment', text: 'after', renderNodeId: 'comment' }];
    expect(ops(before, after)).toMatchObject([{ op: 'reconcileChildren', target: { id: 'inner' } }]);
});

it.each(['text', 'comment'] as const)('reconciles the owned range for changed root %s nodes without targeting unretained IDs', kind => {
    const before = plan([{ kind, text: 'Before', renderNodeId: 'root' }, element([text('Kept')])]);
    const after = plan([{ kind, text: 'After', renderNodeId: 'root' }, element([text('Kept')])]);
    expect(ops(before, after).map(op => op.op)).toEqual(['reconcileRoots']);
    expect(ops(before, before)).toEqual([]);
});

it('bounds repeated native binary hashing and preserves content identity independently of buffer sharing', () => {
    const bytes = new Uint8Array(512 * 1024); bytes[0] = 23;
    expect(edgeContentAddress('render-plan', { bytes: bytes.buffer }).algorithm).toBe('stable-json-fnv1a64-binary-v2');
    const shared = { first: bytes.buffer, second: bytes.buffer };
    const cloned = { first: bytes.buffer.slice(0), second: bytes.buffer.slice(0) };
    const original = edgeContentAddress('render-plan', shared);
    expect(edgeContentAddress('render-plan', cloned)).toEqual(original);
    const repeated = Array.from({ length: 1024 }, () => ({ artifact: bytes.buffer, contentHash: 'untrusted-metadata' }));
    expect(edgeContentAddress('render-plan', repeated).digest).toMatch(/^[0-9a-f]{16}$/);
    bytes[bytes.length - 1] = 42;
    expect(edgeContentAddress('render-plan', shared)).not.toEqual(original);
    const plain = edgeContentAddress('render-plan', { name: 'plain', enabled: true });
    expect(plain.algorithm).toBe('stable-json-fnv1a64-v1');
    expect(edgeContentAddress('render-plan', { enabled: true, name: 'plain' })).toEqual(plain);
});
