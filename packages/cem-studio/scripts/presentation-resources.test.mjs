import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { loadPresenterResources, verifyPresenterResources } from './presentation-resources.mjs';
import { workspaceRoot } from './presenter-project.mjs';

test('participant resources and cues match frozen projects, recorded slide IDs and the 60-minute plan', async () => {
    const result = await verifyPresenterResources();
    assert.equal(result.plannedSeconds, 3600); assert.equal(result.cues, 12);
    assert.equal(result.discussionPosted, false); assert.equal(result.measuredRehearsal, false);
});
for (const [name, mutate, pattern] of [
    ['stale collection ID', data => { data.projects[0].collection = 'amsterdam'; }, /Unknown collection/],
    ['wrong printed route', data => { data.projects[0].route = '/?sample=login'; }, /route changed/],
    ['foreign resource', data => { data.cues[0].resource = 'supplier-stock'; }, /Unknown resource/],
    ['missing configuration', data => { data.cues[0].configurations = ['unavailable']; }, /Unknown configuration/],
    ['wrong native slide owner', data => { data.cues[0].slideId = 'p1'; }, /slide ID binding changed/],
    ['wrong fallback', data => { data.cues[0].fallback.path = data.cues[1].fallback.path; }, /different checkpoint/],
    ['unverified download URL', data => { data.publicResources[0].url = 'https://example.com/download.tgz'; }, /pinned GitHub/],
    ['weakened credential policy', data => { data.privacy.enteredCredentials = 'export'; }, /credential policy changed/],
]) test(`refuses ${name}`, async () => {
    const data = await loadPresenterResources(); mutate(data);
    await assert.rejects(verifyPresenterResources(data), pattern);
});
test('refuses timing drift without claiming a measured rehearsal', async () => {
    const notes = await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/presenter-notes.md'), 'utf8');
    await assert.rejects(verifyPresenterResources(await loadPresenterResources(), notes.replace('54:00–60:00', '54:00–61:00')), /order\/timing changed/);
});
test('refuses a broken local presenter reference', async () => {
    const notes = await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/presenter-notes.md'), 'utf8');
    await assert.rejects(verifyPresenterResources(await loadPresenterResources(), notes.replace('slide-story.md', 'missing-story.md')), /Local resource link missing/);
});
test('refuses cue prose changed after the recorded native notes readback', async () => {
    const notes = await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/presenter-notes.md'), 'utf8');
    await assert.rejects(verifyPresenterResources(await loadPresenterResources(), notes.replace('Explain the DNA metaphor.', 'Invent a different opening.')), /Synced cue table changed/);
});
