/** Internal source/owner transport only. Native CEMQH1 verification and selected
 * execution stay in Rust. These capabilities never come from authored data. */
import { assertNativeGenerationResourceTarget, assertNativeGenerationResourceBudget, registerNativeGenerationResource } from './native-generation.mjs';
const shared = globalThis[Symbol.for('cem.query-source.endpoints.v1')] ??= { endpoints: new WeakMap() };
const fail = code => { throw Error(`cem.query.resource.${code}`); };
const MAX_BYTES = 69643, MAX_PINS = 16384;
const MAX_RECEIVER = 1024 * 1024, MAX_WORKSPACE = 4 * MAX_RECEIVER;
const MAX_STYLESHEET_RECEIPT = 1024 * 1024;
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
function receiverCapture(value) {
    if (typeof value?.revision !== 'string' || !/^cem-bin\/1\+blake3:[0-9a-f]{64}$/.test(value.revision)) fail('receiver');
    return { bytes: source(value.bytes, '', MAX_RECEIVER), revision: value.revision };
}
function receiverRecovery(value) {
    if (typeof value?.runtimeId !== 'string' || !value.runtimeId || value.runtimeId.length > 1024
        || typeof value.pin !== 'string' || !value.pin) fail('receiver');
    return { bytes: source(value.bytes, value.pin, MAX_RECEIVER), journal: source(value.journal, '', MAX_WORKSPACE),
        pin: value.pin, runtimeId: value.runtimeId };
}
function stylesheetTag(tag) {
    if (typeof tag !== 'string' || !tag || tag.length > 256 || new TextEncoder().encode(tag).length > 256) fail('stylesheet');
    return tag;
}
function stylesheetReceipt(receipt) {
    if (typeof receipt !== 'string' || receipt.length > MAX_STYLESHEET_RECEIPT
        || new TextEncoder().encode(receipt).length > MAX_STYLESHEET_RECEIPT) fail('stylesheet');
    return receipt;
}
/** Native endpoints must first be provisioned by their trusted Rust embedding.
 * Keep the actual Memory weakly; an equal factory/foreign heap cannot substitute
 * it, and native tokens never become numeric runtime or worker wire handles. */
export function createQuerySourceWasmBackend(runtime) { return wasmBackend(runtime, 'querySource', MAX_BYTES); }
export function createApplicationSourceWasmBackend(runtime) { return wasmBackend(runtime, 'applicationSource', 4 * 1024 * 1024); }
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
        ...(prefix === 'applicationSource' && {
            stylesheets: (owner, tag) => call('applicationSourceStylesheetsV1', [owner, tag]),
        }),
    });
}
/** Original binding functions must be supplied by the shared physical owner.
 * No native handle is exposed by the returned endpoint or its owner tokens. */
export function createQuerySourceEndpoint(options) {
    const { participant, identity, current, native, timeoutMs = 5000 } = options;
    assertNativeGenerationResourceTarget(participant, undefined, identity);
    if (typeof current !== 'function' || !native || ['open', 'capture', 'release'].some(k => typeof native[k] !== 'function')
        || (native.stylesheets !== undefined && typeof native.stylesheets !== 'function')
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
    const open = async (bytes, pins, signal, recovery) => {
        if (owners.size >= 64) fail('limit');
        const copy = source(bytes, pins, maxSourceBytes);
        const value = await bounded(() => recovery ? native.receiver.open(copy, pins, recovery) : native.open(copy, pins),
            signal, value => native.release(value));
        if (!live()) { await native.release(value); fail('stale'); }
        if (owners.size >= 64) { await native.release(value); fail('limit'); }
        const owner = Object.freeze({ kind: 'cem-query-source-owner' });
        owners.set(owner, { value, releasing: false, styles: native.stylesheets && new AbortController() }); return owner;
    };
    const endpoint = Object.freeze({ participant,
        identity: () => physical.deref(), current: () => live() && pending === 0,
        open: (bytes, pins, signal) => open(bytes, pins, signal),
        async capture(owner, signal) {
            const entry = original(owner);
            const bytes = await bounded(() => native.capture(entry.value), signal);
            return source(bytes, '', maxSourceBytes);
        },
        async stylesheets(owner, tag) {
            const entry = original(owner);
            if (!entry.styles) fail('stylesheet');
            const signal = entry.styles.signal;
            const checkedTag = stylesheetTag(tag);
            const receipt = stylesheetReceipt(await bounded(() => native.stylesheets(entry.value, checkedTag)));
            const isCurrent = () => !signal.aborted && !entry.releasing && live();
            if (!isCurrent()) fail('stale');
            return { receipt, signal, isCurrent };
        },
        async release(owner, signal) {
            const entry = original(owner); entry.releasing = true;
            try {
                await bounded(() => native.release(entry.value), signal, undefined, true);
                owners.delete(owner);
            } finally { entry.styles?.abort(); }
        },
        dispose() {
            if (closed) return;
            closed = true;
            const entries = [...owners.values()];
            owners.clear();
            for (const entry of entries) {
                if (!entry.releasing) void Promise.resolve().then(() => native.release(entry.value)).catch(() => { failed = true; });
                entry.styles?.abort();
            }
            options.close?.();
        },
    });
    const receiver = native.receiver && {
        async capture(owner, signal) {
            const entry = original(owner);
            return receiverCapture(await bounded(() => native.receiver.capture(entry.value), signal));
        },
        async retire(owner, bytes, journal) {
            const entry = original(owner);
            const capsule = source(bytes, '', MAX_RECEIVER), sealedJournal = source(journal, '', MAX_WORKSPACE);
            entry.releasing = true;
            try {
                const pin = await bounded(() => native.receiver.retire(entry.value, capsule, sealedJournal));
                owners.delete(owner);
                entry.styles?.abort();
                if (typeof pin !== 'string' || !pin || pin.length > MAX_PINS
                    || new TextEncoder().encode(pin).length > MAX_PINS) fail('receiver');
                return pin;
            } finally {
                entry.releasing = false;
            }
        },
        open: (bytes, pins, recovery, signal) => open(bytes, pins, signal, receiverRecovery(recovery)),
    };
    shared.endpoints.set(endpoint, { participant, physical, maxSourceBytes, receiver, current: endpoint.current });
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
            stylesheets: options.applicationStylesheets === true ? (id, tag) => call('stylesheets', { id, tag }) : undefined,
            receiver: options.receiverRecovery === true ? {
                capture: id => call('capture-receiver', id),
                retire: (id, bytes, journal) => call('confirm-receiver', { id, bytes, journal }),
                open: async (bytes, pins, recovery) => {
                    const id = await call('open-receiver', { bytes, pins, recovery });
                    if (!Number.isSafeInteger(id) || id < 1) { failed = true; fail('transport'); }
                    return id;
                },
            } : undefined,
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
                if (op === 'open' || op === 'open-receiver') {
                    if (owners.size >= 64) fail('limit');
                    const bytes = source(value?.bytes, value?.pins, limit);
                    const owner = await (op === 'open'
                        ? native.open(bytes, value.pins)
                        : native.receiver?.open(bytes, value.pins, receiverRecovery(value.recovery)) ?? fail('receiver'));
                    if (closed) { await native.release(owner); return; }
                    owners.set(++sequence, owner); result = sequence;
                } else {
                    const key = op === 'confirm-receiver' || op === 'stylesheets' ? value?.id : value;
                    if (!owners.has(key)) fail('owner');
                    const owner = owners.get(key);
                    if (op === 'capture') result = source(await native.capture(owner), '', limit);
                    else if (op === 'stylesheets' && native.stylesheets) {
                        result = stylesheetReceipt(await native.stylesheets(owner, stylesheetTag(value.tag)));
                    } else if (op === 'capture-receiver') result = receiverCapture(await native.receiver?.capture(owner));
                    else if (op === 'confirm-receiver') {
                        result = await (native.receiver?.retire(owner,
                            source(value.bytes, '', MAX_RECEIVER), source(value.journal, '', MAX_WORKSPACE)) ?? fail('receiver'));
                        owners.delete(key);
                    }
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
        if (options.receiverCheckpoint !== undefined
            && (typeof options.receiverCheckpoint !== 'function' || !record.receiver)) {
            await options.endpoint.release(owner); fail('receiver');
        }
        resource.#options = { current: options.current, target: options.target, peers: options.peers,
            onBound: options.onBound, receiverCheckpoint: options.receiverCheckpoint };
        try { resource.#enroll(); } catch (error) { await options.endpoint.release(owner); throw error; }
        return resource;
    }
    async stylesheets(tag) {
        const endpoint = this.#endpoint, owner = this.#owner;
        const current = () => !this.#closed && this.#endpoint === endpoint && this.#owner === owner
            && this.#options.current() === true;
        if (!current()) fail('policy');
        const result = await endpoint.stylesheets(owner, tag);
        const isCurrent = () => current() && result.isCurrent();
        if (!isCurrent()) fail('stale');
        return { receipt: result.receipt, signal: result.signal, isCurrent };
    }
    async prepareRecovery(signal) {
        this.#clearSnapshot();
        if (this.#closed || this.#options.current() !== true) fail('policy');
        const endpoint = this.#endpoint, owner = this.#owner;
        const receiver = this.#options.receiverCheckpoint ? endpointRecord(endpoint).receiver : undefined;
        const captured = receiver ? await receiver.capture(owner, signal) : undefined;
        const bytes = captured?.bytes ?? await endpoint.capture(owner, signal);
        if (this.#closed || this.#endpoint !== endpoint || this.#options.current() !== true || !endpoint.current()
            || !captured && (bytes.length !== this.#bytes.length || bytes.some((byte, i) => byte !== this.#bytes[i]))) {
            captured?.bytes.fill(0);
            fail('stale');
        }
        this.#snapshot = { endpoint, owner, released: false, receiver: captured };
        return captured?.revision;
    }
    #clearSnapshot() {
        this.#snapshot?.receiver?.bytes.fill(0);
        this.#snapshot = undefined;
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
            const receiver = snapshot.receiver;
            // The original native owner issues the pin only at retirement.
            const receiverBytes = receiver ? receiver.bytes.length + MAX_PINS : 0;
            return { current, peers, retainedBytes: this.#bytes.length + new TextEncoder().encode(this.#pins).length + receiverBytes,
                release: async () => {
                    if (!current() || snapshot.released) fail('stale');
                    if (receiver) receiver.pin = await endpointRecord(snapshot.endpoint).receiver.retire(snapshot.owner,
                        receiver.bytes, this.#options.receiverCheckpoint());
                    else await snapshot.endpoint.release(snapshot.owner);
                    snapshot.released = true;
                    if (!current()) fail('stale');
                },
                prepare: async context => {
                    if (!current() || !snapshot.released || context.signal.aborted) fail('stale');
                    const target = this.#options.target(context.generation.runtime), record = endpointRecord(target);
                    assertNativeGenerationResourceTarget(record.participant, context.generation.runtimeId, record.physical.deref());
                    if (receiver && !record.receiver) fail('receiver');
                    const owner = await (receiver ? record.receiver.open(this.#bytes, this.#pins, {
                        bytes: receiver.bytes, pin: receiver.pin, journal: context.generation.checkpoint,
                        runtimeId: context.generation.previousRuntimeId,
                    }, context.signal) : target.open(this.#bytes, this.#pins, context.signal));
                    if (this.#closed) { await target.release(owner); fail('stale'); }
                    this.#staged = { target, owner };
                    let discarded = false;
                    const discard = () => { if (discarded) return; discarded = true; void this.dispose(); };
                    if (!current() || context.signal.aborted) { discard(); fail('stale'); }
                    return { owner: this, current: () => !discarded && current() && !context.signal.aborted,
                        bind: () => {
                            if (!context.active() || !current() || discarded || context.signal.aborted) fail('stale');
                            this.#registration.remove(); this.#endpoint = target; this.#owner = owner; this.#staged = undefined; this.#enroll();
                        }, bound: () => { this.#clearSnapshot(); this.#options.onBound?.(target); }, discard };
                } };
        }, () => { void this.dispose(); });
    }
    async dispose() {
        if (this.#closed) return;
        this.#closed = true; this.#registration?.remove();
        const owner = this.#owner, endpoint = this.#endpoint;
        const released = this.#snapshot?.released && this.#snapshot.owner === owner && this.#snapshot.endpoint === endpoint;
        this.#bytes.fill(0); this.#pins = ''; this.#clearSnapshot();
        if (!released) await endpoint.release(owner).catch(() => {});
        const staged = this.#staged; this.#staged = undefined;
        if (staged && staged.owner !== owner) await staged.target.release(staged.owner).catch(() => {});
    }
}
