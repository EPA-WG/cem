import assert from 'node:assert/strict';

/** Ordinary navigator workflow in the delivery gate's isolated browser profile. */
export async function verifyMarkdownSite({ page, context, url }) {
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const withProject = async operation => {
        const route = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.selection);
        const restore = route.file || route.folder || route.configuration || route.selection === 'output';
        if (restore) {
            await page.evaluate(async route => { const selection = Object.fromEntries(['project', 'sample', 'build'].filter(key => route[key]).map(key => [key, route[key]]));
                await globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(selection).toString()); }, route);
            await settle();
            assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.content), 'project', 'Project controls require admitted navigation; source drafts must remain guarded.');
        }
        try { return await operation(); }
        finally {
            if (restore) {
                await page.evaluate(async route => {
                    const application = globalThis.__cemStudioApplication;
                    const selected = Object.fromEntries(['project', 'sample', 'file', 'folder', 'configuration', 'selection', 'output-folder', 'build', 'artifact'].filter(key => Object.hasOwn(route, key)).map(key => [key, route[key]]));
                    const artifact = application.workbench.snapshot().projectPreview.artifactPath;
                    if (artifact && route.selection !== 'output') selected.artifact = artifact;
                    await application.workbench.openEntity(new URLSearchParams(selected).toString());
                }, route); await settle();
            }
        }
    };
    const click = async name => {
        const button = page.getByRole('button', { name, exact: true });
        const operation = async () => { await button.click(); await settle(); };
        if (await button.isVisible()) await operation(); else await withProject(operation);
    };
    const state = () => page.evaluate(() => {
        const snapshot = globalThis.__cemStudioApplication.workbench.snapshot();
        const preview = snapshot.projectPreview;
        return { projectId: snapshot.navigation.selection.project, file: snapshot.navigation.selection.file,
            comparison: preview.comparison, pinId: preview.pinId, pinError: preview.pinError, pins: preview.pins, status: preview.attempt?.status, entryArtifact: preview.entryArtifact, entryError: preview.entryError?.message, diagnostics: preview.attempt?.diagnostics?.map(item => item.code),
            runId: preview.projection?.buildRun.id, artifact: preview.artifactPath,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            html: preview.projection?.output.text };
    });
    const build = async () => { await click('Build project'); const current = await state(); assert.equal(current.status, 'succeeded', JSON.stringify(current)); return current; };
    const choosePin = async name => withProject(async () => { await page.getByRole('combobox', { name: 'Preview build', exact: true }).click(); await page.getByRole('option', { name, exact: true }).click(); await settle(); });
    const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
    const viewReady = async title => {
        await page.waitForFunction(title => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('h1')?.textContent === title, title);
    };
    const expandGeneratedOutput = async () => {
        const generated = page.getByRole('tree', { name: 'Output files', exact: true }).getByRole('treeitem', { name: 'generated', exact: true });
        if (await generated.getAttribute('aria-expanded') === 'false') { await generated.locator('[part~=marker]').click(); await settle(); }
    };
    const save = async text => { await page.getByRole('textbox', { name: 'CEM source', exact: true }).fill(text); await settle(); await click('Save and validate'); };
    const open = async (project, file) => {
        await page.evaluate(async ({ project, file }) => {
            await globalThis.__cemStudioApplication.workbench.openEntity(`project=${project}&file=${file}`);
        }, { project, file }); await settle();
    };
    await page.goto(`${url}/?sample=markdown-site&build=build-site`); await ready();
    const sample = await build();
    console.log('[cem-studio:markdown-site] catalog build ready');
    assert.equal(sample.artifacts.length, 7); await viewReady('CEM field notes');
    await expandGeneratedOutput();
    await page.screenshot({ path: '/tmp/cem-studio-compact-project-build.png', fullPage: true });
    await frame.contentFrame().getByRole('link', { name: 'a small build', exact: true }).click(); await settle();
    await viewReady('Build a small site');
    assert.equal((await state()).artifact, 'generated/articles/first-build.html');
    await page.waitForFunction(() => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('img')?.naturalWidth === 480);
    assert.equal(await frame.evaluate(frame => frame.contentWindow.getComputedStyle(frame.contentDocument.querySelector('nav')).display), 'flex');
    await page.emulateMedia({ forcedColors: 'active' });
    await frame.contentFrame().getByRole('navigation', { name: 'Site navigation' }).getByRole('link', { name: 'Home', exact: true }).focus();
    await page.keyboard.press('Tab');
    const colors = await frame.evaluate(frame => {
        const document = frame.contentDocument, style = frame.contentWindow.getComputedStyle(document.body);
        return { foreground: style.color, background: style.backgroundColor,
            focusWidth: frame.contentWindow.getComputedStyle(document.activeElement).outlineWidth };
    });
    assert.notEqual(colors.foreground, colors.background); assert.equal(colors.focusWidth, '2px');
    await page.emulateMedia({ forcedColors: 'none' });
    await click('Clone selected');
    await page.getByRole('dialog').getByRole('textbox').fill('Navigator Markdown site');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    const projectId = (await state()).projectId;
    assert.ok(projectId);
    await click('Build entry settings');
    const setEntry = async value => { await page.getByRole('textbox', { name: 'Build entry artifact', exact: true }).fill(value); await click('Save build entry'); };
    await setEntry('../outside.html');
    assert.ok((await state()).entryError.includes('entryArtifact'));
    assert.equal((await state()).entryArtifact, 'generated/index.html');
    await setEntry('generated/articles/first-build.html'); await build(); await viewReady('Build a small site');
    await setEntry('generated/index.html');
    const article = 'content-articles-first-build-md';
    await open(projectId, article);
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const original = await editor.evaluate(element => element.value);
    await save(original.replaceAll('Build a small site', 'Build an edited site'));
    const edited = await build();
    assert.equal(edited.file, article);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact('generated/articles/first-build.html')); await settle();
    await viewReady('Build an edited site');
    await expandGeneratedOutput();
    await page.screenshot({ path: '/tmp/cem-studio-compact-editor-preview.png', fullPage: true });
    const successful = await state();
    await save(original.replace('title: Build a small site', 'title: [broken'));
    await click('Build project');
    const failed = await state();
    assert.equal(failed.status, 'failed'); assert.ok(failed.diagnostics.includes('cem.markdown.front_matter_invalid'));
    assert.equal(failed.runId, successful.runId); assert.deepEqual(failed.artifacts, successful.artifacts);
    await save(original); await build(); await viewReady('Build a small site');
    await open(projectId, 'layouts-partials-chrome-cemt');
    const layout = await editor.evaluate(element => element.value);
    await save(layout.replace('Built from saved project resources.', 'Shared layout edited.'));
    const shared = await build();
    console.log('[cem-studio:markdown-site] saved edits and failure/repair passed');
    const composition = await page.evaluate(() => {
        const projection = globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection;
        return { footers: projection.outputs.filter(output => output.contentType.startsWith('text/html')).map(output => output.text.includes('Shared layout edited.')),
            origins: projection.buildRun.artifacts.find(artifact => artifact.path === 'generated/articles/first-build.html').provenance.map(frame => frame.uri) };
    });
    assert.deepEqual(composition.footers, [true, true, true, true]);
    assert.ok(composition.origins.includes(`studio://${projectId}/content/articles/first-build.md`));
    assert.ok(composition.origins.includes(`studio://${projectId}/layouts/partials/chrome.cemt`));
    const portableId = await page.evaluate(async projectId => {
        const application = globalThis.__cemStudioApplication;
        const { parseCemStudioProjectBundle, serializeCemStudioProjectBundle } = await import('@epa-wg/cem-studio/file-system-provider');
        const exported = await application.fileSystemProvider.exportFallback({ projectId });
        const bundle = parseCemStudioProjectBundle(exported.archive.bytes);
        const id = `site-portable-${crypto.randomUUID()}`;
        bundle.project = { ...bundle.project, id, name: 'Portable Markdown site', rootUri: `studio://${id}/` };
        await application.fileSystemProvider.importFallback({ archive: serializeCemStudioProjectBundle(bundle) });
        return id;
    }, projectId);
    await page.goto(`${url}/?project=${portableId}&file=${article}&build=build-site`); await ready();
    assert.deepEqual((await build()).artifacts, shared.artifacts);
    assert.equal((await state()).artifact, 'generated/index.html');
    await click('Pinned builds'); await click('Pin successful build');
    const pinned = await state();
    assert.equal(pinned.pinError, undefined); assert.equal(pinned.pins.length, 1);
    const pinId = pinned.pins[0].key;
    const pinnedProvenance = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection.buildRun.artifacts.map(item => item.provenance));
    await page.evaluate(async () => {
        await navigator.serviceWorker.ready;
        const url = new URL(location.href); url.searchParams.delete('artifact');
        history.replaceState(null, '', url);
    });
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        await page.waitForFunction(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.pins.length === 1);
        assert.equal((await state()).runId, undefined); // no native build has run after reload
        await click('Pinned builds');
        await choosePin(pinned.pins[0].name);
        const reopened = await state();
        assert.equal(reopened.pinError, undefined); assert.equal(reopened.pinId, pinId); assert.equal(reopened.status, undefined);
        assert.equal(reopened.runId, pinned.runId); assert.deepEqual(reopened.artifacts, pinned.artifacts);
        assert.equal(reopened.file, article); await viewReady('CEM field notes');
        await page.waitForFunction(() => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('img')?.naturalWidth === 480);
        assert.equal(await frame.evaluate(frame => frame.contentWindow.getComputedStyle(frame.contentDocument.querySelector('nav')).display), 'flex');
        assert.deepEqual(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection.buildRun.artifacts.map(item => item.provenance)), pinnedProvenance);
        await choosePin('Current build');
        assert.equal((await state()).runId, undefined); // reopening a pin does not create a current build
        const offline = await build();
        assert.deepEqual(offline.artifacts, shared.artifacts); assert.equal(offline.file, article);
        await viewReady('CEM field notes');
        assert.equal(offline.entryArtifact, 'generated/index.html');
        const layout = await withProject(() => page.locator('cem-studio-workbench-layout').evaluate(host => {
            const toolbar = host.querySelector('cem-studio-workbench-preview[section="build"] [part="build-controls"]');
            const preview = host.querySelector('cem-studio-workbench-preview[section="preview"] [part="preview"]');
            return { overflow: getComputedStyle(toolbar).overflowY, right: toolbar.getBoundingClientRect().right, left: preview.getBoundingClientRect().left };
        }));
        assert.equal(layout.overflow, 'auto'); assert.ok(layout.left >= layout.right, 'Project build controls and output preview occupy separate columns');
        await frame.contentFrame().getByRole('navigation', { name: 'Site navigation' }).getByRole('link', { name: 'Articles', exact: true }).click();
        await settle(); await viewReady('Articles');
        await frame.contentFrame().getByRole('navigation', { name: 'Site navigation' }).getByRole('link', { name: 'Home', exact: true }).click();
        await settle(); await viewReady('CEM field notes');
        assert.equal((await state()).file, article);
        await save(original.replaceAll('Build a small site', 'Only the portable copy'));
        await build();
        const independence = await page.evaluate(async projectId => {
            const { repository, workbench } = globalThis.__cemStudioApplication;
            const original = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId } })).value;
            return { original: new TextDecoder().decode(original.contents['content-articles-first-build-md']),
                edited: workbench.snapshot().projectPreview.projection.outputs.some(output => output.text?.includes('<title>Only the portable copy</title>')) };
        }, projectId);
        assert.equal(independence.original, original); assert.equal(independence.edited, true);
        const current = await state();
        await choosePin(pinned.pins[0].name);
        assert.deepEqual((await state()).artifacts, pinned.artifacts);
        const comparison = (await state()).comparison;
        assert.equal(comparison.baselineRunId, pinned.runId); assert.equal(comparison.currentRunId, current.runId);
        assert.deepEqual(comparison.counts, { added: 0, removed: 0, changed: 1, unchanged: 6 });
        await withProject(async () => {
            await click('Compare pin with current build');
            assert.match(await page.getByRole('list', { name: 'Artifact comparison', exact: true }).innerText(), /changed · generated\/articles\/first-build.html/);
            assert.match(await page.getByRole('list', { name: 'Pinned native provenance', exact: true }).innerText(), /content\/articles\/first-build.md/);
            assert.match(await page.getByRole('list', { name: 'Current native provenance', exact: true }).innerText(), /layouts\/partials\/chrome.cemt/);
        });
        await page.emulateMedia({ forcedColors: 'active' });
        await withProject(async () => {
            await page.getByRole('combobox', { name: 'Compare artifact', exact: true }).click();
            await page.getByRole('option', { name: 'generated/images/harbor.svg', exact: true }).click(); await settle();
            const comparisonLayout = await page.locator('cem-studio-workbench-layout').evaluate(element => {
                const toolbar = element.querySelector('cem-studio-workbench-preview[section="build"] [part="build-controls"]');
                const preview = element.querySelector('cem-studio-workbench-preview[section="preview"] [part="preview"]');
                return { overflow: getComputedStyle(toolbar).overflowY, clear: preview.getBoundingClientRect().left >= toolbar.getBoundingClientRect().right };
            });
            assert.deepEqual(comparisonLayout, { overflow: 'auto', clear: true });
        });
        assert.equal((await state()).artifact, 'generated/index.html');
        assert.equal((await state()).file, article);
        await page.emulateMedia({ forcedColors: 'none' });

        await click('Delete pinned build');
        assert.equal((await state()).comparison, undefined);
        assert.equal((await state()).pinId, undefined); assert.equal((await state()).runId, current.runId);
        assert.deepEqual((await state()).artifacts, current.artifacts); assert.equal((await state()).pins.length, 0);
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:markdown-site] portable independent copy and offline rebuild passed');
    return { outputs: 7, relativeNavigation: true, staticAssets: true, editRepair: true, sharedLayout: true, provenance: true, portableCopies: true, entryArtifact: true, pinnedSnapshots: true, buildComparison: true, offline: true };
}
