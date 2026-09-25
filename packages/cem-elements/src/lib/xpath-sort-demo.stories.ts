import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
const SOURCE_TAG = 'story-xpath-sort-document';
const DEMO_URL = new URL('../../demo/xpath-sort.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Sorting', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const TypedKeysAndSourceSelection: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('sort demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        await waitFor(() => expect(sample('1. Text and numeric keys').querySelector('output')?.textContent)
            .toBe('02 / 1 / 10 / 2 / bad'), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const words = sample('1. Text and numeric keys');
        const table = sample('2. Multiple keys and source selection');
        const output = () => words.querySelector('output')?.textContent;
        const rows = () => Array.from(table.querySelectorAll('tbody tr'), row => [
            (row.querySelector('button') as HTMLButtonElement).value,
            row.querySelector('button')?.textContent,
            ...Array.from(row.querySelectorAll('td'), cell => cell.textContent),
        ]);
        const selection = () => Array.from(table.querySelectorAll('output'), node => node.textContent);
        const edit = (control: HTMLInputElement | HTMLTextAreaElement, value: string, caret = value.length) => {
            control.focus();
            control.value = value;
            control.setSelectionRange(caret, caret);
            control.dispatchEvent(new Event('input', { bubbles: true }));
        };
        const expectEditor = (article: Element, selector: string, control: HTMLInputElement | HTMLTextAreaElement, value: string, caret = value.length) => {
            expect(article.querySelector(selector)).toBe(control);
            expect(control.value).toBe(value);
            expect(document.activeElement).toBe(control);
            expect([control.selectionStart, control.selectionEnd]).toEqual([caret, caret]);
        };
        const toggle = async (article: Element, selector: string, control: HTMLInputElement, checked: boolean) => {
            control.focus();
            if (control.checked !== checked) control.click();
            await whenCemSourceRendered(root);
            expect(article.querySelector(selector)).toBe(control);
            expect(control.checked).toBe(checked);
            expect(document.activeElement).toBe(control);
        };
        const decimals = 'bad 2 NaN 02 INF +2 1e2 2.0 -0 0 -.5 .5';
        const decimalDescending = '2 / 02 / +2 / 2.0 / .5 / -0 / 0 / -.5 / bad / NaN / INF / 1e2';
        const initialRows = [
            ['c', 'Cherry', 'A', '2'], ['d', 'Plum', 'A', '2'], ['b', 'Apple', 'A', '10'],
            ['a', 'Pear', 'B', '2'], ['e', 'Kiwi', 'A', 'bad'], ['f', 'Mango', 'B', '∅'],
        ];
        await step('1. Text and numeric keys', async () => {
            const input = words.querySelector('input[type="text"]') as HTMLInputElement;
            const [numeric, descending] = Array.from(words.querySelectorAll<HTMLInputElement>('input[type="checkbox"]'));
            const expectOutput = async (expected: string) => {
                await waitFor(() => expect(output()).toBe(expected));
                await whenCemSourceRendered(root);
                expect(output()).toBe(expected);
                expect(words.querySelector('[role="alert"]')).toBeNull();
            };
            const mode = async (number: boolean, reverse: boolean, expected: string) => {
                await toggle(words, 'label:nth-of-type(2) input', numeric, number);
                await toggle(words, 'label:nth-of-type(3) input', descending, reverse);
                await expectOutput(expected);
                expect([numeric.checked, descending.checked]).toEqual([number, reverse]);
            };
            const source = async (value: string, expected: string, caret = value.length) => {
                const state = [numeric.checked, descending.checked];
                edit(input, value, caret);
                await expectOutput(expected);
                expectEditor(words, 'input[type="text"]', input, value, caret);
                expect([numeric.checked, descending.checked]).toEqual(state);
            };
            expect([input.value, numeric.checked, descending.checked]).toEqual(['10 2 02 bad 1', false, false]);
            await mode(true, false, '1 / 2 / 02 / 10 / bad');
            await mode(true, true, '10 / 2 / 02 / 1 / bad');
            await mode(false, true, 'bad / 2 / 10 / 1 / 02');
            await mode(false, false, '02 / 1 / 10 / 2 / bad');
            await source(decimals, '+2 / -.5 / -0 / .5 / 0 / 02 / 1e2 / 2 / 2.0 / INF / NaN / bad', 4);
            await mode(false, true, 'bad / NaN / INF / 2.0 / 2 / 1e2 / 02 / 0 / .5 / -0 / -.5 / +2');
            await mode(true, true, decimalDescending);
            await mode(true, false, '-.5 / -0 / 0 / .5 / 2 / 02 / +2 / 2.0 / bad / NaN / INF / 1e2');
            const unicode = 'b\u00a0a a A 🍒 A';
            await source(unicode, 'b\u00a0a / a / A / 🍒 / A');
            await mode(true, true, 'b\u00a0a / a / A / 🍒 / A');
            await mode(false, true, '🍒 / b\u00a0a / a / A / A');
            await mode(false, false, 'A / A / a / b\u00a0a / 🍒');
            await source('', '');
            await mode(true, true, '');
            await source('   ', '');
            await source(decimals, decimalDescending);
            expect(rows()).toEqual(initialRows);
            expect(selection()).toEqual([]);
        });
        await step('2. Multiple keys and source selection', async () => {
            const xml = table.querySelector('textarea') as HTMLTextAreaElement;
            const descending = table.querySelector('input[type="checkbox"]') as HTMLInputElement;
            const expectRows = async (expected: string[][], selected: string, navigation: string[]) => {
                await waitFor(() => expect(rows()).toEqual(expected));
                await whenCemSourceRendered(root);
                expect(table.querySelector('caption')?.textContent).toBe('Group and quantity');
                expect(Array.from(table.querySelectorAll('thead th[scope="col"]'), node => node.textContent)).toEqual(['Row', 'Group', 'Qty']);
                expect(table.querySelectorAll('tbody th[scope="row"]')).toHaveLength(expected.length);
                expect(selection()).toEqual(navigation);
                for (const row of table.querySelectorAll('tbody tr')) {
                    const button = row.querySelector('button') as HTMLButtonElement;
                    expect(row.getAttribute('aria-selected')).toBe(String(button.value === selected));
                    expect(button.getAttribute('aria-pressed')).toBe(String(button.value === selected));
                }
                expect(table.querySelector('[role="alert"]')).toBeNull();
            };
            const select = async (id: string, expected: string[][], navigation: string[]) => {
                (table.querySelector(`button[value="${id}"]`) as HTMLButtonElement).click();
                await expectRows(expected, id, navigation);
            };
            const source = async (value: string, expected: string[][], selected: string, navigation: string[], caret = value.length) => {
                edit(xml, value, caret);
                await expectRows(expected, selected, navigation);
                expectEditor(table, 'textarea', xml, value, caret);
                expect(table.querySelector('input[type="checkbox"]')).toBe(descending);
            };
            await expectRows(initialRows, '', []);
            expect(descending.checked).toBe(false);
            await select('c', initialRows, ['Cherry', 'Apple']);
            await toggle(table, 'input[type="checkbox"]', descending, true);
            await expectRows([initialRows[2], initialRows[0], initialRows[1], ...initialRows.slice(3)], 'c', ['Cherry', 'Apple']);
            const mixed = '<r><row id="b" group="B" qty="1">Birch</row><row id="c" group="A" qty="2">Citrus</row><row id="d" group="A" qty="02">Plum</row><row id="e" group="Z" qty="">Empty</row><row id="f" group="A">Missing</row><row id="g" group="A" qty="1e2">Exponent</row><row id="h" group="A" qty="NaN">NaN</row><row id="i" group="A" qty="INF">Infinity</row><row id="a" group="A" qty="-1.5">Apple</row><row id="j" qty="0">Zero</row></r>';
            const invalidRows = [['e', 'Empty', 'Z', ''], ['f', 'Missing', 'A', '∅'], ['g', 'Exponent', 'A', '1e2'],
                ['h', 'NaN', 'A', 'NaN'], ['i', 'Infinity', 'A', 'INF']];
            const ascendingRows = [['j', 'Zero', '', '0'], ['a', 'Apple', 'A', '-1.5'], ['c', 'Citrus', 'A', '2'],
                ['d', 'Plum', 'A', '02'], ['b', 'Birch', 'B', '1'], ...invalidRows];
            const descendingRows = [ascendingRows[0], ascendingRows[2], ascendingRows[3], ascendingRows[1], ...ascendingRows.slice(4)];
            await source(mixed, descendingRows, 'c', ['Citrus', 'Birch'], 3);
            expect(descending.checked).toBe(true);
            await toggle(table, 'input[type="checkbox"]', descending, false);
            await expectRows(ascendingRows, 'c', ['Citrus', 'Birch']);
            await toggle(table, 'input[type="checkbox"]', descending, true);
            await expectRows(descendingRows, 'c', ['Citrus', 'Birch']);
            await select('j', descendingRows, ['Zero', 'Apple']);
            await select('e', descendingRows, ['Empty', 'Plum']);
            await select('f', descendingRows, ['Missing', 'Empty']);
            await select('b', descendingRows, ['Birch', '']);
            await source('<r><row id="z" group="A" qty="1">Zed</row></r>', [['z', 'Zed', 'A', '1']], '', []);
            const recover = () => source('<r><row id="b" group="A" qty="1">Recovered</row></r>',
                [['b', 'Recovered', 'A', '1']], 'b', ['Recovered', '']);
            for (const empty of ['<r/>', '<r xmlns="urn:other"><row id="b" qty="1">Qualified</row></r>',
                '<other><row id="b">Other</row></other>']) {
                await source(empty, [], '', []);
                await recover();
            }
            for (const invalid of ['<r>', '<r><row></r>']) {
                edit(xml, invalid);
                await waitFor(() => expect(table.querySelector('[role="alert"]')?.textContent).toContain('XML'));
                expect(table.querySelector('table')).toBeNull();
                expect(selection()).toEqual([]);
                expectEditor(table, 'textarea', xml, invalid);
                await recover();
            }
            await source(mixed, descendingRows, 'b', ['Birch', '']);
            expect(descending.checked).toBe(true);
            expect(output()).toBe(decimalDescending);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-sort.cemt', './xpath-nodes.html', './xpath-sequences.html',
                './data-table.html', './xpath-validation.html'].map(relative => new URL(relative, DEMO_URL).href));
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
