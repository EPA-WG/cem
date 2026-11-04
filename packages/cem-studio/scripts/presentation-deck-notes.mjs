import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { loadPresenterResources, verifyPresenterResources } from './presentation-resources.mjs';
import { workspaceRoot, presenterHash } from './presenter-project.mjs';

export const presentationId = '1v5JEclFycuXuYk5Mnpwb0alxdKIi30J6XgvLBmawxI4';
const marker = '[Current operational cues — 2026-11-01]';
const endMarker = '[End current operational cues]\n\n';
const text = element => (element.shape?.text?.textElements ?? []).map(run => run.textRun?.content ?? '').join('');
const unwrap = raw => raw.structuredContent ?? raw;
const table = notes => notes.split('\n').filter(line => /^\| \d{2} \|/.test(line))
    .map(line => line.split('|').slice(1, -1).map(cell => cell.trim()));

/** Notes-only tooling; never renders app UI or changes a slide's visible content. */
export function planDeckNotes(raw, data, notes, reviewedBlocks) {
    const deck = unwrap(raw), rows = table(notes);
    assert.equal(deck.presentationId, presentationId);
    assert.ok(deck.revisionId, 'Missing revision guard');
    assert.equal(deck.slides.length, rows.length);
    assert.deepEqual(deck.slides.map(slide => slide.objectId), rows.map(row => row[1].replaceAll('`', '')), 'Live slide order changed');
    const owners = new Set();
    const entries = deck.slides.map((slide, index) => {
        const page = slide.slideProperties?.notesPage;
        const owner = page?.notesProperties?.speakerNotesObjectId;
        assert.ok(owner && !owners.has(owner), 'Missing or duplicate speaker notes owner'); owners.add(owner);
        const elements = page.pageElements.filter(element => element.objectId === owner);
        assert.equal(elements.length, 1, 'Missing speaker notes element');
        const original = text(elements[0]);
        const cues = data.cues.filter(cue => cue.slideId === slide.objectId);
        const detail = cues.map(cue => `Cue ${cue.id}: project=${cue.project}; resource=${cue.resource}; configurations=${cue.configurations.join(', ')}. Saved fallback: ${cue.fallback.path}.${cue.fallbackMeaning ? ' ' + cue.fallbackMeaning : ''}`).join('\n');
        const block = `${marker}\nElapsed: ${rows[index][2]}. ${rows[index][3].replaceAll('`', '')}\n\nCurrent execution guidance supersedes older preparation/status claims below; original narrative notes are preserved. Use the verified local kit and independent saved copies. Save before executing; preserve unsaved drafts. Frozen output is a reference, never proof that a new execution succeeded.\n${detail ? '\n' + detail + '\n' : ''}\nUse disposable inputs only. Entered passwords/OTP block recovery until explicitly cleared. Never export credentials. Automatic whole-Studio recovery stays disabled; 4 MiB aggregate budget and five-second handoff deadline remain.\n\nResources and updates: ${data.resourceHub}\n${data.publicResources.map(resource => resource.label + ': ' + resource.url).join('\n')}\nPublic Studio: https://cem-amsterdam-2026.trans-acf.workers.dev/studio/ (accepted Site version 928c385e-79cb-4ef9-844a-ce64967a6243; HTTP/2 browser acceptance). Keep the verified local transfer kit as the offline fallback. No public kit download or untested CLI installation is promised. Times are planned, not a measured rehearsal.\n${endMarker}`;
        if (original.startsWith(marker)) {
            if (original.startsWith(block)) return { slideId: slide.objectId, owner, block, original, insert: false };
            const end = original.indexOf(endMarker);
            assert.ok(end >= 0, 'Missing operational block end marker');
            const previous = original.slice(0, end + endMarker.length);
            const reviewed = reviewedBlocks?.find(entry => entry.slideId === slide.objectId && entry.owner === owner);
            assert.equal(reviewed?.sha256, presenterHash(previous), 'Existing operational block differs; review before updating');
            return { slideId: slide.objectId, owner, block, original, insert: false, replaceLength: previous.length };
        }
        assert.ok(!original.includes(marker), 'Unexpected operational marker location');
        return { slideId: slide.objectId, owner, block, original, insert: true };
    });
    return { presentationId, revisionId: deck.revisionId,
        cueTableSha256: presenterHash(JSON.stringify(rows)),
        resourceBindingsSha256: presenterHash(JSON.stringify({ privacy: data.privacy, publicResources: data.publicResources, cues: data.cues })), entries,
        requests: entries.flatMap(entry => entry.replaceLength ? [
            { deleteText: { objectId: entry.owner, textRange: { type: 'FIXED_RANGE', startIndex: 0, endIndex: entry.replaceLength } } },
            { insertText: { objectId: entry.owner, insertionIndex: 0, text: entry.block } },
        ] : entry.insert ? [{ insertText: { objectId: entry.owner, insertionIndex: 0, text: entry.block } }] : []) };
}

export function verifyDeckNotes(before, after, plan) {
    before = unwrap(before); after = unwrap(after);
    assert.equal(after.presentationId, presentationId);
    assert.deepEqual(after.slides.map(slide => slide.objectId), before.slides.map(slide => slide.objectId));
    // Slides issues fresh short-lived image download URLs on each read. They
    // are transport metadata; retain sourceUrl, object IDs and every style.
    const stable = value => {
        if (Array.isArray(value)) return value.map(stable);
        if (!value || typeof value !== 'object') return value;
        return Object.fromEntries(Object.entries(value).map(([key, child]) => [key,
            key === 'image' ? stable({ ...child, contentUrl: undefined }) : stable(child)]));
    };
    const visible = deck => stable({ ...deck, revisionId: undefined, slides: deck.slides.map(slide => ({ ...slide,
        slideProperties: { ...slide.slideProperties, notesPage: undefined } })) });
    assert.deepEqual(visible(after), visible(before), 'Slide content, order or presentation metadata changed');
    for (const entry of plan.entries) {
        const slide = after.slides.find(item => item.objectId === entry.slideId);
        const page = slide.slideProperties.notesPage;
        assert.equal(page.notesProperties.speakerNotesObjectId, entry.owner);
        const element = page.pageElements.find(item => item.objectId === entry.owner);
        assert.equal(text(element), entry.replaceLength ? entry.block + entry.original.slice(entry.replaceLength) : (entry.insert ? entry.block : '') + entry.original, 'Notes or resource links failed exact readback');
        const oldPage = before.slides.find(item => item.objectId === entry.slideId).slideProperties.notesPage;
        assert.deepEqual(page.pageElements.filter(item => item.objectId !== entry.owner), oldPage.pageElements.filter(item => item.objectId !== entry.owner), 'Other notes elements changed');
    }
    return { schemaVersion: 1, presentationId, checkedDate: new Date().toISOString().slice(0, 10), slides: plan.entries.length,
        beforeRevision: before.revisionId, afterRevision: after.revisionId,
        cueTableSha256: plan.cueTableSha256, resourceBindingsSha256: plan.resourceBindingsSha256,
        insertedBlocks: plan.entries.filter(entry => entry.insert).length,
        refreshedBlocks: plan.entries.filter(entry => entry.replaceLength).length, originalNotesPreserved: true, visibleContentUnchanged: true,
        exactResourceLinkReadback: true, measuredRehearsal: false,
        blocks: plan.entries.map(entry => ({ slideId: entry.slideId, owner: entry.owner, sha256: presenterHash(entry.block) })) };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
    const [beforePath, afterPath] = process.argv.slice(2); assert.ok(beforePath, 'Supply a saved native deck response');
    const before = JSON.parse(await readFile(beforePath, 'utf8'));
    const data = await loadPresenterResources(); await verifyPresenterResources(data);
    const notes = await readFile(resolve(workspaceRoot, 'docs/declarative-amsterdam-2026/presenter-notes.md'), 'utf8');
    const plan = planDeckNotes(before, data, notes);
    const result = afterPath ? verifyDeckNotes(before, JSON.parse(await readFile(afterPath, 'utf8')), plan) : plan;
    const output = resolve(workspaceRoot, 'dist/reports/cem-studio', afterPath ? 'deck-notes-readback.json' : 'deck-notes-plan.json');
    await writeFile(output, JSON.stringify(result, null, 2) + '\n'); console.log(output);
}
