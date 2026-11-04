import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import { launchPublicationBrowser } from './publication-browser.mjs';

// Expected counts come from the native Markdown AST and native page fixture,
// rather than a second Markdown parser or table-shaped-text heuristic.
export async function verifyMarkdownTables(origin, entries, audit) {
    assert.equal(audit.status, 'passed');
    const browser = await launchPublicationBrowser();
    const report = { origin, sourceCount: 0, tableCount: 0, cellCount: 0, routes: [] };
    const workspace = resolve(import.meta.dirname, '../../..');
    try {
        const context = await browser.newContext();
        const page = await context.newPage();
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        for (const source of audit.sources) {
            const path = resolve(workspace, 'apps/cem-site', source.source);
            const entry = entries.find(entry => entry.kind === 'page' && resolve(workspace, entry.source) === path);
            assert.ok(entry, `Native Markdown import has no Site route: ${source.importId}`);
            const response = await page.goto(origin + entry.route, { waitUntil: 'domcontentloaded' });
            assert.equal(response.status(), 200, entry.route);
            const tables = page.locator('main table');
            assert.equal(await tables.count(), source.tables, `${entry.route}: missing native Markdown tables`);
            assert.equal(await tables.locator('th, td').count(), source.cells, `${entry.route}: missing native Markdown cells`);
            if (entry.route === '/reference/cem-ml/transform-config/') {
                assert.deepEqual(await tables.locator('thead th').allTextContents(),
                    ['Element', 'Required attributes', 'Optional attributes', 'Child elements']);
                assert.equal(await tables.locator('tbody tr').count(), 8);
                assert.ok((await tables.locator('tbody td code').allTextContents()).includes('rewrite-importmap'));
            }
            report.routes.push({ route: entry.route, tables: source.tables, cells: source.cells });
            report.sourceCount++;
            report.tableCount += source.tables;
            report.cellCount += source.cells;
        }
        assert.equal(report.sourceCount, audit.sourceCount);
        await page.goto(origin + '/guides/declarative-amsterdam-2026/');
        const tokens = page.locator('header nav[aria-label=Primary]').getByRole('link', { name: 'Tokens', exact: true });
        assert.equal(await tokens.getAttribute('href'), '/packages/cem-theme/dist/lib/css-generators/');
        await tokens.click();
        assert.equal(new URL(page.url()).origin, origin);
        assert.equal(new URL(page.url()).pathname, '/packages/cem-theme/dist/lib/css-generators/');
        assert.match(await page.locator('h1').textContent(), /CSS generators/u);
        await page.locator('a.generator').filter({ hasText: /^Colors$/u }).click();
        await page.waitForFunction(() => document.querySelector('code[data-generated-css]')?.textContent.includes('--cem-'));
        assert.equal(await page.locator('cem-css-loader').getAttribute('value'),
            await page.locator('code[data-generated-css]').first().textContent());
        report.tokensNavigation = { href: '/packages/cem-theme/dist/lib/css-generators/', sameOrigin: true, colorsGenerator: 'passed' };
        assert.deepEqual(errors, [], 'Markdown table pages must not introduce browser errors');
        report.status = 'passed';
        await context.close();
        return report;
    } finally { await browser.close(); }
}
