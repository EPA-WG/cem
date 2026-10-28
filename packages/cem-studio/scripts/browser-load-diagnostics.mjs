import { appendFileSync } from 'node:fs';

const observedPages = new WeakMap();
const location = value => {
    try {
        const url = new URL(value), scheme = url.protocol.slice(0, -1);
        return ['http', 'https'].includes(scheme)
            ? { scheme, origin: url.origin, path: url.pathname } : { scheme };
    } catch { return { scheme: 'invalid' }; }
};

/** Test-only metadata capture through public Playwright APIs. No bodies, headers,
 * query values, document contents, controls, retries or routing changes. */
export function observeBrowserLoads(page, write) {
    if (observedPages.has(page)) return observedPages.get(page);
    const pending = attach(page, write);
    observedPages.set(page, pending);
    return pending;
}

async function attach(page, write) {
    const record = (event, fields = {}) => write({ time: Date.now(), event, ...fields });
    const listeners = [];
    let cdp, disposed = false;
    const on = (target, event, listener) => {
        target.on(event, listener); listeners.push(() => target.off(event, listener));
    };
    const dispose = () => {
        if (disposed) return;
        disposed = true; observedPages.delete(page);
        for (const off of listeners) off();
        void cdp?.detach().catch(() => { /* Disposal can race an already closed page. */ });
    };
    on(page, 'close', dispose);
    on(page, 'requestfailed', request => {
        if (request.resourceType() === 'script') record('script-failed', {
            ...location(request.url()), error: request.failure()?.errorText,
        });
    });
    on(page, 'response', response => {
        const resource = location(response.url());
        if (response.request().resourceType() !== 'script') return;
        const entry = resource.path?.endsWith('/cem-elements/dist/index.js');
        if (entry || response.status() >= 400) record(entry ? 'entry-response' : 'script-response', {
            ...resource, status: response.status(),
        });
    });
    on(page, 'console', message => {
        if (/^\[studio-recovery:(worker|fallback)\]/.test(message.text())) record('stage', { stage: message.text() });
    });
    try {
        cdp = await page.context().newCDPSession(page);
        if (disposed) { await cdp.detach().catch(() => { /* Disposal can race an already closed page. */ }); return dispose; }
        const scripts = new Map();
        on(cdp, 'Network.requestWillBeSent', event => {
            if (event.type === 'Script') scripts.set(event.requestId, {
                ...location(event.request.url), frame: event.frameId,
            });
        });
        on(cdp, 'Network.loadingFinished', event => scripts.delete(event.requestId));
        on(cdp, 'Network.loadingFailed', event => {
            const script = scripts.get(event.requestId);
            if (script) record('script-network-failed', { ...script, error: event.errorText,
                canceled: event.canceled, blockedReason: event.blockedReason,
                corsError: event.corsErrorStatus?.corsError });
            scripts.delete(event.requestId);
        });
        on(cdp, 'Page.frameAttached', event => record('frame-attached', {
            frame: event.frameId, parent: event.parentFrameId,
        }));
        on(cdp, 'Page.frameDetached', event => record('frame-detached', {
            frame: event.frameId, reason: event.reason,
        }));
        await cdp.send('Network.enable');
        await cdp.send('Page.enable');
    } catch {
        record('diagnostic-unavailable', { channel: 'cdp' });
    }
    record('observer-ready');
    return dispose;
}

export function browserLoadDiagnosticCommand(destination) {
    return async ({ page }) => {
        await observeBrowserLoads(page, entry => appendFileSync(destination, `${JSON.stringify(entry)}\n`));
        return true;
    };
}
