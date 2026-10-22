import assert from 'node:assert/strict';
import test from 'node:test';
import { execFileSync } from 'node:child_process';
import { CommandNativeAdmission, CommandNativeClientCheckpoint } from '../dist/native-admission.js';

const healthy = () => JSON.stringify({ softPressure: false, hardPressure: false });

test('native pressure closure is sticky, separate heaps stay independent, cleanup drains', async () => {
    let pressure = false;
    const domain = new CommandNativeAdmission(() => JSON.stringify({ softPressure: pressure, hardPressure: false }));
    const other = new CommandNativeAdmission(healthy);
    const release = domain.admit();
    pressure = true;
    assert.throws(() => domain.admit(), { code: 'cem.runtime.memory.closed' });
    pressure = false;
    assert.throws(() => domain.admit(), { code: 'cem.runtime.memory.closed' });
    other.admit()();
    let settled = false;
    const draining = domain.drain().then(proof => { settled = true; return proof; });
    await Promise.resolve(); assert.equal(settled, false);
    const cleanup = domain.admit(true);
    release(); release();
    await Promise.resolve(); assert.equal(settled, false);
    cleanup();
    const proof = await draining;
    assert.equal(domain.current(proof), true);
    assert.equal(proof.replacementAllowed, false);
    assert.equal(other.current(proof), false);
    assert.equal(domain.current({ ...proof }), false);
    assert.equal(domain.current(structuredClone(proof)), false);
    domain.retain('artifact');
    assert.equal(domain.current(proof), false);
    assert.throws(() => domain.checkpoint(proof), { code: 'cem.command.checkpoint.blocked' });
    const retained = await domain.drain();
    assert.throws(() => domain.checkpoint(retained), { code: 'cem.command.checkpoint.blocked' });
    domain.release('artifact');
    assert.equal(domain.checkpoint(await domain.drain()).scope, 'managed-cli-only');
});

test('bad telemetry closes conservatively; a timed-out native drain never revives', async () => {
    for (const sample of [() => '{', () => '{}', () => 'x'.repeat(2049), () => { throw Error('lost'); }]) {
        const domain = new CommandNativeAdmission(sample);
        assert.throws(() => domain.admit(), { code: 'cem.runtime.memory.closed' });
        assert.equal(domain.closure, 'telemetry-unavailable');
    }
    const domain = new CommandNativeAdmission(healthy);
    const release = domain.admit();
    domain.close();
    await assert.rejects(domain.drain(5), { code: 'cem.command.checkpoint.uncertain' });
    release(); domain.admit(true)();
    await assert.rejects(domain.drain(), { code: 'cem.command.checkpoint.uncertain' });
});

test('resource identities, counts, uncertainty and overflow fail closed', async () => {
    const domain = new CommandNativeAdmission(healthy);
    domain.retain('request:x', 2); domain.close();
    domain.release('request:x', 0);
    assert.throws(() => domain.checkpoint({}), { code: 'cem.command.checkpoint.blocked' });
    domain.release('request:x', 1);
    const partial = await domain.drain();
    assert.throws(() => domain.checkpoint(partial), { code: 'cem.command.checkpoint.blocked' });
    domain.release('request:x', 1);
    domain.checkpoint(await domain.drain());
    for (const invalidate of [d => { d.retain('same'); d.retain('same'); }, d => d.retain('x', NaN), d => { d.retain('x'); d.release('x', 2); }, d => d.retain('x'.repeat(2049))]) {
        const uncertain = new CommandNativeAdmission(healthy); invalidate(uncertain); uncertain.close();
        await assert.rejects(uncertain.drain(), { code: 'cem.command.checkpoint.uncertain' });
    }
});

test('host checkpoint witnesses fence callbacks, copied/foreign receipts and cleanup', async () => {
    let idle = true;
    const host = new CommandNativeClientCheckpoint(() => idle);
    const reply = { scope: 'managed-cli-only', replacementAllowed: false, revision: 1 };
    const proof = await host.capture(async () => reply);
    assert.equal(host.current(proof), true);
    assert.equal(host.current({ ...proof }), false);
    assert.equal(new CommandNativeClientCheckpoint(() => true).current(proof), false);
    assert.throws(() => host.assertExecution(), { code: 'cem.runtime.memory.closed' });
    const retire = host.callback();
    assert.equal(host.current(proof), false);
    await assert.rejects(host.capture(async () => reply), { code: 'cem.command.checkpoint.blocked' });
    assert.equal(host.current(undefined), false);
    retire(); idle = false;
    await assert.rejects(host.capture(async () => reply), { code: 'cem.command.checkpoint.blocked' });
    idle = true;
    const next = await host.capture(async () => reply);
    host.changed(); assert.equal(host.current(next), false);
    host.invalidate();
    await assert.rejects(host.capture(async () => reply), { code: 'cem.command.checkpoint.blocked' });
});

test('host checkpoint acknowledgements are bounded and late answers cannot restore trust', async () => {
    const host = new CommandNativeClientCheckpoint(() => true);
    let complete;
    const waiting = host.capture(() => new Promise(resolve => { complete = resolve; }), 5);
    await assert.rejects(waiting, { code: 'cem.command.checkpoint.timeout' });
    const reply = { scope: 'managed-cli-only', replacementAllowed: false, revision: 1 };
    complete(reply);
    await assert.rejects(host.capture(async () => reply), { code: 'cem.command.checkpoint.blocked' });
    for (const bad of [0, -1, NaN, 60001]) await assert.rejects(host.capture(async () => reply, bad), RangeError);
    const clean = new CommandNativeClientCheckpoint(() => true);
    for (const invalid of [{ ...reply, replacementAllowed: true }, { ...reply, scope: 'all' }, { ...reply, extra: 1 }, null]) {
        await assert.rejects(clean.capture(async () => invalid), { code: 'cem.command.checkpoint.blocked' });
    }
});

test('real Node command client closes admission, waits for callbacks and rejects stale witnesses', async () => {
    const { createNodeCommandServiceClient } = await import('../dist/node.js');
    let entered, resume;
    const requested = new Promise(resolve => { entered = resolve; });
    const host = {
        currentRevision: ({ project }) => new Promise(resolve => {
            resume = () => resolve({ project, resourceVersions: {} }); entered();
        }),
        readResource: () => { throw Error('unexpected'); }, prepareWrite: () => { throw Error('unexpected'); },
        commitWrite: () => { throw Error('unexpected'); }, rollbackWrite: () => { throw Error('unexpected'); },
    };
    const client = await createNodeCommandServiceClient({ host });
    const other = await createNodeCommandServiceClient({ host: { ...host, currentRevision: ({ project }) => ({ project, resourceVersions: {} }) } });
    try {
        const running = client.execute(versionRequest('drain-pending'));
        await requested;
        let acknowledged = false;
        const capture = client.checkpointNative().then(proof => { acknowledged = true; return proof; });
        assert.throws(() => client.execute(versionRequest('refused')), { code: 'cem.runtime.memory.closed' });
        await running.cancel('drain fixture');
        assert.equal(acknowledged, false);
        resume();
        assert.equal((await running).status, 'cancelled');
        const proof = await capture;
        assert.equal(client.isNativeCheckpointCurrent(proof), true);
        assert.equal(client.isNativeCheckpointCurrent({ ...proof }), false);
        assert.equal(other.isNativeCheckpointCurrent(proof), false);
        assert.equal((await other.execute(versionRequest('independent'))).status, 'succeeded');
        assert.equal(proof.scope, 'managed-cli-only'); assert.equal(proof.replacementAllowed, false);
        await assert.rejects(client.cancel('unknown'), error => error.code !== undefined);
        assert.equal(client.isNativeCheckpointCurrent(proof), false);
    } finally { await client.close(); await other.close(); }
});

test('real Node artifact handles block checkpoint until cleanup after admission closes', async () => {
    const { createNodeCommandServiceClient } = await import('../dist/node.js');
    const { readFile } = await import('node:fs/promises');
    const runPlan = JSON.parse(await readFile(new URL('./browser-command-run-plan.fixture.json', import.meta.url)));
    const bytes = new TextEncoder().encode('.card { color: red; }');
    const version = { revision: 1, sha256: (await import('node:crypto')).createHash('sha256').update(bytes).digest('hex') };
    const request = { ...versionRequest('retained-artifacts'), resourceVersions: { 'memory:fixture.css': version },
        operation: { kind: 'parse', inputId: 'input:0', projection: 'ast', preserveSourceOffsets: true }, runPlan };
    const client = await createNodeCommandServiceClient({ host: {
        currentRevision: ({ project }) => ({ project, resourceVersions: request.resourceVersions }),
        readResource: () => ({ version, bytes: [...bytes], identity: { contentType: 'text/css', schema: 'https://cem.dev/ns/data/css/1', defaultNamespace: null, namespaces: {}, baseUri: null } }),
        prepareWrite: request => ({ token: request.uri }), commitWrite: token => ({ uri: token }), rollbackWrite: () => undefined,
    } });
    try {
        const handle = client.execute(request);
        const result = await handle;
        assert.equal(result.status, 'succeeded');
        assert.ok(result.artifacts.originalCount > 1);
        await assert.rejects(client.checkpointNative(), { code: 'cem.command.checkpoint.blocked' });
        await assert.rejects(handle.readArtifact(result.artifacts.items[0], { maxBytes: 16 }), { code: 'cem.runtime.memory.closed' });
        await handle.disposeArtifact(result.artifacts.items[0]);
        await assert.rejects(client.checkpointNative(), { code: 'cem.command.checkpoint.blocked' });
        await handle.dispose();
        const proof = await client.checkpointNative();
        assert.equal(client.isNativeCheckpointCurrent(proof), true);
        await handle.dispose(); // idempotent cleanup still changes the activity fence
        assert.equal(client.isNativeCheckpointCurrent(proof), false);
    } finally { await client.close(); }
});

function versionRequest(requestId) {
    return { protocolVersion: 1, requestId, project: { projectId: 'admission-fixture', revision: 1 },
        resourceVersions: {}, operation: { kind: 'version-capabilities' }, runPlan: null, resources: {},
        policyStamp: { resolver: 'fixture', safety: 'fixture', budget: 'fixture' } };
}

test('production Node worker under real native pressure refuses new work and retains cleanup without restart', async () => {
    const { Worker } = await import('node:worker_threads');
    const runtimeUrl = new URL('../../cem-ml-npm/dist/wasm/node/cem_ml.js', import.meta.url).href;
    const workerUrl = new URL('../dist/node-worker.js', import.meta.url).href;
    const worker = new Worker(new URL(`data:text/javascript,${encodeURIComponent(`
        import { parentPort } from 'node:worker_threads';
        import * as native from ${JSON.stringify(runtimeUrl)};
        parentPort.on('message', message => {
            if (message.type !== 'fixture-native-pressure') return;
            const status = JSON.parse(native.runtimeMemoryStatus());
            native.configureRuntimeMemoryBudget(JSON.stringify({ ...status.config, softLimitBytes: 1 }));
            parentPort.postMessage({ type: 'fixture-pressure-set' });
        });
        await import(${JSON.stringify(workerUrl)});
    `)}`), { workerData: { worker: { slot: 1, generation: 1 }, effectiveWorkers: 1 } });
    const queue = [], waiters = new Set();
    let transportError;
    worker.on('message', message => { queue.push(message); for (const wake of [...waiters]) wake(); });
    worker.on('error', error => { transportError = error; for (const wake of [...waiters]) wake(); });
    const wait = predicate => new Promise((resolve, reject) => {
        const timeout = setTimeout(() => finish(Error('fixture message timeout')), 5000);
        const finish = (error, value) => { clearTimeout(timeout); waiters.delete(wake); if (error) reject(error); else resolve(value); };
        const wake = () => {
            if (transportError) return finish(transportError);
            const index = queue.findIndex(predicate);
            if (index >= 0) finish(undefined, queue.splice(index, 1)[0]);
        };
        waiters.add(wake); wake();
    });
    try {
        await wait(message => message.operation?.kind === 'initialize');
        worker.postMessage({ type: 'cem-command-execute', executionId: 1, request: versionRequest('pressure-pending') });
        const callback = await wait(message => message.type === 'cem-command-capability-request');
        worker.postMessage({ type: 'cem-command-capability-response', callbackId: callback.callbackId, ok: true,
            value: { project: versionRequest('pressure-pending').project, resourceVersions: {} } });
        const completed = await wait(message => message.type === 'cem-command-result' && message.executionId === 1);
        assert.equal(completed.result.status, 'succeeded');
        worker.postMessage({ type: 'fixture-native-pressure' });
        await wait(message => message.type === 'fixture-pressure-set');
        worker.postMessage({ type: 'cem-command-execute', executionId: 2, request: versionRequest('pressure-refused') });
        const refusal = await wait(message => message.executionId === 2 && message.type === 'cem-command-result');
        assert.equal(refusal.result.error.code, 'cem.runtime.memory.closed');
        assert.equal(refusal.nativeAdmissionClosed, true);
        worker.postMessage({ type: 'cem-command-native-checkpoint', actionId: 1, timeoutMs: 1000 });
        worker.postMessage({ type: 'cem-command-cancel', actionId: 2, requestId: 'pressure-pending', reason: 'pressure fixture' });
        const cancel = await wait(message => message.type === 'cem-command-action-result' && message.actionId === 2);
        assert.equal(typeof cancel.result.error.code, 'string');
        const checkpoint = await wait(message => message.type === 'cem-command-action-result' && message.actionId === 1);
        assert.equal(checkpoint.result.scope, 'managed-cli-only');
        assert.equal(checkpoint.result.replacementAllowed, false);
        assert.equal(transportError, undefined);
    } finally { await worker.terminate(); }
});

test('main authoring and all direct CLI clients share real pressure closure without covering raw generated exports', async () => {
    const native = await import('../../cem-ml-npm/dist/wasm/node/cem_ml.js');
    const { commandNativeAdmission: domain, commandNativeRuntime: managed } = await import('../dist/managed-native-runtime.js');
    const { buildNodeCommandInvocation, createNodeCommandHost, parseCemMlCommand } = await import('../dist/node.js');
    const duplicate = await import('../dist/managed-native-runtime.js?duplicate-cli-evaluation');
    assert.equal(duplicate.commandNativeAdmission, domain);
    const hostId = JSON.parse(managed.initializeResumableOperationHost(1)).hostId;
    const status = JSON.parse(native.runtimeMemoryStatus());
    native.configureRuntimeMemoryBudget(JSON.stringify({ ...status.config, softLimitBytes: 1 }));
    assert.throws(() => managed.version(), { code: 'cem.runtime.memory.closed' });
    await assert.rejects(buildNodeCommandInvocation(parseCemMlCommand(['version'], { runtime: 'wasm-node' }), createNodeCommandHost()), { code: 'cem.runtime.memory.closed' });
    const retained = await domain.drain();
    assert.throws(() => domain.checkpoint(retained), { code: 'cem.command.checkpoint.blocked' });
    managed.disposeResumableOperationHost(hostId);
    domain.checkpoint(await domain.drain());
    assert.throws(() => managed.configureRuntimeMemoryBudget, { code: 'cem.command.native.unmanaged' });
    // A trusted low-level caller still has a raw alias. Managed coverage explicitly
    // does not claim that alias or authorize replacement of its physical heap.
    native.configureRuntimeMemoryBudget(JSON.stringify(status.config));
    assert.equal(typeof native.version(), 'string');
    assert.throws(() => managed.version(), { code: 'cem.runtime.memory.closed' });
});


test('native admission and waiter caps do not omit pending clients', async () => {
    const domain = new CommandNativeAdmission(healthy);
    const retire = domain.admit(); domain.close();
    const waiting = Array.from({ length: 16 }, () => domain.drain());
    await assert.rejects(domain.drain(), { code: 'cem.command.checkpoint.limit' });
    retire();
    const proofs = await Promise.all(waiting);
    assert.ok(proofs.every(proof => domain.current(proof)));
    const overflowing = new CommandNativeAdmission(healthy);
    const pending = Array.from({ length: 256 }, () => overflowing.admit());
    assert.throws(() => overflowing.admit(), { code: 'cem.command.checkpoint.limit' });
    pending.forEach(finish => finish()); overflowing.close();
    await assert.rejects(overflowing.drain(), { code: 'cem.command.checkpoint.uncertain' });
});

test('automatic native deadline fences capture without an explicit waiter', async context => {
    context.mock.timers.enable({ apis: ['setTimeout'] });
    const domain = new CommandNativeAdmission(healthy);
    const retire = domain.admit(); domain.close();
    context.mock.timers.tick(5000);
    retire();
    await assert.rejects(domain.drain(), { code: 'cem.command.checkpoint.uncertain' });
});


test('revoked checkpoint controllers release their original physical memory', () => {
    const moduleUrl = new URL('../dist/native-admission.js', import.meta.url).href;
    const output = execFileSync(process.execPath, ['--expose-gc', '--input-type=module', '-e', `
        import { CommandNativeClientCheckpoint } from ${JSON.stringify(moduleUrl)};
        const { checkpoint, weak } = (() => {
            const memory = new WebAssembly.Memory({ initial: 1 });
            return { checkpoint: new CommandNativeClientCheckpoint(() => true, memory), weak: new WeakRef(memory) };
        })();
        checkpoint.invalidate();
        for (let i = 0; i < 20; i++) {
            await new Promise(resolve => setImmediate(resolve)); global.gc();
        }
        if (weak.deref() !== undefined) throw Error('disposed memory retained by original checkpoint controller');
        try { checkpoint.claimEmbedding(); throw Error('revoked ownership reopened'); }
        catch (error) { if (error.code !== 'cem.command.embedding.generation') throw error; }
        try { checkpoint.assertExecution(); throw Error('revoked admission reopened'); }
        catch (error) { if (error.code !== 'cem.runtime.memory.closed') throw error; }
        console.log('released');
    `], { encoding: 'utf8', timeout: 30000 });
    assert.equal(output.trim(), 'released');
});
