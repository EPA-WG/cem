import type {
    CemRepositoryCommandResult,
    CemRepositoryPort,
    CemRepositoryQueryResult,
    CemRepositoryRequest,
    CemRepositoryStatus,
} from '@epa-wg/cem-elements';

export declare const CEM_STUDIO_REPOSITORY_ID = 'studio-projects';
export declare const CEM_STUDIO_DATABASE_VERSION = 1;
export declare const CEM_STUDIO_SEARCH_INDEX_VERSION = 1;

export interface CemStudioProjectValidationOptions {
    signal?: AbortSignal;
    operation: 'import' | 'export';
}

export interface CemStudioRepositoryOptions {
    databaseName?: string;
    indexedDB?: IDBFactory;
    crypto?: Crypto;
    storage?: StorageManager;
    BroadcastChannel?: typeof globalThis.BroadcastChannel;
    validateProject(bundle: unknown, options: CemStudioProjectValidationOptions): unknown | Promise<unknown>;
    now?: () => string;
}

export type CemStudioCommandPageTarget =
    | Readonly<{
        mode: 'current';
        entryId: string;
        parentId?: string;
        confirmIncompatibleReplacement?: boolean;
    }>
    | Readonly<{
        mode: 'existing';
        entryId: string;
        parentId?: string;
        confirmIncompatibleReplacement?: boolean;
    }>
    | Readonly<{
        mode: 'existing';
        name: string;
        parentId?: string;
        confirmIncompatibleReplacement?: boolean;
    }>
    | Readonly<{
        mode: 'new';
        name: string;
        entryId?: string;
        parentId?: string;
    }>;

export interface CemStudioApplyCommandPageParameters {
    projectId: string;
    expectedProjectRevision: number;
    target: CemStudioCommandPageTarget;
    commandResource: string | ArrayBuffer | ArrayBufferView;
    referencedResourceIds: readonly string[];
    updatedAt?: string;
}

export interface CemStudioApplyCommandPageRequest extends Omit<CemRepositoryRequest, 'operation' | 'parameters'> {
    operation: 'apply-command-page';
    parameters: CemStudioApplyCommandPageParameters;
}

export interface CemStudioApplyCommandPageValue {
    disposition: 'created' | 'updated';
    operation: 'parse' | 'inspect';
    pageKind: 'inspection';
    projectId: string;
    projectRevision: number;
    entryRevision: number;
    resourceRevision: number;
    sha256: string;
    entry: Readonly<Record<string, unknown>>;
    commandResource: Readonly<Record<string, unknown>>;
    commandBytes: ArrayBuffer;
}

export interface CemStudioApplyCommandPageResult extends Omit<CemRepositoryCommandResult, 'value'> {
    value: CemStudioApplyCommandPageValue;
}

export declare class CemStudioRepositoryError extends Error {
    readonly code: string;
    readonly details: Readonly<Record<string, unknown>>;
    constructor(code: string, message: string, details?: Record<string, unknown>, cause?: unknown);
}

export declare class CemStudioIndexedDbRepository implements CemRepositoryPort {
    readonly databaseName: string;
    constructor(options: CemStudioRepositoryOptions);
    open(): Promise<IDBDatabase>;
    query(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryQueryResult>;
    execute(request: CemStudioApplyCommandPageRequest, signal?: AbortSignal): Promise<CemStudioApplyCommandPageResult>;
    execute(request: CemRepositoryRequest, signal?: AbortSignal): Promise<CemRepositoryCommandResult>;
    status(): Promise<CemRepositoryStatus>;
    subscribe(cursor: number, notify: Parameters<CemRepositoryPort['subscribe']>[1]): () => void;
    close(): void;
    deleteDatabase(): Promise<void>;
}

export declare function createCemStudioProjectRepository(
    options: CemStudioRepositoryOptions,
): CemStudioIndexedDbRepository;

export declare function normalizeRepositoryError(error: unknown, operation: string): CemStudioRepositoryError;
