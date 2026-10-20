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
// Execution/resume is deliberately not performed by the transport store.
store.invalidate(); assert.equal(store.current(), undefined);
assert.ok(store.lastCommitted()); store.dispose(); assert.equal(store.lastCommitted(), undefined);
console.log('Committed checkpoint real WASM: headroom, native receipt validation, independent host bytes and fresh-module import passed');
