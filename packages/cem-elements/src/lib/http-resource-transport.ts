import type { CemHttpRequest, CemHttpResourceLoadResult, CemHttpResourcePolicy } from './cem-elements.js';

const MAX_HTTP_POST_BYTES = 1_048_576;

/** An explicit host dispatch with a bounded original request body. */
export interface CemHttpPostRequest extends Omit<CemHttpRequest, 'method' | 'policy'> {
    method: 'POST';
    body: Uint8Array;
    policy: CemHttpResourcePolicy & { maxRequestBytes: number };
}

/** Shared HTTP byte transport. Interpretation and source admission belong to native consumers. */
export async function openCemHttpResource(request: CemHttpRequest | CemHttpPostRequest): Promise<CemHttpResourceLoadResult> {
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
    const response = await fetch(request.resolvedUrl, {
        method, headers: request.headers,
        credentials: request.credentials as RequestCredentials | undefined,
        cache: method === 'POST' ? 'no-store' : request.cache as RequestCache | undefined,
        redirect: policy.redirect, signal,
        ...(body === undefined ? {} : { body }),
    });
    if (!response.ok) {
        void response.body?.cancel().catch(() => undefined);
        throw new Error(`HTTP ${response.status} for ${request.resolvedUrl}`);
    }
    const headers: Record<string, string> = {};
    response.headers.forEach((value, name) => { headers[name.toLowerCase()] = value; });
    return {
        response: { url: response.url, status: response.status, statusText: response.statusText,
            ok: response.ok, redirected: response.redirected, headers, contentType: response.headers.get('content-type') },
        body: streamCemHttpResponseBody(response),
    };
}

export function streamCemHttpResponseBody(response: Response): AsyncIterable<Uint8Array> {
    if (!response.body) return { async *[Symbol.asyncIterator]() { yield new Uint8Array(await response.arrayBuffer()); } };
    const reader = response.body.getReader();
    const cancel = () => { void reader.cancel().catch(() => undefined); };
    const body = (async function*() {
        let completed = false;
        try {
            for (;;) {
                const chunk = await reader.read();
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
