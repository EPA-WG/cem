import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { storyTiming } from '../../.storybook/story-timing.js';
import { expect, waitFor, within } from 'storybook/test';
import { cemDiagnosticCodes, traceCemReadiness, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';
import {
    applyPatchFramesToRange, applyRenderPlanToRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan, type RenderPlanNode,
} from './projection.js';

import cemtSource from '../../demo/data-table-view.cemt?raw';
import xsltSource from '../../demo/data-table-view.xslt?raw';
import aspectsSource from '../../demo/data-table-aspects.xslt?raw';

const SOURCE_TAG = 'story-data-table-document';
const SOURCE_URL = new URL('../../demo/data-table.html', import.meta.url);
const meta: Meta = { title: 'CEM Elements/Multi-format Data Tables', tags: ['test'] };
export default meta;
type Story = StoryObj;

function renderDocument(): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', SOURCE_TAG);
    declaration.setAttribute('src', SOURCE_URL.href);
    root.append(declaration, document.createElement(SOURCE_TAG));
    traceCemReadiness(root, 'table');
    return root;
}

// null skips a nested cell checked separately through its own table.
type Cells = (string | null)[][];
const jsonCells: Cells = [['10', '🍒', '""'], ['2', '🍋', '∅'], ['3', '🍌', 'null']];

async function expectTable(viewer: HTMLElement, selector: string, cells: Cells, selected: number[] = []): Promise<void> {
    await waitFor(() => {
        const rows = Array.from(viewer.querySelectorAll(`${selector} > tbody > tr`));
        expect(rows).toHaveLength(cells.length);
        for (const [index, row] of rows.entries()) {
            const actual = Array.from(row.querySelectorAll(':scope > td'), cell => cell.textContent?.replace(/\s+/gu, ' ').trim());
            expect(actual).toHaveLength(cells[index].length);
            cells[index].forEach((value, column) => {
                if (value !== null) expect(actual[column]).toBe(value);
            });
            expect(row).toHaveAttribute('aria-selected', String(selected.includes(index)));
            expect(row.querySelector(':scope > th > button')).toHaveAttribute('aria-pressed', String(selected.includes(index)));
        }
    }, { timeout: 10000 });
}

async function verifyTable(viewer: HTMLElement, selector: string, column: string, headings: string[], cells: Cells): Promise<void> {
    const controls = within(viewer);
    const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
    const original = input.value;
    await expectTable(viewer, selector, cells);
    expect(Array.from(viewer.querySelectorAll(`${selector} > thead th`), th => th.textContent)).toEqual(['✓', ...headings]);
    viewer.querySelector<HTMLButtonElement>(`${selector} > tbody > tr:nth-child(2) > th > button`)?.click();
    await whenCemRendered(viewer);
    await expectTable(viewer, selector, cells, [1]);
    await select(controls.getByRole('combobox', { name: 'Sort column' }), column, viewer);
    await expectTable(viewer, selector, cells, [1]);
    await select(controls.getByRole('combobox', { name: 'Compare' }), 'number', viewer);
    await expectTable(viewer, selector, [cells[1], cells[2], cells[0]], [0]);
    await select(controls.getByRole('combobox', { name: 'Direction' }), 'descending', viewer);
    await expectTable(viewer, selector, [cells[0], cells[2], cells[1]], [2]);
    input.value = original.replace('10', '11');
    input.dispatchEvent(new Event('change', { bubbles: true }));
    await whenCemRendered(viewer);
    await expectTable(viewer, selector, [['11', ...cells[0].slice(1)], cells[2], cells[1]]);
    controls.getByRole('button', { name: 'Reset source' }).click();
    await whenCemRendered(viewer);
    await expectTable(viewer, selector, [cells[0], cells[2], cells[1]], [2]);
    expect(input.value).toBe(original);
    await select(controls.getByRole('combobox', { name: 'Sort column' }), '', viewer);
    await expectTable(viewer, selector, cells, [1]);
}

async function verifyAspects(viewer: HTMLElement): Promise<void> {
    const controls = within(viewer);
    const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
    const original = source.value;
    const visits: Cells = [['192.0.2.1', '10'], ['192.0.2.2', '2']];
    await expectTable(viewer, 'table[aria-label="visits"]', visits);
    expect(viewer.querySelector('table[aria-label="notes"]')).toBeNull();
    const notes = Array.from(viewer.querySelectorAll('details')).find(detail =>
        detail.querySelector(':scope > summary')?.textContent?.includes('🌳 notes'));
    expect(notes).toHaveTextContent('🍒 ready');
    expect(notes).toHaveTextContent('🍋 review');
    expect(viewer.querySelector('form output')?.textContent).toBe('allow: 192.0.2.0/24');
    const address = controls.getByRole('textbox', { name: 'Address / CIDR' }) as HTMLInputElement;
    expect(address.value).toBe('192.0.2.0/24');
    address.value = '198.51.100.0/24';
    address.dispatchEvent(new Event('input', { bubbles: true }));
    await whenCemRendered(viewer);
    await select(controls.getByRole('combobox', { name: 'Action' }), 'deny', viewer);
    await waitFor(() => expect(viewer.querySelector('form output')?.textContent).toBe('deny: 198.51.100.0/24'));
    controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
    await whenCemRendered(viewer);
    await expectTable(viewer, 'table[aria-label="notes"]', [['🍒 ready'], ['🍋 review']]);
    expect(viewer.querySelector('form')).toBeNull();
    await expectTable(viewer, 'table[aria-label="visits"]', visits);
    controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
    await whenCemRendered(viewer);
    await waitFor(() => expect(viewer.querySelector('form output')?.textContent).toBe('deny: 198.51.100.0/24'));
    const restored = controls.getByRole('textbox', { name: 'Address / CIDR' }) as HTMLInputElement;
    expect(restored.value).toBe('198.51.100.0/24');
    restored.value = '';
    restored.dispatchEvent(new Event('input', { bubbles: true }));
    await whenCemRendered(viewer);
    await waitFor(() => expect(viewer.querySelector('form output')?.textContent).toBe('deny: '));
    expect(restored.value).toBe('');
    expect(source.value).toBe(original);
    expect(viewer.querySelector('table[aria-label="notes"]')).toBeNull();
    await expectTable(viewer, 'table[aria-label="visits"]', visits);
}

export const EveryAuthoredSample: Story = {
    render: renderDocument,
    play: async ({ canvasElement, step }) => {
        const mark = storyTiming('table/EveryAuthoredSample');
        await whenCemSourceRendered(canvasElement.querySelector(SOURCE_TAG) as HTMLElement);
        const samples = Array.from(canvasElement.querySelectorAll<HTMLElement>('cem-demo-element'));
        expect(samples).toHaveLength(10);
        for (const [index, sample] of samples.entries()) {
            await step(sample.getAttribute('legend') ?? '', async () => {
                if (index >= 7) {
                    const expected = [cemtSource, xsltSource, aspectsSource][index - 7];
                    await waitFor(() => expect(sample.querySelector('[slot=text] code')?.textContent).toBe(expected));
                    expect(sample).toHaveAttribute('data-state', 'ready');
                    expect(sample.querySelector('[slot=demo]')?.innerHTML).toBe('');
                    return;
                }
                const viewer = sample.querySelector('article')?.parentElement;
                if (!viewer) throw new Error('Table viewer missing');
                if (index === 4 || index === 6) {
                    await verifyAspects(viewer);
                } else {
                    const cells: Cells = index === 0
                        ? [['10', 'caterpie', '∅', '∅'], ['2', 'ivysaur', '🌱', '∅'], ['3', 'venusaur', '""', null]]
                        : index === 1 ? [['10', '🍒', 'sweet, red'], ['2', '🍋', 'say "zest"'], ['3', '🍌', '""']]
                        : index === 2 ? [['10', '🍒', null, '∅'], ['2', '🍋', '∅', 'true'], ['3', '🍌', '∅', 'false']]
                        : jsonCells;
                    const table = index === 0 ? 'table[aria-label$="/row"]' : 'table[aria-label="document"]';
                    const headings = index === 0 ? ['@id', '#text', '@mood', 'evolutions']
                        : index === 2 ? ['qty', 'fruit', 'tags', 'fresh'] : ['qty', 'fruit', 'note'];
                    if (index === 0) {
                        await expectTable(viewer, 'table[aria-label$="/name"]', [['🌿'], ['🌸']]);
                    }
                    if (index === 2) {
                        await expectTable(viewer, 'table[aria-label="tags"]', [['red'], ['sweet']]);
                    }
                    expect(viewer.querySelectorAll('table')).toHaveLength(index === 0 || index === 2 ? 2 : 1);
                    await verifyTable(viewer, table, index === 0 ? '@id' : 'qty', headings, cells);
                    if (index === 3 || index === 5) {
                        const input = viewer.querySelector('textarea') as HTMLTextAreaElement;
                        const original = input.value;
                        input.value = '[oops]';
                        input.dispatchEvent(new Event('change', { bubbles: true }));
                        await whenCemRendered(viewer);
                        await waitFor(() => expect(viewer.querySelector('[role=alert]')).toHaveTextContent('⚠'));
                        expect(viewer.querySelector('table')).toBeNull();
                        within(viewer).getByRole('button', { name: 'Reset source' }).click();
                        await whenCemRendered(viewer);
                        await expectTable(viewer, table, cells, [1]);
                        expect(input.value).toBe(original);
                        expect(viewer.querySelector('[role=alert]')).toBeNull();
                    }
                }
                expect(cemDiagnosticCodes(viewer)).toEqual([]);
            });
        }
        mark('complete');
    },
};

export const XmlNamespacesAndInertText: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await whenCemSourceRendered(canvasElement.querySelector(SOURCE_TAG) as HTMLElement);
        const xml = canvasElement.querySelector('cem-data-table[format="xml"]') as HTMLElement;
        const controls = within(xml);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        input.value = '<r xmlns="urn:root" xmlns:p="urn:rows"><p:row xmlns:q="urn:field" id="1" q:xmlns="first"/><p:row xmlns:q="urn:field" id="2" q:xmlns="second"/><single xmlns:s="urn:detail" s:xmlns="detail"><?keep inert?></single></r>';
        input.dispatchEvent(new Event('change', { bubbles: true }));
        await whenCemRendered(xml);
        await waitFor(() => expect(controls.getByRole('columnheader', { name: '@urn:field|xmlns' })).toBeVisible());
        expect(xml.querySelector('table')).toHaveTextContent('first');
        expect(xml.querySelector('table')).toHaveTextContent('second');
        expect(xml.querySelector('details')).toHaveTextContent('@xmlns: detail');
        const headings = controls.getAllByRole('columnheader').map(heading => heading.textContent);
        expect(headings.some(heading => heading?.includes('http://www.w3.org/2000/xmlns/'))).toBe(false);
        for (const prefix of ['p', 'q', 's']) expect(xml.querySelector('details')).not.toHaveTextContent(`@${prefix}:`);
        expect(xml.querySelector('details')).toHaveTextContent('keep inert');
    },
};

export const MatchingPresentationAspects: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('cem-aspect-view form')).not.toBeNull(), { timeout: 30000 });
        const viewer = canvasElement.querySelector('cem-aspect-view') as HTMLElement;
        const controls = within(viewer);
        const original = (controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement).value;
        expect(viewer.querySelector('table[aria-label="visits"]')).not.toBeNull();
        expect(viewer.querySelector('table[aria-label="notes"]')).toBeNull();
        const address = controls.getByRole('textbox', { name: 'Address / CIDR' }) as HTMLInputElement;
        address.value = '198.51.100.0/24';
        address.dispatchEvent(new Event('input', { bubbles: true }));
        await whenCemRendered(viewer);
        await select(controls.getByRole('combobox', { name: 'Action' }), 'deny', viewer);
        await waitFor(() => expect(viewer.querySelector('form output')).toHaveTextContent('deny: 198.51.100.0/24'));
        controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('table[aria-label="notes"]')).not.toBeNull(), { timeout: 10000 });
        expect(viewer.querySelector('form')).toBeNull();
        controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('form output')).toHaveTextContent('deny: 198.51.100.0/24'), { timeout: 10000 });
        expect((controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement).value).toBe(original);

        // A second instance consumes the same authored source, not the first instance's draft.
        const second = document.createElement('cem-aspect-view');
        second.setAttribute('format', 'json');
        second.textContent = original;
        viewer.parentElement?.append(second);
        await whenCemRendered(second);
        await waitFor(() => expect(second.querySelector('form output')).toHaveTextContent('allow: 192.0.2.0/24'));
        expect(viewer.querySelector('form output')).toHaveTextContent('deny: 198.51.100.0/24');
        second.remove();
    },
};

async function select(element: HTMLElement, value: string, viewer: HTMLElement): Promise<void> {
    const control = element as HTMLSelectElement;
    control.value = value;
    control.dispatchEvent(new Event('change', { bubbles: true }));
    await whenCemRendered(viewer);
    await waitFor(() => expect(control).toHaveValue(value), { timeout: 10000 });
}

export const TextPatchIdentityAndDirtyTextarea: Story = {
    render: () => document.createElement('section'),
    play: ({ canvasElement }) => {
        const start = document.createComment('cem-render-start');
        const end = document.createComment('cem-render-end');
        canvasElement.append(start, end);
        const bounds = { start, end };
        const text = (id: string, value: string): RenderPlanNode => ({
            kind: 'text', text: value, renderNodeId: id,
            sourceMapRef: { fidelity: 'author-byte-exact', frame: 'cem:shared-expression' },
        });
        const element = (id: string, tagName: string, children: RenderPlanNode[]): RenderPlanNode => ({
            kind: 'element', namespace: 'http://www.w3.org/1999/xhtml', tag: tagName,
            attributes: [], renderNodeId: id, children,
        });
        const plan = (revision: string, source: string, extra: string | null): RenderPlan => ({
            producedTag: 'test-table-patches', instanceId: 'one', dataRevision: revision,
            templateArtifactId: 'table', scopePolicyStamp: 'test', outputTarget: 'light-dom',
            nodes: [element('root', 'article', [
                element('source', 'textarea', [text('source-text', source)]),
                element('first', 'p', [text('first-text', 'first')]),
                ...(extra === null ? [] : [element('extra', 'p', [text('extra-text', extra)])]),
            ])],
        });
        const first = plan('1', 'original', null);
        applyRenderPlanToRange(bounds, first, document);
        const textarea = canvasElement.querySelector('textarea') as HTMLTextAreaElement;
        textarea.focus();
        textarea.value = 'uncommitted draft';
        textarea.setSelectionRange(2, 5);
        const inserted = plan('2', 'original', 'inserted');
        const patch = (before: RenderPlan, after: RenderPlan): void => {
            const result = applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, after),
                renderPlanIdentity(after), document);
            expect(result.status).toBe('applied');
        };
        patch(first, inserted);
        expect(textarea.value).toBe('uncommitted draft');
        expect(document.activeElement).toBe(textarea);
        expect(textarea.selectionStart).toBe(2);
        // The inserted text arrived through serialized patch ingress; its next patch
        // must address that occurrence, not the sibling sharing its source frame.
        const changed = plan('3', 'original', 'changed');
        patch(inserted, changed);
        expect(Array.from(canvasElement.querySelectorAll('p'), (p) => p.textContent)).toEqual(['first', 'changed']);
        expect(textarea.value).toBe('uncommitted draft');
        const reset = plan('4', 'restored', 'changed');
        patch(changed, reset);
        expect(canvasElement.querySelector('textarea')).toBe(textarea);
        expect(textarea.value).toBe('restored');
        expect(document.activeElement).toBe(textarea);
    },
};

export const NativeXsltViewer: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('cem-demo-element[legend="6. XSLT table: native sorting and selection"] textarea')).toBeTruthy(), { timeout: 30000 });
        const sample = canvasElement.querySelector('cem-demo-element[legend="6. XSLT table: native sorting and selection"]');
        const viewer = sample?.querySelector('article')?.parentElement;
        if (!viewer) throw new Error('XSLT viewer missing');
        const controls = within(viewer);
        const input = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = input.value;
        const quantities = () => Array.from(viewer.querySelectorAll('table:first-of-type > tbody > tr'),
            row => row.querySelector(':scope > td')?.textContent?.trim());
        for (const [format, source] of [
            ['xml', '<r><row qty="10"/><row qty="2"/><row qty="3"/></r>'],
            ['csv', 'qty,fruit\n10,🍒\n2,🍋\n3,🍌'],
            ['yaml', '- qty: 10\n- qty: 2\n- qty: 3'],
            ['json', original],
        ]) {
            viewer.setAttribute('format', format);
            input.value = source;
            input.dispatchEvent(new Event('change', { bubbles: true }));
            await whenCemRendered(viewer);
            await waitFor(() => expect(viewer.querySelector('label')).toHaveTextContent(`Source (${format.toUpperCase()})`));
            await waitFor(() => expect(quantities()).toEqual(['10', '2', '3']), { timeout: 10000 });
            expect(viewer.querySelector('[role="alert"]')).toBeNull();
        }
        viewer.querySelectorAll<HTMLButtonElement>('tbody > tr > th > button')[1].click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('[aria-selected="true"]')).toHaveTextContent('🍋'));
        await select(controls.getByRole('combobox', { name: 'Sort column' }), 'qty', viewer);
        await select(controls.getByRole('combobox', { name: 'Compare' }), 'number', viewer);
        await waitFor(() => expect(quantities()).toEqual(['2', '3', '10']));
        expect(viewer.querySelector('tbody tr:first-child')).toHaveAttribute('aria-selected', 'true');
        await select(controls.getByRole('combobox', { name: 'Direction' }), 'descending', viewer);
        await waitFor(() => expect(quantities()).toEqual(['10', '3', '2']));
        input.value = original.replace('10', '11');
        input.dispatchEvent(new Event('change', { bubbles: true }));
        await whenCemRendered(viewer);
        await waitFor(() => expect(quantities()).toEqual(['11', '3', '2']));
        expect(viewer.querySelector('[aria-selected="true"]')).toBeNull();
        input.focus();
        input.value = '[oops]';
        input.dispatchEvent(new Event('change', { bubbles: true }));
        await whenCemRendered(viewer);
        await waitFor(() => expect(controls.getByRole('alert')).toHaveTextContent('⚠'));
        expect(viewer.querySelector('table')).toBeNull();
        expect(document.activeElement).toBe(input);
        controls.getByRole('button', { name: 'Reset source' }).click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('table')).toBeTruthy());
        expect(input.value).toBe(original);
        // Restoring identical source bytes restores the same provenance key.
        expect(viewer.querySelector('[aria-selected="true"]')).toHaveTextContent('🍋');
        expect(canvasElement.querySelector('cem-data-table[format="json"] textarea')).toHaveValue(original);
    },
};

export const ImportedXsltPresentationAspects: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('cem-demo-element[legend="7. XSLT aspects: tree and IP-filter form"] form')).toBeTruthy(), { timeout: 30000 });
        const sample = canvasElement.querySelector('cem-demo-element[legend="7. XSLT aspects: tree and IP-filter form"]');
        const viewer = sample?.querySelector('article')?.parentElement;
        if (!viewer) throw new Error('XSLT aspect viewer missing');
        const controls = within(viewer);
        const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        const original = source.value;
        expect(viewer.querySelector('table[aria-label="notes"]')).toBeNull();
        expect(viewer.querySelector('table[aria-label="visits"]')).toBeTruthy();
        const address = controls.getByRole('textbox', { name: 'Address / CIDR' }) as HTMLInputElement;
        address.focus();
        address.value = '198.51.100.0/24';
        address.setSelectionRange(3, 3);
        address.dispatchEvent(new Event('input', { bubbles: true }));
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('form output')).toHaveTextContent('198.51.100.0/24'));
        expect(document.activeElement).toBe(address);
        expect(address.selectionStart).toBe(3);
        await select(controls.getByRole('combobox', { name: 'Action' }), 'deny', viewer);
        controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('table[aria-label="notes"]')).toBeTruthy());
        expect(viewer.querySelector('form')).toBeNull();
        controls.getByRole('checkbox', { name: 'Presentation aspects' }).click();
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('form output')).toHaveTextContent('deny: 198.51.100.0/24'));
        const restored = controls.getByRole('textbox', { name: 'Address / CIDR' }) as HTMLInputElement;
        restored.value = '';
        restored.dispatchEvent(new Event('input', { bubbles: true }));
        await whenCemRendered(viewer);
        await waitFor(() => expect(viewer.querySelector('form output')?.textContent).toBe('deny: '));
        expect(source.value).toBe(original);
        expect(canvasElement.querySelector('cem-aspect-view form output')).toHaveTextContent('allow: 192.0.2.0/24');
    },
};
