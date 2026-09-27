import { serializeDeclarationStylesheets } from './declaration-style-markup.js';
import { executeNativeSsrInitialRenderFixture, executeNativeEdgeRenderUpdateFixture } from './edge-ssr-host-fixture.js';
import { CemEdgeSsrJobSequence, createCemEdgeSsrHostRequestEnvelope } from './edge-ssr-host.js';
import { exportDataIslandSnapshotForEdge } from './cem-elements.js';
import { edgeSsrSnapshotFixture, PROCESSING_BOUNDARY_TEMPLATE_SOURCE } from './processing-boundary.fixtures.js';
import { InMemoryEdgeRenderStateStore, readEdgeRenderStateContents } from './projection.js';
import { readFile } from 'node:fs/promises';
import { beforeAll, expect, it, vi } from 'vitest';
// eslint-disable-next-line @nx/enforce-module-boundaries -- exercise the generated native bindings directly in the Node evidence host.
import * as wasm from '../../../cem_ql/dist/wasm/cem_ql.js';
import { loadEdgeStylesheets, type EdgeStylesheetLoadOptions } from './edge-ssr-stylesheets.js';

beforeAll(async () => {
    await wasm.default({ module_or_path: await readFile(new URL('../../../cem_ql/dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
});

function options(overrides: Partial<EdgeStylesheetLoadOptions> = {}): EdgeStylesheetLoadOptions {
    return {
        native: wasm, owner: { kind: 'instance', identity: 'persisted-instance' },
        sources: [{ css: '@import "theme"; @keyframes pulse {from {opacity:0} to {opacity:1}} p {animation:pulse 1s}', scope: null }],
        baseUrl: 'https://example.test/main.css', signal: new AbortController().signal,
        context: { identity: 'page', resolverIdentity: 'resolver', resourcePolicyStamp: 'policy', frames: [
            { frameId: 'outer', baseUrl: 'https://example.test/', scopes: [], specifiers: { imports: {}, resources: {
                theme: { target: './default.css' }, icon: { target: './default.svg' },
            } } },
            { frameId: 'inner', baseUrl: 'https://example.test/inner/', scopes: [], specifiers: { imports: {}, resources: {
                theme: { target: './local.css', contentType: 'text/css' }, icon: { target: './local.svg' },
            } } },
        ] },
        read: async () => ({ bytes: new TextEncoder().encode('p {color:purple;background:url(icon);mask:url(relative.svg)}').buffer,
            finalUrl: 'https://example.test/cdn/redirected.css', contentType: 'text/css' }),
        ...overrides,
    };
}

it('loads imports without DOM globals, resolves consuming maps and final bases, and disposes native owners', async () => {
    expect(typeof document).toBe('undefined');
    const dispose = vi.fn(wasm.disposeTemplate);
    const input = options({ native: { ...wasm, disposeTemplate: dispose } });
    const read = vi.fn(input.read);
    input.read = read;
    const first = await loadEdgeStylesheets(input);
    expect(read).toHaveBeenCalledWith(expect.objectContaining({ url: 'https://example.test/inner/local.css' }), input.signal);
    expect(first.diagnostics).toEqual([]);
    expect(first.styles).toHaveLength(1);
    expect(first.styles[0].css).toContain('@scope to (');
    expect(first.styles[0].css).toContain('https://example.test/inner/local.svg');
    expect(first.styles[0].css).toContain('https://example.test/cdn/relative.svg');
    expect(first.styles[0].css).not.toContain('data-cem-css-context');
    expect(dispose).toHaveBeenCalledTimes(1);
    expect(wasm.disposeTemplate(dispose.mock.calls[0][0])).toBe(false);
    expect(structuredClone(first)).toEqual(first);
    expect((await loadEdgeStylesheets(input)).styles).toEqual(first.styles);
    const other = await loadEdgeStylesheets(options({ owner: { kind: 'instance', identity: 'other-instance' } }));
    expect(other.styles[0].identity.ownerKey).not.toBe(first.styles[0].identity.ownerKey);
    expect(other.styles[0].css).not.toBe(first.styles[0].css);
    const changed = options();
    changed.context.identity = 'another-context';
    changed.context.frames[1].specifiers.resources.icon = { target: './other.svg' };
    const isolated = await loadEdgeStylesheets(changed);
    expect(isolated.styles[0].css).toContain('https://example.test/inner/other.svg');
    expect(isolated.styles[0].identity.ownerKey).not.toBe(first.styles[0].identity.ownerKey);
});

it('uses admitted occurrence scopes after malformed adoption and retains independent valid output', async () => {
    const result = await loadEdgeStylesheets(options({ owner: { kind: 'declaration', identity: 'card', tag: 'cem-card' },
        sources: [{ css: 'p {color:red', scope: null }, { css: 'p {color:green}', scope: 'library' },
            { css: 'p {color:blue}', scope: null }] }));
    expect(result.diagnostics.map(d => d.code)).toContain('cem.ql.template.stylesheet_parse_failed');
    expect(result.styles).toHaveLength(2);
    expect(result.styles[0].css).toContain('[scope="library"]');
    expect(result.styles[1].css).toContain('@scope (cem-card)');
});

it.each(['mime', 'transport'] as const)('preserves native import provenance on %s failure and compiles valid siblings', async failure => {
    const result = await loadEdgeStylesheets(options({ sources: [
        { css: '@import "theme";', scope: null }, { css: 'p {color:green}', scope: null },
    ], read: async () => {
        if (failure === 'transport') throw new Error('offline');
        return { bytes: new TextEncoder().encode('<html/>').buffer, finalUrl: 'https://example.test/error', contentType: 'text/html' };
    } }));
    expect(result.styles).toHaveLength(1);
    expect(result.styles[0]).toMatchObject({ index: 1 });
    expect(result.styles[0].css).toContain('color:green');
    expect(result.diagnostics[0]).toMatchObject({ code: failure === 'mime' ? 'cem.css.import_content_type' : 'cem.css.import_load_failed',
        offset: 0, length: '@import "theme";'.length, stylesheetUrl: 'https://example.test/main.css' });
});

it('cancels an uncooperative reader promptly and never delivers late bytes into a disposed owner', async () => {
    const controller = new AbortController();
    const dispose = vi.fn(wasm.disposeTemplate);
    const deliver = vi.fn(wasm.deliverTemplateStylesheet);
    let resolveRead: (value: Awaited<ReturnType<EdgeStylesheetLoadOptions['read']>>) => void = () => undefined;
    const pending = new Promise<Awaited<ReturnType<EdgeStylesheetLoadOptions['read']>>>(resolve => { resolveRead = resolve; });
    const read = vi.fn(() => pending);
    const result = loadEdgeStylesheets(options({ signal: controller.signal, read,
        native: { ...wasm, disposeTemplate: dispose, deliverTemplateStylesheet: deliver } }));
    expect(read).toHaveBeenCalledTimes(1);
    controller.abort(new Error('disconnected'));
    await expect(result).rejects.toThrow('disconnected');
    expect(dispose).toHaveBeenCalledTimes(1);
    resolveRead({ bytes: new TextEncoder().encode('p{}').buffer, finalUrl: 'https://example.test/late.css', contentType: 'text/css' });
    await Promise.resolve();
    expect(deliver).not.toHaveBeenCalled();
    expect(wasm.disposeTemplate(dispose.mock.calls[0][0])).toBe(false);
});

it('does not adopt after cancellation and diagnoses invalid instance ownership', async () => {
    const controller = new AbortController();
    controller.abort();
    const adopt = vi.fn(wasm.adoptInstanceStylesheets);
    await expect(loadEdgeStylesheets(options({ signal: controller.signal,
        native: { ...wasm, adoptInstanceStylesheets: adopt } }))).rejects.toBeDefined();
    expect(adopt).not.toHaveBeenCalled();
    const invalid = await loadEdgeStylesheets(options({ sources: [{ css: 'p{}', scope: 'shared' }] }));
    expect(invalid.styles).toEqual([]);
    expect(invalid.diagnostics[0].code).toBe('cem.ql.stylesheet_instance_invalid');
});

function initialRequest(css: string) {
    const snapshot = edgeSsrSnapshotFixture();
    snapshot.scopePolicyStamp += ':retained-declaration-css:retained-instance-css';
    snapshot.payload.nodes = [{ kind: 'element', key: 'style-0', tag: 'style', namespace: null, attributes: {}, slot: null,
        children: [{ kind: 'text', key: 'style-0/0', text: css }] }];
    snapshot.payload.slots = {};
    const exported = exportDataIslandSnapshotForEdge(snapshot, { fields: {
        hostAttributes: 'allow', dataset: 'allow', payload: 'allow', slices: 'allow', formData: 'allow',
        validationState: 'allow', eventPayloads: 'allow',
    } });
    return createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-initial', {
        template: { kind: 'serialized-template-source-v1', templateArtifactId: snapshot.templateArtifactId,
            source: PROCESSING_BOUNDARY_TEMPLATE_SOURCE },
        snapshot: exported, sourceMapMode: 'dev', scopeUid: 'server-scope',
        revision: { instanceId: snapshot.instanceId, dataRevision: snapshot.dataRevision,
            renderAttempt: snapshot.renderAttempt, templateArtifactId: snapshot.templateArtifactId,
            scopePolicyStamp: snapshot.scopePolicyStamp, outputTarget: snapshot.outputTarget },
    });
}

it('awaits imports before committing initial SSR and emits native instance styles outside the render range', async () => {
    const request = initialRequest('@import "theme"; @keyframes pulse {} p {animation:pulse 1s}');
    const stateKey = `edge-state:${request.payload.snapshot.scopePolicyStamp}:${request.payload.snapshot.instanceId}`;
    const store = new InMemoryEdgeRenderStateStore();
    const transport = options();
    let release: () => void = () => undefined;
    const gate = new Promise<void>(resolve => { release = resolve; });
    const response = executeNativeSsrInitialRenderFixture(request, store, { ...transport,
        read: async (...args) => { await gate; return transport.read(...args); },
    });
    expect(store.readRecord(stateKey)).toBeUndefined();
    request.payload.snapshot.hostAttributes = { label: 'mutated during read' };
    release();
    const result = await response;
    expect(result.outcome).toBe('success');
    if (result.outcome !== 'success') return;
    expect(result.result.renderedHtml).toContain('Projected');
    expect(result.result.renderedHtml).not.toContain('<style');
    expect(result.result.instanceStylesheetHtml).toContain('<style data-cem-instance-style="0">@scope to (');
    expect(result.result.instanceStylesheetHtml).toContain('https://example.test/inner/local.svg');
    expect(result.result.instanceStylesheetHtml).toContain('@keyframes pulse-');
    expect(result.result.diagnostics).toEqual([]);
    const retained = readEdgeRenderStateContents(store, result.result.renderState);
    expect(retained.ok).toBe(true);
    if (retained.ok) expect(retained.contents.renderedHtml).toBe(result.result.renderedHtml);
    expect(structuredClone(result)).toEqual(result);
});

it('rejects unsafe native style HTML before storing SSR state', async () => {
    const request = initialRequest('p::before {content:"</style><script>bad</script>"}');
    const store = new InMemoryEdgeRenderStateStore();
    const result = await executeNativeSsrInitialRenderFixture(request, store, options());
    expect(result).toMatchObject({ outcome: 'failure', reason: 'render-failed' });
    expect(store.readRecord(`edge-state:${request.payload.snapshot.scopePolicyStamp}:${request.payload.snapshot.instanceId}`)).toBeUndefined();
});

it('cancels initial native SSR without writing state or returning partial HTML', async () => {
    const request = initialRequest('@import "theme";');
    const controller = new AbortController();
    const store = new InMemoryEdgeRenderStateStore();
    const result = executeNativeSsrInitialRenderFixture(request, store, options({ signal: controller.signal,
        read: () => new Promise(() => undefined),
    }));
    controller.abort(new Error('cancelled SSR'));
    expect(await result).toMatchObject({ outcome: 'cancelled', reason: 'cancelled',
        diagnostics: [{ code: 'cem.edge_ssr.stylesheet_cancelled' }] });
    expect(store.readRecord(`edge-state:${request.payload.snapshot.scopePolicyStamp}:${request.payload.snapshot.instanceId}`)).toBeUndefined();
});


it('keeps older declaration-only retained policies gated instead of changing their payload placement', async () => {
    const request = initialRequest('p {color:green}');
    request.payload.snapshot.scopePolicyStamp = request.payload.snapshot.scopePolicyStamp.replace(':retained-instance-css', '');
    request.payload.revision.scopePolicyStamp = request.payload.snapshot.scopePolicyStamp;
    const adopt = vi.fn(wasm.adoptInstanceStylesheets);
    const result = await executeNativeSsrInitialRenderFixture(request, new InMemoryEdgeRenderStateStore(),
        options({ native: { ...wasm, adoptInstanceStylesheets: adopt } }));
    expect(result).toMatchObject({ outcome: 'failure', reason: 'content-unavailable',
        diagnostics: [{ code: 'cem.edge_ssr.retained_css_unavailable' }] });
    expect(adopt).not.toHaveBeenCalled();
});

it('serializes admitted declaration scopes and preserves native context identity for SSR placement', async () => {
    const output = await loadEdgeStylesheets(options({ owner: { kind: 'declaration', identity: 'card', tag: 'cem-card' },
        sources: [{ css: 'p {color:red', scope: null }, { css: '@import "theme";', scope: 'library' },
            { css: 'p {color:blue}', scope: null }] }));
    const styles = output.styles.map(style => {
        if (style.scope.kind === 'instance') throw new Error('expected declaration scope');
        return { index: style.index, scope: style.scope, output: style };
    });
    const serialized = serializeDeclarationStylesheets(styles);
    expect(serialized.html).toContain('data-cem-declaration-style="shared"');
    expect(serialized.html).toContain('data-cem-style-scope="library"');
    expect(serialized.html).toContain('data-cem-declaration-style="private"');
    expect(serialized.html).toContain('data-cem-style-index="0"');
    expect(serialized.html).toContain('data-cem-style-index="1"');
    expect(serialized.html).toContain('https://example.test/inner/local.svg');
    expect(serialized.contextMarker).toBe(output.styles[0].identity.contextMarker);
    expect(serialized.contextMarker).not.toBeNull();
    const first = styles[0];
    expect(() => serializeDeclarationStylesheets([first, first])).toThrow('invalid declaration stylesheet batch');
    expect(() => serializeDeclarationStylesheets([first, { ...styles[1], output: { ...styles[1].output,
        identity: { ...styles[1].output.identity, contextMarker: 'different-context' } } }])).toThrow('invalid declaration stylesheet batch');
    expect(() => serializeDeclarationStylesheets([{ ...first, output: { ...first.output,
        css: 'p::before{content:"</STYLE><script>bad</script>"}' } }])).toThrow('unsafe stylesheet HTML');
    const escaped = serializeDeclarationStylesheets([{ ...first, scope: { kind: 'shared', name: 'a"<&' } }]);
    expect(escaped.html).toContain('data-cem-style-scope="a&quot;&lt;&amp;"');
});

it('waits for copied declaration batches before committing initial SSR and keeps sidecars outside the render range', async () => {
    const request = initialRequest('p {color:green}');
    const store = new InMemoryEdgeRenderStateStore();
    const transport = options();
    let release: () => void = () => undefined;
    const gate = new Promise<void>(resolve => { release = resolve; });
    const read = vi.fn(async (...args: Parameters<typeof transport.read>) => { await gate; return transport.read(...args); });
    const declarations = [{ owner: { kind: 'declaration' as const, identity: 'source-library', tag: 'cem-source' },
        sources: [{ css: '@import "theme";', scope: 'library' }, { css: 'p {color:blue}', scope: null }],
        context: structuredClone(transport.context), baseUrl: transport.baseUrl }];
    const response = executeNativeSsrInitialRenderFixture(request, store, { ...transport, read, declarations });
    await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(1));
    expect(store.readRecord(`edge-state:${request.payload.snapshot.scopePolicyStamp}:${request.payload.snapshot.instanceId}`)).toBeUndefined();
    declarations[0].sources[1].css = 'p {color:red}';
    declarations[0].context.frames[1].specifiers.resources.icon.target = './mutated.svg';
    release();
    const result = await response;
    expect(result.outcome).toBe('success');
    if (result.outcome !== 'success') return;
    expect(result.result.renderedHtml).not.toContain('<style');
    expect(result.result.instanceStylesheetHtml).toContain('color:green');
    const batches = result.result.declarationStylesheets;
    expect(batches).toHaveLength(1);
    expect(batches?.[0]).toMatchObject({ declarationIdentity: 'source-library', tag: 'cem-source' });
    expect(batches?.[0].html).toContain('data-cem-style-scope="library"');
    expect(batches?.[0].html).toContain('@scope (cem-source');
    expect(batches?.[0].html).toContain('color:blue');
    expect(batches?.[0].html).toContain('https://example.test/inner/local.svg');
    expect(batches?.[0].html).not.toContain('mutated.svg');
    expect(batches?.[0].contextMarker).toEqual(expect.any(String));
    expect(batches?.[0].html).toContain(`[data-cem-css-context="${batches?.[0].contextMarker}"]`);
    expect(result.result.diagnostics).toEqual([]);
    expect(structuredClone(result)).toEqual(result);
});

it.each(['unsafe', 'cancelled'] as const)('does not commit initial SSR when a declaration batch is %s', async failure => {
    const request = initialRequest('p {color:green}');
    const store = new InMemoryEdgeRenderStateStore();
    const controller = new AbortController();
    const transport = options({ signal: controller.signal });
    const read = vi.fn(() => new Promise<Awaited<ReturnType<typeof transport.read>>>(() => undefined));
    const input = { ...transport, read, declarations: [{
        owner: { kind: 'declaration' as const, identity: 'source', tag: 'cem-source' },
        sources: [{ css: failure === 'unsafe' ? 'p::before{content:"</style><script>bad</script>"}' : '@import "theme";', scope: null }],
        baseUrl: transport.baseUrl, context: transport.context,
    }] };
    const response = executeNativeSsrInitialRenderFixture(request, store, input);
    if (failure === 'cancelled') {
        await vi.waitFor(() => expect(read).toHaveBeenCalledTimes(1));
        input.signal = new AbortController().signal;
        controller.abort(new Error('cancelled declaration import'));
    }
    expect(await response).toMatchObject(failure === 'cancelled'
        ? { outcome: 'cancelled', reason: 'cancelled' } : { outcome: 'failure', reason: 'render-failed' });
    expect(store.readRecord(`edge-state:${request.payload.snapshot.scopePolicyStamp}:${request.payload.snapshot.instanceId}`)).toBeUndefined();
});

it('keeps valid declaration output and native diagnostics when another occurrence fails', async () => {
    const transport = options();
    const result = await executeNativeSsrInitialRenderFixture(initialRequest('p{}'), new InMemoryEdgeRenderStateStore(), {
        ...transport, declarations: [{ owner: { kind: 'declaration', identity: 'source', tag: 'cem-source' },
            sources: [{ css: 'p{color:red', scope: null }, { css: 'p{color:blue}', scope: 'library' }],
            baseUrl: transport.baseUrl, context: transport.context }],
    });
    expect(result.outcome).toBe('success');
    if (result.outcome !== 'success') return;
    expect(result.result.declarationStylesheets?.[0].html).toContain('color:blue');
    expect(result.result.diagnostics).toEqual([expect.objectContaining({ code: 'cem.ql.template.stylesheet_parse_failed' })]);
});

async function nativeUpdateFixture() {
    const initial = initialRequest('p {color:green}');
    const store = new InMemoryEdgeRenderStateStore();
    const transport = options();
    const context = { baseUrl: transport.baseUrl, context: transport.context, declarations: [{
        owner: { kind: 'declaration' as const, identity: 'source', tag: 'cem-source' },
        sources: [{ css: 'p{color:blue}', scope: 'library' }], baseUrl: transport.baseUrl, context: transport.context,
    }] };
    const seeded = await executeNativeSsrInitialRenderFixture(initial, store, { ...transport, ...context });
    if (seeded.outcome !== 'success') throw new Error('failed native initial render');
    const record = seeded.result.renderState;
    const payload = structuredClone(initial.payload);
    payload.snapshot.hostAttributes = { ...payload.snapshot.hostAttributes, label: 'Updated' };
    payload.snapshot.dataRevision = '2'; payload.revision.dataRevision = '2';
    const request = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-update', {
        ...payload, previousRenderPlan: { stateKey: record.stateKey, expectedEtag: record.etag,
            address: { ...record.currentRenderPlan, kind: 'render-plan' as const }, identity: record.renderRevision },
    });
    return { store, seeded, record, request, context, transport };
}

it('streams a data update while retaining native styles outside the render plan and copies inputs before iteration', async () => {
    const f = await nativeUpdateFixture();
    const stream = executeNativeEdgeRenderUpdateFixture(f.request, f.store, f.context, new AbortController().signal);
    f.request.payload.snapshot.hostAttributes = { label: 'Mutated after submission' };
    f.context.baseUrl = 'https://changed.test/';
    const responses = await collectNativeUpdates(stream);
    const progress = responses.filter(response => response.outcome === 'progress');
    expect(progress.map(response => response.result.frame.type)).toEqual(['begin', 'ops', 'commit']);
    const result = responses.at(-1);
    expect(result?.outcome).toBe('success');
    if (result?.outcome !== 'success') return;
    expect(result.result.renderState.currentStylesheets).toEqual(f.record.currentStylesheets);
    const retained = readEdgeRenderStateContents(f.store, result.result.renderState);
    expect(retained.ok).toBe(true);
    if (!retained.ok) return;
    expect(retained.contents.renderedHtml).toContain('Updated');
    expect(retained.contents.renderedHtml).not.toContain('<style');
    expect(retained.contents.stylesheetState).toMatchObject({ kind: 'native-ssr-stylesheets-v1',
        instanceStylesheetHtml: f.seeded.result.instanceStylesheetHtml,
        declarationStylesheets: f.seeded.result.declarationStylesheets });
});

it.each(['payload-css', 'context', 'base', 'declaration', 'template', 'scope'] as const)(
    'commits changed %s with a complete native stylesheet batch', async change => {
        const f = await nativeUpdateFixture();
        if (change === 'payload-css') f.request.payload.snapshot.payload.nodes = [];
        if (change === 'context') f.context.context.resolverIdentity = 'different';
        if (change === 'base') f.context.baseUrl = 'https://different.test/';
        if (change === 'declaration') f.context.declarations[0].sources[0].css = 'p{color:red}';
        if (change === 'template' && f.request.payload.template.kind === 'serialized-template-source-v1') f.request.payload.template.source = [];
        if (change === 'scope') f.request.payload.scopeUid = 'changed';
        const responses = await collectNativeUpdates(executeNativeEdgeRenderUpdateFixture(f.request, f.store,
            { ...f.transport, ...f.context }, new AbortController().signal));
        const result = responses.at(-1);
        expect(result?.outcome).toBe('success');
        if (result?.outcome !== 'success') return;
        expect(result.result.stylesheets?.batch.kind).toBe('native-css-batch-v1');
        expect(result.result.renderState.currentStylesheets).not.toEqual(f.record.currentStylesheets);
        expect(readEdgeRenderStateContents(f.store, result.result.renderState)).toMatchObject({ ok: true,
            contents: { stylesheetState: result.result.stylesheets } });
        if (change === 'payload-css') expect(result.result.stylesheets?.batch.instance).toEqual([]);
        expect(f.store.readRecord(f.record.stateKey)?.etag).not.toBe(f.record.etag);
    },
);

it.each(['missing', 'corrupt', 'old-state', 'stale', 'cancelled', 'cancel-during-read'] as const)(
    'does not emit native update frames for %s state', async failure => {
        const f = await nativeUpdateFixture();
        const controller = new AbortController();
        const getContent = f.store.getContent.bind(f.store);
        if (failure === 'old-state') f.store.writeRecord({ ...f.record, currentStylesheets: undefined });
        if (failure === 'missing' || failure === 'corrupt' || failure === 'cancel-during-read') {
            vi.spyOn(f.store, 'getContent').mockImplementation(address => {
                if (address.kind !== 'stylesheets') return getContent(address);
                if (failure === 'missing') return undefined;
                if (failure === 'corrupt') return { changed: true };
                controller.abort(new Error('cancel before commit'));
                return getContent(address);
            });
        }
        if (failure === 'stale') f.request.payload.previousRenderPlan.expectedEtag = 'stale';
        if (failure === 'cancelled') controller.abort();
        const before = f.store.readRecord(f.record.stateKey);
        const responses = await collectNativeUpdates(executeNativeEdgeRenderUpdateFixture(f.request, f.store, f.context, controller.signal));
        expect(responses).toHaveLength(1);
        expect(responses[0]).toMatchObject(failure.startsWith('cancel') ? { outcome: 'cancelled', reason: 'cancelled' }
            : { outcome: 'failure', reason: failure === 'stale' ? 'render-state-conflict' : 'content-unavailable' });
        expect(f.store.readRecord(f.record.stateKey)).toEqual(before);
    },
);

async function collectNativeUpdates(stream: ReturnType<typeof executeNativeEdgeRenderUpdateFixture>) {
    const responses = [];
    for await (const response of stream) responses.push(response);
    return responses;
}

it('finishes an already committed stream after abort and preserves stylesheet state on the following update', async () => {
    const f = await nativeUpdateFixture();
    const controller = new AbortController();
    const stream = executeNativeEdgeRenderUpdateFixture(f.request, f.store, f.context, controller.signal);
    const first = await stream.next();
    expect(first.value).toMatchObject({ outcome: 'progress', result: { frame: { type: 'begin' } } });
    expect(f.store.readRecord(f.record.stateKey)?.etag).not.toBe(f.record.etag);
    controller.abort();
    const result = (await collectNativeUpdates(stream)).at(-1);
    expect(result?.outcome).toBe('success');
    if (result?.outcome !== 'success') return;
    const record = result.result.renderState;
    const payload = structuredClone(f.request.payload);
    payload.snapshot.dataRevision = '3'; payload.revision.dataRevision = '3';
    payload.previousRenderPlan = { stateKey: record.stateKey, expectedEtag: record.etag,
        address: { ...record.currentRenderPlan, kind: 'render-plan' }, identity: record.renderRevision };
    if (!record.currentTemplateArtifact) throw new Error('missing retained template');
    payload.template = { kind: 'content-addressed-template-artifact-v1', templateArtifactId: payload.snapshot.templateArtifactId,
        address: { ...record.currentTemplateArtifact, kind: 'template-artifact' } };
    const request = createCemEdgeSsrHostRequestEnvelope(new CemEdgeSsrJobSequence(), 'render-update', payload);
    const next = (await collectNativeUpdates(executeNativeEdgeRenderUpdateFixture(request, f.store, f.context, new AbortController().signal))).at(-1);
    expect(next?.outcome).toBe('success');
    if (next?.outcome === 'success') expect(next.result.renderState.currentStylesheets).toEqual(f.record.currentStylesheets);
});

it.each(['cancel', 'conflict', 'import-failure'] as const)('rejects a changed CSS batch on %s without publishing partial state', async failure => {
    const f = await nativeUpdateFixture();
    const controller = new AbortController();
    const source = f.request.payload.snapshot.payload.nodes[0];
    if (source.kind !== 'element') throw new Error('missing payload stylesheet');
    source.children = [{ kind: 'text', key: 'changed/css', text: '@import "./held.css";' }];
    let release!: () => void; let started!: () => void;
    const gate = new Promise<void>(resolve => { release = resolve; });
    const reading = new Promise<void>(resolve => { started = resolve; });
    const read = async () => { started(); await gate; if (failure === 'import-failure') throw new Error('failed import');
        return { bytes: new TextEncoder().encode('p {color:red}').buffer, finalUrl: 'https://example.test/held.css', contentType: 'text/css' }; };
    const stream = executeNativeEdgeRenderUpdateFixture(f.request, f.store, { ...f.transport, ...f.context, read }, controller.signal);
    const pending = collectNativeUpdates(stream);
    await reading;
    expect(f.store.readRecord(f.record.stateKey)).toEqual(f.record);
    if (failure === 'cancel') controller.abort();
    if (failure === 'conflict') {
        const competing = structuredClone(f.request);
        competing.payload.snapshot.payload = structuredClone(f.seeded.result.hydrationData.snapshot.payload);
        const responses = await collectNativeUpdates(executeNativeEdgeRenderUpdateFixture(competing, f.store,
            { ...f.transport, ...f.context }, new AbortController().signal));
        expect(responses.at(-1)?.outcome).toBe('success');
    }
    const before = f.store.readRecord(f.record.stateKey);
    release();
    const responses = await pending;
    expect(responses).toHaveLength(1);
    expect(responses[0]).toMatchObject(failure === 'cancel' ? { outcome: 'cancelled', reason: 'cancelled' }
        : { outcome: 'failure', reason: failure === 'conflict' ? 'render-state-conflict' : 'render-failed' });
    expect(f.store.readRecord(f.record.stateKey)).toEqual(before);
});
