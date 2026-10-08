import { expect } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { renderCemMlTemplate } from './internal/runtime-support/cem-ql-render.js';

const meta = preview.meta({ title: 'CEM Elements/Attribute Expressions', tags: ['test'] });

export const AdjacentSpansAndDiagnostics = meta.story({
    render: () => '<section aria-label="Attribute expression verification"></section>',
    play: async () => {
        const bindings = { path: '/assets/', image: 'icon.svg' };
        const result = await renderCemMlTemplate('{img @src="{$path}{$image}"}', bindings);
        await expect(result.diagnostics).toEqual([]);
        const image = result.nodes[0];
        if (image?.kind !== 'element') throw new Error('Expected the native image render node');
        await expect(image.attributes.find(attribute => attribute.name === 'src')?.value)
            .toBe('/assets/icon.svg');

        for (const source of ['{img @src="{$path}{$image + }"}', '{img @src="prefix {$image"}']) {
            const invalid = await renderCemMlTemplate(source, bindings);
            const diagnostic = invalid.diagnostics.find(value => value.code === 'cem.ql.render.compile_failed');
            await expect(diagnostic).toBeDefined();
            await expect(diagnostic?.message).toContain('image');
            await expect(diagnostic?.sourceMapRef?.fidelity).toBe('author-byte-exact');
            await expect(diagnostic?.sourceMapRef?.frame).toBe(`cem:${source.indexOf('@src')}`);
        }
    },
});
