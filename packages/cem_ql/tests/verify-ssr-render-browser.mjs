import assert from 'node:assert/strict';
import http from 'node:http';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

// Named JSON observation exports are test controls, never runtime AST handoffs.
const root = path.resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const vectors = JSON.parse(readFileSync(new URL('./fixtures/ssr-render-observations.json', import.meta.url), 'utf8'));
const native = JSON.parse(readFileSync(path.join(root, 'dist/reports/cem-ssr-render-native.json'), 'utf8'));
async function boundedObservation(promise) {
    let timer;
    try {
        return await Promise.race([promise, new Promise((_, reject) => {
            timer = setTimeout(() => reject(new Error('Browser lifecycle observation timed out')), 30_000);
        })]);
    } finally { clearTimeout(timer); }
}
const server = http.createServer(async (req, res) => {
    try {
        const pathname = new URL(req.url, 'http://localhost').pathname;
        if (pathname === '/fixture') {
            res.setHeader('Content-Type', 'text/html');
            res.end('<!doctype html><html><body></body></html>');
            return;
        }
        // Exercise the new core WASM through the existing browser runtime.
        const vendor = '/packages/cem-elements/dist/lib/internal/runtime-support/vendor/';
        const local = pathname.startsWith(vendor) && ['cem_ql.js', 'cem_ql_bg.wasm'].includes(path.basename(pathname))
            ? `/packages/cem_ql/dist/wasm/${path.basename(pathname)}` : pathname;
        const file = path.resolve(root, `.${decodeURIComponent(local)}`);
        assert.ok(file.startsWith(`${root}${path.sep}`));
        const data = await readFile(file);
        const type = { '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm' }[path.extname(file)];
        res.setHeader('Content-Type', type || 'application/octet-stream');
        res.end(data);
    } catch {
        res.writeHead(404);
        res.end('missing');
    }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
    const origin = `http://127.0.0.1:${server.address().port}`;
    const externalRequests = [];
    const errors = [];
    const results = [];
    for (const vector of vectors) {
        const page = await browser.newPage();
        page.setDefaultTimeout(30_000);
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', async route => {
            if (new URL(route.request().url()).origin === origin) await route.continue();
            else {
                externalRequests.push(route.request().url());
                await route.abort();
            }
        });
        await page.goto(`${origin}/fixture`);
        const observation = await boundedObservation(page.evaluate(async vector => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const runtime = new CemElementRuntime({
                declarationTag: 'test-declaration',
                processingWorkerFactory: () => { throw new Error('Use existing main-thread runtime'); },
            });
            runtime.install(window);
            const container = document.createElement('div');
            container.innerHTML = `<test-declaration tag="x-inline"><template id="x-inline" type="text/cem-ml">${vector.template}</template></test-declaration>${vector.body}`;
            document.body.append(container);
            const declaration = container.querySelector('test-declaration');
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const host = container.querySelector('x-inline');
            await runtime.whenRenderSettled(host);
            const internal = new Set(['data-cem-render-scope', 'data-cem-render-node-id', 'data-cem-template-artifact-id', 'data-cem-data-revision', 'data-cem-source-fidelity', 'data-cem-source-frame']);
            function exportObservation(node) {
                if (node.nodeType === 3) return node.textContent;
                if (node.nodeType === 8) return { comment: node.textContent };
                if (node.nodeType !== 1) throw new Error('Unsupported observation node');
                if (node.namespaceURI !== 'http://www.w3.org/1999/xhtml') throw new Error('Unexpected output namespace');
                const attributes = Object.fromEntries(Array.from(node.attributes)
                    .filter(attr => !internal.has(attr.name) && !attr.name.startsWith('xmlns'))
                    .map(attr => [attr.name, attr.value]));
                return { tag: node.localName, attributes, children: Array.from(node.childNodes).filter(child => child.nodeType !== 3 || child.textContent !== '').map(exportObservation) };
            }
            const children = Array.from(host.childNodes);
            const start = children.findIndex(node => node.nodeType === 8 && node.textContent === 'cem-render-start');
            const end = children.findIndex(node => node.nodeType === 8 && node.textContent === 'cem-render-end');
            if (start !== 1 || end !== children.length - 1) throw new Error('Unexpected island/range topology');
            const island = children[0];
            if (island.localName !== 'template' || island.getAttribute('data-cem-island') !== 'instance') throw new Error('Missing inert island');
            const context = island.content.firstElementChild;
            const parts = Array.from(context.children).map(node => node.localName.split(':').at(-1));
            const payload = Array.from(context.children).find(node => node.localName.endsWith(':payload'));
            const hostState = {};
            if (vector.hostAttributes) {
                hostState.hostAttributes = Object.fromEntries(Array.from(host.attributes).filter(a => !internal.has(a.name)).map(a => [a.name, a.value]));
                const section = name => Array.from(context.children).find(node => node.localName.endsWith(`:${name}`));
                hostState.islandAttributes = Object.fromEntries(Array.from(section('attributes').children).filter(node => !internal.has(node.getAttribute('name'))).map(node => [node.getAttribute('name'), node.getAttribute('value')]));
                hostState.islandDataset = Object.fromEntries(Array.from(section('dataset').children).filter(node => node.getAttribute('key') !== 'cemRenderScope').map(node => [node.getAttribute('key'), node.getAttribute('value')]));
            }
            return {
                ...hostState,
                name: vector.name,
                rendered: children.slice(start + 1, end).map(exportObservation),
                payload: Array.from(payload.childNodes).map(exportObservation),
                parts,
                diagnostics: runtime.diagnosticsFor(host),
            };
        }, vector));
        assert.deepEqual(observation.parts, ['data', 'attributes', 'dataset', 'payload', 'slices', 'resources', 'form-state', 'validation-state', 'event-state']);
        assert.deepEqual(observation.diagnostics.map(entry => entry.code), vector.browserDiagnostics || [], vector.name);
        const result = { name: observation.name, rendered: observation.rendered, payload: observation.payload };
        const expected = { name: vector.name, rendered: vector.browserRendered || vector.rendered, payload: vector.payload };
        const nativeExpected = { name: vector.name, rendered: vector.rendered, payload: vector.payload };
        if (vector.hostAttributes) for (const field of ['hostAttributes', 'islandAttributes', 'islandDataset']) {
            result[field] = observation[field]; expected[field] = vector[`browser${field[0].toUpperCase()}${field.slice(1)}`] || vector[field]; nativeExpected[field] = vector[field];
        }
        assert.deepEqual(result, expected, vector.name);
        const nativeResult = native.find(entry => entry.name === vector.name);
        assert.deepEqual(nativeResult, nativeExpected, vector.name);
        if (vector.divergence) {
            assert.notDeepEqual(result, nativeResult, vector.name);
            assert.deepEqual(result.payload, nativeResult.payload, vector.name);
        } else assert.deepEqual(result, nativeResult, vector.name);
        results.push({ ...result, divergence: vector.divergence || null });
        await page.close();
    }
    assert.deepEqual(externalRequests, []);
    assert.deepEqual(errors, []);
    const report = path.join(root, 'dist/reports/cem-ssr-render-browser.json');
    mkdirSync(path.dirname(report), { recursive: true });
    writeFileSync(report, `${JSON.stringify({ browser: browser.version(), authoredJavaScript: false, trustedRuntime: true, ignoresEmptyTextPlaceholders: true, externalRequests, results }, null, 2)}\n`);
    console.log(`Native/browser observations: ${results.length}; matching full observations: ${results.filter(entry => !entry.divergence).length}; documented divergences: ${results.filter(entry => entry.divergence).length}`);
} finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
}
