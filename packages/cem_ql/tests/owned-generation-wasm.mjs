import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as legacy from '../dist/wasm/cem_ql.js';
import { createCemQlWasmGeneration } from '../dist/wasm/cem_ql_factory.js';
const bytes = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const first = createCemQlWasmGeneration(), second = createCemQlWasmGeneration();
const initial = first.initialize(bytes);
assert.throws(() => first.dispose(), /unavailable/);
await Promise.all([initial, first.initialize(bytes), second.initialize(bytes)]);
await legacy.default({ module_or_path: bytes });
assert.notEqual(first.memory(), second.memory());
assert.notEqual(first.memory(), legacy.commandNativeMemoryIdentity());
const compile = runtime => JSON.parse(runtime.call('compileTemplate', ['{p | Owned source}', '[]'])).artifactId;
const a = compile(first), b = compile(second), raw = JSON.parse(legacy.compileTemplate('{p | Legacy source}', '[]')).artifactId;
const source = first.call('parseReferenceSource', [new TextEncoder().encode('<original/>'), 'application/xml', 'memory:original-source', '{"maxBytes":262144,"maxNodes":4096}']);
const census = runtime => JSON.parse(runtime.call('commandNativeResourceInventory', []));
const originalCensus = census(first);
assert.equal(originalCensus.schemaVersion, 2);
assert.equal(originalCensus.profile, 'cem-ml-cem-ql-generation-v2');
assert.equal(originalCensus.registries.length, 36);
assert.equal(originalCensus.registries.filter(entry => entry.name.startsWith('enrolled-')).length, 6);
assert.ok(a && b && raw);
assert.throws(() => first.dispose(), /resources|inventory|retirement/);
// A refused disposal leaves cleanup available under permanently closed admission.
assert.equal(first.call('disposeTemplate', [a]), true);
assert.equal(census(first).registries.find(entry => entry.name === 'reference-sources').live, 1);
assert.throws(() => first.prepareRetirement(census(first).revision), /resources|drained|retirement/);
assert.equal(first.call('disposeReferenceSource', [source]), true);
const staleCensus = census(first);
first.call('disposeTemplate', [0]); // Count-preserving native cleanup invalidates the old census.
assert.throws(() => first.prepareRetirement(staleCensus.revision), /inventory-stale/);
first.prepareRetirement(census(first).revision);
assert.throws(() => first.dispose(() => { first.abortRetirement(); return true; }), /retirement-pending/);
assert.ok(first.memory(), 'reentrant abort must preserve the original physical memory');
const savedAlias = (...args) => first.call('runtimeMemoryStatus', args);
assert.throws(() => first.dispose(() => false), /ownership-stale/);
assert.ok(first.memory());
assert.equal(JSON.parse(first.call('commandNativeResourceInventory', [])).registries.find(entry => entry.name === 'native-entries').live, 0);
const confirmation = first.dispose();
assert.equal(confirmation.frozen, true); assert.equal(confirmation.replacementAllowed, false);
assert.equal(first.memory(), undefined);
assert.throws(() => savedAlias(), /retired/);
await assert.rejects(first.initialize(bytes), /retired/);
assert.throws(() => first.call('initSync', [bytes]), /retired/);
assert.equal(second.call('disposeTemplate', [b]), true);
assert.equal(legacy.disposeTemplate(raw), true);
assert.ok(compile(second));
assert.ok(JSON.parse(legacy.compileTemplate('{p | Still legacy}', '[]')).artifactId);
// A later independent factory cannot retarget old function aliases.
const third = createCemQlWasmGeneration(); await third.initialize(bytes);
assert.notEqual(third.memory(), second.memory()); assert.throws(savedAlias, /retired/);
third.dispose();
const failed = createCemQlWasmGeneration();
await assert.rejects(failed.initialize(new Uint8Array()), /WebAssembly|wasm|buffer/);
assert.equal(failed.memory(), undefined); await assert.rejects(failed.initialize(bytes), /retired/);
if (global.gc) { global.gc(); await new Promise(resolve => setTimeout(resolve, 20)); }
console.log('Owned WASM generations: distinct memories, native-root refusal, closed cleanup, private reference release, stale alias/initializer fencing and legacy independence passed.');

// Default code is shared; mutable heaps and cancellation are always per owner.
const fetchOriginal = globalThis.fetch;
let loads = 0, respond;
globalThis.fetch = async () => { loads++; return new Promise(resolve => { respond = () => resolve(new Response(bytes, { headers: { 'content-type': 'application/wasm' } })); }); };
try {
    const abandoned = createCemQlWasmGeneration(), cancelled = new AbortController();
    const pending = abandoned.initialize(undefined, cancelled.signal);
    const survivor = createCemQlWasmGeneration(), ready = survivor.initialize();
    cancelled.abort(new Error('cancelled fixture'));
    await assert.rejects(pending, /cancelled fixture/);
    respond(); await ready;
    assert.equal(abandoned.memory(), undefined);
    await assert.rejects(abandoned.initialize(), /retired/);
    const renewed = createCemQlWasmGeneration(); await renewed.initialize();
    assert.equal(loads, 1); assert.notEqual(survivor.memory(), renewed.memory());
    const weak = new WeakRef(survivor.memory()); survivor.dispose();
    if (global.gc) {
        for (let n = 0; n < 20; n++) { await new Promise(resolve => setTimeout(resolve, 10)); global.gc(); }
        assert.equal(weak.deref(), undefined, 'immutable module cache retained a disposed memory');
    }
    renewed.dispose();
} finally { globalThis.fetch = fetchOriginal; }
console.log('Default compiled module reuse: single load, distinct memories, independent cancellation and disposed-memory collection passed.');

const retryFactory = await import('../dist/wasm/cem_ql_factory.js?default-load-failure');
let attempts = 0;
globalThis.fetch = async () => ++attempts === 1 ? new Response('', { status: 503 }) : new Response(bytes, { headers: { 'content-type': 'application/wasm' } });
try {
    const rejected = retryFactory.createCemQlWasmGeneration();
    await assert.rejects(rejected.initialize(), /generation.load/);
    await assert.rejects(rejected.initialize(), /retired/);
    const retry = retryFactory.createCemQlWasmGeneration(); await retry.initialize();
    assert.equal(attempts, 2); retry.dispose();
} finally { globalThis.fetch = fetchOriginal; }

const pressured = createCemQlWasmGeneration(); await pressured.initialize(bytes);
const pressureRoot = compile(pressured);
pressured.call('configureRuntimeMemoryBudget', [JSON.stringify({ softLimitBytes: 1, hardLimitBytes: 65536,
    recoveryHeadroomBytes: 32768, maxConcurrentOperations: 1 })]);
assert.equal(JSON.parse(pressured.call('runtimeMemoryStatus', [])).hardPressure, true);
assert.ok(JSON.parse(pressured.call('compileTemplate', ['{p | refused}', '[]'])).diagnostics.some(entry => entry.code === 'cem.runtime.memory.admission'));
assert.throws(() => pressured.dispose(), /resources|drained|retirement|generation/);
assert.equal(pressured.call('disposeTemplate', [pressureRoot]), true);
assert.equal(pressured.dispose().replacementAllowed, false);
assert.equal(pressured.memory(), undefined);
console.log('Joined v2 census, native source roots, stale cleanup/reentrant refusal and hard-pressure retirement passed.');
