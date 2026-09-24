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
    } finally {
        await page.close();
    }
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
