import { processNativeCemValue } from './internal/runtime-support/cem-ql-render.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { connectCemNativeSurface } from './native-surface.js';
import { captureCemSurfaceInvocation } from './surface-invocation.js';

export default { title: 'CEM Elements/Menu Task Relay', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const element = root.querySelector<T>(selector); if (!element) throw new Error(`Missing ${selector}`); return element;
}
function deferred() { let resolve!: () => void, reject!: (reason?: unknown) => void; const promise = new Promise<void>((done, fail) => { resolve = done; reject = fail; }); return { promise, resolve, reject }; }
async function fixture(root: HTMLElement, fallback = false, independent = false) {
    const suffix = crypto.randomUUID(), declarationTag = `declaration-${suffix}`, popupTag = `cem-relay-popup-${suffix}`, menuTag = `cem-relay-menu-${suffix}`;
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
        ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) }); runtime.install(window);
    const declarations: HTMLElement[] = [];
    for (const [tag, capability, text] of [[popupTag, 'popup', '{attribute @name=open | false}{span @part=base | {button @type=button | Menu}}{div @part=popup | {slot}}'],
        [menuTag, 'composite-menu', '{attribute @name=keyboard | menu}{attribute @name=direction | column}{div @part=composite | {slot}}']]) {
        const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', capability);
        const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = text;
        declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); declarations.push(declaration);
    }
    await Promise.all(declarations.map(node => runtime.whenDeclarationSettled(node)));
    const popup = document.createElement(popupTag);
    const leaf = `<${menuTag} slot="submenu"><button type="button" aria-label="Edit">Edit</button></${menuTag}>`;
    popup.innerHTML = `<${menuTag}><div><button type="button" part="control" aria-label="More">More</button><${menuTag} slot="submenu"><div><button type="button" part="control" aria-label="Advanced">Advanced</button>${independent ? '' : leaf}</div></${menuTag}></div></${menuTag}>`;
    root.append(popup); await runtime.whenRenderSettled(popup);
    for (let i = 0; i < 3; i++) await Promise.all([...popup.querySelectorAll<HTMLElement>(menuTag)].map(node => runtime.whenRenderSettled(node)));
    const more = required<HTMLButtonElement>(popup, '[aria-label=More]'), advanced = required<HTMLButtonElement>(popup, '[aria-label=Advanced]');
    let external: HTMLElement | undefined;
    if (independent) {
        external = document.createElement(menuTag); external.innerHTML = '<button type="button" aria-label="Edit">Edit</button>';
        advanced.id ||= `advanced-${suffix}`; external.setAttribute('parent-item', advanced.id); external.setAttribute('data-cem-node-ref-parent-item', '');
        root.append(external); await runtime.whenRenderSettled(external);
        await waitFor(() => expect(advanced.getAttribute('aria-controls')).toBe(external?.id));
    }
    const launcher = required<HTMLButtonElement>(popup, '[part=base] > button'), edit = required<HTMLButtonElement>(external ?? popup, '[aria-label=Edit]');
    return { popup, launcher, more, advanced, edit, async launch() {
        await userEvent.click(launcher); await userEvent.click(more); await userEvent.click(advanced);
        await expect(document.activeElement).toBe(edit);
    }, cleanup() { popup.remove(); external?.remove(); declarations.forEach(node => node.remove()); scope.dispose(); } };
}
export const NestedAndIndependentNativeHandoff: Story = {
    render: () => '<section><button aria-label="Outside">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) for (const independent of [false, true]) {
            const menu = await fixture(root, fallback, independent);
            const dialog = document.createElement('dialog'); dialog.setAttribute('aria-label', 'Task'); dialog.innerHTML = '<input autofocus aria-label="Task entry">'; root.append(dialog);
            const controller = connectCemNativeSurface(dialog), entry = required(dialog, 'input');
            let opened = 0, entries = 0; dialog.addEventListener('cem-open', () => opened++); entry.addEventListener('focus', () => entries++);
            dialog.id = `task-${crypto.randomUUID()}`; menu.edit.setAttribute('commandfor', dialog.id); menu.edit.setAttribute('command', '--cem-show');
            try {
                await menu.launch(); await userEvent.click(menu.edit);
                await waitFor(() => expect(dialog.open).toBe(true)); expect(menu.popup.getAttribute('open')).toBe('false');
                expect(opened).toBe(1); expect(entries).toBe(1); expect(document.activeElement).toBe(entry);
                controller.requestClose('escape'); expect(document.activeElement).toBe(menu.launcher);
                await menu.launch(); await userEvent.click(menu.edit);
                const outside = required<HTMLButtonElement>(root, '[aria-label=Outside]'); await userEvent.click(outside);
                controller.requestClose('outside'); expect(document.activeElement).toBe(outside);
                await menu.launch(); await userEvent.click(menu.edit); outside.focus(); controller.requestClose('tab'); expect(document.activeElement).toBe(outside);
            } finally { controller.disconnect(); dialog.remove(); menu.cleanup(); }
        }
    },
};
export const PreparationAndCapturedInvocation: Story = {
    render: () => '<section><button aria-label="Outside">Outside</button></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), menu = await fixture(root);
        const dialog = document.createElement('dialog'); dialog.setAttribute('aria-label', 'Prepared task'); dialog.setAttribute('anchor', 'pointer'); dialog.setAttribute('placement', 'block-end start');
        dialog.innerHTML = '<input autofocus>'; root.append(dialog);
        const pending: ReturnType<typeof deferred>[] = [], seen: unknown[] = [];
        const controller = connectCemNativeSurface(dialog, { prepareBody({ invocation }) { seen.push(invocation); const next = deferred(); pending.push(next); return next.promise; } });
        dialog.id = `prepared-${crypto.randomUUID()}`; menu.edit.setAttribute('commandfor', dialog.id); menu.edit.setAttribute('command', '--cem-show');
        try {
            await menu.launch();
            const imported = await processNativeCemValue({ action: 'import', bytes: new TextEncoder().encode('<invoice id="42"/>').buffer, contentType: 'application/xml', sourceUri: 'fixture:relay', scopePolicyStamp: 'test', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS });
            if (!('value' in imported)) throw new Error('Missing native fixture value');
            const context = imported.value;
            const invocation = { ...captureCemSurfaceInvocation(menu.edit, new KeyboardEvent('keydown')), contextKey: 'invoice-42', context };
            controller.open(invocation);
            expect(dialog.open).toBe(false); expect(document.activeElement).toBe(menu.edit); expect(menu.popup.getAttribute('open')).toBe('true');
            expect(seen[0]).toMatchObject({ source: menu.edit, contextKey: 'invoice-42', context, logicalParent: expect.any(HTMLElement), returnDestination: menu.launcher });
            expect((seen[0] as typeof invocation).context).toBe(context);
            expect((seen[0] as typeof invocation).geometry).toEqual(invocation.geometry);
            controller.requestClose(); pending[0].resolve(); await Promise.resolve(); expect(dialog.open).toBe(false); expect(menu.popup.getAttribute('open')).toBe('true');
            await userEvent.click(menu.edit); expect(menu.popup.getAttribute('open')).toBe('true');
            pending[1].resolve(); await waitFor(() => expect(dialog.open).toBe(true));
            expect(menu.popup.getAttribute('open')).toBe('false'); expect(document.activeElement).toBe(required(dialog, 'input'));
            controller.requestClose('escape'); expect(document.activeElement).toBe(menu.launcher);
            await menu.launch(); await userEvent.click(menu.edit);
            const outside = required(root, '[aria-label=Outside]'); await userEvent.click(outside); pending[2].resolve();
            await Promise.resolve(); await Promise.resolve(); expect(dialog.open).toBe(false); expect(document.activeElement).toBe(outside);
            await menu.launch(); const veto = (event: Event) => event.preventDefault(); dialog.addEventListener('cem-before-open', veto);
            await userEvent.click(menu.edit); expect(menu.popup.getAttribute('open')).toBe('true'); expect(dialog.open).toBe(false);
            dialog.removeEventListener('cem-before-open', veto);
            await userEvent.click(menu.edit); pending[3].reject(new Error('Preparation rejected'));
            await waitFor(() => expect(dialog.hasAttribute('aria-busy')).toBe(false));
            expect(menu.popup.getAttribute('open')).toBe('true'); expect(document.activeElement).toBe(menu.edit);
            await userEvent.click(menu.edit);
            // Close and reopen the same chain before mutation delivery: a new
            // menu lifetime must not resurrect this pending task invocation.
            menu.more.click(); menu.more.click(); menu.advanced.click(); pending[4].resolve();
            await Promise.resolve(); await Promise.resolve(); expect(dialog.open).toBe(false);
            expect(menu.popup.getAttribute('open')).toBe('true');
            await userEvent.click(menu.edit); controller.disconnect(); pending[5].resolve();
            await Promise.resolve(); expect(dialog.open).toBe(false); expect(menu.popup.getAttribute('open')).toBe('true');
        } finally { controller.disconnect(); pending.forEach(next => next.resolve()); dialog.remove(); menu.cleanup(); }
    },
};
export const NativePopoverAndModalRoutes: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const modal of [false, true]) {
            const menu = await fixture(root, false, true), owner = document.createElement('dialog');
            owner.id = `native-route-${crypto.randomUUID()}`; owner.setAttribute('aria-label', 'Native task'); owner.innerHTML = '<input autofocus>';
            if (!modal) owner.setAttribute('popover', 'manual'); root.append(owner);
            const controller = connectCemNativeSurface(owner); let entries = 0, opens = 0;
            required(owner, 'input').addEventListener('focus', () => entries++); owner.addEventListener('cem-open', () => opens++);
            if (modal) { menu.edit.setAttribute('commandfor', owner.id); menu.edit.setAttribute('command', 'show-modal'); }
            else { menu.edit.setAttribute('popovertarget', owner.id); menu.edit.setAttribute('popovertargetaction', 'show'); }
            try {
                await menu.launch(); await userEvent.click(menu.edit);
                await waitFor(() => expect(owner.matches(modal ? ':modal' : ':popover-open')).toBe(true));
                expect(menu.popup.getAttribute('open')).toBe('false'); expect(entries).toBe(1); expect(opens).toBe(1);
                controller.requestClose('escape'); expect(document.activeElement).toBe(menu.launcher);
            } finally { controller.disconnect(); owner.remove(); menu.cleanup(); }
        }
    },
};
