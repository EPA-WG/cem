import { BrowserRecordedOutputHost, captureBrowserRecordedOutput, createBrowserWorkerPool, createBrowserHtmlBaseHost } from '../dist/browser.js';
import * as ql from '../../cem_ql/dist/wasm/cem_ql.js';
const source = "{style |```p{color:rgb(1,2,3)}```}\n{style @scope=widgets |```p{font-weight:700}```}\n{style |```p{background-color:rgb(4,5,6)}```}\n{result-element @name=html @namespace='http://www.w3.org/1999/xhtml' |\n {result-element @name=head @namespace='http://www.w3.org/1999/xhtml' |\n  {result-element @name=base @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=href | ../assets/}}}\n {result-element @name=body @namespace='http://www.w3.org/1999/xhtml' |\n  {result-element @name=cem-card @namespace='http://www.w3.org/1999/xhtml' |\n   {result-attribute @name=scope | widgets}\n   {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}\n   {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Card}\n   {result-element @name=cem-inner @namespace='http://www.w3.org/1999/xhtml' |\n    {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}\n    {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Inner}}}\n  {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Outside}}}";
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let failed = false; try { work(); } catch { failed = true; } check(failed, message); };
globalThis.runRecordedOutputFixture = async version => {
    await ql.default();
    const cases = [];
    const run = async (name, test, withPool = false) => {
        const frame = document.createElement('iframe');
        frame.src = '/docs/browser-recorded-output.html';
        const loaded = new Promise(resolve => { frame.onload = resolve; });
        document.body.append(frame);
        let pool;
        const hosts = [];
        try {
            await loaded;
            const doc = frame.contentDocument, view = frame.contentWindow;
            if (withPool) pool = await createBrowserWorkerPool({ workerCount: 1, hardCancelGraceMs: 10 });
            const rootUri = new URL('/templates/recorded.cemt', location.href).href;
            const request = {
                rootUri, outputUri: doc.URL, resolutionBase: rootUri, policyStamp: 'recorded:1', browser: { engine: 'chromium', version },
                sources: [{ uri: rootUri, revision: '1', source, contentType: 'text/cem-ml' }], stylesheets: [],
                choices: [{ bodyUri: rootUri, index: 0, hostPaths: [[0, 1, 0]], scope: { kind: 'private', tag: 'cem-card' } },
                    { bodyUri: rootUri, index: 1, hostPaths: [[0, 1, 0]], scope: { kind: 'shared', name: 'widgets' } },
                    { bodyUri: rootUri, index: 2, hostPaths: [[0, 1, 0, 2]], scope: { kind: 'private', tag: 'cem-inner' } }],
            };
            const policy = { stamp: 'recorded:1', decide: () => true };
            const install = html => {
                const parsed = new view.DOMParser().parseFromString(html, 'text/html');
                doc.replaceChild(doc.importNode(parsed.documentElement, true), doc.documentElement);
            };
            const native = () => new ql.RecordedOutputSession(JSON.stringify(request), 1, pool?.workers[0]?.generation ?? 1);
            const capture = (extra = {}, installer = install) => {
                const session = native();
                const options = { ownership: 'exclusive', policy, ...extra };
                const host = pool ? pool.captureRecordedOutput(doc, session, installer, options) : captureBrowserRecordedOutput(doc, session, installer, options);
                hosts.push(host); return host;
            };
            await test({ doc, view, capture, policy, native, install, hosts, pool, request });
            cases.push(name);
        } catch (error) { throw new Error(`${name}: ${String(error)}`, { cause: error }); }
        finally { hosts.forEach(host => host.dispose()); await pool?.close(); frame.remove(); }
    };
    await run('native-whole-output-scope', async ({ capture, doc, view }) => {
        const host = capture();
        await new Promise(resolve => setTimeout(resolve, 25));
        const result = host.read();
        check(result.blockers.includes('atomic-host-transaction-required') && result.blockers.includes('css-runtime-dependency-closure-required'), 'incomplete closure granted admission');
        const card = view.getComputedStyle(doc.querySelector('cem-card > p'));
        const inner = view.getComputedStyle(doc.querySelector('cem-inner > p'));
        const outside = view.getComputedStyle(doc.querySelector('body > p'));
        check(card.color === 'rgb(1, 2, 3)' && card.fontWeight === '700' && inner.backgroundColor === 'rgb(4, 5, 6)' && outside.color !== card.color && outside.fontWeight !== '700', 'native private/shared output scope mismatch');
    });
    for (const mode of ['text', 'extra-node', 'cssom', 'disabled', 'move-style', 'resize', 'load', 'policy', 'url']) {
        await run(`whole-output-${mode}`, ({ capture, doc, view, policy }) => {
            const host = capture();
            const text = doc.querySelector('cem-card > p').firstChild, old = text.nodeValue;
            const sheet = doc.querySelector('style').sheet;
            let undo = () => { /* A fresh session is required regardless of restoration. */ };
            if (mode === 'text') { text.nodeValue = 'Changed'; undo = () => { text.nodeValue = old; }; }
            if (mode === 'extra-node') { const p = doc.createElement('p'); doc.body.append(p); undo = () => p.remove(); }
            if (mode === 'cssom') { sheet.insertRule('p{color:purple}', 0); undo = () => sheet.deleteRule(0); }
            if (mode === 'disabled') { sheet.disabled = true; undo = () => { sheet.disabled = false; }; }
            if (mode === 'move-style') { const style = doc.querySelector('style'); doc.body.append(style); }
            if (mode === 'resize') view.dispatchEvent(new Event('resize'));
            if (mode === 'load') doc.querySelector('style').dispatchEvent(new Event('load'));
            if (mode === 'policy') { policy.stamp = 'changed'; undo = () => { policy.stamp = 'recorded:1'; }; }
            if (mode === 'url') { const url = doc.URL; view.history.replaceState(null, '', '?changed'); undo = () => view.history.replaceState(null, '', url); }
            rejects(() => host.read(), 'changed output accepted'); undo();
            rejects(() => host.read(), 'restoration revived native evidence');
        });
    }
    await run('controlled-away-and-back', ({ capture, doc }) => {
        const host = capture(), sheet = doc.querySelector('style').sheet;
        host.change(() => { sheet.disabled = true; sheet.disabled = false; });
        rejects(() => host.read(), 'same-value controlled change revived evidence');
        check(capture().read().blockers.length > 0, 'fresh capture failed');
    });
    await run('exclusive-owner-and-release', async ({ capture, doc }) => {
        const host = capture();
        rejects(() => capture(), 'two native preparations owned the same document');
        let rejected = false;
        try { await createBrowserHtmlBaseHost(doc, { profile: 'standard-agreement', browser: { engine: 'chromium', version }, contextOwnership: 'exclusive' }); } catch { rejected = true; }
        check(rejected, 'HTML-base host bypassed shared ownership');
        check(host.read().blockers.length > 0, 'foreign claim revoked legitimate owner');
        host.dispose(); check(capture().read().blockers.length > 0, 'release did not restore ownership');
    });
    await run('abort-and-preaborted', ({ capture }) => {
        const controller = new AbortController(); const host = capture({ signal: controller.signal }); controller.abort();
        rejects(() => host.read(), 'abort retained evidence');
        rejects(() => capture({ signal: controller.signal }), 'preaborted preparation admitted');
    });
    await run('callback-abort', ({ capture, install }) => {
        const controller = new AbortController();
        rejects(() => capture({ signal: controller.signal }, (html, stage) => { install(html); if (stage === 'composed') controller.abort(); }), 'installer abort published evidence');
        check(capture().read().blockers.length > 0, 'failed capture leaked ownership');
    });
    await run('policy-reentry', ({ doc, native, policy, install, hosts }) => {
        policy.decide = () => { rejects(() => host.read(), 'reentry accepted'); return true; };
        const host = new BrowserRecordedOutputHost(doc, native(), { ownership: 'exclusive', policy }); hosts.push(host);
        rejects(() => host.capture(install), 'swallowed reentry published evidence');
        rejects(() => host.read(), 'reentry restored evidence');
    });
    await run('policy-mutation', ({ capture, doc, policy }) => {
        policy.decide = () => { doc.body.append(doc.createElement('p')); return true; };
        rejects(() => capture(), 'policy callback write accepted');
    });
    await run('inexact-delivery', ({ capture, install, doc }) => {
        rejects(() => capture({}, html => { install(html); doc.body.append(doc.createElement('p')); }), 'wrong HTML delivery certified');
    });
    await run('async-delivery', ({ capture, install }) => {
        rejects(() => capture({}, html => { install(html); return Promise.resolve(); }), 'async delivery accepted');
    });
    await run('titled-style-rejected', ({ doc, native, install, policy, request }) => {
        // This is an authored output style, separate from the compiled declaration styles.
        request.sources[0].source = request.sources[0].source.replace('{result-element @name=base', '{result-element @name=style @namespace=\'http://www.w3.org/1999/xhtml\' | {result-attribute @name=title | day} ```p{color:red}```} {result-element @name=base');
        const session = native();
        check(session.html().includes('title="day"'), 'fixture did not author a titled output stylesheet');
        rejects(() => captureBrowserRecordedOutput(doc, session, install, { ownership: 'exclusive', policy }), 'unsupported titled output admitted');
    });
    await run('snapshot-import-flattening', ({ capture, request, doc, view }) => {
        request.sources[0].source = source.replace('p{color:rgb(1,2,3)}', "@import './child.css'; p{color:rgb(1,2,3)}");
        const url = new URL('./child.css', request.rootUri).href;
        request.stylesheets = [{ requestedUrl: url, finalUrl: url, revision: '1', source: 'p{font-style:italic}', contentType: 'text/css' }];
        const host = capture();
        check(host.read().blockers.length > 0 && !doc.querySelector('style').textContent.includes('@import'), 'native source import was not flattened');
        check(view.getComputedStyle(doc.querySelector('cem-card > p')).fontStyle === 'italic', `acquired imported rule did not apply: ${doc.querySelector('style').textContent}`);
    });
    await run('pool-close', async ({ capture, pool }) => {
        const host = capture(); await pool.close();
        rejects(() => host.read(), 'closed pool retained evidence'); rejects(() => capture(), 'closed pool admitted preparation');
    }, true);
    await run('pool-replacement-and-recovery', async ({ capture, pool }) => {
        const host = capture();
        const bytes = text => [...new TextEncoder().encode(text)];
        const operation = pool.run({ kind: 'query',
            data: { uri: 'memory:catalog.xml', fromFormat: 'xml', bytes: bytes(`<catalog>${'<book/>'.repeat(600)}</catalog>`),
                identity: { contentType: 'application/xml', schema: 'https://cem.dev/ns/data/xml/1' } },
            query: { uri: 'memory:query.xpath', bytes: bytes('count(/catalog/book)'),
                identity: { contentType: 'application/vnd.cem.xpath', schema: 'https://cem.dev/ns/query/xpath/1' } },
        });
        await new Promise(resolve => operation.subscribe(event => { if (event.kind === 'dispatch' && event.stage.label === 'execute') resolve(); }));
        await operation.cancel('recorded preparation replacement fixture');
        try { await operation; } catch { /* Cancellation is expected. */ }
        rejects(() => host.read(), 'replaced generation retained evidence');
        check(pool.workers[0].generation > 1, 'fixture failed to replace worker');
        check(capture().read().blockers.length > 0, 'replacement did not admit fresh preparation');
    }, true);
    return { cases };
};
