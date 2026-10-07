import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { ELEMENT_REFERENCE_TEMPLATE, elementReferenceFixtureInputs } from './element-reference-lifecycle.fixtures.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- use original native owners through explicit source bundle export.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Element Reference Lifecycle', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends Element>(parent: ParentNode, selector: string): T {
    const node = parent.querySelector<T>(selector);
    if (!node) throw new Error(`Missing fixture ${selector}`);
    return node;
}
function declare(root: HTMLElement, runtime: CemElementRuntime, declarationTag: string, tag: string, source: string, capability?: string): HTMLElement {
    const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag);
    if (capability) declaration.setAttribute('capability', capability);
    const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = source;
    declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); return declaration;
}
export const ExplicitInputsWorkerFallbackReplacementAndDisposal: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = required<HTMLElement>(canvasElement, 'section');
        const common = elementReferenceFixtureInputs();
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const tag = `cem-source-card-${crypto.randomUUID()}`; const declarationTag = `declaration-${tag}`;
            let admitted = false, pending = false, calls = 0, references = common;
            let release: () => void = () => undefined;
            let gate = Promise.resolve();
            let lastSignal: AbortSignal | undefined;
            const hold = () => { pending = true; gate = new Promise<void>(resolve => { release = resolve; }); };
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture fallback'); } } : {}),
                elementReferenceInputs: async (_instance, _snapshot, signal) => {
                    lastSignal = signal;
                    calls++;
                    const inputs = { ...references, grants: admitted ? references.grants : [] };
                    if (pending) await gate;
                    return inputs;
                } });
            runtime.install(window);
            const declaration = declare(root, runtime, declarationTag, tag, ELEMENT_REFERENCE_TEMPLATE);
            const instances = [document.createElement(tag), document.createElement(tag)]; root.append(...instances);
            try {
                await Promise.all(instances.map(i => runtime.whenRenderSettled(i)));
                await expect(instances[0].querySelector('button')).toBeNull();
                admitted = true;
                instances.forEach(i => runtime.refreshElementReferences(i));
                await Promise.all(instances.map(i => runtime.whenRenderSettled(i)));
                const ids = instances.map(i => required<HTMLDialogElement>(i, 'dialog').id);
                await expect(ids[0]).not.toBe(ids[1]);
                const button = required<HTMLButtonElement>(instances[0], 'button');
                const target = required<HTMLDialogElement>(instances[0], 'dialog');
                await expect(button.getAttribute('commandfor')).toBe(target.id);
                await userEvent.click(button); await expect(target.open).toBe(true); target.close();
                admitted = false; runtime.refreshElementReferences(instances[0]);
                await runtime.whenRenderSettled(instances[0]);
                await expect(instances[0].querySelector('button')).toBe(button);
                await expect(instances[0].querySelector('dialog')).toBe(target);
                await expect(button.getAttribute('commandfor')).toBe(ids[0]);
                admitted = true; instances[0].setAttribute('tone', 'replacement');
                await runtime.whenRenderSettled(instances[0]);
                await expect(required<HTMLDialogElement>(instances[0], 'dialog').id).toBe(ids[0]);
                const oldCalls = calls; hold(); runtime.refreshElementReferences(instances[0]);
                await waitFor(() => expect(calls).toBeGreaterThan(oldCalls));
                const priorSignal = lastSignal;
                pending = false; references = elementReferenceFixtureInputs('{dialog @id=replacement-public-part | Replacement}');
                runtime.refreshElementReferences(instances[0]); await runtime.whenRenderSettled(instances[0]);
                await expect(priorSignal?.aborted).toBe(true);
                release(); await gate;
                await waitFor(() => expect(required<HTMLButtonElement>(instances[0], 'button').getAttribute('commandfor')).toBe('replacement-public-part'));
                references = common; runtime.refreshElementReferences(instances[0]); await runtime.whenRenderSettled(instances[0]);
                const before = calls; hold(); instances[0].setAttribute('tone', 'stale');
                await waitFor(() => expect(calls).toBeGreaterThan(before));
                const disconnectedSignal = lastSignal;
                instances[0].remove();
                await expect(disconnectedSignal?.aborted).toBe(true);
                release();
                await runtime.whenRenderSettled(instances[0]);
                await expect(instances[0].textContent).not.toContain('stale');
                pending = false; root.append(instances[0]); await runtime.whenRenderSettled(instances[0]);
                await expect(required<HTMLDialogElement>(instances[0], 'dialog').id).toBe(ids[0]);
            } finally { release(); instances.forEach(i => i.remove()); declaration.remove(); scope.dispose(); }
        }
    },
};

export const TypedPopupAndIndependentSubmenuEndpoints: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required<HTMLElement>(canvasElement, 'section');
        const scope = createCemDeclarationScope({ document }); const suffix = crypto.randomUUID();
        const declarationTag = `cem-provider-declaration-${suffix}`, card = `cem-provider-card-${suffix}`, popup = `cem-provider-popup-${suffix}`, menu = `cem-provider-menu-${suffix}`;
        const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope }); runtime.install(window);
        const declarations = [
            declare(root, runtime, declarationTag, popup, '{attribute @name=open | false}{div @part=base | {button @type=button | Default}}{div @part=popup | {button @type=button | Content}}', 'popup'),
            declare(root, runtime, declarationTag, menu, '{attribute @name=keyboard | menu}{div @part=composite | {slot}}', 'composite-menu'),
            declare(root, runtime, declarationTag, card, `{attribute @name=tone | initial}{cem:variable @name=trigger @select='data:read("<button type=\\"button\\">Trigger</button>", "xml").root.children'}{${popup} @trigger-for={#trigger} @open=false}{$trigger}{span | {$tone}}`),
        ];
        const instance = document.createElement(card); root.append(instance);
        try {
            await runtime.whenRenderSettled(instance);
            const surface = required<HTMLElement>(instance, popup); await runtime.whenRenderSettled(surface);
            const trigger = required<HTMLButtonElement>(instance, ':scope > button');
            await waitFor(() => expect(trigger.getAttribute('aria-controls')).toBe(required<HTMLElement>(surface, '[part=popup]').id));
            await expect(surface.hasAttribute('data-cem-node-ref-trigger-for')).toBe(true);
            await expect(required<HTMLElement>(surface, '[part=base]').hidden).toBe(true);
            await userEvent.click(trigger); await expect(surface.getAttribute('open')).toBe('true');
            await userEvent.keyboard('{Escape}'); await expect(surface.getAttribute('open')).toBe('false');
            instance.setAttribute('tone', 'rerender'); await runtime.whenRenderSettled(instance);
            await expect(required<HTMLButtonElement>(instance, ':scope > button')).toBe(trigger);
            const alternate = document.createElement('button'); alternate.id = `alternate-${suffix}`; alternate.textContent = 'Alternate'; root.append(alternate);
            surface.setAttribute('trigger-for', alternate.id);
            await waitFor(() => expect(alternate.hasAttribute('aria-controls')).toBe(true));
            await expect(trigger.hasAttribute('aria-controls')).toBe(false);
            await userEvent.click(alternate); await expect(surface.getAttribute('open')).toBe('true');
            const errors: string[] = []; surface.addEventListener('cem-interaction-error', e => errors.push((e as CustomEvent).detail.code));
            surface.setAttribute('trigger-for', required<HTMLElement>(instance, ':scope > span').id = `wrong-${suffix}`);
            await waitFor(() => expect(errors).toContain('interaction-reference-conflict'));
            await expect(required<HTMLElement>(surface, '[part=popup]').hidden).toBe(true);
            await expect(alternate.hasAttribute('aria-controls')).toBe(false);
            // Legacy local names stay within the nearest interaction scope.
            const localScope = document.createElement('div'); localScope.setAttribute('interaction-scope', 'inner');
            const local = document.createElement('button'); local.setAttribute('interaction-name', 'invoker');
            surface.removeAttribute('data-cem-node-ref-trigger-for'); surface.setAttribute('trigger-for', '@invoker');
            localScope.append(surface, local); root.append(localScope);
            await waitFor(() => expect(local.hasAttribute('aria-controls')).toBe(true));
            surface.remove(); await waitFor(() => expect(local.hasAttribute('aria-controls')).toBe(false));
            localScope.remove(); alternate.remove();

            const parent = document.createElement(menu), child = document.createElement(menu);
            const owner = document.createElement('button'); owner.id = `menu-owner-${suffix}`; owner.textContent = 'Submenu';
            parent.append(owner); child.innerHTML = '<button type="button">Child action</button>';
            child.setAttribute('parent-item', owner.id); child.setAttribute('data-cem-node-ref-parent-item', '');
            root.append(parent, child); await Promise.all([runtime.whenRenderSettled(parent), runtime.whenRenderSettled(child)]);
            const control = required<HTMLButtonElement>(parent, 'button'), action = required<HTMLButtonElement>(child, 'button');
            await waitFor(() => expect(control.getAttribute('aria-controls')).toBe(child.id));
            await userEvent.click(control); await expect(child.hidden).toBe(false); await expect(document.activeElement).toBe(action);
            await userEvent.keyboard('{Escape}'); await expect(child.hidden).toBe(true); await expect(document.activeElement).toBe(control);
            child.setAttribute('parent-item', `wrong-${suffix}`);
            await waitFor(() => expect(control.hasAttribute('aria-controls')).toBe(false));
            child.setAttribute('parent-item', control.id);
            await waitFor(() => expect(control.getAttribute('aria-controls')).toBe(child.id));
            // Multiple linked panels are rejected instead of choosing one.
            const duplicate = document.createElement(menu); duplicate.innerHTML = '<button>Duplicate</button>';
            duplicate.setAttribute('parent-item', control.id); duplicate.setAttribute('data-cem-node-ref-parent-item', ''); root.append(duplicate);
            await runtime.whenRenderSettled(duplicate);
            await waitFor(() => expect(control.hasAttribute('aria-controls')).toBe(false));
            duplicate.remove(); await waitFor(() => expect(control.getAttribute('aria-controls')).toBe(child.id));
            // A cycle disables execution even though both controls exist.
            parent.setAttribute('parent-item', action.id || (action.id = `child-owner-${suffix}`)); parent.setAttribute('data-cem-node-ref-parent-item', '');
            await waitFor(() => expect(control.getAttribute('role')).toBeNull());
            await expect(parent.hidden).toBe(false);
            await userEvent.click(control); await expect(child.hidden).toBe(true);
            parent.removeAttribute('parent-item'); parent.removeAttribute('data-cem-node-ref-parent-item');
            await waitFor(() => expect(control.getAttribute('role')).toBe('menuitem'));
            const legacy = document.createElement('div'); legacy.setAttribute('interaction-scope', 'menu-local');
            control.setAttribute('interaction-name', 'parent-control');
            child.removeAttribute('data-cem-node-ref-parent-item'); child.setAttribute('parent-item', '@parent-control');
            legacy.append(parent, child); root.append(legacy);
            await Promise.all([runtime.whenRenderSettled(parent), runtime.whenRenderSettled(child)]);
            const localControl = required<HTMLButtonElement>(parent, 'button');
            await waitFor(() => expect(localControl.getAttribute('aria-controls')).toBe(child.id));
            await userEvent.click(localControl); await expect(child.hidden).toBe(false);
            await userEvent.keyboard('{Escape}'); await expect(child.hidden).toBe(true);
            child.removeAttribute('parent-item');
            await waitFor(() => expect(localControl.hasAttribute('aria-controls')).toBe(false));
            await expect(child.hidden).toBe(false);
            child.setAttribute('parent-item', '@parent-control');
            await waitFor(() => expect(localControl.getAttribute('aria-controls')).toBe(child.id));
            child.remove(); await waitFor(() => expect(localControl.hasAttribute('aria-controls')).toBe(false)); parent.remove(); legacy.remove();
        } finally { instance.remove(); declarations.forEach(d => d.remove()); root.replaceChildren(); scope.dispose(); }
    },
};
