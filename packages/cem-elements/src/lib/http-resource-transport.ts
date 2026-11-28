import { applicationNetworkLease } from './application-resources.js';
import type { CemHttpRequest, CemHttpResourceLoadResult, CemHttpResourcePolicy } from './cem-elements.js';

const MAX_HTTP_POST_BYTES = 1_048_576;

/** An explicit host dispatch with a bounded original request body. */
export interface CemHttpPostRequest extends Omit<CemHttpRequest, 'method' | 'policy'> {
    method: 'POST';
    body: Uint8Array;
    policy: CemHttpResourcePolicy & { maxRequestBytes: number };
}

/** Trusted single-hop transport. It must honor cancellation, return redirect headers without
 * following them, and implement the host's endpoint policy at the actual connection. */
export type CemApplicationHttpHop = (url: string, init: RequestInit) => Promise<Response>;
type Lease = NonNullable<ReturnType<typeof applicationNetworkLease>>;
type HttpRequest = CemHttpRequest | CemHttpPostRequest;

export async function openCemApplicationHttpResource(consumer: Element, request: HttpRequest, hop?: CemApplicationHttpHop) {
    const lease = applicationNetworkLease(consumer);
    if (!lease) throw new DOMException('Application boundary required', 'NotAllowedError');
    return openCemHttpResource(request, lease, hop);
}

async function applicationFetch(request: HttpRequest, init: RequestInit, lease: Lease, hop?: CemApplicationHttpHop) {
    let url = new URL(request.resolvedUrl);
    url.hash = '';
    const credentials = (request.credentials ?? 'omit') as RequestCredentials;
    const signal = AbortSignal.any([request.signal, lease.signal]);
    let headers = new Headers(init.headers);
    if ((credentials === 'omit' || credentials === 'same-origin' && url.origin !== globalThis.location.origin)
        && ['authorization', 'proxy-authorization', 'cookie'].some(name => headers.has(name)))
        throw new DOMException('Credentials require a network grant', 'NotAllowedError');
    for (let redirects = 0; ; redirects++) {
        signal.throwIfAborted();
        lease.check(url, request.method, credentials);
        if (!request.policy.allowCrossOrigin && url.origin !== new URL(request.baseUrl).origin)
            throw new DOMException('Cross-origin network operation denied', 'NotAllowedError');
        const response = await (hop ?? fetch)(url.href, { ...init, headers, credentials, signal,
            redirect: hop ? 'manual' : 'error' });
        let consuming = false;
        try {
            signal.throwIfAborted();
            lease.current();
            if (response.redirected || response.type === 'opaqueredirect' || response.url && response.url !== url.href)
                throw new Error('Single-hop transport returned an unverified destination');
            if (![301, 302, 303, 307, 308].includes(response.status)) {
                consuming = true;
                return { response, finalUrl: url.href, redirects };
            }
            if (!hop || request.method === 'POST' || request.policy.redirect !== 'follow' || redirects >= 10)
                throw new Error('Application redirect refused');
            lease.check(url, request.method, credentials, true);
            const target = response.headers.get('location');
            if (!target) throw new Error('Redirect destination unavailable');
            const next = new URL(target, url);
            next.hash = '';
            if (next.origin !== url.origin) headers = new Headers();
            url = next;
        } finally {
            if (!consuming)
                void response.body?.cancel().catch(() => undefined);
        }
    }
}

/** Shared HTTP byte transport. Interpretation and source admission belong to native consumers. */
export async function openCemHttpResource(request: HttpRequest, lease?: Lease, hop?: CemApplicationHttpHop): Promise<CemHttpResourceLoadResult> {
    const { method, policy, signal } = request;
    if (!policy.allowCrossOrigin && new URL(request.baseUrl).origin !== new URL(request.resolvedUrl).origin) {
        throw new Error(`cross-origin http-request \`${request.resolvedUrl}\` requires host policy`);
    }
    let body: Uint8Array<ArrayBuffer> | undefined;
    if (method === 'POST') {
        signal.throwIfAborted();
        const url = new URL(request.resolvedUrl);
        if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
            throw new Error('POST requires an HTTP URL without user info');
        }
        if (policy.redirect !== 'error') {
            throw new Error('POST requires redirect refusal');
        }
        const bytes = request.body;
        if (!(bytes instanceof Uint8Array) || bytes.byteLength === 0) {
            throw new Error('POST requires a nonempty byte body');
        }
        const limit = policy.maxRequestBytes;
        if (!Number.isSafeInteger(limit) || limit < 1 || limit > MAX_HTTP_POST_BYTES || bytes.byteLength > limit) {
            throw new Error('POST exceeds the authorized request byte limit');
        }
        body = new Uint8Array(bytes);
    } else {
        if (method !== 'GET' && method !== 'HEAD') {
            throw new Error('HTTP method requires GET, HEAD or POST');
        }
        if ('body' in request && request.body !== undefined) {
            throw new Error('HTTP request body requires POST');
        }
    }
    const init: RequestInit = {
        method, headers: request.headers,
        credentials: request.credentials as RequestCredentials | undefined,
        cache: method === 'POST' ? 'no-store' : request.cache as RequestCache | undefined,
        redirect: policy.redirect, signal,
        ...(body === undefined ? {} : { body }),
    };
    const { response, finalUrl, redirects } = lease ? await applicationFetch(request, init, lease, hop)
        : { response: await fetch(request.resolvedUrl, init), finalUrl: '', redirects: 0 };
    if (!response.ok) {
        void response.body?.cancel().catch(() => undefined);
        throw new Error(`HTTP ${response.status} for ${request.resolvedUrl}`);
    }
    const headers: Record<string, string> = {};
    response.headers.forEach((value, name) => { headers[name.toLowerCase()] = value; });
    return {
        response: { url: finalUrl || response.url, status: response.status, statusText: response.statusText,
            ok: response.ok, redirected: redirects > 0 || response.redirected, headers, contentType: response.headers.get('content-type') },
        body: streamCemHttpResponseBody(response, lease?.current),
    };
}

export function streamCemHttpResponseBody(response: Response, current?: () => void): AsyncIterable<Uint8Array> {
    if (!response.body) return { async *[Symbol.asyncIterator]() {
        const bytes = await response.arrayBuffer();
        current?.();
        yield new Uint8Array(bytes);
    } };
    const reader = response.body.getReader();
    const cancel = () => { void reader.cancel().catch(() => undefined); };
    const body = (async function*() {
        let completed = false;
        try {
            for (;;) {
                current?.();
                const chunk = await reader.read();
                current?.();
                if (chunk.done) { completed = true; return; }
                yield chunk.value;
            }
        } finally {
            // A hostile cancellation implementation must not hold retirement open.
            if (!completed) cancel();
            reader.releaseLock();
        }
    })();
    return { [Symbol.asyncIterator]() {
        return { next: () => body.next(), return: () => {
            cancel();
            // Even an unopened generator must release its acquired stream lock.
            return body.return().finally(() => { if (response.body?.locked) reader.releaseLock(); });
        } };
    } };
}

export class HttpResourceError extends Error {
    constructor(readonly code: string, message: string) {
        super(message);
        this.name = 'HttpResourceError';
    }
}

export async function readByteStream(body: AsyncIterable<Uint8Array>, maxResponseBytes: number, signal: AbortSignal): Promise<Uint8Array> {
    const chunks: Uint8Array[] = [];
    let size = 0;
    for await (const chunk of body) {
        if (signal.aborted) {
            throw new HttpResourceError('cem-element.http_request_aborted', 'http-request was aborted');
        }
        if (!chunk.byteLength) continue;
        size += chunk.byteLength;
        if (size > maxResponseBytes) {
            throw new HttpResourceError(
                'cem-element.http_request_response_too_large',
                `http-request response exceeded ${maxResponseBytes} bytes`,
            );
        }
        chunks.push(chunk);
    }
    const bytes = new Uint8Array(size);
    let offset = 0;
    for (const chunk of chunks) {
        bytes.set(chunk, offset);
        offset += chunk.byteLength;
    }
    return bytes;
}
