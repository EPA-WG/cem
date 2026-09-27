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
// Sheet numbers alone cannot identify source files at the host boundary.
const diagnosticOwner = JSON.parse(wasm.adoptDomStylesheets(JSON.stringify([
    { css: '@import "child.css";\n#root {color:red}', scope: null },
]))).artifactId;
const childSource = '/* λ source */\n#blocked {color:red}\n.card {color:blue !important; background:green; animation:var(--motion)}\n@layer widgets {.card {color:red}}';
try {
    const pending = JSON.parse(wasm.beginTemplateStylesheet(diagnosticOwner, JSON.stringify(options)));
    assert.equal(pending.status, 'pending');
    const ready = JSON.parse(wasm.deliverTemplateStylesheet(diagnosticOwner, pending.loadId, options.consumer,
        pending.request.id, new TextEncoder().encode(childSource), 'https://example.test/redirected/child.css', 'text/css'));
    assert.equal(ready.status, 'ready');
    const rootDiagnostic = ready.diagnostics.find(d => d.sheet === 0);
    assert.match(rootDiagnostic.sourceUri, /^urn:cem:template-style:/);
    assert.equal(rootDiagnostic.stylesheetUrl, options.baseUrl);
    assert.equal(rootDiagnostic.code, 'cem.scoped_css.id_selector_unsupported');
    const imported = ready.diagnostics.filter(d => d.sheet === 1);
    assert.deepEqual(imported.map(d => d.code).sort(), [
        'cem.scoped_css.id_selector_unsupported', 'cem.scoped_css.important_unsupported',
        'cem.scoped_css.animation_name_dynamic_unsupported', 'cem.scoped_css.layer_unsupported',
    ].sort());
    for (const diagnostic of imported) {
        assert.equal(diagnostic.sourceUri, 'https://example.test/redirected/child.css');
        assert.equal(diagnostic.stylesheetUrl, 'https://example.test/redirected/child.css');
        assert.equal(diagnostic.severity, 'warning');
        assert.ok(diagnostic.line >= 2 && diagnostic.column >= 1 && diagnostic.length > 0);
        assert.ok(diagnostic.offset + diagnostic.length <= Buffer.byteLength(childSource));
    }
    const blocked = imported.find(d => d.code === 'cem.scoped_css.id_selector_unsupported');
    assert.equal(Buffer.from(childSource).subarray(blocked.offset, blocked.offset + blocked.length).toString(), '#blocked');
    assert.ok(ready.css.includes('background:green;'));
    for (const suppressed of ['#root', '#blocked', '!important', 'var(--motion)', '@layer']) {
        assert.ok(!ready.css.includes(suppressed), suppressed);
    }
} finally { wasm.disposeTemplate(diagnosticOwner); }
console.log('Retained CSS WASM: loading, redirects, linear easing, source diagnostics, stale delivery, MIME/byte limits, release and template disposal passed.');
