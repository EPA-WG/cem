// Real compiled WASM boundary: checked lookups, fresh handles and source integrity.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';
await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const stats = () => JSON.parse(wasm.templateCompilationCacheStats());
const hash = text => JSON.parse(wasm.templateArtifactPayloadKey(text, 'dev')).sourceHash;
const source = '{module | {import @as=base @src="./base.cemt"}{body | {call @from=base @template=label}}}';
const partial = '{module | {template @name=label @visibility=public | {body | {strong | Original}}}}';
const closure = { rootUri: 'https://example.test/root.cemt', rootContentHash: hash(source),
    resolverPolicyStamp: 'policy/1', entrypoint: 'body',
    modules: [{ alias: 'base', uri: 'https://example.test/base.cemt', contentHash: hash(partial), source: partial }] };
const ids = [];
const compile = c => { const r = JSON.parse(wasm.compileTemplateModuleClosure(source, JSON.stringify(c), '[]')); ids.push(r.artifactId); return r; };
try {
    wasm.resetTemplateCompilationCache();
    const before = stats();
    const first = compile(closure), hit = compile(closure);
    assert.equal(first.cacheHit, false); assert.equal(hit.cacheHit, true);
    assert.notEqual(first.artifactId, hit.artifactId);
    assert.equal(stats().compilations, before.compilations + 1);
    assert.deepEqual(JSON.parse(wasm.renderTemplate(first.artifactId, '{}')), JSON.parse(wasm.renderTemplate(hit.artifactId, '{}')));
    const tampered = structuredClone(closure); tampered.modules[0].source = partial.replace('Original', 'Edited');
    const rejected = compile(tampered);
    assert.equal(rejected.cacheHit, false); assert.ok(rejected.diagnostics.some(d => d.code === 'cem.ql.template.module_hash_mismatch'));
    tampered.modules[0].contentHash = hash(tampered.modules[0].source);
    const edited = compile(tampered); assert.equal(edited.cacheHit, false); assert.deepEqual(edited.diagnostics, []);
    assert.match(JSON.stringify(JSON.parse(wasm.renderTemplate(edited.artifactId, '{}'))), /Edited/);
    assert.equal(compile({ ...closure, resolverPolicyStamp: 'policy/2' }).cacheHit, false);
    const opaque = { ...closure, resolverPolicyStamp: '' };
    assert.equal(compile(opaque).cacheHit, false); assert.equal(compile(opaque).cacheHit, false);
    wasm.disposeTemplate(first.artifactId); ids.splice(ids.indexOf(first.artifactId), 1);
    assert.deepEqual(JSON.parse(wasm.renderTemplate(hit.artifactId, '{}')).diagnostics, []);
    wasm.resetTemplateCompilationCache();
    assert.equal(stats().residentEntries, 0);
    assert.deepEqual(JSON.parse(wasm.renderTemplate(hit.artifactId, '{}')).diagnostics, []);
    assert.equal(compile(closure).cacheHit, false);
    const simple = '{p | Simple}';
    for (const mode of ['dev', 'prod']) {
        const before = stats().compilations;
        const one = wasm.compileTemplateArtifact(simple, '[]', mode);
        const two = wasm.compileTemplateArtifact(simple, '[]', mode);
        assert.deepEqual(one, two); assert.equal(stats().compilations, before + 1);
    }
    const a = JSON.parse(wasm.compileTemplate(simple, '[]')), b = JSON.parse(wasm.compileTemplate(simple, '[]'));
    ids.push(a.artifactId, b.artifactId); assert.equal(a.cacheHit, true); assert.equal(b.cacheHit, true);
    assert.notEqual(a.artifactId, b.artifactId);
    console.log('Checked WASM compilation: hits, imports, freshness, corruption, binary modes, owner isolation and reset passed.');
} finally { for (const id of ids) wasm.disposeTemplate(id); wasm.resetTemplateCompilationCache(); }
