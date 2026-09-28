import { expect, userEvent, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-action.xhtml?raw';

const meta = preview.meta({
    component: 'cem-action',
    title: 'CEM Components/cem-action',
    loaders: [async () => { await loadCemDeclaration('cem-action', declarationSource); return {}; }],
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
            getComputedStyle(button).backgroundColor;
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
            const stripe = expected(host.getAttribute('variant') === 'destructive' ? 'pending-stripe' : 'active')[0];
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
