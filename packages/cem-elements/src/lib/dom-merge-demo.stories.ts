import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { cemDiagnosticCodes, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';

const SOURCE_TAG = 'story-dom-merge-document';
const DEMO_URL = new URL('../../demo/dom-merge.html', import.meta.url);
const EXPECTED_LEGENDS = [
    '1. Textarea word count',
    '2. Input word and character count',
    '3. XPath word and character count',
] as const;

type TextControl = HTMLInputElement | HTMLTextAreaElement;
type Selection = [number, number, 'backward'?];
const selectionFor = (value: string): Selection => value ? [0, Math.min(2, value.length), 'backward'] : [0, 0];

const meta: Meta = { title: 'CEM Elements/DOM Merge Demo', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const EveryAuthoredSample: Story = {
    render: () => {
        const root = document.createElement('section');
        root.setAttribute('aria-label', 'source-loaded DOM merge demo coverage');
        const declaration = document.createElement('cem-element');
        declaration.hidden = true;
        declaration.setAttribute('tag', SOURCE_TAG);
        declaration.setAttribute('src', DEMO_URL.href);
        root.append(declaration, document.createElement(SOURCE_TAG));
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const host = canvasElement.querySelector<HTMLElement>(SOURCE_TAG);
        if (!host) throw new Error('DOM merge source host is missing');
        await whenCemSourceRendered(host);
        expect(Array.from(host.querySelectorAll('cem-demo-element[legend]'), sample => sample.getAttribute('legend')))
            .toEqual(EXPECTED_LEGENDS);
        const samples = EXPECTED_LEGENDS.map(legend => {
            const article = host.querySelector<HTMLElement>(`cem-demo-element[legend="${legend}"] article`);
            if (!article) throw new Error(`Missing article for ${legend}`);
            return article;
        });

        await step(EXPECTED_LEGENDS[0], async () => {
            const article = samples[0];
            const textarea = controlIn(article);
            await expectState(article, textarea, 'Hello world!', ['2']);
            await userEvent.clear(textarea);
            await userEvent.type(textarea, 'one two three');
            await expectState(article, textarea, 'one two three', ['2'], { focused: true });
            // The actual blur commits the change-bound slice.
            await userEvent.tab();
            await expectState(article, textarea, 'one two three', ['3'], { focused: false });
            for (const [value, words] of [
                [' one\tone\n🍒  🍋 ', '4'], ['\t\n\u00a0\u2003', '0'],
                ['🍒e\u0301', '1'], [' \t\n', '0'], ['', '0'],
            ]) {
                edit(textarea, value, 'change');
                await expectState(article, textarea, value, [words], { focused: true, selection: selectionFor(value) });
            }
            expect(cemDiagnosticCodes(article.parentElement as HTMLElement)).toEqual([]);
        });

        await step(EXPECTED_LEGENDS[1], async () => {
            const article = samples[1];
            const input = controlIn(article);
            await expectState(article, input, 'Type to update', ['14', '3'], { output: 'Type to update' });
            edit(input, 'two words');
            await expectState(article, input, 'two words', ['9', '2'],
                { output: 'two words', focused: true, selection: selectionFor('two words') });
            replaceSelection(input, 4, 4, 'short ');
            await expectState(article, input, 'two short words', ['15', '3'],
                { output: 'two short words', focused: true, selection: [10, 10] });
            replaceSelection(input, 4, 9, '🍒');
            await expectState(article, input, 'two 🍒 words', ['11', '3'],
                { output: 'two 🍒 words', focused: true, selection: [6, 6] });
            expect(controlIn(samples[0]).value).toBe('');
            expect(controlIn(samples[2]).value).toBe('🍒 🍒 🍋');
            for (const [value, words] of [
                ['🍒 🍒 🍋', '3'], ['🍒e\u0301', '1'], ['one\u00a0one\u2003🍋', '3'],
                ['\u00a0\u2003', '0'], ['   ', '0'], ['', '0'],
            ]) {
                edit(input, value);
                await expectState(article, input, value, [String([...value].length), words],
                    { output: value, focused: true, selection: selectionFor(value) });
            }
            expect(cemDiagnosticCodes(article.parentElement as HTMLElement)).toEqual([]);
        });

        await step(EXPECTED_LEGENDS[2], async () => {
            const article = samples[2];
            const textarea = controlIn(article);
            await expectState(article, textarea, '🍒 🍒 🍋', ['5', '3']);
            textarea.focus();
            replaceSelection(textarea, 2, 2, ' red');
            await expectState(article, textarea, '🍒 red 🍒 🍋', ['9', '4'], { focused: true, selection: [6, 6] });
            expect([controlIn(samples[0]).value, controlIn(samples[1]).value]).toEqual(['', '']);
            for (const [value, words] of [
                [' one\tone\n🍒  🍋 ', '4'], ['\t\n\u00a0\u2003', '1'],
                ['one\u00a0one\u2003🍋', '1'], ['🍒e\u0301', '1'], [' \t\n', '0'], ['', '0'],
            ]) {
                edit(textarea, value);
                await expectState(article, textarea, value, [String([...value].length), words],
                    { focused: true, selection: selectionFor(value) });
            }
            expect(cemDiagnosticCodes(article.parentElement as HTMLElement)).toEqual([]);
        });
        // All three same-named slices belong to independent instances.
        expect(samples.map(article => controlIn(article).value)).toEqual(['', '', '']);
        expect(cemDiagnosticCodes(host)).toEqual([]);
    },
};

function controlIn(article: HTMLElement): TextControl {
    const control = article.querySelector<TextControl>('input, textarea');
    if (!control) throw new Error('Counter control is missing');
    return control;
}

async function expectState(article: HTMLElement, original: TextControl, value: string, counts: string[],
    options: { output?: string; focused?: boolean; selection?: Selection } = {}): Promise<void> {
    await whenCemRendered(article.parentElement as HTMLElement);
    const live = controlIn(article);
    expect(live).toBe(original);
    expect(live.isConnected).toBe(true);
    expect(live.value).toBe(value);
    expect(Array.from(article.querySelectorAll('strong'), strong => strong.textContent?.trim())).toEqual(counts);
    if (options.output !== undefined) expect(article.querySelector('output')?.textContent).toBe(options.output);
    if (options.focused !== undefined) expect(document.activeElement === live).toBe(options.focused);
    if (options.selection) {
        expect([live.selectionStart, live.selectionEnd]).toEqual(options.selection.slice(0, 2));
        if (options.selection[2]) expect(live.selectionDirection).toBe(options.selection[2]);
    }
}

function edit(control: TextControl, value: string, eventName = 'input'): void {
    control.focus();
    control.value = value;
    control.setSelectionRange(...selectionFor(value));
    control.dispatchEvent(new Event(eventName, { bubbles: true }));
}

function replaceSelection(control: TextControl, start: number, end: number, text: string): void {
    control.setSelectionRange(start, end);
    control.setRangeText(text, start, end, 'end');
    control.dispatchEvent(new Event('input', { bubbles: true }));
}
