import { expect, userEvent, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-action.xhtml?raw';

const meta = preview.meta({
    component: 'cem-action',
    title: 'CEM Components/cem-action',
    loaders: [async () => { await loadCemDeclaration('cem-action', declarationSource); return {}; }],
});

export const Dimensions = meta.story({
    render: () => `<section class="cem-theme-light" data-cem-size="large" style="display:grid;gap:var(--cem-coupling-guard-min);justify-items:start">
        <cem-action class="cem-bend-round">Inherited</cem-action>
        ${['small', 'medium', 'large', 'x-large', 'xx-large'].map(size => `<cem-action size="${size}" class="cem-bend-round">${size}</cem-action>`).join('')}
        <div style="display:flex;width:320px;height:160px"><cem-action size="medium" style="flex:1">Stretched</cem-action></div>
        <cem-action size="small" aria-label="Icon action"><span aria-hidden="true">+</span></cem-action>
    </section>`,
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('section') as HTMLElement;
        const hosts = [...container.querySelectorAll<HTMLElement>('cem-action')];
        for (const host of hosts) await whenCemRendered(host);
        const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
        const control = (host: HTMLElement) => host.querySelector('button') as HTMLButtonElement;
        const check = async (host: HTMLElement, height: number) => {
            const button = control(host);
            const rect = button.getBoundingClientRect();
            const style = getComputedStyle(button);
            await expect(rect.height).toBeCloseTo(Math.max(3, height) * rem);
            await expect(rect.width).toBeGreaterThanOrEqual(3 * rem);
            await expect(parseFloat(style.borderTopWidth)).toBeCloseTo(Math.max(0, (3 - height) / 2) * rem);
            await expect(style.backgroundClip).toBe('padding-box');
            await expect(parseFloat(style.borderRadius) * 2).toBeCloseTo(rect.height);
            await expect(host.getBoundingClientRect().height).toBeCloseTo(rect.height);
        };
        for (const [i, height] of [4, 2.5, 3, 4, 6, 8].entries()) await check(hosts[i], height);
        const original = control(hosts[0]);
        hosts[0].setAttribute('size', 'small');
        await whenCemRendered(hosts[0]);
        await check(hosts[0], 2.5);
        hosts[0].removeAttribute('size');
        await whenCemRendered(hosts[0]);
        await check(hosts[0], 4);
        container.setAttribute('data-cem-size', 'x-large');
        await check(hosts[0], 6);
        await check(hosts[1], 2.5);
        hosts[0].setAttribute('size', 'unknown');
        await whenCemRendered(hosts[0]);
        await check(hosts[0], 6);
        container.setAttribute('data-cem-size', 'xx-large');
        await check(hosts[0], 8);
        container.style.setProperty('--cem-control-height-xx-large', '9rem');
        await check(hosts[0], 9);
        await check(hosts[5], 9);
        container.style.removeProperty('--cem-control-height-xx-large');
        await expect(control(hosts[0])).toBe(original);
        const stretched = control(hosts[6]).getBoundingClientRect();
        await expect(stretched.width).toBe(320);
        await expect(stretched.height).toBe(160);
        await expect(parseFloat(getComputedStyle(control(hosts[6])).borderTopWidth)).toBe(0);
        const icon = control(hosts[7]).getBoundingClientRect();
        await expect(icon.width).toBeGreaterThanOrEqual(3 * rem);
        await expect(icon.height).toBeGreaterThanOrEqual(3 * rem);
        // Consumers may override a public profile without losing the safety minimum.
        container.style.setProperty('--cem-control-height-small', '2rem');
        await check(hosts[1], 2);
        container.removeAttribute('data-cem-size');
        hosts[0].removeAttribute('size');
        await whenCemRendered(hosts[0]);
        await check(hosts[0], 2.5);
        hosts[0].style.setProperty('--cem-action-border-radius', '3px');
        await whenCemRendered(hosts[0]);
        await expect(parseFloat(getComputedStyle(original).borderRadius) - parseFloat(getComputedStyle(original).borderTopWidth)).toBe(3);
        hosts[0].style.removeProperty('--cem-action-border-radius');
        await whenCemRendered(hosts[0]);
        await check(hosts[0], 2.5);
        const root = document.documentElement;
        const coupling = root.getAttribute('data-cem-coupling');
        try {
            for (const [mode, height] of [['compact', 2.25], ['forgiving', 2.75]] as const) {
                root.setAttribute('data-cem-coupling', mode);
                await check(hosts[0], height);
                await check(hosts[2], 3);
            }
        } finally {
            if (coupling === null) root.removeAttribute('data-cem-coupling');
            else root.setAttribute('data-cem-coupling', coupling);
        }
    },
});

export const Selected = meta.story({
    parameters: { docs: { description: { story: 'Container-owned selection across themes and combined states. Trusted keyboard focus checks run in the browser test runner.' } } },
    render: () => `<section class="cem-theme-light"><cem-action>Command</cem-action><cem-action selectable>Choice</cem-action><cem-action selected>Chosen</cem-action></section>`,
    play: async ({ canvasElement }) => {
        const section = canvasElement.querySelector('section') as HTMLElement;
        const hosts = [...section.querySelectorAll<HTMLElement>('cem-action')];
        for (const host of hosts) await whenCemRendered(host);
        const buttons = hosts.map(host => host.querySelector('button') as HTMLButtonElement);
        await expect(buttons[0]).not.toHaveAttribute('aria-pressed');
        await expect(buttons[1]).toHaveAttribute('aria-pressed', 'false');
        await expect(buttons[2]).toHaveAttribute('aria-pressed', 'true');
        await expect(buttons[2]).not.toHaveAttribute('aria-selected');
        const selected = hosts[2], button = buttons[2];
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            section.className = `cem-theme-${mode}`;
            const baseline = button.getBoundingClientRect();
            button.blur();
            const ring = getComputedStyle(button).boxShadow;
            await expect(ring).not.toBe('none');
            await userEvent.click(button);
            button.blur();
            await expect(selected.hasAttribute('selected')).toBe(true);
            await expect(getComputedStyle(button).boxShadow).toBe(ring);
            await userEvent.click(buttons[1]);
            await expect(hosts[1].hasAttribute('selected')).toBe(false);
            if (import.meta.env.MODE === 'test') {
                const { userEvent: native } = await import('vitest/browser');
                buttons[1].focus();
                await native.keyboard('{Tab}');
                await expect(document.activeElement).toBe(button);
                await expect(button.matches(':focus-visible')).toBe(true);
                await expect(getComputedStyle(button).boxShadow).not.toBe(ring);
            } else button.focus();
            await expect(button).toHaveAttribute('aria-pressed', 'true');
            button.blur();
            selected.setAttribute('disabled', '');
            await whenCemRendered(selected);
            await expect(getComputedStyle(button).boxShadow).toBe(ring);
            selected.setAttribute('loading', 'true');
            await whenCemRendered(selected);
            await expect(button.disabled).toBe(true);
            await expect(getComputedStyle(button).boxShadow).toBe(ring);
            await expect(button.getBoundingClientRect().width).toBe(baseline.width);
            await expect(button.getBoundingClientRect().height).toBe(baseline.height);
            for (const attribute of ['disabled', 'loading']) selected.removeAttribute(attribute);
            await whenCemRendered(selected);
        }
        selected.setAttribute('selected', 'false');
        await whenCemRendered(selected);
        await expect(button).toHaveAttribute('aria-pressed', 'true');
        selected.setAttribute('selectable', '');
        selected.removeAttribute('selected');
        await whenCemRendered(selected);
        button.blur();
        await expect(button).toHaveAttribute('aria-pressed', 'false');
        await expect(getComputedStyle(button).boxShadow).toBe('none');
        selected.removeAttribute('selectable');
        await whenCemRendered(selected);
        await expect(button).not.toHaveAttribute('aria-pressed');
    },
});

export const ContentDimensions = meta.story({
    render: () => `<section class="cem-theme-light" style="display:grid;gap:var(--cem-coupling-guard-min);width:320px;max-width:100%">
        <cem-action size="medium">A standalone label that can wrap within its container</cem-action>
        <cem-action size="x-large"><span style="display:grid;gap:var(--cem-dim-small);min-width:0;text-align:start">
            <img alt="" width="320" height="180" style="width:100%;height:auto" src="data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 320 180'%3E%3Cpath fill='%23548c89' d='M0 180 160 0 320 180z'/%3E%3C/svg%3E">
            <strong>Choose the mountain image for this page</strong>
        </span></cem-action>
        <cem-action size="xx-large"><span style="display:grid;gap:var(--cem-dim-small);min-width:0;text-align:start">
            <strong>Choose the mountain campaign</strong>
            <span>Featured content grows with the available space. A long description must remain visible when the container gets narrow, including an unbroken reference: mountain_campaign_preview_image_for_the_featured_page.</span>
        </span></cem-action>
    </section>`,
    play: async ({ canvasElement }) => {
        const section = canvasElement.querySelector('section') as HTMLElement;
        const hosts = [...section.querySelectorAll<HTMLElement>('cem-action')];
        for (const host of hosts) await whenCemRendered(host);
        const buttons = hosts.map(host => host.querySelector('button') as HTMLButtonElement);
        const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
        const wideHeight = buttons[2].getBoundingClientRect().height;
        section.style.width = '160px';
        await expect(buttons[2].getBoundingClientRect().height).toBeGreaterThan(wideHeight);
        for (const [index, button] of buttons.entries()) {
            await expect(button.getBoundingClientRect().height).toBeGreaterThanOrEqual([3, 6, 8][index] * rem);
            await expect(button.scrollWidth).toBeLessThanOrEqual(button.clientWidth);
            await expect(parseFloat(getComputedStyle(button).borderTopWidth)).toBe(0);
            await expect(button.querySelectorAll('a,button,input,select,textarea').length).toBe(0);
        }
        let activations = 0;
        buttons[1].addEventListener('click', () => activations++);
        await userEvent.click(buttons[1].querySelector('strong') as HTMLElement);
        await expect(activations).toBe(1);
        await expect(buttons[1]).toHaveAccessibleName('Choose the mountain image for this page');
    },
});

export const CompactHitArea = meta.story({
    parameters: { docs: { description: { story: 'Trusted edge clicks run in the Vitest browser runner; the entire transparent compact border belongs to the native button.' } } },
    render: () => `<form class="cem-theme-light" style="display:flex;gap:var(--cem-coupling-guard-min)">
        <cem-action size="small" type="submit">Save</cem-action>
        <cem-action size="small" disabled loading="true">Disabled</cem-action>
    </form>`,
    play: async ({ canvasElement }) => {
        const form = canvasElement.querySelector('form') as HTMLFormElement;
        const hosts = [...form.querySelectorAll<HTMLElement>('cem-action')];
        for (const host of hosts) await whenCemRendered(host);
        const [button, disabled] = hosts.map(host => host.querySelector('button') as HTMLButtonElement);
        let clicks = 0, submits = 0, disabledClicks = 0;
        button.addEventListener('click', () => clicks++);
        disabled.addEventListener('click', () => disabledClicks++);
        form.addEventListener('submit', event => { event.preventDefault(); submits++; });
        const first = button.getBoundingClientRect();
        const second = disabled.getBoundingClientRect();
        const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
        await expect(second.left - first.right).toBeGreaterThanOrEqual(.5 * rem);
        await expect(document.elementFromPoint(first.left + 1, first.top + first.height / 2)).toBe(button);
        await expect(document.elementFromPoint(first.right + (second.left - first.right) / 2, first.top + first.height / 2)).not.toBe(button);
        await expect(getComputedStyle(disabled).backgroundImage).toContain('linear-gradient(45deg');
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        await native.click(button, { position: { x: 1, y: first.height / 2 } });
        await expect(clicks).toBe(1);
        await expect(submits).toBe(1);
        await native.click(disabled, { position: { x: 1, y: second.height / 2 }, force: true });
        await expect(disabledClicks).toBe(0);
        await expect(submits).toBe(1);
        button.focus();
        await native.keyboard('[Space]');
        await expect(clicks).toBe(2);
        await expect(submits).toBe(2);
        await expect(button.matches(':focus-visible')).toBe(true);
        await expect(getComputedStyle(button).boxShadow).not.toBe('none');
    },
});

export const ContrastContours = meta.story({
    globals: { cemTheme: 'light' },
    parameters: { docs: { description: { story: 'Contrast modes keep flat surfaces and animate the pending zebra contour. Trusted pointer checks run in the browser runner.' } } },
    render: () => ['contrast-light', 'contrast-dark'].map(mode => `<section class="cem-theme-${mode}" style="display:flex;flex-wrap:wrap;gap:1rem">${
        ['primary', 'explicit', 'contextual', 'alternate', 'destructive'].map(intent => `<cem-action variant="${intent}">${intent}</cem-action>`).join('')
    }</section>`).join(''),
    play: async ({ canvasElement }) => {
        const native = import.meta.env.MODE === 'test' ? (await import('vitest/browser')).userEvent : null;
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-action')) {
            await whenCemRendered(host);
            const button = host.querySelector('button') as HTMLButtonElement;
            const section = host.parentElement as HTMLElement;
            const probe = document.createElement('span');
            probe.style.backgroundColor = 'var(--cem-palette-comfort)';
            section.append(probe);
            const surface = getComputedStyle(probe).backgroundColor;
            probe.remove();
            const contour = () => getComputedStyle(button, '::before');
            const rect = button.getBoundingClientRect();
            await expect(getComputedStyle(button).backgroundColor).toBe(surface);
            await expect(colorContrast(surface, getComputedStyle(button).color)).toBeGreaterThanOrEqual(4.5);
            await expect(contour().display).toBe('block');
            await expect(contour().maskComposite.split(', ').every(value => value === 'exclude')).toBe(true);
            await expect(parseFloat(contour().paddingTop)).toBe(1);
            await expect(colorContrast(surface, contour().backgroundColor)).toBeGreaterThanOrEqual(3);
            if (native) {
                await native.hover(button);
                await expect(parseFloat(contour().paddingTop)).toBe(2);
                button.focus();
                await native.keyboard('[Space>]');
                await expect(parseFloat(contour().paddingTop)).toBe(3);
                await expect(getComputedStyle(button).backgroundColor).toBe(surface);
                await native.keyboard('[/Space]');
                await native.unhover(button);
            }
            host.setAttribute('loading', 'true');
            await whenCemRendered(host);
            await expect(contour().backgroundImage).toContain('linear-gradient(45deg');
            await expect(contour().animationIterationCount).toBe('infinite');
            await expect(contour().animationDuration).toBe('2s');
            await expect(getComputedStyle(button).backgroundColor).toBe(surface);
            const animations = button.getAnimations({ subtree: true });
            await expect(animations.length).toBeGreaterThan(0);
            for (const animation of animations) { animation.pause(); animation.currentTime = 500; }
            const position = contour().backgroundPositionX;
            for (const animation of animations) animation.currentTime = 1000;
            await expect(contour().backgroundPositionX).not.toBe(position);
            for (const animation of animations) animation.currentTime = 2500;
            await expect(contour().backgroundPositionX).toBe(position);
            for (const animation of animations) animation.play();
            if (native) {
                button.focus();
                await native.keyboard('[Space]');
                await expect(button.matches(':focus-visible')).toBe(true);
                await expect(getComputedStyle(button).boxShadow).not.toBe('none');
            }
            host.setAttribute('disabled', '');
            await whenCemRendered(host);
            await expect(contour().animationIterationCount).toBe('infinite');
            await expect(contour().display).toBe('block');
            await expect(getComputedStyle(button).backgroundColor).toBe(surface);
            let clicks = 0;
            button.addEventListener('click', () => clicks++);
            button.click();
            await expect(clicks).toBe(0);
            await expect(button.getBoundingClientRect().height).toBe(rect.height);
            await expect(button.getBoundingClientRect().width).toBe(rect.width);
            const mode = section.className;
            section.className = 'cem-theme-light';
            await expect(contour().display).toBe('none');
            section.className = mode;
            await expect(contour().display).toBe('block');
            host.setAttribute('loading', 'false');
            await whenCemRendered(host);
            await expect(button.getAnimations()).toHaveLength(0);
            await expect(contour().backgroundColor).toBe(getComputedStyle(button).color);
            button.blur();
        }
    },
});

export const ThemeOverrides = meta.story({
    parameters: { docs: { description: { story: 'Container and individual host overrides use existing theme tokens. Trusted hover/active checks run only in the Vitest browser runner.' } } },
    render: () => `<section class="cem-theme-light"><cem-action>First</cem-action><cem-action>Sibling</cem-action></section>`,
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector('section')!;
        const hosts = [...container.querySelectorAll<HTMLElement>('cem-action')];
        for (const host of hosts) await whenCemRendered(host);
        const [host, sibling] = hosts;
        const button = host.querySelector('button')!;
        const other = sibling.querySelector('button')!;
        const paint = (control = button) => [getComputedStyle(control).backgroundColor, getComputedStyle(control).color];
        const inherited = ['rgb(21, 42, 63)', 'rgb(240, 241, 242)'];
        const individual = ['rgb(63, 42, 21)', 'rgb(230, 231, 232)'];
        const native = import.meta.env.MODE === 'test' ? (await import('vitest/browser')).userEvent : null;
        for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
            for (const item of hosts) item.setAttribute('variant', intent);
            for (const item of hosts) await whenCemRendered(item);
            if (native) await native.unhover(button);
            const original = paint();
            const properties = ['default', 'hover', 'active', 'disabled', 'pending'].flatMap(state =>
                ['background', 'text'].map(channel => `--cem-action-${intent}-${state}-${channel}`));
            properties.forEach((property, index) => container.style.setProperty(property, inherited[index % 2]));
            await expect(paint()).toEqual(inherited);
            await expect(paint(other)).toEqual(inherited);
            properties.forEach((property, index) => host.style.setProperty(property, individual[index % 2]));
            await whenCemRendered(host);
            await expect(paint()).toEqual(individual);
            await expect(paint(other)).toEqual(inherited);
            if (native) {
                await native.hover(button);
                await expect(button.matches(':hover')).toBe(true);
                await expect(paint()).toEqual(individual);
                button.focus();
                await native.keyboard('[Space>]');
                try {
                    await expect(button.matches(':active')).toBe(true);
                    await expect(paint()).toEqual(individual);
                } finally {
                    await native.keyboard('[/Space]');
                }
                await native.unhover(button);
                button.blur();
            }
            host.setAttribute('disabled', '');
            await whenCemRendered(host);
            await expect(paint()).toEqual(individual);
            host.setAttribute('loading', 'true');
            await whenCemRendered(host);
            await expect(paint()).toEqual(individual);
            await expect(getComputedStyle(button).backgroundImage).toContain(individual[0]);
            properties.forEach(property => host.style.removeProperty(property));
            await whenCemRendered(host);
            await expect(paint()).toEqual(inherited);
            await expect(getComputedStyle(button).backgroundImage).toContain(inherited[0]);
            host.removeAttribute('disabled');
            host.removeAttribute('loading');
            await whenCemRendered(host);
            properties.forEach(property => container.style.removeProperty(property));
            await expect(paint()).toEqual(original);
            await expect(paint(other)).toEqual(original);
            await expect(host.querySelector('button')).toBe(button);
        }
    },
});

export const NativeHidden = meta.story({
    parameters: { docs: { description: { story: 'Native host visibility and state retention. Trusted keyboard checks run only in the Vitest browser runner.' } } },
    render: () => `<button data-before>Before</button><cem-action hidden loading="true" label="Save"></cem-action><button data-after>After</button>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-action')!;
        await whenCemRendered(host);
        const control = host.querySelector('button')!;
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
            await expect(host.querySelector('button')!).toBe(control);
            await expect(control).toHaveAttribute('aria-busy', 'true');
            await expect(control).toHaveTextContent('Save');
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
            await expect(host.querySelector('button')!).toBe(control);
        }
        host.removeAttribute('hidden');
        await whenCemRendered(host);
        await expect(host.querySelector('button')!).toBe(control);
        await expect(control).toHaveAttribute('aria-busy', 'true');
        await expect(control).toHaveTextContent('Save');
        if (import.meta.env.MODE === 'test') {
            const { userEvent: native } = await import('vitest/browser');
            canvasElement.querySelector<HTMLButtonElement>('[data-before]')!.focus();
            await native.keyboard('{Tab}');
            await expect(document.activeElement).toBe(control);
        }
    },
});

export const LoadingColors = meta.story({
    // Keep this branded light fixture independent of the default native palette.
    globals: { cemTheme: 'light' },
    parameters: { docs: { description: { story: 'Pending gradients and continuous loading motion. Trusted hover/focus checks run only in the Vitest browser runner.' } } },
    render: () => `<section class="cem-theme-light">${['primary', 'explicit', 'contextual', 'alternate', 'destructive'].map(variant =>
        `<cem-action variant="${variant}" loading="false">Save</cem-action>`).join('')}</section>`,
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-action')) {
            await whenCemRendered(host);
            const button = host.querySelector('button')!;
            const initial = button.getBoundingClientRect();
            const paint = () => [getComputedStyle(button).backgroundColor, getComputedStyle(button).color];
            const expected = (state: string) => {
                const probe = document.createElement('span');
                host.append(probe);
                probe.style.backgroundColor = `var(--cem-action-${host.getAttribute('variant')}-${state}-background)`;
                probe.style.color = `var(--cem-action-${host.getAttribute('variant')}-${state}-text)`;
                const result = [getComputedStyle(probe).backgroundColor, getComputedStyle(probe).color];
                probe.remove();
                return result;
            };
            await expect(paint()).toEqual(expected('default'));
            host.setAttribute('loading', 'true');
            await whenCemRendered(host);
            await expect(button).toHaveAttribute('aria-busy', 'true');
            void getComputedStyle(button).backgroundColor;
            const transitions = button.getAnimations();
            if (!matchMedia('(prefers-reduced-motion: reduce)').matches) {
                await expect(transitions.length).toBeGreaterThan(0);
            }
            const animation = transitions[0];
            if (animation) {
                await expect(animation.effect!.getTiming().iterations).toBe(Infinity);
                await expect(animation.effect!.getTiming().duration).toBe(2000);
                animation.pause();
                animation.currentTime = 500;
                const firstCycle = getComputedStyle(button).backgroundPositionX;
                animation.currentTime = 1000;
                await expect(getComputedStyle(button).backgroundPositionX).not.toBe(firstCycle);
                animation.currentTime = 2500;
                await expect(getComputedStyle(button).backgroundPositionX).toBe(firstCycle);
                animation.play();
            }
            await expect(getComputedStyle(button).backgroundImage).toContain('linear-gradient(45deg');
            await expect(colorContrast(...expected('pending') as [string, string])).toBeGreaterThanOrEqual(4.5);
            const stripe = expected('pending-stripe')[0];
            await expect(colorContrast(stripe, expected('pending')[1])).toBeGreaterThanOrEqual(4.5);
            if (host.getAttribute('variant') === 'destructive') {
                await expect(getComputedStyle(button).backgroundImage).toContain(stripe);
                await expect(colorContrast(stripe, expected('pending')[0])).toBeGreaterThan(1.8);
            }
            await expect(paint()).toEqual(expected('pending'));
            await expect(host.querySelector('button')).toBe(button);
            await expect(button.getBoundingClientRect().width).toBe(initial.width);
            await expect(button.getBoundingClientRect().height).toBe(initial.height);
            if (import.meta.env.MODE === 'test' && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
                const { userEvent: native } = await import('vitest/browser');
                await native.hover(button);
                await expect(button.getAnimations()[0]).toBe(animation);
                await native.unhover(button);
                await expect(button.getAnimations()[0]).toBe(animation);
                button.focus();
                await native.keyboard('{Tab}');
                await native.keyboard('{Shift>}{Tab}{/Shift}');
                await expect(button.matches(':focus-visible')).toBe(true);
                await expect(button.getAnimations()[0]).toBe(animation);
                await expect(getComputedStyle(button).boxShadow).not.toBe('none');
                button.blur();
                await native.unhover(button);
            }
            host.setAttribute('disabled', '');
            await whenCemRendered(host);
            await expect(button.disabled).toBe(true);
            await expect(paint()).toEqual(expected('pending'));
            await expect(getComputedStyle(button).backgroundImage).toContain('linear-gradient(45deg');
            if (animation) await expect(button.getAnimations()[0]).toBe(animation);
            let clicks = 0;
            button.addEventListener('click', () => clicks++);
            button.click();
            await expect(clicks).toBe(0);
            host.setAttribute('loading', 'false');
            await whenCemRendered(host);
            await expect(paint()).toEqual(expected('disabled'));
            await expect(getComputedStyle(button).backgroundImage).toBe('none');
            await expect(button.getAnimations()).toHaveLength(0);
            host.removeAttribute('disabled');
            await whenCemRendered(host);
            await expect(button).toHaveAttribute('aria-busy', 'false');
            await expect(paint()).toEqual(expected('default'));
            await expect(button.getAnimations()).toHaveLength(0);
        }
    },
});

export const DestructivePendingModes = meta.story({
    // Branded mode samples must not inherit the Storybook native palette override.
    globals: { cemTheme: 'light' },
    render: () => ['native', 'light', 'dark', 'contrast-light', 'contrast-dark'].map(mode =>
        `<section class="cem-theme-${mode}"><cem-action variant="destructive" loading="true" disabled>${mode}</cem-action></section>`).join(''),
    play: async ({ canvasElement }) => {
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-action')) {
            await whenCemRendered(host);
            const button = host.querySelector('button')!;
            const probe = document.createElement('span');
            host.append(probe);
            probe.style.backgroundColor = 'var(--cem-action-destructive-pending-background)';
            const pending = getComputedStyle(probe).backgroundColor;
            probe.style.backgroundColor = 'var(--cem-action-destructive-pending-stripe-background)';
            const stripe = getComputedStyle(probe).backgroundColor;
            probe.remove();
            const paint = getComputedStyle(button);
            await expect(paint.backgroundImage).toContain(stripe);
            if (host.parentElement?.className.includes('contrast-')) {
                await expect(pending).toBe(stripe);
                await expect(getComputedStyle(button, '::before').backgroundImage).toContain('linear-gradient(45deg');
                await expect(getComputedStyle(button, '::before').display).toBe('block');
            } else {
                await expect(colorContrast(pending, stripe)).toBeGreaterThan(1.8);
            }
            await expect(colorContrast(pending, paint.color)).toBeGreaterThanOrEqual(4.5);
            await expect(colorContrast(stripe, paint.color), `${host.parentElement!.className}: stripe=${stripe}, text=${paint.color}`).toBeGreaterThanOrEqual(4.5);
            await expect(button.disabled).toBe(true);
            if (!matchMedia('(prefers-reduced-motion: reduce)').matches) {
                await expect(button.getAnimations()[0].effect!.getTiming().iterations).toBe(Infinity);
            }
        }
    },
});

export const PendingIntentThemeParity = meta.story({
    globals: { cemTheme: 'light' },
    parameters: { docs: { description: { story: 'Light contrast outlines brighten the loading stripe; normal fills and dark contours preserve their colors and all modes share motion parameters.' } } },
    render: () => '<section class="cem-theme-light"><cem-action loading="true">Loading</cem-action></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-action');
        if (!host?.parentElement) throw new Error('Missing action theme fixture');
        const section = host.parentElement;
        await whenCemRendered(host);
        const button = host.querySelector('button');
        if (!button) throw new Error('Missing rendered action control');
        const snapshot = (pseudo?: string) => {
            const style = getComputedStyle(button, pseudo);
            return [style.backgroundImage, style.backgroundSize, style.animationDuration,
                style.animationTimingFunction, style.animationIterationCount];
        };
        const images = new Map<string, string>();
        for (const scheme of ['light', 'dark']) {
            const intentImages = new Set<string>();
            for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
                host.setAttribute('variant', intent);
                await whenCemRendered(host);
                section.className = `cem-theme-${scheme}`;
                const normal = snapshot();
                await expect(normal[0]).toContain('linear-gradient(45deg');
                await expect([...normal[0].matchAll(/ (\d+)%/g)].map(match => Number(match[1])))
                    .toEqual([0, 5, 20, 30, 45, 55, 70, 80, 95, 100]);
                const probe = document.createElement('span');
                section.append(probe);
                const readColor = (state: string) => {
                    probe.style.color = `var(--cem-action-${intent}-${state}-background)`;
                    return getComputedStyle(probe).color;
                };
                const pending = readColor('pending');
                const stripe = readColor('pending-stripe');
                const ink = getComputedStyle(button).color;
                await expect(normal[0]).toContain(stripe);
                await expect(colorContrast(pending, stripe), `${scheme} ${intent}: stripe separation`).toBeGreaterThan(1.8);
                await expect(colorContrast(pending, ink)).toBeGreaterThanOrEqual(4.5);
                await expect(colorContrast(stripe, ink)).toBeGreaterThanOrEqual(4.5);
                for (const weight of [25, 50, 75]) {
                    probe.style.color = `color-mix(in srgb, ${pending} ${weight}%, ${stripe})`;
                    await expect(colorContrast(getComputedStyle(probe).color, ink)).toBeGreaterThanOrEqual(4.5);
                }
                const expectedContour = [...normal];
                if (scheme === 'light') {
                    probe.style.color = intent === 'destructive'
                        ? 'var(--cem-color-red-xl)'
                        : `color-mix(in srgb, var(--cem-action-${intent}-active-background) 60%, var(--cem-color-white))`;
                    const brighterStripe = getComputedStyle(probe).color;
                    if (intent === 'destructive') await expect(brighterStripe).toBe('rgb(255, 180, 171)');
                    await expect(colorContrast(pending, brighterStripe), `${intent}: light outline separation`).toBeGreaterThan(3);
                    await expect(brighterStripe).not.toBe(stripe);
                    expectedContour[0] = normal[0].replaceAll(stripe, brighterStripe);
                }
                probe.remove();
                intentImages.add(normal[0]);
                images.set(`${scheme}-${intent}`, normal[0]);
                for (const disabled of [false, true]) {
                    host.toggleAttribute('disabled', disabled);
                    await whenCemRendered(host);
                    section.className = `cem-theme-contrast-${scheme}`;
                    await expect(snapshot('::before')).toEqual(expectedContour);
                    await expect(getComputedStyle(button, '::before').display).toBe('block');
                    section.className = `cem-theme-${scheme}`;
                    await expect(snapshot()).toEqual(normal);
                }
            }
            await expect(intentImages.size).toBe(5);
        }
        for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
            await expect(images.get(`light-${intent}`)).not.toBe(images.get(`dark-${intent}`));
        }
        // Both painted areas share customized motion parameters as well as defaults.
        section.style.setProperty('--cem-pending-angle', '60deg');
        section.style.setProperty('--cem-pending-tile-size', '3rem');
        section.style.setProperty('--cem-duration-pending-cycle', '3500ms');
        const customized = snapshot();
        section.className = 'cem-theme-contrast-dark';
        await expect(snapshot('::before')).toEqual(customized);
        await expect(customized[0]).toContain('linear-gradient(60deg');
        await expect(customized[2]).toBe('3.5s');
    },
});

export const SubmittedPending = meta.story({
    render: () => '<form><cem-action type="submit">Send</cem-action></form>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-action')!;
        await whenCemRendered(host);
        const button = host.querySelector('button')!;
        let submissions = 0;
        canvasElement.querySelector('form')!.addEventListener('submit', event => {
            event.preventDefault();
            submissions++;
            host.setAttribute('loading', 'true');
            host.setAttribute('disabled', '');
        });
        await userEvent.click(button);
        await whenCemRendered(host);
        await expect(submissions).toBe(1);
        await expect(button.disabled).toBe(true);
        await expect(button).toHaveAttribute('aria-busy', 'true');
        await expect(getComputedStyle(button).backgroundImage).toContain('linear-gradient(45deg');
        button.click();
        await expect(submissions).toBe(1);
        await userEvent.tab();
        await expect(document.activeElement).not.toBe(button);
        host.setAttribute('loading', 'false');
        await whenCemRendered(host);
        await expect(button.disabled).toBe(true);
        await expect(button.getAnimations()).toHaveLength(0);
        await expect(getComputedStyle(button).backgroundImage).toBe('none');
    },
});

export const LabelsAndStyles = meta.story({
    render: () => `<cem-action></cem-action><cem-action label="Save"></cem-action>
        <cem-action label="Fallback" aria-label="Publish changes"><strong>Publish</strong><span> changes</span></cem-action>`,
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-action')];
        await Promise.all(hosts.map(whenCemRendered));
        const canvas = within(canvasElement);
        await expect(canvas.getByRole('button', { name: 'Action', exact: true })).toBeVisible();
        await expect(canvas.getByRole('button', { name: 'Save', exact: true })).toBeVisible();
        const control = canvas.getByRole('button', { name: 'Publish changes' }) as HTMLButtonElement;
        await expect(control.type).toBe('button');
        await expect(control.className).toBe('cem-action cem-action--primary');
        await expect(control).toHaveAttribute('part', 'control');
        await expect(control.querySelector('strong')).toHaveTextContent('Publish');
        for (const host of hosts) {
            await expect(host.shadowRoot).toBeNull();
            await expect(host.querySelector('style')).toBeNull();
        }
        const styles = document.querySelectorAll('cem-element[data-cem-storybook-declaration="cem-action"] > style[data-cem-declaration-style="private"]');
        await expect(styles.length).toBe(1);
        await expect(styles[0].textContent).toMatch(/@scope\s*\(\s*cem-action\s*\)/u);
    },
});

export const StatesAndActivation = meta.story({
    render: () => `<cem-action loading="true" expanded="false">Sync</cem-action>
        <cem-action disabled>Unavailable</cem-action><cem-action disabled="false">Presence</cem-action>`,
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-action')];
        await Promise.all(hosts.map(whenCemRendered));
        const control = hosts[0].querySelector('button')!;
        await expect(control.getAttribute('aria-busy')).toBe('true');
        await expect(control.getAttribute('aria-expanded')).toBe('false');
        await expect(control.disabled).toBe(false);
        const clicks: Event[] = [];
        control.addEventListener('click', event => clicks.push(event));
        await userEvent.click(control);
        await whenCemRendered(hosts[0]);
        const snapshot = storybookCemRuntime().snapshotInstance(hosts[0]);
        await expect(snapshot.slices.pressed).toBe('click');
        await expect(snapshot.eventPayloads.pressed).toMatchObject({ type: 'click', sliceValue: 'click', target: { tag: 'button' } });
        await expect(clicks.length).toBe(1);
        await expect(hosts[0].querySelector('button')).toBe(control);
        for (const host of hosts.slice(1)) {
            const button = host.querySelector('button')!;
            await expect(button.disabled).toBe(true);
            let count = 0;
            button.addEventListener('click', () => count++);
            button.click();
            await expect(count).toBe(0);
        }
    },
});

export const LiveAttributes = meta.story({
    render: () => `<cem-action label="Live"><strong>Stable payload</strong></cem-action>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-action')!;
        await whenCemRendered(host);
        const control = host.querySelector('button')!;
        const payload = control.querySelector('strong');
        for (const disabled of ['', 'false', 'true', null]) {
            if (disabled === null) host.removeAttribute('disabled');
            else host.setAttribute('disabled', disabled);
            await new Promise<void>(resolve => queueMicrotask(resolve));
            await whenCemRendered(host);
            await expect(control.disabled).toBe(disabled !== null);
            await expect(host.querySelector('button')).toBe(control);
            await expect(control.querySelector('strong')).toBe(payload);
        }
        for (const type of ['submit', 'reset', 'button']) {
            host.setAttribute('type', type);
            await new Promise<void>(resolve => queueMicrotask(resolve));
            await whenCemRendered(host);
            await expect(control.type).toBe(type);
        }
    },
});

export const NativeForms = meta.story({
    render: () => `<form id="action-form"><input aria-label="Required name" name="title" required value="initial" />
        <cem-action>Command</cem-action>
        <cem-action type="submit" name="intent" value="save">Submit</cem-action>
        <cem-action type="reset">Reset</cem-action>
        <cem-action type="submit" disabled>Disabled submit</cem-action></form>
        <cem-action type="submit" form="action-form" name="intent" value="external" formaction="/action-override" formmethod="post" formenctype="text/plain" formtarget="action-target" formnovalidate>External submit</cem-action>`,
    play: async ({ canvasElement }) => {
        await Promise.all([...canvasElement.querySelectorAll<HTMLElement>('cem-action')].map(whenCemRendered));
        const canvas = within(canvasElement);
        const form = canvasElement.querySelector('form')!;
        const input = form.querySelector('input')!;
        const submissions: Array<{ submitter: HTMLElement | null; entries: Array<[string, FormDataEntryValue]> }> = [];
        form.addEventListener('submit', event => {
            event.preventDefault();
            submissions.push({ submitter: event.submitter, entries: [...new FormData(form, event.submitter)] });
        });
        const submit = canvas.getByRole('button', { name: 'Submit', exact: true }) as HTMLButtonElement;
        await userEvent.click(canvas.getByRole('button', { name: 'Command', exact: true }));
        await expect(submissions.length).toBe(0);
        input.value = '';
        await userEvent.click(submit);
        await expect(submissions.length).toBe(0);
        await expect(input.validity.valueMissing).toBe(true);
        input.value = 'ready';
        await userEvent.click(submit);
        await expect(submissions[0]).toEqual({ submitter: submit, entries: [['title', 'ready'], ['intent', 'save']] });
        const disabled = canvas.getByRole('button', { name: 'Disabled submit' }) as HTMLButtonElement;
        await expect(disabled.disabled).toBe(true);
        disabled.click();
        await expect(submissions.length).toBe(1);
        const external = canvas.getByRole('button', { name: 'External submit' }) as HTMLButtonElement;
        await expect(external.form).toBe(form);
        await expect(external.formAction).toBe(new URL('/action-override', location.href).href);
        await expect(external.formMethod).toBe('post');
        await expect(external.formEnctype).toBe('text/plain');
        await expect(external.formTarget).toBe('action-target');
        await expect(external.formNoValidate).toBe(true);
        input.value = '';
        await userEvent.click(external);
        await expect(submissions[1]).toEqual({ submitter: external, entries: [['title', ''], ['intent', 'external']] });
        const cancel = (event: Event) => event.preventDefault();
        submit.addEventListener('click', cancel, { once: true });
        input.value = 'changed';
        await userEvent.click(submit);
        await expect(submissions.length).toBe(2);
        form.addEventListener('reset', cancel, { once: true });
        await userEvent.click(canvas.getByRole('button', { name: 'Reset', exact: true }));
        await expect(input.value).toBe('changed');
        await userEvent.click(canvas.getByRole('button', { name: 'Reset', exact: true }));
        await expect(input.value).toBe('initial');
    },
});

export const NativePointerAndKeyboard = meta.story({
    parameters: { docs: { description: { story: 'Trusted hover, active paint and keyboard assertions run only in the Vitest browser runner. Use the same controls manually in Storybook.' } } },
    render: () => `<cem-action><strong>Activate</strong></cem-action><cem-action disabled>Disabled</cem-action>`,
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector<HTMLElement>('cem-action')!;
        await whenCemRendered(host);
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const button = host.querySelector('button')!;
        const payload = button.querySelector('strong');
        const rect = () => ({ width: button.getBoundingClientRect().width, height: button.getBoundingClientRect().height });
        const initial = rect();
        const paint = (state: string) => {
            const probe = document.createElement('span');
            host.append(probe);
            probe.style.backgroundColor = `var(--cem-action-primary-${state}-background)`;
            probe.style.color = `var(--cem-action-primary-${state}-text)`;
            const expected = [getComputedStyle(probe).backgroundColor, getComputedStyle(probe).color];
            probe.remove();
            return expected;
        };
        const colors = () => [getComputedStyle(button).backgroundColor, getComputedStyle(button).color];
        await native.hover(button);
        await expect(button.matches(':hover')).toBe(true);
        await expect(colors()).toEqual(paint('hover'));
        const before = storybookCemRuntime().snapshotInstance(host);
        const down = new Promise<PointerEvent>(resolve => button.addEventListener('pointerdown', resolve, { once: true }));
        const click = Promise.resolve(native.click(button, { delay: 200 }));
        const event = await down;
        await expect(event.isTrusted).toBe(true);
        await expect(button.matches(':active')).toBe(true);
        await expect(colors()).toEqual(paint('active'));
        await expect(rect()).toEqual(initial);
        await expect(storybookCemRuntime().snapshotInstance(host).slices).toEqual(before.slices);
        await click;
        await whenCemRendered(host);
        await expect(storybookCemRuntime().snapshotInstance(host).slices.pressed).toBe('click');
        await expect(host.querySelector('button')).toBe(button);
        await expect(button.querySelector('strong')).toBe(payload);
        let clicks = 0;
        button.addEventListener('click', () => clicks++);
        button.focus();
        await native.keyboard('[Space>]');
        await expect(button.matches(':active')).toBe(true);
        await expect(colors()).toEqual(paint('active'));
        await expect(clicks).toBe(0);
        await native.keyboard('[/Space]');
        await whenCemRendered(host);
        await expect(clicks).toBe(1);
        await native.keyboard('{Enter}');
        await whenCemRendered(host);
        await expect(clicks).toBe(2);
        await expect(rect()).toEqual(initial);
    },
});

export const VariationsAndFocus = meta.story({
    parameters: { docs: { description: { story: 'All five action intents and disabled counterparts. Trusted hover/active/focus assertions run only in the Vitest browser runner.' } } },
    render: () => `<section class="cem-theme-light">${['primary', 'explicit', 'contextual', 'alternate', 'destructive'].map(variant =>
        `<cem-action variant="${variant}" class="cem-bend-round">${variant}</cem-action><cem-action variant="${variant}" disabled>${variant} disabled</cem-action>`).join('')}</section>`,
    play: async ({ canvasElement }) => {
        await Promise.all([...canvasElement.querySelectorAll<HTMLElement>('cem-action')].map(whenCemRendered));
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        for (const host of canvasElement.querySelectorAll<HTMLElement>('cem-action:not([disabled])')) {
            const button = host.querySelector('button')!;
            const variant = host.getAttribute('variant');
            const expected = (state: string) => {
                const probe = document.createElement('span');
                host.append(probe);
                probe.style.backgroundColor = `var(--cem-action-${variant}-${state}-background)`;
                probe.style.color = `var(--cem-action-${variant}-${state}-text)`;
                const colors = [getComputedStyle(probe).backgroundColor, getComputedStyle(probe).color];
                probe.remove();
                return colors;
            };
            const paint = () => [getComputedStyle(button).backgroundColor, getComputedStyle(button).color];
            await native.unhover(button);
            await expect(paint()).toEqual(expected('default'));
            await expect(parseFloat(getComputedStyle(button).borderRadius) * 2).toBe(button.getBoundingClientRect().height);
            await native.hover(button);
            await expect(paint()).toEqual(expected('hover'));
            const down = new Promise(resolve => button.addEventListener('pointerdown', resolve, { once: true }));
            const click = Promise.resolve(native.click(button, { delay: 150 }));
            await down;
            await expect(paint()).toEqual(expected('active'));
            await expect(colorContrast(...paint() as [string, string])).toBeGreaterThanOrEqual(4.5);
            await click;
            await whenCemRendered(host);
            const disabled = host.nextElementSibling!.querySelector('button')!;
            await expect(disabled.disabled).toBe(true);
            await native.hover(disabled);
            await expect([getComputedStyle(disabled).backgroundColor, getComputedStyle(disabled).color]).toEqual(expected('disabled'));
            button.focus();
            await native.keyboard('{Tab}');
            await native.keyboard('{Shift>}{Tab}{/Shift}');
            await expect(button.matches(':focus-visible')).toBe(true);
            const probe = document.createElement('span');
            button.append(probe);
            const stripes = [1, 2, 3].map(index => {
                probe.style.boxShadow = `0 0 0 calc(${index} * var(--cem-zebra-strip-size)) var(--cem-zebra-color-${index})`;
                return getComputedStyle(probe).boxShadow;
            });
            probe.remove();
            await expect(getComputedStyle(button).boxShadow).toBe(stripes.join(', '));
        }
    },
});

function colorContrast(first: string, second: string): number {
    const context = document.createElement('canvas').getContext('2d')!;
    const luminance = (color: string) => {
        context.clearRect(0, 0, 1, 1);
        context.fillStyle = color;
        context.fillRect(0, 0, 1, 1);
        const [r, g, b] = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(value => {
            const channel = value / 255;
            return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
    };
    const a = luminance(first), b = luminance(second);
    return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}
