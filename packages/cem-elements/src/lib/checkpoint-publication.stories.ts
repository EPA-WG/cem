import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { ingestContractVersion } from './disposition.js';
import { mountCemView } from './host-view.js';
import sourceEditorDeclaration from '../../../cem-components/src/components/cem-source-editor/cem-source-editor.xhtml?raw';
import { createCemDeclarationScope } from './declaration-scope.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- real native checkpoint transport acceptance.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Checkpoint Publication', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
type Reader = NonNullable<Exclude<NonNullable<CemElementRuntimeOptions['retainedStylesheets']>, false>['read']>;
const decode = (c: CemCheckpointPublicationCoordinator) => JSON.parse(new TextDecoder().decode(c.current()!.bytes)) as { receipt: { owners: { acceptedEventSequence: number }[] }; owners: { instanceId: string; island: string }[] };
function fixture(root: HTMLElement, fallback: boolean, automatic = false, read?: Reader) {
    const document = root.ownerDocument;
    const suffix = crypto.randomUUID();
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: `session-${suffix}`, generation: 1, policyStamp: 'whole-private-island:1',
        allowOwner: () => true, seal: (candidate, receipt) => wasm.sealCommittedCheckpoint(candidate, receipt), automatic });
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationTag: `declaration-${suffix}`, declarationScope: scope, checkpointPublication: coordinator,
        moduleUrlRoot: { baseUrl: 'https://example.test/', importMap: {} }, retainedStylesheets: { read },
        ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture selects fallback'); } } : {}) });
    async function create(source: string, mode: 'dom' | 'cem-ml' = 'cem-ml', container = root, payload?: string, capability?: string) {
        const tag = `checkpoint-${crypto.randomUUID()}`;
        const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
        const template = document.createElement('template');
        if (capability) declaration.setAttribute('capability', capability);
        if (mode === 'cem-ml') { template.type = 'text/cem-ml'; template.textContent = source; } else template.innerHTML = source;
        declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
        await runtime.whenDeclarationSettled(declaration);
        const instance = document.createElement(tag);
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
                await f.runtime.whenRenderSettled(ambient); await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                ambient.remove(); await f.coordinator.capture();
                const effectOutcomes: string[] = [];
                const operation = f.coordinator.operation.bind(f.coordinator);
                f.coordinator.operation = (instanceId, operationId) => {
                    const effect = operation(instanceId, operationId); effectOutcomes.push('unknown');
                    return { acknowledge: () => { effectOutcomes.push('acknowledged'); effect.acknowledge(); } };
                };
                const writer = await f.create(`{local-storage @slice=draft @key="${storageKey}" @type=text @value=recorded}{p | Storage writer}`);
                await f.runtime.whenRenderSettled(writer); expect(localStorage.getItem(storageKey)).toBe('recorded');
                expect(effectOutcomes.length).toBeGreaterThanOrEqual(2);
                expect(effectOutcomes).toEqual(Array.from({ length: effectOutcomes.length / 2 }, () => ['unknown', 'acknowledged']).flat());
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                writer.remove();
                // The external effect belongs to the removed owner and must not
                // vanish just because that owner's resource object was released.
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(f.coordinator.lastCommitted()).toBeDefined();
                expect(localStorage.getItem(storageKey)).toBe('recorded');
            } finally { localStorage.removeItem(storageKey); f.dispose(); }
            const g = fixture(root, fallback);
            const previousUrl = root.ownerDocument.location.href;
            try {
                const base = await g.create('{p | Before rejected URL effect}');
                await g.runtime.whenRenderSettled(base); await g.coordinator.capture();
                const effectOutcomes: string[] = [];
                const operation = g.coordinator.operation.bind(g.coordinator);
                g.coordinator.operation = (instanceId, operationId) => {
                    const effect = operation(instanceId, operationId); effectOutcomes.push('unknown');
                    return { acknowledge: () => { effectOutcomes.push('acknowledged'); effect.acknowledge(); } };
                };
                const writer = await g.create('{location-element @method=history.replaceState @src="https://outside.example.invalid/"}{p | Rejected writer}');
                await g.runtime.whenRenderSettled(writer);
                expect(root.ownerDocument.location.href).toBe(previousUrl);
                expect(g.runtime.diagnosticsFor(writer).some(item => item.code === 'cem-element.location_write_failed')).toBe(true);
                expect(effectOutcomes).toEqual(['unknown']);
                await expect(g.coordinator.capture()).rejects.toThrow('unavailable'); writer.remove();
                await expect(g.coordinator.capture()).rejects.toThrow('unavailable');
                expect(g.coordinator.lastCommitted()).toBeDefined();
            } finally { g.dispose(); }
        }
    },
};
