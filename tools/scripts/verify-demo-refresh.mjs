import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { cp, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { homedir, tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { chromium } from 'playwright';
import { createDemoSiteHandler } from './serve-demo-site.mjs';

// A disposable deployment and browser profile; never touch the running user's data.
const root = await mkdtemp(join(tmpdir(), 'cem-demo-refresh-'));
await cp(resolve(process.argv[2] ?? join(homedir(), 'cem-bin')), root, { recursive: true });
const handler = createDemoSiteHandler({ root });
const legacy = `self.addEventListener('install', e => e.waitUntil(self.skipWaiting()));
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()));
self.addEventListener('fetch', e => { if (e.request.method === 'GET') e.respondWith(caches.match(e.request).then(cached => cached || fetch(e.request))); });`;
const server = createServer((request, response) => {
    if (request.url === '/studio/legacy-worker.js') response.writeHead(200, { 'content-type': 'text/javascript', 'cache-control': 'no-store' }).end(legacy);
    else void handler(request, response);
});
await new Promise(ready => server.listen(0, '127.0.0.1', ready));
const url = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const ready = async () => {
    await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
};
try {
    await page.goto(`${url}/studio/?collection=content-types-documents-ui&sample=css-stylesheet`); await ready();
    await page.getByRole('link', { name: 'README', exact: true }).waitFor();
    const caption = await page.getByRole('combobox', { name: 'Build configuration', exact: true }).evaluate(control => {
        const label = document.getElementById(control.getAttribute('aria-labelledby'));
        return { size: getComputedStyle(label).fontSize, weight: getComputedStyle(label).fontWeight };
    });
    assert.deepEqual(caption, { size: '14px', weight: '700' });
    await page.getByRole('button', { name: 'Build project', exact: true }).click(); await ready();
    const tree = page.getByRole('tree', { name: 'Output files', exact: true });
    assert.deepEqual(await tree.locator('[role=treeitem][aria-level="1"]').evaluateAll(items => items.map(item => item.getAttribute('aria-label')).sort()), ['cards.css', 'cards.html']);

    await page.goto(`${url}/studio/`); await ready();
    await page.evaluate(async () => { await navigator.serviceWorker.register('/studio/legacy-worker.js', { scope: '/studio/' }); });
    // The more specific current worker still controls the versioned page.
    await page.waitForFunction(async () => (await navigator.serviceWorker.getRegistrations()).some(registration => registration.active?.scriptURL.endsWith('/studio/legacy-worker.js')));
    const previous = await page.evaluate(async () => {
        const { workbench, repository } = globalThis.__cemStudioApplication;
        workbench.requestAction('create'); await workbench.submitAction({ name: ['Refresh retention fixture'] });
        const projectId = workbench.snapshot().navigation.selection.project;
        const file = workbench.snapshot().navigation.resources[0];
        await workbench.openEntity(new URLSearchParams({ project: projectId, file: file.id }).toString());
        const draft = '// unsaved refresh fixture\n{p | Keep this draft}'; workbench.updateDraft(draft);
        const stored = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'export-project', parameters: { projectId } })).value;
        return { projectId, resourceId: file.id, draft, persisted: [...new Uint8Array(stored.contents[file.id])] };
    });
    await ready();
    const bookmark = new URL(page.url());
    const before = await page.evaluate(() => document.baseURI);
    const staleUrl = new URL('assets/@epa-wg/cem-studio/main.js', before).href;
    await page.evaluate(async ({ staleUrl, url }) => {
        const cache = await caches.open('cem-studio:0.1.0:runtime');
        await cache.put(staleUrl, new Response('throw new Error("Stale Studio bootstrap");', { headers: { 'content-type': 'text/javascript' } }));
        await cache.put(`${url}/studio/`, new Response('<title>Stale Studio shell</title>', { headers: { 'content-type': 'text/html' } }));
    }, { staleUrl, url });
    assert.match(await page.evaluate(value => fetch(value).then(response => response.text()), staleUrl), /Stale Studio bootstrap/);
    await page.goto(`${url}/studio/`);
    assert.equal(await page.title(), 'Stale Studio shell');

    const mainPath = 'studio/assets/@epa-wg/cem-studio/main.js';
    const main = `${await readFile(join(root, mainPath), 'utf8')}\n// New local deployment\n`;
    await writeFile(join(root, mainPath), main);
    const inventory = JSON.parse(await readFile(join(root, 'demo-deployment.json'), 'utf8'));
    Object.assign(inventory.files.find(file => file.path === mainPath), { bytes: Buffer.byteLength(main), sha256: createHash('sha256').update(main).digest('hex') });
    await writeFile(join(root, 'demo-deployment.json'), JSON.stringify(inventory));
    const home = await fetch(url).then(response => response.text());
    const href = home.match(/href="(\/studio\/__cem_preview__\/[a-f0-9]{16}\/)"/)?.[1];
    assert.ok(href, 'Demo home links to the current Studio revision');
    const nextBase = new URL(href, url).href; assert.notEqual(nextBase, before);
    await page.goto(`${nextBase}${bookmark.search}`); await ready();
    const retained = () => page.evaluate(async () => {
        const { workbench, repository } = globalThis.__cemStudioApplication;
        const state = workbench.snapshot();
        const stored = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'export-project', parameters: { projectId: state.projectId } })).value;
        return { projectId: state.projectId, resourceId: state.resourceId, draft: state.draft, dirty: state.dirty,
            persisted: [...new Uint8Array(stored.contents[state.resourceId])] };
    });
    assert.deepEqual(await retained(), { ...previous, dirty: true });
    assert.equal(new URL(page.url()).search, bookmark.search);
    assert.equal(await page.evaluate(() => document.baseURI), nextBase);
    await page.waitForFunction(base => navigator.serviceWorker.controller?.scriptURL === `${base}service-worker.js`, nextBase);
    assert.equal(await page.evaluate(async () => (await navigator.serviceWorker.ready).scope), nextBase);
    await context.setOffline(true); await page.reload(); await ready();
    assert.deepEqual(await retained(), { ...previous, dirty: true });
    assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), await page.title());
    assert.deepEqual(errors, []);
    console.log('Demo refresh: stale shell/bootstrap bypass, current README/labels/output, same-origin saved project and unsaved draft, URL selection and scoped offline reload passed.');
} finally {
    await browser.close(); server.closeAllConnections(); await new Promise(done => server.close(done));
    await rm(root, { recursive: true, force: true });
}
