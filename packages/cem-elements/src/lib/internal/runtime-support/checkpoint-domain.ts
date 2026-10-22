import { registerNativeGeneration, completeNativeRetirement, type CemNativeGenerationAllocator, type CemNativeGenerationParticipant } from './native-generation.js';
import type { CemCheckpointRetirementHandoff } from '../../committed-checkpoint.js';
import type { CemCheckpointNativeRuntime } from '../../committed-checkpoint.js';

/** Private lifecycle witnesses. Neither serialized IDs nor structural copies
 * can enroll a native client or substitute a physical generation. */
export interface CemCheckpointNativeClient { readonly clientId: string }
interface Client {
    token: CemCheckpointNativeClient;
    domain: CemNativeCheckpointDomain;
    pending: number;
    listeners: Set<() => void>;
    requiredOwners?: (sources: ReadonlySet<string>) => readonly string[] | undefined;
    settled?: () => boolean;
    retirementCleanup?: () => void;
}
const clients = new WeakMap<CemCheckpointNativeClient, Client>();
const LIMIT = 256;

// Keep physical ownership visible across duplicate bundles in this realm.
// Such copies cannot prove each other's private clients and must refuse coverage.
const PHYSICAL_OWNERS = Symbol.for('cem.checkpoint.physical-owners.v1');
const physicalEnvironment = globalThis as typeof globalThis & {
    [PHYSICAL_OWNERS]?: WeakMap<object, { revoke(): void }>;
};
const physicalOwners = physicalEnvironment[PHYSICAL_OWNERS] ??= new WeakMap();

export type CemNativePressureReason = 'soft-pressure' | 'hard-pressure' | 'telemetry-unavailable';
export class CemNativeAdmissionClosedError extends Error {
    constructor() { super('cem.runtime.memory.closed'); this.name = 'CemNativeAdmissionClosedError'; }
}
export interface CemNativeDrain { readonly runtimeId: string; readonly revision: number }
const drainProofs = new WeakMap<CemNativeDrain, CemNativeCheckpointDomain>();
const terminalRenderingGenerations = new WeakSet<CemNativeCheckpointDomain>();
interface DrainWaiter { check(): void; fail(reason: string): void }

export class CemNativeCheckpointDomain {
    private readonly runtimeId = crypto.randomUUID();
    private readonly members = new Set<Client>();
    private readonly resources = new Map<string, CemCheckpointNativeClient | undefined>();
    private revision = 0;
    private alive = true;
    private nativeIdentity?: object;
    private overflow = false;
    private pressure?: CemNativePressureReason;
    private timedOut = false;
    private readonly drains = new Set<DrainWaiter>();
    private pressureSampler?: () => { softPressure: boolean; hardPressure: boolean };
    private readonly pressureListeners = new Set<(reason: CemNativePressureReason) => void>();
    private deadline?: ReturnType<typeof setTimeout>;
    private allocationToken?: CemNativeGenerationParticipant;
    private cleanupOwners?: ReadonlySet<CemCheckpointNativeClient>;

    constructor(private readonly mode: 'worker' | 'main-thread') {}

    /** A transient helper enrolls its pending lease in the same mutation as
     * its client, and retires both together after its finally block. */
    client(cleanup = false, work = false): CemCheckpointNativeClient {
        this.assertAdmission(cleanup);
        const token = Object.freeze({ clientId: crypto.randomUUID() });
        const client: Client = { token, domain: this, pending: work ? 1 : 0, listeners: new Set() };
        clients.set(token, client);
        if (this.members.size >= LIMIT) this.overflow = true; else this.members.add(client);
        this.changed();
        return token;
    }

    move(token: CemCheckpointNativeClient): void {
        const client = clients.get(token);
        if (!client || client.domain === this) return;
        if (terminalRenderingGenerations.has(client.domain)) throw new Error('cem.runtime.generation.retired');
        this.assertAdmission();
        if (client.domain.alive && client.domain.pressure) throw new CemNativeAdmissionClosedError();
        client.domain.members.delete(client); client.domain.changed();
        client.domain = this;
        if (this.members.size >= LIMIT) this.overflow = true; else this.members.add(client);
        this.changed();
    }

    remove(token: CemCheckpointNativeClient): void {
        const client = clients.get(token);
        if (client?.domain !== this) return;
        this.members.delete(client); this.changed();
        clients.delete(token);
        for (const listener of client.listeners) { try { listener(); } catch { /* Finish retiring every client. */ } }
        client.listeners.clear();
    }

    /** Bind to the actual Worker or initialized WASM memory, never a host label.
     * A different native object cannot silently reuse this generation. */
    bindNativeRuntime(identity: object): void {
        const owner = physicalOwners.get(identity);
        if (owner && owner !== this) { owner.revoke(); this.revoke(); }
        else physicalOwners.set(identity, this);
        if (this.nativeIdentity && this.nativeIdentity !== identity) this.revoke();
        else this.nativeIdentity = identity;
    }
    ownsNativeRuntime(identity: object | undefined): boolean {
        return !!identity && this.nativeIdentity === identity && this.alive && !this.overflow;
    }
    emptyForNativeDisposal(): boolean {
        return this.alive && !this.overflow && !this.timedOut && !!this.nativeIdentity && !this.members.size && !this.resources.size;
    }
    /** The release callback is the private native factory's synchronous census /
     * freeze / reference-release control. No user callbacks or async gap. */
    releaseEmptyNative(proof: CemNativeDrain, release: () => void): void {
        if (!this.emptyForNativeDisposal() || !this.drainCurrent(proof)) throw new Error('cem.runtime.generation.peers');
        release();
        terminalRenderingGenerations.add(this);
        this.nativeIdentity = undefined; this.pressureSampler = undefined;
        this.revoke();
    }
    revoke(): void { this.alive = false; this.changed(); }
    /** Only a privately allocated candidate's cleanup uses this after fencing. */
    discardAllocatedIdentity(): void { this.nativeIdentity = undefined; this.pressureSampler = undefined; this.revoke(); }

    /** Trusted native telemetry only; authored data never supplies this sampler. */
    sampleWith(sampler: () => { softPressure: boolean; hardPressure: boolean }): void {
        this.pressureSampler = sampler; this.samplePressure();
    }
    samplePressure(): void {
        if (!this.pressureSampler || this.pressure) return;
        try {
            const sample = this.pressureSampler();
            if (typeof sample.softPressure !== 'boolean' || typeof sample.hardPressure !== 'boolean') throw new Error('invalid native telemetry');
            if (sample.hardPressure || sample.softPressure) this.closeAdmission(sample.hardPressure ? 'hard-pressure' : 'soft-pressure');
        } catch { this.closeAdmission('telemetry-unavailable'); }
    }
    closeAdmission(reason: CemNativePressureReason): void {
        if (this.pressure) return;
        this.pressure = reason;
        for (const listener of this.pressureListeners) { try { listener(reason); } catch { /* Closure must reach every client. */ } }
        this.pressureListeners.clear(); this.changed();
    }
    onPressure(listener: (reason: CemNativePressureReason) => void): () => void {
        if (this.pressure) listener(this.pressure); else this.pressureListeners.add(listener);
        return () => { this.pressureListeners.delete(listener); };
    }
    pressureReason(): CemNativePressureReason | undefined { return this.pressure; }
    assertAdmission(cleanup = false): void {
        if (!this.alive && terminalRenderingGenerations.has(this) && !cleanup) throw new Error('cem.runtime.generation.retired');
        this.samplePressure();
        if (!cleanup && this.pressure) throw new CemNativeAdmissionClosedError();
    }
    private settled(): boolean {
        for (const client of this.members) {
            try { if (client.pending || client.settled && !client.settled()) return false; }
            catch { return false; }
        }
        return true;
    }
    drain({ timeoutMs = 5000, signal }: { timeoutMs?: number; signal?: AbortSignal } = {}): Promise<CemNativeDrain> {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) return Promise.reject(new Error('cem.runtime.drain.timeout-config'));
        if (!this.pressure || !this.alive || this.overflow || this.timedOut || signal?.aborted || this.drains.size >= 16) return Promise.reject(new Error('cem.runtime.drain.unavailable'));
        return new Promise((resolve, reject) => {
            let queued = false, done = false;
            const finish = (reason?: string) => {
                if (done) return; done = true;
                clearTimeout(timer); signal?.removeEventListener('abort', abort); this.drains.delete(waiter);
                if (reason) reject(new Error(reason));
                else {
                    const proof = Object.freeze({ runtimeId: this.runtimeId, revision: this.revision });
                    drainProofs.set(proof, this); resolve(proof);
                }
            };
            const waiter: DrainWaiter = {
                check: () => {
                    if (!this.alive || this.overflow || this.timedOut) finish('cem.runtime.drain.unavailable');
                    else if (!queued && !done && this.settled()) {
                        queued = true;
                        // Cleanup commonly releases a work lease and retires its
                        // transient client in the same stack. Observe both.
                        queueMicrotask(() => {
                            queued = false;
                            if (!this.alive || this.overflow || this.timedOut) finish('cem.runtime.drain.unavailable');
                            else if (this.settled()) finish();
                        });
                    }
                },
                fail: finish,
            };
            const abort = () => finish('cem.runtime.drain.aborted');
            const timer = setTimeout(() => {
                this.timedOut = true;
                for (const pending of [...this.drains]) pending.fail('cem.runtime.drain.timeout');
                this.changed();
            }, timeoutMs);
            this.drains.add(waiter); signal?.addEventListener('abort', abort, { once: true }); waiter.check();
        });
    }
    drainCurrent(proof: CemNativeDrain): boolean {
        return drainProofs.get(proof) === this && proof.revision === this.revision && !!this.pressure
            && this.alive && !this.overflow && !this.timedOut && this.settled();
    }

    /** Retained support work begins unowned. Only its processing engine may
     * adopt it after successful preparation; direct callers remain a blocker. */
    retain(kind: string, id: number): void {
        if (this.resources.size >= 4096) this.overflow = true;
        else this.resources.set(`${kind}:${id}`, undefined);
        this.changed();
    }
    release(kind: string, id: number): void {
        const key = `${kind}:${id}`, owner = this.resources.get(key);
        if (this.resources.delete(key) && !(owner && this.cleanupOwners?.has(owner))) this.changed();
    }
    adopt(kind: string, id: number, token: CemCheckpointNativeClient): void {
        const key = `${kind}:${id}`;
        if (clients.get(token)?.domain === this && this.resources.has(key) && this.resources.get(key) === undefined) {
            this.resources.set(key, token); this.changed();
        }
    }

    transfer(kind: string, id: number, from: CemCheckpointNativeClient, to: CemCheckpointNativeClient): void {
        const key = `${kind}:${id}`;
        if (this.resources.get(key) === from && clients.get(to)?.domain === this) { this.resources.set(key, to); this.changed(); }
    }

    snapshot(bindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>, sources: ReadonlyMap<CemCheckpointNativeClient, ReadonlySet<string>>): CemCheckpointNativeRuntime | undefined {
        if (!this.alive || this.overflow || this.timedOut || !this.members.size || !this.settled()) return undefined;
        for (const token of this.resources.values()) if (!token || !bindings.has(token)) return undefined;
        const records = [];
        for (const client of this.members) {
            const owners = bindings.get(client.token);
            if (client.pending || !owners?.length) return undefined;
            try {
                const required = client.requiredOwners?.(sources.get(client.token) ?? new Set());
                if (client.requiredOwners && !required || required?.some(id => !owners.includes(id))) return undefined;
            }
            catch { return undefined; }
            records.push({ clientId: client.token.clientId, owners: [...owners].sort() });
        }
        return { runtimeId: this.runtimeId, generation: 1, mode: this.mode, revision: this.revision,
            clients: records.sort((a, b) => a.clientId.localeCompare(b.clientId)) };
    }

    retirementEndpoint(endpoint: NativeRenderingRetirementEndpoint): void {
        if (this.mode !== 'worker' || retirementEndpoints.has(this)) throw new Error('cem.runtime.retirement.unsupported');
        retirementEndpoints.set(this, endpoint);
    }

    ownedRetirementEndpoint(endpoint: NativeFallbackRetirementEndpoint): void {
        if (this.mode !== 'main-thread' || !this.nativeIdentity || !this.alive || fallbackRetirementEndpoints.has(this))
            throw new Error('cem.runtime.retirement.unsupported');
        fallbackRetirementEndpoints.set(this, endpoint);
    }
    /** Explicit membership in one complete fresh-generation transaction. */
    retainedAllocationParticipant(): CemNativeGenerationParticipant | undefined { return this.allocationToken; }
    generationParticipant(): CemNativeGenerationParticipant {
        if (this.allocationToken) return this.allocationToken;
        const allocate = (fallbackRetirementEndpoints.get(this) ?? retirementEndpoints.get(this))?.allocate;
        if (!this.nativeIdentity || !allocate || !this.alive) throw new Error('cem.runtime.allocation.unsupported');
        return this.allocationToken = registerNativeGeneration(this.nativeIdentity, this.runtimeId, allocate);
    }
    hasRetirementCleanups(bindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>): boolean {
        return !!this.members.size && [...this.members].every(client => bindings.has(client.token) && !!client.retirementCleanup);
    }
    /** Quiet only the exact covered resource deletions. Membership, input,
     * ownership, work and arbitrary changes retain ordinary invalidation. */
    retireOwnedResources(bindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>, current: () => boolean, ledgerCurrent: () => boolean, commit: (current: () => boolean) => void): void {
        if (this.cleanupOwners || !this.hasRetirementCleanups(bindings) || !current()) throw new Error('cem.runtime.retirement.cleanup');
        this.cleanupOwners = new Set(bindings.keys());
        try {
            for (const client of this.members) {
                if (!ledgerCurrent()) throw new Error('cem.runtime.retirement.stale');
                const cleanup = client.retirementCleanup;
                if (!cleanup || cleanup() !== undefined) throw new Error('cem.runtime.retirement.cleanup');
            }
            if (this.resources.size || !ledgerCurrent()) throw new Error('cem.runtime.retirement.stale');
            // Host freshness callbacks run after cleanup only under the native
            // freeze: even raw cleanup aliases cannot race that last check.
            commit(() => !this.resources.size && ledgerCurrent() && current() && ledgerCurrent());
            this.nativeIdentity = undefined; this.pressureSampler = undefined;
            this.revoke();
        } finally { this.cleanupOwners = undefined; }
    }

    issuedIdentity(): string { return this.runtimeId; }
    identity(): string | undefined { return this.alive && !this.overflow ? this.runtimeId : undefined; }

    changed(): void {
        if (!Number.isSafeInteger(++this.revision)) this.overflow = true;
        if (this.pressure && this.alive && !this.timedOut && !this.settled()) {
            this.deadline ??= setTimeout(() => {
                this.deadline = undefined; this.timedOut = true;
                for (const pending of [...this.drains]) pending.fail('cem.runtime.drain.timeout');
                this.changed();
            }, 5000);
        } else { clearTimeout(this.deadline); this.deadline = undefined; }
        for (const client of this.members) for (const listener of client.listeners) {
            try { listener(); } catch { /* Observers cannot change native work or suppress peer invalidation. */ }
        }
        for (const waiter of [...this.drains]) waiter.check();
    }
}

/** One domain for this module graph's actual shared fallback WASM instance.
 * Worker transports create their own domains at construction. */
export const cemMainThreadCheckpointDomain = new CemNativeCheckpointDomain('main-thread');

export function nativeCheckpointIdentity(token: CemCheckpointNativeClient): string | undefined {
    return clients.get(token)?.domain.identity();
}
/** Lifetime identity independent of recovery coverage/observer failures. */
export function nativeCheckpointGeneration(token: CemCheckpointNativeClient): string | undefined {
    return clients.get(token)?.domain.issuedIdentity();
}
export function nativeCheckpointFailureFence(token: CemCheckpointNativeClient): () => void {
    const domain = clients.get(token)?.domain;
    return () => domain?.revoke();
}
export function nativeCheckpointSubscribe(token: CemCheckpointNativeClient, listener: () => void): () => void {
    const client = clients.get(token);
    if (!client) throw new Error('cem.checkpoint.native-client');
    client.listeners.add(listener); return () => { client.listeners.delete(listener); };
}
export function nativeCheckpointRequirements(token: CemCheckpointNativeClient, requiredOwners: (sources: ReadonlySet<string>) => readonly string[] | undefined): void {
    const client = clients.get(token);
    if (!client || client.requiredOwners) throw new Error('cem.checkpoint.native-client');
    client.requiredOwners = requiredOwners;
}
export function nativeCheckpointRetirementCleanup(token: CemCheckpointNativeClient, cleanup: () => void): void {
    const client = clients.get(token);
    if (!client || client.retirementCleanup) throw new Error('cem.checkpoint.native-client');
    client.retirementCleanup = cleanup;
}
export function nativeCheckpointDrainRequirements(token: CemCheckpointNativeClient, settled: () => boolean): void {
    const client = clients.get(token);
    if (!client || client.settled) throw new Error('cem.checkpoint.native-client');
    client.settled = settled;
}
export function nativeCheckpointAssertAdmission(token: CemCheckpointNativeClient, cleanup = false): void {
    const client = clients.get(token);
    if (!client) throw new Error('cem.checkpoint.native-client');
    client.domain.assertAdmission(cleanup);
}
export function nativeCheckpointDrain(token: CemCheckpointNativeClient, options?: { timeoutMs?: number; signal?: AbortSignal }): Promise<CemNativeDrain> {
    return clients.get(token)?.domain.drain(options) ?? Promise.reject(new Error('cem.runtime.drain.unavailable'));
}
export function nativeCheckpointDrainCurrent(proof: CemNativeDrain): boolean { return drainProofs.get(proof)?.drainCurrent(proof) ?? false; }
/** Independently recheck private drain witnesses and complete client/resource
 * coverage. This is evidence for a future replacement gate, never authority to
 * terminate a runtime or to roll an owner back to old checkpoint bytes. */
export function nativeCheckpointClosedCoverage(
    bindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>,
    sourceWitnesses: readonly CemCheckpointNativeSourceProof[],
    proofs: readonly CemNativeDrain[],
): CemCheckpointNativeRuntime[] | undefined {
    const domains = new Set([...bindings.keys()].map(token => clients.get(token)?.domain));
    if (!domains.size || domains.has(undefined) || proofs.length !== domains.size) return undefined;
    const proven = new Set<CemNativeCheckpointDomain>();
    for (const proof of proofs) {
        const domain = drainProofs.get(proof);
        if (!domain || proven.has(domain) || !domains.has(domain) || !domain.drainCurrent(proof)) return undefined;
        proven.add(domain);
    }
    const sources = new Map<CemCheckpointNativeClient, Set<string>>();
    for (const witness of sourceWitnesses) {
        try {
            if (!nativeCheckpointValidSourceProof(witness) || !witness.current() || !bindings.has(witness.client)) return undefined;
        } catch { return undefined; }
        const resources = sources.get(witness.client) ?? new Set<string>();
        resources.add(witness.resource); sources.set(witness.client, resources);
    }
    const snapshot = nativeCheckpointSnapshot(bindings, sources);
    return proofs.every(nativeCheckpointDrainCurrent) ? snapshot : undefined;
}
export function nativeCheckpointRefuse(token: CemCheckpointNativeClient): void { clients.get(token)?.domain.revoke(); }
export function nativeCheckpointChanged(token: CemCheckpointNativeClient): void { clients.get(token)?.domain.changed(); }
export function nativeCheckpointAdmission(token: CemCheckpointNativeClient, cleanup = false): () => void {
    const client = clients.get(token);
    if (!client) throw new Error('cem.checkpoint.native-client');
    nativeCheckpointAssertAdmission(token, cleanup);
    client.pending++;
    if (client.pending > LIMIT) client.domain.revoke(); else client.domain.changed();
    let released = false;
    return () => {
        if (released) return;
        released = true; client.pending--; client.domain.changed();
    };
}
export function nativeCheckpointSnapshot(bindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>, sources: ReadonlyMap<CemCheckpointNativeClient, ReadonlySet<string>> = new Map()): CemCheckpointNativeRuntime[] | undefined {
    const domains = new Set<CemNativeCheckpointDomain>();
    for (const token of bindings.keys()) {
        const client = clients.get(token);
        if (!client) return undefined;
        domains.add(client.domain);
    }
    const snapshots = [];
    for (const domain of domains) {
        const snapshot = domain.snapshot(bindings, sources);
        if (!snapshot) return undefined;
        snapshots.push(snapshot);
    }
    return snapshots.sort((a, b) => a.runtimeId.localeCompare(b.runtimeId));
}

export interface CemCheckpointNativeSourceProof { readonly client: CemCheckpointNativeClient; readonly resource: string; current(): boolean }
const sourceProofs = new WeakMap<object, CemCheckpointNativeSourceProof>();
const issuedSourceProofs = new WeakSet<object>();
export function nativeCheckpointValidSourceProof(proof: CemCheckpointNativeSourceProof): boolean { return issuedSourceProofs.has(proof); }
export function nativeCheckpointWitnessSource(source: object, client: CemCheckpointNativeClient, resource: string, current: () => boolean): void {
    if (clients.has(client)) {
        const proof = Object.freeze({ client, resource, current }); issuedSourceProofs.add(proof); sourceProofs.set(source, proof);
    }
}
export function nativeCheckpointSourceProof(source: object | undefined): CemCheckpointNativeSourceProof | undefined {
    return source && sourceProofs.get(source);
}

const workers = new WeakMap<object, CemNativeCheckpointDomain>();
export function cemWorkerCheckpointDomain(worker: object): CemNativeCheckpointDomain {
    let domain = workers.get(worker);
    if (!domain) {
        domain = new CemNativeCheckpointDomain('worker'); domain.bindNativeRuntime(worker); workers.set(worker, domain);
    }
    return domain;
}

/** Standalone support calls are native clients too. They may finish normally,
 * but cannot be omitted from an in-flight generation coverage receipt. */
export function nativeCheckpointWork<Args extends unknown[], Result>(operation: (...args: Args) => Promise<Result>, cleanup = false): (...args: Args) => Promise<Result> {
    return async (...args) => {
        const client = cemMainThreadCheckpointDomain.client(cleanup, true);
        try { nativeCheckpointAssertAdmission(client, cleanup); return await operation(...args); }
        finally { cemMainThreadCheckpointDomain.samplePressure(); cemMainThreadCheckpointDomain.remove(client); }
    };
}


interface NativeRenderingRetirementEndpoint {
    allocate?: CemNativeGenerationAllocator;
    close(): Promise<void>;
    /** Terminal cleanup of owned roots precedes the existing native zero-root
     * permit. Failure fences the generation; it never rolls native state back. */
    prepare(): Promise<unknown>;
    abort(): Promise<void>;
    terminate(): void;
}
interface NativeFallbackRetirementEndpoint {
    allocate?: CemNativeGenerationAllocator;
    close(): Promise<void>;
    authorize(consent: object | undefined): () => boolean;
    commit(current: () => boolean): void;
}
const fallbackRetirementEndpoints = new WeakMap<CemNativeCheckpointDomain, NativeFallbackRetirementEndpoint>();
const fallbackClosedConsents = new WeakMap<CemNativeDrain, { consent: object | undefined; current: () => boolean }>();
const retirementEndpoints = new WeakMap<CemNativeCheckpointDomain, NativeRenderingRetirementEndpoint>();
const retirementFlights = new WeakSet<CemNativeCheckpointDomain>();
export interface CemRetiredRenderingGeneration { readonly checkpoint: Uint8Array; readonly replacementAllowed: false }

async function retirementDeadline<T>(operation: () => Promise<T>, timeoutMs: number): Promise<T> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    try { return await Promise.race([operation(), new Promise<never>((_resolve, reject) => {
        timer = setTimeout(() => reject(new Error('cem.runtime.retirement.timeout')), timeoutMs);
    })]); } finally { clearTimeout(timer); }
}
function retirementTimeout(timeoutMs: number): void {
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 5000) throw new Error('cem.runtime.retirement.timeout-config');
}
/** Explicit trusted host lifecycle control. Legacy fallback has no exclusive
 * disposal boundary; owned fallback requires original embedding consent. */
export async function closeNativeRenderingGeneration(client: CemCheckpointNativeClient, timeoutMs = 5000, consent?: object): Promise<CemNativeDrain> {
    retirementTimeout(timeoutMs);
    const domain = clients.get(client)?.domain, fallback = domain && fallbackRetirementEndpoints.get(domain);
    const endpoint = fallback ?? (domain && retirementEndpoints.get(domain));
    if (!domain || !endpoint) throw new Error('cem.runtime.retirement.unsupported');
    if (retirementFlights.has(domain)) throw new Error('cem.runtime.retirement.pending');
    const consentCurrent = fallback?.authorize(consent);
    retirementFlights.add(domain);
    terminalRenderingGenerations.add(domain);
    try {
        domain.closeAdmission('soft-pressure');
        await domain.drain({ timeoutMs });
        if (consentCurrent && !consentCurrent()) throw new Error('cem.runtime.generation.ownership-stale');
        await retirementDeadline(() => endpoint.close(), timeoutMs);
        const proof = await domain.drain({ timeoutMs });
        if (consentCurrent) {
            if (!consentCurrent()) throw new Error('cem.runtime.generation.ownership-stale');
            fallbackClosedConsents.set(proof, { consent, current: consentCurrent });
        }
        return proof;
    } catch (error) { domain.revoke(); throw error; }
    finally { retirementFlights.delete(domain); }
}
/** One exact worker or owned fallback. Multiple heaps need a coordinated
 * transaction; scalar receipts never authorize native disposal. */
export async function retireNativeRenderingGeneration(
    ownerBindings: ReadonlyMap<CemCheckpointNativeClient, readonly string[]>,
    sourceWitnesses: readonly CemCheckpointNativeSourceProof[], proof: CemNativeDrain,
    handoff: CemCheckpointRetirementHandoff, timeoutMs = 5000, consent?: object,
): Promise<CemRetiredRenderingGeneration> {
    retirementTimeout(timeoutMs);
    const domain = drainProofs.get(proof), fallback = domain && fallbackRetirementEndpoints.get(domain);
    const endpoint = domain && retirementEndpoints.get(domain);
    if (!domain || !endpoint && !fallback) throw new Error('cem.runtime.retirement.coverage');
    const closedConsent = fallback && fallbackClosedConsents.get(proof);
    if (fallback && (!closedConsent || closedConsent.consent !== consent || !closedConsent.current())) throw new Error('cem.runtime.generation.ownership');
    if (retirementFlights.has(domain)) throw new Error('cem.runtime.retirement.pending');
    const bindings = new Map([...ownerBindings].map(([client, owners]) => [client, [...owners]]));
    const sources = [...sourceWitnesses];
    const current = () => (!closedConsent || closedConsent.current()) && nativeCheckpointClosedCoverage(bindings, sources, [proof])?.length === 1;
    if (!current()) throw new Error('cem.runtime.retirement.coverage');
    if (fallback && !domain.hasRetirementCleanups(bindings)) throw new Error('cem.runtime.retirement.cleanup');
    retirementFlights.add(domain);
    let preparing = false;
    try {
        if (handoff.current() !== true || !current()) throw new Error('cem.runtime.retirement.stale');
        const bytes = handoff.retain();
        if (!(bytes instanceof Uint8Array) || bytes.byteLength < 1 || bytes.byteLength > 1024 * 1024 || !(bytes.buffer instanceof ArrayBuffer))
            throw new Error('cem.runtime.retirement.handoff');
        const checkpoint = new Uint8Array(bytes);
        if (handoff.current() !== true || !current()) throw new Error('cem.runtime.retirement.stale');
        preparing = true; terminalRenderingGenerations.add(domain);
        if (fallback) {
            const started = performance.now();
            domain.retireOwnedResources(bindings, () => handoff.current() === true && current(),
                () => performance.now() - started < timeoutMs && domain.drainCurrent(proof) && !!closedConsent?.current(), check => fallback.commit(check));
            return completedRenderingRetirement(domain, checkpoint);
        }
        if (!endpoint) throw new Error('cem.runtime.retirement.unsupported');
        const confirmation = await retirementDeadline(() => endpoint.prepare(), timeoutMs);
        if (!confirmation || typeof confirmation !== 'object') throw new Error('cem.runtime.retirement.confirmation');
        const record = confirmation as Record<string, unknown>;
        if (Object.keys(record).length !== 5 || record.schemaVersion !== 1 || record.frozen !== true || record.replacementAllowed !== false
            || !Number.isSafeInteger(record.inventoryRevision) || (record.inventoryRevision as number) < 1
            || !Number.isSafeInteger(record.frozenRevision) || (record.frozenRevision as number) <= (record.inventoryRevision as number))
            throw new Error('cem.runtime.retirement.confirmation');
        if (handoff.current() !== true || !current()) throw new Error('cem.runtime.retirement.stale');
        endpoint.terminate(); domain.revoke();
        return completedRenderingRetirement(domain, checkpoint);
    } catch (error) {
        if (preparing) {
            // Owned native roots may already have been released. Preserve host
            // checkpoint/DOM, revoke old handles, and never select fallback/replay.
            domain.revoke();
            try { if (endpoint) await retirementDeadline(() => endpoint.abort(), timeoutMs); } catch { /* Uncertain cleanup remains fenced. */ }
        }
        throw error;
    } finally { retirementFlights.delete(domain); }
}

export function nativeCheckpointRetirementStarted(client: CemCheckpointNativeClient): boolean {
    const domain = clients.get(client)?.domain;
    return !!domain && terminalRenderingGenerations.has(domain);
}

export function nativeGenerationParticipant(client: CemCheckpointNativeClient): CemNativeGenerationParticipant {
    const domain = clients.get(client)?.domain;
    if (!domain) throw new Error('cem.runtime.allocation.unsupported');
    return domain.generationParticipant();
}

function completedRenderingRetirement(domain: CemNativeCheckpointDomain, checkpoint: Uint8Array): CemRetiredRenderingGeneration {
    const participant = domain.retainedAllocationParticipant();
    return participant ? completeNativeRetirement(participant, checkpoint) : Object.freeze({ checkpoint, replacementAllowed: false });
}
