import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';

// This is a test-only transport. It sends CEMQH1 bytes and independently emitted
// deployment pins, never a JSON source AST or a production recovery receipt.
async function load(directory, suffix) {
    const module = await import(`${pathToFileURL(resolve(directory, 'query_source_probe.js')).href}?heap=${suffix}`);
    module.initSync({ module: await readFile(resolve(directory, 'query_source_probe_bg.wasm')) });
    return module;
}
function exercise(runtime, source, pins) {
    const inventory = () => JSON.parse(runtime.probeInventory());
    assert.equal(inventory().liveRoots, 0);
    assert.throws(() => runtime.probeOpen(source, pins, false));
    assert.equal(inventory().liveRoots, 0, 'failed code selection leaked owners');
    for (const mutation of ['version', 'profile']) {
        const changed = JSON.parse(pins);
        if (mutation === 'version') changed.version += '/other';
        else changed.result_profile.hash = changed.result_profile.hash.slice(0, -1) + (changed.result_profile.hash.endsWith('a') ? 'b' : 'a');
        assert.throws(() => runtime.probeOpen(source, JSON.stringify(changed), true));
        assert.equal(inventory().liveRoots, 0, 'failed profile expectation leaked owners');
    }
    const corrupt = source.slice(); corrupt[corrupt.length - 1] ^= 1;
    assert.throws(() => runtime.probeOpen(corrupt, pins, true));
    assert.equal(inventory().liveRoots, 0);
    const first = runtime.probeOpen(source, pins, true);
    const peer = runtime.probeOpen(source, pins, true);
    assert.equal(runtime.probeCrossRefused(first, peer), true);
    assert.equal(inventory().liveRoots, 14);
    // The scoped ledger lists present roles; the complete census has its own
    // fixed registry manifest. Do not manufacture absent roles in this probe.
    assert.deepEqual(inventory().roots, { host: 2, compilation: 4, registry: 6, preparation: 2 });
    const report = JSON.parse(runtime.probeValidate(first, 'same'));
    assert.equal(report.accepted, true);
    assert.equal(report.diagnostics[0].code, 'query.note');
    assert.equal(report.diagnostics[0].uri, 'https://example.test/schema.cem');
    assert.equal(JSON.parse(runtime.probeValidate(first, 'a')).accepted, false);
    const canary = 'entered-password-not-source-69b3cce2';
    runtime.probeValidate(first, canary);
    const captured = runtime.probeCapture(first);
    assert.deepEqual(captured, new Uint8Array(source));
    assert.equal(Buffer.from(captured).includes(Buffer.from(canary)), false);
    const budget = runtime.probeOpenBudget(source, pins);
    let exhausted = false, completed = false;
    for (let n = 0; n < 100; n++) {
        const accepted = JSON.parse(runtime.probeValidate(budget, 'same')).accepted;
        if (accepted === null) { exhausted = true; break; }
        assert.equal(accepted, true); completed = true;
    }
    assert.ok(completed && exhausted, 'WASM request did not share its conservative native work budget');
    assert.equal(JSON.parse(runtime.probeValidate(budget, 'same')).accepted, null, 'an exhausted request reset its budget');
    runtime.probeDrop(budget);
    for (const mode of ['code', 'profile', 'registry']) {
        const roots = inventory().liveRoots, owner = runtime.probeOpen(source, pins, true);
        runtime.probeFence(owner, mode);
        assert.throws(() => runtime.probeCapture(owner));
        assert.throws(() => runtime.probeValidate(owner, 'same'));
        assert.equal(inventory().liveRoots, roots + 7, `${mode} retirement hid retained owners`);
        runtime.probeDrop(owner); assert.equal(inventory().liveRoots, roots);
    }
    // Source-only capture excludes input data. Enclosing Studio credential
    // refusal still applies; this probe does not authorize a recovery.
    runtime.probeFence(peer, 'context');
    assert.throws(() => runtime.probeValidate(peer, 'same'));
    assert.throws(() => runtime.probeCapture(peer));
    const cancelled = runtime.probeOpen(source, pins, true);
    runtime.probeFence(cancelled, 'cancel');
    assert.throws(() => runtime.probeValidate(cancelled, 'same'));
    assert.throws(() => runtime.probeCapture(cancelled));
    assert.equal(JSON.parse(runtime.probeValidate(first, 'same')).accepted, true);
    runtime.probeFence(first, 'retire');
    assert.equal(inventory().liveRoots, 21, 'retirement hid retained owners');
    assert.throws(() => runtime.probeValidate(first, 'same'));
    const active = runtime.probeOpen(captured, pins, true);
    const alias = runtime.probeValidate;
    runtime.probeClose();
    assert.equal(runtime.probeSeal(), false, 'scoped drain sealed while query roots lived');
    assert.throws(() => alias(active, 'same'));
    assert.throws(() => runtime.probeOpen(source, pins, true));
    assert.equal(inventory().liveRoots, 28);
    for (const id of [first, peer, cancelled, active]) runtime.probeDrop(id);
    assert.equal(inventory().liveRoots, 0);
    assert.equal(inventory().activeEntries, 0);
    assert.equal(runtime.probeSeal(), true);
    assert.throws(() => alias(active, 'same'));
    // Native node addresses identify the original allocation only inside one
    // heap. Keep the exact source node selector, URI, ranges and frame stack;
    // allocation addresses cannot be compared or reused across fresh heaps.
    return { ...report, diagnostics: report.diagnostics.map(d => {
        assert.match(d.node, /^cem:0x[0-9a-f]+:Some\(14\)$/);
        return { ...d, node: d.node.replace(/^cem:0x[0-9a-f]+:/, 'cem:<heap>:') };
    }) };
}
function provisioned(runtime, foreign, bytes, pins) {
    const inventory = () => JSON.parse(runtime.probeInventory());
    assert.throws(() => runtime.querySourceOpenV1(bytes, pins), 'unprovisioned metadata granted native authority');
    assert.equal(inventory().liveRoots, 0);
    assert.equal(runtime.probeForeignProvisionRefused(pins), true);
    assert.equal(inventory().liveRoots, 0);
    runtime.probeProvision(pins);
    assert.equal(inventory().liveRoots, 1);
    assert.throws(() => runtime.probeProvision(pins));
    const changed = JSON.parse(pins); changed.result_profile.hash = 'b'.repeat(64);
    assert.throws(() => runtime.querySourceOpenV1(bytes, JSON.stringify(changed)));
    assert.equal(inventory().liveRoots, 1);
    const owner = runtime.querySourceOpenV1(bytes, pins);
    assert.equal(typeof owner, 'object'); assert.ok(Object.isFrozen(owner));
    assert.equal(inventory().liveRoots, 8);
    assert.throws(() => runtime.querySourceCaptureV1({ ...owner }));
    assert.throws(() => runtime.querySourceReleaseV1(structuredClone(owner)));
    foreign.probeProvision(pins);
    const foreignOwner = foreign.querySourceOpenV1(bytes, pins);
    assert.throws(() => foreign.querySourceCaptureV1(owner));
    assert.throws(() => runtime.querySourceReleaseV1(foreignOwner));
    foreign.querySourceReleaseV1(foreignOwner); foreign.probeUnprovision();
    assert.deepEqual(runtime.querySourceCaptureV1(owner), bytes);
    assert.equal(JSON.parse(runtime.probeValidateOwned(owner, 'same')).accepted, true);
    runtime.probeOwnedRetire();
    assert.throws(() => runtime.querySourceCaptureV1(owner));
    assert.throws(() => runtime.probeValidateOwned(owner, 'same'));
    assert.equal(inventory().liveRoots, 8);
    runtime.probeUnprovision();
    assert.equal(inventory().liveRoots, 7);
    assert.throws(() => runtime.querySourceOpenV1(bytes, pins));
    runtime.probeClose();
    assert.equal(runtime.probeSeal(), false);
    runtime.querySourceReleaseV1(owner);
    assert.throws(() => runtime.querySourceReleaseV1(owner));
    assert.equal(inventory().liveRoots, 0);
    assert.equal(runtime.probeSeal(), true);
}
if (!isMainThread) {
    try {
        const runtime = await load(workerData.directory, 'worker');
        const provision = await load(workerData.directory, 'worker-provision');
        provisioned(provision, runtime, workerData.source, workerData.pins);
        parentPort.postMessage({ report: exercise(runtime, workerData.source, workerData.pins) });
    } catch (error) { parentPort.postMessage({ error: error?.stack ?? String(error) }); }
} else {
    const directory = process.argv[2];
    assert.ok(directory, 'pass the generated test-only WASM directory');
    const source = new Uint8Array(await readFile(resolve(directory, 'source.cemq')));
    const pins = await readFile(resolve(directory, 'pins.json'), 'utf8');
    const first = await load(directory, 'one'), second = await load(directory, 'two');
    const provision = await load(directory, 'provision');
    provisioned(provision, first, source, pins);
    assert.ok(first.probeMemory() instanceof WebAssembly.Memory);
    assert.notEqual(first.probeMemory(), second.probeMemory());
    const expected = exercise(first, source, pins);
    assert.equal(JSON.parse(second.probeInventory()).closed, false, 'one actual heap closed another');
    assert.deepEqual(exercise(second, source, pins), expected);
    const worker = new Worker(new URL(import.meta.url), { workerData: { directory, source, pins } });
    try {
        const report = await new Promise((resolve, reject) => {
            const timer = setTimeout(() => reject(Error('query WASM worker timed out')), 5000);
            worker.once('error', error => { clearTimeout(timer); reject(error); });
            worker.once('exit', code => { clearTimeout(timer); reject(Error(`worker exited before reply: ${code}`)); });
            worker.once('message', result => { clearTimeout(timer); result.error ? reject(Error(result.error)) : resolve(result.report); });
        });
        assert.deepEqual(report, expected, 'worker changed original source diagnostic provenance');
    } finally { await worker.terminate(); }
    console.log('Query source WASM: two independent heaps and a real worker passed native source relinking, provenance, stale contexts, cancellation, retained roots and scoped draining.');
}
