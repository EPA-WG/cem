import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
const SOURCE_TAG = 'story-xpath-functions-document';
const DEMO_URL = new URL('../../demo/xpath-functions.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Function Libraries', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const SeparateLibraryAndLiveSlices: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('source-loaded demo host is missing');
        const sample = (legend: string) => {
            const element = root.querySelector(`cem-demo-element[legend="${legend}"]`);
            if (!element) throw new Error(`missing case: ${legend}`);
            return element;
        };
        await waitFor(() => expect(root.querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        for (const legend of ['1. Named XPath function', '1a. CEM-QL string pair']) {
            const card = sample(legend);
            const input = card.querySelector('input');
            if (!input) throw new Error('string input is missing');
            expect(card.querySelector('output')?.textContent?.trim()).toBe('Hello 🍒');
            input.focus();
            input.value = 'Changed 🍋';
            input.setSelectionRange(3, 3);
            input.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(card.querySelector('output')?.textContent?.trim()).toBe('Changed 🍋 🍒'));
            expect(card.querySelector('input')).toBe(input);
            expect(document.activeElement).toBe(input);
            expect(input.selectionStart).toBe(3);
            if (legend.startsWith('1.')) expect(sample('1a. CEM-QL string pair').querySelector('output')?.textContent?.trim()).toBe('Hello 🍒');
        }
        for (const legend of ['2. Shared XPath predicate', '2a. CEM-QL predicate pair']) {
            const card = sample(legend);
            const input = card.querySelector('input');
            if (!input) throw new Error('predicate input is missing');
            for (const [value, expected] of [['lemon', 'Try cherry'], ['cherry', 'cherry 🍒']]) {
                input.value = value;
                input.dispatchEvent(new Event('input', { bubbles: true }));
                await waitFor(() => expect(card.querySelector('output')?.textContent?.trim()).toBe(expected));
            }
        }
        for (const legend of ['3. XML nodes and matching', '3a. CEM-QL native node pair']) {
            const card = sample(legend);
            await waitFor(() => expect(card.querySelector('textarea')).not.toBeNull());
            const xml = card.querySelector('textarea');
            if (!xml) throw new Error('XML reader control is missing');
            const items = () => Array.from(card.querySelectorAll('li'), item => item.textContent?.trim());
            await waitFor(() => expect(items()).toEqual(['Cherry: stocked']));
            xml.focus();
            xml.value = '<r><item qty="1">Lemon</item><item qty=" 2.5 ">Grape</item></r>';
            xml.setSelectionRange(12, 12);
            xml.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(items()).toEqual(['Lemon: low stock', 'Grape: stocked']));
            expect(card.querySelector('textarea')).toBe(xml);
            expect(document.activeElement).toBe(xml);
            expect(xml.selectionStart).toBe(12);
            if (legend.startsWith('3.')) expect(sample('3a. CEM-QL native node pair').querySelector('li')?.textContent?.trim()).toBe('Cherry: stocked');
            xml.value = '<r>';
            xml.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(card.querySelector('[role="alert"]')?.textContent).toBeTruthy());
            expect(items()).toEqual([]);
            xml.value = '<r><item qty="2">Recovered</item></r>';
            xml.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(items()).toEqual(['Recovered: stocked']));
            expect(card.querySelector('[role="alert"]')).toBeNull();
            expect(card.querySelector('textarea')).toBe(xml);
        }
        expect(root.querySelector('a[href="#cem-ql-import"]')).not.toBeNull();
        expect(root.querySelector('#cem-ql-import')?.textContent).toContain('source metadata');
    },
};


export const FallbackRetriesAndSharesDependency: Story = {
    render: () => '<section aria-label="XPath library fallback lifecycle"></section>',
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector('section');
        if (!root) throw new Error('fallback fixture root is missing');
        const scope = createCemDeclarationScope({ document });
        const libraryUrl = new URL('../../demo/xpath-functions.cemt', import.meta.url).href;
        const library = await (await fetch(libraryUrl)).text();
        let loads = 0;
        const runtime = new CemElementRuntime({
            declarationTag: 'cem-element-xpath-fallback-story', declarationScope: scope,
            processingWorkerFactory: () => { throw new Error('fixture selects fallback'); },
            resolveScopedModuleUrl: async (request) => {
                expect(request.purpose).toBe('template-import');
                expect(request.authoredSpecifier).toBe('fruit-functions');
                return libraryUrl;
            },
            loadSrcDocument: async (uri) => {
                expect(uri).toBe(libraryUrl);
                if (++loads === 1) throw new Error('library temporarily unavailable');
                return library;
            },
        });
        runtime.install(window);
        const mount = async (tag: string, reference = 'xpath-functions="fruit-functions"', xml = false) => {
            const declaration = document.createElement('cem-element-xpath-fallback-story');
            declaration.setAttribute('tag', tag);
            declaration.innerHTML = `<template type="text/cem-ml" ${reference}>
{attribute @name=text}
${xml ? `{cem-data @name=document @select=text @type=xml @projection=xpath}
{cem:for-each @as=item @select='native:call("demo.items", document.root)' |
    {output | {$item}}}` : '{output | {$native:call("demo.label", text)}}'}
</template>`;
            root.append(declaration);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const instance = document.createElement(tag);
            instance.setAttribute('text', xml ? '<r><item>First</item></r>' : 'First');
            root.append(instance);
            await runtime.whenRenderSettled(instance);
            return instance;
        };
        try {
            const first = await mount('story-xpath-fallback-first');
            expect(runtime.diagnosticsFor(first).some((d) => d.message.includes('temporarily unavailable'))).toBe(true);
            first.setAttribute('text', 'Retried');
            await waitFor(() => expect(first.querySelector('output')?.textContent).toBe('Retried 🍒'));
            const second = await mount('story-xpath-fallback-second');
            await waitFor(() => expect(second.querySelector('output')?.textContent).toBe('First 🍒'));
            first.setAttribute('text', 'Changed');
            await waitFor(() => expect(first.querySelector('output')?.textContent).toBe('Changed 🍒'));
            expect(second.querySelector('output')?.textContent).toBe('First 🍒');
            expect(loads).toBe(2);
            const xml = await mount('story-xpath-fallback-xml', 'xpath-functions="fruit-functions"', true);
            const otherXml = await mount('story-xpath-fallback-other-xml', 'xpath-functions="fruit-functions"', true);
            expect(xml.querySelector('output')?.textContent).toBe('First');
            expect(otherXml.querySelector('output')?.textContent).toBe('First');
            xml.setAttribute('text', '<r><item>Changed XML</item></r>');
            await waitFor(() => expect(xml.querySelector('output')?.textContent).toBe('Changed XML'));
            expect(otherXml.querySelector('output')?.textContent).toBe('First');
            const unbound = await mount('story-xpath-fallback-unbound', '');
            expect(runtime.diagnosticsFor(unbound).some((d) => d.code === 'cem.ql.native_function_unavailable')).toBe(true);
            expect(loads).toBe(2);
        } finally {
            root.replaceChildren();
            scope.dispose();
        }
    },
};
