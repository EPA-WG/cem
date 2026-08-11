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
            <cem-paginator id="records-host" length="120" page-index="1" page-size="25">
                <nav class="cem-paginator" aria-label="Record pages">
                    <label class="cem-paginator__page-size">
                        <span class="cem-paginator__page-size-label">Items per page</span>
                        <select id="page-size" class="cem-paginator__page-size-control">
                            <option>10</option>
                            <option selected>25</option>
                            <option>50</option>
                        </select>
                    </label>
                    <div class="cem-paginator__range-actions">
                        <span class="cem-paginator__range" role="status" aria-live="polite" aria-atomic="true">26 – 50 of 120</span>
                        <button id="first" type="button" class="cem-paginator__action" data-page-action="first" aria-disabled="true" tabindex="-1" aria-label="First page"><span class="cem-paginator__icon" aria-hidden="true">«</span></button>
                        <button id="previous" type="button" class="cem-paginator__action" data-page-action="previous" aria-label="Previous page"><span class="cem-paginator__icon" aria-hidden="true">‹</span></button>
                        <button id="next" type="button" class="cem-paginator__action" data-page-action="next" aria-label="Next page"><span class="cem-paginator__icon" aria-hidden="true">›</span></button>
                    </div>
                </nav>
            </cem-paginator>
            <cem-paginator id="disabled-host" disabled>
                <nav class="cem-paginator" aria-label="Disabled pages">
                    <label class="cem-paginator__page-size">
                        <span>Items per page</span>
                        <select id="disabled-select" class="cem-paginator__page-size-control" disabled><option>50</option></select>
                    </label>
                    <div class="cem-paginator__range-actions">
                        <span class="cem-paginator__range" role="status">0 – 0 of 0</span>
                        <button id="disabled-next" type="button" class="cem-paginator__action" disabled aria-disabled="true" tabindex="-1" aria-label="Next page"><span class="cem-paginator__icon" aria-hidden="true">›</span></button>
                    </div>
                </nav>
            </cem-paginator>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__paginatorEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change']) {
            document.querySelector('#next').addEventListener(eventName, (event) => {
                window.__paginatorEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assertPaint(baseline.owner, baseline.system.canvas, baseline.system.canvasText, 'paginator owner');
    assertPaint(baseline.select, baseline.system.canvas, baseline.system.canvasText, 'page-size select');
    assertPaint(baseline.next, baseline.system.canvas, baseline.system.canvasText, 'next action');
    assertPaint(baseline.first, baseline.system.canvas, baseline.system.grayText, 'boundary action');
    assertPaint(baseline.disabledNext, baseline.system.canvas, baseline.system.grayText, 'disabled action');
    assertPaint(baseline.disabledSelect, baseline.system.canvas, baseline.system.grayText, 'disabled select');
    assert(baseline.select.borderColor === baseline.system.canvasText, 'select boundary did not use CanvasText');
    assert(baseline.disabledSelect.borderColor === baseline.system.grayText, 'disabled select boundary did not use GrayText');
    assertClose(baseline.next.width, baseline.tokens.actionSize, 'action inline size');
    assertClose(baseline.next.height, baseline.tokens.actionSize, 'action block size');
    assertClose(baseline.next.iconWidth, baseline.tokens.iconSize, 'icon inline size');
    assertClose(baseline.next.iconHeight, baseline.tokens.iconSize, 'icon block size');
    assert(baseline.select.height >= baseline.tokens.controlHeight, 'select is below the control-height target');

    await page.locator('#next').hover();
    const hovered = await page.evaluate(captureState);
    assertPaint(hovered.next, hovered.system.highlight, hovered.system.highlightText, 'hovered action');
    assertActionGeometry(hovered.next, baseline.next, 'hover changed action geometry');
    assert(hovered.hostHtml === baseline.hostHtml, 'hover changed paginator DOM or state');

    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root: documentNode } = await cdp.send('DOM.getDocument');
    const { nodeId: nextNodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#next',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId: nextNodeId, forcedPseudoClasses: ['active', 'hover'] });
    const active = await page.evaluate(captureState);
    assertPaint(active.next, active.system.highlight, active.system.highlightText, 'active action');
    assertActionGeometry(active.next, baseline.next, 'active changed action geometry');
    assert(active.hostHtml === baseline.hostHtml, 'active input changed paginator DOM or state');
    await cdp.send('CSS.forcePseudoState', { nodeId: nextNodeId, forcedPseudoClasses: [] });

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const selectFocused = await page.evaluate(captureState);
    assert(selectFocused.select.focusVisible, 'page-size select did not match :focus-visible');
    assert(selectFocused.select.outlineColor === selectFocused.system.canvasText, 'select focus did not use CanvasText');
    assertClose(selectFocused.select.outlineWidth, selectFocused.tokens.focusStroke, 'select focus stroke');
    assertClose(selectFocused.select.outlineOffset, selectFocused.tokens.focusOffset, 'select focus offset');

    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    const actionFocused = await page.evaluate(captureState);
    assert(actionFocused.next.focusVisible, 'next action did not match :focus-visible');
    assert(actionFocused.next.outlineColor === actionFocused.system.canvasText, 'action focus did not use CanvasText');
    assertClose(actionFocused.next.outlineWidth, actionFocused.tokens.focusStroke, 'action focus stroke');
    assertClose(actionFocused.next.outlineOffset, actionFocused.tokens.focusOffset, 'action focus offset');
    assertActionGeometry(actionFocused.next, baseline.next, 'focus changed action geometry');

    await page.locator('#first').hover();
    const boundaryHovered = await page.evaluate(captureState);
    assertPaint(boundaryHovered.first, boundaryHovered.system.canvas, boundaryHovered.system.grayText, 'hovered boundary action');
    assert(boundaryHovered.hostHtml === baseline.hostHtml, 'boundary hover changed paginator DOM or state');

    await page.locator('#next').click();
    await page.locator('#pointer-away').hover();
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient pointer/focus input changed paginator DOM or state');
    assert(
        finalState.events.join('|') === 'pointerenter:true|pointerleave:true|pointerenter:true|click:true|pointerleave:true',
        `unexpected paginator event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components paginator forced-colors contract verified.');
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
        probe.style.position = 'absolute';
        document.body.append(probe);
        const resolved = pixels(getComputedStyle(probe).inlineSize);
        probe.remove();
        return resolved;
    }

    function actionState(selector) {
        const action = document.querySelector(selector);
        const icon = action.querySelector('.cem-paginator__icon');
        const rect = action.getBoundingClientRect();
        const iconRect = icon.getBoundingClientRect();
        const style = getComputedStyle(action);
        return {
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: action.matches(':focus-visible'),
            height: rect.height,
            iconHeight: iconRect.height,
            iconWidth: iconRect.width,
            outlineColor: style.outlineColor,
            outlineOffset: pixels(style.outlineOffset),
            outlineWidth: pixels(style.outlineWidth),
            width: rect.width,
        };
    }

    function controlState(selector) {
        const control = document.querySelector(selector);
        const rect = control.getBoundingClientRect();
        const style = getComputedStyle(control);
        return {
            backgroundColor: style.backgroundColor,
            borderColor: style.borderBlockEndColor,
            color: style.color,
            focusVisible: control.matches(':focus-visible'),
            height: rect.height,
            outlineColor: style.outlineColor,
            outlineOffset: pixels(style.outlineOffset),
            outlineWidth: pixels(style.outlineWidth),
            width: rect.width,
        };
    }

    const owner = document.querySelector('#records-host > .cem-paginator');
    const ownerStyle = getComputedStyle(owner);
    return {
        disabledNext: actionState('#disabled-next'),
        disabledSelect: controlState('#disabled-select'),
        events: [...window.__paginatorEvents],
        first: actionState('#first'),
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: document.querySelector('#records-host').outerHTML,
        next: actionState('#next'),
        owner: {
            backgroundColor: ownerStyle.backgroundColor,
            color: ownerStyle.color,
        },
        select: controlState('#page-size'),
        system: {
            canvas: resolveColor('Canvas'),
            canvasText: resolveColor('CanvasText'),
            grayText: resolveColor('GrayText'),
            highlight: resolveColor('Highlight'),
            highlightText: resolveColor('HighlightText'),
        },
        tokens: {
            actionSize: resolveLength('--cem-icon-button-size'),
            controlHeight: resolveLength('--cem-control-height'),
            focusOffset: resolveLength('--cem-stroke-indicator-offset'),
            focusStroke: resolveLength('--cem-stroke-focus'),
            iconSize: resolveLength('--cem-icon-button-icon-size'),
        },
    };
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function assertActionGeometry(actual, expected, message) {
    assertClose(actual.width, expected.width, `${message} (width)`);
    assertClose(actual.height, expected.height, `${message} (height)`);
    assertClose(actual.iconWidth, expected.iconWidth, `${message} (icon width)`);
    assertClose(actual.iconHeight, expected.iconHeight, `${message} (icon height)`);
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) throw new Error(`${message}: ${actual} !== ${expected}`);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
