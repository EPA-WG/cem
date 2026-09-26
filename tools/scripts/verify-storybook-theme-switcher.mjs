import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { resolve, extname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

let server;
let base = process.env.CEM_STORYBOOK_URL ?? 'http://127.0.0.1:4400/';
if (process.argv.includes('--static')) {
    const root = fileURLToPath(new URL('../../packages/cem-elements/storybook-static/', import.meta.url));
    const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
        '.json': 'application/json', '.wasm': 'application/wasm', '.svg': 'image/svg+xml', '.xhtml': 'application/xhtml+xml' };
    server = createServer(async (request, response) => {
        const pathname = decodeURIComponent(new URL(request.url, 'http://local').pathname);
        if (!pathname.startsWith('/nested/storybook/')) { response.writeHead(404); response.end(); return; }
        const file = resolve(root, pathname.slice('/nested/storybook/'.length) || 'index.html');
        if (!file.startsWith(resolve(root) + sep)) { response.writeHead(403); response.end(); return; }
        try { const body = await readFile(file); response.setHeader('content-type', types[extname(file)] ?? 'application/octet-stream'); response.end(body); }
        catch { response.writeHead(404); response.end(); }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    base = `http://127.0.0.1:${server.address().port}/nested/storybook/`;
}
const browser = await chromium.launch({ headless: true });
const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
const errors = [];
page.on('pageerror', error => errors.push(error.message));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
const modes = ['native', 'light', 'dark', 'contrast-light', 'contrast-dark'];
const labels = ['Native', 'Light', 'Dark', 'Contrast light', 'Contrast dark'];
const results = [];
try {
    await page.goto(new URL('?path=/story/cem-components-cem-select--default', base).href);
    const tool = page.locator('[data-cem-theme-tool]');
    const control = tool.getByRole('combobox', { name: 'Theme' });
    const preview = page.frameLocator('#storybook-preview-iframe');
    await preview.getByRole('combobox', { name: 'Role' }).waitFor();
    await page.waitForFunction(() => document.querySelector('#storybook-preview-iframe')?.contentWindow?.__STORYBOOK_PREVIEW__?.storyRenders?.every(render => render.phase === 'finished'));
    await page.getByRole('button', { name: 'Theme', exact: true }).focus();
    await page.getByRole('button', { name: 'Theme', exact: true }).press('Enter');
    await control.waitFor();
    await page.getByRole('dialog', { name: 'Theme', exact: true }).waitFor();
    for (const [index, mode] of modes.entries()) {
        await control.click();
        await tool.getByRole('option').first().waitFor();
        assert.deepEqual((await tool.getByRole('option').allTextContents()).map(text => text.trim()), labels);
        await tool.getByRole('option', { name: labels[index], exact: true }).click();
        await page.waitForFunction(mode => document.querySelector('#storybook-preview-iframe')?.contentDocument
            ?.body.dataset.theme === `cem-theme-${mode}`, mode);
        assert.equal(await tool.locator('cem-select').evaluate(el => el.value), mode);
        await page.waitForFunction(() => document.querySelector('[data-cem-theme-tool] [role=combobox]')?.getAttribute('aria-expanded') === 'false');
        const tokens = await preview.getByRole('combobox', { name: 'Role' }).evaluate(el => {
            const style = getComputedStyle(el);
            const probe = document.createElement('span');
            probe.style.cssText = 'background:var(--cem-select-popup-background);color:var(--cem-select-popup-text)';
            el.parentElement.append(probe);
            const expected = getComputedStyle(probe);
            const value = { background: style.backgroundColor, text: style.color,
                expectedBackground: expected.backgroundColor, expectedText: expected.color };
            probe.remove();
            return value;
        });
        assert.equal(tokens.background, tokens.expectedBackground);
        assert.equal(tokens.text, tokens.expectedText);
        assert.notEqual(tokens.background, 'rgba(0, 0, 0, 0)');
        const managerColors = await control.evaluate(el => ({
            background: getComputedStyle(el).backgroundColor, text: getComputedStyle(el).color,
        }));
        assert.equal(managerColors.background, tokens.background);
        assert.equal(managerColors.text, tokens.text);
        results.push({ mode, ...tokens });
    }
    assert.notEqual(results[1].background, results[2].background, 'light and dark resolve differently');
    assert.notEqual(results[3].background, results[4].background, 'contrast light and dark resolve differently');
    await control.focus();
    await control.press('Home');
    await control.press('Enter');
    await page.waitForFunction(() => document.querySelector('[data-cem-theme-tool] cem-select')?.value === 'native');
    assert.equal(await control.evaluate(el => el === document.activeElement), true);
    assert.equal(await preview.locator('[data-cem-theme-tool]').count(), 0);
    // Exercise Storybook's global channel as an external controller, not a second store.
    for (const [value, expected] of [['dark', 'dark'], ['unrecognized', 'native']]) {
        await page.evaluate(value => window.__STORYBOOK_ADDONS_CHANNEL__.emit('updateGlobals', { globals: { cemTheme: value } }), value);
        await page.waitForFunction(mode => document.querySelector('[data-cem-theme-tool] cem-select')?.value === mode
            && document.querySelector('#storybook-preview-iframe')?.contentDocument?.body.dataset.theme === `cem-theme-${mode}`, expected);
    }
    await page.evaluate(() => window.__STORYBOOK_ADDONS_CHANNEL__.emit('updateGlobals', { globals: { cemTheme: 'dark' } }));
    await page.waitForFunction(() => document.querySelector('[data-cem-theme-tool] cem-select')?.value === 'dark');
    await control.press('Escape');
    await page.waitForFunction(() => document.activeElement === document.querySelector('button[title=Theme]'));
    await page.getByRole('link', { name: 'Stylesheet Ownership', exact: true }).click();
    await page.waitForFunction(() => document.querySelector('#storybook-preview-iframe')?.contentDocument?.body.dataset.theme === 'cem-theme-dark');
    await page.getByRole('button', { name: 'Theme', exact: true }).click();
    await control.waitFor();
    assert.equal(await tool.locator('cem-select').evaluate(el => el.value), 'dark');
    assert.equal(await page.locator('cem-element[data-cem-storybook-declaration="manager-theme-select"]').count(), 1);
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ modes: results, errors }, null, 2));
} finally { await browser.close(); if (server) await new Promise(resolve => server.close(resolve)); }
