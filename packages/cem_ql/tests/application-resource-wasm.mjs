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
function backend(runtime, runtimeId, scenario, observe) {
    const native = createApplicationSourceWasmBackend(runtime);
    let input, owner;
    return { ...native,
        open: (bytes, pins) => {
            owner = native.open(bytes, pins);
            input = runtime.call('probeApplicationReceiverInput', [owner]);
            observe(owner, input); return owner;
        },
        capture: owner => {
            // Fixture-only immutable deployment data is explicitly released.
            // The library refuses to omit live native input from a checkpoint.
            if (input) {
                assert.throws(() => native.capture(owner));
                runtime.call('applicationSourceReleaseReceiverInputV1', [owner, input]);
                input = undefined;
            }
            return native.capture(owner);
        },
        edit: (bytes, pins, edited) => {
            input = runtime.call('probeApplicationEditReceiver', [owner, input, bytes, pins, edited]);
            observe(owner, input);
        },
        receiver: {
            capture(owner) {
                const [bytes, revision] = runtime.call('probeApplicationCaptureReceiver', [owner, input]);
                return { bytes, revision };
            },
            retire(owner, bytes, journal) {
                if (scenario === 'mutable-foreign-bytes') bytes[bytes.length - 1] ^= 1;
                return runtime.call('probeApplicationConfirmReceiver', [owner, bytes, journal, runtimeId]);
            },
            open(bytes, pins, recovery) {
                [owner, input] = runtime.call('probeApplicationRestoreReceiver',
                    [bytes, pins, recovery.bytes, recovery.pin, recovery.journal, recovery.runtimeId]);
                observe(owner, input);
                return owner;
            },
        },
    };
}
const comparableReport = text => text;
async function frame(runtime, owner, inputOwner, directory, expected = '<p>Tea/3/ok/native-one</p>') {
    assert.throws(() => runtime.call('applicationSourceCaptureV1', [owner]));
    assert.throws(() => runtime.call('applicationSourceRenderReceiverV1', [owner, Object.freeze({ ...inputOwner }), 'native-one']));
    const native = runtime.call('applicationSourceRenderReceiverV1', [owner, inputOwner, 'native-one']);
    assert.equal(native, expected);
    const nativeProjection = runtime.call('applicationSourceProjectReceiverV1', [owner, inputOwner, 'native-one']);
    assert.ok(nativeProjection.byteLength > 0);
    assert.throws(() => runtime.call('applicationSourceRenderReceiverV1', [owner, inputOwner, 'bad identity']));
    const [island, badCount, badNamespace, inputs, routing] = await Promise.all([
        ...['frame.island', 'frame.bad-count', 'frame.bad-namespace', 'frame.inputs'].map(name =>
            readFile(resolve(directory, name)).then(bytes => new Uint8Array(bytes))),
        readFile(resolve(directory, 'frame.route.json'), 'utf8'),
    ]);
    const render = (bytes, instance = 'island-one', tag = 'cem-count') =>
        runtime.call('applicationSourceRenderIslandV1', [owner, tag, bytes, instance]);
    const html = render(island);
    assert.match(html, /3\/banana\/ Tea &amp; cake \/island-one/);
    const projection = runtime.call('applicationSourceProjectIslandV1', [owner, 'cem-count', island, 'island-one']);
    assert.equal(runtime.call('probeApplicationProjectionValid', [projection]), true);
    for (const bytes of [badCount, badNamespace, new Uint8Array([0])]) assert.throws(() => render(bytes));
    assert.throws(() => render(island, 'foreign-instance'));
    assert.throws(() => render(island, 'island-one', 'cem-value'));
    assert.throws(() => runtime.call('applicationSourceProjectIslandV1', [owner, 'cem-count', badNamespace, 'island-one']));
    const direct = runtime.call('applicationSourceRenderInputsV1', [owner, 'cem-count', inputs, routing, 'direct-one']);
    assert.match(direct, /3\/banana\/Tea\/direct-one/);
    for (const badRouting of [routing.replace('fruitName', 'wrong'), routing.replace('"index":0', '"index":99'), routing.replace('cart', 'count')]) {
        assert.throws(() => runtime.call('applicationSourceRenderInputsV1', [owner, 'cem-count', inputs, badRouting, 'direct-one']));
    }
    assert.equal(render(island), html, 'a rejected frame corrupted the publication');
    return [html, direct, native];
}
if (!isMainThread) {
    const runtime = await factory(workerData.directory); let last, lastInput;
    runtime.call('probeApplicationProvision', [workerData.pins, new Uint8Array(await readFile(resolve(workerData.directory, 'installed.inputs')))]);
    const route = backend(runtime, workerData.runtimeId, workerData.scenario, (id, input) => { last = id; lastInput = input; });
    installQuerySourceWorkerTransport(workerData.port, route);
    parentPort.on('message', async ({ id, op, args }) => {
        try {
            let value;
            if (op === 'prepare') value = runtime.prepareRetirement(args[0]);
            else if (op === 'dispose') value = runtime.dispose();
            else if (op === 'validate') value = runtime.call('probeApplicationValidate', [last]);
            else if (op === 'projection') value = runtime.call('applicationSourceProjectReceiverV1', [last, lastInput, 'native-one']);
            else if (op === 'frame') value = await frame(runtime, last, lastInput, workerData.directory, ...args);
            else if (op === 'edit') value = route.edit(...args);
            else if (op === 'claimWrite') value = runtime.call('probeApplicationClaimWrite', [last, ...args]);
            else if(op==='tokens') {assert.throws(()=>runtime.call('applicationSourceCaptureV1',[Object.freeze({...last})]));assert.throws(()=>runtime.call('applicationSourceReleaseV1',[{}]));value=runtime.call('probeApplicationReentrant',[last]);}
            else value = runtime.call(op, args);
            parentPort.postMessage({ id, value });
        } catch (error) { parentPort.postMessage({ id, error: String(error) }); }
    });
    parentPort.postMessage({ ready: true });
} else {
    const directory = resolve(process.argv[2]);
    const { CemCommittedCheckpointStore } = await import(pathToFileURL(resolve(directory, 'checkpoint-store.js')).href);
    const { issueCheckpointResumeTicket, resumeCemCheckpointOwners } = await import(pathToFileURL(resolve(directory, 'checkpoint-resume.js')).href);
    const coreDirectory = process.argv[3] && resolve(process.argv[3]);
    async function openSelectedConsumer() {
        const { createCemComponentWasmBindings } = await import(pathToFileURL(resolve(coreDirectory, 'cem_ql_factory.js')).href);
        const core = createCemComponentWasmBindings();
        await core.initialize(await WebAssembly.compile(await readFile(resolve(coreDirectory, 'cem_ql_bg.wasm'))));
        const [template, metadata, pin, route, cart] = await Promise.all([
            readFile(resolve(directory, 'projection-consumer.bin')),
            ...['json', 'hash', 'route.json'].map(ext => readFile(resolve(directory, `projection-consumer.${ext}`), 'utf8')),
            readFile(new URL('../fixtures/component-application/shopping-cart/cart.json', import.meta.url)),
        ]);
        const consumer = core.open(template, metadata, pin);
        const cartHtml = () => core.renderResponse('Cart', cart, 'app://shopping-cart/response.json');
        const original = cartHtml();
        assert.match(original, /Tea.*USD 18\.00/s);
        return {
            memory: core.memory(),
            verify(bytes, expected) {
                assert.ok(bytes instanceof Uint8Array);
                assert.equal(Buffer.from(bytes.subarray(0, 5)).toString('hex'), '43454d5603');
                assert.throws(() => consumer.renderInputs(bytes.subarray(0, 5), route));
                assert.equal(consumer.renderInputs(bytes, route), `<section>${expected}</section>`);
                assert.equal(cartHtml(), original);
            },
            dispose() {
                consumer.free();
                assert.equal(JSON.parse(core.inventory()).liveDeclarationOwners, 0);
                core.release();
            },
        };
    }
    let bytes, pins;
    let allocations = 0, scenarios = 0;
    async function physical(mode, scenario) {
        const configPins = pins;
        const id = crypto.randomUUID(); let alive = true, endpoint, participant, last, lastInput, worker, port, runtime, sequence = 0;
        const requests = new Map();
        let route;
        if (mode === 'worker') {
            const channel = new MessageChannel(); port = channel.port1;
            worker = new Worker(new URL(import.meta.url), { workerData: { directory, pins: configPins, port: channel.port2, runtimeId: id, scenario }, transferList: [channel.port2] });
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
        } else {
            runtime = await factory(directory);
            runtime.call('probeApplicationProvision', [configPins, new Uint8Array(await readFile(resolve(directory,'installed.inputs')))]);
            route = backend(runtime, id, scenario, (id, input) => { last = id; lastInput = input; });
        }
        const identity = () => mode === 'worker' ? worker : runtime.memory();
        const call = (op, args = []) => {
            if (mode !== 'worker') {
                if (op === 'prepare') return runtime.prepareRetirement(args[0]);
                if (op === 'dispose') return runtime.dispose();
                if (op === 'validate') return runtime.call('probeApplicationValidate', [last]);
                if (op === 'projection') return runtime.call('applicationSourceProjectReceiverV1', [last, lastInput, 'native-one']);
                if (op === 'frame') return frame(runtime, last, lastInput, directory, ...args);
                if (op === 'edit') return route.edit(...args);
                if (op === 'claimWrite') return runtime.call('probeApplicationClaimWrite', [last, ...args]);
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
                const fresh = await physical(mode, scenario);
                return { identity: fresh.identity(), runtimeId: fresh.id, runtime: fresh,
                    current: () => !signal.aborted && fresh.alive(), discard: () => { void fresh.discard(); } };
            }),
            endpoint: () => endpoint ??= mode === 'worker'
                ? createQuerySourceWorkerEndpoint({ identity: identity(), participant: controller.participant(), port, maxSourceBytes: 4 * 1024 * 1024,
                    current: () => alive, receiverRecovery: true, applicationStylesheets: true })
                : createQuerySourceEndpoint({ identity: identity(), participant: controller.participant(), current: () => alive, native: route }),
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
    async function checkpoint(controller, outcome = 'acknowledged', pad = 0, revision = 'original-source', operationId = 'terminal-operation') {
        const store = new CemCommittedCheckpointStore(controller.id,pad?4*1024*1024:1024*1024);
        const receipt = { maxBytes:store.maxBytes, runtimeId: controller.id, generation: 1, commitSequence: 1, policyStamp: 'application-source-fixture',
            owners: [{ instanceId: 'source-route', declarationIdentity: JSON.parse(pins).hash, dataRevision: revision, acceptedEventSequence: revision === 'original-source' ? 0 : 1 }],
            operations: [{ instanceId: 'source-route', operationId, outcome }] };
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
    for (const mode of ['fallback', 'worker']) for (const scenario of ['resume', 'stale-checkpoint', 'entered-after-allocation', 'foreign-target', 'expired-owner', 'aggregate-budget',
        'mutable-resume', 'mutable-stale-input', 'mutable-foreign-bytes', 'mutable-modified-journal', 'mutable-private',
        'mutable-aggregate-budget']) {
        const old = await physical(mode, scenario); let privateInput = false, bound, journal;
        const mutable = scenario.startsWith('mutable-');
        const before = allocations, scope = new CemNativeRecoveryScope();
        const resource = await QuerySourceResource.open({ endpoint: old.endpoint(), bytes, pins,
            current: () => !privateInput, target: fresh => scenario === 'foreign-target' ? old.endpoint() : fresh.endpoint(), onBound: endpoint => { bound = endpoint; },
            receiverCheckpoint: mutable ? () => journal.retain() : undefined });
        let admission, selected, originalIdentity, stores = [];
        try {
            if (coreDirectory && scenario === 'mutable-resume') {
                selected = await openSelectedConsumer();
                originalIdentity = old.identity();
                assert.notEqual(selected.memory, originalIdentity);
                selected.verify(await old.call('projection'), '<p>Tea/3/ok/native-one</p>');
            }
            const original = comparableReport(await old.call('validate', ['same']));
            const originalFrame = await old.call('frame');
            assert.equal(original, 'true:true:true');
            assert.equal(await old.call('tokens'),true);
            if(scenario==='expired-owner'){await old.call('probeApplicationRetire');await assert.rejects(async()=>old.call('validate'));await assert.rejects(async()=>old.call('frame'));await assert.rejects(resource.prepareRecovery());continue;}
            const census = JSON.parse(await old.call('commandNativeResourceInventory'));
            assert.equal(census.registries.find(r => r.name === 'enrolled-compilations').live, 1);
            const unknown = await checkpoint(old, 'unknown'); stores.push(unknown);
            assert.throws(() => unknown.handoff(), /operations.unknown/);
            privateInput = true;
            await assert.rejects(resource.prepareRecovery(), /policy/);
            assert.throws(() => new CemNativeGenerationGroup([old.participant()]), /stale/);
            assert.equal(allocations, before);
            privateInput = false;
            const edit = async name => old.call('edit', [bytes, pins, new Uint8Array(await readFile(resolve(directory, `installed.${name}.inputs`)))]);
            if (mutable) await edit(scenario === 'mutable-private' ? 'private' : 'edited');
            if (scenario === 'mutable-private') {
                await assert.rejects(resource.prepareRecovery(), /refused|native-refusal/);
                assert.equal(allocations, before); continue;
            }
            const editedHtml = '<p>Coffee/4/ok/native-one</p>';
            if (mutable) await old.call('frame', [editedHtml]);
            if (selected) selected.verify(await old.call('projection'), editedHtml);
            const styles = await resource.stylesheets('cem-count');
            assert.equal(styles.receipt, '[]');
            assert.equal(styles.isCurrent(), true);
            const revision = await resource.prepareRecovery();
            if (mutable) assert.match(revision, /^cem-bin/);
            let writes = 0;
            const deliverWrite = () => writes++;
            if (mutable) deliverWrite(); // The original terminal write completed once.
            const workspaceLimit = 4 * 1024 * 1024;
            const deploymentBytes = bytes.length + new TextEncoder().encode(pins).length;
            const recoveryPinReserve = 16 * 1024;
            let pad = scenario === 'aggregate-budget' ? workspaceLimit - 4096 : 0;
            if (scenario === 'mutable-aggregate-budget') {
                const base = await checkpoint(old, 'acknowledged', 1, revision); stores.push(base);
                pad = workspaceLimit - deploymentBytes - base.current().bytes.length - recoveryPinReserve - 64;
            }
            const store = await checkpoint(old, 'acknowledged', pad, revision); stores.push(store);
            if (scenario === 'mutable-aggregate-budget') {
                assert.ok(deploymentBytes + store.current().bytes.length + recoveryPinReserve <= workspaceLimit);
            }
            const handoff = store.handoff();
            journal = handoff;
            if (scenario === 'mutable-modified-journal') {
                const alternate = await checkpoint(old, 'not-applied', 0, revision, 'foreign-operation'); stores.push(alternate);
                journal = alternate.handoff();
            }
            const ticket = issueNativeGenerationBindingTicket(old.participant(), () => handoff.current() && !privateInput,
                async context => ({ owner: context.generation.runtime, current: () => !context.signal.aborted && handoff.current() && !privateInput,
                    bind: () => { assert.equal(context.active(), true); }, discard() {} }));
            scope.enroll({ id: old.id, kind: 'command-invocation', prepare: async () => ({ participants: [old.participant()], current: () => handoff.current() && !privateInput }) });
            const coverage = await scope.prepare();
            const group = coverage.createGenerationGroup();
            if (scenario === 'mutable-stale-input') await edit('later');
            if (['mutable-stale-input', 'mutable-foreign-bytes'].includes(scenario)) {
                await assert.rejects(old.retire(handoff, () => !privateInput), /refused|native-refusal/);
                assert.equal(allocations, before); assert.equal(bound, undefined); continue;
            }
            const retirement = await old.retire(handoff, () => !privateInput);
            assert.equal(styles.signal.aborted, true);
            assert.equal(styles.isCurrent(), false);
            if (scenario.endsWith('aggregate-budget')) {
                await assert.rejects(group.allocate([retirement]), /limit/);
                assert.equal(allocations, before); continue;
            }
            if (scenario === 'stale-checkpoint') {
                store.invalidate(); await assert.rejects(group.allocate([retirement]), /stale/);
                assert.equal(allocations, before); assert.equal(bound, undefined); continue;
            }
            await assert.rejects(group.allocate([{ ...retirement }]), /receipt/);
            retirement.checkpoint.fill(0); // Exposed bytes cannot rewrite private checkpoint/journal state.
            admission = await group.allocate([retirement]);
            if (scenario === 'entered-after-allocation') privateInput = true;
            if (!['resume', 'mutable-resume'].includes(scenario)) {
                await assert.rejects(resumeNativeGenerationBindings(admission, [ticket]), /stale|target|refused|native-refusal/);
                assert.equal(bound, undefined); continue;
            }
            await resumeNativeGenerationBindings(admission, [ticket]);
            const fresh = admission.generations[0].runtime;
            assert.equal(bound, fresh.endpoint());
            if (selected) {
                assert.notEqual(selected.memory, fresh.identity());
                assert.notEqual(originalIdentity, fresh.identity());
                selected.verify(await fresh.call('projection'), editedHtml);
                console.log(`Selected core / optional source projection: ${mode} handoff passed.`);
            }
            const replacementStyles = await resource.stylesheets('cem-count');
            assert.equal(replacementStyles.receipt, '[]');
            assert.equal(replacementStyles.isCurrent(), true);
            assert.notEqual(replacementStyles.signal, styles.signal);
            assert.deepEqual(comparableReport(await fresh.call('validate', ['same'])), original);
            assert.deepEqual(await fresh.call('frame', mutable ? [editedHtml] : []), mutable ? [...originalFrame.slice(0, 2), editedHtml] : originalFrame);
            if (mutable) {
                for (const [sequence, id] of [[1, 'implicit-replay'], [2, 'terminal-operation'], [2, 'new-operation'], [3, 'new-operation']]) {
                    if (await fresh.call('claimWrite', [sequence, id])) deliverWrite();
                }
                assert.equal(writes, 2, 'the original operation replayed or the fresh operation was lost');
            }
            assert.equal(JSON.parse(new TextDecoder().decode(admission.generations[0].checkpoint)).receipt.operations[0].outcome, 'acknowledged');
            const freshInventory = JSON.parse(await fresh.call('commandNativeResourceInventory'));
            assert.equal(freshInventory.registries.find(r => r.name === 'enrolled-preparations').live, 0);
            await assert.rejects(old.endpoint().open(bytes, pins), /retired|closed|stale/);
            await resource.dispose();
            assert.equal(JSON.parse(await fresh.call('commandNativeResourceInventory')).blockingResources, 2, 'original deployment owner disappeared from the census');
            await fresh.call('probeApplicationUnprovision');
            assert.equal(JSON.parse(await fresh.call('commandNativeResourceInventory')).blockingResources, 0);
            console.log(`Application resource ${variant}/${mode}: native-sealed journal, full v2 census, original release, physical retirement and fresh source rebinding passed.`);
        } finally {
            scenarios++;
            try { selected?.dispose(); }
            finally { await resource.dispose(); admission?.revoke(); scope.dispose(); await old.discard(); for (const store of stores) store.dispose(); }
        }
    }
    }
    assert.equal(scenarios, 24);
    console.log('All 24 physical application handoff scenarios passed, including edited receivers, consumed writes, stale edits, foreign bytes, modified journals and aggregate limits.');

    for (const mode of ['fallback', 'worker']) for (const scenario of ['repeat', 'release-during-renew', 'credentials-during-renew']) {
        const original = await physical(mode, scenario);
        const generations = [original], admissions = [], stores = [];
        let previous = original, journal, bound, allowed = true;
        const resource = await QuerySourceResource.open({
            endpoint: original.endpoint(), bytes, pins, current: () => allowed, target: fresh => fresh.endpoint(),
            receiverCheckpoint: () => journal.retain(), onBound: endpoint => { bound = endpoint; },
        });
        try {
            const edited = new Uint8Array(await readFile(resolve(directory, 'installed.edited.inputs')));
            await original.call('edit', [bytes, pins, edited]);
            for (let cycle = 0; cycle < (scenario === 'repeat' ? 2 : 1); cycle++) {
                const revision = await resource.prepareRecovery();
                const store = await checkpoint(previous, 'acknowledged', 0, revision);
                stores.push(store);
                journal = store.handoff();
                let rendered = false, renewed = false, discarded = false;
                const ticket = issueCheckpointResumeTicket(previous.id, store.current(), journal.current, new Map([['source-route', {
                    witness: () => journal.current,
                    async prepare(context) {
                        const fresh = context.runtime;
                        assert.equal(context.active(), false);
                        await assert.rejects(resource.stylesheets('cem-count'), /owner|stale/);
                        return {
                            owner: { resume: context.renewOwners }, current: journal.current,
                            bind: () => { assert.equal(context.active(), true); },
                            async renew() {
                                assert.equal(bound, fresh.endpoint());
                                assert.equal((await resource.stylesheets('cem-count')).receipt, '[]');
                                const frame = await fresh.call('frame', ['<p>Coffee/4/ok/native-one</p>']);
                                assert.match(frame[2], /Coffee\/4/);
                                rendered = true;
                                if (scenario === 'release-during-renew') await resource.dispose();
                                if (scenario === 'credentials-during-renew') allowed = false;
                            },
                            renewed: () => { renewed = true; },
                            discard: () => { discarded = true; },
                        };
                    },
                }]]));
                const group = new CemNativeGenerationGroup([previous.participant()]);
                const retired = await previous.retire(journal, () => allowed);
                const admission = await group.allocate([retired]);
                admissions.push(admission);
                const fresh = admission.generations[0].runtime;
                generations.push(fresh);
                const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
                const resume = resumed.owners[0].owner.resume();
                if (scenario !== 'repeat') {
                    await assert.rejects(resume, /source-stale/);
                    assert.equal(rendered, true);
                    assert.equal(renewed, false);
                    assert.equal(discarded, true);
                    assert.equal(fresh.alive(), false);
                    break;
                }
                await resume;
                assert.equal(rendered && renewed, true);
                if (cycle) {
                    admissions[cycle - 1].revoke();
                    assert.equal((await resource.stylesheets('cem-count')).isCurrent(), true,
                        'Revoking a predecessor discarded the newer native source');
                }
                previous = fresh;
            }
        } finally {
            await resource.dispose();
            for (const admission of admissions.reverse()) admission.revoke();
            await Promise.all(generations.map(generation => generation.discard()));
            for (const store of stores) store.dispose();
        }
        console.log(`Application semantic handoff ${mode}/${scenario} passed.`);
    }
}
