import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { connectCemNativeInvokerFocus } from './native-invoker-focus.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { registerCemSurfaceOwner } from './surface-session.js';

export default { title: 'CEM Elements/Native Invoker Focus', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends Element>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
function fixture(root: HTMLElement, commands: boolean) {
    const box = root.ownerDocument.createElement('section'), id = `native-${crypto.randomUUID()}`;
    const invoke = (target: string, action = 'toggle') => commands ? `commandfor="${target}" command="${action}-popover"` : `popovertarget="${target}" popovertargetaction="${action}"`;
    box.innerHTML = `<button type="button" ${invoke(id)}>Open parent</button>
      <aside id="${id}" popover="auto" style="inset:70px auto auto 10px;margin:0">
        <button type="button">Parent action</button>
        <button type="button" ${invoke(id + '-child')}>Open child</button>
        <button type="button" ${invoke(id + '-child', 'show')}>Other child opener</button>
        <article id="${id}-child" popover="auto" style="inset:140px auto auto 20px;margin:0">
          <input aria-label="Popup entry" autofocus>
          <button type="button" ${invoke(id + '-child', 'hide')}>Close child</button>
          <button type="button" ${invoke(id, 'hide')}>Close stack</button>
        </article>
      </aside><button type="button">Outside destination</button>`;
    root.append(box);
    const parent = required<HTMLElement>(box, 'aside'), child = required<HTMLElement>(box, 'article');
    const trigger = required<HTMLButtonElement>(box, ':scope > button');
    const launchers = [...parent.querySelectorAll<HTMLButtonElement>(':scope > button')];
    return { box, parent, child, trigger, sibling: launchers[0], launcher: launchers[1], other: launchers[2],
        entry: required<HTMLInputElement>(child, 'input'), close: required<HTMLButtonElement>(child, 'button'),
        outside: required<HTMLButtonElement>(box, ':scope > button:last-child') };
}
const frame = () => new Promise<void>(resolve => requestAnimationFrame(() => resolve()));
async function nativeContract(root: HTMLElement, commands: boolean) {
    if (import.meta.env.MODE !== 'test') return;
    const { userEvent } = await import('vitest/browser');
    const f = fixture(root, commands), release = connectCemNativeInvokerFocus(document);
    try {
        let visibilityCalls = 0;
        for (const owner of [f.parent, f.child]) for (const method of ['showPopover', 'hidePopover', 'togglePopover'] as const) {
            Object.defineProperty(owner, method, { configurable: true, value: () => { visibilityCalls++; throw new Error('Native invokers must own visibility'); } });
        }
        await userEvent.click(f.trigger); await userEvent.click(f.launcher);
        expect(f.parent.matches(':popover-open')).toBe(true); expect(f.child.matches(':popover-open')).toBe(true);
        expect(document.activeElement).toBe(f.entry);
        await userEvent.keyboard('{Escape}');
        await waitFor(() => expect(document.activeElement).toBe(f.launcher));
        expect(f.child.matches(':popover-open')).toBe(false); expect(f.parent.matches(':popover-open')).toBe(true);
        let nativeReturnCalls = 0;
        const focus = f.trigger.focus;
        f.trigger.focus = function(options) { nativeReturnCalls++; focus.call(this, options); };
        await userEvent.keyboard('{Escape}'); await frame();
        expect(document.activeElement).toBe(f.trigger); expect(nativeReturnCalls).toBe(0);
        delete (f.trigger as Partial<HTMLButtonElement>).focus;
        await userEvent.click(f.trigger); await userEvent.click(f.other);
        await userEvent.click(f.close);
        await waitFor(() => expect(document.activeElement).toBe(f.other));
        await userEvent.click(f.launcher); await userEvent.click(f.sibling);
        expect(f.child.matches(':popover-open')).toBe(false); expect(f.parent.matches(':popover-open')).toBe(true);
        expect(document.activeElement).toBe(f.sibling);
        await userEvent.click(f.launcher); await userEvent.click(f.outside); await frame();
        expect(f.parent.matches(':popover-open')).toBe(false); expect(f.child.matches(':popover-open')).toBe(false);
        expect(document.activeElement).toBe(f.outside);
        await userEvent.click(f.trigger); await userEvent.click(f.launcher);
        for (let step = 0; step < 8 && document.activeElement !== f.outside; step++) await userEvent.keyboard('{Tab}');
        expect(document.activeElement).toBe(f.outside);
        const destination = document.activeElement;
        HTMLElement.prototype.hidePopover.call(f.parent); await frame(); await frame();
        expect(document.activeElement).toBe(destination);
        await userEvent.click(f.trigger); await userEvent.click(f.launcher);
        await userEvent.click(required<HTMLButtonElement>(f.child, 'button:last-child'));
        await waitFor(() => expect(document.activeElement).toBe(f.trigger));
        expect(f.parent.matches(':popover-open')).toBe(false); expect(f.child.matches(':popover-open')).toBe(false);
        expect(visibilityCalls).toBe(0);
    } finally { release(); f.box.remove(); }
}
export const PopoverTargets: Story = {
    name: 'Native popover targets and focus return (trusted browser input)',
    render: () => '<section></section>', play: ({ canvasElement }) => nativeContract(required(canvasElement, 'section'), false),
};
export const InvokerCommands: Story = {
    name: 'Native popover commands and focus return (trusted browser input)',
    render: () => '<section></section>', play: ({ canvasElement }) => nativeContract(required(canvasElement, 'section'), true),
};
export const BindingLifetime: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const root = required<HTMLElement>(canvasElement, 'section'), f = fixture(root, false);
        const release = connectCemNativeInvokerFocus(document);
        try {
            f.child.addEventListener('beforetoggle', event => { if ((event as ToggleEvent).newState === 'open') event.preventDefault(); }, { once: true });
            await userEvent.click(f.trigger); await userEvent.click(f.launcher); await frame();
            expect(f.child.matches(':popover-open')).toBe(false); expect(document.activeElement).toBe(f.launcher);
            await userEvent.click(f.launcher);
            HTMLElement.prototype.hidePopover.call(f.child);
            f.launcher.click(); await frame(); await frame();
            expect(f.child.matches(':popover-open')).toBe(true); expect(document.activeElement).toBe(f.entry);
            f.launcher.setAttribute('popovertarget', f.child.id); // Same-value writes keep the lifetime.
            await frame(); await userEvent.keyboard('{Escape}');
            await waitFor(() => expect(document.activeElement).toBe(f.launcher));
            await userEvent.click(f.launcher);
            f.launcher.setAttribute('popovertarget', 'unresolved');
            f.launcher.setAttribute('popovertarget', f.child.id);
            let restores = 0; const focus = f.launcher.focus;
            f.launcher.focus = function(options) { restores++; focus.call(this, options); };
            await frame(); await userEvent.keyboard('{Escape}'); await frame();
            expect(restores).toBe(0);
            delete (f.launcher as Partial<HTMLButtonElement>).focus;
            await userEvent.click(f.launcher); f.launcher.disabled = true;
            await userEvent.keyboard('{Escape}'); await frame(); expect(document.activeElement).not.toBe(f.launcher);
            f.launcher.disabled = false;
            await userEvent.click(f.launcher);
            const previous = f.child, replacement = previous.cloneNode(true) as HTMLElement;
            previous.replaceWith(replacement); await frame();
            expect(previous.matches(':popover-open')).toBe(false);
            f.child = replacement; f.entry = required(replacement, 'input'); f.close = required(replacement, 'button');
            await userEvent.click(f.launcher); await userEvent.click(f.close);
            await waitFor(() => expect(document.activeElement).toBe(f.launcher));
            let unownedReturns = 0; const actualFocus = f.launcher.focus;
            f.launcher.focus = function(options) { unownedReturns++; actualFocus.call(this, options); };
            replacement.showPopover(); f.close.click(); await frame(); await frame();
            expect(unownedReturns).toBe(0); // No actual native invoker: do not invent one.
            delete (f.launcher as Partial<HTMLButtonElement>).focus;
            await userEvent.click(f.launcher); f.launcher.remove();
            await userEvent.keyboard('{Escape}'); await frame(); expect(document.activeElement).not.toBe(f.launcher);
        } finally { release(); f.box.remove(); }
    },
};
export const SharedOwnershipAndDisposal: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent } = await import('vitest/browser');
        const host = document.createElement('div'); canvasElement.append(host);
        const shadow = host.attachShadow({ mode: 'open' });
        const root = document.createElement('section'); shadow.append(root);
        const f = fixture(root, false), first = connectCemNativeInvokerFocus(shadow), second = connectCemNativeInvokerFocus(shadow);
        let semanticRelease: () => void = () => undefined;
        try {
            first(); await userEvent.click(f.trigger); await userEvent.click(f.launcher);
            await userEvent.keyboard('{Escape}');
            await waitFor(() => expect(shadow.activeElement).toBe(f.launcher));
            semanticRelease = registerCemSurfaceOwner(f.box, f.child, 'fixture', () => undefined);
            let restores = 0; const focus = f.launcher.focus;
            f.launcher.focus = function(options) { restores++; focus.call(this, options); };
            await userEvent.click(f.launcher); await userEvent.keyboard('{Escape}'); await frame();
            expect(restores).toBe(0); semanticRelease();
            delete (f.launcher as Partial<HTMLButtonElement>).focus;
            second(); await userEvent.click(f.launcher);
            f.launcher.focus = function(options) { restores++; focus.call(this, options); };
            await userEvent.keyboard('{Escape}'); await frame(); expect(restores).toBe(0);
        } finally { first(); second(); semanticRelease(); host.remove(); }
    },
};

export const RuntimeInstallationAndScopeDisposal: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const iframe = document.createElement('iframe'); iframe.title = 'Independent native invoker document'; canvasElement.append(iframe);
        const doc = iframe.contentDocument;
        if (!doc?.defaultView) throw new Error('Missing independent document');
        const parent = createCemDeclarationScope({ document: doc }), scope = createCemDeclarationScope({ document: doc, parent });
        const runtime = new CemElementRuntime({ declarationTag: `declaration-${crypto.randomUUID()}`, declarationScope: scope });
        const f = fixture(doc.body, false);
        const prototype = (doc.defaultView as Window & typeof globalThis).ToggleEvent.prototype;
        const sourceDescriptor = Object.getOwnPropertyDescriptor(prototype, 'source');
        if (sourceDescriptor?.configurable) Reflect.deleteProperty(prototype, 'source');
        try {
            runtime.install(doc.defaultView as Window & typeof globalThis); runtime.install(doc.defaultView as Window & typeof globalThis);
            f.trigger.focus(); f.trigger.click(); f.launcher.focus(); f.launcher.click();
            expect(doc.activeElement).toBe(f.entry);
            f.close.click(); await waitFor(() => expect(doc.activeElement).toBe(f.launcher));
            parent.dispose();
            f.launcher.click();
            let restores = 0; const focus = f.launcher.focus;
            f.launcher.focus = function(options) { restores++; focus.call(this, options); };
            f.close.click(); await frame(); await frame(); expect(restores).toBe(0);
        } finally { if (sourceDescriptor) Object.defineProperty(prototype, 'source', sourceDescriptor); scope.dispose(); parent.dispose(); iframe.remove(); }
    },
};
