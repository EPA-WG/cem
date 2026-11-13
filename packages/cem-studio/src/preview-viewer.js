import { CemElementRuntime, mountCemView, createCemPreviewSnapshotStore, receiveCemPreviewSnapshot } from '@epa-wg/cem-elements';
import { CEM_STUDIO_THEME_MODES, createCemStudioThemeController } from './shell.js';

let installation;
async function installViewer() {
    const runtime = new CemElementRuntime({ declarationTag: 'cem-studio-preview-declaration' });
    const source = import.meta.resolve('@epa-wg/cem-components/components.xhtml');
    for (const [tag, url] of [
        ...['cem-stack', 'cem-alert', 'cem-media-preview', 'cem-data-view'].map(tag => [tag, `${source}#${tag}`]),
        ['cem-studio-preview-viewer', new URL('./preview-viewer.xhtml#cem-studio-preview-viewer', import.meta.url).href],
    ]) {
        if (customElements.get(tag)) continue;
        const declaration = document.createElement(runtime.declarationTag);
        declaration.setAttribute('tag', tag); declaration.setAttribute('src', url); document.head.append(declaration);
        runtime.registerDeclaration(declaration); await runtime.whenDeclarationSettled(declaration);
        const failed = runtime.diagnosticsFor(declaration).find(item => ['error', 'fatal'].includes(item.severity));
        if (failed) throw new Error(`Preview component unavailable: ${failed.message}`);
    }
    return runtime;
}

/** Browser transport and lifetime only; the XHTML view owns all visible UI. */
export async function mountCemStudioPreviewViewer({ root, token, signal, timeoutMs, store = createCemPreviewSnapshotStore(), themeMode }) {
    signal?.throwIfAborted();
    let disposed = false, view, retained;
    const lifetime = new AbortController();
    const initialTheme = CEM_STUDIO_THEME_MODES.includes(themeMode) ? themeMode : createCemStudioThemeController().mode;
    let state = { phase: 'waiting', theme: initialTheme };
    const publish = () => { if (!disposed) view?.update({ state }); };
    function dispose() {
        if (disposed) return;
        disposed = true; signal?.removeEventListener('abort', dispose); lifetime.abort(); view?.dispose(); view = undefined; store.dispose(); retained = undefined;
        state = { phase: 'closed', theme: state.theme };
    }
    signal?.addEventListener('abort', dispose, { once: true });
    // Receive before native component startup so cold rendering cannot consume
    // the offer's entire lifetime. Receipt is separate from view readiness.
    const received = receiveCemPreviewSnapshot(store, token, { signal: lifetime.signal, timeoutMs }).then(snapshot => {
        if (disposed) { snapshot.dispose(); return; }
        retained = snapshot;
        state = { ...state, phase: 'ready', identity: snapshot.identity, preview: snapshot.preview };
    }, error => {
        if (!disposed) state = { ...state, phase: 'error', error: `${error?.message === 'cem.preview.snapshot_handoff_expired' ? 'Preview transfer expired.' : 'Preview transfer unavailable.'} Open the preview from Studio again.` };
    });
    try {
        const runtime = await (installation ??= installViewer());
        if (disposed || !root.isConnected) { dispose(); throw new DOMException('Preview viewer closed', 'AbortError'); }
        view = mountCemView(runtime, { root, tag: 'cem-studio-preview-viewer', state: { state }, commands: {
            navigate({ value }) {
                if (disposed || !retained || state.preview?.mode !== 'artifact-tree' || typeof value !== 'string') return;
                state = { ...state, preview: Object.freeze({ ...state.preview, artifactPath: value }) }; publish();
            },
        } });
        const ready = received.then(async () => { if (!disposed) { publish(); await view?.whenSettled(); } });
        return Object.freeze({ ready, dispose, snapshot: () => Object.freeze({ ...state }),
            async whenSettled() { await ready; await view?.whenSettled(); },
            setTheme(theme) { if (!disposed && CEM_STUDIO_THEME_MODES.includes(theme)) { state = { ...state, theme }; publish(); } },
        });
    } catch (error) { dispose(); throw error; }
}
