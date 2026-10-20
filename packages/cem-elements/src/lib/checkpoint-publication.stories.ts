import { CemNativeCapabilitySession } from './native-capability-session.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
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
function fixture(root: HTMLElement, fallback: boolean, automatic = false, read?: Reader, options: Partial<CemElementRuntimeOptions> = {}) {
    const document = root.ownerDocument;
    const suffix = crypto.randomUUID();
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: `session-${suffix}`, generation: 1, policyStamp: 'whole-private-island:1',
        allowOwner: () => true, seal: (candidate, receipt) => wasm.sealCommittedCheckpoint(candidate, receipt), automatic });
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
    return { runtime, coordinator, scope, create, dispose() { root.replaceChildren(); scope.dispose(); coordinator.dispose(); } };
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
                const ready = () => waitFor(() => expect(cemSuggestionsCheckpoint(host)).toBeDefined());
                const capture = () => waitFor(async () => { await f.coordinator.capture(); });
                await ready();
                const revision = f.runtime.readInstanceSnapshot(host).dataRevision;
                expect(f.runtime.readInstanceSnapshot(host).dataRevision).toBe(revision);
                await capture(); expect(decode(f.coordinator).owners).toHaveLength(2);
                expect(f.coordinator.current()!.receipt.relationships?.some(edge => edge.kind === 'editor-provider')).toBe(true);
                input.focus(); input.value = 'same'; input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await ready(); await capture();
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
                await ready(); await capture(); expect(cemSuggestionsCheckpoint(host)?.activeIndex).toBe(0);
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
                await capture(); expect(cemSuggestionsCheckpoint(host)?.activeIndex).toBe(1);
                const listbox = host.querySelector<HTMLElement>('[part~=surface]')!;
                listbox.querySelector('[role=option]')!.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, pointerType: 'mouse', isPrimary: true, pointerId: 1, button: 0 }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                listbox.dispatchEvent(new PointerEvent('pointercancel', { bubbles: true, pointerId: 1 })); await capture();
                input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', code: 'Enter', bubbles: true, cancelable: true }));
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                input.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
                await ready(); await capture();
                expect(cemSuggestionsCheckpoint(host)?.committedIndex).toBe(1); expect(input.value).toBe('same');
                const saved = decode(f.coordinator).owners.find(owner => owner.island.includes('cem-native-source/1'))!.island;
                expect(saved).toContain('committedIndex'); expect(saved).not.toContain('sessionKey');
                const captured = cemSuggestionsCheckpoint(host)!;
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
