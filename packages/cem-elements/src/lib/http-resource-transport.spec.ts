import { once } from 'node:events';
import { readFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { afterEach, expect, it, vi } from 'vitest';
import { openCemHttpResource, streamCemHttpResponseBody, type CemHttpPostRequest } from './http-resource-transport.js';
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

const post = (body: Uint8Array = new Uint8Array([1, 2, 3])): CemHttpPostRequest => {
    const base = request();
    return { ...base, method: 'POST', body, policy: { ...base.policy, maxRequestBytes: 1024 } };
};

it('sends an owned copy of the exact bounded request view', async () => {
    const storage = new Uint8Array([0, 1, 2, 3, 0]);
    const view = storage.subarray(1, 4);
    const value = post(view);
    value.policy.maxRequestBytes = view.byteLength;
    const fetch = vi.fn(async (_url: string, options: RequestInit) => {
        await Promise.resolve();
        expect(options.body).toEqual(new Uint8Array([1, 2, 3]));
        expect(options.body).not.toBe(view);
        expect(options.signal).toBe(value.signal);
        expect(options).toMatchObject({ method: 'POST', cache: 'no-store', redirect: 'error' });
        return new Response('ok');
    });
    vi.stubGlobal('fetch', fetch);
    const sent = openCemHttpResource({ ...value, cache: 'force-cache' });
    storage.fill(0);
    const loaded = await sent;
    await loaded.body[Symbol.asyncIterator]().return?.();
    expect(fetch).toHaveBeenCalledOnce();
});

it('refuses invalid POST sizes, non-POST bodies and unsafe policies before fetch', async () => {
    const fetch = vi.fn();
    vi.stubGlobal('fetch', fetch);
    for (const limit of [undefined, 0, -1, 2.5, Infinity, 1_048_577]) {
        const value = post();
        await expect(openCemHttpResource({ ...value, policy: { ...value.policy, maxRequestBytes: limit as number } })).rejects.toThrow('request byte limit');
    }
    const value = post();
    await expect(openCemHttpResource({ ...value, policy: { ...value.policy, maxRequestBytes: 2 } })).rejects.toThrow('request byte limit');
    await expect(openCemHttpResource({ ...value, body: undefined } as unknown as CemHttpPostRequest)).rejects.toThrow('nonempty byte body');
    await expect(openCemHttpResource(post(new Uint8Array()))).rejects.toThrow('nonempty byte body');
    for (const method of ['GET', 'HEAD'] as const) {
        await expect(openCemHttpResource({ ...value, method })).rejects.toThrow('body requires POST');
    }
    await expect(openCemHttpResource({ ...value, policy: { ...value.policy, redirect: 'follow' } })).rejects.toThrow('redirect');
    await expect(openCemHttpResource({ ...value, resolvedUrl: 'https://foreign.test/data' })).rejects.toThrow('cross-origin');
    await expect(openCemHttpResource({ ...value, resolvedUrl: 'https://user:pass@example.test/data' })).rejects.toThrow('HTTP URL');
    const controller = new AbortController();
    controller.abort();
    await expect(openCemHttpResource({ ...value, signal: controller.signal })).rejects.toMatchObject({ name: 'AbortError' });
    expect(fetch).not.toHaveBeenCalled();
});

it('sends GraphQL JSON as original bytes and refuses a redirect without replaying the POST', async () => {
    const requests: { url?: string; method?: string; contentType?: string; bytes: Buffer }[] = [];
    const fixture = new URL('../../../cem_ql/fixtures/component-application/shopping-cart/', import.meta.url);
    const responseBytes = readFileSync(new URL('updated.json', fixture));
    const server = createServer(async (incoming, outgoing) => {
        const chunks: Buffer[] = [];
        for await (const chunk of incoming) chunks.push(chunk);
        requests.push({ url: incoming.url, method: incoming.method, contentType: incoming.headers['content-type'], bytes: Buffer.concat(chunks) });
        if (incoming.url === '/redirect') {
            outgoing.writeHead(307, { Location: '/graphql' }).end();
        } else {
            outgoing.writeHead(200, { 'Content-Type': 'application/graphql-response+json' }).end(responseBytes);
        }
    });
    server.listen(0, '127.0.0.1');
    await once(server, 'listening');
    try {
        const address = server.address();
        if (!address || typeof address === 'string') throw new Error('Missing loopback address');
        const baseUrl = `http://127.0.0.1:${address.port}/`;
        const bytes = new TextEncoder().encode(JSON.stringify({
            operationName: 'SetCartLineQuantity',
            query: readFileSync(new URL('operations.graphql', fixture), 'utf8'),
            variables: { cartId: 'cart-1', lineId: 'tea', quantity: 3 },
        }));
        const value = post(bytes);
        const headers = { 'Content-Type': 'application/json', Accept: 'application/graphql-response+json' };
        const loaded = await openCemHttpResource({ ...value, baseUrl, resolvedUrl: baseUrl + 'graphql', headers });
        expect(loaded.response.contentType).toBe('application/graphql-response+json');
        const response = [];
        for await (const chunk of loaded.body) response.push(chunk);
        expect(Buffer.concat(response)).toEqual(responseBytes);
        await expect(openCemHttpResource({ ...value, baseUrl, resolvedUrl: baseUrl + 'redirect', headers })).rejects.toThrow();
        expect(requests).toEqual(['/graphql', '/redirect'].map(url => ({
            url, method: 'POST', contentType: 'application/json', bytes: Buffer.from(bytes),
        })));
    } finally {
        await new Promise<void>((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    }
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
