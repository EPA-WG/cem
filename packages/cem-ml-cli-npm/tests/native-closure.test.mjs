import assert from 'node:assert/strict';
import test from 'node:test';
import * as native from '@epa-wg/cem-ml/wasm';
import { CommandNativeClientCheckpoint } from '../dist/native-admission.js';
import { parseCommandNativeClosure } from '../dist/native-embedding.js';
import { claimNodeCommandNativeEmbedding, releaseNodeCommandNativeEmbedding, closeNodeCommandNativeAdmission,
    revalidateNodeCommandNativeCheckpoint, isNodeCommandNativeInventoryCurrent, createNodeCommandServiceClient } from '../dist/node.js';

const inventory = () => JSON.parse(native.commandNativeResourceInventory());
const host = { currentRevision: ({ project }) => ({ project, resourceVersions: {} }),
    readResource: () => { throw Error('unexpected read'); }, prepareWrite: () => { throw Error('unexpected write'); },
    commitWrite: () => { throw Error('unexpected commit'); }, rollbackWrite: () => undefined };
const version = requestId => ({ protocolVersion: 1, requestId, project: { projectId: 'closure-fixture', revision: 1 },
    resourceVersions: {}, operation: { kind: 'version-capabilities' }, runPlan: null, resources: {},
    policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } });
const closed = error => error === 'cem.runtime.admission.closed';

function response(snapshot) {
    return { revision: 1, scope: 'managed-cli-only', replacementAllowed: false, nativeInventory: snapshot,
        nativeClosure: { schemaVersion: 1, closed: true, inventoryRevision: snapshot.revision, replacementAllowed: false } };
}

test('native closure records remain bounded control evidence', () => {
    const original = { schemaVersion: 1, closed: true, inventoryRevision: 3, replacementAllowed: false };
    assert.equal(Object.isFrozen(parseCommandNativeClosure(original, 3)), true);
    for (const value of [{ ...original, extra: true }, { ...original, closed: false }, { ...original, replacementAllowed: true },
        { ...original, inventoryRevision: 4 }, 'x'.repeat(513)]) assert.throws(() => parseCommandNativeClosure(value, 3));
});

test('owner mutations and stale native replies fence coordinated asynchronous validation', async () => {
    const state = new CommandNativeClientCheckpoint(() => true);
    const owner = state.claimEmbedding();
    const proof = await state.capture(async () => response(inventory()), 1000, [owner], 'close');
    assert.throws(() => state.requireCoordinated({ ...proof }), { code: 'cem.command.native.stale' });
    let finish;
    const capturing = state.capture(() => new Promise(resolve => { finish = resolve; }), 1000, [owner], proof);
    const second = state.claimEmbedding(); state.releaseEmbedding(second);
    finish(response(inventory()));
    await assert.rejects(capturing, { code: 'cem.command.checkpoint.blocked' });
    assert.equal(state.current(proof), false);
    const fresh = await state.capture(async () => response(inventory()), 1000, [owner], 'close');
    const stale = inventory(); stale.revision += 1;
    await assert.rejects(state.capture(async () => response(stale), 1000, [owner], fresh), { code: 'cem.command.native.stale' });
});

test('coordinated host deadlines and worker loss never revive late native evidence', async () => {
    const state = new CommandNativeClientCheckpoint(() => true), owner = state.claimEmbedding();
    let finish;
    const pending = state.capture(() => new Promise(resolve => { finish = resolve; }), 5, [owner], 'close');
    await assert.rejects(pending, { code: 'cem.command.checkpoint.timeout' });
    finish(response(inventory()));
    await assert.rejects(state.capture(async () => response(inventory()), 1000, [owner], 'close'));
    const lost = new CommandNativeClientCheckpoint(() => true), lease = lost.claimEmbedding();
    const proof = await lost.capture(async () => response(inventory()), 1000, [lease], 'close');
    lost.invalidate(); assert.equal(lost.current(proof), false);
});

test('Node worker close and revalidation require current complete originals and independent heaps', async () => {
    const client = await createNodeCommandServiceClient({ host }), other = await createNodeCommandServiceClient({ host });
    try {
        const a = client.claimNativeEmbedding(), b = client.claimNativeEmbedding(), foreign = other.claimNativeEmbedding();
        for (const owners of [[a], [a, a], [{ ...a }, b], [a, foreign]]) await assert.rejects(client.closeNativeAdmission(owners));
        const proof = await client.closeNativeAdmission([a, b]);
        assert.equal(proof.nativeClosure.closed, true);
        assert.equal(proof.nativeClosure.inventoryRevision, proof.nativeInventory.revision);
        assert.equal(proof.replacementAllowed, false);
        assert.throws(() => client.revalidateNativeCheckpoint(structuredClone(proof), [a, b]), { code: 'cem.command.native.stale' });
        assert.throws(() => other.revalidateNativeCheckpoint(proof, [foreign]), { code: 'cem.command.native.stale' });
        const validated = await client.revalidateNativeCheckpoint(proof, [a, b]);
        assert.equal(client.isNativeCheckpointCurrent(validated), true);
        assert.equal(client.isNativeCheckpointCurrent(proof), false);
        client.releaseNativeEmbedding(b);
        assert.throws(() => client.revalidateNativeCheckpoint(validated, [a]), { code: 'cem.command.native.stale' });
        assert.equal((await client.closeNativeAdmission([a])).embeddingOwners, 1);
        assert.equal((await other.execute(version('other-open'))).status, 'succeeded');
    } finally { await client.close(); await other.close(); }
});

test('raw main heap closure catches admitted callbacks and retained roots, preserves cleanup, and fences same-count mutations', async () => {
    const a = claimNodeCommandNativeEmbedding(), b = claimNodeCommandNativeEmbedding();
    const bytes = new TextEncoder().encode('<item/>');
    const document = native.retainCemDocument(bytes, 'application/xml', 'memory:before-close');
    const template = JSON.parse(native.compileTemplate('{p | retained}', '{}')).artifactId;
    const source = native.parseReferenceSource(bytes, 'application/xml', 'memory:before-source', '{"maxBytes":262144,"maxNodes":4096}');
    native.onParseEvent(() => undefined);
    const metadata = (await import('@epa-wg/cem-ml/runtime.json', { with: { type: 'json' } })).default;
    const capability = JSON.stringify({ runtime: 'wasm-node', targetIdentity: 'wasm32-unknown-unknown:nodejs', abiIdentity: metadata.abi.identity, debugControlActive: false });
    let entered, resume;
    const requested = new Promise(resolve => { entered = resolve; });
    const pending = native.executeCommandServiceV1(JSON.stringify(version('admitted-before-close')), capability,
        json => new Promise(resolve => { resume = () => resolve(JSON.stringify({ project: JSON.parse(json).project, resourceVersions: {} })); entered(); }),
        () => { throw Error('unexpected read'); }, () => { throw Error('unexpected write'); }, () => { throw Error('unexpected commit'); }, () => undefined);
    try {
        assert.equal(inventory().registries.find(entry => entry.name === 'native-entries').live, 1);
        await requested;
        await assert.rejects(closeNodeCommandNativeAdmission([a, b]), { code: 'cem.command.inventory.blocked' });
        for (const retain of [() => native.retainCemDocument(bytes, 'application/xml', 'memory:denied'),
            () => native.parseReferenceSource(bytes, 'application/xml', 'memory:denied', '{"maxBytes":262144,"maxNodes":4096}'),
            () => native.onParseEvent(() => undefined)]) assert.throws(retain, closed);
        assert.equal(JSON.parse(native.compileTemplate('{p | denied}', '{}')).error.code, 'cem.runtime.admission.closed');
        assert.equal(JSON.parse(native.authorTransformConfigV1('{}')).error.code, 'cem.runtime.admission.closed');
        let calls = 0; const forbidden = () => { calls++; throw Error('unexpected callback'); };
        const denied = JSON.parse(await native.executeCommandServiceV1('{}', capability, forbidden, forbidden, forbidden, forbidden, forbidden));
        assert.equal(denied.error.code, 'cem.runtime.admission.closed'); assert.equal(calls, 0);
        resume(); assert.equal(JSON.parse(await pending).status, 'succeeded');
        native.offParseEvent(); native.disposeCemDocument(document); native.disposeTemplate(template); native.disposeReferenceSource(source);
        const proof = await closeNodeCommandNativeAdmission([a, b]);
        assert.equal(isNodeCommandNativeInventoryCurrent(proof), true);
        await assert.rejects(revalidateNodeCommandNativeCheckpoint({ ...proof }, [a, b]), { code: 'cem.command.native.stale' });
        const validated = await revalidateNodeCommandNativeCheckpoint(proof, [a, b]);
        assert.equal(validated.nativeClosure.inventoryRevision, validated.nativeInventory.revision);
        native.resetRetainedRenderCache(); // no live root count changes
        assert.equal(inventory().blockingResources, 0);
        assert.equal(isNodeCommandNativeInventoryCurrent(validated), false);
        await assert.rejects(revalidateNodeCommandNativeCheckpoint(validated, [a, b]), { code: 'cem.command.native.stale' });
        const fresh = await closeNodeCommandNativeAdmission([a, b]);
        releaseNodeCommandNativeEmbedding(b);
        await assert.rejects(revalidateNodeCommandNativeCheckpoint(fresh, [a]), { code: 'cem.command.native.stale' });
    } finally {
        resume?.(); await pending;
        native.offParseEvent(); native.disposeCemDocument(document); native.disposeTemplate(template); native.disposeReferenceSource(source);
        releaseNodeCommandNativeEmbedding(a);
    }
});
