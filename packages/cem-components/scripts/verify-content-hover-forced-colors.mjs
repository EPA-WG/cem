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
    await cdp.send('DOM.enable');
    await cdp.send('CSS.enable');
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <main class="cem-theme-light">
            <button id="focus-start" type="button">Start</button>
            <cem-list id="list-host" selectable>
                <select id="list-owner" class="cem-list cem-list--selectable" aria-label="Asset type" size="3">
                    <option value="image" aria-selected="false">Image</option>
                    <option value="document" selected aria-selected="true">Document</option>
                    <option value="archive" disabled aria-selected="false">Archive</option>
                </select>
            </cem-list>
            <cem-chip id="unchecked-host" checkable>
                <button id="unchecked-owner" type="button" class="cem-chip" aria-pressed="false">Unchecked</button>
            </cem-chip>
            <cem-chip id="checked-host" checkable checked>
                <button id="checked-owner" type="button" class="cem-chip" aria-pressed="true">Checked</button>
            </cem-chip>
            <cem-list id="disabled-list-host" selectable>
                <select
                    id="disabled-list-owner"
                    class="cem-list cem-list--selectable"
                    aria-label="Unavailable asset type"
                    size="2"
                    disabled
                >
                    <option value="image" selected aria-selected="true">Image</option>
                    <option value="document" aria-selected="false">Document</option>
                </select>
            </cem-list>
            <cem-chip id="disabled-chip-host" checkable checked>
                <button id="disabled-chip-owner" type="button" class="cem-chip" aria-pressed="true" disabled>
                    Unavailable
                </button>
            </cem-chip>
            <cem-list id="passive-list-host">
                <ul id="passive-list-owner" class="cem-list" aria-label="Static topics"><li>Static topic</li></ul>
            </cem-list>
            <cem-chip id="passive-chip-host">
                <span id="passive-chip-owner" class="cem-chip">Passive chip</span>
            </cem-chip>
            <cem-table id="passive-table-host">
                <div id="passive-table-owner" class="cem-table" role="table" aria-label="Static comparison">
                    <div role="row"><span role="cell">Static cell</span></div>
                </div>
            </cem-table>
            <button id="focus-end" type="button">End</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__contentPointerEvents = {};
        window.__contentMutationEvents = [];
        for (const id of [
            'list-owner',
            'unchecked-owner',
            'checked-owner',
            'disabled-list-owner',
            'disabled-chip-owner',
            'passive-list-owner',
            'passive-chip-owner',
            'passive-table-owner',
        ]) {
            const element = document.getElementById(id);
            window.__contentPointerEvents[id] = [];
            for (const eventName of ['pointerenter', 'pointerleave']) {
                element.addEventListener(eventName, (event) => {
                    window.__contentPointerEvents[id].push(`${eventName}:${event.isTrusted}`);
                });
            }
        }
        for (const eventName of ['click', 'input', 'change']) {
            document.addEventListener(eventName, () => window.__contentMutationEvents.push(eventName));
        }
    });

    const baseline = await page.evaluate(captureContentForcedColorState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    for (const key of ['listOwner', 'uncheckedOwner']) {
        assert(baseline.owners[key].backgroundColor === baseline.system.canvas, `${key} did not map to Canvas`);
        assert(baseline.owners[key].color === baseline.system.canvasText, `${key} did not map to CanvasText`);
    }
    assert(
        baseline.owners.checkedOwner.backgroundColor === baseline.system.selectedItem,
        'checked chip did not map to SelectedItem',
    );
    assert(
        baseline.owners.checkedOwner.color === baseline.system.selectedItemText,
        'checked chip text did not map to SelectedItemText',
    );
    for (const key of ['disabledListOwner', 'disabledChipOwner']) {
        assert(baseline.owners[key].backgroundColor === baseline.system.canvas, `${key} did not map to Canvas`);
        assert(baseline.owners[key].color === baseline.system.grayText, `${key} did not map to GrayText`);
    }

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    await page.evaluate(nextFrame);
    const focused = await page.evaluate(captureContentForcedColorState);
    assert(focused.activeElement === 'list-owner', 'selectable list composite did not receive keyboard focus');
    assert(focused.owners.listOwner.focusVisible, 'selectable list composite did not match :focus-visible');
    assert(focused.owners.listOwner.outlineStyle !== 'none', 'selectable list composite lost visible focus');

    for (const id of ['list-owner', 'unchecked-owner', 'checked-owner']) {
        const key = camelCase(id);
        const before = await page.evaluate(captureContentForcedColorState);
        await hoverOwner(page, id);
        await forcePseudoState(cdp, id, ['hover']);
        await page.evaluate(nextFrame);
        const hovered = await page.evaluate(captureContentForcedColorState);
        if (id === 'list-owner') {
            assert(
                hovered.owners[key].backgroundColor === hovered.system.canvas,
                `native listbox hover did not retain Canvas fill`,
            );
            assert(hovered.owners[key].color === hovered.system.canvasText, `native listbox hover lost CanvasText`);
            assert(
                hovered.owners[key].borderColor === hovered.system.highlight,
                `native listbox hover border did not map to Highlight: ${hovered.owners[key].borderColor} !== ${hovered.system.highlight}`,
            );
        } else {
            assert(
                hovered.owners[key].backgroundColor === hovered.system.highlight,
                `${id} hover did not map to Highlight: ${hovered.owners[key].backgroundColor} !== ${hovered.system.highlight}`,
            );
            assert(
                hovered.owners[key].color === hovered.system.highlightText,
                `${id} hover text did not map to HighlightText: ${hovered.owners[key].color} !== ${hovered.system.highlightText}`,
            );
        }
        assert(equalSize(hovered.owners[key].size, before.owners[key].size), `${id} hover changed owner geometry`);
        assert(equalSize(hovered.hosts[key].size, before.hosts[key].size), `${id} hover changed host geometry`);
        assert(hovered.hosts[key].html === before.hosts[key].html, `${id} hover changed DOM or ARIA`);
        assert(
            hovered.hosts[key].backgroundColor === before.hosts[key].backgroundColor,
            `${id} hover styled its host wrapper`,
        );
        assert(hovered.owners[key].semanticState === before.owners[key].semanticState, `${id} hover changed state`);
        if (id === 'list-owner') {
            assert(hovered.owners[key].focusVisible, 'hover removed :focus-visible from the selectable list');
            assert(equalOutline(hovered.owners[key], focused.owners.listOwner), 'hover changed the list focus treatment');
        }

        await forcePseudoState(cdp, id, []);
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
        const restored = await page.evaluate(captureContentForcedColorState);
        assert(restored.owners[key].backgroundColor === before.owners[key].backgroundColor, `${id} fill did not restore`);
        assert(restored.owners[key].color === before.owners[key].color, `${id} text did not restore`);
    }

    for (const id of ['disabled-list-owner', 'disabled-chip-owner']) {
        const key = camelCase(id);
        const before = await page.evaluate(captureContentForcedColorState);
        await hoverOwner(page, id);
        await forcePseudoState(cdp, id, ['hover']);
        await page.evaluate(nextFrame);
        const hovered = await page.evaluate(captureContentForcedColorState);
        assert(hovered.owners[key].backgroundColor === before.owners[key].backgroundColor, `${id} acquired hover fill`);
        assert(hovered.owners[key].color === before.owners[key].color, `${id} acquired hover text`);
        assert(equalSize(hovered.owners[key].size, before.owners[key].size), `${id} hover changed geometry`);
        assert(hovered.owners[key].semanticState === before.owners[key].semanticState, `${id} hover changed state`);
        await forcePseudoState(cdp, id, []);
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
    }

    for (const id of ['passive-list-owner', 'passive-chip-owner', 'passive-table-owner']) {
        const key = camelCase(id);
        const before = await page.evaluate(captureContentForcedColorState);
        await hoverOwner(page, id);
        await page.evaluate(nextFrame);
        const hovered = await page.evaluate(captureContentForcedColorState);
        assert(hovered.owners[key].backgroundColor === before.owners[key].backgroundColor, `${id} acquired hover fill`);
        assert(hovered.owners[key].color === before.owners[key].color, `${id} acquired hover text`);
        assert(equalSize(hovered.owners[key].size, before.owners[key].size), `${id} hover changed geometry`);
        assert(hovered.hosts[key].html === before.hosts[key].html, `${id} hover changed DOM or ARIA`);
        await page.mouse.move(0, 0);
        await page.evaluate(nextFrame);
    }

    await page.locator('#focus-start').focus();
    await page.evaluate(nextFrame);
    const focusBaseline = await page.evaluate(captureContentForcedColorState);
    const focusIds = ['list-owner', 'unchecked-owner', 'checked-owner'];
    for (const [index, id] of focusIds.entries()) {
        const key = camelCase(id);
        await page.keyboard.press('Tab');
        await page.evaluate(nextFrame);
        const focusedOwner = await page.evaluate(captureContentForcedColorState);
        assert(focusedOwner.activeElement === id, `${id} did not receive keyboard focus`);
        assert(focusedOwner.owners[key].focusVisible, `${id} did not match :focus-visible`);
        assert(
            focusedOwner.owners[key].outlineColor === focusedOwner.system.canvasText,
            `${id} focus outline did not map to CanvasText`,
        );
        assert(focusedOwner.owners[key].outlineStyle === 'solid', `${id} focus outline was not solid`);
        assert(
            focusedOwner.owners[key].outlineWidth === focusedOwner.tokens.focusWidth,
            `${id} focus width did not resolve from --cem-stroke-focus`,
        );
        assert(
            focusedOwner.owners[key].outlineOffset === focusedOwner.tokens.focusOffset,
            `${id} focus offset did not resolve from --cem-stroke-indicator-offset`,
        );
        assert(
            equalSize(focusedOwner.owners[key].size, focusBaseline.owners[key].size),
            `${id} focus changed owner geometry`,
        );
        assert(
            equalSize(focusedOwner.hosts[key].size, focusBaseline.hosts[key].size),
            `${id} focus changed host geometry`,
        );
        assert(focusedOwner.hosts[key].html === focusBaseline.hosts[key].html, `${id} focus changed DOM or ARIA`);
        assert(
            equalOutline(focusedOwner.hosts[key], focusBaseline.hosts[key]),
            `${id} focus styled its host wrapper`,
        );
        assert(
            focusedOwner.owners[key].semanticState === focusBaseline.owners[key].semanticState,
            `${id} focus changed selection or checked state`,
        );

        if (index > 0) {
            const previousId = focusIds[index - 1];
            const previousKey = camelCase(previousId);
            assert(!focusedOwner.owners[previousKey].focusVisible, `${previousId} retained :focus-visible after Tab`);
            assert(
                equalOutline(focusedOwner.owners[previousKey], focusBaseline.owners[previousKey]),
                `${previousId} focus outline did not restore`,
            );
        }

        await forcePseudoState(cdp, id, ['hover']);
        await page.evaluate(nextFrame);
        const hoveredFocused = await page.evaluate(captureContentForcedColorState);
        assert(hoveredFocused.owners[key].focusVisible, `${id} hover removed :focus-visible`);
        assert(
            equalOutline(hoveredFocused.owners[key], focusedOwner.owners[key]),
            `${id} hover replaced its focus outline`,
        );
        assert(
            hoveredFocused.owners[key].semanticState === focusedOwner.owners[key].semanticState,
            `${id} focused hover changed selection or checked state`,
        );
        if (id === 'list-owner') {
            assert(
                hoveredFocused.owners[key].borderColor === hoveredFocused.system.highlight,
                'focused listbox hover border did not map to Highlight',
            );
        } else {
            assert(
                hoveredFocused.owners[key].backgroundColor === hoveredFocused.system.highlight,
                `${id} focused hover did not map to Highlight`,
            );
            assert(
                hoveredFocused.owners[key].color === hoveredFocused.system.highlightText,
                `${id} focused hover text did not map to HighlightText`,
            );
        }
        await forcePseudoState(cdp, id, []);
        await page.evaluate(nextFrame);

        assert(focusedOwner.activeElement !== 'disabled-list-owner', 'disabled listbox entered the focus order');
        assert(focusedOwner.activeElement !== 'disabled-chip-owner', 'disabled chip entered the focus order');
    }

    await page.keyboard.press('Tab');
    await page.evaluate(nextFrame);
    const restoredFocus = await page.evaluate(captureContentForcedColorState);
    assert(restoredFocus.activeElement === 'focus-end', 'focus did not leave content at the end sentinel');
    assert(!restoredFocus.owners.checkedOwner.focusVisible, 'checked chip retained :focus-visible after Tab');
    assert(
        equalOutline(restoredFocus.owners.checkedOwner, focusBaseline.owners.checkedOwner),
        'checked chip focus outline did not restore',
    );
    for (const key of ['disabledListOwner', 'disabledChipOwner', 'passiveListOwner', 'passiveChipOwner', 'passiveTableOwner']) {
        assert(!restoredFocus.owners[key].focusVisible, `${key} acquired focus-visible`);
    }

    const finalState = await page.evaluate(captureContentForcedColorState);
    for (const id of [
        'list-owner',
        'unchecked-owner',
        'checked-owner',
        'disabled-list-owner',
        'disabled-chip-owner',
        'passive-list-owner',
        'passive-chip-owner',
        'passive-table-owner',
    ]) {
        assert(
            finalState.pointerEvents[id].join('|') === 'pointerenter:true|pointerleave:true',
            `${id} did not receive one trusted pointer enter/leave pair`,
        );
    }
    assert(finalState.mutationEvents.length === 0, 'content hover/focus dispatched a mutation event');
    assert(finalState.owners.listOwner.semanticState === baseline.owners.listOwner.semanticState, 'selection mutated');
    assert(finalState.owners.checkedOwner.semanticState === baseline.owners.checkedOwner.semanticState, 'checked state mutated');
    assert(
        finalState.owners.uncheckedOwner.semanticState === baseline.owners.uncheckedOwner.semanticState,
        'unchecked state mutated',
    );

    await context.close();
    console.log('cem-components content hover/focus forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureContentForcedColorState() {
    const readSystemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const readOwner = (id) => {
        const element = document.getElementById(id);
        const styles = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        const semanticState =
            element instanceof HTMLSelectElement
                ? JSON.stringify({
                      disabled: element.disabled,
                      options: Array.from(element.options, (option) => ({
                          ariaSelected: option.getAttribute('aria-selected'),
                          disabled: option.disabled,
                          selected: option.selected,
                          value: option.value,
                      })),
                      value: element.value,
                  })
                : JSON.stringify({
                      ariaPressed: element.getAttribute('aria-pressed'),
                      disabled: element instanceof HTMLButtonElement ? element.disabled : null,
                  });
        return {
            backgroundColor: styles.backgroundColor,
            borderColor: styles.borderColor,
            color: styles.color,
            focusVisible: element.matches(':focus-visible'),
            outlineColor: styles.outlineColor,
            outlineOffset: styles.outlineOffset,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
            semanticState,
            size: [rect.width, rect.height],
        };
    };
    const readHost = (id) => {
        const element = document.getElementById(id);
        const styles = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return {
            backgroundColor: styles.backgroundColor,
            html: element.outerHTML,
            outlineColor: styles.outlineColor,
            outlineOffset: styles.outlineOffset,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
            size: [rect.width, rect.height],
        };
    };
    const ownerIds = {
        checkedOwner: 'checked-owner',
        disabledChipOwner: 'disabled-chip-owner',
        disabledListOwner: 'disabled-list-owner',
        listOwner: 'list-owner',
        passiveChipOwner: 'passive-chip-owner',
        passiveListOwner: 'passive-list-owner',
        passiveTableOwner: 'passive-table-owner',
        uncheckedOwner: 'unchecked-owner',
    };
    const hostIds = Object.fromEntries(
        Object.entries(ownerIds).map(([key, id]) => [key, id.replace('-owner', '-host')]),
    );
    const readFocusTokens = () => {
        const probe = document.createElement('span');
        probe.style.display = 'block';
        probe.style.height = 'var(--cem-stroke-indicator-offset)';
        probe.style.width = 'var(--cem-stroke-focus)';
        document.querySelector('main').append(probe);
        const styles = getComputedStyle(probe);
        const tokens = { focusOffset: styles.height, focusWidth: styles.width };
        probe.remove();
        return tokens;
    };

    return {
        activeElement: document.activeElement?.id ?? '',
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hosts: Object.fromEntries(Object.entries(hostIds).map(([key, id]) => [key, readHost(id)])),
        mutationEvents: [...window.__contentMutationEvents],
        owners: Object.fromEntries(Object.entries(ownerIds).map(([key, id]) => [key, readOwner(id)])),
        pointerEvents: structuredClone(window.__contentPointerEvents),
        system: {
            canvas: readSystemColor('Canvas'),
            canvasText: readSystemColor('CanvasText'),
            grayText: readSystemColor('GrayText'),
            highlight: readSystemColor('Highlight'),
            highlightText: readSystemColor('HighlightText'),
            selectedItem: readSystemColor('SelectedItem'),
            selectedItemText: readSystemColor('SelectedItemText'),
        },
        tokens: readFocusTokens(),
    };
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function camelCase(id) {
    return id.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}

async function hoverOwner(page, id) {
    const box = await page.locator(`#${id}`).boundingBox();
    if (!box) {
        throw new Error(`Expected ${id} to expose hover geometry`);
    }
    await page.mouse.move(box.x + Math.min(4, box.width / 2), box.y + Math.min(4, box.height / 2));
}

async function forcePseudoState(cdp, id, forcedPseudoClasses) {
    const { root } = await cdp.send('DOM.getDocument');
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: `#${id}` });
    if (!nodeId) {
        throw new Error(`Expected ${id} to expose a Chromium DOM node`);
    }
    await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses });
}

function equalOutline(first, second) {
    return (
        first.outlineColor === second.outlineColor
        && first.outlineOffset === second.outlineOffset
        && first.outlineStyle === second.outlineStyle
        && first.outlineWidth === second.outlineWidth
    );
}

function equalSize(first, second) {
    return first.length === second.length && first.every((value, index) => value === second[index]);
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
