import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { createServer } from 'node:http';
import { test } from 'node:test';
import { captureStudioStartup, summarizeStartup } from './studio-startup-capture.mjs';
import { launchPublicationBrowser } from './publication-browser.mjs';

const origin = 'https://site.example';
const emitters = () => {
    const context = new EventEmitter(), page = new EventEmitter();
    context.serviceWorkers = () => []; page.workers = () => [];
    return { context, page };
};
const request = (url = origin + '/runtime.wasm?secret=omitted', serviceWorker = null) => ({
    url: () => url, resourceType: () => 'fetch', serviceWorker: () => serviceWorker,
    timing: () => ({ startTime: 999999999999, responseStart: 9, responseEnd: -1 }),
    failure: () => ({ errorText: 'net::ERR_QUIC_PROTOCOL_ERROR https://private.example/?secret=value' }),
});

test('captures repeated transfers, ownership, failures and pending work using relative time', () => {
    const { context, page } = emitters();
    let now = 500;
    const capture = captureStudioStartup(context, page, { origin, clock: () => now });
    const first = request(), second = request(undefined, {}), pending = request();
    context.emit('request', first); now += 10;
    context.emit('response', { request: () => first, status: () => 200, fromServiceWorker: () => false });
    context.emit('requestfinished', first); context.emit('request', second);
    now += 20; context.emit('requestfailed', second); context.emit('request', pending);
    const worker = new EventEmitter(); worker.url = () => origin + '/worker.js?secret=value';
    page.emit('worker', worker); capture.mark('readiness wait');
    const result = capture.finish('failed');
    assert.equal(result.elapsedMs, 30);
    assert.equal(result.requests.length, 3);
    assert.deepEqual(result.requests.map(item => item.owner), ['page-or-dedicated-worker', 'service-worker', 'page-or-dedicated-worker']);
    assert.deepEqual(result.requests.map(item => item.path), ['/runtime.wasm', '/runtime.wasm', '/runtime.wasm']);
    assert.deepEqual(result.requests[0].timing, { responseStart: 9, responseEnd: -1 });
    assert.equal(result.requests[0].endMs, 10);
    assert.equal(result.requests[1].failure, 'net::ERR_QUIC_PROTOCOL_ERROR');
    assert.equal(summarizeStartup(result).pending, 1);
    assert.equal(summarizeStartup(result).failed, 1);
    assert.deepEqual(result.workers, [{ kind: 'dedicated', path: '/worker.js', observedMs: 30, existing: false }]);
    assert.ok(!JSON.stringify(result).includes('secret'));
    now += 100; context.emit('requestfailed', pending); worker.emit('close');
    result.requests.length = 0;
    assert.equal(capture.finish('ready').requests.length, 3);
    assert.equal(capture.finish('ready').outcome, 'failed');
    assert.equal(context.listenerCount('request'), 0);
    assert.equal(worker.listenerCount('close'), 0);
});

test('bounds capture and ignores external URLs; tolerates unavailable diagnostics', () => {
    const { context, page } = emitters();
    const capture = captureStudioStartup(context, page, { origin, requestLimit: 1, workerLimit: 1, markLimit: 1 });
    context.emit('request', request('https://external.example/ignored'));
    context.emit('request', { url() { throw new Error('gone'); } });
    context.emit('request', request()); context.emit('request', request());
    for (let index = 0; index < 2; index++) {
        const worker = new EventEmitter(); worker.url = () => 'blob:https://site.example/private-id';
        page.emit('worker', worker); capture.mark('stage');
    }
    const result = capture.finish('ready');
    assert.deepEqual(result.dropped, { requests: 1, workers: 1, marks: 1 });
    assert.equal(result.requests.length, 1);
    assert.equal(result.workers[0].path, '[blob-worker]');
});

test('a held dedicated-worker response remains visible at the unchanged 30-second readiness deadline', async () => {
    let release;
    const held = new Promise(resolve => { release = resolve; });
    const server = createServer(async (req, res) => {
        if (req.url === '/worker.js') {
            res.setHeader('content-type', 'text/javascript');
            res.end("fetch('/runtime.wasm').then(r => r.text()).then(() => postMessage('ready'))");
        } else if (req.url === '/runtime.wasm') {
            await held; res.end('released');
        } else {
            res.setHeader('content-type', 'text/html');
            res.end('<script>new Worker("/worker.js").onmessage = () => { window.ready = true; };</script>');
        }
    });
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const localOrigin = `http://127.0.0.1:${server.address().port}`;
    let browser;
    try {
        browser = await launchPublicationBrowser();
        const context = await browser.newContext({ serviceWorkers: 'allow' });
        const page = await context.newPage(); page.setDefaultTimeout(30_000);
        const capture = captureStudioStartup(context, page, { origin: localOrigin });
        await page.goto(localOrigin); capture.mark('readiness wait');
        await assert.rejects(page.waitForFunction(() => window.ready), /Timeout 30000ms exceeded/);
        const result = capture.finish('failed');
        const runtime = result.requests.find(item => item.path === '/runtime.wasm');
        assert.ok(runtime, 'captures the worker-owned pending fetch');
        assert.equal(runtime.endMs, undefined); assert.equal(runtime.failedMs, undefined);
        assert.ok(result.elapsedMs >= 30_000);
        assert.ok(result.workers.some(item => item.kind === 'dedicated' && item.path === '/worker.js'));
        release(); await page.waitForFunction(() => window.ready);
        assert.deepEqual(capture.finish('ready'), result, 'late completion cannot rewrite the failed capture');
    } finally {
        release(); await browser?.close();
        server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
    }
});
