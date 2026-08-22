import type {
    CemStudioBrowserValidator,
    CemStudioCommandOutput,
    CemStudioFeatureTourSeed,
    CemStudioInspectView,
    CemStudioParseProjection,
    CemStudioResourceCommandPreview,
} from './feature-tour.js';
import type { CemStudioIndexedDbRepository } from './repository.js';

export declare const CEM_STUDIO_PARSE_PROJECTIONS: readonly CemStudioParseProjection[];
export declare const CEM_STUDIO_INSPECT_VIEWS: readonly CemStudioInspectView[];

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
    readonly projection?: CemStudioWorkbenchProjection;
    readonly command?: CemStudioWorkbenchCommand;
    readonly selection?: CemStudioWorkbenchSelection;
    readonly error?: Readonly<{ code: string; message: string }>;
}

export interface CemStudioWorkbenchCommandChange {
    readonly category: string;
    readonly path: string;
    readonly kind: 'added' | 'removed' | 'changed';
    readonly before?: unknown;
    readonly after?: unknown;
}

export interface CemStudioWorkbenchCommand {
    readonly projection: 'studio';
    readonly status: 'checking' | 'current' | 'changed' | 'invalid';
    readonly current: CemStudioResourceCommandPreview;
    readonly draftText: string;
    readonly parsed?: Readonly<Record<string, unknown>>;
    readonly preview?: CemStudioResourceCommandPreview;
    readonly changes: readonly CemStudioWorkbenchCommandChange[];
    readonly diagnostic?: Readonly<{ code: string; message: string }>;
    readonly copy?: Readonly<{
        status: 'copying' | 'success' | 'failed';
        message?: string;
    }>;
    readonly revision: Readonly<{
        projectRevision: number;
        resourceRevision: number;
        sha256: string;
    }>;
}

export interface CemStudioWorkbenchProjection {
    readonly kind: 'parse' | 'inspect';
    readonly mode: CemStudioParseProjection | CemStudioInspectView;
    readonly requestId?: string;
    readonly exitCode?: number;
    readonly executionIdentity?: Readonly<Record<string, unknown>>;
    readonly revision: Readonly<{
        projectRevision: number;
        resourceRevision: number;
        sha256: string;
    }>;
    readonly output: CemStudioCommandOutput;
    readonly nativeResult: unknown;
    readonly diagnostics: readonly Readonly<Record<string, unknown>>[];
    readonly provenance: readonly Readonly<Record<string, unknown>>[];
    readonly presentation: unknown;
    readonly sourceByteLength: number;
    readonly stale: boolean;
}

export interface CemStudioFeatureTourWorkbench {
    snapshot(): CemStudioWorkbenchState;
    subscribe(notify: (state: CemStudioWorkbenchState) => void): () => void;
    updateDraft(draft: string): CemStudioWorkbenchState;
    reload(): Promise<CemStudioWorkbenchState>;
    saveAndValidate(options?: { signal?: AbortSignal }): Promise<CemStudioWorkbenchState>;
    validatePersisted(options?: { signal?: AbortSignal }): Promise<CemStudioWorkbenchState>;
    parsePersisted(
        projection?: CemStudioParseProjection,
        options?: { signal?: AbortSignal },
    ): Promise<CemStudioWorkbenchState>;
    inspectPersisted(
        view?: CemStudioInspectView,
        options?: { signal?: AbortSignal },
    ): Promise<CemStudioWorkbenchState>;
    updateCommandDraft(text: string): Promise<CemStudioWorkbenchState>;
    resetCommandDraft(): Promise<CemStudioWorkbenchState>;
    copyCommand(writeText?: (text: string) => Promise<void>): Promise<CemStudioWorkbenchState>;
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
    clipboard?: Pick<Clipboard, 'writeText'>;
}): Promise<Readonly<{
    root: Element;
    workbench: CemStudioFeatureTourWorkbench;
    whenSettled(): Promise<void>;
    dispose(): void;
}>>;
