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
const layout = () => page.locator('cem-workspace[layout=project]').evaluate(workspace => {
    const measure = element => {
        const style = getComputedStyle(element), rect = element.getBoundingClientRect();
        return { visible: element.checkVisibility(), display: style.display, direction: style.flexDirection,
            overflowY: style.overflowY, height: rect.height, width: rect.width, x: rect.x, y: rect.y };
    };
    const root = workspace.querySelector(':scope > [part~=root]');
    const regions = [...root.querySelectorAll(':scope > [part~=region]')].filter(element => element.checkVisibility());
    const navigation = root.querySelector(':scope > [part~=navigation] > [part~=content]');
    const accordion = navigation.querySelector('cem-accordion[fit=fill][label=Navigation]');
    const accordionRoot = accordion.querySelector(':scope > [part~=root]');
    return { workspace: measure(root), columns: getComputedStyle(root).gridTemplateColumns.split(/\s+/u).length,
        regions: regions.map(element => ({ part: element.getAttribute('part'), ...measure(element) })),
        navigation: measure(navigation), accordion: measure(accordionRoot),
        panels: [...accordionRoot.querySelectorAll(':scope > cem-expansion')].map(element => ({
            label: element.getAttribute('label'), expanded: element.hasAttribute('expanded'), ...measure(element),
            header: measure(element.querySelector(':scope > [part~=root] > [part~=heading] > [part~=header]')),
            body: measure(element.querySelector(':scope > [part~=root] > [part~=panel]')),
        })),
        scopedStyles: ['cem-accordion', 'cem-expansion', 'cem-studio-workbench-navigation'].map(tag =>
            [...document.querySelectorAll('style[data-cem-declaration-style]')].some(style => style.textContent.includes(`@scope (${tag})`))),
    };
});
const assertIdeLayout = current => {
    assert.equal(current.workspace.display, 'grid'); assert.equal(current.columns, 3);
    assert.deepEqual(current.regions.map(region => region.part), ['region navigation', 'region editor', 'region preview']);
    for (const region of current.regions) {
        assert.ok(region.width > 0 && region.height > 500, 'The IDE regions retain usable viewport height');
        assert.ok(Math.abs(region.height - current.workspace.height) < 1);
    }
    assert.ok(current.regions[0].x < current.regions[1].x && current.regions[1].x < current.regions[2].x);
    assert.equal(current.accordion.display, 'flex'); assert.equal(current.accordion.direction, 'column');
    assert.equal(current.accordion.overflowY, 'auto');
    assert.ok(Math.abs(current.accordion.height - current.navigation.height) < 1, 'The accordion fills the navigation body');
    assert.deepEqual(current.panels.map(panel => panel.label), ['Projects', 'Structure', 'Output files']);
    for (const panel of current.panels) {
        assert.equal(panel.display, 'flex'); assert.equal(panel.header.visible, true);
        assert.ok(panel.header.height > 0 && panel.header.height < 48, 'Accordion headings keep content-sized compact geometry');
        assert.equal(panel.body.overflowY, 'auto'); assert.equal(panel.body.visible, panel.expanded);
        if (panel.expanded) assert.ok(panel.body.height > 0);
    }
    assert.deepEqual(current.scopedStyles, [true, true, true], 'The current navigation declarations install their scoped CSS');
};
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
    assert.ok(await page.locator('cem-workspace[layout=project]').isVisible());
    assert.match(await page.evaluate(() => navigator.serviceWorker.controller?.scriptURL), /\/legacy-worker\.js$/);
    const beforeCollapse = await layout(); assertIdeLayout(beforeCollapse);
    const structure = page.locator('cem-accordion[label=Navigation] > [part~=root] > cem-expansion[label=Structure]');
    const structureButton = structure.locator(':scope > [part~=root] > [part~=heading] > [part~=header]');
    assert.equal(await structureButton.getAttribute('aria-expanded'), 'true');
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    assert.equal(await editor.evaluate(element => element.value), previous.draft);
    const retainedEditor = await editor.elementHandle();
    const retainedSelection = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.trees.structure.selected);
    const assertRetainedSource = async () => {
        const retained = await page.evaluate(async () => {
            const { workbench, repository } = globalThis.__cemStudioApplication;
            const state = workbench.snapshot();
            const stored = (await repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'export-project', parameters: { projectId: state.projectId } })).value;
            return { projectId: state.projectId, resourceId: state.resourceId, draft: state.draft, dirty: state.dirty,
                selected: state.navigation.trees.structure.selected, persisted: [...new Uint8Array(stored.contents[state.resourceId])] };
        });
        assert.equal(retained.projectId, previous.projectId); assert.equal(retained.resourceId, previous.resourceId);
        assert.equal(retained.draft, previous.draft); assert.equal(retained.dirty, true); assert.equal(retained.selected, retainedSelection);
        assert.deepEqual(retained.persisted, previous.persisted); assert.equal(page.url(), bookmark);
        assert.equal(await editor.evaluate((element, original) => element === original, retainedEditor), true, 'Collapsing navigation retains the editor DOM owner');
        assert.equal(await editor.evaluate(element => element.value), previous.draft);
    };
    await structureButton.click(); await ready();
    assert.equal(await structureButton.getAttribute('aria-expanded'), 'false');
    const collapsed = await layout(); assertIdeLayout(collapsed);
    assert.ok(collapsed.panels[1].height < beforeCollapse.panels[1].height);
    assert.ok(collapsed.panels[0].height > beforeCollapse.panels[0].height, 'Other expanded panes receive the released height');
    await assertRetainedSource();
    await structureButton.click(); await ready();
    assert.equal(await structureButton.getAttribute('aria-expanded'), 'true');
    const reopened = await layout(); assertIdeLayout(reopened);
    assert.ok(Math.abs(reopened.panels[1].height - beforeCollapse.panels[1].height) < 1);
    await assertRetainedSource();
    const selectedFile = structure.locator('cem-tree[label="Project files"] [role=treeitem][aria-selected=true]');
    assert.equal(await selectedFile.getAttribute('data-tree-value'), retainedSelection);
    assert.ok(await selectedFile.isVisible());
    await retainedEditor.dispose();
    assert.equal(await page.getByRole('button', { name: 'Install', exact: true }).count(), 0);
    assert.equal(await page.getByText(/Local storage/).count(), 0);
    assert.equal(await page.evaluate(async staleUrl => (await (await caches.open('cem-studio:0.1.0:runtime')).match(staleUrl)).text(), staleUrl), 'throw new Error("Stale Studio bootstrap");');
    assert.deepEqual(errors, []);
    console.log('Preview reload passed with a stale controlling worker: fresh three-region IDE, full-height accordion and collapse/reopen, stable project/file URL, persisted project and recovered draft. No cache or browser-data clearing.');
} finally {
    await browser.close();
    await new Promise(resolveClose => server.close(resolveClose));
    await rm(fixture, { recursive: true, force: true });
}
