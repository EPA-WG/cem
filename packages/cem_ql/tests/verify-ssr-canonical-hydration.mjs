import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import http from 'node:http';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const temp = await mkdtemp(path.join(os.tmpdir(), 'cem-canonical-resume-'));
const source = path.join(temp, 'source.html');
const output = path.join(temp, 'rendered.html');
const builtin = path.join(root, 'packages/cem_ml_transform_cem_ql/templates/ssr-page.cemt');
const cli = path.join(root, `dist/target/cem_ml_cli/debug/cem-ml${process.platform === 'win32' ? '.exe' : ''}`);
const template = '{article | {$ datadom.slots.heading}}';
let server, browser;
try {
    await writeFile(source, `<!doctype html><html><head><title>Canonical SSR</title></head><body><cem-element tag="x-resume" version="1.2.3"><template id="x-resume" type="text/cem-ml">${template}</template></cem-element><x-resume><strong slot="heading">Island input</strong></x-resume></body></html>`);
    execFileSync(cli, ['transform', source, '--template', builtin, '--template-content-type', 'text/cem-ml',
        '--template-entrypoint', 'ssr:render-page', '--to-content-type', 'text/html', '--out', output,
        ...['page-url=https://ssr.example.test/', 'consumer-versions=x-resume=1.9.0', 'max-depth=8',
            'max-instances=8', 'max-render-passes=8', 'max-resources=16', 'max-bytes=1048576',
            'wall-ms=60000', 'cpu-ms=10000'].flatMap(value => ['--param', value])]);
    const html = await readFile(output, 'utf8');
    assert.match(html, /<article><strong\b[^>]*\bslot="heading"[^>]*>Island input<\/strong><\/article>/);
    assert.match(html, /data-cem-ssr-profile="cem-native-inline\/1"/);
    server = http.createServer(async (req, res) => {
        try {
            const pathname = new URL(req.url, 'http://localhost').pathname;
            if (pathname === '/ssr.html') { res.setHeader('Content-Type', 'text/html; charset=utf-8'); res.end(html); return; }
            const file = path.resolve(root, `.${pathname}`);
            assert.ok(file.startsWith(path.join(root, 'packages/cem-elements/dist/')));
            res.setHeader('Content-Type', pathname.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
            res.end(await readFile(file));
        } catch { res.statusCode = 404; res.end(); }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
    const requests = [], errors = [];
    const cases = ['compatible', 'missing', 'malformed', 'incompatible', 'unknown-schema', 'unknown-profile', 'tampered-range', 'stale', 'duplicate'];
    for (const scenario of cases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/ssr.html`);
        const result = await page.evaluate(async scenario => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const host = document.querySelector('x-resume');
            const declaration = document.querySelector('cem-element');
            const island = host.querySelector('template[data-cem-island]');
            const root = island.content.firstElementChild;
            const field = name => [...root.firstElementChild.children].find(f => f.getAttribute('name') === name);
            const retained = host.querySelector('article');
            const version = field('declarationVersion');
            declaration.setAttribute('version', scenario === 'incompatible' ? '2.0.0' : '1.9.0');
            if (scenario === 'missing') version.remove();
            if (scenario === 'malformed') version.textContent = 'v1';
            if (scenario === 'incompatible') {
                declaration.querySelector('template').innerHTML = '{section | {$ datadom.slots.heading}}';
                retained.querySelector('strong').textContent = 'Stale output';
            }
            if (scenario === 'unknown-profile') island.setAttribute('data-cem-ssr-profile', 'unknown/2');
            if (scenario === 'unknown-schema') { root.setAttribute('version', '0.3.0'); field('version').textContent = '0.3.0'; }
            if (scenario === 'tampered-range') retained.querySelector('strong').textContent = 'Tampered output';
            const duplicate = scenario === 'duplicate' ? host.cloneNode(true) : null;
            if (duplicate) host.after(duplicate);
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        retained.querySelector('strong').textContent = 'Changed during admission';
                        return decision;
                    } };
                };
            }
            const before = host.innerHTML;
            runtime.install(window);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            await runtime.whenRenderSettled(host);
            if (duplicate) await runtime.whenRenderSettled(duplicate);
            const retainedNow = host.querySelector('article') === retained;
            const unchanged = host.innerHTML === before;
            const text = host.querySelector('article > strong, section > strong')?.textContent;
            const diagnostics = runtime.diagnosticsFor(host).map(d => d.code);
            let reconnect = null, invalidated = null, legacy = null;
            if (scenario === 'compatible') {
                const restored = host.cloneNode(true);
                const restoredOutput = restored.querySelector('article');
                host.remove(); document.body.append(host); await runtime.whenRenderSettled(host);
                reconnect = host.querySelector('article') === retained;
                const payload = [...root.children].find(e => e.localName === 'cem-payload:payload');
                payload.querySelector('strong').textContent = 'Updated island';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(host);
                invalidated = host.querySelector('article > strong')?.textContent;
                host.remove(); document.body.append(restored); await runtime.whenRenderSettled(restored);
                legacy = {
                    retained: restored.querySelector('article') === restoredOutput,
                    marked: restored.querySelector('template[data-cem-ssr-profile]') !== null,
                    diagnostics: runtime.diagnosticsFor(restored).map(d => d.code),
                };
            }
            return { retainedNow, unchanged, text, diagnostics, reconnect, invalidated, legacy,
                duplicateRefusals: duplicate ? [host, duplicate].filter(node => runtime.diagnosticsFor(node)
                    .some(d => d.code === 'cem-element.hydration_native_refused')).length : 0 };
        }, scenario);
        if (scenario === 'compatible') {
            assert.equal(result.retainedNow, true, JSON.stringify(result)); assert.equal(result.reconnect, true);
            assert.equal(result.invalidated, 'Updated island', JSON.stringify(result)); assert.deepEqual(result.diagnostics, []);
            assert.deepEqual(result.legacy, { retained: true, marked: false, diagnostics: [] });
        } else if (['missing', 'malformed', 'incompatible'].includes(scenario)) {
            assert.equal(result.retainedNow, false, JSON.stringify(result));
            assert.equal(result.text, 'Island input');
            assert.ok(result.diagnostics.includes(`cem-element.hydration_declaration_version_${scenario === 'missing' ? 'missing' : 'incompatible'}`));
        } else if (scenario === 'duplicate') {
            assert.equal(result.duplicateRefusals, 1, JSON.stringify(result));
        } else {
            assert.equal(result.retainedNow, true); assert.ok(result.diagnostics.includes('cem-element.hydration_native_refused'), JSON.stringify(result));
            if (scenario === 'stale') assert.equal(result.text, 'Changed during admission');
            else assert.equal(result.unchanged, true);
        }
        await page.close();
    }
    assert.deepEqual(errors, []); assert.deepEqual(requests, []);
    console.log(`Canonical CLI hydration: ${cases.length}/${cases.length}, zero external requests`);
} finally {
    await browser?.close();
    if (server) await new Promise(resolve => server.close(resolve));
    await rm(temp, { recursive: true, force: true });
}
