import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';

/** Catalog transport/admission coverage. Format helpers own execution and repairs. */
export async function verifyCatalogProjects({ page, context, url, projectIds }) {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    const generated = new URL('../generated/feature-tour/', import.meta.url);
    const index = JSON.parse(await readFile(new URL('index.json', generated), 'utf8'));
    const schemaCatalog = JSON.parse(await readFile(new URL('catalog.json', generated), 'utf8'));
    const selected = projectIds ? index.projects.filter(record => projectIds.includes(record.id)) : index.projects;
    if (projectIds) assert.deepEqual(selected.map(record => record.id).sort(), [...projectIds].sort(), 'Requested catalog projects must exist');
    const expected = await Promise.all(selected.map(async record => {
        const archive = await readFile(new URL(record.archive, generated));
        assert.equal(createHash('sha256').update(archive).digest('hex'), record.sha256);
        const bundle = parseCemStudioProjectBundle(archive);
        const definition = record.id === 'schema-examples' ? undefined : JSON.parse(await readFile(
            new URL(`../../../examples/projects/${record.id}/example.json`, import.meta.url), 'utf8'));
        const configurations = bundle.project.entries.filter(entry => entry.kind !== 'subproject').map(entry => {
            const sourceId = definition?.configurations.find(configuration => configuration.id === entry.id)?.source
                ?? schemaCatalog.examples.find(example => example.runConfigResourceId === entry.runConfigResourceId)?.resourceId
                ?? schemaCatalog.workbenches.find(workbench => workbench.entryId === entry.id)?.resourceId;
            assert.ok(sourceId, `${record.id}/${entry.id} has no declared source`);
            assert.ok(entry.resourceIds.includes(sourceId));
            return { ...entry, sourceId };
        });
        const folders = new Set();
        for (const resource of bundle.project.resources) {
            const parts = resource.path.split('/');
            for (let length = 1; length < parts.length; length++) folders.add(`${parts.slice(0, length).join('/')}/`);
        }
        if (bundle.project.entries.length) folders.add('@configurations');
        for (const entry of bundle.project.entries.filter(entry => entry.kind === 'subproject')) folders.add(`@configurations/${entry.id}`);
        return { record, bundle, configurations, folders: [...folders].sort() };
    }));
    const bounded = async (operation, label) => {
        let timer;
        try {
            return await Promise.race([operation, new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error(`Studio catalog ${label} exceeded 30 seconds: ${page.url()}`)), 30_000);
            })]);
        } finally { clearTimeout(timer); }
    };
    const observe = (callback, argument) => bounded(page.evaluate(callback, argument), 'observation');
    const settle = () => observe(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => {
        await bounded(page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route),
            `opening ${new URLSearchParams(route)}`);
        await settle();
    };
    const state = () => observe(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), nav = current.navigation;
        return { route: nav.selection, content: nav.content, selectedKey: nav.selected.key,
            structureSelection: nav.trees.structure.selected, catalogSelection: nav.trees.projects.selected,
            children: nav.children.map(child => ({ name: child.name, description: child.description })),
            sampleTextAvailable: nav.sampleTextAvailable, sampleAvailable: nav.sampleAvailable,
            navigationError: current.navigationError ?? nav.error, status: current.status,
            projectId: current.projectId, resourceId: current.resourceId, entryId: current.entryId,
            editable: current.editable, runnable: current.runnable, dirty: current.dirty,
            draft: current.draft, persistedText: current.persistedText,
            buildChoices: current.projectPreview?.configurations.map(choice => choice.id) ?? [],
            buildAvailable: current.projectPreview?.available ?? false };
    });
    const readonly = page.locator('cem-studio-project-picker cem-source-editor[readonly] [role=textbox]');
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const text = bytes => new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    const binary = resource => resource.contentType.split(';')[0].trim().endsWith('+cem-bin');
    const readBundle = (projectId, sample = false) => observe(async ({ projectId, sample }) => {
        const app = globalThis.__cemStudioApplication;
        const bundle = sample ? await app.exampleProjects.examples.find(example => example.id === projectId).load()
            : (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
                operation: 'export-project', parameters: { projectId } })).value;
        const bytes = value => Array.from(value instanceof Uint8Array ? value : new Uint8Array(value));
        return { project: bundle.project, contents: Object.fromEntries(Object.entries(bundle.contents).map(([id, value]) => [id, bytes(value)])) };
    }, { projectId, sample });
    const persistence = () => observe(async () => {
        const projects = (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'list-projects', parameters: { includeTrash: true } })).value;
        return { projects, drafts: Object.fromEntries(Object.keys(localStorage)
            .filter(key => key.startsWith('cem-studio-selection-v1:draft-v1:')).map(key => [key, localStorage.getItem(key)])) };
    });
    const assertBundle = (actual, item, cloned = false) => {
        const { bundle } = item;
        assert.deepEqual(actual.project.entries.map(entry => entry.id), bundle.project.entries.map(entry => entry.id));
        assert.deepEqual(actual.project.resources.map(resource => resource.id).sort(), bundle.project.resources.map(resource => resource.id).sort());
        for (const resource of bundle.project.resources) {
            const actualResource = actual.project.resources.find(candidate => candidate.id === resource.id);
            for (const field of ['path', 'contentType', 'schema', 'sha256']) assert.equal(actualResource[field], resource[field], `${item.record.id}/${resource.id}: ${field}`);
            assert.deepEqual(Buffer.from(actual.contents[resource.id]), Buffer.from(bundle.contents[resource.id]), `${item.record.id}/${resource.id}: bytes`);
        }
        if (cloned) {
            assert.notEqual(actual.project.id, bundle.project.id);
            assert.notEqual(actual.project.rootUri, bundle.project.rootUri);
            assert.equal(actual.project.rootUri, `studio://${actual.project.id}/`);
        }
    };
    const checkRoute = async route => {
        const current = await state(), params = new URL(page.url()).searchParams;
        assert.equal(current.navigationError, undefined, JSON.stringify(current.navigationError));
        for (const [key, value] of Object.entries(route)) {
            assert.equal(current.route[key], value); assert.equal(params.get(key), value);
        }
        assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), await page.title());
        if (route.file || route.configuration || route.folder) {
            assert.equal(current.structureSelection, current.selectedKey);
            const selected = new URLSearchParams(current.structureSelection);
            for (const [key, value] of Object.entries(route)) assert.equal(selected.get(key), value);
        }
        return current;
    };
    const exactDownload = async (item, resource) => {
        assert.equal(await page.getByRole('button', { name: 'Download resource', exact: true }).isVisible(), true);
        const downloaded = await observe(async () => {
            let result;
            await globalThis.__cemStudioApplication.workbench.downloadSource(async value => {
                result = { filename: value.filename, contentType: value.contentType,
                    bytes: Array.from(value.bytes instanceof Uint8Array ? value.bytes : new Uint8Array(value.bytes)) };
            });
            return result;
        });
        assert.equal(downloaded.filename, resource.path.split('/').at(-1));
        assert.equal(downloaded.contentType, resource.contentType);
        assert.deepEqual(Buffer.from(downloaded.bytes), Buffer.from(item.bundle.contents[resource.id]));
    };
    const browseFolders = async (item, base) => {
        for (const folder of item.folders) {
            const route = { ...base, folder }; await open(route);
            const current = await checkRoute(route); assert.equal(current.content, 'folder');
            const listing = page.getByRole('list', { name: 'Folder contents', exact: true });
            for (const child of current.children) {
                assert.ok(child.description);
                assert.equal(await listing.getByRole('button', { name: child.name, exact: true }).isVisible(), true);
            }
        }
    };
    const browseOriginalResource = async (item, base, resource) => {
        const route = { ...base, file: resource.id }; await open(route);
        const current = await checkRoute(route); assert.equal(current.content, 'sample-file'); assert.equal(current.sampleAvailable, true);
        assert.equal(current.sampleTextAvailable, !binary(resource), `${item.record.id}/${resource.id}: original source policy`);
        if (binary(resource)) {
            assert.equal(await readonly.count(), 0); await exactDownload(item, resource);
        } else {
            assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
            assert.equal(await bounded(readonly.evaluate(element => element.isContentEditable), 'readonly control'), false);
            assert.equal(await bounded(readonly.evaluate(element => element.value), 'original source'), text(item.bundle.contents[resource.id]));
        }
    };
    const browseOriginalConfiguration = async (item, base, configuration) => {
        const route = { ...base, configuration: configuration.id }; await open(route);
        const current = await checkRoute(route); assert.equal(current.content, 'sample-file');
        assert.equal(current.sampleTextAvailable, true); assert.equal(current.sampleAvailable, true);
        assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        assert.equal(await bounded(readonly.evaluate(element => element.isContentEditable), 'readonly configuration control'), false);
        assert.equal(await bounded(readonly.evaluate(element => element.value), 'original configuration'), text(item.bundle.contents[configuration.runConfigResourceId]));
        assert.equal(await editor.count(), 0);
        assert.equal(await page.getByRole('button', { name: 'Run configuration', exact: true }).isVisible(), false);
        assert.equal(await page.getByRole('button', { name: 'Validate persisted revision', exact: true }).isVisible(), false);
    };
    const browseSavedResource = async (item, project, resource) => {
        const route = { project, file: resource.id }; await open(route);
        const current = await checkRoute(route); assert.equal(current.content, 'file'); assert.equal(current.resourceId, resource.id);
        assert.equal(current.editable, !binary(resource), `${item.record.id}/${resource.id}: saved source policy`);
        if (binary(resource)) {
            assert.equal(current.status, 'read-only'); assert.equal(current.runnable, false);
            assert.equal(await editor.count(), 0); await exactDownload(item, resource);
        } else assert.equal(await bounded(editor.evaluate(element => element.value), 'saved source'), text(item.bundle.contents[resource.id]));
    };
    const browseSavedConfiguration = async (item, project, configuration) => {
        const route = { project, configuration: configuration.id }; await open(route);
        const current = await checkRoute(route), resource = item.bundle.project.resources.find(resource => resource.id === configuration.sourceId);
        const command = item.bundle.project.resources.find(resource => resource.id === configuration.runConfigResourceId);
        const runnable = !binary(resource) && command.schema === 'https://cem.dev/ns/cli/command/1';
        assert.equal(current.content, 'file'); assert.equal(current.entryId, configuration.id); assert.equal(current.resourceId, resource.id);
        assert.equal(current.editable, !binary(resource)); assert.equal(current.runnable, runnable);
        if (binary(resource)) {
            assert.equal(current.status, 'read-only'); assert.equal(await editor.count(), 0); await exactDownload(item, resource);
            assert.equal(await page.getByRole('button', { name: 'Run configuration', exact: true }).isVisible(), false);
            return { id: configuration.id, sourceId: resource.id, admission: 'read-only-binary', runEnabled: false, actions: ['open-configuration', 'download-resource'] };
        }
        assert.equal(await bounded(editor.evaluate(element => element.value), 'configuration source'), text(item.bundle.contents[resource.id]));
        assert.equal(await page.getByRole('button', { name: 'Run configuration', exact: true }).isDisabled(), !runnable);
        const action = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        const button = page.getByRole('button', { name: action, exact: true });
        if (!await button.isVisible()) {
            await page.locator('cem-expansion[label="Editor details"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        assert.equal(await button.isVisible(), true); assert.equal(await button.isDisabled(), false);
        return { id: configuration.id, sourceId: resource.id, admission: runnable ? 'editable-command' : 'editable-validation',
            runEnabled: runnable, actions: ['open-configuration', `available:${action}`] };
    };
    const visitOccurrence = async (item, collection) => {
        await open({ collection }); await checkRoute({ collection });
        await page.getByRole('list', { name: 'Folder contents', exact: true }).getByRole('button', { name: item.record.name, exact: true }).click(); await settle();
        const current = await checkRoute({ collection, sample: item.record.id }); assert.equal(current.content, 'project');
        const selected = new URLSearchParams(current.catalogSelection);
        assert.equal(selected.get('collection'), collection); assert.equal(selected.get('sample'), item.record.id);
    };

    await page.goto(`${url}/`); await ready();
    const controlled = await observe(async () => { await navigator.serviceWorker.ready; return navigator.serviceWorker.controller !== null; });
    if (!controlled) { await page.reload(); await ready(); }
    const baseline = await persistence(), projects = [], roots = new Set();
    assert.notEqual((await state()).dirty, true, 'Catalog verification must never discard an admitted draft');
    assert.deepEqual(await observe(() => globalThis.__cemStudioApplication.exampleProjects.examples.map(example => example.id).sort()),
        index.projects.map(item => item.id).sort());
    for (const item of expected) {
        const { record, bundle } = item, collections = record.collections?.length ? record.collections : ['examples'];
        console.log(`[cem-studio:catalog-projects] ${record.id}: ${collections.length} occurrences, ${bundle.project.resources.length} resources, ${item.configurations.length} configuration routes`);
        assertBundle(await readBundle(record.id, true), item);
        for (const collection of collections) await visitOccurrence(item, collection);
        const base = { collection: collections[0], sample: record.id };
        await browseFolders(item, base);
        for (const resource of bundle.project.resources) await browseOriginalResource(item, base, resource);
        for (const configuration of item.configurations) await browseOriginalConfiguration(item, base, configuration);
        await open(base); await page.getByRole('button', { name: 'Clone selected', exact: true }).click(); await settle();
        await page.getByRole('dialog').getByRole('textbox').fill(`Catalog flow · ${record.name}`);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const cloned = await state(), projectId = cloned.route.project; assert.ok(projectId); assert.equal(cloned.content, 'project');
        const copy = await readBundle(projectId); assertBundle(copy, item, true);
        assert.equal(roots.has(copy.project.rootUri), false); roots.add(copy.project.rootUri);
        assert.equal(await page.getByRole('button', { name: 'Build project', exact: true }).isDisabled(), !cloned.buildAvailable);
        await browseFolders(item, { project: projectId });
        for (const resource of bundle.project.resources) await browseSavedResource(item, projectId, resource);
        const configurations = [];
        for (const configuration of item.configurations) configurations.push(await browseSavedConfiguration(item, projectId, configuration));
        assertBundle(await readBundle(projectId), item, true);
        projects.push({ id: record.id, collectionIds: [...collections], folderPaths: item.folders,
            resourceIds: bundle.project.resources.map(resource => resource.id), configurationIds: item.configurations.map(configuration => configuration.id),
            readonlyResourceIds: bundle.project.resources.filter(binary).map(resource => resource.id),
            savedProjectId: projectId, rootUri: copy.project.rootUri, storedEntryIds: copy.project.entries.map(entry => entry.id),
            buildChoiceIds: cloned.buildChoices, configurations, actions: ['visit-contextual-occurrences', 'open-original-resources', 'open-original-configurations',
                'clone-selected', 'open-saved-resources', 'open-saved-configurations'] });
    }

    const draftItem = expected.find(item => item.record.id === 'schema-examples') ?? expected[0];
    const draftOwner = projects.find(project => project.id === draftItem.record.id), draftRoute = { project: draftOwner.savedProjectId, file: 'readme' };
    const persisted = text(draftItem.bundle.contents.readme), draft = `${persisted}\nUnsaved catalog navigation note: Café Amsterdam.\n`;
    await open(draftRoute);
    await observe(value => navigator.clipboard.writeText(value), draft);
    await editor.focus(); await editor.press('ControlOrMeta+A'); await editor.press('ControlOrMeta+V'); await settle();
    assert.equal(await bounded(editor.evaluate(element => element.value), 'unsaved README'), draft);
    assert.equal((await state()).dirty, true);
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        const restored = await checkRoute(draftRoute);
        assert.equal(restored.dirty, true); assert.equal(restored.draft, draft); assert.equal(restored.persistedText, persisted);
        assert.equal(await bounded(editor.evaluate(element => element.value), 'offline README draft'), draft);
        for (const item of expected) {
            console.log(`[cem-studio:catalog-projects] ${item.record.id}: offline archive, folders, resources and configuration URLs`);
            const record = projects.find(project => project.id === item.record.id);
            assertBundle(await readBundle(item.record.id, true), item);
            for (const collection of record.collectionIds) await visitOccurrence(item, collection);
            const base = { collection: record.collectionIds[0], sample: item.record.id };
            await browseFolders(item, base);
            for (const resource of item.bundle.project.resources) await browseOriginalResource(item, base, resource);
            for (const configuration of item.configurations) await browseOriginalConfiguration(item, base, configuration);
            const retained = await state(); assert.equal(retained.dirty, true); assert.equal(retained.draft, draft);
            assertBundle(await readBundle(record.savedProjectId), item, true);
            record.offline = { archive: true, collectionIds: record.collectionIds, folderPaths: item.folders,
                resourceIds: record.resourceIds, configurationIds: record.configurationIds, actions: ['load-archive', 'open-original-resources', 'open-original-configurations', 'read-saved-copy'] };
        }
        await open(draftRoute); await checkRoute(draftRoute);
        assert.equal(await bounded(editor.evaluate(element => element.value), 'retained offline README'), draft);
    } finally { await context.setOffline(false); }
    for (const item of expected) assertBundle(await readBundle(item.record.id, true), item);
    const final = await persistence();
    for (const project of baseline.projects) assert.deepEqual(final.projects.find(candidate => candidate.id === project.id), project, `Existing project changed: ${project.id}`);
    for (const [key, value] of Object.entries(baseline.drafts)) assert.equal(final.drafts[key], value, 'Existing draft journal changed');
    assert.equal(final.projects.length, baseline.projects.length + expected.length);
    console.log(`[cem-studio:catalog-projects] ${projects.length} projects: exact independent copies and complete online/offline routing passed; configuration execution remains separate`);
    return { coverage: 'navigation-clone-admission-offline', projects, executedConfigurationIds: [],
        unchangedOriginalProjectIds: projects.map(project => project.id), existingProjectIds: baseline.projects.map(project => project.id),
        existingDraftsPreserved: Object.keys(baseline.drafts).length,
        draft: { projectId: draftOwner.savedProjectId, resourceId: 'readme', actions: ['native-paste', 'offline-reload', 'restore-unsaved-draft'],
            persistedBytesUnchanged: true } };
}
