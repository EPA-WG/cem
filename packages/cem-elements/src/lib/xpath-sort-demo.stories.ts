import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { whenCemRendered } from '../../.storybook/preview.js';
const SOURCE_TAG = 'story-xpath-sort-document';
const DEMO_URL = new URL('../../demo/xpath-sort.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Sorting', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const TypedKeysAndSourceSelection: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('sort demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        const settle = async (article: Element) => {
            if (!article.parentElement) throw new Error('sort sample instance is missing');
            await whenCemRendered(article.parentElement);
        };
        const toggle = (input: HTMLInputElement, checked: boolean) => {
            input.checked = checked;
            input.dispatchEvent(new Event('change', { bubbles: true }));
        };
        await waitFor(() => expect(sample('1. Text and numeric keys').querySelector('output')?.textContent).toBe('02 / 1 / 10 / 2 / bad'), { timeout: 20000 });
        const words = sample('1. Text and numeric keys');
        const [numeric, descending] = Array.from(words.querySelectorAll<HTMLInputElement>('input[type=checkbox]'));
        const output = () => words.querySelector('output')?.textContent;
        toggle(numeric, true);
        await settle(words);
        await waitFor(() => expect(output()).toBe('1 / 2 / 02 / 10 / bad'));
        toggle(descending, true);
        await settle(words);
        await waitFor(() => expect(output()).toBe('10 / 2 / 02 / 1 / bad'));
        const input = words.querySelector('input[type=text]') as HTMLInputElement;
        input.focus();
        input.value = '3 nope 03 bad -1';
        input.setSelectionRange(2, 2);
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await settle(words);
        await waitFor(() => expect(output()).toBe('3 / 03 / -1 / nope / bad'));
        expect(document.activeElement).toBe(input);
        expect(input.selectionStart).toBe(2);
        toggle(numeric, false);
        await settle(words);
        await waitFor(() => expect(output()).toBe('nope / bad / 3 / 03 / -1'));
        input.value = '';
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await settle(words);
        await waitFor(() => expect(output()).toBe(''));
        const rows = sample('2. Multiple keys and source selection');
        const labels = () => Array.from(rows.querySelectorAll('tbody button'), button => button.textContent);
        await waitFor(() => expect(labels()).toEqual(['Cherry', 'Plum', 'Apple', 'Pear', 'Kiwi', 'Mango']));
        (Array.from(rows.querySelectorAll('tbody button')).find(button => button.textContent === 'Cherry') as HTMLButtonElement).click();
        await settle(rows);
        const selection = () => Array.from(rows.querySelectorAll('output'), node => node.textContent);
        await waitFor(() => expect(selection()).toEqual(['Cherry', 'Apple']));
        toggle(rows.querySelector('input[type=checkbox]') as HTMLInputElement, true);
        await settle(rows);
        await waitFor(() => expect(labels()).toEqual(['Apple', 'Cherry', 'Plum', 'Pear', 'Kiwi', 'Mango']));
        expect(selection()).toEqual(['Cherry', 'Apple']);
        expect(rows.querySelector('button[aria-pressed=true]')?.textContent).toBe('Cherry');
        const xml = rows.querySelector('textarea') as HTMLTextAreaElement;
        xml.focus();
        xml.value = '<r><row id="x" group="A" qty="0">Zero</row><row id="c" group="A" qty="-1">Changed</row></r>';
        xml.setSelectionRange(3, 3);
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await settle(rows);
        await waitFor(() => expect(labels()).toEqual(['Zero', 'Changed']));
        expect(selection()).toEqual(['Changed', 'Zero']);
        expect(document.activeElement).toBe(xml);
        expect(xml.selectionStart).toBe(3);
        xml.value = '<r>';
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await settle(rows);
        await waitFor(() => expect(rows.querySelector('[role=alert]')?.textContent).toBeTruthy());
        expect(rows.querySelector('table')).toBeNull();
        xml.value = '<r/>';
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await settle(rows);
        await waitFor(() => expect(rows.querySelector('table')).not.toBeNull());
        expect(labels()).toEqual([]);
        expect(selection()).toEqual([]);
        expect(output()).toBe('');
    },
};
