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
        const expectedCounts = [5, 4, 3, 3, 5, 1, 2, 9, 20];
        await page.waitForFunction(counts => {
            const cards = [...document.querySelectorAll('cem-demo-element')];
            return cards.length === counts.length && cards.every((card, index) =>
                card.getAttribute('data-state') === 'ready'
                && card.querySelectorAll('[slot=demo] button').length === counts[index]);
        }, expectedCounts).catch(async error => {
            const cards = await page.locator('cem-demo-element').evaluateAll(nodes => nodes.map(node => ({
                legend: node.getAttribute('legend'), state: node.getAttribute('data-state'),
                buttons: node.querySelectorAll('[slot=demo] button').length,
            })));
            throw new Error(`${packagePath}: ${error.message}\n${JSON.stringify({ errors, cards })}`);
        });
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
        await verifyActionStates(page);
        await verifyActionVariations(page);
        assert.deepEqual(errors, [], `${packagePath}: action demo local module/WASM loading`);
    } finally {
        await page.close();
    }
}

async function verifyActionStates(page) {
    const card = page.locator('cem-demo-element[legend="Interaction states"] [slot=demo]');
    const enabled = card.locator('button').first();
    const disabled = card.locator('button').nth(1);
    const presenceDisabled = card.locator('button').nth(2);
    const expected = await enabled.evaluate(button => {
        const probe = document.createElement('span');
        const intent = button.parentElement.getAttribute('variant') ?? 'primary';
        button.parentElement.append(probe);
        const result = Object.fromEntries(['default', 'hover', 'active', 'disabled'].map(state => {
            probe.style.backgroundColor = `var(--cem-action-${intent}-${state}-background)`;
            probe.style.color = `var(--cem-action-${intent}-${state}-text)`;
            const style = getComputedStyle(probe);
            return [state, { background: style.backgroundColor, color: style.color }];
        }));
        probe.remove();
        return result;
    });
    const read = locator => locator.evaluate(button => {
        const style = getComputedStyle(button);
        const rect = button.getBoundingClientRect();
        return {
            paint: { background: style.backgroundColor, color: style.color },
            geometry: { width: rect.width, height: rect.height, radius: style.borderRadius },
            hover: button.matches(':hover'), active: button.matches(':active'),
            focus: button.matches(':focus-visible'), shadow: style.boxShadow,
            outline: style.outlineStyle, outlineWidth: parseFloat(style.outlineWidth),
            outlineColor: style.outlineColor,
        };
    });
    assert.notDeepEqual(expected.default, expected.hover, 'theme hover differs from default');
    assert.notDeepEqual(expected.hover, expected.active, 'theme active differs from hover');
    await page.mouse.move(0, 0);
    const initial = await read(enabled);
    assert.deepEqual(initial.paint, expected.default, 'default state uses theme colors');
    await enabled.evaluate(button => {
        button.dataset.activationCount = '0';
        button.addEventListener('click', () => {
            button.dataset.activationCount = String(Number(button.dataset.activationCount) + 1);
        });
    });
    await enabled.hover();
    assert.equal((await read(enabled)).hover, true);
    assert.deepEqual((await read(enabled)).paint, expected.hover, 'native hover uses theme colors');
    await page.mouse.down();
    try {
        const held = await read(enabled);
        assert.equal(held.active, true);
        assert.deepEqual(held.paint, expected.active, 'held pointer uses active colors');
        assert.deepEqual(held.geometry, initial.geometry, 'pointer press preserves geometry');
        assert.equal(await enabled.getAttribute('data-activation-count'), '0');
    } finally {
        await page.mouse.up();
    }
    assert.equal(await enabled.getAttribute('data-activation-count'), '1');
    await page.mouse.move(0, 0);
    await enabled.focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    const focused = await read(enabled);
    assert.equal(focused.focus, true, 'keyboard navigation exposes focus-visible');
    await verifyActionZebra(enabled);
    assert.deepEqual(focused.geometry, initial.geometry, 'focus preserves geometry');
    await page.keyboard.down('Space');
    try {
        assert.equal((await read(enabled)).active, true);
        assert.deepEqual((await read(enabled)).paint, expected.active, 'held Space uses active colors');
        assert.equal(await enabled.getAttribute('data-activation-count'), '1');
    } finally {
        await page.keyboard.up('Space');
    }
    assert.equal(await enabled.getAttribute('data-activation-count'), '2');
    for (const button of [disabled, presenceDisabled]) {
        assert.equal(await button.evaluate(node => node.disabled), true, 'disabled is a native presence binding');
        await button.hover();
        const state = await read(button);
        assert.deepEqual(state.paint, expected.disabled, 'disabled hover keeps disabled colors');
        assert.equal(await button.evaluate(node => {
            let clicks = 0;
            node.addEventListener('click', () => clicks++, { once: true });
            node.click();
            return clicks;
        }), 0, 'native disabled button suppresses activation');
    }
    await page.emulateMedia({ forcedColors: 'active' });
    await enabled.focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    const forced = await read(enabled);
    assert.equal(forced.focus, true);
    assert.equal(forced.outline, 'solid');
    assert(forced.outlineWidth > 0, 'forced colors retains a visible focus outline');
    assert.notEqual(forced.outlineColor, 'rgba(0, 0, 0, 0)');
    await page.emulateMedia({ forcedColors: 'none' });
}

async function verifyActionZebra(button) {
    const ring = await button.evaluate(node => {
        const probe = document.createElement('span');
        node.append(probe);
        const stripes = [1, 2, 3].map(index => {
            probe.style.boxShadow = `0 0 0 calc(${index} * var(--cem-zebra-strip-size)) var(--cem-zebra-color-${index})`;
            probe.style.color = `var(--cem-zebra-color-${index})`;
            return { shadow: getComputedStyle(probe).boxShadow, color: getComputedStyle(probe).color };
        });
        const actual = getComputedStyle(node).boxShadow;
        probe.remove();
        return { actual, stripes };
    });
    assert.deepEqual(ring.actual, ring.stripes.map(stripe => stripe.shadow).join(', '),
        'focus ring resolves each stripe on the focused control');
    assert.notEqual(ring.stripes[0].color, ring.stripes[1].color,
        'focus stripe stays distinct from the inactive target stripe');
}

async function verifyActionVariations(page) {
    const intents = ['primary', 'explicit', 'destructive', 'contextual', 'alternate', 'contextual', 'explicit', 'alternate'];
    const samples = await page.locator('cem-demo-element:not([legend="Action variations matrix"])').evaluateAll(cards =>
        cards.map(card => [...new Set([...card.querySelectorAll('[slot=demo] cem-action')]
            .map(host => host.getAttribute('variant')))]));
    assert.deepEqual(samples, intents.map(intent => [intent]), 'each sample uses one consistent action intent');
    const matrix = page.locator('cem-demo-element[legend="Action variations matrix"] [slot=demo]');
    for (const intent of ['primary', 'explicit', 'contextual', 'alternate', 'destructive']) {
        const buttons = matrix.locator(`cem-action[variant="${intent}"] button`);
        assert.equal(await buttons.count(), 4);
        const geometry = await buttons.evaluateAll(nodes => nodes.map(node => ({
            radius: parseFloat(getComputedStyle(node).borderRadius),
            height: node.getBoundingClientRect().height,
        })));
        assert.equal(geometry[0].radius, 0, `${intent} sharp column`);
        assert(geometry[1].radius > 0, `${intent} smooth column`);
        assert.equal(geometry[2].radius * 2, geometry[2].height, `${intent} round column`);
        assert.equal(geometry[3].radius, geometry[1].radius, `${intent} disabled retains default bend`);
        const enabled = buttons.first();
        const assertPaint = async (button, state) => {
            const result = await button.evaluate((node, { intent, state }) => {
                const probe = document.createElement('span');
                node.parentElement.append(probe);
                probe.style.backgroundColor = `var(--cem-action-${intent}-${state}-background)`;
                probe.style.color = `var(--cem-action-${intent}-${state}-text)`;
                const actual = getComputedStyle(node);
                const expected = getComputedStyle(probe);
                const result = { actual: [actual.backgroundColor, actual.color], expected: [expected.backgroundColor, expected.color] };
                probe.remove();
                return result;
            }, { intent, state });
            assert.deepEqual(result.actual, result.expected, `${intent} ${state} matches theme tokens`);
        };
        await page.mouse.move(0, 0);
        await assertPaint(enabled, 'default');
        await enabled.hover();
        await assertPaint(enabled, 'hover');
        await page.mouse.down();
        try { await assertPaint(enabled, 'active'); }
        finally { await page.mouse.up(); }
        await buttons.last().hover();
        assert.equal(await buttons.last().evaluate(node => node.disabled), true);
        await assertPaint(buttons.last(), 'disabled');
    }
    const focus = matrix.locator('button').first();
    await focus.focus();
    await page.keyboard.press('Tab');
    await page.keyboard.press('Shift+Tab');
    const originalClass = await page.locator('body').getAttribute('class');
    for (const mode of ['light', 'dark', 'contrast-light', 'contrast-dark', 'native']) {
        await page.locator('body').evaluate((body, mode) => body.className = `cem-theme-${mode}`, mode);
        assert.equal(await focus.evaluate(node => node.matches(':focus-visible')), true);
        await verifyActionZebra(focus);
    }
    await page.locator('body').evaluate((body, value) => { body.className = value ?? ''; }, originalClass);
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
