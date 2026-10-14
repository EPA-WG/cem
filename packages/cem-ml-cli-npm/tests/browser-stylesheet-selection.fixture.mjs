import { createBrowserHtmlBaseHost } from '../dist/browser.js';
import { retainCemDocument, disposeCemDocument } from '@epa-wg/cem-ml/wasm';
import titled from '../../cem_ml/tests/fixtures/browser-stylesheet-selection.xhtml?raw';
import inactive from '../../cem_ml/tests/fixtures/browser-stylesheet-inactive.xhtml?raw';
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let rejected = false; try { work(); } catch { rejected = true; } check(rejected, message); };

globalThis.runStylesheetSelectionFixture = async (version) => {
    const cases = [];
    const run = async (name, test, kind = 'selection') => {
        const frame = document.createElement('iframe'); frame.src = `/docs/browser-stylesheet-${kind}.html`;
        const loaded = new Promise(resolve => { frame.onload = resolve; }); document.body.append(frame);
        const text = kind === 'selection' ? titled : kind === 'inactive' ? inactive : kind === 'missing' ? inactive.replace(' print;', ' screen;').replace('unloaded.css', 'missing.css') : inactive.replace('print;', 'supports(display: cem-unimplemented);');
        const native = retainCemDocument(new TextEncoder().encode(text.trim()), 'application/xml', 'studio://fixture/stylesheet-selection.xhtml');
        let html, css;
        try {
            if (kind === 'inactive') {
                for (let attempt = 0; attempt < 400; attempt++) {
                    if (frame.contentDocument.URL.endsWith('/docs/browser-stylesheet-inactive.html') && frame.contentDocument.readyState !== 'loading') break;
                    if (attempt === 399) throw new Error('Inactive import DOM did not become ready');
                    await new Promise(resolve => setTimeout(resolve, 5));
                }
            } else await loaded;
            const doc = frame.contentDocument, view = frame.contentWindow;
            html = await createBrowserHtmlBaseHost(doc, { profile: 'chromium-148', browser: { engine: 'chromium', version }, contextOwnership: 'exclusive' });
            html.capture(native, { stamp: 'allow:1', decide: () => true });
            const sources = (kind === 'selection' ? [['night.css', 'p{color:rgb(40,30,20)}']]
                : [[kind === 'missing' ? 'missing.css' : 'unloaded.css', '@import "grand.css"; p{color:blue}'], ['grand.css', 'p{border-style:solid}']])
                .map(([path, text]) => ({ requestedUrl: new URL(path, doc.baseURI).href, finalUrl: new URL(path, doc.baseURI).href,
                    revision: 'fixture:1', contentType: 'text/css', text }));
            const capture = (titledSets) => {
                const result = html.captureStylesheets(sources, { ownership: 'exclusive', titledSets }); css = result; return result;
            };
            await test({ doc, view, capture, loaded }); cases.push(name);
        } catch (error) { throw new Error(`${name}: ${String(error)}`, { cause: error }); }
        finally { css?.dispose(); html?.dispose(); disposeCemDocument(native); frame.remove(); }
    };
    await run('explicit-inactive-preferred-and-alternate-sets', ({ capture, doc, view }) => {
        const css = capture('disabled');
        check(css.read().occurrences === 3, 'titled root coverage');
        const style = view.getComputedStyle(doc.querySelector('p'));
        check(style.fontStyle === 'italic' && style.color !== 'rgb(40, 30, 20)' && style.color !== 'rgb(20, 30, 40)', 'persistent style or titled inactivity disagrees');
        check([...doc.styleSheets].filter(sheet => sheet.title).every(sheet => sheet.disabled), 'titled set flags not disabled');
    });
    await run('same-value-titled-disable-invalidates', ({ capture }) => {
        const css = capture('disabled'); css.disableTitled();
        rejects(() => css.read(), 'same-value selection retained history'); css.capture();
    });
    await run('implicit-titled-selection-rejected', ({ capture }) => rejects(() => capture(), 'title alone inferred browser selection'));
    await run('active-titled-selection-rejected', ({ capture }) => rejects(() => capture('Night'), 'unobservable active set admitted'));
    await run('disabled-false-does-not-prove-alternate-activation', ({ capture, doc, view }) => {
        const sheet = doc.querySelector('link').sheet;
        check(sheet.disabled === false, 'Chromium alternate flag profile changed');
        check(view.getComputedStyle(doc.querySelector('p')).color !== 'rgb(40, 30, 20)', 'alternate unexpectedly active');
        rejects(() => capture(), 'disabled=false was mistaken for actual alternate activation');
    });
    await run('unowned-titled-enable-rejected', ({ capture, doc }) => {
        const css = capture('disabled'); doc.querySelector('link').sheet.disabled = false;
        rejects(() => css.read(), 'unowned alternate enable accepted');
    });
    await run('inactive-unloaded-import-and-descendant', async ({ capture, doc, loaded }) => {
        const rule = doc.querySelector('style').sheet.cssRules[0];
        check(rule.styleSheet === null, 'pending inactive import already loaded');
        const css = capture();
        check(css.read().occurrences === 3, 'unloaded import descendants not recorded');
        await fetch('/release-inactive-css'); await loaded;
        check(rule.styleSheet !== null, 'released response did not install the import');
        rejects(() => css.read(), 'import load revived old inactive evidence');
        check(css.capture().occurrences === 3, 'fresh loaded inactive import capture failed');
    }, 'inactive');
    await run('active-missing-import-rejected', ({ capture }) => rejects(() => capture(), 'active missing stylesheet accepted'), 'missing');
    await run('cssom-omitted-import-rejected', ({ capture, doc }) => {
        check(doc.querySelector('style').sheet.cssRules[0].type !== 3, 'Chromium unsupported-import profile changed');
        rejects(() => capture(), 'CSSOM-omitted import was certified as unloaded');
    }, 'omitted');
    return { cases };
};
