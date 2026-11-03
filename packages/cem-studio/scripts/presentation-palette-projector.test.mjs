import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { loadProjectorPalette, verifyProjectorPalette, planProjectorPalette, verifyProjectorReadback, verifyProjectorImages } from './presentation-palette-projector.mjs';
import { presentationId } from './presentation-deck-notes.mjs';
import { workspaceRoot } from './presenter-project.mjs';

const rgb = value => Object.fromEntries(['red', 'green', 'blue'].map((k, i) => [k, parseInt(value.slice(i * 2, i * 2 + 2), 16) / 255]));
const element = (id, fill, text) => ({ objectId: id, size: { width: { magnitude: 100, unit: 'PT' }, height: { magnitude: 50, unit: 'PT' } },
    transform: { scaleX: 1, scaleY: 1, unit: 'PT' }, shape: { shapeProperties: { shapeBackgroundFill: fill ? { solidFill: { color: { rgbColor: rgb(fill) }, alpha: 1 } } : { propertyState: 'NOT_RENDERED' } },
        text: text ? { textElements: [{ startIndex: 0, endIndex: text.length, textRun: { content: text, style: { bold: true, foregroundColor: { opaqueColor: { rgbColor: rgb('001010') } }, link: { url: 'https://example.com/retained' } } } }] } : undefined } });
function fixture() {
    return { presentationId, revisionId: 'fresh', slides: [
        { objectId: 'p2', pageProperties: {}, pageElements: [element('panel', 'E1BEE7'), element('label', null, 'Original label')] },
        { objectId: 'h36c7c826c4eee100_0_0', pageElements: [element('title', '001010', 'Preserve title')] },
        { objectId: 'p9', pageProperties: {}, pageElements: [element('p9_i10', '001010'), element('p9_i11', null, 'Original diagnostic')] },
    ] };
}
function apply(before, plan) {
    const after = structuredClone(before); after.revisionId = 'after';
    for (const { path, value } of plan.patches) { let owner = after; for (const key of path.slice(0, -1)) owner = owner[key]; owner[path.at(-1)] = value; }
    return after;
}
test('projector uses the published canonical palette and rejects export/source drift', async () => {
    const palette = await loadProjectorPalette(); assert.equal(palette.colors.canvas, 'FFFFFF'); assert.ok(palette.checks.every(check => check.ratio >= 7));
    const xhtml = await readFile(resolve(workspaceRoot, 'packages/cem-theme/dist/lib/tokens/cem-colors.xhtml'), 'utf8');
    const catalog = JSON.parse(await readFile(resolve(workspaceRoot, 'packages/cem-theme/dist/lib/tokens/cem.tokens.catalog.json'), 'utf8'));
    catalog.tokens.find(token => token.name === '--cem-palette-creativity').profiles.projector = '#ffffff';
    assert.throws(() => verifyProjectorPalette(xhtml, catalog), /palette drift/);
});
test('preserves colored panels and pairs separately authored labels with white ink', async () => {
    const before = fixture(), palette = await loadProjectorPalette();
    const ids = before.slides.map(slide => slide.objectId), plan = planProjectorPalette(before, ids, palette), after = apply(before, plan);
    assert.deepEqual(after.slides[0].pageElements[0].shape.shapeProperties.shapeBackgroundFill.solidFill.color.rgbColor, rgb(palette.colors.primary));
    const run = after.slides[0].pageElements[1].shape.text.textElements[0].textRun;
    assert.deepEqual(run.style.foregroundColor.opaqueColor.rgbColor, rgb('FFFFFF'));
    assert.deepEqual(run.style.link, before.slides[0].pageElements[1].shape.text.textElements[0].textRun.style.link);
    assert.equal(run.style.bold, true);
    assert.deepEqual(after.slides[1], before.slides[1]);
    assert.deepEqual(after.slides[2].pageElements, before.slides[2].pageElements);
    assert.equal(verifyProjectorReadback(before, after, plan).exactColorReadback, true);
    assert.equal(planProjectorPalette(after, ids, palette).requests.length, 0);
    assert.throws(() => planProjectorPalette(before, [...ids].reverse(), palette), /order changed/);
    after.slides[0].pageElements[1].shape.text.textElements[0].textRun.content = 'Unrelated edit';
    assert.throws(() => verifyProjectorReadback(before, after, plan), /reviewed color-only/);
});
test('a non-rendered panel does not force white labels and grouped fills retain their context', async () => {
    const palette = await loadProjectorPalette(), before = fixture(), ids = before.slides.map(slide => slide.objectId);
    before.slides[0].pageElements[0].shape.shapeProperties.shapeBackgroundFill.propertyState = 'NOT_RENDERED';
    let plan = planProjectorPalette(before, ids, palette);
    assert.equal(plan.textChecks[0].background, 'FFFFFF');
    before.slides[0].pageElements[0].shape.shapeProperties.shapeBackgroundFill.propertyState = 'RENDERED';
    before.slides[0].pageElements = [{ objectId: 'group', transform: { scaleX: 2, scaleY: 2, translateX: 200, unit: 'PT' }, elementGroup: { children: before.slides[0].pageElements } }];
    plan = planProjectorPalette(before, ids, palette);
    assert.equal(plan.textChecks[0].background, palette.colors.primary);
    assert.equal(plan.textChecks[0].foreground, 'FFFFFF');
});

test('native scheme readback accounts for Google derived aliases and refuses unrelated scheme drift', async () => {
    const before = fixture(), palette = await loadProjectorPalette();
    before.masters = [{ objectId: 'master', pageProperties: { colorScheme: { colors: [
        { type: 'LIGHT2', color: rgb('E7E6E6') }, { type: 'TEXT2', color: rgb('E7E6E6') },
    ] } } }];
    const plan = planProjectorPalette(before, before.slides.map(slide => slide.objectId), palette), after = apply(before, plan);
    after.masters[0].pageProperties.colorScheme.colors[1].color = rgb(palette.colors.neutral);
    assert.equal(verifyProjectorReadback(before, after, plan).exactColorReadback, true);
    after.masters[0].pageProperties.colorScheme.colors[0].color = rgb('FF0000');
    assert.throws(() => verifyProjectorReadback(before, after, plan), /reviewed color-only/);
});

test('source-image replacement retains metadata, source evidence, framing and all other native content', () => {
    const before = fixture();
    before.slides[0].pageElements.push({ objectId: 'source-image', description: 'markup cem.png',
        size: { width: { magnitude: 100, unit: 'PT' }, height: { magnitude: 50, unit: 'PT' } },
        transform: { scaleX: 1, scaleY: 1, unit: 'PT' }, image: { sourceUrl: 'https://example.com/old', imageProperties: {} } });
    const after = structuredClone(before); after.revisionId = 'new';
    after.slides[0].pageElements[2].image.sourceUrl = 'https://example.com/projector';
    const evidence = { profile: 'projector', sourceTextPreserved: true, dimensionsPreserved: true, assets: [{ name: 'markup-cem', sha256: 'reviewed-image-bytes' }] };
    assert.equal(verifyProjectorImages(before, after, evidence).replaced.length, 1);
    const broken = structuredClone(after); delete broken.slides[0].pageElements[2].description;
    assert.throws(() => verifyProjectorImages(before, broken, evidence), /reviewed color-only/);
    after.slides[0].pageElements[2].transform.translateX = 2;
    assert.throws(() => verifyProjectorImages(before, after, evidence), /frame moved/);
});
