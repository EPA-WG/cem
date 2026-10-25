import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** The ordinary native selector project uses the same shared source controls. */
export async function verifySelectorProject({ page, context, url }) {
    const id = 'css-selector';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = async name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url), 'utf8');
    const expectedCards = JSON.parse(await fixture('selector-summary'));
    const expectedNamespace = JSON.parse(await fixture('namespace-summary'));
    const repair = operation('repair-selector');
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
    const values = current => {
        const report = JSON.parse(current.text); assert.equal(report.language, 'css-selector');
        return report.matches.map(({ localName, namespaceUri, documentOrder, nodeId }) => ({ localName, namespaceUri, documentOrder, nodeId }));
    };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const originalRoute = file => ({ collection: 'content-types-queries-transformations', sample: id, ...(file ? { file } : {}) });

    await page.goto(`${url}/?collection=content-types-queries-transformations`); await ready();
    const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
    assert.ok(record?.description);
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
    assert.equal((await state()).route.sample, id);
    const originals = {};
    for (const name of ['document-source', 'selector-source', 'invalid-selector']) {
        await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
        originals[name] = await readonly.evaluate(element => element.value);
    }
    await open(originalRoute());
    const project = await clone(`Navigator ${definition.name}`);
    for (const command of definition.configurations.filter(item => !item.repair)) {
        const result = await run(project, command.id);
        assert.equal(result.status, command.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(result));
        for (const text of command.expectedText ?? []) assert.ok(result.text.includes(text), `${command.id}: missing ${text}`);
        if (command.expectedSemanticContract) assert.deepEqual(values(result), JSON.parse(await fixture(command.expectedSemanticContract)));
    }
    const baseline = await run(project, 'query-cards'), report = JSON.parse(baseline.text);
    assert.deepEqual(values(baseline), expectedCards);
    const sourceBytes = new TextEncoder().encode(originals['document-source']);
    for (const [index, match] of report.matches.entries()) {
        const range = match.sourceMap.frames[0].span.ranges;
        assert.deepEqual(range, [{ start: 128, len: 62 }, { start: 369, len: 64 }][index]);
        const source = new TextDecoder('utf-8', { fatal: true }).decode(sourceBytes.slice(range.start, range.start + range.len));
        assert.ok(source.startsWith('<article ') && source.endsWith('>'), source);
    }
    assert.deepEqual(report.sourceMap.frames[0].span.ranges, { start: 0, len: new TextEncoder().encode(originals['selector-source']).byteLength });
    const failed = await run(project, repair.id), expectedFailure = repair.expectedValidation;
    const diagnostic = failed.diagnostics?.find(item => item.code === expectedFailure.errorCode);
    assert.ok(diagnostic, JSON.stringify(failed)); assert.equal(diagnostic.severity, 'error');
    assert.equal(diagnostic.uri, `studio://${project}/${resource(repair.source).path}`);
    for (const [key, value] of Object.entries(expectedFailure.location)) assert.equal(diagnostic[key] ?? null, value);
    await open({ project, file: repair.source });
    const repaired = originals[repair.source].replace(repair.repair.find, repair.repair.replace); assert.notEqual(repaired, originals[repair.source]);
    await save(repaired); assert.equal((await run(project, repair.id)).status, 'valid');
    console.log('[cem-studio:css-selector] catalog, native identities/source spans, both query contexts and exact repair passed');

    await run(project, 'query-cards');
    const editedDocument = originals['document-source'].replace('data-state="draft"', 'data-state="ready"'); assert.notEqual(editedDocument, originals['document-source']);
    await editor.fill(editedDocument); await settle(); await click('Run configuration');
    assert.deepEqual(values(await state()), expectedCards); assert.equal((await state()).dirty, true);
    await click('Save and validate'); await click('Run configuration');
    const editedCards = values(await state()); assert.equal(editedCards.length, 3);
    assert.deepEqual(editedCards.map(item => item.nodeId), ['html:event:13', 'html:event:22', 'html:event:31']);
    assert.deepEqual(values(await run(project, 'query-namespaces')), expectedNamespace);
    await open({ project });
    const copy = await clone(`Portable ${definition.name}`); assert.notEqual(copy, project);
    await open({ project: copy, file: 'selector-source' });
    const editedSelector = originals['selector-source'].replace('[data-state="ready"]', '[data-state="draft"]'); assert.notEqual(editedSelector, originals['selector-source']);
    await save(editedSelector);
    assert.deepEqual(values(await run(copy, 'query-cards')), expectedCards.slice(0, 1));
    assert.deepEqual(values(await run(project, 'query-cards')), editedCards);
    await open({ project: copy, file: 'selector-source' }); await save(originals['invalid-selector']);
    const refused = await run(copy, 'query-cards');
    assert.equal(refused.status, 'projection-invalid', JSON.stringify(refused)); assert.equal(refused.exitCode, 1); assert.equal(refused.outputAvailable, false);
    assert.ok(refused.diagnostics?.some(item => item.code === expectedFailure.errorCode), JSON.stringify(refused));
    await open({ project: copy, file: 'selector-source' }); await save(editedSelector);
    assert.deepEqual(values(await run(copy, 'query-cards')), expectedCards.slice(0, 1));
    console.log('[cem-studio:css-selector] saved document/selector changes, deduplication, invalid-query refusal and independent copies passed');

    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assert.deepEqual(values(await run(copy, 'query-cards')), expectedCards.slice(0, 1));
        assert.deepEqual(values(await run(copy, 'query-namespaces')), expectedNamespace);
        assert.equal((await run(copy, 'validate-selector')).status, 'valid');
        await open({ project: copy, file: 'document-source' }); assert.equal(await editor.evaluate(element => element.value), editedDocument);
        await open({ project: copy, file: 'selector-source' }); assert.equal(await editor.evaluate(element => element.value), editedSelector);
        await open({ project: copy, file: repair.source }); assert.equal(await editor.evaluate(element => element.value), repaired);
        for (const [name, source] of Object.entries(originals)) {
            await open(originalRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true');
            assert.equal(await readonly.evaluate(element => element.value), source);
        }
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:css-selector] offline native queries, repaired resource and unchanged read-only originals passed');
    return { catalog: true, readonlyOriginals: true, nativeCommands: true, exactIdentityContracts: true, nativeSourceRanges: true,
        namespaceQuery: true, nativeRepair: true, unsavedSourceGuard: true, savedDocument: true, savedSelector: true,
        deduplication: true, invalidQueryRefusal: true, independentCopies: true, offline: true };
}
