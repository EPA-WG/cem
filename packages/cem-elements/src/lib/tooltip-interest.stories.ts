import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { connectCemNativeSurface } from './native-surface.js';

export default { title: 'CEM Elements/Native Tooltip Interest', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
export const SlottedInterestAndDescription: Story = {
    render: () => '<section><span id="tooltip-existing-help">Existing help.</span><div show-delay="0" hide-delay="0"><input slot="trigger" aria-label="Name" aria-describedby="tooltip-existing-help"><span part="surface" role="tooltip" popover="manual">Supplemental help.</span></div><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('div')!, source = host.querySelector('input')!, owner = host.querySelector<HTMLElement>('[part=surface]')!;
        const controller = connectCemNativeSurface(owner, { host });
        try {
            expect(source.getAttribute('aria-describedby')?.split(' ')).toEqual(['tooltip-existing-help', owner.id]);
            source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' }));
            await waitFor(() => expect(owner.matches(':popover-open')).toBe(true));
            source.focus(); source.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' }));
            expect(owner.matches(':popover-open')).toBe(true); expect(document.activeElement).toBe(source);
            source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); canvasElement.querySelector('button')!.focus();
            expect(owner.matches(':popover-open')).toBe(true);
            owner.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); source.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' }));
            expect(owner.matches(':popover-open')).toBe(true);
            await userEvent.keyboard('{Escape}'); expect(owner.matches(':popover-open')).toBe(false);
            host.setAttribute('placement', 'block-end center'); await Promise.resolve(); expect(owner.matches(':popover-open')).toBe(false);
            owner.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' })); source.focus();
            await waitFor(() => expect(owner.matches(':popover-open')).toBe(true));
            host.setAttribute('disabled', 'false'); await waitFor(() => expect(owner.matches(':popover-open')).toBe(false));
            expect(source.getAttribute('aria-describedby')).toContain(owner.id);
            controller.disconnect(); expect(source.getAttribute('aria-describedby')).toBe('tooltip-existing-help');
        } finally { controller.disconnect(); }
    },
};
export const TouchAndPendingCancellation: Story = {
    render: () => '<section><div show-delay="50" hide-delay="0"><button type="button" slot="trigger">Help</button><span part="surface" role="tooltip" popover="manual">Touch stays native.</span></div><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('div')!, source = host.querySelector('button')!, owner = host.querySelector<HTMLElement>('[part=surface]')!;
        const controller = connectCemNativeSurface(owner, { host });
        try {
            const touch = new PointerEvent('pointerdown', { pointerType: 'touch', bubbles: true, cancelable: true });
            source.dispatchEvent(touch); source.focus(); source.dispatchEvent(new PointerEvent('pointerup', { pointerType: 'touch', bubbles: true }));
            await new Promise(resolve => setTimeout(resolve, 70)); expect(touch.defaultPrevented).toBe(false); expect(owner.matches(':popover-open')).toBe(false);
            await userEvent.keyboard('{ArrowRight}'); expect(owner.matches(':popover-open')).toBe(true);
            canvasElement.querySelector<HTMLElement>('section > button')!.focus(); expect(owner.matches(':popover-open')).toBe(false);
            source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); await userEvent.keyboard('{Escape}');
            await new Promise(resolve => setTimeout(resolve, 70)); expect(owner.matches(':popover-open')).toBe(false);
            source.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' })); source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' })); controller.disconnect();
            await new Promise(resolve => setTimeout(resolve, 70)); expect(owner.matches(':popover-open')).toBe(false); expect(source.hasAttribute('aria-describedby')).toBe(false);
        } finally { controller.disconnect(); }
    },
};
export const InterestDelaySurvivesUnrelatedUpdates: Story = {
    render: () => '<section><div show-delay="40" hide-delay="40"><button type="button" slot="trigger">Help</button><span part="surface" role="tooltip" popover="manual">Stable delay.</span></div><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = canvasElement.querySelector('div')!, source = host.querySelector('button')!, owner = host.querySelector<HTMLElement>('[part=surface]')!, outside = canvasElement.querySelector('section > button')!;
        const controller = connectCemNativeSurface(owner, { host });
        let updates = 0;
        const timer = setInterval(() => outside.setAttribute('aria-label', `Other control ${++updates}`), 10);
        try {
            source.dispatchEvent(new PointerEvent('pointerenter', { pointerType: 'mouse' }));
            await waitFor(() => expect(owner.matches(':popover-open')).toBe(true), { timeout: 1000 });
            source.dispatchEvent(new PointerEvent('pointerleave', { pointerType: 'mouse' }));
            await waitFor(() => expect(owner.matches(':popover-open')).toBe(false), { timeout: 1000 });
            expect(updates).toBeGreaterThan(0);
        } finally { clearInterval(timer); controller.disconnect(); }
    },
};
