import { afterEach, expect, it, vi } from 'vitest';
import { readRetainedStylesheet } from './stylesheet-reader.js';

const request = { id: 1, url: 'https://example.test/source.css', contentType: 'text/css', integrity: null };
afterEach(() => vi.unstubAllGlobals());
function fetchResponse(response: Response) {
    const fetcher = vi.fn(async () => response);
    vi.stubGlobal('fetch', fetcher);
    return fetcher;
}

it('preserves bytes, redirected URL and actual MIME metadata for native admission', async () => {
    const response = new Response(new Uint8Array([0, 255, 195, 169]), { headers: { 'content-type': 'text/css; charset=utf-8' } });
    Object.defineProperty(response, 'url', { value: 'https://example.test/cdn/final.css' });
    const fetcher = fetchResponse(response);
    const signal = new AbortController().signal;
    const result = await readRetainedStylesheet(request, signal);
    expect(fetcher).toHaveBeenCalledWith(request.url, { signal });
    expect(result).toMatchObject({ finalUrl: response.url, contentType: 'text/css; charset=utf-8' });
    expect([...new Uint8Array(result.bytes)]).toEqual([0, 255, 195, 169]);
    expect(response.body?.locked).toBe(false);
});

it('does not substitute a request hint for a missing response content type', async () => {
    const response = new Response(new Uint8Array());
    Object.defineProperty(response, 'url', { value: request.url });
    fetchResponse(response);
    expect(await readRetainedStylesheet(request, new AbortController().signal)).toMatchObject({
        bytes: new ArrayBuffer(0), finalUrl: request.url, contentType: undefined,
    });
});

it('rejects failed HTTP responses and cancels their body without reading it', async () => {
    const cancel = vi.fn();
    const response = new Response(new ReadableStream({ cancel }), { status: 404 });
    fetchResponse(response);
    await expect(readRetainedStylesheet(request, new AbortController().signal)).rejects.toThrow('HTTP 404');
    expect(cancel).toHaveBeenCalledTimes(1);
});

it('cancels an outstanding read on abort even if stream cancellation never settles', async () => {
    const cancel = vi.fn(() => new Promise<void>(() => undefined));
    const response = new Response(new ReadableStream({ cancel }));
    const fetcher = fetchResponse(response);
    const controller = new AbortController();
    const pending = readRetainedStylesheet(request, controller.signal);
    // Resume the reader after the fetch promise.
    await Promise.resolve();
    expect(response.body?.locked).toBe(true);
    controller.abort();
    await expect(pending).rejects.toMatchObject({ name: 'AbortError' });
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(response.body?.locked).toBe(false);
    await expect(readRetainedStylesheet(request, controller.signal)).rejects.toMatchObject({ name: 'AbortError' });
    expect(fetcher).toHaveBeenCalledTimes(1);
});

it('caps buffered bytes at the native limit plus one and cancels excess stream data', async () => {
    const limit = 16 * 1024 * 1024;
    let reads = 0;
    const cancel = vi.fn();
    const response = new Response(new ReadableStream<Uint8Array>({
        pull(controller) { reads++; controller.enqueue(new Uint8Array(limit + 100).fill(42)); }, cancel,
    }));
    fetchResponse(response);
    const result = await readRetainedStylesheet(request, new AbortController().signal);
    expect(result.bytes.byteLength).toBe(limit + 1);
    expect(new Uint8Array(result.bytes)[limit]).toBe(42);
    expect(reads).toBeLessThanOrEqual(2); // A stream can prefetch one chunk.
    expect(cancel).toHaveBeenCalledTimes(1);
    expect(response.body?.locked).toBe(false);
});

it('preserves network read failures and releases the stream lock', async () => {
    const response = new Response(new ReadableStream({ pull(controller) { controller.error(new Error('network interrupted')); } }));
    fetchResponse(response);
    await expect(readRetainedStylesheet(request, new AbortController().signal)).rejects.toThrow('network interrupted');
    expect(response.body?.locked).toBe(false);
});
