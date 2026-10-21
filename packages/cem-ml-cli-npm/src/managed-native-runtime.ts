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
