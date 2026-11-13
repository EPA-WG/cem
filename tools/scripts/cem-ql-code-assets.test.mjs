import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { cemQlCodeAssets } from './cem-ql-code-assets.mjs';

const bytes=Uint8Array.from([0,97,115,109,1,0,0,0,5,3,1,0,1,7,10,1,6,109,101,109,111,114,121,2,0]);
async function fixture(run) {
    const directory=await mkdtemp(join(tmpdir(),'cem-code-'));
    const assets=cemQlCodeAssets(bytes,'original bindings');
    await writeFile(join(directory,'code.mjs'),assets.js);
    const code=await import(pathToFileURL(join(directory,'code.mjs')));
    const fetch=globalThis.fetch;
    try { await run(code,assets); } finally {globalThis.fetch=fetch;await rm(directory,{recursive:true,force:true});}
}
test('verified code loads once under concurrent demand; a failed read/hash does not poison retry', async()=>fixture(async code=>{
    let reads=0;
    globalThis.fetch=async()=>{reads++;return new Response(bytes,{status:200});};
    const [a,b]=await Promise.all([code.loadCemQlWasmModule(),code.loadCemQlWasmModule()]);
    assert.equal(a,b);assert.equal(reads,1);
    const first=new WebAssembly.Instance(a),second=new WebAssembly.Instance(b);
    assert.notEqual(first.exports.memory,second.exports.memory);
}));
test('failed acquisition resets only the immutable code promise and refuses edited bytes',async()=>fixture(async code=>{
    let reads=0;
    globalThis.fetch=async()=>{reads++;return reads===1?new Response('unavailable',{status:503}):new Response(bytes);};
    await assert.rejects(code.loadCemQlWasmModule(),/code.load/);
    await code.loadCemQlWasmModule();assert.equal(reads,2);
}));
test('identity, interface and duplicate installs refuse before a supplied module can be selected',async()=>fixture(async(code,{identity})=>{
    const module=new WebAssembly.Module(bytes);
    await assert.rejects(code.installCemQlWasmModule(module,{...identity,profile:'foreign'}),/code.identity/);
    let inspected=0;const accessor={...identity};Object.defineProperty(accessor,'wasmSha256',{get(){inspected++;return identity.wasmSha256;}});
    await assert.rejects(code.installCemQlWasmModule(module,accessor),/code.identity/);assert.equal(inspected,0);
    await assert.rejects(code.installCemQlWasmModule(new WebAssembly.Module(bytes.slice(0,8)),identity),/code.interface/);
    const installation=code.installCemQlWasmModule(module,identity);
    await assert.rejects(code.installCemQlWasmModule(module,identity),/already-selected/);
    await installation;
    assert.equal(await code.loadCemQlWasmModule(),module);
    await assert.rejects(code.installCemQlWasmModule(module,identity),/already-selected/);
}));
test('corrupt bytes cannot enter compilation despite a successful HTTP response',async()=>fixture(async code=>{
    const changed=bytes.slice();changed[changed.length-1]^=1;
    globalThis.fetch=async()=>new Response(changed);
    await assert.rejects(code.loadCemQlWasmModule(),/code.hash/);
    globalThis.fetch=async()=>new Response(bytes.slice(0,8));
    await assert.rejects(code.loadCemQlWasmModule(),/code.size/);
    globalThis.fetch=async()=>new Response(new Uint8Array(bytes.length+1));
    await assert.rejects(code.loadCemQlWasmModule(),/code.size/);
    globalThis.fetch=async()=>new Response(bytes);
    assert(await code.loadCemQlWasmModule() instanceof WebAssembly.Module);
}));
