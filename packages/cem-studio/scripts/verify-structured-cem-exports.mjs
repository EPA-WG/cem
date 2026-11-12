import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

/** Exercise ordinary project controls against the freshly assembled native runtime. */
export async function verifyStructuredCemExports({ page, context, url }) {
    const workspace = resolve(import.meta.dirname, '../../..');
    const digest = bytes => createHash('sha256').update(bytes).digest('hex');
    const map = JSON.parse(await readFile(resolve(workspace, 'packages/cem-studio/runtime/destination.module-map.json'), 'utf8'));
    const runtimeUrl = new URL(map.resources['@epa-wg/cem-ml/wasm-binary'].path, `${url}/`).href;
    const deployed = await context.request.get(runtimeUrl);
    assert.ok(deployed.ok());
    const runtimeSha256 = digest(await deployed.body());
    assert.equal(runtimeSha256, digest(await readFile(resolve(workspace, 'packages/cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm'))));
    await context.grantPermissions(['clipboard-read', 'clipboard-write']);
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView); await settle(); };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const state = () => page.evaluate(() => {
        const current = globalThis.__cemStudioApplication.workbench.snapshot(), result = current.projection;
        return { route: current.navigation.selection, resourceId: current.resourceId, entryId: current.entryId,
            status: current.status, dirty: current.dirty, text: result?.output?.text,
            outputType: result?.output?.contentType, available: result?.output?.available,
            sha256: result?.output?.sha256, provenance: result?.provenance,
            artifacts: result?.buildRun.artifacts.map(item => ({ uri: item.uri, sha256: item.sha256 })),
            inputs: result?.buildRun.inputs.map(item => ({ resourceId: item.resourceId, uri: item.uri, sha256: item.sha256 })),
            diagnostics: result?.diagnostics ?? current.validation?.diagnostics };
    });
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const copy = page.getByRole('button', { name: 'Copy exact output', exact: true });
    const download = page.getByRole('button', { name: 'Download exact output', exact: true });
    const results = async () => {
        const tab = page.locator('cem-tabs[label="Workbench panes"]').getByRole('tab', { name: 'Results', exact: true });
        if (!await tab.isVisible()) {
            await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await tab.click(); await settle();
    };
    const run = async () => {
        const button = page.getByRole('button', { name: 'Run configuration', exact: true });
        if (!await button.isVisible()) {
            await page.locator('cem-expansion[label="Commands and results"] > [part~=root] [part~=header]').first().click(); await settle();
        }
        await click('Run configuration'); return state();
    };
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); };
    const cases = [];
    for (const [id, configurationId, sourceId] of [
        ['json-yaml-report', 'json-to-cem', 'api-payload'],
        ['json-schema', 'schema-to-cem', 'note-schema'],
    ]) {
        const definition = JSON.parse(await readFile(resolve(workspace, `examples/projects/${id}/example.json`), 'utf8'));
        const configuration = definition.configurations.find(item => item.id === configurationId);
        const resource = definition.sources.find(item => item.id === sourceId);
        const original = await readFile(resolve(workspace, resource.source), 'utf8');
        await page.goto(`${url}/?collection=${definition.collections[0]}`); await ready();
        await page.getByRole('list', { name: 'Folder contents', exact: true }).getByRole('button', { name: definition.name, exact: true }).click(); await settle();
        await click('Clone selected');
        await page.getByRole('dialog').getByRole('textbox').fill(`UI ${configuration.name}`);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const projectId = (await state()).route.project; assert.ok(projectId);
        const files = page.getByRole('tree', { name: 'Project files', exact: true });
        await files.getByRole('treeitem', { name: 'Configurations', exact: true }).click(); await settle();
        await page.getByRole('list', { name: 'Folder contents', exact: true }).getByRole('button', { name: configuration.name, exact: true }).click(); await settle();
        assert.equal((await state()).entryId, configurationId);
        assert.equal((await state()).resourceId, sourceId);
        assert.equal(await editor.evaluate(element => element.value), original);
        const sourceUri = `studio://${projectId}/${resource.path}`;
        const assertResult = async (result, source, status = 'projected') => {
            assert.equal(result.status, status, JSON.stringify(result));
            assert.equal(result.outputType, 'application/cem'); assert.notEqual(result.available, false);
            assert.ok(result.text.startsWith('@doc cem-ml 1\n')); assert.ok(!result.text.includes('\x1b'));
            assert.ok(result.provenance.some(frame => frame.uri === sourceUri && frame.range?.len > 0));
            assert.ok(result.inputs.some(item => item.resourceId === sourceId && item.uri === sourceUri && item.sha256 === digest(source)));
            assert.equal(result.sha256, digest(result.text));
            assert.equal(await page.locator('[data-cem-studio-operation-preview] cem-media-preview[mode="text"] pre').textContent(), result.text);
        };
        const baseline = await run(); await assertResult(baseline, original);
        for (const expected of configuration.expectedText) assert.ok(baseline.text.includes(expected), expected);
        const native = spawnSync(resolve(workspace, 'dist/target/cem_ml_cli/debug/cem-ml'),
            configuration.argv.map(argument => argument === '$source' ? resolve(workspace, resource.source) : argument), { encoding: 'utf8', cwd: workspace });
        assert.equal(native.status, 0, native.stderr); assert.equal(baseline.text, native.stdout);
        await results(); await copy.click(); await settle();
        assert.equal(await page.evaluate(() => navigator.clipboard.readText()), baseline.text);
        const pending = page.waitForEvent('download'); await download.click();
        const file = await pending; assert.equal(await readFile(await file.path(), 'utf8'), baseline.text); await file.delete();

        const changed = JSON.parse(original), literal = 'Café {$never()} {#never} 🧬';
        if (id === 'json-yaml-report') changed.records[0].name = literal; else changed.title = literal;
        const edited = JSON.stringify(changed, null, 2) + '\n';
        await editor.fill(edited); await settle();
        const unsaved = await run(); await assertResult(unsaved, original, 'projection-stale'); assert.equal(unsaved.text, baseline.text);
        await click('Save and validate');
        const saved = await run(); await assertResult(saved, edited); assert.ok(saved.text.includes(literal));
        assert.notEqual(saved.sha256, baseline.sha256);
        await save(id === 'json-yaml-report' ? '{"records":[1,}' : edited.replace('https://json-schema.org/draft/2020-12/schema', 'urn:unsupported'));
        const refused = await run(); assert.equal(refused.status, 'projection-invalid'); assert.equal(refused.available, false);
        await results(); assert.deepEqual(refused.artifacts, []); assert.equal(await copy.isDisabled(), true); assert.equal(await download.isDisabled(), true);
        const errorCode = id === 'json-yaml-report' ? 'cem.json.parse_error' : 'cem.json_schema.unsupported_dialect';
        assert.ok(refused.diagnostics.some(item => item.code === errorCode && item.uri === sourceUri));
        await save(edited); const repaired = await run(); await assertResult(repaired, edited); assert.equal(repaired.sha256, saved.sha256);

        await page.evaluate(() => navigator.serviceWorker.ready); await page.waitForFunction(() => navigator.serviceWorker.controller);
        await context.setOffline(true);
        try {
            await page.reload(); await ready(); assert.equal((await state()).route.project, projectId);
            assert.equal(await editor.evaluate(element => element.value), edited);
            const offline = await run(); await assertResult(offline, edited); assert.equal(offline.sha256, saved.sha256);
            await page.goto(`${url}/?sample=${id}&file=${sourceId}`); await ready();
            assert.equal(await page.locator('cem-studio-project-picker cem-source-editor[readonly] [role=textbox]').evaluate(element => element.value), original);
        } finally { await context.setOffline(false); }
        cases.push({ sample: id, projectId, configurationId, resourceId: sourceId, sourceUri,
            sourceContentType: resource.contentType, sourceSchema: resource.schema,
            originalSha256: baseline.sha256, editedSha256: saved.sha256,
            nativeParity: true, exactCopyAndDownload: true, originalProvenance: true,
            persistedRevision: true, failureCode: errorCode, repair: true, offline: true, sampleUnchanged: true });
        console.log(`[cem-studio:${id}] CEM export UI, native parity, edits, refusal/repair and offline passed`);
    }
    return { runtimeSha256, cases };
}
