import type { CemCheckpointOwnerReceipt, CemStoredCheckpoint } from './committed-checkpoint.js';
import { claimNativeGenerationAdmission, type CemNativeGenerationAdmission } from './internal/runtime-support/native-generation.js';

/** Prepared owners must not publish, subscribe or deliver effects while inactive.
 * Native imports and fresh authority acquisition belong to the shared adapter.
 * current/discard are original bound controls, independent of their receiver. */
export interface CemPreparedCheckpointOwner {
    readonly owner: object;
    readonly current: () => boolean;
    readonly discard: () => void;
}
export interface CemCheckpointResumeContext {
    readonly previousRuntimeId: string;
    readonly runtimeId: string;
    readonly runtime: object;
    readonly receipt: Readonly<CemCheckpointOwnerReceipt>;
    /** Canonical inert island bytes; never rendered HTML or a JS AST. */
    readonly island: string;
    readonly signal: AbortSignal;
    /** Every resumed owner shares this gate. No callback runs when it opens. */
    readonly active: () => boolean;
}
export interface CemCheckpointResumeAdapter {
    /** Capture a host-only witness BEFORE retirement; it must detect property
     * edits, disconnects and policy changes without reading the retired heap. */
    witness(): () => boolean;
    prepare(context: CemCheckpointResumeContext): Promise<CemPreparedCheckpointOwner>;
}
export interface CemCheckpointResumeTicket { readonly kind: 'cem-checkpoint-resume-ticket' }
export interface CemResumedCheckpointOwners {
    readonly owners: readonly Readonly<{ instanceId: string; runtimeId: string; owner: object }>[];
    /** Close all owner gates, discard staged state and revoke every fresh heap. */
    revoke(): void;
}
interface Ticket {
    runtimeId: string;
    checkpoint: Uint8Array;
    current: () => boolean;
    owners: { receipt: Readonly<CemCheckpointOwnerReceipt>; island: string; prepare: CemCheckpointResumeAdapter['prepare'] }[];
    used: boolean;
    cancel?: () => void;
}
const tickets = new WeakMap<CemCheckpointResumeTicket, Ticket>();
const fail = (code: string): never => { throw new Error(`cem.checkpoint.resume.${code}`); };

/** @internal Publication coordinator only: native-sealed current bytes and
 * original complete endpoint adapters, never a reconstruction from user data. */
export function issueCheckpointResumeTicket(runtimeId: string, saved: CemStoredCheckpoint,
    current: () => boolean, adapters: ReadonlyMap<string, CemCheckpointResumeAdapter>): CemCheckpointResumeTicket {
    if (!saved.bytes.byteLength || saved.bytes.byteLength > 1024 * 1024 || !current()) fail('stale');
    if (saved.receipt.operations.some(operation => operation.outcome !== 'acknowledged' && operation.outcome !== 'not-applied')) fail('unknown');
    const witnesses: (() => boolean)[] = [];
    // This is the named checkpoint control DTO. Island strings remain opaque.
    const body = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(saved.bytes)) as { owners: { instanceId: string; island: string }[] };
    if (adapters.size !== saved.receipt.owners.length || adapters.size < 1 || adapters.size > 256) fail('coverage');
    const owners = saved.receipt.owners.map(receipt => {
        const adapter = adapters.get(receipt.instanceId);
        const island = body.owners.find(owner => owner.instanceId === receipt.instanceId)?.island;
        if (!adapter || typeof island !== 'string' || !island) return fail('coverage');
        witnesses.push(adapter.witness());
        // Bind a snapshot of the adapter's method; later caller mutation is inert.
        return { receipt: Object.freeze({ ...receipt }), island, prepare: adapter.prepare.bind(adapter) };
    });
    const valid = () => current() && witnesses.every(witness => witness() === true) && current();
    if (!valid()) fail('stale');
    const ticket = Object.freeze({ kind: 'cem-checkpoint-resume-ticket' as const });
    tickets.set(ticket, { runtimeId, checkpoint: new Uint8Array(saved.bytes), current: valid, owners, used: false });
    return ticket;
}
/** @internal Erase superseded/private bytes and cancel an active preparation. */
export function revokeCheckpointResumeTicket(ticket: CemCheckpointResumeTicket): void {
    const record = tickets.get(ticket);
    if (!record) return;
    record.used = true; record.checkpoint.fill(0); record.checkpoint = new Uint8Array(0); record.owners.length = 0; record.current = () => false;
    const cancel = record.cancel; record.cancel = undefined; cancel?.();
}
function originalTickets(values: readonly CemCheckpointResumeTicket[]): Ticket[] {
    if (!Array.isArray(values)) return fail('coverage');
    const length = Object.getOwnPropertyDescriptor(values, 'length')?.value;
    if (!Number.isSafeInteger(length) || length < 1 || length > 16) return fail('coverage');
    const records: Ticket[] = [];
    for (let index = 0; index < length; index++) {
        const slot = Object.getOwnPropertyDescriptor(values, String(index));
        const record = slot && 'value' in slot && tickets.get(slot.value);
        if (!record || records.includes(record)) return fail('ticket');
        records.push(record);
    }
    if (records.some(record => record.used)) return fail('ticket');
    return records;
}
function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
    if (a.byteLength !== b.byteLength) return false;
    for (let index = 0; index < a.byteLength; index++) if (a[index] !== b[index]) return false;
    return true;
}

/** Explicit, complete, one-shot semantic-adapter transaction. It does not
 * retarget old aliases, infer adapters, replay effects or restart Studio. */
export async function resumeCemCheckpointOwners(admission: CemNativeGenerationAdmission,
    originals: readonly CemCheckpointResumeTicket[], timeoutMs = 5000): Promise<CemResumedCheckpointOwners> {
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) return fail('timeout-config');
    const records = originalTickets(originals);
    // Claim after snapshotting caller slots; proxies cannot reuse a consumed
    // ticket/admission by starting a nested transaction during inspection.
    if (records.some(record => record.used)) return fail('ticket');
    const fresh = claimNativeGenerationAdmission(admission);
    const controller = new AbortController();
    const staged: { current: () => boolean; discard: () => void }[] = [];
    let active = false, revoked = false, timer: ReturnType<typeof setTimeout> | undefined;
    const discarded = new WeakSet<CemPreparedCheckpointOwner>();
    const discard = (prepared: CemPreparedCheckpointOwner) => {
        if (!prepared || typeof prepared !== 'object' || discarded.has(prepared)) return;
        discarded.add(prepared);
        try { prepared.discard(); } catch { /* Whole heap revocation still follows. */ }
    };
    const dispose = () => {
        if (revoked) return;
        revoked = true; active = false; controller.abort();
        for (const prepared of staged.splice(0)) { try { prepared.discard(); } catch { /* Continue all owner cleanup. */ } }
    };
    // The original group closes native admission before invoking dispose, so
    // reentrant adapter cleanup cannot start ordinary work on another peer.
    const revoke = () => fresh.revoke();
    fresh.onRevoke(dispose);
    for (const record of records) { record.used = true; record.cancel = revoke; }
    let rejectAbort!: (error: Error) => void;
    const aborted = new Promise<never>((_, reject) => { rejectAbort = reject; });
    void aborted.catch(() => undefined);
    const onAbort = () => rejectAbort(new Error('cem.checkpoint.resume.revoked'));
    controller.signal.addEventListener('abort', onAbort, { once: true });
    if (controller.signal.aborted) onAbort();
    const deadline = performance.now() + timeoutMs;
    const current = () => !revoked && fresh.current() && records.every(record => record.current());
    try {
        if (records.length !== fresh.generations.length || !current()) fail('coverage');
        const pairs = fresh.generations.map(generation => {
            const record = records.find(record => record.runtimeId === generation.previousRuntimeId);
            if (!record || !sameBytes(record.checkpoint, generation.checkpoint)) return fail('checkpoint');
            return { record, generation };
        });
        if (new Set(pairs.map(pair => pair.record)).size !== records.length) fail('coverage');
        if (records.reduce((count, record) => count + record.owners.length, 0) > 256) fail('limit');
        const owners: { instanceId: string; runtimeId: string; owner: object }[] = [];
        for (const { record, generation } of pairs) for (const owner of record.owners) {
            if (!current()) fail('stale');
            const remaining = deadline - performance.now();
            if (remaining <= 0) fail('timeout');
            const pending = Promise.resolve().then(() => {
                if (!current()) return fail('stale');
                return owner.prepare(Object.freeze({
                    previousRuntimeId: generation.previousRuntimeId, runtimeId: generation.runtimeId, runtime: generation.runtime,
                    receipt: owner.receipt, island: owner.island, signal: controller.signal, active: () => active && !revoked && fresh.current(),
                }));
            });
            void pending.then(prepared => { if (revoked) discard(prepared); }, () => undefined);
            const prepared = await Promise.race([pending, aborted, new Promise<never>((_, reject) => {
                timer = setTimeout(() => reject(new Error('cem.checkpoint.resume.timeout')), remaining);
            })]);
            clearTimeout(timer); timer = undefined;
            if (revoked) { discard(prepared); fail('stale'); }
            // Only bound cleanup/current controls survive; no preparation object
            // or predecessor source/runtime ownership chain is retained.
            staged.push({ current: prepared.current, discard: prepared.discard });
            if (!prepared.owner || typeof prepared.owner !== 'object' || typeof prepared.current !== 'function' || typeof prepared.discard !== 'function') fail('adapter');
            owners.push(Object.freeze({ instanceId: owner.receipt.instanceId, runtimeId: generation.runtimeId, owner: prepared.owner }));
            if (!current()) fail('stale');
        }
        for (const prepared of staged) if (prepared.current() !== true) fail('stale');
        if (!current()) fail('stale');
        if (performance.now() >= deadline) fail('timeout');
        // All preparation/probing is finished. Opening this one gate invokes no
        // host callback and admits all prepared owners in the same JS turn.
        active = true;
        return Object.freeze({ owners: Object.freeze(owners), revoke });
    } catch (error) { revoke(); throw error; }
    finally {
        clearTimeout(timer); controller.signal.removeEventListener('abort', onAbort);
        // A caller-held ticket or new owner gate must not pin old DOM owners,
        // source adapters or private checkpoint copies after the attempt.
        for (const record of records) { record.checkpoint.fill(0); record.checkpoint = new Uint8Array(0); record.owners.length = 0; record.current = () => false; record.cancel = undefined; }
    }
}
