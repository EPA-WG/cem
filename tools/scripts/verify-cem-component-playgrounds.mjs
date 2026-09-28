import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { resolve, extname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { chromium } from 'playwright';

const root = resolve(import.meta.dirname, '../..');
const temporary = await mkdtemp(join(tmpdir(), 'cem-action-playground-'));
const mime = { '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.wasm': 'application/wasm' };
let browser;
const server = createServer(async (request, response) => {
    const pathname = new URL(request.url, 'http://localhost').pathname;
    const base = pathname.startsWith('/installed/') ? temporary : root;
    const file = resolve(base, '.' + pathname);
    if (!file.startsWith(base + '/')) { response.writeHead(403).end(); return; }
    try { const body = await readFile(file); response.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' }).end(body); }
    catch { response.writeHead(404).end(); }
});
try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    browser = await chromium.launch({ headless: true });
    await verify(`${origin}/packages/cem-components/playgrounds/cem-action.html`);
    await verifyPendingTheme(`${origin}/packages/cem-theme/dist/lib/css-generators/cem-colors.html`);
    for (const [folder, name] of [['cem-components','cem-components'], ['cem-elements','cem-elements'], ['cem-demo-element','cem-demo-element'], ['cem-theme','cem-theme'], ['cem-ml-npm','cem-ml']]) {
        const output = JSON.parse(execFileSync('npm', ['pack', '--json', '--pack-destination', temporary], { cwd: join(root, 'packages', folder), encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } }));
        const target = join(temporary, 'installed/node_modules/@epa-wg', name);
        await mkdir(target, { recursive: true });
        execFileSync('tar', ['-xzf', join(temporary, output[0].filename), '--strip-components=1', '-C', target]);
    }
    await verify(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-action.html`);
    await verifyPendingTheme(`${origin}/installed/node_modules/@epa-wg/cem-theme/dist/lib/css-generators/cem-colors.html`);
    console.log('Action playground verified from source and isolated package archives.');
} finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
    await rm(temporary, { recursive: true, force: true });
}

async function verify(url) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
    try {
        await page.goto(url);
        await page.locator('#action-preview button').waitFor();
        await page.waitForFunction(() => document.querySelector('cem-demo-element')?.getAttribute('data-state') === 'ready');
        const button = page.locator('#action-preview button');
        assert.equal(await button.innerText(), 'Save changes');
        await page.evaluate(() => { window.originalPreview = document.querySelector('#action-preview button'); });
        const choose = async (label, option) => {
            const group = page.getByRole('radiogroup', { name: label, exact: true });
            const radio = group.getByRole('radio', { name: option, exact: true });
            await radio.click({ delay: 180 });
            await page.waitForFunction(({ label, option }) => {
                const group = [...document.querySelectorAll('[role=radiogroup]')].find(node => node.querySelector('legend')?.textContent.trim() === label);
                const checked = group?.querySelectorAll('input:checked');
                return checked?.length === 1 && checked[0].value === option;
            }, { label, option: option.toLowerCase() });
        };
        assert.equal(await page.getByRole('combobox').count(), 0);
        assert.equal(await page.getByRole('radiogroup').count(), 6);
        assert.equal(await page.getByRole('radio').count(), 18);
        for (const [label, value] of [['Intent', 'Primary'], ['Bend', 'Smooth'], ['Button type', 'Button'], ['Disabled', 'False'], ['Loading', 'False'], ['Expanded', 'Unset']]) {
            const group = page.getByRole('radiogroup', { name: label, exact: true });
            assert(await group.getByRole('radio', { name: value, exact: true }).isChecked());
            for (const radio of await group.getByRole('radio').all()) assert(await radio.isVisible());
        }
        for (const variant of ['Explicit', 'Contextual', 'Alternate', 'Destructive', 'Primary']) {
            await choose('Intent', variant);
            await page.waitForFunction(variant => document.querySelector('#action-preview')?.getAttribute('variant') === variant, variant.toLowerCase());
        }
        const primary = page.getByRole('radiogroup', { name: 'Intent', exact: true }).getByRole('radio', { name: 'Primary', exact: true });
        await primary.focus();
        await page.keyboard.press('ArrowRight');
        await page.waitForFunction(() => document.querySelector('#action-preview')?.getAttribute('variant') === 'explicit');
        assert(await page.getByRole('radio', { name: 'Explicit', exact: true }).isChecked());
        await page.keyboard.press('ArrowLeft');
        await page.waitForFunction(() => document.querySelector('#action-preview')?.getAttribute('variant') === 'primary');
        await choose('Bend', 'Round');
        await page.waitForFunction(() => document.querySelector('#action-preview')?.classList.contains('cem-bend-round'));
        const label = page.getByRole('textbox', { name: 'Label', exact: true });
        await label.fill('Publish now');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.textContent.trim() === 'Publish now');
        for (const type of ['Submit','Reset','Button']) {
            await choose('Button type', type);
            await page.waitForFunction(type => document.querySelector('#action-preview button')?.type === type, type.toLowerCase());
        }
        await choose('Disabled', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === true);
        await choose('Disabled', 'False');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === false);
        await button.evaluate(node => {
            window.loadingTransitions = [];
            node.addEventListener('animationstart', event => window.loadingTransitions.push(event.animationName));
        });
        await choose('Loading', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'true');
        await page.waitForFunction(() => window.loadingTransitions.some(name => name.startsWith('cem-pending-shift-')));
        const movingPosition = await button.evaluate(node => getComputedStyle(node).backgroundPositionX);
        await page.waitForFunction(position => getComputedStyle(document.querySelector('#action-preview button')).backgroundPositionX !== position, movingPosition);
        assert.equal(await button.evaluate(node => node.getAnimations()[0].effect.getTiming().iterations), Infinity);
        assert.equal(await button.evaluate(node => node.getAnimations()[0].effect.getTiming().duration), 2000);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        const pendingPaint = await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]);
        await choose('Disabled', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === true);
        assert.deepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        const disabledPosition = await button.evaluate(node => getComputedStyle(node).backgroundPositionX);
        await page.waitForFunction(position => getComputedStyle(document.querySelector('#action-preview button')).backgroundPositionX !== position, disabledPosition);
        await choose('Loading', 'False');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'false');
        assert.notDeepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        await page.emulateMedia({ reducedMotion: 'reduce' });
        await choose('Loading', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'true');
        assert.deepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        assert.equal(await button.evaluate(node => node.getAnimations().length), 0);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await button.evaluate(node => getComputedStyle(node).backgroundImage), 'none');
        assert.equal(await button.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        assert.equal(await button.evaluate(node => node.getAnimations().length), 0);
        await page.emulateMedia({ reducedMotion: 'no-preference', forcedColors: 'none' });
        await choose('Disabled', 'False');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === false);
        for (const value of ['True','False','Unset']) {
            await choose('Expanded', value);
            await page.waitForFunction(value => document.querySelector('#action-preview button')?.getAttribute('aria-expanded') === value, value === 'Unset' ? null : value.toLowerCase());
        }
        for (const [label, value] of [['Intent', 'Primary'], ['Bend', 'Round'], ['Button type', 'Button'], ['Disabled', 'False'], ['Loading', 'True'], ['Expanded', 'Unset']]) {
            const group = page.getByRole('radiogroup', { name: label, exact: true });
            assert(await group.getByRole('radio', { name: value, exact: true }).isChecked());
            assert.equal(await group.locator('input:checked').count(), 1);
        }
        await button.hover();
        assert(await button.evaluate(node => node.matches(':hover')));
        await page.mouse.down();
        assert(await button.evaluate(node => node.matches(':active')));
        await page.mouse.up();
        await button.focus();
        await page.keyboard.press('Shift+Tab');
        await page.keyboard.press('Tab');
        assert(await button.evaluate(node => node.matches(':focus-visible')));
        assert.notEqual(await button.evaluate(node => getComputedStyle(node).boxShadow), 'none');
        assert(await page.evaluate(() => window.originalPreview === document.querySelector('#action-preview button')));
        assert.equal(await page.locator('cem-action').count(), 1);
        assert.equal(await page.locator('cem-demo-element cem-action, cem-demo-element cem-element').count(), 0);
        assert((await page.locator('cem-demo-element').innerText()).includes('id="cem-action"'));
        assert.equal(await page.locator('cem-element[tag="cem-action"]').count(), 1);
        const canonical = await page.getByRole('link', { name: 'Open canonical XHTML' }).getAttribute('href');
        const response = await page.request.get(new URL(canonical, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components/cem-action/cem-action.xhtml'), 'utf8'));
        assert.equal(await page.getByRole('link', { name: 'Automated stories' }).count(), 1);
        assert.deepEqual(errors, []);
        const gallery = await page.getByRole('link', { name: 'Full examples and variation matrix (legacy gallery)' }).getAttribute('href');
        await page.route(/https:\/\/(fonts\.googleapis\.com|fonts\.gstatic\.com|use\.fontawesome\.com)\//, route => route.fulfill({ contentType: 'text/css', body: '' }));
        await page.goto(gallery);
        await page.waitForFunction(() => document.querySelectorAll('cem-demo-element [slot=demo] button').length === 52);
        assert.equal(await page.locator('cem-demo-element').count(), 9);
        const returnLink = page.getByRole('link', { name: 'Action property playground and source' });
        await returnLink.waitFor();
        assert.equal(new URL(await returnLink.getAttribute('href'), page.url()).href, url);
        assert.deepEqual(errors, []);
    } catch (error) {
        console.error(url, errors, await page.locator('body').innerText());
        throw error;
    } finally { await page.close(); }
}


async function verifyPendingTheme(url) {
    const page = await browser.newPage();
    try {
        await page.goto(url);
        await page.locator('td.cem-pending').first().waitFor();
        const cells = page.locator('td.cem-pending');
        assert.equal(await cells.count(), 25);
        const styles = await cells.evaluateAll(nodes => nodes.map(node => {
            const style = getComputedStyle(node);
            return { image: style.backgroundImage, duration: style.animationDuration, iterations: style.animationIterationCount };
        }));
        for (const style of styles) {
            assert.match(style.image, /linear-gradient\(45deg/);
            assert.equal(style.duration, '2s');
            assert.equal(style.iterations, 'infinite');
        }
        const contrastRatios = await cells.evaluateAll(nodes => {
            const context = document.createElement('canvas').getContext('2d');
            const luminance = color => {
                context.clearRect(0, 0, 1, 1);
                context.fillStyle = color;
                context.fillRect(0, 0, 1, 1);
                const rgb = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(value => {
                    const channel = value / 255;
                    return channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
                });
                return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
            };
            return nodes.flatMap(node => {
                const probe = document.createElement('span');
                node.append(probe);
                const foreground = luminance(getComputedStyle(node).color);
                const ratios = [1, 2].map(index => {
                    probe.style.backgroundColor = `var(--cem-pending-color-${index})`;
                    const background = luminance(getComputedStyle(probe).backgroundColor);
                    return (Math.max(background, foreground) + 0.05) / (Math.min(background, foreground) + 0.05);
                });
                probe.remove();
                return ratios;
            });
        });
        for (const ratio of contrastRatios) assert(ratio >= 4.5, `Pending text contrast ${ratio}`);
        const cell = cells.first();
        const position = await cell.evaluate(node => getComputedStyle(node).backgroundPositionX);
        await page.waitForFunction(position => getComputedStyle(document.querySelector('td.cem-pending')).backgroundPositionX !== position, position);
        const cycle = await cell.evaluate(node => {
            const animation = node.getAnimations()[0];
            animation.pause();
            animation.currentTime = 500;
            const first = getComputedStyle(node).backgroundPositionX;
            animation.currentTime = 2500;
            const second = getComputedStyle(node).backgroundPositionX;
            animation.play();
            return [first, second];
        });
        assert.equal(cycle[0], cycle[1]);
        await page.emulateMedia({ reducedMotion: 'reduce' });
        assert.equal(await cell.evaluate(node => node.getAnimations().length), 0);
        assert.match(await cell.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await cell.evaluate(node => getComputedStyle(node).backgroundImage), 'none');
        assert.equal(await cell.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        console.log('Pending theme gradients verified:', url);
    } finally { await page.close(); }
}
