import assert from 'node:assert/strict';
import { PNG } from 'pngjs';

// Exercise shipped declarations and CSS, including actual top-layer pixels.
export async function verifyModalBackdrops(browser, baseUrl) {
    const context = await browser.newContext({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
    const page = await context.newPage(), errors = [];
    page.on('pageerror', error => errors.push(error.message));
    const pixel = async () => {
        const png = PNG.sync.read(await page.screenshot({ clip: { x: 0, y: 0, width: 8, height: 8 }, animations: 'disabled' }));
        return [...png.data.subarray((4 * png.width + 4) * 4, (4 * png.width + 4) * 4 + 3)];
    };
    const rgb = color => color.match(/[\d.]+/g).slice(0, 3).map(Number);
    const blend = (base, color, opacity, painted, label) => base.forEach((channel, index) => {
        assert(Math.abs(painted[index] - (channel * (1 - opacity) + color[index] * opacity)) <= 2, label + ': ' + JSON.stringify({ base, color, opacity, painted }));
    });
    const read = which => page.evaluate(which => {
        const host = window.backdropFixture[which], owner = host.querySelector('dialog'), style = getComputedStyle(owner, '::backdrop');
        const probe = document.createElement('span'); probe.style.color = 'var(--cem-scrim-color)'; owner.append(probe);
        const expectedColor = getComputedStyle(probe).color; probe.remove();
        return { background: style.backgroundColor, opacity: Number(style.opacity), adjustment: style.forcedColorAdjust,
            expectedColor, modal: owner.matches(':modal'), popover: owner.matches(':popover-open') };
    }, which);
    const open = async which => {
        await page.waitForFunction(which => window.backdropFixture[which]?.querySelector('[part=trigger], button[slot=toggle]'), which);
        await page.evaluate(which => { const trigger = window.backdropFixture[which].querySelector('[part=trigger], button[slot=toggle]'); trigger.focus(); trigger.click(); }, which);
        await page.waitForFunction(which => { const owner = window.backdropFixture[which].querySelector('dialog'); return owner.open || owner.matches(':popover-open'); }, which);
    };
    const close = async which => {
        await page.keyboard.press('Escape');
        await page.waitForFunction(which => { const owner = window.backdropFixture[which].querySelector('dialog'); return !owner.open && !owner.matches(':popover-open'); }, which);
    };
    const configure = (which, popover) => page.evaluate(async ({ which, popover }) => {
        const host = window.backdropFixture[which]; host.setAttribute('mode', popover === null ? 'modal' : 'nonmodal');
        if (popover) host.setAttribute('popover', popover); else host.removeAttribute('popover');
        await window.cemPlaygroundRuntime.whenRenderSettled(host);
    }, { which, popover });
    try {
        await page.goto(new URL('cem-bundle.html', baseUrl).href);
        await page.locator('main > cem-dialog dialog').waitFor({ state: 'attached' });
        for (const tag of ['cem-dialog', 'cem-dialog-shell']) {
            await page.evaluate(async tag => {
                window.backdropFixture?.stage.remove();
                const stage = document.createElement('section');
                document.documentElement.style.minHeight = '300vh'; window.scrollTo(0, 0);
                stage.style.cssText = 'position:fixed;inset:0;background:white;z-index:2147483647';
                const outer = document.createElement(tag), inner = document.createElement(tag === 'cem-dialog' ? 'cem-dialog-shell' : 'cem-dialog');
                for (const [host, label] of [[outer, 'Outer task'], [inner, 'Inner task']]) {
                    host.setAttribute('label', label); host.setAttribute('mode', 'modal'); host.setAttribute('trigger', label); host.setAttribute('close-label', 'Done');
                    const input = document.createElement('input'); input.setAttribute('aria-label', label + ' entry'); input.autofocus = true; host.append(input);
                }
                outer.append(inner); stage.append(outer); document.body.append(stage);
                window.backdropFixture = { stage, outer, get inner() { return outer.querySelector(inner.localName); } };
                await window.cemPlaygroundRuntime.whenRenderSettled(outer);
                await window.cemPlaygroundRuntime.whenRenderSettled(window.backdropFixture.inner);
            }, tag);
            const profiles = ['light', 'dark', 'contrast-light', 'contrast-dark', 'native', 'projector', 'native-dark'];
            for (const profile of profiles) for (const forced of [false, true]) {
                const scheme = profile.includes('dark') ? 'dark' : 'light';
                await page.emulateMedia({ forcedColors: forced ? 'active' : 'none', colorScheme: scheme });
                await page.evaluate(profile => { window.backdropFixture.stage.className = 'cem-theme-' + (profile === 'native-dark' ? 'native' : profile); }, profile);
                const baseline = await pixel(); await open('outer'); const paint = await read('outer');
                assert(paint.modal); assert.equal(paint.opacity, 0.32); assert.equal(paint.background, paint.expectedColor);
                if (forced) assert.equal(paint.adjustment, 'auto');
                blend(baseline, rgb(paint.background), paint.opacity, await pixel(), tag + '/' + profile + '/forced=' + forced);
                await close('outer'); assert.deepEqual(await pixel(), baseline, 'Closure removes the veil');
            }
            await page.emulateMedia({ forcedColors: 'none', colorScheme: 'light' });
            await page.evaluate(() => {
                const stage = window.backdropFixture.stage; stage.className = 'cem-theme-light';
                stage.style.setProperty('--cem-scrim-color', 'rgb(20, 40, 60)'); stage.style.setProperty('--cem-scrim-opacity', '0.5');
            });
            const baseline = await pixel(); await open('outer'); const outer = await read('outer');
            assert.equal(outer.background, 'rgb(20, 40, 60)'); assert.equal(outer.opacity, 0.5);
            const single = await pixel(); blend(baseline, rgb(outer.background), 0.5, single, 'Inherited overrides');
            // Compare with an unadapted native reference: non-scrollable
            // overscroll containment is progressive enhancement in some engines.
            const wheelResults = async reference => {
                const results = [];
                for (const inside of [false, true]) {
                    await page.evaluate(() => window.scrollTo(0, 0));
                    const point = inside ? await page.evaluate(reference => {
                        const state = window.backdropFixture;
                        const rect = (reference ? state.reference : state.outer.querySelector('dialog')).getBoundingClientRect();
                        return { x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 };
                    }, reference) : { x: 4, y: 4 };
                    await page.mouse.move(point.x, point.y); await page.mouse.wheel(0, 350);
                    await page.evaluate(async () => { for (let n = 0; n < 3; n++) await new Promise(resolve => requestAnimationFrame(resolve)); });
                    results.push(await page.evaluate(() => scrollY > 0));
                }
                await page.evaluate(() => window.scrollTo(0, 0)); return results;
            };
            await page.evaluate(() => {
                const reference = document.createElement('dialog'), style = document.createElement('style');
                reference.setAttribute('aria-label', 'Native scroll reference'); reference.style.overscrollBehavior = 'contain';
                reference.innerHTML = '<button autofocus>Native reference</button>';
                style.textContent = 'dialog[aria-label="Native scroll reference"]::backdrop { overflow:hidden; overscroll-behavior:contain; }';
                reference.append(style); window.backdropFixture.stage.append(reference); window.backdropFixture.reference = reference; reference.showModal();
            });
            const nativeScroll = await wheelResults(true);
            await page.evaluate(() => { const reference = window.backdropFixture.reference; reference.close(); reference.remove(); });
            const componentScroll = await wheelResults(false);
            assert.deepEqual(componentScroll, nativeScroll, 'Scroll containment matches the native reference');
            console.log('Modal scroll containment ' + browser.browserType().name() + '/' + tag + ': background scrolled over backdrop/inside=' + JSON.stringify(componentScroll));
            await open('inner'); const inner = await read('inner'); assert(inner.modal);
            blend(single, rgb(inner.background), inner.opacity, await pixel(), 'Nested modal backdrops composite');
            await close('inner'); assert.equal((await read('outer')).modal, true); assert.deepEqual(await pixel(), single);
            await configure('inner', 'manual'); await open('inner');
            assert.deepEqual([(await read('inner')).modal, (await read('inner')).background, (await read('inner')).opacity], [false, 'rgba(0, 0, 0, 0)', 1]);
            assert.deepEqual(await pixel(), single, 'An overlay above a modal adds no veil'); await close('inner'); await close('outer');
            for (const presentation of ['', 'manual', 'auto']) {
                await configure('outer', presentation); await open('outer'); assert.equal((await read('outer')).modal, false);
                assert.deepEqual(await pixel(), baseline, 'Nonmodal presentation adds no veil: ' + presentation); await close('outer');
            }
            await configure('outer', null); await configure('inner', null);
            await page.emulateMedia({ colorScheme: 'dark' });
            await page.evaluate(() => {
                const { stage, inner } = window.backdropFixture; stage.style.removeProperty('--cem-scrim-color'); stage.style.removeProperty('--cem-scrim-opacity');
                stage.className = 'cem-theme-native'; inner.className = 'cem-theme-light';
            });
            await open('outer'); await open('inner'); assert.equal((await read('inner')).background, 'rgb(0, 0, 0)', 'A nested branded theme resets native system ink');
            await close('inner'); await close('outer');
        }
        await page.evaluate(async () => {
            window.backdropFixture.stage.remove();
            const stage = document.createElement('section'); stage.style.cssText = 'position:fixed;inset:0;background:white;z-index:2147483647';
            stage.innerHTML = '<cem-datepicker style="position:fixed;inset-inline-start:50%;inset-block-start:20%"><input slot="input" type="text" aria-label="Modal date"><button slot="toggle" type="button">Choose date</button></cem-datepicker>';
            document.body.append(stage); const outer = stage.querySelector('cem-datepicker'); window.backdropFixture = { stage, outer };
            await window.cemPlaygroundRuntime.whenRenderSettled(outer);
        });
        for (const theme of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native', 'projector', 'native-dark']) for (const forced of [false, true]) {
            await page.emulateMedia({ forcedColors: forced ? 'active' : 'none', colorScheme: theme.includes('dark') ? 'dark' : 'light' });
            await page.evaluate(theme => { window.backdropFixture.stage.className = 'cem-theme-' + (theme === 'native-dark' ? 'native' : theme); }, theme);
            const baseline = await pixel(); await open('outer'); const paint = await read('outer');
            assert(paint.modal); assert.equal(paint.opacity, 0.32); assert.equal(paint.background, paint.expectedColor);
            blend(baseline, rgb(paint.background), paint.opacity, await pixel(), 'Datepicker modal scrim/' + theme + '/forced=' + forced);
            await close('outer'); assert.deepEqual(await pixel(), baseline);
        }
        await page.emulateMedia({ forcedColors: 'none' });
        await page.evaluate(() => {
            const stage = window.backdropFixture.stage; stage.style.setProperty('--cem-scrim-color', 'rgb(20, 40, 60)'); stage.style.setProperty('--cem-scrim-opacity', '0.5');
        });
        await open('outer'); const datePaint = await read('outer'); assert.equal(datePaint.background, 'rgb(20, 40, 60)'); assert.equal(datePaint.opacity, 0.5); await close('outer');
        assert.deepEqual(errors, []);
        console.log('Modal backdrop paint passed in ' + browser.browserType().name() + ' ' + browser.version() + ': dialog/shell/datepicker, six themes, system schemes, forced colors, inherited overrides, nested modals, overlays and nonmodal pixels (' + baseUrl + ').');
    } finally { await context.close(); }
}
