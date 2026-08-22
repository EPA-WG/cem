export declare const CEM_STUDIO_THEME_MODES: readonly [
    'cem-theme-light',
    'cem-theme-dark',
    'cem-theme-contrast-light',
    'cem-theme-contrast-dark',
    'cem-theme-native',
];

export type CemStudioThemeMode = (typeof CEM_STUDIO_THEME_MODES)[number];

export interface CemStudioThemeController {
    readonly mode: CemStudioThemeMode;
    setMode(mode: CemStudioThemeMode): CemStudioThemeMode;
}

export interface CemStudioInstallStatus {
    readonly state: 'waiting' | 'ready' | 'prompting' | 'installed' | 'dismissed';
    readonly outcome?: string;
    readonly platform?: string;
}

export interface CemStudioInstallController {
    status(): CemStudioInstallStatus;
    prompt(): Promise<CemStudioInstallStatus>;
    subscribe(notify: (status: CemStudioInstallStatus) => void): () => void;
    dispose(): void;
}

export interface CemStudioUpdateStatus {
    readonly state: 'current' | 'ready' | 'blocked' | 'persisting' | 'activating';
    readonly reason?: 'active-work' | 'persistence-failed';
    readonly activeRequestCount: number;
    readonly dirty: boolean;
    readonly updateReady: boolean;
}

export interface CemStudioUpdateCoordinator {
    status(): CemStudioUpdateStatus;
    setWaitingWorker(worker?: Pick<ServiceWorker, 'postMessage'>): void;
    setActiveRequestCount(count: number): void;
    setDirty(dirty: boolean): void;
    activateUpdate(): Promise<CemStudioUpdateStatus>;
    subscribe(notify: (status: CemStudioUpdateStatus) => void): () => void;
    dispose(): void;
}

export interface CemStudioApplicationShell {
    readonly root: Element;
    readonly theme: CemStudioThemeController;
    readonly install: CemStudioInstallController;
    readonly update: CemStudioUpdateCoordinator;
    readonly fileSystem: CemStudioFileSystemController;
    dispose(): void;
}

export interface CemStudioFileSystemControllerStatus {
    readonly state: string;
    readonly operation?: string;
    readonly message: string;
    readonly projectId?: string;
    readonly resourceId?: string;
    readonly capabilities: Readonly<Record<string, unknown>>;
    readonly lastResult?: unknown;
    readonly lastError?: Readonly<Record<string, unknown>>;
}

export interface CemStudioFileSystemController {
    status(): CemStudioFileSystemControllerStatus;
    refresh(): Promise<CemStudioFileSystemControllerStatus>;
    setProject(projectId?: string, resourceId?: string): Promise<CemStudioFileSystemControllerStatus>;
    openProjectDirectory(): Promise<CemStudioFileSystemControllerStatus>;
    bindProjectDirectory(): Promise<CemStudioFileSystemControllerStatus>;
    openResource(): Promise<CemStudioFileSystemControllerStatus>;
    reconnect(): Promise<CemStudioFileSystemControllerStatus>;
    writeBack(): Promise<CemStudioFileSystemControllerStatus>;
    importFallback(): Promise<CemStudioFileSystemControllerStatus>;
    exportFallback(): Promise<CemStudioFileSystemControllerStatus>;
    subscribe(notify: (status: CemStudioFileSystemControllerStatus) => void): () => void;
    dispose(): void;
}

export function installCemStudioShellComponents(): Promise<Readonly<Record<string, unknown>>>;
export function createCemStudioThemeController(options: {
    root: Element;
    storage?: Storage;
    storageKey?: string;
    defaultMode?: CemStudioThemeMode;
}): CemStudioThemeController;
export function createCemStudioInstallController(options?: { eventTarget?: EventTarget }): CemStudioInstallController;
export function createCemStudioUpdateCoordinator(options?: {
    registration?: ServiceWorkerRegistration;
    persistState?: () => Promise<void>;
    serviceWorkerContainer?: ServiceWorkerContainer;
    reload?: () => void;
}): CemStudioUpdateCoordinator;
export function createCemStudioFileSystemController(options?: {
    provider?: CemStudioFileSystemProvider;
    projectId?: string;
    resourceId?: string;
    selectImport?: () => Promise<Uint8Array | ArrayBuffer | ArrayBufferView>;
    downloadExport?: (archive: {
        filename: string;
        contentType: string;
        bytes: Uint8Array;
    }) => Promise<void>;
}): CemStudioFileSystemController;
export function mountCemStudioApplicationShell(options: {
    root: Element;
    storage?: Storage;
    registration?: ServiceWorkerRegistration;
    repository?: { status(): Promise<{ state: string; persisted?: boolean }> };
    persistState?: () => Promise<void>;
    fileSystem?: Parameters<typeof createCemStudioFileSystemController>[0];
}): Promise<CemStudioApplicationShell>;
import type { CemStudioFileSystemProvider } from './file-system-provider.js';
