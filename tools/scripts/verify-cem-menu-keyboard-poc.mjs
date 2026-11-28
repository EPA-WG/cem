import assert from 'node:assert/strict';
import { chromium, firefox } from 'playwright';

import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const component = process.argv.includes('--component');
const repo = fileURLToPath(new URL('../../', import.meta.url));
const server = createServer(async (request, response) => {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    const file = path.resolve(repo, '.' + decodeURIComponent(pathname));
    if (!file.startsWith(repo)) { response.writeHead(403).end(); return; }
    try {
        const mime = { '.js': 'text/javascript', '.xhtml': 'application/xhtml+xml', '.html': 'text/html',
            '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json' }[path.extname(file)];
        const bytes = await readFile(file);
        response.writeHead(200, { 'Content-Type': mime ?? 'application/octet-stream' }).end(bytes);
    } catch { response.writeHead(404).end(); }
});
if (component) await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));

const samples = new URL('../../packages/cem-components/src/components/cem-menu/', import.meta.url);
try {
    for (const engine of [chromium, firefox]) {
        const browser = await engine.launch();
        try {
            for (const dir of ['ltr', 'rtl']) {
                const page = await browser.newPage();
                try {
                    const errors = [];
                    page.on('pageerror', error => errors.push(error.message));
                    await page.addInitScript(() => window.addEventListener('DOMContentLoaded', () => {
                        window.activations = [];
                        document.addEventListener('click', event => {
                            if (!event.defaultPrevented && event.target.matches('button')) {
                                window.activations.push([event.target.textContent, event.isTrusted]);
                            }
                        });
                        document.addEventListener('keydown', event => { window.consumed = event.defaultPrevented; });
                    }));
                    const suffix = dir === 'rtl' ? '-rtl' : '';
                    await page.goto(component ? `http://127.0.0.1:${server.address().port}/packages/cem-components/playgrounds/cem-menu-keynav${suffix}.html`
                        : new URL('css-menu-poc' + suffix + '.html', samples).href);
                    await ready(page);
                    assert.equal(await page.locator('html').getAttribute('dir'), dir);
                    assert.equal(await page.locator('[dir]').count(), 1);
                    assert.equal(await page.locator('input, label, [role="radio"]').count(), 0);
                    const extraControls = page.locator(':is(nav, [role="menu"]) :is(button, a, input, select, textarea, [tabindex]):not(button[role="menuitem"]):not([part=composite][tabindex="-1"])');
                    assert.equal(await extraControls.count(), 0);
                    for (const orientation of ['horizontal', 'vertical']) await verifyBar(page, orientation, dir);
                    await verifyStandalone(page, dir);
                    await verifyTab(page);
                    if (component) await verifyForm(page, dir);
                    assert.deepEqual(errors, []);
                    console.log(component ? 'cem-menu' : 'POC', engine.name(), browser.version(), dir, component ? 'menus, native activation, Tab and inherited form navigation passed' : 'horizontal, vertical, standalone and Tab passed');
                } finally { await page.close(); }
            }
        } finally { await browser.close(); }
    }

} finally { if (server.listening) await new Promise(resolve => server.close(resolve)); }

function levelSelector(level) {
    if (['horizontal', 'vertical'].includes(level)) return '[role="menubar"][aria-orientation="' + level + '"]';
    if (!component) return '#' + level;
    return '#' + level + (level === 'actions' ? ' cem-menu' : '') + ' > [part=composite]';
}
function itemSelector(level) {
    const owner = levelSelector(level);
    return component ? owner + ' > cem-menu-item > button' : owner + ' > button';
}
function item(page, level, index) { return page.locator(itemSelector(level)).nth(index); }

async function ready(page) {
    if (component) {
        const html = await readFile(path.join(repo, new URL(page.url()).pathname.slice(1)), 'utf8');
        const expected = [...html.matchAll(/<cem-menu-item\s/g)].length;
        await page.waitForFunction(count => document.querySelectorAll('button[role=menuitem]').length === count && document.querySelectorAll('[role=menubar]').length === 2 &&
            !!document.querySelector('#actions [part=base] button')?.popoverTargetElement, expected);
        await page.evaluate(async () => {
            const hosts = document.querySelectorAll('cem-menu,cem-menu-item,cem-dropdown');
            await Promise.all([...hosts].map(host => window.cemPlaygroundRuntime.whenRenderSettled(host)));
        });
    }
}
async function state(page, level, index, panels = []) {
    await page.waitForFunction(({ selector, index, panels }) => {
        const button = document.querySelectorAll(selector)[index];
        const open = [...document.querySelectorAll(':popover-open')].map(panel =>
            panel.matches('[part=popup]') ? panel.closest('cem-dropdown').id : panel.id);
        const bars = [...document.querySelectorAll('[role=menubar]')];
        const launchers = [...document.querySelectorAll('button[commandfor],button[popovertarget]')];
        return button === document.activeElement && JSON.stringify(open) === JSON.stringify(panels)
            && launchers.every(b => b.getAttribute('aria-expanded') === String((b.commandForElement ?? b.popoverTargetElement).matches(':popover-open')))
            && bars.every(bar => bar.querySelectorAll(':scope > button[tabindex="0"],:scope > cem-menu-item > button[tabindex="0"]').length === 1)
            && button.tabIndex === (button.closest('[role=menu],[role=menubar]').matches('[role=menubar]') ? 0 : -1);
    }, { selector: itemSelector(level), index, panels }, { timeout: 4000 }).catch(async error => {
        const actual = await page.evaluate(() => ({
            focus: [document.activeElement.localName, document.activeElement.textContent?.trim().slice(0, 65), document.activeElement.tabIndex],
            open: [...document.querySelectorAll(':popover-open')].map(panel => panel.id),
            expanded: [...document.querySelectorAll('button[aria-expanded=true]')].map(button => button.textContent),
            stops: [...document.querySelectorAll('[role=menuitem][tabindex="0"]')].map(button => button.textContent),
        }));
        throw Error('Expected ' + JSON.stringify({ level, index, panels }) + ', observed ' + JSON.stringify(actual), { cause: error });
    });
}
async function key(page, value, consumed = value.startsWith('Arrow')) {
    await page.keyboard.press(value);
    assert.equal(await page.evaluate(() => window.consumed), consumed, value + ' default');
}
function clicks(page) { return page.evaluate(() => window.activations); }

async function verifyBar(page, root, dir) {
    const row = root === 'horizontal';
    const forward = dir === 'ltr' ? 'ArrowRight' : 'ArrowLeft';
    const back = dir === 'ltr' ? 'ArrowLeft' : 'ArrowRight';
    const [next, prev, toward, away] = row ? [forward, back, 'ArrowDown', 'ArrowUp'] : ['ArrowDown', 'ArrowUp', forward, back];
    const file = root + '-file', exp = root + '-export', edit = root + '-edit';
    const positions = await page.locator(itemSelector(root)).evaluateAll(items =>
        items.map(item => { const { x, y } = item.getBoundingClientRect(); return { x, y }; }));
    if (row) assert.ok(dir === 'ltr' ? positions[0].x < positions[1].x : positions[0].x > positions[1].x);
    else assert.ok(positions[0].y < positions[1].y);
    const initial = (await clicks(page)).length;
    await item(page, root, 0).focus();
    await key(page, next); await state(page, root, 1);
    await key(page, toward); await state(page, root, 2); // Cross-axis leaf fallback lands on closed Edit.
    await key(page, prev); await state(page, root, 1);
    await key(page, away); await state(page, root, 0);
    await key(page, away); await state(page, file, 2, [file]);
    await key(page, 'Escape'); await state(page, root, 0);
    await key(page, toward); await state(page, file, 0, [file]);
    await key(page, 'ArrowDown'); await state(page, file, 1, [file]);
    await key(page, forward); await state(page, exp, 0, [file, exp]);
    await key(page, 'ArrowUp'); await state(page, exp, 1, [file, exp]);
    await key(page, back); await state(page, file, 1, [file]);
    await key(page, back); await state(page, root, 4, [root + '-view']);
    await key(page, next); await state(page, root, 0, [file]); // Wrap with a branch open.
    await key(page, prev); await state(page, root, 4, [root + '-view']);
    await key(page, prev); await state(page, root, 3);
    await key(page, toward); await state(page, root, 3); // Disabled but focusable.
    await key(page, 'Enter', true); await state(page, root, 3);
    await key(page, 'Space', true); await state(page, root, 3);
    const disabled = await item(page, root, 3).boundingBox();
    await page.mouse.click(disabled.x + disabled.width / 2, disabled.y + disabled.height / 2);
    await state(page, root, 3);
    await key(page, next); await key(page, next); await state(page, root, 0);
    await key(page, toward); await key(page, forward); await state(page, root, 1);
    await key(page, toward); await key(page, toward); await state(page, edit, 0, [edit]);
    await key(page, row ? 'ArrowDown' : forward); await state(page, edit, 1, [edit]);
    await key(page, row ? forward : 'ArrowDown'); await state(page, root, 3);
    assert.equal((await clicks(page)).length, initial, 'all arrow routes and refusals delivered zero activations');

    const fileButton = item(page, root, 0), exportButton = item(page, file, 1), leaf = item(page, exp, 0);
    await fileButton.focus(); await key(page, 'Enter'); await state(page, file, 0, [file]);
    await key(page, 'ArrowDown'); await key(page, 'Space'); await state(page, exp, 0, [file, exp]);
    await key(page, 'Enter'); await state(page, root, 0);
    await fileButton.click(); await state(page, file, 0, [file]);
    await exportButton.click(); await state(page, exp, 0, [file, exp]);
    await key(page, 'Escape'); await state(page, file, 1, [file]);
    await key(page, 'Escape'); await state(page, root, 0);
    const help = item(page, root, 1);
    await key(page, next); await key(page, 'Space'); await state(page, root, 1);
    await help.click();
    const names = await Promise.all([fileButton, exportButton, leaf, fileButton, exportButton, help, help].map(b => b.textContent()));
    assert.deepEqual((await clicks(page)).slice(initial), names.map(name => [name, true]), 'one trusted click per activation');
    assert.equal(await page.locator('output').textContent(), await help.textContent());

    // Activate an open branch preview with its launcher still focused.
    await fileButton.focus(); await key(page, toward); await key(page, back);
    await state(page, root, 4, [root + '-view']);
    await key(page, 'Enter'); await state(page, root + '-view', 0, [root + '-view']);
    await key(page, 'Escape'); await state(page, root, 4);
}

async function verifyStandalone(page, dir) {
    const launcher = page.locator(component ? '#actions [part=base] button' : 'button[commandfor="actions"]');
    await launcher.focus(); await key(page, 'ArrowUp'); await state(page, 'actions', 1, ['actions']);
    await key(page, 'Escape');
    await key(page, 'Enter'); await state(page, 'actions', 0, ['actions']);
    await key(page, dir === 'ltr' ? 'ArrowRight' : 'ArrowLeft'); await state(page, 'actions', 0, ['actions']);
    await key(page, 'Escape');
    assert.equal(await launcher.evaluate(button => button === document.activeElement), true);
}

async function verifyTab(page) {
    await page.reload();
    await ready(page);
    await key(page, 'Tab'); await state(page, 'horizontal', 0);
    await key(page, 'ArrowDown'); await key(page, 'ArrowDown');
    await key(page, await page.locator('html').getAttribute('dir') === 'ltr' ? 'ArrowRight' : 'ArrowLeft');
    await state(page, 'horizontal-export', 0, ['horizontal-file', 'horizontal-export']);
    await key(page, 'Tab'); await state(page, 'vertical', 0);
    await key(page, 'ArrowDown'); await key(page, 'ArrowDown'); await state(page, 'vertical', 2);
    await key(page, 'Enter'); await state(page, 'vertical-edit', 0, ['vertical-edit']);
    await key(page, 'Shift+Tab'); await state(page, 'horizontal', 0);
    await key(page, 'Tab'); await state(page, 'vertical', 2); // Re-entry remembers Edit.
}

async function verifyForm(page, dir) {
    await page.evaluate(async () => {
        const declaration = document.createElement('cem-element');
        declaration.setAttribute('tag', 'file-list-tools');
        declaration.setAttribute('keynav', '');
        declaration.innerHTML = `<template><form><button type="button">Preview</button><button type="button">Print</button>
          <fieldset><legend>Filters</legend><input type="text" value="abcd" aria-label="Filter">
          <input type="range" value="50" aria-label="Size"><select aria-label="Type"><option>All</option><option>Documents</option></select>
          <label><input type="radio" name="order" checked>First</label><label><input type="radio" name="order">Last</label>
          <button type="submit">Apply</button><button type="reset">Reset</button>
          <cem-menu><button type="button" role="menuitemradio" aria-checked="true">Name</button>
          <button type="button" role="menuitemradio" aria-checked="false">Date</button></cem-menu></fieldset></form></template>`;
        document.body.append(declaration);
        await window.cemPlaygroundRuntime.whenDeclarationSettled(declaration);
        const region = document.createElement('section');
        region.setAttribute('keynav', 'off');
        region.innerHTML = '<file-list-tools></file-list-tools>';
        document.body.append(region);
        const instance = region.firstElementChild;
        await window.cemPlaygroundRuntime.whenRenderSettled(instance);
        await window.cemPlaygroundRuntime.whenRenderSettled(instance.querySelector('cem-menu'));
        window.formSubmissions = 0;
        region.addEventListener('submit', event => { event.preventDefault(); window.formSubmissions++; });
    });
    const form = page.locator('file-list-tools form');
    const group = form.locator('fieldset'), menu = group.locator('cem-menu');
    const inline = dir === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
    const focused = locator => locator.evaluate(node => node === document.activeElement);
    await form.getByText('Preview').focus(); await key(page, inline);
    assert.ok(await focused(form.getByText('Print')));
    const text = group.locator('input[type=text]');
    await text.focus(); await text.evaluate(node => node.setSelectionRange(2, 2));
    await key(page, 'Shift+ArrowLeft', false);
    assert.equal(await text.evaluate(node => node.selectionEnd - node.selectionStart), 1);
    await key(page, 'Space'); await key(page, 'Enter');
    assert.equal(await page.evaluate(() => window.formSubmissions), 1);
    const range = group.locator('input[type=range]');
    await range.focus(); await key(page, 'ArrowUp', false);
    assert.ok(await focused(range)); assert.equal(await range.inputValue(), '51');
    const select = group.locator('select');
    await select.focus(); await key(page, 'ArrowDown', false); assert.equal(await select.inputValue(), 'Documents');
    const radios = group.locator('input[type=radio]');
    await radios.first().focus(); await key(page, 'ArrowDown', false); assert.ok(await radios.last().isChecked());
    const apply = group.getByText('Apply'), reset = group.getByText('Reset');
    await apply.focus(); await key(page, inline); assert.ok(await focused(reset));
    await key(page, inline); assert.ok(await focused(reset));
    const name = menu.getByText('Name'), date = menu.getByText('Date');
    await name.focus(); await key(page, inline); assert.ok(await focused(date)); assert.equal(await name.getAttribute('aria-checked'), 'true');
    await group.evaluate(node => node.setAttribute('keynav', 'off'));
    await apply.focus(); await key(page, inline, false); assert.ok(await focused(apply));
    await name.focus(); await key(page, inline, false); assert.ok(await focused(name));
    await menu.evaluate(node => node.setAttribute('keynav', 'on'));
    await key(page, inline); assert.ok(await focused(date));
    await group.evaluate(node => node.removeAttribute('keynav')); await menu.evaluate(node => node.removeAttribute('keynav'));
    await apply.focus(); await key(page, inline); assert.ok(await focused(reset));
}
