import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';
await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const owner = JSON.parse(wasm.adoptDomStylesheets(JSON.stringify([{ css: '@import "child.css"; .card {color:green} @keyframes linear {from{opacity:0}to{opacity:1}}', scope: null }]))).artifactId;
const options = { consumer: 'host-one', index: 0, declarationIdentity: 'card', baseUrl: 'https://example.test/main.css',
    scope: { kind: 'private', tag: 'cem-card' }, context: { identity: 'page', resolverIdentity: 'resolver', resourcePolicyStamp: 'policy',
        frames: [{ frameId: 'page', baseUrl: 'https://example.test/page.html', scopes: [], specifiers: { imports: {}, resources: {} } }] } };
const begin = () => JSON.parse(wasm.beginTemplateStylesheet(owner, JSON.stringify(options)));
const deliver = (pending, type = 'text/css') => JSON.parse(wasm.deliverTemplateStylesheet(owner, pending.loadId, options.consumer,
    pending.request.id, new TextEncoder().encode('.card {background:url(icon.svg);animation:1s linear(0,.25 25% 75%,1) linear paused}'), 'https://example.test/cdn/child.css', type));
try {
    const stale = begin();
    assert.equal(stale.status, 'pending');
    const current = begin();
    assert.equal(JSON.parse(wasm.releaseTemplateStylesheets(owner, options.consumer, stale.loadId)).count, 0);
    assert.equal(deliver(stale).status, 'error');
    const ready = deliver(current);
    assert.equal(ready.status, 'ready');
    assert.ok(ready.css.includes('https://example.test/cdn/icon.svg'));
    assert.match(ready.css, /animation:1s linear\(0,\.25 25% 75%,1\) "linear-/);
    assert.ok(ready.css.includes(ready.identity.contextMarker));
    assert.deepEqual(ready.diagnostics, []);
    assert.equal(deliver(current).status, 'error');
    const bad = begin();
    assert.equal(deliver(bad, 'text/html').status, 'error');
    const oversized = begin();
    const limit = JSON.parse(wasm.deliverTemplateStylesheet(owner, oversized.loadId, options.consumer,
        oversized.request.id, new Uint8Array(16 * 1024 * 1024 + 1), 'https://example.test/child.css', 'text/css'));
    assert.equal(limit.status, 'error');
    assert.match(limit.message, /byte limits/);
    const released = begin();
    assert.equal(JSON.parse(wasm.releaseTemplateStylesheets(owner, options.consumer, 0)).status, 'released');
    assert.equal(deliver(released).status, 'error');
    const disposed = begin();
    assert.equal(wasm.disposeTemplate(owner), true);
    assert.equal(deliver(disposed).status, 'error');
    assert.equal(begin().status, 'error');
} finally { wasm.disposeTemplate(owner); }
console.log('Retained CSS WASM: loading, redirects, linear easing, stale delivery, MIME/byte limits, release and template disposal passed.');
