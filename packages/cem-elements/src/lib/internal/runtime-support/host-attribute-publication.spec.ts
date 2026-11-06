import { describe, expect, it } from 'vitest';
import { hostAttributeCommitControl, prepareHostAttributePublication } from './host-attribute-publication.js';
function fixture() {
    const seen: string[] = []; let valid = true; let value = 'old';
    const publication = prepareHostAttributePublication({
        current: () => valid,
        apply: () => { seen.push('apply'); value = 'new'; },
        rollback: () => { seen.push('rollback'); value = 'old'; },
        published: () => seen.push('published'),
    });
    return { publication, seen, value: () => value, expire: () => { valid = false; } };
}
describe('native host/island publication', () => {
    it('publishes synchronously with its existing DOM transaction', () => {
        const f = fixture();
        expect(f.publication.publish(() => { expect(f.value()).toBe('new'); return 'applied'; }, value => value === 'applied')).toBe('applied');
        expect(f.seen).toEqual(['apply', 'published']);
        expect(() => f.publication.publish(() => true, Boolean)).toThrow();
    });
    it('rolls back a rejected patch and preserves its result', () => {
        const f = fixture(); expect(f.publication.publish(() => 'aborted', value => value === 'applied')).toBe('aborted');
        expect(f.value()).toBe('old'); expect(f.seen).toEqual(['apply', 'rollback']);
    });
    it('rolls back a synchronous publication exception', () => {
        const f = fixture(); expect(() => f.publication.publish(() => { throw Error('patch failure'); }, Boolean)).toThrow('patch failure');
        expect(f.value()).toBe('old'); expect(f.seen).toEqual(['apply', 'rollback']);
    });
    it('rejects a stale frame before mutating and rejects cancellation/replay', () => {
        const f = fixture(); f.expire(); expect(() => f.publication.publish(() => true, Boolean)).toThrow(); expect(f.seen).toEqual([]);
        const g = fixture(); g.publication.cancel(); expect(() => g.publication.publish(() => true, Boolean)).toThrow(); expect(g.seen).toEqual([]);
    });
    it('detects authority changes during publication and restores the host state', () => {
        const f = fixture(); expect(() => f.publication.publish(() => { f.expire(); return true; }, Boolean)).toThrow();
        expect(f.value()).toBe('old'); expect(f.seen).toEqual(['apply', 'rollback']);
    });
    it('owns primitive native metadata and rejects incomplete or duplicate controls', () => {
        const input = { version: 1, writes: [{ name: 'data-user-id', value: 'new', datasetKey: 'userId' }] };
        const control = hostAttributeCommitControl(input); input.writes[0].value = 'changed'; expect(control.writes[0].value).toBe('new');
        expect(() => hostAttributeCommitControl({ version: 1, writes: [{ name: 'label', value: 'new' }] })).toThrow();
        expect(() => hostAttributeCommitControl({ version: 1, writes: [control.writes[0], control.writes[0]] })).toThrow();
    });
});
