import type { CemHttpResourcePolicy } from './cem-elements.js';

/** Bounded byte transport only. Declaration interpretation and integrity stay native. */
export async function readCompiledDeclarationAsset(url: string, baseUrl: string, kind: 'metadata' | 'template',
    policy: CemHttpResourcePolicy, signal: AbortSignal): Promise<ArrayBuffer> {
    const target = new URL(url), base = new URL(baseUrl);
    if (!['http:', 'https:'].includes(target.protocol) || target.hash || target.username || target.password
        || (!policy.allowCrossOrigin && target.origin !== base.origin)) throw Error('compiled asset URL requires host policy');
    const limit = Math.min(kind === 'metadata' ? 1024 * 1024 : 64 * 1024 * 1024, policy.maxResponseBytes);
    if (!Number.isSafeInteger(limit) || limit <= 0) throw Error('invalid compiled asset byte limit');
    const guarded = <T>(work: Promise<T>) => compiledDeclarationWork(work, signal);
    signal.throwIfAborted();
    const pending = fetch(target.href, { signal, redirect: 'error', credentials: 'omit',
        headers: { Accept: kind === 'metadata' ? 'application/json' : 'application/octet-stream' } });
    // An uncooperative transport may resolve after cancellation; release its body too.
    void pending.then(response => { if (signal.aborted) void response.body?.cancel().catch(() => undefined); }, () => undefined);
    const response = await guarded(pending);
    const cancel = () => { void response.body?.cancel().catch(() => undefined); };
    const mime = response.headers.get('content-type')?.split(';')[0].trim().toLowerCase();
    if (!response.ok || response.redirected || response.url && response.url !== target.href
        || mime !== (kind === 'metadata' ? 'application/json' : 'application/octet-stream')
        || Number(response.headers.get('content-length')) > limit) {
        cancel(); throw Error(`invalid compiled asset response: HTTP ${response.status}`);
    }
    const reader = response.body?.getReader();
    if (!reader) throw Error('compiled asset response has no body');
    const chunks: Uint8Array[] = []; let length = 0;
    const abort = () => { void reader.cancel(signal.reason).catch(() => undefined); };
    signal.addEventListener('abort', abort, { once: true });
    try {
        for (;;) {
            const chunk = await guarded(reader.read()); signal.throwIfAborted();
            if (chunk.done) break;
            length += chunk.value.byteLength;
            if (length > limit) throw Error('compiled asset response exceeds byte limit');
            chunks.push(chunk.value.slice());
        }
        if (!length) throw Error('compiled asset response is empty');
        const bytes = new Uint8Array(length); let offset = 0;
        for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
        return bytes.buffer;
    } finally {
        signal.removeEventListener('abort', abort); abort(); reader.releaseLock();
    }
}

/** Drain late results without allowing an uncooperative acquisition to hold admission. */
export function compiledDeclarationWork<T>(work: Promise<T>, signal: AbortSignal): Promise<T> {
    let abort: () => void;
    return new Promise<T>((resolve, reject) => {
        abort = () => reject(signal.reason);
        signal.addEventListener('abort', abort, { once: true });
        work.then(resolve, reject);
        if (signal.aborted) abort();
    }).finally(() => signal.removeEventListener('abort', abort));
}
