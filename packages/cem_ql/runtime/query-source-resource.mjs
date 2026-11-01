/** Internal source/owner transport only. Native CEMQH1 verification and selected
 * execution stay in Rust. These capabilities never come from authored data. */
import { assertNativeGenerationResourceTarget, assertNativeGenerationResourceBudget, registerNativeGenerationResource } from './native-generation.mjs';
const shared = globalThis[Symbol.for('cem.query-source.endpoints.v1')] ??= { endpoints: new WeakMap() };
const fail = code => { throw Error(`cem.query.resource.${code}`); };
const MAX_BYTES = 69643, MAX_PINS = 16384;
function source(bytes, pins, limit = MAX_BYTES) {
    if (!(bytes instanceof Uint8Array) || !(bytes.buffer instanceof ArrayBuffer) || !bytes.length || bytes.length > limit
        || typeof pins !== 'string' || pins.length > MAX_PINS || new TextEncoder().encode(pins).length > MAX_PINS) fail('limit');
    return bytes.slice();
}
function endpointRecord(endpoint) {
    const record = shared.endpoints.get(endpoint);
    if (!record) fail('endpoint');
    return record;
}
/** Native endpoints must first be provisioned by their trusted Rust embedding.
 * Keep the actual Memory weakly; an equal factory/foreign heap cannot substitute
 * it, and native tokens never become numeric runtime or worker wire handles. */
export function createQuerySourceWasmBackend(runtime) { return wasmBackend(runtime, 'querySource', MAX_BYTES); }
export function createRuleSourceWasmBackend(runtime) { return wasmBackend(runtime, 'ruleSource', 4 * 1024 * 1024); }
function wasmBackend(runtime, prefix, maxSourceBytes) {
    const identity = runtime?.memory?.();
    if (!(identity instanceof WebAssembly.Memory) || typeof runtime?.call !== 'function') fail('endpoint');
    const physical = new WeakRef(identity);
    const call = (name, args) => {
        const original = physical.deref();
        if (!original || runtime.memory() !== original) fail('stale');
        return runtime.call(name, args);
    };
    return Object.freeze({ maxSourceBytes,
        open: (bytes, pins) => call(`${prefix}OpenV1`, [bytes, pins]),
        capture: owner => call(`${prefix}CaptureV1`, [owner]),
        release: owner => call(`${prefix}ReleaseV1`, [owner]),
    });
}
/** Original binding functions must be supplied by the shared physical owner.
 * No native handle is exposed by the returned endpoint or its owner tokens. */
export function createQuerySourceEndpoint(options) {
    const { participant, identity, current, native, timeoutMs = 5000 } = options;
    assertNativeGenerationResourceTarget(participant, undefined, identity);
    if (typeof current !== 'function' || !native || ['open', 'capture', 'release'].some(k => typeof native[k] !== 'function')
        || !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('endpoint');
    const maxSourceBytes = options.maxSourceBytes ?? native.maxSourceBytes ?? MAX_BYTES;
    if (!Number.isSafeInteger(maxSourceBytes) || maxSourceBytes < 1 || maxSourceBytes > 4 * 1024 * 1024) fail('limit');
    const physical = new WeakRef(identity), owners = new Map();
    let closed = false, failed = false, pending = 0;
    const live = () => !closed && !failed && current() === true;
    const cleanupLive = () => !closed && (options.cleanupCurrent ?? current)() === true;
    async function bounded(work, signal, late = () => {}, cleanup = false) {
        const available = cleanup ? cleanupLive : live;
        if (!available()) fail('stale');
        if (pending >= 16) fail('limit');
        if (signal?.aborted) fail('cancelled');
        pending++;
        const deadline = performance.now() + timeoutMs;
        let timer, cancel, expired = false;
        const operation = Promise.resolve().then(work).then(value => {
            if (expired || signal?.aborted || !available() || performance.now() >= deadline) {
                failed = true;
                void Promise.resolve().then(() => late(value)).catch(() => { failed = true; });
                fail('late');
            }
            return value;
        }).finally(() => pending--);
        try {
            return await Promise.race([operation, new Promise((_, reject) => {
                const stop = code => { expired = true; failed = true; reject(Error(`cem.query.resource.${code}`)); };
                timer = setTimeout(() => stop('timeout'), timeoutMs);
                cancel = () => stop('cancelled'); signal?.addEventListener('abort', cancel, { once: true });
            })]);
        } finally { clearTimeout(timer); if (cancel) signal?.removeEventListener('abort', cancel); }
    }
    const original = owner => { const entry = owners.get(owner); if (!entry || entry.releasing) fail('owner'); return entry; };
    const endpoint = Object.freeze({ participant,
        identity: () => physical.deref(), current: () => live() && pending === 0,
        async open(bytes, pins, signal) {
            if (owners.size >= 64) fail('limit');
            const copy = source(bytes, pins, maxSourceBytes);
            const value = await bounded(() => native.open(copy, pins), signal, value => native.release(value));
            if (!live()) { await native.release(value); fail('stale'); }
            if (owners.size >= 64) { await native.release(value); fail('limit'); }
            const owner = Object.freeze({ kind: 'cem-query-source-owner' });
            owners.set(owner, { value, releasing: false }); return owner;
        },
        async capture(owner, signal) {
            const entry = original(owner);
            const bytes = await bounded(() => native.capture(entry.value), signal);
            return source(bytes, '', maxSourceBytes);
        },
        async release(owner, signal) {
            const entry = original(owner); entry.releasing = true;
            await bounded(() => native.release(entry.value), signal, undefined, true);
            owners.delete(owner);
        },
        dispose() {
            if (closed) return;
            closed = true;
            for (const entry of owners.values()) if (!entry.releasing)
                void Promise.resolve().then(() => native.release(entry.value)).catch(() => { failed = true; });
            owners.clear(); options.close?.();
        },
    });
    shared.endpoints.set(endpoint, { participant, physical, maxSourceBytes, current: endpoint.current });
    return endpoint;
}
/** A private MessagePort created/transferred by the trusted Worker owner. The
 * original registered Worker identity is checked separately from all wire IDs. */
export function createQuerySourceWorkerEndpoint(options) {
    const { port, timeoutMs = 5000 } = options;
    assertNativeGenerationResourceTarget(options.participant, undefined, options.identity);
    if (typeof options.current !== 'function' || !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('endpoint');
    let sequence = 0, failed = false, closed = false;
    const pending = new Map();
    const receive = event => {
        const reply = event.data, entry = reply && pending.get(reply.id);
        if (!entry || entry.op !== reply.op || typeof reply.ok !== 'boolean') { failed = true; return; }
        pending.delete(reply.id); clearTimeout(entry.timer);
        if (reply.ok) entry.resolve(reply.value); else entry.reject(Error('cem.query.resource.native-refusal'));
    };
    port.addEventListener('message', receive); port.start();
    const call = (op, value) => new Promise((resolve, reject) => {
        if (closed || failed || pending.size >= 16) { reject(Error('cem.query.resource.transport')); return; }
        const id = ++sequence;
        const timer = setTimeout(() => { pending.delete(id); failed = true; reject(Error('cem.query.resource.timeout')); }, timeoutMs);
        pending.set(id, { op, resolve, reject, timer });
        try { port.postMessage({ id, op, value }); } catch (error) { clearTimeout(timer); pending.delete(id); failed = true; reject(error); }
    });
    return createQuerySourceEndpoint({ ...options,
        current: () => !failed && !closed && options.current() === true,
        cleanupCurrent: () => !closed && (options.cleanupCurrent ?? options.current)() === true,
        close: () => { closed = true; port.removeEventListener('message', receive); port.close();
            for (const entry of pending.values()) { clearTimeout(entry.timer); entry.reject(Error('cem.query.resource.closed')); } pending.clear(); },
        native: {
            open: async (bytes, pins) => { const id = await call('open', { bytes, pins }); if (!Number.isSafeInteger(id) || id < 1) { failed = true; fail('transport'); } return id; },
            capture: id => call('capture', id),
            release: async id => { if (await call('release', id) !== true) { failed = true; fail('transport'); } },
        } });
}
/** Worker-side trusted binding adapter. IDs select only this original private
 * port's retained owners; no source/AST is reconstructed in JavaScript. */
export function installQuerySourceWorkerTransport(port, native) {
    const limit = native.maxSourceBytes ?? MAX_BYTES;
    if (!Number.isSafeInteger(limit) || limit < 1 || limit > 4 * 1024 * 1024) fail('limit');
    const owners = new Map(); let sequence = 0, latest = 0, closed = false, tail = Promise.resolve();
    const receive = event => {
        const request = event.data;
        tail = tail.then(async () => {
            if (closed) return;
            const { id, op, value } = request ?? {};
            try {
                if (!Number.isSafeInteger(id) || id <= latest) fail('transport');
                latest = id;
                let result;
                if (op === 'open') {
                    if (owners.size >= 64) fail('limit');
                    const bytes = source(value?.bytes, value?.pins, limit);
                    const owner = await native.open(bytes, value.pins);
                    if (closed) { await native.release(owner); return; }
                    owners.set(++sequence, owner); result = sequence;
                } else {
                    if (!owners.has(value)) fail('owner');
                    const owner = owners.get(value);
                    if (op === 'capture') result = source(await native.capture(owner), '', limit);
                    else if (op === 'release') { await native.release(owner); owners.delete(value); result = true; }
                    else fail('transport');
                }
                if (!closed) port.postMessage({ id, op, ok: true, value: result });
            } catch { if (!closed) port.postMessage({ id, op, ok: false }); }
        }).catch(() => { closed = true; });
    };
    port.addEventListener('message', receive); port.start();
    return Object.freeze({ async dispose() { closed = true; port.removeEventListener('message', receive); port.close();
        await tail; for (const owner of owners.values()) await native.release(owner); owners.clear(); } });
}

/** Stable source route enrolled in the existing complete native generation
 * transaction. Privacy/policy controls are trusted host callbacks, not bytes. */
export class QuerySourceResource {
    #endpoint; #owner; #bytes; #pins; #options; #registration; #snapshot; #closed = false; #staged;
    static async open(options, signal) {
        const record = endpointRecord(options.endpoint);
        const bytes = source(options.bytes, options.pins, record.maxSourceBytes);
        if (typeof options.current !== 'function' || typeof options.target !== 'function') fail('policy');
        const owner = await options.endpoint.open(bytes, options.pins, signal);
        const resource = new QuerySourceResource();
        resource.#endpoint = options.endpoint; resource.#owner = owner; resource.#bytes = bytes; resource.#pins = options.pins;
        resource.#options = { current: options.current, target: options.target, peers: options.peers, onBound: options.onBound };
        try { resource.#enroll(); } catch (error) { await options.endpoint.release(owner); throw error; }
        return resource;
    }
    async prepareRecovery(signal) {
        this.#snapshot = undefined;
        if (this.#closed || this.#options.current() !== true) fail('policy');
        const endpoint = this.#endpoint, owner = this.#owner;
        const bytes = await endpoint.capture(owner, signal);
        if (this.#closed || this.#endpoint !== endpoint || this.#options.current() !== true || !endpoint.current()
            || bytes.length !== this.#bytes.length || bytes.some((byte, i) => byte !== this.#bytes[i])) fail('stale');
        this.#snapshot = { endpoint, owner, released: false };
    }
    #enroll() {
        const participant = endpointRecord(this.#endpoint).participant;
        this.#registration = registerNativeGenerationResource(participant, () => {
            assertNativeGenerationResourceBudget(participant);
            const snapshot = this.#snapshot, peers = this.#options.peers?.().slice();
            const current = () => !this.#closed && this.#snapshot === snapshot && !!snapshot
                && this.#options.current() === true && (snapshot.released || snapshot.endpoint.current())
                && (!peers || this.#options.peers?.().length === peers.length && peers.every(p => this.#options.peers().includes(p)));
            if (!current()) fail('stale');
            return { current, peers, retainedBytes: this.#bytes.length + new TextEncoder().encode(this.#pins).length,
                release: async () => {
                    if (!current() || snapshot.released) fail('stale');
                    await snapshot.endpoint.release(snapshot.owner);
                    if (!current()) fail('stale');
                    snapshot.released = true;
                },
                prepare: async context => {
                    if (!current() || !snapshot.released || context.signal.aborted) fail('stale');
                    const target = this.#options.target(context.generation.runtime), record = endpointRecord(target);
                    assertNativeGenerationResourceTarget(record.participant, context.generation.runtimeId, record.physical.deref());
                    const owner = await target.open(this.#bytes, this.#pins, context.signal);
                    this.#staged = { target, owner };
                    let discarded = false;
                    const discard = () => { if (discarded) return; discarded = true; void this.dispose(); };
                    if (!current() || context.signal.aborted) { discard(); fail('stale'); }
                    return { owner: this, current: () => !discarded && current() && !context.signal.aborted,
                        bind: () => {
                            if (!context.active() || !current() || discarded || context.signal.aborted) fail('stale');
                            this.#registration.remove(); this.#endpoint = target; this.#owner = owner; this.#staged = undefined; this.#enroll();
                        }, bound: () => { this.#snapshot = undefined; this.#options.onBound?.(target); }, discard };
                } };
        }, () => { void this.dispose(); });
    }
    async dispose() {
        if (this.#closed) return;
        this.#closed = true; this.#registration?.remove();
        const owner = this.#owner, endpoint = this.#endpoint;
        const released = this.#snapshot?.released && this.#snapshot.owner === owner && this.#snapshot.endpoint === endpoint;
        this.#bytes.fill(0); this.#pins = ''; this.#snapshot = undefined;
        if (!released) await endpoint.release(owner).catch(() => {});
        const staged = this.#staged; this.#staged = undefined;
        if (staged && staged.owner !== owner) await staged.target.release(staged.owner).catch(() => {});
    }
}
