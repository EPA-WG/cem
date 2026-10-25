import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, within } from 'storybook/test';
import { cemStoryRuntime, loadCemDeclaration, whenCemRendered } from '../../.storybook/preview.js';
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

export const ProjectedBindingsKeepTheirAuthor: Story = {
    loaders: [async () => {
        await loadCemDeclaration('cem-fixture-projected-action', `<cem-element tag="cem-fixture-projected-action"><template type="text/cem-ml">
            {attribute @name=label | Projected component action}
            {button @type=button @aria-label={$label} | {$label}}
        </template></cem-element>`);
        await loadCemDeclaration('cem-fixture-projected-wrapper', '<cem-element tag="cem-fixture-projected-wrapper"><template type="text/cem-ml">{div | {slot}}</template></cem-element>');
        await loadCemDeclaration('cem-fixture-projected-container', `<cem-element tag="cem-fixture-projected-container"><template type="text/cem-ml">
            {attribute @name=caption | Initial container}
            {slice @name=value | child-initial}
            {section @aria-label={$caption} |
                {button @type=button @aria-label="Child action" @slice=value @slice-event=click @slice-value="'child-edited'" | Child action}
                {output @aria-label="Child value" | {$datadom.slices.value}}
                {slot @name=actions}
                {slot}
            }
        </template></cem-element>`);
        await loadCemDeclaration('cem-fixture-projected-owner', `<cem-element tag="cem-fixture-projected-owner"><template type="text/cem-ml">
            {slice @name=value | outer-initial}
            {button @type=button @aria-label="Outer action" @slice=value @slice-event=click @slice-value="'outer-own'" | Outer action}
            {cem-fixture-projected-container |
                {button @slot=actions @type=button @aria-label="Projected native action" @slice=value @slice-event=click @slice-value="'outer-button'" | Projected native action}
                {cem-fixture-projected-action @label="Projected component action" @slice=value @slice-event=click @slice-value="'outer-component'"}
                {cem-fixture-projected-wrapper | {button @type=button @aria-label="Nested projected action" @slice=value @slice-event=click @slice-value="'outer-nested'" | Nested projected action}}
                {output @aria-label="Projected value" | {$datadom.slices.value}}
            }
            {output @aria-label="Outer value" | {$datadom.slices.value}}
        </template></cem-element>`);
        return {};
    }],
    render: () => '<cem-fixture-projected-owner></cem-fixture-projected-owner>',
    play: async ({ canvasElement }) => {
        const outer = canvasElement.querySelector<HTMLElement>('cem-fixture-projected-owner');
        if (!outer) throw new Error('missing projected binding owner');
        const settle = async () => {
            await whenCemRendered(outer);
            for (const child of outer.querySelectorAll<HTMLElement>('cem-fixture-projected-container, cem-fixture-projected-action, cem-fixture-projected-wrapper')) await whenCemRendered(child);
        };
        await settle();
        const container = outer.querySelector<HTMLElement>('cem-fixture-projected-container');
        if (!container) throw new Error('missing projection container');
        const values = (outerValue: string, childValue: string) => {
            expect(within(outer).getByLabelText('Outer value').textContent).toBe(outerValue);
            expect(within(container).getByLabelText('Projected value').textContent).toBe(outerValue);
            expect(within(container).getByLabelText('Child value').textContent).toBe(childValue);
        };
        values('outer-initial', 'child-initial');
        await userEvent.click(within(container).getByRole('button', { name: 'Projected native action' })); await settle();
        values('outer-button', 'child-initial');
        await userEvent.click(within(container).getByRole('button', { name: 'Projected component action' })); await settle();
        values('outer-component', 'child-initial');
        await userEvent.click(within(container).getByRole('button', { name: 'Child action' })); await settle();
        values('outer-component', 'child-edited');
        container.setAttribute('caption', 'Rerendered container'); await settle();
        expect(container.querySelector('section')?.getAttribute('aria-label')).toBe('Rerendered container');
        await userEvent.click(within(outer).getByRole('button', { name: 'Outer action' })); await settle();
        values('outer-own', 'child-edited');
        expect(outer.querySelector('cem-fixture-projected-container')).toBe(container);
        await userEvent.click(within(container).getByRole('button', { name: 'Projected native action' })); await settle();
        values('outer-button', 'child-edited');
        await userEvent.click(within(container).getByRole('button', { name: 'Projected component action' })); await settle();
        values('outer-component', 'child-edited');
        await userEvent.click(within(container).getByRole('button', { name: 'Nested projected action' })); await settle();
        values('outer-nested', 'child-edited');

        const projected = within(container).getByRole('button', { name: 'Projected native action' });
        const identity = projected.getAttribute('data-cem-render-node-id');
        if (!identity) throw new Error('missing projected action identity');
        projected.setAttribute('data-cem-render-node-id', 'forged-owner');
        await userEvent.click(projected); await settle();
        values('outer-nested', 'child-edited');
        projected.setAttribute('data-cem-render-node-id', identity);
        await userEvent.click(projected); await settle();
        values('outer-button', 'child-edited');

        await userEvent.click(within(outer).getByRole('button', { name: 'Outer action' })); await settle();
        values('outer-own', 'child-edited');
        const originalParent = projected.parentNode, next = projected.nextSibling;
        if (!originalParent) throw new Error('missing projected action parent');
        canvasElement.append(projected);
        await userEvent.click(projected); await settle();
        values('outer-own', 'child-edited');
        originalParent.insertBefore(projected, next);
        await userEvent.click(projected); await settle();
        values('outer-button', 'child-edited');

        outer.remove(); canvasElement.append(projected);
        await userEvent.click(projected);
        values('outer-button', 'child-edited');
    },
};

export const InitialSliceEventsBeforePublication: Story = {
    loaders: [async () => {
        if (!customElements.get('cem-fixture-init-probe')) customElements.define('cem-fixture-init-probe', class extends HTMLElement {
            valueAtPublication: string | null = null;
            connectedCallback() { this.valueAtPublication = this.closest('cem-fixture-initial-owner')?.getAttribute('initialized') ?? null; }
        });
        await loadCemDeclaration('cem-fixture-initial-owner', `<cem-element tag="cem-fixture-initial-owner"><template>
            <attribute name="caption">Initial</attribute><slice name="value">before-init</slice>
            <button type="button" slice="value|/datadom/attributes/initialized" slice-event="init" slice-value="$event.type">Initialize</button>
            <output aria-label="Initialized value">\${$value}</output>
            <p>\${$caption}</p><cem-fixture-init-probe></cem-fixture-init-probe>
        </template></cem-element>`);
        return {};
    }],
    render: () => '<cem-fixture-initial-owner></cem-fixture-initial-owner>',
    play: async ({ canvasElement }) => {
        const owner = canvasElement.querySelector<HTMLElement>('cem-fixture-initial-owner');
        if (!owner) throw new Error('missing initial slice owner');
        await whenCemRendered(owner);
        expect((owner.querySelector('cem-fixture-init-probe') as HTMLElement & { valueAtPublication: string | null }).valueAtPublication).toBe('init');
        expect(within(owner).getByLabelText('Initialized value')).toHaveTextContent('init');
        owner.setAttribute('caption', 'Rerendered');
        await whenCemRendered(owner);
        expect(within(owner).getByLabelText('Initialized value')).toHaveTextContent('init');
        expect(owner.querySelector('p')).toHaveTextContent('Rerendered');
    },
};

export const DeferredSurfaceBindingsKeepTheirOwner: Story = {
    loaders: [async () => {
        await loadCemDeclaration('cem-fixture-surface-binding-owner', `<cem-element tag="cem-fixture-surface-binding-owner"><template type="text/cem-ml">
            {slice @name=value | initial}
            {button @type=button @slice=value @slice-event=click @slice-value="'owner-own'" | Own action}
            {output @aria-label="Surface owner value" | {$datadom.slices.value}}
        </template></cem-element>`);
        return {};
    }],
    render: () => '<cem-fixture-surface-binding-owner></cem-fixture-surface-binding-owner><section aria-label="External deferred body"><button type="button" slice="value" slice-event="click" slice-value="\'external\'">External action</button></section>',
    play: async ({ canvasElement }) => {
        const owner = canvasElement.querySelector<HTMLElement>('cem-fixture-surface-binding-owner');
        if (!owner) throw new Error('missing surface binding owner');
        const body = canvasElement.querySelector<HTMLElement>('section');
        if (!body) throw new Error('missing external deferred body');
        const action = body.querySelector('button');
        if (!action) throw new Error('missing external surface action');
        const controller = new AbortController();
        try {
            await whenCemRendered(owner);
            await cemStoryRuntime().prepareSurfaceBody(owner, body, controller.signal);
            await userEvent.click(action); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('external');
            await userEvent.click(within(owner).getByRole('button', { name: 'Own action' })); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('owner-own');
            canvasElement.append(action);
            await userEvent.click(action); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('owner-own');
            body.append(action);
            await userEvent.click(action); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('external');
            await userEvent.click(within(owner).getByRole('button', { name: 'Own action' })); await whenCemRendered(owner);
            controller.abort();
            await userEvent.click(action); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('owner-own');
            owner.append(body);
            // Exercise the expired grant inside its owner before observation
            // can rerender the host and replace this explicit body placement.
            action.click(); await whenCemRendered(owner);
            expect(within(owner).getByLabelText('Surface owner value')).toHaveTextContent('owner-own');
        } finally { controller.abort(); }
    },
};
