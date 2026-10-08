import { expect, it } from 'vitest';
import { CemPlacementAuthority } from './element-placement-authority.js';
import type { CemElementReferenceInputs } from './element-reference-inputs.js';
const source = { bundle: new ArrayBuffer(1), primarySourceId: 1, context: true };
const inputs = (): CemElementReferenceInputs => ({ kind: 'cem-element-reference-inputs-v1', requesting: 0, sources: [source], bindings: [], grants: [] });
function fixture() {
    const authority = new CemPlacementAuthority<string>();
    let ready = true, reserved = false;
    const placement = authority.register({ producer: 'owner', path: [0], revision: '1', id: 'owner-ref-0', source, select: 'input.children' }, {
        ready: () => ready, reserve: () => { const old = reserved; reserved = true; return () => { reserved = old; }; },
    });
    const revoke = authority.grant('consumer', placement, ['commandfor']);
    const use = { token: placement.token, producer: 'owner', revision: '1', id: 'owner-ref-0', renderNodeId: 'button', attribute: 'commandfor' };
    return { authority, placement, revoke, use, unavailable: () => { ready = false; }, reserved: () => reserved };
}
it('keeps source crossing authority separate and rejects copied or stale snapshots', () => {
    const f = fixture(); const prepared = f.authority.prepare('consumer', inputs(), 'root');
    expect(prepared.grants).toEqual([]);
    expect(prepared.placements?.admissions).toHaveLength(1);
    expect(() => f.authority.check({ ...prepared }, [f.use])).toThrow();
    f.unavailable(); expect(() => f.authority.check(prepared, [f.use])).toThrow();
});
it('admits editor-for only through its own property grant', () => {
    const f = fixture();
    const use = { ...f.use, attribute: 'editor-for' };
    expect(() => f.authority.check(f.authority.prepare('consumer', inputs(), 'root'), [use])).toThrow();
    const revoke = f.authority.grant('consumer', f.placement, ['editor-for']);
    const snapshot = f.authority.prepare('consumer', inputs(), 'root');
    expect(() => f.authority.check(snapshot, [use])).not.toThrow();
    revoke(); expect(() => f.authority.check(snapshot, [use])).toThrow();
});
it('reserves IDs once, retains live dependencies and rolls reservations back on failure', () => {
    const f = fixture(); const prepared = f.authority.prepare('consumer', inputs(), 'root');
    expect(() => f.authority.publish(prepared, [f.use], () => { throw new Error('publication'); })).toThrow('publication');
    expect(f.reserved()).toBe(false);
    let invalidations = 0;
    f.authority.publish(prepared, [f.use, f.use], () => undefined, () => { invalidations++; });
    expect(f.reserved()).toBe(true); f.revoke(); expect(invalidations).toBe(1);
    expect(() => f.authority.check(prepared, [f.use])).toThrow();
});
it('admits prepared placements only in their live transaction and rolls back failed groups', () => {
    const authority = new CemPlacementAuthority<string>();
    const transaction = authority.transaction(['owner', 'consumer'], { owner: '2', consumer: '2' });
    let committed = false;
    const placement = authority.register({ producer: 'owner', revision: '2', path: [0], id: 'owner-ref-0', source, select: 'input.children' }, {
        ready: (_root, prepared) => prepared || committed, reserve: () => () => undefined,
    }, transaction);
    authority.grant('consumer', placement, ['commandfor']);
    expect(authority.prepare('consumer', inputs(), 'root').placements?.admissions).toHaveLength(0);
    const prepared = authority.prepare('consumer', inputs(), 'root', transaction);
    const use = { token: placement.token, producer: 'owner', revision: '2', id: 'owner-ref-0', renderNodeId: 'button', attribute: 'commandfor', transaction: transaction.token };
    expect(() => authority.publish(prepared, [use], () => undefined)).toThrow();
    const owner = { requester: 'owner', current: () => true, commit: () => { committed = true; }, rollback: () => { committed = false; } };
    const consumer = { requester: 'consumer', inputs: prepared, uses: [use], current: () => true, commit: () => undefined, rollback: () => undefined };
    expect(() => authority.publishGroup(transaction, [owner, { ...consumer, commit: () => { throw new Error('group'); } }])).toThrow('group');
    expect(committed).toBe(false);
    authority.publishGroup(transaction, [owner, consumer]); expect(committed).toBe(true);
    expect(() => authority.publishGroup(transaction, [owner, consumer])).toThrow();
    const fresh = authority.prepare('consumer', inputs(), 'root'); expect(fresh.placements?.preparedTransaction).toBeUndefined();
    expect(fresh.placements?.admissions).toHaveLength(1);
});
it('continues monitoring remaining relationships after one placement loses its grant', () => {
    const f = fixture();
    const second = f.authority.register({ producer: 'other', revision: '1', path: [0], id: 'other-ref-0', source, select: 'input.children' }, {
        ready: () => true, reserve: () => () => undefined,
    });
    const revoke = f.authority.grant('consumer', second, ['commandfor']);
    const otherUse = { ...f.use, token: second.token, producer: 'other', id: 'other-ref-0', renderNodeId: 'other-button' };
    const snapshot = f.authority.prepare('consumer', inputs(), 'root');
    const lost: string[] = [];
    f.authority.publish(snapshot, [f.use, otherUse], () => undefined, uses => lost.push(...uses.map(use => use.token)));
    f.revoke(); revoke(); expect(lost).toEqual([f.placement.token, second.token]);
});
it('rolls back every publication and reservation when activation fails', () => {
    const f = fixture(); const transaction = f.authority.transaction(['owner', 'consumer'], { owner: '1', consumer: '1' });
    const snapshot = f.authority.prepare('consumer', inputs(), 'root', transaction);
    const live = new Set<string>();
    expect(() => f.authority.publishGroup(transaction, ['owner', 'consumer'].map(requester => ({
        requester, current: () => true, ...(requester === 'consumer' ? { inputs: snapshot, uses: [f.use] } : {}),
        commit: () => { live.add(requester); }, rollback: () => { live.delete(requester); },
        activate: () => { if (requester === 'consumer') throw new Error('activation failed'); },
    })))).toThrow('activation failed');
    expect(live.size).toBe(0); expect(f.reserved()).toBe(false); transaction.cancel();
});
