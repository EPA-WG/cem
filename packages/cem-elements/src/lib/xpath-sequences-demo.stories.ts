import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
const SOURCE_TAG = 'story-xpath-sequences-document';
const DEMO_URL = new URL('../../demo/xpath-sequences.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Sequences', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const WindowsAndColumns: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('sequence demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        await waitFor(() => expect(sample('1. A window into a word sequence').querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const words = sample('1. A window into a word sequence');
        const table = sample('2. First-seen XML columns');
        const outputs = () => Array.from(words.querySelectorAll('output'), item => item.textContent);
        const headings = () => Array.from(table.querySelectorAll('th[scope="col"]'), item => item.textContent);
        const rows = () => Array.from(table.querySelectorAll('tbody tr'), row => Array.from(row.querySelectorAll('td'), cell => cell.textContent));
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
        const expectOutputs = async (expected: readonly string[]) => {
            await waitFor(() => expect(outputs()).toEqual(expected));
            await whenCemSourceRendered(root);
            expect(outputs()).toEqual(expected);
            expect(words.querySelector('[role="alert"]')).toBeNull();
        };
        const initialHeadings = ['@id', 'fruit', '@qty', 'note'];
        const initialRows = [['2', 'Apple', '∅', '∅'], ['1', 'Cherry', '3', '""']];
        await step('1. A window into a word sequence', async () => {
            const input = words.querySelector('textarea') as HTMLTextAreaElement;
            const bounds = Array.from(words.querySelectorAll<HTMLInputElement>('input[type="text"]'));
            const reverse = words.querySelector('input[type="checkbox"]') as HTMLInputElement;
            expect([input.value, ...bounds.map(control => control.value), reverse.checked])
                .toEqual(['cherry apple cherry pear', '2', '2', false]);
            await expectOutputs(['3', 'apple | cherry', 'apple', 'cherry']);
            const toggle = async (checked: boolean, expected: string[]) => {
                reverse.focus();
                reverse.click();
                await expectOutputs(expected);
                expect(words.querySelector('input[type="checkbox"]')).toBe(reverse);
                expect(reverse.checked).toBe(checked);
                expect(document.activeElement).toBe(reverse);
            };
            await toggle(true, ['3', 'cherry | apple', 'cherry', 'apple']);
            edit(input, '🍒 a a b', 3);
            await expectOutputs(['3', 'a | a', 'a', 'a']);
            expectEditor(words, 'textarea', input, '🍒 a a b', 3);
            expect(reverse.checked).toBe(true);
            await toggle(false, ['3', 'a | a', 'a', 'a']);
            edit(input, 'a b c d e');
            await expectOutputs(['5', 'b | c', 'b', 'c']);
            const setBounds = async (start: string, length: string, expected: readonly string[]) => {
                edit(bounds[0], start);
                await whenCemSourceRendered(root);
                expectEditor(words, 'label:nth-child(1) input[type="text"]', bounds[0], start);
                edit(bounds[1], length);
                await expectOutputs(expected);
                expectEditor(words, 'label:nth-child(2) input[type="text"]', bounds[1], length);
                expect(bounds[0].value).toBe(start);
                expect(reverse.checked).toBe(false);
            };
            for (const [start, length, expected] of [
                ['2.5', '1.5', ['5', 'c | d', 'c', 'd']],
                ['0', '3', ['5', 'a | b', 'a', 'b']],
                ['-1', '3', ['5', 'a', 'a', '']],
                ['-1.5', '3', ['5', 'a', 'a', '']],
                ['-0.5', '2', ['5', 'a', 'a', '']],
                ['1', '0', ['5', '', '', '']],
                ['1', '-2', ['5', '', '', '']],
                ['6', '2', ['5', '', '', '']],
                ['NaN', '2', ['5', '', '', '']],
                ['1', 'INF', ['5', 'a | b | c | d | e', 'a', 'b | c | d | e']],
                ['-INF', 'INF', ['5', '', '', '']],
            ] as const) await setBounds(start, length, expected);
            await setBounds('1', '2', ['5', 'a | b', 'a', 'b']);
            for (const [index, control] of bounds.entries()) {
                const selector = `label:nth-child(${index + 1}) input[type="text"]`;
                for (const invalid of ['oops', '']) {
                    edit(control, invalid);
                    await waitFor(() => expect(words.querySelector('[role="alert"]')?.textContent)
                        .toBe('Enter numeric start and length values.'));
                    expect(outputs()).toEqual(['5']);
                    expectEditor(words, selector, control, invalid);
                    edit(control, index === 0 ? '1' : '2');
                    await expectOutputs(['5', 'a | b', 'a', 'b']);
                }
            }
            await setBounds('1', '10', ['5', 'a | b | c | d | e', 'a', 'b | c | d | e']);
            for (const [value, expected] of [
                ['', ['0', '', '', '']],
                [' \t\n', ['0', '', '', '']],
                ['é e\u0301 É é', ['3', 'é | e\u0301 | É | é', 'é', 'e\u0301 | É | é']],
                ['a\u00a0b a\u00a0b', ['1', 'a\u00a0b | a\u00a0b', 'a\u00a0b', 'a\u00a0b']],
                ['\tapple\ncherry\tapple\n', ['2', 'apple | cherry | apple', 'apple', 'cherry | apple']],
            ] as const) {
                edit(input, value);
                await expectOutputs(expected);
                expectEditor(words, 'textarea', input, value);
                expect(reverse.checked).toBe(false);
            }
            expect(headings()).toEqual(initialHeadings);
            expect(rows()).toEqual(initialRows);
        });
        await step('2. First-seen XML columns', async () => {
            const xml = table.querySelector('textarea') as HTMLTextAreaElement;
            const expectTable = async (expectedHeadings: readonly string[], expectedRows: readonly (readonly string[])[]) => {
                await waitFor(() => {
                    expect(headings()).toEqual(expectedHeadings);
                    expect(rows()).toEqual(expectedRows);
                });
                await whenCemSourceRendered(root);
                expect(table.querySelector('caption')?.textContent).toBe('Columns in first-seen order');
                expect(table.querySelectorAll('th')).toHaveLength(expectedHeadings.length);
                expect(table.querySelector('[role="alert"]')).toBeNull();
            };
            await expectTable(initialHeadings, initialRows);
            const source = async (value: string, columns: readonly string[], cells: readonly (readonly string[])[], caret = value.length) => {
                edit(xml, value, caret);
                await expectTable(columns, cells);
                expectEditor(table, 'textarea', xml, value, caret);
            };
            await source('<r xmlns:x="urn:x"><row id="1"><fruit>A</fruit><x:fruit>B</x:fruit><fruit>C</fruit></row><row extra="new"><fruit>D</fruit></row></r>',
                ['@id', 'fruit', 'fruit [urn:x]', '@extra'], [['1', 'A / C', 'B', '∅'], ['∅', 'D', '∅', 'new']], 12);
            const kinds = '<r xmlns:x="urn:x" xmlns:y="urn:x"><row fruit="attr" x:fruit="ns"><fruit/><y:fruit>B</y:fruit></row><row><fruit>C</fruit><fruit>D</fruit><x:fruit/></row></r>';
            const kindHeadings = ['@fruit', '@fruit [urn:x]', 'fruit', 'fruit [urn:x]'];
            const kindRows = [['attr', 'ns', '""', 'B'], ['∅', '∅', 'C / D', '""']];
            await source(kinds, kindHeadings, kindRows);
            await source('<r><row><fruit> A<b>B</b><![CDATA[C]]><!--skip--><?skip it?> D </fruit><fruit/></row><row flag=""><fruit/><fruit/></row><row/></r>',
                ['fruit', '@flag'], [[' ABC D  / ', '∅'], [' / ', '""'], ['∅', '∅']]);
            await source('<r><row b="2" a="1"><z>Z</z><a>A</a></row><row c="3"><z/></row></r>',
                ['@b', '@a', 'z', 'a', '@c'], [['2', '1', 'Z', 'A', '∅'], ['∅', '∅', '""', '∅', '3']]);
            await source('<r><row/></r>', [], [[]]);
            await source('<r/>', [], []);
            const recover = () => source('<r><row><fruit>Recovered</fruit></row></r>', ['fruit'], [['Recovered']]);
            await recover();
            for (const invalid of ['<r>', '<r><row></r>']) {
                edit(xml, invalid);
                await waitFor(() => expect(table.querySelector('[role="alert"]')?.textContent).toContain('XML'));
                expect(table.querySelector('table')).toBeNull();
                expectEditor(table, 'textarea', xml, invalid);
                await recover();
            }
            await source(kinds, kindHeadings, kindRows);
            expect(outputs()).toEqual(['2', 'apple | cherry | apple', 'apple', 'cherry | apple']);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-sequences.cemt', './xpath-sort.html', './xpath-maps-arrays.html',
                './xpath-aggregates.html', './xpath-nodes.html', './functions/str.html', './data-table.html']
                .map(relative => new URL(relative, DEMO_URL).href));
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
