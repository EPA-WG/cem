import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';
import { parseCemStudioProjectBundle } from '../src/file-system-provider.js';
import { captureStudioRenderFailure } from './studio-browser-diagnostics.mjs';
import { catalogAcceptanceStamp } from './catalog-acceptance.mjs';

/** Real saved-project metadata must not prevent the shared editor from rendering. */
export async function verifyPopulatedWorkspace({ page, context, url }) {
    const output = new URL('../dist/static/samples/', import.meta.url);
    const index = JSON.parse(await readFile(new URL('index.json', output), 'utf8'));
    const expected = await Promise.all(index.projects.map(async record => {
        const bytes = await readFile(new URL(record.archive, output));
        assert.equal(digest(bytes), record.sha256, `${record.id}: archive bytes`);
        const bundle = parseCemStudioProjectBundle(bytes);
        const hashes = bundle.project.resources.map(resource => [resource.id, digest(bundle.contents[resource.id])]).sort();
        return { record, bundle, hashes };
    }));
    const pageErrors = [];
    const onPageError = error => { if (pageErrors.length < 20) pageErrors.push(error.message.slice(0, 500)); };
    page.on('pageerror', onPageError);
    let phase = 'startup';
    const bounded = async (operation, label, timeout = 30_000) => {
        let timer;
        try {
            return await Promise.race([operation, new Promise((_, reject) => {
                timer = setTimeout(() => reject(new Error(`Populated workspace ${label} exceeded ${timeout} ms`)), timeout);
            })]);
        } finally { clearTimeout(timer); }
    };
    const observe = (callback, argument) => bounded(page.evaluate(callback, argument), 'observation');
    const settle = () => observe(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
    const ready = async () => { await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication)); await settle(); };
    const query = operation => observe(async operation => (await globalThis.__cemStudioApplication.repository.query({
        protocolVersion: 1, repository: 'studio-projects', requestRevision: 1, operation, parameters: { includeTrash: true },
    })).value, operation);
    const hashes = (sample, savedId) => observe(async ({ sample, savedId }) => {
        const app = globalThis.__cemStudioApplication;
        const bundle = savedId ? (await app.repository.query({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
            operation: 'export-project', parameters: { projectId: savedId } })).value
            : await app.exampleProjects.examples.find(record => record.id === sample).load();
        return { id: bundle.project.id, rootUri: bundle.project.rootUri, entries: bundle.project.entries.map(entry => entry.id),
            resources: await Promise.all(bundle.project.resources.map(async resource => {
                const source = bundle.contents[resource.id], bytes = source instanceof Uint8Array ? source : new Uint8Array(source);
                const hash = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(byte => byte.toString(16).padStart(2, '0')).join('');
                return [resource.id, hash];
            })) };
    }, { sample, savedId });
    try {
        console.log('[cem-studio:populated-workspace] opening fresh isolated profile');
        await context.setOffline(false); await page.goto(`${url}/`); await ready();
        assert.deepEqual(await query('list-projects'), [], 'Use a fresh isolated profile; never clear existing projects');
        assert.equal(await observe(() => Object.keys(localStorage).filter(key => key.startsWith('cem-studio-selection-v1:draft-v1:')).length), 0,
            'Use a fresh isolated profile; never clear existing drafts');
        const copies = [], roots = new Set();
        phase = 'seed-catalog-copies';
        for (const item of expected) {
            assert.deepEqual((await hashes(item.record.id)).resources.sort(), item.hashes, `${item.record.id}: original bytes before seeding`);
            for (let ordinal = 1; ordinal <= 2; ordinal++) {
                const id = `project-${randomUUID()}`;
                await observe(async ({ sample, id, ordinal }) => {
                    const app = globalThis.__cemStudioApplication;
                    const bundle = structuredClone(await app.exampleProjects.examples.find(record => record.id === sample).load());
                    const now = new Date().toISOString();
                    bundle.project = { ...bundle.project, id, rootUri: `studio://${id}/`, name: `Populated ${bundle.project.name} ${ordinal}`,
                        revision: 1, createdAt: now, updatedAt: now };
                    await app.repository.execute({ protocolVersion: 1, repository: 'studio-projects', requestRevision: 1,
                        operation: 'import-project', parameters: { bundle, mode: 'create' } });
                }, { sample: item.record.id, id, ordinal });
                const copy = await hashes(item.record.id, id);
                assert.equal(copy.id, id); assert.equal(copy.rootUri, `studio://${id}/`);
                assert.notEqual(copy.rootUri, item.bundle.project.rootUri); assert.equal(roots.has(copy.rootUri), false); roots.add(copy.rootUri);
                assert.deepEqual(copy.entries, item.bundle.project.entries.map(entry => entry.id));
                assert.deepEqual(copy.resources.sort(), item.hashes, `${item.record.id}: copied resource bytes`);
                copies.push({ sample: item.record.id, projectId: id });
            }
            console.log(`[cem-studio:populated-workspace] seeded ${copies.length}/${expected.length * 2} independent projects`);
        }
        assert.equal((await query('list-projects')).length, expected.length * 2);
        phase = 'reload-populated-workspace';
        console.log(`[cem-studio:populated-workspace] reloading with ${copies.length} saved projects`);
        await page.reload(); await ready();
        const projectId = copies.find(copy => copy.sample === 'module-maps').projectId;
        phase = 'open-module-map-validation';
        await observe(route => globalThis.__cemStudioApplication.workbench.openEntity(new URLSearchParams(route).toString()),
            { project: projectId, configuration: 'validate-v1' });
        await settle();
        const selected = await observe(() => {
            const state = globalThis.__cemStudioApplication.workbench.snapshot();
            return { projectId: state.projectId, entryId: state.entryId, resourceId: state.resourceId, editable: state.editable, dirty: state.dirty };
        });
        assert.deepEqual(selected, { projectId, entryId: 'validate-v1', resourceId: 'source-v1', editable: true, dirty: false });
        phase = 'editor-and-validation-attached';
        const editor = page.getByRole('textbox', { name: 'CEM source', exact: true, includeHidden: true });
        const validate = page.getByRole('button', { name: 'Validate persisted revision', exact: true, includeHidden: true });
        await editor.waitFor({ state: 'attached' }); await validate.waitFor({ state: 'attached' });
        assert.equal(await editor.isVisible(), true);
        const details = page.getByRole('button', { name: 'Editor details', exact: true });
        if (await details.getAttribute('aria-expanded') === 'false') { await details.click(); await settle(); }
        assert.equal(await validate.isVisible(), true); assert.equal(await validate.isDisabled(), false);
        const source = expected.find(item => item.record.id === 'module-maps').bundle.contents['source-v1'];
        assert.equal(digest(Buffer.from(await bounded(editor.evaluate(element => element.value), 'editor value'), 'utf8')), digest(source));
        phase = 'retained-project-integrity';
        for (const item of expected) {
            assert.deepEqual((await hashes(item.record.id)).resources.sort(), item.hashes, `${item.record.id}: original bytes after rendering`);
            for (const copy of copies.filter(copy => copy.sample === item.record.id)) {
                assert.deepEqual((await hashes(item.record.id, copy.projectId)).resources.sort(), item.hashes, `${item.record.id}: saved bytes after rendering`);
            }
        }
        assert.equal(await observe(() => Object.keys(localStorage).filter(key => key.startsWith('cem-studio-selection-v1:draft-v1:')).length), 0,
            'Readiness checks must not create source drafts');
        assert.deepEqual(pageErrors, []);
        console.log('[cem-studio:populated-workspace] shared editor and validation control ready; all original/copy bytes unchanged');
        return { catalogProjects: expected.length, copiesPerProject: 2, savedProjects: copies.length,
            savedResources: expected.reduce((count, item) => count + item.bundle.project.resources.length * 2, 0),
            selectedProjectId: projectId, configuration: 'validate-v1', resourceId: 'source-v1',
            editorAttached: true, validationAttached: true, validationEnabled: true,
            originalsUnchanged: true, copiesUnchanged: true, noDraftsCreated: true };
    } catch (error) {
        let evidence;
        try {
            evidence = await bounded(captureStudioRenderFailure(page, pageErrors), 'failure diagnostics', 5_000);
        } catch (diagnosticError) {
            evidence = { pageErrors, diagnostics: [{ code: 'test.diagnostics_unavailable', message: diagnosticError.message.slice(0, 500) }] };
        }
        console.error('Populated workspace failure evidence:', JSON.stringify({ phase, ...evidence }));
        throw error;
    } finally { page.off('pageerror', onPageError); }
}

function digest(bytes) { return createHash('sha256').update(bytes).digest('hex'); }

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const projectRoot = resolve(import.meta.dirname, '..');
    const stamp = await catalogAcceptanceStamp(resolve(projectRoot, '../..'));
    const server = await staticServer(resolve(projectRoot, 'dist/static'));
    const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
    try {
        const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
        const page = await context.newPage(); page.setDefaultTimeout(30_000);
        const evidence = await verifyPopulatedWorkspace({ page, context, url: server.url });
        assert.deepEqual(await catalogAcceptanceStamp(resolve(projectRoot, '../..')), stamp, 'Deployment changed during populated-workspace acceptance');
        await mkdir(resolve(projectRoot, '../../dist/reports/cem-studio'), { recursive: true });
        await writeFile(resolve(projectRoot, '../../dist/reports/cem-studio/populated-workspace.json'), `${JSON.stringify({ schemaVersion: 1, ...stamp, evidence }, null, 2)}\n`);
    } finally { await browser.close(); await server.close(); }
}

async function staticServer(root) {
    const security = JSON.parse(await readFile(resolve(root, 'security-headers.json'), 'utf8'));
    const server = createServer(async (request, response) => {
        try {
            const requestPath = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
            const candidate = resolve(root, `.${requestPath === '/' ? '/index.html' : requestPath}`);
            if (candidate !== root && !candidate.startsWith(`${root}${sep}`)) { response.writeHead(403).end('Forbidden'); return; }
            const metadata = await stat(candidate), path = metadata.isDirectory() ? resolve(candidate, 'index.html') : candidate;
            const bytes = await readFile(path);
            response.writeHead(200, { ...security.headers, 'content-type': contentType(path), 'cache-control': 'no-store', 'service-worker-allowed': '/' }).end(bytes);
        } catch { response.writeHead(404).end('Not found'); }
    });
    await new Promise((resolveServer, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolveServer); });
    const address = server.address(); assert.ok(address && typeof address !== 'string');
    return { url: `http://127.0.0.1:${address.port}`, close: () => new Promise((resolveClose, reject) => server.close(error => error ? reject(error) : resolveClose())) };
}

function contentType(path) {
    return { '.css': 'text/css; charset=utf-8', '.html': 'text/html; charset=utf-8', '.js': 'text/javascript; charset=utf-8',
        '.json': 'application/json; charset=utf-8', '.svg': 'image/svg+xml', '.wasm': 'application/wasm',
        '.webmanifest': 'application/manifest+json' }[extname(path)] ?? 'application/octet-stream';
}
