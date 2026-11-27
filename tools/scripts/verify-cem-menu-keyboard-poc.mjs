import assert from 'node:assert/strict';
import { chromium, firefox } from 'playwright';

const samples = new URL('../../packages/cem-components/src/components/cem-menu/', import.meta.url);
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
                await page.goto(new URL(dir === 'ltr' ? 'css-menu-poc.html' : 'css-menu-poc-rtl.html', samples).href);
                assert.equal(await page.locator('html').getAttribute('dir'), dir);
                assert.equal(await page.locator('[dir]').count(), 1);
                assert.equal(await page.locator('input, label, [role="radio"]').count(), 0);
                const extraControls = page.locator(':is(nav, [role="menu"]) :is(button, a, input, select, textarea, [tabindex]):not(button[role="menuitem"])');
                assert.equal(await extraControls.count(), 0);
                for (const orientation of ['horizontal', 'vertical']) await verifyBar(page, orientation, dir);
                await verifyStandalone(page, dir);
                await verifyTab(page);
                assert.deepEqual(errors, []);
                console.log(engine.name(), browser.version(), dir, 'horizontal, vertical, standalone and Tab passed');
            } finally { await page.close(); }
        }
    } finally { await browser.close(); }
}

function levelSelector(level) {
    return ['horizontal', 'vertical'].includes(level) ? 'nav[aria-orientation="' + level + '"]' : '#' + level;
}
function item(page, level, index) {
    return page.locator(levelSelector(level) + ' > button').nth(index);
}
async function state(page, level, index, panels = []) {
    await page.waitForFunction(({ selector, index, panels }) => {
        const button = document.querySelectorAll(selector + ' > button')[index];
        const open = [...document.querySelectorAll(':popover-open')].map(panel => panel.id);
        const bars = [...document.querySelectorAll('nav')];
        const launchers = [...document.querySelectorAll('button[commandfor]')];
        return button === document.activeElement && JSON.stringify(open) === JSON.stringify(panels)
            && launchers.every(b => b.getAttribute('aria-expanded') === String(b.commandForElement.matches(':popover-open')))
            && bars.every(bar => bar.querySelectorAll(':scope > button[tabindex="0"]').length === 1)
            && button.tabIndex === (button.parentElement.matches('nav') ? 0 : -1);
    }, { selector: levelSelector(level), index, panels }, { timeout: 2500 }).catch(async error => {
        const actual = await page.evaluate(() => [document.activeElement.outerHTML.slice(0, 160),
            [...document.querySelectorAll(':popover-open')].map(panel => panel.id)]);
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
    const positions = await page.locator(levelSelector(root) + ' > button').evaluateAll(items =>
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
}

async function verifyStandalone(page, dir) {
    const launcher = page.locator('button[commandfor="actions"]');
    await launcher.focus(); await key(page, 'ArrowUp'); await state(page, 'actions', 1, ['actions']);
    await key(page, 'Escape');
    await key(page, 'Enter'); await state(page, 'actions', 0, ['actions']);
    await key(page, dir === 'ltr' ? 'ArrowRight' : 'ArrowLeft'); await state(page, 'actions', 0, ['actions']);
    await key(page, 'Escape');
    assert.equal(await launcher.evaluate(button => button === document.activeElement), true);
}

async function verifyTab(page) {
    await page.reload();
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
