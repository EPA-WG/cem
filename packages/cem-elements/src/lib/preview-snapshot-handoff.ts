import { exportCemPreviewSnapshot, importCemPreviewSnapshot, type CemPreviewSnapshot, type CemPreviewSnapshotStore } from './preview-snapshot.js';

export interface CemPreviewSnapshotHandoffOptions { signal?: AbortSignal; timeoutMs?: number }
export interface CemPreviewSnapshotOffer {
    /** One-use bearer token, intended for a trusted same-origin viewer. */
    readonly token: string;
    readonly completed: Promise<void>;
    cancel(): void;
}
const protocol = 'cem-preview-snapshot/1';
const uuid = (value: unknown): value is string => typeof value === 'string' && /^[\da-f]{8}-[\da-f]{4}-4[\da-f]{3}-[89ab][\da-f]{3}-[\da-f]{12}$/i.test(value);
const error = (reason: string) => new Error(`cem.preview.snapshot_handoff_${reason}`);
function deadline(options: CemPreviewSnapshotHandoffOptions): number {
    const value = options.timeoutMs ?? 15000;
    if (!Number.isSafeInteger(value) || value < 1 || value > 60000) throw error('invalid');
    options.signal?.throwIfAborted();
    return value;
}
interface Message { protocol: typeof protocol; type: string; client: string; snapshot?: unknown }
function message(value: unknown): value is Message {
    if (!value || typeof value !== 'object') return false;
    const candidate = value as Partial<Message>;
    return candidate.protocol === protocol && typeof candidate.type === 'string' && uuid(candidate.client);
}

/** No opener/window reference or durable storage. BroadcastChannel supplies the
 * same-origin/storage-partition boundary; the random token scopes one session.
 * A receiver validates its own bounded copy before requesting commit. */
export function offerCemPreviewSnapshot(snapshot: CemPreviewSnapshot, options: CemPreviewSnapshotHandoffOptions = {}): CemPreviewSnapshotOffer {
    options = { ...options };
    const timeout = deadline(options), token = crypto.randomUUID(), held = snapshot.retain();
    let channel: BroadcastChannel;
    try { channel = new BroadcastChannel(`cem-preview-snapshot-v1:${token}`); }
    catch (cause) { held.dispose(); throw cause; }
    let peer: string | undefined, phase: 'waiting' | 'sent' | 'committed' | 'done' = 'waiting';
    let resolve!: () => void, reject!: (reason: unknown) => void;
    const completed = new Promise<void>((yes, no) => { resolve = yes; reject = no; });
    const send = (type: string, client: string, snapshot?: unknown) => channel.postMessage({ protocol, type, client, ...(snapshot === undefined ? {} : { snapshot }) });
    const finish = (cause?: unknown) => {
        if (phase === 'done') return;
        phase = 'done'; clearTimeout(timer);
        options.signal?.removeEventListener('abort', cancelled); held.signal.removeEventListener('abort', released);
        channel.onmessage = channel.onmessageerror = null; channel.close(); held.dispose();
        if (cause !== undefined) reject(cause); else resolve();
    };
    const fail = (cause: unknown) => {
        if (phase === 'done') return;
        try { if (peer) send('cancel', peer); } catch { /* Local release must still run. */ }
        finish(cause ?? error('cancelled'));
    };
    const cancelled = () => fail(options.signal?.reason ?? error('cancelled'));
    const released = () => fail(error('released'));
    const timer = setTimeout(() => fail(error('expired')), timeout);
    options.signal?.addEventListener('abort', cancelled, { once: true }); held.signal.addEventListener('abort', released, { once: true });
    channel.onmessageerror = () => fail(error('invalid'));
    channel.onmessage = event => {
        const value: unknown = event.data;
        if (!message(value) || phase === 'done') return;
        try {
            if (value.type === 'hello') {
                if (peer) { if (peer !== value.client) send('unavailable', value.client); return; }
                peer = value.client;
                const payload = exportCemPreviewSnapshot(held);
                phase = 'sent'; send('snapshot', peer, payload);
            } else if (value.client === peer) {
                if (value.type === 'accept' && phase === 'sent') {
                    held.signal.throwIfAborted(); options.signal?.throwIfAborted();
                    phase = 'committed'; send('commit', peer);
                } else if (value.type === 'done' && phase === 'committed') finish();
                else if (value.type === 'reject' && phase !== 'committed') fail(error('refused'));
                else if (value.type === 'cancel') fail(error('cancelled'));
            }
        } catch (cause) { fail(cause); }
    };
    // A caller may wait for viewer setup before observing completion. Keep the
    // original rejection observable without an unhandled-rejection side effect.
    void completed.catch(() => undefined);
    return Object.freeze({ token, completed, cancel: () => fail(error('cancelled')) });
}

/** The imported registration belongs to this store/realm, independently of the
 * sender. Uncommitted candidates are released on every failure path. */
export async function receiveCemPreviewSnapshot(store: CemPreviewSnapshotStore, token: string, options: CemPreviewSnapshotHandoffOptions = {}): Promise<CemPreviewSnapshot> {
    options = { ...options };
    const timeout = deadline(options);
    if (!uuid(token)) throw error('invalid');
    const client = crypto.randomUUID(), channel = new BroadcastChannel(`cem-preview-snapshot-v1:${token}`);
    return new Promise<CemPreviewSnapshot>((resolve, reject) => {
        let candidate: CemPreviewSnapshot | undefined, done = false;
        const send = (type: string) => channel.postMessage({ protocol, type, client });
        const cleanup = () => {
            clearTimeout(timer); options.signal?.removeEventListener('abort', cancelled);
            candidate?.signal.removeEventListener('abort', released);
            channel.onmessage = channel.onmessageerror = null; channel.close();
        };
        const fail = (cause: unknown) => {
            if (done) return;
            done = true;
            try { send('reject'); } catch { /* Expiry also releases an unreachable sender. */ }
            cleanup(); candidate?.dispose(); candidate = undefined; reject(cause);
        };
        const cancelled = () => fail(options.signal?.reason ?? error('cancelled'));
        const released = () => fail(error('released'));
        const timer = setTimeout(() => fail(error('expired')), timeout);
        options.signal?.addEventListener('abort', cancelled, { once: true });
        channel.onmessageerror = () => fail(error('invalid'));
        channel.onmessage = event => {
            const value: unknown = event.data;
            if (!message(value) || value.client !== client || done) return;
            try {
                if (value.type === 'snapshot') {
                    if (candidate) throw error('invalid');
                    candidate = importCemPreviewSnapshot(store, value.snapshot);
                    candidate.signal.addEventListener('abort', released, { once: true });
                    options.signal?.throwIfAborted(); send('accept');
                } else if (value.type === 'commit') {
                    if (!candidate) throw error('invalid');
                    candidate.signal.throwIfAborted(); options.signal?.throwIfAborted();
                    send('done'); done = true; cleanup();
                    const accepted = candidate; candidate = undefined; resolve(accepted);
                } else if (['cancel', 'unavailable'].includes(value.type)) fail(error(value.type));
            } catch (cause) { fail(cause); }
        };
        try { send('hello'); } catch (cause) { fail(cause); }
    });
}
