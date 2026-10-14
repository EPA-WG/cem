import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { emitNativeHtmlBaseEvidence } from './verify-html-base.mjs';

export async function verifyStylesheetSvg(browser, workspaceRoot) {
    const source = await readFile(resolve(workspaceRoot, 'packages/cem_ml/tests/fixtures/stylesheet-svg.xhtml'), 'utf8');
    const main = "@import './child.css' screen; p{background-color:rgb(4,5,6)}";
    const child = 'p{border-top:1px solid rgb(7,8,9)}';
    const context = await browser.newContext();
    try {
        const page = await context.newPage();
        await page.route('**/*', async route => {
            const url = route.request().url();
            if (url === 'https://page.test/docs/styles.xhtml') return route.fulfill({ contentType: 'application/xhtml+xml', body: source });
            if (url === 'https://page.test/assets/theme.css' || url === 'https://page.test/assets/child.css') return route.fulfill({ contentType: 'text/css', headers: { 'access-control-allow-origin': '*' }, body: url.endsWith('/theme.css') ? main : child });
            return route.fulfill({ status: 404, body: '' });
        });
        await page.goto('https://page.test/docs/styles.xhtml');
        const actual = await page.evaluate(async ({ main, child }) => {
            const sheets = [document.querySelector('style').sheet, ...Array.from(document.querySelectorAll('link'), link => link.sheet)];
            const identities = new Map();
            const observations = [];
            const walk = async (sheet, root, index, selected, conditions_match, source) => {
                if (sheet) {
                    identities.set(sheet, identities.size + 1);
                    if (sheet.href) {
                        const loaded = await (await fetch(sheet.href, { credentials: 'omit' })).text();
                        if (loaded !== source) throw new Error('CSS response differs from fixture');
                    }
                    // Access to each native CSSOM occurrence must succeed.
                    if (!sheet.cssRules.length) throw new Error('stylesheet rules missing');
                }
                observations.push({ root, sheet: index, selected, conditions_match, source: sheet ? source : null, url: sheet?.href ?? null, installation: sheet ? identities.get(sheet) : null });
            };
            await walk(sheets[0], 0, 0, true, true, document.querySelector('style').textContent);
            for (const root of [1, 2]) {
                const sheet = sheets[root];
                const selected = root === 1 && !sheet.disabled;
                await walk(sheet, root, 0, selected, matchMedia('screen').matches, main);
                const imported = sheet?.cssRules[0]?.styleSheet ?? null;
                await walk(imported, root, 1, selected, matchMedia('screen').matches, child);
            }
            const style = getComputedStyle(document.querySelector('p'));
            const svg = getComputedStyle(document.querySelector('path'));
            const before = { color: style.color, background: style.backgroundColor, border: style.borderTopColor, fill: svg.fill, filter: svg.filter };
            sheets[1].disabled = true;
            const disabled = getComputedStyle(document.querySelector('p')).backgroundColor;
            sheets[1].disabled = false;
            const restored = getComputedStyle(document.querySelector('p')).backgroundColor;
            document.querySelector('link').media = 'print';
            const unmatched = getComputedStyle(document.querySelector('p')).backgroundColor;
            return { base: document.baseURI, observations, before, disabled, restored, unmatched };
        }, { main, child });
        assert.equal(actual.before.color, 'rgb(1, 2, 3)');
        assert.equal(actual.before.background, 'rgb(4, 5, 6)');
        assert.equal(actual.before.border, 'rgb(7, 8, 9)');
        assert.notEqual(actual.disabled, actual.before.background);
        assert.equal(actual.restored, actual.before.background);
        assert.notEqual(actual.unmatched, actual.before.background);
        const [native] = await emitNativeHtmlBaseEvidence(workspaceRoot, '--verify-stylesheet-svg', { version: browser.version(), base: actual.base, observations: actual.observations });
        assert.equal(native.observations, 5);
        assert.ok(actual.before.fill.includes(native.svg_urls[0]));
        // CSS local fragments stay local in Chromium's computed style serialization.
        assert.equal(actual.before.filter, 'url("#filter")');
        assert.deepEqual(native.local_fragments, ['#filter']);
        console.log(`Verified ${native.observations} stylesheet occurrences, disable/restore/media transitions and SVG URLs in Chromium ${browser.version()}.`);
        return { ...native, chromiumVersion: browser.version() };
    } finally { await context.close(); }
}
