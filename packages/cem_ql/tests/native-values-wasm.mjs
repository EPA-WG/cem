// CEMT-VALUE-TRANSPORT: real separate WASM workers, saved bytes and main-thread fallback.
import assert from 'node:assert/strict';
import { readFile, writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';
const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const ready = () => readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)).then(bytes => wasm.default({ module_or_path: bytes }));
const referenceInput = `{cem:variable @name=n @select='data:read("<name>ivy</name>", "xml").root.children'}{cem:variable @name=refs @select='dom:reference((n, n))'}`;
const source = referenceInput + `{template @mode=label @match=true | {$node}}{template @on=expression @into=attribute @match='context.attribute.name == "count"' @returns=integer | {$value + 1}}{child | {attribute @name=count @type=integer | {cem:if @test=true | {$1}}}{attribute @name=day @type=date @value=2024-02-29}{attribute @name=label @type=node @content-type=text/html | {cem:if @test=true | {$cemt:apply_templates(data:read("<name>ivy<em>saur</em></name>", "xml").root.children, "label")}}}{attribute @name=empty @type=any | {cem:if @test=true | {$()}}}{attribute @name=large @type=integer @value=922337203685477580812345}{attribute @name=refs @type=node @value='{refs}'}}`;
function verifyReferenceConstructors(render) {
    const plan = render(`{cem:variable @name=copy @select=dom:clone(refs)}{p | {$seq:count(copy)}|{$copy.kind}|{$seq:count(copy.targets)}|{$seq:first(copy.targets).id == seq:last(copy.targets).id}|{$seq:first(copy.targets).id == seq:first(refs.targets).id}}{b | {$dom:element(refs.targets)}}{i | {$seq:count(dom:clone(refs.targets))}}`);
    assert.deepEqual(plan.diagnostics, []);
    assert.equal(plan.nodes[0].children.map(node => node.text).join(''), '1|reference|2|true|false');
    assert.deepEqual(plan.nodes[1].children.map(node => [node.tag, node.children.length]), [['name', 0], ['name', 0]]);
    assert.equal(plan.nodes[2].children[0].text, '2');
    const rejected = render('{$dom:element(refs)}');
    assert.deepEqual(rejected.nodes, []);
    assert.ok(rejected.diagnostics.some(d => d.code === 'cem.ql.type_error'));
}
function verifyDirectReferences() {
    verifyReferenceConstructors(template => JSON.parse(wasm.renderTemplateSource(referenceInput + template, '{}')));
}
function verifyDirectInteger() {
    const plan = JSON.parse(wasm.renderTemplateSource('{attribute @name=large @type=integer | 922337203685477580812345}{p | {$large + 1.0}|{$large is decimal}}', '{}'));
    assert.deepEqual(plan.diagnostics, []);
    assert.equal(plan.nodes[0].children.map(node => node.text).join(''), '922337203685477580812346|true');
    if (plan.nativeValueArtifactId) wasm.takeRenderValueArtifact(plan.nativeValueArtifactId);
}
function verifyElementReferenceIds() {
    const source = `{cem:variable @name=target @select='data:read("<dialog><b>body</b></dialog>", "xml").root.children'}{button @commandfor={#target}}{cem-action @command-target={#target} @command=show-modal @focus-target={#target} @return-focus={#target} @anchor={#target} @boundary={#target}}{span @aria-controls={#(target, target)}}{$target}`;
    const artifact = JSON.parse(wasm.compileTemplate(source, '[]'));
    assert.deepEqual(artifact.diagnostics, []);
    try {
        for (const instance of ['instance-a', 'instance-b', 'instance-a']) {
            const result = JSON.parse(wasm.renderTemplateWithNativeValues(artifact.artifactId, 0, '{}', '[]', '[]', limits, instance));
            assert.deepEqual(result.diagnostics, []);
            const id = `${instance}-ref-3`;
            const value = (node, name) => node.attributes.find(a => a.name === name)?.value;
            assert.equal(value(result.nodes[0], 'commandfor'), id);
            assert.equal(value(result.nodes[1], 'command-target'), id);
            assert.equal(value(result.nodes[1], 'data-cem-node-ref-command-target'), '');
            for (const name of ['focus-target', 'return-focus', 'anchor', 'boundary']) {
                assert.equal(value(result.nodes[1], name), id);
                assert.equal(value(result.nodes[1], `data-cem-node-ref-${name}`), '');
            }
            assert.equal(value(result.nodes[2], 'aria-controls'), `${id} ${id}`);
            assert.equal(value(result.nodes[3], 'id'), id);
        }
        const ambiguous = JSON.parse(wasm.compileTemplate(source + '{$target}', '[]'));
        try {
            const rejected = JSON.parse(wasm.renderTemplateWithNativeValues(ambiguous.artifactId, 0, '{}', '[]', '[]', limits, 'instance'));
            assert.deepEqual(rejected.nodes, []);
            assert.ok(rejected.diagnostics.some(d => d.code === 'cem.element_reference.target_ambiguous'));
        } finally { wasm.disposeTemplate(ambiguous.artifactId); }
    } finally { wasm.disposeTemplate(artifact.artifactId); }
}
function verifyElementReferenceLifecycle() {
    const parse = (text, uri) => wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', uri, '');
    const relation = parse('{#datadom.slices.destination}', 'memory:element-relation.cem');
    const target = parse('{dialog | Native target}', 'memory:element-target.cem');
    const artifact = JSON.parse(wasm.compileTemplate('{slice @name=relation}{slice @name=destination}{button @commandfor={#relation} @aria-controls={#relation}}{$destination}', '[]'));
    const input = { requesting: 0, sources: [{ sourceId: relation, context: true }, { sourceId: target, context: true }],
        bindings: [{ source: 0, name: 'relation', select: 'input.children' }, { source: 1, name: 'destination', select: 'input.children' }], grants: [[0, 1]] };
    const render = (metadata, instance = 'lifecycle') => JSON.parse(wasm.renderTemplateWithNativeValues(artifact.artifactId, 0, '{}', '[]', '[]', limits, instance, JSON.stringify(metadata)));
    try {
        for (const bad of [ { ...input, grants: [] }, { ...input, sources: [{ sourceId: relation, context: false }, input.sources[1]] },
            { ...input, sources: [input.sources[0], { sourceId: target, context: true, maxWork: 1 }] } ]) {
            const result = render(bad);
            assert.equal(result.referenceProjectionComplete, false);
            assert.deepEqual(result.nodes, []);
        }
        for (const instance of ['first', 'second', 'first']) {
            const result = render(input, instance);
            assert.deepEqual(result.diagnostics, []);
            assert.equal(result.nodes[0].attributes.find(a => a.name === 'commandfor').value, `${instance}-ref-1`);
            assert.equal(result.nodes[1].attributes.find(a => a.name === 'id').value, `${instance}-ref-1`);
        }
        const noInstance = JSON.parse(wasm.renderTemplateWithNativeValues(artifact.artifactId, 0, '{}', '[]', '[]', limits, undefined, JSON.stringify(input)));
        assert.equal(noInstance.referenceProjectionComplete, false);
        assert.equal(noInstance.referenceProjectionCode, 'cem.element_reference.inputs');
        for (const unresolved of ['warning', 'ignore']) {
            const result = render({ ...input, grants: [], sources: [{ sourceId: relation, context: true, unresolved }, { sourceId: target, context: true, unresolved }] });
            assert.equal(result.referenceProjectionComplete, false);
            assert.deepEqual(result.nodes, []);
            assert.ok(result.diagnostics.every(d => d.severity !== 'error'));
            if (unresolved === 'warning') assert.ok(result.diagnostics.some(d => d.severity === 'warning'));
        }
        assert.equal(JSON.parse(wasm.inspectReferenceSource(relation)).lexicalReady, true);
        // Source handles are heap local, disposable and never grant authority.
        wasm.disposeReferenceSource(target);
        assert.equal(render(input).referenceProjectionComplete, false);
    } finally { wasm.disposeReferenceSource(relation); wasm.disposeReferenceSource(target); wasm.disposeTemplate(artifact.artifactId); }
}
function verifyPlacementAdmissions() {
    const parse = (text, uri) => wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', uri, '');
    const relation = parse('{#datadom.slices.destination}', 'memory:foreign-relation.cem');
    const target = parse('{dialog}', 'memory:foreign-target.cem');
    const compiled = JSON.parse(wasm.compileTemplate('{slice @name=relation}{slice @name=destination}{button @commandfor={#relation}}', '[]'));
    const input = { requesting: 0, sources: [{ sourceId: relation, context: true }, { sourceId: target, context: true }],
        bindings: [{ source: 0, name: 'relation', select: 'input.children' }, { source: 1, name: 'destination', select: 'input.children' }], grants: [[0, 1]],
        placements: { admissions: [{ source: 1, select: 'input.children', token: 'placement', producer: 'owner', path: [0], revision: '1', id: 'owner-ref-0' }],
            grants: [{ requester: 'consumer', token: 'placement', properties: ['commandfor'] }], committedRevisions: { owner: '1' } } };
    const render = metadata => JSON.parse(wasm.renderTemplateWithNativeValues(compiled.artifactId, 0, '{}', '[]', '[]', limits, 'consumer', JSON.stringify(metadata)));
    try {
        const ready = render(input);
        assert.deepEqual(ready.diagnostics, []);
        assert.equal(ready.nodes[0].attributes.find(a => a.name === 'commandfor').value, 'owner-ref-0');
        assert.equal(ready.nodes[0].attributes.find(a => a.name === 'data-cem-placement-ref-commandfor').value, '');
        assert.deepEqual(ready.elementPlacementUses, [{ token: 'placement', producer: 'owner', revision: '1', id: 'owner-ref-0', path: [0], attribute: 'commandfor' }]);
        const staged = { ...input, placements: { ...input.placements, committedRevisions: {}, preparedTransaction: {
            token: 'host-transaction', participants: ['owner', 'consumer'], producerRevisions: { owner: '1', consumer: '1' },
        } } };
        const prepared = render(staged);
        assert.deepEqual(prepared.diagnostics, []);
        assert.equal(prepared.elementPlacementUses[0].transaction, 'host-transaction');
        assert.equal(render({ ...staged, placements: { ...staged.placements, preparedTransaction: undefined } }).referenceProjectionComplete, false);
        for (const bad of [{ ...input, grants: [] }, { ...input, placements: { ...input.placements, grants: [] } },
            { ...input, placements: { ...input.placements, committedRevisions: { owner: '2' } } },
            { ...input, placements: { ...input.placements, grants: [{ requester: 'consumer', token: 'placement', properties: ['anchor'] }] } }]) {
            const rejected = render(bad); assert.equal(rejected.referenceProjectionComplete, false); assert.deepEqual(rejected.nodes, []);
        }
    } finally { wasm.disposeReferenceSource(relation); wasm.disposeReferenceSource(target); wasm.disposeTemplate(compiled.artifactId); }
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
        verifyReferenceConstructors(source => {
            const probe = JSON.parse(wasm.compileTemplate('{attribute @name=refs @type=node @required=true}' + source, '["refs"]'));
            try {
                return JSON.parse(wasm.renderTemplateWithNativeValues(probe.artifactId, 0, '{}', '[]', native, limits));
            } finally { wasm.disposeTemplate(probe.artifactId); }
        });
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
        verifyDirectReferences();
        verifyElementReferenceIds();
        verifyElementReferenceLifecycle();
        verifyPlacementAdmissions();
        assert.equal(consume(restored), true);
        console.log('Native CEM values: separate workers, saved pipeline, fallback and disposal passed.');
    } finally { await rm(directory, { recursive: true, force: true }); }
} else {
    await ready();
    verifyDirectInteger();
    verifyDirectReferences();
    verifyElementReferenceIds();
        verifyElementReferenceLifecycle();
    verifyPlacementAdmissions();
    parentPort.postMessage(workerData.operation === 'produce' ? produce() : consume(workerData.artifact));
}
