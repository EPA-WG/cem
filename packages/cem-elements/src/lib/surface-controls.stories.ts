import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { connectCemNativeSurface } from './native-surface.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Surface Control Conveniences', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector); if (!element) throw new Error(`Missing ${selector}`); return element;
}
export const GeneratedSlottedExternalAndCleanup: Story = {
    render: () => '<section><button type="button" aria-checked="true">External</button><div trigger="Edit" close-label="Done"><dialog part="surface" aria-label="Editor"><input autofocus></dialog></div></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section > div'), owner = required<HTMLDialogElement>(host, 'dialog'), external = required<HTMLButtonElement>(canvasElement, 'section > button');
        const controller = connectCemNativeSurface(owner, { host }); let opened = 0; host.addEventListener('cem-open', () => opened++);
        try {
            const trigger = required<HTMLButtonElement>(host, '[part=trigger]'), close = required<HTMLButtonElement>(owner, '[part=close]');
            expect(trigger.type).toBe('button'); expect(close.type).toBe('button'); expect(trigger.getAttribute('aria-haspopup')).toBe('dialog');
            await userEvent.click(trigger); expect(owner.open).toBe(true); expect(opened).toBe(1);
            await waitFor(() => expect(trigger.getAttribute('aria-expanded')).toBe('true'));
            const veto = (event: Event) => event.preventDefault(); host.addEventListener('cem-before-close', veto);
            await userEvent.click(close); expect(owner.open).toBe(true); host.removeEventListener('cem-before-close', veto);
            await userEvent.click(close); expect(owner.open).toBe(false); expect(document.activeElement).toBe(trigger);
            const slotted = document.createElement('button'); slotted.type = 'button'; slotted.slot = 'trigger'; slotted.textContent = 'Custom'; host.append(slotted);
            await waitFor(() => expect(trigger.isConnected).toBe(false)); expect(slotted.getAttribute('commandfor')).toBe(owner.id);
            await userEvent.click(slotted); expect(opened).toBe(2); controller.requestClose();
            slotted.remove(); host.removeAttribute('trigger');
            external.id = `external-${crypto.randomUUID()}`; host.setAttribute('trigger-for', external.id); host.setAttribute('data-cem-node-ref-trigger-for', '');
            await waitFor(() => expect(external.getAttribute('commandfor')).toBe(owner.id));
            await userEvent.click(external); expect(opened).toBe(3); expect(external.getAttribute('aria-checked')).toBe('true');
            controller.requestClose(); expect(document.activeElement).toBe(external);
            controller.disconnect(); expect(external.hasAttribute('commandfor')).toBe(false); expect(external.hasAttribute('aria-expanded')).toBe(false); expect(external.getAttribute('aria-checked')).toBe('true');
            expect(owner.querySelector('[part=close]')).toBeNull();
        } finally { controller.disconnect(); }
    },
};
export const NativeAdoptionAndInvalidControls: Story = {
    render: () => '<section><form><div trigger="Conflict" close-label="Close"><button slot="trigger">Submit</button><dialog part="surface" aria-label="Editor"><input autofocus></dialog></div></form><button type="button">External</button></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'form > div'), owner = required<HTMLDialogElement>(host, 'dialog'), source = required<HTMLButtonElement>(host, '[slot=trigger]');
        const errors: string[] = []; host.addEventListener('cem-interaction-error', event => errors.push((event as CustomEvent).detail.code));
        let submits = 0; required<HTMLFormElement>(canvasElement, 'form').addEventListener('submit', event => { event.preventDefault(); submits++; });
        const controller = connectCemNativeSurface(owner, { host });
        try {
            expect(source.hasAttribute('commandfor')).toBe(false); await userEvent.click(source); expect(submits).toBe(1); expect(owner.open).toBe(false);
            source.type = 'button'; owner.id ||= `adopt-${crypto.randomUUID()}`; source.setAttribute('commandfor', owner.id); source.setAttribute('command', '--cem-show');
            await waitFor(() => expect(source.getAttribute('aria-controls')).toBe(owner.id));
            let opens = 0; host.addEventListener('cem-open', () => opens++); await userEvent.click(source); expect(opens).toBe(1); controller.requestClose();
            const close = document.createElement('button'); close.type = 'button'; close.slot = 'close'; close.textContent = 'Authored close';
            close.setAttribute('commandfor', owner.id); close.setAttribute('command', 'request-close'); owner.append(close);
            await waitFor(() => expect(owner.querySelector('[part=close]')).toBeNull());
            await userEvent.click(source); await userEvent.click(close); expect(owner.open).toBe(false); expect(document.activeElement).toBe(source);
            const external = required<HTMLButtonElement>(canvasElement, 'section > button'); external.id = `conflict-${crypto.randomUUID()}`;
            host.setAttribute('trigger-for', external.id); host.setAttribute('data-cem-node-ref-trigger-for', '');
            await waitFor(() => expect(errors).toContain('interaction-trigger-ambiguous')); expect(external.hasAttribute('commandfor')).toBe(false);
            host.removeAttribute('trigger-for'); source.setAttribute('popovertarget', owner.id);
            await waitFor(() => expect(errors).toContain('interaction-dual-route'));
            expect(source.getAttribute('commandfor')).toBe(owner.id); // authored invalid routes are diagnosed, not rewritten
            await userEvent.click(source); expect(owner.open).toBe(false);
            controller.disconnect(); expect(source.getAttribute('command')).toBe('--cem-show'); expect(close.getAttribute('command')).toBe('request-close');
        } finally { controller.disconnect(); }
    },
};
export const ProviderNamingAndPopoverClose: Story = {
    render: () => '<section><div><div slot="trigger"><button type="button" part="control">Open task</button></div><dialog part="surface" popover="manual" aria-label="Popover task"><input autofocus><button type="button" slot="close">Done</button></dialog></div></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section > div'), owner = required<HTMLDialogElement>(host, 'dialog');
        const source = required<HTMLButtonElement>(host, '[part=control]'), close = required<HTMLButtonElement>(owner, '[slot=close]');
        owner.id = `popover-controls-${crypto.randomUUID()}`;
        for (const [control, action] of [[source, 'show'], [close, 'hide']] as const) {
            control.setAttribute('popovertarget', owner.id); control.setAttribute('popovertargetaction', action);
        }
        const errors: string[] = []; host.addEventListener('cem-interaction-error', event => errors.push((event as CustomEvent).detail.code));
        const controller = connectCemNativeSurface(owner, { host }); let opens = 0; host.addEventListener('cem-open', () => opens++);
        try {
            await userEvent.click(source); expect(owner.matches(':popover-open')).toBe(true); expect(opens).toBe(1);
            const veto = (event: Event) => event.preventDefault(); host.addEventListener('cem-before-close', veto);
            await userEvent.click(close); expect(owner.matches(':popover-open')).toBe(true); host.removeEventListener('cem-before-close', veto);
            await userEvent.click(close); expect(owner.matches(':popover-open')).toBe(false); expect(document.activeElement).toBe(source);
            expect(source.hasAttribute('commandfor')).toBe(false); expect(close.hasAttribute('commandfor')).toBe(false);
            required(host, '[slot=trigger]').remove();
            await waitFor(() => expect(source.hasAttribute('aria-controls')).toBe(false)); expect(host.querySelector('[part=trigger]')).toBeNull();
            host.setAttribute('trigger', ''); await waitFor(() => expect(errors).toContain('interaction-control-name-missing'));
            expect(host.querySelector('[part=trigger]')).toBeNull();
            host.setAttribute('trigger-aria-label', 'Edit task');
            await waitFor(() => expect(required(host, '[part=trigger]').getAttribute('aria-label')).toBe('Edit task'));
            await userEvent.click(required(host, '[part=trigger]')); expect(opens).toBe(2); controller.requestClose();
            controller.disconnect(); expect(close.getAttribute('popovertarget')).toBe(owner.id);
        } finally { controller.disconnect(); }
    },
};
export const DeclarativeIdentityWorkerAndFallback: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID(), tag = `cem-task-controls-${suffix}`, declarationTag = `declaration-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) }); runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', 'native-surface');
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = '{attribute @name=caption | Task}{dialog @part=surface @aria-label={$caption} | {template @slot=body | {input @autofocus=true}}}';
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
            const host = document.createElement(tag); host.setAttribute('trigger', 'Edit'); host.setAttribute('close-label', 'Close'); root.append(host);
            try {
                await runtime.whenRenderSettled(host); const owner = required<HTMLDialogElement>(host, 'dialog'), trigger = required<HTMLButtonElement>(host, '[part=trigger]');
                await userEvent.click(trigger); await waitFor(() => expect(owner.open).toBe(true));
                const close = required(owner, '[part=close]'), input = required<HTMLInputElement>(owner, 'input'); input.value = 'Draft';
                host.setAttribute('caption', 'Changed'); host.setAttribute('trigger', 'Review'); await runtime.whenRenderSettled(host);
                expect(host.querySelector('dialog')).toBe(owner); expect(host.querySelector('[part=trigger]')).toBe(trigger); expect(owner.querySelector('[part=close]')).toBe(close);
                expect(trigger.textContent).toBe('Review'); expect(trigger.getAttribute('aria-expanded')).toBe('true'); expect(input.value).toBe('Draft');
                await userEvent.click(close); expect(owner.open).toBe(false); expect(document.activeElement).toBe(trigger);
                host.setAttribute('trigger-disabled', ''); await runtime.whenRenderSettled(host); expect(trigger.disabled).toBe(true);
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
};
export const PendingVisibilityAndExplicitNativeId: Story = {
    render: () => '<section><button type="button">Native launcher</button><div><dialog part="surface" aria-label="Prepared task"><input autofocus></dialog></div></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section > div'), owner = required<HTMLDialogElement>(host, 'dialog'), external = required<HTMLButtonElement>(canvasElement, 'section > button');
        const id = `explicit-surface-${crypto.randomUUID()}`; host.setAttribute('surface-id', id);
        const pending: (() => void)[] = [];
        const controller = connectCemNativeSurface(owner, { host, prepareBody: () => new Promise<void>(resolve => pending.push(resolve)) });
        try {
            expect(owner.id).toBe(id); expect(host.querySelector('[part=trigger]')).toBeNull();
            external.setAttribute('commandfor', id); external.setAttribute('command', '--cem-show');
            await userEvent.click(external); expect(owner.open).toBe(false); expect(document.activeElement).toBe(external);
            host.setAttribute('trigger', 'Local launcher'); host.setAttribute('close-label', 'Cancel');
            await waitFor(() => expect(required(host, '[part=trigger]').getAttribute('aria-expanded')).toBe('false'));
            expect(host.getAttribute('data-state')).toBe('preparing'); controller.requestClose(); pending[0]();
            await Promise.resolve(); expect(owner.open).toBe(false);
            const source = required<HTMLButtonElement>(host, '[part=trigger]'); await userEvent.click(source);
            expect(source.getAttribute('aria-expanded')).toBe('false'); expect(source.getAttribute('aria-busy')).toBe('true');
            pending[1](); await waitFor(() => expect(source.getAttribute('aria-expanded')).toBe('true'));
            expect(owner.open).toBe(true); expect(source.hasAttribute('aria-busy')).toBe(false);
            await userEvent.click(required(owner, '[part=close]')); expect(owner.open).toBe(false); expect(document.activeElement).toBe(source);
        } finally { controller.disconnect(); pending.forEach(resolve => resolve()); }
    },
};
export const HeadingNamesRetainedBody: Story = {
    render: () => '<section><div trigger="Edit" close-label="Done"><dialog part="surface"><h2 part="heading"><em>Projected title</em></h2><template slot="body"><input autofocus></template></dialog></div></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'section > div'), owner = required<HTMLDialogElement>(host, 'dialog');
        const controller = connectCemNativeSurface(owner, { host });
        try {
            expect(owner.getAttribute('aria-labelledby')).toBe(required(owner, 'h2').id);
            await userEvent.click(required(host, '[part=trigger]')); expect(owner.open).toBe(true);
            expect(owner).toHaveAccessibleName('Projected title');
            owner.setAttribute('aria-label', 'Explicit name');
            await waitFor(() => expect(owner.hasAttribute('aria-labelledby')).toBe(false)); expect(owner).toHaveAccessibleName('Explicit name');
            await userEvent.click(required(owner, '[part=close]')); expect(owner.open).toBe(false);
        } finally { controller.disconnect(); }
    },
};
