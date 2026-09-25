import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-xpath-aggregates-document';
const DEMO_URL = new URL('../../demo/xpath-aggregates.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/XPath Aggregates', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const DecimalStatisticsAndBasket: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const root = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!root) throw new Error('aggregate demo document is missing');
        const sample = (legend: string) => {
            const article = root.querySelector(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing ${legend}`);
            return article;
        };
        await waitFor(() => expect(sample('1. Decimal sequence statistics').querySelectorAll('output')).toHaveLength(4), { timeout: 20000 });
        await whenCemSourceRendered(root);
        const numbers = sample('1. Decimal sequence statistics');
        const basket = sample('2. A basket that accepts new fruits');
        const values = (article: Element) => Array.from(article.querySelectorAll('output'), item => item.textContent);
        const rows = () => Array.from(basket.querySelectorAll('tbody tr'), row =>
            Array.from(row.children, cell => cell.textContent?.trim()));
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
        await step('1. Decimal sequence statistics', async () => {
            const input = numbers.querySelector('textarea') as HTMLTextAreaElement;
            expect(input.value).toBe('0.1 0.2');
            expect(values(numbers)).toEqual(['0.3', '0.1', '0.2', '0.15']);
            edit(input, '-2\t1\n4', 2);
            await waitFor(() => expect(values(numbers)).toEqual(['3', '-2', '4', '1']));
            expectEditor(numbers, input, '-2\t1\n4', 2);
            for (const [source, expected] of [
                ['+001.20 -.20 0', ['1', '-0.2', '1.2', '0.333333333333333333']],
                ['0 0 1', ['1', '0', '1', '0.333333333333333333']],
                ['0 1 1', ['2', '0', '1', '0.666666666666666667']],
                ['0 -1 -1', ['-2', '-1', '0', '-0.666666666666666667']],
                ['-0 .5 5.', ['5.5', '0', '5', '1.83333333333333333']],
            ] as const) {
                edit(input, source);
                await waitFor(() => expect(values(numbers)).toEqual(expected));
                expectEditor(numbers, input, source);
            }
            for (const invalid of ['1 bad', 'NaN', '1e2', '1\u00a02']) {
                edit(input, invalid);
                await waitFor(() => expect(numbers.querySelector('[role="alert"]')?.textContent).toContain('Enter decimal'));
                expect(values(numbers)).toEqual([]);
                expectEditor(numbers, input, invalid);
                // Recover between invalid cases so a stale alert cannot pass.
                edit(input, '0.1 0.2');
                await waitFor(() => expect(values(numbers)).toEqual(['0.3', '0.1', '0.2', '0.15']));
                expect(numbers.querySelector('[role="alert"]')).toBeNull();
            }
            for (const empty of ['', ' \t\n']) {
                edit(input, empty);
                await waitFor(() => expect(values(numbers)).toEqual(['0', '∅', '∅', '∅']));
                await whenCemSourceRendered(root);
                expectEditor(numbers, input, empty);
            }
            expect(values(basket)).toEqual(['3.75', '1.25', '2.5', '1.875']);
        });
        await step('2. A basket that accepts new fruits', async () => {
            expect(values(basket)).toEqual(['3.75', '1.25', '2.5', '1.875']);
            expect(basket.querySelector('caption')?.textContent).toBe('Fruit amounts');
            expect(Array.from(basket.querySelectorAll('thead th[scope="col"]'), cell => cell.textContent)).toEqual(['Fruit', 'Amount']);
            expect(rows()).toEqual([['apple', '1.25'], ['cherry', '2.5']]);
            const xml = basket.querySelector('textarea') as HTMLTextAreaElement;
            const added = '<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>';
            edit(xml, added, 8);
            await waitFor(() => expect(values(basket)).toEqual(['0.9', '0.1', '0.6', '0.3']));
            expect(rows()).toEqual([['apple', '0.1'], ['cherry', '0.2'], ['pear', '0.6']]);
            expect(basket.querySelectorAll('tbody th[scope="row"]')).toHaveLength(3);
            expectEditor(basket, xml, added, 8);
            const canonical = '<basket><pear>+001.20</pear><lime>-0</lime></basket>';
            edit(xml, canonical);
            await waitFor(() => expect(values(basket)).toEqual(['1.2', '0', '1.2', '0.6']));
            expect(rows()).toEqual([['pear', '1.2'], ['lime', '0']]);
            for (const invalid of [
                '<basket><pear>bad</pear></basket>', '<basket><pear/></basket>',
                '<basket><pear>-1</pear></basket>', '<basket><pear><qty>1</qty></pear></basket>',
                '<other><pear>1</pear></other>', '<basket>',
            ]) {
                edit(xml, invalid);
                await waitFor(() => expect(basket.querySelector('[role="alert"]')?.textContent)
                    .toContain(invalid === '<basket>' ? 'XML' : 'non-negative decimal'));
                expect(values(basket)).toEqual([]);
                expect(basket.querySelector('table')).toBeNull();
                expectEditor(basket, xml, invalid);
                edit(xml, '<basket><plum>7</plum></basket>');
                await waitFor(() => expect(values(basket)).toEqual(['7', '7', '7', '7']));
                expect(rows()).toEqual([['plum', '7']]);
                expect(basket.querySelector('[role="alert"]')).toBeNull();
            }
            edit(xml, '<basket/>');
            await waitFor(() => expect(values(basket)).toEqual(['0', '∅', '∅', '∅']));
            expect(rows()).toEqual([]);
            expect(basket.querySelector('table')).not.toBeNull();
            edit(xml, '<basket><plum>7</plum></basket>');
            await waitFor(() => expect(values(basket)).toEqual(['7', '7', '7', '7']));
            expectEditor(basket, xml, '<basket><plum>7</plum></basket>');
            expect(values(numbers)).toEqual(['0', '∅', '∅', '∅']);
        });
        expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../index.html', './xpath-aggregates.cemt', './xpath-maps-arrays.html',
                './xpath-sequences.html', './xpath-nodes.html', './data-table.html'].map(relative => new URL(relative, DEMO_URL).href));
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
