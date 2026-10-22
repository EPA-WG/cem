import * as native from '@epa-wg/cem-ml/wasm';
import assert from 'node:assert/strict';
import test from 'node:test';
import { CommandNativeClientCheckpoint } from '../dist/native-admission.js';
import { parseCommandNativeRetirement } from '../dist/native-embedding.js';
import { CemNativeGenerationGroup, createNodeCommandServiceClient } from '../dist/node.js';

const inventory = { schemaVersion: 1, profile: 'cem-ml-cem-ql-command-v1', revision: 3, complete: true,
    blockingResources: 0, registries: [{ name: 'native-entries', live: 0, discardable: 0 }], replacementAllowed: false };
const response = { revision: 1, scope: 'managed-cli-only', replacementAllowed: false, nativeInventory: inventory,
    nativeClosure: { schemaVersion: 1, closed: true, inventoryRevision: 3, replacementAllowed: false } };
const confirmation = { schemaVersion: 1, frozen: true, inventoryRevision: 3, frozenRevision: 4, replacementAllowed: false };
const handoff = () => ({ current: () => true, retain: () => new TextEncoder().encode('native-sealed checkpoint with terminal journal') });
async function state() {
    const state = new CommandNativeClientCheckpoint(() => true), owner = state.claimEmbedding();
    const proof = await state.capture(async () => response, 1000, [owner], 'close');
    return { state, owner, proof };
}
const host = { currentRevision: ({ project }) => ({ project, resourceVersions: {} }),
    readResource: () => { throw Error('unexpected read'); }, prepareWrite: () => { throw Error('unexpected write'); },
    commitWrite: () => { throw Error('unexpected commit'); }, rollbackWrite: () => undefined };

test('retirement confirmations are bounded evidence, never release or replacement permits', () => {
    parseCommandNativeRetirement(confirmation, 3);
    for (const value of [{ ...confirmation, extra: true }, { ...confirmation, inventoryRevision: 2 },
        { ...confirmation, frozenRevision: 3 }, { ...confirmation, frozen: false }, { ...confirmation, replacementAllowed: true }, 'x'.repeat(513)]) {
        assert.throws(() => parseCommandNativeRetirement(value, 3));
    }
});

test('original ownership and checkpoint currency span native preparation and synchronous termination', async () => {
    const { state: s, owner, proof } = await state();
    let end, aborted = 0, terminated = 0;
    const source = handoff(); let current = true; source.current = () => current;
    const pending = s.retire(proof, [owner], source, () => new Promise(resolve => { end = resolve; }),
        async () => { aborted++; }, async () => { terminated++; });
    current = false; end(confirmation);
    await assert.rejects(pending, { code: 'cem.command.retirement.stale' });
    assert.equal(aborted, 1); assert.equal(terminated, 0); assert.equal(s.current(proof), false);
    const f = await state();
    await assert.rejects(f.state.retire({ ...f.proof }, [f.owner], handoff(), async () => confirmation, async () => undefined, async () => undefined));
    let finish;
    const changing = f.state.retire(f.proof, [f.owner], handoff(), () => new Promise(resolve => { finish = resolve; }), async () => { aborted++; }, async () => { terminated++; });
    const peer = f.state.claimEmbedding(); f.state.releaseEmbedding(peer); finish(confirmation);
    await assert.rejects(changing, { code: 'cem.command.retirement.stale' });
    assert.equal(terminated, 0);
});

test('handoff reentrancy, copied owners and late timeout replies cannot retire a generation', async () => {
    const f = await state(); let terminated = 0;
    await assert.rejects(f.state.retire(f.proof, [{ ...f.owner }], handoff(), async () => confirmation, async () => undefined, async () => { terminated++; }));
    let checks = 0; const source = handoff(); source.current = () => {
        if (++checks === 3) { const peer = f.state.claimEmbedding(); f.state.releaseEmbedding(peer); } return true;
    };
    await assert.rejects(f.state.retire(f.proof, [f.owner], source, async () => confirmation, async () => undefined, async () => { terminated++; }));
    const g = await state(); let late, aborted = 0;
    const pending = g.state.retire(g.proof, [g.owner], handoff(), () => new Promise(resolve => { late = resolve; }), async () => { aborted++; }, async () => { terminated++; }, 5);
    await assert.rejects(pending, { code: 'cem.command.retirement.timeout' }); late(confirmation);
    assert.equal(aborted, 1); assert.equal(terminated, 0); assert.equal(g.state.current(g.proof), false);
});

test('successful handoff retains an independent byte copy before termination and revokes old ownership', async () => {
    const { state: s, owner, proof } = await state(); let terminated = false;
    const bytes = new Uint8Array([1, 2, 3]);
    const result = await s.retire(proof, [owner], { current: () => true, retain: () => bytes }, async () => {
        bytes.fill(9); return confirmation;
    }, async () => { throw Error('unexpected abort'); }, async () => { terminated = true; });
    assert.equal(terminated, true); assert.deepEqual([...result.checkpoint], [1, 2, 3]);
    assert.equal(result.replacementAllowed, false); assert.equal(s.current(proof), false);
    assert.throws(() => s.claimEmbedding());
});

test('real Node worker retires only after a retained handoff and leaves independent heaps running', async () => {
    const client = await createNodeCommandServiceClient({ host }), other = await createNodeCommandServiceClient({ host });
    try {
        const owner = client.claimNativeEmbedding();
        const proof = await client.closeNativeAdmission([owner]);
        await assert.rejects(client.retireNativeGeneration(proof, [owner], { current: () => false, retain: () => { throw Error('stale'); } }));
        const result = await client.retireNativeGeneration(proof, [owner], handoff());
        assert.match(new TextDecoder().decode(result.checkpoint), /terminal journal/);
        assert.equal(result.replacementAllowed, false);
        assert.throws(() => client.claimNativeEmbedding(), { code: 'cem.node_command.client_closed' });
        const lease = other.claimNativeEmbedding(); const otherProof = await other.closeNativeAdmission([lease]);
        assert.equal(otherProof.nativeClosure.closed, true);
    } finally { await client.close(); await other.close(); }
});

test('real worker abort restores cleanup after a publication changes during native preparation', async () => {
    const client = await createNodeCommandServiceClient({ host });
    try {
        const owner = client.claimNativeEmbedding(); const proof = await client.closeNativeAdmission([owner]);
        let checks = 0; const source = handoff(); source.current = () => ++checks < 3;
        await assert.rejects(client.retireNativeGeneration(proof, [owner], source), { code: 'cem.command.retirement.stale' });
        // This native cleanup increments the revision, proving the barrier released.
        await assert.rejects(client.cancel('nothing-active'), { code: 'cem.command_service.request_inactive' });
        const fresh = await client.closeNativeAdmission([owner]);
        await client.retireNativeGeneration(fresh, [owner], handoff());
    } finally { await client.close(); }
});


test('real raw WASM cleanup cannot cross a private retirement permit or release it with scalar evidence', () => {
    native.closeCommandNativeAdmission();
    const old = JSON.parse(native.commandNativeResourceInventory());
    native.resetRetainedRenderCache();
    assert.throws(() => native.prepareCommandNativeRetirement(old.revision));
    const current = JSON.parse(native.commandNativeResourceInventory());
    const permit = native.prepareCommandNativeRetirement(current.revision);
    const record = JSON.parse(permit.confirmation());
    parseCommandNativeRetirement(record, current.revision);
    assert.throws(() => native.resetRetainedRenderCache(), error => error === 'cem.runtime.retirement.pending');
    const frozen = JSON.parse(native.commandNativeResourceInventory());
    assert.equal(frozen.revision, record.frozenRevision);
    assert.equal(frozen.registries.find(entry => entry.name === 'native-entries').live, 1);
    assert.equal(frozen.blockingResources, 1);
    assert.throws(() => native.prepareCommandNativeRetirement(record.frozenRevision));
    const counterfeit = new native.CommandNativeRetirement(record);
    assert.throws(() => counterfeit.free());
    assert.throws(() => native.resetRetainedRenderCache(), error => error === 'cem.runtime.retirement.pending');
    assert.equal(typeof record.free, 'undefined');
    permit.free(); native.resetRetainedRenderCache();
    assert.ok(JSON.parse(native.commandNativeResourceInventory()).revision > record.frozenRevision);
    assert.equal(JSON.parse(native.compileTemplate('{p | denied}', '{}')).error.code, 'cem.runtime.admission.closed');
});


test('handoff admission bounds bytes and copies typed storage without caller slice hooks', async () => {
    const { state: s, owner, proof } = await state();
    for (const bytes of [new Uint8Array(), new Uint8Array(1024 * 1024 + 1), new Uint8Array(new SharedArrayBuffer(1))]) {
        await assert.rejects(s.retire(proof, [owner], { current: () => true, retain: () => bytes },
            async () => { throw Error('unexpected preparation'); }, async () => undefined, async () => undefined), { code: 'cem.command.retirement.handoff' });
    }
    const bytes = new Uint8Array([1, 2]); bytes.slice = () => { throw Error('caller slice hook'); };
    const result = await s.retire(proof, [owner], { current: () => true, retain: () => bytes }, async () => confirmation,
        async () => undefined, async () => undefined);
    bytes.fill(8); assert.deepEqual([...result.checkpoint], [1, 2]);
});

test('two original command workers allocate and admit a complete fresh set without retargeting old clients', async () => {
    const first = await createNodeCommandServiceClient({ host }), second = await createNodeCommandServiceClient({ host });
    let admission;
    try {
        const group = new CemNativeGenerationGroup([first.generationParticipant(), second.generationParticipant()]);
        const a = first.claimNativeEmbedding(), b = second.claimNativeEmbedding();
        const ap = await first.closeNativeAdmission([a]), bp = await second.closeNativeAdmission([b]);
        const ar = await first.retireNativeGeneration(ap, [a], handoff()), br = await second.retireNativeGeneration(bp, [b], handoff());
        ar.checkpoint.fill(0);
        await assert.rejects(group.allocate([ar]), /coverage/);
        await assert.rejects(group.allocate([{ ...ar }, br]), /receipt/);
        admission = await group.allocate([br, ar]);
        assert.equal(admission.generations.length, 2);
        assert.match(new TextDecoder().decode(admission.generations[0].checkpoint), /terminal journal/);
        for (const item of admission.generations) {
            const fresh = item.runtime;
            assert.equal(fresh.runtimeInstanceId, item.runtimeId);
            assert.notEqual(item.previousRuntimeId, item.runtimeId);
            const owner = fresh.claimNativeEmbedding(); fresh.releaseNativeEmbedding(owner);
        }
        assert.throws(() => first.claimNativeEmbedding(), /closed/);
        assert.throws(() => second.claimNativeEmbedding(), /closed/);
        admission.revoke();
        for (const item of admission.generations) assert.throws(() => item.runtime.claimNativeEmbedding(), /allocation.closed/);
    } finally { admission?.revoke(); await first.close(); await second.close(); }
});

test('cancelled fresh startup terminates immediately and never accepts late initialization', async () => {
    const { initializeFreshGeneration } = await import('../dist/generation-lifecycle.js');
    const cancellation = new AbortController(); cancellation.abort();
    let started = 0, terminated = 0;
    await assert.rejects(initializeFreshGeneration(cancellation.signal, async () => { started++; }, () => terminated++), /cancelled/);
    assert.equal(started, 0); assert.equal(terminated, 1);
    const active = new AbortController(); let resolve;
    const pending = initializeFreshGeneration(active.signal, () => new Promise(done => { resolve = done; }), () => terminated++);
    active.abort(); await assert.rejects(pending, /cancelled/); resolve('late');
    assert.equal(terminated, 2);
});
