import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect, waitFor } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession, type CemNativeSuggestionsConfig } from './native-capability-session.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { CemProcessingHost, CemProcessingNativeSessionInput } from './internal/runtime-support/processing-host.js';
import type { RenderPlanNode } from './projection.js';
import { isCemNativeSuggestionsBinding } from './native-suggestions-publication.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemProcessingTextSource } from './internal/runtime-support/processing-host.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- fixture sources enter through the explicit native CEMB boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';

export default { title: 'CEM Elements/Native Suggestions Consumer', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
function renderedText(nodes: readonly RenderPlanNode[]): string {
    return nodes.map(node => node.kind === 'text' ? node.text : node.kind === 'element' ? renderedText(node.children ?? []) : '').join('');
}
function request(text: string): Extract<CemProcessingNativeSessionInput, { action: 'prepare' }> {
    const id = wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', 'memory:suggestions.cem', '');
    try {
        return { action: 'prepare', adapter: 'suggestions-v1', handle: { sessionKey: crypto.randomUUID(), instanceId: 'suggestions-fixture', scopePolicyStamp: 'suggestions-scope', sourceRevision: 'source-1' },
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
export const ComponentFramesRouteToOriginalOwner: Story = {
    render: () => '<section></section>',
    play: async ({ canvasElement }) => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const root = canvasElement.querySelector('section'); if (!root) throw new Error('Missing fixture');
        for (const fallback of [false, true]) {
            const sourceScope = createCemDeclarationScope({ document }), consumerScope = createCemDeclarationScope({ document });
            const host = cemProcessingHostForScope(sourceScope, { workerScriptUrl,
                ...(fallback ? { workerFactory: () => { throw new Error('fixture fallback'); } } : {}) });
            const session = await CemNativeCapabilitySession.prepare(host, request('{cem-option @value=x | Original {#later}}'), () => true);
            const publication = await session.publishSuggestions({ query: 'Original', queryRevision: 1 }, () => true);
            const suffix = crypto.randomUUID(), declarationTag = `native-frame-declaration-${suffix}`, tag = `native-frame-${suffix}`;
            let multiple = false;
            const runtime = new CemElementRuntime({ declarationTag, declarationScope: consumerScope,
                ...(fallback ? { processingWorkerFactory: () => { throw new Error('fixture fallback'); } } : {}),
                nativeSuggestionsInputs(instance, snapshot) {
                    const bind = () => publication.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp,
                        revision: snapshot.dataRevision, current: () => instance.isConnected });
                    return multiple ? [bind(), bind()] : [bind()];
                } });
            runtime.install(window);
            const declaration = document.createElement(declarationTag); declaration.setAttribute('tag', tag);
            const template = document.createElement('template'); template.type = 'text/cem-ml';
            template.textContent = '{attribute @name=caption | First}{span | {$caption}|{$datadom.slices.suggestions.dom:attribute("query").value}|{$datadom.slices.suggestions.children.source.children.expression}}';
            declaration.append(template); root.append(declaration); runtime.registerDeclaration(declaration);
            const instance = document.createElement(tag); root.append(instance);
            try {
                await runtime.whenRenderSettled(instance);
                expect(runtime.diagnosticsFor(instance)).toEqual([]);
                expect(instance.querySelector('span')?.textContent).toBe('First|Original|#later');
                const span = instance.querySelector('span'); instance.setAttribute('caption', 'Second');
                await new Promise(resolve => setTimeout(resolve));
                await runtime.whenRenderSettled(instance);
                expect(runtime.diagnosticsFor(instance)).toEqual([]);
                expect(instance.querySelector('span')).toBe(span);
                await waitFor(() => expect(span?.textContent).toBe('Second|Original|#later'));
                const snapshot = runtime.snapshotInstance(instance);
                const imported = await host.value({ action: 'import', scopePolicyStamp: snapshot.scopePolicyStamp,
                    bytes: new TextEncoder().encode('<b>Materialized</b>').buffer, contentType: 'application/xml', sourceUri: 'memory:extra.xml',
                    limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS }).result;
                if (!('value' in imported)) throw new Error('Missing materialized input');
                const direct = publication.bind({ instanceId: snapshot.instanceId, scopePolicyStamp: snapshot.scopePolicyStamp,
                    revision: snapshot.dataRevision, current: () => instance.isConnected });
                const id = 'materialized-extra';
                const source = '{span | {$extra.children.children}|{$datadom.slices.suggestions.children.source.children.expression}}';
                const frame = { compile: { language: 'cem-ml' as const, producedTag: tag, templateArtifactId: id, registrationIdentity: id,
                    source: createCemProcessingTextSource(source), sourceRef: { kind: 'inline' as const, value: id }, resolverIdentity: 'fixture',
                    scopePolicyStamp: snapshot.scopePolicyStamp, sourceMapMode: 'dev' as const, hostBindings: ['extra', 'datadom'] },
                    render: { revision: { instanceId: snapshot.instanceId, templateArtifactId: id, dataRevision: snapshot.dataRevision,
                        scopePolicyStamp: snapshot.scopePolicyStamp, outputTarget: snapshot.outputTarget, renderAttempt: snapshot.renderAttempt },
                        snapshot: { ...snapshot, templateArtifactId: id }, data: {}, scopeUid: snapshot.instanceId,
                        nativeSlices: [{ name: 'extra', value: imported.value }], nativeValueLimits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } };
                const rendered = await direct.renderComponent(frame);
                expect(JSON.stringify(rendered.frames)).toContain('Materialized');
                expect(JSON.stringify(rendered.frames)).toContain('#later');
                await expect(direct.renderComponent({ ...frame, render: { ...frame.render, nativeSlices: [{ name: 'suggestions', value: imported.value }] } })).rejects.toThrow('reserved');
                const stale = direct.renderComponent(frame); direct.release(); await expect(stale).rejects.toThrow('superseded');
                expect(snapshot.slices).not.toHaveProperty('suggestions');
                expect(snapshot.nativeSlices?.some(slice => slice.name === 'suggestions')).not.toBe(true);
                expect(() => runtime.setInstanceSlices(instance, { suggestions: 'forged' })).toThrow('reserved');
                multiple = true; runtime.refreshElementReferences(instance); await runtime.whenRenderSettled(instance);
                expect(runtime.diagnosticsFor(instance).some(d => d.message.includes('one verified live'))).toBe(true);
                expect(span?.textContent).toBe('Second|Original|#later');
            } finally { instance.remove(); declaration.remove(); consumerScope.dispose(); await publication.release(); await session.release();
                await host.dispose({ reason: 'runtime-disposed' }).result; sourceScope.dispose(); }
        }
    },
};
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
                await expect(scalar(host, session, 'dom:attribute(input, "query").value', { ...config, query: 'Next', queryRevision: 2 })).resolves.toBe('"Next"');
                await expect(session.view('input', undefined, { ...config, queryRevision: -1 })).rejects.toThrow('query revision');
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

export const OwnerRoutedPublicationsAndIndependentConsumers: Story = {
    render: () => '<section aria-label="Owner routed suggestions fixture"></section>',
    play: async () => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document }), peerScope = createCemDeclarationScope({ document });
            const options = { workerScriptUrl, ...(fallback ? { workerFactory: () => { throw new Error('fixture fallback'); } } : {}) };
            const host = cemProcessingHostForScope(scope, options), peer = cemProcessingHostForScope(peerScope, options);
            const session = await CemNativeCapabilitySession.prepare(host, request('{cem-option @value=x | Original {#later}}'), () => true);
            let publicationCurrent = true, revision = 1;
            const config = { query: 'Original', queryRevision: 1 };
            const publication = await session.publishSuggestions(config, () => publicationCurrent);
            config.query = 'Mutated';
            const first = publication.bind({ instanceId: 'first', scopePolicyStamp: 'first-scope', revision: '1', current: () => revision === 1 });
            const second = publication.bind({ instanceId: 'second', scopePolicyStamp: 'second-scope', revision: '1', current: () => true });
            const template = '{span | {$consumer}|{$datadom.slices.suggestions.dom:attribute("query").value}|{$datadom.slices.suggestions.children.source.children.expression}}';
            try {
                await expect(peer.nativeSession({ action: 'view', handle: session.handle, expression: 'input.name' }).result).rejects.toThrow();
                await expect(publication.config.query).toBe('Original');
                await expect(renderedText((await first.render(template, { consumer: 'First' })).nodes)).toBe('First|Original|#later');
                await expect(renderedText((await second.render(template, { consumer: 'Second' })).nodes)).toBe('Second|Original|#later');
                await expect(first.render(template, { suggestions: 'forged' })).rejects.toThrow('reserved');
                await expect(() => JSON.stringify(first)).toThrow('cannot be serialized');
                await expect(() => structuredClone(first)).toThrow();
                const copied = { ...first }; await expect(copied.render(template, {})).rejects.toThrow('copied native binding');
                await expect(isCemNativeSuggestionsBinding(first)).toBe(true); await expect(isCemNativeSuggestionsBinding(copied)).toBe(false);
                const isolated = await import(/* @vite-ignore */ new URL('./native-suggestions-publication.ts?interop', import.meta.url).href) as typeof import('./native-suggestions-publication.js');
                await expect(isolated.isCemNativeSuggestionsBinding(first)).toBe(true);
                const data = { consumer: 'Captured' }, pending = first.render(template, data); data.consumer = 'Changed'; revision = 2;
                await expect(pending).rejects.toThrow('superseded');
                revision = 1; await expect(first.valid).toBe(false); first.release();
                await expect(second.render(template, { consumer: 'Still current' })).resolves.toHaveProperty('status', 'rendered');
                const fresh = await session.publishSuggestions({ query: 'Next', queryRevision: 2 }, () => true);
                try {
                    const binding = fresh.bind({ instanceId: 'fresh', scopePolicyStamp: 'fresh', revision: '2', current: () => true });
                    try {
                        await expect(renderedText((await binding.render(template, { consumer: 'New' })).nodes)).toBe('New|Next|#later');
                        await expect(renderedText((await second.render(template, { consumer: 'Old' })).nodes)).toBe('Old|Original|#later');
                        const stale = second.render(template, { consumer: 'Stale' });
                        publicationCurrent = false;
                        await expect(stale).rejects.toThrow('superseded');
                        await expect(second.valid).toBe(false);
                        publicationCurrent = true; await expect(second.valid).toBe(false);
                        await expect(second.render(template, {})).rejects.toThrow('no longer current');
                        await publication.release();
                        await expect(session.valid).toBe(true);
                        await expect(fresh.config.queryRevision).toBe(2);
                        await expect(binding.render('{span | {$datadom.slices.suggestions.children.content}}', {})).resolves.toHaveProperty('status', 'rendered');
                    } finally { binding.release(); }
                } finally { await fresh.release(); }
            } finally { first.release(); second.release(); await publication.release(); await session.release();
                await peer.dispose({ reason: 'runtime-disposed' }).result; peerScope.dispose(); await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); }
        }
    },
};

export const PublicationOwnerLossRequiresFreshLeases: Story = {
    render: () => '<section aria-label="Native publication owner loss fixture"></section>',
    play: async () => {
        await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
        const scope = createCemDeclarationScope({ document }); let worker: Worker | undefined;
        const host = cemProcessingHostForScope(scope, { workerScriptUrl, workerFactory: settings => {
            worker = new Worker(settings.scriptUrl, { type: 'module', name: settings.name }); return worker;
        } });
        const session = await CemNativeCapabilitySession.prepare(host, request('{cem-option @value=x | Original}'), () => true);
        const publication = await session.publishSuggestions({ query: '', queryRevision: 1 }, () => true);
        const binding = publication.bind({ instanceId: 'consumer', scopePolicyStamp: 'consumer', revision: '1', current: () => true });
        try {
            if (!worker) throw new Error('Missing native publication worker');
            const rows = await binding.rows(), lost: string[] = [];
            binding.subscribe(() => lost.push('binding')); rows[0].source.subscribe(() => lost.push('source'));
            worker.dispatchEvent(new ErrorEvent('error', { message: 'publication owner loss' }));
            expect(lost).toContain('binding'); expect(lost).toContain('source'); expect(rows[0].valid).toBe(false);
            await expect(binding.render('{span | {$datadom.slices.suggestions.children.content}}', {})).rejects.toThrow();
            await expect(binding.valid).toBe(false); await expect(publication.valid).toBe(false);
            const resumed = await CemNativeCapabilitySession.prepare(host, request('{cem-option @value=x | Fresh}'), () => true);
            const next = await resumed.publishSuggestions({ query: '', queryRevision: 1 }, () => true);
            try {
                const fresh = next.bind({ instanceId: 'consumer', scopePolicyStamp: 'consumer', revision: '2', current: () => true });
                await expect(renderedText((await fresh.render('{span | {$datadom.slices.suggestions.children.content}}', {})).nodes)).toBe('Fresh');
                await expect(binding.render('{span}', {})).rejects.toThrow('no longer current'); fresh.release();
            } finally { await next.release(); await resumed.release(); }
        } finally { binding.release(); await publication.release().catch(() => undefined); await session.release().catch(() => undefined);
            await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); }
    },
};
