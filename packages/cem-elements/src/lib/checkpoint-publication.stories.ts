import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
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
    async function create(source: string, mode: 'dom' | 'cem-ml' = 'cem-ml', container = root, payload?: string) {
        const tag = `checkpoint-${crypto.randomUUID()}`;
        const declaration = document.createElement('div'); declaration.setAttribute('tag', tag);
        const template = document.createElement('template');
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
                const unsupported = await f.create('{label | Unbound edit {input @name=draft}}');
                await f.runtime.whenRenderSettled(unsupported);
                const input = unsupported.querySelector('input')!; input.value = 'Keep this edit';
                input.dispatchEvent(new InputEvent('input', { bubbles: true }));
                await f.runtime.whenRenderSettled(unsupported);
                await expect(f.coordinator.capture()).rejects.toThrow('unavailable');
                expect(input.value).toBe('Keep this edit'); expect(f.coordinator.current()).toBeUndefined();
                expect(f.coordinator.lastCommitted()!.bytes).toEqual(good);
                unsupported.remove(); await f.coordinator.capture();
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
