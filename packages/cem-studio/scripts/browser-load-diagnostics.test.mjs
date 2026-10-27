import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { once } from 'node:events';
import test from 'node:test';
import { chromium } from 'playwright';
import { observeBrowserLoads } from './browser-load-diagnostics.mjs';

test('records failed module HTTP/network and frame lifetime without bodies, query values or retries', { timeout: 15_000 }, async t => {
    const query = 'fixture-query-not-recorded', body = 'fixture-body-not-recorded';
    let requests = 0;
    const server = createServer((request, response) => {
        response.setHeader('Cache-Control', 'no-store');
        if (request.url.startsWith('/packages/cem-elements/dist/index.js')) {
            requests++;
            response.writeHead(requests === 1 ? 503 : 200, { 'Content-Type': 'text/javascript' });
            response.end(requests === 1 ? body : 'export const ready = 42;');
        } else { response.writeHead(200, { 'Content-Type': 'text/html' }); response.end('<!doctype html><title>Module probe</title>'); }
    });
    server.listen(0, '127.0.0.1'); await once(server, 'listening');
    let browser, stop, cleanup;
    const close = () => cleanup ??= (async () => {
        stop?.(); await browser?.close();
        server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
    })();
    t.after(close);
    const events = [], pending = new Map();
    const next = event => new Promise(resolve => pending.set(event, resolve));
    try {
        browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
        const context = await browser.newContext(), page = await context.newPage();
        const initialResponseListeners = page.listenerCount('response');
        stop = await observeBrowserLoads(page, entry => {
            events.push(entry); pending.get(entry.event)?.(entry);
        });
        assert.equal(await observeBrowserLoads(page, () => assert.fail('duplicate recorder')), stop);
        const origin = `http://127.0.0.1:${server.address().port}`;
        await page.goto(origin);
        const importInFreshFrame = () => page.evaluate(async moduleUrl => {
            const frame = document.createElement('iframe'), id = crypto.randomUUID();
            document.body.append(frame);
            let receive;
            try {
                const result = new Promise(resolve => {
                    receive = event => {
                        if (event.source === frame.contentWindow && event.data?.id === id) resolve(event.data.result);
                    };
                    window.addEventListener('message', receive);
                });
                frame.srcdoc = `<script type="module">import(${JSON.stringify(moduleUrl)})
                    .then(module => parent.postMessage({id:${JSON.stringify(id)},result:{ready:module.ready}}, '*'),
                    error => parent.postMessage({id:${JSON.stringify(id)},result:{error:error.name}}, '*'));</script>`;
                return await result;
            } finally { window.removeEventListener('message', receive); frame.remove(); }
        }, `${origin}/packages/cem-elements/dist/index.js?probe=${query}`);
        const failed = next('script-network-failed'), detached = next('frame-detached');
        assert.deepEqual(await importInFreshFrame(), { error: 'TypeError' }, 'the original import error must propagate');
        assert.equal(requests, 1, 'observation must not retry imports');
        const networkFailure = await failed;
        const removal = await detached;
        assert.equal(networkFailure.path, '/packages/cem-elements/dist/index.js');
        assert.equal(typeof networkFailure.frame, 'string');
        assert.equal(removal.frame, networkFailure.frame, 'the failed script and removed realm share an owner');
        assert.match(networkFailure.error, /net::ERR_/);
        assert.ok(events.some(entry => entry.event === 'entry-response' && entry.status === 503));
        assert.deepEqual(await importInFreshFrame(), { ready: 42 });
        assert.equal(requests, 2, 'a separate realm owns its explicit successful attempt');
        assert.ok(events.some(entry => entry.event === 'entry-response' && entry.status === 200));
        assert.equal(events.some(entry => entry.event === 'diagnostic-unavailable'), false);
        const encoded = JSON.stringify(events);
        assert.equal(encoded.includes(query), false);
        assert.equal(encoded.includes(body), false);
        stop(); stop();
        assert.equal(page.listenerCount('response'), initialResponseListeners, 'recorder listeners are released');
        await context.close();
    } finally {
        await close();
    }
});
