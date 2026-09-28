import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, within } from 'storybook/test';
import { loadCemDeclaration, whenCemRendered } from '../../.storybook/preview.js';
import { applyRenderPlanToRange, applyPatchFramesToRange, diffRenderPlansToPatchFrames,
    renderPlanIdentity, type RenderPlan } from './projection.js';

export default { title: 'CEM Elements/Custom Element Ownership', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const tag = 'cem-fixture-owned-output';
if (!customElements.get(tag)) customElements.define(tag, class extends HTMLElement {
    connectedCallback() {
        if (this.querySelector('output')) return;
        this.setAttribute('data-state', 'ready');
        const output = document.createElement('output');
        output.textContent = 'Component-owned output';
        this.append(output);
    }
});
const plan = (revision: string, label: string | null): RenderPlan => ({ producedTag: 'ownership-parent', instanceId: 'parent',
    templateArtifactId: 'ownership', dataRevision: revision, outputTarget: 'light-dom', scopePolicyStamp: 'policy',
    nodes: [{ kind: 'element', tag: 'section', namespace: null, renderNodeId: 'section', attributes: [], children: [
        { kind: 'element', tag, namespace: null, renderNodeId: 'child', attributes: label === null ? [] : [{ name: 'label', value: label }], children: [] },
        { kind: 'element', tag: 'p', namespace: null, renderNodeId: 'sibling', attributes: [], children: [{ kind: 'text', text: revision, renderNodeId: 'text' }] },
    ] }] });

function story(patches: boolean): Story {
    return { render: () => '<main></main>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('main');
        if (!host) throw new Error('missing fixture');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const initial = plan('1', 'Before');
        applyRenderPlanToRange(bounds, initial, document);
        const child = host.querySelector(tag);
        if (!child) throw new Error('missing custom element');
        const output = child.querySelector('output');
        expect(output).not.toBeNull();
        let before = initial;
        for (const next of [plan('2', 'After'), plan('3', null), plan('4', null)]) {
            if (patches) expect(applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, next), renderPlanIdentity(next), document).status).toBe('applied');
            else applyRenderPlanToRange(bounds, next, document);
            expect(host.querySelector(tag)).toBe(child);
            expect(child.querySelector('output')).toBe(output);
            expect(child).toHaveAttribute('data-state', 'ready');
            expect(child.getAttribute('label')).toBe(next.dataRevision === '2' ? 'After' : null);
            expect(output?.textContent).toBe('Component-owned output');
            expect(host.querySelector('p')?.textContent).toBe(next.dataRevision);
            before = next;
        }
    } };
}
export const RenderPlan = story(false);
export const WorkerPatches = story(true);

function childInputs(patches: boolean, dynamicTextRanges = false): Story {
    return { render: () => '<main></main>', play: ({ canvasElement }) => {
        const host = canvasElement.querySelector('main');
        if (!host) throw new Error('missing fixture');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') };
        host.append(bounds.start, bounds.end);
        const make = (revision: string, values: string[]): RenderPlan => {
            const result = plan(revision, 'Inputs');
            const root = result.nodes[0];
            if (root.kind !== 'element' || root.children[0].kind !== 'element') throw new Error('missing fixture');
            root.children[0].children = values.map(value => ({ kind: 'element', tag: 'span', namespace: null,
                renderNodeId: `input-${value}`, attributes: [], children: [{ kind: 'text', renderNodeId: `text-${value}`, text: `${value}:${revision}` }] }));
            return result;
        };
        let before = make('1', ['A', 'B']);
        applyRenderPlanToRange(bounds, before, document, { dynamicTextRanges });
        const child = host.querySelector(tag);
        if (!child) throw new Error('missing custom element');
        const output = child.querySelector('output');
        for (const [index, values] of [['B', 'A'], ['B', 'C'], []].entries()) {
            const next = make(String(index + 2), values);
            if (patches) expect(applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, next), renderPlanIdentity(next), document).status).toBe('applied');
            else applyRenderPlanToRange(bounds, next, document, { dynamicTextRanges });
            expect(child.querySelector('output')).toBe(output);
            expect(child).toHaveAttribute('data-state', 'ready');
            expect([...child.querySelectorAll('span')].map(node => node.textContent)).toEqual(values.map(value => `${value}:${index + 2}`));
            before = next;
        }
    } };
}
export const AuthoredChildren = childInputs(false);
export const AuthoredChildrenPatches = childInputs(true);
export const AuthoredChildrenDynamicRanges = childInputs(false, true);

export const NestedBindingsKeepTheirOwner: Story = {
    loaders: [async () => {
        await loadCemDeclaration('cem-fixture-ownership-inner', `<cem-element tag="cem-fixture-ownership-inner"><template type="text/cem-ml">
            {slice @name=value | initial}
            {label | Inner value {input @slice=value @slice-event=input @slice-value="$target.value"}}
            {output | {$datadom.slices.value}}
        </template></cem-element>`);
        await loadCemDeclaration('cem-fixture-ownership-outer', `<cem-element tag="cem-fixture-ownership-outer"><template type="text/cem-ml">
            {slice @name=value | initial}
            {label | Outer value {input @slice=value @slice-event=input @slice-value="$target.value"}}
            {output | {$datadom.slices.value}}
            {slot}
        </template></cem-element>`);
        return {};
    }],
    render: () => '<cem-fixture-ownership-outer><cem-fixture-ownership-inner></cem-fixture-ownership-inner></cem-fixture-ownership-outer>',
    play: async ({ canvasElement }) => {
        const outer = canvasElement.querySelector<HTMLElement>('cem-fixture-ownership-outer');
        if (!outer) throw new Error('missing outer instance');
        await whenCemRendered(outer);
        const inner = canvasElement.querySelector<HTMLElement>('cem-fixture-ownership-inner');
        if (!inner) throw new Error('missing inner instance');
        await whenCemRendered(inner);
        const input = within(inner).getByRole('textbox', { name: 'Inner value' });
        await userEvent.type(input, 'first'); await whenCemRendered(inner);
        await userEvent.type(within(outer).getByRole('textbox', { name: 'Outer value' }), 'changed');
        await whenCemRendered(outer);
        await userEvent.clear(input); await userEvent.type(input, 'second');
        await whenCemRendered(inner);
        await expect(inner.querySelector('output')).toHaveTextContent('second');
        await expect(outer.querySelector('output')).toHaveTextContent('changed');
        await expect(within(inner).getByRole('textbox')).toBe(input);
    },
};
