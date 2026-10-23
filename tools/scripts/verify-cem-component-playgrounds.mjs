import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { readFile, readdir, mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises';
import { resolve, extname, join } from 'node:path';
import { tmpdir, release } from 'node:os';
import { chromium, firefox, webkit } from 'playwright';
import { verifyFieldsetPage } from '../cem-components/fieldset-page-checks.mjs';

const root = resolve(import.meta.dirname, '../..');
const temporary = await mkdtemp(join(tmpdir(), 'cem-action-playground-'));
const mime = { '.svg': 'image/svg+xml', '.html': 'text/html', '.xhtml': 'application/xhtml+xml', '.js': 'text/javascript', '.json': 'application/json', '.css': 'text/css', '.wasm': 'application/wasm' };
const nativeDatalistOnly = process.argv.includes('--native-datalist-only');
const historyOnly = process.argv.includes('--history-only');
const evidenceOnly = nativeDatalistOnly || historyOnly;
const suggestionsOnly = evidenceOnly || process.argv.includes('--suggestions-only');
const nativeBrowser = process.argv.find(value => value.startsWith('--browser='))?.split('=')[1] ?? process.argv.find(value => value.startsWith('--native-browser='))?.split('=')[1] ?? 'chromium';
const headed = process.argv.includes('--headed');
const evidenceOutput = process.argv.find(value => value.startsWith('--evidence-output='))?.slice('--evidence-output='.length);
const nativeEvidence = [];
const historyEvidence = [];
const navigationOnly = process.argv.includes('--navigation-only');
const actionOnly = process.argv.includes('--action-only');
const iconsOnly = process.argv.includes('--icons-only');
const surfacesOnly = process.argv.includes('--surfaces-only');
const sheetOnly = process.argv.includes('--sheet-only');
const fieldsetOnly = process.argv.includes('--fieldset-only');
const navigationContentOnly = process.argv.includes('--navigation-content-only');
const passiveOnly = process.argv.includes('--passive-only');
const passiveComponent = process.argv.find(value => value.startsWith('--passive-component='))?.slice('--passive-component='.length);
const passiveTags = ['cem-option', 'cem-option-group', 'cem-badge', 'cem-avatar', 'cem-media-preview', 'cem-tree-item', 'cem-app-bar', 'cem-tab', 'cem-step'].filter(tag => !passiveComponent || tag === passiveComponent);
if (!passiveTags.length) throw new Error(`Unknown passive component: ${passiveComponent}`);
const feedbackOnly = process.argv.includes('--feedback-only');
const disclosureDataOnly = process.argv.includes('--disclosure-data-only');
const layoutOnly = process.argv.includes('--layout-only');
const sourceEditorOnly = process.argv.includes('--source-editor-only');
const recoveryStatusOnly = process.argv.includes('--recovery-status-only');
const timeRangeOnly = process.argv.includes('--time-range-only');
const datepickerOnly = process.argv.includes('--datepicker-only');
const autocompleteOnly = process.argv.includes('--autocomplete-only');
const checkablesOnly = process.argv.includes('--checkables-only');
const installedOnly = process.argv.includes('--installed-only');
const verifiedLocations = installedOnly ? 'isolated package archives' : 'source and isolated package archives';
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
    if (!['chromium', 'firefox', 'webkit'].includes(nativeBrowser)) throw new Error('Unknown native browser');
    if (iconsOnly && (navigationOnly || actionOnly || suggestionsOnly)) throw new Error('Choose one focused verification mode');
    if (nativeDatalistOnly && historyOnly) throw new Error('Choose one evidence mode');
    if (!evidenceOnly && (headed || nativeBrowser !== 'chromium' || evidenceOutput)) throw new Error('Browser evidence flags require --native-datalist-only or --history-only');
    browser = await ({ chromium, firefox, webkit })[nativeBrowser].launch({ headless: !headed });
    if (!installedOnly && recoveryStatusOnly) await verifyRecoveryStatus(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && sourceEditorOnly) await verifyGalleries(`${origin}/packages/cem-components/playgrounds/`, ['cem-source-editor']);
    else if (!installedOnly && fieldsetOnly) await verifyFieldset(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && passiveOnly) await verifyPassive(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && feedbackOnly) await verifyFeedback(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && disclosureDataOnly) await verifyDisclosureData(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && layoutOnly) await verifyLayout(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && navigationContentOnly) await verifyNavigationContent(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && timeRangeOnly) await verifyTimeRange(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && datepickerOnly) await verifyDatepicker(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && autocompleteOnly) await verifyAutocomplete(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && checkablesOnly) await verifyCheckables(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && sheetOnly) await verifySheet(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && surfacesOnly) await verifySurfaces(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly && iconsOnly) await verifyIcons(`${origin}/packages/cem-components/playgrounds/`);
    else if (!installedOnly) {
        if (!navigationOnly && !suggestionsOnly) {
            await verify(`${origin}/packages/cem-components/playgrounds/cem-action.html`);
            if (!actionOnly) await verifyThemeSwitch(`${origin}/packages/cem-components/playgrounds/cem-theme-switch.html`);
            if (!actionOnly) await verifySelect(`${origin}/packages/cem-components/playgrounds/cem-select.html`);
            if (!actionOnly) for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
            if (!actionOnly) for (const tag of ['cem-field', 'cem-text-field', 'cem-textarea']) await verifyField(`${origin}/packages/cem-components/playgrounds/${tag}.html`, tag);
            if (!actionOnly) await verifyIcon(`${origin}/packages/cem-components/playgrounds/cem-icon.html`);
        }
        if (!actionOnly && !suggestionsOnly) await verifyNavigation(`${origin}/packages/cem-components/playgrounds/`);
        if (!navigationOnly && !suggestionsOnly) {
            if (!actionOnly) await verifyGalleries(`${origin}/packages/cem-components/playgrounds/`);
            if (!actionOnly) await verifyBundle(`${origin}/packages/cem-components/playgrounds/cem-bundle.html`);
            if (!actionOnly) await verifyPendingTheme(`${origin}/packages/cem-theme/dist/lib/css-generators/cem-colors.html`);
        }
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySurfaces(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySheet(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyDatepicker(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyTimeRange(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyNavigationContent(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyLayout(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyDisclosureData(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyFeedback(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyPassive(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyAutocomplete(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyCheckables(`${origin}/packages/cem-components/playgrounds/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySuggestions(`${origin}/packages/cem-components/playgrounds/`);
        if (nativeDatalistOnly) await verifyNativeDatalist(`${origin}/packages/cem-components/playgrounds/`, 'source');
        if (historyOnly) await verifyNativeHistory(`${origin}/packages/cem-components/playgrounds/`, 'source');
    }
    for (const [folder, name] of [['cem-components','cem-components'], ['cem-elements','cem-elements'], ['cem-demo-element','cem-demo-element'], ['cem-theme','cem-theme'], ['cem-ml-npm','cem-ml']]) {
        const output = JSON.parse(execFileSync('npm', ['pack', '--json', '--pack-destination', temporary], { cwd: join(root, 'packages', folder), encoding: 'utf8', env: { ...process.env, npm_config_update_notifier: 'false' } }));
        const target = join(temporary, 'installed/node_modules/@epa-wg', name);
        await mkdir(target, { recursive: true });
        execFileSync('tar', ['-xzf', join(temporary, output[0].filename), '--strip-components=1', '-C', target]);
    }
    if (recoveryStatusOnly) await verifyRecoveryStatus(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (sourceEditorOnly) await verifyGalleries(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`, ['cem-source-editor']);
    else if (fieldsetOnly) await verifyFieldset(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (passiveOnly) await verifyPassive(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (feedbackOnly) await verifyFeedback(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (disclosureDataOnly) await verifyDisclosureData(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (layoutOnly) await verifyLayout(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (navigationContentOnly) await verifyNavigationContent(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (timeRangeOnly) await verifyTimeRange(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (datepickerOnly) await verifyDatepicker(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (autocompleteOnly) await verifyAutocomplete(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (checkablesOnly) await verifyCheckables(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (sheetOnly) await verifySheet(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (surfacesOnly) await verifySurfaces(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else if (iconsOnly) await verifyIcons(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
    else {
        if (!navigationOnly && !suggestionsOnly) {
            await verify(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-action.html`);
            if (!actionOnly) await verifyThemeSwitch(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-theme-switch.html`);
            if (!actionOnly) await verifySelect(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-select.html`);
            if (!actionOnly) for (const tag of ['cem-icon-button', 'cem-menu-item']) await verifyCommand(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
            if (!actionOnly) for (const tag of ['cem-field', 'cem-text-field', 'cem-textarea']) await verifyField(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/${tag}.html`, tag);
            if (!actionOnly) await verifyIcon(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-icon.html`);
        }
        if (!actionOnly && !suggestionsOnly) await verifyNavigation(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!navigationOnly && !suggestionsOnly) {
            if (!actionOnly) await verifyGalleries(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
            if (!actionOnly) await verifyBundle(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/cem-bundle.html`);
            if (!actionOnly) await verifyPendingTheme(`${origin}/installed/node_modules/@epa-wg/cem-theme/dist/lib/css-generators/cem-colors.html`);
        }
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySurfaces(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySheet(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyDatepicker(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyTimeRange(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyNavigationContent(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyLayout(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyDisclosureData(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyFeedback(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyPassive(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyAutocomplete(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifyCheckables(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (!evidenceOnly && !navigationOnly && !actionOnly) await verifySuggestions(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`);
        if (nativeDatalistOnly) {
            await verifyNativeDatalist(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`, 'installed');
            const report = { schema: 'cem-native-datalist-evidence-v1', recordedAt: new Date().toISOString(), revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), browser: nativeBrowser, version: browser.version(), headed, platform: process.platform, osRelease: release(), observations: nativeEvidence, actualPickerSelection: 'unconfirmed', physicalIme: 'not-run', mobileKeyboard: 'not-run', assistiveTechnology: 'not-run' };
            if (evidenceOutput) await writeFile(resolve(evidenceOutput), JSON.stringify(report, null, 2) + '\n');
            console.log(JSON.stringify(report, null, 2));
        }
        else if (historyOnly) {
            await verifyNativeHistory(`${origin}/installed/node_modules/@epa-wg/cem-components/dist/`, 'installed');
            const report = { schema: 'cem-native-history-evidence-v1', recordedAt: new Date().toISOString(), revision: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(), browser: nativeBrowser, version: browser.version(), headed, platform: process.platform, osRelease: release(), observations: historyEvidence };
            if (evidenceOutput) await writeFile(resolve(evidenceOutput), JSON.stringify(report, null, 2) + '\n');
            console.log(JSON.stringify(report, null, 2));
        }
    }
    if (disclosureDataOnly) console.log(`Disclosure, sorting, pagination and passive table delivery verified from ${verifiedLocations}.`);
    else if (layoutOnly) console.log(`Passive layout, content state, property controls, galleries and bundle verified from ${verifiedLocations}.`);
    else if (navigationContentOnly) console.log(`Navigation and content properties, galleries and bundle verified from ${verifiedLocations}.`);
    else if (timeRangeOnly) console.log(`Timepicker and slider properties, native forms, galleries and bundle verified from ${verifiedLocations}.`);
    else if (datepickerOnly) console.log(`Datepicker property controls, native forms, calendar, galleries and bundle verified from ${verifiedLocations}.`);
    else if (autocompleteOnly) console.log(`Autocomplete property controls, galleries, live payload, forms and bundle verified from ${verifiedLocations}.`);
    else if (checkablesOnly) console.log(`Checkbox, radio and switch playgrounds, galleries, native forms, bundle and forced colors verified from ${verifiedLocations}.`);
    else if (fieldsetOnly) console.log(`Fieldset properties, native forms, five themes, narrow/RTL, forced colors, galleries and bundle verified from ${verifiedLocations}.`);
    else if (sheetOnly) console.log(`Sheet region playground, gallery, bundle and forced colors verified from ${verifiedLocations}.`);
    else if (surfacesOnly) console.log(`Native dialog, dialog-shell and tooltip playgrounds, galleries, bundle and forced colors verified from ${verifiedLocations}.`);
    else if (iconsOnly) console.log(`Icon and icon-button playgrounds, galleries and release bundle verified from ${verifiedLocations}.`);
    else if (suggestionsOnly && !evidenceOnly) console.log(`Suggestions playground and gallery verified from ${verifiedLocations}.`);
    else if (actionOnly) console.log(`Action playground and gallery verified from ${verifiedLocations}.`);
    else if (navigationOnly) console.log(`Gallery navigation verified from ${verifiedLocations}.`);
    else if (recoveryStatusOnly) console.log(`Recovery status property playground, gallery, inherited themes and forced colors verified from ${verifiedLocations}.`);
    else if (sourceEditorOnly) console.log(`Source editor property playground and gallery verified from ${verifiedLocations}.`);
    else if (passiveOnly) console.log(`${passiveTags.join(', ')} playgrounds verified from ${verifiedLocations}.`);
    else if (!evidenceOnly) console.log(`Action, field, text-field, textarea, icon, icon-button, menu-item, select, suggestions, theme-switch and bundle playgrounds verified from ${verifiedLocations}.`);
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
        return cards.length === 11 && cards.every(card => card.getAttribute('data-state') === 'ready');
    });
    const declaration = await readFile(join(root, 'packages/cem-components/src/components/cem-action/cem-action.xhtml'), 'utf8');
    const implemented = [...new Set([
        ...[...declaration.matchAll(/\{attribute @name=([\w-]+)(?=\s*\|)/g)].map(match => match[1]),
        ...[...declaration.matchAll(/datadom\.attributes\.([\w-]+)/g)].map(match => match[1]),
        ...[...declaration.matchAll(/:scope[^\s{]*?\[([\w-]+)/g)].map(match => match[1]),
        'class', 'command-target', 'interaction', 'context-key',
    ])].sort();
    await page.locator('section[aria-label="Implemented attributes"] [data-action-attribute]').first().waitFor({ state: 'attached' });
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
    await page.locator('cem-demo-element[legend="Controlled selection"]').evaluate(card => {
        window.retainedLayoutChoice = { card, declaration: card.querySelector('[slot=demo] cem-element'),
            instance: card.querySelector('[data-cem-anonymous-instance]'), group: card.querySelector('[aria-label="Layout choice"]') };
    });
    await page.locator('cem-demo-element[legend="Controlled selection"] [slot=demo] cem-element').evaluate(async node => { await window.cemPlaygroundRuntime.whenRenderSettled(node); });
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
    const sample = legend => page.locator(`cem-demo-element[legend="${legend}"]`);
    const popover = sample('Native popover commands').locator('[popover]');
    await sample('Native popover commands').getByRole('button', { name: 'Toggle native details' }).click();
    assert(await popover.evaluate(node => node.matches(':popover-open')));
    await popover.getByRole('button', { name: 'Close', exact: true }).click();
    assert(!await popover.evaluate(node => node.matches(':popover-open')));
    await sample('Native modal dialog').getByRole('button', { name: 'Open native task' }).click();
    assert(await sample('Native modal dialog').locator('dialog').evaluate(node => node.matches(':modal')));
    await page.keyboard.press('Escape');
    await sample('Native modal dialog').locator('dialog').waitFor({ state: 'hidden' });
    await sample('Native popover targets').getByRole('button', { name: 'Toggle native hint' }).click();
    assert(await sample('Native popover targets').locator('[popover]').evaluate(node => node.matches(':popover-open')));
    await page.keyboard.press('Escape');
    const scoped = sample('Local command scopes');
    await scoped.getByRole('button', { name: 'First details', exact: true }).click();
    await scoped.getByRole('button', { name: 'Second details', exact: true }).click();
    assert.deepEqual(await scoped.locator('[popover]').evaluateAll(nodes => nodes.map(node => node.matches(':popover-open'))), [true, true]);
    await scoped.getByRole('button', { name: 'First details', exact: true }).click();
    await scoped.getByRole('button', { name: 'Second details', exact: true }).click();
    await verifyThemeControls(page);
    assert.equal(await page.locator('[role="group"][aria-label="Layout choice"] cem-action:nth-of-type(2) button').getAttribute('aria-pressed'), 'true',
        'Native activation followed by theme projection must retain the anonymous choice slice');
    assert(await page.evaluate(() => {
        const retained = window.retainedLayoutChoice;
        const card = document.querySelector('cem-demo-element[legend="Controlled selection"]');
        return card === retained.card && card.querySelector('[slot=demo] cem-element') === retained.declaration
            && card.querySelector('[data-cem-anonymous-instance]') === retained.instance
            && card.querySelector('[aria-label="Layout choice"]') === retained.group;
    }), 'Native activation and theme projection must retain the unrelated demo owners');
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

async function choosePageTheme(page, theme) {
    const control = page.locator('cem-gallery-page > cem-gallery-nav[part=navigation] cem-theme-switch[part=theme]');
    await control.locator('[part=trigger] button').click();
    await control.locator(`cem-menu cem-icon-button[value="cem-theme-${theme}"] button`).click();
    await page.waitForFunction(value => document.querySelector('cem-gallery-page')?.getAttribute('data-theme') === value, `cem-theme-${theme}`);
}

async function verifyThemeControls(page) {
    for (const theme of ['dark', 'contrast-dark', 'native']) await choosePageTheme(page, theme);
    await page.emulateMedia({ colorScheme: 'dark' });
    const nativeDark = await page.locator('cem-gallery-page').evaluate(node => getComputedStyle(node).backgroundColor);
    await page.emulateMedia({ colorScheme: 'light' });
    const nativeLight = await page.locator('cem-gallery-page').evaluate(node => getComputedStyle(node).backgroundColor);
    assert.notEqual(nativeDark, nativeLight);
    for (const theme of ['contrast-light', 'light']) await choosePageTheme(page, theme);
}

async function verifyThemeSwitch(url) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => { if (response.status() >= 400) errors.push(`${response.status()} ${response.url()}`); });
    try {
        await page.goto(url);
        await page.getByRole('navigation', { name: 'Gallery navigation' }).getByRole('button', { name: 'Change theme', exact: true }).waitFor();
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
            }, { label, option: label === 'Native command' ? ({ 'Toggle popover': 'popover', 'Open modal dialog': 'dialog', None: 'none' })[option] : option.toLowerCase() });
        };
        assert.equal(await page.getByRole('combobox').count(), 0);
        assert.equal(await page.getByRole('radiogroup').count(), 9);
        await page.waitForFunction(() => [...document.querySelectorAll('cem-radio')].every(host => host.querySelector('input[type=radio]')), undefined, { timeout: 30000 });
        assert.equal(await page.getByRole('radio').count(), 30);
        await choose('Native command', 'Toggle popover');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('commandfor') === 'action-preview-popup');
        await page.locator('#action-preview button').click();
        assert(await page.locator('#action-preview-popup').evaluate(node => node.matches(':popover-open')));
        await page.keyboard.press('Escape');
        await choose('Native command', 'Open modal dialog');
        await page.waitForFunction(() => document.querySelector('#action-preview button')?.getAttribute('command') === 'show-modal');
        await page.locator('#action-preview button').click();
        assert(await page.locator('#action-preview-dialog').evaluate(node => node.matches(':modal')));
        await page.keyboard.press('Escape');
        await page.locator('#action-preview-dialog').waitFor({ state: 'hidden' });
        await choose('Native command', 'None');
        await page.waitForFunction(() => !document.querySelector('#action-preview button')?.hasAttribute('commandfor'));
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
        for (const mode of ['Dark', 'Light']) {
            await choosePageTheme(page, `contrast-${mode.toLowerCase()}`);
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
        assert.equal(await page.locator('cem-action').count(), 12);
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
        assert.equal(await page.locator('cem-demo-element').count(), 11);
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
        await page.getByRole('button', { name: 'Bundled icon command', exact: true }).locator('cem-icon [part="glyph"]').waitFor();
        await page.locator('#bundle-select [role=combobox]').waitFor();
        await page.locator('#bundle-field input').waitFor();
        await page.locator('#bundle-text-field input').waitFor();
        await page.getByRole('group', { name: 'Grouped contact settings', exact: true }).getByRole('textbox', { name: 'Grouped display name', exact: true }).waitFor();
        assert.equal(await page.locator('#bundle-field input').inputValue(), 'Field value');
        assert.equal(await page.locator('#bundle-text-field input').inputValue(), 'Text value');
        await page.evaluate(async () => {
            for (const host of document.querySelectorAll('#bundle-action, #bundle-select, #bundle-theme, #bundle-field, #bundle-text-field')) {
                await window.cemPlaygroundRuntime.whenRenderSettled(host);
            }
        });
        for (const tag of passiveTags) await page.locator(`main > ${tag} > [part=root]`).waitFor({ state: 'attached' });
        const result = await page.evaluate(async () => {
            const runtime = window.cemPlaygroundRuntime;
            const declaration = [...document.querySelectorAll('cem-element[tag="cem-action"]')][0];
            const specifier = declaration.getAttribute('src').split('#')[0];
            const imports = JSON.parse(document.querySelector('script[type=importmap]').textContent).imports;
            const bundleUrl = new URL(imports[specifier] ?? specifier, document.baseURI).href;
            const bundle = new DOMParser().parseFromString(await (await fetch(bundleUrl)).text(), 'application/xml');
            const checks = [];
            for (const tag of [...document.querySelectorAll('cem-element[tag][src]')].filter(node => node.getAttribute('src').split('#')[0] === declaration.getAttribute('src').split('#')[0]).map(node => node.getAttribute('tag'))) {
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
            const navigationStyleCount = document.querySelectorAll('cem-element[tag="cem-gallery-nav"] > style[data-cem-declaration-style]').length;
            const pageStyleCount = document.querySelectorAll('cem-element[tag="cem-gallery-page"] > style[data-cem-declaration-style]').length;
            const duplicate = document.createElement('cem-element');
            duplicate.setAttribute('tag', 'cem-action');
            duplicate.setAttribute('src', declaration.getAttribute('src'));
            declaration.parentElement.append(duplicate);
            await runtime.whenDeclarationSettled(duplicate);
            const diagnostics = runtime.diagnosticsFor(duplicate).map(item => item.code);
            const stylesAfter = document.querySelectorAll('style[data-cem-declaration-style]').length;
            duplicate.remove();
            return { checks, invalidXml: !!bundle.querySelector('parsererror'), styleCount, navigationStyleCount, pageStyleCount, stylesAfter, diagnostics };
        });
        assert.equal(result.invalidXml, false);
        for (const check of result.checks) {
            assert.equal(check.same, true, `${check.tag}: bundle template text differs`);
            assert.equal(check.metadata, true, `${check.tag}: declaration metadata differs`);
            assert.equal(check.count, 1);
            assert.equal(check.base, check.sourceUrl);
        }
        assert.equal(result.navigationStyleCount, 1);
        assert.equal(result.pageStyleCount, 1);
        assert.equal(result.styleCount, result.checks.length + result.navigationStyleCount + result.pageStyleCount);
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
            ...[...source.matchAll(/\{attribute @name=([\w-]+)(?=\s*\|)/g)].map(match => match[1]),
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
        if (tag === 'cem-menu-item') {
            await page.getByRole('textbox', { name: 'href', exact: true }).fill('#menu-item-help');
            await page.locator('#command-preview a[href="#menu-item-help"]').waitFor();
            await page.getByRole('textbox', { name: 'aria-label', exact: true }).fill('Open help');
            await page.waitForFunction(() => document.querySelector('#command-preview a')?.getAttribute('aria-label') === 'Open help');
            await page.getByRole('textbox', { name: 'href', exact: true }).fill('');
            await page.locator('#command-preview button').waitFor();
            await page.getByRole('textbox', { name: 'aria-label', exact: true }).fill('');
            await page.waitForFunction(() => !document.querySelector('#command-preview button')?.hasAttribute('aria-label'));
        }
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

async function verifyIcons(baseUrl) {
    await verifyCommand(new URL('cem-icon-button.html', baseUrl).href, 'cem-icon-button');
    await verifyIcon(new URL('cem-icon.html', baseUrl).href);
    await verifyGalleries(baseUrl, ['cem-icon', 'cem-icon-button']);
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
}

async function verifyFieldset(baseUrl) {
    const context = await browser.newContext();
    try {
        for (const name of ['cem-fieldset.html', 'cem-fieldset-gallery.html']) {
            const page = await context.newPage();
            try {
                await page.goto(new URL(name, baseUrl).href);
                await page.getByRole('navigation', { name: 'Gallery navigation' }).waitFor();
                await verifyFieldsetPage(page, `/${name}`);
            } finally { await page.close(); }
        }
    } finally { await context.close(); }
    await verifyGalleries(baseUrl, ['cem-fieldset']);
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
}

async function verifyGalleries(baseUrl, tags) {
    const folders = await readdir(join(root, 'packages/cem-components/src/components'), { withFileTypes: true });
    for (const folder of folders.filter(entry => entry.isDirectory() && (!tags || tags.includes(entry.name)))) {
        const tag = folder.name;
        if (tag === 'cem-recovery-status') { await verifyRecoveryStatus(baseUrl); continue; }
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
            }, sampleSelector).catch(async error => {
                const missing = await page.locator(sampleSelector).evaluateAll(nodes => nodes.filter(node => !node.querySelector('[part]')).map(node => ({ html: node.outerHTML, diagnostics: window.cemPlaygroundRuntime.diagnosticsFor(node) })));
                throw new Error(`${tag} gallery samples did not render at ${page.url()}: ${JSON.stringify(missing)}`, { cause: error });
            });
            assert.equal(await page.locator('[data-gallery-theme]').count(), 5, `${tag}: five theme modes`);
            await page.waitForFunction(() => {
                const demos = [...document.querySelectorAll('cem-demo-element')];
                return demos.length > 0 && demos.every(node => node.getAttribute('data-state') === 'ready');
            }).catch(async error => {
                const failed = await page.locator('cem-demo-element').evaluateAll(nodes => nodes.filter(node => node.getAttribute('data-state') !== 'ready')
                    .map(node => ({ legend: node.getAttribute('legend'), state: node.getAttribute('data-state'), text: node.innerText.slice(-1500) })));
                throw new Error(`${tag} gallery examples failed at ${page.url()}: ${JSON.stringify(failed)}`, { cause: error });
            });
            const declaration = await readFile(join(root, 'packages/cem-components/src/components', tag, tag + '.xhtml'), 'utf8');
            // Public parameter defaults have bodies; @value instructions emit owned DOM attributes.
            const implemented = [...new Set([
                ...[...declaration.matchAll(/\{attribute @name=([\w-]+)(?=\s*\|)/g)].map(match => match[1]),
                ...[...declaration.matchAll(/datadom\.attributes\.([\w-]+)/g)].map(match => match[1]),
                'hidden', 'class',
            ])].filter(name => name !== 'data-theme');
            const documented = await page.locator('[data-gallery-attribute], [data-action-attribute]').evaluateAll(nodes => nodes.map(node => node.getAttribute('data-gallery-attribute') ?? node.getAttribute('data-action-attribute')));
            for (const attribute of implemented) assert(documented.includes(attribute), `${tag}: missing ${attribute} in gallery inventory`);
            await page.getByRole('link', { name: 'Automated stories', exact: true }).waitFor();
            if (tag === 'cem-media-preview') {
                const demo = page.locator('cem-demo-element[legend="Immutable artifact tree"]');
                const output = page.frameLocator('cem-demo-element[legend="Immutable artifact tree"] cem-media-preview iframe');
                await output.getByRole('heading', { name: 'Built site', exact: true }).waitFor();
                await page.waitForFunction(() => {
                    const frame = document.querySelector('cem-demo-element[legend="Immutable artifact tree"] cem-media-preview iframe');
                    const heading = frame?.contentDocument?.querySelector('h2'), image = frame?.contentDocument?.querySelector('img');
                    return heading && frame.contentWindow.getComputedStyle(heading).color === 'rgb(23, 90, 140)' && image?.naturalWidth === 32;
                });
                await demo.getByRole('combobox', { name: 'Output path', exact: true }).click();
                await page.getByRole('option', { name: 'posts/intro.html', exact: true }).click();
                await output.getByRole('heading', { name: 'Intro article', exact: true }).waitFor();
                await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Immutable artifact tree"] cem-media-preview iframe')?.contentDocument?.querySelector('img')?.naturalWidth === 32);
            }
            const first = page.locator(sampleSelector).first();
            const control = first.locator(['cem-nav', 'cem-tabs', 'cem-stepper', 'cem-tree', 'cem-chip', 'cem-expansion', 'cem-sort-header', 'cem-paginator'].includes(tag) ? 'button' : tag === 'cem-list' ? 'select' : tag === 'cem-menu' ? 'button, a[href]' : ['cem-dialog', 'cem-dialog-shell', 'cem-tooltip'].includes(tag) ? '[part=trigger], [slot=trigger]' : '[part="control"], input').first();
            if (!['cem-icon', 'cem-surface', 'cem-text', 'cem-stack', 'cem-grid', 'cem-divider', 'cem-card', 'cem-table', 'cem-toast', 'cem-progress', 'cem-progress-spinner', 'cem-skeleton', 'cem-alert', ...passiveTags].includes(tag)) {
                await control.hover();
                await control.focus();
            }
            if (tag === 'cem-field' || tag === 'cem-text-field') {
                const constraints = page.locator('cem-demo-element[legend="Native input constraints"]');
                const number = constraints.locator('input[type="number"]');
                await number.waitFor();
                for (const [name, value] of Object.entries({ min: '1', max: '10', step: '1', inputmode: 'numeric' })) {
                    assert.equal(await number.getAttribute(name), value, `${tag}: native ${name}`);
                }
                const text = constraints.locator('input[list]');
                for (const [name, value] of Object.entries({ minlength: '2', maxlength: '3', pattern: '[A-Z]+', autocomplete: 'off', autocapitalize: 'characters', spellcheck: 'false' })) {
                    assert.equal(await text.getAttribute(name), value, `${tag}: native ${name}`);
                }
                assert.equal(await text.evaluate(input => input.list?.options.length), 2);
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
            if (tag === 'cem-source-editor') {
                await page.waitForFunction(() => window.cemPlaygroundRuntime && document.querySelector('main > cem-element.properties'));
                await page.evaluate(async () => {
                    const runtime = window.cemPlaygroundRuntime;
                    await runtime.whenDeclarationSettled(document.querySelector('main > cem-element.properties'));
                    for (const host of document.querySelectorAll('main cem-source-editor, main cem-textarea, main cem-field, main cem-checkbox, main cem-radio')) await runtime.whenRenderSettled(host);
                });
                const source = page.getByRole('textbox', { name: 'Source document', exact: true });
                await source.fill('<p>Plain source</p>\n{h1 | Edited}');
                await page.waitForFunction(() => document.querySelector('cem-textarea[label="value"]')?.value === '<p>Plain source</p>\n{h1 | Edited}').catch(async error => {
                    console.error(await page.evaluate(() => [...document.querySelectorAll('cem-source-editor, cem-textarea')].map(host => ({ tag: host.localName, value: host.value, text: host.querySelector('[part="control"]')?.innerText, diagnostics: window.cemPlaygroundRuntime.diagnosticsFor(host) }))));
                    throw error;
                });
                assert.equal(await source.locator('p').count(), 0);
                await page.getByRole('textbox', { name: 'Source content type', exact: true }).fill('text/html');
                await page.waitForFunction(() => {
                    const host = document.querySelector('main cem-source-editor');
                    return [...CSS.highlights.values()].some(paint => [...paint].some(range => host.contains(range.startContainer)));
                });
                assert.equal(await source.evaluate(node => node.value), '<p>Plain source</p>\n{h1 | Edited}', 'Coloring preserves authored source');
                await page.getByRole('textbox', { name: 'label', exact: true }).fill('Edited filename');
                await page.waitForFunction(() => document.querySelector('main cem-source-editor')?.getAttribute('label') === 'Edited filename');
                assert.equal(await source.evaluate(node => node.value), '<p>Plain source</p>\n{h1 | Edited}');
                await page.locator('cem-checkbox[label="readonly"] input').check();
                await page.waitForFunction(() => document.querySelector('main cem-source-editor [part="control"]')?.contentEditable === 'false');
                await page.emulateMedia({ forcedColors: 'active' });
                await source.focus(); await page.keyboard.press('ArrowRight');
                assert.deepEqual(await source.evaluate(node => {
                    const style = getComputedStyle(node);
                    return [style.forcedColorAdjust, style.borderTopStyle, parseFloat(style.borderTopWidth) > 0,
                        node.matches(':focus-visible'), style.outlineStyle, parseFloat(style.outlineWidth) > 0];
                }), ['auto', 'solid', true, true, 'solid', true], 'Source boundary and keyboard focus survive forced colors');
                for (const disabled of [false, true]) {
                    if (disabled) await page.evaluate(async () => {
                        const host = document.querySelector('main cem-source-editor'); host.setAttribute('disabled', '');
                        await window.cemPlaygroundRuntime.whenRenderSettled(host);
                    });
                    assert.equal(await source.evaluate((node, disabled) => {
                        const marker = document.createElement('span'); marker.style.color = disabled ? 'GrayText' : 'CanvasText'; document.body.append(marker);
                        const expected = getComputedStyle(marker).color; marker.remove();
                        return getComputedStyle(node, '::highlight(cem-source-name)').color === expected;
                    }, disabled), true, `Source paint uses forced ${disabled ? 'GrayText' : 'CanvasText'}`);
                }
            }
        } finally { await context.close(); }
    }
    console.log(`${tags ? tags.join(', ') : 'All canonical component'} galleries verified: ${baseUrl}`);
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
        await page.getByRole('textbox', { name: 'label', exact: true }).fill('Updated fallback');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="content"]')?.textContent?.replace(/\s/g, "") === 'settingsUpdatedfallback');
        await page.getByRole('textbox', { name: 'label', exact: true }).fill('');
        await page.waitForFunction(() => document.querySelector('#icon-preview [part="content"]')?.textContent?.trim() === 'settings');
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
    // Gallery sections may regroup examples; require each legacy case exactly once.
    assert.deepEqual(await page.locator('[data-legacy-icon-case]').evaluateAll(nodes => nodes.map(node => node.dataset.legacyIconCase).sort()), [...cases].sort());
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
    assert.deepEqual(colors.map(color => color.token), ['var(--cem-palette-danger)', 'green', 'blue']);
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
    const projected = page.locator('cem-demo-element cem-icon').filter({ has: page.locator('a').filter({ hasText: /^Details$/ }) });
    assert.equal(await projected.count(), 1);
    assert.equal(await projected.locator('[part="content"]').evaluate(node => node.textContent.replace(/\s/g, '')), `${await projected.getAttribute('image')}Details`);
    assert.equal(await projected.locator('a :is(strong, b)').textContent(), 'Details');
    assert.equal(await projected.locator('a').evaluate(node => node.closest('[role="img"], [aria-hidden="true"]')), null);
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
            const title = source.match(/<cem-gallery-page[^>]* heading="([^"]*)"/)[1];
            await page.goto(new URL(file, baseUrl).href);
            const nav = page.getByRole('navigation', { name: 'Gallery navigation' });
            await nav.waitFor().catch(error => { throw new Error(`Gallery navigation did not render at ${page.url()}`, { cause: error }); });
            assert.equal(await nav.getByRole('heading', { level: 1 }).textContent(), title);
            assert.equal(await page.locator('h1').count(), 1);
            const index = nav.getByRole('link', { name: 'Component playground', exact: true });
            await index.waitFor();
            assert.equal(new URL(await index.getAttribute('href'), page.url()).href, new URL('index.html', baseUrl).href);
            const theme = nav.getByRole('button', { name: 'Change theme', exact: true });
            await theme.waitFor();
            assert.equal(await theme.isEnabled(), true);
            await theme.click();
            await nav.getByRole('menuitemradio', { name: 'Dark', exact: true }).click();
            await page.waitForFunction(() => document.querySelector('cem-gallery-page')?.getAttribute('data-theme') === 'cem-theme-dark');
            assert.equal(await page.evaluate(() => localStorage.getItem('cem-playground-theme')), 'cem-theme-dark');
            await index.click();
            assert.equal(page.url(), new URL('index.html', baseUrl).href);
            await page.getByRole('heading', { name: 'CEM component playground', exact: true }).waitFor().catch(async error => {
                throw new Error(`Index navigation from ${file} failed: ${JSON.stringify(await capturePlaygroundReadiness(page))}; headings=${await page.locator('h1').allTextContents()}`, { cause: error });
            });
        }
    } finally {
        await context.close();
    }
}

async function verifySuggestions(baseUrl) {
    const context = await browser.newContext();
    const page = await context.newPage();
    page.setDefaultTimeout(30_000);
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(new URL('cem-suggestions.html', baseUrl).href);
        await page.waitForFunction(() => {
            const host = document.querySelector('cem-suggestions');
            return host && window.cemPlaygroundRuntime?.renderedSuggestionsFor(host)?.current();
        });
        const editor = page.getByRole('combobox', { name: 'Letter', exact: true });
        await editor.focus();
        await page.waitForFunction(() => document.querySelector('cem-suggestions [slot=editor] input')?.getAttribute('aria-expanded') === 'true');
        await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter');
        assert.equal(await editor.inputValue(), 'a');
        await page.getByRole('link', { name: 'Full examples and variation matrix', exact: true }).click();
        await page.waitForFunction(() => {
            const hosts = [...document.querySelectorAll('[data-gallery-theme] cem-suggestions')];
            return hosts.length === 30 && hosts.every(host => host.querySelector('[part=surface]') && host.querySelector('[slot=editor] input'));
        });
        assert.equal(await page.locator('[data-gallery-theme]').count(), 5);
        const custom = page.locator('cem-demo-element[legend="Native custom labels"] cem-suggestions');
        await custom.locator('[part=option-label] strong').first().waitFor({ state: 'attached' });
        assert.equal(await custom.locator('[part=option-label] strong').first().textContent(), 'Alpha');
        assert.equal(await custom.locator('[part=option-value]').first().textContent(), 'a');
        const demo = legend => page.locator(`cem-demo-element[legend="${legend}"]`);
        const sourceLegends = ['No value', 'No value with placeholder', 'Value defined', 'No initial value with placeholder',
            'Number, No initial value without placeholder', 'Options text as value', 'Options Grouping'];
        for (const legend of sourceLegends) {
            assert.equal(await demo(legend).count(), 1, `one source-intent example: ${legend}`);
            await demo(legend).locator('input').waitFor();
        }
        for (const legend of ['No value', 'No value with placeholder', 'Number, No initial value without placeholder']) {
            const input = demo(legend).locator('input');
            assert.equal(await demo(legend).locator('cem-suggestions').count(), 0);
            await input.fill(legend === 'No value' ? 'ordinary edit' : '2');
            assert.equal(await input.inputValue(), legend === 'No value' ? 'ordinary edit' : '2');
            assert.equal(await input.getAttribute('role'), null);
        }
        assert.equal(await demo('No value with placeholder').locator('input').getAttribute('placeholder'), 'Enter a number');
        const readyDemo = async legend => {
            await page.waitForFunction(legend => {
                const host = document.querySelector(`cem-demo-element[legend="${legend}"] cem-suggestions`);
                return host && window.cemPlaygroundRuntime.renderedSuggestionsFor(host)?.current();
            }, legend);
        };
        const submitted = legend => demo(legend).locator('form').evaluate(form => new FormData(form).get('choice'));
        const initial = demo('Value defined').locator('input'); await readyDemo('Value defined');
        assert.equal(await initial.inputValue(), 'abc'); await initial.fill('edited'); await readyDemo('Value defined');
        await demo('Value defined').locator('cem-suggestions').evaluate(host => host.setAttribute('label', 'Updated suggestions'));
        await readyDemo('Value defined'); assert.equal(await initial.inputValue(), 'edited');
        await initial.fill(''); await readyDemo('Value defined'); assert.equal(await initial.inputValue(), '');
        assert.equal(await submitted('Value defined'), '');
        const choose = async (legend, index, expected, pointer = false) => {
            await readyDemo(legend); const input = demo(legend).locator('input');
            await input.scrollIntoViewIfNeeded(); await input.evaluate(node => node.blur()); await input.fill(''); await readyDemo(legend); await input.focus();
            await page.waitForFunction(legend => document.querySelector(`cem-demo-element[legend="${legend}"] input`)?.getAttribute('aria-expanded') === 'true', legend);
            const rows = demo(legend).locator('[part=option]');
            if (pointer) await rows.nth(index).click();
            else { for (let i = 0; i <= index; i++) await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter'); }
            assert.equal(await input.inputValue(), expected);
            assert.equal(await submitted(legend), expected);
            assert.equal(await input.evaluate(node => node === document.activeElement), true);
        };
        await choose('No initial value with placeholder', 0, 'apple');
        await choose('Options text as value', 0, 'Apple pie', true);
        await choose('Native option value and label edges', 0, '');
        await choose('Native option value and label edges', 1, 'stored text', true);
        await choose('Native option value and label edges', 2, 'explicit');
        await choose('Native option value and label edges', 3, 'A\u00a0B', true);
        const grouped = demo('Options Grouping'), groupInput = grouped.locator('input');
        await readyDemo('Options Grouping'); await groupInput.scrollIntoViewIfNeeded(); await groupInput.fill('Str'); await readyDemo('Options Grouping');
        await page.waitForFunction(() => {
            const host = document.querySelector('cem-demo-element[legend="Options Grouping"]');
            return [...host.querySelectorAll('[part=group]')].some(group => group.getAttribute('aria-label') === 'Orchard' && group.hidden);
        });
        assert.equal(await grouped.locator('[part=group]:not([hidden])').getAttribute('aria-label'), 'Berries');
        await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter');
        assert.equal(await submitted('Options Grouping'), 'strawberry');
        await page.emulateMedia({ forcedColors: 'active' });
        await groupInput.fill(''); await readyDemo('Options Grouping');
        await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Options Grouping"] input')?.getAttribute('aria-expanded') === 'true');
        await page.keyboard.press('ArrowDown');
        const active = grouped.locator('[part=option][aria-selected=true]');
        await active.waitFor();
        assert.equal(await active.evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
        assert(await active.evaluate(node => parseFloat(getComputedStyle(node).outlineWidth) > 0), 'forced-colors preview has a visible outline');
        assert.equal(await groupInput.getAttribute('aria-activedescendant'), await active.getAttribute('id'));
        await page.keyboard.press('Escape'); assert.equal(await groupInput.inputValue(), '');
        await page.emulateMedia({ forcedColors: 'none' });
        const diagnostics = await page.evaluate(() => [...document.querySelectorAll('cem-suggestions')].flatMap(host => window.cemPlaygroundRuntime.diagnosticsFor(host)));
        assert.deepEqual(diagnostics, []);
        assert.deepEqual(errors, []);
        const back = page.getByRole('link', { name: 'Suggestions property playground and source', exact: true });
        assert.equal(new URL(await back.getAttribute('href'), page.url()).href, new URL('cem-suggestions.html', baseUrl).href);
    } catch (error) {
        console.error('Suggestions page failure:', page.url(), await page.evaluate(() => [...document.querySelectorAll('cem-suggestions, cem-element')].slice(0, 8).map(host => ({ tag: host.localName, diagnostics: window.cemPlaygroundRuntime?.diagnosticsFor(host) }))), errors);
        throw error;
    } finally { await context.close(); }
}


/** Observations only: accepting a showPicker call is not proof of a visible/chosen option. */
async function verifyNativeDatalist(base, packaging) {
    const page = await browser.newPage({ viewport: { width: 1200, height: 850 } });
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(`${base}cem-suggestions.html`);
        await page.locator('cem-suggestions[profile="native-datalist"] input').waitFor();
        for (const tag of ['cem-field', 'cem-text-field']) {
            for (const type of ['number', 'text']) {
                // Test fixture uses the production declaration/runtime and original provider.
                // Its event recorder observes native input; it implements no UI behavior.
                await page.evaluate(async ({ tag, type }) => {
                    document.querySelector('section[aria-label="Native acceptance fixture"]')?.remove();
                    const fixture = document.createElement('section'); fixture.setAttribute('aria-label', 'Native acceptance fixture');
                    fixture.innerHTML = `<form><cem-suggestions profile="native-datalist"><template><${tag} slot="editor" name="choice" type="${type}" label="Native acceptance" min="0" max="30" step="1"></${tag}><template slot="options"><option value="1" label="One"></option><option value="2" label="Two"></option><option value="10" label="Ten"></option></template></template></cem-suggestions></form>`;
                    document.querySelector('main').prepend(fixture);
                    const host = fixture.querySelector('cem-suggestions');
                    await window.cemPlaygroundRuntime.whenRenderSettled(host);
                    await window.cemPlaygroundRuntime.whenRenderSettled(host.querySelector('[slot=editor]'));
                    window.nativeAcceptanceEvents = [];
                    for (const eventName of ['input', 'change']) fixture.addEventListener(eventName, event => {
                        if (event.target instanceof HTMLInputElement) window.nativeAcceptanceEvents.push({ type: event.type, trusted: event.isTrusted, value: event.target.value });
                    });
                    fixture.querySelector('form').addEventListener('submit', event => event.preventDefault());
                }, { tag, type });
                const fixture = page.locator('section[aria-label="Native acceptance fixture"]');
                const input = fixture.locator('input');
                await page.waitForFunction(() => document.querySelector('section[aria-label="Native acceptance fixture"] input')?.list?.options.length === 3);
                await input.scrollIntoViewIfNeeded();
                const initial = await input.evaluate(node => ({ type: node.type, role: node.getAttribute('role'), options: [...node.list.options].map(o => ({ value: o.value, label: o.label })) }));
                assert.equal(initial.type, type); assert.equal(initial.role, null);
                await input.click();
                const picker = await input.evaluate(node => {
                    try { node.showPicker(); return 'request-accepted'; } catch (error) { return `${error.name}: ${error.message}`; }
                });
                await page.keyboard.press('ArrowDown'); await page.keyboard.press('Enter');
                const afterPicker = await input.evaluate(node => ({ value: node.value, submitted: new FormData(node.closest('form')).get('choice'), events: [...window.nativeAcceptanceEvents] }));
                assert.notEqual(afterPicker.value, 'One'); // labels must not replace numeric or textual values
                assert.equal(afterPicker.submitted, afterPicker.value);
                await page.keyboard.press('Escape');
                await input.fill('1');
                await page.waitForFunction(() => document.querySelector('section[aria-label="Native acceptance fixture"] input')?.list?.options.length === 3);
                // Browser filtering is not exposed as a shared-runtime match list.
                const filterRequest = await input.evaluate(node => { try { node.showPicker(); return 'request-accepted'; } catch (error) { return error.name; } });
                const accessibility = await input.ariaSnapshot();
                const filtered = await input.evaluate(node => ({ value: node.value, optionCount: node.list.options.length, customSurface: !!node.closest('cem-suggestions').querySelector('[part~=surface]') }));
                assert.equal(filtered.optionCount, 3); assert.equal(filtered.customSurface, false);
                await fixture.locator('cem-suggestions').evaluate(async host => {
                    host.setAttribute('options-state', 'pending'); await window.cemPlaygroundRuntime.whenRenderSettled(host);
                });
                await page.waitForFunction(() => !document.querySelector('section[aria-label="Native acceptance fixture"] input')?.hasAttribute('list'));
                const withdrawn = await input.evaluate(node => ({ value: node.value, list: node.getAttribute('list'), options: node.closest('cem-suggestions').querySelector('datalist').options.length, submitted: new FormData(node.closest('form')).get('choice') }));
                assert.equal(withdrawn.value, '1'); assert.equal(withdrawn.options, 0); assert.equal(withdrawn.submitted, '1');
                await page.keyboard.press('Escape');
                await input.fill('2');
                assert.equal(await input.inputValue(), '2');
                nativeEvidence.push({ packaging, field: tag, type, initial, picker, afterPicker, filterRequest, filtered, accessibility, withdrawn, editingAfterWithdrawal: true, pickerSelection: 'unconfirmed', visibleFilteredChoices: 'not-observed' });
            }
        }
        assert.deepEqual(errors, [], 'Acceptance page must not have script failures');
    } finally { await page.close(); }
}


/** Native history observations; only coherence is portable, not undo grouping. */
async function verifyNativeHistory(base, packaging) {
    for (const variant of ['native-typing', 'value-assignment', 'range-replacement', 'cem-field', 'cem-text-field']) {
        // A fresh context isolates the undo manager from every earlier fixture.
        const context = await browser.newContext();
        const page = await context.newPage();
        const errors = [];
        page.on('pageerror', error => errors.push(error.message));
        try {
            await page.goto(`${base}cem-suggestions.html`);
            await page.locator('cem-suggestions input').first().waitFor();
            const field = variant.startsWith('cem-');
            await page.evaluate(async ({ variant, field }) => {
                const fixture = document.createElement('section');
                fixture.setAttribute('aria-label', 'History acceptance fixture');
                fixture.innerHTML = field
                    ? `<form><cem-suggestions filter="none" require-selection><template><${variant} slot="editor" name="choice" label="History choice" required pattern="alpha"></${variant}><template slot="options"><option value="alpha">Alpha</option></template></template></cem-suggestions></form>`
                    : '<form><label>History choice<input name="choice" required pattern="alpha"></label></form>';
                document.querySelector('main').prepend(fixture);
                const host = fixture.querySelector('cem-suggestions');
                if (host) {
                    await window.cemPlaygroundRuntime.whenRenderSettled(host);
                    await window.cemPlaygroundRuntime.whenRenderSettled(host.querySelector('[slot=editor]'));
                }
                window.historyAcceptanceInput = fixture.querySelector('input');
                window.historyAcceptanceEvents = [];
                for (const type of ['beforeinput', 'input', 'change']) fixture.addEventListener(type, event => {
                    if (event.target === window.historyAcceptanceInput) window.historyAcceptanceEvents.push({
                        type, inputType: event.inputType ?? null, trusted: event.isTrusted,
                        value: event.target.value, start: event.target.selectionStart, end: event.target.selectionEnd,
                    });
                });
                fixture.querySelector('form').addEventListener('submit', event => event.preventDefault());
            }, { variant, field });
            const input = page.locator('section[aria-label="History acceptance fixture"] input');
            const ready = async () => {
                if (field) await page.waitForFunction(() => window.cemPlaygroundRuntime.renderedSuggestionsFor(
                    document.querySelector('section[aria-label="History acceptance fixture"] cem-suggestions'))?.current());
            };
            const steps = [];
            const snapshot = async phase => {
                await ready();
                const state = await input.evaluate(node => {
                    const field = node.closest('[slot=editor]'), form = node.closest('form');
                    return { value: node.value, fieldValue: field?.value ?? null,
                        submitted: new FormData(form).getAll('choice'),
                        start: node.selectionStart, end: node.selectionEnd, direction: node.selectionDirection,
                        sameInput: node === window.historyAcceptanceInput, focused: node === document.activeElement,
                        nativeValid: node.validity.valid, fieldValid: field?.validity.valid ?? null,
                        formValid: form.checkValidity(), selectionProofMissing: field?.validity.customError ?? null,
                        events: window.historyAcceptanceEvents.splice(0),
                    };
                });
                assert.equal(state.sameInput, true); assert.equal(state.focused, true);
                assert.deepEqual(state.submitted, [state.value]);
                if (field) {
                    assert.equal(state.fieldValue, state.value);
                    assert.equal(state.formValid, state.fieldValid);
                    if (state.value && state.events.some(event => event.type === 'input' && event.trusted
                        && ['historyUndo', 'historyRedo'].includes(event.inputType))) {
                        assert.equal(state.selectionProofMissing, true, 'Native history must not retain or restore source proof');
                    }
                } else assert.equal(state.formValid, state.nativeValid);
                steps.push({ phase, ...state });
                return state;
            };
            await ready(); await input.click();
            await page.keyboard.type('al');
            await snapshot('typed-prefix');
            // Selection is recorded before replacement and during all history commands.
            await page.keyboard.press('ControlOrMeta+a');
            await snapshot('selected-prefix');
            if (field) {
                await page.keyboard.press('ArrowDown');
                await page.waitForFunction(() => document.querySelector('section[aria-label="History acceptance fixture"] input')?.hasAttribute('aria-activedescendant'));
                await page.keyboard.press('Enter');
            } else if (variant === 'native-typing') await page.keyboard.type('alpha');
            else await input.evaluate((node, variant) => {
                if (variant === 'range-replacement') node.setRangeText('alpha', 0, node.value.length, 'end');
                else node.value = 'alpha';
            }, variant);
            const committed = await snapshot('replacement');
            assert.equal(committed.value, 'alpha'); assert.equal(committed.formValid, true);
            await page.keyboard.press('ControlOrMeta+z');
            const undone = await snapshot('immediate-undo');
            if (variant === 'native-typing') {
                assert.notEqual(undone.value, 'alpha', 'Plain native typing must establish functioning undo input');
                assert(undone.events.some(event => event.type === 'input' && event.inputType === 'historyUndo' && event.trusted));
            }
            await page.keyboard.press('ControlOrMeta+Shift+z');
            await snapshot('immediate-redo');
            await page.keyboard.press('End'); await page.keyboard.type('x');
            const typed = await snapshot('later-typing');
            if (field) assert.equal(typed.selectionProofMissing, true);
            await page.keyboard.press('ControlOrMeta+z');
            const restored = await snapshot('undo-later-typing');
            if (field && restored.value) assert.equal(restored.selectionProofMissing, true, 'Restoring a string must not restore source proof');
            await page.keyboard.press('ControlOrMeta+Shift+z');
            await snapshot('redo-later-typing');
            assert.deepEqual(errors, [], 'History probe must not have script failures');
            historyEvidence.push({ packaging, variant, steps });
        } finally { await context.close(); }
    }
}

async function verifyCheckables(baseUrl) {
    const tags = ['cem-checkbox', 'cem-radio', 'cem-switch'];
    await verifyGalleries(baseUrl, tags);
    for (const tag of tags) {
        const context = await browser.newContext(), page = await context.newPage(), errors = [];
        page.on('pageerror', error => errors.push(error.message));
        try {
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties form > ${tag}`), input = host.locator('input');
            await input.waitFor();
            const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            const text = async (name, value) => {
                await page.locator(`.properties cem-field[label="${name}"] input`).fill(value);
                await page.waitForFunction(({ tag, name, value }) => document.querySelector(`.properties form > ${tag}`)?.getAttribute(name) === value, { tag, name, value });
                await settle();
            };
            const flag = async (name, present) => {
                await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: present ? 'present' : 'absent', exact: true }).check();
                await page.waitForFunction(({ tag, name, present }) => document.querySelector(`.properties form > ${tag}`)?.hasAttribute(name) === present, { tag, name, present });
                await settle();
            };
            const role = tag === 'cem-radio' ? 'radio' : tag === 'cem-switch' ? 'switch' : 'checkbox';
            await text('label', 'Edited choice'); await flag('required', true);
            assert.equal(await host.getByRole(role, { name: 'Edited choice', exact: true }).count(), 1);
            await text('required-marker', 'Required'); assert.equal(await host.getByRole(role, { name: 'Edited choice', exact: true }).count(), 1);
            await text('required-marker', ''); assert.equal(await host.locator('[part=required-marker]').textContent(), '');
            await text('aria-label', 'Accessible choice'); assert.equal(await host.getByRole(role, { name: 'Accessible choice', exact: true }).count(), 1);
            await text('aria-labelledby', 'choice-name'); await text('aria-describedby', 'choice-help'); await text('aria-errormessage', 'choice-error');
            assert.equal(await host.getByRole(role, { name: 'Referenced choice', exact: true }).count(), 1);
            await flag('checked', true); assert.equal(await input.isChecked(), true);
            await input.evaluate(node => { window.checkableInput = node; node.checked = false; });
            await text('class', 'preference-choice'); assert.equal(await input.isChecked(), false);
            await page.locator('.properties').getByRole('button', { name: 'Reset choice', exact: true }).click(); assert.equal(await input.isChecked(), true);
            await flag('disabled', true); assert.equal(await input.isDisabled(), true);
            assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), []);
            await flag('disabled', false); await flag('busy', true);
            assert.equal(await input.getAttribute('aria-busy'), 'true'); assert.equal(await input.isDisabled(), false);
            await text('value', 'accepted'); assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), [['choice', 'accepted']]);
            await text('value', ''); assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), [['choice', '']]);
            await text('form', 'choice-external'); assert.equal(await input.evaluate(node => node.form?.id), 'choice-external');
            await input.evaluate(node => { node.checked = false; }); await page.getByRole('button', { name: 'Reset external form', exact: true }).click(); assert.equal(await input.isChecked(), true);
            assert.equal(await input.evaluate(node => node === window.checkableInput), true);
            if (tag === 'cem-checkbox') {
                await flag('indeterminate', true); assert.equal(await input.evaluate(node => node.indeterminate), true);
                await input.click(); assert.equal(await input.evaluate(node => node.indeterminate), false);
                await text('label', 'Unchanged mixed default'); assert.equal(await input.evaluate(node => node.indeterminate), false);
            }
            await flag('hidden', true); assert.equal(await input.isVisible(), false); await flag('hidden', false);
            await page.goto(new URL(`${tag}-gallery.html`, baseUrl).href);
            const demo = legend => page.locator(`cem-demo-element[legend="${legend}"]`);
            await demo('Labels and required markers').getByRole(role, { name: 'Projected choice', exact: true }).waitFor();
            assert.equal(await demo('Labels and required markers').getByRole(role, { name: 'Visible choice', exact: true }).count(), 1);
            const nativeForm = demo('Native forms and reset').locator('form').first(), control = nativeForm.locator(`${tag}[name=choice] input`);
            await control.waitFor(); assert.deepEqual(await nativeForm.evaluate(node => [...new FormData(node)]), [['choice', 'yes']]);
            await control.evaluate(node => { node.checked = false; }); assert.equal(await nativeForm.evaluate(node => node.checkValidity()), false);
            await nativeForm.getByRole('button', { name: 'Reset choices', exact: true }).click(); assert.equal(await control.isChecked(), true);
            if (tag === 'cem-radio') {
                const groups = demo('Native radio groups'), basic = groups.locator('cem-radio[value=basic] input'), pro = groups.locator('cem-radio[value=pro] input');
                await basic.focus(); await page.keyboard.press('ArrowRight'); assert.equal(await pro.isChecked(), true); assert.equal(await basic.isChecked(), false);
                await page.keyboard.press('ArrowRight'); assert.equal(await groups.locator('input[value=native]').isChecked(), true); assert.equal(await pro.isChecked(), false);
                assert.equal(await groups.locator('cem-radio[value=separate] input').isChecked(), true);
                await groups.getByRole('button', { name: 'Reset plan', exact: true }).click(); assert.equal(await basic.isChecked(), true);
            } else {
                const binding = demo('Declarative change binding'); await binding.locator('input').check();
                await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Declarative change binding"] output')?.textContent === 'true');
            }
            const first = page.locator(`[data-gallery-theme] ${tag}[label=Default] input`).first();
            await first.focus(); await page.keyboard.press('Space'); assert.equal(await first.isChecked(), true);
            await page.emulateMedia({ forcedColors: 'active' }); await first.focus(); await page.keyboard.press('Shift');
            const paint = await first.evaluate(node => {
                const style = getComputedStyle(node, node.type === 'radio' ? '::before' : null);
                return { focused: node.matches(':focus-visible'), shadow: style.boxShadow, outline: style.outlineStyle, width: parseFloat(style.outlineWidth) };
            });
            assert.equal(paint.focused, true); assert.equal(paint.shadow, 'none'); assert.equal(paint.outline, 'solid'); assert(paint.width > 0);
            assert.deepEqual(await page.evaluate(tag => [...document.querySelectorAll(tag)].flatMap(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), tag), []);
            assert.deepEqual(errors, []);
        } finally { await context.close(); }
    }
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
}

async function verifySheet(baseUrl) {
    await verifyGalleries(baseUrl, ['cem-sheet']);
    const context = await browser.newContext(), page = await context.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(new URL('cem-sheet.html', baseUrl).href);
        const host = page.locator('.properties cem-sheet'), owner = host.locator('[part=surface]');
        await owner.waitFor();
        assert.equal(await owner.getAttribute('role'), 'region');
        const setText = async (name, value) => {
            await page.locator(`.properties cem-field[label="${name}"] input`).fill(value);
            await page.waitForFunction(({ name, value }) => document.querySelector('.properties cem-sheet')?.getAttribute(name) === value, { name, value });
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        };
        await setText('label', 'Edited filters');
        assert.equal(await owner.getAttribute('aria-label'), 'Edited filters');
        assert.equal(await host.locator('[part=heading]').textContent(), 'Edited filters');
        await setText('aria-label', 'Accessible filters');
        assert.equal(await owner.getAttribute('aria-label'), 'Accessible filters');
        await setText('aria-labelledby', 'sheet-playground-name');
        await setText('aria-describedby', 'sheet-playground-help');
        assert.equal(await page.getByRole('region', { name: 'Referenced filters', exact: true }).count(), 1);
        const projected = page.locator('cem-demo-element[legend="Projected label"]');
        await projected.locator('cem-sheet [part=heading] strong').waitFor();
        assert.equal(await projected.locator('cem-sheet [part=heading] strong').textContent(), 'Projected title');
        assert.equal(await projected.getByRole('region', { name: 'Projected title', exact: true }).count(), 1);
        await setText('class', 'filter-region');
        await page.getByRole('group', { name: 'dir', exact: true }).getByRole('radio', { name: 'rtl', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('.properties cem-sheet')?.getAttribute('dir') === 'rtl');
        assert.equal(await owner.evaluate(node => getComputedStyle(node).direction), 'rtl');
        await host.locator('cem-field input').fill('Retained draft');
        await host.evaluate(node => { window.sheetOwner = node.querySelector('aside'); window.sheetInput = node.querySelector('input'); });
        const toggle = async (name, present) => {
            const control = page.getByRole('group', { name, exact: true }).getByRole('radio', { name: present ? 'present' : 'absent', exact: true });
            await control.check();
            await page.waitForFunction(({ name, present }) => document.querySelector('.properties cem-sheet')?.hasAttribute(name) === present, { name, present });
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            assert.equal(await control.evaluate(node => document.activeElement === node), true, `${name}: visibility leaves focus on its control`);
        };
        await toggle('transient', true); assert.equal(await owner.evaluate(node => node.hidden), true);
        await toggle('expanded', true); assert.equal(await owner.evaluate(node => node.hidden), false);
        await host.locator('input').focus(); await page.keyboard.press('Escape');
        assert.equal(await owner.evaluate(node => node.hidden), false);
        assert.equal(await host.locator('input').evaluate(node => document.activeElement === node), true);
        await toggle('expanded', false); await toggle('transient', false);
        assert.equal(await owner.evaluate(node => node.hidden), false);
        assert.equal(await host.evaluate(node => node.querySelector('aside') === window.sheetOwner && node.querySelector('input') === window.sheetInput), true);
        assert.equal(await host.locator('input').inputValue(), 'Retained draft');
        await host.getByRole('button', { name: 'Reset', exact: true }).click();
        assert.equal(await host.locator('input').inputValue(), 'Editable draft');
        await toggle('hidden', true); assert.equal(await owner.isVisible(), false); await toggle('hidden', false);
        await page.goto(new URL('cem-sheet-gallery.html', baseUrl).href);
        await page.waitForFunction(() => {
            const demos = [...document.querySelectorAll('cem-demo-element')];
            return demos.length === 5 && demos.every(node => node.getAttribute('data-state') === 'ready');
        });
        const demo = legend => page.locator(`cem-demo-element[legend="${legend}"]`);
        await demo('Visible labels and accessible names').getByRole('region', { name: 'Projected filters', exact: true }).waitFor();
        await demo('Visible labels and accessible names').getByRole('region', { name: 'Search settings', exact: true }).waitFor();
        assert.equal(await demo('Visible labels and accessible names').getByRole('region', { name: 'Projected filters', exact: true }).count(), 1);
        assert.equal(await demo('Visible labels and accessible names').getByRole('region', { name: 'Search settings', exact: true }).count(), 1);
        const controlled = demo('Application-controlled visibility'), sheet = controlled.locator('cem-sheet');
        await sheet.locator('aside').waitFor({ state: 'attached' });
        assert.equal(await sheet.locator('aside').evaluate(node => node.hidden), true);
        const open = controlled.getByRole('radio', { name: 'Open', exact: true }); await open.check();
        await sheet.locator('aside').waitFor(); assert.equal(await open.evaluate(node => node === document.activeElement), true);
        await sheet.locator('input').fill('Gallery draft'); await controlled.getByRole('radio', { name: 'Closed', exact: true }).check();
        await sheet.locator('aside').waitFor({ state: 'hidden' }); await open.check(); await sheet.locator('aside').waitFor();
        assert.equal(await sheet.locator('input').inputValue(), 'Gallery draft');
        assert.equal(await demo('Presence and persistent mode').locator('cem-sheet[expanded="false"] aside').isVisible(), true);
        assert.equal(await demo('Host visibility and direction').locator('cem-sheet[hidden] aside').isVisible(), false);
        const form = demo('Native form ownership'); await form.locator('input').fill('Changed'); await form.getByRole('button', { name: 'Reset filters', exact: true }).click();
        assert.equal(await form.locator('input').inputValue(), 'Saved');
        await page.emulateMedia({ forcedColors: 'active' });
        const first = page.locator('[data-gallery-theme] cem-sheet').first();
        await first.locator('input').focus(); await page.keyboard.press('ArrowRight');
        assert.equal(await first.locator('input').evaluate(node => node.matches(':focus-visible')), true);
        assert.equal(await first.locator('aside').getAttribute('tabindex'), null);
        const paint = await first.locator('aside').evaluate(node => {
            const probe = document.createElement('span'); probe.style.cssText = 'background: Canvas; color: CanvasText'; node.append(probe);
            const style = getComputedStyle(node), system = getComputedStyle(probe);
            const result = { background: style.backgroundColor, color: style.color, canvas: system.backgroundColor, text: system.color, border: parseFloat(style.borderTopWidth) };
            probe.remove(); return result;
        });
        assert.equal(paint.background, paint.canvas); assert.equal(paint.color, paint.text); assert(paint.border > 0);
        assert.deepEqual(await page.evaluate(() => [...document.querySelectorAll('cem-sheet')].flatMap(node => window.cemPlaygroundRuntime.diagnosticsFor(node))), []);
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        const bundled = page.locator('main > cem-sheet'); await bundled.locator('input').waitFor();
        await bundled.locator('input').fill('Bundled draft');
        await bundled.evaluate(async node => { node.removeAttribute('expanded'); await window.cemPlaygroundRuntime.whenRenderSettled(node); });
        assert.equal(await bundled.locator('aside').evaluate(node => node.hidden), true);
        await bundled.evaluate(async node => { node.setAttribute('expanded', ''); await window.cemPlaygroundRuntime.whenRenderSettled(node); });
        assert.equal(await bundled.locator('input').inputValue(), 'Bundled draft');
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
}

async function verifySurfaces(baseUrl) {
    const tags = ['cem-dialog', 'cem-dialog-shell', 'cem-tooltip'];
    await verifyGalleries(baseUrl, tags);
    for (const tag of tags) {
        const context = await browser.newContext();
        const page = await context.newPage();
        try {
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).first();
            await host.locator('[part=surface]').waitFor({ state: 'attached' });
            const property = tag === 'cem-tooltip' ? 'message' : 'label';
            await page.locator(`.properties cem-field[label="${property}"] input`).fill('Edited surface');
            await page.waitForFunction(({ tag, property }) => document.querySelector(`.properties ${tag}`)?.getAttribute(property) === 'Edited surface', { tag, property });
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            const source = tag === 'cem-tooltip' ? page.getByRole('button', { name: 'External tooltip launcher', exact: true }) : host.locator('[part=trigger], [slot=trigger]');
            if (tag === 'cem-tooltip') {
                await source.focus();
                await page.waitForFunction(() => document.querySelector('.properties cem-tooltip [part=surface]')?.matches(':popover-open'));
                assert.equal(await source.evaluate(node => document.getElementById(node.getAttribute('aria-describedby').split(/\s+/).at(-1)).textContent), 'Edited surface');
                await page.keyboard.press('Escape');
                assert.equal(await host.locator('[part=surface]').evaluate(node => node.matches(':popover-open')), false);
                await page.locator('.properties cem-field[label="trigger-for"] input').fill('#tooltip-playground-alternate');
                await page.waitForFunction(() => {
                    const host = document.querySelector('.properties cem-tooltip'), external = document.getElementById('tooltip-playground-alternate');
                    return !host?.querySelector('[slot=trigger]') && external?.getAttribute('aria-describedby') === host?.querySelector('[part=surface]')?.id;
                });
                await page.getByRole('button', { name: 'Alternate tooltip launcher', exact: true }).focus();
                await page.waitForFunction(() => document.querySelector('.properties cem-tooltip [part=surface]')?.matches(':popover-open'));
                await page.keyboard.press('Escape');
            } else {
                await source.click();
                await page.waitForFunction(tag => document.querySelector(`.properties ${tag} dialog`)?.open, tag);
                assert.equal(await host.locator('dialog').evaluate(node => node.matches(':modal')), false);
                await host.locator('[part=close], [slot=close]').click();
                assert.equal(await source.evaluate(node => document.activeElement === node), true);
            }
            await page.goto(new URL(`${tag}-gallery.html`, baseUrl).href);
            const first = page.locator(`[data-gallery-theme] ${tag}`).first();
            await first.locator('[part=surface]').waitFor({ state: 'attached' });
            const sourceControl = first.locator('[part=trigger], [slot=trigger]');
            await sourceControl.focus();
            if (tag === 'cem-tooltip') {
                await page.waitForFunction(() => document.querySelector('[data-gallery-theme] cem-tooltip [part=surface]')?.matches(':popover-open'));
                await page.keyboard.press('Escape');
                assert.equal(await sourceControl.evaluate(node => document.activeElement === node), true);
            } else {
                await page.keyboard.press('Enter');
                await page.waitForFunction(tag => document.querySelector(`[data-gallery-theme] ${tag} dialog`)?.open, tag);
                assert.equal(await first.locator('dialog').evaluate(node => node.matches(':modal')), false);
                await first.locator('[part=close]').click();
                const modal = page.locator(`[data-gallery-theme] ${tag}[mode=modal]`).first();
                await modal.locator('[part=trigger]').click();
                await page.waitForFunction(tag => document.querySelector(`[data-gallery-theme] ${tag}[mode=modal] dialog`)?.matches(':modal'), tag);
                await page.keyboard.press('Escape');
                await page.waitForFunction(tag => !document.querySelector(`[data-gallery-theme] ${tag}[mode=modal] dialog`)?.open, tag);
                assert.equal(await modal.locator('[part=trigger]').evaluate(node => document.activeElement === node), true);
            }
            await page.getByRole('link', { name: 'Property playground', exact: true }).focus();
            await page.emulateMedia({ forcedColors: 'active' });
            await sourceControl.focus();
            if (tag !== 'cem-tooltip') await page.keyboard.press('Enter');
            await page.waitForFunction(tag => {
                const owner = document.querySelector(`[data-gallery-theme] ${tag} [part=surface]`);
                return owner && (owner.open || owner.matches(':popover-open'));
            }, tag);
            const paint = await first.locator('[part=surface]').evaluate(node => {
                const style = getComputedStyle(node), probe = document.createElement('span');
                probe.style.cssText = 'background: Canvas; color: CanvasText; outline: 2px solid Highlight'; node.append(probe);
                const system = getComputedStyle(probe), result = { background: style.backgroundColor, color: style.color, adjustment: style.forcedColorAdjust, canvas: system.backgroundColor, canvasText: system.color };
                probe.remove(); return result;
            });
            assert.equal(paint.background, paint.canvas); assert.equal(paint.color, paint.canvasText); assert.equal(paint.adjustment, 'auto');
            if (tag !== 'cem-tooltip') {
                await first.locator('[part=close]').focus(); await page.keyboard.press('ArrowRight');
                assert.equal(await first.locator('[part=close]').evaluate(node => getComputedStyle(node).outlineStyle), 'solid');
            }
        } finally { await context.close(); }
    }
    const context = await browser.newContext(), page = await context.newPage();
    try {
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        for (const tag of tags) {
            const host = page.locator(`main > ${tag}`), owner = host.locator('[part=surface]'), source = host.locator('[part=trigger], [slot=trigger]');
            await owner.waitFor({ state: 'attached' });
            if (tag === 'cem-tooltip') await source.focus(); else await source.click();
            await page.waitForFunction(tag => {
                const node = document.querySelector(`main > ${tag} [part=surface]`); return node?.open || node?.matches(':popover-open');
            }, tag);
            await page.keyboard.press('Escape');
        }
    } finally { await context.close(); }
}

async function verifyAutocomplete(baseUrl) {
    console.log(`Autocomplete gallery: ${baseUrl}`);
    await verifyGalleries(baseUrl, ['cem-autocomplete']);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        console.log(`Autocomplete property controls: ${baseUrl}`);
        await page.goto(new URL('cem-autocomplete.html', baseUrl).href);
        const host = page.locator('.properties form > cem-autocomplete'), input = host.locator('input');
        await input.waitFor();
        const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        const text = async (name, value) => {
            await page.locator(`.properties cem-field[label="${name}"] input`).fill(value);
            await page.waitForFunction(({ name, value }) => document.querySelector('.properties form > cem-autocomplete')?.getAttribute(name) === value, { name, value });
            await settle();
        };
        const flag = async (name, present) => {
            await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: present ? 'present' : 'absent', exact: true }).check();
            await page.waitForFunction(({ name, present }) => document.querySelector('.properties form > cem-autocomplete')?.hasAttribute(name) === present, { name, present });
            await settle();
        };
        assert.equal(await input.inputValue(), 'Ada Lovelace');
        assert.deepEqual(await host.evaluate(node => [...new FormData(node.form)]), [['person', 'ada']]);
        await text('label', 'Edited person'); assert.equal(await host.getByRole('combobox', { name: 'Edited person', exact: true }).count(), 1);
        await text('placeholder', 'Type a name'); assert.equal(await input.getAttribute('placeholder'), 'Type a name');
        await text('describedby', 'person-help'); await text('error', 'person-error');
        assert.equal(await input.getAttribute('aria-describedby'), 'person-help'); assert.equal(await input.getAttribute('aria-errormessage'), 'person-error');
        await text('value', 'grace'); assert.equal(await input.inputValue(), 'Grace Hopper');
        await flag('disabled', true); assert.equal(await input.isDisabled(), true); assert.deepEqual(await host.evaluate(node => [...new FormData(node.form)]), []);
        await flag('disabled', false); await flag('readonly', true); assert.equal(await input.evaluate(node => node.readOnly), true);
        await flag('readonly', false); await flag('busy', true); assert.equal(await input.getAttribute('aria-busy'), 'true'); assert.equal(await input.isDisabled(), false);
        await flag('require-selection', true); await flag('required', true);
        await input.fill('Unmatched'); await settle(); assert.equal(await host.evaluate(node => node.value), '');
        await host.evaluate(node => { window.autocompleteEvents = []; for (const name of ['input', 'change']) node.addEventListener(name, event => window.autocompleteEvents.push([event.type, node.value, node.displayValue])); });
        await input.press('Escape'); await settle(); assert.equal(await input.inputValue(), '');
        assert.deepEqual(await page.evaluate(() => window.autocompleteEvents), [['input', '', ''], ['change', '', '']]);
        assert.equal(await host.evaluate(node => node.validity.valueMissing), true);
        await flag('require-selection', false); await flag('busy', false);
        await page.locator('.properties').getByRole('button', { name: 'Reset person', exact: true }).click(); await settle();
        assert.equal(await input.inputValue(), 'Ada Lovelace');
        await text('form', 'person-external'); assert.equal(await host.evaluate(node => node.form.id), 'person-external');
        await input.fill('New person'); await input.press('Tab');
        assert.deepEqual(await host.evaluate(node => [...new FormData(node.form)]), [['person', 'New person']]);
        await page.getByRole('button', { name: 'Reset external person', exact: true }).click(); await settle(); assert.equal(await input.inputValue(), 'Ada Lovelace');
        await input.focus(); await settle();
        await host.evaluate(node => {
            window.autocompleteInput = node.querySelector('input'); window.autocompleteEvents = [];
            const payload = node.querySelector('template[data-cem-island=instance]').content.querySelector('cem-payload\\:payload');
            payload.querySelectorAll('cem-option, cem-option-group').forEach(option => option.remove());
            const option = document.createElement('cem-option'); option.setAttribute('value', 'lin'); option.textContent = 'Lin Chen'; payload.append(option);
        });
        await page.waitForFunction(() => document.querySelector('.properties form > cem-autocomplete')?.querySelector('[role=option]')?.textContent.trim() === 'Lin Chen');
        assert.deepEqual(await host.evaluate(node => ({ value: node.value, display: node.displayValue, selected: node.selectedIndex, same: node.querySelector('input') === window.autocompleteInput, focus: document.activeElement === window.autocompleteInput })), { value: 'ada', display: 'Ada Lovelace', selected: -1, same: true, focus: true });
        assert.deepEqual(await page.evaluate(() => window.autocompleteEvents), []);
        await host.evaluate(node => node.querySelector('template[data-cem-island=instance]').content.querySelector('cem-option').remove());
        await page.waitForFunction(() => !document.querySelector('.properties form > cem-autocomplete').expanded);
        await settle(); assert.equal(await input.getAttribute('aria-controls'), null); assert.equal(await input.getAttribute('aria-activedescendant'), null);
        await flag('hidden', true); assert.equal(await input.isVisible(), false); await flag('hidden', false);
        assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
        console.log(`Autocomplete gallery interactions and bundle: ${baseUrl}`);
        await page.goto(new URL('cem-autocomplete-gallery.html', baseUrl).href);
        const bound = page.locator('cem-demo-element[legend="Declarative value binding"]');
        await bound.getByRole('combobox', { name: 'Bound person' }).fill('Person');
        await bound.getByRole('combobox').press('ArrowDown'); await bound.getByRole('combobox').press('Enter');
        await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Declarative value binding"] output')?.textContent === 'ada');
        await page.emulateMedia({ forcedColors: 'active' });
        const first = page.locator('[data-gallery-theme] cem-autocomplete').first(); await first.locator('input').focus();
        await first.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        assert.equal(await first.locator('input').evaluate(node => getComputedStyle(node).boxShadow), 'none');
        assert.equal(await first.locator('[part=popup]').evaluate(node => getComputedStyle(node).zIndex), '1');
        await page.emulateMedia({ forcedColors: 'none' });
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        const bundled = page.getByRole('combobox', { name: 'Bundled autocomplete', exact: true }); await bundled.waitFor();
        assert.equal(await bundled.inputValue(), 'Ada Lovelace');
        await bundled.focus(); await bundled.press('ArrowDown'); await bundled.press('Enter');
        await page.waitForFunction(() => document.querySelector('cem-autocomplete')?.value === 'grace');
        assert.equal(await bundled.inputValue(), 'Grace Hopper');
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}

async function verifyDatepicker(baseUrl) {
    console.log(`Datepicker gallery: ${baseUrl}`);
    await verifyGalleries(baseUrl, ['cem-datepicker']);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        console.log(`Datepicker properties and native forms: ${baseUrl}`);
        await page.goto(new URL('cem-datepicker.html', baseUrl).href);
        const host = page.locator('.properties form > cem-datepicker'), input = host.locator('input[slot=input]');
        await host.getByRole('combobox').waitFor();
        const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await settle();
        const field = (name) => page.locator(`.properties cem-field[label="${name}"] input`);
        const text = async (name, value) => {
            await field(name).fill(value);
            try { await page.waitForFunction(({ name, value }) => {
                const host = document.querySelector('.properties form > cem-datepicker');
                const input = host?.querySelector('input[slot=input]');
                return ['label', 'name', 'form', 'value'].includes(name)
                    ? input?.getAttribute(name === 'label' ? 'aria-label' : name) === value
                    : host?.getAttribute(name) === value;
            }, { name, value }); }
            catch (error) {
                const state = await host.evaluate(node => ({
                    attributes: [...node.attributes].map(attribute => [attribute.name, attribute.value]),
                    input: node.querySelector('input[slot=input]')?.outerHTML,
                    diagnostics: window.cemPlaygroundRuntime.diagnosticsFor(node),
                }));
                throw new Error(`Datepicker property ${name} did not reach ${value}: ${JSON.stringify(state)}`, { cause: error });
            }
            await settle();
        };
        const flag = async (name, present) => {
            await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: present ? 'present' : 'absent', exact: true }).check();
            await page.waitForFunction(({ name, present }) => document.querySelector('.properties form > cem-datepicker')?.hasAttribute(name) === present, { name, present });
            await settle();
        };
        assert.equal(await input.inputValue(), '2026-08-11');
        assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), [['arrival', '2026-08-11']]);
        await text('label', 'Departure date'); assert.equal(await input.getAttribute('aria-label'), 'Departure date');
        await text('name', 'departure'); assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), [['departure', '2026-08-11']]);
        await text('class', 'cem-theme-dark'); assert.equal(await host.getAttribute('class'), 'cem-theme-dark');
        await text('lang', 'fr-FR'); await text('min', '2026-08-01'); await text('max', '2026-08-31');
        await host.evaluate(node => { window.datepickerEvents = []; for (const type of ['input', 'change']) node.addEventListener(type, event => window.datepickerEvents.push([type, event.target.value, node.querySelector('dialog').open, document.activeElement === event.target])); });
        await input.focus(); await input.press('ArrowDown'); await settle();
        assert.equal(await host.locator('dialog').evaluate(node => node.matches(':modal')), true);
        assert.equal(await host.locator('[part=heading]').textContent(), 'août 2026');
        assert.equal(await host.locator('[role=gridcell]').count(), 42);
        await host.locator('[data-date="2026-08-12"]').click(); await settle();
        assert.equal(await input.inputValue(), '2026-08-11'); assert.deepEqual(await page.evaluate(() => window.datepickerEvents), []);
        await host.locator('[data-calendar-action=apply]').click(); await settle();
        assert.equal(await input.inputValue(), '2026-08-12');
        assert.deepEqual(await page.evaluate(() => window.datepickerEvents), [['input', '2026-08-12', false, true], ['change', '2026-08-12', false, true]]);
        await page.locator('.properties').getByRole('button', { name: 'Reset date', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('.properties form > cem-datepicker input')?.value === '2026-08-11'); await settle();
        await flag('required', true); await input.fill(''); assert.equal(await input.evaluate(node => node.validity.valueMissing), true);
        await input.fill('2026-02-30'); assert.equal(await input.evaluate(node => node.validity.customError), true);
        await input.fill('2026-08-15'); assert.equal(await input.evaluate(node => node.validity.valid), true);
        await flag('invalid', true); assert.equal(await input.getAttribute('aria-invalid'), 'true'); await flag('invalid', false);
        await flag('disabled', true); assert.equal(await input.isDisabled(), true); assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), []); await flag('disabled', false);
        await page.getByRole('group', { name: 'indicator', exact: true }).getByRole('radio', { name: 'outline', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('.properties cem-datepicker')?.getAttribute('indicator') === 'outline'); await settle();
        assert.equal(await input.evaluate(node => getComputedStyle(node).getPropertyValue('--_cem-input-indicator-appearance').trim()), await input.evaluate(node => getComputedStyle(node).getPropertyValue('--cem-indicator-appearance-outline').trim()));
        await page.getByRole('group', { name: 'dir', exact: true }).getByRole('radio', { name: 'rtl', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('.properties cem-datepicker')?.getAttribute('dir') === 'rtl');
        await text('form', 'date-external'); assert.equal(await input.evaluate(node => node.form?.id), 'date-external');
        await page.getByRole('button', { name: 'Reset external date', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('.properties form > cem-datepicker input')?.value === '2026-08-11');
        await flag('hidden', true); assert.equal(await input.isVisible(), false); await flag('hidden', false);
        assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
        console.log(`Datepicker forced colors and bundle: ${baseUrl}`);
        await page.goto(new URL('cem-datepicker-gallery.html', baseUrl).href);
        const first = page.locator('[data-gallery-theme] cem-datepicker').first();
        await first.getByRole('combobox').waitFor(); await first.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await page.emulateMedia({ forcedColors: 'active' });
        await first.locator('input').focus(); await first.locator('input').press('ArrowDown'); await first.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        assert.equal(await first.locator('dialog').evaluate(node => node.matches(':modal')), true);
        assert.equal(await first.locator('dialog').evaluate(node => getComputedStyle(node).boxShadow), 'none');
        assert.equal(await first.locator('[role=gridcell][tabindex="0"]').count(), 1);
        await page.keyboard.press('Escape'); await page.emulateMedia({ forcedColors: 'none' });
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        const bundled = page.getByRole('combobox', { name: 'Bundled date', exact: true }); await bundled.waitFor();
        await bundled.focus(); await bundled.press('ArrowDown');
        const calendar = page.locator('cem-datepicker'); await calendar.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await calendar.locator('[data-date="2026-08-12"]').click(); await calendar.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await calendar.locator('[data-calendar-action=apply]').click(); await calendar.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        assert.equal(await bundled.inputValue(), '2026-08-12'); assert.deepEqual(errors, []);
    } finally { await context.close(); }
}

async function verifyTimeRange(baseUrl) {
    console.log(`Timepicker and slider galleries: ${baseUrl}`);
    await verifyGalleries(baseUrl, ['cem-timepicker', 'cem-slider']);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        for (const tag of ['cem-timepicker', 'cem-slider']) {
            console.log(`${tag} properties and native forms: ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties form > ${tag}`), input = host.locator('input').first();
            try { await host.locator('[part=root]').waitFor(); }
            catch (error) {
                console.error(`${tag} root readiness`, await page.evaluate(tag => {
                    const host = document.querySelector(`.properties form > ${tag}`), runtime = window.cemPlaygroundRuntime;
                    return { attributes: host && [...host.attributes].map(a => [a.name, a.value]),
                        children: host && [...host.children].filter(n => n.localName !== 'template').map(n => n.outerHTML), diagnostics: host && runtime?.diagnosticsFor(host),
                        declarations: [...document.querySelectorAll('cem-element')].map(node => ({ tag: node.getAttribute('tag'), diagnostics: runtime?.diagnosticsFor(node) })).filter(entry => entry.diagnostics?.length) };
                }, tag), errors);
                throw error;
            }
            const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            await settle();
            const text = async (name, value, native = false) => {
                await page.locator(`.properties cem-field[label="${name}"] input`).fill(value);
                await page.waitForFunction(({ tag, name, value, native }) => {
                    const host = document.querySelector(`.properties form > ${tag}`);
                    return (native ? host?.querySelector('input') : host)?.getAttribute(name === 'label' ? 'aria-label' : name) === value;
                }, { tag, name, value, native });
                await settle();
            };
            const radio = async (name, value) => {
                await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: value, exact: true }).check();
                await page.waitForFunction(({ tag, name, value }) => {
                    const host = document.querySelector(`.properties form > ${tag}`);
                    if (name === 'mode') return host?.querySelector('[part=root]')?.getAttribute('data-mode') === value;
                    return ['present', 'absent'].includes(value) ? host?.hasAttribute(name) === (value === 'present') : host?.getAttribute(name) === value;
                }, { tag, name, value });
                await settle();
            };
            const initial = tag === 'cem-slider' ? '50' : '09:30';
            assert.equal(await input.inputValue(), initial);
            await text('label', 'Native value', true); await text('name', 'native-value', true);
            assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), [['native-value', initial]]);
            await host.evaluate(node => { window.timeRangeEvents = []; for (const type of ['input', 'change']) node.addEventListener(type, event => window.timeRangeEvents.push([type, event.target.tagName, event.target.value])); });
            if (tag === 'cem-timepicker') {
                await text('interval', '15'); await input.focus(); await input.press('ArrowDown'); await settle();
                assert.equal(await host.locator('[part=popup]').evaluate(node => node.matches(':popover-open')), true);
                await host.locator('[data-value="10:00"]').click(); await settle();
                assert.equal(await input.inputValue(), '10:00');
                assert.deepEqual(await page.evaluate(() => window.timeRangeEvents), [['input', 'INPUT', '10:00'], ['change', 'INPUT', '10:00']]);
                await radio('required', 'present'); await input.fill('9:30'); assert.equal(await input.evaluate(node => node.validity.customError), true);
                await radio('indicator', 'outline');
            } else {
                await radio('discrete', 'present'); await radio('show-tick-marks', 'present');
                assert.equal(await host.locator('[part~=single-value]').evaluate(node => getComputedStyle(node).display), 'block');
                await input.focus(); await input.press('ArrowRight');
                assert.equal(await input.inputValue(), '55');
                assert.deepEqual(await page.evaluate(() => window.timeRangeEvents), [['input', 'INPUT', '55'], ['change', 'INPUT', '55']]);
                await radio('mode', 'range');
                assert.equal(await host.locator('input').count(), 2);
                await input.focus(); await input.press('End'); assert.equal(await input.inputValue(), '75');
            }
            await page.locator('.properties').getByRole('button', { name: 'Reset values', exact: true }).click();
            await page.waitForFunction(({ tag, initial }) => document.querySelector(`.properties form > ${tag} input`)?.value === initial, { tag, initial });
            await settle();
            await radio('disabled', 'present'); assert.equal(await input.isDisabled(), true); assert.deepEqual(await input.evaluate(node => [...new FormData(node.form)]), []); await radio('disabled', 'absent');
            await radio('dir', 'rtl'); await radio('hidden', 'present'); assert.equal(await input.isVisible(), false); await radio('hidden', 'absent');
            await text('form', `${tag.slice(4)}-external`, true); assert.equal(await input.evaluate(node => node.form.id), `${tag.slice(4)}-external`);
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
        }
        console.log(`Timepicker and slider bundle: ${baseUrl}`);
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        const time = page.locator('cem-timepicker'); await time.getByRole('combobox').waitFor();
        await time.getByRole('combobox').focus(); await time.getByRole('combobox').press('ArrowDown'); await time.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await time.locator('[data-value="10:00"]').click(); assert.equal(await time.locator('input').inputValue(), '10:00');
        const range = page.locator('cem-slider'); await range.locator('[part=root]').waitFor(); await range.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await range.locator('input').focus(); await range.locator('input').press('ArrowRight'); assert.equal(await range.locator('input').inputValue(), '51');
        await page.emulateMedia({ forcedColors: 'active' });
        const trackSize = await range.locator('[part=track]').evaluate(node => {
            const probe = document.createElement('span'); probe.style.cssText = 'position:absolute;block-size:var(--cem-slider-track-thickness)'; node.parentElement.append(probe);
            const expected = getComputedStyle(probe).height; probe.remove();
            return { actual: getComputedStyle(node).height, expected };
        });
        assert.equal(trackSize.actual, trackSize.expected);
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}

async function verifyNavigationContent(baseUrl) {
    const tags = ['cem-nav', 'cem-tabs', 'cem-stepper', 'cem-tree', 'cem-list', 'cem-chip'];
    await verifyGalleries(baseUrl, tags);
    await verifyProjectNavigation(baseUrl);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const interact = async (host, tag) => {
        const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
        await host.locator('[part]').first().waitFor(); await settle();
        if (tag === 'cem-nav' || tag === 'cem-chip') {
            const owner = host.locator('button').first(), attribute = tag === 'cem-nav' ? 'aria-expanded' : 'aria-pressed';
            const initial = await owner.getAttribute(attribute);
            await owner.focus(); await owner.press('Space'); await settle();
            assert.equal(await owner.getAttribute(attribute), initial === 'true' ? 'false' : 'true');
            assert.equal(await owner.evaluate(node => document.activeElement === node), true);
        } else if (tag === 'cem-tabs' || tag === 'cem-stepper') {
            const owners = host.locator(tag === 'cem-tabs' ? '[part=tab]' : '[part=header]');
            await owners.first().focus(); await owners.first().press('ArrowRight'); await settle();
            assert.equal(await owners.nth(1).evaluate(node => document.activeElement === node), true);
            assert.equal(await host.evaluate(node => node.selectedIndex), 0);
            await owners.nth(1).press('Enter'); await settle(); assert.equal(await host.evaluate(node => node.selectedIndex), 1);
        } else if (tag === 'cem-tree') {
            const owner = host.locator('[part=node]').first(); await owner.focus();
            const initial = await owner.getAttribute('aria-expanded'); await owner.press(initial === 'true' ? 'ArrowLeft' : 'ArrowRight'); await settle();
            assert.equal(await owner.getAttribute('aria-expanded'), initial === 'true' ? 'false' : 'true');
        } else {
            const owner = host.locator('select'); await owner.selectOption('document'); await settle();
            assert.equal(await owner.inputValue(), 'document');
            assert.equal(await host.evaluate(node => window.cemPlaygroundRuntime.snapshotInstance(node).slices.value), 'document');
        }
        assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
    };
    try {
        for (const tag of tags) {
            console.log(`${tag} properties: ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).last();
            await interact(host, tag);
            await page.locator('.properties cem-field[label="label"] input').fill('Renamed owner');
            await page.waitForFunction(tag => document.querySelector(`.properties ${tag}`)?.getAttribute('label') === 'Renamed owner', tag);
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            const owner = host.locator(tag === 'cem-tabs' ? '[role=tablist]' : tag === 'cem-chip' ? 'button' : '[part=root]').first();
            assert.equal(await owner.getAttribute('aria-label'), 'Renamed owner');
            await page.getByRole('group', { name: 'hidden', exact: true }).getByRole('radio', { name: 'present', exact: true }).check();
            await host.waitFor({ state: 'hidden' });
            await page.getByRole('group', { name: 'hidden', exact: true }).getByRole('radio', { name: 'absent', exact: true }).check();
            await host.waitFor({ state: 'visible' });
        }
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        for (const tag of tags) await interact(page.locator(tag).first(), tag);
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}

async function verifyProjectNavigation(baseUrl) {
    console.log(`Shared project navigation: ${baseUrl}`);
    const context = await browser.newContext({ viewport: { width: 360, height: 800 } });
    const page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(new URL('cem-tree-gallery.html', baseUrl).href);
        await page.waitForFunction(() => ['cem-action', 'cem-list', 'cem-tree', 'cem-tabs', 'cem-dialog', 'cem-stack', 'cem-alert', 'cem-surface'].every(tag => customElements.get(tag)));
        await page.evaluate(async ({ source, textareaSource }) => {
            const { mountCemView } = await import('@epa-wg/cem-elements');
            const runtime = window.cemPlaygroundRuntime;
            const textarea = document.createElement('cem-element');
            textarea.setAttribute('tag', 'cem-textarea'); textarea.setAttribute('capability', 'form-control'); textarea.setAttribute('src', `${textareaSource}#cem-textarea`);
            document.body.append(textarea); await runtime.whenDeclarationSettled(textarea);
            const declaration = document.createElement('cem-element');
            declaration.setAttribute('tag', 'project-navigation-example'); declaration.setAttribute('src', `${source}#project-navigation-example`);
            document.body.append(declaration); await runtime.whenDeclarationSettled(declaration);
            const root = document.createElement('section'); root.setAttribute('aria-label', 'Project navigation example'); document.body.append(root);
            const records = { alpha: 'Alpha saved', beta: 'Beta saved' };
            let state = { selected: 'alpha', draft: records.alpha, saved: records.alpha, dirty: false, pending: '', saving: false, failed: false, status: 'ready' };
            const publish = () => { window.projectNavigationState = state; view.update(state); };
            const commit = value => { state = { ...state, selected: value, draft: records[value], saved: records[value], dirty: false, pending: '' }; publish(); };
            const navigate = value => { if (state.dirty) { state = { ...state, pending: value }; publish(); } else commit(value); };
            const view = window.projectNavigationView = mountCemView(runtime, { root, tag: 'project-navigation-example', state, commands: {
                navigate: ({ value }) => navigate(value), 'open-alpha': () => navigate('alpha'), 'open-beta': () => navigate('beta'),
                edit: ({ value }) => { state = { ...state, draft: value, dirty: value !== state.saved }; publish(); },
                cancel: () => { state = { ...state, pending: '' }; publish(); },
                discard: () => commit(state.pending),
                save: () => { records[state.selected] = state.draft; commit(state.pending); },
            } });
            publish(); await view.whenSettled();
        }, { source: new URL('../playgrounds/project-navigation.xhtml', baseUrl).href,
            textareaSource: new URL('../src/components/cem-textarea/cem-textarea.xhtml', baseUrl).href });
        const root = page.getByRole('region', { name: 'Project navigation example' });
        const settle = () => page.evaluate(() => window.projectNavigationView.whenSettled());
        const beta = root.locator('[data-tree-value=beta]'), editor = root.locator('textarea');
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            await root.evaluate((node, theme) => { node.className = `cem-theme-${theme}`; node.style.setProperty('--cem-layout-stack-gap', '23px'); node.style.setProperty('--cem-stroke-focus', '3px'); }, theme);
            await editor.fill(`Draft in ${theme}`); await settle();
            await editor.evaluate(node => { window.navigationEditor = node; node.setSelectionRange(2, 5); });
            await root.getByRole('tab', { name: 'Preview', exact: true }).click(); await settle();
            await root.getByRole('tab', { name: 'Editor', exact: true }).click(); await settle();
            assert.deepEqual(await editor.evaluate(node => [node === window.navigationEditor, node.selectionStart, node.selectionEnd]), [true, 2, 5]);
            await beta.focus(); await beta.press('Enter'); await settle();
            const dialog = root.getByRole('dialog', { name: 'Save changes before opening another resource?' });
            await dialog.waitFor({ state: 'visible' });
            assert.equal(await beta.getAttribute('aria-selected'), 'false');
            await dialog.getByRole('button', { name: 'Cancel', exact: true }).click(); await settle();
            await page.waitForFunction(() => document.activeElement?.getAttribute('data-tree-value') === 'beta');
            assert.equal(await dialog.count(), 0);
            assert.equal(await editor.inputValue(), `Draft in ${theme}`);
            await beta.press('ArrowDown');
            assert.equal(await root.locator('cem-stack > [part=root]').first().evaluate(node => getComputedStyle(node).gap), '23px');
            assert.equal(await beta.evaluate(node => getComputedStyle(node).outlineWidth), '3px');
            assert(await root.evaluate(node => node.scrollWidth <= node.clientWidth + 1), `${theme}: compact workflow overflow`);
        }
        await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
        await beta.focus(); await beta.press('ArrowUp'); await beta.press('ArrowDown');
        const paint = await beta.evaluate(node => {
            const probe = document.createElement('span'); probe.style.color = 'CanvasText'; document.body.append(probe);
            const canvasText = getComputedStyle(probe).color; probe.remove();
            const style = getComputedStyle(node); return { outline: style.outlineColor, canvasText, adjust: style.forcedColorAdjust, focused: node.matches(':focus-visible') };
        });
        assert.equal(paint.outline, paint.canvasText); assert.equal(paint.adjust, 'auto'); assert.equal(paint.focused, true);
        assert.deepEqual(await page.evaluate(() => window.projectNavigationView.errors.map(error => String(error))), []);
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}


async function verifyLayout(baseUrl) {
    const tags = ['cem-surface', 'cem-text', 'cem-stack', 'cem-grid', 'cem-divider', 'cem-card'];
    console.log(`Passive component galleries: ${baseUrl}`);
    await verifyGalleries(baseUrl, tags);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        for (const tag of tags) {
            console.log(`Passive component properties: ${tag} at ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).last(), owner = host.locator('[part=root]');
            await owner.waitFor();
            const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            await settle();
            const flag = async (name, value) => {
                await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: value, exact: true }).check();
                await page.waitForFunction(({tag, name, value}) => {
                    const node = document.querySelector(`.properties ${tag}`);
                    return ['present', 'absent'].includes(value) ? node?.hasAttribute(name) === (value === 'present') : node?.getAttribute(name) === value;
                }, {tag, name, value});
                await settle();
            };
            if (tag === 'cem-surface' || tag === 'cem-card') {
                const input = host.locator('input'); await input.fill('Edited draft');
                await owner.evaluate(node => { window.retainedLayoutOwner = node; window.retainedLayoutInput = node.querySelector('input'); });
                if (tag === 'cem-surface') { await flag('empty', 'present'); assert.equal(await owner.getAttribute('data-state'), 'empty'); }
                await flag('busy', 'present'); assert.equal(await owner.getAttribute('data-state'), 'loading'); assert.equal(await owner.getAttribute('aria-busy'), 'true');
                await flag('busy', 'absent'); assert.equal(await owner.getAttribute('data-state'), tag === 'cem-surface' ? 'empty' : null);
                assert.equal(await input.inputValue(), 'Edited draft');
                assert.equal(await owner.evaluate(node => node === window.retainedLayoutOwner && node.querySelector('input') === window.retainedLayoutInput), true);
                if (tag === 'cem-surface') { await flag('tone', 'trust'); await flag('empty', 'absent'); }
                const label = page.locator('.properties cem-field[label="label"] input'); await label.fill('Renamed content');
                await page.waitForFunction(tag => document.querySelector(`.properties ${tag} [part=root]`)?.getAttribute('aria-label') === 'Renamed content', tag);
                await page.emulateMedia({ forcedColors: 'active' });
                assert.equal(await owner.evaluate(node => getComputedStyle(node).forcedColorAdjust), 'auto');
                assert.equal(await owner.evaluate(node => getComputedStyle(node).boxShadow), 'none');
                assert.equal(await owner.evaluate(node => getComputedStyle(node).borderBlockStartStyle), 'solid');
                await page.emulateMedia({ forcedColors: 'none' });
            } else if (tag === 'cem-divider') {
                await flag('orientation', 'vertical'); assert.equal(await owner.getAttribute('aria-orientation'), 'vertical');
                assert(await owner.evaluate(node => node.getBoundingClientRect().height) > 0);
                await flag('decorative', 'present'); assert.equal(await owner.getAttribute('aria-hidden'), 'true'); assert.equal(await owner.getAttribute('role'), null);
                await flag('decorative', 'absent'); assert.equal(await owner.getAttribute('role'), 'separator');
                await flag('spacing', 'section'); await flag('inset', 'present');
            } else if (tag === 'cem-text') {
                await page.locator('.properties cem-field[label="text"] input').fill('Updated fallback');
                await page.waitForFunction(() => document.querySelector('.properties cem-text [part=root]')?.textContent === 'Updated fallback');
                await flag('variant', 'data');
            } else {
                await flag('gap', 'lg'); assert.equal(await owner.getAttribute('data-gap'), 'lg');
                assert.equal(await owner.evaluate(node => getComputedStyle(node).display), tag === 'cem-stack' ? 'flex' : 'grid');
                if (tag === 'cem-grid') { await flag('columns', '3'); assert.equal(await owner.evaluate(node => getComputedStyle(node).gridTemplateColumns.split(' ').length), 3); }
            }
            assert.equal(await owner.getAttribute('tabindex'), null);
            await flag('dir', 'rtl'); await flag('hidden', 'present'); assert.equal(await owner.isVisible(), false); await flag('hidden', 'absent');
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
            const source = page.getByRole('link', { name: 'Open canonical XHTML', exact: true }); await source.waitFor();
            const response = await page.request.get(await source.getAttribute('href'));
            assert.equal(await response.text(), await readFile(join(root, `packages/cem-components/src/components/${tag}/${tag}.xhtml`), 'utf8'));
        }
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        for (const tag of tags) {
            const host = page.locator(`main ${tag}`).first(); await host.locator('[part=root]').waitFor();
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
        }
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}


async function verifyDisclosureData(baseUrl) {
    const tags = ['cem-expansion', 'cem-table', 'cem-sort-header', 'cem-paginator'];
    await verifyGalleries(baseUrl, tags);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        for (const tag of tags) {
            console.log(`Disclosure/data properties: ${tag} at ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).last(), owner = host.locator(':scope > [part=root]');
            await owner.waitFor();
            const settle = () => host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            await settle();
            const flag = async (name, value) => {
                await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: value, exact: true }).check();
                await page.waitForFunction(({tag, name, value}) => {
                    const node = document.querySelector(`.properties ${tag}`);
                    return ['present', 'absent'].includes(value) ? node?.hasAttribute(name) === (value === 'present') : node?.getAttribute(name) === value;
                }, {tag, name, value});
                await settle();
            };
            await host.evaluate(node => { window.retainedDataOwner = node.querySelector(':scope > [part=root]'); window.dataEvents = []; for (const type of ['cem-sort', 'cem-page']) node.addEventListener(type, event => window.dataEvents.push({type, detail: event.detail})); });
            if (tag === 'cem-expansion') {
                const button = host.locator('[part=header]'), panel = host.locator('[part=panel]');
                assert.equal(await panel.isVisible(), false);
                await button.click(); await settle(); assert.equal(await panel.isVisible(), true);
                const input = host.locator('input'); await input.fill('Retained edit');
                await button.press('Enter'); await settle(); assert.equal(await panel.isVisible(), false);
                await flag('expanded', 'present'); await flag('region', 'present'); await flag('disabled', 'present');
                assert.equal(await button.isDisabled(), true); assert.equal(await panel.getAttribute('role'), 'region'); assert.equal(await input.inputValue(), 'Retained edit');
                await flag('disabled', 'absent'); await flag('heading-level', '2'); assert.equal(await host.locator('[part=heading]').getAttribute('aria-level'), '2');
            } else if (tag === 'cem-sort-header') {
                const button = host.locator('button'); await button.click(); await settle(); assert.equal(await owner.getAttribute('aria-sort'), 'ascending');
                await button.press('Enter'); await settle(); assert.equal(await owner.getAttribute('aria-sort'), 'descending');
                await button.press('Space'); await settle(); assert.equal(await owner.getAttribute('aria-sort'), null);
                assert.deepEqual(await page.evaluate(() => window.dataEvents.map(event => event.detail.direction)), ['ascending','descending','none']);
                await flag('direction', 'descending'); await flag('disabled', 'present'); assert.equal(await button.isDisabled(), true);
                assert.equal(await page.evaluate(() => window.dataEvents.length), 3);
            } else if (tag === 'cem-paginator') {
                const next = host.locator('[data-page-action=next]'); await next.click(); await settle();
                assert.equal(await host.locator('[part=range]').textContent(), '51 – 75 of 120');
                await host.locator('select').selectOption('10'); await settle(); assert.equal(await host.locator('[part=range]').textContent(), '51 – 60 of 120');
                assert.equal(await page.evaluate(() => window.dataEvents.length), 2);
                await flag('show-first-last', 'present'); await flag('hide-page-size', 'present');
                assert.equal(await host.locator('[part=action]').count(), 4); assert.equal(await host.locator('select').count(), 0);
                await flag('disabled', 'present'); assert.equal(await next.isDisabled(), true);
                assert.equal(await page.evaluate(() => window.dataEvents.length), 2);
            } else {
                assert.equal(await owner.getAttribute('role'), 'table'); assert.equal(await host.locator('[role=row]').count(), 2);
                await page.locator('.properties cem-field[label="label"] input').fill('Renamed records');
                await page.waitForFunction(() => document.querySelector('.properties cem-table [part=root]')?.getAttribute('aria-label') === 'Renamed records');
                assert.equal(await page.evaluate(() => window.dataEvents.length), 0);
            }
            assert.equal(await owner.evaluate(node => node === window.retainedDataOwner), true);
            await flag('dir', 'rtl'); await flag('hidden', 'present'); assert.equal(await owner.isVisible(), false); await flag('hidden', 'absent');
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
            await page.emulateMedia({ forcedColors: 'active' }); assert.equal(await owner.evaluate(node => getComputedStyle(node).forcedColorAdjust), 'auto'); await page.emulateMedia({ forcedColors: 'none' });
            const source = page.getByRole('link', { name: 'Open canonical XHTML', exact: true }); await source.waitFor();
            const response = await page.request.get(await source.getAttribute('href'));
            assert.equal(await response.text(), await readFile(join(root, `packages/cem-components/src/components/${tag}/${tag}.xhtml`), 'utf8'));
        }
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        for (const tag of tags) {
            const host = page.locator(`main ${tag}`).first(); await host.locator(':scope > [part=root]').waitFor();
            await host.evaluate(node => window.cemPlaygroundRuntime.whenRenderSettled(node));
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
        }
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
}

async function verifyFeedback(baseUrl) {
    const tags = ['cem-toast', 'cem-progress', 'cem-progress-spinner', 'cem-skeleton', 'cem-alert'];
    await verifyGalleries(baseUrl, tags);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        for (const tag of tags) {
            console.log(`Feedback properties: ${tag} at ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).last(), owner = host.locator(':scope > [part=root]');
            await owner.waitFor();
            const settle = () => host.evaluate(async node => {
                const runtime = window.cemPlaygroundRuntime;
                await runtime.whenRenderSettled(node.closest('.properties'));
                await runtime.whenRenderSettled(node);
            });
            await settle();
            await host.evaluate(node => { window.feedbackOwner = node.querySelector(':scope > [part=root]'); });
            const choose = async (name, value) => {
                await page.getByRole('group', { name, exact: true }).getByRole('radio', { name: value, exact: true }).check();
                await page.waitForFunction(({tag, name, value}) => {
                    const host = document.querySelector(`.properties ${tag}`);
                    if (name === 'mode') return host?.hasAttribute('value') === (value === 'determinate');
                    if (name === 'content') return !!host?.querySelector('strong') === (value === 'projected');
                    if (name === 'hidden') return host?.hasAttribute(name) === (value === 'present');
                    return host?.getAttribute(name) === value;
                }, {tag, name, value});
                await settle();
            };
            const fill = async (name, value) => {
                await page.getByRole('textbox', { name, exact: true }).fill(value);
                await page.waitForFunction(({tag, name, value}) => document.querySelector(`.properties ${tag}`)?.getAttribute(name) === value, {tag, name, value});
                await settle();
            };
            await fill('label', 'Updated task');
            if (tag === 'cem-progress' || tag === 'cem-progress-spinner') {
                await choose('mode', 'determinate'); await fill('max', '12'); await fill('value', '30');
                if (tag === 'cem-progress') {
                    assert.equal(await owner.evaluate(node => node.value), 12); assert.equal(await owner.evaluate(node => node.max), 12);
                } else {
                    assert.equal(await owner.getAttribute('aria-valuenow'), '12'); assert.equal(await owner.getAttribute('aria-valuemax'), '12');
                    assert.equal(await host.locator('[part=indicator]').getAttribute('stroke-dasharray'), '100 0');
                }
                assert.equal(await owner.getAttribute('aria-label'), 'Updated task');
                await fill('value', '');
                assert.equal(await host.getAttribute('value'), '');
                assert.equal(await owner.evaluate(node => node instanceof HTMLProgressElement ? node.position : Number(node.getAttribute('aria-valuenow'))), 0);
                await choose('mode', 'indeterminate');
                assert.equal(await host.getAttribute('value'), null);
                if (tag === 'cem-progress') assert.equal(await owner.evaluate(node => node.position), -1);
                else assert.equal(await owner.getAttribute('aria-valuenow'), null);
            } else {
                assert.equal((await owner.textContent()).trim(), 'Updated task');
                await choose('content', 'projected');
                assert.equal((await owner.textContent()).trim(), 'Authored message markup');
                if (tag === 'cem-toast') { assert.equal(await owner.getAttribute('role'), 'status'); assert.equal(await owner.getAttribute('aria-live'), 'polite'); }
                if (tag === 'cem-alert') { await choose('tone', 'danger'); await choose('role', 'alert'); assert.equal(await owner.getAttribute('data-tone'), 'danger'); assert.equal(await owner.getAttribute('role'), 'alert'); }
                if (tag === 'cem-skeleton') { assert.equal(await owner.getAttribute('aria-hidden'), 'true'); assert.equal(await owner.evaluate(node => node.inert), true); }
            }
            await choose('dir', 'rtl'); assert.equal(await owner.evaluate(node => getComputedStyle(node).direction), 'rtl');
            await choose('hidden', 'present'); assert.equal(await owner.isVisible(), false);
            await choose('hidden', 'absent'); assert.equal(await owner.isVisible(), true);
            assert.equal(await owner.evaluate(node => node === window.feedbackOwner), true, `${tag}: retained owner`);
            for (const reducedMotion of ['no-preference', 'reduce']) {
                await page.emulateMedia({ forcedColors: 'active', reducedMotion });
                const state = await host.evaluate(node => {
                    const owner = node.querySelector('[part=root]'), style = getComputedStyle(owner);
                    const resolveColor = value => { const probe = document.createElement('span'); probe.style.color = value; document.body.append(probe); const result = getComputedStyle(probe).color; probe.remove(); return result; };
                    const indicator = node.querySelector('[part=indicator]'), track = node.querySelector('[part=track]');
                    return { color: style.color, background: style.backgroundColor, border: style.borderTopColor, adjustment: style.forcedColorAdjust,
                        canvas: resolveColor('Canvas'), text: resolveColor('CanvasText'), gray: resolveColor('GrayText'), highlight: resolveColor('Highlight'),
                        indicator: indicator && getComputedStyle(indicator).stroke, track: track && getComputedStyle(track).stroke,
                        animation: indicator && getComputedStyle(indicator).animationName,
                        dash: indicator?.getAttribute('stroke-dasharray'), width: owner.getBoundingClientRect().width };
                });
                assert.equal(state.adjustment, 'auto');
                if (tag === 'cem-progress-spinner') {
                    assert.equal(state.indicator, state.highlight); assert.equal(state.track, state.gray); assert.equal(state.dash, '25 75');
                    if (reducedMotion === 'reduce') assert.equal(state.animation, 'none');
                    else assert.match(state.animation, /^cem-progress-spinner-cycle-cem-[a-f0-9]+$/);
                    assert(state.width > 0);
                } else if (tag === 'cem-toast' || tag === 'cem-alert') { assert.equal(state.color, state.text); assert.equal(state.background, state.canvas); }
                else if (tag === 'cem-skeleton') assert.equal(state.border, state.gray);
            }
            await page.emulateMedia({ forcedColors: 'none', reducedMotion: 'no-preference' });
            assert.deepEqual(await host.evaluate(node => window.cemPlaygroundRuntime.diagnosticsFor(node)), []);
            const source = page.getByRole('link', { name: 'Open canonical XHTML', exact: true }); await source.waitFor();
            assert((await page.request.get(new URL(await source.getAttribute('href'), page.url()).href)).ok());
        }
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
    console.log(`Feedback properties, native semantics, forced colors and reduced motion verified: ${baseUrl}`);
}

async function verifyPassive(baseUrl) {
    await verifyGalleries(baseUrl, passiveTags);
    const context = await browser.newContext(), page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        for (const tag of passiveTags) {
            console.log(`Passive properties: ${tag} at ${baseUrl}`);
            await page.goto(new URL(`${tag}.html`, baseUrl).href);
            const host = page.locator(`.properties ${tag}`).first(), owner = host.locator(':scope > [part=root]');
            await owner.waitFor({ state: 'attached' });
            await host.evaluate(node => { window.passiveOwner = node.querySelector(':scope > [part=root]'); });
            const settle = () => host.evaluate(async node => {
                await window.cemPlaygroundRuntime.whenRenderSettled(node.closest('.properties'));
                await window.cemPlaygroundRuntime.whenRenderSettled(node);
            });
            await page.getByRole('textbox', { name: 'label', exact: true }).fill('Updated label');
            await page.waitForFunction(tag => document.querySelector(`.properties ${tag}`)?.getAttribute('label') === 'Updated label', tag);
            await settle();
            if (['cem-avatar', 'cem-media-preview', 'cem-app-bar'].includes(tag)) assert.equal(await owner.getAttribute('aria-label'), 'Updated label');
            if (tag === 'cem-badge') assert.equal(await owner.innerText(), 'Updated label');
            for (const name of await page.locator('.properties fieldset[aria-label]').evaluateAll(nodes => nodes.map(node => node.getAttribute('aria-label')))) {
                if (name === 'content') continue;
                const radios = page.getByRole('group', { name, exact: true })
                    .or(page.getByRole('radiogroup', { name, exact: true })).getByRole('radio');
                for (let index = 0; index < await radios.count(); index++) {
                    const radio = radios.nth(index), value = await radio.getAttribute('value');
                    await radio.check();
                    await page.waitForFunction(({ tag, name, value }) => {
                        const host = document.querySelector(`.properties ${tag}`);
                        return value === 'absent' || value === 'present' ? host?.hasAttribute(name) === (value === 'present') : host?.getAttribute(name) === value;
                    }, { tag, name, value });
                    await settle();
                    if (name === 'tone') assert.equal(await owner.getAttribute('data-tone'), value);
                    if (name === 'dir') assert.equal(await owner.evaluate(node => getComputedStyle(node).direction), value);
                }
            }
            await page.getByRole('group', { name: 'hidden', exact: true }).or(page.getByRole('radiogroup', { name: 'hidden', exact: true })).getByRole('radio', { name: 'absent', exact: true }).check(); await settle();
            if (tag === 'cem-media-preview') {
                const mode = page.getByRole('radiogroup', { name: 'mode', exact: true });
                const setMode = async value => {
                    await mode.getByRole('radio', { name: value, exact: true }).check();
                    await page.waitForFunction(value => document.querySelector('.properties cem-media-preview')?.getAttribute('mode') === value, value);
                    await settle();
                };
                await setMode('text');
                const value = '  <p>Exact output</p>\n';
                await page.getByRole('textbox', { name: 'value', exact: true }).fill(value);
                await page.waitForFunction(value => document.querySelector('.properties cem-media-preview')?.getAttribute('value') === value, value);
                await settle();
                assert.equal(await host.locator('pre').textContent(), value);
                await page.getByRole('checkbox', { name: 'truncated', exact: true }).check();
                await page.waitForFunction(() => document.querySelector('.properties cem-media-preview')?.hasAttribute('truncated'));
                await settle();
                assert.equal(await host.locator('pre').getAttribute('aria-label'), 'Updated label (truncated)');
                await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
                await page.keyboard.press('Tab');
                await host.locator('pre').focus();
                const outline = await host.locator('pre').evaluate(node => getComputedStyle(node).outlineStyle);
                assert.equal(outline, 'solid');
                await setMode('document');
                const frame = host.locator('iframe');
                assert.equal(await frame.getAttribute('sandbox'), '');
                assert.equal(await frame.getAttribute('referrerpolicy'), 'no-referrer');
                assert((await frame.getAttribute('srcdoc')).includes(value));
                assert((await frame.getAttribute('srcdoc')).includes("script-src 'none'"));
                assert.equal(await frame.evaluate(node => getComputedStyle(node).borderTopStyle), 'solid');
                await page.emulateMedia({ forcedColors: 'none', reducedMotion: 'no-preference' });
                await setMode('media');
            }
            await page.getByRole('group', { name: 'content', exact: true }).or(page.getByRole('radiogroup', { name: 'content', exact: true })).getByRole('radio', { name: 'projected', exact: true }).check();
            await page.waitForFunction(tag => /Projected|Child/.test(document.querySelector(`.properties ${tag} > [part=root]`)?.textContent ?? '') || !!document.querySelector(`.properties ${tag} cem-tree-item[label="Child node"]`), tag);
            await settle();
            assert(await host.evaluate(node => node.querySelector(':scope > [part=root]') === window.passiveOwner), `${tag}: retained owner`);
            assert.equal(await owner.evaluate(node => node.tabIndex), -1, `${tag}: no added tab stop`);
            const source = await page.getByRole('link', { name: 'Open canonical XHTML', exact: true }).getAttribute('href');
            const declarationSource = await (await page.request.get(source)).text();
            assert.equal(await page.evaluate(({ declarationSource, tag }) => {
                const declaration = new DOMParser().parseFromString(declarationSource, 'application/xhtml+xml');
                const template = declaration.querySelector(`cem-element[tag="${tag}"] > template[id="${tag}"]`);
                return !declaration.querySelector('parsererror') && template?.getAttribute('type') === 'text/cem-ml';
            }, { declarationSource, tag }), true);
            await page.emulateMedia({ forcedColors: 'active', reducedMotion: 'reduce' });
            const forced = await owner.evaluate(node => {
                const probe = document.createElement('span'); probe.style.cssText = 'color:CanvasText;background:Canvas'; document.body.append(probe);
                const actual = getComputedStyle(node), expected = getComputedStyle(probe);
                const value = { color: actual.color, background: actual.backgroundColor, text: expected.color, canvas: expected.backgroundColor, adjust: actual.forcedColorAdjust }; probe.remove(); return value;
            });
            assert.equal(forced.adjust, 'auto');
            if (['cem-badge', 'cem-avatar', 'cem-media-preview', 'cem-app-bar'].includes(tag)) { assert.equal(forced.color, forced.text); assert.equal(forced.background, forced.canvas); }
            await page.emulateMedia({ forcedColors: 'none', reducedMotion: 'no-preference' });
        }
        assert.deepEqual(errors, []);
    } finally { await context.close(); }
    await verifyBundle(new URL('cem-bundle.html', baseUrl).href);
}

async function verifyRecoveryStatus(baseUrl) {
    const page = await browser.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    try {
        await page.goto(`${baseUrl}cem-recovery-status.html`);
        const host = page.locator('main .properties cem-recovery-status'), summary = host.locator('summary');
        await summary.waitFor();
        assert.match(await summary.textContent(), /Not configured/);
        await summary.click();
        await page.getByRole('textbox', { name: 'label', exact: true }).fill('Workspace');
        await page.getByRole('radio', { name: 'observing', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('main .properties cem-recovery-status summary')?.textContent.includes('Workspace · Status observed'));
        assert.equal(await host.locator('details').evaluate(node => node.open), true);
        await page.getByRole('textbox', { name: 'runtime-count', exact: true }).fill('3');
        await page.waitForFunction(() => document.querySelector('main .properties cem-recovery-status')?.textContent.includes('Runtime registrations: 3.'));
        assert.match(await host.textContent(), /Entered passwords or one-time codes block recovery/);
        assert.match(await host.textContent(), /Automatic recovery is off/);
        assert.equal(await host.locator('button, input, a').count(), 0);
        assert.equal(await page.getByRole('link', { name: 'Full examples and variation matrix', exact: true }).getAttribute('href'), './cem-recovery-status-gallery.html');
        await page.goto(`${baseUrl}cem-recovery-status-gallery.html`);
        const examples = page.locator('main cem-demo-element cem-recovery-status');
        await examples.first().locator('summary').waitFor();
        await page.waitForFunction(() => document.querySelectorAll('main cem-demo-element cem-recovery-status').length === 14);
        assert.equal(await examples.count(), 14);
        const themed = page.locator('main [data-gallery-theme] cem-recovery-status');
        assert.equal(await themed.count(), 5);
        for (const host of await themed.all()) {
            await host.evaluate(node => {
                node.style.setProperty('--cem-typography-ui-font-size', '19px');
                node.style.setProperty('--cem-palette-comfort-text', 'rgb(24, 36, 48)');
                node.style.setProperty('--cem-coupling-zone-min', '57px');
            });
            assert.deepEqual(await host.locator('summary').evaluate(node => ({ size: getComputedStyle(node).fontSize, color: getComputedStyle(node).color, target: getComputedStyle(node).minBlockSize })),
                { size: '19px', color: 'rgb(24, 36, 48)', target: '57px' });
        }
        await page.emulateMedia({ forcedColors: 'active' });
        await page.keyboard.press('Tab');
        const first = examples.first().locator('summary'); await first.focus();
        const forced = await first.evaluate(node => {
            const probe = document.createElement('span'); probe.style.color = 'CanvasText'; document.body.append(probe);
            const text = getComputedStyle(probe).color; probe.style.color = 'Highlight'; const highlight = getComputedStyle(probe).color; probe.remove();
            const style = getComputedStyle(node);
            return { active: matchMedia('(forced-colors: active)').matches, color: style.color, text, highlight, outline: style.outlineColor, width: parseFloat(style.outlineWidth), focus: node.matches(':focus-visible') };
        });
        assert.equal(forced.active, true); assert.equal(forced.color, forced.text);
        assert.equal(forced.focus, true); assert.equal(forced.outline, forced.highlight); assert(forced.width > 0);
        assert.deepEqual(errors, []);
    } finally { await page.close(); }
}
