import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import {
    assertAuthoredInventory, assertSourceLoadedCoverage, coverageStoryId, demoCoverage, normalizeLegend,
} from '../../.storybook/demo-coverage.js';
import rootPage from '../../index.html?raw';

const demoPages = import.meta.glob('../../demo/**/*.html', { eager: true, query: '?raw', import: 'default' });

export default { title: 'CEM Elements/Authored Demo Inventory', tags: ['test'] } satisfies Meta;

export const EveryPageAndLegend: StoryObj = {
    render: () => document.createElement('section'),
    play: async () => {
        const sources = new Map([['index.html', rootPage], ...Object.entries(demoPages)
            .map(([path, source]): [string, string] => [path.slice('../../'.length), source as string])]);
        const pages = new Map(Array.from(sources, ([path, source]) => {
            // Use the browser HTML parser: comments, scripts and inert template contents
            // must not create phantom samples; entities and quoted attributes are decoded.
            const document = new DOMParser().parseFromString(source, 'text/html');
            return [path, Array.from(document.querySelectorAll('cem-demo-element'), sample =>
                normalizeLegend(sample.getAttribute('legend') ?? ''))];
        }));
        assertAuthoredInventory(pages);
    },
};

export const RejectsMissingSourceAndSamples: StoryObj = {
    render: () => document.createElement('section'),
    play: async () => {
        const entry = demoCoverage[0];
        const id = coverageStoryId(entry);
        const root = document.createElement('section');
        expect(() => assertSourceLoadedCoverage(id, root)).toThrow('expected one source declaration');
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('src', new URL('../../index.html', import.meta.url).href);
        declaration.setAttribute('tag', 'inventory-fixture-host');
        root.append(declaration);
        expect(() => assertSourceLoadedCoverage(id, root)).toThrow('missing produced source-document host');
        const host = document.createElement('inventory-fixture-host');
        root.append(host);
        expect(() => assertSourceLoadedCoverage(id, root)).toThrow('stale contracts');
        for (const legend of entry.legends) {
            const sample = document.createElement('cem-demo-element');
            sample.setAttribute('legend', legend);
            host.append(sample);
        }
        assertSourceLoadedCoverage(id, root);
        host.append(host.firstElementChild?.cloneNode(true) as Node);
        expect(() => assertSourceLoadedCoverage(id, root)).toThrow('duplicate actual');
        root.append(declaration.cloneNode());
        expect(() => assertSourceLoadedCoverage(id, root)).toThrow('expected one source declaration');

        const inert = new DOMParser().parseFromString(`
            <!-- <cem-demo-element legend="Comment"></cem-demo-element> -->
            <script>const example = '<cem-demo-element legend="Script">';</script>
            <template><cem-demo-element legend="Inert"></cem-demo-element></template>
            <cem-demo-element legend='One &amp; two'></cem-demo-element>`, 'text/html');
        expect(Array.from(inert.querySelectorAll('cem-demo-element'), sample => sample.getAttribute('legend')))
            .toEqual(['One & two']);
    },
};
