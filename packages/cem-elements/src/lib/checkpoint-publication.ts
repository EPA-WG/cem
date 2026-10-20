import { CemCommittedCheckpointStore, type CemCheckpointOwnerReceipt, type CemCheckpointSealer, type CemCommitReceipt, type CemStoredCheckpoint } from './committed-checkpoint.js';

export interface CemCheckpointPublicationOptions {
    /** Trusted host session/domain identity. All clients of a replaceable runtime
     * must share this coordinator; this API never discovers a WASM heap. */
    runtimeId: string;
    generation: number;
    policyStamp: string;
    seal: CemCheckpointSealer;
    /** Explicit whole-island consent; absent/false denies recovery for the domain. */
    allowOwner?: (owner: Readonly<CemCheckpointOwnerReceipt>) => boolean;
    /** Coalesce automatic captures after publication/admission drain. Default false. */
    automatic?: boolean;
}
export interface CemCheckpointOperation { acknowledge(): void }
export interface CemCheckpointPublicationOwner {
    invalidate(): void;
    input(): void;
    published(receipt: Omit<CemCheckpointOwnerReceipt, 'acceptedEventSequence'>): void;
    remove(): void;
}
interface Owner {
    id: string;
    sequence: number;
    receipt?: CemCheckpointOwnerReceipt;
    exportIsland: () => string | undefined | Promise<string | undefined>;
    current: () => boolean;
}
const MAX_BYTES = 1024 * 1024;
const MAX_OWNERS = 256;

/** Host-side publication barrier. Keeps only active export hooks, bounded
 * metadata and the last sealed bytes, never event payload queues or WASM views.
 * No restart, input interception, action retry or semantic resume authority. */
export class CemCheckpointPublicationCoordinator {
    private store: CemCommittedCheckpointStore;
    private readonly owners = new Map<string, Owner>();
    private readonly operations = new Map<string, CemCommitReceipt['operations'][number]>();
    private policySequence = 0;
    private readonly admissions = new Set<object>();
    private policyStamp: string;
    private allowOwner?: CemCheckpointPublicationOptions['allowOwner'];
    private sequence = 0;
    private epoch = 0;
    private capturing = false;
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

    register(id: string, exportIsland: () => string | undefined | Promise<string | undefined>, current: () => boolean = () => true, initialEventSequence = 0): CemCheckpointPublicationOwner {
        if (this.disposed || this.owners.has(id) || !id || this.owners.size >= MAX_OWNERS
            || !Number.isSafeInteger(initialEventSequence) || initialEventSequence < 0) {
            // An unregistered owner must never disappear from the all-owner proof.
            this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.owners');
        }
        const owner: Owner = { id, exportIsland, current, sequence: initialEventSequence };
        this.owners.set(id, owner); this.invalidate();
        const active = () => { if (this.disposed || this.owners.get(id) !== owner) throw new Error('cem.checkpoint.stale'); };
        return {
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
                this.invalidate(); this.schedule();
            },
            remove: () => {
                if (this.owners.get(id) !== owner) return;
                this.owners.delete(id); this.invalidate(); this.schedule();
            },
        };
    }

    /** Record before delivering a host operation. No arguments, command names
     * or callbacks enter the journal, and no operation is ever retried. */
    operation(instanceId: string, operationId: string): CemCheckpointOperation {
        const owner = this.owners.get(instanceId);
        const key = JSON.stringify([instanceId, operationId]);
        if (this.disposed || !owner || !operationId || operationId.length > 1024 || /\p{Cc}/u.test(operationId)
            || this.operations.has(key) || this.operations.size >= 256) {
            this.blocked = true; this.invalidate(); throw new Error('cem.checkpoint.operations.limit');
        }
        const record: CemCommitReceipt['operations'][number] = { instanceId, operationId, outcome: 'unknown' };
        this.operations.set(key, record); this.invalidate(); this.schedule();
        const source = new WeakRef(owner), policy = this.policySequence;
        return { acknowledge: () => {
            if (this.disposed || this.policySequence !== policy || this.owners.get(instanceId) !== source.deref()
                || record.outcome === 'acknowledged') return;
            record.outcome = 'acknowledged'; this.invalidate(); this.schedule();
        } };
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
        this.policySequence++;
        this.policyStamp = stamp; this.allowOwner = allowOwner;
        for (const owner of this.owners.values()) owner.receipt = undefined;
    }

    private invalidate(): void { this.epoch++; this.store.invalidate(); }
    private ready(): boolean {
        if (this.disposed || this.blocked || this.admissions.size || !this.owners.size) return false;
        for (const operation of this.operations.values()) if (!this.owners.has(operation.instanceId)) return false;
        for (const owner of this.owners.values()) {
            if (!owner.receipt || !owner.current()) { this.store.invalidate(); return false; }
        }
        return true;
    }

    async capture(signal?: AbortSignal): Promise<void> {
        if (this.capturing || signal?.aborted || !this.ready()) throw new Error('cem.checkpoint.unavailable');
        this.capturing = true;
        const epoch = this.epoch;
        const store = this.store;
        try {
            const sources = [...this.owners.values()];
            const receipt: CemCommitReceipt = { runtimeId: this.options.runtimeId, generation: this.options.generation,
                commitSequence: ++this.sequence, policyStamp: this.policyStamp,
                owners: sources.map(owner => ({ ...owner.receipt! })), operations: [...this.operations.values()].map(record => ({ ...record })) };
            const owners = [];
            let bytes = 0;
            for (const owner of sources) {
                if (epoch !== this.epoch || !this.ready()) throw new Error('cem.checkpoint.stale');
                if (this.allowOwner?.(Object.freeze({ ...owner.receipt! })) !== true) throw new Error('cem.checkpoint.privacy');
                const exported = owner.exportIsland();
                const island = typeof exported === 'string' || exported === undefined ? exported : await exported;
                if (epoch !== this.epoch || !this.ready()) throw new Error('cem.checkpoint.stale');
                if (typeof island !== 'string' || !island) throw new Error('cem.checkpoint.unavailable');
                bytes += island.length;
                if (bytes > MAX_BYTES) throw new Error('cem.checkpoint.limit');
                owners.push({ instanceId: owner.id, island });
            }
            if (epoch !== this.epoch || !this.ready()) throw new Error('cem.checkpoint.stale');
            const candidate = JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt, owners });
            if (candidate.length > MAX_BYTES || new TextEncoder().encode(candidate).byteLength > MAX_BYTES) throw new Error('cem.checkpoint.limit');
            store.acknowledge(receipt);
            await store.capture(candidate, async (candidateJson, receiptJson) => {
                const sealed = await this.options.seal(candidateJson, receiptJson);
                if (epoch !== this.epoch || !this.ready()) throw new Error('cem.checkpoint.stale');
                return sealed;
            }, signal);
            if (epoch !== this.epoch || !this.ready()) { store.invalidate(); throw new Error('cem.checkpoint.stale'); }
            this.error = undefined;
        } finally {
            this.capturing = false;
            // At most one successor after newer publication, never an input queue.
            if (epoch !== this.epoch) this.schedule();
        }
    }

    current(): CemStoredCheckpoint | undefined { return this.ready() ? this.store.current() : undefined; }
    lastCommitted(): CemStoredCheckpoint | undefined { return this.store.lastCommitted(); }
    dispose(): void {
        this.disposed = true; this.invalidate(); this.store.dispose(); this.owners.clear(); this.admissions.clear(); this.operations.clear();
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
