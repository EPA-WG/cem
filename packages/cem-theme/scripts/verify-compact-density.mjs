import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { chromium } from 'playwright';

const css = await readFile(new URL('../dist/lib/css/cem-combined.css', import.meta.url), 'utf8');
// Chromium ignores pointer/hover CDP media overrides. Blink input enums provide
// real hybrid and no-pointer fixtures; matchMedia assertions below verify them.
const inputs = [
    { mode: 'fine', hasTouch: false, isMobile: false, args: [] },
    { mode: 'coarse', hasTouch: true, isMobile: true, args: [] },
    { mode: 'hybrid', hasTouch: false, isMobile: false, args: ['--blink-settings=primaryHoverType=2,availableHoverTypes=2,primaryPointerType=4,availablePointerTypes=6'] },
    { mode: 'unknown', hasTouch: false, isMobile: false, args: ['--blink-settings=primaryHoverType=1,availableHoverTypes=1,primaryPointerType=1,availablePointerTypes=1'] },
];
let browser;
try {
    for (const input of inputs) {
        browser = await chromium.launch({ headless: true, args: input.args });
        const context = await browser.newContext({ hasTouch: input.hasTouch, isMobile: input.isMobile, viewport: { width: 1200, height: 800 } });
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
        .application-bar, .application-actions { display:flex; align-items:center; }
        .application-control { display:inline-flex; align-items:center; justify-content:center; box-sizing:border-box; font:inherit; min-inline-size:var(--cem-coupling-zone-min); min-block-size:max(var(--cem-coupling-zone-min),var(--cem-shape-height,var(--cem-control-height))); padding-inline:var(--cem-control-padding-x); padding-block:var(--cem-control-padding-y); border:max(0px,calc((var(--cem-coupling-zone-min) - var(--cem-shape-height,var(--cem-control-height))) / 2)) solid transparent; }
        .button-glyph { display:inline-block; inline-size:var(--cem-icon-button-icon-size); block-size:var(--cem-icon-button-icon-size); }
        .spacing-sample { display:grid; row-gap:var(--cem-gap-related); padding:var(--cem-inset-container); }
        </style><section data-cem-density="compact"><button class="chrome">Build <span class="glyph">◇</span></button>
        <section class="rows" data-cem-density="super-compact"><button class="row"><span class="data-glyph">◇</span><span>First resource</span></button><button class="row">Second resource</button><span class="project">Project</span><span class="file">Source</span></section></section>
        <section class="application" data-cem-density="super-compact" data-cem-spacing="normal">
          <header class="application-bar"><span>Studio</span><div class="application-actions"><button class="application-control"><span class="button-glyph">◐</span><span>Theme</span></button><button class="application-control icon-only"><span class="button-glyph">◐</span></button></div></header>
          <section class="ordinary spacing-sample" data-cem-density="normal" data-cem-spacing="normal"><button class="application-control"><span class="button-glyph">◐</span><span>Normal action</span></button></section>
          <section class="sparse spacing-sample" data-cem-density="normal" data-cem-spacing="sparse"><button class="application-control">Sparse action</button>
            <section class="spacing-reset spacing-sample" data-cem-spacing="normal"><span>Normal spacing inside sparse</span></section>
            <section class="condensed spacing-sample" data-cem-density="super-compact" data-cem-spacing="normal"><button class="application-control"><span class="button-glyph">◐</span><span>Condensed again</span></button></section>
          </section>
          <button class="application-control explicit-size" data-cem-size="medium">Explicit medium action</button>
        </section>`);
        const measure = () => page.evaluate(() => {
            const chrome = document.querySelector('.chrome');
            const rows = [...document.querySelectorAll('.row')];
            const rowStyle = getComputedStyle(rows[0]);
            return {
                fine: matchMedia('(hover: hover) and (pointer: fine) and (not (any-pointer: coarse))').matches,
                anyCoarse: matchMedia('(any-pointer: coarse)').matches,
                primaryFine: matchMedia('(pointer: fine)').matches,
                primaryCoarse: matchMedia('(pointer: coarse)').matches,
                pointerNone: matchMedia('(pointer: none)').matches,
                anyNone: matchMedia('(any-pointer: none)').matches,
                hover: matchMedia('(hover: hover)').matches,
                chrome: chrome.getBoundingClientRect().height,
                rows: rows.map((row) => row.getBoundingClientRect().height),
                top: rows.map((row) => row.getBoundingClientRect().top),
                line: parseFloat(rowStyle.lineHeight),
                padding: rowStyle.paddingBlockStart,
                glyph: document.querySelector('.glyph').getBoundingClientRect().width,
                dataGlyph: document.querySelector('.data-glyph').getBoundingClientRect().width,
            };
        });
        const measureApplication = () => page.evaluate(() => {
            const application = document.querySelector('.application');
            const control = application.querySelector('.application-actions .application-control');
            const ordinary = application.querySelector('.ordinary .application-control');
            const condensed = application.querySelector('.condensed .application-control');
            const spacing = (selector) => {
                const style = getComputedStyle(application.querySelector(selector));
                return { gap: style.rowGap, inset: style.paddingBlockStart };
            };
            const style = getComputedStyle(control);
            return {
                line: parseFloat(style.lineHeight),
                height: control.getBoundingClientRect().height,
                barHeight: application.querySelector('.application-bar').getBoundingClientRect().height,
                padding: style.paddingBlockStart,
                zone: style.getPropertyValue('--cem-coupling-zone-min').trim(),
                controlMinimum: style.getPropertyValue('--cem-control-height').trim(),
                glyph: control.querySelector('.button-glyph').getBoundingClientRect().width,
                iconOnlyHeight: application.querySelector('.icon-only').getBoundingClientRect().height,
                ordinaryHeight: ordinary.getBoundingClientRect().height,
                ordinaryPadding: getComputedStyle(ordinary).paddingBlockStart,
                ordinaryGlyph: ordinary.querySelector('.button-glyph').getBoundingClientRect().width,
                condensedHeight: condensed.getBoundingClientRect().height,
                condensedPadding: getComputedStyle(condensed).paddingBlockStart,
                condensedGlyph: condensed.querySelector('.button-glyph').getBoundingClientRect().width,
                explicitHeight: application.querySelector('.explicit-size').getBoundingClientRect().height,
                normal: spacing('.ordinary'),
                sparse: spacing('.sparse'),
                reset: spacing('.spacing-reset'),
            };
        });
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            await page.locator('section').first().evaluate((node, mode) => { node.className = `cem-theme-${mode}`; }, theme);
            await page.locator('.application').evaluate((node, mode) => { node.setAttribute('data-theme', `cem-theme-${mode}`); }, theme);
            const metrics = await measure();
            assert.equal(metrics.glyph, 16, `${theme}: chrome glyph follows one text em`);
            assert.equal(metrics.dataGlyph, 16, `${theme}: data glyph follows one text em`);
            if (input.mode === 'fine') {
                assert.equal(metrics.fine, true);
                assert.ok(metrics.chrome >= 24 && metrics.chrome < 48, `${theme}: explicit compact chrome`);
            } else {
                assert.equal(metrics.fine, false);
                assert.ok(metrics.chrome >= 48, `${theme}: compact retains3rem under ${input.mode} input`);
            }
            if (input.mode === 'coarse') {
                assert.equal(metrics.primaryCoarse, true);
                assert.equal(metrics.anyCoarse, true);
            }
            if (input.mode === 'hybrid') {
                assert.equal(metrics.primaryFine, true);
                assert.equal(metrics.hover, true);
                assert.equal(metrics.anyCoarse, true);
            }
            if (input.mode === 'unknown') {
                assert.equal(metrics.pointerNone, true, 'unknown input fixture must actually report pointer:none');
                assert.equal(metrics.anyNone, true, 'unknown input fixture must actually report any-pointer:none');
                assert.equal(metrics.primaryFine, false);
                assert.equal(metrics.primaryCoarse, false);
                assert.equal(metrics.anyCoarse, false);
                assert.equal(metrics.hover, false);
            }
            assert.equal(metrics.rows[0], metrics.line, `${theme}/${input.mode}: explicit super-compact row is one paragraph line`);
            assert.equal(metrics.top[1] - metrics.top[0], metrics.line, `${theme}/${input.mode}: no extra vertical row spacing`);
            assert.equal(metrics.padding, '0px');
            let application = await measureApplication();
            assert.equal(application.glyph, 16, `${theme}: top-level density controls one-em button glyphs`);
            assert.equal(application.ordinaryHeight, 48, `${theme}: nested normal restores the3rem target`);
            assert.equal(application.ordinaryPadding, '8px', `${theme}: nested normal restores canonical control padding`);
            assert.equal(application.ordinaryGlyph, 20, `${theme}: normal resets input-neutral glyph size`);
            assert.equal(application.condensedGlyph, 16, `${theme}: nested super-compact restores one-em glyphs`);
            assert.equal(application.explicitHeight, 48, `${theme}: explicit medium size survives inherited density`);
            assert.deepEqual(application.normal, { gap: '8px', inset: '16px' }, `${theme}: normal density and spacing restore canonical endpoints`);
            assert.deepEqual(application.sparse, { gap: '12px', inset: '24px' }, `${theme}: normal density plus sparse spacing keeps sparse rhythm`);
            assert.deepEqual(application.reset, application.normal, `${theme}: explicit normal spacing resets a sparse ancestor`);
            assert.equal(application.zone, '0px', `${theme}/${input.mode}: explicit super-compact has no coupling height floor`);
            assert.equal(application.controlMinimum, '0px', `${theme}/${input.mode}: content determines generic control height`);
            assert.equal(application.height, application.line, `${theme}/${input.mode}: shared action height rule fits one content line`);
            assert.equal(application.iconOnlyHeight, application.glyph, `${theme}/${input.mode}: icon-only button fits its glyph`);
            assert.equal(application.barHeight, application.line, `${theme}/${input.mode}: app-bar actions do not force a48px header`);
            assert.equal(application.condensedHeight, application.line, `${theme}/${input.mode}: nested super-compact restores content-sized controls`);
            assert.equal(application.padding, '0px');
            assert.equal(application.condensedPadding, '0px');
            await page.locator('.application').evaluate((node) => { node.style.cssText = 'font-size:20px;line-height:1.5;'; });
            application = await measureApplication();
            assert.equal(application.glyph, 20, `${theme}: app chrome glyph follows enlarged text`);
            assert.equal(application.line, 30, `${theme}: app chrome keeps enlarged line height`);
            assert.equal(application.height, 30, `${theme}/${input.mode}: content-sized controls grow with text`);
            assert.equal(application.iconOnlyHeight, 20, `${theme}/${input.mode}: icon-only button grows with its glyph`);
            assert.equal(application.explicitHeight, 48, `${theme}: explicit medium survives text scaling`);
            await page.locator('.application').evaluate((node) => { node.style.cssText = '--cem-control-height:44px;--cem-control-padding-y:3px;'; });
            application = await measureApplication();
            assert.equal(application.height, 44, `${theme}/${input.mode}: local public control geometry overrides inherited density`);
            assert.equal(application.padding, '3px');
            await page.locator('.application').evaluate((node) => { node.style.cssText = '--cem-shape-height:52px;'; });
            application = await measureApplication();
            assert.equal(application.height, 52, `${theme}: optional shape height retains its deliberate override`);
            await page.locator('.application').evaluate((node) => { node.style.cssText = '--cem-control-height-medium:60px;'; });
            application = await measureApplication();
            assert.equal(application.explicitHeight, 60, `${theme}: explicit size tokens retain consumer overrides`);
            await page.locator('.application').evaluate((node) => { node.removeAttribute('style'); });
        }
        {
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
        await browser.close();
        browser = undefined;
    }
    process.stdout.write('Compact density browser contract passed: five themes; verified fine/coarse/hybrid/no-pointer input; gated compact chrome; unconditional content-sized super-compact controls; normal/sparse/nested resets; explicit sizes;1em glyphs; paragraph-line rows; enlarged text; public overrides and forced colors.\n');
} finally {
    await browser?.close();
}
