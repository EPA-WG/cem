import { performance } from 'node:perf_hooks';

// Passive diagnostics only: no interception, payloads, headers or URL queries.
// Durations use a monotonic clock; wall-clock corrections cannot stretch them.
export function captureStudioStartup(context, page, {
    origin, clock = () => performance.now(), requestLimit = 1024, workerLimit = 16, markLimit = 32,
}) {
    const start = clock();
    const elapsed = () => Math.round((clock() - start) * 100) / 100;
    const requests = [], workers = [], marks = [];
    const dropped = { requests: 0, workers: 0, marks: 0 };
    const entries = new WeakMap();
    const listeners = [];
    let result;
    const path = value => {
        const url = new URL(value);
        if (url.protocol === 'blob:') return '[blob-worker]';
        return url.origin === origin && ['http:', 'https:'].includes(url.protocol) ? url.pathname : undefined;
    };
    const listen = (emitter, event, callback) => {
        // Observability must never throw into the browser workflow.
        const safe = (...args) => { try { callback(...args); } catch { /* unavailable diagnostic */ } };
        emitter.on(event, safe);
        listeners.push(() => emitter.off(event, safe));
    };
    const timing = (request, entry) => {
        const values = request.timing();
        entry.timing = Object.fromEntries(Object.entries(values).filter(([name, value]) =>
            name !== 'startTime' && Number.isFinite(value)));
    };
    listen(context, 'request', request => {
        const requestPath = path(request.url());
        if (!requestPath) return;
        if (requests.length >= requestLimit) { dropped.requests++; return; }
        const entry = {
            path: requestPath, resourceType: request.resourceType(), startMs: elapsed(),
            // Playwright does not distinguish page and dedicated-worker owners here.
            owner: request.serviceWorker?.() ? 'service-worker' : 'page-or-dedicated-worker',
        };
        entries.set(request, entry);
        requests.push(entry);
    });
    listen(context, 'response', response => {
        const entry = entries.get(response.request());
        if (!entry) return;
        Object.assign(entry, { responseMs: elapsed(), status: response.status(), fromServiceWorker: response.fromServiceWorker() });
        timing(response.request(), entry);
    });
    listen(context, 'requestfinished', request => {
        const entry = entries.get(request);
        if (entry) { entry.endMs = elapsed(); timing(request, entry); }
    });
    listen(context, 'requestfailed', request => {
        const entry = entries.get(request);
        if (!entry) return;
        entry.failedMs = elapsed();
        entry.failure = request.failure()?.errorText?.match(/net::ERR_[A-Z0-9_]+/)?.[0] ?? 'request-failed';
        timing(request, entry);
    });
    const worker = (value, kind, existing = false) => {
        const workerPath = path(value.url());
        if (!workerPath) return;
        if (workers.length >= workerLimit) { dropped.workers++; return; }
        const entry = { kind, path: workerPath, observedMs: elapsed(), existing };
        workers.push(entry);
        listen(value, 'close', () => { entry.closedMs = elapsed(); });
    };
    listen(page, 'worker', value => worker(value, 'dedicated'));
    listen(context, 'serviceworker', value => worker(value, 'service-worker'));
    for (const value of page.workers()) worker(value, 'dedicated', true);
    for (const value of context.serviceWorkers()) worker(value, 'service-worker', true);
    return {
        mark(name) {
            if (result) return;
            if (marks.length >= markLimit) { dropped.marks++; return; }
            marks.push({ name, elapsedMs: elapsed() });
        },
        finish(outcome) {
            if (!result) {
                // Detach and clone before teardown so cleanup aborts cannot become startup failures.
                for (const detach of listeners) detach();
                result = structuredClone({ outcome, elapsedMs: elapsed(), requests, workers, marks, dropped });
            }
            return structuredClone(result);
        },
    };
}

export function summarizeStartup(capture) {
    return {
        outcome: capture.outcome, elapsedMs: capture.elapsedMs, requests: capture.requests.length,
        pending: capture.requests.filter(request => request.endMs === undefined && request.failedMs === undefined).length,
        failed: capture.requests.filter(request => request.failedMs !== undefined).length,
        workers: capture.workers, marks: capture.marks, dropped: capture.dropped,
    };
}
