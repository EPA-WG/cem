import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { createCemDeclarationScope } from './declaration-scope.js';
import { CemNativeCapabilitySession, type CemNativeSessionHandle, type CemNativeSessionSources } from './native-capability-session.js';
import { DEFAULT_CEM_VALUE_ARTIFACT_LIMITS } from './native-values.js';
import { cemProcessingHostForScope } from './internal/runtime-support/processing-host-runtime.js';
import type { CemProcessingNativeSessionInput } from './internal/runtime-support/processing-host.js';
// eslint-disable-next-line @nx/enforce-module-boundaries -- original native owners enter through the explicit CEMB export boundary.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
export default { title: 'CEM Elements/Native Capability Sessions', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const workerScriptUrl = new URL('./internal/runtime-support/processing-worker.ts', import.meta.url);
function handle(revision = 1): CemNativeSessionHandle {
    return { sessionKey: crypto.randomUUID(), instanceId: 'native-session-fixture', scopePolicyStamp: 'session-fixture', sourceRevision: `source-${revision}` };
}
function sources(label: string): CemNativeSessionSources {
    const bundle = (text: string) => {
        const id = wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', 'memory:options.cem', '');
        try { return wasm.exportReferenceReloadBundle(id, '').slice().buffer as ArrayBuffer; }
        finally { wasm.disposeReferenceSource(id); }
    };
    return { kind: 'cem-native-session-sources-v1', requesting: 0, sources: [
        { bundle: bundle('{#datadom.slices.options}'), primarySourceId: 1, context: true },
        { bundle: bundle(`@ns v = urn:vendor\n{v:option @value=same | ${label} {#datadom.slices.other}}{v:option @value=same | Second}`), primarySourceId: 1, context: true }],
        bindings: [{ source: 0, name: 'relation', select: 'input.children' },
            { source: 1, name: 'options', select: 'seq:where(input.children, fn(n) => n.kind == "element" && n.name != "@ns")' }], grants: [[0, 1]] };
}
function input(label: string, revision = 1): Extract<CemProcessingNativeSessionInput, { action: 'prepare' }> {
    return { action: 'prepare', handle: handle(revision), sources: sources(label), data: {}, select: 'relation', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS };
}
async function ready(): Promise<void> {
    await wasm.default({ module_or_path: new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url) });
}
export const RetainedSourcesWorkerAndFallback: Story = {
    render: () => '<section aria-label="Native capability transport fixture"></section>',
    play: async () => {
        await ready();
        for (const fallback of [false, true]) {
            const scope = createCemDeclarationScope({ document });
            const host = cemProcessingHostForScope(scope, { workerScriptUrl,
                ...(fallback ? { workerFactory: () => { throw new Error('fixture fallback'); } } : {}) });
            let current = true;
            const firstInput = input('First');
            const first = await CemNativeCapabilitySession.prepare(host, firstInput, () => current);
            const peerScope = createCemDeclarationScope({ document });
            const peerHost = cemProcessingHostForScope(peerScope, { workerScriptUrl,
                ...(fallback ? { workerFactory: () => { throw new Error('fixture fallback'); } } : {}) });
            await expect(peerHost.nativeSession({ action: 'view', handle: first.handle, expression: 'input.name' }).result).rejects.toThrow();
            const peer = await CemNativeCapabilitySession.prepare(peerHost, { ...input('Peer'), handle: { ...first.handle } }, () => true);
            const controller = new AbortController();
            const second = await CemNativeCapabilitySession.prepare(host, input('Other', 2), () => true, controller.signal);
            try {
                await expect(host.mode).toBe(fallback ? 'main-thread' : 'worker');
                await expect(first.length).toBe(2);
                const view = await first.view('input.name');
                await expect(view.handle.sourceRevision).toBe('source-1');
                await expect(view.values.length).toBe(2);
                const exported = await host.value({ action: 'export-json', scopePolicyStamp: first.handle.scopePolicyStamp,
                    limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS, value: view.values[0] }).result;
                await expect(exported).toEqual({ text: '"option"' });
                const label = await first.render('{span | {$input.namespace}|{$input.children.kind}|{$input.children.expression}}', 0);
                const text = JSON.stringify(label.nodes);
                await expect(text).toContain('urn:vendor');
                await expect(text).toContain('#datadom.slices.other');
                const independent = await second.render('{span | {$input.children}}', 1);
                await expect(JSON.stringify(independent.nodes)).toContain('Second');
                // CEMV must not smuggle executable descendants through the view channel.
                await expect(first.view('input', 0)).rejects.toThrow();
                await expect(host.nativeSession({ action: 'view', handle: { ...first.handle, sourceRevision: 'source-9' }, expression: 'input.name' }).result).rejects.toThrow('mismatch');
                await expect(host.nativeSession({ action: 'release', handle: { ...first.handle, sourceRevision: 'source-9' } }).result).rejects.toThrow('mismatch');
                await expect(first.view('input.name')).resolves.toHaveProperty('values.length', 2);
                await expect(host.nativeSession(firstInput).result).rejects.toThrow('already issued');
                await expect(first.view('input.name', 0)).resolves.toHaveProperty('values.length', 1);
                const mutableInput = input('Immutable', 5);
                const preparing = CemNativeCapabilitySession.prepare(host, mutableInput, () => true);
                mutableInput.handle.sourceRevision = 'source-99';
                const immutable = await preparing;
                try { await expect(immutable.handle.sourceRevision).toBe('source-5'); }
                finally { await immutable.release(); }
                const originalText = '<option value="original">Original</option>';
                const originalBytes = new TextEncoder().encode(originalText).buffer;
                const capturedInput: Extract<CemProcessingNativeSessionInput, { action: 'prepare' }> = {
                    action: 'prepare', adapter: 'suggestions-v1', handle: handle(6), data: {}, select: 'input.children', limits: DEFAULT_CEM_VALUE_ARTIFACT_LIMITS,
                    sources: { kind: 'cem-native-session-import-v1', bytes: originalBytes, contentType: 'application/xml', sourceUri: 'memory:checkpoint.xml' } };
                const capturing = CemNativeCapabilitySession.prepare(host, capturedInput, () => true, undefined, true);
                new Uint8Array(originalBytes).fill(0); capturedInput.select = 'invalid-caller-selector'; capturedInput.handle.sourceRevision = 'caller-alias';
                const capturedSession = await capturing;
                try {
                    const saved = capturedSession.checkpointSource();
                    expect(saved?.select).toBe('input.children'); expect(saved?.revision).toBe('source-6');
                    expect(atob(saved!.sources[0].base64)).toBe(originalText);
                    const intent = capturedSession.checkpointSourceIntent();
                    expect(intent?.rowCount).toBe(0); expect(intent?.select).toBe(saved?.select);
                    intent!.sources[0].base64 = 'caller-mutated-intent';
                    expect(capturedSession.checkpointSource()?.sources[0].base64).toBe(saved!.sources[0].base64);
                    const publication = await capturedSession.publishSuggestions({ query: '', queryRevision: 1, filter: 'none' }, () => true);
                    const binding = publication.bind({ instanceId: 'checkpoint-reader', scopePolicyStamp: 'test', revision: '1', current: () => true });
                    try { expect((await binding.rows())[0].value).toBe('original'); } finally { binding.release(); await publication.release(); }
                } finally { await capturedSession.release(); }
                const denied = input('Denied'); denied.sources.grants = [];
                await expect(CemNativeCapabilitySession.prepare(host, denied, () => true)).rejects.toThrow();
                const pending = input('Pending'); pending.sources.sources[0].context = false;
                await expect(CemNativeCapabilitySession.prepare(host, pending, () => true)).rejects.toThrow();
                const bounded = input('Bounded'); bounded.sources.sources[1].maxWork = 1;
                await expect(CemNativeCapabilitySession.prepare(host, bounded, () => true)).rejects.toThrow();
                current = false;
                await expect(first.view('input.name')).rejects.toThrow('no longer current');
                await expect(first.valid).toBe(false);
                await expect(peer.view('input.name')).resolves.toHaveProperty('values.length', 2);
                await expect(host.nativeSession({ action: 'view', handle: first.handle, expression: 'input.name' }).result).rejects.toThrow();
                controller.abort();
                await second.release();
                await expect(second.valid).toBe(false);
                const stale = input('Stale');
                await expect(CemNativeCapabilitySession.prepare(host, stale, () => false)).rejects.toThrow('superseded');
                await expect(host.nativeSession({ action: 'view', handle: stale.handle, expression: 'input.name' }).result).rejects.toThrow();
            } finally {
                await first.release(); await second.release();
                await peer.release(); await peerHost.dispose({ reason: 'runtime-disposed' }).result; peerScope.dispose();
                await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose();
            }
        }
    },
};
export const WorkerLossRequiresFreshSourceAuthority: Story = {
    render: () => '<section aria-label="Native session worker loss fixture"></section>',
    play: async () => {
        await ready();
        const scope = createCemDeclarationScope({ document });
        let worker: Worker | undefined;
        const host = cemProcessingHostForScope(scope, { workerScriptUrl, workerFactory: settings => {
            worker = new Worker(settings.scriptUrl, { type: 'module', name: settings.name }); return worker;
        } });
        const original = await CemNativeCapabilitySession.prepare(host, input('Before'), () => true);
        try {
            if (!worker) throw new Error('Missing real processing worker');
            worker.dispatchEvent(new ErrorEvent('error', { message: 'fixture worker loss' }));
            await expect(original.view('input.name')).rejects.toThrow('worker owner was lost');
            await expect(host.mode).toBe('main-thread');
            await expect(original.valid).toBe(false);
            const resumed = await CemNativeCapabilitySession.prepare(host, input('After', 2), () => true);
            try {
                await expect(resumed.handle.sessionKey).not.toBe(original.handle.sessionKey);
                await expect(resumed.view('input.name')).resolves.toHaveProperty('values.length', 2);
                await expect(original.view('input.name')).rejects.toThrow('worker owner was lost');
            } finally { await resumed.release(); }
        } finally { await original.release().catch(() => undefined); await host.dispose({ reason: 'runtime-disposed' }).result; scope.dispose(); }
    },
};

export const PreparationInputsAndNativeBindingsAreImmutable: Story = {
    render: () => '<section aria-label="Native preparation input checkpoint transport"></section>',
    play: async () => {
        await ready();
        for (const fallback of [false,true]) {
            const scope = createCemDeclarationScope({document});
            const host = cemProcessingHostForScope(scope,{workerScriptUrl,...(fallback ? {workerFactory:()=>{throw new Error('input fixture fallback');}} : {})});
            const compiled = JSON.parse(wasm.compileTemplate('{div @count={42}}','[]'));
            let session: CemNativeCapabilitySession | undefined;
            try {
                const plan = JSON.parse(wasm.renderTemplateWithNativeValues(compiled.artifactId,0,'{}','[]','[]','','native-preparation',undefined,undefined));
                const artifact = wasm.takeRenderValueArtifact(plan.nativeValueArtifactId).slice().buffer as ArrayBuffer;
                const native = {kind:'cem-native-value-v1' as const,artifact,contentHash:plan.nativeValueContentHash,index:plan.nodes[0].attributes[0].nativeValueIndex};
                const data = {datadom:{slices:{caption:'Original é😀'}}};
                const preparing = CemNativeCapabilitySession.prepare(host,{action:'prepare',adapter:'suggestions-v1',handle:handle(10),
                    sources:{kind:'cem-native-session-import-v1',bytes:new TextEncoder().encode('<option value="same">Original</option>').buffer,contentType:'application/xml',sourceUri:'memory:input-options.xml'},
                    data,select:'input.children',limits:DEFAULT_CEM_VALUE_ARTIFACT_LIMITS,
                    nativeAttributes:[{name:'count',value:native}],nativeSlices:[{name:'answer',attribute:'count',value:native},{name:'raw',value:native}]},()=>true,undefined,true);
                data.datadom.slices.caption = 'Caller mutation'; new Uint8Array(artifact).fill(0); native.index = 999;
                session = await preparing;
                const saved = session.checkpointSource()!;
                expect(saved.resolve).toBe(true);
                expect(saved.preparation?.dataJson).toContain('Original é😀');
                expect(saved.preparation?.nativeBindings.artifacts).toHaveLength(1);
                expect(saved.preparation?.nativeBindings.bindings.map(b=>b.target)).toEqual(['attribute','slice','slice']);
                const rendered = await session.render('{span | {$datadom.slices.caption}|{$datadom.slices.answer}}',0);
                expect(JSON.stringify(rendered.nodes)).toContain('Original é😀'); expect(JSON.stringify(rendered.nodes)).toContain('42');
                saved.preparation!.dataJson = '{}'; saved.preparation!.nativeBindings.artifacts[0].base64 = 'forged';
                expect(session.checkpointSource()?.preparation?.dataJson).toContain('Original é😀');
                expect(session.checkpointSource()?.preparation?.nativeBindings.artifacts[0].base64).not.toBe('forged');
                const state = {version:'cem-checkpoint-runtime/1',capability:null,references:session.checkpointSource(),resources:[],writers:[],writeDisposition:'suppress-until-new-input'};
                expect(wasm.validateCheckpointRuntime(JSON.stringify(state))).toContain('cem-native-bindings/1');
                await session.release(); expect(session.checkpointSource()).toBeUndefined();
            } finally { await session?.release(); wasm.disposeTemplate(compiled.artifactId); scope.dispose(); }
        }
    },
};
