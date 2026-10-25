import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { extname, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

// Only isolated browser storage is used. Catalogs/reports below are explicitly
// named control DTOs; source parsing and all deployment emission stay native.
const project = resolve(import.meta.dirname, '..');
const workspace = resolve(project, '../..');
const output = resolve(project, 'dist/schema-assets');
const ordinary = resolve(project, 'dist/static');
const pack = resolve(workspace, 'packages/cem-ml-npm/dist-schema-assets');
const json = async path => JSON.parse(await readFile(path, 'utf8'));
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const build = await json(resolve(output, 'build.json'));
const maps = await json(resolve(output, 'module-map.json'));
const index = await json(resolve(output, 'schema-assets/index.json'));
const selected = index.sources.find(source => source.packageId === 'csv' && source.path.endsWith('/formatters/compact.cemt'));
assert.ok(selected);
const formats = await Promise.all([
    { packageId: 'markdown', path: 'basic-document.md', contentType: 'text/markdown; variant=CommonMark', schema: 'https://cem.dev/ns/data/markdown/1', expected: 'CEM Markdown Example' },
    { packageId: 'scss', path: 'basic.scss', contentType: 'text/x-scss', schema: 'https://cem.dev/ns/data/scss/1',
        targetContentType: 'text/css', targetSchema: 'https://cem.dev/ns/data/css/1', expected: 'card' },
    { packageId: 'xslt', path: 'basic-stylesheet.xsl', contentType: 'application/xslt+xml', schema: 'https://cem.dev/ns/transform/xslt/1', expected: 'stylesheet' },
    { packageId: 'relax-ng', path: 'basic-schema.rng', contentType: 'application/relax-ng+xml', schema: 'https://cem.dev/ns/data/relax-ng/1', expected: 'grammar' },
].map(async format => ({ ...format, source: await readFile(resolve(workspace, 'packages/cem_ml/schema-packages', format.packageId, 'v1/examples', format.path), 'utf8') })));
const packageIds = ['csv', ...formats.map(format => format.packageId)];
assert.equal(build.deploymentProfile, 'schema-assets');
assert.equal(build.automaticRecovery, false);
assert.equal(build.packagePublication, 'explicit-host-admission');
assert.equal(build.embeddedBodies, 'bootstrap-only');
assert.equal(build.embeddingProfile.externalBodies, true);
assert.ok(build.embeddingProfile.externalPackages.includes('csv'));
assert.deepEqual(build.schemaPackageHostConfiguration.schemaPackageReplacementGrants,
    packageIds.map(packageId => ({ packageId, expectedOrigin: { kind: 'builtin' }, replacementManifestUri: `schema-packages/${packageId}/v1/package.cem` })));
assert.equal((await json(resolve(ordinary, 'build.json'))).deploymentProfile, undefined);
assert.equal((await json(resolve(ordinary, 'cache-inventory.json'))).groups.some(group => group.id === 'schema-assets'), false);
const packFiles = await files(pack);
assert.deepEqual(await files(resolve(output, 'schema-assets')), packFiles);
for (const path of packFiles) assert.deepEqual(await readFile(resolve(output, 'schema-assets', path)), await readFile(resolve(pack, path)), path);
const pin = await json(resolve(pack, 'bootstrap.json'));
assert.equal(pin.indexDigest, build.schemaAssetIndexDigest);
const binary = await readFile(resolve(workspace, 'packages/cem-ml-npm/dist-schema-assets-wasm/wasm/browser/cem_ml_bg.wasm'));
for (const id of ['@epa-wg/cem-ml/wasm-binary', '@epa-wg/cem-elements/cem-ql-wasm']) {
    assert.equal(sha256(await readFile(resolve(output, maps.resources[id].path))), sha256(binary), id);
}
const graphReport = await json(resolve(workspace, 'dist/reports/cem-studio/schema-assets-build.json'));
assert.equal(graphReport.summary.errorCount + graphReport.summary.fatalCount, 0);
const baseline = await treeHashes(ordinary);
const probeImportMap = (await readFile(resolve(output, 'index.html'), 'utf8')).match(/<script type="importmap">[\s\S]*?<\/script>/)?.[0];
assert.ok(probeImportMap, 'native output must provide its import map to the isolated package probe');
let disconnected = false, disconnectedRequests = 0;
const security = await json(resolve(output, 'security-headers.json'));
const server = createServer(async (request, response) => {
    if (disconnected) { disconnectedRequests++; response.destroy(); return; }
    try {
        const match = /^\/(ordinary|optional|invalid|orphan)\/(.*)$/.exec(decodeURIComponent(new URL(request.url, 'http://localhost').pathname));
        if (!match) { response.writeHead(404).end(); return; }
        const [, profile, path] = match;
        const root = profile === 'ordinary' ? ordinary : output;
        const file = resolve(root, path || 'index.html');
        if (!file.startsWith(`${root}${sep}`)) { response.writeHead(403).end(); return; }
        let bytes = path === 'package-probe.html' ? `<!doctype html><title>Isolated package recovery probe</title>${probeImportMap}` : await readFile(file);
        if (profile === 'invalid' || profile === 'orphan') {
            if (path === 'index.html') bytes = '<!doctype html><title>Profile metadata refusal fixture</title>';
            if (path === 'cache-inventory.json') {
                const inventory = JSON.parse(bytes);
                if (profile === 'orphan') delete inventory.deploymentProfile;
                else inventory.deploymentDigest = 'invalid';
                bytes = JSON.stringify(inventory);
            }
        }
        response.writeHead(200, { ...security.headers, 'cache-control': 'no-store',
            'content-type': ({ '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm',
                '.css': 'text/css', '.xhtml': 'application/xhtml+xml', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' })[extname(file)] ?? 'application/octet-stream' }).end(bytes);
    } catch { response.writeHead(404).end(); }
});
await new Promise(ready => server.listen(0, '127.0.0.1', ready));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ serviceWorkers: 'allow' });
const errors = [];
context.on('page', page => {
    page.on('pageerror', error => errors.push(error.message));
    page.on('console', message => { if (message.type() === 'error' && !message.text().startsWith('Failed to load resource:')) errors.push(message.text()); });
});
try {
    const defaultPage = await context.newPage();
    await openStudio(defaultPage, `${origin}/ordinary/index.html`);
    assert.equal(await defaultPage.evaluate(() => globalThis.__cemStudioApplication.packageStatus()), undefined);
    await install(defaultPage);
    const defaultNames = await defaultPage.evaluate(() => caches.keys());
    assert.equal(defaultNames.length, 3);
    const page = await context.newPage();
    await openStudio(page, `${origin}/optional/index.html`);
    const studioDemand = await verifyStudioDemand(page);
    const formatDemand = await verifyFormatDemands(context, `${origin}/optional/index.html`, formats);
    const outputPage = await context.newPage();
    await openStudio(outputPage, `${origin}/optional/index.html?sample=login&file=login-html`);
    const outputDrivenDemand = await verifyStudioDemand(outputPage, true);
    await outputPage.close();
    const resourcePage = await context.newPage();
    await openStudio(resourcePage, `${origin}/optional/index.html?sample=schema-examples&file=example-csv`);
    await resourcePage.waitForFunction(() => globalThis.__cemStudioApplication?.packageStatus()?.demandCount === 1);
    assert.equal(await resourcePage.evaluate(() => globalThis.__cemStudioApplication.workbench.snapshot().navigation.selection.file), 'example-csv');
    await resourcePage.close();
    await install(page);
    const names = await page.evaluate(() => caches.keys());
    assert.equal(names.length, 7);
    assert.ok(defaultNames.every(name => names.includes(name)));
    const optionalNames = names.filter(name => name.startsWith('cem-studio:profile:'));
    assert.equal(optionalNames.length, 4);
    assert.ok(optionalNames.every(name => name.includes(`:schema-assets:${build.deploymentDigest}:`)));
    // Reinstall the ordinary worker after the profile to exercise cleanup in
    // both directions. Its historical cache names remain unchanged.
    await defaultPage.evaluate(async () => {
        const registration = await navigator.serviceWorker.getRegistration();
        await registration.unregister();
        const replacement = await navigator.serviceWorker.register('./service-worker.js?profile-isolation=1', { scope: './', type: 'module' });
        if (replacement.active?.state === 'activated') return;
        await new Promise(done => {
            const worker = replacement.installing ?? replacement.waiting;
            if (!worker) throw Error('missing replacement worker');
            const changed = () => { if (worker.state === 'activated' || worker.state === 'redundant') done(); };
            worker.addEventListener('statechange', changed); changed();
        });
        if (replacement.active?.state !== 'activated') throw Error('ordinary replacement failed');
    });
    assert.deepEqual((await page.evaluate(() => caches.keys())).sort(), [...names].sort());
    console.log('Verified native output, runtime/pack pairing and bidirectional cache isolation.');
    const renderingProbes = [];
    for (const mode of ['worker', 'fallback']) {
        const probe = await context.newPage(); await probe.goto(`${origin}/optional/package-probe.html`);
        renderingProbes.push({ mode, probe });
    }
    // The standalone raw-loader fixture must not borrow Studio's already-bound
    // logical root, trusted configuration or selected helper generation.
    const loaderProbe = await context.newPage(); await loaderProbe.goto(`${origin}/optional/package-probe.html`);
    await context.setOffline(true);
    disconnected = true;
    await page.reload({ waitUntil: 'load' });
    await ready(page);
    const offlineStudioDemand = await verifyStudioDemand(page);
    const offlineFormatDemand = await verifyFormatDemands(context, `${origin}/optional/index.html`, formats);
    const command = await versionCommand(page);
    assert.equal(command.exitCode, 0);
    assert.equal(command.runtime, 'wasm-browser-worker');
    const evidence = await loaderProbe.evaluate(async ({ selected, maps, pin }) => {
        const { CemSchemaAssetPackLoader } = await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
        const { CemElementRuntime } = await import(maps.resources['@epa-wg/cem-elements/cem-elements'].path);
        const { createCemMlWasmGeneration } = await import(maps.resources['@epa-wg/cem-ml/wasm/factory'].path);
        const { createCemQlWasmGeneration } = await import(maps.resources['@epa-wg/cem-elements/cem-ql-factory'].path);
        const renderer = createCemQlWasmGeneration();
        await renderer.initialize();
        if (JSON.parse(renderer.call('schemaAssetPackBootstrapV1', [])).indexDigest !== pin.indexDigest) throw Error('renderer pin mismatch');
        if (JSON.parse(renderer.call('schemaAssetEmbeddedSourcesV1', [])).externalBodies !== true) throw Error('renderer retained optional bodies');
        let missingSourceRefused = false;
        try { renderer.call('retainCemDocument', [new TextEncoder().encode('id,name\n1,Ada\n'), 'text/csv', 'memory:missing.csv']); }
        catch (error) { missingSourceRefused = String(error).includes('cem.schema_asset.missing'); }
        if (!missingSourceRefused) throw Error('renderer imported CSV before package admission');
        renderer.dispose();
        const first = createCemMlWasmGeneration(), second = createCemMlWasmGeneration();
        await first.initialize(); await second.initialize();
        const ensure = (condition, message) => { if (!condition) throw Error(message); };
        let sourceCurrent = true, credentialsEntered = false;
        const binding = (generation, current) => ({ current, native: {
            ...Object.fromEntries(['schemaAssetPackBootstrapV1', 'schemaAssetPackSummaryV1', 'schemaAssetPackImportV1', 'schemaAssetOwnerOpenV1',
                'schemaAssetOwnerAdmitV1', 'schemaAssetOwnerActivateV1', 'schemaAssetOwnerDisposeV1'].map(name => [name, (...args) => generation.call(name, args)])),
            commandNativeMemoryIdentity: () => generation.memory(),
        } });
        const loader = new CemSchemaAssetPackLoader({ baseUrl: new URL('./schema-assets/', location.href).href,
            runtime: binding(first, () => sourceCurrent && !credentialsEntered && first.memory() !== undefined),
            activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'csv', expectedOrigin: { kind: 'builtin' },
                replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] }) });
        let fresh, runtime;
        const owners = generation => JSON.parse(generation.call('commandNativeResourceInventory', [])).registries.find(entry => entry.name === 'schema-package-owners')?.live;
        try {
            credentialsEntered = true;
            let credentialCode;
            try { await loader.activatePackages([{ packageId: 'csv', version: '1.0.0' }]); } catch (error) { credentialCode = error.code; }
            ensure(credentialCode === 'cem.schema_asset.stale' && owners(first) === 0, 'credential admission bypassed');
            credentialsEntered = false;
            runtime = new CemElementRuntime({ declarationTag: 'cem-profile-demand-declaration', loadHttpResource: request => loader.loadResource(request),
                processingWorkerFactory: () => { throw Error('isolated demand uses shared fallback rendering'); } });
            runtime.install(window);
            const declaration = document.createElement('cem-profile-demand-declaration');
            declaration.setAttribute('tag', 'profile-demand-example');
            const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml');
            template.textContent = '{http-request @slice=pack @url="./schema-assets/demand/csv/1.0.0" @content-type="application/json"}{p @role=status | {$datadom.slices.pack.state}}';
            declaration.append(template); document.body.append(declaration); runtime.registerDeclaration(declaration);
            document.body.append(document.createElement('profile-demand-example'));
            await new Promise((done, reject) => {
                const deadline = setTimeout(() => { observer.disconnect(); reject(Error('offline declarative demand timed out')); }, 30_000);
                const check = () => { if (document.querySelector('profile-demand-example [role=status]')?.textContent === 'loaded') {
                    clearTimeout(deadline); observer.disconnect(); done();
                } };
                const observer = new MutationObserver(check); observer.observe(document.body, { childList: true, subtree: true, characterData: true }); check();
            });
            ensure(owners(first) === 1, 'declarative demand did not activate a native owner');
            const loaded = await loader.loadSource(selected);
            const urls = await loader.deploymentUrls();
            const names = await caches.keys(), name = names.find(name => name.startsWith('cem-studio:profile:') && name.includes(':schema-assets:') && name.endsWith(pin.indexDigest));
            const cache = await caches.open(name);
            const stored = (await cache.keys()).map(request => request.url).sort();
            ensure(JSON.stringify(stored) === JSON.stringify([...urls].sort()), 'incomplete offline pack');
            sourceCurrent = false;
            fresh = await loader.prepareFreshRuntime(binding(second, () => second.memory() !== undefined));
            ensure(first.memory() !== second.memory() && owners(second) === 1 && owners(first) === 1, 'owners were transferred instead of recompiled');
            let staleCode;
            try { await loader.loadSource(selected); } catch (error) { staleCode = error.code; }
            ensure(staleCode === 'cem.schema_asset.stale', 'retired source binding remained admitted');
            ensure(JSON.parse(second.call('commandNativeResourceInventory', [])).replacementAllowed === false, 'preparation granted retirement');
            let retirementRefused = false;
            try { first.dispose(); } catch { retirementRefused = true; }
            ensure(retirementRefused, 'active package owner did not block retirement');
            loader.dispose(); first.dispose();
            const reimported = await fresh.loadSource(selected);
            ensure(reimported.sourceUri === loaded.sourceUri && reimported.digest === loaded.digest && reimported.nodeCount === loaded.nodeCount, 'fresh source provenance changed');
            return { sourceUri: reimported.sourceUri, inventoryFiles: stored.length, independentOwners: true, staleCode, credentialCode, retirementRefused };
        } finally {
            document.querySelector('profile-demand-example')?.remove();
            loader.dispose(); fresh?.dispose();
            if (first.memory()) first.dispose(); if (second.memory()) second.dispose();
        }
    }, { selected, maps, pin });
    await loaderProbe.close();
    assert.equal(evidence.inventoryFiles, packFiles.length);
    assert.equal(evidence.retirementRefused, true);
    const packageHandoff = await verifyOwnedPackageHandoff(page, maps, packageIds);
    const workerPackageHandoff = await verifyWorkerPackageHandoff(page, maps, formats);
    const renderingPackageHandoffs = {};
    for (const { mode, probe } of renderingProbes) {
        renderingPackageHandoffs[mode] = await verifyRenderingPackageHandoff(probe, maps, mode, formats);
        await probe.close();
    }
    disconnected = false;
    for (const variant of ['invalid', 'orphan']) {
        const refused = await browser.newContext();
        try {
            const probe = await refused.newPage(); await probe.goto(`${origin}/${variant}/index.html`);
            const state = await probe.evaluate(async () => {
                const registration = await navigator.serviceWorker.register('./service-worker.js', { scope: './', type: 'module' });
                const worker = registration.installing;
                if (!worker) throw Error('missing install candidate');
                return new Promise(done => {
                    const changed = () => { if (worker.state === 'activated' || worker.state === 'redundant') done(worker.state); };
                    worker.addEventListener('statechange', changed); changed();
                });
            });
            assert.equal(state, 'redundant', `${variant} profile must refuse installation`);
            assert.deepEqual(await probe.evaluate(() => caches.keys()), []);
        } finally { await refused.close(); }
    }
    assert.deepEqual(errors, []);
    assert.deepEqual(await treeHashes(ordinary), baseline);
    const report = { schemaVersion: 1, status: 'passed', deploymentDigest: build.deploymentDigest,
        nativeErrors: graphReport.summary.errorCount + graphReport.summary.fatalCount, emittedPackFiles: packFiles.length, scopedCacheGroups: optionalNames.length,
        defaultCacheIsolation: true, defaultOutputPreserved: true, coldOfflineStudio: true, studioDemand, outputDrivenDemand,
        selectedCsvResourceDemand: true, offlineStudioDemand, formatDemand, offlineFormatDemand, command, ...evidence, packageHandoff, workerPackageHandoff, renderingPackageHandoffs,
        automaticRecovery: false, packagePublication: 'explicit-host-admission', disconnectedRequests };
    const reportPath = resolve(workspace, 'dist/reports/cem-studio/schema-assets-profile.json');
    await mkdir(resolve(workspace, 'dist/reports/cem-studio'), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    console.log(`Verified cold offline Studio/worker command, declarative package demand, ${packFiles.length} assets, both factory pins, independent source reimport and credential/stale/retirement refusals.`);
} finally {
    await context.close(); await browser.close();
    await new Promise(closed => server.close(closed));
}

async function files(root, prefix = '') {
    const result = [];
    for (const entry of (await readdir(resolve(root, prefix), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
        const path = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) result.push(...await files(root, path)); else result.push(path);
    }
    return result;
}
async function treeHashes(root) { return Promise.all((await files(root)).map(async path => [path, sha256(await readFile(resolve(root, path)))])); }
async function ready(page) {
    await page.waitForFunction(() => globalThis.__cemStudioApplication
        && document.querySelector('[data-cem-studio-shell]')?.getAttribute('data-cem-studio-state') === 'ready', undefined, { timeout: 60_000 });
}
async function verifyStudioDemand(page, outputFirst = false) {
    await page.waitForFunction(() => {
        const status = globalThis.__cemStudioApplication?.packageStatus();
        return status?.state === 'idle' && status.demandCount === 0 && status.runtimeCount >= 4;
    }, undefined, { timeout: 30_000 });
    return page.evaluate(async outputFirst => {
        const app = globalThis.__cemStudioApplication;
        const startup = app.packageStatus();
        const convert = async () => {
            const converted = await app.validator.runResourceCommand({ bytes: new TextEncoder().encode('[{"id":1,"name":"Ada"}]'), uri: 'table.json',
                contentType: 'application/json', schema: 'https://cem.dev/ns/data/json/1',
                argv: ['convert', 'studio://cem-studio-command/table.json', '--content-type', 'application/json', '--schema', 'https://cem.dev/ns/data/json/1',
                    '--to-content-type', 'text/csv', '--to-schema', 'https://cem.dev/ns/data/csv/1'] });
            if (converted.result.exitCode !== 0 || !converted.output?.text.includes('Ada')) throw Error('Studio did not use the externally delivered CSV formatter');
        };
        if (outputFirst) await convert();
        const options = { bytes: new TextEncoder().encode('id,name\n1,Ada\n'), uri: 'demand.csv', contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' };
        const parsed = await app.validator.parseResource(options);
        if (parsed.result.exitCode !== 0) throw Error('Studio package command did not complete');
        let invalidRefused = false;
        try { await app.validator.parseResource({ ...options, bytes: new TextEncoder().encode('id,name\n1,"Ada\n') }); }
        catch (error) { invalidRefused = JSON.stringify(error.result).includes('cem.csv.unclosed_quote'); }
        if (!invalidRefused) throw Error('Studio package command lost native CSV diagnostics');
        if (!outputFirst) await convert();
        const status = app.packageStatus();
        if (status.state !== 'idle' || status.demandCount !== 1 || status.runtimeCount < 4) throw Error('incomplete native package publication');
        return { ...status, startup, outputFirst, nativeCommand: true, nativeInvalidCsvRefused: true, externalCsvFormatter: true };
    }, outputFirst);
}
async function verifyFormatDemands(context, url, formats) {
    const evidence = {};
    for (const format of formats) {
        const page = await context.newPage(), packRequests = [];
        page.on('request', request => { if (new URL(request.url()).pathname.includes('/schema-assets/')) packRequests.push(request.url()); });
        try {
            await openStudio(page, url);
            assert.equal(packRequests.length, 0, `${format.packageId}: startup acquired optional assets before demand`);
            evidence[format.packageId] = await page.evaluate(async format => {
                const app = globalThis.__cemStudioApplication, startup = app.packageStatus();
                const ensure = (condition, message) => { if (!condition) throw Error(`${format.packageId}: ${message}`); };
                ensure(startup.state === 'idle' && startup.demandCount === 0 && startup.runtimeCount >= 4, 'unexpected startup publication');
                const options = { bytes: new TextEncoder().encode(format.source), uri: format.path,
                    contentType: format.contentType, schema: format.schema, projectId: 'format-demand' };
                const commandUri = `studio://format-demand/${format.path}`;
                console.info(`[format-demand:${format.packageId}] parse`);
                const parsed = await app.validator.parseResource(options);
                ensure(parsed.result.exitCode === 0 && parsed.output?.text.includes('@doc') && parsed.output.text.includes('{ast'),
                    'native parsing did not produce its CEM AST presentation');
                ensure(!parsed.result.diagnostics.items.some(diagnostic => ['error', 'fatal'].includes(diagnostic.severity)),
                    'native parsing returned hard diagnostics despite its successful CLI presentation');
                const expandsCss = format.packageId === 'scss';
                console.info(`[format-demand:${format.packageId}] ${expandsCss ? 'CSS expansion' : 'format/color'}`);
                const formatted = await app.validator.runResourceCommand({ ...options, argv: ['convert', commandUri,
                    '--content-type', format.contentType, '--schema', format.schema,
                    '--to-content-type', format.targetContentType ?? format.contentType, '--to-schema', format.targetSchema ?? format.schema,
                    ...(expandsCss ? ['--output-color-type', 'none']
                        : ['--cemt-formatter-profile', 'tabular', '--cemt-color-profile', 'html'])] });
                ensure(formatted.result.exitCode === 0 && formatted.output?.text,
                    expandsCss ? 'native SCSS-to-CSS expansion failed' : 'externally delivered formatter failed');
                if (expandsCss) {
                    ensure(formatted.output.text.includes('.card') && formatted.output.text.includes('.card__title')
                        && formatted.output.text.includes('color:') && /font-weight:\s*700/.test(formatted.output.text),
                        'native SCSS expansion lost its CSS rules');
                } else {
                    ensure(formatted.output.text.includes('<span'), 'externally delivered colorizer did not produce HTML syntax markup');
                    // Native syntax output can be a pre or sibling span fragment.
                    // Read under an inert XML root to compare decoded text without
                    // applying HTML styles or triggering a multi-root parsererror.
                    const syntax = new DOMParser().parseFromString(`<artifact>${formatted.output.text}</artifact>`, 'application/xml');
                    ensure(!syntax.querySelector('parsererror') && syntax.documentElement.textContent.includes(format.expected),
                        'formatted/colorized source lost its semantic text');
                }
                if (format.packageId === 'markdown') {
                    console.info(`[format-demand:${format.packageId}] HTML conversion`);
                    const html = await app.validator.runResourceCommand({ ...options, argv: ['convert', commandUri,
                        '--content-type', format.contentType, '--schema', format.schema, '--to-content-type', 'text/html',
                        '--to-schema', 'https://cem.dev/ns/data/html/1', '--output-color-type', 'none'] });
                    ensure(html.result.exitCode === 0 && new DOMParser().parseFromString(html.output.text, 'text/html').querySelector('h1')?.textContent === format.expected,
                        'native Markdown HTML conversion failed');
                }
                const status = app.packageStatus();
                ensure(status.state === 'idle' && status.demandCount === 1 && status.runtimeCount === startup.runtimeCount,
                    'demand did not publish to the complete original physical cohort');
                return { startup, ...status, nativeParse: true,
                    ...(expandsCss ? { nativeCssExpansion: true }
                        : { externalFormatter: true, externalColorizer: true }),
                    ...(format.packageId === 'markdown' && { nativeHtmlConversion: true }) };
            }, format);
            // A fresh physical cohort must prepare the selected resource itself,
            // rather than borrowing the preceding command's active package.
            await page.reload(); await ready(page);
            assert.equal(await page.evaluate(() => globalThis.__cemStudioApplication.packageStatus().demandCount), 0);
            await page.evaluate(packageId => globalThis.__cemStudioApplication.workbench.openEntity(`sample=schema-examples&file=example-${packageId}`), format.packageId);
            await page.waitForFunction(() => {
                const app = globalThis.__cemStudioApplication, navigation = app.workbench.snapshot().navigation;
                return !navigation.busy && navigation.sourcePreview?.status !== 'pending' && app.packageStatus().state === 'idle';
            }, undefined, { timeout: 30_000 });
            evidence[format.packageId].selectedResource = await page.evaluate(async ({ format, originalRuntimeCount }) => {
                const app = globalThis.__cemStudioApplication;
                await app.workbenchView.whenSettled();
                const selected = app.workbench.snapshot().navigation, status = app.packageStatus(), resourceId = `example-${format.packageId}`;
                const ensure = (condition, message) => { if (!condition) throw Error(`${format.packageId}: ${message}`); };
                ensure(selected.selection.sample === 'schema-examples' && selected.selection.file === resourceId && selected.content === 'sample-file',
                    'selected example resource did not publish');
                ensure(!selected.error && selected.status !== 'error' && selected.sourcePreview?.status !== 'error' && selected.sampleText.length > 0,
                    'selected example source is unavailable');
                ensure(new URL(location.href).searchParams.get('file') === resourceId, 'URL did not follow selected example resource');
                ensure(status.state === 'idle' && status.demandCount === 1 && status.runtimeCount >= originalRuntimeCount,
                    'selected example did not preserve complete package publication');
                const editor = [...document.querySelectorAll('cem-source-editor[readonly]')].find(editor => editor.getAttribute('aria-label') === selected.path);
                ensure(editor?.getAttribute('value') === selected.sampleText && editor.querySelector('[role=textbox]')?.textContent.includes(format.expected),
                    'selected example editor did not present its admitted source');
                return { id: resourceId, sourceAvailable: true, ...status };
            }, { format, originalRuntimeCount: evidence[format.packageId].runtimeCount });
            console.log(`Verified native ${format.packageId} demand, ${format.packageId === 'scss' ? 'CSS expansion' : 'formatter/colorizer'} and selected resource.`);
        } catch (error) {
            throw new Error(`${format.packageId}: ${error.message}; browser errors: ${JSON.stringify(errors)}`, { cause: error });
        } finally { await page.close(); }
    }
    return evidence;
}
async function openStudio(page, url) {
    await page.goto(url);
    try { await ready(page); }
    catch (error) { throw new Error(`${error.message} at ${url}; browser errors: ${JSON.stringify(errors)}`, { cause: error }); }
}
async function install(page) {
    await page.evaluate(async () => { await navigator.serviceWorker.register('./service-worker.js', { scope: './', type: 'module' }); await Promise.race([navigator.serviceWorker.ready, new Promise((_, reject) => setTimeout(() => reject(Error('service worker ready timed out')), 30_000))]); });
    await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
}
async function versionCommand(page) {
    return page.evaluate(async () => {
        const api = await import('@epa-wg/cem-ml-cli/browser-commands');
        const parsed = api.parseCemMlCommand(['version'], { runtime: 'wasm-browser-worker' });
        const invocation = await api.buildBrowserCommandInvocation({ schemaVersion: parsed.schemaVersion, commonVersion: parsed.commonVersion,
            commandPath: ['version'], globalOptions: parsed.globalOptions, options: {}, positionals: {} }, () => { throw Error('unexpected resource request'); }, { requestId: 'optional-profile-offline' });
        const forbidden = async () => { throw Error('unexpected host mutation/read'); };
        const client = await api.createBrowserCommandServiceClient({ host: { currentRevision: async ({ project }) => ({ project, resourceVersions: {} }),
            readResource: forbidden, prepareWrite: forbidden, commitWrite: forbidden, rollbackWrite: async () => undefined } });
        try {
            const result = await client.execute(invocation.request).result();
            return { exitCode: result.exitCode, status: result.status, runtime: client.capability.runtime };
        } finally { await client.close(); }
    });
}

async function verifyOwnedPackageHandoff(page, maps, packageIds) {
    return page.evaluate(async ({ maps, packageIds }) => {
        const { CemSchemaAssetPackLoader, CemSchemaAssetPackRecoveryBinding, cemSchemaAssetRuntimeFromBinding } =
            await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
        const { CemNativeRecoveryScope, resumeNativeGenerationBindings } = await import(maps.resources['@epa-wg/cem-elements/native-generation'].path);
        const api = await import('@epa-wg/cem-ml-cli/browser-commands');
        const ensure = (condition, message) => { if (!condition) throw Error(message); };
        let credentialsEntered = false;
        const scope = new CemNativeRecoveryScope();
        const host = await api.createBrowserCommandInvocationBinding({ recoveryScope: scope, id: 'optional-package-authoring',
            hostWitness: () => () => !credentialsEntered });
        const initial = host.runtime, baseUrl = new URL('./schema-assets/', location.href).href;
        const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: cemSchemaAssetRuntimeFromBinding(initial.nativeResourceBinding()),
            activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: packageIds.map(packageId => ({ packageId, expectedOrigin: { kind: 'builtin' },
                replacementManifestUri: `schema-packages/${packageId}/v1/package.cem` })) }) });
        const packages = new CemSchemaAssetPackRecoveryBinding({ loader, participant: initial.generationParticipant(), current: () => !credentialsEntered,
            target: runtime => {
                if (!(runtime instanceof api.OwnedBrowserCommandInvocation)) throw Error('foreign package target');
                return { runtime: cemSchemaAssetRuntimeFromBinding(runtime.nativeResourceBinding()), participant: runtime.generationParticipant() };
            } });
        const demand = (packageId = 'csv') => {
            const demandUrl = new URL(`demand/${packageId}/1.0.0`, baseUrl).href;
            return packages.loadResource({ authoredUrl: demandUrl, resolvedUrl: demandUrl, baseUrl,
            resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'fixture:owned-invocation', method: 'GET', headers: {},
            policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' }, signal: new AbortController().signal });
        };
        const edit = JSON.stringify({ source: '{run | {import @id=input @src=data.xml | {export @id=result @out=result.xml}}}',
            edit: { node: 'result', attribute: 'out', value: 'changed.xml' } });
        const baseline = host.authorTransformConfigV1(edit), identities = [initial.runtimeId], admissions = [];
        try {
            const credentialRefusals = [];
            for (const packageId of packageIds) {
                credentialsEntered = true;
                await demand(packageId);
                ensure(host.authorTransformConfigV1(edit) === baseline, `${packageId}: credentials incorrectly fenced ordinary graph authoring`);
                const blocked = await scope.prepare().then(() => false, () => true);
                ensure(blocked, `${packageId}: entered credentials did not block group preparation`);
                credentialRefusals.push(packageId); credentialsEntered = false;
            }
            for (let round = 0; round < 2; round++) {
                const old = host.runtime, alias = old.nativeResourceBinding();
                const owner = old.claimNativeEmbedding(), other = old.claimNativeEmbedding();
                const omitted = await old.closeNativeAdmission([owner]).then(() => false, () => true);
                ensure(omitted, 'incomplete embedding ownership released packages');
                const plan = await scope.prepare(), ticket = host.prepareResume(), group = plan.createGenerationGroup();
                const missingHandoff = await old.closeNativeAdmission([owner, other]).then(() => false, () => true);
                ensure(missingHandoff, 'package cleanup accepted no checkpoint handoff');
                const oversized = await old.closeNativeAdmission([owner, other], 5000, { current: plan.current,
                    retain: () => new Uint8Array(1024 * 1024 + 1) }).then(() => false, () => true);
                ensure(oversized, 'package cleanup widened the existing command handoff budget');
                const handoff = { current: () => plan.current() && !credentialsEntered,
                    retain: () => new TextEncoder().encode('Pure invocation: no retained document, artifact, credential or external effect.') };
                const proof = await old.closeNativeAdmission([owner, other], 5000, handoff);
                ensure(proof.embeddingOwners === 2 && proof.nativeInventory.blockingResources === 0, 'package cleanup bypassed empty native census');
                const retired = await old.retireNativeGeneration(proof, [owner, other], handoff);
                ensure(alias.current() === false && alias.memory() === undefined, 'old package binding retained its heap');
                const admission = await group.allocate([retired]); admissions.push(admission);
                const partial = (() => { try { host.resume(admission); return false; } catch (error) { return error.message === 'cem.command.recovery.resources'; } })();
                ensure(partial, 'legacy resume silently omitted resource bindings');
                await resumeNativeGenerationBindings(admission, [ticket]);
                identities.push(host.runtime.runtimeId);
                ensure(host.authorTransformConfigV1(edit) === baseline, 'native authoring changed after resource restore');
                await demand();
                if (round) { admissions[0].revoke(); ensure(host.authorTransformConfigV1(edit) === baseline, 'old revocation fenced renewed package routes'); }
            }
            ensure(new Set(identities).size === 3, 'package handoff reused a physical generation');
            admissions.at(-1).revoke();
            let fenced = false; try { await demand(); } catch { fenced = true; }
            ensure(fenced, 'revoked package route remained live');
            return { rounds: 2, distinctGenerations: identities.length, credentialRefusal: true, credentialRefusals, activatedPackages: packageIds,
                ordinaryWorkWithCredentials: true,
                completeOwnership: true, checkpointRequired: true, originalCommandBudget: true, legacyPartialResumeRefused: true, staleAliasesFenced: true, automaticRecovery: false };
        } finally { packages.dispose(); loader.dispose(); host.close(); admissions.forEach(admission => admission.revoke()); scope.dispose(); }
    }, { maps, packageIds });
}


async function verifyWorkerPackageHandoff(page, maps, formats) {
    return page.evaluate(async ({ maps, formats }) => {
        const packageIds = ['csv', ...formats.map(format => format.packageId)];
        const { CemSchemaAssetPackLoader, CemSchemaAssetPackRecoveryBinding } =
            await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
        const { CemNativeRecoveryScope, resumeNativeGenerationBindings } = await import(maps.resources['@epa-wg/cem-elements/native-generation'].path);
        const { createCemMlWasmGeneration } = await import(maps.resources['@epa-wg/cem-ml/wasm/factory'].path);
        const api = await import('@epa-wg/cem-ml-cli/browser-commands');
        const ensure = (condition, message) => { if (!condition) throw Error(message); };
        const bytes = new TextEncoder().encode('id,name\n1,"Ada\n'), uri = 'memory:worker-command.csv';
        const sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(byte => byte.toString(16).padStart(2, '0')).join('');
        const version = { revision: 1, sha256 }, project = { projectId: 'worker-package-fixture', revision: 1 }, resourceVersions = { [uri]: version };
        const resources = {};
        for (const format of formats) {
            const source = new TextEncoder().encode(format.source), sourceUri = `memory:${format.path}`;
            const sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', source))].map(byte => byte.toString(16).padStart(2, '0')).join('');
            resourceVersions[sourceUri] = { revision: 1, sha256 };
            resources[sourceUri] = { bytes: [...source], identity: { contentType: format.contentType, schema: format.schema } };
        }
        const factory = createCemMlWasmGeneration(); await factory.initialize();
        let runPlan;
        const parsePlans = {};
        try { runPlan = JSON.parse(factory.call('normalizeCommandRunPlanV1', [JSON.stringify({ configBytes: [...new TextEncoder().encode(JSON.stringify({
            inputs: [{ uri, rootScope: { defaultContentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }], outputs: [],
        }))] })]));
            for (const format of formats) parsePlans[format.packageId] = JSON.parse(factory.call('normalizeCommandRunPlanV1', [JSON.stringify({
                configBytes: [...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri: `memory:${format.path}`,
                    rootScope: { defaultContentType: format.contentType, schema: format.schema } }], outputs: [] }))],
            })]));
        } finally { factory.dispose(); }
        const request = id => ({ protocolVersion: 1, requestId: id, project, resourceVersions,
            operation: { kind: 'validate', inputIds: ['input:0'], projection: 'text' }, runPlan, resources: {},
            policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } });
        const forbidden = async () => { throw Error('unexpected publication'); };
        const hostCapabilities = { currentRevision: async () => ({ project, resourceVersions }),
            readResource: async ({ uri: requested }) => requested === uri
                ? { version, bytes: [...bytes], identity: { contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }
                : { version: resourceVersions[requested], ...resources[requested] },
            prepareWrite: forbidden, commitWrite: forbidden, rollbackWrite: async () => undefined };
        const grants = { schemaPackageReplacementGrants: packageIds.map(packageId => ({ packageId, expectedOrigin: { kind: 'builtin' },
            replacementManifestUri: `schema-packages/${packageId}/v1/package.cem` })) };
        const ungranted = await api.createBrowserCommandServiceClient({ host: hostCapabilities, hostConfiguration: {} });
        const ungrantedLoader = new CemSchemaAssetPackLoader({ baseUrl: new URL('./schema-assets/', location.href).href,
            runtime: ungranted.packageResourceBinding(), activationHostConfiguration: JSON.stringify(grants) });
        try { ensure(await ungrantedLoader.activatePackages([{ packageId: 'csv', version: '1.0.0' }]).then(() => false, () => true),
            'loader configuration overrode trusted worker package grants'); }
        finally { ungrantedLoader.dispose(); await ungranted.close(); }
        let credentialsEntered = false;
        const scope = new CemNativeRecoveryScope(), host = await api.createBrowserCommandServiceBinding({ host: hostCapabilities, hostConfiguration: grants },
            { recoveryScope: scope, id: 'optional-worker-packages', hostWitness: () => () => !credentialsEntered });
        const initial = host.runtime, baseUrl = new URL('./schema-assets/', location.href).href;
        const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: initial.packageResourceBinding() });
        const packages = new CemSchemaAssetPackRecoveryBinding({ loader, participant: initial.generationParticipant(), current: () => !credentialsEntered,
            target: runtime => {
                if (!(runtime instanceof api.BrowserCommandServiceClient)) throw Error('foreign worker target');
                return { runtime: runtime.packageResourceBinding(), participant: runtime.generationParticipant() };
            } });
        const demand = (packageId = 'csv') => {
            const demandUrl = new URL(`demand/${packageId}/1.0.0`, baseUrl).href;
            return packages.loadResource({ authoredUrl: demandUrl, resolvedUrl: demandUrl, baseUrl,
            resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'fixture:worker', method: 'GET', headers: {},
            policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' }, signal: new AbortController().signal });
        };
        const identities = [initial.worker.runtimeInstanceId], admissions = [];
        const validateCsv = async id => {
            const handle = host.execute(request(id));
            const result = await handle.result();
            ensure(JSON.stringify(result).includes('cem.csv.unclosed_quote'), 'worker ignored persistent native CSV context');
            await handle.dispose();
        };
        const parseFormat = async (format, id) => {
            const sourceUri = `memory:${format.path}`;
            const handle = host.execute({ ...request(`${id}-${format.packageId}`),
                operation: { kind: 'parse', inputId: 'input:0', projection: 'ast', preserveSourceOffsets: true },
                runPlan: parsePlans[format.packageId], resources: { [sourceUri]: resources[sourceUri] } });
            try {
                const result = await handle.result();
                ensure(result.status === 'succeeded' && result.exitCode === 0,
                    `${format.packageId}: worker did not retain its native package context: ${JSON.stringify({
                        status: result.status, exitCode: result.exitCode,
                        diagnostics: result.diagnostics.items.map(({ code, severity, message }) => ({ code, severity, message })),
                    })}`);
            } finally { await handle.dispose(); }
        };
        const validate = async id => {
            await validateCsv(id);
            for (const format of formats) await parseFormat(format, id);
        };
        try {
            const credentialRefusals = [];
            for (const packageId of packageIds) {
                credentialsEntered = true;
                await demand(packageId);
                if (packageId === 'csv') await validateCsv('entered-credentials');
                else await parseFormat(formats.find(format => format.packageId === packageId), 'entered-credentials');
                ensure(await scope.prepare().then(() => false, () => true), `${packageId}: credentials did not block worker recovery`);
                credentialRefusals.push(packageId); credentialsEntered = false;
            }
            await validate('before-worker-handoff');
            ensure((() => { try { new CemSchemaAssetPackLoader({ baseUrl, runtime: initial.packageResourceBinding() }); return false; }
                catch (error) { return error.code === 'cem.schema_asset.owner_busy'; } })(), 'duplicate loader shared worker package ownership');
            for (let round = 0; round < 2; round++) {
                const old = host.runtime, alias = old.packageResourceBinding();
                const first = old.claimNativeEmbedding(), second = old.claimNativeEmbedding();
                ensure(await old.closeNativeAdmission([first]).then(() => false, () => true), 'worker cleanup accepted omitted embedding owner');
                const plan = await scope.prepare(), ticket = host.prepareResume(), group = plan.createGenerationGroup();
                ensure(await old.closeNativeAdmission([first, second]).then(() => false, () => true), 'worker package cleanup omitted handoff');
                ensure(await old.closeNativeAdmission([first, second], 5000, { current: plan.current,
                    retain: () => new Uint8Array(1024 * 1024 + 1) }).then(() => false, () => true), 'worker cleanup widened checkpoint budget');
                const handoff = { current: () => plan.current() && !credentialsEntered,
                    retain: () => new TextEncoder().encode('No artifacts, credential controls or pending external effects retained.') };
                credentialsEntered = true;
                ensure(await old.closeNativeAdmission([first, second], 5000, handoff).then(() => false, () => true), 'entered credential allowed worker package release');
                credentialsEntered = false;
                let proof = await old.closeNativeAdmission([first, second], 5000, handoff);
                proof = await old.revalidateNativeCheckpoint(proof, [first, second]);
                ensure(proof.embeddingOwners === 2 && proof.nativeInventory.blockingResources === 0, 'worker retirement skipped native empty census');
                const retired = await old.retireNativeGeneration(proof, [first, second], handoff);
                ensure(!alias.current() && alias.identity() === undefined, 'retired package alias retained worker');
                ensure(await alias.bootstrap().then(() => false, () => true), 'retired worker package action remained admitted');
                const admission = await group.allocate([retired]); admissions.push(admission);
                await resumeNativeGenerationBindings(admission, [ticket]);
                identities.push(host.worker.runtimeInstanceId);
                await validate(`after-worker-handoff-${round}`); await demand();
                if (round) { admissions[0].revoke(); await validate('after-previous-revocation'); }
            }
            ensure(new Set(identities).size === 3, 'worker package handoff reused physical generation');
            // Delay only this isolated fixture's cleanup acknowledgment. Even
            // after late cleanup succeeds, the original timed-out checkpoint must
            // remain uncertain and cannot yield a retirement receipt.
            const old = host.runtime, endpoint = old.packageResourceBinding(), physical = endpoint.identity();
            const owner = old.claimNativeEmbedding(), plan = await scope.prepare();
            plan.createGenerationGroup();
            const postMessage = physical.postMessage.bind(physical); let delayed, delayedReply;
            physical.postMessage = message => {
                if (message.type === 'cem-command-package' && message.operation === 'release') { delayed = message; return; }
                postMessage(message);
            };
            try {
                const handoff = { current: plan.current, retain: () => new Uint8Array([1]) };
                ensure(await old.closeNativeAdmission([owner], 100, handoff).then(() => false,
                    error => error.code === 'cem.command.checkpoint.timeout'), 'missing package cleanup acknowledgment granted checkpoint');
                ensure(delayed, 'cleanup delay fixture never reached native release');
                delayedReply = new Promise(resolve => {
                    const receive = event => { if (event.data?.actionId === delayed.actionId) {
                        physical.removeEventListener('message', receive); resolve();
                    } }; physical.addEventListener('message', receive);
                });
                postMessage(delayed); await delayedReply;
                ensure(await old.checkpointNative(100, [owner]).then(() => false, () => true), 'late package acknowledgment revived an uncertain checkpoint');
            } finally { physical.postMessage = postMessage; }
            admissions.at(-1).revoke();
            ensure(await Promise.resolve().then(demand).then(() => false, () => true), 'revoked worker package demand remained live');
            return { rounds: 2, distinctWorkers: identities.length, offlineDemand: true, ordinaryCsvValidation: true,
                credentialRefusal: true, credentialRefusals, nativeParses: formats.map(format => format.packageId), ordinaryWorkWithCredentials: true,
                completeOwnership: true, checkpointRequired: true, originalCommandBudget: true,
                emptyNativeInventory: true, staleAliasesFenced: true, workerOwnedGrants: true, exclusiveLoader: true, lateAcknowledgmentRefused: true, automaticRecovery: false };
        } finally { packages.dispose(); loader.dispose(); await host.close(); admissions.forEach(admission => admission.revoke()); scope.dispose(); }
    }, { maps, formats });
}


async function verifyRenderingPackageHandoff(page, maps, mode, formats) {
    return page.evaluate(async ({ maps, mode, formats }) => {
        const packageIds = ['csv', ...formats.map(format => format.packageId)];
        const { CemSchemaAssetPackLoader, CemSchemaAssetPackRecoveryBinding } =
            await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
        const { CemNativeGenerationGroup, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } =
            await import(maps.resources['@epa-wg/cem-elements/native-generation'].path);
        const domain = await import(maps.resources['@epa-wg/cem-elements/checkpoint-domain'].path);
        const native = await import(maps.resources['@epa-wg/cem-elements/cem-ql-render'].path.replace('cem-ql-render.js', 'native-runtime.js'));
        const grants = JSON.stringify({ schemaPackageReplacementGrants: packageIds.map(packageId => ({ packageId, expectedOrigin: { kind: 'builtin' },
            replacementManifestUri: `schema-packages/${packageId}/v1/package.cem` })) });
        let activeRuntime = mode === 'fallback' ? native.isolateCemFallbackGeneration({ packageHostConfiguration: grants }) : undefined;
        await activeRuntime?.ready();
        const processing = await import(maps.resources['@epa-wg/cem-elements/processing-host-runtime'].path);
        const { createCemDeclarationScope } = await import(maps.resources['@epa-wg/cem-elements/declaration-scope'].path);
        const { createCemProcessingTextSource } = await import(maps.resources['@epa-wg/cem-elements/processing-host'].path);
        const scopes = [createCemDeclarationScope({ document }), createCemDeclarationScope({ document })];
        const options = { workerScriptUrl: new URL(maps.resources['@epa-wg/cem-elements/processing-worker'].path, location.href),
            packageHostConfiguration: grants, poolPolicy: { workerCount: 1, maxWorkers: 1 },
            ...(mode === 'fallback' ? { workerFactory: () => { throw Error('isolated fallback fixture'); } } : {}) };
        let hosts = scopes.map(scope => processing.cemProcessingHostForScope(scope, options));
        await Promise.all(hosts.map(host => host.ready));
        const ensure = (condition, message) => { if (!condition) throw Error(`${mode}: ${message}`); };
        ensure(hosts.every(host => host.mode === (mode === 'fallback' ? 'main-thread' : 'worker')), 'incorrect initial physical backend');
        const initial = await processing.processingSchemaAssetBinding(hosts[0]);
        const baseUrl = new URL('./schema-assets/', location.href).href;
        const loader = new CemSchemaAssetPackLoader({ baseUrl, runtime: initial.runtime });
        let credentialsEntered = false, revision = 0;
        const packages = new CemSchemaAssetPackRecoveryBinding({ loader, participant: initial.participant, current: () => !credentialsEntered,
            target: runtime => ({ runtime: runtime.packageResourceBinding(), participant: runtime.generationParticipant() }) });
        const demand = (packageId = 'csv') => {
            const demandUrl = new URL(`demand/${packageId}/1.0.0`, baseUrl).href;
            return packages.loadResource({ authoredUrl: demandUrl, resolvedUrl: demandUrl, baseUrl,
            resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: `fixture:render-${mode}`, method: 'GET', headers: {},
            policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' }, signal: new AbortController().signal });
        };
        // The rendering document API supports CSV data import. Markdown/SCSS
        // are command-source formats; +xml imports would only prove XML parsing.
        // Their exact activation and fresh replay still pass through the native
        // owner, while command workers above exercise each source consumer.
        const csv = { packageId: 'csv', path: 'fixture.csv', source: 'id,name\n1,Ada\n', contentType: 'text/csv' };
        const consumeDocument = async (format, id) => {
            revision++;
            const host = hosts[0], handle = { documentKey: crypto.randomUUID(), instanceId: `render-${mode}`, scopePolicyStamp: 'fixture' };
            const imported = await host.document({ action: 'retain', handle, bytes: new TextEncoder().encode(format.source).buffer,
                contentType: format.contentType, sourceUri: `memory:${id}/${format.path}` }).result;
            ensure(imported.retained, `${format.packageId}: ordinary native import failed`);
            await host.document({ action: 'release', handle }).result;
        };
        const consume = async id => {
            await consumeDocument(csv, id);
            const result = await hosts[0].compile({ language: 'cem-ml', producedTag: 'cem-pack-fixture', templateArtifactId: id,
                registrationIdentity: `fixture:${id}`, source: createCemProcessingTextSource('{p | Package rendering fixture}'),
                sourceRef: { kind: 'inline', value: id }, resolverIdentity: location.href, scopePolicyStamp: 'fixture', sourceMapMode: 'dev' }).result;
            ensure(result.artifact, 'ordinary template compilation failed');
        };
        const identities = [domain.nativeCheckpointIdentity(hosts[0].checkpointClient)], admissions = [];
        try {
            const credentialRefusals = [];
            for (const packageId of packageIds) {
                credentialsEntered = true;
                await demand(packageId);
                await consumeDocument(csv, `entered-credentials-${packageId}`);
                ensure((() => { try { new CemNativeGenerationGroup([initial.participant]); return false; } catch { return true; } })(),
                    `${packageId}: entered credentials permitted group capture`);
                credentialRefusals.push(packageId); credentialsEntered = false;
            }
            await consume('before-render-handoff');
            for (let round = 0; round < 2; round++) {
                const old = hosts, before = revision, source = await processing.processingSchemaAssetBinding(old[0]);
                const alias = source.runtime, embedding = mode === 'fallback' ? activeRuntime : undefined;
                const owner = embedding?.claimOwnership(), other = embedding?.claimOwnership();
                if (embedding) ensure((() => { try { embedding.retirementConsent([owner]); return false; } catch { return true; } })(), 'omitted fallback embedding lease was accepted');
                const consent = embedding?.retirementConsent([owner, other]);
                const current = () => hosts === old && revision === before && !credentialsEntered;
                const ticket = issueNativeGenerationBindingTicket(source.participant, current, async context => {
                    const fresh = scopes.map(scope => processing.createFreshCemProcessingHost(scope, context.generation.runtime));
                    await Promise.all(fresh.map(host => host.ready));
                    return { owner: {}, current, bind: () => { ensure(context.active() && current(), 'inactive or stale host publication'); hosts = fresh; },
                        discard: () => { for (const host of fresh) void host.dispose({ reason: 'runtime-disposed' }).result.catch(() => undefined); } };
                });
                const group = new CemNativeGenerationGroup([source.participant]);
                const proof = await domain.closeNativeRenderingGeneration(old[0].checkpointClient, 5000, consent);
                const bindings = new Map(old.map((host, index) => [host.checkpointClient, [`render-owner-${index}`]]));
                const handoff = { current, retain: () => new TextEncoder().encode('No credential, pending write or external effect retained in adapter fixture.') };
                ensure(await domain.retireNativeRenderingGeneration(new Map([[old[0].checkpointClient, ['render-owner-0']]]), [], proof, handoff, 5000, consent)
                    .then(() => false, () => true), 'omitted rendering peer released packages');
                credentialsEntered = true;
                ensure(await domain.retireNativeRenderingGeneration(bindings, [], proof, handoff, 5000, consent).then(() => false, () => true), 'credential gate allowed native retirement');
                credentialsEntered = false;
                ensure(await domain.retireNativeRenderingGeneration(bindings, [], proof, { current, retain: () => new Uint8Array(4 * 1024 * 1024 + 1) }, 5000, consent)
                    .then(() => false, () => true), 'rendering handoff budget widened');
                const retired = await domain.retireNativeRenderingGeneration(bindings, [], proof, handoff, 5000, consent);
                ensure(!alias.current() && alias.identity() === undefined, 'old package endpoint retained physical identity');
                const admission = await group.allocate([retired]); admissions.push(admission);
                await resumeNativeGenerationBindings(admission, [ticket]);
                activeRuntime = admission.generations[0].runtime;
                identities.push(domain.nativeCheckpointIdentity(hosts[0].checkpointClient));
                await consume(`after-render-handoff-${round}`);
                for (const packageId of packageIds) await demand(packageId);
                await Promise.all(old.map(host => host.dispose({ reason: 'runtime-disposed' }).result.catch(() => undefined)));
                if (round) { admissions[0].revoke(); await consume('after-previous-revocation'); }
            }
            ensure(new Set(identities).size === 3, 'physical identity reused');
            admissions.at(-1).revoke();
            ensure(await Promise.resolve().then(demand).then(() => false, () => true), 'revoked package route remained active');
            return { rounds: 2, distinctGenerations: 3, offlineDemand: true, nativeCsvImport: true, nativeTemplateCompile: true,
                credentialRefusal: true, credentialRefusals, activatedPackages: packageIds, ordinaryWorkWithCredentials: true,
                completePeerCoverage: true, completeFallbackEmbedding: mode === 'fallback',
                originalRenderingBudget: true, staleAliasesFenced: true, automaticRecovery: false };
        } finally {
            packages.dispose(); loader.dispose(); admissions.forEach(admission => admission.revoke());
            await Promise.all(hosts.map(host => host.dispose({ reason: 'runtime-disposed' }).result.catch(() => undefined)));
            scopes.forEach(scope => scope.dispose());
        }
    }, { maps, mode, formats });
}
