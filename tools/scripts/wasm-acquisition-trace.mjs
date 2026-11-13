// Diagnostic-only CDP attachment: never enable this during latency measurements.
import assert from 'node:assert/strict';

export const nativeHeapInspection = `(async () => {
    const bindings = await import('/packages/cem_ql/dist/wasm/cem_ql.js');
    let version;
    try { version=bindings.cemQlVersion(); }
    catch { return {status:'unavailable',reason:'native exports are not ready; inspection did not initialize them'}; }
    const first = await bindings.default(), second = await bindings.default();
    const native = await import('/packages/cem-elements/dist/lib/internal/runtime-support/native-runtime.js');
    return {status:'available',version,cachedInstance:first===second && first.memory===second.memory,
        linearMemoryBytes:first.memory.buffer.byteLength,bufferKind:Object.prototype.toString.call(first.memory.buffer),
        memoryStatus:JSON.parse(native.runtimeMemoryStatus()),
        inventory:JSON.parse(native.commandNativeResourceInventory())};
})()`;

export async function traceWasmAcquisition(cdp) {
    const requests = [], workerRequests = new Map();
    cdp.on('Network.requestWillBeSent', data => {
        if (!new URL(data.request.url).pathname.endsWith('.wasm')) return;
        requests.push({ realm:'page', target:'page', requestId:data.requestId, url:data.request.url,
            evidence:'CDP Network.requestWillBeSent', initiator:data.initiator });
    });
    await cdp.send('Network.enable');
    await cdp.send('Debugger.enable');
    await cdp.send('Debugger.setAsyncCallStackDepth',{maxDepth:32});
    return {
        get requests() { return [...requests,...[...workerRequests.values()].flat()]; },
        async inspectWorkers(page, expression) {
            return Promise.all(page.workers().map(async worker => {
                const resources = () => worker.evaluate(() => performance.getEntriesByType('resource')
                    .filter(entry => new URL(entry.name).pathname.endsWith('.wasm'))
                    .map(entry => ({url:entry.name,startTime:entry.startTime,initiatorType:entry.initiatorType,
                        decodedBodyBytes:entry.decodedBodySize,transferBytes:entry.transferSize})));
                const before = await resources();
                const value = await worker.evaluate(expression);
                assert.deepEqual(await resources(),before,'worker inspection caused WASM acquisition');
                workerRequests.set(worker,before.map(resource=>({realm:'worker',target:worker.url(),...resource,
                    evidence:'original worker PerformanceResourceTiming',
                    initiator:{type:resource.initiatorType,stack:null,reason:'worker request stack unavailable through this diagnostic endpoint'}})));
                return {target:worker.url(),value};
            }));
        },
        finish() { return this.requests; },
    };
}
