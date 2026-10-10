import { expect, userEvent, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-dialog-shell.xhtml?raw';
function required<T extends HTMLElement = HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
const meta = preview.meta({ component: 'cem-dialog-shell', title: 'CEM Components/cem-dialog-shell', loaders: [async () => { await loadCemDeclaration('cem-dialog-shell', declarationSource); return {}; }] });
export const NativeAlias = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dialog-shell label="Shell task" trigger="Open shell" close-label="Done"><input autofocus aria-label="Value"></cem-dialog-shell></section>',
    play: async ({ canvasElement }) => {
        const host = required<HTMLElement>(canvasElement, 'cem-dialog-shell'); await whenCemRendered(host);
        const owner = required<HTMLDialogElement>(host, 'dialog'), trigger = required<HTMLButtonElement>(host, '[part=trigger]');
        expect(owner.querySelector('[part=heading]')?.textContent).toBe('Shell task'); expect(owner.open).toBe(false); expect(owner.hasAttribute('aria-modal')).toBe(false);
        await userEvent.click(trigger); await waitFor(() => expect(owner.open).toBe(true)); expect(owner.matches(':modal')).toBe(false); expect(owner).toHaveAccessibleName('Shell task');
        await userEvent.click(required<HTMLElement>(owner, '[part=close]')); expect(owner.open).toBe(false); expect(document.activeElement).toBe(trigger);
        host.setAttribute('mode', 'modal'); await whenCemRendered(host); await userEvent.click(trigger);
        await waitFor(() => expect(owner.matches(':modal')).toBe(true)); expect(host.querySelector('dialog')).toBe(owner);
        host.remove(); expect(owner.open).toBe(false); expect(host.querySelector('[part=trigger]')).toBeNull();
    },
});

export const KeyboardFallbackFocus = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dialog-shell label="Keyboard task" trigger="Open task" mode="modal"><p>Read this task.</p></cem-dialog-shell></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const host = required<HTMLElement>(canvasElement, 'cem-dialog-shell'); await whenCemRendered(host);
        const owner = required<HTMLDialogElement>(host, 'dialog'), source = required<HTMLButtonElement>(host, '[part=trigger]');
        source.focus(); await native.keyboard('{Enter}'); await waitFor(() => expect(owner.open).toBe(true));
        expect(document.activeElement).toBe(owner); expect(owner.matches(':focus-visible')).toBe(true);
        const style = getComputedStyle(owner);
        expect(style.outlineStyle).toBe('solid'); expect(parseFloat(style.outlineWidth)).toBeGreaterThan(0);
        expect(host.hasAttribute('tabindex')).toBe(false); expect(owner.hasAttribute('tabindex')).toBe(false);
        await native.keyboard('{Escape}'); await waitFor(() => expect(owner.open).toBe(false)); expect(document.activeElement).toBe(source);
    },
    parameters: { docs: { description: { story: 'Browser-runner-only: trusted keyboard entry, native fallback focus, token outline and Escape restoration.' } } },
});
