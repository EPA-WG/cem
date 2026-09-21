// CEMT-VALUE-TRANSPORT: real separate WASM workers, saved bytes and main-thread fallback.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';
const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const ready = () => readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)).then(bytes => wasm.default({ module_or_path: bytes }));
const source = `{template @mode=label @match=true | {$node}}{template @on=expression @into=attribute @match='context.attribute.name == "count"' @returns=integer | {$value + 1}}{child | {attribute @name=count @type=integer | {cem:if @test=true | {$1}}}{attribute @name=day @type=date @value=2024-02-29}{attribute @name=label @type=node @content-type=text/html | {cem:if @test=true | {$cemt:apply_templates(data:read("<name>ivy<em>saur</em></name>", "xml").root.children, "label")}}}{attribute @name=empty @type=any | {cem:if @test=true | {$()}}}{attribute @name=large @type=integer @value=922337203685477580812345}}`;
function verifyDirectInteger() {
    const plan = JSON.parse(wasm.renderTemplateSource('{attribute @name=large @type=integer | 922337203685477580812345}{p | {$large + 1.0}|{$large is decimal}}', '{}'));
    assert.deepEqual(plan.diagnostics, []);
    assert.equal(plan.nodes[0].children.map(node => node.text).join(''), '922337203685477580812346|true');
    if (plan.nativeValueArtifactId) wasm.takeRenderValueArtifact(plan.nativeValueArtifactId);
}
function produce() {
    const plan = JSON.parse(wasm.renderTemplateSource(source, '{}'));
    assert.deepEqual(plan.diagnostics, []);
    const bytes = wasm.takeRenderValueArtifact(plan.nativeValueArtifactId).slice().buffer;
    assert.throws(() => wasm.takeRenderValueArtifact(plan.nativeValueArtifactId));
    return { bytes, bindings: plan.nodes[0].attributes.map(attribute => ({ name: attribute.name, index: attribute.nativeValueIndex })) };
}
function consume({ bytes, bindings }) {
    const id = wasm.importNativeValueArtifact(new Uint8Array(bytes), limits);
    const companion = JSON.parse(wasm.retainCemtXPathFunctions(`@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=value.text @visibility=public @returns=string |
{param @name=node @type=any @required=true}
{body | {xpath @context=node @sequence-type="xs:string" | {expression | string(.)}}}}}`, 'memory:native-value-functions.cemt'));

    const template = JSON.parse(wasm.compileTemplate('{attribute @name=count @type=integer @minInclusive=1 @required=true}{attribute @name=day @type=date @required=true}{attribute @name=label @type=node @required=true}{attribute @name=empty @type=any}{attribute @name=large @type=integer @required=true}{output | {$count + 1}{span | {$day}}{$label}{b | {$native:call("value.text", label)}}{i | {$seq:count(empty)}}{u | {$large + 1.0}|{$large is decimal}}}', JSON.stringify(bindings.map(b => b.name))));
    try {
        const native = JSON.stringify(bindings.map(b => ({ ...b, artifactId: id })));
        const plan = JSON.parse(wasm.renderTemplateWithNativeValues(template.artifactId, companion.companionId, '{}', '[]', native, limits));
        assert.deepEqual(plan.diagnostics, []);
        const invalid = JSON.parse(wasm.compileTemplate('{attribute @name=count @type=integer @minInclusive=3}{p | invalid}', '["count"]'));
        try {
            const rejected = JSON.parse(wasm.renderTemplateWithNativeValues(invalid.artifactId, 0, '{}', '[]', native, limits));
            assert.deepEqual(rejected.nodes, []);
            assert.ok(rejected.diagnostics.some(d => d.severity === 'error' || d.severity === 'Error'));
        } finally { wasm.disposeTemplate(invalid.artifactId); }
        const children = plan.nodes[0].children;
        assert.equal(children[0].text, '3');
        assert.equal(children[1].children[0].text, '2024-02-29');
        assert.equal(children[2].tag, 'name');
        assert.equal(children[2].children[1].tag, 'em');
        assert.equal(children[2].children[1].children[0].text, 'saur');
        assert.equal(children[3].children[0].text, 'ivysaur');
        assert.equal(children[4].children[0].text, '0');
        assert.equal(children[5].children.map(node => node.text).join(''), '922337203685477580812346|true');
        assert.equal(wasm.disposeNativeValueArtifact(id), true);
        const stale = JSON.parse(wasm.renderTemplateWithNativeValues(template.artifactId, companion.companionId, '{}', '[]', native, limits));
        assert.ok(stale.diagnostics.some(d => d.code === 'cem.value.binding'));
        assert.throws(() => wasm.importNativeValueArtifact(new Uint8Array(bytes), JSON.stringify({ maxBytes: 8, maxValues: 20, maxDepth: 10 })));
        return true;
    } finally { wasm.disposeNativeValueArtifact(id); wasm.disposeTemplate(template.artifactId); wasm.disposeCemtXPathFunctions(companion.companionId); }
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
        verifyDirectInteger();
        assert.equal(consume(restored), true);
        console.log('Native CEM values: separate workers, saved pipeline, fallback and disposal passed.');
    } finally { await rm(directory, { recursive: true, force: true }); }
} else {
    await ready();
    verifyDirectInteger();
    parentPort.postMessage(workerData.operation === 'produce' ? produce() : consume(workerData.artifact));
}
