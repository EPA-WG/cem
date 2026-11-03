import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { chromium } from 'playwright';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';
import { deploymentRevisionModule } from './deployment-revision.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const root = resolve(workspace, 'packages/cem-studio/dist/static');
const reportPath = resolve(workspace, 'dist/reports/cem-studio/deployment-upgrade.json');
const asset = 'assets/@epa-wg/cem-ml-cli/dist/browser-command.js';
const current = await readFile(resolve(root, asset), 'utf8');
const baseline = process.env.CEM_STUDIO_UPGRADE_BASELINE_ROOT;
assert.ok(baseline, 'Set CEM_STUDIO_UPGRADE_BASELINE_ROOT to the previous graph-emitted Studio directory');
const previous = await readFile(resolve(baseline, asset), 'utf8');
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
assert.notEqual(hash(previous), hash(current));
const handler = createCemStudioPreviewHandler({ root, versionedAssets: false });
const previousHandler = createCemStudioPreviewHandler({ root: resolve(baseline), versionedAssets: false });
let currentDeployment = false;
let dependencyUpdate = false;
const laterModule = `// Subsequent deployment fixture\n${current}`;
const laterRevision = hash(laterModule);
const headers = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8')).headers;
const server = createServer((request, response) => {
    if (request.url === '/upgrade-control') {
        response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><title>Upgrade controller</title>'); return;
    }
    if (!request.url?.startsWith('/studio/')) { response.writeHead(404).end(); return; }
    request.url = request.url.slice('/studio'.length);
    if (dependencyUpdate && [ `/${asset}`, '/deployment-revision.js' ].includes(request.url)) {
        response.writeHead(200, { ...headers, 'content-type': 'text/javascript', 'cache-control': 'no-store' })
            .end(request.url === `/${asset}` ? laterModule : deploymentRevisionModule(laterRevision)); return;
    }
    void (currentDeployment ? handler : previousHandler)(request, response);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), browser: browser.version(),
    scope: `http://127.0.0.1:${server.address().port}/studio/`,
    isPublicAcceptance: false, fixture: 'previous and current graph-emitted deployments at unchanged scope/version', status: 'started' };
try {
    const context = await browser.newContext();
    const control = await context.newPage();
    await control.goto(new URL('/upgrade-control', report.scope).href);
    const inspector = await context.newCDPSession(control);
    report.workerVersions = []; report.workerErrors = [];
    inspector.on('ServiceWorker.workerVersionUpdated', event => report.workerVersions.push(...event.versions));
    inspector.on('ServiceWorker.workerErrorReported', event => report.workerErrors.push(event.errorMessage));
    await inspector.send('ServiceWorker.enable');
    context.on('serviceworker', worker => worker.on('console', message => {
        if (message.type() === 'error') report.workerErrors.push({ message: message.text() });
    }));
    let page = await context.newPage();
    page.setDefaultTimeout(30_000);
    const ready = async () => {
        await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView);
        await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    };
    await page.goto(report.scope); await ready();
    await page.evaluate(() => navigator.serviceWorker.ready);
    await page.waitForFunction(() => navigator.serviceWorker.controller);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.openEntity('collection=content-types-documents-ui&sample=scss-stylesheet&build=build-html'));
    await ready();
    await page.getByRole('button', { name: 'Clone selected', exact: true }).click();
    await page.getByRole('dialog').getByRole('textbox').fill('Deployment upgrade witness');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click();
    await ready();
    await page.evaluate(async () => {
        const app = globalThis.__cemStudioApplication;
        await app.workbench.refreshNavigation();
        const choice = app.workbench.snapshot().home.projects.find(project => project.name === 'Deployment upgrade witness');
        const { projectId } = JSON.parse(choice.id);
        const { value: project } = await app.repository.query({ protocolVersion: 1, repository: 'studio-projects',
            operation: 'get-project', requestRevision: 1, parameters: { projectId } });
        const resource = project.resources.find(resource => resource.contentType === 'text/cem-ml') ?? project.resources[0];
        await app.workbench.selectWorkbench(JSON.stringify({ projectId, resourceId: resource.id }));
        await app.workbenchView.whenSettled();
    });
    const snapshot = () => page.evaluate(() => {
        const state = globalThis.__cemStudioApplication.workbench.snapshot();
        return { projectId: state.projectId, projectRevision: state.projectRevision,
            resourceId: state.resourceId, persistedText: state.persistedText, draft: state.draft, dirty: state.dirty };
    });
    const saved = await snapshot();
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true, includeHidden: true });
    await editor.waitFor({ state: 'attached' });
    const tabs = await editor.evaluate(element => {
        const tabs = [];
        for (let ancestor = element.parentElement; ancestor; ancestor = ancestor.parentElement) {
            if (ancestor.getAttribute('role') !== 'tabpanel') continue;
            const tab = document.getElementById(ancestor.getAttribute('aria-labelledby'));
            if (tab) tabs.unshift(tab.textContent.trim());
        }
        return tabs;
    });
    for (const name of tabs) await page.getByRole('tab', { name, exact: true }).click();
    await editor.fill(`${saved.draft}\n// Unsaved deployment-upgrade witness\n`);
    await ready();
    await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication.session.draftBackup()));
    const before = await snapshot(); assert.equal(before.dirty, true); assert.ok(before.projectId);
    report.before = before;
    const url = new URL(asset, report.scope).href;
    report.cachedAsset = await page.evaluate(async url => {
        const names = await caches.keys();
        for (const name of names) {
            const cache = await caches.open(name);
            if (await cache.match(url)) {
                return { cacheName: name, url };
            }
        }
        throw Error('Expected the installed command module in the deployment cache');
    }, url);
    assert.equal(hash(await page.evaluate(async url => (await fetch(url)).text(), url)), hash(previous));
    const earlierVersions = new Set(report.workerVersions.map(version => version.versionId));
    currentDeployment = true;
    await page.evaluate(async () => { const registration = await navigator.serviceWorker.ready; await registration.update(); });
    await page.waitForFunction(async () => Boolean((await navigator.serviceWorker.ready).waiting));
    report.waitingWorkerInstalled = true;
    const nextVersion = report.workerVersions.find(version => !earlierVersions.has(version.versionId));
    assert.ok(nextVersion, 'The update must install a new service-worker version');
    const activated = new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { inspector.off('ServiceWorker.workerVersionUpdated', check); reject(Error('Updated service worker did not activate after closing its clients')); }, 30_000);
        function check(event) {
            if (!event.versions.some(version => version.versionId === nextVersion.versionId && version.status === 'activated')) return;
            clearTimeout(timeout); inspector.off('ServiceWorker.workerVersionUpdated', check); resolve();
        }
        inspector.on('ServiceWorker.workerVersionUpdated', check);
        check({ versions: report.workerVersions });
    });
    const savedUrl = page.url();
    // Ordinary close/reopen activates a waiting update. Do not bypass the host's
    // explicit activation/dirty-work policy or clear any browser storage.
    await page.close(); await activated;
    page = await context.newPage(); page.setDefaultTimeout(30_000);
    await page.goto(new URL('/upgrade-control', report.scope).href);
    await page.waitForFunction(async () => {
        const registration = await navigator.serviceWorker.getRegistration('/studio/');
        return registration?.active?.state === 'activated' && !registration.waiting && !registration.installing;
    });
    await page.goto(savedUrl); await ready();
    const after = await snapshot();
    assert.deepEqual(after, before, 'Deployment refresh must preserve the saved project and draft');
    report.projectAndDraftPreserved = true;
    const received = await page.evaluate(async url => (await fetch(url)).text(), url);
    report.expectedSha256 = hash(current); report.previousSha256 = hash(previous); report.receivedSha256 = hash(received);
    report.updateState = await page.evaluate(async () => {
        const registration = await navigator.serviceWorker.ready;
        return { waiting: Boolean(registration.waiting), installing: Boolean(registration.installing) };
    });
    assert.equal(hash(received), hash(current), 'Existing cached clients must receive the current command module after deployment refresh');
    // A subsequent release changes only a runtime module and the revision
    // dependency; the primary service-worker script remains byte-identical.
    const existingVersions = new Set(report.workerVersions.map(version => version.versionId));
    dependencyUpdate = true;
    await page.evaluate(async () => (await navigator.serviceWorker.ready).update());
    await page.waitForFunction(async () => Boolean((await navigator.serviceWorker.ready).waiting));
    assert.equal(hash(await page.evaluate(async url => (await fetch(url)).text(), url)), hash(current), 'An old active worker must stay pinned while the new revision waits');
    const dependencyVersion = report.workerVersions.find(version => !existingVersions.has(version.versionId));
    assert.ok(dependencyVersion, 'Changing the revision dependency must install an update');
    const dependencyActivated = new Promise((resolve, reject) => {
        const timeout = setTimeout(() => { inspector.off('ServiceWorker.workerVersionUpdated', check); reject(Error('Dependency update did not activate')); }, 30_000);
        function check(event) {
            if (!event.versions.some(version => version.versionId === dependencyVersion.versionId && version.status === 'activated')) return;
            clearTimeout(timeout); inspector.off('ServiceWorker.workerVersionUpdated', check); resolve();
        }
        inspector.on('ServiceWorker.workerVersionUpdated', check);
        check({ versions: report.workerVersions });
    });
    await page.close(); await dependencyActivated;
    page = await context.newPage(); page.setDefaultTimeout(30_000);
    await page.goto(savedUrl); await ready();
    assert.deepEqual(await snapshot(), before);
    assert.equal(hash(await page.evaluate(async url => (await fetch(url)).text(), url)), hash(laterModule));
    report.dependencyOnlyUpdate = { installed: true, oldWorkerPinnedWhileWaiting: true,
        projectAndDraftPreserved: true, primaryWorkerUnchanged: true, revision: laterRevision };
    await context.setOffline(true); await page.reload(); await ready();
    assert.deepEqual(await snapshot(), before, 'The upgraded project and draft must also reload offline');
    report.offlineReloadPreserved = true;
    report.status = 'passed';
} catch (error) {
    report.status = 'failed'; report.error = error.message.split('\n')[0]; throw error;
} finally {
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    await browser.close(); server.closeAllConnections(); await new Promise(done => server.close(done));
}
