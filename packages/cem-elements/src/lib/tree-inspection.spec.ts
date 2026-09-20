import { readFileSync } from 'node:fs';
import { beforeAll, describe, expect, it } from 'vitest';
// eslint-disable-next-line @nx/enforce-module-boundaries -- exercise the production CEM-QL WASM import and writer.
import { initSync } from '../../../cem_ql/dist/wasm/cem_ql.js';
import { renderCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';
import type { RenderPlanNode } from './projection.js';

const textContent = (nodes: RenderPlanNode[]): string => nodes.map((node) =>
    node.kind === 'text' ? node.text : node.kind === 'element' ? textContent(node.children) : '').join('');

describe('retained CEM inspection through the production WASM adapter', () => {
    beforeAll(() => {
        initSync({ module: readFileSync(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
    });

    for (const [format, source] of [
        ['xml', '<r><![CDATA[<raw>🍒]]><?keep inert?></r>'],
        ['json', '{"fruit":"<raw>🍒"}'],
        ['yaml', "fruit: '<raw>🍒'"],
        ['csv', 'fruit\n<raw>🍒\n'],
    ]) {
        it(`imports ${format} in CEM-ML and presents its retained owner as inert CEM text`, async () => {
            const result = await renderCemMlTemplate(
                '{cem-data @name=document @select=source @type="{$format}"}' +
                '{pre | {code | {$cemml:inspect(document.root)}}}',
                { source, format },
            );
            expect(result.diagnostics).toEqual([]);
            const output = textContent(result.nodes);
            expect(output).toContain('{ast');
            expect(output).toContain('<raw>🍒');
            expect(output).not.toContain('\u001b');
            const pre = result.nodes.find((node) => node.kind === 'element' && node.tag === 'pre');
            expect(pre?.kind).toBe('element');
            if (pre?.kind !== 'element') throw new Error('missing inspection display');
            const code = pre.children.find((node) => node.kind === 'element' && node.tag === 'code');
            if (code?.kind !== 'element') throw new Error('missing code display');
            expect(code.children.every((node) => node.kind === 'text')).toBe(true);
            expect(code.children.some((node) => node.sourceMapRef !== undefined)).toBe(true);
            if (format === 'xml') {
                expect(output).toContain('@kind=cdata');
                expect(output).toContain('@target=keep');
            }
        });
    }
});
