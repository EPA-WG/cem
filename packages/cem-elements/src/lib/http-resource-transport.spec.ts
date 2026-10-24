import { afterEach, expect, it, vi } from 'vitest';
import { openCemHttpResource, streamCemHttpResponseBody } from './http-resource-transport.js';
import type { CemHttpRequest } from './cem-elements.js';

afterEach(() => vi.unstubAllGlobals());
const request = (): CemHttpRequest => ({ authoredUrl: '/data', baseUrl: 'https://example.test/', resolvedUrl: 'https://example.test/data',
    resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'fixture', method: 'GET',
    headers: { Accept: 'application/json' }, signal: new AbortController().signal,
    policy: { allowCrossOrigin: false, maxResponseBytes: 1024, timeoutMs: 1000, redirect: 'error' } });

it('preserves response metadata and streams original bytes through the shared HTTP contract', async () => {
    const fetch = vi.fn(async () => new Response('original', { headers: { 'content-type': 'application/json', 'x-source': 'native' } }));
    vi.stubGlobal('fetch', fetch);
    const loaded = await openCemHttpResource(request());
    expect(loaded.response.contentType).toBe('application/json');
    expect(loaded.response.headers['x-source']).toBe('native');
    const chunks = [];
    for await (const chunk of loaded.body) chunks.push(...chunk);
    expect(new TextDecoder().decode(new Uint8Array(chunks))).toBe('original');
    expect(fetch).toHaveBeenCalledWith('https://example.test/data', expect.objectContaining({ redirect: 'error' }));
});

it('refuses unauthorized cross-origin transport before fetch', async () => {
    const fetch = vi.fn(); vi.stubGlobal('fetch', fetch);
    await expect(openCemHttpResource({ ...request(), resolvedUrl: 'https://foreign.test/data' })).rejects.toThrow('cross-origin');
    expect(fetch).not.toHaveBeenCalled();
});

it('cancels an HTTP error body', async () => {
    const cancel = vi.fn();
    vi.stubGlobal('fetch', async () => new Response(new ReadableStream({ cancel }), { status: 404 }));
    await expect(openCemHttpResource(request())).rejects.toThrow('HTTP 404');
    expect(cancel).toHaveBeenCalledOnce();
});

it('cancels and releases a body returned before its first read', async () => {
    const cancel = vi.fn();
    const response = new Response(new ReadableStream({ cancel }));
    const body = streamCemHttpResponseBody(response)[Symbol.asyncIterator]();
    await body.return?.();
    expect(cancel).toHaveBeenCalledOnce();
    expect(response.body?.locked).toBe(false);
});

it('cancels early iteration without waiting for an uncooperative underlying cancel', async () => {
    const cancel = vi.fn(() => new Promise<void>(() => undefined));
    const response = new Response(new ReadableStream<Uint8Array>({ start(controller) { controller.enqueue(new Uint8Array([1])); }, cancel }));
    for await (const chunk of streamCemHttpResponseBody(response)) { expect(chunk).toEqual(new Uint8Array([1])); break; }
    expect(cancel).toHaveBeenCalledOnce();
    expect(response.body?.locked).toBe(false);
});
