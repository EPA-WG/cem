import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent } from 'storybook/test';
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
