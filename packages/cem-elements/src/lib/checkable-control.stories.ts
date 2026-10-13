import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
import { captureNativeCheckedEdits, applyRenderPlanToRange, applyPatchFramesToRange, diffRenderPlansToPatchFrames, renderPlanIdentity, materializeRenderPlan, mergeRenderedFragmentIntoRange, type RenderPlan } from './projection.js';
import { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Checkable Control', tags: ['test'] };
export default meta;
type Story = StoryObj;

export const NativeOwnerBoundaries: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        for (const template of [
            '{input @part=control @type=checkbox @role=switch}',
            '{input @part=control @type=radio}',
            '{input @part=control @type=checkbox}{input @part=control @type=checkbox}',
            '{another-control | {input @part=control @type=checkbox}}',
        ]) {
            const tag = `checkable-boundary-${crypto.randomUUID()}`;
            const declaration = await declare(`<cem-element capability="checkable-control"><template type="text/cem-ml">${template}</template></cem-element>`, tag);
            const host = document.createElement(tag); host.setAttribute('indeterminate', ''); canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                expect([...host.querySelectorAll('input')].every(input => !input.indeterminate)).toBe(true);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); declaration.remove(); }
        }
        scope.dispose();
    },
};

export const NativeMixedLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `checkable-contract-${crypto.randomUUID()}`;
            const declaration = await declare('<cem-element capability="checkable-control"><template type="text/cem-ml">{label | Mixed choice {input @part=control @type=checkbox @name=choice @value=yes @checked={if seq:count(datadom.attributes.checked) > 0 { true } else { null }}}}</template></cem-element>', tag);
            const form = document.createElement('form');
            form.innerHTML = `<${tag} indeterminate checked></${tag}>`;
            canvasElement.append(form);
            const host = form.querySelector(tag) as HTMLElement;
            try {
                await runtime.whenRenderSettled(host);
                const input = host.querySelector('input') as HTMLInputElement;
                expect(input.indeterminate).toBe(true); expect(input.checked).toBe(true);
                expect(input.getAttribute('aria-checked')).toBeNull();
                const events: string[] = []; input.addEventListener('input', () => events.push('input')); input.addEventListener('change', () => events.push('change'));
                await userEvent.click(input);
                expect(input.indeterminate).toBe(false); expect(events).toEqual(['input', 'change']);
                host.setAttribute('revision', 'next'); await runtime.whenRenderSettled(host);
                expect(input.indeterminate).toBe(false); expect(input.defaultChecked).toBe(true);
                form.reset(); expect(input.checked).toBe(true);
                host.removeAttribute('indeterminate'); await runtime.whenRenderSettled(host);
                for (const value of ['', 'false', 'mixed']) {
                    host.setAttribute('indeterminate', value); await runtime.whenRenderSettled(host); expect(input.indeterminate).toBe(true);
                    host.removeAttribute('indeterminate'); await runtime.whenRenderSettled(host); expect(input.indeterminate).toBe(false);
                }
                host.setAttribute('indeterminate', ''); await runtime.whenRenderSettled(host); await userEvent.click(input);
                host.remove(); form.append(host); await runtime.whenRenderSettled(host);
                expect(host.querySelector('input')).toBe(input); expect(input.indeterminate).toBe(false);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { form.remove(); declaration.remove(); scope.dispose(); }
        }
    },
};

export const NativeRadioEditsDuringPublication: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const mode of ['plan', 'patch', 'fragment']) {
            const root = document.createElement('div'); canvasElement.append(root);
            const bounds = { start: document.createComment('start'), end: document.createComment('end') }; root.append(bounds.start, bounds.end);
            const plan = (checked: string, revision: string): RenderPlan => ({ producedTag: 'radio-race', instanceId: 'radio-race', templateArtifactId: 'radio-race',
                dataRevision: revision, outputTarget: 'light-dom', scopePolicyStamp: 'radio-race', nodes: [{ kind: 'element', namespace: null, tag: 'form', renderNodeId: 'form', attributes: [],
                    children: ['a', 'b'].map(value => ({ kind: 'element', namespace: null, tag: 'input', renderNodeId: value,
                        attributes: [{ name: 'type', value: 'radio' }, { name: 'name', value: 'choice' }, { name: 'value', value }, ...(checked === value ? [{ name: 'checked', value: '' }] : [])], children: [] })) }] });
            try {
                let before = plan('a', '1'); applyRenderPlanToRange(bounds, before, document);
                const [a, b] = root.querySelectorAll('input'); const events: string[] = [];
                for (const input of [a, b]) for (const type of ['input', 'change']) input.addEventListener(type, () => events.push(`${type}:${input.value}`));
                const commit = (next: RenderPlan, preserveNativeCheckedState = captureNativeCheckedEdits(document)) => {
                    const options = { preserveNativeCheckedState };
                    if (mode === 'patch') expect(applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, next), renderPlanIdentity(next), document, options).status).toBe('applied');
                    else if (mode === 'fragment') mergeRenderedFragmentIntoRange(bounds, materializeRenderPlan(next, document), options);
                    else applyRenderPlanToRange(bounds, next, document, options);
                    before = next;
                };
                b.click();
                const pending = captureNativeCheckedEdits(document);
                // The parent has requested B. While its child publication is in
                // flight, the user selects A again on the same native owners.
                a.click(); commit(plan('b', '2'), pending);
                expect([...root.querySelectorAll('input')]).toEqual([a, b]);
                expect([a.checked, b.checked]).toEqual([true, false]);
                expect([a.defaultChecked, b.defaultChecked]).toEqual([false, true]);
                commit(plan('a', '3')); commit(plan('b', '4'));
                expect([a.checked, b.checked]).toEqual([false, true]);
                expect(events).toEqual(['input:b', 'change:b', 'input:a', 'change:a']);
            } finally { root.remove(); }
        }
    },
};

export const RuntimeRadioEditsDuringPublication: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `radio-publication-${crypto.randomUUID()}`, source = document.createElement('div');
            source.hidden = true; source.setAttribute('tag', tag);
            source.innerHTML = '<template type="text/cem-ml">{input @type=radio @name=publication @value={datadom.attributes.value} @checked={if seq:count(datadom.attributes.checked) > 0 {true} else {null}}}</template>';
            canvasElement.append(source);
            let editDuringRender: (() => void) | undefined;
            runtime.registerDeclaration(source, { behaviorIdentity: 'radio-publication-fixture', behavior: {
                beforeRender() { const edit = editDuringRender; editDuringRender = undefined; if (edit) queueMicrotask(edit); },
            } });
            await runtime.whenDeclarationSettled(source);
            const form = document.createElement('form');
            form.innerHTML = `<${tag} value="a" checked></${tag}><${tag} value="b"></${tag}>`; canvasElement.append(form);
            const [first, second] = [...form.children] as HTMLElement[];
            try {
                await Promise.all([first, second].map(host => runtime.whenRenderSettled(host)));
                const [a, b] = form.querySelectorAll('input');
                b.click(); editDuringRender = () => a.click();
                first.removeAttribute('checked'); second.setAttribute('checked', '');
                await Promise.all([first, second].map(host => runtime.whenRenderSettled(host)));
                expect([...form.querySelectorAll('input')]).toEqual([a, b]);
                expect([a.checked, b.checked]).toEqual([true, false]);
                expect([a.defaultChecked, b.defaultChecked]).toEqual([false, true]);
                first.setAttribute('checked', ''); second.removeAttribute('checked');
                await Promise.all([first, second].map(host => runtime.whenRenderSettled(host)));
                expect([...new FormData(form)]).toEqual([['publication', 'a']]);
                expect(runtime.diagnosticsFor(first)).toEqual([]); expect(runtime.diagnosticsFor(second)).toEqual([]);
            } finally { form.remove(); source.remove(); scope.dispose(); }
        }
    },
};
