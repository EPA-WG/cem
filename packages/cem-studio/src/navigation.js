import { captureCemStudioBuildSnapshot, openCemStudioBuildSnapshot } from './build-snapshots.js';
import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { createCemStudioProjectWorkbench, createCemStudioWorkbench } from './workbench.js';
import { projectResourceUri } from './resource-reference.js';
import { registerCemArtifactTree } from '@epa-wg/cem-elements';

const request = (operation, parameters) => ({ protocolVersion: 1, repository: CEM_STUDIO_REPOSITORY_ID, requestRevision: 1, operation, parameters });
const routeKey = route => new URLSearchParams(route).toString() || 'samples';
const node = (route, name, kind, description = '', children = []) => ({ key: routeKey(route), route, name, kind, description, children,
    icon: kind });

/** Navigation is domain data. Views render this model; browser history is only a transport. */
export async function createCemStudioNavigation({ workbench, repository, examples = [], collections = [], browser, validator, openBuildWorkbench }) {
    const subscribers = new Set();
    const bundles = new Map();
    let projects = [], selected = node({}, 'Samples', 'folder', 'Predefined example projects. Clone a sample to save your own version.');
    let opened, selection = {}, error, action, pendingRoute, busy = false, version = 0, refreshVersion = 0, disposed = false;
    let base = workbench.snapshot();
    let acceptedUrl = browser ? new URL(browser.location.href) : new URL('https://studio.invalid/');
    for (const key of ['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration']) acceptedUrl.searchParams.delete(key);
    let model;
    const sampleRoot = () => {
        const roots = [];
        for (const example of examples) {
            // A hierarchy has one parent; additional catalog memberships remain metadata.
            const ids = [example.collections?.[0] ?? 'examples'];
            for (const id of ids) {
                let group = roots.find(item => item.route.collection === id);
                if (!group) {
                    const collection = collections.find(item => item.id === id);
                    group = node({ collection: id }, collection?.name ?? (id === 'examples' ? 'Examples' : id), 'folder', collection?.description ?? 'Example projects');
                    roots.push(group);
                }
                group.children.push(node({ sample: example.id }, example.name, 'project', example.description ?? 'Open this sample or clone it into Saved projects.'));
            }
        }
        return node({}, 'Samples', 'folder', 'Predefined example projects. Clone a sample to save your own version.', roots);
    };
    function fileTree(project, route) {
        const root = node(route, project.name, 'project', project.description ?? 'Project files and configurations');
        for (const resource of project.resources) {
            const parts = (resource.path ?? resource.id).split('/');
            let parent = root, path = '';
            for (const part of parts.slice(0, -1)) {
                path += `${part}/`;
                let child = parent.children.find(item => item.kind === 'folder' && item.name === part);
                if (!child) { child = node({ ...route, folder: path }, part, 'folder', `Files in ${path}`); parent.children.push(child); }
                parent = child;
            }
            parent.children.push({ ...node({ ...route, file: resource.id }, parts.at(-1), 'file', resource.description ?? resource.contentType), resourceId: resource.id, path: resource.path, contentType: resource.contentType });
        }
        if (project.entries?.length) {
            const configurations = node({ ...route, folder: '@configurations' }, 'Configurations', 'folder', 'Saved commands and transformation configurations');
            const folders = new Map(project.entries.filter(entry => entry.kind === 'subproject').map(entry => [entry.id,
                node({ ...route, folder: `@configurations/${entry.id}` }, entry.name, 'folder', entry.description ?? 'Subproject configurations')]));
            for (const entry of project.entries) {
                const parent = folders.get(entry.parentId) ?? configurations;
                parent.children.push(folders.get(entry.id) ?? node({ ...route, configuration: entry.id }, entry.name, 'configuration', entry.description ?? entry.kind));
            }
            root.children.push(configurations);
        }
        const sort = item => { item.children.sort((a, b) => (a.kind === 'folder' ? 0 : 1) - (b.kind === 'folder' ? 0 : 1) || a.name.localeCompare(b.name)); item.children.forEach(sort); };
        sort(root); return root;
    }
    const find = (root, key) => root.key === key ? root : root.children.map(child => find(child, key)).find(Boolean);
    function publish() {
        if (disposed) return;
        const samples = sampleRoot();
        const saved = node({ view: 'projects' }, 'Saved projects', 'folder', 'Projects saved in this browser.', projects.map(project => node({ project: project.id }, project.name, 'project', project.description ?? 'Saved in this browser')));
        const files = opened ? fileTree(opened.project, opened.route) : undefined;
        selected = find(samples, routeKey(selection)) ?? find(saved, routeKey(selection)) ?? (files && find(files, routeKey(selection))) ?? selected;
        // Project roots list their resources, rather than the catalog's project leaf.
        if (files && routeKey(selection) === files.key) selected = files;
        const children = selected.children;
        const count = selected.kind === 'file' || selected.kind === 'configuration' ? 1 : children.length;
        const countKind = ['file', 'configuration'].includes(selected.kind) ? selected.kind : children.every(item => item.kind === 'project') ? 'project' : children.every(item => item.kind === 'file') ? 'file' : children.every(item => item.kind === 'folder') ? 'folder' : 'item';
        const path = selected.path ?? (selection.folder || (selection.configuration ? base.path || selected.name : ''));
        const title = opened && (selection.project || selection.sample) ? `${opened.project.name}${path ? ` · ${path}` : ''} - CEM Studio` : `${selected.name} - CEM Studio`;
        const ancestors = [];
        const collect = (root, key, chain = []) => { if (root.key === key) return chain; for (const child of root.children) { const found = collect(child, key, [...chain, root.key]); if (found) return found; } };
        for (const root of [samples, saved, files].filter(Boolean)) ancestors.push(...(collect(root, selected.key) ?? []));
        model = { samples, saved, files, selection: { ...selection, key: selected.key, kind: selected.kind }, selected, children, resources: opened?.project.resources ?? [], count, countKind, title, path,
            expanded: [...new Set([...ancestors, ...(selected.children.length ? [selected.key] : [])])].join(' '), busy, error, action,
            canClone: selected.kind === 'project' || (selected.kind === 'file' && Boolean(selection.project)),
            canDelete: Boolean(selection.project) && ['project', 'file'].includes(selected.kind),
            createKind: selection.project ? 'file' : 'project',
            content: selected.kind === 'file' || selected.kind === 'configuration' ? (selection.sample ? 'sample-file' : 'file') : 'folder',
            sampleText: opened?.text ?? '',
            status: busy ? 'opening' : error ? 'error' : base.dirty ? 'dirty' : selection.sample ? 'sample' : !selection.project || base.status === 'empty' ? 'ready' : base.status,
        };
        for (const notify of subscribers) notify(snapshot());
    }
    function snapshot() {
        const projectPreview = preview?.snapshot();
        const entryEditable = Boolean(selection.project && projectPreview?.available && projectPreview.configurations.some(item => item.id === projectPreview.configurationId && ['conversion', 'query', 'transformation', 'transformation-graph'].includes(item.kind)));
        return { ...base, navigation: model, projectPreview: projectPreview && { ...projectPreview,
            entryEditable, canEditEntry: Boolean(entryEditable && !projectPreview.busy && !base.structure?.busy && !base.dirty && !base.transition
                && !(base.command && base.command.draftText !== base.command.current?.text)),
            entryBusy: Boolean(base.structure?.busy), entryError: base.structure?.error, entryMessage: base.structure?.message } };
    }
    async function refresh() {
        const currentVersion = ++refreshVersion;
        const projectId = selection.project, previousOpened = opened;
        const response = await repository.query(request('list-projects', {}));
        if (disposed || currentVersion !== refreshVersion) return;
        projects = response.value;
        if (projectId && previousOpened) {
            const record = (await repository.query(request('get-project', { projectId }))).value;
            if (disposed || currentVersion !== refreshVersion) return;
            if (record && opened === previousOpened && selection.project === projectId) {
                opened = { ...opened, project: record };
                preview?.setContext({ ...opened, key: routeKey(opened.route) });
            }
            if (!record || record.trashedAt) preview?.setContext(undefined);
        }
        publish();
    }
    function writeUrl(route, mode) {
        if (!browser) return;
        const url = new URL(browser.location.href);
        for (const name of ['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration', 'build', 'artifact']) url.searchParams.delete(name);
        const output = preview?.snapshot();
        const combined = { ...route, ...(output?.configurationId ? { build: output.configurationId } : {}), ...(output?.currentArtifactPath ? { artifact: output.currentArtifactPath } : {}) };
        for (const [name, value] of Object.entries(combined)) url.searchParams.set(name, value);
        if (mode === 'push' && url.href !== acceptedUrl.href) browser.history.pushState(null, '', url);
        else if (mode !== 'push' || url.href !== browser.location.href) browser.history.replaceState(null, '', url);
        acceptedUrl = url;
    }
    function admitPreview(nextOpened, settings) {
        preview.setContext(nextOpened && { ...nextOpened, key: routeKey(nextOpened.route) }, settings);
    }
    async function navigate(requestedRoute, mode = 'push', preservePreview = false) {
        const { build, artifact, ...route } = requestedRoute;
        const settings = { build, artifact, configuration: route.configuration, reset: !preservePreview };
        const currentVersion = ++version;
        if (pendingRoute) { pendingRoute = undefined; await workbench.resolveNavigation('cancel'); }
        busy = true; error = undefined; action = undefined; publish();
        try {
            if (['view', 'collection', 'project', 'sample'].filter(key => route[key]).length > 1) throw new Error('The Studio URL has conflicting roots.');
            if ((route.project && route.sample) || (route.file && (route.folder || route.configuration)) || (route.folder && route.configuration)) throw new Error('The Studio URL has conflicting selections.');
            if ((route.file || route.folder || route.configuration) && !route.project && !route.sample) throw new Error('Choose a project before a file or folder.');
            let nextOpened;
            if (route.sample) {
                const example = examples.find(item => item.id === route.sample);
                if (!example) throw new Error('The requested sample is unavailable.');
                let bundle = bundles.get(example.id);
                if (!bundle) { bundle = await example.load(); bundles.set(example.id, bundle); }
                nextOpened = { project: bundle.project, bundle, route: { sample: example.id }, text: '' };
                if (route.file) {
                    const resource = bundle.project.resources.find(item => item.id === route.file);
                    if (!resource) throw new Error('The requested sample file is unavailable.');
                    const bytes = bundle.contents[resource.id];
                    nextOpened.text = new TextDecoder('utf-8', { fatal: true }).decode(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes ?? []));
                }
                if (route.configuration) {
                    const entry = bundle.project.entries.find(item => item.id === route.configuration);
                    if (!entry) throw new Error('The requested configuration is unavailable.');
                    const resource = bundle.project.resources.find(item => item.id === entry.runConfigResourceId);
                    const bytes = bundle.contents[resource?.id];
                    nextOpened.text = new TextDecoder().decode(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes ?? []));
                }
            } else if (route.project) {
                const record = (await repository.query(request('get-project', { projectId: route.project }))).value;
                if (!record || !projects.some(item => item.id === record.id)) throw new Error('The requested project is missing or in Trash.');
                nextOpened = { project: record, route: { project: record.id } };
            } else if (route.collection) {
                if (!find(sampleRoot(), routeKey(route))) throw new Error('The requested samples folder is unavailable.');
            } else if (route.view && route.view !== 'projects') throw new Error('The requested Studio view is unavailable.');
            if (currentVersion !== version) return;
            if (nextOpened && !find(fileTree(nextOpened.project, nextOpened.route), routeKey(route))) throw new Error('The requested file or directory is unavailable.');
            if (route.project) {
                const target = { projectId: route.project, ...(route.file ? { resourceId: route.file } : {}), ...(route.configuration ? { entryId: route.configuration } : {}) };
                pendingRoute = { route, mode, opened: nextOpened, settings, target: JSON.stringify(target) };
                const current = workbench.snapshot();
                if (current.workbenchId !== JSON.stringify(target) && JSON.stringify(current.transition?.target) !== JSON.stringify(target)) await workbench.selectWorkbench(JSON.stringify(target));
                if (currentVersion !== version) return;
                if (workbench.snapshot().transition) return;
                if (workbench.snapshot().workbenchId !== JSON.stringify(target)) throw new Error('Finish the pending operation before changing files.');
                pendingRoute = undefined;
            }
            selection = route; opened = nextOpened; admitPreview(opened, settings); writeUrl(route, mode);
        } catch (cause) { if (currentVersion !== version) return; pendingRoute = undefined; error = { message: cause.message }; if (mode === 'pop' && browser) browser.history.replaceState(null, '', acceptedUrl); }
        finally { if (currentVersion === version) { busy = false; publish(); } }
    }
    const preview = createCemStudioProjectPreview({ repository, validator, openBuildWorkbench });
    const unsubscribePreview = preview.subscribe(publish);
    const unsubscribe = workbench.subscribe(state => { base = state; preview.updateSource(state); publish(); });
    const unsubscribeRepository = repository.subscribe?.(0, () => { void preview.refreshPins(); void refresh().catch(cause => { error = { message: cause.message }; publish(); }); });
    const readUrl = () => {
        const params = new URL(browser.location.href).searchParams;
        return Object.fromEntries(['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration', 'build', 'artifact'].filter(key => params.has(key)).map(key => [key, params.get(key)]));
    };
    const onPop = () => { void navigate(readUrl(), 'pop'); };
    await refresh();
    if (browser) { await navigate(readUrl(), 'replace'); browser.addEventListener('popstate', onPop); }
    const api = {
        ...workbench, snapshot,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        async openEntity(key) { await navigate(key === 'samples' ? {} : Object.fromEntries(new URLSearchParams(key)), 'push', true); },
        selectPreviewConfiguration(id) { preview.selectConfiguration(id); writeUrl(selection, 'push'); },
        selectPreviewArtifact(path) { preview.selectArtifact(path); if (!preview.snapshot().pinId) writeUrl(selection, 'push'); },
        pinProjectBuild() { return preview.pin(); },
        selectPinnedBuild(key) { return preview.selectPin(key); },
        deletePinnedBuild() { return preview.deletePin(); },
        async saveBuildEntryArtifact(path) {
            if (!snapshot().projectPreview?.canEditEntry) return;
            await workbench.editStructure({ kind: 'set-entry-artifact', id: preview.snapshot().configurationId, entryArtifact: path });
            await refresh(); writeUrl(selection, 'replace');
        },
        async buildProjectPreview() {
            const key = preview.snapshot().contextKey;
            await preview.build();
            if (key === preview.snapshot().contextKey) writeUrl(selection, 'replace');
        },
        cancelProjectPreview() { preview.cancel(); },
        copyPreview(writeText) { return preview.copy(writeText); },
        downloadPreview(save) { return preview.download(save); },
        async selectWorkbench(id) {
            const target = JSON.parse(id);
            await navigate({ project: target.projectId, ...(target.resourceId ? { file: target.resourceId } : {}), ...(target.entryId ? { configuration: target.entryId } : {}) }, 'push', true);
        },
        async navigateProvenance(index) {
            const result = await workbench.navigateProvenance(index);
            const current = workbench.snapshot();
            const target = current.transition?.target ?? JSON.parse(current.workbenchId);
            await navigate({ project: target.projectId, ...(target.resourceId ? { file: target.resourceId } : {}), ...(target.entryId ? { configuration: target.entryId } : {}) }, 'push', true);
            return result;
        },
        async resolveNavigation(decision) {
            await workbench.resolveNavigation(decision);
            if (decision === 'cancel') { pendingRoute = undefined; if (browser) browser.history.replaceState(null, '', acceptedUrl); }
            else if (pendingRoute && !workbench.snapshot().transition && workbench.snapshot().workbenchId === pendingRoute.target) {
                const pending = pendingRoute; pendingRoute = undefined;
                selection = pending.route; opened = pending.opened; admitPreview(opened, pending.settings); writeUrl(selection, pending.mode);
            }
            publish();
        },
        requestAction(kind) { if (kind === 'clone' && !model.canClone || kind === 'delete' && !model.canDelete) return; action = { kind, targetName: selected.name, createKind: kind === 'clone' && selected.kind === 'project' ? 'project' : model.createKind }; publish(); },
        cancelAction() { if (!busy) action = undefined; publish(); },
        async submitAction(fields) {
            const kind = action?.kind;
            if (!kind || busy) return;
            error = undefined; busy = true; publish();
            try {
                const name = fields.name?.[0]?.trim();
                if (kind !== 'delete' && (!name || name.length > 120)) throw new Error('Enter a name of 1–120 characters.');
                if (kind === 'create' && !selection.project) {
                    const previousIds = new Set(projects.map(item => item.id));
                    await workbench.createProject(name); await refresh();
                    if (workbench.snapshot().home?.error) throw new Error(workbench.snapshot().home.error.message);
                    const created = projects.find(item => !previousIds.has(item.id));
                    if (created) await navigate({ project: created.id });
                } else if (kind === 'clone' && selected.kind === 'project') {
                    let bundle = selection.sample ? await examples.find(item => item.id === selection.sample).load() : (await repository.query(request('export-project', { projectId: selection.project }))).value;
                    bundle = structuredClone(bundle);
                    const id = `project-${crypto.randomUUID()}`, now = new Date().toISOString();
                    bundle.project = { ...bundle.project, id, name, rootUri: `studio://${id}/`, revision: 1, createdAt: now, updatedAt: now };
                    await repository.execute(request('import-project', { bundle, mode: 'create' })); await workbench.refreshNavigation(); await refresh(); await navigate({ project: id });
                } else if (kind === 'delete' && selected.kind === 'project') {
                    if (base.dirty || base.command && base.command.draftText !== base.command.current.text) throw new Error('Save or reset your drafts before deleting the project.');
                    await workbench.setProjectTrashed(selection.project, true); await refresh(); await navigate({ view: 'projects' });
                } else {
                    const directory = selection.folder && !selection.folder.startsWith('@configurations') ? selection.folder : selected.path?.includes('/') ? selected.path.slice(0, selected.path.lastIndexOf('/') + 1) : '';
                    const path = `${directory}${name ?? ''}`;
                    await workbench.editStructure({ kind: kind === 'delete' ? 'delete-resource' : kind === 'clone' ? 'copy-resource' : 'create-resource', id: selected.resourceId, path, content: '' });
                    if (workbench.snapshot().structure?.error) throw new Error(workbench.snapshot().structure.error.message);
                    await refresh();
                    await navigate({ project: selection.project, ...(kind !== 'delete' ? { file: opened.project.resources.find(item => item.path === path)?.id } : {}) });
                }
                action = undefined;
            } catch (cause) { error = { message: cause.message }; }
            finally { busy = false; }
            publish();
        },
        dispose() { disposed = true; ++version; ++refreshVersion; unsubscribe(); unsubscribePreview(); preview.dispose(); unsubscribeRepository?.(); browser?.removeEventListener('popstate', onPop); subscribers.clear(); workbench.dispose(); },
    };
    return Object.freeze(api);
}

/** Project build orchestration. Owns one bounded successful run, never an editor. */
export function createCemStudioProjectPreview({ repository, validator, openBuildWorkbench }) {
    const subscribers = new Set();
    let context, configurationId, artifactPath, explicitArtifact = false, success, successStamp, artifactTree, attempt, active, source, disposed = false, generation = 0;
    let pins = [], pinId, pinSelection, pinned, pinnedTree, pinnedPath, pinBusy = false, pinError, pinMessage, pinVersion = 0, pinListVersion = 0, pinListController, pinReadController;
    const viewed = () => pinned ?? success;
    const selectedPath = () => artifactPath?.split(/[?#]/)[0];
    const treeFor = projection => {
        const artifacts = projection.buildRun.artifacts.filter(item => !/^[a-z][\w+.-]*:/i.test(item.path)).map(item => ({
            path: item.path, contentType: item.contentType, bytes: projection.outputs.find(output => output.uri === item.uri).bytes }));
        return artifacts.length ? registerCemArtifactTree(artifacts) : undefined;
    };
    function closePin() {
        ++pinVersion; pinReadController?.abort(); pinned?.dispose(); pinnedTree?.dispose(); pinned = undefined; pinnedTree = undefined;
        pinId = undefined; pinSelection = undefined; pinnedPath = undefined; pinBusy = false;
    }
    async function refreshPins() {
        const key = context?.key, projectId = context?.project.id, version = ++pinListVersion;
        pinListController?.abort();
        if (!projectId || disposed) return;
        const controller = new AbortController(); pinListController = controller;
        try {
            const result = await repository.query(request('list-build-snapshots', { projectId }), controller.signal);
            if (disposed || context?.key !== key || pinListVersion !== version) return;
            pins = result.value;
            if (pinId && !pins.some(item => item.key === pinId) || pinSelection && !pins.some(item => item.key === pinSelection)) closePin();
        } catch (error) { if (!controller.signal.aborted && !disposed && context?.key === key && pinListVersion === version) pinError = { message: error.message }; }
        publish();
    }
    async function pin() {
        if (!success || pinSelection || pinBusy || active || disposed) return;
        const key = context.key, version = ++pinVersion;
        pinBusy = true; pinError = undefined; pinMessage = undefined; publish();
        try {
            await repository.execute(request('pin-build-snapshot', { snapshot: captureCemStudioBuildSnapshot(success.snapshot().projection) }));
            if (disposed || context?.key !== key || version !== pinVersion) return;
            pinMessage = 'Successful build pinned for offline reopening.';
            await refreshPins();
        } catch (error) { if (!disposed && version === pinVersion) pinError = { message: error.message }; }
        finally { if (!disposed && version === pinVersion) { pinBusy = false; publish(); } }
    }
    async function selectPin(id) {
        if (disposed || active) return;
        if (!id) { closePin(); pinError = undefined; publish(); return; }
        const projectId = context?.project.id, key = context?.key, version = ++pinVersion;
        if (!projectId) return;
        pinReadController?.abort(); pinSelection = id;
        const controller = new AbortController(); pinReadController = controller;
        pinBusy = true; pinError = undefined; pinMessage = undefined; publish();
        let candidate, tree;
        try {
            const { value } = await repository.query(request('get-build-snapshot', { projectId, key: id }), controller.signal);
            if (disposed || context?.key !== key || version !== pinVersion) return;
            candidate = openCemStudioBuildSnapshot(value);
            const path = value.run.configuration.entryArtifact ?? defaultArtifact(value.run.artifacts)?.path;
            const selected = value.run.artifacts.find(item => item.path === path);
            if (selected) candidate.selectArtifact(selected.id);
            tree = treeFor(candidate.snapshot().projection);
            pinned?.dispose(); pinnedTree?.dispose();
            pinned = candidate; pinnedTree = tree; pinId = id; pinnedPath = path; candidate = undefined; tree = undefined;
        } catch (error) { if (!disposed && version === pinVersion) pinError = { message: error.message }; }
        finally {
            candidate?.dispose(); tree?.dispose();
            if (!disposed && version === pinVersion) { pinBusy = false; publish(); }
        }
    }
    async function deletePin() {
        if (!pinSelection || pinBusy || disposed) return;
        const key = context.key, id = pinSelection, version = ++pinVersion;
        pinBusy = true; pinError = undefined; pinMessage = undefined; publish();
        try {
            await repository.execute(request('delete-build-snapshot', { projectId: context.project.id, key: id }));
            if (disposed || context?.key !== key || version !== pinVersion) return;
            closePin(); pinMessage = 'Pinned build deleted. Current build retained.'; await refreshPins();
        } catch (error) { if (!disposed && version === pinVersion) pinError = { message: error.message }; }
        finally { if (!disposed && version === pinVersion) { pinBusy = false; publish(); } }
    }
    const stamp = project => {
        const entry = project.entries.find(item => item.id === configurationId);
        return JSON.stringify(entry && [entry.kind, entry.runConfigResourceId, entry.resourceIds, entry.tags, entry.entryArtifact]);
    };
    const changed = projection => projection.buildRun.inputs.some(input => {
        const resource = context.project.resources.find(resource => resource.id === input.resourceId);
        return !resource || resource.revision !== input.revision || resource.sha256 !== input.sha256;
    });
    const defaultArtifact = artifacts => {
        const index = artifacts.filter(item => item.path === 'index.html' || item.path.endsWith('/index.html'));
        const html = artifacts.filter(item => ['text/html', 'application/xhtml+xml'].includes(item.contentType.split(';')[0]));
        return artifacts.length === 1 ? artifacts[0] : index.length === 1 ? index[0] : html.length === 1 ? html[0] : undefined;
    };
    const choices = () => {
        if (!context) return [];
        const project = context.project;
        const configurations = project.entries.filter(entry => ['transformation-graph', 'transformation', 'conversion', 'query'].includes(entry.kind)
            || entry.tags?.includes('live-preview')).filter(entry => project.resources.some(resource => resource.id === entry.runConfigResourceId && resource.schema === 'https://cem.dev/ns/cli/command/1'))
            .map(entry => ({ id: entry.id, name: entry.name, kind: entry.kind, entryArtifact: entry.entryArtifact }));
        const documents = project.resources.filter(resource => resource.contentType === 'application/cem' && resource.schema === 'https://cem.dev/ns/cem-ml/1')
            .map(resource => ({ id: `html-v1:${resource.id}`, name: resource.path, kind: 'document-html', resourceId: resource.id }));
        return [...configurations.filter(entry => entry.kind === 'transformation-graph'),
            ...configurations.filter(entry => entry.kind === 'transformation'), ...documents,
            ...configurations.filter(entry => !['transformation-graph', 'transformation'].includes(entry.kind))];
    };
    function snapshot() {
        const configurations = choices();
        const available = configurations.some(item => item.id === configurationId);
        const projection = viewed()?.snapshot().projection;
        const path = pinned ? pinnedPath : artifactPath, tree = pinned ? pinnedTree : artifactTree;
        const artifact = projection?.buildRun.artifacts.find(item => item.path === path || item.path === path?.split(/[?#]/)[0]);
        const inputs = projection?.buildRun.inputs ?? [];
        const pinnedConfiguration = projection?.buildRun.configuration;
        const pinConfigurationStale = Boolean(pinned && !configurations.some(item => item.id === pinnedConfiguration.id
            && item.entryArtifact === pinnedConfiguration.entryArtifact
            && (item.kind === 'document-html' || item.kind === pinnedConfiguration.kind)));
        const stale = Boolean(projection && (projection.stale || changed(projection) || (pinned ? pinConfigurationStale : successStamp !== stamp(context.project))
            || !context.bundle && source?.projectId === context.project.id && (source.dirty && inputs.some(input => input.resourceId === source.resourceId)
            || source.entryId === configurationId && source.command?.draftText !== source.command?.current?.text)));
        return Object.freeze({ projectId: context?.project.id, contextKey: context?.key, configurations, configurationId, artifactPath: path, currentArtifactPath: artifactPath,
            pins, pinId, pinSelection, pinBusy, pinError, pinMessage, canPin: Boolean(success && !pinSelection && !active && !pinBusy), currentRunId: success?.snapshot().projection?.buildRun.id,
            entryArtifact: configurations.find(item => item.id === configurationId)?.entryArtifact,
            available, busy: Boolean(active), attempt, projection, stale,
            preview: artifact && tree && !/^[a-z][\w+.-]*:/i.test(artifact.path) && ['text/html', 'application/xhtml+xml'].includes(artifact.contentType.split(';')[0]) && projection.preview?.kind === 'sandboxed-html'
                ? { kind: 'artifact-tree', label: projection.preview.label, resourceSet: tree.resourceSet, artifactPath: path } : artifact ? projection?.preview : undefined,
            missingArtifact: Boolean(projection && path !== undefined && !artifact),
            selectedArtifact: artifact, artifacts: projection?.buildRun.artifacts ?? [] });
    }
    const publish = () => { if (!disposed) for (const notify of subscribers) notify(snapshot()); };
    function cancel() {
        if (!active) return;
        const job = active; active = undefined; ++generation;
        job.controller.abort();
        attempt = { id: job.id, status: 'cancelled' }; publish();
    }
    function clear() { closePin(); pinError = undefined; pinMessage = undefined; cancel(); success?.dispose(); artifactTree?.dispose(); artifactTree = undefined; success = undefined; attempt = undefined; artifactPath = undefined; explicitArtifact = false; }
    function setContext(next, settings = {}) {
        const changedContext = context?.key !== next?.key;
        if (changedContext) { pinListController?.abort(); clear(); pins = []; context = next; configurationId = undefined; }
        else context = next;
        if (!context) { configurationId = undefined; publish(); return; }
        const configurations = choices();
        const selected = settings.build ?? (settings.configuration && configurations.some(item => item.id === settings.configuration) ? settings.configuration : undefined);
        const nextId = selected ?? (settings.reset ? configurations[0]?.id : configurationId ?? configurations[0]?.id);
        if (nextId !== configurationId) { clear(); configurationId = nextId; }
        if (settings.artifact !== undefined || settings.reset) { artifactPath = settings.artifact; explicitArtifact = settings.artifact !== undefined; }
        const artifacts = success?.snapshot().projection?.buildRun.artifacts;
        if (!explicitArtifact) artifactPath = configurations.find(item => item.id === configurationId)?.entryArtifact ?? (artifacts && defaultArtifact(artifacts)?.path);
        const artifact = artifacts?.find(item => item.path === artifactPath || item.path === selectedPath());
        if (artifact) success.selectArtifact(artifact.id);
        publish();
        if (changedContext) void refreshPins();
    }
    async function open(bundle, configuration, signal) {
        // A read-only captured repository makes every native read use the same
        // saved revision, including sample builds. It never imports a sample.
        const captured = { query: async ({ operation }) => {
            if (operation === 'list-projects') return { value: [bundle.project] };
            if (operation === 'export-project') return { value: bundle, repositoryRevision: bundle.project.revision };
            throw new Error(`Build snapshot does not support ${operation}`);
        }, execute: async () => { throw new Error('Build inputs are read-only'); } };
        if (openBuildWorkbench) return openBuildWorkbench({ bundle, configuration, repository: captured, signal });
        if (!validator) throw new Error('The native project builder is unavailable');
        if (!configuration.resourceId) return createCemStudioProjectWorkbench({ repository: captured, validator,
            selection: { projectId: bundle.project.id, entryId: configuration.id }, signal });
        const resource = bundle.project.resources.find(item => item.id === configuration.resourceId);
        signal.throwIfAborted();
        return createCemStudioWorkbench({ repository: captured, validator, projectId: bundle.project.id, example: {
            id: configuration.id, name: resource.path, kind: 'conversion', operation: 'convert', resourceId: resource.id,
            runConfigResourceId: configuration.id, path: resource.path, referenceUri: projectResourceUri(bundle.project, resource),
            projectRootUri: bundle.project.rootUri, contentType: resource.contentType, schema: resource.schema,
            dependencies: bundle.project.resources.filter(item => item.id !== resource.id && item.role !== 'run-config').map(item => ({ resourceId: item.id, path: projectResourceUri(bundle.project, item) })),
            commandArguments: ['convert', '$input', '--content-type', resource.contentType, '--schema', resource.schema,
                '--to-content-type', 'text/html', '--to-schema', 'https://cem.dev/ns/data/html/1', '--output-color-type', 'none'],
        } });
    }
    async function build() {
        if (disposed || !context || !choices().some(item => item.id === configurationId) || active) return;
        closePin();
        const capturedContext = context, configuration = choices().find(item => item.id === configurationId);
        const job = { id: ++generation, controller: new AbortController() };
        active = job; attempt = { id: job.id, status: 'building' }; publish();
        const current = () => !disposed && active === job && generation === job.id;
        const aborted = new Promise(resolve => job.controller.signal.addEventListener('abort', resolve, { once: true }));
        const execution = (async () => {
            let candidate;
            try {
                const bundle = capturedContext.bundle ?? (await repository.query(request('export-project', { projectId: capturedContext.project.id }))).value;
                job.controller.signal.throwIfAborted();
                candidate = await open(bundle, configuration, job.controller.signal);
                job.controller.signal.throwIfAborted();
                await candidate.runPersistedOperation({ signal: job.controller.signal });
                if (!current()) return;
                if (!capturedContext.bundle) {
                    const latest = (await repository.query(request('get-project', { projectId: capturedContext.project.id }))).value;
                    if (!current()) return;
                    if (!latest || latest.trashedAt) throw new Error('The build project is missing or in Trash');
                    context = { ...context, project: latest };
                }
                const projection = candidate.snapshot().projection;
                const stale = projection && (projection.stale || changed(projection) || stamp(bundle.project) !== stamp(context.project));
                if (!projection || projection.exitCode !== 0 || stale) {
                    attempt = { id: job.id, status: stale ? 'stale' : 'failed', diagnostics: projection?.diagnostics ?? [] }; return;
                }
                const artifacts = projection.buildRun.artifacts;
                const declaredEntry = bundle.project.entries.find(entry => entry.id === configuration.id)?.entryArtifact;
                const nextPath = explicitArtifact ? artifactPath : declaredEntry ?? defaultArtifact(artifacts)?.path;
                const selected = artifacts.find(item => item.path === nextPath || item.path === nextPath?.split(/[?#]/)[0]);
                if (selected) candidate.selectArtifact(selected.id);
                const nextTree = treeFor(projection);
                artifactPath = nextPath;
                artifactTree?.dispose(); artifactTree = nextTree;
                success?.dispose(); success = candidate; successStamp = stamp(bundle.project); candidate = undefined;
                attempt = { id: job.id, status: 'succeeded' };
            } catch (error) {
                if (current()) attempt = { id: job.id, status: 'failed', error: { message: error.message } };
            } finally {
                candidate?.dispose();
                if (current()) { active = undefined; publish(); }
            }
        })();
        await Promise.race([execution, aborted]);
    }
    return Object.freeze({ snapshot, setContext, build, cancel, pin, selectPin, deletePin, refreshPins,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        updateSource(value) { source = value; publish(); },
        selectConfiguration(id) { setContext(context, { build: id, reset: true }); },
        selectArtifact(path) {
            if (pinned) pinnedPath = path; else { artifactPath = path; explicitArtifact = true; }
            const artifact = viewed()?.snapshot().projection?.buildRun.artifacts.find(item => item.path === path || item.path === path?.split(/[?#]/)[0]);
            if (artifact) viewed().selectArtifact(artifact.id);
            publish();
        },
        async copy(writeText) { if (snapshot().selectedArtifact) { await viewed().copyProjection(writeText); publish(); } },
        async download(save) { if (snapshot().selectedArtifact) { await viewed().downloadProjection(save); publish(); } },
        dispose() { disposed = true; pinListController?.abort(); clear(); subscribers.clear(); },
    });
}
