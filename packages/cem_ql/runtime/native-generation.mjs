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
state.physicalParticipants ??= new WeakMap();
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
    const record = { runtimeId, allocate, group: undefined, receipt: undefined, consumed: false, resources: new Set(), resourceRevision: 0 };
    state.participants.set(participant, record); state.physicalParticipants.set(identity, record);
    return participant;
}
/** Internal shared resource adapters, bound to an original physical participant.
 * Registration/capture grants no checkpoint, credential or retirement authority. */
export function registerNativeGenerationResource(participant, capture, dispose) {
    const record = state.participants.get(participant);
    if (!record || record.group || record.consumed || typeof capture !== 'function' || typeof dispose !== 'function' || (record.resources?.size ?? 0) >= 16) fail('resource');
    record.resources ??= new Set(); record.resourceRevision ??= 0;
    const resource = { capture, dispose };
    record.resources.add(resource); record.resourceRevision++;
    return Object.freeze({ remove: () => { if (record.resources.delete(resource)) record.resourceRevision++; } });
}
export function hasNativeGenerationResources(participant) {
    return (state.participants.get(participant)?.resources?.size ?? 0) !== 0;
}
/** Explicit host shutdown only; never produces a retirement receipt. */
export function discardNativeGenerationResources(participant) {
    const record = state.participants.get(participant);
    if (!record) fail('resource');
    for (const resource of [...(record.resources ?? [])]) resource.dispose();
    if (record.resources?.size) fail('resource-cleanup');
}
/** Internal original-capability check; a copied runtime label grants nothing. */
export function assertNativeGenerationResourceTarget(participant, runtimeId, identity) {
    const record = state.participants.get(participant);
    if (!record || record.consumed || runtimeId !== undefined && record.runtimeId !== runtimeId
        || !object(identity) || state.physicalParticipants.get(identity) !== record) fail('resource-target');
}
function resourcesCurrent(record) {
    const snapshot = record.resourceSnapshot;
    return !record.resources?.size && !snapshot || !!snapshot && snapshot.revision === record.resourceRevision
        && snapshot.entries.every(entry => entry.current() === true) && snapshot.revision === record.resourceRevision;
}
/** Called only inside the existing original checkpoint/embedding cleanup gate.
 * Synchronous cleanup never opens admission; any failure leaves retirement blocked. */
export function releaseNativeGenerationResources(participant, current) {
    const record = state.participants.get(participant);
    if (!record) fail('resource');
    if (!record.resources?.size && !record.resourceSnapshot) return;
    if (!record.group || record.receipt || typeof current !== 'function' || current() !== true || !resourcesCurrent(record)) fail('resource');
    for (const entry of record.resourceSnapshot.entries) {
        if (current() !== true || !resourcesCurrent(record)) fail('resource-stale');
        if (!entry.released) { if (entry.release() !== undefined) fail('resource'); entry.released = true; }
        if (current() !== true || !resourcesCurrent(record)) fail('resource-stale');
    }
}
/** Internal: called only after original ownership/census/handoff retirement. */
export function completeNativeRetirement(participant, bytes) {
    const record = object(participant) && state.participants.get(participant);
    if (!record || record.receipt || !(bytes instanceof Uint8Array) || !(bytes.buffer instanceof ArrayBuffer) || bytes.byteLength < 1 || bytes.byteLength > 4 * 1024 * 1024) fail('receipt');
    if (!resourcesCurrent(record) || record.resources?.size && !record.resourceSnapshot?.entries.every(entry => entry.released)) fail('resource');
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
    const resources = physical.flatMap((record, index) => {
        if (!resourcesCurrent(record)) fail('resource-stale');
        return (record.resourceSnapshot?.entries ?? []).map(entry => {
            if (!entry.released) fail('resource');
            return { generation: authority.generations[index], current: entry.current, prepare: entry.prepare };
        });
    });
    const prepared = [...resources, ...records.map(record => ({ generation: authority.generations[physical.indexOf(record.physical)],
        current: record.current, prepare: record.prepare }))];
    for (const record of physical) { record.resourceSnapshot = undefined; record.resources?.clear(); }
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
        try {
            for (const record of records) if (record.resources?.size) {
                const revision = record.resourceRevision;
                const entries = [...record.resources].map(resource => {
                    const snapshot = resource.capture();
                    if (!snapshot || typeof snapshot.current !== 'function' || typeof snapshot.release !== 'function' || typeof snapshot.prepare !== 'function') fail('resource');
                    return { current: snapshot.current, release: snapshot.release, prepare: snapshot.prepare, released: false };
                });
                record.resourceSnapshot = { revision, entries };
                if (!resourcesCurrent(record)) fail('resource');
            }
        } catch (error) {
            // Capture has no native cleanup authority. A transient busy resource
            // must allow a new complete preparation after work settles.
            for (const record of records) { if (record.group === this) record.group = undefined; record.resourceSnapshot = undefined; }
            throw error;
        }
    }
    async allocate(retirements, timeoutMs = 5000) {
        if (this.#state !== 'idle') fail('state');
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('timeout-config');
        const guard = state.groupGuards.get(this) ?? (() => true);
        if (guard() !== true) fail('enrollment-stale');
        if (this.#records.some(record => !resourcesCurrent(record))) fail('resource-stale');
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
            if (this.#records.some(record => !resourcesCurrent(record))) fail('resource-stale');
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

/** Complete host-only binding transaction; mixed rendering owners use the shared
 * checkpoint-resume transaction instead. No effects run while preparations stage. */
export async function resumeNativeGenerationBindings(admission, tickets, timeoutMs = 5000) {
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) fail('timeout-config');
    const authority = claimNativeGenerationAdmission(admission), controller = new AbortController(), staged = [], discarded = new WeakSet();
    const discard = prepared => {
        if (!object(prepared) || discarded.has(prepared)) return;
        discarded.add(prepared);
        try { prepared.discard(); } catch { /* Fence every peer despite cleanup failure. */ }
    };
    let active = false, revoked = false, timer;
    const dispose = () => {
        active = false; revoked = true; controller.abort();
        for (const prepared of staged.splice(0).reverse()) discard(prepared);
    };
    authority.onRevoke(dispose);
    const deadline = performance.now() + timeoutMs;
    try {
        const bindings = claimNativeGenerationBindings(authority, tickets, []);
        const current = () => !revoked && authority.current() && bindings.every(binding => binding.current() === true);
        for (const binding of bindings) {
            if (!current()) fail('binding-stale');
            const pending = Promise.resolve().then(() => binding.prepare(Object.freeze({ generation: binding.generation,
                active: () => active && !revoked && authority.current(), signal: controller.signal, revoke: authority.revoke })));
            void pending.then(prepared => { if (revoked) discard(prepared) }, () => undefined);
            const remaining = deadline - performance.now();
            if (remaining <= 0) fail('timeout');
            const prepared = await Promise.race([pending, new Promise((_, reject) => { timer = setTimeout(() => reject(Error('cem.runtime.allocation.timeout')), remaining); })]);
            clearTimeout(timer); timer = undefined;
            if (!prepared || !object(prepared.owner) || typeof prepared.current !== 'function' || typeof prepared.discard !== 'function' || typeof prepared.bind !== 'function') { discard(prepared); fail('binding'); }
            staged.push({ owner: prepared.owner, current: prepared.current, discard: () => discard(prepared), bind: prepared.bind, bound: prepared.bound });
            if (!current()) fail('binding-stale');
        }
        if (!current() || staged.some(prepared => prepared.current() !== true) || performance.now() >= deadline) fail('binding-stale');
        active = true;
        for (const prepared of staged) {
            if (revoked || !authority.current()) fail('binding-stale');
            prepared.bind();
        }
        for (const prepared of staged) prepared.bound?.();
        return Object.freeze({ revoke: authority.revoke });
    } catch (error) { authority.revoke(); throw error; }
    finally { clearTimeout(timer); }
}
