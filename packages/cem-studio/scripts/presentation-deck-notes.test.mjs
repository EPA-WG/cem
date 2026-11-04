import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { planDeckNotes, verifyDeckNotes, presentationId } from './presentation-deck-notes.mjs';
import { loadPresenterResources } from './presentation-resources.mjs';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';

const notes = await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/presenter-notes.md'), 'utf8');
const data = await loadPresenterResources();
const ids = notes.split('\n').filter(line => /^\| \d{2} \|/.test(line)).map(line => line.split('|')[2].trim().replaceAll('`', ''));
const makeDeck = () => ({ presentationId, revisionId: 'fresh-revision', title: 'User-edited title', slides: ids.map(id => ({
    objectId: id, pageElements: [{ objectId: id + '-visible', description: 'Keep original visual' }],
    slideProperties: { notesPage: { notesProperties: { speakerNotesObjectId: id + '-notes' },
        pageElements: [{ objectId: id + '-notes', shape: { text: { textElements: [{ textRun: { content: 'Original user notes 🍒\n' } }] } } }] } },
})) });
function apply(deck, plan) {
    const result = structuredClone(deck); result.revisionId = 'renewed-revision';
    for (const entry of plan.entries.filter(entry => entry.insert || entry.replaceLength)) {
        const target = result.slides.find(slide => slide.objectId === entry.slideId).slideProperties.notesPage.pageElements[0].shape.text;
        const original = target.textElements.map(element => element.textRun?.content ?? '').join('');
        target.textElements = [{ textRun: { content: entry.block + original.slice(entry.replaceLength ?? 0) } }];
    }
    return result;
}
test('notes-only insertion preserves original notes and visuals, reads exact links and is idempotent', () => {
    const before = makeDeck(), plan = planDeckNotes(before, data, notes), after = apply(before, plan);
    assert.equal(plan.requests.length, 30); assert.equal(verifyDeckNotes(before, after, plan).slides, 30);
    assert.equal(planDeckNotes(after, data, notes).requests.length, 0);
    assert.ok(plan.entries.every(entry => data.publicResources.every(resource => entry.block.includes(resource.url))));
});
for (const [name, mutate, pattern] of [
    ['slide order drift', deck => deck.slides.reverse(), /order changed/],
    ['missing note owner', deck => { delete deck.slides[0].slideProperties.notesPage.notesProperties.speakerNotesObjectId; }, /Missing or duplicate/],
    ['duplicate note owner', deck => { deck.slides[1].slideProperties.notesPage.notesProperties.speakerNotesObjectId = ids[0] + '-notes'; }, /Missing or duplicate/],
    ['missing revision', deck => { delete deck.revisionId; }, /revision guard/],
]) test(`refuses ${name}`, () => {
    const deck = makeDeck(); mutate(deck); assert.throws(() => planDeckNotes(deck, data, notes), pattern);
});
test('refuses changed visible content during readback', () => {
    const before = makeDeck(), plan = planDeckNotes(before, data, notes), after = apply(before, plan);
    after.slides[0].pageElements[0].description = 'Changed';
    assert.throws(() => verifyDeckNotes(before, after, plan), /content, order/);
});
test('allows renewed image download URLs but refuses a changed image source', () => {
    const before = makeDeck(); before.slides[0].pageElements.push({ objectId: 'photo', image: { contentUrl: 'temporary-before', sourceUrl: 'original-source' } });
    const plan = planDeckNotes(before, data, notes), after = apply(before, plan);
    after.slides[0].pageElements[1].image.contentUrl = 'temporary-after';
    assert.equal(verifyDeckNotes(before, after, plan).visibleContentUnchanged, true);
    after.slides[0].pageElements[1].image.sourceUrl = 'different-source';
    assert.throws(() => verifyDeckNotes(before, after, plan), /content, order/);
});
test('refuses lost original notes or altered public links', () => {
    const before = makeDeck(), plan = planDeckNotes(before, data, notes), after = apply(before, plan);
    after.slides[0].slideProperties.notesPage.pageElements[0].shape.text.textElements[0].textRun.content = plan.entries[0].block;
    assert.throws(() => verifyDeckNotes(before, after, plan), /exact readback/);
    const changed = apply(before, plan);
    changed.slides[0].slideProperties.notesPage.pageElements[0].shape.text.textElements[0].textRun.content += 'Unreviewed';
    assert.throws(() => verifyDeckNotes(before, changed, plan), /exact readback/);
});
test('refuses silently replacing a previously inserted operational block', () => {
    const before = makeDeck(), plan = planDeckNotes(before, data, notes), after = apply(before, plan);
    assert.throws(() => planDeckNotes(after, data, notes.replace('DNA metaphor', 'different metaphor')), /review before updating/);
});

test('refresh requires exact reviewed prior blocks and preserves original note tails', () => {
    const first = planDeckNotes(makeDeck(), data, notes);
    const before = apply(makeDeck(), first);
    const reviewed = first.entries.map(entry => ({ slideId: entry.slideId, owner: entry.owner, sha256: presenterHash(entry.block) }));
    const changed = notes.replace('DNA metaphor', 'DNA example');
    const plan = planDeckNotes(before, data, changed, reviewed);
    assert.equal(plan.requests.length, 2);
    assert.equal(plan.requests[0].deleteText.textRange.endIndex, first.entries[0].block.length);
    const after = apply(before, plan);
    assert.equal(verifyDeckNotes(before, after, plan).refreshedBlocks, 1);
    assert.equal(planDeckNotes(after, data, changed).requests.length, 0);
    const unreviewed = structuredClone(before);
    unreviewed.slides[0].slideProperties.notesPage.pageElements[0].shape.text.textElements[0].textRun.content = 'Unexpected user change\n' + first.entries[0].block;
    assert.throws(() => planDeckNotes(unreviewed, data, changed, reviewed), /marker location/);
    reviewed[0].sha256 = 'wrong';
    assert.throws(() => planDeckNotes(before, data, changed, reviewed), /review before updating/);
});
