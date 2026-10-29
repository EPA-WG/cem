// Strict deployment admission and portable pins across physically separate heaps.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createCemQlWasmGeneration } from '../dist/wasm/cem_ql_factory.js';

const wasmBytes = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const first = createCemQlWasmGeneration(); await first.initialize(wasmBytes);
const call = (runtime, name, ...args) => runtime.call(name, args);
const parse = (runtime, name, ...args) => JSON.parse(call(runtime, name, ...args));
const source = '{style | ``` :scope { display: block; } ```}{input @value=Original}{p | {$label}}';
const bindings = '["label"]';
function build(text) {
    const key = parse(first, 'templateArtifactPayloadKey', text, 'prod');
    const bytes = call(first, 'compileTemplateArtifact', text, bindings, 'prod');
    const retained = parse(first, 'importTemplateArtifact', bytes, '', text, bindings, 'prod');
    assert.ok(retained.artifactId);
    call(first, 'disposeTemplate', retained.artifactId);
    return { bytes, pin: { contentHash: retained.contentHash, sourceHash: key.sourceHash,
        cemMlVersion: key.cemMlVersion, cemQlVersion: key.cemQlVersion,
        hostBindings: ['label'], sourceMapMode: 'prod' } };
}
const built = build(source), invalid = build('{p | {$missing}}');
const next = createCemQlWasmGeneration(); await next.initialize(wasmBytes);
assert.notEqual(first.memory(), next.memory());
assert.equal(parse(next, 'renderTemplate', 1, '{}').diagnostics[0].code, 'cem.ql.wasm.unknown_artifact');
for (const runtime of [first, next]) {
    for (const key of ['contentHash', 'sourceHash', 'cemMlVersion', 'cemQlVersion']) {
        for (const wrong of ['', 'unknown', `cem-bin/1+blake3:${'0'.repeat(64)}`]) {
            const result = parse(runtime, 'importPinnedTemplateArtifact', built.bytes, JSON.stringify({ ...built.pin, [key]: wrong }));
            assert.equal(result.artifactId, null);
            assert.equal(result.diagnostics[0].code, 'cem.ql.template_artifact_hash_mismatch');
        }
    }
    for (const changed of [{ hostBindings: [] }, { sourceMapMode: 'dev' }]) {
        const result = parse(runtime, 'importPinnedTemplateArtifact', built.bytes, JSON.stringify({ ...built.pin, ...changed }));
        assert.equal(result.artifactId, null); assert.equal(result.diagnostics[0].code, 'cem.cc.policy_mismatch');
    }
    const extra = parse(runtime, 'importPinnedTemplateArtifact', built.bytes, JSON.stringify({ ...built.pin, source }));
    assert.equal(extra.diagnostics[0].code, 'cem.ql.template_artifact_invalid_pin');
    const badBuild = parse(runtime, 'importPinnedTemplateArtifact', invalid.bytes, JSON.stringify(invalid.pin));
    assert.equal(badBuild.diagnostics[0].code, 'cem.ql.template_artifact_invalid_build');
    const corrupt = built.bytes.slice(); corrupt[corrupt.length - 1] ^= 1;
    const rejected = parse(runtime, 'importPinnedTemplateArtifact', corrupt, JSON.stringify(built.pin));
    assert.equal(rejected.artifactId, null);
    const retained = parse(runtime, 'importPinnedTemplateArtifact', built.bytes, JSON.stringify(built.pin));
    assert.ok(retained.artifactId); assert.equal(retained.stylesheets.length, 1);
    try {
        for (const label of ['First', 'Changed <img>']) {
            const plan = parse(runtime, 'renderTemplate', retained.artifactId, JSON.stringify({ label }));
            assert.deepEqual(plan.diagnostics, []);
            assert.equal(plan.nodes.find(node => node.tag === 'p').children[0].text, label);
        }
    } finally { call(runtime, 'disposeTemplate', retained.artifactId); }
    runtime.dispose();
    assert.throws(() => call(runtime, 'importPinnedTemplateArtifact', built.bytes, JSON.stringify(built.pin)), /retired/);
}
console.log('Pinned template admission: strict pins, invalid-build refusal, native CSS, changing data and separate-heap reimport passed.');
