import assert from 'node:assert/strict';

/** Exercise production declarations and inherited tokens on either Site view. */
export async function verifySiteThemes(page) {
    const modes = ['light', 'dark', 'contrast-light', 'contrast-dark', 'native'];
    const themes = [];
    for (const mode of modes) {
        const result = await page.evaluate(mode => {
            const root = document.querySelector('main');
            root.className = `cem-theme-${mode}`;
            const action = root.querySelector('cem-action button');
            const field = root.querySelector('cem-field input');
            const surface = root.querySelector('cem-surface [part~=root]');
            const preview = root.querySelector('cem-media-preview [part~=text]');
            const original = { background: getComputedStyle(action).backgroundColor, color: getComputedStyle(action).color,
                fieldIndicator: getComputedStyle(field).boxShadow };
            root.style.setProperty('--cem-action-border-radius', '13px');
            root.style.setProperty('--cem-inset-container', '19px');
            root.style.setProperty('--cem-typography-script-font-size', '17px');
            root.style.setProperty('--cem-palette-comfort', 'rgb(24, 36, 48)');
            root.style.setProperty('--cem-palette-comfort-text', 'rgb(241, 243, 245)');
            const style = getComputedStyle(action);
            const result = { mode, ...original, radius: parseFloat(style.borderTopLeftRadius),
                coupling: parseFloat(style.borderTopWidth),
                ...(surface ? { surfaceInset: getComputedStyle(surface).paddingTop,
                    surfaceBackground: getComputedStyle(surface).backgroundColor, surfaceColor: getComputedStyle(surface).color } : {}),
                ...(preview ? { previewFont: getComputedStyle(preview).fontSize, previewInset: getComputedStyle(preview).paddingTop } : {}) };
            root.removeAttribute('style');
            return result;
        }, mode);
        assert.notEqual(result.background, 'rgba(0, 0, 0, 0)', `${mode} action is unstyled`);
        assert.notEqual(result.background, result.color, `${mode} action has no foreground distinction`);
        assert.equal(result.radius, 13 + result.coupling, `${mode} action lost coupled inherited radius`);
        assert.notEqual(result.fieldIndicator, 'none', `${mode} field has no indicator`);
        if (result.surfaceInset) {
            assert.equal(result.surfaceInset, '19px');
            assert.equal(result.surfaceBackground, 'rgb(24, 36, 48)');
            assert.equal(result.surfaceColor, 'rgb(241, 243, 245)');
            assert.equal(result.previewFont, '17px');
            assert.equal(result.previewInset, '19px');
        }
        themes.push(result);
    }
    await page.emulateMedia({ forcedColors: 'active' });
    const forced = await page.evaluate(() => [...document.querySelectorAll('main cem-action button, main cem-field input, main cem-surface [part~=root], main cem-media-preview [part~=root]')]
        .map(node => { const style = getComputedStyle(node); return { tag: node.localName, color: style.color,
            background: style.backgroundColor, adjust: style.forcedColorAdjust, border: style.borderStyle,
            outline: style.outlineStyle, outlineWidth: parseFloat(style.outlineWidth) }; }));
    assert.ok(forced.length >= 2);
    for (const owner of forced) {
        assert.equal(owner.adjust, 'auto');
        assert.notEqual(owner.color, owner.background);
        if (owner.tag === 'input') {
            assert.equal(owner.outline, 'solid');
            assert.ok(owner.outlineWidth > 0);
        } else assert.notEqual(owner.border, 'none');
    }
    await page.emulateMedia({ forcedColors: 'none' });
    await page.evaluate(() => document.querySelector('main').removeAttribute('class'));
    return { themes, forcedColors: forced };
}
