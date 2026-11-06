import assert from 'node:assert/strict';
import http from 'node:http';
import { mkdirSync, writeFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';
const root = path.resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const server = http.createServer(async (req, res) => {
    try {
        const pathname = new URL(req.url, 'http://localhost').pathname;
        if (pathname === '/fixture') { res.setHeader('Content-Type', 'text/html'); res.end('<!doctype html><html><body></body></html>'); return; }
        const vendor = '/packages/cem-elements/dist/lib/internal/runtime-support/vendor/';
        const local = pathname.startsWith(vendor) && ['cem_ql.js', 'cem_ql_bg.wasm'].includes(path.basename(pathname))
            ? `/packages/cem_ql/dist/wasm/${path.basename(pathname)}` : pathname;
        const file = path.resolve(root, `.${decodeURIComponent(local)}`);
        assert.ok(file.startsWith(`${root}${path.sep}`));
        res.setHeader('Content-Type', { '.js': 'text/javascript', '.mjs': 'text/javascript', '.json': 'application/json', '.wasm': 'application/wasm' }[path.extname(file)] || 'application/octet-stream');
        res.end(await readFile(file));
    } catch (error) { console.error(`Fixture asset unavailable: ${req.url}: ${error.message}`); res.writeHead(404); res.end('missing'); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
let browser;
try {
    browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
    const origin = `http://127.0.0.1:${server.address().port}`;
    const results = [];
    const external = [];
    for (const profile of ['fallback-plan', 'fallback-legacy', 'fallback-retained', 'worker-retained']) {
        const page = await browser.newPage();
        await page.route('**/*', async route => {
            if (new URL(route.request().url()).origin === origin) await route.continue();
            else { external.push(route.request().url()); await route.abort(); }
        });
        await page.goto(`${origin}/fixture`);
        let timer;
        try {
            const result = await Promise.race([page.evaluate(async profile => {
                const { CemElementRuntime } = await import('/packages/cem-elements/dist/lib/cem-elements.js');
                const trace = [];
                const runtime = new CemElementRuntime({ declarationTag: 'test-declaration', onProcessingTrace: event => trace.push(event),
                    ...(profile.startsWith('fallback') ? { processingWorkerFactory: () => { throw Error('fixture fallback'); } } : {}),
                    ...(profile.endsWith('legacy') || profile.endsWith('plan') ? { retainedStylesheets: false } : {}),
                });
                runtime.install(window);
                // Fixture-only routing isolates the production full-plan publisher without creating resources.
                runtime.usesProcessingHost = () => !profile.endsWith('plan');
                const container = document.createElement('div');
                container.innerHTML = `<test-declaration tag="x-commit"><template id="x-commit" type="text/cem-ml">{attribute @name=label @select='datadom.attributes.source'}{attribute @name=data-mode @select='datadom.attributes.source'}{p | {$ label}}</template></test-declaration><x-commit source="first"><template><b slot="title">Original</b></template></x-commit>`;
                const host = container.querySelector('x-commit');
                const originalPayload = host.querySelector('template').content.firstElementChild;
                document.body.append(container);
                const declaration = container.querySelector('test-declaration');
                runtime.registerDeclaration(declaration);
                await runtime.whenDeclarationSettled(declaration);
                await runtime.whenRenderSettled(host);
                const island = host.querySelector('template[data-cem-island]');
                const context = island.content.firstElementChild;
                const section = name => Array.from(context.children).find(node => node.localName.endsWith(`:${name}`));
                const entries = (name, key) => Object.fromEntries(Array.from(section(name).children).map(node => [node.getAttribute(key), node.getAttribute('value') ?? node.textContent]));
                const payload = section('payload');
                const observe = () => ({ label: host.getAttribute('label'), mode: host.getAttribute('data-mode'), source: host.getAttribute('source'),
                    canonicalLabel: entries('attributes', 'name').label, canonicalMode: entries('dataset', 'key').mode,
                    text: host.querySelector('p')?.textContent, revision: entries('data', 'name').dataRevision,
                    renderedRevision: host.querySelector('p')?.getAttribute('data-cem-data-revision'), payloadSame: payload.firstElementChild === originalPayload });
                function check(value, name) { if (!value) throw Error(`${profile}: ${name}`); }
                let current = observe();
                check(current.label === 'first' && current.mode === 'first' && current.canonicalLabel === 'first' && current.canonicalMode === 'first', 'initial host/island mismatch');
                check(current.revision === current.renderedRevision && current.payloadSame, 'initial revision or payload mismatch');
                const revision = current.revision;
                await new Promise(resolve => setTimeout(resolve, 30));
                check(observe().revision === revision, 'host synchronization recursively rendered');
                // Fixture-only interception rejects publication after native planning.
                const prepare = runtime.prepareHostAttributeCommit.bind(runtime);
                let attempts = 0;
                let originalSections;
                const originalBody = host.querySelector('p');
                runtime.prepareHostAttributeCommit = (...args) => {
                    const publication = prepare(...args);
                    if (!publication || args[3].hostAttributes.source !== 'fail') return publication;
                    attempts += 1;
                    originalSections = [section('data'), section('attributes'), section('dataset')];
                    return { cancel: () => publication.cancel(), publish: (_commit, succeeded) => publication.publish(() => { throw Error('fixture rejected publication'); }, succeeded) };
                };
                host.setAttribute('source', 'fail');
                await runtime.whenRenderSettled(host);
                current = observe();
                check(attempts >= 1 && attempts <= 2, 'unexpected recovery/re-render count');
                check(current.label === 'first' && current.mode === 'first' && current.canonicalLabel === 'first' && current.canonicalMode === 'first', 'failed publication leaked host/island writes');
                check(current.source === 'fail' && host.querySelector('p') === originalBody && current.text === 'first' && current.payloadSame, 'rollback replaced input, output or payload');
                check(originalSections.every((node, index) => node === [section('data'), section('attributes'), section('dataset')][index]), 'rollback lost canonical section owners');
                runtime.prepareHostAttributeCommit = prepare;
                host.setAttribute('source', 'recovered');
                await runtime.whenRenderSettled(host);
                current = observe();
                check(current.label === 'recovered' && current.canonicalLabel === 'recovered' && current.canonicalMode === 'recovered' && current.text === 'recovered', 'authoritative recovery failed');
                check(current.revision === current.renderedRevision && current.payloadSame, 'recovery changed revision or payload ownership');
                let injected = false;
                runtime.prepareHostAttributeCommit = (...args) => {
                    const publication = prepare(...args);
                    if (publication && !injected && args[3].hostAttributes.source === 'stale') { injected = true; host.setAttribute('source', 'latest'); }
                    return publication;
                };
                host.setAttribute('source', 'stale');
                await runtime.whenRenderSettled(host);
                current = observe();
                check(injected && current.source === 'latest' && current.label === 'latest' && current.canonicalLabel === 'latest' && current.canonicalMode === 'latest' && current.text === 'latest', 'stale input overwrote a newer source');
                check(current.revision === current.renderedRevision && current.payloadSame, 'latest frame revision or payload mismatch');
                const settledRevision = current.revision;
                await new Promise(resolve => setTimeout(resolve, 30));
                check(observe().revision === settledRevision, 'publication triggered recursive rendering');
                if (profile.startsWith('worker')) check(trace.some(event => event.kind === 'dispatch'), `no processing dispatch observed: ${JSON.stringify(trace)}`);
                if (!profile.endsWith('plan')) check(profile.startsWith('fallback') ? trace.some(event => event.kind === 'fallback') : !trace.some(event => event.kind === 'fallback'), `processing profile did not use its expected worker/fallback path: ${JSON.stringify(trace)}`);
                return { profile, workerFallback: trace.some(event => event.kind === 'fallback'), rollbackAttempts: attempts, revisionStable: true, payloadIdentityRetained: true, canonicalStateMatches: true, staleInputRefused: true };
            }, profile), new Promise((_, reject) => { timer = setTimeout(() => reject(Error(`${profile}: browser publication timed out`)), 60_000); })]);
            results.push(result);
        } finally { clearTimeout(timer); await page.close(); }
    }
    assert.deepEqual(external, []);
    const report = path.join(root, 'dist/reports/cem-browser-host-publication.json');
    mkdirSync(path.dirname(report), { recursive: true });
    writeFileSync(report, `${JSON.stringify({ browser: browser.version(), authoredJavaScript: false, trustedRuntime: true, externalRequests: external, results }, null, 2)}\n`);
    console.log(`Browser host/island publication: ${results.length}/${results.length} profiles`);
} finally { await browser?.close(); await new Promise(resolve => server.close(resolve)); }
