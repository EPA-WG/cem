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
            <cem-tree id="tree-host" expanded-values="branch loading" selection="single" selected-values="selected">
                <div id="tree-owner" class="cem-tree" role="tree" aria-label="Resources">
                    <div id="branch-wrapper" class="cem-tree__item" role="none">
                        <button id="branch-node" type="button" class="cem-tree__node" role="treeitem" data-tree-value="branch" data-expanded="true" data-selected="false" data-loading="false" tabindex="-1" aria-label="Branch" aria-level="1" aria-posinset="1" aria-setsize="4" aria-expanded="true" aria-selected="false" aria-owns="branch-group">
                            <span class="cem-tree__marker" aria-hidden="true">▾</span>
                            <span class="cem-tree__label">Branch</span>
                            <span class="cem-tree__status" hidden>Loading</span>
                        </button>
                        <div id="branch-group" class="cem-tree__group" role="group">
                            <div class="cem-tree__item" role="none">
                                <button id="child-node" type="button" class="cem-tree__node" role="treeitem" data-tree-value="child" data-expanded="false" data-selected="false" data-loading="false" tabindex="-1" aria-label="Child" aria-level="2" aria-posinset="1" aria-setsize="1" aria-selected="false">
                                    <span class="cem-tree__marker" aria-hidden="true"></span>
                                    <span class="cem-tree__label">Child</span>
                                    <span class="cem-tree__status" hidden>Loading</span>
                                </button>
                                <div class="cem-tree__group" hidden></div>
                            </div>
                        </div>
                    </div>
                    <div id="selected-wrapper" class="cem-tree__item" role="none">
                        <button id="selected-node" type="button" class="cem-tree__node" role="treeitem" data-tree-value="selected" data-expanded="false" data-selected="true" data-loading="false" tabindex="0" aria-label="Selected" aria-level="1" aria-posinset="2" aria-setsize="4" aria-selected="true">
                            <span class="cem-tree__marker" aria-hidden="true"></span>
                            <span class="cem-tree__label">Selected</span>
                            <span class="cem-tree__selection" aria-hidden="true">✓</span>
                            <span class="cem-tree__status" hidden>Loading</span>
                        </button>
                        <div class="cem-tree__group" hidden></div>
                    </div>
                    <div class="cem-tree__item" role="none">
                        <button id="loading-node" type="button" class="cem-tree__node" role="treeitem" data-tree-value="loading" data-expanded="true" data-selected="false" data-loading="true" tabindex="-1" aria-label="Loading branch" aria-level="1" aria-posinset="3" aria-setsize="4" aria-expanded="true" aria-selected="false" aria-busy="true" aria-owns="loading-group">
                            <span class="cem-tree__marker" aria-hidden="true">▾</span>
                            <span class="cem-tree__label">Loading branch</span>
                            <span class="cem-tree__status">Loading children</span>
                        </button>
                        <div id="loading-group" class="cem-tree__group" role="group"></div>
                    </div>
                    <div class="cem-tree__item" role="none">
                        <button id="disabled-node" type="button" class="cem-tree__node" role="treeitem" data-tree-value="disabled" data-expanded="false" data-selected="true" data-loading="false" tabindex="-1" aria-label="Disabled" aria-level="1" aria-posinset="4" aria-setsize="4" aria-selected="true" aria-disabled="true">
                            <span class="cem-tree__marker" aria-hidden="true"></span>
                            <span class="cem-tree__label">Disabled</span>
                            <span class="cem-tree__selection" aria-hidden="true">✓</span>
                            <span class="cem-tree__status" hidden>Loading</span>
                        </button>
                        <div class="cem-tree__group" hidden></div>
                    </div>
                </div>
            </cem-tree>
            <button id="pointer-away" type="button">Pointer away</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__treeEvents = [];
        for (const eventName of ['pointerenter', 'pointerleave', 'click', 'input', 'change', 'cem-tree-toggle', 'cem-tree-activate']) {
            document.querySelector('#selected-node').addEventListener(eventName, (event) => {
                window.__treeEvents.push(`${event.type}:${event.isTrusted}`);
            });
        }
    });

    await page.locator('#pointer-away').hover();
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assertPaint(baseline.branch, baseline.system.buttonFace, baseline.system.buttonText, 'default branch');
    assertPaint(baseline.selected, baseline.system.selectedItem, baseline.system.selectedItemText, 'selected node');
    assertPaint(baseline.disabled, baseline.system.buttonFace, baseline.system.grayText, 'disabled selected node');
    assert(baseline.root.backgroundColor === baseline.system.canvas, 'tree root did not map to Canvas');
    assert(baseline.root.color === baseline.system.canvasText, 'tree root text did not map to CanvasText');
    assert(baseline.branch.marker === '▾', 'expanded marker was not visibly distinct');
    assert(baseline.selected.selection === '✓', 'selected marker was not visibly distinct');
    assert(baseline.loading.status === 'Loading children', 'loading status was not visible');
    assert(baseline.loading.ariaBusy === 'true', 'loading status lost aria-busy');
    assert(baseline.branch.height >= baseline.tokens.zoneMinimum, 'treeitem fell below the D2 target minimum');
    assert(baseline.branch.animationName === 'none', 'treeitem introduced component animation');
    assert(baseline.branch.transitionDuration === '0s', 'treeitem introduced component transition');
    assert(baseline.branch.zIndex === 'auto', 'tree introduced numeric z-index');
    assert(
        /,\s*0\)$/u.test(baseline.wrapperBackground),
        `structural wrapper received state paint: ${baseline.wrapperBackground}`,
    );

    await page.locator('#branch-node').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(captureState);
    assertPaint(hovered.branch, hovered.system.highlight, hovered.system.highlightText, 'hovered branch');
    assertNodeGeometry(hovered.branch, baseline.branch, 'hover changed treeitem geometry');
    assert(hovered.semanticState === baseline.semanticState, 'hover changed tree semantics');

    await forceState(cdp, '#branch-node', ['active', 'hover']);
    const active = await page.evaluate(captureState);
    assertPaint(active.branch, active.system.highlight, active.system.highlightText, 'active branch');
    assertNodeGeometry(active.branch, baseline.branch, 'active changed treeitem geometry');
    assert(active.semanticState === baseline.semanticState, 'active changed tree semantics');
    await forceState(cdp, '#branch-node', []);

    await page.locator('#selected-node').hover();
    await page.evaluate(nextFrame);
    const selectedHover = await page.evaluate(captureState);
    assertPaint(selectedHover.selected, selectedHover.system.highlight, selectedHover.system.highlightText, 'selected hover');
    assert(selectedHover.selected.selection === '✓', 'selected hover erased the selection marker');
    assertNodeGeometry(selectedHover.selected, baseline.selected, 'selected hover changed geometry');

    await forceState(cdp, '#selected-node', ['active', 'hover']);
    const selectedActive = await page.evaluate(captureState);
    assertPaint(selectedActive.selected, selectedActive.system.highlight, selectedActive.system.highlightText, 'selected active');
    assert(selectedActive.selected.selection === '✓', 'selected active erased the selection marker');
    assertNodeGeometry(selectedActive.selected, baseline.selected, 'selected active changed geometry');
    assert(selectedActive.semanticState === baseline.semanticState, 'selected active changed tree semantics');
    await forceState(cdp, '#selected-node', []);

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const focused = await page.evaluate(captureState);
    assert(focused.activeElement === 'selected-node', 'roving tab stop did not reach the selected treeitem');
    assert(focused.selected.focusVisible, 'selected treeitem did not retain :focus-visible');
    assert(focused.selected.outlineColor === focused.system.canvasText, 'focus outline did not map to CanvasText');
    assert(focused.selected.outlineWidth === focused.tokens.focus, 'focus stroke width changed');
    assertPaint(focused.selected, focused.system.highlight, focused.system.highlightText, 'focused selected hover');
    assertNodeGeometry(focused.selected, baseline.selected, 'focus changed selected geometry');

    await page.locator('#disabled-node').hover({ force: true });
    await page.evaluate(nextFrame);
    const disabledHover = await page.evaluate(captureState);
    assertPaint(disabledHover.disabled, disabledHover.system.buttonFace, disabledHover.system.grayText, 'disabled hover');
    assertNodeGeometry(disabledHover.disabled, baseline.disabled, 'disabled hover changed geometry');
    assert(disabledHover.semanticState === baseline.semanticState, 'disabled hover changed tree semantics');

    await page.locator('#pointer-away').hover();
    await page.evaluate(nextFrame);
    const finalState = await page.evaluate(captureState);
    assert(finalState.hostHtml === baseline.hostHtml, 'transient input changed tree DOM or ARIA state');
    assert(
        finalState.events.join('|') === 'pointerenter:true|pointerleave:true',
        `unexpected tree event boundary: ${finalState.events.join('|')}`,
    );

    console.log('cem-components tree forced-colors contract verified.');
} finally {
    await browser.close();
}

async function forceState(cdp, selector, forcedPseudoClasses) {
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector });
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses });
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
    const nodeState = (selector) => {
        const node = required(selector);
        const style = getComputedStyle(node);
        return {
            animationName: style.animationName,
            ariaBusy: node.getAttribute('aria-busy'),
            backgroundColor: style.backgroundColor,
            color: style.color,
            focusVisible: node.matches(':focus-visible'),
            height: node.getBoundingClientRect().height,
            marker: node.querySelector('.cem-tree__marker')?.textContent?.trim() ?? '',
            outlineColor: style.outlineColor,
            outlineWidth: style.outlineWidth,
            rect: rect(node),
            selection: node.querySelector('.cem-tree__selection')?.textContent?.trim() ?? '',
            status: node.querySelector('.cem-tree__status:not([hidden])')?.textContent?.trim() ?? '',
            transitionDuration: style.transitionDuration,
            zIndex: style.zIndex,
        };
    };
    const host = required('#tree-host');
    const tracked = ['#branch-node', '#selected-node', '#loading-node', '#disabled-node'].map(required);
    const root = required('#tree-owner');
    const rootStyle = getComputedStyle(root);
    return {
        activeElement: document.activeElement?.id ?? '',
        branch: nodeState('#branch-node'),
        disabled: nodeState('#disabled-node'),
        events: [...window.__treeEvents],
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.outerHTML,
        loading: nodeState('#loading-node'),
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        root: { backgroundColor: rootStyle.backgroundColor, color: rootStyle.color },
        selected: nodeState('#selected-node'),
        semanticState: tracked.map((node) => [
            node.tabIndex,
            node.getAttribute('aria-expanded'),
            node.getAttribute('aria-selected'),
            node.getAttribute('aria-disabled'),
            node.getAttribute('aria-busy'),
            node.getAttribute('aria-owns'),
        ].join(':')).join('|'),
        system: {
            buttonFace: systemColor('ButtonFace'),
            buttonText: systemColor('ButtonText'),
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
        wrapperBackground: getComputedStyle(required('#selected-wrapper')).backgroundColor,
    };
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function assertPaint(actual, background, color, label) {
    assert(actual.backgroundColor === background, `${label} background ${actual.backgroundColor} !== ${background}`);
    assert(actual.color === color, `${label} text ${actual.color} !== ${color}`);
}

function assertNodeGeometry(actual, expected, message) {
    assert(equalRect(actual.rect, expected.rect), message);
}

function equalRect(left, right) {
    return left.length === right.length && left.every((value, index) => Math.abs(value - right[index]) <= 0.01);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
