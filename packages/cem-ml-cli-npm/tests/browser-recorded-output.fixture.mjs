import { BrowserRecordedOutputHost, captureBrowserRecordedOutput, captureBrowserRecordedOutputAsync, createBrowserWorkerPool, createBrowserHtmlBaseHost } from '../dist/browser.js';
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
            // Initial iframe layout can queue resize after load. Capture a settled
            // viewport; subsequent resize must still revoke the production host.
            await new Promise(resolve => view.requestAnimationFrame(() => view.requestAnimationFrame(resolve)));
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
            const captureAsync = async (extra = {}, installer = install, configure = () => undefined) => {
                const session = native(), options = { ownership: 'exclusive', policy, ...extra };
                configure(session);
                const host = await (pool ? pool.captureRecordedOutputAsync(doc, session, installer, options) : captureBrowserRecordedOutputAsync(doc, session, installer, options));
                hosts.push(host); return host;
            };
            await test({ doc, view, captureAsync, capture, policy, native, install, hosts, pool, request });
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

    const addHead = (request, markup) => {
        request.sources[0].source = request.sources[0].source.replace('{result-element @name=base', markup.replaceAll('} {result-attribute', '}\n{result-attribute') + ' {result-element @name=base');
    };
    const addLink = (request, path, extra = '') => {
        const url = new URL(path, location.href).href;
        addHead(request, `{link @rel=stylesheet @href='${url}' ${extra ? '@title=day' : ''}}`);
        return url;
    };
    const acquired = (request, path, text, final = path) => {
        request.stylesheets.push({ requestedUrl: new URL(path, location.href).href, finalUrl: new URL(final, location.href).href,
            revision: '1', source: text, contentType: 'text/css' });
    };
    await run('external-acquired-imports', async ({ captureAsync, request, doc, view }) => {
        addLink(request, '/styles/theme.css');
        acquired(request, '/styles/theme.css', '@import "child.css" supports(display: grid) screen and (min-width: 200px); p{background-color:rgb(4,5,6)}');
        acquired(request, '/styles/child.css', 'p{border-style:solid}');
        const host = await captureAsync();
        check(host.read().blockers.length > 0 && view.getComputedStyle(doc.querySelector('body > p')).borderStyle === 'solid', 'acquired imports did not apply');
        doc.querySelector('link').sheet.cssRules[0].styleSheet.insertRule('p{color:red}', 0);
        rejects(() => host.read(), 'imported CSSOM mutation retained evidence');
    });
    for (const media of ['screen', 'print']) {
        await run(`outer-media-${media}`, async ({ captureAsync, request, doc, view }) => {
            addHead(request, `{result-element @name=style @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=media | ${media}} \`\`\`body > p{font-style:italic}\`\`\`}`);
            const host = await captureAsync();
            check((view.getComputedStyle(doc.querySelector('body > p')).fontStyle === 'italic') === (media === 'screen'), 'outer media application mismatch');
            host.read();
            doc.querySelector('style').sheet.media.mediaText = media === 'screen' ? 'print' : 'screen';
            rejects(() => host.read(), 'outer media change retained evidence');
        });
    }
    await run('titled-inline-disabled', async ({ captureAsync, request, doc, view }) => {
        addHead(request, "{result-element @name=style @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=title | day} ```body > p{font-style:italic}```}");
        const host = await captureAsync({ stylesheetTitle: '' });
        check(doc.querySelector('style').sheet.disabled && view.getComputedStyle(doc.querySelector('body > p')).fontStyle !== 'italic', 'inline title not disabled');
        host.read();
    });
    for (const title of ['day', '', 'alternate']) {
        await run(`external-title-${title || 'disabled'}`, async ({ captureAsync, request, doc, view }) => {
            addLink(request, '/styles/day.css', '{result-attribute @name=title | day}');
            if (title === 'alternate') request.sources[0].source = request.sources[0].source.replace('@rel=stylesheet', "@rel='alternate stylesheet'");
            acquired(request, '/styles/day.css', 'p{color:rgb(20,30,40)}');
            const host = await captureAsync({ stylesheetTitle: title === 'alternate' ? 'day' : title });
            host.read();
            check((view.getComputedStyle(doc.querySelector('body > p')).color === 'rgb(20, 30, 40)') === !!title, 'native title selection application mismatch');
            host.change(() => { doc.querySelector('link').sheet.disabled = true; });
            rejects(() => host.read(), 'controlled title change retained evidence');
            const fresh = await captureAsync({ stylesheetTitle: title === 'alternate' ? 'day' : title });
            fresh.read();
        });
    }

    await run('outer-media-environment-revocation', async ({ captureAsync, request, view }) => {
        addHead(request, "{result-element @name=style @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=media | screen} ```p{font-style:italic}```}");
        const original = view.matchMedia.bind(view), queries = [];
        view.matchMedia = query => { const result = original(query); queries.push(result); return result; };
        const host = await captureAsync();
        check(queries.length > 0, 'media listener not installed');
        queries[0].dispatchEvent(new Event('change'));
        rejects(() => host.read(), 'media environment event retained evidence');
    });
    await run('active-inline-title-rejected', async ({ captureAsync, request }) => {
        addHead(request, "{result-element @name=style @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=title | day} ```p{color:red}```}");
        let failed = false;
        try { await captureAsync({ stylesheetTitle: 'day' }); } catch { failed = true; }
        check(failed, 'unsupported inline title activation accepted');
    });
    for (const kind of ['missing-source', 'wrong-source', 'load-error', 'timeout', 'abort', 'pool-close']) {
        await run(`external-${kind}`, async ({ captureAsync, request, pool, doc }) => {
            const path = kind === 'load-error' ? '/styles/recorded-error.css' :
                ['timeout', 'abort', 'pool-close'].includes(kind) ? '/styles/recorded-pending.css' : '/styles/day.css';
            addLink(request, path);
            if (kind !== 'missing-source') acquired(request, path, kind === 'wrong-source' ? 'p{color:red}' : 'p{color:rgb(20,30,40)}');
            const controller = new AbortController();
            let failed = false;
            const promise = captureAsync({ signal: controller.signal, timeoutMs: kind === 'timeout' ? 20 : 5000 }).catch(() => { failed = true; });
            if (kind === 'abort') controller.abort();
            if (kind === 'pool-close') await pool.close();
            await promise;
            check(failed, 'invalid/pending external stylesheet published evidence');
            check(doc.URL.includes('browser-recorded-output'), 'fixture navigated document');
        }, kind === 'pool-close');
    }

    const addImage = (request, markup) => {
        const marker = "{result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Outside}";
        request.sources[0].source = request.sources[0].source.replace(marker, markup + ' ' + marker);
    };
    for (const mode of ['src', 'density', 'density-only', 'width', 'picture']) {
        await run(`image-selection-${mode}`, async ({ captureAsync, request, doc }) => {
            const img = mode === 'src' ? "{img @src='/recorded-images/a.png' @alt=Example}" :
                mode === 'density' ? "{img @srcset='/recorded-images/a.png 1x, /recorded-images/b.png 2x' @alt=Example}" :
                mode === 'density-only' ? "{img @srcset='/recorded-images/b.png 2x' @alt=Example}" :
                mode === 'width' ? "{img @srcset='/recorded-images/a.png 64w, /recorded-images/b.png 128w' @sizes=64px @alt=Example}" :
                "{picture | {source @media=screen @type='image/png' @srcset='/recorded-images/b.png 1x'} {img @src='/recorded-images/a.png' @alt=Example}}";
            addImage(request, img);
            const host = await captureAsync();
            const summary = host.read(), selected = doc.querySelector('img').currentSrc;
            check(summary.imageAcquisitions === 1 && summary.blockers.includes('image-browser-response-identity-required'), 'image acquisition not retained or response identity overclaimed');
            check(selected.endsWith(['picture', 'density-only'].includes(mode) ? '/b.png' : '/a.png'), 'unexpected responsive image source');
            await new Promise(resolve => setTimeout(resolve, 20));
            host.read();
            doc.querySelector('img').setAttribute('srcset', '/recorded-images/b.png 1x');
            rejects(() => host.read(), 'image source mutation retained evidence');
        });
    }
    for (const mode of ['missing', 'acquisition-error', 'oversized', 'redirect', 'timeout', 'abort', 'mutation', 'pool-close']) {
        await run(`image-acquisition-${mode}`, async ({ captureAsync, request, doc, pool }) => {
            const path = ['missing', 'acquisition-error', 'oversized', 'redirect'].includes(mode) ? mode : 'pending';
            addImage(request, `{img @src='/recorded-images/${path}.png' @alt=Example}`);
            const controller = new AbortController();
            let failed = false;
            const pending = captureAsync({ signal: controller.signal, timeoutMs: mode === 'timeout' ? 100 : 5000 }).catch(() => { failed = true; });
            if (mode === 'abort') setTimeout(() => controller.abort(), 25);
            if (mode === 'mutation') setTimeout(() => doc.querySelector('img')?.setAttribute('sizes', '20px'), 25);
            if (mode === 'pool-close') setTimeout(() => pool.close(), 25);
            await pending;
            check(failed, 'failed/stale image acquisition published preparation');
        }, mode === 'pool-close');
    }

    for (const [mode, markup] of [
        ['density', "{img @srcset='/recorded-images/a.png 1x, /recorded-images/./a.png 2x' @alt=Example}"],
        ['fallback', "{img @src='/recorded-images/a.png' @srcset='/recorded-images/a.png 2x' @alt=Example}"],
        ['picture', "{picture | {source @media=screen @srcset='/recorded-images/a.png'} {img @src='/recorded-images/a.png' @alt=Example}}"],
    ]) {
        await run(`image-ambiguous-${mode}`, async ({ captureAsync, request }) => {
            addImage(request, markup);
            let error;
            try { await captureAsync(); } catch (value) { error = value; }
            check(error !== undefined, 'duplicate URL published an image preparation');
        });
    }
    await run('image-changed-response-stays-unbound', async ({ captureAsync, request }) => {
        addImage(request, "{img @src='/recorded-images/changed.png' @alt=Example}");
        const host = await captureAsync();
        check(host.read().blockers.includes('image-browser-response-identity-required'), 'changed response acquired historical identity');
    });

    await run('image-media-revocation', async ({ captureAsync, request, view }) => {
        addImage(request, "{picture | {source @media=screen @srcset='/recorded-images/b.png'} {img @src='/recorded-images/a.png' @alt=Example}}");
        const original = view.matchMedia.bind(view), queries = [];
        view.matchMedia = value => { const query = original(value); queries.push(query); return query; };
        const host = await captureAsync();
        const media = queries.find(query => query.media === 'screen');
        check(media, 'picture media observer missing');
        media.dispatchEvent(new Event('change'));
        rejects(() => host.read(), 'picture media transition retained evidence');
    });
    await run('image-synchronous-capture-rejected', ({ capture, request }) => {
        addImage(request, "{img @src='/recorded-images/a.png' @alt=Example}");
        rejects(() => capture(), 'synchronous capture claimed image acquisition');
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
    await run('image-delivery-no-controller', async ({ captureAsync }) => {
        let failed = false;
        try { await captureAsync({ imageDelivery: 'service-worker' }); } catch { failed = true; }
        check(failed, 'delivery transport admitted an uncontrolled document');
    });
    const registration = await navigator.serviceWorker.register('/image-worker.js', { type: 'module', scope: '/' });
    await navigator.serviceWorker.ready;
    if (!navigator.serviceWorker.controller) await new Promise(resolve => navigator.serviceWorker.addEventListener('controllerchange', resolve, { once: true }));
    try {
        for (const mode of ['single', 'shared', 'empty']) {
            await run(`image-delivery-${mode}`, async ({ captureAsync, request, doc, install }) => {
                const url = `/recorded-images/delivery.png?execution=${crypto.randomUUID()}`;
                if (mode !== 'empty') addImage(request, `{img @src='${url}' @alt=Example}` + (mode === 'shared' ? ` {img @src='${url}' @alt=Second}` : ''));
                let receipts = 0;
                const host = await captureAsync({ imageDelivery: 'service-worker' }, install, session => {
                    const accept = session.acceptImageDelivery.bind(session);
                    session.acceptImageDelivery = (url, type, bytes) => {
                        check(bytes.byteLength > 0 && type === 'image/png', 'original response bytes missing');
                        check([...doc.images].every(image => !image.complete), 'image was delivered before native receipt');
                        accept(url, type, bytes); receipts++;
                    };
                });
                const count = mode === 'empty' ? 0 : 1;
                check(receipts === count && host.read().imageDeliveries === count, 'native delivery join missing');
                check(host.read().imageAcquisitions === count, 'selected delivery body missing');
                if (count) check(host.read().blockers.includes('image-browser-response-identity-required'), 'cache-selection history overclaimed');
                if (mode === 'single') {
                    doc.defaultView.navigator.serviceWorker.dispatchEvent(new Event('controllerchange'));
                    rejects(() => host.read(), 'controller loss retained native delivery evidence');
                }
                host.dispose(); rejects(() => host.read(), 'disposed delivery retained native evidence');
            });
        }
        for (const mode of ['warm-cache', 'duplicate-density', 'duplicate-picture', 'shared-consumers']) {
            await run(`image-replay-${mode}`, async ({ captureAsync, request, doc, install }) => {
                const url = `/recorded-images/replay.png?execution=${crypto.randomUUID()}`;
                const warm = doc.createElement('img'); warm.src = url; doc.body.append(warm); await warm.decode(); warm.remove();
                const markup = mode === 'duplicate-density' ? `{img @srcset='${url} 1x, ${url} 2x' @alt=Example}` :
                    mode === 'duplicate-picture' ? `{picture | {source @media=screen @srcset='${url}'} {img @src='${url}' @alt=Example}}` :
                    `{img @src='${url}' @alt=Example}` + (mode === 'shared-consumers' ? ` {img @src='${url}' @alt=Second}` : '');
                addImage(request, markup);
                let receipts = 0;
                const configure = session => {
                    const accept = session.acceptImageDelivery.bind(session);
                    session.acceptImageDelivery = (alias, type, bytes) => {
                        check(alias.includes('/__cem_image_delivery/'), 'replay receipt used an authored URL');
                        check(new TextDecoder().decode(bytes.slice(-13)) === 'replayed-body', 'replay reused the old cached body');
                        accept(alias, type, bytes); receipts++;
                    };
                };
                const host = await captureAsync({ imageDelivery: 'service-worker-replay' }, install, configure);
                const selected = doc.querySelector('img').currentSrc;
                check(selected.includes('/__cem_image_delivery/'), 'selected consumer escaped replay namespace');
                const count = mode === 'shared-consumers' ? 2 : 1;
                check(receipts === count && host.read().imageDeliveries === count, 'replay receipts did not bind selected consumers');
                check(!host.read().blockers.includes('image-browser-response-identity-required'), 'bounded replay still lacks response binding');
                check(host.read().blockers.includes('atomic-host-transaction-required'), 'image replay granted transaction admission');
                host.dispose();
                const next = await captureAsync({ imageDelivery: 'service-worker-replay' }, install, configure);
                check(doc.querySelector('img').currentSrc !== selected, 'new execution reused old replay routes');
                let staleRejected = false;
                try { await fetch(selected); } catch { staleRejected = true; }
                check(staleRejected, 'stale replay route reached the network');
                next.read();
                doc.querySelector('img').setAttribute('src', selected);
                rejects(() => next.read(), 'stale route mutation retained replay evidence');
            });
        }
        await run('image-replay-foreign-source', async ({ captureAsync, request }) => {
            addImage(request, "{img @src='https://foreign.invalid/image.png' @alt=Example}");
            let failed = false;
            try { await captureAsync({ imageDelivery: 'service-worker-replay' }); } catch { failed = true; }
            check(failed, 'foreign replay source admitted');
        });
        await run('image-delivery-missing-receipt', async ({ captureAsync, request, doc, install }) => {
            addImage(request, `{img @src='/recorded-images/bypass.png?execution=${crypto.randomUUID()}' @alt=Example}`);
            let failed = false, diagnostic = '';
            try {
                await captureAsync({ imageDelivery: 'service-worker' }, install, session => {
                    const record = session.recordDeliveredImages.bind(session);
                    session.recordDeliveredImages = (...args) => {
                        try { return record(...args); } catch (error) { diagnostic = String(error); throw error; }
                    };
                });
            } catch { failed = true; }
            check(failed && diagnostic.includes('complete live image delivery receipts required'), 'bypassed response acquired native delivery evidence');
            check(doc.querySelector('img').naturalWidth > 0, 'bypass fixture did not deliver an image');
        });
        for (const mode of ['throw', 'async']) await run(`image-delivery-native-rejection-${mode}`, async ({ captureAsync, request, doc, install }) => {
            addImage(request, `{img @src='/recorded-images/delivery.png?execution=${crypto.randomUUID()}' @alt=Example}`);
            let received = false, failed = false;
            try {
                await captureAsync({ imageDelivery: 'service-worker' }, install, session => {
                    session.acceptImageDelivery = () => { received = true; if (mode === 'async') return Promise.resolve(); throw new Error('Receipt rejected'); };
                });
            } catch { failed = true; }
            check(received && failed && doc.querySelector('img').naturalWidth === 0, 'rejected receipt released image pixels');
        });
        for (const mode of ['error', 'redirect', 'oversized', 'abort', 'pool-close']) {
            await run(`image-delivery-${mode}`, async ({ captureAsync, request, pool }) => {
                const path = ['abort', 'pool-close'].includes(mode) ? 'pending' : mode;
                addImage(request, `{img @src='/recorded-images/delivery-${path}.png?execution=${crypto.randomUUID()}' @alt=Example}`);
                const controller = new AbortController();
                let failed = false;
                const pending = captureAsync({ imageDelivery: 'service-worker', signal: controller.signal }).catch(() => { failed = true; });
                if (mode === 'abort') setTimeout(() => controller.abort(), 50);
                if (mode === 'pool-close') setTimeout(() => pool.close(), 50);
                await pending; check(failed, 'failed delivery published preparation');
            }, mode === 'pool-close');
        }
    } finally { await registration.unregister(); }
    return { cases };
};
