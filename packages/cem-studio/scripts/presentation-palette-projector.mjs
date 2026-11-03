import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { extractTable } from '../../cem-theme/scripts/manifest-utils.mjs';
import { contrast, verifyPaletteReadback } from './presentation-palette.mjs';
import { presentationId } from './presentation-deck-notes.mjs';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';

const titleId = 'h36c7c826c4eee100_0_0';
const diagnosticIds = new Set(['p9_i10', 'p9_i11', 'p9_i12']);
const rgb = hex => Object.fromEntries(['red', 'green', 'blue'].map((key, i) => [key, parseInt(hex.slice(i * 2, i * 2 + 2), 16) / 255]));
const hex = color => ['red', 'green', 'blue'].map(key => Math.round((color[key] ?? 0) * 255).toString(16).padStart(2, '0')).join('').toUpperCase();
const unwrap = raw => raw.structuredContent ?? raw;

/** Resolve the published profile and check it against the canonical source table. */
export async function loadProjectorPalette() {
    const xhtml = await readFile(resolve(workspaceRoot, 'packages/cem-theme/dist/lib/tokens/cem-colors.xhtml'), 'utf8');
    const catalog = JSON.parse(await readFile(resolve(workspaceRoot, 'packages/cem-theme/dist/lib/tokens/cem.tokens.catalog.json'), 'utf8'));
    return verifyProjectorPalette(xhtml, catalog);
}
export function verifyProjectorPalette(xhtml, catalog) {
    assert.deepEqual(catalog.profiles?.projector, { mode: 'light', theme: 'cem-theme-projector' });
    const source = Object.fromEntries((extractTable(xhtml, 'cem-projector-overrides') ?? []).map(row => [row[0], row[1]]));
    const bindings = {};
    for (const [name, value] of Object.entries(source)) {
        if (!/^#[\da-f]{6}$/i.test(value)) continue;
        const exported = catalog.tokens.find(token => token.name === name)?.profiles?.projector;
        assert.equal(exported?.toUpperCase(), value.toUpperCase(), `Projector palette drift: ${name}`);
        bindings[name] = value.slice(1).toUpperCase();
    }
    const get = name => { assert.ok(bindings[name], `Missing canonical projector token ${name}`); return bindings[name]; };
    const colors = { canvas: get('--cem-palette-comfort'), ink: get('--cem-palette-comfort-text'),
        secondary: get('--cem-palette-conservative-x'), neutral: get('--cem-palette-conservative'),
        primary: get('--cem-palette-creativity'), trust: get('--cem-palette-trust'), calm: get('--cem-palette-calm'),
        enthusiasm: get('--cem-palette-enthusiasm'), error: get('--cem-palette-danger'), stroke: get('--cem-separator-color'),
        onAccent: get('--cem-palette-creativity-text') };
    const checks = [];
    for (const background of [colors.canvas, colors.neutral]) {
        for (const foreground of [colors.ink, colors.secondary, colors.primary, colors.trust, colors.calm, colors.enthusiasm, colors.error]) {
            const ratio = contrast(foreground, background); assert.ok(ratio >= 7, `${foreground}/${background}: ${ratio}`);
            checks.push({ foreground, background, ratio });
        }
    }
    for (const background of [colors.primary, colors.trust, colors.calm, colors.enthusiasm, colors.error]) {
        const ratio = contrast(colors.onAccent, background); assert.ok(ratio >= 7);
        checks.push({ foreground: colors.onAccent, background, ratio });
    }
    return { colors, bindings, checks, sourceSha256: presenterHash(xhtml) };
}

// Use absolute transformed bounds to pair separately authored labels with their
// preceding panel fills, including grouped elements. Never mutate geometry.
function transform(element, parent) {
    const t = element.transform;
    const unit = t?.unit === 'PT' ? 12700 : 1;
    const local = t ? [t.scaleX ?? 0, t.shearY ?? 0, t.shearX ?? 0, t.scaleY ?? 0,
        (t.translateX ?? 0) * unit, (t.translateY ?? 0) * unit] : [1, 0, 0, 1, 0, 0];
    const [a, b, c, d, e, f] = parent, [g, h, i, j, k, l] = local;
    return [a * g + c * h, b * g + d * h, a * i + c * j, b * i + d * j, a * k + c * l + e, b * k + d * l + f];
}
function bounds(element, t) {
    const size = key => (element.size?.[key]?.magnitude ?? 0) * (element.size?.[key]?.unit === 'PT' ? 12700 : 1);
    const w = size('width'), h = size('height');
    const points = [[0, 0], [w, 0], [0, h], [w, h]].map(([x, y]) => [t[0] * x + t[2] * y + t[4], t[1] * x + t[3] * y + t[5]]);
    return { x: Math.min(...points.map(p => p[0])), y: Math.min(...points.map(p => p[1])),
        right: Math.max(...points.map(p => p[0])), bottom: Math.max(...points.map(p => p[1])) };
}
function contains(panel, text) {
    const cx = (text.x + text.right) / 2, cy = (text.y + text.bottom) / 2;
    return cx >= panel.x && cx <= panel.right && cy >= panel.y && cy <= panel.bottom
        && text.right - text.x <= (panel.right - panel.x) * 1.05 && text.bottom - text.y <= (panel.bottom - panel.y) * 1.05;
}

export function planProjectorPalette(raw, slideIds, palette) {
    const deck = unwrap(raw), c = palette.colors;
    assert.equal(deck.presentationId, presentationId); assert.ok(deck.revisionId, 'A fresh revision is required');
    assert.deepEqual(deck.slides.map(slide => slide.objectId), slideIds, 'Live slide order changed');
    const requests = [], patches = [], textChecks = [];
    const add = (request, path, value) => { requests.push(request); patches.push({ path, value }); };
    const colorMap = { F1FEFE: c.canvas, F1FAFA: c.canvas, '001010': c.ink, '16343B': c.ink,
        '1A1C18': c.secondary, '506A70': c.secondary, F1F1EB: c.neutral, FFFFFF: c.canvas,
        E1BEE7: c.primary, EEE7F7: c.primary, D7E3FF: c.trust, D6EFED: c.trust,
        '6A1B9A': c.primary, '6333A6': c.primary, '002F65': c.trust, '006A6A': c.calm, '087E83': c.calm,
        BA1A1A: c.error, '9F334A': c.error, C6DCDC: c.stroke };
    const schemes = { DARK1: c.ink, LIGHT1: c.canvas, DARK2: c.secondary, LIGHT2: c.neutral,
        ACCENT1: c.primary, ACCENT2: c.trust, ACCENT3: c.calm, ACCENT4: c.enthusiasm,
        ACCENT5: c.error, ACCENT6: c.stroke, HYPERLINK: c.trust, FOLLOWED_HYPERLINK: c.primary };
    for (const [i, master] of (deck.masters ?? []).entries()) {
        const old = master.pageProperties?.colorScheme;
        if (!old) continue;
        const colors = old.colors.map(pair => ({ ...pair, color: schemes[pair.type] ? rgb(schemes[pair.type]) : pair.color }));
        if (colors.every((pair, index) => hex(pair.color) === hex(old.colors[index].color))) continue;
        const value = { colors };
        add({ updatePageProperties: { objectId: master.objectId, pageProperties: { colorScheme: value }, fields: 'colorScheme' } }, ['masters', i, 'pageProperties', 'colorScheme'], value);
    }
    for (const [index, slide] of deck.slides.entries()) {
        if (slide.objectId === titleId) continue;
        const base = ['slides', index];
        const oldBg = slide.pageProperties?.pageBackgroundFill?.solidFill?.color?.rgbColor;
        if (!oldBg || hex(oldBg) !== c.canvas) {
            const value = { solidFill: { color: { rgbColor: rgb(c.canvas) }, alpha: 1 } };
            add({ updatePageProperties: { objectId: slide.objectId, pageProperties: { pageBackgroundFill: value }, fields: 'pageBackgroundFill' } }, [...base, 'pageProperties', 'pageBackgroundFill'], value);
        }
        const flat = [];
        const collect = (elements, path, parent) => elements.forEach((element, i) => {
            const here = [...path, i], t = transform(element, parent);
            if (element.elementGroup) collect(element.elementGroup.children, [...here, 'elementGroup', 'children'], t);
            else flat.push({ element, here, box: bounds(element, t) });
        });
        collect(slide.pageElements, [...base, 'pageElements'], [1, 0, 0, 1, 0, 0]);
        const panels = [];
        for (const { element, here, box } of flat) {
            const shape = element.shape;
            const fill = shape?.shapeProperties?.shapeBackgroundFill;
            const old = fill?.solidFill?.color?.rgbColor;
            const original = old && hex(old);
            const fillColor = original && (diagnosticIds.has(element.objectId) ? original : colorMap[original] ?? original);
            const painted = fill?.propertyState !== 'NOT_RENDERED' && fillColor && (fill.solidFill.alpha ?? 1) > .95;
            if (painted) {
                if (fillColor !== original) {
                    const value = { ...fill, solidFill: { ...fill.solidFill, color: { rgbColor: rgb(fillColor) } } };
                    add({ updateShapeProperties: { objectId: element.objectId, shapeProperties: { shapeBackgroundFill: value }, fields: 'shapeBackgroundFill' } }, [...here, 'shape', 'shapeProperties', 'shapeBackgroundFill'], value);
                }
                panels.push({ box, color: fillColor });
            }
            if (element.image || diagnosticIds.has(element.objectId)) continue;
            const surface = panels.findLast(panel => contains(panel.box, box))?.color ?? c.canvas;
            for (const [j, run] of (shape?.text?.textElements ?? []).entries()) {
                const text = run.textRun;
                if (!text?.content || run.endIndex <= (run.startIndex ?? 0)) continue;
                const ink = text.style?.foregroundColor?.opaqueColor;
                const originalInk = ink?.rgbColor && hex(ink.rgbColor);
                let color = originalInk ? colorMap[originalInk] ?? originalInk : schemes[ink?.themeColor] ?? c.ink;
                if (text.style?.link?.url) color = c.trust;
                if (contrast(color, surface) < 7) color = contrast(c.ink, surface) >= 7 ? c.ink : c.onAccent;
                if (text.content.trim()) {
                    const ratio = contrast(color, surface); assert.ok(ratio >= 7, `${element.objectId}: ${ratio}`);
                    textChecks.push({ slide: slide.objectId, objectId: element.objectId, foreground: color, background: surface, ratio });
                }
                if (originalInk === color) continue;
                const value = { opaqueColor: { rgbColor: rgb(color) } };
                add({ updateTextStyle: { objectId: element.objectId, textRange: { type: 'FIXED_RANGE', startIndex: run.startIndex ?? 0, endIndex: run.endIndex }, style: { foregroundColor: value }, fields: 'foregroundColor' } }, [...here, 'shape', 'text', 'textElements', j, 'textRun', 'style', 'foregroundColor'], value);
            }
            for (const [kind, value] of [['line', element.line?.lineProperties?.lineFill], ['outline', shape?.shapeProperties?.outline?.outlineFill]]) {
                const old = value?.solidFill?.color?.rgbColor;
                if (!old) continue;
                const original = hex(old), color = colorMap[original] ?? original;
                if (original === color && (value.solidFill.alpha ?? 1) === 1) continue;
                const next = { ...value, solidFill: { ...value.solidFill, color: { rgbColor: rgb(color) }, alpha: 1 } };
                if (kind === 'line') add({ updateLineProperties: { objectId: element.objectId, lineProperties: { lineFill: next }, fields: 'lineFill' } }, [...here, 'line', 'lineProperties', 'lineFill'], next);
                else add({ updateShapeProperties: { objectId: element.objectId, shapeProperties: { outline: { outlineFill: next } }, fields: 'outline.outlineFill' } }, [...here, 'shape', 'shapeProperties', 'outline', 'outlineFill'], next);
            }
        }
    }
    return { presentationId, profile: 'projector', revisionId: deck.revisionId, requests, patches, textChecks };
}

export function verifyProjectorReadback(before, after, plan) {
    const receipt = verifyPaletteReadback(before, after, plan);
    return { ...receipt, checkedDate: '2026-11-02', profile: 'projector', baseMode: 'light', diagnosticPanelPreserved: true,
        textChecks: plan.textChecks, minimumNativeTextContrast: Math.min(...plan.textChecks.map(check => check.ratio)) };
}

/** Permit only the reviewed source-image replacements and Google's subpixel fit rounding. */
export function verifyProjectorImages(beforeRaw, afterRaw, evidence) {
    const expected = structuredClone(unwrap(beforeRaw)), after = unwrap(afterRaw);
    assert.equal(evidence.profile, 'projector');
    assert.equal(evidence.sourceTextPreserved, true);
    assert.equal(evidence.dimensionsPreserved, true);
    const assets = new Map(evidence.assets.map(asset => [asset.name, asset]));
    const collect = deck => {
        const result = new Map();
        const visit = elements => elements.forEach(element => {
            if (element.elementGroup) visit(element.elementGroup.children);
            if (element.image) result.set(element.objectId, element);
        });
        deck.slides.forEach(slide => visit(slide.pageElements)); return result;
    };
    const actual = collect(after), replaced = [];
    for (const [id, element] of collect(expected)) {
        const name = element.description?.replace(/ /g, '-').replace(/\.png$/, '');
        if (!assets.has(name)) continue;
        const next = actual.get(id);
        assert.ok(next?.image, `Missing source image ${id}`);
        assert.deepEqual(next.size, element.size, `${id}: image size changed`);
        assert.deepEqual(next.image.imageProperties, element.image.imageProperties, `${id}: image properties changed`);
        const oldBounds = bounds(element, transform(element, [1, 0, 0, 1, 0, 0]));
        const newBounds = bounds(next, transform(next, [1, 0, 0, 1, 0, 0]));
        for (const key of Object.keys(oldBounds)) assert.ok(Math.abs(oldBounds[key] - newBounds[key]) <= 12.7, `${id}: image frame moved`);
        for (const key of ['shearX', 'shearY']) assert.equal(next.transform?.[key] ?? 0, element.transform?.[key] ?? 0, `${id}: image rotated`);
        assert.ok(next.image.sourceUrl && next.image.sourceUrl !== element.image.sourceUrl, `${id}: image not replaced`);
        element.transform = next.transform;
        element.image.sourceUrl = next.image.sourceUrl;
        replaced.push({ objectId: id, asset: name, sha256: assets.get(name).sha256 });
    }
    assert.ok(replaced.length > 0, 'No reviewed images replaced');
    verifyPaletteReadback(expected, after, { patches: [], requests: [] });
    return { replaced, exactImageReadback: true, sourceTextPreserved: true,
        sourceImageDimensionsPreserved: true, maximumFrameRoundingPt: .001,
        suppliedArtworkAndQRPreserved: true, notesAndNativeContentPreserved: true };
}
