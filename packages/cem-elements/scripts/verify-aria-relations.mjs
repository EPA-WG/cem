import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { platform, release } from 'node:os';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { createDemoSiteHandler } from '../../../tools/scripts/serve-demo-site.mjs';

const root = resolve(import.meta.dirname, '../../..');
const output = resolve(root, 'dist/reports/cem-elements/aria-relations.json');
assert.ok(process.argv.slice(2).every(arg => arg === '--serve'), 'Usage: verify-aria-relations.mjs [--serve]');
const serveOnly = process.argv.includes('--serve');
const cases = [
    { profile: 'wai-aria-1.2-rec-20230606', targets: ['first'] },
    { profile: 'wai-aria-1.3-wd-20260604', targets: ['first', 'second', 'first'] },
    { profile: 'wai-aria-1.3-wd-20260604', targets: ['second', 'first', 'second'] },
];

// Development fixture only: exercise the published runtime's native reference
// exporter, then compare its browser AX relations to equivalent literal HTML.
const fixture = String.raw`
import { CemElementRuntime, createCemDeclarationScope } from '/packages/cem-elements/dist/index.js';
window.ariaProbePhase = 'runtime setup';
window.ariaProbe = (async () => {
    const params = new URL(location.href).searchParams;
    const mode = params.get('mode'), profile = params.get('profile'), targets = params.getAll('target');
    const invalid = params.get('invalid') === 'false' ? 'false' : 'true';
    const counters = { created: 0, ready: 0, succeeded: 0, failed: 0, fallbackRequests: 0 };
    window.ariaProbeCounters = counters;
    const scope = createCemDeclarationScope({ document });
    const runtime = new CemElementRuntime({ declarationTag: 'cem-aria-probe-declaration', declarationScope: scope,
        ariaReferenceProfile: profile, processingWorkerFactory: ({ scriptUrl, name }) => {
            if (mode === 'fallback') { counters.fallbackRequests++; throw Error('fixture selects fallback'); }
            const worker = new Worker(scriptUrl, { name, type: 'module' }); counters.created++;
            worker.addEventListener('message', ({ data }) => {
                if (data?.direction === 'ready' && data.mode === 'worker') counters.ready++;
                if (data?.direction === 'response') {
                    if (data.outcome === 'success') counters.succeeded++;
                    else counters.failed++;
                }
            });
            return worker;
        } });
    runtime.install(window);
    const declaration = document.createElement('cem-aria-probe-declaration');
    declaration.setAttribute('tag', 'cem-aria-probe');
    const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml');
    const operand = targets.length === 1 ? '#' + targets[0] : '#(' + targets.join(', ') + ')';
    template.textContent = '{cem:variable @name=first @select=\'data:read("<p>First detail</p>", "xml").root.children\'}'
        + '{cem:variable @name=second @select=\'data:read("<p>Second detail</p>", "xml").root.children\'}'
        + '{input @type=text @aria-label="Profile field" @aria-invalid=' + invalid + ' @aria-details={' + operand
        + '} @aria-errormessage={' + operand + '}}{$first}{$second}';
    declaration.append(template); document.body.append(declaration); runtime.registerDeclaration(declaration);
    const nativeGroup = document.createElement('fieldset'), nativeLegend = document.createElement('legend');
    nativeLegend.textContent = 'CEM native export'; nativeGroup.append(nativeLegend); document.body.append(nativeGroup);
    const host = document.createElement('cem-aria-probe'); host.id = 'produced'; nativeGroup.append(host);
    window.ariaProbePhase = 'native rendering';
    await runtime.whenRenderSettled(host);
    window.ariaProbePhase = 'literal control setup';
    const paragraphs = [...host.querySelectorAll('p')];
    if (paragraphs.length !== 2 || paragraphs[0].textContent !== 'First detail' || paragraphs[1].textContent !== 'Second detail')
        throw Error('Missing native target content');
    const names = ['first', 'second'];
    const ids = Object.fromEntries(paragraphs.filter(p => p.id).map(p => [p.id, names[paragraphs.indexOf(p)]]));
    const control = document.createElement('fieldset'); control.id = 'literal';
    const legend = document.createElement('legend'); legend.textContent = 'Literal HTML control'; control.append(legend);
    const input = document.createElement('input'); input.type = 'text'; input.setAttribute('aria-label', 'Profile field');
    input.setAttribute('aria-invalid', invalid);
    for (const attribute of ['aria-details', 'aria-errormessage']) input.setAttribute(attribute, targets.map(name => 'literal-' + name).join(' '));
    control.append(input);
    for (let index = 0; index < names.length; index++) {
        const p = document.createElement('p'); p.id = 'literal-' + names[index]; p.textContent = paragraphs[index].textContent;
        ids[p.id] = names[index]; control.append(p);
    }
    document.body.append(control);
    window.ariaProbePhase = 'ready';
    return { counters, ids, profile: runtime.snapshotInstance(host).ariaReferenceProfile,
        diagnostics: runtime.diagnosticsFor(host).map(d => ({ code: d.code, severity: d.severity })) };
})();
window.ariaProbe.then(state => { window.ariaProbeResult = state; }, error => { window.ariaProbeError = error.message; });
`;

const staticHandler = createDemoSiteHandler({ root });
const server = createServer((request, response) => {
    const url = new URL(request.url, 'http://localhost');
    if (url.pathname === '/__aria-probe__/fixture.mjs') response.writeHead(200, { 'content-type': 'text/javascript' }).end(fixture);
    else if (url.pathname === '/__aria-probe__/') response.writeHead(200, { 'content-type': 'text/html' }).end(
        `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>ARIA relationship fixture</title></head><body><script type="module" src="fixture.mjs${url.search.replaceAll('&', '&amp;')}"></script></body></html>`);
    else if (url.pathname === '/favicon.ico') response.writeHead(204).end();
    else void staticHandler(request, response);
});

async function observe(cdp, page, selector, ids) {
    await page.locator(selector).focus();
    const { root: document } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: document.nodeId, selector });
    const { node } = await cdp.send('DOM.describeNode', { nodeId });
    const { nodes } = await cdp.send('Accessibility.getPartialAXTree', { nodeId, fetchRelatives: false });
    const ax = nodes.find(value => value.backendDOMNodeId === node.backendNodeId);
    assert.ok(ax && !ax.ignored, 'The exact input must be exposed in the browser AX tree');
    const property = name => ax.properties?.find(item => item.name === name)?.value;
    const relation = async name => {
        const value = property(name);
        if (!value) return null;
        const targets = await Promise.all((value.relatedNodes ?? []).map(async target => {
            // Resolve the actual related DOM node, not just the AX idref string.
            const { node } = await cdp.send('DOM.describeNode', { backendNodeId: target.backendDOMNodeId });
            const idIndex = node.attributes?.indexOf('id') ?? -1;
            const id = idIndex >= 0 ? node.attributes[idIndex + 1] : undefined;
            assert.ok(ids[id], `Unknown AX target ${id}`);
            if (target.idref !== undefined) assert.equal(id, target.idref);
            return ids[id];
        }));
        return { type: value.type, targets };
    };
    return { role: ax.role?.value, name: ax.name?.value, invalid: property('invalid')?.value,
        details: await relation('details'), errorMessage: await relation('errormessage') };
}

const report = { schemaVersion: 1, checkedAt: new Date().toISOString(), status: 'started',
    environment: { platform: platform(), kernel: release(), node: process.version, headless: true, assistiveTechnology: null },
    scope: 'Chromium CDP accessibility tree; not OS accessibility mappings, announcements or physical AT acceptance',
    results: [], identity: [] };
let browser;
try {
    report.environment.osRelease = await readFile('/etc/os-release', 'utf8').catch(() => null);
    for (const path of ['packages/cem-elements/dist/index.js', 'packages/cem-elements/dist/lib/cem-elements.js',
        'packages/cem-elements/dist/lib/aria-reference-profile.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/cem-ql-render.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/processing-worker.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm']) {
        const bytes = await readFile(resolve(root, path));
        report.identity.push({ path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex') });
    }
    await new Promise(done => server.listen(0, '127.0.0.1', done));
    if (serveOnly) {
        for (const mode of ['worker', 'fallback']) for (const entry of cases) for (const invalid of ['true', 'false']) {
            const params = new URLSearchParams({ mode, profile: entry.profile, invalid });
            for (const target of entry.targets) params.append('target', target);
            console.log(`http://127.0.0.1:${server.address().port}/__aria-probe__/?${params}`);
        }
        console.log('Manual fixture server only; no browser/AT acceptance is recorded. Stop with Ctrl+C.');
        await new Promise(done => { process.once('SIGINT', done); process.once('SIGTERM', done); });
    } else {
        browser = await chromium.launch({ headless: true }); report.environment.chromium = browser.version();
        for (const mode of ['worker', 'fallback']) for (const entry of cases) {
            const context = await browser.newContext({ serviceWorkers: 'block' });
            const errors = [], failedRequests = [], httpFailures = [];
            let page;
            const result = { mode, profile: entry.profile, targets: entry.targets, status: 'started', states: [] };
            report.results.push(result);
            try {
                page = await context.newPage(); page.setDefaultTimeout(30_000);
                page.on('pageerror', error => errors.push(error.message));
                context.on('requestfailed', request => failedRequests.push({ path: new URL(request.url()).pathname,
                    type: request.resourceType(), owner: request.serviceWorker?.() ? 'service-worker' : 'page-or-dedicated-worker',
                    failure: request.failure()?.errorText?.match(/net::ERR_[A-Z0-9_]+/)?.[0] ?? 'request-failed' }));
                context.on('response', response => { if (response.status() >= 400) httpFailures.push(response.status()); });
                const cdp = await context.newCDPSession(page); await cdp.send('Accessibility.enable');
                const params = new URLSearchParams({ mode, profile: entry.profile, invalid: mode === 'worker' ? 'true' : 'false' });
                for (const target of entry.targets) params.append('target', target);
                await page.goto(`http://127.0.0.1:${server.address().port}/__aria-probe__/?${params}`);
                await page.waitForFunction(() => window.ariaProbeResult || window.ariaProbeError);
                const failure = await page.evaluate(() => window.ariaProbeError);
                assert.equal(failure, undefined, 'Native fixture must render successfully');
                const state = await page.evaluate(() => window.ariaProbeResult);
                assert.ok(state, 'Native fixture must complete'); assert.equal(state.profile, entry.profile);
                result.initialInvalid = await page.locator('#produced input').getAttribute('aria-invalid');
                assert.equal(result.initialInvalid, params.get('invalid'), 'Manual URL must select the initial native invalid state');
                result.counters = state.counters; result.diagnostics = state.diagnostics;
                assert.deepEqual(state.diagnostics.filter(item => ['error', 'fatal'].includes(item.severity)), []);
                if (mode === 'worker') {
                    assert.ok(state.counters.created > 0 && state.counters.ready > 0 && state.counters.succeeded > 0);
                    assert.equal(state.counters.failed, 0); assert.equal(state.counters.fallbackRequests, 0);
                } else { assert.equal(state.counters.created, 0); assert.ok(state.counters.fallbackRequests > 0); }
                for (const invalid of ['true', 'false']) {
                    // This changes only browser invalid-state exposure, not exported reference sequences.
                    await page.locator('input').evaluateAll((inputs, value) => inputs.forEach(input => input.setAttribute('aria-invalid', value)), invalid);
                    const dom = await page.locator('#produced input').evaluate((input, ids) => Object.fromEntries(
                        ['aria-details', 'aria-errormessage'].map(name => [name, input.getAttribute(name)?.split(/\s+/).map(id => ids[id])])), state.ids);
                    const observation = { invalid, dom }; result.states.push(observation);
                    for (const value of Object.values(dom)) assert.deepEqual(value, entry.targets, 'Native DOM must retain authored order and repeats');
                    const produced = await observe(cdp, page, '#produced input', state.ids);
                    const literal = await observe(cdp, page, '#literal input', state.ids);
                    Object.assign(observation, { produced, literal });
                    assert.deepEqual(produced, literal, 'Native-exported AX relations must match the literal browser control');
                    assert.equal(produced.role, 'textbox'); assert.equal(produced.name, 'Profile field');
                    assert.equal(produced.invalid, invalid);
                    assert.ok(produced.details?.targets.length, 'Chromium must expose a resolved details relation');
                    if (invalid === 'true') assert.ok(produced.errorMessage?.targets.length, 'Invalid input must expose a resolved error-message relation');
                    else assert.equal(produced.errorMessage, null, 'Valid input must omit the error-message relation');
                }
                assert.deepEqual(errors, []); assert.deepEqual(failedRequests, []); assert.deepEqual(httpFailures, []);
                result.status = 'passed';
                console.log(`${mode}/${entry.profile}/${entry.targets.join(',')}: native DOM and literal AX parity passed`);
            } catch (error) { result.status = 'failed'; result.error = error.message; throw error; }
            finally {
                // Snapshot before teardown can generate aborted-request events.
                result.initialization = await page?.evaluate(() => ({ phase: window.ariaProbePhase ?? 'module entry not executed',
                    counters: window.ariaProbeCounters, error: window.ariaProbeError })).catch(() => null);
                result.browserErrors = [...errors]; result.failedRequests = [...failedRequests]; result.httpFailures = [...httpFailures];
                await context.close();
            }
        }
        report.status = 'passed';
    }
} catch (error) { report.status = 'failed'; report.error = error.message; throw error; }
finally {
    report.finishedAt = new Date().toISOString();
    if (!serveOnly) {
        await mkdir(resolve(output, '..'), { recursive: true }); await writeFile(output, JSON.stringify(report, null, 2) + '\n');
    }
    await browser?.close(); server.closeAllConnections(); await new Promise(done => server.close(done));
}
