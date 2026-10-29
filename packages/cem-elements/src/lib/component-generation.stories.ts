import type {Meta, StoryObj} from '@storybook/web-components-vite';
import {expect, waitFor} from 'storybook/test';
import {createCemComponentGeneration, type CemComponentWasmBindings} from './internal/runtime-support/component-generation.js';
import {createCemDeclarationScope} from './declaration-scope.js';
import {DeclarationStyleOwnership} from './declaration-style-ownership.js';
import type {CemModuleUrlContextWire} from './internal/runtime-support/module-url-resolution.js';

export default {title:'CEM Elements/Selected Component Generation',tags:['test']} satisfies Meta;
type Story=StoryObj;
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
        const wasmRequests=performance.getEntriesByType('resource').filter(entry=>(entry as PerformanceResourceTiming).initiatorType==='fetch').map(entry=>entry.name).filter(name=>name.includes('cem_ql_bg.wasm'));
        expect(wasmRequests).toEqual([new URL(wasmUrl,location.href).href]);
        expect(performance.getEntriesByType('resource').some(entry=>entry.name.startsWith('https://app.test/'))).toBe(false);
    },
};
