import assert from 'node:assert/strict';
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
// Explicit JSON observations only; runtime inputs remain native retained trees.
const root = path.resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const vectors = JSON.parse(readFileSync(new URL('./fixtures/ssr-page-observations.json', import.meta.url), 'utf8'));
const native = JSON.parse(readFileSync(path.join(root, 'dist/reports/cem-ssr-page-native.json'), 'utf8'));
const server = http.createServer(async (req, res) => {
    try {
        const url = new URL(req.url, 'http://localhost');
        if (url.pathname === '/fixture') { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><html><body></body></html>'); return; }
        const vendor = '/packages/cem-elements/dist/lib/internal/runtime-support/vendor/';
        const local = url.pathname.startsWith(vendor) && ['cem_ql.js', 'cem_ql_bg.wasm'].includes(path.basename(url.pathname))
            ? `/packages/cem_ql/dist/wasm/${path.basename(url.pathname)}` : url.pathname;
        const file = path.resolve(root, `.${decodeURIComponent(local)}`);
        assert.ok(file.startsWith(`${root}${path.sep}`));
        res.setHeader('Content-Type', { '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm' }[path.extname(file)] || 'application/octet-stream');
        res.end(await readFile(file));
    } catch { res.writeHead(404); res.end('missing'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
    const origin = `http://127.0.0.1:${server.address().port}`;
    const results = []; const errors = []; const externalRequests = [];
    for (const vector of vectors) {
        const page = await browser.newPage(); page.setDefaultTimeout(30_000);
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', async route => {
            if (new URL(route.request().url()).origin === origin) await route.continue();
            else { externalRequests.push(route.request().url()); await route.abort(); }
        });
        await page.goto(`${origin}/fixture`);
        const result = await page.evaluate(async vector => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const runtime = new CemElementRuntime({ declarationTag: 'test-declaration', processingWorkerFactory: () => { throw new Error('Use existing main-thread runtime'); } });
            runtime.install(window);
            document.body.innerHTML = vector.declarations.map(d => `<test-declaration tag="${d.tag}"><template id="${d.tag}" type="text/cem-ml">${d.template}</template></test-declaration>`).join('') + vector.body;
            for (const declaration of document.querySelectorAll('test-declaration')) { runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration); }
            const selector = vector.declarations.map(d => d.tag).join(',');
            let previous = [];
            for (let pass = 0; pass < 16; pass++) {
                const active = Array.from(document.querySelectorAll(selector));
                await Promise.all(active.map(host => runtime.whenRenderSettled(host)));
                const current = Array.from(document.querySelectorAll(selector));
                if (current.length === previous.length && current.every((host, index) => host === previous[index])) break;
                previous = current;
                if (pass === 15) throw new Error('Browser participant discovery did not settle');
            }
            function snapshot(node, rendered, publicOutput = rendered) {
                if (node.nodeType === 3) return node.textContent === '' ? undefined : node.textContent;
                if (node.nodeType === 8) return rendered && ['cem-render-start', 'cem-render-end'].includes(node.textContent) ? undefined : { comment: node.textContent };
                if (node.nodeType !== 1) throw new Error('Unsupported observation node');
                if (rendered && node.localName === 'template' && node.getAttribute('data-cem-island') === 'instance') return undefined;
                const attributes = Object.fromEntries(Array.from(node.attributes).filter(a => (!publicOutput || !a.name.startsWith('data-cem-')) && a.namespaceURI !== 'http://www.w3.org/2000/xmlns/' && a.name !== 'xmlns' && !a.name.startsWith('xmlns:')).map(a => [a.name, a.value]));
                const content = node.localName === 'template' ? node.content.childNodes : node.childNodes;
                const children = Array.from(content).map(child => snapshot(child, rendered && node.localName !== 'template', publicOutput)).filter(value => value !== undefined);
                return { tag: node.localName, attributes, children, ...(node.namespaceURI === 'http://www.w3.org/1999/xhtml' ? {} : { namespace: node.namespaceURI }) };
            }
            const hosts = Array.from(document.querySelectorAll(selector)).map(host => {
                const island = Array.from(host.children).find(node => node.localName === 'template' && node.getAttribute('data-cem-island') === 'instance');
                if (!island) throw new Error('Missing active participant island');
                const payload = Array.from(island.content.firstElementChild.children).find(node => node.localName.endsWith(':payload'));
                if (!payload) throw new Error('Missing canonical participant payload');
                if (runtime.diagnosticsFor(host).length) throw new Error(`Unexpected participant diagnostics: ${runtime.diagnosticsFor(host).map(d => d.code).join(',')}`);
                return { host: snapshot(host, true), payload: Array.from(payload.childNodes).map(node => snapshot(node, false)).filter(value => value !== undefined) };
            });
            return { name: vector.name, hosts };
        }, vector);
        assert.deepEqual(result, native.find(entry => entry.name === vector.name), vector.name);
        results.push(result); await page.close();
    }
    assert.deepEqual(errors, []); assert.deepEqual(externalRequests, []);
    writeFileSync(path.join(root, 'dist/reports/cem-ssr-page-browser.json'), `${JSON.stringify({ browser: browser.version(), externalRequests, results }, null, 2)}\n`);
    console.log(`Native/browser passive pages: ${results.length}/${vectors.length} complete matches`);
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
