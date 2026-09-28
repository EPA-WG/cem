import { expect, userEvent, within } from 'storybook/test';

import preview, {
    loadCemDeclaration,
    whenCemRendered,
} from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-select.xhtml?raw';

interface CemSelectElement extends HTMLElement {
    disabled: boolean;
    form: HTMLFormElement | null;
    multiple: boolean;
    required: boolean;
    size: number;
    type: 'select-multiple' | 'select-one';
    value: string;
    selectedValues: string[];
    validity: ValidityState;
    validationMessage: string;
    checkValidity(): boolean;
    setSelectedValues(values: readonly string[]): void;
}

const meta = preview.meta({
    component: 'cem-select',
    title: 'CEM Components/cem-select',
    loaders: [async () => {
        await loadCemDeclaration('cem-select', declarationSource);
        return {};
    }],
});

export const PopupStacking = meta.story({
    render: () => `<section><cem-select label="First"><cem-option value="one">One</cem-option><cem-option value="two">Two</cem-option></cem-select><cem-select label="Second"><cem-option value="one">One</cem-option><cem-option value="two">Two</cem-option></cem-select></section>`,
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('section')!;
        const hosts = [...container.querySelectorAll<CemSelectElement>('cem-select')];
        for (const host of hosts) await whenCemRendered(host);
        const host = hosts[0];
        const control = host.querySelector<HTMLElement>('[role=combobox]')!;
        await userEvent.click(control);
        await whenCemRendered(host);
        const popup = host.querySelector<HTMLElement>('[part~=popup]')!;
        await expect(popup).not.toBeNull();
        await expect(getComputedStyle(popup).zIndex).toBe('1');
        container.style.setProperty('--cem-select-popup-z-index', '7');
        await expect(getComputedStyle(popup).zIndex).toBe('7');
        host.style.setProperty('--cem-select-popup-z-index', '9');
        await whenCemRendered(host);
        await expect(getComputedStyle(popup).zIndex).toBe('9');
        await expect(getComputedStyle(hosts[1]).getPropertyValue('--cem-select-popup-z-index').trim()).toBe('7');
        host.style.removeProperty('--cem-select-popup-z-index');
        await whenCemRendered(host);
        await expect(getComputedStyle(popup).zIndex).toBe('7');
        const option = within(popup).getByRole('option', { name: 'Two' });
        if (import.meta.env.MODE === 'test') {
            const { userEvent: native } = await import('vitest/browser');
            await native.click(option);
        } else {
            await userEvent.click(option);
        }
        await whenCemRendered(host);
        await expect(host.value).toBe('two');
        container.style.removeProperty('--cem-select-popup-z-index');
        await userEvent.click(control);
        await whenCemRendered(host);
        await expect(getComputedStyle(host.querySelector<HTMLElement>('[part~=popup]')!).zIndex).toBe('1');
    },
});

export const IndicatorThemeOverrides = meta.story({
    parameters: { docs: { description: { story: 'Existing theme indicator colors inherit through combined select states. Trusted focus and hover checks run only in the Vitest browser runner.' } } },
    render: () => `<section class="cem-theme-light"><cem-select label="Role"><cem-option value="author">Author</cem-option><cem-option value="editor">Editor</cem-option></cem-select></section>`,
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('section')!;
        const host = container.querySelector<HTMLElement>('cem-select')!;
        await whenCemRendered(host);
        const control = host.querySelector<HTMLElement>('[role=combobox]')!;
        const colors = {
            '--cem-input-indicator-anchor-color': 'rgb(11, 22, 33)',
            '--cem-input-indicator-anchor-hover-color': 'rgb(22, 33, 44)',
            '--cem-input-indicator-anchor-pending-color': 'rgb(33, 44, 55)',
            '--cem-input-indicator-anchor-invalid-color': 'rgb(44, 55, 66)',
            '--cem-input-indicator-anchor-invalid-hover-color': 'rgb(55, 66, 77)',
            '--cem-input-indicator-anchor-disabled-color': 'rgb(66, 77, 88)',
            '--cem-input-indicator-selection-color': 'rgb(88, 99, 110)',
        };
        Object.entries(colors).forEach(([property, value]) => container.style.setProperty(property, value));
        const shadow = () => getComputedStyle(control).boxShadow;
        for (const appearance of ['underline', 'outline']) {
            host.setAttribute('indicator', appearance);
            await whenCemRendered(host);
            await expect(shadow()).toContain(colors['--cem-input-indicator-anchor-color']);
            host.setAttribute('busy', '');
            await whenCemRendered(host);
            await expect(shadow()).toContain(colors['--cem-input-indicator-anchor-pending-color']);
            host.setAttribute('invalid', '');
            await whenCemRendered(host);
            await expect(shadow()).toContain(colors['--cem-input-indicator-anchor-invalid-color']);
            if (import.meta.env.MODE === 'test') {
                const { userEvent: native } = await import('vitest/browser');
                await native.hover(control);
                await expect(shadow()).toContain(colors['--cem-input-indicator-anchor-invalid-hover-color']);
                await native.unhover(control);
                control.focus();
                await native.keyboard('{ArrowDown}');
                await whenCemRendered(host);
                await expect(control.matches(':focus-visible')).toBe(true);
                await expect(control).toHaveAttribute('aria-expanded', 'true');
                // The theme binds zebra colors on focused controls for the active mode.
                const probe = document.createElement('span');
                probe.style.color = 'var(--cem-zebra-color-1)';
                control.append(probe);
                const focusColor = getComputedStyle(probe).color;
                probe.remove();
                await expect(shadow()).toContain(focusColor);
                await expect(shadow()).toContain(colors['--cem-input-indicator-selection-color']);
                host.style.setProperty('--cem-input-indicator-selection-color', 'rgb(99, 110, 121)');
                await whenCemRendered(host);
                await expect(shadow()).toContain('rgb(99, 110, 121)');
                host.style.removeProperty('--cem-input-indicator-selection-color');
                await whenCemRendered(host);
                await expect(shadow()).toContain(colors['--cem-input-indicator-selection-color']);
                await native.keyboard('{Escape}');
                control.blur();
            }
            host.setAttribute('disabled', '');
            await whenCemRendered(host);
            await expect(shadow()).toContain(colors['--cem-input-indicator-anchor-disabled-color']);
            for (const attribute of ['disabled', 'busy', 'invalid']) host.removeAttribute(attribute);
            await whenCemRendered(host);
        }
        Object.keys(colors).forEach(property => container.style.removeProperty(property));
        await expect(shadow()).not.toContain(colors['--cem-input-indicator-anchor-color']);
    },
});

export const NativeHidden = meta.story({
    parameters: { docs: { description: { story: 'Native host visibility and state retention. Trusted keyboard checks run only in the Vitest browser runner.' } } },
    render: () => `<button data-before>Before</button><cem-select hidden label="Role"><cem-option value="author">Author</cem-option><cem-option value="editor" selected>Editor</cem-option></cem-select><button data-after>After</button>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-select')!;
        await whenCemRendered(host);
        const control = host.querySelector<HTMLElement>('[role=combobox]')!;
        const verifyHidden = async () => {
            await expect(getComputedStyle(host).display).toBe('none');
            await expect(host.getClientRects().length).toBe(0);
            await expect(control.getClientRects().length).toBe(0);
            await expect(control.hasAttribute('hidden')).toBe(false);
            if (import.meta.env.MODE === 'test') {
                const { userEvent: native } = await import('vitest/browser');
                canvasElement.querySelector<HTMLButtonElement>('[data-before]')!.focus();
                await native.keyboard('{Tab}');
                await expect(document.activeElement).toBe(canvasElement.querySelector('[data-after]'));
            }
        };
        await verifyHidden();
        for (const value of ['', 'hidden', 'false']) {
            host.removeAttribute('hidden');
            await whenCemRendered(host);
            await expect(getComputedStyle(host).display).not.toBe('none');
            await expect(host.getClientRects().length).toBeGreaterThan(0);
            await expect(host.querySelector<HTMLElement>('[role=combobox]')!).toBe(control);
            await expect((host as CemSelectElement).value).toBe('editor');
            host.setAttribute('hidden', value);
            await whenCemRendered(host);
            await verifyHidden();
        }
        for (const value of ['until-found', 'UnTiL-FoUnD']) {
            host.setAttribute('hidden', value);
            await whenCemRendered(host);
            await expect(host.hidden).toBe('until-found');
            await expect(getComputedStyle(host).display).not.toBe('none');
            await expect(getComputedStyle(host).contentVisibility).toBe('hidden');
            await expect(host.querySelector<HTMLElement>('[role=combobox]')!).toBe(control);
        }
        host.removeAttribute('hidden');
        await whenCemRendered(host);
        await expect(host.querySelector<HTMLElement>('[role=combobox]')!).toBe(control);
        await expect((host as CemSelectElement).value).toBe('editor');
        if (import.meta.env.MODE === 'test') {
            const { userEvent: native } = await import('vitest/browser');
            canvasElement.querySelector<HTMLButtonElement>('[data-before]')!.focus();
            await native.keyboard('{Tab}');
            await expect(document.activeElement).toBe(control);
        }
    },
});

export const Default = meta.story({
    render: () => `
        <cem-select name="role" label="Role">
            <cem-option-group label="Publishing">
                <cem-option value="author" selected>
                    <strong>Author</strong>
                    <small>Can create content</small>
                </cem-option>
                <cem-option value="editor">Editor</cem-option>
            </cem-option-group>
            <cem-option value="reader">Reader</cem-option>
            <cem-option value="retired" disabled>Retired</cem-option>
        </cem-select>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);
        const events: string[] = [];
        select?.addEventListener('input', () => events.push('input'));
        select?.addEventListener('change', () => events.push('change'));

        const control = canvas.getByRole('combobox', { name: 'Role' });
        await expect(control).toHaveAttribute('part', 'control');
        await expect(select?.querySelector('[part~="root"]')).not.toBeNull();
        await expect(select?.querySelector('[part~="label"]')).not.toBeNull();
        await expect(control).toHaveTextContent('Author');
        await expect(select?.value).toBe('author');
        await expect(select?.type).toBe('select-one');
        const declaration = document.querySelector<HTMLElement>(
            'cem-element[data-cem-storybook-declaration="cem-select"]'
        );
        const declarationStyles = declaration?.querySelectorAll<HTMLStyleElement>(
            ':scope > style[data-cem-declaration-style="private"]'
        );
        await expect(select?.querySelector('style')).toBeNull();
        await expect(declarationStyles?.length).toBe(1);
        await expect(declarationStyles?.[0]?.textContent).toMatch(/@scope\s*\(\s*cem-select\s*\)/u);
        await expect(select).toHaveAttribute('data-cem-render-scope');
        await expect(select).not.toHaveAttribute('data-cem-instance-scope');
        await expect(select).not.toHaveAttribute('data-cem-scope');

        await userEvent.tab();
        await expect(control).toHaveFocus();
        await expect(control.matches(':focus-visible')).toBe(true);

        await userEvent.click(control);
        await whenCemRendered(select as CemSelectElement);
        await expect(control).toHaveAttribute('aria-expanded', 'true');
        await expect(control).toHaveAttribute('aria-controls');
        await expect(canvas.getByRole('group', { name: 'Publishing' })).toBeVisible();
        await expect(canvas.getByRole('group', { name: 'Publishing' })).toHaveAttribute('part', 'group');
        await expect(canvas.getByRole('option', { name: /Author/ }).querySelector('strong')).toHaveTextContent('Author');
        await expect(canvas.getByRole('option', { name: /Author/ })).toHaveAttribute('part', 'option');
        await expect(canvas.getByRole('option', { name: 'Retired' })).toHaveAttribute('aria-disabled', 'true');

        await userEvent.click(canvas.getByRole('option', { name: 'Editor' }));
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.value).toBe('editor');
        await expect(control).toHaveAttribute('aria-expanded', 'false');
        await expect(events).toEqual(['input', 'change']);

        await userEvent.click(control);
        await userEvent.keyboard('{ArrowDown}{Escape}');
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.value).toBe('editor');
        await expect(control).toHaveAttribute('aria-expanded', 'false');
        await expect(events).toEqual(['input', 'change']);
    },
});

export const StylesheetOwnership = meta.story({
    render: () => `
        <cem-select label="First">
            <cem-option value="one" selected>One</cem-option>
        </cem-select>
        <cem-select label="Second">
            <cem-option value="two" selected>Two</cem-option>
        </cem-select>
    `,
    play: async ({ canvasElement }) => {
        const instances = Array.from(canvasElement.querySelectorAll<CemSelectElement>('cem-select'));
        await expect(instances).toHaveLength(2);
        await Promise.all(instances.map(whenCemRendered));

        const declaration = document.querySelector<HTMLElement>(
            'cem-element[data-cem-storybook-declaration="cem-select"]'
        );
        const styles = declaration?.querySelectorAll<HTMLStyleElement>(
            ':scope > style[data-cem-declaration-style]'
        );
        await expect(styles?.length).toBe(1);
        await expect(instances.every((instance) => instance.querySelector('style') === null)).toBe(true);
        await expect(getComputedStyle(instances[0]).display).toBe('inline-block');
        await expect(getComputedStyle(instances[1]).display).toBe('inline-block');
    },
});

export const FormAndKeyboard = meta.story({
    render: () => `
        <form>
            <p id="role-help">Choose the closest role.</p>
            <p id="role-error">Role is required.</p>
            <cem-select id="required-role" name="role" label="Role" required invalid describedby="role-help" error="role-error">
                <cem-option value="">Choose a role</cem-option>
                <cem-option value="author">
                    <strong>Author</strong>
                    <small>Can create content</small>
                </cem-option>
                <cem-option value="editor">Editor</cem-option>
                <cem-option value="retired" disabled>Retired</cem-option>
            </cem-select>
        </form>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const form = canvasElement.querySelector<HTMLFormElement>('form');
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(form).not.toBeNull();
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);
        const events: string[] = [];
        select?.addEventListener('input', () => events.push('input'));
        select?.addEventListener('change', () => events.push('change'));

        const control = canvas.getByRole('combobox', { name: 'Role' });
        await expect(select as CemSelectElement).toHaveAttribute('required');
        await expect(control).toHaveAttribute('aria-invalid', 'true');
        await expect(control).toHaveAttribute('aria-describedby', 'role-help');
        await expect(control).toHaveAttribute('aria-errormessage', 'role-error');
        await expect(select?.checkValidity()).toBe(false);
        await expect(select?.validity.valueMissing).toBe(true);
        await expect(select?.validationMessage).not.toBe('');
        await expect(select?.form).toBe(form);

        await userEvent.click(control);
        await userEvent.keyboard('{ArrowDown}{Enter}');
        await whenCemRendered(select as CemSelectElement);

        await expect(select?.value).toBe('author');
        await expect(control).toHaveTextContent('Author');
        await expect(select?.checkValidity()).toBe(true);
        await expect(new FormData(form as HTMLFormElement).get('role')).toBe('author');
        await expect(events).toEqual(['input', 'change']);

        select?.setSelectedValues(['']);
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.checkValidity()).toBe(false);
        await expect(events).toEqual(['input', 'change']);

        form?.reset();
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.value).toBe('');
    },
});

export const Loading = meta.story({
    render: () => `
        <cem-select name="role" label="Role" busy>
            <cem-option value="author" selected>Author</cem-option>
            <cem-option value="editor">Editor</cem-option>
        </cem-select>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);

        const control = canvas.getByRole('combobox', { name: 'Role' });
        await expect(control).toHaveAttribute('data-state', 'loading');
        await expect(control).toHaveAttribute('aria-busy', 'true');
        await expect(select?.value).toBe('author');
    },
});

export const MultipleListbox = meta.story({
    render: () => `
        <form>
            <cem-select name="tag" label="Tags" multiple size="4">
                <cem-option value="accessibility" selected>Accessibility</cem-option>
                <cem-option value="design">Design</cem-option>
                <cem-option value="runtime">Runtime</cem-option>
                <cem-option value="deprecated" disabled>Deprecated</cem-option>
            </cem-select>
        </form>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const form = canvasElement.querySelector<HTMLFormElement>('form');
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(form).not.toBeNull();
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);

        const listbox = canvas.getByRole('listbox', { name: 'Tags' });
        await expect(select?.type).toBe('select-multiple');
        await expect(select?.multiple).toBe(true);
        await expect(select?.size).toBe(4);
        await expect(listbox).toHaveAttribute('aria-multiselectable', 'true');
        await expect(new FormData(form as HTMLFormElement).getAll('tag')).toEqual(['accessibility']);
        await userEvent.click(canvas.getByRole('option', { name: 'Design' }));
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.selectedValues).toEqual(['accessibility', 'design']);

        listbox.focus();
        await userEvent.keyboard('{Control>}a{/Control}');
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.selectedValues).toEqual(['accessibility', 'design', 'runtime']);
        await expect(new FormData(form as HTMLFormElement).getAll('tag')).toEqual([
            'accessibility',
            'design',
            'runtime',
        ]);
    },
});

export const SingleListbox = meta.story({
    render: () => `
        <cem-select name="tier" label="Tier" size="3">
            <cem-option value="one">One</cem-option>
            <cem-option value="two" selected>Two</cem-option>
            <cem-option value="three">Three</cem-option>
        </cem-select>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);

        const listbox = canvas.getByRole('listbox', { name: 'Tier' });
        await expect(select?.type).toBe('select-one');
        await expect(select?.size).toBe(3);
        await expect(listbox).not.toHaveAttribute('aria-multiselectable');

        listbox.focus();
        await userEvent.keyboard('{ArrowDown}');
        await whenCemRendered(select as CemSelectElement);
        await expect(select?.value).toBe('three');
    },
});

export const Disabled = meta.story({
    render: () => `
        <form>
            <cem-select name="role" label="Role" disabled>
                <cem-option value="author" selected>Author</cem-option>
                <cem-option value="editor">Editor</cem-option>
            </cem-select>
        </form>
    `,
    play: async ({ canvasElement }) => {
        const canvas = within(canvasElement);
        const form = canvasElement.querySelector<HTMLFormElement>('form');
        const select = canvasElement.querySelector<CemSelectElement>('cem-select');
        await expect(form).not.toBeNull();
        await expect(select).not.toBeNull();
        await whenCemRendered(select as CemSelectElement);

        const control = canvas.getByRole('combobox', { name: 'Role' });
        await expect(select?.disabled).toBe(true);
        await expect(control).toBeDisabled();
        await expect(new FormData(form as HTMLFormElement).has('role')).toBe(false);

        await userEvent.click(control);
        await expect(control).toHaveAttribute('aria-expanded', 'false');
    },
});

export const NativeMouseSelection = meta.story({
    parameters: { docs: { description: { story: 'Hold and release an option with the mouse. Trusted pointer assertions run in the Vitest browser runner.' } } },
    render: () => `<button type="button">Outside select</button><cem-select label="Mouse selection" value="primary">
        <cem-option value="primary">Primary</cem-option>
        <cem-option value="destructive"><strong>Destructive</strong></cem-option>
    </cem-select>`,
    play: async ({ canvasElement }) => {
        const select = canvasElement.querySelector('cem-select') as CemSelectElement;
        await whenCemRendered(select);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const canvas = within(canvasElement);
        const control = canvas.getByRole('combobox', { name: 'Mouse selection' });
        const events: string[] = [];
        select.addEventListener('input', () => events.push('input'));
        select.addEventListener('change', () => events.push('change'));
        await native.click(control);
        await whenCemRendered(select);
        const option = canvas.getByRole('option', { name: 'Destructive' });
        const down = new Promise<void>(resolve => option.addEventListener('pointerdown', () => resolve(), { once: true }));
        const click = Promise.resolve(native.click(option.querySelector('strong') as HTMLElement, { delay: 250 }));
        try {
            await down;
            await new Promise(resolve => setTimeout(resolve, 80));
            await expect(document.activeElement).toBe(control);
            await expect(control).toHaveAttribute('aria-expanded', 'true');
            await expect(select.value).toBe('primary');
            await expect(events).toEqual([]);
        } finally {
            await click;
        }
        await whenCemRendered(select);
        await expect(select.value).toBe('destructive');
        await expect(control).toHaveAttribute('aria-expanded', 'false');
        await expect(document.activeElement).toBe(control);
        await expect(events).toEqual(['input', 'change']);
        await native.click(control);
        await whenCemRendered(select);
        const outside = canvas.getByRole('button', { name: 'Outside select' });
        await native.click(outside);
        await whenCemRendered(select);
        await expect(document.activeElement).toBe(outside);
        await expect(control).toHaveAttribute('aria-expanded', 'false');
        await expect(events).toEqual(['input', 'change']);
    },
});
