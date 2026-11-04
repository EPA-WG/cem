import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { loadPresenterResources, verifyPresenterResources, verifyDiscussionDemoDraft } from './presentation-resources.mjs';
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


const demoDraft = () => readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/discussion-resource-comment-draft.md'), 'utf8');
test('all 30 slides retain 14 frozen demos and reviewed references without claiming public acceptance', async () => {
    const result = await verifyDiscussionDemoDraft();
    assert.equal(result.links.length, 14); assert.equal(result.slideCount, 30);
    assert.equal(result.demoSlideCount, 13); assert.equal(result.referenceSlideCount, 17);
    assert.equal(result.publicFunctionalityVerified, false); assert.equal(result.discussionPosted, false);
    const query = result.links.find(link => link.cue === 'query');
    assert.equal(query.resource, 'login-cem'); assert.equal(query.configurationInput, 'login-html');
});
for (const [name, mutate, pattern] of [
    ['foreign host', text => text.replace('https://green-morning-fd6e.trans-acf.workers.dev/studio/', 'https://example.test/studio/'), /host\/subtree/],
    ['wrong Studio subtree', text => text.replace('/studio/?', '/other/?'), /host\/subtree/],
    ['stale slide ID', text => text.replace('| `p2` |', '| `p999` |'), /stable slide/],
    ['missing cue', text => text.split('\n').filter(line => !line.startsWith('| 08 |')).join('\n'), /every cue/],
    ['duplicate URL key', text => text.replace('collection=amsterdam-2026', 'collection=amsterdam-2026&collection=other'), /ambiguous/],
    ['foreign collection', text => text.replace('collection=amsterdam-2026', 'collection=other'), /collection/],
    ['wrong resource', text => text.replace('file=readme', 'file=login-cem'), /viewed resource/],
    ['wrong command', text => text.replace('build=read-edit', 'build=validate'), /entry configuration/],
    ['omitted alternate configuration', text => text.replace('— `format-pretty`, `format-tabular`, `source-colors`', '— `format-pretty`'), /advertised configurations/],
    ['weakened credential policy', text => text.replace('block recovery until explicitly cleared', 'allow recovery immediately'), /publication\/privacy/],
    ['relative footer URL', text => text.replace(/https:\/\/github\.com\/EPA-WG\/cem\/blob\/[^)]+\/participant-resources\.md/g, 'participant-resources.md'), /pinned public footer/],
    ['unreviewed destination', text => text + '\n[Download](https://example.test/kit.tgz)\n', /unreviewed destination/],
    ['missing reference slide', text => text.split('\n').filter(line => !line.startsWith('| 02 |')).join('\n'), /every remaining slide/],
    ['duplicate reference slide', text => text + '\n' + text.split('\n').find(line => line.startsWith('| 02 |')), /every remaining slide/],
    ['wrong reference native ID', text => text.replace('| 25 | `p27` |', '| 25 | `p28` |'), /Reference native slide ID/],
    ['wrong source anchor', text => text.replace('#six-resource-layers)', '#imaginary-layer)'), /Reference destination/],
    ['unreviewed source revision', text => text.replace(/blob\/bc51404e[^/]+\/docs\/declarative-amsterdam-2026\/opening-teaser/, 'blob/develop/docs/declarative-amsterdam-2026/opening-teaser'), /Reference destination/],
    ['omitted published example', text => text.split('\n').filter(line => !line.startsWith('- Slide 08: [Hello, CEM-ML]')).join('\n'), /Previously published example/],
    ['premature posting claim', text => text.replace(/No comment\s+has been sent\./, 'The comment was posted.'), /publication\/privacy/],
]) test(`demo draft refuses ${name}`, async () => {
    await assert.rejects(verifyDiscussionDemoDraft(undefined, mutate(await demoDraft())), pattern);
});
