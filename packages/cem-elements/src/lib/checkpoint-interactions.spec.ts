import { describe, expect, it } from 'vitest';
import { checkpointReferenceSource, snapshotCheckpointReferences, checkpointXmlSource, checkpointNativeBindings, checkpointNativeCapsule, checkpointPreparation, sameCheckpointNativeValue, snapshotCheckpointNativeValue } from './checkpoint-interactions.js';
import type { NativeCemValue } from './native-values.js';
import type { CemElementReferenceInputs } from './element-reference-inputs.js';
const input = (): CemElementReferenceInputs => ({ kind: 'cem-element-reference-inputs-v1', requesting: 0,
    sources: [{ bundle: Uint8Array.of(1, 2, 3).buffer, primarySourceId: 1, context: true }],
    bindings: [{ source: 0, name: 'relation', select: 'input.children' }], grants: [[0, 0]] });
describe('checkpoint source invocation snapshots', () => {
    it('captures exact original bytes/metadata without granting authority from caller aliases', () => {
        const original = input(); expect(checkpointReferenceSource(original, '1')).toBeUndefined();
        const snapshot = snapshotCheckpointReferences(original)!;
        new Uint8Array(original.sources[0].bundle)[0] = 9;
        (original.bindings[0] as { select: string }).select = 'changed';
        const portable = checkpointReferenceSource(snapshot, '1')!;
        expect(portable.sources[0].base64).toBe(btoa(String.fromCharCode(1, 2, 3)));
        expect(portable.bindings[0].select).toBe('input.children');
        expect(portable.crossings).toEqual([{ from: 0, to: 0 }]); expect(portable.disposition).toBe('reacquire');
        expect(JSON.stringify(portable)).not.toContain('placements'); expect(JSON.stringify(portable)).not.toContain('grants');
    });
    it('refuses oversized sources and unaccounted traversal policy before copying', () => {
        const oversized = input(); oversized.sources[0].bundle = new ArrayBuffer(256 * 1024 + 1);
        expect(snapshotCheckpointReferences(oversized)).toBeUndefined();
        const policy = input(); policy.sources[0].maxWork = 10;
        expect(snapshotCheckpointReferences(policy)).toBeUndefined();
    });
});

describe('inactive original XML source intent', () => {
    it('copies scalar labels and retains bytes without manufacturing row proofs', () => {
        const labels = { option: 'input.children', group: null };
        const source = checkpointXmlSource('<options/>', 'memory:options.xml', 'source:1', 'input.children', labels)!;
        labels.option = 'changed';
        expect(source.labels.option).toBe('input.children'); expect(source.rowCount).toBe(0);
        expect(atob(source.sources[0].base64)).toBe('<options/>'); expect(source.disposition).toBe('reacquire');
        expect(checkpointXmlSource('é'.repeat(256 * 1024), 'memory:options.xml', '1', 'input.children')).toBeUndefined();
    });
});

const value = (): NativeCemValue => ({kind:'cem-native-value-v1',artifact:new Uint8Array([1,2,3]).buffer,contentHash:'opaque-test-only',index:0});
const limits = {maxBytes:262144,maxValues:4096,maxDepth:64};
describe('opaque checkpoint input transport (native validation remains mandatory)', () => {
    it('copies value bytes and checks metadata and in-place mutation without decoding', () => {
        const input = value(), witness = snapshotCheckpointNativeValue(input)!;
        expect(witness.artifact).not.toBe(input.artifact);
        expect(sameCheckpointNativeValue(input,witness)).toBe(true);
        new Uint8Array(input.artifact)[0] = 9;
        expect(sameCheckpointNativeValue(input,witness)).toBe(false);
        expect(new Uint8Array(witness.artifact)[0]).toBe(1);
        input.index = 2; expect(sameCheckpointNativeValue(input,witness)).toBe(false);
    });
    it('keeps additional attributes and wrapped/unwrapped slices with shared artifacts', () => {
        const input = value(), independent = value();
        const saved = checkpointNativeBindings([{name:'count',value:input}], [{name:'wrapped',attribute:'count',value:independent},{name:'raw',value:input}])!;
        expect(saved.artifacts).toHaveLength(1);
        expect(saved.bindings).toEqual([{target:'attribute',name:'count',artifact:0,index:0},{target:'slice',name:'wrapped',artifact:0,index:0,attribute:'count'},{target:'slice',name:'raw',artifact:0,index:0}]);
        new Uint8Array(input.artifact).fill(0);
        expect(saved.artifacts[0].base64).toBe('AQID');
    });
    it('retains exact preparation control-frame serialization and independent limits', () => {
        const data = {datadom:{slices:{caption:'é😀',enabled:true}}};
        const input = {data,limits:{...limits},nativeSlices:[{name:'data',value:value()}]};
        const saved = checkpointPreparation(input)!;
        data.datadom.slices.caption = 'changed'; input.limits.maxDepth = 1;
        expect(saved.dataJson).toBe('{"datadom":{"slices":{"caption":"é😀","enabled":true}}}');
        expect(saved.limits.maxDepth).toBe(64);
        expect(saved.nativeBindings.bindings[0].target).toBe('slice');
    });
    it('refuses unsupported values, metadata, getters, nonfinite controls and oversized inputs', () => {
        const input = value();
        expect(snapshotCheckpointNativeValue({...input,handle:7} as NativeCemValue)).toBeUndefined();
        expect(snapshotCheckpointNativeValue({...input,get artifact(){throw new Error('must not evaluate getter');}})).toBeUndefined();
        expect(checkpointNativeCapsule({...input,index:-1})).toBeUndefined();
        expect(checkpointNativeCapsule({...input,artifact:new ArrayBuffer(262145)})).toBeUndefined();
        expect(checkpointPreparation({data:{number:Infinity},limits})).toBeUndefined();
        expect(checkpointPreparation({data:{get field(){throw new Error('must not evaluate getter');}},limits})).toBeUndefined();
        expect(checkpointPreparation({data:{unicode:'😀'.repeat(65536)},limits})).toBeUndefined();
        expect(checkpointNativeBindings(Array(257).fill({name:'a',value:input}))).toBeUndefined();
        expect(checkpointNativeBindings([], [{name:'x',attribute:'',value:input}])).toBeUndefined();
    });
});
