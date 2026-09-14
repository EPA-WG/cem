import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
// eslint-disable-next-line @nx/enforce-module-boundaries -- initialize the same generated WASM used by the production adapter.
import { initSync } from '../../../cem_ql/dist/wasm/cem_ql.js';
import { renderCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import { diffRenderPlansToPatchFrames, type RenderPlanNode, type RenderPlan } from './projection.js';

describe('repeated native expression patch identities', () => {
    beforeAll(() => {
        initSync({ module: readFileSync(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
    });
    it('addresses each changed table cell independently while preserving its shared source frame', async () => {
        const source = '{table | {tbody | {cem:for-each @select=values @as=value | {tr | {td | {$value}}}}}}';
        const before = await renderCemMlTemplate(source, { values: ['10', '2', '3'] });
        const after = await renderCemMlTemplate(source, { values: ['2', '3', '10'] });
        const flatten = (nodes: RenderPlanNode[]): RenderPlanNode[] => nodes.flatMap((node) =>
            [node, ...(node.kind === 'element' ? flatten(node.children) : [])]);
        const text = flatten(before.nodes).filter((node) => node.kind === 'text');
        expect(new Set(text.map((node) => node.sourceMapRef?.frame)).size).toBe(1);
        const identity: Omit<RenderPlan, 'nodes'> = {
            producedTag: 'test-table', instanceId: 'one', dataRevision: '1',
            templateArtifactId: 'table', scopePolicyStamp: 'test', outputTarget: 'light-dom',
        };
        const frames = diffRenderPlansToPatchFrames({ ...identity, nodes: before.nodes },
            { ...identity, dataRevision: '2', nodes: after.nodes });
        const changes = frames.flatMap((frame) => frame.type === 'ops' ? frame.ops : [])
            .filter((op) => op.op === 'setText');
        expect(changes).toHaveLength(3);
        expect(new Set(changes.map((op) => op.target.id)).size).toBe(3);
    });
});
