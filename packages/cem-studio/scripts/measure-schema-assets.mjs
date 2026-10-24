import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { cpus, platform, release } from 'node:os';
import { extname, resolve, sep } from 'node:path';
import { gzipSync } from 'node:zlib';
import { chromium } from 'playwright';
import { distribution, memoryObservation, summarize } from './schema-assets-measurements.mjs';

// Isolated measurement fixtures only. No application telemetry endpoint is
// widened, no source is interpreted in JS, and no real browser storage is used.
const argument = process.argv.slice(2);
assert.ok(argument.length <= 1 && (!argument.length || /^--samples=[3-9]$/.test(argument[0])), 'usage: measure-schema-assets.mjs [--samples=3..9]');
const sampleCount = argument.length ? Number(argument[0].split('=')[1]) : 5;
const project = resolve(import.meta.dirname, '..'), workspace = resolve(project, '../..');
const reportRoot = resolve(workspace, 'dist/reports/cem-studio'), reportPath = resolve(reportRoot, 'schema-assets-measurements.json');
await mkdir(reportRoot, { recursive: true });
await rm(reportPath, { force: true });
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const profiles = [];
for (const [id, directory] of [['complete', 'static'], ['external', 'schema-assets']]) {
    const root = resolve(project, 'dist', directory), entries = await loadFiles(root);
    const maps = JSON.parse(entries.get('module-map.json').bytes);
    const build = JSON.parse(entries.get('build.json').bytes);
    assert.equal(build.automaticRecovery, id === 'external' ? false : undefined);
    assert.equal(build.deploymentProfile, id === 'external' ? 'schema-assets' : undefined);
    if (id === 'external') assert.equal(build.embeddingProfile.externalBodies, true);
    const importMap = entries.get('index.html').bytes.toString().match(/<script type="importmap">[\s\S]*?<\/script>/)?.[0];
    assert.ok(importMap);
    const binaries = Object.fromEntries(['@epa-wg/cem-ml/wasm-binary', '@epa-wg/cem-elements/cem-ql-wasm'].map(key => {
        const path = maps.resources[key].path.replace(/^\.\//, '');
        const { rawBytes, gzipBytes, sha256 } = entries.get(path);
        return [key, { path, rawBytes, gzipBytes, sha256 }];
    }));
    profiles.push({ id, root, entries, maps, build, importMap, binaries,
        delivery: { files: entries.size, rawBytes: [...entries.values()].reduce((sum, file) => sum + file.rawBytes, 0),
            gzipBytes: [...entries.values()].reduce((sum, file) => sum + file.gzipBytes, 0) } });
}
// This is the original published example archive, not a live Login form value.
const archiveBytes = profiles[0].entries.get('samples/examples/login.cem-studio.json').bytes;
assert.deepEqual(profiles[1].entries.get('samples/examples/login.cem-studio.json').bytes, archiveBytes);
const archive = JSON.parse(archiveBytes);
const sourceBytes = Buffer.from(archive.contents.find(content => content.resourceId === 'login-html').base64, 'base64');
const source = sourceBytes.toString('utf8');
const fixture = { source, sourceSha256: hash(sourceBytes), sourceBytes: sourceBytes.length };
const requests = new Map();
const server = createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    const match = /^\/(complete|external)\/(.*)$/.exec(pathname);
    const profile = profiles.find(profile => profile.id === match?.[1]);
    const path = match?.[2] || 'index.html';
    if (!profile || path.split('/').includes('..')) { response.writeHead(404).end(); return; }
    const file = path === '__measure__.html' ? { bytes: Buffer.from(`<!doctype html><title>Isolated native measurement</title>${profile.importMap}`) }
        : profile.entries.get(path);
    if (!file) { response.writeHead(404).end(); return; }
    const log = requests.get(request.headers['x-cem-measurement']);
    if (log) log.push({ path: pathname, rawBytes: file.bytes.length });
    response.writeHead(200, { ...JSON.parse(profile.entries.get('security-headers.json').bytes).headers, 'cache-control': 'no-store',
        'content-type': ({ '.html': 'text/html', '.js': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm',
            '.css': 'text/css', '.xhtml': 'application/xhtml+xml', '.svg': 'image/svg+xml', '.webmanifest': 'application/manifest+json' })[extname(path)] ?? 'application/octet-stream' }).end(file.bytes);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const origin = `http://127.0.0.1:${server.address().port}`, samples = [];
let browserVersion;
try {
    for (let round = 0; round < sampleCount; round++) {
        // Alternate profile order, with a fresh browser process for every lane.
        for (const profile of round % 2 ? [...profiles].reverse() : profiles) {
            for (const workload of ['command', 'paint', 'studio-editor']) {
                const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
                browserVersion = browser.version();
                const context = await browser.newContext({ serviceWorkers: 'block', viewport: { width: 1440, height: 1000 } });
                const errors = [], cancelledRequests = [], requestLog = [];
                const sampleId = `${round}:${profile.id}:${workload}`;
                requests.set(sampleId, requestLog);
                await context.setExtraHTTPHeaders({ 'x-cem-measurement': sampleId });
                await context.route('**/*', route => new URL(route.request().url()).origin === origin ? route.continue() : route.abort());
                const page = await context.newPage();
                page.on('pageerror', error => errors.push(error.message));
                page.on('requestfailed', request => {
                    const path = new URL(request.url()).pathname, reason = request.failure()?.errorText;
                    if (reason === 'net::ERR_ABORTED') cancelledRequests.push(path);
                    else errors.push(`request failed: ${path} (${reason})`);
                });
                try {
                    let result;
                    if (workload === 'studio-editor') {
                        await page.goto(`${origin}/${profile.id}/index.html?sample=login&file=login-html`);
                        await page.waitForFunction(() => {
                            const app = globalThis.__cemStudioApplication;
                            const editor = document.querySelector('cem-source-editor[readonly] [part=control]');
                            const painted = editor && [...CSS.highlights.entries()].filter(([name]) => name.startsWith('cem-source-'))
                                .some(([, highlight]) => [...highlight].some(range => editor.contains(range.startContainer)));
                            const status = app?.packageStatus();
                            if (!app || !painted || (status && (status.state !== 'idle' || status.runtimeCount < 4 || status.demandCount !== 1))) return false;
                            globalThis.__cemMeasurementReadyMs ??= performance.now();
                            return true;
                        }, undefined, { timeout: 60_000 });
                        result = await page.evaluate(async () => {
                            const app = globalThis.__cemStudioApplication;
                            await app.workbenchView.whenSettled();
                            const editor = document.querySelector('cem-source-editor[readonly] [part=control]');
                            if (app.workbench.snapshot().navigation.selection.file !== 'login-html') throw Error('wrong selected editor resource');
                            return { timings: { navigationToPaintReadyMs: globalThis.__cemMeasurementReadyMs },
                                source: editor.textContent, packageStatus: app.packageStatus() ?? null };
                        });
                        assert.equal(result.source, source); delete result.source;
                    } else {
                        await page.goto(`${origin}/${profile.id}/__measure__.html`);
                        result = await page.evaluate(measureNative, { workload, profile: profile.id, maps: profile.maps,
                            source, hostConfiguration: profile.build.schemaPackageHostConfiguration });
                        result.memory = Object.fromEntries(Object.entries(result.memory).map(([key, value]) => [key, memoryObservation(value)]));
                        assert.ok(Object.values(result.memory).every(status => status.activeOperations === 0 && status.reservedBytes === 0 && status.recoveryReservePreserved));
                        assert.equal(result.inventory.complete, true);
                        assert.equal(result.inventory.blockingResources, 0);
                    }
                    assert.deepEqual(errors, []);
                    const sample = { round, profile: profile.id, workload, ...result, cancelledRequests: [...cancelledRequests], delivery: { responses: requestLog.length,
                        rawBodyBytes: requestLog.reduce((sum, request) => sum + request.rawBytes, 0), requests: [...requestLog] } };
                    samples.push(sample);
                    console.log(`${round + 1}/${sampleCount} ${profile.id} ${workload}: ${JSON.stringify(result.timings)}`);
                } finally { await context.close(); await browser.close(); requests.delete(sampleId); }
            }
        }
    }
    for (const workload of ['command', 'paint']) {
        assert.equal(new Set(samples.filter(sample => sample.workload === workload).map(sample => sample.resultDigest)).size, 1,
            `${workload} did not produce equivalent native results`);
    }
    const summary = Object.fromEntries(profiles.map(profile => [profile.id, Object.fromEntries(['command', 'paint', 'studio-editor'].map(workload => [workload,
        summarize(samples.filter(sample => sample.profile === profile.id && sample.workload === workload))]))]));
    const memorySummary = Object.fromEntries(profiles.map(profile => [profile.id, Object.fromEntries(['command', 'paint'].map(workload => {
        const matching = samples.filter(sample => sample.profile === profile.id && sample.workload === workload);
        return [workload, Object.fromEntries(Object.keys(matching[0].memory).map(stage => [stage, summarize(matching.map(sample => ({ timings: {
            allocatorLiveBytes: sample.memory[stage].sample.allocatorLiveBytes, allocatorPeakBytes: sample.memory[stage].sample.allocatorPeakBytes,
            linearMemoryBytes: sample.memory[stage].sample.linearMemoryBytes, workHeadroomBytes: sample.memory[stage].workHeadroomBytes,
        } })))]))];
    }))]));
    const deliverySummary = Object.fromEntries(profiles.map(profile => [profile.id, Object.fromEntries(['command', 'paint', 'studio-editor'].map(workload => [workload,
        distribution(samples.filter(sample => sample.profile === profile.id && sample.workload === workload).map(sample => sample.delivery.rawBodyBytes))]))]));
    const report = { schemaVersion: 1, status: 'passed', sampleCount, fixture: { sourceSha256: fixture.sourceSha256, sourceBytes: fixture.sourceBytes },
        environment: { node: process.version, chromium: browserVersion, platform: platform(), release: release(), cpu: cpus()[0]?.model,
            logicalCpus: cpus().length, revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: workspace, encoding: 'utf8' }).trim() },
        method: { serial: true, freshBrowserPerSample: true, freshContext: true, browserCache: 'no-store', serviceWorkers: 'blocked',
            gzipLevel: 9, clock: 'browser performance.now; Studio starts at navigation time origin',
            transport: 'uncompressed loopback HTTP; per-file gzip is a delivery estimate excluding headers and TLS',
            serverCache: 'preloaded files; OS/filesystem and host scheduler are uncontrolled',
            cancellations: 'ERR_ABORTED startup/supersession requests recorded; other transport failures refuse the run',
            comparison: 'total deployed profiles; default command/rendering binaries differ, optional adds pack ABI and native identity metadata; not a causal body-only comparison',
            memory: 'isolated actual command/rendering factory heaps; requested allocator bytes and linear capacity, not Studio aggregate/RSS or JavaScript/loader byte storage',
            studio: 'first native-painted readonly Login HTML example; readiness includes explicit optional CSV demand; not an editable keystroke or live Login credential measurement',
            timingAssertions: false, checkpointSavingsClaimed: false, automaticRecovery: false },
        profiles: profiles.map(({ id, build, binaries, delivery }) => ({ id, deploymentDigest: build.deploymentDigest ?? null, binaries, delivery })),
        summary, memorySummary, deliverySummary, samples };
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    console.log('Measured complete and external profiles; no timing threshold, checkpoint saving or RSS inference.');
} catch (error) {
    await writeFile(reportPath, `${JSON.stringify({ schemaVersion: 1, status: 'failed', sampleCount, completedSamples: samples, error: error.message }, null, 2)}\n`);
    throw error;
} finally { await new Promise(done => server.close(done)); }

async function loadFiles(root, prefix = '', result = new Map()) {
    for (const entry of (await readdir(resolve(root, prefix), { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
        const path = prefix ? `${prefix}/${entry.name}` : entry.name;
        if (entry.isDirectory()) await loadFiles(root, path, result);
        else {
            assert.ok(resolve(root, path).startsWith(`${root}${sep}`));
            const bytes = await readFile(resolve(root, path));
            result.set(path, { bytes, rawBytes: bytes.length, gzipBytes: gzipSync(bytes, { level: 9 }).length, sha256: hash(bytes) });
        }
    }
    return result;
}

async function measureNative({ workload, profile, maps, source, hostConfiguration }) {
    const ensure = (condition, message) => { if (!condition) throw Error(message); };
    const started = performance.now(), timings = {};
    const key = workload === 'command' ? '@epa-wg/cem-ml/wasm/factory' : '@epa-wg/cem-elements/cem-ql-factory';
    const factoryModule = await import(maps.resources[key].path);
    const factory = workload === 'command' ? factoryModule.createCemMlWasmGeneration() : factoryModule.createCemQlWasmGeneration();
    timings.factoryImportMs = performance.now() - started;
    const initializing = performance.now(); await factory.initialize();
    timings.initializeMs = performance.now() - initializing;
    const memory = {}, status = () => JSON.parse(factory.call('runtimeMemoryStatus', []));
    memory.initial = status();
    let loader, resultText;
    try {
        const activating = performance.now();
        if (profile === 'external' && workload === 'command') {
            const { CemSchemaAssetPackLoader, cemSchemaAssetRuntimeFromBinding } = await import(maps.resources['@epa-wg/cem-elements/dist/lib/schema-asset-pack.js'].path);
            loader = new CemSchemaAssetPackLoader({ baseUrl: new URL('./schema-assets/', location.href).href,
                runtime: cemSchemaAssetRuntimeFromBinding({ current: () => factory.memory() !== undefined,
                    memory: () => factory.memory(), call: (name, args) => factory.call(name, args) }),
                activationHostConfiguration: JSON.stringify(hostConfiguration) });
            await loader.activatePackages([{ packageId: 'csv', version: '1.0.0' }]);
        }
        timings.activationMs = performance.now() - activating;
        memory.ready = status();
        const operation = performance.now();
        if (workload === 'paint') {
            resultText = factory.call('sourceHighlightPaintV1', [JSON.stringify({ source, contentType: 'text/html' })]);
            const result = JSON.parse(resultText);
            ensure(result.status === 'painted' && result.ranges.length > 0, 'native source paint failed');
        } else {
            const bytes = new TextEncoder().encode('id,name\n1,Ada\n'), uri = 'memory:measure.csv', requestId = 'measure-csv';
            const digest = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(value => value.toString(16).padStart(2, '0')).join('');
            const version = { revision: 1, sha256: digest };
            const runPlan = JSON.parse(factory.call('normalizeCommandRunPlanV1', [JSON.stringify({ configBytes: [...new TextEncoder().encode(JSON.stringify({
                inputs: [{ uri, rootScope: { defaultContentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }], outputs: [],
            }))] })]));
            const request = { protocolVersion: 1, requestId, project: { projectId: 'measure-fixture', revision: 1 },
                resourceVersions: { [uri]: version }, operation: { kind: 'validate', inputIds: ['input:0'], projection: 'text' }, runPlan, resources: {},
                policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } };
            const forbidden = () => { throw Error('unexpected command publication'); };
            const text = await factory.call('executeCommandServiceV1', [JSON.stringify(request), JSON.stringify({ runtime: 'wasm-browser-worker',
                targetIdentity: 'fixture', abiIdentity: 'fixture', debugControlActive: false }),
                () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
                () => JSON.stringify({ version, bytes: [...bytes], identity: { contentType: 'text/csv', schema: 'https://cem.dev/ns/data/csv/1' } }),
                forbidden, forbidden, () => undefined]);
            const result = JSON.parse(text);
            ensure(!result.error && result.exitCode === 0, `native CSV validation failed: ${text}`);
            // Canonical public command outcome, excluding timing/instance IDs.
            resultText = JSON.stringify({ exitCode: result.exitCode, status: result.status, diagnostics: result.diagnostics });
            factory.call('disposeCommandArtifactsV1', [requestId]);
        }
        timings.operationMs = performance.now() - operation;
        timings.firstResultMs = performance.now() - started;
        memory.afterOperation = status();
    } finally { loader?.dispose(); }
    memory.afterCleanup = status();
    const inventory = JSON.parse(factory.call('commandNativeResourceInventory', []));
    const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(resultText));
    const resultDigest = [...new Uint8Array(digest)].map(value => value.toString(16).padStart(2, '0')).join('');
    factory.dispose();
    return { timings, memory, inventory, resultDigest };
}
