import type {
    CemStudioFeatureTourWorkbench,
    CemStudioWorkbenchState,
} from './workbench.js';

/** Durable repository identities; local selection is never added to an export. */
export interface CemStudioProjectSelection {
    readonly projectId: string;
    readonly entryId?: string;
    readonly resourceId?: string;
}

/** Each resolver call returns a fresh service, even for a previously opened identity. */
export type CemStudioSessionWorkbench = Pick<CemStudioFeatureTourWorkbench,
    'snapshot' | 'subscribe' | 'dispose'> & Partial<Pick<CemStudioFeatureTourWorkbench,
    'updateDraft' | 'updateCommandDraft' | 'setCommandTarget' | 'saveSource' | 'applyCommand' | 'confirmCommandReplacement' | 'cancelCommandReplacement'>>;

export interface CemStudioProjectSessionOptions {
    storage?: Pick<Storage, 'getItem' | 'setItem'>;
    storageKey?: string;
    openWorkbench(
        selection: CemStudioProjectSelection,
        options: { signal: AbortSignal },
    ): Promise<CemStudioSessionWorkbench>;
}

export interface CemStudioProjectTransition {
    readonly id: number;
    readonly target: CemStudioProjectSelection;
    readonly stage: 'source' | 'command' | 'confirmation' | 'saving' | 'applying' | 'opening';
    readonly error?: Readonly<{ code: string; message: string }>;
}

export interface CemStudioDraftRecovery {
    readonly status: 'ready' | 'saved' | 'recovered' | 'conflict' | 'error';
    readonly message: string;
    readonly available: boolean;
    readonly blocked: boolean;
}

export interface CemStudioProjectSessionState {
    readonly draftRecovery?: CemStudioDraftRecovery;
    readonly status: 'empty' | 'ready' | 'guarding' | 'saving' | 'applying' | 'opening' | 'settling' | 'failed' | 'disposed';
    readonly recoveryError?: Readonly<{code: string; message: string}>;
    readonly selection?: CemStudioProjectSelection;
    readonly workbench?: CemStudioWorkbenchState;
    /** Cancellation revokes admission; an already-started transaction may still commit. */
    readonly pendingWrite: boolean;
    readonly transition?: CemStudioProjectTransition;
}

export type CemStudioProjectTransitionDecision =
    'save' | 'apply' | 'discard' | 'cancel' | 'confirm-replacement' | 'retry';

export interface CemStudioProjectSession {
    snapshot(): CemStudioProjectSessionState;
    refresh(): Promise<CemStudioProjectSessionState>;
    recover(fallback?: CemStudioProjectSelection): Promise<CemStudioProjectSessionState>;
    draftBackup(): string;
    clearDraftRecovery(): CemStudioProjectSessionState;
    activeWorkbench(): CemStudioSessionWorkbench | undefined;
    subscribe(notify: (state: CemStudioProjectSessionState) => void): () => void;
    /** Duplicate requests coalesce. Cancel a pending target before requesting another. */
    select(selection: CemStudioProjectSelection): Promise<CemStudioProjectSessionState>;
    /** A stale transition id is a no-op. Discard consent applies only to the exact draft. */
    resolveTransition(id: number, decision: CemStudioProjectTransitionDecision): Promise<CemStudioProjectSessionState>;
    cancelTransition(id: number): CemStudioProjectSessionState;
    dispose(): void;
}

export declare function createCemStudioProjectSession(options: CemStudioProjectSessionOptions): CemStudioProjectSession;
