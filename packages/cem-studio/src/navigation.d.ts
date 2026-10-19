import type { CemStudioFeatureTourWorkbench, CemStudioWorkbenchState } from './workbench.js';
import type { CemStudioIndexedDbRepository } from './repository.js';
import type { CemStudioExampleProject } from './file-system-provider.js';

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
}
export function createCemStudioNavigation(options: {
    workbench: CemStudioFeatureTourWorkbench;
    repository: CemStudioIndexedDbRepository;
    examples?: readonly CemStudioExampleProject[];
    collections?: readonly { id: string; name: string; description?: string }[];
    browser?: Pick<Window, 'location' | 'history' | 'addEventListener' | 'removeEventListener'>;
}): Promise<CemStudioNavigation>;
