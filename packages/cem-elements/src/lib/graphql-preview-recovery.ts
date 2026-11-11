import type { CemGraphqlGuard, CemGraphqlQueryOptions } from './graphql-actions.js';
import type { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { assertNativeGenerationResourceBudget, assertNativeGenerationResourceTarget,
    completeNativeRetirement, issueNativeGenerationBindingTicket, registerNativeGeneration,
    registerNativeGenerationResource, releaseNativeGenerationResources,
    type CemNativeGenerationBindingTicket, type CemNativeGenerationParticipant,
    type CemNativeGenerationRetirement, type CemNativeRecoveryScope } from './internal/runtime-support/native-generation.js';

export type CemGraphqlPreviewGuard = CemGraphqlGuard & CemGraphqlQueryOptions['guard'] & {
    applicationIdentity(): string;
    retainResponse(operation: string, bytes: Uint8Array, uri: string): string;
    claimMutation(operationId: string): number;
    captureResponse(): Uint8Array;
    responseRevision(): string;
    sealCheckpoint(candidate: string, receipt: string): string;
    confirmResponseRetirement(capsule: Uint8Array, journal: Uint8Array, runtimeId: string, instanceId: string): string;
    restoreResponse(capsule: Uint8Array, pin: string, journal: Uint8Array, previousRuntimeId: string): string;
    disposeResponse(): void;
    inventory(): string;
    memory(): WebAssembly.Memory;
    release(): void;
};
export interface CemGraphqlPreviewRecovery {
    /** Original host controls. Capture first through the complete recovery scope. */
    participant(): CemNativeGenerationParticipant;
    /** Called by the shared frame lifecycle after its original controls settle. */
    attachRenderingPeers(): void;
    bindingTicket(): CemNativeGenerationBindingTicket;
    retire(): CemNativeGenerationRetirement;
}
type Heap = { guard: CemGraphqlPreviewGuard; runtime: object; runtimeId: string; participant?: CemNativeGenerationParticipant;
    released: boolean; admitted: () => boolean };
function fail(): never { throw Error('cem.preview.application.recovery.stale'); }
const MAX_PIN_BYTES = 16 * 1024;
function ownerCensus(guard: CemGraphqlPreviewGuard): string {
    const census = JSON.parse(guard.inventory());
    if (census.contract !== 'cem-component-owner-inventory/1' || census.liveApplicationResponses !== 1
        || census.liveDeclarationOwners !== 0 || census.stylesheetConsumerLeases !== 0) fail();
    // Entry admission itself advances the native inventory revision. Compare
    // owner counts here; the exact response capsule fences source mutations.
    return JSON.stringify([census.liveApplicationResponses, census.liveDeclarationOwners,
        census.stylesheetConsumerLeases, census.applicationResponseBytes]);
}

/** Stable shared route to one actual selected heap. Neither a serialized pin
 * nor the public guard can allocate/rebind a replacement. Only the original
 * physical group delivers a fresh privately branded heap to the source route. */
export class CemGraphqlPreviewResponseOwner {
    private heap: Heap;
    private readonly heaps = new WeakMap<object, Heap>();
    private registration?: { remove(): void };
    private enrollment?: { remove(): void };
    private peerRegistrations: { remove(): void }[] = [];
    private snapshot?: { heap: Heap; capsule: Uint8Array; journal: Uint8Array; revision: string; inventory: string;
        journalId: string; current: () => boolean; peers: readonly CemNativeGenerationParticipant[];
        released: boolean; pin?: string; ticket?: CemNativeGenerationBindingTicket };
    private closed = false;
    private parked = false;
    private connected?: { journal: CemCheckpointPublicationCoordinator; rendering: CemCheckpointPublicationCoordinator;
        current: () => boolean };
    readonly guard: CemGraphqlPreviewGuard;

    constructor(original: CemGraphqlPreviewGuard, private readonly fresh: () => Promise<CemGraphqlPreviewGuard>) {
        this.heap = this.makeHeap(original, () => true);
        this.enrollHeap();
        const current = () => {
            if (this.closed || this.heap.released || !this.heap.admitted()) fail();
            return this.heap.guard;
        };
        const action = () => { if (this.parked) fail(); return current(); };
        // All calls follow the bound route. No raw predecessor heap escapes.
        this.guard = {
            applicationDocument: () => current().applicationDocument(),
            applicationIdentity: () => current().applicationIdentity(),
            validateVariables: (...args) => action().validateVariables(...args),
            validateQueryVariables: (...args) => action().validateQueryVariables(...args),
            renderResponse: (...args) => current().renderResponse(...args),
            retainResponse: (...args) => action().retainResponse(...args),
            claimMutation: id => action().claimMutation(id),
            captureResponse: () => current().captureResponse(),
            responseRevision: () => current().responseRevision(),
            sealCheckpoint: (...args) => current().sealCheckpoint(...args),
            confirmResponseRetirement: () => fail(), restoreResponse: () => fail(),
            inventory: () => current().inventory(), memory: () => current().memory(),
            disposeResponse: () => current().disposeResponse(), release: () => this.dispose(),
        };
    }

    private makeHeap(guard: CemGraphqlPreviewGuard, admitted: () => boolean): Heap {
        const heap = { guard, runtime: Object.freeze({ kind: 'cem-selected-application-generation' }),
            runtimeId: crypto.randomUUID(), released: false, admitted };
        this.heaps.set(heap.runtime, heap);
        return heap;
    }
    private disposeHeap(heap: Heap): void {
        if (heap.released) return;
        heap.guard.disposeResponse(); heap.guard.release(); heap.released = true;
    }
    private enrollHeap(): void {
        const heap = this.heap;
        heap.participant = registerNativeGeneration(heap.guard.memory(), heap.runtimeId, async (admitted, signal) => {
            if (this.closed || signal.aborted) fail();
            const next = this.makeHeap(await this.fresh(), admitted);
            if (this.closed || signal.aborted) { this.disposeHeap(next); fail(); }
            return { identity: next.guard.memory(), runtimeId: next.runtimeId, runtime: next.runtime,
                current: () => !this.closed && !next.released,
                discard: () => { this.disposeHeap(next); } };
        });
        const participant = heap.participant;
        this.registration = registerNativeGenerationResource(participant, () => {
            assertNativeGenerationResourceBudget(participant);
            const snapshot = this.snapshot;
            const current = () => !this.closed && !!snapshot && this.snapshot === snapshot && snapshot.heap === heap
                && snapshot.current() && (snapshot.released || !heap.released && heap.guard.responseRevision() === snapshot.revision
                    && ownerCensus(heap.guard) === snapshot.inventory);
            if (!snapshot || !current()) fail();
            return { current, peers: snapshot.peers, retainedBytes: snapshot.capsule.byteLength + MAX_PIN_BYTES,
                release: () => {
                    if (!current() || snapshot.released) fail();
                    snapshot.pin = heap.guard.confirmResponseRetirement(snapshot.capsule, snapshot.journal,
                        snapshot.journalId, 'application');
                    snapshot.released = true;
                },
                prepare: async context => {
                    if (!current() || !snapshot.released || context.signal.aborted
                        || context.generation.previousRuntimeId !== heap.runtimeId) fail();
                    const target = this.heaps.get(context.generation.runtime);
                    if (!target || target.released || target.participant
                        || target.runtimeId !== context.generation.runtimeId) fail();
                    // The logical journal survives physical heap replacement.
                    // The separate original group route checks physical IDs.
                    target.guard.restoreResponse(snapshot.capsule, snapshot.pin!, context.generation.checkpoint, snapshot.journalId);
                    let discarded = false;
                    const stagedCurrent = () => !discarded && !context.signal.aborted && !target.released && current();
                    const boundCurrent = () => !discarded && !this.closed && this.heap === target && !target.released
                        && target.admitted() && !context.signal.aborted && this.connected?.current() === true;
                    return { owner: this, current: stagedCurrent,
                        bind: () => {
                            if (!context.active() || !stagedCurrent()) fail();
                            this.registration?.remove(); this.heap = target; this.enrollHeap();
                            assertNativeGenerationResourceTarget(target.participant!, target.runtimeId, target.guard.memory());
                        },
                        bound: () => { this.clearSnapshot(); this.attachRenderingPeers(); }, boundCurrent,
                        discard: () => {
                            discarded = true; this.disposeHeap(target);
                            if (this.heap === target) this.dispose();
                        } };
                } };
        }, () => this.dispose());
    }

    connect(scope: CemNativeRecoveryScope, journal: CemCheckpointPublicationCoordinator,
        rendering: CemCheckpointPublicationCoordinator, current: () => boolean): CemGraphqlPreviewRecovery {
        if (this.closed || this.connected) fail();
        this.connected = { journal, rendering, current };
        this.enrollment = scope.enroll({ id: `cem-preview-application:${crypto.randomUUID()}`, kind: 'live-preview',
            prepare: async signal => {
                this.clearSnapshot();
                if (this.closed || !current() || signal.aborted) fail();
                if (journal.inspectRecovery().unknownOperations) throw Error('cem.checkpoint.recovery.unknown');
                const peer = rendering.recoveryEnrollment();
                const revision = this.guard.responseRevision();
                const inventory = ownerCensus(this.guard);
                await journal.capture();
                if (signal.aborted || this.closed || !current()) fail();
                const handoff = journal.handoff(), bytes = handoff.retain();
                const saved = journal.current();
                if (!saved) fail();
                // Inherited host privacy admission precedes copying any source
                // bytes out of the selected heap.
                if (this.guard.responseRevision() !== revision || ownerCensus(this.guard) !== inventory) fail();
                const capsule = this.guard.captureResponse();
                const snapshot = { heap: this.heap, capsule, journal: bytes, revision, inventory, journalId: saved.receipt.runtimeId,
                    peers: peer.participants, released: false,
                    current: () => !this.closed && current() && peer.current() && handoff.current() };
                this.snapshot = snapshot;
                return { participants: [this.heap.participant!], current: snapshot.current };
            } });
        return Object.freeze({
            participant: () => this.heap.participant!,
            attachRenderingPeers: () => this.attachRenderingPeers(),
            bindingTicket: () => {
                const snapshot = this.snapshot;
                if (!snapshot || !snapshot.current()) fail();
                // Like rendering resume tickets, this original host ticket
                // parks effects while the complete physical transaction runs.
                this.parked = true;
                return snapshot.ticket ??= issueNativeGenerationBindingTicket(snapshot.heap.participant!, snapshot.current, async context => {
                    if (!snapshot.current() || context.generation.previousRuntimeId !== snapshot.heap.runtimeId
                        || !this.heaps.has(context.generation.runtime)) fail();
                    let bound = false;
                    const target = this.heaps.get(context.generation.runtime)!;
                    return { owner: this, current: snapshot.current,
                        bind: () => { if (!context.active()) fail(); bound = true; },
                        boundCurrent: () => bound && this.heap === target && !this.closed,
                        discard: () => { if (this.heap === target) this.dispose(); } };
                });
            },
            retire: () => {
                const snapshot = this.snapshot, heap = this.heap;
                if (!snapshot || !snapshot.current() || snapshot.heap !== heap || heap.released) fail();
                assertNativeGenerationResourceBudget(heap.participant!);
                releaseNativeGenerationResources(heap.participant!, snapshot.current);
                heap.guard.release(); heap.released = true;
                return completeNativeRetirement(heap.participant!, snapshot.journal);
            },
        });
    }

    private attachRenderingPeers(): void {
        if (!this.connected || this.closed) fail();
        const peer = this.connected.rendering.generationParticipant();
        for (const registration of this.peerRegistrations.splice(0)) registration.remove();
        this.peerRegistrations.push(registerNativeGenerationResource(peer, () => {
            assertNativeGenerationResourceBudget(peer);
            const snapshot = this.snapshot;
            if (!snapshot || !snapshot.current() || !snapshot.peers.includes(peer)) fail();
            const current = () => !this.closed && this.snapshot === snapshot && snapshot.current();
            return { current, peers: [snapshot.heap.participant!], retainedBytes: 0,
                release: () => { if (!current()) fail(); },
                prepare: async context => ({ owner: this, current,
                    bind: () => { if (!context.active() || !current()) fail(); },
                    boundCurrent: () => !this.closed && this.heap.admitted() && !this.heap.released
                        && this.connected?.current() === true,
                    discard: () => undefined }) };
        }, () => this.dispose()));
    }

    private clearSnapshot(): void {
        this.snapshot?.capsule.fill(0); this.snapshot?.journal.fill(0); this.snapshot = undefined;
        this.parked = false;
    }
    dispose(): void {
        if (this.closed) return;
        this.closed = true; this.enrollment?.remove(); this.registration?.remove(); this.clearSnapshot();
        for (const registration of this.peerRegistrations.splice(0)) registration.remove();
        this.disposeHeap(this.heap); this.connected = undefined;
    }
}
