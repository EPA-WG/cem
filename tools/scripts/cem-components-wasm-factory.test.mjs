import { test } from 'node:test';
import assert from 'node:assert/strict';
import { componentWasmFactory } from './cem-components-wasm-factory.mjs';
const glue=`let wasmModule, wasmInstance, wasm;
let cachedUint8ArrayMemory0 = null;
export class CompiledDeclarationOwner {}
export function renderCompiledApplication() {}
export function compiledDeclarationInventory() {}
const finalizer = new FinalizationRegistry(ptr => wasm.__wbg_compileddeclarationowner_free(ptr, 1));
function initSync() {}
function __wbg_init() {}
export { initSync, __wbg_init as default };`;
test('isolates the pinned component ABI and fences finalizers after retirement',()=>{
    const output=componentWasmFactory(glue);
    assert.match(output,/export function createCemComponentWasmBindings/);
    assert.match(output,/wasm === undefined \? undefined : wasm\.__wbg_compileddeclarationowner_free/);
    assert.match(output,/await __wbg_init/);
    assert.match(output,/census\.liveDeclarationOwners !== 0/);
    assert.match(output,/wasmModule = wasmInstance = wasm = undefined/);
    assert.doesNotMatch(output,/export class CompiledDeclarationOwner/);
});
test('refuses unknown exports, imports, finalizers and memory-cache shapes',()=>{
    for(const source of [glue+'\nexport function compileTemplate() {}', 'import x from "./shared.js";\n'+glue,
        glue.replace('wasm.__wbg_compileddeclarationowner_free(ptr, 1)','unsafe(ptr)'),
        glue+'\nlet cachedDataViewMemory0 = null;', glue.replace('CompiledDeclarationOwner {}','OtherOwner {}')]) {
        assert.throws(()=>componentWasmFactory(source));
    }
});
