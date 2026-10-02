import { expect, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-textarea.xhtml?raw';

const meta = preview.meta({
    component: 'cem-textarea',
    title: 'CEM Components/cem-textarea',
    loaders: [async () => { await loadCemDeclaration('cem-textarea', declarationSource); return {}; }],
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
            for (const indicator of ['underline', 'outline', 'unsupported']) {
                host.setAttribute('indicator', indicator);
                for (const value of ['', 'Filled value']) {
                    host.setAttribute('value', value);
                    await whenCemRendered(host);
                    expect(input.value).toBe(value);
                    expect(input.matches(':focus')).toBe(false);
                    expect(getComputedStyle(input).borderWidth).toBe('0px');
                    expect(getComputedStyle(input).boxShadow).not.toBe('none');
                    expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(
                        getComputedStyle(input).getPropertyValue('--cem-stroke-boundary').trim());
                    expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-focus-width').trim()).toBe(
                        getComputedStyle(input).getPropertyValue('--cem-stroke-none').trim());
                }
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
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-color'));
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(token('--cem-stroke-boundary'));
                await userEvent.hover(input);
                expect(getComputedStyle(input).boxShadow).not.toBe('none');
                expect(getComputedStyle(input).getPropertyValue('--_cem-input-indicator-anchor-width').trim()).toBe(token('--cem-stroke-boundary'));
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-hover-color'));
                host.setAttribute('readonly', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-readonly-color'));
                host.setAttribute('busy', '');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-pending-color'));
                host.setAttribute('invalid', 'true');
                await whenCemRendered(host);
                expect(getComputedStyle(input).borderWidth).toBe('0px');
                expect(anchor()).toBe(token('--cem-input-indicator-anchor-invalid-hover-color'));
                await userEvent.hover((canvasElement.querySelector('button') as HTMLButtonElement));
                expect(getComputedStyle(input).borderWidth).toBe('0px');
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
