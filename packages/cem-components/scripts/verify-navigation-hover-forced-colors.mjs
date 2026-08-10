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
    const cdp = await context.newCDPSession(page);
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Start</button>
            <cem-nav id="primary-host">
                <nav id="primary-wrapper" class="cem-nav" aria-label="Primary navigation">
                    <a id="nav-default" href="#overview">Overview</a>
                    <a id="nav-current" href="#current" aria-current="page">Current page</a>
                    <a id="nav-aria-disabled" href="#unavailable" aria-disabled="true">Unavailable link</a>
                    <button id="nav-disabled" type="button" disabled>Unavailable action</button>
                </nav>
            </cem-nav>
            <cem-nav id="collapsible-host" collapsible expanded>
                <nav id="collapsible-wrapper" class="cem-nav cem-nav--collapsible" aria-label="Workspace navigation">
                    <button id="nav-disclosure" type="button" class="cem-nav__disclosure" aria-expanded="true">
                        Workspace navigation
                    </button>
                    <div class="cem-nav__content"><a id="nav-content" href="#workspace">Workspace</a></div>
                </nav>
            </cem-nav>
            <cem-tabs id="tabs-host">
                <div id="tabs-wrapper" class="cem-tabs" role="tablist" aria-label="Profile sections">
                    <button id="tab-default" type="button" role="tab" aria-selected="false">Overview tab</button>
                    <button id="tab-selected" type="button" role="tab" aria-selected="true">Security tab</button>
                    <button id="tab-disabled" type="button" role="tab" aria-selected="false" disabled>
                        Disabled tab
                    </button>
                </div>
            </cem-tabs>
            <button id="focus-end" type="button">End</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__navigationPointerEvents = {};
        window.__navigationFocusOrder = [];
        window.__navigationMutationEvents = [];
        for (const id of [
            'nav-default',
            'nav-current',
            'nav-disclosure',
            'nav-content',
            'tab-default',
            'tab-selected',
            'nav-aria-disabled',
            'nav-disabled',
            'tab-disabled',
        ]) {
            const element = document.getElementById(id);
            window.__navigationPointerEvents[id] = [];
            for (const eventName of ['pointerenter', 'pointerleave']) {
                element.addEventListener(eventName, (event) => {
                    window.__navigationPointerEvents[id].push(`${eventName}:${event.isTrusted}`);
                });
            }
        }
        for (const eventName of ['click', 'input', 'change']) {
            document.addEventListener(eventName, (event) => {
                if (eventName === 'click' && event.target instanceof HTMLAnchorElement) {
                    event.preventDefault();
                }
                window.__navigationMutationEvents.push(eventName);
            });
        }
        document.addEventListener('focusin', (event) => {
            if (
                event.target instanceof HTMLElement
                && (event.target.id.startsWith('nav-') || event.target.id.startsWith('tab-'))
            ) {
                window.__navigationFocusOrder.push(event.target.id);
            }
        });
    });

    const baseline = await page.evaluate(captureNavigationForcedColorState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(
        baseline.items.navCurrent.backgroundColor === baseline.system.selectedItem,
        'current navigation link did not map to SelectedItem',
    );
    assert(
        baseline.items.navCurrent.color === baseline.system.selectedItemText,
        'current navigation link text did not map to SelectedItemText',
    );
    assert(
        baseline.items.tabSelected.backgroundColor === baseline.system.selectedItem,
        'selected tab did not map to SelectedItem',
    );
    for (const id of ['navAriaDisabled', 'navDisabled', 'tabDisabled']) {
        assert(baseline.items[id].backgroundColor === baseline.system.canvas, `${id} did not map to Canvas`);
        assert(baseline.items[id].color === baseline.system.grayText, `${id} did not map to GrayText`);
    }

    await page.evaluate(() => {
        window.__navigationFocusOrder = [];
    });
    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    assert(
        await page.locator('#nav-default').evaluate((element) => document.activeElement === element),
        'nav link did not receive keyboard focus',
    );
    const focusedBaseline = await page.evaluate(captureNavigationForcedColorState);
    assert(focusedBaseline.items.navDefault.focusVisible, 'focused navigation link did not match :focus-visible');
    assert(focusedBaseline.items.navDefault.outlineStyle !== 'none', 'focused navigation link lost its outline');

    for (const id of ['nav-default', 'nav-current', 'nav-disclosure', 'nav-content', 'tab-default', 'tab-selected']) {
        const key = id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
        const needsForcedPseudoState = ['nav-disclosure', 'tab-default', 'tab-selected'].includes(id);
        const before = await page.evaluate(captureNavigationForcedColorState);
        await hoverOwner(page, id);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover']);
        }
        await page.evaluate(nextFrame);
        const hovered = await page.evaluate(captureNavigationForcedColorState);
        if (!needsForcedPseudoState) {
            assert(hovered.items[key].hover, `${id} did not match :hover`);
        }
        assert(hovered.items[key].backgroundColor === hovered.system.highlight, `${id} hover did not map to Highlight`);
        assert(
            hovered.items[key].color === hovered.system.highlightText,
            `${id} hover text did not map to HighlightText`,
        );
        assert(equalRect(hovered.items[key].rect, before.items[key].rect), `${id} hover changed owner geometry`);
        assert(equalRect(hovered.hosts[key].rect, before.hosts[key].rect), `${id} hover changed host geometry`);
        assert(hovered.hosts[key].html === before.hosts[key].html, `${id} hover changed host DOM/ARIA`);
        assert(
            hovered.wrappers[key].backgroundColor === before.wrappers[key].backgroundColor,
            `${id} styled a structural wrapper`,
        );
        if (id === 'nav-default') {
            assert(hovered.items[key].focusVisible, 'hover removed :focus-visible from the focused navigation link');
            assert(
                equalOutline(hovered.items[key], focusedBaseline.items.navDefault),
                'hover changed the focused navigation link outline',
            );
        }
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, []);
        }
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
        const restored = await page.evaluate(captureNavigationForcedColorState);
        assert(
            restored.items[key].backgroundColor === before.items[key].backgroundColor,
            `${id} did not restore its fill`,
        );
        assert(restored.items[key].color === before.items[key].color, `${id} did not restore its text`);
    }

    for (const id of ['nav-aria-disabled', 'nav-disabled', 'tab-disabled']) {
        const key = id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
        const needsForcedPseudoState = ['nav-disabled', 'tab-disabled'].includes(id);
        const before = await page.evaluate(captureNavigationForcedColorState);
        await hoverOwner(page, id);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover']);
        }
        await page.evaluate(nextFrame);
        const hovered = await page.evaluate(captureNavigationForcedColorState);
        if (!needsForcedPseudoState) {
            assert(hovered.items[key].hover, `${id} did not receive pointer designation`);
        }
        assert(hovered.items[key].backgroundColor === before.items[key].backgroundColor, `${id} acquired hover fill`);
        assert(hovered.items[key].color === before.items[key].color, `${id} acquired hover text`);
        assert(equalRect(hovered.items[key].rect, before.items[key].rect), `${id} hover changed geometry`);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, []);
        }
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
    }

    const finalState = await page.evaluate(captureNavigationForcedColorState);
    for (const id of [
        'nav-default',
        'nav-current',
        'nav-disclosure',
        'nav-content',
        'tab-default',
        'tab-selected',
        'nav-aria-disabled',
        'nav-disabled',
        'tab-disabled',
    ]) {
        assert(
            finalState.pointerEvents[id].join('|') === 'pointerenter:true|pointerleave:true',
            `${id} did not receive one trusted pointer enter/leave pair`,
        );
    }
    assert(finalState.mutationEvents.length === 0, 'navigation hover dispatched a mutation event');
    assert(finalState.items.navCurrent.ariaCurrent === 'page', 'current navigation state mutated');
    assert(finalState.items.tabSelected.ariaSelected === 'true', 'selected tab state mutated');

    await page.evaluate(() => {
        window.__navigationFocusOrder = [];
    });
    await page.locator('#focus-start').focus();
    const focusBaseline = await page.evaluate(captureNavigationForcedColorState);
    const focusOrder = [
        'nav-default',
        'nav-current',
        'nav-aria-disabled',
        'nav-disclosure',
        'nav-content',
        'tab-default',
        'tab-selected',
    ];
    for (const [index, id] of focusOrder.entries()) {
        await page.keyboard.press('Tab');
        await page.evaluate(nextFrame);
        const key = id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
        const focused = await page.evaluate(captureNavigationForcedColorState);
        assert(focused.activeElement === id, `${id} did not receive keyboard focus`);
        assert(focused.items[key].focusVisible, `${id} did not match :focus-visible`);
        assert(focused.items[key].outlineColor === focused.system.canvasText, `${id} focus did not map to CanvasText`);
        assert(focused.items[key].outlineStyle === 'solid', `${id} focus outline was not solid`);
        assert(
            focused.items[key].outlineWidth === focused.tokens.focusWidth,
            `${id} focus width did not resolve from --cem-stroke-focus`,
        );
        assert(
            focused.items[key].outlineOffset === focused.tokens.focusOffset,
            `${id} focus offset did not resolve from --cem-stroke-indicator-offset: ${focused.items[key].outlineOffset} !== ${focused.tokens.focusOffset}`,
        );
        assert(
            equalRect(focused.items[key].rect, focusBaseline.items[key].rect),
            `${id} keyboard focus changed owner geometry`,
        );
        assert(
            equalRect(focused.hosts[key].rect, focusBaseline.hosts[key].rect),
            `${id} keyboard focus changed host geometry`,
        );
        assert(focused.hosts[key].html === focusBaseline.hosts[key].html, `${id} keyboard focus changed DOM/ARIA`);
        assert(
            equalOutline(focused.wrappers[key], focusBaseline.wrappers[key]),
            `${id} keyboard focus styled a structural wrapper`,
        );
        if (index > 0) {
            const previousId = focusOrder[index - 1];
            const previousKey = previousId.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
            assert(!focused.items[previousKey].focusVisible, `${previousId} retained :focus-visible after Tab`);
            assert(
                equalOutline(focused.items[previousKey], focusBaseline.items[previousKey]),
                `${previousId} focus outline did not restore`,
            );
        }
        assert(focused.activeElement !== 'nav-disabled', 'native disabled navigation button entered the tab order');
        assert(focused.activeElement !== 'tab-disabled', 'native disabled tab entered the tab order');
    }

    await page.keyboard.press('Tab');
    await page.evaluate(nextFrame);
    const restoredFocus = await page.evaluate(captureNavigationForcedColorState);
    assert(restoredFocus.activeElement === 'focus-end', 'focus did not leave navigation at the end sentinel');
    assert(!restoredFocus.items.tabSelected.focusVisible, 'selected tab retained :focus-visible after Tab');
    assert(
        equalOutline(restoredFocus.items.tabSelected, focusBaseline.items.tabSelected),
        'selected tab focus outline did not restore',
    );
    assert(
        restoredFocus.focusOrder.join('|') === focusOrder.join('|'),
        `navigation keyboard order changed or included a native disabled owner: ${restoredFocus.focusOrder.join('|')}`,
    );
    assert(restoredFocus.mutationEvents.length === 0, 'navigation focus dispatched a mutation event');
    assert(restoredFocus.items.navCurrent.ariaCurrent === 'page', 'focus changed current navigation state');
    assert(restoredFocus.items.tabSelected.ariaSelected === 'true', 'focus changed selected tab state');

    await page.evaluate(() => {
        window.__navigationMutationEvents = [];
    });
    for (const id of ['nav-default', 'nav-current', 'nav-disclosure', 'nav-content', 'tab-default', 'tab-selected']) {
        const key = id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
        const needsForcedPseudoState = ['nav-disclosure', 'tab-default', 'tab-selected'].includes(id);
        if (id === 'nav-current') {
            await page.locator('#focus-start').focus();
            await page.keyboard.press('Tab');
            await page.keyboard.press('Tab');
            await page.evaluate(nextFrame);
        }
        const before = await page.evaluate(captureNavigationForcedColorState);
        const mutationCount = before.mutationEvents.length;
        await hoverOwner(page, id);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover', 'active']);
        } else {
            await page.mouse.down();
        }
        await page.evaluate(nextFrame);
        const active = await page.evaluate(captureNavigationForcedColorState);
        if (!needsForcedPseudoState) {
            assert(active.items[key].active, `${id} did not match :active during a trusted pointer hold`);
        }
        assert(active.items[key].backgroundColor === active.system.highlight, `${id} active did not map to Highlight`);
        assert(
            active.items[key].color === active.system.highlightText,
            `${id} active text did not map to HighlightText`,
        );
        assert(equalRect(active.items[key].rect, before.items[key].rect), `${id} active changed owner geometry`);
        assert(equalRect(active.hosts[key].rect, before.hosts[key].rect), `${id} active changed host geometry`);
        assert(active.hosts[key].html === before.hosts[key].html, `${id} active changed host DOM/ARIA`);
        assert(active.mutationEvents.length === mutationCount, `${id} mutated before pointer release`);
        assert(
            active.wrappers[key].backgroundColor === before.wrappers[key].backgroundColor,
            `${id} active styled a structural wrapper`,
        );
        if (id === 'nav-current') {
            assert(active.items[key].focusVisible, 'active removed :focus-visible from the current navigation link');
            assert(equalOutline(active.items[key], before.items[key]), 'active changed the navigation focus outline');
        }
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover']);
            await page.evaluate(nextFrame);
            const released = await page.evaluate(captureNavigationForcedColorState);
            assert(!released.items[key].active, `${id} retained :active after inspection release`);
            await forcePseudoState(cdp, id, []);
        } else {
            await page.mouse.up();
        }
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
        const restored = await page.evaluate(captureNavigationForcedColorState);
        assert(!restored.items[key].active, `${id} retained :active after pointer release`);
        assert(
            restored.items[key].backgroundColor === before.items[key].backgroundColor,
            `${id} did not restore its fill after active: ${restored.items[key].backgroundColor} !== ${before.items[key].backgroundColor}`,
        );
        assert(restored.items[key].color === before.items[key].color, `${id} did not restore its text after active`);
        const expectedClicks = needsForcedPseudoState ? 0 : 1;
        assert(
            restored.mutationEvents.length === mutationCount + expectedClicks,
            `${id} release-time click boundary changed`,
        );
    }

    for (const id of ['nav-aria-disabled', 'nav-disabled', 'tab-disabled']) {
        const key = id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
        const needsForcedPseudoState = ['nav-disabled', 'tab-disabled'].includes(id);
        const before = await page.evaluate(captureNavigationForcedColorState);
        const mutationCount = before.mutationEvents.length;
        await hoverOwner(page, id);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover', 'active']);
        } else {
            await page.mouse.down();
        }
        await page.evaluate(nextFrame);
        const active = await page.evaluate(captureNavigationForcedColorState);
        assert(active.items[key].backgroundColor === before.items[key].backgroundColor, `${id} acquired active fill`);
        assert(active.items[key].color === before.items[key].color, `${id} acquired active text`);
        assert(equalRect(active.items[key].rect, before.items[key].rect), `${id} active changed geometry`);
        assert(active.mutationEvents.length === mutationCount, `${id} mutated before pointer release`);
        if (needsForcedPseudoState) {
            await forcePseudoState(cdp, id, ['hover']);
            await page.evaluate(nextFrame);
            await forcePseudoState(cdp, id, []);
        } else {
            await page.mouse.up();
        }
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
        const restored = await page.evaluate(captureNavigationForcedColorState);
        const expectedClicks = id === 'nav-aria-disabled' ? 1 : 0;
        assert(
            restored.mutationEvents.length === mutationCount + expectedClicks,
            `${id} release-time click boundary changed`,
        );
    }

    const finalActiveState = await page.evaluate(captureNavigationForcedColorState);
    assert(
        finalActiveState.mutationEvents.every((eventName) => eventName === 'click'),
        'forced active interaction dispatched a non-click mutation event',
    );
    assert(finalActiveState.items.navCurrent.ariaCurrent === 'page', 'active changed current navigation state');
    assert(finalActiveState.items.tabSelected.ariaSelected === 'true', 'active changed selected tab state');

    await context.close();
    console.log('cem-components navigation hover/focus/active forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureNavigationForcedColorState() {
    const readSystemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const readItem = (id) => {
        const element = document.getElementById(id);
        const styles = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
            active: element.matches(':active'),
            ariaCurrent: element.getAttribute('aria-current'),
            ariaSelected: element.getAttribute('aria-selected'),
            backgroundColor: styles.backgroundColor,
            color: styles.color,
            focusVisible: element.matches(':focus-visible'),
            hover: element.matches(':hover'),
            outlineColor: styles.outlineColor,
            outlineOffset: styles.outlineOffset,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
            rect: [rect.x, rect.y, rect.width, rect.height],
        };
    };
    const readHost = (id) => {
        const element = document.getElementById(id);
        const rect = element.getBoundingClientRect();
        return { html: element.outerHTML, rect: [rect.x, rect.y, rect.width, rect.height] };
    };
    const readWrapper = (id) => {
        const element = document.getElementById(id);
        const styles = getComputedStyle(element);
        return {
            backgroundColor: styles.backgroundColor,
            outlineColor: styles.outlineColor,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
        };
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
    const itemIds = {
        navAriaDisabled: 'nav-aria-disabled',
        navContent: 'nav-content',
        navCurrent: 'nav-current',
        navDefault: 'nav-default',
        navDisabled: 'nav-disabled',
        navDisclosure: 'nav-disclosure',
        tabDefault: 'tab-default',
        tabDisabled: 'tab-disabled',
        tabSelected: 'tab-selected',
    };
    const ownership = {
        navAriaDisabled: ['primary-host', 'primary-wrapper'],
        navContent: ['collapsible-host', 'collapsible-wrapper'],
        navCurrent: ['primary-host', 'primary-wrapper'],
        navDefault: ['primary-host', 'primary-wrapper'],
        navDisabled: ['primary-host', 'primary-wrapper'],
        navDisclosure: ['collapsible-host', 'collapsible-wrapper'],
        tabDefault: ['tabs-host', 'tabs-wrapper'],
        tabDisabled: ['tabs-host', 'tabs-wrapper'],
        tabSelected: ['tabs-host', 'tabs-wrapper'],
    };

    return {
        activeElement: document.activeElement?.id ?? '',
        forcedColors: matchMedia('(forced-colors: active)').matches,
        focusOrder: [...window.__navigationFocusOrder],
        hosts: Object.fromEntries(Object.entries(ownership).map(([key, [host]]) => [key, readHost(host)])),
        items: Object.fromEntries(Object.entries(itemIds).map(([key, id]) => [key, readItem(id)])),
        mutationEvents: [...window.__navigationMutationEvents],
        pointerEvents: structuredClone(window.__navigationPointerEvents),
        system: {
            canvas: readSystemColor('Canvas'),
            canvasText: readSystemColor('CanvasText'),
            grayText: readSystemColor('GrayText'),
            highlight: readSystemColor('Highlight'),
            highlightText: readSystemColor('HighlightText'),
            selectedItem: readSystemColor('SelectedItem'),
            selectedItemText: readSystemColor('SelectedItemText'),
        },
        tokens: readFocusTokens(),
        wrappers: Object.fromEntries(
            Object.entries(ownership).map(([key, [, wrapper]]) => [key, readWrapper(wrapper)]),
        ),
    };
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

async function hoverOwner(page, id) {
    const box = await page.locator(`#${id}`).boundingBox();
    if (!box) {
        throw new Error(`Expected ${id} to expose hover geometry`);
    }
    await page.mouse.move(box.x + Math.min(4, box.width / 2), box.y + Math.min(4, box.height / 2));
}

async function forcePseudoState(cdp, id, forcedPseudoClasses) {
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: `#${id}` });
    if (!nodeId) {
        throw new Error(`Expected ${id} to expose a Chromium DOM node`);
    }
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses });
}

function equalOutline(actual, expected) {
    return (
        actual.outlineColor === expected.outlineColor &&
        actual.outlineStyle === expected.outlineStyle &&
        actual.outlineWidth === expected.outlineWidth
    );
}

function equalRect(actual, expected) {
    return actual.length === expected.length && actual.every((value, index) => value === expected[index]);
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
