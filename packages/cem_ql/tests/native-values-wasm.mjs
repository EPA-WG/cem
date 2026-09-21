// CEMT-VALUE-TRANSPORT: real separate WASM workers, saved bytes and main-thread fallback.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';
const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const ready = () => readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)).then(bytes => wasm.default({ module_or_path: bytes }));
const source = '{child | {attribute @name=count @type=integer @value=002}{attribute @name=day @type=date @value=2024-02-29}{attribute @name=label @type=node @content-type=text/html @value=\'{data:read("<name>ivy<em>saur</em></name>", "xml").root.children}\'}}';
function produce() {
    const plan = JSON.parse(wasm.renderTemplateSource(source, '{}'));
    assert.deepEqual(plan.diagnostics, []);
    const bytes = wasm.takeRenderValueArtifact(plan.nativeValueArtifactId).slice().buffer;
    assert.throws(() => wasm.takeRenderValueArtifact(plan.nativeValueArtifactId));
    return { bytes, bindings: plan.nodes[0].attributes.map(attribute => ({ name: attribute.name, index: attribute.nativeValueIndex })) };
}
function consume({ bytes, bindings }) {
    const id = wasm.importNativeValueArtifact(new Uint8Array(bytes), limits);
    const template = JSON.parse(wasm.compileTemplate('{output | {$count + 1}{span | {$day}}{$label}}', JSON.stringify(bindings.map(b => b.name))));
    try {
        const native = JSON.stringify(bindings.map(b => ({ ...b, artifactId: id })));
        const plan = JSON.parse(wasm.renderTemplateWithNativeValues(template.artifactId, 0, '{}', '[]', native, limits));
        assert.deepEqual(plan.diagnostics, []);
        const children = plan.nodes[0].children;
        assert.equal(children[0].text, '3');
        assert.equal(children[1].children[0].text, '2024-02-29');
        assert.equal(children[2].tag, 'name');
        assert.equal(children[2].children[1].tag, 'em');
        assert.equal(children[2].children[1].children[0].text, 'saur');
        assert.equal(wasm.disposeNativeValueArtifact(id), true);
        const stale = JSON.parse(wasm.renderTemplateWithNativeValues(template.artifactId, 0, '{}', '[]', native, limits));
        assert.ok(stale.diagnostics.some(d => d.code === 'cem.value.binding'));
        assert.throws(() => wasm.importNativeValueArtifact(new Uint8Array(bytes), JSON.stringify({ maxBytes: 8, maxValues: 20, maxDepth: 10 })));
        return true;
    } finally { wasm.disposeNativeValueArtifact(id); wasm.disposeTemplate(template.artifactId); }
}
async function runWorker(task) {
    const worker = new Worker(new URL(import.meta.url), { workerData: task });
    try { return await new Promise((resolve, reject) => { worker.once('message', resolve); worker.once('error', reject); worker.once('exit', code => { if (code) reject(new Error(`worker exited ${code}`)); }); }); }
    finally { await worker.terminate(); }
}
if (isMainThread) {
    const artifact = await runWorker({ operation: 'produce' });
    const directory = await mkdtemp(join(tmpdir(), 'cem-native-values-'));
    try {
        const path = join(directory, 'values.cemv');
        await writeFile(path, new Uint8Array(artifact.bytes));
        const saved = await readFile(path);
        const restored = { ...artifact, bytes: saved.buffer.slice(saved.byteOffset, saved.byteOffset + saved.byteLength) };
        assert.equal(await runWorker({ operation: 'consume', artifact: restored }), true);
        await ready();
        assert.equal(consume(restored), true);
        console.log('Native CEM values: separate workers, saved pipeline, fallback and disposal passed.');
    } finally { await rm(directory, { recursive: true, force: true }); }
} else {
    await ready();
    parentPort.postMessage(workerData.operation === 'produce' ? produce() : consume(workerData.artifact));
}
