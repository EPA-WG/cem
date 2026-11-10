// Transport for candidate SSR resource receipts. Native code verifies bytes.
export const SSR_DECLARATION_RECEIPT = 'data-cem-ssr-declaration';
export const SSR_PAYLOAD_RECEIPT = 'data-cem-ssr-payload';
export const SSR_STYLESHEETS_RECEIPT = 'data-cem-ssr-stylesheets';

export const SSR_STYLESHEET_RESOURCE_TYPES = [
    'text/css', 'image/png', 'image/jpeg', 'image/gif', 'image/webp', 'image/avif',
    'font/woff', 'font/woff2', 'font/otf', 'font/ttf',
] as const;
const SSR_RESOURCE_TYPES = ['text/html', 'application/xhtml+xml', 'text/cem-ml', ...SSR_STYLESHEET_RESOURCE_TYPES] as const;

/** A trusted runtime grant for one public SSR response on one page origin. */
export interface CemSsrCrossOriginReadGrant {
    documentOrigin: string;
    url: string;
    contentType: typeof SSR_RESOURCE_TYPES[number];
}

type SsrCrossOriginReads = ReadonlyMap<string, ReadonlyMap<string, string>>;

export function compileSsrCrossOriginReads(grants: readonly CemSsrCrossOriginReadGrant[] = []): SsrCrossOriginReads {
    if (!Array.isArray(grants) || grants.length > 64) throw new TypeError('Invalid SSR cross-origin read grants');
    const reads = new Map<string, Map<string, string>>();
    for (const grant of grants) {
        if (!grant || typeof grant.documentOrigin !== 'string' || grant.documentOrigin.length > 4096
            || typeof grant.url !== 'string' || !SSR_RESOURCE_TYPES.includes(grant.contentType)) {
            throw new TypeError('Invalid SSR cross-origin read grant');
        }
        const document = new URL(grant.documentOrigin);
        const url = resourceUrl(grant.url);
        if (!['http:', 'https:'].includes(document.protocol) || document.origin !== grant.documentOrigin
            || document.origin === url.origin || grant.documentOrigin.includes('*') || /[?#*]/u.test(grant.url)
            || (url.protocol === 'http:' && url.hostname !== '[::1]'
                && !/^127(?:\.\d{1,3}){3}$/u.test(url.hostname))) {
            throw new TypeError('Invalid SSR cross-origin read authority');
        }
        const urls = reads.get(document.origin) ?? new Map<string, string>();
        if (urls.has(url.href)) throw new TypeError('Duplicate SSR cross-origin read grant');
        urls.set(url.href, grant.contentType);
        reads.set(document.origin, urls);
    }
    return reads;
}

interface SourceReceipt {
    url: string;
    contentType: string;
    byteLength: number;
    hash: string;
}

export function readSsrSourceReceipt(raw: string, limit: number): SourceReceipt {
    if (raw.length > 8192) throw new Error('cem.ssr.resume_resource_bounds');
    return sourceReceipt(JSON.parse(raw), limit);
}

function sourceReceipt(value: unknown, limit: number): SourceReceipt {
    const receipt = value as Partial<SourceReceipt> | null;
    if (!receipt || Object.keys(receipt).length !== 4 || typeof receipt.url !== 'string' || receipt.url.length > 4096
        || typeof receipt.contentType !== 'string' || receipt.contentType.length > 128 || typeof receipt.hash !== 'string'
        || !/^[0-9a-f]{64}$/u.test(receipt.hash) || typeof receipt.byteLength !== 'number'
        || !Number.isSafeInteger(receipt.byteLength) || receipt.byteLength < 0 || receipt.byteLength > limit) {
        throw new Error('cem.ssr.resume_resource_identity');
    }
    resourceUrl(receipt.url);
    return receipt as SourceReceipt;
}

function resourceUrl(raw: string): URL {
    if (raw.length > 4096) throw new Error('cem.ssr.resume_resource_url');
    const url = new URL(raw);
    if (!['http:', 'https:'].includes(url.protocol) || url.href !== raw || url.username || url.password
        || url.search || url.hash) throw new Error('cem.ssr.resume_resource_url');
    return url;
}

export function collectSsrStylesheetReceipts(owners: readonly HTMLElement[]): SourceReceipt[] {
    const receipts = new Map<string, SourceReceipt>();
    let totalBytes = 0;
    for (const owner of owners) {
        const raw = owner.getAttribute(SSR_STYLESHEETS_RECEIPT);
        if (raw === null) continue;
        if (raw.length > 64 * 1024) throw new Error('cem.ssr.resume_stylesheet_bounds');
        const entries: unknown = JSON.parse(raw);
        if (!Array.isArray(entries) || entries.length > 64 || entries.every(entry => entry === null)) {
            throw new Error('cem.ssr.resume_stylesheet_identity');
        }
        for (const entry of entries) {
            if (entry === null) continue;
            if (!Array.isArray(entry?.sources) || entry.sources.length === 0 || entry.sources.length >= 64) {
                throw new Error('cem.ssr.resume_stylesheet_identity');
            }
            for (const value of entry.sources) {
                const receipt = sourceReceipt(value, 256 * 1024);
                const previous = receipts.get(receipt.url);
                if (previous) {
                    if (previous.hash !== receipt.hash || previous.contentType !== receipt.contentType
                        || previous.byteLength !== receipt.byteLength) throw new Error('cem.ssr.resume_stylesheet_identity');
                    continue;
                }
                totalBytes += receipt.byteLength;
                receipts.set(receipt.url, receipt);
                if (receipts.size > 64 || totalBytes > 1024 * 1024) throw new Error('cem.ssr.resume_stylesheet_bounds');
            }
        }
    }
    return [...receipts.values()];
}

export async function fetchSsrSource(
    document: Document, receipt: SourceReceipt, allowedTypes: readonly string[], grants: SsrCrossOriginReads,
    signal?: AbortSignal,
): Promise<{ bytes: Uint8Array; crossOrigin: boolean }> {
    const mediaType = (value: string) => value.split(';')[0].trim().toLowerCase();
    const expected = mediaType(receipt.contentType);
    const origin = new URL(document.URL).origin;
    const crossOrigin = new URL(receipt.url).origin !== origin;
    const view = document.defaultView;
    if (!view || view.origin !== origin || !allowedTypes.includes(expected)
        || (crossOrigin && grants.get(origin)?.get(receipt.url) !== expected)) {
        throw new Error('cem.ssr.resume_resource_authority');
    }
    const controller = new AbortController();
    const abort = () => controller.abort();
    if (signal?.aborted) abort();
    signal?.addEventListener('abort', abort, { once: true });
    const timer = setTimeout(abort, 5000);
    try {
        const response = await view.fetch(receipt.url, {
            mode: crossOrigin ? 'cors' : 'same-origin', credentials: 'omit', redirect: 'error', cache: 'no-store',
            ...(crossOrigin ? { referrerPolicy: 'no-referrer' as const } : {}),
            signal: controller.signal, headers: { accept: expected },
        });
        if (response.status !== 200 || response.url !== receipt.url || (crossOrigin && response.type !== 'cors')
            || response.headers.has('content-range')
            || mediaType(response.headers.get('content-type') ?? '') !== expected
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
            return { bytes, crossOrigin };
        } finally {
            reader.releaseLock();
        }
    } finally {
        controller.abort();
        clearTimeout(timer);
        signal?.removeEventListener('abort', abort);
    }
}
