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
            <label for="datepicker-input">Arrival date</label>
            <cem-datepicker id="datepicker-host" min="2026-08-01" max="2026-08-31">
                <span class="cem-datepicker" data-mode="valid">
                    <input
                        id="datepicker-input"
                        slot="input"
                        type="text"
                        value="2026-08-11"
                        role="combobox"
                        aria-autocomplete="none"
                        aria-haspopup="dialog"
                        aria-expanded="true"
                        aria-controls="datepicker-dialog"
                    >
                    <button
                        id="datepicker-toggle"
                        slot="toggle"
                        type="button"
                        aria-label="Choose arrival date"
                        aria-haspopup="dialog"
                        aria-expanded="true"
                        aria-controls="datepicker-dialog"
                    >Choose</button>
                    <dialog
                        id="datepicker-dialog"
                        class="cem-datepicker__dialog"
                        aria-labelledby="datepicker-heading"
                    >
                        <div class="cem-datepicker__header">
                            <button class="cem-datepicker__action" type="button">Previous</button>
                            <h2 id="datepicker-heading" class="cem-datepicker__heading">August 2026</h2>
                            <button class="cem-datepicker__action" type="button">Next</button>
                        </div>
                        <div class="cem-datepicker__grid" role="grid" aria-labelledby="datepicker-heading">
                            <div class="cem-datepicker__week" role="row">
                                <button
                                    id="current-selected-day"
                                    class="cem-datepicker__day"
                                    type="button"
                                    role="gridcell"
                                    data-active="true"
                                    data-date="2026-08-11"
                                    tabindex="0"
                                    aria-selected="true"
                                    aria-current="date"
                                    aria-disabled="false"
                                >11</button>
                                <button
                                    id="hover-day"
                                    class="cem-datepicker__day"
                                    type="button"
                                    role="gridcell"
                                    data-active="false"
                                    data-date="2026-08-12"
                                    tabindex="-1"
                                    aria-selected="false"
                                    aria-disabled="false"
                                >12</button>
                                <button
                                    id="disabled-day"
                                    class="cem-datepicker__day"
                                    type="button"
                                    role="gridcell"
                                    data-active="false"
                                    data-date="2026-08-13"
                                    tabindex="-1"
                                    aria-selected="false"
                                    aria-disabled="true"
                                    disabled
                                >13</button>
                            </div>
                        </div>
                        <div class="cem-datepicker__actions">
                            <button class="cem-datepicker__action" type="button">Cancel</button>
                            <button class="cem-datepicker__action" type="button">Apply</button>
                        </div>
                    </dialog>
                </span>
            </cem-datepicker>
            <button id="later-action" type="button">Later action</button>
        </main>
    `);
    await page.evaluate(() => {
        window.__datepickerEvents = [];
        window.__datepickerPointers = { day: [], input: [] };
        const host = document.querySelector('#datepicker-host');
        for (const eventName of ['input', 'change', 'cem-datepicker-toggle']) {
            host.addEventListener(eventName, (event) => window.__datepickerEvents.push(event.type));
        }
        for (const [key, selector] of [['day', '#hover-day'], ['input', '#datepicker-input']]) {
            const element = document.querySelector(selector);
            for (const eventName of ['pointerenter', 'pointerleave']) {
                element.addEventListener(eventName, (event) => {
                    window.__datepickerPointers[key].push(`${eventName}:${event.isTrusted}`);
                });
            }
        }
    });

    await page.locator('#focus-start').focus();
    await page.keyboard.press('Tab');
    const baseline = await page.evaluate(captureState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.reducedMotion, 'reduced-motion media query did not activate');
    assert(baseline.activeElement === 'datepicker-input', 'keyboard focus did not reach the exact input owner');
    assert(baseline.input.focusVisible, 'datepicker input did not retain :focus-visible');
    assert(baseline.input.boxShadow === 'none', 'forced colors retained the input indicator shadow');
    assert(baseline.input.outlineColor === baseline.system.canvasText, 'input focus did not map to CanvasText');
    assert(baseline.input.outlineWidth === baseline.tokens.focus, 'input focus width changed');
    assert(!baseline.open, 'datepicker dialog must begin closed');

    await page.evaluate(() => document.querySelector('#datepicker-dialog').showModal());
    await page.locator('#current-selected-day').focus();
    const open = await page.evaluate(captureState);
    assert(open.open, 'datepicker dialog did not enter the top layer');
    assert(open.popup.backgroundColor === open.system.canvas, 'datepicker dialog did not map to Canvas');
    assert(open.popup.borderColor === open.system.canvasText, 'dialog contour did not map to CanvasText');
    assert(open.popup.color === open.system.canvasText, 'dialog text did not map to CanvasText');
    assert(open.popup.boxShadow === 'none', 'forced colors retained semantic dialog shadow');
    assert(open.popup.zIndex === 'auto', 'top-layer datepicker must not declare numeric z-index');
    assert(open.popup.animationName === 'none', 'datepicker dialog introduced component animation');
    assert(open.popup.transitionDuration === '0s', 'datepicker dialog introduced component transition');
    assert(open.popup.positionAnchor === '--_cem-datepicker-anchor', 'dialog lost its input anchor');
    assert(
        ['block-end', 'end center'].includes(open.popup.positionArea),
        `dialog did not use logical block placement: ${open.popup.positionArea}`,
    );
    assert(open.popup.top >= open.input.bottom, 'dialog overlapped the anchored input');
    assert(open.activeElement === 'current-selected-day', 'modal focus did not reach the roving day owner');
    assert(open.currentSelected.focusVisible, 'selected current day lost :focus-visible');
    assert(open.currentSelected.backgroundColor === open.system.selectedItem, 'selected day did not map to SelectedItem');
    assert(open.currentSelected.color === open.system.selectedItemText, 'selected day text did not map to SelectedItemText');
    assert(open.currentSelected.borderColor === open.system.mark, 'current day did not map to Mark');
    assert(
        open.currentSelected.outlineColor === open.system.canvasText,
        `day focus did not map to CanvasText: ${open.currentSelected.outlineColor} != ${open.system.canvasText}`,
    );
    assert(open.currentSelected.outlineWidth === open.tokens.focus, 'day focus width changed');
    assert(open.disabled.color === open.system.grayText, 'disabled day did not map to GrayText');
    assert(equalRect(open.input.rect, baseline.input.rect), 'opening the dialog changed input geometry');

    await page.evaluate(() => document.querySelector('#later-action').focus());
    assert(
        await page.evaluate(() => document.activeElement?.id === 'current-selected-day'),
        'modal dialog did not suppress background focus',
    );

    await page.locator('#hover-day').hover();
    await page.evaluate(nextFrame);
    const hovered = await page.evaluate(captureState);
    assert(hovered.hover.backgroundColor === hovered.system.highlight, 'day hover did not map to Highlight');
    assert(hovered.hover.color === hovered.system.highlightText, 'day hover text did not map to HighlightText');
    assert(
        hovered.currentSelected.backgroundColor === hovered.system.selectedItem,
        'hover changed selected coexistence paint',
    );
    assert(hovered.currentSelected.borderColor === hovered.system.mark, 'hover removed current coexistence paint');
    assert(equalRect(hovered.hover.rect, open.hover.rect), 'day hover changed geometry');
    assert(hovered.semanticState === open.semanticState, 'day hover mutated calendar state');

    await page.mouse.move(0, 0);
    await page.locator('#disabled-day').hover();
    await page.evaluate(nextFrame);
    const disabledHover = await page.evaluate(captureState);
    assert(disabledHover.disabled.color === disabledHover.system.grayText, 'disabled hover changed GrayText');
    assert(
        disabledHover.disabled.backgroundColor === hovered.disabled.backgroundColor,
        'disabled day acquired hover fill',
    );
    assert(equalRect(disabledHover.disabled.rect, hovered.disabled.rect), 'disabled hover changed geometry');
    assert(disabledHover.semanticState === open.semanticState, 'disabled hover mutated calendar state');

    await page.mouse.move(0, 0);
    await page.evaluate(() => document.querySelector('#datepicker-dialog').close());
    await page.locator('#datepicker-input').focus();
    await page.locator('#datepicker-input').hover();
    await page.evaluate(nextFrame);
    const inputHover = await page.evaluate(captureState);
    assert(!inputHover.open, 'datepicker dialog did not close');
    assert(inputHover.input.borderColor === inputHover.system.highlight, 'input hover did not map to Highlight');
    assert(inputHover.input.focusVisible, 'input hover removed keyboard focus-visible');
    assert(inputHover.input.outlineColor === inputHover.system.canvasText, 'input hover changed focus outline');
    assert(equalRect(inputHover.input.rect, baseline.input.rect), 'transient states changed input geometry');
    assert(inputHover.input.value === baseline.input.value, 'transient states changed input value');
    assert(inputHover.hostAttributes === baseline.hostAttributes, 'transient states changed host attributes');
    assert(inputHover.events.length === 0, `unexpected mutation events: ${inputHover.events.join('|')}`);

    await page.mouse.move(0, 0);
    const closed = await page.evaluate(captureState);
    assert(
        closed.pointers.day.join('|') === 'pointerenter:true|pointerleave:true',
        'day did not receive one trusted enter/leave pair',
    );
    assert(
        closed.pointers.input.join('|') === 'pointerenter:true|pointerleave:true',
        'input did not receive one trusted enter/leave pair',
    );

    console.log('cem-components datepicker forced-colors/modal/top-layer contract verified.');
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
    const dayState = (element) => {
        const style = getComputedStyle(element);
        return {
            backgroundColor: style.backgroundColor,
            borderColor: style.borderColor,
            color: style.color,
            focusVisible: element.matches(':focus-visible'),
            outlineColor: style.outlineColor,
            outlineWidth: style.outlineWidth,
            rect: rect(element),
        };
    };
    const host = required('#datepicker-host');
    const input = required('#datepicker-input');
    const popup = required('#datepicker-dialog');
    const currentSelected = required('#current-selected-day');
    const hover = required('#hover-day');
    const disabled = required('#disabled-day');
    const inputStyle = getComputedStyle(input);
    const popupStyle = getComputedStyle(popup);
    return {
        activeElement: document.activeElement?.id ?? '',
        currentSelected: dayState(currentSelected),
        disabled: dayState(disabled),
        events: [...window.__datepickerEvents],
        forcedColors: matchMedia('(forced-colors: active)').matches,
        hostAttributes: [...host.attributes].map(({ name, value }) => `${name}=${value}`).join('|'),
        hover: dayState(hover),
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
        open: popup.matches(':modal'),
        pointers: structuredClone(window.__datepickerPointers),
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
        semanticState: [currentSelected, hover, disabled]
            .map((day) => [day.dataset.active, day.getAttribute('aria-selected'), day.getAttribute('aria-disabled')].join(':'))
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
