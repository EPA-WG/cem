import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession, type CemNativeSuggestionsConfig } from './native-capability-session.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { CemProcessingHost, CemProcessingNativeSessionInput } from './internal/runtime-support/processing-host.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixture sources enter through the explicit native CEMB boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Native Suggestions Consumer', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
function request(text: string): Extract<CemProcessingNativeSessionInput, { action: 'prepare' }> {
    const id = wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', 'memory:suggestions.cem', '');
    try {
        return { action: 'prepare', adapter: 'suggestions-v1', handle: { sessionKey: crypto.randomUUID(), instanceId: 'suggestions-fixture', scopePolicyStamp: 'suggestions-scope', sourceRevision: 'source-1', queryRevision: 1 },
            sources: { kind: 'cem-native-session-sources-v1', requesting: 0, sources: [{ bundle: wasm.exportReferenceReloadBundle(id, '').slice().buffer as ArrayBuffer, primarySourceId: 1, context: true }], bindings: [], grants: [] },
            data: {}, select: 'input.children', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS };
    } finally { wasm.disposeReferenceSource(id); }
}
async function scalar(host: CemProcessingHost, session: CemNativeCapabilitySession, expression: string, config: CemNativeSuggestionsConfig, index?: number): Promise<string | null> {
    const result = await session.view(expression, index, config);
    if (result.values.length !== 1) throw new Error('Expected one explicit presentation scalar');
    const exported = await host.value({ action: 'export-json', scopePolicyStamp: session.handle.scopePolicyStamp, limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, value: result.values[0] }).result;
    if (!('text' in exported)) throw new Error('Expected a named JSON export');
    return exported.text;
}
export const SourceViewsLabelsAndFilteringWorkerAndFallback: Story = {
    render: () => '<section aria-label="Native suggestions consumer fixture"></section>',
    play: async () => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const host = cemProcessingHostForScope(scope, { workerScriptUrl, ...(fallback ? { workerFactory: () => { throw new Error('fixture fallback'); } } : {}) });
            let current = true;
            const session = await CemNativeCapabilitySession.prepare(host, request('{optgroup @label=Group | {option @value=a | Apple}{option @value=same @label=Straße @selected=false | {#datadom.slices.later}}}{option @value=same | Second}'), () => current);
            const config: CemNativeSuggestionsConfig = { query: 'STRASSE', queryRevision: 1 };
            try {
                await expect(host.mode).toBe(fallback ? 'main-thread' : 'worker');
                await expect(session.suggestions?.rows).toBe(3);
                await expect(session.suggestions?.groups).toBe(1);
                await expect(session.suggestions?.identity).toBe('cem-suggestions-v1-unicode-17.0.0');
                await expect(session.suggestions?.diagnostics[0].code).toBe('cem.suggestions.selected_ignored');
                await expect(session.suggestions?.diagnostics[0].sourceMapRef?.fidelity).toBe('author-byte-exact');
                await expect(scalar(host, session, 'dom:attribute(input, "matched").value', config, 0)).resolves.toBe('false');
                await expect(scalar(host, session, 'dom:attribute(input, "matched").value', config, 1)).resolves.toBe('true');
                await expect(scalar(host, session, 'dom:attribute(input, "value").value', config, 1)).resolves.toBe('"same"');
                await expect(session.view('input.children.children', undefined, config)).rejects.toThrow('live suggestions source/content edges');
                const label = await session.render('{span | {$dom:attribute(suggestion, "label").value}|{$suggestion.content.expression}}', 1, config);
                await expect(JSON.stringify(label.nodes)).toContain('Straße');
                await expect(JSON.stringify(label.nodes)).toContain('#datadom.slices.later');
                const group = await session.render('{span | {$dom:attribute(group, "label").value}}', 0, config, true);
                await expect(JSON.stringify(group.nodes)).toContain('Group');
                const complete = await session.view('input.children.children.dom:attribute("value").value', undefined, config);
                await expect(complete.values.length).toBe(2);
                await expect(session.view('input', undefined, { ...config, filter: 'none', filterBy: 'label' })).rejects.toThrow('conflicts');
                await expect(session.view('input', undefined, { ...config, queryRevision: 2 })).rejects.toThrow('revision mismatch');
                await expect(session.view('input', 3, config)).rejects.toThrow('row handle');
                current = false;
                await expect(session.view('input', undefined, config)).rejects.toThrow('no longer current');
                await expect(session.valid).toBe(false);
                for (const text of ['{data | Missing value}', '{data @value=x | Good}{option | Mixed}']) {
                    await expect(CemNativeCapabilitySession.prepare(host, request(text), () => true)).rejects.toMatchObject({ diagnostics: [{ code: 'cem.suggestions.source_invalid', sourceMapRef: { fidelity: 'author-byte-exact' } }] });
                }
                const empty = await CemNativeCapabilitySession.prepare(host, request(''), () => true);
                try { await expect(empty.suggestions?.rows).toBe(0); }
                finally { await empty.release(); }
            } finally { await session.release(); await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); }
        }
    },
};
