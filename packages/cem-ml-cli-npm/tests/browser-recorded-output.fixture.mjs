import { BrowserRecordedOutputHost, captureBrowserRecordedOutput, captureBrowserRecordedOutputAsync, createBrowserWorkerPool, createBrowserHtmlBaseHost } from '../dist/browser.js';
import * as ql from '../../cem_ql/dist/wasm/cem_ql.js';
const source = "{style |```p{color:rgb(1,2,3)}```}\n{style @scope=widgets |```p{font-weight:700}```}\n{style |```p{background-color:rgb(4,5,6)}```}\n{result-element @name=html @namespace='http://www.w3.org/1999/xhtml' |\n {result-element @name=head @namespace='http://www.w3.org/1999/xhtml' |\n  {result-element @name=base @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=href | ../assets/}}}\n {result-element @name=body @namespace='http://www.w3.org/1999/xhtml' |\n  {result-element @name=cem-card @namespace='http://www.w3.org/1999/xhtml' |\n   {result-attribute @name=scope | widgets}\n   {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}\n   {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Card}\n   {result-element @name=cem-inner @namespace='http://www.w3.org/1999/xhtml' |\n    {result-element @name=template @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=data-cem-island | instance}}\n    {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Inner}}}\n  {result-element @name=p @namespace='http://www.w3.org/1999/xhtml' | Outside}}}";
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let failed = false; try { work(); } catch { failed = true; } check(failed, message); };
globalThis.runRecordedOutputFixture = async version => {
    await ql.default();
    const cases = [];
    const cloneReports = new Map();
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
    const paintRequest = (request, doc, defs, fill = 'url(#a) red', base = doc.URL) => {
        request.choices = [];
        request.sources[0].contentType = 'text/cem-ml';
        request.sources[0].source = `{result-element @name=html @namespace='http://www.w3.org/1999/xhtml' | {result-element @name=head @namespace='http://www.w3.org/1999/xhtml' | {result-element @name=base @namespace='http://www.w3.org/1999/xhtml' | {result-attribute @name=href | ${base}}}}{result-element @name=body @namespace='http://www.w3.org/1999/xhtml' | {svg @xmlns='http://www.w3.org/2000/svg' @width=16 @height=16 | {defs | ${defs}}{rect @width=16 @height=16 @fill='${fill}'}}}}`;
    };
    for (const mode of ['unopened', 'repeat', 'closed']) {
        await run(`svg-snapshot-ingress-${mode}`, ({ native }) => {
            const session = native();
            try {
                if (mode === 'repeat') session.beginSvgSnapshots();
                if (mode === 'closed') session.close();
                if (mode === 'unopened') rejects(() => session.addSvgSnapshot('https://example.test/a.svg', 'https://example.test/a.svg', 'image/svg+xml', '1', new Uint8Array()), 'unopened ingress accepted');
                else rejects(() => session.beginSvgSnapshots(), 'repeated/closed ingress accepted');
                rejects(() => session.html(), 'invalid ingress failed to close session');
            } finally { session.close(); session.free(); }
        });
    }
    for (const mode of ['sync', 'async', 'copy', 'abort', 'mutation', 'pool', 'missing', 'duplicate', 'mime', 'bounds', 'preabort', 'empty']) {
        await run(`svg-snapshot-${mode}`, async ({ request, doc, capture, captureAsync, pool, install }) => {
            paintRequest(request, doc, '', `url(${doc.URL}#p)`);
            const controller = new AbortController();
            const bytes = new TextEncoder().encode('<svg xmlns="http://www.w3.org/2000/svg"><linearGradient id="p"/></svg>');
            const response = { requestedUrl: doc.URL, finalUrl: doc.URL, revision: 'snapshot:1', contentType: 'image/svg+xml', bytes };
            const responses = [response];
            if (mode === 'missing') response.requestedUrl = response.finalUrl = new URL('/missing.svg', doc.URL).href;
            if (mode === 'duplicate') responses.push(response);
            if (mode === 'mime') response.contentType = 'text/html';
            if (mode === 'bounds') response.bytes = new Uint8Array(4 * 1024 * 1024);
            if (mode === 'empty') responses.length = 0;
            if (mode === 'preabort') controller.abort();
            const options = { svgSnapshots: { responses, signal: controller.signal } };
            if (['missing', 'duplicate', 'mime', 'bounds', 'preabort', 'empty'].includes(mode)) {
                rejects(() => capture(options), 'invalid SVG snapshot accepted');
                return;
            }
            const installer = html => {
                if (mode === 'copy') { bytes.fill(0); response.revision = 'changed'; responses.length = 0; }
                install(html);
            };
            const host = ['async', 'pool'].includes(mode) ? await captureAsync(options, installer) : capture(options, installer);
            const result = host.read();
            check(result.svgResourceBindings === 1, 'native SVG binding missing');
            check(result.blockers.includes('svg-resource-browser-response-identity-required')
                && result.blockers.includes('svg-resource-consumption-required'), 'snapshot claimed consumption');
            if (mode === 'abort') controller.abort();
            if (mode === 'mutation') { const rect = doc.querySelector('rect'); const fill = rect.getAttribute('fill'); rect.setAttribute('fill', 'red'); rect.setAttribute('fill', fill); }
            if (mode === 'pool') await pool.close();
            if (['abort', 'mutation', 'pool'].includes(mode)) {
                rejects(() => host.read(), 'revoked SVG acquisition survived');
                rejects(() => host.read(), 'revoked SVG acquisition revived');
            }
        }, mode === 'pool');
    }
    for (const mode of ['local', 'chain', 'cycle', 'duplicate', 'missing', 'wrong-kind', 'base', 'nonfragment']) {
        await run(`paint-candidate-${mode}`, ({ request, doc, view, capture }) => {
            let defs = '{linearGradient @id=a | {stop @stop-color=red}}', links = 0, blocker;
            if (mode === 'chain' || mode === 'base') { defs = '{linearGradient @id=a @href="#b"}{radialGradient @id=b | {stop @stop-color=blue}}'; links = 1; }
            if (mode === 'base') blocker = 'svg-paint-base-context-required';
            if (mode === 'cycle') { defs = '{linearGradient @id=a @href="#b"}{radialGradient @id=b @href="#a"}'; links = 2; blocker = 'svg-paint-template-cycle'; }
            if (mode === 'duplicate') { defs += '{pattern @id=a}'; blocker = 'svg-paint-target-ambiguous'; }
            if (mode === 'missing') { defs = ''; blocker = 'svg-paint-target-missing'; }
            if (mode === 'wrong-kind') { defs = '{path @id=a}'; blocker = 'svg-paint-target-kind-unsupported'; }
            if (mode === 'nonfragment') blocker = 'svg-paint-url-untraversed';
            paintRequest(request, doc, defs, mode === 'nonfragment' ? `url(${doc.URL}#a) red` : 'url(#a) red', mode === 'base' ? new URL('../assets/', doc.URL).href : doc.URL);
            const host = capture(), summary = host.read();
            check(summary.svgPaintRoots === 1 && summary.svgPaintLinks === links, 'native paint traversal counts differ');
            if (blocker) check(summary.blockers.includes(blocker), 'unresolved paint edge disappeared');
            check(summary.blockers.includes('svg-paint-rendering-evidence-required') && summary.blockers.includes('svg-paint-content-dependencies-required') && summary.blockers.includes('atomic-host-transaction-required'), 'paint candidate granted closure');
            const target = doc.getElementById('a');
            check(Boolean(target) === (mode !== 'missing'), 'Chromium fragment lookup mismatch');
            if (mode === 'duplicate') check(doc.querySelectorAll('[id="a"]').length === 2 && target.localName === 'linearGradient', 'duplicate fixture lost browser first-match behavior');
            if (links) check(target.href.baseVal === '#b' && doc.getElementById('b') instanceof view.SVGRadialGradientElement, 'Chromium template href/target mismatch');
            const paint = view.getComputedStyle(doc.querySelector('rect')).fill;
            check(paint.includes('#a') && paint.includes('rgb(255, 0, 0)'), 'Chromium computed fragment/fallback serialization mismatch');
        });
    }
    for (const mode of ['target-id', 'target-replace', 'template-href', 'base', 'pool']) {
        await run(`paint-lifetime-${mode}`, async ({ request, doc, captureAsync, pool }) => {
            paintRequest(request, doc, '{linearGradient @id=a @href="#b"}{linearGradient @id=b}');
            const host = await captureAsync();
            check(host.read().svgPaintLinks === 1, 'paint plan not published');
            const target = doc.getElementById('a');
            if (mode === 'target-id') { target.id = 'away'; target.id = 'a'; }
            if (mode === 'target-replace') target.replaceWith(target.cloneNode(true));
            if (mode === 'template-href') { target.setAttribute('href', '#a'); target.setAttribute('href', '#b'); }
            if (mode === 'base') doc.querySelector('base').setAttribute('href', doc.URL);
            if (mode === 'pool') await pool.close();
            rejects(() => host.read(), 'stale paint graph retained evidence');
            rejects(() => host.read(), 'paint graph revival accepted');
        }, mode === 'pool');
    }
    for (const mode of ['stop-value', 'stop-replace', 'pattern-text', 'pattern-child', 'ancestor-style', 'ancestor-class', 'pool']) {
        await run(`paint-content-lifetime-${mode}`, async ({ request, doc, capture, captureAsync, pool }) => {
            const pattern = mode.startsWith('pattern');
            paintRequest(request, doc, pattern
                ? '{pattern @id=a @width=1 @height=1 | {title | Pattern}{g | {rect @width=1 @height=1}}}'
                : '{linearGradient @id=a @href="#b" | {stop @stop-color=red}}{linearGradient @id=b | {stop @stop-color=blue}}');
            const host = mode === 'stop-value' ? capture() : await captureAsync();
            const summary = host.read();
            check(summary.svgPaintServers === (pattern ? 1 : 2), 'content servers not bound to traversal');
            check(summary.svgPaintContentNodes === (pattern ? 5 : 4), 'retained content membership differs');
            check(summary.blockers.includes('svg-paint-clone-style-provenance-required')
                && summary.blockers.includes('svg-paint-content-evaluation-required')
                && summary.blockers.includes('atomic-host-transaction-required'), 'content plan granted rendered closure');
            const stop = doc.querySelector('stop'), defs = doc.querySelector('defs');
            if (mode === 'stop-value') { stop.setAttribute('stop-color', 'blue'); stop.setAttribute('stop-color', 'red'); }
            if (mode === 'stop-replace') stop.replaceWith(stop.cloneNode(true));
            if (mode === 'pattern-text') { const text = doc.querySelector('title').firstChild; text.data = 'Changed'; text.data = 'Pattern'; }
            if (mode === 'pattern-child') { const rect = doc.querySelector('pattern rect'); rect.remove(); doc.querySelector('pattern g').append(rect); }
            if (mode === 'ancestor-style') { defs.setAttribute('style', 'color:blue'); defs.removeAttribute('style'); }
            if (mode === 'ancestor-class') { defs.classList.add('changed'); defs.classList.remove('changed'); }
            if (mode === 'pool') await pool.close();
            rejects(() => host.read(), 'mutated content retained preparation');
            rejects(() => host.read(), 'restored content revived preparation');
        }, mode === 'pool');
    }
    for (const kind of ['gradient', 'pattern']) {
        for (const mode of ['clone', 'own', 'title']) {
            await run(`paint-clone-compatibility-${kind}-${mode}`, async ({ request, doc, view, capture }) => {
                const tag = kind === 'gradient' ? 'linearGradient' : 'pattern';
                const attrs = kind === 'pattern' ? ' @width=1 @height=1 @viewBox="0 0 16 16"' : '';
                const child = kind === 'gradient' ? '{stop @stop-color=currentColor}' : '{rect @width=16 @height=16 @fill=currentColor}';
                paintRequest(request, doc, `{${tag} @id=a @href="#b" @color=blue${attrs} | ${mode === 'own' ? child : mode === 'title' ? '{title | Host}' : ''}}{${tag} @id=b @color=red${attrs} | ${child}}`);
                const host = capture(), summary = host.read();
                check(summary.svgPaintStyleHosts === 2 && summary.svgPaintStyleInstances === (mode === 'own' ? 0 : 1), 'native SVG 2 clone selection differs');
                check(summary.blockers.includes('svg-paint-clone-browser-compatibility-required')
                    && summary.blockers.includes('svg-paint-clone-style-evaluation-required'), 'clone compatibility granted authority');
                check(view.getComputedStyle(doc.getElementById('a')).color === 'rgb(0, 0, 255)'
                    && view.getComputedStyle(doc.getElementById('b')).color === 'rgb(255, 0, 0)', 'source/host style contexts lost');
                // Isolated SVG-image raster probe: no receipt or pixel evidence is
                // attached to the prepared document's native execution.
                const svg = new view.XMLSerializer().serializeToString(doc.querySelector('svg'));
                const img = new Image(); img.src = 'data:image/svg+xml,' + encodeURIComponent(svg); await img.decode();
                const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
                const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
                const pixel = [...ctx.getImageData(8, 8, 1, 1).data];
                const expected = mode === 'own' ? [0, 0, 255, 255]
                    : kind === 'pattern' && mode === 'title' ? [0, 0, 0, 0] : [255, 0, 0, 255];
                check(pixel.every((n, i) => n === expected[i]), `Chromium clone compatibility changed: ${pixel}; review SVG 2 profile before changing expectation`);
                const index = (kind === 'gradient' ? 0 : 3) + ({ own: 0, clone: 1, title: 2 })[mode];
                const color = element => [...view.getComputedStyle(element).color.match(/\d+/g).map(Number), 255];
                cloneReports.set(index, [...color(doc.getElementById('b')), ...color(doc.getElementById('a')), ...pixel]);
                check(host.read().svgCloneProfileConfirmed === false, 'profile implicitly selected');
            });
        }
    }
    for (const mode of ['sync', 'async', 'pool', 'mismatch', 'malformed', 'short-report', 'callback-failure', 'unknown', 'mutating-callback']) {
        await run(`clone-profile-${mode}`, async ({ request, doc, capture, captureAsync, pool }) => {
            paintRequest(request, doc, '{linearGradient @id=a @href="#b"}{linearGradient @id=b | {stop @stop-color=currentColor}}');
            const cloneProfile = {
                name: mode === 'unknown' ? 'unknown' : 'chromium-148-solid-currentcolor-v1',
                observe(index) {
                    const value = [...cloneReports.get(index)];
                    if (mode === 'mismatch') value[8] ^= 1;
                    if (mode === 'malformed') value[0] = 0.5;
                    if (mode === 'short-report') return value.slice(0, 11);
                    if (mode === 'callback-failure') throw new Error('probe unavailable');
                    if (mode === 'mutating-callback') {
                        doc.getElementById('a').setAttribute('color', 'blue');
                        doc.getElementById('a').removeAttribute('color');
                    }
                    return value;
                },
            };
            if (['mismatch', 'malformed', 'short-report', 'callback-failure', 'unknown', 'mutating-callback'].includes(mode)) {
                rejects(() => capture({ cloneProfile }), 'invalid clone profile report accepted');
                return;
            }
            const host = mode === 'sync' ? capture({ cloneProfile }) : await captureAsync({ cloneProfile });
            const summary = host.read();
            check(summary.svgCloneProfileConfirmed === true, 'explicit profile confirmation missing');
            check(summary.blockers.includes('svg-paint-clone-execution-evidence-required')
                && summary.blockers.includes('svg-paint-clone-style-evaluation-required'), 'compatibility report granted execution authority');
            if (pool) await pool.close();
            else { doc.getElementById('b').setAttribute('color', 'red'); doc.getElementById('b').removeAttribute('color'); }
            rejects(() => host.read(), 'stale profile confirmation retained');
        }, mode === 'pool');
    }
    const colorOptions = () => ({
        boundedPaintColors: true,
        cloneProfile: { name: 'chromium-148-solid-currentcolor-v1', observe: index => [...cloneReports.get(index)] },
    });
    for (const mode of ['literal', 'hex', 'current', 'forward', 'override', 'transparent', 'pattern', 'suppressed']) {
        await run(`paint-color-parity-${mode}`, async ({ request, doc, view, capture, captureAsync }) => {
            let defs = '{linearGradient @id=a | {stop @stop-color=blue}}', expected = [0, 0, 255, 255], count = 1, suppressed = 0;
            if (mode === 'hex') { defs = '{linearGradient @id=a | {stop @stop-color="#1a2b3c"}}'; expected = [26, 43, 60, 255]; }
            if (mode === 'current') { defs = '{linearGradient @id=a @href="#b" @color=blue}{linearGradient @id=b @color=red | {stop @stop-color=currentColor}}'; expected = [255, 0, 0, 255]; count = 2; }
            if (mode === 'forward') { defs = '{g @color="#0f0" | {linearGradient @id=a @color=currentColor | {stop @color=currentColor @stop-color=currentColor}}}'; expected = [0, 255, 0, 255]; }
            if (mode === 'override') defs = '{linearGradient @id=a @color=red | {stop @color=blue @stop-color=currentColor}}';
            if (mode === 'transparent') { defs = '{linearGradient @id=a | {stop @stop-color=transparent}}'; expected = [0, 0, 0, 0]; }
            if (mode === 'pattern') defs = '{pattern @id=a @width=1 @height=1 @viewBox="0 0 16 16" @color=blue | {rect @width=16 @height=16 @fill=currentColor}}';
            if (mode === 'suppressed') { defs = '{pattern @id=a @href="#b" @width=1 @height=1 @viewBox="0 0 16 16" | {title | Host}}{pattern @id=b @width=1 @height=1 @viewBox="0 0 16 16" | {rect @width=16 @height=16 @fill=blue}}'; suppressed = 1; expected = [0, 0, 0, 0]; }
            paintRequest(request, doc, defs);
            const host = mode === 'literal' ? capture(colorOptions()) : await captureAsync(colorOptions());
            const summary = host.read();
            check(summary.svgPaintColors === count && summary.svgPaintSuppressedHosts === suppressed, 'native color derivations missing');
            check(summary.blockers.includes('svg-computed-winning-source-required')
                && summary.blockers.includes('svg-paint-rendering-evidence-required'), 'color derivations granted execution closure');
            const img = new Image();
            img.src = 'data:image/svg+xml,' + encodeURIComponent(new view.XMLSerializer().serializeToString(doc.querySelector('svg')));
            await img.decode();
            const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
            const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
            const pixel = [...ctx.getImageData(8, 8, 1, 1).data];
            check(pixel.every((n, i) => n === expected[i]), `paint color raster mismatch: ${mode}: ${pixel}`);
            host.read();
        });
    }
    for (const mode of ['source-color', 'host-color', 'stop-color', 'source-replace', 'pool']) {
        await run(`paint-color-lifetime-${mode}`, async ({ request, doc, captureAsync, pool }) => {
            paintRequest(request, doc, '{linearGradient @id=a @href="#b" @color=blue}{linearGradient @id=b @color=red | {stop @stop-color=currentColor}}');
            const host = await captureAsync(colorOptions());
            check(host.read().svgPaintColors === 2, 'color lifetime fixture missing derivations');
            if (mode === 'source-color' || mode === 'host-color') {
                const node = doc.getElementById(mode === 'source-color' ? 'b' : 'a'), before = node.getAttribute('color');
                node.setAttribute('color', 'green'); node.setAttribute('color', before);
            }
            if (mode === 'stop-color') { const node = doc.querySelector('stop'); node.setAttribute('stop-color', 'blue'); node.setAttribute('stop-color', 'currentColor'); }
            if (mode === 'source-replace') { const node = doc.getElementById('b'); node.replaceWith(node.cloneNode(true)); }
            if (pool) await pool.close();
            rejects(() => host.read(), 'stale paint color derivation retained');
            rejects(() => host.read(), 'paint color derivation revived');
        }, mode === 'pool');
    }
    for (const mode of ['unsupported-inline-css', 'variable', 'missing-profile']) {
        await run(`paint-color-reject-${mode}`, ({ request, doc, capture }) => {
            paintRequest(request, doc, mode === 'unsupported-inline-css'
                ? '{linearGradient @id=a | {stop @style="filter:blur(1px)" @stop-color=red}}'
                : mode === 'variable' ? '{linearGradient @id=a | {stop @stop-color="var(--paint)"}}'
                : '{linearGradient @id=a | {stop @stop-color=red}}');
            rejects(() => capture(mode === 'missing-profile' ? { boundedPaintColors: true } : colorOptions()), 'unsupported color evaluation accepted');
        });
    }
    for (const mode of ['normal', 'last', 'important', 'important-last', 'inline-only', 'inheritance', 'child-local', 'style-restore', 'priority-restore']) {
        await run(`paint-inline-${mode}`, async ({ request, doc, view, capture, captureAsync }) => {
            let style = 'stop-color:blue', attrs = '@stop-color=red', parent = '';
            if (mode === 'last') style = 'stop-color:red;stop-color:blue';
            if (mode === 'important' || mode === 'priority-restore') style = 'stop-color:blue!important;stop-color:red';
            if (mode === 'important-last') style = 'stop-color:red!important;stop-color:blue!important';
            if (mode === 'inline-only') attrs = '';
            if (mode === 'inheritance') { style = 'stop-color:currentColor'; parent = '@color=red @style="color:blue!important"'; }
            if (mode === 'child-local') { style = 'color:blue;stop-color:currentColor'; parent = '@style="color:red!important"'; }
            paintRequest(request, doc, `{linearGradient @id=a ${parent} | {stop ${attrs} @style="${style}"}}`);
            const host = mode === 'normal' ? capture(colorOptions()) : await captureAsync(colorOptions());
            check(host.read().svgPaintColors === 1, 'inline derivation missing');
            const stop = doc.querySelector('stop');
            check(view.getComputedStyle(stop).stopColor === 'rgb(0, 0, 255)', 'inline priority differs from native');
            const img = new Image();
            img.src = 'data:image/svg+xml,' + encodeURIComponent(new view.XMLSerializer().serializeToString(doc.querySelector('svg')));
            await img.decode();
            const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
            const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
            check([...ctx.getImageData(8, 8, 1, 1).data].join(',') === '0,0,255,255', 'inline paint raster differs');
            host.read();
            if (mode === 'style-restore' || mode === 'priority-restore') {
                stop.style.setProperty('stop-color', 'red', 'important');
                stop.setAttribute('style', style);
                rejects(() => host.read(), 'restored inline declaration revived preparation');
            }
        });
    }
    const paintSheet = text => '{style |```' + text + '```}';
    for (const mode of ['universal', 'specificity', 'list', 'important', 'inline-important', 'order', 'inheritance', 'clone', 'pattern', 'pool']) {
        await run(`paint-sheet-parity-${mode}`, async ({ request, doc, view, capture, captureAsync }) => {
            let css = 'stop{stop-color:blue}', attrs = '@id=s @class=hot @stop-color=red';
            let gradient = '', extra = '';
            if (mode === 'universal') css = '*{stop-color:blue}';
            if (mode === 'specificity') css = '#s{stop-color:blue} .hot{stop-color:red}';
            if (mode === 'list') css = '#missing, stop{stop-color:red} .hot, stop.hot{stop-color:blue}';
            if (mode === 'important') { css = 'stop{stop-color:blue!important}'; attrs += ' @style="stop-color:red"'; }
            if (mode === 'inline-important') { css = '#s{stop-color:red!important}'; attrs += ' @style="stop-color:blue!important"'; }
            if (mode === 'order') { css = 'stop{stop-color:red}'; extra = paintSheet('stop{stop-color:blue}'); }
            if (mode === 'inheritance') { css = '#a{color:red!important}stop{color:blue;stop-color:currentColor}'; }
            if (mode === 'clone') { css = '#b{color:blue}stop{stop-color:currentColor}'; gradient = '{linearGradient @id=a @href="#b" @color=red}{linearGradient @id=b | {stop}}'; }
            if (mode === 'pattern') { css = 'rect.tile{fill:blue}'; gradient = '{pattern @id=a @width=1 @height=1 @viewBox="0 0 16 16" | {rect @class=tile @width=16 @height=16 @fill=red}}'; }
            paintRequest(request, doc, paintSheet(css) + (gradient || `{linearGradient @id=a | {stop ${attrs}}}`) + extra);
            const host = mode === 'universal' ? capture(colorOptions()) : await captureAsync(colorOptions());
            check(host.read().svgPaintColors === (mode === 'clone' ? 2 : 1), 'sheet color derivations missing');
            const computed = view.getComputedStyle(doc.querySelector(mode === 'pattern' ? '.tile' : 'stop'));
            check((mode === 'pattern' ? computed.fill : computed.stopColor) === 'rgb(0, 0, 255)', 'sheet CSSOM/native priority differs');
            const img = new Image();
            img.src = 'data:image/svg+xml,' + encodeURIComponent(new view.XMLSerializer().serializeToString(doc.querySelector('svg')));
            await img.decode();
            const canvas = document.createElement('canvas'); canvas.width = canvas.height = 16;
            const ctx = canvas.getContext('2d'); ctx.drawImage(img, 0, 0);
            check([...ctx.getImageData(8, 8, 1, 1).data].join(',') === '0,0,255,255', `sheet raster differs: ${mode}`);
            host.read();
        }, mode === 'pool');
    }
    for (const mode of ['class', 'id', 'rule-restore', 'priority-restore', 'disable-restore', 'replace', 'delete-rule', 'cssom-source', 'pool', 'abort']) {
        await run(`paint-sheet-revoke-${mode}`, async ({ request, doc, captureAsync, pool }) => {
            paintRequest(request, doc, paintSheet('stop.hot{stop-color:blue!important}') + '{linearGradient @id=a | {stop @id=s @class=hot}}');
            const abort = new AbortController();
            const host = await captureAsync({ ...colorOptions(), signal: abort.signal });
            check(host.read().svgPaintColors === 1, 'sheet lifetime fixture missing');
            const node = doc.querySelector('style'), sheet = node.sheet, stop = doc.querySelector('stop');
            if (mode === 'class' || mode === 'id') { const before = stop.getAttribute(mode); stop.setAttribute(mode, 'changed'); stop.setAttribute(mode, before); }
            // CSSOM change-and-restore requires the existing exclusive-owner write boundary.
            if (mode === 'rule-restore') host.change(() => { sheet.insertRule('stop{stop-color:red}', 0); sheet.deleteRule(0); });
            if (mode === 'priority-restore') host.change(() => { const style = sheet.cssRules[0].style; style.setProperty('stop-color', 'blue'); style.setProperty('stop-color', 'blue', 'important'); });
            if (mode === 'disable-restore') host.change(() => { sheet.disabled = true; sheet.disabled = false; });
            if (mode === 'replace') node.replaceWith(node.cloneNode(true));
            if (mode === 'delete-rule') sheet.deleteRule(0);
            if (mode === 'cssom-source') sheet.cssRules[0].style.setProperty('stop-color', 'red', 'important');
            if (mode === 'pool') await pool.close();
            if (mode === 'abort') abort.abort();
            rejects(() => host.read(), 'stale sheet colors retained');
            rejects(() => host.read(), 'sheet colors revived');
        }, mode === 'pool');
    }
    for (const mode of ['disabled', 'tampered', 'media']) {
        await run(`paint-sheet-reject-${mode}`, ({ request, doc, capture, install }) => {
            const sheet = mode === 'media' ? '{style @media=all |```stop{stop-color:blue}```}' : paintSheet(mode === 'disabled' ? '.missing{stop-color:red}' : 'stop{stop-color:blue}');
            paintRequest(request, doc, sheet + '{linearGradient @id=a | {stop @stop-color=blue}}');
            rejects(() => capture(colorOptions(), html => {
                install(html);
                if (mode === 'disabled') doc.querySelector('style').sheet.disabled = true;
                if (mode === 'tampered') doc.querySelector('style').sheet.cssRules[0].style.setProperty('stop-color', 'red');
            }), 'invalid sheet installation accepted');
        });
    }
    for (const mode of ['source-style', 'host-style', 'href', 'own-child', 'pool']) {
        await run(`paint-clone-lifetime-${mode}`, async ({ request, doc, captureAsync, pool }) => {
            paintRequest(request, doc, '{linearGradient @id=a @href="#b" @color=blue}{linearGradient @id=b @color=red | {stop @stop-color=currentColor}}');
            const host = await captureAsync();
            check(host.read().svgPaintStyleInstances === 1, 'clone plan missing');
            const a = doc.getElementById('a'), b = doc.getElementById('b');
            if (mode === 'source-style' || mode === 'host-style') {
                const target = mode === 'source-style' ? b : a, color = target.getAttribute('color');
                target.setAttribute('color', 'green'); target.setAttribute('color', color);
            }
            if (mode === 'href') { a.setAttribute('href', '#a'); a.setAttribute('href', '#b'); }
            if (mode === 'own-child') { const child = doc.querySelector('stop').cloneNode(true); a.append(child); child.remove(); }
            if (mode === 'pool') await pool.close();
            rejects(() => host.read(), 'stale clone context accepted');
            rejects(() => host.read(), 'clone context revived');
        }, mode === 'pool');
    }
    const addSvg = (request, extra = '') => {
        const svg = "{result-element @name=svg @namespace='http://www.w3.org/2000/svg' | {result-element @name=path @namespace='http://www.w3.org/2000/svg' | {result-attribute @name=fill | red}} " + extra + "}";
        request.sources[0].source = source.replace('p{color:rgb(1,2,3)}', 'p{color:rgb(1,2,3)}path{fill:blue}').replace('| Card}', '| Card ' + svg + '}');
    };
    for (const mode of ['sync', 'async', 'pool']) {
        await run(`svg-computed-${mode}`, async ({ request, capture, captureAsync, doc, view, pool }) => {
            addSvg(request);
            const host = mode === 'sync' ? capture() : await captureAsync();
            const summary = host.read();
            check(summary.svgComputedSlots === 1, 'missing computed SVG slot');
            check(view.getComputedStyle(doc.querySelector('svg path')).fill === 'rgb(0, 0, 255)', 'cascade override missing');
            check(summary.blockers.includes('svg-computed-resource-acquisition-required') && summary.blockers.includes('atomic-host-transaction-required'), 'SVG observation widened admission');
            if (pool) { await pool.close(); rejects(() => host.read(), 'pool closure retained SVG evidence'); }
        }, mode === 'pool');
    }
    for (const mode of ['owner', 'cssom', 'focus', 'animation']) {
        await run(`svg-computed-${mode}`, ({ request, capture, doc, view }) => {
            addSvg(request);
            const host = capture(), path = doc.querySelector('svg path');
            if (mode === 'owner') path.replaceWith(path.cloneNode(true));
            if (mode === 'cssom') doc.querySelector('style').sheet.insertRule('path{fill:green}', 0);
            if (mode === 'focus') path.dispatchEvent(new view.FocusEvent('focusin', { bubbles: true }));
            if (mode === 'animation') path.animate([{ fill: 'red' }, { fill: 'green' }], { duration: 10000 });
            rejects(() => host.read(), 'changed SVG state accepted');
            for (const animation of doc.getAnimations()) animation.cancel();
            rejects(() => host.read(), 'restoration revived SVG evidence');
        });
    }
    await run('svg-computed-smil', ({ request, capture }) => {
        addSvg(request, "{result-element @name=animate @namespace='http://www.w3.org/2000/svg' | {result-attribute @name=attributeName | fill}}");
        rejects(() => capture(), 'SMIL acquired static evidence');
    });
    await run('svg-computed-preference', ({ request, capture, view }) => {
        addSvg(request);
        const original = view.matchMedia.bind(view), queries = [];
        view.matchMedia = query => { const result = original(query); queries.push(result); return result; };
        const host = capture();
        const forced = queries.find(query => query.media === '(forced-colors: active)');
        check(forced, 'SVG preference listener missing');
        forced.dispatchEvent(new Event('change'));
        rejects(() => host.read(), 'SVG preference transition retained evidence');
    });
    await run('svg-computed-namespace', ({ request, capture, install, doc }) => {
        addSvg(request);
        rejects(() => capture({}, html => {
            install(html);
            doc.querySelector('svg').setAttributeNS('http://www.w3.org/2000/xmlns/', 'xmlns', 'urn:wrong');
        }), 'incorrect SVG namespace declaration was ignored');
    });
    await run('svg-computed-post-read', async ({ request, captureAsync }) => {
        addSvg(request);
        const host = await captureAsync({}, undefined, session => {
            const read = session.read.bind(session);
            session.read = (...args) => {
                const observe = args[5]; let calls = 0;
                args[5] = (...slot) => ++calls === 1 ? observe(...slot) : 'none';
                return read(...args);
            };
        });
        rejects(() => host.read(), 'post-read computed change accepted');
        rejects(() => host.read(), 'post-read failure did not latch closed');
    });
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
        const screen = queries.find(query => query.media === 'screen');
        check(screen, 'screen media listener not installed');
        screen.dispatchEvent(new Event('change'));
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
