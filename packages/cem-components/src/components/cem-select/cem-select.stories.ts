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
        await expect(declarationStyles?.[0]?.textContent).toContain('@scope (\n    cem-select');
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
