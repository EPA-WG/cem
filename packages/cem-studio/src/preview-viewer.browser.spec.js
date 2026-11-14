import { beforeAll, afterEach, expect, it } from 'vitest';
import { createCemPreviewSnapshotStore, offerCemPreviewSnapshot, registerCemArtifactTree } from '@epa-wg/cem-elements';
import { captureCemStudioDisplayedPreview } from './workbench.js';
import { mountCemStudioPreviewViewer } from './preview-viewer.js';
import themeCss from '@epa-wg/cem-theme/styles.css?raw';

beforeAll(() => {
    const map = document.createElement('script'); map.type = 'importmap';
    map.textContent = JSON.stringify({ imports: { '@epa-wg/cem-components/components.xhtml': new URL('../../cem-components/dist/components.xhtml', import.meta.url).href } }); document.head.append(map);
});
const cleanup = [];
afterEach(() => { for (const stop of cleanup.splice(0).reverse()) stop(); });
function store() { const value = createCemPreviewSnapshotStore(); cleanup.push(() => value.dispose()); return value; }
const encode = text => new TextEncoder().encode(text);
const literal = (text, kind = 'text') => kind === 'text' ? { kind, text, label: text } : { kind, source: text, label: text };
function built(preview, { pinned = false, stale = false } = {}) {
    const output = { uri: 'studio://p/output.txt', bytes: encode('Built text'), contentType: 'text/plain' };
    const artifact = { path: 'output.txt', uri: output.uri, contentType: output.contentType };
    const run = { id: 'run-1', projectId: 'p', projectRevision: 3, configuration: { id: 'build' }, inputs: [{ resourceId: 'source', revision: 2 }], artifacts: [artifact] };
    return { projectId: 'p', resourceId: 'source', projectRevision: 4, resourceRevision: 3,
        navigation: { selection: { project: 'p', file: 'source' }, sourcePreview: { preview: literal('Draft source'), draft: true, revision: 3 } },
        projectPreview: { projectId: 'p', previewContext: 'source', configurationId: 'build', selectedArtifact: artifact, selectedOutput: output, relatedArtifacts: [artifact],
            preview, projection: { buildRun: run, preview, output }, stale, pinId: pinned ? 'pin-1' : undefined } };
}
it('captures the displayed build and pin instead of the current source draft, preserving original identities', () => {
    const state = built(literal('Built text'), { pinned: true, stale: true });
    const snapshot = captureCemStudioDisplayedPreview(state, store());
    expect(snapshot.preview).toEqual({ mode: 'text', value: 'Built text', truncated: false });
    expect(snapshot.identity).toMatchObject({ ownerId: 'p', configurationId: 'build', runId: 'run-1', projectRevision: 3, sourceRevision: 2, pinned: true, stale: true, draft: false });
    state.projectPreview.preview.text = 'New build'; expect(snapshot.preview.value).toBe('Built text');
    state.projectPreview.previewContext = 'output'; state.resourceId = 'unrelated';
    state.projectPreview.projection.buildRun.inputs.push({ resourceId: 'unrelated', revision: 99 });
    expect(captureCemStudioDisplayedPreview(state, store()).identity.sourceRevision).toBeUndefined();
});
it('captures source overrides and owned invocation output using the same visible precedence', () => {
    const source = { navigation: { selection: { project: 'p', file: 'source' }, path: 'source.md', sourcePreview: { preview: literal('<h1>Draft HTML</h1>', 'sandboxed-html'), draft: true, stale: true, revision: 7 } }, projectPreview: { projectId: 'p', previewContext: 'source', relatedArtifacts: [] } };
    const draft = captureCemStudioDisplayedPreview(source, store());
    expect(draft.preview).toEqual({ mode: 'document', value: '<h1>Draft HTML</h1>' });
    expect(draft.identity).toMatchObject({ draft: true, stale: true, sourceRevision: 7 });
    const csv = { ...source, navigation: { ...source.navigation, sourcePreview: { text: 'name\nAda\n', contentType: 'text/csv', revision: 8, draft: true } } };
    expect(captureCemStudioDisplayedPreview(csv, store()).preview).toEqual({ mode: 'data', format: 'csv', value: 'name\nAda\n' });
    const state = built(literal('Built text'));
    state.entryId = 'build'; state.navigation.selection.configuration = 'build';
    state.projection = { preview: literal('Invocation'), stale: true, output: { available: true, uri: 'cem-stdio://stdout', contentType: 'text/plain', bytes: encode('Invocation') },
        buildRun: { id: 'invocation', projectId: 'p', configuration: { id: 'build' }, inputs: [{ resourceId: 'source', revision: 5 }], artifacts: [{ uri: 'cem-stdio://stdout', path: 'invocation.txt' }] } };
    const invocation = captureCemStudioDisplayedPreview(state, store());
    expect(invocation.preview.value).toBe('Invocation'); expect(invocation.identity.runId).toBe('invocation'); expect(invocation.identity.pinned).toBe(false);
});
it('captures data as the displayed bounded table and refuses unavailable, live and oversized previews', () => {
    const state = built(literal('Fallback'));
    state.projectPreview.selectedOutput = { contentType: 'application/json', bytes: encode('[{"name":"Ada"}]') };
    const table = captureCemStudioDisplayedPreview(state, store()); expect(table.preview).toEqual({ mode: 'data', format: 'json', value: '[{"name":"Ada"}]' });
    state.projectPreview.selectedOutput.bytes = encode('a'.repeat(262145)); expect(() => captureCemStudioDisplayedPreview(state, store())).toThrow('too large');
    for (const preview of [undefined, { kind: 'live-cem', source: 'interactive' }, { kind: 'download', reason: 'Exact download only' }]) {
        const value = built(preview); expect(() => captureCemStudioDisplayedPreview(value, store())).toThrow();
    }
    const missing = built(literal('Do not substitute')); missing.projectPreview.missingArtifact = true;
    expect(() => captureCemStudioDisplayedPreview(missing, store())).toThrow();
});

async function viewer(input, options = {}) {
    const sender = store(), held = sender.capture(input), offered = offerCemPreviewSnapshot(held); held.dispose(); cleanup.push(offered.cancel);
    const root = document.createElement('main'); document.body.append(root); cleanup.push(() => root.remove());
    const target = store(), view = await mountCemStudioPreviewViewer({ root, token: offered.token, store: target, ...options }); cleanup.push(view.dispose);
    await view.ready; await offered.completed; sender.dispose();
    return { root, view, target };
}
it('renders literal text, isolated document and bounded native tables through declarative shared controls', async () => {
    const colors = document.createElement('style'); colors.textContent = themeCss; document.head.append(colors); cleanup.push(() => colors.remove());
    const identity = { label: 'Captured result', runId: 'old-run', stale: true, pinned: true };
    const plain = await viewer({ identity, preview: { mode: 'text', value: '<b>Literal</b>', truncated: true } });
    expect(plain.root.querySelector('pre').textContent).toBe('<b>Literal</b>'); expect(plain.root.querySelector('pre b')).toBeNull();
    expect(plain.root.textContent).toContain('old-run'); expect(plain.root.textContent).toContain('Stale'); expect(plain.root.textContent).toContain('Pinned');
    for (const mode of ['cem-theme-light', 'cem-theme-dark', 'cem-theme-projector', 'cem-theme-contrast-light', 'cem-theme-contrast-dark', 'cem-theme-native']) {
        plain.view.setTheme(mode); await plain.view.whenSettled(); expect(plain.root.querySelector('[data-theme]').getAttribute('data-theme')).toBe(mode);
        expect(getComputedStyle(plain.root.querySelector('pre')).visibility).toBe('visible');
    }
    plain.root.querySelector('[data-theme]').style.setProperty('--cem-typography-script-font-size', '23px');
    expect(getComputedStyle(plain.root.querySelector('pre')).fontSize).toBe('23px');
    const docView = await viewer({ identity, preview: { mode: 'document', value: '<p>Isolated</p><script>parent.__viewerAttack=true</script>' } });
    expect(docView.root.querySelector('iframe').getAttribute('sandbox')).toBe(''); expect(globalThis.__viewerAttack).toBeUndefined();
    const data = await viewer({ identity, preview: { mode: 'data', format: 'json', value: JSON.stringify(Array.from({ length: 101 }, (_, id) => ({ id, name: 'Ada' }))) } });
    expect(data.root.querySelector('cem-data-view table')).not.toBeNull(); expect(data.root.textContent).toContain('Row limit: 100');
    plain.view.dispose(); expect(plain.target.usage().snapshots).toBe(0);
    expect(plain.view.snapshot().phase).toBe('closed'); expect(plain.view.snapshot().preview).toBeUndefined(); await plain.view.whenSettled();
}, 60_000);
it('keeps artifact navigation within the captured tree after source release', async () => {
    const original = registerCemArtifactTree([{ path: 'index.html', contentType: 'text/html', bytes: encode('<a href="other.html#next">Next</a>') }, { path: 'other.html', contentType: 'text/html', bytes: encode('<h1 id="next">Captured page</h1>') }]); cleanup.push(original.dispose);
    const result = await viewer({ identity: { label: 'Site' }, preview: { mode: 'artifact-tree', resourceSet: original.resourceSet, artifactPath: 'index.html?q=old' } }); original.dispose();
    const frame = result.root.querySelector('iframe'); frame.contentDocument.querySelector('a').click(); await result.view.whenSettled();
    expect(result.root.querySelector('iframe').contentDocument.body.textContent).toContain('Captured page');
    expect(result.view.snapshot().preview.artifactPath).toBe('other.html#next');
}, 60_000);
it('shows expired/invalid transfer errors and releases a cancelled receiver without publishing late content', async () => {
    const root = document.createElement('main'); document.body.append(root); cleanup.push(() => root.remove());
    const invalid = await mountCemStudioPreviewViewer({ root, token: '' }); cleanup.push(invalid.dispose); await invalid.ready;
    expect(root.textContent).toContain('unavailable'); expect(root.querySelector('[role=alert]')).not.toBeNull();
    invalid.dispose();
    const expired = await mountCemStudioPreviewViewer({ root, token: crypto.randomUUID(), timeoutMs: 30 }); cleanup.push(expired.dispose); await expired.ready;
    expect(root.textContent).toContain('expired'); expired.dispose();
    const target = store(), abort = new AbortController();
    const pending = await mountCemStudioPreviewViewer({ root, store: target, token: crypto.randomUUID(), signal: abort.signal }); cleanup.push(pending.dispose);
    abort.abort(); await pending.ready;
    expect(target.usage().snapshots).toBe(0); expect(root.querySelector('iframe, pre, table')).toBeNull();
}, 60_000);

it('replaces a detached snapshot only after successful receipt and keeps it after a failed replacement', async () => {
    const { view } = await viewer({ identity: { label: 'First snapshot' }, preview: { mode: 'text', value: 'First' } });
    const sender = store();
    const candidate = sender.capture({ identity: { label: 'Second snapshot' }, preview: { mode: 'text', value: 'Second' } });
    const offer = offerCemPreviewSnapshot(candidate); candidate.dispose(); cleanup.push(offer.cancel);
    await view.replace(offer.token); await offer.completed;
    expect(view.snapshot().preview.value).toBe('Second');
    const abort = new AbortController(); abort.abort();
    await expect(view.replace(crypto.randomUUID(), { signal: abort.signal })).rejects.toThrow();
    expect(view.snapshot().preview.value).toBe('Second');
    expect(view.snapshot().identity.label).toBe('Second snapshot');
});

it('preserves artifact navigation performed while a failed replacement is waiting', async () => {
    const { view, root } = await viewer({ identity: { label: 'Site' }, preview: { mode: 'artifact-tree', artifactPath: 'index.html', artifacts: [
        { path: 'index.html', contentType: 'text/html', bytes: encode('<a href="next.html">Next</a>') },
        { path: 'next.html', contentType: 'text/html', bytes: encode('<h1>Still available</h1>') },
    ] } });
    const abort = new AbortController();
    const replacing = view.replace(crypto.randomUUID(), { signal: abort.signal });
    // Attach the failure observer before cancellation can reject the handoff.
    const failed = expect(replacing).rejects.toThrow();
    await Promise.resolve();
    root.querySelector('iframe').contentDocument.querySelector('a').click(); await view.whenSettled();
    expect(view.snapshot().preview.artifactPath).toBe('next.html');
    abort.abort(); await failed;
    expect(view.snapshot().preview.artifactPath).toBe('next.html');
    expect(root.querySelector('iframe').contentDocument.body.textContent).toContain('Still available');
});
