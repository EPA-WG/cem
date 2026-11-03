import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';
import { presentationId } from './presentation-deck-notes.mjs';

export const colors = { canvas: 'F1FEFE', ink: '001010', secondary: '1A1C18', primary: '6A1B9A',
    primarySurface: 'E1BEE7', neutral: 'F1F1EB', trustSurface: 'D7E3FF', calm: '006A6A',
    error: 'BA1A1A', link: '002F65', terminal: '001010', terminalError: 'F0F070' };
const titleId = 'h36c7c826c4eee100_0_0';
const map = { F1FAFA: colors.canvas, FFFFFF: colors.neutral, '16343B': colors.ink, '506A70': colors.secondary,
    '6333A6': colors.primary, EEE7F7: colors.primarySurface, '087E83': colors.calm, D6EFED: colors.trustSurface,
    '9F334A': colors.error, C6DCDC: colors.calm };
const closingMap = { FFFFFF: colors.ink, B6DEDE: colors.secondary, BDEAE6: colors.calm,
    CBE4E4: colors.secondary, '385A62': colors.calm, '19333C': colors.canvas };
const rgb = hex => Object.fromEntries(['red', 'green', 'blue'].map((key, index) => [key, parseInt(hex.slice(index * 2, index * 2 + 2), 16) / 255]));
const hex = rgb => ['red', 'green', 'blue'].map(key => Math.round((rgb[key] ?? 0) * 255).toString(16).padStart(2, '0')).join('').toUpperCase();
const unwrap = raw => raw.structuredContent ?? raw;
export function contrast(foreground, background) {
    const luminance = color => ['red', 'green', 'blue'].map(key => rgb(color)[key]).map(value => value <= .04045 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4)
        .reduce((sum, value, index) => sum + value * [.2126, .7152, .0722][index], 0);
    const a = luminance(foreground), b = luminance(background); return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
}
export async function verifyPaletteTokens(source) {
    source ??= await readFile(resolve(workspaceRoot, 'packages/cem-theme/src/lib/tokens/cem-colors.md'), 'utf8');
    const bindings = { 'cyan-xl': colors.canvas, 'cyan-xd': colors.ink, 'grey-d': colors.secondary,
        'purple-d': colors.primary, 'purple-l': colors.primarySurface, 'grey-l': colors.neutral,
        'blue-l': colors.trustSurface, 'blue-d': colors.link, 'cyan-d': colors.calm, 'red-l': colors.error, 'orange-xl': colors.terminalError };
    for (const [token, value] of Object.entries(bindings)) assert.match(source, new RegExp('`--cem-color-' + token + '`[^\\n]+#' + value, 'i'), 'Canonical palette changed');
    const checks = [];
    for (const background of [colors.canvas, colors.neutral, colors.primarySurface, colors.trustSurface]) {
        // Calm labels occur on neutral/trust/canvas, never the creativity fill.
        for (const foreground of [colors.ink, colors.secondary, colors.primary, colors.link, ...(background === colors.primarySurface ? [] : [colors.calm])]) {
            const ratio = contrast(foreground, background); assert.ok(ratio >= 4.5, `Insufficient contrast: ${foreground}/${background}`);
            checks.push({ foreground, background, ratio });
        }
    }
    for (const foreground of [colors.canvas, colors.terminalError]) {
        const ratio = contrast(foreground, colors.terminal); assert.ok(ratio >= 4.5); checks.push({ foreground, background: colors.terminal, ratio });
    }
    return { bindings, checks, sourceSha256: presenterHash(source) };
}

export function planPalette(raw, slideIds) {
    const deck = unwrap(raw); assert.equal(deck.presentationId, presentationId); assert.ok(deck.revisionId);
    assert.deepEqual(deck.slides.map(slide => slide.objectId), slideIds, 'Live slide order changed');
    const requests = [], patches = [];
    const add = (request, path, value) => { requests.push(request); patches.push({ path, value }); };
    for (const [index, slide] of deck.slides.entries()) {
        if (slide.objectId === titleId) continue;
        const base = ['slides', index], mapping = slide.objectId === 'p24' ? closingMap : map;
        const background = slide.pageProperties?.pageBackgroundFill?.solidFill?.color?.rgbColor;
        if (!background || hex(background) !== colors.canvas) {
            const fill = { solidFill: { color: { rgbColor: rgb(colors.canvas) }, alpha: 1 } };
            add({ updatePageProperties: { objectId: slide.objectId, pageProperties: { pageBackgroundFill: fill }, fields: 'pageBackgroundFill' } }, [...base, 'pageProperties', 'pageBackgroundFill'], fill);
        }
        const visit = (elements, path) => elements.forEach((element, i) => {
            const here = [...path, i];
            if (element.elementGroup) visit(element.elementGroup.children, [...here, 'elementGroup', 'children']);
            if (element.image) return; // Keep semantic source colors, QR and supplied security image.
            const shape = element.shape, line = element.line;
            const fill = shape?.shapeProperties?.shapeBackgroundFill;
            const shapeRgb = fill?.solidFill?.color?.rgbColor;
            let shapeColor = shapeRgb && mapping[hex(shapeRgb)];
            if (element.objectId === 'p9_i10') shapeColor = colors.terminal;
            if (shapeColor && fill.propertyState !== 'NOT_RENDERED' && hex(shapeRgb) !== shapeColor) {
                const value = { ...fill, solidFill: { ...fill.solidFill, color: { rgbColor: rgb(shapeColor) } } };
                add({ updateShapeProperties: { objectId: element.objectId, shapeProperties: { shapeBackgroundFill: value }, fields: 'shapeBackgroundFill' } }, [...here, 'shape', 'shapeProperties', 'shapeBackgroundFill'], value);
            }
            for (const [j, run] of (shape?.text?.textElements ?? []).entries()) {
                if (!run.textRun?.content || run.endIndex <= (run.startIndex ?? 0)) continue;
                const old = run.textRun.style?.foregroundColor?.opaqueColor?.rgbColor;
                let color = old && mapping[hex(old)];
                if (run.textRun.style?.link?.url) color = colors.link;
                if (element.objectId === 'p9_i11') color = colors.terminalError;
                if (element.objectId === 'p9_i12') color = colors.canvas;
                if (!color || (old && hex(old) === color)) continue;
                const value = { opaqueColor: { rgbColor: rgb(color) } };
                add({ updateTextStyle: { objectId: element.objectId, textRange: { type: 'FIXED_RANGE', startIndex: run.startIndex ?? 0, endIndex: run.endIndex }, style: { foregroundColor: value }, fields: 'foregroundColor' } }, [...here, 'shape', 'text', 'textElements', j, 'textRun', 'style', 'foregroundColor'], value);
            }
            const properties = line?.lineProperties;
            const outline = shape?.shapeProperties?.outline;
            for (const [kind, value] of [['line', properties?.lineFill], ['outline', outline?.outlineFill]]) {
                const old = value?.solidFill?.color?.rgbColor, color = old && mapping[hex(old)];
                if (!color || color === hex(old)) continue;
                const next = { ...value, solidFill: { ...value.solidFill, color: { rgbColor: rgb(color) }, ...(hex(old) === 'C6DCDC' ? { alpha: .3 } : {}) } };
                if (kind === 'line') add({ updateLineProperties: { objectId: element.objectId, lineProperties: { lineFill: next }, fields: 'lineFill' } }, [...here, 'line', 'lineProperties', 'lineFill'], next);
                else add({ updateShapeProperties: { objectId: element.objectId, shapeProperties: { outline: { outlineFill: next } }, fields: 'outline.outlineFill' } }, [...here, 'shape', 'shapeProperties', 'outline', 'outlineFill'], next);
            }
        });
        visit(slide.pageElements, [...base, 'pageElements']);
    }
    return { presentationId, revisionId: deck.revisionId, requests, patches };
}

export function verifyPaletteReadback(before, after, plan) {
    const expected = structuredClone(unwrap(before)); after = unwrap(after);
    for (const { path, value } of plan.patches) {
        let owner = expected; for (const part of path.slice(0, -1)) owner = owner[part]; owner[path.at(-1)] = value;
        if (path.at(-1) === 'colorScheme') {
            // Google derives TEXT/BACKGROUND aliases from the editable LIGHT/DARK slots.
            owner.colorScheme = structuredClone(value);
            for (const [alias, slot] of Object.entries({ TEXT1: 'DARK1', BACKGROUND1: 'LIGHT1', TEXT2: 'LIGHT2', BACKGROUND2: 'DARK2' })) {
                const pair = owner.colorScheme.colors.find(pair => pair.type === alias);
                const source = owner.colorScheme.colors.find(pair => pair.type === slot);
                if (pair && source) pair.color = source.color;
            }
        }
    }
    const normalize = value => {
        if (Array.isArray(value)) return value.map(normalize);
        if (typeof value === 'number') return Math.round(value * 1e6) / 1e6;
        if (!value || typeof value !== 'object') return value;
        return Object.fromEntries(Object.entries(value).filter(([key]) => !['revisionId', 'contentUrl'].includes(key)).map(([key, child]) => {
            // Slides omits zero RGB channels, merges identical runs, and
            // propagates a paragraph's paint to its nonpainting newline.
            if (key === 'rgbColor') return [key, hex(child)];
            if (key === 'colors' && Array.isArray(child) && child.every(pair => pair.type && pair.color)) {
                return [key, child.map(pair => ({ ...pair, color: hex(pair.color) }))];
            }
            if (key === 'textElements') return [key, normalize(child.flatMap(run => {
                if (!run.textRun) return [run];
                return run.textRun.content.split('').map(character => ({ textRun: { ...run.textRun, content: character,
                    style: character === '\n' ? { ...run.textRun.style, foregroundColor: undefined } : run.textRun.style } }));
            }))];
            return [key, normalize(child)];
        }));
    };
    assert.deepEqual(normalize(after), normalize(expected), 'Readback differs from reviewed color-only changes');
    return { presentationId, checkedDate: '2026-11-01', beforeRevision: unwrap(before).revisionId, afterRevision: after.revisionId,
        slides: after.slides.length, colorRequests: plan.requests.length, titlePreserved: true, notesPreserved: true,
        imagesPreserved: true, textAndGeometryPreserved: true, exactColorReadback: true };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const [beforePath, afterPath] = process.argv.slice(2); assert.ok(beforePath);
    const palette = await verifyPaletteTokens();
    const raw = JSON.parse(await readFile(beforePath, 'utf8'));
    const ids = (await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/native-slide-order.md'), 'utf8')).split('\n')
        .filter(line => /^\| \d{2} \|/.test(line)).map(line => line.split('|')[2].trim().replaceAll('`', ''));
    const plan = planPalette(raw, ids);
    const result = afterPath ? { ...verifyPaletteReadback(raw, JSON.parse(await readFile(afterPath, 'utf8')), plan), palette } : plan;
    const output = resolve(workspaceRoot, 'dist/reports/cem-studio', afterPath ? 'palette-readback.json' : 'palette-plan.json');
    await writeFile(output, JSON.stringify(result, null, 2) + '\n'); console.log(output);
}
