import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { chromium } from 'playwright';
import { verifySvgEnginePrimitives } from './verify-svg-engine-primitives.mjs';

const VERSION = '148.0.7778.96';
const blockers = ['browser-origin-completeness-unproven', 'browser-origin-revision-incomplete',
    'browser-mutation-interval-unavailable', 'browser-environment-provenance-unproven'];
/** Diagnostic prototype only. CDP rule records stay in this test harness;
 * they never cross native ingress or create a lease. No cascade is evaluated here. */
class OriginProbe {
    constructor(session) {
        this.session = session;
        this.revision = 0;
        this.closed = false;
        this.changed = () => { this.revision++; };
        this.events = ['CSS.styleSheetAdded', 'CSS.styleSheetChanged', 'CSS.styleSheetRemoved',
            'CSS.mediaQueryResultChanged', 'DOM.attributeModified', 'DOM.attributeRemoved', 'DOM.documentUpdated'];
        this.events.forEach(event => session.on(event, this.changed));
    }
    async open() {
        await this.session.send('DOM.enable');
        await this.session.send('CSS.enable');
        const { root } = await this.session.send('DOM.getDocument');
        const { nodeId } = await this.session.send('DOM.querySelector', { nodeId: root.nodeId, selector: 'rect' });
        this.nodeId = nodeId;
        this.backend = (await this.session.send('DOM.describeNode', { nodeId })).node.backendNodeId;
    }
    async read(property = 'fill') {
        assert.ok(!this.closed, 'provider closed');
        const before = this.revision;
        const { node } = await this.session.send('DOM.describeNode', { nodeId: this.nodeId });
        assert.equal(node.backendNodeId, this.backend, 'consumer identity changed');
        const matched = await this.session.send('CSS.getMatchedStylesForNode', { nodeId: this.nodeId });
        const computed = await this.session.send('CSS.getComputedStyleForNode', { nodeId: this.nodeId });
        const environment = await this.session.send('Runtime.evaluate', { expression: "matchMedia('(forced-colors: active)').matches", returnByValue: true });
        return {
            property, backend: this.backend, before, after: this.revision,
            presentation: matched.attributesStyle?.cssProperties.filter(p => p.name === property).map(p => p.value) ?? [],
            computed: computed.computedStyle.find(p => p.name === property)?.value,
            matches: (matched.matchedCSSRules ?? []).flatMap(({ rule }) => rule.style.cssProperties
                .filter(p => p.name === property).map(p => ({ origin: rule.origin, value: p.value, important: !!p.important }))),
            forcedColors: environment.result.value,
            admission: false, blockers,
        };
    }
    async close() {
        this.closed = true;
        this.events.forEach(event => this.session.off(event, this.changed));
        await this.session.detach();
    }
}

export async function verifySvgOriginProvider(browser) {
    assert.equal(browser.version(), VERSION);
    const server = createServer((request, response) => {
        if (request.url.endsWith('.svg')) {
            response.writeHead(200, { 'content-type': 'image/svg+xml' });
            response.end('<svg xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="p"><stop stop-color="lime"/></linearGradient></defs></svg>');
        } else {
            response.writeHead(200, { 'content-type': 'text/html' });
            response.end('<!doctype html><html><head></head><body><svg width="16" height="16"><rect width="16" height="16" fill="url(/source.svg#p)" stroke="url(/source.svg#p)"/></svg></body></html>');
        }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    const directory = mkdtempSync(join(tmpdir(), 'cem-origin-provider-'));
    // An isolated test extension supplies actual author/user origins. It does not
    // modify the user's browser/profile or act as a production provider.
    writeFileSync(join(directory, 'manifest.json'), JSON.stringify({ manifest_version: 3,
        name: 'CEM origin fixture', version: '1.0', permissions: ['scripting', 'tabs'],
        host_permissions: [`${origin}/*`], background: { service_worker: 'worker.js' } }));
    writeFileSync(join(directory, 'worker.js'), 'chrome.runtime.onInstalled.addListener(() => {});');
    let context;
    const cases = [];
    try {
        context = await chromium.launchPersistentContext('', { channel: 'chromium', headless: true,
            args: [`--disable-extensions-except=${directory}`, `--load-extension=${directory}`] });
        assert.equal(context.browser().version(), VERSION);
        const worker = context.serviceWorkers()[0] ?? await context.waitForEvent('serviceworker', { timeout: 10000 });
        const page = await context.newPage();
        const source = `url("${origin}/source.svg#p")`;
        const alias = `url("${origin}/alias.svg#p")`;
        const inject = async (css, cssOrigin, remove = false) => worker.evaluate(async ({ css, cssOrigin, remove, origin }) => {
            const tabs = await chrome.tabs.query({ url: `${origin}/*` });
            if (tabs.length !== 1) throw new Error('unique fixture tab required');
            await chrome.scripting[remove ? 'removeCSS' : 'insertCSS']({ target: { tabId: tabs[0].id }, css, origin: cssOrigin });
        }, { css, cssOrigin, remove, origin });
        const open = async () => {
            await page.goto(origin);
            const probe = new OriginProbe(await context.newCDPSession(page));
            await probe.open();
            return probe;
        };
        let probe = await open();
        for (const property of ['fill', 'stroke']) {
            const report = await probe.read(property);
            assert.equal(report.presentation.length, 1);
            assert.equal(report.computed, source);
            assert.equal(report.admission, false);
            cases.push({ name: `presentation-${property}`, report });
        }
        const { root } = await probe.session.send('DOM.getDocument');
        const { nodeId } = await probe.session.send('DOM.querySelector', { nodeId: root.nodeId, selector: 'svg' });
        const ua = await probe.session.send('CSS.getMatchedStylesForNode', { nodeId });
        assert.ok(ua.matchedCSSRules.some(m => m.rule.origin === 'user-agent'));
        cases.push({ name: 'ua-rule-visible', provesUaWinnerCompleteness: false });
        await probe.close();

        for (const cssOrigin of ['AUTHOR', 'USER']) {
            probe = await open();
            const css = `rect { fill: ${source} !important; }`;
            const baseline = await probe.read();
            await inject(css, cssOrigin);
            const same = await probe.read();
            assert.equal(same.computed, source);
            assert.ok(same.matches.some(m => m.important), `${cssOrigin} rule not observed`);
            assert.deepEqual(same.matches.map(m => m.origin), [cssOrigin === 'USER' ? 'user-agent' : 'injected']);
            if (cssOrigin === 'USER') {
                await inject(css, cssOrigin, true);
                const restored = await probe.read();
                assert.equal(restored.computed, baseline.computed);
                assert.equal(restored.after, baseline.after, 'review new USER-origin invalidation behavior');
                assert.equal(same.after, baseline.after, 'review new USER-origin addition event');
                cases.push({ name: 'user-origin-unreported-aba', before: baseline.after, during: same.after, after: restored.after, revisionComplete: false });
                await inject(css, cssOrigin);
            }
            await probe.session.send('DOM.setAttributeValue', { nodeId: probe.nodeId, name: 'fill', value: alias });
            const retargeted = await probe.read();
            assert.equal(retargeted.computed, source, 'override must keep original URL after alias write');
            assert.notEqual(retargeted.presentation[0], same.presentation[0]);
            cases.push({ name: `${cssOrigin.toLowerCase()}-same-value-override`, same, retargeted });
            await inject(css, cssOrigin, true);
            assert.equal((await probe.read()).computed, alias);
            await probe.close();
        }
        probe = await open();
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal((await probe.read()).forcedColors, true);
        cases.push({ name: 'initial-forced-colors', admission: false });
        // Runtime.evaluate is a diagnostic page observation, not trusted media
        // provenance. Prove that routing it through CDP does not authenticate it.
        await page.evaluate(() => { globalThis.savedMatchMedia = matchMedia; globalThis.matchMedia = () => ({ matches: false }); });
        assert.equal((await probe.read()).forcedColors, false);
        cases.push({ name: 'page-can-spoof-media-observation', environmentAuthenticated: false });
        await page.evaluate(() => { globalThis.matchMedia = globalThis.savedMatchMedia; delete globalThis.savedMatchMedia; });
        assert.equal((await probe.read()).forcedColors, true);
        await page.emulateMedia({ forcedColors: 'none' });
        assert.equal((await probe.read()).forcedColors, false);
        cases.push({ name: 'forced-colors-restored', admission: false });
        const before = await probe.read();
        await page.evaluate(() => { const r = document.querySelector('rect'), value = r.getAttribute('fill'); r.setAttribute('fill', 'red'); r.setAttribute('fill', value); });
        const restored = await probe.read();
        assert.equal(restored.computed, before.computed);
        assert.ok(restored.after > before.after, 'DOM A-B-A must be observable');
        cases.push({ name: 'dom-same-value-revision', before: before.after, after: restored.after });
        // A CDP read is not a mutation interval: a privileged competing write
        // succeeds between the observation and the proposed native alias write.
        const css = `rect { fill: ${source} !important; }`;
        const observation = await probe.read();
        await inject(css, 'USER');
        await probe.session.send('DOM.setAttributeValue', { nodeId: probe.nodeId, name: 'fill', value: alias });
        const raced = await probe.read();
        assert.equal(observation.computed, raced.computed);
        assert.equal(raced.computed, source);
        cases.push({ name: 'observation-write-gap', observation, raced, intervalEstablished: false });
        await inject(css, 'USER', true);
        await page.goto(origin + '/replaced');
        await assert.rejects(() => probe.read());
        cases.push({ name: 'navigation-invalidates-node' });
        await probe.close();
        await assert.rejects(() => probe.read());
        cases.push({ name: 'disconnected-provider' });
        probe = await open();
        await probe.session.detach();
        await assert.rejects(() => probe.read());
        cases.push({ name: 'transport-detached' });
        const enginePrimitives = await verifySvgEnginePrimitives(context, page, origin);
        assert.equal(enginePrimitives.cases.length, 5);
        assert.equal(cases.length, 14);
        console.log(`Verified ${cases.length} privileged SVG origin feasibility cases in Chromium ${VERSION}; production admission remains blocked.`);
        return { profile: 'chromium-148-cdp-origin-feasibility-v1', version: VERSION,
            executionSeal: false, productionAdmission: false, blockers, cases, enginePrimitives };
    } finally {
        try { await context?.close(); }
        finally {
            rmSync(directory, { recursive: true, force: true });
            await new Promise(resolve => server.close(resolve));
        }
    }
}
