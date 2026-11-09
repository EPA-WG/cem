// Transport for candidate SSR resource receipts. Native code verifies bytes.
export const SSR_DECLARATION_RECEIPT = 'data-cem-ssr-declaration';
export const SSR_PAYLOAD_RECEIPT = 'data-cem-ssr-payload';

interface SourceReceipt {
    url: string;
    contentType: string;
    byteLength: number;
    hash: string;
}

export function readSsrSourceReceipt(raw: string, limit: number): SourceReceipt {
    if (raw.length > 8192) throw new Error('cem.ssr.resume_resource_bounds');
    const receipt = JSON.parse(raw) as Partial<SourceReceipt> | null;
    if (!receipt || Object.keys(receipt).length !== 4 || typeof receipt.url !== 'string' || receipt.url.length > 4096
        || typeof receipt.contentType !== 'string' || typeof receipt.hash !== 'string'
        || !/^[0-9a-f]{64}$/u.test(receipt.hash) || typeof receipt.byteLength !== 'number'
        || !Number.isSafeInteger(receipt.byteLength) || receipt.byteLength < 0 || receipt.byteLength > limit) {
        throw new Error('cem.ssr.resume_resource_identity');
    }
    const url = new URL(receipt.url);
    if (!['http:', 'https:'].includes(url.protocol) || url.href !== receipt.url || url.username || url.password
        || url.search || url.hash) throw new Error('cem.ssr.resume_resource_url');
    return receipt as SourceReceipt;
}

export async function fetchSsrSource(
    document: Document, receipt: SourceReceipt, allowedTypes: readonly string[], signal?: AbortSignal,
): Promise<Uint8Array> {
    if (new URL(receipt.url).origin !== new URL(document.URL).origin
        || !allowedTypes.includes(receipt.contentType)) {
        throw new Error('cem.ssr.resume_resource_authority');
    }
    const controller = new AbortController();
    const abort = () => controller.abort();
    if (signal?.aborted) abort();
    signal?.addEventListener('abort', abort, { once: true });
    const timer = setTimeout(abort, 5000);
    try {
        const response = await fetch(receipt.url, {
            mode: 'same-origin', credentials: 'omit', redirect: 'error', cache: 'no-store',
            signal: controller.signal, headers: { accept: receipt.contentType },
        });
        if (response.status !== 200 || response.url !== receipt.url
            || response.headers.has('content-range')
            || response.headers.get('content-type')?.split(';')[0].trim().toLowerCase() !== receipt.contentType
            || !response.body) {
            throw new Error('cem.ssr.resume_resource_response');
        }
        const bytes = new Uint8Array(receipt.byteLength);
        let offset = 0;
        const reader = response.body.getReader();
        try {
            for (;;) {
                const { done, value } = await reader.read();
                if (done) break;
                if (value.byteLength > bytes.length - offset) throw new Error('cem.ssr.resume_resource_bounds');
                bytes.set(value, offset);
                offset += value.byteLength;
            }
            if (offset !== bytes.length) throw new Error('cem.ssr.resume_resource_identity');
            return bytes;
        } finally {
            reader.releaseLock();
        }
    } finally {
        controller.abort();
        clearTimeout(timer);
        signal?.removeEventListener('abort', abort);
    }
}
