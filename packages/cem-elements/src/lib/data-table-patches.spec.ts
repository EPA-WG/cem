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

    it('keeps repeated disclosures and their text identities when conditional labels disappear', async () => {
        const source = `{cem:for-each @select='("first", "second")' @as=name |
            {section | {label | {$name}{cem:if @test=selected | {strong | Selected}}}
                {details @open=open | {summary | {$name}}{p | Details}}}}`;
        const before = await renderCemMlTemplate(source, { selected: true });
        const after = await renderCemMlTemplate(source, { selected: false });
        expect(before.diagnostics).toEqual([]);
        expect(after.diagnostics).toEqual([]);
        const retained = (nodes: RenderPlanNode[]) => flatten(nodes).flatMap(node =>
            node.kind === 'element' && node.tag === 'details' ? flatten([node]) : []);
        expect(retained(before.nodes).map(node => node.sourceMapRef)).toEqual(retained(after.nodes).map(node => node.sourceMapRef));
        expect(retained(before.nodes).map(node => node.renderNodeId)).toEqual(retained(after.nodes).map(node => node.renderNodeId));
        for (const result of [before, after]) {
            const nodes = flatten(result.nodes);
            expect(new Set(nodes.map(node => node.renderNodeId)).size).toBe(nodes.length);
        }
    });

    it('keeps the authored JSON tree branch identities across parent deselection', async () => {
        const source = readFileSync(new URL('../../demo/data-tree-view.cemt', import.meta.url), 'utf8');
        const data = (selected: boolean) => ({
            format: 'json',
            datadom: {
                payload: { nodes: { text: '{"fruit":"🍒","note":"","tags":["red","sweet"]}' } },
                slices: { 'branch.1.3': selected ? 'edit-0' : '', 'branch.1.3.1.2': 'edit-0' },
                eventPayloads: {},
            },
        });
        const before = await renderCemMlTemplate(source, data(true));
        const after = await renderCemMlTemplate(source, data(false));
        expect(before.diagnostics).toEqual([]);
        expect(after.diagnostics).toEqual([]);
        const details = (nodes: RenderPlanNode[]) => flatten(nodes).filter(node => node.kind === 'element' && node.tag === 'details');
        expect(details(before.nodes)).toHaveLength(9);
        expect(details(before.nodes).map(node => node.renderNodeId)).toEqual(details(after.nodes).map(node => node.renderNodeId));
    });
});

function flatten(nodes: RenderPlanNode[]): RenderPlanNode[] {
    return nodes.flatMap(node => [node, ...(node.kind === 'element' ? flatten(node.children) : [])]);
}
