import { createBrowserHtmlBaseHost, createBrowserWorkerPool } from '../dist/browser.js';
import { retainCemDocument, disposeCemDocument } from '@epa-wg/cem-ml/wasm';
import source from '../../cem_ml/tests/fixtures/browser-stylesheet-active.xhtml?raw';
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let failed = false; try { work(); } catch { failed = true; } check(failed, message); };
const rejectsAsync = async (work, message) => { let failed = false; try { await work(); } catch { failed = true; } check(failed, message); };
const mode = value => fetch(`/selection-mode?mode=${value}`);

globalThis.runActiveStylesheetFixture = async version => {
    const cases = [];
    const run = async (name, work) => {
        await mode('normal');
        const frame = document.createElement('iframe'); frame.src = '/docs/browser-stylesheet-active.html';
        const loaded = new Promise(resolve => { frame.onload = resolve; }); document.body.append(frame);
        let html, css, pool;
        const native = retainCemDocument(new TextEncoder().encode(source.trim()), 'application/xml', 'studio://fixture/active.xhtml');
        try {
            await loaded;
            const doc = frame.contentDocument, view = frame.contentWindow;
            html = await createBrowserHtmlBaseHost(doc, { profile: 'chromium-148', browser: { engine: 'chromium', version }, contextOwnership: 'exclusive' });
            const policy = { stamp: 'allow:1', decide: () => true };
            html.capture(native, policy);
            const responses = [['day.css', 'p{color:rgb(20,30,40)}'], ['night.css', 'p{color:rgb(40,30,20)}'],
                ['night-extra.css', '@import "grand.css" screen; p{background-color:rgb(4,5,6)}'], ['grand.css', 'p{border-style:solid}']]
                .map(([path, text]) => ({ requestedUrl: new URL(path, doc.baseURI).href, finalUrl: new URL(path, doc.baseURI).href, revision: 'fixture:1', contentType: 'text/css', text }));
            const select = async (title = 'Night', options = {}) => {
                const result = await html.selectStylesheets(native, policy, responses, title, { ownership: 'exclusive', ...options }); css = result; return result;
            };
            const makePool = async () => { pool = await createBrowserWorkerPool({ workerCount: 1, hardCancelGraceMs: 10 }); return pool; };
            await work({ doc, view, html, native, policy, responses, select, makePool });
            cases.push(name);
        } catch (error) { throw new Error(`${name}: ${String(error)}`, { cause: error }); }
        finally { css?.dispose(); html?.dispose(); await pool?.close(); disposeCemDocument(native); frame.remove(); await mode('normal'); }
    };
    await run('active-alternate-multiple-links-and-import-apply', async ({ select, doc, view }) => {
        const css = await select(); check(css.read().occurrences === 6, 'complete source forest');
        const style = view.getComputedStyle(doc.querySelector('p'));
        check(style.color === 'rgb(40, 30, 20)' && style.backgroundColor === 'rgb(4, 5, 6)' && style.borderStyle === 'solid' && style.fontStyle === 'italic', 'selected links/import/persistent style did not apply');
        check(doc.querySelector('style[title]').sheet.disabled, 'unselected inline title enabled');
    });
    await run('same-title-and-a-b-a-never-revive-evidence', async ({ select, doc, view }) => {
        const a = await select(); const same = await select(); rejects(() => a.read(), 'same title revived evidence');
        const day = await select('Day'); rejects(() => same.read(), 'old Night evidence remains');
        check(view.getComputedStyle(doc.querySelector('p')).color === 'rgb(20, 30, 40)', 'Day did not apply');
        const again = await select(); rejects(() => day.read(), 'A-B-A revived Day'); check(again.read().occurrences === 6, 'recaptured A');
        const none = await select(''); rejects(() => again.read(), 'disable retained active evidence');
        check(none.read().occurrences === 6 && [...doc.styleSheets].filter(s => s.title).every(s => s.disabled), 'empty title did not disable all');
    });
    await run('pending-reads-and-competing-transitions-reject', async ({ select, html }) => {
        await mode('slow'); const pending = select();
        rejects(() => html.read(), 'pending HTML read accepted');
        rejects(() => html.captureStylesheets([], { ownership: 'exclusive' }), 'pending CSS capture accepted');
        await rejectsAsync(() => select('Day'), 'competing transition accepted');
        await mode('normal'); check((await pending).read().occurrences === 6, 'original transition did not complete');
    });
    await run('synthetic-load-cannot-complete-transition', async ({ select, doc, html }) => {
        await mode('slow'); const pending = select();
        doc.querySelector('link[href="night.css"]').dispatchEvent(new Event('load'));
        await new Promise(resolve => setTimeout(resolve, 30));
        rejects(() => html.read(), 'synthetic load certified activation');
        await mode('normal'); await pending;
    });
    await run('load-error-revokes-html-and-css', async ({ select, html }) => {
        await mode('error'); await rejectsAsync(() => select(), 'failed load accepted'); rejects(() => html.read(), 'load error retained HTML');
    });
    await run('loaded-source-mismatch-rejects', async ({ select }) => {
        await mode('changed'); await rejectsAsync(() => select(), 'changed network source accepted');
    });
    await run('timeout-revokes-pending-transition', async ({ select, html }) => {
        await mode('slow'); await rejectsAsync(() => select('Night', { timeoutMs: 25 }), 'timeout accepted'); rejects(() => html.read(), 'timeout retained HTML');
    });
    await run('abort-during-load-revokes', async ({ select, html }) => {
        await mode('slow'); const abort = new AbortController(); const pending = select('Night', { signal: abort.signal });
        abort.abort(); await rejectsAsync(() => pending, 'aborted selection accepted'); rejects(() => html.read(), 'abort retained HTML');
    });
    await run('dom-mutation-during-load-revokes', async ({ select, doc }) => {
        await mode('slow'); const pending = select(); doc.querySelector('p').textContent = 'Changed';
        await rejectsAsync(() => pending, 'DOM drift accepted');
    });
    await run('policy-change-at-recapture-rejects', async ({ select, policy }) => {
        await mode('slow'); const pending = select(); policy.stamp = 'allow:2'; await mode('normal');
        await rejectsAsync(() => pending, 'policy drift accepted');
    });
    await run('foreign-native-document-recapture-rejects', async ({ html, policy, responses }) => {
        const foreign = retainCemDocument(new TextEncoder().encode(source.trim()), 'application/xml', 'studio://fixture/foreign.xhtml');
        try { await rejectsAsync(() => html.selectStylesheets(foreign, policy, responses, 'Night', { ownership: 'exclusive' }), 'foreign native source accepted'); }
        finally { disposeCemDocument(foreign); }
    });
    await run('selected-inline-title-rejects', async ({ select }) => { await rejectsAsync(() => select('Inline'), 'inline active title accepted'); });
    await run('unknown-title-rejects', async ({ select }) => { await rejectsAsync(() => select('night'), 'case-insensitive or missing title accepted'); });
    await run('cssom-change-invalidates-activation-receipt', async ({ select, doc }) => {
        const css = await select(); css.change(() => { doc.querySelector('link[href="night.css"]').sheet.disabled = false; });
        rejects(() => css.read(), 'same-value CSS write retained evidence'); rejects(() => css.capture(), 'CSS write reused activation receipt');
    });
    await run('post-capture-unowned-cssom-write-rejects', async ({ select, doc }) => {
        const css = await select(); doc.querySelector('link[href="night.css"]').sheet.disabled = true;
        rejects(() => css.read(), 'unowned enable/disable accepted');
    });
    await run('dispose-during-load-rejects', async ({ select, html }) => {
        await mode('slow'); const pending = select(); html.dispose(); await rejectsAsync(() => pending, 'disposed HTML published selection');
    });
    await run('worker-pool-close-cancels-pending-selection', async ({ html, native, policy, responses, makePool }) => {
        const pool = await makePool(); await mode('slow');
        const pending = pool.selectStylesheets(html, native, policy, responses, 'Night', { ownership: 'exclusive' });
        await pool.close(); await rejectsAsync(() => pending, 'closed pool published selection');
    });
    await run('worker-replacement-cancels-pending-selection', async ({ html, native, policy, responses, makePool }) => {
        const pool = await makePool(); await mode('slow');
        const pending = pool.selectStylesheets(html, native, policy, responses, 'Night', { ownership: 'exclusive' });
        const rejected = rejectsAsync(() => pending, 'replaced worker published selection');
        const bytes = text => [...new TextEncoder().encode(text)];
        const operation = pool.run({ kind: 'query',
            data: { uri: 'memory:catalog.xml', fromFormat: 'xml', bytes: bytes(`<catalog>${'<book/>'.repeat(600)}</catalog>`),
                identity: { contentType: 'application/xml', schema: 'https://cem.dev/ns/data/xml/1' } },
            query: { uri: 'memory:query.xpath', bytes: bytes('count(/catalog/book)'),
                identity: { contentType: 'application/vnd.cem.xpath', schema: 'https://cem.dev/ns/query/xpath/1' } },
        });
        await new Promise(resolve => operation.subscribe(event => { if (event.kind === 'dispatch' && event.stage.label === 'execute') resolve(); }));
        await operation.cancel('pending stylesheet selection fixture');
        try { await operation; } catch { /* cancellation is expected */ }
        check(pool.workers[0].generation > 1, 'busy worker was not replaced');
        await rejected; rejects(() => html.read(), 'worker replacement retained pending HTML capture');
    });
    await run('worker-pool-close-revokes-completed-selection', async ({ html, native, policy, responses, makePool }) => {
        const pool = await makePool(); const css = await pool.selectStylesheets(html, native, policy, responses, 'Night', { ownership: 'exclusive' });
        check(css.read().occurrences === 6, 'pool selection failed'); await pool.close(); rejects(() => css.read(), 'closed pool retained selection');
    });
    return { cases };
};
