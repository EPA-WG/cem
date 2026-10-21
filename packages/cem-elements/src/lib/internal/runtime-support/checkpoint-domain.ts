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

export class CemNativeCheckpointDomain {
    private readonly runtimeId = crypto.randomUUID();
    private readonly members = new Set<Client>();
    private readonly resources = new Map<string, CemCheckpointNativeClient | undefined>();
    private revision = 0;
    private alive = true;
    private nativeIdentity?: object;
    private overflow = false;

    constructor(private readonly mode: 'worker' | 'main-thread') {}

    client(): CemCheckpointNativeClient {
        const token = Object.freeze({ clientId: crypto.randomUUID() });
        const client: Client = { token, domain: this, pending: 0, listeners: new Set() };
        clients.set(token, client);
        if (this.members.size >= LIMIT) this.overflow = true; else this.members.add(client);
        this.changed();
        return token;
    }

    move(token: CemCheckpointNativeClient): void {
        const client = clients.get(token);
        if (!client || client.domain === this) return;
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
    revoke(): void { this.alive = false; this.changed(); }

    /** Retained support work begins unowned. Only its processing engine may
     * adopt it after successful preparation; direct callers remain a blocker. */
    retain(kind: string, id: number): void {
        if (this.resources.size >= 4096) this.overflow = true;
        else this.resources.set(`${kind}:${id}`, undefined);
        this.changed();
    }
    release(kind: string, id: number): void { if (this.resources.delete(`${kind}:${id}`)) this.changed(); }
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
        if (!this.alive || this.overflow || !this.members.size) return undefined;
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

    issuedIdentity(): string { return this.runtimeId; }
    identity(): string | undefined { return this.alive && !this.overflow ? this.runtimeId : undefined; }

    changed(): void {
        if (!Number.isSafeInteger(++this.revision)) this.overflow = true;
        for (const client of this.members) for (const listener of client.listeners) {
            try { listener(); } catch { /* Observers cannot change native work or suppress peer invalidation. */ }
        }
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
export function nativeCheckpointRefuse(token: CemCheckpointNativeClient): void { clients.get(token)?.domain.revoke(); }
export function nativeCheckpointChanged(token: CemCheckpointNativeClient): void { clients.get(token)?.domain.changed(); }
export function nativeCheckpointAdmission(token: CemCheckpointNativeClient): () => void {
    const client = clients.get(token);
    if (!client) return () => undefined;
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
export function nativeCheckpointWork<Args extends unknown[], Result>(operation: (...args: Args) => Promise<Result>): (...args: Args) => Promise<Result> {
    return async (...args) => {
        const client = cemMainThreadCheckpointDomain.client();
        try { return await operation(...args); }
        finally { cemMainThreadCheckpointDomain.remove(client); }
    };
}
