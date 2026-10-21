import assert from 'node:assert/strict';
import test from 'node:test';
import * as native from '@epa-wg/cem-ml/wasm';
import { copyCommandNativeEmbeddingOwnership, CommandNativeEmbeddingOwners, parseCommandNativeInventory } from '../dist/native-embedding.js';
import { CommandNativeClientCheckpoint } from '../dist/native-admission.js';
import { createNodeCommandServiceClient, claimNodeCommandNativeEmbedding, releaseNodeCommandNativeEmbedding,
    checkpointNodeCommandNativeEmbedding, isNodeCommandNativeInventoryCurrent } from '../dist/node.js';

const inventory = () => parseCommandNativeInventory(native.commandNativeResourceInventory());
const count = (snapshot, name) => snapshot.registries.find(entry => entry.name === name).live;
const versionRequest = requestId => ({ protocolVersion: 1, requestId, project: { projectId: 'inventory-fixture', revision: 1 },
    resourceVersions: {}, operation: { kind: 'version-capabilities' }, runPlan: null, resources: {},
    policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } });
const host = { currentRevision: ({ project }) => ({ project, resourceVersions: {} }),
    readResource: () => { throw Error('unexpected read'); }, prepareWrite: () => { throw Error('unexpected write'); },
    commitWrite: () => { throw Error('unexpected commit'); }, rollbackWrite: () => undefined };

test('original leases require every live owner and pin the actual memory generation', () => {
    let identity = {};
    const owners = new CommandNativeEmbeddingOwners(identity, () => identity, () => undefined);
    assert.throws(() => copyCommandNativeEmbeddingOwnership(Array(257)), { code: 'cem.command.embedding.limit' });
    const a = owners.claim(), b = owners.claim();
    for (const values of [[], [a], [a, a], [{ ...a }, b], [structuredClone(a), b]]) assert.throws(() => owners.acknowledge(values));
    const ack = owners.acknowledge([a, b]);
    assert.equal(owners.current(ack), true);
    assert.equal(owners.current({ ...ack }), false);
    const c = owners.claim(); assert.equal(owners.current(ack), false);
    owners.release(c);
    assert.throws(() => owners.acknowledge([a, b, c]));
    const current = owners.acknowledge([a, b]);
    identity = {};
    assert.equal(owners.current(current), false);
    assert.throws(() => owners.claim(), { code: 'cem.command.embedding.generation' });
    owners.revoke();
    assert.equal(owners.current(current), false);
    const bounded = new CommandNativeEmbeddingOwners({}, () => generation, () => undefined);
    const generation = {};
    // A foreign identity cannot allocate even its first lease.
    assert.throws(() => bounded.claim(), { code: 'cem.command.embedding.generation' });
    const full = new CommandNativeEmbeddingOwners(generation, () => generation, () => undefined);
    const leases = Array.from({ length: 256 }, () => full.claim());
    assert.throws(() => full.claim(), { code: 'cem.command.embedding.limit' });
    assert.equal(full.current(full.acknowledge(leases)), true);
});

test('ownership changes during asynchronous acknowledgement fence the original result', async () => {
    const state = new CommandNativeClientCheckpoint(() => true);
    const owner = state.claimEmbedding();
    let finish;
    const capturing = state.capture(() => new Promise(resolve => { finish = resolve; }), 1000, [owner]);
    state.claimEmbedding();
    finish({ scope: 'managed-cli-only', replacementAllowed: false, revision: 1, nativeInventory: inventory() });
    await assert.rejects(capturing, { code: 'cem.command.checkpoint.blocked' });
});

test('census DTO validation remains bounded, immutable and separate from replacement authority', () => {
    const snapshot = inventory();
    assert.equal(snapshot.registries.length, 29);
    assert.equal(Object.isFrozen(snapshot.registries[0]), true);
    assert.equal(snapshot.replacementAllowed, false);
    for (const invalid of [{ ...snapshot, complete: false }, { ...snapshot, replacementAllowed: true },
        { ...snapshot, blockingResources: snapshot.blockingResources + 1 }, { ...snapshot, extra: true },
        { ...snapshot, registries: [snapshot.registries[0], snapshot.registries[0]] }, 'x'.repeat(16385)]) {
        assert.throws(() => parseCommandNativeInventory(invalid));
    }
});

test('command-worker inventory acknowledgements require all original embedding leases', async () => {
    const client = await createNodeCommandServiceClient({ host });
    const other = await createNodeCommandServiceClient({ host });
    try {
        const a = client.claimNativeEmbedding(), b = client.claimNativeEmbedding(), foreign = other.claimNativeEmbedding();
        for (const owners of [[a], [a, a], [{ ...a }, b], [a, foreign]]) await assert.rejects(client.checkpointNative(1000, owners));
        assert.equal((await client.execute(versionRequest('ownership-before-close'))).status, 'succeeded');
        const proof = await client.checkpointNative(1000, [a, b]);
        assert.equal(proof.nativeInventory.blockingResources, 0);
        assert.equal(proof.embeddingOwners, 2);
        assert.equal(proof.replacementAllowed, false);
        assert.equal(client.isNativeCheckpointCurrent(proof), true);
        assert.equal(client.isNativeCheckpointCurrent({ ...proof }), false);
        assert.equal(other.isNativeCheckpointCurrent(proof), false);
        client.releaseNativeEmbedding(b);
        assert.equal(client.isNativeCheckpointCurrent(proof), false);
        await assert.rejects(client.checkpointNative(1000, [a, b]));
        assert.equal((await client.checkpointNative(1000, [a])).embeddingOwners, 1);
        const legacy = await other.checkpointNative();
        assert.equal(legacy.nativeInventory, undefined);
        assert.equal(legacy.scope, 'managed-cli-only');
    } finally { await client.close(); await other.close(); }
});

test('native census catches raw retained roots, callbacks, and same-count changes outside the CLI ledger', async () => {
    const a = claimNodeCommandNativeEmbedding(), b = claimNodeCommandNativeEmbedding();
    const bytes = new TextEncoder().encode('<item/>');
    assert.ok(native.commandNativeMemoryIdentity() instanceof WebAssembly.Memory);
    const document = native.retainCemDocument(bytes, 'application/xml', 'memory:raw-document');
    assert.equal(count(inventory(), 'cem-documents'), 1);
    await assert.rejects(checkpointNodeCommandNativeEmbedding([a, b]), { code: 'cem.command.inventory.blocked' });
    native.disposeCemDocument(document);
    const template = JSON.parse(native.compileTemplate('{p | hello}', '{}')).artifactId;
    assert.ok(template > 0);
    assert.equal(count(inventory(), 'templates'), 1);
    await assert.rejects(checkpointNodeCommandNativeEmbedding([a, b]), { code: 'cem.command.inventory.blocked' });
    native.disposeTemplate(template);
    const source = native.parseReferenceSource(bytes, 'application/xml', 'memory:raw-source', '{"maxBytes":262144,"maxNodes":4096}');
    assert.equal(count(inventory(), 'reference-sources'), 1);
    await assert.rejects(checkpointNodeCommandNativeEmbedding([a, b]), { code: 'cem.command.inventory.blocked' });
    native.disposeReferenceSource(source);
    native.onParseEvent(() => undefined);
    assert.equal(count(inventory(), 'parse-observers'), 1);
    await assert.rejects(checkpointNodeCommandNativeEmbedding([a, b]), { code: 'cem.command.inventory.blocked' });
    native.offParseEvent();
    const proof = await checkpointNodeCommandNativeEmbedding([a, b]);
    assert.equal(proof.nativeInventory.blockingResources, 0);
    assert.equal(isNodeCommandNativeInventoryCurrent(proof), true);
    assert.equal(isNodeCommandNativeInventoryCurrent(structuredClone(proof)), false);
    const transient = native.retainCemDocument(bytes, 'application/xml', 'memory:transient');
    native.disposeCemDocument(transient);
    assert.equal(inventory().blockingResources, 0);
    assert.equal(isNodeCommandNativeInventoryCurrent(proof), false);
    const next = await checkpointNodeCommandNativeEmbedding([a, b]);
    releaseNodeCommandNativeEmbedding(b);
    assert.equal(isNodeCommandNativeInventoryCurrent(next), false);
    assert.equal((await checkpointNodeCommandNativeEmbedding([a])).embeddingOwners, 1);
    releaseNodeCommandNativeEmbedding(a);
});

test('native bridge and memory reservations include commands before their first poll and through callback waits', async () => {
    const metadata = (await import('@epa-wg/cem-ml/runtime.json', { with: { type: 'json' } })).default;
    const capability = JSON.stringify({ runtime: 'wasm-node', targetIdentity: 'wasm32-unknown-unknown:nodejs',
        abiIdentity: metadata.abi.identity, debugControlActive: false });
    let entered, resume;
    const requested = new Promise(resolve => { entered = resolve; });
    const pending = native.executeCommandServiceV1(JSON.stringify(versionRequest('unpolled-command')), capability,
        json => new Promise(resolve => { resume = () => resolve(JSON.stringify({ project: JSON.parse(json).project, resourceVersions: {} })); entered(); }),
        () => { throw Error('unexpected read'); }, () => { throw Error('unexpected write'); },
        () => { throw Error('unexpected commit'); }, () => undefined);
    let owner;
    try {
        const unpolled = inventory();
        assert.equal(count(unpolled, 'command-bridges'), 1);
        assert.equal(count(unpolled, 'memory-operations'), 1);
        await requested;
        assert.equal(count(inventory(), 'command-operations'), 1);
        owner = claimNodeCommandNativeEmbedding();
        await assert.rejects(checkpointNodeCommandNativeEmbedding([owner]), { code: 'cem.command.inventory.blocked' });
        resume();
        assert.equal(JSON.parse(await pending).status, 'succeeded');
        assert.equal(inventory().blockingResources, 0);
        assert.equal((await checkpointNodeCommandNativeEmbedding([owner])).nativeInventory.blockingResources, 0);
    } finally { resume?.(); await pending; if (owner) releaseNodeCommandNativeEmbedding(owner); }
});
