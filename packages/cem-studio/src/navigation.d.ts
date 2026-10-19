import type { CemStudioFeatureTourWorkbench, CemStudioWorkbenchState } from './workbench.js';
import type { CemStudioIndexedDbRepository } from './repository.js';
import type { CemStudioExampleProject } from './file-system-provider.js';
import type { CemStudioBrowserValidator } from './feature-tour.js';
import type { CemStudioWorkbenchProjection, CemStudioBuildArtifact } from './workbench.js';

export interface CemStudioProjectPreviewState {
    readonly projectId?: string;
    readonly contextKey?: string;
    readonly configurations: readonly { id: string; name: string; kind: string; resourceId?: string }[];
    readonly configurationId?: string;
    readonly artifactPath?: string;
    readonly available: boolean;
    readonly busy: boolean;
    readonly stale: boolean;
    readonly missingArtifact: boolean;
    readonly selectedArtifact?: CemStudioBuildArtifact;
    readonly artifacts: readonly CemStudioBuildArtifact[];
    readonly projection?: CemStudioWorkbenchProjection;
    readonly preview?: import('./preview.js').CemStudioPreview;
    readonly attempt?: { id: number; status: 'building' | 'succeeded' | 'failed' | 'cancelled' | 'stale'; diagnostics?: readonly Readonly<Record<string, unknown>>[]; error?: { message: string } };
}

export interface CemStudioNavigationNode {
    readonly key: string;
    readonly route: Readonly<Partial<Record<'view' | 'collection' | 'project' | 'sample' | 'folder' | 'file' | 'configuration', string>>>;
    readonly name: string;
    readonly kind: 'folder' | 'project' | 'file' | 'configuration';
    readonly description: string;
    readonly icon: string;
    readonly path?: string;
    readonly resourceId?: string;
    readonly children: readonly CemStudioNavigationNode[];
}
export interface CemStudioNavigationState {
    readonly samples: CemStudioNavigationNode;
    readonly saved: CemStudioNavigationNode;
    readonly files?: CemStudioNavigationNode;
    readonly selected: CemStudioNavigationNode;
    readonly children: readonly CemStudioNavigationNode[];
    readonly selection: CemStudioNavigationNode['route'] & { readonly key: string; readonly kind: string };
    readonly title: string;
    readonly status: string;
    readonly content: 'folder' | 'file' | 'sample-file';
    readonly count: number;
    readonly countKind: string;
    readonly error?: { readonly message: string };
    readonly action?: {
        readonly kind: 'create' | 'clone' | 'delete';
        readonly targetName: string;
        readonly createKind: 'project' | 'file';
    };
}
export interface CemStudioNavigation extends CemStudioFeatureTourWorkbench {
    snapshot(): CemStudioWorkbenchState & { readonly navigation: CemStudioNavigationState };
    subscribe(notify: (state: CemStudioWorkbenchState & { readonly navigation: CemStudioNavigationState }) => void): () => void;
    requestAction(kind: 'create' | 'clone' | 'delete'): void;
    cancelAction(): void;
    submitAction(fields: Readonly<Record<string, readonly string[]>>): Promise<void>;
    selectPreviewConfiguration(id: string): void;
    selectPreviewArtifact(path: string): void;
    buildProjectPreview(): Promise<void>;
    cancelProjectPreview(): void;
    copyPreview(writeText?: (text: string) => Promise<void>): Promise<void>;
    downloadPreview(save?: (file: { filename: string; contentType: string; bytes: Uint8Array }) => Promise<void>): Promise<void>;
}
export function createCemStudioNavigation(options: {
    workbench: CemStudioFeatureTourWorkbench;
    repository: CemStudioIndexedDbRepository;
    validator?: CemStudioBrowserValidator;
    openBuildWorkbench?: (options: Readonly<Record<string, unknown>>) => Promise<CemStudioFeatureTourWorkbench>;
    examples?: readonly CemStudioExampleProject[];
    collections?: readonly { id: string; name: string; description?: string }[];
    browser?: Pick<Window, 'location' | 'history' | 'addEventListener' | 'removeEventListener'>;
}): Promise<CemStudioNavigation>;

export function createCemStudioProjectPreview(options: {
    repository: CemStudioIndexedDbRepository;
    validator?: CemStudioBrowserValidator;
    openBuildWorkbench?: (options: Readonly<Record<string, unknown>>) => Promise<CemStudioFeatureTourWorkbench>;
}): {
    snapshot(): CemStudioProjectPreviewState;
    subscribe(notify: (state: CemStudioProjectPreviewState) => void): () => void;
    setContext(context?: Readonly<Record<string, unknown>>, settings?: { build?: string; artifact?: string; configuration?: string; reset?: boolean }): void;
    updateSource(state: CemStudioWorkbenchState): void;
    selectConfiguration(id: string): void;
    selectArtifact(path: string): void;
    build(): Promise<void>;
    cancel(): void;
    copy(writeText?: (text: string) => Promise<void>): Promise<void>;
    download(save?: (file: { filename: string; contentType: string; bytes: Uint8Array }) => Promise<void>): Promise<void>;
    dispose(): void;
};
