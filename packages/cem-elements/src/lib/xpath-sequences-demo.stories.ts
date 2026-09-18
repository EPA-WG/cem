import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
const SOURCE_TAG = 'story-xpath-sequences-document';
const DEMO_URL = new URL('../../demo/xpath-sequences.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Sequences', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const WindowsAndColumns: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('sequence demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        await waitFor(() => expect(sample('1. A window into a word sequence').querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        const windowSample = sample('1. A window into a word sequence');
        const outputs = () => Array.from(windowSample.querySelectorAll('output'), item => item.textContent);
        expect(outputs()).toEqual(['3', 'apple | cherry', 'apple', 'cherry']);
        const reverse = windowSample.querySelector('input[type="checkbox"]') as HTMLInputElement;
        expect(reverse.checked).toBe(false);
        reverse.click();
        await waitFor(() => expect(outputs()).toEqual(['3', 'cherry | apple', 'cherry', 'apple']));
        expect(reverse.checked).toBe(true);
        const input = windowSample.querySelector('textarea') as HTMLTextAreaElement;
        const bounds = windowSample.querySelectorAll<HTMLInputElement>('input[type="text"]');
        const edit = (control: HTMLInputElement | HTMLTextAreaElement, value: string) => {
            control.value = value;
            control.dispatchEvent(new Event('input', { bubbles: true }));
        };
        input.focus();
        input.value = '🍒 a a b';
        input.setSelectionRange(3, 3);
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(outputs()).toEqual(['3', 'a | a', 'a', 'a']));
        expect(windowSample.querySelector('textarea')).toBe(input);
        expect(document.activeElement).toBe(input);
        expect(input.selectionStart).toBe(3);
        edit(bounds[0], '2.5');
        edit(bounds[1], '1.5');
        await waitFor(() => expect(outputs()).toEqual(['3', 'b | a', 'b', 'a']));
        edit(bounds[0], 'oops');
        await waitFor(() => expect(windowSample.querySelector('[role="alert"]')?.textContent).toContain('Enter numeric'));
        edit(bounds[0], '1');
        edit(input, '');
        await waitFor(() => expect(outputs()).toEqual(['0', '', '', '']));
        edit(input, 'a\u00a0b a\u00a0b');
        await waitFor(() => expect(outputs()[0]).toBe('1'));
        const table = sample('2. First-seen XML columns');
        const headings = () => Array.from(table.querySelectorAll('th'), item => item.textContent);
        const cells = () => Array.from(table.querySelectorAll('td'), item => item.textContent);
        await waitFor(() => expect(headings()).toEqual(['@id', 'fruit', '@qty', 'note']));
        expect(cells()).toEqual(['2', 'Apple', '∅', '∅', '1', 'Cherry', '3', '""']);
        const xml = table.querySelector('textarea') as HTMLTextAreaElement;
        xml.focus();
        xml.value = '<r xmlns:x="urn:x"><row id="1"><fruit>A</fruit><x:fruit>B</x:fruit><fruit>C</fruit></row><row extra="new"><fruit>D</fruit></row></r>';
        xml.setSelectionRange(12, 12);
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(headings()).toEqual(['@id', 'fruit', 'fruit [urn:x]', '@extra']));
        expect(cells()).toEqual(['1', 'A / C', 'B', '∅', '∅', 'D', '∅', 'new']);
        expect(table.querySelector('textarea')).toBe(xml);
        expect(document.activeElement).toBe(xml);
        expect(xml.selectionStart).toBe(12);
        edit(xml, '<r>');
        await waitFor(() => expect(table.querySelector('[role="alert"]')?.textContent).toBeTruthy());
        expect(table.querySelector('table')).toBeNull();
        edit(xml, '<r/>');
        await waitFor(() => expect(table.querySelector('table')).not.toBeNull());
        expect(headings()).toEqual([]);
        expect(cells()).toEqual([]);
        expect(outputs()[0]).toBe('1');
    },
};
