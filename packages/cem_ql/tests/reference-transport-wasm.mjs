// Low-level source handles stay local; workers transfer explicit bundle/CEMV bytes.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';

await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const encode = text => new TextEncoder().encode(text);
const fail = (fn, code) => assert.throws(fn, error => JSON.parse(String(error)).code === code);

function consume(bundle) {
    const source = wasm.importReferenceReloadBundle(new Uint8Array(bundle), 1, '');
    assert.equal(JSON.parse(wasm.inspectReferenceSource(source)).lexicalReady, true);
    let saved;
    for (const expression of ['#()', '#(input.children, input.children)', 'input']) {
        const result = wasm.queryReferenceSource(source, expression, 'memory:worker-query.cemql');
        if (expression === '#()') saved = wasm.exportNativeValueArtifact(result, '');
        else assert.throws(() => wasm.exportNativeValueArtifact(result, ''), error => {
            const failure = JSON.parse(String(error));
            return failure.code === 'cem.value.unsupported_source_reference'
                && failure.kind === 'UnsupportedSourceReference'
                && failure.sourceUri === 'memory:original.cem'
                && failure.sourceMap.frames.length > 0;
        });
        assert.equal(wasm.disposeNativeValueArtifact(result), true);
        fail(() => wasm.exportNativeValueArtifact(result, ''), 'cem.value.handle');
    }
    const result = wasm.queryReferenceSource(source, '#()', 'memory:retained.cemql');
    assert.equal(wasm.disposeReferenceSource(source), true);
    assert.ok(wasm.exportNativeValueArtifact(result, '').length);
    wasm.disposeNativeValueArtifact(result);
    fail(() => wasm.inspectReferenceSource(source), 'cem.reference.source_handle');
    fail(() => wasm.importReferenceReloadBundle(saved, 1, ''), 'cem.reference.reload');
    return saved;
}

function lifecycle() {
    const source = wasm.parseReferenceSource(encode('{#(#items)}'), 'text/cem-ml', 'memory:lifecycle.cem', '');
    const schema = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=item}}}'), 'text/cem-ml', 'memory:schema.cem', '');
    const session = wasm.beginReferenceValidationSession(source, schema);
    const other = wasm.beginReferenceValidationSession(source, schema);
    const run = () => JSON.parse(wasm.runReferenceValidationSession(session));
    assert.equal(run().complete, false);
    const empty = wasm.queryReferenceSource(source, '()', 'memory:context.cemql');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name: 'items', valueId: empty}]));
    wasm.disposeNativeValueArtifact(empty);
    assert.equal(run().complete, true);
    assert.equal(JSON.parse(wasm.runReferenceValidationSession(other)).complete, false);
    const library = wasm.parseReferenceSource(encode('{item}'), 'text/cem-ml', 'memory:library.cem', '');
    const destination = wasm.registerReferenceValidationSource(session, library);
    wasm.setReferenceValidationContext(session, destination, true, '[]');
    const targets = wasm.queryReferenceSource(library, 'input.children', 'memory:context.cemql');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name: 'items', valueId: targets}]));
    wasm.disposeNativeValueArtifact(targets);
    fail(() => wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name: 'items', valueId: targets, grants: true}])), 'cem.reference.validation');
    assert.equal(run().complete, false);
    assert.ok(run().dependencies.some(dependency => dependency.kind === 'ScopeDenied'));
    wasm.allowReferenceValidationCrossing(session, 0, destination);
    assert.equal(run().complete, true);
    wasm.setReferenceValidationPolicyBounds(session, 0, 128, 1);
    assert.equal(run().complete, false);
    assert.ok(run().dependencies.some(dependency => dependency.kind === 'WorkLimit'));
    wasm.setReferenceValidationPolicyBounds(session, 0, 128, 100000);
    for (const handle of [source, schema, library]) wasm.disposeReferenceSource(handle);
    assert.equal(run().complete, true);
    assert.equal(wasm.disposeReferenceValidationSession(other), true);
    assert.equal(run().complete, true);
    assert.equal(wasm.disposeReferenceValidationSession(session), true);
    fail(() => wasm.runReferenceValidationSession(session), 'cem.reference.validation');
}

lifecycle();

if (isMainThread) {
    const source = wasm.parseReferenceSource(encode('{div}{#input}'), 'text/cem-ml', 'memory:original.cem', '');
    const bytes = wasm.exportReferenceReloadBundle(source, '');
    const worker = new Worker(new URL(import.meta.url), { workerData: { bundle: bytes.slice().buffer } });
    const output = await new Promise((resolve, reject) => {
        worker.once('message', resolve);
        worker.once('error', reject);
        worker.once('exit', code => { if (code !== 0) reject(new Error(`worker exit ${code}`)); });
    });
    const imported = wasm.importNativeValueArtifact(new Uint8Array(output), JSON.stringify({maxBytes: 16777216, maxValues: 100000, maxDepth: 128}));
    wasm.disposeNativeValueArtifact(imported);
    consume(bytes);
    // Pending source capture metadata is never manufactured at import.
    fail(() => wasm.importReferenceReloadBundle(bytes, 77, ''), 'cem.reference.reload');
    fail(() => wasm.exportReferenceReloadBundle(source, '{"maxBytes":1,"maxNodes":1}'), 'cem.reference.reload_export');
    wasm.disposeReferenceSource(source);
    console.log('reference transport: native handles, attributed guards, worker reload and disposal passed');
} else {
    parentPort.postMessage(consume(workerData.bundle));
}
