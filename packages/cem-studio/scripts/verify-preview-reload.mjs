import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

const fixture = await mkdtemp(resolve(tmpdir(), 'cem-studio-preview-'));
const report = resolve(fixture, 'build.json');
const revision = hash => writeFile(report, JSON.stringify({ hashScheme: 'sha256', hash: hash.repeat(64), assets: [] }));
await revision('a');
const handler = createCemStudioPreviewHandler({ buildReport: report });
// Reproduce the installed worker's cache-first behavior independently of the new preview.
const legacyWorker = `self.addEventListener('install', e => e.waitUntil(self.skipWaiting()));
self.addEventListener('activate', e => e.waitUntil(self.clients.claim()));
self.addEventListener('fetch', e => { if (e.request.method === 'GET') e.respondWith(caches.match(e.request).then(cached => cached || fetch(e.request))); });`;
const server = createServer((request, response) => {
    if (request.url === '/legacy-worker.js') response.writeHead(200, { 'content-type': 'text/javascript', 'service-worker-allowed': '/', 'cache-control': 'no-store' }).end(legacyWorker);
    else void handler(request, response);
});
await new Promise(resolveReady => server.listen(0, '127.0.0.1', resolveReady));
const url = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
page.setDefaultTimeout(30_000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled()); };
try {
    await page.goto(url); await ready();
    await page.evaluate(async () => { await navigator.serviceWorker.register('/legacy-worker.js', { scope: '/' }); });
    await page.waitForFunction(() => navigator.serviceWorker.controller?.scriptURL.endsWith('/legacy-worker.js'));
    await page.waitForFunction(async () => (await navigator.serviceWorker.getRegistrations()).every(registration => registration.active?.state === 'activated'));
    const baseBefore = await page.evaluate(() => document.baseURI);
    assert.match(baseBefore, /\/__cem_preview__\/[a-f0-9]{16}\/$/);
    // Persist a real project and unsaved source journal before rebuilding the preview.
    const previous = await page.evaluate(async () => {
        const { workbench, repository } = globalThis.__cemStudioApplication;
        workbench.requestAction('create');
        await workbench.submitAction({ name: ['Preview project'] });
        const projectId = workbench.snapshot().navigation.selection.project;
        const file = workbench.snapshot().navigation.resources[0];
        await workbench.openEntity(new URLSearchParams({ project: projectId, file: file.id }).toString());
        const draft = '// unsaved preview fixture\n{p | Keep my draft}';
        workbench.updateDraft(draft);
        const stored = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId } })).value;
        return { projectId, resourceId: file.id, draft, persisted: [...new Uint8Array(stored.contents[file.id])] };
    });
    await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const bookmark = page.url();
    const staleUrl = new URL('assets/@epa-wg/cem-studio/main.js', baseBefore).href;
    await page.evaluate(async staleUrl => {
        const cache = await caches.open('cem-studio:0.1.0:runtime');
        await cache.put(staleUrl, new Response('throw new Error("Stale Studio bootstrap");', { headers: { 'content-type': 'text/javascript' } }));
    }, staleUrl);
    assert.match(await page.evaluate(url => fetch(url).then(response => response.text()), staleUrl), /Stale Studio bootstrap/);
    await revision('b');
    await page.reload(); await ready();
    const next = await page.evaluate(async () => {
        const { workbench, repository } = globalThis.__cemStudioApplication;
        const state = workbench.snapshot();
        const stored = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId: state.projectId } })).value;
        return { base: document.baseURI, projectId: state.projectId, resourceId: state.resourceId, draft: state.draft, dirty: state.dirty,
            title: document.title, persisted: [...new Uint8Array(stored.contents[state.resourceId])] };
    });
    assert.notEqual(next.base, baseBefore);
    assert.equal(page.url(), bookmark);
    assert.equal(next.projectId, previous.projectId); assert.equal(next.resourceId, previous.resourceId);
    assert.equal(next.draft, previous.draft); assert.equal(next.dirty, true); assert.deepEqual(next.persisted, previous.persisted);
    assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), next.title);
    assert.ok(await page.locator('cem-workspace[layout=browser]').isVisible());
    assert.equal(await page.getByRole('button', { name: 'Install', exact: true }).count(), 0);
    assert.equal(await page.getByText(/Local storage/).count(), 0);
    assert.equal(await page.evaluate(async staleUrl => (await (await caches.open('cem-studio:0.1.0:runtime')).match(staleUrl)).text(), staleUrl), 'throw new Error("Stale Studio bootstrap");');
    assert.deepEqual(errors, []);
    console.log('Preview reload passed with a stale controlling worker: fresh layout, stable project/file URL, persisted project and recovered draft. No cache or browser-data clearing.');
} finally {
    await browser.close();
    await new Promise(resolveClose => server.close(resolveClose));
    await rm(fixture, { recursive: true, force: true });
}
