import { copyCommandNativeEmbeddingOwnership, CommandNativeEmbeddingOwners, parseCommandNativeInventory, type CommandNativeInventory, type CommandNativeEmbeddingOwnership } from './native-embedding.js';
/** Managed CLI admission only. This is never whole-heap replacement authority. */
export type CommandNativeClosure = 'soft-pressure' | 'hard-pressure' | 'telemetry-unavailable' | 'checkpoint';

const errorKey = Symbol.for('cem.command.native-admission.errors.v1');
const errorRealm = globalThis as typeof globalThis & { [errorKey]?: WeakSet<object> };
const admissionErrors = errorRealm[errorKey] ??= new WeakSet<object>();

export function isCommandNativeAdmissionError(error: unknown): error is CommandNativeAdmissionError {
    return typeof error === 'object' && error !== null && admissionErrors.has(error);
}

export class CommandNativeAdmissionError extends Error {
    readonly code: string;
    constructor(code = 'cem.runtime.memory.closed', message = 'Managed command native admission is closed') {
        super(message);
        this.name = 'CommandNativeAdmissionError';
        admissionErrors.add(this);
        this.code = code;
    }
}

export interface CommandNativeDrain {
    readonly revision: number;
    readonly scope: 'managed-cli-only';
    readonly replacementAllowed: false;
    readonly nativeInventory?: CommandNativeInventory;
    readonly embeddingOwners?: number;
}

const proofs = new WeakMap<object, { readonly domain: CommandNativeAdmission; readonly revision: number }>();
const MAX_PENDING = 256;
const MAX_RESOURCES = 4096;
const MAX_WAITERS = 16;

/** One ledger per generated module singleton; raw WASM aliases remain unmanaged. */
export class CommandNativeAdmission {
    #sample: () => string;
    #closed: CommandNativeClosure | undefined;
    #pending = 0;
    #resources = new Map<string, number>();
    #revision = 0;
    #failed = false;
    #timer: ReturnType<typeof setTimeout> | undefined;
    #waiters = new Set<() => void>();

    constructor(sample: () => string) { this.#sample = sample; }
    get closure(): CommandNativeClosure | undefined { return this.#closed; }

    sample(): void {
        if (this.#closed !== undefined) return;
        try {
            const json = this.#sample();
            if (typeof json !== 'string' || json.length > 2048) throw new Error('telemetry');
            const status = JSON.parse(json) as { softPressure?: unknown; hardPressure?: unknown };
            if (typeof status?.softPressure !== 'boolean' || typeof status.hardPressure !== 'boolean') throw new Error('telemetry');
            if (status.hardPressure) this.close('hard-pressure');
            else if (status.softPressure) this.close('soft-pressure');
        } catch { this.close('telemetry-unavailable'); }
    }

    admit(cleanup = false): () => void {
        this.sample();
        if (this.#closed !== undefined && !cleanup) throw new CommandNativeAdmissionError();
        if (this.#pending >= MAX_PENDING) {
            this.invalidate();
            throw new CommandNativeAdmissionError('cem.command.checkpoint.limit', 'Too many managed native calls');
        }
        this.#pending++;
        this.#changed();
        let settled = false;
        return () => {
            if (settled) return;
            settled = true;
            this.#pending--;
            this.sample();
            this.#changed();
        };
    }

    close(reason: CommandNativeClosure = 'checkpoint'): void {
        if (this.#closed !== undefined) return;
        this.#closed = reason;
        this.#changed();
    }

    retain(key: string, count = 1): void {
        if (key.length > 2048 || !Number.isSafeInteger(count) || count < 1 || this.#resources.size >= MAX_RESOURCES) {
            this.invalidate();
            return;
        }
        // Reusing a live resource identity is uncertain, never an overwrite.
        if (this.#resources.has(key)) { this.invalidate(); return; }
        this.#resources.set(key, count);
        this.#changed();
    }

    release(key: string, count?: number): void {
        const current = this.#resources.get(key);
        if (current === undefined) return;
        if (count !== undefined && (!Number.isSafeInteger(count) || count < 0 || count > current)) {
            this.invalidate(); return;
        }
        if (count === undefined || current === count) this.#resources.delete(key);
        else this.#resources.set(key, current - count);
        this.#changed();
    }

    touch(): void { this.#changed(); }

    invalidate(): void { this.#failed = true; this.#changed(); }

    async drain(timeoutMs = 5000): Promise<CommandNativeDrain> {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) throw new RangeError('Invalid command drain timeout');
        if (this.#closed === undefined) throw new CommandNativeAdmissionError('cem.command.checkpoint.open', 'Close managed admission before draining');
        if (this.#waiters.size >= MAX_WAITERS) throw new CommandNativeAdmissionError('cem.command.checkpoint.limit', 'Too many drain waiters');
        if (this.#pending !== 0 && !this.#failed) await new Promise<void>((resolve) => {
            const finish = () => {
                if (this.#pending !== 0 && !this.#failed) return;
                clearTimeout(timer);
                this.#waiters.delete(finish);
                resolve();
            };
            const timer = setTimeout(() => this.invalidate(), timeoutMs);
            this.#waiters.add(finish);
            finish();
        });
        if (this.#failed) throw new CommandNativeAdmissionError('cem.command.checkpoint.uncertain', 'Managed native drain timed out or ownership is uncertain');
        const proof: CommandNativeDrain = Object.freeze({ revision: this.#revision, scope: 'managed-cli-only', replacementAllowed: false });
        proofs.set(proof, { domain: this, revision: this.#revision });
        return proof;
    }

    current(proof: CommandNativeDrain): boolean {
        const stored = proofs.get(proof);
        return stored?.domain === this && stored.revision === this.#revision && this.#closed !== undefined && this.#pending === 0 && !this.#failed;
    }

    checkpoint(proof: CommandNativeDrain): CommandNativeDrain {
        if (!this.current(proof) || this.#resources.size !== 0) {
            throw new CommandNativeAdmissionError('cem.command.checkpoint.blocked', 'Pending, retained, stale or unmanaged command state cannot be checkpointed');
        }
        return proof;
    }

    #changed(): void {
        this.#revision++;
        if (this.#closed !== undefined && this.#pending !== 0 && !this.#failed) {
            this.#timer ??= setTimeout(() => this.invalidate(), 5000);
        } else {
            clearTimeout(this.#timer);
            this.#timer = undefined;
        }
        for (const waiter of [...this.#waiters]) waiter();
    }
}

/** Host-side witness tied to the owning transport object and current activity. */
export class CommandNativeClientCheckpoint {
    #closed = false;
    #uncertain = false;
    #proof: CommandNativeDrain | undefined;
    #revision = 0;
    #callbacks = 0;
    #waiters = 0;
    #idle: () => boolean;
    #owners: CommandNativeEmbeddingOwners;
    #ownershipAck: object | undefined;

    constructor(idle: () => boolean, identity: object = {}) {
        this.#idle = idle;
        this.#owners = new CommandNativeEmbeddingOwners(identity, () => identity, () => this.changed());
    }
    claimEmbedding(): CommandNativeEmbeddingOwnership { return this.#owners.claim(); }
    releaseEmbedding(lease: CommandNativeEmbeddingOwnership): void { this.#owners.release(lease); }
    changed(): void { this.#revision++; this.#proof = undefined; }
    close(): void { this.#closed = true; this.changed(); }
    invalidate(): void { this.#uncertain = true; this.#owners.revoke(); this.close(); }
    assertExecution(): void { if (this.#closed) throw new CommandNativeAdmissionError(); }
    callback(): () => void {
        this.#callbacks++; this.changed();
        return () => { this.#callbacks--; this.changed(); };
    }
    current(proof: CommandNativeDrain): boolean {
        return this.#proof !== undefined && proof === this.#proof && !this.#uncertain && this.#closed && this.#callbacks === 0 && this.#idle() && (this.#ownershipAck === undefined || this.#owners.current(this.#ownershipAck));
    }
    async capture(send: () => Promise<unknown>, timeoutMs = 5000, ownership?: readonly CommandNativeEmbeddingOwnership[]): Promise<CommandNativeDrain> {
        if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) throw new RangeError('Invalid command drain timeout');
        if (this.#uncertain) throw new CommandNativeAdmissionError('cem.command.checkpoint.blocked', 'Command checkpoint ownership is uncertain');
        if (this.#waiters >= MAX_WAITERS) throw new CommandNativeAdmissionError('cem.command.checkpoint.limit', 'Too many checkpoint requests');
        const ownershipSnapshot = ownership === undefined ? undefined : copyCommandNativeEmbeddingOwnership(ownership);
        const ownershipAck = ownershipSnapshot === undefined ? undefined : this.#owners.acknowledge(ownershipSnapshot);
        this.close();
        this.#waiters++;
        let timer: ReturnType<typeof setTimeout> | undefined;
        try {
            const value = await Promise.race([send(), new Promise<never>((_resolve, reject) => {
                timer = setTimeout(() => {
                    this.invalidate();
                    reject(new CommandNativeAdmissionError('cem.command.checkpoint.timeout', 'Command checkpoint acknowledgement timed out'));
                }, Math.min(timeoutMs, 5000));
            })]);
            const reply = value as Partial<CommandNativeDrain> | null;
            if (this.#uncertain || this.#callbacks !== 0 || !this.#idle() || (ownershipAck !== undefined && !this.#owners.current(ownershipAck)) || reply?.scope !== 'managed-cli-only' || reply.replacementAllowed !== false || !Number.isSafeInteger(reply.revision) || (reply.revision as number) < 0 || Object.keys(reply).length !== (ownershipAck === undefined ? 3 : 4)) {
                throw new CommandNativeAdmissionError('cem.command.checkpoint.blocked', 'Command checkpoint acknowledgement is incomplete');
            }
            // Never trust a transferable/copyable worker receipt as authority.
            // Only this original object is current for this client and activity.
            const nativeInventory = ownershipAck === undefined ? undefined : parseCommandNativeInventory(reply.nativeInventory);
            if (nativeInventory !== undefined && nativeInventory.blockingResources !== 0) throw new CommandNativeAdmissionError('cem.command.inventory.blocked', 'Native resources are still retained');
            const revision = this.#revision;
            this.#ownershipAck = ownershipAck;
            this.#proof = Object.freeze({ revision, scope: 'managed-cli-only', replacementAllowed: false,
                ...(nativeInventory === undefined ? {} : { nativeInventory, embeddingOwners: ownershipSnapshot?.length ?? 0 }) });
            return this.#proof;
        } finally { clearTimeout(timer); this.#waiters--; }
    }
}
