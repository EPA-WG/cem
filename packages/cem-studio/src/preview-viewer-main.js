import { mountCemStudioPreviewViewer } from './preview-viewer.js';

const token = new URLSearchParams(location.hash.slice(1)).get('snapshot') ?? '';
// A handoff token is temporary authority, not a durable/shareable route.
history.replaceState(history.state, '', location.pathname + location.search);
const controller = new AbortController();
let viewer;
addEventListener('pagehide', () => { controller.abort(); viewer?.dispose(); }, { once: true });
try {
    viewer = await mountCemStudioPreviewViewer({ root: document.querySelector('[data-cem-studio-root]'), token, signal: controller.signal });
    await viewer.ready;
} catch (error) { if (error.name !== 'AbortError') console.error('CEM Studio preview startup failed', error); }
