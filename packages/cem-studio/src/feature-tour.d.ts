import type { CemStudioIndexedDbRepository } from './repository.js';

export declare const CEM_STUDIO_FEATURE_TOUR_SEED_ID = 'cem-ml-feature-tour-seed';
export declare const CEM_STUDIO_FEATURE_TOUR_COPY_ID = 'feature-tour';
export declare const CEM_STUDIO_PROJECT_CONTENT_TYPE = 'application/vnd.cem.studio-project+json';
export declare const CEM_STUDIO_PROJECT_SCHEMA = 'https://cem.dev/ns/studio/project/1';

export interface CemStudioFeatureTourSeed {
    readonly catalog: Readonly<Record<string, unknown>>;
    readonly bundle: {
        project: Readonly<Record<string, unknown>>;
        contents: Record<string, Uint8Array>;
    };
}

export interface CemStudioBrowserValidator {
    readonly capability: Readonly<Record<string, unknown>>;
    readonly commonVersion: string;
    validateResource(options: {
        bytes: Uint8Array | ArrayBuffer;
        contentType: string;
        schema: string;
        uri?: string;
        dependencies?: readonly {
            bytes: Uint8Array | ArrayBuffer;
            contentType: string;
            schema: string;
            path: string;
        }[];
        projectId?: string;
        projectRevision?: number;
        resourceRevision?: number;
        signal?: AbortSignal;
    }): Promise<Readonly<{ result: unknown; presentation: unknown }>>;
    validateProject(bundle: unknown, options?: { signal?: AbortSignal }): Promise<unknown>;
    assertCatalog(catalog: unknown): void;
    close(): Promise<void>;
}

export declare function createCemStudioBrowserValidator(): Promise<CemStudioBrowserValidator>;
export declare function loadCemStudioFeatureTour(options?: {
    baseUrl?: string | URL;
    fetch?: typeof globalThis.fetch;
    validator?: CemStudioBrowserValidator;
}): Promise<CemStudioFeatureTourSeed>;
export declare function installCemStudioFeatureTour(
    repository: CemStudioIndexedDbRepository,
    seed: CemStudioFeatureTourSeed,
    options?: { reset?: boolean; now?: () => string },
): Promise<Readonly<{
    status: 'installed' | 'preserved' | 'reset';
    projectId: string;
    seedVersion: string;
    repositoryRevision?: number;
}>>;
export declare function createCemStudioFeatureTourCopy(
    seed: CemStudioFeatureTourSeed,
    options: { projectId: string; now: string },
): CemStudioFeatureTourSeed['bundle'];
