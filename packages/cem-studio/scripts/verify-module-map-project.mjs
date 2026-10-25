import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';

/** Real native deployment maps use the ordinary workspace and shared previews. */
export async function verifyModuleMapProject({ page, context, url }) {
    const id = 'module-maps';
    const definition = JSON.parse(await readFile(new URL(`../../../examples/projects/${id}/example.json`, import.meta.url), 'utf8'));
    const resource = name => definition.sources.find(item => item.id === name);
    const operation = name => definition.configurations.find(item => item.id === name);
    const fixture = name => readFile(new URL(`../../../${resource(name).source}`, import.meta.url));
    const hash = bytes => createHash('sha256').update(Buffer.from(bytes)).digest('hex');
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
            buildStatus: preview.attempt?.status, buildError: preview.attempt?.error?.message,
            buildDiagnostics: preview.attempt?.diagnostics, runId: preview.projection?.buildRun.id, entry: preview.entryArtifact,
            manifest: preview.projection?.nativeResult?.result?.value?.value?.value?.moduleAssetManifest,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, uri: item.uri, sha256: item.sha256, byteLength: item.byteLength, provenance: item.provenance })),
            outputs: preview.projection?.outputs.map(item => ({ path: preview.projection.buildRun.artifacts.find(artifact => artifact.uri === item.uri)?.path, text: item.text, bytes: Array.from(item.bytes) })) };
    });
    const output = (current, path) => { const found = current.outputs?.find(item => item.path === path); assert.ok(found, `Missing native output ${path}`); return found; };
    const save = async value => { await editor.fill(value); await settle(); await click('Save and validate'); };
    const edit = async (project, file, value, version = 3) => { await open({ project, file, build: `build-v${version}` }); await save(value); };
    const run = async (project, name, version = 3) => {
        const configuration = operation(name);
        await open({ project, configuration: name, build: `build-v${version}` }); assert.equal((await state()).editable, true);
        const button = configuration.kind === 'validation' ? 'Validate persisted revision' : 'Run configuration';
        if (!await page.getByRole('button', { name: button, exact: true }).isVisible()) {
            const label = configuration.kind === 'validation' ? 'Editor details' : 'Commands and results';
            await page.locator(`cem-expansion[label="${label}"] > [part~=root] [part~=header]`).first().click(); await settle();
        }
        await click(button); return state();
    };
    const build = async (project, version = 3) => { await open({ project, build: `build-v${version}` }); await click('Build project'); return state(); };
    const clone = async name => {
        await click('Clone selected'); await page.getByRole('dialog').getByRole('textbox').fill(name);
        await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
        const project = (await state()).route.project; assert.ok(project); return project;
    };
    const owned = (uri, project) => new URL(uri, `studio://${project}/`).href;
    const assertManifest = async (current, project, version, sources = {}, targets = {}) => {
        const configuration = operation(`build-v${version}`), manifest = current.manifest;
        assert.equal(current.buildStatus, 'succeeded', JSON.stringify(current)); assert.equal(current.entry, configuration.entryArtifact);
        assert.equal(manifest?.contractVersion, version === 3 ? 2 : 1); assert.equal(manifest.hashScheme, 'sha256');
        assert.equal(manifest.assetCount, configuration.expectedNativeAssets.length); assert.equal(manifest.assets.length, manifest.assetCount);
        if (!Object.keys(sources).length && !Object.keys(targets).length) assert.equal(manifest.hash, configuration.expectedNativeAssetHash);
        assert.ok(current.artifacts.every(item => item.provenance.length === 0));
        for (const expected of configuration.expectedNativeAssets) {
            const asset = manifest.assets.find(item => item.specifier === expected.specifier); assert.ok(asset);
            const target = targets[expected.specifier] ?? expected.target;
            const destination = targets[expected.specifier] ? `generated/v${version}/${target.slice(2)}` : expected.destination;
            assert.equal(asset.contentType, expected.contentType); assert.equal(asset.target, target);
            assert.equal(owned(asset.sourceUri, project), `studio://${project}/${resource(expected.source).path}`);
            assert.equal(owned(asset.sourceMapUri, project), `studio://${project}/${resource(expected.sourceMap).path}`);
            assert.equal(owned(asset.destination, project), `studio://${project}/${destination}`);
            const source = sources[expected.source] === undefined ? await fixture(expected.source) : Buffer.from(sources[expected.source]);
            const published = output(current, destination), artifact = current.artifacts.find(item => item.path === destination);
            assert.equal(asset.sourceByteLength, source.byteLength); assert.equal(asset.sourceSha256, hash(source));
            assert.equal(asset.byteLength, published.bytes.length); assert.equal(asset.sha256, hash(published.bytes));
            assert.equal(artifact.sha256, asset.sha256); assert.equal(artifact.byteLength, asset.byteLength);
            if (expected.source === expected.expectedOutput) assert.deepEqual(Buffer.from(published.bytes), source);
        }
    };
    const assertBaseline = async (current, project, version) => {
        const configuration = operation(`build-v${version}`); await assertManifest(current, project, version);
        assert.deepEqual(current.artifacts.map(item => item.path).sort(), Object.keys(configuration.expectedGraphOutputs).sort());
        for (const [path, expected] of Object.entries(configuration.expectedGraphOutputs)) assert.deepEqual(Buffer.from(output(current, path).bytes), await fixture(expected));
    };
    const rendered = async version => {
        await page.evaluate(path => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact(path), `generated/v${version}/index.html`); await settle();
        await page.waitForFunction(version => {
            const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument;
            return doc?.querySelector('h1')?.textContent === 'Module-map deployment' && doc.body.textContent.includes(`Version ${version}`);
        }, version);
        const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
        assert.equal(await frame.getAttribute('sandbox'), 'allow-same-origin');
        const content = await frame.evaluate(element => ({ scripts: element.contentDocument.querySelectorAll('script,iframe,object,embed').length,
            policy: element.contentDocument.querySelector('meta[http-equiv="Content-Security-Policy"]')?.getAttribute('content') }));
        assert.equal(content.scripts, 0); assert.ok(content.policy.includes("script-src 'none'"));
    };
    const jsonRecord = async (path, caption) => {
        await page.evaluate(path => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact(path), path); await settle();
        const view = page.locator('cem-data-view[format="json"]:visible'), structure = view.locator('[part~="structure"]').first();
        await structure.waitFor(); assert.equal(await structure.locator('[part~="property"]').count(), 2);
        const text = await structure.innerText(); assert.ok(text.includes(caption)); assert.match(text, /version:\s*3/);
        assert.equal(await view.getAttribute('data-cem-density'), 'super-compact');
    };
    const sampleRoute = file => ({ collection: 'content-types-schema-authoring', sample: id, ...(file ? { file } : {}), build: 'build-v3' });
    for (const collection of ['content-types-schema-authoring', 'content-types-queries-transformations']) {
        await page.goto(`${url}/?collection=${collection}`); await ready();
        const record = await page.evaluate(id => globalThis.__cemStudioApplication.workbench.snapshot().navigation.children.find(item => item.route.sample === id), id);
        assert.ok(record?.description);
        await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: record.name, exact: true }).click(); await settle();
        assert.equal((await state()).route.sample, id);
    }
    const originalIds = definition.sources.filter(item => !item.path.startsWith('expected/') && item.id !== 'readme').map(item => item.id), originals = {}, repaired = {};
    for (const name of originalIds) { await open(sampleRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); originals[name] = await readonly.evaluate(element => element.value); }
    await open(sampleRoute()); const project = await clone(`Navigator ${definition.name}`);
    for (const configuration of definition.configurations.filter(item => item.kind !== 'transformation-graph')) {
        const current = await run(project, configuration.id);
        if (!configuration.repair) {
            assert.equal(current.status, configuration.kind === 'validation' ? 'valid' : 'projected', JSON.stringify(current));
            for (const text of configuration.expectedText ?? []) assert.ok(current.text.includes(text), `${configuration.id}: ${text}`);
            if (configuration.expectedOutput) assert.equal(current.text, (await fixture(configuration.expectedOutput)).toString('utf8'));
        } else {
            const expected = configuration.expectedValidation, diagnostic = current.diagnostics.find(item => item.code === expected.errorCode);
            assert.ok(diagnostic); assert.equal(diagnostic.severity, 'error'); assert.equal(current.diagnostics.filter(item => item.severity === 'error').length, expected.errorCount);
            assert.equal(diagnostic.uri, `studio://${project}/${resource(configuration.source).path}`);
            for (const [key, value] of Object.entries(expected.location)) assert.equal(diagnostic[key] ?? null, value);
            repaired[configuration.source] = originals[configuration.source].replace(configuration.repair.find, configuration.repair.replace);
            assert.equal(repaired[configuration.source], originals[`source-v${configuration.id.slice(-1)}`]);
            await edit(project, configuration.source, repaired[configuration.source]); assert.equal((await run(project, configuration.id)).status, 'valid');
        }
    }
    const baselines = {};
    for (const version of [1, 2, 3]) { baselines[version] = await build(project, version); await assertBaseline(baselines[version], project, version); await rendered(version); }
    console.log('[cem-studio:module-maps] catalog, native syntax/source operations, exact repairs and all three deployment contracts passed');
    for (const version of [1, 2, 3]) {
        const good = await build(project, version); await assertBaseline(good, project, version);
        await edit(project, `source-v${version}`, originals[`invalid-contract-v${version}`], version); assert.equal((await run(project, `validate-v${version}`, version)).status, 'valid');
        const refused = await build(project, version); assert.equal(refused.buildStatus, 'failed'); assert.equal(refused.runId, good.runId);
        assert.ok(refused.buildError?.includes(operation(`build-v${version}`).expectedGraphRefusals[0].code), refused.buildError);
        assert.equal(refused.buildDiagnostics, undefined, 'Planning refusal must not acquire completed-command diagnostics');
        assert.deepEqual(refused.artifacts, good.artifacts);
        await edit(project, `source-v${version}`, originals[`source-v${version}`], version); await assertBaseline(await build(project, version), project, version);
    }
    const target = originals['target-v3'].replace('./assets/metadata.json', './assets/catalog.json'); assert.notEqual(target, originals['target-v3']); await edit(project, 'target-v3', target);
    const renamed = await build(project); await assertManifest(renamed, project, 3, {}, { '@demo/metadata': './assets/catalog.json' });
    assert.ok(output(renamed, 'generated/v3/assets/app.js').text.includes("from './catalog.json'")); assert.ok(output(renamed, 'generated/v3/index.html').text.includes('./assets/catalog.json'));
    const data = originals['module-metadata'].replace('Café modules', 'Saved modules'); assert.notEqual(data, originals['module-metadata']); await edit(project, 'module-metadata', data);
    const saved = await build(project); await assertManifest(saved, project, 3, { 'module-metadata': data }, { '@demo/metadata': './assets/catalog.json' });
    assert.equal(output(saved, 'generated/v3/assets/catalog.json').text, data); await jsonRecord('generated/v3/assets/catalog.json', 'Saved modules');
    console.log('[cem-studio:module-maps] semantic planning refusals, last-good artifacts, saved target rewrites and exact JSON resource edits passed');
    await open({ project, build: 'build-v3' }); const copy = await clone(`Portable ${definition.name}`);
    const copiedTarget = target.replace('./assets/catalog.json', './assets/catalog-copy.json'); await edit(copy, 'target-v3', copiedTarget);
    const copied = await build(copy); await assertManifest(copied, copy, 3, { 'module-metadata': data }, { '@demo/metadata': './assets/catalog-copy.json' });
    assert.ok(output(copied, 'generated/v3/assets/app.js').text.includes("from './catalog-copy.json'"));
    const unchanged = await build(project); assert.equal(unchanged.manifest.hash, saved.manifest.hash); assert.deepEqual(unchanged.outputs, saved.outputs);
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        for (const version of [1, 2]) await assertBaseline(await build(copy, version), copy, version);
        const offline = await build(copy); await assertManifest(offline, copy, 3, { 'module-metadata': data }, { '@demo/metadata': './assets/catalog-copy.json' });
        assert.deepEqual(offline.outputs, copied.outputs); await rendered(3); await jsonRecord('generated/v3/assets/catalog-copy.json', 'Saved modules');
        for (const [name, text] of Object.entries({ ...repaired, 'target-v3': copiedTarget, 'module-metadata': data })) { await open({ project: copy, file: name, build: 'build-v3' }); assert.equal(await editor.evaluate(element => element.value), text); }
        for (const version of [1, 2, 3]) assert.equal((await run(copy, `repair-v${version}`)).status, 'valid');
        for (const name of originalIds) { await open(sampleRoute(name)); assert.equal(await readonly.getAttribute('aria-readonly'), 'true'); assert.equal(await readonly.evaluate(element => element.value), originals[name]); }
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:module-maps] independent target maps, native manifest ownership/hashes, all-version offline builds and unchanged originals passed');
    return { catalog: true, readonlyOriginals: true, versionedSources: true, syntaxOnlyValidation: true, sourceFormatting: true,
        structuralInspection: true, nativeRepairs: true, nativeDeployment: true, exactOpaqueCopies: true, declaredEdgeRewrite: true,
        assetManifest: true, nativeSourceOwnership: true, nativeHashes: true, unmappedOutputBoundary: true, scriptFreePreview: true,
        semanticRefusals: true, retainedFailure: true, savedTargetMap: true, savedResource: true, structuredJson: true, independentCopies: true, offline: true };
}
