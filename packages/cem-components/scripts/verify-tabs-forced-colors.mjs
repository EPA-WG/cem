#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = join(packageRoot, '..', '..');
const componentCss = await readFile(join(packageRoot, 'src', 'styles.css'), 'utf8');
const themeCss = await readFile(join(repoRoot, 'packages', 'cem-theme', 'dist', 'lib', 'css', 'cem-combined.css'), 'utf8');
const browser = await chromium.launch({ headless: true });

try {
    const context = await browser.newContext({ forcedColors: 'active', reducedMotion: 'reduce' });
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Focus start</button>
            <cem-tabs id="tabs-host">
                <div class="cem-tabs__list" role="tablist" aria-label="Workspace panes" aria-orientation="horizontal">
                    <button id="tab-default" class="cem-tabs__tab" type="button" role="tab" tabindex="-1" aria-selected="false" aria-controls="panel-default">Explorer</button>
                    <button id="tab-selected" class="cem-tabs__tab" type="button" role="tab" tabindex="0" aria-selected="true" aria-controls="panel-selected">Editor</button>
                    <button id="tab-disabled" class="cem-tabs__tab" type="button" role="tab" tabindex="-1" aria-selected="false" aria-controls="panel-disabled" disabled>Preview</button>
                </div>
                <div class="cem-tabs__panels">
                    <div id="panel-default" class="cem-tabs__panel" role="tabpanel" tabindex="0" aria-labelledby="tab-default" hidden>Explorer panel</div>
                    <div id="panel-selected" class="cem-tabs__panel" role="tabpanel" tabindex="0" aria-labelledby="tab-selected">Editor panel</div>
                    <div id="panel-disabled" class="cem-tabs__panel" role="tabpanel" tabindex="0" aria-labelledby="tab-disabled" hidden>Preview panel</div>
                </div>
            </cem-tabs>
        </main>
    `);

    const baseline = await page.evaluate(capture);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assertPaint(baseline.selected, baseline.system.selectedItem, baseline.system.selectedItemText, 'selected tab');
    assert(baseline.selected.indicator === baseline.system.highlight, 'selected indicator did not map to Highlight');
    assertPaint(baseline.disabled, baseline.system.canvas, baseline.system.grayText, 'disabled tab');
    assertPaint(baseline.panel, baseline.system.canvas, baseline.system.canvasText, 'selected panel');
    assert(baseline.selected.height >= baseline.tokens.zoneMinimum, 'tab fell below the D2 target minimum');
    assert(baseline.selected.animationName === 'none', 'tabs introduced component animation');
    assert(baseline.selected.transitionDuration === '0s', 'tabs introduced component transition');
    assert(baseline.selected.zIndex === 'auto', 'tabs introduced numeric z-index');

    await page.locator('#tab-default').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(capture);
    assertPaint(hovered.default, hovered.system.highlight, hovered.system.highlightText, 'hovered tab');
    assert(equalRect(hovered.default.rect, baseline.default.rect), 'hover changed tab geometry');
    assert(hovered.hostHtml === baseline.hostHtml, 'hover changed tab DOM or ARIA');

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(capture);
    assert(focused.activeElement === 'tab-selected', 'roving selected tab was not the entry stop');
    assert(focused.selected.focusVisible, 'selected tab did not retain :focus-visible');
    assert(focused.selected.outlineColor === focused.system.canvasText, 'focus outline did not map to CanvasText');
    assert(focused.selected.outlineWidth === focused.tokens.focus, 'focus width changed');
    assertPaint(focused.selected, focused.system.selectedItem, focused.system.selectedItemText, 'focused selected tab');
    assert(equalRect(focused.selected.rect, baseline.selected.rect), 'focus changed selected tab geometry');

    await forcePseudoState(cdp, '#tab-selected', ['hover', 'active']);
    const active = await page.evaluate(capture);
    assertPaint(active.selected, active.system.highlight, active.system.highlightText, 'active selected tab');
    assert(active.selected.indicator === active.system.highlight, 'active erased the selected indicator');
    assert(active.selected.outlineColor === active.system.canvasText, 'active erased the focus outline');
    assert(equalRect(active.selected.rect, baseline.selected.rect), 'active changed selected tab geometry');
    assert(active.hostHtml === baseline.hostHtml, 'active changed tab DOM or ARIA');
    await forcePseudoState(cdp, '#tab-selected', []);

    await forcePseudoState(cdp, '#tab-disabled', ['hover', 'active']);
    const disabled = await page.evaluate(capture);
    assertPaint(disabled.disabled, disabled.system.canvas, disabled.system.grayText, 'disabled active tab');
    assert(equalRect(disabled.disabled.rect, baseline.disabled.rect), 'disabled input changed tab geometry');
    assert(disabled.hostHtml === baseline.hostHtml, 'disabled input changed tab DOM or ARIA');
    await forcePseudoState(cdp, '#tab-disabled', []);

    console.log('cem-components tabs forced-colors contract verified.');
} finally {
    await browser.close();
}

function capture() {
    const required = (selector) => {
        const element = document.querySelector(selector);
        if (!(element instanceof HTMLElement)) throw new Error(`Expected ${selector}`);
        return element;
    };
    const systemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const lengthToken = (token) => {
        const probe = document.createElement('span');
        probe.style.inlineSize = `var(${token})`;
        document.body.append(probe);
        const value = getComputedStyle(probe).inlineSize;
        probe.remove();
        return value;
    };
    const item = (selector) => {
        const element = required(selector);
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
            animationName: style.animationName,
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: element.matches(':focus-visible'),
            height: rect.height,
            indicator: style.borderBottomColor,
            outlineColor: style.outlineColor,
            outlineWidth: style.outlineWidth,
            rect: [rect.x, rect.y, rect.width, rect.height],
            transitionDuration: style.transitionDuration,
            zIndex: style.zIndex,
        };
    };
    const host = required('#tabs-host');
    return {
        activeElement: document.activeElement?.id ?? '',
        default: item('#tab-default'),
        disabled: item('#tab-disabled'),
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.outerHTML,
        panel: item('#panel-selected'),
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        selected: item('#tab-selected'),
        system: {
            canvas: systemColor('Canvas'),
            canvasText: systemColor('CanvasText'),
            grayText: systemColor('GrayText'),
            highlight: systemColor('Highlight'),
            highlightText: systemColor('HighlightText'),
            selectedItem: systemColor('SelectedItem'),
            selectedItemText: systemColor('SelectedItemText'),
        },
        tokens: {
            focus: lengthToken('--cem-stroke-focus'),
            zoneMinimum: Number.parseFloat(lengthToken('--cem-coupling-zone-min')),
        },
    };
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(resolve));
}

async function forcePseudoState(cdp, selector, forcedPseudoClasses) {
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector });
    if (!nodeId) throw new Error(`Expected ${selector}`);
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses });
    await new Promise((resolve) => setTimeout(resolve, 0));
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function equalRect(left, right) {
    return left.length === right.length && left.every((value, index) => Math.abs(value - right[index]) <= 0.01);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
