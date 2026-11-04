import assert from 'node:assert/strict';
import { chromium } from 'playwright';

export async function verifyCodeSnippets(origin, entries) {
    const browser = await chromium.launch();
    const report = { origin, snippetCount: 0, coloredSnippetCount: 0, routes: [] };
    try {
        const context = await browser.newContext();
        for (const entry of entries.filter(entry => entry.kind === 'page')) {
            const page = await context.newPage();
            const errors = []; page.on('pageerror', error => errors.push(error.message));
            await page.goto(origin + entry.route, { waitUntil: 'domcontentloaded' });
            if (entry.contentRole === 'interactive-example') {
                await page.waitForFunction(() => globalThis.__cemSiteInteractive?.done === true);
            }
            const viewers = page.locator('cem-demo-element[demo="false"][source]');
            if (await viewers.count()) {
                await page.waitForFunction(() => [...document.querySelectorAll('cem-demo-element[demo="false"][source]')]
                    .every(node => node.getAttribute('data-state') === 'ready'));
                const snippets = await viewers.evaluateAll(nodes => nodes.map(node => ({
                    source: node.getAttribute('source'), text: node.querySelector('[slot=text] code')?.textContent,
                    colored: Boolean(node.querySelector('[slot=text] code strong, [slot=text] code i, [slot=text] code var, [slot=text] code b, [slot=text] code dfn')),
                    demoChildren: node.querySelector('[slot=demo]')?.childNodes.length,
                })));
                for (const snippet of snippets) {
                    assert.equal(snippet.source, snippet.source.trim(), `${entry.route}: surrounding snippet whitespace`);
                    assert.equal(snippet.text, snippet.source, `${entry.route}: source text changed during coloring`);
                    assert.equal(snippet.demoChildren, 0, `${entry.route}: source-only example was rendered`);
                }
                if (entry.route === '/reference/cem-ml/wasm-runtime/') {
                    assert.ok(snippets.some(snippet => snippet.source.startsWith('import init, { version }')));
                    assert.ok(snippets.some(snippet => snippet.colored), 'WASM reference JavaScript must be colored');
                    assert.equal(await page.locator('main > pre').count(), 0);
                    assert.equal(await page.locator('html').evaluate(node => getComputedStyle(node).backgroundColor), 'rgb(255, 255, 255)');
                }
                report.routes.push({ route: entry.route, snippetCount: snippets.length,
                    coloredSnippetCount: snippets.filter(snippet => snippet.colored).length });
                report.snippetCount += snippets.length;
                report.coloredSnippetCount += snippets.filter(snippet => snippet.colored).length;
            }
            assert.deepEqual(errors, [], `${entry.route}: snippet browser errors`);
            await page.close();
        }
        assert.ok(report.snippetCount && report.coloredSnippetCount, 'No colored code snippets verified');
        assert.ok(report.routes.some(entry => entry.route === '/reference/cem-ml/wasm-runtime/'));
        report.status = 'passed';
        await context.close();
        return report;
    } finally { await browser.close(); }
}
