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
assert.equal(build.deploymentProfile, 'schema-assets');
assert.equal(build.automaticRecovery, false);
assert.equal(build.packagePublication, 'explicit-host-admission');
assert.equal(build.embeddedBodies, true);
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
        let bytes = await readFile(file);
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
    await install(defaultPage);
    const defaultNames = await defaultPage.evaluate(() => caches.keys());
    assert.equal(defaultNames.length, 3);
    const page = await context.newPage();
    await openStudio(page, `${origin}/optional/index.html`);
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
    await context.setOffline(true);
    disconnected = true;
    await page.reload({ waitUntil: 'load' });
    await ready(page);
    const command = await versionCommand(page);
    assert.equal(command.exitCode, 0);
    assert.equal(command.runtime, 'wasm-browser-worker');
    const evidence = await page.evaluate(async ({ selected, maps, pin }) => {
        const { CemSchemaAssetPackLoader } = await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
        const { CemElementRuntime } = await import(maps.resources['@epa-wg/cem-elements/cem-elements'].path);
        const { createCemMlWasmGeneration } = await import(maps.resources['@epa-wg/cem-ml/wasm/factory'].path);
        const { createCemQlWasmGeneration } = await import(maps.resources['@epa-wg/cem-elements/cem-ql-factory'].path);
        const renderer = createCemQlWasmGeneration();
        await renderer.initialize();
        if (JSON.parse(renderer.call('schemaAssetPackBootstrapV1', [])).indexDigest !== pin.indexDigest) throw Error('renderer pin mismatch');
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
    assert.equal(evidence.inventoryFiles, packFiles.length);
    assert.equal(evidence.retirementRefused, true);
    const packageHandoff = await verifyOwnedPackageHandoff(page, maps);
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
        defaultCacheIsolation: true, defaultOutputPreserved: true, coldOfflineStudio: true, command, ...evidence, packageHandoff,
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

async function verifyOwnedPackageHandoff(page, maps) {
    return page.evaluate(async maps => {
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
            activationHostConfiguration: JSON.stringify({ schemaPackageReplacementGrants: [{ packageId: 'csv', expectedOrigin: { kind: 'builtin' },
                replacementManifestUri: 'schema-packages/csv/v1/package.cem' }] }) });
        const packages = new CemSchemaAssetPackRecoveryBinding({ loader, participant: initial.generationParticipant(), current: () => !credentialsEntered,
            target: runtime => {
                if (!(runtime instanceof api.OwnedBrowserCommandInvocation)) throw Error('foreign package target');
                return { runtime: cemSchemaAssetRuntimeFromBinding(runtime.nativeResourceBinding()), participant: runtime.generationParticipant() };
            } });
        const demandUrl = new URL('demand/csv/1.0.0', baseUrl).href;
        const demand = () => packages.loadResource({ authoredUrl: demandUrl, resolvedUrl: demandUrl, baseUrl,
            resolverIdentity: 'fixture', resourcePolicyStamp: 'fixture', contextIdentity: 'fixture:owned-invocation', method: 'GET', headers: {},
            policy: { allowCrossOrigin: false, maxResponseBytes: 4096, timeoutMs: 30_000, redirect: 'error' }, signal: new AbortController().signal });
        const edit = JSON.stringify({ source: '{run | {import @id=input @src=data.xml | {export @id=result @out=result.xml}}}',
            edit: { node: 'result', attribute: 'out', value: 'changed.xml' } });
        const baseline = host.authorTransformConfigV1(edit), identities = [initial.runtimeId], admissions = [];
        try {
            await demand();
            credentialsEntered = true;
            const blocked = await scope.prepare().then(() => false, () => true);
            ensure(blocked, 'entered credentials did not block group preparation'); credentialsEntered = false;
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
            return { rounds: 2, distinctGenerations: identities.length, credentialRefusal: blocked,
                completeOwnership: true, checkpointRequired: true, originalCommandBudget: true, legacyPartialResumeRefused: true, staleAliasesFenced: true, automaticRecovery: false };
        } finally { packages.dispose(); loader.dispose(); host.close(); admissions.forEach(admission => admission.revoke()); scope.dispose(); }
    }, maps);
}
