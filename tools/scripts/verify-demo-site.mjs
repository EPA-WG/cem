#!/usr/bin/env node

import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, readdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
import { DEMO_FILE_LIMIT, listDemoFiles, readDemoInventory } from './demo-deployment.mjs';

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const destination = resolve(process.argv[2] ?? join(homedir(), 'cem-bin'));
const url = process.argv[3] ?? 'http://127.0.0.1:8081/';
const inventory = await readDemoInventory(destination);
assert.ok(inventory, 'Deployment inventory is required.');
const totalFiles = await listDemoFiles(destination);
assert.ok(totalFiles.length <= DEMO_FILE_LIMIT, `${totalFiles.length} files exceeds ${DEMO_FILE_LIMIT}`);
for (const entry of inventory.files) {
    const bytes = await readFile(join(destination, entry.path));
    assert.equal(bytes.length, entry.bytes, entry.path);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.sha256, entry.path);
    assert.ok(!entry.path.startsWith('packages/cem-theme/dist/vendor/'), entry.path);
    assert.ok(!(entry.path.startsWith('packages/cem-components/dist/') && entry.path.endsWith('.html')), entry.path);
}
console.log(`Demo inventory: ${inventory.fileCount} managed files, ${totalFiles.length} total; all owned hashes match.`);
async function compare(source, target) {
    assert.deepEqual(await readFile(join(destination, target)), await readFile(join(repo, source)), target);
}
async function studioFiles(path = '') {
    let count = 0;
    for (const entry of await readdir(join(repo, 'packages/cem-studio/dist/static', path), { withFileTypes: true })) {
        const relative = path ? `${path}/${entry.name}` : entry.name;
        if (entry.isDirectory()) count += await studioFiles(relative);
        else { assert.equal(entry.isFile(), true); await compare(`packages/cem-studio/dist/static/${relative}`, `studio/${relative}`); count++; }
    }
    return count;
}
const copied = await studioFiles();
for (const file of ['cem_ql.js', 'cem_ql_factory.js', 'cem_ql_bg.wasm']) await compare(`packages/cem_ql/dist/wasm/${file}`, `packages/cem_ql/dist/wasm/${file}`);
for (const file of ['cem_ml.js', 'cem_ml_factory.js', 'cem_ml_bg.wasm']) await compare(`packages/cem-ml-npm/dist/wasm/browser/${file}`, `packages/cem-ml-npm/dist/wasm/browser/${file}`);
console.log(`Demo copy: all ${copied} Studio files and paired gallery runtime bytes match their build outputs.`);

const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const context = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
const page = await context.newPage();
page.setDefaultTimeout(30_000);
const errors = [], failures = [];
page.on('pageerror', error => errors.push(error.message));
page.on('response', response => { if (response.status() >= 400 && response.url().startsWith(url) && !response.url().endsWith('/favicon.ico')) failures.push(`${response.status()} ${response.url()}`); });
try {
    await page.goto(url);
    const links = await page.locator('a').evaluateAll(elements => elements.map(element => element.href));
    for (const link of links) assert.equal((await context.request.get(link)).status(), 200, link);
    const gallery = await page.evaluate(async () => {
        const primary = await import('/packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql.js');
        const canonical = await import('/packages/cem_ql/dist/wasm/runtime.js');
        const first = primary.default(), second = canonical.default();
        const shared = first === second; await first; await second;
        const { createCemQlWasmGeneration } = await import('/packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_factory.js');
        const a = createCemQlWasmGeneration(), b = createCemQlWasmGeneration();
        await a.initialize(); await b.initialize();
        const separate = a.memory() !== b.memory(); a.dispose(); b.dispose();
        const ml = await import('/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js');
        await ml.default();
        const source = '{p | Native gallery rendering}';
        return { shared, separate, initializer: typeof ml.initialize_common_query_runtime,
            render: JSON.parse(ml.renderCemMlToHtmlV1(JSON.stringify({ source }))),
            highlight: JSON.parse(ml.highlightSourceToHtmlV1(JSON.stringify({ source, contentType: 'text/cem-ml' }))) };
    });
    assert.equal(gallery.shared, true); assert.equal(gallery.separate, true); assert.equal(gallery.initializer, 'function');
    assert.equal(gallery.render.status, 'rendered'); assert.match(gallery.render.html, /Native gallery rendering/);
    assert.equal(gallery.highlight.status, 'highlighted'); assert.ok(gallery.highlight.spans.length > 0);
    console.log('Demo galleries: shared legacy initialization, isolated factory memories and native rendering/highlighting passed.');

    await page.getByRole('link', { name: 'CEM Studio', exact: true }).click();
    const ready = async () => { await page.waitForFunction(() => globalThis.__cemStudioApplication?.workbenchView); await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled()); };
    await ready(); assert.equal(new URL(page.url()).pathname, '/studio/');
    await page.evaluate(() => navigator.serviceWorker.ready);
    await page.waitForFunction(() => navigator.serviceWorker.controller);
    assert.equal(await page.evaluate(async () => (await navigator.serviceWorker.ready).scope), new URL('studio/', url).href);
    await page.evaluate(() => globalThis.__cemStudioApplication.workbench.openEntity('collection=content-types-documents-ui&sample=scss-stylesheet&build=build-html'));
    await ready();
    await page.getByRole('button', { name: 'Clone selected', exact: true }).click();
    await page.getByRole('dialog').getByRole('textbox').fill('Demo site verification');
    await page.getByRole('dialog').getByRole('button', { name: 'Clone', exact: true }).click(); await ready();
    assert.equal(new URL(page.url()).pathname, '/studio/'); assert.ok(new URL(page.url()).searchParams.get('project'));
    const build = async () => {
        await page.getByRole('button', { name: 'Build project', exact: true }).click(); await ready();
        const result = await page.evaluate(() => { const preview = globalThis.__cemStudioApplication.workbench.snapshot().projectPreview; return { status: preview.attempt?.status, text: preview.projection?.output.text }; });
        assert.equal(result.status, 'succeeded');
        await page.waitForFunction(() => { const doc = document.querySelector('cem-media-preview[mode="artifact-tree"] iframe')?.contentDocument; const card = doc?.querySelector('.demo-card'); return card && doc.defaultView.getComputedStyle(card).paddingTop === '12px'; });
        return result.text;
    };
    const online = await build();
    const definition = JSON.parse(await readFile(join(repo, 'examples/projects/scss-stylesheet/example.json'), 'utf8'));
    const graph = definition.configurations.find(item => item.id === 'build-html');
    const expected = definition.sources.find(item => item.id === graph.expectedGraphOutputs[graph.entryArtifact]);
    assert.equal(online, await readFile(join(repo, expected.source), 'utf8'));
    await context.setOffline(true); await page.reload(); await ready(); assert.equal(await build(), online);
    await context.setOffline(false); await page.goto(url); assert.equal(await page.evaluate(() => navigator.serviceWorker.controller), null);

    const baseline = process.env.CEM_DEMO_THEME_BASELINE
        ? JSON.parse(await readFile(process.env.CEM_DEMO_THEME_BASELINE, 'utf8')) : undefined;
    const generators = (await readdir(join(repo, 'packages/cem-theme/src/lib/css-generators')))
        .filter(name => name.endsWith('.html') && name !== 'index.html').sort();
    for (const name of generators) {
        await page.goto(new URL(`packages/cem-theme/dist/lib/css-generators/${name}`, url).href);
        await page.waitForFunction(() => document.querySelector('code[data-generated-css]')?.textContent.includes('--cem-'));
        const css = await page.locator('code[data-generated-css]').allTextContents();
        assert.equal(await page.locator('cem-css-loader').getAttribute('value'), css[0], name);
        if (baseline) assert.equal(createHash('sha256').update(JSON.stringify(css)).digest('hex'), baseline.find(item => item.name === name)?.sha256, name);
    }
    const playgrounds = inventory.files.filter(entry => entry.path.startsWith('packages/cem-components/playgrounds/') && entry.path.endsWith('.html'));
    for (const entry of playgrounds) assert.equal((await context.request.get(new URL(entry.path, url).href)).status(), 200, entry.path);
    await page.goto(new URL('packages/cem-components/playgrounds/cem-tree-gallery.html', url).href);
    await page.waitForFunction(() => [...document.querySelectorAll('cem-tree')].some(tree => tree.querySelector('[role="treeitem"]')));
    assert.deepEqual(errors, []); assert.deepEqual(failures, []);
    console.log('Demo Studio: link, subpath selection, independent saved copy, native HTML/CSS preview, offline rebuild and worker scope passed.');
    console.log(`Demo themes/components: all ${generators.length} generators rendered${baseline ? ' with unchanged CSS' : ''}; ${playgrounds.length} canonical playground pages and the live tree gallery passed.`);
} finally { await context.close(); await browser.close(); }
