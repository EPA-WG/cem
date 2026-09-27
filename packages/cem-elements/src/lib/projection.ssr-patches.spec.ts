import { expect, it } from 'vitest';
import { diffRenderPlansToPatchFrames, type RenderPlan, type RenderPlanNode } from './projection.js';

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

it.each(['text', 'comment'] as const)('falls back to the owned range for changed root %s nodes', kind => {
    const before = plan([{ kind, text: 'Before', renderNodeId: 'root' }, element([text('Kept')])]);
    const after = plan([{ kind, text: 'After', renderNodeId: 'root' }, element([text('Kept')])]);
    expect(ops(before, after).map(op => op.op)).toEqual(['replaceScope', 'replaceScope']);
    expect(ops(before, before)).toEqual([]);
});
