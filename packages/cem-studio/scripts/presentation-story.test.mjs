import assert from 'node:assert/strict';
import { test } from 'node:test';
import { loadStoryMap, verifyStoryMap } from './presentation-story.mjs';

test('all manifesto and pitch sections map to native slides, six website sections and evidence', async () => {
    const result = await verifyStoryMap(); assert.equal(result.principles, 14); assert.equal(result.pitchSections, 7); assert.equal(result.plannedMinutes, 60);
});
for (const [name, mutate, pattern] of [
    ['omitted principle', data => { data.mappings.shift(); }, /coverage incomplete/],
    ['stale slide ID', data => { data.mappings[0].slides = ['deleted-slide']; }, /native slide owner/],
    ['changed question period', data => { data.blocks.at(-1).minutes = [54, 65]; }, /intervals changed/],
    ['missing website section', data => { data.mappings[0].websiteHeading = 'Not published'; }, /website story/],
    ['missing evidence', data => { data.mappings[0].evidence = 'missing-proof.md'; }, /ENOENT/],
    ['promoted insulation claim', data => { data.mappings.find(row => row.heading === 'Insulate every included scope').status = 'demonstrated'; }, /architectural target/],
    ['invented public hosting', data => { data.publication = 'publicly-hosted'; }, /Assertion/],
]) test(`refuses ${name}`, async () => {
    const data = await loadStoryMap(); mutate(data); await assert.rejects(verifyStoryMap(data), pattern);
});
