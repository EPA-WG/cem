import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';

const output = resolve(isMainThread ? process.argv[2] : workerData.output);
const binary = await readFile(resolve(output, 'application_source_probe_bg.wasm'));
const source = await readFile(resolve(output, 'application.cemah'));
const pins = await readFile(resolve(output, 'application.pin.json'), 'utf8');

async function verify(heap) {
    const url = pathToFileURL(resolve(output, 'application_source_probe.js'));
    url.searchParams.set('heap', heap);
    const reader = await import(url.href);
    const wasm = await reader.default({ module_or_path: binary });
    const read = (bytes = source, pin = pins, cancelled = false) => reader.probeApplicationSources(bytes, pin, cancelled);
    assert.equal(read(), '3:5:1:2');
    assert.throws(() => read(source, pins, true));
    assert.throws(() => read(source.subarray(0, 10)));
    assert.throws(() => read(new Uint8Array(4 * 1024 * 1024 + 1)));
    assert.throws(() => read(source, '{'));
    const changed = JSON.parse(pins);
    changed.application.registry_hash = changed.hash;
    assert.throws(() => read(source, JSON.stringify(changed)));
    changed.hash = 'foreign';
    assert.throws(() => read(source, JSON.stringify(changed)));
    assert.equal(read(), '3:5:1:2', 'refusal must release temporary sources and leave reader usable');
    assert.equal(typeof reader.applicationSourceOpenV1, 'function');
    assert.throws(() => reader.applicationSourceOpenV1(source, pins), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceCaptureV1(Object.freeze({})), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceReleaseV1(Object.freeze({})), /application.endpoint.refused/);

    return wasm.memory;
}

if (isMainThread) {
    const first = await verify('first');
    const fresh = await verify('fresh');
    assert.notEqual(first, fresh, 'fresh native source trees belong to separate heaps');
    const worker = new Worker(new URL(import.meta.url), { workerData: { output } });
    await new Promise((resolve, reject) => {
        worker.once('error', reject);
        worker.once('exit', code => code === 0 ? resolve() : reject(Error(`reader worker exited ${code}`)));
    });
    console.log(`Application source reader: 36 checks across two fresh heaps and a real worker; ${source.length} source bytes. Native provisioning is required; worker/fallback installation acceptance remains gated.`);
} else {
    await verify('worker');
    parentPort.close();
}
