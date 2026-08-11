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
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <cem-tooltip id="tooltip-host">
                <span class="cem-tooltip" data-mode="valid" data-position="above">
                    <button id="tooltip-trigger" slot="trigger" type="button" aria-describedby="tooltip-description">Save</button>
                    <span id="tooltip-description" class="cem-tooltip__description">Save the current document</span>
                    <span id="tooltip-surface" class="cem-tooltip__surface" role="tooltip" popover="manual">Save the current document</span>
                </span>
            </cem-tooltip>
        </main>
    `);
    await page.evaluate(() => {
        window.__tooltipEvents = [];
        const trigger = document.querySelector('#tooltip-trigger');
        for (const eventName of ['click', 'input', 'change', 'cem-tooltip-toggle']) {
            trigger.addEventListener(eventName, (event) => {
                window.__tooltipEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#tooltip-trigger').focus();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assert(!baseline.open, 'tooltip surface must begin closed');

    await page.evaluate(() => document.querySelector('#tooltip-surface').showPopover());
    const open = await page.evaluate(captureState);
    assert(open.open, 'tooltip surface did not enter the top layer');
    assert(open.backgroundColor === open.system.canvas, 'tooltip surface did not use Canvas');
    assert(open.color === open.system.canvasText, 'tooltip text did not use CanvasText');
    assert(open.borderColor === open.system.canvasText, 'tooltip contour did not use CanvasText');
    assert(open.forcedColorAdjust === 'auto', 'tooltip must retain automatic forced-color adjustment');
    assert(open.boxShadow === 'none', 'forced colors must not depend on tooltip shadow');
    assert(open.animationName === 'none', 'tooltip must not add component animation');
    assert(open.transitionDuration === '0s', 'tooltip must not add a component transition');
    assert(open.zIndex === 'auto', 'native top-layer tooltip must not declare z-index');
    assert(open.positionAnchor === '--_cem-tooltip-anchor', 'tooltip did not retain its CSS anchor');
    assert(open.positionArea === 'block-end', 'top-edge tooltip did not use its opposite-side fallback');
    assert(open.surfaceTop >= open.triggerBottom, 'fallback tooltip overlapped its trigger');
    assert(open.activeElement === 'tooltip-trigger', 'showing the tooltip moved focus');
    assert(open.focusableSurfaceDescendants === 0, 'tooltip surface acquired interactive descendants');
    assert(open.pointerEvents === 'auto', 'tooltip surface cannot retain pointer hover continuity');
    assert(open.descriptionClipped, 'persistent description is not visually hidden');
    assertGeometry(open, baseline, 'show changed trigger geometry');

    await page.evaluate(() => document.querySelector('#tooltip-surface').hidePopover());
    const closed = await page.evaluate(captureState);
    assert(!closed.open, 'tooltip surface did not close');
    assert(closed.hostHtml === baseline.hostHtml, 'transient top-layer state mutated tooltip DOM attributes');
    assert(closed.events.length === 0, `unexpected tooltip activation events: ${closed.events.join('|')}`);
    assertGeometry(closed, baseline, 'hide changed trigger geometry');

    console.log('cem-components tooltip forced-colors/top-layer contract verified.');
} finally {
    await browser.close();
}

function captureState() {
    function resolveColor(value) {
        const probe = document.createElement('span');
        probe.style.color = value;
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    }

    const host = document.querySelector('#tooltip-host');
    const trigger = document.querySelector('#tooltip-trigger');
    const description = document.querySelector('#tooltip-description');
    const surface = document.querySelector('#tooltip-surface');
    const triggerRect = trigger.getBoundingClientRect();
    const surfaceRect = surface.getBoundingClientRect();
    const descriptionStyle = getComputedStyle(description);
    const surfaceStyle = getComputedStyle(surface);
    return {
        activeElement: document.activeElement?.id ?? '',
        animationName: surfaceStyle.animationName,
        backgroundColor: surfaceStyle.backgroundColor,
        borderColor: surfaceStyle.borderColor,
        boxShadow: surfaceStyle.boxShadow,
        color: surfaceStyle.color,
        descriptionClipped: descriptionStyle.clip !== 'auto' && descriptionStyle.overflow === 'hidden',
        events: [...window.__tooltipEvents],
        focusableSurfaceDescendants: surface.querySelectorAll('a, button, input, select, textarea, [tabindex]').length,
        forcedColorAdjust: surfaceStyle.forcedColorAdjust,
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.outerHTML,
        open: surface.matches(':popover-open'),
        positionAnchor: surfaceStyle.positionAnchor,
        positionArea: surfaceStyle.positionArea,
        pointerEvents: surfaceStyle.pointerEvents,
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        surfaceTop: surfaceRect.top,
        system: {
            canvas: resolveColor('Canvas'),
            canvasText: resolveColor('CanvasText'),
        },
        transitionDuration: surfaceStyle.transitionDuration,
        triggerBottom: triggerRect.bottom,
        triggerHeight: triggerRect.height,
        triggerWidth: triggerRect.width,
        triggerX: triggerRect.x,
        triggerY: triggerRect.y,
        zIndex: surfaceStyle.zIndex,
    };
}

function assertGeometry(actual, expected, message) {
    assertClose(actual.triggerWidth, expected.triggerWidth, `${message} (width)`);
    assertClose(actual.triggerHeight, expected.triggerHeight, `${message} (height)`);
    assertClose(actual.triggerX, expected.triggerX, `${message} (x)`);
    assertClose(actual.triggerY, expected.triggerY, `${message} (y)`);
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) throw new Error(`${message}: ${actual} !== ${expected}`);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
