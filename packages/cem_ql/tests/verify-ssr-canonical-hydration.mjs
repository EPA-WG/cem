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
async function render(body, versions) {
    await writeFile(source, `<!doctype html><html><head><title>Canonical SSR</title></head><body>${body}</body></html>`);
    execFileSync(cli, ['transform', source, '--template', builtin, '--template-content-type', 'text/cem-ml',
        '--template-entrypoint', 'ssr:render-page', '--to-content-type', 'text/html', '--out', output,
        ...['page-url=https://ssr.example.test/', `consumer-versions=${versions}`, 'max-depth=8',
            'max-instances=8', 'max-render-passes=8', 'max-resources=16', 'max-bytes=1048576',
            'wall-ms=60000', 'cpu-ms=10000'].flatMap(value => ['--param', value])]);
    return readFile(output, 'utf8');
}
let server, browser;
try {
    const html = await render(`<cem-element tag="x-resume" version="1.2.3"><template id="x-resume" type="text/cem-ml">${template}</template></cem-element><x-resume><strong slot="heading">Island input</strong></x-resume>`, 'x-resume=1.9.0');
    const childSource = '{x-child | {em @slot=heading | Nested input}}';
    const renderNested = parentSource => render([
        `<cem-element tag="x-parent" version="1.2.3"><template id="x-parent" type="text/cem-ml">${parentSource}</template></cem-element>`,
        `<cem-element tag="x-child" version="1.2.3"><template id="x-child" type="text/cem-ml">${template}</template></cem-element>`,
        '<x-parent></x-parent>',
    ].join(''), 'x-parent=1.9.0,x-child=1.9.0');
    const nested = await renderNested(`{section | ${childSource}}`);
    const direct = await renderNested(childSource);
    const pages = new Map([['/ssr.html', html], ['/nested.html', nested], ['/direct.html', direct]]);
    assert.match(html, /<article><strong\b[^>]*\bslot="heading"[^>]*>Island input<\/strong><\/article>/);
    assert.match(html, /data-cem-ssr-profile="cem-native-inline\/1"/);
    assert.match(nested, /<article><em\b[^>]*\bslot="heading"[^>]*>Nested input<\/em><\/article>/);
    server = http.createServer(async (req, res) => {
        try {
            const pathname = new URL(req.url, 'http://localhost').pathname;
            if (pages.has(pathname)) {
                res.setHeader('Content-Type', 'text/html; charset=utf-8');
                res.end(pages.get(pathname));
                return;
            }
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
    const accepted = ['compatible', 'child-first', 'direct-root'];
    const nestedCases = [...accepted, 'child-version', 'child-output', 'child-payload', 'child-marker', 'duplicate-id', 'stale-child', 'stale-declaration'];
    for (const scenario of nestedCases) {
        const page = await browser.newPage();
        page.on('pageerror', error => errors.push(error.message));
        await page.route('**/*', route => {
            if (new URL(route.request().url()).origin === origin) return route.continue();
            requests.push(route.request().url()); return route.abort();
        });
        await page.goto(`${origin}/${scenario === 'direct-root' ? 'direct' : 'nested'}.html`);
        const result = await page.evaluate(async ({ scenario, adopt }) => {
            const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
            const parent = document.querySelector('x-parent');
            const child = parent.querySelector('x-child');
            const hosts = [parent, child];
            const islands = hosts.map(host => host.querySelector(':scope > template[data-cem-island]'));
            const field = (island, name) => island.content.querySelector(`[name="${name}"]`);
            const payload = islands[1].content.querySelector('em');
            const section = parent.querySelector('section');
            const article = child.querySelector('article');
            const projected = article.querySelector('em');
            if (scenario === 'child-first') document.body.prepend(document.querySelector('cem-element[tag="x-child"]'));
            if (scenario === 'child-version') field(islands[1], 'declarationVersion').textContent = '2.0.0';
            if (scenario === 'child-output') projected.textContent = 'Changed output';
            if (scenario === 'child-payload') payload.textContent = projected.textContent = 'Changed input';
            if (scenario === 'child-marker') islands[1].removeAttribute('data-cem-ssr-profile');
            if (scenario === 'duplicate-id') field(islands[1], 'instanceId').textContent = field(islands[0], 'instanceId').textContent;
            const before = parent.innerHTML;
            const runtime = new CemElementRuntime({ processingWorkerFactory: () => { throw new Error('Use main-thread runtime'); } });
            if (scenario === 'stale-child') {
                const support = runtime.renderSupport.bind(runtime);
                runtime.renderSupport = scope => {
                    const bindings = support(scope);
                    return { ...bindings, admitCanonicalSsrInstance: async (...args) => {
                        const decision = await bindings.admitCanonicalSsrInstance(...args);
                        payload.textContent = 'Changed during admission';
                        return decision;
                    } };
                };
            }
            runtime.install(window);
            if (scenario === 'stale-declaration') document.querySelector('cem-element[tag="x-child"]').setAttribute('version', '2.0.0');
            await Promise.all([...document.querySelectorAll('cem-element')].map(node => runtime.whenDeclarationSettled(node)));
            await Promise.all(hosts.map(host => runtime.whenRenderSettled(host)));
            const unchanged = parent.innerHTML === before;
            const retained = parent.querySelector('section') === section && parent.querySelector('x-child') === child
                && child.querySelector('article') === article && article.querySelector('em') === projected;
            const marks = islands.map(island => island.hasAttribute('data-cem-ssr-profile'));
            const diagnostics = hosts.map(host => runtime.diagnosticsFor(host).map(diagnostic => diagnostic.code));
            const leakedMetadata = islands[1].content.querySelector('[name="data-cem-template-artifact-id"]') !== null;
            let updated = null;
            if (adopt) {
                payload.textContent = 'Live child input';
                await new Promise(resolve => setTimeout(resolve, 0));
                await runtime.whenRenderSettled(child);
                updated = child.querySelector('article em')?.textContent;
            }
            return { unchanged, retained, marks, diagnostics, leakedMetadata, updated };
        }, { scenario, adopt: accepted.includes(scenario) });
        const detail = `${scenario} ${JSON.stringify(result)}`;
        assert.equal(result.retained, true, detail);
        if (accepted.includes(scenario)) {
            assert.deepEqual(result.marks, [false, false], detail);
            assert.deepEqual(result.diagnostics, [[], []], detail);
            assert.equal(result.leakedMetadata, false, detail);
            assert.equal(result.updated, 'Live child input', detail);
        } else {
            assert.deepEqual(result.marks, [true, scenario !== 'child-marker'], detail);
            assert.ok(result.diagnostics[0].includes('cem-element.hydration_native_refused'), detail);
            if (scenario !== 'child-marker') assert.ok(result.diagnostics[1].includes('cem-element.hydration_native_refused'), detail);
            if (scenario !== 'stale-child') assert.equal(result.unchanged, true, detail);
        }
        await page.close();
    }
    assert.deepEqual(errors, []); assert.deepEqual(requests, []);
    const total = cases.length + nestedCases.length;
    console.log(`Canonical CLI hydration: ${total}/${total}, including ${nestedCases.length} nested scenarios, zero external requests`);
} finally {
    await browser?.close();
    if (server) await new Promise(resolve => server.close(resolve));
    await rm(temp, { recursive: true, force: true });
}
