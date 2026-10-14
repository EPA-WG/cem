import { captureBrowserCompiledStylesheets, createBrowserWorkerPool } from '../dist/browser.js';
import * as ql from '../../cem_ql/dist/wasm/cem_ql.js';
const check = (value, message) => { if (!value) throw new Error(message); };
const rejects = (work, message) => { let rejected = false; try { work(); } catch { rejected = true; } check(rejected, message); };
const child = 'p{color:rgb(3, 4, 5)}';
const source = '@import "child.css" supports(display: grid); p{font-style:italic}';

globalThis.runCompiledStylesheetsFixture = async () => {
    await ql.default();
    const cases = [];
    const run = async (name, test, { kind = 'private', pool = false, css = source } = {}) => {
        const frame = document.createElement('iframe');
        frame.srcdoc = '<!doctype html><html><head></head><body><cem-card><template data-cem-island="instance"></template><p>Scoped</p></cem-card><p>Outside</p></body></html>';
        const loaded = new Promise(resolve => { frame.onload = resolve; });
        document.body.append(frame);
        let owner, host, workers;
        const otherHosts = [];
        try {
            await loaded;
            const doc = frame.contentDocument, view = frame.contentWindow;
            const template = kind === 'instance'
                ? JSON.parse(ql.adoptInstanceStylesheets(JSON.stringify([{ css, scope: null }]), 'fixture-instance'))
                : JSON.parse(ql.compileTemplate(`{style${kind === 'shared' ? ' @scope=cards' : ''} |\`\`\`${css}\`\`\`}`, '[]'));
            owner = template.artifactId;
            check(owner > 0 && template.diagnostics.length === 0, 'native template compilation failed');
            const options = { consumer: 'host-one', index: 0, declarationIdentity: kind === 'instance' ? 'fixture-instance' : 'card',
                baseUrl: new URL('/styles/main.css', location.href).href,
                scope: kind === 'shared' ? { kind, name: 'cards' } : kind === 'private' ? { kind, tag: 'cem-card' } : { kind },
                context: { identity: 'page', resolverIdentity: 'resolver', resourcePolicyStamp: 'policy',
                    frames: [{ frameId: 'page', baseUrl: location.href, scopes: [], specifiers: { imports: {}, resources: {} } }] } };
            const load = (consumer = 'host-one') => {
                const pending = JSON.parse(ql.beginTemplateStylesheet(owner, JSON.stringify({ ...options, consumer })));
                if (pending.status === 'ready') return pending;
                check(pending.status === 'pending', `native load failed: ${JSON.stringify(pending)}`);
                return JSON.parse(ql.deliverTemplateStylesheet(owner, pending.loadId, consumer, pending.request.id,
                    new TextEncoder().encode(child), new URL('/styles/child.css', location.href).href, 'text/css'));
            };
            const ready = load();
            check(ready.status === 'ready', 'native closure not ready');
            const card = doc.querySelector('cem-card');
            if (kind !== 'instance') card.setAttribute('data-cem-css-context', ready.identity.contextMarker);
            if (kind === 'shared') card.setAttribute('scope', 'cards');
            const node = doc.createElement('style'); node.textContent = ready.css;
            (kind === 'instance' ? card : doc.head).append(node);
            if (pool) workers = await createBrowserWorkerPool({ workerCount: 1, hardCancelGraceMs: 10 });
            const capture = (consumer = 'host-one', nodes = [node], extra = {}) => {
                const native = new ql.CompiledStylesheetSession(owner, consumer, '[0]');
                const options = { ownership: 'exclusive', ...extra };
                const result = workers ? workers.captureCompiledStylesheets(doc, native, nodes, options)
                    : captureBrowserCompiledStylesheets(doc, native, nodes, options);
                if (!host) host = result; else otherHosts.push(result);
                return result;
            };
            await test({ doc, view, node, card, ready, capture, load, workers, owner });
            cases.push(name);
        } catch (error) { throw new Error(`${name}: ${String(error)}`, { cause: error }); }
        finally { host?.dispose(); for (const item of otherHosts) item.dispose(); if (owner) ql.disposeTemplate(owner); await workers?.close(); frame.remove(); }
    };
    for (const kind of ['private', 'shared', 'instance']) await run(`${kind}-native-emission-and-scope`, ({ capture, view, doc, card, ready }) => {
        const host = capture(), summary = host.read();
        check(summary.occurrences === 1, 'flattened import was given an invented installation');
        check(summary.blockers.includes('compiled-stylesheet-conditions-runtime-required') && summary.blockers.includes('atomic-host-transaction-required'), 'installation granted complete closure/edit');
        check(!ready.css.includes('@import') && ready.css.includes('@supports'), 'native import flattening missing');
        check(view.getComputedStyle(card.querySelector('p')).color === 'rgb(3, 4, 5)', 'scoped imported CSS not applied');
        check(view.getComputedStyle(doc.body.lastElementChild).color !== 'rgb(3, 4, 5)', 'scope leaked');
    }, { kind });
    await run('shared-consumer-release', ({ owner, capture, load }) => {
        const a = capture(); load('host-two'); const b = capture('host-two');
        ql.releaseTemplateStylesheets(owner, 'host-one', 0);
        rejects(() => a.read(), 'shared cache kept released consumer alive');
        check(b.read().occurrences === 1, 'unrelated consumer lost its live lease');
    });
    await run('consumer-replacement', ({ capture, load }) => {
        const host = capture(); load();
        rejects(() => host.read(), 'replacement revived old consumer generation');
        check(capture().read().occurrences === 1, 'replacement could not capture');
    });
    await run('owner-disposal', ({ owner, capture }) => {
        const host = capture(); ql.disposeTemplate(owner);
        rejects(() => host.read(), 'disposed template retained native authority');
    });
    await run('compiled-cssom-tampering', ({ capture, node }) => {
        const host = capture(); node.sheet.insertRule('p{color:blue}', 0);
        rejects(() => host.read(), 'unowned CSSOM edit was accepted');
        node.sheet.deleteRule(0);
        rejects(() => host.capture(), 'restore revived rejected session');
    });
    await run('compiled-source-replacement', ({ capture, node }) => {
        const host = capture(); node.textContent += 'p{color:blue}';
        rejects(() => host.read(), 'source DOM mutation retained evidence');
        rejects(() => host.capture(), 'source replacement retained installation identity');
    });
    await run('wrong-installed-emission', ({ capture, node }) => {
        node.textContent = '@scope(cem-card){p{color:purple}}';
        rejects(() => capture(), 'different native CSS accepted');
    });
    await run('controlled-cssom-away-and-back', ({ capture, node }) => {
        const host = capture();
        host.change(() => { node.sheet.insertRule('p{color:blue}', 0); node.sheet.deleteRule(0); });
        rejects(() => host.read(), 'CSSOM A-B-A revived old evidence');
        check(host.capture().occurrences === 1, 'fresh controlled capture failed');
    });
    await run('compiled-selection-disable-and-restore', ({ capture, node, card, view }) => {
        const host = capture();
        host.change(() => { node.sheet.disabled = true; });
        rejects(() => host.read(), 'selection retained old evidence'); host.capture();
        check(view.getComputedStyle(card.querySelector('p')).color !== 'rgb(3, 4, 5)', 'disabled sheet applied');
        host.change(() => { node.sheet.disabled = false; });
        rejects(() => host.read(), 'restore revived old evidence'); host.capture();
        check(view.getComputedStyle(card.querySelector('p')).color === 'rgb(3, 4, 5)', 'restored sheet not applied');
    });
    await run('unowned-selection-change', ({ capture, node }) => {
        const host = capture(); node.sheet.disabled = true;
        rejects(() => host.read(), 'unowned selection accepted');
    });
    await run('instance-parent-replacement', ({ capture, node, doc }) => {
        const host = capture(); host.change(() => { doc.head.append(node); });
        rejects(() => host.capture(), 'implicit instance scope moved to different owner');
    }, { kind: 'instance' });
    await run('suppressed-emission-rejected', ({ capture, ready }) => {
        check(ready.diagnostics.length > 0, 'fixture did not suppress unsupported syntax');
        rejects(() => capture(), 'incomplete native emission certified');
    }, { css: '@import "child.css"; #unsupported{color:red}' });
    await run('complete-installation-count', ({ capture }) => rejects(() => capture('host-one', []), 'missing installation accepted'));
    await run('compiled-lifecycle-abort', ({ capture }) => {
        const controller = new AbortController(); const host = capture('host-one', undefined, { signal: controller.signal });
        controller.abort(); rejects(() => host.read(), 'aborted compiled host survived');
    });
    await run('compiled-worker-close', async ({ capture, workers }) => {
        const host = capture(); await workers.close();
        rejects(() => host.read(), 'pool close retained compiled evidence');
        rejects(() => capture(), 'closed pool admitted a new compiled host');
    }, { pool: true });
    await run('compiled-reentry', ({ capture }) => {
        const host = capture();
        rejects(() => host.change(() => { rejects(() => host.read(), 'reentry admitted'); }), 'swallowed reentry retained session');
        rejects(() => host.capture(), 'reentrant compiled host survived');
    });
    await run('compiled-async-change', ({ capture }) => {
        const host = capture(); rejects(() => host.change(() => Promise.resolve()), 'async change admitted');
        rejects(() => host.capture(), 'async compiled host survived');
    });
    await run('native-callback-release', ({ owner }) => {
        const native = new ql.CompiledStylesheetSession(owner, 'host-one', '[0]');
        try { rejects(() => native.capture(() => '1', () => {
            ql.releaseTemplateStylesheets(owner, 'host-one', 0); return [1, true, true];
        }), 'callback release published evidence'); } finally { native.free(); }
    });
    return { cases };
};
