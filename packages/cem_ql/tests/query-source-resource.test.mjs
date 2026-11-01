import assert from 'node:assert/strict';
import test from 'node:test';
import { MessageChannel } from 'node:worker_threads';
import { readFile } from 'node:fs/promises';
import { createQuerySourceEndpoint, createQuerySourceWorkerEndpoint, installQuerySourceWorkerTransport, QuerySourceResource } from '../runtime/query-source-resource.mjs';
import { CemNativeGenerationGroup, registerNativeGeneration, registerNativeGenerationResource, releaseNativeGenerationResourcesAsync, completeNativeRetirement } from '../runtime/native-generation.mjs';

// Protocol unit doubles only. Native execution/census/retirement acceptance is
// in query-resource-wasm.mjs against the actual full WASM and a worker.
const bytes = new TextEncoder().encode('CEMQH1\nunit-source');
function fixture(native, identity = new WebAssembly.Memory({ initial: 1 })) {
    const participant = registerNativeGeneration(identity, crypto.randomUUID(), async () => { throw Error('unit does not allocate'); });
    const values = new Map(); let sequence = 0;
    const endpoint = createQuerySourceEndpoint({ participant, identity, native: native ?? {
        open: () => { const id = ++sequence; values.set(id, bytes.slice()); return id; },
        capture: id => values.get(id).slice(), release: id => { assert.ok(values.delete(id)); },
    }, current: () => true });
    return { endpoint, participant, values, identity };
}
test('query endpoints require original physical participants and original private owner handles', async () => {
    const a = fixture(), b = fixture();
    assert.throws(() => createQuerySourceEndpoint({ participant: a.participant, identity: b.identity, native: {}, current: () => true }), /target/);
    const owner = await a.endpoint.open(bytes, '{}');
    await assert.rejects(a.endpoint.capture({ ...owner }), /owner/);
    await assert.rejects(b.endpoint.release(owner), /owner/);
    const captured = await a.endpoint.capture(owner); captured.fill(0);
    assert.deepEqual(await a.endpoint.capture(owner), bytes);
    await a.endpoint.release(owner);
    await assert.rejects(a.endpoint.capture(owner), /owner/);
    assert.equal(a.values.size, 0);
    a.endpoint.dispose(); b.endpoint.dispose();
});
test('privacy and source changes fence capture and resource release before native cleanup', async () => {
    const a = fixture(); let privateInput = false;
    const resource = await QuerySourceResource.open({ endpoint: a.endpoint, bytes, pins: '{}', current: () => !privateInput, target: () => { throw Error('not allocating'); } });
    privateInput = true;
    await assert.rejects(resource.prepareRecovery(), /policy/);
    assert.throws(() => new CemNativeGenerationGroup([a.participant]), /query|resource/);
    assert.equal(a.values.size, 1);
    privateInput = false; await resource.prepareRecovery();
    new CemNativeGenerationGroup([a.participant]); privateInput = true;
    await assert.rejects(releaseNativeGenerationResourcesAsync(a.participant, () => true), /stale|resource/);
    assert.equal(a.values.size, 1);
    await resource.dispose(); assert.equal(a.values.size, 0); a.endpoint.dispose();
});
test('original owner cleanup remains possible behind closed ordinary admission without reopening it', async () => {
    let admitted = true, released = 0;
    const identity = new WebAssembly.Memory({ initial: 1 });
    const participant = registerNativeGeneration(identity, 'closed-cleanup', async () => { throw Error('unit'); });
    const endpoint = createQuerySourceEndpoint({ identity, participant, current: () => admitted, cleanupCurrent: () => true,
        native: { open: () => 1, capture: () => bytes, release: () => { released++; } } });
    const owner = await endpoint.open(bytes, '{}'); admitted = false;
    await assert.rejects(endpoint.capture(owner), /stale/);
    await endpoint.release(owner); assert.equal(released, 1); assert.equal(endpoint.current(), false); endpoint.dispose();
});
test('cancelled imports refuse late owner publication and clean original native results', async () => {
    let finish, released = 0;
    const a = fixture({ open: () => new Promise(resolve => { finish = resolve; }), capture: () => bytes, release: () => { released++; } });
    const abort = new AbortController();
    const pending = a.endpoint.open(bytes, '{}', abort.signal);
    await new Promise(resolve => setImmediate(resolve)); abort.abort();
    await assert.rejects(pending, /cancel|aborted/);
    finish(7); await new Promise(resolve => setImmediate(resolve));
    assert.equal(released, 1); assert.equal(a.endpoint.current(), false); a.endpoint.dispose();
});
test('changed native source bytes invalidate prepared handoff without releasing the original owner', async () => {
    let changed = false, released = 0;
    const a = fixture({ open: () => 1, capture: () => changed ? bytes.slice(0, -1) : bytes, release: () => { released++; } });
    const resource = await QuerySourceResource.open({ endpoint: a.endpoint, bytes, pins: '{}', current: () => true, target: () => a.endpoint });
    await resource.prepareRecovery(); changed = true;
    await assert.rejects(resource.prepareRecovery(), /stale/);
    assert.throws(() => new CemNativeGenerationGroup([a.participant]), /stale/);
    assert.equal(released, 0); await resource.dispose(); assert.equal(released, 1); a.endpoint.dispose();
});
test('private worker byte transport refuses foreign handles and late cleanup acknowledgments', async () => {
    const channel = new MessageChannel(); let finish, delay = true;
    const values = new Map(); let id = 0;
    const server = installQuerySourceWorkerTransport(channel.port2, {
        open: () => { values.set(++id, bytes); return id; }, capture: handle => values.get(handle),
        release: handle => delay ? new Promise(resolve => { finish = () => { values.delete(handle); resolve(); }; }) : values.delete(handle),
    });
    const identity = {}, participant = registerNativeGeneration(identity, 'worker-unit', async () => { throw Error('unit'); });
    const endpoint = createQuerySourceWorkerEndpoint({ identity, participant, port: channel.port1, current: () => true, timeoutMs: 200 });
    try {
        const owner = await endpoint.open(bytes, '{}');
        assert.deepEqual(await endpoint.capture(owner), bytes);
        await assert.rejects(endpoint.release({ ...owner }), /owner/);
        await assert.rejects(endpoint.release(owner), /timeout/);
        assert.equal(endpoint.current(), false); finish();
        await new Promise(resolve => setTimeout(resolve, 10));
        assert.equal(values.size, 0); assert.equal(endpoint.current(), false);
    } finally { delay = false; finish?.(); endpoint.dispose(); await server.dispose(); }
});
test('retained query/source bytes join the existing aggregate checkpoint budget', async () => {
    const a = fixture();
    registerNativeGenerationResource(a.participant, () => ({ retainedBytes: 4 * 1024 * 1024,
        current: () => true, release() {}, prepare: async () => { throw Error('not allocating'); } }), () => {});
    const group = new CemNativeGenerationGroup([a.participant]);
    await releaseNativeGenerationResourcesAsync(a.participant, () => true);
    const retirement = completeNativeRetirement(a.participant, new Uint8Array([1]));
    await assert.rejects(group.allocate([retirement]), /limit/);
    a.endpoint.dispose();
});
test('invalid retained-byte counts cannot authorize resource capture or release', () => {
    for (const retainedBytes of [-1, NaN, 1.5, 4 * 1024 * 1024 + 1]) {
        const a = fixture();
        registerNativeGenerationResource(a.participant, () => ({ retainedBytes, current: () => true, release() {}, prepare: async () => {} }), () => {});
        assert.throws(() => new CemNativeGenerationGroup([a.participant]), /limit|resource/); a.endpoint.dispose();
    }
});
test('an older vendored group without retained-byte accounting cannot capture a query resource', async () => {
    const text = await readFile(new URL('../runtime/native-generation.mjs', import.meta.url), 'utf8');
    // Model the previous constructor by removing only its new private group
    // attestation. This is compatibility coverage, not native ownership proof.
    const older = await import(`data:text/javascript;base64,${Buffer.from(text.replace('record.budgetGroup = this;', '')).toString('base64')}`);
    const a = fixture();
    const resource = await QuerySourceResource.open({ endpoint: a.endpoint, bytes, pins: '{}', current: () => true, target: () => a.endpoint });
    await resource.prepareRecovery();
    assert.throws(() => new older.CemNativeGenerationGroup([a.participant]), /resource-budget/);
    assert.equal(a.values.size, 1);
    new CemNativeGenerationGroup([a.participant]); // A failed read-only capture leaves the original participant reusable.
    await resource.dispose(); a.endpoint.dispose();
});
