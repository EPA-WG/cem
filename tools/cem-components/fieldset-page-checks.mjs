import assert from 'node:assert/strict';

// Integration checks shared by repository and isolated-package gallery previews.
export async function verifyFieldsetPage(page, path) {
    let group;
    if (path.endsWith('/cem-fieldset.html')) {
        const host = page.locator('cem-fieldset:has(cem-text-field[name=email])');
        group = host.locator(':scope > fieldset');
        const input = host.getByRole('textbox', { name: 'Email', exact: true });
        await input.fill('edited@example.test');
        const retained = await input.elementHandle();
        await page.getByRole('textbox', { name: 'Group label', exact: true }).fill('Delivery preferences');
        await page.getByRole('group', { name: 'Delivery preferences', exact: true }).waitFor();
        assert.equal(await input.evaluate((node, previous) => node === previous, retained), true, `${path}: label update replaced an edited field`);
        assert.equal(await input.inputValue(), 'edited@example.test');
        const disabled = page.getByRole('group', { name: 'Disabled presence', exact: true });
        await disabled.getByRole('radio', { name: 'present', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('cem-text-field[name=email] input')?.matches(':disabled'));
        assert.equal(await group.getAttribute('disabled') !== null, true, `${path}: disabled presence`);
        await disabled.getByRole('radio', { name: 'absent', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('cem-text-field[name=email] input')?.matches(':enabled'));
        assert.equal(await input.inputValue(), 'edited@example.test');
        await page.getByRole('group', { name: 'External label reference', exact: true }).getByRole('radio', { name: 'present', exact: true }).check();
        await page.getByRole('group', { name: 'Live field group', exact: true }).waitFor();
        await page.getByRole('group', { name: 'External description reference', exact: true }).getByRole('radio', { name: 'present', exact: true }).check();
        await page.waitForFunction(() => document.querySelector('cem-fieldset[aria-describedby="fieldset-preview-help"] > fieldset')?.getAttribute('aria-describedby') === 'fieldset-preview-help');
        assert.equal(await group.getAttribute('aria-labelledby'), 'fieldset-preview-label');
    } else {
        const demo = page.locator('cem-demo-element[legend="Native form ownership"]');
        const form = demo.locator('form');
        group = form.getByRole('group', { name: 'Contact details', exact: true });
        await group.getByRole('combobox', { name: 'Channel', exact: true }).waitFor();
        await group.getByRole('textbox', { name: 'Email', exact: true }).fill('edited@example.test');
        assert.deepEqual(await form.evaluate(node => [...new FormData(node)]), [['email', 'edited@example.test'], ['channel', 'email']]);
        await form.getByRole('button', { name: 'Reset contact', exact: true }).click();
        await page.waitForFunction(() => document.querySelector('cem-demo-element[legend="Native form ownership"] input')?.value === 'reader@example.test');
        for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
            const row = page.locator(`[data-gallery-theme="${mode}"]`);
            await row.getByRole('textbox', { name: 'Email', exact: true }).last().waitFor();
            const paint = await row.evaluate(node => {
                const groups = [...node.querySelectorAll('cem-fieldset > fieldset')];
                const probe = document.createElement('span');
                probe.style.cssText = 'background:var(--cem-palette-comfort);color:var(--cem-palette-comfort-text)';
                node.append(probe);
                const expected = getComputedStyle(probe);
                const result = { count: groups.length, enabled: groups.map(group => !group.disabled),
                    actual: groups.map(group => [getComputedStyle(group).backgroundColor, getComputedStyle(group).color]),
                    expected: [expected.backgroundColor, expected.color] };
                probe.remove(); return result;
            });
            assert.equal(paint.count, 2, `${path}: ${mode} enabled/disabled matrix`);
            assert.deepEqual(paint.enabled, [true, false]);
            assert.deepEqual(paint.actual, [paint.expected, paint.expected], `${path}: ${mode} inherited group palette`);
        }
        const falseGroup = page.locator('cem-demo-element[legend="Presence and nested groups"] cem-fieldset[disabled="false"]').first();
        assert.equal(await falseGroup.getByRole('textbox', { name: 'Retained value', exact: true }).isDisabled(), true, `${path}: false-valued disabled presence`);
    }
    const references = await page.locator('cem-fieldset > fieldset').evaluateAll(groups => groups.map(group => group.getAttribute('aria-describedby')).filter(id => id && id !== 'fieldset-preview-help'));
    assert.equal(new Set(references).size, references.length, `${path}: group help references must be unique`);
    await group.evaluate(node => {
        node.parentElement.style.setProperty('--cem-inset-container', '23px');
        node.parentElement.style.setProperty('--cem-gap-group', '27px');
    });
    assert.deepEqual(await group.evaluate(node => [getComputedStyle(node).paddingInlineStart, getComputedStyle(node.querySelector('[part=body]')).rowGap]), ['23px', '27px']);
    await page.setViewportSize({ width: 360, height: 900 });
    assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true, `${path}: narrow page overflow`);
    for (const direction of ['ltr', 'rtl']) {
        await group.evaluate((node, dir) => { node.parentElement.dir = dir; }, direction);
        const geometry = await group.evaluate(node => ({
            width: node.getBoundingClientRect().width, available: node.parentElement.getBoundingClientRect().width,
            bodyWidth: node.querySelector('[part=body]').clientWidth, bodyScroll: node.querySelector('[part=body]').scrollWidth,
        }));
        assert.ok(geometry.width <= geometry.available + 1 && geometry.bodyScroll <= geometry.bodyWidth + 1, `${path}: narrow ${direction} group overflow: ${JSON.stringify(geometry)}`);
    }
    await page.emulateMedia({ forcedColors: 'active' });
    const forced = await group.evaluate(node => {
        const probe = document.createElement('span');
        probe.style.cssText = 'background:Canvas;color:CanvasText'; document.body.append(probe);
        const actual = getComputedStyle(node), expected = getComputedStyle(probe);
        const result = { actual: [actual.backgroundColor, actual.color, actual.borderTopColor], expected: [expected.backgroundColor, expected.color, expected.color] };
        probe.remove(); return result;
    });
    assert.deepEqual(forced.actual, forced.expected, `${path}: forced-color group boundary and content`);
}
