import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { storyTiming } from '../../.storybook/story-timing.js';
import { cemDiagnosticCodes, whenCemSourceRendered } from '../../.storybook/preview.js';
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
    declaration.setAttribute('link-base', 'source');
    root.append(declaration, document.createElement('story-table-inspector-document'));
    return root;
}
async function ready(root: HTMLElement): Promise<void> {
    await waitFor(() => expect(root.querySelectorAll('cem-table-inspector table')).toHaveLength(6), { timeout: 30000 });
    await whenCemSourceRendered(root.querySelector('story-table-inspector-document') as HTMLElement);
    expect(Array.from(root.querySelectorAll<HTMLAnchorElement>('nav a, main > section a'), link => link.href))
        .toEqual(['../index.html', './data-table-view.cemt', './data-table.html', './data-tree.html', './xpath-sort.html']
            .map(relative => new URL(relative, SOURCE_URL).href));
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
    play: async ({ canvasElement, step }) => {
        await ready(canvasElement);
        const columns = card(canvasElement, '1. Columns from every row');
        await step('1. Columns from every row', async () => {
            const table = columns.querySelector('table') as HTMLTableElement;
            expect(headings(table)).toEqual(['✓', '@early', '#text', '@later']);
            expect(matrix(table)).toEqual([['""', '🍒', '∅'], ['∅', '🍋', 'ripe']]);
            assertSelection(table, []);
            const checkbox = rows(table)[0].querySelector('input') as HTMLInputElement;
            checkbox.focus();
            await userEvent.keyboard(' ');
            await waitFor(() => assertSelection(table, [0]));
            await userEvent.keyboard(' ');
            await waitFor(() => assertSelection(table, []));
            expect(columns.querySelector('table')).toBe(table);
            expect(rows(table)[0].querySelector('input')).toBe(checkbox);
            expect(document.activeElement).toBe(checkbox);
        });
        await step('2. Text-only rows stay visible', async () => {
            const text = card(canvasElement, '2. Text-only rows stay visible');
            const table = () => text.querySelector('table') as HTMLTableElement;
            expect(headings(table())).toEqual(['✓', '#text']);
            expect(matrix(table())).toEqual([['ivysaur'], ['venusaur']]);
            const sort = within(text).getByRole('button', { name: 'Sort #text descending in document/name' });
            sort.focus();
            await userEvent.keyboard('{Enter}');
            await order(text, ['venusaur', 'ivysaur']);
            expect(text.querySelector('th[aria-sort]')).toHaveAttribute('aria-sort', 'descending');
            await userEvent.click(within(text).getByRole('button', { name: 'Sort #text ascending in document/name' }));
            await order(text, ['ivysaur', 'venusaur']);
            expect(text.querySelector('th[aria-sort]')).toHaveAttribute('aria-sort', 'ascending');
            await userEvent.click(within(text).getByRole('button', { name: 'Restore source order in document/name' }));
            await waitFor(() => expect(text.querySelector('th[aria-sort]')).toBeNull());
            assertSelection(table(), []);
            expect(columns.querySelector('th[aria-sort]')).toBeNull();
            expect(matrix(columns.querySelector('table') as HTMLTableElement)).toEqual([['""', '🍒', '∅'], ['∅', '🍋', 'ripe']]);
        });
        await assertDiagnostics(canvasElement);
    },
};

export const MultipleSelectionAndRecovery: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        const mark = storyTiming('inspector/MultipleSelectionAndRecovery');
        mark('setup:start');
        await ready(canvasElement);
        mark('setup:verified');
        const viewer = card(canvasElement, '4. Multiple selections survive sorting');
        const controls = within(viewer);
        const table = () => viewer.querySelector('table') as HTMLTableElement;
        expect(headings(table())).toEqual(['✓', '@qty', '#text']);
        expect(matrix(table())).toEqual([['2', 'Cherry 🍒'], ['10', 'Lemon 🍋'], ['02', 'Apple 🍏'], ['∅', 'Banana 🍌']]);
        assertSelection(table(), []);
        mark('select-first:start');
        await userEvent.click(rows(table())[1].querySelector('input') as HTMLInputElement);
        mark('select-first:dispatched', viewer);
        await waitFor(() => expect(table().querySelector('caption output')).toHaveTextContent(/^1$/), { timeout: 10000 });
        mark('select-first:verified');
        mark('select-second:start');
        await userEvent.click(rows(table())[2].querySelector('input') as HTMLInputElement);
        mark('select-second:dispatched', viewer);
        await waitFor(() => expect(table().querySelector('caption output')).toHaveTextContent(/^2$/), { timeout: 10000 });
        mark('select-second:verified');
        const mode = controls.getByRole('combobox', { name: 'Compare document/row' }) as HTMLSelectElement;
        mark('compare-number:start');
        await userEvent.selectOptions(mode, 'number');
        mark('compare-number:dispatched', viewer);
        mark('sort-ascending:start');
        await userEvent.click(controls.getByRole('button', { name: 'Sort @qty ascending in document/row' }));
        mark('sort-ascending:dispatched', viewer);
        await order(viewer, ['Cherry 🍒', 'Apple 🍏', 'Lemon 🍋', 'Banana 🍌']);
        mark('sort-ascending:verified');
        expect(selected(table())).toEqual(['Apple 🍏', 'Lemon 🍋']);
        assertSelection(table(), [1, 2]);
        expect(mode.value).toBe('number');
        expect(table().querySelector('th[aria-sort]')).toHaveAttribute('aria-sort', 'ascending');
        mark('sort-descending:start');
        await userEvent.click(controls.getByRole('button', { name: 'Sort @qty descending in document/row' }));
        mark('sort-descending:dispatched', viewer);
        await order(viewer, ['Lemon 🍋', 'Cherry 🍒', 'Apple 🍏', 'Banana 🍌']);
        mark('sort-descending:verified');
        expect(selected(table())).toEqual(['Lemon 🍋', 'Apple 🍏']);
        assertSelection(table(), [0, 2]);
        expect(table().querySelector('caption output')).toHaveTextContent(/^2$/);
        mark('deselect-first:start');
        await userEvent.click(rows(table())[0].querySelector('input') as HTMLInputElement);
        mark('deselect-first:dispatched', viewer);
        await waitFor(() => expect(selected(table())).toEqual(['Apple 🍏']), { timeout: 10000 });
        mark('deselect-first:verified');
        mark('source-order:start');
        await userEvent.click(controls.getByRole('button', { name: 'Restore source order in document/row' }));
        mark('source-order:dispatched', viewer);
        await order(viewer, ['Cherry 🍒', 'Lemon 🍋', 'Apple 🍏', 'Banana 🍌']);
        mark('source-order:verified');
        expect(selected(table())).toEqual(['Apple 🍏']);
        expect(table().querySelector('th[aria-sort]')).toBeNull();
        expect(card(canvasElement, '1. Columns from every row').querySelectorAll('input:checked')).toHaveLength(0);

        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        const second = document.createElement('cem-table-inspector');
        second.setAttribute('format', 'xml');
        second.setAttribute('inspector', 'true');
        second.textContent = original;
        mark('second-instance:start');
        viewer.parentElement?.append(second);
        mark('second-instance:dispatched', second);
        await waitFor(() => expect(second.querySelector('table')).not.toBeNull(), { timeout: 10000 });
        mark('second-instance:verified');
        expect(selected(second.querySelector('table') as HTMLTableElement)).toEqual([]);
        mark('second-selection:start');
        await userEvent.click(second.querySelector('input') as HTMLInputElement);
        mark('second-selection:dispatched', second);
        await waitFor(() => expect(second.querySelector('caption output')).toHaveTextContent(/^1$/), { timeout: 10000 });
        mark('second-selection:verified');
        expect(selected(table())).toEqual(['Apple 🍏']);
        second.remove();
        mark('reset:start');
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        mark('reset:dispatched', viewer);
        await waitFor(() => expect(selected(table())).toEqual([]), { timeout: 10000 });
        mark('reset:verified');
        expect(input.value).toBe(original);
        mark('invalid-source:start');
        input.focus(); change(input, '<broken>');
        mark('invalid-source:dispatched', viewer);
        await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
        mark('invalid-source:verified');
        expect(viewer.querySelector('table')).toBeNull();
        expect(document.activeElement).toBe(input);
        mark('restore-source:start');
        change(input, original);
        mark('restore-source:dispatched', viewer);
        await order(viewer, ['Cherry 🍒', 'Lemon 🍋', 'Apple 🍏', 'Banana 🍌']);
        mark('restore-source:verified');
        expect(selected(table())).toEqual([]);
        expect(controls.getByRole('textbox', { name: 'Source' })).toBe(input);
        expect(input.value).toBe(original);
        expect(viewer.querySelector('[role="alert"]')).toBeNull();
        // A real source replacement clears selection and updates every cell.
        await userEvent.click(rows(table())[0].querySelector('input') as HTMLInputElement);
        await waitFor(() => assertSelection(table(), [0]));
        await userEvent.clear(input);
        await userEvent.type(input, '<fruit><row qty="3">Pear</row><row qty="1">Peach</row></fruit>');
        await userEvent.tab();
        await order(viewer, ['Pear', 'Peach']);
        expect(matrix(table())).toEqual([['3', 'Pear'], ['1', 'Peach']]);
        assertSelection(table(), []);
        expect(controls.getByRole('textbox', { name: 'Source' })).toBe(input);
        await userEvent.click(controls.getByRole('button', { name: 'Reset source' }));
        await order(viewer, ['Cherry 🍒', 'Lemon 🍋', 'Apple 🍏', 'Banana 🍌']);
        expect(input.value).toBe(original);
        assertSelection(table(), []);
        await assertDiagnostics(canvasElement);
        mark('complete');
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
        const summary = disclosure.querySelector('summary') as HTMLElement;
        await userEvent.click(summary);
        expect(disclosure.open).toBe(false);
        // Trigger a render while the nested table is collapsed, then query its
        // live disclosure again so a replaced, detached node cannot pass.
        await userEvent.click(rows(tables()[0])[0].querySelector('input') as HTMLInputElement);
        await waitFor(() => assertSelection(tables()[0], [0]));
        expect(tables()[1].closest('details')).toBe(disclosure);
        expect(disclosure.isConnected).toBe(true);
        expect(disclosure.open).toBe(false);
        expect(selected(tables()[1])).toEqual(['red']);
        expect(selected(tables()[2])).toEqual([]);
        await userEvent.click(summary);
        expect(disclosure.open).toBe(true);
        expect(disclosure.querySelector('summary')).toBe(summary);
        // Give the second nested table its own sort without changing the first.
        await userEvent.click(within(tables()[2]).getByRole('button', { name: 'Sort #text ascending in tags/tag' }));
        await waitFor(() => expect(values(tables()[2])).toEqual(['tart', 'yellow']));
        expect(values(tables()[1])).toEqual(['sweet', 'red']);
        expect(selected(tables()[1])).toEqual(['red']);
        assertSelection(tables()[0], [0]);
        assertSelection(tables()[1], [1]);
        assertSelection(tables()[2], []);
        expect(tables()[0].querySelector(':scope > thead th[aria-sort]')).toBeNull();
        await assertDiagnostics(canvasElement);
    },
};

function headings(table: HTMLTableElement): string[] {
    return Array.from(table.querySelectorAll(':scope > thead > tr > th'), node =>
        (node.textContent ?? '').replace(/[↑↓]/gu, '').trim());
}

function matrix(table: HTMLTableElement): string[][] {
    return rows(table).map(row => Array.from(row.cells).slice(1).map(cell => cell.textContent?.trim() ?? ''));
}

function assertSelection(table: HTMLTableElement, indices: number[]): void {
    const ownRows = rows(table);
    expect(ownRows.map(row => row.querySelector('input')?.checked)).toEqual(ownRows.map((_, index) => indices.includes(index)));
    expect(ownRows.map(row => row.getAttribute('aria-selected'))).toEqual(ownRows.map((_, index) => String(indices.includes(index))));
    expect(table.querySelector(':scope > caption > output')?.textContent?.trim()).toBe(String(indices.length));
}

async function assertDiagnostics(root: HTMLElement): Promise<void> {
    const host = root.querySelector('story-table-inspector-document') as HTMLElement;
    await whenCemSourceRendered(host);
    expect(cemDiagnosticCodes(host)).toEqual([]);
    for (const declaration of root.querySelectorAll<HTMLElement>('cem-element[tag]')) {
        expect(cemDiagnosticCodes(declaration)).toEqual([]);
        const tag = declaration.getAttribute('tag');
        if (tag) for (const instance of root.querySelectorAll<HTMLElement>(tag)) {
            expect(cemDiagnosticCodes(instance)).toEqual([]);
        }
    }
}
