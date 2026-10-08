// Real WASM sources, executable descendants and presentation-only CEMV.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';
await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const source = text => wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', 'memory:options.cem', '');
const relation = source('{#datadom.slices.options}');
const options = source('{option @value=same | First {#datadom.slices.other}}{option @value=same | Second}');
const metadata = { requesting: 0, sources: [{ sourceId: relation, context: true }, { sourceId: options, context: true }], bindings: [
    { source: 0, name: 'relation', select: 'input.children' }, { source: 1, name: 'options', select: 'input.children' }], grants: [[0, 1]] };
const prepare = value => wasm.prepareNativeCapabilitySession('{}', JSON.stringify(value), 'relation', true, '[]', limits);
for (const bad of [{ ...metadata, grants: [] }, { ...metadata, sources: [{ sourceId: relation, context: false }, metadata.sources[1]] },
    { ...metadata, sources: [metadata.sources[0], { sourceId: options, context: true, maxWork: 1 }] }]) assert.throws(() => prepare(bad));
const session = prepare(metadata);
assert.equal(wasm.nativeCapabilitySessionLength(session), 2);
// The session keeps its owners after disposable ingress handles are released.
wasm.disposeReferenceSource(relation); wasm.disposeReferenceSource(options);
const label = JSON.parse(wasm.compileTemplate('{span | {$input.name}|{$input.children.kind}|{$input.children.expression}}', '["input"]'));
assert.deepEqual(label.diagnostics, []);
try {
    const plan = JSON.parse(wasm.renderNativeCapabilityTemplate(session, label.artifactId, 0));
    assert.deepEqual(plan.diagnostics, []);
    assert.ok(plan.nodes[0].children.some(n => n.text === '#datadom.slices.other'));
    assert.throws(() => wasm.exportNativeCapabilityView(session, 'input', 0), /executable source reference/);
    const presentation = JSON.parse(wasm.exportNativeCapabilityView(session, 'input.name', undefined));
    assert.equal(presentation.length, 2);
    const bytes = wasm.takeRenderValueArtifact(presentation.artifactId);
    assert.equal(wasm.exportCemJsonValue(bytes, 0, '', limits), '"option"');
    assert.equal(wasm.exportCemJsonValue(bytes, 1, '', limits), '"option"');
    const empty = JSON.parse(wasm.exportNativeCapabilityView(session, '()', undefined));
    assert.equal(empty.length, 0); assert.equal(empty.artifactId, null);
    assert.throws(() => wasm.renderNativeCapabilityTemplate(session, label.artifactId, 2));
    assert.equal(wasm.disposeNativeCapabilitySession(session), true);
    assert.equal(wasm.disposeNativeCapabilitySession(session), false);
    assert.throws(() => wasm.nativeCapabilitySessionLength(session), /Unknown native/);
} finally { wasm.disposeTemplate(label.artifactId); wasm.disposeNativeCapabilitySession(session); }
console.log('native capability session WASM checks passed');
