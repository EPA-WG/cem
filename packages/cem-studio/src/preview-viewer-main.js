import { mountCemStudioPreviewViewer } from './preview-viewer.js';
import { connectCemStudioPreviewWindow } from './preview-window.js';

const parameters = new URLSearchParams(location.hash.slice(1));
// Temporary authority is never a durable/shareable route.
history.replaceState(history.state, '', location.pathname + location.search);
const controller = new AbortController();
let viewer, connection, startupTimer;
addEventListener('pagehide', () => { clearTimeout(startupTimer); connection?.dispose(); controller.abort(); viewer?.dispose(); }, { once: true });
async function receive(token, options = {}) {
    clearTimeout(startupTimer);
    if (viewer) return viewer.replace(token, options);
    viewer = await mountCemStudioPreviewViewer({ root: document.querySelector('[data-cem-studio-root]'), token,
        signal: AbortSignal.any([controller.signal, ...(options.signal ? [options.signal] : [])]) });
    await viewer.ready;
    if (viewer.snapshot().phase !== 'ready') throw new Error('Preview unavailable');
}
try {
    const id = parameters.get('viewer');
    if (id) {
        connection = connectCemStudioPreviewWindow({ id, receive });
        startupTimer = setTimeout(() => {
            connection.dispose();
            void receive('').catch(error => { if (error.name !== 'AbortError') console.error('CEM Studio preview startup failed', error); });
        }, 15000);
    }
    else await receive(parameters.get('snapshot') ?? '');
} catch (error) { if (error.name !== 'AbortError') console.error('CEM Studio preview startup failed', error); }
