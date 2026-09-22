#!/usr/bin/env node
// Diagnostic control reports. Documents still enter through native CEM-ML import.
// Instrument served copies only; fail if the packaged module structure changes.
import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { cpus } from 'node:os';
import { extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { chromium } from 'playwright';

const { values } = parseArgs({ options: {
    output: { type: 'string', default: '/tmp/cem-tree-render-profile.json' },
    fixture: { type: 'string', default: 'tree' },
    'omit-island': { type: 'boolean', default: false },
} });
if (!['tree', 'table'].includes(values.fixture)) throw new Error('fixture must be tree or table');
if (values['omit-island'] && values.fixture !== 'table') throw new Error('omit-island is a table-only diagnostic');
const root = resolve(fileURLToPath(new URL('../../', import.meta.url)));
const runtimeDir = 'packages/cem-elements/dist/lib/internal/runtime-support/';
const files = [
    'tools/scripts/profile-cem-tree-render.mjs',
    'packages/cem-elements/demo/data-tree-view.cemt',
    'packages/cem-elements/demo/data-tree-request.cemt',
    'packages/cem-elements/demo/tree-source.xml',
    ...(values.fixture === 'table' ? [
        ...['data-table.html', 'data-table-view.cemt', 'data-table-aspects.cemt', 'data-table-view.xslt', 'data-table-aspects.xslt']
            .map(name => 'packages/cem-elements/demo/' + name),
        'packages/cem-demo-element/dist/index.js',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm',
    ] : []),
    ...['processing-worker.js', 'processing-engine.js', 'cem-ql-render.js', 'vendor/cem_ql_bg.wasm'].map(name => runtimeDir + name),
];
const hashes = Object.fromEntries(await Promise.all(files.map(async file =>
    [file, createHash('sha256').update(await readFile(resolve(root, file))).digest('hex')])));

function replaceOnce(source, search, replacement) {
    if (source.split(search).length !== 2) throw new Error(`Expected one instrumentation anchor: ${search}`);
    return source.replace(search, replacement);
}
function wrap(source, name, label, asynchronous = false, local = false) {
    const original = `__profileOriginal_${name}`;
    if (local) source = replaceOnce(source, `function ${name}(`, `function ${original}(`);
    else {
        let count = 0;
        source = source.replace(/import\s+(?:[\w$]+\s*,\s*)?\{[^}]*\}\s*from\s*['"][^'"]+['"]/g, statement =>
            statement.replace(new RegExp(`\\b${name}\\b`, 'g'), () => { count++; return `${name} as ${original}`; }));
        if (count !== 1) throw new Error(`Expected one import for ${name}, found ${count}`);
    }
    return source + `\n${asynchronous ? 'async ' : ''}function ${name}(...args) {
        ${values['omit-island'] && name === 'renderTemplateWithNativeValues' ? `
        // Counterfactual for this fixed fixture only. Public island access must
        // remain available in production; this is not a proposed runtime change.
        const controlEnvelope = JSON.parse(args[2]);
        delete controlEnvelope.island;
        if (controlEnvelope.datadom) delete controlEnvelope.datadom.island;
        args[2] = JSON.stringify(controlEnvelope);` : ''}
        const start = performance.now();
        try { return ${asynchronous ? 'await ' : ''}${original}(...args); }
        finally { globalThis.__cemProfile?.stages.push({stage:${JSON.stringify(label)}, start, end:performance.now(),
            ...((${JSON.stringify(name)} === 'renderTemplateWithNativeValues') ? {
                dataCharacters: args[2].length,
                // Explicit host control envelope, not imported document data.
                bindingSizes: Object.fromEntries(Object.entries(JSON.parse(args[2])).map(([key, value]) =>
                    [key, JSON.stringify(value).length])),
                datadomSizes: Object.fromEntries(Object.entries(JSON.parse(args[2]).datadom ?? {}).map(([key, value]) =>
                    [key, JSON.stringify(value).length])),
            } : {})}); }
    }\n`;
}
function instrument(name, source) {
    if (name === 'processing-worker.js') {
        source = replaceOnce(source, 'request = message;', `request = message;
        if (message.operation !== 'cancel' && message.operation !== 'dispose') {
            globalThis.__cemProfile = {jobId:message.jobId, operation:message.operation,
                revision:message.payload.revision, started:performance.now(), stages:[]};
        }`);
        return `const __post = self.postMessage.bind(self);
        self.postMessage = (message, ...options) => {
            const profile = globalThis.__cemProfile;
            if (message.direction === 'response' && profile?.jobId === message.jobId) {
                console.debug('[cem-render-profile] '+JSON.stringify({...profile, worker:self.name,
                    workerMs:performance.now()-profile.started, outcome:message.outcome}));
                globalThis.__cemProfile = undefined;
            }
            __post(message, ...options);
        };\n` + source;
    }
    if (name === 'processing-engine.js') {
        for (const fn of ['scopeRenderPlan', 'diffRenderPlansToPatchFrames', 'edgeContentAddress', 'validateRenderPlanGeneratedIds']) {
            source = wrap(source, fn, fn);
        }
        source = wrap(source, 'processRetainedCemMlTemplate', 'native-process-total', true);
        source = wrap(source, 'retainLoadedCemDocument', 'document-import-total', true);
        source = wrap(source, 'lowerResourceControls', 'resource-lowering', false, true);
    }
    if (name === 'cem-ql-render.js') {
        source = wrap(source, 'renderTemplateWithNativeValues', 'wasm-render-and-string-transfer');
        for (const fn of ['compileTemplate', 'compileTemplateArtifact', 'compileTemplateModuleClosure',
            'importTemplateArtifact', 'retainXsltComponent', 'renderXsltComponentWithNativeValues']) {
            source = wrap(source, fn, fn);
        }
        source = wrap(source, 'retainCemDocument', 'wasm-document-import');
        source = wrap(source, 'mapWasmRenderPlan', 'result-json-and-node-mapping', false, true);
        source = wrap(source, 'projectSlotsInRenderPlan', 'slot-projection');
        source = wrap(source, 'assertProcessingBoundaryValue', 'boundary-validation');
    }
    return source;
}
const copies = new Map();
for (const name of ['processing-worker.js', 'processing-engine.js', 'cem-ql-render.js']) {
    copies.set('/' + runtimeDir + name, instrument(name, await readFile(resolve(root, runtimeDir + name), 'utf8')));
}

// Runs in the browser. The two components load the unchanged authored templates.
function installProbe(installCemElementRuntime) {
    const trace = window.profileTrace = { jobs: [], schedule: [], assertions: [], marks: [] };
    window.markProfile = (phase, detail = {}) => trace.marks.push({ phase, at: performance.now(), ...detail });
    window.runtime = installCemElementRuntime(window, {
        onProcessingTrace(event) { trace.schedule.push({ at: performance.now(), ...event }); },
        processingWorkerFactory(input) {
            const worker = new Worker(input.scriptUrl, { name: input.name, type: input.type });
            const send = worker.postMessage.bind(worker);
            const pending = new Map();
            worker.postMessage = (message, ...options) => {
                const record = { jobId: message.jobId, worker: input.name, operation: message.operation,
                    tag: message.payload.snapshot?.producedTag ?? message.payload.producedTag,
                    revision: message.payload.revision, sent: performance.now(),
                    scopeUid: message.payload.scopeUid,
                    format: message.payload.snapshot?.hostAttributes.format,
                    column: message.payload.snapshot?.slices.column,
                    compare: message.payload.snapshot?.slices.mode };
                trace.jobs.push(record); pending.set(message.jobId, record);
                send(message, ...options);
            };
            worker.addEventListener('message', ({ data }) => {
                const job = pending.get(data.jobId);
                if (data.direction !== 'response' || !job) return;
                job.received = performance.now(); job.outcome = data.outcome;
                job.diagnostics = data.diagnostics ?? data.result?.diagnostics ?? [];
                pending.delete(data.jobId);
            });
            return worker;
        },
    });
    window.mount = async (tag, file, source, format) => {
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('tag', tag);
        declaration.setAttribute('src', '/packages/cem-elements/demo/' + file);
        const instance = document.createElement(tag);
        if (source) instance.textContent = source;
        if (format) instance.setAttribute('format', format);
        document.body.append(declaration, instance);
        await window.runtime.whenDeclarationSettled(declaration);
        await window.runtime.whenRenderSettled(instance);
    };
    window.assertCount = (tag, expected) => {
        const viewer = document.querySelector(tag);
        const count = viewer.querySelector('output[aria-label="Selected branches"]')?.textContent;
        const errors = window.runtime.diagnosticsFor(viewer).filter(d => ['error', 'fatal'].includes(d.severity));
        if (!viewer.isConnected || count !== String(expected) || errors.length) throw new Error(`${tag}: count=${count}, expected=${expected}, errors=${errors.length}`);
        trace.assertions.push({ tag, expected, connected: true });
    };
}
const types = { '.js': 'text/javascript', '.html': 'text/html', '.wasm': 'application/wasm', '.cemt': 'text/cem-ml', '.xml': 'application/xml', '.xslt': 'application/xslt+xml', '.css': 'text/css', '.svg': 'image/svg+xml' };
const server = createServer(async (request, response) => {
    try {
        const url = new URL(request.url, 'http://local');
        if (url.pathname === '/profile.html' || url.pathname === '/packages/cem-elements/demo/profile.html') {
            response.setHeader('content-type', 'text/html');
            response.end(`<!doctype html><script type="importmap">{"imports":{"@epa-wg/cem-ml/wasm":"/packages/cem-ml-npm/dist/wasm/browser/cem_ml.js"}}</script>
            ${values.fixture === 'table' ? '<script type="module" src="/packages/cem-demo-element/dist/index.js"></script>' : ''}
            <body><script type="module">import {installCemElementRuntime} from '/packages/cem-elements/dist/index.js';
            (${installProbe.toString()})(installCemElementRuntime);</script>`);
            return;
        }
        const file = resolve(root, '.' + decodeURIComponent(url.pathname));
        if (!file.startsWith(root + sep)) { response.writeHead(403); response.end(); return; }
        response.setHeader('content-type', types[extname(file)] ?? 'text/plain');
        response.end(copies.get(url.pathname) ?? await readFile(file));
    } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const browser = await chromium.launch({ headless: true });
const report = { version: 'cem-tree-render-profile-v1', fixture: values.fixture, omitIsland: values['omit-island'],
    started: new Date().toISOString(),
    revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    hashes, environment: { node: process.version, chromium: browser.version(), cpu: cpus()[0]?.model },
    workerProfiles: [], errors: [] };
try {
    for (const scenario of values.fixture === 'tree' ? ['tree'] : ['isolated-table', 'authored-table']) {
        const page = await browser.newPage();
        page.on('console', message => {
            if (message.text().startsWith('[cem-render-profile] ')) report.workerProfiles.push({
                ...JSON.parse(message.text().slice('[cem-render-profile] '.length)), scenario });
        });
        page.on('pageerror', error => report.errors.push(error.message));
        page.on('requestfailed', request => report.errors.push(`${request.url()}: ${request.failure()?.errorText}`));
        const profilePath = values.fixture === 'table' ? '/packages/cem-elements/demo/profile.html' : '/profile.html';
        await page.goto(`http://127.0.0.1:${server.address().port}${profilePath}`);
        await page.waitForFunction(() => !!window.mount);
        if (scenario === 'tree') {
            for (const [tag, file, source] of [
                ['profile-tree', 'data-tree-view.cemt', '<orchard xmlns:f="urn:fruit"><f:fruit color="">🍒</f:fruit><f:fruit>pre<![CDATA[<raw>🍋]]><?keep inert?>post</f:fruit></orchard>'],
                ['profile-request', 'data-tree-request.cemt', ''],
            ]) {
                await page.evaluate(args => window.mount(...args), [tag, file, source]);
                await page.waitForFunction(tag => document.querySelector(tag)?.querySelector('pre')?.textContent.includes('{ast'), tag);
                await page.evaluate(tag => window.assertCount(tag, 0), tag);
                for (let round = 0; round < 2; round++) {
                    for (const [branch, expected] of [[0, 1], [1, 2], [0, 1], [1, 0]]) {
                        await page.evaluate(async ({ tag, branch }) => {
                            const viewer = document.querySelector(tag);
                            viewer.querySelectorAll('input[type=checkbox]')[branch].click();
                            await window.runtime.whenRenderSettled(viewer);
                        }, { tag, branch });
                        await page.evaluate(({ tag, expected }) => window.assertCount(tag, expected), { tag, expected });
                    }
                }
            }

        } else {
            await page.evaluate(() => window.markProfile('mount:start'));
            if (scenario === 'authored-table') {
                await page.evaluate(() => window.mount('profile-table-page', 'data-table.html', ''));
                await page.waitForFunction(() => document.querySelectorAll('cem-data-table textarea').length === 4);
                await page.evaluate(() => window.markProfile('four-tables:ready'));
            } else {
                for (const [format, source] of [
                    ['xml', '<r><row qty="10">🍒</row><row qty="2">🍋</row><row qty="3">🍌</row></r>'],
                    ['csv', 'qty,fruit\n10,🍒\n2,🍋\n3,🍌'],
                    ['yaml', '- qty: 10\n  fruit: 🍒\n- qty: 2\n  fruit: 🍋\n- qty: 3\n  fruit: 🍌'],
                    ['json', '[{"qty":10,"fruit":"🍒"},{"qty":2,"fruit":"🍋"},{"qty":3,"fruit":"🍌"}]'],
                ]) await page.evaluate(args => window.mount(...args), ['profile-' + format, 'data-table-view.cemt', source, format]);
                await page.evaluate(() => window.markProfile('four-tables:ready'));
            }
            // The authored case starts interaction at the same four-textarea
            // boundary as the story, retaining competition from the other cards.
            for (const format of ['xml', 'csv', 'yaml', 'json']) {
                const selector = scenario === 'authored-table' ? `cem-data-table[format="${format}"]` : `profile-${format}`;
                const column = format === 'xml' ? (scenario === 'authored-table' ? '@id' : '@qty') : 'qty';
                for (const [control, value, expected] of [
                    [null, null, ['10', '2', '3']], ['Sort column', column, ['10', '2', '3']],
                    ['Compare', 'number', ['2', '3', '10']], ['Compare', 'text', ['10', '2', '3']],
                    ['Compare', 'number', ['2', '3', '10']],
                ]) {
                    await page.evaluate(async ({ selector, control, value, expected }) => {
                        const viewer = document.querySelector(selector);
                        window.markProfile('action:start', { selector, control, value });
                        if (control) {
                            const input = viewer.querySelector(`select[aria-label="${control}"]`);
                            input.value = value;
                            input.dispatchEvent(new Event('change', { bubbles: true }));
                        }
                        await window.runtime.whenRenderSettled(viewer);
                        const rows = Array.from(viewer.querySelector('table').querySelectorAll(':scope > tbody > tr'),
                            row => row.querySelector(':scope > td')?.textContent.trim());
                        if (!viewer.isConnected || JSON.stringify(rows) !== JSON.stringify(expected)) {
                            throw new Error(`${selector}: got ${rows}, expected ${expected}`);
                        }
                        if (window.runtime.diagnosticsFor(viewer).some(d => ['error', 'fatal'].includes(d.severity))) {
                            throw new Error(`${selector}: runtime diagnostics`);
                        }
                        window.profileTrace.assertions.push({ selector, control, value, expected, connected: true });
                        window.markProfile('action:verified', { selector, control, value });
                    }, { selector, control, value, expected });
                }
            }
            if (scenario === 'authored-table') {
                await page.waitForFunction(() => Array.from(document.querySelectorAll('cem-demo-element')).slice(0, 7)
                    .filter(card => card.querySelector(':scope > [slot=demo] table')).length === 7);
                await page.evaluate(() => window.markProfile('seven-cards:ready'));
            }
        }
        const trace = await page.evaluate(() => window.profileTrace);
        if (scenario === 'tree') report.trace = trace;
        else (report.cases ??= []).push({ scenario, trace });
        if (report.errors.length || trace.jobs.some(job => job.outcome !== 'success' || job.diagnostics?.length)) throw new Error('Profile diagnostics failed');
        for (const job of trace.jobs.filter(job => ['compile', 'render-diff'].includes(job.operation))) {
            const profile = report.workerProfiles.find(p => p.scenario === scenario && p.jobId === job.jobId && p.worker === job.worker);
            if (!profile) throw new Error(`Missing worker profile for job ${job.jobId}`);
            const totals = {};
            for (const span of profile.stages) totals[span.stage] = (totals[span.stage] ?? 0) + span.end - span.start;
            const stages = Object.fromEntries(Object.entries(totals).map(([name, ms]) => [name, ms.toFixed(2)]));
            console.log(JSON.stringify({ scenario, operation: job.operation, tag: job.tag, format: job.format,
                revision: job.revision?.dataRevision, roundTripMs: (job.received - job.sent).toFixed(2),
                workerMs: profile.workerMs.toFixed(2), stages }));
        }
        await page.close();
    }
} catch (error) { report.failure = error.message; process.exitCode = 1; }
finally {
    await writeFile(values.output, JSON.stringify(report, null, 2) + '\n');
    await browser.close();
    await new Promise(resolve => server.close(resolve));
}
if (report.failure) throw new Error(report.failure);
