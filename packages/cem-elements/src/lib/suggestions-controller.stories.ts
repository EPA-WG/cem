import { createCemPlatformCloseCoordinator, type CemPlatformCloseCoordinator } from './platform-close.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { nativeTap, nativeTouchGesture, nativePenDrag, nativeInputPoint, nativeWheel } from '../../.storybook/native-input.js';
import { cemComponentDeclarationSource, loadCemComponent, whenCemRendered } from '../../.storybook/preview.js';
import { connectCemNativeSurface } from './native-surface.js';
import { captureCemSurfaceLifetime, type CemSurfaceLifetime } from './surface-session.js';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { CemSuggestionsPlacementCoordinator, type CemSuggestionsPlacementLease } from './suggestions-placements.js';
import { cemSuggestionsControllerFor } from './suggestions-capability.js';
import { connectCemSuggestionsController, type CemSuggestionsControllerOptions, type CemSuggestionsFeedback } from './suggestions-controller.js';
import { getCemEditorProvider } from './form-control-capability.js';
import type { CemNativeSuggestionsPublication, CemNativeSuggestionsBinding } from './native-suggestions-publication.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixture source capture uses the adopted native CEMB boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Suggestions Controller', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
async function fixture(root: HTMLElement, fallback: boolean, declarative = false, productionField?: string, platformClose?: CemPlatformCloseCoordinator) {
    await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
    if (productionField) await loadCemComponent(productionField);
    const suffix = crypto.randomUUID(), declarationTag = `controller-declaration-${suffix}`, tag = productionField ?? `controller-field-${suffix}`;
    let capabilityOptions: CemSuggestionsControllerOptions | undefined;
    let renderPublication: CemNativeSuggestionsPublication | undefined;
    const runtime = new CemElementRuntime({ declarationTag,
        nativeSuggestionsInputs(instance, snapshot) {
            const current = renderPublication;
            return declarative && instance === host && current ? [current.bind({ instanceId: snapshot.instanceId,
                scopePolicyStamp: snapshot.scopePolicyStamp, revision: snapshot.dataRevision,
                current: () => instance.isConnected && renderPublication === current })] : [];
        }, suggestionsControllerInputs: instance => instance === host ? capabilityOptions : undefined, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback fixture'); } } : {}) }); runtime.install(window);
    const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', 'form-control');
    const template = document.createElement('template'); template.type = 'text/cem-ml';
    template.textContent = '{input @part=control @form="" @type=text @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}';
    declaration.append(template); if (!productionField) { root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration); }
    let parentDeclaration: HTMLElement | undefined;
    const parentTag = `controller-attachment-${suffix}`;
    if (declarative) {
        parentDeclaration = document.createElement(declarationTag); parentDeclaration.setAttribute('tag', parentTag); parentDeclaration.setAttribute('capability', 'suggestions');
        const parentTemplate = document.createElement('template'); parentTemplate.type = 'text/cem-ml';
        parentTemplate.textContent = '{slot @name=editor}{div @part=surface @role=listbox @popover=manual @aria-label=Suggestions | {cem:for-each @select="datadom.slices.suggestions.children" @as=row | {div @role=option @suggestion-row={#row} @hidden={if row.dom:attribute("hidden").value {true} else {null}} @aria-disabled={row.dom:attribute("disabled").value} | {$row.dom:attribute("label").value}}}}{slot @name=outside}';
        if (productionField) {
            const source = document.createElement('template'); source.innerHTML = await cemComponentDeclarationSource('cem-suggestions');
            const authored = source.content.querySelector('template');
            if (!authored) throw new Error('Missing production suggestions template');
            parentTemplate.textContent = authored.content.textContent;
        }
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
    if (productionField) { const payload = document.createElement('template'); payload.content.append(field, outside); host.replaceChildren(payload); }
    form.append(host); root.append(form);
    if (declarative) {
        await runtime.whenRenderSettled(host);
        field = host.querySelector(tag) as HTMLElement & { value: string }; panel = host.querySelector('[part=surface]') as HTMLElement;
        if (!field || !panel) throw new Error(`Missing declarative fixture output: ${runtime.diagnosticsFor(host).map(d => d.message).join('; ')}`);
        panel.style.cssText = 'width:160px;height:80px'; rowElements = [...panel.querySelectorAll<HTMLElement>('[role=option]')];
    }
    if (productionField) await whenCemRendered(field); else await runtime.whenRenderSettled(field); const editor = field.querySelector('input'), provider = getCemEditorProvider(field);
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
    const errors: unknown[] = [], feedbacks: CemSuggestionsFeedback[] = [];
    let failed = false;
    const controllerOptions: CemSuggestionsControllerOptions = { editorHost: field, listbox: panel, platformClose, onError: error => errors.push(error),
        onFeedback: feedback => feedbacks.push(feedback),
        async prepare(query, queryRevision) {
            latest = queryRevision;
            if (delay) await delay;
            if (failed) throw new Error('Fixture source unavailable');
            const next = await session.publishSuggestions({ query, queryRevision }, () => latest === queryRevision);
            let nextBinding: CemNativeSuggestionsBinding | undefined;
            try {
                let mapped: ReturnType<CemElementRuntime['renderedSuggestionsFor']>;
                if (declarative) {
                    renderPublication = next; runtime.refreshElementReferences(host); await runtime.whenRenderSettled(host);
                    mapped = runtime.renderedSuggestionsFor(host);
                    if (!mapped) throw new Error(`Missing committed native row map: ${runtime.diagnosticsFor(host).map(d => d.message).join('; ')}`);
                    nextBinding = mapped.binding; rowElements = mapped.rows.map(row => row.element);
                } else nextBinding = next.bind({ instanceId: 'listbox', scopePolicyStamp: 'listbox', revision: String(queryRevision), current: () => latest === queryRevision });
                const rows = mapped ? mapped.rows.map(row => row.native) : await nextBinding.rows();
                if (latest !== queryRevision) throw new Error('Fixture preparation superseded');
                for (const lease of rowPlacements) lease.dispose(); binding?.release(); await publication?.release();
                publication = next; binding = nextBinding;
                rowPlacements = rows.map((row, i) => {
                    if (!declarative) rowElements[i].hidden = !row.eligible;
                    const registered = placements.register({ producer: 'listbox', revision: String(queryRevision), current: () => latest === queryRevision && (!mapped || mapped.current()),
                        kind: 'row', element: rowElements[i], row });
                    placements.grant('field', registered, ['aria-activedescendant']); return registered;
                });
                return placements.prepare(nextBinding, editorPlacement, panelPlacement, rowPlacements);
            } catch (error) { nextBinding?.release(); await next.release(); throw error; }
        },
    };
    if (declarative) { capabilityOptions = controllerOptions; runtime.setInstanceSlices(host, { fixtureTick: 1 });
        await runtime.whenRenderSettled(host); await waitFor(() => expect(cemSuggestionsControllerFor(host)).toBeDefined()); }
    const controller = declarative ? cemSuggestionsControllerFor(host) : connectCemSuggestionsController(host, controllerOptions);
    if (!controller) throw new Error('Missing shared controller');
    const settled = () => waitFor(() => { expect(controller.pending).toBe(false); expect(errors).toEqual([]); });
    const key = (name: string, extra: KeyboardEventInit = {}) => { const event = new KeyboardEvent('keydown', { key: name, code: name, bubbles: true, cancelable: true, ...extra }); editor.dispatchEvent(event); return event; };
    const up = (name: string) => editor.dispatchEvent(new KeyboardEvent('keyup', { key: name, code: name, bubbles: true }));
    return { host, field, form, panel, editor, provider, controller, feedbacks, errors, get rowElements() { return rowElements; }, outside, settled, key, up,
        async admitAncestors(ancestors: readonly CemSurfaceLifetime[]) {
            capabilityOptions = { ...controllerOptions, ancestors }; runtime.setInstanceSlices(host, { fixtureTick: crypto.randomUUID() });
            await runtime.whenRenderSettled(host);
            await waitFor(() => expect(cemSuggestionsControllerFor(host)).toBeDefined());
            const admitted = cemSuggestionsControllerFor(host);
            if (!admitted) throw new Error('Missing admitted controller');
            return admitted;
        },
        fail(value: boolean) { failed = value; },
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
                const originalFirst = f.rowElements[0];
                f.editor.value = 'Second'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.settled();
                expect(f.field.querySelector('input')).toBe(f.editor); expect(f.rowElements[0]).toBe(originalFirst);
                expect(f.rowElements[0].hidden).toBe(true); expect(f.controller.active).toBeUndefined();
                expect(f.key('ArrowDown').defaultPrevented).toBe(true);
                expect(f.editor.getAttribute('aria-activedescendant')).toBe(f.rowElements[1].id);
                expect(f.key('Enter').defaultPrevented).toBe(true); f.up('Enter'); await f.settled();
                expect(f.field.value).toBe('same'); expect(new FormData(f.form).get('choice')).toBe('same');
                expect(f.controller.committed).toBeDefined();
                f.host.setAttribute('editor-for', '@missing');
                await waitFor(() => expect(cemSuggestionsControllerFor(f.host)).toBeUndefined());
                expect(f.editor.hasAttribute('aria-controls')).toBe(false); expect(f.panel.matches(':popover-open')).toBe(false);
            } finally { await f.cleanup(); }
        }
    },
};

async function localFixture(root: HTMLElement, fallback: boolean, enabled = true, empty = false, customLabels = false) {
    const suffix = crypto.randomUUID(), declarationTag = `local-declaration-${suffix}`, fieldTag = `local-field-${suffix}`, tag = `local-suggestions-${suffix}`;
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationTag, declarationScope: scope, localSuggestions: enabled,
        ...(fallback ? { processingWorkerFactory: () => { throw new Error('local fallback'); } } : {}) }); runtime.install(window);
    const declarations: HTMLElement[] = [];
    for (const [name, capability, source] of [
        [fieldTag, 'form-control', '{input @part=control @form="" @type=text @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}'],
        [tag, 'suggestions', '{slot @name=editor}{slot @name=options}{slot @name=option}{slot @name=group-label}{div @part=surface @role=listbox @popover=manual @aria-label=Suggestions | {cem:for-each @select="datadom.slices.suggestions.children" @as=row | {div @role=option @suggestion-row={#row} @hidden={if row.dom:attribute("hidden").value {true} else {null}} | {$row.labelContent}}}}{div @part=status @role=status @aria-live=polite @aria-atomic=true @pending-message="Loading suggestions." @failure-message="Suggestions are unavailable." @empty-message="No suggestions available." @single-message="1 suggestion available." @multiple-message="%count suggestions available."}'],
    ]) {
        const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', name); declaration.setAttribute('capability', capability);
        const template = document.createElement('template'); template.type = 'text/cem-ml'; template.textContent = source; declaration.append(template);
        root.append(declaration); runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration); declarations.push(declaration);
    }
    const form = document.createElement('form'), host = document.createElement(tag);
    let field = document.createElement(fieldTag) as HTMLElement & { value: string };
    field.slot = 'editor'; field.setAttribute('name', 'choice');
    const options = document.createElement('template'); options.slot = 'options'; options.innerHTML = '<option value="a">Alpha</option><option value="b">Beta</option>';
    const payload = document.createElement('template'); payload.content.append(field);
    if (customLabels) { const label = document.createElement('template'); label.slot = 'option'; label.setAttribute('type', 'text/cem-ml'); label.setAttribute('xmlns:s', 'http://www.w3.org/2000/svg'); label.content.append(document.createTextNode('{strong | {$suggestion.dom:attribute("label").value}}{s:svg | {s:path @d="M0 0"}}')); payload.content.append(label); }
     if (!empty) payload.content.append(options); host.append(payload); form.append(host); root.append(form); await runtime.whenRenderSettled(host); field = host.querySelector(fieldTag) as HTMLElement & { value: string }; if (!field) throw new Error(`Local fixture rendering failed: ${JSON.stringify(runtime.diagnosticsFor(host))}`); await runtime.whenRenderSettled(field);
    const editor = field.querySelector('input'); if (!editor) throw new Error('Missing local editor'); editor.style.width = '150px';
    const ready = async () => { await waitFor(() => { const controller = cemSuggestionsControllerFor(host); expect(controller).toBeDefined(); expect(controller?.pending).toBe(false); expect(runtime.renderedSuggestionsFor(host)?.current()).toBe(true); }); expect(runtime.localSuggestionsEnvironmentFor(host)?.owner.mode).toBe(fallback ? 'main-thread' : 'worker'); };
    return { runtime, host, field, editor, form, options: host.querySelector('template[slot=options]') as HTMLTemplateElement, ready,
        async cleanup() { form.remove(); for (const declaration of declarations) declaration.remove(); await Promise.resolve(); scope.dispose(); },
    };
}
export const LocalOptInRetainsSourcesAndRevokesAuthority: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await localFixture(canvasElement, fallback, false);
            try {
                expect(cemSuggestionsControllerFor(f.host)).toBeUndefined();
                f.runtime.setLocalSuggestionsEnabled(true); await f.ready();
                const first = f.runtime.renderedSuggestionsFor(f.host); expect(first?.rows).toHaveLength(2);
                const controller = cemSuggestionsControllerFor(f.host), competing = getCemEditorProvider(f.field)?.lease({});
                if (!controller || !competing) throw new Error('Missing local lease fixture');
                expect(competing.valid).toBe(false); expect(competing.current).toBe(true);
                f.host.setAttribute('filter', 'prefix'); await new Promise(resolve => setTimeout(resolve));
                expect(cemSuggestionsControllerFor(f.host)).toBe(controller); expect(controller.retained).toBe(true);
                expect(competing.valid).toBe(false); expect(f.editor.hasAttribute('aria-controls')).toBe(false);
                competing.release(); f.host.removeAttribute('filter'); await f.ready();
                f.editor.focus(); await waitFor(() => expect(cemSuggestionsControllerFor(f.host)?.visible).toBe(true));
                f.editor.value = 'Beta'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true, inputType: 'insertText' }));
                await f.ready(); const filtered = f.runtime.renderedSuggestionsFor(f.host);
                expect(filtered?.rows[0].native.source).toBe(first?.rows[0].native.source);
                expect(filtered?.rows[0].native.eligible).toBe(false); expect(filtered?.rows[1].native.eligible).toBe(true);
                f.runtime.setLocalSuggestionsEnabled(false);
                expect(cemSuggestionsControllerFor(f.host)).toBeUndefined(); expect(filtered?.current()).toBe(false);
                expect(f.editor.hasAttribute('aria-controls')).toBe(false); expect(f.editor.hasAttribute('aria-activedescendant')).toBe(false);
                f.runtime.setLocalSuggestionsEnabled(true); await f.ready();
                expect(cemSuggestionsControllerFor(f.host)?.visible).toBe(false);
                expect(f.runtime.renderedSuggestionsFor(f.host)?.rows[0].native.source).not.toBe(first?.rows[0].native.source);
                const before = f.runtime.renderedSuggestionsFor(f.host);
                const source = f.runtime.localSuggestionsEnvironmentFor(f.host)?.optionsSources[0] as HTMLTemplateElement;
                source.innerHTML = '<option value="c">Gamma</option>'; f.runtime.refreshElementReferences(f.host); await waitFor(() => expect(before?.current()).toBe(false));
                await f.ready(); expect(f.runtime.renderedSuggestionsFor(f.host)?.rows).toHaveLength(1);
                expect(f.runtime.renderedSuggestionsFor(f.host)?.rows[0].native.value).toBe('c');
                f.form.remove(); await Promise.resolve(); expect(cemSuggestionsControllerFor(f.host)).toBeUndefined();
                canvasElement.append(f.form); await f.ready(); expect(cemSuggestionsControllerFor(f.host)?.visible).toBe(false);
            } finally { await f.cleanup(); }
        }
    },
};
export const LocalOptInDoesNotAdmitExplicitForeignInputs: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        const f = await localFixture(canvasElement, true);
        try {
            await f.ready(); const ready = f.runtime.renderedSuggestionsFor(f.host);
            f.host.setAttribute('options-state', 'pending'); expect(ready?.current()).toBe(false);
            await waitFor(() => { expect(cemSuggestionsControllerFor(f.host)?.pending).toBe(false); expect(f.editor.hasAttribute('aria-controls')).toBe(false); });
            f.host.setAttribute('options-state', 'ready'); await f.ready();
            expect(f.runtime.renderedSuggestionsFor(f.host)?.rows[0].native.source).toBe(ready?.rows[0].native.source);
            f.host.setAttribute('options', 'foreign');
            await waitFor(() => expect(cemSuggestionsControllerFor(f.host)).toBeUndefined());
            expect(f.editor.hasAttribute('aria-controls')).toBe(false);
            f.host.removeAttribute('options'); await f.ready();
            const options = f.options.cloneNode(true); f.host.append(options);
            await waitFor(() => expect(cemSuggestionsControllerFor(f.host)).toBeUndefined());
            options.remove(); await f.ready();
            f.runtime.setLocalSuggestionsEnabled(false); f.runtime.setLocalSuggestionsEnabled(true); f.runtime.setLocalSuggestionsEnabled(false);
            await new Promise(resolve => setTimeout(resolve, 100));
            expect(cemSuggestionsControllerFor(f.host)).toBeUndefined(); expect(f.editor.hasAttribute('aria-controls')).toBe(false);
        } finally { await f.cleanup(); }
    },
};

export const LocalEmptySourcesAndForeignEditorsRemainBounded: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const local = await localFixture(canvasElement, fallback, true, true);
            const foreign = await localFixture(canvasElement, fallback, false);
            try {
                await local.ready(); expect(local.runtime.renderedSuggestionsFor(local.host)?.rows).toHaveLength(0);
                local.editor.focus(); expect(cemSuggestionsControllerFor(local.host)?.visible).toBe(false);
                local.host.insertBefore(foreign.field, local.field); local.field.remove();
                await waitFor(() => expect(cemSuggestionsControllerFor(local.host)).toBeUndefined());
                expect(foreign.editor.hasAttribute('aria-controls')).toBe(false);
                expect(foreign.runtime.renderedSuggestionsFor(foreign.host)).toBeUndefined();
            } finally { await local.cleanup(); await foreign.cleanup(); }
        }
    },
};
export const NativeLocalImportRejectsBoundsAndGrantInjection: Story = {
    render: () => '<section></section>',
    play: async () => {
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), owner = cemProcessingHostForScope(scope, { workerScriptUrl,
                ...(fallback ? { workerFactory: () => { throw new Error('import fallback'); } } : {}) });
            const input = { action: 'prepare' as const, adapter: 'suggestions-v1' as const,
                handle: { sessionKey: crypto.randomUUID(), instanceId: 'import', sourceRevision: '1', scopePolicyStamp: 'import' },
                sources: { kind: 'cem-native-session-import-v1' as const, contentType: 'application/xml' as const,
                    sourceUri: 'memory:local.xml', bytes: new TextEncoder().encode('<options xmlns="http://www.w3.org/1999/xhtml"><option value="a">Alpha</option></options>').buffer as ArrayBuffer },
                data: {}, select: 'input.children.children', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS };
            try {
                const session = await CemNativeCapabilitySession.prepare(owner, input, () => true);
                expect(session.length).toBe(1); await session.release();
                await expect(CemNativeCapabilitySession.prepare(owner, { ...input, handle: { ...input.handle, sessionKey: crypto.randomUUID() },
                    limits: { ...input.limits, maxBytes: 8 } }, () => true)).rejects.toThrow('Invalid bounded native source import');
                const injected = { ...input.sources, grants: [[0, 1]] };
                await expect(CemNativeCapabilitySession.prepare(owner, { ...input, handle: { ...input.handle, sessionKey: crypto.randomUUID() },
                    sources: injected }, () => true)).rejects.toThrow('Invalid bounded native source import');
            } finally { await owner.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); }
        }
    },
};

export const LocalOwnerLossCannotRecoverFromMarkup: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await localFixture(canvasElement, fallback);
            try {
                await f.ready(); const mapped = f.runtime.renderedSuggestionsFor(f.host);
                const owner = f.runtime.localSuggestionsEnvironmentFor(f.host)?.owner; if (!owner) throw new Error('Missing local owner');
                await owner.dispose({ reason: 'runtime-disposed' }).result;
                expect(mapped?.current()).toBe(false); expect(mapped?.rows[0].native.source.valid).toBe(false);
                expect(cemSuggestionsControllerFor(f.host)).toBeUndefined(); expect(f.editor.hasAttribute('aria-controls')).toBe(false);
                f.host.setAttribute('filter', 'prefix'); await new Promise(resolve => setTimeout(resolve));
                expect(cemSuggestionsControllerFor(f.host)).toBeUndefined();
            } finally { await f.cleanup(); }
        }
    },
};

export const FeedbackQualifiesFocusAndFencesLateRequests: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await fixture(canvasElement, fallback);
            try {
                await f.settled(); expect(f.controller.feedback.qualifying).toBe(false);
                f.editor.focus(); expect(f.controller.feedback).toMatchObject({ state: 'ready', eligibleCount: 2, qualifying: true });
                expect(Object.isFrozen(f.controller.feedback)).toBe(true);
                const announcements = f.feedbacks.length;
                f.key('ArrowDown'); f.key('ArrowDown'); expect(f.feedbacks).toHaveLength(announcements);
                const finish = f.block();
                f.editor.value = 'No match'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
                expect(f.controller.feedback).toMatchObject({ state: 'pending', eligibleCount: 0, qualifying: true });
                expect(f.key('Escape').defaultPrevented).toBe(true); f.up('Escape');
                expect(f.controller.feedback.qualifying).toBe(false);
                finish(); await f.settled(); expect(f.controller.feedback).toMatchObject({ state: 'ready', eligibleCount: 0, qualifying: false });
                f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.settled();
                expect(f.controller.feedback).toMatchObject({ state: 'ready', eligibleCount: 0, qualifying: true });
                f.fail(true); f.editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await waitFor(() => expect(f.controller.pending).toBe(false));
                expect(f.controller.feedback).toMatchObject({ state: 'failed', qualifying: true });
                f.outside.focus(); expect(f.controller.feedback.qualifying).toBe(false);
                f.controller.disconnect(); expect(f.controller.feedback).toMatchObject({ state: 'idle', qualifying: false });
                expect(f.feedbacks.at(-1)).toEqual(f.controller.feedback);
            } finally { await f.cleanup(); }
        }
    },
};

export const DeclarativeFeedbackPreservesFieldAndLiveRegion: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await localFixture(canvasElement, fallback);
            try {
                await f.ready(); const status = f.host.querySelector<HTMLElement>('[part=status]'), panel = f.host.querySelector<HTMLElement>('[part=surface]');
                if (!status || !panel) throw new Error('Missing declaration-owned feedback fixture');
                expect(status.textContent).toBe(''); f.editor.focus();
                await waitFor(() => expect(status.textContent).toBe('2 suggestions available.'));
                status.setAttribute('multiple-message', '%count available locally.');
                await waitFor(() => expect(status.textContent).toBe('2 available locally.'));
                status.setAttribute('multiple-message', '%count suggestions available.');
                await waitFor(() => expect(status.textContent).toBe('2 suggestions available.'));
                let mutations = 0; const observer = new MutationObserver(records => { mutations += records.length; }); observer.observe(status, { childList: true, subtree: true, characterData: true });
                const key = (key: string) => f.editor.dispatchEvent(new KeyboardEvent('keydown', { key, code: key, bubbles: true, cancelable: true }));
                key('ArrowDown'); key('ArrowDown'); await new Promise(resolve => setTimeout(resolve)); expect(mutations).toBe(0); observer.disconnect();
                f.host.setAttribute('options-state', 'pending');
                cemSuggestionsControllerFor(f.host)?.reconcile(); // Visibility may notice readiness before observer-driven refresh.
                await waitFor(() => { expect(status.textContent).toBe('Loading suggestions.'); expect(panel.getAttribute('aria-busy')).toBe('true'); });
                expect(f.editor.getAttribute('aria-busy')).toBeNull(); expect(f.editor.disabled).toBe(false);
                f.host.setAttribute('options-error', 'Service offline'); f.host.setAttribute('options-state', 'failed');
                await waitFor(() => { expect(status.textContent).toBe('Service offline'); expect(panel.hasAttribute('aria-busy')).toBe(false); });

                f.host.setAttribute('options-state', 'ready'); await f.ready();
                f.editor.value = 'Beta'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.ready();
                await waitFor(() => expect(status.textContent).toBe('1 suggestion available.'));
                f.editor.value = 'none'; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.ready();
                await waitFor(() => expect(status.textContent).toBe('No suggestions available.'));
                f.editor.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true })); expect(status.textContent).toBe('');
                f.editor.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })); await f.ready();
                await waitFor(() => expect(status.textContent).toBe('No suggestions available.'));
                key('Escape'); expect(status.textContent).toBe('');
                f.editor.dispatchEvent(new KeyboardEvent('keyup', { key: 'Escape', code: 'Escape', bubbles: true }));
                f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.ready();
                const source = f.runtime.localSuggestionsEnvironmentFor(f.host)?.optionsSources[0] as HTMLTemplateElement;
                source.innerHTML = '<option value="a" disabled>Alpha</option><option value="b" disabled>Beta</option>'; await f.ready();
                f.editor.value = ''; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true })); await f.ready();
                await waitFor(() => expect(status.textContent).toBe('No suggestions available.'));
                expect(f.runtime.renderedSuggestionsFor(f.host)?.rows.every(row => !row.native.eligible)).toBe(true);
                status.setAttribute('role', 'alert'); await waitFor(() => expect(status.textContent).toBe(''));
                status.setAttribute('role', 'status'); await waitFor(() => expect(status.textContent).toBe('No suggestions available.'));
                f.field.setAttribute('readonly', ''); await waitFor(() => expect(status.textContent).toBe(''));
                f.field.removeAttribute('readonly'); await f.ready();
                f.runtime.setLocalSuggestionsEnabled(false); expect(status.textContent).toBe(''); expect(panel.hasAttribute('aria-busy')).toBe(false);
            } finally { await f.cleanup(); }
        }
    },
};

export const CapturedLocalLabelsRetainScopeAndInvalidate: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const fallback of [false, true]) {
            const f = await localFixture(canvasElement, fallback, true, false, true);
            try {
                await f.ready();
                expect(f.host.querySelector('[role=option] strong')?.textContent).toBe('Alpha');
                expect(f.host.querySelector('[role=option] svg')?.namespaceURI).toBe('http://www.w3.org/2000/svg');
                const original = f.runtime.localSuggestionsEnvironmentFor(f.host)?.labelSources[0] as HTMLTemplateElement;
                original.content.textContent = '{em | Updated {$suggestion.dom:attribute("label").value}}';
                await waitFor(() => expect(f.host.querySelector('[role=option] em')?.textContent).toBe('Updated Alpha'));
                await f.ready();
                original.content.textContent = '{button | Interactive}';
                await waitFor(() => expect(cemSuggestionsControllerFor(f.host)?.feedback.state).toBe('failed'));
                expect(cemSuggestionsControllerFor(f.host)?.visible).toBe(false);
                expect(f.editor.hasAttribute('aria-controls')).toBe(false);
                original.content.textContent = '{span | Recovered {$suggestion.dom:attribute("label").value}}';
                await waitFor(() => expect(f.host.querySelector('[role=option] span')?.textContent).toBe('Recovered Alpha'));
                await f.ready();
            } finally { await f.cleanup(); }
        }
    },
};

export const NativeTouchPenRowsPreserveFocusAndScrolling: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return; // Chromium input-protocol evidence; physical devices are separate acceptance runs.
        const { cdp } = await import('vitest/browser');
        for (const fallback of [false, true]) {
            const f = await fixture(canvasElement, fallback), driver = cdp();
            const downs: PointerEvent[] = [], values: string[] = [];
            const down = (event: PointerEvent) => downs.push(event);
            const changed = (event: Event) => values.push(event.type);
            f.panel.addEventListener('pointerdown', down); f.editor.addEventListener('input', changed); f.editor.addEventListener('change', changed);
            try {
                for (const pointerType of ['mouse', 'touch', 'pen'] as const) {
                    f.field.value = ''; await f.settled(); f.editor.focus(); f.key('ArrowDown');
                    const row = f.rowElements[0]; await nativeTap(driver, row, pointerType);
                    await waitFor(() => expect(f.editor.value).toBe('same'));
                    const press = downs.at(-1); expect(press?.isTrusted).toBe(true); expect(press?.pointerType).toBe(pointerType);
                    expect(press?.defaultPrevented).toBe(pointerType === 'mouse');
                    expect(f.editor.ownerDocument.activeElement).toBe(f.editor); expect(f.controller.committed).toBeDefined();
                    await f.settled();
                }
                expect(values).toEqual(['input', 'change', 'input', 'change', 'input', 'change']);
                f.field.value = ''; await f.settled(); f.key('ArrowDown');
                f.panel.style.height = '64px'; f.panel.style.overflow = 'auto'; for (const row of f.rowElements) row.style.height = '40px';
                f.controller.reconcile();
                await nativeTouchGesture(driver, f.panel, 'pan');
                expect(f.editor.value).toBe(''); expect(values).toHaveLength(6);
                await nativeWheel(driver, f.panel);
                await waitFor(() => expect(f.panel.scrollTop).toBeGreaterThan(0));
                expect(f.editor.value).toBe(''); expect(f.editor.ownerDocument.activeElement).toBe(f.editor);
                expect(f.controller.visible).toBe(true); expect(values).toHaveLength(6);
                f.panel.scrollTop = 0;
                await nativeTouchGesture(driver, f.rowElements[0], 'cancel');
                expect(f.editor.value).toBe(''); expect(values).toHaveLength(6);
                await nativePenDrag(driver, f.rowElements[0]);
                expect(f.editor.value).toBe(''); expect(values).toHaveLength(6);
                expect(f.editor.ownerDocument.activeElement).toBe(f.editor);
                // A native press cannot retain authority past source loss.
                const point = nativeInputPoint(f.rowElements[0]);
                await driver.send('Emulation.setTouchEmulationEnabled', { enabled: true });
                try {
                    await driver.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ ...point, id: 1 }] });
                    await f.releaseSource();
                    await driver.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
                    expect(f.editor.value).toBe(''); expect(values).toHaveLength(6);
                    expect(f.editor.hasAttribute('aria-controls')).toBe(false);
                } finally { await driver.send('Emulation.setTouchEmulationEnabled', { enabled: false }); }
            } finally {
                f.panel.removeEventListener('pointerdown', down); f.editor.removeEventListener('input', changed); f.editor.removeEventListener('change', changed);
                await f.cleanup();
            }
        }
    },
};

export const ProductionFieldsExternalSourcesAndTrustedAncestorResume: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        for (const field of ['cem-field', 'cem-text-field']) for (const fallback of [false, true]) {
            const dialog = document.createElement('dialog'); dialog.setAttribute('mode', 'modal');
            dialog.setAttribute('aria-label', 'Host task'); canvasElement.append(dialog);
            const parent = connectCemNativeSurface(dialog);
            expect(parent.open()).toBe(true);
            const f = await fixture(dialog, fallback, true, field);
            let child = f.controller;
            try {
                const lifetime = captureCemSurfaceLifetime(dialog);
                if (!lifetime) throw new Error('Missing trusted parent lifetime');
                child = await f.admitAncestors([lifetime]);
                f.editor.focus();
                await waitFor(() => expect(child.visible).toBe(true));
                expect(dialog.matches(':modal')).toBe(true);
                expect(f.host.contains(f.panel)).toBe(true);
                f.key('ArrowDown');
                expect(f.editor.getAttribute('aria-activedescendant')).toBe(f.rowElements[0].id);
                f.key('Enter'); f.up('Enter');
                await waitFor(() => expect(f.field.value).toBe('same'));
                expect(new FormData(f.form).get('choice')).toBe('same');
                expect(f.field.querySelector('input')).toBe(f.editor);
                f.editor.value = ''; f.editor.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await waitFor(() => expect(child.visible).toBe(true));
                expect(f.key('Escape').defaultPrevented).toBe(true);
                expect(child.visible).toBe(false); expect(dialog.open).toBe(true);
                expect(f.key('Escape', { repeat: true }).defaultPrevented).toBe(true);
                expect(dialog.open).toBe(true); f.up('Escape');
                f.key('ArrowDown'); await waitFor(() => expect(child.visible).toBe(true)); f.up('ArrowDown');
                dialog.close(); await waitFor(() => expect(child.visible).toBe(false));
                expect(lifetime.current()).toBe(false);
                expect(parent.open()).toBe(true); f.editor.focus(); f.key('ArrowDown'); f.up('ArrowDown');
                await Promise.resolve(); expect(child.visible).toBe(false);
                const resumed = captureCemSurfaceLifetime(dialog);
                if (!resumed) throw new Error('Missing resumed parent lifetime');
                const previous = child; child = await f.admitAncestors([resumed]);
                expect(child).not.toBe(previous);
                await waitFor(() => expect(child.pending).toBe(false));
                f.editor.focus(); f.key('ArrowDown'); f.up('ArrowDown');
                await waitFor(() => { if (f.errors.length) throw new Error(f.errors.map(String).join('; ')); expect(child.visible).toBe(true); });
                parent.disconnect(); await waitFor(() => expect(child.visible).toBe(false));
                expect(resumed.current()).toBe(false);
                expect(f.editor.hasAttribute('aria-activedescendant')).toBe(false);
            } finally { child.disconnect(); await f.cleanup(); parent.disconnect(); dialog.remove(); }
        }
    },
};


export const PlatformCloseCancelsPendingAndPreservesEditorRouting: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        if (import.meta.env.MODE !== 'test') return;
        const { cdp } = await import('vitest/browser'), driver = cdp();
        type Watcher = EventTarget & { requestClose(): void; destroy(): void };
        const view = window as unknown as { CloseWatcher: new () => Watcher }, Native = view.CloseWatcher;
        expect(Native).toBeDefined();
        const watchers: Watcher[] = [], revokers = new Set<() => void>();
        let immediateRevoke = false;
        view.CloseWatcher = class extends Native { constructor() { super(); watchers.push(this); } };
        const coordinator = createCemPlatformCloseCoordinator(window, () => ({ current: () => true,
            subscribe(fn) { revokers.add(fn); if (immediateRevoke) fn(); return () => { revokers.delete(fn); }; } }));
        const press = async (key: string) => {
            await driver.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key, code: key, windowsVirtualKeyCode: key === 'Escape' ? 27 : 40 });
            await driver.send('Input.dispatchKeyEvent', { type: 'keyUp', key, code: key, windowsVirtualKeyCode: key === 'Escape' ? 27 : 40 });
        };
        try {
            for (const tag of ['cem-field', 'cem-text-field']) {
                const f = await fixture(canvasElement, true, true, tag, coordinator);
                let resume: (() => void) | undefined;
                try {
                    f.editor.focus(); await f.settled(); f.controller.dismiss();
                    immediateRevoke = true; await press('ArrowDown');
                    expect(f.controller.visible).toBe(false); expect(revokers.size).toBe(0); immediateRevoke = false;
                    const before = watchers.length;
                    await press('ArrowDown'); await waitFor(() => expect(f.controller.visible).toBe(true));
                    expect(watchers.length).toBe(before + 1);
                    await driver.send('Input.dispatchKeyEvent', { type: 'rawKeyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
                    expect(f.controller.visible).toBe(false); expect(revokers.size).toBe(0);
                    expect(f.key('Escape', { repeat: true }).defaultPrevented).toBe(true);
                    await driver.send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
                    await press('ArrowDown'); await waitFor(() => expect(f.controller.visible).toBe(true));
                    f.key('Escape', { isComposing: true }); expect(f.controller.visible).toBe(true);
                    watchers.at(-1)?.requestClose(); expect(f.controller.visible).toBe(false);
                    expect(f.editor.value).toBe(''); expect(new FormData(f.form).get('choice')).toBe('');
                    expect(document.activeElement).toBe(f.editor); expect(f.controller.active).toBeUndefined();
                    resume = f.block(); const refresh = f.controller.refresh(); f.controller.dismiss();
                    const pendingBefore = watchers.length; await press('ArrowDown');
                    expect(f.controller.pending).toBe(true); expect(watchers.length).toBe(pendingBefore + 1);
                    watchers.at(-1)?.requestClose(); expect(f.controller.feedback.qualifying).toBe(false);
                    resume(); resume = undefined; await refresh; await f.settled();
                    expect(f.controller.visible).toBe(false); expect(f.controller.active).toBeUndefined();
                    await press('ArrowDown'); await waitFor(() => expect(f.controller.visible).toBe(true));
                    for (const revoke of [...revokers]) revoke();
                    expect(f.controller.visible).toBe(false); expect(revokers.size).toBe(0);
                    expect(f.field.querySelector('input')).toBe(f.editor);
                    await press('ArrowDown'); await waitFor(() => expect(f.controller.visible).toBe(true));
                } finally { resume?.(); await f.cleanup(); expect(revokers.size).toBe(0); }
            }
        } finally { for (const watcher of watchers) watcher.destroy(); view.CloseWatcher = Native; }
    },
};
