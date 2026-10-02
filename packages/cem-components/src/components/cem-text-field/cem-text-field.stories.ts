import { expect, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-text-field.xhtml?raw';

const meta = preview.meta({
    component: 'cem-text-field',
    title: 'CEM Components/cem-text-field',
    loaders: [async () => { await loadCemDeclaration('cem-text-field', declarationSource); return {}; }],
});

export const AllAttributes = meta.story({
    render: () => '<cem-text-field></cem-text-field><p id="field-description">Description</p><p id="field-error">Error</p>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-text-field') as HTMLElement;
        await whenCemRendered(host);
        const input = within(host).getByRole('textbox', { name: 'Text field' }) as HTMLInputElement;
        expect(input.type).toBe('text');
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
        host.setAttribute('type', 'email');
        host.setAttribute('value', 'a@example.com');
        await whenCemRendered(host);
        expect(input).toHaveAccessibleName('Account');
        expect(input.type).toBe('email');
        expect(input.value).toBe('a@example.com');
        for (const attribute of ['label', 'type', 'value']) host.removeAttribute(attribute);
        await whenCemRendered(host);
        expect(input).toHaveAccessibleName('Text field');
        expect(input.type).toBe('text');
        expect(input.value).toBe('');
        host.setAttribute('invalid', 'false');
        await whenCemRendered(host);
        expect(input.getAttribute('aria-invalid')).toBe('false');
        host.removeAttribute('invalid');
        for (const attribute of ['disabled', 'required', 'readonly', 'busy']) {
            for (const value of ['', 'false', 'true']) {
                host.setAttribute(attribute, value);
                await whenCemRendered(host);
                expect(host.querySelector('input')).toBe(input);
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
    render: () => '<form><cem-text-field name="account" value="initial"><span slot="label">Account name</span><span slot="help">Help text</span></cem-text-field></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-text-field') as HTMLElement;
        await whenCemRendered(host);
        const input = within(host).getByRole('textbox', { name: 'Account name' }) as HTMLInputElement;
        expect(host.querySelector('[slot="help"]')?.textContent).toBe('Help text');
        expect(new FormData((canvasElement.querySelector('form') as HTMLFormElement)).get('account')).toBe('initial');
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await userEvent.fill(input, 'edited');
        await whenCemRendered(host);
        expect(storybookCemRuntime().snapshotInstance(host).slices.value).toBe('edited');
        expect(new FormData((canvasElement.querySelector('form') as HTMLFormElement)).get('account')).toBe('edited');
        input.setSelectionRange(1, 3);
        const width = input.getBoundingClientRect().width;
        host.setAttribute('busy', '');
        await whenCemRendered(host);
        expect(host.querySelector('input')).toBe(input);
        expect(document.activeElement).toBe(input);
        expect(input.value).toBe('edited');
        expect(input.selectionStart).toBe(1);
        expect(input.selectionEnd).toBe(3);
        expect(input.getBoundingClientRect().width).toBe(width);
        host.removeAttribute('busy');
        await whenCemRendered(host);
        expect(input.hasAttribute('data-state')).toBe(false);
        expect(input.value).toBe('edited');
    },
    parameters: { docs: { description: { story: 'Native editing, focus and selection checks run only in the browser runner.' } } },
});

export const FormReset = meta.story({
    render: () => '<form><cem-text-field name="account" value="initial"></cem-text-field><button type="reset">Reset</button></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-text-field') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('input') as HTMLInputElement);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        await userEvent.fill(input, 'edited');
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
    render: () => '<section class="cem-theme-light"><cem-text-field label="Paint"></cem-text-field></section>',
    play: async ({ canvasElement }) => {
        const section = (canvasElement.querySelector('section') as HTMLElement);
        const host = section.querySelector('cem-text-field') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('input') as HTMLInputElement);
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
    render: () => '<button>Before</button><cem-text-field label="Interactive"></cem-text-field>',
    parameters: { docs: { description: { story: 'Trusted hover and keyboard focus checks run only in the browser runner.' } } },
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-text-field') as HTMLElement;
        await whenCemRendered(host);
        const input = (host.querySelector('input') as HTMLInputElement);
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

export const RequiredMarker = meta.story({
    render: () => '<cem-text-field label="Email"></cem-text-field><cem-text-field required label="Fallback"><span slot="label">Account name</span></cem-text-field>',
    play: async ({ canvasElement }) => {
        const hosts = canvasElement.querySelectorAll('cem-text-field');
        const host = hosts[0] as HTMLElement;
        const projected = hosts[1] as HTMLElement;
        await whenCemRendered(host);
        await whenCemRendered(projected);
        const input = within(host).getByRole('textbox', { name: 'Email' }) as HTMLInputElement;
        expect(host.querySelector('[part="required-marker"]')).toBeNull();
        expect(input.required).toBe(false);
        const labeledInput = within(projected).getByRole('textbox', { name: 'Account name' }) as HTMLInputElement;
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
    render: () => '<cem-text-field required label="Email"></cem-text-field><cem-text-field label="Account" required-marker="Attribute"><strong slot="required-marker">Custom</strong></cem-text-field>',
    play: async ({ canvasElement }) => {
        const hosts = canvasElement.querySelectorAll('cem-text-field');
        const host = hosts[0] as HTMLElement;
        const projected = hosts[1] as HTMLElement;
        await whenCemRendered(host);
        await whenCemRendered(projected);
        const input = within(host).getByRole('textbox', { name: 'Email' }) as HTMLInputElement;
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
