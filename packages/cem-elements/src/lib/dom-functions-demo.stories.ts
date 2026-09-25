import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor, within } from 'storybook/test';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
import authoredPage from '../../demo/functions/dom.html?raw';

const meta: Meta = { title: 'CEM Elements/CEM-QL DOM Functions', tags: ['test'] };
export default meta;
type Story = StoryObj;
const SOURCE_TAG = 'story-dom-functions-document';
const DEMO_URL = new URL('../../demo/functions/dom.html', import.meta.url);
const RESULTS: Record<string, string> = {
    "Wrap values": "a, b, a",
    "Parent": "row",
    "Element children": "a, b",
    "All child nodes": "4",
    "Ancestors": "section, row",
    "Closest ancestor": "section",
    "Local names": "fruit",
    "Node text": "ivysaur",
    "Unqualified attribute": "2",
    "Qualified attribute": "other",
    "First match": "2",
    "Last match": "b",
    "Filter": "a, a",
    "First value": "a",
    "Last value": "b",
    "Zero-based selection": "b",
    "Take a prefix": "a, b",
    "Skip a prefix": "b, c",
    "Map values": "a!, b!",
    "Flatten mapped values": "a, a, b, b",
    "Any match": "true",
    "All match": "false",
    "Empty chain": "true",
    "Count values": "3",
    "Sort values": "a, b, b",
    "Sort native nodes": "second, first, third",
    "Reverse order": "c, b, a"
};

const firstMatchCases = [
    ['name', 'ivy'], ['missing', ''], ['', ''], ['Name', ''], [' id ', ''],
    ['row', ''], ['name/id', ''], ['<id>', ''], ['名', ''],
];

function render(): string {
    return `<cem-element tag="${SOURCE_TAG}" src="${DEMO_URL.href}" link-base="source" hidden></cem-element><${SOURCE_TAG}></${SOURCE_TAG}>`;
}
function sample(root: HTMLElement, legend: string): HTMLElement {
    const card = root.querySelector<HTMLElement>(`cem-demo-element[legend="${legend}"]`);
    if (!card) throw new Error(`Missing ${legend}`);
    return card;
}
async function settle(root: HTMLElement): Promise<void> {
    const host = root.querySelector<HTMLElement>(SOURCE_TAG);
    if (!host) throw new Error('DOM functions document is missing');
    await whenCemSourceRendered(host);
}
async function verify(root: HTMLElement, legends: string[]): Promise<void> {
    for (const legend of legends) {
        await waitFor(() => {
            const card = sample(root, legend);
            expect(card.querySelectorAll('article output')).toHaveLength(1);
            expect(card.querySelector('article output')?.textContent).toBe(RESULTS[legend]);
        }, { timeout: 30000 });
    }
    await settle(root);
}
function diagnostics(root: HTMLElement): void {
    const host = root.querySelector<HTMLElement>(SOURCE_TAG);
    if (!host) throw new Error('DOM functions document is missing');
    expect(cemDiagnosticCodes(host)).toEqual([]);
    for (const declaration of root.querySelectorAll<HTMLElement>('cem-element[tag]')) {
        expect(cemDiagnosticCodes(declaration)).toEqual([]);
        const tag = declaration.getAttribute('tag');
        if (tag) for (const instance of root.querySelectorAll<HTMLElement>(tag)) {
            expect(cemDiagnosticCodes(instance)).toEqual([]);
        }
    }
}
async function firstMatch(root: HTMLElement): Promise<void> {
    const card = sample(root, 'First match');
    const input = within(card).getByRole('textbox', { name: 'Local name' }) as HTMLInputElement;
    expect(input.value).toBe('id');
    const edit = async (value: string, expected: string) => {
        input.focus();
        input.value = value;
        const caret = Math.min(1, value.length);
        input.setSelectionRange(caret, caret);
        input.dispatchEvent(new Event('input', { bubbles: true }));
        await waitFor(() => expect(card.querySelector('output')?.textContent).toBe(expected));
        await settle(root);
        expect(card.querySelector('output')?.textContent).toBe(expected);
        expect(within(card).getByRole('textbox', { name: 'Local name' })).toBe(input);
        expect(input.value).toBe(value);
        expect(document.activeElement).toBe(input);
        expect([input.selectionStart, input.selectionEnd]).toEqual([caret, caret]);
    };
    for (const [value, expected] of firstMatchCases) {
        await edit(value, expected);
        await edit('id', '2');
    }
    await edit('name', 'ivy');
}
async function sortNodes(root: HTMLElement): Promise<void> {
    const card = sample(root, 'Sort native nodes');
    const select = within(card).getByRole('combobox', { name: 'Direction' }) as HTMLSelectElement;
    expect(Array.from(select.options, option => [option.value, option.text])).toEqual([
        ['ascending', 'Ascending'], ['descending', 'Descending'],
    ]);
    expect(select.value).toBe('ascending');
    for (const direction of ['descending', 'ascending', 'descending', 'ascending', 'descending']) {
        const expected = direction === 'ascending' ? 'second, first, third' : 'first, third, second';
        select.focus();
        select.value = direction;
        select.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => expect(card.querySelector('output')?.textContent).toBe(expected));
        await settle(root);
        expect(card.querySelector('output')?.textContent).toBe(expected);
        expect(within(card).getByRole('combobox', { name: 'Direction' })).toBe(select);
        expect(select.value).toBe(direction);
        expect(select.selectedIndex).toBe(direction === 'ascending' ? 0 : 1);
        expect(document.activeElement).toBe(select);
    }
}
export const AuthoredSamples: Story = {
    render,
    play: async ({ canvasElement, step }) => {
        await verify(canvasElement, Object.keys(RESULTS));
        for (const [legend, expected] of Object.entries(RESULTS)) {
            await step(legend, async () => {
                const card = sample(canvasElement, legend);
                expect(card.querySelectorAll('article')).toHaveLength(1);
                expect(card.querySelectorAll('article output')).toHaveLength(1);
                expect(card.querySelector('article output')?.textContent).toBe(expected);
                if (legend === 'First match') await firstMatch(canvasElement);
                if (legend === 'Sort native nodes') await sortNodes(canvasElement);
            });
        }
        for (const [legend, expected] of Object.entries(RESULTS)) {
            const final = legend === 'First match' ? 'ivy'
                : legend === 'Sort native nodes' ? 'first, third, second' : expected;
            expect(sample(canvasElement, legend).querySelector('output')?.textContent).toBe(final);
        }
        expect(sample(canvasElement, 'First match').querySelector('input')?.value).toBe('name');
        const template = document.createElement('template');
        template.innerHTML = authoredPage;
        const expected = Array.from(template.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: (card.querySelector('template') as HTMLTemplateElement).innerHTML,
        }));
        await waitFor(() => expect(Array.from(canvasElement.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: card.querySelector('[slot=text] code')?.textContent,
        }))).toEqual(expected), { timeout: 30000 });
        expect(Array.from(canvasElement.querySelectorAll<HTMLAnchorElement>(`${SOURCE_TAG} nav a, ${SOURCE_TAG} main > section a`), link => link.href))
            .toEqual(['../../index.html', '../../../../docs/cem-ql-chains.md', '../cell-overrides.html',
                '../xpath-functions.html#cem-ql-nodes', './str.html'].map(relative => new URL(relative, DEMO_URL).href));
        await settle(canvasElement);
        diagnostics(canvasElement);
    },
};
export const NavigationAndExtraction: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, [
            'Parent', 'Element children', 'All child nodes', 'Ancestors', 'Closest ancestor',
            'Local names', 'Node text', 'Unqualified attribute', 'Qualified attribute',
        ]);
        diagnostics(canvasElement);
    },
};
export const SelectionAndMapping: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, ['First match', 'Last match', 'Filter', 'First value',
            'Last value', 'Zero-based selection', 'Take a prefix', 'Skip a prefix', 'Map values', 'Flatten mapped values']);
        await firstMatch(canvasElement);
        diagnostics(canvasElement);
    },
};
export const ScalarQuestionsAndEmptyChains: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, ['Any match', 'All match', 'Empty chain', 'Count values']);
        diagnostics(canvasElement);
    },
};
export const ImmutableOrdering: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, ['Sort values', 'Sort native nodes', 'Reverse order']);
        await sortNodes(canvasElement);
        diagnostics(canvasElement);
    },
};
