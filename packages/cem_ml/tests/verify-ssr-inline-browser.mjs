import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { chromium } from 'playwright';

// Explicit JSON test-control input; native import owns template source semantics.
const vectors = JSON.parse(readFileSync(new URL('./fixtures/inline-cem-source.json', import.meta.url), 'utf8'));
const browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_CHROME });
try {
    const context = await browser.newContext({ javaScriptEnabled: false });
    const url = 'https://ssr.example.test/guide/';
    let html;
    const externalRequests = [];
    await context.route('**/*', async route => {
        if (route.request().isNavigationRequest() && route.request().url() === url) {
            await route.fulfill({ contentType: 'text/html; charset=utf-8', body: html });
        } else {
            externalRequests.push(route.request().url());
            await route.abort();
        }
    });
    const page = await context.newPage();
    const results = [];
    for (const vector of vectors) {
        html = vector.html;
        await page.goto(url);
        const text = await page.evaluate(() => document.querySelector('cem-element > template').content.textContent);
        assert.equal(text, vector.text, vector.name);
        results.push({ name: vector.name, text });
    }
    assert.deepEqual(externalRequests, []);
    const report = new URL('../../../dist/reports/cem-ssr-inline-browser.json', import.meta.url);
    mkdirSync(new URL('.', report), { recursive: true });
    writeFileSync(report, `${JSON.stringify({ browser: browser.version(), authoredJavaScript: false, externalRequests, results }, null, 2)}\n`);
    console.log(`Inline CEM source browser parity: ${results.length}/${results.length}`);
} finally {
    await browser.close();
}
