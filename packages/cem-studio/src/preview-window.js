import { createCemPreviewSnapshotStore, offerCemPreviewSnapshot } from '@epa-wg/cem-elements';

const protocol = 'cem-studio-preview-window/1';
const channelName = id => `cem-studio-preview-window-v1:${id}`;
const validId = id => typeof id === 'string' && /^[0-9a-f]{8}-[0-9a-f-]{27}$/i.test(id);
const popupMessage = 'Preview window did not respond. Allow popups and try again.';

/** Browser API orchestration only. Call open on the trusted command event stack. */
export function createCemStudioPreviewWindow({ url, capture, onState = () => undefined,
    openWindow = (...args) => globalThis.open(...args), timeoutMs = 15000 }) {
    if (!Number.isFinite(timeoutMs) || timeoutMs < 1 || timeoutMs > 60000) throw new RangeError('Invalid preview window timeout');
    const store = createCemPreviewSnapshotStore();
    let session, pending, disposed = false;
    const publish = (message = '') => { if (!disposed) onState({ busy: Boolean(pending), open: Boolean(session?.ready), message }); };
    function closeSession() { session?.channel.close(); session = undefined; }
    function finish(success, message = '', keepSession = true) {
        const request = pending;
        if (!request) return;
        pending = undefined; clearTimeout(request.timer);
        if (!success) {
            session?.channel.postMessage({ protocol, type: 'cancel', requestId: request.id });
            request.offer?.cancel();
        }
        if (success && session) session.hasSnapshot = true;
        if (!keepSession) closeSession();
        publish(message); request.resolve(success);
    }
    function send() {
        if (pending?.offer && session?.ready) session.channel.postMessage({ protocol, type: 'load', requestId: pending.id, token: pending.offer.token });
    }
    function open() {
        if (disposed) return Promise.resolve(false);
        if (pending) return pending.completed;
        let resolve;
        const completed = new Promise(done => { resolve = done; });
        pending = { id: crypto.randomUUID(), resolve, completed };
        pending.timer = setTimeout(() => finish(false, popupMessage, false), timeoutMs);
        if (!session) {
            const id = crypto.randomUUID(), channel = new BroadcastChannel(channelName(id));
            session = { channel, ready: false };
            channel.onmessage = ({ data }) => {
                if (disposed || session?.channel !== channel || data?.protocol !== protocol) return;
                if (data.type === 'hello') { session.ready = true; send(); }
                else if (data.type === 'closed') {
                    if (pending) finish(false, 'Preview window closed. Open it again.', false);
                    else { closeSession(); publish(); }
                } else if (data.requestId === pending?.id) {
                    if (data.type === 'loaded') { pending.loaded = true; if (pending.received) finish(true); }
                    else if (data.type === 'failed') finish(false, session.hasSnapshot ? 'Preview replacement failed. The previous snapshot remains available. Try again.' : 'Preview transfer failed. Try opening the preview again.');
                }
            };
            const target = new URL(url); target.hash = new URLSearchParams({ viewer: id }).toString();
            // noopener deliberately returns null even when the popup succeeds.
            // Only the bounded same-origin handshake proves viewer availability.
            try { openWindow(target.href, '_blank', 'popup,noopener'); }
            catch { finish(false, popupMessage, false); return completed; }
        }
        try {
            const snapshot = capture(store);
            try { pending.offer = offerCemPreviewSnapshot(snapshot, { timeoutMs }); }
            finally { snapshot.dispose(); }
            const request = pending;
            request.offer.completed.then(() => {
                if (pending !== request) return;
                request.received = true; if (request.loaded) finish(true);
            }, () => { if (pending === request) finish(false, 'Preview transfer failed. Try opening the preview again.'); });
            send(); publish();
        } catch (error) {
            const message = error?.message?.startsWith('cem.')
                ? 'Preview capture failed. Try a smaller output or open the preview again.'
                : error?.message || 'The displayed preview is unavailable.';
            finish(false, message, Boolean(session?.ready));
        }
        return completed;
    }
    const pageExit = event => { if (!event.persisted) dispose(); };
    globalThis.addEventListener('pagehide', pageExit);
    function dispose() {
        if (disposed) return;
        if (pending) finish(false);
        disposed = true; globalThis.removeEventListener('pagehide', pageExit); closeSession(); store.dispose();
    }
    return Object.freeze({ open, usage: () => store.usage(), dispose });
}

/** Viewer-side control session. Snapshot content travels only through one-use handoffs. */
export function connectCemStudioPreviewWindow({ id, receive, focus = () => globalThis.focus() }) {
    if (!validId(id)) throw new TypeError('Invalid preview window session');
    const channel = new BroadcastChannel(channelName(id));
    let disposed = false, pending;
    const post = value => { if (!disposed) channel.postMessage({ protocol, ...value }); };
    channel.onmessage = ({ data }) => {
        if (disposed || data?.protocol !== protocol || !validId(data.requestId)) return;
        if (data.type === 'cancel' && data.requestId === pending?.id) { pending.controller.abort(); return; }
        if (data.type !== 'load' || pending || !validId(data.token)) return;
        const request = { id: data.requestId, controller: new AbortController() }; pending = request;
        try { focus(); } catch { /* Focus is browser policy, not a receipt requirement. */ }
        Promise.resolve().then(() => receive(data.token, { signal: request.controller.signal })).then(() => {
            if (!request.controller.signal.aborted) post({ type: 'loaded', requestId: request.id });
        }, () => { if (!request.controller.signal.aborted) post({ type: 'failed', requestId: request.id }); })
            .finally(() => { if (pending === request) pending = undefined; });
    };
    post({ type: 'hello' });
    return Object.freeze({ dispose() {
        if (disposed) return;
        post({ type: 'closed' }); disposed = true; pending?.controller.abort(); channel.close();
    } });
}
