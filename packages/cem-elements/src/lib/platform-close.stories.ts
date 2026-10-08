import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemPlatformCloseCoordinator, type CemPlatformCloseLease } from './platform-close.js';
import { captureCemSurfaceLifetime, registerCemSurfaceOwner } from './surface-session.js';
import { nativeTap } from '../../.storybook/native-input.js';

export default { title: 'CEM Elements/Platform Close Admission', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
type Watcher = EventTarget & { requestClose(): void; destroy(): void };
type Browser = { CloseWatcher?: new () => Watcher };

export const AdmissionRefusalRevocationAndReentrancy: Story = {
    render: () => '<input aria-label="Platform editor">',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { cdp } = await import('vitest/browser'), driver = cdp();
        const editor = canvasElement.querySelector('input'); if (!editor) throw new Error('Missing editor');
        editor.focus();
        const view = window as unknown as Browser, Native = view.CloseWatcher;
        if (!Native) throw new Error('Chromium platform-close fixture requires CloseWatcher');
        const watchers: Watcher[] = [], reasons: string[] = [], subscribers = new Set<() => void>();
        let admitted = true, current = true, consume = false, immediateRevocation = false;
        let lease: CemPlatformCloseLease | undefined;
        view.CloseWatcher = class extends Native { constructor() { super(); watchers.push(this); } };
        const coordinator = createCemPlatformCloseCoordinator(window, () => admitted ? { current: () => true,
            subscribe(fn) { subscribers.add(fn); if (immediateRevocation) fn(); return () => { subscribers.delete(fn); }; } } : undefined);
        const handler = (event: KeyboardEvent) => {
            if (consume) coordinator.consume(event);
            lease = coordinator.reserve({ event, editor, ancestors: [] }, () => current,
                reason => { reasons.push(reason); lease?.dispose(); });
        };
        editor.addEventListener('keydown', handler);
        const arrow = async () => {
            await driver.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'ArrowDown', code: 'ArrowDown', windowsVirtualKeyCode: 40 });
            await driver.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'ArrowDown', code: 'ArrowDown', windowsVirtualKeyCode: 40 });
        };
        try {
            editor.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
            expect(lease).toBeUndefined(); expect(watchers).toHaveLength(0);
            admitted = false; await arrow(); expect(watchers).toHaveLength(0); admitted = true;
            editor.setAttribute('list', 'native-list'); await arrow(); expect(watchers).toHaveLength(0); editor.removeAttribute('list');
            consume = true; await arrow(); expect(watchers).toHaveLength(0); consume = false;
            view.CloseWatcher = undefined; await arrow(); expect(lease).toBeUndefined();
            view.CloseWatcher = class extends Native { constructor() { super(); watchers.push(this); } };
            await arrow(); expect(lease?.current).toBe(true); expect(subscribers.size).toBe(1);
            watchers[0].dispatchEvent(new Event('close')); expect(lease?.current).toBe(true);
            watchers[0].requestClose(); expect(reasons).toEqual(['platform-close']); expect(subscribers.size).toBe(0);
            await arrow(); current = false; lease?.reconcile(); expect(reasons.at(-1)).toBe('platform-authority');
            expect(subscribers.size).toBe(0); current = true;
            immediateRevocation = true; await arrow(); expect(lease).toBeUndefined(); expect(subscribers.size).toBe(0);
            immediateRevocation = false;
            const dialog = document.createElement('dialog'); canvasElement.append(dialog); dialog.append(editor); dialog.showModal(); editor.focus();
            const before = watchers.length; await arrow(); expect(watchers.length).toBe(before);
            dialog.close(); canvasElement.append(editor); dialog.remove(); editor.focus();
            await arrow(); editor.remove();
            await waitFor(() => expect(subscribers.size).toBe(0));
            expect(reasons.at(-1)).toBe('platform-authority');
            canvasElement.append(editor); editor.focus();
            await arrow(); lease?.dispose(); watchers.at(-1)?.requestClose();
            expect(subscribers.size).toBe(0);
        } finally { lease?.dispose(); editor.removeEventListener('keydown', handler); for (const watcher of watchers) watcher.destroy(); view.CloseWatcher = Native; }
    },
};

export const NativeRequestsCloseSeparatelyActivatedChildBeforeParent: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { cdp } = await import('vitest/browser'), driver = cdp();
        for (const kind of ['modal', 'auto', 'hint']) {
            const parent = document.createElement(kind === 'modal' ? 'dialog' : 'div');
            if (kind !== 'modal') parent.setAttribute('popover', kind);
            const editor = document.createElement('input'), opener = document.createElement('button');
            editor.setAttribute('aria-label', 'Nested editor'); opener.textContent = 'Open parent';
            parent.append(editor); canvasElement.append(opener, parent);
            const unregister = registerCemSurfaceOwner(parent, parent, 'fixture-parent', () => undefined);
            opener.addEventListener('click', () => { if (kind === 'modal') (parent as HTMLDialogElement).showModal(); else parent.showPopover(); editor.focus(); });
            const coordinator = createCemPlatformCloseCoordinator(window, () => ({ current: () => true, subscribe: () => () => undefined }));
            let lease: CemPlatformCloseLease | undefined, closes = 0;
            editor.addEventListener('keydown', event => {
                if (event.key !== 'ArrowDown') return;
                const lifetime = captureCemSurfaceLifetime(parent); if (!lifetime) throw new Error('Missing parent lifetime');
                lease = coordinator.reserve({ event, editor, ancestors: [lifetime] }, () => true, () => { closes++; });
            });
            const visible = () => kind === 'modal' ? (parent as HTMLDialogElement).open : parent.matches(':popover-open');
            const key = async (key: string, code: number) => {
                await driver.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code: key, windowsVirtualKeyCode: code });
                await driver.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code: key, windowsVirtualKeyCode: code });
            };
            try {
                await nativeTap(driver, opener, 'mouse'); expect(visible()).toBe(true);
                await key('ArrowDown', 40); expect(lease?.current).toBe(true);
                // No key handler consumes Escape here: exercise actual UA watcher grouping.
                await key('Escape', 27); expect(closes).toBe(1); expect(visible()).toBe(true);
                await key('Escape', 27); expect(visible()).toBe(false); expect(closes).toBe(1);
                await nativeTap(driver, opener, 'mouse'); await key('ArrowDown', 40);
                expect(lease?.current).toBe(true);
                if (kind === 'modal') (parent as HTMLDialogElement).close(); else parent.hidePopover();
                await waitFor(() => expect(closes).toBe(2)); expect(lease?.current).toBe(false);
            } finally { lease?.dispose(); unregister(); if (kind === 'modal') (parent as HTMLDialogElement).close(); else parent.hidePopover(); parent.remove(); opener.remove(); }
        }
    },
};
