import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { chromium } from 'playwright';

// Explicit browser-observation control data, not a production AST handoff.
const vectors = JSON.parse(readFileSync(new URL('./fixtures/ssr-island-capture.json', import.meta.url), 'utf8'));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
try {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const url = 'https://ssr.example.test/guide/';
    let body;
    const externalRequests = [];
    await context.route('**/*', async route => {
        if (route.request().isNavigationRequest() && route.request().url() === url) {
            await route.fulfill({ contentType: 'text/html; charset=utf-8', body: `<!doctype html><html><body>${body}</body></html>` });
        } else {
            externalRequests.push(route.request().url());
            await route.abort();
        }
    });
    const page = await context.newPage();
    const results = [];
    for (const vector of vectors) {
        body = vector.body;
        await page.goto(url);
        const observation = await page.evaluate(() => {
            const host = document.querySelector('x-inline');
            const templates = Array.from(host.children).filter(child => child.localName === 'template');
            const meaningful = Array.from(host.childNodes).filter(node => !(node.nodeType === 3 && node.textContent.trim() === '')
                && !(node.nodeType === 8 && ['cem-render-start', 'cem-render-end'].includes(node.textContent)));
            const mode = templates.length === 0 ? 'live' : templates.length === 1 && meaningful.length === 1 ? 'html-envelope' : 'mixed';
            const nodes = mode === 'mixed' ? [] : Array.from(mode === 'live' ? host.childNodes : templates[0].content.childNodes)
                .map(node => node.nodeType === 3 ? '#text' : node.nodeType === 8 ? '#comment' : node.localName);
            return { mode, nodes };
        });
        assert.deepEqual(observation, { mode: vector.mode, nodes: vector.nodes }, vector.name);
        results.push({ name: vector.name, ...observation });
    }
    assert.deepEqual(externalRequests, []);
    const report = new URL('../../../dist/reports/cem-ssr-island-browser.json', import.meta.url);
    mkdirSync(new URL('.', report), { recursive: true });
    writeFileSync(report, `${JSON.stringify({ browser: browser.version(), authoredJavaScript: false, externalRequests, results }, null, 2)}\n`);
    console.log(`Native/browser payload capture observations: ${results.length}/${results.length}`);
} finally {
    await browser.close();
}
