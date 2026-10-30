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
// Binary attributes/slices use the shared native codec, never a JSON data frame.
const inputFixture=await fixture('owned-inputs');
const routes=[{target:'attribute',name:'label',index:0},{target:'slice',name:'amount',index:1}];
for(const generation of [first,next]) {
    const owner=generation.open(...inputFixture);
    for(const name of ['first','second']) {
        const input=await readFile(new URL(`owned-input-${name}.cemv`,root));
        const expected=await readFile(new URL(`owned-input-${name}.html`,root),'utf8');
        assert.equal(owner.renderInputs(input,routes),expected);
        assert.equal(owner.renderInputs(input,routes.toReversed()),expected);
        assert.throws(()=>owner.renderInputs(input,[{target:'attribute',name:'wrong',index:0}]));
        assert.throws(()=>owner.renderInputs(input,[{target:'slice',name:'suggestions',index:1}]));
        assert.equal(owner.renderInputs(input,routes),expected);
    }
    owner.dispose();
    assert.throws(()=>owner.renderInputs(new Uint8Array(),routes),/disposed/);
    assert.equal(generation.inventory().liveDeclarationOwners,0);
}
const receiverFixture=await fixture('owned-receiver');
const receiverRoutes=[{target:'attribute',name:'quantity',index:0}];
for(const generation of [first,next]) {
    const owner=generation.open(...receiverFixture);
    for(const name of ['first','second']) {
        const input=await readFile(new URL(`owned-receiver-${name}.cemv`,root));
        const expected=await readFile(new URL(`owned-receiver-${name}.html`,root),'utf8');
        assert.equal(owner.renderInputs(input,receiverRoutes,`receiver-${name}`),expected);
        for(const invalid of ['low','high','invalid','missing']) {
            const bad=await readFile(new URL(`owned-receiver-${invalid}.cemv`,root));
            assert.throws(()=>owner.renderInputs(bad,invalid==='missing'?[]:receiverRoutes,'refused'),error=>error==='compiled application render failed');
        }
        for(const id of ['', 'a b', 'line\nbreak', 'x'.repeat(1025), 'é'.repeat(600), '\ud800']) assert.throws(()=>owner.renderInputs(input,receiverRoutes,id));
        for(const name of ['instanceID','island','suggestions']) assert.throws(()=>owner.renderJson(Buffer.from('{}'),'app://data/receiver.json',name),error=>error==='compiled declaration needs a pinned native JSON binding');
        assert.throws(()=>owner.renderInputs(input,receiverRoutes),error=>error==='compiled application render failed');
        assert.equal(owner.renderInputs(input,receiverRoutes,`receiver-${name}`),expected);
    }
    owner.dispose();
    assert.equal(generation.inventory().liveDeclarationOwners,0);
}
const islandFixture=await fixture('owned-island');
for(const generation of [first,next]) {
    const owner=generation.open(...islandFixture);
    for(const name of ['first','second','empty']) {
        const input=await readFile(new URL(`owned-island-${name}.cemv`,root));
        const expected=await readFile(new URL(`owned-island-${name}.html`,root),'utf8');
        const expectedProjection=await readFile(new URL(`owned-island-${name}.projection.cemv`,root));
        const id=name==='second'?'island-two':'island-one';
        assert.equal(owner.renderIsland(input,id),expected);
        const projected=owner.projectIsland(input,id);
        assert.deepEqual(Buffer.from(projected),expectedProjection);
        assert.throws(()=>owner.renderIsland(projected,id));
        assert.throws(()=>owner.projectIsland(projected,id));
        assert.throws(()=>owner.renderIsland(input,'wrong-instance'));
        for(const invalid of ['bad-version','bad-namespace','bad-state','credential']) {
            const bytes=await readFile(new URL(`owned-island-${invalid}.cemv`,root));
            assert.throws(()=>owner.renderIsland(bytes,id),error=>error==='compiled declaration native island refused');
            assert.throws(()=>owner.projectIsland(bytes,id),error=>error==='compiled declaration native island refused');
        }
        assert.equal(owner.renderIsland(input,id),expected);
    }
    owner.dispose();assert.throws(()=>owner.projectIsland(new Uint8Array(),'island-one'),/disposed/);
    assert.equal(generation.inventory().liveDeclarationOwners,0);
}
const domainFixture=await fixture('owned-domains');
for(const generation of [first,next]) {
    const owner=generation.open(...domainFixture);
    for(const name of ['first','second']) {
        const input=await readFile(new URL(`owned-domains-${name}.cemv`,root));
        const expected=await readFile(new URL(`owned-domains-${name}.html`,root),'utf8');
        const id=name==='second'?'island-two':'island-one';
        assert.equal(owner.renderIsland(input,id),expected);
        const projected=owner.projectIsland(input,id);
        assert.equal(Buffer.from(projected.subarray(0,5)).toString('hex'),'43454d5603');
        assert.deepEqual(owner.projectIsland(input,id),projected);
        assert.throws(()=>owner.renderIsland(projected,id));
        for(const invalid of ['dataset','boolean','alias','credential','bounds']) {
            const bytes=await readFile(new URL(`owned-domains-${invalid}.cemv`,root));
            const message=invalid==='bounds'?'compiled application render failed':'compiled declaration native island refused';
            assert.throws(()=>owner.renderIsland(bytes,'island-one'),error=>error===message);
            assert.throws(()=>owner.projectIsland(bytes,'island-one'),error=>error===message);
        }
        assert.equal(owner.renderIsland(input,id),expected);
    }
    owner.dispose();
    assert.throws(()=>owner.projectIsland(new Uint8Array(),'island-one'),/disposed/);
    assert.equal(generation.inventory().liveDeclarationOwners,0);
}
const referenceFixture=await fixture('owned-references');
for(const generation of [first,next]) {
    const owner=generation.open(...referenceFixture);
    for(const name of ['first','second','empty']) {
        const input=await readFile(new URL(`owned-references-${name}.cemv`,root));
        const expected=await readFile(new URL(`owned-references-${name}.html`,root),'utf8');
        const projected=await readFile(new URL(`owned-references-${name}.projection.cemv`,root));
        const id=name==='second'?'island-two':'island-one';
        assert.equal(owner.renderIsland(input,id),expected);
        assert.deepEqual(Buffer.from(owner.projectIsland(input,id)),projected);
        assert.throws(()=>owner.renderIsland(projected,id));
        for(const invalid of ['foreign','source','opaque-source','password','file','otp','nested-credential','document','atomic','reference-source','schema','cycle','dangling','depth','expanded']) {
            const bytes=await readFile(new URL(`owned-references-${invalid}.cemv`,root));
            assert.throws(()=>owner.renderIsland(bytes,'island-one'),error=>error==='compiled declaration native island refused');
            assert.throws(()=>owner.projectIsland(bytes,'island-one'),error=>error==='compiled declaration native island refused');
        }
        assert.equal(owner.renderIsland(input,id),expected);
    }
    owner.dispose();assert.equal(generation.inventory().liveDeclarationOwners,0);
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
        renderIsland(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.renderIsland(...input);},
        projectIsland(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.projectIsland(...input);},
        renderInstanceInputs(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.renderInstanceInputs(...input);},
        renderInputs(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.renderInputs(...input);},
        renderJson(...input){assert.throws(()=>protectedOwner.dispose(),/busy/);assert.throws(()=>protectedGeneration.dispose(),/busy/);return raw.renderJson(...input);}};
}};
const protectedGeneration=await createCemComponentGeneration(module,guarded);const protectedOwner=protectedGeneration.open(...inline);
assert.match(protectedOwner.renderJson(Buffer.from('{"value":"Guarded"}'),'app://cart/response.json','datadom'),/Guarded/);
const protectedInputs=protectedGeneration.open(...inputFixture);
const input=await readFile(new URL('owned-input-first.cemv',root));
assert.match(protectedInputs.renderInputs(input,routes),/Tea &lt;img&gt;/);
const protectedReceiver=protectedGeneration.open(...receiverFixture);
const receiverInput=await readFile(new URL('owned-receiver-first.cemv',root));
assert.match(protectedReceiver.renderInputs(receiverInput,receiverRoutes,'protected'),/3\/3\/protected/);
const protectedIsland=protectedGeneration.open(...islandFixture);
const islandInput=await readFile(new URL('owned-island-first.cemv',root));
assert.match(protectedIsland.renderIsland(islandInput,'island-one'),/Heading/);
assert.ok(protectedIsland.projectIsland(islandInput,'island-one').byteLength>0);
const reentrantIsland=protectedGeneration.open(...islandFixture);
assert.throws(()=>reentrantIsland.renderIsland({get byteLength(){reentrantIsland.dispose();return 0;}},'island-one'),/disposed/);
const reentrantInputs=protectedGeneration.open(...inputFixture);
assert.throws(()=>reentrantInputs.renderInputs(input,{toJSON(){reentrantInputs.dispose();return routes;},length:2}),/disposed/);
protectedGeneration.dispose();
const cleanup=await createCemComponentGeneration(module,createCemComponentWasmBindings());
const cleanupOwner=cleanup.open(...inline), cleanupAbort=new AbortController();
cleanupOwner.installStylesheets('cleanup',context,{signal:cleanupAbort.signal,commit:()=>true,release:()=>{cleanupAbort.abort();cleanupOwner.dispose();assert.throws(()=>cleanup.open(...inline),/retired/);cleanup.dispose();}});
cleanup.dispose();cleanup.dispose();
console.log('Selected declaration owners: native pins, scoped imports, native JSON/attribute/slice inputs, receiver constraints, native island projections and instance identity, CSS leases, failed/stale/reentrant cleanup, independent heaps and foreign-root retirement refusal passed.');
