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

export type CemStudioParseProjection = 'ast' | 'events';
export type CemStudioInspectView = 'summary' | 'ast' | 'events' | 'diagnostics' | 'source-offsets' | 'tree';
export type CemStudioPortableOperation = 'parse' | 'inspect' | 'convert' | 'query' | 'transform' | 'trace';

export interface CemStudioResourceCommandOptions {
    readonly bytes: Uint8Array | ArrayBuffer;
    readonly contentType: string;
    readonly schema: string;
    readonly uri?: string;
    readonly dependencies?: readonly {
        readonly bytes: Uint8Array | ArrayBuffer;
        readonly contentType: string;
        readonly schema: string;
        readonly path: string;
    }[];
    readonly projectId?: string;
    readonly projectRevision?: number;
    readonly resourceRevision?: number;
    readonly signal?: AbortSignal;
    readonly argv?: readonly string[];
}

export interface CemStudioCommandOutput {
    readonly uri: string;
    readonly contentType: string;
    readonly byteLength: number;
    readonly sha256: string;
    readonly bytes: readonly number[];
    readonly text: string;
}

export interface CemStudioBrowserCommandOutcome {
    readonly result: unknown;
    readonly presentation: unknown;
    readonly output?: CemStudioCommandOutput;
}

export interface CemStudioAuthoredResourceCommandOptions extends CemStudioResourceCommandOptions {
    readonly commandResource: string | Uint8Array | ArrayBuffer;
}

export interface CemStudioResourceCommandPreview {
    readonly projection: 'studio';
    readonly binaryName: 'cem-ml';
    readonly commonVersion: string;
    readonly argv: readonly string[];
    readonly text: string;
    readonly parsed: Readonly<Record<string, unknown>>;
    readonly semantic: Readonly<Record<string, unknown>>;
}

export interface CemStudioResourceCommandPreviewOptions extends CemStudioResourceCommandOptions {
    readonly operation?: CemStudioPortableOperation | 'validate';
    readonly projection?: CemStudioParseProjection;
    readonly view?: CemStudioInspectView;
    readonly text?: string;
}

export interface CemStudioBrowserValidator {
    readonly capability: Readonly<Record<string, unknown>>;
    readonly commonVersion: string;
    parseResource(options: CemStudioResourceCommandOptions & {
        readonly projection?: CemStudioParseProjection;
    }): Promise<CemStudioBrowserCommandOutcome & { readonly output: CemStudioCommandOutput }>;
    inspectResource(options: CemStudioResourceCommandOptions & {
        readonly view?: CemStudioInspectView;
    }): Promise<CemStudioBrowserCommandOutcome & { readonly output: CemStudioCommandOutput }>;
    runResourceCommand(options: CemStudioResourceCommandOptions & {
        readonly argv: readonly string[];
    }): Promise<CemStudioBrowserCommandOutcome & { readonly output: CemStudioCommandOutput }>;
    executeAuthoredResourceCommand(
        options: CemStudioAuthoredResourceCommandOptions,
    ): Promise<CemStudioBrowserCommandOutcome & {
        readonly output: CemStudioCommandOutput;
        readonly parsed: Readonly<Record<string, unknown>>;
    }>;
    previewResourceCommand(options: CemStudioResourceCommandPreviewOptions): Promise<CemStudioResourceCommandPreview>;
    serializeResourceCommand(command: Readonly<Record<string, unknown>>): string;
    validateResource(options: CemStudioResourceCommandOptions): Promise<CemStudioBrowserCommandOutcome>;
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
