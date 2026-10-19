import { projectResourceUri } from './resource-reference.js';
import { describeCemStudioOperationFields, describeCemStudioOperationGroups, editCemStudioOperationField, editCemStudioOperationFields } from './feature-tour.js';
import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { CEM_STUDIO_LIMITS, createCemStudioPreview, createCemStudioLivePreview, redactCemStudioSecrets } from './preview.js';
import { installCemStudioShellComponents } from './shell.js';
import { mountCemView } from '@epa-wg/cem-elements';

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

export const CEM_STUDIO_PARSE_PROJECTIONS = Object.freeze(['ast', 'events']);
export const CEM_STUDIO_INSPECT_VIEWS = Object.freeze([
    'summary',
    'ast',
    'events',
    'diagnostics',
    'source-offsets',
    'tree',
]);

/** Resolve durable project records. A catalog is never required to open a project. */
export async function createCemStudioProjectWorkbench({ repository, validator, selection, signal, sample }) {
    signal?.throwIfAborted();
    const projects = await repository.query(repositoryRequest('list-projects', {}), signal);
    if (!projects.value.some(project => project.id === selection.projectId)) throw new Error('Selected project is missing or in Trash. Restore it from Home.');
    const response = await repository.query(repositoryRequest('export-project', { projectId: selection.projectId }), signal);
    const bundle = response.value;
    if (!bundle?.project) throw new Error(`Project ${selection.projectId} is unavailable`);
    const project = bundle.project;
    const entry = selection.entryId === undefined ? undefined : project.entries.find(item => item.id === selection.entryId);
    if (selection.entryId !== undefined && !entry) throw new Error(`Configuration ${selection.entryId} is missing`);
    const commandRecord = project.resources.find(item => item.id === entry?.runConfigResourceId);
    let inputResourceId;
    if (selection.resourceId === undefined && commandRecord?.schema === 'https://cem.dev/ns/cli/command/1') {
        const parsed = validator.decodeResourceCommand(toBytes(bundle.contents[commandRecord.id]));
        const inputs = parsed.positionals?.input ?? parsed.positionals?.inputs ?? parsed.options?.config;
        const input = Array.isArray(inputs) ? inputs[0] : inputs;
        if (typeof input === 'string') {
            inputResourceId = project.resources.find(item => projectResourceUri(project, item) === resolveVirtualUri(project.rootUri, input))?.id;
            if (!inputResourceId || !entry.resourceIds?.includes(inputResourceId)) throw new Error(`Command input ${input} is not owned by this configuration`);
        }
    }
    const resourceId = selection.resourceId ?? inputResourceId ?? entry?.resourceIds?.find(id => id !== entry.runConfigResourceId);
    const resource = resourceId === undefined ? undefined : project.resources.find(item => item.id === resourceId);
    if (resourceId !== undefined && !resource) throw new Error(`Resource ${resourceId} is missing`);
    if (entry && resource && !entry.resourceIds?.includes(resource.id) && entry.runConfigResourceId !== resource.id) {
        throw new Error(`Resource ${resource.id} does not belong to configuration ${entry.id}`);
    }
    const available = Boolean(resource && bundle.contents[resource.id] !== undefined);
    const bytes = available ? toBytes(bundle.contents[resource.id]) : new Uint8Array();
    let text;
    const mediaType = resource?.contentType?.split(';')[0].trim().toLowerCase();
    const supported = available && ['inline', 'project-file'].includes(resource.sourceKind)
        && (mediaType.startsWith('text/') || mediaType.startsWith('application/vnd.cem.') || ['application/cem', 'application/json', 'application/xml', 'application/yaml', 'application/xhtml+xml', 'image/svg+xml'].includes(mediaType))
        && bytes.byteLength <= CEM_STUDIO_LIMITS.sourceBytes;
    if (supported) {
        try { text = new TextDecoder('utf-8', { fatal: true }).decode(bytes); } catch { /* binary stays read-only */ }
    }
    if (text === undefined) {
        const state = Object.freeze({ status: resource ? 'read-only' : 'empty', projectId: project.id,
            entryId: entry?.id, resourceId: resource?.id ?? '', path: resource?.path ?? '',
            contentType: resource?.contentType ?? '', schema: resource?.schema ?? '',
            projectRevision: project.revision, resourceRevision: resource?.revision ?? 0,
            repositoryRevision: response.repositoryRevision, draft: '', persistedText: '', dirty: false,
            editable: false, runnable: false, workbenchId: '', workbenches: [], operation: '',
            resourceAvailable: available, byteLength: bytes.byteLength });
        return Object.freeze({ snapshot: () => state, subscribe(notify) { notify(state); return () => { /* Immutable snapshot has no live subscription. */ }; },
            dispose() { /* No requests or subscriptions are retained. */ }, async downloadSource(download = browserDownload) {
                if (!available) throw new Error('No local resource bytes are available');
                await download({ filename: resource.path?.split('/').pop() ?? resource.id, contentType: resource.contentType, bytes: bytes.slice() });
            } });
    }
    const example = { ...sample, id: entry?.id ?? resource.id, name: entry?.name ?? resource.path,
        resourceId: resource.id, referenceUri: projectResourceUri(project, resource), projectRootUri: project.rootUri, path: resource.path, contentType: resource.contentType, schema: resource.schema,
        description: entry?.description,
        expectedResourceId: entry?.resourceIds?.find(id => project.resources.find(resource => resource.id === id)?.role === 'expected'),
        entryId: entry?.id, runConfigResourceId: entry?.runConfigResourceId, kind: entry?.kind,
        graphConfiguration: resource.role === 'transform-config',
        dependencies: (entry?.resourceIds ?? []).filter(id => id !== resource.id && id !== entry.runConfigResourceId).map(resourceId => ({ resourceId, path: projectResourceUri(project, project.resources.find(item => item.id === resourceId)) })), projectOwned: true };
    const workbench = await createCemStudioWorkbench({ repository, validator, projectId: project.id, example });
    if (signal?.aborted) { workbench.dispose(); signal.throwIfAborted(); }
    return workbench;
}

/** A stable service adapter; all selection requests go through guarded admission. */
export async function createCemStudioSessionWorkbench({ repository, session, home }) {
    let choices = [];
    let navigationError;
    let records = [];
    let searchState = { query: '', status: 'idle', results: [] };
    let searchVersion = 0;
    let navigationVersion = 0;
    let disposed = false;
    let homeState = home ? { projects: [], status: 'ready', collections: home.collections ?? [], examples: (home.examples ?? []).map(({ id, name, collections }) => ({ id, name, collections: collections ?? [] })) } : undefined;
    let homePending = false;
    let structurePending = false;
    let structureMessage;
    let structureError;
    let projectRecords = new Map();
    let provenanceNavigation;
    const histories = new Map();
    const subscribers = new Set();
    const identity = selection => JSON.stringify(selection);
    function snapshot() {
        const current = session.snapshot();
        const record = projectRecords.get(current.selection?.projectId);
        // Repository refresh can precede guarded admission of its new revision.
        // Derive notices from the current session rather than retaining that gap.
        const available = !current.selection || choices.some(item => item.id === identity(current.selection));
        const selectionError = !available ? { code: 'cem.studio.selection.unavailable', message: 'The active selection is missing or in Trash. Your drafts are retained; restore the project or choose another workbench.' }
            : current.workbench && record?.revision > current.workbench.projectRevision
                ? { code: 'cem.studio.selection.changed', message: 'The project changed in another operation or tab. Your drafts are retained; saving checks the original revision. Reload explicitly to use persisted bytes.' }
                : current.recoveryError;
        const history = histories.get(current.selection?.projectId);
        const structure = record && { projectId: record.id, name: record.name, revision: record.revision,
            entries: record.entries, resources: record.resources,
            parents: record.entries.filter(entry => entry.kind === 'subproject'),
            busy: structurePending, message: structureMessage, error: structureError,
            canUndo: Boolean(history?.undo.length && history.revision === record.revision),
            canRedo: Boolean(history?.redo.length && history.revision === record.revision) };
        return Object.freeze({ status: 'empty', editable: false, runnable: false, ...current.workbench, transition: current.transition, sessionStatus: current.status, draftRecovery: current.draftRecovery, structure, navigationError: navigationError ?? selectionError, home: homeState, explorer: { nodes: records, selected: current.selection ? encodeURIComponent(identity(current.selection)) : '', search: searchState },
            workbenchId: current.selection ? identity(current.selection) : '', workbenches: choices });
    }
    function publish() { for (const notify of subscribers) notify(snapshot()); }
    async function refreshNavigation() {
        if (disposed) return;
        const version = ++navigationVersion;
        try {
            const response = await repository.query(repositoryRequest('list-projects', {}));
            if (disposed || version !== navigationVersion) return;
            const projects = response.value;
            const next = [];
            const trees = [];
            const loadedProjects = new Map();
            for (const project of projects) {
                const loaded = await repository.query(repositoryRequest('get-project', { projectId: project.id }));
                if (disposed || version !== navigationVersion) return;
                if (!loaded.value) continue;
                const record = loaded.value;
                loadedProjects.set(project.id, record);
                const key = selection => encodeURIComponent(identity(selection));
                const entries = new Map(record.entries.map(entry => [entry.id, { key: key({ projectId: project.id, entryId: entry.id }), name: entry.name, children: [] }]));
                const roots = [];
                for (const entry of record.entries) {
                    const node = entries.get(entry.id);
                    const parent = entries.get(entry.parentId);
                    if (parent) parent.children.push(node); else roots.push(node);
                }
                // Branch activation belongs to expansion; a branch's own configuration is an explicit leaf.
                for (const node of entries.values()) {
                    if (node.children.length) {
                        node.children.unshift({ key: node.key, name: node.name, children: [] });
                        node.key = `branch:${node.key}`;
                    }
                }
                if (record.resources.length) roots.push({ key: `resources:${key({ projectId: project.id })}`, name: 'Resources', children: record.resources.map(resource => ({ key: key({ projectId: project.id, resourceId: resource.id }), name: resource.path ?? resource.id, children: [] })) });
                trees.push({ key: `project:${key({ projectId: project.id })}`, name: project.name, children: [{ key: key({ projectId: project.id }), name: project.name, children: [] }, ...roots] });
                const add = (selection, name, operation = '') => next.push(Object.freeze({ id: identity(selection), selection: Object.freeze(selection), name, operation }));
                add({ projectId: project.id }, project.name);
                for (const entry of record.entries) {
                    const names = [entry.name];
                    const seen = new Set([entry.id]);
                    let parent = record.entries.find(item => item.id === entry.parentId);
                    while (parent && !seen.has(parent.id)) {
                        seen.add(parent.id); names.unshift(parent.name);
                        parent = record.entries.find(item => item.id === parent.parentId);
                    }
                    add({ projectId: project.id, entryId: entry.id }, `${project.name} / ${names.join(' / ')}`, entry.kind);
                }
                for (const resource of record.resources) {
                    add({ projectId: project.id, resourceId: resource.id }, `${project.name} / ${resource.path ?? resource.id}`);
                }
            }
            const all = homeState ? (await repository.query(repositoryRequest('list-projects', { includeTrash: true }))).value : [];
            if (disposed || version !== navigationVersion) return;
            const storage = homeState ? await repository.status() : undefined;
            if (disposed || version !== navigationVersion) return;
            records = trees;
            projectRecords = loadedProjects;
            choices = Object.freeze(next);
            navigationError = undefined;
            if (homeState && storage) {
                homeState = { ...homeState, trash: all.filter(project => project.trashedAt).map(project => ({ id: project.id, name: project.name, revision: project.revision })), projects: projects.map(project => ({ id: identity({ projectId: project.id }), key: encodeURIComponent(identity({ projectId: project.id })), name: project.name })),
                    storage: storage.persisted ? 'Persistent browser storage' : 'Browser storage may be cleared; export projects to keep a backup.',
                    usage: storage.usage, quota: storage.quota };
            }
            publish();
        } catch (error) { if (!disposed && version === navigationVersion) { navigationError = normalizedError(error); publish(); } throw error; }
    }
    async function searchProjects(query) {
        const version = ++searchVersion;
        const text = String(query ?? '').trim().slice(0, 500);
        searchState = { query: text, status: text ? 'searching' : 'idle', results: [] }; publish();
        if (!text) return;
        try {
            const response = await repository.query(repositoryRequest('search', { query: text, limit: 50 }));
            if (disposed || version !== searchVersion) return;
            searchState = { query: text, status: 'ready', results: response.value.map(result => ({ ...result,
                key: encodeURIComponent(identity({ projectId: result.projectId, ...(result.kind === 'entry' ? { entryId: result.entityId } : result.kind === 'resource' ? { resourceId: result.entityId } : {}) })) })) };
        } catch (error) {
            if (disposed || version !== searchVersion) return;
            searchState = { query: text, status: 'failed', results: [], error: normalizedError(error) };
        }
        publish();
    }
    const unsubscribe = session.subscribe(publish);
    const unsubscribeRepository = repository.subscribe?.(0, () => {
        void refreshNavigation().catch(() => undefined);
        if (searchState.query) void searchProjects(searchState.query);
    });
    await refreshNavigation().catch(() => undefined);
    async function homeOperation(operation) {
        if (!home || homePending) return;
        homePending = true;
        homeState = { ...homeState, status: 'working', error: undefined, message: undefined }; publish();
        try {
            const project = await operation();
            homeState = { ...homeState, status: 'ready', message: `Added ${project.name}. Choose Open project to enter it.` };
            await refreshNavigation();
        } catch (error) {
            homeState = { ...homeState, status: 'ready', ...(error?.name === 'AbortError' ? {} : { error: normalizedError(error) }) };
        } finally { homePending = false; publish(); }
    }
    async function copyProject(template, name) {
        const bundle = structuredClone(template);
        const id = `project-${crypto.randomUUID()}`;
        const now = new Date().toISOString();
        bundle.project = { ...bundle.project, id, name, rootUri: `studio://${id}/`, revision: 1, createdAt: now, updatedAt: now };
        return (await repository.execute(repositoryRequest('import-project', { bundle, mode: 'create' }))).value;
    }
    async function editStructure(edit) {
        if (structurePending) return snapshot();
        const current = session.snapshot();
        const selection = current.selection;
        const loaded = current.workbench;
        const record = projectRecords.get(selection?.projectId);
        const dirty = loaded?.dirty || (loaded?.command && loaded.command.draftText !== loaded.command.current.text);
        structureError = undefined; structureMessage = undefined;
        if (!record || current.transition || current.pendingWrite || dirty || workbenchBusy(loaded ?? {})) {
            structureError = { message: 'Finish pending work and Save/Apply or reset drafts before editing project structure.' };
            publish(); return snapshot();
        }
        structurePending = true; publish();
        const projectId = record.id;
        const oldHistory = histories.get(projectId);
        const history = oldHistory?.revision === record.revision ? oldHistory : { revision: record.revision, undo: [], redo: [] };
        try {
            const replay = edit.kind === 'undo' || edit.kind === 'redo';
            const from = edit.kind === 'undo' ? history.undo : history.redo;
            if (replay && !from.length) throw new Error('History is unavailable after a new revision or reload.');
            const response = await repository.execute(repositoryRequest('edit-project', { projectId, expectedRevision: record.revision,
                edit: replay ? { kind: 'restore', bundle: from.at(-1) } : edit }));
            const before = response.value.before;
            if (replay) { from.pop(); (edit.kind === 'undo' ? history.redo : history.undo).push(before); }
            else { history.undo.push(before); history.redo = []; }
            // Bound history by count and actual retained source bytes plus metadata.
            const size = bundle => JSON.stringify(bundle.project).length * 2 + Object.values(bundle.contents).reduce((sum, bytes) => sum + bytes.byteLength, 0);
            while (history.undo.length + history.redo.length > 20 || [...history.undo, ...history.redo].reduce((sum, bundle) => sum + size(bundle), 0) > 8 * 1024 * 1024) {
                if (history.undo.length) history.undo.shift(); else history.redo.shift();
            }
            history.revision = response.value.project.revision;
            histories.set(projectId, history);
            structureMessage = 'Project structure saved. Undo history is local to this session.';
            await refreshNavigation();
            if (searchState.query) await searchProjects(searchState.query);
            if (session.snapshot().selection === selection) {
                const next = response.value.project;
                if ((selection.entryId && !next.entries.some(entry => entry.id === selection.entryId)) ||
                    (selection.resourceId && !next.resources.some(resource => resource.id === selection.resourceId))) await session.select({ projectId });
                else await session.refresh();
            }
        } catch (error) { structureError = normalizedError(error); }
        finally { structurePending = false; publish(); }
        return snapshot();
    }
    async function finishProvenanceNavigation() {
        const pending = provenanceNavigation;
        const current = session.snapshot();
        if (!pending || current.transition) return snapshot();
        provenanceNavigation = undefined;
        if (current.selection?.projectId !== pending.projectId || current.selection?.resourceId !== pending.resourceId) return snapshot();
        try {
            const project = (await repository.query(repositoryRequest('get-project', { projectId: pending.projectId }))).value;
            if (session.snapshot().selection !== current.selection || project?.revision !== pending.projectRevision) {
                throw new Error('Source provenance is stale. Rerun the configuration before following it.');
            }
            session.activeWorkbench().selectSourceRange(pending.range, pending.projectRevision, pending.index);
        } catch (error) { navigationError = normalizedError(error); publish(); }
        return snapshot();
    }
    async function navigateProvenance(index) {
        const current = session.snapshot();
        const state = current.workbench;
        const projection = state?.projection ?? state?.validation;
        const frame = projection?.provenance?.[index];
        provenanceNavigation = undefined;
        navigationError = undefined;
        try {
            if (!frame) throw new RangeError(`provenance frame ${index} is unavailable`);
            if (!frame.uri) {
                session.activeWorkbench().navigateProvenance(index);
                return snapshot();
            }
            const project = (await repository.query(repositoryRequest('get-project', { projectId: current.selection.projectId }))).value;
            if (session.snapshot().selection !== current.selection || project?.revision !== projection.revision.projectRevision) {
                throw new Error('Source provenance is stale. Rerun the configuration before following it.');
            }
            const resource = project.resources.find(resource => projectResourceUri(project, resource) === frame.uri);
            if (!resource) throw new Error('The provenance source is not an available resource in this project.');
            provenanceNavigation = { projectId: project.id, resourceId: resource.id, projectRevision: project.revision, range: frame.range, index };
            await session.select({ projectId: project.id, resourceId: resource.id });
            return await finishProvenanceNavigation();
        } catch (error) { provenanceNavigation = undefined; navigationError = normalizedError(error); publish(); return snapshot(); }
    }
    const adapter = { snapshot, subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        async selectWorkbench(id) {
            provenanceNavigation = undefined;
            const target = choices.find(item => item.id === id);
            if (!target) throw new Error('The selected workbench is unavailable');
            await session.select(target.selection);
            return snapshot();
        },
        searchProjects,
        editStructure,
        clearDraftRecovery() { session.clearDraftRecovery(); return snapshot(); },
        async downloadDraftRecovery(save) {
            const text = session.draftBackup();
            if (text) await save({ filename: 'cem-studio-draft-recovery.json', contentType: 'application/json', bytes: new TextEncoder().encode(text) });
        },
        async openEntity(key) {
            provenanceNavigation = undefined;
            const target = choices.find(item => encodeURIComponent(item.id) === key);
            if (!target) { navigationError = normalizedError(new Error('The requested entity is unavailable. Refresh projects.')); publish(); return snapshot(); }
            await session.select(target.selection);
            return snapshot();
        },
        async resolveNavigation(decision) {
            if (decision === 'cancel') provenanceNavigation = undefined;
            const transition = session.snapshot().transition;
            if (transition) await session.resolveTransition(transition.id, decision);
            await refreshNavigation().catch(() => undefined);
            await finishProvenanceNavigation();
            return session.snapshot();
        },
        navigateProvenance,
        createProject(name) {
            return homeOperation(async () => {
                if (typeof name !== 'string' || !name.trim() || name.trim().length > 120) throw new Error('Enter a project name of 1–120 characters.');
                return copyProject(home.starter, name.trim());
            });
        },
        createExampleProject(id) {
            return homeOperation(async () => {
                const example = home.examples?.find(example => example.id === id);
                if (!example) throw new Error('Choose an available example project.');
                const template = await example.load();
                const projects = (await repository.query(repositoryRequest('list-projects', { includeTrash: true }))).value;
                const names = new Set(projects.map(project => project.name));
                let name = example.name;
                for (let ordinal = 2; names.has(name); ordinal++) name = `${example.name} (${ordinal})`;
                return copyProject(template, name);
            });
        },
        importProject() {
            return homeOperation(async () => (await home.provider.importFallback({ archive: await home.selectImport(), mode: 'create' })).project);
        },
        async setProjectTrashed(id, trashed) {
            if (homePending) return;
            homePending = true;
            if (homeState) homeState = { ...homeState, status: 'working' };
            publish();
            try {
                const all = (await repository.query(repositoryRequest('list-projects', { includeTrash: true }))).value;
                const project = all.find(project => project.id === id);
                if (!project) throw new Error('Project is unavailable');
                await repository.execute(repositoryRequest(trashed ? 'trash-project' : 'restore-project', { projectId: id, expectedRevision: project.revision }));
                await refreshNavigation();
                if (searchState.query) await searchProjects(searchState.query);
            } catch (error) { navigationError = normalizedError(error); publish(); throw error; }
            finally { homePending = false; if (homeState) homeState = { ...homeState, status: 'ready' }; publish(); }
        },
        refreshNavigation, dispose() { unsubscribeRepository?.(); disposed = true; ++searchVersion; ++navigationVersion; unsubscribe(); subscribers.clear(); } };
    for (const method of ['updateDraft', 'updateGraphAttribute', 'createGraphNode', 'updateGraphEdge', 'undoGraphDraft', 'redoGraphDraft', 'reload', 'saveSource', 'saveAndValidate', 'validatePersisted', 'parsePersisted',
        'inspectPersisted', 'runPersistedOperation', 'updateCommandDraft', 'updateCommandField', 'updateCommandFields', 'updateCommandGroup', 'resetCommandDraft', 'copyCommand',
        'copyProjection', 'downloadProjection', 'selectArtifact', 'downloadSource', 'setCommandTarget', 'applyCommand', 'applyAndRun',
        'confirmCommandReplacement', 'useNewCommandTarget', 'cancelCommandReplacement', 'navigateDiagnostic', 'selectSourceRange']) {
        adapter[method] = (...args) => {
            const active = session.activeWorkbench();
            if (typeof active?.[method] !== 'function') throw new Error('This resource does not support that operation');
            const result = active[method](...args);
            if (['saveSource', 'saveAndValidate', 'applyCommand', 'applyAndRun', 'confirmCommandReplacement'].includes(method)) {
                return Promise.resolve(result).then(async value => { await refreshNavigation().catch(() => undefined); if (searchState.query) await searchProjects(searchState.query); return value; });
            }
            return result;
        };
    }
    return Object.freeze(adapter);
}

/**
 * Own one editable project resource and its durable validation revision.
 * @param {{repository: object, validator: object, seed?: object, projectId: string, example?: object}} options
 */
export async function createCemStudioWorkbench(options) {
    const { repository, validator, seed, projectId } = options;
    if (!repository?.query || !repository?.execute) throw new TypeError('Studio workbench requires a repository');
    if (
        !validator?.validateResource ||
        !validator?.parseResource ||
        !validator?.inspectResource ||
        !validator?.previewResourceCommand ||
        !validator?.serializeResourceCommand ||
        !validator?.executeAuthoredResourceCommand
    ) {
        throw new TypeError('Studio workbench requires a browser command validator');
    }
    if (typeof projectId !== 'string' || projectId.length === 0) {
        throw new TypeError('Studio workbench requires a project id');
    }
    const initialExample = options.example ?? featureTourCemExample(seed);
    const workbenches = featureTourWorkbenches(seed, initialExample);
    if (
        workbenches.some((entry) => !['parse', 'inspect'].includes(workbenchOperation(entry))) &&
        !validator?.runResourceCommand
    ) {
        throw new TypeError('Portable operation workbenches require a browser command runner');
    }
    let [example] = workbenches;
    const subscribers = new Set();
    let editVersion = 0;
    let commandEditVersion = 0;
    const commandFormId = crypto.randomUUID();
    let graphCache;
    const graphUndo = [], graphRedo = [];
    const isGraphConfiguration = () => typeof validator.authorGraph === 'function' && (example.graphConfiguration || example.schema === 'https://cem.dev/ns/cli/transform-config/1' || (!example.projectOwned && example.kind === 'transformation-graph'));
    let commandTargetVersion = 0;
    let state = freezeState({
        status: 'loading',
        projectId,
        entryId: example.entryId,
        editable: true,
        runnable: !example.projectOwned,
        resourceId: example.resourceId,
        path: example.path,
        description: example.description,
        contentType: example.contentType,
        schema: example.schema,
        projectRevision: 0,
        resourceRevision: 0,
        repositoryRevision: 0,
        persistedText: '',
        draft: '',
        dirty: false,
        validation: undefined,
        projection: undefined,
        command: undefined,
        selection: undefined,
        error: undefined,
        workbenchId: example.id,
        workbenches: Object.freeze(workbenches.map(workbenchDescriptor)),
        operation: workbenchOperation(example),
        expected: undefined,
    });

    function snapshot() {
        return state;
    }

    function publish(next) {
        if (next.command) next = { ...next, command: { ...next.command, formVersion: `${commandFormId}:${commandEditVersion}` } };
        if (isGraphConfiguration()) {
            if (graphCache?.source !== next.draft) {
                try {
                    const { nodes, parents, nodeTypes } = validator.authorGraph({ source: next.draft });
                    graphCache = { source: next.draft, nodes, parents, nodeTypes };
                } catch (error) { graphCache = { source: next.draft, nodes: [], parents: [], nodeTypes: [], error: normalizedError(error) }; }
            }
            next = { ...next, graphConfiguration: { nodes: graphCache.nodes, parents: graphCache.parents, nodeTypes: graphCache.nodeTypes, error: next.graphConfiguration?.error ?? graphCache.error,
                version: `${commandFormId}:source:${editVersion}`, canUndo: graphUndo.length > 0, canRedo: graphRedo.length > 0 } };
        } else next = { ...next, graphConfiguration: undefined };
        state = freezeState(next);
        for (const notify of subscribers) notify(state);
        return state;
    }

    function subscribe(notify) {
        if (typeof notify !== 'function') throw new TypeError('workbench subscriber must be a function');
        subscribers.add(notify);
        notify(state);
        return () => subscribers.delete(notify);
    }

    async function readPersisted() {
        const response = await repository.query(repositoryRequest('export-project', { projectId }));
        const bundle = response.value;
        if (!bundle?.project || !bundle?.contents) throw new Error(`Project ${projectId} is unavailable`);
        const resource = bundle.project.resources?.find(({ id }) => id === example.resourceId);
        if (!resource) throw new Error(`Resource ${example.resourceId} is unavailable`);
        const currentEntries = bundle.project.entries.filter(
            ({ id, runConfigResourceId, resourceIds }) => example.projectOwned ? id === example.entryId :
                runConfigResourceId === example.runConfigResourceId ||
                (!example.runConfigResourceId && resourceIds?.includes(example.resourceId)),
        );
        if (currentEntries.length > 1) {
            throw new Error(`Resource ${example.resourceId} resolves to more than one current configuration`);
        }
        example = { ...example, path: resource.path, referenceUri: projectResourceUri(bundle.project, resource) };
        const bytes = toBytes(bundle.contents[example.resourceId]);
        if (example.projectOwned && example.entryId && currentEntries.length !== 1) throw new Error(`Configuration ${example.entryId} is missing`);
        const dependencyDescriptors = example.projectOwned
            ? (currentEntries[0]?.resourceIds ?? []).filter(id => id !== example.resourceId && id !== currentEntries[0]?.runConfigResourceId)
                .map(resourceId => ({ resourceId, path: projectResourceUri(bundle.project, bundle.project.resources.find(item => item.id === resourceId)) }))
            : example.dependencies;
        const dependencies = dependencyDescriptors.map((dependency) => {
            const dependencyResource = bundle.project.resources.find(({ id }) => id === dependency.resourceId);
            if (!dependencyResource) throw new Error(`Dependency ${dependency.resourceId} is unavailable`);
            return {
                bytes: toBytes(bundle.contents[dependency.resourceId]),
                contentType: dependencyResource.contentType,
                schema: dependencyResource.schema,
                path: dependency.path,
                resourceId: dependency.resourceId,
            };
        });
        const expected = freezeWorkbenchValue(
            example.expectedSummary ??
                (example.expectedResourceId
                    ? JSON.parse(textDecoder.decode(toBytes(bundle.contents[example.expectedResourceId])))
                    : undefined),
        );
        const commandRecord = example.projectOwned && bundle.project.resources.find(item => item.id === currentEntries[0]?.runConfigResourceId);
        return {
            commandBytes: commandRecord?.schema === 'https://cem.dev/ns/cli/command/1' ? toBytes(bundle.contents[commandRecord.id]) : undefined,
            bytes,
            text: textDecoder.decode(bytes),
            dependencies,
            projectRevision: bundle.project.revision,
            resourceRevision: resource.revision,
            repositoryRevision: response.repositoryRevision,
            sha256: resource.sha256,
            project: bundle.project,
            contents: bundle.contents,
            currentEntry: currentEntries[0],
            expected,
            operation: workbenchOperation(example),
            pageKind: example.kind ?? 'inspection',
            referencedResourceIds: Object.freeze([
                example.resourceId,
                ...dependencyDescriptors.map(({ resourceId }) => resourceId),
            ]),
        };
    }

    function captureBuild(persisted) {
        const entry = example.projectOwned ? persisted.currentEntry : undefined;
        const resourceId = entry?.runConfigResourceId ?? example.runConfigResourceId;
        const configuration = persisted.project.resources.find(resource => resource.id === resourceId);
        const ids = new Set([...persisted.referencedResourceIds, ...(resourceId ? [resourceId] : [])]);
        return freezeWorkbenchValue({ projectId, projectRootUri: persisted.project.rootUri,
            projectRevision: persisted.projectRevision,
            configuration: { id: entry?.id ?? example.id, name: entry?.name ?? example.name,
                kind: entry?.kind ?? example.kind, resourceId: configuration?.id,
                ...(entry?.entryArtifact === undefined ? {} : { entryArtifact: entry.entryArtifact }),
                revision: configuration?.revision ?? persisted.resourceRevision, sha256: configuration?.sha256 ?? persisted.sha256 },
            inputs: persisted.project.resources.filter(resource => ids.has(resource.id)).map(resource => ({
                resourceId: resource.id, uri: projectResourceUri(persisted.project, resource), path: resource.path,
                revision: resource.revision, sha256: resource.sha256,
            })) });
    }

    async function reload() {
        commandEditVersion += 1;
        const persisted = await readPersisted();
        const command = await commandForPersisted(persisted, true);
        editVersion += 1;
        graphUndo.length = 0; graphRedo.length = 0; graphCache = undefined;
        return publish({
            ...state,
            status: 'loaded',
            path: example.path,
            runnable: !example.projectOwned || Boolean(persisted.commandBytes) || Boolean(example.commandArguments),
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            draft: persisted.text,
            dirty: false,
            validation: undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            command,
            selection: undefined,
            error: undefined,
            graphConfiguration: undefined,
            workbenchId: example.id,
            operation: command?.parsed?.commandPath?.[0] ?? persisted.operation,
            expected: persisted.expected,
        });
    }

    async function selectWorkbench(workbenchId) {
        const selected = workbenches.find(({ id }) => id === workbenchId);
        if (!selected) throw new RangeError(`Feature Tour workbench ${String(workbenchId)} is unavailable`);
        example = selected;
        commandEditVersion += 1;
        editVersion += 1;
        publish({
            ...state,
            status: 'loading',
            workbenchId: selected.id,
            resourceId: selected.resourceId,
            path: selected.path,
            contentType: selected.contentType,
            schema: selected.schema,
            operation: workbenchOperation(selected),
            validation: undefined,
            projection: undefined,
            command: undefined,
            selection: undefined,
            expected: undefined,
            error: undefined,
        });
        return reload();
    }

    function updateDraft(draft, remember = true) {
        if (typeof draft !== 'string') throw new TypeError('Feature Tour draft must be text');
        if (isGraphConfiguration() && remember && draft !== state.draft) {
            graphUndo.push(state.draft); graphRedo.length = 0;
            trimGraphHistory();
        }
        editVersion += 1;
        return publish({
            ...state,
            status: draft === state.persistedText ? 'loaded' : 'dirty',
            draft,
            dirty: draft !== state.persistedText,
            validation: state.validation ? freezeValidation({ ...state.validation, stale: true }) : undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            selection: undefined,
            error: undefined,
            graphConfiguration: undefined,
        });
    }

    function trimGraphHistory() {
        const size = () => [...graphUndo, ...graphRedo].reduce((sum, text) => sum + textEncoder.encode(text).byteLength, 0);
        while (graphUndo.length + graphRedo.length > 20 || size() > 8 * 1024 * 1024) {
            if (graphUndo.length) graphUndo.shift(); else graphRedo.shift();
        }
    }

    async function updateGraphAttribute(fields) {
        if (!isGraphConfiguration()) throw new Error('This resource is not an editable graph configuration');
        try {
            if (workbenchBusy(state) || state.command?.status === 'checking' || fields.version?.[0] !== `${commandFormId}:source:${editVersion}`) throw new Error('The graph changed. Review the current configuration and retry.');
            for (const [name, values] of Object.entries(fields)) {
                if (!['node', 'attribute', 'value', 'mode', 'version'].includes(name) || !Array.isArray(values) || values.length !== 1 || typeof values[0] !== 'string') throw new Error('Unexpected or repeated graph form fields');
            }
            const mode = fields.mode?.[0];
            if (!['set', 'reset'].includes(mode)) throw new Error('Choose Set or Reset for the graph attribute');
            if (mode === 'set' && typeof fields.value?.[0] !== 'string') throw new Error('A graph attribute value must be text');
            const candidate = validator.authorGraph({ source: state.draft, edit: { node: fields.node?.[0], attribute: fields.attribute?.[0], value: mode === 'reset' ? null : fields.value[0] } });
            if (candidate.source === state.draft) return publish({ ...state, graphConfiguration: { ...state.graphConfiguration, error: undefined } });
            return updateDraft(candidate.source);
        } catch (error) {
            return publish({ ...state, graphConfiguration: { ...state.graphConfiguration, error: normalizedError(error) } });
        }
    }

    async function createGraphNode(fields) {
        if (!isGraphConfiguration()) throw new Error('This resource is not an editable graph configuration');
        try {
            if (workbenchBusy(state) || state.command?.status === 'checking' || fields.version?.[0] !== `${commandFormId}:source:${editVersion}`) throw new Error('The graph changed. Review the current configuration and retry.');
            const kind = state.graphConfiguration.nodeTypes.find(kind => kind.id === fields.kind?.[0]);
            if (!kind) throw new Error('Choose a native graph node kind');
            const allowed = ['parent', 'kind', 'id', 'version', ...kind.fields.map(field => `attribute.${field.id}`)];
            for (const [name, values] of Object.entries(fields)) {
                if (!allowed.includes(name) || !Array.isArray(values) || values.length !== 1 || typeof values[0] !== 'string') throw new Error('Unexpected or repeated graph creation fields');
            }
            if (['parent', 'kind', 'id'].some(name => typeof fields[name]?.[0] !== 'string')) throw new Error('Choose a parent, node kind and authored ID');
            const attributes = Object.fromEntries(kind.fields.flatMap(field => {
                const value = fields[`attribute.${field.id}`]?.[0];
                return value === undefined || (!field.required && value === '') ? [] : [[field.id, value]];
            }));
            const candidate = validator.authorGraph({ source: state.draft, create: { parent: fields.parent[0] || null, kind: fields.kind[0], id: fields.id[0], attributes } });
            return updateDraft(candidate.source);
        } catch (error) {
            return publish({ ...state, graphConfiguration: { ...state.graphConfiguration, error: normalizedError(error) } });
        }
    }

    async function updateGraphEdge(fields) {
        if (!isGraphConfiguration()) throw new Error('This resource is not an editable graph configuration');
        try {
            if (workbenchBusy(state) || state.command?.status === 'checking' || fields.version?.[0] !== `${commandFormId}:source:${editVersion}`) throw new Error('The graph changed. Review the current configuration and retry.');
            const field = state.graphConfiguration.nodes.find(node => node.id === fields.node?.[0])?.edges.find(field => field.id === fields.role?.[0]);
            if (!field) throw new Error('Choose a native graph edge target and role');
            const allowed = ['node', 'role', 'source', 'version', ...(field.requiresName ? ['name'] : [])];
            for (const [name, values] of Object.entries(fields)) {
                if (!allowed.includes(name) || !Array.isArray(values) || values.length !== 1 || typeof values[0] !== 'string') throw new Error('Unexpected or repeated graph edge fields');
            }
            if (typeof fields.source?.[0] !== 'string' || (field.requiresName && typeof fields.name?.[0] !== 'string')) throw new Error('Choose an edge source and any required binding name');
            const candidate = validator.authorGraph({ source: state.draft, edge: { node: fields.node[0], role: fields.role[0], source: fields.source[0] || null, ...(field.requiresName ? { name: fields.name[0] } : {}) } });
            if (candidate.source === state.draft) return publish({ ...state, graphConfiguration: { ...state.graphConfiguration, error: undefined } });
            return updateDraft(candidate.source);
        } catch (error) { return publish({ ...state, graphConfiguration: { ...state.graphConfiguration, error: normalizedError(error) } }); }
    }

    function replayGraphDraft(undo) {
        if (!isGraphConfiguration() || workbenchBusy(state) || state.command?.status === 'checking') return state;
        const from = undo ? graphUndo : graphRedo, to = undo ? graphRedo : graphUndo;
        if (!from.length) return state;
        const draft = from.pop();
        to.push(state.draft);
        trimGraphHistory();
        return updateDraft(draft, false);
    }

    const undoGraphDraft = () => replayGraphDraft(true);
    const redoGraphDraft = () => replayGraphDraft(false);

    async function saveSource(options = {}) {
        return persistSource(options, false);
    }

    async function saveAndValidate(options = {}) {
        return persistSource(options, true);
    }

    async function persistSource(options, validate) {
        const captured = {
            draft: state.draft,
            editVersion,
            projectRevision: state.projectRevision,
            resourceRevision: state.resourceRevision,
        };
        publish({ ...state, status: 'saving', error: undefined });
        let saved;
        try {
            saved = await repository.execute(
                repositoryRequest('save-resource', {
                    projectId,
                    resourceId: example.resourceId,
                    expectedProjectRevision: captured.projectRevision,
                    expectedResourceRevision: captured.resourceRevision,
                    content: captured.draft,
                }),
                options.signal,
            );
        } catch (error) {
            publish({
                ...state,
                status: error?.code === 'cem.studio.repository.revision_conflict' ? 'conflict' : 'failed',
                error: normalizedError(error),
            });
            throw error;
        }

        const persisted = await readPersisted();
        const expectedBytes = textEncoder.encode(captured.draft);
        if (
            !equalBytes(expectedBytes, persisted.bytes) ||
            persisted.sha256 !== saved.value.sha256 ||
            persisted.projectRevision !== saved.value.projectRevision ||
            persisted.resourceRevision !== saved.value.resourceRevision
        ) {
            const error = new Error('Feature Tour repository did not reload the exact committed revision');
            error.code = 'cem.studio.workbench.persistence_mismatch';
            publish({ ...state, status: 'failed', error: normalizedError(error) });
            throw error;
        }
        const capturedCommandVersion = commandEditVersion;
        let command;
        try {
            command = await commandForPersisted(persisted);
        } catch (error) {
            // Saving source is a repository transaction. A failed native preview
            // must not hide its durable revision or erase either draft.
            command = state.command && freezeCommand({
                ...state.command,
                status: 'invalid',
                parsed: undefined,
                preview: undefined,
                diagnostic: normalizedError(error),
            });
        }
        const changedDuringSave = editVersion !== captured.editVersion;
        const dirty = changedDuringSave && state.draft !== persisted.text;
        publish({
            ...state,
            status: dirty ? 'dirty' : 'saved',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            draft: changedDuringSave ? state.draft : persisted.text,
            dirty,
            validation: undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            command: capturedCommandVersion === commandEditVersion ? command : state.command,
            selection: undefined,
            error: undefined,
        });
        if (!validate) return state;
        return validateRevision(
            { ...persisted, editVersion: captured.editVersion, draftMatches: true },
            options.signal,
        );
    }

    async function validatePersisted(options = {}) {
        const capturedCommandVersion = commandEditVersion;
        const persisted = await readPersisted();
        const command = await commandForPersisted(persisted);
        const capturedVersion = editVersion;
        publish({
            ...state,
            status: 'validating',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            command: capturedCommandVersion === commandEditVersion ? command : state.command,
            error: undefined,
        });
        return validateRevision(
            {
                ...persisted,
                editVersion: capturedVersion,
                draftMatches: state.draft === persisted.text,
            },
            options.signal,
        );
    }

    async function validateRevision(persisted, signal) {
        publish({ ...state, status: 'validating', error: undefined });
        let outcome;
        try {
            outcome = await validator.validateResource({
                bytes: persisted.bytes,
                contentType: example.contentType,
                schema: example.schema,
                uri: example.referenceUri ?? example.path,
                resourceScheme: 'studio',
                dependencies: persisted.dependencies,
                projectId,
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                signal,
            });
        } catch (error) {
            if (!error?.result) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation };
        }
        const validation = projectValidation(outcome.result, outcome.presentation, persisted.text, {
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            sha256: persisted.sha256,
        });
        const stale =
            editVersion !== persisted.editVersion ||
            persisted.draftMatches === false ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
        const nextValidation = freezeValidation({ ...validation, stale });
        publish({
            ...state,
            status: stale ? 'stale' : validation.hardViolationCount > 0 ? 'invalid' : 'valid',
            validation: nextValidation,
            selection: undefined,
            error: undefined,
        });
        return state;
    }

    async function parsePersisted(projection = 'ast', options = {}) {
        if (!CEM_STUDIO_PARSE_PROJECTIONS.includes(projection)) {
            throw new TypeError(`unsupported source parse projection: ${projection}`);
        }
        return projectPersisted('parse', projection, options);
    }

    async function inspectPersisted(view = 'summary', options = {}) {
        if (!CEM_STUDIO_INSPECT_VIEWS.includes(view)) {
            throw new TypeError(`unsupported source inspect view: ${view}`);
        }
        return projectPersisted('inspect', view, options);
    }

    async function runPersistedOperation(options = {}) {
        if (example.projectOwned) {
            const persisted = await readPersisted();
            if (persisted.commandBytes) {
                const version = editVersion;
                const commandVersion = commandEditVersion;
                publish({ ...state, status: 'projecting', error: undefined });
                try {
                    let outcome;
                    try {
                        outcome = await validator.executeAuthoredResourceCommand({ ...commandResourceOptions(persisted), commandResource: persisted.commandBytes, signal: options.signal });
                    } catch (error) {
                        if (!error?.result || !error?.output || !error?.parsed) throw error;
                        outcome = { result: error.result, presentation: error.presentation, output: error.output, outputs: error.outputs, admission: error.admission, parsed: error.parsed };
                    }
                    const live = persisted.currentEntry?.tags?.includes('live-preview')
                        && persisted.project.resources.find(resource => resource.id === example.resourceId)?.role === 'template'
                        && outcome.result.exitCode === 0
                        ? await createCemStudioLivePreview({ source: persisted.text, sourceUri: example.referenceUri, dependencies: persisted.dependencies,
                            components: persisted.currentEntry.tags.filter(tag => tag.startsWith('preview-component:')).map(tag => tag.slice('preview-component:'.length)) }) : undefined;
                    const latest = await readPersisted();
                    const operation = outcome.parsed.commandPath[0];
                    const stale = version !== editVersion || commandVersion !== commandEditVersion || state.dirty
                        || latest.projectRevision !== persisted.projectRevision || latest.resourceRevision !== persisted.resourceRevision || Boolean(outcome.result.stale);
                    const projection = freezeProjection({ ...projectCommandProjection(operation, commandProjectionMode(outcome.parsed, operation), outcome,
                        persisted.text, { projectRevision: persisted.projectRevision, resourceRevision: persisted.resourceRevision, sha256: persisted.sha256 }, persisted.expected, captureBuild(persisted)), ...(live ? { preview: live } : {}), stale });
                    return publish({ ...state, status: stale ? 'projection-stale' : projection.exitCode === 0 ? 'projected' : 'projection-invalid', projection, operation });
                } catch (error) { publish({ ...state, status: 'failed', error: normalizedError(error) }); throw error; }
            }
            if (!example.commandArguments) throw new Error('This configuration has no persisted executable command');
        }
        const operation = workbenchOperation(example);
        if (operation === 'parse') return parsePersisted('ast', options);
        if (operation === 'inspect') return inspectPersisted('summary', options);
        const capturedCommandVersion = commandEditVersion;
        const persisted = await readPersisted();
        const capturedVersion = editVersion;
        const draftMatches = state.draft === persisted.text;
        const argv = commandArgumentsForExample(example, projectId, 'studio');
        publish({
            ...state,
            status: 'projecting',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            error: undefined,
        });
        let outcome;
        try {
            outcome = await validator.runResourceCommand({
                ...commandResourceOptions(persisted),
                argv,
                signal: options.signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation, output: error.output, outputs: error.outputs, admission: error.admission };
        }
        const mode = commandProjectionMode(undefined, operation, example.kind);
        const projected = projectCommandProjection(
            operation,
            mode,
            outcome,
            persisted.text,
            {
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                sha256: persisted.sha256,
            },
            persisted.expected,
            captureBuild(persisted),
        );
        const command = await commandForUneditedProjection(persisted, operation, mode, capturedCommandVersion);
        const stale =
            editVersion !== capturedVersion ||
            !draftMatches ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
        const projection = freezeProjection({ ...projected, stale });
        publish({
            ...state,
            status: stale ? 'projection-stale' : projected.exitCode === 0 ? 'projected' : 'projection-invalid',
            projection,
            command: command ?? state.command,
            selection: undefined,
            error: undefined,
        });
        return state;
    }

    async function projectPersisted(kind, mode, options) {
        const capturedCommandVersion = commandEditVersion;
        const persisted = await readPersisted();
        const capturedVersion = editVersion;
        const draftMatches = state.draft === persisted.text;
        publish({
            ...state,
            status: 'projecting',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            error: undefined,
        });
        let outcome;
        try {
            outcome = await validator[`${kind}Resource`]({
                bytes: persisted.bytes,
                contentType: example.contentType,
                schema: example.schema,
                uri: example.referenceUri ?? example.path,
                resourceScheme: 'studio',
                dependencies: persisted.dependencies,
                projectId,
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                [kind === 'parse' ? 'projection' : 'view']: mode,
                signal: options.signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publish({ ...state, status: 'failed', error: normalizedError(error) });
                throw error;
            }
            outcome = { result: error.result, presentation: error.presentation, output: error.output, outputs: error.outputs, admission: error.admission };
        }
        const projected = projectCommandProjection(
            kind,
            mode,
            outcome,
            persisted.text,
            {
                projectRevision: persisted.projectRevision,
                resourceRevision: persisted.resourceRevision,
                sha256: persisted.sha256,
            },
            persisted.expected,
            captureBuild(persisted),
        );
        const command = await commandForUneditedProjection(persisted, kind, mode, capturedCommandVersion);
        const stale =
            editVersion !== capturedVersion ||
            !draftMatches ||
            state.projectRevision !== persisted.projectRevision ||
            state.resourceRevision !== persisted.resourceRevision ||
            Boolean(outcome.result.stale);
        const projection = freezeProjection({ ...projected, stale });
        publish({
            ...state,
            status: stale ? 'projection-stale' : projected.exitCode === 0 ? 'projected' : 'projection-invalid',
            projection,
            command: command ?? state.command,
            selection: undefined,
            error: undefined,
        });
        return state;
    }

    async function updateCommandDraft(text, { atomic = false } = {}) {
        if (typeof text !== 'string') throw new TypeError('CEM Studio command draft must be text');
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        const capturedVersion = ++commandEditVersion;
        const original = state.command;
        const current = state.command.current;
        publish({
            ...state,
            command: freezeCommand({
                ...state.command,
                status: 'checking',
                formError: undefined,
                draftText: atomic ? original.draftText : text,
                diagnostic: undefined,
                copy: undefined,
                application: atomic ? state.command.application : commandApplicationAfterEdit(state.command.application),
            }),
            projection:
                !atomic && state.command.application?.execution && state.projection
                    ? freezeProjection({ ...state.projection, stale: true })
                    : state.projection,
        });
        try {
            const persisted = await readPersisted();
            if (capturedVersion !== commandEditVersion) return state;
            const preview = await validator.previewResourceCommand({
                ...commandResourceOptions(persisted),
                text,
            });
            if (capturedVersion !== commandEditVersion) return state;
            const changes = semanticChanges(current.semantic, preview.semantic);
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    status: changes.length === 0 ? 'current' : 'changed',
                    draftText: text,
                    parsed: preview.parsed,
                    preview,
                    changes,
                    formError: undefined,
                    // Recheck against a source revision saved by this workbench.
                    // A concurrently advanced repository still requires reload.
                    revision: state.projectRevision === persisted.projectRevision &&
                        state.resourceRevision === persisted.resourceRevision
                        ? commandRevision(persisted) : state.command.revision,
                    diagnostic: undefined,
                    copy: undefined,
                    application: commandApplicationState(persisted, state.command.application, preview.parsed),
                }),
                projection: original.application?.execution && state.projection
                    ? freezeProjection({ ...state.projection, stale: true }) : state.projection,
            });
        } catch (error) {
            if (capturedVersion !== commandEditVersion) return state;
            publish({
                ...state,
                command: atomic ? freezeCommand({ ...original, formError: normalizedError(error) }) : freezeCommand({
                    ...state.command,
                    status: 'invalid',
                    draftText: text,
                    parsed: undefined,
                    preview: undefined,
                    changes: Object.freeze([]),
                    diagnostic: normalizedError(error),
                    copy: undefined,
                }),
            });
        }
        return state;
    }

    async function updateCommandField(id, value, version, reset = false) {
        if (!state.command) return state;
        try {
            if (`${commandFormId}:${commandEditVersion}` !== String(version) || state.command.status === 'checking') throw new Error('The command changed. Review the current operation form and retry.');
            const text = editCemStudioOperationField(state.command.draftText, id, value, reset);
            return await updateCommandDraft(text);
        } catch (error) {
            return publish({ ...state, command: { ...state.command, formError: normalizedError(error) } });
        }
    }

    async function updateCommandFields(edits, version) {
        if (!state.command) return state;
        try {
            if (`${commandFormId}:${commandEditVersion}` !== String(version) || state.command.status === 'checking' || workbenchBusy(state)) throw new Error('The command changed. Review the current operation form and retry.');
            const text = editCemStudioOperationFields(state.command.draftText, edits);
            if (text === state.command.draftText) return state.command.formError
                ? publish({ ...state, command: { ...state.command, formError: undefined } }) : state;
            return await updateCommandDraft(text, { atomic: true });
        } catch (error) {
            return publish({ ...state, command: { ...state.command, formError: normalizedError(error) } });
        }
    }

    // Native form capture is a command protocol boundary, not a DOM/UI adapter.
    async function updateCommandGroup(id, fields, version) {
        if (!state.command) return state;
        try {
            const group = describeCemStudioOperationGroups(state.command.draftText).find(group => group.id === id);
            if (!group) throw new Error('This option group is not available');
            const allowed = new Set(['group', 'version', ...group.fields.flatMap(field => [`mode.${field.id}`, `value.${field.id}`])]);
            for (const [name, values] of Object.entries(fields)) {
                if (!allowed.has(name) || !Array.isArray(values) || values.length !== 1 || typeof values[0] !== 'string') throw new Error('The option group contains unexpected or repeated fields');
            }
            const edits = group.fields.flatMap(field => {
                const mode = fields[`mode.${field.id}`]?.[0];
                if (mode === 'keep') return [];
                if (!['set', 'reset'].includes(mode)) throw new Error('Choose Keep, Set or Reset for every option');
                return [{ id: field.id, value: mode === 'reset' ? '' : fields[`value.${field.id}`]?.[0], reset: mode === 'reset' }];
            });
            return await updateCommandFields(edits, version);
        } catch (error) {
            return publish({ ...state, command: { ...state.command, formError: normalizedError(error) } });
        }
    }

    async function resetCommandDraft() {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        return updateCommandDraft(state.command.current.text);
    }

    async function copyCommand(writeText) {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        publish({
            ...state,
            command: freezeCommand({ ...state.command, copy: Object.freeze({ status: 'copying' }) }),
        });
        try {
            if (typeof writeText !== 'function') {
                const error = new Error('Clipboard writing is unavailable; select and copy the command text instead.');
                error.code = 'cem.studio.command.clipboard_unavailable';
                throw error;
            }
            await writeText(state.command.draftText);
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    copy: Object.freeze({
                        status: 'success',
                        message: 'Displayed Studio command copied.',
                    }),
                }),
            });
        } catch (error) {
            publish({
                ...state,
                command: freezeCommand({
                    ...state.command,
                    copy: Object.freeze({
                        status: 'failed',
                        message: normalizedError(error).message,
                    }),
                }),
            });
        }
        return state;
    }

    function selectArtifact(id) {
        const projection = state.projection;
        const artifact = projection?.buildRun.artifacts.find(artifact => artifact.id === id);
        const output = projection?.outputs.find(output => output.uri === artifact?.uri);
        if (!artifact || !output) throw new Error(`Output artifact ${id} is unavailable`);
        if (projection.selectedArtifactId === id) return state;
        return publish({ ...state, projection: freezeProjection({ ...projection, selectedArtifactId: id, output,
            preview: createCemStudioPreview({ bytes: output.bytes, contentType: output.contentType, label: artifact.path }),
            provenance: artifact.provenance, transfer: undefined }) });
    }

    async function copyProjection(writeText) {
        const projection = state.projection;
        if (!projection?.output || projection.output.available === false) throw new Error('CEM Studio has no operation output to copy');
        if (state.projection.preview?.kind === 'download') {
            throw new Error('Binary, invalid-text, and oversized active results must be downloaded as exact bytes.');
        }
        try {
            if (typeof writeText !== 'function') {
                const error = new Error('Clipboard writing is unavailable; select and copy the result text instead.');
                error.code = 'cem.studio.projection.clipboard_unavailable';
                throw error;
            }
            await writeText(projection.output.text);
            if (state.projection !== projection) return state;
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'copied', message: 'Exact operation output copied.' }),
                }),
            });
        } catch (error) {
            if (state.projection !== projection) return state;
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'failed', message: normalizedError(error).message }),
                }),
            });
        }
        return state;
    }

    async function downloadProjection(save) {
        const projection = state.projection;
        if (!projection?.output || projection.output.available === false) throw new Error('CEM Studio has no operation output to download');
        const extension = ({ 'text/html': 'html', 'application/xhtml+xml': 'xhtml', 'image/svg+xml': 'svg',
            'application/json': 'json', 'application/yaml': 'yaml', 'text/plain': 'txt', 'text/css': 'css',
            'application/octet-stream': 'bin' })[projection.output.contentType?.split(';')[0].trim()] ?? 'cem';
        const artifact = projection.buildRun.artifacts.find(artifact => artifact.id === projection.selectedArtifactId);
        const filename = artifact?.path && !artifact.uri.startsWith('cem-stdio:') ? artifact.path.split('/').pop()
            : `${projection.kind}-${String(projection.mode).replace(/[^a-z0-9-]+/gi, '-')}.${extension}`;
        try {
            if (typeof save !== 'function') throw new Error('Result download is unavailable in this browser.');
            await save({
                filename,
                contentType: projection.output.contentType,
                bytes: new Uint8Array(projection.output.bytes),
            });
            if (state.projection !== projection) return state;
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'downloaded', message: `Downloaded ${filename}.` }),
                }),
            });
        } catch (error) {
            if (state.projection !== projection) return state;
            publish({
                ...state,
                projection: freezeProjection({
                    ...state.projection,
                    transfer: Object.freeze({ status: 'failed', message: normalizedError(error).message }),
                }),
            });
        }
        return state;
    }

    function setCommandTarget(target) {
        if (!state.command) throw new Error('CEM Studio command view is unavailable');
        const application = configureCommandApplication(state.command.application, target);
        commandTargetVersion += 1;
        publish({
            ...state,
            command: freezeCommand({ ...state.command, application }),
            error: undefined,
        });
        return state;
    }

    async function applyCommand(options = {}) {
        return applyCommandPage(false, options);
    }

    async function applyAndRun(options = {}) {
        return applyCommandPage(true, options);
    }

    async function confirmCommandReplacement(options = {}) {
        const confirmation = state.command?.application?.confirmation;
        if (!confirmation) throw new Error('No incompatible command-configuration replacement is awaiting confirmation');
        return applyCommandPage(confirmation.runAfterApply, options, {
            ...confirmation.target,
            confirmIncompatibleReplacement: true,
        });
    }

    function useNewCommandTarget() {
        const application = state.command?.application;
        if (!application) throw new Error('CEM Studio command target is unavailable');
        return setCommandTarget({
            mode: 'new',
            name: application.newPageName,
            ...(application.newPageParentId ? { parentId: application.newPageParentId } : {}),
        });
    }

    function cancelCommandReplacement() {
        const command = state.command;
        if (!command?.application) return state;
        publish({
            ...state,
            command: freezeCommand({
                ...command,
                application: freezeCommandApplication({
                    ...command.application,
                    status: 'ready',
                    confirmation: undefined,
                    error: undefined,
                }),
            }),
        });
        return state;
    }

    async function applyCommandPage(runAfterApply, options, confirmedTarget) {
        const command = state.command;
        if (!command?.preview?.parsed || command.status === 'invalid' || command.status === 'checking') {
            throw new Error('A valid checked Studio command is required before Apply');
        }
        const capturedVersion = commandEditVersion;
        const capturedTargetVersion = commandTargetVersion;
        const application = command.application;
        const target = confirmedTarget ?? application.target;
        const commandResource = validator.serializeResourceCommand(command.preview.parsed);
        const commandBytes = textEncoder.encode(commandResource);
        publish({
            ...state,
            command: freezeCommand({
                ...command,
                application: freezeCommandApplication({
                    ...application,
                    status: 'applying',
                    confirmation: undefined,
                    error: undefined,
                }),
            }),
            error: undefined,
        });

        let applied;
        try {
            applied = await repository.execute(
                repositoryRequest('apply-command-page', {
                    projectId,
                    expectedProjectRevision: command.revision.projectRevision,
                    target,
                    commandResource,
                    referencedResourceIds: application.referencedResourceIds,
                }),
                options.signal,
            );
        } catch (error) {
            if (error?.code === 'cem.studio.repository.command_target_incompatible') {
                publish({
                    ...state,
                    command: freezeCommand({
                        ...state.command,
                        application: freezeCommandApplication({
                            ...state.command.application,
                            status: 'confirmation-required',
                            confirmation: Object.freeze({ target, runAfterApply }),
                            error: normalizedError(error),
                        }),
                    }),
                });
                return state;
            }
            publishCommandApplicationFailure(error);
            throw error;
        }

        const persisted = await readPersisted();
        try {
            assertAppliedCommandCommit(persisted, applied.value, commandBytes);
        } catch (error) {
            publishCommandApplicationFailure(error);
            throw error;
        }
        const appliedPreview = await validator.previewResourceCommand({
            ...commandResourceOptions(persisted),
            text: command.preview.text,
        });
        const refreshedCommand = commandState(appliedPreview, persisted, command.application);
        const changedDuringApply = capturedVersion !== commandEditVersion || capturedTargetVersion !== commandTargetVersion;
        const selectedApplication = commandApplicationState(persisted, {
            ...refreshedCommand.application,
            currentEntryId: applied.value.entry.id,
            target: { mode: 'current', entryId: applied.value.entry.id },
            result: freezeAppliedCommandResult(applied.value),
        });
        const appliedApplication = freezeCommandApplication({
            ...selectedApplication,
            status: changedDuringApply ? 'stale' : 'applied',
            confirmation: undefined,
            error: undefined,
        });
        // The committed baseline revokes older command previews/projections.
        const appliedVersion = ++commandEditVersion;
        publish({
            ...state,
            status: state.dirty ? 'dirty' : state.validation ? 'stale' : 'loaded',
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            repositoryRevision: persisted.repositoryRevision,
            persistedText: persisted.text,
            command: freezeCommand({
                ...(changedDuringApply ? {
                    ...state.command,
                    current: appliedPreview,
                    revision: commandRevision(persisted),
                    changes: state.command.preview ? semanticChanges(appliedPreview.semantic, state.command.preview.semantic) : [],
                } : refreshedCommand),
                application: changedDuringApply ? freezeCommandApplication({
                    ...appliedApplication,
                    target: state.command.application.target,
                    newPageName: state.command.application.newPageName,
                    newPageParentId: state.command.application.newPageParentId,
                }) : appliedApplication,
            }),
            validation: state.validation ? freezeValidation({ ...state.validation, stale: true }) : undefined,
            projection: state.projection ? freezeProjection({ ...state.projection, stale: true }) : undefined,
            error: undefined,
        });
        if (changedDuringApply) return updateCommandDraft(state.command.draftText);
        if (!runAfterApply) return state;
        return runAppliedCommand(persisted, applied.value, commandBytes, appliedVersion, options.signal);
    }

    async function runAppliedCommand(persisted, applied, commandBytes, capturedVersion, signal) {
        publish({
            ...state,
            command: freezeCommand({
                ...state.command,
                application: freezeCommandApplication({
                    ...state.command.application,
                    status: 'running',
                }),
            }),
        });
        let outcome;
        try {
            outcome = await validator.executeAuthoredResourceCommand({
                ...commandResourceOptions(persisted),
                commandResource: commandBytes,
                signal,
            });
        } catch (error) {
            if (!error?.result || !error?.output) {
                publishCommandApplicationFailure(error);
                throw error;
            }
            outcome = {
                result: error.result,
                presentation: error.presentation,
                output: error.output,
                outputs: error.outputs,
                admission: error.admission,
                parsed: error.parsed ?? state.command.preview?.parsed,
            };
        }
        const latest = await readPersisted();
        const stale =
            capturedVersion !== commandEditVersion ||
            !isAppliedCommandCommit(latest, applied, commandBytes) ||
            Boolean(outcome.result?.stale);
        const operation = outcome.parsed?.commandPath?.[0] ?? applied.operation;
        const mode = commandProjectionMode(outcome.parsed, operation);
        const projection = freezeProjection({
            ...projectCommandProjection(
                operation,
                mode,
                outcome,
                persisted.text,
                {
                    projectRevision: applied.projectRevision,
                    resourceRevision: applied.resourceRevision,
                    sha256: applied.sha256,
                },
                persisted.expected,
                captureBuild(persisted),
            ),
            stale,
        });
        publish({
            ...state,
            status: stale ? 'projection-stale' : projection.exitCode === 0 ? 'projected' : 'projection-invalid',
            projection,
            command: freezeCommand({
                ...state.command,
                application: freezeCommandApplication({
                    ...state.command.application,
                    status: stale ? 'run-stale' : projection.exitCode === 0 ? 'ran' : 'run-invalid',
                    execution: Object.freeze({
                        requestId: projection.requestId,
                        exitCode: projection.exitCode,
                        projectRevision: applied.projectRevision,
                        resourceRevision: applied.resourceRevision,
                        sha256: applied.sha256,
                        stale,
                    }),
                }),
            }),
            error: undefined,
        });
        return state;
    }

    function publishCommandApplicationFailure(error) {
        const command = state.command;
        publish({
            ...state,
            command: command
                ? freezeCommand({
                      ...command,
                      application: freezeCommandApplication({
                          ...command.application,
                          status: error?.code === 'cem.studio.repository.revision_conflict' ? 'conflict' : 'failed',
                          confirmation: undefined,
                          error: normalizedError(error),
                      }),
                  })
                : command,
            error: normalizedError(error),
        });
    }

    async function commandForPersisted(persisted, reload = false) {
        const command = reload && example.projectOwned ? undefined : state.command;
        if (command?.current?.text) {
            const current = await validator.previewResourceCommand({
                ...commandResourceOptions(persisted),
                text: command.current.text,
            });
            if (command.draftText === command.current.text) {
                return commandState(current, persisted, command.application);
            }
            try {
                const preview = await validator.previewResourceCommand({
                    ...commandResourceOptions(persisted),
                    text: command.draftText,
                });
                const changes = semanticChanges(current.semantic, preview.semantic);
                return freezeCommand({
                    ...command,
                    status: changes.length === 0 ? 'current' : 'changed',
                    current,
                    parsed: preview.parsed,
                    preview,
                    changes,
                    diagnostic: undefined,
                    revision: commandRevision(persisted),
                    copy: undefined,
                    application: commandApplicationState(persisted, command.application, preview.parsed),
                });
            } catch (error) {
                return freezeCommand({
                    ...command,
                    status: 'invalid',
                    current,
                    parsed: undefined,
                    preview: undefined,
                    changes: Object.freeze([]),
                    diagnostic: normalizedError(error),
                    revision: commandRevision(persisted),
                    copy: undefined,
                    application: commandApplicationState(persisted, command.application),
                });
            }
        }
        if (example.projectOwned) {
            if (persisted.commandBytes) {
                const preview = await validator.previewResourceCommand({ ...commandResourceOptions(persisted), commandResource: persisted.commandBytes });
                return commandState(preview, persisted, state.command?.application);
            }
            if (!example.commandArguments) return undefined;
        }
        const operation = persisted.operation ?? 'parse';
        return commandForProjection(
            persisted,
            operation,
            operation === 'parse' ? 'ast' : operation === 'inspect' ? 'summary' : example.kind,
        );
    }

    async function commandForUneditedProjection(persisted, kind, mode, version) {
        if (version !== commandEditVersion || state.command?.draftText !== state.command?.current.text) return undefined;
        const command = await commandForProjection(persisted, kind, mode);
        if (version !== commandEditVersion) return undefined;
        commandEditVersion += 1;
        return command;
    }

    async function commandForProjection(persisted, kind, mode) {
        const preview = await validator.previewResourceCommand({
            ...commandResourceOptions(persisted),
            operation: kind,
            ...(kind === 'parse' || kind === 'inspect'
                ? {}
                : { argv: commandArgumentsForExample(example, projectId, 'studio') }),
            [kind === 'parse' ? 'projection' : 'view']: mode,
        });
        return commandState(preview, persisted, state.command?.application);
    }

    function commandResourceOptions(persisted) {
        return {
            bytes: persisted.bytes,
            contentType: example.contentType,
            schema: example.schema,
            uri: example.referenceUri ?? example.path,
            resourceScheme: 'studio',
            projectRootUri: example.projectRootUri,
            dependencies: persisted.dependencies,
            projectId,
            projectRevision: persisted.projectRevision,
            resourceRevision: persisted.resourceRevision,
            ...(example.commandArguments ? { argv: commandArgumentsForExample(example, projectId, 'studio') } : {}),
        };
    }

    function navigateDiagnostic(index) {
        const diagnostic = (state.projection?.diagnostics ?? state.validation?.diagnostics)?.[index];
        if (!diagnostic) throw new RangeError(`diagnostic ${index} is unavailable`);
        return selectRange('diagnostic', index, diagnostic.range);
    }

    function navigateProvenance(index) {
        const frame = (state.projection?.provenance ?? state.validation?.provenance)?.[index];
        if (!frame) throw new RangeError(`provenance frame ${index} is unavailable`);
        if (frame.uri && frame.uri !== resourceUri(example.referenceUri ?? example.path, projectId)) {
            throw new Error('Open this provenance source through the project workspace.');
        }
        return selectRange('provenance', index, frame.range);
    }

    function selectSourceRange(range, projectRevision, index = 0) {
        if (state.projectRevision !== projectRevision || state.dirty) throw new Error('Source provenance is stale. Rerun the configuration before following it.');
        if (!Number.isSafeInteger(range.start) || !Number.isSafeInteger(range.len) || range.start < 0 || range.len < 0 || range.start + range.len > textEncoder.encode(state.draft).byteLength) {
            throw new RangeError('The provenance range is outside the source resource.');
        }
        for (const offset of [range.start, range.start + range.len]) {
            if (textEncoder.encode(state.draft.slice(0, utf8ByteOffsetToCodeUnit(state.draft, offset))).byteLength !== offset) throw new RangeError('The provenance range splits a UTF-8 character.');
        }
        return selectRange('provenance', index, range);
    }

    function selectRange(kind, index, range) {
        const selection = Object.freeze({
            kind,
            index,
            byteStart: range.start,
            byteLength: range.len,
            start: utf8ByteOffsetToCodeUnit(state.draft, range.start),
            end: utf8ByteOffsetToCodeUnit(state.draft, range.start + range.len),
        });
        publish({ ...state, selection });
        return selection;
    }

    await reload();
    return Object.freeze({
        snapshot,
        subscribe,
        updateDraft,
        updateGraphAttribute,
        createGraphNode,
        updateGraphEdge,
        undoGraphDraft,
        redoGraphDraft,
        reload,
        selectWorkbench,
        saveSource,
        saveAndValidate,
        validatePersisted,
        parsePersisted,
        inspectPersisted,
        runPersistedOperation,
        updateCommandDraft,
        updateCommandField,
        updateCommandFields,
        updateCommandGroup,
        resetCommandDraft,
        copyCommand,
        copyProjection,
        downloadProjection,
        selectArtifact,
        async downloadSource(download = browserDownload) {
            const persisted = await readPersisted();
            await download({ filename: example.path.split('/').pop(), contentType: example.contentType, bytes: persisted.bytes.slice() });
        },
        setCommandTarget,
        applyCommand,
        applyAndRun,
        confirmCommandReplacement,
        useNewCommandTarget,
        cancelCommandReplacement,
        navigateDiagnostic,
        navigateProvenance,
        selectSourceRange,
        dispose() {
            subscribers.clear();
        },
    });
}

/** Mount the workbench with production CEM controls only. */
export async function mountCemStudioWorkbench({
    root,
    workbench,
    clipboard = navigator.clipboard,
    download = browserDownload,
}) {
    if (!(root instanceof Element)) throw new TypeError('Feature Tour workbench root must be an Element');
    const components = await installCemStudioShellComponents();
    // The shell authors its workspace. Standalone hosts receive an empty mount
    // boundary; all visible markup still comes from the layout declaration.
    const authoredLayout = root.querySelector('[data-cem-studio-workspace-mount] > cem-studio-workbench-layout');
    const mount = authoredLayout?.parentElement ?? document.createElement('div');
    if (!authoredLayout) root.append(mount);
    const layout = mountCemView(components.runtime, { root: mount, element: authoredLayout ?? undefined, tag: 'cem-studio-workbench-layout',
        state: { pane: 'source', navigation: workbenchLayoutNavigation(workbench.snapshot().navigation) }, commands: { 'select-pane': ({ value }) => layout.update({ pane: value === 'results' ? 'results' : 'source' }) } });
    await layout.whenSettled();
    const host = layout.element;

    let renderPromise = Promise.resolve();
    let controlPromise = Promise.resolve();
    let buildPromise = Promise.resolve();
    let editPromise = Promise.resolve();
    let pending = false;
    let active = true;
    let selection;
    let selectionRequest = 0;
    const perform = (operation) => {
        if (pending || !active || workbenchBusy(workbench.snapshot())) return;
        pending = true;
        navigation.update({ pending });
        navigationActions.update({ pending });
        controls.update({ pending });
        tools.update({ pending });
        projectPicker.update({ pending });
        commandView.update({ pending });
        results.update({ pending });
        // Start on the command event stack; the service owns durable state.
        controlPromise = (async () => {
            try { await operation(); }
            catch { /* Workbench services publish their structured failure. */ }
            finally {
                pending = false;
                navigation.update({ pending });
                navigationActions.update({ pending });
                controls.update({ pending });
                tools.update({ pending });
                projectPicker.update({ pending });
                commandView.update({ pending });
                results.update({ pending });
            }
        })();
        return controlPromise;
    };
    const navigation = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-navigation-mount]'),
        element: host.querySelector('[data-cem-studio-workbench-navigation-mount] > cem-studio-workbench-navigation'),
        tag: 'cem-studio-workbench-navigation',
        state: { state: workbenchNavigationState(workbench.snapshot()), pending },
        commands: {
            'create-selected': () => workbench.requestAction?.('create'),
            'clone-selected': () => workbench.requestAction?.('clone'),
            'delete-selected': () => workbench.requestAction?.('delete'),
            'refresh-navigation': () => perform(() => workbench.refreshNavigation?.()),
            'download-source': () => perform(() => workbench.downloadSource(download)),
            'structure-edit': ({ fields }) => perform(() => workbench.editStructure({ kind: fields.action?.[0] ?? fields.kind?.[0], id: fields.id?.[0],
                name: fields.name?.[0], parentId: fields.parent?.[0], path: fields.path?.[0], content: fields.content?.[0], direction: fields.direction?.[0] })),
            'structure-undo': () => perform(() => workbench.editStructure({ kind: 'undo' })),
            'structure-redo': () => perform(() => workbench.editStructure({ kind: 'redo' })),
            'recovery-download': () => perform(() => workbench.downloadDraftRecovery(download)),
            'recovery-clear': () => perform(() => workbench.clearDraftRecovery()),
            'navigation-save': () => perform(() => workbench.resolveNavigation('save')),
            'navigation-apply': () => perform(() => workbench.resolveNavigation('apply')),
            'navigation-confirm': () => perform(() => workbench.resolveNavigation('confirm-replacement')),
            'navigation-discard': () => perform(() => workbench.resolveNavigation('discard')),
            'navigation-retry': () => perform(() => workbench.resolveNavigation('retry')),
            'navigation-cancel': () => workbench.resolveNavigation('cancel'),
            'select-workbench': ({ value }) => perform(() => workbench.selectWorkbench(value)),
            'open-entity': ({ value }) => perform(() => workbench.openEntity(value)),
            'search-projects': ({ fields }) => perform(() => workbench.searchProjects(fields.query?.[0])),
            'open-search': ({ fields }) => perform(() => workbench.openEntity(fields.result?.[0])),
        },
    });
    const navigationActions = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-navigation-actions-mount]'),
        element: host.querySelector('[data-cem-studio-workbench-navigation-actions-mount] > cem-studio-workbench-navigation'),
        tag: 'cem-studio-workbench-navigation',
        state: { state: workbenchNavigationState(workbench.snapshot()), pending },
        commands: {
            'create-selected': () => workbench.requestAction?.('create'),
            'clone-selected': () => workbench.requestAction?.('clone'),
            'delete-selected': () => workbench.requestAction?.('delete'),
        },
    });
    const projectPicker = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-project-picker-mount]'),
        element: host.querySelector('cem-studio-project-picker'), tag: 'cem-studio-project-picker',
        state: { navigation: workbenchListingNavigation(workbench.snapshot().navigation), pending },
        commands: {
            'submit-action': ({ fields }) => perform(() => workbench.submitAction(fields)),
            'cancel-action': () => workbench.cancelAction?.(),
            'home-create': ({ fields }) => perform(() => workbench.createProject(fields.name?.[0] ?? '')),
            'home-example': ({ fields }) => perform(() => workbench.createExampleProject(fields.example?.[0] ?? '')),
            'home-trash': ({ fields }) => perform(() => workbench.setProjectTrashed(JSON.parse(fields.project?.[0]).projectId, true)),
            'home-restore': ({ fields }) => perform(() => workbench.setProjectTrashed(fields.project?.[0], false)),
            'home-import': () => perform(() => workbench.importProject()),
            'home-open': ({ fields }) => perform(() => workbench.selectWorkbench(fields.project?.[0])),
            'open-entity': ({ value }) => perform(() => workbench.openEntity(value)),
        },
    });
    const controlCommands = {
        edit: ({ value }) => workbench.updateDraft(value ?? ''),
        'edit-graph': ({ fields }) => perform(() => workbench.updateGraphAttribute(fields)),
        'create-graph-node': ({ fields }) => perform(() => workbench.createGraphNode(fields)),
        'edit-graph-edge': ({ fields }) => perform(() => workbench.updateGraphEdge(fields)),
        'undo-graph': () => workbench.undoGraphDraft(),
        'redo-graph': () => workbench.redoGraphDraft(),
        save: () => perform(() => workbench.saveAndValidate()),
        validate: () => perform(() => workbench.validatePersisted()),
        reload: () => perform(() => workbench.reload()),
        'download-source': () => perform(() => workbench.downloadSource(download)),
        parse: ({ fields }) => perform(() => workbench.parsePersisted(fields.mode[0])),
        inspect: ({ fields }) => perform(() => workbench.inspectPersisted(fields.mode[0])),
        run: () => perform(() => workbench.runPersistedOperation()),
    };
    const controls = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-controls-mount]'),
        element: host.querySelector('cem-studio-workbench-controls'),
        tag: 'cem-studio-workbench-controls',
        state: { state: workbenchControlState(workbench.snapshot()), selectionRequest: '', pending },
        commands: controlCommands,
    });
    const tools = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-tools-mount]'),
        element: host.querySelector('cem-studio-workbench-tools'), tag: 'cem-studio-workbench-tools',
        state: { state: workbenchToolsState(workbench.snapshot()), pending }, commands: controlCommands,
    });
    const previewView = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-preview-mount]'),
        element: host.querySelector('cem-studio-workbench-preview'), tag: 'cem-studio-workbench-preview',
        state: { state: workbenchPreviewState(workbench.snapshot()) },
        commands: {
            'pin-build': () => perform(() => workbench.pinProjectBuild()),
            'select-pinned-build': ({ value }) => perform(() => workbench.selectPinnedBuild(value)),
            'delete-pinned-build': () => perform(() => workbench.deletePinnedBuild()),
            'select-build': ({ value }) => workbench.selectPreviewConfiguration(value),
            'select-preview-artifact': ({ value }) => workbench.selectPreviewArtifact(value),
            'save-build-entry': ({ fields }) => perform(() => workbench.saveBuildEntryArtifact(fields.entryArtifact?.[0] ?? '')),
            'build-project': () => { buildPromise = workbench.buildProjectPreview(); return buildPromise; },
            'cancel-build': () => workbench.cancelProjectPreview(),
            'copy-preview': () => perform(() => workbench.copyPreview(typeof clipboard?.writeText === 'function' ? text => clipboard.writeText(text) : undefined)),
            'download-preview': () => perform(() => workbench.downloadPreview(download)),
        },
    });

    const newTarget = (values = {}) => {
        const application = workbench.snapshot().command?.application;
        workbench.setCommandTarget({ mode: 'new', name: application?.newPageName ?? '',
            ...(application?.newPageParentId ? { parentId: application.newPageParentId } : {}), ...values });
    };
    const dismissReplacement = () => {
        if (workbench.snapshot().command?.application?.status === 'confirmation-required') {
            workbench.cancelCommandReplacement();
        }
    };
    const commandView = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-command-mount]'),
        element: host.querySelector('cem-studio-workbench-command'),
        tag: 'cem-studio-workbench-command',
        state: { command: workbenchCommandState(workbench.snapshot().command, workbench.snapshot().transition), pending, busy: workbenchBusy(workbench.snapshot()) },
        commands: {
            edit: ({ value }) => {
                const edit = workbench.updateCommandDraft(value ?? '').catch(() => undefined);
                editPromise = Promise.all([editPromise, edit]).then(() => undefined);
                return edit;
            },
            copy: () => perform(() => workbench.copyCommand(typeof clipboard?.writeText === 'function'
                ? text => clipboard.writeText(text) : undefined)),
            'edit-field': ({ fields }) => perform(() => workbench.updateCommandField(fields.option?.[0], fields.value?.[0] ?? '', fields.version?.[0], fields.mode?.[0] === 'reset')),
            'edit-group': ({ fields }) => perform(() => workbench.updateCommandGroup(fields.group?.[0], fields, fields.version?.[0])),
            reset: () => perform(() => workbench.resetCommandDraft()),
            'target-name': ({ value }) => newTarget({ name: value ?? '' }),
            'target-parent': ({ value }) => newTarget({ parentId: value || undefined }),
            'target-existing': ({ value }) => workbench.setCommandTarget({ mode: 'existing', entryId: value }),
            'target-mode': ({ value }) => {
                if (value === 'current') workbench.setCommandTarget({ mode: 'current' });
                else if (value === 'existing') {
                    const targets = workbench.snapshot().command?.application?.targets.existing ?? [];
                    const entry = targets.find(({ compatible }) => compatible) ?? targets[0];
                    if (entry) workbench.setCommandTarget({ mode: 'existing', entryId: entry.id });
                } else if (value === 'new') newTarget();
            },
            apply: () => perform(() => workbench.applyCommand()),
            'apply-run': () => perform(() => workbench.applyAndRun()),
            'confirm-replacement': () => perform(() => workbench.confirmCommandReplacement()),
            'use-new': () => workbench.useNewCommandTarget(),
            'cancel-replacement': dismissReplacement,
            'dismiss-replacement': dismissReplacement,
        },
    });
    const results = mountCemView(components.runtime, {
        root: host.querySelector('[data-cem-studio-workbench-results-mount]'),
        element: host.querySelector('cem-studio-workbench-results'),
        tag: 'cem-studio-workbench-results',
        state: { state: workbenchResultState(workbench.snapshot()), pending },
        commands: {
            'copy-output': () => perform(() => workbench.copyProjection(typeof clipboard?.writeText === 'function'
                ? text => clipboard.writeText(text) : undefined)),
            'download-output': () => perform(() => workbench.downloadProjection(download)),
            'select-artifact': ({ value }) => workbench.selectArtifact(value),
            diagnostic: ({ value }) => perform(async () => { layout.update({ pane: 'source' }); await layout.whenSettled(); workbench.navigateDiagnostic(Number(value)); }),
            provenance: ({ value }) => perform(async () => { layout.update({ pane: 'source' }); await layout.whenSettled(); await workbench.navigateProvenance(Number(value)); }),
        },
    });
    let latestSnapshot;
    const render = (snapshot) => {
        latestSnapshot = snapshot;
        renderPromise = renderPromise.then(async () => {
            if (!active) return;
            const current = latestSnapshot;
            if (current.selection && current.selection !== selection) selectionRequest++;
            selection = current.selection;
            navigation.update({ state: workbenchNavigationState(current), pending });
            navigationActions.update({ state: workbenchNavigationState(current), pending });
            controls.update({ state: workbenchControlState(current), selectionRequest: selection ? String(selectionRequest) : '', pending });
            commandView.update({ command: workbenchCommandState(current.command, current.transition), pending, busy: workbenchBusy(current) });
            results.update({ state: workbenchResultState(current), pending });
            previewView.update({ state: workbenchPreviewState(current) });
            tools.update({ state: workbenchToolsState(current), pending });
            projectPicker.update({ navigation: workbenchListingNavigation(current.navigation), pending });
            layout.update({ navigation: workbenchLayoutNavigation(current.navigation) });
            await Promise.all([layout.whenSettled(), navigation.whenSettled(), navigationActions.whenSettled(), controls.whenSettled(), commandView.whenSettled(), results.whenSettled(), tools.whenSettled(), previewView.whenSettled(), projectPicker.whenSettled()]);
        });
    };
    const unsubscribe = workbench.subscribe(render);
    await renderPromise;

    return Object.freeze({
        root: host,
        workbench,
        async whenSettled() {
            while (true) {
                const controlAction = controlPromise;
                const edits = editPromise;
                const build = buildPromise;
                await controlAction;
                await edits;
                await build;
                const renders = renderPromise;
                await renders;
                await Promise.all([layout.whenSettled(), navigation.whenSettled(), navigationActions.whenSettled(), controls.whenSettled(), commandView.whenSettled(), results.whenSettled(), tools.whenSettled(), previewView.whenSettled(), projectPicker.whenSettled()]);
                if (controlAction === controlPromise && edits === editPromise && build === buildPromise && renders === renderPromise) return;
            }
        },
        dispose() {
            active = false;
            unsubscribe();
            navigation.dispose();
            navigationActions.dispose();
            controls.dispose();
            tools.dispose();
            projectPicker.dispose();
            previewView.dispose();
            commandView.dispose();
            results.dispose();
            layout.dispose();
            if (!authoredLayout) mount.remove();
        },
    });
}

function featureTourCemExample(seed) {
    const examples = seed?.catalog?.examples;
    if (!Array.isArray(examples)) throw new Error('Feature Tour catalog has no examples');
    const example = examples.find(
        ({ packageId, contentType }) =>
            packageId === 'cem-ml' && typeof contentType === 'string' && contentType.includes('cem'),
    );
    if (!example) throw new Error('Feature Tour catalog has no editable CEM-ML example');
    return example;
}

function featureTourWorkbenches(seed, initialExample) {
    const scenarios = Array.isArray(seed?.catalog?.workbenches) ? seed.catalog.workbenches : [];
    const values = [initialExample, ...scenarios.filter(({ id }) => id !== initialExample.id)];
    return Object.freeze(
        values.map((entry, index) =>
            Object.freeze({
                ...entry,
                id: entry.id ?? (index === 0 ? 'source-editor' : `workbench-${index + 1}`),
                name:
                    entry.name ?? (index === 0 ? 'CEM-ML source editor' : humanizeKind(entry.kind ?? entry.operation)),
                dependencies: Object.freeze([...(entry.dependencies ?? [])]),
                commandArguments: entry.commandArguments ? Object.freeze([...entry.commandArguments]) : undefined,
            }),
        ),
    );
}

function workbenchDescriptor(example) {
    return Object.freeze({
        id: example.id,
        name: example.name ?? humanizeKind(example.kind ?? 'inspection'),
        operation: workbenchOperation(example),
        kind: example.kind ?? 'inspection',
        contentType: example.contentType,
        schema: example.schema,
    });
}

function workbenchOperation(example) {
    return example.kind && ['convert', 'query', 'transform', 'trace'].includes(example.operation)
        ? example.operation
        : 'parse';
}

function commandArgumentsForExample(example, projectId, scheme) {
    if (!Array.isArray(example.commandArguments)) return undefined;
    const inputUri = resourceUri(example.referenceUri ?? example.path, projectId, scheme);
    const dependencies = new Map(
        (example.dependencies ?? []).map((dependency) => [
            dependency.resourceId,
            resolveVirtualUri(inputUri, dependency.path),
        ]),
    );
    return Object.freeze(
        example.commandArguments.map((argument) => {
            if (argument === '$input') return inputUri;
            if (argument.startsWith('$resource:')) {
                const resourceId = argument.slice('$resource:'.length);
                const uri = dependencies.get(resourceId);
                if (!uri) throw new Error(`Feature Tour command references unavailable resource ${resourceId}`);
                return uri;
            }
            return argument;
        }),
    );
}

function resourceUri(uri, projectId, scheme = 'studio') {
    return /^[a-z][a-z0-9+.-]*:/i.test(uri) ? uri : new URL(uri, `${scheme}://${projectId}/`).href;
}

function resolveVirtualUri(baseUri, relativePath) {
    return new URL(relativePath, baseUri).href;
}

function projectValidation(result, presentation, source, revision) {
    const report = inlineValidateReport(result);
    const terminalDiagnostics = Array.isArray(result?.diagnostics?.items) ? result.diagnostics.items : [];
    const diagnostics = (Array.isArray(report?.diagnostics) ? report.diagnostics : terminalDiagnostics)
        .slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
            }
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
            }
        });
    }
    const summary = report?.summary ?? summaryFromDiagnostics(diagnostics);
    return {
        requestId: result?.requestId,
        exitCode: result?.exitCode,
        executionIdentity: result?.identity,
        revision,
        reportSummary: Object.freeze({ ...summary }),
        hardViolationCount: summary.hardViolationCount ?? diagnostics.filter(isHardDiagnostic).length,
        diagnostics: Object.freeze(diagnostics),
        provenance: Object.freeze(provenance),
        presentation,
        sourceByteLength: textEncoder.encode(source).byteLength,
        stale: false,
    };
}

function outputArtifactPath(uri, root) {
    if (!uri.startsWith(root)) return uri;
    const path = uri.slice(root.length);
    try { return decodeURI(path); } catch { return path; }
}

function projectCommandProjection(kind, mode, outcome, source, revision, expected, capturedBuild) {
    const result = outcome.result;
    const diagnostics = (result?.diagnostics?.items ?? [])
        .slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map((diagnostic) => normalizeDiagnostic(diagnostic));
    const provenance = [];
    diagnostics.forEach((diagnostic, diagnosticIndex) => {
        diagnostic.sourceMap?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { diagnosticIndex, frameIndex }));
            }
        });
    });
    for (const reference of result?.sourceMaps?.items ?? []) {
        const stack = reference.sourceMap?.storage === 'inline' ? reference.sourceMap.value : undefined;
        stack?.frames?.forEach((frame, frameIndex) => {
            if (provenance.length < CEM_STUDIO_LIMITS.structuredRows) {
                provenance.push(normalizeFrame(frame, { sourceMapId: reference.sourceMapId, frameIndex }));
            }
        });
    }
    const summary = summarizePortableOperation(kind, mode, result);
    const expectedMatches = expected === undefined ? undefined : matchesExpectedSummary(summary, expected);
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    const trace =
        kind === 'trace' && Array.isArray(operation?.value?.body?.events)
            ? operation.value.body.events.slice(0, CEM_STUDIO_LIMITS.structuredRows)
                .map((value, index) => Object.freeze({ path: `value.body.events.${index}`, value }))
            : collectOperationRecords(operation, ['trace', 'stages', 'events']);
    const graph =
        kind === 'transform' && mode === 'graph'
            ? collectOperationRecords(operation, ['artifacts', 'stages', 'nodes', 'edges'])
            : [];
    const outputs = Object.freeze([...(outcome.outputs ?? (outcome.output.available === false ? [] : [outcome.output]))]);
    const nativeOutputs = mode === 'graph' ? operation?.value?.value?.artifacts ?? []
        : ((kind === 'convert' ? operation?.value?.outputs?.items : operation?.value?.value?.outputs?.items) ?? [])
            .map(output => ({ ...output.response, destination: output.destination }));
    const artifacts = outputs.map(output => {
        const native = nativeOutputs.find(candidate => candidate.destination === output.uri);
        const frames = [];
        for (const span of native?.outputSpans ?? []) for (const frame of span.origin?.frames ?? []) {
            if (frame.transform?.kind !== 'SourceBoundary' || !frame.transform.uri) continue;
            const ranges = frame.span?.kind === 'Multi' ? frame.span.ranges : frame.span?.kind === 'Single' ? [frame.span.ranges] : [];
            for (const range of ranges) if (frames.length < CEM_STUDIO_LIMITS.structuredRows) {
                frames.push(Object.freeze({ ...normalizeFrame(frame, {}), range: Object.freeze({ ...range }), outputRange: span.outputRange, outputUri: output.uri }));
            }
        }
        const sourceMaps = (result?.sourceMaps?.items ?? []).filter(reference =>
            (output.sourceMapId != null && reference.sourceMapId === output.sourceMapId) || (output.handleId !== undefined && reference.owner?.handleId === output.handleId));
        return Object.freeze({ id: `${result.requestId}#${output.handleId ?? output.uri}`, uri: output.uri,
            path: outputArtifactPath(output.uri, capturedBuild.projectRootUri),
            handleId: output.handleId, sourceMapId: output.sourceMapId, contentType: output.contentType,
            byteLength: output.byteLength, sha256: output.sha256, available: output.available !== false,
            provenance: freezeWorkbenchValue(frames), sourceMaps: freezeWorkbenchValue(sourceMaps) });
    });
    const buildRun = Object.freeze({ ...capturedBuild, id: result.requestId,
        status: result.status ?? (result.exitCode === 0 ? 'succeeded' : 'failed'),
        executionIdentity: freezeWorkbenchValue(result.identity), admission: outcome.admission,
        artifacts: Object.freeze(artifacts), byteLength: outputs.reduce((sum, output) => sum + output.byteLength, 0) });
    const selectedArtifactId = artifacts.find(artifact => artifact.uri === outcome.output.uri)?.id;
    return {
        kind,
        mode,
        requestId: result?.requestId,
        exitCode: result?.exitCode,
        executionIdentity: result?.identity,
        revision,
        output: outcome.output,
        outputs,
        buildRun,
        selectedArtifactId,
        preview: createCemStudioPreview({
            bytes: outcome.output.bytes,
            contentType: outcome.output.contentType,
            label: `${humanizeKind(kind)} result preview`,
        }),
        nativeResult: result,
        diagnostics: Object.freeze(diagnostics),
        provenance: nativeOutputs.length ? artifacts.find(artifact => artifact.id === selectedArtifactId)?.provenance ?? Object.freeze([]) : Object.freeze(provenance),
        presentation: outcome.presentation,
        sourceByteLength: textEncoder.encode(source).byteLength,
        summary: Object.freeze(summary),
        expected: expected === undefined ? undefined : Object.freeze({ ...expected }),
        expectedMatches,
        trace: Object.freeze(trace),
        graph: Object.freeze(graph),
        stale: false,
    };
}

function summarizePortableOperation(kind, mode, result) {
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    const value = operation?.value;
    if (kind === 'convert') {
        return { kind, outputCount: value?.outputs?.items?.length ?? 0 };
    }
    if (kind === 'query') {
        return { kind, itemCount: sequenceItems(value?.output).length };
    }
    if (kind === 'transform') {
        const outputs = value?.value?.outputs?.items ?? [];
        const artifacts = value?.value?.artifacts ?? [];
        const primary = mode === 'graph' ? artifacts[0]?.primary : outputs[0]?.response?.primary;
        return {
            kind,
            mode,
            ...(mode === 'graph' ? { artifactCount: artifacts.length } : {}),
            itemCount: sequenceItems(primary).length,
        };
    }
    if (kind === 'trace') {
        return { kind, hasEvents: (value?.body?.events?.length ?? 0) > 0 };
    }
    return { kind, mode };
}

function sequenceItems(value) {
    return value?.result?.sequence?.items ?? value?.sequence?.items ?? [];
}

function matchesExpectedSummary(actual, expected) {
    return Object.entries(expected).every(([key, value]) => JSON.stringify(actual[key]) === JSON.stringify(value));
}

function collectOperationRecords(value, keys) {
    const records = [];
    const visit = (candidate, path = []) => {
        if (!candidate || typeof candidate !== 'object' || records.length >= CEM_STUDIO_LIMITS.structuredRows) return;
        for (const [key, child] of Object.entries(candidate)) {
            if (keys.includes(key) && Array.isArray(child)) {
                for (const [index, entry] of child.entries()) {
                    if (records.length >= CEM_STUDIO_LIMITS.structuredRows) break;
                    records.push(
                        Object.freeze({
                            path: [...path, key, index].join('.'),
                            value: entry,
                        }),
                    );
                }
            } else if (path.length < 8) {
                visit(child, [...path, key]);
            }
        }
    };
    visit(value);
    return records;
}

function inlineValidateReport(result) {
    const operation = result?.result?.storage === 'inline' ? result.result.value : undefined;
    if (operation?.kind === 'validate') return operation.value?.report;
    return result?.report?.storage === 'inline' ? result.report.value : undefined;
}

function normalizeDiagnostic(diagnostic) {
    const sourceMap = diagnostic?.sourceMap;
    const range = innermostStackRange(sourceMap) ?? {
        start: Number.isSafeInteger(diagnostic?.byteOffset) ? diagnostic.byteOffset : 0,
        len: 0,
    };
    return Object.freeze({
        code: diagnostic?.code ?? 'cem.studio.validation.unknown',
        severity: diagnostic?.severity ?? 'error',
        message: diagnostic?.message ?? 'Validation failed',
        uri: diagnostic?.uri,
        line: diagnostic?.line,
        column: diagnostic?.column,
        byteOffset: diagnostic?.byteOffset,
        range: Object.freeze(range),
        sourceMap,
    });
}

function normalizeFrame(frame, owner) {
    return Object.freeze({
        ...owner,
        sourceId: frame?.source_id,
        uri: frame?.transform?.kind === 'SourceBoundary' ? frame.transform.uri : undefined,
        transform: frame?.transform?.kind ?? 'Unknown',
        range: Object.freeze(firstFrameRange(frame) ?? { start: 0, len: 0 }),
    });
}

function innermostStackRange(stack) {
    // Native stacks retain enclosing tokenizer spans before specific AST spans.
    return stack?.frames?.map(firstFrameRange).findLast(range => range !== undefined);
}

function firstFrameRange(frame) {
    const span = frame?.span;
    if (!span) return undefined;
    if (span.kind === 'Single') return span.ranges;
    if (span.kind === 'Multi') return span.ranges?.[0];
    return undefined;
}

function summaryFromDiagnostics(diagnostics) {
    const summary = {
        inputCount: 1,
        infoCount: 0,
        warningCount: 0,
        errorCount: 0,
        fatalCount: 0,
        hardViolationCount: 0,
    };
    for (const diagnostic of diagnostics) {
        const key = `${diagnostic.severity}Count`;
        if (key in summary) summary[key] += 1;
        if (isHardDiagnostic(diagnostic)) summary.hardViolationCount += 1;
    }
    return summary;
}

function isHardDiagnostic({ severity }) {
    return severity === 'error' || severity === 'fatal';
}

function freezeState(value) {
    return Object.freeze({ ...value });
}

function freezeWorkbenchValue(value) {
    if (Array.isArray(value)) return Object.freeze(value.map((entry) => freezeWorkbenchValue(entry)));
    if (value && typeof value === 'object') {
        return Object.freeze(
            Object.fromEntries(Object.entries(value).map(([key, entry]) => [key, freezeWorkbenchValue(entry)])),
        );
    }
    return value;
}

function freezeValidation(value) {
    return Object.freeze({ ...value });
}

function freezeProjection(value) {
    return Object.freeze({ ...value });
}

function freezeCommand(value) {
    return Object.freeze({
        ...value,
        changes: Object.freeze([...(value.changes ?? [])]),
    });
}

function freezeCommandApplication(value) {
    return Object.freeze({
        ...value,
        target: value.target ? Object.freeze({ ...value.target }) : undefined,
        referencedResourceIds: Object.freeze([...(value.referencedResourceIds ?? [])]),
        targets: value.targets
            ? Object.freeze({
                  current: value.targets.current ? Object.freeze({ ...value.targets.current }) : undefined,
                  existing: Object.freeze(value.targets.existing.map((entry) => Object.freeze({ ...entry }))),
                  parents: Object.freeze(value.targets.parents.map((entry) => Object.freeze({ ...entry }))),
              })
            : undefined,
    });
}

function commandApplicationState(persisted, previous, parsed) {
    const pageKind = commandPageKindFromParsed(parsed, persisted.pageKind);
    const entries = persisted.project.entries.filter(({ kind }) => kind !== 'subproject');
    const preferredCurrentId = previous?.currentEntryId ?? persisted.currentEntry?.id;
    const currentEntry = entries.find(({ id }) => id === preferredCurrentId) ?? persisted.currentEntry;
    const describe = (entry) =>
        Object.freeze({
            id: entry.id,
            name: entry.name,
            kind: entry.kind,
            parentId: entry.parentId,
            compatible: entry.kind === pageKind,
        });
    const current = currentEntry ? describe(currentEntry) : undefined;
    const existing = entries
        .map(describe)
        .sort(
            (left, right) =>
                Number(right.compatible) - Number(left.compatible) ||
                left.name.localeCompare(right.name) ||
                left.id.localeCompare(right.id),
        );
    const parents = persisted.project.entries
        .filter(({ kind }) => kind === 'subproject')
        .map(({ id, name, parentId }) => Object.freeze({ id, name, parentId }))
        .sort((left, right) => left.name.localeCompare(right.name) || left.id.localeCompare(right.id));
    const defaultName = `${current?.name ?? 'Command'} ${humanizeKind(pageKind)}`;
    const newPageName = previous?.newPageName ?? defaultName;
    const newPageParentId = previous?.newPageParentId ?? current?.parentId;
    const defaultExisting = existing.find(({ compatible }) => compatible) ?? existing[0];
    const defaultMode = current?.compatible ? 'current' : 'new';
    let target = previous?.target;
    if (
        (target?.mode === 'current' && !current) ||
        (target?.mode === 'existing' && !existing.some(({ id }) => id === target.entryId)) ||
        !['current', 'existing', 'new'].includes(target?.mode)
    ) {
        target = undefined;
    }
    target ??=
        defaultMode === 'current'
            ? { mode: 'current', entryId: current.id }
            : { mode: 'new', name: newPageName, ...(newPageParentId ? { parentId: newPageParentId } : {}) };
    if (target.mode === 'current') target = { mode: 'current', entryId: current.id };
    if (target.mode === 'existing' && !target.entryId && defaultExisting) {
        target = { mode: 'existing', entryId: defaultExisting.id };
    }
    if (target.mode === 'new') {
        target = {
            mode: 'new',
            name: target.name ?? newPageName,
            ...((target.parentId ?? newPageParentId) ? { parentId: target.parentId ?? newPageParentId } : {}),
        };
    }
    const resultCurrent =
        previous?.result &&
        previous.result.projectRevision === persisted.projectRevision &&
        persisted.project.resources.some(
            ({ id, revision, sha256 }) =>
                id === previous.result.commandResource.id &&
                revision === previous.result.resourceRevision &&
                sha256 === previous.result.sha256,
        );
    return freezeCommandApplication({
        status: resultCurrent ? previous.status : previous?.result ? 'stale' : 'ready',
        currentEntryId: current?.id,
        newPageName,
        newPageParentId,
        referencedResourceIds: persisted.referencedResourceIds,
        targets: { current, existing, parents },
        target,
        result: previous?.result,
        execution: previous?.execution,
        confirmation: resultCurrent ? previous?.confirmation : undefined,
        error: resultCurrent ? previous?.error : undefined,
    });
}

function configureCommandApplication(application, target) {
    if (!application || !target || typeof target !== 'object') {
        throw new TypeError('CEM Studio command target must be an object');
    }
    let normalized;
    let newPageName = application.newPageName;
    let newPageParentId = application.newPageParentId;
    if (target.mode === 'current') {
        if (!application.targets.current) throw new Error('The workbench has no current configuration target');
        normalized = { mode: 'current', entryId: application.targets.current.id };
    } else if (target.mode === 'existing') {
        const entry = application.targets.existing.find(({ id }) => id === target.entryId);
        if (!entry) throw new Error(`Existing command-configuration target ${String(target.entryId)} is unavailable`);
        normalized = { mode: 'existing', entryId: entry.id };
    } else if (target.mode === 'new') {
        if (typeof target.name !== 'string') throw new TypeError('A new command configuration requires a name');
        newPageName = target.name;
        newPageParentId = target.parentId || undefined;
        normalized = {
            mode: 'new',
            name: newPageName,
            ...(newPageParentId ? { parentId: newPageParentId } : {}),
        };
    } else {
        throw new TypeError(`Unsupported CEM Studio command target mode: ${String(target.mode)}`);
    }
    return freezeCommandApplication({
        ...application,
        status: application.result ? 'stale' : 'ready',
        target: normalized,
        newPageName,
        newPageParentId,
        confirmation: undefined,
        error: undefined,
    });
}

function commandApplicationAfterEdit(application) {
    return freezeCommandApplication({
        ...application,
        status: application.result ? 'stale' : 'ready',
        confirmation: undefined,
        error: undefined,
    });
}

function freezeAppliedCommandResult(value) {
    return Object.freeze({
        ...value,
        entry: Object.freeze({ ...value.entry }),
        commandResource: Object.freeze({ ...value.commandResource }),
        commandBytes: value.commandBytes.slice(0),
    });
}

function isAppliedCommandCommit(persisted, applied, expectedBytes) {
    const resource = persisted.project.resources.find(({ id }) => id === applied.commandResource.id);
    const entry = persisted.project.entries.find(({ id }) => id === applied.entry.id);
    const content = persisted.contents[applied.commandResource.id];
    return (
        persisted.projectRevision === applied.projectRevision &&
        resource?.revision === applied.resourceRevision &&
        resource?.sha256 === applied.sha256 &&
        entry?.runConfigResourceId === applied.commandResource.id &&
        content !== undefined &&
        equalBytes(toBytes(content), expectedBytes) &&
        equalBytes(toBytes(applied.commandBytes), expectedBytes)
    );
}

function assertAppliedCommandCommit(persisted, applied, expectedBytes) {
    if (isAppliedCommandCommit(persisted, applied, expectedBytes)) return;
    const error = new Error('CEM Studio repository did not reload the exact applied command revision');
    error.code = 'cem.studio.workbench.apply_persistence_mismatch';
    throw error;
}

function commandProjectionMode(parsed, operation, fallback) {
    if (operation === 'parse') return parsed?.options?.format ?? 'ast';
    if (operation === 'inspect') return parsed?.options?.show ?? 'summary';
    if (operation === 'convert')
        return parsed?.options?.to_format ?? parsed?.options?.to_content_type ?? fallback ?? 'convert';
    if (operation === 'query') return parsed?.options?.query_content_type ?? fallback ?? 'query';
    if (operation === 'transform')
        return parsed?.options?.config ? 'graph' : fallback === 'transformation-graph' ? 'graph' : 'direct';
    if (operation === 'trace') return parsed?.options?.format ?? fallback ?? 'trace';
    return fallback ?? 'unknown';
}

function humanizeKind(value) {
    return String(value)
        .split('-')
        .map((part) => part[0]?.toUpperCase() + part.slice(1))
        .join(' ');
}

function commandPageKindFromParsed(parsed, fallback = 'inspection') {
    const operation = parsed?.commandPath?.[0];
    if (operation === 'parse' || operation === 'inspect') return 'inspection';
    if (operation === 'convert') return 'conversion';
    if (operation === 'query') return 'query';
    if (operation === 'trace') return 'trace';
    if (operation === 'transform') return parsed?.options?.config ? 'transformation-graph' : 'transformation';
    return fallback;
}

function commandState(preview, persisted, previousApplication) {
    return freezeCommand({
        projection: 'studio',
        status: 'current',
        current: preview,
        draftText: preview.text,
        parsed: preview.parsed,
        preview,
        changes: [],
        diagnostic: undefined,
        copy: undefined,
        revision: commandRevision(persisted),
        application: commandApplicationState(persisted, previousApplication, preview.parsed),
    });
}

function commandRevision(persisted) {
    return Object.freeze({
        projectRevision: persisted.projectRevision,
        resourceRevision: persisted.resourceRevision,
        sha256: persisted.sha256,
    });
}

function semanticChanges(current, preview) {
    const before = new Map();
    const after = new Map();
    flattenSemanticValue(current, [], before);
    flattenSemanticValue(preview, [], after);
    const paths = [...new Set([...before.keys(), ...after.keys()])].sort();
    return Object.freeze(
        paths.flatMap((path) => {
            const left = before.get(path);
            const right = after.get(path);
            if (JSON.stringify(left) === JSON.stringify(right)) return [];
            return [
                Object.freeze({
                    category: path.split('.')[0] ?? 'command',
                    path,
                    kind: left === undefined ? 'added' : right === undefined ? 'removed' : 'changed',
                    before: left,
                    after: right,
                }),
            ];
        }),
    );
}

function flattenSemanticValue(value, path, output) {
    if (Array.isArray(value) || value === null || typeof value !== 'object') {
        output.set(path.join('.'), value);
        return;
    }
    const entries = Object.entries(value);
    if (entries.length === 0) {
        output.set(path.join('.'), value);
        return;
    }
    for (const [key, entry] of entries) flattenSemanticValue(entry, [...path, key], output);
}

function normalizedError(error) {
    return Object.freeze({
        code: error?.code ?? 'cem.studio.workbench.failed',
        message: redactCemStudioSecrets(error instanceof Error ? error.message : String(error)),
    });
}

function repositoryRequest(operation, parameters) {
    return {
        protocolVersion: 1,
        repository: CEM_STUDIO_REPOSITORY_ID,
        operation,
        requestRevision: 1,
        parameters,
    };
}

function toBytes(value) {
    if (value instanceof Uint8Array) return value;
    if (value instanceof ArrayBuffer) return new Uint8Array(value);
    if (ArrayBuffer.isView(value)) return new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
    throw new TypeError('Feature Tour resource bytes are unavailable');
}

function equalBytes(left, right) {
    return left.byteLength === right.byteLength && left.every((byte, index) => byte === right[index]);
}

function utf8ByteOffsetToCodeUnit(value, target) {
    const bounded = Math.max(0, Math.min(target, textEncoder.encode(value).byteLength));
    let bytes = 0;
    let codeUnits = 0;
    for (const character of value) {
        const width = textEncoder.encode(character).byteLength;
        if (bytes + width > bounded) break;
        bytes += width;
        codeUnits += character.length;
    }
    return codeUnits;
}

function workbenchBusy(state) {
    return ['saving', 'validating', 'projecting'].includes(state.status)
        || ['applying', 'running'].includes(state.command?.application?.status);
}

// Forward service data used by this view, excluding potentially large result
// bytes, graphs and command previews. Presentation remains in the declaration.
function workbenchLayoutNavigation(navigation) {
    return navigation && { selected: { name: navigation.selected.name }, content: navigation.content };
}
function workbenchListingNavigation(navigation) {
    return navigation && { selected: { name: navigation.selected.name, description: navigation.selected.description },
        children: navigation.children, content: navigation.content, path: navigation.path,
        sampleText: navigation.sampleText, action: navigation.action, error: navigation.error };
}
function workbenchNavigationState(state) {
    const navigation = state.navigation;
    return { transition: state.transition, navigationError: state.navigationError,
        navigation: navigation && { samples: navigation.samples, saved: navigation.saved, files: navigation.files,
            selection: navigation.selection, expanded: navigation.expanded, count: navigation.count, countKind: navigation.countKind,
            busy: navigation.busy, canClone: navigation.canClone, canDelete: navigation.canDelete } };
}

function workbenchControlState(state) {
    const { status, projectId, workbenchId, workbenches, draft, dirty, projectRevision,
        resourceRevision, selection, error, path, description, contentType, validation } = state;
    return { graphConfiguration: state.graphConfiguration, structure: state.structure, home: state.home, explorer: state.explorer, status, projectId, workbenchId, workbenches, draft, dirty, projectRevision,
        resourceRevision, selection, error, path, description, contentType, validation,
        editable: state.editable !== false, runnable: state.runnable !== false, transition: state.transition,
        sessionStatus: state.sessionStatus, draftRecovery: state.draftRecovery, navigationError: state.navigationError, resourceAvailable: state.resourceAvailable ?? Boolean(state.resourceId),
        command: { status: state.command?.status, application: { status: state.command?.application?.status } } };
}

function workbenchCommandState(command, transition) {
    if (!command) return null;
    const { status, draftText, copy, diagnostic, revision, current, changes, application } = command;
    const { result, execution: _execution, ...targeting } = application ?? {};
    const text = value => typeof value === 'string' ? value : JSON.stringify(value);
    let fields = [], groups = [];
    if (status !== 'invalid') {
        try { fields = describeCemStudioOperationFields(draftText); groups = describeCemStudioOperationGroups(draftText); } catch { /* Invalid literal text stays repairable in the command editor. */ }
    }
    return { fields, groups, formVersion: command.formVersion, formError: command.formError, status, draftText, copy, diagnostic, revision, current: { commonVersion: current.commonVersion },
        changes: changes.map(({ category, path, kind, before, after }) => ({ category, path, kind,
            beforeText: text(before), afterText: text(after) })),
        application: application ? { ...targeting, navigationPending: Boolean(transition), result: result && {
            projectRevision: result.projectRevision, resourceRevision: result.resourceRevision } } : null };
}

// Transport only the data this declaration consumes. Preserve exact downloadable
// bytes in the service; serialize typed record values without presentation labels.
function workbenchToolsState(state) {
    return { status: state.status, editable: state.editable !== false,
        graphConfiguration: state.graphConfiguration,
        command: { status: state.command?.status, application: { status: state.command?.application?.status } } };
}

function workbenchPreviewState(state) {
    const project = state.projectPreview;
    const projection = project ? project.selectedArtifact ? project.projection : undefined : state.projection;
    return { projection: projection && { preview: project?.preview ?? projection.preview,
        output: projection.output && { available: projection.output.available } },
        projectPreview: project && { projectId: project.projectId, configurations: project.configurations, configurationId: project.configurationId,
            pins: project.pins, pinId: project.pinId, pinSelection: project.pinSelection, pinBusy: project.pinBusy, pinError: project.pinError, pinMessage: project.pinMessage, canPin: project.canPin,
            pinnedConfiguration: project.pinId ? project.projection?.buildRun.configuration : undefined,
            pinnedInputs: project.pinId ? project.projection?.buildRun.inputs : undefined,
            pinnedArtifacts: project.pinId ? project.artifacts.map(({ path, sha256, byteLength }) => ({ path, sha256, byteLength })) : undefined,
            entryArtifact: project.entryArtifact, entryEditable: project.entryEditable, canEditEntry: project.canEditEntry, entryBusy: project.entryBusy, entryError: project.entryError, entryMessage: project.entryMessage,
            artifactPath: project.selectedArtifact?.path ?? project.artifactPath, available: project.available, busy: project.busy, attempt: project.attempt,
            stale: project.stale, hasRun: Boolean(project.projection), missingArtifact: project.missingArtifact,
            selected: Boolean(project.selectedArtifact), artifacts: project.artifacts.map(({ path }) => ({ path })),
            runId: project.projection?.buildRun.id, projectRevision: project.projection?.buildRun.projectRevision,
            transfer: project.projection?.transfer } };
}

function workbenchResultState(state) {
    const { status, error, operation, path, contentType, schema, expected, validation, command, projection } = state;
    const pinned = projection?.expected ?? expected;
    const actual = projection?.summary;
    const expectedRows = pinned ? [...new Set([...Object.keys(pinned), ...Object.keys(actual ?? {})])].sort()
        .map(key => ({ key, expectedText: JSON.stringify(pinned[key]), actualText: JSON.stringify(actual?.[key]) })) : [];
    const records = values => (values ?? []).slice(0, CEM_STUDIO_LIMITS.structuredRows)
        .map(({ path, value }) => ({ path, text: typeof value === 'string' ? value : JSON.stringify(value) }));
    return { status, error, operation, path, contentType, schema, expected, expectedRows,
        validation: validation && { reportSummary: validation.reportSummary, diagnostics: validation.diagnostics,
            provenance: validation.provenance },
        command: { current: { argv: command?.current?.argv }, application: { status: command?.application?.status } },
        projection: projection && { kind: projection.kind, mode: projection.mode, stale: projection.stale,
            selectedArtifactId: projection.selectedArtifactId,
            artifacts: projection.buildRun.artifacts.map(({ id, path, contentType, byteLength }) => ({ id, path, contentType, byteLength })),
            build: { id: projection.buildRun.id, configuration: projection.buildRun.configuration.name,
                byteLength: projection.buildRun.byteLength, count: projection.buildRun.artifacts.length },
            expectedMatches: projection.expectedMatches, executionIdentity: projection.executionIdentity,
            revision: projection.revision, diagnostics: projection.diagnostics, provenance: projection.provenance,
            preview: projection.preview, expected: projection.expected, summary: projection.summary,
            transfer: projection.transfer, output: projection.output && {
                available: projection.output.available, uri: projection.output.uri, sha256: projection.output.sha256,
                contentType: projection.output.contentType, byteLength: projection.output.byteLength },
            trace: records(projection.trace), graph: records(projection.graph) } };
}

async function browserDownload({ filename, contentType, bytes }) {
    const url = URL.createObjectURL(new Blob([bytes], { type: contentType }));
    try {
        const link = document.createElement('a');
        link.href = url;
        link.download = filename;
        link.hidden = true;
        document.body.append(link);
        link.click();
        link.remove();
    } finally {
        URL.revokeObjectURL(url);
    }
}

// Compatibility exports for existing embedders. All projects use the same workspace.
export const createCemStudioFeatureTourWorkbench = createCemStudioWorkbench;
export const mountCemStudioFeatureTourWorkbench = mountCemStudioWorkbench;
