import assert from 'node:assert/strict';

/** Ordinary navigator workflow in the delivery gate's isolated browser profile. */
export async function verifyMarkdownSite({ page, context, url }) {
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
    const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
    const state = () => page.evaluate(() => {
        const snapshot = globalThis.__cemStudioApplication.workbench.snapshot();
        const preview = snapshot.projectPreview;
        return { projectId: snapshot.navigation.selection.project, file: snapshot.navigation.selection.file,
            status: preview.attempt?.status, diagnostics: preview.attempt?.diagnostics?.map(item => item.code),
            runId: preview.projection?.buildRun.id, artifact: preview.artifactPath,
            artifacts: preview.projection?.buildRun.artifacts.map(item => ({ path: item.path, sha256: item.sha256 })),
            html: preview.projection?.output.text };
    });
    const build = async () => { await click('Build project'); const current = await state(); assert.equal(current.status, 'succeeded', JSON.stringify(current)); return current; };
    const frame = page.locator('cem-media-preview[mode="artifact-tree"] iframe');
    const viewReady = async title => {
        await page.waitForFunction(title => document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument?.querySelector('h1')?.textContent === title, title);
    };
    const save = async text => { await page.getByRole('textbox', { name: 'CEM source', exact: true }).fill(text); await settle(); await click('Save and validate'); };
    const open = async (project, file) => {
        await page.evaluate(async ({ project, file }) => {
            await globalThis.__cemStudioApplication.workbench.openEntity(`project=${project}&file=${file}`);
        }, { project, file }); await settle();
    };
    await page.goto(`${url}/?sample=markdown-site&build=build-site&artifact=generated%2Findex.html`); await ready();
    const sample = await build();
    console.log('[cem-studio:markdown-site] catalog build ready');
    assert.equal(sample.artifacts.length, 7); await viewReady('CEM field notes');
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
    const article = 'content-articles-first-build-md';
    await open(projectId, article);
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    const original = await editor.evaluate(element => element.value);
    await save(original.replaceAll('Build a small site', 'Build an edited site'));
    const edited = await build();
    assert.equal(edited.file, article);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.selectPreviewArtifact('generated/articles/first-build.html')); await settle();
    await viewReady('Build an edited site');
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
    await page.goto(`${url}/?project=${portableId}&file=${article}&build=build-site&artifact=generated%2Farticles%2Ffirst-build.html`); await ready();
    assert.deepEqual((await build()).artifacts, shared.artifacts);
    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await context.setOffline(true);
    try {
        await page.reload(); await ready();
        const offline = await build();
        assert.deepEqual(offline.artifacts, shared.artifacts); assert.equal(offline.file, article);
        await viewReady('Build a small site');
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
    } finally { await context.setOffline(false); }
    console.log('[cem-studio:markdown-site] portable independent copy and offline rebuild passed');
    return { outputs: 7, relativeNavigation: true, staticAssets: true, editRepair: true, sharedLayout: true, provenance: true, portableCopies: true, offline: true };
}
