import { describe, expect, it } from 'vitest';
import { classifyRetainedPayloadElement, resolveRetainedPayloadContract } from './retained-payload-contract.js';

describe('declaration-owned retained payload vocabularies', () => {
    const html = 'http://www.w3.org/1999/xhtml';
    it('normalizes equivalent header blocks and freezes every member', () => {
        const root = { 'payload-item': 'entry option', 'payload-group': 'section optgroup', 'payload-topology': 'grouped' };
        const contract = resolveRetainedPayloadContract(root, {
            ...root, 'payload-item': 'Q{}entry\noption',
        });
        expect(contract).toEqual({ contract: 'cem-retained-payload/1', topology: 'grouped', vocabularies: [
            { item: { namespace: null, localName: 'entry' }, group: { namespace: null, localName: 'section' } },
            { item: { namespace: null, localName: 'option' }, group: { namespace: null, localName: 'optgroup' } },
        ] });
        expect(Object.isFrozen(contract)).toBe(true);
        expect(Object.isFrozen(contract?.vocabularies)).toBe(true);
        expect(Object.isFrozen(contract?.vocabularies[0].item)).toBe(true);
    });
    it('matches expanded names without treating prefixes or keys as authority', () => {
        const contract = resolveRetainedPayloadContract({ 'payload-item': 'Q{urn:payload}entrée Q{}item', 'payload-topology': 'recursive' });
        const node = { tag: 'other:entrée', namespace: 'urn:payload', key: 'original-key', children: [] };
        expect(classifyRetainedPayloadElement(contract, node)).toEqual({ role: 'item', vocabulary: 0 });
        expect(node).toEqual({ tag: 'other:entrée', namespace: 'urn:payload', key: 'original-key', children: [] });
        expect(classifyRetainedPayloadElement(contract, { tag: 'entrée', namespace: html })).toBeNull();
        expect(classifyRetainedPayloadElement(contract, { tag: 'item', namespace: null })).toEqual({ role: 'item', vocabulary: 1 });
        expect(classifyRetainedPayloadElement(contract, { tag: 'item', namespace: html })).toBeNull();
        expect(classifyRetainedPayloadElement(null, node)).toBeNull();
    });
    it('keeps absence distinct from an invalid or partially inherited contract', () => {
        expect(resolveRetainedPayloadContract({ title: 'Unrelated' })).toBeNull();
        expect(resolveRetainedPayloadContract({}, { 'payload-item': 'item' })?.topology).toBe('flat');
        expect(() => resolveRetainedPayloadContract({ 'payload-item': 'item' }, { 'payload-group': 'group' })).toThrow('payload_contract.invalid');
        expect(() => resolveRetainedPayloadContract({ 'payload-item': 'item' }, { 'payload-item': 'different' })).toThrow('payload_contract.invalid');
    });
    it.each([
        { 'payload-item': '' }, { 'payload-group': 'group' }, { 'payload-topology': 'flat' },
        { 'payload-item': 'item', 'payload-topology': 'unknown' },
        { 'payload-item': 'item', 'payload-group': 'group' },
        { 'payload-item': 'item', 'payload-topology': 'grouped' },
        { 'payload-item': 'a b', 'payload-group': 'group', 'payload-topology': 'grouped' },
        { 'payload-item': 'item item' }, { 'payload-item': 'Q{}item Q{}item' },
        { 'payload-item': 'item', 'payload-group': 'item', 'payload-topology': 'grouped' },
        { 'payload-item': 'item', 'payload-group': 'group', 'payload-topology': 'recursive' },
        ...['*', '.entry', '[slot=item]', 'a:item', '1item', 'Q{urn:x', 'Q{urn:bad space}item', 'a'.repeat(257), Array.from({ length: 9 }, (_, i) => `item${i}`).join(' ')].map(value => ({ 'payload-item': value })),
    ])('refuses malformed, ambiguous or unbounded metadata: %j', header => {
        expect(() => resolveRetainedPayloadContract(header)).toThrow('payload_contract.invalid');
    });
});
