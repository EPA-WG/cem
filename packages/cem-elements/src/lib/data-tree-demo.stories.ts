import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { cemDiagnosticCodes, whenCemRendered, whenCemSourceRendered } from '../../.storybook/preview.js';
import { storyTiming } from '../../.storybook/story-timing.js';
import { startTreeProcessingTiming, watchTreeSelection } from '../../.storybook/tree-processing-timing.js';
import { expect, userEvent, waitFor, within } from 'storybook/test';
import {
    applyPatchFramesToRange, applyRenderPlanToRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan, type RenderPlanNode,
} from './projection.js';

const meta: Meta = { title: 'CEM Elements/Retained Data Trees', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const DirtyCheckboxPatches: Story = {
    render: () => document.createElement('section'),
    play: ({ canvasElement }) => {
        const start = document.createComment('cem-render-start');
        const end = document.createComment('cem-render-end');
        canvasElement.append(start, end);
        const bounds = { start, end };
        const input = (id: string, checked: boolean): RenderPlanNode => ({
            kind: 'element', namespace: 'http://www.w3.org/1999/xhtml', tag: 'input',
            attributes: [{ name: 'type', value: 'checkbox' }, { name: 'aria-label', value: id },
                ...(checked ? [{ name: 'checked', value: '' }] : [])],
            renderNodeId: id, children: [],
        });
        const plan = (revision: string, checked: boolean): RenderPlan => ({
            producedTag: 'test-tree-patches', instanceId: 'one', dataRevision: revision,
            templateArtifactId: 'tree', scopePolicyStamp: 'test', outputTarget: 'light-dom',
            nodes: [input('Branch', checked), input('Uncontrolled', false)],
        });
        const initial = plan('1', true);
        applyRenderPlanToRange(bounds, initial, document);
        const [branch, unrelated] = canvasElement.querySelectorAll('input');
        branch.click();
        branch.click(); // Dirty, checked, with an authored checked attribute.
        unrelated.click();
        branch.focus();
        const patch = (before: RenderPlan, after: RenderPlan): void => {
            const result = applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, after),
                renderPlanIdentity(after), document);
            expect(result.status).toBe('applied');
        };
        const cleared = plan('2', false);
        patch(initial, cleared);
        expect(branch).not.toBeChecked();
        expect(unrelated).toBeChecked();
        expect(document.activeElement).toBe(branch);
        expect(canvasElement.querySelector('input')).toBe(branch);
        const restored = plan('3', true);
        patch(cleared, restored);
        expect(branch).toBeChecked();
        branch.click();
        patch(restored, plan('4', true));
        expect(branch).not.toBeChecked(); // No authored change: retain the user's edit.
        expect(unrelated).toBeChecked();
    },
};

const SOURCE_URL = new URL('../../demo/data-tree.html', import.meta.url);
function renderDocument(): HTMLElement {
    const root = document.createElement('section');
    const declaration = document.createElement('cem-element');
    declaration.hidden = true;
    declaration.setAttribute('tag', 'story-data-tree-document');
    declaration.setAttribute('src', SOURCE_URL.href);
    root.append(declaration, document.createElement('story-data-tree-document'));
    return root;
}
function card(root: HTMLElement, legend: string): HTMLElement {
    const found = root.querySelector<HTMLElement>(`cem-demo-element[legend="${legend}"]`);
    if (!found) throw new Error(`missing ${legend}`);
    const viewer = found.querySelector('article')?.parentElement;
    if (!viewer) throw new Error(`viewer missing in ${legend}`);
    return viewer;
}
function replaceSource(input: HTMLTextAreaElement, source: string): void {
    input.value = source;
    input.dispatchEvent(new Event('change', { bubbles: true }));
}

function branch(viewer: HTMLElement, label: string): HTMLElement {
    const found = viewer.querySelector<HTMLElement>(`li:has(> label > input[aria-label="Select branch ${label}"])`);
    if (!found) throw new Error(`Missing branch ${label}`);
    return found;
}

async function expectSelection(viewer: HTMLElement, selected: string[]): Promise<void> {
    await waitFor(() => {
        expect(Array.from(viewer.querySelectorAll('input:checked'), input => input.getAttribute('aria-label')))
            .toEqual(selected.map(label => `Select branch ${label}`));
        expect(viewer.querySelector('output[aria-label="Selected branches"]')?.textContent).toBe(String(selected.length));
        expect(viewer.querySelectorAll('strong')).toHaveLength(selected.length);
        for (const input of viewer.querySelectorAll<HTMLInputElement>('input[type=checkbox]')) {
            const li = input.closest('li');
            expect(li?.classList.contains('selected')).toBe(input.checked);
            expect(li?.querySelector(':scope > label > strong')?.textContent ?? '').toBe(input.checked ? 'Selected' : '');
        }
    }, { timeout: 10000 });
}

async function verifyJsonTree(canvasElement: HTMLElement): Promise<void> {
    const json = card(canvasElement, '2. JSON through the same CEM tree');
    const xml = card(canvasElement, '1. XML branches: independent selection');
    const controls = within(json);
    const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
    const original = source.value;
    expect(Array.from(json.querySelectorAll('input'), input => input.getAttribute('aria-label'))).toEqual([
        'Select branch 1: object', 'Select branch 1.1: property', 'Select branch 1.1.1: string',
        'Select branch 1.2: property', 'Select branch 1.2.1: string', 'Select branch 1.3: property',
        'Select branch 1.3.1: array', 'Select branch 1.3.1.1: string', 'Select branch 1.3.1.2: string',
    ]);
    expect(json.querySelector('pre')).toHaveTextContent('{ast');
    expect(branch(json, '1.1: property')).toHaveTextContent('fruit');
    expect(branch(json, '1.1.1: string')).toHaveTextContent('🍒');
    expect(branch(json, '1.2: property')).toHaveTextContent('note');
    expect(branch(json, '1.2.1: string')).toHaveTextContent('""');
    expect(branch(json, '1.3.1.1: string')).toHaveTextContent('red');
    expect(branch(json, '1.3.1.2: string')).toHaveTextContent('sweet');
    await expectSelection(json, []);
    await userEvent.click(controls.getByRole('checkbox', { name: 'Select branch 1.3: property' }));
    await expectSelection(json, ['1.3: property']);
    await userEvent.click(controls.getByRole('checkbox', { name: 'Select branch 1.3.1.2: string' }));
    await expectSelection(json, ['1.3: property', '1.3.1.2: string']);
    const disclosure = () => branch(json, '1.3: property').querySelector('details') as HTMLDetailsElement;
    const retainedDisclosure = disclosure();
    const retainedSummary = retainedDisclosure.querySelector('summary') as HTMLElement;
    const retainedChild = branch(json, '1.3.1.2: string').querySelector('input');
    await userEvent.click(retainedSummary);
    expect(disclosure().open).toBe(false);
    await expectSelection(json, ['1.3: property', '1.3.1.2: string']);
    await userEvent.click(controls.getByRole('checkbox', { name: 'Select branch 1.3: property' }));
    await expectSelection(json, ['1.3.1.2: string']);
    expect(disclosure()).toBe(retainedDisclosure);
    expect(retainedDisclosure.isConnected).toBe(true);
    expect(disclosure().open).toBe(false);
    expect(disclosure().querySelector('summary')).toBe(retainedSummary);
    expect(branch(json, '1.3.1.2: string').querySelector('input')).toBe(retainedChild);
    await userEvent.click(disclosure().querySelector('summary') as HTMLElement);
    expect(disclosure().open).toBe(true);
    await expectSelection(xml, []);
    // Dispatching unchanged bytes still starts a new source revision.
    replaceSource(source, original);
    await whenCemRendered(json);
    await expectSelection(json, []);
    await userEvent.click(controls.getByRole('checkbox', { name: 'Select branch 1.2.1: string' }));
    await expectSelection(json, ['1.2.1: string']);
    controls.getByRole('button', { name: 'Reload original' }).click();
    await whenCemRendered(json);
    await expectSelection(json, []);
    expect(source.value).toBe(original);
    expect(cemDiagnosticCodes(json)).toEqual([]);
}

export const EditingSelectionAndDisclosure: Story = {
    render: () => {
        const root = renderDocument();
        startTreeProcessingTiming(root);
        return root;
    },
    play: async ({ canvasElement, step }) => {
        const mark = storyTiming('tree/EditingSelectionAndDisclosure');
        mark('setup:start');
        await whenCemSourceRendered(canvasElement.querySelector('story-data-tree-document') as HTMLElement);
        mark('setup:verified');
        expect(canvasElement.querySelectorAll('cem-demo-element')).toHaveLength(6);
        await step('1. XML branches: independent selection', async () => {
            const xml = card(canvasElement, '1. XML branches: independent selection');
            watchTreeSelection(xml);
            const controls = within(xml);
            await waitFor(() => expect(xml.querySelector('pre[aria-label="CEM-ML document"]')).toHaveTextContent('{ast'), { timeout: 10000 });
            mark('initial-document:verified');
            expect(Array.from(xml.querySelectorAll('input'), input => input.getAttribute('aria-label'))).toEqual([
                'Select branch 1: orchard', 'Select branch 1.1: fruit', 'Select branch 1.2: fruit',
            ]);
            await expectSelection(xml, []);
            const firstBranch = branch(xml, '1.1: fruit');
            const secondBranch = branch(xml, '1.2: fruit');
            expect(firstBranch.querySelector('summary')).toHaveTextContent('fruit · urn:fruit');
            expect(firstBranch.querySelector('ul .value')?.textContent).toBe('""');
            expect(firstBranch.querySelector('ul')).toHaveTextContent('@color');
            expect(xml.querySelector('article > section:nth-of-type(2)')).not.toHaveTextContent('@xmlns');
            expect(Array.from(secondBranch.querySelectorAll(':scope > details > ol > li'), li =>
                li.textContent?.replace(/\s+/gu, ' ').trim())).toEqual([
                'text : pre', 'cdata : <raw>🍋', 'processing-instruction · keep : inert', 'text : post',
            ]);
            expect(xml.querySelector('pre')).toHaveTextContent('@kind=cdata');
            expect(xml.querySelector('pre')).toHaveTextContent('@target=keep');
            let first = controls.getByRole('checkbox', { name: 'Select branch 1.1: fruit' });
            expect(first).not.toBeChecked();
            mark('select-first:start');
            await userEvent.click(first);
            mark('select-first:dispatched', xml);
            await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^1$/), { timeout: 10000 });
            mark('select-first:verified');
            mark('select-second:start');
            await userEvent.click(controls.getByRole('checkbox', { name: 'Select branch 1.2: fruit' }));
            mark('select-second:dispatched', xml);
            await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^2$/), { timeout: 10000 });
            mark('select-second:verified');
            await expectSelection(xml, ['1.1: fruit', '1.2: fruit']);
            first = controls.getByRole('checkbox', { name: 'Select branch 1.1: fruit' });
            const second = controls.getByRole('checkbox', { name: 'Select branch 1.2: fruit' });
            const disclosure = first.closest('li')?.querySelector('details');
            const summary = disclosure?.querySelector('summary');
            if (!disclosure || !summary) throw new Error('branch disclosure missing');
            mark('close-disclosure:start');
            await userEvent.click(summary);
            mark('close-disclosure:dispatched', xml);
            await waitFor(() => expect(disclosure.open).toBe(false), { timeout: 10000 });
            mark('close-disclosure:verified');
            expect(first).toBeChecked();
            expect(second).toBeChecked();
            expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^2$/);
            mark('open-disclosure:start');
            await userEvent.click(summary);
            mark('open-disclosure:dispatched', xml);
            await waitFor(() => expect(disclosure.open).toBe(true), { timeout: 10000 });
            mark('open-disclosure:verified');
            await expectSelection(xml, ['1.1: fruit', '1.2: fruit']);
            expect(xml.querySelectorAll('strong')).toHaveLength(2);
            expect(xml).toHaveTextContent('urn:fruit');
            expect(xml).toHaveTextContent('""');
            expect(xml.querySelector('raw')).toBeNull();
            expect(getComputedStyle(xml.querySelector('pre code') as HTMLElement).whiteSpace).toBe('pre-wrap');
            await userEvent.click(first);
            await expectSelection(xml, ['1.2: fruit']);
            const json = card(canvasElement, '2. JSON through the same CEM tree');
            expect(json.querySelectorAll('input:checked')).toHaveLength(0);
            const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
            const original = source.value;
            mark('reset-selections:start');
            controls.getByRole('button', { name: 'Reload original' }).click();
            mark('reset-selections:dispatched', xml);
            await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^0$/), { timeout: 10000 });
            mark('reset-selections:verified');
            expect(source.value).toBe(original);
            expect(xml.querySelectorAll('input:checked')).toHaveLength(0);
            mark('reselect-first:start');
            (controls.getByRole('checkbox', { name: 'Select branch 1.1: fruit' }) as HTMLElement).click();
            mark('reselect-first:dispatched', xml);
            await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^1$/), { timeout: 10000 });
            mark('reselect-first:verified');
            source.focus();
            mark('replace-source:start');
            replaceSource(source, '<r><script>neverRun()</script><fruit>🍏</fruit></r>');
            mark('replace-source:dispatched', xml);
            await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^0$/), { timeout: 10000 });
            mark('replace-source:verified');
            expect(xml.querySelectorAll('input:checked')).toHaveLength(0);
            expect(xml.querySelector('script')).toBeNull();
            expect(document.activeElement).toBe(source);
            mark('restore-source:start');
            controls.getByRole('button', { name: 'Reload original' }).click();
            mark('restore-source:dispatched', xml);
            await waitFor(() => expect(source.value).toBe(original), { timeout: 10000 });
            mark('restore-source:verified');
            await expectSelection(xml, []);
            expect(cemDiagnosticCodes(xml)).toEqual([]);
        });
        await step('2. JSON through the same CEM tree', async () => verifyJsonTree(canvasElement));
        await step('3. Malformed source and repair', async () => verifyMalformedSource(canvasElement));
        await step('4. Load and release a local document', async () => verifySourceRequests(canvasElement));
        for (const file of ['tree-source.xml', 'tree-source.json']) {
            await step(file, async () => verifyExternalFilePreviews(canvasElement, SOURCE_URL, [file]));
        }
        mark('complete');
    },
};

async function verifyMalformedSource(canvasElement: HTMLElement): Promise<void> {
    await whenCemSourceRendered(canvasElement.querySelector('story-data-tree-document') as HTMLElement);
    const failed = card(canvasElement, '3. Malformed source and repair');
    const controls = within(failed);
    await waitFor(() => expect(controls.getByRole('alert')).toHaveTextContent('could not be imported'), { timeout: 10000 });
    expect(failed.querySelector('pre')).toBeNull();
    expect(failed.querySelector('input')).toBeNull();
    expect(failed.querySelector('output')).toBeNull();
    const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
    const original = source.value;
    source.focus();
    replaceSource(source, '<orchard><fruit>🍒</fruit></orchard>');
    await waitFor(() => expect(failed.querySelector('pre')).toHaveTextContent('{ast'), { timeout: 10000 });
    expect(controls.queryByRole('alert')).toBeNull();
    expect(document.activeElement).toBe(source);
    await expectSelection(failed, []);
    controls.getByRole('checkbox', { name: 'Select branch 1.1: fruit' }).click();
    await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^1$/), { timeout: 10000 });
    controls.getByRole('button', { name: 'Reload original' }).click();
    await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
    expect(failed.querySelector('pre')).toBeNull();
    expect(failed.querySelector('input')).toBeNull();
    expect(failed.querySelector('output')).toBeNull();
    expect(source.value).toBe(original);
    replaceSource(source, '<orchard><fruit>🍒</fruit></orchard>');
    await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^0$/), { timeout: 10000 });
    await expectSelection(failed, []);
    expect(cemDiagnosticCodes(failed)).toEqual([]);
}

export const MalformedSourceRecovery: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await verifyMalformedSource(canvasElement);
    },
};

async function verifySourceRequests(canvasElement: HTMLElement): Promise<void> {
    await whenCemSourceRendered(canvasElement.querySelector('story-data-tree-document') as HTMLElement);
    const loaded = card(canvasElement, '4. Load and release a local document');
    const controls = within(loaded);
    const select = controls.getByRole('combobox', { name: 'Local source' }) as HTMLSelectElement;
    await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('xml-stylesheet'), { timeout: 20000 });
    expect(select.value).toBe('./tree-source.xml');
    expect(controls.getByRole('status', { name: 'Request state' })).toHaveTextContent(/^loaded$/);
    await expectSelection(loaded, []);
    expect(loaded.querySelector('a')?.href).toBe(new URL('./tree-source.xml', SOURCE_URL).href);
    expect(loaded.querySelector('a')).toHaveAttribute('download', '');
    expect(loaded.querySelector('pre')).toHaveTextContent('🍒');
    controls.getAllByRole('checkbox')[0].click();
    await waitFor(() => expect(loaded.querySelectorAll('input:checked')).toHaveLength(1), { timeout: 10000 });
    select.value = './tree-source.json';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('tree-source.json'), { timeout: 20000 });
    expect(select.value).toBe('./tree-source.json');
    expect(controls.getByRole('status', { name: 'Request state' })).toHaveTextContent(/^loaded$/);
    expect(loaded.querySelector('a')?.href).toBe(new URL('./tree-source.json', SOURCE_URL).href);
    expect(loaded.querySelector('a')).toHaveAttribute('download', '');
    expect(loaded.querySelector('pre')).toHaveTextContent('🍋');
    await waitFor(() => expect(loaded.querySelector('pre')).not.toHaveTextContent('xml-stylesheet'), { timeout: 10000 });
    await expectSelection(loaded, []);
    controls.getAllByRole('checkbox')[0].click();
    await waitFor(() => expect(loaded.querySelectorAll('input:checked')).toHaveLength(1));
    select.value = '';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => expect(loaded.querySelector('pre')).toBeNull(), { timeout: 10000 });
    expect(controls.getByRole('status', { name: 'Request state' })).toHaveTextContent(/^idle$/);
    expect(select.value).toBe('');
    expect(loaded.querySelector('a')).toBeNull();
    expect(loaded.querySelector('input')).toBeNull();
    expect(controls.queryByRole('status', { name: 'Selected branches' })).toBeNull();
    select.value = './tree-source.xml';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('xml-stylesheet'), { timeout: 20000 });
    await expectSelection(loaded, []);
    expect(loaded.querySelector('a')?.href).toBe(new URL('./tree-source.xml', SOURCE_URL).href);
    expect(controls.getByRole('status', { name: 'Request state' })).toHaveTextContent(/^loaded$/);
    expect(loaded.querySelector('script')).toBeNull();
    expect(cemDiagnosticCodes(loaded)).toEqual([]);
}

export const SourceRelativeRequests: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await verifySourceRequests(canvasElement);
    },
};

export const ExternalSourcePreviews: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await verifyExternalFilePreviews(canvasElement, SOURCE_URL, ['tree-source.xml', 'tree-source.json']);
    },
};
