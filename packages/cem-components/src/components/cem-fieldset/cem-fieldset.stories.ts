import { expect, userEvent, within, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, loadCemWorkflowDependencies, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-fieldset.xhtml?raw';
import '../../styles.css';

const meta = preview.meta({
    component: 'cem-fieldset', title: 'CEM Components/cem-fieldset',
    loaders: [async () => { await loadCemDeclaration('cem-fieldset', declarationSource); await loadCemWorkflowDependencies(); return {}; }],
});

async function verifyFormOwnership(canvasElement: HTMLElement, fallback: boolean) {
    const { runtime, declare, scope } = createCemStoryRuntime(canvasElement, fallback);
    const suffix = crypto.randomUUID();
    const tags = { group: `cem-group-${suffix}`, field: 'cem-text-field', select: 'cem-select', action: 'cem-action' };
    await declare(declarationSource, tags.group);
    const form = document.createElement('form');
    form.innerHTML = `<${tags.group} label="Contact details" description="Choose where to receive updates.">
        <${tags.field} name="email" label="Email" value="reader@example.test" required></${tags.field}>
        <${tags.select} name="channel" label="Channel" value="email"><option value="email">Email</option><option value="post">Post</option></${tags.select}>
        <${tags.action} type="submit" variant="primary">Save contact</${tags.action}>
    </${tags.group}><label>Outside group <input name="outside" value="kept"></label>`;
    canvasElement.append(form);
    const host = form.querySelector<HTMLElement>(tags.group)!;
    const settle = async () => { await runtime.whenRenderSettled(host); for (const el of host.querySelectorAll<HTMLElement>([tags.field, tags.select, tags.action].join(','))) await whenCemRendered(el); };
    try {
        await settle();
        const group = within(form).getByRole('group', { name: 'Contact details' }) as HTMLFieldSetElement;
        expect(group.localName).toBe('fieldset');
        expect(group).toHaveAccessibleDescription('Choose where to receive updates.');
        const field = form.querySelector<HTMLElement>(tags.field)!;
        await waitFor(() => expect(field.querySelector('input')).toBeTruthy());
        const input = within(group).getByRole('textbox', { name: 'Email' }) as HTMLInputElement;
        const choice = within(group).getByRole('combobox', { name: 'Channel' }) as HTMLButtonElement;
        const submit = within(group).getByRole('button', { name: 'Save contact' }) as HTMLButtonElement;
        const outside = within(form).getByRole('textbox', { name: 'Outside group' }) as HTMLInputElement;
        input.value = 'edited@example.test'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await settle();
        expect([...new FormData(form).entries()]).toEqual([['email', 'edited@example.test'], ['channel', 'email'], ['outside', 'kept']]);
        let submissions = 0;
        form.addEventListener('submit', event => { event.preventDefault(); submissions++; });
        submit.click(); expect(submissions).toBe(1);
        const descriptionId = group.getAttribute('aria-describedby');
        expect(descriptionId).toBeTruthy();
        for (const disabled of ['', 'false']) {
            host.setAttribute('disabled', disabled); await settle();
            await waitFor(() => expect(input.disabled).toBe(true));
            expect(choice.disabled).toBe(true);
            expect(submit.matches(':disabled')).toBe(true);
            submit.click(); expect(submissions).toBe(1);
            expect([...new FormData(form).entries()]).toEqual([['outside', 'kept']]);
            expect(outside.matches(':disabled')).toBe(false);
            expect(host.querySelector('fieldset')).toBe(group);
            expect(field.querySelector('input')).toBe(input);
            expect(input.value).toBe('edited@example.test');
            expect(group.getAttribute('aria-describedby')).toBe(descriptionId);
        }
        host.removeAttribute('disabled'); await settle();
        await waitFor(() => expect(input.disabled).toBe(false));
        submit.click(); expect(submissions).toBe(2);
        form.reset(); await settle();
        expect(input.value).toBe('reader@example.test');
        host.setAttribute('label', 'Updated contact details'); host.setAttribute('description', 'Updated help'); await settle();
        expect(group).toHaveAccessibleName('Updated contact details');
        expect(group).toHaveAccessibleDescription('Updated help');
        expect(group.getAttribute('aria-describedby')).toBe(descriptionId);
        form.remove(); canvasElement.append(form); await settle();
        expect(host.querySelector('fieldset')).toBe(group);
        expect(field.querySelector('input')).toBe(input);
        host.hidden = true; await settle(); expect(getComputedStyle(host).display).toBe('none');
        expect(runtime.diagnosticsFor(host).filter(d => d.severity === 'error' || d.severity === 'fatal')).toEqual([]);
    } finally { form.remove(); scope.dispose(); }
}

export const WorkerFormOwnership = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => { await verifyFormOwnership(canvasElement, false); } });
export const FallbackFormOwnership = meta.story({ render: () => '<div></div>', play: async ({ canvasElement }) => { await verifyFormOwnership(canvasElement, true); } });

export const LabelsAndRetainedProjection = meta.story({
    render: () => '<cem-fieldset label="Fallback group" description="Fallback help"><strong slot="label">Projected group</strong><span slot="help">Projected help</span><label>Draft <input value="Initial"></label></cem-fieldset><cem-fieldset label="Second group" description="Independent help"><label>Other <input></label></cem-fieldset>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-fieldset')];
        await Promise.all(hosts.map(whenCemRendered));
        const [first, second] = hosts.map(host => host.querySelector<HTMLFieldSetElement>('fieldset')!);
        expect(first).toHaveAccessibleName('Projected group'); expect(first).toHaveAccessibleDescription('Projected help');
        expect(second).toHaveAccessibleName('Second group'); expect(second).toHaveAccessibleDescription('Independent help');
        expect(first.getAttribute('aria-describedby')).not.toBe(second.getAttribute('aria-describedby'));
        const input = first.querySelector('input')!, label = first.querySelector('[slot=label]')!, help = first.querySelector('[slot=help]')!;
        await userEvent.type(input, ' edited');
        input.focus(); input.setSelectionRange(1, 4);
        hosts[0].setAttribute('label', 'Changed fallback'); hosts[0].setAttribute('description', 'Changed fallback help');
        await whenCemRendered(hosts[0]);
        expect(first).toHaveAccessibleName('Projected group'); expect(first).toHaveAccessibleDescription('Projected help');
        expect(hosts[0].querySelector('input')).toBe(input); expect(input.value).toBe('Initial edited');
        expect(document.activeElement).toBe(input); expect([input.selectionStart, input.selectionEnd]).toEqual([1, 4]);
        expect(hosts[0].contains(label)).toBe(true); expect(hosts[0].contains(help)).toBe(true);
        hosts[0].setAttribute('aria-label', 'Accessible group'); await whenCemRendered(hosts[0]);
        expect(first).toHaveAccessibleName('Accessible group'); expect(first.querySelector('legend')?.textContent).toBe('Projected group');
        expect(first.hasAttribute('tabindex')).toBe(false); expect(first.hasAttribute('role')).toBe(false);
    },
});

export const NativeDisabledAndKeyboard = meta.story({
    name: 'Native disabled groups and keyboard (browser runner)',
    render: () => '<form><label>Before group <input name="before" value="before"></label><cem-fieldset label="Preferences" disabled="false"><cem-text-field label="Name" name="name" value="Reader"></cem-text-field><cem-select label="Language" name="language" value="en"><option value="en">English</option><option value="fr">French</option></cem-select><cem-fieldset label="Nested preferences"><cem-text-field label="Nested field" name="nested" value="nested"></cem-text-field></cem-fieldset><cem-action type="submit">Save preferences</cem-action></cem-fieldset><label>After group <input name="after" value="after"></label></form>',
    play: async ({ canvasElement }) => {
        const form = canvasElement.querySelector('form')!;
        const host = form.querySelector<HTMLElement>('cem-fieldset')!;
        for (const el of form.querySelectorAll<HTMLElement>('cem-fieldset,cem-text-field,cem-select,cem-action')) await whenCemRendered(el);
        const group = host.querySelector('fieldset')!;
        expect(group.disabled).toBe(true);
        expect(new FormData(form).has('name')).toBe(false);
        expect(new FormData(form).has('language')).toBe(false);
        expect(new FormData(form).has('nested')).toBe(false);
        const before = within(form).getByRole('textbox', { name: 'Before group' });
        const after = within(form).getByRole('textbox', { name: 'After group' });
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        await native.click(before); await native.keyboard('{Tab}');
        expect(document.activeElement).toBe(after);
        host.removeAttribute('disabled'); await whenCemRendered(host);
        await waitFor(() => expect(new FormData(form).get('nested')).toBe('nested'));
        expect(new FormData(form).get('name')).toBe('Reader');
        expect(new FormData(form).get('language')).toBe('en');
        await native.click(before); await native.keyboard('{Tab}');
        expect(document.activeElement).toBe(within(group).getByRole('textbox', { name: 'Name' }));
        // Native pointer/Tab checks run only in the browser runner.
    },
});

export const LegendExemptionAndChildState = meta.story({
    render: () => '<form><cem-fieldset disabled><label slot="label">Enable settings <input type="checkbox" name="enabled" value="yes" checked></label><cem-text-field label="Normal setting" name="normal" value="normal"></cem-text-field><cem-text-field label="Independently disabled" name="locked" value="locked" disabled></cem-text-field></cem-fieldset></form>',
    play: async ({ canvasElement }) => {
        const form = canvasElement.querySelector('form')!;
        const host = form.querySelector<HTMLElement>('cem-fieldset')!;
        for (const el of form.querySelectorAll<HTMLElement>('cem-fieldset,cem-text-field')) await whenCemRendered(el);
        const legendControl = within(form).getByRole('checkbox', { name: 'Enable settings' }) as HTMLInputElement;
        expect(legendControl.matches(':disabled')).toBe(false);
        expect([...new FormData(form).entries()]).toEqual([['enabled', 'yes']]);
        host.removeAttribute('disabled'); await whenCemRendered(host);
        await waitFor(() => expect(new FormData(form).get('normal')).toBe('normal'));
        expect(new FormData(form).has('locked')).toBe(false);
        expect(within(form).getByRole('textbox', { name: 'Independently disabled' })).toBeDisabled();
    },
});

export const ThemeGeometry = meta.story({
    render: () => '<div class="theme-audit" style="inline-size:300px"><cem-fieldset label="A long settings group name that wraps within narrow layouts" description="UnbrokenLongDescriptionTextThatMustNotWidenThePageOrCoverAnotherControl"><cem-text-field label="Display name" name="display" value="Reader"></cem-text-field><cem-action>Save settings</cem-action></cem-fieldset></div>',
    play: async ({ canvasElement }) => {
        const container = canvasElement.querySelector<HTMLElement>('.theme-audit')!;
        const host = container.querySelector<HTMLElement>('cem-fieldset')!;
        for (const el of container.querySelectorAll<HTMLElement>('cem-fieldset,cem-text-field,cem-action')) await whenCemRendered(el);
        const group = host.querySelector('fieldset')!, body = host.querySelector<HTMLElement>('[part=body]')!;
        const input = await within(host).findByRole('textbox', { name: 'Display name' }) as HTMLInputElement;
        const reference = group.getAttribute('aria-describedby');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            container.className = `theme-audit cem-theme-${mode}`;
            container.style.setProperty('--cem-gap-group', '27px');
            container.style.setProperty('--cem-inset-container', '23px');
            container.style.setProperty('--cem-bend-surface', '7px');
            container.style.setProperty('--cem-typography-ui-font-size', '19px');
            await whenCemRendered(host);
            const probe = document.createElement('span');
            host.append(probe); probe.style.color = 'var(--cem-palette-comfort-text)'; probe.style.backgroundColor = 'var(--cem-palette-comfort)';
            expect(getComputedStyle(group).color).toBe(getComputedStyle(probe).color);
            expect(getComputedStyle(group).backgroundColor).toBe(getComputedStyle(probe).backgroundColor);
            probe.remove();
            expect(getComputedStyle(group).paddingInlineStart).toBe('23px');
            expect(getComputedStyle(group).borderTopLeftRadius).toBe('7px');
            expect(getComputedStyle(group).fontSize).toBe('19px');
            expect(getComputedStyle(body).rowGap).toBe('27px');
            for (const direction of ['ltr', 'rtl']) {
                host.dir = direction; await whenCemRendered(host);
                expect(group.getBoundingClientRect().width).toBeLessThanOrEqual(container.getBoundingClientRect().width + 1);
                expect(body.scrollWidth).toBeLessThanOrEqual(body.clientWidth + 1);
                expect(host.querySelector('input')).toBe(input);
                expect(group.getAttribute('aria-describedby')).toBe(reference);
            }
        }
    },
});
