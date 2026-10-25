import { captureCemStudioBuildSnapshot, openCemStudioBuildSnapshot, compareCemStudioBuildRuns } from './build-snapshots.js';
import { CEM_STUDIO_REPOSITORY_ID } from './repository.js';
import { createCemStudioProjectWorkbench, createCemStudioWorkbench } from './workbench.js';
import { projectResourceUri } from './resource-reference.js';
import { CEM_STUDIO_LIMITS, createCemStudioPreview } from './preview.js';
import { registerCemArtifactTree } from '@epa-wg/cem-elements';

const request = (operation, parameters) => ({ protocolVersion: 1, repository: CEM_STUDIO_REPOSITORY_ID, requestRevision: 1, operation, parameters });
const routeFields = ['view', 'collection', 'project', 'sample', 'folder', 'file', 'configuration', 'selection', 'output-folder', 'build', 'artifact'];
const routeKey = route => new URLSearchParams(routeFields.filter(key => route[key] !== undefined).map(key => [key, route[key]])).toString() || 'samples';
const selectedKey = route => routeKey(Object.fromEntries(Object.entries(route).filter(([key]) => !['build', 'artifact'].includes(key) || route.selection === 'output')));
const treeAncestors = (root, key, chain = []) => { if (root.key === key) return chain; for (const child of root.children) { const found = treeAncestors(child, key, [...chain, root.key]); if (found) return found; } };
const normalizedOutputFolder = value => {
    if (value === '') return '';
    if (typeof value !== 'string' || value.startsWith('/') || value.includes('\\') || /[?#]/u.test(value) || [...value].some(character => character.codePointAt(0) < 32)) throw new Error('The output directory must be a relative directory.');
    const parts = value.replace(/\/$/u, '').split('/');
    if (parts.some(part => !part || part === '.' || part === '..' || part.includes(':'))) throw new Error('The output directory must be normalized.');
    return `${parts.join('/')}/`;
};
const node = (route, name, kind, description = '', children = []) => ({ key: routeKey(route), route, name, kind, description, children,
    icon: kind });

/** Navigation is domain data. Views render this model; browser history is only a transport. */
export async function createCemStudioNavigation({ workbench, repository, examples = [], collections = [], browser, validator, openBuildWorkbench }) {
    const subscribers = new Set();
    const bundles = new Map();
    let projects = [], selected = node({}, 'Examples', 'folder', 'Predefined example projects. Clone a sample to save your own version.');
    let opened, selection = {}, error, action, pendingRoute, navigationController, busy = false, version = 0, refreshVersion = 0, disposed = false;
    let base = workbench.snapshot();
    let acceptedUrl = browser ? new URL(browser.location.href) : new URL('https://studio.invalid/');
    for (const key of routeFields) acceptedUrl.searchParams.delete(key);
    let model;
    const sections = { projects: true, structure: false, output: false };
    const sectionTouched = new Set();
    const trees = { projects: { selected: 'samples', expanded: new Set(['samples', 'view=projects']) }, structure: { selected: '', expanded: new Set() }, output: { selected: '', expanded: new Set() } };
    let retainedProjectKey, revealPending = true, outputRevealed = false;
    const memberships = example => [...new Set(example.collections?.length ? example.collections : ['examples'])];
    const catalogRoute = route => route.sample && !route.collection
        ? { collection: memberships(examples.find(example => example.id === route.sample))[0], ...route } : route;
    async function loadSample(example) {
        let pending = bundles.get(example.id);
        if (!pending) { pending = Promise.resolve().then(() => example.load()); bundles.set(example.id, pending); }
        try { return await pending; }
        catch (cause) { if (bundles.get(example.id) === pending) bundles.delete(example.id); throw cause; }
    }
    const sampleRoot = () => {
        const roots = [], groups = new Map(collections.map(collection => [collection.id,
            node({ collection: collection.id }, collection.name, 'folder', collection.description ?? 'Example projects')]));
        for (const collection of collections) {
            const parent = collection.parentId && groups.get(collection.parentId);
            (parent ? parent.children : roots).push(groups.get(collection.id));
        }
        for (const example of examples) {
            // Occurrences are catalog routes to the same ordinary project.
            // Including their direct membership keeps every tree value unique.
            for (const id of memberships(example)) {
                let group = groups.get(id);
                if (!group) {
                    group = node({ collection: id }, id === 'examples' ? 'Examples' : id, 'folder', 'Example projects');
                    groups.set(id, group);
                    roots.push(group);
                }
                group.children.push(node({ collection: id, sample: example.id }, example.name, 'project', example.description ?? 'Open this sample or clone it into Saved projects.'));
            }
        }
        return node({}, 'Examples', 'folder', 'Predefined example projects. Clone a sample to save your own version.', roots);
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
    function outputTree() {
        const output = preview?.snapshot();
        if (!opened || !output?.projection) return undefined;
        const viewedConfiguration = output.projection.buildRun.configuration?.id ?? output.configurationId;
        const route = { ...opened.route, selection: 'output', ...(viewedConfiguration ? { build: viewedConfiguration } : {}) };
        const root = node({ ...route, 'output-folder': '' }, 'Output files', 'output-folder', 'Generated files from the admitted successful build.');
        for (const artifact of output.artifacts) {
            // The native manifest is authoritative. Absolute publication URIs are
            // individual leaves rather than guessed local directory structures.
            const parts = /^[a-z][\w+.-]*:/iu.test(artifact.path) ? [artifact.path] : artifact.path.split('/');
            let parent = root, path = '';
            for (const part of parts.slice(0, -1)) {
                path += `${part}/`;
                let child = parent.children.find(item => item.path === path && item.kind === 'output-folder');
                if (!child) { child = { ...node({ ...route, 'output-folder': path }, part, 'output-folder', `Generated files in ${path}`), path }; parent.children.push(child); }
                parent = child;
            }
            parent.children.push({ ...node({ ...route, artifact: artifact.path }, parts.at(-1), 'output-file', artifact.contentType), path: artifact.path, contentType: artifact.contentType, artifactId: artifact.id, byteLength: artifact.byteLength, sha256: artifact.sha256 });
        }
        const sort = item => { item.children.sort((a, b) => (a.kind === 'output-folder' ? 0 : 1) - (b.kind === 'output-folder' ? 0 : 1) || a.name.localeCompare(b.name)); item.children.forEach(sort); };
        sort(root); return root;
    }
    const sourceDescriptor = (nextOpened = opened, route = selection, state = base) => {
        const resource = route.file && nextOpened?.project.resources.find(item => item.id === route.file);
        // Read-only/unavailable records have no admitted UTF-8 text. Empty
        // editor placeholders must not become fabricated data or HTML input.
        if (!route.sample && (state.editable === false || state.resourceAvailable === false
            || state.projectId !== nextOpened?.project.id || state.resourceId !== route.file)) return undefined;
        return resource && { resourceId: resource.id, projectId: nextOpened.project.id, projectRootUri: nextOpened.project.rootUri, projectRevision: nextOpened.project.revision, sourceUri: projectResourceUri(nextOpened.project, resource), contentType: resource.contentType, schema: resource.schema,
            text: route.sample ? nextOpened.text : state.draft ?? state.persistedText ?? '', revision: resource.revision, sha256: resource.sha256, draft: Boolean(!route.sample && state.dirty) };
    };
    const prepareSelection = async (nextOpened, route, signal) => {
        if (!validator?.prepareResource) return;
        const descriptor = sourceDescriptor(nextOpened, route, workbench.snapshot());
        if (!descriptor) return;
        await validator.prepareResource({ bytes: new TextEncoder().encode(descriptor.text), uri: descriptor.sourceUri,
            contentType: descriptor.contentType, schema: descriptor.schema, resourceScheme: 'studio',
            projectId: descriptor.projectId, projectRootUri: descriptor.projectRootUri,
            projectRevision: descriptor.projectRevision, resourceRevision: descriptor.revision, signal });
    };
    function publish() {
        if (disposed) return;
        const samples = sampleRoot();
        const saved = node({ view: 'projects' }, 'Saved projects', 'folder', 'Projects saved in this browser.', projects.map(project => node({ project: project.id }, project.name, 'project', project.description ?? 'Saved in this browser')));
        const files = opened ? fileTree(opened.project, opened.route) : undefined;
        const output = outputTree();
        const key = selectedKey(selection);
        selected = find(samples, key) ?? find(saved, key) ?? (files && find(files, key)) ?? (output && find(output, key)) ?? selected;
        if (files && key === files.key) selected = files;
        // A direct output route may precede its first build. Its identity remains
        // active while the native output is unavailable, never a source fallback.
        if (selection.selection === 'output' && (!output || !find(output, key))) {
            const folder = Object.hasOwn(selection, 'output-folder');
            selected = { ...node(selection, folder ? selection['output-folder'] || 'Output files' : selection.artifact, folder ? 'output-folder' : 'output-file', 'Requested generated output is not available in this successful run.'), path: folder ? selection['output-folder'] : selection.artifact };
        }
        const projectKey = opened && routeKey(opened.route);
        if (projectKey !== retainedProjectKey) {
            retainedProjectKey = projectKey; outputRevealed = false;
            trees.structure.selected = ''; trees.structure.expanded.clear();
            if (files) trees.structure.expanded.add(files.key);
            trees.output.selected = ''; trees.output.expanded.clear();
            if (opened && !sectionTouched.has('structure')) sections.structure = true;
        }
        if (output && !outputRevealed) {
            outputRevealed = true;
            if (!sectionTouched.has('output')) sections.output = true;
            trees.output.expanded.add(output.key);
        }
        if (revealPending) {
            if (opened) {
                trees.projects.selected = routeKey(catalogRoute(opened.route));
                for (const root of [samples, saved]) for (const ancestor of treeAncestors(root, trees.projects.selected) ?? []) trees.projects.expanded.add(ancestor);
            }
            const group = selection.selection === 'output' ? 'output' : selection.file || selection.folder || selection.configuration ? 'structure' : 'projects';
            const treeKey = group === 'projects' && opened?.route.sample ? routeKey(catalogRoute(opened.route)) : selected.key;
            trees[group].selected = treeKey;
            for (const root of group === 'projects' ? [samples, saved] : group === 'structure' ? [files] : [output]) if (root) {
                for (const ancestor of treeAncestors(root, treeKey) ?? []) trees[group].expanded.add(ancestor);
            }
            if (selected.children.length) trees[group].expanded.add(treeKey);
            // Project activation is remembered in both its catalog and structure.
            if (selected.kind === 'project' && files) trees.structure.selected = files.key;
            revealPending = false;
        }
        const children = selected.children;
        const leaf = ['file', 'configuration', 'output-file'].includes(selected.kind);
        const count = leaf ? 1 : children.length;
        const emptyCountKind = selected.kind === 'output-folder' || selected.kind === 'project' || selection.folder ? 'file' : selection.view === 'projects' || selection.collection ? 'project' : selected.key === 'samples' ? 'folder' : 'item';
        const countKind = leaf ? selected.kind === 'output-file' ? 'file' : selected.kind : !children.length ? emptyCountKind : children.every(item => item.kind === 'project') ? 'project' : children.every(item => ['file', 'output-file'].includes(item.kind)) ? 'file' : children.every(item => ['folder', 'output-folder'].includes(item.kind)) ? 'folder' : 'item';
        const path = selected.path ?? (selection.folder || (selection.configuration ? opened?.resource?.path || base.path || selected.name : ''));
        const title = opened && (selection.project || selection.sample) ? `${opened.project.name}${path ? ` · ${path}` : ''} - CEM Studio` : `${selected.name} - CEM Studio`;
        const projectedTrees = Object.fromEntries(Object.entries(trees).map(([name, tree]) => [name, { selected: tree.selected, expanded: [...tree.expanded].join(' ') }]));
        const descriptor = sourceDescriptor();
        const sourcePreview = descriptor && { ...filePreview?.snapshot(), ...descriptor, mode: descriptor.draft ? 'draft' : 'saved' };
        model = { samples, saved, files, output, sections: { ...sections }, trees: projectedTrees, selection: { ...selection, key: selected.key, kind: selected.kind }, selected, children, resources: opened?.project.resources ?? [], count, countKind, title, path, projectName: opened?.project.name, pathKind: selected.kind, sourcePreview,
            expanded: [...new Set(Object.values(trees).flatMap(tree => [...tree.expanded]))].join(' '), busy, error, action,
            canCreate: !selection.selection,
            canClone: !selection.selection && (selected.kind === 'project' || (selected.kind === 'file' && Boolean(selection.project))),
            canDelete: !selection.selection && Boolean(selection.project) && ['project', 'file'].includes(selected.kind),
            createKind: selection.project ? 'file' : 'project',
            content: selected.kind === 'project' ? 'project' : selected.kind === 'output-file' ? 'output-file' : selected.kind === 'output-folder' ? 'output-folder' : selected.kind === 'file' || selected.kind === 'configuration' ? (selection.sample ? 'sample-file' : 'file') : 'folder',
            sampleText: opened?.text ?? '', contentType: opened?.resource?.contentType ?? 'text/plain',
            sourceId: JSON.stringify(['sample', selection.sample, opened?.resource?.id ?? selected.key]),
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
        for (const name of routeFields) url.searchParams.delete(name);
        const output = preview?.snapshot();
        const combined = { ...route, ...(output?.configurationId ? { build: output.configurationId } : {}), ...(!Object.hasOwn(route, 'output-folder') && output?.currentArtifactPath ? { artifact: output.currentArtifactPath } : {}) };
        for (const [name, value] of Object.entries(combined)) url.searchParams.set(name, value);
        if (mode === 'push' && url.href !== acceptedUrl.href) browser.history.pushState(null, '', url);
        else if (mode !== 'push' || url.href !== browser.location.href) browser.history.replaceState(null, '', url);
        acceptedUrl = url;
    }
    function admitPreview(nextOpened, settings) {
        preview.setContext(nextOpened && { ...nextOpened, key: routeKey(nextOpened.route) }, settings);
        if (selection.selection === 'output' && !selection.build && preview.snapshot().configurationId) selection.build = preview.snapshot().configurationId;
        preview.setSelection(selection);
        filePreview.setSource(sourceDescriptor());
    }
    async function navigate(requestedRoute, mode = 'push', preservePreview = false) {
        const { build, artifact, ...rest } = requestedRoute;
        const route = { ...rest, ...(rest.selection === 'output' && artifact !== undefined ? { artifact } : {}), ...(rest.selection === 'output' && build !== undefined ? { build } : {}) };
        const settings = { build, artifact, configuration: route.configuration, reset: !preservePreview, sourceActivation: preservePreview && Boolean(route.file), sourceResourceId: route.file };
        const currentVersion = ++version;
        navigationController?.abort();
        const controller = new AbortController(); navigationController = controller;
        if (pendingRoute) { pendingRoute = undefined; await workbench.resolveNavigation('cancel'); }
        if (disposed || currentVersion !== version) return;
        busy = true; error = undefined; action = undefined; publish();
        try {
            if (['view', 'project', 'sample'].filter(key => route[key]).length > 1 || route.collection !== undefined && (route.view !== undefined || route.project !== undefined)) throw new Error('The Studio URL has conflicting roots.');
            if ((route.project && route.sample) || (route.file && (route.folder || route.configuration)) || (route.folder && route.configuration)) throw new Error('The Studio URL has conflicting selections.');
            if (route.selection !== undefined && route.selection !== 'output') throw new Error('The navigation selection is unavailable.');
            if (route.selection === 'output' && (route.file || route.folder || route.configuration || Object.hasOwn(route, 'output-folder') && artifact !== undefined)) throw new Error('The Studio URL has conflicting output selections.');
            if (route.selection === 'output' && !Object.hasOwn(route, 'output-folder') && !artifact) throw new Error('Choose an output file or directory.');
            if (Object.hasOwn(route, 'output-folder')) {
                if (route.selection !== 'output') throw new Error('Choose output navigation before an output directory.');
                route['output-folder'] = normalizedOutputFolder(route['output-folder']);
            }
            if ((route.file || route.folder || route.configuration || route.selection) && !route.project && !route.sample) throw new Error('Choose a project before a file or folder.');
            let nextOpened;
            if (route.sample) {
                const example = examples.find(item => item.id === route.sample);
                if (!example) throw new Error('The requested sample is unavailable.');
                if (route.collection !== undefined && !memberships(example).includes(route.collection)) throw new Error('The requested sample is not in this samples folder.');
                const bundle = await loadSample(example);
                nextOpened = { project: bundle.project, bundle, route: { ...(route.collection !== undefined ? { collection: route.collection } : {}), sample: example.id }, text: '' };
                if (route.file) {
                    const resource = bundle.project.resources.find(item => item.id === route.file);
                    if (!resource) throw new Error('The requested sample file is unavailable.');
                    const bytes = bundle.contents[resource.id];
                    nextOpened.resource = resource;
                    nextOpened.text = new TextDecoder('utf-8', { fatal: true }).decode(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes ?? []));
                }
                if (route.configuration) {
                    const entry = bundle.project.entries.find(item => item.id === route.configuration);
                    if (!entry) throw new Error('The requested configuration is unavailable.');
                    const resource = bundle.project.resources.find(item => item.id === entry.runConfigResourceId);
                    const bytes = bundle.contents[resource?.id];
                    nextOpened.resource = resource;
                    nextOpened.text = new TextDecoder().decode(bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes ?? []));
                }
            } else if (route.project) {
                const record = (await repository.query(request('get-project', { projectId: route.project }))).value;
                if (!record || !projects.some(item => item.id === record.id)) throw new Error('The requested project is missing or in Trash.');
                nextOpened = { project: record, route: { project: record.id } };
            } else if (route.collection !== undefined) {
                if (!find(sampleRoot(), routeKey(route))) throw new Error('The requested samples folder is unavailable.');
            } else if (route.view && route.view !== 'projects') throw new Error('The requested Studio view is unavailable.');
            if (currentVersion !== version) return;
            if (nextOpened && route.selection !== 'output' && !find(fileTree(nextOpened.project, nextOpened.route), routeKey(route))) throw new Error('The requested file or directory is unavailable.');
            if (route.project && route.selection !== 'output') {
                const target = { projectId: route.project, ...(route.file ? { resourceId: route.file } : {}), ...(route.configuration ? { entryId: route.configuration } : {}) };
                pendingRoute = { route, mode, opened: nextOpened, settings, target: JSON.stringify(target), version: currentVersion, controller };
                const current = workbench.snapshot();
                if (current.workbenchId !== JSON.stringify(target) && JSON.stringify(current.transition?.target) !== JSON.stringify(target)) await workbench.selectWorkbench(JSON.stringify(target));
                if (currentVersion !== version) return;
                if (workbench.snapshot().transition) return;
                if (workbench.snapshot().workbenchId !== JSON.stringify(target)) throw new Error('Finish the pending operation before changing files.');
                pendingRoute = undefined;
            }
            await prepareSelection(nextOpened, route, controller.signal);
            if (disposed || currentVersion !== version || controller.signal.aborted) return;
            selection = route; opened = nextOpened; revealPending = true; admitPreview(opened, settings); writeUrl(route, mode);
        } catch (cause) { if (currentVersion !== version) return; pendingRoute = undefined; error = { message: cause.message }; if (mode === 'pop' && browser) browser.history.replaceState(null, '', acceptedUrl); }
        finally { if (currentVersion === version) { busy = false; if (pendingRoute?.version !== currentVersion) navigationController = undefined; publish(); } }
    }
    const preview = createCemStudioProjectPreview({ repository, validator, openBuildWorkbench });
    const filePreview = createCemStudioSourcePreview({ validator });
    const unsubscribePreview = preview.subscribe(publish);
    const unsubscribeFilePreview = filePreview.subscribe(publish);
    const unsubscribe = workbench.subscribe(state => { base = state; preview.updateSource(state); filePreview.setSource(sourceDescriptor()); publish(); });
    const unsubscribeRepository = repository.subscribe?.(0, () => { void preview.refreshPins(); void refresh().catch(cause => { error = { message: cause.message }; publish(); }); });
    const readUrl = () => {
        const params = new URL(browser.location.href).searchParams;
        return Object.fromEntries(routeFields.filter(key => params.has(key)).map(key => [key, params.get(key)]));
    };
    const onPop = () => { void navigate(readUrl(), 'pop'); };
    await refresh();
    if (browser) { await navigate(readUrl(), 'replace'); browser.addEventListener('popstate', onPop); }
    const api = {
        ...workbench, snapshot,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        async openEntity(key) { await navigate(key === 'samples' ? {} : Object.fromEntries(new URLSearchParams(key)), 'push', true); },
        toggleNavigationSection(name) { if (Object.hasOwn(sections, name)) { sectionTouched.add(name); sections[name] = !sections[name]; publish(); } },
        toggleNavigationExpanded(name, key) { if (!Object.hasOwn(trees, name) || typeof key !== 'string') return; const expanded = trees[name].expanded; if (expanded.has(key)) expanded.delete(key); else expanded.add(key); publish(); },
        setNavigationSection(name, open) { if (!Object.hasOwn(sections, name)) return; sectionTouched.add(name); sections[name] = Boolean(open); publish(); },
        setNavigationExpanded(name, keys) { if (!Object.hasOwn(trees, name)) return; trees[name].expanded = new Set(Array.isArray(keys) ? keys : String(keys ?? '').split(/\s+/u).filter(Boolean)); publish(); },
        selectPreviewConfiguration(id) { preview.selectConfiguration(id); writeUrl(selection, 'push'); },
        selectPreviewArtifact(path) {
            if (selection.selection === 'output') {
                const route = { ...selection, artifact: path };
                delete route['output-folder'];
                return navigate(route, 'push', true);
            }
            preview.selectArtifact(path); if (!preview.snapshot().pinId) writeUrl(selection, 'push');
        },
        pinProjectBuild() { return preview.pin(); },
        selectPinnedBuild(key) { return preview.selectPin(key); },
        deletePinnedBuild() { return preview.deletePin(); },
        selectComparisonArtifact(path) { preview.selectComparisonArtifact(path); },
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
            const currentVersion = version;
            await workbench.resolveNavigation(decision);
            if (disposed || currentVersion !== version) return;
            if (decision === 'cancel') { navigationController?.abort(); navigationController = undefined; pendingRoute = undefined; if (browser) browser.history.replaceState(null, '', acceptedUrl); }
            else if (pendingRoute && !workbench.snapshot().transition && workbench.snapshot().workbenchId === pendingRoute.target) {
                const pending = pendingRoute;
                busy = true; publish();
                try {
                    await prepareSelection(pending.opened, pending.route, pending.controller.signal);
                    if (disposed || currentVersion !== version || pendingRoute !== pending || pending.controller.signal.aborted) return;
                    pendingRoute = undefined; navigationController = undefined;
                    selection = pending.route; opened = pending.opened; revealPending = true; admitPreview(opened, pending.settings); writeUrl(selection, pending.mode);
                } catch (cause) {
                    if (currentVersion !== version) return;
                    pendingRoute = undefined; navigationController = undefined; error = { message: cause.message };
                } finally { if (currentVersion === version) busy = false; }
            }
            publish();
        },
        requestAction(kind) { if (kind === 'create' && !model.canCreate || kind === 'clone' && !model.canClone || kind === 'delete' && !model.canDelete) return; action = { kind, targetName: selected.name, createKind: kind === 'clone' && selected.kind === 'project' ? 'project' : model.createKind }; publish(); },
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
                    let bundle = selection.sample ? await loadSample(examples.find(item => item.id === selection.sample)) : (await repository.query(request('export-project', { projectId: selection.project }))).value;
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
        dispose() { disposed = true; ++version; ++refreshVersion; navigationController?.abort(); navigationController = undefined; unsubscribe(); unsubscribePreview(); unsubscribeFilePreview(); filePreview.dispose(); preview.dispose(); unsubscribeRepository?.(); browser?.removeEventListener('popstate', onPop); subscribers.clear(); workbench.dispose(); },
    };
    return Object.freeze(api);
}

/** Project build orchestration. Owns one bounded successful run, never an editor. */
export function createCemStudioProjectPreview({ repository, validator, openBuildWorkbench }) {
    const subscribers = new Set();
    let context, configurationId, artifactPath, explicitArtifact = false, success, successStamp, artifactTree, attempt, active, source, navigationSelection = {}, disposed = false, generation = 0;
    let pins = [], pinId, pinSelection, pinned, pinnedTree, pinnedPath, pinBusy = false, pinError, pinMessage, pinVersion = 0, pinListVersion = 0, pinListController, pinReadController;
    let comparisonCache, comparisonArtifactPath;
    const viewed = () => pinned ?? success;
    const comparison = () => {
        const baseline = pinned?.snapshot().projection.buildRun, current = success?.snapshot().projection.buildRun;
        if (!baseline || !current || pinSelection !== pinId || pinBusy || pinError) { comparisonCache = undefined; return undefined; }
        if (comparisonCache?.baseline !== baseline || comparisonCache?.current !== current)
            comparisonCache = { baseline, current, value: compareCemStudioBuildRuns(baseline, current) };
        return comparisonCache.value;
    };
    const selectedPath = () => artifactPath?.split(/[?#]/)[0];
    const treeFor = projection => {
        const artifacts = projection.buildRun.artifacts.filter(item => !/^[a-z][\w+.-]*:/i.test(item.path)).map(item => ({
            path: item.path, contentType: item.contentType, bytes: projection.outputs.find(output => output.uri === item.uri).bytes }));
        return artifacts.length ? registerCemArtifactTree(artifacts) : undefined;
    };
    function closePin() {
        comparisonCache = undefined; comparisonArtifactPath = undefined;
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
            const related = snapshot().selectedArtifact; if (related) pinned.selectArtifact(related.id);
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
    const sourceMatches = (artifact, resourceId, projection) => {
        const resource = context?.project.resources.find(item => item.id === resourceId);
        if (!resource) return false;
        const uri = projectResourceUri(context.project, resource);
        if (artifact.provenance?.some(frame => frame.uri === uri)) return true;
        const choice = choices().find(item => item.id === configurationId);
        if (choice?.kind === 'document-html' && choice.resourceId === resourceId) return true;
        // A native single-input/single-output command has an unambiguous
        // relationship even when a format does not emit byte source maps.
        return projection?.buildRun.inputs.length === 1 && projection.buildRun.inputs[0].resourceId === resourceId && projection.buildRun.artifacts.length === 1;
    };
    function snapshot() {
        const configurations = choices();
        const available = configurations.some(item => item.id === configurationId);
        const projection = viewed()?.snapshot().projection;
        const storedPath = pinned ? pinnedPath : artifactPath, tree = pinned ? pinnedTree : artifactTree;
        const previewContext = navigationSelection.selection === 'output' ? Object.hasOwn(navigationSelection, 'output-folder') ? 'none' : 'output' : navigationSelection.file ? 'source' : navigationSelection.folder || !context ? 'none' : 'project';
        const relatedArtifacts = previewContext === 'source' ? (projection?.buildRun.artifacts ?? []).filter(item => sourceMatches(item, navigationSelection.file, projection)) : [];
        const relatedPath = relatedArtifacts.length === 1 ? relatedArtifacts[0].path : undefined;
        const path = previewContext === 'none' ? undefined : previewContext === 'source' && !explicitArtifact ? relatedPath : storedPath;
        const artifact = projection?.buildRun.artifacts.find(item => item.path === path || item.path === path?.split(/[?#]/)[0]);
        const inputs = projection?.buildRun.inputs ?? [];
        const pinnedConfiguration = projection?.buildRun.configuration;
        const pinConfigurationStale = Boolean(pinned && !configurations.some(item => item.id === pinnedConfiguration.id
            && item.entryArtifact === pinnedConfiguration.entryArtifact
            && (item.kind === 'document-html' || item.kind === pinnedConfiguration.kind)));
        const stale = Boolean(projection && (projection.stale || changed(projection) || (pinned ? pinConfigurationStale : successStamp !== stamp(context.project))
            || !context.bundle && source?.projectId === context.project.id && (source.dirty && inputs.some(input => input.resourceId === source.resourceId)
            || source.entryId === configurationId && source.command?.draftText !== source.command?.current?.text)));
        const selectedOutput = artifact && projection?.outputs.find(item => item.uri === artifact.uri);
        const selectedPreview = projection?.preview?.kind === 'live-cem' && projection.output?.uri === artifact?.uri ? projection.preview
            : selectedOutput?.bytes ? createCemStudioPreview({ bytes: selectedOutput.bytes, contentType: artifact.contentType, label: artifact.path }) : projection?.preview;
        return Object.freeze({ previewContext, relatedArtifacts, missingReason: previewContext === 'source' && !artifact ? relatedArtifacts.length > 1 ? 'Multiple generated outputs match this source. Choose a related artifact.' : 'No generated output is available for this source revision.' : previewContext === 'none' ? 'Select a generated output or a source resource to preview.' : undefined, selectedOutput, projectId: context?.project.id, contextKey: context?.key, configurations, configurationId, artifactPath: path, currentArtifactPath: path,
            comparison: comparison(), comparisonArtifactPath, pins, pinId, pinSelection, pinBusy, pinError, pinMessage, canPin: Boolean(success && !pinSelection && !active && !pinBusy), currentRunId: success?.snapshot().projection?.buildRun.id,
            entryArtifact: configurations.find(item => item.id === configurationId)?.entryArtifact,
            available, busy: Boolean(active), attempt, projection, stale,
            preview: artifact && tree && !/^[a-z][\w+.-]*:/i.test(artifact.path) && ['text/html', 'application/xhtml+xml'].includes(artifact.contentType.split(';')[0]) && selectedPreview?.kind === 'sandboxed-html'
                ? { kind: 'artifact-tree', label: selectedPreview.label, resourceSet: tree.resourceSet, artifactPath: path } : artifact ? selectedPreview : undefined,
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
        if (settings.sourceActivation && settings.artifact === undefined) {
            const projection = success?.snapshot().projection;
            const prior = projection?.buildRun.artifacts.find(item => item.path === selectedPath());
            if (!prior || !sourceMatches(prior, settings.sourceResourceId, projection)) explicitArtifact = false;
        }
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
                const related = snapshot().selectedArtifact; if (related) success.selectArtifact(related.id);
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
    return Object.freeze({ snapshot, setContext, setSelection(value) { navigationSelection = { ...value }; const artifact = snapshot().selectedArtifact; if (artifact) viewed().selectArtifact(artifact.id); publish(); }, build, cancel, pin, selectPin, deletePin, refreshPins,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        updateSource(value) { source = value; publish(); },
        selectConfiguration(id) { setContext(context, { build: id, reset: true }); },
        selectArtifact(path) {
            if (pinned) pinnedPath = path; else { artifactPath = path; explicitArtifact = true; }
            const artifact = viewed()?.snapshot().projection?.buildRun.artifacts.find(item => item.path === path || item.path === path?.split(/[?#]/)[0]);
            if (artifact) viewed().selectArtifact(artifact.id);
            publish();
        },
        selectComparisonArtifact(path) {
            if (comparison()?.artifacts.some(item => item.path === path)) { comparisonArtifactPath = path; publish(); }
        },
        async copy(writeText) { const artifact = snapshot().selectedArtifact; if (artifact) { viewed().selectArtifact(artifact.id); await viewed().copyProjection(writeText); publish(); } },
        async download(save) { const artifact = snapshot().selectedArtifact; if (artifact) { viewed().selectArtifact(artifact.id); await viewed().downloadProjection(save); publish(); } },
        dispose() { disposed = true; pinListController?.abort(); clear(); subscribers.clear(); },
    });
}

/** A bounded, read-only per-resource projection. No project graph or save runs. */
export function createCemStudioSourcePreview({ validator, delay = 80 } = {}) {
    const subscribers = new Set();
    let source, controller, timer, version = 0, disposed = false;
    let state = { status: 'idle', stale: false };
    const snapshot = () => Object.freeze({ ...state });
    const publish = () => { if (!disposed) for (const notify of subscribers) notify(snapshot()); };
    const sameContext = next => source?.sourceUri === next?.sourceUri && source?.resourceId === next?.resourceId;
    function setSource(next) {
        if (disposed || sameContext(next) && source?.text === next?.text && source?.revision === next?.revision && source?.contentType === next?.contentType && source?.schema === next?.schema && source?.draft === next?.draft) return;
        const retain = sameContext(next) && state.preview;
        controller?.abort(); clearTimeout(timer); controller = undefined; timer = undefined;
        const currentVersion = ++version;
        source = next;
        if (!next || next.contentType.split(';')[0].trim().toLowerCase() !== 'text/markdown' || next.schema !== 'https://cem.dev/ns/data/markdown/1' || !validator?.runResourceCommand) {
            state = { status: 'idle', stale: false }; publish(); return;
        }
        state = { status: 'pending', stale: Boolean(retain), ...(retain ? { preview: state.preview } : {}) }; publish();
        // Coalesce source edits before allocating native work. Retained success is
        // scoped to the exact source owner and labeled stale while replacing it.
        timer = setTimeout(async () => {
            timer = undefined;
            const active = new AbortController(); controller = active;
            const current = () => !disposed && version === currentVersion && !active.signal.aborted;
            try {
                const bytes = new TextEncoder().encode(next.text);
                if (bytes.byteLength > CEM_STUDIO_LIMITS.sourceBytes) throw new Error('The source exceeds the bounded preview byte limit.');
                const outcome = await validator.runResourceCommand({ bytes, uri: next.sourceUri, contentType: next.contentType, schema: next.schema,
                    projectId: next.projectId, projectRootUri: next.projectRootUri, projectRevision: next.projectRevision, resourceRevision: next.revision, signal: active.signal,
                    argv: ['convert', next.sourceUri, '--content-type', next.contentType, '--schema', next.schema,
                        '--to-content-type', 'text/html', '--to-schema', 'https://cem.dev/ns/data/html/1', '--output-color-type', 'none'] });
                if (!current()) return;
                if (!outcome.output || outcome.output.available === false) throw new Error('The native source transformation produced no available output.');
                if (outcome.output.contentType.split(';')[0].trim().toLowerCase() !== 'text/html') throw new Error('The native source transformation did not produce declared HTML.');
                const preview = createCemStudioPreview({ bytes: outcome.output.bytes, contentType: outcome.output.contentType,
                    label: next.draft ? 'Draft Markdown preview' : 'Saved Markdown preview' });
                state = { status: 'ready', stale: false, preview, sourceUri: next.sourceUri, revision: next.revision, sha256: next.sha256, draft: next.draft };
            } catch (error) {
                if (!current()) return;
                state = { ...state, status: 'error', stale: Boolean(state.preview), error: { message: error.message } };
            } finally { if (current()) { controller = undefined; publish(); } }
        }, delay);
    }
    return Object.freeze({ snapshot, setSource,
        subscribe(notify) { subscribers.add(notify); notify(snapshot()); return () => subscribers.delete(notify); },
        dispose() { disposed = true; ++version; controller?.abort(); clearTimeout(timer); subscribers.clear(); },
    });
}
