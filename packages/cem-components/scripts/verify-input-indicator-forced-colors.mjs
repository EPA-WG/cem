#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = join(packageRoot, '..', '..');
const componentCss = await readFile(join(packageRoot, 'src', 'styles.css'), 'utf8');
const selectDeclaration = await readFile(
    join(packageRoot, 'src', 'components', 'cem-select', 'cem-select.xhtml'),
    'utf8',
);
const selectCss = selectDeclaration.match(/\{style(?:\s+[^|{}]*)?\s*\|```([\s\S]*?)```\s*\}/i)?.[1];
if (!selectCss) throw new Error('cem-select.xhtml must contain embedded CEM-ML style content');
const themeCss = await readFile(join(repoRoot, 'packages', 'cem-theme', 'dist', 'lib', 'css', 'cem-combined.css'), 'utf8');
const browser = await chromium.launch({ headless: true });

try {
    const context = await browser.newContext({ forcedColors: 'active', javaScriptEnabled: true });
    const page = await context.newPage();
    await page.setContent(`
        <style>${themeCss}\n${componentCss}\n${selectCss}</style>
        <button id="focus-start" type="button">Start</button>
        <cem-field><input id="field" value="alpha"></cem-field>
        <cem-text-field><input id="text-field" value="bravo"></cem-text-field>
        <cem-textarea><textarea id="textarea">charlie</textarea></cem-textarea>
        <cem-select>
            <button id="select" class="cem-select__control" type="button" aria-expanded="true">Delta</button>
            <div id="select-popup" class="cem-select__popup">
                <div id="select-active" class="cem-select__option" data-active="true" aria-selected="false">Alpha</div>
                <div id="select-selected" class="cem-select__option" aria-selected="true">Delta</div>
                <div id="select-disabled" class="cem-select__option" aria-disabled="true">Unavailable</div>
            </div>
        </cem-select>
        <cem-checkbox>
            <label id="binary-label"><input id="binary" type="checkbox"><span>Choice</span></label>
        </cem-checkbox>
        <cem-radio>
            <label id="radio-label"><input id="radio" type="radio"><span>Radio</span></label>
        </cem-radio>
        <cem-switch>
            <label id="switch-label">
                <input id="switch" type="checkbox" role="switch"><span>Switch</span>
            </label>
        </cem-switch>
        <cem-radio>
            <label id="disabled-binary-label">
                <input id="disabled-binary" type="radio" disabled><span>Disabled</span>
            </label>
        </cem-radio>
        <button id="focus-end" type="button">End</button>
        <cem-field><input id="pending-field" data-state="loading" aria-busy="true" value="pending"></cem-field>
        <cem-checkbox>
            <label id="pending-binary-label">
                <input id="pending-binary" type="checkbox" data-state="loading" aria-busy="true">
                <span>Pending choice</span>
            </label>
        </cem-checkbox>
    `);

    const baseline = await page.evaluate(captureForcedColorState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.field.boxShadow === 'none', 'field indicator shadow did not collapse in forced colors');
    assert(baseline.binary.boxShadow === 'none', 'binary indicator shadow did not collapse in forced colors');
    assert(baseline.pendingField.boxShadow === 'none', 'pending field restored a shadow in forced colors');
    assert(baseline.pendingField.outlineStyle === 'solid', 'pending field fallback is not a solid outline');
    assert(baseline.pendingField.outlineWidth === baseline.tokens.pending, 'pending field fallback has the wrong width');
    assert(baseline.pendingField.outlineColor === baseline.system.canvasText, 'pending field did not map to CanvasText');
    assert(baseline.pendingBinary.boxShadow === 'none', 'pending binary restored a shadow in forced colors');
    assert(baseline.pendingBinary.outlineStyle === 'solid', 'pending binary fallback is not a solid outline');
    assert(
        baseline.pendingBinary.outlineWidth === baseline.tokens.pending,
        'pending binary fallback has the wrong width',
    );
    assert(baseline.pendingBinary.outlineColor === baseline.system.canvasText, 'pending binary did not map to CanvasText');
    assert(baseline.select.popup.backgroundColor === baseline.system.canvas, 'select popup did not map to Canvas');
    assert(baseline.select.popup.borderColor === baseline.system.canvasText, 'select popup border did not map to CanvasText');
    assert(baseline.select.popup.color === baseline.system.canvasText, 'select popup text did not map to CanvasText');
    assert(baseline.select.active.backgroundColor === baseline.system.highlight, 'active option did not map to Highlight');
    assert(baseline.select.active.color === baseline.system.highlightText, 'active option did not map to HighlightText');
    assert(baseline.select.selected.outlineStyle === 'solid', 'selected option fallback is not a solid outline');
    assert(
        baseline.select.selected.outlineWidth === baseline.tokens.selected,
        'selected option fallback has the wrong width',
    );
    assert(
        baseline.select.selected.outlineColor === baseline.system.selectedItem,
        'selected option did not map to SelectedItem',
    );
    assert(baseline.select.disabled.color === baseline.system.grayText, 'disabled option did not map to GrayText');

    await page.locator('#pending-field').focus();
    const pendingFocus = await page.evaluate(captureForcedColorState);
    assert(pendingFocus.pendingField.focusVisible, 'pending field did not match :focus-visible');
    assert(
        pendingFocus.pendingField.outlineWidth === pendingFocus.tokens.focus,
        'focus did not replace the pending outline width',
    );

    await page.locator('#field').hover();
    const fieldHover = await page.evaluate(captureForcedColorState);
    assert(fieldHover.field.borderColor === fieldHover.system.highlight, 'field hover did not map to Highlight');
    assert(fieldHover.field.boxShadow === 'none', 'field hover restored a shadow in forced colors');

    await page.locator('#field').focus();
    const fieldFocus = await page.evaluate(captureForcedColorState);
    assert(fieldFocus.field.focusVisible, 'field did not match :focus-visible');
    assert(fieldFocus.field.outlineStyle === 'solid', 'field focus fallback is not a solid outline');
    assert(fieldFocus.field.outlineWidth === fieldFocus.tokens.focus, 'field focus fallback has the wrong width');
    assert(fieldFocus.field.outlineColor === fieldFocus.system.canvasText, 'field focus did not map to CanvasText');

    await page.locator('#binary').hover();
    const binaryHover = await page.evaluate(captureForcedColorState);
    assert(binaryHover.binary.outlineStyle === 'solid', 'binary hover fallback is not a solid outline');
    assert(binaryHover.binary.outlineWidth === binaryHover.tokens.boundary, 'binary hover fallback has the wrong width');
    assert(binaryHover.binary.outlineColor === binaryHover.system.highlight, 'binary hover did not map to Highlight');

    await page.locator('#binary').focus();
    const binaryFocus = await page.evaluate(captureForcedColorState);
    assert(binaryFocus.binaryControlFocusVisible, 'binary control did not match :focus-visible');
    assert(binaryFocus.binary.outlineStyle === 'solid', 'binary focus fallback is not a solid outline');
    assert(binaryFocus.binary.outlineWidth === binaryFocus.tokens.focus, 'binary focus fallback has the wrong width');
    assert(binaryFocus.binary.outlineColor === binaryFocus.system.canvasText, 'binary focus did not map to CanvasText');

    await page.locator('#disabled-binary').hover();
    const disabledHover = await page.evaluate(captureForcedColorState);
    assert(
        disabledHover.disabledBinary.outlineWidth === disabledHover.tokens.none,
        'disabled binary control acquired a hover outline',
    );

    const focusCases = [
        { control: '#field', target: '#field' },
        { control: '#text-field', target: '#text-field' },
        { control: '#textarea', target: '#textarea' },
        { control: '#select', target: '#select' },
        { control: '#binary', target: '#binary-label' },
        { control: '#radio', target: '#radio-label' },
        { control: '#switch', target: '#switch-label' },
    ];
    await page.mouse.move(0, 0);
    await page.locator('#focus-start').focus();

    for (const focusCase of focusCases) {
        await page.keyboard.press('Tab');
        const focused = await page.evaluate(captureForcedFocusState, focusCase);
        assert(focused.activeSelector === focusCase.control, `${focusCase.control} did not receive keyboard focus`);
        assert(focused.focusVisible, `${focusCase.control} did not match :focus-visible`);
        assert(focused.boxShadow === 'none', `${focusCase.target} restored a shadow in forced colors`);
        assert(focused.outlineStyle === 'solid', `${focusCase.target} focus fallback is not a solid outline`);
        assert(focused.outlineWidth === focused.focusWidth, `${focusCase.target} focus fallback has the wrong width`);
        assert(focused.outlineColor === focused.canvasText, `${focusCase.target} focus did not map to CanvasText`);
    }

    await page.keyboard.press('Tab');
    assert(
        await page.locator('#focus-end').evaluate((element) => document.activeElement === element),
        'disabled input was not skipped before the focus sequence ended',
    );

    await context.close();
    console.log('cem-components input indicator forced-colors contract verified.');
} finally {
    await browser.close();
}

function captureForcedFocusState({ control, target }) {
    const controlElement = document.querySelector(control);
    const targetElement = document.querySelector(target);

    if (!(controlElement instanceof HTMLElement) || !(targetElement instanceof HTMLElement)) {
        throw new Error(`Expected forced-colors focus owners ${control} and ${target}`);
    }

    const probe = document.createElement('span');
    probe.style.color = 'CanvasText';
    probe.style.forcedColorAdjust = 'none';
    document.body.append(probe);
    const canvasText = getComputedStyle(probe).color;
    probe.remove();

    const styles = getComputedStyle(targetElement);
    return {
        activeSelector: document.activeElement === controlElement ? control : null,
        boxShadow: styles.boxShadow,
        canvasText,
        focusVisible: controlElement.matches(':focus-visible'),
        focusWidth: getComputedStyle(document.documentElement).getPropertyValue('--cem-stroke-focus').trim(),
        outlineColor: styles.outlineColor,
        outlineStyle: styles.outlineStyle,
        outlineWidth: styles.outlineWidth,
    };
}

function captureForcedColorState() {
    const readIndicatorStyles = (styles) => ({
        boxShadow: styles.boxShadow,
        outlineColor: styles.outlineColor,
        outlineStyle: styles.outlineStyle,
        outlineWidth: styles.outlineWidth,
    });
    const readSystemColor = (color) => {
        const probe = document.createElement('span');
        probe.style.color = color;
        probe.style.forcedColorAdjust = 'none';
        document.body.append(probe);
        const resolved = getComputedStyle(probe).color;
        probe.remove();
        return resolved;
    };
    const field = document.querySelector('#field');
    const binary = document.querySelector('#binary');
    const binaryLabel = document.querySelector('#binary-label');
    const disabledBinaryLabel = document.querySelector('#disabled-binary-label');
    const pendingField = document.querySelector('#pending-field');
    const pendingBinaryLabel = document.querySelector('#pending-binary-label');
    const selectPopup = document.querySelector('#select-popup');
    const selectActive = document.querySelector('#select-active');
    const selectSelected = document.querySelector('#select-selected');
    const selectDisabled = document.querySelector('#select-disabled');

    if (
        !(field instanceof HTMLInputElement) ||
        !(binary instanceof HTMLInputElement) ||
        !(pendingField instanceof HTMLInputElement)
    ) {
        throw new Error('Expected forced-colors input owners');
    }
    if (
        !(binaryLabel instanceof HTMLLabelElement) ||
        !(disabledBinaryLabel instanceof HTMLLabelElement) ||
        !(pendingBinaryLabel instanceof HTMLLabelElement)
    ) {
        throw new Error('Expected forced-colors binary label owners');
    }
    if (
        !(selectPopup instanceof HTMLElement) ||
        !(selectActive instanceof HTMLElement) ||
        !(selectSelected instanceof HTMLElement) ||
        !(selectDisabled instanceof HTMLElement)
    ) {
        throw new Error('Expected forced-colors custom select owners');
    }

    const rootStyles = getComputedStyle(document.documentElement);
    const fieldStyles = getComputedStyle(field);
    const binaryStyles = getComputedStyle(binaryLabel);
    const disabledBinaryStyles = getComputedStyle(disabledBinaryLabel);
    const pendingFieldStyles = getComputedStyle(pendingField);
    const pendingBinaryStyles = getComputedStyle(pendingBinaryLabel);
    const selectPopupStyles = getComputedStyle(selectPopup);
    const selectActiveStyles = getComputedStyle(selectActive);
    const selectSelectedStyles = getComputedStyle(selectSelected);
    const selectDisabledStyles = getComputedStyle(selectDisabled);

    return {
        binary: readIndicatorStyles(binaryStyles),
        binaryControlFocusVisible: binary.matches(':focus-visible'),
        disabledBinary: readIndicatorStyles(disabledBinaryStyles),
        field: {
            ...readIndicatorStyles(fieldStyles),
            borderColor: fieldStyles.borderColor,
            focusVisible: field.matches(':focus-visible'),
        },
        forcedColors: matchMedia('(forced-colors: active)').matches,
        pendingBinary: readIndicatorStyles(pendingBinaryStyles),
        pendingField: {
            ...readIndicatorStyles(pendingFieldStyles),
            focusVisible: pendingField.matches(':focus-visible'),
        },
        select: {
            active: {
                backgroundColor: selectActiveStyles.backgroundColor,
                color: selectActiveStyles.color,
            },
            disabled: {
                color: selectDisabledStyles.color,
            },
            popup: {
                backgroundColor: selectPopupStyles.backgroundColor,
                borderColor: selectPopupStyles.borderColor,
                color: selectPopupStyles.color,
            },
            selected: readIndicatorStyles(selectSelectedStyles),
        },
        system: {
            canvas: readSystemColor('Canvas'),
            canvasText: readSystemColor('CanvasText'),
            grayText: readSystemColor('GrayText'),
            highlight: readSystemColor('Highlight'),
            highlightText: readSystemColor('HighlightText'),
            selectedItem: readSystemColor('SelectedItem'),
        },
        tokens: {
            boundary: rootStyles.getPropertyValue('--cem-stroke-boundary').trim(),
            focus: rootStyles.getPropertyValue('--cem-stroke-focus').trim(),
            none: rootStyles.getPropertyValue('--cem-stroke-none').trim(),
            pending: rootStyles.getPropertyValue('--cem-stroke-pending').trim(),
            selected: rootStyles.getPropertyValue('--cem-stroke-selected').trim(),
        },
    };
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
