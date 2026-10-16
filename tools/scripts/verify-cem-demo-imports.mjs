import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, readdir } from 'node:fs/promises';
import { extname, resolve } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(import.meta.dirname, '../..');
const componentRoot = '@epa-wg/cem-components/components/';
const mime = { '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png' };
const selected = process.argv.find(value => value.startsWith('--pages='))?.slice('--pages='.length).split(',');
const pages = (await readdir(resolve(root, 'packages/cem-components/playgrounds')))
    .filter(name => name.endsWith('.html') && (!selected || selected.includes(name))).sort();
assert.ok(pages.length, 'No demo pages selected');
const fixtures = [];

for (const directory of ['playgrounds', 'dist']) {
    for (const name of pages) {
        const path = `packages/cem-components/${directory}/${name}`;
        const source = await readFile(resolve(root, path), 'utf8');
        const imports = JSON.parse(source.match(/<script type="importmap">([\s\S]*?)<\/script>/u)?.[1] ?? '{}').imports;
        assert.ok(imports?.[componentRoot]?.endsWith('/'), `${path}: missing component folder mapping`);
        assert.deepEqual(Object.keys(imports).filter(key => key.startsWith(componentRoot) && key !== componentRoot), [], `${path}: individual component aliases remain`);
        const declarations = [...source.matchAll(/\{cem-element\s+@tag=([\w-]+)((?:[^{}"']|"[^"]*"|'[^']*')*)/gu)].flatMap(match => {
            const src = match[2].match(/@src="([^"]+)"/u)?.[1];
            if (src === undefined) return [];
            assert.ok(!src.includes('$datadom.slices'), `${path}: declaration ${match[1]} still waits for a URL slice`);
            return [{ tag: match[1], src }];
        });
        assert.ok(declarations.length, `${path}: no direct declaration imports`);
        for (const { src } of declarations) {
            const [specifier, fragment] = src.split('#');
            assert.ok(fragment, `${path}: declaration has no fragment: ${src}`);
            let target = imports[specifier];
            if (!target) {
                const prefix = Object.keys(imports).filter(key => key.endsWith('/') && specifier.startsWith(key)).sort((a, b) => b.length - a.length)[0];
                assert.ok(prefix, `${path}: unmapped declaration source ${src}`);
                target = imports[prefix] + specifier.slice(prefix.length);
            }
            const document = await readFile(resolve(root, path, '..', target), 'utf8');
            assert.ok(document.includes(`id="${fragment}"`), `${path}: missing ${src} definition`);
        }
        fixtures.push({ path, tags: [...new Set(declarations.map(({ tag }) => tag))], bundle: name === 'cem-bundle.html', runtime: source.includes('window.cemPlaygroundRuntime') });
    }
}

if (!selected) {
    const path = 'packages/cem-components/index.html';
    const source = await readFile(resolve(root, path), 'utf8');
    const tags = [...source.matchAll(/<cem-element\b[^>]*\btag="([\w-]+)"[^>]*\bsrc="[^"]+"/gu)].map(match => match[1]);
    assert.ok(tags.length, 'Component overview has no source declarations');
    fixtures.push({ path, tags: [...new Set(tags)], navigation: false });
    fixtures.push({ path: 'packages/custom-element/material/components/action.html', tags: ['cem-action'], navigation: false, runtime: false });
}

const server = createServer(async (request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname)
        .replace(/^\/packages\/cem-ml\//u, '/packages/cem-ml-npm/');
    const file = resolve(root, `.${pathname}`);
    if (!file.startsWith(root + '/')) { response.writeHead(403).end(); return; }
    try {
        const bytes = await readFile(file);
        response.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' }).end(bytes);
    } catch { response.writeHead(404).end(); }
});
let browser;
let completed = 0;
let cursor = 0;
const failures = [];
try {
    await new Promise(accept => server.listen(0, '127.0.0.1', accept));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true });
    await Promise.all(Array.from({ length: 3 }, async () => {
        const context = await browser.newContext();
        try {
            while (cursor < fixtures.length) {
                const fixture = fixtures[cursor++];
                const page = await context.newPage();
                const errors = [];
                const bundleRequests = [];
                page.on('pageerror', error => errors.push(error.message));
                page.on('response', response => {
                    if (response.status() >= 400 && response.url().startsWith(origin)
                        && /\.(?:xhtml|js|wasm|css)(?:\?|$)/u.test(response.url())) errors.push(`HTTP ${response.status()}: ${response.url()}`);
                });
                page.on('request', request => {
                    if (new URL(request.url()).pathname.endsWith('/components.xhtml')) bundleRequests.push(request.url());
                });
                try {
                    await page.goto(`${origin}/${fixture.path}`);
                    await page.waitForFunction(({ tags, runtime }) => {
                        return (runtime === false || Boolean(window.cemPlaygroundRuntime))
                            && tags.every(tag => customElements.get(tag));
                    }, fixture, { timeout: 60_000 });
                    if (fixture.navigation !== false) await page.getByRole('navigation', { name: 'Gallery navigation' }).waitFor();
                    const diagnostics = await page.evaluate(async tags => {
                        const runtime = window.cemPlaygroundRuntime;
                        if (!runtime) return [];
                        const declarations = [...document.querySelectorAll('cem-element[tag][src]')];
                        await Promise.all(declarations.map(node => runtime.whenDeclarationSettled(node)));
                        const pageImports = declarations.filter(node => !node.closest('cem-demo-element')
                            && !node.parentElement?.closest(tags.join(',')));
                        const observed = new Set(pageImports.map(node => node.getAttribute('tag')));
                        const missing = tags.filter(tag => !observed.has(tag));
                        if (missing.length) throw new Error(`Page import declarations missing: ${missing.join(', ')}`);
                        return declarations.flatMap(node => runtime.diagnosticsFor(node)
                            // Existing component-owned dependency repeats are outside page import ownership.
                            // Every page-level diagnostic and every nested acquisition/selection failure is checked.
                            .filter(value => pageImports.includes(node) || value.code !== 'cem-element.registry_same_scope_duplicate')
                            .map(value => ({ tag: node.getAttribute('tag'), src: node.getAttribute('src'), ...value })));
                    }, fixture.tags);
                    assert.deepEqual(diagnostics, [], `${fixture.path}: declaration diagnostics`);
                    assert.deepEqual(errors, [], `${fixture.path}: browser errors`);
                    if (fixture.bundle) assert.equal(bundleRequests.length, 1, `${fixture.path}: bundle fragments must share one document fetch`);
                    completed++;
                    if (completed % 20 === 0) console.log(`Verified ${completed}/${fixtures.length} source/package demo pages.`);
                } catch (error) {
                    const state = await page.evaluate(tags => ({
                        missing: tags.filter(tag => !customElements.get(tag)),
                        diagnostics: [...document.querySelectorAll('cem-element')].flatMap(node => window.cemPlaygroundRuntime?.diagnosticsFor(node) ?? []),
                    }), fixture.tags).catch(() => ({}));
                    failures.push({ path: fixture.path, error: error.message, errors, state });
                    console.error(`Failed ${fixture.path}: ${JSON.stringify(failures.at(-1))}`);
                } finally { await page.close(); }
            }
        } finally { await context.close(); }
    }));
    assert.deepEqual(failures, [], 'Demo import verification failures');
    console.log(`Verified ${completed} source/package demo pages: folder maps, document fragments, component registration, gallery navigation and shared bundle loading.`);
} finally {
    await browser?.close();
    await new Promise(accept => server.close(accept));
}
