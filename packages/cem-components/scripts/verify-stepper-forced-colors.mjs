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
    const context = await browser.newContext({ forcedColors: 'active', reducedMotion: 'reduce' });
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Focus start</button>
            <cem-stepper id="stepper-host" selected-index="0">
                <section class="cem-stepper" data-orientation="horizontal" aria-label="Checkout">
                    <ol class="cem-stepper__steps">
                        <li id="completed-item" class="cem-stepper__item" data-completed="true">
                            <button id="current-header" type="button" class="cem-stepper__header" data-step-index="0" data-marker-state="completed" tabindex="0" aria-current="step" aria-controls="panel-current">
                                <span class="cem-stepper__marker" aria-hidden="true">✓</span>
                                <span class="cem-stepper__label">Account</span>
                                <span class="cem-stepper__status">Complete</span>
                            </button>
                        </li>
                        <li id="invalid-item" class="cem-stepper__item" data-completed="false">
                            <button id="invalid-header" type="button" class="cem-stepper__header" data-step-index="1" data-marker-state="invalid" tabindex="-1" aria-invalid="true" aria-controls="panel-invalid">
                                <span class="cem-stepper__marker" aria-hidden="true">!</span>
                                <span class="cem-stepper__label">Payment</span>
                                <span class="cem-stepper__status">Error</span>
                            </button>
                        </li>
                        <li class="cem-stepper__item" data-completed="true">
                            <button id="disabled-header" type="button" class="cem-stepper__header" data-step-index="2" data-marker-state="completed" tabindex="-1" disabled aria-disabled="true" aria-controls="panel-disabled">
                                <span class="cem-stepper__marker" aria-hidden="true">✓</span>
                                <span class="cem-stepper__label">Confirm</span>
                                <span class="cem-stepper__status">Complete</span>
                            </button>
                        </li>
                    </ol>
                    <div class="cem-stepper__panels">
                        <div id="panel-current" class="cem-stepper__panel" role="region" aria-labelledby="current-header">Account panel</div>
                        <div id="panel-invalid" class="cem-stepper__panel" role="region" aria-labelledby="invalid-header" hidden>Payment panel</div>
                        <div id="panel-disabled" class="cem-stepper__panel" role="region" aria-labelledby="disabled-header" hidden>Confirm panel</div>
                    </div>
                </section>
            </cem-stepper>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__stepperEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-step']) {
            document.querySelector('#invalid-header').addEventListener(eventName, (event) => {
                window.__stepperEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assertPaint(baseline.current, baseline.system.selectedItem, baseline.system.selectedItemText, 'current header');
    assertPaint(baseline.invalid, baseline.system.canvas, baseline.system.canvasText, 'invalid header');
    assertPaint(baseline.disabled, baseline.system.canvas, baseline.system.grayText, 'disabled header');
    assert(baseline.current.markerColor === baseline.system.highlight, 'completed marker did not map to Highlight');
    assert(baseline.invalid.markerColor === baseline.system.mark, 'invalid marker did not map to Mark');
    assert(baseline.completedConnector === baseline.system.highlight, 'completed connector did not map to Highlight');
    assert(baseline.remainingConnector === baseline.system.grayText, 'remaining connector did not map to GrayText');
    assert(baseline.current.height >= baseline.tokens.zoneMinimum, 'step header fell below the D2 target minimum');
    assert(baseline.current.animationName === 'none', 'step header introduced component animation');
    assert(baseline.current.transitionDuration === '0s', 'step header introduced component transition');
    assert(baseline.current.zIndex === 'auto', 'stepper introduced numeric z-index');

    await page.locator('#invalid-header').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(captureState);
    assertPaint(hovered.invalid, hovered.system.highlight, hovered.system.highlightText, 'hovered invalid header');
    assert(hovered.invalid.markerColor === hovered.system.mark, 'hover erased invalid marker paint');
    assertHeaderGeometry(hovered.invalid, baseline.invalid, 'hover changed header geometry');
    assert(hovered.semanticState === baseline.semanticState, 'hover changed workflow semantics');

    await page.locator('#disabled-header').hover({ force: true });
    await page.evaluate(nextFrame);
    const disabledHover = await page.evaluate(captureState);
    assertPaint(disabledHover.disabled, disabledHover.system.canvas, disabledHover.system.grayText, 'disabled hover');
    assertHeaderGeometry(disabledHover.disabled, baseline.disabled, 'disabled hover changed geometry');
    assert(disabledHover.semanticState === baseline.semanticState, 'disabled hover changed workflow semantics');

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(captureState);
    assert(focused.activeElement === 'current-header', 'roving tab stop did not reach the current header');
    assert(focused.current.focusVisible, 'current header did not retain :focus-visible');
    assert(focused.current.outlineColor === focused.system.canvasText, 'focus outline did not map to CanvasText');
    assert(focused.current.outlineWidth === focused.tokens.focus, 'focus stroke width changed');
    assertPaint(focused.current, focused.system.selectedItem, focused.system.selectedItemText, 'focused current header');
    assert(focused.current.markerColor === focused.system.highlight, 'focus erased completion paint');
    assertHeaderGeometry(focused.current, baseline.current, 'focus changed current geometry');

    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root: documentNode } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#current-header',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: ['active', 'hover'] });
    const active = await page.evaluate(captureState);
    assertPaint(active.current, active.system.highlight, active.system.highlightText, 'active current header');
    assert(active.current.markerColor === active.system.highlight, 'active erased completion marker');
    assertHeaderGeometry(active.current, baseline.current, 'active changed current geometry');
    assert(active.semanticState === baseline.semanticState, 'active input mutated workflow semantics');
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: [] });

    await page.locator('#pointer-away').hover();
    await page.evaluate(nextFrame);
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient input changed stepper DOM or ARIA state');
    assert(
        finalState.events.join('|') === 'pointerenter:true|pointerleave:true',
        `unexpected stepper event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components stepper forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureState() {
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
        probe.style.display = 'block';
        probe.style.inlineSize = `var(${token})`;
        document.body.append(probe);
        const value = getComputedStyle(probe).inlineSize;
        probe.remove();
        return value;
    };
    const rect = (element) => {
        const bounds = element.getBoundingClientRect();
        return [bounds.x, bounds.y, bounds.width, bounds.height];
    };
    const headerState = (selector) => {
        const header = required(selector);
        const style = getComputedStyle(header);
        const markerStyle = getComputedStyle(required(`${selector} > .cem-stepper__marker`));
        return {
            animationName: style.animationName,
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: header.matches(':focus-visible'),
            height: header.getBoundingClientRect().height,
            markerColor: markerStyle.color,
            outlineColor: style.outlineColor,
            outlineWidth: style.outlineWidth,
            rect: rect(header),
            transitionDuration: style.transitionDuration,
            zIndex: style.zIndex,
        };
    };
    const host = required('#stepper-host');
    const current = required('#current-header');
    const invalid = required('#invalid-header');
    const disabled = required('#disabled-header');
    return {
        activeElement: document.activeElement?.id ?? '',
        completedConnector: getComputedStyle(required('#completed-item'), '::after').backgroundColor,
        current: headerState('#current-header'),
        disabled: headerState('#disabled-header'),
        events: [...window.__stepperEvents],
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.outerHTML,
        invalid: headerState('#invalid-header'),
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        remainingConnector: getComputedStyle(required('#invalid-item'), '::after').backgroundColor,
        semanticState: [current, invalid, disabled]
            .map((header) => [header.tabIndex, header.disabled, header.getAttribute('aria-current'), header.getAttribute('aria-invalid'), header.getAttribute('aria-disabled')].join(':'))
            .join('|'),
        system: {
            canvas: systemColor('Canvas'),
            canvasText: systemColor('CanvasText'),
            grayText: systemColor('GrayText'),
            highlight: systemColor('Highlight'),
            highlightText: systemColor('HighlightText'),
            mark: systemColor('Mark'),
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
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function assertHeaderGeometry(actual, expected, message) {
    assert(equalRect(actual.rect, expected.rect), message);
}

function equalRect(left, right) {
    return left.length === right.length && left.every((value, index) => Math.abs(value - right[index]) <= 0.01);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
