import { describe, expect, it } from 'vitest';
import { checkpointReferenceSource, snapshotCheckpointReferences, checkpointXmlSource } from './checkpoint-interactions.js';
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
