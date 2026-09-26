import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';

// Shared test-control values and authored CEMT, not serialized runtime trees.
// eslint-disable-next-line @nx/enforce-module-boundaries -- cross-runtime URL contract fixture.
import cases from '../../../cem_ql/fixtures/url/query-matrix.json';
// eslint-disable-next-line @nx/enforce-module-boundaries -- same authored template as native SSR.
import source from '../../../cem_ql/fixtures/url/render.cemt?raw';
import { evaluateCemQlQuery } from './internal/runtime-support/cem-ql-query.js';
import {
    retainCemMlTemplateSource, renderRetainedCemMlTemplate, disposeRetainedCemMlTemplate,
} from './internal/runtime-support/cem-ql-render.js';
import { applyRenderPlanToRange, type RenderPlan } from './projection.js';

const meta: Meta = { title: 'CEM Elements/CEM-QL URL Integration', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const QueryMatrix: Story = {
    render: () => '<p>URL functions execute through the shared Rust WASM query engine.</p>',
    play: async () => {
        for (const row of cases) {
            const result = await evaluateCemQlQuery(row.query, {});
            expect(result.items, row.id).toEqual(row.items);
            expect(result.diagnostics.map(d => d.code), row.id).toEqual(row.diagnosticCodes);
            expect(result.error !== null, row.id).toBe(row.error);
            for (const diagnostic of result.diagnostics) {
                expect(diagnostic.byteOffset, row.id).toBeTypeOf('number');
                expect(diagnostic.sourceMapRef, row.id).toBeDefined();
            }
        }
    },
};

export const NativeServerTemplateParity: Story = {
    render: () => '<section data-url-output></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('[data-url-output]');
        if (!host) throw new Error('URL output host is missing');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const artifact = await retainCemMlTemplateSource(source, ['path', 'base', 'search']);
        expect(artifact.diagnostics).toEqual([]);
        try {
            let revision = 0;
            for (const [path, search, href, text] of [
                ['../b', 'b=2&a=1&a=3', 'https://h/b?b=2&a=1&a=3', 'a=1&a=3&b=2'],
                ['next', 'x=%3Ctag%3E&x=a+b', 'https://h/a/next?x=%3Ctag%3E&x=a+b', 'x=%3Ctag%3E&x=a+b'],
            ]) {
                const rendered = await renderRetainedCemMlTemplate(artifact.artifactId, {path, base: 'https://h/a/c', search});
                expect(rendered.diagnostics).toEqual([]);
                const plan: RenderPlan = {
                    nodes: rendered.nodes, producedTag: 'url-parity', instanceId: 'url-parity',
                    templateArtifactId: 'url-parity', dataRevision: String(++revision),
                    scopePolicyStamp: 'url-parity', outputTarget: 'light-dom',
                };
                expect(applyRenderPlanToRange(bounds, plan, document).diagnostics).toEqual([]);
                const link = host.querySelector('a');
                expect(link?.getAttribute('href')).toBe(href);
                expect(link?.textContent).toBe(text);
                expect(host.querySelectorAll('a')).toHaveLength(1);
                expect(host.querySelector('tag')).toBeNull();
            }
            const failed = await renderRetainedCemMlTemplate(artifact.artifactId, {path: 'bad', base: 'bad', search: ''});
            expect(failed.diagnostics.map(d => d.code)).toContain('cem.ql.url_base_invalid');
        } finally {
            disposeRetainedCemMlTemplate(artifact.artifactId);
        }
    },
};
