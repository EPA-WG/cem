import { connectCemNativeSurface } from './native-surface.js';
import type { NativeCemValue } from './native-values.js';
import { importNativeCemAttributes } from './native-values.js';
import { checkpointPopupGeometry, createCemSurfaceGeometryLease, releasePopupGeometry } from './popup-controller.js';
import { connectCemStandaloneDatalist } from './standalone-datalist.js';
import { CemSurfaceBody } from './surface-body.js';
import { applyRenderPlanToRange, checkpointRenderedDatalistOrigin, checkpointRenderedTemplateProjection, renderedNativeAttributeBindings, restoreNativeAttributeBindings, type RenderPlan } from './projection.js';
import { connectCemManualListbox } from './manual-listbox.js';
import type { CemCheckpointSealer } from './committed-checkpoint.js';
import type { CheckpointSource } from './checkpoint-interactions.js';
import type { CemNativeSuggestionsPublication } from './native-suggestions-publication.js';
import type { CemSuggestionsControllerOptions } from './suggestions-controller.js';
import { CemSuggestionsPlacementCoordinator, type CemSuggestionsPlacementLease } from './suggestions-placements.js';
import { cemCompositeMenuCheckpoint } from './composite-menu-capability.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, userEvent, waitFor } from 'storybook/test';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { ingestContractVersion } from './disposition.js';
import { CemRepositoryRegistry, type CemRepositoryChange } from './repository.js';
import { cemSuggestionsCheckpoint, cemSuggestionsControllerFor } from './suggestions-capability.js';
import { CemElementPlacementCoordinator } from './element-placement-coordinator.js';
import { elementReferenceFixtureInputs, ELEMENT_REFERENCE_TEMPLATE } from './element-reference-lifecycle.fixtures.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import suggestionsDeclaration from '../../../cem-components/src/components/cem-suggestions/cem-suggestions.xhtml?raw';
import { getCemEditorProvider } from './form-control-capability.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import selectDeclaration from '../../../cem-components/src/components/cem-select/cem-select.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import autocompleteDeclaration from '../../../cem-components/src/components/cem-autocomplete/cem-autocomplete.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import fieldDeclaration from '../../../cem-components/src/components/cem-field/cem-field.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import checkboxDeclaration from '../../../cem-components/src/components/cem-checkbox/cem-checkbox.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import treeDeclaration from '../../../cem-components/src/components/cem-tree/cem-tree.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import tabsDeclaration from '../../../cem-components/src/components/cem-tabs/cem-tabs.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import navDeclaration from '../../../cem-components/src/components/cem-nav/cem-nav.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import menuDeclaration from '../../../cem-components/src/components/cem-menu/cem-menu.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import dropdownDeclaration from '../../../cem-components/src/components/cem-dropdown/cem-dropdown.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import dialogDeclaration from '../../../cem-components/src/components/cem-dialog/cem-dialog.xhtml?raw';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import gridDeclaration from '../../../cem-components/src/components/cem-grid/cem-grid.xhtml?raw';
import { mountCemView } from './host-view.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- production declarative XHTML is fixture input, not a runtime dependency.
import sourceEditorDeclaration from '../../../cem-components/src/components/cem-source-editor/cem-source-editor.xhtml?raw';
import { createCemDeclarationScope } from './declaration-scope.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- real native checkpoint transport acceptance.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Checkpoint Publication', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
type Reader = NonNullable<Exclude<NonNullable<CemElementRuntimeOptions['retainedStylesheets']>, false>['read']>;
const decode = (c: CemCheckpointPublicationCoordinator) => JSON.parse(new TextDecoder().decode(c.current()!.bytes)) as { receipt: { owners: { acceptedEventSequence: number }[] }; owners: { instanceId: string; island: string }[] };
function fixture(root: HTMLElement, fallback: boolean, automatic = false, read?: Reader, options: Partial<CemElementRuntimeOptions> = {}, seal: CemCheckpointSealer = (candidate, receipt) => wasm.sealCommittedCheckpoint(candidate, receipt), shared?: CemCheckpointPublicationCoordinator) {
    const document = root.ownerDocument;
    const suffix = crypto.randomUUID();
    const coordinator = shared ?? new CemCheckpointPublicationCoordinator({ runtimeId: `session-${suffix}`, generation: 1, policyStamp: 'whole-private-island:1',
        allowOwner: () => true, seal, automatic });
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ ...options, declarationTag: `declaration-${suffix}`, declarationScope: scope, checkpointPublication: coordinator,
        moduleUrlRoot: { baseUrl: 'https://example.test/', importMap: {} }, retainedStylesheets: { read },
        ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture selects fallback'); } } : {}) });
    async function create(source: string, mode: 'dom' | 'cem-ml' = 'cem-ml', container = root, payload?: string, capability?: string, attributes: Record<string, string> = {}) {
        const tag = `checkpoint-${crypto.randomUUID()}`;
        const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
        const template = document.createElement('template');
        if (capability) declaration.setAttribute('capability', capability);
        if (mode === 'cem-ml') { template.type = 'text/cem-ml'; template.textContent = source; } else template.innerHTML = source;
        declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const instance = document.createElement(tag);
        for (const [name, value] of Object.entries(attributes)) instance.setAttribute(name, value);
        if (payload) { const inert = document.createElement('template'); inert.innerHTML = payload; instance.append(inert); }
        container.append(instance);
        return instance;
    }
    return { runtime, coordinator, scope, create, dispose() { root.replaceChildren(); scope.dispose(); if (!shared) coordinator.dispose(); } };
}
const source = '{attribute @name=label | Before}{slice @name=typed}{label | Draft {input @name=draft @slice=typed @slice-event=input @slice-value="$target.value"}}{p | {$label}}{output | {$typed}}';
async function init() { await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) }); }
export const AllOwnerPublicationAndImmediateInput: Story = {
    render: () => '<section aria-label="Checkpoint publication fixture"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const parent = await f.create(source); await f.runtime.whenRenderSettled(parent);
                const child = await f.create('{p | Nested owner}', 'cem-ml', parent);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                await f.runtime.whenRenderSettled(child); await f.coordinator.capture();
                expect(decode(f.coordinator).owners).toHaveLength(2);
                const initial = f.coordinator.lastCommitted()!.bytes;
                const input = parent.querySelector('input')!;
                input.value = 'Unsaved é draft'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                expect(f.coordinator.current()).toBeUndefined();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(initial);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                await f.runtime.whenRenderSettled(parent); await f.coordinator.capture();
                const saved = decode(f.coordinator);
                expect(saved.owners.some(owner => owner.island.includes('Unsaved é draft'))).toBe(true);
                expect(saved.owners.every(owner => owner.island.startsWith('<template') && owner.island.includes('cem-island:context-root'))).toBe(true);
                expect(saved.receipt.owners.some(owner => owner.acceptedEventSequence > 0)).toBe(true);
                const ordinal = saved.receipt.owners.find((_owner, index) => saved.owners[index].island.includes('Unsaved é draft'))!.acceptedEventSequence;
                const container = parent.parentElement!; parent.remove();
                expect(f.coordinator.current()).toBeUndefined();
                container.append(parent); await f.runtime.whenRenderSettled(parent);
                if (child.isConnected) await f.runtime.whenRenderSettled(child);
                await f.coordinator.capture();
                expect(decode(f.coordinator).receipt.owners.some(owner => owner.acceptedEventSequence === ordinal)).toBe(true);
                parent.setAttribute('label', 'Mutated before observer delivery');
                expect(f.coordinator.current()).toBeUndefined();
                await f.runtime.whenRenderSettled(parent); await f.coordinator.capture();
                expect(parent.querySelector('p')!.textContent).toBe('Mutated before observer delivery');
                expect(parent.querySelector('input')).toBe(input); expect(input.value).toBe('Unsaved é draft');
                // Parent reconciliation can remove an un-authored child; the
                // coordinator must reflect actual connected owners, not skip one.
                const count = root.querySelectorAll('template[data-cem-island="instance"]').length;
                expect(decode(f.coordinator).owners).toHaveLength(count);
                const beforeProbe = f.coordinator.lastCommitted()!.bytes;
                f.runtime.snapshotInstance(parent);
                expect(f.coordinator.current()).toBeUndefined();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(beforeProbe);
                await f.runtime.whenRenderSettled(parent); await f.coordinator.capture();
                f.coordinator.setPolicy('private:revoked', () => false);
                expect(f.coordinator.lastCommitted()).toBeUndefined();
                parent.setAttribute('label', 'New policy'); await f.runtime.whenRenderSettled(parent);
                if (child.isConnected) { child.setAttribute('title', 'New policy'); await f.runtime.whenRenderSettled(child); }
                await expect(f.coordinator.capture()).rejects.toThrow('privacy');
            } finally { f.dispose(); }
        }
    },
};
export const CssDrainFailureAndAutomaticCapture: Story = {
    render: () => '<section aria-label="Checkpoint CSS barrier fixture"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            let resolve!: (value: { bytes: ArrayBuffer; finalUrl: string; contentType: string }) => void;
            let reads = 0;
            const f = fixture(root, fallback, true, () => { reads++; return new Promise(done => { resolve = done; }); });
            try {
                const instance = await f.create('{attribute @name=label | Before}{p | {$label}}', 'cem-ml', root, '<style>p { --checkpoint-style: old; }</style>');
                await f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(f.coordinator.current()).toBeDefined());
                const prior = f.coordinator.lastCommitted()!.bytes;
                const island = instance.querySelector<HTMLTemplateElement>('template[data-cem-island="instance"]')!;
                const style = island.content.querySelector('style')!; style.textContent = '@import "./held.css";';
                instance.setAttribute('label', 'After');
                expect(f.coordinator.current()).toBeUndefined();
                await waitFor(() => expect(reads).toBe(1));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(prior);
                resolve({ bytes: new TextEncoder().encode('p { --checkpoint-style: ready; }').buffer, finalUrl: 'https://example.test/held.css', contentType: 'text/css' });
                await f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(f.coordinator.current()).toBeDefined());
                expect(instance.querySelector('p')!.textContent).toBe('After');
                expect(getComputedStyle(instance.querySelector('p')!).getPropertyValue('--checkpoint-style').trim()).toBe('ready');
                const good = f.coordinator.lastCommitted()!.bytes;
                style.textContent = '@import "./failure.css";'; instance.setAttribute('label', 'Rejected');
                await waitFor(() => expect(reads).toBe(2));
                resolve({ bytes: new TextEncoder().encode('wrong content type').buffer, finalUrl: 'https://example.test/failure.css', contentType: 'text/html' });
                await f.runtime.whenRenderSettled(instance);
                expect(f.coordinator.current()).toBeUndefined(); expect(f.coordinator.lastCommitted()!.bytes).toEqual(good);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                style.textContent = 'p { --checkpoint-style: repaired; }'; instance.setAttribute('label', 'Repaired');
                await f.runtime.whenRenderSettled(instance);
                await waitFor(() => expect(f.coordinator.current()).toBeDefined());
                expect(instance.querySelector('p')!.textContent).toBe('Repaired');
                instance.remove(); expect(f.coordinator.current()).toBeUndefined();
            } finally { f.dispose(); }
        }
    },
};

export const UnsupportedOwnersBlockTheWholeDomain: Story = {
    render: () => '<section aria-label="Checkpoint completeness refusal fixture"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const plain = await f.create('{p | Portable owner}'); await f.runtime.whenRenderSettled(plain);
                await f.coordinator.capture(); const good = f.coordinator.lastCommitted()!.bytes;
                const unsupported = await f.create('{label | Private control {input @name=draft @type=password}}');
                await f.runtime.whenRenderSettled(unsupported);
                const input = unsupported.querySelector('input')!; input.value = 'Keep this edit';
                input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await f.runtime.whenRenderSettled(unsupported);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(input.value).toBe('Keep this edit'); expect(f.coordinator.current()).toBeUndefined();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(good);
                unsupported.remove(); await f.coordinator.capture();
                for (const attributes of ['@type=file', '@type=PASSWORD', '@autocomplete=CURRENT-PASSWORD']) {
                    const secret = await f.create(`{input ${attributes}}`); await f.runtime.whenRenderSettled(secret);
                    await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                    secret.remove(); await f.coordinator.capture();
                }
                expect(decode(f.coordinator).owners).toHaveLength(1);
                const action = await f.create('{button @host-command=save | Save}'); await f.runtime.whenRenderSettled(action);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                action.remove(); await f.coordinator.capture();
                // Overflow blocks capture but must not break normal rendering.
                expect(() => f.coordinator.register(decode(f.coordinator).owners[0].instanceId, () => '<template/>')).toThrow('owners');
                plain.setAttribute('title', 'Still renders'); await f.runtime.whenRenderSettled(plain);
                expect(plain.querySelector('p')!.textContent).toBe('Portable owner');
                const extra = await f.create('{p | Admission remains ordinary}'); await f.runtime.whenRenderSettled(extra);
                expect(extra.querySelector('p')!.textContent).toBe('Admission remains ordinary');
                expect(f.coordinator.current()).toBeUndefined();
            } finally { f.dispose(); }
        }
    },
};

export const CrossDocumentTemplateTraversalLimits: Story = {
    render: () => '<section aria-label="Cross-document checkpoint fixture"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        const frame = document.createElement('iframe'); root.append(frame);
        const body = frame.contentDocument!.body;
        const f = fixture(body, true);
        try {
            const instance = await f.create('<p>Other document</p>', 'dom', body, '<template><b>Nested inert content</b></template>');
            await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
            expect(decode(f.coordinator).owners[0].island).toContain('Nested inert content');
            const deep = '<template>' + '<div>'.repeat(70) + 'Bounded content' + '</div>'.repeat(70) + '</template>';
            const oversized = await f.create('<p>Deep template owner</p>', 'dom', body, deep);
            await f.runtime.whenRenderSettled(oversized);
            await expect(f.coordinator.capture()).rejects.toThrow('limit');
            expect(f.coordinator.current()).toBeUndefined();
        } finally { f.dispose(); frame.remove(); }
    },
};

export const CompleteNativeControlEditsAndProperties: Story = {
    render: () => '<section aria-label="Portable native control edits"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const instance = await f.create('{input @aria-label=Draft}{input @type=checkbox}{textarea}{select @multiple=true | {option @value=a | A}{option @value=b | B}}');
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                const input = instance.querySelector('input')!, check = instance.querySelector<HTMLInputElement>('input[type=checkbox]')!;
                input.value = 'Unsaved é😀'; input.setSelectionRange(1, 4, 'backward'); input.readOnly = true;
                check.checked = false; check.indeterminate = true; check.disabled = true;
                instance.querySelector('textarea')!.value = 'Multiline\nedit\n';
                const select = instance.querySelector('select')!; select.options[0].selected = true; select.options[1].selected = true;
                input.setCustomValidity('Review this draft');
                expect(f.coordinator.current()).toBeUndefined();
                // A barred control hides its private custom-validity message.
                // Refuse that owner until the complete message is available.
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.readOnly = false; instance.querySelector('textarea')!.readOnly = true;
                expect(f.coordinator.current()).toBeUndefined();
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                const island = decode(f.coordinator).owners[0].island;
                expect(island).toContain('cem-checkpoint-controls/1'); expect(island).toContain('Unsaved é😀');
                expect(island).toContain('version="1.0.0-checkpoint.1"');
                expect(ingestContractVersion('1.0.0-checkpoint.1', '0.2.0', 'application', 'data-snapshot').accept).toBe(false);
                const exported = document.createElement('template'); exported.innerHTML = island;
                const canonical = exported.content.querySelector<HTMLTemplateElement>('template')!.content;
                expect(canonical.querySelector('cem-form\\:form-state')!.textContent).toContain('Unsaved é😀');
                expect(island).toContain('Review this draft'); expect(island).toContain('indeterminate'); expect(island).toContain('backward');
                expect(input.value).toBe('Unsaved é😀'); expect(check.indeterminate).toBe(true);
                expect(check.checked).toBe(false); expect(check.disabled).toBe(true); expect(instance.querySelector('textarea')!.readOnly).toBe(true);
                expect([...select.selectedOptions].map(option => option.value)).toEqual(['a', 'b']);
                input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                // An exported checkpoint cannot silently enter ordinary HTML hydration.
                const copy = document.createElement(instance.localName); copy.innerHTML = decode(f.coordinator).owners[0].island; root.append(copy);
                await f.runtime.whenRenderSettled(copy);
                expect(f.runtime.diagnosticsFor(copy).some(item => item.code === 'cem-element.checkpoint_resume_required')).toBe(true);
            } finally { f.dispose(); }
        }
    },
};
export const SharedSourceEditorPortableState: Story = {
    render: () => '<section aria-label="Portable shared source editor"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        const original = new DOMParser().parseFromString(sourceEditorDeclaration, 'application/xml');
        const originalTemplate = original.querySelector('template') as HTMLTemplateElement;
        const template = originalTemplate.content?.textContent || originalTemplate.textContent!;
        expect(template).toContain('part=control');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const instance = await f.create(template, 'cem-ml', root, undefined, 'plaintext-editor');
                await f.runtime.whenRenderSettled(instance);
                (instance as HTMLElement & { value: string }).value = 'Draft é😀\n';
                await f.runtime.whenRenderSettled(instance);
                await f.coordinator.capture();
                expect(decode(f.coordinator).owners[0].island).toContain('Draft é😀');
                const control = instance.querySelector<HTMLElement>('[contenteditable]')!;
                control.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                control.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                expect((instance as HTMLElement & { value: string }).value).toBe('Draft é😀\n');
            } finally { f.dispose(); }
        }
    },
};
export const HostCommandJournalRecordsOutcomesWithoutReplay: Story = {
    render: () => '<section aria-label="Host command checkpoint journal"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            let release!: () => void, calls = 0;
            let view: ReturnType<typeof mountCemView> | undefined;
            try {
                const instance = await f.create('{button @host-command=save | Save}{button @host-command=fail | Fail}{button @host-command=reject | Reject}');
                view = mountCemView(f.runtime, { root, element: instance, tag: instance.localName,
                    commands: { save: () => { calls++; return new Promise<void>(resolve => { release = resolve; }); },
                        fail: () => { calls++; throw new Error('Uncertain external outcome'); },
                        reject: async () => { calls++; throw new Error('Rejected external outcome'); } } });
                await view.whenSettled(); await f.coordinator.capture();
                instance.querySelector<HTMLButtonElement>('button')!.click(); await view.whenSettled(); await f.coordinator.capture();
                const receipts = () => JSON.parse(new TextDecoder().decode(f.coordinator.current()!.bytes)).receipt.operations as { outcome: string; operationId: string }[];
                expect(receipts().map(item => item.outcome)).toEqual(['unknown']);
                release(); await Promise.resolve(); await f.coordinator.capture();
                expect(receipts().map(item => item.outcome)).toEqual(['acknowledged']);
                instance.querySelectorAll<HTMLButtonElement>('button')[1].click(); await view.whenSettled(); await f.coordinator.capture();
                expect(receipts().map(item => item.outcome)).toEqual(['acknowledged', 'unknown']);
                expect(receipts().every(item => !item.operationId.includes('save') && !item.operationId.includes('fail'))).toBe(true);
                expect(calls).toBe(2); expect(view.errors).toHaveLength(1);
                await f.coordinator.capture(); expect(calls).toBe(2);
                instance.querySelectorAll<HTMLButtonElement>('button')[2].click();
                await Promise.resolve(); await view.whenSettled(); await f.coordinator.capture();
                expect(receipts().map(item => item.outcome)).toEqual(['acknowledged', 'unknown', 'unknown']);
                expect(calls).toBe(3); expect(view.errors).toHaveLength(2);
                instance.querySelector<HTMLButtonElement>('button')!.click(); await view.whenSettled(); await f.coordinator.capture();
                expect(receipts().map(item => item.outcome)).toEqual(['acknowledged', 'unknown', 'unknown', 'unknown']);
                view.dispose(); expect(f.coordinator.current()).toBeUndefined(); release(); await Promise.resolve();
                const releaseAuthority = f.runtime.registerCheckpointCommands(instance);
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                expect(receipts().map(item => item.outcome)).toEqual(['acknowledged', 'unknown', 'unknown', 'unknown']);
                expect(calls).toBe(4); releaseAuthority();
            } finally { view?.dispose(); f.dispose(); }
        }
    },
};
export const PinnedReadOnlyResourcesAndAmbientRefusal: Story = {
    render: () => '<section aria-label="Portable resource disposition"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section'); if (!root) throw new Error('missing fixture');
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            const storageKey = `checkpoint-write-${crypto.randomUUID()}`;
            try {
                const pinned = await f.create('{location-element @slice=page @href="https://example.test/docs?theme=light" @live=false}{p | Fixed URL}');
                await f.runtime.whenRenderSettled(pinned); await f.coordinator.capture();
                const island = decode(f.coordinator).owners[0].island;
                expect(island).toContain('https://example.test/docs'); expect(island).toContain('read-again');
                expect(island).not.toContain('AbortController');
                const ambient = await f.create('{location-element @slice=current @live=true}{p | Ambient URL}');
                await f.runtime.whenRenderSettled(ambient); await f.coordinator.capture();
                expect(decode(f.coordinator).owners.some(owner => owner.island.includes('read-and-subscribe'))).toBe(true);
                ambient.remove(); await f.coordinator.capture();
                const effectOutcomes: string[] = [];
                const operation = f.coordinator.operation.bind(f.coordinator);
                f.coordinator.operation = (instanceId) => {
                    const effect = operation(instanceId); effectOutcomes.push('unknown');
                    return { operationId: effect.operationId, acknowledge: () => { effectOutcomes.push('acknowledged'); effect.acknowledge(); } };
                };
                const writer = await f.create(`{local-storage @slice=draft @key="${storageKey}" @type=text @value=recorded}{p | Storage writer}`);
                await f.runtime.whenRenderSettled(writer); expect(localStorage.getItem(storageKey)).toBe('recorded');
                expect(effectOutcomes.length).toBeGreaterThanOrEqual(2);
                expect(effectOutcomes).toEqual(Array.from({ length: effectOutcomes.length / 2 }, () => ['unknown', 'acknowledged']).flat());
                await f.coordinator.capture();
                const receipt = f.coordinator.current()!.receipt;
                expect(receipt.operations.every(record => record.outcome === 'acknowledged')).toBe(true);
                writer.remove();
                // The external effect belongs to the removed owner and must not
                // vanish just because that owner's resource object was released.
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()).toBeDefined();
                expect(localStorage.getItem(storageKey)).toBe('recorded');
                f.coordinator.retireOperations(receipt.operations.map(record => record.operationId), receipt);
                await f.coordinator.capture(); expect(f.coordinator.current()!.receipt.operations).toHaveLength(0);
            } finally { localStorage.removeItem(storageKey); f.dispose(); }
            const g = fixture(root, fallback);
            const previousUrl = root.ownerDocument.location.href;
            try {
                const base = await g.create('{p | Before rejected URL effect}');
                await g.runtime.whenRenderSettled(base); await g.coordinator.capture();
                const effectOutcomes: string[] = [];
                const operation = g.coordinator.operation.bind(g.coordinator);
                g.coordinator.operation = (instanceId) => {
                    const effect = operation(instanceId); effectOutcomes.push('unknown');
                    return { operationId: effect.operationId, acknowledge: () => { effectOutcomes.push('acknowledged'); effect.acknowledge(); } };
                };
                const writer = await g.create('{location-element @method=history.replaceState @src="https://outside.example.invalid/"}{p | Rejected writer}');
                await g.runtime.whenRenderSettled(writer);
                expect(root.ownerDocument.location.href).toBe(previousUrl);
                expect(g.runtime.diagnosticsFor(writer).some(item => item.code === 'cem-element.location_write_failed')).toBe(true);
                expect(effectOutcomes).toEqual(['unknown']);
                await g.coordinator.capture();
                const receipt = g.coordinator.current()!.receipt;
                expect(receipt.operations.map(record => record.outcome)).toEqual(['unknown']);
                expect(decode(g.coordinator).owners.some(owner => owner.island.includes('suppress-until-new-input'))).toBe(true);
                writer.remove(); await expect(g.coordinator.capture()).rejects.toThrow('unavailable');
                expect(g.coordinator.lastCommitted()).toBeDefined();
                g.coordinator.reconcileOperation(receipt.operations[0].operationId, 'not-applied', receipt);
                g.coordinator.retireOperations([receipt.operations[0].operationId], receipt);
                await g.coordinator.capture(); expect(g.coordinator.current()!.receipt.operations).toHaveLength(0);
            } finally { g.dispose(); }
        }
    },
};

function declarationTemplate(source: string): string {
    const template = new DOMParser().parseFromString(source, 'application/xml').querySelector('template') as HTMLTemplateElement;
    return template.content?.textContent || template.textContent!;
}
function islandContent(island: string): DocumentFragment {
    return (new DOMParser().parseFromString(island, 'text/html').querySelector('template') as HTMLTemplateElement).content;
}
export const SharedFormAndCheckableCapabilityState: Story = {
    render: () => '<section aria-label="Shared form capability checkpoints"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const form = root.ownerDocument.createElement('form'); root.append(form);
                const field = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', form, undefined, 'form-control');
                field.setAttribute('name', 'draft'); await f.runtime.whenRenderSettled(field);
                const control = field.querySelector<HTMLInputElement>('input')!;
                control.value = 'Unsaved é😀'; control.setSelectionRange(1, 5, 'backward');
                control.dispatchEvent(new InputEvent('input', { bubbles: true }));
                (field as HTMLElement & { setCustomValidity(message: string): void }).setCustomValidity('Review draft');
                await f.runtime.whenRenderSettled(field); await f.coordinator.capture();
                const saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('Unsaved é😀'); expect(saved).toContain('Review draft'); expect(saved).toContain('form-control');
                expect(new FormData(form).get('draft')).toBe('Unsaved é😀');
                const lease = getCemEditorProvider(field)!.lease({});
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); lease.release();
                await f.runtime.whenRenderSettled(field); await f.coordinator.capture();
                control.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                control.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                await f.runtime.whenRenderSettled(field); await f.coordinator.capture();
                const check = await f.create(declarationTemplate(checkboxDeclaration), 'cem-ml', form, undefined, 'checkable-control');
                check.setAttribute('indeterminate', ''); await f.runtime.whenRenderSettled(check);
                const checkbox = check.querySelector<HTMLInputElement>('input')!;
                checkbox.checked = false; checkbox.indeterminate = true;
                await f.coordinator.capture();
                expect(decode(f.coordinator).owners.some(owner => owner.island.includes('checkable-control'))).toBe(true);
                expect(checkbox.checked).toBe(false); expect(checkbox.indeterminate).toBe(true);
            } finally { f.dispose(); }
        }
    },
};
export const HttpDocumentsAndPendingResourceRefusal: Story = {
    render: () => '<section aria-label="HTTP document checkpoint source"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            let release!: () => void, reads = 0, credentialRead = false;
            const raw = '<catalog><entry>é😀 source</entry></catalog>';
            const f = fixture(root, fallback, false, undefined, {
                httpResourcePolicy: { allowCrossOrigin: true },
                loadHttpResource: request => {
                    reads++;
                    if (request.authoredUrl.includes('secret')) credentialRead = request.headers.authorization === 'fixture-secret';
                    if (request.authoredUrl.includes('failed')) throw new Error('Fixture read failure');
                    const source = request.authoredUrl.includes('large') ? 'x'.repeat(256 * 1024 + 1) : raw;
                    const contentType = request.authoredUrl.includes('large') ? 'text/plain' : 'application/xml';
                    const result = { response: { url: request.resolvedUrl, status: 200, statusText: 'OK', ok: true, redirected: false,
                        headers: { 'content-type': contentType }, contentType }, body: (async function* () { yield new TextEncoder().encode(source); })() };
                    return reads === 1 ? new Promise(resolve => { release = () => resolve(result); }) : Promise.resolve(result);
                },
            });
            try {
                const instance = await f.create('{http-request @slice=remote @url="https://example.test/source.xml" @content-type="application/xml"}{p | {$datadom.slices.remote.state}}');
                await waitFor(() => expect(reads).toBe(1));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); release();
                await f.runtime.whenRenderSettled(instance); await f.coordinator.capture();
                const saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('reimport-and-read-again'); expect(saved).not.toContain('documentKey');
                const content = islandContent(saved);
                const base64 = content.querySelector('[name=base64]')!.textContent!;
                const bytes = Uint8Array.from(atob(base64), c => c.charCodeAt(0));
                expect(new TextDecoder().decode(bytes)).toBe(raw);
                expect(content.querySelector('[name=digest]')!.textContent).toMatch(/^[a-f0-9]{64}$/);
                const native = wasm.retainCemDocument(bytes, 'application/xml', 'https://example.test/source.xml');
                expect(native).toBeGreaterThan(0); expect(wasm.disposeCemDocument(native)).toBe(true);
                await f.coordinator.capture(); expect(reads).toBe(1);
                const large = await f.create('{http-request @slice=large @url="https://example.test/large.txt" @content-type="text/plain"}{p | {$datadom.slices.large.state}}');
                await f.runtime.whenRenderSettled(large); await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); large.remove();
                const credential = await f.create(`<http-request slice="secret" url="https://example.test/secret.xml" header-authorization="fixture-secret"></http-request><p>Credential owner</p>`, 'dom');
                await f.runtime.whenRenderSettled(credential); expect(credentialRead).toBe(true); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                credential.remove(); await f.coordinator.capture(); expect(reads).toBe(3);
                const failed = await f.create('{http-request @slice=failed @url="https://example.test/failed.xml"}{p | {$datadom.slices.failed.state}}');
                await f.runtime.whenRenderSettled(failed); await f.coordinator.capture();
                expect(decode(f.coordinator).owners.some(owner => owner.island.includes('Fixture read failure'))).toBe(true);
                await f.coordinator.capture(); expect(reads).toBe(4);
            } finally { f.dispose(); }
        }
    },
};
export const LiveRepositoryStorageAndLocationState: Story = {
    render: () => '<section aria-label="Live provider checkpoint state"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const key = `checkpoint-json-${crypto.randomUUID()}`;
            localStorage.setItem(key, '{"label":"Original é"}');
            const registry = new CemRepositoryRegistry(), subscribers = new Set<(change: CemRepositoryChange) => void>();
            let revision = 1, queries = 0, releases = 0, executions = 0;
            registry.register('checkpoint-tests', {
                query: async request => ({ protocolVersion: 1, repository: request.repository, operation: request.operation,
                    requestRevision: request.requestRevision, repositoryRevision: revision, value: { label: `Repository ${++queries}` }, diagnostics: [] }),
                execute: async () => { executions++; throw new Error('Render must not execute commands'); },
                status: async () => ({ protocolVersion: 1, repository: 'checkpoint-tests', state: 'ready', repositoryRevision: revision, persisted: true, diagnostics: [] }),
                subscribe: (_cursor, notify) => { subscribers.add(notify); return () => { releases++; subscribers.delete(notify); }; },
            });
            const f = fixture(root, fallback, false, undefined, { repositoryRegistry: registry.readOnly() });
            const href = root.ownerDocument.location.href;
            try {
                const instance = await f.create(`{repository-query @slice=projects @repository=checkpoint-tests @operation=list @live=true @cursor=0}{storage-status @slice=status @repository=checkpoint-tests @live=true @cursor=0}{local-storage @slice=draft @key="${key}" @type=json @live=true}{location-element @slice=page @live=true}{p | {$datadom.slices.projects.data.label}}`);
                await waitFor(async () => { await f.runtime.whenRenderSettled(instance); await f.coordinator.capture(); });
                let saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('Original é'); expect(saved).toContain('cem-native-slices-v1');
                expect(saved).toContain('repository-query'); expect(saved).toContain('storage-status'); expect(saved).toContain('read-and-subscribe');
                revision++; for (const notify of subscribers) notify({ protocolVersion: 1, repository: 'checkpoint-tests', cursor: revision, repositoryRevision: revision });
                expect(f.coordinator.current()).toBeUndefined();
                await waitFor(async () => { await f.runtime.whenRenderSettled(instance); await f.coordinator.capture(); });
                expect(queries).toBe(2);
                localStorage.setItem(key, '{"label":"Changed é"}'); expect(f.coordinator.current()).toBeUndefined();
                await waitFor(async () => { await f.runtime.whenRenderSettled(instance); await f.coordinator.capture(); });
                saved = decode(f.coordinator).owners[0].island; expect(saved).toContain('Changed é');
                root.ownerDocument.defaultView!.history.replaceState({}, '', `${href.split('#')[0]}#checkpoint-live`);
                expect(f.coordinator.current()).toBeUndefined();
                await waitFor(async () => { await f.runtime.whenRenderSettled(instance); await f.coordinator.capture(); });
                expect(decode(f.coordinator).owners[0].island).toContain('#checkpoint-live');
                expect(executions).toBe(0); expect(subscribers.size).toBe(2);
                instance.remove(); expect(subscribers.size).toBe(0); expect(releases).toBe(2);
            } finally { f.dispose(); root.ownerDocument.defaultView!.history.replaceState({}, '', href); localStorage.removeItem(key); }
            const failed = fixture(root, fallback, false, undefined, { repositoryRegistry: {
                query: async () => { throw new Error('Unavailable reader'); },
                status: async () => { throw new Error('Unavailable status'); },
                subscribe: () => { throw new Error('Unavailable subscription'); },
            } });
            try {
                const instance = await failed.create('{repository-query @slice=query @repository=missing @operation=list @live=true}{storage-status @slice=status @repository=missing @live=true}{p | Failed subscriptions}');
                await failed.runtime.whenRenderSettled(instance); await failed.coordinator.capture();
                const saved = decode(failed.coordinator).owners[0].island;
                expect(saved).toContain('read-and-subscribe'); expect(saved).toContain('Unavailable subscription');
            } finally { failed.dispose(); }

        }
    },
};

export const FormAssociatedChoiceState: Story = {
    render: () => '<section aria-label="Choice capability checkpoint edits"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        const payload = '<cem-option value="a" selected>Alpha</cem-option><cem-option value="b">Beta</cem-option>';
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const select = await f.create(declarationTemplate(selectDeclaration), 'cem-ml', root, payload, 'choice-select');
                await f.runtime.whenRenderSettled(select);
                (select as HTMLElement & { value: string }).value = 'b';
                await f.runtime.whenRenderSettled(select); select.querySelector<HTMLButtonElement>('[role=combobox]')!.click();
                await f.runtime.whenRenderSettled(select); await f.coordinator.capture();
                let saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('choice-select'); expect(saved).toContain('previewSelection'); expect(saved).toContain('Beta');
                const editor = await f.create(declarationTemplate(autocompleteDeclaration), 'cem-ml', root, payload, 'editable-choice');
                await f.runtime.whenRenderSettled(editor);
                const input = editor.querySelector<HTMLInputElement>('input')!;
                input.value = 'Unsaved choice é'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await f.runtime.whenRenderSettled(editor); await f.coordinator.capture();
                saved = decode(f.coordinator).owners.find(owner => owner.island.includes('editable-choice'))!.island;
                expect(saved).toContain('Unsaved choice é'); expect(saved).toContain('defaultDisplayValue');
                input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                await f.runtime.whenRenderSettled(editor); await f.coordinator.capture();
                expect(input.value).toBe('Unsaved choice é');
            } finally { f.dispose(); }
        }
    },
};


export const NonFormWidgetContracts: Story = {
    render: () => '<section aria-label="Non-form checkpoint interaction"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            const capture = async () => { await f.coordinator.capture(); return decode(f.coordinator); };
            try {
                const tree = await f.create(declarationTemplate(treeDeclaration), 'cem-ml', root,
                    '<cem-tree-item value="docs" label="Documentation"><cem-tree-item value="readme" label="Readme"></cem-tree-item></cem-tree-item><cem-tree-item value="source" label="Source"></cem-tree-item>',
                    'tree-view', { label: 'Project resources', selection: 'single', 'expanded-values': 'docs', 'selected-values': 'readme' });
                await f.runtime.whenRenderSettled(tree);
                const node = tree.querySelector<HTMLButtonElement>('[data-tree-value="readme"]')!;
                node.focus(); node.dispatchEvent(new KeyboardEvent('keydown', { key: 's', bubbles: true }));
                await f.runtime.whenRenderSettled(tree);
                let saved = (await capture()).owners[0].island;
                expect(saved).toContain('tree-view'); expect(saved).toContain('typeahead'); expect(saved).toContain('source');
                expect(saved).toContain('selected-values'); expect(saved).toContain('expanded-values'); expect(saved).toContain('nodeId');
                const focused = document.activeElement; await capture(); expect(document.activeElement).toBe(focused);
                const tabs = await f.create(declarationTemplate(tabsDeclaration), 'cem-ml', root,
                    '<cem-tab value="one" label="One"><p>First</p></cem-tab><cem-tab value="two" label="Two"><p>Second</p></cem-tab>', 'tab-panels', { label: 'Views' });
                await f.runtime.whenRenderSettled(tabs);
                tabs.querySelectorAll<HTMLButtonElement>('[role=tab]')[1].focus();
                await f.runtime.whenRenderSettled(tabs);
                saved = (await capture()).owners.find(owner => owner.island.includes('tab-panels'))!.island;
                expect(saved).toContain('focusIndex'); expect(saved).toContain('tabCount');
                const nav = await f.create(declarationTemplate(navDeclaration), 'cem-ml', root, '<a href="#checkpoint" aria-current="page">Current</a>', 'navigation-landmark');
                await f.runtime.whenRenderSettled(nav); nav.querySelector('a')!.focus();
                await f.runtime.whenRenderSettled(nav); saved = (await capture()).owners.find(owner => owner.island.includes('navigation-landmark'))!.island;
                expect(saved).toContain('aria-current'); expect(saved).toContain('nodeId');
                const grid = await f.create(declarationTemplate(gridDeclaration), 'cem-ml', root, '<button type="button">Grid action</button>');
                await f.runtime.whenRenderSettled(grid); grid.querySelector('button')!.focus(); await f.runtime.whenRenderSettled(grid);
                expect((await capture()).owners.length).toBe(4);
            } finally { f.dispose(); }
        }
    },
};
export const SettledSurfacesAndTransientSessionRefusal: Story = {
    render: () => '<section aria-label="Checkpoint surface admission"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            try {
                const popup = await f.create(declarationTemplate(dropdownDeclaration), 'cem-ml', root, '<p>Popup content</p>', 'popup', { open: 'false' });
                await f.runtime.whenRenderSettled(popup); await f.coordinator.capture();
                expect(decode(f.coordinator).owners[0].island).toContain('popup');
                popup.setAttribute('open', 'true'); await f.runtime.whenRenderSettled(popup);
                await f.coordinator.capture(); expect(decode(f.coordinator).owners[0].island).toContain('reacquire');
                popup.setAttribute('open', 'false'); await f.runtime.whenRenderSettled(popup); await f.coordinator.capture();
                const surface = await f.create(declarationTemplate(dialogDeclaration), 'cem-ml', root, '<p>Dialog content</p>', 'native-surface');
                await f.runtime.whenRenderSettled(surface); await f.coordinator.capture();
                expect(decode(f.coordinator).owners.find(owner => owner.island.includes('native-surface'))!.island).toContain('defaultsInitialized');
                const dialog = surface.querySelector('dialog')!; dialog.showModal();
                await f.runtime.whenRenderSettled(surface); await waitFor(async () => { await f.coordinator.capture(); });
                expect(dialog.open).toBe(true);
                dialog.close('complete'); await f.runtime.whenRenderSettled(surface);
                await waitFor(async () => { await f.coordinator.capture(); });
                expect(decode(f.coordinator).owners.find(owner => owner.island.includes('native-surface'))!.island).toContain('complete');
                dialog.returnValue = ''; await waitFor(async () => { await f.runtime.whenRenderSettled(surface); await f.coordinator.capture(); });
                // A menu without an open submenu carries only roving focus/typeahead.
                const menu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root, '<button type="button">First</button><button type="button">Second</button>', 'composite-menu');
                await f.runtime.whenRenderSettled(menu); await f.coordinator.capture();
                expect(decode(f.coordinator).owners.find(owner => owner.island.includes('composite-menu'))!.island).toContain('activeNode');
            } finally { f.dispose(); }
        }
    },
};

function readySuggestionsCheckpoint(host: HTMLElement) {
    const state = cemSuggestionsCheckpoint(host);
    return state && !('phase' in state) ? state : undefined;
}
async function activeSuggestionsCheckpointContract(canvasElement: HTMLElement, mode: boolean): Promise<void> {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [mode]) {
            const f = fixture(root, fallback, false, undefined, { localSuggestions: true });
            try {
                const original = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control');
                await f.runtime.whenRenderSettled(original); const tag = original.localName; original.remove();
                const host = await f.create(declarationTemplate(suggestionsDeclaration), 'cem-ml', root,
                    `<${tag} slot="editor" name="choice" label="Choice"></${tag}><template slot="options"><option value="same">Alpha</option><option value="same">Another</option><option value="b">Beta</option></template>`, 'suggestions', { filter: 'none', 'require-selection': '' });
                await f.runtime.whenRenderSettled(host);
                const field = host.querySelector<HTMLElement>(tag)!; await f.runtime.whenRenderSettled(field);
                const input = field.querySelector('input')!;
                const ready = () => waitFor(() => expect(readySuggestionsCheckpoint(host)).toBeDefined());
                const capture = () => waitFor(async () => { await f.coordinator.capture(); });
                await ready();
                const revision = f.runtime.readInstanceSnapshot(host).dataRevision;
                expect(f.runtime.readInstanceSnapshot(host).dataRevision).toBe(revision);
                await capture(); expect(decode(f.coordinator).owners).toHaveLength(2);
                expect(f.coordinator.current()!.receipt.relationships?.some(edge => edge.kind === 'editor-provider')).toBe(true);
                input.focus(); input.value = 'same'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await ready(); await capture();
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
                await ready(); await capture(); expect(readySuggestionsCheckpoint(host)?.activeIndex).toBe(0);
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
                await capture(); expect(readySuggestionsCheckpoint(host)?.activeIndex).toBe(1);
                const listbox = host.querySelector<HTMLElement>('[part~=surface]')!;
                listbox.querySelector('[role=option]')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerType: 'mouse', isPrimary: true, pointerId: 1, button: 0 }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                listbox.dispatchEvent(new PointerEvent('pointercancel', { bubbles: true, pointerId: 1 })); await capture();
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
                await ready(); await capture();
                expect(readySuggestionsCheckpoint(host)?.committedIndex).toBe(1); expect(input.value).toBe('same');
                const saved = decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island;
                expect(saved).toContain('committedIndex'); expect(saved).not.toContain('sessionKey');
                const captured = readySuggestionsCheckpoint(host)!;
                const copyScope = createCemDeclarationScope({ document });
                const copyOwner = cemProcessingHostForScope(copyScope, { workerScriptUrl: new URL('./internal/runtime-support/processing-worker.ts', import.meta.url),
                    ...(fallback ? { workerFactory: () => { throw new Error('independent checkpoint fallback'); } } : {}) });
                const originalSource = captured.source.sources[0];
                const bytes = Uint8Array.from(atob(originalSource.base64), character => character.charCodeAt(0));
                const copy = await CemNativeCapabilitySession.prepare(copyOwner, { action: 'prepare', adapter: 'suggestions-v1',
                    handle: { sessionKey: crypto.randomUUID(), instanceId: 'checkpoint-copy', scopePolicyStamp: 'copy-policy', sourceRevision: captured.source.revision },
                    sources: { kind: 'cem-native-session-import-v1', bytes: bytes.buffer, sourceUri: originalSource.sourceUri!, contentType: 'application/xml' },
                    data: {}, select: captured.source.select, limits: { maxBytes: 256 * 1024, maxValues: 1024, maxDepth: 64 } }, () => true);
                try {
                    const publication = await copy.publishSuggestions({ query: captured.query, queryRevision: captured.queryRevision, filter: 'none' }, () => true);
                    const binding = publication.bind({ instanceId: 'fresh-consumer', scopePolicyStamp: 'fresh-policy', revision: '1', current: () => true });
                    const rows = await binding.rows();
                    expect(rows[captured.committedIndex].value).toBe(captured.committedValue);
                    expect(rows[0].source).not.toBe(rows[1].source);
                    expect(rows[captured.committedIndex].source).not.toBe(cemSuggestionsControllerFor(host)?.committed);
                    binding.release(); await publication.release();
                } finally { await copy.release(); copyScope.dispose(); }
                // The same native proof survives a read-only handoff without serializing a publication lease.
                const retained = cemSuggestionsControllerFor(host)!.committed;
                input.blur(); field.setAttribute('readonly', ''); await f.runtime.whenRenderSettled(field);
                const inactive = () => { const state = cemSuggestionsCheckpoint(host); return state && 'phase' in state && state.phase === 'inactive' ? state : undefined; };
                await waitFor(() => expect(inactive()?.proof?.index).toBe(1)); await capture();
                expect(inactive()?.proof?.source).toEqual(captured.source);
                expect(inactive()?.proof?.value).toBe('same'); expect(input.validationMessage).toBe(''); expect(input.validity.customError).toBe(false);
                expect(cemSuggestionsControllerFor(host)!.committed).toBe(retained);
                const inactiveForeign = getCemEditorProvider(field)!.subscribe(() => undefined);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); inactiveForeign(); await capture();
                const inactiveIsland = decode(f.coordinator).owners.find(owner => owner.island.includes('revalidate'))!.island;
                expect(inactiveIsland).toContain('proof'); expect(inactiveIsland).not.toContain('sessionKey');
                field.removeAttribute('readonly'); await f.runtime.whenRenderSettled(field);
                await ready(); await capture(); input.focus(); await ready(); await capture();
                const provider = getCemEditorProvider(field)!;
                const foreign = provider.subscribe(() => undefined);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign(); await capture();
                const emptyValidity = provider.validity({});
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); emptyValidity.release(); await capture();
                input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })); await ready(); await capture();
                host.setAttribute('options-state', 'pending'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                host.setAttribute('options-state', 'ready'); await ready(); await capture();
                expect(input.value).toBe('same');
                input.blur(); field.setAttribute('readonly', ''); await f.runtime.whenRenderSettled(field);
                await waitFor(() => expect(inactive()?.proof).toBeDefined()); await capture();
                const retainedCommit = f.coordinator.lastCommitted()!.bytes;
                const originalOptions = f.runtime.localSuggestionsEnvironmentFor(host)!.optionsSources[0] as HTMLTemplateElement;
                originalOptions.innerHTML = '<option value=changed>Changed source</option>';
                await expect(f.coordinator.capture()).rejects.toThrow();
                await waitFor(() => expect(cemSuggestionsControllerFor(host)?.committed).toBeUndefined());
                await waitFor(() => expect(inactive()?.feedbackState).toBe('unavailable'));
                expect(inactive()?.proof).toBeUndefined(); expect(input.validity.customError).toBe(true);
                // Read-only native controls hide customError messages; preserve whole-domain refusal.
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(retainedCommit);
                field.removeAttribute('readonly'); await f.runtime.whenRenderSettled(field); input.focus();
                await ready(); await capture(); expect(input.validationMessage).toBe('Select a suggestion.');
                expect(input.value).toBe('same');
            } finally { f.dispose(); }
        }

}
export const ActiveSuggestionsProviderAndProofCapture: Story = {
    render: () => '<section aria-label="Active suggestions worker checkpoint"></section>',
    play: ({ canvasElement }) => activeSuggestionsCheckpointContract(canvasElement, false),
};
export const ActiveSuggestionsProviderAndProofCaptureFallback: Story = {
    render: () => '<section aria-label="Active suggestions fallback checkpoint"></section>',
    play: ({ canvasElement }) => activeSuggestionsCheckpointContract(canvasElement, true),
};
export const CommittedPlacementCaptureAndRevocation: Story = {
    render: () => '<section aria-label="Committed placement checkpoint"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const placements = new CemElementPlacementCoordinator(document), references = elementReferenceFixtureInputs();
            const f = fixture(root, fallback, false, undefined, { placementCoordinator: placements, elementReferenceInputs: () => references });
            try {
                const producer = await f.create(`{dialog @id=checkpoint-target-${crypto.randomUUID()} | Target}`); await f.runtime.whenRenderSettled(producer);
                await f.coordinator.capture();
                const target = producer.querySelector('dialog')!, receipt = f.coordinator.current()!.receipt.owners[0];
                const lease = placements.register({ producer: receipt.instanceId, path: [0], revision: String(receipt.dataRevision), element: target,
                    source: references.sources[1], select: 'input.children', currentRevision: () => producer.isConnected ? String(receipt.dataRevision) : undefined });
                const consumer = await f.create(ELEMENT_REFERENCE_TEMPLATE.replace('{$destination}', '')); await f.runtime.whenRenderSettled(consumer);
                const revoke = placements.grant(f.runtime.snapshotInstance(consumer).instanceId, lease, ['commandfor']);
                f.runtime.refreshElementReferences(consumer); await f.runtime.whenRenderSettled(consumer);
                expect(consumer.querySelector('button')?.getAttribute('commandfor')).toBe(target.id);
                await f.coordinator.capture(); expect(f.coordinator.current()!.receipt.relationships).toHaveLength(1);
                expect(decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island).toContain('cemb');
                const last = f.coordinator.lastCommitted()!.bytes;
                producer.remove(); await expect(f.coordinator.capture()).rejects.toThrow();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
                revoke(); lease.dispose(); consumer.remove();
                const repaired = await f.create('{p | Portable owner after revocation}'); await f.runtime.whenRenderSettled(repaired);
                await waitFor(async () => { await f.coordinator.capture(); });
                const button = repaired.querySelector('p');
                if (button) { button.setAttribute('data-cem-placement-ref-commandfor', ''); await expect(f.coordinator.capture()).rejects.toThrow(); }
            } finally { f.dispose(); placements.dispose(); }
        }
    },
};

async function remainingSuggestionsFixture(root: HTMLElement, fallback: boolean, profile: 'listbox' | 'native-datalist', type = 'text', readonly = false) {
    const f = fixture(root, fallback, false, undefined, { localSuggestions: true });
    const declaration = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control');
    await f.runtime.whenRenderSettled(declaration); const tag = declaration.localName; declaration.remove();
    const host = await f.create(declarationTemplate(suggestionsDeclaration), 'cem-ml', root,
        `<${tag} slot="editor" type="${type}" name="choice" label="Choice"${readonly ? ' readonly' : ''}></${tag}><template slot="options"><option value="1" label="One">One</option><option value="2" label="Two">Two</option></template>`,
        'suggestions', profile === 'listbox' ? { filter: 'none' } : { profile });
    await f.runtime.whenRenderSettled(host);
    const field = host.querySelector<HTMLElement>(tag)!; await f.runtime.whenRenderSettled(field);
    const input = field.querySelector('input')!;
    const capture = () => waitFor(async () => { await f.coordinator.capture(); });
    return { f, host, field, input, capture };
}
async function inactiveSuggestionsContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    const { f, host, field, input, capture } = await remainingSuggestionsFixture(root, fallback, 'listbox', 'text', true);
    const inactive = () => { const state = cemSuggestionsCheckpoint(host); return state && 'phase' in state && state.phase === 'inactive' ? state : undefined; };
    try {
        await waitFor(() => expect(inactive()?.feedbackState).toBe('unavailable')); await capture();
        expect(inactive()?.source.rowCount).toBe(0);
        field.removeAttribute('readonly'); await f.runtime.whenRenderSettled(field);
        await waitFor(() => expect(inactive()?.feedbackState).toBe('idle')); await capture();
        const provider = getCemEditorProvider(field)!;
        const foreign = provider.subscribe(() => undefined);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign(); await capture();
        const validity = provider.validity({});
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); validity.release(); await capture();
        input.focus(); await waitFor(() => expect(readySuggestionsCheckpoint(host)).toBeDefined());
        host.setAttribute('require-selection', '');
        input.value = 'Unsaved'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
        await waitFor(() => expect(readySuggestionsCheckpoint(host)?.query).toBe('Unsaved')); await capture();
        host.setAttribute('options-state', 'failed');
        await waitFor(() => expect(inactive()?.feedbackState).toBe('failed')); await capture();
        expect(inactive()?.query).toBe('Unsaved'); expect(inactive()?.source.rowCount).toBe(0);
        expect(input.validationMessage).toBe('Select a suggestion.');
        expect(atob(inactive()!.source.sources[0].base64)).toContain('One');
        const saved = decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island;
        expect(saved).not.toContain('committedIndex'); expect(saved).not.toContain('sessionKey');
        input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true })); await capture();
        host.setAttribute('options-state', 'pending'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        host.setAttribute('options-state', 'ready');
        await waitFor(() => expect(readySuggestionsCheckpoint(host)).toBeDefined()); await capture();
        expect(input.value).toBe('Unsaved');
        const last = f.coordinator.lastCommitted()!.bytes;
        f.runtime.setLocalSuggestionsEnabled(false); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        f.runtime.setLocalSuggestionsEnabled(true);
        await waitFor(() => expect(readySuggestionsCheckpoint(host)).toBeDefined()); await capture();
    } finally { f.dispose(); }
}
export const InactiveSuggestionsCheckpoint: Story = {
    render: () => '<section aria-label="Inactive suggestions worker checkpoint"></section>',
    play: ({ canvasElement }) => inactiveSuggestionsContract(canvasElement, false),
};
export const InactiveSuggestionsCheckpointFallback: Story = {
    render: () => '<section aria-label="Inactive suggestions fallback checkpoint"></section>',
    play: ({ canvasElement }) => inactiveSuggestionsContract(canvasElement, true),
};
async function nativeDatalistCheckpointContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!; root.tabIndex = -1;
    for (const type of ['text', 'email', 'number']) {
        const { f, host, field, input, capture } = await remainingSuggestionsFixture(root, fallback, 'native-datalist', type);
        const datalist = () => { const state = cemSuggestionsCheckpoint(host); return state && 'phase' in state && state.phase === 'native-datalist' ? state : undefined; };
        try {
            await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
            expect(input.type).toBe(type); expect(input.list?.options.length).toBe(2);
            expect(input.hasAttribute('aria-controls')).toBe(false);
            expect(datalist()?.source.rowCount).toBe(2);
            expect(f.coordinator.current()!.receipt.relationships?.some(edge => edge.kind === 'editor-provider')).toBe(true);
            const last = f.coordinator.lastCommitted()!.bytes;
            input.focus(); input.value = '2'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
            root.focus(); await capture(); expect(input.value).toBe('2');
            const foreign = getCemEditorProvider(field)!.subscribe(() => undefined);
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign(); await capture();
            field.setAttribute('readonly', ''); await f.runtime.whenRenderSettled(field);
            await waitFor(() => expect(datalist()?.claim).toBe('unavailable')); await capture();
            expect(input.hasAttribute('list')).toBe(false);
            field.removeAttribute('readonly'); await f.runtime.whenRenderSettled(field);
            await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
            const source = f.runtime.localSuggestionsEnvironmentFor(host)!.optionsSources[0] as HTMLTemplateElement;
            source.innerHTML = '<option value="3" label="New">New</option>';
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
            await waitFor(() => expect(datalist()?.source.rowCount).toBe(1)); await capture();
            expect(atob(datalist()!.source.sources[0].base64)).toContain('New'); expect(input.list?.options[0].value).toBe('3');
            if (type === 'text') {
                const inactive = () => { const state = cemSuggestionsCheckpoint(host); return state && 'phase' in state && state.phase === 'inactive' ? state : undefined; };
                host.setAttribute('options-state', 'failed');
                await waitFor(() => expect(inactive()?.feedbackState).toBe('failed')); await capture();
                expect(input.hasAttribute('list')).toBe(false); expect(inactive()?.profile).toBe('native-datalist');
                host.setAttribute('options-state', 'pending'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                host.setAttribute('options-state', 'ready'); await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
                source.innerHTML = '<optgroup label="Unsupported"><option value="x">X</option></optgroup>';
                await waitFor(() => expect(inactive()?.feedbackState).toBe('failed')); await capture();
                expect(atob(inactive()!.source.sources[0].base64)).toContain('Unsupported');
                expect(input.hasAttribute('list')).toBe(false);
                // Equal bytes still schedule a source handover; reject the now-stale async export.
                const failedSourceCommit = f.coordinator.lastCommitted()!.bytes;
                const sameSourceText = source.innerHTML; source.innerHTML = sameSourceText;
                await expect(f.coordinator.capture()).rejects.toThrow('cem.checkpoint.stale');
                expect(f.coordinator.current()).toBeUndefined();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(failedSourceCommit);
                await waitFor(() => expect(inactive()?.feedbackState).toBe('failed')); await capture();
                source.innerHTML = '<option value="3" label="New">New</option>';
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
            }
            input.setAttribute('list', 'author-owned'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
            input.removeAttribute('list'); await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
            f.runtime.setLocalSuggestionsEnabled(false); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
            f.runtime.setLocalSuggestionsEnabled(true); await waitFor(() => expect(datalist()?.claim).toBe('attached')); await capture();
            expect(input.value).toBe('2');
        } finally { f.dispose(); }
    }
}
export const NativeDatalistCheckpoint: Story = {
    render: () => '<section aria-label="Native datalist worker checkpoint"></section>',
    play: ({ canvasElement }) => nativeDatalistCheckpointContract(canvasElement, false),
};
export const NativeDatalistCheckpointFallback: Story = {
    render: () => '<section aria-label="Native datalist fallback checkpoint"></section>',
    play: ({ canvasElement }) => nativeDatalistCheckpointContract(canvasElement, true),
};

async function submenuTaskCheckpointContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    const f = fixture(root, fallback);
    try {
        const menu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root,
            '<div><button part="control" type="button">More</button></div>', 'composite-menu', { keyboard: 'menu', direction: 'column' });
        await f.runtime.whenRenderSettled(menu);
        const launcher = menu.querySelector<HTMLButtonElement>('button')!; launcher.id = `submenu-launcher-${crypto.randomUUID()}`;
        const submenu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root,
            '<button type="button">Edit</button>', 'composite-menu', { 'parent-item': `#${launcher.id}` });
        await f.runtime.whenRenderSettled(submenu);
        const surface = await f.create(declarationTemplate(dialogDeclaration), 'cem-ml', root, '<input aria-label="Task entry" autofocus>', 'native-surface');
        await f.runtime.whenRenderSettled(surface);
        const dialog = surface.querySelector('dialog')!; dialog.id = `submenu-task-${crypto.randomUUID()}`;
        const edit = submenu.querySelector<HTMLButtonElement>('button')!;
        edit.setAttribute('commandfor', dialog.id); edit.setAttribute('command', '--cem-show');
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture(); expect(submenu.hidden).toBe(true);
        await userEvent.click(launcher); await waitFor(() => expect(submenu.hidden).toBe(false));
        await capture();
        expect(decode(f.coordinator).owners.filter(owner => owner.island.includes('composite-menu'))).toHaveLength(2);
        expect(f.coordinator.current()!.receipt.relationships?.filter(edge => edge.id.includes(':menu:'))).toHaveLength(3);
        expect(cemCompositeMenuCheckpoint(menu, () => ({ instanceId: 'fixture', dataRevision: '1', nodeId: 'node' }))?.open).toBe(true);
        launcher.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true }));
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        launcher.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true })); await capture();
        let queuedRefused = false;
        submenu.addEventListener('click', () => { queuedRefused = !cemCompositeMenuCheckpoint(submenu, () => ({ instanceId: 'fixture', dataRevision: '1', nodeId: 'node' })); }, { once: true });
        await userEvent.click(edit); await waitFor(() => expect(dialog.open).toBe(true));
        expect(queuedRefused).toBe(true); await capture();
        const edges = f.coordinator.current()!.receipt.relationships!;
        expect(edges.some(edge => edge.id.includes(':surface:interaction'))).toBe(true);
        expect(edges.some(edge => edge.id.includes(':surface:return-focus') && edge.provider.nodeId === launcher.getAttribute('data-cem-render-node-id'))).toBe(true);
        expect(submenu.hidden).toBe(true); expect(document.activeElement).toBe(dialog.querySelector('input'));
        dialog.close('saved'); await waitFor(() => expect(document.activeElement).toBe(launcher)); await capture();
        const last = f.coordinator.lastCommitted()!.bytes;
        submenu.setAttribute('parent-item', '#missing-submenu-parent');
        await expect(f.coordinator.capture()).rejects.toThrow(); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        submenu.setAttribute('parent-item', `#${launcher.id}`); await f.runtime.whenRenderSettled(submenu); await capture();
    } finally { f.dispose(); }
}
export const SubmenuAndCompletedTaskCheckpoint: Story = {
    render: () => '<section aria-label="Submenu and task worker checkpoint"></section>',
    play: ({ canvasElement }) => submenuTaskCheckpointContract(canvasElement, false),
};
export const SubmenuAndCompletedTaskCheckpointFallback: Story = {
    render: () => '<section aria-label="Submenu and task fallback checkpoint"></section>',
    play: ({ canvasElement }) => submenuTaskCheckpointContract(canvasElement, true),
};

async function explicitSubmenuCheckpointContract(canvasElement: HTMLElement, fallback: boolean, crossRuntime = false) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    root.setAttribute('interaction-scope', '');
    let duringSeal: (() => void) | undefined;
    const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
        duringSeal?.(); duringSeal = undefined;
        await Promise.resolve(); return wasm.sealCommittedCheckpoint(candidate, receipt);
    });
    const peerRoot = root.appendChild(document.createElement('div'));
    const peer = crossRuntime ? fixture(peerRoot, !fallback, false, undefined, {}, undefined, f.coordinator) : undefined;
    const producer = peer ?? f;
    try {
        const targets = await producer.create('{div @interaction-name=bounds @style="position:relative;width:600px;height:260px" | {button @type=button @interaction-name=dock | Dock}{button @type=button @interaction-name=return | Return}{input @name=draft @value=Original}}');
        await producer.runtime.whenRenderSettled(targets);
        const dock = targets.querySelector<HTMLButtonElement>('button')!, returnTo = targets.querySelector<HTMLButtonElement>('button:nth-of-type(2)')!;
        const input = targets.querySelector('input')!;
        expect(f.coordinator.endpoint(dock)?.instanceId).toBe(producer.runtime.readInstanceSnapshot(targets).instanceId);
        const forged = dock.cloneNode(true) as HTMLButtonElement;
        (forged as HTMLElement & { cemRenderNodeId?: string }).cemRenderNodeId = dock.getAttribute('data-cem-render-node-id')!;
        targets.append(forged); expect(f.coordinator.endpoint(forged)).toBeUndefined(); forged.remove();
        input.focus(); input.value = 'Unsaved draft'; input.setSelectionRange(2, 8);
        input.dispatchEvent(new InputEvent('input', { bubbles: true })); await producer.runtime.whenRenderSettled(targets);
        const menu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root,
            '<button part="control" type="button">More</button>', 'composite-menu', { keyboard: 'menu', direction: 'column', boundary: '@bounds' });
        await f.runtime.whenRenderSettled(menu);
        const launcher = menu.querySelector<HTMLButtonElement>('button')!; launcher.id = `explicit-launcher-${crypto.randomUUID()}`;
        const submenu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root,
            '<button type="button">Alpha</button><button type="button" interaction-name="entry">Beta</button>', 'composite-menu',
            { 'parent-item': `#${launcher.id}`, anchor: '@dock', 'focus-target': '@entry', 'return-focus': '@return', 'anchor-lost': 'freeze' });
        await f.runtime.whenRenderSettled(submenu);
        const entry = submenu.querySelector<HTMLButtonElement>('button:nth-of-type(2)')!;
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        const policyIsland = () => decode(f.coordinator).owners.find(owner => owner.instanceId === f.runtime.readInstanceSnapshot(submenu).instanceId)!.island;
        await capture(); expect(submenu.hidden).toBe(true);
        expect(policyIsland()).toContain('recompute'); expect(policyIsland()).toContain('boundaryOwner');
        const edges = f.coordinator.current()!.receipt.relationships!;
        const hostId = f.runtime.readInstanceSnapshot(submenu).instanceId;
        for (const property of ['anchor', 'boundary', 'focus-target', 'return-focus']) {
            expect(edges.some(edge => edge.requester.instanceId === hostId && edge.requester.nodeId === 'host' && edge.property === property)).toBe(true);
        }
        expect(edges.some(edge => edge.requester.instanceId === hostId && edge.property === 'interaction' && edge.requester.nodeId === 'host'
            && edge.provider.instanceId === f.runtime.readInstanceSnapshot(menu).instanceId)).toBe(true);
        await userEvent.click(launcher); await waitFor(() => expect(submenu.hidden).toBe(false));
        await waitFor(() => expect(document.activeElement).toBe(entry)); await capture();
        const style = submenu.getAttribute('style'); const revision = f.runtime.readInstanceSnapshot(submenu).dataRevision;
        await capture(); expect(document.activeElement).toBe(entry); expect(submenu.getAttribute('style')).toBe(style);
        expect(f.runtime.readInstanceSnapshot(submenu).dataRevision).toBe(revision);
        expect(input.value).toBe('Unsaved draft'); expect(input.selectionStart).toBe(2); expect(input.selectionEnd).toBe(8);
        const geometryCurrent = () => checkpointPopupGeometry(launcher, submenu, true, submenu, menu);
        expect(geometryCurrent()).toBe(true);
        const last = f.coordinator.lastCommitted()!.bytes;
        dock.style.transform = 'translateX(25px)';
        expect(geometryCurrent()).toBe(false);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        await waitFor(() => expect(geometryCurrent()).toBe(true)); await capture();
        targets.querySelector<HTMLElement>('[interaction-name=bounds]')!.style.width = '580px';
        expect(geometryCurrent()).toBe(false); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        await waitFor(() => expect(geometryCurrent()).toBe(true)); await capture();
        submenu.style.left = '13px'; expect(geometryCurrent()).toBe(false);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        await f.runtime.whenRenderSettled(submenu); window.dispatchEvent(new Event('resize')); await capture();
        dock.hidden = true; await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(submenu.hidden).toBe(false); // freeze keeps presentation, never stale geometry authority.
        dock.hidden = false; window.dispatchEvent(new Event('resize')); await capture();
        releasePopupGeometry(submenu);
        const foreign = createCemSurfaceGeometryLease(root, submenu, () => undefined);
        try {
            expect(foreign.fit(dock, undefined, 'block-end start')).toBe(true);
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        } finally { foreign.release(); }
        await userEvent.keyboard('{Escape}'); await waitFor(() => expect(submenu.hidden).toBe(true));
        await userEvent.click(launcher); await capture();
        await userEvent.keyboard('{Escape}'); await waitFor(() => expect(submenu.hidden).toBe(true));
        expect(document.activeElement).toBe(returnTo); await capture();
        // Closed panels keep target identity; eligibility and focus are reacquired on opening.
        entry.disabled = true; await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); entry.disabled = false; await capture();
        const unowned = document.createElement('button'); unowned.id = `unowned-${crypto.randomUUID()}`; root.append(unowned);
        for (const target of [`#${unowned.id}`, '#missing-target']) {
            submenu.setAttribute('anchor', target); await f.runtime.whenRenderSettled(submenu);
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        }
        unowned.remove(); submenu.setAttribute('anchor', '@dock'); await f.runtime.whenRenderSettled(submenu); await capture();
        const duplicate = document.createElement('button'); duplicate.setAttribute('interaction-name', 'dock'); root.append(duplicate);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); duplicate.remove(); await capture();
        submenu.setAttribute('focus-target', 'none'); submenu.setAttribute('return-focus', 'none'); submenu.setAttribute('boundary', 'viewport');
        await f.runtime.whenRenderSettled(submenu); await capture();
        expect(policyIsland()).toContain('none');
        await userEvent.click(launcher); expect(document.activeElement).toBe(launcher); await capture();
        await userEvent.keyboard('{Escape}'); await capture();
        const good = f.coordinator.lastCommitted()!.bytes;
        duringSeal = () => submenu.setAttribute('anchor', '#missing-during-seal');
        await expect(f.coordinator.capture()).rejects.toThrow('stale'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(good);
        submenu.setAttribute('anchor', '@dock'); await f.runtime.whenRenderSettled(submenu); await capture();
        if (peer) {
            const lastPeer = f.coordinator.lastCommitted()!.bytes;
            duringSeal = () => { input.value = 'Peer input during seal'; input.dispatchEvent(new InputEvent('input', { bubbles: true })); };
            await expect(f.coordinator.capture()).rejects.toThrow('stale');
            expect(f.coordinator.lastCommitted()!.bytes).toEqual(lastPeer);
            await producer.runtime.whenRenderSettled(targets); await capture();
        }
        f.coordinator.setPolicy('revoked:submenu', () => false);
        for (const host of [targets, menu, submenu]) { host.setAttribute('title', 'New policy'); await (host === targets ? producer : f).runtime.whenRenderSettled(host); }
        await expect(f.coordinator.capture()).rejects.toThrow('privacy'); expect(f.coordinator.lastCommitted()).toBeUndefined();
    } finally { peer?.dispose(); f.dispose(); root.removeAttribute('interaction-scope'); }
}
export const ExplicitSubmenuGeometryAndFocusCheckpoint: Story = {
    render: () => '<section aria-label="Explicit submenu worker checkpoint"></section>',
    play: ({ canvasElement }) => explicitSubmenuCheckpointContract(canvasElement, false),
};
export const ExplicitSubmenuGeometryAndFocusCheckpointFallback: Story = {
    render: () => '<section aria-label="Explicit submenu fallback checkpoint"></section>',
    play: ({ canvasElement }) => explicitSubmenuCheckpointContract(canvasElement, true),
};

async function typedSubmenuCheckpointContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    const f = fixture(root, fallback);
    try {
        const menu = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root,
            '<button part="control" type="button">More</button>', 'composite-menu', { keyboard: 'menu', direction: 'column' });
        await f.runtime.whenRenderSettled(menu);
        const launcher = menu.querySelector('button')!; launcher.id = `typed-launcher-${crypto.randomUUID()}`;
        const definition = await f.create(declarationTemplate(menuDeclaration), 'cem-ml', root, undefined, 'composite-menu');
        await f.runtime.whenRenderSettled(definition); const tag = definition.localName; definition.remove();
        // Native reference expressions publish the typed markers; the fixture does not author them.
        const producer = await f.create(`{cem:variable @name=entry @select='data:read("<button type=\\"button\\">Entry</button>", "xml").root.children'}{cem:variable @name=returner @select='data:read("<button type=\\"button\\">Return</button>", "xml").root.children'}{cem:variable @name=anchorNode @select='data:read("<div style=\\"width:80px;height:20px\\">Anchor</div>", "xml").root.children'}{cem:variable @name=bounds @select='data:read("<div style=\\"width:600px;height:300px\\">Boundary</div>", "xml").root.children'}{${tag} @parent-item="#${launcher.id}" @focus-target={#entry} @return-focus={#returner} @anchor={#anchorNode} @boundary={#bounds} | {$entry}}{$returner}{$anchorNode}{$bounds}`);
        await f.runtime.whenRenderSettled(producer);
        const submenu = producer.querySelector<HTMLElement>(tag)!;
        expect(submenu).not.toBeNull(); await f.runtime.whenRenderSettled(submenu);
        for (const name of ['anchor', 'boundary', 'focus-target', 'return-focus']) expect(submenu.hasAttribute(`data-cem-node-ref-${name}`)).toBe(true);
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture();
        await userEvent.click(launcher); await waitFor(() => expect(submenu.hidden).toBe(false)); await capture();
        expect(document.activeElement).toBe(submenu.querySelector('button'));
        const id = f.runtime.readInstanceSnapshot(submenu).instanceId;
        const edges = f.coordinator.current()!.receipt.relationships!.filter(edge => edge.requester.instanceId === id && edge.requester.nodeId === 'host');
        expect(edges.map(edge => edge.property).sort()).toEqual(['anchor', 'boundary', 'focus-target', 'return-focus']);
        await userEvent.keyboard('{Escape}'); await capture();
        expect(document.activeElement?.textContent).toBe('Return');
        producer.remove(); await capture();
        expect(f.coordinator.current()!.receipt.owners).toHaveLength(1);
    } finally { f.dispose(); }
}
export const TypedSubmenuGeometryAndFocusCheckpoint: Story = {
    render: () => '<section aria-label="Typed submenu worker checkpoint"></section>',
    play: ({ canvasElement }) => typedSubmenuCheckpointContract(canvasElement, false),
};
export const TypedSubmenuGeometryAndFocusCheckpointFallback: Story = {
    render: () => '<section aria-label="Typed submenu fallback checkpoint"></section>',
    play: ({ canvasElement }) => typedSubmenuCheckpointContract(canvasElement, true),
};

async function externalCheckpointSource(fallback: boolean, portable = true, captured?: CheckpointSource, datalist = false, withInputs = false) {
    const scope = createCemDeclarationScope({ document });
    const owner = cemProcessingHostForScope(scope, { workerScriptUrl: new URL('./internal/runtime-support/processing-worker.ts', import.meta.url),
        ...(fallback ? { workerFactory: () => { throw new Error('external source fallback'); } } : {}) });
    let source = captured;
    if (!source) {
        const id = wasm.parseReferenceSource(new TextEncoder().encode(datalist ? '{option @value=same @label=Alpha}{option @value=same @label=Another}' : '{cem-option @value=same @label=Alpha}{cem-option @value=same @label=Another}'), 'text/cem-ml', 'memory:external-options.cem', '');
        try { source = { version: 'cem-native-source/1', revision: crypto.randomUUID(), rowCount: 2, select: 'input.children', resolve: false, requesting: 0,
            sources: [{ codec: 'cemb', base64: btoa(String.fromCharCode(...wasm.exportReferenceReloadBundle(id, ''))), sourceUri: null, primarySourceId: 1, context: true }],
            bindings: [], crossings: [], labels: { option: null, group: null }, disposition: 'reacquire' }; }
        finally { wasm.disposeReferenceSource(id); }
    }
    const preparation = source.preparation;
    const nativeAttributes = preparation ? importNativeCemAttributes({ kind: 'cem-native-attributes-v1', artifacts: preparation.nativeBindings.artifacts,
        attributes: preparation.nativeBindings.bindings.filter(b => b.target === 'attribute').map(({name,artifact,index}) => ({name,artifact,index})) })
        : withInputs ? [{name:'native-count',value:nativeCheckpointFixtureValue()}] : [];
    const nativeSlices = preparation ? preparation.nativeBindings.bindings.filter(b => b.target === 'slice').map(b => ({
        name:b.name, attribute:b.attribute, value:importNativeCemAttributes({kind:'cem-native-attributes-v1',artifacts:preparation.nativeBindings.artifacts,
            attributes:[{name:b.name,artifact:b.artifact,index:b.index}]})[0].value }))
        : withInputs ? [{name:'count',attribute:'native-count',value:nativeAttributes[0].value}, {name:'raw',value:nativeAttributes[0].value}] : [];
    const session = await CemNativeCapabilitySession.prepare(owner, { action: 'prepare', adapter: datalist ? 'native-datalist-v1' : 'suggestions-v1',
        handle: { sessionKey: crypto.randomUUID(), instanceId: 'external-source', scopePolicyStamp: 'external-policy', sourceRevision: source.revision },
        sources: { kind: 'cem-native-session-sources-v1', requesting: source.requesting,
            sources: source.sources.map(input => ({ bundle: Uint8Array.from(atob(input.base64), c => c.charCodeAt(0)).buffer, primarySourceId: input.primarySourceId, context: input.context })),
            bindings: source.bindings, grants: source.crossings.map(route => [route.from, route.to] as const) },
        data: preparation ? JSON.parse(preparation.dataJson) : withInputs ? {datadom:{slices:{caption:'Original é😀',enabled:true}}} : {},
        nativeAttributes, nativeSlices, select: source.select, resolve: source.resolve,
        limits: preparation?.limits ?? { maxBytes: 256 * 1024, maxValues: 1024, maxDepth: 64 } }, () => true, undefined, portable);
    return { scope, session, async dispose() { await session.release(); scope.dispose(); } };
}
async function nativePublicationCheckpointContract(canvasElement: HTMLElement, fallback: boolean, withInputs = false) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    let admitted = true, publication: CemNativeSuggestionsPublication | undefined, sealFence: (() => void) | undefined;
    const sources: Awaited<ReturnType<typeof externalCheckpointSource>>[] = [];
    const f = fixture(root, fallback, false, undefined, { nativeSuggestionsInputs(instance, snapshot) {
        const current = publication;
        return current ? [current.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp, revision: snapshot.dataRevision,
            current: () => admitted && publication === current && instance.isConnected })] : [];
    } }, async (candidate, receipt) => { const sealed = wasm.sealCommittedCheckpoint(candidate, receipt); sealFence?.(); sealFence = undefined; await Promise.resolve(); return sealed; });
    try {
        const source = await externalCheckpointSource(fallback, true, undefined, false, withInputs); sources.push(source);
        publication = await source.session.publishSuggestions({ query: 'same', queryRevision: 1, filter: 'none' }, () => admitted);
        const frame = await f.create('{cem:for-each @select="datadom.slices.suggestions.children" @as=row | {span @role=option @suggestion-row={#row} | {$row.dom:attribute("label").value}}}');
        await f.runtime.whenRenderSettled(frame);
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture(); const saved = decode(f.coordinator).owners[0].island;
        expect(saved).toContain('nativePublication'); expect(saved).toContain('cemb'); expect(saved).not.toContain('sessionKey');
        const mapped = f.runtime.renderedSuggestionsFor(frame)!; expect(mapped.rows).toHaveLength(2);
        const copy = await externalCheckpointSource(fallback, true, source.session.checkpointSource()); sources.push(copy);
        if (withInputs) {
            expect(saved).toContain('cem-native-bindings/1'); expect(saved).toContain('Original é😀');
            expect(copy.session.checkpointSource()?.preparation).toEqual(source.session.checkpointSource()?.preparation);
            const view = await copy.session.render('{span | {$datadom.slices.count}}');
            expect(JSON.stringify(view.nodes)).toContain('42');
        }
        const copiedPublication = await copy.session.publishSuggestions({ query: 'same', queryRevision: 1, filter: 'none' }, () => true);
        const binding = copiedPublication.bind({ instanceId: 'fresh', scopePolicyStamp: 'fresh', revision: '1', current: () => true });
        try { const rows = await binding.rows(); expect(rows.map(row => row.value)).toEqual(['same', 'same']); expect(rows[1].source).not.toBe(mapped.rows[1].native.source); }
        finally { binding.release(); await copiedPublication.release(); }
        const row = mapped.rows[0].element, nodeId = row.getAttribute('data-cem-render-node-id')!;
        row.setAttribute('data-cem-render-node-id', 'forged'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        row.setAttribute('data-cem-render-node-id', nodeId); await capture();
        const last = f.coordinator.lastCommitted()!.bytes;
        sealFence = () => { admitted = false; };
        await expect(f.coordinator.capture()).rejects.toThrow('cem.checkpoint.stale');
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        await source.session.release();
        const replacement = await externalCheckpointSource(fallback); sources.push(replacement); admitted = true;
        publication = await replacement.session.publishSuggestions({ query: '', queryRevision: 2, filter: 'none' }, () => admitted);
        f.runtime.refreshElementReferences(frame); await f.runtime.whenRenderSettled(frame); await capture();
        expect(frame.querySelectorAll('[role=option]')).toHaveLength(2);
        const opaque = await externalCheckpointSource(fallback, false); sources.push(opaque);
        publication = await opaque.session.publishSuggestions({ query: '', queryRevision: 3, filter: 'none' }, () => true);
        f.runtime.refreshElementReferences(frame); await f.runtime.whenRenderSettled(frame);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
    } finally { f.dispose(); for (const source of sources) await source.dispose(); }
}
export const ExternalNativePublicationCheckpoint: Story = {
    render: () => '<section aria-label="External native publication worker checkpoint"></section>',
    play: ({ canvasElement }) => nativePublicationCheckpointContract(canvasElement, false),
};
export const ExternalNativePublicationCheckpointFallback: Story = {
    render: () => '<section aria-label="External native publication fallback checkpoint"></section>',
    play: ({ canvasElement }) => nativePublicationCheckpointContract(canvasElement, true),
};

async function externalProviderCheckpointContract(canvasElement: HTMLElement, fallback: boolean, crossRuntime = false) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    let host: HTMLElement | undefined, options: CemSuggestionsControllerOptions | undefined, publication: CemNativeSuggestionsPublication | undefined;
    let admitted = true, latest = 0, failed = false, delay: Promise<void> | undefined;
    const sources: Awaited<ReturnType<typeof externalCheckpointSource>>[] = [];
    let source = await externalCheckpointSource(fallback); sources.push(source);
    const placements = new CemSuggestionsPlacementCoordinator(document);
    let leases: CemSuggestionsPlacementLease[] = [];
    const errors: unknown[] = [];
    const f = fixture(root, fallback, false, undefined, {
        suggestionsControllerInputs: instance => instance === host ? options : undefined,
        nativeSuggestionsInputs(instance, snapshot) {
            const current = publication;
            return instance === host && current ? [current.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp,
                revision: snapshot.dataRevision, current: () => admitted && instance.isConnected && current === publication })] : [];
        },
    });
    const peerRoot = root.appendChild(document.createElement('div'));
    const peer = crossRuntime ? fixture(peerRoot, !fallback, false, undefined, {}, undefined, f.coordinator) : undefined;
    const editorRuntime = peer ?? f;
    try {
        const field = await editorRuntime.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control');
        await editorRuntime.runtime.whenRenderSettled(field); root.setAttribute('interaction-scope', ''); field.setAttribute('interaction-name', 'external-field');
        const input = field.querySelector('input')!, provider = getCemEditorProvider(field)!;
        host = await f.create(declarationTemplate(suggestionsDeclaration), 'cem-ml', root, undefined, 'suggestions', { 'editor-for': '@external-field', filter: 'none', 'require-selection': '' });
        await f.runtime.whenRenderSettled(host); const listbox = host.querySelector<HTMLElement>('[part~=surface]')!;
        options = { editorHost: field, listbox, current: () => admitted && source.session.valid,
            checkpointSource: () => source.session.checkpointSourceIntent(), onError: error => errors.push(error),
            async prepare(query, queryRevision) {
                latest = queryRevision; if (delay) await delay;
                if (failed) throw new Error('External source temporarily unavailable');
                for (const lease of leases) lease.dispose(); leases = [];
                const capturedSource = source.session, revision = provider.revision;
                const current = () => admitted && source.session === capturedSource && capturedSource.valid && latest === queryRevision && provider.revision === revision;
                publication = await capturedSource.publishSuggestions({ query, queryRevision, filter: 'none' }, current);
                f.runtime.refreshElementReferences(host!); await f.runtime.whenRenderSettled(host!);
                const mapped = f.runtime.renderedSuggestionsFor(host!); if (!mapped) throw new Error('External native row publication missing');
                const editor = placements.register({ producer: editorRuntime.runtime.readInstanceSnapshot(field).instanceId, revision: String(revision), current,
                    element: field, kind: 'editor' }); leases.push(editor);
                const panel = placements.register({ producer: f.runtime.readInstanceSnapshot(host!).instanceId, revision: String(queryRevision), current,
                    element: listbox, kind: 'listbox' }); leases.push(panel);
                placements.grant(f.runtime.readInstanceSnapshot(host!).instanceId, editor, ['editor-for']);
                placements.grant(editorRuntime.runtime.readInstanceSnapshot(field).instanceId, panel, ['aria-controls']);
                const rows = mapped.rows.map(row => {
                    const placed = placements.register({ producer: f.runtime.readInstanceSnapshot(host!).instanceId, revision: String(queryRevision),
                        current: () => current() && mapped.current(), element: row.element, kind: 'row', row: row.native }); leases.push(placed);
                    placements.grant(editorRuntime.runtime.readInstanceSnapshot(field).instanceId, placed, ['aria-activedescendant']); return placed;
                });
                return placements.prepare(mapped.binding, editor, panel, rows);
            },
        };
        f.runtime.setInstanceSlices(host, { externalReady: true }); await f.runtime.whenRenderSettled(host);
        const ready = () => waitFor(() => { expect(errors.filter(error => !(error instanceof Error) || error.message !== 'External source temporarily unavailable')).toEqual([]); expect(readySuggestionsCheckpoint(host!)).toBeDefined(); });
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await ready(); await capture();
        expect(f.coordinator.current()!.receipt.relationships?.some(edge => edge.kind === 'editor-provider')).toBe(true);
        expect(field.parentElement).toBe(root); expect(host.contains(field)).toBe(false);
        input.focus(); input.value = 'same'; input.dispatchEvent(new InputEvent('input', { bubbles: true })); await ready();
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
        input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
        input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true })); await ready(); await capture();
        expect(readySuggestionsCheckpoint(host)?.committedIndex).toBe(1);
        expect(source.session.checkpointSourceIntent()?.rowCount).toBe(0);
        const foreign = provider.lease({}); await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign.release();
        await ready(); await capture();
        failed = true; await cemSuggestionsControllerFor(host)!.refresh();
        const inactive = () => { const state = cemSuggestionsCheckpoint(host!); return state && 'phase' in state && state.phase === 'inactive' ? state : undefined; };
        await waitFor(() => expect(inactive()?.feedbackState).toBe('failed')); await capture();
        expect(inactive()?.proof?.index).toBe(1); expect(inactive()?.proof?.source).toEqual(source.session.checkpointSource());
        failed = false; await cemSuggestionsControllerFor(host)!.refresh(); await ready(); await capture();
        let resume!: () => void; delay = new Promise<void>(resolve => { resume = resolve; });
        const pending = cemSuggestionsControllerFor(host)!.refresh();
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); delay = undefined; resume(); await pending; await ready(); await capture();
        const last = f.coordinator.lastCommitted()!.bytes;
        await source.session.release(); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        source = await externalCheckpointSource(fallback); sources.push(source);
        await cemSuggestionsControllerFor(host)!.refresh(); await ready(); await capture();
        expect(readySuggestionsCheckpoint(host)?.committedIndex).toBe(-1); expect(input.value).toBe('same');
        admitted = false; await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(errors.every(error => error instanceof Error && error.message === 'External source temporarily unavailable')).toBe(true);
    } finally { peer?.dispose(); f.dispose(); placements.dispose(); for (const source of sources) await source.dispose(); }
}
export const ExternalProviderCheckpoint: Story = {
    render: () => '<section aria-label="External provider worker checkpoint"></section>',
    play: ({ canvasElement }) => externalProviderCheckpointContract(canvasElement, false),
};
export const ExternalProviderCheckpointFallback: Story = {
    render: () => '<section aria-label="External provider fallback checkpoint"></section>',
    play: ({ canvasElement }) => externalProviderCheckpointContract(canvasElement, true),
};

export const StandaloneDelegateOwnership: Story = {
    render: () => '<section aria-label="Standalone delegate checkpoint ownership"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            let ready = true, race = false;
            const listeners = new Set<() => void>();
            const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
                const sealed = wasm.sealCommittedCheckpoint(candidate, receipt);
                if (race) ready = false;
                return sealed;
            });
            let controller: ReturnType<typeof connectCemManualListbox> | undefined;
            try {
                const field = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control', { value: 'Unsaved draft' });
                const host = await f.create('{div @role=listbox @popover=manual | {div @role=option | Choice}}');
                await f.runtime.whenRenderSettled(field); await f.runtime.whenRenderSettled(host);
                const panel = host.querySelector<HTMLElement>('[role=listbox]')!, input = field.querySelector('input')!;
                const provider = getCemEditorProvider(field)!;
                const lifecycle = { current: () => ready, ready: () => ready, subscribe: (listener: () => void) => { listeners.add(listener); return () => listeners.delete(listener); } };
                controller = connectCemManualListbox(panel, { host, editorHost: field, lifecycle });
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                controller.disconnect();
                controller = connectCemManualListbox(panel, { host, editorHost: field, lifecycle: { ...lifecycle, checkpoint: true } });
                await waitFor(async () => { await f.coordinator.capture(); });
                let saved = decode(f.coordinator); expect(saved.owners.some(owner => owner.island.includes('manualListbox'))).toBe(true);
                expect(saved.owners.some(owner => owner.island.includes('reset-on-resume'))).toBe(true);
                input.focus(); input.setSelectionRange(1, 5);
                expect(controller.open()).toBe(true);
                await waitFor(async () => { await f.coordinator.capture(); });
                expect(document.activeElement).toBe(input); expect(input.value).toBe('Unsaved draft');
                expect([input.selectionStart, input.selectionEnd]).toEqual([1, 5]);
                const foreign = provider.lease({});
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign.release();
                await waitFor(async () => { await f.coordinator.capture(); });
                controller.dismiss('fixture');
                const pending = controller.prepare()!; expect(pending).toBeTruthy();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); controller.dismiss('cancel');
                await waitFor(async () => { await f.coordinator.capture(); });
                const lastGood = f.coordinator.lastCommitted()!.bytes;
                race = true;
                await expect(f.coordinator.capture()).rejects.toThrow('stale');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(lastGood);
                race = false;
                for (const listener of listeners) listener();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                ready = true;
                await waitFor(async () => { await f.coordinator.capture(); });
                controller.disconnect(); controller = undefined;
                await waitFor(async () => { await f.coordinator.capture(); });
                saved = decode(f.coordinator); expect(saved.owners.some(owner => owner.island.includes('manualListbox'))).toBe(false);
                expect(listeners.size).toBe(0);
            } finally { controller?.disconnect(); f.dispose(); }
        }
    },
};
export const DeferredBodyProjectionOwnership: Story = {
    render: () => '<section aria-label="Deferred body checkpoint projection"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            let raceTarget: HTMLElement | undefined;
            const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
                const sealed = wasm.sealCommittedCheckpoint(candidate, receipt);
                raceTarget?.setAttribute('title', 'Changed while sealing');
                return sealed;
            });
            try {
                const host = await f.create('{attribute @name=caption | Initial}{dialog @part=surface @aria-label=Deferred | {template @slot=body | {input @value={$caption}}{p | {$caption}}}}', 'cem-ml', root, undefined, 'native-surface');
                await f.runtime.whenRenderSettled(host);
                await waitFor(async () => { await f.coordinator.capture(); });
                let saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('rematerialize'); expect(host.querySelector('input')).toBeNull();
                const templateInput = host.querySelector<HTMLTemplateElement>('template[slot=body]')!.content.querySelector('input')!;
                const initial = f.coordinator.lastCommitted()!.bytes;
                templateInput.setAttribute('value', 'Forged projection');
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(initial);
                templateInput.setAttribute('value', 'Initial');
                await waitFor(async () => { await f.coordinator.capture(); });
                const dialog = host.querySelector('dialog')!; dialog.showModal();
                const input = await waitFor(() => { const value = dialog.querySelector('input'); expect(value).not.toBeNull(); return value!; });
                input.value = 'Unsaved retained é'; input.focus(); input.setSelectionRange(2, 6);
                await waitFor(async () => { await f.coordinator.capture(); });
                saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('Unsaved retained é'); expect(saved).toContain('materialized');
                expect([input.selectionStart, input.selectionEnd]).toEqual([2, 6]);
                host.setAttribute('title', 'Unrelated render'); await f.runtime.whenRenderSettled(host);
                expect(dialog.querySelector('input')).toBe(input); expect(input.value).toBe('Unsaved retained é');
                expect(document.activeElement).toBe(input);
                await waitFor(async () => { await f.coordinator.capture(); });
                const beforeRace = f.coordinator.lastCommitted()!.bytes;
                raceTarget = dialog.querySelector('p')!;
                await expect(f.coordinator.capture()).rejects.toThrow('stale');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(beforeRace);
                raceTarget.removeAttribute('title'); raceTarget = undefined;
                await waitFor(async () => { await f.coordinator.capture(); });
                dialog.close(); await waitFor(async () => { await f.coordinator.capture(); });
                expect(dialog.querySelector('input')).toBe(input);
                let lastGood = f.coordinator.lastCommitted()!.bytes;
                const foreign = document.createElement('span'); dialog.append(foreign);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(lastGood); foreign.remove();
                await waitFor(async () => { await f.coordinator.capture(); });
                lastGood = f.coordinator.lastCommitted()!.bytes;
                host.setAttribute('caption', 'Changed'); await f.runtime.whenRenderSettled(host);
                expect(dialog.querySelector('input')).toBe(input); expect(input.value).toBe('Unsaved retained é');
                await waitFor(async () => { await f.coordinator.capture(); });
                saved = decode(f.coordinator).owners[0].island;
                expect(saved).toContain('cem-render-projection/1'); expect(saved).toContain('Initial'); expect(saved).toContain('Changed');
                expect(saved).toContain('Unsaved retained é');
                host.setAttribute('materialize', 'dispose'); await f.runtime.whenRenderSettled(host);
                dialog.showModal(); await waitFor(() => expect(dialog.open).toBe(true)); dialog.close();
                await waitFor(() => expect(dialog.querySelector('input')).toBeNull());
                await waitFor(async () => { await f.coordinator.capture(); });
                dialog.showModal();
                await waitFor(() => expect(dialog.querySelector('input')!.value).toBe('Changed'));
                await waitFor(async () => { await f.coordinator.capture(); });
                dialog.close(); await waitFor(() => expect(dialog.querySelector('input')).toBeNull());
                const empty = await f.create('{dialog @part=surface @aria-label=Empty | {template @slot=body}}', 'cem-ml', root, undefined, 'native-surface');
                await f.runtime.whenRenderSettled(empty); empty.querySelector('dialog')!.showModal();
                await waitFor(async () => { await f.coordinator.capture(); });
                empty.querySelector('dialog')!.close();
                await waitFor(async () => { await f.coordinator.capture(); });
            } finally { f.dispose(); }
        }
    },
};

export const NestedRetainedBodyOwners: Story = {
    render: () => '<section aria-label="Nested retained body checkpoints"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            let raceTarget: HTMLInputElement | undefined;
            const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
                const sealed = wasm.sealCommittedCheckpoint(candidate, receipt);
                if (raceTarget) raceTarget.value = 'Changed while sealing';
                return sealed;
            });
            try {
                const fieldDefinition = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control');
                const fieldTag = fieldDefinition.localName; await f.runtime.whenRenderSettled(fieldDefinition); fieldDefinition.remove();
                const childDefinition = await f.create('{dialog @part=surface @aria-label=Inner | {template @slot=body | {input @value="Inner original"}}}', 'cem-ml', root, undefined, 'native-surface');
                const childTag = childDefinition.localName; await f.runtime.whenRenderSettled(childDefinition); childDefinition.remove();
                const host = await f.create(`{attribute @name=caption | Original}{attribute @name=shown | true}{dialog @part=surface @aria-label=Outer | {template @slot=body | {${fieldTag} @value={$caption}}{${childTag}}{cem:if @test='$shown == "true"' | {p | {$caption}}}}}`, 'cem-ml', root, undefined, 'native-surface');
                await f.runtime.whenRenderSettled(host); const outer = host.querySelector('dialog')!; outer.showModal();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                const field = await waitFor(() => { const child = outer.querySelector<HTMLElement>(fieldTag); expect(child?.querySelector('input')).toBeTruthy(); return child!; });
                const child = outer.querySelector<HTMLElement>(childTag)!;
                await f.runtime.whenRenderSettled(field); await f.runtime.whenRenderSettled(child);
                const input = field.querySelector('input')!; input.value = 'Independent draft é'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await f.runtime.whenRenderSettled(field); input.focus(); input.setSelectionRange(2, 7);
                await waitFor(async () => { await f.coordinator.capture(); });
                expect(decode(f.coordinator).owners).toHaveLength(3);
                const hostMarker = field.getAttribute('data-cem-render-node-id')!;
                field.setAttribute('data-cem-render-node-id', 'forged');
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); field.setAttribute('data-cem-render-node-id', hostMarker);
                await waitFor(async () => { await f.coordinator.capture(); });
                input.dispatchEvent(new CompositionEvent('compositionstart', { bubbles: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new CompositionEvent('compositionend', { bubbles: true }));
                await f.runtime.whenRenderSettled(field); await waitFor(async () => { await f.coordinator.capture(); });
                expect(decode(f.coordinator).owners.some(owner => owner.island.includes('Independent draft é'))).toBe(true);
                host.setAttribute('caption', 'Latest template'); host.setAttribute('shown', 'false'); await f.runtime.whenRenderSettled(host);
                expect(outer.querySelector(fieldTag)).toBe(field); expect(outer.querySelector('p')!.textContent).toBe('Original');
                await waitFor(async () => { await f.coordinator.capture(); });
                expect(input.value).toBe('Independent draft é'); expect(document.activeElement).toBe(input);
                expect([input.selectionStart, input.selectionEnd]).toEqual([2, 7]);
                const inner = child.querySelector('dialog')!; inner.showModal();
                const innerInput = await waitFor(() => { const control = inner.querySelector('input'); expect(control).toBeTruthy(); return control!; });
                innerInput.value = 'Inner draft'; innerInput.focus();
                await waitFor(async () => { await f.coordinator.capture(); });
                const beforeRace = f.coordinator.lastCommitted()!.bytes; raceTarget = input;
                await expect(f.coordinator.capture()).rejects.toThrow('stale');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(beforeRace); raceTarget = undefined;
                input.value = 'Independent draft é'; await waitFor(async () => { await f.coordinator.capture(); });
                const foreign = document.createElement('span'); outer.append(foreign);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign.remove();
                await waitFor(async () => { await f.coordinator.capture(); });
                // An original produced host cannot be substituted by a same-tag island.
                const next = field.nextSibling; field.remove();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                const counterfeit = document.createElement(fieldTag); counterfeit.setAttribute('data-cem-render-node-id', field.getAttribute('data-cem-render-node-id')!);
                outer.insertBefore(counterfeit, next); await f.runtime.whenRenderSettled(counterfeit);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); counterfeit.remove(); outer.insertBefore(field, next);
                await f.runtime.whenRenderSettled(field); await waitFor(async () => { await f.coordinator.capture(); });
                inner.close(); outer.close(); await waitFor(async () => { await f.coordinator.capture(); });
                expect(outer.querySelector(fieldTag)).toBe(field); expect(inner.querySelector('input')).toBe(innerInput);
            } finally { f.dispose(); }
        }
    },
};

async function retainedNativeCapsulesContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector('section')!;
    let references = elementReferenceFixtureInputs('{option @value=alpha @label=Alpha}{option @value=beta @label=Beta}');
    let race: (() => void) | undefined;
    const f = fixture(root, fallback, false, undefined, { elementReferenceInputs: host => host.hasAttribute('capsule-source') ? references : undefined },
        async (candidate, receipt) => { const sealed = wasm.sealCommittedCheckpoint(candidate, receipt); race?.(); race = undefined; return sealed; });
    try {
        const childDefinition = await f.create('{input @value=Child}');
        await f.runtime.whenRenderSettled(childDefinition); const childTag = childDefinition.localName; childDefinition.remove();
        const host = await f.create(`{slice @name=relation}{slice @name=destination}{attribute @name=caption | Original}{dialog @part=surface @aria-label=Capsules | {template @slot=body | {div @native-count={42} @native-source={$destination}}{div @native-flag={false}}{input @value={$caption}}{p | {$caption}}{${childTag} @native-model={true}}}}`, 'cem-ml', root, undefined, 'native-surface', { 'capsule-source': '' });
        await f.runtime.whenRenderSettled(host);
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture(); const dialog = host.querySelector('dialog')!;
        const template = host.querySelector<HTMLTemplateElement>('template[slot=body]')!;
        const original = checkpointRenderedTemplateProjection(template)!;
        expect(original.nativeArtifacts).toHaveLength(1); // All roots share the native render artifact.
        const inertBinding = renderedNativeAttributeBindings(template.content.querySelector('div')!)[0];
        const inertIndex = inertBinding.value.index; inertBinding.value.index = 999;
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); inertBinding.value.index = inertIndex; await capture();
        const capsule = original.entries.find(entry => entry.kind === 'element' && entry.attributes.some(a => a.name === 'native-count'))!;
        if (capsule.kind !== 'element') throw new Error('missing capsule');
        const count = capsule.attributes.find(a => a.name === 'native-count')!.nativeValue!;
        const bytes = Uint8Array.from(atob(original.nativeArtifacts![count.artifact].base64), c => c.charCodeAt(0));
        expect(wasm.exportCemJsonValue(bytes, count.index, 'native-count', '')).toBe('42');
        const imported = wasm.importNativeValueArtifact(bytes, JSON.stringify({maxBytes:262144,maxValues:4096,maxDepth:64}));
        expect(wasm.disposeNativeValueArtifact(imported)).toBe(true);
        dialog.showModal(); const input = await waitFor(() => { const field = dialog.querySelector('input'); expect(field).not.toBeNull(); return field!; });
        const target = dialog.querySelector<HTMLElement>('div[native-count]')!;
        const child = await waitFor(() => { const nested = dialog.querySelector<HTMLElement>(childTag); expect(nested?.querySelector('input')).toBeTruthy(); return nested!; });
        await f.runtime.whenRenderSettled(child); const childInput = child.querySelector('input')!; childInput.value = 'Independent child draft';
        input.value = 'Unsaved capsule draft é'; input.focus(); input.setSelectionRange(2, 8); await capture();
        expect(decode(f.coordinator).owners).toHaveLength(2);
        expect(f.coordinator.current()!.receipt.relationships!.some(edge => edge.provider.instanceId === f.runtime.readInstanceSnapshot(child).instanceId)).toBe(true);
        expect(decode(f.coordinator).owners.find(owner => owner.instanceId === f.runtime.readInstanceSnapshot(host).instanceId)!.island).toContain('nativeArtifacts');
        expect(decode(f.coordinator).owners.some(owner => owner.island.includes('cem-native-source/1'))).toBe(true);
        host.setAttribute('caption', 'Current template'); await f.runtime.whenRenderSettled(host); await capture();
        expect(dialog.querySelector('input')).toBe(input); expect(input.value).toBe('Unsaved capsule draft é');
        expect(document.activeElement).toBe(input); expect([input.selectionStart, input.selectionEnd]).toEqual([2, 8]);
        references = elementReferenceFixtureInputs('{option @value=gamma @label=Gamma}{option @value=delta @label=Delta}');
        f.runtime.refreshElementReferences(host); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        await f.runtime.whenRenderSettled(host); await capture();
        const currentTemplate = host.querySelector<HTMLTemplateElement>('template[slot=body]')!;
        expect(checkpointRenderedTemplateProjection(currentTemplate)!.nativeArtifacts![0].contentHash).not.toBe(original.nativeArtifacts![0].contentHash);
        expect(decode(f.coordinator).owners.some(owner => owner.island.includes(original.nativeArtifacts![0].base64))).toBe(true);
        expect(childInput.value).toBe('Independent child draft');
        expect(input.value).toBe('Unsaved capsule draft é'); expect(document.activeElement).toBe(input);
        const childBindings = renderedNativeAttributeBindings(child);
        const oldChildIndex = childBindings[0].value.index; childBindings[0].value.index = 999;
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); childBindings[0].value.index = oldChildIndex; await capture();
        const originalBindings = renderedNativeAttributeBindings(target);
        const binding = originalBindings.find(b => b.name === 'native-count')!;
        const bad = { ...binding.value, artifact: binding.value.artifact.slice(0) };
        new Uint8Array(bad.artifact)[bad.artifact.byteLength - 1] ^= 1;
        const prior = f.coordinator.lastCommitted()!.bytes;
        restoreNativeAttributeBindings(target, originalBindings.map(b => b === binding ? { name:b.name, value:bad } : b));
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(f.coordinator.lastCommitted()!.bytes).toEqual(prior);
        restoreNativeAttributeBindings(target, originalBindings); await capture();
        // Mutation in place cannot be hidden behind a claimed unchanged content hash.
        const actual = new Uint8Array(binding.value.artifact), last = actual.length - 1, byte = actual[last]; actual[last] ^= 1;
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); actual[last] = byte; await capture();
        const good = f.coordinator.lastCommitted()!.bytes;
        race = () => { new Uint8Array(binding.value.artifact)[last] ^= 1; };
        await expect(f.coordinator.capture()).rejects.toThrow('stale'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(good);
        actual[last] = byte; await capture();
        // Caller changes to a returned checkpoint cannot mutate the retained original bytes.
        original.nativeArtifacts![0].base64 = 'forged'; await capture();
        expect(input.value).toBe('Unsaved capsule draft é'); expect(document.activeElement).toBe(input);
        host.setAttribute('materialize', 'dispose'); await f.runtime.whenRenderSettled(host); dialog.close();
        await waitFor(() => expect(dialog.querySelector('input')).toBeNull()); await capture();
        dialog.showModal(); await waitFor(() => expect(dialog.querySelector('input')!.value).toBe('Current template')); await capture();
        dialog.close(); await capture();
    } finally { f.dispose(); }
}
export const RetainedNativeAttributeAndSourceCapsules: Story = {
    render: () => '<section aria-label="Retained native capsules worker checkpoint"></section>',
    play: ({ canvasElement }) => retainedNativeCapsulesContract(canvasElement, false),
};
export const RetainedNativeAttributeAndSourceCapsulesFallback: Story = {
    render: () => '<section aria-label="Retained native capsules fallback checkpoint"></section>',
    play: ({ canvasElement }) => retainedNativeCapsulesContract(canvasElement, true),
};

export const NativeDynamicBodyProjection: Story = {
    render: () => '<section aria-label="Native dynamic body projection transport"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        const bounds = { start: document.createComment('start'), end: document.createComment('end') }; root.append(bounds.start,bounds.end);
        const plan = (text: string): RenderPlan => ({ producedTag: 'fixture', instanceId: 'fixture', templateArtifactId: 'body', dataRevision: text,
            outputTarget: 'light-dom', scopePolicyStamp: 'test', nodes: [ { kind: 'element', namespace: null, tag: 'div', renderNodeId: 'owner', attributes: [], children: [
                { kind: 'element', namespace: null, tag: 'template', renderNodeId: 'template', attributes: [{ name: 'slot', value: 'body' }], children: [
                    { kind: 'text', renderNodeId: 'value', text }, { kind: 'element', namespace: null, tag: 'input', renderNodeId: 'editor', attributes: [{name:'value',value:'Initial'}], children: [] },
                ] },
            ] } ] });
        applyRenderPlanToRange(bounds, plan('Original dynamic'), document, { dynamicTextRanges: true });
        const owner = root.querySelector('div')!, body = new CemSurfaceBody(owner, owner, () => undefined);
        try {
            expect(body.mount()).toBe(true); const input = owner.querySelector('input')!; input.value = 'Draft';
            const retainedRoots = new Set([...owner.childNodes].filter(node => node !== owner.querySelector('template')));
            const before = body.checkpoint()!; expect(before.projection!.entries.some(entry => entry.kind === 'range-start')).toBe(true);
            expect(before.projection!.origin.dataRevision).toBe('Original dynamic');
            applyRenderPlanToRange(bounds, plan('Current dynamic'), document, { dynamicTextRanges: true, preserveNode: node => retainedRoots.has(node as ChildNode) });
            const after = body.checkpoint()!; expect(after.projection).toEqual(before.projection); expect(input.value).toBe('Draft');
            const textEntry = before.projection!.entries.find(entry => entry.kind === 'text');
            if (textEntry?.kind === 'text') textEntry.text = 'Caller mutation';
            expect(body.checkpoint()!.projection!.entries.some(entry => entry.kind === 'text' && entry.text === 'Original dynamic')).toBe(true);
            const dto = { version: 'cem-checkpoint-runtime/1', resources: [], writers: [], writeDisposition: 'suppress-until-new-input',
                capability: { kind: 'native-surface', phase: 'closed', defaultsInitialized: true, state: { body: after,
                    returnValue: null, modal: false, focusWithin: false, closeReason: 'native', invocation: null, disposition: 'reacquire' } } };
            const saved = wasm.validateCheckpointRuntime(JSON.stringify(dto)); expect(wasm.validateCheckpointRuntime(saved)).toBe(saved);
            const marker = [...owner.childNodes].find(node => node.nodeType === 8)!; const old = marker.textContent;
            marker.textContent = 'cem-start:forged'; expect(body.checkpoint()).toBeUndefined(); marker.textContent = old;
            expect(body.checkpoint()).toBeDefined();
            const foreign = document.createComment('cem-end:forged'); owner.append(foreign); expect(body.valid()).toBe(true); expect(body.checkpoint()).toBeUndefined(); foreign.remove();
            const replacement = plan('Replacement template');
            if (replacement.nodes[0].kind === 'element' && replacement.nodes[0].children[0].kind === 'element') replacement.nodes[0].children[0].renderNodeId = 'replacement-template';
            applyRenderPlanToRange(bounds, replacement, document, { dynamicTextRanges: true, preserveNode: node => retainedRoots.has(node as ChildNode) });
            expect(body.checkpoint()!.reuseOnOpen).toBe(false); expect(input.value).toBe('Draft');
            body.mount(); expect(body.checkpoint()!.reuseOnOpen).toBe(true); expect(owner.querySelector('input')).not.toBe(input);
            body.dispose(); expect(body.checkpoint()!.materialized).toBe(false);
            body.mount(); expect(body.checkpoint()!.projection!.entries.some(entry => entry.kind === 'text' && entry.text === 'Replacement template')).toBe(true);
            body.dispose();
            const privatePlan = plan('Private input');
            const frame = privatePlan.nodes[0];
            if (frame.kind === 'element' && frame.children[0].kind === 'element' && frame.children[0].children[1].kind === 'element') frame.children[0].children[1].attributes.push({ name: 'type', value: 'password' });
            applyRenderPlanToRange(bounds, privatePlan, document, { dynamicTextRanges: true });
            body.mount(); expect(body.checkpoint()).toBeUndefined();
        } finally { body.dispose(); root.replaceChildren(); }
    },
};

async function standaloneDatalistCheckpointContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector('section')!;
    let ready = true, race = false, publication: CemNativeSuggestionsPublication | undefined;
    const listeners = new Set<() => void>(), sources: Awaited<ReturnType<typeof externalCheckpointSource>>[] = [];
    const f = fixture(root, fallback, false, undefined, { nativeSuggestionsInputs(instance, snapshot) {
        const current = publication;
        return current && instance.hasAttribute('native-options-frame') ? [current.bind({ instanceId: snapshot.instanceId,
            scopePolicyStamp: snapshot.scopePolicyStamp, revision: snapshot.dataRevision,
            current: () => ready && publication === current && instance.isConnected })] : [];
    } }, async (candidate, receipt) => { const sealed = wasm.sealCommittedCheckpoint(candidate, receipt); if (race) ready = false; return sealed; });
    let attachment: ReturnType<typeof connectCemStandaloneDatalist> | undefined;
    try {
        let source = await externalCheckpointSource(fallback, true, undefined, true); sources.push(source);
        publication = await source.session.publishDatalist({}, () => true);
        const field = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control', { value: 'Unsaved draft' });
        const host = await f.create('{datalist | {cem:for-each @select=datadom.slices.suggestions.children @as=row | {option @value={row.dom:attribute("value").value} @label={row.dom:attribute("label").value}}}}', 'cem-ml', root, undefined, undefined, { 'native-options-frame': '' });
        await f.runtime.whenRenderSettled(field); await f.runtime.whenRenderSettled(host);
        const input = field.querySelector('input')!, target = host.querySelector('datalist')!, provider = getCemEditorProvider(field)!;
        const lifecycle = { current: () => ready, ready: () => ready, subscribe: (listener: () => void) => { listeners.add(listener); return () => listeners.delete(listener); } };
        const connect = (checkpoint = true) => connectCemStandaloneDatalist({ runtime: f.runtime, host, editorHost: field, target, lifecycle: { ...lifecycle, checkpoint } });
        expect(() => connectCemStandaloneDatalist({ runtime: f.runtime, host, editorHost: field,
            target: document.createElement('datalist'), lifecycle })).toThrow('owned target');
        expect(() => connectCemStandaloneDatalist({ runtime: f.runtime, host, editorHost: field, target,
            lifecycle: { ...lifecycle, checkpoint: true, subscribe: () => { throw new Error('failed lifecycle'); } } })).toThrow('failed lifecycle');
        attachment = connect(false); await waitFor(() => expect(input.list).toBe(target));
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); attachment.disconnect();
        attachment = connect();
        expect(() => connect()).toThrow('Duplicate standalone');
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture(); const saved = decode(f.coordinator);
        expect(saved.owners.some(owner => owner.island.includes('standaloneDatalist'))).toBe(true);
        expect(saved.owners.some(owner => owner.island.includes('reset-on-resume'))).toBe(true);
        expect(saved.owners.some(owner => owner.island.includes('sessionKey'))).toBe(false);
        expect(f.coordinator.current()!.receipt.relationships?.filter(item => item.kind === 'editor-provider')).toHaveLength(1);
        expect(f.coordinator.current()!.receipt.relationships?.some(item => item.kind === 'placement' && item.property === 'interaction')).toBe(true);
        const capsuleHost = await f.create('{datalist @native-options={1} | {cem:for-each @select=datadom.slices.suggestions.children @as=row | {option @value={row.dom:attribute("value").value} @label={row.dom:attribute("label").value}}}}', 'cem-ml', root, undefined, undefined, { 'native-options-frame': '' });
        try {
            await f.runtime.whenRenderSettled(capsuleHost);
            const capsuleTarget = capsuleHost.querySelector('datalist')!;
            expect(renderedNativeAttributeBindings(capsuleTarget)).toHaveLength(1);
            expect(checkpointRenderedDatalistOrigin(capsuleTarget)).toBeUndefined();
            await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        } finally { capsuleHost.remove(); }
        await capture();
        input.focus(); input.value = 'Edited draft'; input.dispatchEvent(new Event('input', { bubbles: true }));
        await f.runtime.whenRenderSettled(field);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        input.setSelectionRange(2, 8); input.blur(); await capture(); expect(input.value).toBe('Edited draft'); expect(input.list).toBe(target);
        expect([input.selectionStart, input.selectionEnd]).toEqual([2, 8]);
        field.setAttribute('disabled', ''); await f.runtime.whenRenderSettled(field); await capture();
        expect(decode(f.coordinator).owners.some(owner => owner.island.includes('unavailable'))).toBe(true);
        field.removeAttribute('disabled'); await f.runtime.whenRenderSettled(field); attachment.reconcile(); await capture();
        const foreign = provider.lease({}); await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign.release();
        attachment.reconcile(); await capture();
        const option = target.options[0], label = option.label;
        option.label = 'Foreign label'; attachment.reconcile(); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        option.label = label; attachment.reconcile(); await capture();
        const forged = document.createElement('option'); forged.value = 'foreign'; target.append(forged);
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); forged.remove(); attachment.reconcile(); await capture();
        input.setAttribute('list', 'authored'); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        expect(input.getAttribute('list')).toBe('authored'); input.removeAttribute('list'); attachment.reconcile(); await capture();
        const copied = await externalCheckpointSource(fallback, true, source.session.checkpointSource(), true); sources.push(copied);
        const copiedPublication = await copied.session.publishDatalist({}, () => true);
        expect(copied.session.suggestions?.rows).toBe(2);
        const copiedBinding = copiedPublication.bind({ instanceId: 'fresh-datalist', scopePolicyStamp: 'fresh-policy', revision: '1', current: () => true });
        try {
            const output = await copiedBinding.render('{datalist | {cem:for-each @select=suggestions.children @as=row | {option @value={row.dom:attribute("value").value} @label={row.dom:attribute("label").value}}}}', {});
            expect(output.diagnostics).toEqual([]); expect(JSON.stringify(output.nodes)).toContain('Another');
        } finally { copiedBinding.release(); await copiedPublication.release(); }
        const last = f.coordinator.lastCommitted()!.bytes; race = true;
        await expect(f.coordinator.capture()).rejects.toThrow('stale'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        race = false; await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        ready = true; for (const listener of listeners) listener();
        // A revoked native binding never revives: replace source/frame and reacquire its exact target owner.
        attachment.disconnect(); attachment = undefined; await source.session.release();
        source = await externalCheckpointSource(fallback, true, undefined, true); sources.push(source);
        publication = await source.session.publishDatalist({}, () => true);
        f.runtime.refreshElementReferences(host); await f.runtime.whenRenderSettled(host);
        const replacementOrigin = checkpointRenderedDatalistOrigin(host.querySelector('datalist')!);
        const replacementSnapshot = f.runtime.readInstanceSnapshot(host);
        expect(replacementOrigin).toMatchObject({ instanceId: replacementSnapshot.instanceId, templateArtifactId: replacementSnapshot.templateArtifactId, scopePolicyStamp: replacementSnapshot.scopePolicyStamp });
        expect(host.querySelector('datalist')).toBe(target);
        expect(replacementOrigin?.dataRevision).not.toBe(replacementSnapshot.dataRevision);
        attachment = connectCemStandaloneDatalist({ runtime: f.runtime, host, editorHost: field, target: host.querySelector('datalist')!, lifecycle: { ...lifecycle, checkpoint: true } });
        await capture(); expect(input.value).toBe('Edited draft');
        attachment.disconnect(); attachment = undefined; expect(input.hasAttribute('list')).toBe(false);
        expect(listeners.size).toBe(0); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        const opaque = await externalCheckpointSource(fallback, false, undefined, true); sources.push(opaque);
        publication = await opaque.session.publishDatalist({}, () => true); f.runtime.refreshElementReferences(host); await f.runtime.whenRenderSettled(host);
        attachment = connectCemStandaloneDatalist({ runtime: f.runtime, host, editorHost: field, target: host.querySelector('datalist')!, lifecycle: { ...lifecycle, checkpoint: true } });
        await waitFor(() => expect(input.list?.options.length).toBe(2)); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        attachment.disconnect(); attachment = undefined; host.remove(); await capture();
    } finally { attachment?.disconnect(); f.dispose(); for (const source of sources) await source.dispose(); }
}
export const StandaloneDatalistOwnership: Story = {
    render: () => '<section aria-label="Standalone datalist checkpoint ownership"></section>',
    play: ({ canvasElement }) => standaloneDatalistCheckpointContract(canvasElement, false),
};
export const StandaloneDatalistOwnershipFallback: Story = {
    render: () => '<section aria-label="Standalone datalist fallback checkpoint ownership"></section>',
    play: ({ canvasElement }) => standaloneDatalistCheckpointContract(canvasElement, true),
};

function nativeCheckpointFixtureValue(): NativeCemValue {
    const compiled = JSON.parse(wasm.compileTemplate('{div @native-count={42}}', '[]'));
    try {
        const plan = JSON.parse(wasm.renderTemplateWithNativeValues(compiled.artifactId, 0, '{}', '[]', '[]', '', 'native-context-fixture', undefined, undefined));
        expect(plan.diagnostics).toEqual([]);
        return {kind:'cem-native-value-v1',artifact:wasm.takeRenderValueArtifact(plan.nativeValueArtifactId).slice().buffer,
            contentHash:plan.nativeValueContentHash,index:plan.nodes[0].attributes[0].nativeValueIndex};
    } finally { wasm.disposeTemplate(compiled.artifactId); }
}
export const NativePreparationAndBindingsCheckpoint: Story = {
    render: () => '<section aria-label="Native preparation and bindings worker checkpoint"></section>',
    play: ({ canvasElement }) => nativePublicationCheckpointContract(canvasElement, false, true),
};
export const NativePreparationAndBindingsCheckpointFallback: Story = {
    render: () => '<section aria-label="Native preparation and bindings fallback checkpoint"></section>',
    play: ({ canvasElement }) => nativePublicationCheckpointContract(canvasElement, true, true),
};
async function nativeContextCheckpointContract(canvasElement: HTMLElement, fallback: boolean) {
    await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
    let fence: (() => void) | undefined;
    const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
        const sealed = wasm.sealCommittedCheckpoint(candidate, receipt); fence?.(); fence = undefined; await Promise.resolve(); return sealed;
    });
    try {
        const host = await f.create('{dialog @part=surface @aria-label=Context | {input @value=Original}{button | Close}}', 'cem-ml', root, undefined, 'native-surface');
        await f.runtime.whenRenderSettled(host);
        const dialog = host.querySelector('dialog')!, input = dialog.querySelector('input')!;
        const controller = connectCemNativeSurface(dialog, {host});
        const context = nativeCheckpointFixtureValue();
        const originalBytes = context.artifact.slice(0), originalIndex = context.index;
        expect(controller.open({context,contextKey:'item:42'})).toBe(true);
        await waitFor(() => expect(dialog.open).toBe(true));
        input.focus(); input.value = 'Unsaved é'; input.setSelectionRange(1,4);
        const capture = () => waitFor(async () => { await f.coordinator.capture(); });
        await capture();
        const saved = decode(f.coordinator).owners[0].island;
        expect(saved).toContain('contentHash'); expect(saved).toContain('item:42'); expect(saved).toContain('Unsaved é'); expect(saved).not.toContain('sessionKey');
        expect(document.activeElement).toBe(input); expect([input.selectionStart,input.selectionEnd]).toEqual([1,4]);
        const last = f.coordinator.lastCommitted()!.bytes;
        context.index = 999;
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
        context.index = originalIndex; await capture();
        new Uint8Array(context.artifact)[0] ^= 1;
        await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
        new Uint8Array(context.artifact).set(new Uint8Array(originalBytes)); await capture();
        const beforeRace = f.coordinator.lastCommitted()!.bytes;
        fence = () => { context.index = 999; };
        await expect(f.coordinator.capture()).rejects.toThrow('stale'); expect(f.coordinator.lastCommitted()!.bytes).toEqual(beforeRace);
        context.index = originalIndex; await capture();
        expect(input.value).toBe('Unsaved é'); expect(document.activeElement).toBe(input);
        dialog.close(); await capture();
        expect(decode(f.coordinator).owners[0].island).not.toContain('item:42');
        const keeper = await f.create('{p | Remaining owner}'); await f.runtime.whenRenderSettled(keeper);
        host.remove(); await capture(); expect(f.coordinator.current()!.receipt.owners).toHaveLength(1);
        expect(decode(f.coordinator).owners[0].island).not.toContain('item:42');
    } finally { f.dispose(); }
}
export const NativeInvocationContextCheckpoint: Story = {
    render: () => '<section aria-label="Native invocation context worker checkpoint"></section>',
    play: ({ canvasElement }) => nativeContextCheckpointContract(canvasElement, false),
};
export const NativeInvocationContextCheckpointFallback: Story = {
    render: () => '<section aria-label="Native invocation context fallback checkpoint"></section>',
    play: ({ canvasElement }) => nativeContextCheckpointContract(canvasElement, true),
};

export const CrossRuntimeGeometryAndFocusCheckpoint: Story = {
    render: () => '<section aria-label="Cross runtime geometry and focus checkpoint"></section>',
    play: async ({ canvasElement }) => { for (const fallback of [false, true]) await explicitSubmenuCheckpointContract(canvasElement, fallback, true); },
};
export const CrossRuntimeSuggestionsProviderCheckpoint: Story = {
    render: () => '<section aria-label="Cross runtime suggestions provider checkpoint"></section>',
    play: async ({ canvasElement }) => { for (const fallback of [false, true]) await externalProviderCheckpointContract(canvasElement, fallback, true); },
};

export const CrossRuntimeCustomSourcePlacement: Story = {
    render: () => '<section aria-label="Cross runtime custom source placement checkpoint"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            const placements = new CemElementPlacementCoordinator(document), references = elementReferenceFixtureInputs();
            const f = fixture(root, fallback, false, undefined, { placementCoordinator: placements, elementReferenceInputs: () => references });
            const peerRoot = root.appendChild(document.createElement('div'));
            const peer = fixture(peerRoot, !fallback, false, undefined, { placementCoordinator: placements }, undefined, f.coordinator);
            Object.assign(references.sources[0], { maxDepth: 128, maxWork: 100000, unresolved: 'warning' });
            Object.assign(references.sources[1], { maxDepth: 32, maxWork: 700, unresolved: 'ignore' });
            try {
                const producer = await peer.create(`{dialog @id=checkpoint-target-${crypto.randomUUID()} | Target}`); await peer.runtime.whenRenderSettled(producer);
                await f.coordinator.capture();
                const target = producer.querySelector('dialog')!, receipt = f.coordinator.current()!.receipt.owners[0];
                const lease = placements.register({ producer: receipt.instanceId, path: [0], revision: String(receipt.dataRevision), element: target,
                    source: references.sources[1], select: 'input.children', currentRevision: () => producer.isConnected ? String(receipt.dataRevision) : undefined });
                const consumer = await f.create(ELEMENT_REFERENCE_TEMPLATE.replace('{$destination}', '')); await f.runtime.whenRenderSettled(consumer);
                const revoke = placements.grant(f.runtime.snapshotInstance(consumer).instanceId, lease, ['commandfor']);
                f.runtime.refreshElementReferences(consumer); await f.runtime.whenRenderSettled(consumer);
                expect(consumer.querySelector('button')?.getAttribute('commandfor')).toBe(target.id);
                await f.coordinator.capture(); expect(f.coordinator.current()!.receipt.relationships).toHaveLength(1);
                expect(decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island).toContain('cemb');
                const saved = decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island;
                expect(saved).toContain('maxDepth'); expect(saved).toContain('100000'); expect(saved).toContain('warning');
                expect(f.coordinator.current()!.receipt.relationships![0].provider.instanceId).toBe(peer.runtime.readInstanceSnapshot(producer).instanceId);
                const last = f.coordinator.lastCommitted()!.bytes;
                producer.remove(); await expect(f.coordinator.capture()).rejects.toThrow();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(last);
                revoke(); lease.dispose(); consumer.remove();
                const repaired = await f.create('{p | Portable owner after revocation}'); await f.runtime.whenRenderSettled(repaired);
                await waitFor(async () => { await f.coordinator.capture(); });
                const button = repaired.querySelector('p');
                if (button) { button.setAttribute('data-cem-placement-ref-commandfor', ''); await expect(f.coordinator.capture()).rejects.toThrow(); }
            } finally { peer.dispose(); f.dispose(); placements.dispose(); }
        }
    },
};

export const CrossRuntimeStandaloneDelegateOwnership: Story = {
    render: () => '<section aria-label="Cross runtime standalone delegate ownership"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector('section')!;
        for (const fallback of [false, true]) {
            let ready = true, race = false;
            const listeners = new Set<() => void>();
            const f = fixture(root, fallback, false, undefined, {}, async (candidate, receipt) => {
                const sealed = wasm.sealCommittedCheckpoint(candidate, receipt);
                if (race) ready = false;
                return sealed;
            });
            const peerRoot = root.appendChild(document.createElement('div'));
            const peer = fixture(peerRoot, !fallback, false, undefined, {}, undefined, f.coordinator);
            let controller: ReturnType<typeof connectCemManualListbox> | undefined;
            try {
                const field = await f.create(declarationTemplate(fieldDeclaration), 'cem-ml', root, undefined, 'form-control', { value: 'Unsaved draft' });
                const host = await peer.create('{div @role=listbox @popover=manual | {div @role=option | Choice}}');
                await f.runtime.whenRenderSettled(field); await peer.runtime.whenRenderSettled(host);
                const panel = host.querySelector<HTMLElement>('[role=listbox]')!, input = field.querySelector('input')!;
                const provider = getCemEditorProvider(field)!;
                const lifecycle = { current: () => ready, ready: () => ready, subscribe: (listener: () => void) => { listeners.add(listener); return () => listeners.delete(listener); } };
                controller = connectCemManualListbox(panel, { host, editorHost: field, lifecycle });
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                controller.disconnect();
                controller = connectCemManualListbox(panel, { host, editorHost: field, lifecycle: { ...lifecycle, checkpoint: true } });
                await waitFor(async () => { await f.coordinator.capture(); });
                let saved = decode(f.coordinator); expect(saved.owners.some(owner => owner.island.includes('manualListbox'))).toBe(true);
                expect(saved.owners.some(owner => owner.island.includes('reset-on-resume'))).toBe(true);
                input.focus(); input.setSelectionRange(1, 5);
                expect(controller.open()).toBe(true);
                await waitFor(async () => { await f.coordinator.capture(); });
                expect(document.activeElement).toBe(input); expect(input.value).toBe('Unsaved draft');
                expect([input.selectionStart, input.selectionEnd]).toEqual([1, 5]);
                const foreign = provider.lease({});
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); foreign.release();
                await waitFor(async () => { await f.coordinator.capture(); });
                controller.dismiss('fixture');
                const pending = controller.prepare()!; expect(pending).toBeTruthy();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable'); controller.dismiss('cancel');
                await waitFor(async () => { await f.coordinator.capture(); });
                const lastGood = f.coordinator.lastCommitted()!.bytes;
                race = true;
                await expect(f.coordinator.capture()).rejects.toThrow('stale');
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(lastGood);
                race = false;
                for (const listener of listeners) listener();
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                ready = true;
                await waitFor(async () => { await f.coordinator.capture(); });
                controller.disconnect(); controller = undefined;
                await waitFor(async () => { await f.coordinator.capture(); });
                saved = decode(f.coordinator); expect(saved.owners.some(owner => owner.island.includes('manualListbox'))).toBe(false);
                expect(listeners.size).toBe(0);
            } finally { controller?.disconnect(); peer.dispose(); f.dispose(); }
        }
    },
};

export const CrossRuntimeEndpointOwnershipBoundaries: Story = {
    render: () => '<section aria-label="Cross runtime endpoint ownership boundaries"></section>',
    play: async ({ canvasElement }) => {
        await init(); const root = canvasElement.querySelector<HTMLElement>('section')!;
        for (const fallback of [false, true]) {
            const f = fixture(root, fallback);
            const peerRoot = root.appendChild(document.createElement('div'));
            const peer = fixture(peerRoot, !fallback, false, undefined, {}, undefined, f.coordinator);
            const foreignRoot = root.appendChild(document.createElement('div'));
            const foreign = fixture(foreignRoot, fallback);
            try {
                const parent = await f.create('{button @type=button | First}'); await f.runtime.whenRenderSettled(parent);
                const other = await peer.create('{button @type=button | Second}'); await peer.runtime.whenRenderSettled(other);
                const outsider = await foreign.create('{button @type=button | Foreign}', 'cem-ml', parent); await foreign.runtime.whenRenderSettled(outsider);
                expect(f.coordinator.endpoint(outsider)).toBeUndefined();
                expect(f.coordinator.endpoint(outsider.querySelector('button')!)).toBeUndefined();
                expect(foreign.coordinator.endpoint(other)).toBeUndefined();
                const button = parent.querySelector('button')!;
                expect(f.coordinator.endpoint(button)?.instanceId).toBe(f.runtime.readInstanceSnapshot(parent).instanceId);
                other.append(button); expect(f.coordinator.endpoint(button)).toBeUndefined(); parent.prepend(button);
                const boundary = document.createElement('div'), inert = document.createElement('template');
                inert.setAttribute('data-cem-island', 'instance'); boundary.append(inert); parent.append(boundary); boundary.append(button);
                expect(f.coordinator.endpoint(button)).toBeUndefined(); parent.prepend(button); boundary.remove();
                const before = f.coordinator.endpoint(other); expect(before?.nodeId).toBe('host');
                peer.scope.dispose(); expect(f.coordinator.endpoint(other)).toBeUndefined();
                expect(f.coordinator.endpoint(other.querySelector('button')!)).toBeUndefined();
                expect(() => f.coordinator.register('duplicate-host', () => '<template/>', () => true, 0, () => [],
                    { host: parent, resolve: () => ({ ...before!, instanceId: 'duplicate-host' }) })).toThrow('owners');
                expect(f.coordinator.endpoint(parent)).toBeUndefined();
            } finally { foreign.dispose(); peer.dispose(); f.dispose(); }
        }
    },
};
