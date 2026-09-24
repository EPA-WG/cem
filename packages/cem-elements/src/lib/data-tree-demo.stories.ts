import { verifyExternalFilePreviews } from '../../.storybook/external-file-previews.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
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

export const EditingSelectionAndDisclosure: Story = {
    render: () => {
        const root = renderDocument();
        startTreeProcessingTiming(root);
        return root;
    },
    play: async ({ canvasElement }) => {
        const mark = storyTiming('tree/EditingSelectionAndDisclosure');
        mark('setup:start');
        await waitFor(() => expect(canvasElement.querySelectorAll('cem-data-tree textarea')).toHaveLength(3), { timeout: 30000 });
        mark('setup:verified');
        const xml = card(canvasElement, '1. XML branches: independent selection');
        watchTreeSelection(xml);
        const controls = within(xml);
        await waitFor(() => expect(xml.querySelector('pre[aria-label="CEM-ML document"]')).toHaveTextContent('{ast'), { timeout: 10000 });
        mark('initial-document:verified');
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
        expect(xml.querySelectorAll('strong')).toHaveLength(2);
        expect(xml).toHaveTextContent('urn:fruit');
        expect(xml).toHaveTextContent('""');
        expect(xml.querySelector('raw')).toBeNull();
        expect(getComputedStyle(xml.querySelector('pre code') as HTMLElement).whiteSpace).toBe('pre-wrap');
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
        expect(xml.querySelectorAll('input:checked')).toHaveLength(0);
        mark('complete');
    },
};

export const MalformedSourceRecovery: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelectorAll('cem-data-tree textarea')).toHaveLength(3), { timeout: 30000 });
        const failed = card(canvasElement, '3. Malformed source and repair');
        const controls = within(failed);
        await waitFor(() => expect(controls.getByRole('alert')).toHaveTextContent('could not be imported'), { timeout: 10000 });
        expect(failed.querySelector('pre')).toBeNull();
        const source = controls.getByRole('textbox', { name: 'Source' }) as HTMLTextAreaElement;
        source.focus();
        replaceSource(source, '<orchard><fruit>🍒</fruit></orchard>');
        await waitFor(() => expect(failed.querySelector('pre')).toHaveTextContent('{ast'), { timeout: 10000 });
        expect(controls.queryByRole('alert')).toBeNull();
        expect(document.activeElement).toBe(source);
        controls.getByRole('checkbox', { name: 'Select branch 1.1: fruit' }).click();
        await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^1$/), { timeout: 10000 });
        controls.getByRole('button', { name: 'Reload original' }).click();
        await waitFor(() => expect(controls.getByRole('alert')).toBeVisible(), { timeout: 10000 });
        expect(failed.querySelector('pre')).toBeNull();
        expect(failed.querySelector('input')).toBeNull();
        replaceSource(source, '<orchard><fruit>🍒</fruit></orchard>');
        await waitFor(() => expect(controls.getByRole('status', { name: 'Selected branches' })).toHaveTextContent(/^0$/), { timeout: 10000 });
        expect(failed.querySelectorAll('input:checked')).toHaveLength(0);
    },
};

export const SourceRelativeRequests: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await waitFor(() => expect(canvasElement.querySelector('select[aria-label="Local source"]')).not.toBeNull(), { timeout: 30000 });
        const loaded = card(canvasElement, '4. Load and release a local document');
        const controls = within(loaded);
        const select = controls.getByRole('combobox', { name: 'Local source' }) as HTMLSelectElement;
        await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('xml-stylesheet'), { timeout: 20000 });
        expect(select.value).toBe('./tree-source.xml');
        expect(loaded.querySelector('a')?.href).toBe(new URL('./tree-source.xml', SOURCE_URL).href);
        expect(loaded.querySelector('a')).toHaveAttribute('download', '');
        expect(loaded.querySelector('pre')).toHaveTextContent('🍒');
        controls.getAllByRole('checkbox')[0].click();
        await waitFor(() => expect(loaded.querySelectorAll('input:checked')).toHaveLength(1), { timeout: 10000 });
        select.value = './tree-source.json';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('tree-source.json'), { timeout: 20000 });
        expect(select.value).toBe('./tree-source.json');
        await waitFor(() => expect(loaded.querySelector('pre')).not.toHaveTextContent('xml-stylesheet'), { timeout: 10000 });
        expect(loaded.querySelectorAll('input:checked')).toHaveLength(0);
        select.value = '';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => expect(loaded.querySelector('pre')).toBeNull(), { timeout: 10000 });
        expect(controls.getByRole('status', { name: 'Request state' })).toHaveTextContent('idle');
        select.value = './tree-source.xml';
        select.dispatchEvent(new Event('change', { bubbles: true }));
        await waitFor(() => expect(loaded.querySelector('pre')).toHaveTextContent('xml-stylesheet'), { timeout: 20000 });
        expect(loaded.querySelectorAll('input:checked')).toHaveLength(0);
        expect(loaded.querySelector('script')).toBeNull();
    },
};

export const ExternalSourcePreviews: Story = {
    render: renderDocument,
    play: async ({ canvasElement }) => {
        await verifyExternalFilePreviews(canvasElement, SOURCE_URL, ['tree-source.xml', 'tree-source.json']);
    },
};
