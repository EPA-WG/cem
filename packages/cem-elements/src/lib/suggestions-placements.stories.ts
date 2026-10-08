import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession } from './native-capability-session.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { CemSuggestionsPlacementCoordinator } from './suggestions-placements.js';
import { getCemEditorProvider } from './form-control-capability.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixtures use the explicit native CEMB source boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Suggestions Placement Admissions', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
export const ExactNativeRowsRequireBothDirections: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        for (const fallback of [false, true]) {
            const suffix = crypto.randomUUID(), declarationTag = `placement-declaration-${suffix}`, tag = `placement-editor-${suffix}`;
            const runtime = new CemElementRuntime({ declarationTag, ...(fallback ? { processingWorkerFactory: () => { throw new Error('fallback fixture'); } } : {}) });
            runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag); declaration.setAttribute('capability', 'form-control');
            const template = document.createElement('template'); template.type = 'text/cem-ml';
            template.textContent = '{input @part=control @form="" @type=text @value={datadom.slices.value} @slice=value @slice-event=input @slice-value="$target.value"}';
            declaration.append(template); canvasElement.append(declaration); runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const field = document.createElement(tag), panel = document.createElement('div');
            panel.setAttribute('role', 'listbox'); panel.setAttribute('popover', 'manual');
            const first = document.createElement('div'), second = document.createElement('div');
            for (const [row, label] of [[first, 'First'], [second, 'Second']] as const) { row.setAttribute('role', 'option'); row.textContent = label; panel.append(row); }
            canvasElement.append(field, panel); await runtime.whenRenderSettled(field);
            const scope = createCemDeclarationScope({ document });
            const host = cemProcessingHostForScope(scope, { workerScriptUrl, ...(fallback ? { workerFactory: () => { throw new Error('fallback fixture'); } } : {}) });
            const source = wasm.parseReferenceSource(new TextEncoder().encode('{cem-option @value=same | First}{cem-option @value=same | Second}'), 'text/cem-ml', 'memory:rows.cem', '');
            const bundle = wasm.exportReferenceReloadBundle(source, '').slice().buffer as ArrayBuffer; wasm.disposeReferenceSource(source);
            const session = await CemNativeCapabilitySession.prepare(host, { action: 'prepare', adapter: 'suggestions-v1',
                handle: { sessionKey: suffix, instanceId: 'source', scopePolicyStamp: 'source', sourceRevision: '1' },
                sources: { kind: 'cem-native-session-sources-v1', requesting: 0, sources: [{ bundle, primarySourceId: 1, context: true }], bindings: [], grants: [] },
                data: {}, select: 'input.children', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS }, () => true);
            const publication = await session.publishSuggestions({ query: '', queryRevision: 1 }, () => true);
            const binding = publication.bind({ instanceId: 'listbox', scopePolicyStamp: 'listbox', revision: '1', current: () => true });
            const coordinator = new CemSuggestionsPlacementCoordinator(document);
            try {
                const native = await binding.rows();
                expect(native).toHaveLength(2); expect(native[0].value).toBe(native[1].value); expect(native[0].source).not.toBe(native[1].source);
                expect(() => JSON.stringify(native[0])).toThrow('cannot be serialized');
                expect(() => coordinator.register({ producer: 'listbox', revision: '1', current: () => true, element: first, kind: 'row', row: { ...native[0] } })).toThrow('Invalid');
                const editor = coordinator.register({ producer: 'field', revision: '1', current: () => true, element: field, kind: 'editor' });
                const listbox = coordinator.register({ producer: 'listbox', revision: '1', current: () => true, element: panel, kind: 'listbox' });
                const rows = [first, second].map((element, i) => coordinator.register({ producer: 'listbox', revision: '1', current: () => true, element, kind: 'row', row: native[i] }));
                expect(() => coordinator.prepare(binding, editor, listbox, rows)).toThrow('both directions'); expect(panel.id).toBe('');
                coordinator.grant('listbox', editor, ['editor-for']);
                expect(() => coordinator.prepare(binding, editor, listbox, rows)).toThrow('both directions');
                coordinator.grant('field', listbox, ['aria-controls']);
                const revoke = rows.map(row => coordinator.grant('field', row, ['aria-activedescendant']));
                const placements = coordinator.prepare(binding, editor, listbox, rows);
                expect(placements.current()).toBe(true); expect(placements.rows[0].native).toBe(native[0]); expect(panel.id).not.toBe('');
                const provider = getCemEditorProvider(field); if (!provider) throw new Error('Missing provider');
                const lease = provider.lease({});
                expect(lease.attributes.set({ revision: provider.revision, current: placements.current, listbox: panel, expanded: false, autocomplete: 'list' })).toBe(true);
                revoke[0](); expect(placements.current()).toBe(false); lease.attributes.refresh();
                expect(provider.control?.hasAttribute('aria-controls')).toBe(false); lease.release(); placements.release();
                coordinator.grant('field', rows[0], ['aria-activedescendant']);
                const fresh = coordinator.prepare(binding, editor, listbox, rows);
                first.remove(); coordinator.refresh(); expect(fresh.current()).toBe(false); panel.prepend(first);
                expect(fresh.current()).toBe(false); fresh.release();
                const next = await session.publishSuggestions({ query: 'Second', queryRevision: 2 }, () => true);
                const nextBinding = next.bind({ instanceId: 'listbox', scopePolicyStamp: 'listbox', revision: '2', current: () => true });
                try {
                    const nextRows = await nextBinding.rows(); expect(nextRows[0].source).toBe(native[0].source);
                    expect(nextRows[0].eligible).toBe(false); expect(nextRows[1].eligible).toBe(true);
                    expect(() => coordinator.prepare(nextBinding, editor, listbox, rows)).toThrow('mapping');
                    const peer = publication.bind({ instanceId: 'peer', scopePolicyStamp: 'peer', revision: '1', current: () => true });
                    try { expect((await peer.rows())[0].source).toBe(native[0].source); }
                    finally { peer.release(); }
                    const pending = nextBinding.rows(); nextBinding.release(); await expect(pending).rejects.toThrow('superseded');
                } finally { nextBinding.release(); await next.release(); }
                await publication.release(); expect(native[0].valid).toBe(false); expect(session.valid).toBe(true);
                await session.release(); expect(native[0].source.valid).toBe(false);
            } finally { coordinator.dispose(); binding.release(); await publication.release(); await session.release();
                await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); field.remove(); panel.remove(); declaration.remove(); }
        }
    },
};
