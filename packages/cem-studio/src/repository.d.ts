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
