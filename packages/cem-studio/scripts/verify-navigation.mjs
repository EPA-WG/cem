import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

const projectRoot = resolve(import.meta.dirname, '..');
const server = await startStaticServer(resolve(projectRoot, 'dist/static'));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
const errors = [];
page.on('pageerror', error => errors.push(error.message));
const settle = () => page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication); await settle(); };
const state = () => page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation);
const click = async name => { await page.getByRole('button', { name, exact: true }).click(); await settle(); };
const tree = async (name, owner = 'Samples') => { await page.getByRole('tree', { name: owner, exact: true }).getByRole('treeitem', { name, exact: true }).click(); await settle(); };
const submit = async (name, action) => { if (name) await page.getByRole('dialog').getByRole('textbox').fill(name); await page.getByRole('dialog').getByRole('button', { name: action, exact: true }).click(); await settle(); };
const heading = async () => assert.equal(await page.getByRole('heading', { level: 1 }).innerText(), await page.title());

try {
    await page.goto(server.url); await ready();
    assert.equal((await state()).selection.key, 'samples'); await heading();
    const collection = await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.samples.children.find(group => group.children.some(item => item.route.sample === 'supplier-pipeline')));
    await tree(collection.name);
    assert.ok(await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'Supplier pipeline', exact: true }).isVisible());
    await tree('Supplier pipeline');
    assert.equal((await state()).content, 'folder');
    assert.equal(await page.evaluate(async () => (await globalThis.__cemStudioApplication.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation: 'list-projects', parameters: {} })).value.length), 0);
    await click('Clone selected'); await submit('Navigator Supplier', 'Clone');
    const projectId = (await state()).selection.project;
    assert.ok(projectId); await heading();
    const projectUrl = page.url();
    assert.ok(new URL(projectUrl).searchParams.get('project') === projectId);
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'data', exact: true }).click(); await settle();
    assert.equal((await state()).selection.folder, 'data/');
    const file = (await state()).children.find(item => item.name === 'stock.csv');
    await page.getByRole('list', { name: 'Folder contents' }).getByRole('button', { name: 'stock.csv', exact: true }).click(); await settle();
    assert.equal(new URL(page.url()).searchParams.get('file'), file.resourceId); await heading();
    const editor = page.getByRole('textbox', { name: 'CEM source', exact: true });
    await page.screenshot({ path: '/tmp/cem-studio-navigator.png', fullPage: true });
    const original = await editor.inputValue();
    await editor.fill(`${original}\n`); await settle();
    const editorIdentity = await editor.elementHandle();
    for (const mode of ['cem-theme-dark', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native', 'cem-theme-light']) {
        await page.evaluate(mode => globalThis.__cemStudioApplication.shell.theme.setMode(mode), mode); await settle();
        assert.ok(await editorIdentity.evaluate(element => element.isConnected)); await heading();
        const palette = await page.locator('cem-workspace [part~=region][part~=editor]').evaluate(element => { const style = getComputedStyle(element); return [style.color, style.backgroundColor]; });
        assert.notEqual(palette[0], palette[1]);
    }
    // A folder change is guarded by the same source admission as a file change.
    await page.getByRole('tree', { name: 'Navigator Supplier', exact: true }).getByRole('treeitem', { name: 'data', exact: true }).click(); await settle();
    assert.ok(await page.getByRole('dialog', { name: 'Resolve drafts before navigation' }).isVisible());
    await click('Cancel navigation');
    assert.equal((await state()).selection.file, file.resourceId);
    assert.equal(await editor.inputValue(), `${original}\n`);
    await click('Save and validate');
    await page.reload(); await ready();
    assert.equal((await state()).selection.file, file.resourceId); await heading();
    assert.equal(await page.getByRole('textbox', { name: 'CEM source', exact: true }).inputValue(), `${original}\n`);
    const bookmarked = page.url();
    await page.goto(projectUrl); await ready();
    await click('Create'); await submit('notes.cem', 'Create');
    assert.equal((await state()).selected.name, 'notes.cem');
    await click('Clone selected'); await submit('notes-copy.cem', 'Clone');
    assert.equal((await state()).selected.name, 'notes-copy.cem');
    await click('Delete selected'); await submit(undefined, 'Delete');
    assert.equal((await state()).content, 'folder');
    await page.goto(bookmarked); await ready();
    const fileUrl = page.url();
    await page.getByRole('tree', { name: 'Samples', exact: true }).getByRole('treeitem', { name: 'Samples', exact: true }).click(); await settle();
    assert.equal((await state()).selection.key, 'samples');
    await page.goBack(); await settle();
    assert.equal(page.url(), fileUrl); assert.equal((await state()).selection.file, file.resourceId);
    await page.goForward(); await settle(); assert.equal((await state()).selection.key, 'samples');
    await page.reload(); await ready(); assert.equal((await state()).selection.key, 'samples');
    const inset = await page.locator('cem-studio-shell').evaluate(shell => {
        shell.style.setProperty('--cem-inset-container', '23px');
        const region = shell.querySelector('cem-workspace [part~=editor] > [part=content]');
        const value = getComputedStyle(region).paddingInlineStart;
        shell.style.removeProperty('--cem-inset-container');
        return value;
    });
    assert.equal(inset, '23px');
    const layouts = [];
    for (const width of [1440, 900, 390, 320]) {
        await page.setViewportSize({ width, height: 1000 });
        const geometry = await page.locator('cem-workspace').evaluate(host => {
            const root = host.querySelector('[part=root]');
            const rect = part => host.querySelector(`[part~="${part}"]`).getBoundingClientRect().toJSON();
            return { width: innerWidth, pageWidth: document.documentElement.scrollWidth, navigation: rect('navigation'), editor: rect('editor'), columns: getComputedStyle(root).gridTemplateColumns.split(' ').length };
        });
        assert.equal(geometry.width, geometry.pageWidth); assert.equal(geometry.columns, width >= 900 ? 2 : 1); layouts.push(geometry);
    }
    await page.setViewportSize({ width: 1440, height: 1000 });
    await page.emulateMedia({ forcedColors: 'active' });
    const paint = await page.locator('cem-workspace').evaluate(host => [...host.querySelectorAll('[part~=region]')].filter(element => element.checkVisibility()).map(element => { const style = getComputedStyle(element); return { color: style.color, background: style.backgroundColor, border: style.borderTopWidth }; }));
    assert.equal(paint.length, 2); for (const region of paint) { assert.notEqual(region.color, region.background); assert.notEqual(region.border, '0px'); }
    await page.emulateMedia({ forcedColors: 'none' });
    // Run the ordinary Supplier configuration through the new file/configuration route.
    await page.goto(`${projectUrl}&configuration=availability`); await ready();
    assert.equal((await state()).error, undefined);
    await click('Run configuration');
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().status), 'projected');
    const provenance = await page.evaluate(async () => {
        const { workbench } = globalThis.__cemStudioApplication;
        const index = workbench.snapshot().projection.provenance.findIndex(frame => frame.uri?.endsWith('/templates/view.cemt'));
        if (index < 0) throw new Error('Supplier source provenance is missing');
        await workbench.navigateProvenance(index);
        return { resourceId: workbench.snapshot().resourceId, selection: workbench.snapshot().selection };
    });
    await settle();
    assert.equal(new URL(page.url()).searchParams.get('file'), provenance.resourceId);
    assert.equal(provenance.selection.kind, 'provenance'); await heading();
    await page.goto(`${projectUrl}&configuration=availability`); await ready();
    await page.evaluate(async () => { await navigator.serviceWorker.ready; });
    await context.setOffline(true); await page.reload(); await ready();
    assert.equal((await state()).content, 'file');
    await click('Run configuration');
    assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().status), 'projected');
    await context.setOffline(false);
    assert.deepEqual(errors, []);
    await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
    await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/navigation.json'), `${JSON.stringify({ schemaVersion: 1, urlSelection: true, history: true, modelTrees: true, sampleBrowsing: true, createCloneDelete: true, guardedDraft: true, themeRetention: true, bookmarkReload: true, supplierNativeRun: true, offline: true, layouts, forcedColors: paint }, null, 2)}\n`);
    console.log('Studio navigation: URL/history, model trees, CRUD, drafts, themes, responsive layout and offline Supplier Run passed.');
} catch (error) {
    const failure = await state().catch(() => ({}));
    console.error('Navigation failure:', JSON.stringify({ selection: failure.selection, title: failure.title, error: failure.error, status: failure.status }));
    await page.screenshot({ path: '/tmp/cem-studio-navigation-failure.png', fullPage: true }).catch(() => undefined);
    throw error;
} finally { await browser.close(); await server.close(); }

async function startStaticServer(root) {
    const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
    return new Promise((resolveServer, reject) => {
        const server = createServer(async (request, response) => {
            try {
                const requestPath = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
                const candidate = resolve(root, `.${requestPath === '/' ? '/index.html' : requestPath}`);
                const rootPrefix = root.endsWith(sep) ? root : `${root}${sep}`;
                if (candidate !== root && !candidate.startsWith(rootPrefix)) {
                    response.writeHead(403).end('Forbidden');
                    return;
                }
                const metadata = await stat(candidate);
                const path = metadata.isDirectory() ? resolve(candidate, 'index.html') : candidate;
                const bytes = await readFile(path);
                response.writeHead(200, {
                    ...security.headers,
                    'content-type': contentType(path),
                    'cache-control': 'no-store',
                    'service-worker-allowed': '/',
                });
                response.end(bytes);
            } catch {
                response.writeHead(404).end('Not found');
            }
        });
        server.on('error', reject);
        server.listen(0, '127.0.0.1', () => {
            const address = server.address();
            if (!address || typeof address === 'string') {
                reject(new Error('static server did not expose a TCP address'));
                return;
            }
            resolveServer({
                url: `http://127.0.0.1:${address.port}`,
                close: () =>
                    new Promise((resolveClose, rejectClose) => {
                        server.close((error) => (error ? rejectClose(error) : resolveClose()));
                    }),
            });
        });
    });
}

function contentType(path) {
    return (
        {
            '.css': 'text/css; charset=utf-8',
            '.html': 'text/html; charset=utf-8',
            '.js': 'text/javascript; charset=utf-8',
            '.json': 'application/json; charset=utf-8',
            '.svg': 'image/svg+xml',
            '.wasm': 'application/wasm',
            '.webmanifest': 'application/manifest+json',
        }[extname(path)] ?? 'application/octet-stream'
    );
}
