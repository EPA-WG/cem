import { expect, it } from 'vitest';

// A new realm owns each heap, module registry and temporary IndexedDB database.
// Never enable recovery on, or clear storage from, an existing user session.
it.each(['worker', 'fallback'])('refuses incomplete Studio recovery and preserves a long editing/build session: %s', async mode => {
    const frame = document.createElement('iframe'); document.body.append(frame);
    const id = crypto.randomUUID(); let stop;
    try {
        const result = new Promise((resolve, reject) => {
            let stage = 'load';
            const timeout = setTimeout(() => reject(Error(`${mode}/${stage}: Studio recovery timed out`)), 110_000);
            const receive = event => {
                if (event.source !== frame.contentWindow || event.data?.id !== id) return;
                if (event.data.stage) { stage = event.data.stage; console.log(`[studio-recovery:${mode}] ${stage}`); return; }
                if (event.data.error) reject(Error(event.data.error)); else resolve(event.data.result);
            };
            window.addEventListener('message', receive);
            stop = () => { clearTimeout(timeout); window.removeEventListener('message', receive); };
        });
        const fixture = new URL('./recovery.fixture.js', import.meta.url).href;
        frame.srcdoc = `<script type="module">
            globalThis.__vitest_browser_runner__ = { wrapDynamicImport: f => f() };
            const id = ${JSON.stringify(id)};
            import(${JSON.stringify(fixture)}).then(({ studioRecovery }) => studioRecovery(${JSON.stringify(mode)}, stage => parent.postMessage({id, stage}, '*')))
                .then(result => parent.postMessage({id, result}, '*'), error => parent.postMessage({id, error: error.stack || String(error)}, '*'));
            </script>`;
        const observed = await result;
        expect(observed).toMatchObject({ recoveries: 0, refusedRecoveries: 3, edits: 12, unknownRefused: true, unsupported: expect.arrayContaining(['cem-media-preview']) });
        expect(observed.unsupported).not.toContain('cem-theme-switch');
    } finally { stop?.(); frame.remove(); }
}, 120_000);
