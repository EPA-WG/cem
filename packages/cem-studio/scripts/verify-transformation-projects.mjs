import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Ordinary native transformation projects use Studio's shared editor and previews. */
export async function verifyTransformationProjects({ page, context, url }) {
    const evidence = {};
    const collection = 'content-types-queries-transformations';
    const buildId = 'build-report';
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty, transition: current.transition?.stage,
            text: current.projection?.output?.text, diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            buildStatus: preview.attempt?.status, runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256, provenance: item.provenance })),
            outputs: preview.projection?.outputs.map(item => ({ path: preview.projection.buildRun.artifacts.find(artifact => artifact.uri === item.uri)?.path, text: item.text })) };
    });
    const output = (current, path) => { const found = current.outputs?.find(item => item.path === path); assert.ok(found, `Missing native output ${path}`); return found.text; };
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); };
    const run = async (project, configuration) => {
        await open({ project, configuration: configuration.id, build: buildId });
        const current = await state(); assert.equal(current.editable, true, JSON.stringify(current));
        const name = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name, exact: true }).isVisible()) {
            const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(name); return state();
    };
    const build = async project => { await open({ project, build: buildId }); await click('Build project'); return state(); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const selectArtifact = async path => { await page.evaluate(path => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact(path), path); await settle(); };
    const rendered = async (count, heading = 'Ready orders', list = 'ul', customer = 'Café Noord') => {
        await selectArtifact('generated/index.html');
        await page.waitForFunction(({ count, heading, list, customer }) => {
            const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument;
            return doc?.querySelector('h1')?.textContent === heading && doc.querySelectorAll(`${list} > li`).length === count
                && doc.querySelector(`${list} > li`)?.textContent?.includes(customer);
        }, { count, heading, list, customer });
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin'); assert.equal(await frame.getAttribute('allow'), '');
        const view = await frame.evaluate(element => {
            const doc = element.contentDocument;
            return { ids: [...doc.querySelectorAll('li')].map(node => node.getAttribute('data-order')),
                scripts: doc.querySelectorAll('script,iframe,object,embed').length, width: doc.querySelector('main').getBoundingClientRect().width };
        });
        assert.deepEqual(view.ids, count === 2 ? ['O-101', 'O-103'] : ['O-101']); assert.equal(view.scripts, 0); assert.ok(view.width > 0);
    };
    const jsonTable = async count => {
        await selectArtifact('generated/report.json');
        const table = page.locator('cem-data-view[format="json"] table:visible'); await table.waitFor();
        assert.equal(await table.locator('tbody tr').count(), count); assert.match(await table.innerText(), /Café/);
        assert.equal(await page.locator('cem-data-view[format="json"]:visible').getAttribute('data-cem-density'), 'super-compact');
    };

    for (const id of ['cem-transform', 'xslt-transform']) {
        const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
        const resource = name => definition.sources.find(item => item.id === name);
        const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
        const repair = definition.configurations.find(item => item.repair), graph = definition.configurations.find(item => item.kind === 'transformation-graph');
        const hasJson = Object.hasOwn(graph.expectedGraphOutputs, 'generated/report.json');
        const originalIds = ['orders-source', 'report-template', 'page-template', 'report-graph', repair.source, 'invalid-orders'];
        const originals = {};
        const sampleRoute = file => ({ collection, sample: id, ...(file ? { file } : {}), build: buildId });
        const assertArtifacts = async current => {
            assert.equal(current.buildStatus, 'succeeded', JSON.stringify(current)); assert.equal(current.entry, graph.entryArtifact);
            assert.deepEqual(current.artifacts.map(item => item.path).sort(), Object.keys(graph.expectedGraphOutputs).sort());
            for (const [path, expected] of Object.entries(graph.expectedGraphOutputs)) {
                if (path.endsWith('.json')) assert.deepEqual(JSON.parse(output(current, path)), JSON.parse(await fixture(expected)));
                else assert.equal(output(current, path), await fixture(expected));
            }
            const html = current.artifacts.find(item => item.path === graph.entryArtifact);
            assert.ok(html.provenance.some(item => item.uri.endsWith('/templates/page.cemt')));
            if (hasJson) assert.deepEqual(current.artifacts.find(item => item.path.endsWith('.json')).provenance, []);
        };
        await page.goto(`${url}/?collection=${collection}`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
        for (const name of originalIds) {
            await open(sampleRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
            originals[name] = await readonly.evaluate(element => element.value);
        }
        await open(sampleRoute()); await click('Build project'); const sample = await state(); await assertArtifacts(sample); await rendered(2);
        if (hasJson) await jsonTable(2);
        await open(sampleRoute()); const project = await clone(`Navigator ${definition.name}`);
        for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph' && !item.repair)) {
            const result = await run(project, configuration); assert.equal(result.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
            for (const text of configuration.expectedText ?? []) assert.ok(result.text.includes(text), `${id}/${configuration.id} misses ${text}`);
            if (configuration.expectedOutput) assert.equal(result.text, await fixture(configuration.expectedOutput));
        }
        const failed = await run(project, repair), expected = repair.expectedValidation;
        const diagnostic = failed.diagnostics?.find(item => item.code === expected.errorCode); assert.ok(diagnostic, JSON.stringify(failed));
        assert.equal(diagnostic.severity, 'error'); assert.equal(failed.diagnostics.filter(item => item.severity === 'error').length, 1);
        assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
        for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value);
        await open({ project, file: repair.source, build: buildId });
        const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]);
        assert.equal(repaired, originals['report-template']); await save(repaired); assert.equal((await run(project, repair)).status, 'valid');
        console.log(`[cem-studio:${id}] catalog, exact native commands, source repair, entry preview and graph provenance passed`);

        const baseline = await build(project); await assertArtifacts(baseline);
        await open({ project, file: 'orders-source', build: buildId });
        const editedData = originals['orders-source'].replace('Café Noord', 'Café Haven'); assert.notEqual(editedData, originals['orders-source']);
        await editor.fill(editedData); await settle(); assert.equal((await state()).dirty, true);
        await open({ project, build: buildId });
        const unsaved = await state(); assert.equal(unsaved.route.file, 'orders-source'); assert.equal(unsaved.transition, 'source'); assert.equal(unsaved.dirty, true);
        assert.equal(unsaved.runId, baseline.runId); assert.equal(output(unsaved, graph.entryArtifact), output(baseline, graph.entryArtifact));
        await click('Cancel navigation'); assert.equal(await editor.evaluate(element => element.value), editedData);
        assert.equal((await state()).dirty, true); await click('Save and validate');
        const changed = await build(project); assert.ok(output(changed, graph.entryArtifact).includes('Café Haven')); await rendered(2, 'Ready orders', 'ul', 'Café Haven');
        await open({ project, file: 'report-graph', build: buildId });
        const editedGraph = originals['report-graph'].replace('@value=30', '@value=45'); assert.notEqual(editedGraph, originals['report-graph']); await save(editedGraph);
        const selected = await build(project); assert.ok(output(selected, graph.entryArtifact).includes('O-101')); assert.ok(!output(selected, graph.entryArtifact).includes('O-103'));
        await rendered(1, 'Ready orders', 'ul', 'Café Haven'); if (hasJson) { assert.equal(JSON.parse(output(selected, 'generated/report.json')).length, 1); await jsonTable(1); }
        await open({ project, file: 'orders-source', build: buildId }); await save(originals['invalid-orders']);
        const refused = await build(project); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, selected.runId); assert.deepEqual(refused.artifacts, selected.artifacts);
        await open({ project, file: 'orders-source', build: buildId }); await save(editedData);
        assert.equal(output(await build(project), graph.entryArtifact), output(selected, graph.entryArtifact));
        console.log(`[cem-studio:${id}] persisted XML/graph parameters, draft guard and last-good artifacts passed`);

        const copy = await clone(`Portable ${definition.name}`); assert.notEqual(copy, project);
        await open({ project: copy, file: 'page-template', build: buildId });
        const editedTemplate = originals['page-template'].replace('@name=ul', '@name=ol'); assert.notEqual(editedTemplate, originals['page-template']); await save(editedTemplate);
        const copied = await build(copy); assert.ok(output(copied, graph.entryArtifact).includes('<ol>')); await rendered(1, 'Ready orders', 'ol', 'Café Haven');
        if (hasJson) assert.equal(output(copied, 'generated/report.json'), output(selected, 'generated/report.json'));
        assert.equal(output(await build(project), graph.entryArtifact), output(selected, graph.entryArtifact));
        await context.setOffline(true);
        try {
            await page.reload(); await ready();
            const offline = await build(copy); assert.equal(offline.buildStatus, 'succeeded'); assert.equal(output(offline, graph.entryArtifact), output(copied, graph.entryArtifact));
            await rendered(1, 'Ready orders', 'ol', 'Café Haven'); if (hasJson) { assert.equal(output(offline, 'generated/report.json'), output(copied, 'generated/report.json')); await jsonTable(1); }
            assert.equal((await run(copy, repair)).status, 'valid');
            for (const [name, text] of [['orders-source', editedData], ['report-graph', editedGraph], ['page-template', editedTemplate], [repair.source, repaired]]) {
                await open({ project: copy, file: name, build: buildId }); assert.equal(await editor.evaluate(element => element.value), text);
            }
            for (const name of originalIds) { await open(sampleRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); assert.equal(await readonly.evaluate(element => element.value), originals[name]); }
        } finally { await context.setOffline(false); }
        console.log(`[cem-studio:${id}] independent template copies, offline native rebuilds and unchanged catalog originals passed`);
        evidence[id] = { catalog: true, readonlyOriginals: true, nativeCommands: true, exactFormatter: true, sourceRepair: true,
            multiStepGraph: true, explicitEntry: true, renderedHtml: true, nativeProvenance: true, unsavedSourceGuard: true,
            savedData: true, savedGraphParameters: true, retainedFailure: true, independentTemplates: true, offline: true,
            ...(hasJson ? { jsonTable: true, unmappedJsonBoundary: true } : { directXslt: true }) };
    }
    return evidence;
}
