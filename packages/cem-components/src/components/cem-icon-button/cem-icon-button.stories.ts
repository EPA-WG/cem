import { expect, within, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-icon-button.xhtml?raw';

const meta = preview.meta({
    component: 'cem-icon-button',
    title: 'CEM Components/cem-icon-button',
    loaders: [async () => { await loadCemDeclaration('cem-icon-button', declarationSource); return {}; }],
});

export const AllAttributes = meta.story({
    render: () => '<section class="cem-theme-light"><cem-icon-button></cem-icon-button></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const button = within(host).getByRole('button', { name: 'Icon action' }) as HTMLButtonElement;
        expect(button.type).toBe('button');
        expect(button.getAttribute('part')).toBe('control');
        expect(button.disabled).toBe(false);
        expect(button.hasAttribute('aria-expanded')).toBe(false);
        host.setAttribute('label', 'Open settings');
        await whenCemRendered(host);
        expect(within(host).getByRole('button', { name: 'Open settings' })).toBe(button);
        host.removeAttribute('label');
        await whenCemRendered(host);
        expect(within(host).getByRole('button', { name: 'Icon action' })).toBe(button);
        expect(button.classList.contains('cem-icon-button--quiet')).toBe(true);
        expect(host.querySelector('[part="icon"]')?.textContent?.trim()).toBe('circle');
        host.setAttribute('name', 'settings');
        host.setAttribute('variant', 'custom');
        await whenCemRendered(host);
        expect(host.querySelector('[part="icon"]')?.textContent?.trim()).toBe('settings');
        expect(host.querySelector('[part="icon"]')?.getAttribute('aria-hidden')).toBe('true');
        expect(button.classList.contains('cem-icon-button--custom')).toBe(true);
        host.removeAttribute('name');
        host.removeAttribute('variant');
        await whenCemRendered(host);
        expect(host.querySelector('[part="icon"]')?.textContent?.trim()).toBe('circle');
        expect(button.classList.contains('cem-icon-button--quiet')).toBe(true);

        for (const value of ['', 'false', 'true']) {
            host.setAttribute('disabled', value);
            await whenCemRendered(host);
            expect(button.disabled).toBe(true);
        }
        host.removeAttribute('disabled');
        for (const value of ['false', 'true']) {
            host.setAttribute('expanded', value);
            await whenCemRendered(host);
            expect(button.getAttribute('aria-expanded')).toBe(value);
        }
        host.removeAttribute('expanded');
        host.setAttribute('class', 'consumer-class');
        await whenCemRendered(host);
        expect(button.disabled).toBe(false);
        expect(button.hasAttribute('aria-expanded')).toBe(false);
        expect(host.classList.contains('consumer-class')).toBe(true);
        host.hidden = true;
        await whenCemRendered(host);
        expect(getComputedStyle(host).display).toBe('none');
        host.hidden = false;
        await whenCemRendered(host);
        expect(getComputedStyle(host).display).not.toBe('none');
        expect(host.querySelector('button')).toBe(button);
        expect(host.querySelector('style')).toBeNull();
    },
});

export const LegacyIconSources = meta.story({
    render: () => '<cem-icon-button label="Open documentation" href="#documentation" icon="recycling">Documentation</cem-icon-button>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const link = within(host).getByRole('link', { name: 'Open documentation' });
        expect(link.getAttribute('href')).toBe('#documentation');
        expect(link.textContent).toContain('Documentation');
        expect(host.querySelector('[part="icon"]')?.classList.contains('material-icons')).toBe(true);
        host.setAttribute('icon', 'fas fa-cloud-upload-alt');
        await whenCemRendered(host);
        expect(host.querySelector('i[part="icon"]')?.classList.contains('fa-cloud-upload-alt')).toBe(true);
        const image = 'data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="16" height="16"%3E%3C/svg%3E';
        host.setAttribute('icon', image);
        await whenCemRendered(host);
        expect(host.querySelector('img')?.getAttribute('src')).toBe(image);
        expect(host.querySelector('img')?.getAttribute('alt')).toBe('');
        expect(host.querySelector('img')?.getAttribute('aria-hidden')).toBe('true');
        host.setAttribute('name', 'fallback');
        host.setAttribute('icon', '');
        await whenCemRendered(host);
        expect(host.querySelector('[part="icon"]')).toBeNull();
        host.removeAttribute('icon');
        await whenCemRendered(host);
        expect(host.querySelector('[part="icon"]')?.textContent).toBe('fallback');
        host.setAttribute('kind', 'alert');
        host.setAttribute('direction', 'column');
        await whenCemRendered(host);
        expect(link.getAttribute('data-kind')).toBe('alert');
        expect(getComputedStyle(link).flexDirection).toBe('column');
        host.removeAttribute('kind');
        host.removeAttribute('direction');
        await whenCemRendered(host);
        expect(link.getAttribute('data-kind')).toBe('normal');
        expect(getComputedStyle(link).flexDirection).toBe('row');
        host.removeAttribute('href');
        await whenCemRendered(host);
        expect(within(host).getByRole('button', { name: 'Open documentation' })).toHaveTextContent('Documentation');
        expect(host.querySelector('a')).toBeNull();
        expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
    },
});

export const LegacyLinkNavigation = meta.story({
    render: () => '<cem-icon-button href="#icon-link-target" icon="shopping_cart">Open cart</cem-icon-button><div id="icon-link-target">Cart</div>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const link = within(host).getByRole('link', { name: 'Open cart' }) as HTMLAnchorElement;
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const original = location.href;
        const trusted: boolean[] = [];
        link.addEventListener('click', event => { trusted.push(event.isTrusted); event.preventDefault(); });
        try {
            await userEvent.click(link);
            expect(link.getAttribute('href')).toBe('#icon-link-target');
            await whenCemRendered(host);
            expect(storybookCemRuntime().snapshotInstance(host).slices.pressed).toBe('click');
            history.replaceState(null, '', original);
            link.focus();
            await userEvent.keyboard('{Enter}');
            expect(link.getAttribute('href')).toBe('#icon-link-target');
            expect(trusted).toEqual([true, true]);
            for (const value of ['', 'false', 'true']) {
                history.replaceState(null, '', original);
                host.setAttribute('disabled', value);
                await whenCemRendered(host);
                const before = storybookCemRuntime().snapshotInstance(host).eventPayloads;
                expect(link.hasAttribute('href')).toBe(false);
                expect(link.getAttribute('aria-disabled')).toBe('true');
                expect(link.tabIndex).toBe(-1);
                link.click();
                await whenCemRendered(host);
                expect(location.href).toBe(original);
                expect(storybookCemRuntime().snapshotInstance(host).eventPayloads).toEqual(before);
            }
            host.removeAttribute('disabled');
            host.setAttribute('href', '');
            await whenCemRendered(host);
            expect(host.querySelector('a')).toBe(link);
            expect(link.getAttribute('href')).toBe('');
            expect(link.hasAttribute('aria-disabled')).toBe(false);
            expect(link.hasAttribute('tabindex')).toBe(false);
            host.setAttribute('label', 'Named link');
            await whenCemRendered(host);
            expect(link).toHaveAccessibleName('Named link');
        } finally { history.replaceState(null, '', original); }
    },
});

export const LegacyLinkPaint = meta.story({
    render: () => '<section class="cem-theme-light"><cem-icon-button href="#paint" icon="recycling">Recycle</cem-icon-button></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const section = canvasElement.querySelector('section') as HTMLElement;
        const host = section.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const link = host.querySelector('a') as HTMLAnchorElement;
        link.addEventListener('click', event => event.preventDefault());
        const tokenColor = (name: string) => {
            const probe = document.createElement('span');
            probe.style.colorScheme = getComputedStyle(link).colorScheme;
            probe.style.color = getComputedStyle(link).getPropertyValue(name);
            section.append(probe);
            const color = paintedColor(getComputedStyle(probe).color);
            probe.remove();
            return color;
        };
        const paint = async (state: string) => {
            const background = tokenColor(`--cem-action-primary-${state}-background`);
            const foreground = tokenColor(`--cem-action-primary-${state}-text`);
            await waitFor(() => expect(paintedColor(getComputedStyle(link).backgroundColor)).toBe(background));
            expect(paintedColor(getComputedStyle(link).color)).toBe(foreground);
        };
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            section.className = `cem-theme-${theme}`;
            host.removeAttribute('disabled');
            await whenCemRendered(host);
            await userEvent.unhover(link);
            await paint('default');
            for (const kind of ['normal', 'primary', 'secondary', 'alert', 'blend']) {
                host.setAttribute('kind', kind);
                await whenCemRendered(host);
                expect(link.getAttribute('data-kind')).toBe(kind);
                await paint('default');
            }
            const geometry = rectTuple(link);
            await userEvent.hover(link);
            await paint('hover');
            const down = nextTrustedPointerDown(link);
            const click = userEvent.click(link, { delay: 500 });
            await eventBeforeInteractionCompletes(down, click, 'pointerdown');
            await waitForPseudoClass(link, ':active');
            await paint('active');
            await click;
            await whenCemRendered(host);
            await userEvent.keyboard('{Tab}');
            link.focus();
            expect(link.matches(':focus-visible')).toBe(true);
            expect(getComputedStyle(link).outlineStyle).toBe('solid');
            expect(rectTuple(link)).toEqual(geometry);
            host.setAttribute('disabled', 'false');
            await whenCemRendered(host);
            await paint('disabled');
            expect(storybookCemRuntime().diagnosticsFor(host)).toEqual([]);
        }
    },
});

export const NativeActivation = meta.story({
    render: () => '<section class="cem-theme-light"><p>Trusted pointer and keyboard checks run in the browser test runner.</p><cem-icon-button label="Run command"></cem-icon-button></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = canvasElement.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const button = host.querySelector('button') as HTMLButtonElement;
        const runtime = storybookCemRuntime();
        const clicks: boolean[] = [];
        button.addEventListener('click', event => clicks.push(event.isTrusted));
        await userEvent.click(host.querySelector("[part=icon]") as HTMLElement);
        await whenCemRendered(host);
        expect(runtime.snapshotInstance(host).eventPayloads.pressed).toMatchObject({ type: 'click', revision: 1, target: { tag: 'span' }, currentTarget: { tag: 'button' } });
        const before = runtime.snapshotInstance(host);
        await userEvent.keyboard('[Space>]');
        expect(runtime.snapshotInstance(host).eventPayloads).toEqual(before.eventPayloads);
        await userEvent.keyboard('[/Space]');
        await whenCemRendered(host);
        await userEvent.keyboard('{Enter}');
        await whenCemRendered(host);
        expect(clicks).toEqual([true, true, true]);
        expect(runtime.snapshotInstance(host).slices.pressed).toBe('click');
        expect(runtime.snapshotInstance(host).eventPayloads.pressed).toMatchObject({ revision: 3, target: { tag: 'button' } });
        expect(button.hasAttribute('aria-pressed')).toBe(false);
        host.setAttribute('disabled', '');
        await whenCemRendered(host);
        button.click();
        expect(clicks).toHaveLength(3);
        expect(host.querySelector('button')).toBe(button);
    },
});

export const ProjectedContent = meta.story({
    render: () => '<section class="cem-theme-light"><cem-icon-button label="Accessible command"><span data-content="projected">Projected command</span></cem-icon-button></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('cem-icon-button') as HTMLElement;
        await whenCemRendered(host);
        const button = within(host).getByRole('button', { name: 'Accessible command' });
        expect(button.querySelector('[data-content="projected"]')?.textContent).toBe('Projected command');
        expect(host.shadowRoot).toBeNull();
    },
});

export const Hover = meta.story({
    render: () => `
            <section class="cem-theme-light">
                <cem-icon-button name="settings" label="Open settings"></cem-icon-button>
                <cem-icon-button name="settings" label="Disabled settings" disabled></cem-icon-button>
            </section>
        `,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const runtime = storybookCemRuntime();
        await Promise.all(Array.from(canvasElement.querySelectorAll<HTMLElement>('cem-icon-button'), whenCemRendered));
        const harness = {
            root: canvasElement,
            query<T extends Element>(selector: string): T {
                const node = canvasElement.querySelector<T>(selector);
                if (!node) throw new Error(`Missing ${selector}`);
                return node;
            },
        };
        const actionCases = [
            {
                host: harness.query<HTMLElement>('cem-icon-button:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-icon-button:not([disabled]) button'),
                name: 'Open settings',
                role: null,
                tokens: {
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
        ] as const;
        const disabledCases = [
            {
                host: harness.query<HTMLElement>('cem-icon-button[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-icon-button[disabled] button'),
                name: 'Disabled settings',
                role: null,
                tokens: actionCases[0].tokens,
            },
        ] as const;
        const activationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => activationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-icon-button');

        for (const actionCase of actionCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(false);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);
            await assertFocusVisible(button);

            const baseline = captureActionState(runtime, host, button);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));

            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expect(hovered.backgroundColor).not.toBe(baseline.backgroundColor);
            expectActionStructureAndGeometry(hovered, baseline);
            expect(hovered.focusTreatment).toEqual(baseline.focusTreatment);
            expect(document.activeElement).toBe(button);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(restored.focusTreatment).toEqual(baseline.focusTreatment);
            expect(document.activeElement).toBe(button);
        }

        for (const actionCase of disabledCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(true);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);

            const focusOwner = document.activeElement;
            button.focus();
            expect(document.activeElement).toBe(focusOwner);

            const baseline = captureActionState(runtime, host, button);
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expect(baseline.backgroundColor).not.toBe(resolveTokenColor(button, tokens.hoverBackground));

            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(baseline.backgroundColor);
            expect(hovered.color).toBe(baseline.color);
            expectActionStructureAndGeometry(hovered, baseline);
            expect(document.activeElement).toBe(focusOwner);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(document.activeElement).toBe(focusOwner);
        }

        expect(activationEvents).toEqual([]);
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    },
});

export const Active = meta.story({
    render: () => `
            <section class="cem-theme-light">
                <cem-icon-button name="settings" label="Open settings"></cem-icon-button>
                <cem-icon-button name="settings" label="Disabled settings" disabled></cem-icon-button>
            </section>
        `,
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const runtime = storybookCemRuntime();
        await Promise.all(Array.from(canvasElement.querySelectorAll<HTMLElement>('cem-icon-button'), whenCemRendered));
        const harness = {
            root: canvasElement,
            query<T extends Element>(selector: string): T {
                const node = canvasElement.querySelector<T>(selector);
                if (!node) throw new Error(`Missing ${selector}`);
                return node;
            },
        };
        const actionCases = [
            {
                host: harness.query<HTMLElement>('cem-icon-button:not([disabled])'),
                button: harness.query<HTMLButtonElement>('cem-icon-button:not([disabled]) button'),
                name: 'Open settings',
                role: null,
                slice: 'pressed',
                targetTag: 'span',
                tokens: {
                    activeBackground: '--cem-action-contextual-active-background',
                    activeText: '--cem-action-contextual-active-text',
                    defaultBackground: '--cem-action-contextual-default-background',
                    defaultText: '--cem-action-contextual-default-text',
                    hoverBackground: '--cem-action-contextual-hover-background',
                    hoverText: '--cem-action-contextual-hover-text',
                },
            },
        ] as const;
        const disabledCases = [
            {
                host: harness.query<HTMLElement>('cem-icon-button[disabled]'),
                button: harness.query<HTMLButtonElement>('cem-icon-button[disabled] button'),
                name: 'Disabled settings',
                role: null,
                tokens: actionCases[0].tokens,
            },
        ] as const;
        const activationEvents: string[] = [];
        for (const eventName of ['click', 'input', 'change', 'cem-loaded', 'cem-error', 'cem-cancel']) {
            harness.root.addEventListener(eventName, () => activationEvents.push(eventName));
        }

        assertStateHostsRendered(harness.root, 'cem-icon-button');

        for (const [index, actionCase] of actionCases.entries()) {
            const { button, host, name, role, slice, targetTag, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(false);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);
            await assertFocusVisible(button);
            await userEvent.hover(button);
            await nextRenderFrame();

            const hovered = captureActionState(runtime, host, button);
            expect(hovered.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(hovered.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expect(hovered.forcedColorAdjust).toBe('auto');

            const pointerDown = nextTrustedPointerDown(button);
            const click = userEvent.click(button, { delay: 200 });
            const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'pointerdown');
            expect(downEvent.isTrusted).toBe(true);
            await waitForPseudoClass(button, ':active');
            await nextRenderFrame();

            const active = captureActionState(runtime, host, button);
            expect(button.matches(':active')).toBe(true);
            expectPaintedColorToResolveFromToken(active.backgroundColor, button, tokens.activeBackground);
            expectPaintedColorToResolveFromToken(active.color, button, tokens.activeText);
            expect(active.backgroundColor).not.toBe(hovered.backgroundColor);
            expect(contrastRatio(active.backgroundColor, active.color)).toBeGreaterThanOrEqual(4.5);
            expectActionStructureAndGeometry(active, hovered);
            expect(active.focusTreatment).toEqual(hovered.focusTreatment);
            expect(active.forcedColorAdjust).toBe(hovered.forcedColorAdjust);
            expect(document.activeElement).toBe(button);
            expect(activationEvents).toHaveLength(index);

            await click;
            await runtime.whenRenderSettled(host);
            await nextRenderFrame();

            const released = captureActionState(runtime, host, button);
            expect(button.matches(':active')).toBe(false);
            expect(released.backgroundColor).toBe(resolveTokenColor(button, tokens.hoverBackground));
            expect(released.color).toBe(resolveTokenColor(button, tokens.hoverText));
            expectActionStructureAndGeometryAfterActivation(released, hovered);
            expect(released.focusTreatment).toEqual(hovered.focusTreatment);
            expect(released.forcedColorAdjust).toBe('auto');
            expect(released.runtime).not.toBe(active.runtime);
            expect(document.activeElement).toBe(button);
            expect(activationEvents).toEqual(Array.from({ length: index + 1 }, () => 'click'));

            const releaseSnapshot = runtime.snapshotInstance(host);
            const releasePayload = eventPayload(releaseSnapshot, slice);
            expect(releaseSnapshot.slices[slice]).toBe('click');
            expect(releasePayload.type).toBe('click');
            expect(releasePayload.sliceValue).toBe('click');
            expect(releasePayload.currentTarget?.tag).toBe('button');
            expect(releasePayload.target?.tag).toBe(targetTag);

            await userEvent.unhover(button);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(restored.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expectActionStructureAndGeometryAfterActivation(restored, hovered);
            expect(restored.focusTreatment).toEqual(hovered.focusTreatment);
            expect(restored.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).toBe(button);
        }

        for (const actionCase of disabledCases) {
            const { button, host, name, role, tokens } = actionCase;
            expect(button.type).toBe('button');
            expect(button.disabled).toBe(true);
            expect(button.getAttribute('role')).toBe(role);
            expect(assertAccessibleName(button, name)).toBe(name);

            const focusOwner = document.activeElement;
            button.focus();
            expect(document.activeElement).toBe(focusOwner);
            const baseline = captureActionState(runtime, host, button);
            const eventCount = activationEvents.length;
            expect(baseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));
            expect(baseline.color).toBe(resolveTokenColor(button, tokens.defaultText));
            expect(baseline.forcedColorAdjust).toBe('auto');

            const pointerDown = nextTrustedPointerDown(button);
            const click = userEvent.click(button, { delay: 200, force: true });
            const downEvent = await eventBeforeInteractionCompletes(pointerDown, click, 'disabled pointerdown');
            expect(downEvent.isTrusted).toBe(true);
            await nextRenderFrame();

            const held = captureActionState(runtime, host, button);
            expect(held.backgroundColor).toBe(baseline.backgroundColor);
            expect(held.color).toBe(baseline.color);
            expect(held.backgroundColor).not.toBe(resolveTokenColor(button, tokens.activeBackground));
            expectActionStructureAndGeometry(held, baseline);
            expect(held.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).not.toBe(button);
            expect(activationEvents).toHaveLength(eventCount);

            await click;
            await runtime.whenRenderSettled(host);
            await nextRenderFrame();

            const restored = captureActionState(runtime, host, button);
            expect(restored.backgroundColor).toBe(baseline.backgroundColor);
            expect(restored.color).toBe(baseline.color);
            expectActionStructureAndGeometry(restored, baseline);
            expect(restored.forcedColorAdjust).toBe('auto');
            expect(document.activeElement).not.toBe(button);
            expect(activationEvents).toHaveLength(eventCount);
            await userEvent.unhover(button);
        }

        const keyboardCase = actionCases[0];
        const { button, host, slice, tokens } = keyboardCase;
        await userEvent.unhover(button);
        await assertFocusVisible(button);
        const keyboardBaseline = captureActionState(runtime, host, button);
        const keyboardEventCount = activationEvents.length;
        expect(keyboardBaseline.backgroundColor).toBe(resolveTokenColor(button, tokens.defaultBackground));

        await userEvent.keyboard('[Space>]');
        await waitForPseudoClass(button, ':active');
        await nextRenderFrame();

        const keyboardActive = captureActionState(runtime, host, button);
        expect(button.matches(':active')).toBe(true);
        expectPaintedColorToResolveFromToken(keyboardActive.backgroundColor, button, tokens.activeBackground);
        expectPaintedColorToResolveFromToken(keyboardActive.color, button, tokens.activeText);
        expect(keyboardActive.backgroundColor).not.toBe(keyboardBaseline.backgroundColor);
        expect(contrastRatio(keyboardActive.backgroundColor, keyboardActive.color)).toBeGreaterThanOrEqual(4.5);
        expectActionStructureAndGeometry(keyboardActive, keyboardBaseline);
        expect(keyboardActive.focusTreatment).toEqual(keyboardBaseline.focusTreatment);
        expect(keyboardActive.forcedColorAdjust).toBe('auto');
        expect(document.activeElement).toBe(button);
        expect(activationEvents).toHaveLength(keyboardEventCount);

        await userEvent.keyboard('[/Space]');
        await runtime.whenRenderSettled(host);
        await nextRenderFrame();

        const keyboardReleased = captureActionState(runtime, host, button);
        expect(button.matches(':active')).toBe(false);
        expect(keyboardReleased.backgroundColor).toBe(keyboardBaseline.backgroundColor);
        expect(keyboardReleased.color).toBe(keyboardBaseline.color);
        expectActionStructureAndGeometryAfterActivation(keyboardReleased, keyboardBaseline);
        expect(keyboardReleased.focusTreatment).toEqual(keyboardBaseline.focusTreatment);
        expect(keyboardReleased.forcedColorAdjust).toBe('auto');
        const beforeRelease = JSON.parse(keyboardActive.runtime);
        const afterRelease = JSON.parse(keyboardReleased.runtime);
        for (const field of ['formData', 'payload', 'slices', 'validationState']) {
            expect(afterRelease[field]).toEqual(beforeRelease[field]);
        }
        expect(afterRelease.eventPayloads[slice].revision).toBe(beforeRelease.eventPayloads[slice].revision + 1);
        expect(document.activeElement).toBe(button);
        expect(activationEvents).toEqual(Array.from({ length: keyboardEventCount + 1 }, () => 'click'));

        const keyboardReleaseSnapshot = runtime.snapshotInstance(host);
        const keyboardReleasePayload = eventPayload(keyboardReleaseSnapshot, slice);
        expect(keyboardReleaseSnapshot.slices[slice]).toBe('click');
        expect(keyboardReleasePayload.type).toBe('click');
        expect(keyboardReleasePayload.sliceValue).toBe('click');
        expect(keyboardReleasePayload.currentTarget?.tag).toBe('button');
        expect(keyboardReleasePayload.target?.tag).toBe('button');
        expect(() => assertAriaReferenceIntegrity(harness.root)).not.toThrow();
    },
});

interface SerializedEventTarget {
    checked: boolean | null;
    name: string | null;
    tag: string;
    type: string | null;
    value: string | null;
}

interface SerializedEventPayload {
    bubbles: boolean;
    currentTarget: SerializedEventTarget | null;
    sliceValue: unknown;
    target: SerializedEventTarget | null;
    type: string;
}

function assertStateHostsRendered(root: ParentNode, selector: string): void {
    for (const host of Array.from(root.querySelectorAll<HTMLElement>(selector))) {
        assertLightDomRendered(host);
        expect(host.shadowRoot).toBeNull();
    }
}

function eventPayload(snapshot: DataIslandSnapshot, name: string): SerializedEventPayload {
    const payload = snapshot.eventPayloads[name];

    if (!isSerializedEventPayload(payload)) {
        throw new Error(`Expected serialized event payload for ${name}`);
    }

    return payload;
}

function isSerializedEventPayload(value: unknown): value is SerializedEventPayload {
    if (!value || typeof value !== 'object') {
        return false;
    }

    const record = value as Partial<SerializedEventPayload>;

    return typeof record.type === 'string' && 'sliceValue' in record;
}

interface ActionStateSnapshot {
    backgroundColor: string;
    buttonHtml: string;
    buttonRect: readonly number[];
    color: string;
    focusTreatment: readonly string[];
    forcedColorAdjust: string;
    hostAttributes: readonly string[];
    hostRect: readonly number[];
    runtime: string;
}

function captureActionState(
    runtime: CemElementRuntime,
    host: HTMLElement,
    button: HTMLButtonElement,
): ActionStateSnapshot {
    const styles = getComputedStyle(button);
    const runtimeSnapshot = runtime.snapshotInstance(host);

    return {
        backgroundColor: paintedColor(styles.backgroundColor),
        buttonHtml: button.outerHTML,
        buttonRect: rectTuple(button),
        color: paintedColor(styles.color),
        focusTreatment: [styles.outlineColor, styles.outlineStyle, styles.outlineWidth, styles.boxShadow],
        forcedColorAdjust: styles.getPropertyValue('forced-color-adjust'),
        hostAttributes: Array.from(host.attributes, ({ name, value }) => `${name}=${value}`),
        hostRect: rectTuple(host),
        runtime: JSON.stringify({
            eventPayloads: runtimeSnapshot.eventPayloads,
            formData: runtimeSnapshot.formData,
            payload: runtimeSnapshot.payload,
            slices: runtimeSnapshot.slices,
            validationState: runtimeSnapshot.validationState,
        }),
    };
}

function expectActionStructureAndGeometry(actual: ActionStateSnapshot, expected: ActionStateSnapshot): void {
    expectActionStructureAndGeometryAfterActivation(actual, expected);
    expect(actual.runtime).toBe(expected.runtime);
}

function expectActionStructureAndGeometryAfterActivation(
    actual: ActionStateSnapshot,
    expected: ActionStateSnapshot,
): void {
    expect(actual.buttonHtml).toBe(expected.buttonHtml);
    expect(actual.buttonRect).toEqual(expected.buttonRect);
    expect(actual.hostAttributes).toEqual(expected.hostAttributes);
    expect(actual.hostRect).toEqual(expected.hostRect);
}

function nextTrustedPointerDown(button: HTMLElement): Promise<PointerEvent> {
    return new Promise((resolve, reject) => {
        const timeout = window.setTimeout(() => {
            button.removeEventListener('pointerdown', onPointerDown);
            reject(new Error('Expected a trusted pointerdown before the provider interaction completed'));
        }, 1000);
        const onPointerDown = (event: PointerEvent): void => {
            window.clearTimeout(timeout);
            resolve(event);
        };

        button.addEventListener('pointerdown', onPointerDown, { once: true });
    });
}

async function eventBeforeInteractionCompletes<T extends Event>(
    event: Promise<T>,
    interaction: Promise<void>,
    label: string,
): Promise<T> {
    return Promise.race([
        event,
        interaction.then(() => {
            throw new Error(`Expected ${label} while the provider interaction was still held`);
        }),
    ]);
}

async function waitForPseudoClass(element: Element, pseudoClass: string): Promise<void> {
    const deadline = Date.now() + 1000;

    while (Date.now() < deadline) {
        if (element.matches(pseudoClass)) {
            return;
        }
        await nextRenderFrame();
    }

    throw new Error(`Expected ${element.tagName.toLowerCase()} to match ${pseudoClass}`);
}

function contrastRatio(first: string, second: string): number {
    const firstLuminance = relativeLuminance(first);
    const secondLuminance = relativeLuminance(second);
    return (Math.max(firstLuminance, secondLuminance) + 0.05) / (Math.min(firstLuminance, secondLuminance) + 0.05);
}

function relativeLuminance(painted: string): number {
    const channels = painted.split(',').map(Number);
    if (channels.length !== 4 || channels.some((channel) => !Number.isFinite(channel)) || channels[3] !== 255) {
        throw new Error(`Expected an opaque painted RGBA color, received ${painted}`);
    }

    const [red, green, blue] = channels.slice(0, 3).map((channel) => {
        const normalized = channel / 255;
        return normalized <= 0.04045 ? normalized / 12.92 : ((normalized + 0.055) / 1.055) ** 2.4;
    });

    return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function rectTuple(element: Element): readonly number[] {
    const rect = element.getBoundingClientRect();
    return [rect.x, rect.y, rect.width, rect.height];
}

function resolveTokenColor(element: Element, tokenName: string): string {
    const styles = getComputedStyle(element);
    const tokenValue = styles.getPropertyValue(tokenName).trim();

    if (!tokenValue) {
        throw new Error(`Expected generated theme token ${tokenName}`);
    }

    return paintedColor(resolveLightDark(tokenValue, styles.colorScheme));
}

function expectPaintedColorToResolveFromToken(actual: string, element: Element, tokenName: string): void {
    const expected = resolveTokenColor(element, tokenName);
    const actualChannels = actual.split(',').map(Number);
    const expectedChannels = expected.split(',').map(Number);

    expect(actualChannels).toHaveLength(4);
    expect(expectedChannels).toHaveLength(4);
    expect(actualChannels.every((channel) => Number.isFinite(channel))).toBe(true);
    expect(expectedChannels.every((channel) => Number.isFinite(channel))).toBe(true);
    for (const [index, channel] of actualChannels.entries()) {
        expect(Math.abs(channel - expectedChannels[index])).toBeLessThanOrEqual(1);
    }
}

function resolveLightDark(value: string, colorScheme: string): string {
    let resolved = value;
    let start = resolved.indexOf('light-dark(');

    while (start >= 0) {
        const open = start + 'light-dark'.length;
        const close = matchingParen(resolved, open);
        const choices = splitTopLevel(resolved.slice(open + 1, close));

        if (choices.length !== 2) {
            throw new Error(`Expected light-dark() token value to contain two colors: ${value}`);
        }

        const choice = colorScheme.includes('dark') ? choices[1] : choices[0];
        resolved = `${resolved.slice(0, start)}${choice.trim()}${resolved.slice(close + 1)}`;
        start = resolved.indexOf('light-dark(');
    }

    return resolved;
}

function matchingParen(value: string, open: number): number {
    let depth = 0;

    for (let index = open; index < value.length; index += 1) {
        if (value[index] === '(') {
            depth += 1;
        } else if (value[index] === ')') {
            depth -= 1;
            if (depth === 0) {
                return index;
            }
        }
    }

    throw new Error(`Unclosed CSS function in token value: ${value}`);
}

function splitTopLevel(value: string): string[] {
    const values: string[] = [];
    let depth = 0;
    let start = 0;

    for (let index = 0; index < value.length; index += 1) {
        if (value[index] === '(') {
            depth += 1;
        } else if (value[index] === ')') {
            depth -= 1;
        } else if (value[index] === ',' && depth === 0) {
            values.push(value.slice(start, index));
            start = index + 1;
        }
    }

    values.push(value.slice(start));
    return values;
}

function paintedColor(value: string): string {
    const canvas = document.createElement('canvas');
    canvas.width = 1;
    canvas.height = 1;
    const context = canvas.getContext('2d', { willReadFrequently: true });

    if (!context) {
        throw new Error('Expected a 2D canvas context for computed color comparison');
    }

    context.fillStyle = value;
    context.fillRect(0, 0, 1, 1);
    return Array.from(context.getImageData(0, 0, 1, 1).data).join(',');
}

type CemElementRuntime = ReturnType<typeof storybookCemRuntime>;
type DataIslandSnapshot = ReturnType<CemElementRuntime['snapshotInstance']>;

function nextRenderFrame(): Promise<void> {
    return new Promise(resolve => requestAnimationFrame(() => resolve()));
}

function assertAccessibleName(element: Element, expected: string): string {
    expect(element).toHaveAccessibleName(expected);
    return expected;
}

function assertLightDomRendered(host: HTMLElement): void {
    expect(host.shadowRoot).toBeNull();
    expect(host.querySelector('button')).not.toBeNull();
}

async function assertFocusVisible(element: HTMLElement): Promise<void> {
    const { userEvent } = await import('vitest/browser');
    await userEvent.keyboard('{Tab}');
    element.focus();
    await nextRenderFrame();
    expect(document.activeElement).toBe(element);
    const styles = getComputedStyle(element);
    expect((styles.outlineStyle !== 'none' && styles.outlineWidth !== '0px') || styles.boxShadow !== 'none').toBe(true);
}

function assertAriaReferenceIntegrity(root: HTMLElement): void {
    for (const attribute of ['aria-labelledby', 'aria-describedby', 'aria-controls', 'aria-errormessage']) {
        for (const element of root.querySelectorAll(`[${attribute}]`)) {
            for (const id of (element.getAttribute(attribute) ?? '').split(/\s+/).filter(Boolean)) {
                expect(document.getElementById(id)).not.toBeNull();
            }
        }
    }
}
