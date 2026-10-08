import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { getCemEditorProvider } from './form-control-capability.js';
import { connectCemNativeSurface } from './native-surface.js';
import { connectCemManualListbox } from './manual-listbox.js';
import { captureCemSurfaceLifetime } from './surface-session.js';

export default { title: 'CEM Elements/Manual Listbox Sessions', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
let sequence = 0;
function required<T>(value: T | null | undefined): T {
    if (value == null) throw new Error('Missing manual-listbox fixture value');
    return value;
}
async function frames(): Promise<void> {
    for (let i = 0; i < 3; i++) await new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
}
async function fixture(root: HTMLElement, parent: HTMLElement = root) {
    const suffix = ++sequence, runtime = new CemElementRuntime({ declarationTag: `listbox-declaration-${suffix}` });
    runtime.install(window);
    const declaration = document.createElement(`listbox-declaration-${suffix}`);
    declaration.setAttribute('tag', `listbox-editor-${suffix}`); declaration.setAttribute('capability', 'form-control');
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = '{input @part=control @form="" @type=text @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}';
    declaration.append(template); root.append(declaration);
    runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
    const host = document.createElement('section'), editorHost = document.createElement(`listbox-editor-${suffix}`);
    editorHost.setAttribute('value', 'original');
    const panel = document.createElement('div'); panel.setAttribute('popover', 'manual'); panel.setAttribute('role', 'listbox');
    panel.style.cssText = 'width:80px;height:70px'; panel.innerHTML = '<div role="option">Choice</div>';
    const outside = document.createElement('button'); outside.textContent = 'Outside';
    host.append(editorHost, panel, outside); parent.append(host); await runtime.whenRenderSettled(editorHost);
    const editor = required(editorHost.querySelector('input')), provider = required(getCemEditorProvider(editorHost));
    editor.setAttribute('aria-label', 'Editor'); editor.style.width = '160px'; editor.focus(); editor.setSelectionRange(2, 4);
    let admitted = true, ready = true;
    const listeners = new Set<() => void>();
    const lifecycle = { current: () => admitted, ready: () => ready,
        subscribe(listener: () => void) { listeners.add(listener); return () => { listeners.delete(listener); }; } };
    return { host, editorHost, editor, provider, panel, outside, lifecycle,
        revoke() { admitted = false; for (const listener of listeners) listener(); },
        readiness(value: boolean) { ready = value; for (const listener of listeners) listener(); } };
}

export const ExactAdmissionActualVisibilityAndStaleAttempts: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement), changes: boolean[] = [];
        const controller = connectCemManualListbox(f.panel, { ...f, onVisibility: open => changes.push(open) });
        try {
            await expect(() => connectCemManualListbox(f.panel, { ...f })).toThrow('semantic owner');
            const copyUrl = new URL('./surface-session.ts', import.meta.url); copyUrl.searchParams.set('fixture-copy', 'independent');
            const copy = await import(/* @vite-ignore */ copyUrl.href) as typeof import('./surface-session.js');
            await expect(() => copy.registerCemSurfaceOwner(f.host, f.panel, 'tooltip', () => undefined)).toThrow('semantic owner');
            await expect(() => connectCemManualListbox(document.createElement('div'), { ...f, editorHost: f.editor })).toThrow('exact native editor provider');
            f.readiness(false); f.panel.showPopover();
            await expect(f.panel.matches(':popover-open')).toBe(false);
            await expect(controller.open()).toBe(false); f.readiness(true);
            const stale = required(controller.prepare()); controller.dismiss('escape');
            await expect(controller.complete(stale)).toBe(false);
            await expect(controller.open()).toBe(true);
            await expect(changes).toEqual([true]);
            await expect(controller.complete(stale)).toBe(false); await expect(controller.visible).toBe(true);
            f.panel.dispatchEvent(new ToggleEvent('toggle', { oldState: 'open', newState: 'closed' }));
            f.panel.dispatchEvent(new ToggleEvent('beforetoggle', { oldState: 'open', newState: 'closed' }));
            await expect(controller.visible).toBe(true); await expect(changes).toEqual([true]);
            await expect(document.activeElement).toBe(f.editor);
            await expect([f.editor.selectionStart, f.editor.selectionEnd, f.editor.value]).toEqual([2, 4, 'original']);
            await expect(f.panel.getBoundingClientRect().width).toBeGreaterThanOrEqual(f.editor.getBoundingClientRect().width);
            f.panel.hidePopover(); await expect(controller.visible).toBe(false);
            await expect(changes).toEqual([true, false]);
            f.editor.style.width = '180px'; await frames(); await expect(controller.visible).toBe(false);
            // A direct show has to be admitted as a new attempt; delayed toggle events are not authority.
            f.panel.showPopover(); await expect(controller.visible).toBe(true);
            f.revoke(); await expect(controller.visible).toBe(false);
            f.panel.showPopover(); await expect(controller.visible).toBe(false);
            await expect(changes).toEqual([true, false, true, false]);
            await expect(f.editor.hasAttribute('aria-expanded')).toBe(false);
        } finally { controller.disconnect(); }
    },
};

export const EligibilityGeometryAndProfileFailures: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement), changes: boolean[] = [];
        const controller = connectCemManualListbox(f.panel, { ...f, onVisibility: open => changes.push(open) });
        try {
            f.panel.style.height = '0px'; f.panel.style.padding = '0'; f.panel.style.border = '0';
            await expect(controller.open()).toBe(false); await expect(changes).toEqual([]);
            f.panel.style.height = '70px';
            f.host.setAttribute('anchor', 'pointer'); await expect(controller.open()).toBe(false); f.host.removeAttribute('anchor');
            f.panel.innerHTML = '<button>Forbidden focus stop</button>'; await expect(controller.open()).toBe(false);
            f.panel.innerHTML = '<div role="option">Choice</div>';
            for (const type of ['search', 'email', 'tel', 'url', 'number', 'password']) {
                f.editor.type = type; await expect(controller.open()).toBe(false);
            }
            f.editor.type = 'text'; f.editor.setAttribute('list', 'native-list');
            await expect(controller.open()).toBe(false); f.editor.removeAttribute('list');
            f.editor.readOnly = true; await expect(controller.open()).toBe(false); f.editor.readOnly = false;
            f.outside.focus(); await expect(controller.open()).toBe(false); f.editor.focus();
            await expect(controller.open()).toBe(true);
            f.editor.style.position = 'fixed'; f.editor.style.left = '-10000px';
            await waitFor(() => expect(controller.visible).toBe(false));
            f.editor.style.left = '30px'; await frames(); await expect(controller.visible).toBe(false);
            await expect(controller.open()).toBe(true);
            f.host.setAttribute('boundary', '#missing-boundary'); await waitFor(() => expect(changes.at(-1)).toBe(false));
            f.host.removeAttribute('boundary'); await expect(controller.open()).toBe(true);
            f.editor.setAttribute('list', 'native-list'); await waitFor(() => expect(changes.at(-1)).toBe(false));
            f.editor.removeAttribute('list'); await expect(controller.open()).toBe(true);
            f.host.inert = true; await waitFor(() => expect(controller.visible).toBe(false));
        } finally { controller.disconnect(); }
    },
};

function pointer(target: HTMLElement, type: string, pointerId = 1, x = 0) {
    return target.dispatchEvent(new PointerEvent(type, { bubbles: true, cancelable: true, pointerId, pointerType: 'mouse', isPrimary: true, button: 0, clientX: x }));
}
export const DismissalRegionPointerSequencesAndFocusDestination: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement), reasons: string[] = [];
        const controller = connectCemManualListbox(f.panel, { ...f, onVisibility: (open, reason) => { if (!open) reasons.push(reason); } });
        try {
            await expect(controller.open()).toBe(true);
            pointer(f.editor, 'pointerdown'); pointer(f.editor, 'pointerup');
            pointer(f.panel, 'pointerdown'); pointer(f.panel, 'pointerup'); await frames(); await expect(controller.visible).toBe(true);
            pointer(f.outside, 'pointerdown'); pointer(f.outside, 'pointercancel'); pointer(f.outside, 'pointerup');
            await frames(); await expect(controller.visible).toBe(true);
            pointer(f.outside, 'pointerdown'); pointer(f.outside, 'pointermove', 1, 40); pointer(f.outside, 'pointerup', 1, 40);
            await frames(); await expect(controller.visible).toBe(true);
            pointer(f.outside, 'pointerdown'); f.host.dispatchEvent(new Event('scroll')); pointer(f.outside, 'pointerup');
            await frames(); await expect(controller.visible).toBe(true);
            const cancel = (e: Event) => e.preventDefault(); f.outside.addEventListener('pointerdown', cancel);
            pointer(f.outside, 'pointerdown'); pointer(f.outside, 'pointerup'); await frames(); await expect(controller.visible).toBe(true);
            f.outside.removeEventListener('pointerdown', cancel);
            pointer(f.outside, 'pointerdown'); controller.dismiss('revision'); await expect(controller.open()).toBe(true);
            pointer(f.outside, 'pointerup'); await frames(); await expect(controller.visible).toBe(true);
            reasons.length = 0;
            pointer(f.outside, 'pointerdown'); pointer(f.outside, 'pointerup');
            await frames(); await expect(controller.visible).toBe(false); await expect(reasons).toEqual(['outside']);
            await expect(document.activeElement).toBe(f.editor);
            await expect(controller.open()).toBe(true); f.outside.focus();
            await waitFor(() => expect(controller.visible).toBe(false)); await expect(document.activeElement).toBe(f.outside);
            f.editor.focus(); await expect(controller.open()).toBe(true);
            const tab = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true }); f.editor.dispatchEvent(tab);
            await Promise.resolve(); await expect(controller.visible).toBe(false); await expect(tab.defaultPrevented).toBe(false);
        } finally { controller.disconnect(); }
    },
};

export const NativeVetoReentrancyAndPendingEscape: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement), changes: boolean[] = [];
        const controller = connectCemManualListbox(f.panel, { ...f, onVisibility: open => changes.push(open) });
        try {
            const veto = (event: Event) => { if ((event as ToggleEvent).newState === 'open') event.preventDefault(); };
            f.panel.addEventListener('beforetoggle', veto);
            await expect(controller.open()).toBe(false); await expect(controller.pending).toBe(false);
            await expect(changes).toEqual([]); f.panel.removeEventListener('beforetoggle', veto);
            const pending = required(controller.prepare());
            const cancelled = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }); cancelled.preventDefault();
            f.editor.dispatchEvent(cancelled); await expect(controller.pending).toBe(true);
            const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }); f.editor.dispatchEvent(escape);
            await expect(escape.defaultPrevented).toBe(true); await expect(controller.complete(pending)).toBe(false);
            f.editor.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', bubbles: true }));
            const invalid = required(controller.prepare()); f.editor.readOnly = true;
            await Promise.resolve(); await expect(controller.pending).toBe(false);
            f.editor.readOnly = false; await expect(controller.complete(invalid)).toBe(false);
            const invalidate = (event: Event) => { if ((event as ToggleEvent).newState === 'open') f.readiness(false); };
            f.panel.addEventListener('beforetoggle', invalidate); await expect(controller.open()).toBe(false);
            await expect(changes).toEqual([]); f.panel.removeEventListener('beforetoggle', invalidate); f.readiness(true);
            await expect(controller.open()).toBe(true); f.panel.hidePopover(); f.panel.showPopover();
            await expect(controller.visible).toBe(true); await expect(changes).toEqual([true, false, true]);
            controller.disconnect(); const generation = controller.generation;
            f.editor.style.width = '200px'; await frames();
            await expect(controller.open()).toBe(false); await expect(controller.generation).toBe(generation);
        } finally { controller.disconnect(); }
    },
};

export const NativeAncestorLifetimeSurvivesAttributeWritesButNotCloseReopen: Story = {
    render: () => '<section><dialog aria-label="Parent task" style="width:400px;height:260px"></dialog></section>',
    play: async ({ canvasElement }) => {
        const dialog = required(canvasElement.querySelector('dialog')), parent = connectCemNativeSurface(dialog);
        parent.open(); const f = await fixture(canvasElement, dialog), changes: boolean[] = [];
        const controller = connectCemManualListbox(f.panel, { ...f, ancestors: [required(captureCemSurfaceLifetime(dialog))], onVisibility: open => changes.push(open) });
        try {
            await expect(controller.open()).toBe(true); dialog.setAttribute('open', 'still-open');
            await frames(); await expect(controller.visible).toBe(true);
            dialog.close(); dialog.show(); f.editor.focus();
            await waitFor(() => expect(controller.visible).toBe(false)); await expect(changes).toEqual([true, false]);
            await expect(controller.open()).toBe(false);
        } finally { controller.disconnect(); parent.disconnect(); }
    },
};

export const NestedEscapeLifetimeAndModalContainment: Story = {
    render: () => '<section><dialog aria-label="Parent task" style="width:400px;height:260px"></dialog></section>',
    play: async ({ canvasElement }) => {
        const dialog = required(canvasElement.querySelector('dialog')), parent = connectCemNativeSurface(dialog);
        await expect(parent.open()).toBe(true);
        const f = await fixture(canvasElement, dialog), lifetime = required(captureCemSurfaceLifetime(dialog));
        const controller = connectCemManualListbox(f.panel, { ...f, ancestors: [lifetime] });
        try {
            await expect(controller.open()).toBe(true);
            const ime = new KeyboardEvent('keydown', { key: 'Escape', isComposing: true, bubbles: true, cancelable: true }); f.editor.dispatchEvent(ime);
            await expect(controller.visible).toBe(true); await expect(ime.defaultPrevented).toBe(false);
            f.editor.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', bubbles: true }));
            const escape = new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', bubbles: true, cancelable: true }); f.editor.dispatchEvent(escape);
            await expect(controller.visible).toBe(false); await expect(escape.defaultPrevented).toBe(true); await expect(dialog.open).toBe(true);
            const repeat = new KeyboardEvent('keydown', { key: 'Escape', code: 'Escape', repeat: true, bubbles: true, cancelable: true }); f.editor.dispatchEvent(repeat);
            await expect(repeat.defaultPrevented).toBe(true); await expect(dialog.open).toBe(true);
            f.editor.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', code: 'Escape', bubbles: true }));
            await expect(controller.open()).toBe(true); dialog.close();
            await waitFor(() => expect(controller.visible).toBe(false));
            dialog.show(); f.editor.focus(); await expect(controller.open()).toBe(false); // an old ancestor lifetime cannot be revived
        } finally { controller.disconnect(); parent.disconnect(); }
        dialog.setAttribute('mode', 'modal'); const modal = connectCemNativeSurface(dialog); modal.open();
        const other = await fixture(canvasElement); other.editorHost.remove(); dialog.append(other.editorHost); other.editor.focus();
        const separate = connectCemManualListbox(other.panel, { ...other, ancestors: [required(captureCemSurfaceLifetime(dialog))] });
        try {
            await expect(separate.open()).toBe(false);
            const inside = await fixture(canvasElement, dialog), attached = connectCemManualListbox(inside.panel, {
                ...inside, ancestors: [required(captureCemSurfaceLifetime(dialog))],
            });
            try {
                await expect(attached.open()).toBe(true);
                if (import.meta.env.MODE === 'test') {
                    const { userEvent: nativeUser } = await import('vitest/browser');
                    await nativeUser.keyboard('{Escape}');
                    await expect(attached.visible).toBe(false); await expect(dialog.matches(':modal')).toBe(true);
                    await expect(document.activeElement).toBe(inside.editor); await expect(inside.editor.value).toBe('original');
                }
            } finally { attached.disconnect(); }
        } finally { separate.disconnect(); modal.disconnect(); }
    },
};

export const IndependentOwnersAndProviderRevocation: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const a = await fixture(canvasElement), b = await fixture(canvasElement);
        const one = connectCemManualListbox(a.panel, { ...a }), two = connectCemManualListbox(b.panel, { ...b });
        try {
            a.editor.focus(); await expect(one.open()).toBe(true); b.editor.focus();
            await expect(two.open()).toBe(true); await expect(one.visible).toBe(false);
            one.disconnect(); await expect(two.visible).toBe(true);
            const competing = b.provider.lease({}); await expect(two.visible).toBe(false); competing.release();
            await frames(); await expect(two.visible).toBe(false); await expect(two.open()).toBe(true);
            b.editorHost.remove(); await waitFor(() => expect(two.visible).toBe(false));
        } finally { one.disconnect(); two.disconnect(); }
    },
};
