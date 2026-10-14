import { createBrowserHtmlBaseHost, createBrowserWorkerPool } from '../dist/browser.js';
import { retainCemDocument, disposeCemDocument } from '@epa-wg/cem-ml/wasm';
import source from '../../cem_ml/tests/fixtures/browser-stylesheets.xhtml?raw';
const theme = '@import "child.css" supports(display: grid) screen and (min-width: 200px); p{background-color:rgb(4,5,6)}';
const child = 'p{border-style:solid}';
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let rejected = false; try { work(); } catch { rejected = true; } check(rejected, message); };

globalThis.runBrowserStylesheetsFixture = async (version) => {
    const passed = [];
    const run = async (name, test, { text = source.trim(), responses = true, pool = false } = {}) => {
        const frame = document.createElement('iframe');
        frame.src = '/docs/browser-stylesheets.html';
        const loaded = new Promise((resolve, reject) => { frame.onload = resolve; frame.onerror = reject; });
        document.body.append(frame);
        let html, css, workers;
        const native = retainCemDocument(new TextEncoder().encode(text), 'application/xml', 'studio://fixture/browser-stylesheets.xhtml');
        try {
            await loaded;
            const doc = frame.contentDocument;
            html = await createBrowserHtmlBaseHost(doc, { profile: 'chromium-148', browser: { engine: 'chromium', version }, contextOwnership: 'exclusive' });
            html.capture(native, { stamp: 'allow:1', decide: () => true });
            const sources = responses ? [ ['theme.css', theme], ['child.css', child] ].map(([path, text]) => ({
                requestedUrl: new URL(path, doc.baseURI).href, finalUrl: new URL(path, doc.baseURI).href,
                revision: 'fixture:1', contentType: 'text/css; charset=utf-8', text,
            })) : [];
            if (pool) workers = await createBrowserWorkerPool({ workerCount: 1, hardCancelGraceMs: 10 });
            const capture = (options = { ownership: 'exclusive' }, entries = sources) => workers
                ? workers.captureStylesheets(html, entries, options) : html.captureStylesheets(entries, options);
            await test({ html, doc, view: frame.contentWindow, capture, workers, setCss: (value) => { css = value; } });
            passed.push(name);
        } catch (error) { throw new Error(`${name}: ${String(error)}`, { cause: error }); }
        finally { css?.dispose(); html?.dispose(); await workers?.close(); disposeCemDocument(native); frame.remove(); }
    };
    await run('native-source-import-and-cssom-correspondence', ({ capture, doc, view, setCss }) => {
        const css = capture(); setCss(css);
        const result = css.read();
        check(result.occurrences === 4, 'root/import occurrence coverage');
        check(!result.blockers.includes('stylesheet-execution-history-required'), 'history blocker retained');
        check(result.blockers.includes('atomic-host-transaction-required'), 'observation granted an edit');
        check(view.getComputedStyle(doc.querySelector('p')).backgroundColor === 'rgb(4, 5, 6)', 'external sheet not actually applied');
        check(view.getComputedStyle(doc.querySelector('p')).borderTopStyle === 'solid', 'supports import not applied');
    });
    await run('controlled-cssom-away-and-back', ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        const sheet = doc.querySelector('style').sheet;
        css.change(() => { sheet.insertRule('p{color:blue}', 0); sheet.deleteRule(0); });
        rejects(() => css.read(), 'CSSOM A-B-A revived evidence');
        check(css.capture().occurrences === 4, 'fresh post-mutation capture failed');
    });
    await run('selection-disable-and-restore', ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        const sheet = doc.querySelector('link').sheet;
        css.change(() => { sheet.disabled = true; });
        rejects(() => css.read(), 'selection mutation retained evidence');
        css.capture();
        css.change(() => { sheet.disabled = false; });
        rejects(() => css.read(), 'selection restore revived evidence');
        css.capture();
    });
    await run('unowned-cssom-source-change', ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        doc.querySelector('style').sheet.insertRule('p{color:blue}', 0);
        rejects(() => css.read(), 'CSSOM source mismatch accepted');
        doc.querySelector('style').sheet.deleteRule(0);
        rejects(() => css.capture(), 'mismatch restoration revived disposed session');
    });
    await run('wrong-acquired-response', ({ capture }) => {
        const origin = new URL('/styles/', location.href);
        rejects(() => capture({ ownership: 'exclusive' }, [ ['theme.css', theme], ['child.css', 'p{border-style:dashed}'] ].map(([path, text]) => ({
            requestedUrl: new URL(path, origin).href, finalUrl: new URL(path, origin).href,
            revision: 'wrong:1', contentType: 'text/css', text,
        }))), 'caller-supplied source key was trusted without CSSOM comparison');
    });
    await run('missing-acquisition', ({ capture }) => rejects(() => capture(), 'missing import snapshot fell back to fetch'), { responses: false });
    await run('unowned-dom-same-value', ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        const link = doc.querySelector('link'); link.setAttribute('href', link.getAttribute('href'));
        rejects(() => css.read(), 'unowned same-value load assignment retained evidence');
    });
    for (const event of ['load', 'error']) await run(`load-event-${event}`, ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        doc.querySelector('link').dispatchEvent(new Event(event));
        rejects(() => css.read(), 'load history did not invalidate before read');
        css.capture();
    });
    await run('environment-event', ({ capture, view, setCss }) => {
        const css = capture(); setCss(css);
        view.dispatchEvent(new Event('resize'));
        rejects(() => css.read(), 'environment change retained evidence');
        css.capture();
    });
    await run('actual-media-transition', async ({ capture, view, setCss }) => {
        const css = capture(); setCss(css);
        const changed = new Promise(resolve => view.addEventListener('resize', resolve, { once: true }));
        view.frameElement.style.width = '50px';
        await changed;
        check(!view.matchMedia('(min-width: 200px)').matches, 'viewport did not cross import media boundary');
        rejects(() => css.read(), 'media transition retained evidence');
        check(css.capture().occurrences === 4, 'inactive import capture failed');
    });
    await run('unowned-cssom-media', ({ capture, doc, setCss }) => {
        const css = capture(); setCss(css);
        doc.querySelector('link').sheet.media.mediaText = 'print';
        rejects(() => css.read(), 'CSSOM media diverged from retained root without rejection');
    });
    await run('lifecycle-abort', ({ capture, setCss }) => {
        const controller = new AbortController();
        const css = capture({ ownership: 'exclusive', signal: controller.signal }); setCss(css);
        controller.abort();
        rejects(() => css.read(), 'aborted source lifetime retained evidence');
        rejects(() => css.capture(), 'aborted lifetime recaptured');
    });
    await run('html-context-change', ({ html, capture, setCss }) => {
        const css = capture(); setCss(css);
        html.changeContext(() => { /* same-value context transition */ });
        rejects(() => css.read(), 'HTML invalidation did not revoke CSS evidence');
    });
    await run('callback-reentry', ({ capture, setCss }) => {
        const css = capture(); setCss(css);
        rejects(() => css.change(() => { rejects(() => css.read(), 'reentry accepted'); }), 'swallowed reentry restored session');
        rejects(() => css.capture(), 'reentrant host survived');
    });
    await run('async-change', ({ capture, setCss }) => {
        const css = capture(); setCss(css);
        rejects(() => css.change(() => Promise.resolve()), 'async CSSOM changes admitted');
        rejects(() => css.read(), 'async change left evidence');
    });
    await run('worker-pool-close', async ({ capture, workers, setCss }) => {
        const css = capture(); setCss(css);
        check(css.read().occurrences === 4 && workers.size === 1, 'worker-owned session absent');
        await workers.close();
        rejects(() => css.read(), 'worker close left stylesheet evidence alive');
        rejects(() => capture(), 'closed pool admitted new stylesheet capture');
    }, { pool: true });
    await run('worker-generation-replacement', async ({ capture, workers, setCss }) => {
        const css = capture(); setCss(css);
        const bytes = text => [...new TextEncoder().encode(text)];
        const operation = workers.run({ kind: 'query',
            data: { uri: 'memory:catalog.xml', fromFormat: 'xml', bytes: bytes(`<catalog>${'<book/>'.repeat(600)}</catalog>`),
                identity: { contentType: 'application/xml', schema: 'https://cem.dev/ns/data/xml/1' } },
            query: { uri: 'memory:query.xpath', bytes: bytes('count(/catalog/book)'),
                identity: { contentType: 'application/vnd.cem.xpath', schema: 'https://cem.dev/ns/query/xpath/1' } },
        });
        await new Promise(resolve => operation.subscribe(event => { if (event.kind === 'dispatch' && event.stage.label === 'execute') resolve(); }));
        await operation.cancel('stylesheet lifecycle fixture');
        try { await operation; } catch { /* cancellation is expected */ }
        check(workers.workers[0].generation > 1, 'fixture did not replace its busy worker');
        rejects(() => css.read(), 'worker generation replacement retained old DOM evidence');
        const replacement = capture(); setCss(replacement);
        check(replacement.read().occurrences === 4, 'replacement generation could not acquire a fresh session');
    }, { pool: true });
    return { cases: passed };
};
