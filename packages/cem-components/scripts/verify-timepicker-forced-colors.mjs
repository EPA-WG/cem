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
            <button id="focus-start" type="button">Focus start</button>
            <label for="timepicker-input">Meeting time</label>
            <cem-timepicker id="timepicker-host">
                <span class="cem-timepicker" data-mode="valid">
                    <input
                        id="timepicker-input"
                        slot="input"
                        type="text"
                        value="09:30"
                        role="combobox"
                        aria-autocomplete="list"
                        aria-haspopup="listbox"
                        aria-expanded="true"
                        aria-controls="timepicker-popup"
                        aria-activedescendant="selected-active-option"
                    >
                    <button
                        id="timepicker-toggle"
                        slot="toggle"
                        type="button"
                        aria-label="Choose meeting time"
                        aria-haspopup="listbox"
                        aria-expanded="true"
                        aria-controls="timepicker-popup"
                    >Choose</button>
                    <div
                        id="timepicker-popup"
                        class="cem-timepicker__popup"
                        role="listbox"
                        popover="manual"
                        aria-label="Meeting time options"
                    >
                        <div
                            id="selected-active-option"
                            class="cem-timepicker__option"
                            role="option"
                            data-option-index="0"
                            data-value="09:30"
                            data-active="true"
                            aria-selected="true"
                            aria-disabled="false"
                        >9:30 AM</div>
                        <div
                            id="hover-option"
                            class="cem-timepicker__option"
                            role="option"
                            data-option-index="1"
                            data-value="10:00"
                            data-active="false"
                            aria-selected="false"
                            aria-disabled="false"
                        >10:00 AM</div>
                        <div
                            id="disabled-option"
                            class="cem-timepicker__option"
                            role="option"
                            data-option-index="2"
                            data-value="10:30"
                            data-active="false"
                            aria-selected="false"
                            aria-disabled="true"
                        >10:30 AM</div>
                    </div>
                </span>
            </cem-timepicker>
            <button id="later-action" type="button">Later action</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__timepickerEvents = [];
        window.__timepickerPointers = { input: [], option: [] };
        for (const eventName of ['input', 'change', 'cem-timepicker-toggle']) {
            document.querySelector('#timepicker-host').addEventListener(eventName, (event) => {
                window.__timepickerEvents.push(event.type);
            });
        }
        for (const [key, selector] of [['input', '#timepicker-input'], ['option', '#hover-option']]) {
            const element = document.querySelector(selector);
            for (const eventName of ['pointerenter', 'pointerleave']) {
                element.addEventListener(eventName, (event) => {
                    window.__timepickerPointers[key].push(`${eventName}:${event.isTrusted}`);
                });
            }
        }
    });

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assert(baseline.activeElement === 'timepicker-input', 'keyboard focus did not reach the exact input owner');
    assert(baseline.input.focusVisible, 'timepicker input did not retain :focus-visible');
    assert(baseline.input.boxShadow === 'none', 'forced colors retained the input indicator shadow');
    assert(baseline.input.outlineColor === baseline.system.canvasText, 'input focus did not map to CanvasText');
    assert(baseline.input.outlineWidth === baseline.tokens.focus, 'input focus width changed');
    assert(!baseline.open, 'timepicker popup must begin closed');

    await page.evaluate(() => document.querySelector('#timepicker-popup').showPopover());
    const open = await page.evaluate(captureState);
    assert(open.open, 'timepicker popup did not enter the top layer');
    assert(open.popup.backgroundColor === open.system.canvas, 'timepicker popup did not map to Canvas');
    assert(open.popup.borderColor === open.system.canvasText, 'popup contour did not map to CanvasText');
    assert(open.popup.color === open.system.canvasText, 'popup text did not map to CanvasText');
    assert(open.popup.boxShadow === 'none', 'forced colors retained semantic popup shadow');
    assert(open.popup.zIndex === 'auto', 'top-layer timepicker must not declare numeric z-index');
    assert(open.popup.animationName === 'none', 'timepicker popup introduced component animation');
    assert(open.popup.transitionDuration === '0s', 'timepicker popup introduced component transition');
    assert(open.popup.positionAnchor === '--_cem-timepicker-anchor', 'popup lost its input anchor');
    assert(
        ['block-end', 'end center'].includes(open.popup.positionArea),
        `popup did not use logical block placement: ${open.popup.positionArea}`,
    );
    assert(open.popup.top >= open.input.bottom, 'popup overlapped the anchored input');
    assert(open.activeElement === 'timepicker-input', 'showing the popup moved input focus');
    assert(equalRect(open.input.rect, baseline.input.rect), 'showing the popup changed input geometry');
    assert(open.selectedActive.backgroundColor === open.system.highlight, 'active option did not map to Highlight');
    assert(open.selectedActive.color === open.system.highlightText, 'active option text did not map to HighlightText');
    assert(open.selectedActive.outlineColor === open.system.selectedItem, 'selected coexistence did not map to SelectedItem');
    assert(open.selectedActive.outlineWidth === open.tokens.selected, 'selected outline width changed');
    assert(open.disabled.color === open.system.grayText, 'disabled option did not map to GrayText');

    await page.locator('#hover-option').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(captureState);
    assert(hovered.hover.backgroundColor === hovered.system.highlight, 'option hover did not map to Highlight');
    assert(hovered.hover.color === hovered.system.highlightText, 'option hover text did not map to HighlightText');
    assert(hovered.selectedActive.backgroundColor === hovered.system.highlight, 'hover changed active option paint');
    assert(hovered.selectedActive.outlineColor === hovered.system.selectedItem, 'hover removed selected coexistence');
    assert(equalRect(hovered.hover.rect, open.hover.rect), 'option hover changed geometry');
    assert(hovered.semanticState === open.semanticState, 'option hover mutated active, selected, or disabled state');

    await page.mouse.move(0, 0);
    await page.locator('#disabled-option').hover();
    await page.evaluate(nextFrame);
    const disabledHover = await page.evaluate(captureState);
    assert(disabledHover.disabled.color === disabledHover.system.grayText, 'disabled hover changed GrayText');
    assert(
        disabledHover.disabled.backgroundColor === hovered.disabled.backgroundColor,
        'disabled option acquired hover fill',
    );
    assert(equalRect(disabledHover.disabled.rect, hovered.disabled.rect), 'disabled hover changed geometry');
    assert(disabledHover.semanticState === open.semanticState, 'disabled hover mutated timepicker state');

    await page.mouse.move(0, 0);
    await page.evaluate(() => document.querySelector('#timepicker-popup').hidePopover());
    await page.evaluate(nextFrame);
    await page.locator('#timepicker-input').hover();
    await page.evaluate(nextFrame);
    const inputHover = await page.evaluate(captureState);
    assert(inputHover.input.borderColor === inputHover.system.highlight, 'input hover did not map to Highlight');
    assert(inputHover.input.focusVisible, 'input hover removed keyboard focus-visible');
    assert(inputHover.input.outlineColor === inputHover.system.canvasText, 'input hover changed focus outline');
    assert(equalRect(inputHover.input.rect, baseline.input.rect), 'input hover changed geometry');

    await page.mouse.move(0, 0);
    await page.evaluate(() => {
        const popup = document.querySelector('#timepicker-popup');
        if (popup.matches(':popover-open')) popup.hidePopover();
    });
    const closed = await page.evaluate(captureState);
    assert(!closed.open, 'timepicker popup did not close');
    assert(equalRect(closed.input.rect, baseline.input.rect), 'closing the popup changed input geometry');
    assert(closed.input.value === baseline.input.value, 'transient states changed input value');
    assert(closed.hostAttributes === baseline.hostAttributes, 'transient states changed host attributes');
    assert(closed.events.length === 0, `unexpected mutation events: ${closed.events.join('|')}`);
    assert(
        closed.pointers.option.join('|') === 'pointerenter:true|pointerleave:true',
        'option did not receive one trusted enter/leave pair',
    );
    assert(
        closed.pointers.input.join('|') === 'pointerenter:true|pointerleave:true',
        'input did not receive one trusted enter/leave pair',
    );

    console.log('cem-components timepicker forced-colors/top-layer contract verified.');
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
    const optionState = (element) => {
        const style = getComputedStyle(element);
        return {
            backgroundColor: style.backgroundColor,
            color: style.color,
            outlineColor: style.outlineColor,
            outlineWidth: style.outlineWidth,
            rect: rect(element),
        };
    };
    const host = required('#timepicker-host');
    const input = required('#timepicker-input');
    const popup = required('#timepicker-popup');
    const selectedActive = required('#selected-active-option');
    const hover = required('#hover-option');
    const disabled = required('#disabled-option');
    const inputStyle = getComputedStyle(input);
    const popupStyle = getComputedStyle(popup);
    return {
        activeElement: document.activeElement?.id ?? '',
        disabled: optionState(disabled),
        events: [...window.__timepickerEvents],
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostAttributes: [...host.attributes].map(({ name, value }) => `${name}=${value}`).join('|'),
        hover: optionState(hover),
        input: {
            borderColor: inputStyle.borderColor,
            bottom: input.getBoundingClientRect().bottom,
            boxShadow: inputStyle.boxShadow,
            focusVisible: input.matches(':focus-visible'),
            outlineColor: inputStyle.outlineColor,
            outlineWidth: inputStyle.outlineWidth,
            rect: rect(input),
            value: input.value,
        },
        open: popup.matches(':popover-open'),
        pointers: structuredClone(window.__timepickerPointers),
        popup: {
            animationName: popupStyle.animationName,
            backgroundColor: popupStyle.backgroundColor,
            borderColor: popupStyle.borderColor,
            boxShadow: popupStyle.boxShadow,
            color: popupStyle.color,
            positionAnchor: popupStyle.positionAnchor,
            positionArea: popupStyle.positionArea,
            top: popup.getBoundingClientRect().top,
            transitionDuration: popupStyle.transitionDuration,
            zIndex: popupStyle.zIndex,
        },
        reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches,
        selectedActive: optionState(selectedActive),
        semanticState: [selectedActive, hover, disabled]
            .map((option) => [option.dataset.active, option.getAttribute('aria-selected'), option.getAttribute('aria-disabled')].join(':'))
            .join('|'),
        system: {
            canvas: systemColor('Canvas'),
            canvasText: systemColor('CanvasText'),
            grayText: systemColor('GrayText'),
            highlight: systemColor('Highlight'),
            highlightText: systemColor('HighlightText'),
            selectedItem: systemColor('SelectedItem'),
        },
        tokens: {
            focus: lengthToken('--cem-stroke-focus'),
            selected: lengthToken('--cem-stroke-selected'),
        },
    };
}

function nextFrame() {
    return new Promise((resolve) => requestAnimationFrame(() => resolve()));
}

function equalRect(left, right) {
    return left.length === right.length && left.every((value, index) => Math.abs(value - right[index]) <= 0.01);
}

function assert(condition, message) {
    if (!condition) throw new Error(message);
}
