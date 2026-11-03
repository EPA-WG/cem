import assert from 'node:assert/strict';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { resolve } from 'node:path';
import { chromium } from 'playwright';

const root = resolve(import.meta.dirname, '..');
const css = (await Promise.all(['cem-colors', 'cem-stroke', 'cem-timing'].map(name =>
    readFile(resolve(root, `dist/lib/css/${name}.css`), 'utf8')))).join('\n');
const catalog = JSON.parse(await readFile(resolve(root, 'dist/lib/tokens/cem.tokens.catalog.json'), 'utf8'));
assert.deepEqual(catalog.profiles.projector, { mode: 'light', theme: 'cem-theme-projector' });
const browser = await chromium.launch({ headless: true });
const checks = [];
try {
    const page = await browser.newPage({ colorScheme: 'dark' });
    await page.setContent(`<style>${css}</style><main class="cem-theme-projector">
        <section class="inherited"><span>Retained content</span></section>
        <section class="cem-theme-light"><span>Nested light</span></section>
        <section class="cem-theme-native"><span>Nested native</span></section>
        <section class="cem-theme-contrast-light"><section class="cem-theme-projector">Nested projector</section></section>
        </main><aside data-theme="cem-theme-projector">Attribute selection</aside>
        <aside data-theme="cem-theme-light" data-cem-presentation="projector">Light profile</aside>`);
    const read = async (selector, names) => page.locator(selector).evaluate((el, names) => {
        const result = { scheme: getComputedStyle(el).colorScheme };
        for (const name of names) {
            const probe = document.createElement('span');
            probe.style.backgroundColor = `var(${name})`;
            el.append(probe);
            result[name] = getComputedStyle(probe).backgroundColor;
            probe.remove();
        }
        result.outlineDisplay = getComputedStyle(el).getPropertyValue('--cem-action-outline-display').trim();
        return result;
    }, names);
    const ratio = (a, b) => {
        const luminance = color => {
            const channels = color.match(/[\d.]+/g).slice(0, 3).map(Number).map(v => color.startsWith('color(srgb ') ? v : v / 255);
            return channels.reduce((sum, c, i) => sum + (c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4) * [.2126, .7152, .0722][i], 0);
        };
        const [low, high] = [luminance(a), luminance(b)].sort((x, y) => x - y);
        return (high + .05) / (low + .05);
    };
    const palette = ['--cem-palette-comfort', '--cem-palette-comfort-text', '--cem-palette-creativity'];
    const expected = { scheme: 'light', outlineDisplay: 'none', '--cem-palette-comfort': 'rgb(255, 255, 255)',
        '--cem-palette-comfort-text': 'rgb(0, 0, 0)', '--cem-palette-creativity': 'rgb(106, 27, 154)' };
    for (const selector of ['main', '.inherited', 'aside[data-theme="cem-theme-projector"]', 'aside[data-cem-presentation]', '.cem-theme-contrast-light .cem-theme-projector']) {
        assert.deepEqual(await read(selector, palette), expected, selector);
    }
    await page.evaluate(() => document.documentElement.className = 'cem-theme-projector');
    assert.deepEqual(await read('html', palette), expected);
    await page.evaluate(() => document.documentElement.className = '');
    const nested = await read('main > .cem-theme-light', palette);
    assert.equal(nested['--cem-palette-comfort'], 'rgb(241, 254, 254)');
    assert.equal(nested['--cem-palette-creativity'], 'rgb(225, 190, 231)');
    assert.notEqual((await read('main > .cem-theme-native', palette))['--cem-palette-comfort'], expected['--cem-palette-comfort']);
    for (const emotion of ['comfort', 'conservative', 'creativity', 'trust', 'calm', 'enthusiasm', 'danger']) {
        const names = ['', '-x', '-text', '-text-x'].map(suffix => `--cem-palette-${emotion}${suffix}`);
        const values = await read('main', names);
        for (const [bg, ink] of [[names[0], names[2]], [names[1], names[3]]]) {
            const contrast = ratio(values[bg], values[ink]);
            assert.ok(contrast >= 7, `${emotion}: ${contrast}`);
            checks.push({ bg, ink, contrast });
        }
    }
    for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
        for (const state of ['readonly', 'editable', 'default', 'indeterminate', 'hover', 'active', 'pending']) {
            if (intent === 'contextual' && state === 'default') continue; // Canonically transparent.
            const bg = `--cem-action-${intent}-${state}-background`, ink = `--cem-action-${intent}-${state}-text`;
            const values = await read('main', [bg, ink]);
            const contrast = ratio(values[bg], values[ink]);
            assert.ok(contrast >= 7, `${intent}/${state}: ${contrast}`);
            checks.push({ bg, ink, contrast });
        }
        const first = `--cem-action-${intent}-pending-background`, second = `--cem-action-${intent}-pending-stripe-background`, ink = `--cem-action-${intent}-pending-text`;
        const values = await read('main', [first, second, ink]);
        const separation = ratio(values[first], values[second]);
        assert.ok(separation > 1.8, `${intent} pending stripe separation: ${separation}`);
        assert.ok(ratio(values[second], values[ink]) >= 7, `${intent} pending stripe text`);
        checks.push({ intent, separation, stripeTextContrast: ratio(values[second], values[ink]) });
    }
    // A container override must flow through inherited action endpoints.
    for (const token of catalog.tokens.filter(token => /^--cem-(select|navigation-item|content-interaction)-/.test(token.name) && token.name.endsWith('-background') && !token.name.includes('-disabled-'))) {
        const bg = token.name, ink = bg.replace(/-background$/, '-text');
        if (!catalog.tokens.some(token => token.name === ink)) continue;
        const values = await read('main', [bg, ink]);
        const background = values[bg] === 'rgba(0, 0, 0, 0)' ? expected['--cem-palette-comfort'] : values[bg];
        const contrast = ratio(background, values[ink]);
        assert.ok(contrast >= 7, `${bg}: ${contrast}`);
        checks.push({ bg, ink, contrast });
    }
    const group = await read('main', ['--cem-select-group-text']);
    assert.ok(ratio(group['--cem-select-group-text'], expected['--cem-palette-comfort']) >= 7);
    for (const selector of ['.inherited', 'main', 'aside[data-cem-presentation]']) {
        await page.locator(selector).evaluate(el => { for (const state of ['focus', 'target', 'selected']) el.setAttribute(state, ''); });
        const stripes = await read(selector, ['--cem-zebra-color-1', '--cem-zebra-color-2', '--cem-zebra-color-3']);
        for (const state of [1, 2, 3]) assert.ok(ratio(stripes[`--cem-zebra-color-${state}`], expected['--cem-palette-comfort']) >= 3, JSON.stringify({ selector, state, stripes }));
        for (const [first, second] of [[1, 2], [2, 3]]) assert.ok(ratio(stripes[`--cem-zebra-color-${first}`], stripes[`--cem-zebra-color-${second}`]) >= 3);
        await page.locator(selector).evaluate(el => { for (const state of ['focus', 'target', 'selected']) el.removeAttribute(state); });
    }
    await page.locator('main').evaluate(el => el.style.setProperty('--cem-palette-creativity', '#002f65'));
    assert.equal((await read('.inherited', ['--cem-action-primary-default-background']))['--cem-action-primary-default-background'], 'rgb(0, 47, 101)');
    await page.locator('main').evaluate(el => el.style.removeProperty('--cem-palette-creativity'));
    // Profile replacement and removal must not leak projector-specific pending paint.
    await page.locator('main').evaluate(el => { el.className = 'cem-theme-light'; });
    assert.equal((await read('main', palette))['--cem-palette-comfort'], nested['--cem-palette-comfort']);
    await page.locator('main').evaluate(el => { el.className = 'cem-theme-projector'; });
    const exported = catalog.tokens.find(token => token.name === '--cem-palette-comfort');
    assert.equal(exported.profiles.projector.toLowerCase(), '#ffffff');
    assert.equal(Object.keys(exported.modes).length, 5);
    await page.emulateMedia({ forcedColors: 'active' });
    const system = await page.evaluate(() => {
        const probe = document.createElement('span'); probe.style.backgroundColor = 'Canvas'; probe.style.color = 'CanvasText'; document.body.append(probe);
        const result = { bg: getComputedStyle(probe).backgroundColor, ink: getComputedStyle(probe).color }; probe.remove(); return result;
    });
    const forced = await read('main', palette);
    assert.equal(forced['--cem-palette-comfort'], system.bg);
    assert.equal(forced['--cem-palette-comfort-text'], system.ink);
    const output = resolve(root, '../../../dist/reports/cem-theme/projector.json');
    await mkdir(resolve(output, '..'), { recursive: true });
    await writeFile(output, JSON.stringify({ profile: 'projector', baseMode: 'light', checks, inheritance: true, forcedColors: true }, null, 2) + '\n');
    console.log(`projector: ${checks.length} contrast/state checks; selection, nested themes, overrides, profile replacement, exports and forced colors passed`);
} finally { await browser.close(); }
