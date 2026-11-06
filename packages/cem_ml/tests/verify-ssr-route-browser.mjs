import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

// JSON is test-control input/output only. Native HTML import owns the AST.
const vectors = JSON.parse(readFileSync(new URL('./fixtures/ssr-route-urls.json', import.meta.url), 'utf8'));
const pageUrl = 'https://ssr.example.test/guide/page.html?edition=2';
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
const results = [];
const refusedRequests = [];
try {
    const context = await browser.newContext({ javaScriptEnabled: false });
    let html;
    await context.route('**/*', async route => {
        if (route.request().isNavigationRequest() && route.request().url() === pageUrl) {
            await route.fulfill({ contentType: 'text/html; charset=utf-8', body: html });
        } else {
            refusedRequests.push(route.request().url());
            await route.abort();
        }
    });
    const page = await context.newPage();
    for (const vector of [...vectors, {
        name: 'CSP blocks selected base',
        html: '<html><head><meta http-equiv="Content-Security-Policy" content="base-uri \'none\'"><base href="/assets/"></head></html>',
        base: pageUrl,
    }]) {
        html = vector.html;
        await page.goto(pageUrl);
        // Playwright evaluation reads browser state even with authored JS disabled.
        const actual = await page.evaluate(() => ({ pageUrl: document.URL, base: document.baseURI }));
        assert.deepEqual(actual, { pageUrl, base: vector.base }, vector.name);
        results.push({ name: vector.name, ...actual,
            ...(vector.nativeError ? { nativeError: vector.nativeError } : {}) });
    }
    assert.deepEqual(refusedRequests, [], 'fixtures must not request external resources');
    const receipt = {
        browser: browser.version(), authoredJavaScript: false, externalRequests: refusedRequests,
        results,
    };
    const report = new URL('../../../dist/reports/cem-ssr-route-browser.json', import.meta.url);
    mkdirSync(new URL('.', report), { recursive: true });
    writeFileSync(report, `${JSON.stringify(receipt, null, 2)}\n`);
    console.log(`SSR route browser URL parity: ${results.length}/${results.length}; ${fileURLToPath(report)}`);
} finally {
    await browser.close();
}
