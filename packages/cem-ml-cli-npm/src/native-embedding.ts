/** Explicit control metadata; never a native AST, handle or replacement permit. */
export interface CommandNativeInventory {
    readonly schemaVersion: 1;
    readonly profile: 'cem-ml-cem-ql-command-v1';
    readonly revision: number;
    readonly complete: true;
    readonly blockingResources: number;
    readonly registries: readonly { readonly name: string; readonly live: number; readonly discardable: number }[];
    readonly replacementAllowed: false;
}

/** Native confirmation at one closed, idle inventory revision; never a replacement permit. */
export interface CommandNativeClosureValidation {
    readonly schemaVersion: 1;
    readonly closed: true;
    readonly inventoryRevision: number;
    readonly replacementAllowed: false;
}

export function parseCommandNativeClosure(value: unknown, revision: number): CommandNativeClosureValidation {
    if (typeof value === 'string') {
        if (value.length > 512) throw new CommandNativeEmbeddingError('cem.command.native.limit');
        value = JSON.parse(value) as unknown;
    }
    if (!record(value) || Object.keys(value).length !== 4 || value.schemaVersion !== 1 ||
        value.closed !== true || value.replacementAllowed !== false || !counter(value.inventoryRevision) || value.inventoryRevision < 1 ||
        value.inventoryRevision !== revision) throw new CommandNativeEmbeddingError('cem.command.native.stale');
    return Object.freeze({ schemaVersion: 1, closed: true, inventoryRevision: revision, replacementAllowed: false });
}

export interface CommandNativeEmbeddingOwnership {
    readonly kind: 'command-native-embedding';
}

export class CommandNativeEmbeddingError extends Error {
    readonly code: string;
    constructor(code = 'cem.command.embedding.invalid', message = 'Original, complete native embedding ownership is required') {
        super(message); this.name = 'CommandNativeEmbeddingError'; this.code = code;
    }
}

/** A bounded native census received from the owning WASM instance/worker only. */
export function parseCommandNativeInventory(value: unknown): CommandNativeInventory {
    if (typeof value === 'string') {
        if (value.length > 16384) throw new CommandNativeEmbeddingError('cem.command.inventory.limit');
        value = JSON.parse(value) as unknown;
    }
    if (!record(value) || Object.keys(value).length !== 7 || value.schemaVersion !== 1 ||
        value.profile !== 'cem-ml-cem-ql-command-v1' || value.complete !== true || value.replacementAllowed !== false ||
        !counter(value.revision) || !counter(value.blockingResources) || !Array.isArray(value.registries) ||
        value.registries.length < 1 || value.registries.length > 64) throw new CommandNativeEmbeddingError('cem.command.inventory.invalid');
    const names = new Set<string>();
    let live = 0;
    const registries = value.registries.map((entry: unknown) => {
        if (!record(entry) || Object.keys(entry).length !== 3 || typeof entry.name !== 'string' ||
            !/^[a-z-]{1,64}$/.test(entry.name) || names.has(entry.name) || !counter(entry.live) || !counter(entry.discardable)) {
            throw new CommandNativeEmbeddingError('cem.command.inventory.invalid');
        }
        names.add(entry.name); live += entry.live;
        return Object.freeze({ name: entry.name, live: entry.live, discardable: entry.discardable });
    });
    if (!counter(live) || live !== value.blockingResources) throw new CommandNativeEmbeddingError('cem.command.inventory.invalid');
    return Object.freeze({ schemaVersion: 1, profile: 'cem-ml-cem-ql-command-v1', revision: value.revision,
        complete: true, blockingResources: live, registries: Object.freeze(registries), replacementAllowed: false });
}

/** Bound the caller-controlled list before allocating an asynchronous snapshot. */
export function copyCommandNativeEmbeddingOwnership(leases: readonly CommandNativeEmbeddingOwnership[]): readonly CommandNativeEmbeddingOwnership[] {
    if (!Array.isArray(leases) || leases.length > 256) throw new CommandNativeEmbeddingError('cem.command.embedding.limit');
    return Object.freeze([...leases]);
}

/** Original leases acknowledge this heap only. A flag/label/copy is not ownership. */
export class CommandNativeEmbeddingOwners {
    #identity: object | undefined;
    #actual: () => object | undefined;
    #changed: () => void;
    #owners = new Set<CommandNativeEmbeddingOwnership>();
    #revision = 0;
    #acks = new WeakMap<object, number>();

    constructor(identity: object, actual: () => object | undefined, changed: () => void) {
        this.#identity = identity; this.#actual = actual; this.#changed = changed;
    }
    claim(): CommandNativeEmbeddingOwnership {
        this.#requireIdentity();
        if (this.#owners.size >= 256) throw new CommandNativeEmbeddingError('cem.command.embedding.limit');
        const lease = Object.freeze({ kind: 'command-native-embedding' as const });
        this.#owners.add(lease); this.#mutation();
        return lease;
    }
    release(lease: CommandNativeEmbeddingOwnership): void {
        this.#requireIdentity();
        if (!this.#owners.delete(lease)) throw new CommandNativeEmbeddingError();
        this.#mutation();
    }
    acknowledge(leases: readonly CommandNativeEmbeddingOwnership[]): object {
        this.#requireIdentity();
        if (!Array.isArray(leases) || leases.length < 1 || leases.length !== this.#owners.size ||
            new Set(leases).size !== leases.length || leases.some(lease => !this.#owners.has(lease))) throw new CommandNativeEmbeddingError();
        const ack = Object.freeze({ owners: leases.length });
        this.#acks.set(ack, this.#revision);
        return ack;
    }
    current(ack: object): boolean {
        try { this.#requireIdentity(); } catch { return false; }
        return this.#acks.has(ack) && this.#acks.get(ack) === this.#revision;
    }
    revoke(): void {
        this.#identity = undefined; this.#actual = () => undefined;
        this.#owners.clear(); this.#mutation();
    }
    #mutation(): void { this.#revision++; this.#changed(); }
    #requireIdentity(): void {
        if (this.#identity === undefined || this.#actual() !== this.#identity) throw new CommandNativeEmbeddingError('cem.command.embedding.generation');
    }
}

function record(value: unknown): value is Record<string, unknown> { return typeof value === 'object' && value !== null && !Array.isArray(value); }
function counter(value: unknown): value is number { return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0; }
