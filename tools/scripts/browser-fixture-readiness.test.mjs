import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'node:http';
import { chromium } from 'playwright';
import { browserFixtureReadiness } from './browser-fixture-readiness.mjs';

const markup = {
    controls: '<form><cem-action><button type="button">Apply</button></cem-action><cem-text-field name="item"><label>Item<input value="Sample"></label></cem-text-field></form>',
    data: '<cem-data-view format="json" value=""><div part="root" role="group" aria-label="Inventory"><p part="notice">No data</p></div></cem-data-view>',
    editor: '<cem-source-editor><span part="label">Source</span><pre style="min-height:1em" role="textbox" aria-label="Source" aria-multiline="true" aria-disabled="false" aria-readonly="false" contenteditable="plaintext-only"></pre></cem-source-editor>',
};
async function fixture(kind, run) {
    const server = createServer((_request, response) => response.writeHead(200, { 'content-type': 'text/html' }).end('<!doctype html><title>Readiness acceptance</title>'));
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    let browser;
    try {
        browser = await chromium.launch({ headless: true, executablePath: process.env.CEM_STUDIO_CHROME });
        const page = await browser.newPage(); await page.addInitScript(browserFixtureReadiness, { kind });
        await page.goto(`http://127.0.0.1:${server.address().port}/`); await run(page);
    } finally { await browser?.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
}
const frames = page => page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));

test('each complete fixture publishes a separately confirmed two-frame timestamp and stops sampling', async () => {
    for (const kind of ['controls', 'data', 'editor', 'selected']) await fixture(kind, async page => {
        await page.evaluate(html => { document.body.innerHTML = html; globalThis.auditSelectedReady = true; }, markup[kind] ?? '');
        await frames(page);
        const result = await page.evaluate(() => {
            const proof = __cemFixtureReadiness.finish(true);
            return { proof, stopped: !__cemFixtureReadiness.inspect().active };
        });
        assert(result.stopped); assert(result.proof.eligibleFrames >= 2);
        assert(result.proof.firstEligibleMs < result.proof.readyMs);
        assert(result.proof.readyMs <= result.proof.assertionConfirmedMs);
        const count = await page.evaluate(() => __cemFixtureReadiness.inspect().frameCount);
        await frames(page); assert.equal(await page.evaluate(() => __cemFixtureReadiness.inspect().frameCount), count);
    });
});

test('hidden, opaque, inert, disabled, wrong-name and partial controls cannot publish', async () => {
    const mutations = ['hidden', 'opacity', 'inert', 'disabled', 'label', 'partial'];
    for (const mutation of mutations) await fixture('controls', async page => {
        await page.evaluate(({ html, mutation }) => {
            document.body.innerHTML = html;
            const actions = {
                hidden: () => { document.querySelector('form').hidden = true; },
                opacity: () => { document.querySelector('form').style.opacity = '0'; },
                inert: () => { document.querySelector('form').inert = true; },
                disabled: () => { document.querySelector('input').disabled = true; },
                label: () => { document.querySelector('label').firstChild.textContent = 'Wrong'; },
                partial: () => { document.querySelector('button').remove(); },
            };
            actions[mutation]();
        }, { html: markup.controls, mutation });
        await frames(page);
        await assert.rejects(page.evaluate(() => __cemFixtureReadiness.finish(true)), /hidden, incomplete or unsettled/);
        assert.equal(await page.evaluate(() => __cemFixtureReadiness.inspect().active), false);
    });
});

test('readiness resets after hiding or node replacement and refuses a last-moment disconnection', async () => fixture('controls', async page => {
    await page.evaluate(html => { document.body.innerHTML = html; }, markup.controls); await frames(page);
    await page.evaluate(() => { document.querySelector('form').hidden = true; }); await frames(page);
    assert.equal(await page.evaluate(() => __cemFixtureReadiness.inspect().readyMs), null);
    await page.evaluate(() => { document.querySelector('form').hidden = false; }); await frames(page);
    const before = await page.evaluate(() => __cemFixtureReadiness.inspect().readyMs);
    await page.evaluate(() => { const input = document.querySelector('input'); input.replaceWith(input.cloneNode(true)); }); await frames(page);
    assert(await page.evaluate(() => __cemFixtureReadiness.inspect().readyMs) > before);
    await assert.rejects(page.evaluate(() => { document.querySelector('button').remove(); return __cemFixtureReadiness.finish(true); }), /incomplete/);
}));

test('failed original assertions and cancellation refuse publication even after eligibility', async () => {
    for (const cancel of [false, true]) await fixture('data', async page => {
        await page.evaluate(html => { document.body.innerHTML = html; }, markup.data); await frames(page);
        await assert.rejects(page.evaluate(cancel => {
            if (cancel) __cemFixtureReadiness.cancel();
            return __cemFixtureReadiness.finish(false);
        }, cancel), cancel ? /closed/ : /original browser assertions/);
        assert.equal(await page.evaluate(() => __cemFixtureReadiness.inspect().active), false);
    });
});
