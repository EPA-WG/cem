import assert from 'node:assert/strict';

// The same examples must work from workspace source, dist, and an installed archive.
export async function verifyLoaderExamples(browser, origin, packagePath) {
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const errors = [];
    page.on('pageerror', (error) => errors.push(error.message));
    // Sprite downloads are decorative and must not make this gate depend on a CDN.
    await page.route('https://unpkg.com/**', (route) => route.fulfill({
        contentType: 'image/svg+xml', body: '<svg xmlns="http://www.w3.org/2000/svg"/>',
    }));
    try {
        await page.goto(`${origin}${packagePath}/demo/http-request.html`);
        await page.waitForFunction(() => document.querySelectorAll('.result-buttons button[aria-label]').length === 6);
        await verifySourcePreviews(page, origin, packagePath,
            ['http-data.json', 'http-data-compact.json', 'http-data-invalid.json', 'http-pokemon.json']);
        const sample = page.locator('.loader-example').first();
        await sample.getByRole('button', { name: 'GET', exact: true }).click();
        await page.waitForFunction(() => [...document.querySelector('.loader-example').querySelectorAll('li')].map(n => n.textContent.trim()).join('|') === 'alpha: ready|beta: loaded');
        await sample.getByRole('button', { name: 'Compact records', exact: true }).click();
        await sample.locator('input').waitFor();
        await page.waitForFunction(() => document.querySelector('.loader-example input')?.value === './http-data-compact.json');
        await sample.getByRole('button', { name: 'GET', exact: true }).click();
        await page.waitForFunction(() => [...document.querySelectorAll('.loader-example li')].map(n => n.textContent.trim()).join('|') === 'solo: compact');
        assert(await page.locator('template[data-cem-island]').count() > 0, 'native runtime owns the data islands');
        const snapshots = await page.locator('template[data-cem-island]').evaluateAll((islands) => islands.map(n => n.innerHTML).join(''));
        assert(!snapshots.includes('bulbasaur'), 'serialized islands must not contain imported response documents');
        await page.goto(`${origin}${packagePath}/demo/npm-versions-demo.html`);
        await page.waitForFunction(() => document.querySelectorAll('select').length === 5
            && document.querySelector('select')?.value === '0.1.0');
        await verifySourcePreviews(page, origin, packagePath, ['npm-versions.json']);
        assert.equal(await page.locator('select').nth(1).inputValue(), '0.0.22');
        await page.locator('select').nth(2).selectOption('0.0.25');
        await page.waitForFunction(() => document.querySelector('cem-npm-version-propagated')?.getAttribute('value') === '0.0.25');
        assert.deepEqual(errors, [], `${packagePath}: migrated loader examples`);
        await verifyActionDemo(browser, origin, packagePath);
    } finally {
        await page.close();
    }
}

export async function verifyActionDemo(browser, origin, packagePath) {
    const page = await browser.newPage();
    const errors = [];
    page.on('pageerror', error => errors.push(error.message));
    page.on('response', response => {
        if (response.url().startsWith(origin) && response.status() >= 400) {
            errors.push(`${response.status()} ${response.url()}`);
        }
    });
    page.on('requestfailed', request => {
        if (request.url().startsWith(origin)) errors.push(`${request.url()}: ${request.failure()?.errorText}`);
    });
    // Font services are presentation-only; module and WASM loads must be local.
    await page.route(/https:\/\/(fonts\.googleapis\.com|fonts\.gstatic\.com|use\.fontawesome\.com)\//,
        route => route.fulfill({ contentType: 'text/css', body: '' }));
    try {
        await page.goto(`${origin}${packagePath}/material/components/action.html`);
        const expectedCounts = [5, 4, 3, 5, 1, 2, 9];
        await page.waitForFunction(counts => {
            const cards = [...document.querySelectorAll('cem-demo-element')];
            return cards.length === counts.length && cards.every((card, index) =>
                card.getAttribute('data-state') === 'ready'
                && card.querySelectorAll('[slot=demo] button').length === counts[index]);
        }, expectedCounts);
        assert(await page.evaluate(() => Boolean(customElements.get('cem-element'))));
        assert.equal(await page.locator('custom-element, html-demo-element').count(), 0);
        const typical = page.locator('cem-demo-element[legend="Typical use"] [slot=demo]');
        await typical.getByRole('button', { name: 'with icon search', exact: true }).waitFor();
        assert.equal(await typical.getByRole('button', { name: 'text in attribute', exact: true }).count(), 1);
        assert.equal(await typical.getByRole('button', { name: 'with icon search', exact: true }).count(), 1);
        assert(!/\{\$?(text|image)\}/.test(await page.locator('main').innerText()), 'legacy expressions are evaluated');
        const bend = page.locator('cem-demo-element[legend="Bend"] [slot=demo]');
        await bend.getByRole('button', { name: 'square', exact: true }).click();
        await page.waitForFunction(() => document.querySelector(
            'cem-demo-element[legend="Bend"] [slot=demo] cem-action',
        )?.classList.contains('cem-bend-sharp'));
        await verifyActionCurvature(page);
        assert.deepEqual(errors, [], `${packagePath}: action demo local module/WASM loading`);
    } finally {
        await page.close();
    }
}

async function verifyActionCurvature(page) {
    await page.waitForFunction(() => {
        const button = document.querySelector('cem-demo-element[legend="Typical use"] [slot=demo] button');
        return button && getComputedStyle(button).getPropertyValue('--cem-bend-smooth').trim() !== '';
    });
    const readGeometry = () => page.evaluate(() => {
        const probe = document.createElement('span');
        document.documentElement.append(probe);
        const tokenRadius = token => {
            probe.style.borderRadius = `var(${token})`;
            return getComputedStyle(probe).borderRadius;
        };
        const tokens = Object.fromEntries(['sharp', 'smooth', 'round'].map(name =>
            [name, tokenRadius(`--cem-bend-${name}`)]));
        tokens.default = tokenRadius('--cem-action-border-radius');
        probe.remove();
        const geometry = button => ({
            radius: getComputedStyle(button).borderRadius,
            height: button.getBoundingClientRect().height,
        });
        const typical = document.querySelector('cem-demo-element[legend="Typical use"] [slot=demo]');
        return {
            tokens,
            default: geometry(typical.querySelector('cem-action:not([class]) button')),
            sharp: geometry(typical.querySelector('cem-action.cem-bend-sharp button')),
            round: geometry(typical.querySelector('cem-action.cem-bend-round button')),
            multiline: geometry([...typical.querySelectorAll('cem-action.cem-bend-round button')].at(-1)),
            live: geometry(document.querySelector('cem-demo-element[legend="Bend"] [slot=demo] cem-action button')),
        };
    });
    let current = await readGeometry();
    assert.equal(current.default.radius, current.tokens.default, 'default action consumes the theme action binding');
    assert.equal(current.sharp.radius, current.tokens.sharp, 'sharp action has the theme sharp corners');
    assert.equal(current.round.radius, current.tokens.round, 'round action consumes the theme round bend');
    assert.equal(parseFloat(current.round.radius) * 2, current.round.height, 'single-line round ends are semicircles');
    assert.equal(current.multiline.height, current.round.height * 2, 'multiline shape height follows its local token override');
    assert.equal(parseFloat(current.multiline.radius) * 2, current.multiline.height, 'multiline round ends are semicircles');
    assert.notEqual(current.tokens.smooth, current.tokens.sharp, 'the generated theme supplies distinct geometry');
    const bend = page.locator('cem-demo-element[legend="Bend"] [slot=demo]');
    for (const [label, mode] of [['square', 'sharp'], ['smooth', 'smooth'], ['round', 'round']]) {
        await bend.getByRole('button', { name: label, exact: true }).click();
        await page.waitForFunction(mode => document.querySelector(
            'cem-demo-element[legend="Bend"] [slot=demo] cem-action',
        )?.classList.contains(`cem-bend-${mode}`), mode);
        current = await readGeometry();
        assert.equal(current.live.radius, current.tokens[mode], `${mode} changes the rendered curvature`);
    }
    for (const mode of ['compact', 'forgiving']) {
        await page.evaluate(mode => document.documentElement.setAttribute('data-cem-coupling', mode), mode);
        current = await readGeometry();
        assert.equal(current.round.radius, current.tokens.round, `${mode} round bend follows theme control geometry`);
        assert.equal(parseFloat(current.round.radius) * 2, current.round.height, `${mode} round ends remain semicircles`);
        assert.equal(current.multiline.height, current.round.height * 2, `${mode} multiline shape height follows control sizing`);
        assert.equal(parseFloat(current.multiline.radius) * 2, current.multiline.height, `${mode} multiline round ends remain semicircles`);
    }
    await page.evaluate(() => document.documentElement.removeAttribute('data-cem-coupling'));
}

async function verifySourcePreviews(page, origin, packagePath, files) {
    for (const file of files) {
        const card = page.locator(`cem-demo-element[src="./${file}"]`);
        await page.waitForFunction(file => document.querySelector(`cem-demo-element[src="./${file}"]`)
            ?.getAttribute('data-state') === 'ready', file);
        const response = await page.request.get(`${origin}${packagePath}/demo/${file}`);
        assert(response.ok(), `${file} is included in the package`);
        assert.equal(await card.locator('[slot=text] code').textContent(), await response.text());
        assert.equal(await card.locator('[slot=demo]').textContent(), '');
        if (file !== 'http-data-invalid.json') {
            assert(await card.locator('[slot=text] code i').count() > 0, `${file} has JSON highlighting`);
        }
    }
}
