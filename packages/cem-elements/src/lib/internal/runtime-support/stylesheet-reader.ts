import { applicationNetworkLease } from '../../application-resources.js';
import type { CemStylesheetInstallationOptions } from './stylesheet-installation.js';

// cem_ml::import::MAX_DOCUMENT_BYTES. Preserve one excess byte so native
// admission diagnoses an oversized response instead of accepting truncated CSS.
const responseCapacity = 16 * 1024 * 1024 + 1;

/** Browser byte transport only; CSS parsing, MIME and integrity checks stay native. */
type Reader = CemStylesheetInstallationOptions['read'];
export const readRetainedStylesheet: Reader = (request, signal) => readStylesheet(request, signal);

export function applicationStylesheetReader(consumer: Element, pageReader: Reader): Reader {
    return async (request, signal) => {
        const lease = applicationNetworkLease(consumer);
        if (!lease) return pageReader(request, signal);
        lease.check(new URL(request.url), 'GET', 'omit');
        const result = await readStylesheet(request, AbortSignal.any([signal, lease.signal]), { redirect: 'error', credentials: 'omit' });
        lease.current();
        return result;
    };
}

async function readStylesheet(request: Parameters<Reader>[0], signal: AbortSignal, policy?: RequestInit) {
    signal.throwIfAborted();
    const response = await fetch(request.url, { ...policy, signal, headers: { Accept: 'text/css' } });
    if (!response.ok) {
        void response.body?.cancel().catch(() => undefined);
        throw new Error(`HTTP ${response.status} for ${request.url}`);
    }
    const reader = response.body?.getReader();
    const chunks: Uint8Array[] = [];
    let length = 0;
    if (reader) {
        const cancel = () => { void reader.cancel(signal.reason).catch(() => undefined); };
        signal.addEventListener('abort', cancel, { once: true });
        if (signal.aborted) cancel();
        try {
            for (;;) {
                signal.throwIfAborted();
                const chunk = await reader.read();
                signal.throwIfAborted();
                if (chunk.done) break;
                const bytes = chunk.value.slice(0, responseCapacity - length);
                if (bytes.byteLength) chunks.push(bytes);
                length += bytes.byteLength;
                if (length === responseCapacity) break;
            }
        } finally {
            signal.removeEventListener('abort', cancel);
            // Cancellation may be implemented by an uncooperative underlying source.
            // It must not prevent the caller from releasing its native generation.
            cancel();
            reader.releaseLock();
        }
    }
    signal.throwIfAborted();
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
    return { bytes: bytes.buffer, finalUrl: response.url,
        contentType: response.headers.get('content-type') ?? undefined };
}
