import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

/** Ordinary document projects, through shared controls and isolated saved copies. */
export async function verifyXmlXhtmlProjects({ page, context, url }) {
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const state = () => page.evaluate(() => {
        const snapshot = globalThis.__cemStudioApplication.workbench.snapshot(), preview = snapshot.projectPreview;
        return { route: snapshot.navigation.selection, content: snapshot.navigation.content, status: snapshot.status,
            diagnostics: snapshot.projection?.diagnostics ?? snapshot.validation?.diagnostics,
            text: snapshot.projection?.output?.text,
            buildStatus: preview.attempt?.status, buildDiagnostics: preview.attempt?.diagnostics,
            runId: preview.projection?.buildRun.id, entryArtifact: preview.entryArtifact,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            inputs: preview.projection?.buildRun.inputs.map(item => ({ resourceId: item.resourceId, path: item.path, uri: item.uri, sha256: item.sha256 })),
            origins: preview.projection?.buildRun.artifacts.flatMap(item => item.provenance.map(frame => frame.uri)) };
    });
    const open = async route => {
        await page.evaluate(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()), route);
        await settle();
    };
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const readonlySource = page.locator('cem-source-editor[readonly] [role=textbox]');
    const save = async text => { await editor.fill(text); await settle(); await click('Save and validate'); return state(); };
    const values = current => {
        const report = JSON.parse(current.text);
        return report.language === 'xpath' ? report.result.sequence.items.map(item => item.value.lexicalValue) : report.result.items.map(item => item.value);
    };
    const run = async (projectId, configuration, build, validation = false) => {
        await open({ project: projectId, configuration, build });
        const button = validation ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name: button, exact: true }).isVisible()) {
            const label = validation ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(button); return state();
    };
    const build = async (projectId, buildId) => {
        await open({ project: projectId, build: buildId });
        await click('Build project');
        return state();
    };
    const rendered = async text => {
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        await page.waitForFunction(text => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.body?.textContent?.includes(text), text);
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        return frame.contentFrame().locator('body').innerText();
    };
    const results = {};
    for (const sample of [
        { id: 'supplier-pipeline', collection: 'content-types-data', source: 'supplier-catalog', query: 'query-products', repair: 'repair-catalog',
            build: 'availability', output: 'generated/availability.html', find: 'Café cups', replacement: 'Harbor cups', close: '</catalog>', brokenClose: '</catalog-broken>' },
        { id: 'xhtml-document', collection: 'content-types-documents-ui', source: 'article-source', query: 'query-outline', repair: 'repair-namespace',
            build: 'build-html', output: 'generated/article.html', find: 'An XHTML field guide', replacement: 'Saved XHTML field guide', close: '</html>', brokenClose: '</html-broken>' },
    ]) {
        const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${sample.id}/example.json`, import.meta.url), 'utf8'));
        const query = definition.configurations.find(item => item.id === sample.query);
        const repair = definition.configurations.find(item => item.id === sample.repair);
        const expectedSource = definition.sources.find(item => item.id === query.expectedSemanticContract);
        const expected = JSON.parse(await readFile(new URL(`../../../${expectedSource.source}`, import.meta.url), 'utf8'));
        await page.goto(`${url}/?collection=${sample.collection}`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), sample.id);
        assert.ok(record?.description, `${sample.id} is discoverable in its ordinary catalog folder`);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, sample.id);
        await open({ collection: sample.collection, sample: sample.id, file: sample.source, build: sample.build });
        const original = await readonlySource.evaluate(element => element.value);
        assert.equal(await readonlySource.getAttribute('aria-readonly'), 'true'); assert.ok(original.includes(sample.find));
        await open({ collection: sample.collection, sample: sample.id, file: repair.source, build: sample.build });
        const invalidOriginal = await readonlySource.evaluate(element => element.value);
        await open({ collection: sample.collection, sample: sample.id, build: sample.build });
        await click('Build project');
        const baseline = await state(); assert.equal(baseline.buildStatus, 'succeeded', JSON.stringify(baseline));
        assert.equal(baseline.entryArtifact, sample.output);
        assert.ok(baseline.artifacts.some(item => item.path === sample.output));
        assert.ok((await rendered(sample.find)).includes(sample.find));
        await click('Clone selected');
        await page.getByRole('dialog').getByRole('textbox').fill(`Navigator ${record.name}`);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const projectId = (await state()).route.project;
        assert.ok(projectId); assert.equal((await state()).route.sample, undefined);
        assert.deepEqual(values(await run(projectId, sample.query, sample.build)), expected);
        const invalid = await run(projectId, sample.repair, sample.build, true);
        const diagnostic = invalid.diagnostics?.find(item => item.code === repair.expectedValidation.errorCode);
        const invalidResource = definition.sources.find(item => item.id === repair.source);
        assert.ok(diagnostic, JSON.stringify(invalid));
        assert.equal(diagnostic.severity, 'error'); assert.equal(diagnostic.uri, `studio://${projectId}/${invalidResource.path}`);
        for (const [key, value] of Object.entries(repair.expectedValidation.location ?? {})) assert.equal(diagnostic[key] ?? null, value);
        await open({ project: projectId, file: repair.source, build: sample.build });
        assert.equal(await editor.evaluate(element => element.value), invalidOriginal);
        const repairedSource = invalidOriginal.replace(repair.repair.find, repair.repair.replace);
        assert.notEqual(repairedSource, invalidOriginal); await save(repairedSource);
        const valid = await run(projectId, sample.repair, sample.build, true);
        assert.equal(valid.status, 'valid', JSON.stringify(valid));
        assert.ok(!valid.diagnostics?.some(item => item.severity === 'error'));
        await open({ project: projectId, file: sample.source, build: sample.build });
        assert.equal(await editor.evaluate(element => element.value), original);
        const editedSource = original.replace(sample.find, sample.replacement);
        assert.notEqual(editedSource, original); await save(editedSource);
        assert.deepEqual(values(await run(projectId, sample.query, sample.build)), expected.map(value => value.replace(sample.find, sample.replacement)));
        const edited = await build(projectId, sample.build);
        assert.equal(edited.buildStatus, 'succeeded', JSON.stringify(edited));
        assert.ok((await rendered(sample.replacement)).includes(sample.replacement));
        assert.ok(edited.origins.includes(`studio://${projectId}/templates/view.cemt`));
        const source = definition.sources.find(item => item.id === sample.source);
        assert.ok(edited.inputs.some(input => input.resourceId === source.id && input.path === source.path && input.uri === `studio://${projectId}/${source.path}`));
        await open({ project: projectId, file: sample.source, build: sample.build });
        await save(editedSource.replace(sample.close, sample.brokenClose));
        const failed = await build(projectId, sample.build);
        assert.equal(failed.buildStatus, 'failed'); assert.ok(failed.buildDiagnostics.some(item => item.severity === 'error'));
        assert.equal(failed.runId, edited.runId); assert.deepEqual(failed.artifacts, edited.artifacts);
        assert.ok((await rendered(sample.replacement)).includes(sample.replacement));
        await open({ project: projectId, file: sample.source, build: sample.build });
        await save(editedSource);
        const repaired = await build(projectId, sample.build);
        assert.equal(repaired.buildStatus, 'succeeded', JSON.stringify(repaired)); assert.deepEqual(repaired.artifacts, edited.artifacts);
        await open({ project: projectId, file: sample.source, build: sample.build });
        await page.evaluate(async () => { await navigator.serviceWorker.ready; });
        await context.setOffline(true);
        try {
            await page.reload(); await ready();
            assert.equal((await state()).route.project, projectId); assert.equal((await state()).route.file, sample.source);
            assert.equal(await editor.evaluate(element => element.value), editedSource);
            assert.equal((await state()).runId, undefined);
            const offline = await build(projectId, sample.build);
            assert.equal(offline.buildStatus, 'succeeded', JSON.stringify(offline));
            assert.deepEqual(offline.artifacts, repaired.artifacts); assert.deepEqual(offline.inputs, repaired.inputs); assert.deepEqual(offline.origins, repaired.origins);
            assert.ok((await rendered(sample.replacement)).includes(sample.replacement));
            assert.deepEqual(values(await run(projectId, sample.query, sample.build)), expected.map(value => value.replace(sample.find, sample.replacement)));
            assert.equal((await run(projectId, sample.repair, sample.build, true)).status, 'valid');
            await open({ project: projectId, file: repair.source, build: sample.build });
            assert.equal(await editor.evaluate(element => element.value), repairedSource);
            await open({ collection: sample.collection, sample: sample.id, file: sample.source, build: sample.build });
            assert.equal(await readonlySource.evaluate(element => element.value), original);
            await open({ collection: sample.collection, sample: sample.id, file: repair.source, build: sample.build });
            assert.equal(await readonlySource.evaluate(element => element.value), invalidOriginal);
        } finally { await context.setOffline(false); }
        results[sample.id] = { catalog: true, readonlyOriginal: true, independentClone: true, namespaceQuery: true, sourceValidationRepair: true,
            savedEdits: true, htmlPreview: true, failedBuildRetainsArtifacts: true, nativeInputIdentity: true, templateProvenance: true, offline: true };
        console.log(`[cem-studio:xml-xhtml] ${sample.id} namespace query, validation repair, saved HTML build and offline reopening passed`);
    }
    return results;
}
