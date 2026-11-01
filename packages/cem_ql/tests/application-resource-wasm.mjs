import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker, MessageChannel, isMainThread, parentPort, workerData } from 'node:worker_threads';
import { createQuerySourceEndpoint, createQuerySourceWorkerEndpoint, installQuerySourceWorkerTransport, createApplicationSourceWasmBackend, QuerySourceResource } from '../runtime/query-source-resource.mjs';
import { registerNativeGeneration, CemNativeGenerationGroup, CemNativeRecoveryScope, releaseNativeGenerationResourcesAsync, completeNativeRetirement, issueNativeGenerationBindingTicket, resumeNativeGenerationBindings } from '../runtime/native-generation.mjs';

async function factory(directory) {
    const module = await import(pathToFileURL(resolve(directory, 'application_factory.js')).href);
    const runtime = module.createCemQlWasmGeneration();
    await runtime.initialize(await readFile(resolve(directory, 'application_source_probe_bg.wasm'))); return runtime;
}
function backend(runtime, observe) {
    const native = createApplicationSourceWasmBackend(runtime);
    return { ...native, open: (bytes, pins) => { const owner = native.open(bytes, pins); observe(owner); return owner; } };
}
const comparableReport = text => text;
if (!isMainThread) {
    const runtime = await factory(workerData.directory); let last;
    runtime.call('probeApplicationProvision', [workerData.pins, new Uint8Array(await readFile(resolve(workerData.directory, 'installed.inputs')))]);
    installQuerySourceWorkerTransport(workerData.port, backend(runtime, id => { last = id; }));
    parentPort.on('message', ({ id, op, args }) => {
        try {
            let value;
            if (op === 'prepare') value = runtime.prepareRetirement(args[0]);
            else if (op === 'dispose') value = runtime.dispose();
            else if (op === 'validate') value = runtime.call('probeApplicationValidate', [last]);
            else if(op==='tokens') {assert.throws(()=>runtime.call('applicationSourceCaptureV1',[Object.freeze({...last})]));assert.throws(()=>runtime.call('applicationSourceReleaseV1',[{}]));value=runtime.call('probeApplicationReentrant',[last]);}
            else value = runtime.call(op, args);
            parentPort.postMessage({ id, value });
        } catch (error) { parentPort.postMessage({ id, error: String(error) }); }
    });
    parentPort.postMessage({ ready: true });
} else {
    const directory = resolve(process.argv[2]);
    const { CemCommittedCheckpointStore } = await import(pathToFileURL(resolve(directory, 'checkpoint-store.js')).href);
    let bytes, pins;
    let allocations = 0, scenarios = 0;
    async function physical(mode) {
        const configPins = pins;
        const id = crypto.randomUUID(); let alive = true, endpoint, participant, last, worker, port, runtime, sequence = 0;
        const requests = new Map();
        if (mode === 'worker') {
            const channel = new MessageChannel(); port = channel.port1;
            worker = new Worker(new URL(import.meta.url), { workerData: { directory, pins: configPins, port: channel.port2 }, transferList: [channel.port2] });
            await new Promise((resolve, reject) => {
                const timer = setTimeout(() => reject(Error('worker startup timed out')), 5000);
                worker.on('error', reject);
                worker.on('message', reply => {
                    if (reply.ready) { clearTimeout(timer); resolve(); return; }
                    const pending = requests.get(reply.id); if (!pending) return;
                    requests.delete(reply.id); clearTimeout(pending.timer);
                    reply.error ? pending.reject(Error(reply.error)) : pending.resolve(reply.value);
                });
            });
        } else { runtime = await factory(directory); runtime.call('probeApplicationProvision', [configPins, new Uint8Array(await readFile(resolve(directory,'installed.inputs')))]); }
        const identity = () => mode === 'worker' ? worker : runtime.memory();
        const call = (op, args = []) => {
            if (mode !== 'worker') {
                if (op === 'prepare') return runtime.prepareRetirement(args[0]);
                if (op === 'dispose') return runtime.dispose();
                if (op === 'validate') return runtime.call('probeApplicationValidate', [last]);
                if(op==='tokens') {assert.throws(()=>runtime.call('applicationSourceCaptureV1',[Object.freeze({...last})]));assert.throws(()=>runtime.call('applicationSourceReleaseV1',[{}]));return runtime.call('probeApplicationReentrant',[last]);}
                return runtime.call(op, args);
            }
            return new Promise((resolve, reject) => {
                const n = ++sequence, timer = setTimeout(() => { requests.delete(n); reject(Error('worker control timeout')); }, 5000);
                requests.set(n, { resolve, reject, timer }); worker.postMessage({ id: n, op, args });
            });
        };
        const controller = { id, identity, call,
            participant: () => participant ??= registerNativeGeneration(identity(), id, async (admitted, signal) => {
                allocations++;
                const fresh = await physical(mode);
                return { identity: fresh.identity(), runtimeId: fresh.id, runtime: fresh,
                    current: () => !signal.aborted && fresh.alive(), discard: () => { void fresh.discard(); } };
            }),
            endpoint: () => endpoint ??= mode === 'worker'
                ? createQuerySourceWorkerEndpoint({ identity: identity(), participant: controller.participant(), port, maxSourceBytes: 4 * 1024 * 1024, current: () => alive })
                : createQuerySourceEndpoint({ identity: identity(), participant: controller.participant(), current: () => alive, native: backend(runtime, id => { last = id; }) }),
            alive: () => alive,
            async retire(handoff, current) {
                const guard = () => alive && handoff.current() && current();
                if (!guard()) throw Error('stale checkpoint');
                const checkpoint = handoff.retain();
                await releaseNativeGenerationResourcesAsync(controller.participant(), guard);
                if (!guard()) throw Error('stale policy');
                await call('probeApplicationUnprovision');
                await call('closeCommandNativeAdmission');
                const inventory = JSON.parse(await call('commandNativeResourceInventory'));
                assert.equal(inventory.schemaVersion, 2); assert.equal(inventory.profile, 'cem-ml-cem-ql-generation-v2');
                assert.equal(inventory.registries.length, 36); assert.equal(inventory.blockingResources, 0);
                await call('prepare', [inventory.revision]);
                if (!guard()) throw Error('stale checkpoint');
                const confirmation = await call('dispose'); assert.equal(confirmation.frozen, true);
                if (!guard()) throw Error('stale checkpoint');
                if (worker) await worker.terminate();
                else assert.equal(runtime.memory(), undefined);
                endpoint?.dispose(); alive = false;
                return completeNativeRetirement(participant, checkpoint);
            },
            async discard() {
                if (!alive) return; alive = false; endpoint?.dispose();
                if (worker) { await worker.terminate(); return; }
                await new Promise(resolve => setImmediate(resolve));
                try { runtime.call('probeApplicationUnprovision', []); } catch { /* Already unprovisioned or fenced. */ }
                try { runtime.dispose(); } catch { /* Remains fenced if native cleanup has not settled. */ }
            },
        };
        return controller;
    }
    async function checkpoint(controller, outcome = 'acknowledged', pad = 0) {
        const store = new CemCommittedCheckpointStore(controller.id,pad?4*1024*1024:1024*1024);
        const receipt = { maxBytes:store.maxBytes, runtimeId: controller.id, generation: 1, commitSequence: 1, policyStamp: 'application-source-fixture',
            owners: [{ instanceId: 'source-route', declarationIdentity: JSON.parse(pins).hash, dataRevision: 'original-source', acceptedEventSequence: 0 }],
            operations: [{ instanceId: 'source-route', operationId: 'terminal-operation', outcome }] };
        if(pad) for(let i=1;i<4;i++)receipt.owners.push({...receipt.owners[0],instanceId:`source-budget-${i}`});
        store.acknowledge(receipt);
        // Transport-valid source-route checkpoint only. No DOM or action is
        // restored from its island; application source reimport uses the resource cap.
        const candidate = JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt, owners: receipt.owners.map(owner=>({instanceId:owner.instanceId,island:'inert original source route'+' '.repeat(Math.floor(pad/receipt.owners.length))})) });
        await store.capture(candidate, (candidate, receipt) => controller.call('sealCommittedCheckpoint', [candidate, receipt]));
        return store;
    }
    for (const variant of ['application']) {
        bytes = new Uint8Array(await readFile(resolve(directory,'installed.cemah')));
        pins = await readFile(resolve(directory,'installed.pin.json'),'utf8');
    for (const mode of ['fallback', 'worker']) for (const scenario of ['resume', 'stale-checkpoint', 'entered-after-allocation', 'foreign-target', 'expired-owner', 'aggregate-budget']) {
        const old = await physical(mode); let privateInput = false, bound;
        const before = allocations, scope = new CemNativeRecoveryScope();
        const resource = await QuerySourceResource.open({ endpoint: old.endpoint(), bytes, pins,
            current: () => !privateInput, target: fresh => scenario === 'foreign-target' ? old.endpoint() : fresh.endpoint(), onBound: endpoint => { bound = endpoint; } });
        let admission, stores = [];
        try {
            const original = comparableReport(await old.call('validate', ['same']));
            assert.equal(original, 'true:true:true');
            assert.equal(await old.call('tokens'),true);
            if(scenario==='expired-owner'){await old.call('probeApplicationRetire');await assert.rejects(async()=>old.call('validate'));await assert.rejects(resource.prepareRecovery());continue;}
            const census = JSON.parse(await old.call('commandNativeResourceInventory'));
            assert.equal(census.registries.find(r => r.name === 'enrolled-compilations').live, 1);
            const unknown = await checkpoint(old, 'unknown'); stores.push(unknown);
            assert.throws(() => unknown.handoff(), /operations.unknown/);
            privateInput = true;
            await assert.rejects(resource.prepareRecovery(), /policy/);
            assert.throws(() => new CemNativeGenerationGroup([old.participant()]), /stale/);
            assert.equal(allocations, before);
            privateInput = false; await resource.prepareRecovery();
            const store = await checkpoint(old,'acknowledged',scenario==='aggregate-budget'?4*1024*1024-4096:0); stores.push(store);
            const handoff = store.handoff();
            const ticket = issueNativeGenerationBindingTicket(old.participant(), () => handoff.current() && !privateInput,
                async context => ({ owner: context.generation.runtime, current: () => !context.signal.aborted && handoff.current() && !privateInput,
                    bind: () => { assert.equal(context.active(), true); }, discard() {} }));
            scope.enroll({ id: old.id, kind: 'command-invocation', prepare: async () => ({ participants: [old.participant()], current: () => handoff.current() && !privateInput }) });
            const coverage = await scope.prepare();
            const group = coverage.createGenerationGroup();
            const retirement = await old.retire(handoff, () => !privateInput);
            if(scenario==='aggregate-budget'){await assert.rejects(group.allocate([retirement]),/limit/);assert.equal(allocations,before);continue;}
            if (scenario === 'stale-checkpoint') {
                store.invalidate(); await assert.rejects(group.allocate([retirement]), /stale/);
                assert.equal(allocations, before); assert.equal(bound, undefined); continue;
            }
            await assert.rejects(group.allocate([{ ...retirement }]), /receipt/);
            retirement.checkpoint.fill(0); // Exposed bytes cannot rewrite private checkpoint/journal state.
            admission = await group.allocate([retirement]);
            if (scenario === 'entered-after-allocation') privateInput = true;
            if (scenario !== 'resume') {
                await assert.rejects(resumeNativeGenerationBindings(admission, [ticket]), /stale|target/);
                assert.equal(bound, undefined); continue;
            }
            await resumeNativeGenerationBindings(admission, [ticket]);
            const fresh = admission.generations[0].runtime;
            assert.equal(bound, fresh.endpoint());
            assert.deepEqual(comparableReport(await fresh.call('validate', ['same'])), original);
            assert.equal(JSON.parse(new TextDecoder().decode(admission.generations[0].checkpoint)).receipt.operations[0].outcome, 'acknowledged');
            const freshInventory = JSON.parse(await fresh.call('commandNativeResourceInventory'));
            assert.equal(freshInventory.registries.find(r => r.name === 'enrolled-preparations').live, 0);
            await assert.rejects(old.endpoint().open(bytes, pins), /retired|closed|stale/);
            await resource.dispose();
            assert.equal(JSON.parse(await fresh.call('commandNativeResourceInventory')).blockingResources, 2, 'original deployment owner disappeared from the census');
            await fresh.call('probeApplicationUnprovision');
            assert.equal(JSON.parse(await fresh.call('commandNativeResourceInventory')).blockingResources, 0);
            console.log(`Application resource ${variant}/${mode}: native-sealed journal, full v2 census, original release, physical retirement and fresh source rebinding passed.`);
        } finally { scenarios++; await resource.dispose(); admission?.revoke(); scope.dispose(); await old.discard(); for (const store of stores) store.dispose(); }
    }
    }
    assert.equal(scenarios, 12);
    console.log('All 12 physical application handoff scenarios passed, including stale checkpoints, late credential policy and foreign targets.');
}
