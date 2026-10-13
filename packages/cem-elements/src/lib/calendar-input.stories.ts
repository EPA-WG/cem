import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Calendar Input', tags: ['test'] };
export default meta;
type Story = StoryObj;
type Calendar = HTMLElement & { expanded: boolean };
const declaration = `<cem-element capability="calendar-input"><template type="text/cem-ml">
{div @part=root | {slot @name=input}{slot @name=toggle}
  {dialog @part=dialog @id={datadom.slices.dialogId} |
    {h2 @id={datadom.slices.headingId} | {$datadom.slices.heading}}
    {div @role=grid @aria-labelledby={datadom.slices.headingId} |
      {cem:for-each @select=datadom.slices.weeks @as=week | {div @role=row |
        {cem:for-each @select=week.days @as=day | {cem:if @test=day.value |
          {button @part=day @role=gridcell @type=button @data-date={day.value} @data-active={day.active} @aria-label={day.fullLabel} @aria-selected={day.selected} @disabled={if day.disabled {true} else {null}} @tabindex={day.tabIndex} | {$day.number}}
        }}
      }}
    }
    {button @type=button @data-calendar-action=cancel | Cancel}
    {button @type=button @data-calendar-action=apply | Apply}
  }
}
</template></cem-element>`;
function required<T extends Element>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing calendar fixture ${selector}`);
    return node;
}
export const NativeOwnershipAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `calendar-input-${crypto.randomUUID()}`, source = await declare(declaration, tag);
            const form = document.createElement('form');
            form.innerHTML = `<fieldset><${tag} min="2024-01-01" max="2025-12-31" required><input slot="input" type="text" name="date" value="2024-02-29" aria-label="Departure"><button type="button" slot="toggle"><strong>Choose</strong></button></${tag}></fieldset><button type="reset">Reset</button>`;
            canvasElement.append(form);
            const host = required<Calendar>(form, tag);
            try {
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), dialog = required<HTMLDialogElement>(host, 'dialog');
                expect(new FormData(form).getAll('date')).toEqual(['2024-02-29']);
                const events: string[] = [];
                for (const name of ['input', 'change']) input.addEventListener(name, event => events.push(`${event.type}:${input.value}:${dialog.open}:${document.activeElement === input}`));
                for (const modifiers of [{ isComposing: true }, { ctrlKey: true }, { metaKey: true }, { shiftKey: true }]) {
                    const event = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true, ...modifiers });
                    input.dispatchEvent(event); expect(event.defaultPrevented).toBe(false); expect(host.expanded).toBe(false);
                }
                await userEvent.click(required(host, 'button[slot=toggle] strong'));
                await runtime.whenRenderSettled(host);
                await waitFor(() => expect(dialog.matches(':modal')).toBe(true));
                expect(dialog).toHaveAccessibleName('Departure calendar');
                expect(required<HTMLElement>(dialog, '[tabindex="0"]').dataset.date).toBe('2024-02-29');
                await userEvent.keyboard('{PageDown}'); await runtime.whenRenderSettled(host);
                await userEvent.keyboard('{Enter}'); await runtime.whenRenderSettled(host);
                expect(input.value).toBe('2024-02-29'); expect(events).toEqual([]);
                await userEvent.click(required(dialog, '[data-calendar-action=apply]'));
                await waitFor(() => expect(events).toEqual(['input:2024-03-29:false:true', 'change:2024-03-29:false:true']));
                expect(new FormData(form).getAll('date')).toEqual(['2024-03-29']);
                host.expanded = true; await runtime.whenRenderSettled(host);
                required<HTMLFieldSetElement>(form, 'fieldset').disabled = true;
                await waitFor(() => expect(dialog.open).toBe(false));
                host.expanded = true; expect(host.expanded).toBe(false); expect([...new FormData(form)]).toEqual([]);
                required<HTMLFieldSetElement>(form, 'fieldset').disabled = false;
                form.reset(); await waitFor(() => expect(input.value).toBe('2024-02-29')); await runtime.whenRenderSettled(host);
                events.length = 0; host.expanded = true; await runtime.whenRenderSettled(host);
                host.remove(); required(form, 'fieldset').append(host); await runtime.whenRenderSettled(host);
                expect(required(host, 'input')).toBe(input); expect(required(host, 'dialog')).toBe(dialog);
                expect(dialog.open).toBe(false); expect(host.expanded).toBe(false); expect(events).toEqual([]);
                input.value = '2025-01-10'; host.expanded = true; await runtime.whenRenderSettled(host);
                expect(required<HTMLElement>(dialog, '[tabindex="0"]').dataset.date).toBe('2025-01-10');
                await userEvent.keyboard('{Escape}'); await runtime.whenRenderSettled(host);
                expect(events).toEqual([]); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { form.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const AuthoredPayloadUpdates: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `calendar-input-${crypto.randomUUID()}`;
            const parentTag = `calendar-form-${crypto.randomUUID()}`;
            const source = await declare(declaration, tag);
            const parentSource = await declare(`<cem-element><template type="text/cem-ml">
{slice @name=label | Arrival}{slice @name=name | arrival}{slice @name=value | 2024-02-29}{slice @name=extra | false}
{form | {cem:if @test=datadom.slices.extra | {p | Additional guidance}}
  {${tag} | {input @slot=input @type=text @aria-label={datadom.slices.label} @name={datadom.slices.name} | {attribute @name=value @value={datadom.slices.value}}}}
}
</template></cem-element>`, parentTag);
            const parent = document.createElement(parentTag);
            canvasElement.append(parent);
            try {
                await runtime.whenRenderSettled(parent);
                const host = required<Calendar>(parent, tag);
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), dialog = required(host, 'dialog');
                const events: string[] = [];
                input.addEventListener('input', () => events.push('input'));
                input.addEventListener('change', () => events.push('change'));
                input.value = '2024-03-01';
                for (const values of [
                    { extra: true, label: 'Departure', name: 'departure', value: '2024-02-29' },
                    { extra: false, label: 'Return', name: 'return', value: '2025-01-10' },
                ]) {
                    runtime.setInstanceSlices(parent, values);
                    await runtime.whenRenderSettled(parent);
                    await runtime.whenRenderSettled(host);
                    expect(required(parent, tag)).toBe(host);
                    expect(required(host, 'input')).toBe(input);
                    expect(required(host, 'dialog')).toBe(dialog);
                    expect(input).toHaveAccessibleName(values.label);
                    expect(input.name).toBe(values.name);
                    expect(input.defaultValue).toBe(values.value);
                    expect(input.value).toBe(values.extra ? '2024-03-01' : values.value);
                    expect([...new FormData(required<HTMLFormElement>(parent, 'form'))]).toEqual([[values.name, input.value]]);
                }
                expect(events).toEqual([]);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
                expect(runtime.diagnosticsFor(parent)).toEqual([]);
            } finally { parent.remove(); parentSource.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const AuthoredExplicitPayloadUpdates: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `calendar-input-${crypto.randomUUID()}`;
            const parentTag = `calendar-form-${crypto.randomUUID()}`;
            const source = await declare(declaration, tag);
            const parentSource = await declare(`<cem-element><template type="text/cem-ml">
{slice @name=label | Arrival}{slice @name=name | arrival}{slice @name=value | 2024-02-29}{slice @name=extra | false}
{form | {cem:if @test=datadom.slices.extra | {p | Additional guidance}}
  {${tag} | {template | {input @slot=input @type=text @aria-label={datadom.slices.label} @name={datadom.slices.name} | {attribute @name=value @value={datadom.slices.value}}}{cem:if @test=datadom.slices.extra | {button @slot=toggle @type=button | Choose}}}}
}
</template></cem-element>`, parentTag);
            const parent = document.createElement(parentTag);
            canvasElement.append(parent);
            try {
                await runtime.whenRenderSettled(parent);
                const host = required<Calendar>(parent, tag);
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), dialog = required(host, 'dialog');
                const events: string[] = [];
                input.addEventListener('input', () => events.push('input'));
                input.addEventListener('change', () => events.push('change'));
                input.value = '2024-03-01';
                for (const values of [
                    { extra: true, label: 'Departure', name: 'departure', value: '2024-02-29' },
                    { extra: false, label: 'Return', name: 'return', value: '2025-01-10' },
                ]) {
                    runtime.setInstanceSlices(parent, values);
                    await runtime.whenRenderSettled(parent);
                    await runtime.whenRenderSettled(host);
                    expect(required(parent, tag)).toBe(host);
                    expect(required(host, 'input')).toBe(input);
                    expect(required(host, 'dialog')).toBe(dialog);
                    expect(input).toHaveAccessibleName(values.label);
                    expect(input.name).toBe(values.name);
                    expect(input.defaultValue).toBe(values.value);
                    expect(input.value).toBe(values.extra ? '2024-03-01' : values.value);
                    expect([...new FormData(required<HTMLFormElement>(parent, 'form'))]).toEqual([[values.name, input.value]]);
                }
                expect(events).toEqual([]);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
                expect(runtime.diagnosticsFor(parent)).toEqual([]);
            } finally { parent.remove(); parentSource.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const NativeCloseCompletesBeforeRendering: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `calendar-close-${crypto.randomUUID()}`, source = await declare(declaration, tag);
            const host = document.createElement(tag) as Calendar;
            host.innerHTML = '<input slot="input" type="text" value="2024-02-29" aria-label="Arrival">';
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), dialog = required<HTMLDialogElement>(host, 'dialog');
                input.focus(); host.expanded = true; await runtime.whenRenderSettled(host);
                const events: string[] = [];
                for (const type of ['input', 'change']) input.addEventListener(type, event => events.push(`${event.type}:${dialog.open}:${document.activeElement === input}`));
                const apply = required<HTMLButtonElement>(dialog, '[data-calendar-action=apply]');
                apply.focus(); apply.click();
                // Native close, focus and commit form one operation, even while the
                // next worker publication has not had an opportunity to complete.
                expect(dialog.open).toBe(false);
                expect(document.activeElement).toBe(input);
                expect(input.getAttribute('aria-expanded')).toBe('false');
                expect(events).toEqual(['input:false:true', 'change:false:true']);
                await runtime.whenRenderSettled(host);
                expect(required(host, 'input')).toBe(input);
                expect(events).toHaveLength(2);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};
