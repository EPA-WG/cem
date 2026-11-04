import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const root = resolve(workspace, 'packages/cem-studio/dist/static');
const mainPath = 'assets/@epa-wg/cem-studio/main.js';
const candidate = await readFile(resolve(root, mainPath), 'utf8');
const baselinePath = process.env.CEM_STUDIO_PREFETCH_BASELINE;
const baseline = baselinePath ? await readFile(resolve(baselinePath), 'utf8') : undefined;
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const handler = createCemStudioPreviewHandler({ root, versionedAssets: false });
const bytesPerSecond = 3.2 * 1024 * 1024, intervalMs = 40;
const active = new Set(); let requests = [], started, override;
const timer = setInterval(() => {
    const streams = [...active];
    const quota = Math.floor(bytesPerSecond * intervalMs / 1000 / Math.max(streams.length, 1));
    for (const stream of streams) {
        const end = Math.min(stream.offset + quota, stream.bytes.length);
        stream.response.write(stream.bytes.subarray(stream.offset, end)); stream.offset = end;
        if (end === stream.bytes.length) { stream.entry.completedMs = Date.now() - started; active.delete(stream); stream.response.end(); }
    }
}, intervalMs);
const server = createServer(async (request, response) => {
    const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname).slice(1);
    if (path === mainPath) { response.writeHead(200, { 'content-type': 'text/javascript', 'cache-control': 'no-store' }).end(override); return; }
    if (path.endsWith('.wasm')) {
        const bytes = await readFile(resolve(root, path));
        const entry = { path, startedMs: Date.now() - started, bytes: bytes.length }; requests.push(entry);
        response.writeHead(200, { 'content-type': 'application/wasm', 'content-length': bytes.length, 'cache-control': 'no-store' });
        const stream = { response, bytes, offset: 0, entry }; active.add(stream);
        response.on('close', () => { active.delete(stream); if (!entry.completedMs) entry.cancelledMs = Date.now() - started; });
        return;
    }
    await handler(request, response);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const browser = await chromium.launch({ headless: true });
const report = { schemaVersion: 1, browser: browser.version(), readinessDeadlineMs: 30_000,
    aggregateWasmBytesPerSecond: bytesPerSecond, serviceWorkers: 'enabled', isPublicAcceptance: false, results: [] };
try {
    for (const variant of baseline ? ['baseline', 'candidate'] : ['candidate']) {
        override = variant === 'baseline' ? baseline : candidate; requests = []; started = Date.now();
        const context = await browser.newContext(), page = await context.newPage(), errors = [];
        const result = { variant, mainSha256: hash(override), errors, requests };
        report.results.push(result);
        page.on('pageerror', error => errors.push(error.message));
        context.on('serviceworker', () => { result.serviceWorkerCreatedMs = Date.now() - started; });
        try {
            await page.goto(`http://127.0.0.1:${server.address().port}/`);
            result.readyWithinDeadline = true;
            try { await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication), undefined, { timeout: 30_000 }); }
            catch { result.readyWithinDeadline = false; }
            result.observedMs = Date.now() - started;
            result.runtimeWorkers = page.workers().map(worker => worker.url());
            console.log(`${variant}: ${JSON.stringify({ ready: result.readyWithinDeadline, elapsedMs: result.observedMs, workerCreatedMs: result.serviceWorkerCreatedMs, wasmRequests: requests.length, errors })}`);
            assert.deepEqual(errors, []);
            assert.equal(result.readyWithinDeadline, variant === 'candidate');
            if (result.readyWithinDeadline) assert.equal(result.runtimeWorkers.length, 2, 'Both original dedicated runtimes remain');
        } finally { await context.close(); }
    }
    report.status = 'passed';
} catch (error) { report.status = 'failed'; report.error = error.message; throw error; }
finally {
    await mkdir(resolve(workspace, 'dist/reports/cem-studio'), { recursive: true });
    await writeFile(resolve(workspace, 'dist/reports/cem-studio/startup-prefetch.json'), `${JSON.stringify(report, null, 2)}\n`);
    clearInterval(timer); await browser.close(); server.closeAllConnections(); await new Promise(done => server.close(done));
}
