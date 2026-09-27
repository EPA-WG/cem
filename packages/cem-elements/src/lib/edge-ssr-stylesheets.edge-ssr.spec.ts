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
