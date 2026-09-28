import { expect, userEvent, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-theme-switch.xhtml?raw';

const meta = preview.meta({
    component: 'cem-theme-switch',
    title: 'CEM Components/cem-theme-switch',
    globals: { cemTheme: 'light' },
    loaders: [async () => { await loadCemDeclaration('cem-theme-switch', declarationSource); return {}; }],
});

export const ModesAndContrast = meta.story({
    render: () => `<cem-theme-switch><label>Retained value <input data-retained value="Draft"></label></cem-theme-switch>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-theme-switch')!;
        await whenCemRendered(host);
        const canvas = within(host);
        const contrast = canvas.getByRole('checkbox', { name: 'Contrast' });
        const retained = host.querySelector<HTMLInputElement>('[data-retained]')!;
        retained.value = 'Edited';
        await expect(host).toHaveAttribute('data-theme', 'cem-theme-native');
        await expect(canvas.getByRole('radio', { name: 'Native' })).toBeChecked();
        await expect(contrast).toBeDisabled();
        for (const mode of ['Light', 'Dark']) {
            await userEvent.click(canvas.getByRole('radio', { name: mode }));
            await whenCemRendered(host);
            await expect(host).toHaveAttribute('data-theme', `cem-theme-${mode.toLowerCase()}`);
            await expect(contrast).toBeEnabled();
            await userEvent.click(contrast);
            await whenCemRendered(host);
            await expect(host).toHaveAttribute('data-theme', `cem-theme-contrast-${mode.toLowerCase()}`);
            await expect(getComputedStyle(host).colorScheme).toBe(mode.toLowerCase());
            await userEvent.click(canvas.getByRole('radio', { name: 'Native' }));
            await whenCemRendered(host);
            await expect(host).toHaveAttribute('data-theme', 'cem-theme-native');
            await expect(contrast).toBeDisabled();
            await expect(contrast).not.toBeChecked();
            await userEvent.click(canvas.getByRole('radio', { name: mode }));
            await whenCemRendered(host);
            await expect(contrast).toBeChecked();
            await expect(host).toHaveAttribute('data-theme', `cem-theme-contrast-${mode.toLowerCase()}`);
            await userEvent.click(contrast);
            await whenCemRendered(host);
            await expect(host.querySelector('[data-retained]')).toBe(retained);
            await expect(retained.value).toBe('Edited');
        }
    },
});

export const IndependentScopes = meta.story({
    render: () => `<cem-theme-switch name="first-theme" mode="dark" contrast="true"><p>First scope</p></cem-theme-switch><cem-theme-switch name="second-theme" mode="light"><p>Second scope</p></cem-theme-switch>`,
    play: async ({ canvasElement }) => {
        const [first, second] = [...canvasElement.querySelectorAll<HTMLElement>('cem-theme-switch')];
        for (const host of [first, second]) await whenCemRendered(host);
        await expect(first).toHaveAttribute('data-theme', 'cem-theme-contrast-dark');
        await expect(second).toHaveAttribute('data-theme', 'cem-theme-light');
        await userEvent.click(within(first).getByRole('radio', { name: 'Native' }));
        await whenCemRendered(first);
        await expect(within(second).getByRole('radio', { name: 'Light' })).toBeChecked();
        await expect(second).toHaveAttribute('data-theme', 'cem-theme-light');
    },
});

export const Keyboard = meta.story({
    parameters: { docs: { description: { story: 'Trusted radio arrow navigation and checkbox keyboard activation run only in the Vitest browser runner.' } } },
    render: () => `<cem-theme-switch mode="light"><button>Preview action</button></cem-theme-switch>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-theme-switch')!;
        await whenCemRendered(host);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const canvas = within(host);
        canvas.getByRole('radio', { name: 'Light' }).focus();
        await native.keyboard('{ArrowRight}');
        await whenCemRendered(host);
        await expect(canvas.getByRole('radio', { name: 'Dark' })).toHaveFocus();
        await expect(host).toHaveAttribute('data-theme', 'cem-theme-dark');
        await native.keyboard('{Tab}');
        await expect(canvas.getByRole('checkbox', { name: 'Contrast' })).toHaveFocus();
        await native.keyboard('[Space]');
        await whenCemRendered(host);
        await expect(host).toHaveAttribute('data-theme', 'cem-theme-contrast-dark');
        canvas.getByRole('radio', { name: 'Dark' }).focus();
        await native.keyboard('{ArrowRight}');
        await whenCemRendered(host);
        await expect(host).toHaveAttribute('data-theme', 'cem-theme-native');
        await native.keyboard('{Tab}');
        await expect(canvas.getByRole('button', { name: 'Preview action' })).toHaveFocus();
    },
});
