import type { CemStudioIndexedDbRepository } from './repository.js';

export declare const CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID = 'file-system-access';
export declare const CEM_STUDIO_PROJECT_MANIFEST = 'project.cem';
export declare const CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE = 'application/vnd.cem.studio-project-bundle+json';

export type CemStudioFileSystemState =
    | 'unsupported'
    | 'unbound'
    | 'prompt-permission'
    | 'denied-permission'
    | 'ready';

export interface CemStudioFileSystemStatus {
    readonly provider: typeof CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID;
    readonly state: CemStudioFileSystemState;
    readonly scope?: 'project' | 'resource';
    readonly permission?: PermissionState;
    readonly bindingRevision?: number;
    readonly name?: string;
}

export interface CemStudioFileSystemProviderOptions {
    repository: Pick<CemStudioIndexedDbRepository, 'query' | 'execute'>;
    decodeProjectManifest?(
        bytes: Uint8Array,
        options?: { signal?: AbortSignal },
    ): Promise<Readonly<Record<string, unknown>>>;
    encodeProjectManifest?(
        project: Readonly<Record<string, unknown>>,
        options?: { signal?: AbortSignal },
    ): Promise<Uint8Array | ArrayBuffer | ArrayBufferView | string>;
    showOpenFilePicker?(options?: OpenFilePickerOptions): Promise<FileSystemFileHandle[]>;
    showDirectoryPicker?(options?: DirectoryPickerOptions): Promise<FileSystemDirectoryHandle>;
    crypto?: Crypto;
}

export declare class CemStudioFileSystemError extends Error {
    readonly code: string;
    readonly details: Readonly<Record<string, unknown>>;
    constructor(code: string, message: string, details?: Record<string, unknown>, cause?: unknown);
}

export interface CemStudioFileSystemProvider {
    capabilities(): Readonly<{
        provider: typeof CEM_STUDIO_FILE_SYSTEM_PROVIDER_ID;
        available: boolean;
        openFile: boolean;
        directory: boolean;
        retainedHandles: true;
        explicitPermission: true;
        indexedDbFallback: true;
        importExportFallback: true;
    }>;
    status(input: {
        projectId: string;
        scope?: 'project' | 'resource';
        resourceId?: string;
        signal?: AbortSignal;
    }): Promise<CemStudioFileSystemStatus>;
    reconnect(input: {
        projectId: string;
        scope?: 'project' | 'resource';
        resourceId?: string;
        mode?: 'read' | 'readwrite';
        requestPermission?: boolean;
        signal?: AbortSignal;
    }): Promise<CemStudioFileSystemStatus>;
    openResource(input: {
        projectId: string;
        resourceId: string;
        handle?: FileSystemFileHandle;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    pullResource(input: {
        projectId: string;
        resourceId: string;
        requestPermission?: boolean;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    writeResource(input: {
        projectId: string;
        resourceId: string;
        requestPermission?: boolean;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    bindProjectDirectory(input: {
        projectId: string;
        handle?: FileSystemDirectoryHandle;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    openProjectDirectory(input?: {
        handle?: FileSystemDirectoryHandle;
        mode?: 'create' | 'replace';
        expectedRevision?: number;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    writeProjectDirectory(input: {
        projectId: string;
        requestPermission?: boolean;
        signal?: AbortSignal;
    }): Promise<Readonly<Record<string, unknown>>>;
    exportFallback(input: {
        projectId: string;
        signal?: AbortSignal;
    }): Promise<Readonly<{
        status: 'exported';
        bundle: Readonly<{
            project: Readonly<Record<string, unknown>>;
            contents: Readonly<Record<string, Uint8Array>>;
        }>;
        archive: Readonly<{
            filename: string;
            contentType: typeof CEM_STUDIO_PROJECT_BUNDLE_CONTENT_TYPE;
            bytes: Uint8Array;
        }>;
    }>>;
    importFallback(input: {
        bundle?: unknown;
        archive?: Uint8Array | ArrayBuffer | ArrayBufferView | string;
        mode?: 'create' | 'replace';
        expectedRevision?: number;
        signal?: AbortSignal;
    }): Promise<Readonly<{ status: 'imported'; project: unknown }>>;
}

export declare function createCemStudioFileSystemProvider(
    options: CemStudioFileSystemProviderOptions,
): CemStudioFileSystemProvider;

export declare function serializeCemStudioProjectBundle(bundle: {
    project: Readonly<Record<string, unknown>>;
    contents: Record<string, Uint8Array | ArrayBuffer | ArrayBufferView>;
}): Uint8Array;

export declare function parseCemStudioProjectBundle(
    value: Uint8Array | ArrayBuffer | ArrayBufferView | string,
): {
    project: Readonly<Record<string, unknown>>;
    contents: Record<string, Uint8Array>;
};
