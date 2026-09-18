import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
const SOURCE_TAG = 'story-xpath-nodes-document';
const DEMO_URL = new URL('../../demo/xpath-nodes.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath XML Nodes', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const TablesAndTrees: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('source-loaded node demo is missing');
        const sample = (legend: string) => {
            const item = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!item) throw new Error(`Missing ${legend}`);
            return item;
        };
        await waitFor(() => expect(sample('1. XML table with native navigation').querySelectorAll('tbody tr')).toHaveLength(2), { timeout: 20000 });
        const table = sample('1. XML table with native navigation');
        const rows = () => Array.from(table.querySelectorAll('tbody tr'), row =>
            Array.from(row.querySelectorAll('td'), cell => cell.textContent?.replace(/\s+/gu, ' ').trim()));
        expect(rows()).toEqual([['1', 'item urn:b', 'Apple', '2'], ['2', 'item urn:a', 'Zest', '10']]);
        (table.querySelector('button[aria-label="Select row 1"]') as HTMLButtonElement).click();
        await waitFor(() => expect(table.querySelectorAll('output')[1]?.textContent).toBe('Zest'));
        expect(table.querySelectorAll('output')[0]?.textContent).toBe('basket');
        const order = table.querySelector('select') as HTMLSelectElement;
        order.value = 'descending';
        order.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => expect(rows()[0][2]).toBe('Zest'));
        expect(table.querySelector('tr[aria-selected="true"]')?.textContent).toContain('Apple');
        expect(table.querySelectorAll('output')[1]?.textContent).toBe('Zest');
        const tree = sample('2. XML tree with attributes and mixed text');
        await waitFor(() => expect(tree.querySelectorAll('summary')).toHaveLength(2));
        const codes = () => Array.from(tree.querySelectorAll('code'), node => node.textContent);
        expect(codes()).toEqual(['bright', 'Hello & welcome', 'friend', '!']);
        expect(tree.querySelector('summary')?.textContent).toContain('urn:notes');
        const source = tree.querySelector('textarea') as HTMLTextAreaElement;
        source.focus();
        source.value = '<r xmlns:p="urn:new" p:mood="calm">Hi<![CDATA[ there]]><p:em>🍒</p:em></r>';
        source.setSelectionRange(10, 10);
        source.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(codes()).toEqual(['calm', 'Hi there', '🍒']));
        expect(tree.querySelector('textarea')).toBe(source);
        expect(document.activeElement).toBe(source);
        expect(source.selectionStart).toBe(10);
        expect(tree.querySelector('li li')?.textContent).toContain('urn:new');
        source.value = '<broken>';
        source.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(tree.querySelector('[role="alert"]')?.textContent).toBeTruthy());
        expect(tree.querySelector('summary')).toBeNull();
        source.value = '<r>Recovered</r>';
        source.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(codes()).toEqual(['Recovered']));
        expect(tree.querySelector('[role="alert"]')).toBeNull();
        expect(rows()[0][2]).toBe('Zest');
    },
};
