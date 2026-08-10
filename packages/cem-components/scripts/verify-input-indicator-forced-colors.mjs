#!/usr/bin/env node

import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const repoRoot = join(packageRoot, '..', '..');
const componentCss = await readFile(join(packageRoot, 'src', 'styles.css'), 'utf8');
const themeCss = await readFile(join(repoRoot, 'packages', 'cem-theme', 'dist', 'lib', 'css', 'cem-combined.css'), 'utf8');
const browser = await chromium.launch({ headless: true });

try {
    const context = await browser.newContext({ forcedColors: 'active', javaScriptEnabled: true });
    const page = await context.newPage();
    await page.setContent(`
        <style>${themeCss}\n${componentCss}</style>
        <button id="focus-start" type="button">Start</button>
        <cem-field><input id="field" value="alpha"></cem-field>
        <cem-text-field><input id="text-field" value="bravo"></cem-text-field>
        <cem-textarea><textarea id="textarea">charlie</textarea></cem-textarea>
        <cem-select><select id="select"><option value="delta">Delta</option></select></cem-select>
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
    `);

    const baseline = await page.evaluate(captureForcedColorState);
    assert(baseline.forcedColors, 'forced-colors media query did not activate');
    assert(baseline.field.boxShadow === 'none', 'field indicator shadow did not collapse in forced colors');
    assert(baseline.binary.boxShadow === 'none', 'binary indicator shadow did not collapse in forced colors');

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

    if (!(field instanceof HTMLInputElement) || !(binary instanceof HTMLInputElement)) {
        throw new Error('Expected forced-colors input owners');
    }
    if (!(binaryLabel instanceof HTMLLabelElement) || !(disabledBinaryLabel instanceof HTMLLabelElement)) {
        throw new Error('Expected forced-colors binary label owners');
    }

    const rootStyles = getComputedStyle(document.documentElement);
    const fieldStyles = getComputedStyle(field);
    const binaryStyles = getComputedStyle(binaryLabel);
    const disabledBinaryStyles = getComputedStyle(disabledBinaryLabel);

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
        system: {
            canvasText: readSystemColor('CanvasText'),
            highlight: readSystemColor('Highlight'),
        },
        tokens: {
            boundary: rootStyles.getPropertyValue('--cem-stroke-boundary').trim(),
            focus: rootStyles.getPropertyValue('--cem-stroke-focus').trim(),
            none: rootStyles.getPropertyValue('--cem-stroke-none').trim(),
        },
    };
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
