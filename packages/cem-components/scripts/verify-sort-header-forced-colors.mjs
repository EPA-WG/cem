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
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Focus start</button>
            <cem-sort-header id="none-host">
                <div class="cem-sort-header" role="columnheader">
                    <button id="none-button" type="button" class="cem-sort-header__button" aria-label="Sort by Name">
                        <span class="cem-sort-header__label">Name</span>
                        <span class="cem-sort-header__indicator" aria-hidden="true">◇</span>
                    </button>
                </div>
            </cem-sort-header>
            <cem-sort-header id="ascending-host" direction="ascending">
                <div class="cem-sort-header" role="columnheader" aria-sort="ascending">
                    <button id="ascending-button" type="button" class="cem-sort-header__button" aria-label="Sort by Created">
                        <span class="cem-sort-header__label">Created</span>
                        <span class="cem-sort-header__indicator" aria-hidden="true">▲</span>
                    </button>
                </div>
            </cem-sort-header>
            <cem-sort-header id="descending-host" direction="descending">
                <div class="cem-sort-header" role="columnheader" aria-sort="descending">
                    <button id="descending-button" type="button" class="cem-sort-header__button" aria-label="Sort by Updated">
                        <span class="cem-sort-header__label">Updated</span>
                        <span class="cem-sort-header__indicator" aria-hidden="true">▼</span>
                    </button>
                </div>
            </cem-sort-header>
            <cem-sort-header id="disabled-host" disabled>
                <div class="cem-sort-header" role="columnheader">
                    <button id="disabled-button" type="button" class="cem-sort-header__button" aria-label="Sort by Status" disabled>
                        <span class="cem-sort-header__label">Status</span>
                        <span class="cem-sort-header__indicator" aria-hidden="true">◇</span>
                    </button>
                </div>
            </cem-sort-header>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__sortEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-sort']) {
            document.querySelector('#none-button').addEventListener(eventName, (event) => {
                window.__sortEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assertPaint(baseline.none, baseline.system.canvas, baseline.system.canvasText, 'none button');
    assertPaint(baseline.ascending, baseline.system.canvas, baseline.system.canvasText, 'ascending button');
    assertPaint(baseline.descending, baseline.system.canvas, baseline.system.canvasText, 'descending button');
    assertPaint(baseline.disabled, baseline.system.canvas, baseline.system.grayText, 'disabled button');
    assert(baseline.none.forcedColorAdjust === 'auto', 'sort button must retain automatic forced-color adjustment');
    assert(baseline.none.indicator === '◇', 'none direction mark changed');
    assert(baseline.ascending.indicator === '▲', 'ascending direction mark changed');
    assert(baseline.descending.indicator === '▼', 'descending direction mark changed');
    assert(baseline.none.ariaSort === null, 'none button gained aria-sort');
    assert(baseline.ascending.ariaSort === 'ascending', 'ascending aria-sort changed');
    assert(baseline.descending.ariaSort === 'descending', 'descending aria-sort changed');
    assert(
        baseline.none.height >= Math.max(baseline.tokens.tableRowHeight, baseline.tokens.zoneMinimum),
        'sort target is below the accepted D2/D2c floor',
    );
    assertClose(baseline.none.indicatorWidth, baseline.tokens.iconSize, 'indicator inline size');
    assertClose(baseline.none.indicatorHeight, baseline.tokens.iconSize, 'indicator block size');
    assertButtonGeometry(baseline.ascending, baseline.none, 'ascending changed geometry');
    assertButtonGeometry(baseline.descending, baseline.none, 'descending changed geometry');

    await page.locator('#none-button').hover();
    const hovered = await page.evaluate(captureState);
    assertPaint(hovered.none, hovered.system.highlight, hovered.system.highlightText, 'hovered button');
    assertButtonGeometry(hovered.none, baseline.none, 'hover changed geometry');
    assert(hovered.none.ariaSort === null, 'hover changed sort state');

    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root: documentNode } = await cdp.send('DOM.getDocument');
    const { nodeId: buttonNodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#none-button',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId: buttonNodeId, forcedPseudoClasses: ['active', 'hover'] });
    const active = await page.evaluate(captureState);
    assertPaint(active.none, active.system.highlight, active.system.highlightText, 'active button');
    assertButtonGeometry(active.none, baseline.none, 'active changed geometry');
    assert(active.none.ariaSort === null, 'held active input changed sort state');
    await cdp.send('CSS.forcePseudoState', { nodeId: buttonNodeId, forcedPseudoClasses: [] });

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(captureState);
    assert(focused.none.focusVisible, 'keyboard focus did not match :focus-visible');
    assert(
        focused.none.outlineColor === focused.system.canvasText,
        `focus outline did not use CanvasText: ${focused.none.outlineColor}`,
    );
    assertClose(focused.none.outlineWidth, focused.tokens.focusStroke, 'focus stroke width');
    assertClose(focused.none.outlineOffset, focused.tokens.focusOffset, 'focus outline offset');
    assertButtonGeometry(focused.none, baseline.none, 'focus changed geometry');

    await page.locator('#none-button').hover();
    const focusedHovered = await page.evaluate(captureState);
    assertPaint(focusedHovered.none, focusedHovered.system.highlight, focusedHovered.system.highlightText, 'focused hovered button');
    assertClose(focusedHovered.none.outlineWidth, focusedHovered.tokens.focusStroke, 'hover suppressed focus stroke');
    assertButtonGeometry(focusedHovered.none, baseline.none, 'focus plus hover changed geometry');

    await page.locator('#pointer-away').hover();
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient pointer/focus input changed sort DOM or ARIA');
    assert(
        finalState.events.join('|') === 'pointerenter:true|pointerleave:true',
        `unexpected sort event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components sort-header forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureState() {
    function pixels(value) {
        const parsed = Number.parseFloat(value);
        if (!Number.isFinite(parsed)) throw new Error(`Expected a resolved CSS length, received ${value}`);
        return parsed;
    }

    function resolveColor(value) {
        const probe = document.createElement('span');
        probe.style.color = value;
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    }

    function resolveLength(token) {
        const probe = document.createElement('span');
        probe.style.display = 'block';
        probe.style.inlineSize = `var(${token})`;
        document.body.append(probe);
        const resolved = pixels(getComputedStyle(probe).inlineSize);
        probe.remove();
        return resolved;
    }

    function buttonState(selector) {
        const button = document.querySelector(selector);
        const owner = button.parentElement;
        const indicator = button.querySelector('.cem-sort-header__indicator');
        const rect = button.getBoundingClientRect();
        const indicatorRect = indicator.getBoundingClientRect();
        const style = getComputedStyle(button);
        return {
            ariaSort: owner.getAttribute('aria-sort'),
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: button.matches(':focus-visible'),
            forcedColorAdjust: style.forcedColorAdjust,
            height: rect.height,
            indicator: indicator.textContent,
            indicatorHeight: indicatorRect.height,
            indicatorWidth: indicatorRect.width,
            outlineColor: style.outlineColor,
            outlineOffset: pixels(style.outlineOffset),
            outlineWidth: pixels(style.outlineWidth),
            width: rect.width,
        };
    }

    return {
        ascending: buttonState('#ascending-button'),
        descending: buttonState('#descending-button'),
        disabled: buttonState('#disabled-button'),
        events: [...window.__sortEvents],
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: document.querySelector('#none-host').outerHTML,
        none: buttonState('#none-button'),
        system: {
            canvas: resolveColor('Canvas'),
            canvasText: resolveColor('CanvasText'),
            grayText: resolveColor('GrayText'),
            highlight: resolveColor('Highlight'),
            highlightText: resolveColor('HighlightText'),
        },
        tokens: {
            focusOffset: resolveLength('--cem-stroke-indicator-offset'),
            focusStroke: resolveLength('--cem-stroke-focus'),
            iconSize: resolveLength('--cem-icon-button-icon-size'),
            tableRowHeight: resolveLength('--cem-table-row-height'),
            zoneMinimum: resolveLength('--cem-coupling-zone-min'),
        },
    };
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function assertButtonGeometry(actual, expected, message) {
    assertClose(actual.width, expected.width, `${message} (width)`);
    assertClose(actual.height, expected.height, `${message} (height)`);
    assertClose(actual.indicatorWidth, expected.indicatorWidth, `${message} (indicator width)`);
    assertClose(actual.indicatorHeight, expected.indicatorHeight, `${message} (indicator height)`);
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) throw new Error(`${message}: ${actual} !== ${expected}`);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
