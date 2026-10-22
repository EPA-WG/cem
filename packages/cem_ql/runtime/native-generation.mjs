/** Shared host lifecycle control, never a serialized/native replacement permit.
 * Vendored copies share original capabilities within one realm. Authored data
 * cannot join this registry; its registration exports are internal host APIs. */
const key = Symbol.for('cem.native-generation.groups.v1');
const state = globalThis[key] ??= { participants: new WeakMap(), receipts: new WeakMap(), identities: new WeakSet(), candidates: new WeakSet() };
state.admissions ??= new WeakMap();
state.groupGuards ??= new WeakMap();
state.authorities ??= new WeakSet();
state.authorityRecords ??= new WeakMap();
state.bindingTickets ??= new WeakMap();
state.bindingClaims ??= new WeakSet();
state.coverages ??= new WeakMap();
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
    // Uniqueness is weakly held above. The physical owner releases memory;
    // a retained participant must not keep a disposed WASM memory alive.
    state.participants.set(participant, { runtimeId, allocate, group: undefined, receipt: undefined, consumed: false });
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
    const authority = Object.freeze({ current: record.current, revoke: record.revoke, onRevoke: record.onRevoke,
        generations: Object.freeze(record.generations.map(entry => Object.freeze({ ...entry,
            checkpoint: new Uint8Array(entry.checkpoint) }))) });
    state.authorities.add(authority); state.authorityRecords.set(authority, record.records); return authority;
}

/** Internal host adapters only. Original tickets bind stable host routes; they
 * never revive a retired native alias or authorize an external action replay. */
export function issueNativeGenerationBindingTicket(participant, current, prepare) {
    const physical = state.participants.get(participant), runtimeId = physical?.runtimeId;
    if (!physical || typeof runtimeId !== 'string' || !runtimeId || typeof current !== 'function' || typeof prepare !== 'function' || current() !== true) fail('binding');
    const ticket = Object.freeze({ kind: 'cem-native-generation-binding-ticket' });
    state.bindingTickets.set(ticket, { physical, runtimeId, current, prepare, used: false });
    return ticket;
}
/** Claim all remaining generations exactly once, alongside original rendering
 * tickets. The returned callbacks are snapshots from trusted shared adapters. */
export function claimNativeGenerationBindings(authority, originals, renderingIds) {
    if (!state.authorities.has(authority) || state.bindingClaims.has(authority) || !Array.isArray(originals) || originals.length > 16) fail('binding');
    const entries = originals.length ? slots(originals) : [];
    const records = entries.map(ticket => state.bindingTickets.get(ticket));
    if (records.some(record => !record || record.used) || new Set(records).size !== records.length) fail('binding');
    const physical = state.authorityRecords.get(authority);
    if (!physical || records.some(record => !physical.includes(record.physical))) fail('binding');
    const ids = [...renderingIds, ...records.map(record => record.runtimeId)];
    if (ids.length !== authority.generations.length || new Set(ids).size !== ids.length
        || authority.generations.some(generation => !ids.includes(generation.previousRuntimeId))) fail('coverage');
    if (!authority.current() || records.some(record => record.current() !== true)) fail('binding-stale');
    state.bindingClaims.add(authority);
    const prepared = records.map(record => ({ generation: authority.generations[physical.indexOf(record.physical)],
        current: record.current, prepare: record.prepare }));
    for (const record of records) { record.used = true; record.prepare = undefined; record.current = () => false; }
    return prepared;
}
/** Original attestation from a shared adapter that checked full physical owner
 * coverage. Duplicate logical members may share only this exact attestation. */
export function registerNativeGenerationCoverage(participant, current) {
    if (!state.participants.has(participant) || typeof current !== 'function' || current() !== true) fail('coverage');
    const coverage = Object.freeze({ kind: 'cem-native-generation-coverage' });
    state.coverages.set(coverage, { participant, current }); return coverage;
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
        const guard = state.groupGuards.get(this) ?? (() => true);
        if (guard() !== true) fail('enrollment-stale');
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
                if (guard() !== true) fail('enrollment-stale');
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
            if (guard() !== true) fail('enrollment-stale');
            if (performance.now() >= deadline) fail('timeout');
            for (const record of this.#records) { record.consumed = true; record.allocate = undefined; }
            identities.clear();
            this.#state = 'admitted';
            gate.admitted = true;
            const admission = Object.freeze({ generations: Object.freeze(generations), revoke });
            state.admissions.set(admission, { claimed: false, revoke, records: [...this.#records],
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
        finally { clearTimeout(timer); state.groupGuards.delete(this); }
    }
}

/** Host-owned membership, including unsupported heaps. Labels are diagnostics,
 * never retirement authority. Authored attributes cannot enroll a runtime.
 * Preparation is read-only: native closure, sealed checkpoints and original
 * ownership acknowledgments are still required by each retirement adapter. */
export class CemNativeRecoveryScope {
    #members = new Map();
    #revision = 0;
    #preparing = false;
    #disposed = false;
    enroll(endpoint) {
        const { id, kind, prepare } = endpoint;
        if (this.#disposed || typeof id !== 'string' || !id || id.length > 256 || this.#members.has(id)
            || !['rendering', 'live-preview', 'command-service', 'command-invocation'].includes(kind)
            || typeof prepare !== 'function' || this.#members.size >= 256) fail('enrollment');
        const member = { id, kind, prepare };
        this.#members.set(id, member); this.#revision++;
        return Object.freeze({ remove: () => {
            if (this.#members.get(id) === member) { this.#members.delete(id); this.#revision++; }
        }, changed: () => { if (this.#members.get(id) === member) this.#revision++; } });
    }
    /** No callbacks, native work, source bytes or replacement permits. */
    inspect() {
        return Object.freeze({ disposed: this.#disposed, preparing: this.#preparing,
            members: Object.freeze([...this.#members.values()].map(({ id, kind }) => Object.freeze({ id, kind }))) });
    }
    async prepare(timeoutMs = 5000) {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('timeout-config');
        if (this.#disposed || this.#preparing || !this.#members.size) fail('enrollment');
        this.#preparing = true;
        const revision = this.#revision, members = [...this.#members.values()], witnesses = [], participants = [], coverages = new Map();
        const cancellation = new AbortController();
        const current = () => !this.#disposed && revision === this.#revision
            && witnesses.every(witness => witness() === true) && !this.#disposed && revision === this.#revision;
        let timer;
        try {
            await Promise.race([(async () => {
                for (const member of members) {
                    if (!current()) fail('enrollment-stale');
                    const prepared = await member.prepare(cancellation.signal);
                    if (cancellation.signal.aborted) fail('enrollment-stale');
                    if (!prepared || typeof prepared.current !== 'function') fail('enrollment');
                    const entries = slots(prepared.participants);
                    if (entries.some(entry => !state.participants.has(entry))) fail('coverage');
                    witnesses.push(prepared.current);
                    for (const entry of entries) {
                        const coverage = state.coverages.get(prepared.coverage);
                        if (coverage && (coverage.participant !== entry || coverage.current() !== true)) fail('coverage');
                        if (participants.includes(entry)) {
                            if (!coverage || coverages.get(entry) !== prepared.coverage) fail('enrollment-overlap');
                        } else { participants.push(entry); coverages.set(entry, prepared.coverage); }
                        if (coverage) witnesses.push(coverage.current);
                    }
                    if (participants.length > 16) fail('limit');
                    if (!current()) fail('enrollment-stale');
                }
            })(), new Promise((_, reject) => { timer = setTimeout(() => reject(Error('cem.runtime.allocation.timeout')), timeoutMs); })]);
            if (!current()) fail('enrollment-stale');
            let used = false;
            return Object.freeze({ current, createGenerationGroup: () => {
                if (used || !current()) fail('enrollment-stale');
                used = true;
                const group = new CemNativeGenerationGroup(participants);
                // The guard is private and cannot be removed by copying the
                // preparation result. It is checked through allocation's gate.
                state.groupGuards.set(group, current);
                return group;
            } });
        } finally { clearTimeout(timer); cancellation.abort(); this.#preparing = false; }
    }
    dispose() { this.#disposed = true; this.#revision++; this.#members.clear(); }
}
