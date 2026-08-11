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
            <cem-divider id="horizontal">
                <div class="cem-divider" data-orientation="horizontal" role="separator" aria-orientation="horizontal"></div>
            </cem-divider>
            <cem-divider id="inset" spacing="block" inset>
                <div class="cem-divider" data-orientation="horizontal" role="separator" aria-orientation="horizontal"></div>
            </cem-divider>
            <div id="vertical-frame">
                <span>Before</span>
                <cem-divider id="vertical" orientation="vertical" inset>
                    <div class="cem-divider" data-orientation="vertical" role="separator" aria-orientation="vertical"></div>
                </cem-divider>
                <span>After</span>
            </div>
        </main>
    `);
    await page.locator('#vertical-frame').evaluate((element) => {
        element.style.display = 'flex';
        element.style.height = '8rem';
    });

    const state = await page.evaluate(captureState);
    assert(state.forcedColors, 'forced-colors media query did not activate');
    assert(
        state.separatorToken === state.systemCanvasText,
        `--cem-separator-color did not resolve to CanvasText: ${state.separatorToken} !== ${state.systemCanvasText}`,
    );
    assert(
        state.horizontal.color === state.systemCanvasText,
        `horizontal divider did not paint CanvasText: ${state.horizontal.color} !== ${state.systemCanvasText}`,
    );
    assert(
        state.horizontal.width === state.strokeDivider,
        `horizontal divider width did not resolve from --cem-stroke-divider: ${state.horizontal.width} !== ${state.strokeDivider}`,
    );
    assertClose(
        state.horizontal.track,
        Math.max(state.gapGroup, state.guardMinimum),
        'horizontal divider margin box did not preserve the D1/D2 track',
    );
    assert(
        state.inset.inlineStart === state.insetContainer,
        `inset divider did not resolve --cem-inset-container: ${state.inset.inlineStart} !== ${state.insetContainer}`,
    );
    assertClose(
        state.inset.track,
        Math.max(state.gapBlock, state.guardMinimum),
        'inset divider changed the cross-axis D1/D2 track',
    );
    assert(
        state.vertical.color === state.systemCanvasText,
        `vertical divider did not paint CanvasText: ${state.vertical.color} !== ${state.systemCanvasText}`,
    );
    assert(
        state.vertical.width === state.strokeDivider,
        `vertical divider width did not resolve from --cem-stroke-divider: ${state.vertical.width} !== ${state.strokeDivider}`,
    );
    assertClose(
        state.vertical.track,
        Math.max(state.gapGroup, state.guardMinimum),
        'vertical divider margin box did not preserve the D1/D2 track',
    );
    assert(state.vertical.extent > 0, 'vertical divider did not stretch along its line axis');

    console.log('cem-components divider forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureState() {
    function pixels(value) {
        const parsed = Number.parseFloat(value);
        if (!Number.isFinite(parsed)) {
            throw new Error(`Expected a resolved CSS length, received ${value}`);
        }
        return parsed;
    }

    function resolveColor(owner, value) {
        const probe = document.createElement('span');
        probe.style.color = value;
        owner.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    }

    function resolveLength(owner, token) {
        const probe = document.createElement('span');
        probe.style.display = 'block';
        probe.style.inlineSize = `var(${token})`;
        owner.append(probe);
        const resolved = getComputedStyle(probe).inlineSize;
        probe.remove();
        return resolved;
    }

    function horizontalState(host, line) {
        const hostStyle = getComputedStyle(host);
        const lineStyle = getComputedStyle(line);
        return {
            color: lineStyle.borderBlockStartColor,
            track:
                pixels(hostStyle.marginBlockStart) +
                pixels(lineStyle.borderBlockStartWidth) +
                pixels(hostStyle.marginBlockEnd),
            width: lineStyle.borderBlockStartWidth,
        };
    }

    function verticalState(host, line) {
        const hostStyle = getComputedStyle(host);
        const lineStyle = getComputedStyle(line);
        return {
            color: lineStyle.borderInlineStartColor,
            extent: line.getBoundingClientRect().height,
            track:
                pixels(hostStyle.marginInlineStart) +
                pixels(lineStyle.borderInlineStartWidth) +
                pixels(hostStyle.marginInlineEnd),
            width: lineStyle.borderInlineStartWidth,
        };
    }

    const horizontalHost = document.querySelector('#horizontal');
    const horizontalLine = document.querySelector('#horizontal > .cem-divider');
    const insetHost = document.querySelector('#inset');
    const insetLine = document.querySelector('#inset > .cem-divider');
    const verticalHost = document.querySelector('#vertical');
    const verticalLine = document.querySelector('#vertical > .cem-divider');
    const root = document.documentElement;

    return {
        forcedColors: matchMedia('(forced-colors: active)').matches,
        systemCanvasText: resolveColor(root, 'CanvasText'),
        separatorToken: resolveColor(root, 'var(--cem-separator-color)'),
        strokeDivider: resolveLength(root, '--cem-stroke-divider'),
        guardMinimum: pixels(resolveLength(root, '--cem-coupling-guard-min')),
        gapGroup: pixels(resolveLength(root, '--cem-gap-group')),
        gapBlock: pixels(resolveLength(root, '--cem-gap-block')),
        insetContainer: resolveLength(root, '--cem-inset-container'),
        horizontal: horizontalState(horizontalHost, horizontalLine),
        inset: {
            ...horizontalState(insetHost, insetLine),
            inlineStart: getComputedStyle(insetLine).marginInlineStart,
        },
        vertical: verticalState(verticalHost, verticalLine),
    };
}

function assertClose(actual, expected, message) {
    if (Math.abs(actual - expected) > 0.01) {
        throw new Error(`${message}: ${actual} !== ${expected}`);
    }
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
