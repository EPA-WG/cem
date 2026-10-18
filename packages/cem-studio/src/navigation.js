import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';

const request = (operation, parameters) => ({ protocolVersion: 1, repository: CEM_STUDIO_REPOSITORY_ID, requestRevision: 1, operation, parameters });
const routeKey = route => new URLSearchParams(route).toString() || 'samples';
const node = (route, name, kind, description = '', children = []) => ({ key: routeKey(route), route, name, kind, description, children,
    icon: kind });

/** Navigation is domain data. Views render this model; browser history is only a transport. */
export async function createCemStudioNavigation({ workbench, repository, examples = [], collections = [], browser }) {
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
    function snapshot() { return { ...base, navigation: model }; }
    async function refresh() {
        const currentVersion = ++refreshVersion;
        const projectId = selection.project, previousOpened = opened;
        const response = await repository.query(request('list-projects', {}));
        if (disposed || currentVersion !== refreshVersion) return;
        projects = response.value;
        if (projectId && previousOpened) {
            const record = (await repository.query(request('get-project', { projectId }))).value;
            if (disposed || currentVersion !== refreshVersion) return;
            if (record && opened === previousOpened && selection.project === projectId) opened = { ...opened, project: record };
        }
        publish();
    }
    function writeUrl(route, mode) {
        if (!browser) return;
        const url = new URL(browser.location.href);
        for (const name of ['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration']) url.searchParams.delete(name);
        for (const [name, value] of Object.entries(route)) url.searchParams.set(name, value);
        if (mode === 'push' && url.href !== acceptedUrl.href) browser.history.pushState(null, '', url);
        else if (mode !== 'push' || url.href !== browser.location.href) browser.history.replaceState(null, '', url);
        acceptedUrl = url;
    }
    async function navigate(route, mode = 'push') {
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
                nextOpened = { project: bundle.project, route: { sample: example.id }, text: '' };
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
                pendingRoute = { route, mode, opened: nextOpened, target: JSON.stringify(target) };
                const current = workbench.snapshot();
                if (current.workbenchId !== JSON.stringify(target) && JSON.stringify(current.transition?.target) !== JSON.stringify(target)) await workbench.selectWorkbench(JSON.stringify(target));
                if (currentVersion !== version) return;
                if (workbench.snapshot().transition) return;
                if (workbench.snapshot().workbenchId !== JSON.stringify(target)) throw new Error('Finish the pending operation before changing files.');
                pendingRoute = undefined;
            }
            selection = route; opened = nextOpened; writeUrl(route, mode);
        } catch (cause) { if (currentVersion !== version) return; pendingRoute = undefined; error = { message: cause.message }; if (mode === 'pop' && browser) browser.history.replaceState(null, '', acceptedUrl); }
        finally { if (currentVersion === version) { busy = false; publish(); } }
    }
    const unsubscribe = workbench.subscribe(state => { base = state; publish(); });
    const unsubscribeRepository = repository.subscribe?.(0, () => { void refresh().catch(cause => { error = { message: cause.message }; publish(); }); });
    const readUrl = () => {
        const params = new URL(browser.location.href).searchParams;
        return Object.fromEntries(['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration'].filter(key => params.has(key)).map(key => [key, params.get(key)]));
    };
    const onPop = () => { void navigate(readUrl(), 'pop'); };
    await refresh();
    if (browser) { await navigate(readUrl(), 'replace'); browser.addEventListener('popstate', onPop); }
    const api = {
        ...workbench, snapshot,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        async openEntity(key) { await navigate(key === 'samples' ? {} : Object.fromEntries(new URLSearchParams(key))); },
        async selectWorkbench(id) {
            const target = JSON.parse(id);
            await navigate({ project: target.projectId, ...(target.resourceId ? { file: target.resourceId } : {}), ...(target.entryId ? { configuration: target.entryId } : {}) });
        },
        async navigateProvenance(index) {
            const result = await workbench.navigateProvenance(index);
            const current = workbench.snapshot();
            const target = current.transition?.target ?? JSON.parse(current.workbenchId);
            await navigate({ project: target.projectId, ...(target.resourceId ? { file: target.resourceId } : {}), ...(target.entryId ? { configuration: target.entryId } : {}) });
            return result;
        },
        async resolveNavigation(decision) {
            await workbench.resolveNavigation(decision);
            if (decision === 'cancel') { pendingRoute = undefined; if (browser) browser.history.replaceState(null, '', acceptedUrl); }
            else if (pendingRoute && !workbench.snapshot().transition && workbench.snapshot().workbenchId === pendingRoute.target) {
                const pending = pendingRoute; pendingRoute = undefined;
                selection = pending.route; opened = pending.opened; writeUrl(selection, pending.mode);
            }
            publish();
        },
        requestAction(kind) { if (kind === 'clone' && !model.canClone || kind === 'delete' && !model.canDelete) return; action = { kind, target: selected.name, createKind: kind === 'clone' && selected.kind === 'project' ? 'project' : model.createKind }; publish(); },
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
        dispose() { disposed = true; ++version; ++refreshVersion; unsubscribe(); unsubscribeRepository?.(); browser?.removeEventListener('popstate', onPop); subscribers.clear(); workbench.dispose(); },
    };
    return Object.freeze(api);
}
