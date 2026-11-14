import { afterEach, expect, it } from 'vitest';
import { createCemPreviewSnapshotStore, receiveCemPreviewSnapshot } from '@epa-wg/cem-elements';
import { createCemStudioPreviewWindow, connectCemStudioPreviewWindow } from './preview-window.js';

const cleanup = [];
afterEach(() => { for (const stop of cleanup.splice(0).reverse()) stop(); });
function fixture(openWindow, timeoutMs = 300) {
    const states = [], store = createCemPreviewSnapshotStore(); cleanup.push(() => store.dispose());
    let text = 'First';
    const source = createCemStudioPreviewWindow({ url: new URL('/preview.html', location.href), openWindow, timeoutMs,
        capture: target => target.capture({ identity: { label: text }, preview: { mode: 'text', value: text } }),
        onState: state => states.push(state) });
    cleanup.push(source.dispose);
    return { source, states, setText: value => { text = value; } };
}
function receiver(url, receive) {
    const id = new URLSearchParams(new URL(url).hash.slice(1)).get('viewer');
    const connection = connectCemStudioPreviewWindow({ id, receive }); cleanup.push(connection.dispose); return connection;
}
it('accepts noopener null handles, reuses the viewer and releases sender snapshots after receipt', async () => {
    let opens = 0, held;
    const target = createCemPreviewSnapshotStore();
    cleanup.push(() => target.dispose());
    const f = fixture(url => { opens++; receiver(url, async token => {
        held?.dispose(); held = await receiveCemPreviewSnapshot(target, token);
    }); return null; });
    const first = f.source.open(); expect(f.source.open()).toBe(first);
    expect(await first).toBe(true); expect(held.preview.value).toBe('First');
    expect(f.source.usage().snapshots).toBe(0);
    f.setText('Second'); expect(await f.source.open()).toBe(true);
    expect(opens).toBe(1); expect(held.preview.value).toBe('Second');
    f.source.dispose(); expect(held.preview.value).toBe('Second');
});
it('reports blocked and nonresponsive popups and permits a fresh synchronous retry', async () => {
    let opens = 0;
    const f = fixture(() => { opens++; return null; }, 30);
    expect(await f.source.open()).toBe(false);
    expect(f.states.at(-1)).toMatchObject({ busy: false, open: false });
    expect(f.states.at(-1).message).toContain('Allow popups');
    expect(f.source.usage().snapshots).toBe(0);
    expect(await f.source.open()).toBe(false); expect(opens).toBe(2);
    const thrown = fixture(() => { throw new Error('blocked'); });
    expect(await thrown.source.open()).toBe(false); expect(thrown.states.at(-1).message).toContain('Allow popups');
});
it('keeps a responding viewer after a failed replacement and permits retry', async () => {
    let fail = false, opens = 0, held;
    const target = createCemPreviewSnapshotStore(); cleanup.push(() => target.dispose());
    const f = fixture(url => { opens++; receiver(url, async token => {
        if (fail) throw new Error('Unavailable');
        held?.dispose(); held = await receiveCemPreviewSnapshot(target, token);
    }); });
    expect(await f.source.open()).toBe(true); fail = true; f.setText('Second');
    expect(await f.source.open()).toBe(false); expect(held.preview.value).toBe('First');
    expect(f.source.usage().snapshots).toBe(0);
    fail = false; expect(await f.source.open()).toBe(true); expect(opens).toBe(1);
});
it('cancels a pending transfer when either window closes and reopens after viewer closure', async () => {
    let connection, opens = 0;
    const f = fixture(url => { opens++; connection = receiver(url, () => new Promise(() => undefined)); });
    const pending = f.source.open(); await new Promise(resolve => setTimeout(resolve, 20)); connection.dispose();
    expect(await pending).toBe(false); expect(f.source.usage().snapshots).toBe(0);
    const retry = f.source.open(); f.source.dispose(); expect(await retry).toBe(false);
    expect(opens).toBe(2); expect(f.source.usage().snapshots).toBe(0);
});

it('releases rejected captures and opens a fresh session for a smaller-output retry', async () => {
    let opens = 0;
    const target = createCemPreviewSnapshotStore(); cleanup.push(() => target.dispose());
    const f = fixture(url => {
        if (++opens === 2) receiver(url, token => receiveCemPreviewSnapshot(target, token));
        return null;
    });
    f.setText('x'.repeat(262145));
    expect(await f.source.open()).toBe(false);
    expect(f.states.at(-1).message).toContain('smaller output');
    expect(f.states.at(-1).message).not.toContain('cem.');
    expect(f.source.usage().snapshots).toBe(0);
    f.setText('Fits'); expect(await f.source.open()).toBe(true); expect(opens).toBe(2);
});
