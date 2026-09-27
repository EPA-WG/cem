import { expect, userEvent, within } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, storybookCemRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-action.xhtml?raw';

const meta = preview.meta({
    component: 'cem-action',
    title: 'CEM Components/cem-action',
    loaders: [async () => { await loadCemDeclaration('cem-action', declarationSource); return {}; }],
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
