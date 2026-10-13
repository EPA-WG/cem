import { createBrowserHtmlBaseHost } from '../dist/browser.js';
import { retainCemDocument, disposeCemDocument } from '@epa-wg/cem-ml/wasm';
import source from '../../cem_ml/tests/fixtures/html-base-host.xhtml?raw';
import inventorySource from '../../cem_ml/tests/fixtures/html-execution-inventory.xhtml?raw';

const markup = source.trim();
const check = (condition, message) => { if (!condition) throw new Error(message); };
const rejects = (callback, message) => {
    let rejected = false;
    try { callback(); } catch { rejected = true; }
    check(rejected, message);
};

globalThis.runHtmlBaseHostFixture = async (version) => {
    const passed = [];
    let inventoryObservation;
    const options = { profile: 'chromium-148', browser: { engine: 'chromium', version }, contextOwnership: 'exclusive' };
    const imported = [];
    const retain = (text = markup) => {
        const handle = retainCemDocument(new TextEncoder().encode(text), 'application/xml', 'studio://fixture/host.xhtml');
        imported.push(handle);
        return handle;
    };
    const run = async (name, test, overrides = {}, path = '/a/html-base-host.html') => {
        const frame = document.createElement('iframe');
        frame.src = path;
        const ready = new Promise((resolve, reject) => { frame.onload = resolve; frame.onerror = reject; });
        document.body.append(frame);
        let host;
        try {
            await ready;
            const doc = frame.contentDocument;
            host = await createBrowserHtmlBaseHost(doc, { ...options, ...overrides });
            await test(host, doc, frame.contentWindow);
            passed.push(name);
        } catch (error) {
            throw new Error(`${name}: ${String(error)}`, { cause: error });
        } finally {
            host?.dispose();
            frame.remove();
            while (imported.length) disposeCemDocument(imported.pop());
        }
    };
    const allow = { stamp: 'allow:1', decide: () => true };
    await run('native-ownership-and-history-refreeze', (host, doc, view) => {
        let calls = 0;
        const policy = { stamp: 'allow:1', decide: (_candidate, fallback, stamp) => {
            check(fallback === doc.URL && stamp === 'allow:1', 'wrong freeze policy context');
            calls += 1;
            return true;
        } };
        const native = retain();
        const initial = host.capture(native, policy);
        check(initial === new URL('/a/child/', doc.URL).href && host.read() === initial, 'initial native confirmation failed');
        host.changeContext(() => { view.history.pushState(null, '', '/b/page#new'); });
        check(doc.baseURI === initial, 'history unexpectedly refroze active base');
        rejects(() => host.read(), 'history retained stale evidence');
        const next = host.capture(native, policy);
        check(next === new URL('/b/child/', doc.URL).href && next !== initial, 'fresh same-value href did not use new fallback');
        host.changeContext(() => {
            view.history.pushState(null, '', '/c/transient');
            view.history.replaceState(null, '', '/b/page#new');
        });
        rejects(() => host.read(), 'history A-B-A retained stale evidence');
        check(host.capture(native, policy) === next && calls === 3, 'same-value refreeze policy count');
        check(disposeCemDocument(native), 'native source handle was not retained');
        check(host.read() === next, 'host lost its retained native source owner');
    });
    await run('policy-removal-and-unsupported-fallback', (host, doc, view) => {
        host.capture(retain(), allow);
        const metaMarkup = '<meta http-equiv="Content-Security-Policy" content="base-uri &apos;none&apos;"/>';
        host.changeContext(() => {
            const meta = doc.createElement('meta');
            meta.httpEquiv = 'Content-Security-Policy';
            meta.content = "base-uri 'none'";
            doc.head.append(meta);
            doc.querySelector('base').setAttribute('href', 'other/');
        });
        rejects(() => host.read(), 'changed policy retained evidence');
        let snapshot = markup.replace('href="child/"', 'href="other/"').replace('</head>', `${metaMarkup}</head>`);
        const deny = { stamp: 'deny:2', decide: () => false };
        check(host.capture(retain(snapshot), deny) === doc.URL, 'denied base failed to use fallback');
        host.changeContext(() => { doc.querySelector('base').remove(); });
        snapshot = snapshot.replace('<base href="other/"/>', '');
        check(host.capture(retain(snapshot), deny) === doc.URL, 'next selected base ignored policy');
        host.changeContext(() => { doc.querySelector('base').remove(); });
        snapshot = snapshot.replace('<base href="https://later.test/"/>', '');
        check(host.capture(retain(snapshot), deny) === doc.URL, 'no-base fallback failed');
        host.changeContext(() => { view.history.pushState(null, '', '/c/final#last'); });
        rejects(() => host.capture(retain(snapshot), deny), 'unsupported stale Chromium fallback was admitted');
        rejects(() => host.read(), 'failed confirmation left evidence');
    });
    await run('late-policy-same-value-freeze-rejected', (host, doc) => {
        const original = host.capture(retain(), allow);
        host.changeContext(() => {
            const meta = doc.createElement('meta');
            meta.httpEquiv = 'Content-Security-Policy';
            meta.content = "base-uri 'none'";
            doc.head.append(meta);
        });
        const snapshot = markup.replace('</head>', '<meta http-equiv="Content-Security-Policy" content="base-uri &apos;none&apos;"/></head>');
        rejects(() => host.capture(retain(snapshot), { stamp: 'deny:2', decide: () => false }), 'Chromium stale policy on same-value assignment admitted');
        check(doc.baseURI === original, 'measured Chromium same-value/CSP divergence changed');
        rejects(() => host.read(), 'unsupported policy observation left evidence');
    });
    await run('inert-template-exclusion', (host, doc) => {
        const url = host.capture(retain(), allow);
        const base = doc.createElement('base');
        base.href = 'https://inert.test/';
        doc.querySelector('template').content.append(base);
        check(host.read() === url, 'inert template content changed document selection');
    });
    for (const [name, mutate] of [
        ['body-attribute-away-and-back', (doc) => { const link = doc.querySelector('a'); link.setAttribute('href', 'other'); link.setAttribute('href', 'next'); }],
        ['same-value-unowned-freeze', (doc) => { doc.querySelector('base').setAttribute('href', 'child/'); }],
        ['same-markup-node-replacement', (doc) => { const link = doc.querySelector('a'); link.replaceWith(link.cloneNode(true)); }],
    ]) await run(name, (host, doc) => {
        const native = retain();
        host.capture(native, allow);
        mutate(doc);
        rejects(() => host.read(), `${name} retained evidence before observer delivery`);
        rejects(() => host.capture(native, allow), `${name} failed to revoke session`);
    });
    await run('observer-delivery-revokes', async (host, doc) => {
        host.capture(retain(), allow);
        doc.querySelector('a').firstChild.data = 'Changed';
        await Promise.resolve();
        rejects(() => host.read(), 'observer delivery left evidence');
    });
    await run('topology-mismatch-before-policy', (host) => {
        let calls = 0;
        rejects(() => host.capture(retain(markup.replace('<a href="next">Next</a>', '')), { stamp: 'allow:1', decide: () => { calls += 1; return true; } }), 'missing native body slot accepted');
        check(calls === 0, 'policy ran before complete topology validation');
    });
    await run('bounded-dom-before-policy', (host) => {
        let calls = 0;
        rejects(() => host.capture(retain(), { stamp: 'allow:1', decide: () => { calls += 1; return true; } }), 'topology budget ignored');
        check(calls === 0, 'policy ran on truncated topology');
    }, { maxNodes: 2 });
    await run('unowned-history-revokes', (host, _doc, view) => {
        host.capture(retain(), allow);
        view.history.pushState(null, '', '/unowned');
        rejects(() => host.read(), 'unowned URL change left evidence');
    });
    await run('policy-reentry-revokes', (host) => {
        rejects(() => host.capture(retain(), { stamp: 'allow:1', decide: () => {
            rejects(() => host.read(), 'reentrant read succeeded');
            return true; // Swallowing the callback error must not revive admission.
        } }), 'reentrant policy was admitted');
        rejects(() => host.capture(retain(), allow), 'reentrant policy failed to revoke');
    });
    await run('policy-mutation-revokes', (host, doc) => {
        rejects(() => host.capture(retain(), { stamp: 'allow:1', decide: () => { doc.body.setAttribute('title', 'changed'); return true; } }), 'mutating policy was admitted');
    });
    await run('async-change-revokes', (host) => {
        host.capture(retain(), allow);
        rejects(() => host.changeContext(() => Promise.resolve()), 'asynchronous change admitted');
        rejects(() => host.read(), 'asynchronous change retained evidence');
    });
    await run('unknown-browser-rejected', (host) => {
        rejects(() => host.capture(retain(), allow), 'unknown browser admitted');
    }, { browser: { engine: 'chromium', version: 'unknown' } });
    for (const [name, href] of [['invalid-href-profile', 'http://['], ['empty-href-profile', '']]) {
        await run(name, (host, doc, view) => {
            host.changeContext(() => {
                view.history.replaceState(null, '', '/a/html-base-host.html#fragment');
                doc.querySelector('base').setAttribute('href', href);
            });
            const actual = host.capture(retain(markup.replace('href="child/"', `href="${href}"`)), allow);
            check(actual === (href ? 'about:blank' : doc.URL) && host.read() === actual, 'explicit Chromium compatibility lost');
        });
    }
    await run('reference-and-inline-css-candidates', (host, doc, view) => {
        const base = host.capture(retain(inventorySource.trim()), allow);
        check(host.read() === base, 'inventory fixture lost live native correspondence');
        const background = (element) => {
            const value = view.getComputedStyle(element).backgroundImage;
            check(value.startsWith('url("') && value.endsWith('")'), 'expected one computed CSS URL');
            return value.slice(5, -2); // Scalar browser serialization, never source CSS parsing.
        };
        inventoryObservation = { document_url: doc.URL, base_uri: base, engine: 'chromium', version,
            image_url: doc.querySelector('img').src, navigation_url: doc.querySelector('a').href,
            stylesheet_url: background(doc.querySelector('p')), inline_url: background(doc.querySelector('span')) };
    }, {}, '/docs/html-execution-inventory.html');
    return { chromiumVersion: version, cases: passed, inventoryObservation };
};
