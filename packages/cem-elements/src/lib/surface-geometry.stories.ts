import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemSurfaceGeometryLease, fitNativeSurface, releasePopupGeometry, observePopupGeometry } from './popup-controller.js';
import { connectCemNativeSurface } from './native-surface.js';

export default { title: 'CEM Elements/Surface Geometry Leases', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required(root: ParentNode, selector: string): HTMLElement {
    const node = root.querySelector<HTMLElement>(selector);
    if (!node) throw new Error(`Missing ${selector}`);
    return node;
}
async function frames(): Promise<void> {
    for (let i = 0; i < 3; i++) await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
}
const fixture = () => '<section><input aria-label="First anchor" style="position:fixed;left:30px;top:40px;width:80px"><input aria-label="Second anchor" style="position:fixed;left:300px;top:40px;width:100px"><div popover="manual" style="width:70px;height:30px">First surface</div><div popover="manual" style="width:90px;height:30px">Second surface</div></section>';

export const IndependentSurfacesAndObserverLifetimes: Story = {
    render: fixture,
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section'), a = required(host, 'input'), b = required(host, 'input:nth-of-type(2)'),
            first = required(host, 'div'), second = required(host, 'div:nth-of-type(2)');
        let firstUpdates = 0, secondUpdates = 0;
        const one = createCemSurfaceGeometryLease(host, first, () => { firstUpdates++; one.fit(a, undefined, 'block-end start'); });
        const two = createCemSurfaceGeometryLease(host, second, () => { secondUpdates++; two.fit(b, undefined, 'block-end start'); });
        first.showPopover(); second.showPopover();
        try {
            await expect(one.fit(a, undefined, 'block-end start')).toBe(true);
            await expect(two.fit(b, undefined, 'block-end start')).toBe(true);
            await frames(); const beforeFirst = firstUpdates, beforeSecond = secondUpdates;
            a.style.width = '120px';
            await waitFor(() => expect(firstUpdates).toBeGreaterThan(beforeFirst));
            await frames(); await expect(secondUpdates).toBe(beforeSecond);
            one.release(); const afterRelease = firstUpdates, stillSecond = secondUpdates;
            a.style.width = '140px'; b.style.width = '150px';
            await waitFor(() => expect(secondUpdates).toBeGreaterThan(stillSecond));
            await frames(); await expect(firstUpdates).toBe(afterRelease);
            await expect(first.style.position).toBe('');
            await expect(second.style.position).toBe('fixed');
            await expect(first.matches(':popover-open')).toBe(true);
            await expect(second.matches(':popover-open')).toBe(true);
        } finally { one.release(); two.release(); first.hidePopover(); second.hidePopover(); }
    },
};

export const AuthoredClaimsAndStaleLeaseFences: Story = {
    render: fixture,
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section'), anchor = required(host, 'input'), panel = required(host, 'div');
        panel.style.setProperty('left', '7px', 'important'); panel.style.overflow = 'hidden'; panel.setAttribute('data-placement', 'authored');
        panel.style.setProperty('margin-left', '13px', 'important'); panel.style.marginTop = '3px';
        panel.style.setProperty('overflow-x', 'hidden', 'important'); panel.style.overflowY = 'scroll';
        const lease = createCemSurfaceGeometryLease(host, panel, () => undefined);
        panel.showPopover();
        try {
            await expect(lease.fit(anchor, undefined, 'block-end start')).toBe(true);
            await expect(() => createCemSurfaceGeometryLease(host, panel, () => undefined)).toThrow('already has a geometry owner');
            await expect(fitNativeSurface(host, panel, anchor, undefined, 'block-end start')).toBe(false);
            releasePopupGeometry(panel); await expect(panel.style.position).toBe('fixed');
            panel.style.setProperty('left', '41px', 'important'); panel.style.color = 'purple';
            panel.setAttribute('data-placement', 'new-authored');
            await expect(lease.fit(anchor, undefined, 'block-end start')).toBe(true);
            lease.release();
            await expect(panel.style.left).toBe('41px'); await expect(panel.style.getPropertyPriority('left')).toBe('important');
            await expect(panel.style.overflowX).toBe('hidden'); await expect(panel.style.getPropertyPriority('overflow-x')).toBe('important');
            await expect(panel.style.overflowY).toBe('scroll');
            await expect(panel.style.marginLeft).toBe('13px'); await expect(panel.style.getPropertyPriority('margin-left')).toBe('important');
            await expect(panel.style.marginTop).toBe('3px'); await expect(panel.style.color).toBe('purple');
            await expect(panel.getAttribute('data-placement')).toBe('new-authored');
            const next = createCemSurfaceGeometryLease(host, panel, () => undefined);
            try {
                const copyUrl = new URL('./surface-geometry.ts', import.meta.url); copyUrl.searchParams.set('fixture-copy', 'independent');
                const copy = await import(/* @vite-ignore */ copyUrl.href) as typeof import('./surface-geometry.js');
                await expect(() => copy.claimSurfaceGeometry(panel)).toThrow('already has a geometry owner');
                await expect(next.fit(anchor, undefined, 'block-start start')).toBe(true);
                const left = panel.style.left; lease.release();
                await expect(lease.valid).toBe(false); await expect(lease.fit(anchor, undefined, 'center')).toBe(false);
                await expect(panel.style.left).toBe(left); await expect(next.valid).toBe(true);
                panel.style.setProperty('top', '53px', 'important'); next.reset();
                await expect(panel.style.top).toBe('53px'); await expect(panel.style.getPropertyPriority('top')).toBe('important');
                await expect(next.valid).toBe(true);
            } finally { next.release(); }
        } finally { lease.release(); panel.hidePopover(); }
    },
};

export const ResetCancelsQueuedWorkAndLegacyObserversStayIndependent: Story = {
    render: fixture,
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section'), anchor = required(host, 'input'), panel = required(host, 'div');
        let updates = 0;
        const lease = createCemSurfaceGeometryLease(host, panel, () => { updates++; lease.fit(anchor, undefined, 'block-end start'); });
        panel.showPopover();
        try {
            lease.fit(anchor, undefined, 'block-end start'); await frames(); const before = updates;
            anchor.classList.add('changed'); await Promise.resolve(); lease.reset();
            await frames(); await expect(updates).toBe(before); await expect(panel.style.position).toBe('');
            await expect(lease.fit(anchor, undefined, 'block-end start')).toBe(true);
            lease.release(); let old = 0, current = 0;
            const stopOld = observePopupGeometry(host, () => old++), stopCurrent = observePopupGeometry(host, () => current++);
            const other = required(host, 'input:nth-of-type(2)'), second = required(host, 'div:nth-of-type(2)');
            try {
                fitNativeSurface(host, panel, anchor, undefined, 'block-end start'); await frames(); stopOld();
                fitNativeSurface(host, panel, other, undefined, 'block-end start'); await frames();
                const beforeOld = old, beforeCurrent = current; other.style.width = '170px';
                await waitFor(() => expect(current).toBeGreaterThan(beforeCurrent));
                await frames(); await expect(old).toBe(beforeOld);
                second.showPopover(); fitNativeSurface(host, second, anchor, undefined, 'block-end start'); await frames();
                releasePopupGeometry(panel); const afterFirst = current; anchor.style.width = '190px';
                await waitFor(() => expect(current).toBeGreaterThan(afterFirst));
            } finally { stopOld(); stopCurrent(); releasePopupGeometry(panel); releasePopupGeometry(second); second.hidePopover(); }
        } finally { lease.release(); panel.hidePopover(); }
    },
};

export const NativeRegistrationRejectsAnotherProfile: Story = {
    render: () => '<section><button>Invoker</button><dialog popover="manual" aria-label="Task" style="width:140px;height:70px">Content</dialog></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section'), dialog = required(host, 'dialog'), source = required(host, 'button');
        const controller = connectCemNativeSurface(dialog);
        try {
            await expect(connectCemNativeSurface(dialog)).toBe(controller);
            await expect(controller.open({ source })).toBe(true);
            dialog.setAttribute('kind', 'tooltip'); dialog.setAttribute('role', 'tooltip');
            await expect(() => connectCemNativeSurface(dialog)).toThrow('semantic profile');
            await expect(controller.open({ source })).toBe(false);
            await waitFor(() => expect(dialog.matches(':popover-open')).toBe(false));
        } finally { controller.disconnect(); }
    },
};
