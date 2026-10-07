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
