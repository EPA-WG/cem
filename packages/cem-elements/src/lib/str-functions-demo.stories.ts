import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
import authoredPage from '../../demo/functions/str.html?raw';

const SOURCE_TAG = 'story-str-functions-document';
const DEMO_URL = new URL('../../demo/functions/str.html', import.meta.url);
const MATRIX_LEGEND = 'str:shorten query/result matrix';
const meta: Meta = { title: 'CEM Elements/CEM-QL String Functions Demo', tags: ['test'] };
export default meta;
type Story = StoryObj;
type StringCase = {
    legend: string;
    fields: [label: string, selector: string, initial: string][];
    initial: string;
    initialParts?: string[];
    edits: [field: number, value: string, output: string, parts?: string[]][];
};

const shortenRows = [
    ['str:shorten("short", 8)', 'short'],
    ['str:shorten("abcdefghij", 7)', 'abc…hij'],
    ['str:shorten("abcdefghij", 8)', 'abc…ghij'],
    ['str:shorten("abcdefghij", 8, "...")', 'ab...hij'],
    ['str:shorten("abcdefghij", 6, "")', 'abchij'],
    ['str:shorten("αβ😀δεζη", 5, "💠")', 'αβ💠ζη'],
    ['str:shorten( "https://example.test/lib/semantic-card.cem" , 32)', 'https://example…emantic-card.cem'],
];
const stringCases: StringCase[] = [
    {
        legend: 'URL ID with a string chain',
        fields: [
            ['Pokémon URL', 'article label:nth-of-type(1) input', 'https://pokeapi.co/api/v2/pokemon/1/'],
        ],
        initial: '1',
        edits: [
            [0, 'https://pokeapi.co/api/v2/pokemon/10/', '10'],
            [0, '/short/', 'No ID'],
            [0, '', 'No ID'],
            [0, 'https://pokeapi.co/api/v2/pokemon/', ''],
            [0, 'https://pokeapi.co/api/v2/pokemon', 'No ID'],
            [0, 'a/b/c/d/e/f/🍒', '🍒'],
            [0, 'a/b/c/d/e/f/  ivy  /more', '  ivy  '],
            [0, 'https://pokeapi.co/api/v2/pokemon/25/', '25'],
        ],
    },
    {
        legend: 'str:split',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒,🍋,,🍌'],
            ['Separator', 'article label:nth-of-type(2) input', ','],
        ],
        initial: '4',
        initialParts: ['🍒', '🍋', '', '🍌'],
        edits: [
            [0, 'a::b::::', '1', ['a::b::::']],
            [1, '::', '4', ['a', 'b', '', '']],
            [0, 'aaa', '1', ['aaa']],
            [1, 'aa', '2', ['', 'a']],
            [0, 'a.b.*c', '1', ['a.b.*c']],
            [1, '.', '3', ['a', 'b', '*c']],
            [0, '🍒🍋🍒', '1', ['🍒🍋🍒']],
            [1, '🍒', '3', ['', '🍋', '']],
            [0, 'a🍒e\u0301', '2', ['a', 'e\u0301']],
            [1, '', '6', ['', 'a', '🍒', 'e', '\u0301', '']],
            [0, '', '2', ['', '']],
            [1, ',', '1', ['']],
            [0, '🍒,🍋,,🍌,', '5', ['🍒', '🍋', '', '🍌', '']],
        ],
    },
    {
        legend: 'str:trim',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '🍒  🍋',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', '🍌  🍒'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '🍒  🍋'],
        ],
    },
    {
        legend: 'str:trim_start',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '🍒  🍋  ',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', '🍌  🍒 \uFEFF'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '🍒  🍋  '],
        ],
    },
    {
        legend: 'str:trim_end',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '  🍒  🍋  '],
        ],
        initial: '  🍒  🍋',
        edits: [
            [0, ' \u00a0🍌  🍒 \uFEFF', ' \u00a0🍌  🍒'],
            [0, '\u0085🍒\u0085', '\u0085🍒\u0085'],
            [0, '\u200b🍒\u200b', '\u200b🍒\u200b'],
            [0, '\t \u00a0\u2003\uFEFF', ''],
            [0, '', ''],
            [0, '  🍒  🍋  ', '  🍒  🍋'],
        ],
    },
    {
        legend: 'str:char_at',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍌'],
            ['Index', 'article label:nth-of-type(2) input', '1'],
        ],
        initial: '🍋',
        edits: [
            [1, '99', ''],
            [1, '-1', ''],
            [1, '-3', ''],
            [1, '-4', ''],
            [1, '0', '🍒'],
            [1, '2', '🍌'],
            [1, '3', ''],
            [1, '', '🍒'],
            [1, '1.5', '🍒'],
            [1, '1e2', '🍒'],
            [0, 'a🍒e\u0301', 'a'],
            [1, '1', '🍒'],
            [1, '2', 'e'],
            [1, '3', '\u0301'],
            [1, '-1', ''],
            [0, '', ''],
            [1, '0', ''],
            [0, 'a🍒b', 'a'],
            [1, '1', '🍒'],
        ],
    },
    {
        legend: 'str:at',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍌'],
            ['Index', 'article label:nth-of-type(2) input', '-1'],
        ],
        initial: '🍌',
        edits: [
            [1, '99', '∅'],
            [1, '-1', '🍌'],
            [1, '-3', '🍒'],
            [1, '-4', '∅'],
            [1, '0', '🍒'],
            [1, '2', '🍌'],
            [1, '3', '∅'],
            [1, '', '🍒'],
            [1, '1.5', '🍒'],
            [1, '1e2', '🍒'],
            [0, 'a🍒e\u0301', 'a'],
            [1, '1', '🍒'],
            [1, '2', 'e'],
            [1, '3', '\u0301'],
            [1, '-1', '\u0301'],
            [0, '', '∅'],
            [1, '0', '∅'],
            [0, 'a🍒b', 'a'],
            [1, '1', '🍒'],
        ],
    },
    {
        legend: 'str:index_of',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍒'],
            ['Find', 'article label:nth-of-type(2) input', '🍒'],
            ['Position', 'article label:nth-of-type(3) input', '0'],
        ],
        initial: '0',
        edits: [
            [2, '1', '2'],
            [1, '🍋', '1'],
            [1, '🥦', '-1'],
            [1, '', '1'],
            [2, '99', '3'],
            [0, 'aaaa', '4'],
            [1, 'aa', '-1'],
            [2, '1', '1'],
            [2, '-9', '0'],
            [2, '', '0'],
            [2, '1.5', '0'],
            [2, '1e2', '0'],
            [0, 'a🍒e\u0301🍒', '-1'],
            [1, '🍒', '1'],
            [2, '2', '4'],
            [1, 'e\u0301', '2'],
            [1, 'é', '-1'],
            [0, '', '-1'],
            [1, '', '0'],
            [2, '99', '0'],
            [0, 'a🍒a', '3'],
            [1, 'a', '-1'],
            [2, '0', '0'],
        ],
    },
    {
        legend: 'str:last_index_of',
        fields: [
            ['Text', 'article label:nth-of-type(1) input', '🍒🍋🍒'],
            ['Find', 'article label:nth-of-type(2) input', '🍒'],
            ['Position', 'article label:nth-of-type(3) input', '3'],
        ],
        initial: '2',
        edits: [
            [2, '1', '0'],
            [1, '🍋', '1'],
            [1, '🥦', '-1'],
            [1, '', '1'],
            [2, '99', '3'],
            [0, 'aaaa', '4'],
            [1, 'aa', '2'],
            [2, '1', '1'],
            [2, '-9', '0'],
            [2, '', '2'],
            [2, '1.5', '2'],
            [2, '1e2', '2'],
            [0, 'a🍒e\u0301🍒', '-1'],
            [1, '🍒', '4'],
            [2, '2', '1'],
            [1, 'e\u0301', '2'],
            [1, 'é', '-1'],
            [0, '', '-1'],
            [1, '', '0'],
            [2, '99', '0'],
            [0, 'a🍒a', '3'],
            [1, 'a', '2'],
            [2, '0', '0'],
        ],
    },
    {
        legend: 'XPath normalize-space',
        fields: [
            ['Text', 'article label:nth-of-type(1) textarea', '  🍒  🍋  '],
        ],
        initial: '🍒 🍋',
        edits: [
            [0, ' a\ta\n🍒  ', 'a a 🍒'],
            [0, ' a\u00a0\u2003b ', 'a\u00a0\u2003b'],
            [0, ' \t\n', ''],
            [0, '', ''],
            [0, ' \uFEFFa\u0085b\u200bc ', '\uFEFFa\u0085b\u200bc'],
            [0, '  a a  🍒 \n', 'a a 🍒'],
        ],
    },
    {
        legend: 'XPath tokenize and string-join',
        fields: [
            ['Text', 'article label:nth-of-type(1) textarea', '🍒 🍒 🍋'],
            ['Separator', 'article label:nth-of-type(2) input', '/'],
        ],
        initial: '🍒/🍒/🍋',
        edits: [
            [0, 'a\ta\nb', 'a/a/b'],
            [1, '🍒', 'a🍒a🍒b'],
            [1, '', 'aab'],
            [1, ' <&> ', 'a <&> a <&> b'],
            [0, 'a\u00a0b', 'a\u00a0b'],
            [0, ' a\u00a0\u2003b a ', 'a\u00a0\u2003b <&> a'],
            [0, ' \t\n', ''],
            [0, '', ''],
            [1, '|', ''],
            [0, ' 🍒 🍒 🍋 ', '🍒|🍒|🍋'],
        ],
    },
];

export const EveryAuthoredSample: Story = {
    render: () => `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`,
    play: async ({ canvasElement, step }) => {
        const host = requiredElement(canvasElement, SOURCE_TAG);
        const sample = (legend: string) => requiredElement(host, `cem-demo-element[legend="${legend}"]`);
        await waitForCondition(() => host.querySelectorAll('cem-demo-element[legend]').length === 12,
            'all string samples render from the authored source');
        expect(Array.from(host.querySelectorAll('cem-demo-element'), card => card.getAttribute('legend')))
            .toEqual([MATRIX_LEGEND, ...stringCases.map(entry => entry.legend)]);
        await step(MATRIX_LEGEND, async () => {
            const card = sample(MATRIX_LEGEND);
            const rows = () => Array.from(card.querySelectorAll('tbody tr'), row =>
                Array.from(row.querySelectorAll('td'), cell => normalize(cell.textContent ?? '')));
            await waitForCondition(() => JSON.stringify(rows()) === JSON.stringify(shortenRows), 'all shortening queries and results');
            expect(Array.from(card.querySelectorAll('thead th'), cell => cell.textContent)).toEqual(['Query', 'Result']);
            expect(card.querySelectorAll('tbody code')).toHaveLength(14);
        });
        for (const entry of stringCases) {
            await step(entry.legend, async () => {
                const card = sample(entry.legend);
                const fields = entry.fields.map(([, selector]) => requiredElement(card, selector) as HTMLInputElement | HTMLTextAreaElement);
                const values = entry.fields.map(([, , value]) => value);
                await waitForOutput(card, entry.initial);
                await whenCemSourceRendered(host);
                expect(fields.map(field => field.value)).toEqual(values);
                expect(fields.map(field => normalize(field.labels?.[0]?.firstChild?.textContent ?? '')))
                    .toEqual(entry.fields.map(([label]) => label));
                if (entry.initialParts) expect(parts(card)).toEqual(entry.initialParts.map(part => `“${part}”`));
                for (const [index, value, output, expectedParts] of entry.edits) {
                    const field = fields[index];
                    const numeric = field instanceof HTMLInputElement && field.type === 'number';
                    field.focus();
                    // Storybook's user-event interceptor canonicalizes numeric strings
                    // (1e2 becomes 100); use the browser setter to match real input.
                    if (numeric) Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')?.set?.call(field, value);
                    else field.value = value;
                    const caret = Math.min(2, value.length);
                    if (!numeric) field.setSelectionRange(caret, caret);
                    field.dispatchEvent(new Event('input', { bubbles: true }));
                    await waitForCondition(() => (field instanceof HTMLTextAreaElement ? field.textContent
                        : field.getAttribute('value') ?? '') === value, `${entry.legend} commits ${entry.fields[index][0]}`);
                    await waitForOutput(card, output);
                    await whenCemSourceRendered(host);
                    values[index] = value;
                    expect(card.querySelectorAll('output')).toHaveLength(1);
                    expect(card.querySelector('output')?.textContent).toBe(output);
                    expect(entry.fields.map(([, selector]) => card.querySelector(selector))).toEqual(fields);
                    expect(fields.map(input => input.value)).toEqual(values);
                    expect(document.activeElement).toBe(field);
                    if (!numeric) expect([field.selectionStart, field.selectionEnd]).toEqual([caret, caret]);
                    if (expectedParts) expect(parts(card)).toEqual(expectedParts.map(part => `“${part}”`));
                }
            });
        }
        for (const entry of stringCases) {
            const card = sample(entry.legend);
            const values = entry.fields.map(([, , value]) => value);
            for (const [index, value] of entry.edits) values[index] = value;
            expect(entry.fields.map(([, selector]) => (card.querySelector(selector) as HTMLInputElement).value)).toEqual(values);
            expect(card.querySelector('output')?.textContent).toBe(entry.edits.at(-1)?.[2]);
        }
        const inert = document.createElement('template');
        inert.innerHTML = authoredPage;
        const expected = Array.from(inert.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'), source: (card.querySelector('template') as HTMLTemplateElement).innerHTML,
        }));
        await waitForCondition(() => JSON.stringify(Array.from(host.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'), source: card.querySelector('[slot="text"] code')?.textContent,
        }))) === JSON.stringify(expected), 'all source previews preserve the authored templates');
        expect(Array.from(host.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
            .toEqual(['../../index.html', '../xpath-text.cemt', './dom.html', '../xpath-functions.html',
                '../xpath-sequences.html', '../dom-merge.html', '../data-slices.html', '../module-url.html']
                .map(relative => new URL(relative, DEMO_URL).href));
        await whenCemSourceRendered(host);
        expect(cemDiagnosticCodes(host)).toEqual([]);
        for (const declaration of canvasElement.querySelectorAll<HTMLElement>('cem-element[tag]')) {
            expect(cemDiagnosticCodes(declaration)).toEqual([]);
            const tag = declaration.getAttribute('tag');
            if (tag) for (const instance of canvasElement.querySelectorAll<HTMLElement>(tag)) {
                expect(cemDiagnosticCodes(instance)).toEqual([]);
            }
        }
    },
};

function parts(root: HTMLElement): string[] {
    return Array.from(root.querySelectorAll('li'), item => item.textContent ?? '');
}
async function waitForOutput(root: HTMLElement, expected: string): Promise<void> {
    await waitForCondition(() => root.querySelector('output')?.textContent === expected,
        `${root.getAttribute('legend')} output should be ${JSON.stringify(expected)}`);
}
async function waitForCondition(condition: () => boolean, message: string, attempts = 600): Promise<void> {
    for (let attempt = 0; attempt < attempts; attempt += 1) {
        if (condition()) return;
        await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
    }
    throw new Error(message);
}
function requiredElement(root: ParentNode, selector: string): HTMLElement {
    const element = root.querySelector<HTMLElement>(selector);
    if (!element) throw new Error(`expected ${selector}`);
    return element;
}
function normalize(value: string): string {
    return value.replace(/\s+/gu, ' ').trim();
}
