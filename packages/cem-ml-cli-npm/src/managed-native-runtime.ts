import { parseCommandNativeClosure, copyCommandNativeEmbeddingOwnership, CommandNativeEmbeddingOwners, parseCommandNativeInventory, type CommandNativeInventory, type CommandNativeEmbeddingOwnership } from './native-embedding.js';
import type { CommandNativeDrain } from './native-admission.js';
import * as native from '@epa-wg/cem-ml/wasm';
import { CommandNativeAdmission, CommandNativeAdmissionError } from './native-admission.js';

// The generated status function closes over one module singleton. Sharing this
// exact function across CLI adapters shares the ledger, including duplicate CLI
// module evaluations. Raw initSync/generated exports are an unmanaged boundary.
const registryKey = Symbol.for('cem.command.native-admission.v1');
const realm = globalThis as typeof globalThis & { [registryKey]?: WeakMap<object, CommandNativeAdmission> };
const registry = realm[registryKey] ??= new WeakMap<object, CommandNativeAdmission>();
const identity = native.runtimeMemoryStatus;
export const commandNativeAdmission = registry.get(identity) ?? new CommandNativeAdmission(() => native.runtimeMemoryStatus());
registry.set(identity, commandNativeAdmission);

const cleanup = new Set([
    'cancelCommandServiceV1', 'disposeCommandArtifactV1', 'disposeCommandArtifactsV1',
    'disposeResumableOperationHost', 'pollResumableOperation', 'acceptResumableOperationResult',
    'cancelResumableOperation', 'pauseResumableOperation', 'acknowledgeResumableOperationStop',
]);
const ordinary = new Set([
    'authorTransformConfigV1', 'buildCommandInvocationV1', 'projectCommandPresentationV1',
    'executeCommandServiceV1', 'readCommandArtifactV1', 'initializeResumableOperationHost',
    'startResumableOperation', 'replaceResumableOperationWorker', 'executeOperationWork',
    'continueResumableOperation', 'stepResumableOperation',
    'version', 'workerProtocolDescriptor', 'nodeWorkerCapabilityManifest',
    'browserWorkerCapabilityManifest', 'capabilityManifest',
]);
const wrappers = new Map<PropertyKey, unknown>();

/** All shipped CLI native entrypoints pass through this bounded adapter. */
export const commandNativeRuntime: typeof native = new Proxy(Object.create(null) as typeof native, {
    get(_target, key) {
        if (wrappers.has(key)) return wrappers.get(key);
        const fn = Reflect.get(native, key) as unknown;
        // Browser initialization does no command work. Do not expose initSync or
        // another arbitrary generated export through the managed namespace.
        if (key === 'default') return typeof fn === 'function' ? fn : undefined;
        if (typeof key !== 'string' || (!cleanup.has(key) && !ordinary.has(key))) {
            throw new CommandNativeAdmissionError('cem.command.native.unmanaged', 'Generated export has no managed CLI contract');
        }
        if (typeof fn !== 'function') {
            commandNativeAdmission.invalidate();
            throw new CommandNativeAdmissionError('cem.command.native.unavailable', 'Managed native method is unavailable');
        }
        const method = key;
        const wrapped = (...args: unknown[]): unknown => {
            const release = commandNativeAdmission.admit(cleanup.has(method));
            const complete = (value: unknown) => {
                try { trackResources(method, args, value); return value; }
                catch (error) { commandNativeAdmission.invalidate(); throw error; }
                finally { release(); }
            };
            try {
                const value: unknown = Reflect.apply(fn, native, args);
                if (value instanceof Promise) return value.then(complete, (error: unknown) => {
                    commandNativeAdmission.invalidate(); release(); throw error;
                });
                return complete(value);
            } catch (error) {
                commandNativeAdmission.invalidate(); release(); throw error;
            }
        };
        wrappers.set(key, wrapped);
        return wrapped;
    },
});

function trackResources(method: string, args: unknown[], value: unknown): void {
    if (!['executeCommandServiceV1', 'initializeResumableOperationHost', 'disposeResumableOperationHost',
        'disposeCommandArtifactV1', 'disposeCommandArtifactsV1'].includes(method)) return;
    // Only explicit Rust-owned API/control DTOs are inspected here, never CEM AST.
    const result = JSON.parse(value as string) as Record<string, unknown>;
    if (result.error !== undefined) { commandNativeAdmission.invalidate(); return; }
    if (method === 'executeCommandServiceV1') {
        const artifacts = result.artifacts as { originalCount: number };
        if (!Number.isSafeInteger(artifacts?.originalCount) || artifacts.originalCount < 0 || typeof result.requestId !== 'string') throw new Error('Invalid command artifact control DTO');
        if (artifacts.originalCount !== 0) commandNativeAdmission.retain(`request:${result.requestId}`, artifacts.originalCount);
    } else if (method === 'initializeResumableOperationHost') {
        if (!Number.isSafeInteger(result.hostId) || (result.hostId as number) < 1) throw new Error('Invalid native host control DTO');
        commandNativeAdmission.retain(`host:${result.hostId}`);
    } else if (method === 'disposeResumableOperationHost') {
        commandNativeAdmission.release(`host:${args[0]}`);
    } else {
        if (result.protocolVersion !== 1 || result.requestId !== args[0] ||
            !Number.isSafeInteger(result.disposedCount) || (result.disposedCount as number) < 0 ||
            !['disposed', 'already-disposed'].includes(result.disposition as string)) {
            throw new Error('Invalid native artifact-disposal control DTO');
        }
        if (method === 'disposeCommandArtifactsV1') {
            if (result.handleId !== null && result.handleId !== undefined) throw new Error('Invalid request-disposal control DTO');
            commandNativeAdmission.release(`request:${args[0]}`);
        } else {
            if (result.handleId !== args[1] || (result.disposedCount as number) > 1) throw new Error('Invalid handle-disposal control DTO');
            commandNativeAdmission.release(`request:${args[0]}`, result.disposedCount as number);
        }
    }
}

/** Census is a bounded Rust control DTO, independent of the managed CLI ledger. */
export function readCommandNativeInventory(): CommandNativeInventory {
    try { return parseCommandNativeInventory(native.commandNativeResourceInventory()); }
    catch (error) {
        commandNativeAdmission.invalidate();
        throw new CommandNativeAdmissionError('cem.command.inventory.uncertain', error instanceof Error ? error.message : 'Native census failed');
    }
}

const ownerRegistryKey = Symbol.for('cem.command.native-embedding-owners.v1');
const ownerRealm = globalThis as typeof globalThis & { [ownerRegistryKey]?: WeakMap<object, CommandNativeEmbeddingOwners> };
const ownerRegistry = ownerRealm[ownerRegistryKey] ??= new WeakMap<object, CommandNativeEmbeddingOwners>();
const ownedProofs = new WeakMap<object, { readonly owners: CommandNativeEmbeddingOwners; readonly ack: object;
    readonly drain: CommandNativeDrain; readonly nativeRevision: number; readonly coordinated: boolean }>();

function mainEmbeddingOwners(): CommandNativeEmbeddingOwners {
    const memory = native.commandNativeMemoryIdentity() as object;
    if (!(memory instanceof WebAssembly.Memory)) throw new CommandNativeAdmissionError('cem.command.embedding.generation');
    let owners = ownerRegistry.get(memory);
    if (owners === undefined) {
        owners = new CommandNativeEmbeddingOwners(memory, () => native.commandNativeMemoryIdentity() as object,
            () => commandNativeAdmission.touch());
        ownerRegistry.set(memory, owners);
    }
    return owners;
}

export function claimCommandNativeEmbedding(): CommandNativeEmbeddingOwnership { return mainEmbeddingOwners().claim(); }
export function releaseCommandNativeEmbedding(lease: CommandNativeEmbeddingOwnership): void { mainEmbeddingOwners().release(lease); }

export async function checkpointCommandNativeEmbedding(
    leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000, coordination?: 'close' | CommandNativeDrain,
): Promise<CommandNativeDrain> {
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) throw new RangeError('Invalid command drain timeout');
    const owners = mainEmbeddingOwners();
    const previous = coordination === undefined || coordination === 'close' ? undefined : ownedProofs.get(coordination);
    if (coordination !== undefined && coordination !== 'close' && (previous === undefined || !previous.coordinated || previous.owners !== owners ||
        !owners.current(previous.ack) || !commandNativeAdmission.current(previous.drain))) throw new CommandNativeAdmissionError('cem.command.native.stale');
    const snapshot = copyCommandNativeEmbeddingOwnership(leases);
    const ack = owners.acknowledge(snapshot);
    commandNativeAdmission.close();
    if (coordination === 'close') closeCommandNativeAdmission();
    const drain = commandNativeAdmission.checkpoint(await commandNativeAdmission.drain(timeoutMs));
    const validated = coordination === undefined ? { nativeInventory: readCommandNativeInventory() } : validateCommandNativeClosure(previous?.nativeRevision);
    const inventory = validated.nativeInventory;
    if (inventory.blockingResources !== 0 || !owners.current(ack) || !commandNativeAdmission.current(drain)) {
        throw new CommandNativeAdmissionError('cem.command.inventory.blocked', 'Native retained resources or embedding ownership still block the census acknowledgement');
    }
    const proof: CommandNativeDrain = Object.freeze({ ...drain, ...validated, embeddingOwners: snapshot.length });
    ownedProofs.set(proof, { owners, ack, drain, nativeRevision: inventory.revision, coordinated: coordination !== undefined });
    return proof;
}

export function isCommandNativeInventoryCurrent(proof: CommandNativeDrain): boolean {
    const stored = ownedProofs.get(proof);
    if (stored === undefined || !stored.owners.current(stored.ack) || !commandNativeAdmission.current(stored.drain)) return false;
    try {
        const inventory = stored.coordinated ? validateCommandNativeClosure(stored.nativeRevision).nativeInventory : readCommandNativeInventory();
        return inventory.revision === stored.nativeRevision && inventory.blockingResources === 0 && stored.owners.current(stored.ack) && commandNativeAdmission.current(stored.drain);
    }
    catch { return false; }
}


/** Close ordinary raw exports in this linked memory before waiting for managed work. */
export function closeCommandNativeAdmission(): void {
    try { native.closeCommandNativeAdmission(); }
    catch { commandNativeAdmission.invalidate(); throw new CommandNativeAdmissionError('cem.command.native.uncertain'); }
}

/** No asynchronous boundary or JavaScript callback occurs inside native validation. */
export function validateCommandNativeClosure(expectedRevision?: number): Pick<CommandNativeDrain, 'nativeInventory' | 'nativeClosure'> & { nativeInventory: CommandNativeInventory } {
    const inventory = readCommandNativeInventory();
    if (inventory.blockingResources !== 0) throw new CommandNativeAdmissionError('cem.command.inventory.blocked');
    const expected = expectedRevision ?? inventory.revision;
    try {
        const closure = parseCommandNativeClosure(native.validateCommandNativeClosure(expected), expected);
        if (inventory.revision !== expected) throw new Error('stale');
        return { nativeInventory: inventory, nativeClosure: closure };
    } catch { throw new CommandNativeAdmissionError('cem.command.native.stale', 'Native admission or inventory changed before validation'); }
}

export function closeCommandNativeEmbeddingAdmission(leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000): Promise<CommandNativeDrain> {
    return checkpointCommandNativeEmbedding(leases, timeoutMs, 'close');
}
export function revalidateCommandNativeEmbedding(proof: CommandNativeDrain, leases: readonly CommandNativeEmbeddingOwnership[], timeoutMs = 5000): Promise<CommandNativeDrain> {
    if (proof === undefined || proof === null) return Promise.reject(new CommandNativeAdmissionError('cem.command.native.stale'));
    return checkpointCommandNativeEmbedding(leases, timeoutMs, proof);
}

// The original generated object owns a non-cloneable native RAII permit. Its
// scalar confirmation never allows another alias to release the barrier.
let retirement: { id: number; permit: native.CommandNativeRetirement } | undefined;
export function prepareCommandNativeRetirement(revision: number, id: number): unknown {
    if (retirement !== undefined) throw new CommandNativeAdmissionError('cem.command.retirement.pending');
    try {
        const permit = native.prepareCommandNativeRetirement(revision);
        retirement = { id, permit };
        return JSON.parse(permit.confirmation()) as unknown;
    } catch { throw new CommandNativeAdmissionError('cem.command.retirement.stale'); }
}
export function abortCommandNativeRetirement(id: number): void {
    if (retirement === undefined) return;
    if (retirement.id !== id) throw new CommandNativeAdmissionError('cem.command.retirement.stale');
    retirement.permit.free(); retirement = undefined;
}
