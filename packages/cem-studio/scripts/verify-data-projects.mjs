import assert from 'node:assert/strict';

/** Native data workflows through the ordinary Studio navigator in an isolated profile. */
export async function verifyDataProjects({ page, context, url }) {
    const sampleId = 'json-yaml-report';
    const buildId = 'build-report';
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const state = () => page.evaluate(() => {
        const snapshot = globalThis.__cemStudioApplication.workbench.snapshot();
        const preview = snapshot.projectPreview;
        return { route: snapshot.navigation.selection, content: snapshot.navigation.content,
            status: preview.attempt?.status, diagnostics: preview.attempt?.diagnostics?.map(item => item.code),
            runId: preview.projection?.buildRun.id, entryArtifact: preview.entryArtifact, artifact: preview.artifactPath,
            inputs: preview.projection?.buildRun.inputs.map(item => ({ resourceId: item.resourceId, path: item.path, uri: item.uri, sha256: item.sha256 })),
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)),
            outputs: preview.projection?.outputs.map(item => ({ path: preview.projection.buildRun.artifacts.find(artifact => artifact.uri === item.uri)?.path,
                contentType: item.contentType, text: item.text })) };
    });
    const open = async route => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route);
        await settle();
    };
    const withProject = async operation => {
        const route = (await state()).route;
        const restore = route.file || route.folder || route.configuration || route.selection === 'output';
        if (restore) {
            await open(Object.fromEntries(['project', 'sample', 'collection', 'build'].filter(key => route[key]).map(key => [key, route[key]])));
            assert.equal((await state()).content, 'project');
        }
        try { return await operation(); }
        finally {
            if (restore) {
                const artifact = (await state()).artifact;
                await open({ ...Object.fromEntries(['project', 'sample', 'collection', 'file', 'folder', 'configuration', 'selection', 'output-folder', 'build', 'artifact']
                    .filter(key => Object.hasOwn(route, key)).map(key => [key, route[key]])), ...(artifact && route.selection !== 'output' ? { artifact } : {}) });
            }
        }
    };
    const click = async name => {
        const button = page.getByRole('button', { name, exact: true });
        const operation = async () => { await button.click(); await settle(); };
        if (await button.isVisible()) await operation(); else await withProject(operation);
    };
    const build = async () => {
        await click('Build project'); const current = await state();
        assert.equal(current.status, 'succeeded', JSON.stringify(current));
        return current;
    };
    const selectArtifact = async path => {
        await page.evaluate(path => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact(path), path); await settle();
    };
    const output = (current, path) => {
        const result = current.outputs?.find(item => item.path === path);
        assert.ok(result, `Missing native output ${path}`);
        return result.text;
    };
    const reportReady = async title => {
        await page.waitForFunction(title => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('h1')?.textContent === title, title);
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        return frame.contentFrame().locator('body').innerText();
    };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); };
    const readonlySource = page.locator('cem-source-editor[readonly] [role=textbox]');

    await page.goto(`${url}/?collection=content-types-data`); await ready();
    const sample = await page.evaluate(sampleId => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === sampleId), sampleId);
    assert.ok(sample?.description, 'The Data folder exposes the ordinary JSON/YAML project with a description');
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: sample.name, exact: true }).click(); await settle();
    assert.equal((await state()).route.sample, sampleId);
    assert.equal(new URL(page.url()).searchParams.get('collection'), 'content-types-data');
    await open({ collection: 'content-types-data', sample: sampleId, build: buildId });
    const baseline = await build();
    assert.equal(baseline.entryArtifact, 'generated/report.html');
    assert.deepEqual(baseline.artifacts.map(item => item.path).sort(), ['generated/report.html', 'generated/report.json', 'generated/report.yaml']);
    assert.match(await reportReady('Active service report'), /API-101/);
    await selectArtifact('generated/report.json');
    const table = page.locator('cem-data-view[format="json"] table');
    await table.waitFor();
    assert.equal(await table.locator('tbody tr').count(), 2);
    assert.match(await table.innerText(), /API-101/); assert.match(await table.innerText(), /API-103/);
    assert.doesNotMatch(await table.innerText(), /API-102/);
    assert.equal(await page.locator('cem-data-view[format="json"]').getAttribute('data-cem-density'), 'super-compact');

    await open({ collection: 'content-types-data', sample: sampleId, file: 'api-payload', build: buildId });
    const originalPayload = await readonlySource.evaluate(element => element.value);
    assert.equal(await readonlySource.getAttribute('aria-readonly'), 'true');
    assert.match(originalPayload, /API-101/);
    await open({ collection: 'content-types-data', sample: sampleId, file: 'report-settings', build: buildId });
    const originalSettings = await readonlySource.evaluate(element => element.value);
    assert.match(originalSettings, /title: Active service report/);
    await open({ collection: 'content-types-data', sample: sampleId, build: buildId });
    await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Navigator JSON and YAML report');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const projectId = (await state()).route.project;
    assert.ok(projectId); assert.equal((await state()).route.sample, undefined);
    await open({ project: projectId, file: 'report-settings', build: buildId });
    assert.equal(await editor.evaluate(element => element.value), originalSettings);
    const editedSettings = originalSettings.replace('title: Active service report', 'title: Paused service report').replace('status: active', 'status: paused');
    assert.notEqual(editedSettings, originalSettings);
    await save(editedSettings);
    const edited = await build();
    assert.equal(edited.route.file, 'report-settings');
    await selectArtifact('generated/report.json'); await table.waitFor();
    assert.equal(await table.locator('tbody tr').count(), 1);
    assert.match(await table.innerText(), /API-102/); assert.doesNotMatch(await table.innerText(), /API-101/);
    await selectArtifact('generated/report.html');
    assert.match(await reportReady('Paused service report'), /API-102/);
    const successful = await state();
    await save(editedSettings.replace('title: Paused service report', 'title: [broken'));
    await click('Build project');
    const failed = await state();
    assert.equal(failed.status, 'failed'); assert.ok(failed.diagnostics.includes('cem.yaml.parse_error'));
    assert.equal(failed.runId, successful.runId); assert.deepEqual(failed.artifacts, successful.artifacts);
    assert.match(await reportReady('Paused service report'), /API-102/);
    await save(originalSettings); const repaired = await build();
    assert.equal(repaired.route.file, 'report-settings');
    await selectArtifact('generated/report.html');
    assert.match(await reportReady('Active service report'), /API-101/);
    assert.deepEqual(repaired.artifacts, baseline.artifacts);
    assert.ok(repaired.origins.includes(`studio://${projectId}/templates/view.cemt`), 'Native HTML output retains its template origin');
    for (const [resourceId, path] of [['api-payload', 'data/payload.json'], ['report-settings', 'data/settings.yaml'],
        ['report-model', 'templates/model.cemt'], ['report-view', 'templates/view.cemt'], ['report-graph', 'graphs/report.cem']]) {
        assert.ok(repaired.inputs.some(input => input.resourceId === resourceId && input.path === path && input.uri === `studio://${projectId}/${path}`),
            `Captured build dependency is missing ${path}`);
    }
    assert.deepEqual(JSON.parse(output(repaired, 'generated/report.json')), JSON.parse(output(baseline, 'generated/report.json')));
    console.log('[cem-studio:data-projects] readonly sample, shared previews, saved YAML edits and native failure/repair passed');

    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        assert.equal((await state()).route.project, projectId); assert.equal((await state()).route.file, 'report-settings');
        assert.equal(await editor.evaluate(element => element.value), originalSettings);
        assert.equal((await state()).runId, undefined);
        const offline = await build();
        assert.deepEqual(offline.artifacts, repaired.artifacts);
        assert.deepEqual(offline.origins, repaired.origins);
        assert.deepEqual(offline.inputs, repaired.inputs);
        await selectArtifact('generated/report.json'); await table.waitFor();
        assert.equal(await table.locator('tbody tr').count(), 2);
        await selectArtifact('generated/report.html');
        assert.match(await reportReady('Active service report'), /API-103/);
        await open({ collection: 'content-types-data', sample: sampleId, file: 'report-settings', build: buildId });
        assert.equal(await readonlySource.evaluate(element => element.value), originalSettings, 'Saved clone edits never alter the sample configuration');
        await open({ collection: 'content-types-data', sample: sampleId, file: 'api-payload', build: buildId });
        assert.equal(await readonlySource.evaluate(element => element.value), originalPayload, 'Saved clone edits never alter the sample payload');
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:data-projects] offline catalog, saved source reload and native rebuild passed');
    return { catalog: true, readonlyOriginal: true, independentClone: true, jsonTable: true, htmlReport: true,
        yamlEdits: true, failureRepair: true, retainedSuccessfulArtifacts: true, nativeProvenance: true, offline: true };
}
