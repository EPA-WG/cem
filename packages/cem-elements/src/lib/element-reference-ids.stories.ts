import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Element Reference IDs', tags: ['test'] } satisfies Meta;
type Story = StoryObj;

function required<T extends Element>(parent: ParentNode, selector: string): T {
    const element = parent.querySelector<T>(selector);
    if (!element) throw new Error(`Missing fixture element: ${selector}`);
    return element;
}

export const NativeRelationshipsWorkerAndFallback: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required<HTMLElement>(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const suffix = crypto.randomUUID();
            const declarationTag = `cem-reference-declaration-${suffix}`;
            const tag = `cem-reference-card-${suffix}`;
            const actionTag = `cem-reference-action-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture selects fallback'); } } : {}) });
            runtime.install(window);
            const declare = (name: string, source: string, capability?: string) => {
                const declaration = document.createElement(declarationTag);
                declaration.setAttribute('tag', name);
                if (capability) declaration.setAttribute('capability', capability);
                const template = document.createElement('template');
                template.setAttribute('type', 'text/cem-ml');
                template.textContent = source;
                declaration.append(template); root.append(declaration);
                runtime.registerDeclaration(declaration);
                return declaration;
            };
            const action = declare(actionTag, '{button @part=control @type=button | Open}', 'action-command');
            const card = declare(tag, `{attribute @name=tone | initial}{cem:variable @name=target @select='data:read("<dialog><p>Body</p></dialog>", "xml").root.children'}{${actionTag} @command-target={#target} @command=show-modal}{button @type=button @commandfor={#target} @command=show-modal | Native}{$target}{span | {$tone}}`);
            const instances = [document.createElement(tag), document.createElement(tag)];
            root.append(...instances);
            try {
                await Promise.all(instances.map(instance => runtime.whenRenderSettled(instance)));
                const dialogs = instances.map(instance => required<HTMLDialogElement>(instance, 'dialog'));
                await expect(dialogs[0].id).toBeTruthy();
                await expect(dialogs[0].id).not.toBe(dialogs[1].id);
                for (const instance of instances) {
                    const dialog = required<HTMLDialogElement>(instance, 'dialog');
                    const invoker = required<HTMLElement>(instance, actionTag);
                    await runtime.whenRenderSettled(invoker);
                    const control = required<HTMLButtonElement>(invoker, 'button');
                    await waitFor(() => expect(control.getAttribute('commandfor')).toBe(dialog.id));
                    await expect(invoker.getAttribute('command-target')).toBe(dialog.id);
                    await expect(invoker.hasAttribute('data-cem-node-ref-command-target')).toBe(true);
                    await expect(required<HTMLButtonElement>(instance, ':scope > button').getAttribute('commandfor')).toBe(dialog.id);
                    await userEvent.click(control);
                    await expect(dialog.open).toBe(true);
                    dialog.close();
                    const before = dialog.id;
                    instance.setAttribute('tone', 'changed');
                    await runtime.whenRenderSettled(instance);
                    await expect(required<HTMLDialogElement>(instance, 'dialog').id).toBe(before);
                }
                // Existing @name conveniences remain scoped browser inputs.
                const legacyScope = document.createElement('div'); legacyScope.setAttribute('interaction-scope', 'local');
                const local = document.createElement(actionTag); local.setAttribute('command-target', '@surface'); local.setAttribute('command', 'show-modal');
                const surface = document.createElement('dialog'); surface.setAttribute('interaction-name', 'surface');
                legacyScope.append(local, surface); root.append(legacyScope);
                await runtime.whenRenderSettled(local);
                await waitFor(() => expect(required<HTMLButtonElement>(local, 'button').getAttribute('commandfor')).toBe(surface.id));
                await expect(local.hasAttribute('data-cem-node-ref-command-target')).toBe(false);
                legacyScope.remove();
            } finally {
                for (const instance of instances) instance.remove();
                action.remove(); card.remove(); scope.dispose();
            }
        }
    },
};
