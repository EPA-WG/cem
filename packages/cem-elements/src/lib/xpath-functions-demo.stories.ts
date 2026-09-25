import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
const SOURCE_TAG = 'story-xpath-functions-document';
const DEMO_URL = new URL('../../demo/xpath-functions.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Function Libraries', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const SeparateLibraryAndLiveSlices: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('source-loaded demo host is missing');
        const sample = (legend: string) => {
            const element = root.querySelector(`cem-demo-element[legend="${legend}"]`);
            if (!element) throw new Error(`missing case: ${legend}`);
            return element;
        };
        const output = (card: Element) => card.querySelector('output')?.textContent;
        const items = (card: Element) => Array.from(card.querySelectorAll('li'), item => item.textContent);
        const edit = (control: HTMLInputElement | HTMLTextAreaElement, value: string, caret = value.length) => {
            control.focus();
            control.value = value;
            control.setSelectionRange(caret, caret);
            control.dispatchEvent(new Event('input', { bubbles: true }));
        };
        const expectEditor = (card: Element, control: HTMLInputElement | HTMLTextAreaElement, value: string, caret = value.length) => {
            expect(card.querySelector(control.localName)).toBe(control);
            expect(control.value).toBe(value);
            expect(document.activeElement).toBe(control);
            expect([control.selectionStart, control.selectionEnd]).toEqual([caret, caret]);
        };
        await waitFor(() => expect(root.querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        await whenCemSourceRendered(root);
        for (const [index, legend] of ['1. Named XPath function', '1a. CEM-QL string pair'].entries()) {
            await step(legend, async () => {
                const card = sample(legend);
                const input = card.querySelector('input') as HTMLInputElement;
                expect(input.value).toBe('Hello');
                expect(output(card)).toBe('Hello 🍒');
                edit(input, 'Changed 🍋', 3);
                await waitFor(() => expect(output(card)).toBe('Changed 🍋 🍒'));
                expectEditor(card, input, 'Changed 🍋', 3);
                for (const value of ['  🍋 & <é>  ', '', index === 0 ? 'First' : 'Second']) {
                    edit(input, value);
                    await waitFor(() => expect(output(card)).toBe(`${value} 🍒`));
                    expectEditor(card, input, value);
                }
                expect(output(sample(index === 0 ? '1a. CEM-QL string pair' : '1. Named XPath function')))
                    .toBe(index === 0 ? 'Hello 🍒' : 'First 🍒');
            });
        }
        for (const [index, legend] of ['2. Shared XPath predicate', '2a. CEM-QL predicate pair'].entries()) {
            await step(legend, async () => {
                const card = sample(legend);
                const input = card.querySelector('input') as HTMLInputElement;
                expect(input.value).toBe('cherry');
                expect(output(card)).toBe('cherry 🍒');
                for (const value of ['lemon', 'Cherry', ' cherry ', '']) {
                    edit(input, value);
                    await waitFor(() => expect(output(card)).toBe('Try cherry'));
                    expectEditor(card, input, value);
                    edit(input, 'cherry', 2);
                    await waitFor(() => expect(output(card)).toBe('cherry 🍒'));
                    expectEditor(card, input, 'cherry', 2);
                }
                if (index === 1) {
                    edit(input, 'lemon');
                    await waitFor(() => expect(output(card)).toBe('Try cherry'));
                }
                expect(output(sample(index === 0 ? '2a. CEM-QL predicate pair' : '2. Shared XPath predicate'))).toBe('cherry 🍒');
            });
        }
        for (const [index, legend] of ['3. XML nodes and matching', '3a. CEM-QL native node pair'].entries()) {
            await step(legend, async () => {
                const card = sample(legend);
                const xml = card.querySelector('textarea') as HTMLTextAreaElement;
                expect(xml.value).toBe('<r><item qty="2">Cherry</item></r>');
                expect(items(card)).toEqual(['Cherry: stocked']);
                const valid = async (source: string, expected: string[], caret = source.length) => {
                    edit(xml, source, caret);
                    await waitFor(() => expect(items(card)).toEqual(expected));
                    await whenCemSourceRendered(root);
                    expect(card.querySelector('[role="alert"]')).toBeNull();
                    expectEditor(card, xml, source, caret);
                };
                await valid('<r><item qty="1.999">Below</item><item qty="2">At</item><item qty=" 2.5 ">Above</item><item qty="-1">Negative</item><item>Missing</item></r>',
                    ['Below: low stock', 'At: stocked', 'Above: stocked', 'Negative: low stock', 'Missing: low stock'], 12);
                await valid('<r xmlns:p="urn:fruit"><!--skip--><?skip it?><p:item qty="9">Qualified</p:item><group><item qty="9">Nested</item></group><item p:qty="9">Qualified qty</item><item qty="2"> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </item></r>',
                    ['Qualified qty: low stock', ' ABC D : stocked']);
                const recovered = index === 0 ? 'Recovered XPath' : 'Recovered CEM-QL';
                const recover = () => valid(`<r><item qty="2">${recovered}</item></r>`, [`${recovered}: stocked`]);
                for (const empty of ['<r/>', '<r xmlns="urn:fruit"><item qty="2">Qualified root</item></r>',
                    '<other><r><item qty="2">Nested root</item></r></other>']) {
                    await valid(empty, []);
                    expect(card.querySelector('ul')).not.toBeNull();
                    await recover();
                }
                for (const invalid of ['<r>', '<r><item></r>']) {
                    edit(xml, invalid);
                    await waitFor(() => expect(card.querySelector('[role="alert"]')?.textContent).toContain('XML'));
                    expect(items(card)).toEqual([]);
                    expect(card.querySelector('ul')).toBeNull();
                    expectEditor(card, xml, invalid);
                    await recover();
                }
                expect(items(sample(index === 0 ? '3a. CEM-QL native node pair' : '3. XML nodes and matching')))
                    .toEqual(index === 0 ? ['Cherry: stocked'] : ['Recovered XPath: stocked']);
            });
        }
        const links = ['../index.html', './xpath-functions.cemt', '#cem-ql-use-cases', '#cem-ql-label',
            '#cem-ql-predicate', '#cem-ql-nodes', '#cem-ql-import', './xpath-nodes.html', './dom-merge.html',
            './functions/str.html', './functions/str.html', './cell-overrides.html', './functions/dom.html'];
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(links.map(relative => new URL(relative, relative.startsWith('#') ? location.href : DEMO_URL).href));
        for (const fragment of links.filter(link => link.startsWith('#'))) {
            expect(root.querySelector(`a[href="${fragment}"]`)).not.toBeNull();
            expect(root.querySelector(fragment)).not.toBeNull();
        }
        await whenCemSourceRendered(root);
        expect(cemDiagnosticCodes(root)).toEqual([]);
        for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
            expect(cemDiagnosticCodes(declaration)).toEqual([]);
            const tag = declaration.getAttribute('tag');
            if (tag) for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                expect(cemDiagnosticCodes(instance)).toEqual([]);
            }
        }
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
