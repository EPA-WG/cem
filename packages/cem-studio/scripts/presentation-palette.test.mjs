import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { planPalette, verifyPaletteTokens, verifyPaletteReadback, contrast } from './presentation-palette.mjs';
import { presentationId } from './presentation-deck-notes.mjs';
import { workspaceRoot } from './presenter-project.mjs';

const ids = ['p9', 'h36c7c826c4eee100_0_0', 'p24'];
const rgb = { red: .388235294, green: .2, blue: .650980392 };
function makeDeck() {
    return { presentationId, revisionId: 'fresh', slides: ids.map(id => ({ objectId: id,
        pageProperties: { pageBackgroundFill: { solidFill: { color: { rgbColor: { red: 1, green: 1, blue: 1 } }, alpha: 1 } } },
        slideProperties: { notesPage: { originalNotes: 'Keep privacy and user notes' } },
        pageElements: [{ objectId: id + '-text', shape: { text: { textElements: [{ startIndex: 0, endIndex: 11,
            textRun: { content: 'Keep source', style: { bold: true, foregroundColor: { opaqueColor: { rgbColor: rgb } } } } }] } } },
        { objectId: id + '-image', image: { contentUrl: 'temporary', sourceUrl: 'source' }, transform: { scaleX: 1 } }] })) };
}
function apply(before, plan) {
    const after = structuredClone(before); after.revisionId = 'new';
    for (const { path, value } of plan.patches) { let node = after; for (const key of path.slice(0, -1)) node = node[key]; node[path.at(-1)] = value; }
    return after;
}
test('canonical palette pairs meet 4.5 contrast and agree with source tokens', async () => {
    const result = await verifyPaletteTokens(); assert.ok(result.checks.length >= 17);
    assert.ok(contrast('001010', 'F1FEFE') > 18);
});
test('refuses changed canonical token values', async () => {
    const source = await readFile(resolve(workspaceRoot, 'packages/cem-theme/src/lib/tokens/cem-colors.md'), 'utf8');
    await assert.rejects(verifyPaletteTokens(source.replace('#f1fefe', '#ffffff')), /Canonical palette changed/);
});
test('color-only readback preserves notes, original title, image, text and geometry', () => {
    const before = makeDeck(), plan = planPalette(before, ids), after = apply(before, plan);
    assert.equal(verifyPaletteReadback(before, after, plan).titlePreserved, true);
    assert.ok(plan.patches.every(patch => patch.path[1] !== 1));
    assert.deepEqual(after.slides[1], before.slides[1]);
    assert.equal(planPalette(after, ids).requests.length, 0);
});
test('refuses changed slide order before writing', () => {
    assert.throws(() => planPalette(makeDeck(), [...ids].reverse()), /order changed/);
});
test('normalizes merged runs and nonpainting newline color without losing visible emphasis', () => {
    const before = makeDeck(), elements = before.slides[0].pageElements[0].shape.text.textElements;
    elements.push({ startIndex: 11, endIndex: 12, textRun: { content: '\n', style: { ...elements[0].textRun.style, foregroundColor: { opaqueColor: { themeColor: 'DARK1' } } } } });
    const plan = planPalette(before, ids), after = apply(before, plan), runs = after.slides[0].pageElements[0].shape.text.textElements;
    runs[0].textRun.content += '\n'; runs[0].endIndex++; runs.pop();
    assert.equal(verifyPaletteReadback(before, after, plan).exactColorReadback, true);
    runs[0].textRun.style.bold = false;
    assert.throws(() => verifyPaletteReadback(before, after, plan), /reviewed color-only/);
});
test('maps hyperlink paint while preserving its target and mixed styling', () => {
    const before = makeDeck(), run = before.slides[0].pageElements[0].shape.text.textElements[0].textRun;
    run.style.foregroundColor = { opaqueColor: { themeColor: 'HYPERLINK' } }; run.style.link = { url: 'https://github.com/EPA-WG/cem/discussions/24' };
    const plan = planPalette(before, ids), after = apply(before, plan);
    assert.equal(verifyPaletteReadback(before, after, plan).exactColorReadback, true);
    assert.deepEqual(after.slides[0].pageElements[0].shape.text.textElements[0].textRun.style.link, run.style.link);
    assert.equal(after.slides[0].pageElements[0].shape.text.textElements[0].textRun.style.bold, true);
});
for (const [name, mutate] of [
    ['notes', after => { after.slides[0].slideProperties.notesPage.originalNotes = 'lost'; }],
    ['source text', after => { after.slides[0].pageElements[0].shape.text.textElements[0].textRun.content = 'lost'; }],
    ['image source', after => { after.slides[0].pageElements[1].image.sourceUrl = 'lost'; }],
    ['geometry', after => { after.slides[0].pageElements[1].transform.scaleX = 2; }],
    ['title style', after => { after.slides[1].pageElements[0].shape.text.textElements[0].textRun.style.bold = false; }],
]) test(`refuses unrelated ${name} changes`, () => {
    const before = makeDeck(), plan = planPalette(before, ids), after = apply(before, plan); mutate(after);
    assert.throws(() => verifyPaletteReadback(before, after, plan), /reviewed color-only/);
});
