import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { CemSuggestionsPlacementCoordinator, type CemSuggestionsPlacementLease } from './suggestions-placements.js';
import { cemSuggestionsControllerFor } from './suggestions-capability.js';
import { connectCemSuggestionsController, type CemSuggestionsControllerOptions } from './suggestions-controller.js';
import { getCemEditorProvider } from './form-control-capability.js';
import type { CemNativeSuggestionsPublication, CemNativeSuggestionsBinding } from './native-suggestions-publication.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixture source capture uses the adopted native CEMB boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Suggestions Controller', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
async function fixture(root: HTMLElement, fallback: boolean, declarative = false) {
    await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
    const suffix = crypto.randomUUID(), declarationTag = `controller-declaration-${suffix}`, tag = `controller-field-${suffix}`;
    let capabilityOptions: CemSuggestionsControllerOptions | undefined;
    const runtime = new CemElementRuntime({ declarationTag, suggestionsControllerInputs: instance => instance === host ? capabilityOptions : undefined, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback fixture'); } } : {}) }); runtime.install(window);
    const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', 'form-control');
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = '{input @part=control @form="" @type=text @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}';
    declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
    let parentDeclaration: HTMLElement | undefined;
    const parentTag = `controller-attachment-${suffix}`;
    if (declarative) {
        parentDeclaration = document.createElement(declarationTag); parentDeclaration.setAttribute('tag', parentTag); parentDeclaration.setAttribute('capability', 'suggestions');
        const parentTemplate = document.createElement('template'); parentTemplate.type = 'text/cem-ml';
        parentTemplate.textContent = '{slot @name=editor}{div @part=surface @role=listbox @popover=manual @aria-label=Suggestions | {div @role=option | First}{div @role=option | Second}{div @role=option | Disabled}}{slot @name=outside}';
        parentDeclaration.append(parentTemplate); root.append(parentDeclaration); runtime.registerDeclaration(parentDeclaration); await runtime.whenDeclarationSettled(parentDeclaration);
    }
    const form = document.createElement('form'), host = document.createElement(declarative ? parentTag : 'section');
    let field = document.createElement(tag) as HTMLElement & { value: string };
    if (declarative) field.setAttribute('slot', 'editor');
    field.setAttribute('name', 'choice'); host.setAttribute('require-selection', '');
    let panel = document.createElement('div'); panel.setAttribute('role', 'listbox'); panel.setAttribute('popover', 'manual');
    panel.style.cssText = 'width:160px;height:80px'; let rowElements = ['First', 'Second', 'Disabled'].map(label => {
        const row = document.createElement('div'); row.setAttribute('role', 'option'); row.textContent = label; panel.append(row); return row;
    });
    const outside = document.createElement('button'); outside.type = 'button'; outside.textContent = 'Outside'; if (declarative) { outside.setAttribute('slot', 'outside'); host.append(field, outside); } else host.append(field, panel, outside);
    form.append(host); root.append(form);
    if (declarative) {
        await runtime.whenRenderSettled(host);
        field = host.querySelector(tag) as HTMLElement & { value: string }; panel = host.querySelector('[part=surface]') as HTMLElement;
        if (!field || !panel) throw new Error('Missing declarative fixture output');
        panel.style.cssText = 'width:160px;height:80px'; rowElements = [...panel.querySelectorAll<HTMLElement>('[role=option]')];
    }
    await runtime.whenRenderSettled(field); const editor = field.querySelector('input'), provider = getCemEditorProvider(field);
    if (!editor || !provider) throw new Error('Missing fixture editor'); editor.style.width = '150px';
    const scope = createCemDeclarationScope({ document }), owner = cemProcessingHostForScope(scope, { workerScriptUrl,
        ...(fallback ? { workerFactory: () => { throw new Error('fallback fixture'); } } : {}) });
    const source = wasm.parseReferenceSource(new TextEncoder().encode('{cem-option @value=same | First}{cem-option @value=same | Second}{cem-option @value=no @disabled | Disabled}'), 'text/cem-ml', 'memory:controller.cem', '');
    const bundle = wasm.exportReferenceReloadBundle(source, '').slice().buffer as ArrayBuffer; wasm.disposeReferenceSource(source);
    const session = await CemNativeCapabilitySession.prepare(owner, { action: 'prepare', adapter: 'suggestions-v1',
        handle: { sessionKey: suffix, instanceId: 'source', scopePolicyStamp: 'source', sourceRevision: '1' },
        sources: { kind: 'cem-native-session-sources-v1', requesting: 0, sources: [{ bundle, primarySourceId: 1, context: true }], bindings: [], grants: [] },
        data: {}, select: 'input.children', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS }, () => true);
    const placements = new CemSuggestionsPlacementCoordinator(document);
    const editorPlacement = placements.register({ producer: 'field', revision: '1', current: () => field.isConnected, kind: 'editor', element: field });
    const panelPlacement = placements.register({ producer: 'listbox', revision: '1', current: () => panel.isConnected, kind: 'listbox', element: panel });
    placements.grant('listbox', editorPlacement, ['editor-for']); placements.grant('field', panelPlacement, ['aria-controls']);
    let publication: CemNativeSuggestionsPublication | undefined, binding: CemNativeSuggestionsBinding | undefined, rowPlacements: CemSuggestionsPlacementLease[] = [];
    let latest = 0, delay: Promise<void> | undefined;
    const errors: unknown[] = [];
    const controllerOptions: CemSuggestionsControllerOptions = { editorHost: field, listbox: panel, onError: error => errors.push(error),
        async prepare(query, queryRevision) {
            latest = queryRevision;
            if (delay) await delay;
            const next = await session.publishSuggestions({ query, queryRevision }, () => latest === queryRevision);
            const nextBinding = next.bind({ instanceId: 'listbox', scopePolicyStamp: 'listbox', revision: String(queryRevision), current: () => latest === queryRevision });
            try {
                const rows = await nextBinding.rows();
                if (latest !== queryRevision) throw new Error('Fixture preparation superseded');
                for (const lease of rowPlacements) lease.dispose(); binding?.release(); await publication?.release();
                publication = next; binding = nextBinding;
                rowPlacements = rows.map((row, i) => {
                    rowElements[i].hidden = !row.eligible;
                    const registered = placements.register({ producer: 'listbox', revision: String(queryRevision), current: () => latest === queryRevision,
                        kind: 'row', element: rowElements[i], row });
                    placements.grant('field', registered, ['aria-activedescendant']); return registered;
                });
                return placements.prepare(nextBinding, editorPlacement, panelPlacement, rowPlacements);
            } catch (error) { nextBinding.release(); await next.release(); throw error; }
        },
    };
    if (declarative) { capabilityOptions = controllerOptions; runtime.setInstanceSlices(host, { fixtureTick: 1 });
        await runtime.whenRenderSettled(host); await waitFor(() => expect(cemSuggestionsControllerFor(host)).toBeDefined()); }
    const controller = declarative ? cemSuggestionsControllerFor(host) : connectCemSuggestionsController(host, controllerOptions);
    if (!controller) throw new Error('Missing shared controller');
    const settled = () => waitFor(() => { expect(controller.pending).toBe(false); expect(errors).toEqual([]); });
    const key = (name: string, extra: KeyboardEventInit = {}) => { const event = new KeyboardEvent('keydown', { key: name, code: name, bubbles: true, cancelable: true, ...extra }); editor.dispatchEvent(event); return event; };
    const up = (name: string) => editor.dispatchEvent(new KeyboardEvent('keyup', { key: name, code: name, bubbles: true }));
    return { host, field, form, panel, editor, provider, controller, rowElements, outside, settled, key, up,
        releaseSource: () => session.release(),
        block() { let finish!: () => void; delay = new Promise(resolve => { finish = resolve; }); return () => { delay = undefined; finish(); }; },
        async cleanup() { controller.disconnect(); binding?.release(); await publication?.release(); placements.dispose(); await session.release();
            await owner.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); form.remove(); declaration.remove(); parentDeclaration?.remove(); },
    };
}
export const KeyboardCommitsPreserveNativeOwnershipAndProvenance: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await fixture(canvasElement, fallback);
            try {
                await f.settled(); expect(f.controller.visible).toBe(false);
                f.editor.focus(); await f.settled(); expect(f.controller.visible).toBe(true); expect(f.controller.active).toBeUndefined();
                expect(f.key('ArrowDown').defaultPrevented).toBe(true); const first = f.controller.active;
                expect(first).toBeDefined(); expect(f.editor.value).toBe(''); expect(f.editor.getAttribute('aria-activedescendant')).toBe(f.rowElements[0].id);
                f.key('ArrowDown'); const second = f.controller.active; expect(second).not.toBe(first);
                f.key('ArrowDown'); expect(f.controller.active).toBe(second); f.key('ArrowUp'); expect(f.controller.active).toBe(first);
                const events: string[] = [];
                f.editor.addEventListener('input', () => { events.push('input'); expect(f.field.value).toBe('same'); expect(new FormData(f.form).get('choice')).toBe('same'); expect(f.controller.visible).toBe(false); expect(f.controller.committed).toBe(first); });
                f.editor.addEventListener('change', () => events.push('change'));
                expect(f.key('Enter').defaultPrevented).toBe(true); expect(events).toEqual(['input', 'change']); f.up('Enter');
                await f.settled(); expect(f.controller.committed).toBe(first); expect(f.controller.visible).toBe(false);
                f.field.value = 'same'; expect(f.controller.committed).toBeUndefined(); expect(f.form.checkValidity()).toBe(false); await f.settled();
                f.form.reset(); expect(f.editor.value).toBe(''); expect(f.controller.committed).toBeUndefined(); expect(f.form.checkValidity()).toBe(true);
                await f.settled(); f.key('ArrowDown'); expect(f.controller.visible).toBe(true);
                const veto = (event: Event) => event.preventDefault(); f.editor.addEventListener('beforeinput', veto, { once: true });
                expect(f.key('Enter').defaultPrevented).toBe(true); expect(f.editor.value).toBe(''); expect(events).toEqual(['input', 'change']);
                expect(f.controller.visible).toBe(true); f.up('Enter');
                f.key('Escape'); expect(f.controller.visible).toBe(false); expect(f.key('Escape', { repeat: true }).defaultPrevented).toBe(true); f.up('Escape');
                f.key('ArrowUp'); expect(f.controller.active).toBe(second); f.key('Tab'); expect(f.controller.visible).toBe(false); expect(f.editor.value).toBe('');
                f.key('ArrowDown'); f.editor.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                expect(f.controller.visible).toBe(false); expect(f.key('Enter', { isComposing: true }).defaultPrevented).toBe(false);
                f.editor.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })); f.up('Enter');
                await new Promise(resolve => setTimeout(resolve)); await f.settled();
                f.outside.focus(); expect(f.controller.visible).toBe(false);
            } finally { await f.cleanup(); }
        }
    },
};
export const PendingWorkCannotReplayKeysOrRestoreDismissedOpening: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement, false);
        try {
            await f.settled(); const resume = f.block();
            f.editor.focus(); f.editor.value = 'Second'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
            expect(f.controller.pending).toBe(true); expect(f.key('ArrowDown').defaultPrevented).toBe(false);
            expect(f.key('Escape').defaultPrevented).toBe(true); f.up('Escape'); resume(); await f.settled();
            expect(f.controller.visible).toBe(false); expect(f.controller.active).toBeUndefined();
            expect(f.key('ArrowDown').defaultPrevented).toBe(true); const source = f.controller.active; expect(source).toBeDefined();
            f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.settled();
            expect(f.controller.active).toBeUndefined(); expect(f.controller.committed).toBeUndefined();
            f.controller.disconnect(); expect(f.editor.hasAttribute('aria-controls')).toBe(false); expect(f.panel.matches(':popover-open')).toBe(false);
        } finally { await f.cleanup(); }
    },
};

export const NativeMouseActivationAndSourceLossUseTheSameCommitRoute: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await fixture(canvasElement, false);
        try {
            await f.settled(); f.editor.focus(); await f.settled();
            if (import.meta.env.MODE !== 'test') return; // Native pointer focus evidence requires the browser runner.
            const { userEvent } = await import('vitest/browser');
            const events: string[] = []; f.editor.addEventListener('input', () => events.push('input')); f.editor.addEventListener('change', () => events.push('change'));
            await userEvent.click(f.rowElements[1]);
            expect(f.editor.ownerDocument.activeElement).toBe(f.editor); expect(f.editor.value).toBe('same'); expect(events).toEqual(['input', 'change']);
            const selected = f.controller.committed; expect(selected).toBeDefined(); await f.settled();
            f.field.value = ''; await f.settled(); f.key('ArrowDown');
            const row = f.rowElements[0];
            row.dispatchEvent(new PointerEvent('pointerdown', { pointerType: 'mouse', pointerId: 10, isPrimary: true, button: 0, clientX: 1, clientY: 1, bubbles: true, cancelable: true }));
            row.dispatchEvent(new PointerEvent('pointermove', { pointerType: 'mouse', pointerId: 10, clientX: 30, clientY: 1, bubbles: true }));
            row.dispatchEvent(new PointerEvent('pointerup', { pointerType: 'mouse', pointerId: 10, bubbles: true }));
            row.dispatchEvent(new MouseEvent('click', { detail: 1, bubbles: true, cancelable: true }));
            expect(f.editor.value).toBe(''); expect(events).toEqual(['input', 'change']);
            f.editor.addEventListener('beforeinput', () => f.controller.dismiss('listener'), { once: true });
            f.key('ArrowDown'); expect(f.key('Enter').defaultPrevented).toBe(true); f.up('Enter');
            expect(f.editor.value).toBe(''); expect(events).toEqual(['input', 'change']);
            f.key('ArrowDown'); await userEvent.click(f.rowElements[1]); await f.settled();
            expect(f.controller.committed).toBe(selected); expect(f.form.checkValidity()).toBe(true);
            await f.releaseSource(); expect(f.controller.committed).toBeUndefined(); expect(f.editor.value).toBe('same');
            expect(f.form.checkValidity()).toBe(false); expect(f.editor.hasAttribute('aria-controls')).toBe(false);
        } finally { await f.cleanup(); }
    },
};

export const DeclarativeCapabilityRequiresFreshHostAdmission: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await fixture(canvasElement, fallback, true);
            try {
                await f.settled(); expect(f.controller.visible).toBe(false);
                f.editor.focus(); await f.settled(); expect(f.controller.visible).toBe(true);
                f.key('ArrowDown'); expect(f.editor.getAttribute('aria-activedescendant')).toBe(f.rowElements[0].id);
                f.host.setAttribute('editor-for', '@missing');
                await waitFor(() => expect(cemSuggestionsControllerFor(f.host)).toBeUndefined());
                expect(f.editor.hasAttribute('aria-controls')).toBe(false); expect(f.panel.matches(':popover-open')).toBe(false);
            } finally { await f.cleanup(); }
        }
    },
};
