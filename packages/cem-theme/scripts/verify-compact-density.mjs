import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const css = await readFile(new URL('../dist/lib/css/cem-combined.css', import.meta.url), 'utf8');
const browser = await chromium.launch({ headless: true });
try {
    for (const coarse of [false, true]) {
        const context = await browser.newContext({ hasTouch: coarse, isMobile: coarse, viewport: { width: 1200, height: 800 } });
        const page = await context.newPage();
        await page.setContent(`<style>${css}
        html { font-size:16px; }
        section { font-size:16px; line-height:1.5; }
        .chrome { min-block-size:var(--cem-coupling-zone-min); font:inherit; padding:0; border:0; }
        .rows { display:grid; row-gap:var(--cem-data-row-gap); }
        .row { display:flex; box-sizing:border-box; min-block-size:max(var(--cem-data-row-height),var(--cem-coupling-zone-min)); padding-block:var(--cem-data-row-padding-block); padding-inline:var(--cem-data-row-padding-inline); border:var(--cem-data-row-border-width) solid transparent; font:inherit; }
        .glyph { display:inline-block; inline-size:var(--cem-icon-size); block-size:var(--cem-icon-size); }
        .data-glyph { display:inline-block; inline-size:var(--cem-data-icon-size); block-size:var(--cem-data-icon-size); }
        .project { color:var(--cem-resource-project-color); } .file { color:var(--cem-resource-file-color); }
        </style><section data-cem-density="compact"><button class="chrome">Build <span class="glyph">◇</span></button>
        <section class="rows" data-cem-density="super-compact"><button class="row"><span class="data-glyph">◇</span><span>First resource</span></button><button class="row">Second resource</button><span class="project">Project</span><span class="file">Source</span></section></section>`);
        const measure = () => page.evaluate(() => {
            const chrome = document.querySelector('.chrome');
            const rows = [...document.querySelectorAll('.row')];
            const rowStyle = getComputedStyle(rows[0]);
            return {
                fine: matchMedia('(hover: hover) and (pointer: fine) and (not (any-pointer: coarse))').matches,
                anyCoarse: matchMedia('(any-pointer: coarse)').matches,
                chrome: chrome.getBoundingClientRect().height,
                rows: rows.map((row) => row.getBoundingClientRect().height),
                top: rows.map((row) => row.getBoundingClientRect().top),
                line: parseFloat(rowStyle.lineHeight),
                padding: rowStyle.paddingBlockStart,
                glyph: document.querySelector('.glyph').getBoundingClientRect().width,
                dataGlyph: document.querySelector('.data-glyph').getBoundingClientRect().width,
            };
        });
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            await page.locator('section').first().evaluate((node, mode) => { node.className = `cem-theme-${mode}`; }, theme);
            const metrics = await measure();
            assert.equal(metrics.glyph, 16, `${theme}: chrome glyph follows one text em`);
            assert.equal(metrics.dataGlyph, 16, `${theme}: data glyph follows one text em`);
            if (coarse) {
                assert.equal(metrics.anyCoarse, true);
                assert.equal(metrics.fine, false);
                assert.ok(metrics.chrome >= 48 && metrics.rows.every((height) => height >= 48), `${theme}: coarse targets retain3rem`);
            } else {
                assert.equal(metrics.fine, true);
                assert.ok(metrics.chrome >= 24 && metrics.chrome < 48, `${theme}: explicit compact chrome`);
                assert.equal(metrics.rows[0], metrics.line, `${theme}: row is one paragraph line`);
                assert.equal(metrics.top[1] - metrics.top[0], metrics.line, `${theme}: no extra vertical row spacing`);
                assert.equal(metrics.padding, '0px');
            }
        }
        if (!coarse) {
            await page.locator('.rows').evaluate((node) => { node.style.cssText = 'font-size:20px;line-height:1.5;'; });
            let metrics = await measure();
            assert.equal(metrics.rows[0], 30, 'paragraph-line geometry follows enlarged text');
            assert.equal(metrics.dataGlyph, 20, 'data icon follows enlarged text');
            await page.locator('.rows').evaluate((node) => { node.style.cssText = '--cem-data-row-height:44px; --cem-data-row-padding-block:3px;'; });
            metrics = await measure();
            assert.equal(metrics.rows[0], 44, 'inherited public row override wins');
            assert.equal(metrics.padding, '3px');
        }
        await page.emulateMedia({ forcedColors: 'active' });
        const forced = await page.locator('.rows').evaluate((node) => ({
            project: getComputedStyle(node).getPropertyValue('--cem-resource-project-color').trim(),
            file: getComputedStyle(node).getPropertyValue('--cem-resource-file-color').trim(),
        }));
        assert.equal(forced.project, 'CanvasText');
        assert.equal(forced.file, 'CanvasText');
        await context.close();
    }
    process.stdout.write('Compact density browser contract passed: five themes, precise/coarse input,1em glyphs, adjacent paragraph lines, enlarged text, inherited row overrides and forced colors. Hybrid exclusion is covered by the generated media contract.\n');
} finally {
    await browser.close();
}
