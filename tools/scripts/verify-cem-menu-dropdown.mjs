import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { resolve, join, extname } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(import.meta.dirname, '../..');
const temporary = await mkdtemp(join(tmpdir(), 'cem-menu-dropdown-'));
const mime = { '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.svg': 'image/svg+xml', '.wasm': 'application/wasm' };
const server = createServer(async (request, response) => {
    try {
        const pathname = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
        const path = pathname.startsWith('/installed/') ? join(temporary, pathname) : join(root, pathname);
        response.setHeader('Content-Type', mime[extname(path)] ?? 'application/octet-stream');
        response.end(await readFile(path));
    } catch { response.writeHead(404); response.end(); }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
try {
    for (const [folder, name] of [['cem-components', 'cem-components'], ['cem-elements', 'cem-elements'], ['cem-demo-element', 'cem-demo-element'], ['cem-theme', 'cem-theme'], ['cem-ml-npm', 'cem-ml']]) {
        const packed = JSON.parse(execFileSync('npm', ['pack', '--json', '--pack-destination', temporary], { cwd: join(root, 'packages', folder), encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } }));
        const target = join(temporary, 'installed/node_modules/@epa-wg', name);
        await mkdir(target, { recursive: true });
        execFileSync('tar', ['-xzf', join(temporary, packed[0].filename), '--strip-components=1', '-C', target]);
    }
    for (const base of ['/packages/cem-components/playgrounds/', '/packages/cem-components/dist/', '/installed/node_modules/@epa-wg/cem-components/dist/']) {
        for (const tag of ['cem-menu', 'cem-dropdown']) {
            const page = await browser.newPage();
            const errors = [];
            page.on('pageerror', error => errors.push(error.message));
            await page.goto(origin + base + tag + '.html');
            await page.getByRole('link', { name: 'Full examples and variation matrix', exact: true }).waitFor();
            await page.waitForFunction(tag => document.querySelector(`main ${tag} [part]`), tag);
            await page.getByRole('link', { name: 'Full examples and variation matrix', exact: true }).click();
            await page.waitForFunction(() => [...document.querySelectorAll('cem-demo-element')].every(demo => demo.getAttribute('data-state') === 'ready') && document.querySelectorAll('cem-demo-element').length > 0);
            assert.equal(await page.locator('[data-gallery-theme]').count(), 5);
            for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
                const sample = page.locator(`[data-gallery-theme="${mode}"] > ${tag}`).first();
                const control = sample.locator(tag === 'cem-menu' ? ':scope > [part="composite"] > cem-menu-item > button' : ':scope > [part="base"] > button').first();
                await control.scrollIntoViewIfNeeded();
                const colors = await control.evaluate(node => {
                    const probe = document.createElement('span');
                    node.append(probe);
                    probe.style.backgroundColor = 'var(--cem-action-contextual-default-background)';
                    probe.style.color = 'var(--cem-action-contextual-default-text)';
                    const actual = getComputedStyle(node); const expected = getComputedStyle(probe);
                    const result = [actual.backgroundColor, expected.backgroundColor, actual.color, expected.color];
                    probe.remove(); return result;
                });
                assert.equal(colors[0], colors[1], `${tag}/${mode}: background`);
                assert.equal(colors[2], colors[3], `${tag}/${mode}: text`);
                await control.focus(); await page.keyboard.press('ArrowDown');
                assert.equal(await control.getAttribute('aria-expanded'), 'true');
                const panel = page.locator('#' + await control.getAttribute('aria-controls'));
                const bounds = await panel.evaluate(node => { const box = node.getBoundingClientRect(); return { left: box.left, top: box.top, right: box.right, bottom: box.bottom, width: innerWidth, height: innerHeight }; });
                assert(bounds.left >= 0 && bounds.top >= 0 && bounds.right <= bounds.width + 1 && bounds.bottom <= bounds.height + 1, `${tag}/${mode}: popup stays in viewport`);
                if (tag === 'cem-menu') assert.equal(await control.evaluate(node => getComputedStyle(node, '::after').content), '"▴"');
                await page.keyboard.press('Escape');
                assert.equal(await control.getAttribute('aria-expanded'), 'false');
                assert.equal(await control.evaluate(node => document.activeElement === node), true);
                await page.emulateMedia({ forcedColors: 'active' });
                await page.keyboard.press('Tab'); await control.focus();
                assert.equal(await control.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
                assert.notEqual(await control.evaluate(node => getComputedStyle(node).outlineWidth), '0px');
                await page.emulateMedia({ forcedColors: 'none' });
            }
            assert.deepEqual(errors, []);
            await page.close();
            console.log(`${tag}: source/package gallery, five themes and forced colors passed (${base}).`);
        }
    }
} finally {
    await browser.close();
    await new Promise(resolve => server.close(resolve));
    await rm(temporary, { recursive: true, force: true });
}
