import { expect, userEvent, waitFor, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, cemComponentDeclarationSource, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-theme-switch.xhtml?raw';

const meta = preview.meta({
    component: 'cem-theme-switch',
    title: 'CEM Components/cem-theme-switch',
    globals: { cemTheme: 'light' },
    loaders: [async () => {
        for (const tag of ['cem-icon', 'cem-icon-button', 'cem-action', 'cem-menu-item', 'cem-menu', 'cem-dropdown']) await loadCemDeclaration(tag, await cemComponentDeclarationSource(tag));
        await loadCemDeclaration('cem-theme-switch', declarationSource); return {};
    }],
});

async function choose(host: HTMLElement, value: string): Promise<void> {
    await waitFor(() => expect(host.querySelector('[part=trigger] button')).not.toBeNull());
    await waitFor(() => expect(host.querySelectorAll('cem-menu cem-icon-button button')).toHaveLength(5));
    await userEvent.click(required(host, '[part=trigger] button'));
    await userEvent.click(required(host, `cem-icon-button[value="cem-theme-${value}"] button`));
    await whenCemRendered(host);
    await waitFor(() => expect(host).toHaveAttribute('data-theme', `cem-theme-${value}`));
}

export const ModesAndContrast = meta.story({
    render: () => `<cem-theme-switch storage-key="story-theme-modes"><label>Retained value <input data-retained value="Draft"></label></cem-theme-switch>`,
    beforeEach: () => { localStorage.removeItem('story-theme-modes'); },
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-theme-switch');
        await whenCemRendered(host);
        const retained = required<HTMLInputElement>(host, '[data-retained]');
        retained.value = 'Edited';
        await waitFor(() => expect(host.querySelectorAll('cem-menu cem-icon-button button')).toHaveLength(5));
        await expect(host.querySelector('cem-dropdown > [part=base] button')).toBeNull();
        await expect(required(host, 'cem-icon-button[part=trigger]')).toHaveAttribute('image', '◐');
        await expect(required(host, 'cem-icon-button[part=trigger]')).toHaveAttribute('variant', 'contextual');
        await expect(host).toHaveAttribute('data-theme', 'cem-theme-light');
        for (const [index, item] of [...host.querySelectorAll<HTMLElement>('cem-menu cem-icon-button')].entries()) {
            await expect(item).toHaveAttribute('align', 'start');
            await whenCemRendered(item);
            const icon = required(item, 'cem-icon');
            await whenCemRendered(icon);
            const glyph = required(icon, '[part=icon]');
            expect(glyph.textContent?.trim()).toBe(['☀', '☾', '☀', '☾', '◐'][index]);
            expect(parseFloat(getComputedStyle(glyph).fontSize)).toBeGreaterThan(parseFloat(getComputedStyle(required(item, 'button')).fontSize));
            await expect(required(item, 'button')).toHaveAttribute('role', 'menuitemradio');
            await expect(required(item, 'button')).not.toHaveAttribute('aria-pressed');
        }
        for (const mode of ['dark', 'contrast-light', 'contrast-dark', 'native', 'light']) {
            await choose(host, mode);
            await waitFor(() => expect(localStorage.getItem('story-theme-modes')).toBe(`cem-theme-${mode}`));
            await expect(host.querySelector('[data-retained]')).toBe(retained);
            await expect(retained.value).toBe('Edited');
            const selected = required(host, `cem-icon-button[value="cem-theme-${mode}"]`);
            await waitFor(() => expect(selected).toHaveAttribute('selected'));
            await waitFor(() => expect(required(selected, 'button')).toHaveAttribute('aria-checked', 'true'));
            await expect(selected).toHaveAttribute('variant', 'contextual');
            await expect(getComputedStyle(required(selected, 'button')).boxShadow).not.toBe('none');
        }
    },
});

export const IndependentScopes = meta.story({
    render: () => `<cem-theme-switch storage-key="story-theme-first" mode="dark" contrast="true"><p>First scope</p></cem-theme-switch><cem-theme-switch storage-key="story-theme-second"><p>Second scope</p></cem-theme-switch>`,
    beforeEach: () => { localStorage.removeItem('story-theme-first'); localStorage.removeItem('story-theme-second'); },
    play: async ({ canvasElement }) => {
        const [first, second] = [...canvasElement.querySelectorAll<HTMLElement>('cem-theme-switch')];
        for (const host of [first, second]) await whenCemRendered(host);
        await expect(first).toHaveAttribute('data-theme', 'cem-theme-contrast-dark');
        await choose(first, 'dark');
        await expect(second).toHaveAttribute('data-theme', 'cem-theme-light');
        first.remove();
        const restored = document.createElement('cem-theme-switch'); restored.setAttribute('storage-key', 'story-theme-first');
        canvasElement.append(restored); await whenCemRendered(restored);
        await waitFor(() => expect(restored).toHaveAttribute('data-theme', 'cem-theme-dark'));
    },
});

export const SavedSelection = meta.story({
    render: () => '<cem-theme-switch storage-key="story-theme-saved"></cem-theme-switch><cem-theme-switch storage-key="story-theme-invalid"></cem-theme-switch>',
    beforeEach: () => {
        localStorage.setItem('story-theme-saved', 'cem-theme-contrast-dark');
        localStorage.setItem('story-theme-invalid', 'unrecognized');
    },
    play: async ({ canvasElement }) => {
        const [saved, invalid] = [...canvasElement.querySelectorAll<HTMLElement>('cem-theme-switch')];
        await whenCemRendered(saved); await whenCemRendered(invalid);
        await waitFor(() => expect(saved).toHaveAttribute('data-theme', 'cem-theme-contrast-dark'));
        await expect(invalid).toHaveAttribute('data-theme', 'cem-theme-light');
    },
});

export const Keyboard = meta.story({
    render: () => '<cem-theme-switch storage-key="story-theme-keyboard"></cem-theme-switch>',
    beforeEach: () => { localStorage.removeItem('story-theme-keyboard'); },
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-theme-switch'); await whenCemRendered(host);
        await waitFor(() => expect(host.querySelector('[part=trigger] button')).not.toBeNull());
        const trigger = required(host, '[part=trigger] button');
        await waitFor(() => expect(host.querySelectorAll('cem-menu cem-icon-button button')).toHaveLength(5));
        await userEvent.click(trigger);
        await userEvent.keyboard('{ArrowDown}{Enter}');
        await waitFor(() => expect(host).toHaveAttribute('data-theme', 'cem-theme-dark'));
        await userEvent.click(trigger);
        await userEvent.keyboard('{Escape}');
        await expect(trigger).toHaveFocus();
        await expect(trigger).toHaveAttribute('aria-expanded', 'false');
    },
});

export const ViewportMenu = meta.story({
    render: () => '<section style="position:fixed;inset-block-start:0;inset-inline-end:0"><cem-theme-switch storage-key="story-theme-viewport"></cem-theme-switch></section>',
    beforeEach: () => { localStorage.removeItem('story-theme-viewport'); },
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-theme-switch'); await whenCemRendered(host);
        await waitFor(() => expect(host.querySelectorAll('cem-menu button')).toHaveLength(5));
        await Promise.all([...host.querySelectorAll<HTMLElement>('cem-dropdown,cem-menu,cem-icon-button,cem-icon')].map(whenCemRendered));
        await userEvent.click(required(host, '[part=trigger] button'));
        const popup = required(host, 'cem-dropdown > [part=popup]');
        await waitFor(() => {
            const rect = popup.getBoundingClientRect();
            expect(rect.left).toBeGreaterThanOrEqual(0);
            expect(rect.right).toBeLessThanOrEqual(window.innerWidth);
            expect(rect.top).toBeGreaterThanOrEqual(0);
            expect(rect.bottom).toBeLessThanOrEqual(window.innerHeight);
        });
        await userEvent.click(required(host, 'cem-icon-button[value="cem-theme-dark"] [part=glyph]'));
        await waitFor(() => expect(host).toHaveAttribute('data-theme', 'cem-theme-dark'));
    },
});

export const MenuIndicatorClearance = meta.story({
    render: () => '<p>Adjacent keyboard ring checks run in the browser test runner.</p>'
        + ['', '--cem-inset-control:20px;--cem-gap-related:18px', '--cem-inset-control:0px;--cem-gap-related:0px']
        .map((style, index) => `<section style="${style}"><cem-theme-switch storage-key="story-theme-menu-clearance-${index}"></cem-theme-switch></section>`).join(''),
    beforeEach: () => { for (let index = 0; index < 3; index++) localStorage.removeItem(`story-theme-menu-clearance-${index}`); },
    play: async ({ canvasElement }) => {
        for (const [index, host] of [...canvasElement.querySelectorAll<HTMLElement>('cem-theme-switch')].entries()) {
            await whenCemRendered(host);
            await waitFor(() => expect(host.querySelectorAll('cem-menu cem-icon-button button')).toHaveLength(5));
            await Promise.all([...host.querySelectorAll<HTMLElement>('cem-dropdown,cem-menu,cem-icon-button,cem-icon')].map(whenCemRendered));
            await userEvent.click(required(host, '[part=trigger] button'));
            const popup = required(host, 'cem-dropdown > [part=popup]');
            expect(popup.scrollHeight).toBeLessThanOrEqual(popup.clientHeight + 1);
            const panel = popup.getBoundingClientRect();
            for (const button of host.querySelectorAll('cem-menu cem-icon-button button')) {
                const rect = button.getBoundingClientRect();
                expect(rect.top).toBeGreaterThanOrEqual(panel.top);
                expect(rect.bottom).toBeLessThanOrEqual(panel.bottom);
                const style = getComputedStyle(button);
                expect(required(button, '[part=icon]').getBoundingClientRect().left)
                    .toBeCloseTo(rect.left + parseFloat(style.borderLeftWidth) + parseFloat(style.paddingLeft), 0);
            }
            const selected = required(host, 'cem-menu cem-icon-button[selected] button');
            await expect(selected).toHaveAttribute('aria-checked', 'true');
            const style = getComputedStyle(selected);
            const ring = Math.max(...(style.boxShadow.match(/[\d.]+px/g) ?? []).map(parseFloat));
            expect(ring).toBeGreaterThan(0);
            const bounds = popup.getBoundingClientRect();
            const control = selected.getBoundingClientRect();
            expect(control.left - bounds.left).toBeGreaterThanOrEqual(ring);
            expect(bounds.right - control.right).toBeGreaterThanOrEqual(ring);
            expect(control.top - bounds.top).toBeGreaterThanOrEqual(ring);
            const next = required(host, 'cem-icon-button[value="cem-theme-dark"] button').getBoundingClientRect();
            expect(next.top - control.bottom).toBeGreaterThanOrEqual(ring);
            if (index === 1) {
                expect(control.left - bounds.left).toBeGreaterThanOrEqual(20);
                expect(next.top - control.bottom).toBeGreaterThanOrEqual(18);
            }
            await adjacentIndicators(host, selected, ring);
            await userEvent.click(required(host, '[part=trigger] button'));
        }
    },
});

export const SuperCompactMenuSurface = meta.story({
    render: () => ['light', 'dark', 'contrast-light', 'contrast-dark', 'native'].map(theme =>
        `<section data-cem-density="super-compact"><cem-theme-switch value="cem-theme-${theme}" storage-key="story-compact-menu-${theme}"></cem-theme-switch></section>`).join(''),
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-theme-switch')) {
            await whenCemRendered(host);
            await waitFor(() => expect(host.querySelectorAll('cem-menu button')).toHaveLength(5));
            await Promise.all([...host.querySelectorAll<HTMLElement>('cem-dropdown,cem-menu,cem-icon-button,cem-icon')].map(whenCemRendered));
            await userEvent.click(required(host, '[part=trigger] button'));
            const popup = required(host, 'cem-dropdown > [part=popup]');
            const buttons = [...host.querySelectorAll<HTMLElement>('cem-menu button')];
            const expectSurface = () => {
                expect(getComputedStyle(popup).backgroundColor).toBe(getComputedStyle(host).backgroundColor);
                expect(getComputedStyle(popup).backgroundColor).not.toBe('rgba(0, 0, 0, 0)');
                expect(popup.scrollHeight).toBeLessThanOrEqual(popup.clientHeight + 1);
                const bounds = popup.getBoundingClientRect();
                const selected = required(host, 'cem-menu cem-icon-button[selected] button');
                const ring = Math.max(...(getComputedStyle(selected).boxShadow.match(/[\d.]+px/g) ?? []).map(parseFloat));
                expect(ring).toBeGreaterThan(0);
                for (const [index, button] of buttons.entries()) {
                    const rect = button.getBoundingClientRect(), style = getComputedStyle(button);
                    expect(style.getPropertyValue('--cem-coupling-zone-min').trim()).toBe('0px');
                    expect(style.paddingBlockStart).toBe('0px');
                    expect(rect.left - bounds.left).toBeGreaterThanOrEqual(ring);
                    expect(bounds.right - rect.right).toBeGreaterThanOrEqual(ring);
                    expect(rect.top - bounds.top).toBeGreaterThanOrEqual(ring);
                    expect(bounds.bottom - rect.bottom).toBeGreaterThanOrEqual(ring);
                    if (index) expect(rect.top - buttons[index - 1].getBoundingClientRect().bottom).toBeGreaterThanOrEqual(2 * ring);
                }
            };
            expectSurface();
            const selected = required(host, 'cem-menu cem-icon-button[selected] button');
            selected.focus(); await userEvent.keyboard('{ArrowDown}');
            const next = buttons[(buttons.indexOf(selected) + 1) % buttons.length];
            await waitFor(() => expect(next).toHaveFocus());
            expectSurface();
            host.style.setProperty('--cem-zebra-strip-size', '3px');
            expectSurface();
            host.style.setProperty('--cem-palette-comfort', 'rgb(31, 47, 63)');
            expect(getComputedStyle(popup).backgroundColor).toBe('rgb(31, 47, 63)');
            expectSurface();
            await userEvent.keyboard('{Escape}');
            await waitFor(() => expect(popup).not.toBeVisible());
            expect(required(host, '[part=trigger] button')).toHaveFocus();
        }
    },
});

async function adjacentIndicators(host: HTMLElement, selected: HTMLElement, ring: number): Promise<void> {
    if (import.meta.env.MODE !== 'test') return;
    const { userEvent: nativeInput } = await import('vitest/browser');
    selected.focus();
    await nativeInput.keyboard('{ArrowDown}');
    const focused = required(host, 'cem-icon-button[value="cem-theme-dark"] button');
    await expect(focused).toHaveFocus();
    const focusRing = Math.max(...(getComputedStyle(focused).boxShadow.match(/[\d.]+px/g) ?? []).map(parseFloat));
    expect(focusRing).toBeGreaterThan(0);
    expect(focused.getBoundingClientRect().top - selected.getBoundingClientRect().bottom).toBeGreaterThanOrEqual(ring + focusRing);
}

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
            const suffix = crypto.randomUUID();
            const actionTag = `action-${suffix}`, themeTag = `theme-${suffix}`;
            const { runtime, scope, declarationTag, declare } = createCemStoryRuntime(root, fallback);
            const actionSource = await cemComponentDeclarationSource('cem-action');
            const declarations = [await declare(actionSource, actionTag), await declare(declarationSource, themeTag)];
            const theme = document.createElement(themeTag); theme.setAttribute('mode', 'light'); theme.setAttribute('storage-key', `theme-${suffix}`);
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
                    await choose(theme, name.toLowerCase());
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
