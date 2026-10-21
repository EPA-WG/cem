import type { CemFallbackRetirementConsent } from './internal/runtime-support/native-runtime.js';
import { closeNativeRenderingGeneration, retireNativeRenderingGeneration, type CemNativeDrain, nativeCheckpointIdentity, nativeCheckpointValidSourceProof, nativeCheckpointSubscribe, nativeCheckpointSnapshot, type CemCheckpointNativeClient, type CemCheckpointNativeSourceProof } from './internal/runtime-support/checkpoint-domain.js';
import type { CheckpointEndpoint } from './checkpoint-interactions.js';
import { CemCommittedCheckpointStore, type CemCheckpointOwnerReceipt, type CemCheckpointSealer, type CemCommitReceipt, type CemStoredCheckpoint, type CemCheckpointRelationship } from './committed-checkpoint.js';

export interface CemCheckpointPublicationOptions {
    /** Logical host/session identity. Physical runtime coverage is issued by the
     * shared processing hosts independently of these caller-supplied labels. */
    runtimeId: string;
    generation: number;
    policyStamp: string;
    seal: CemCheckpointSealer;
    /** Explicit whole-island consent; absent/false denies recovery for the domain. */
    allowOwner?: (owner: Readonly<CemCheckpointOwnerReceipt>) => boolean;
    /** Coalesce automatic captures after publication/admission drain. Default false. */
    automatic?: boolean;
}
export interface CemCheckpointJournalAuthority { runtimeId: string; generation: number; policyStamp: string }
export interface CemCheckpointOperation { readonly operationId: string; acknowledge(): void }
export interface CemCheckpointPublicationOwner {
    readonly acknowledged: boolean;
    invalidate(): void;
    input(): void;
    published(receipt: Omit<CemCheckpointOwnerReceipt, 'acceptedEventSequence'>): void;
    remove(): void;
}
/** Explicit host registration, never inferred from attributes or instance IDs. */
export interface CemCheckpointEndpointOwner {
    host: HTMLElement;
    nativeClient?: CemCheckpointNativeClient;
    nativeSources?: () => readonly CemCheckpointNativeSourceProof[] | undefined;
    resolve: (element: HTMLElement) => CheckpointEndpoint | undefined;
    /** Retry an already-settled interaction after a peer finishes publication. */
    refresh?: () => void;
}
interface Owner {
    id: string;
    endpoint?: CemCheckpointEndpointOwner;
    sequence: number;
    receipt?: CemCheckpointOwnerReceipt;
    publicationPolicy?: number;
    publicationRuntime?: string;
    stopNative?: () => void;
    exportIsland: () => string | undefined | Promise<string | undefined>;
    current: () => boolean;
    dependencies: () => CemCheckpointRelationship[] | undefined;
}
const MAX_BYTES = 1024 * 1024;
const MAX_OWNERS = 256;

/** Host-side publication barrier. Keeps only active export hooks, bounded
 * metadata and the last sealed bytes, never event payload queues or WASM views.
 * No restart, input interception, action retry or semantic resume authority. */
export class CemCheckpointPublicationCoordinator {
    private store: CemCommittedCheckpointStore;
    private readonly sourceClients = new Map<CemCheckpointNativeClient, () => void>();
    private readonly endpoints = new WeakMap<HTMLElement, Owner>();
    private readonly owners = new Map<string, Owner>();
    private readonly operations = new Map<string, CemCommitReceipt['operations'][number]>();
    private readonly retirement = new Set<string>();
    private readonly relationships = new Map<string, { descriptor: CemCheckpointRelationship; current: () => boolean; policy: number }>();
    private readonly journalIdentity = crypto.randomUUID();
    private operationSequence = 0;
    private policySequence = 0;
    private readonly admissions = new Set<object>();
    private policyStamp: string;
    private allowOwner?: CemCheckpointPublicationOptions['allowOwner'];
    private sequence = 0;
    private epoch = 0;
    private capturing = false;
    private dependencyIdentity?: string;
    private scheduled = false;
    private disposed = false;
    private blocked = false;
    private error?: string;

    constructor(private readonly options: CemCheckpointPublicationOptions) {
        if (!options.runtimeId || !options.policyStamp || !Number.isSafeInteger(options.generation) || options.generation < 1) throw new Error('cem.checkpoint.identity');
        this.store = new CemCommittedCheckpointStore(options.runtimeId);
        this.policyStamp = options.policyStamp;
        this.allowOwner = options.allowOwner;
        // Copy host options: mutation of a caller's record is not a policy update.
        this.options = { ...options };
    }

    get lastError(): string | undefined { return this.error; }

    register(id: string, exportIsland: () => string | undefined | Promise<string | undefined>, current: () => boolean = () => true, initialEventSequence = 0, dependencies: () => CemCheckpointRelationship[] | undefined = () => [], endpoint?: CemCheckpointEndpointOwner): CemCheckpointPublicationOwner {
        if (this.disposed || this.owners.has(id) || endpoint && this.endpoints.has(endpoint.host) || !id || this.owners.size >= MAX_OWNERS
            || !Number.isSafeInteger(initialEventSequence) || initialEventSequence < 0) {
            // An unregistered owner must never disappear from the all-owner proof.
            this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.owners');
        }
        const owner: Owner = { id, exportIsland, current, sequence: initialEventSequence, dependencies, ...(endpoint ? { endpoint: { ...endpoint } } : {}) };
        if (owner.endpoint?.nativeClient) {
            try { owner.stopNative = nativeCheckpointSubscribe(owner.endpoint.nativeClient, () => { this.invalidate(); this.schedule(); }); }
            catch { this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.native-client'); }
        }
        this.owners.set(id, owner); if (owner.endpoint) this.endpoints.set(owner.endpoint.host, owner); this.invalidate();
        const acknowledged = () => !this.disposed && this.owners.get(id) === owner && !!owner.receipt;
        const active = () => { if (this.disposed || this.owners.get(id) !== owner) throw new Error('cem.checkpoint.stale'); };
        return {
            get acknowledged() { return acknowledged(); },
            invalidate: () => { if (this.disposed) return; active(); owner.receipt = undefined; this.invalidate(); },
            input: () => {
                if (this.disposed) return;
                active();
                if (!Number.isSafeInteger(owner.sequence + 1)) { this.blocked = true; throw new Error('cem.checkpoint.limit'); }
                owner.sequence++; owner.receipt = undefined; this.invalidate();
            },
            published: receipt => {
                if (this.disposed) return;
                active();
                if (receipt.instanceId !== id || !receipt.declarationIdentity || !receipt.dataRevision) throw new Error('cem.checkpoint.identity');
                owner.receipt = { ...receipt, acceptedEventSequence: owner.sequence };
                owner.publicationPolicy = this.policySequence;
                owner.publicationRuntime = owner.endpoint?.nativeClient ? nativeCheckpointIdentity(owner.endpoint.nativeClient) : undefined;
                this.invalidate(); this.schedule();
            },
            remove: () => {
                if (this.owners.get(id) !== owner) return;
                owner.stopNative?.(); this.owners.delete(id); if (owner.endpoint) this.endpoints.delete(owner.endpoint.host); this.invalidate(); this.schedule();
            },
        };
    }

    /** Resolve only through the nearest explicitly registered owner in this domain.
     * Full current()/receipt checks belong to the publication barrier: resolving an
     * owner's dependency while computing that same fingerprint must not recurse. */
    endpoint(element: HTMLElement): CheckpointEndpoint | undefined {
        if (this.disposed || this.blocked || !element.isConnected) return undefined;
        for (let host: HTMLElement | null = element; host; host = host.parentElement) {
            const owner = this.endpoints.get(host);
            if (owner) {
                if (this.owners.get(owner.id) !== owner) return undefined;
                const endpoint = owner.endpoint?.resolve(element);
                return endpoint?.instanceId === owner.id ? { ...endpoint } : undefined;
            }
            if (host.ownerDocument.defaultView?.customElements.get(host.localName)
                || Array.from(host.children).some(child => child.localName === 'template' && child.getAttribute('data-cem-island') === 'instance')) return undefined;
        }
        return undefined;
    }

    /** Trusted host registration of a complete dependency intent. This does not
     * admit unsupported widget owners or authorize a restored relationship.
     * Endpoint publication revisions and current host authority must still match. */
    relationship(descriptor: CemCheckpointRelationship, current: () => boolean, authority: CemCheckpointJournalAuthority): () => void {
        this.journalAuthority(authority);
        if (!descriptor.id || this.relationships.has(descriptor.id) || this.relationships.size >= 256) {
            this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.relationships.limit');
        }
        // Copy the named scalar contract, never retain the caller's mutable record.
        const record = { descriptor: { ...descriptor, requester: { ...descriptor.requester }, provider: { ...descriptor.provider } }, current, policy: this.policySequence };
        this.relationships.set(descriptor.id, record); this.invalidate(); this.schedule();
        return () => {
            if (this.relationships.get(record.descriptor.id) !== record) return;
            this.relationships.delete(record.descriptor.id); this.invalidate(); this.schedule();
        };
    }

    /** Record before delivering a host operation. No arguments, command names
     * or callbacks enter the journal, and no operation is ever retried. */
    operation(instanceId: string): CemCheckpointOperation {
        const owner = this.owners.get(instanceId);
        if (this.disposed || !owner || this.operations.size >= 256 || !Number.isSafeInteger(this.operationSequence + 1)) {
            this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.operations.limit');
        }
        // Monotonic, host-issued identities need no unbounded retired-ID set.
        const operationId = `operation:${this.journalIdentity}:${++this.operationSequence}`;
        const record: CemCommitReceipt['operations'][number] = { instanceId, operationId, outcome: 'unknown' };
        this.operations.set(operationId, record); this.invalidate(); this.schedule();
        const source = new WeakRef(owner), policy = this.policySequence;
        return Object.freeze({ operationId, acknowledge: () => {
            if (this.disposed || this.policySequence !== policy || this.owners.get(instanceId) !== source.deref()
                || this.operations.get(operationId) !== record || record.outcome !== 'unknown') return;
            record.outcome = 'acknowledged'; this.invalidate(); this.schedule();
        } });
    }

    private journalAuthority(authority: CemCheckpointJournalAuthority): void {
        if (this.disposed || authority.runtimeId !== this.options.runtimeId || authority.generation !== this.options.generation
            || authority.policyStamp !== this.policyStamp) throw new Error('cem.checkpoint.operations.authority');
    }

    /** Trusted host reconciliation after independently establishing the outcome.
     * Authored data and a late handler completion cannot supply this authority. */
    reconcileOperation(operationId: string, outcome: 'acknowledged' | 'not-applied', authority: CemCheckpointJournalAuthority): void {
        this.journalAuthority(authority);
        const record = this.operations.get(operationId);
        if (!record || !['acknowledged', 'not-applied'].includes(outcome)
            || record.outcome !== 'unknown' && record.outcome !== outcome) throw new Error('cem.checkpoint.operations.reconciliation');
        record.outcome = outcome; this.invalidate(); this.schedule();
    }

    /** Stage terminal records for retirement. Only the next successful complete
     * all-owner seal compacts the journal. Failures retain records and intent. */
    retireOperations(operationIds: readonly string[], authority: CemCheckpointJournalAuthority): void {
        this.journalAuthority(authority);
        if (!operationIds.length || operationIds.length > 256 || new Set(operationIds).size !== operationIds.length)
            throw new Error('cem.checkpoint.operations.retirement');
        for (const id of operationIds) {
            const record = this.operations.get(id);
            if (!record) throw new Error('cem.checkpoint.operations.retirement');
            if (record.outcome === 'unknown') throw new Error('cem.checkpoint.operations.unknown');
        }
        for (const id of operationIds) this.retirement.add(id);
        this.invalidate(); this.schedule();
    }

    /** Track preparation/host admissions, including superseded work until its
     * finally block drains. Refuse rather than buffer work beyond the ceiling. */
    admit(): () => void {
        if (this.disposed || this.admissions.size >= MAX_OWNERS) { this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.limit'); }
        const lease = {}; this.admissions.add(lease); this.invalidate();
        return () => { if (this.admissions.delete(lease)) this.schedule(); };
    }

    /** Policy boundaries erase older private bytes, including last-good exports. */
    setPolicy(stamp: string, allowOwner?: CemCheckpointPublicationOptions['allowOwner']): void {
        if (this.disposed || !stamp) throw new Error('cem.checkpoint.identity');
        this.invalidate(); this.store.dispose(); this.store = new CemCommittedCheckpointStore(this.options.runtimeId);
        this.policySequence++; this.retirement.clear();
        this.policyStamp = stamp; this.allowOwner = allowOwner;
        for (const owner of this.owners.values()) owner.receipt = undefined;
    }

    private invalidate(): void { this.epoch++; this.store.invalidate(); }
    private nativeSnapshot() {
        const bindings = new Map<CemCheckpointNativeClient, string[]>();
        const sources = new Map<CemCheckpointNativeClient, Set<string>>();
        let managed = false;
        for (const owner of this.owners.values()) {
            const client = owner.endpoint?.nativeClient;
            if (!client) continue;
            managed = true;
            if (!owner.publicationRuntime || owner.publicationRuntime !== nativeCheckpointIdentity(client)) return undefined;
            const owners = bindings.get(client) ?? []; if (!owners.includes(owner.id)) owners.push(owner.id); bindings.set(client, owners);
            const proofs = owner.endpoint?.nativeSources?.();
            if (owner.endpoint?.nativeSources && !proofs) return undefined;
            for (const proof of proofs ?? []) {
                if (!nativeCheckpointValidSourceProof(proof) || !proof.current()) return undefined;
                const nativeOwners = bindings.get(proof.client) ?? [];
                if (!nativeOwners.includes(owner.id)) nativeOwners.push(owner.id);
                bindings.set(proof.client, nativeOwners);
                const resources = sources.get(proof.client) ?? new Set<string>(); resources.add(proof.resource); sources.set(proof.client, resources);
            }
        }
        // Logical-only callers retain transport compatibility. Once a managed
        // client participates, every owner and every physical peer must be covered.
        if (!managed) return [];
        if ([...this.owners.values()].some(owner => !owner.endpoint?.nativeClient)) return undefined;
        for (const [client, stop] of this.sourceClients) if (!sources.has(client)) { stop(); this.sourceClients.delete(client); }
        for (const client of sources.keys()) if (!this.sourceClients.has(client)) {
            try { this.sourceClients.set(client, nativeCheckpointSubscribe(client, () => { this.invalidate(); this.schedule(); })); }
            catch { return undefined; }
        }
        return nativeCheckpointSnapshot(bindings, sources);
    }
    private dependencySnapshot(): CemCheckpointRelationship[] | undefined {
        const dependencies = [...this.relationships.values()].map(record => record.descriptor);
        for (const owner of this.owners.values()) {
            const next = owner.dependencies();
            if (!next || next.some(edge => edge.requester.instanceId !== owner.id)) return undefined;
            dependencies.push(...next); if (dependencies.length > 256) return undefined;
        }
        for (const edge of dependencies) for (const endpoint of [edge.requester, edge.provider]) {
            if (this.owners.get(endpoint.instanceId)?.receipt?.dataRevision !== endpoint.dataRevision) return undefined;
        }
        return dependencies;
    }
    private ready(): boolean {
        if (this.disposed || this.blocked || this.admissions.size || !this.owners.size) return false;
        for (const operation of this.operations.values()) if (!this.owners.has(operation.instanceId) && !this.retirement.has(operation.operationId)) return false;
        let acknowledged = true;
        for (const owner of this.owners.values()) {
            // A capability may have settled while a peer was unpublished. Retry
            // its acknowledgement without recursively evaluating endpoint owners
            // or starting another render while that owner's work is in flight.
            if (!owner.receipt) {
                if (owner.publicationPolicy === this.policySequence) owner.endpoint?.refresh?.();
                acknowledged = false;
            }
            else if (!owner.current()) acknowledged = false;
        }
        if (!acknowledged) { this.store.invalidate(); return false; }
        for (const relationship of this.relationships.values()) {
            if (relationship.policy !== this.policySequence || !relationship.current()) { this.store.invalidate(); return false; }
            for (const endpoint of [relationship.descriptor.requester, relationship.descriptor.provider]) {
                const owner = this.owners.get(endpoint.instanceId);
                if (!owner?.receipt || owner.receipt.dataRevision !== endpoint.dataRevision) { this.store.invalidate(); return false; }
            }
        }
        const dependencies = this.dependencySnapshot();
        const native = this.nativeSnapshot();
        if (!dependencies || !native) { this.store.invalidate(); return false; }
        const identity = JSON.stringify([dependencies, native]);
        if (this.dependencyIdentity !== identity) { this.dependencyIdentity = identity; this.invalidate(); }
        return true;
    }

    async capture(signal?: AbortSignal): Promise<void> {
        if (this.capturing || signal?.aborted || !this.ready()) throw new Error('cem.checkpoint.unavailable');
        this.capturing = true;
        const epoch = this.epoch;
        const store = this.store;
        try {
            const sources = [...this.owners.values()];
            const retired = [...this.retirement];
            const dependencies = this.dependencySnapshot();
            if (!dependencies) throw new Error('cem.checkpoint.unavailable');
            const native = this.nativeSnapshot();
            if (!native) throw new Error('cem.checkpoint.unavailable');
            const dependencyIdentity = JSON.stringify([dependencies, native]);
            const dependenciesCurrent = () => JSON.stringify([this.dependencySnapshot(), this.nativeSnapshot()]) === dependencyIdentity;
            const receipt: CemCommitReceipt = { runtimeId: this.options.runtimeId, generation: this.options.generation,
                commitSequence: ++this.sequence, policyStamp: this.policyStamp,
                ...(native.length ? { nativeRuntimes: native } : {}),
                owners: sources.map(owner => ({ ...owner.receipt! })),
                ...(dependencies.length ? { relationships: dependencies.map(descriptor => ({ ...descriptor, requester: { ...descriptor.requester }, provider: { ...descriptor.provider } })) } : {}), operations: [...this.operations.values()].filter(record => !this.retirement.has(record.operationId)).map(record => ({ ...record })) };
            const owners = [];
            let bytes = 0;
            for (const owner of sources) {
                if (epoch !== this.epoch || !this.ready() || !dependenciesCurrent()) throw new Error('cem.checkpoint.stale');
                if (this.allowOwner?.(Object.freeze({ ...owner.receipt! })) !== true) throw new Error('cem.checkpoint.privacy');
                const exported = owner.exportIsland();
                const island = typeof exported === 'string' || exported === undefined ? exported : await exported;
                if (epoch !== this.epoch || !this.ready() || !dependenciesCurrent()) throw new Error('cem.checkpoint.stale');
                if (typeof island !== 'string' || !island) throw new Error('cem.checkpoint.unavailable');
                bytes += island.length;
                if (bytes > MAX_BYTES) throw new Error('cem.checkpoint.limit');
                owners.push({ instanceId: owner.id, island });
            }
            if (epoch !== this.epoch || !this.ready() || !dependenciesCurrent()) throw new Error('cem.checkpoint.stale');
            const candidate = JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt, owners });
            if (candidate.length > MAX_BYTES || new TextEncoder().encode(candidate).byteLength > MAX_BYTES) throw new Error('cem.checkpoint.limit');
            store.acknowledge(receipt);
            await store.capture(candidate, async (candidateJson, receiptJson) => {
                const sealed = await this.options.seal(candidateJson, receiptJson);
                if (epoch !== this.epoch || !this.ready() || !dependenciesCurrent()) throw new Error('cem.checkpoint.stale');
                return sealed;
            }, signal);
            if (epoch !== this.epoch || !this.ready() || !dependenciesCurrent()) { store.invalidate(); throw new Error('cem.checkpoint.stale'); }
            for (const id of retired) { this.operations.delete(id); this.retirement.delete(id); }
            this.error = undefined;
        } finally {
            this.capturing = false;
            // At most one successor after newer publication, never an input queue.
            if (epoch !== this.epoch) this.schedule();
        }
    }

    current(): CemStoredCheckpoint | undefined { return this.ready() ? this.store.current() : undefined; }
    /** Shared lifecycle adapter; no app-local event interception or state serializer. */
    handoff(): import('./committed-checkpoint.js').CemCheckpointRetirementHandoff {
        if (this.capturing || !this.ready()) throw new Error('cem.checkpoint.unavailable');
        const epoch = this.epoch;
        const handoff = this.store.handoff();
        const current = () => !this.capturing && epoch === this.epoch && this.ready() && handoff.current();
        return Object.freeze({ current, retain: () => {
            if (!current()) throw new Error('cem.checkpoint.stale');
            return handoff.retain();
        } });
    }
    /** Close one complete physical rendering generation, then capture again
     * before retirement. Owned fallback also requires the original consent token;
     * multi-heap transactions and legacy fallback remain unsupported. */
    async closeRenderingGeneration(timeoutMs = 5000, consent?: CemFallbackRetirementConsent): Promise<CemNativeDrain> {
        const coverage = this.renderingCoverage();
        const client = coverage.bindings.keys().next().value;
        if (!client) throw new Error('cem.runtime.retirement.coverage');
        return closeNativeRenderingGeneration(client, timeoutMs, consent);
    }

    async retireRenderingGeneration(proof: CemNativeDrain, timeoutMs = 5000, consent?: CemFallbackRetirementConsent): Promise<import('./internal/runtime-support/checkpoint-domain.js').CemRetiredRenderingGeneration> {
        const coverage = this.renderingCoverage();
        return retireNativeRenderingGeneration(coverage.bindings, coverage.sources, proof, this.handoff(), timeoutMs, consent);
    }

    private renderingCoverage() {
        if (this.disposed || this.blocked || !this.owners.size) throw new Error('cem.runtime.retirement.coverage');
        const bindings = new Map<CemCheckpointNativeClient, string[]>();
        const sources: CemCheckpointNativeSourceProof[] = [];
        const generations = new Set<string>();
        for (const owner of this.owners.values()) {
            const client = owner.endpoint?.nativeClient;
            const generation = client && nativeCheckpointIdentity(client);
            if (!client || !generation) throw new Error('cem.runtime.retirement.coverage');
            generations.add(generation);
            const owners = bindings.get(client) ?? []; owners.push(owner.id); bindings.set(client, owners);
            const proofs = owner.endpoint?.nativeSources?.();
            if (owner.endpoint?.nativeSources && !proofs) throw new Error('cem.runtime.retirement.coverage');
            for (const source of proofs ?? []) {
                sources.push(source);
                const owners = bindings.get(source.client) ?? [];
                if (!owners.includes(owner.id)) owners.push(owner.id);
                bindings.set(source.client, owners);
                const identity = nativeCheckpointIdentity(source.client);
                if (!identity) throw new Error('cem.runtime.retirement.coverage');
                generations.add(identity);
            }
        }
        if (generations.size !== 1) throw new Error('cem.runtime.retirement.multiple-generations');
        return { bindings, sources };
    }

    lastCommitted(): CemStoredCheckpoint | undefined { return this.store.lastCommitted(); }
    dispose(): void {
        this.disposed = true; this.invalidate(); this.store.dispose();
        for (const stop of this.sourceClients.values()) stop(); this.sourceClients.clear();
        for (const owner of this.owners.values()) { owner.stopNative?.(); if (owner.endpoint) this.endpoints.delete(owner.endpoint.host); }
        this.owners.clear(); this.admissions.clear(); this.operations.clear(); this.retirement.clear(); this.relationships.clear();
    }
    private schedule(): void {
        if (!this.options.automatic || this.scheduled || this.capturing || this.disposed) return;
        this.scheduled = true;
        queueMicrotask(() => {
            this.scheduled = false;
            if (!this.ready()) return;
            void this.capture().catch(error => { this.error = error instanceof Error ? error.message : String(error); });
        });
    }
}
