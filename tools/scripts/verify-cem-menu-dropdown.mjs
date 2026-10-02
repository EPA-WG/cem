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
            if (tag === 'cem-dropdown') {
                for (const width of [1280, 390]) {
                    await page.setViewportSize({ width, height: 844 });
                    const preview = page.locator('main > cem-element:first-child cem-dropdown').first();
                    const trigger = preview.locator(':scope > [part="base"] > button');
                    await trigger.scrollIntoViewIfNeeded();
                    await trigger.focus(); await page.keyboard.press('ArrowDown');
                    const clear = await preview.evaluate(node => {
                        const panel = node.querySelector(':scope > [part="popup"]').getBoundingClientRect();
                        const frame = node.closest('cem-element').getBoundingClientRect();
                        const properties = node.previousElementSibling.getBoundingClientRect();
                        return { fits: panel.top >= properties.bottom - 1 && panel.bottom <= frame.bottom + 1, panel: panel.toJSON(), frame: frame.toJSON(), properties: properties.toJSON() };
                    });
                    assert(clear.fits, 'property preview reserves space between controls and explanatory content: ' + JSON.stringify(clear));
                    await page.keyboard.press('Escape');
                }
                await page.setViewportSize({ width: 1280, height: 720 });
            }
            await page.getByRole('link', { name: 'Full examples and variation matrix', exact: true }).click();
            await page.waitForFunction(() => [...document.querySelectorAll('cem-demo-element')].every(demo => demo.getAttribute('data-state') === 'ready') && document.querySelectorAll('cem-demo-element').length > 0);
            await page.waitForFunction(() => [...document.querySelectorAll('cem-menu-item')].every(item => item.querySelector(':scope > [part~="control"]')) && [...document.querySelectorAll('cem-menu')].every(menu => menu.querySelector(':scope > [part~="composite"]')) && [...document.querySelectorAll('cem-dropdown')].every(dropdown => dropdown.querySelector(':scope > [part~="popup"]')));
            if (tag === 'cem-dropdown') {
                const initial = await page.locator('cem-demo-element > [slot="demo"] cem-dropdown:not([open="false"], [disabled], [hidden])').evaluateAll(nodes => nodes.map(node => {
                    const panel = node.querySelector(':scope > [part="popup"]');
                    const trigger = node.querySelector(':scope > [part="base"]');
                    const region = node.closest('cem-demo-element').querySelector(':scope > [slot="demo"]');
                    const popupBox = panel.getBoundingClientRect(), triggerBox = trigger.getBoundingClientRect(), regionBox = region.getBoundingClientRect();
                    const style = getComputedStyle(panel);
                    return { attached: Math.abs(popupBox.top - triggerBox.bottom) < 1, contained: popupBox.top >= regionBox.top - 1 && popupBox.bottom <= regionBox.bottom + 1, zIndex: style.zIndex, display: style.display, topCorners: [style.borderTopLeftRadius, style.borderTopRightRadius] };
                }));
                assert(initial.length > 0, 'initially open source examples exist');
                for (const result of initial) {
                    assert(result.attached, 'initial panel stays attached below its trigger without viewport clamping');
                    assert(result.contained, 'initial panel remains inside its reserved demo region');
                    assert.equal(result.zIndex, '1'); assert.equal(result.display, 'flex');
                    assert.deepEqual(result.topCorners, ['0px', '0px']);
                }
            }
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
                await page.waitForFunction(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve(true)))));
                assert.equal(await control.getAttribute('aria-expanded'), 'true', `${base}${tag}/${mode}: opening completes`);
                const panel = page.locator('#' + await control.getAttribute('aria-controls'));
                const bounds = await panel.evaluate(node => { const box = node.getBoundingClientRect(); return { left: box.left, top: box.top, right: box.right, bottom: box.bottom, width: innerWidth, height: innerHeight }; });
                assert(bounds.left >= 0 && bounds.top >= 0 && bounds.right <= bounds.width + 1 && bounds.bottom <= bounds.height + 1, `${tag}/${mode}: popup stays in viewport`);
                if (tag === 'cem-dropdown') {
                    const region = await control.evaluate(node => {
                        const box = node.closest('cem-demo-element').querySelector(':scope > [slot="demo"]').getBoundingClientRect();
                        return { left: box.left, top: box.top, right: box.right, bottom: box.bottom };
                    });
                    assert(bounds.left >= region.left - 1 && bounds.top >= region.top - 1 && bounds.right <= region.right + 1 && bounds.bottom <= region.bottom + 1, `${tag}/${mode}: popup stays inside reserved demo space`);
                }
                if (tag === 'cem-dropdown') {
                    const scroll = await control.evaluate(async node => {
                        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
                        const panel = document.getElementById(node.getAttribute('aria-controls'));
                        const spacer = document.createElement('div');
                        spacer.style.height = `${innerHeight}px`;
                        document.body.append(spacer);
                        const before = { trigger: node.getBoundingClientRect().top, panel: panel.getBoundingClientRect().top, scroll: window.scrollY };
                        window.scrollBy(0, node.getBoundingClientRect().bottom + 100);
                        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
                        const after = { trigger: node.getBoundingClientRect().top, panel: panel.getBoundingClientRect().top };
                        window.scrollTo(0, before.scroll);
                        spacer.remove();
                        return { before, after, position: getComputedStyle(panel).position };
                    });
                    assert.equal(scroll.position, 'absolute');
                    assert.equal(await panel.evaluate(node => getComputedStyle(node).zIndex), '1', 'interactive opening retains declaration-owned stacking');
                    assert(scroll.after.trigger < 0, 'scroll fixture moves the trigger above the viewport');
                    assert(Math.abs((scroll.after.panel - scroll.before.panel) - (scroll.after.trigger - scroll.before.trigger)) < 1, 'dropdown follows its trigger past the viewport edge: ' + JSON.stringify(scroll));
                    const nestedScroll = await control.evaluate(async node => {
                        const panel = document.getElementById(node.getAttribute('aria-controls'));
                        const region = node.closest('cem-demo-element').querySelector(':scope > [slot="demo"]');
                        const authoredStyle = region.getAttribute('style');
                        const host = node.closest('cem-dropdown');
                        const hostStyle = host.getAttribute('style');
                        region.style.height = '100px'; region.style.minHeight = '0'; region.style.padding = '0'; region.style.overflow = 'auto';
                        host.style.marginBlock = '200px'; host.style.flexShrink = '0';
                        const before = { trigger: node.getBoundingClientRect().top, panel: panel.getBoundingClientRect().top };
                        region.scrollTop = 40;
                        await new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
                        const after = { trigger: node.getBoundingClientRect().top, panel: panel.getBoundingClientRect().top, scroll: region.scrollTop };
                        region.scrollTop = 0;
                        if (hostStyle === null) host.removeAttribute('style'); else host.setAttribute('style', hostStyle);
                        if (authoredStyle === null) region.removeAttribute('style'); else region.setAttribute('style', authoredStyle);
                        return { before, after };
                    });
                    assert(nestedScroll.after.scroll > 0, 'nested scroll fixture scrolls');
                    assert(Math.abs((nestedScroll.after.panel - nestedScroll.before.panel) - (nestedScroll.after.trigger - nestedScroll.before.trigger)) < 1, 'dropdown follows its trigger inside a scrolling container');
                }
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
            if (tag === 'cem-dropdown') {
                await page.setViewportSize({ width: 390, height: 844 });
                for (const host of await page.locator('cem-demo-element > [slot="demo"] cem-dropdown:not([disabled], [hidden])').all()) {
                    const trigger = host.locator(':scope > [part="base"] > button').first();
                    await trigger.scrollIntoViewIfNeeded();
                    await trigger.focus(); await page.keyboard.press('ArrowDown');
                    const clearance = await host.evaluate(node => {
                        const panel = node.querySelector(':scope > [part="popup"]').getBoundingClientRect();
                        const demo = node.closest('cem-demo-element').querySelector(':scope > [slot="demo"]').getBoundingClientRect();
                        return panel.top >= demo.top - 1 && panel.bottom <= demo.bottom + 1;
                    });
                    assert(clearance, 'narrow dropdown overlay stays in reserved demo space');
                    await page.keyboard.press('Escape');
                }
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
