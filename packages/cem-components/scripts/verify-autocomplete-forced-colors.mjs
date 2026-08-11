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
            <button id="focus-start" type="button">Focus start</button>
            <cem-autocomplete id="autocomplete-host">
                <div class="cem-autocomplete">
                    <label class="cem-autocomplete__label" for="autocomplete-input">Person</label>
                    <input
                        id="autocomplete-input"
                        class="cem-autocomplete__control"
                        role="combobox"
                        aria-autocomplete="list"
                        aria-expanded="true"
                        aria-controls="autocomplete-popup"
                        aria-activedescendant="selected-active-option"
                        value="Ada Lovelace"
                    >
                    <div id="autocomplete-popup" class="cem-autocomplete__popup" role="listbox">
                        <div
                            id="selected-active-option"
                            class="cem-autocomplete__option"
                            role="option"
                            aria-selected="true"
                            aria-disabled="false"
                            data-active="true"
                        >Ada Lovelace</div>
                        <div
                            id="hover-option"
                            class="cem-autocomplete__option"
                            role="option"
                            aria-selected="false"
                            aria-disabled="false"
                        >Grace Hopper</div>
                        <div
                            id="disabled-option"
                            class="cem-autocomplete__option"
                            role="option"
                            aria-selected="false"
                            aria-disabled="true"
                        >Unavailable</div>
                    </div>
                </div>
            </cem-autocomplete>
            <cem-autocomplete id="later-host">
                <div class="cem-autocomplete">
                    <label id="later-label" class="cem-autocomplete__label" for="later-input">Later field</label>
                    <input id="later-input" class="cem-autocomplete__control" value="Later content">
                </div>
            </cem-autocomplete>
        </main>
    `);
    await page.evaluate(() => {
        window.__autocompletePointerEvents = { input: [], option: [] };
        window.__autocompleteMutationEvents = [];
        for (const [key, id] of [
            ['input', 'autocomplete-input'],
            ['option', 'hover-option'],
        ]) {
            const element = document.getElementById(id);
            for (const eventName of ['pointerenter', 'pointerleave']) {
                element.addEventListener(eventName, (event) => {
                    window.__autocompletePointerEvents[key].push(`${eventName}:${event.isTrusted}`);
                });
            }
        }
        for (const eventName of ['click', 'input', 'change']) {
            document.addEventListener(eventName, () => window.__autocompleteMutationEvents.push(eventName));
        }
    });

    const baseline = await page.evaluate(captureAutocompleteForcedColorState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.input.boxShadow === 'none', 'autocomplete input retained its indicator shadow');
    assert(baseline.popup.backgroundColor === baseline.system.canvas, 'autocomplete popup did not map to Canvas');
    assert(baseline.popup.borderColor === baseline.system.canvasText, 'autocomplete popup border did not map to CanvasText');
    assert(baseline.popup.color === baseline.system.canvasText, 'autocomplete popup text did not map to CanvasText');
    assert(baseline.popup.zIndex === '1', 'autocomplete popup lost its bounded physical stacking value');
    assert(
        baseline.selectedActive.backgroundColor === baseline.system.highlight,
        'active selected option did not map to Highlight',
    );
    assert(
        baseline.selectedActive.color === baseline.system.highlightText,
        'active selected option text did not map to HighlightText',
    );
    assert(baseline.selectedActive.outlineStyle === 'solid', 'selected option lost its inset outline');
    assert(
        baseline.selectedActive.outlineWidth === baseline.tokens.selected,
        'selected option outline has the wrong width',
    );
    assert(
        baseline.selectedActive.outlineColor === baseline.system.selectedItem,
        'selected option outline did not map to SelectedItem',
    );
    assert(baseline.disabled.color === baseline.system.grayText, 'disabled option did not map to GrayText');

    await page.locator('#hover-option').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(captureAutocompleteForcedColorState);
    assert(hovered.hover.backgroundColor === hovered.system.highlight, 'option hover did not map to Highlight');
    assert(hovered.hover.color === hovered.system.highlightText, 'option hover text did not map to HighlightText');
    assert(!hovered.laterLabelHover, 'later structural content intercepted popup hover');
    assert(equalRect(hovered.hover.rect, baseline.hover.rect), 'option hover changed option geometry');
    assert(equalRect(hovered.hostRect, baseline.hostRect), 'option hover changed host geometry');
    assert(hovered.hostHtml === baseline.hostHtml, 'option hover changed autocomplete DOM or ARIA');
    assert(hovered.semanticState === baseline.semanticState, 'option hover changed selected or active state');
    assert(
        hovered.selectedActive.backgroundColor === hovered.system.highlight,
        'hover on another option changed active selected paint',
    );
    assert(
        hovered.selectedActive.outlineColor === hovered.system.selectedItem,
        'hover on another option removed selected coexistence',
    );

    await page.mouse.move(0, 0);
    await page.evaluate(nextFrame);
    const disabledBaseline = await page.evaluate(captureAutocompleteForcedColorState);
    await page.locator('#disabled-option').hover();
    await page.evaluate(nextFrame);
    const disabledHover = await page.evaluate(captureAutocompleteForcedColorState);
    assert(
        disabledHover.disabled.backgroundColor === disabledBaseline.disabled.backgroundColor,
        'disabled option acquired hover fill',
    );
    assert(disabledHover.disabled.color === disabledBaseline.disabled.color, 'disabled option acquired hover text');
    assert(equalRect(disabledHover.disabled.rect, disabledBaseline.disabled.rect), 'disabled hover changed geometry');
    assert(disabledHover.semanticState === baseline.semanticState, 'disabled hover changed component state');

    await page.mouse.move(0, 0);
    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    assert(
        await page.locator('#autocomplete-input').evaluate((element) => document.activeElement === element),
        'autocomplete input did not receive keyboard focus',
    );
    const focused = await page.evaluate(captureAutocompleteForcedColorState);
    assert(focused.input.focusVisible, 'autocomplete input did not match :focus-visible');
    assert(focused.input.boxShadow === 'none', 'focused autocomplete input restored an indicator shadow');
    assert(focused.input.outlineStyle === 'solid', 'autocomplete focus fallback is not a solid outline');
    assert(focused.input.outlineWidth === focused.tokens.focus, 'autocomplete focus outline has the wrong width');
    assert(focused.input.outlineColor === focused.system.canvasText, 'autocomplete focus did not map to CanvasText');
    assert(focused.input.ariaExpanded === 'true', 'focus changed the accepted expanded state');

    const focusedBaseline = focused;
    await page.locator('#autocomplete-input').hover();
    await page.evaluate(nextFrame);
    const inputHover = await page.evaluate(captureAutocompleteForcedColorState);
    assert(inputHover.input.focusVisible, 'input hover removed :focus-visible');
    assert(inputHover.input.borderColor === inputHover.system.highlight, 'input hover did not map to Highlight');
    assert(inputHover.input.boxShadow === 'none', 'input hover restored an indicator shadow');
    assert(equalRect(inputHover.input.rect, focusedBaseline.input.rect), 'input hover changed geometry');
    assert(equalOutline(inputHover.input, focusedBaseline.input), 'input hover changed the focus-visible outline');
    assert(inputHover.semanticState === baseline.semanticState, 'input focus/hover changed component state');

    await page.mouse.move(0, 0);
    await page.evaluate(nextFrame);
    const finalState = await page.evaluate(captureAutocompleteForcedColorState);
    assert(
        finalState.pointerEvents.option.join('|') === 'pointerenter:true|pointerleave:true',
        'option did not receive one trusted pointer enter/leave pair',
    );
    assert(
        finalState.pointerEvents.input.join('|') === 'pointerenter:true|pointerleave:true',
        'input did not receive one trusted pointer enter/leave pair',
    );
    assert(finalState.mutationEvents.length === 0, 'hover or focus dispatched a mutation event');
    assert(finalState.hostHtml === baseline.hostHtml, 'hover or focus changed autocomplete DOM or ARIA');
    assert(finalState.semanticState === baseline.semanticState, 'hover or focus changed selected or active state');

    await context.close();
    console.log('cem-components autocomplete forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureAutocompleteForcedColorState() {
    const required = (selector) => {
        const element = document.querySelector(selector);
        if (!(element instanceof HTMLElement)) throw new Error(`Expected ${selector}`);
        return element;
    };
    const readSystemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const readRect = (element) => {
        const bounds = element.getBoundingClientRect();
        return [bounds.x, bounds.y, bounds.width, bounds.height];
    };
    const readOption = (element) => {
        const styles = getComputedStyle(element);
        return {
            backgroundColor: styles.backgroundColor,
            color: styles.color,
            hover: element.matches(':hover'),
            outlineColor: styles.outlineColor,
            outlineStyle: styles.outlineStyle,
            outlineWidth: styles.outlineWidth,
            rect: readRect(element),
        };
    };
    const host = required('#autocomplete-host');
    const input = required('#autocomplete-input');
    const popup = required('#autocomplete-popup');
    const selectedActive = required('#selected-active-option');
    const hover = required('#hover-option');
    const disabled = required('#disabled-option');
    const inputStyles = getComputedStyle(input);
    const popupStyles = getComputedStyle(popup);
    const rootStyles = getComputedStyle(document.documentElement);

    return {
        disabled: readOption(disabled),
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostHtml: host.innerHTML,
        hostRect: readRect(host),
        input: {
            ariaExpanded: input.getAttribute('aria-expanded'),
            borderColor: inputStyles.borderColor,
            boxShadow: inputStyles.boxShadow,
            focusVisible: input.matches(':focus-visible'),
            hover: input.matches(':hover'),
            outlineColor: inputStyles.outlineColor,
            outlineOffset: inputStyles.outlineOffset,
            outlineStyle: inputStyles.outlineStyle,
            outlineWidth: inputStyles.outlineWidth,
            rect: readRect(input),
        },
        hover: readOption(hover),
        laterLabelHover: required('#later-label').matches(':hover'),
        mutationEvents: [...window.__autocompleteMutationEvents],
        pointerEvents: structuredClone(window.__autocompletePointerEvents),
        popup: {
            backgroundColor: popupStyles.backgroundColor,
            borderColor: popupStyles.borderColor,
            color: popupStyles.color,
            zIndex: popupStyles.zIndex,
        },
        selectedActive: readOption(selectedActive),
        semanticState: JSON.stringify({
            activeDescendant: input.getAttribute('aria-activedescendant'),
            disabled: disabled.getAttribute('aria-disabled'),
            disabledSelected: disabled.getAttribute('aria-selected'),
            expanded: input.getAttribute('aria-expanded'),
            hoverSelected: hover.getAttribute('aria-selected'),
            selected: selectedActive.getAttribute('aria-selected'),
            selectedActive: selectedActive.getAttribute('data-active'),
        }),
        system: {
            canvas: readSystemColor('Canvas'),
            canvasText: readSystemColor('CanvasText'),
            grayText: readSystemColor('GrayText'),
            highlight: readSystemColor('Highlight'),
            highlightText: readSystemColor('HighlightText'),
            selectedItem: readSystemColor('SelectedItem'),
        },
        tokens: {
            focus: rootStyles.getPropertyValue('--cem-stroke-focus').trim(),
            selected: rootStyles.getPropertyValue('--cem-stroke-selected').trim(),
        },
    };
}

function equalRect(first, second) {
    return first.every((value, index) => value === second[index]);
}

function equalOutline(first, second) {
    return ['outlineColor', 'outlineOffset', 'outlineStyle', 'outlineWidth'].every(
        (property) => first[property] === second[property],
    );
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
