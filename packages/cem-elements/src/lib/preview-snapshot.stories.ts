import { expect, spyOn, waitFor } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { registerCemArtifactTree, mountCemArtifactPreview } from './artifact-preview.js';
import { createCemPreviewSnapshotStore, exportCemPreviewSnapshot } from './preview-snapshot.js';
import { offerCemPreviewSnapshot, receiveCemPreviewSnapshot } from './preview-snapshot-handoff.js';

const meta = preview.meta({ title: 'Runtime/Preview snapshot ownership' });
const artifact = (path: string, contentType: string, text: string) => ({ path, contentType, bytes: new TextEncoder().encode(text) });
const identity = { label: 'Captured output', ownerId: 'project', configurationId: 'build', runId: 'run-1', projectRevision: 3, sourceRevision: 2, draft: true, stale: true, pinned: true };
const text = { identity, preview: { mode: 'text' as const, value: 'Original <b>text</b>', truncated: true } };
const pixel = { path: 'assets/pixel.png', contentType: 'image/png', bytes: Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jJXcAAAAASUVORK5CYII='), char => char.charCodeAt(0)) };
function required<T>(value: T | null | undefined): T { if (value == null) throw new Error('Missing fixture value'); return value; }

export const IndependentResourceLifetime = meta.story({
    render: () => '<iframe title="Snapshot output"></iframe>',
    play: async ({ canvasElement }) => {
        const source = artifact('index.html', 'text/html', '<link rel="stylesheet" href="css/site.css"><a href="other.html#end">Other page</a><img alt="Captured pixel" src="assets/pixel.png"><p>Original</p>');
        const tree = registerCemArtifactTree([source, pixel, artifact('css/site.css', 'text/css', '@import "nested/colors.css";'), artifact('css/nested/colors.css', 'text/css', 'p { color: rgb(1, 2, 3); }'), artifact('other.html', 'text/html', '<h1 id="end">Other output</h1>')]);
        const store = createCemPreviewSnapshotStore();
        const snapshot = store.capture({ identity, preview: { mode: 'artifact-tree', resourceSet: tree.resourceSet, artifactPath: 'index.html?view=one#start' } });
        const held = snapshot.retain(), frame = required(canvasElement.querySelector<HTMLIFrameElement>('iframe'));
        try {
            const exported = exportCemPreviewSnapshot(snapshot);
            if (exported.preview.mode !== 'artifact-tree') throw new Error('Wrong export mode');
            expect(exported.preview.artifacts.find(item => item.path === pixel.path)?.bytes).toEqual(pixel.bytes);
            for (const item of exported.preview.artifacts) item.bytes.fill(0);
            source.bytes.fill(0); tree.dispose(); snapshot.dispose();
            expect(held.identity).toEqual(identity); expect(Object.isFrozen(held.identity)).toBe(true);
            expect(held.preview.mode).toBe('artifact-tree');
            if (held.preview.mode !== 'artifact-tree') throw new Error('Wrong mode');
            expect(held.preview.resourceSet).not.toBe(tree.resourceSet);
            const paths: string[] = [];
            const mount = mountCemArtifactPreview(frame, { resourceSet: held.preview.resourceSet, path: held.preview.artifactPath, onNavigate: path => paths.push(path) });
            await mount.ready;
            const doc = required(frame.contentDocument);
            expect(doc.body.textContent).toContain('Original');
            await waitFor(() => expect(getComputedStyle(required(doc.querySelector('p'))).color).toBe('rgb(1, 2, 3)'));
            const image = required(doc.querySelector('img'));
            await waitFor(() => expect(image.naturalWidth).toBe(1));
            required(doc.querySelector('a')).click(); expect(paths).toEqual(['other.html#end']);
            expect(store.usage().snapshots).toBe(1); expect(store.usage().leases).toBe(1);
            const revoked = spyOn(URL, 'revokeObjectURL');
            try {
                held.dispose(); expect(store.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
                expect(() => held.preview).toThrow('released'); expect(frame.hasAttribute('srcdoc')).toBe(false);
                expect(revoked).toHaveBeenCalledWith(image.src);
            } finally { revoked.mockRestore(); }
        } finally { held.dispose(); snapshot.dispose(); store.dispose(); tree.dispose(); }
    },
});

export const AdmissionAndAccounting = meta.story({
    render: () => '<p>Snapshots retain passive bytes and explicit selection metadata.</p>',
    play: async () => {
        const store = createCemPreviewSnapshotStore({ maxSnapshots: 2, maxBytes: 64 });
        try {
            const input = { identity: { ...identity }, preview: { ...text.preview } };
            const first = store.capture(input), second = store.capture({ identity, preview: { mode: 'data', format: 'json', value: '{"ok":true}' } });
            input.identity.label = 'Changed'; input.preview.value = 'Changed';
            expect(first.identity).toEqual(identity); expect(first.preview).toEqual(text.preview); expect(second.preview.mode).toBe('data');
            expect(() => store.capture(text)).toThrow('capacity');
            first.dispose(); second.dispose();
            expect(() => store.capture({ identity, preview: { mode: 'text', value: 'a'.repeat(65) } })).toThrow('capacity');
            expect(() => store.capture({ identity, preview: { mode: 'live', value: 'x' } } as never)).toThrow('invalid');
            expect(() => store.capture({ identity: { ...identity, projectRevision: -1 }, preview: text.preview })).toThrow('invalid');
            expect(() => store.capture({ identity, preview: { mode: 'document', value: 'x'.repeat(262145) } })).toThrow('limit');
            expect(() => store.capture({ identity, preview: { mode: 'data', format: 'json', value: 'a'.repeat(65537) } })).toThrow('limit');
            expect(() => store.capture({ identity, preview: { mode: 'artifact-tree', resourceSet: 'not JSON', artifactPath: 'index.html' } })).toThrow('invalid');
            const released = registerCemArtifactTree([artifact('index.html', 'text/html', '<p>Released</p>')]); released.dispose();
            expect(() => store.capture({ identity, preview: { mode: 'artifact-tree', resourceSet: released.resourceSet, artifactPath: 'index.html' } })).toThrow('released');
            expect(store.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
            const document = store.capture({ identity, preview: { mode: 'document', value: '<script>inert source</script>' } });
            expect(document.preview.mode).toBe('document'); store.dispose();
            expect(() => document.retain()).toThrow('released'); document.dispose();
            expect(() => store.capture(text)).toThrow('released');
        } finally { store.dispose(); }
    },
});

export const OneReceiverAndReplayRefusal = meta.story({
    render: () => '<p>A one-use handoff creates one independent receiver owner.</p>',
    play: async () => {
        const source = createCemPreviewSnapshotStore(), one = createCemPreviewSnapshotStore(), two = createCemPreviewSnapshotStore();
        const snapshot = source.capture(text), offered = offerCemPreviewSnapshot(snapshot);
        const result = Promise.allSettled([receiveCemPreviewSnapshot(one, offered.token), receiveCemPreviewSnapshot(two, offered.token)]);
        snapshot.dispose();
        try {
            const outcomes = await result; await offered.completed;
            expect(outcomes.filter(outcome => outcome.status === 'fulfilled')).toHaveLength(1);
            expect(one.usage().snapshots + two.usage().snapshots).toBe(1);
            source.dispose();
            const accepted = outcomes.find(outcome => outcome.status === 'fulfilled');
            if (accepted?.status !== 'fulfilled') throw new Error('No receiver');
            expect(accepted.value.preview).toEqual(text.preview);
            await expect(receiveCemPreviewSnapshot(createCemPreviewSnapshotStore(), offered.token, { timeoutMs: 30 })).rejects.toThrow('expired');
            accepted.value.dispose();
        } finally { offered.cancel(); source.dispose(); one.dispose(); two.dispose(); }
    },
});

export const CancellationExpiryAndQuota = meta.story({
    render: () => '<p>Failed transfers release reservations and do not consume the source.</p>',
    play: async () => {
        const source = createCemPreviewSnapshotStore(), target = createCemPreviewSnapshotStore({ maxBytes: 1 });
        const snapshot = source.capture(text);
        try {
            const quota = offerCemPreviewSnapshot(snapshot);
            const quotaResult = Promise.allSettled([quota.completed, receiveCemPreviewSnapshot(target, quota.token)]);
            expect((await quotaResult).every(result => result.status === 'rejected')).toBe(true);
            expect(target.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
            const abort = new AbortController(), cancelled = offerCemPreviewSnapshot(snapshot, { signal: abort.signal });
            const cancelledResult = Promise.allSettled([cancelled.completed]); abort.abort();
            expect((await cancelledResult)[0].status).toBe('rejected');
            const expired = offerCemPreviewSnapshot(snapshot, { timeoutMs: 30 });
            await expect(expired.completed).rejects.toThrow('expired');
            expect(source.usage().leases).toBe(1); expect(snapshot.preview).toEqual(text.preview);
            const stopped = offerCemPreviewSnapshot(snapshot), stoppedResult = Promise.allSettled([stopped.completed]);
            source.dispose(); expect((await stoppedResult)[0].status).toBe('rejected');
        } finally { source.dispose(); target.dispose(); }
    },
});

export const MalformedEnvelopeAndAbortedReceiver = meta.story({
    render: () => '<p>Malformed transport data never publishes a snapshot.</p>',
    play: async () => {
        const store = createCemPreviewSnapshotStore(), token = crypto.randomUUID();
        const channel = new BroadcastChannel(`cem-preview-snapshot-v1:${token}`);
        channel.onmessage = event => { if (event.data?.type === 'hello') channel.postMessage({ protocol: 'cem-preview-snapshot/1', type: 'snapshot', client: event.data.client, snapshot: { schemaVersion: 99 } }); };
        try {
            await expect(receiveCemPreviewSnapshot(store, token)).rejects.toThrow('invalid');
            expect(store.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
            const controller = new AbortController(); controller.abort();
            await expect(receiveCemPreviewSnapshot(store, crypto.randomUUID(), { signal: controller.signal })).rejects.toThrow();
            await expect(receiveCemPreviewSnapshot(store, 'invalid')).rejects.toThrow('invalid');
        } finally { channel.close(); store.dispose(); }
    },
});

export const CancellationBeforeCommit = meta.story({
    render: () => '<p>Uncommitted receiver copies are released on cancellation or store closure.</p>',
    play: async () => {
        for (const cause of ['receiver-abort', 'receiver-close', 'sender-abort']) {
            const source = createCemPreviewSnapshotStore(), target = createCemPreviewSnapshotStore();
            const snapshot = source.capture(text), sender = new AbortController(), receiver = new AbortController();
            const original = BroadcastChannel.prototype.postMessage;
            const posting = spyOn(BroadcastChannel.prototype, 'postMessage').mockImplementation(function (this: BroadcastChannel, value) {
                if (value.type === 'accept') {
                    if (cause === 'receiver-abort') receiver.abort(new Error('cancel-before-commit'));
                    else if (cause === 'receiver-close') target.dispose();
                    else sender.abort(false);
                    return;
                }
                original.call(this, value);
            });
            try {
                const offered = offerCemPreviewSnapshot(snapshot, { signal: sender.signal });
                const result = await Promise.allSettled([offered.completed, receiveCemPreviewSnapshot(target, offered.token, { signal: receiver.signal })]);
                expect(result.every(outcome => outcome.status === 'rejected')).toBe(true);
                expect(target.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
                expect(source.usage().leases).toBe(1); expect(snapshot.preview).toEqual(text.preview);
            } finally { posting.mockRestore(); source.dispose(); target.dispose(); }
        }
    },
});

export const FailedDeliveryAndInvalidPayload = meta.story({
    render: () => '<p>Delivery failures and invalid control/byte envelopes publish no owner.</p>',
    play: async () => {
        const source = createCemPreviewSnapshotStore(), target = createCemPreviewSnapshotStore();
        const snapshot = source.capture(text), original = BroadcastChannel.prototype.postMessage;
        const random = spyOn(crypto, 'randomUUID').mockImplementationOnce(() => { throw new Error('Token creation failed'); });
        try {
            expect(() => offerCemPreviewSnapshot(snapshot)).toThrow('Token creation failed');
            expect(source.usage().leases).toBe(1);
        } finally { random.mockRestore(); }
        const posting = spyOn(BroadcastChannel.prototype, 'postMessage').mockImplementation(function (this: BroadcastChannel, value) {
            if (value.type === 'snapshot') throw new Error('Delivery failed');
            original.call(this, value);
        });
        try {
            const offered = offerCemPreviewSnapshot(snapshot);
            const result = await Promise.allSettled([offered.completed, receiveCemPreviewSnapshot(target, offered.token)]);
            expect(result.every(outcome => outcome.status === 'rejected')).toBe(true);
            expect(target.usage().snapshots).toBe(0); expect(source.usage().leases).toBe(1);
        } finally { posting.mockRestore(); }
        const local = artifact('index.html', 'text/html', '<p>Receiver local</p>');
        const localTree = registerCemArtifactTree([local]);
        try {
            const valid = exportCemPreviewSnapshot(snapshot);
            for (const payload of [
                { ...valid, byteLength: valid.byteLength + 1 },
                { ...valid, preview: { mode: 'live', value: 'restart' } },
                { ...valid, identity: { ...identity, credentials: 'forbidden' } },
                { ...valid, byteLength: local.bytes.byteLength, preview: { mode: 'artifact-tree', artifactPath: 'index.html', resourceSet: localTree.resourceSet } },
                { ...valid, byteLength: 1, preview: { mode: 'artifact-tree', artifactPath: 'index.html', artifacts: [{ ...artifact('index.html', 'text/html', ''), bytes: new Uint8Array(new ArrayBuffer(64), 16, 1) }] } },
                { ...valid, preview: { mode: 'artifact-tree', artifactPath: '../escape.html', artifacts: [artifact('../escape.html', 'text/html', '<p>Invalid</p>')] } },
            ]) {
                const token = crypto.randomUUID(), channel = new BroadcastChannel(`cem-preview-snapshot-v1:${token}`);
                channel.onmessage = event => { if (event.data?.type === 'hello') channel.postMessage({ protocol: 'cem-preview-snapshot/1', type: 'snapshot', client: event.data.client, snapshot: payload }); };
                try { await expect(receiveCemPreviewSnapshot(target, token)).rejects.toThrow(/invalid|artifact_path/); }
                finally { channel.close(); }
                expect(target.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
            }
        } finally { localTree.dispose(); source.dispose(); target.dispose(); }
    },
});

export const IndependentReceiverRealm = meta.story({
    render: () => '<iframe title="Independent viewer realm"></iframe>',
    play: async ({ canvasElement }) => {
        const frame = required(canvasElement.querySelector<HTMLIFrameElement>('iframe'));
        const tree = registerCemArtifactTree([artifact('index.html', 'text/html', '<link rel="stylesheet" href="site.css"><h1>Transferred bytes</h1>'), artifact('site.css', 'text/css', 'h1 { color: rgb(2, 3, 4); }')]);
        const source = createCemPreviewSnapshotStore(), captured = source.capture({ identity, preview: { mode: 'artifact-tree', resourceSet: tree.resourceSet, artifactPath: 'index.html' } });
        const offered = offerCemPreviewSnapshot(captured);
        const origin = location.origin;
        let listener: ((event: MessageEvent) => void) | undefined;
        const ready = new Promise<{ text: string; color: string; sandbox: string }>((resolve, reject) => {
            listener = event => { if (event.source !== frame.contentWindow || event.origin !== origin) return;
                if (event.data?.type === 'snapshot-fixture-ready') resolve(event.data);
                if (event.data?.type === 'snapshot-fixture-error') reject(new Error(event.data.error)); };
            window.addEventListener('message', listener);
        });
        const module = new URL('./preview-snapshot.ts', import.meta.url).href, handoff = new URL('./preview-snapshot-handoff.ts', import.meta.url).href, artifacts = new URL('./artifact-preview.ts', import.meta.url).href;
        frame.srcdoc = `<iframe title="Transferred output"></iframe><script type="module">
import {createCemPreviewSnapshotStore} from ${JSON.stringify(module)};
import {receiveCemPreviewSnapshot} from ${JSON.stringify(handoff)};
import {mountCemArtifactPreview} from ${JSON.stringify(artifacts)};
const store=createCemPreviewSnapshotStore();
try { const snapshot=await receiveCemPreviewSnapshot(store,${JSON.stringify(offered.token)}); const view=snapshot.preview;
const frame=document.querySelector('iframe'); const mount=mountCemArtifactPreview(frame,{resourceSet:view.resourceSet,path:view.artifactPath});
await mount.ready; await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));
parent.postMessage({type:'snapshot-fixture-ready',text:frame.contentDocument.body.textContent,color:getComputedStyle(frame.contentDocument.querySelector('h1')).color,sandbox:frame.sandbox.value},${JSON.stringify(origin)});
addEventListener('pagehide',()=>store.dispose(),{once:true});
} catch(error) { store.dispose(); parent.postMessage({type:'snapshot-fixture-error',error:String(error)},${JSON.stringify(origin)}); }
</script>`;
        try {
            await offered.completed; captured.dispose(); source.dispose(); tree.dispose();
            const result = await ready;
            expect(result.text).toContain('Transferred bytes'); expect(result.color).toBe('rgb(2, 3, 4)'); expect(result.sandbox).toBe('allow-same-origin');
            expect(source.usage()).toEqual({ snapshots: 0, leases: 0, byteLength: 0 });
        } finally { if (listener) window.removeEventListener('message', listener); offered.cancel(); source.dispose(); tree.dispose(); frame.srcdoc = ''; }
    },
});
