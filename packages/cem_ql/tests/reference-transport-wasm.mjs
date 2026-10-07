// Low-level source handles stay local; workers transfer explicit bundle/CEMV bytes.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';
import * as wasm from '../dist/wasm/cem_ql.js';

await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const encode = text => new TextEncoder().encode(text);
const entityMarkup = "<item xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target' target='{#(items &lt;)}'/>";
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

function xmlSlots(bundle, nativeReference) {
    const source = wasm.importReferenceReloadBundle(new Uint8Array(bundle), 1, '');
    const schema = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=item @required-attributes=target @optional-attributes=literal}} {attributes | {attribute @name=target @type=node} {attribute @name=literal @type=string}}}'), 'text/cem-ml', 'memory:xml-schema.cem', '');
    const session = wasm.beginReferenceValidationSession(source, schema);
    const other = wasm.beginReferenceValidationSession(source, schema);
    const run = () => JSON.parse(wasm.runReferenceValidationSession(session));
    assert.equal(run().complete, false, 'reload supplies no execution context');
    wasm.setReferenceValidationContext(session, 0, true, '[]');
    const snapshot = wasm.prepareReferenceQuerySnapshot(session);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(snapshot)).complete, true);
    const authored = wasm.queryReferenceQuerySnapshot(snapshot, 'seq:where(input.attributes, fn(a) => a.name == "target").valueNodes', 'memory:xml-slot.cemql');
    const inspectAuthored = () => {
        if (nativeReference) fail(() => wasm.exportNativeValueArtifact(authored, ''), 'cem.value.unsupported_source_reference');
    };
    inspectAuthored();
    const kind = wasm.queryReferenceQuerySnapshot(snapshot, 'seq:where(input.attributes, fn(a) => a.name == "target").valueNodes.kind', 'memory:xml-kind.cemql');
    assert.ok(wasm.exportNativeValueArtifact(kind, '').length);
    const literal = wasm.queryReferenceQuerySnapshot(snapshot, 'seq:where(input.attributes, fn(a) => a.name == "literal").value', 'memory:xml-literal.cemql');
    assert.ok(new TextDecoder().decode(wasm.exportNativeValueArtifact(literal, '')).includes('{#items}'));
    const library = wasm.parseReferenceSource(encode('{target | {#not-consumed}}'), 'text/cem-ml', 'memory:xml-target.cem', '');
    const targets = wasm.queryReferenceSource(library, 'input.children', 'memory:xml-bindings.cemql');
    const destination = wasm.registerReferenceValidationSource(session, library);
    wasm.setReferenceValidationContext(session, destination, true, '[]');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name:'items', valueId:targets}]));
    assert.equal(run().complete, false, 'transport creates no crossing grant');
    wasm.allowReferenceValidationCrossing(session, 0, destination);
    const result = run();
    assert.equal(result.complete, true, JSON.stringify(result));
    assert.equal(result.failed, false);
    assert.equal(JSON.parse(wasm.runReferenceValidationSession(other)).complete, false);
    // The export still contains authored slots, never the evaluated targets.
    const exported = wasm.exportReferenceReloadBundle(source, '');
    assert.deepEqual(exported, new Uint8Array(bundle));
    for (const id of [session, other]) wasm.disposeReferenceValidationSession(id);
    for (const id of [source, schema, library]) wasm.disposeReferenceSource(id);
    wasm.disposeReferenceQuerySnapshot(snapshot);
    inspectAuthored();
    for (const id of [authored, kind, literal, targets]) wasm.disposeNativeValueArtifact(id);
    return exported;
}
function xmlEntityDiagnostics(bundle) {
    const source = wasm.importReferenceReloadBundle(new Uint8Array(bundle), 1, '');
    const schema = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=item @required-attributes=target}} {attributes | {attribute @name=target @type=node}}}'), 'text/cem-ml', 'memory:xml-schema.cem', '');
    const session = wasm.beginReferenceValidationSession(source, schema);
    const empty = wasm.queryReferenceSource(source, '()', 'memory:empty.cemql');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name:'items',valueId:empty}]));
    const report = JSON.parse(wasm.runReferenceValidationSession(session));
    assert.equal(report.complete, false);
    assert.equal(report.failed, true);
    const attributed = report.diagnostics.find(d => d.uri === 'memory:entity.xml' && d.sourceMap?.frames.some(f => f.transform.kind === 'ExpressionEmbedding'));
    assert.ok(attributed, JSON.stringify(report));
    assert.ok(attributed.sourceMap.frames.some(f => f.transform.kind === 'ExpressionEmbedding' && f.span.ranges.start === entityMarkup.indexOf('&lt;') && f.span.ranges.len === 4), JSON.stringify(attributed));
    wasm.disposeNativeValueArtifact(empty); wasm.disposeReferenceValidationSession(session);
    wasm.disposeReferenceSource(source); wasm.disposeReferenceSource(schema);
}

function resources() {
    const input = wasm.parseReferenceSource(encode('{host @schema-src=child.cem | {leaf}}'), 'text/cem-ml', 'https://vendor.test/main.cem', '');
    const schema = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=host @children=leaf}}}'), 'text/cem-ml', 'memory:base.cem', '');
    const parent = wasm.beginReferenceValidationSession(input, schema);
    wasm.setReferenceValidationContext(parent, 0, true, '[]');
    const run = wasm.startReferenceResourceExecution(parent);
    const request = JSON.parse(wasm.advanceReferenceResourceExecution(run));
    assert.equal(request.state, 'awaitResources');
    assert.equal(request.result[0].uri, 'https://vendor.test/child.cem');
    const id = request.result[0].id;
    fail(() => wasm.advanceReferenceResourceExecution(run), 'cem.reference.validation');
    fail(() => wasm.completeReferenceResource(run, id + 1, encode('{schema}'), 'text/cem-ml', 'https://vendor.test/child.cem'), 'cem.reference.validation');
    const bytes = encode('@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=leaf}}}');
    const loaded = JSON.parse(wasm.completeReferenceResource(run, id, bytes, 'text/cem-ml', 'https://vendor.test/child.cem'));
    assert.equal(JSON.parse(wasm.inspectReferenceSource(loaded.sourceId)).lexicalReady, true);
    fail(() => wasm.completeReferenceResource(run, id, bytes, 'text/cem-ml', 'https://vendor.test/child.cem'), 'cem.reference.validation');
    wasm.setReferenceResourceContext(run, loaded.sourceIndex, true, '[]');
    wasm.allowReferenceResourceCrossing(run, 0, loaded.sourceIndex);
    wasm.disposeReferenceSource(loaded.sourceId);
    wasm.disposeReferenceSource(input);
    wasm.disposeReferenceSource(schema);
    wasm.disposeReferenceValidationSession(parent);
    const result = JSON.parse(wasm.advanceReferenceResourceExecution(run));
    assert.equal(result.state, 'finished');
    assert.equal(result.result.complete, true, JSON.stringify(result));
    wasm.disposeReferenceResourceExecution(run);
    fail(() => wasm.advanceReferenceResourceExecution(run), 'cem.reference.validation');
}
function snapshots() {
    const source = wasm.parseReferenceSource(encode('@ns public = urn:vendor\n{item @xmlns:p={#namespace} | {p:item} {#later}}'), 'text/cem-ml', 'memory:names.cem', '');
    const schema = wasm.parseReferenceSource(encode('{schema}'), 'text/cem-ml', 'memory:schema.cem', '');
    const session = wasm.beginReferenceValidationSession(source, schema);
    const pending = wasm.prepareReferenceQuerySnapshot(session);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(pending)).complete, false);
    const namespace = wasm.queryReferenceSource(source, 'seq:where(input.children, fn(node) => node.kind == "element" && node.name == "@ns")', 'memory:bindings.cemql');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name:'namespace', valueId:namespace}]));
    const ready = wasm.prepareReferenceQuerySnapshot(session);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(ready)).complete, true);
    const values = wasm.queryReferenceQuerySnapshot(ready, 'input.children.namespace', 'memory:completed.cemql');
    const artifact = wasm.exportNativeValueArtifact(values, '');
    assert.ok(new TextDecoder().decode(artifact).includes('urn:vendor'));
    wasm.disposeReferenceQuerySnapshot(ready);
    assert.ok(wasm.exportNativeValueArtifact(values, '').length);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(pending)).complete, false);
    fail(() => wasm.queryReferenceQuerySnapshot(ready, 'input', 'memory:stale.cemql'), 'cem.reference.validation');
    for (const id of [namespace, values]) wasm.disposeNativeValueArtifact(id);
    wasm.disposeReferenceQuerySnapshot(pending); wasm.disposeReferenceValidationSession(session);
    wasm.disposeReferenceSource(source); wasm.disposeReferenceSource(schema);
}
function hostAdapters() {
    const source = wasm.parseReferenceSource(encode('{one | {#items}}{two | {#items}}'), 'text/cem-ml', 'memory:occurrences.cem', '');
    const schema = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=one @children=item}{element @name=two @children=item}{element @name=item}}}'), 'text/cem-ml', 'memory:schema.cem', '');
    const session = wasm.beginReferenceValidationSession(source, schema);
    const empty = wasm.queryReferenceSource(source, '()', 'memory:context.cemql');
    const occurrence = wasm.queryReferenceSource(source, 'input.children', 'memory:occurrences.cemql');
    const library = wasm.parseReferenceSource(encode('{item}'), 'text/cem-ml', 'memory:library.cem', '');
    const destination = wasm.registerReferenceValidationSource(session, library);
    const targets = wasm.queryReferenceSource(library, 'input.children', 'memory:targets.cemql');
    wasm.setReferenceValidationContext(session, 0, true, JSON.stringify([{name:'items',valueId:empty}]));
    wasm.setReferenceValidationContext(session, destination, true, '[]');
    wasm.allowReferenceValidationCrossing(session,0,destination);
    wasm.setReferenceValidationOccurrenceContext(session,0,occurrence,0,true,JSON.stringify([{name:'items',valueId:targets}]));
    assert.equal(JSON.parse(wasm.runReferenceValidationSession(session)).complete,true);
    wasm.setReferenceValidationOccurrenceContext(session,0,occurrence,1,false,'[]');
    assert.equal(JSON.parse(wasm.runReferenceValidationSession(session)).complete,false);
    wasm.clearReferenceValidationOccurrenceContext(session,0,occurrence,1);
    assert.equal(JSON.parse(wasm.runReferenceValidationSession(session)).complete,true);
    fail(() => wasm.setReferenceValidationOccurrenceContext(session,0,targets,0,true,'[]'),'cem.reference.validation');
    for(const id of [empty,occurrence,targets]) wasm.disposeNativeValueArtifact(id);
    wasm.disposeReferenceValidationSession(session);
    for(const id of [source,schema,library]) wasm.disposeReferenceSource(id);

    const input = wasm.parseReferenceSource(encode('{host @schema-src=child.cem#leaf | {leaf}}'), 'text/cem-ml', 'https://vendor.test/main.cem', '');
    const base = wasm.parseReferenceSource(encode('{schema | {elements | {element @name=host @children=leaf}}}'), 'text/cem-ml', 'memory:base.cem', '');
    const parent = wasm.beginReferenceValidationSession(input,base);
    wasm.setReferenceValidationContext(parent,0,true,'[]');
    const run = wasm.startReferenceResourceExecution(parent);
    const request = JSON.parse(wasm.advanceReferenceResourceExecution(run)).result[0];
    const bytes = encode('@ns s = https://cem.dev/ns/schema/1\n{s:schema | {elements | {element @name=leaf}}}\n@ns public = urn:loaded\n{extras @xmlns:p={#namespace} | {p:item}{#later}}');
    fail(() => wasm.completeReferenceResourceWithExports(run,request.id,bytes,'text/cem-ml','https://vendor.test/child.cem',JSON.stringify([{part:'leaf',select:'input.children',grant:true}])),'cem.reference.validation');
    const loaded = JSON.parse(wasm.completeReferenceResourceWithExports(run,request.id,bytes,'text/cem-ml','https://vendor.test/child.cem',JSON.stringify([{part:'leaf',select:'seq:where(input.children, fn(node) => node.kind == "element" && node.name == "schema")'}])));
    const pendingNames = wasm.prepareReferenceResourceQuerySnapshot(run, loaded.sourceIndex);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(pendingNames)).complete, false);
    const namespace = wasm.queryReferenceSource(loaded.sourceId, 'seq:last(seq:where(input.children, fn(node) => node.kind == "element" && node.name == "@ns"))', 'memory:loaded-bindings.cemql');
    wasm.setReferenceResourceContext(run,loaded.sourceIndex,true,JSON.stringify([{name:'namespace',valueId:namespace}]));
    assert.equal(JSON.parse(wasm.prepareReferenceResourceNamespaces(run,loaded.sourceIndex)).complete,true);
    const snapshot = wasm.prepareReferenceResourceQuerySnapshot(run,loaded.sourceIndex);
    const nodes = wasm.queryReferenceQuerySnapshot(snapshot,'seq:where(input, fn(node) => node.kind == "element" && node.name == "schema")','memory:loaded.cemql');
    wasm.setReferenceResourceOccurrenceContext(run,loaded.sourceIndex,nodes,0,false,'[]');
    wasm.clearReferenceResourceOccurrenceContext(run,loaded.sourceIndex,nodes,0);
    assert.equal(JSON.parse(wasm.prepareReferenceResourceNamespaces(run,loaded.sourceIndex)).complete,true);
    wasm.allowReferenceResourceCrossing(run,0,loaded.sourceIndex);
    assert.equal(JSON.parse(wasm.advanceReferenceResourceExecution(run)).result.complete,true);
    fail(() => wasm.prepareReferenceResourceNamespaces(run, loaded.sourceIndex), 'cem.reference.validation');
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(pendingNames)).complete, false);
    const completedNames = wasm.queryReferenceQuerySnapshot(snapshot, 'input.children.namespace', 'memory:loaded-names.cemql');
    assert.ok(new TextDecoder().decode(wasm.exportNativeValueArtifact(completedNames, '')).includes('urn:loaded'));
    const saved = wasm.prepareReferenceResourceQuerySnapshot(run,0);
    wasm.disposeReferenceResourceExecution(run);
    wasm.disposeReferenceValidationSession(parent);
    for(const id of [loaded.sourceId,input,base]) wasm.disposeReferenceSource(id);
    const names = wasm.queryReferenceQuerySnapshot(snapshot,'input.children.name','memory:saved.cemql');
    assert.ok(wasm.exportNativeValueArtifact(names,'').length);
    assert.equal(JSON.parse(wasm.inspectReferenceQuerySnapshot(saved)).complete,true);
    for(const id of [nodes,names,namespace,completedNames]) wasm.disposeNativeValueArtifact(id);
    for(const id of [snapshot,saved,pendingNames]) wasm.disposeReferenceQuerySnapshot(id);
}
hostAdapters();
resources();
snapshots();

lifecycle();

if (isMainThread) {
    const source = wasm.parseReferenceSource(encode('{div}{#input}'), 'text/cem-ml', 'memory:original.cem', '');
    const bytes = wasm.exportReferenceReloadBundle(source, '');
    const xmlBundles = ['#items','items'].map(expression => {
        const xml = wasm.parseReferenceSource(encode(`<item xmlns:c='https://cem.dev/ns/core/1' c:expression-attributes='target' target='{${expression}}' literal='{#items}'/>`), 'application/xml', 'memory:slots.xml', '');
        const bundle = wasm.exportReferenceReloadBundle(xml, '');
        wasm.disposeReferenceSource(xml);
        return xmlSlots(bundle, expression === '#items').slice().buffer;
    });
    const invalid = wasm.parseReferenceSource(encode(entityMarkup), 'application/xml', 'memory:entity.xml', '');
    const entity = wasm.exportReferenceReloadBundle(invalid, '');
    wasm.disposeReferenceSource(invalid);
    xmlEntityDiagnostics(entity);
    const worker = new Worker(new URL(import.meta.url), { workerData: { bundle: bytes.slice().buffer, xmlBundles, entity: entity.slice().buffer } });
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
    console.log('reference transport: loaded namespaces, XML slots/entities, independent worker contexts/grants, reload and disposal passed');
} else {
    for (const [index, bundle] of workerData.xmlBundles.entries()) xmlSlots(bundle, index === 0);
    xmlEntityDiagnostics(workerData.entity);
    parentPort.postMessage(consume(workerData.bundle));
}
