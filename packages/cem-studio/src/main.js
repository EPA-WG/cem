import '@epa-wg/custom-element';

import {
    createCemStudioBrowserValidator,
    createCemStudioFileSystemProvider,
    createCemStudioFeatureTourWorkbench,
    createCemStudioProjectRepository,
    installCemStudioFeatureTour,
    loadCemStudioFeatureTour,
    mountCemStudio,
    mountCemStudioApplicationShell,
    mountCemStudioFeatureTourWorkbench,
    registerCemStudioServiceWorker,
} from '@epa-wg/cem-studio';

const mounted = mountCemStudio();
const registration = await registerCemStudioServiceWorker().catch(() => undefined);
const validator = await createCemStudioBrowserValidator();
const repository = createCemStudioProjectRepository({
    validateProject: validator.validateProject,
});
const seed = await loadCemStudioFeatureTour({
    baseUrl: new URL('./samples/feature-tour/', mounted.baseUrl),
    validator,
});
const featureTour = await installCemStudioFeatureTour(repository, seed);
const fileSystemProvider = createCemStudioFileSystemProvider({
    repository,
    decodeProjectManifest: validator.decodeProjectManifest,
    encodeProjectManifest: validator.encodeProjectManifest,
});
const shell = await mountCemStudioApplicationShell({
    root: mounted.root,
    registration,
    repository,
    fileSystem: {
        provider: fileSystemProvider,
        projectId: featureTour.projectId,
        selectImport: selectProjectArchive,
        downloadExport: downloadProjectArchive,
    },
});
const workbench = await createCemStudioFeatureTourWorkbench({
    repository,
    validator,
    seed,
    projectId: featureTour.projectId,
});
const workbenchView = await mountCemStudioFeatureTourWorkbench({
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
        repository,
        validator,
        seed,
        featureTour,
        fileSystemProvider,
        workbench,
        workbenchView,
    }),
});

function selectProjectArchive() {
    return new Promise((resolve, reject) => {
        const input = document.createElement('input');
        input.type = 'file';
        input.accept = 'application/vnd.cem.studio-project-bundle+json,.cem-studio.json';
        input.hidden = true;
        input.addEventListener('change', async () => {
            const file = input.files?.[0];
            input.remove();
            if (!file) {
                reject(new DOMException('project archive selection was cancelled', 'AbortError'));
                return;
            }
            resolve(new Uint8Array(await file.arrayBuffer()));
        }, { once: true });
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
