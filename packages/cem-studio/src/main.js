import { CemElementRuntime, CemNativeRecoveryScope } from '@epa-wg/cem-elements';
import { createCemStudioNavigation } from './navigation.js';
import '@epa-wg/custom-element';

import {
    createCemStudioBrowserValidator,
    createCemStudioFileSystemProvider,
    createCemStudioProjectWorkbench,
    createCemStudioProjectSession,
    createCemStudioSessionWorkbench,
    createCemStudioProjectRepository,
    loadCemStudioExampleProjects,
    mountCemStudio,
    installCemStudioShellComponents,
    mountCemStudioApplicationShell,
    mountCemStudioWorkbench,
    registerCemStudioServiceWorker,
} from '@epa-wg/cem-studio';

const mounted = mountCemStudio();
const registration = await registerCemStudioServiceWorker().catch(() => undefined);
const recoveryScope = new CemNativeRecoveryScope();
const validator = await createCemStudioBrowserValidator({ recoveryScope });
await installCemStudioShellComponents({ runtime: new CemElementRuntime({ declarationTag: 'cem-studio-component-declaration', recoveryScope }) });
const repository = createCemStudioProjectRepository({
    validateProject: validator.validateProject,
});
const exampleProjects = await loadCemStudioExampleProjects({
    baseUrl: new URL('./samples/', mounted.baseUrl), commonVersion: validator.commonVersion,
});
const fileSystemProvider = createCemStudioFileSystemProvider({
    repository,
    decodeProjectManifest: validator.decodeProjectManifest,
    encodeProjectManifest: validator.encodeProjectManifest,
});
const session = createCemStudioProjectSession({
    storage: { getItem: key => globalThis.localStorage.getItem(key), setItem: (key, value) => globalThis.localStorage.setItem(key, value) },
    openWorkbench: (selection, { signal }) => createCemStudioProjectWorkbench({ repository, validator, selection, signal }),
});
const starter = await exampleProjects.starter.load();
const sessionWorkbench = await createCemStudioSessionWorkbench({ repository, session,
    home: { starter, examples: exampleProjects.examples, collections: Array.isArray(exampleProjects.catalog.collections) ? exampleProjects.catalog.collections : [], provider: fileSystemProvider, selectImport: selectProjectArchive } });
const workbench = await createCemStudioNavigation({ workbench: sessionWorkbench, repository, validator, examples: exampleProjects.examples, collections: Array.isArray(exampleProjects.catalog.collections) ? exampleProjects.catalog.collections : [], browser: globalThis });
const shell = await mountCemStudioApplicationShell({
    root: mounted.root,
    registration,
    repository,
    persistState: async () => {
        throw Object.assign(new Error('Save source and Apply or reset the command draft before updating.'), { code: 'unsaved-work' });
    },
    fileSystem: {
        provider: fileSystemProvider,
        selectImport: selectProjectArchive,
        downloadExport: downloadProjectArchive,
    },
});
/** @type {string | undefined} */
let providerSelection;
const unsubscribeUpdateState = workbench.subscribe((snapshot) => {
    shell.setWorkspaceState(snapshot.navigation.title, snapshot.navigation.status, {
        projectName: snapshot.navigation.projectName, path: snapshot.navigation.path, pathKind: snapshot.navigation.pathKind,
    });
    document.title = snapshot.navigation.title;
    const identity = JSON.stringify([snapshot.projectId, snapshot.resourceId]);
    if (identity !== providerSelection) {
        providerSelection = identity;
        void shell.fileSystem.setProject(snapshot.projectId, snapshot.resourceId || undefined).catch(() => undefined);
    }
    shell.update.setDirty(snapshot.dirty || Boolean(snapshot.command && snapshot.command.draftText !== snapshot.command.current.text));
    const active =
        snapshot.projectPreview?.busy || snapshot.structure?.busy || snapshot.home?.status === 'working' || Boolean(snapshot.transition) || ['opening', 'settling'].includes(snapshot.sessionStatus ?? '') ||
        Boolean(shell.fileSystem.status().operation) ||
        ['loading', 'saving', 'validating', 'projecting'].includes(snapshot.status) ||
        snapshot.command?.application?.status === 'applying' ||
        snapshot.command?.application?.status === 'running';
    shell.update.setActiveRequestCount(active ? 1 : 0);
});
const unsubscribeProvider = shell.fileSystem.subscribe(status => {
    if (status.operation) shell.update.setActiveRequestCount(1);
    else {
        const state = workbench.snapshot();
        shell.update.setActiveRequestCount(state.projectPreview?.busy || state.structure?.busy || state.home?.status === 'working' || state.transition || ['opening', 'settling'].includes(state.sessionStatus ?? '') || ['loading', 'saving', 'validating', 'projecting'].includes(state.status) || ['applying', 'running'].includes(state.command?.application?.status ?? '') ? 1 : 0);
    }
});
const workbenchView = await mountCemStudioWorkbench({
    root: mounted.root,
    workbench,
});

Object.defineProperty(globalThis, '__cemStudioBootstrap', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({
        mounted: true,
        baseUrl: mounted.baseUrl.href,
    }),
});

Object.defineProperty(globalThis, '__cemStudioApplication', {
    configurable: false,
    enumerable: false,
    writable: false,
    value: Object.freeze({
        shell,
        session,
        repository,
        validator,
        exampleProjects,
        fileSystemProvider,
        workbench,
        workbenchView,
        unsubscribeUpdateState,
        unsubscribeProvider,
    }),
});

function selectProjectArchive() {
    return new Promise((resolve, reject) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = 'application/vnd.cem.studio-project-bundle+json,.cem-studio.json';
        input.hidden = true;
        input.addEventListener('cancel', () => { input.remove(); reject(new DOMException('project archive selection was cancelled', 'AbortError')); }, { once: true });
        input.addEventListener(
            'change',
            async () => {
                const file = input.files?.[0];
                input.remove();
                if (!file) {
                    reject(new DOMException('project archive selection was cancelled', 'AbortError'));
                    return;
                }
                try { resolve(new Uint8Array(await file.arrayBuffer())); } catch (error) { reject(error); }
            },
            { once: true },
        );
        document.body.append(input);
        input.click();
    });
}

/** @param {{filename: string, contentType: string, bytes: Uint8Array}} archive */
async function downloadProjectArchive(archive) {
    const bytes = new Uint8Array(archive.bytes.byteLength);
    bytes.set(archive.bytes);
    const url = URL.createObjectURL(new Blob([bytes.buffer], { type: archive.contentType }));
    try {
        const link = document.createElement('a');
        link.href = url;
        link.download = archive.filename;
        link.click();
    } finally {
        URL.revokeObjectURL(url);
    }
}
