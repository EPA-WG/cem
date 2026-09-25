import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
const SOURCE_TAG = 'story-xpath-maps-arrays-document';
const DEMO_URL = new URL('../../demo/xpath-maps-arrays.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Maps and Arrays', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const OptionalEntriesAndRetainedMembers: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('map/array demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        const values = (article: Element) => Array.from(article.querySelectorAll('output'), item => item.textContent);
        const edit = (control: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement, value: string, caret = value.length) => {
            control.focus();
            control.value = value;
            if (!(control instanceof HTMLSelectElement)) control.setSelectionRange(caret, caret);
            control.dispatchEvent(new Event(control instanceof HTMLSelectElement ? 'change' : 'input', { bubbles: true }));
        };
        const expectControl = (article: Element, selector: string, control: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
            value: string, caret = value.length) => {
            expect(article.querySelector(selector)).toBe(control);
            expect(control.value).toBe(value);
            expect(document.activeElement).toBe(control);
            if (!(control instanceof HTMLSelectElement)) {
                expect([control.selectionStart, control.selectionEnd]).toEqual([caret, caret]);
            }
        };
        const expectValues = async (article: Element, expected: string[]) => {
            await waitFor(() => expect(values(article)).toEqual(expected));
            await whenCemSourceRendered(root);
            expect(values(article)).toEqual(expected);
            expect(article.querySelector('[role="alert"]')).toBeNull();
        };
        await waitFor(() => expect(values(sample('1. An IP-filter map with an optional note')))
            .toEqual(['allow: 192.0.2.0/24', 'Absent entry', '2']), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const filter = sample('1. An IP-filter map with an optional note');
        const basket = sample('2. Select a retained fruit by array position');
        const imported = sample('3. Query an imported JSON tree');
        await step('1. An IP-filter map with an optional note', async () => {
            const address = filter.querySelector('input') as HTMLInputElement;
            const action = filter.querySelector('select[aria-label="Action"]') as HTMLSelectElement;
            const note = filter.querySelector('select[aria-label="Note entry"]') as HTMLSelectElement;
            expect([address.value, action.value, note.value]).toEqual(['192.0.2.0/24', 'allow', 'absent']);
            for (const value of ['', '  🍒 & <local>  ', '198.51.100.0/24']) {
                const caret = Math.min(4, value.length);
                edit(address, value, caret);
                await expectValues(filter, [`allow: ${value}`, 'Absent entry', '2']);
                expectControl(filter, 'input', address, value, caret);
            }
            edit(action, 'deny');
            await expectValues(filter, ['deny: 198.51.100.0/24', 'Absent entry', '2']);
            expectControl(filter, 'select[aria-label="Action"]', action, 'deny');
            for (const [state, expected, count] of [['empty', 'Present, empty sequence', '3'],
                ['value', 'Local preview', '3'], ['absent', 'Absent entry', '2']]) {
                edit(note, state);
                await expectValues(filter, ['deny: 198.51.100.0/24', expected, count]);
                expectControl(filter, 'select[aria-label="Note entry"]', note, state);
                expect(action.value).toBe('deny');
                expect(address.value).toBe('198.51.100.0/24');
            }
            expect(values(basket)).toEqual(['2', 'apple: 2', 'Empty member (array size 1)']);
            expect(values(imported)).toEqual(['3 members; numeric total 5', 'Null value']);
        });
        await step('2. Select a retained fruit by array position', async () => {
            const position = basket.querySelector('input') as HTMLInputElement;
            const xml = basket.querySelector('textarea') as HTMLTextAreaElement;
            expect(position.value).toBe('1');
            expect(xml.value).toBe('<basket><apple>2</apple><pear>3</pear></basket>');
            const pick = async (value: string, expected: string) => {
                edit(position, value);
                await expectValues(basket, ['2', expected, 'Empty member (array size 1)']);
                expectControl(basket, 'input', position, value);
            };
            await pick('2', 'pear: 3');
            for (const invalid of ['0', '3', '-1', '1.5', '1e0', 'bad', '']) {
                await pick(invalid, 'No member at this position');
                await pick('1', 'apple: 2');
            }
            for (const [position, label] of [[' 02 ', 'pear: 3'], ['+1', 'apple: 2'], ['001', 'apple: 2']]) {
                await pick(position, label);
            }
            await pick('3', 'No member at this position');
            const source = async (value: string, expected: string[], caret = value.length) => {
                edit(xml, value, caret);
                await expectValues(basket, expected);
                expectControl(basket, 'textarea', xml, value, caret);
                expect(basket.querySelector('input')).toBe(position);
            };
            await source('<basket note="Fresh"><apple>2</apple><pear>3</pear><plum>4</plum></basket>',
                ['3', 'plum: 4', 'Note: Fresh'], 8);
            expect(position.value).toBe('3');
            edit(position, '1');
            await expectValues(basket, ['3', 'apple: 2', 'Note: Fresh']);
            await source('<basket xmlns:p="urn:fruit" note=" Fresh &amp; ripe "><!--skip--><?skip it?><p:plum> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </p:plum><pear>3</pear></basket>',
                ['2', 'plum:  ABC D ', 'Note:  Fresh & ripe ']);
            await source('<basket note=""/>', ['0', 'No member at this position', 'Note: ']);
            await source('<basket/>', ['0', 'No member at this position', 'Empty member (array size 1)']);
            const recovered = '<basket><cherry>5</cherry></basket>';
            const recover = () => source(recovered, ['1', 'cherry: 5', 'Empty member (array size 1)']);
            await recover();
            for (const invalid of ['<basket>', '<other/>', '<basket xmlns="urn:fruit"><apple>2</apple></basket>']) {
                edit(xml, invalid);
                await waitFor(() => expect(basket.querySelector('[role="alert"]')?.textContent)
                    .toContain(invalid === '<basket>' ? 'XML' : 'Use a basket root'));
                expect(values(basket)).toEqual([]);
                expectControl(basket, 'textarea', xml, invalid);
                await recover();
            }
            expect(values(filter)).toEqual(['deny: 198.51.100.0/24', 'Absent entry', '2']);
            expect(values(imported)).toEqual(['3 members; numeric total 5', 'Null value']);
        });
        await step('3. Query an imported JSON tree', async () => {
            const json = imported.querySelector('textarea') as HTMLTextAreaElement;
            const source = async (value: string, expected: string[], caret = value.length) => {
                edit(json, value, caret);
                await expectValues(imported, expected);
                expectControl(imported, 'textarea', json, value, caret);
            };
            await source('{"cherry":4,"note":""}', ['2 members; numeric total 4', 'Empty string'], 5);
            for (const [value, expected] of [
                ['{"plum":1e2}', ['1 members; numeric total 100', 'Absent member']],
                ['{"cherry":1.25,"pear":2.5,"note":"  Fresh 🍒  "}', ['3 members; numeric total 3.75', '  Fresh 🍒  ']],
                ['{"plum":-2,"pear":1e2,"note":null}', ['3 members; numeric total 98', 'Null value']],
                ['{"nested":{"pear":99},"array":[100],"flag":true,"text":"2","plum":4,"note":""}',
                    ['6 members; numeric total 4', 'Empty string']],
                ['{}', ['0 members; numeric total 0', 'Absent member']],
            ] as const) {
                await source(value, [...expected]);
            }
            const recover = () => source('{"note":null}', ['1 members; numeric total 0', 'Null value']);
            for (const invalid of ['{', '[]', 'null', '42', '"fruit"']) {
                edit(json, invalid);
                await waitFor(() => expect(imported.querySelector('[role="alert"]')?.textContent)
                    .toContain(invalid === '{' ? 'JSON' : 'Use a JSON object'));
                expect(values(imported)).toEqual([]);
                expectControl(imported, 'textarea', json, invalid);
                await recover();
            }
            expect(values(filter)).toEqual(['deny: 198.51.100.0/24', 'Absent entry', '2']);
            expect(values(basket)).toEqual(['1', 'cherry: 5', 'Empty member (array size 1)']);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-maps-arrays.cemt', './xpath-validation.html', './xpath-sequences.html',
                './xpath-aggregates.html', './data-table.html'].map(relative => new URL(relative, DEMO_URL).href));
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
