import assert from 'node:assert/strict';

/** CSV source policies and reports use the same ordinary navigator as saved projects. */
export async function verifyCsvInventory({ page, context, url }) {
    const sampleId = 'csv-inventory', buildId = 'build-inventory', sourceId = 'inventory-source';
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const state = () => page.evaluate(() => {
        const snapshot = globalThis.__cemStudioApplication.workbench.snapshot(), preview = snapshot.projectPreview;
        return { route: snapshot.navigation.selection, content: snapshot.navigation.content, status: snapshot.status,
            validation: snapshot.validation, commandDiagnostics: snapshot.projection?.diagnostics,
            buildStatus: preview.attempt?.status, buildDiagnostics: preview.attempt?.diagnostics,
            runId: preview.projection?.buildRun.id, entryArtifact: preview.entryArtifact, artifact: preview.artifactPath,
            inputs: preview.projection?.buildRun.inputs.map(item => ({ resourceId: item.resourceId, path: item.path, uri: item.uri, sha256: item.sha256 })),
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)) };
    });
    const open = async route => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route);
        await settle();
    };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const build = async () => {
        const route = (await state()).route;
        const restore = Boolean(route.file || route.configuration);
        if (restore) await open({ project: route.project, build: buildId });
        await click('Build project');
        if (restore) await open({ project: route.project, ...(route.file ? { file: route.file } : { configuration: route.configuration }), build: buildId,
            artifact: (await state()).artifact ?? 'generated/low-stock.html' });
        return state();
    };
    const selectArtifact = async path => {
        await page.evaluate(path => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact(path), path); await settle();
    };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonlySource = page.locator('cem-source-editor[readonly] [role=textbox]');
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); return state(); };
    const reportReady = async () => {
        await page.waitForFunction(() => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('h1')?.textContent === 'Low-stock inventory');
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        return frame.contentFrame().locator('body').innerText();
    };
    const runConfiguration = async (projectId, configuration) => {
        await open({ project: projectId, configuration, build: buildId });
        const run = page.getByRole('button', { name: 'Run configuration', exact: true });
        if (!await run.isVisible()) {
            await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await click('Run configuration'); return state();
    };
    const diagnostic = (diagnostics, code, severity, projectId) => {
        const item = diagnostics?.find(item => item.code === code);
        assert.ok(item, `Missing native diagnostic ${code}: ${JSON.stringify(diagnostics)}`);
        assert.equal(item.severity, severity);
        assert.equal(item.uri, `studio://${projectId}/data/inventory.csv`);
        if (code === 'cem.csv.invalid_header_parameter') {
            assert.equal(item.line == null && item.column == null && item.byteOffset == null, true, 'Metadata warnings do not invent source coordinates');
        } else {
            assert.ok(item.line > 0); assert.ok(item.byteOffset >= 0);
            if (severity === 'error') {
                const expected = code === 'cem.csv.unclosed_quote' ? { line: 2, column: 21, byteOffset: 52 } : { line: 2, column: 11, byteOffset: 42 };
                assert.deepEqual({ line: item.line, column: item.column, byteOffset: item.byteOffset }, expected);
            } else assert.equal(item.column == null, true, 'The native row warning has a line/byte location without a fabricated column');
        }
        return item;
    };

    await page.goto(`${url}/?collection=content-types-data`); await ready();
    const sample = await page.evaluate(sampleId => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === sampleId), sampleId);
    assert.ok(sample?.description, 'The Data folder exposes the ordinary CSV inventory project');
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: sample.name, exact: true }).click(); await settle();
    assert.equal((await state()).route.sample, sampleId);
    await open({ collection: 'content-types-data', sample: sampleId, file: sourceId, build: buildId });
    assert.equal(await readonlySource.getAttribute('aria-readonly'), 'true');
    const original = await readonlySource.evaluate(element => element.value);
    assert.match(original, /"Café cups, set"/); assert.match(original, /"Tea ""travel"" pot"/);
    const table = page.locator('cem-data-view[format="csv"] table');
    const reportTable = page.locator('cem-data-view[format="json"] table');
    await table.waitFor();
    assert.equal(await table.locator('tbody tr').count(), 4);
    assert.match(await table.innerText(), /Café cups, set/); assert.match(await table.innerText(), /Tea "travel" pot/);
    assert.equal(await page.locator('cem-data-view[format="csv"]').getAttribute('data-cem-density'), 'super-compact');
    const invalidSources = {};
    for (const id of ['unclosed-quote', 'invalid-escape', 'ragged-rows']) {
        await open({ collection: 'content-types-data', sample: sampleId, file: id, build: buildId });
        invalidSources[id] = await readonlySource.evaluate(element => element.value);
    }
    await open({ collection: 'content-types-data', sample: sampleId, build: buildId });
    await click('Build project');
    const baseline = await state(); assert.equal(baseline.buildStatus, 'succeeded', JSON.stringify(baseline));
    assert.equal(baseline.entryArtifact, 'generated/low-stock.html');
    assert.deepEqual(baseline.artifacts.map(item => item.path).sort(), ['generated/low-stock.html', 'generated/low-stock.json']);
    assert.match(await reportReady(), /SKU-A/);
    await selectArtifact('generated/low-stock.json'); await reportTable.waitFor();
    assert.equal(await reportTable.locator('tbody tr').count(), 2);
    assert.match(await reportTable.innerText(), /SKU-C/); assert.doesNotMatch(await reportTable.innerText(), /SKU-B/);
    await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Navigator CSV inventory');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const projectId = (await state()).route.project;
    assert.ok(projectId); assert.equal((await state()).route.sample, undefined);

    const headerWarning = await runConfiguration(projectId, 'repair-header-policy');
    diagnostic(headerWarning.commandDiagnostics ?? headerWarning.validation?.diagnostics, 'cem.csv.invalid_header_parameter', 'warning', projectId);
    assert.ok(['projected', 'valid'].includes(headerWarning.status), JSON.stringify(headerWarning));
    await open({ project: projectId, file: 'invalid-header-policy', build: buildId });
    const policy = await editor.evaluate(element => element.value);
    assert.match(policy, /header=maybe/);
    await save(policy.replace('header=maybe', 'header=present'));
    const repairedPolicy = await runConfiguration(projectId, 'repair-header-policy');
    assert.ok(['projected', 'valid'].includes(repairedPolicy.status), JSON.stringify(repairedPolicy));
    assert.ok(!(repairedPolicy.commandDiagnostics ?? repairedPolicy.validation?.diagnostics ?? []).some(item => item.code === 'cem.csv.invalid_header_parameter'));

    await open({ project: projectId, file: sourceId, build: buildId });
    assert.equal(await editor.evaluate(element => element.value), original);
    const editedText = original.replace('SKU-A,"Café cups, set",3,5', 'SKU-A,"Café cups, set",8,5');
    assert.notEqual(editedText, original);
    await save(editedText);
    const edited = await build(); assert.equal(edited.buildStatus, 'succeeded', JSON.stringify(edited));
    assert.equal(edited.route.file, sourceId);
    await selectArtifact('generated/low-stock.json'); await reportTable.waitFor();
    assert.equal(await reportTable.locator('tbody tr').count(), 1);
    assert.match(await reportTable.innerText(), /SKU-C/); assert.doesNotMatch(await reportTable.innerText(), /SKU-A/);
    await selectArtifact('generated/low-stock.html');
    assert.doesNotMatch(await reportReady(), /SKU-A/);
    const successful = await state();
    for (const [text, code] of [[invalidSources['unclosed-quote'], 'cem.csv.unclosed_quote'],
        [invalidSources['invalid-escape'], 'cem.csv.invalid_quote_escape']]) {
        const invalid = await save(text);
        diagnostic(invalid.validation?.diagnostics, code, 'error', projectId);
        const failed = await build();
        assert.equal(failed.buildStatus, 'failed'); diagnostic(failed.buildDiagnostics, code, 'error', projectId);
        assert.equal(failed.runId, successful.runId); assert.deepEqual(failed.artifacts, successful.artifacts);
        assert.doesNotMatch(await reportReady(), /SKU-A/);
        await save(editedText);
    }
    const ragged = await save(invalidSources['ragged-rows']);
    diagnostic(ragged.validation?.diagnostics, 'cem.csv.inconsistent_field_count', 'warning', projectId);
    assert.equal(ragged.validation.hardViolationCount, 0);
    await save(editedText);
    const repaired = await build(); assert.equal(repaired.buildStatus, 'succeeded', JSON.stringify(repaired));
    assert.deepEqual(repaired.artifacts, successful.artifacts);
    assert.ok(repaired.origins.includes(`studio://${projectId}/templates/view.cemt`), 'Native HTML output retains its template origin');
    for (const [resourceId, path] of [[sourceId, 'data/inventory.csv'], ['inventory-model', 'templates/model.cemt'],
        ['inventory-view', 'templates/view.cemt'], ['inventory-graph', 'graphs/inventory.cem']]) {
        assert.ok(repaired.inputs.some(input => input.resourceId === resourceId && input.path === path && input.uri === `studio://${projectId}/${path}`),
            `Captured CSV build dependency is missing ${path}`);
    }
    console.log('[cem-studio:csv-inventory] shared previews, saved edits, native header/width warnings and quote failure/repair passed');

    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assert.equal((await state()).route.project, projectId); assert.equal((await state()).route.file, sourceId);
        assert.equal(await editor.evaluate(element => element.value), editedText);
        assert.equal((await state()).runId, undefined);
        const offline = await build(); assert.equal(offline.buildStatus, 'succeeded', JSON.stringify(offline));
        assert.deepEqual(offline.artifacts, repaired.artifacts); assert.deepEqual(offline.inputs, repaired.inputs); assert.deepEqual(offline.origins, repaired.origins);
        await selectArtifact('generated/low-stock.json'); await reportTable.waitFor();
        assert.equal(await reportTable.locator('tbody tr').count(), 1);
        await open({ project: projectId, file: 'invalid-header-policy', build: buildId });
        assert.equal(await editor.evaluate(element => element.value), policy.replace('header=maybe', 'header=present'),
            'The saved header policy repair survives an offline reload');
        const offlinePolicy = await runConfiguration(projectId, 'repair-header-policy');
        assert.ok(['projected', 'valid'].includes(offlinePolicy.status), JSON.stringify(offlinePolicy));
        assert.ok(!(offlinePolicy.commandDiagnostics ?? offlinePolicy.validation?.diagnostics ?? []).some(item => item.code === 'cem.csv.invalid_header_parameter'));
        await open({ collection: 'content-types-data', sample: sampleId, file: sourceId, build: buildId });
        assert.equal(await readonlySource.evaluate(element => element.value), original, 'Saved CSV edits never alter the sample inventory');
        await open({ collection: 'content-types-data', sample: sampleId, file: 'invalid-header-policy', build: buildId });
        assert.match(await readonlySource.evaluate(element => element.value), /header=maybe/, 'Repairing a saved policy never changes its original');
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:csv-inventory] offline catalog, saved CSV and repaired policy reload and native rebuild passed');
    return { catalog: true, readonlyOriginal: true, independentClone: true, compactCsvTable: true, quotedCells: true, htmlReport: true,
        savedEdits: true, headerPolicyRepair: true, rowWidthWarningRepair: true, quoteFailureRepair: true, retainedSuccessfulArtifacts: true,
        nativeCoordinates: true, nativeProvenance: true, offline: true };
}
