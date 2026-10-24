import type { CemHttpRequest, CemHttpResourceLoadResult } from './cem-elements.js';

/** Shared HTTP byte transport. Interpretation and source admission belong to native consumers. */
export async function openCemHttpResource(request: CemHttpRequest): Promise<CemHttpResourceLoadResult> {
    if (!request.policy.allowCrossOrigin && new URL(request.baseUrl).origin !== new URL(request.resolvedUrl).origin) {
        throw new Error(`cross-origin http-request \`${request.resolvedUrl}\` requires host policy`);
    }
    const response = await fetch(request.resolvedUrl, {
        method: request.method, headers: request.headers,
        credentials: request.credentials as RequestCredentials | undefined,
        cache: request.cache as RequestCache | undefined,
        redirect: request.policy.redirect, signal: request.signal,
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
