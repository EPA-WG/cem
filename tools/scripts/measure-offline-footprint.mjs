import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { chromium } from 'playwright';
import { studioOfflinePlan, catalogOfflinePlan, verifyOfflineBodies, bodyTotals, bodyHash } from './studio-offline-footprint.mjs';
import { prepareComponentBenchmark } from './component-benchmark-deployment.mjs';

const root = resolve(import.meta.dirname, '../..');
const catalogBytes = await readFile(resolve(root, 'packages/cem-components/dist/catalog/cem.components.catalog.json'));
const catalog = JSON.parse(catalogBytes);
await prepareComponentBenchmark(root, resolve(root, 'dist/fixtures/component-browser-startup'));
const profiles = [
    { id: 'full-catalog', path: resolve(root, 'dist/fixtures/component-browser-startup'), auditWorker: true },
    { id: 'studio-default', path: resolve(root, 'packages/cem-studio/dist/static') },
    { id: 'studio-schema-assets', path: resolve(root, 'packages/cem-studio/dist/schema-assets') },
];
for (const profile of profiles) profile.plan = profile.auditWorker ? await catalogOfflinePlan(profile.path, catalog) : await studioOfflinePlan(profile.path);
let offline = false, blocked = 0;
const served = [];
const server = createServer(async (request, response) => {
    try {
        if (offline) { blocked++; response.writeHead(503).end(); return; }
        const url = new URL(request.url, 'http://local'), match = /^\/([^/]+)\/(.*)$/.exec(url.pathname);
        const profile = profiles.find(profile => profile.id === match?.[1]);
        if (!profile) { response.writeHead(404).end(); return; }
        const path = decodeURIComponent(match[2]);
        if (path === 'footprint-probe.html') {
            response.writeHead(200, { 'content-type': 'text/html', 'cache-control': 'no-store' }).end('<!doctype html><title>Isolated offline byte audit</title>'); return;
        }
        if (path === 'footprint-worker.js' && profile.auditWorker) {
            const paths = profile.plan.groups[0].files.map(file => './' + file.path);
            response.writeHead(200, { 'content-type': 'text/javascript', 'cache-control': 'no-store' }).end(`
self.addEventListener('install',event=>event.waitUntil((async()=>{const cache=await caches.open('cem-footprint:catalog');await cache.addAll(${JSON.stringify(paths)});await self.skipWaiting();})()));
self.addEventListener('activate',event=>event.waitUntil(self.clients.claim()));
self.addEventListener('fetch',event=>{if(event.request.method==='GET')event.respondWith((async()=>await(await caches.open('cem-footprint:catalog')).match(event.request)||fetch(event.request))());});`); return;
        }
        const file = resolve(profile.path, path);
        assert(file.startsWith(profile.path + sep));
        const content = await readFile(file);
        served.push({ profile: profile.id, path, bytes: content.length, sha256: bodyHash(content) });
        response.writeHead(200, { 'cache-control': 'no-store', 'content-length': content.length,
            'content-type': ({ '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json',
                '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.css': 'text/css', '.woff2': 'font/woff2', '.svg': 'image/svg+xml' })[extname(path)] ?? 'application/octet-stream' }).end(content);
    } catch (error) { response.writeHead(404).end(String(error)); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
const observations = [];
try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
    for (const profile of profiles) {
        const context = await browser.newContext({ serviceWorkers: 'allow' }), page = await context.newPage(), errors = [];
        page.on('pageerror', error => errors.push(error.message));
        const prefix = '/' + profile.id + '/', origin = `http://127.0.0.1:${server.address().port}`;
        try {
            await page.goto(origin + prefix + 'footprint-probe.html');
            await page.evaluate(async worker => {
                await navigator.serviceWorker.register('./' + worker, { scope: './', type: 'module' });
                let timer;
                try { await Promise.race([navigator.serviceWorker.ready,
                    new Promise((_, reject) => { timer = setTimeout(() => reject(Error('offline inventory installation timed out')), 30_000); })]); }
                finally { clearTimeout(timer); }
            }, profile.auditWorker ? 'footprint-worker.js' : 'service-worker.js');
            await page.waitForFunction(() => navigator.serviceWorker.controller, undefined, { timeout: 30_000 });
            await page.waitForLoadState('networkidle');
            const caches = await page.evaluate(async prefix => {
                const record = async (request, response) => {
                    const bytes = await response.arrayBuffer();
                    const sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(n => n.toString(16).padStart(2, '0')).join('');
                    return { path: decodeURIComponent(new URL(request.url).pathname.slice(prefix.length)), bytes: bytes.byteLength, sha256 };
                };
                const result = [];
                for (const name of await window.caches.keys()) {
                    const cache = await window.caches.open(name), files = [];
                    for (const request of await cache.keys()) files.push(await record(request, await cache.match(request)));
                    result.push({ name, files });
                }
                return result;
            }, prefix);
            const paths = [...new Set(profile.plan.groups.flatMap(group => group.files.map(file => file.path)))];
            await context.setOffline(true); offline = true;
            const fetched = await page.evaluate(async paths => {
                const result = [];
                for (const path of paths) {
                    const response = await fetch('./' + path, { cache: 'no-store' });
                    if (!response.ok) throw Error('offline asset unavailable: ' + path);
                    const bytes = await response.arrayBuffer();
                    const sha256 = [...new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))].map(n => n.toString(16).padStart(2, '0')).join('');
                    result.push({ path, bytes: bytes.byteLength, sha256 });
                }
                return result;
            }, paths);
            const installed = verifyOfflineBodies(profile.plan, caches, fetched);
            assert.deepEqual(errors, []); assert.equal(blocked, 0, 'offline verification reached HTTP server');
            const transferFiles = served.filter(file => file.profile === profile.id).map(({ profile, ...file }) => file);
            const declared = new Set(profile.plan.delivery.map(file => file.path));
            assert(transferFiles.every(file => declared.has(file.path)), 'installation fetched an undeclared artifact');
            const componentTags = catalog.components.filter(component => profile.plan.delivery.some(file => file.path.endsWith('/' + component.tag + '.xhtml'))).map(component => component.tag);
            observations.push({ id: profile.id, scope: profile.auditWorker ? 'all canonical declarations plus complete benchmark delivery; audit-only conservative preload'
                : 'complete built Studio delivery and actual production cache groups; capabilities remain build-profile scoped',
                ...profile.plan, installed, installationServed: bodyTotals(transferFiles), offlineFetchCount: fetched.length,
                caches: caches.map(cache => ({ name: cache.name, totals: bodyTotals(cache.files) })),
                canonicalComponents: { declared: componentTags, omitted: catalog.components.map(component => component.tag).filter(tag => !componentTags.includes(tag)) } });
            console.log(`${profile.id}: delivery ${profile.plan.deliveryTotals.bodyBytes} bytes; cached bodies ${installed.bodyBytes}; ${fetched.length} offline URL hashes verified.`);
        } finally { offline = false; await context.close(); }
    }
    const report = { contract: 'cem-offline-footprint/1', measuredAt: new Date().toISOString(), chromium: browser.version(), node: process.version,
        catalogSha256: bodyHash(catalogBytes), canonicalComponentCount: catalog.components.length,
        harnessHashes: Object.fromEntries(await Promise.all(['measure-offline-footprint.mjs', 'studio-offline-footprint.mjs', 'component-benchmark-deployment.mjs', 'demo-deployment.mjs'].map(async file => [file, bodyHash(await readFile(resolve(root, 'tools/scripts', file)))]))),
        limits: ['Cached response body bytes exclude browser metadata, indexes, filesystem allocation, headers and user projects; physical disk use is not measured.',
            'Per-file gzip level 9 is an estimate, not CDN wire delivery. Served-body counts cover installation using a thin audit probe, not application startup.',
            'The full-catalog cache worker is an audit-only preload; Studio uses its actual workers/inventories. Body availability is not complete UI/format conformance.'],
        observations };
    await mkdir(resolve(root, 'dist/reports/components-wasm'), { recursive: true });
    const encoded = JSON.stringify(report, null, 2) + '\n';
    await writeFile(resolve(root, 'dist/reports/components-wasm/offline-footprint.json'), encoded);
    await writeFile(resolve(root, 'docs/cem-components-offline-footprint.json'), encoded);
} finally { await browser?.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
