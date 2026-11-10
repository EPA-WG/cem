import type { CemNativeGenerationParticipant, CemNativeGenerationBindingContext } from './native-generation.mjs';
/** Internal original owner; copied objects and wire IDs confer no authority. */
export interface QuerySourceOwner { readonly kind: 'cem-query-source-owner' }
export interface QuerySourceOwnedReceipt {
    readonly receipt: string;
    /** Aborts when the original native owner releases or the endpoint closes. */
    readonly signal: AbortSignal;
    /** Synchronous guard for staging the original receipt. */
    isCurrent(): boolean;
}
export type QuerySourceStylesheetReceipt = QuerySourceOwnedReceipt;
export interface ApplicationBrowserIslandInput { tag: string; bytes: Uint8Array; instance: string }
export interface QuerySourceEndpoint {
    readonly participant: CemNativeGenerationParticipant;
    identity(): object | undefined;
    current(): boolean;
    open(bytes: Uint8Array, pins: string, signal?: AbortSignal): Promise<QuerySourceOwner>;
    capture(owner: QuerySourceOwner, signal?: AbortSignal): Promise<Uint8Array>;
    stylesheets(owner: QuerySourceOwner, tag: string): Promise<QuerySourceStylesheetReceipt>;
    renderBrowserIsland(owner: QuerySourceOwner, input: ApplicationBrowserIslandInput): Promise<QuerySourceOwnedReceipt>;
    release(owner: QuerySourceOwner, signal?: AbortSignal): Promise<void>;
    dispose(): void;
}
export interface QuerySourceNativeBackend {
    readonly maxSourceBytes?: number;
    open(bytes: Uint8Array, pins: string): unknown | Promise<unknown>;
    capture(owner: unknown): Uint8Array | Promise<Uint8Array>;
    release(owner: unknown): unknown | Promise<unknown>;
    /** Application-only, bounded receipt from the original publication. */
    stylesheets?(owner: unknown, tag: string): string | Promise<string>;
    renderBrowserIsland?(owner: unknown, input: ApplicationBrowserIslandInput): string | Promise<string>;
    /** Original Rust embedding routes on the exact native heap. */
    receiver?: {
        capture(owner: unknown): { bytes: Uint8Array; revision: string } | Promise<{ bytes: Uint8Array; revision: string }>;
        /** Confirm the current native seal and release the same owner atomically. */
        retire(owner: unknown, bytes: Uint8Array, journal: Uint8Array): string | Promise<string>;
        open(bytes: Uint8Array, pins: string, recovery: { bytes: Uint8Array; pin: string; journal: Uint8Array; runtimeId: string }): unknown | Promise<unknown>;
    };
}
/** Requires prior trusted native provisioning; tokens stay in the original heap. */
export declare function createQuerySourceWasmBackend(runtime: {
    memory(): WebAssembly.Memory | undefined;
    call(name: string, args: unknown[]): unknown;
}): QuerySourceNativeBackend;
export declare function createApplicationSourceWasmBackend(runtime: {
    memory(): WebAssembly.Memory | undefined;
    call(name: string, args: unknown[]): unknown;
}): QuerySourceNativeBackend;
export declare function createRuleSourceWasmBackend(runtime: {
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
    maxSourceBytes?: number;
    native: QuerySourceNativeBackend;
    close?(): void;
}
export declare function createQuerySourceEndpoint(options: QuerySourceEndpointOptions): QuerySourceEndpoint;
export declare function createQuerySourceWorkerEndpoint(options: Omit<QuerySourceEndpointOptions, 'native' | 'close'> & {
    port: MessagePort;
    receiverRecovery?: boolean;
    applicationStylesheets?: boolean;
    applicationBrowserFrames?: boolean;
}): QuerySourceEndpoint;
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
    /** Retain the current trusted native-sealed journal for receiver retirement. */
    receiverCheckpoint?(): Uint8Array;
}
export declare class QuerySourceResource {
    #private;
    static open(options: QuerySourceResourceOptions, signal?: AbortSignal): Promise<QuerySourceResource>;
    stylesheets(tag: string): Promise<QuerySourceStylesheetReceipt>;
    renderBrowserIsland(input: ApplicationBrowserIslandInput): Promise<QuerySourceOwnedReceipt>;
    /** Receiver recovery returns the original native capsule revision. */
    prepareRecovery(signal?: AbortSignal): Promise<string | undefined>;
    dispose(): Promise<void>;
}
