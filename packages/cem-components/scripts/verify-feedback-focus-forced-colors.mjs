#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = join(packageRoot, '..', '..');
const componentCss = await readFile(join(packageRoot, 'src', 'styles.css'), 'utf8');
const themeCss = await readFile(
    join(repoRoot, 'packages', 'cem-theme', 'dist', 'lib', 'css', 'cem-combined.css'),
    'utf8',
);
const browser = await chromium.launch({ headless: true });

try {
    const context = await browser.newContext({ forcedColors: 'active', javaScriptEnabled: true });
    const page = await context.newPage();
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Start</button>
            <cem-dialog id="static-host">
                <div id="static-owner" class="cem-dialog" role="dialog" aria-modal="true">
                    <button id="static-child" type="button">Static dialog action</button>
                </div>
            </cem-dialog>
            <button id="dialog-opener" type="button">Open dialog</button>
            <cem-dialog id="dialog-host" transient expanded>
                <dialog id="dialog-owner" class="cem-dialog" aria-label="Fallback dialog">
                    <p>Fallback content</p>
                    <button id="dialog-disabled" type="button" autofocus disabled>Unavailable</button>
                </dialog>
            </cem-dialog>
            <button id="shell-opener" type="button">Open dialog shell</button>
            <cem-dialog-shell id="shell-host" transient expanded>
                <dialog id="shell-owner" class="cem-dialog-shell" aria-label="Fallback dialog shell">
                    <p>Fallback shell content</p>
                    <button id="shell-disabled" type="button" autofocus disabled>Unavailable</button>
                </dialog>
            </cem-dialog-shell>
            <button id="authored-opener" type="button">Open authored dialog</button>
            <cem-dialog id="authored-host" transient expanded>
                <dialog id="authored-owner" class="cem-dialog" aria-label="Authored target dialog">
                    <button id="authored-target" type="button" autofocus>Continue</button>
                </dialog>
            </cem-dialog>
            <button id="sheet-opener" type="button">Show sheet</button>
            <cem-sheet id="sheet-host" transient expanded>
                <aside id="sheet-owner" class="cem-sheet" role="region" aria-label="Details">
                    <label>Reference <input id="sheet-target" type="text" value="CEM-42"></label>
                </aside>
            </cem-sheet>
            <button id="focus-end" type="button">End</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__feedbackMutationEvents = [];
        window.__feedbackStateEvents = [];
        window.__feedbackObserver = new MutationObserver((records) => {
            window.__feedbackMutationEvents.push(
                ...records.map((record) => `${record.target.id || record.target.localName}:${record.attributeName}`),
            );
        });
        window.__feedbackObserver.observe(document.querySelector('main'), {
            attributeFilter: ['aria-expanded', 'aria-hidden', 'open', 'tabindex', 'value'],
            attributes: true,
            subtree: true,
        });
        for (const eventName of ['cancel', 'change', 'click', 'close', 'dismiss', 'input']) {
            document.addEventListener(eventName, () => window.__feedbackStateEvents.push(eventName));
        }
    });

    const initial = await page.evaluate(captureFeedbackForcedColorState);
    assert(initial.forcedColors, 'forced-colors media query did not activate');

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    await page.evaluate(nextFrame);
    const staticFocused = await page.evaluate(captureFeedbackForcedColorState);
    assert(staticFocused.activeElement === 'static-child', 'static authored child did not receive keyboard focus');
    assert(staticFocused.elements.staticChild.focusVisible, 'static authored child did not retain native focus-visible');
    assert(!staticFocused.elements.staticOwner.focusVisible, 'static dialog wrapper acquired focus-visible');
    assert(!staticFocused.elements.staticHost.focusVisible, 'static dialog host acquired focus-visible');
    assert(
        equalOutline(staticFocused.elements.staticOwner, initial.elements.staticOwner),
        'static dialog wrapper acquired component focus paint',
    );
    assert(
        equalOutline(staticFocused.elements.staticHost, initial.elements.staticHost),
        'static dialog host acquired component focus paint',
    );

    for (const fixture of [
        { disabled: 'dialog-disabled', host: 'dialog-host', opener: 'dialog-opener', owner: 'dialog-owner' },
        { disabled: 'shell-disabled', host: 'shell-host', opener: 'shell-opener', owner: 'shell-owner' },
    ]) {
        await page.locator(`#${fixture.opener}`).focus();
        await page.keyboard.press('ArrowDown');
        await page.locator(`#${fixture.owner}`).evaluate((element) => element.showModal());
        await page.evaluate(nextFrame);
        await page.evaluate(resetFeedbackObservation);

        const focused = await page.evaluate(captureFeedbackForcedColorState);
        const owner = focused.elements[camelCase(fixture.owner)];
        const host = focused.elements[camelCase(fixture.host)];
        assert(focused.activeElement === fixture.owner, `${fixture.owner} did not receive native fallback focus`);
        assert(owner.modal, `${fixture.owner} did not retain native modal state`);
        assert(owner.focusVisible, `${fixture.owner} did not match :focus-visible`);
        assert(
            focused.activeElement !== fixture.disabled,
            `${fixture.owner} did not skip its disabled autofocus descendant`,
        );
        assert(owner.tabIndexAttribute === null, `${fixture.owner} acquired authored tabindex`);
        assert(owner.outlineColor === focused.system.canvasText, `${fixture.owner} outline did not map to CanvasText`);
        assert(owner.outlineStyle === 'solid', `${fixture.owner} focus outline was not solid`);
        assert(
            owner.outlineWidth === focused.tokens.focusWidth,
            `${fixture.owner} focus width did not resolve from --cem-stroke-focus`,
        );
        assert(
            owner.outlineOffset === focused.tokens.focusOffset,
            `${fixture.owner} focus offset did not resolve from --cem-stroke-indicator-offset`,
        );
        assert(owner.forcedColorAdjust === 'auto', `${fixture.owner} disabled automatic forced-color adjustment`);
        assert(!host.focusVisible, `${fixture.host} acquired focus-visible`);
        assert(
            equalOutline(host, initial.elements[camelCase(fixture.host)]),
            `${fixture.host} acquired component focus paint`,
        );

        await page.locator(`#${fixture.owner}`).evaluate((element) => element.blur());
        await page.evaluate(nextFrame);
        const blurred = await page.evaluate(captureFeedbackForcedColorState);
        const blurredOwner = blurred.elements[camelCase(fixture.owner)];
        assert(!blurredOwner.focusVisible, `${fixture.owner} retained :focus-visible after blur`);
        assert(
            !equalOutline(blurredOwner, owner),
            `${fixture.owner} retained the component focus outline after blur`,
        );

        await page.keyboard.press('ArrowDown');
        await page.locator(`#${fixture.owner}`).focus();
        await page.evaluate(nextFrame);
        const refocused = await page.evaluate(captureFeedbackForcedColorState);
        const refocusedOwner = refocused.elements[camelCase(fixture.owner)];
        assert(refocused.activeElement === fixture.owner, `${fixture.owner} did not regain focus`);
        assert(refocusedOwner.focusVisible, `${fixture.owner} did not regain :focus-visible`);
        assert(equalOutline(refocusedOwner, owner), `${fixture.owner} did not restore the forced-colors outline`);
        assert(equalRect(refocusedOwner.rect, owner.rect), `${fixture.owner} focus changed owner geometry`);
        assert(
            equalRect(refocused.elements[camelCase(fixture.host)].rect, host.rect),
            `${fixture.owner} focus changed host geometry`,
        );
        assert(refocusedOwner.html === owner.html, `${fixture.owner} focus changed owner DOM or ARIA`);
        assert(
            refocused.elements[camelCase(fixture.host)].html === host.html,
            `${fixture.owner} focus changed host DOM or ARIA`,
        );
        assert(refocused.mutationEvents.length === 0, `${fixture.owner} focus mutated component state`);
        assert(refocused.stateEvents.length === 0, `${fixture.owner} focus dispatched a component state event`);

        await page.evaluate(() => window.__feedbackObserver.disconnect());
        await page.locator(`#${fixture.owner}`).evaluate((element) => element.close());
        await page.evaluate(nextFrame);
        const closed = await page.evaluate(captureFeedbackForcedColorState);
        assert(closed.activeElement === fixture.opener, `${fixture.owner} did not restore its opener on close`);
        assert(!closed.elements[camelCase(fixture.owner)].focusVisible, `${fixture.owner} retained focus after close`);
        await page.evaluate(restartFeedbackObservation);
    }

    await page.locator('#authored-opener').focus();
    await page.keyboard.press('ArrowDown');
    await page.locator('#authored-owner').evaluate((element) => element.showModal());
    await page.evaluate(nextFrame);
    const authored = await page.evaluate(captureFeedbackForcedColorState);
    assert(authored.activeElement === 'authored-target', 'eligible authored autofocus target did not receive focus');
    assert(authored.elements.authoredTarget.focusVisible, 'authored target did not retain its own focus-visible state');
    assert(!authored.elements.authoredOwner.focusVisible, 'authored-target dialog owner acquired focus-visible');
    assert(!authored.elements.authoredHost.focusVisible, 'authored-target dialog host acquired focus-visible');
    assert(
        equalOutline(authored.elements.authoredOwner, initial.elements.authoredOwner),
        'authored-target dialog owner acquired component focus paint',
    );
    await page.evaluate(() => window.__feedbackObserver.disconnect());
    await page.locator('#authored-owner').evaluate((element) => element.close());
    await page.evaluate(nextFrame);
    assert(
        await page.locator('#authored-opener').evaluate((element) => document.activeElement === element),
        'authored-target dialog did not restore its opener',
    );
    await page.evaluate(restartFeedbackObservation);

    await page.locator('#sheet-opener').focus();
    await page.evaluate(nextFrame);
    const sheetShown = await page.evaluate(captureFeedbackForcedColorState);
    assert(sheetShown.activeElement === 'sheet-opener', 'transient sheet showing state moved focus');
    await page.keyboard.press('ArrowDown');
    await page.locator('#sheet-target').focus();
    await page.evaluate(nextFrame);
    const sheetFocused = await page.evaluate(captureFeedbackForcedColorState);
    assert(sheetFocused.activeElement === 'sheet-target', 'authored sheet control did not receive focus');
    assert(sheetFocused.elements.sheetTarget.focusVisible, 'authored sheet control did not own focus-visible');
    assert(!sheetFocused.elements.sheetOwner.focusVisible, 'sheet region acquired focus-visible');
    assert(!sheetFocused.elements.sheetHost.focusVisible, 'sheet host acquired focus-visible');
    assert(
        equalOutline(sheetFocused.elements.sheetOwner, sheetShown.elements.sheetOwner),
        'sheet region acquired component focus paint',
    );
    assert(
        equalOutline(sheetFocused.elements.sheetHost, sheetShown.elements.sheetHost),
        'sheet host acquired component focus paint',
    );
    assert(
        equalRect(sheetFocused.elements.sheetOwner.rect, sheetShown.elements.sheetOwner.rect),
        'authored descendant focus changed sheet geometry',
    );
    assert(sheetFocused.elements.sheetOwner.html === sheetShown.elements.sheetOwner.html, 'sheet focus changed DOM or ARIA');
    assert(sheetFocused.mutationEvents.length === 0, 'sheet focus mutated component state');
    assert(sheetFocused.stateEvents.length === 0, 'sheet focus dispatched a component state event');

    await context.close();
    console.log('cem-components feedback focus-visible forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureFeedbackForcedColorState() {
    const toCamelCase = (id) => id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
    const readElement = (id) => {
        const element = document.getElementById(id);
        const styles = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
            focusVisible: element.matches(':focus-visible'),
            forcedColorAdjust: styles.forcedColorAdjust,
            html: element.outerHTML,
            modal: element.matches(':modal'),
            outlineColor: styles.outlineColor,
            outlineOffset: styles.outlineOffset,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
            rect: [rect.x, rect.y, rect.width, rect.height],
            tabIndexAttribute: element.getAttribute('tabindex'),
        };
    };
    const readSystemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const readFocusTokens = () => {
        const probe = document.createElement('span');
        probe.style.display = 'block';
        probe.style.height = 'var(--cem-stroke-indicator-offset)';
        probe.style.width = 'var(--cem-stroke-focus)';
        document.querySelector('main').append(probe);
        const styles = getComputedStyle(probe);
        const tokens = { focusOffset: styles.height, focusWidth: styles.width };
        probe.remove();
        return tokens;
    };
    const ids = [
        'static-host',
        'static-owner',
        'static-child',
        'dialog-host',
        'dialog-owner',
        'shell-host',
        'shell-owner',
        'authored-host',
        'authored-owner',
        'authored-target',
        'sheet-host',
        'sheet-owner',
        'sheet-target',
    ];

    return {
        activeElement: document.activeElement?.id ?? '',
        elements: Object.fromEntries(ids.map((id) => [toCamelCase(id), readElement(id)])),
        forcedColors: matchMedia('(forced-colors: active)').matches,
        mutationEvents: [...window.__feedbackMutationEvents],
        stateEvents: [...window.__feedbackStateEvents],
        system: { canvasText: readSystemColor('CanvasText') },
        tokens: readFocusTokens(),
    };
}

function resetFeedbackObservation() {
    window.__feedbackObserver.takeRecords();
    window.__feedbackMutationEvents = [];
    window.__feedbackStateEvents = [];
}

function restartFeedbackObservation() {
    window.__feedbackMutationEvents = [];
    window.__feedbackStateEvents = [];
    window.__feedbackObserver.observe(document.querySelector('main'), {
        attributeFilter: ['aria-expanded', 'aria-hidden', 'open', 'tabindex', 'value'],
        attributes: true,
        subtree: true,
    });
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function camelCase(id) {
    return id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

function equalOutline(first, second) {
    return (
        first.outlineColor === second.outlineColor
        && first.outlineOffset === second.outlineOffset
        && first.outlineStyle === second.outlineStyle
        && first.outlineWidth === second.outlineWidth
    );
}

function equalRect(first, second) {
    return first.length === second.length && first.every((value, index) => value === second[index]);
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
