import { expect, userEvent, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-tooltip.xhtml?raw';
function required<T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
const meta = preview.meta({ component: 'cem-tooltip', title: 'CEM Components/cem-tooltip', loaders: [async () => { await loadCemDeclaration('cem-tooltip', declarationSource); return {}; }] });
export const DescriptionIdentityAndInterest = meta.story({
    render: () => '<section class="cem-theme-light"><span id="tooltip-authored-help">Existing help.</span><cem-tooltip message="Save the current document" show-delay="0" hide-delay="0"><button slot="trigger" type="button" aria-describedby="tooltip-authored-help">Save</button></cem-tooltip><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-tooltip'); await whenCemRendered(host);
        const source = required<HTMLButtonElement>(host, '[slot=trigger]'), owner = required<HTMLElement>(host, '[part=surface]'), id = owner.id;
        expect(owner.getAttribute('role')).toBe('tooltip'); expect(owner.getAttribute('popover')).toBe('manual'); expect(source.getAttribute('aria-describedby')).toBe(`tooltip-authored-help ${id}`);
        expect(source).toHaveAccessibleName('Save'); expect(source).toHaveAccessibleDescription('Existing help. Save the current document');
        source.focus(); await waitFor(() => expect(owner.matches(':popover-open')).toBe(true)); expect(document.activeElement).toBe(source);
        source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); required<HTMLElement>(canvasElement, 'section > button').focus(); expect(owner.matches(':popover-open')).toBe(true);
        owner.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); source.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' })); expect(owner.matches(':popover-open')).toBe(true);
        await userEvent.keyboard('{Escape}'); expect(owner.matches(':popover-open')).toBe(false);
        host.setAttribute('message', 'Updated help'); await whenCemRendered(host);
        expect(host.querySelector('[part=surface]')).toBe(owner); expect(owner.id).toBe(id); expect(owner.matches(':popover-open')).toBe(false);
        expect(source.getAttribute('aria-describedby')).toBe(`tooltip-authored-help ${id}`); expect(source).toHaveAccessibleDescription('Existing help. Updated help');
        host.remove(); expect(source.getAttribute('aria-describedby')).toBe('tooltip-authored-help');
    },
});
export const EditorTriggersAndDisabledTouch = meta.story({
    render: () => '<section><cem-tooltip message="Your display name" hide-delay="0"><input slot="trigger" aria-label="Name"></cem-tooltip><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-tooltip'); await whenCemRendered(host);
        const source = required<HTMLInputElement>(host, 'input'), owner = required<HTMLElement>(host, '[part=surface]');
        source.dispatchEvent(new PointerEvent('pointerdown', { pointerType: 'touch', bubbles: true })); source.focus();
        expect(owner.matches(':popover-open')).toBe(false); await userEvent.keyboard('{ArrowRight}'); expect(owner.matches(':popover-open')).toBe(true);
        host.setAttribute('disabled', 'false'); await whenCemRendered(host); await waitFor(() => expect(owner.matches(':popover-open')).toBe(false));
        expect(source.disabled).toBe(false); expect(source.getAttribute('aria-describedby')).toBe(owner.id);
        host.removeAttribute('disabled'); await whenCemRendered(host); await waitFor(() => expect(owner.matches(':popover-open')).toBe(true));
        source.disabled = true; await waitFor(() => expect(owner.matches(':popover-open')).toBe(false));
    },
});
export const ScopedExternalAndProjectedDescription = meta.story({
    render: () => '<section><div interaction-scope><button type="button" interaction-name="help">First</button><cem-tooltip trigger-for="@help" message="Fallback" hide-delay="0"><strong slot="label">Projected help</strong></cem-tooltip></div><div interaction-scope><button type="button" interaction-name="help">Second</button><cem-tooltip trigger-for="@help" message="Second help" hide-delay="0"></cem-tooltip></div></section>',
    play: async ({ canvasElement }) => {
        const hosts = [...canvasElement.querySelectorAll<HTMLElement>('cem-tooltip')]; await Promise.all(hosts.map(whenCemRendered));
        const sources = [...canvasElement.querySelectorAll<HTMLButtonElement>('button')], owners = hosts.map(host => required<HTMLElement>(host, '[part=surface]'));
        expect(sources[0].getAttribute('aria-describedby')).toBe(owners[0].id); expect(sources[1].getAttribute('aria-describedby')).toBe(owners[1].id);
        expect(sources[0]).toHaveAccessibleDescription('Projected help'); sources[0].focus(); await waitFor(() => expect(owners[0].matches(':popover-open')).toBe(true));
        expect(owners[1].matches(':popover-open')).toBe(false); sources[1].focus(); await waitFor(() => expect(owners[1].matches(':popover-open')).toBe(true)); expect(owners[0].matches(':popover-open')).toBe(false);
        hosts[1].remove(); expect(sources[1].hasAttribute('aria-describedby')).toBe(false);
    },
});
