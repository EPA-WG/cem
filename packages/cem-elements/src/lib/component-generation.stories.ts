import type {Meta, StoryObj} from '@storybook/web-components-vite';
import {expect, waitFor} from 'storybook/test';
import {createCemComponentGeneration, type CemComponentWasmBindings} from './internal/runtime-support/component-generation.js';
import {createCemDeclarationScope} from './declaration-scope.js';
import {DeclarationStyleOwnership} from './declaration-style-ownership.js';
import type {CemModuleUrlContextWire} from './internal/runtime-support/module-url-resolution.js';

export default {title:'CEM Elements/Selected Component Generation',tags:['test']} satisfies Meta;
type Story=StoryObj;
function networkTraceStart() {
    // Retain current-story requests after Storybook's module/fixture downloads.
    performance.setResourceTimingBufferSize(4096);
    return performance.now();
}
const inputs=import.meta.glob<string>('../../../../dist/artifacts/cem_ql/component-runtime/owned-*',{eager:true,query:'?url',import:'default'});
const runtime=import.meta.glob<string>('../../../cem_ql/dist/wasm-component/{cem_ql_factory.js,cem_ql_bg.wasm}',{eager:true,query:'?url',import:'default'});
async function artifact(name:string,suffix:string) {
    const url=inputs[`../../../../dist/artifacts/cem_ql/component-runtime/${name}.${suffix}`];
    if(!url) throw Error('Build native component runtime fixtures before this story');
    const response=await fetch(url);if(!response.ok)throw Error('Missing native runtime fixture');return response;
}
export const NativeDeclarationAndStyleLeases:Story={
    render:()=>'<section aria-label="Selected runtime declaration ownership">Native compiled declarations and CSS leases.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const root=canvasElement.querySelector('section');
        if(!root)throw Error('Missing selected runtime fixture');
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const bytes=await(await fetch(wasmUrl)).arrayBuffer();
        const module=await WebAssembly.compile(bytes);
        const context:CemModuleUrlContextWire={identity:'owned',resolverIdentity:'resolver',resourcePolicyStamp:'policy',frames:[{frameId:'root',baseUrl:'https://app.test/components/',scopes:[],specifiers:{imports:{},resources:{}}}]};
        for(const name of ['owned-inline','owned-imports']) {
            const bindings:CemComponentWasmBindings=factory.createCemComponentWasmBindings();
            const generation=await createCemComponentGeneration(module,bindings);
            const scope=createCemDeclarationScope({document});
            const styles=new DeclarationStyleOwnership(document,scope);
            const declaration=document.createElement('div'), first=document.createElement('cem-owned-proof'), second=document.createElement('cem-owned-proof');
            for(const element of [first,second]) {element.setAttribute('scope','owned-proof');
                // A shared scope needs the produced-instance inert boundary.
                const island=document.createElement('template');island.setAttribute('data-cem-island','instance');element.append(island);
                const label=document.createElement('span');label.className='label';label.textContent='Retained content';element.append(label);}
            root.append(declaration,first,second);styles.add(declaration,scope);
            const component=generation.open(new Uint8Array(await(await artifact(name,'bin')).arrayBuffer()),await(await artifact(name,'json')).text(),(await(await artifact(name,'hash')).text()).trim());
            try {
                const one=component.installStylesheets('one',context,styles.beginConsumer(first,scope));
                const two=component.installStylesheets('two',context,styles.beginConsumer(second,scope));
                await waitFor(()=>expect(getComputedStyle(first).getPropertyValue('--private-proof').trim()).toBe('ready'));
                const label=first.querySelector('.label');if(!label)throw Error('Missing scoped label');
                expect(getComputedStyle(label).getPropertyValue('--shared-proof').trim()).toBe('ready');
                if(name==='owned-imports')expect(getComputedStyle(first).getPropertyValue('--imported-proof').trim()).toBe('ready');
                expect(declaration.querySelectorAll('style')).toHaveLength(2);
                const nodes=[...declaration.querySelectorAll('style')];
                expect(generation.inventory().stylesheetConsumerLeases).toBe(4);
                if(name==='owned-imports') {
                    const denied=structuredClone(context);denied.frames[0].specifiers.resources['./theme.css']={target:'./changed.css'};
                    expect(()=>component.installStylesheets('bad',denied,styles.stageConsumer(first,scope))).toThrow();
                    expect([...declaration.querySelectorAll('style')]).toEqual(nodes);
                    expect(getComputedStyle(first).getPropertyValue('--imported-proof').trim()).toBe('ready');
                    expect(generation.inventory().stylesheetConsumerLeases).toBe(4);
                }
                one.dispose();expect(declaration.querySelectorAll('style')).toHaveLength(2);
                expect(generation.inventory().stylesheetConsumerLeases).toBe(2);
                second.remove();
                await waitFor(()=>expect(generation.inventory().stylesheetConsumerLeases).toBe(0));
                expect(declaration.querySelectorAll('style')).toHaveLength(0);
                two.dispose();component.dispose();
                expect(generation.inventory().liveDeclarationOwners).toBe(0);
                generation.dispose();expect(()=>component.metadata()).toThrow('retired');
            } finally {scope.dispose();generation.dispose();declaration.remove();first.remove();second.remove();}
        }
        const wasmRequests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name).filter(name=>name.includes('cem_ql_bg.wasm'));
        expect(wasmRequests).toEqual([new URL(wasmUrl,location.href).href]);
        expect(performance.getEntriesByType('resource').some(entry=>entry.name.startsWith('https://app.test/'))).toBe(false);
    },
};

export const NativeAttributeAndSliceInputs:Story={
    render:()=>'<section aria-label="Selected runtime native inputs">Native attribute and slice values.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing native input fixture');
        const routes=[{target:'attribute' as const,name:'label',index:0},{target:'slice' as const,name:'amount',index:1}];
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-inputs','bin')).arrayBuffer()),await(await artifact('owned-inputs','json')).text(),(await(await artifact('owned-inputs','hash')).text()).trim());
                for(const name of ['first','second']) {
                    const input=new Uint8Array(await(await artifact(`owned-input-${name}`,'cemv')).arrayBuffer());
                    const expected=await(await artifact(`owned-input-${name}`,'html')).text();
                    const output=component.renderInputs(input,routes);
                    expect(output).toBe(expected);
                    expect(component.renderInputs(input,[...routes].reverse())).toBe(expected);
                    root.innerHTML=output; // Test projection only; production ownership API returns native HTML.
                    const paragraph=root.querySelector('p');if(!paragraph)throw Error('Missing native input output');
                    expect(paragraph.textContent).toBe(name==='first'?'Tea <img> / 2':'Coffee & cake / 3');
                    expect(root.querySelector('img')).toBeNull();
                    expect(()=>component.renderInputs(input,[{target:'attribute',name:'wrong',index:0}])).toThrow();
                    expect(()=>component.renderInputs(input,[{target:'slice',name:'suggestions',index:1}])).toThrow();
                    expect(component.renderInputs(input,routes)).toBe(expected);
                }
                component.dispose();expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xhtml')||name.endsWith('.cem'))).toBe(false);
    },
};

export const NativeReceiversAndInstanceIdentity:Story={
    render:()=>'<section aria-label="Selected runtime receiver contracts">Native receiver constraints and host instance identity.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing receiver fixture');
        const routes=[{target:'attribute' as const,name:'quantity',index:0}];
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-receiver','bin')).arrayBuffer()),await(await artifact('owned-receiver','json')).text(),(await(await artifact('owned-receiver','hash')).text()).trim());
                for(const name of ['first','second']) {
                    const input=new Uint8Array(await(await artifact(`owned-receiver-${name}`,'cemv')).arrayBuffer());
                    const expected=await(await artifact(`owned-receiver-${name}`,'html')).text();
                    const actual=component.renderInputs(input,routes,`receiver-${name}`);
                    expect(actual).toBe(expected);
                    root.innerHTML=actual; // Test projection of native output.
                    expect(root.querySelector('p')?.id).toBe(`receiver-${name}`);
                    expect(root.querySelector('p')?.textContent).toBe(`${name==='first'?3:4}/${name==='first'?3:4}/receiver-${name}`);
                    for(const invalid of ['low','high','invalid','missing']) {
                        const bytes=new Uint8Array(await(await artifact(`owned-receiver-${invalid}`,'cemv')).arrayBuffer());
                        let failure:unknown;
                        try {component.renderInputs(bytes,invalid==='missing'?[]:routes,'refused');} catch(error) {failure=error;}
                        expect(failure).toBe('compiled application render failed');
                    }
                    expect(()=>component.renderInputs(input,routes,'a b')).toThrow();
                    expect(()=>component.renderInputs(input,routes,'\ud800')).toThrow();
                    expect(()=>component.renderInputs(input,[{target:'slice',name:'instanceID',index:0}],'protected')).toThrow();
                    expect(()=>component.renderInputs(input,routes)).toThrow();
                    expect(component.renderInputs(input,routes,`receiver-${name}`)).toBe(expected);
                    const id='cart:é🛒"><svg>';
                    root.innerHTML=component.renderInputs(input,routes,id);
                    expect(root.querySelector('p')?.id).toBe(id);
                    expect(root.querySelector('svg')).toBeNull();
                }
                component.dispose();expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xhtml')||name.endsWith('.cem'))).toBe(false);
    },
};

export const PassiveNativeIslandProjection:Story={
    render:()=>'<section aria-label="Selected runtime native island projection">Native island payload, slots and projection.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing island fixture');
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-island','bin')).arrayBuffer()),await(await artifact('owned-island','json')).text(),(await(await artifact('owned-island','hash')).text()).trim());
                for(const name of ['first','second','empty']) {
                    const input=new Uint8Array(await(await artifact(`owned-island-${name}`,'cemv')).arrayBuffer());
                    const expected=await(await artifact(`owned-island-${name}`,'html')).text();
                    const expectedProjection=new Uint8Array(await(await artifact(`owned-island-${name}.projection`,'cemv')).arrayBuffer());
                    const id=name==='second'?'island-two':'island-one';
                    const html=component.renderIsland(input,id);
                    expect(html).toBe(expected);
                    const projection=component.projectIsland(input,id);
                    expect(projection).toEqual(expectedProjection); // Opaque binary comparison; native code owns decoding.
                    root.innerHTML=html; // Test-only projection of native HTML.
                    expect(root.textContent).toBe(name==='empty'?'':`${name==='first'?'Heading':'Updated'}Second`);
                    expect(root.querySelector('article')?.id).toBe(name==='empty'?undefined:id);
                    expect(()=>component.renderIsland(projection,id)).toThrow();
                    expect(()=>component.projectIsland(projection,id)).toThrow();
                    expect(()=>component.renderIsland(input,'wrong-instance')).toThrow();
                    for(const invalid of ['bad-version','bad-namespace','bad-state','credential']) {
                        const bytes=new Uint8Array(await(await artifact(`owned-island-${invalid}`,'cemv')).arrayBuffer());
                        expect(()=>component.renderIsland(bytes,id)).toThrow();
                        expect(()=>component.projectIsland(bytes,id)).toThrow();
                    }
                    expect(component.renderIsland(input,id)).toBe(expected);
                }
                component.dispose();expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xhtml')||name.endsWith('.cem')||name.startsWith('app://island/'))).toBe(false);
    },
};

export const NativeIslandDomains:Story={
    render:()=>'<section aria-label="Selected runtime native island domains">Typed attributes, dataset and nested slices.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing domain fixture');
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-domains','bin')).arrayBuffer()),await(await artifact('owned-domains','json')).text(),(await(await artifact('owned-domains','hash')).text()).trim());
                for(const name of ['first','second']) {
                    const input=new Uint8Array(await(await artifact(`owned-domains-${name}`,'cemv')).arrayBuffer());
                    const expected=await(await artifact(`owned-domains-${name}`,'html')).text();
                    const id=name==='second'?'island-two':'island-one';
                    const html=component.renderIsland(input,id);
                    expect(html).toBe(expected);
                    root.innerHTML=html; // Test-only mounting of native HTML, with no UI implementation here.
                    expect(root.querySelector('p')?.id).toBe(id);
                    expect(root.querySelector('p')?.title).toBe(name==='first'?' Tea & cake ':' Coffee & cake ');
                    expect(root.textContent).toContain('/9007199254740993.25/true');
                    expect(root.querySelector('p')?.textContent).toContain(name==='first'?'3/3/3/003/banana':'4/4/4/004/pear');
                    const projection=component.projectIsland(input,id);
                    expect(Array.from(projection.subarray(0,5))).toEqual([67,69,77,86,3]);
                    expect(component.projectIsland(input,id)).toEqual(projection);
                    expect(()=>component.renderIsland(projection,id)).toThrow();
                    for(const invalid of ['dataset','boolean','alias','credential','bounds']) {
                        const bytes=new Uint8Array(await(await artifact(`owned-domains-${invalid}`,'cemv')).arrayBuffer());
                        expect(()=>component.renderIsland(bytes,'island-one')).toThrow();
                        expect(()=>component.projectIsland(bytes,'island-one')).toThrow();
                    }
                    expect(component.renderIsland(input,id)).toBe(expected);
                }
                component.dispose();expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xhtml')||name.endsWith('.cem')||name.startsWith('app://island/'))).toBe(false);
    },
};

export const NativePayloadReferences:Story={
    render:()=>'<section aria-label="Selected runtime native payload references">Repeated, nested and empty native references.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing reference fixture');
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-references','bin')).arrayBuffer()),await(await artifact('owned-references','json')).text(),(await(await artifact('owned-references','hash')).text()).trim());
                try {
                    for(const name of ['first','second','empty']) {
                        const input=new Uint8Array(await(await artifact(`owned-references-${name}`,'cemv')).arrayBuffer());
                        const expected=await(await artifact(`owned-references-${name}`,'html')).text();
                        const projected=new Uint8Array(await(await artifact(`owned-references-${name}.projection`,'cemv')).arrayBuffer());
                        const id=name==='second'?'island-two':'island-one';
                        const html=component.renderIsland(input,id);
                        expect(html).toBe(expected);
                        expect(component.projectIsland(input,id)).toEqual(projected);
                        root.innerHTML=html; // Test-only mounting; native code owns reference/slot projection.
                        const word=name==='second'?'Updated':'Referenced';
                        expect(root.querySelector('header')?.textContent??'').toBe(name==='empty'?'':`HeadingSecond${word}${word}`);
                        expect(root.querySelector('main')?.textContent??'').toBe(name==='empty'?'':`Default${word}`);
                        expect(root.querySelectorAll('header > mark').length).toBe(name==='empty'?0:2);
                        expect(root.querySelectorAll('main > section > mark').length).toBe(name==='empty'?0:1);
                        expect(root.querySelector('article')?.id).toBe(name==='empty'?undefined:id);
                        expect(()=>component.renderIsland(projected,id)).toThrow();
                        for(const invalid of ['foreign','source','opaque-source','password','file','otp','nested-credential','document','atomic','reference-source','schema','cycle','dangling','depth','expanded']) {
                            const bytes=new Uint8Array(await(await artifact(`owned-references-${invalid}`,'cemv')).arrayBuffer());
                            expect(()=>component.renderIsland(bytes,'island-one')).toThrow();
                            expect(()=>component.projectIsland(bytes,'island-one')).toThrow();
                        }
                        expect(component.renderIsland(input,id)).toBe(expected);
                    }
                } finally {component.dispose();}
                expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xhtml')||name.endsWith('.cem')||name.startsWith('app://island/'))).toBe(false);
    },
};

export const PinnedNamedReceiverTypes:Story={
    render:()=>'<section aria-label="Selected runtime pinned named types">Schema-pinned receiver ranges, patterns and defaults.</section>',
    play:async({canvasElement})=>{
        const started=networkTraceStart();
        const factoryUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_factory.js'];
        const wasmUrl=runtime['../../../cem_ql/dist/wasm-component/cem_ql_bg.wasm'];
        if(!factoryUrl||!wasmUrl)throw Error('Build the selected component WASM profile before this story');
        const factory=await import(/* @vite-ignore */ factoryUrl);
        const module=await WebAssembly.compile(await(await fetch(wasmUrl)).arrayBuffer());
        const root=canvasElement.querySelector('section');if(!root)throw Error('Missing type fixture');
        const first=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        const second=await createCemComponentGeneration(module,factory.createCemComponentWasmBindings());
        try {
            expect(first.memory()).not.toBe(second.memory());
            for(const generation of [first,second]) {
                const component=generation.open(new Uint8Array(await(await artifact('owned-types','bin')).arrayBuffer()),await(await artifact('owned-types','json')).text(),(await(await artifact('owned-types','hash')).text()).trim());
                try {
                    expect(component.metadata().tag).toBe('cem-owned-types');
                    for(const name of ['first','second']) {
                        const input=new Uint8Array(await(await artifact(`owned-types-${name}`,'cemv')).arrayBuffer());
                        const routes=[{target:'attribute' as const,name:'quantity',index:0},...(name==='second'?[{target:'attribute' as const,name:'label',index:1}]:[])];
                        const expected=await(await artifact(`owned-types-${name}`,'html')).text();
                        const html=component.renderInputs(input,routes,'named-owner');
                        expect(html).toBe(expected);
                        root.innerHTML=html; // Test-only mounting of native output.
                        expect(root.querySelector('p')?.id).toBe('named-owner');
                        expect(root.querySelector('p')?.title).toBe(name==='first'?'ABC':'XYZ');
                        expect(root.textContent).toBe(name==='first'?'3/3/ABC':'4/4/XYZ');
                        for(const bad of ['low','high','pattern','short']) {
                            const bytes=new Uint8Array(await(await artifact(`owned-types-${bad}`,'cemv')).arrayBuffer());
                            const bindings=[{target:'attribute' as const,name:'quantity',index:0},...(['pattern','short'].includes(bad)?[{target:'attribute' as const,name:'label',index:1}]:[])];
                            expect(()=>component.renderInputs(bytes,bindings,'named-owner')).toThrow();
                        }
                        for(const bad of ['changed','missing','duplicate','source','schema','unsupported','unbound','trailing','stale','node','list','content-type','restriction-node','restriction-pending','policy','callback-extension','future-envelope']) {
                            const bytes=new Uint8Array(await(await artifact(`owned-types-${bad}`,'bin')).arrayBuffer());
                            const json=await(await artifact(`owned-types-${bad}`,'json')).text();
                            const pin=(await(await artifact(`owned-types-${bad}`,'hash')).text()).trim();
                            expect(()=>generation.open(bytes,json,pin)).toThrow();
                            expect(generation.inventory().liveDeclarationOwners).toBe(1);
                        }
                        expect(component.renderInputs(input,routes,'named-owner')).toBe(expected);
                    }
                    const island=new Uint8Array(await(await artifact('owned-types-island','cemv')).arrayBuffer());
                    expect(component.renderIsland(island,'island-one')).toContain('3/3/ABC');
                    expect(Array.from(component.projectIsland(island,'island-one').subarray(0,5))).toEqual([67,69,77,86,3]);
                } finally {component.dispose();}
                expect(generation.inventory().liveDeclarationOwners).toBe(0);
            }
        } finally {first.dispose();second.dispose();}
        const requests=performance.getEntriesByType('resource').filter(entry=>entry.startTime>=started&&(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name);
        expect(requests.filter(name=>name.includes('cem_ql_bg.wasm'))).toEqual([new URL(wasmUrl,location.href).href]);
        expect(requests.some(name=>name.endsWith('.xml')||name.endsWith('.xhtml')||name.endsWith('.cem')||name.startsWith('app://types/'))).toBe(false);
    },
};
