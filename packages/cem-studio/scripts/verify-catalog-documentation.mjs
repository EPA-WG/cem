import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';

/** The ordinary README uses the generic native source preview, not a project build. */
export async function verifyCemqlReadmePreview(page) {
    const route = { collection: 'content-types-data', sample: 'cemql-orders', file: 'readme', build: 'report-ready' };
    await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    await page.waitForFunction(() => {
        const source = globalThis.__cemStudioApplication.workbench.snapshot().navigation.sourcePreview;
        return source?.status === 'ready' && source.resourceId === 'readme';
    });
    const state = await page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot();
        return { route: current.navigation.selection, build: current.projectPreview.configurationId,
            error: current.navigation.sourcePreview.error };
    });
    assert.equal(state.error, undefined);
    assert.equal(state.build, route.build);
    for (const [key, value] of Object.entries(route)) assert.equal(new URL(page.url()).searchParams.get(key), value);
    for (const key of ['collection', 'sample', 'file']) assert.equal(state.route[key], route[key]);
    const preview = page.locator('[data-cem-studio-preview] cem-media-preview[mode="artifact-tree"] iframe');
    await preview.scrollIntoViewIfNeeded();
    assert.equal(await preview.getAttribute('sandbox'), 'allow-same-origin');
    assert.equal(await preview.getAttribute('allow'), '');
    const frame = preview.contentFrame();
    const table = frame.getByRole('table').filter({ hasText: /Configuration\s*Typical use\s*Expected result/ });
    await table.waitFor();
    assert.deepEqual((await table.getByRole('columnheader').allTextContents()).map(text => text.trim()), ['Configuration', 'Typical use', 'Expected result']);
    assert.equal(await table.locator('tbody tr').count(), 14);
    assert.equal(await table.locator('tbody tr').first().locator('code').first().textContent(), 'inspect-data');
    assert.ok((await table.textContent()).includes('report-ready'));
}

/** Documentation remains an ordinary resource in the caller's isolated test context. */
export async function verifyCatalogDocumentation({ page, context, url }) {
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    const authoredRoot = new URL('../../../examples/projects/', import.meta.url);
    const directories = (await readdir(authoredRoot, { withFileTypes: true }))
        .filter(item => item.isDirectory()).sort((left, right) => left.name.localeCompare(right.name));
    const definitions = await Promise.all(directories.map(async directory =>
        JSON.parse(await readFile(new URL(`${directory.name}/example.json`, authoredRoot), 'utf8'))));
    const originals = new Map(await Promise.all(definitions.map(async definition => {
        const resource = definition.sources.find(source => source.path === 'README.md');
        assert.equal(resource?.id, 'readme', `${definition.id} must expose the stable README resource`);
        return [definition.id, await readFile(new URL(`../../../${resource.source}`, import.meta.url))];
    })));
    originals.set('schema-examples', await readFile(new URL('schema-examples.md', authoredRoot)));
    const starterReadme = await readFile(new URL('starter.md', authoredRoot));
    const starterSource = Buffer.from('@doc cem-ml 1\n{main | My project}\n');
    const schemaOriginal = originals.get('schema-examples').toString('utf8');
    const bounded = async (operation, label) => {
        let timer;
        try {
            return await Promise.race([
                operation,
                new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`Studio catalog ${label} exceeded 30 seconds: ${page.url()}`)), 30_000); }),
            ]);
        } finally { clearTimeout(timer); }
    };
    const observe = (callback, argument) => bounded(page.evaluate(callback, argument), 'observation');
    const settle = () => bounded(page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled()), 'settling');
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => {
        await bounded(page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route), `opening ${new URLSearchParams(route)}`);
        await settle();
    };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-studio-project-picker cem-source-editor[readonly] [role=textbox]');
    const state = () => observe(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot();
        return { route: current.navigation.selection, content: current.navigation.content,
            structureSelection: current.navigation.trees.structure.selected,
            sampleTextAvailable: current.navigation.sampleTextAvailable, navigationError: current.navigationError,
            draft: current.draft, persistedText: current.persistedText, dirty: current.dirty,
            projectId: current.projectId, resourceId: current.resourceId, editable: current.editable,
            previewConfiguration: current.projectPreview?.configurationId };
    });
    const originalBytes = sample => observe(async sample => {
        const entry = globalThis.__cemStudioApplication.exampleProjects.examples.find(item => item.id === sample);
        if (!entry) throw new Error(`Catalog project ${sample} is unavailable`);
        const bundle = await entry.load(), resource = bundle.project.resources.find(item => item.id === 'readme');
        return { resource, bytes: resource && Array.from(bundle.contents[resource.id]) };
    }, sample);
    const exported = projectId => observe(async projectId => {
        const bundle = (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'export-project', parameters: { projectId } })).value;
        // Repository blobs export ArrayBuffers; catalog loaders use Uint8Arrays.
        const bytes = value => Array.from(value instanceof Uint8Array ? value : new Uint8Array(value));
        return { project: bundle.project, readme: bytes(bundle.contents.readme),
            source: bundle.contents.source === undefined ? undefined : bytes(bundle.contents.source) };
    }, projectId);
    const persistence = () => observe(async () => {
        const projects = (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'list-projects', parameters: { includeTrash: true } })).value;
        return { projects: projects.map(project => ({ id: project.id, name: project.name, revision: project.revision, rootUri: project.rootUri, trashedAt: project.trashedAt })),
            drafts: Object.fromEntries(Object.keys(localStorage).filter(key => key.startsWith('cem-studio-selection-v1:draft-v1:')).map(key => [key, localStorage.getItem(key)])) };
    });
    const verifyOriginalBytes = async () => {
        for (const [sample, expected] of originals) {
            const actual = await originalBytes(sample);
            assert.equal(actual.resource?.path, 'README.md');
            assert.match(actual.resource.contentType, /^text\/markdown(?:;|$)/);
            assert.equal(actual.resource.schema, 'https://cem.dev/ns/data/markdown/1');
            assert.deepEqual(Buffer.from(actual.bytes), expected, `Catalog README bytes changed: ${sample}`);
            assert.equal(actual.resource.sha256, createHash('sha256').update(expected).digest('hex'));
        }
    };
    const checkRoute = (current, route) => {
        assert.equal(current.route.file, 'readme');
        assert.equal(current.navigationError, undefined);
        const params = new URL(page.url()).searchParams, selected = new URLSearchParams(current.structureSelection);
        for (const [key, value] of Object.entries(route)) {
            assert.equal(current.route[key], value); assert.equal(params.get(key), value); assert.equal(selected.get(key), value);
        }
    };
    const browseOriginal = async (sample, retainedDraft) => {
        const route = { sample, file: 'readme' };
        await open({ sample });
        const link = page.getByRole('link', { name: 'README', exact: true });
        const href = new URL(await link.getAttribute('href'), page.url());
        assert.equal(href.searchParams.get('sample'), sample); assert.equal(href.searchParams.get('file'), 'readme');
        const marker = await observe(() => globalThis.__cemReadmeNavigationMarker = crypto.randomUUID());
        await link.click(); await settle();
        assert.equal(await observe(() => globalThis.__cemReadmeNavigationMarker), marker, 'README link must use guarded navigation without reloading');
        const current = await state(); checkRoute(current, route);
        assert.equal(current.content, 'sample-file'); assert.equal(current.sampleTextAvailable, true);
        assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        assert.equal(await bounded(readonly.evaluate(element => element.isContentEditable), 'readonly control'), false);
        assert.deepEqual(Buffer.from(await bounded(readonly.evaluate(element => element.value), 'readonly source'), 'utf8'), originals.get(sample));
        assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), await page.title());
        if (retainedDraft !== undefined) { assert.equal(current.dirty, true); assert.equal(current.draft, retainedDraft); }
    };
    const submit = async (name, action) => {
        await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: action, exact: true }).click(); await settle();
    };
    const cloneSchema = async name => {
        await open({ sample: 'schema-examples' }); await click('Clone selected'); await submit(name, 'Clone');
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const edit = async value => {
        await observe(value => navigator.clipboard.writeText(value), value);
        await editor.focus(); await editor.press('ControlOrMeta+A'); await editor.press('ControlOrMeta+V');
        assert.equal(await bounded(editor.evaluate(element => element.value), 'edited source'), value); await settle();
    };

    await page.goto(`${url}/`); await ready();
    const baseline = await persistence();
    assert.notEqual((await state()).dirty, true, 'Use an isolated clean editor; existing drafts must not be discarded by verification');
    const catalogIds = await observe(() => globalThis.__cemStudioApplication.exampleProjects.examples.map(item => item.id).sort());
    assert.deepEqual(catalogIds, [...originals.keys()].sort());
    await verifyOriginalBytes();
    console.log(`[cem-studio:catalog-docs] browsing ${originals.size} original README resources`);
    for (const sample of originals.keys()) await browseOriginal(sample);
    await verifyCemqlReadmePreview(page);
    const sourceLink = page.locator('[data-cem-studio-preview] cem-media-preview[mode="artifact-tree"] iframe').contentFrame()
        .getByRole('link', { name: 'data/orders.json', exact: true });
    await sourceLink.click(); await settle();
    const orders = definitions.find(item => item.id === 'cemql-orders').sources.find(item => item.path === 'data/orders.json');
    assert.equal((await state()).route.file, orders.id);
    assert.equal(new URL(page.url()).searchParams.get('build'), 'report-ready');
    assert.equal(await readonly.evaluate(element => element.value), await readFile(new URL(`../../../${orders.source}`, import.meta.url), 'utf8'));
    const showcase = await observe(() => globalThis.__cemStudioApplication.exampleProjects.examples.find(item => item.id === 'schema-examples').load());
    await browseOriginal('schema-examples');
    await page.waitForFunction(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.sourcePreview?.status === 'ready');
    const showcaseFrame = page.locator('[data-cem-studio-preview] cem-media-preview[mode="artifact-tree"] iframe').contentFrame();
    for (const resource of showcase.project.resources.filter(item => item.id.startsWith('example-'))) {
        await showcaseFrame.getByRole('link', { name: resource.path, exact: true }).waitFor();
    }


    await open({}); await click('Create'); await submit('Catalog documentation starter', 'Create');
    const created = await state(), starterId = created.route.project; assert.ok(starterId);
    assert.equal(created.previewConfiguration, 'html-v1:source');
    const starter = await exported(starterId);
    // Repository exports sort resources independently of the authored starter.
    // Its initial source is observed through the selected document preview above.
    assert.deepEqual(starter.project.resources.map(resource => resource.id).sort(), ['readme', 'source']);
    assert.deepEqual(starter.project.entries, []);
    const sourceResource = starter.project.resources.find(resource => resource.id === 'source');
    const readmeResource = starter.project.resources.find(resource => resource.id === 'readme');
    assert.equal(sourceResource.path, 'data/main.cem');
    assert.equal(sourceResource.contentType, 'application/cem');
    assert.equal(sourceResource.schema, 'https://cem.dev/ns/cem-ml/1');
    assert.equal(readmeResource.path, 'README.md');
    assert.equal(readmeResource.contentType, 'text/markdown; charset=utf-8');
    assert.equal(readmeResource.schema, 'https://cem.dev/ns/data/markdown/1');
    assert.deepEqual(Buffer.from(starter.source), starterSource); assert.deepEqual(Buffer.from(starter.readme), starterReadme);
    await open({ project: starterId }); await page.getByRole('link', { name: 'README', exact: true }).click(); await settle(); checkRoute(await state(), { project: starterId, file: 'readme' });
    assert.equal((await state()).editable, true);
    assert.equal(await bounded(editor.evaluate(element => element.value), 'starter README'), starterReadme.toString('utf8'));
    console.log('[cem-studio:catalog-docs] original README URLs/read-only views and source-first two-resource starter passed');

    const first = await cloneSchema('Catalog documentation saved copy');
    await open({ project: first }); await page.getByRole('link', { name: 'README', exact: true }).click(); await settle();
    assert.equal(await bounded(editor.evaluate(element => element.value), 'schema README'), schemaOriginal);
    const saved = `${schemaOriginal}\n## Saved documentation note\n\nCafé Amsterdam belongs to this saved copy.\n`;
    await edit(saved); await click('Save and validate');
    assert.equal((await state()).dirty, false); assert.equal((await state()).persistedText, saved);
    const second = await cloneSchema('Catalog documentation independent copy'); assert.notEqual(first, second);
    await open({ project: second, file: 'readme' });
    assert.equal(await bounded(editor.evaluate(element => element.value), 'independent README'), schemaOriginal);
    const firstSaved = await exported(first), secondSaved = await exported(second);
    assert.notEqual(firstSaved.project.rootUri, secondSaved.project.rootUri);
    assert.equal(Buffer.from(firstSaved.readme).toString('utf8'), saved);
    assert.deepEqual(Buffer.from(secondSaved.readme), originals.get('schema-examples'));
    assert.ok(firstSaved.project.revision > secondSaved.project.revision);

    await open({ project: first }); await page.getByRole('link', { name: 'README', exact: true }).click(); await settle();
    const draft = `${saved}\nUnsaved note: keep this draft while browsing the catalog.\n`;
    await edit(draft); assert.equal((await state()).dirty, true);
    await browseOriginal('schema-examples', draft);
    await browseOriginal('login', draft);
    await open({ project: first });
    assert.equal(await observe(() => globalThis.__cemStudioApplication.workbench.snapshot().transition?.stage), 'source');
    assert.equal((await state()).draft, draft);
    await click('Cancel navigation');
    await open({ project: first, file: 'readme' });
    assert.equal(await bounded(editor.evaluate(element => element.value), 'retained draft'), draft);
    console.log('[cem-studio:catalog-docs] saved README editing, independent clones and retained unsaved catalog browsing passed');

    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        const restored = await state(); checkRoute(restored, { project: first, file: 'readme' });
        assert.equal(restored.dirty, true); assert.equal(restored.draft, draft); assert.equal(restored.persistedText, saved);
        assert.equal(await bounded(editor.evaluate(element => element.value), 'offline restored draft'), draft);
        for (const sample of originals.keys()) await browseOriginal(sample, draft);
        await verifyCemqlReadmePreview(page);
        await sourceLink.click(); await settle();
        assert.equal((await state()).route.file, orders.id);
        assert.equal((await state()).dirty, true); assert.equal((await state()).draft, draft);
        await verifyOriginalBytes();
        await open({ project: first, file: 'readme' });
        assert.equal(await bounded(editor.evaluate(element => element.value), 'offline retained draft'), draft);
        assert.equal(Buffer.from((await exported(first)).readme).toString('utf8'), saved);
        assert.deepEqual(Buffer.from((await exported(second)).readme), originals.get('schema-examples'));
    } finally { await context.setOffline(false); }
    const final = await persistence();
    for (const record of baseline.projects) assert.deepEqual(final.projects.find(project => project.id === record.id), record, `Existing project changed: ${record.id}`);
    for (const [key, value] of Object.entries(baseline.drafts)) assert.equal(final.drafts[key], value, 'Existing draft recovery must remain unchanged');
    assert.equal(final.projects.length, baseline.projects.length + 3);
    console.log(`[cem-studio:catalog-docs] all ${originals.size} READMEs available offline; drafts, saved copies and originals unchanged`);
    return { authoredProjects: definitions.length, catalogReadmes: originals.size, originalReadmeBytes: true,
        readonlyOriginals: true, readmeUrlSelection: true, starterReadme: true, starterSourceFirst: true,
        starterTwoResources: true, starterDefaultPreview: true, savedSchemaReadme: true, independentCopies: true,
        unsavedDraftRetained: true, offlineDraftRestored: true, offlineReadmes: originals.size,
        unchangedOriginals: true, savedBytesPreserved: true, existingProjectsPreserved: true, existingDraftsPreserved: true,
        projectHomeReadmeLinks: true, sourceFileLinks: true, offlineSourceFileLinks: true, allServedTypeFiles: true,
        nativeReadmeTables: true, offlineReadmeTables: true };
}
