import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

/** Ordinary CEM-QL source, native results and declared parameter-map builds. */
export async function verifyCemqlProject({ page, context, url }) {
    const id = 'cemql-orders';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
    const contract = async name => JSON.parse(await fixture(name));
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), preview = current.projectPreview;
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty,
            text: current.projection?.output?.text, diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics,
            provenance: current.projection?.provenance, selection: current.selection, resourceId: current.resourceId, navigationError: current.navigationError,
            buildStatus: preview.attempt?.status, runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            buildText: preview.projection?.output?.text, artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256, provenance: item.provenance })) };
    });
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); };
    const run = async (project, name) => {
        const configuration = operation(name);
        await open({ project, configuration: name, build: 'report-ready' });
        const current = await state(); assert.equal(current.editable, true, JSON.stringify(current));
        const buttonName = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name: buttonName, exact: true }).isVisible()) {
            const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(buttonName); return state();
    };
    const build = async (project, name = 'report-ready') => { await open({ project, build: name }); await click('Build project'); return state(); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const items = current => {
        const report = JSON.parse(current.text); assert.equal(report.language, 'cem-ql'); return report.result.items;
    };
    const field = (row, name) => {
        assert.equal(row.kind, 'record'); assert.equal(row.fields[name].length, 1);
        assert.equal(row.fields[name][0].kind, 'atomic'); return row.fields[name][0].value;
    };
    const assertNodes = async (current, project) => {
        const selected = items(current), ranges = await contract('expected-native-ranges');
        assert.equal(selected.length, ranges.length);
        const uri = `studio://${project}/${resource('order-data').path}`;
        for (const [index, item] of selected.entries()) {
            assert.equal(item.kind, 'native'); assert.equal(item.representation, 'cem.ql.imported-cem-ast');
            const frame = item.sourceMap.frames.find(frame => frame.transform.kind === 'SourceBoundary' && frame.transform.uri === uri);
            assert.ok(frame, JSON.stringify(item.sourceMap)); assert.equal(frame.span.kind, 'Single'); assert.deepEqual(frame.span.ranges, ranges[index]);
        }
    };
    const table = page.locator('cem-data-view[format="json"] table:visible');
    const assertTable = async rows => {
        await table.waitFor(); assert.equal(await table.locator('tbody tr').count(), rows);
        assert.equal(await page.locator('cem-data-view[format="json"]:visible').getAttribute('data-cem-density'), 'super-compact');
        assert.match(await table.innerText(), /customer/); assert.match(await table.innerText(), /order/);
    };
    const originalRoute = (file, collection = 'content-types-queries-transformations') => ({ collection, sample: id, ...(file ? { file } : {}), build: 'report-ready' });

    for (const collection of ['content-types-data', 'content-types-queries-transformations']) {
        await page.goto(`${url}/?collection=${collection}`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
    }
    const originals = {};
    const originalIds = ['order-data', 'filter-expression', 'order-module', 'native-node-query', 'parameter-model', 'ready-graph', 'draft-graph', 'invalid-expression', 'invalid-data'];
    for (const name of originalIds) {
        await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        originals[name] = await readonly.evaluate(element => element.value);
    }
    await open(originalRoute());
    const project = await clone(`Navigator ${definition.name}`);
    for (const configuration of definition.configurations.filter(item => !item.repair)) {
        if (configuration.kind === 'transformation-graph') {
            const built = await build(project, configuration.id); assert.equal(built.buildStatus, 'succeeded', JSON.stringify(built));
            assert.equal(built.entry, configuration.entryArtifact);
            const expected = await fixture(configuration.expectedGraphOutputs[configuration.entryArtifact]);
            assert.equal(built.buildText, expected);
            assert.equal(built.artifacts.find(item => item.path === configuration.entryArtifact).sha256,
                createHash('sha256').update(expected).digest('hex'));
            assert.ok(built.artifacts.every(item => item.provenance.length === 0));
            await assertTable(JSON.parse(built.buildText).length);
        } else {
            const result = await run(project, configuration.id); assert.equal(result.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
            for (const text of configuration.expectedText ?? []) assert.ok(result.text.includes(text), `${configuration.id}: missing ${text}`);
            if (configuration.expectedOutput) assert.equal(result.text, await fixture(configuration.expectedOutput));
            if (configuration.expectedNativeItems) assert.deepEqual(items(result), await contract(configuration.expectedNativeItems));
            if (configuration.expectedNativeRanges) await assertNodes(result, project);
        }
    }
    const repair = operation('repair-expression'), expectedFailure = repair.expectedValidation;
    const failed = await run(project, repair.id), diagnostic = failed.diagnostics?.find(item => item.code === expectedFailure.errorCode);
    assert.ok(diagnostic, JSON.stringify(failed)); assert.equal(diagnostic.severity, 'error');
    assert.equal(failed.diagnostics.filter(item => item.severity === 'error').length, 2);
    assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
    for (const [key, value] of Object.entries(expectedFailure.location)) assert.equal(diagnostic[key] ?? null, value);
    await open({ project, file: repair.source, build: 'report-ready' });
    const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]);
    assert.equal(repaired, originals['filter-expression']); await save(repaired); assert.equal((await run(project, repair.id)).status, 'valid');
    console.log('[cem-studio:cemql-orders] catalog, all native typed/source contracts, JSON tables and exact repair passed');

    const filtered = await run(project, 'query-filter'); assert.deepEqual(items(filtered), await contract('expected-filter'));
    const queryUri = `studio://${project}/${resource('filter-expression').path}`;
    const origin = filtered.provenance?.map((frame, index) => ({ ...frame, index })).find(frame => frame.uri === queryUri && frame.range?.len > 0);
    assert.ok(origin, JSON.stringify(filtered.provenance));
    const queryBytes = new TextEncoder().encode(originals['filter-expression']); assert.ok(origin.range.start >= 0 && origin.range.start + origin.range.len <= queryBytes.byteLength);
    const inspector = page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first();
    if (await inspector.getAttribute('aria-expanded') === 'false') { await inspector.click(); await settle(); }
    await page.getByRole('tab', { name: 'Results', exact: true }).click(); await settle();
    await page.getByRole('tab', { name: 'Provenance', exact: true }).click(); await settle();
    await page.getByRole('listbox', { name: 'Source-map provenance', exact: true }).selectOption(String(origin.index)); await settle();
    const followed = await state(); assert.equal(followed.navigationError, undefined); assert.equal(followed.route.file, 'filter-expression'); assert.equal(followed.resourceId, 'filter-expression');
    assert.equal(followed.selection.kind, 'provenance'); assert.equal(followed.selection.byteStart, origin.range.start); assert.equal(followed.selection.byteLength, origin.range.len);
    const selected = await editor.evaluate(element => ({ value: element.value, start: element.selectionStart, end: element.selectionEnd }));
    assert.equal(selected.value.slice(selected.start, selected.end), new TextDecoder('utf-8', { fatal: true }).decode(queryBytes.slice(origin.range.start, origin.range.start + origin.range.len)));

    await run(project, 'query-join');
    const editedData = originals['order-data'].replace('Café Noord', 'Café Haven'); assert.notEqual(editedData, originals['order-data']);
    await editor.fill(editedData); await settle(); await click('Run configuration'); assert.deepEqual(items(await state()), await contract('expected-join')); assert.equal((await state()).dirty, true);
    await click('Save and validate'); await click('Run configuration');
    const joined = items(await state()); assert.equal(joined.length, 2); assert.equal(field(joined[0], 'customer'), 'Café Haven');
    assert.deepEqual(items(await run(project, 'query-filter')), await contract('expected-filter'));
    await open({ project, file: 'filter-expression', build: 'report-ready' });
    const editedFilter = originals['filter-expression'].replace('== "ready"', '== "draft"'); assert.notEqual(editedFilter, originals['filter-expression']); await save(editedFilter);
    const draftItems = items(await run(project, 'query-filter')); assert.equal(draftItems.length, 1); assert.equal(field(draftItems[0], 'order'), 'O-102');
    await open({ project, file: 'ready-graph', build: 'report-ready' });
    const editedGraph = originals['ready-graph'].replace('@value=30', '@value=45'); assert.notEqual(editedGraph, originals['ready-graph']); await save(editedGraph);
    const built = await build(project); assert.equal(built.buildStatus, 'succeeded', JSON.stringify(built));
    const rows = JSON.parse(built.buildText); assert.equal(rows.length, 1); assert.equal(rows[0].customer, 'Café Haven'); assert.equal(rows[0].order, 'O-101'); await assertTable(1);
    assert.ok(built.artifacts.every(item => item.provenance.length === 0));
    await open({ project, file: 'order-data', build: 'report-ready' }); await save(originals['invalid-data']);
    const refused = await build(project); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, built.runId); assert.deepEqual(refused.artifacts, built.artifacts);
    await open({ project, file: 'order-data', build: 'report-ready' }); await save(editedData);
    assert.equal((await build(project)).buildText, built.buildText);
    console.log('[cem-studio:cemql-orders] query-source navigation, persisted data/query edits, parameter maps and last-good JSON output passed');

    const copy = await clone(`Portable ${definition.name}`); assert.notEqual(copy, project);
    await open({ project: copy, file: 'order-module', build: 'report-ready' });
    const editedModule = originals['order-module'].replace('>= 30', '>= 45'); assert.notEqual(editedModule, originals['order-module']); await save(editedModule);
    const copiedJoin = items(await run(copy, 'query-join')); assert.equal(copiedJoin.length, 1); assert.equal(field(copiedJoin[0], 'order'), 'O-101');
    assert.deepEqual(items(await run(project, 'query-join')), joined);
    const copied = await build(copy); assert.equal(copied.buildStatus, 'succeeded'); assert.equal(copied.buildText, built.buildText); await assertTable(1);
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assert.deepEqual(items(await run(copy, 'query-join')), copiedJoin); assert.deepEqual(items(await run(copy, 'query-filter')), draftItems);
        await assertNodes(await run(copy, 'query-source-nodes'), copy);
        const offline = await build(copy); assert.equal(offline.buildStatus, 'succeeded'); assert.equal(offline.buildText, copied.buildText); assert.deepEqual(offline.artifacts, copied.artifacts); await assertTable(1);
        for (const [name, expected] of [['order-data', editedData], ['filter-expression', editedFilter], ['order-module', editedModule], ['ready-graph', editedGraph], [repair.source, repaired]]) {
            await open({ project: copy, file: name, build: 'report-ready' }); assert.equal(await editor.evaluate(element => element.value), expected);
        }
        assert.equal((await run(copy, repair.id)).status, 'valid');
        for (const name of originalIds) { await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); assert.equal(await readonly.evaluate(element => element.value), originals[name]); }
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:cemql-orders] independent module/query copies, native node ranges, JSON builds and saved repairs passed offline');
    return { catalog: true, readonlyOriginals: true, nativeCommands: true, typedRecords: true, nativeSourceRanges: true,
        nativeRepair: true, querySourceNavigation: true, unsavedSourceGuard: true, savedData: true, savedQuery: true,
        parameterMaps: true, jsonTable: true, unmappedJsonBoundary: true, retainedFailure: true, independentCopies: true, offline: true };
}
