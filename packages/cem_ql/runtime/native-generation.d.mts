/** Original host capability, never serialized or reconstructed from metadata. */
export interface CemNativeGenerationParticipant { readonly kind: 'cem-native-generation-participant' }
export interface CemNativeGenerationRetirement { readonly checkpoint: Uint8Array; readonly replacementAllowed: false }
export interface CemNativeGenerationCandidate {
    readonly identity: object;
    readonly runtimeId: string;
    readonly runtime: object;
    /** Original bound controls: neither uses the candidate object as receiver. */
    readonly current: () => boolean;
    readonly discard: () => void;
}
export type CemNativeGenerationAllocator = (admitted: () => boolean, signal: AbortSignal) => Promise<CemNativeGenerationCandidate>;
/** @internal Shared runtime adapters only. */
export declare function registerNativeGeneration(identity: object, runtimeId: string, allocate: CemNativeGenerationAllocator): CemNativeGenerationParticipant;
/** @internal Called after original complete ownership and native retirement. */
export declare function completeNativeRetirement(participant: CemNativeGenerationParticipant, bytes: Uint8Array): CemNativeGenerationRetirement;
export interface CemNativeGenerationAdmission {
    readonly generations: readonly Readonly<{ previousRuntimeId: string; runtimeId: string; runtime: object; checkpoint: Uint8Array }>[];
    /** Fence the entire admitted set before attempting each physical cleanup. */
    revoke(): void;
}
export declare class CemNativeGenerationGroup {
    #private;
    constructor(participants: readonly CemNativeGenerationParticipant[]);
    allocate(retirements: readonly CemNativeGenerationRetirement[], timeoutMs?: number): Promise<CemNativeGenerationAdmission>;
}

/** Original host membership. Preparation never closes or retires a heap. */
export interface CemNativeRecoveryEnrollment { remove(): void; changed(): void }
export interface CemNativeRecoveryEndpoint {
    readonly id: string;
    readonly kind: 'rendering' | 'live-preview' | 'command-service' | 'command-invocation';
    /** Return exact original participants and a host witness surviving retirement.
     * Busy, unknown or unsupported members must reject; never omit a heap. */
    readonly prepare: (signal: AbortSignal) => Promise<Readonly<{
        participants: readonly CemNativeGenerationParticipant[];
        current: () => boolean;
    }>>;
}
export declare class CemNativeRecoveryScope {
    #private;
    enroll(endpoint: CemNativeRecoveryEndpoint): CemNativeRecoveryEnrollment;
    inspect(): Readonly<{ disposed: boolean; preparing: boolean;
        members: readonly Readonly<Pick<CemNativeRecoveryEndpoint, 'id' | 'kind'>>[] }>;
    prepare(timeoutMs?: number): Promise<Readonly<{
        current(): boolean;
        createGenerationGroup(): CemNativeGenerationGroup;
    }>>;
    dispose(): void;
}

/** @internal One original live admission can authorize one complete resume attempt.
 * Snapshot bytes come from the private successful retirement, not public exports. */
export declare function claimNativeGenerationAdmission(admission: CemNativeGenerationAdmission): Readonly<{
    current(): boolean;
    revoke(): void;
    onRevoke(listener: () => void): void;
    generations: CemNativeGenerationAdmission['generations'];
}>;
