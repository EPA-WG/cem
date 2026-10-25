import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Ordinary portable controls execute through the existing native/project services. */
export async function verifyPortableConfigProject({ page, context, url }) {
    const id = 'portable-config', buildId = 'build-card';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty,
            transition: current.transition?.stage, text: current.projection?.output?.text,
            diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            commandText: current.command?.current?.text, commandArgv: current.command?.current?.argv,
            commandSemantic: current.command?.current?.semantic,
            buildStatus: preview.attempt?.status, buildDiagnostics: preview.attempt?.diagnostics,
            runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            buildText: preview.projection?.output?.text,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256, provenance: item.provenance })) };
    });
    const save = async value => { await editor.fill(value); await settle(); await click('Save and validate'); };
    const edit = async (project, file, value) => { await open({ project, file, build: buildId }); await save(value); };
    const run = async (project, name) => {
        const configuration = operation(name);
        await open({ project, configuration: name, build: buildId }); assert.equal((await state()).editable, true, JSON.stringify(await state()));
        const buttonName = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name: buttonName, exact: true }).isVisible()) {
            const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(buttonName); return state();
    };
    const build = async project => { await open({ project, build: buildId }); await click('Build project'); return state(); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const inspectCommand = async () => {
        const expansion = page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first();
        if (await expansion.getAttribute('aria-expanded') === 'false') { await expansion.click(); await settle(); }
        await page.getByRole('tab', { name: 'Source and command', exact: true }).click(); await settle();
        await page.getByRole('tab', { name: 'Studio command', exact: true }).click(); await settle();
        return page.getByRole('textbox', { name: 'Studio command (literal argv)', exact: true }).inputValue();
    };
    const rendered = async (heading = 'Dispatch board', customer = 'Café Noord') => {
        await page.waitForFunction(({ heading, customer }) => {
            const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument;
            return doc?.querySelector('h1')?.textContent === heading && doc.querySelector('main')?.textContent?.includes(customer);
        }, { heading, customer });
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        assert.equal(await frame.evaluate(element => element.contentDocument.querySelectorAll('script,iframe,object,embed').length), 0);
    };
    const originalRoute = file => ({ collection: 'content-types-schema-authoring', sample: id, ...(file ? { file } : {}), build: buildId });
    const originalIds = definition.sources.filter(item => !item.path.startsWith('expected/') && item.id !== 'readme').map(item => item.id);
    const originals = {};
    for (const collection of ['content-types-schema-authoring', 'content-types-queries-transformations']) {
        await page.goto(`${url}/?collection=${collection}`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
    }
    for (const name of originalIds) {
        await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        originals[name] = await readonly.evaluate(element => element.value);
    }
    await open(originalRoute()); const project = await clone(`Navigator ${definition.name}`);
    const repaired = {};
    for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
        const current = await run(project, configuration.id);
        if (!configuration.repair) {
            assert.equal(current.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(current));
            for (const text of configuration.expectedText ?? []) assert.ok(current.text.includes(text), `${configuration.id}: ${text}`);
            if (configuration.expectedOutput) assert.equal(current.text, await fixture(configuration.expectedOutput));
            if (configuration.id === 'run-check') {
                const report = JSON.parse(current.text); assert.equal(report.summary.errorCount, 0); assert.equal(report.summary.inputCount, 1);
                assert.equal(new URL(report.inputs[0], `studio://${project}/`).href, `studio://${project}/data/card.xml`);
                assert.ok(JSON.stringify(current.commandSemantic.inputs.normalized).includes(`studio://${project}/data/card.xml`));
                assert.match(await inspectCommand(), /validate/);
            }
        } else {
            const expected = configuration.expectedValidation, diagnostic = current.diagnostics?.find(item => item.code === expected.errorCode);
            assert.ok(diagnostic, JSON.stringify(current)); assert.equal(diagnostic.severity, 'error');
            assert.equal(current.diagnostics.filter(item => item.severity === 'error').length, 1);
            assert.equal(diagnostic.uri, `studio://${project}/${resource(configuration.source).path}`);
            for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value);
            repaired[configuration.source] = originals[configuration.source].replace(configuration.repair.find, configuration.repair.replace);
            assert.notEqual(repaired[configuration.source], originals[configuration.source]);
            await edit(project, configuration.source, repaired[configuration.source]);
            assert.equal((await run(project, configuration.id)).status, 'valid');
        }
    }
    const baseline = await build(project); assert.equal(baseline.buildStatus, 'succeeded', JSON.stringify(baseline));
    assert.equal(baseline.entry, 'generated/index.html'); assert.equal(baseline.buildText, await fixture('expected-html'));
    assert.ok(baseline.artifacts[0].provenance.some(item => item.uri.endsWith('/templates/card.cemt'))); await rendered();
    console.log('[cem-studio:portable-config] catalog, native control codecs, source repairs, normalized run config and exact HTML entry passed');

    // The source manifest is a public control document, not a parser AST.
    const importManifest = async outerProject => {
        const imported = await page.evaluate(async projectId => {
            const app = globalThis.__cemStudioApplication;
            const outer = (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', operation: 'export-project', requestRevision: 1, parameters: { projectId } })).value;
            const project = await app.validator.decodeProjectManifest(new Uint8Array(outer.contents['project-cem']));
            const contents = Object.fromEntries(project.resources.map(resource => [resource.id, outer.contents[resource.id]]));
            const result = await app.fileSystemProvider.importFallback({ bundle: { project, contents }, mode: 'create' });
            return { id: result.project.id, root: result.project.rootUri, files: result.project.resources.length };
        }, outerProject); await settle(); return imported;
    };
    const first = await importManifest(project); assert.equal(first.id, 'dispatch-card'); assert.equal(first.root, 'studio://dispatch-card/'); assert.equal(first.files, 7);
    const firstBuild = await build(first.id); assert.equal(firstBuild.buildStatus, 'succeeded'); assert.equal(firstBuild.buildText, baseline.buildText); await rendered();
    await open({ project: first.id, configuration: buildId, build: buildId });
    assert.match(await inspectCommand(), /graphs\/card\.cem/); assert.ok(JSON.stringify((await state()).commandSemantic).includes('studio://dispatch-card/graphs/card.cem'));
    const manifestCopy = originals['project-cem'].replaceAll('dispatch-card', 'portable-imported-card'); assert.notEqual(manifestCopy, originals['project-cem']);
    await edit(project, 'project-cem', manifestCopy);
    const converted = await run(project, 'cem-to-json'); assert.equal(JSON.parse(converted.text).id, 'portable-imported-card');
    const imported = await importManifest(project); assert.equal(imported.id, 'portable-imported-card'); assert.equal(imported.root, 'studio://portable-imported-card/');
    assert.notEqual(imported.root, first.root);
    const importedBuild = await build(imported.id); assert.equal(importedBuild.buildStatus, 'succeeded'); assert.equal(importedBuild.buildText, baseline.buildText); await rendered();

    await open({ project: imported.id, file: 'card-command', build: buildId });
    const editedCommand = originals['card-command'].replace('queueSize=1024', 'queueSize=768'); assert.notEqual(editedCommand, originals['card-command']); await save(editedCommand);
    await open({ project: imported.id, configuration: buildId, build: buildId }); assert.match(await inspectCommand(), /queueSize=768/);
    const commandBuild = await build(imported.id); assert.equal(commandBuild.buildStatus, 'succeeded'); assert.equal(commandBuild.buildText, baseline.buildText);
    await edit(imported.id, 'card-source', originals['card-source'].replace('Café Noord', 'Café Haven'));
    await edit(imported.id, 'card-graph', originals['card-graph'].replace('Dispatch board', 'Saved dispatch'));
    const changed = await build(imported.id); assert.equal(changed.buildStatus, 'succeeded'); await rendered('Saved dispatch', 'Café Haven');
    assert.equal((await build(first.id)).buildText, baseline.buildText);
    const resumed = await build(imported.id); assert.equal(resumed.buildStatus, 'succeeded'); assert.equal(resumed.buildText, changed.buildText);
    await open({ project: imported.id, file: 'card-source', build: buildId }); await save(originals['invalid-card']);
    const refused = await build(imported.id); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, resumed.runId); assert.deepEqual(refused.artifacts, resumed.artifacts);
    assert.ok(refused.buildDiagnostics?.some(item => item.severity === 'error'));
    await edit(imported.id, 'card-source', originals['card-source'].replace('Café Noord', 'Café Haven'));
    assert.equal((await build(imported.id)).buildText, changed.buildText);
    console.log('[cem-studio:portable-config] native manifest import, independent namespace, authored argv edits and saved graph/data builds passed');

    await open({ project: imported.id, file: 'card-check', build: buildId });
    const editedRun = originals['card-check'].replace('data/card.xml', 'data/alternate.xml'); assert.notEqual(editedRun, originals['card-check']); await editor.fill(editedRun); await settle();
    await open({ project: imported.id, configuration: 'run-check', build: buildId }); assert.equal((await state()).transition, 'source');
    await click('Save source'); assert.equal((await state()).route.configuration, 'run-check');
    await click('Run configuration'); const configured = await state(), report = JSON.parse(configured.text);
    assert.equal(report.summary.errorCount, 0); assert.equal(new URL(report.inputs[0], 'studio://portable-imported-card/').href, 'studio://portable-imported-card/data/alternate.xml');
    assert.ok(JSON.stringify(configured.commandSemantic.inputs.normalized).includes('studio://portable-imported-card/data/alternate.xml'));
    assert.equal((await build(imported.id)).buildText, changed.buildText);
    const archived = await page.evaluate(async projectId => {
        const app = globalThis.__cemStudioApplication, first = await app.fileSystemProvider.exportFallback({ projectId }), second = await app.fileSystemProvider.exportFallback({ projectId });
        const before = JSON.stringify(first.bundle.project);
        let duplicateRejected = false;
        try { await app.fileSystemProvider.importFallback({ archive: first.archive.bytes, mode: 'create' }); } catch { duplicateRejected = true; }
        if (!duplicateRejected) throw new Error('Duplicate archive import must refuse create mode');
        const restored = await app.fileSystemProvider.importFallback({ archive: first.archive.bytes, mode: 'replace', expectedRevision: first.bundle.project.revision });
        const after = await app.fileSystemProvider.exportFallback({ projectId });
        return { first: [...first.archive.bytes], second: [...second.archive.bytes], after: [...after.archive.bytes], before,
            restored: restored.project.id, project: after.bundle.project,
            source: new TextDecoder().decode(after.bundle.contents['card-source']), run: new TextDecoder().decode(after.bundle.contents['card-check']) };
    }, imported.id); await settle();
    assert.deepEqual(archived.first, archived.second); assert.deepEqual(archived.after, archived.first); assert.equal(archived.restored, imported.id);
    assert.ok(!['credentials', 'providerBindings', 'selection', 'openTabs', 'drafts'].some(key => Object.hasOwn(archived.project, key)));
    assert.ok(archived.source.includes('Café Haven')); assert.equal(archived.run, editedRun);
    console.log('[cem-studio:portable-config] saved run-config selection, deterministic archives, duplicate refusal and guarded replacement passed');

    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        const offline = await build(imported.id); assert.equal(offline.buildStatus, 'succeeded'); assert.equal(offline.buildText, changed.buildText); await rendered('Saved dispatch', 'Café Haven');
        await open({ project: imported.id, configuration: 'run-check', build: buildId }); await click('Run configuration');
        const checked = JSON.parse((await state()).text); assert.equal(checked.summary.errorCount, 0); assert.equal(new URL(checked.inputs[0], 'studio://portable-imported-card/').href, 'studio://portable-imported-card/data/alternate.xml');
        await open({ project: imported.id, file: 'card-command', build: buildId }); assert.equal(await editor.evaluate(element => element.value), editedCommand);
        await open({ project: imported.id, file: 'card-check', build: buildId }); assert.equal(await editor.evaluate(element => element.value), editedRun);
        assert.equal((await build(first.id)).buildText, baseline.buildText);
        for (const name of originalIds) { await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); assert.equal(await readonly.evaluate(element => element.value), originals[name]); }
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:portable-config] offline imported/archived builds, persisted controls and unchanged catalog originals passed');
    return { catalog: true, readonlyOriginals: true, nativeCommands: true, controlVersionRepairs: true, nativeProjectCodecs: true,
        nativeCommandInspection: true, normalizedRunConfig: true, explicitEntry: true, renderedHtml: true, nativeProvenance: true,
        nativeManifestImport: true, namespaceIndependence: true, authoredCommandEdits: true, savedData: true, savedGraph: true,
        retainedFailure: true, savedRunConfig: true, deterministicArchive: true, duplicateImportRefusal: true,
        guardedArchiveReplacement: true, excludedHostState: true, offline: true };
}
