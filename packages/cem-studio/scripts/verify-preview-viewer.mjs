import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

const projectRoot = resolve(import.meta.dirname, '..');
const security = JSON.parse(await readFile(resolve(projectRoot, 'dist/static/security-headers.json'), 'utf8'));
const staticHandler = createCemStudioPreviewHandler({ versionedAssets: false });
const versionedHandler = createCemStudioPreviewHandler();
const server = createServer((request, response) => {
    if (request.url === '/sender.html') response.writeHead(200, { ...security.headers, 'content-type': 'text/html' }).end('<!doctype html><title>Snapshot transport fixture</title>');
    else if (new URL(request.url, 'http://localhost').searchParams.has('sample')) void versionedHandler(request, response);
    else void staticHandler(request, response);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
let browser;
const results = [];
try {
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    const sender = await context.newPage();
    await sender.goto(`${origin}/sender.html`);
    await sender.evaluate(async () => {
        await navigator.serviceWorker.register('./service-worker.js', { type: 'module' });
        let timeout;
        try {
            await Promise.race([navigator.serviceWorker.ready, new Promise((_, reject) => { timeout = setTimeout(() => reject(new Error('Offline viewer installation timed out')), 30000); })]);
        } finally { clearTimeout(timeout); }
        if (!navigator.serviceWorker.controller) await new Promise((resolve, reject) => {
            const ready = () => { clearTimeout(timeout); resolve(); };
            timeout = setTimeout(() => { navigator.serviceWorker.removeEventListener('controllerchange', ready); reject(new Error('Offline viewer control timed out')); }, 30000);
            navigator.serviceWorker.addEventListener('controllerchange', ready, { once: true });
        });
        globalThis.snapshotFixtureApi = await import('./assets/@epa-wg/cem-elements/dist/index.js');
        globalThis.snapshotFixtureCapture = (await import('./assets/@epa-wg/cem-studio/workbench.js')).captureCemStudioDisplayedPreview;
        localStorage.setItem('cem-studio-theme', 'cem-theme-projector');
    });
    await context.setOffline(true);
    for (const mode of ['text', 'data', 'document', 'artifact-tree']) {
        const { token, label } = await sender.evaluate(mode => {
            const api = globalThis.snapshotFixtureApi;
            const bytes = text => new TextEncoder().encode(text);
            let view;
            if (mode === 'artifact-tree') {
                const tree = api.registerCemArtifactTree([
                    { path: 'index.html', contentType: 'text/html', bytes: bytes('<link rel="stylesheet" href="css/site.css"><h1>Captured site</h1><img alt="Pixel" src="pixel.png"><a href="next.html#land">Next page</a>') },
                    { path: 'css/site.css', contentType: 'text/css', bytes: bytes('h1 { color: rgb(4, 5, 6); }') },
                    { path: 'next.html', contentType: 'text/html', bytes: bytes('<h1 id="land">Second captured page</h1>') },
                    { path: 'pixel.png', contentType: 'image/png', bytes: Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jJXcAAAAASUVORK5CYII='), ch => ch.charCodeAt(0)) },
                ]);
                view = { kind: 'artifact-tree', resourceSet: tree.resourceSet, artifactPath: 'index.html?selected=one#top', label: 'Site snapshot' };
                globalThis.snapshotFixtureTree = tree;
            } else if (mode === 'document') view = { kind: 'sandboxed-html', source: '<p>Isolated document</p><script>parent.__snapshotAttack=true</script>', label: 'Document snapshot' };
            else view = { kind: 'text', text: mode === 'text' ? '<b>Café literal</b>\n' : '[{"name":"Ada"}]', label: 'Result snapshot' };
            const contentType = mode === 'data' ? 'application/json' : 'text/plain';
            const output = { uri: 'cem-stdio://stdout', contentType, bytes: bytes(view.text ?? 'unused') };
            const store = api.createCemPreviewSnapshotStore();
            const snapshot = globalThis.snapshotFixtureCapture({ projectId: 'project', resourceId: 'source', entryId: 'build',
                navigation: { selection: { project: 'project', file: 'source', configuration: 'build' } },
                projection: { preview: view, output, stale: true,
                buildRun: { id: `captured-${mode}`, projectId: 'project', projectRevision: 1, configuration: { id: 'build' }, inputs: [{ resourceId: 'source', revision: 1 }], artifacts: [{ uri: output.uri, path: 'output.txt' }] } } }, store);
            const label = snapshot.identity.label;
            const offer = api.offerCemPreviewSnapshot(snapshot); snapshot.dispose();
            globalThis.snapshotFixtureStore = store; globalThis.snapshotFixtureOffer = offer;
            return { token: offer.token, label };
        }, mode);
        const viewer = await context.newPage(), errors = [], diagnostics = [];
        viewer.on('pageerror', error => errors.push(error.message));
        viewer.on('console', message => { if (message.type() === 'error') diagnostics.push(message.text()); });
        await viewer.goto(`${origin}/preview.html#snapshot=${token}`);
        try { await sender.evaluate(() => globalThis.snapshotFixtureOffer.completed); }
        catch (error) { throw new Error(`${mode} viewer handoff failed: ${await viewer.locator('body').innerText()} ${JSON.stringify(diagnostics)}`, { cause: error }); }
        await sender.evaluate(() => { globalThis.snapshotFixtureStore.dispose(); globalThis.snapshotFixtureTree?.dispose(); });
        if (mode === 'artifact-tree') await sender.close();
        await viewer.getByRole('heading', { level: 1, name: label, exact: true }).waitFor();
        assert.equal(await viewer.evaluate(() => location.hash), '');
        assert.equal(await viewer.evaluate(() => window.opener), null);
        assert.equal(await viewer.locator('[data-theme]').getAttribute('data-theme'), 'cem-theme-projector');
        if (mode === 'text') {
            assert.equal(await viewer.locator('pre').textContent(), '<b>Café literal</b>\n');
            assert.equal(await viewer.locator('pre b').count(), 0);
        } else if (mode === 'data') {
            // Native data cells retain the JSON string's quoted presentation.
            try { await viewer.getByRole('cell', { name: '"Ada"', exact: true }).waitFor(); }
            catch (error) { throw new Error(`Data viewer content failed: ${await viewer.locator('body').innerText()} ${JSON.stringify(diagnostics)}`, { cause: error }); }
        } else if (mode === 'document') {
            assert.equal(await viewer.locator('iframe').getAttribute('sandbox'), '');
            assert.equal(await viewer.evaluate(() => globalThis.__snapshotAttack), undefined);
        } else {
            await viewer.waitForFunction(() => document.querySelector('iframe')?.contentDocument?.querySelector('img')?.naturalWidth === 1);
            assert.equal(await viewer.evaluate(() => getComputedStyle(document.querySelector('iframe').contentDocument.querySelector('h1')).color), 'rgb(4, 5, 6)');
            await viewer.frameLocator('iframe').getByRole('link', { name: 'Next page' }).click();
            await viewer.frameLocator('iframe').getByRole('heading', { name: 'Second captured page' }).waitFor();
            await viewer.emulateMedia({ forcedColors: 'active' });
            assert.equal(await viewer.evaluate(() => matchMedia('(forced-colors: active)').matches), true);
        }
        await viewer.reload();
        await viewer.locator('cem-alert [part~=root][role=alert]').filter({ hasText: 'Preview transfer unavailable' }).waitFor();
        assert.deepEqual(errors, []);
        results.push({ mode, offline: true, tokenRemoved: true, reloadRefused: true, sourceIndependent: true });
        console.log(`Verified packaged ${mode} viewer offline.`);
        await viewer.close();
    }
    await context.close();
    // The native header command must use trusted activation and a noopener
    // browsing context; service-only fixtures cannot prove popup policy.
    const studioContext = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
    const studio = await studioContext.newPage(); studio.setDefaultTimeout(30000);
    await studio.goto(`${origin}/?sample=markdown-site&build=build-site`);
    await studio.waitForFunction(() => globalThis.__cemStudioApplication);
    const settle = () => studio.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    await studio.getByRole('button', { name: 'Clone selected', exact: true }).click();
    const clone = studio.getByRole('dialog');
    await clone.getByRole('textbox').fill('Detached preview lifecycle fixture');
    await clone.getByRole('button', { name: 'Clone', exact: true }).click(); await settle();
    await studio.evaluate(() => globalThis.__cemStudioApplication.workbench.buildProjectPreview()); await settle();
    const captured = await studio.evaluate(async () => {
        const { createCemPreviewSnapshotStore } = await import(new URL('./assets/@epa-wg/cem-elements/dist/index.js', document.baseURI).href);
        const store = createCemPreviewSnapshotStore();
        try {
            const snapshot = globalThis.__cemStudioApplication.workbenchView.capturePreview(store);
            return { identity: snapshot.identity, mode: snapshot.preview.mode, path: snapshot.preview.artifactPath };
        } finally { store.dispose(); }
    });
    assert.equal(captured.mode, 'artifact-tree');
    const sourceRoute = studio.url();
    const action = studio.getByRole('button', { name: 'Open in separate window', exact: true });
    const opened = studioContext.waitForEvent('page'); await action.focus(); await action.press('Enter');
    let detached = await opened; detached.setDefaultTimeout(30000);
    await settle();
    await detached.getByRole('heading', { level: 1, name: captured.identity.label, exact: true }).waitFor();
    assert.equal(await detached.evaluate(() => window.opener), null);
    assert.equal(await detached.evaluate(() => location.hash), '');
    assert.equal(studio.url(), sourceRoute);
    const viewerUrl = await studio.evaluate(() => new URL('preview.html', document.baseURI).href);
    assert.equal(detached.url(), viewerUrl);
    assert.ok(new URL(viewerUrl).pathname.startsWith('/__cem_preview__/'));
    assert.equal(await action.evaluate(button => document.activeElement === button), true);
    const originalRun = captured.identity.runId;
    const nextPath = await studio.evaluate(async initial => {
        const workbench = globalThis.__cemStudioApplication.workbench;
        const path = workbench.snapshot().projectPreview.projection.buildRun.artifacts.find(item => item.path !== initial && item.contentType === 'text/html')?.path;
        if (!path) throw Error('Markdown fixture needs a second HTML artifact');
        await workbench.selectPreviewArtifact(path); return path;
    }, captured.path);
    await settle();
    await action.click(); await settle();
    assert.equal(studioContext.pages().length, 2, 'Repeated opening reuses the existing viewer');
    await detached.waitForFunction(path => document.querySelector('cem-media-preview')?.getAttribute('artifact-path') === path, nextPath);

    assert.equal(await studio.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection.buildRun.id), originalRun, 'Opening never rebuilds');
    const pin = await studio.evaluate(async () => {
        const workbench = globalThis.__cemStudioApplication.workbench;
        await workbench.pinProjectBuild();
        const state = workbench.snapshot().projectPreview;
        if (state.pinError) throw Error(state.pinError.message);
        return state.pins[0].key;
    });
    await studio.evaluate(() => globalThis.__cemStudioApplication.workbench.buildProjectPreview()); await settle();
    const newRun = await studio.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.projection.buildRun.id);
    assert.notEqual(newRun, originalRun);
    await detached.getByText(`Captured run: ${originalRun}`, { exact: true }).waitFor();
    await action.click(); await settle();
    await detached.getByText(`Captured run: ${newRun}`, { exact: true }).waitFor();
    await studio.evaluate(pin => globalThis.__cemStudioApplication.workbench.selectPinnedBuild(pin), pin); await settle();
    await action.click(); await settle();
    await detached.getByText('Pinned build snapshot', { exact: true }).waitFor();
    await detached.getByText(`Captured run: ${originalRun}`, { exact: true }).waitFor();
    await studio.evaluate(async () => {
        const workbench = globalThis.__cemStudioApplication.workbench;
        await workbench.selectPinnedBuild('');
        const { project } = workbench.snapshot().navigation.selection;
        const build = workbench.snapshot().projectPreview.configurationId;
        await workbench.openEntity(new URLSearchParams({ project, build, file: 'content-articles-first-build-md' }).toString());
        await workbench.selectPreviewArtifact('generated/articles/first-build.html');
        workbench.updateDraft('# A changed draft awaiting rebuild');
    }); await settle();
    const retainedPath = await studio.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().projectPreview.preview.artifactPath);
    await action.click(); await settle();
    await detached.getByText('Stale output retained from the captured preview', { exact: true }).waitFor();
    await detached.getByText(`Captured run: ${newRun}`, { exact: true }).waitFor();
    assert.equal(studioContext.pages().length, 2);
    await studio.emulateMedia({ forcedColors: 'active' });
    await action.hover();
    assert.equal(await action.evaluate(button => getComputedStyle(button, '::before').display), 'none');
    await detached.emulateMedia({ forcedColors: 'active' });
    assert.equal(await detached.evaluate(() => matchMedia('(forced-colors: active)').matches), true);
    await detached.close();
    // Wait for the close notification rather than adding a scheduler sleep.
    await studio.waitForFunction(() => !document.querySelector('[part=preview-actions]')?.textContent.includes('Separate preview is open.'));
    await studio.waitForFunction(async () => Boolean((await navigator.serviceWorker.getRegistration(document.baseURI))?.active));
    await studioContext.setOffline(true);
    const reopened = studioContext.waitForEvent('page'); await action.click(); detached = await reopened; await settle();
    await detached.locator('cem-media-preview[mode=artifact-tree]').waitFor();
    await studio.close();
    assert.equal(await detached.locator('cem-media-preview').getAttribute('artifact-path'), retainedPath);
    await detached.locator('iframe').waitFor();
    results.push({ mode: 'header-launcher', keyboard: true, noopener: true, reuse: true, reopen: true, offline: true, sourceIndependent: true, selectionPreserved: true, scopedBase: true, runReplacement: true, pinned: true, stale: true });
    await studioContext.close();
    const reportRoot = resolve(projectRoot, '../../dist/reports/cem-studio');
    await mkdir(reportRoot, { recursive: true });
    await writeFile(resolve(reportRoot, 'preview-viewer.json'), `${JSON.stringify({ cases: results }, null, 2)}\n`);
    console.log(`Verified ${results.length} packaged offline preview modes, independent viewer ownership and expired reloads.`);
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
