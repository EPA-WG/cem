import { describe, expect, it } from 'vitest';
import { checkpointScalarPortable } from './checkpoint-controls.js';
describe('checkpoint scalar repository boundary', () => {
    it('admits ordinary scalar DTOs and repeated acyclic values', () => {
        const shared={label:'é😀', values:[true,false,null,0]};
        expect(checkpointScalarPortable({first:shared,second:shared})).toBe(true);
    });
    it('refuses cloneable host values and values DOM record encoding would lose', () => {
        for(const value of [Object.defineProperty({},'hidden',{value:'lost'}),{[Symbol('hidden')]:'lost'},Object.defineProperty({},'callback',{enumerable:true,get:()=>{throw new Error('must not call');}}),new Date(),new Map(),new Set(),new Uint8Array([1]),new ArrayBuffer(1),undefined,NaN,Infinity,1n,()=>0,[undefined],Array(1)])
            expect(checkpointScalarPortable(value)).toBe(false);
        const decorated: string[] & { extra?: string }=[]; decorated.extra='lost'; expect(checkpointScalarPortable(decorated)).toBe(false);
        const cycle: Record<string,unknown>={}; cycle.self=cycle; expect(checkpointScalarPortable(cycle)).toBe(false);
    });
    it('bounds value, key, node and nesting work before export', () => {
        expect(checkpointScalarPortable('x'.repeat(512*1024+1))).toBe(false);
        expect(checkpointScalarPortable({['x'.repeat(512*1024+1)]:true})).toBe(false);
        expect(checkpointScalarPortable(Array(16385).fill(true))).toBe(false);
        let value:unknown=0; for(let i=0;i<70;i++) value=[value]; expect(checkpointScalarPortable(value)).toBe(false);
    });
});
