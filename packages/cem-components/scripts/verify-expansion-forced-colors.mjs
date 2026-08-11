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
            <cem-expansion id="collapsed-host">
                <div class="cem-expansion">
                    <div class="cem-expansion__heading" role="heading" aria-level="3">
                        <button id="collapsed-header" type="button" class="cem-expansion__header" aria-labelledby="collapsed-summary" aria-expanded="false" aria-controls="collapsed-panel">
                            <span id="collapsed-summary" class="cem-expansion__summary">Account details</span>
                            <span class="cem-expansion__indicator" aria-hidden="true">▸</span>
                        </button>
                    </div>
                    <div id="collapsed-panel" class="cem-expansion__panel" aria-labelledby="collapsed-header" hidden>Account content</div>
                </div>
            </cem-expansion>
            <cem-expansion id="expanded-host" expanded>
                <div class="cem-expansion">
                    <div class="cem-expansion__heading" role="heading" aria-level="3">
                        <button id="expanded-header" type="button" class="cem-expansion__header" aria-labelledby="expanded-summary" aria-expanded="true" aria-controls="expanded-panel">
                            <span id="expanded-summary" class="cem-expansion__summary">Account details</span>
                            <span class="cem-expansion__indicator" aria-hidden="true">▾</span>
                        </button>
                    </div>
                    <div id="expanded-panel" class="cem-expansion__panel" aria-labelledby="expanded-header">Account content</div>
                </div>
            </cem-expansion>
            <cem-expansion id="disabled-host" disabled>
                <div class="cem-expansion">
                    <div class="cem-expansion__heading" role="heading" aria-level="3">
                        <button id="disabled-header" type="button" class="cem-expansion__header" aria-labelledby="disabled-summary" aria-expanded="false" aria-controls="disabled-panel" disabled>
                            <span id="disabled-summary" class="cem-expansion__summary">Disabled details</span>
                            <span class="cem-expansion__indicator" aria-hidden="true">▸</span>
                        </button>
                    </div>
                    <div id="disabled-panel" class="cem-expansion__panel" aria-labelledby="disabled-header" hidden>Disabled content</div>
                </div>
            </cem-expansion>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__expansionEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change']) {
            document.querySelector('#collapsed-header').addEventListener(eventName, (event) => {
                window.__expansionEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assertPaint(baseline.collapsed, baseline.system.canvas, baseline.system.canvasText, 'collapsed header');
    assertPaint(baseline.expanded, baseline.system.canvas, baseline.system.canvasText, 'expanded header');
    assertPaint(baseline.panel, baseline.system.canvas, baseline.system.canvasText, 'expanded panel');
    assertPaint(baseline.disabled, baseline.system.canvas, baseline.system.grayText, 'disabled header');
    assert(
        baseline.collapsed.height >= baseline.tokens.zoneMinimum,
        `header target height ${baseline.collapsed.height} is below ${baseline.tokens.zoneMinimum}`,
    );
    assertClose(baseline.collapsed.paddingBlock, baseline.tokens.controlPaddingY, 'header block padding');
    assertClose(baseline.collapsed.paddingInline, baseline.tokens.controlPaddingX, 'header inline padding');
    assertClose(baseline.collapsed.indicatorWidth, baseline.tokens.iconSize, 'indicator inline size');
    assertClose(baseline.collapsed.indicatorHeight, baseline.tokens.iconSize, 'indicator block size');
    assertClose(baseline.panel.paddingBlock, baseline.tokens.containerInset, 'panel block inset');
    assertClose(baseline.panel.paddingInline, baseline.tokens.containerInset, 'panel inline inset');
    assertHeaderGeometry(baseline.expanded, baseline.collapsed, 'expanded state changed header geometry');

    await page.locator('#collapsed-header').hover();
    const hovered = await page.evaluate(captureState);
    assertPaint(hovered.collapsed, hovered.system.highlight, hovered.system.highlightText, 'hovered header');
    assertHeaderGeometry(hovered.collapsed, baseline.collapsed, 'hover changed header geometry');
    assert(hovered.collapsed.ariaExpanded === 'false', 'hover changed expansion state');

    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root: documentNode } = await cdp.send('DOM.getDocument');
    const { nodeId: headerNodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#collapsed-header',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId: headerNodeId, forcedPseudoClasses: ['active', 'hover'] });
    const active = await page.evaluate(captureState);
    assertPaint(active.collapsed, active.system.highlight, active.system.highlightText, 'active header');
    assertHeaderGeometry(active.collapsed, baseline.collapsed, 'active changed header geometry');
    assert(active.collapsed.ariaExpanded === 'false', 'active input changed static expansion state before release');
    await cdp.send('CSS.forcePseudoState', { nodeId: headerNodeId, forcedPseudoClasses: [] });
    await page.locator('#collapsed-header').click();

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(captureState);
    assert(focused.collapsed.focusVisible, 'keyboard focus did not match :focus-visible');
    assert(
        focused.collapsed.outlineColor === focused.system.canvasText,
        `focus outline did not use CanvasText: ${focused.collapsed.outlineColor}`,
    );
    assertClose(focused.collapsed.outlineWidth, focused.tokens.focusStroke, 'focus stroke width');
    assertClose(focused.collapsed.outlineOffset, focused.tokens.focusOffset, 'focus outline offset');
    assertHeaderGeometry(focused.collapsed, baseline.collapsed, 'focus changed header geometry');
    assert(focused.collapsed.ariaExpanded === 'false', 'focus changed expansion state');

    await page.locator('#pointer-away').hover();
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient pointer/focus input changed expansion DOM or ARIA');
    assert(
        finalState.events.join('|') === 'pointerenter:true|click:true|pointerleave:true',
        `unexpected expansion event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components expansion forced-colors contract verified.');
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

    function headerState(selector) {
        const header = document.querySelector(selector);
        const indicator = header.querySelector('.cem-expansion__indicator');
        const rect = header.getBoundingClientRect();
        const indicatorRect = indicator.getBoundingClientRect();
        const style = getComputedStyle(header);
        return {
            active: header.matches(':active'),
            ariaExpanded: header.getAttribute('aria-expanded'),
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: header.matches(':focus-visible'),
            height: rect.height,
            indicatorHeight: indicatorRect.height,
            indicatorWidth: indicatorRect.width,
            outlineColor: style.outlineColor,
            outlineOffset: pixels(style.outlineOffset),
            outlineWidth: pixels(style.outlineWidth),
            paddingBlock: pixels(style.paddingBlockStart),
            paddingInline: pixels(style.paddingInlineStart),
            width: rect.width,
        };
    }

    function panelState(selector) {
        const panel = document.querySelector(selector);
        const style = getComputedStyle(panel);
        return {
            backgroundColor: style.backgroundColor,
            color: style.color,
            paddingBlock: pixels(style.paddingBlockStart),
            paddingInline: pixels(style.paddingInlineStart),
        };
    }

    return {
        collapsed: headerState('#collapsed-header'),
        disabled: headerState('#disabled-header'),
        events: [...window.__expansionEvents],
        expanded: headerState('#expanded-header'),
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: document.querySelector('#collapsed-host').outerHTML,
        panel: panelState('#expanded-panel'),
        system: {
            canvas: resolveColor('Canvas'),
            canvasText: resolveColor('CanvasText'),
            grayText: resolveColor('GrayText'),
            highlight: resolveColor('Highlight'),
            highlightText: resolveColor('HighlightText'),
        },
        tokens: {
            containerInset: resolveLength('--cem-inset-container'),
            controlPaddingX: resolveLength('--cem-control-padding-x'),
            controlPaddingY: resolveLength('--cem-control-padding-y'),
            focusOffset: resolveLength('--cem-stroke-indicator-offset'),
            focusStroke: resolveLength('--cem-stroke-focus'),
            iconSize: resolveLength('--cem-icon-button-icon-size'),
            zoneMinimum: resolveLength('--cem-coupling-zone-min'),
        },
    };
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function assertHeaderGeometry(actual, expected, message) {
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
