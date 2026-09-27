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
    const staleError = deliver(stale);
    assert.equal(staleError.status, 'error');
    assert.deepEqual(staleError.diagnostics, []);
    const ready = deliver(current);
    assert.equal(ready.status, 'ready');
    assert.ok(ready.css.includes('https://example.test/cdn/icon.svg'));
    assert.match(ready.css, /animation:1s linear\(0,\.25 25% 75%,1\) "linear-/);
    assert.ok(ready.css.includes(ready.identity.contextMarker));
    assert.deepEqual(ready.diagnostics, []);
    assert.equal(deliver(current).status, 'error');
    const bad = begin();
    const mimeError = deliver(bad, 'text/html');
    assert.equal(mimeError.status, 'error');
    assert.equal(mimeError.diagnostics[0].code, 'cem.css.import_content_type');
    assert.match(mimeError.diagnostics[0].sourceUri, /^urn:cem:template-style:/);
    assert.equal(mimeError.diagnostics[0].stylesheetUrl, options.baseUrl);
    assert.equal(mimeError.diagnostics[0].offset, 0);
    assert.equal(mimeError.diagnostics[0].length, '@import "child.css";'.length);
    const integrityOptions = structuredClone(options);
    integrityOptions.context.frames[0].specifiers.resources['child.css'] = {
        target: './child.css', integrity: 'sha256-mkSHzL7faOU7/U/v8Umg+058R69+vN2A2xmE3Fz1q98=',
    };
    const integrityLoad = JSON.parse(wasm.beginTemplateStylesheet(owner, JSON.stringify(integrityOptions)));
    assert.equal(integrityLoad.status, 'pending');
    const integrityError = deliver(integrityLoad);
    assert.equal(integrityError.status, 'error');
    assert.equal(integrityError.diagnostics[0].code, 'cem.css.import_integrity');
    assert.equal(integrityError.diagnostics[0].stylesheetUrl, options.baseUrl);
    const oversized = begin();
    const limit = JSON.parse(wasm.deliverTemplateStylesheet(owner, oversized.loadId, options.consumer,
        oversized.request.id, new Uint8Array(16 * 1024 * 1024 + 1), 'https://example.test/child.css', 'text/css'));
    assert.equal(limit.status, 'error');
    assert.match(limit.message, /byte limits/);
    assert.equal(limit.diagnostics[0].code, 'cem.css.import_byte_limit');
    const released = begin();
    assert.equal(JSON.parse(wasm.releaseTemplateStylesheets(owner, options.consumer, 0)).status, 'released');
    assert.equal(deliver(released).status, 'error');
    const transport = begin();
    const wrongConsumer = JSON.parse(wasm.failTemplateStylesheet(owner, transport.loadId, 'other', transport.request.id, 'wrong'));
    assert.equal(wrongConsumer.status, 'error');
    assert.deepEqual(wrongConsumer.diagnostics, []);
    const transportError = JSON.parse(wasm.failTemplateStylesheet(owner, transport.loadId, options.consumer, transport.request.id, 'network offline'));
    assert.equal(transportError.status, 'error');
    assert.equal(transportError.diagnostics[0].code, 'cem.css.import_load_failed');
    assert.equal(transportError.diagnostics[0].stylesheetUrl, options.baseUrl);
    assert.equal(transportError.diagnostics[0].offset, 0);
    assert.match(transportError.diagnostics[0].sourceUri, /^urn:cem:template-style:/);
    assert.equal(deliver(transport).status, 'error');
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
console.log('Retained CSS WASM: loading, redirects, linear easing, source diagnostics, stale delivery, transport/MIME/byte limits, release and template disposal passed.');

const instanceSource = JSON.stringify([{ css: '@import "child.css"; @keyframes pulse {from{opacity:0}to{opacity:1}} .card {animation:pulse 1s}', scope: null }]);
const instanceOwner = JSON.parse(wasm.adoptInstanceStylesheets(instanceSource, 'persisted-instance')).artifactId;
assert.ok(instanceOwner > 0);
const instanceOptions = { ...options, declarationIdentity: 'persisted-instance', scope: { kind: 'instance' } };
const beginInstance = (overrides = {}) => JSON.parse(wasm.beginTemplateStylesheet(instanceOwner, JSON.stringify({ ...instanceOptions, ...overrides })));
try {
    assert.equal(beginInstance({ scope: options.scope }).status, 'error');
    assert.equal(beginInstance({ declarationIdentity: 'another-instance' }).status, 'error');
    const declarationOwner = JSON.parse(wasm.adoptDomStylesheets(instanceSource)).artifactId;
    try { assert.equal(JSON.parse(wasm.beginTemplateStylesheet(declarationOwner, JSON.stringify(instanceOptions))).status, 'error'); }
    finally { wasm.disposeTemplate(declarationOwner); }
    const load = () => {
        const pending = beginInstance();
        assert.equal(pending.status, 'pending');
        return JSON.parse(wasm.deliverTemplateStylesheet(instanceOwner, pending.loadId, options.consumer,
            pending.request.id, new TextEncoder().encode('.card {background:url(icon.svg)}'), 'https://example.test/cdn/child.css', 'text/css'));
    };
    const first = load();
    assert.equal(first.status, 'ready');
    assert.match(first.css, /^@scope to \(/);
    assert.ok(first.css.includes('https://example.test/cdn/icon.svg'));
    assert.match(first.css, /@keyframes pulse-/);
    assert.ok(!first.css.includes('data-cem-css-context'));
    assert.deepEqual(first.diagnostics, []);
    assert.equal(JSON.parse(wasm.releaseTemplateStylesheets(instanceOwner, options.consumer, first.loadId)).count, 1);
    const resumed = load();
    assert.deepEqual(resumed.identity, first.identity);
    assert.equal(resumed.css, first.css);
    const pending = beginInstance();
    assert.equal(JSON.parse(wasm.releaseTemplateStylesheets(instanceOwner, options.consumer, pending.loadId)).count, 1);
    assert.equal(JSON.parse(wasm.deliverTemplateStylesheet(instanceOwner, pending.loadId, options.consumer,
        pending.request.id, new TextEncoder().encode('.card{}'), pending.request.url, 'text/css')).status, 'error');
} finally { wasm.disposeTemplate(instanceOwner); }
assert.equal(beginInstance().status, 'error');
for (const [sources, identity] of [[instanceSource, ''], [JSON.stringify([{ css: '.card{}', scope: 'shared' }]), 'instance']]) {
    const rejected = JSON.parse(wasm.adoptInstanceStylesheets(sources, identity));
    assert.equal(rejected.artifactId, undefined);
    assert.equal(rejected.diagnostics[0].code, 'cem.ql.stylesheet_instance_invalid');
}
assert.equal(JSON.parse(wasm.adoptInstanceStylesheets('{', 'instance')).diagnostics[0].code, 'cem.ql.stylesheet_source_invalid');
console.log('Instance CSS WASM: fixed ownership, implicit scope, imports, reconnect identity, cancellation and disposal passed.');
