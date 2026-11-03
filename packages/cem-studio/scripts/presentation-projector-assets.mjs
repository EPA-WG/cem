// Re-export approved source-image DOM snapshots; never rewrite their source text.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';

const input = resolve(workspaceRoot, 'dist/declarative-amsterdam-2026/slide-assets');
const output = resolve(workspaceRoot, 'dist/reports/cem-studio/projector-assets');
const css = await readFile(resolve(workspaceRoot, 'packages/cem-theme/dist/lib/css/cem-combined.css'), 'utf8');
const names = ['markup-cem', 'markup-xml', 'edit-before', 'edit-after', 'validation-before', 'validation-after', 'inspect', 'query-xpath', 'query-cemql'];
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ headless: true });
const assets = [];
try {
    const page = await browser.newPage({ viewport: { width: 2200, height: 1400 }, deviceScaleFactor: 2, colorScheme: 'light' });
    for (const name of names) {
        const source = await readFile(resolve(input, `${name}.html`), 'utf8');
        await page.setContent(source);
        await page.evaluate(() => document.fonts.ready);
        const text = await page.locator('code.cem-source-code').textContent();
        const originalBounds = await page.locator('cem-demo-element').boundingBox();
        const snapshot = source.replace(/<style>[\s\S]*?<\/style>/, `<style>${css}</style>`)
            .replace(/(<body[^>]*data-theme=)"cem-theme-light"/, '$1"cem-theme-projector"')
            .replace('</head>', `<style>
            /* Presentation content styling consumes the theme palette. */
            code.cem-source-code { color: var(--cem-palette-conservative-x) !important; }
            code.cem-source-code [data-role="syntax.punctuation"] { color: var(--cem-palette-conservative-x) !important; }
            code.cem-source-code > kbd, code.cem-source-code [data-role="syntax.function"] { color: var(--cem-palette-enthusiasm) !important; }
            </style></head>`);
        await writeFile(resolve(output, `${name}.html`), snapshot);
        await page.setContent(snapshot);
        await page.evaluate(() => document.fonts.ready);
        assert.equal(await page.locator('code.cem-source-code').textContent(), text, `${name}: source changed`);
        const colors = await page.locator('code.cem-source-code').evaluate(code => {
            const rgb = color => {
                const context = document.createElement('canvas').getContext('2d');
                context.fillStyle = color; context.fillRect(0, 0, 1, 1);
                return [...context.getImageData(0, 0, 1, 1).data].slice(0, 3);
            };
            const luminance = color => color.map(v => v / 255).reduce((sum, v, i) => sum + (v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4) * [.2126, .7152, .0722][i], 0);
            return [...new Set([getComputedStyle(code).color, ...[...code.querySelectorAll('*')].map(el => getComputedStyle(el).color)])].map(color => ({ color, contrast: 1.05 / (luminance(rgb(color)) + .05) }));
        });
        assert.ok(colors.every(pair => pair.contrast >= 7), `${name}: ${JSON.stringify(colors)}`);
        const target = page.locator('cem-demo-element');
        const bounds = await target.boundingBox();
        const original = await readFile(resolve(input, `${name}.png`));
        // Preserve the original pixel frame, including its fractional-layout rounding margin.
        const width = original.readUInt32BE(16), height = original.readUInt32BE(20);
        assert.equal(bounds.width, originalBounds.width, `${name}: width changed`);
        assert.equal(bounds.height, originalBounds.height, `${name}: height changed`);
        const bytes = await page.screenshot({ path: resolve(output, `${name}.png`), clip: { x: bounds.x, y: bounds.y, width: width / 2, height: height / 2 } });
        assert.equal(bytes.readUInt32BE(16), width);
        assert.equal(bytes.readUInt32BE(20), height);
        assets.push({ name, sourceTextSha256: presenterHash(text), snapshotSha256: presenterHash(source), sha256: presenterHash(bytes),
            width: original.readUInt32BE(16), height: original.readUInt32BE(20), colors });
    }
    await writeFile(resolve(output, 'evidence.json'), JSON.stringify({ profile: 'projector', sourceTextPreserved: true, dimensionsPreserved: true, assets }, null, 2) + '\n');
    console.log(`Re-exported ${assets.length} source images, unchanged source text/dimensions, all measured text colors >=7:1.`);
} finally { await browser.close(); }
