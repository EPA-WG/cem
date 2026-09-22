#!/usr/bin/env node

// Diagnostic control data only: authored documents still use the native CEM runtime.
// Build cem-elements and cem-demo-element before running; see the stabilization review.
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { availableParallelism, cpus, release } from 'node:os';
import { extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
    concurrency: { type: 'string', default: '4' },
    batches: { type: 'string', default: '2' },
    label: { type: 'string', default: 'standalone' },
    output: { type: 'string', default: '/tmp/cem-stock-startup-timing.json' },
} });
function boundedInteger(value, name, maximum) {
    const number = Number(value);
    if (!Number.isInteger(number) || number < 1 || number > maximum) {
        throw new Error(`${name} must be an integer from 1 to ${maximum}`);
    }
    return number;
}
const concurrency = boundedInteger(values.concurrency, 'concurrency', 16);
const batches = boundedInteger(values.batches, 'batches', 20);
const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
const timeout = 45_000; // The existing gallery budget; timings are observations, not new limits.
const identityFiles = [
    'tools/scripts/diagnose-cem-stock-startup.mjs',
    'tools/scripts/verify-cem-elements-demo-fixtures.mjs',
    'packages/cem-elements/demo/cell-overrides.html',
    'packages/cem-elements/demo/stock-cell.cemt',
    'packages/cem-elements/demo/data-table-view.cemt',
    'packages/cem-elements/dist/lib/cem-elements.js',
    'packages/cem-elements/dist/lib/internal/runtime-support/processing-host-runtime.js',
    'packages/cem-elements/dist/lib/internal/runtime-support/processing-worker.js',
    'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm',
    'packages/cem-demo-element/dist/cem-demo-element.js',
];
const hashes = Object.fromEntries(await Promise.all(identityFiles.map(async path =>
    [path, createHash('sha256').update(await readFile(resolve(root, path))).digest('hex')])));
const verifier = await readFile(resolve(root, identityFiles[1]), 'utf8');
const helper = verifier.match(/const htmlDemoElementModule = `([\s\S]*?)`;/)?.[1];
if (!helper) throw new Error('Cannot locate the gallery card helper');

// Runs in the browser, using existing host observation/construction seams.
function installProbe(installCemElementRuntime) {
    const trace = window.startupTrace = { marks: {}, scheduling: [], workers: [], jobs: [] };
    const jobs = new Map();
    const tags = new Map();
    performance.setResourceTimingBufferSize(2000);
    const now = () => performance.now();
    window.runtime = installCemElementRuntime(window, {
        onProcessingTrace(event) { trace.scheduling.push({ at: now(), ...event }); },
        processingWorkerFactory({ scriptUrl, name, type }) {
            const worker = new Worker(scriptUrl, { name, type });
            const workerState = { name, created: now() };
            trace.workers.push(workerState);
            const send = worker.postMessage.bind(worker);
            worker.addEventListener('message', ({ data }) => {
                if (data.direction === 'ready') workerState.ready = now();
                if (data.direction !== 'response') return;
                const job = jobs.get(`${name}:${data.jobId}`);
                if (!job) return;
                job.received = now();
                job.outcome = data.outcome;
                job.diagnostics = data.diagnostics ?? data.result?.diagnostics ?? [];
                if (job.operation === 'compile' && data.result?.artifact) {
                    job.artifactId = data.result.artifact.artifactId;
                    job.cacheKey = data.result.artifact.cacheKey;
                    tags.set(`${name}:${job.artifactId}`, job.tag);
                }
            });
            worker.postMessage = (message, ...rest) => {
                const job = {
                    worker: name, jobId: message.jobId, operation: message.operation, sent: now(),
                    tag: message.payload?.producedTag ?? tags.get(`${name}:${message.payload?.artifact?.artifactId}`),
                };
                jobs.set(`${name}:${message.jobId}`, job);
                trace.jobs.push(job);
                send(message, ...rest);
            };
            return worker;
        },
    });
    const mark = (name, condition) => {
        if (condition && trace.marks[name] === undefined) trace.marks[name] = now();
    };
    const observer = new MutationObserver(() => {
        mark('stockOwner', document.querySelector('cem-element[tag="cem-stock-cells"]'));
        mark('stockStyles', document.querySelector('cem-element[tag="cem-stock-cells"] style[data-cem-declaration-style]'));
        mark('stockTable', document.querySelector('cem-stock-cells table'));
        mark('warning', document.querySelector('cem-stock-cells strong'));
        mark('pokemonTable', document.querySelector('cem-pokemon-cells table'));
    });
    observer.observe(document.body, { subtree: true, childList: true });
    window.mountProbe = () => {
        trace.marks.mount = now();
        const owner = document.createElement('cem-element');
        owner.setAttribute('tag', 'timed-stock-gallery');
        owner.setAttribute('src', '/packages/cem-elements/demo/cell-overrides.html');
        document.body.append(owner, document.createElement('timed-stock-gallery'));
    };
    window.readProbe = () => {
        observer.disconnect();
        return {
            ...trace,
            resources: performance.getEntriesByType('resource').map(entry => ({
                url: entry.name, start: entry.startTime, headers: entry.responseStart,
                end: entry.responseEnd, duration: entry.duration, bytes: entry.transferSize,
            })),
            stock: {
                defined: !!customElements.get('cem-stock-cells'),
                warnings: document.querySelectorAll('cem-stock-cells strong').length,
                styles: document.querySelectorAll('cem-element[tag="cem-stock-cells"] style[data-cem-declaration-style]').length,
            },
            cards: Array.from(document.querySelectorAll('cem-demo-element'), card => ({
                legend: card.getAttribute('legend'), template: !!card.querySelector(':scope > template'),
                mounted: !!card.querySelector(':scope > [slot=demo]'),
            })),
            diagnostics: Array.from(document.querySelectorAll('cem-element[tag],cem-stock-cells'), element => ({
                tag: element.getAttribute('tag') ?? element.localName,
                diagnostics: window.runtime.diagnosticsFor(element),
            })),
        };
    };
}

function harness(real) {
    return `<!doctype html><html><head>
<script type="importmap">{"imports":{"@epa-wg/cem-ml/wasm":"/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js"}}</script>
${real ? '<script type="module" src="/packages/cem-demo-element/dist/index.js"></script>' : `<script>${helper}</script>`}
</head><body><script type="module">
import {installCemElementRuntime} from '/packages/cem-elements/dist/index.js';
(${installProbe.toString()})(installCemElementRuntime);
</script></body></html>`;
}
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm',
    '.cemt': 'text/cem-ml', '.svg': 'image/svg+xml', '.css': 'text/css', '.png': 'image/png' };
const server = createServer(async (request, response) => {
    const url = new URL(request.url, 'http://local');
    if (url.pathname === '/harness.html') {
        response.setHeader('content-type', 'text/html');
        response.end(harness(url.searchParams.get('implementation') === 'real'));
        return;
    }
    const file = resolve(root, `.${decodeURIComponent(url.pathname)}`);
    if (!file.startsWith(root + sep)) { response.writeHead(403); response.end(); return; }
    try {
        const body = await readFile(file);
        response.setHeader('content-type', types[extname(file)] ?? 'text/plain');
        response.end(body);
    } catch { response.writeHead(404); response.end(); }
});

function summarize(trace) {
    const jobs = trace.jobs.filter(job => job.tag === 'cem-stock-cells');
    return {
        warningMs: trace.marks.warning - trace.marks.mount,
        stockSourceMs: trace.resources.filter(r => new URL(r.url).pathname.endsWith('/stock-cell.cemt')).map(r => r.duration),
        importMs: trace.resources.filter(r => new URL(r.url).pathname.endsWith('/data-table-view.cemt')).map(r => r.duration),
        workerReadyMs: trace.workers.map(w => w.ready - w.created),
        stockJobs: jobs.map(job => {
            const events = trace.scheduling.filter(event => event.jobId === job.jobId);
            const enqueued = events.find(event => event.kind === 'enqueue');
            const dispatched = events.find(event => event.kind === 'dispatch');
            return { operation: job.operation, outcome: job.outcome,
                queueMs: enqueued && dispatched ? dispatched.at - enqueued.at : null,
                dispatchToSendMs: dispatched ? job.sent - dispatched.at : null,
                roundTripMs: job.received === undefined ? null : job.received - job.sent };
        }),
    };
}

await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ headless: true });
const report = {
    version: 'cem-stock-startup-timing-v1', label: values.label, concurrency, batches, timeout,
    started: new Date().toISOString(), revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    changedFiles: execFileSync('git', ['status', '--short'], { cwd: root, encoding: 'utf8' }).trim(), hashes,
    environment: { node: process.version, browser: browser.version(), platform: process.platform,
        kernel: release(), cpu: cpus()[0]?.model, availableParallelism: availableParallelism() },
    runs: [],
};
try {
    for (let batch = 0; batch < batches; batch++) {
        const results = await Promise.all(Array.from({ length: concurrency }, async (_, index) => {
            const id = batch * concurrency + index;
            const implementation = id % 2 ? 'real' : 'helper';
            const started = new Date().toISOString();
            const page = await browser.newPage(); // Fresh browser context: no shared HTTP cache.
            const errors = [];
            page.on('pageerror', error => errors.push(error.message));
            page.on('requestfailed', request => errors.push(`${request.url()}: ${request.failure()?.errorText}`));
            page.on('response', response => { if (response.status() >= 400) errors.push(`${response.url()}: HTTP ${response.status()}`); });
            let failure;
            try {
                await page.goto(`http://127.0.0.1:${server.address().port}/harness.html?implementation=${implementation}`);
                await page.waitForFunction(() => typeof window.mountProbe === 'function', null, { timeout });
                await page.evaluate(() => window.mountProbe());
                await page.waitForSelector('cem-stock-cells strong', { timeout });
                await page.evaluate(() => {
                    window.stockSettled = false;
                    void Promise.all([
                        window.runtime.whenDeclarationSettled(document.querySelector('cem-element[tag="cem-stock-cells"]')),
                        window.runtime.whenRenderSettled(document.querySelector('cem-stock-cells')),
                    ]).then(() => { window.stockSettled = true; });
                });
                await page.waitForFunction(() => window.stockSettled, null, { timeout });
            } catch (error) { failure = error.message; }
            const trace = await page.evaluate(() => window.readProbe?.() ?? null);
            if (!trace || trace.stock.warnings !== 1 || trace.stock.styles !== 2 || trace.cards.length !== 3
                || trace.cards.some(card => !card.mounted || !card.template)
                || trace.diagnostics.some(entry => entry.diagnostics.length) || errors.length) {
                failure ??= 'Startup state or diagnostics failed';
            }
            const result = { id, implementation, started, finished: new Date().toISOString(),
                failure, errors, summary: trace ? summarize(trace) : null, trace };
            console.log(JSON.stringify({ id, implementation, failure, ...result.summary }));
            await page.close();
            return result;
        }));
        report.runs.push(...results);
        await writeFile(values.output, JSON.stringify(report, null, 2) + '\n');
        if (results.some(result => result.failure)) { process.exitCode = 1; break; }
    }
} finally {
    await browser.close();
    await new Promise(resolve => server.close(resolve));
}
