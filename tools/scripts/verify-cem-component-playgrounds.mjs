import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, readdir, mkdtemp, mkdir, rm } from 'node:fs/promises';
import { resolve, extname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { chromium } from 'playwright';

const root = resolve(import.meta.dirname, '../..');
const temporary = await mkdtemp(join(tmpdir(), 'cem-action-playground-'));
const mime = { '.svg': 'image/svg+xml', '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.wasm': 'application/wasm' };
const navigationOnly = process.argv.includes('--navigation-only');
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
    if (!navigationOnly) {
        await verify(`${origin}/packages/cem-components/playgrounds/cem-action.html`);
        await verifyThemeSwitch(`${origin}/packages/cem-components/playgrounds/cem-theme-switch.html`);
        await verifySelect(`${origin}/packages/cem-components/playgrounds/cem-select.html`);
        for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
        for (const tag of ['cem-field', 'cem-text-field', 'cem-textarea']) await verifyField(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
        await verifyIcon(`${origin}/packages/cem-components/playgrounds/cem-icon.html`);
    }
    await verifyNavigation(`${origin}/packages/cem-components/playgrounds/`);
    if (!navigationOnly) {
        await verifyGalleries(`${origin}/packages/cem-components/playgrounds/`);
        await verifyBundle(`${origin}/packages/cem-components/playgrounds/cem-bundle.html`);
        await verifyPendingTheme(`${origin}/packages/cem-theme/dist/lib/css-generators/cem-colors.html`);
    }
    for (const [folder, name] of [['cem-components','cem-components'], ['cem-elements','cem-elements'], ['cem-demo-element','cem-demo-element'], ['cem-theme','cem-theme'], ['cem-ml-npm','cem-ml']]) {
        const output = JSON.parse(execFileSync('npm', ['pack', '--json', '--pack-destination', temporary], { cwd: join(root, 'packages', folder), encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } }));
        const target = join(temporary, 'installed/node_modules/@epa-wg', name);
        await mkdir(target, { recursive: true });
        execFileSync('tar', ['-xzf', join(temporary, output[0].filename), '--strip-components=1', '-C', target]);
    }
    if (!navigationOnly) {
        await verify(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-action.html`);
        await verifyThemeSwitch(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-theme-switch.html`);
        await verifySelect(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-select.html`);
        for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
        for (const tag of ['cem-field', 'cem-text-field', 'cem-textarea']) await verifyField(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
        await verifyIcon(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-icon.html`);
    }
    await verifyNavigation(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    if (!navigationOnly) {
        await verifyGalleries(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        await verifyBundle(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-bundle.html`);
        await verifyPendingTheme(`${origin}/installed/node_modules/@epa-wg/cem-theme/dist/lib/css-generators/cem-colors.html`);
    }
    if (navigationOnly) console.log('Gallery navigation verified on all source and isolated-package pages.');
    else console.log('Action, field, text-field, textarea, icon, icon-button, menu-item, select, theme-switch and bundle playgrounds verified from source and isolated package archives.');
} finally {
    await browser?.close();
    await new Promise(resolve => server.close(resolve));
    await rm(temporary, { recursive: true, force: true });
}

async function capturePlaygroundReadiness(page) {
    return page.evaluate(() => {
        const runtime = window.cemPlaygroundRuntime;
        const declaration = document.querySelector('body > cem-element');
        const instance = declaration?.querySelector('[data-cem-anonymous-instance]');
        const snapshot = runtime && instance ? runtime.snapshotInstance(instance) : null;
        return {
            url: location.href,
            stylesheets: [...document.querySelectorAll('link[rel=stylesheet]')].map(link => ({ href: link.href, loaded: !!link.sheet })),
            controlHeight: getComputedStyle(document.documentElement).getPropertyValue('--cem-control-height-small'),
            loaderSlices: snapshot?.slices,
            dataRevision: snapshot?.dataRevision,
            diagnostics: runtime && declaration ? runtime.diagnosticsFor(declaration) : [],
        };
    }).catch(error => ({ captureError: error.message }));
}

async function verifyActionGallery(page) {
    await page.waitForFunction(() => getComputedStyle(document.querySelector('#playground-theme')).getPropertyValue('--cem-control-height-small').trim() !== '');
    await page.waitForFunction(() => {
        const cards = [...document.querySelectorAll('cem-demo-element')];
        return cards.length === 7 && cards.every(card => card.getAttribute('data-state') === 'ready');
    });
    const declaration = await readFile(join(root, 'packages/cem-components/src/components/cem-action/cem-action.xhtml'), 'utf8');
    const implemented = [...new Set([
        ...[...declaration.matchAll(/\{attribute @name=([\w-]+)/g)].map(match => match[1]),
        ...[...declaration.matchAll(/datadom\.attributes\.([\w-]+)/g)].map(match => match[1]),
        ...[...declaration.matchAll(/:scope[^\s{]*?\[([\w-]+)/g)].map(match => match[1]),
        'class',
    ])].sort();
    const documented = await page.locator('[data-action-attribute]').evaluateAll(nodes => nodes.map(node => node.getAttribute('data-action-attribute')).sort());
    const demonstrated = await page.locator('cem-demo-element[data-covers]').evaluateAll(nodes => [...new Set(nodes.flatMap(node => node.getAttribute('data-covers').split(' ')))].sort());
    assert.deepEqual(documented, implemented, 'Gallery attribute inventory must match the canonical declaration');
    assert.deepEqual(demonstrated, implemented, 'Every implemented attribute needs a live example');
    assert.equal(await page.locator('cem-demo-element[legend="Labels and accessible names"] cem-action[label="Fallback label"] button').innerText(), 'Fallback label');
    assert.equal(await page.locator('cem-demo-element[legend="Labels and accessible names"] cem-action[aria-label="Add item"] button').getAttribute('aria-label'), 'Add item');
    for (const size of ['small', 'medium', 'large', 'x-large', 'xx-large']) {
        const sample = page.locator(`cem-demo-element[legend="Size profiles"] cem-action[size="${size}"]`);
        await sample.locator('button').waitFor();
        await sample.evaluate(async host => {
            await window.cemPlaygroundRuntime.whenRenderSettled(host);
        });
        const actual = await sample.evaluate(host => {
            const button = host.querySelector('button');
            const probe = document.createElement('span');
            probe.style.height = `var(--cem-control-height-${host.getAttribute('size')})`;
            probe.style.position = 'absolute';
            host.append(probe);
            const expected = probe.getBoundingClientRect().height;
            probe.remove();
            return { actual: button.getBoundingClientRect().height, expected };
        });
        assert(actual.expected > 0 && actual.actual >= actual.expected, `${size} minimum height: ${JSON.stringify(actual)}`);
    }
    assert.equal(await page.locator('cem-demo-element[legend="Size profiles"] cem-action:not([size])').getAttribute('size'), null);
    assert.equal(await page.locator('cem-demo-element[legend="Controlled selection"] cem-action[selected="false"] button').getAttribute('aria-pressed'), 'true');
    assert.equal(await page.locator('cem-demo-element[legend="Controlled selection"] cem-action[selectable="false"] button').getAttribute('aria-pressed'), 'false');
    await page.locator('cem-demo-element[legend="Controlled selection"] cem-action[selectable="false"] button').click();
    assert.equal(await page.locator('cem-demo-element[legend="Controlled selection"] cem-action[selectable="false"] button').getAttribute('aria-pressed'), 'false');
    assert.equal(await page.locator('cem-demo-element[legend="Controlled selection"] [slot="demo"] > cem-action:not([selected]):not([selectable]) button').getAttribute('aria-pressed'), null);
    await page.locator('[role="group"][aria-label="Layout choice"] cem-action:nth-of-type(2) button').click();
    await page.waitForFunction(() => document.querySelector('[role="group"][aria-label="Layout choice"] cem-action:nth-of-type(2) button')?.getAttribute('aria-pressed') === 'true' && document.querySelector('[role="group"][aria-label="Layout choice"] cem-action:nth-of-type(1) button')?.getAttribute('aria-pressed') === 'false');
    await verifyThemeControls(page);
    assert.equal(await page.locator('[role="group"][aria-label="Layout choice"] cem-action:nth-of-type(2) button').getAttribute('aria-pressed'), 'true');
    assert((await page.locator('cem-demo-element[legend="Controlled selection"] [slot=text]').innerText()).includes('selected="false"'));
    assert(await page.locator('cem-demo-element[legend="Pending and disabled"] cem-action[disabled="false"] button').isDisabled());
    assert(await page.locator('cem-demo-element[legend="Pending and disabled"] cem-action[disabled][pending="true"] button').isDisabled());
    assert.equal(await page.locator('cem-demo-element[legend="Pending and disabled"] cem-action[pending="true"]:not([disabled]) button').getAttribute('aria-busy'), 'true');
    assert(await page.locator('cem-demo-element[legend="Expanded and visibility"] cem-action:not([expanded]):not([hidden="until-found"])').isHidden());
    await page.locator('cem-demo-element[legend="Expanded and visibility"] cem-action[expanded] button').click();
    await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Expanded and visibility"] cem-action[expanded] button')?.getAttribute('aria-expanded') === 'true' && !document.querySelector('cem-demo-element[legend="Expanded and visibility"] cem-action:not([expanded]):not([hidden="until-found"])')?.hasAttribute('hidden'));
    await page.locator('cem-demo-element[legend="Expanded and visibility"] cem-action[expanded] button').click();
    await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Expanded and visibility"] cem-action[expanded] button')?.getAttribute('aria-expanded') === 'false' && document.querySelector('cem-demo-element[legend="Expanded and visibility"] cem-action:not([expanded]):not([hidden="until-found"])')?.hasAttribute('hidden'));
    assert.equal(await page.locator('cem-demo-element[legend="Expanded and visibility"] cem-action[hidden="until-found"]').getAttribute('hidden'), 'until-found');
    await page.locator('#gallery-form').evaluate(form => {
        window.gallerySubmissions = [];
        form.addEventListener('submit', event => {
            event.preventDefault();
            window.gallerySubmissions.push([...new FormData(form, event.submitter)]);
        });
    });
    const title = page.getByRole('textbox', { name: 'Example title', exact: true });
    await title.fill('');
    await page.locator('cem-demo-element[legend="Native forms"] cem-action[type="submit"][value="save"] button').click();
    assert.equal(await page.evaluate(() => window.gallerySubmissions.length), 0);
    await title.fill('Ready');
    await page.locator('cem-demo-element[legend="Native forms"] cem-action[type="submit"][value="save"] button').click();
    assert.deepEqual(await page.evaluate(() => window.gallerySubmissions[0]), [['title', 'Ready'], ['intent', 'save']]);
    await page.locator('cem-demo-element[legend="Native forms"] cem-action[type="button"] button').click();
    assert.equal(await page.evaluate(() => window.gallerySubmissions.length), 1);
    await page.locator('cem-demo-element cem-action[type="reset"] button').click();
    assert.equal(await title.inputValue(), 'Initial title');
    await title.fill('');
    const external = page.locator('cem-demo-element[legend="Native forms"] cem-action[form="gallery-form"][value="external"] button');
    assert.deepEqual(await external.evaluate(button => ({ form: button.form.id, method: button.formMethod, encoding: button.formEnctype, target: button.formTarget, noValidate: button.formNoValidate, action: new URL(button.formAction).pathname.split('/').pop() })), { form: 'gallery-form', method: 'get', encoding: 'application/x-www-form-urlencoded', target: '_blank', noValidate: true, action: 'cem-action-gallery.html' });
    await external.click();
    assert.deepEqual(await page.evaluate(() => window.gallerySubmissions[1]), [['title', ''], ['intent', 'external']]);
}

async function verifySelect(url) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
    try {
        await page.goto(url);
        await page.locator('#select-preview [role=combobox]').waitFor();
        await page.waitForFunction(() => document.querySelector('cem-demo-element')?.getAttribute('data-state') === 'ready');
        await page.evaluate(async () => {
            const runtime = window.cemPlaygroundRuntime;
            await Promise.all([...document.querySelectorAll('cem-select')].map(host => runtime.whenRenderSettled(host)));
        });
        assert.equal(await page.evaluate(() => window.scrollY), 0, 'Listbox examples must not scroll the page on mount');
        const host = page.locator('#select-preview');
        const control = () => host.locator('[part~=control]');
        const properties = page.getByRole('region', { name: 'Select properties', exact: true });
        const choose = async (label, value) => {
            const group = properties.getByRole('radiogroup', { name: label, exact: true });
            await group.getByRole('radio', { name: value, exact: true }).check();
            assert.equal(await group.getByRole('radio', { checked: true }).count(), 1);
        };
        const waitAttribute = (name, value) => page.waitForFunction(({ name, value }) => document.querySelector('#select-preview [part~=control]')?.getAttribute(name) === value, { name, value });
        await page.evaluate(() => { window.previewSelect = document.querySelector('#select-preview'); });
        assert.equal(await host.evaluate(node => node.value), 'ada');
        await control().click();
        await host.getByRole('option', { name: 'Grace Hopper', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('#select-committed')?.textContent.includes('grace'));
        await choose('Indicator', 'Outline');
        assert.equal(await host.evaluate(node => node.value), 'grace');
        assert(await page.evaluate(() => window.previewSelect === document.querySelector('#select-preview')));
        await properties.getByRole('textbox', { name: 'Label', exact: true }).fill('Assignee');
        await page.getByRole('combobox', { name: 'Assignee', exact: true }).waitFor();
        assert.equal(await control().getAttribute('aria-describedby'), 'select-preview-help');
        assert.equal(await page.locator('#select-preview-help').innerText(), 'Choose an available person.');
        await properties.getByRole('textbox', { name: 'Value', exact: true }).fill('grace');
        await page.waitForFunction(() => document.querySelector('#select-preview')?.getAttribute('value') === 'grace');
        await properties.getByRole('textbox', { name: 'Value', exact: true }).fill('ada');
        await page.waitForFunction(() => document.querySelector('#select-preview')?.value === 'ada');
        await properties.getByRole('textbox', { name: 'Form name', exact: true }).fill('assignee');
        await page.waitForFunction(() => new FormData(document.querySelector('#select-preview-form')).get('assignee') === 'ada');
        await properties.getByRole('textbox', { name: 'Autocomplete', exact: true }).fill('name');
        await page.waitForFunction(() => document.querySelector('#select-preview')?.getAttribute('autocomplete') === 'name');
        await choose('Busy', 'True');
        await waitAttribute('aria-busy', 'true');
        await choose('Busy', 'False');
        await waitAttribute('aria-busy', null);
        await choose('Invalid', 'True');
        await waitAttribute('aria-invalid', 'true');
        assert.equal(await control().getAttribute('aria-errormessage'), 'select-preview-error');
        await choose('Invalid', 'False');
        await waitAttribute('aria-invalid', null);
        await choose('Disabled', 'True');
        await page.waitForFunction(() => document.querySelector('#select-preview button')?.disabled);
        assert.equal(await page.locator('#select-preview-form').evaluate(form => new FormData(form).get('assignee')), null);
        await choose('Disabled', 'False');
        await page.waitForFunction(() => document.querySelector('#select-preview button')?.disabled === false);
        await choose('Visible rows', '4');
        await host.getByRole('listbox', { name: 'Assignee' }).waitFor();
        await choose('Multiple', 'True');
        await waitAttribute('aria-multiselectable', 'true');
        await host.getByRole('option', { name: 'Grace Hopper' }).click();
        await page.waitForFunction(() => document.querySelector('#select-preview')?.selectedValues.join(',') === 'ada,grace');
        assert.deepEqual(await page.locator('#select-preview-form').evaluate(form => new FormData(form).getAll('assignee')), ['ada', 'grace']);
        await choose('Multiple', 'False');
        await waitAttribute('aria-multiselectable', null);
        await choose('Visible rows', 'Default');
        await host.getByRole('combobox').waitFor();
        assert.equal(await host.getAttribute('size'), null);
        await properties.getByRole('textbox', { name: 'Value', exact: true }).fill('');
        await page.waitForFunction(() => !document.querySelector('#select-preview')?.hasAttribute('value'));
        await properties.getByRole('textbox', { name: 'Value', exact: true }).fill('unmatched');
        await properties.getByRole('textbox', { name: 'Placeholder', exact: true }).fill('Pick a colleague');
        await page.waitForFunction(() => document.querySelector('#select-preview button')?.textContent.includes('Pick a colleague'));
        await choose('Required', 'True');
        await page.waitForFunction(() => document.querySelector('#select-preview')?.validity.valueMissing);
        await choose('Required', 'False');
        await page.waitForFunction(() => document.querySelector('#select-preview')?.validity.valid);
        await verifyThemeControls(page);
        await page.locator('#example-grouped [role=combobox]').click();
        await page.locator('#example-grouped').getByRole('option', { name: 'Grace Hopper' }).click();
        await page.locator('#example-single').getByRole('option', { name: 'High', exact: true }).click();
        await page.locator('#example-multiple').getByRole('option', { name: 'Chat', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('#example-grouped')?.value === 'grace' && document.querySelector('#example-single')?.value === 'high' && document.querySelector('#example-multiple')?.selectedValues.join(',') === 'email,chat');
        const source = await page.getByRole('link', { name: 'Open canonical XHTML' }).getAttribute('href');
        const response = await page.request.get(new URL(source, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components/cem-select/cem-select.xhtml'), 'utf8'));
        assert.equal(await page.locator('cem-demo-element cem-select, cem-demo-element cem-element').count(), 0);
        assert((await page.locator('cem-demo-element').innerText()).includes('id="cem-select"'));
        await page.setViewportSize({ width: 360, height: 800 });
        assert(await page.locator('.workspace').evaluate(node => getComputedStyle(node).gridTemplateColumns.trim().split(/\s+/).length === 1));
        assert.deepEqual(errors, []);
    } catch (error) {
        console.error('Playground readiness', await capturePlaygroundReadiness(page));
        console.error(url, errors, await page.locator('body').innerText());
        throw error;
    } finally { await page.close(); }
}

async function verifyThemeControls(page) {
    const group = page.getByRole('radiogroup', { name: 'Theme', exact: true });
    const contrast = page.getByRole('checkbox', { name: 'Contrast', exact: true });
    const waitTheme = value => page.waitForFunction(value => document.querySelector('#playground-theme')?.getAttribute('data-theme') === value, value);
    await group.getByRole('radio', { name: 'Dark', exact: true }).click();
    await waitTheme('cem-theme-dark');
    await contrast.check();
    await waitTheme('cem-theme-contrast-dark');
    await group.getByRole('radio', { name: 'Native', exact: true }).click();
    await waitTheme('cem-theme-native');
    assert(await contrast.isDisabled());
    assert(!(await contrast.isChecked()));
    await page.emulateMedia({ colorScheme: 'dark' });
    const nativeDark = await page.locator('#playground-theme').evaluate(node => getComputedStyle(node).backgroundColor);
    await page.emulateMedia({ colorScheme: 'light' });
    const nativeLight = await page.locator('#playground-theme').evaluate(node => getComputedStyle(node).backgroundColor);
    assert.notEqual(nativeDark, nativeLight);
    await waitTheme('cem-theme-native');
    await group.getByRole('radio', { name: 'Light', exact: true }).click();
    await waitTheme('cem-theme-contrast-light');
    assert(await contrast.isChecked());
    await contrast.uncheck();
    await waitTheme('cem-theme-light');
}

async function verifyThemeSwitch(url) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
    try {
        await page.goto(url);
        await page.getByRole('radiogroup', { name: 'Theme', exact: true }).waitFor();
        await page.waitForFunction(() => document.querySelector('cem-demo-element')?.getAttribute('data-state') === 'ready');
        await page.locator('#retained-value').fill('Retained through themes');
        await page.evaluate(() => { window.originalInput = document.querySelector('#retained-value'); });
        await verifyThemeControls(page);
        assert((await page.locator('cem-demo-element').innerText()).includes('id="cem-theme-switch"'));
        assert.equal(await page.locator('#retained-value').inputValue(), 'Retained through themes');
        assert(await page.evaluate(() => window.originalInput === document.querySelector('#retained-value')));
        const source = await page.getByRole('link', { name: 'Open canonical XHTML' }).getAttribute('href');
        const response = await page.request.get(new URL(source, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components/cem-theme-switch/cem-theme-switch.xhtml'), 'utf8'));
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
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
        assert.equal(await page.getByRole('radiogroup').count(), 8);
        assert.equal(await page.getByRole('radio').count(), 27);
        for (const [label, value] of [['Size', 'Undefined'], ['Intent', 'Primary'], ['Bend', 'Smooth'], ['Button type', 'Button'], ['Disabled', 'False'], ['Pending', 'False'], ['Selected', 'Unset']]) {
            const group = page.getByRole('radiogroup', { name: label, exact: true });
            assert(await group.getByRole('radio', { name: value, exact: true }).isChecked());
            for (const radio of await group.getByRole('radio').all()) assert(await radio.isVisible());
        }
        for (const variant of ['Explicit', 'Contextual', 'Alternate', 'Destructive', 'Primary']) {
            await choose('Intent', variant);
            await page.waitForFunction(variant => document.querySelector('#action-preview')?.getAttribute('variant') === variant, variant.toLowerCase());
        }
        for (const [size, height] of [['Small', 3], ['Medium', 3], ['Large', 4], ['X-large', 6], ['XX-large', 8], ['Undefined', 3]]) {
            await choose('Size', size);
            await page.waitForFunction(({ size, height }) => {
                const host = document.querySelector('#action-preview');
                const button = host?.querySelector('button');
                const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
                return host?.getAttribute('size') === (size === 'Undefined' ? null : size.toLowerCase()) &&
                    button?.getBoundingClientRect().height === height * rem;
            }, { size, height });
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
            window.pendingTransitions = [];
            node.addEventListener('animationstart', event => window.pendingTransitions.push(event.animationName));
        });
        await choose('Pending', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'true');
        await page.waitForFunction(() => window.pendingTransitions.some(name => name.startsWith('cem-pending-shift-')));
        const movingPosition = await button.evaluate(node => getComputedStyle(node).backgroundPositionX);
        await page.waitForFunction(position => getComputedStyle(document.querySelector('#action-preview button')).backgroundPositionX !== position, movingPosition);
        assert.equal(await button.evaluate(node => node.getAnimations()[0].effect.getTiming().iterations), Infinity);
        assert.equal(await button.evaluate(node => node.getAnimations()[0].effect.getTiming().duration), 2000);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        await verifyThemeControls(page);
        const modes = page.getByRole('radiogroup', { name: 'Theme', exact: true });
        const contrastToggle = page.getByRole('checkbox', { name: 'Contrast', exact: true });
        for (const mode of ['Dark', 'Light']) {
            await modes.getByRole('radio', { name: mode, exact: true }).click();
            await contrastToggle.check();
            await page.waitForFunction(mode => document.querySelector('#playground-theme')?.getAttribute('data-theme') === `cem-theme-contrast-${mode}`, mode.toLowerCase());
            const contour = await button.evaluate(node => {
                const style = getComputedStyle(node, '::before');
                const probe = document.createElement('span');
                probe.style.backgroundColor = 'var(--cem-palette-comfort)';
                node.append(probe);
                const surface = getComputedStyle(probe).backgroundColor;
                probe.remove();
                return { display: style.display, image: style.backgroundImage, position: style.backgroundPositionX, fill: getComputedStyle(node).backgroundColor, surface };
            });
            assert.equal(contour.display, 'block');
            assert.match(contour.image, /linear-gradient\(45deg/);
            assert.equal(contour.fill, contour.surface);
            await page.waitForFunction(position => getComputedStyle(document.querySelector('#action-preview button'), '::before').backgroundPositionX !== position, contour.position);
            for (const intent of ['Primary', 'Explicit', 'Contextual', 'Alternate', 'Destructive']) {
                await choose('Intent', intent);
                await page.waitForFunction(intent => document.querySelector('#action-preview')?.getAttribute('variant') === intent, intent.toLowerCase());
                await button.evaluate(node => {
                    for (const animation of node.getAnimations({ subtree: true })) {
                        animation.pause();
                        animation.currentTime = 125;
                    }
                });
                const firstFrame = await button.screenshot({ animations: 'allow' });
                await button.evaluate(node => {
                    for (const animation of node.getAnimations({ subtree: true })) animation.currentTime = 625;
                });
                const secondFrame = await button.screenshot({ animations: 'allow' });
                assert(!firstFrame.equals(secondFrame), `${mode} ${intent}: pending contour must visibly move`);
                await button.evaluate(node => {
                    for (const animation of node.getAnimations({ subtree: true })) animation.play();
                });
            }
            await choose('Intent', 'Primary');
        }
        assert.equal(await label.inputValue(), 'Publish now');
        assert.equal(await button.innerText(), 'Publish now');
        assert(await page.evaluate(() => window.originalPreview === document.querySelector('#action-preview button')));
        assert.equal(await button.getAttribute('aria-busy'), 'true');
        assert(await page.locator('#action-preview').evaluate(node => node.classList.contains('cem-bend-round')));
        const pendingPaint = await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]);
        await choose('Disabled', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === true);
        assert.deepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        const disabledPosition = await button.evaluate(node => getComputedStyle(node).backgroundPositionX);
        await page.waitForFunction(position => getComputedStyle(document.querySelector('#action-preview button')).backgroundPositionX !== position, disabledPosition);
        await choose('Pending', 'False');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'false');
        assert.notDeepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        await page.emulateMedia({ reducedMotion: 'reduce' });
        await choose('Pending', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('aria-busy') === 'true');
        assert.deepEqual(await button.evaluate(node => [getComputedStyle(node).backgroundColor, getComputedStyle(node).color]), pendingPaint);
        assert.equal(await button.evaluate(node => node.getAnimations({ subtree: true }).length), 0);
        assert.match(await button.evaluate(node => getComputedStyle(node, '::before').backgroundImage), /linear-gradient\(45deg/);
        assert.match(await button.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await button.evaluate(node => getComputedStyle(node).backgroundImage), 'none');
        assert.equal(await button.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        assert.equal(await button.evaluate(node => getComputedStyle(node, '::before').display), 'none');
        assert.equal(await button.evaluate(node => node.getAnimations({ subtree: true }).length), 0);
        await page.emulateMedia({ reducedMotion: 'no-preference', forcedColors: 'none' });
        await choose('Disabled', 'False');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === false);
        for (const value of ['True','False','Unset']) {
            await choose('Selected', value);
            await page.waitForFunction(value => document.querySelector('#action-preview button')?.getAttribute('aria-pressed') === value, value === 'Unset' ? null : value.toLowerCase());
        }
        for (const [label, value] of [['Size', 'Undefined'], ['Intent', 'Primary'], ['Bend', 'Round'], ['Button type', 'Button'], ['Disabled', 'False'], ['Pending', 'True'], ['Selected', 'Unset']]) {
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
        assert.equal(await page.locator('cem-action').count(), 11);
        for (const [id, result] of [['size-inline', 'Change undone.'], ['size-text', 'Draft saved.'], ['size-icon', 'Item added.'], ['size-play', 'Playback sample activated.'], ['size-tile', 'Mountain image selected.'], ['size-hero', 'Mountain campaign selected.']]) {
            await page.locator(`#${id} button`).click();
            await page.waitForFunction(result => document.querySelector('#dimension-example-result')?.textContent.trim() === result, result);
        }
        await page.setViewportSize({ width: 360, height: 800 });
        const samples = await page.locator('.dimension-examples button').evaluateAll(nodes => nodes.map(button => ({
            size: button.parentElement.getAttribute('size'),
            border: parseFloat(getComputedStyle(button).borderTopWidth),
            width: button.clientWidth, scrollWidth: button.scrollWidth,
            height: button.getBoundingClientRect().height,
            rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
        })));
        for (const sample of samples) {
            assert(sample.scrollWidth <= sample.width, `Sample ${sample.size} must wrap without horizontal overflow`);
            assert.equal(sample.border > 0, sample.size === 'small');
            if (sample.size === 'xx-large') assert(sample.height >= 8 * sample.rem);
        }
        assert(await page.locator('.choice-content img').evaluateAll(images => images.every(img => img.complete && img.naturalWidth > 0)));
        assert(await page.locator('.icon-actions').filter({ has: page.locator('#size-icon') }).evaluate(node => {
            const buttons = node.querySelectorAll('button');
            const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
            return buttons[1].getBoundingClientRect().left - buttons[0].getBoundingClientRect().right >= .5 * rem;
        }));
        assert(await page.locator('.hero-content').evaluate(node => getComputedStyle(node).gridTemplateColumns.trim().split(/\s+/).length === 1));
        await page.setViewportSize({ width: 1280, height: 720 });
        assert.equal(await page.locator('cem-demo-element cem-action, cem-demo-element cem-element').count(), 0);
        assert((await page.locator('cem-demo-element').innerText()).includes('id="cem-action"'));
        assert.equal(await page.locator('cem-element[tag="cem-action"]').count(), 1);
        assert.equal(await page.getByRole('radiogroup', { name: 'Expanded', exact: true }).count(), 0);
        assert.equal(await page.locator('#choice-compact button').getAttribute('aria-pressed'), 'true');
        await page.locator('#choice-comfortable button').click();
        await page.waitForFunction(() => document.querySelector('#choice-comfortable button')?.getAttribute('aria-pressed') === 'true' && document.querySelector('#choice-compact button')?.getAttribute('aria-pressed') === 'false');
        assert(await page.locator('cem-radio > label, cem-switch > label, cem-checkbox > label').evaluateAll(nodes => nodes.every(node => getComputedStyle(node).boxShadow === 'none')));
        await page.locator('h1').click();
        assert.notEqual(await page.getByRole('textbox', { name: 'Label', exact: true }).evaluate(node => getComputedStyle(node).boxShadow), 'none');
        await choose('Selected', 'True');
        await choose('Disabled', 'True');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.disabled === true && document.querySelector('#action-preview button')?.getAttribute('aria-pressed') === 'true');
        assert.notEqual(await button.evaluate(node => getComputedStyle(node).boxShadow), 'none');
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await button.evaluate(node => getComputedStyle(node, '::before').display), 'block');
        assert.equal(await button.evaluate(node => getComputedStyle(node, '::before').borderStyle), 'solid');
        assert.equal(await button.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        await page.emulateMedia({ forcedColors: 'none' });
        const canonical = await page.getByRole('link', { name: 'Open canonical XHTML' }).getAttribute('href');
        const response = await page.request.get(new URL(canonical, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components/cem-action/cem-action.xhtml'), 'utf8'));
        assert.equal(await page.getByRole('link', { name: 'Automated stories' }).count(), 1);
        assert.deepEqual(errors, []);
        const gallery = await page.getByRole('link', { name: 'Full examples and variation matrix' }).getAttribute('href');
        assert.equal(new URL(gallery, page.url()).href, new URL('cem-action-gallery.html', url).href);
        await page.goto(gallery);
        await verifyActionGallery(page);
        assert.equal(await page.locator('cem-demo-element').count(), 7);
        const returnLink = page.getByRole('link', { name: 'Action property playground and source' });
        await returnLink.waitFor();
        assert.equal(new URL(await returnLink.getAttribute('href'), page.url()).href, url);
        assert.deepEqual(errors, []);
    } catch (error) {
        console.error('Playground readiness', await capturePlaygroundReadiness(page));
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
            const contour = getComputedStyle(node, '::before');
            const style = contour.display === 'block' ? contour : getComputedStyle(node);
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
        assert.equal(await cell.evaluate(node => node.getAnimations({ subtree: true }).length), 0);
        assert.match(await cell.evaluate(node => getComputedStyle(node).backgroundImage), /linear-gradient\(45deg/);
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await cell.evaluate(node => getComputedStyle(node).backgroundImage), 'none');
        assert.equal(await cell.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        console.log('Pending theme gradients verified:', url);
    } finally { await page.close(); }
}


async function verifyBundle(url) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(url);
        await page.locator('#bundle-textarea textarea').waitFor();
        assert.equal(await page.locator('#bundle-textarea textarea').inputValue(), 'First line\nSecond line');
        await page.locator('#bundle-icon [role=img]').waitFor();
        await page.locator('#bundle-action button').waitFor();
        await page.locator('cem-icon-button button cem-icon [part="glyph"]').waitFor();
        await page.locator('#bundle-select [role=combobox]').waitFor();
        await page.locator('#bundle-field input').waitFor();
        await page.locator('#bundle-text-field input').waitFor();
        assert.equal(await page.locator('#bundle-field input').inputValue(), 'Field value');
        assert.equal(await page.locator('#bundle-text-field input').inputValue(), 'Text value');
        await page.evaluate(async () => {
            for (const host of document.querySelectorAll('#bundle-action, #bundle-select, #bundle-theme, #bundle-field, #bundle-text-field')) {
                await window.cemPlaygroundRuntime.whenRenderSettled(host);
            }
        });
        const result = await page.evaluate(async () => {
            const runtime = window.cemPlaygroundRuntime;
            const declaration = [...document.querySelectorAll('cem-element[tag="cem-action"]')][0];
            const bundleUrl = declaration.getAttribute('src').split('#')[0];
            const bundle = new DOMParser().parseFromString(await (await fetch(bundleUrl)).text(), 'application/xml');
            const checks = [];
            for (const tag of ['cem-action', 'cem-select', 'cem-theme-switch', 'cem-icon', 'cem-icon-button', 'cem-menu-item', 'cem-field', 'cem-text-field', 'cem-textarea']) {
                const sourceUrl = new URL(`../src/components/${tag}/${tag}.xhtml`, bundleUrl).href;
                const original = new DOMParser().parseFromString(await (await fetch(sourceUrl)).text(), 'application/xml');
                const template = bundle.getElementById(tag);
                const originalTemplate = original.getElementById(tag);
                checks.push({ tag, same: template?.textContent === originalTemplate?.textContent,
                    count: bundle.querySelectorAll(`[id="${tag}"]`).length,
                    base: document.querySelector(tag)[Symbol.for('@epa-wg/cem-elements/resource-base-url')], sourceUrl });
                const attrs = node => [...node.attributes].filter(a => a.name !== 'xmlns').map(a => [a.name, a.value]).sort();
                checks.at(-1).metadata = JSON.stringify(attrs(template.parentElement)) === JSON.stringify(attrs(originalTemplate.parentElement));
            }
            const styleCount = document.querySelectorAll('style[data-cem-declaration-style]').length;
            const duplicate = document.createElement('cem-element');
            duplicate.setAttribute('tag', 'cem-action');
            duplicate.setAttribute('src', declaration.getAttribute('src'));
            declaration.parentElement.append(duplicate);
            await runtime.whenDeclarationSettled(duplicate);
            const diagnostics = runtime.diagnosticsFor(duplicate).map(item => item.code);
            const stylesAfter = document.querySelectorAll('style[data-cem-declaration-style]').length;
            duplicate.remove();
            return { checks, invalidXml: !!bundle.querySelector('parsererror'), styleCount, stylesAfter, diagnostics };
        });
        assert.equal(result.invalidXml, false);
        for (const check of result.checks) {
            assert.equal(check.same, true, `${check.tag}: bundle template text differs`);
            assert.equal(check.metadata, true, `${check.tag}: declaration metadata differs`);
            assert.equal(check.count, 1);
            assert.equal(check.base, check.sourceUrl);
        }
        assert.equal(result.styleCount, result.checks.length);
        assert.equal(result.stylesAfter, result.styleCount);
        assert.ok(result.diagnostics.includes('cem-element.registry_same_scope_duplicate'));
        assert.equal(await page.locator('#bundle-action button').getAttribute('aria-pressed'), 'true');
        await page.locator('#bundle-select [role=combobox]').click();
        await page.locator('#bundle-select [role=option]').filter({ hasText: 'Grace' }).click();
        assert.equal(await page.locator('#bundle-select').evaluate(host => host.value), 'grace');
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
}

async function verifyCommand(url, tag) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(url);
        await page.waitForSelector('#command-preview button');
        await page.waitForSelector(tag === 'cem-icon-button'
            ? 'cem-demo-element[legend="Action variants"] cem-icon-button[variant="primary"] button'
            : '#command-configured button');
        await page.evaluate(async () => {
            await Promise.all([...document.querySelectorAll('template[data-cem-island="instance"]')].map(island => window.cemPlaygroundRuntime.whenRenderSettled(island.parentElement)));
        });
        const source = await readFile(join(root, `packages/cem-components/src/components/${tag}/${tag}.xhtml`), 'utf8');
        const attributes = [...new Set([
            ...[...source.matchAll(/\{attribute @name=([\w-]+)/g)].map(match => match[1]),
            ...[...source.matchAll(/datadom\.attributes\.([\w-]+)/g)].map(match => match[1]),
            ...[...source.matchAll(/:scope[^\s{]*?\[([\w-]+)/g)].map(match => match[1]), 'class',
        ])].sort();
        assert.deepEqual(await page.locator('[data-command-attribute]').evaluateAll(nodes => nodes.map(node => node.getAttribute('data-command-attribute')).sort()), attributes);
        assert.deepEqual(await page.locator('cem-demo-element[data-covers]').evaluateAll(nodes => [...new Set(nodes.flatMap(node => node.getAttribute('data-covers').split(' ')))].sort()), attributes);
        if (tag !== 'cem-icon-button') {
            assert(await page.locator('#command-disabled button').isDisabled());
            assert(await page.locator('#command-hidden').isHidden());
            assert.equal(await page.locator('#command-expanded button').getAttribute('aria-expanded'), 'true');
            assert.equal(await page.locator('#command-configured button').getAttribute('aria-expanded'), 'false');
        }
        const layout = await page.locator('.property-groups').evaluate(node => ({ display: getComputedStyle(node).display, wrap: getComputedStyle(node).flexWrap,
            options: [...node.querySelectorAll('.property-options')].map(options => [getComputedStyle(options).display, getComputedStyle(options).flexDirection]) }));
        assert.equal(layout.display, 'flex');
        assert.equal(layout.wrap, 'wrap');
        assert(layout.options.every(([display, direction]) => display === 'flex' && direction === 'column'));
        const disabled = page.locator(`input[name="${tag.slice(4)}-disabled"][value="false"]`);
        await disabled.check();
        await page.waitForFunction(() => document.querySelector('#command-preview button')?.disabled === true);
        await page.locator(`input[name="${tag.slice(4)}-disabled"][value="absent"]`).check();
        await page.waitForFunction(() => document.querySelector('#command-preview button')?.disabled === false);
        await page.locator('#command-preview button').click();
        await page.locator('#command-preview button').press('Space');
        await page.evaluate(() => window.cemPlaygroundRuntime.whenRenderSettled(document.querySelector('#command-preview')));
        assert.equal(await page.locator('#command-preview').evaluate((host, name) => window.cemPlaygroundRuntime.snapshotInstance(host).slices[name], tag === 'cem-icon-button' ? 'pressed' : 'selected'), 'click');
        await page.setViewportSize({ width: 375, height: 812 });
        assert(await page.locator('.property-groups').evaluate(node => node.getBoundingClientRect().right <= innerWidth));
        if (tag === 'cem-icon-button') await verifyIconLinkExamples(page);
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
}


async function verifyField(url, tag) {
    const context = await browser.newContext();
    const page = await context.newPage();
    try {
        await page.goto(url);
        await page.waitForSelector('#field-preview [part="control"]');
        const input = page.locator('#field-preview [part="control"]');
        await page.waitForFunction(() => new FormData(document.querySelector('#field-form')).get('account') === 'initial');
        await page.evaluate(() => window.cemPlaygroundRuntime.whenRenderSettled(document.querySelector('#field-preview')));
        const edited = tag === 'cem-textarea' ? 'edited\nsecond line' : 'edited';
        await input.fill(edited);
        await page.waitForFunction(value => new FormData(document.querySelector('#field-form')).get('account') === value, edited);
        await page.evaluate(() => window.cemPlaygroundRuntime.whenRenderSettled(document.querySelector('#field-preview')));
        await page.getByRole('button', { name: 'Reset', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('#field-preview [part="control"]')?.value === 'initial');
        assert.deepEqual(await page.evaluate(() => new FormData(document.querySelector('#field-form')).getAll('account')), ['initial']);
        if (tag === 'cem-textarea') {
            await page.getByRole('textbox', { name: 'value', exact: true }).fill('first\nsecond');
            await page.waitForFunction(() => document.querySelector('#field-preview textarea')?.value === 'first\nsecond');
            await page.getByRole('textbox', { name: 'rows', exact: true }).fill('6');
            await page.waitForFunction(() => document.querySelector('#field-preview textarea')?.rows === 6);
        }

        for (const attribute of ['disabled', 'required', 'readonly', 'busy']) {
            const group = page.getByRole('radiogroup', { name: attribute, exact: true });
            await group.getByRole('radio', { name: 'false', exact: true }).check();
            await page.waitForFunction(attribute => {
                const control = document.querySelector('#field-preview [part="control"]');
                return attribute === 'busy' ? control?.getAttribute('aria-busy') === 'true' : control?.hasAttribute(attribute);
            }, attribute);
            await group.getByRole('radio', { name: 'absent', exact: true }).check();
            await page.waitForFunction(attribute => {
                const control = document.querySelector('#field-preview [part="control"]');
                return !control?.hasAttribute(attribute === 'busy' ? 'aria-busy' : attribute);
            }, attribute);
        }
        await page.getByRole('radiogroup', { name: 'busy', exact: true }).getByRole('radio', { name: 'true', exact: true }).check();
        await page.waitForFunction(() => getComputedStyle(document.querySelector('#field-preview [part="control"]')).boxShadow !== 'none');
        await page.emulateMedia({ forcedColors: 'active' });
        assert.equal(await input.evaluate(node => getComputedStyle(node).borderWidth), '0px');
        assert.equal(await input.evaluate(node => getComputedStyle(node).boxShadow), 'none');
        assert.equal(await input.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        const sourceUrl = await page.getByRole('link', { name: 'Canonical XHTML', exact: true }).getAttribute('href');
        const response = await page.request.get(new URL(sourceUrl, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components', tag, tag + '.xhtml'), 'utf8'));
        const diagnostics = await page.evaluate(() => window.cemPlaygroundRuntime.diagnosticsFor(document.querySelector('#field-preview')));
        assert.deepEqual(diagnostics, [], tag + ': runtime diagnostics');
    } finally { await context.close(); }
}

async function verifyGalleries(baseUrl) {
    const folders = await readdir(join(root, 'packages/cem-components/src/components'), { withFileTypes: true });
    for (const folder of folders.filter(entry => entry.isDirectory())) {
        const tag = folder.name;
        const context = await browser.newContext();
        const page = await context.newPage();
        try {
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const link = page.getByRole('link', { name: 'Full examples and variation matrix', exact: true });
            await link.waitFor();
            const href = await link.getAttribute('href');
            assert.equal(new URL(href, page.url()).href, new URL(`${tag}-gallery.html`, baseUrl).href);
            await link.click();
            if (tag === 'cem-action') {
                // The existing intent/bend matrix uses the shared five-mode switch.
                await verifyActionGallery(page);
                await page.getByRole('link', { name: 'Action property playground and source' }).click();
                assert.equal(new URL(page.url()).pathname, new URL(`${tag}.html`, baseUrl).pathname);
                continue;
            }
            const sampleSelector = `[data-gallery-theme] ${tag}`;
            await page.waitForFunction(selector => {
                const samples = [...document.querySelectorAll(selector)];
                return samples.length > 0 && samples.every(node => node.querySelector('[part]'));
            }, sampleSelector);
            assert.equal(await page.locator('[data-gallery-theme]').count(), 5, `${tag}: five theme modes`);
            await page.waitForFunction(() => {
                const demos = [...document.querySelectorAll('cem-demo-element')];
                return demos.length > 0 && demos.every(node => node.getAttribute('data-state') === 'ready');
            });
            const declaration = await readFile(join(root, 'packages/cem-components/src/components', tag, tag + '.xhtml'), 'utf8');
            const implemented = [...new Set([
                ...[...declaration.matchAll(/\{attribute @name=([\w-]+)/g)].map(match => match[1]),
                ...[...declaration.matchAll(/datadom\.attributes\.([\w-]+)/g)].map(match => match[1]),
                'hidden', 'class',
            ])].filter(name => name !== 'data-theme');
            const documented = await page.locator('[data-gallery-attribute], [data-action-attribute]').evaluateAll(nodes => nodes.map(node => node.getAttribute('data-gallery-attribute') ?? node.getAttribute('data-action-attribute')));
            for (const attribute of implemented) assert(documented.includes(attribute), `${tag}: missing ${attribute} in gallery inventory`);
            await page.getByRole('link', { name: 'Automated stories', exact: true }).waitFor();
            const first = page.locator(sampleSelector).first();
            const control = first.locator('[part="control"], input').first();
            if (tag !== 'cem-icon') {
                await control.hover();
                await control.focus();
            }
            if (tag === 'cem-field' || tag === 'cem-text-field' || tag === 'cem-textarea') {
                assert.equal(await control.evaluate(node => getComputedStyle(node).borderWidth), '0px');
                await page.locator('cem-demo-element form :is(cem-field, cem-text-field, cem-textarea)[required] [part=control]').fill('edited@example.com');
                await page.locator('cem-demo-element cem-action[type="reset"] button').click();
                await page.waitForFunction(value => document.querySelector('cem-demo-element form :is(cem-field, cem-text-field, cem-textarea)[required] [part=control]')?.value === value, tag === 'cem-textarea' ? 'First line\nSecond line' : 'reader@example.com');
            }
            if (tag === 'cem-select') {
                const outline = page.locator(`${sampleSelector}[indicator="outline"]`).first().locator('[part="control"]');
                assert.equal(await outline.evaluate(node => getComputedStyle(node).getPropertyValue('--_cem-input-indicator-appearance').trim()),
                    await outline.evaluate(node => getComputedStyle(node).getPropertyValue('--cem-indicator-appearance-outline').trim()));
                await control.click();
                await page.keyboard.press('ArrowDown');
                await page.keyboard.press('Enter');
                assert.equal(await first.evaluate(node => node.value), 'grace');
            }
            if (tag === 'cem-icon-button') await verifyIconLinkExamples(page);
            if (tag === 'cem-icon') { await verifyLegacyIconExamples(page); await verifyIconLabels(page); await verifyIconImageExamples(page); }
            const diagnostics = await page.evaluate(selector => [...document.querySelectorAll(selector)].flatMap(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), sampleSelector);
            assert.deepEqual(diagnostics, [], `${tag}: gallery diagnostics`);
            const back = page.locator(`a[href="./${tag}.html"]`);
            if (await back.count()) await back.click();
            else await page.getByRole('link', { name: 'Action property playground and source' }).click();
            assert.equal(new URL(page.url()).pathname, new URL(`${tag}.html`, baseUrl).pathname);
        } finally { await context.close(); }
    }
    console.log('All canonical component galleries verified: ' + baseUrl);
}

async function verifyIconButtonSamples(page) {
    const sample = legend => page.locator(`cem-demo-element[legend="${legend}"]`);
    const legends = ['Command buttons', 'Command activation', 'Navigation links', 'Disabled buttons', 'Disabled links', 'Selection', 'Pending actions', 'Action variants', 'Button types', 'External form ownership', 'Submit overrides', 'Action bends', 'Host classes', 'Visibility'];
    assert.equal(await sample('All implemented attributes').count(), 0);
    for (const legend of legends) {
        assert.equal(await sample(legend).count(), 1, `${legend}: dedicated sample`);
        assert(await sample(legend).getAttribute('description'), `${legend}: explanation`);
        await page.waitForFunction(legend => {
            const demo = [...document.querySelectorAll('cem-demo-element')].find(node => node.getAttribute('legend') === legend);
            const hosts = [...demo.querySelectorAll('cem-icon-button')];
            return hosts.length > 0 && hosts.every(host => host.querySelector('[part="control"] cem-icon [part="content"]'));
        }, legend);
    }
    for (const button of await sample('Command buttons').locator('button').all()) {
        assert.equal(await button.getAttribute('type'), 'button');
        assert.equal(await button.getAttribute('aria-expanded'), null);
    }
    const activation = sample('Command activation');
    const command = activation.locator('button');
    await command.click();
    await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Command activation"] [role="status"]')?.textContent.includes('click'));
    await command.press('Space');
    await command.press('Enter');
    assert.equal(await command.getAttribute('aria-pressed'), null);
    const navigation = sample('Navigation links');
    for (const href of ['#icon-link-target', './cem-icon.html', 'https://github.com/EPA-WG/cem', '']) {
        const link = navigation.locator(`cem-icon-button[href="${href}"] a`);
        assert.equal(await link.getAttribute('href'), href);
    }
    for (const host of await sample('Disabled buttons').locator('cem-icon-button').all()) {
        assert.equal(await host.locator('button').isDisabled(), await host.getAttribute('disabled') !== null);
    }
    for (const host of await sample('Disabled links').locator('cem-icon-button').all()) {
        const disabled = await host.getAttribute('disabled') !== null;
        const link = host.locator('a');
        assert.equal(await link.getAttribute('href'), disabled ? null : '#icon-link-target');
        assert.equal(await link.getAttribute('aria-disabled'), disabled ? 'true' : null);
        assert.equal(await link.getAttribute('tabindex'), disabled ? '-1' : null);
        if (disabled) {
            const before = await host.evaluate(node => window.cemPlaygroundRuntime.snapshotInstance(node).eventPayloads);
            await link.evaluate(node => node.click());
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.snapshotInstance(node).eventPayloads), before);
        }
    }
    for (const host of await sample('Selection').locator('cem-icon-button').all()) {
        const control = host.locator('[part="control"]');
        const link = await host.getAttribute('href') !== null;
        const selected = await host.getAttribute('selected') !== null;
        const selectable = await host.getAttribute('selectable') !== null;
        assert.equal(await control.getAttribute('aria-pressed'), link ? null : selected ? 'true' : selectable ? 'false' : null);
        assert.equal(await control.getAttribute('aria-current'), link && selected ? 'true' : null);
        assert.equal(await control.getAttribute('aria-expanded'), null);
        if (selected) assert.notEqual(await control.evaluate(node => getComputedStyle(node).boxShadow), 'none');
    }
    for (const host of await sample('Pending actions').locator('cem-icon-button').all()) {
        const control = host.locator('[part="control"]');
        const pending = await host.getAttribute('pending');
        assert.equal(await control.getAttribute('aria-busy'), pending);
        const image = await control.evaluate(node => getComputedStyle(node).backgroundImage);
        if (pending === 'true') assert(image.includes('linear-gradient'));
        else assert.equal(image, 'none');
    }
    const pendingControls = sample('Pending actions').locator('cem-icon-button[pending="true"] [part="control"]');
    await page.emulateMedia({ reducedMotion: 'reduce' });
    for (const control of await pendingControls.all()) {
        assert.equal(await control.evaluate(node => getComputedStyle(node).animationName), 'none');
        assert.equal(await control.evaluate(node => getComputedStyle(node, '::before').animationName), 'none');
    }
    await page.emulateMedia({ forcedColors: 'active' });
    for (const control of await pendingControls.all()) {
        assert.equal(await control.evaluate(node => getComputedStyle(node).backgroundImage), 'none');
        assert.equal(await control.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
    }
    for (const control of await sample('Pending actions').locator('cem-icon-button[selected] [part="control"]').all()) {
        assert.equal(await control.evaluate(node => getComputedStyle(node, '::before').borderTopStyle), 'solid');
        assert.equal(await control.evaluate(node => getComputedStyle(node, '::before').display), 'block');
    }
    await page.emulateMedia({ reducedMotion: 'no-preference', forcedColors: 'none' });
    for (const host of await sample('Action variants').locator('cem-icon-button').all()) {
        const variant = await host.getAttribute('variant') ?? 'primary';
        assert(await host.locator('[part="control"]').evaluate((node, variant) => node.classList.contains(`cem-icon-button--${variant}`), variant));
    }
    for (const legend of ['Button types', 'External form ownership', 'Submit overrides']) {
        for (const host of await sample(legend).locator('cem-icon-button').all()) {
            const authored = await host.evaluate(node => Object.fromEntries([...node.attributes].map(attr => [attr.name, attr.value])));
            const button = host.locator('button');
            assert.equal(await button.getAttribute('type'), authored.type ?? 'button');
            for (const attribute of ['name', 'value', 'form', 'formaction', 'formenctype', 'formmethod', 'formtarget']) {
                assert.equal(await button.getAttribute(attribute), authored[attribute] ?? null);
            }
            assert.equal(await button.evaluate(node => node.formNoValidate), 'formnovalidate' in authored);
            assert.equal(await button.evaluate(node => node.form?.id), 'icon-button-example-form');
        }
    }
    for (const host of await sample('Host classes').locator('cem-icon-button').all()) {
        assert.equal(await host.getAttribute('class'), 'consumer-command');
        assert.equal(await host.locator('cem-icon').getAttribute('class'), 'consumer-command');
    }
    for (const host of await sample('Visibility').locator('cem-icon-button').all()) {
        assert.equal(await host.isHidden(), await host.getAttribute('hidden') !== null);
    }
}

async function verifyIconLinkExamples(page) {
    await verifyIconButtonSamples(page);
    const material = page.locator('cem-demo-element[legend="Icon links"] cem-icon-button[image="recycling"] a');
    await material.waitFor();
    assert.equal(await material.getAttribute('href'), '#icon-link-target');
    assert.equal(await material.getAttribute('aria-label'), null);
    await material.locator('cem-icon .material-icons').waitFor();
    assert(await material.locator('cem-icon .material-icons').count() === 1);
    const embedded = page.locator('cem-demo-element[legend="Embedded icon attributes"]');
    await embedded.locator('cem-icon-button[image="★"][size="small"] cem-icon .unicode').waitFor();
    assert.equal(await embedded.locator('cem-icon-button[image=""] cem-icon [part="icon"]').count(), 0);
    assert.equal(await embedded.locator('cem-icon-button[image="favorite"] button').getAttribute('aria-label'), 'Favorite');
    assert.equal(await embedded.locator('cem-icon-button[href] cem-icon strong').textContent(), 'Projected favorite');
    assert(await page.locator('cem-demo-element[legend="Icon links"] cem-icon-button[image="fas fa-cloud-upload-alt"] i.fas.fa-cloud-upload-alt').count() === 1);
    await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Icon links"] cem-icon-button[image^="data:image/"] img')?.naturalWidth > 0);
    assert.equal(await page.locator('cem-demo-element[legend="Icon links"] cem-icon-button[image^="data:image/"] img').getAttribute('alt'), '');
    assert.equal(await page.locator('cem-demo-element[legend="Icon links"] cem-icon-button[image^="data:image/"] cem-icon [part="content"]').evaluate(node => getComputedStyle(node).flexDirection), 'column');
    const original = page.url();
    await material.click();
    assert.equal(new URL(page.url()).hash, '#icon-link-target');
    await page.evaluate(url => history.replaceState(null, '', url), original);
    await material.focus();
    await material.press('Enter');
    assert.equal(new URL(page.url()).hash, '#icon-link-target');
    await page.evaluate(url => history.replaceState(null, '', url), original);
    const disabled = page.locator('cem-demo-element[legend="Icon links"] cem-icon-button[disabled] a');
    assert.equal(await disabled.getAttribute('href'), null);
    assert.equal(await disabled.getAttribute('aria-disabled'), 'true');
    const before = await disabled.evaluate(node => window.cemPlaygroundRuntime.snapshotInstance(node.parentElement).eventPayloads);
    await disabled.evaluate(node => node.click());
    assert.equal(page.url(), original);
    assert.deepEqual(await disabled.evaluate(node => window.cemPlaygroundRuntime.snapshotInstance(node.parentElement).eventPayloads), before);
    await page.emulateMedia({ forcedColors: 'active' });
    await material.press('Tab');
    await material.focus();
    assert.equal(await material.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
    assert.equal(await material.evaluate(node => getComputedStyle(node).boxShadow), 'none');
    await page.emulateMedia({ forcedColors: 'none' });
}

async function verifyIcon(url) {
    const context = await browser.newContext();
    const page = await context.newPage();
    try {
        await page.goto(url);
        await page.locator('#icon-preview [role="img"]').waitFor();
        assert.equal(await page.locator('#icon-preview .material-icons').textContent(), 'settings');
        await verifyIconLabels(page);
        await page.getByRole('textbox', { name: 'label', exact: true }).fill('Visible fallback');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="content"]')?.textContent?.replace(/\s/g, "") === 'settingsVisiblefallback');
        await page.getByRole('textbox', { name: 'content', exact: true }).fill('Projected text');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="content"]')?.textContent?.replace(/\s/g, "") === 'settingsProjectedtext');
        await page.getByRole('textbox', { name: 'label', exact: true }).fill('Updated fallback');
        await page.getByRole('textbox', { name: 'content', exact: true }).fill('');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="content"]')?.textContent?.replace(/\s/g, "") === 'settingsUpdatedfallback');
        await page.getByRole('textbox', { name: 'image', exact: true }).fill('★');
        await page.waitForFunction(() => document.querySelector('#icon-preview .unicode')?.textContent === '★');
        await page.getByRole('radio', { name: 'large', exact: true }).check();
        await page.waitForFunction(() => {
            const glyph = document.querySelector('#icon-preview [part="icon"]');
            return glyph && parseFloat(getComputedStyle(glyph).fontSize) === 3 * parseFloat(getComputedStyle(document.documentElement).fontSize);
        });
        await page.getByRole('radio', { name: 'column', exact: true }).check();
        await page.waitForFunction(() => getComputedStyle(document.querySelector('#icon-preview [part="content"]')).flexDirection === 'column');
        await page.getByRole('radio', { name: 'empty', exact: true }).check();
        await page.waitForFunction(() => !document.querySelector('#icon-preview [part="icon"]'));
        await page.locator('fieldset').filter({ has: page.locator('legend', { hasText: /^image-mode$/ }) }).getByRole('radio', { name: 'absent', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('#icon-preview .material-icons')?.textContent === 'circle');
        await page.getByRole('textbox', { name: 'aria-label', exact: true }).fill('');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="icon"]')?.getAttribute('aria-hidden') === 'true');
        await page.waitForFunction(() => {
            const image = document.querySelector('cem-demo-element cem-icon[image^="data:image/svg+xml,"] img');
            return image?.naturalWidth > 0;
        });
        const source = await page.getByRole('link', { name: 'Canonical XHTML', exact: true }).getAttribute('href');
        const response = await page.request.get(new URL(source, page.url()).href);
        assert(response.ok());
        assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/src/components/cem-icon/cem-icon.xhtml'), 'utf8'));
        assert.deepEqual(await page.evaluate(() => window.cemPlaygroundRuntime.diagnosticsFor(document.querySelector('#icon-preview'))), []);
        console.log('Icon playground verified: ' + url);
    } finally { await context.close(); }
}

async function verifyLegacyIconExamples(page) {
    const cases = ['direction', 'size', 'unicode', 'material', 'fontawesome', 'module-image', 'color'];
    assert.deepEqual(await page.locator('[data-legacy-icon-case]').evaluateAll(nodes => nodes.map(node => node.dataset.legacyIconCase)), cases);
    const sources = {
        unicode: ['🚀', '👁', '🎄', '😭', '🔥', '💀', '🛒', '✨', '😊', '😂', '⭐', '🫶', '🎁', '✅'],
        material: ['recycling', 'shopping_cart', 'search', 'home', 'menu', 'close', 'check_circle', 'favorite', 'add', 'star', 'chevron_right', 'logout', 'add_circle', 'cancel'],
        fontawesome: ['fab fa-github', 'fas fa-bookmark', 'fab fa-discord', 'fab fa-android', 'fab fa-apple', 'far fa-user', 'far fa-envelope', 'fas fa-thumbs-up', 'far fa-thumbs-down', 'far fa-star', 'fas fa-star', 'fas fa-location-arrow', 'fas fa-map-marker', 'fas fa-map-marked-alt', 'fas fa-globe'],
    };
    for (const [name, expected] of Object.entries(sources)) {
        const icons = page.locator(`[data-legacy-icon-case="${name}"] > [slot="demo"] cem-icon`);
        assert.deepEqual(await icons.evaluateAll(nodes => nodes.map(node => node.getAttribute('image'))), expected);
        await page.waitForFunction(selector => [...document.querySelectorAll(selector)].every(node => node.querySelector('[part="glyph"]')), `[data-legacy-icon-case="${name}"] > [slot="demo"] cem-icon`);
    }
    const wrappers = await page.locator('[data-legacy-icon-case="fontawesome"] > [slot="demo"] cem-icon [part="icon"]').evaluateAll(nodes => nodes.map(node => ({
        classes: [...node.classList],
        before: getComputedStyle(node, '::before').content,
    })));
    for (const wrapper of wrappers) {
        assert(!wrapper.classes.some(name => /^(?:fa[brs]?|fa-.*)$/.test(name)), 'Font Awesome classes belong only to the glyph');
        assert(['none', 'normal'].includes(wrapper.before), 'Icon wrapper must not generate a second glyph');
    }
    const directions = await page.locator('[data-legacy-icon-case="direction"] > [slot="demo"] cem-icon [part="content"]').evaluateAll(nodes => nodes.map(node => getComputedStyle(node).flexDirection));
    assert.deepEqual(directions, ['row', 'column', 'row']);
    const sizes = await page.locator('[data-legacy-icon-case="size"] > [slot="demo"] cem-icon [part="glyph"]').evaluateAll(nodes => nodes.map(node => parseFloat(getComputedStyle(node).fontSize) / parseFloat(getComputedStyle(document.documentElement).fontSize)));
    assert.deepEqual(sizes, [1, 2, 3]);
    const colors = await page.locator('[data-legacy-icon-case="color"] > [slot="demo"] cem-icon').evaluateAll(nodes => nodes.map(node => ({
        host: getComputedStyle(node).color,
        glyph: getComputedStyle(node.querySelector('[part="glyph"]')).color,
        token: node.style.color,
    })));
    assert.deepEqual(colors.map(color => color.token), ['var(--cem-palette-danger)', 'var(--cem-palette-calm)', 'var(--cem-palette-trust)']);
    for (const color of colors) assert.equal(color.host, color.glyph);
    assert.equal(new Set(colors.map(color => color.host)).size, 3);
    await page.waitForFunction(() => document.querySelector('[data-legacy-icon-case="module-image"] cem-icon[image$="/wc-square.svg"] img')?.naturalWidth > 0);
    const image = page.locator('[data-legacy-icon-case="module-image"] cem-icon[image$="/wc-square.svg"] img');
    const response = await page.request.get(await image.getAttribute('src'));
    assert(response.ok());
    assert.equal(await response.text(), await readFile(join(root, 'packages/cem-components/playgrounds/assets/wc-square.svg'), 'utf8'));
    assert.equal(await page.locator('[data-legacy-icon-case="module-image"] cem-icon[image^="https://unpkg.com/"] img').getAttribute('src'), 'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg');
    for (const selector of ['[data-legacy-icon-case="unicode"] a', '[data-legacy-icon-case="material"] a', '[data-legacy-icon-case="fontawesome"] a']) {
        assert(await page.locator(selector).count() > 0);
        assert(await page.locator(selector).evaluateAll(nodes => nodes.every(node => !node.closest('[aria-hidden="true"]'))));
    }
    assert.deepEqual(await page.evaluate(() => [...document.querySelectorAll('[data-legacy-icon-case] cem-icon, [data-legacy-icon-case="module-image"] cem-element')].flatMap(node => window.cemPlaygroundRuntime.diagnosticsFor(node))), []);
}

async function verifyIconLabels(page) {
    await page.waitForFunction(() => document.querySelector('cem-demo-element cem-icon[label="Favorite"] [part="content"]')?.textContent?.replace(/\s/g, "") === '★Favorite');
    assert.equal(await page.locator('cem-demo-element cem-icon[label="Favorite"] [part="icon"]').getAttribute('aria-hidden'), 'true');
    assert.equal(await page.locator('cem-demo-element cem-icon[label="Fallback"] [part="content"]').evaluate(node => node.textContent.replace(/\s/g, '')), '★Details');
    assert.equal(await page.locator('cem-demo-element cem-icon[label="Fallback"] a strong').textContent(), 'Details');
    assert.equal(await page.locator('cem-demo-element cem-icon[label="Fallback"] a').evaluate(node => node.closest('[role="img"], [aria-hidden="true"]')), null);
    assert.equal(await page.locator('cem-demo-element cem-icon[image=""][label="Text without glyph"] [part="content"]').evaluate(node => node.textContent.trim()), 'Text without glyph');
    assert.equal(await page.locator('cem-demo-element cem-icon[image=""][label="Text without glyph"] [part="icon"]').count(), 0);
}

async function verifyIconImageExamples(page) {
    assert.equal(await page.locator('cem-demo-element[legend="Sources, naming, sizes, and content"]').count(), 0);
    const inline = page.locator('section[aria-label="Image examples"] cem-demo-element cem-icon[image^="data:image/svg+xml,"] img');
    await inline.waitFor();
    assert((await inline.getAttribute('src')).startsWith('data:image/svg+xml,'));
    await page.waitForFunction(() => document.querySelector('section[aria-label="Image examples"] cem-demo-element cem-icon[image^="data:image/svg+xml,"] img')?.naturalWidth > 0);
    assert.equal(await page.locator('section[aria-label="Image examples"] cem-demo-element cem-icon[image^="https://unpkg.com/"] img').getAttribute('src'), 'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg');
    assert.equal(await page.locator('section[aria-label="Image examples"] cem-demo-element').count(), 2);
    const hidden = page.locator('cem-demo-element[legend="Hidden attribute presence"] cem-icon[hidden]');
    assert.equal(await hidden.count(), 3);
    assert(await hidden.evaluateAll(nodes => nodes.every(node => getComputedStyle(node).display === 'none')));
}

async function verifyNavigation(baseUrl) {
    const folders = await readdir(join(root, 'packages/cem-components/src/components'), { withFileTypes: true });
    const pages = folders.filter(entry => entry.isDirectory()).flatMap(({ name }) => [
        [name + '.html', name], [name + '-gallery.html', name],
    ]);
    pages.push(['index.html', 'Components'], ['cem-bundle.html', 'CEM release bundle']);
    const context = await browser.newContext();
    const page = await context.newPage();
    try {
        for (const [file] of pages) {
            const source = await readFile(join(root, 'packages/cem-components/playgrounds', file), 'utf8');
            const title = source.match(/<cem-gallery-nav heading="([^"]*)"/)[1];
            await page.goto(new URL(file, baseUrl).href);
            const nav = page.getByRole('navigation', { name: 'Gallery navigation' });
            await nav.waitFor();
            assert.equal(await nav.getByRole('heading', { level: 1 }).textContent(), title);
            assert.equal(await page.locator('h1').count(), 1);
            const index = nav.getByRole('link', { name: 'Component playground', exact: true });
            await index.waitFor();
            assert.equal(new URL(await index.getAttribute('href'), page.url()).href, new URL('index.html', baseUrl).href);
            const theme = nav.getByRole('button', { name: 'Switch theme (coming soon)', exact: true });
            await theme.waitFor();
            assert.equal(await theme.isDisabled(), true);
            await index.click();
            assert.equal(page.url(), new URL('index.html', baseUrl).href);
            await page.getByRole('heading', { name: 'CEM component playground', exact: true }).waitFor();
        }
    } finally {
        await context.close();
    }
}
