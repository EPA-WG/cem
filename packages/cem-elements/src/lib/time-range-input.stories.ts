import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemStoryRuntime } from '../../.storybook/preview.js';

const meta: Meta = { title: 'CEM Elements/Time And Range Input', tags: ['test'] };
export default meta;
type Story = StoryObj;
const timeDeclaration = `<cem-element capability="time-input"><template type="text/cem-ml">
{div @part=root | {slot @name=input}{slot @name=toggle}
 {div @part=popup @role=listbox @popover=manual @id={datadom.slices.listboxId} |
  {cem:for-each @select=datadom.slices.options @as=option |
   {div @part=option @role=option @id={option.id} @data-option-index={option.index} @data-value={option.value} @aria-selected={option.selected} @aria-disabled={option.disabled} | {$option.label}}
  }
 }
}
</template></cem-element>`;
const rangeDeclaration = `<cem-element capability="range-input"><template type="text/cem-ml">
{div @part=root | {div @part=visual @aria-hidden=true | {span @part="value single-value"}{span @part="value start-value"}{span @part="value end-value"}}
 {div @part=inputs | {cem:for-each @select=datadom.slices.thumbSlots @as=thumb | {slot @name={thumb}}}}
}
</template></cem-element>`;
function required<T extends Element>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing native input fixture ${selector}`);
    return node;
}
export const TimeOwnershipAndLifecycle: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `time-input-${crypto.randomUUID()}`, source = await declare(timeDeclaration, tag);
            const form = document.createElement('form');
            form.innerHTML = `<fieldset><${tag} min="09:00" max="11:00" interval="30" required="false"><input slot="input" type="text" name="time" value="09:30" aria-label="Meeting"><button type="button" slot="toggle"><strong>Choose</strong></button></${tag}></fieldset>`;
            canvasElement.append(form);
            const host = required<HTMLElement & { expanded: boolean }>(form, tag);
            try {
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), popup = required<HTMLElement>(host, '[part=popup]');
                expect(input.required).toBe(true); expect(new FormData(form).getAll('time')).toEqual(['09:30']);
                const events: string[] = [];
                for (const name of ['input', 'change']) input.addEventListener(name, event => events.push(`${event.type}:${input.value}:${popup.matches(':popover-open')}:${document.activeElement === input}`));
                for (const modifiers of [{ isComposing: true }, { keyCode: 229 }, { ctrlKey: true }, { altKey: true }, { metaKey: true }, { shiftKey: true }]) {
                    const event = new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true, cancelable: true, ...modifiers });
                    input.dispatchEvent(event); expect(event.defaultPrevented, JSON.stringify(modifiers)).toBe(false); expect(host.expanded).toBe(false);
                    input.dispatchEvent(new KeyboardEvent('keyup', { key: 'ArrowDown', bubbles: true }));
                }
                required<HTMLElement>(host, 'strong').click(); await runtime.whenRenderSettled(host);
                expect(popup.matches(':popover-open')).toBe(true); expect(popup).toHaveAccessibleName('Meeting options');
                input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true }));
                input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', repeat: true, bubbles: true }));
                expect(host.expanded).toBe(true); expect(input.value).toBe('09:30'); expect(events).toEqual([]);
                input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
                required<HTMLElement>(popup, '[data-value="10:00"]').click();
                await runtime.whenRenderSettled(host);
                expect(events).toEqual(['input:10:00:false:true', 'change:10:00:false:true']);
                expect(new FormData(form).getAll('time')).toEqual(['10:00']);
                host.expanded = true; await runtime.whenRenderSettled(host);
                required<HTMLFieldSetElement>(form, 'fieldset').disabled = true;
                await waitFor(() => expect(popup.matches(':popover-open')).toBe(false));
                host.expanded = true; expect(host.expanded).toBe(false);
                required<HTMLFieldSetElement>(form, 'fieldset').disabled = false;
                input.readOnly = true; host.expanded = true; expect(host.expanded).toBe(false); input.readOnly = false;
                form.reset(); await waitFor(() => expect(input.value).toBe('09:30')); await runtime.whenRenderSettled(host);
                await waitFor(() => expect(required(popup, '[data-value="09:30"]')).toHaveAttribute('aria-selected', 'true'));
                events.length = 0; input.value = '11:00'; host.expanded = true; await runtime.whenRenderSettled(host);
                expect(input.getAttribute('aria-activedescendant')).toBe(required(popup, '[data-value="11:00"]').id);
                host.remove(); required(form, 'fieldset').append(host); await runtime.whenRenderSettled(host);
                expect(required(host, 'input')).toBe(input); expect(host.expanded).toBe(false); expect(popup.matches(':popover-open')).toBe(false);
                expect(events).toEqual([]); expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { form.remove(); source.remove(); scope.dispose(); }
        }
    },
};
export const RangeResetAndStrictOwners: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `range-input-${crypto.randomUUID()}`, source = await declare(rangeDeclaration, tag);
            const form = document.createElement('form');
            form.innerHTML = `<${tag} min="0" max="100" step="5"><input slot="start" type="range" name="start" value="25" aria-label="Minimum"><input slot="end" type="range" name="end" value="75" aria-label="Maximum"></${tag}>`;
            canvasElement.append(form);
            const host = required<HTMLElement>(form, tag);
            try {
                await runtime.whenRenderSettled(host);
                const start = required<HTMLInputElement>(host, 'input[slot=start]'), end = required<HTMLInputElement>(host, 'input[slot=end]');
                const owner = required<HTMLElement>(host, '[part=root]');
                expect(owner.dataset.mode).toBe('range'); expect([...new FormData(form)]).toEqual([['start', '25'], ['end', '75']]);
                const events: string[] = [];
                start.addEventListener('input', () => events.push(`start:${start.value}`)); end.addEventListener('change', () => events.push(`end:${end.value}`));
                start.value = '90'; start.dispatchEvent(new Event('input', { bubbles: true })); expect(events).toEqual(['start:75']);
                end.value = '20'; end.dispatchEvent(new Event('change', { bubbles: true })); expect(events).toEqual(['start:75', 'end:75']);
                form.addEventListener('reset', event => event.preventDefault(), { once: true }); form.reset();
                await new Promise<void>(resolve => window.setTimeout(resolve, 0));
                expect(start.value).toBe('75'); expect(owner.style.getPropertyValue('--_cem-range-start-position')).toBe('75%');
                form.reset(); await waitFor(() => expect(owner.style.getPropertyValue('--_cem-range-start-position')).toBe('25%'));
                expect(start.defaultValue).toBe('25'); expect(end.defaultValue).toBe('75');
                expect(required(host, '[part~="start-value"]')).toHaveTextContent('25');
                expect(events).toEqual(['start:75', 'end:75']);
                host.setAttribute('disabled', 'false'); await runtime.whenRenderSettled(host); expect(start.disabled).toBe(true); expect(end.disabled).toBe(true); expect([...new FormData(form)]).toEqual([]);
                host.removeAttribute('disabled'); await runtime.whenRenderSettled(host);
                host.remove(); form.append(host); await runtime.whenRenderSettled(host); expect(required(host, 'input[slot=start]')).toBe(start);
                expect(events).toEqual(['start:75', 'end:75']); expect(runtime.diagnosticsFor(host)).toEqual([]);
                const defaults = document.createElement(tag); defaults.setAttribute('min', '-10');
                defaults.innerHTML = '<input slot="single" type="range" value="40" aria-label="Default bounds">'; form.append(defaults); await runtime.whenRenderSettled(defaults);
                expect(required<HTMLInputElement>(defaults, 'input').max).toBe('90'); defaults.remove();
                const reversed = document.createElement(tag);
                reversed.innerHTML = '<input slot="end" type="range" value="75" aria-label="Maximum first"><input slot="start" type="range" value="25" aria-label="Minimum second">';
                form.append(reversed); await runtime.whenRenderSettled(reversed);
                expect([...reversed.querySelectorAll('[part=inputs] > input')].map(input => input.getAttribute('slot'))).toEqual(['end', 'start']); reversed.remove();
                for (const payload of ['<input slot="single" type="range"><input type="text">', '<div><input slot="single" type="range"></div>', '<input slot="start" type="range"><input slot="start" type="range">']) {
                    const malformed = document.createElement(tag); malformed.innerHTML = payload; form.append(malformed); await runtime.whenRenderSettled(malformed);
                    expect(required<HTMLElement>(malformed, '[part=root]').dataset.mode).toBe('invalid'); malformed.remove();
                }
            } finally { form.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const ProjectedNativeThumbScope: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `native-thumb-scope-${crypto.randomUUID()}`;
            const source = await declare(`<cem-element><template type="text/cem-ml">
{style |\`\`\`
:scope { display:block; inline-size:320px; }
input[slot=thumb] { appearance:none; inline-size:100%; block-size:48px; margin:0; padding:0; pointer-events:none; }
:where(input[slot=thumb])::-webkit-slider-runnable-track { block-size:4px; }
:where(input[slot=thumb])::-webkit-slider-thumb { appearance:none; inline-size:48px; block-size:48px; margin-block-start:-22px; pointer-events:auto; }
[part=probe] { background-color:rgb(255, 0, 0); }
\`\`\`}
{slot @name=thumb}{span @part=probe | Owned}{slot @name=content}
</template></cem-element>`, tag);
            const host = document.createElement(tag);
            host.innerHTML = '<input type="range" slot="thumb" value="50" aria-label="Native thumb"><span slot="content"><span part="probe">Projected descendant</span></span>';
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'); input.scrollIntoView({ block: 'center' });
                const rect = input.getBoundingClientRect();
                expect(document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2)).toBe(input);
                expect(getComputedStyle(required(host, ':scope > [part=probe]')).backgroundColor).toBe('rgb(255, 0, 0)');
                expect(getComputedStyle(required(host, '[slot=content] > [part=probe]')).backgroundColor).toBe('rgba(0, 0, 0, 0)');
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const RangeAuthoredModeChanges: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `range-modes-${crypto.randomUUID()}`, parentTag = `range-parent-${crypto.randomUUID()}`;
            const source = await declare(rangeDeclaration, tag);
            const parentSource = await declare(`<cem-element><template type="text/cem-ml">
{slice @name=mode | single}
{form | {${tag} |
 {input @slot={if datadom.slices.mode == "range" {"start"} else {"single"}} @type=range @name=start @value=25 @aria-label=Start}
 {cem:if @test='datadom.slices.mode == "range"' | {input @slot=end @type=range @name=end @value=75 @aria-label=End}}
}}
</template></cem-element>`, parentTag);
            const parent = document.createElement(parentTag); canvasElement.append(parent);
            try {
                await runtime.whenRenderSettled(parent);
                const host = required<HTMLElement>(parent, tag); await runtime.whenRenderSettled(host);
                const first = required<HTMLInputElement>(host, 'input'); first.value = '30';
                for (const pair of [true, false, true]) {
                    runtime.setInstanceSlices(parent, { mode: pair ? 'range' : 'single' }); await runtime.whenRenderSettled(parent); await runtime.whenRenderSettled(host);
                    expect([...host.querySelectorAll('input')].map(input => input.slot)).toEqual(pair ? ['start', 'end'] : ['single']);
                    expect(required(host, 'input')).toBe(first); expect(first.value).toBe('30');
                    expect(runtime.snapshotInstance(host)?.slices.mode).toBe(pair ? 'range' : 'single');
                    expect([...new FormData(required<HTMLFormElement>(parent, 'form'))]).toEqual(pair ? [['start', '30'], ['end', '75']] : [['start', '30']]);
                }
                expect(runtime.diagnosticsFor(host)).toEqual([]); expect(runtime.diagnosticsFor(parent)).toEqual([]);
            } finally { parent.remove(); parentSource.remove(); source.remove(); scope.dispose(); }
        }
    },
};

export const NativeAvailabilityBeforeRendering: Story = {
    render: () => document.createElement('section'),
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(canvasElement, fallback);
            const tag = `time-availability-${crypto.randomUUID()}`, source = await declare(timeDeclaration, tag);
            const host = document.createElement(tag); host.setAttribute('disabled', 'false');
            host.innerHTML = '<input slot="input" type="text" value="09:30" aria-label="Meeting"><button slot="toggle" type="button">Choose</button>';
            canvasElement.append(host);
            try {
                await runtime.whenRenderSettled(host);
                const input = required<HTMLInputElement>(host, 'input'), toggle = required<HTMLButtonElement>(host, 'button');
                const events: string[] = [];
                for (const type of ['input', 'change']) input.addEventListener(type, () => events.push(type));
                expect(input.disabled).toBe(true); expect(toggle.disabled).toBe(true);
                for (const disabled of [false, true, false]) {
                    host.toggleAttribute('disabled', disabled);
                    // Availability is a native owner update at mutation delivery,
                    // independent of the asynchronous option-list publication.
                    await Promise.resolve();
                    expect(input.disabled).toBe(disabled); expect(toggle.disabled).toBe(disabled);
                    expect(required(host, 'input')).toBe(input);
                    await runtime.whenRenderSettled(host);
                }
                input.click();
                expect(required<HTMLElement>(host, '[part=popup]').matches(':popover-open')).toBe(true);
                expect(input.getAttribute('aria-expanded')).toBe('true');
                host.setAttribute('disabled', ''); await Promise.resolve();
                expect(required<HTMLElement>(host, '[part=popup]').matches(':popover-open')).toBe(false);
                expect(input.value).toBe('09:30'); expect(events).toEqual([]);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { host.remove(); source.remove(); scope.dispose(); }
        }
    },
};
