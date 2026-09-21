import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import authoredPage from '../../demo/cell-overrides.html?raw';

const meta: Meta = { title: 'CEM Elements/Cell Template Overrides', tags: ['test'] };
export default meta;
type Story = StoryObj;
const SOURCE_URL = new URL('../../demo/cell-overrides.html', import.meta.url);
function renderDocument(): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', 'story-cell-overrides-document');
    declaration.setAttribute('src', SOURCE_URL.href);
    root.append(declaration, document.createElement('story-cell-overrides-document'));
    return root;
}
async function ready(root: HTMLElement, tag: string): Promise<HTMLElement> {
    await waitFor(() => expect(root.querySelector(`${tag} table`)).not.toBeNull(), { timeout: 30000 });
    return root.querySelector(tag) as HTMLElement;
}
function change(input: HTMLTextAreaElement, source: string): void {
    input.value = source;
    input.dispatchEvent(new Event('change', { bubbles: true }));
}
function select(element: HTMLElement, value: string): void {
    (element as HTMLSelectElement).value = value;
    element.dispatchEvent(new Event('change', { bubbles: true }));
}

export const AuthoredSourcePreviews: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const authored = document.createElement('template');
        authored.innerHTML = authoredPage;
        const expected = Array.from(authored.content.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: (card.querySelector('template') as HTMLTemplateElement).innerHTML,
        }));
        expect(expected).toHaveLength(3);
        await waitFor(() => {
            expect(canvasElement.querySelectorAll('cem-demo-element[data-state=ready]')).toHaveLength(3);
        }, { timeout: 30000 });
        expect(Array.from(canvasElement.querySelectorAll('cem-demo-element'), card => ({
            legend: card.getAttribute('legend'),
            source: card.querySelector('[slot=text] code')?.textContent,
        }))).toEqual(expected);
        const template = canvasElement.querySelector('cem-demo-element > template') as HTMLTemplateElement;
        expect(template.content.querySelector('cem-element')?.hasAttribute('data-cem-render-node-id')).toBe(true);
        await waitFor(() => {
            expect(canvasElement.querySelectorAll('cem-pokemon-cells img')).toHaveLength(2);
            expect(canvasElement.querySelector('cem-stock-cells strong')).toHaveTextContent('Out of stock (0)');
            expect(canvasElement.querySelector('cem-native-value-card article')).toHaveTextContent('Next count: 3');
        }, { timeout: 10000 });
    },
};

export const PokemonCellPictures: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const viewer = await ready(canvasElement, 'cem-pokemon-cells');
        const controls = within(viewer);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        const images = () => Array.from(viewer.querySelectorAll('img'));
        await waitFor(() => {
            expect(images().map(image => image.alt)).toEqual(['venusaur', 'ivysaur']);
            expect(images().every(image => image.complete && image.naturalWidth > 0)).toBe(true);
        }, { timeout: 10000 });
        for (const image of images()) expect(image.src).toContain(new URL('./pokemon/', SOURCE_URL).href);
        expect(viewer.querySelector('table')).toHaveTextContent('venusaur');
        expect(viewer.querySelector('table')).toHaveTextContent('ivysaur');
        expect(Array.from(viewer.querySelectorAll('thead th'), th => th.textContent?.trim())).toEqual(['✓', '#text', 'id', 'name', 'type']);
        await userEvent.click(viewer.querySelector('tbody button') as HTMLButtonElement);
        await waitFor(() => expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('venusaur'), { timeout: 10000 });
        select(controls.getByRole('combobox', { name: 'Sort column' }), 'name');
        await waitFor(() => expect(images().map(image => image.alt)).toEqual(['ivysaur', 'venusaur']), { timeout: 10000 });
        expect(viewer.querySelector('tr[aria-selected=true]')).toHaveTextContent('venusaur');
        expect(input.value).toBe(original);
        input.focus();
        change(input, original.replace('<name>venusaur</name>', '<title>venusaur</title>'));
        await waitFor(() => expect(images()).toHaveLength(1), { timeout: 10000 });
        expect(viewer.querySelector('table')).toHaveTextContent('venusaur');
        expect(viewer.querySelector('table')).toHaveTextContent('grass');
        expect(document.activeElement).toBe(input);
        change(input, '<broken>');
        await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
        expect(viewer.querySelector('table')).toBeNull();
        expect(images()).toHaveLength(0);
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await waitFor(() => expect(images()).toHaveLength(2), { timeout: 10000 });
        expect(input.value).toBe(original);
        expect(viewer.querySelectorAll('td .pokemon-name')).toHaveLength(2);
        for (const cell of viewer.querySelectorAll('td .pokemon-name')) {
            expect(cell).toHaveTextContent((cell.querySelector('img') as HTMLImageElement).alt);
        }
    },
};

export const ConditionalStockFallback: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const viewer = await ready(canvasElement, 'cem-stock-cells');
        const controls = within(viewer);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        expect(viewer.querySelectorAll('strong')).toHaveLength(1);
        expect(viewer.querySelector('strong')).toHaveTextContent('Out of stock (0)');
        expect(viewer.querySelector('table')).toHaveTextContent('5');
        const reference = Array.from(viewer.querySelectorAll('details')).find(details =>
            details.querySelector(':scope > summary')?.textContent?.includes('document/reference'));
        expect(reference).toHaveTextContent('0');
        expect(reference?.querySelector('strong')).toBeNull();
        input.focus();
        change(input, original.replace('<stock>0</stock>', '<stock>7</stock>'));
        await waitFor(() => expect(viewer.querySelector('table')).toHaveTextContent('7'), { timeout: 10000 });
        expect(viewer.querySelector('strong')).toBeNull();
        expect(document.activeElement).toBe(input);
        change(input, original.replace('<stock>5</stock>', '<stock>0</stock>'));
        await waitFor(() => expect(viewer.querySelectorAll('strong')).toHaveLength(2), { timeout: 10000 });
        expect(input.value).toContain('<name>Lemon</name><stock>0</stock>');
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await waitFor(() => expect(viewer.querySelectorAll('strong')).toHaveLength(1), { timeout: 10000 });
        expect(input.value).toBe(original);
    },
};

export const NativeAttributeValues: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('cem-native-value-card article')).not.toBeNull(), { timeout: 10000 });
        const card = canvasElement.querySelector('cem-native-value-card article');
        expect(card).toHaveTextContent('Next count: 3');
        expect(card).toHaveTextContent('Date: 2024-02-29');
        expect(card?.querySelector('.value-label > name > em')).toHaveTextContent('saur');
        expect(card?.querySelector(':scope > section > p')).toHaveTextContent('Text-only label: ivysaur');
        expect(card?.querySelector(':scope > section name')).toBeNull();
    },
};
