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
            <cem-slider id="single-host" min="0" max="100" step="5" discrete show-tick-marks>
                <div class="cem-slider" data-mode="single" style="--_cem-slider-start-position: 0%; --_cem-slider-end-position: 50%; --_cem-slider-tick-spacing: 5%;">
                    <div class="cem-slider__visual" aria-hidden="true">
                        <span class="cem-slider__track"></span>
                        <span class="cem-slider__active-track"></span>
                        <span class="cem-slider__ticks"></span>
                        <span class="cem-slider__value" data-cem-slider-value="single">50 percent</span>
                        <span class="cem-slider__value" data-cem-slider-value="start"></span>
                        <span class="cem-slider__value" data-cem-slider-value="end"></span>
                    </div>
                    <div class="cem-slider__inputs">
                        <input id="single-thumb" type="range" data-cem-slider-thumb="single" min="0" max="100" step="5" value="50" aria-label="Volume">
                    </div>
                </div>
            </cem-slider>
            <cem-slider id="disabled-host" min="0" max="100" step="5" disabled>
                <div class="cem-slider" data-mode="single" style="--_cem-slider-start-position: 0%; --_cem-slider-end-position: 50%; --_cem-slider-tick-spacing: 5%;">
                    <div class="cem-slider__visual" aria-hidden="true">
                        <span class="cem-slider__track"></span>
                        <span class="cem-slider__active-track"></span>
                        <span class="cem-slider__ticks"></span>
                    </div>
                    <div class="cem-slider__inputs">
                        <input id="disabled-thumb" type="range" data-cem-slider-thumb="single" min="0" max="100" step="5" value="50" aria-label="Disabled volume" disabled>
                    </div>
                </div>
            </cem-slider>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__sliderEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'input', 'change', 'cem-slider-change']) {
            document.querySelector('#single-thumb').addEventListener(eventName, (event) => {
                window.__sliderEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.trackColor === baseline.system.grayText, 'remaining track did not use GrayText');
    assert(baseline.activeTrackColor === baseline.system.highlight, 'active track did not use Highlight');
    assert(baseline.tokens.thumbColor === baseline.system.canvasText, 'resting thumb token did not use CanvasText');
    assert(baseline.disabledTrackColor === baseline.system.grayText, 'disabled track did not use GrayText');
    assert(baseline.disabledActiveTrackColor === baseline.system.grayText, 'disabled active track did not use GrayText');
    assert(baseline.tokens.disabledThumbColor === baseline.system.grayText, 'disabled thumb token did not use GrayText');
    assert(
        baseline.tickImage.includes(baseline.system.canvasText),
        `tick marks ${baseline.tickImage} did not retain CanvasText ${baseline.system.canvasText}`,
    );
    assert(baseline.forcedColorAdjust === 'auto', 'native thumb must retain automatic forced-color adjustment');
    assertClose(baseline.inputHeight, baseline.tokens.zoneMinimum, 'native thumb target block size');
    assertClose(baseline.trackHeight, baseline.tokens.trackThickness, 'visible track thickness');

    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root: documentNode } = await cdp.send('DOM.getDocument');
    const { nodeId: thumbNodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#single-thumb',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId: thumbNodeId, forcedPseudoClasses: ['hover'] });
    const hovered = await page.evaluate(captureState);
    assert(hovered.tokens.hoverThumbColor === hovered.system.highlight, 'hovered thumb token did not use Highlight');
    assertGeometry(hovered, baseline, 'hover changed slider geometry');
    assert(hovered.hostHtml === baseline.hostHtml, 'hover changed slider DOM or state');

    await cdp.send('CSS.forcePseudoState', { nodeId: thumbNodeId, forcedPseudoClasses: ['active', 'hover'] });
    const active = await page.evaluate(captureState);
    assert(active.tokens.activeThumbColor === active.system.highlight, 'active thumb token did not use Highlight');
    assertGeometry(active, baseline, 'active changed slider geometry');
    assert(active.hostHtml === baseline.hostHtml, 'active changed slider DOM or state');
    await cdp.send('CSS.forcePseudoState', { nodeId: thumbNodeId, forcedPseudoClasses: [] });

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(captureState);
    assert(focused.focusVisible, 'keyboard focus did not remain on the native slider input');
    assert(focused.outlineColor === focused.system.canvasText, 'focus outline did not use CanvasText');
    assertClose(focused.outlineWidth, focused.tokens.focusStroke, 'focus stroke width');
    assertClose(focused.outlineOffset, focused.tokens.focusOffset, 'focus outline offset');
    assertGeometry(focused, baseline, 'focus changed slider geometry');

    const { nodeId: disabledThumbNodeId } = await cdp.send('DOM.querySelector', {
        nodeId: documentNode.nodeId,
        selector: '#disabled-thumb',
    });
    await cdp.send('CSS.forcePseudoState', { nodeId: disabledThumbNodeId, forcedPseudoClasses: ['hover'] });
    const disabledHovered = await page.evaluate(captureState);
    assert(
        disabledHovered.tokens.disabledThumbColor === disabledHovered.system.grayText,
        'disabled hover changed thumb semantics',
    );
    assertGeometry(disabledHovered, baseline, 'disabled hover changed slider geometry');
    await cdp.send('CSS.forcePseudoState', { nodeId: disabledThumbNodeId, forcedPseudoClasses: [] });

    await page.locator('#pointer-away').hover();
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient pointer/focus input changed slider DOM or state');
    assert(
        finalState.events.length === 0,
        `unexpected slider event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components slider forced-colors contract verified.');
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

    function resolveTokenColor(token) {
        const probe = document.createElement('span');
        probe.style.backgroundColor = `var(${token})`;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).backgroundColor;
        probe.remove();
        return resolved;
    }

    const host = document.querySelector('#single-host');
    const owner = host.querySelector(':scope > .cem-slider');
    const input = document.querySelector('#single-thumb');
    const disabledInput = document.querySelector('#disabled-thumb');
    const track = owner.querySelector('.cem-slider__track');
    const activeTrack = owner.querySelector('.cem-slider__active-track');
    const ticks = owner.querySelector('.cem-slider__ticks');
    const disabledOwner = document.querySelector('#disabled-host > .cem-slider');
    const inputStyle = getComputedStyle(input);
    const thumbStyle = getComputedStyle(input, '::-webkit-slider-thumb');
    const disabledThumbStyle = getComputedStyle(disabledInput, '::-webkit-slider-thumb');
    return {
        active: input.matches(':active'),
        activeTrackColor: getComputedStyle(activeTrack).backgroundColor,
        disabledActiveTrackColor: getComputedStyle(disabledOwner.querySelector('.cem-slider__active-track')).backgroundColor,
        disabledThumbColor: disabledThumbStyle.backgroundColor,
        disabledTrackColor: getComputedStyle(disabledOwner.querySelector('.cem-slider__track')).backgroundColor,
        events: [...window.__sliderEvents],
        focusVisible: input.matches(':focus-visible'),
        forcedColorAdjust: inputStyle.forcedColorAdjust,
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.outerHTML,
        hover: input.matches(':hover'),
        inputHeight: input.getBoundingClientRect().height,
        outlineColor: inputStyle.outlineColor,
        outlineOffset: pixels(inputStyle.outlineOffset),
        outlineWidth: pixels(inputStyle.outlineWidth),
        ownerHeight: owner.getBoundingClientRect().height,
        ownerWidth: owner.getBoundingClientRect().width,
        system: {
            canvasText: resolveColor('CanvasText'),
            grayText: resolveColor('GrayText'),
            highlight: resolveColor('Highlight'),
        },
        thumbColor: thumbStyle.backgroundColor,
        tickImage: getComputedStyle(ticks).backgroundImage,
        tokens: {
            activeThumbColor: resolveTokenColor('--cem-slider-thumb-active-color'),
            disabledThumbColor: resolveTokenColor('--cem-slider-disabled-thumb-color'),
            focusOffset: resolveLength('--cem-stroke-indicator-offset'),
            focusStroke: resolveLength('--cem-stroke-focus'),
            hoverThumbColor: resolveTokenColor('--cem-slider-thumb-hover-color'),
            thumbColor: resolveTokenColor('--cem-slider-thumb-color'),
            trackThickness: resolveLength('--cem-slider-track-thickness'),
            zoneMinimum: resolveLength('--cem-coupling-zone-min'),
        },
        trackColor: getComputedStyle(track).backgroundColor,
        trackHeight: track.getBoundingClientRect().height,
    };
}

function assertGeometry(actual, expected, message) {
    assertClose(actual.ownerWidth, expected.ownerWidth, `${message} (owner width)`);
    assertClose(actual.ownerHeight, expected.ownerHeight, `${message} (owner height)`);
    assertClose(actual.inputHeight, expected.inputHeight, `${message} (input height)`);
    assertClose(actual.trackHeight, expected.trackHeight, `${message} (track height)`);
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) throw new Error(`${message}: ${actual} !== ${expected}`);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
