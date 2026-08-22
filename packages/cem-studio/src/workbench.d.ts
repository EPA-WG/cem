import type { CemStudioFeatureTourSeed, CemStudioBrowserValidator } from './feature-tour.js';
import type { CemStudioIndexedDbRepository } from './repository.js';

export interface CemStudioWorkbenchSelection {
    readonly kind: 'diagnostic' | 'provenance';
    readonly index: number;
    readonly byteStart: number;
    readonly byteLength: number;
    readonly start: number;
    readonly end: number;
}

export interface CemStudioWorkbenchState {
    readonly status: string;
    readonly projectId: string;
    readonly resourceId: string;
    readonly path: string;
    readonly contentType: string;
    readonly schema: string;
    readonly projectRevision: number;
    readonly resourceRevision: number;
    readonly repositoryRevision: number;
    readonly persistedText: string;
    readonly draft: string;
    readonly dirty: boolean;
    readonly validation?: Readonly<Record<string, unknown>>;
    readonly selection?: CemStudioWorkbenchSelection;
    readonly error?: Readonly<{ code: string; message: string }>;
}

export interface CemStudioFeatureTourWorkbench {
    snapshot(): CemStudioWorkbenchState;
    subscribe(notify: (state: CemStudioWorkbenchState) => void): () => void;
    updateDraft(draft: string): CemStudioWorkbenchState;
    reload(): Promise<CemStudioWorkbenchState>;
    saveAndValidate(options?: { signal?: AbortSignal }): Promise<CemStudioWorkbenchState>;
    validatePersisted(options?: { signal?: AbortSignal }): Promise<CemStudioWorkbenchState>;
    navigateDiagnostic(index: number): CemStudioWorkbenchSelection;
    navigateProvenance(index: number): CemStudioWorkbenchSelection;
    dispose(): void;
}

export declare function createCemStudioFeatureTourWorkbench(options: {
    repository: CemStudioIndexedDbRepository;
    validator: CemStudioBrowserValidator;
    seed: CemStudioFeatureTourSeed;
    projectId: string;
    example?: Readonly<Record<string, unknown>>;
}): Promise<CemStudioFeatureTourWorkbench>;

export declare function mountCemStudioFeatureTourWorkbench(options: {
    root: Element;
    workbench: CemStudioFeatureTourWorkbench;
}): Promise<Readonly<{
    root: Element;
    workbench: CemStudioFeatureTourWorkbench;
    whenSettled(): Promise<void>;
    dispose(): void;
}>>;
