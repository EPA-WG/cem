import { executeNativeSsrInitialRenderFixture } from './edge-ssr-host-fixture.js';
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
