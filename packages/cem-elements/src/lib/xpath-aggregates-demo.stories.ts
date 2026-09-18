import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
const SOURCE_TAG = 'story-xpath-aggregates-document';
const DEMO_URL = new URL('../../demo/xpath-aggregates.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Aggregates', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const DecimalStatisticsAndBasket: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement }) => {
        const root = canvasElement.querySelector(SOURCE_TAG);
        if (!root) throw new Error('aggregate demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        await waitFor(() => expect(sample('1. Decimal sequence statistics').querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        const numbers = sample('1. Decimal sequence statistics');
        const values = (article: Element) => Array.from(article.querySelectorAll('output'), item => item.textContent);
        const input = numbers.querySelector('textarea') as HTMLTextAreaElement;
        const edit = (control: HTMLTextAreaElement, value: string) => {
            control.value = value;
            control.dispatchEvent(new Event('input', { bubbles: true }));
        };
        expect(values(numbers)).toEqual(['0.3', '0.1', '0.2', '0.15']);
        input.focus();
        input.value = '-2\t1\n4';
        input.setSelectionRange(2, 2);
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(values(numbers)).toEqual(['3', '-2', '4', '1']));
        expect(numbers.querySelector('textarea')).toBe(input);
        expect(document.activeElement).toBe(input);
        expect(input.selectionStart).toBe(2);
        edit(input, '0 0 1');
        await waitFor(() => expect(values(numbers)[3]).toBe('0.333333333333333333'));
        for (const invalid of ['1 bad', 'NaN', '1e2', '1\u00a02']) {
            edit(input, invalid);
            await waitFor(() => expect(numbers.querySelector('[role="alert"]')?.textContent).toContain('Enter decimal'));
            expect(values(numbers)).toEqual([]);
        }
        edit(input, ' \t\n');
        await waitFor(() => expect(values(numbers)).toEqual(['0', '∅', '∅', '∅']));
        const basket = sample('2. A basket that accepts new fruits');
        await waitFor(() => expect(values(basket)).toEqual(['3.75', '1.25', '2.5', '1.875']));
        const xml = basket.querySelector('textarea') as HTMLTextAreaElement;
        xml.focus();
        xml.value = '<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>';
        xml.setSelectionRange(8, 8);
        xml.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(values(basket)).toEqual(['0.9', '0.1', '0.6', '0.3']));
        expect(Array.from(basket.querySelectorAll('tbody th'), item => item.textContent)).toEqual(['apple', 'cherry', 'pear']);
        expect(basket.querySelector('textarea')).toBe(xml);
        expect(document.activeElement).toBe(xml);
        expect(xml.selectionStart).toBe(8);
        for (const invalid of ['<basket><pear>bad</pear></basket>', '<basket><pear/></basket>', '<basket><pear>-1</pear></basket>', '<basket>']) {
            edit(xml, invalid);
            await waitFor(() => expect(basket.querySelector('[role="alert"]')?.textContent).toBeTruthy());
            expect(values(basket)).toEqual([]);
            expect(basket.querySelector('table')).toBeNull();
        }
        edit(xml, '<basket/>');
        await waitFor(() => expect(values(basket)).toEqual(['0', '∅', '∅', '∅']));
        expect(basket.querySelectorAll('tbody tr')).toHaveLength(0);
        edit(xml, '<basket><plum>7</plum></basket>');
        await waitFor(() => expect(values(basket)).toEqual(['7', '7', '7', '7']));
        expect(values(numbers)).toEqual(['0', '∅', '∅', '∅']);
    },
};
