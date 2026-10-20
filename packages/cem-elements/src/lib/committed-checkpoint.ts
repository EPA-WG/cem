/** Host-only storage: no WASM views, owner objects, DOM nodes or callbacks are
 * retained. Construct before pressure; two fixed slots preserve the last good
 * checkpoint while validating a replacement. No runtime restart/action replay. */
export interface CemCheckpointOwnerReceipt {
    instanceId: string;
    declarationIdentity: string;
    dataRevision: string;
    acceptedEventSequence: number;
}
export interface CemCommitReceipt {
    runtimeId: string;
    generation: number;
    commitSequence: number;
    policyStamp: string;
    owners: CemCheckpointOwnerReceipt[];
    operations: { instanceId: string; operationId: string; outcome: 'acknowledged' | 'unknown' | 'not-applied' }[];
}
export type CemCheckpointSealer = (candidate: string, receipt: string) => string | Promise<string>;
export interface CemStoredCheckpoint {
    /** Independent owned bytes, never a view into the store or WASM memory. */
    bytes: Uint8Array;
    receipt: CemCommitReceipt;
}
const MAX_BYTES = 1024 * 1024;

export class CemCommittedCheckpointStore {
    private readonly slots = [new Uint8Array(MAX_BYTES), new Uint8Array(MAX_BYTES)];
    private slot = 0;
    private length = 0;
    private acknowledged?: string;
    private committed?: string;
    private generation = 0;
    private sequence = 0;
    private epoch = 0;
    private pending = false;
    private disposed = false;

    constructor(private readonly runtimeId: string) {}

    /** Call only after all affected owners have acknowledged atomic DOM/CSS
     * publication. This trusted host receipt must not come from authored data. */
    acknowledge(receipt: CemCommitReceipt): void {
        if (this.disposed || receipt.runtimeId !== this.runtimeId
            || !Number.isSafeInteger(receipt.generation) || receipt.generation < 1
            || !Number.isSafeInteger(receipt.commitSequence) || receipt.commitSequence < 1
            || receipt.generation < this.generation
            || (receipt.generation === this.generation && receipt.commitSequence <= this.sequence)) {
            throw new Error('cem.checkpoint.stale');
        }
        const json = JSON.stringify(receipt);
        if (json.length > MAX_BYTES) throw new Error('cem.checkpoint.limit');
        this.acknowledged = json;
        this.generation = receipt.generation;
        this.sequence = receipt.commitSequence;
        this.epoch++;
    }

    /** Invalidates an in-flight capture on input/owner/policy changes even
     * before another publication. Old bytes remain exportable, not current. */
    invalidate(): void {
        this.acknowledged = undefined;
        this.epoch++;
    }

    async capture(candidate: string, seal: CemCheckpointSealer, signal?: AbortSignal): Promise<void> {
        if (this.disposed || this.pending || !this.acknowledged || signal?.aborted) {
            throw new Error('cem.checkpoint.unavailable');
        }
        if (candidate.length > MAX_BYTES) throw new Error('cem.checkpoint.limit');
        const epoch = this.epoch;
        const receipt = this.acknowledged;
        this.pending = true;
        try {
            // Native sealing owns validation; the host does not parse an AST or
            // derive equality from authored hashes/revisions.
            const sealed = await seal(candidate, receipt);
            if (this.disposed || signal?.aborted || epoch !== this.epoch) {
                throw new Error('cem.checkpoint.stale');
            }
            if (sealed.length > MAX_BYTES) throw new Error('cem.checkpoint.limit');
            const next = 1 - this.slot;
            const result = new TextEncoder().encodeInto(sealed, this.slots[next]);
            if (result.read !== sealed.length) {
                this.slots[next].fill(0);
                throw new Error('cem.checkpoint.limit');
            }
            this.slots[this.slot].fill(0);
            this.slot = next;
            this.length = result.written;
            this.committed = receipt;
        } finally {
            this.pending = false;
        }
    }

    /** Last successful export, possibly older than live state. For recovery
     * use current(); a failed capture never authorizes rollback of newer edits. */
    lastCommitted(): CemStoredCheckpoint | undefined {
        if (this.disposed || !this.committed) return undefined;
        return { bytes: this.slots[this.slot].slice(0, this.length), receipt: JSON.parse(this.committed) };
    }

    current(): CemStoredCheckpoint | undefined {
        return this.acknowledged === this.committed ? this.lastCommitted() : undefined;
    }

    /** Erase session-private state on host/session disposal. */
    dispose(): void {
        this.disposed = true;
        this.invalidate();
        this.committed = undefined;
        this.length = 0;
        for (const slot of this.slots) slot.fill(0);
        this.slots.length = 0;
    }
}
