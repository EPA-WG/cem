import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'node:http';
import { chromium } from 'playwright';
import { browserStartupPhases } from './browser-startup-phases.mjs';

async function fixture(options, run) {
    const server = createServer((_request, response) => response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><title>Startup diagnostic acceptance</title>'));
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    let browser;
    try {
        browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
        const page = await browser.newPage();
        await page.addInitScript(browserStartupPhases, options);
        await page.goto(`http://127.0.0.1:${server.address().port}/`);
        await run(page);
    } finally { await browser?.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}

test('startup capture preserves native promises/results/errors and restores every wrapper', async () => fixture({ kind: 'controls' }, async page => {
    const result = await page.evaluate(async () => {
        const probe = globalThis.__cemStartupPhases;
        const expected = Promise.resolve('original owner');
        const prototype = Object.fromEntries(['registerDeclaration', 'registerResolvedDeclaration', 'surfaceDeclarationDiagnostics',
            'prepareRetainedStylesheets', 'renderInstance'].map(name => [name, function () { return expected; }]));
        const original = prototype.renderInstance;
        probe.instrumentRuntime(prototype);
        const samePromise = prototype.renderInstance() === expected;
        const bytes = Uint8Array.from([0,97,115,109,1,0,0,0]);
        const module = await WebAssembly.compile(bytes);
        const instance = await WebAssembly.instantiate(module);
        await crypto.subtle.digest('SHA-256', bytes);
        let rejected = false;
        try { await WebAssembly.compile(new Uint8Array(0)); } catch (error) { rejected = error instanceof WebAssembly.CompileError; }
        document.body.innerHTML = '<cem-action><button>Apply</button></cem-action><cem-text-field name="item"><input></cem-text-field>';
        await new Promise(resolve => requestAnimationFrame(resolve));
        const beforeStop = performance.now();
        const evidence = probe.stop(), count = evidence.events.length;
        await WebAssembly.compile(bytes);
        prototype.renderInstance();
        document.querySelector('button').remove();
        await new Promise(resolve => requestAnimationFrame(resolve));
        return { evidence, samePromise, module: module instanceof WebAssembly.Module, instance: instance instanceof WebAssembly.Instance,
            rejected, restored: prototype.renderInstance === original, countAfterStop: evidence.events.length, count, beforeStop };
    });
    assert(result.samePromise && result.module && result.instance && result.rejected && result.restored);
    assert.equal(result.countAfterStop, result.count);
    assert(result.evidence.visibleAt > 0 && result.evidence.visibleAt <= result.beforeStop);
    assert.equal(result.evidence.overflow, false);
    assert.equal(result.evidence.events.filter(event => event.label === 'wasm.compile').length, 2);
    assert.equal(result.evidence.events.find(event => event.label === 'wasm.compile' && event.inputBytes === 0).status, 'rejected');
    assert(result.evidence.events.every(event => event.end >= event.start));
}));

test('hidden controls do not publish a visibility witness; calls remain unwrapped in observe-only mode', async () => fixture({ kind: 'controls', instrumentCalls: false }, async page => {
    const result = await page.evaluate(async () => {
        document.body.innerHTML = '<cem-action><button hidden>Apply</button></cem-action><cem-text-field name="item"><input></cem-text-field>';
        await WebAssembly.compile(Uint8Array.from([0,97,115,109,1,0,0,0]));
        return globalThis.__cemStartupPhases.stop();
    });
    assert.equal(result.visibleAt, null);
    assert.deepEqual(result.events, []);
}));

test('phase capture is bounded without dropping original calls', async () => fixture({ kind: 'editor' }, async page => {
    const result = await page.evaluate(() => {
        let calls = 0;
        const prototype = Object.fromEntries(['registerDeclaration', 'registerResolvedDeclaration', 'surfaceDeclarationDiagnostics',
            'prepareRetainedStylesheets', 'renderInstance'].map(name => [name, () => ++calls]));
        globalThis.__cemStartupPhases.instrumentRuntime(prototype);
        for (let n = 0; n < 5001; n++) prototype.renderInstance();
        return { calls, evidence: globalThis.__cemStartupPhases.stop() };
    });
    assert.equal(result.calls, 5001);
    assert.equal(result.evidence.events.length, 5000);
    assert.equal(result.evidence.overflow, true);
}));
