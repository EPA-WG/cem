import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Native XPath values and source identities in an ordinary saved project. */
export async function verifyXpathProject({ page, context, url }) {
    const id = 'xpath-catalog';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
    const contract = async name => JSON.parse(await fixture(name));
    const repair = operation('repair-query');
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const open = async route => { await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonly = page.locator('cem-source-editor[readonly] [role=textbox]');
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot();
        return { route: current.navigation.selection, status: current.status, editable: current.editable, dirty: current.dirty,
            text: current.projection?.output?.text, outputAvailable: current.projection?.output?.available,
            outputContentType: current.projection?.output?.contentType,
            exitCode: current.projection?.exitCode, diagnostics: current.projection?.diagnostics ?? current.validation?.diagnostics };
    });
    const save = async value => { await editor.fill(value); await settle(); await click('Save and validate'); };
    const run = async (project, configuration) => {
        await open({ project, configuration });
        const current = await state(); assert.equal(current.editable, true, JSON.stringify(current));
        const validating = operation(configuration).kind === 'validation';
        const name = validating ? 'Validate persisted revision' : 'Run configuration';
        const button = page.getByRole('button', { name, exact: true });
        if (!await button.isVisible()) {
            const label = validating ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await button.click(); await settle(); return state();
    };
    const report = current => {
        const result = JSON.parse(current.text); assert.equal(result.language, 'xpath'); return result.result;
    };
    const items = current => report(current).sequence.items;
    const values = current => items(current).map(item => item.value.lexicalValue);
    const nodeFields = item => ({ kind: item.kind, nodeId: item.nodeId, nodeKind: item.nodeKind, expandedName: item.expandedName, sourceRange: item.sourceRange });
    const assertNodes = (current, project, source, expectedIds) => {
        const selected = items(current); assert.deepEqual(selected.map(item => item.nodeId), expectedIds);
        const bytes = new TextEncoder().encode(source), uri = `studio://${project}/${resource('catalog-source').path}`;
        for (const item of selected) {
            assert.equal(item.kind, 'node'); assert.equal(item.nodeKind, 'element'); assert.equal(item.expandedName, '{urn:example:catalog}product');
            assert.equal(item.sourceUri, uri);
            const start = item.sourceRange.start.byteOffset, length = item.sourceRange.byteLength;
            assert.ok(start >= 0 && length > 0 && start + length <= bytes.byteLength);
            const original = new TextDecoder('utf-8', { fatal: true }).decode(bytes.slice(start, start + length));
            assert.ok(original.startsWith('<product ') && original.endsWith('>'), original);
            assert.ok(item.sourceMap.frames.some(frame => frame.transform.kind === 'SourceBoundary' && frame.transform.uri === uri
                && frame.span.kind === 'Single' && frame.span.ranges.start === start && frame.span.ranges.len === length));
        }
    };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const assertSourcePresentation = async (result, source) => {
        assert.equal(result.outputContentType, 'text/html');
        const displayed = await page.evaluate(html => {
            const document = new DOMParser().parseFromString(html, 'text/html');
            return { text: document.body.textContent,
                spans: document.querySelectorAll('span[data-role]').length,
                scripts: document.querySelectorAll('script').length };
        }, result.text);
        assert.equal(displayed.text, source);
        assert.ok(displayed.spans > 0);
        assert.equal(displayed.scripts, 0);
    };
    const assertSourceStructure = (result, project, events = false) => {
        assert.equal(result.status, 'projected', JSON.stringify(result));
        assert.ok(result.text.includes(`studio://${project}/${resource('filter-query').path}`));
        assert.ok(result.text.includes('@schema=https://cem.dev/ns/query/xpath/1'));
        assert.ok(result.text.includes(events ? '@token-kind=name' : '@kind=xpath-expression'));
        assert.ok(result.text.includes('@byte-offset='));
        assert.ok(result.text.includes('Q{urn:example:catalog}'));
    };
    const originalRoute = file => ({ collection: 'content-types-queries-transformations', sample: id, ...(file ? { file } : {}) });
    const mapValue = current => {
        const map = items(current)[0]; assert.equal(map.kind, 'map');
        const entry = key => map.entries.find(item => item.key.lexicalValue === key).value.items[0];
        assert.equal(entry('minimum-stock').value.typeName, 'xs:integer');
        assert.equal(entry('region').value.typeName, 'xs:string');
        assert.equal(entry('available-skus').kind, 'array');
        return { minimum: entry('minimum-stock').value.lexicalValue, region: entry('region').value.lexicalValue };
    };

    await page.goto(`${url}/?collection=content-types-queries-transformations`); await ready();
    const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
    assert.ok(record?.description);
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
    assert.equal((await state()).route.sample, id);
    const originals = {};
    for (const name of ['catalog-source', 'filter-query', 'node-query', 'invalid-query']) {
        await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        originals[name] = await readonly.evaluate(element => element.value);
    }
    await open(originalRoute());
    const project = await clone(`Navigator ${definition.name}`);
    for (const command of definition.configurations.filter(item => !item.repair)) {
        const result = await run(project, command.id);
        assert.equal(result.status, command.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
        if (command.expectedOutput) assert.equal(result.text, await fixture(command.expectedOutput));
        for (const text of command.expectedText ?? []) assert.ok(result.text.includes(text), `${command.id}: missing ${text}`);
        if (['parse-filter', 'inspect-filter'].includes(command.id)) assertSourceStructure(result, project);
        if (command.id === 'parse-filter-events') assertSourceStructure(result, project, true);
        if (command.id === 'color-filter') await assertSourcePresentation(result, originals['filter-query']);
        if (command.expectedSemanticContract) assert.deepEqual(values(result), await contract(command.expectedSemanticContract));
        if (command.expectedNativeNodes) {
            assert.deepEqual(items(result).map(nodeFields), await contract(command.expectedNativeNodes));
            assertNodes(result, project, originals['catalog-source'], ['cem:node:27', 'cem:node:45']);
        }
        if (command.expectedNativeItems) {
            assert.deepEqual(items(result), await contract(command.expectedNativeItems));
            assert.equal(report(result).expressionUri, `studio://${project}/${resource('map-query').path}`);
            assert.deepEqual(mapValue(result), { minimum: '4', region: 'west' });
        }
    }
    const failed = await run(project, repair.id), expectedFailure = repair.expectedValidation;
    const diagnostic = failed.diagnostics?.find(item => item.code === expectedFailure.errorCode);
    assert.ok(diagnostic, JSON.stringify(failed)); assert.equal(diagnostic.severity, 'error');
    assert.equal(failed.diagnostics.filter(item => item.severity === 'error').length, 2);
    assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
    for (const [key, value] of Object.entries(expectedFailure.location)) assert.equal(diagnostic[key] ?? null, value);
    await open({ project, file: repair.source });
    const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]);
    assert.equal(repaired, originals['node-query']); await save(repaired);
    assert.equal((await run(project, repair.id)).status, 'valid');
    console.log('[cem-studio:xpath-catalog] catalog, native scalar/node/map contracts, original UTF-8 ranges and repair passed');

    await run(project, 'query-filter');
    const editedCatalog = originals['catalog-source'].replace('stock="2"', 'stock="8"'); assert.notEqual(editedCatalog, originals['catalog-source']);
    await editor.fill(editedCatalog); await settle(); await click('Run configuration');
    assert.deepEqual(values(await state()), await contract('filter-summary')); assert.equal((await state()).dirty, true);
    await click('Save and validate'); await click('Run configuration');
    const allProducts = ['tea:Green tea', 'coffee:Harbor coffee', 'cocoa:Cocoa']; assert.deepEqual(values(await state()), allProducts);
    assertNodes(await run(project, 'query-nodes'), project, editedCatalog, ['cem:node:27', 'cem:node:36', 'cem:node:45']);
    const parameterCatalog = editedCatalog.replace('minimum-stock="4"', 'minimum-stock="6"'); assert.notEqual(parameterCatalog, editedCatalog);
    await open({ project, file: 'catalog-source' }); await save(parameterCatalog);
    const parameters = ['minimum-stock=6', 'region=west', 'tea:Green tea:Café Harbor'];
    assert.deepEqual(values(await run(project, 'query-parameters')), parameters);
    assert.deepEqual(values(await run(project, 'query-join')), await contract('join-summary'));
    assert.deepEqual(mapValue(await run(project, 'inspect-parameters')), { minimum: '6', region: 'west' });
    await open({ project });
    const copy = await clone(`Portable ${definition.name}`); assert.notEqual(copy, project);
    const editedNodes = originals['node-query'].replace('@stock >= 4', '@stock >= 6'); assert.notEqual(editedNodes, originals['node-query']);
    const editedFilter = originals['filter-query'].replace('@stock >= 4', '@stock >= 6'); assert.notEqual(editedFilter, originals['filter-query']);
    await open({ project: copy, file: 'filter-query' }); await save(editedFilter);
    assert.equal((await run(copy, 'format-filter')).text, editedFilter);
    await assertSourcePresentation(await run(copy, 'color-filter'), editedFilter);
    assert.deepEqual(values(await run(copy, 'query-filter')), allProducts.slice(0, 2));
    assert.deepEqual(values(await run(project, 'query-filter')), allProducts);
    await open({ project: copy, file: 'node-query' }); await save(editedNodes);
    assertNodes(await run(copy, 'query-nodes'), copy, parameterCatalog, ['cem:node:27', 'cem:node:36']);
    assertNodes(await run(project, 'query-nodes'), project, parameterCatalog, ['cem:node:27', 'cem:node:36', 'cem:node:45']);
    await open({ project: copy, file: 'node-query' }); await save(originals['invalid-query']);
    const refused = await run(copy, 'query-nodes');
    assert.equal(refused.status, 'projection-invalid', JSON.stringify(refused)); assert.equal(refused.exitCode, 1); assert.equal(refused.outputAvailable, false);
    assert.ok(refused.diagnostics?.some(item => item.code === expectedFailure.errorCode), JSON.stringify(refused));
    await open({ project: copy, file: 'node-query' }); await save(editedNodes);
    assertNodes(await run(copy, 'query-nodes'), copy, parameterCatalog, ['cem:node:27', 'cem:node:36']);
    console.log('[cem-studio:xpath-catalog] saved data/options, namespace join, independent query edits and invalid-query refusal passed');

    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assertNodes(await run(copy, 'query-nodes'), copy, parameterCatalog, ['cem:node:27', 'cem:node:36']);
        assert.deepEqual(values(await run(copy, 'query-parameters')), parameters);
        assert.deepEqual(values(await run(copy, 'query-join')), await contract('join-summary'));
        assert.deepEqual(mapValue(await run(copy, 'inspect-parameters')), { minimum: '6', region: 'west' });
        assertSourceStructure(await run(copy, 'parse-filter'), copy);
        assertSourceStructure(await run(copy, 'parse-filter-events'), copy, true);
        assertSourceStructure(await run(copy, 'inspect-filter'), copy);
        assert.equal((await run(copy, 'format-filter')).text, editedFilter);
        await assertSourcePresentation(await run(copy, 'color-filter'), editedFilter);
        await open({ project: copy, file: 'catalog-source' }); assert.equal(await editor.evaluate(element => element.value), parameterCatalog);
        await open({ project: copy, file: 'node-query' }); assert.equal(await editor.evaluate(element => element.value), editedNodes);
        await open({ project: copy, file: 'filter-query' }); assert.equal(await editor.evaluate(element => element.value), editedFilter);
        await open({ project: copy, file: repair.source }); assert.equal(await editor.evaluate(element => element.value), repaired);
        for (const [name, source] of Object.entries(originals)) {
            await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
            assert.equal(await readonly.evaluate(element => element.value), source);
        }
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:xpath-catalog] offline native source operations, values/nodes/maps and unchanged originals passed');
    return { catalog: true, readonlyOriginals: true, nativeCommands: true, exactValueContracts: true, nativeNodeIdentity: true,
        nativeSourceRanges: true, typedMapArray: true, namespaceQuery: true, nativeRepair: true, unsavedSourceGuard: true,
        savedData: true, savedParameterMap: true, independentCopies: true, invalidQueryRefusal: true, offline: true,
        typedSourceParse: true, typedSourceInspect: true, sourceFormatting: true, htmlSourceColoring: true, offlineSourceOperations: true };
}
