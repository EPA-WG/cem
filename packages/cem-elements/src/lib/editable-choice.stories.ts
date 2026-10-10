import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Editable Choice', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Choice = HTMLElement & { value: string; displayValue: string; selectedIndex: number; expanded: boolean; validity: ValidityState; reportValidity(): boolean; formStateRestoreCallback(value: string, mode: string): void };
const declarationSource = `<cem-element capability="editable-choice"><template type="text/cem-ml">
{div @part=root | {input @part=control @role=combobox @aria-label=Choice @value={datadom.slices.displayValue} @aria-expanded={datadom.slices.expanded} @aria-controls={if datadom.slices.expanded {datadom.slices.listboxId} else {null}} @aria-activedescendant={if datadom.slices.expanded {datadom.slices.activeOptionId} else {null}}}
{cem:if @test=datadom.slices.expanded | {div @part=popup @role=listbox @id={datadom.slices.listboxId} | {cem:for-each @select=datadom.slices.groups @as=group | {cem:for-each @select=group.options @as=option | {div @part=option @role=option @id={option.id} @data-option-index={option.index} @aria-disabled={option.disabled} @aria-selected={option.selected} | {$option.label}}}}}}
}
</template></cem-element>`;

export const AuthoritativePayloadAndNativeForms: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `editable-choice-${crypto.randomUUID()}`;
            const declaration = await declare(declarationSource, tag);
            const form = document.createElement('form');
            form.innerHTML = `<fieldset><${tag} name="choice" required auto-active-first><cem-option value="a" selected>Alpha</cem-option><cem-option value="b">Beta</cem-option></${tag}></fieldset><button type="reset">Reset</button>`;
            canvasElement.append(form);
            const host = form.querySelector(tag) as Choice;
            try {
                await runtime.whenRenderSettled(host);
                const input = required(host.querySelector('input'));
                expect(host.value).toBe('a'); expect(input.value).toBe('Alpha');
                expect(new FormData(form).getAll('choice')).toEqual(['a']);
                input.focus(); await runtime.whenRenderSettled(host);
                expect(host.querySelector('input')).toBe(input);
                const events: string[] = [];
                for (const name of ['input', 'change']) host.addEventListener(name, () => events.push(`${name}:${host.value}:${input.value}:${new FormData(form).get('choice')}`));
                await userEvent.keyboard('{ArrowDown}{Enter}');
                expect(events).toEqual(['input:b:Beta:b', 'change:b:Beta:b']);
                await runtime.whenRenderSettled(host); events.length = 0;
                host.expanded = true; await runtime.whenRenderSettled(host);
                const island = required(host.querySelector<HTMLTemplateElement>('template[data-cem-island=instance]'));
                island.content.querySelectorAll('cem-option').forEach(option => option.remove());
                const option = document.createElement('cem-option'); option.setAttribute('value', 'c'); option.textContent = 'Gamma'; required(island.content.querySelector('cem-payload\\:payload')).append(option);
                await waitFor(() => expect(host.querySelectorAll('[role=option]')).toHaveLength(1));
                expect(host.querySelector('[role=option]')?.textContent).toBe('Gamma');
                expect(host.querySelector('input')).toBe(input); expect(document.activeElement).toBe(input);
                expect(host.value).toBe('b'); expect(host.displayValue).toBe('Beta'); expect(host.selectedIndex).toBe(-1); expect(events).toEqual([]);
                option.remove(); await waitFor(() => expect(host.expanded).toBe(false));
                expect(input.hasAttribute('aria-controls')).toBe(false); expect(input.hasAttribute('aria-activedescendant')).toBe(false);
                host.value = 'free'; await runtime.whenRenderSettled(host); expect(input.value).toBe('free');
                form.reset(); await runtime.whenRenderSettled(host); expect(host.value).toBe('a'); expect(input.value).toBe('Alpha');
                host.formStateRestoreCallback('restored', 'restore'); await runtime.whenRenderSettled(host); expect(input.value).toBe('restored');
                host.setAttribute('require-selection', ''); host.formStateRestoreCallback('missing', 'restore'); await runtime.whenRenderSettled(host);
                expect(host.value).toBe(''); expect(host.validity.valueMissing).toBe(true);
                input.focus(); await userEvent.type(input, 'missing'); expect(host.expanded).toBe(false);
                await userEvent.keyboard('{Escape}'); expect(host.displayValue).toBe(''); expect(input.value).toBe(''); events.length = 0;
                required(form.querySelector('button')).focus(); expect(host.reportValidity()).toBe(false); expect(document.activeElement).toBe(input);
                required(form.querySelector('fieldset')).disabled = true; expect([...new FormData(form)]).toEqual([]);
                host.remove(); required(form.querySelector('fieldset')).append(host); await runtime.whenRenderSettled(host);
                expect(host.querySelector('input')).toBe(input); expect(events).toEqual([]);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { form.remove(); declaration.remove(); scope.dispose(); }
        }
    },
};

export const ActiveIdentityAndOwnerBoundaries: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, false);
        const tag = `editable-boundaries-${crypto.randomUUID()}`;
        const declaration = await declare(declarationSource.replace('{div @part=root |', '{div @part=root | {nested-choice | {input @part=control @role=combobox @aria-label=Nested}{div @part=option @data-option-index=1 | Nested option}}'), tag);
        const host = document.createElement(tag) as Choice;
        host.innerHTML = '<cem-option value="a" selected>Alpha</cem-option><cem-option value="b">Beta</cem-option><cem-option value="c" disabled>Unavailable</cem-option>';
        canvasElement.append(host);
        try {
            await runtime.whenRenderSettled(host);
            const input = required(host.querySelector<HTMLInputElement>('[part=root] > input'));
            const nested = required(host.querySelector<HTMLInputElement>('nested-choice input'));
            const events: string[] = [];
            host.addEventListener('input', event => { if (event.target === host) events.push('input'); });
            nested.focus(); await userEvent.type(nested, 'nested'); await runtime.whenRenderSettled(host);
            expect(host.value).toBe('a'); expect(host.expanded).toBe(false);
            await userEvent.click(required(host.querySelector('nested-choice [part=option]'))); expect(host.value).toBe('a');
            input.focus(); await runtime.whenRenderSettled(host);
            const active = input.getAttribute('aria-activedescendant');
            for (const modifiers of [{ isComposing: true }, { ctrlKey: true }, { metaKey: true }, { shiftKey: true }]) {
                const event = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true, ...modifiers });
                input.dispatchEvent(event); expect(event.defaultPrevented).toBe(false); expect(input.getAttribute('aria-activedescendant')).toBe(active);
            }
            await userEvent.keyboard('{ArrowDown}'); await runtime.whenRenderSettled(host);
            const island = required(host.querySelector<HTMLTemplateElement>('template[data-cem-island=instance]'));
            const payload = required(island.content.querySelector('cem-payload\\:payload'));
            payload.prepend(required(payload.querySelector('[value=b]')));
            await waitFor(() => expect(host.selectedIndex).toBe(1));
            expect(host.querySelector(`#${input.getAttribute('aria-activedescendant')}`)?.textContent).toBe('Beta');
            expect(host.selectedIndex).toBe(1); expect(host.value).toBe('a');
            required(payload.querySelector('[value=b]')).setAttribute('disabled', 'false');
            await waitFor(() => expect(host.querySelector(`#${input.getAttribute('aria-activedescendant')}`)?.textContent).toBe('Alpha'));
            expect(events).toEqual([]); expect(host.querySelector('[part=root] > input')).toBe(input);
            host.remove(); canvasElement.append(host); await runtime.whenRenderSettled(host);
            input.focus(); host.expanded = true; await runtime.whenRenderSettled(host);
            await userEvent.keyboard('{Enter}'); expect(events).toEqual(['input']);
            expect(runtime.diagnosticsFor(host)).toEqual([]);
        } finally { host.remove(); declaration.remove(); scope.dispose(); }
    },
};

function required<T extends Element>(element: T | null): T {
    if (!element) throw new Error('Missing editable-choice fixture element');
    return element;
}
