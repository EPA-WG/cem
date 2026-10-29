import { afterEach, expect, it, vi } from 'vitest';
import { readCompiledDeclarationAsset } from './compiled-declaration-assets.js';

const url = 'https://app.test/field.json';
const policy = { allowCrossOrigin: false, maxResponseBytes: 8, timeoutMs: 15_000, redirect: 'error' as const };
afterEach(() => vi.unstubAllGlobals());
const read = (signal = new AbortController().signal) => readCompiledDeclarationAsset(url, 'https://app.test/', 'metadata', policy, signal);
it('preserves exact bytes and requests credential-free, redirect-free assets', async () => {
    const fetcher = vi.fn(async () => new Response(new Uint8Array([123, 125]), { headers: { 'content-type': 'application/json' } }));
    vi.stubGlobal('fetch', fetcher);
    expect([...new Uint8Array(await read())]).toEqual([123, 125]);
    expect(fetcher).toHaveBeenCalledWith(url, expect.objectContaining({ credentials: 'omit', redirect: 'error' }));
});
it('refuses cross-origin, fragment, credential and non-HTTP URLs before fetch', async () => {
    const fetcher = vi.fn(); vi.stubGlobal('fetch', fetcher);
    for (const bad of ['https://other.test/a', `${url}#a`, 'https://user@app.test/a', 'data:text/plain,{}']) {
        await expect(readCompiledDeclarationAsset(bad, url, 'metadata', policy, new AbortController().signal)).rejects.toThrow('policy');
    }
    expect(fetcher).not.toHaveBeenCalled();
});
it('refuses bad HTTP, MIME, empty bodies and oversized declared lengths', async () => {
    for (const response of [new Response('oops', { status: 404 }), new Response('<html>', { headers: { 'content-type': 'text/html' } }),
        new Response('', { headers: { 'content-type': 'application/json' } }),
        new Response('{}', { headers: { 'content-type': 'application/json', 'content-length': '100' } })]) {
        vi.stubGlobal('fetch', vi.fn(async () => response)); await expect(read()).rejects.toThrow('compiled asset');
    }
});
it('bounds streamed bytes even without a content length and releases the body', async () => {
    const cancel = vi.fn(); const response = new Response(new ReadableStream({ start(c) { c.enqueue(new Uint8Array(9)); }, cancel }), { headers: { 'content-type': 'application/json' } });
    vi.stubGlobal('fetch', vi.fn(async () => response));
    await expect(read()).rejects.toThrow('byte limit'); expect(cancel).toHaveBeenCalled(); expect(response.body?.locked).toBe(false);
});
it('settles abort despite an uncooperative fetch or stream cancellation', async () => {
    vi.stubGlobal('fetch', vi.fn(() => new Promise(() => undefined)));
    const first = new AbortController(), pending = read(first.signal); first.abort();
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
    const response = new Response(new ReadableStream({ cancel: () => new Promise(() => undefined) }), { headers: { 'content-type': 'application/json' } });
    vi.stubGlobal('fetch', vi.fn(async () => response));
    const next = new AbortController(), body = read(next.signal); await Promise.resolve(); await Promise.resolve(); next.abort();
    await expect(body).rejects.toMatchObject({ name: 'AbortError' }); expect(response.body?.locked).toBe(false);
});
