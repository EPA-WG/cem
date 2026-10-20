import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Exercise the actual shared host store with two independent generated WASM
// glue modules/memories. No mock sealing or WASM-memory snapshots.
const source = await readFile(new URL('../../cem-elements/src/lib/committed-checkpoint.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { CemCommittedCheckpointStore } = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const wasmBytes = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const load = async generation => {
    const url = new URL('../dist/wasm/cem_ql.js', import.meta.url); url.searchParams.set('checkpoint-generation', generation);
    const wasm = await import(url.href);
    const native = await wasm.default({ module_or_path: wasmBytes });
    return { wasm, native };
};
const receipt = { runtimeId: 'runtime-a', generation: 1, commitSequence: 3, policyStamp: 'session-only',
    owners: [{ instanceId: 'editor', declarationIdentity: 'source:1', dataRevision: 'draft:2', acceptedEventSequence: 2 }],
    operations: [{ instanceId: 'editor', operationId: 'save:2', outcome: 'unknown' }] };
const candidate = { version: 'cem-committed-checkpoint/1', receipt,
    owners: [{ instanceId: 'editor', island: '<template data-cem-data-island>unsaved é text</template>' }] };
const store = new CemCommittedCheckpointStore('runtime-a');
store.acknowledge(receipt);
const runtimeState = { version: 'cem-checkpoint-runtime/1', capability: null,
    resources: [{ name: 'remote', kind: 'http', disposition: 'reimport-and-read-again', revision: 1, cursor: 0, live: false, state: 'loaded',
        url: 'https://example.test/source.xml', method: 'GET', document: { base64: Buffer.from('<data>é😀</data>').toString('base64'),
            contentType: 'application/xml', sourceUri: 'https://example.test/source.xml', digest: null } }],
    writers: [{ index: 0, trigger: null, implicit: true }], writeDisposition: 'suppress-until-new-input' };
let portableRuntime;
let previousMemory;
{
    const { wasm, native } = await load(1);
    previousMemory = native.memory;
    const sample = JSON.parse(wasm.runtimeMemoryStatus()).sample;
    const observed = Math.max(sample.allocatorLiveBytes, sample.linearMemoryBytes);
    // Ordinary admission is stopped; a bounded transport reservation is allowed.
    wasm.configureRuntimeMemoryBudget(JSON.stringify({ softLimitBytes: 1, hardLimitBytes: observed + 32 * 1024 * 1024,
        recoveryHeadroomBytes: 32 * 1024 * 1024, maxConcurrentOperations: 2 }));
    assert.ok(JSON.parse(wasm.compileTemplate('{p | refused}', '[]')).diagnostics.some(d => d.code === 'cem.runtime.memory.admission'));
    const controls = { version: 'cem-checkpoint-controls/1', controls: [{ id: 'editor:1', kind: 'text', inputType: 'plaintext',
        value: 'Draft é😀', checked: false, indeterminate: false, multiple: false, options: [],
        selection: { start: 1, end: 3, direction: 'backward' }, focused: true, disabled: false, readOnly: true, customValidity: '' }] };
    assert.deepEqual(JSON.parse(wasm.validateCheckpointControls(JSON.stringify(controls))), controls);
    const secret = structuredClone(controls); secret.controls[0].inputType = 'password';
    assert.throws(() => wasm.validateCheckpointControls(JSON.stringify(secret)));
    const invalidSelection = structuredClone(controls); invalidSelection.controls[0].selection.end = 1000;
    assert.throws(() => wasm.validateCheckpointControls(JSON.stringify(invalidSelection)));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);

    portableRuntime = wasm.validateCheckpointRuntime(JSON.stringify(runtimeState));
    assert.match(JSON.parse(portableRuntime).resources[0].document.digest, /^[a-f0-9]{64}$/);
    const badRuntime = structuredClone(runtimeState); badRuntime.resources[0].url = 'https://user:secret@example.test/source.xml';
    assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(badRuntime)));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);
    await store.capture(JSON.stringify(candidate), wasm.sealCommittedCheckpoint);
    assert.notEqual(store.current().bytes.buffer, native.memory.buffer);
    const changed = structuredClone(candidate); changed.receipt.owners[0].dataRevision = 'not-committed';
    await assert.rejects(store.capture(JSON.stringify(changed), wasm.sealCommittedCheckpoint));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);
    // Even recovery refuses if there is no physical room; last-good host state survives.
    wasm.configureRuntimeMemoryBudget(JSON.stringify({ softLimitBytes: 1, hardLimitBytes: 65536,
        recoveryHeadroomBytes: 32768, maxConcurrentOperations: 1 }));
    await assert.rejects(store.capture(JSON.stringify(candidate), wasm.sealCommittedCheckpoint));
}
const saved = store.current();
assert.equal(JSON.parse(new TextDecoder().decode(saved.bytes)).owners[0].island, candidate.owners[0].island);
const { wasm, native } = await load(2);
assert.notEqual(native.memory, previousMemory);
const resealed = wasm.sealCommittedCheckpoint(new TextDecoder().decode(saved.bytes), JSON.stringify(saved.receipt));
assert.deepEqual(JSON.parse(resealed), candidate);
assert.equal(JSON.parse(resealed).receipt.operations[0].outcome, 'unknown');
assert.equal(wasm.validateCheckpointRuntime(portableRuntime), portableRuntime);
const body = JSON.parse(portableRuntime).resources[0].document;
const document = wasm.retainCemDocument(Buffer.from(body.base64, 'base64'), body.contentType, body.sourceUri);
assert.ok(document > 0); assert.ok(wasm.disposeCemDocument(document));
const terminal = structuredClone(candidate); terminal.receipt.operations[0].outcome = 'not-applied';
assert.equal(JSON.parse(wasm.sealCommittedCheckpoint(JSON.stringify(terminal), JSON.stringify(terminal.receipt))).receipt.operations[0].outcome, 'not-applied');
const widget = { ...JSON.parse(portableRuntime), capability: { kind: 'tree-view', focusValue: 'docs', typeahead: 'd', typeaheadAt: 1 },
    focus: { target: 'node', nodeId: 'tree.1' } };
assert.deepEqual(JSON.parse(wasm.validateCheckpointRuntime(JSON.stringify(widget))), widget);
const dependent = structuredClone(candidate);
dependent.receipt.relationships = [{ id: 'control', kind: 'placement', disposition: 'reacquire', property: 'aria-controls',
    requester: { instanceId: 'editor', dataRevision: 'draft:2', nodeId: 'button.1' },
    provider: { instanceId: 'editor', dataRevision: 'draft:2', nodeId: 'panel.1' } }];
assert.deepEqual(JSON.parse(wasm.sealCommittedCheckpoint(JSON.stringify(dependent), JSON.stringify(dependent.receipt))), dependent);
dependent.receipt.relationships[0].provider.dataRevision = 'stale';
assert.throws(() => wasm.sealCommittedCheckpoint(JSON.stringify(dependent), JSON.stringify(dependent.receipt)));
const active = { ...runtimeState, resources: [], capability: { kind: 'suggestions', state: {
    source: { version: 'cem-native-source/1', revision: 'options:1', rowCount: 2, select: 'input.children', resolve: false, requesting: 0,
        sources: [{ codec: 'xml', base64: Buffer.from('<options><option value="same">First</option><option value="same">Second</option></options>').toString('base64'), sourceUri: 'https://example.test/options.xml', primarySourceId: 1, context: true }],
        bindings: [], crossings: [], labels: { option: null, group: null }, disposition: 'reacquire' },
    publication: { filter: 'none', filterBy: null, expanded: null, active: null, committed: null },
    providerRevision: 0, preparedRevision: 0, queryRevision: 1, query: 'same', intent: false, feedbackIntent: false, feedbackState: 'ready', activeIndex: -1,
    committedIndex: 1, committedValue: 'same', visible: false, scrollTop: 0, scrollLeft: 0,
    rows: [{ index: 0, value: 'same', eligible: true, available: true, nodeId: 'row.0' }, { index: 1, value: 'same', eligible: true, available: true, nodeId: 'row.1' }] } } };
const activeSaved = wasm.validateCheckpointRuntime(JSON.stringify(active));
assert.equal(JSON.parse(activeSaved).capability.state.committedIndex, 1);
assert.equal(wasm.validateCheckpointRuntime(activeSaved), activeSaved);
const completedCommit = { ...runtimeState, resources: [], capability: { kind: 'form-control', authoredValue: null, formDisabled: false, customValidity: '', selectionRequest: null,
    provider: { revision: 4, consumer: 'suggestions', validity: '' }, changeSuppression: { revision: 4, value: 'same', disposition: 'suppress-once' } } };
assert.equal(JSON.parse(wasm.validateCheckpointRuntime(JSON.stringify(completedCommit))).capability.changeSuppression.disposition, 'suppress-once');
completedCommit.capability.changeSuppression.revision = 3; assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(completedCommit)));
const invalidActive = structuredClone(active); invalidActive.capability.state.source.sources[0].digest = 'obsolete';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(invalidActive)));
// Execution/resume is deliberately not performed by the transport store.
store.invalidate(); assert.equal(store.current(), undefined);
assert.ok(store.lastCommitted()); store.dispose(); assert.equal(store.lastCommitted(), undefined);
console.log('Committed checkpoint real WASM: headroom, receipt/control/runtime validation, source reimport, terminal outcomes and independent host bytes passed');
