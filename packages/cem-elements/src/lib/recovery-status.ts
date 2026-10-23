import type { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import type { CemNativeRecoveryScope } from './internal/runtime-support/native-generation.js';

export type CemRecoveryStatusState = 'unconfigured' | 'observing' | 'busy' | 'blocked' | 'unavailable';
export type CemRecoveryStatusReason = 'not-configured' | 'observed' | 'preparing' | 'admissions' | 'unknown-operations'
    | 'owner-adapters' | 'owner-publications' | 'publication-unavailable' | 'scope-unavailable' | 'coverage-limit' | 'inspection-failed';
/** Advisory metadata only. No field/source bytes, owner IDs, errors, consent,
 * checkpoint availability or replacement/resume authority cross this boundary. */
export interface CemRecoveryStatus {
    readonly version: 'cem-recovery-status/1';
    readonly state: CemRecoveryStatusState;
    readonly reason: CemRecoveryStatusReason;
    readonly runtimeCount: number;
    readonly ownerCount: number;
    readonly acknowledgedCount: number;
    readonly admissionCount: number;
    readonly unknownOperationCount: number;
    readonly unsupportedOwnerCount: number;
    readonly automaticRecovery: false;
    readonly credentialPolicy: 'block-entered';
}
export interface CemRecoveryStatusOptions {
    publication?: CemCheckpointPublicationCoordinator;
    recoveryScope?: CemNativeRecoveryScope;
}

/** Metadata polling deliberately avoids current(), export, capture, prepare,
 * native telemetry and DOM inspection. Property-only credential edits cannot
 * therefore be inferred here: the original capture-time privacy gate applies.
 * One 1s timer exists only while subscribed; unchanged snapshots are coalesced. */
export class CemRecoveryStatusObserver {
    private readonly listeners = new Set<(status: CemRecoveryStatus) => void>();
    private timer?: ReturnType<typeof setInterval>;
    private disposed = false;
    private readonly suspensions = new Set<object>();
    private previous?: CemRecoveryStatus;
    private readonly options: CemRecoveryStatusOptions;

    constructor(options: CemRecoveryStatusOptions = {}) { this.options = { ...options }; }

    snapshot(): CemRecoveryStatus {
        const counts = { runtimeCount: 0, ownerCount: 0, acknowledgedCount: 0, admissionCount: 0, unknownOperationCount: 0, unsupportedOwnerCount: 0 };
        const result = (state: CemRecoveryStatusState, reason: CemRecoveryStatusReason): CemRecoveryStatus => Object.freeze({
            version: 'cem-recovery-status/1', state, reason, ...counts, automaticRecovery: false, credentialPolicy: 'block-entered',
        });
        if (this.disposed) return result('unavailable', 'inspection-failed');
        try {
            const scope = this.options.recoveryScope?.inspect();
            counts.runtimeCount = scope?.members.length ?? 0;
            if (scope?.disposed) return result('unavailable', 'scope-unavailable');
            const root = this.options.publication;
            if (!root) return result('unconfigured', 'not-configured');
            const visited = new Set<CemCheckpointPublicationCoordinator>();
            const pending = [root];
            let blocked = false, unavailable = false, busy = !!scope?.preparing;
            while (pending.length) {
                const coordinator = pending.pop();
                if (!coordinator) break;
                if (visited.has(coordinator) || visited.size >= 256) return result('blocked', 'coverage-limit');
                visited.add(coordinator);
                const status = coordinator.inspectRecovery();
                blocked ||= status.blocked; unavailable ||= status.disposed; busy ||= status.capturing;
                counts.ownerCount += status.owners.length;
                counts.acknowledgedCount += status.owners.filter(owner => owner.acknowledged).length;
                counts.unsupportedOwnerCount += status.owners.filter(owner => !owner.adapterAvailable).length;
                counts.admissionCount += status.inFlightAdmissions;
                counts.unknownOperationCount += status.unknownOperations;
                const children = coordinator.previewCoordinators();
                if (pending.length + visited.size + children.length > 256) return result('blocked', 'coverage-limit');
                pending.push(...children);
            }
            if (unavailable || blocked) return result('unavailable', 'publication-unavailable');
            if (counts.unknownOperationCount) return result('blocked', 'unknown-operations');
            if (busy) return result('busy', 'preparing');
            if (counts.admissionCount) return result('busy', 'admissions');
            if (counts.unsupportedOwnerCount) return result('blocked', 'owner-adapters');
            if (!counts.ownerCount || counts.acknowledgedCount !== counts.ownerCount) return result('blocked', 'owner-publications');
            // Registration/publication is not a current checkpoint or complete
            // physical-runtime coverage proof, including combined fallback heaps.
            return result('observing', 'observed');
        } catch { return result('unavailable', 'inspection-failed'); }
    }

    refresh(): CemRecoveryStatus {
        const next = this.snapshot();
        if ((!this.suspensions.size || this.disposed) && JSON.stringify(next) !== JSON.stringify(this.previous)) {
            this.previous = next;
            for (const notify of this.listeners) this.notify(notify, next);
        }
        return next;
    }

    subscribe(notify: (status: CemRecoveryStatus) => void): () => void {
        const next = this.snapshot();
        if (this.disposed) { this.notify(notify, next); return () => undefined; }
        this.listeners.add(notify);
        if (!this.suspensions.size) this.notify(notify, next);
        if (!this.timer && this.listeners.size && !this.suspensions.size) {
            this.previous = next;
            this.timer = setInterval(() => this.refresh(), 1000);
        }
        return () => {
            this.listeners.delete(notify);
            if (!this.listeners.size) this.stop();
        };
    }

    /** Quiesce host status publications before draining/capturing/retiring.
     * Polling must not introduce a new UI input into its own checkpoint barrier.
     * Release only after replacement owners and their publications settle. */
    suspend(): () => void {
        if (this.disposed) return () => undefined;
        const token = {}; this.suspensions.add(token); this.stop();
        return () => {
            if (!this.suspensions.delete(token) || this.suspensions.size || this.disposed) return;
            this.refresh();
            if (!this.disposed && !this.suspensions.size && !this.timer && this.listeners.size) this.timer = setInterval(() => this.refresh(), 1000);
        };
    }

    dispose(): void {
        if (this.disposed) return;
        this.disposed = true; this.stop(); this.refresh(); this.listeners.clear(); this.suspensions.clear();
    }
    private stop(): void { if (this.timer) clearInterval(this.timer); this.timer = undefined; }
    private notify(notify: (status: CemRecoveryStatus) => void, status: CemRecoveryStatus): void {
        // A presentation subscriber cannot break recovery or sibling observers.
        try { notify(status); } catch { /* No arbitrary error/source bytes are logged. */ }
    }
}
