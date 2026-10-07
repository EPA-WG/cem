import { expect, userEvent, waitFor, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-theme-switch.xhtml?raw';
import actionSource from '../cem-action/cem-action.xhtml?raw';
import { CemElementRuntime, createCemDeclarationScope } from '../../../../cem-elements/src/index.js';

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

function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}

export const NativeActivationThenThemeProjection = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const suffix = crypto.randomUUID(), declarationTag = `declaration-${suffix}`;
            const actionTag = `action-${suffix}`, themeTag = `theme-${suffix}`;
            const scope = createCemDeclarationScope({ document });
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback fixture'); } } : {}) });
            runtime.install(window);
            const declare = async (source: string, tag: string) => {
                const template = document.createElement('template'); template.innerHTML = source;
                const original = required(template.content, 'cem-element');
                const declaration = document.createElement(declarationTag);
                for (const attribute of original.attributes) if (attribute.name !== 'tag') declaration.setAttribute(attribute.name, attribute.value);
                declaration.setAttribute('tag', tag); declaration.append(...original.childNodes); root.append(declaration);
                await runtime.whenDeclarationSettled(declaration); return declaration;
            };
            const declarations = [await declare(actionSource, actionTag), await declare(declarationSource, themeTag)];
            const theme = document.createElement(themeTag); theme.setAttribute('mode', 'light');
            theme.innerHTML = `<template><cem-demo-element legend="Controlled selection"><template slot="source">
                <${declarationTag}><template type="text/cem-ml">
                {slice @name=choice | compact}
                {div @role=group @aria-label="Layout choice" |
                    {${actionTag} @selectable=true @selected={if datadom.slices.choice == "compact" { true } else { null }} @slice=choice @slice-event=click @slice-value="'compact'" | Compact}
                    {${actionTag} @selectable=true @selected={if datadom.slices.choice == "comfortable" { true } else { null }} @slice=choice @slice-event=click @slice-value="'comfortable'" | Comfortable}}
                </template></${declarationTag}>
                </template></cem-demo-element>
                <${actionTag} commandfor="popover-${suffix}" command="toggle-popover">Toggle details</${actionTag}>
                <div id="popover-${suffix}" popover="auto" role="dialog" aria-label="Details"><button type="button" popovertarget="popover-${suffix}" popovertargetaction="hide">Close details</button></div>
                <${actionTag} commandfor="dialog-${suffix}" command="show-modal">Open task</${actionTag}>
                <dialog id="dialog-${suffix}" aria-label="Task"><button type="button" commandfor="dialog-${suffix}" command="request-close">Close task</button></dialog></template>`;
            root.append(theme);
            try {
                await runtime.whenRenderSettled(theme);
                const card = required(theme, 'cem-demo-element');
                await (card as HTMLElement & { updateComplete: Promise<void> }).updateComplete;
                const declaration = required(card, `[slot=demo] ${declarationTag}`);
                await runtime.whenDeclarationSettled(declaration);
                const instance = required(card, '[data-cem-anonymous-instance]');
                await runtime.whenRenderSettled(instance);
                for (const action of theme.querySelectorAll<HTMLElement>(actionTag)) await runtime.whenRenderSettled(action);
                const group = required(card, '[aria-label="Layout choice"]');
                await userEvent.click(within(group).getByRole('button', { name: 'Comfortable' }));
                await runtime.whenRenderSettled(instance);
                await waitFor(() => expect(within(group).getByRole('button', { name: 'Comfortable' })).toHaveAttribute('aria-pressed', 'true'));
                const controls = within(theme), popover = required(theme, '[popover]'), dialog = required<HTMLDialogElement>(theme, 'dialog');
                await userEvent.click(controls.getByRole('button', { name: 'Toggle details' }));
                await expect(popover.matches(':popover-open')).toBe(true);
                await userEvent.click(controls.getByRole('button', { name: 'Close details' }));
                await userEvent.click(controls.getByRole('button', { name: 'Open task' }));
                await expect(dialog.matches(':modal')).toBe(true);
                await userEvent.click(controls.getByRole('button', { name: 'Close task' }));
                await expect(dialog.open).toBe(false);
                for (const name of ['Dark', 'Native', 'Light']) {
                    await userEvent.click(controls.getByRole('radio', { name, exact: true }));
                    await runtime.whenRenderSettled(theme);
                    await expect(theme).toHaveAttribute('data-theme', `cem-theme-${name.toLowerCase()}`);
                    await expect(required(theme, 'cem-demo-element')).toBe(card);
                    await expect(required(card, `[slot=demo] ${declarationTag}`)).toBe(declaration);
                    await expect(required(card, '[data-cem-anonymous-instance]')).toBe(instance);
                    await expect(required(card, '[aria-label="Layout choice"]')).toBe(group);
                    await expect(runtime.snapshotInstance(instance).slices.choice).toBe('comfortable');
                    await expect(within(group).getByRole('button', { name: 'Comfortable' })).toHaveAttribute('aria-pressed', 'true');
                    await expect(within(group).getByRole('button', { name: 'Compact' })).toHaveAttribute('aria-pressed', 'false');
                }
            } finally { theme.remove(); declarations.forEach(declaration => declaration.remove()); scope.dispose(); }
        }
    },
});
