import assert from 'node:assert/strict';
import test from 'node:test';
import { createServer } from 'node:http';
import { chromium } from 'playwright';
import { traceWasmAcquisition, nativeHeapInspection } from './wasm-acquisition-trace.mjs';

test('diagnostic attachment observes original page/worker fetches and repeated inspection does not acquire code', async () => {
    const binary = Buffer.from([0,97,115,109,1,0,0,0,5,3,1,0,1,7,10,1,6,109,101,109,111,114,121,2,0]);
    const script = `globalThis.ready=fetch('/core.wasm').then(response=>response.arrayBuffer()).then(WebAssembly.instantiate).then(({instance})=>{globalThis.native=instance;return true;});`;
    const server = createServer((request,response) => {
        response.setHeader('Cache-Control','no-store');
        if(request.url==='/core.wasm') response.writeHead(200,{'Content-Type':'application/wasm'}).end(binary);
        else if(request.url==='/worker.js') response.writeHead(200,{'Content-Type':'text/javascript'}).end(script+'ready.then(()=>postMessage(true));');
        else response.writeHead(200,{'Content-Type':'text/html'}).end(`<script>${script}globalThis.workerReady=new Promise(resolve=>{const worker=new Worker('/worker.js');worker.onmessage=()=>resolve(true);});</script>`);
    });
    await new Promise(done=>server.listen(0,'127.0.0.1',done));
    const browser = await chromium.launch({headless:true,executablePath:process.env.CEM_STUDIO_CHROME});
    try {
        const context=await browser.newContext(), page=await context.newPage(), cdp=await context.newCDPSession(page);
        const trace=await traceWasmAcquisition(cdp);
        await page.goto(`http://127.0.0.1:${server.address().port}/`);
        await page.evaluate(()=>Promise.all([ready,workerReady]));
        assert.equal(page.workers().length,1);
        const expression='({capacity:native.exports.memory.buffer.byteLength,kind:Object.prototype.toString.call(native.exports.memory.buffer)})';
        assert.equal((await trace.inspectWorkers(page, expression))[0].value.capacity,65536);
        assert.deepEqual(trace.requests.map(request=>request.realm).sort(),['page','worker']);
        assert(trace.requests.find(request=>request.realm==='page').initiator.stack.callFrames.length>0);
        assert.equal(trace.requests.find(request=>request.realm==='worker').initiator.stack,null);
        assert.equal((await trace.inspectWorkers(page, expression))[0].value.kind,'[object ArrayBuffer]');
        assert.equal((await trace.finish()).length,2);
        await cdp.detach();
        assert.equal(page.workers().length,1,'debugger detachment must not retire the worker');
        await context.close();
    } finally {await browser.close();server.closeAllConnections();await new Promise(done=>server.close(done));}
});

test('native heap inspection refuses uninitialized exports and reuses an already owned heap', async () => {
    const source=`let native, calls=0;
        export function cemQlVersion(){if(!native)throw Error('not ready');return 'fixture';}
        export default async function initialize(){calls++;return native??={memory:new WebAssembly.Memory({initial:1})};}
        export function prepare(){native={memory:new WebAssembly.Memory({initial:1})};}
        export function invocations(){return calls;}`;
    const server=createServer((request,response)=> {
        const body=request.url.endsWith('/cem_ql.js')?source:request.url.endsWith('/native-runtime.js')?'export function runtimeMemoryStatus(){return "{}";} export function commandNativeResourceInventory(){return "{}";}':'<!doctype html>';
        response.writeHead(200,{'Content-Type':request.url.endsWith('.js')?'text/javascript':'text/html'}).end(body);
    });
    await new Promise(done=>server.listen(0,'127.0.0.1',done));
    const browser=await chromium.launch({headless:true,executablePath:process.env.CEM_STUDIO_CHROME});
    try {
        const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}/`);
        const refused=await page.evaluate(nativeHeapInspection);
        assert.equal(refused.status,'unavailable');
        assert.equal(await page.evaluate(async()=> (await import('/packages/cem_ql/dist/wasm/cem_ql.js')).invocations()),0);
        await page.evaluate(async()=> (await import('/packages/cem_ql/dist/wasm/cem_ql.js')).prepare());
        const admitted=await page.evaluate(nativeHeapInspection);
        assert.equal(admitted.cachedInstance,true);
        assert.equal(admitted.bufferKind,'[object ArrayBuffer]');
        assert.equal(admitted.linearMemoryBytes,65536);
        assert.equal((await page.evaluate(nativeHeapInspection)).cachedInstance,true);
    } finally {await browser.close();server.closeAllConnections();await new Promise(done=>server.close(done));}
});
