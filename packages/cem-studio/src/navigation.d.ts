import type { CemStudioBuildComparison, CemStudioBuildSnapshotSummary, CemStudioPinnedBuildProjection } from './build-snapshots.js';
import type { CemStudioFeatureTourWorkbench, CemStudioWorkbenchState } from './workbench.js';
import type { CemStudioIndexedDbRepository } from './repository.js';
import type { CemStudioExampleProject } from './file-system-provider.js';
import type { CemStudioBrowserValidator } from './feature-tour.js';
import type { CemStudioWorkbenchProjection, CemStudioBuildArtifact } from './workbench.js';

export interface CemStudioProjectPreviewState {
    readonly pins: readonly CemStudioBuildSnapshotSummary[];
    readonly pinId?: string;
    readonly pinSelection?: string;
    readonly pinBusy: boolean;
    readonly pinError?: { readonly message: string };
    readonly pinMessage?: string;
    readonly canPin: boolean;
    readonly comparison?: CemStudioBuildComparison;
    readonly comparisonArtifactPath?: string;
    readonly currentRunId?: string;
    readonly previewContext: 'project' | 'source' | 'output' | 'none';
    readonly relatedArtifacts: readonly CemStudioBuildArtifact[];
    readonly missingReason?: string;
    readonly selectedOutput?: import('./workbench.js').CemStudioCommandOutput;
    readonly currentArtifactPath?: string;
    readonly projectId?: string;
    readonly contextKey?: string;
    readonly configurations: readonly { id: string; name: string; kind: string; resourceId?: string; entryArtifact?: string }[];
    readonly configurationId?: string;
    readonly entryArtifact?: string;
    readonly entryEditable?: boolean;
    readonly canEditEntry?: boolean;
    readonly entryBusy?: boolean;
    readonly entryError?: { readonly message: string };
    readonly entryMessage?: string;
    readonly artifactPath?: string;
    readonly available: boolean;
    readonly busy: boolean;
    readonly stale: boolean;
    readonly missingArtifact: boolean;
    readonly selectedArtifact?: CemStudioBuildArtifact;
    readonly artifacts: readonly CemStudioBuildArtifact[];
    readonly projection?: CemStudioWorkbenchProjection | CemStudioPinnedBuildProjection;
    readonly preview?: import('./preview.js').CemStudioPreview;
    readonly attempt?: { id: number; status: 'building' | 'succeeded' | 'failed' | 'cancelled' | 'stale'; diagnostics?: readonly Readonly<Record<string, unknown>>[]; error?: { message: string } };
}

export interface CemStudioNavigationNode {
    readonly key: string;
    readonly route: Readonly<Partial<Record<'view' | 'collection' | 'project' | 'sample' | 'folder' | 'file' | 'configuration' | 'selection' | 'output-folder' | 'build' | 'artifact', string>>>;
    readonly name: string;
    readonly kind: 'folder' | 'project' | 'file' | 'configuration' | 'output-folder' | 'output-file';
    readonly description: string;
    readonly icon: string;
    readonly path?: string;
    readonly resourceId?: string;
    readonly contentType?: string;
    readonly children: readonly CemStudioNavigationNode[];
}
export interface CemStudioNavigationState {
    readonly samples: CemStudioNavigationNode;
    readonly saved: CemStudioNavigationNode;
    readonly files?: CemStudioNavigationNode;
    readonly output?: CemStudioNavigationNode;
    readonly sections: Readonly<Record<'projects' | 'structure' | 'output', boolean>>;
    readonly trees: Readonly<Record<'projects' | 'structure' | 'output', { readonly selected: string; readonly expanded: string }>>;
    readonly projectName?: string;
    readonly path?: string;
    readonly pathKind: CemStudioNavigationNode['kind'];
    readonly canCreate: boolean;
    readonly canClone: boolean;
    readonly canDelete: boolean;
    readonly sourcePreview?: CemStudioSourcePreviewState & Readonly<{ resourceId: string; sourceUri: string; contentType: string; schema?: string; text: string; revision: number; sha256: string; draft: boolean; mode: 'saved' | 'draft' }>;
    readonly expanded: string;
    readonly selected: CemStudioNavigationNode;
    readonly children: readonly CemStudioNavigationNode[];
    readonly selection: CemStudioNavigationNode['route'] & { readonly key: string; readonly kind: string };
    readonly title: string;
    readonly status: string;
    readonly content: 'folder' | 'project' | 'file' | 'sample-file' | 'output-file' | 'output-folder';
    readonly sampleText: string;
    readonly contentType: string;
    readonly sourceId: string;
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
    setNavigationSection(name: 'projects' | 'structure' | 'output', open: boolean): void;
    toggleNavigationSection(name: 'projects' | 'structure' | 'output'): void;
    setNavigationExpanded(tree: 'projects' | 'structure' | 'output', keys: string | readonly string[]): void;
    toggleNavigationExpanded(tree: 'projects' | 'structure' | 'output', key: string): void;
    requestAction(kind: 'create' | 'clone' | 'delete'): void;
    cancelAction(): void;
    submitAction(fields: Readonly<Record<string, readonly string[]>>): Promise<void>;
    selectPreviewConfiguration(id: string): void;
    selectPreviewArtifact(path: string): void | Promise<void>;
    pinProjectBuild(): Promise<void>;
    selectPinnedBuild(key: string): Promise<void>;
    deletePinnedBuild(): Promise<void>;
    selectComparisonArtifact(path: string): void;
    saveBuildEntryArtifact(path: string): Promise<void>;
    buildProjectPreview(): Promise<void>;
    cancelProjectPreview(): void;
    copyPreview(writeText?: (text: string) => Promise<void>): Promise<void>;
    downloadPreview(save?: (file: { filename: string; contentType: string; bytes: Uint8Array }) => Promise<void>): Promise<void>;
}
export function createCemStudioNavigation(options: {
    workbench: CemStudioFeatureTourWorkbench;
    repository: CemStudioIndexedDbRepository;
    /** Selected resources prepare native-declared demand before navigation publication. */
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
    setContext(context?: Readonly<Record<string, unknown>>, settings?: { build?: string; artifact?: string; configuration?: string; reset?: boolean; sourceActivation?: boolean; sourceResourceId?: string }): void;
    setSelection(selection: CemStudioNavigationNode['route']): void;
    updateSource(state: CemStudioWorkbenchState): void;
    selectConfiguration(id: string): void;
    selectArtifact(path: string): void;
    pin(): Promise<void>;
    selectPin(key: string): Promise<void>;
    deletePin(): Promise<void>;
    selectComparisonArtifact(path: string): void;
    refreshPins(): Promise<void>;
    build(): Promise<void>;
    cancel(): void;
    copy(writeText?: (text: string) => Promise<void>): Promise<void>;
    download(save?: (file: { filename: string; contentType: string; bytes: Uint8Array }) => Promise<void>): Promise<void>;
    dispose(): void;
};

export interface CemStudioSourcePreviewState {
    readonly status: 'idle' | 'pending' | 'ready' | 'error';
    readonly stale: boolean;
    readonly preview?: import('./preview.js').CemStudioPreview;
    readonly error?: { readonly message: string };
}
export function createCemStudioSourcePreview(options?: { validator?: Pick<CemStudioBrowserValidator, 'runResourceCommand'>; delay?: number }): {
    snapshot(): CemStudioSourcePreviewState;
    setSource(source?: Readonly<{ resourceId: string; sourceUri: string; contentType: string; schema?: string; text: string; revision: number; sha256: string; draft: boolean; projectId?: string; projectRootUri?: string; projectRevision?: number }>): void;
    subscribe(notify: (state: CemStudioSourcePreviewState) => void): () => void;
    dispose(): void;
};
