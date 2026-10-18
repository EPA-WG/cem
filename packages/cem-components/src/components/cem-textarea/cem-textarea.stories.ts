import { expect, within } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemComponent, cemComponentPlaygroundSource, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-textarea.xhtml?raw';

const meta = preview.meta({
    component: 'cem-textarea',
    title: 'CEM Components/cem-textarea',
    loaders: [async () => { await loadCemDeclaration('cem-textarea', declarationSource); return {}; }],
});

export const TextSelectionRequests = meta.story({
    render: () => '<cem-textarea label="Selected source" value="Select this source text" selection-request="initial-range" selection-start="7" selection-end="11"></cem-textarea>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const editor = within(host).getByRole('textbox', { name: 'Selected source' }) as HTMLTextAreaElement;
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([7, 11]);
        expect(document.activeElement).not.toBe(editor);
        host.setAttribute('selection-focus', ''); host.setAttribute('selection-request', 'navigate');
        await whenCemRendered(host);
        expect(document.activeElement).toBe(editor);
        editor.setSelectionRange(0, 0);
        host.setAttribute('busy', ''); await whenCemRendered(host);
        expect(host.querySelector('textarea')).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd]).toEqual([0, 0]);
        expect(editor.value).toBe('Select this source text');
    },
});

export const SelectionPropertyControls = meta.story({
    loaders: [async () => {
        const source = await cemComponentPlaygroundSource('cem-textarea');
        await Promise.all(['cem-field', 'cem-radio', 'cem-action'].map(loadCemComponent));
        const document = new DOMParser().parseFromString(source, 'text/html');
        const template = document.querySelector('cem-gallery-page > template')!.content.querySelector('main > cem-element > template') as HTMLTemplateElement;
        return { selectionPropertyMarkup: `<cem-element>${template.outerHTML}</cem-element>` };
    }],
    render: (_args, { loaded }) => String(loaded.selectionPropertyMarkup),
    play: async ({ canvasElement }) => {
        const declaration = canvasElement.querySelector('cem-element') as HTMLElement;
        const settle = async () => {
            await storybookCemRuntime().whenDeclarationSettled(declaration);
            const instance = declaration.querySelector('[data-cem-anonymous-instance]') as HTMLElement;
            await whenCemRendered(instance);
            await Promise.all([...canvasElement.querySelectorAll<HTMLElement>('cem-field, cem-textarea, cem-radio, cem-action')].map(whenCemRendered));
        };
        await settle();
        const editor = canvasElement.querySelector('[aria-label="Field preview"] textarea') as HTMLTextAreaElement;
        const request = within(canvasElement).getByRole('textbox', { name: 'selection-request' }) as HTMLInputElement;
        const start = within(canvasElement).getByRole('textbox', { name: 'selection-start' }) as HTMLInputElement;
        const end = within(canvasElement).getByRole('textbox', { name: 'selection-end' }) as HTMLInputElement;
        start.value = '1'; start.dispatchEvent(new Event('input', { bubbles: true }));
        end.value = '4'; end.dispatchEvent(new Event('input', { bubbles: true }));
        await settle();
        const direction = within(canvasElement).getByRole('radiogroup', { name: 'selection-direction' });
        within(direction).getByRole('radio', { name: 'backward' }).click();
        const focus = within(canvasElement).getByRole('radiogroup', { name: 'selection-focus' });
        within(focus).getByRole('radio', { name: 'present' }).click();
        await settle();
        request.focus(); request.value = 'navigate';
        request.dispatchEvent(new Event('input', { bubbles: true })); await settle();
        expect(document.activeElement).toBe(request);
        request.dispatchEvent(new Event('change', { bubbles: true })); await settle();
        expect(document.activeElement).toBe(editor);
        expect([editor.selectionStart, editor.selectionEnd, editor.selectionDirection]).toEqual([1, 4, 'backward']);
        expect(canvasElement.querySelector('[aria-label="Field preview"] textarea')).toBe(editor);
        expect(storybookCemRuntime().diagnosticsFor(declaration)).toEqual([]);
    },
});

export const AllAttributes = meta.story({
    render: () => '<cem-textarea></cem-textarea><p id="field-description">Description</p><p id="field-error">Error</p>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const input = within(host).getByRole('textbox', { name: 'Textarea' }) as HTMLTextAreaElement;
        expect(input.type).toBe('textarea');
        host.setAttribute('name', 'account');
        expect(input.hasAttribute('name')).toBe(false);
        expect(input.getAttribute('part')).toBe('control');
        expect(input.value).toBe('');
        expect(host.shadowRoot).toBeNull();
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        for (const [attribute, value, target] of [
            ['placeholder', 'Enter account', 'placeholder'],
            ['invalid', 'true', 'aria-invalid'], ['describedby', 'field-description', 'aria-describedby'],
            ['error', 'field-error', 'aria-errormessage'],
        ]) {
            host.setAttribute(attribute, value);
            await whenCemRendered(host);
            expect(input.getAttribute(target)).toBe(value);
            host.removeAttribute(attribute);
            await whenCemRendered(host);
            expect(input.hasAttribute(target)).toBe(false);
        }
        host.setAttribute('label', 'Account');
        host.setAttribute('value', 'a@example.com');
        await whenCemRendered(host);
        expect(input).toHaveAccessibleName('Account');
        expect(input.value).toBe('a@example.com');
        for (const attribute of ['label', 'value']) host.removeAttribute(attribute);
        await whenCemRendered(host);
        expect(input).toHaveAccessibleName('Textarea');
        expect(input.type).toBe('textarea');
        expect(input.value).toBe('');
        host.setAttribute('invalid', 'false');
        await whenCemRendered(host);
        expect(input.getAttribute('aria-invalid')).toBe('false');
        host.removeAttribute('invalid');
        for (const attribute of ['disabled', 'required', 'readonly', 'busy']) {
            for (const value of ['', 'false', 'true']) {
                host.setAttribute(attribute, value);
                await whenCemRendered(host);
                expect(host.querySelector('textarea')).toBe(input);
                if (attribute === 'busy') {
                    expect(input.getAttribute('aria-busy')).toBe('true');
                    expect(input.getAttribute('data-state')).toBe('loading');
                    expect(input.disabled).toBe(false);
                } else {
                    expect(input.hasAttribute(attribute)).toBe(true);
                }
            }
            host.removeAttribute(attribute);
            await whenCemRendered(host);
            expect(input.hasAttribute(attribute === 'busy' ? 'aria-busy' : attribute)).toBe(false);
        }
        host.setAttribute('hidden', 'false');
        await whenCemRendered(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.removeAttribute('hidden');
        host.setAttribute('class', 'custom-field');
        await whenCemRendered(host);
        expect(getComputedStyle(host).display).not.toBe('none');
        expect(input.hasAttribute('data-state')).toBe(false);
        expect(host.classList.contains('custom-field')).toBe(true);
    },
});

export const SlotsAndInput = meta.story({
    render: () => '<form><cem-textarea name="account" value="initial"><span slot="label">Account name</span><span slot="help">Help text</span></cem-textarea></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const input = within(host).getByRole('textbox', { name: 'Account name' }) as HTMLTextAreaElement;
        expect(host.querySelector('[slot="help"]')?.textContent).toBe('Help text');
        expect(new FormData((canvasElement.querySelector('form') as HTMLFormElement)).get('account')).toBe('initial');
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await userEvent.fill(input, 'edited\nsecond line');
        await whenCemRendered(host);
        expect(storybookCemRuntime().snapshotInstance(host).slices.value).toBe('edited\nsecond line');
        expect(new FormData((canvasElement.querySelector('form') as HTMLFormElement)).get('account')).toBe('edited\nsecond line');
        input.setSelectionRange(1, 3);
        const width = input.getBoundingClientRect().width;
        host.setAttribute('busy', '');
        await whenCemRendered(host);
        expect(host.querySelector('textarea')).toBe(input);
        expect(document.activeElement).toBe(input);
        expect(input.value).toBe('edited\nsecond line');
        expect(input.selectionStart).toBe(1);
        expect(input.selectionEnd).toBe(3);
        expect(input.getBoundingClientRect().width).toBe(width);
        host.removeAttribute('busy');
        await whenCemRendered(host);
        expect(input.hasAttribute('data-state')).toBe(false);
        expect(input.value).toBe('edited\nsecond line');
    },
    parameters: { docs: { description: { story: 'Native editing, focus and selection checks run only in the browser runner.' } } },
});

export const FormReset = meta.story({
    render: () => '<form><cem-textarea name="account" value="initial"></cem-textarea><button type="reset">Reset</button></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('textarea') as HTMLTextAreaElement);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await userEvent.fill(input, 'edited\nsecond line');
        await whenCemRendered(host);
        await userEvent.click((canvasElement.querySelector('button') as HTMLButtonElement));
        await whenCemRendered(host);
        expect(input.value).toBe('initial');
        host.setAttribute('busy', '');
        await whenCemRendered(host);
        expect(input.value).toBe('initial');
    },
    parameters: { docs: { description: { story: 'Native form reset checks run only in the browser runner.' } } },
});

export const ThemeStates = meta.story({
    render: () => '<section class="cem-theme-light"><cem-textarea label="Paint"></cem-textarea></section>',
    play: async ({ canvasElement }) => {
        const section = (canvasElement.querySelector('section') as HTMLElement);
        const host = section.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('textarea') as HTMLTextAreaElement);
        for (const theme of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
            section.className = theme;
            for (const indicator of [null, 'underline', 'outline', 'unsupported']) {
                if (indicator === null) host.removeAttribute('indicator');
                else host.setAttribute('indicator', indicator);
                for (const value of ['', 'Filled value']) {
                    host.setAttribute('value', value);
                    await whenCemRendered(host);
                    expect(input.value).toBe(value);
                    expect(input.matches(':focus')).toBe(false);
                    expect(getComputedStyle(input).borderWidth).toBe('0px');
                    expect(getComputedStyle(input).boxShadow).not.toBe('none');
                    const style = getComputedStyle(input);
                    expect(style.getPropertyValue('--cem-input-background-color').trim() === 'transparent').toBe(indicator === 'outline');
                    expect(style.backgroundColor === 'rgba(0, 0, 0, 0)').toBe(indicator === 'outline');
                    const width = parseFloat(style.getPropertyValue('--_cem-input-indicator-anchor-width'));
                    const offsets = style.boxShadow.match(/-?\d+(?:\.\d+)?px/g)?.slice(0, 4).map(parseFloat);
                    expect(offsets).toEqual(indicator === 'outline' ? [0, 0, 0, width] : [0, width, 0, 0]);
                    expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(
                        getComputedStyle(input).getPropertyValue('--cem-stroke-boundary').trim());
                    expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-focus-width').trim()).toBe(
                        getComputedStyle(input).getPropertyValue('--cem-stroke-none').trim());
                }
                const before = input.getBoundingClientRect();
                const retainedValue = input.value;
                host.setAttribute('indicator', indicator === 'outline' ? 'underline' : 'outline');
                await whenCemRendered(host);
                expect(host.querySelector('[part="control"]')).toBe(input);
                expect(input.value).toBe(retainedValue);
                expect(getComputedStyle(input).backgroundColor === 'rgba(0, 0, 0, 0)').toBe(indicator !== 'outline');
                const after = input.getBoundingClientRect();
                expect(after.width).toBe(before.width);
                expect(after.height).toBe(before.height);
                if (indicator === null) host.removeAttribute('indicator');
                else host.setAttribute('indicator', indicator);
                await whenCemRendered(host);
                host.setAttribute('busy', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).boxShadow).not.toBe('none');
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(
                    getComputedStyle(input).getPropertyValue('--cem-stroke-pending').trim());
                host.setAttribute('disabled', '');
                await whenCemRendered(host);
                expect(input.disabled).toBe(true);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(
                    getComputedStyle(input).getPropertyValue('--cem-stroke-none').trim());
                host.removeAttribute('disabled');
                host.removeAttribute('busy');
                await whenCemRendered(host);
            }
        }
    },
});

export const InteractionPaint = meta.story({
    render: () => '<button>Before</button><cem-textarea label="Interactive"></cem-textarea>',
    parameters: { docs: { description: { story: 'Trusted hover and keyboard focus checks run only in the browser runner.' } } },
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('textarea') as HTMLTextAreaElement);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const anchor = () => getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-color').trim();
        const checkBackground = (indicator: string) => expect(getComputedStyle(input).backgroundColor === 'rgba(0, 0, 0, 0)').toBe(indicator === 'outline');
        const token = (name: string) => getComputedStyle(input).getPropertyValue(name).trim();
        for (const theme of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
            host.className = theme;
            for (const indicator of ['underline', 'outline']) {
                host.setAttribute('indicator', indicator);
                for (const name of ['readonly', 'busy', 'invalid', 'disabled']) host.removeAttribute(name);
                input.blur();
                await whenCemRendered(host);
                await userEvent.hover((canvasElement.querySelector('button') as HTMLButtonElement));
                expect(input.matches(':hover')).toBe(false);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(getComputedStyle(input).boxShadow).not.toBe('none');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-color'));
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(token('--cem-stroke-boundary'));
                await userEvent.hover(input);
                expect(getComputedStyle(input).boxShadow).not.toBe('none');
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(token('--cem-stroke-boundary'));
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-hover-color'));
                host.setAttribute('readonly', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-readonly-color'));
                host.setAttribute('busy', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-pending-color'));
                host.setAttribute('invalid', 'true');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-invalid-hover-color'));
                await userEvent.hover((canvasElement.querySelector('button') as HTMLButtonElement));
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-invalid-color'));
                (canvasElement.querySelector('button') as HTMLButtonElement).focus();
                await userEvent.tab();
                expect(document.activeElement).toBe(input);
                expect(input.matches(':focus-visible')).toBe(true);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(getComputedStyle(input).boxShadow).not.toBe('none');
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-focus-width').trim()).toBe(token('--cem-zebra-strip-size'));
                host.setAttribute('disabled', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                checkBackground(indicator);
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-disabled-color'));
            }
        }
        expect(storybookCemRuntime().snapshotInstance(host).slices).not.toHaveProperty('busy');
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const NativeConstraintsAndFormOwnership = meta.story({
    render: () => '<form id="notes-owner"><fieldset><cem-textarea name="notes" label="Notes" rows="4" cols="30" minlength="3" maxlength="120" autocomplete="off" required></cem-textarea></fieldset><button type="reset">Reset notes</button><button type="submit">Save notes</button></form><form id="other-notes-owner"></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement & {
            value: string; defaultValue: string; checkValidity(): boolean; setCustomValidity(message: string): void;
        };
        const form = canvasElement.querySelector('#notes-owner') as HTMLFormElement;
        const other = canvasElement.querySelector('#other-notes-owner') as HTMLFormElement;
        const fieldset = form.querySelector('fieldset') as HTMLFieldSetElement;
        await whenCemRendered(host);
        const control = within(host).getByRole('textbox', { name: 'Notes' }) as HTMLTextAreaElement;
        expect(control.form).toBeNull();
        expect(control.rows).toBe(4);
        expect(control.cols).toBe(30);
        expect(control.minLength).toBe(3);
        expect(control.maxLength).toBe(120);
        expect(control.autocomplete).toBe('off');
        expect(host.checkValidity()).toBe(false);
        host.value = 'first\nsecond';
        await whenCemRendered(host);
        expect(control.value).toBe('first\nsecond');
        expect(host.checkValidity()).toBe(true);
        expect(new FormData(form).getAll('notes')).toEqual(['first\nsecond']);
        host.setCustomValidity('Review these notes');
        expect(host.checkValidity()).toBe(false);
        host.setCustomValidity('');
        fieldset.disabled = true;
        await whenCemRendered(host);
        expect(control.disabled).toBe(true);
        expect(new FormData(form).has('notes')).toBe(false);
        fieldset.disabled = false;
        await whenCemRendered(host);
        expect(control.disabled).toBe(false);
        host.setAttribute('form', 'other-notes-owner');
        await whenCemRendered(host);
        expect(new FormData(form).has('notes')).toBe(false);
        expect(new FormData(other).getAll('notes')).toEqual(['first\nsecond']);
        host.defaultValue = 'reset\nvalue';
        await whenCemRendered(host);
        host.value = 'dirty\nvalue';
        await whenCemRendered(host);
        other.reset();
        await whenCemRendered(host);
        expect(control.value).toBe('reset\nvalue');
        for (const attr of ['rows', 'cols', 'minlength', 'maxlength', 'autocomplete']) host.removeAttribute(attr);
        await whenCemRendered(host);
        expect(control.rows).toBe(2);
        expect(control.cols).toBe(20);
        expect(control.hasAttribute('minlength')).toBe(false);
        expect(control.hasAttribute('maxlength')).toBe(false);
        expect(control.hasAttribute('autocomplete')).toBe(false);
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const EnterEditsWithoutSubmitting = meta.story({
    render: () => '<form><cem-textarea name="notes" value="first"></cem-textarea><button type="submit">Save</button></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const control = host.querySelector('textarea') as HTMLTextAreaElement;
        const form = canvasElement.querySelector('form') as HTMLFormElement;
        let submissions = 0;
        form.addEventListener('submit', event => { event.preventDefault(); submissions++; });
        await userEvent.click(control);
        control.setSelectionRange(control.value.length, control.value.length);
        await userEvent.keyboard('{Enter}second');
        await whenCemRendered(host);
        expect(control.value).toBe('first\nsecond');
        expect(submissions).toBe(0);
        const snapshot = storybookCemRuntime().snapshotInstance(host);
        expect(snapshot.slices.value).toBe('first\nsecond');
        expect(snapshot.eventPayloads.value?.target?.value).toBe('first\nsecond');
        await userEvent.click(form.querySelector('button') as HTMLButtonElement);
        expect(submissions).toBe(1);
        expect(new FormData(form).getAll('notes')).toEqual(['first\nsecond']);
    },
});

export const WhitespaceAndValueUpdates = meta.story({
    render: () => '<cem-textarea value="&#10;  first line&#10;second line  &#10;"></cem-textarea>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-textarea') as HTMLElement;
        await whenCemRendered(host);
        const control = host.querySelector('textarea') as HTMLTextAreaElement;
        expect(control.value).toBe('\n  first line\nsecond line  \n');
        host.setAttribute('value', '\n  replacement\n');
        await whenCemRendered(host);
        expect(control.value).toBe('\n  replacement\n');
        host.setAttribute('value', '');
        await whenCemRendered(host);
        expect(control.value).toBe('');
        expect(host.querySelector('textarea')).toBe(control);
    },
});

export const RequiredMarker = meta.story({
    render: () => '<cem-textarea label="Email"></cem-textarea><cem-textarea required label="Fallback"><span slot="label">Account name</span></cem-textarea>',
    play: async ({ canvasElement }) => {
        const hosts = canvasElement.querySelectorAll('cem-textarea');
        const host = hosts[0] as HTMLElement;
        const projected = hosts[1] as HTMLElement;
        await whenCemRendered(host);
        await whenCemRendered(projected);
        const input = within(host).getByRole('textbox', { name: 'Email' }) as HTMLTextAreaElement;
        expect(host.querySelector('[part="required-marker"]')).toBeNull();
        expect(input.required).toBe(false);
        const labeledInput = within(projected).getByRole('textbox', { name: 'Account name' }) as HTMLTextAreaElement;
        expect(labeledInput.required).toBe(true);
        expect(projected.querySelector('[part="required-marker"]')?.textContent).toBe('*');
        for (const theme of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
            host.className = theme;
            for (const indicator of ['underline', 'outline']) {
                host.setAttribute('indicator', indicator);
                for (const value of ['', 'false', 'true']) {
                    host.setAttribute('required', value);
                    await whenCemRendered(host);
                    const marker = host.querySelector('[part="required-marker"]') as HTMLElement;
                    expect(marker).not.toBeNull();
                    expect(marker.textContent).toBe('*');
                    expect(marker.getAttribute('aria-hidden')).toBe('true');
                    expect(getComputedStyle(marker).color).toBe(getComputedStyle(marker.parentElement!).color);
                    expect(input).toHaveAccessibleName('Email');
                    expect(input.required).toBe(true);
                    expect(input.validity.valueMissing).toBe(true);
                    host.setAttribute('value', 'Filled value');
                    await whenCemRendered(host);
                    expect(input.validity.valueMissing).toBe(false);
                    for (const state of ['disabled', 'readonly', 'busy']) {
                        host.setAttribute(state, '');
                        await whenCemRendered(host);
                        expect(host.querySelectorAll('[part="required-marker"]').length).toBe(1);
                        expect(input.required).toBe(true);
                        host.removeAttribute(state);
                    }
                    host.removeAttribute('required');
                    host.removeAttribute('value');
                    await whenCemRendered(host);
                    expect(host.querySelector('[part="required-marker"]')).toBeNull();
                    expect(input.required).toBe(false);
                    expect(input).toHaveAccessibleName('Email');
                    expect(host.querySelector('[part="control"]')).toBe(input);
                }
            }
        }
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const RequiredMarkerOverrides = meta.story({
    render: () => '<cem-textarea required label="Email"></cem-textarea><cem-textarea label="Account" required-marker="Attribute"><strong slot="required-marker">Custom</strong></cem-textarea>',
    play: async ({ canvasElement }) => {
        const hosts = canvasElement.querySelectorAll('cem-textarea');
        const host = hosts[0] as HTMLElement;
        const projected = hosts[1] as HTMLElement;
        await whenCemRendered(host);
        await whenCemRendered(projected);
        const input = within(host).getByRole('textbox', { name: 'Email' }) as HTMLTextAreaElement;
        expect(host.querySelector('[part="required-marker"]')?.textContent).toBe('*');
        for (const value of ['(required)', '✦', '']) {
            host.setAttribute('required-marker', value);
            await whenCemRendered(host);
            expect(host.querySelector('[part="required-marker"]')?.textContent).toBe(value);
            expect(input.required).toBe(true);
            expect(input).toHaveAccessibleName('Email');
        }
        host.removeAttribute('required-marker');
        await whenCemRendered(host);
        expect(host.querySelector('[part="required-marker"]')?.textContent).toBe('*');
        host.removeAttribute('required');
        host.setAttribute('required-marker', 'Hidden');
        await whenCemRendered(host);
        expect(host.querySelector('[part="required-marker"]')).toBeNull();
        expect(input.required).toBe(false);
        expect(projected.querySelector('[part="required-marker"]')).toBeNull();
        expect(projected.querySelector('[slot="required-marker"]')).toBeNull();
        for (const required of ['', 'false', 'true']) {
            projected.setAttribute('required', required);
            await whenCemRendered(projected);
            const marker = projected.querySelector('[part="required-marker"]') as HTMLElement;
            expect(marker.textContent).toBe('Custom');
            expect(marker.querySelector('strong[slot="required-marker"]')).not.toBeNull();
            expect(marker.getAttribute('aria-hidden')).toBe('true');
            expect(within(projected).getByRole('textbox')).toHaveAccessibleName('Account');
            projected.setAttribute('required-marker', 'Changed attribute');
            await whenCemRendered(projected);
            expect(marker.textContent).toBe('Custom');
            projected.removeAttribute('required');
            await whenCemRendered(projected);
            expect(projected.querySelector('[part="required-marker"]')).toBeNull();
            expect(projected.querySelector('[slot="required-marker"]')).toBeNull();
        }
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        expect(storybookCemRuntime().diagnosticsFor(projected)).toEqual([]);
    },
});

export const FilledPanel = meta.story({
    render: () => '<div style="inline-size:600px;block-size:400px"><cem-textarea fit="fill" label="Current file" value="Retained source"><span slot="help">Saved explicitly</span></cem-textarea></div>',
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('div')!;
        const host = canvasElement.querySelector<HTMLElement>('cem-textarea')!;
        await whenCemRendered(host);
        const input = host.querySelector('textarea')!;
        input.value = 'Unsaved source'; input.dispatchEvent(new Event('input', { bubbles: true })); await whenCemRendered(host);
        input.focus(); input.setSelectionRange(2, 7);
        for (const width of [600, 240]) {
            container.style.inlineSize = `${width}px`;
            const rect = input.getBoundingClientRect();
            expect(rect.width).toBe(width); expect(rect.height).toBeGreaterThan(300);
            expect(host.getBoundingClientRect().height).toBe(400);
            expect(host.scrollWidth).toBe(width);
        }
        host.setAttribute('label', 'Selected file'); await whenCemRendered(host);
        expect(host.querySelector('textarea')).toBe(input);
        expect(input.value).toBe('Unsaved source'); expect([input.selectionStart, input.selectionEnd]).toEqual([2, 7]);
        expect(document.activeElement).toBe(input);
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});
