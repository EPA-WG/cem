import { expect, userEvent, waitFor } from 'storybook/test';
import preview, { loadCemDeclaration, whenCemRendered, createCemStoryRuntime } from '../../../../cem-elements/.storybook/preview.js';
import declarationSource from './cem-dialog.xhtml?raw';
const meta = preview.meta({ component: 'cem-dialog', title: 'CEM Components/cem-dialog', loaders: [async () => { await loadCemDeclaration('cem-dialog', declarationSource); return {}; }] });
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
export const NamesAndNonmodalLifecycle = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dialog label="Edit record" trigger="Edit" close-label="Done"><label>Name <input autofocus value="Saved"></label></cem-dialog><button type="button">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-dialog'); await whenCemRendered(host);
        const owner = required<HTMLDialogElement>(host, 'dialog'), trigger = required<HTMLButtonElement>(host, '[part=trigger]');
        expect(owner.open).toBe(false); expect(required(owner, '[part=heading]').textContent).toBe('Edit record'); expect(owner.hasAttribute('aria-modal')).toBe(false);
        await userEvent.click(trigger); await waitFor(() => expect(owner.open).toBe(true)); expect(owner.matches(':modal')).toBe(false); expect(owner).toHaveAccessibleName('Edit record');
        expect(document.activeElement).toBe(required(owner, 'input')); expect(trigger.getAttribute('aria-expanded')).toBe('true');
        const input = required<HTMLInputElement>(owner, 'input'); input.value = 'Draft'; host.setAttribute('label', 'Renamed record'); await whenCemRendered(host);
        expect(required(host, 'dialog')).toBe(owner); expect(owner).toHaveAccessibleName('Renamed record'); expect(input.value).toBe('Draft');
        await userEvent.click(required(canvasElement, 'section > button')); expect(owner.open).toBe(true);
        await userEvent.click(required(owner, '[part=close]')); expect(owner.open).toBe(false); expect(document.activeElement).toBe(trigger);
        host.setAttribute('aria-label', 'Accessible task'); await whenCemRendered(host); await userEvent.click(trigger); await waitFor(() => expect(owner.open).toBe(true)); expect(owner).toHaveAccessibleName('Accessible task'); await userEvent.click(required(owner, '[part=close]'));
        expect(required(owner, '[part=heading]').textContent).toBe('Renamed record'); expect(host.querySelector('style')).toBeNull();
    },
});
export const ModalFormsAndCancel = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dialog label="Confirm details" mode="modal" trigger="Confirm" close-label="Cancel"><form method="dialog"><label>Required name <input autofocus required name="name"></label><button type="submit" value="saved">Save</button></form></cem-dialog></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-dialog'); await whenCemRendered(host); const owner = required<HTMLDialogElement>(host, 'dialog'), trigger = required(host, '[part=trigger]');
        await userEvent.click(trigger); await waitFor(() => expect(owner.matches(':modal')).toBe(true));
        await userEvent.click(required(owner, 'button[type=submit]')); expect(owner.open).toBe(true);
        const veto = (event: Event) => event.preventDefault(); host.addEventListener('cem-before-close', veto);
        await userEvent.click(required(owner, '[part=close]')); expect(owner.open).toBe(true); host.removeEventListener('cem-before-close', veto);
        await userEvent.type(required(owner, 'input'), 'Ada'); await userEvent.click(required(owner, 'button[type=submit]'));
        await waitFor(() => expect(owner.open).toBe(false)); expect(owner.returnValue).toBe('saved'); expect(document.activeElement).toBe(trigger);
        await userEvent.click(trigger); await waitFor(() => expect(owner.matches(':modal')).toBe(true));
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser'); await native.keyboard('{Escape}');
        await waitFor(() => expect(owner.open).toBe(false)); expect(document.activeElement).toBe(trigger);
    },
    parameters: { docs: { description: { story: 'Trusted native Escape is checked in the browser runner. Form validation and dialog return values stay browser-owned.' } } },
});
export const SlotsRetentionAndDisposal = meta.story({
    render: () => '<section><cem-dialog label="Fallback" trigger="Replaced" close-label="Replaced" materialize="retain"><template><button slot="trigger" type="button">Custom launcher</button><strong slot="label">Projected task title</strong><template slot="body"><input autofocus aria-label="Draft"></template><button slot="close" type="button">Custom close</button></template></cem-dialog></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-dialog'); await whenCemRendered(host); const owner = required<HTMLDialogElement>(host, 'dialog'), source = required(host, '[slot=trigger]');
        expect(required(owner, '[part=heading]').textContent).toBe('Projected task title'); expect(host.querySelector('[part=trigger]')).toBeNull(); expect(owner.querySelector('input')).toBeNull();
        await userEvent.click(source); await waitFor(() => expect(owner.open).toBe(true)); expect(owner).toHaveAccessibleName('Projected task title'); const input = required<HTMLInputElement>(owner, 'input'); input.value = 'Retained draft';
        await userEvent.click(required(owner, '[slot=close]')); await userEvent.click(source); await waitFor(() => expect(owner.open).toBe(true));
        expect(required(owner, 'input')).toBe(input); expect(input.value).toBe('Retained draft');
        host.setAttribute('materialize', 'dispose'); await whenCemRendered(host); await userEvent.click(required(owner, '[slot=close]'));
        await waitFor(() => expect(input.isConnected).toBe(false)); expect(document.activeElement).toBe(source);
        await userEvent.click(source); await waitFor(() => expect(owner.open).toBe(true)); expect(required(owner, 'input')).not.toBe(input);
        host.remove(); expect(owner.open).toBe(false);
    },
});
export const ExternalReferencesPopoverAndDefaultOpen = meta.story({
    render: () => '<section interaction-scope><button type="button" interaction-name="launcher">External</button><button type="button" interaction-name="workflow">Workflow</button><cem-dialog label="Preview" trigger-for="@launcher" return-focus="@workflow" close-label="Done" popover="manual"><input autofocus></cem-dialog><cem-dialog label="Initially open" default-open close-label="Close">Persistent task</cem-dialog></section>',
    play: async ({ canvasElement }) => {
        const host = required(canvasElement, 'cem-dialog[label=Preview]'), initial = required(canvasElement, 'cem-dialog[default-open]'); await whenCemRendered(host); await whenCemRendered(initial);
        const owner = required<HTMLDialogElement>(host, 'dialog'), initialOwner = required<HTMLDialogElement>(initial, 'dialog');
        await waitFor(() => expect(initialOwner.open).toBe(true)); await userEvent.click(required(initialOwner, '[part=close]'));
        await userEvent.click(required(canvasElement, '[interaction-name=launcher]')); await waitFor(() => expect(owner.matches(':popover-open')).toBe(true));
        expect(owner.open).toBe(false); await userEvent.click(required(owner, '[part=close]')); expect(owner.matches(':popover-open')).toBe(false);
        expect(document.activeElement).toBe(required(canvasElement, '[interaction-name=workflow]'));
        initial.remove(); required(canvasElement, 'section').append(initial); await whenCemRendered(initial); expect(required<HTMLDialogElement>(initial, 'dialog').open).toBe(false);
    },
});

export const KeyboardFallbackFocus = meta.story({
    render: () => '<section class="cem-theme-light"><cem-dialog label="Keyboard task" trigger="Open task" mode="modal"><p>Read this task.</p></cem-dialog></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { userEvent: native } = await import('vitest/browser');
        const host = required<HTMLElement>(canvasElement, 'cem-dialog'); await whenCemRendered(host);
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

export const TypedReferencesWorkerAndFallback = meta.story({
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const { runtime, scope, declare } = createCemStoryRuntime(root, fallback);
            const suffix = crypto.randomUUID(), tag = `cem-dialog-contract-${suffix}`, producer = `cem-dialog-producer-${suffix}`;
            const component = await declare(declarationSource, tag);
            const template = `{cem:variable @name=launcher @select='data:read("<button type=\\"button\\">Launch</button>", "xml").root.children'}{cem:variable @name=entry @select='data:read("<input aria-label=\\"Entry\\"/>", "xml").root.children'}{cem:variable @name=returner @select='data:read("<button type=\\"button\\">Return</button>", "xml").root.children'}{$launcher}{${tag} @label="Typed task" @trigger-for={#launcher} @focus-target={#entry} @return-focus={#returner} @close-label=Done | {$entry}}{$returner}`;
            const source = await declare(`<cem-element><template type="text/cem-ml">${template.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')}</template></cem-element>`, producer);
            const instance = document.createElement(producer); root.append(instance);
            try {
                await runtime.whenRenderSettled(instance); const host = required(instance, tag); await runtime.whenRenderSettled(host);
                const owner = required<HTMLDialogElement>(host, 'dialog'), launcher = required<HTMLButtonElement>(instance, ':scope > button'), entry = required<HTMLInputElement>(owner, 'input'), returner = required<HTMLButtonElement>(instance, ':scope > button:last-of-type');
                for (const name of ['trigger-for', 'focus-target', 'return-focus']) expect(host.hasAttribute(`data-cem-node-ref-${name}`)).toBe(true);
                await userEvent.click(launcher); await waitFor(() => expect(owner.open).toBe(true)); expect(document.activeElement).toBe(entry);
                entry.value = 'Retained'; instance.setAttribute('revision', 'next'); await runtime.whenRenderSettled(instance); await runtime.whenRenderSettled(host);
                expect(required(host, 'dialog')).toBe(owner); expect(entry.value).toBe('Retained');
                await userEvent.click(required(owner, '[part=close]')); await waitFor(() => expect(owner.open).toBe(false)); expect(document.activeElement).toBe(returner);
                expect(runtime.diagnosticsFor(host)).toEqual([]);
            } finally { instance.remove(); component.remove(); source.remove(); scope.dispose(); }
        }
    },
});
