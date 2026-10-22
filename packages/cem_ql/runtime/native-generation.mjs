/** Shared host lifecycle control, never a serialized/native replacement permit.
 * Vendored copies share original capabilities within one realm. Authored data
 * cannot join this registry; its registration exports are internal host APIs. */
const key = Symbol.for('cem.native-generation.groups.v1');
const state = globalThis[key] ??= { participants: new WeakMap(), receipts: new WeakMap(), identities: new WeakSet(), candidates: new WeakSet() };
state.admissions ??= new WeakMap();
const fail = code => { throw new Error(`cem.runtime.allocation.${code}`); };
const object = value => typeof value === 'object' && value !== null;
function slots(values) {
    if (!Array.isArray(values)) fail('coverage');
    const length = Object.getOwnPropertyDescriptor(values, 'length')?.value;
    if (!Number.isSafeInteger(length) || length < 1 || length > 16) fail('coverage');
    const copy = [];
    for (let i = 0; i < length; i++) {
        const slot = Object.getOwnPropertyDescriptor(values, String(i));
        if (!slot || !('value' in slot) || !object(slot.value)) fail('coverage');
        copy.push(slot.value);
    }
    return copy;
}
/** Internal: one original participant per actual owned Memory/Worker. */
export function registerNativeGeneration(identity, runtimeId, allocate) {
    if (!object(identity) || state.identities.has(identity) || typeof runtimeId !== 'string' || !runtimeId || typeof allocate !== 'function') fail('identity');
    const participant = Object.freeze({ kind: 'cem-native-generation-participant' });
    state.identities.add(identity);
    state.participants.set(participant, { identity, runtimeId, allocate, group: undefined, receipt: undefined, consumed: false });
    return participant;
}
/** Internal: called only after original ownership/census/handoff retirement. */
export function completeNativeRetirement(participant, bytes) {
    const record = object(participant) && state.participants.get(participant);
    if (!record || record.receipt || !(bytes instanceof Uint8Array) || !(bytes.buffer instanceof ArrayBuffer) || bytes.byteLength < 1 || bytes.byteLength > 1024 * 1024) fail('receipt');
    const checkpoint = new Uint8Array(bytes);
    const receipt = Object.freeze({ checkpoint: new Uint8Array(checkpoint), replacementAllowed: false });
    record.receipt = receipt; record.identity = undefined;
    state.receipts.set(receipt, { record, checkpoint });
    return receipt;
}

/** Internal: consume original resume authority once, never trust exposed bytes.
 * These are host controls, not permission for authored data or old aliases. */
export function claimNativeGenerationAdmission(admission) {
    const record = object(admission) && state.admissions.get(admission);
    if (!record || record.claimed) fail('admission');
    record.claimed = true;
    if (!record.current()) { record.revoke(); fail('admission'); }
    return Object.freeze({ current: record.current, revoke: record.revoke, onRevoke: record.onRevoke,
        generations: Object.freeze(record.generations.map(entry => Object.freeze({ ...entry,
            checkpoint: new Uint8Array(entry.checkpoint) }))) });
}

/** An explicit complete set of original shared runtime participants. It never
 * discovers unknown heaps or upgrades legacy/raw aliases into managed owners. */
export class CemNativeGenerationGroup {
    #records;
    #state = 'idle';
    constructor(participants) {
        const entries = slots(participants), records = entries.map(value => state.participants.get(value));
        if (records.some(record => !record || record.group || record.consumed) || new Set(records).size !== records.length) fail('coverage');
        this.#records = records;
        for (const record of records) record.group = this;
    }
    async allocate(retirements, timeoutMs = 5000) {
        if (this.#state !== 'idle') fail('state');
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('timeout-config');
        // Snapshot caller data before locking or allocating. Proxy reentrancy
        // must not replace an in-flight original group transaction.
        const entries = slots(retirements), receipts = entries.map(value => state.receipts.get(value));
        if (this.#state !== 'idle') fail('state');
        if (entries.length !== this.#records.length) fail('coverage');
        if (receipts.some(receipt => !receipt || receipt.record.group !== this || receipt.record.consumed)
            || new Set(receipts.map(receipt => receipt.record)).size !== receipts.length) fail('receipt');
        if (receipts.reduce((size, receipt) => size + receipt.checkpoint.byteLength, 0) > 4 * 1024 * 1024) fail('limit');
        this.#state = 'allocating';
        const cancellation = new AbortController();
        const gate = { admitted: false, revoked: false }, candidates = [], identities = new Set(), revokeListeners = [];
        const admitted = () => gate.admitted && !gate.revoked;
        const discard = candidate => { try { candidate.discard(); } catch { /* Fence every peer even if native cleanup refuses. */ } };
        const revoke = () => {
            if (gate.revoked) return;
            gate.revoked = true; identities.clear();
            for (const listener of revokeListeners.splice(0)) { try { listener(); } catch { /* Continue every physical cleanup. */ } }
            cancellation.abort();
            for (const candidate of candidates) discard(candidate);
        };
        const deadline = performance.now() + timeoutMs;
        let timer;
        try {
            const generations = [];
            for (const record of this.#records) {
                const remaining = deadline - performance.now();
                if (remaining <= 0) fail('timeout');
                const pending = Promise.resolve().then(() => record.allocate(admitted, cancellation.signal));
                // A timed-out initializer owns its eventual candidate until it
                // can discard it; no late reply may open this gate.
                void pending.then(candidate => { if (gate.revoked) discard(candidate); }, () => undefined);
                const candidate = await Promise.race([pending, new Promise((_, reject) => {
                    timer = setTimeout(() => reject(new Error('cem.runtime.allocation.timeout')), remaining);
                })]);
                clearTimeout(timer); timer = undefined;
                // Retain only original callable controls, never the candidate
                // object carrying actual native Memory/Worker identity. A host
                // may keep the admission/runtime handles after revocation.
                candidates.push({ current: candidate.current, discard: candidate.discard });
                if (!object(candidate.identity) || state.identities.has(candidate.identity) || state.candidates.has(candidate.identity) || identities.has(candidate.identity)
                    || typeof candidate.runtimeId !== 'string' || !candidate.runtimeId || candidate.runtimeId === record.runtimeId) fail('identity');
                // Registration by the allocating runtime is allowed; another
                // candidate or the old physical heap is never allowed.
                identities.add(candidate.identity); state.candidates.add(candidate.identity);
                const receipt = state.receipts.get(record.receipt);
                generations.push(Object.freeze({ previousRuntimeId: record.runtimeId, runtimeId: candidate.runtimeId,
                    runtime: candidate.runtime, checkpoint: new Uint8Array(receipt.checkpoint) }));
            }
            // All callbacks execute with admission closed. The single gate
            // assignment below admits the entire set with no await/user hook.
            for (const candidate of candidates) if (candidate.current() !== true) fail('stale');
            if (performance.now() >= deadline) fail('timeout');
            for (const record of this.#records) { record.consumed = true; record.allocate = undefined; }
            identities.clear();
            this.#state = 'admitted';
            gate.admitted = true;
            const admission = Object.freeze({ generations: Object.freeze(generations), revoke });
            state.admissions.set(admission, { claimed: false, revoke,
                onRevoke: listener => {
                    if (typeof listener !== 'function' || revokeListeners.length) fail('admission');
                    if (!admitted()) listener(); else revokeListeners.push(listener);
                },
                current: () => {
                    if (!admitted()) return false;
                    try { return candidates.every(candidate => candidate.current() === true) && admitted(); } catch { return false; }
                },
                generations: generations.map(entry => ({ ...entry, checkpoint: new Uint8Array(entry.checkpoint) })) });
            return admission;
        } catch (error) { this.#state = 'failed'; revoke(); throw error; }
        finally { clearTimeout(timer); }
    }
}
