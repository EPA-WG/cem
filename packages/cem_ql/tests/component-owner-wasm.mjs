import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createCemComponentWasmBindings} from '../dist/wasm-component/cem_ql_factory.js';
import {createCemComponentGeneration} from '../../cem-elements/dist/lib/internal/runtime-support/component-generation.js';
const module=await WebAssembly.compile(await readFile(new URL('../dist/wasm-component/cem_ql_bg.wasm',import.meta.url)));
const root=new URL('../../../dist/artifacts/cem_ql/component-runtime/',import.meta.url);
const context={identity:'owned',resolverIdentity:'resolver',resourcePolicyStamp:'policy',frames:[{frameId:'root',baseUrl:'https://app.test/components/',scopes:[],specifiers:{imports:{},resources:{}}}]};
const fixture=async name=>[await readFile(new URL(`${name}.bin`,root)),await readFile(new URL(`${name}.json`,root),'utf8'),await readFile(new URL(`${name}.hash`,root),'utf8')];
const inline=await fixture('owned-inline'), imported=await fixture('owned-imports');
const bindings=createCemComponentWasmBindings(), first=await createCemComponentGeneration(module,bindings);
const next=await createCemComponentGeneration(module,createCemComponentWasmBindings());
assert.notEqual(first.memory(),next.memory());
for (const generation of [first,next]) {
    assert.equal(generation.inventory().liveDeclarationOwners,0);
    assert.throws(()=>generation.open(inline[0],inline[1],''));
    assert.equal(generation.inventory().liveDeclarationOwners,0);
    for(const data of [inline,imported]) {
        const declaration=generation.open(...data);
        assert.equal(generation.inventory().liveDeclarationOwners,1);
        const metadata=declaration.metadata();metadata.tag='changed-by-caller';assert.equal(declaration.metadata().tag,'cem-owned-proof');
        assert.match(declaration.renderJson(Buffer.from('{"value":"Tea <img>"}'),'app://cart/response.json','datadom'),/Tea &lt;img&gt;/);
        const controller=new AbortController(); let committed;
        const lease={signal:controller.signal,commit:outputs=>{committed=outputs;return true;},release:()=>controller.abort()};
        const installation=declaration.installStylesheets('client',context,lease);
        assert.equal(committed.length,2);assert.equal(generation.inventory().stylesheetConsumerLeases,2);
        assert.ok(committed.every(output=>output.output.css.includes('@scope')));
        installation.dispose();installation.dispose();assert.equal(generation.inventory().stylesheetConsumerLeases,0);
        const stale=new AbortController();stale.abort();
        assert.throws(()=>declaration.installStylesheets('stale',context,{signal:stale.signal,commit:()=>{throw Error('stale committed');},release:()=>stale.abort()}),/stale/);
        assert.equal(generation.inventory().stylesheetConsumerLeases,0);
        if(data===imported) {
            const changed=structuredClone(context);changed.frames[0].specifiers.resources['./theme.css']={target:'./changed.css'};
            const denied=new AbortController();
            assert.throws(()=>declaration.installStylesheets('denied',changed,{signal:denied.signal,commit:()=>{throw Error('denied committed');},release:()=>denied.abort()}));
            assert.equal(generation.inventory().stylesheetConsumerLeases,0);
        }
        declaration.dispose();declaration.dispose();
        assert.throws(()=>declaration.metadata(),/disposed/);
        assert.equal(generation.inventory().liveDeclarationOwners,0);
    }
}
const foreign=bindings.open(...inline);
foreign.dispose(); // Still a native allocation until its opaque owner is freed.
assert.throws(()=>first.dispose(),/foreign-owner/);
foreign.free();
const reentrant=first.open(...inline), abort=new AbortController();
assert.throws(()=>reentrant.installStylesheets('reentrant',context,{signal:abort.signal,commit:()=>{reentrant.dispose();return true;},release:()=>abort.abort()}),/disposed/);
assert.equal(first.inventory().liveDeclarationOwners,0);
first.dispose();first.dispose();
assert.throws(()=>first.open(...inline),/retired/);
assert.throws(()=>bindings.inventory(),/inactive/);
assert.equal(next.inventory().liveDeclarationOwners,0);
next.dispose();
// A host callback cannot free an owned pointer while its native call is active.
const transport=createCemComponentWasmBindings();
const guarded={...transport,open(...args){
    const raw=transport.open(...args);
    return {describe:()=>raw.describe(),stylesheet:(...input)=>raw.stylesheet(...input),releaseConsumer:input=>raw.releaseConsumer(input),dispose:()=>raw.dispose(),free:()=>raw.free(),
        renderJson(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.renderJson(...input);}};
}};
const protectedGeneration=await createCemComponentGeneration(module,guarded);const protectedOwner=protectedGeneration.open(...inline);
assert.match(protectedOwner.renderJson(Buffer.from('{"value":"Guarded"}'),'app://cart/response.json','datadom'),/Guarded/);
protectedGeneration.dispose();
const cleanup=await createCemComponentGeneration(module,createCemComponentWasmBindings());
const cleanupOwner=cleanup.open(...inline), cleanupAbort=new AbortController();
cleanupOwner.installStylesheets('cleanup',context,{signal:cleanupAbort.signal,commit:()=>true,release:()=>{cleanupAbort.abort();cleanupOwner.dispose();assert.throws(()=>cleanup.open(...inline),/retired/);cleanup.dispose();}});
cleanup.dispose();cleanup.dispose();
console.log('Selected declaration owners: native pins, scoped imports, native JSON, CSS leases, failed/stale/reentrant cleanup, independent heaps and foreign-root retirement refusal passed.');
