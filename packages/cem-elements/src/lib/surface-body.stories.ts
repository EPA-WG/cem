import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { connectCemNativeSurface, type CemSurfacePreparation } from './native-surface.js';
import { CemElementRuntime, writeDataIslandHydrationData } from './cem-elements.js';
import { applyRenderPlanToRange, applyPatchFramesToRange, diffRenderPlansToPatchFrames, renderPlanIdentity, renderedNativeAttributeBindings, type RenderPlan } from './projection.js';
import { ownsCemSurfaceBodyNode } from './surface-body.js';
import { createCemDeclarationScope } from './declaration-scope.js';

export default { title: 'CEM Elements/Surface Body Lifecycle', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
function required<T extends HTMLElement>(root: ParentNode, selector: string): T {
    const node = root.querySelector<T>(selector); if (!node) throw new Error(`Missing ${selector}`); return node;
}
function deferred() { let resolve!: () => void; const promise = new Promise<void>(done => { resolve = done; }); return { promise, resolve }; }

export const RetainDisposeAndExitCleanup: Story = {
    render: () => '<section><button>Launch</button><input aria-label="Application draft" value="Saved separately"><dialog aria-label="Editor"><template slot="body"><input aria-label="Body draft" value="Initial"></template></dialog></section>',
    play: async ({ canvasElement }) => {
        const dialog = required<HTMLDialogElement>(canvasElement, 'dialog'), source = required(canvasElement, 'button');
        const application = required<HTMLInputElement>(canvasElement, 'section > input');
        const controller = connectCemNativeSurface(dialog);
        try {
            expect(dialog.querySelector('input')).toBeNull(); source.focus();
            expect(controller.open({ source })).toBe(true);
            const input = required<HTMLInputElement>(dialog, 'input'); input.value = 'Draft'; input.focus();
            expect(controller.requestClose()).toBe(true);
            await waitFor(() => expect(dialog.dataset.state).toBe('closed'));
            expect(dialog.querySelector('input')).toBe(input);
            controller.open({ source }); expect(dialog.querySelector('input')).toBe(input); expect(input.value).toBe('Draft');
            dialog.setAttribute('materialize', 'dispose');
            const exit = input.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 100_000 });
            controller.requestClose();
            expect(dialog.open).toBe(false); expect(document.activeElement).toBe(source);
            expect(dialog.dataset.state).toBe('closing'); expect(input.isConnected).toBe(true);
            exit.finish(); await waitFor(() => expect(input.isConnected).toBe(false));
            expect(application.value).toBe('Saved separately');
            controller.open({ source }); const next = required<HTMLInputElement>(dialog, 'input');
            expect(next).not.toBe(input); expect(next.value).toBe('Initial');
            // A late exit completion from an earlier close cannot dispose a reopened body.
            const oldExit = next.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 100_000 });
            controller.requestClose(); controller.open({ source }); oldExit.finish();
            await Promise.resolve(); await Promise.resolve(); expect(dialog.open).toBe(true); expect(next.isConnected).toBe(true);
        } finally { controller.disconnect(); }
    },
};
export const PreparationCancellationStalenessAndFailure: Story = {
    render: () => '<section><button aria-expanded="false">Launch</button><dialog materialize="dispose" aria-label="Prepared editor"><template slot="body"><input value="Initial"></template></dialog></section>',
    play: async ({ canvasElement }) => {
        const dialog = required<HTMLDialogElement>(canvasElement, 'dialog'), source = required(canvasElement, 'button');
        const requests: { preparation: CemSurfacePreparation; done: ReturnType<typeof deferred> }[] = [];
        let fail = false, opens = 0; dialog.addEventListener('cem-open', () => opens++);
        const controller = connectCemNativeSurface(dialog, { prepareBody(preparation) {
            if (fail) return Promise.reject(new Error('Fixture failure'));
            const done = deferred(); requests.push({ preparation, done }); return done.promise;
        } });
        try {
            source.focus(); expect(controller.open({ source })).toBe(true);
            expect(dialog.open).toBe(false); expect(dialog.dataset.state).toBe('preparing');
            expect(document.activeElement).toBe(source); expect(source.getAttribute('aria-busy')).toBe('true');
            expect(source.getAttribute('aria-expanded')).toBe('false');
            const veto = (e: Event) => e.preventDefault(); dialog.addEventListener('cem-before-close', veto);
            expect(controller.requestClose()).toBe(false); dialog.removeEventListener('cem-before-close', veto);
            expect(controller.requestClose()).toBe(true); expect(requests[0].preparation.signal.aborted).toBe(true);
            expect(source.hasAttribute('aria-busy')).toBe(false); expect(dialog.querySelector('input')).toBeNull();
            controller.open({ source }); requests[0].done.resolve(); await Promise.resolve();
            expect(dialog.open).toBe(false); expect(dialog.dataset.state).toBe('preparing');
            requests[1].done.resolve(); await waitFor(() => expect(dialog.open).toBe(true)); expect(opens).toBe(1);
            controller.requestClose(); await waitFor(() => expect(dialog.dataset.state).toBe('closed'));
            fail = true; controller.open({ source }); await waitFor(() => expect(dialog.dataset.state).toBe('closed'));
            expect(dialog.open).toBe(false); expect(dialog.querySelector('input')).toBeNull();
            fail = false; controller.open({ source }); controller.disconnect(); requests[2].done.resolve();
            await Promise.resolve(); await Promise.resolve(); expect(dialog.open).toBe(false); expect(source.hasAttribute('aria-busy')).toBe(false);
        } finally { controller.disconnect(); }
    },
};
export const BodyContractAndNativeOpening: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const body of ['<input>', '<template slot="body"><input></template><template slot="body"></template>', '<template slot="body"><input></template><input>']) {
            const dialog = document.createElement('dialog'); dialog.setAttribute('materialize', 'dispose'); dialog.innerHTML = body; root.append(dialog);
            const controller = connectCemNativeSurface(dialog);
            try { expect(controller.open()).toBe(false); expect(dialog.open).toBe(false); } finally { controller.disconnect(); dialog.remove(); }
        }
        const dialog = document.createElement('dialog'); dialog.setAttribute('materialize', 'eager'); dialog.innerHTML = '<template slot="body"><input value="Ready"></template>'; root.append(dialog);
        const controller = connectCemNativeSurface(dialog);
        try { expect(dialog.querySelector('input')).not.toBeNull(); dialog.showModal(); await waitFor(() => expect(dialog.matches(':modal')).toBe(true)); }
        finally { controller.disconnect(); dialog.remove(); }
    },
};
export const DeclarativeBodyWorkerAndFallback: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID();
            const tag = `cem-lazy-surface-${suffix}`, declarationTag = `declaration-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}) }); runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', 'native-surface');
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml');
            template.textContent = `{attribute @name=caption | Initial}{slice @name=draft | Initial}{dialog @part=surface @aria-label={$caption} | {template @slot=body | {input @value={$caption} @slice=draft @slice-event=input @slice-value="$target.value"}{button @slice=selection @slice-event=click | {attribute @name=slice-value @type=node @value='{data:read("null", "json").root}'}Select}}}`;
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
            const host = document.createElement(tag); host.setAttribute('default-open', ''); root.append(host);
            try {
                await runtime.whenRenderSettled(host); const owner = required<HTMLDialogElement>(host, 'dialog');
                await waitFor(() => expect(owner.open).toBe(true)); const input = required<HTMLInputElement>(owner, 'input'); input.value = 'Retained';
                const nativeButton = required<HTMLButtonElement>(owner, 'button');
                expect(renderedNativeAttributeBindings(required<HTMLButtonElement>(required<HTMLTemplateElement>(owner, 'template').content, 'button')).some(binding => binding.name === 'slice-value')).toBe(true);
                nativeButton.click(); await runtime.whenRenderSettled(host);
                await waitFor(() => expect(runtime.snapshotInstance(host).nativeSlices?.some(binding => binding.name === 'selection')).toBe(true));
                input.dispatchEvent(new Event('input', { bubbles: true })); await runtime.whenRenderSettled(host);
                expect(runtime.snapshotInstance(host).slices.draft).toBe('Retained');
                host.setAttribute('caption', 'Changed'); await runtime.whenRenderSettled(host);
                expect(owner.getAttribute('aria-label')).toBe('Changed');
                expect(required<HTMLInputElement>(required<HTMLTemplateElement>(owner, 'template').content, 'input').getAttribute('value')).toBe('Changed');
                expect(host.querySelector('dialog')).toBe(owner); expect(owner.querySelector('input')).toBe(input); expect(input.value).toBe('Retained');
                owner.close(); await waitFor(() => expect(host.dataset.state).toBe('closed'));
                host.remove(); root.append(host); await runtime.whenRenderSettled(host);
                expect(host.querySelector('dialog')).toBe(owner); expect(owner.open).toBe(false); expect(owner.querySelector('input')).toBe(input);
                host.setAttribute('materialize', 'dispose');
                const controller = connectCemNativeSurface(owner, { host }); controller.open(); controller.requestClose();
                await waitFor(() => expect(owner.querySelector('input')).toBeNull());
                expect(runtime.snapshotInstance(host).slices.draft).toBe('Retained');
                controller.open(); expect(required<HTMLInputElement>(owner, 'input').value).toBe('Changed');
                controller.requestClose(); await waitFor(() => expect(owner.querySelector('input')).toBeNull());
                const snapshot = runtime.snapshotInstance(host), resumed = host.cloneNode(true) as HTMLElement;
                writeDataIslandHydrationData(required<HTMLTemplateElement>(resumed, ':scope > template[data-cem-island]'), snapshot);
                host.remove(); root.append(resumed);
                try {
                    await runtime.whenRenderSettled(resumed);
                    expect(required<HTMLDialogElement>(resumed, 'dialog').open).toBe(false);
                    expect(resumed.dataset.state).toBe('closed');
                    expect(runtime.diagnosticsFor(resumed).filter(diagnostic => diagnostic.severity === 'error')).toEqual([]);
                } finally { resumed.remove(); }
            } finally { host.remove(); declaration.remove(); scope.dispose(); }
        }
    },
};

export const DerivedBodySurvivesReconciliation: Story = {
    render: () => '<section></section>',
    play: ({ canvasElement }) => {
        const root = required(canvasElement, 'section');
        const bounds = { start: document.createComment('start'), end: document.createComment('end') }; root.append(bounds.start, bounds.end);
        const plan = (value: string): RenderPlan => ({ producedTag: 'fixture', instanceId: 'fixture', templateArtifactId: 'body', dataRevision: value, outputTarget: 'light-dom', scopePolicyStamp: 'test', nodes: [
            { kind: 'element', namespace: null, tag: 'dialog', renderNodeId: 'owner', attributes: [], children: [
                { kind: 'element', namespace: null, tag: 'template', renderNodeId: 'body', attributes: [{ name: 'slot', value: 'body' }], children: [
                    { kind: 'element', namespace: null, tag: 'input', renderNodeId: 'field', attributes: [{ name: 'value', value }], children: [] },
                ] },
            ] },
        ] });
        const before = plan('Initial'); applyRenderPlanToRange(bounds, before, document);
        const owner = required<HTMLDialogElement>(root, 'dialog'), controller = connectCemNativeSurface(owner);
        const options = { preserveNode: (node: Node) => ownsCemSurfaceBodyNode(owner, node), preserveElementAttribute: (_current: Element, _desired: Element, attr: Attr) => ['open', 'style', 'data-state', 'data-placement'].includes(attr.name) };
        try {
            controller.open(); const input = required<HTMLInputElement>(owner, 'input'); input.value = 'Draft'; input.focus();
            const changed = plan('Changed');
            expect(applyPatchFramesToRange(bounds, diffRenderPlansToPatchFrames(before, changed), renderPlanIdentity(changed), document, options).status).toBe('applied');
            expect(input.value).toBe('Draft'); expect(required<HTMLInputElement>(required<HTMLTemplateElement>(owner, 'template').content, 'input').value).toBe('Changed');
            applyRenderPlanToRange(bounds, plan('Reconciled'), document, options);
            expect(owner.querySelector('input')).toBe(input); expect(input.value).toBe('Draft'); expect(document.activeElement).toBe(input);
        } finally { controller.disconnect(); }
    },
};

export const NestedNativeReadinessCancellation: Story = {
    render: () => '<section><button>Launch</button></section>',
    play: async ({ canvasElement }) => {
        const root = required(canvasElement, 'section'), source = required(root, 'button');
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID();
            const tag = `cem-prepared-surface-${suffix}`, childTag = `cem-prepared-body-${suffix}`, declarationTag = `declaration-${suffix}`;
            const pending: ReturnType<typeof deferred>[] = [];
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback'); } } : {}),
                resolveModuleUrl: async () => { const done = deferred(); pending.push(done); await done.promise; return 'https://example.test/ready'; },
            }); runtime.install(window);
            const declarations: HTMLElement[] = [];
            const declare = (name: string, text: string, capability?: string) => {
                const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', name);
                if (capability) declaration.setAttribute('capability', capability);
                const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = text;
                declaration.append(template); declarations.push(declaration); root.append(declaration); runtime.registerDeclaration(declaration);
            };
            declare(childTag, '{module-url @slice=ready @src=fixture:body}{input @value=Ready @autofocus=true}');
            declare(tag, `{dialog @part=surface @aria-label=Prepared | {template @slot=body | {${childTag}}}}`, 'native-surface');
            const host = document.createElement(tag); host.setAttribute('materialize', 'dispose'); root.append(host);
            try {
                await runtime.whenRenderSettled(host); const owner = required<HTMLDialogElement>(host, 'dialog');
                const controller = connectCemNativeSurface(owner, { host }); source.focus();
                controller.open({ source }); await waitFor(() => expect(pending.length).toBe(1));
                expect(owner.open).toBe(false); expect(host.dataset.state).toBe('preparing'); expect(document.activeElement).toBe(source);
                const stale = required(owner, childTag); controller.requestClose(); expect(stale.isConnected).toBe(false);
                pending[0].resolve(); await runtime.whenRenderSettled(stale); expect(owner.open).toBe(false);
                controller.open({ source }); await waitFor(() => expect(owner.open).toBe(true));
                expect(document.activeElement).toBe(required(owner, 'input'));
                controller.requestClose(); await waitFor(() => expect(owner.querySelector(childTag)).toBeNull());
            } finally { pending.forEach(done => done.resolve()); host.remove(); declarations.forEach(node => node.remove()); scope.dispose(); }
        }
    },
};

export const NestedNativeExitBeforeAncestorDisposal: Story = {
    render: () => '<section><button>Launch</button><dialog materialize="dispose" aria-label="Parent"><template slot="body"><dialog materialize="dispose" aria-label="Child"><template slot="body"><input></template></dialog></template></dialog></section>',
    play: async ({ canvasElement }) => {
        const source = required(canvasElement, 'button'), owner = required<HTMLDialogElement>(canvasElement, 'dialog');
        const parent = connectCemNativeSurface(owner); source.focus(); parent.open({ source });
        const childOwner = required<HTMLDialogElement>(owner, 'dialog'), child = connectCemNativeSurface(childOwner);
        const order: string[] = [];
        owner.addEventListener('cem-close', event => { if (event.target === owner) order.push('parent'); });
        childOwner.addEventListener('cem-close', () => order.push('child'));
        try {
            child.open(); const input = required<HTMLInputElement>(childOwner, 'input'); input.focus();
            const exit = input.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 100_000 });
            parent.requestClose();
            expect(order).toEqual(['child', 'parent']); expect(childOwner.open).toBe(false); expect(owner.open).toBe(false);
            expect(document.activeElement).toBe(source); expect(input.isConnected).toBe(true);
            exit.finish(); await waitFor(() => expect(childOwner.isConnected).toBe(false));
            const replacement = connectCemNativeSurface(childOwner); expect(replacement).not.toBe(child); replacement.disconnect();
        } finally { child.disconnect(); parent.disconnect(); }
    },
};
