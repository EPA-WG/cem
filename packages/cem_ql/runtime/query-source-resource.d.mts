import type { CemNativeGenerationParticipant, CemNativeGenerationBindingContext } from './native-generation.mjs';
/** Internal original owner; copied objects and wire IDs confer no authority. */
export interface QuerySourceOwner { readonly kind: 'cem-query-source-owner' }
export interface QuerySourceEndpoint {
    readonly participant: CemNativeGenerationParticipant;
    identity(): object | undefined;
    current(): boolean;
    open(bytes: Uint8Array, pins: string, signal?: AbortSignal): Promise<QuerySourceOwner>;
    capture(owner: QuerySourceOwner, signal?: AbortSignal): Promise<Uint8Array>;
    release(owner: QuerySourceOwner, signal?: AbortSignal): Promise<void>;
    dispose(): void;
}
export interface QuerySourceNativeBackend {
    open(bytes: Uint8Array, pins: string): unknown | Promise<unknown>;
    capture(owner: unknown): Uint8Array | Promise<Uint8Array>;
    release(owner: unknown): unknown | Promise<unknown>;
}
/** Requires prior trusted native provisioning; tokens stay in the original heap. */
export declare function createQuerySourceWasmBackend(runtime: {
    memory(): WebAssembly.Memory | undefined;
    call(name: string, args: unknown[]): unknown;
}): QuerySourceNativeBackend;
export interface QuerySourceEndpointOptions {
    participant: CemNativeGenerationParticipant;
    /** Actual owned Memory/Worker, already registered by its shared adapter. */
    identity: object;
    current(): boolean;
    timeoutMs?: number;
    /** Original owner cleanup only, still available behind closed admission. */
    cleanupCurrent?(): boolean;
    native: QuerySourceNativeBackend;
    close?(): void;
}
export declare function createQuerySourceEndpoint(options: QuerySourceEndpointOptions): QuerySourceEndpoint;
export declare function createQuerySourceWorkerEndpoint(options: Omit<QuerySourceEndpointOptions, 'native' | 'close'> & { port: MessagePort }): QuerySourceEndpoint;
export declare function installQuerySourceWorkerTransport(port: MessagePort, native: QuerySourceNativeBackend): Readonly<{ dispose(): Promise<void> }>;
export interface QuerySourceResourceOptions {
    endpoint: QuerySourceEndpoint;
    bytes: Uint8Array;
    pins: string;
    /** Recovery policy, including entered credentials; never authored metadata. */
    current(): boolean;
    target(runtime: CemNativeGenerationBindingContext['generation']['runtime']): QuerySourceEndpoint;
    peers?(): readonly CemNativeGenerationParticipant[];
    onBound?(endpoint: QuerySourceEndpoint): void;
}
export declare class QuerySourceResource {
    #private;
    static open(options: QuerySourceResourceOptions, signal?: AbortSignal): Promise<QuerySourceResource>;
    prepareRecovery(signal?: AbortSignal): Promise<void>;
    dispose(): Promise<void>;
}
