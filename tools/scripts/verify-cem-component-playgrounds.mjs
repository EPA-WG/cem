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
    await verifyThemeSwitch(`${origin}/packages/cem-components/playgrounds/cem-theme-switch.html`);
    await verifySelect(`${origin}/packages/cem-components/playgrounds/cem-select.html`);
    for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
    for (const tag of ['cem-field', 'cem-text-field']) await verifyField(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
    await verifyBundle(`${origin}/packages/cem-components/playgrounds/cem-bundle.html`);
    await verifyPendingTheme(`${origin}/packages/cem-theme/dist/lib/css-generators/cem-colors.html`);
    for (const [folder, name] of [['cem-components','cem-components'], ['cem-elements','cem-elements'], ['cem-demo-element','cem-demo-element'], ['cem-theme','cem-theme'], ['cem-ml-npm','cem-ml']]) {
        const output = JSON.parse(execFileSync('npm', ['pack', '--json', '--pack-destination', temporary], { cwd: join(root, 'packages', folder), encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } }));
        const target = join(temporary, 'installed/node_modules/@epa-wg', name);
        await mkdir(target, { recursive: true });
        execFileSync('tar', ['-xzf', join(temporary, output[0].filename), '--strip-components=1', '-C', target]);
    }
    await verify(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-action.html`);
    await verifyThemeSwitch(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-theme-switch.html`);
    await verifySelect(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-select.html`);
    for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
    for (const tag of ['cem-field', 'cem-text-field']) await verifyField(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
    await verifyBundle(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-bundle.html`);
    await verifyPendingTheme(`${origin}/installed/node_modules/@epa-wg/cem-theme/dist/lib/css-generators/cem-colors.html`);
    console.log('Action, field, text-field, icon-button, menu-item, select, theme-switch and bundle playgrounds verified from source and isolated package archives.');
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
    assert.equal(await page.locator('#gallery-fallback button').innerText(), 'Fallback label');
    assert.equal(await page.locator('#gallery-icon button').getAttribute('aria-label'), 'Add item');
    for (const size of ['small', 'medium', 'large', 'x-large', 'xx-large']) {
        const sample = page.locator(`#gallery-size-${size}`);
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
    assert.equal(await page.locator('#gallery-size-default').getAttribute('size'), null);
    assert.equal(await page.locator('#gallery-selected-false button').getAttribute('aria-pressed'), 'true');
    assert.equal(await page.locator('#gallery-selectable-false button').getAttribute('aria-pressed'), 'false');
    await page.locator('#gallery-selectable-false button').click();
    assert.equal(await page.locator('#gallery-selectable-false button').getAttribute('aria-pressed'), 'false');
    assert.equal(await page.locator('#gallery-command button').getAttribute('aria-pressed'), null);
    await page.locator('#gallery-comfortable button').click();
    await page.waitForFunction(() => document.querySelector('#gallery-comfortable button')?.getAttribute('aria-pressed') === 'true' && document.querySelector('#gallery-compact button')?.getAttribute('aria-pressed') === 'false');
    await verifyThemeControls(page);
    assert.equal(await page.locator('#gallery-comfortable button').getAttribute('aria-pressed'), 'true');
    assert((await page.locator('cem-demo-element[legend="Controlled selection"] [slot=text]').innerText()).includes('gallery-selected-false'));
    assert(await page.locator('#gallery-disabled-false button').isDisabled());
    assert(await page.locator('#gallery-disabled-pending button').isDisabled());
    assert.equal(await page.locator('#gallery-pending button').getAttribute('aria-busy'), 'true');
    assert(await page.locator('#gallery-hidden').isHidden());
    await page.locator('#gallery-disclosure button').click();
    await page.waitForFunction(() => document.querySelector('#gallery-disclosure button')?.getAttribute('aria-expanded') === 'true' && !document.querySelector('#gallery-hidden')?.hasAttribute('hidden'));
    await page.locator('#gallery-disclosure button').click();
    await page.waitForFunction(() => document.querySelector('#gallery-disclosure button')?.getAttribute('aria-expanded') === 'false' && document.querySelector('#gallery-hidden')?.hasAttribute('hidden'));
    assert.equal(await page.locator('#gallery-until-found').getAttribute('hidden'), 'until-found');
    await page.locator('#gallery-form').evaluate(form => {
        window.gallerySubmissions = [];
        form.addEventListener('submit', event => {
            event.preventDefault();
            window.gallerySubmissions.push([...new FormData(form, event.submitter)]);
        });
    });
    const title = page.getByRole('textbox', { name: 'Example title', exact: true });
    await title.fill('');
    await page.locator('#gallery-submit button').click();
    assert.equal(await page.evaluate(() => window.gallerySubmissions.length), 0);
    await title.fill('Ready');
    await page.locator('#gallery-submit button').click();
    assert.deepEqual(await page.evaluate(() => window.gallerySubmissions[0]), [['title', 'Ready'], ['intent', 'save']]);
    await page.locator('#gallery-button button').click();
    assert.equal(await page.evaluate(() => window.gallerySubmissions.length), 1);
    await page.locator('#gallery-reset button').click();
    assert.equal(await title.inputValue(), 'Initial title');
    await title.fill('');
    const external = page.locator('#gallery-external button');
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
        assert.equal(await page.getByRole('textbox', { name: 'Label', exact: true }).evaluate(node => getComputedStyle(node).boxShadow), 'none');
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
        await page.locator('#bundle-action button').waitFor();
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
            for (const tag of ['cem-action', 'cem-select', 'cem-theme-switch', 'cem-icon-button', 'cem-menu-item', 'cem-field', 'cem-text-field']) {
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
        await page.waitForSelector('#command-configured button');
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
        assert(await page.locator('#command-disabled button').isDisabled());
        assert(await page.locator('#command-hidden').isHidden());
        assert.equal(await page.locator('#command-expanded button').getAttribute('aria-expanded'), 'true');
        assert.equal(await page.locator('#command-configured button').getAttribute('aria-expanded'), 'false');
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
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
}


async function verifyField(url, tag) {
    const context = await browser.newContext();
    const page = await context.newPage();
    try {
        await page.goto(url);
        await page.waitForSelector('#field-preview input');
        const input = page.locator('#field-preview input');
        await input.fill('edited');
        await page.getByRole('button', { name: 'Reset', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('#field-preview input')?.value === 'initial');
        assert.deepEqual(await page.evaluate(() => new FormData(document.querySelector('#field-form')).getAll('account')), ['initial']);
        for (const attribute of ['disabled', 'required', 'readonly', 'busy']) {
            const group = page.getByRole('radiogroup', { name: attribute, exact: true });
            await group.getByRole('radio', { name: 'false', exact: true }).check();
            await page.waitForFunction(attribute => {
                const control = document.querySelector('#field-preview input');
                return attribute === 'busy' ? control?.getAttribute('aria-busy') === 'true' : control?.hasAttribute(attribute);
            }, attribute);
            await group.getByRole('radio', { name: 'absent', exact: true }).check();
            await page.waitForFunction(attribute => {
                const control = document.querySelector('#field-preview input');
                return !control?.hasAttribute(attribute === 'busy' ? 'aria-busy' : attribute);
            }, attribute);
        }
        await page.getByRole('radiogroup', { name: 'busy', exact: true }).getByRole('radio', { name: 'true', exact: true }).check();
        await page.waitForFunction(() => getComputedStyle(document.querySelector('#field-preview input')).boxShadow !== 'none');
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
