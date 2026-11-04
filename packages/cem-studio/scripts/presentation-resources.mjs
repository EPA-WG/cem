import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { access, mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';
import { amsterdamStudioUrl } from './studio-acceptance-url.mjs';
import { readLoginPresenter } from './login-presenter.mjs';
import { readSupplierPresenter } from './supplier-presenter.mjs';
import { workspaceRoot, presenterPrivacy, presenterFile, presenterHash } from './presenter-project.mjs';

const docs = resolve(workspaceRoot, 'docs/declarative-amsterdam-2026');
const discussionSourceRevision = 'bc51404ef57639c4b627ce88f615b629cbfeb5e8';
export async function loadPresenterResources() {
    return JSON.parse(await readFile(resolve(docs, 'presenter-resources.json'), 'utf8'));
}
const rows = text => text.split('\n').filter(line => /^\| \d{2} \|/.test(line))
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()));
const seconds = value => {
    assert.match(value, /^\d{2}:\d{2}$/);
    const [minutes, seconds] = value.split(':').map(Number);
    assert.ok(seconds < 60); return minutes * 60 + seconds;
};

/** Validate a prepared comment's control links; this never establishes public functionality or posts. */
export async function verifyDiscussionDemoDraft(data, draft, { freezes, native } = {}) {
    data ??= await loadPresenterResources();
    draft ??= await readFile(resolve(docs, 'discussion-resource-comment-draft.md'), 'utf8');
    freezes ??= { login: await readLoginPresenter(), 'supplier-pipeline': await readSupplierPresenter() };
    native ??= rows(await readFile(resolve(docs, 'native-slide-order.md'), 'utf8'));
    const expected = [...data.cues.map(cue => ({ ...cue, name: cue.id })),
        { slide: 15, slideId: 'p14', name: 'companion pipeline overview', project: 'supplier-pipeline', resource: 'supplier-graph', configurations: ['availability'] },
        { slide: 18, slideId: 'p17', name: 'source provenance', project: 'login', resource: 'login-view-template', configurations: ['render-graph'] },
    ].sort((a, b) => a.slide - b.slide);
    const resources = JSON.parse(await readFile(resolve(docs, 'discussion-slide-resources.json'), 'utf8'));
    assert.equal(resources.schemaVersion, 1);
    assert.equal(resources.sourceRevision, discussionSourceRevision, 'Slide references must use the reviewed revision');
    const allRows = rows(draft);
    const demoSlides = new Set(expected.map(cue => cue.slide));
    const actual = allRows.filter(row => demoSlides.has(Number(row[0])));
    assert.equal(actual.length, expected.length, 'Demo draft must include every cue and companion row');
    const links = [];
    for (const [index, row] of actual.entries()) {
        const cue = expected[index];
        assert.equal(row.length, 4, 'Demo draft table shape changed');
        assert.equal(Number(row[0]), cue.slide, 'Demo draft slide order changed');
        assert.equal(row[1], '`' + cue.slideId + '`', 'Demo draft stable slide ID changed');
        assert.equal(native.find(slide => Number(slide[0]) === cue.slide)?.[1], row[1], 'Demo draft differs from recorded native slides');
        assert.equal(row[2], cue.name, 'Demo draft cue coverage/order changed');
        const link = row[3].match(/^\[([^\]]+)\]\(([^)]+)\) — (.+)$/);
        assert.ok(link, 'Demo draft must have one destination and explicit configurations');
        const target = new URL(link[2]);
        assert.equal(target.origin + target.pathname, amsterdamStudioUrl, 'Demo draft host/subtree changed');
        assert.equal(target.username, ''); assert.equal(target.password, ''); assert.equal(target.hash, '');
        assert.deepEqual([...target.searchParams.keys()].sort(), ['build', 'collection', 'file', 'sample'], 'Demo draft URL parameters are ambiguous');
        const project = data.projects.find(project => project.id === cue.project); assert.ok(project);
        assert.equal(target.searchParams.get('collection'), project.collection, 'Demo draft collection changed');
        assert.equal(target.searchParams.get('sample'), cue.project, 'Demo draft project changed');
        assert.equal(target.searchParams.get('file'), cue.resource, 'Demo draft viewed resource changed');
        assert.equal(target.searchParams.get('build'), cue.configurations[0], 'Demo draft entry configuration changed');
        const frozen = freezes[cue.project];
        assert.ok(frozen.bundle.project.resources.some(resource => resource.id === cue.resource), 'Demo draft resource is absent from frozen project');
        const configurations = [...link[3].matchAll(/`([^`]+)`/g)].map(match => match[1]);
        assert.deepEqual(configurations, cue.configurations, 'Demo draft advertised configurations changed');
        for (const id of configurations) assert.ok(frozen.definition.configurations.some(config => config.id === id), 'Demo draft configuration is absent from frozen project');
        links.push({ slide: cue.slide, slideId: cue.slideId, cue: cue.name, project: cue.project, resource: cue.resource,
            configuration: cue.configurations[0], configurationInput: frozen.definition.configurations.find(config => config.id === cue.configurations[0]).source,
            configurations, url: target.href });
    }
    const referenceFor = (path, fragment = '') => {
        const url = `https://github.com/EPA-WG/cem/blob/${discussionSourceRevision}/${path}${fragment ? '#' + fragment : ''}`;
        const bytes = execFileSync('git', ['show', `${discussionSourceRevision}:${path}`], { cwd: workspaceRoot });
        if (fragment) assert.ok(bytes.toString().split('\n').filter(line => /^#+ /.test(line)).some(line =>
            line.replace(/^#+ /, '').toLowerCase().replace(/[^\w\s-]/g, '').replace(/\s/g, '-') === fragment), 'Pinned source anchor is missing');
        return { path, url, revision: discussionSourceRevision, sha256: presenterHash(bytes) };
    };
    const references = ['participant-resources.md', 'public-demo-4d21f9cd-verification.md'].map(name => {
        const reference = referenceFor(`docs/declarative-amsterdam-2026/${name}`);
        const { url } = reference;
        assert.ok(draft.includes('](' + url + ')'), 'Demo draft must use pinned public footer links');
        return reference;
    });
    const referenceRows = allRows.filter(row => !demoSlides.has(Number(row[0])));
    assert.deepEqual(resources.slides.map(slide => slide.slide), native.map(row => Number(row[0])).filter(slide => !demoSlides.has(slide)), 'Reference map must cover the remaining native slides');
    assert.equal(referenceRows.length, resources.slides.length, 'Draft must include every remaining slide exactly once');
    for (const [index, row] of referenceRows.entries()) {
        const source = resources.slides[index], nativeRow = native.find(row => Number(row[0]) === source.slide);
        assert.equal(row.length, 4);
        assert.equal(Number(row[0]), source.slide, 'Reference slide order changed');
        assert.equal(row[1], nativeRow[1], 'Reference native slide ID changed');
        assert.equal('`' + source.slideId + '`', nativeRow[1], 'Reference map native ID changed');
        assert.equal(source.title, nativeRow[2], 'Reference map native title changed');
        const suffix = nativeRow[3] === 'Optional reference' ? ' (optional reference)' : ' (reference)';
        assert.equal(row[2], source.title + suffix, 'Reference slide title/role changed');
        const targets = source.resources.map(resource => ({ ...referenceFor(resource.path, resource.fragment), label: resource.label }));
        assert.equal(row[3], targets.map(target => `[${target.label}](${target.url})`).join('; '), 'Reference destination differs from reviewed source');
        references.push(...targets);
    }
    assert.deepEqual([...new Set(allRows.map(row => Number(row[0])))], native.map(row => Number(row[0])), 'Draft must cover all 30 slides in native order');
    for (const link of resources.publishedLinks) assert.ok(draft.includes(`[${link.label}](${link.url})`), 'Previously published example is missing');
    const allowed = new Set([data.resourceHub, ...resources.publishedLinks.map(link => link.url), ...links.map(link => link.url), ...references.map(reference => reference.url)]);
    for (const [, target] of draft.matchAll(/\]\(([^)]+)\)/g)) assert.ok(allowed.has(target), 'Demo draft contains an unreviewed destination');
    const prose = draft.replace(/\s+/g, ' ');
    for (const text of ['Do not post yet.', 'No comment has been sent.', 'Clone a sample before editing.',
        'Do not enter real credentials.', 'Entered passwords or one-time codes block recovery until explicitly cleared;', 'they must never be exported.']) {
        assert.ok(prose.includes(text), 'Demo draft publication/privacy boundary changed');
    }
    return { links, references, publishedLinks: resources.publishedLinks, slideCount: new Set(allRows.map(row => row[1])).size,
        demoSlideCount: new Set(links.map(link => link.slideId)).size, referenceSlideCount: referenceRows.length,
        publicFunctionalityVerified: false, discussionPosted: false, checks: 'frozen identities and recorded slide bindings only' };
}

/** Checks documentation/control identities only. Native trees stay in the shared engine. */
export async function verifyPresenterResources(data, notes) {
    data ??= await loadPresenterResources();
    assert.equal(data.schemaVersion, 1);
    assert.equal(data.nativeDeckNotes, 'synced-and-read-back');
    assert.equal(data.rehearsal, 'not-yet-measured');
    assert.match(data.referenceRevision, /^[0-9a-f]{40}$/);
    assert.equal(data.resourceHub, 'https://github.com/EPA-WG/cem/discussions/24');
    assert.deepEqual(data.privacy, presenterPrivacy, 'Presenter credential policy changed');
    const index = JSON.parse(await readFile(resolve(workspaceRoot, 'packages/cem-studio/generated/feature-tour/index.json'), 'utf8'));
    const freezes = { login: await readLoginPresenter(), 'supplier-pipeline': await readSupplierPresenter() };
    assert.deepEqual(data.projects.map(item => item.id), Object.keys(freezes));
    for (const project of data.projects) {
        const frozen = freezes[project.id];
        assert.equal(project.version, frozen.manifest.projectVersion);
        assert.ok(index.collections.some(item => item.id === project.collection), 'Unknown collection ID');
        assert.ok(index.projects.find(item => item.id === project.id).collections.includes(project.collection));
        assert.ok(frozen.bundle.project.resources.some(item => item.id === project.readme));
        assert.equal(project.route, '/?' + new URLSearchParams({ collection: project.collection, sample: project.id, file: project.readme }), 'Participant route changed');
        assert.deepEqual(JSON.parse(await readFile(presenterFile(workspaceRoot, project.freeze), 'utf8')), frozen.manifest);
    }
    const native = rows(await readFile(resolve(docs, 'native-slide-order.md'), 'utf8'));
    notes ??= await readFile(resolve(docs, 'presenter-notes.md'), 'utf8');
    const cues = rows(notes);
    assert.equal(native.length, 30); assert.equal(cues.length, 30);
    assert.deepEqual(cues.map(row => row.slice(0, 3)), native.map(row => [row[0], row[1], row[3]]), 'Recorded native slide order/timing changed');
    let elapsed = 0, timed = 0;
    for (const row of cues) {
        if (row[2] === 'Optional reference') continue;
        const [start, end] = row[2].split('–').map(seconds);
        assert.equal(start, elapsed, 'Presenter timeline is not contiguous'); assert.ok(end > start);
        elapsed = end; timed++;
    }
    assert.equal(timed, 26); assert.equal(elapsed, 3600);
    assert.equal(cues.at(-1)[2], '54:00–60:00', 'Keep the final question period');
    assert.equal(new Set(data.cues.map(cue => cue.id)).size, data.cues.length);
    for (const cue of data.cues) {
        const slide = cues.find(row => Number(row[0]) === cue.slide);
        assert.ok(slide, `Unknown slide: ${cue.id}`);
        assert.equal(slide[1], '`' + cue.slideId + '`', 'Presenter cue/native slide ID binding changed');
        assert.ok(notes.includes('`' + cue.id + '`'), `Missing speaking cue: ${cue.id}`);
        const project = data.projects.find(item => item.id === cue.project); assert.ok(project);
        const frozen = freezes[project.id];
        assert.ok(frozen.definition.sources.some(item => item.id === cue.resource), `Unknown resource: ${cue.id}`);
        for (const id of cue.configurations) assert.ok(frozen.definition.configurations.some(item => item.id === id), `Unknown configuration: ${id}`);
        const checkpoint = frozen.manifest.checkpoints.find(item => (item.id ?? item.scenario.id) === cue.fallback.checkpoint); assert.ok(checkpoint);
        assert.ok(['output', 'diagnostics'].includes(cue.fallback.field));
        const descriptor = checkpoint[cue.fallback.field];
        const path = relative(workspaceRoot, resolve(dirname(presenterFile(workspaceRoot, project.freeze)), descriptor.file)).split(sep).join('/');
        assert.equal(cue.fallback.path, path, 'Fallback belongs to a different checkpoint');
        assert.equal(presenterHash(await readFile(presenterFile(workspaceRoot, path))), descriptor.sha256, 'Fallback bytes changed');
    }
    const publicDocs = await Promise.all(['participant-resources.md', 'discussion-24-resources.md'].map(path => readFile(resolve(docs, path), 'utf8')));
    assert.equal(data.publicResources.length, 3);
    for (const resource of data.publicResources) {
        assert.ok(resource.path.startsWith('docs/declarative-amsterdam-2026/'));
        assert.equal(resource.url, `https://github.com/EPA-WG/cem/blob/${data.referenceRevision}/${resource.path}`, 'Use a pinned GitHub source link');
        const bytes = execFileSync('git', ['show', `${data.referenceRevision}:${resource.path}`], { cwd: workspaceRoot });
        assert.equal(presenterHash(bytes), resource.sha256, 'Pinned public source bytes changed');
        for (const doc of publicDocs) assert.ok(doc.includes(resource.url), 'Participant link missing from resource draft');
    }
    const allowedPublicLinks = new Set([data.resourceHub,
        data.resourceHub + '#discussioncomment-18867103',
        ...data.publicResources.map(item => item.url)]);
    for (const doc of [...publicDocs, notes]) {
        for (const [, target] of doc.matchAll(/\]\(([^)]+)\)/g)) {
            if (target.startsWith('https:') || target.startsWith('http:')) {
                assert.ok(allowedPublicLinks.has(target), 'Participant materials contain an unreviewed public link');
                continue;
            }
            const path = resolve(docs, target.split('#')[0]);
            presenterFile(workspaceRoot, relative(workspaceRoot, path));
            assert.equal(await access(path).then(() => true, () => false), true, `Local resource link missing: ${target}`);
        }
    }
    const deckNotes = JSON.parse(await readFile(resolve(docs, 'native-deck-notes-readback.json'), 'utf8'));
    assert.equal(deckNotes.cueTableSha256, presenterHash(JSON.stringify(cues)), 'Synced cue table changed; review native notes');
    assert.equal(deckNotes.resourceBindingsSha256, presenterHash(JSON.stringify({ privacy: data.privacy, publicResources: data.publicResources, cues: data.cues })), 'Synced resource bindings changed');
    assert.equal(deckNotes.slides, 30);
    for (const field of ['originalNotesPreserved', 'visibleContentUnchanged', 'exactResourceLinkReadback']) assert.equal(deckNotes[field], true);
    assert.deepEqual(deckNotes.blocks.map(block => block.slideId), cues.map(row => row[1].replaceAll('`', '')));
    const palette = JSON.parse(await readFile(resolve(docs, 'native-deck-projector-readback.json'), 'utf8'));
    assert.equal(palette.profile, 'projector');
    assert.equal(palette.media.exactImageReadback, true);
    assert.equal(palette.media.suppliedArtworkAndQRPreserved, true);
    assert.equal(palette.media.sourceTextPreserved, true);
    assert.equal(palette.media.replaced.length, 12);
    assert.equal(palette.palette.sourceSha256, presenterHash(await readFile(resolve(workspaceRoot, 'packages/cem-theme/src/lib/tokens/cem-colors.md'))), 'Reviewed palette source changed');
    assert.equal(palette.slides, 30); assert.equal(palette.visualReview.pages, 30);
    for (const key of ['titlePreserved', 'notesPreserved', 'textAndGeometryPreserved', 'exactColorReadback']) assert.equal(palette[key], true);
    const opening = JSON.parse(await readFile(resolve(docs, 'native-opening-readback.json'), 'utf8'));
    assert.equal(opening.native.presentationId, palette.presentationId);
    assert.equal(opening.native.completeMs, 25100);
    assert.equal(opening.native.slideCount, 30);
    assert.equal(opening.playback.clicks, 0);
    assert.equal(opening.playback.slideEntryStarts, true);
    assert.equal(opening.playback.earlierStagesRetainedAt30Seconds, true);
    assert.equal(opening.notesPaletteAndOrderUnchanged, true);
    const demoLinks = await verifyDiscussionDemoDraft(data, undefined, { freezes, native });
    return { schemaVersion: 1, demoLinks, referenceRevision: data.referenceRevision, publicSourceLinks: 3,
        recordedWebReadDate: data.webReadVerified, projects: data.projects.map(item => item.id), cues: data.cues.length,
        nativeSlideIds: 30, timedSlides: timed, plannedSeconds: elapsed, measuredRehearsal: false,
        nativeDeckSync: data.nativeDeckNotes, nativeDeckReadbackDate: deckNotes.checkedDate, recordedPaletteReviewDate: palette.checkedDate,
        recordedOpeningPlaybackDate: opening.checkedDate,
        discussionPosted: false, checks: 'local identities, pinned git blobs, fallback bytes, planned timing and recorded deck-note readback bindings' };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const result = await verifyPresenterResources();
    const path = resolve(workspaceRoot, 'dist/reports/cem-studio/presentation-resources.json');
    await mkdir(dirname(path), { recursive: true }); await writeFile(path, JSON.stringify(result, null, 2) + '\n');
    console.log(`Verified ${result.cues} presenter cues, 30 recorded slide IDs and three pinned participant source links; ${result.demoLinks.links.length} prepared demo routes (public browser acceptance recorded separately).`);
}
