import assert from 'node:assert/strict';
import test from 'node:test';
import { CemNativeGenerationGroup, registerNativeGeneration, completeNativeRetirement } from '../runtime/native-generation.mjs';

function source(allocate) {
    const identity = {}, calls = [];
    const participant = registerNativeGeneration(identity, crypto.randomUUID(), allocate ?? (async admitted => {
        const runtime = Object.freeze({ use: () => { if (!admitted()) throw Error('closed'); return 'fresh'; } });
        calls.push(runtime); let alive = true;
        return { identity: {}, runtimeId: crypto.randomUUID(), runtime, current: () => alive,
            discard: () => { alive = false; calls.push('discard'); } };
    }));
    const bytes = new Uint8Array([1, 2, 3]);
    return { participant, calls, retire: () => completeNativeRetirement(participant, bytes), bytes };
}
test('complete original groups admit together; old receipt data cannot authorize or rewrite another handoff', async () => {
    const a = source(), b = source(), group = new CemNativeGenerationGroup([a.participant, b.participant]);
    const ar = a.retire(), br = b.retire(); ar.checkpoint.fill(99); a.bytes.fill(88);
    await assert.rejects(group.allocate([ar]), /coverage/);
    await assert.rejects(group.allocate([{ ...ar }, br]), /receipt/);
    assert.equal(a.calls.length, 0);
    const result = await group.allocate([br, ar]);
    assert.deepEqual([...result.generations[0].checkpoint], [1, 2, 3]);
    assert.equal(result.generations[0].runtime.use(), 'fresh');
    assert.equal(result.generations[1].runtime.use(), 'fresh');
    await assert.rejects(group.allocate([ar, br]), /state/);
    result.revoke();
    for (const item of result.generations) assert.throws(() => item.runtime.use(), /closed/);
});
test('a failed peer and late allocation never admit any member', async () => {
    let resolve, admitted;
    const a = source(async gate => { admitted = gate; return new Promise(done => { resolve = done; }); });
    const b = source(), group = new CemNativeGenerationGroup([a.participant, b.participant]);
    const pending = group.allocate([a.retire(), b.retire()], 5);
    await assert.rejects(pending, /timeout/);
    let discarded = 0;
    resolve({ identity: {}, runtimeId: 'late', runtime: {}, current: () => true, discard: () => discarded++ });
    await new Promise(done => setImmediate(done));
    assert.equal(discarded, 1); assert.equal(admitted(), false); assert.equal(b.calls.length, 0);
});
test('stale candidates and reused physical identities cannot partially admit peers', async () => {
    let gate, discarded = 0; const identity = {};
    const a = source(async admitted => { gate = admitted; return { identity, runtimeId: 'a', runtime: {}, current: () => true, discard: () => discarded++ }; });
    const b = source(async () => ({ identity, runtimeId: 'b', runtime: {}, current: () => true, discard: () => discarded++ }));
    const group = new CemNativeGenerationGroup([a.participant, b.participant]);
    await assert.rejects(group.allocate([a.retire(), b.retire()]), /identity/);
    assert.equal(gate(), false); assert.equal(discarded, 2);
});

test('original participant membership is exclusive and array accessors do not execute', async () => {
    const a = source(), b = source(); let read = 0;
    const accessor = [a.participant]; Object.defineProperty(accessor, '0', { get() { read++; return a.participant; } });
    assert.throws(() => new CemNativeGenerationGroup(accessor), /coverage/); assert.equal(read, 0);
    assert.throws(() => new CemNativeGenerationGroup([{ ...a.participant }]), /coverage/);
    assert.throws(() => new CemNativeGenerationGroup([a.participant, a.participant]), /coverage/);
    const group = new CemNativeGenerationGroup([a.participant, b.participant]);
    assert.throws(() => new CemNativeGenerationGroup([a.participant]), /coverage/);
    const ar = a.retire(), br = b.retire();
    const copies = [ar, br]; copies[Symbol.iterator] = () => { throw Error('iterator called'); };
    const result = await group.allocate(copies); result.revoke();
});
test('inspection reentrancy cannot supersede an in-flight group', async () => {
    const a = source(), group = new CemNativeGenerationGroup([a.participant]), receipt = a.retire();
    let nested;
    const proxy = new Proxy([receipt], { getOwnPropertyDescriptor(target, property) {
        if (property === '0') nested = group.allocate([receipt]);
        return Reflect.getOwnPropertyDescriptor(target, property);
    } });
    await assert.rejects(group.allocate(proxy), /state/);
    const result = await nested; assert.equal(a.calls.length, 1); result.revoke();
});
test('a candidate from another registered heap and missing currentness refuse admission', async () => {
    const identity = {}, other = registerNativeGeneration(identity, 'foreign', async () => {});
    assert.ok(other);
    for (const [physical, current] of [[identity, () => true], [{}, () => false], [{}, () => { throw Error('probe failure'); }]]) {
        let gate, discarded = 0;
        const a = source(async admitted => { gate = admitted; return { identity: physical,
            runtimeId: crypto.randomUUID(), runtime: {}, current, discard: () => discarded++ }; });
        const group = new CemNativeGenerationGroup([a.participant]);
        await assert.rejects(group.allocate([a.retire()]));
        assert.equal(gate(), false); assert.equal(discarded, 1);
    }
});
test('fresh allocation retains headroom deadline through final synchronous probes', async () => {
    let discarded = 0, gate;
    const a = source(async admitted => { gate = admitted; return { identity: {}, runtimeId: 'overdue', runtime: {},
        current: () => { const until = performance.now() + 8; while (performance.now() < until) { /* synchronous native probe */ } return true; },
        discard: () => discarded++ }; });
    const group = new CemNativeGenerationGroup([a.participant]);
    await assert.rejects(group.allocate([a.retire()], 5), /timeout/);
    assert.equal(gate(), false); assert.equal(discarded, 1);
});

test('retained revoked admission handles release original and candidate memory identities', { skip: typeof global.gc !== 'function' }, async () => {
    let weak;
    let original = new WebAssembly.Memory({ initial: 1 });
    const originalWeak = new WeakRef(original);
    const participant = registerNativeGeneration(original, crypto.randomUUID(), async admitted => {
        let memory = new WebAssembly.Memory({ initial: 1 });
        weak = new WeakRef(memory);
        return { identity: memory, runtimeId: crypto.randomUUID(), runtime: { admitted },
            current: () => memory !== undefined, discard: () => { memory = undefined; } };
    });
    const group = new CemNativeGenerationGroup([participant]);
    const receipt = completeNativeRetirement(participant, new Uint8Array([1])); original = undefined;
    const admission = await group.allocate([receipt]);
    admission.revoke();
    // Forced test-only GC verifies reachability while all host handles stay
    // alive. Production neither forces GC nor promises an RSS reclamation time.
    for (let attempt = 0; attempt < 20; attempt++) {
        await new Promise(done => setImmediate(done)); global.gc();
        if (!weak.deref() && !originalWeak.deref()) break;
    }
    assert.equal(weak.deref(), undefined);
    assert.equal(originalWeak.deref(), undefined);
    assert.equal(admission.generations[0].runtime.admitted(), false);
    assert.ok(group);
});
