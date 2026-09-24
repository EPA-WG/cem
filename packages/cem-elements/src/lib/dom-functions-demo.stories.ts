import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor, within } from 'storybook/test';
import authoredPage from '../../demo/functions/dom.html?raw';

const meta: Meta = { title: 'CEM Elements/CEM-QL DOM Functions', tags: ['test'] };
export default meta;
type Story = StoryObj;
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

function render(): string {
    return `<cem-element tag="story-dom-functions-document" src="${DEMO_URL.href}" hidden></cem-element><story-dom-functions-document></story-dom-functions-document>`;
}
async function verify(root: HTMLElement, legends: string[]): Promise<void> {
    for (const legend of legends) {
        await waitFor(() => {
            const card = root.querySelector(`cem-demo-element[legend="${legend}"]`);
            expect(card?.querySelector('output')?.textContent?.trim()).toBe(RESULTS[legend]);
        }, { timeout: 30000 });
    }
}
export const AuthoredSamples: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, Object.keys(RESULTS));
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
    },
};
export const NavigationAndExtraction: Story = {
    render,
    play: async ({ canvasElement }) => verify(canvasElement, [
        'Parent', 'Element children', 'All child nodes', 'Ancestors', 'Closest ancestor',
        'Local names', 'Node text', 'Unqualified attribute', 'Qualified attribute',
    ]),
};
export const SelectionAndMapping: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, ['First match', 'Last match', 'Filter', 'First value',
            'Last value', 'Take a prefix', 'Skip a prefix', 'Map values', 'Flatten mapped values']);
        const card = canvasElement.querySelector('cem-demo-element[legend="First match"]');
        if (!(card instanceof HTMLElement)) throw new Error('First match sample missing');
        const input = within(card).getByRole('textbox', { name: 'Local name' }) as HTMLInputElement;
        for (const [value, expected] of [['name', 'ivy'], ['missing', ''], ['id', '2']]) {
            input.focus();
            input.value = value;
            input.dispatchEvent(new Event('input', { bubbles: true }));
            await waitFor(() => expect(card.querySelector('output')?.textContent?.trim()).toBe(expected));
            expect(document.activeElement).toBe(input);
        }
    },
};
export const ScalarQuestionsAndEmptyChains: Story = {
    render,
    play: async ({ canvasElement }) => verify(canvasElement, ['Any match', 'All match', 'Empty chain', 'Count values']),
};
export const ImmutableOrdering: Story = {
    render,
    play: async ({ canvasElement }) => {
        await verify(canvasElement, ['Sort values', 'Sort native nodes', 'Reverse order']);
        const card = canvasElement.querySelector('cem-demo-element[legend="Sort native nodes"]');
        if (!(card instanceof HTMLElement)) throw new Error('Sort sample missing');
        const select = within(card).getByRole('combobox', { name: 'Direction' }) as HTMLSelectElement;
        for (const [direction, expected] of [['descending', 'first, third, second'], ['ascending', 'second, first, third']]) {
            select.value = direction;
            select.dispatchEvent(new Event('change', { bubbles: true }));
            await waitFor(() => expect(card.querySelector('output')?.textContent?.trim()).toBe(expected));
        }
    },
};
