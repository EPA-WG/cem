import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import {
    applyPatchFramesToRange, applyRenderPlanToRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan,
} from './projection.js';

const meta: Meta = { title: 'CEM Elements/Table Inspector', tags: ['test'] };
export default meta;
type Story = StoryObj;
const SOURCE_URL = new URL('../../demo/table-inspector.html', import.meta.url);
function renderDocument(): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', 'story-table-inspector-document');
    declaration.setAttribute('src', SOURCE_URL.href);
    root.append(declaration, document.createElement('story-table-inspector-document'));
    return root;
}
async function ready(root: HTMLElement): Promise<void> {
    await waitFor(() => expect(root.querySelectorAll('cem-table-inspector table')).toHaveLength(6), { timeout: 30000 });
}
function card(root: HTMLElement, legend: string): HTMLElement {
    const viewer = root.querySelector<HTMLElement>(`cem-demo-element[legend="${legend}"] cem-table-inspector`);
    if (!viewer) throw new Error(`missing ${legend}`);
    return viewer;
}
function rows(table: HTMLTableElement): HTMLTableRowElement[] {
    return Array.from(table.querySelectorAll<HTMLTableRowElement>(':scope > tbody > tr'));
}
const values = (table: HTMLTableElement): string[] => rows(table).map(row => row.cells[row.cells.length - 1].textContent?.trim() ?? '');
const selected = (table: HTMLTableElement): string[] => rows(table).filter(row => row.querySelector('input')?.checked)
    .map(row => row.cells[row.cells.length - 1].textContent?.trim() ?? '');
async function order(viewer: HTMLElement, expected: string[]): Promise<void> {
    await waitFor(() => expect(values(viewer.querySelector('table') as HTMLTableElement)).toEqual(expected), { timeout: 10000 });
}
function change(input: HTMLTextAreaElement, value: string): void {
    input.value = value;
    input.dispatchEvent(new Event('change', { bubbles: true }));
}

export const ReboundDirtyCheckbox: Story = {
    render: () => document.createElement('section'),
    play: ({ canvasElement }) => {
        const start = document.createComment('start');
        const end = document.createComment('end');
        canvasElement.append(start, end);
        const bounds = { start, end };
        const plan = (revision: string, slice: string, checked = false): RenderPlan => ({
            producedTag: 'test-table-binding', instanceId: 'one', dataRevision: revision,
            templateArtifactId: 'table', scopePolicyStamp: 'test', outputTarget: 'light-dom',
            nodes: [slice, 'unrelated'].map((name, index) => ({
                kind: 'element', tag: 'input', namespace: 'http://www.w3.org/1999/xhtml',
                renderNodeId: `control-${index}`, children: [],
                attributes: [{ name: 'type', value: 'checkbox' }, { name: 'slice', value: name },
                    ...(checked && index === 0 ? [{ name: 'checked', value: '' }] : [])],
            })),
        });
        const first = plan('1', 'row-A');
        applyRenderPlanToRange(bounds, first, document);
        const [control, unrelated] = canvasElement.querySelectorAll('input');
        control.click(); unrelated.click(); control.focus();
        // The adapter consumes the directive; the patcher must use its retained authored value.
        control.removeAttribute('slice');
        const second = plan('2', 'row-B');
        const result = applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(first, second),
            renderPlanIdentity(second), document);
        expect(result.status).toBe('applied');
        expect(control).not.toBeChecked();
        expect(unrelated).toBeChecked();
        expect(document.activeElement).toBe(control);
        control.click();
        applyRenderPlanToRange(bounds, plan('3', 'row-C'), document);
        expect(canvasElement.querySelector('input')).toBe(control);
        expect(control).not.toBeChecked();
        expect(unrelated).toBeChecked();
        const fourth = plan('4', 'row-D', true);
        applyRenderPlanToRange(bounds, fourth, document);
        control.click();
        const fifth = plan('5', 'row-E', true);
        applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(fourth, fifth), renderPlanIdentity(fifth), document);
        expect(control).toBeChecked();
        control.click();
        applyRenderPlanToRange(bounds, plan('6', 'row-E', true), document);
        expect(control).not.toBeChecked();
        expect(document.activeElement).toBe(control);
    },
};

export const ColumnsAndText: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await ready(canvasElement);
        const columns = card(canvasElement, '1. Columns from every row');
        const table = columns.querySelector('table') as HTMLTableElement;
        const headings = Array.from(table.querySelectorAll('thead th'), node => node.textContent?.replace(/[↑↓]/gu, '').trim());
        expect(headings).toEqual(['✓', '@early', '#text', '@later']);
        expect(rows(table)[0].cells[1]).toHaveTextContent('""');
        expect(rows(table)[0].cells[3]).toHaveTextContent('∅');
        expect(rows(table)[1].cells[1]).toHaveTextContent('∅');
        expect(rows(table)[1].cells[3]).toHaveTextContent('ripe');
        const text = card(canvasElement, '2. Text-only rows stay visible');
        await order(text, ['ivysaur', 'venusaur']);
        await userEvent.click(within(text).getByRole('button', { name: 'Sort #text descending in document/name' }));
        await order(text, ['venusaur', 'ivysaur']);
        expect(text.querySelector('th[aria-sort]')).toHaveAttribute('aria-sort', 'descending');
        expect(columns.querySelector('th[aria-sort]')).toBeNull();
    },
};

export const MultipleSelectionAndRecovery: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await ready(canvasElement);
        const viewer = card(canvasElement, '4. Multiple selections survive sorting');
        const controls = within(viewer);
        const table = () => viewer.querySelector('table') as HTMLTableElement;
        await userEvent.click(rows(table())[1].querySelector('input') as HTMLInputElement);
        await waitFor(() => expect(table().querySelector('caption output')).toHaveTextContent(/^1$/), { timeout: 10000 });
        await userEvent.click(rows(table())[2].querySelector('input') as HTMLInputElement);
        await waitFor(() => expect(table().querySelector('caption output')).toHaveTextContent(/^2$/), { timeout: 10000 });
        const mode = controls.getByRole('combobox', { name: 'Compare document/row' }) as HTMLSelectElement;
        mode.value = 'number'; mode.dispatchEvent(new Event('change', { bubbles: true }));
        await userEvent.click(controls.getByRole('button', { name: 'Sort @qty ascending in document/row' }));
        await order(viewer, ['Cherry 🍒', 'Apple 🍏', 'Lemon 🍋', 'Banana 🍌']);
        expect(selected(table())).toEqual(['Apple 🍏', 'Lemon 🍋']);
        expect(table().querySelector('th[aria-sort]')).toHaveAttribute('aria-sort', 'ascending');
        await userEvent.click(controls.getByRole('button', { name: 'Sort @qty descending in document/row' }));
        await order(viewer, ['Lemon 🍋', 'Cherry 🍒', 'Apple 🍏', 'Banana 🍌']);
        expect(selected(table())).toEqual(['Lemon 🍋', 'Apple 🍏']);
        expect(table().querySelector('caption output')).toHaveTextContent(/^2$/);
        await userEvent.click(rows(table())[0].querySelector('input') as HTMLInputElement);
        await waitFor(() => expect(selected(table())).toEqual(['Apple 🍏']), { timeout: 10000 });
        await userEvent.click(controls.getByRole('button', { name: 'Restore source order in document/row' }));
        await order(viewer, ['Cherry 🍒', 'Lemon 🍋', 'Apple 🍏', 'Banana 🍌']);
        expect(selected(table())).toEqual(['Apple 🍏']);
        expect(table().querySelector('th[aria-sort]')).toBeNull();
        expect(card(canvasElement, '1. Columns from every row').querySelectorAll('input:checked')).toHaveLength(0);

        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        const second = document.createElement('cem-table-inspector');
        second.setAttribute('format', 'xml');
        second.setAttribute('inspector', 'true');
        second.textContent = original;
        viewer.parentElement?.append(second);
        await waitFor(() => expect(second.querySelector('table')).not.toBeNull(), { timeout: 10000 });
        expect(selected(second.querySelector('table') as HTMLTableElement)).toEqual([]);
        await userEvent.click(second.querySelector('input') as HTMLInputElement);
        await waitFor(() => expect(second.querySelector('caption output')).toHaveTextContent(/^1$/), { timeout: 10000 });
        expect(selected(table())).toEqual(['Apple 🍏']);
        second.remove();
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await waitFor(() => expect(selected(table())).toEqual([]), { timeout: 10000 });
        expect(input.value).toBe(original);
        input.focus(); change(input, '<broken>');
        await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
        expect(viewer.querySelector('table')).toBeNull();
        expect(document.activeElement).toBe(input);
        change(input, original);
        await order(viewer, ['Cherry 🍒', 'Lemon 🍋', 'Apple 🍏', 'Banana 🍌']);
        expect(selected(table())).toEqual([]);
    },
};

export const IndependentNestedTables: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await ready(canvasElement);
        const viewer = card(canvasElement, '3. Nested tables keep their own state');
        const tables = () => Array.from(viewer.querySelectorAll('table'));
        expect(values(tables()[1])).toEqual(['red', 'sweet']);
        expect(values(tables()[2])).toEqual(['yellow', 'tart']);
        await userEvent.click(rows(tables()[1])[0].querySelector('input') as HTMLInputElement);
        await waitFor(() => expect(tables()[1].querySelector('caption output')).toHaveTextContent(/^1$/), { timeout: 10000 });
        expect(selected(tables()[1])).toEqual(['red']);
        await userEvent.click(within(tables()[1]).getByRole('button', { name: 'Sort #text descending in tags/tag' }));
        await waitFor(() => expect(values(tables()[1])).toEqual(['sweet', 'red']), { timeout: 10000 });
        expect(values(tables()[2])).toEqual(['yellow', 'tart']);
        expect(selected(tables()[1])).toEqual(['red']);
        expect(selected(tables()[2])).toEqual([]);
        expect(rows(tables()[0]).map(row => row.getAttribute('aria-selected'))).toEqual(['false', 'false']);
        const disclosure = tables()[1].closest('details') as HTMLDetailsElement;
        await userEvent.click(disclosure.querySelector('summary') as HTMLElement);
        expect(disclosure.open).toBe(false);
        expect(selected(tables()[1])).toEqual(['red']);
    },
};
