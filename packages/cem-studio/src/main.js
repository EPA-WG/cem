import '@epa-wg/custom-element';

import {
    createCemStudioBrowserValidator,
    createCemStudioFileSystemProvider,
    createCemStudioProjectWorkbench,
    createCemStudioProjectSession,
    createCemStudioSessionWorkbench,
    createCemStudioProjectRepository,
    installCemStudioFeatureTour,
    loadCemStudioFeatureTour,
    mountCemStudio,
    mountCemStudioApplicationShell,
    mountCemStudioFeatureTourWorkbench,
    parseCemStudioProjectBundle,
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
/** @typedef {{id: string, runConfigResourceId?: string}} PageRecord */
/** @typedef {{packageId?: string, contentType: string, runConfigResourceId?: string}} SampleRecord */
const examples = /** @type {SampleRecord[]} */ (seed.catalog.examples);
const scenarios = /** @type {SampleRecord[]} */ (seed.catalog.workbenches ?? []);
const session = createCemStudioProjectSession({
    storage: { getItem: key => globalThis.localStorage.getItem(key), setItem: (key, value) => globalThis.localStorage.setItem(key, value) },
    openWorkbench: async (selection, { signal }) => {
        // Optional sample expectations/portable scenarios do not own project records.
        const record = await repository.query({ protocolVersion: 1, repository: 'studio-projects',
            operation: 'get-project', requestRevision: 1, parameters: { projectId: selection.projectId } }, signal);
        const project = /** @type {{entries: PageRecord[]} | null} */ (record.value);
        const entry = project?.entries.find(item => item.id === selection.entryId);
        const sample = selection.projectId === featureTour.projectId
            ? [...examples, ...scenarios].find(item =>
                entry?.runConfigResourceId === item.runConfigResourceId) : undefined;
        return createCemStudioProjectWorkbench({ repository, validator, selection, signal, sample });
    },
});
const initial = examples.find(item => item.packageId === 'cem-ml' && item.contentType.includes('cem'));
const initialProject = await repository.query({ protocolVersion: 1, repository: 'studio-projects',
    operation: 'get-project', requestRevision: 1, parameters: { projectId: featureTour.projectId } });
const initialRecord = /** @type {{entries: PageRecord[]}} */ (initialProject.value);
const initialEntry = initialRecord.entries.find(item => item.runConfigResourceId === initial?.runConfigResourceId);
await session.recover({ projectId: featureTour.projectId, ...(initialEntry ? { entryId: initialEntry.id } : {}) });
// A bounded starter uses only the bundled source. Command pages are authored explicitly.
const starterResource = /** @type {{id: string}[]} */ (seed.bundle.project.resources).find(resource => resource.id === /** @type {{resourceId?: string}} */ (initial)?.resourceId);
if (!starterResource) throw new Error('Bundled CEM-ML starter is unavailable');
const starter = { project: { ...seed.bundle.project, entries: [], resources: [starterResource] },
    contents: { [starterResource.id]: seed.bundle.contents[starterResource.id] } };
const samplesUrl = new URL('./samples/', mounted.baseUrl);
const sampleResponse = await fetch(new URL('index.json', samplesUrl));
if (!sampleResponse.ok) throw new Error('Studio sample catalog is unavailable');
const sampleIndex = await sampleResponse.json();
if (sampleIndex.schemaVersion !== 1 || sampleIndex.commonVersion !== seed.catalog.commonVersion || !Array.isArray(sampleIndex.samples)) throw new Error('Studio sample catalog is incompatible');
/** @type {{id: string, name: string, archive?: string, sha256?: string}[]} */
const sampleEntries = sampleIndex.samples;
const samples = sampleEntries.filter(sample => typeof sample.archive === 'string').map(sample => {
    const archive = sample.archive;
    if (!archive || !sample.sha256 || typeof sample.id !== 'string' || typeof sample.name !== 'string') throw new Error('Studio sample metadata is invalid');
    return {
        id: sample.id, name: sample.name,
        async load() {
            const response = await fetch(new URL(archive, samplesUrl));
            if (!response.ok) throw new Error('The selected sample is unavailable');
            const bytes = new Uint8Array(await response.arrayBuffer());
            const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(byte => byte.toString(16).padStart(2, '0')).join('');
            if (digest !== sample.sha256) throw new Error('The selected sample failed its integrity check');
            return parseCemStudioProjectBundle(bytes);
        },
    };
});
const workbench = await createCemStudioSessionWorkbench({ repository, session,
    home: { starter, samples, provider: fileSystemProvider, selectImport: selectProjectArchive } });
const shell = await mountCemStudioApplicationShell({
    root: mounted.root,
    registration,
    repository,
    persistState: async () => {
        throw Object.assign(new Error('Save source and Apply or reset the command draft before updating.'), { code: 'unsaved-work' });
    },
    fileSystem: {
        provider: fileSystemProvider,
        projectId: featureTour.projectId,
        selectImport: selectProjectArchive,
        downloadExport: downloadProjectArchive,
    },
});
/** @type {string | undefined} */
let providerSelection;
const unsubscribeUpdateState = workbench.subscribe((snapshot) => {
    const identity = JSON.stringify([snapshot.projectId, snapshot.resourceId]);
    if (identity !== providerSelection) {
        providerSelection = identity;
        void shell.fileSystem.setProject(snapshot.projectId, snapshot.resourceId || undefined).catch(() => undefined);
    }
    shell.update.setDirty(snapshot.dirty || Boolean(snapshot.command && snapshot.command.draftText !== snapshot.command.current.text));
    const active =
        snapshot.structure?.busy || snapshot.home?.status === 'working' || Boolean(snapshot.transition) || ['opening', 'settling'].includes(snapshot.sessionStatus ?? '') ||
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
        shell.update.setActiveRequestCount(state.structure?.busy || state.home?.status === 'working' || state.transition || ['opening', 'settling'].includes(state.sessionStatus ?? '') || ['loading', 'saving', 'validating', 'projecting'].includes(state.status) || ['applying', 'running'].includes(state.command?.application?.status ?? '') ? 1 : 0);
    }
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
        session,
        repository,
        validator,
        seed,
        featureTour,
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
