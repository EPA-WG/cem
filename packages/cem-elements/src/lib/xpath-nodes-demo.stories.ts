import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
const SOURCE_TAG = 'story-xpath-nodes-document';
const DEMO_URL = new URL('../../demo/xpath-nodes.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath XML Nodes', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const TablesAndTrees: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('source-loaded node demo is missing');
        const sample = (legend: string) => {
            const item = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!item) throw new Error(`Missing ${legend}`);
            return item;
        };
        const normalized = (node: Element) => node.textContent?.replace(/\s+/gu, ' ').trim();
        const edit = (control: HTMLTextAreaElement, value: string, caret = value.length) => {
            control.focus();
            control.value = value;
            control.setSelectionRange(caret, caret);
            control.dispatchEvent(new Event('input', { bubbles: true }));
        };
        const expectEditor = (article: Element, control: HTMLTextAreaElement, value: string, caret = value.length) => {
            expect(article.querySelector('textarea')).toBe(control);
            expect(control.value).toBe(value);
            expect(document.activeElement).toBe(control);
            expect([control.selectionStart, control.selectionEnd]).toEqual([caret, caret]);
        };
        await waitFor(() => expect(sample('1. XML table with native navigation').querySelectorAll('tbody tr')).toHaveLength(2), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const table = sample('1. XML table with native navigation');
        const tree = sample('2. XML tree with attributes and mixed text');
        const rows = () => Array.from(table.querySelectorAll('tbody tr'), row => Array.from(row.querySelectorAll('td'), normalized));
        const outputs = () => Array.from(table.querySelectorAll('output'), node => node.textContent);
        const codes = () => Array.from(tree.querySelectorAll('code'), node => node.textContent);
        const initialRows = [['1', 'item urn:b', 'Apple', '2'], ['2', 'item urn:a', 'Zest', '10']];
        const mixedXml = '<crate xmlns:p="urn:fruit"><p:item id="a" qty="02">Pear</p:item>gap<!--skip--><?skip it?><item id="b"> A<b>B</b><![CDATA[C]]> </item><item id="c" qty="3">Pear</item></crate>';
        const mixedRows = [['b', 'item', 'ABC', ''], ['a', 'item urn:fruit', 'Pear', '02'], ['c', 'item', 'Pear', '3']];
        const expectTable = async (expected: string[][], selected: string, navigation: string[]) => {
            await waitFor(() => expect(rows()).toEqual(expected));
            await whenCemSourceRendered(root);
            expect(table.querySelector('caption')?.textContent).toBe('Basket nodes');
            expect(Array.from(table.querySelectorAll('th[scope="col"]'), node => node.textContent)).toEqual(['Select', 'Node', 'Value', 'Qty']);
            expect(outputs()).toEqual(navigation);
            for (const row of table.querySelectorAll('tbody tr')) {
                const button = row.querySelector('button') as HTMLButtonElement;
                expect(button.getAttribute('aria-label')).toBe(`Select row ${button.value}`);
                expect(row.getAttribute('aria-selected')).toBe(String(button.value === selected));
                expect(button.getAttribute('aria-pressed')).toBe(String(button.value === selected));
            }
            expect(table.querySelector('[role="alert"]')).toBeNull();
        };
        await step('1. XML table with native navigation', async () => {
            const source = table.querySelector('textarea') as HTMLTextAreaElement;
            const order = table.querySelector('select') as HTMLSelectElement;
            await expectTable(initialRows, '', []);
            expect(order.value).toBe('ascending');
            (table.querySelector('button[value="1"]') as HTMLButtonElement).click();
            await expectTable(initialRows, '1', ['basket', 'Zest']);
            const setOrder = async (value: string, expected: string[][], selected: string, navigation: string[]) => {
                order.focus();
                order.value = value;
                order.dispatchEvent(new Event('change', { bubbles: true }));
                await expectTable(expected, selected, navigation);
                expect(table.querySelector('select')).toBe(order);
                expect(order.value).toBe(value);
                expect(document.activeElement).toBe(order);
            };
            await setOrder('descending', [...initialRows].reverse(), '1', ['basket', 'Zest']);
            (table.querySelector('button[value="2"]') as HTMLButtonElement).click();
            await expectTable([...initialRows].reverse(), '2', ['basket', '']);
            const sourceEdit = async (value: string, expected: string[][], selected: string, navigation: string[], caret = value.length) => {
                edit(source, value, caret);
                await expectTable(expected, selected, navigation);
                expectEditor(table, source, value, caret);
                expect(table.querySelector('select')).toBe(order);
            };
            const descending = [mixedRows[1], mixedRows[2], mixedRows[0]];
            await sourceEdit(mixedXml, descending, '', [], 8);
            expect(order.value).toBe('descending');
            (table.querySelector('button[value="c"]') as HTMLButtonElement).click();
            await expectTable(descending, 'c', ['crate', ' ABC ']);
            await setOrder('ascending', mixedRows, 'c', ['crate', ' ABC ']);
            expect(table.querySelector('tbody tr:first-child td:nth-child(3)')?.textContent).toBe(' ABC ');
            (table.querySelector('button[value="b"]') as HTMLButtonElement).click();
            await expectTable(mixedRows, 'b', ['crate', 'Pear']);
            (table.querySelector('button[value="c"]') as HTMLButtonElement).click();
            await expectTable(mixedRows, 'c', ['crate', ' ABC ']);
            await sourceEdit('<crate/>', [], '', []);
            const recovered = '<crate><item id="c" qty="4">Recovered</item></crate>';
            const recover = () => sourceEdit(recovered, [['c', 'item', 'Recovered', '4']], 'c', ['crate', '']);
            await recover();
            for (const invalid of ['<crate>', '<crate><item></crate>']) {
                edit(source, invalid);
                await waitFor(() => expect(table.querySelector('[role="alert"]')?.textContent).toContain('XML'));
                expect(table.querySelector('table')).toBeNull();
                expect(outputs()).toEqual([]);
                expectEditor(table, source, invalid);
                await recover();
            }
            await sourceEdit(mixedXml, mixedRows, 'c', ['crate', ' ABC ']);
            expect(codes()).toEqual(['bright', 'Hello & welcome', 'friend', '!']);
        });
        await step('2. XML tree with attributes and mixed text', async () => {
            const source = tree.querySelector('textarea') as HTMLTextAreaElement;
            const summaries = () => Array.from(tree.querySelectorAll('summary'), summary => [
                summary.querySelector('strong')?.textContent, summary.querySelector('small')?.textContent,
            ]);
            expect(summaries()).toEqual([['note', '[urn:notes]'], ['em', '[urn:marks]']]);
            expect(codes()).toEqual(['bright', 'Hello & welcome', 'friend', '!']);
            const [initialRoot, child] = Array.from(tree.querySelectorAll('details'));
            expect([initialRoot.open, child.open]).toEqual([true, true]);
            for (const disclosure of [child, initialRoot]) {
                (disclosure.querySelector('summary') as HTMLElement).click();
                expect(disclosure.open).toBe(false);
                expect(disclosure.isConnected).toBe(true);
                (disclosure.querySelector('summary') as HTMLElement).click();
                expect(disclosure.open).toBe(true);
            }
            const mixed = '<r xmlns:p="urn:new" p:mood="calm">Hi<![CDATA[ there]]><!--remark--><?say yes?><p:em>🍒</p:em>!</r>';
            const updated = mixed.replace('Hi<![CDATA[ there]]>', 'Updated<![CDATA[ text]]>');
            const expectedCodes = ['calm', 'Updated text', 'remark', 'yes', '🍒', '!'];
            const sourceEdit = async (value: string, expected: string[], caret = value.length) => {
                edit(source, value, caret);
                await waitFor(() => expect(codes()).toEqual(expected));
                await whenCemSourceRendered(root);
                expect(tree.querySelector('[role="alert"]')).toBeNull();
                expectEditor(tree, source, value, caret);
            };
            await sourceEdit(mixed, ['calm', 'Hi there', 'remark', 'yes', '🍒', '!'], 10);
            expect(summaries()).toEqual([['r', '[]'], ['em', '[urn:new]']]);
            expect(Array.from(tree.querySelectorAll('li:not(:has(details))'), normalized)).toEqual([
                '@mood [urn:new] = calm', 'text: Hi there', 'comment: remark', 'processing instruction: yes', 'text: 🍒', 'text: !',
            ]);
            const liveRoot = tree.querySelector('details') as HTMLDetailsElement;
            (liveRoot.querySelector('summary') as HTMLElement).click();
            expect(liveRoot.open).toBe(false);
            await sourceEdit(updated, expectedCodes);
            expect(tree.querySelector('details')).toBe(liveRoot);
            expect(liveRoot.open).toBe(false);
            (liveRoot.querySelector('summary') as HTMLElement).click();
            expect(liveRoot.open).toBe(true);
            await sourceEdit('<r/>', []);
            expect(summaries()).toEqual([['r', '[]']]);
            await sourceEdit('<r>Recovered</r>', ['Recovered']);
            for (const invalid of ['<broken>', '<r><em></r>']) {
                edit(source, invalid);
                await waitFor(() => expect(tree.querySelector('[role="alert"]')?.textContent).toContain('XML'));
                expect(tree.querySelector('ul')).toBeNull();
                expect(codes()).toEqual([]);
                expectEditor(tree, source, invalid);
                await sourceEdit('<r>Recovered</r>', ['Recovered']);
            }
            await sourceEdit(updated, expectedCodes);
            await expectTable(mixedRows, 'c', ['crate', ' ABC ']);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-nodes.cemt', './xpath-sort.html', './xpath-aggregates.html',
                './xpath-sequences.html', './xpath-functions.html', './data-table.html'].map(relative => new URL(relative, DEMO_URL).href));
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
