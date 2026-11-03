import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { chromium } from 'playwright';
import { createCemStudioPreviewHandler } from './serve-preview.mjs';

const workspace = resolve(import.meta.dirname, '../../..');
const root = resolve(workspace, 'packages/cem-studio/dist/static');
const reportPath = resolve(workspace, 'dist/reports/cem-studio/startup-latency.json');
const baseline = process.env.CEM_STUDIO_STARTUP_BASELINE;
const baselineBytes = baseline ? await readFile(resolve(baseline)) : undefined;
const main = 'assets/@epa-wg/cem-studio/main.js';
const hash = bytes => createHash('sha256').update(bytes).digest('hex');
const handler = createCemStudioPreviewHandler({ root, versionedAssets: false });
let override, requests = [], started;
const server = createServer(async (request, response) => {
    const path = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
    if (path === `/${main}` && override) {
        response.writeHead(200, { 'content-type': 'text/javascript', 'cache-control': 'no-store' }).end(override); return;
    }
    if (path.endsWith('.wasm')) {
        const entry = { path, startedMs: Date.now() - started };
        requests.push(entry);
        await new Promise(done => setTimeout(done, 8000));
        entry.releasedMs = Date.now() - started;
    }
    await handler(request, response);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
const results = [];
try {
    for (const variant of baselineBytes ? ['baseline', 'candidate'] : ['candidate']) {
        override = variant === 'baseline' ? baselineBytes : undefined;
        requests = []; started = Date.now();
        // Isolate the download DAG from service-worker prewarming. Normal offline
        // acceptance runs separately with workers enabled and unchanged assertions.
        const context = await browser.newContext({ serviceWorkers: 'block' });
        const page = await context.newPage(), errors = [];
        page.on('pageerror', error => errors.push(error.message));
        try {
            await page.goto(`http://127.0.0.1:${server.address().port}/`);
            let readyWithinDeadline = true;
            try { await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication), undefined, { timeout: 30_000 }); }
            catch { readyWithinDeadline = false; }
            const deadlineObservationMs = Date.now() - started;
            // Observe a failed baseline settling without relabeling its deadline.
            if (!readyWithinDeadline) await page.waitForFunction(() => Boolean(globalThis.__cemStudioApplication), undefined, { timeout: 30_000 });
            await page.evaluate(() => globalThis.__cemStudioApplication.workbenchView.whenSettled());
            const result = { variant, mainSha256: hash(override ?? await readFile(resolve(root, main))),
                readyWithinDeadline, deadlineObservationMs, settledMs: Date.now() - started,
                wasmRequests: [...requests], errors };
            results.push(result);
            console.log(`${variant}: readyWithinDeadline=${readyWithinDeadline}; deadline observation ${deadlineObservationMs} ms; ${requests.length} WASM requests`);
            assert.deepEqual(errors, []);
            assert.equal(requests.length, 4, 'Both original main/worker runtimes must still initialize');
            assert.equal(readyWithinDeadline, variant === 'candidate', `${variant} cold-start deadline outcome changed`);
        } finally { await context.close(); }
    }
} finally {
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify({ schemaVersion: 1, browser: browser.version(), delayBeforeEachWasmResponseMs: 8000,
        readinessDeadlineMs: 30_000, serviceWorkers: 'blocked-for-controlled-download-fixture',
        isPublicAcceptance: false, results }, null, 2)}\n`);
    await browser.close(); server.closeAllConnections(); await new Promise(done => server.close(done));
}
