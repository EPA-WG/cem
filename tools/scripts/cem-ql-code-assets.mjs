import { createHash } from 'node:crypto';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
export function codeInterface(module) {
    return JSON.stringify({
        imports: WebAssembly.Module.imports(module).map(entry => [entry.module, entry.name, entry.kind]).sort(),
        exports: WebAssembly.Module.exports(module).map(entry => [entry.name, entry.kind]).sort(),
    });
}

/** Build-owned runtime pins; these are never runtime capability grants. */
export function cemQlCodeAssets(bytes, bindings, profile = 'full-default-debug-control') {
    if (!['full-default-debug-control', 'cem-ml-optional-schema-assets'].includes(profile)) throw Error('unsupported native code profile');
    const identity = { protocol: 'cem-ql-code/1', wasmSha256: hash(bytes), sizeBytes: bytes.length, bindingsSha256: hash(bindings),
        interfaceSha256: hash(codeInterface(new WebAssembly.Module(bytes))), abi: 'cem-owned-wasm-interface/1', profile };
    const js = `// Generated from the paired native runtime. Immutable code only, never instances or memories.
export const CEM_QL_CODE_IDENTITY = Object.freeze(${JSON.stringify(identity)});
let pending, installing=false;
export function assertCemQlCodeIdentity(value) {
    if(!value || typeof value!=='object' || Object.keys(value).length!==Object.keys(CEM_QL_CODE_IDENTITY).length)
        throw Error('cem.runtime.code.identity');
    for(const [key,expected] of Object.entries(CEM_QL_CODE_IDENTITY)) {
        const field=Object.getOwnPropertyDescriptor(value,key);
        if(!field || !('value' in field) || field.value!==expected) throw Error('cem.runtime.code.identity');
    }
}
const digest=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),byte=>byte.toString(16).padStart(2,'0')).join('');
function descriptor(module) {
    return JSON.stringify({imports:WebAssembly.Module.imports(module).map(entry=>[entry.module,entry.name,entry.kind]).sort(),
        exports:WebAssembly.Module.exports(module).map(entry=>[entry.name,entry.kind]).sort()});
}
async function verify(module) {
    if(!(module instanceof WebAssembly.Module) || await digest(new TextEncoder().encode(descriptor(module)))!==CEM_QL_CODE_IDENTITY.interfaceSha256)
        throw Error('cem.runtime.code.interface');
    return module;
}
export function loadCemQlWasmModule() {
    if(installing) return Promise.reject(Error('cem.runtime.code.installing'));
    if(!globalThis.crypto?.subtle) return Promise.reject(Error('cem.runtime.code.integrity-unavailable'));
    return pending??=fetch(new URL('cem_ql_bg.wasm',import.meta.url)).then(async response=>{
        if(!response.ok) throw Error('cem.runtime.code.load');
        const reader=response.body?.getReader();
        if(!reader) throw Error('cem.runtime.code.size');
        const bytes=new Uint8Array(CEM_QL_CODE_IDENTITY.sizeBytes);let offset=0;
        try { while(true) {
            const chunk=await reader.read();if(chunk.done)break;
            if(offset+chunk.value.byteLength>bytes.length) throw Error('cem.runtime.code.size');
            bytes.set(chunk.value,offset);offset+=chunk.value.byteLength;
        }} catch(error) { await reader.cancel().catch(()=>{});throw error; }
        finally { reader.releaseLock(); }
        if(offset!==bytes.length) throw Error('cem.runtime.code.size');
        if(await digest(bytes)!==CEM_QL_CODE_IDENTITY.wasmSha256) throw Error('cem.runtime.code.hash');
        return verify(await WebAssembly.compile(bytes));
    }).catch(error=>{pending=undefined;throw error;});
}
// Trusted original-host bootstrap. The sender verified the bytes before compiling;
// receivers check their own build pins/interface, not a fictitious hash of a Module.
export async function installCemQlWasmModule(module,identity) {
    assertCemQlCodeIdentity(identity);
    if(pending || installing) throw Error('cem.runtime.code.already-selected');
    installing=true;
    try { pending=Promise.resolve(await verify(module)); }
    finally { installing=false; }
}
`;
    const dts = `export interface CemQlCodeIdentity { readonly protocol: 'cem-ql-code/1'; readonly wasmSha256: string; readonly sizeBytes: number; readonly bindingsSha256: string; readonly interfaceSha256: string; readonly abi: string; readonly profile: string; }
export declare const CEM_QL_CODE_IDENTITY: Readonly<CemQlCodeIdentity>;
export declare function assertCemQlCodeIdentity(value: unknown): asserts value is CemQlCodeIdentity;
export declare function loadCemQlWasmModule(): Promise<WebAssembly.Module>;
export declare function installCemQlWasmModule(module: WebAssembly.Module, identity: unknown): Promise<void>;
`;
    return { js, dts, identity };
}
