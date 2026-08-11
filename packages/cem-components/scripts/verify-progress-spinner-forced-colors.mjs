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
    const animated = await capturePreference('no-preference');
    assert(animated.forcedColors, 'forced-colors media query did not activate');
    assert(animated.reducedMotion === false, 'no-preference context unexpectedly reduced motion');
    assertSystemContract(animated);
    assert(animated.indeterminate.animationName === 'cem-progress-spinner-cycle', 'indeterminate cycle is missing');
    assert(animated.determinate.animationName === 'none', 'determinate spinner must remain static');
    assert(
        animated.events.join('|') === 'pointerenter:true|click:true|pointerleave:true',
        `unexpected spinner event boundary: ${animated.events.join('|')}`,
    );
    assert(animated.hostHtml === animated.finalHostHtml, 'pointer/click input changed spinner DOM or ARIA');

    const reduced = await capturePreference('reduce');
    assert(reduced.forcedColors, 'forced-colors media query did not remain active with reduced motion');
    assert(reduced.reducedMotion, 'reduced-motion media query did not activate');
    assertSystemContract(reduced);
    assert(reduced.indeterminate.animationName === 'none', 'reduced motion did not stop the indeterminate cycle');
    assert(
        reduced.indeterminate.dashArray === animated.indeterminate.dashArray,
        'reduced motion changed the static indeterminate arc',
    );
    assertSpinnerGeometry(reduced.indeterminate, animated.indeterminate, 'reduced motion changed spinner geometry');
    assert(
        reduced.indeterminate.ariaValueNow === null,
        'reduced motion introduced an indeterminate aria-valuenow',
    );

    console.log('cem-components progress-spinner forced-colors and reduced-motion contract verified.');
} finally {
    await browser.close();
}

async function capturePreference(reducedMotion) {
    const context = await browser.newContext({
        forcedColors: 'active',
        javaScriptEnabled: true,
        reducedMotion,
    });
    try {
        const page = await context.newPage();
        await page.setContent(`
            <style>${themeCss}\n${componentCss}</style>
            <main class="cem-theme-light">
                <button id="focus-start" type="button">Focus start</button>
                <cem-progress-spinner id="indeterminate-host" label="Loading account">
                    <span class="cem-progress-spinner" role="progressbar" data-mode="indeterminate" aria-label="Loading account">
                        <svg class="cem-progress-spinner__svg" viewBox="0 0 100 100" aria-hidden="true" focusable="false">
                            <circle class="cem-progress-spinner__track" cx="50" cy="50" r="42" pathLength="100"></circle>
                            <circle class="cem-progress-spinner__indicator" cx="50" cy="50" r="42" pathLength="100" stroke-dasharray="25 75"></circle>
                        </svg>
                    </span>
                </cem-progress-spinner>
                <cem-progress-spinner id="determinate-host" label="Uploading files" value="25">
                    <span class="cem-progress-spinner" role="progressbar" data-mode="determinate" aria-label="Uploading files" aria-valuemin="0" aria-valuemax="100" aria-valuenow="25">
                        <svg class="cem-progress-spinner__svg" viewBox="0 0 100 100" aria-hidden="true" focusable="false">
                            <circle class="cem-progress-spinner__track" cx="50" cy="50" r="42" pathLength="100"></circle>
                            <circle class="cem-progress-spinner__indicator" cx="50" cy="50" r="42" pathLength="100" stroke-dasharray="25 75"></circle>
                        </svg>
                    </span>
                </cem-progress-spinner>
                <button id="focus-end" type="button">Focus end</button>
            </main>
        `);
        await page.evaluate(() => {
            window.__spinnerEvents = [];
            const owner = document.querySelector('#indeterminate-host > .cem-progress-spinner');
            for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-progress']) {
                owner.addEventListener(eventName, (event) => {
                    window.__spinnerEvents.push(`${event.type}:${event.isTrusted}`);
                });
            }
        });
        const hostHtml = await page.locator('#indeterminate-host').evaluate((element) => element.outerHTML);
        await page.locator('#indeterminate-host > .cem-progress-spinner').hover();
        await page.locator('#indeterminate-host > .cem-progress-spinner').click();
        await page.locator('#focus-end').hover();
        await page.locator('#focus-start').focus();
        await page.keyboard.press('Tab');
        return await page.evaluate(captureState, hostHtml);
    } finally {
        await context.close();
    }
}

function captureState(baselineHtml) {
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

    function spinnerState(hostSelector) {
        const host = document.querySelector(hostSelector);
        const owner = host.querySelector(':scope > .cem-progress-spinner');
        const svg = owner.querySelector('svg');
        const track = svg.querySelector('.cem-progress-spinner__track');
        const indicator = svg.querySelector('.cem-progress-spinner__indicator');
        const ownerRect = owner.getBoundingClientRect();
        const svgRect = svg.getBoundingClientRect();
        const ownerStyle = getComputedStyle(owner);
        const trackStyle = getComputedStyle(track);
        const indicatorStyle = getComputedStyle(indicator);
        return {
            animationName: indicatorStyle.animationName,
            ariaValueNow: owner.getAttribute('aria-valuenow'),
            dashArray: indicator.getAttribute('stroke-dasharray'),
            forcedColorAdjust: ownerStyle.forcedColorAdjust,
            height: ownerRect.height,
            indicatorColor: indicatorStyle.stroke,
            strokeWidth: pixels(indicatorStyle.strokeWidth),
            svgHeight: svgRect.height,
            svgWidth: svgRect.width,
            trackColor: trackStyle.stroke,
            trackWidth: pixels(trackStyle.strokeWidth),
            width: ownerRect.width,
        };
    }

    return {
        determinate: spinnerState('#determinate-host'),
        events: [...window.__spinnerEvents],
        finalHostHtml: document.querySelector('#indeterminate-host').outerHTML,
        focusOwner: document.activeElement?.id,
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: baselineHtml,
        indeterminate: spinnerState('#indeterminate-host'),
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        system: {
            grayText: resolveColor('GrayText'),
            highlight: resolveColor('Highlight'),
        },
        tokens: {
            size: resolveLength('--cem-progress-spinner-size'),
            thickness: resolveLength('--cem-progress-track-thickness'),
        },
    };
}

function assertSystemContract(state) {
    for (const [label, spinner] of [
        ['indeterminate', state.indeterminate],
        ['determinate', state.determinate],
    ]) {
        assert(spinner.trackColor === state.system.grayText, `${label} track did not resolve to GrayText`);
        assert(spinner.indicatorColor === state.system.highlight, `${label} indicator did not resolve to Highlight`);
        assert(spinner.forcedColorAdjust === 'auto', `${label} spinner opted out of forced color adjustment`);
        assertClose(spinner.strokeWidth, state.tokens.thickness, `${label} indicator thickness`);
        assertClose(spinner.trackWidth, state.tokens.thickness, `${label} track thickness`);
        assertClose(spinner.width, state.tokens.size, `${label} width`);
        assertClose(spinner.height, state.tokens.size, `${label} height`);
        assertClose(spinner.svgWidth, state.tokens.size, `${label} SVG width`);
        assertClose(spinner.svgHeight, state.tokens.size, `${label} SVG height`);
    }
    assert(state.focusOwner === 'focus-end', `spinner interrupted keyboard focus order: ${state.focusOwner}`);
    assert(state.indeterminate.ariaValueNow === null, 'indeterminate spinner exposed aria-valuenow');
    assert(state.determinate.ariaValueNow === '25', 'determinate spinner lost aria-valuenow');
}

function assertSpinnerGeometry(actual, expected, message) {
    for (const field of ['width', 'height', 'svgWidth', 'svgHeight', 'strokeWidth', 'trackWidth']) {
        assertClose(actual[field], expected[field], `${message} (${field})`);
    }
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) throw new Error(`${message}: ${actual} !== ${expected}`);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
