import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, resolve } from 'node:path';
import { chromium } from 'playwright';

// Isolated deployment fixture: the production Studio worker and shared loader,
// with an explicitly opted-in pack. No user's projects, drafts or caches open.
const root = resolve(import.meta.dirname, '../..');
const packRoot = resolve(root, 'packages/cem-ml-npm/dist-schema-assets');
const nativeRoot = resolve(root, 'packages/cem-ml-npm/dist-schema-assets-wasm/wasm/browser');
const worker = await readFile(resolve(root, 'packages/cem-studio/src/service-worker.js'));
const index = JSON.parse(await readFile(resolve(packRoot, 'index.json')));
const selected = index.sources.find(source => source.packageId === 'csv' && source.path.endsWith('/formatters/compact.cemt'));
assert.ok(selected);
const runtimeFiles = ['schema-asset-pack.js', 'http-resource-transport.js', 'cem_ml_factory.js', 'cem_ml_bg.wasm'];
const groups = [
    { id: 'shell', urls: ['./index.html', './service-worker.js', './cache-inventory.json'] },
    { id: 'runtime', moduleMap: './module-map.json' },
    { id: 'samples', strategy: 'cache-first', catalog: './samples/index.json' },
    { id: 'schema-assets', strategy: 'cache-first', inventory: './schema-assets/integrity.json' },
];
let escaped = 0, disconnected = false, disconnectedRequests = 0;
const server = createServer(async (request, response) => {
    try {
        const match = /^\/(valid|invalid|missing|omitted|declarative)\/(.*)$/.exec(new URL(request.url, 'http://localhost').pathname);
        if (!match) { escaped++; response.writeHead(404).end(); return; }
        const [, variant, path] = match;
        // Chromium's page-level offline emulation can leave worker network
        // fetches reachable. Physically close the fixture transport as well.
        if (variant === 'valid' && disconnected) { disconnectedRequests++; response.destroy(); return; }
        let bytes;
        if (path === 'index.html') bytes = '<!doctype html><title>Optional schema deployment fixture</title>';
        else if (path === 'service-worker.js') bytes = worker;
        else if (path === 'cache-inventory.json') bytes = JSON.stringify({ schemaVersion: 2, commonVersion: '0.1.0', groups });
        else if (path === 'module-map.json') bytes = JSON.stringify({ resources: Object.fromEntries(runtimeFiles.map(file => [file, { path: `./runtime/${file}` }])) });
        else if (path === 'samples/index.json') bytes = JSON.stringify({ schemaVersion: 1, commonVersion: '0.1.0', cacheUrls: [] });
        else if (variant === 'declarative' && path.startsWith('elements/')) {
            const file = path.slice('elements/'.length);
            if (!/^(?:[a-zA-Z0-9_.-]+\/)*[a-zA-Z0-9_.-]+$/u.test(file) || file.split('/').includes('..')) throw Error('fixture path');
            bytes = await readFile(resolve(root, 'packages/cem-elements/dist', file));
        }
        else if (path.startsWith('runtime/') && runtimeFiles.includes(path.slice('runtime/'.length))) {
            const file = path.slice('runtime/'.length);
            bytes = await readFile(resolve(file.startsWith('cem_ml') ? nativeRoot : resolve(root, 'packages/cem-elements/dist/lib'), file));
        } else if (path === 'schema-assets/integrity.json' && (variant === 'invalid' || variant === 'omitted')) {
            const inventory = JSON.parse(await readFile(resolve(packRoot, 'integrity.json')));
            if (variant === 'invalid') {
                const first = inventory.files[0].path;
                inventory.files[0].path = '../escape';
                inventory.offlineUrls[inventory.offlineUrls.indexOf(first)] = '../escape';
            } else {
                const omitted = `sources/${selected.digest}`;
                inventory.files = inventory.files.filter(file => file.path !== omitted);
                inventory.offlineUrls = inventory.offlineUrls.filter(path => path !== omitted);
            }
            bytes = JSON.stringify(inventory);
        } else if (path.startsWith('schema-assets/')) {
            const file = path.slice('schema-assets/'.length);
            if (!/^(?:[a-z0-9_.-]+\/)*[a-z0-9_.-]+$/u.test(file) || file.split('/').includes('..')) throw Error('fixture path');
            if (variant === 'missing' && file === `sources/${selected.digest}`) { response.writeHead(404).end(); return; }
            bytes = await readFile(resolve(packRoot, file));
        } else { response.writeHead(404).end(); return; }
        const mime = { '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm' }[extname(path)] ?? 'application/octet-stream';
        response.writeHead(200, { 'content-type': mime, 'cache-control': 'no-store' }).end(bytes);
    } catch { response.writeHead(404).end(); }
});
await new Promise(ready => server.listen(0, '127.0.0.1', ready));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
try {
    await verifyDeclarativeDemand(browser, origin);
    const context = await browser.newContext();
    const page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    await page.goto(`${origin}/valid/index.html`);
    await page.evaluate(async () => { await navigator.serviceWorker.register('./service-worker.js', { scope: './' }); await navigator.serviceWorker.ready; });
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
    await context.setOffline(true);
    disconnected = true;
    await page.reload();
    const result = await page.evaluate(async selected => {
        const { CemSchemaAssetPackLoader } = await import('./runtime/schema-asset-pack.js');
        const { createCemMlWasmGeneration } = await import('./runtime/cem_ml_factory.js');
        const baseUrl = new URL('./schema-assets/', location.href).href;
        const make = generation => ({
            current: () => generation.memory() !== undefined,
            native: {
                schemaAssetPackBootstrapV1: () => generation.call('schemaAssetPackBootstrapV1', []),
                schemaAssetPackSummaryV1: (...args) => generation.call('schemaAssetPackSummaryV1', args),
                schemaAssetPackImportV1: (...args) => generation.call('schemaAssetPackImportV1', args),
                schemaAssetOwnerOpenV1: (...args) => generation.call('schemaAssetOwnerOpenV1', args),
                schemaAssetOwnerAdmitV1: (...args) => generation.call('schemaAssetOwnerAdmitV1', args),
                schemaAssetOwnerActivateV1: (...args) => generation.call('schemaAssetOwnerActivateV1', args),
                schemaAssetOwnerDisposeV1: (...args) => generation.call('schemaAssetOwnerDisposeV1', args),
                commandNativeMemoryIdentity: () => generation.memory(),
            },
        });
        const first = createCemMlWasmGeneration(), second = createCemMlWasmGeneration();
        await first.initialize();
        const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: make(first),
            activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'csv',
                expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] }) });
        let fresh, missing;
        try {
            const loaded = await loader.loadSource(selected);
            // The same opener serves an ordinary declarative http-request. Its
            // status DTO carries no native owner or replacement authority.
            const demandUrl = new URL('demand/csv/1.0.0', baseUrl).href;
            const demanded = await loader.loadResource({ authoredUrl: demandUrl, resolvedUrl: demandUrl, baseUrl,
                resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'fixture:declaration',
                method: 'GET', headers: {}, policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' },
                signal: new AbortController().signal });
            let status;
            for await (const bytes of demanded.body) status = JSON.parse(new TextDecoder().decode(bytes));
            if (status?.state !== 'ready' || status.packageId !== 'csv') throw Error('offline demand did not activate');
            const ownerCount = generation => JSON.parse(generation.call('commandNativeResourceInventory', []))
                .registries.find(entry => entry.name === 'schema-package-owners')?.live;
            if (ownerCount(first) !== 1) throw Error('missing native package ownership');
            const urls = await loader.deploymentUrls();
            const names = await caches.keys();
            const cache = await caches.open(names.find(name => name.includes(':schema-assets:')));
            const stored = (await cache.keys()).map(request => request.url);
            if (JSON.stringify([...urls].sort()) !== JSON.stringify(stored.sort())) throw Error('incomplete offline cache');
            await second.initialize();
            const independent = first.memory() !== second.memory();
            fresh = await loader.prepareFreshRuntime(make(second));
            if (ownerCount(first) !== 1 || ownerCount(second) !== 1) throw Error('fresh ownership was not compiled independently');
            loader.dispose(); first.dispose();
            const reimported = await fresh.loadSource(selected);
            if (reimported.sourceUri !== loaded.sourceUri || reimported.digest !== loaded.digest
                || reimported.nodeCount !== loaded.nodeCount) throw Error('fresh source drift');
            // Remove only this isolated fixture's cached source to prove that an
            // embedded catalog never satisfies failed optional acquisition.
            if (!await cache.delete(new URL(`sources/${selected.digest}`, baseUrl))) throw Error('fixture source was not cached');
            missing = new CemSchemaAssetPackLoader({ baseUrl, runtime: make(second) });
            let missingCode, missingError;
            try { await missing.loadSource(selected); } catch (error) { missingCode = error.code; missingError = String(error); }
            if (missingCode !== 'cem.schema_asset.unavailable') throw Error(`offline missing source did not refuse: ${missingCode}; ${missingError}`);
            return { independent, sourceUri: loaded.sourceUri, nodeCount: loaded.nodeCount, inventoryFiles: urls.length, missingCode };
        } finally {
            loader.dispose(); fresh?.dispose(); missing?.dispose();
            if (first.memory()) first.dispose();
            if (second.memory()) second.dispose();
        }
    }, selected);
    assert.equal(result.independent, true);
    assert.equal(result.sourceUri, selected.path);
    assert.ok(result.nodeCount > 0);
    assert.ok(disconnectedRequests > 0, 'missing-source probe must reach the disconnected transport boundary');
    assert.deepEqual(errors, []);
    await context.close();
    for (const variant of ['invalid', 'missing', 'omitted']) {
        const refused = await browser.newContext();
        const probe = await refused.newPage();
        await probe.goto(`${origin}/${variant}/index.html`);
        const state = await probe.evaluate(async () => {
            const registration = await navigator.serviceWorker.register('./service-worker.js', { scope: './' });
            const candidate = registration.installing;
            if (!candidate) throw Error('missing installation candidate');
            await new Promise(resolveState => {
                const changed = () => { if (candidate.state === 'redundant' || candidate.state === 'activated') resolveState(); };
                candidate.addEventListener('statechange', changed); changed();
            });
            return candidate.state;
        });
        assert.equal(state, 'redundant', `${variant} deployment must refuse installation`);
        await refused.close();
    }
    assert.equal(escaped, 0);
    console.log(`Verified declarative http-request package demand, ${result.inventoryFiles} optional offline assets, cold offline source load and package demand, distinct-heap reimport/reactivation, native ownership cleanup, missing-source refusal and invalid/incomplete deployment refusal.`);
} finally {
    await browser.close();
    await new Promise(closed => server.close(closed));
}

async function verifyDeclarativeDemand(browser, origin) {
    const context = await browser.newContext();
    try {
        const page = await context.newPage();
        await page.goto(`${origin}/declarative/index.html`);
        await page.evaluate(async () => {
            const { CemElementRuntime } = await import('./elements/lib/cem-elements.js');
            const { CemSchemaAssetPackLoader } = await import('./runtime/schema-asset-pack.js');
            const { createCemMlWasmGeneration } = await import('./runtime/cem_ml_factory.js');
            const generation = createCemMlWasmGeneration();
            await generation.initialize();
            const native = Object.fromEntries(['schemaAssetPackBootstrapV1', 'schemaAssetPackSummaryV1', 'schemaAssetPackImportV1',
                'schemaAssetOwnerOpenV1', 'schemaAssetOwnerAdmitV1', 'schemaAssetOwnerActivateV1', 'schemaAssetOwnerDisposeV1']
                .map(name => [name, (...args) => generation.call(name, args)]));
            native.commandNativeMemoryIdentity = () => generation.memory();
            const loader = new CemSchemaAssetPackLoader({ baseUrl: new URL('./schema-assets/', location.href).href,
                runtime: { native, current: () => generation.memory() !== undefined },
                activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'csv',
                    expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] }) });
            const runtime = new CemElementRuntime({ declarationTag: 'cem-package-demand-declaration',
                loadHttpResource: request => loader.loadResource(request),
                processingWorkerFactory: () => { throw Error('fixture uses the shared fallback renderer'); } });
            runtime.install(window);
            const declaration = document.createElement('cem-package-demand-declaration');
            declaration.setAttribute('tag', 'package-demand-example');
            const template = document.createElement('template');
            template.setAttribute('type', 'text/cem-ml');
            template.textContent = '{http-request @slice=pack @url="./schema-assets/demand/csv/1.0.0" @content-type="application/json"}'
                + '{p @role=status | {$datadom.slices.pack.state}}';
            declaration.append(template); document.body.append(declaration);
            runtime.registerDeclaration(declaration);
            document.body.append(document.createElement('package-demand-example'));
            window.packageDemandFixture = { generation, loader };
        });
        await page.waitForFunction(() => document.querySelector('package-demand-example [role=status]')?.textContent === 'loaded', undefined, { timeout: 30_000 });
        const owned = await page.evaluate(() => {
            const { generation, loader } = window.packageDemandFixture;
            const census = () => JSON.parse(generation.call('commandNativeResourceInventory', []));
            const count = census().registries.find(entry => entry.name === 'schema-package-owners')?.live;
            document.querySelector('package-demand-example').remove();
            loader.dispose();
            if (census().blockingResources !== 0) throw Error('declarative demand leaked a native owner');
            generation.dispose();
            return count;
        });
        assert.equal(owned, 1);
    } finally { await context.close(); }
}
