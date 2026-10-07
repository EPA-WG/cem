import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { connectCemNativeSurface } from './native-surface.js';
import { captureCemSurfaceInvocation } from './surface-invocation.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Native Surface Adapters', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const ModalFocusContextCancellationAndDisconnect: Story = {
    render: () => '<section><button>Invoker</button><button>Return</button><dialog aria-label="Task"><h2 tabindex="-1">Heading</h2><button autofocus>Entry</button></dialog></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), invoker = required<HTMLButtonElement>(root, 'button'), returner = required<HTMLButtonElement>(root, 'button:nth-child(2)'), dialog = required<HTMLDialogElement>(root, 'dialog'), heading = required(root, 'h2');
        dialog.setAttribute('mode', 'modal'); dialog.setAttribute('focus-target', 'none');
        const controller = connectCemNativeSurface(dialog);
        const opens: Event[] = [], closes: Event[] = []; dialog.addEventListener('cem-open', e => opens.push(e)); dialog.addEventListener('cem-close', e => closes.push(e));
        invoker.focus();
        try {
            await expect(controller.open({ source: invoker, contextKey: 'one' })).toBe(true);
            await expect(dialog.matches(':modal')).toBe(true);
            await expect(document.activeElement).toBe(required(dialog, 'button'));
            await expect(controller.open({ source: returner, contextKey: 'two' })).toBe(false);
            await expect(opens.length).toBe(1);
            dialog.setAttribute('context-change', 'request');
            const rejectContext = (e: Event) => e.preventDefault(); dialog.addEventListener('cem-context-change', rejectContext);
            await expect(controller.open({ source: returner, contextKey: 'two' })).toBe(false);
            dialog.removeEventListener('cem-context-change', rejectContext);
            await expect(controller.open({ source: returner, contextKey: 'two' })).toBe(true);
            dialog.setAttribute('context-change', 'replace'); await expect(controller.open({ source: invoker, contextKey: 'one' })).toBe(true);
            await expect(opens.length).toBe(1);
            const cancel = (event: Event) => event.preventDefault(); dialog.addEventListener('cem-before-close', cancel);
            await expect(controller.requestClose('escape')).toBe(false); await expect(dialog.open).toBe(true);
            dialog.removeEventListener('cem-before-close', cancel); heading.id = `entry-${crypto.randomUUID()}`; returner.id = `return-${crypto.randomUUID()}`;
            dialog.setAttribute('focus-target', heading.id); dialog.setAttribute('data-cem-node-ref-focus-target', '');
            dialog.setAttribute('return-focus', returner.id); dialog.setAttribute('data-cem-node-ref-return-focus', '');
            await expect(controller.requestClose('escape')).toBe(true); await expect(document.activeElement).toBe(returner);
            await expect(controller.open({ source: invoker, contextKey: 'one' })).toBe(true); await expect(document.activeElement).toBe(heading);
            heading.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }));
            await Promise.resolve(); // Tab did not close this modal; a later native close has its own reason.
            dialog.addEventListener('cem-before-close', cancel); dialog.close('forced');
            await waitFor(() => expect(dialog.getAttribute('data-state')).toBe('closed')); await expect(closes.length).toBe(2);
            await expect(document.activeElement).toBe(returner);
            dialog.removeEventListener('cem-before-close', cancel); controller.open({ source: invoker }); controller.disconnect();
            await expect(dialog.open).toBe(false); await expect(dialog.style.left).toBe('');
        } finally { controller.disconnect(); }
    },
};
export const CapturedGeometryAndPopoverDialogVisibility: Story = {
    render: () => '<section><button>Invoker</button><dialog popover="manual" aria-label="Floating task" anchor="pointer" style="width:140px;height:70px"><button>Inside</button></dialog><div style="position:fixed;left:20px;top:20px;width:320px;height:240px;pointer-events:none">Bounds</div></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), invoker = required<HTMLButtonElement>(root, 'button'), dialog = required<HTMLDialogElement>(root, 'dialog'), bounds = required(root, ':scope > div');
        bounds.id = `fit-${crypto.randomUUID()}`; dialog.setAttribute('boundary', bounds.id); dialog.setAttribute('data-cem-node-ref-boundary', ''); dialog.setAttribute('placement', 'block-end start');
        const errors: string[] = []; dialog.addEventListener('cem-interaction-error', e => errors.push((e as CustomEvent).detail.code));
        const controller = connectCemNativeSurface(dialog);
        try {
            invoker.focus(); await expect(controller.open({ source: invoker })).toBe(false); await expect(dialog.matches(':popover-open')).toBe(false);
            const invocation = captureCemSurfaceInvocation(invoker, new MouseEvent('click', { detail: 1, clientX: 100, clientY: 90 }));
            await expect(controller.open(invocation)).toBe(true); await expect(dialog.open).toBe(false); await expect(dialog.matches(':popover-open')).toBe(true);
            await expect(Math.abs(dialog.getBoundingClientRect().left - 100)).toBeLessThan(1);
            await expect(dialog.getBoundingClientRect().right).toBeLessThanOrEqual(bounds.getBoundingClientRect().right - 3);
            dialog.setAttribute('overflow', 'shift');
            await waitFor(() => expect(dialog.style.maxWidth).toBe(''));
            await expect(dialog.style.overflow).toBe('');
            await expect(controller.requestClose('escape')).toBe(true); await expect(document.activeElement).toBe(invoker);
            invoker.setAttribute('commandfor', dialog.id = `floating-${crypto.randomUUID()}`); invoker.setAttribute('command', 'show-popover');
            await userEvent.click(invoker); await waitFor(() => expect(dialog.matches(':popover-open')).toBe(true));
            controller.requestClose('escape');
            await expect(dialog.dispatchEvent(new CommandEvent('command', { source: invoker, command: 'show-popover', cancelable: true }))).toBe(false);
            await expect(dialog.matches(':popover-open')).toBe(false); // no previous pointer context may be borrowed
            dialog.setAttribute('anchor', 'selection');
            await expect(controller.open({ source: invoker })).toBe(false);
            const text = bounds.firstChild; if (!text) throw new Error('Missing selection text');
            const range = document.createRange(); range.selectNodeContents(bounds); document.getSelection()?.removeAllRanges(); document.getSelection()?.addRange(range);
            const selected = captureCemSurfaceInvocation(invoker, new KeyboardEvent('keydown', { key: 'ContextMenu' }), { selectionOwner: bounds });
            await expect(controller.open(selected)).toBe(true); document.getSelection()?.removeAllRanges();
            await expect(dialog.matches(':popover-open')).toBe(true); controller.requestClose('outside');
            await expect(errors).toContain('interaction-anchor-unavailable');
        } finally { controller.disconnect(); document.getSelection()?.removeAllRanges(); }
    },
};
export const NativeCommandsAndNonmodalPersistence: Story = {
    render: () => '<section><button type="button" command="--cem-show">Show</button><button type="button" command="request-close">Close request</button><button>Outside</button><dialog aria-label="Persistent task"><button>Inside</button></dialog></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), dialog = required<HTMLDialogElement>(root, 'dialog'); dialog.id = `task-${crypto.randomUUID()}`;
        const invoker = required<HTMLButtonElement>(root, 'button'), close = required<HTMLButtonElement>(root, 'button:nth-child(2)'), outside = required(root, 'button:nth-child(3)');
        invoker.setAttribute('commandfor', dialog.id); close.setAttribute('commandfor', dialog.id);
        const controller = connectCemNativeSurface(dialog); let opens = 0; dialog.addEventListener('cem-open', () => opens++);
        try {
            await userEvent.click(invoker); await expect(dialog.open).toBe(true); await expect(dialog.matches(':modal')).toBe(false);
            await userEvent.click(outside); await expect(dialog.open).toBe(true);
            await userEvent.click(invoker); await expect(opens).toBe(1);
            await userEvent.click(close); await waitFor(() => expect(dialog.open).toBe(false));
            await expect(document.activeElement).toBe(close); // a close invoker never replaces the session launcher or outside focus
        } finally { controller.disconnect(); }
    },
};
export const TooltipDescriptionInterestAndNoninteractiveValidation: Story = {
    render: () => '<section><button aria-describedby="existing">Interest</button><span id="existing">Existing help</span><div role="tooltip" popover="hint" show-delay="0" hide-delay="0" style="width:100px;height:35px">Stable description</div></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), source = required<HTMLButtonElement>(root, 'button'), tooltip = required(root, '[role=tooltip]'); tooltip.id = `tip-${crypto.randomUUID()}`;
        source.setAttribute('interestfor', tooltip.id); const controller = connectCemNativeSurface(tooltip);
        try {
            source.focus(); tooltip.dispatchEvent(new (globalThis as unknown as { InterestEvent: new(type: string, init: { source: Element; cancelable: boolean }) => Event }).InterestEvent('interest', { source, cancelable: true }));
            await waitFor(() => expect(tooltip.matches(':popover-open')).toBe(true));
            await expect(document.activeElement).toBe(source); await expect(source.getAttribute('aria-describedby')).toBe(`existing ${tooltip.id}`);
            tooltip.dispatchEvent(new Event('loseinterest', { cancelable: true })); await waitFor(() => expect(tooltip.matches(':popover-open')).toBe(false));
            await expect(source.getAttribute('aria-describedby')).toBe(`existing ${tooltip.id}`);
            source.setAttribute('aria-describedby', `existing ${tooltip.id} additional`);
            const invalid = document.createElement('button'); invalid.textContent = 'Wrong'; tooltip.append(invalid);
            await expect(controller.open({ source })).toBe(false); invalid.remove();
            tooltip.setAttribute('role', 'dialog');
            await waitFor(() => expect(source.getAttribute('aria-describedby')).toBe('existing additional'));
        } finally { controller.disconnect(); await expect(source.getAttribute('aria-describedby')).toBe('existing additional'); }
    },
};
export const DeclarativeTypedAdapterWorkerAndFallback: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID();
            const tag = `cem-native-surface-${suffix}`, card = `cem-surface-card-${suffix}`, declarationTag = `declaration-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) }); runtime.install(window);
            const declare = (name: string, source: string, capability?: string) => {
                const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', name); if (capability) declaration.setAttribute('capability', capability);
                const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = source;
                declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); return declaration;
            };
            const declarations = [declare(tag, '{dialog @part=surface @aria-label=Task | {slot}}', 'native-surface'),
                declare(card, `{cem:variable @name=entry @select='data:read("<button type=\\"button\\">Entry</button>", "xml").root.children'}{cem:variable @name=returner @select='data:read("<button type=\\"button\\">Return</button>", "xml").root.children'}{${tag} @mode=modal @focus-target={#entry} @return-focus={#returner} | {$entry}}{$returner}`)];
            const instance = document.createElement(card); root.append(instance);
            try {
                await runtime.whenRenderSettled(instance); const host = required(instance, tag); await runtime.whenRenderSettled(host);
                const owner = required<HTMLDialogElement>(host, 'dialog'), entry = required(host, 'button'), returner = required(instance, ':scope > button');
                owner.showModal(); await waitFor(() => expect(document.activeElement).toBe(entry));
                instance.setAttribute('tone', 'changed'); await runtime.whenRenderSettled(instance); await runtime.whenRenderSettled(host);
                await expect(required(host, 'dialog')).toBe(owner); await expect(owner.matches(':modal')).toBe(true);
                await expect(document.activeElement).toBe(entry);
                owner.requestClose(); await waitFor(() => expect(document.activeElement).toBe(returner));
                await expect(host.getAttribute('data-state')).toBe('closed');
                owner.showModal(); instance.remove(); await waitFor(() => expect(owner.open).toBe(false));
            } finally { instance.remove(); declarations.forEach(node => node.remove()); scope.dispose(); }
        }
    },
};
