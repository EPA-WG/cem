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
    const identity = {}, other = registerNativeGeneration(identity, 'foreign', async () => { /* This fixture never allocates the foreign participant. */ });
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
    assert.equal(original, undefined);
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

test('resume claims require an original live admission and retain private checkpoint bytes once', async () => {
    const { claimNativeGenerationAdmission } = await import('../runtime/native-generation.mjs');
    const a = source(), group = new CemNativeGenerationGroup([a.participant]);
    const admission = await group.allocate([a.retire()]);
    assert.throws(() => claimNativeGenerationAdmission({ ...admission }), /admission/);
    admission.generations[0].checkpoint.fill(7);
    const claimed = claimNativeGenerationAdmission(admission);
    assert.deepEqual([...claimed.generations[0].checkpoint], [1, 2, 3]);
    assert.equal(claimed.current(), true);
    assert.throws(() => claimNativeGenerationAdmission(admission), /admission/);
    claimed.revoke(); assert.equal(claimed.current(), false);
    const b = source(), second = await new CemNativeGenerationGroup([b.participant]).allocate([b.retire()]);
    second.revoke(); assert.throws(() => claimNativeGenerationAdmission(second), /admission/);
});

test('currentness probes cannot reenter and claim the same admission twice', async () => {
    const { claimNativeGenerationAdmission } = await import('../runtime/native-generation.mjs');
    let probe = false, nestedRefused = false;
    const a = source(async () => ({ identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, discard: () => undefined,
        current: () => {
            if (probe) {
                probe = false;
                try { claimNativeGenerationAdmission(admission); } catch { nestedRefused = true; }
            }
            return true;
        } }));
    const admission = await new CemNativeGenerationGroup([a.participant]).allocate([a.retire()]);
    probe = true; claimNativeGenerationAdmission(admission);
    assert.equal(nestedRefused, true); admission.revoke();
});

test('owner cleanup runs behind the closed gate before allocation abort and physical disposal', async () => {
    const { claimNativeGenerationAdmission } = await import('../runtime/native-generation.mjs');
    const events = []; let gate;
    const a = source(async (admitted, signal) => {
        gate = admitted; signal.addEventListener('abort', () => events.push('allocation abort'), { once: true });
        return { identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, current: () => true,
            discard: () => events.push('physical disposal') };
    });
    const admission = await new CemNativeGenerationGroup([a.participant]).allocate([a.retire()]);
    claimNativeGenerationAdmission(admission).onRevoke(() => { assert.equal(gate(), false); events.push('owner cleanup'); });
    admission.revoke();
    assert.deepEqual(events, ['owner cleanup', 'allocation abort', 'physical disposal']);
});


test('shared physical participants require one original complete coverage attestation', async () => {
    const { CemNativeRecoveryScope, registerNativeGenerationCoverage } = await import('../runtime/native-generation.mjs');
    for (const invalid of ['none', 'copy', 'stale', 'valid']) {
        const a = source(), scope = new CemNativeRecoveryScope(); let current = true;
        const coverage = registerNativeGenerationCoverage(a.participant, () => current);
        scope.enroll({ id: 'parent', kind: 'rendering', prepare: async () => ({ participants: [a.participant], coverage, current: () => true }) });
        scope.enroll({ id: 'child', kind: 'live-preview', prepare: async () => ({ participants: [a.participant],
            coverage: invalid === 'none' ? undefined : invalid === 'copy' ? { ...coverage } : coverage, current: () => true }) });
        if (invalid === 'stale') current = false;
        if (invalid === 'valid') {
            const prepared = await scope.prepare(), admission = await prepared.createGenerationGroup().allocate([a.retire()]);
            assert.equal(admission.generations.length, 1); admission.revoke();
        } else await assert.rejects(scope.prepare(), /coverage|overlap/);
        scope.dispose();
    }
});

test('mixed semantic tickets cover every remaining physical generation and consume original authority once', async () => {
    const { issueNativeGenerationBindingTicket, claimNativeGenerationBindings, claimNativeGenerationAdmission } = await import('../runtime/native-generation.mjs');
    const a = source(), b = source(), group = new CemNativeGenerationGroup([a.participant, b.participant]);
    const admission = await group.allocate([a.retire(), b.retire()]), authority = claimNativeGenerationAdmission(admission);
    const rendering = authority.generations[0].previousRuntimeId, runtimeId = authority.generations[1].previousRuntimeId;
    let valid = true;
    const ticket = issueNativeGenerationBindingTicket(b.participant, () => valid, async () => ({}));
    assert.throws(() => claimNativeGenerationBindings({ ...authority }, [ticket], [rendering]), /binding/);
    assert.throws(() => claimNativeGenerationBindings(authority, [{ ...ticket }], [rendering]), /binding/);
    assert.throws(() => claimNativeGenerationBindings(authority, [], [rendering]), /coverage/);
    valid = false; assert.throws(() => claimNativeGenerationBindings(authority, [ticket], [rendering]), /stale/);
    valid = true; const bindings = claimNativeGenerationBindings(authority, [ticket], [rendering]);
    assert.equal(bindings[0].generation.previousRuntimeId, runtimeId);
    assert.throws(() => claimNativeGenerationBindings(authority, [ticket], [rendering]), /binding/);
    admission.revoke();
});


test('matching runtime labels never substitute a foreign physical binding participant', async () => {
    const { issueNativeGenerationBindingTicket, claimNativeGenerationBindings, claimNativeGenerationAdmission } = await import('../runtime/native-generation.mjs');
    const runtimeId = crypto.randomUUID();
    const create = () => registerNativeGeneration({}, runtimeId, async () => ({ identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, current: () => true, discard() { /* No retained fixture owner. */ } }));
    const source = create(), foreign = create();
    assert.throws(() => issueNativeGenerationBindingTicket({ ...source }, () => true, async () => ({})), /binding/);
    assert.throws(() => issueNativeGenerationBindingTicket(runtimeId, () => true, async () => ({})), /binding/);
    const ticket = issueNativeGenerationBindingTicket(source, () => true, async () => ({}));
    const admission = await new CemNativeGenerationGroup([foreign]).allocate([completeNativeRetirement(foreign, new Uint8Array([1]))]);
    const authority = claimNativeGenerationAdmission(admission);
    assert.throws(() => claimNativeGenerationBindings(authority, [ticket], []), /binding/);
    admission.revoke();
});

test('workspace receipt transport keeps the complete group bounded to 4 MiB', async () => {
    const a = source(), b = source();
    const group = new CemNativeGenerationGroup([a.participant, b.participant]);
    const first = completeNativeRetirement(a.participant, new Uint8Array(3 * 1024 * 1024));
    const second = completeNativeRetirement(b.participant, new Uint8Array(1024 * 1024 + 1));
    await assert.rejects(group.allocate([first, second]), /limit/);
    assert.equal(a.calls.length, 0); assert.equal(b.calls.length, 0);
    const c = source(), single = new CemNativeGenerationGroup([c.participant]);
    assert.throws(() => completeNativeRetirement(c.participant, new Uint8Array(4 * 1024 * 1024 + 1)), /receipt/);
    const receipt = completeNativeRetirement(c.participant, new Uint8Array(4 * 1024 * 1024));
    const admitted = await single.allocate([receipt]); assert.equal(admitted.generations[0].checkpoint.byteLength, 4 * 1024 * 1024); admitted.revoke();
});

test('original generation resources require checked release and join complete binding claims', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources, issueNativeGenerationBindingTicket,
        claimNativeGenerationAdmission, claimNativeGenerationBindings } = await import('../runtime/native-generation.mjs');
    let released = false, current = true, prepared = 0;
    const a = source();
    const enrollment = registerNativeGenerationResource(a.participant, () => ({ current: () => current,
        release: () => { released = true; }, prepare: async () => { prepared++; return { owner: {}, current: () => true, discard() { /* No retained fixture owner. */ }, bind() { /* No separate fixture route. */ } }; } }), () => { /* No native owner in this control-only fixture. */ });
    const group = new CemNativeGenerationGroup([a.participant]);
    assert.throws(() => a.retire(), /resource/);
    assert.throws(() => releaseNativeGenerationResources(a.participant, () => false), /resource/);
    assert.equal(released, false);
    releaseNativeGenerationResources(a.participant, () => true);
    assert.equal(released, true);
    const ticket = issueNativeGenerationBindingTicket(a.participant, () => true, async () => ({ owner: {}, current: () => true, discard() { /* No retained fixture owner. */ }, bind() { /* No separate fixture route. */ } }));
    const admission = await group.allocate([a.retire()]);
    const authority = claimNativeGenerationAdmission(admission);
    const bindings = claimNativeGenerationBindings(authority, [ticket], []);
    assert.equal(bindings.length, 2);
    for (const binding of bindings) await binding.prepare({ generation: binding.generation, active: () => false, signal: new AbortController().signal, revoke: authority.revoke });
    assert.equal(prepared, 1);
    assert.throws(() => claimNativeGenerationBindings(authority, [ticket], []), /binding/);
    current = false; enrollment.remove(); authority.revoke();
});

test('resource membership and source changes fence release and allocation', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources } = await import('../runtime/native-generation.mjs');
    let current = true, released = 0;
    const a = source();
    const registration = registerNativeGenerationResource(a.participant, () => ({ current: () => current,
        release: () => { released++; }, prepare: async () => ({}) }), () => { /* No native owner in this control-only fixture. */ });
    new CemNativeGenerationGroup([a.participant]);
    current = false;
    assert.throws(() => releaseNativeGenerationResources(a.participant, () => true), /resource/);
    assert.equal(released, 0);
    registration.remove();
    assert.throws(() => a.retire(), /resource/);
});

test('failed resource staging revokes the complete fresh group without binding a host route', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } =
        await import('../runtime/native-generation.mjs');
    let discarded = 0, bound = 0;
    const a = source();
    registerNativeGenerationResource(a.participant, () => ({ current: () => true, release() { /* No native owner in this fixture. */ },
        prepare: async context => {
            assert.equal(context.active(), false);
            return { owner: {}, current: () => false, discard: () => { discarded++; }, bind: () => { bound++; } };
        } }), () => { /* Explicit source disposal is unused. */ });
    const ticket = issueNativeGenerationBindingTicket(a.participant, () => true,
        async () => ({ owner: {}, current: () => true, discard() { /* No retained fixture owner. */ }, bind: () => { bound++; } }));
    const group = new CemNativeGenerationGroup([a.participant]);
    releaseNativeGenerationResources(a.participant, () => true);
    const admission = await group.allocate([a.retire()]);
    await assert.rejects(resumeNativeGenerationBindings(admission, [ticket]), /binding-stale/);
    assert.equal(bound, 0); assert.equal(discarded, 1); assert.ok(a.calls.includes('discard'));
});

test('late resource preparation is cleaned once after the shared deadline', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } =
        await import('../runtime/native-generation.mjs');
    let finish, discarded = 0, bound = 0;
    const a = source();
    registerNativeGenerationResource(a.participant, () => ({ current: () => true, release() { /* No native owner. */ },
        prepare: async () => new Promise(resolve => { finish = resolve; }) }), () => { /* No source heap in this fixture. */ });
    const group = new CemNativeGenerationGroup([a.participant]);
    const ticket = issueNativeGenerationBindingTicket(a.participant, () => true, async () => { throw Error('must not prepare host'); });
    releaseNativeGenerationResources(a.participant, () => true);
    const admission = await group.allocate([a.retire()]);
    await assert.rejects(resumeNativeGenerationBindings(admission, [ticket], 5), /timeout/);
    finish({ owner: {}, current: () => true, discard: () => { discarded++; }, bind: () => { bound++; } });
    await new Promise(done => setImmediate(done));
    assert.equal(discarded, 1); assert.equal(bound, 0); assert.ok(a.calls.includes('discard'));
});

test('resource targets bind original participants to exact physical identities', async () => {
    const { assertNativeGenerationResourceTarget } = await import('../runtime/native-generation.mjs');
    const memory = {}, foreign = {}, participant = registerNativeGeneration(memory, 'target', async () => { throw Error('unused'); });
    assertNativeGenerationResourceTarget(participant, 'target', memory);
    assert.throws(() => assertNativeGenerationResourceTarget({ ...participant }, 'target', memory), /resource-target/);
    assert.throws(() => assertNativeGenerationResourceTarget(participant, 'target', foreign), /resource-target/);
    assert.throws(() => assertNativeGenerationResourceTarget(participant, 'other', memory), /resource-target/);
});

test('a busy resource capture leaves participants available for a new complete preparation', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources } = await import('../runtime/native-generation.mjs');
    const a = source(); let busy = true;
    registerNativeGenerationResource(a.participant, () => {
        if (busy) throw Error('busy');
        return { current: () => true, release() { /* No native owner. */ }, prepare: async () => { throw Error('unused'); } };
    }, () => { /* No native source heap. */ });
    assert.throws(() => new CemNativeGenerationGroup([a.participant]), /busy/);
    busy = false;
    const group = new CemNativeGenerationGroup([a.participant]);
    releaseNativeGenerationResources(a.participant, () => true);
    const admission = await group.allocate([a.retire()]); admission.revoke();
});


test('async resource release rechecks original witnesses and refuses late completion', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResourcesAsync } = await import('../runtime/native-generation.mjs');
    for (const scenario of ['success', 'stale', 'late', 'reject']) {
        const a = source(); let current = true, finish;
        registerNativeGenerationResource(a.participant, () => ({ current: () => true,
            release: () => new Promise((resolve, reject) => { finish = scenario === 'reject' ? () => reject(Error('cleanup failed')) : resolve; }),
            prepare: async () => { throw Error('unused'); } }), () => { /* No fixture heap. */ });
        new CemNativeGenerationGroup([a.participant]);
        const releasing = releaseNativeGenerationResourcesAsync(a.participant, () => current, scenario === 'late' ? 5 : 5000);
        assert.throws(() => a.retire(), /resource/);
        if (scenario === 'stale') current = false;
        if (scenario === 'late') {
            await assert.rejects(releasing, /timeout/); finish();
            await new Promise(done => setImmediate(done));
        } else {
            finish();
            if (scenario === 'success') { await releasing; a.retire(); continue; }
            await assert.rejects(releasing, scenario === 'reject' ? /cleanup failed/ : /stale/);
        }
        assert.throws(() => a.retire(), /resource/);
        await assert.rejects(releaseNativeGenerationResourcesAsync(a.participant, () => true), /resource/);
    }
});

test('resource cohorts require every original peer before any release', async () => {
    const { registerNativeGenerationResource, releaseNativeGenerationResources } = await import('../runtime/native-generation.mjs');
    const a = source(), b = source(); let released = 0;
    const attach = (participant, peers) => registerNativeGenerationResource(participant, () => ({ peers, current: () => true,
        release: () => { released++; }, prepare: async () => { throw Error('unused'); } }), () => undefined);
    const membership = attach(a.participant, [a.participant, b.participant]);
    assert.throws(() => new CemNativeGenerationGroup([a.participant]), /coverage/);
    assert.throws(() => releaseNativeGenerationResources(a.participant, () => true), /resource/);
    assert.equal(released, 0);
    membership.remove();
    const copied = attach(a.participant, [a.participant, { ...b.participant }]);
    assert.throws(() => new CemNativeGenerationGroup([a.participant, b.participant]), /coverage/);
    copied.remove();
    attach(a.participant, [a.participant, b.participant]);
    new CemNativeGenerationGroup([a.participant, b.participant]);
    releaseNativeGenerationResources(a.participant, () => true);
    assert.equal(released, 1);
});
