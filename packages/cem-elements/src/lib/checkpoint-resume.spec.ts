import { describe, expect, it, vi } from 'vitest';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';
import { issueCheckpointResumeTicket, revokeCheckpointResumeTicket, resumeCemCheckpointOwners, type CemCheckpointResumeAdapter, type CemCheckpointResumeContext } from './checkpoint-resume.js';
import { CemNativeGenerationGroup, registerNativeGeneration, completeNativeRetirement } from './internal/runtime-support/native-generation.js';
import type { CemStoredCheckpoint } from './committed-checkpoint.js';
import { CemNativeCheckpointDomain } from './internal/runtime-support/checkpoint-domain.js';
const receipt = { runtimeId: 'session', generation: 1, commitSequence: 1, policyStamp: 'private:1', owners: [
    { instanceId: 'editor', declarationIdentity: 'template:1', dataRevision: '2', acceptedEventSequence: 3 },
    { instanceId: 'tree', declarationIdentity: 'template:tree', dataRevision: '4', acceptedEventSequence: 0 },
], operations: [] };
const saved = (): CemStoredCheckpoint => ({ receipt: structuredClone(receipt), bytes: new TextEncoder().encode(JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt,
    owners: receipt.owners.map(owner => ({ instanceId: owner.instanceId, island: `<template>${owner.instanceId} é😀</template>` })) })) });
async function fixture(adjust?: (id: string, context: CemCheckpointResumeContext) => Promise<void>, hooks?: (id: string, context: CemCheckpointResumeContext) => object) {
    let valid = true, alive = true;
    const contexts: CemCheckpointResumeContext[] = [], discarded: string[] = [];
    const checkpoint = saved(), previous = crypto.randomUUID();
    const participant = registerNativeGeneration({}, previous, async gate => ({ identity: {}, runtimeId: crypto.randomUUID(),
        runtime: { invoke: () => { if (!gate()) throw Error('closed'); } }, current: () => alive, discard: () => { alive = false; } }));
    const group = new CemNativeGenerationGroup([participant]);
    const adapter = (id: string): CemCheckpointResumeAdapter => ({ witness: () => () => valid,
        async prepare(context) {
            contexts.push(context); await adjust?.(id, context);
            return { owner: { use: () => { if (!context.active()) throw Error('inactive'); return context.island; } },
                current: () => alive, discard: () => { discarded.push(id); }, ...hooks?.(id, context) };
        } });
    const ticket = issueCheckpointResumeTicket(previous, checkpoint, () => valid, new Map(receipt.owners.map(owner => [owner.instanceId, adapter(owner.instanceId)])));
    const admission = await group.allocate([completeNativeRetirement(participant, checkpoint.bytes)]);
    return { ticket, admission, contexts, discarded, checkpoint, invalidate: () => { valid = false; }, alive: () => alive };
}
describe('fresh generation semantic owner transaction', () => {
    it('requires all preparations before opening one owner gate, preserves opaque islands and fences revocation', async () => {
        const f = await fixture(async (_id, context) => { expect(context.active()).toBe(false); });
        f.admission.generations[0].checkpoint.fill(9);
        const result = await resumeCemCheckpointOwners(f.admission, [f.ticket]);
        expect(result.owners.map(owner => owner.instanceId)).toEqual(['editor', 'tree']);
        for (const owner of result.owners) expect((owner.owner as { use(): string }).use()).toContain('é😀');
        expect(f.contexts.every(context => context.active())).toBe(true);
        f.admission.revoke(); expect(f.discarded).toEqual(['editor', 'tree']);
        expect(f.contexts.every(context => !context.active() && context.signal.aborted)).toBe(true);
        result.revoke(); expect(f.discarded).toHaveLength(2);
        await expect(resumeCemCheckpointOwners(f.admission, [f.ticket])).rejects.toThrow('ticket');
    });
    it('binds every participating owner before renewing a provider, independent of caller order', async () => {
        const events: string[] = [];
        const f = await fixture(undefined, id => ({ bind: () => events.push(`bind:${id}`), bound: () => events.push(`bound:${id}`),
            renew: async () => { expect(events.slice(0, 2)).toEqual(['bind:editor', 'bind:tree']); events.push(`renew:${id}`); } }));
        const resumed = await resumeCemCheckpointOwners(f.admission, [f.ticket]);
        expect(events).toEqual([]);
        await f.contexts[1].renewOwners(); await f.contexts[0].renewOwners();
        expect(events).toEqual(['bind:editor', 'bind:tree', 'bound:editor', 'bound:tree', 'renew:editor', 'renew:tree']);
        resumed.revoke();
    });
    it('rejects late host edits before any peer binds and revokes every fresh owner', async () => {
        const bind = vi.fn(), f = await fixture(undefined, () => ({ bind }));
        await resumeCemCheckpointOwners(f.admission, [f.ticket]); f.invalidate();
        await expect(f.contexts[1].renewOwners()).rejects.toThrow('stale');
        expect(bind).not.toHaveBeenCalled(); expect(f.alive()).toBe(false); expect(f.discarded).toEqual(['editor', 'tree']);
    });
    it('fences partial renewal failures and never retries a provider action', async () => {
        const renew = vi.fn(async () => { throw Error('withdrawn source policy'); });
        const f = await fixture(undefined, id => ({ renew: id === 'tree' ? renew : async () => undefined }));
        await resumeCemCheckpointOwners(f.admission, [f.ticket]);
        await expect(f.contexts[0].renewOwners()).rejects.toThrow('withdrawn');
        await expect(f.contexts[0].renewOwners()).rejects.toThrow('stale');
        expect(renew).toHaveBeenCalledTimes(1); expect(f.alive()).toBe(false);
    });
    it('checks every peer after asynchronous renewal and refuses intervening edits', async () => {
        let edited = false;
        const f = await fixture(undefined, id => ({ renew: async () => { if (id === 'tree') edited = true; },
            renewed: () => { if (id === 'editor' && edited) throw Error('late editor edit'); } }));
        await resumeCemCheckpointOwners(f.admission, [f.ticket]);
        await expect(f.contexts[0].renewOwners()).rejects.toThrow('late editor edit'); expect(f.alive()).toBe(false);
    });
    it('aborts stalled provider renewal immediately on group revocation without retry', async () => {
        let finish!: () => void;
        const renew = vi.fn(() => new Promise<void>(resolve => { finish = resolve; }));
        const f = await fixture(undefined, () => ({ renew })); await resumeCemCheckpointOwners(f.admission, [f.ticket]);
        const pending = f.contexts[0].renewOwners(); await vi.waitFor(() => expect(renew).toHaveBeenCalledTimes(1));
        f.admission.revoke(); await expect(pending).rejects.toThrow('revoked');
        finish(); expect(f.discarded).toEqual(['editor', 'tree']);
    });
    it('parks all presentations before provider claims and releases each hold on ticket revocation', () => {
        const events: string[] = [];
        const adapters = new Map(receipt.owners.map(owner => [owner.instanceId, {
            witness: () => () => true, prepare: vi.fn(),
            park: () => { events.push(`park:${owner.instanceId}`); return () => { events.push(`unpark:${owner.instanceId}`); }; },
            parkProvider: () => { events.push(`provider:${owner.instanceId}`); return () => { events.push(`release:${owner.instanceId}`); }; },
        }]));
        const ticket = issueCheckpointResumeTicket('runtime', saved(), () => true, adapters);
        expect(events).toEqual(['park:editor', 'park:tree', 'provider:editor', 'provider:tree']);
        revokeCheckpointResumeTicket(ticket); revokeCheckpointResumeTicket(ticket);
        expect(events.slice(4)).toEqual(['release:tree', 'release:editor', 'unpark:tree', 'unpark:editor']);
    });
    it('refuses copied tickets/admissions without consuming or revoking the original group', async () => {
        const f = await fixture();
        await expect(resumeCemCheckpointOwners(f.admission, [{ ...f.ticket }])).rejects.toThrow('ticket');
        await expect(resumeCemCheckpointOwners({ ...f.admission }, [f.ticket])).rejects.toThrow('admission');
        expect(f.alive()).toBe(true); expect(f.contexts).toHaveLength(0);
        const result = await resumeCemCheckpointOwners(f.admission, [f.ticket]); result.revoke();
    });
    it('refuses stale input after asynchronous preparation and discards all peers without opening a gate', async () => {
        let invalidate = () => undefined;
        const f = await fixture(async id => { if (id === 'tree') invalidate(); }); invalidate = f.invalidate;
        await expect(resumeCemCheckpointOwners(f.admission, [f.ticket])).rejects.toThrow('stale');
        expect(f.discarded).toEqual(['editor', 'tree']); expect(f.alive()).toBe(false);
        expect(f.contexts.every(context => !context.active())).toBe(true); expect(f.checkpoint.bytes.byteLength).toBeGreaterThan(0);
    });
    it('times out, aborts preparation, and discards a late owner exactly once', async () => {
        let finish!: () => void;
        const f = await fixture(async () => new Promise<void>(resolve => { finish = resolve; }));
        await expect(resumeCemCheckpointOwners(f.admission, [f.ticket], 10)).rejects.toThrow('timeout');
        expect(f.contexts[0].signal.aborted).toBe(true); expect(f.alive()).toBe(false);
        finish(); await vi.waitFor(() => expect(f.discarded).toEqual(['editor']));
        expect(f.contexts).toHaveLength(1);
    });
    it('cancels externally revoked preparation and cleans late completion without replay', async () => {
        let finish!: () => void;
        const f = await fixture(async () => new Promise<void>(resolve => { finish = resolve; }));
        const pending = resumeCemCheckpointOwners(f.admission, [f.ticket]);
        await vi.waitFor(() => expect(f.contexts).toHaveLength(1)); f.admission.revoke(); finish();
        await expect(pending).rejects.toThrow(/revoked|stale/);
        expect(f.discarded).toEqual(['editor']); expect(f.contexts[0].active()).toBe(false);
    });
    it('requires exact checkpoint bytes and complete physical coverage before preparing owners', async () => {
        const f = await fixture(), foreign = await fixture();
        await expect(resumeCemCheckpointOwners(f.admission, [foreign.ticket])).rejects.toThrow('checkpoint');
        expect(f.contexts).toHaveLength(0); expect(f.alive()).toBe(false); foreign.admission.revoke();
    });
    it('does not invoke caller iterators, array methods or accessors', async () => {
        const f = await fixture(), slots = [f.ticket], get = vi.fn(() => f.ticket);
        Object.defineProperty(slots, '0', { get });
        await expect(resumeCemCheckpointOwners(f.admission, slots)).rejects.toThrow('ticket'); expect(get).not.toHaveBeenCalled();
        const originals = [f.ticket]; originals[Symbol.iterator] = () => { throw Error('iterator'); };
        const resumed = await resumeCemCheckpointOwners(f.admission, originals); resumed.revoke();
    });
    it('binds exact private bytes even for a matching runtime and refuses mismatched state', async () => {
        const f = await fixture(), changed = saved(); changed.bytes = new TextEncoder().encode(new TextDecoder().decode(changed.bytes).replace('editor é', 'edited é'));
        const adapter: CemCheckpointResumeAdapter = { witness: () => () => true, prepare: vi.fn() };
        const ticket = issueCheckpointResumeTicket(f.admission.generations[0].previousRuntimeId, changed, () => true,
            new Map(receipt.owners.map(owner => [owner.instanceId, adapter])));
        await expect(resumeCemCheckpointOwners(f.admission, [ticket])).rejects.toThrow('checkpoint');
        expect(adapter.prepare).not.toHaveBeenCalled(); expect(f.alive()).toBe(false);
    });
    it('erases an invalidated ticket and aborts an uncooperative preparation immediately', async () => {
        let finish!: () => void;
        const f = await fixture(async () => new Promise<void>(resolve => { finish = resolve; }));
        const pending = resumeCemCheckpointOwners(f.admission, [f.ticket]);
        const rejected = expect(pending).rejects.toThrow('revoked');
        await vi.waitFor(() => expect(f.contexts).toHaveLength(1)); revokeCheckpointResumeTicket(f.ticket);
        await rejected; expect(f.alive()).toBe(false); expect(f.contexts[0].signal.aborted).toBe(true);
        finish(); await vi.waitFor(() => expect(f.discarded).toEqual(['editor']));
    });
    it('does not start preparation after immediate external revocation', async () => {
        const f = await fixture();
        const pending = resumeCemCheckpointOwners(f.admission, [f.ticket]); f.admission.revoke();
        await expect(pending).rejects.toThrow(/revoked|stale/); expect(f.contexts).toHaveLength(0);
    });
    it('requires every original ticket for a multi-generation group before preparing a peer', async () => {
        for (const missing of [false, true]) {
            const contexts: CemCheckpointResumeContext[] = [], records = [], originals = [], retired = [], alive = [true, true];
            for (let index = 0; index < 2; index++) {
                const previous = crypto.randomUUID(), checkpoint = saved();
                const participant = registerNativeGeneration({}, previous, async () => ({ identity: {}, runtimeId: crypto.randomUUID(), runtime: {},
                    current: () => alive[index], discard: () => { alive[index] = false; } }));
                records.push(participant); retired.push(completeNativeRetirement(participant, checkpoint.bytes));
                const adapter: CemCheckpointResumeAdapter = { witness: () => () => true, prepare: async context => {
                    contexts.push(context); expect(contexts.every(value => !value.active())).toBe(true);
                    return { owner: {}, current: () => alive[index], discard: () => undefined };
                } };
                originals.push(issueCheckpointResumeTicket(previous, checkpoint, () => true, new Map(receipt.owners.map(owner => [owner.instanceId, adapter]))));
            }
            const admission = await new CemNativeGenerationGroup(records).allocate(retired);
            if (missing) {
                await expect(resumeCemCheckpointOwners(admission, [originals[0]])).rejects.toThrow('coverage');
                expect(contexts).toHaveLength(0); expect(alive).toEqual([false, false]);
            } else {
                const result = await resumeCemCheckpointOwners(admission, originals.reverse());
                expect(result.owners).toHaveLength(4); expect(contexts.every(value => value.active())).toBe(true); result.revoke();
            }
        }
    });
    it('refuses unknown outcomes before issuing a ticket or acquiring fresh source/widget authority', () => {
        const unknown = saved(); unknown.receipt.operations.push({ instanceId: 'editor', operationId: 'save:1', outcome: 'unknown' } as never);
        expect(() => issueCheckpointResumeTicket('runtime', unknown, () => true, new Map())).toThrow('unknown');
    });
    it('requires shared adapters and fences publication input, owner removal and policy changes', async () => {
        for (const change of ['input', 'remove', 'policy', 'dispose', 'native'] as const) {
            const domain = new CemNativeCheckpointDomain('main-thread'); domain.bindNativeRuntime({});
            const client = domain.client(), c = new CemCheckpointPublicationCoordinator({ runtimeId: 'host', generation: 1, policyStamp: 'private:1', allowOwner: () => true, seal: text => text });
            const adapter = { witness: () => () => true, prepare: vi.fn() };
            const owner = c.register('editor', () => '<template>draft</template>', () => true, 3, () => [], {
                host: {} as HTMLElement, nativeClient: client, resolve: () => undefined, resume: adapter });
            owner.published(receipt.owners[0]); await c.capture(); const ticket = c.prepareResume();
            const participant = registerNativeGeneration({}, domain.issuedIdentity(), async () => ({ identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, current: () => true, discard: vi.fn() }));
            const group = new CemNativeGenerationGroup([participant]), committed = c.current();
            if (!committed) throw Error('missing current fixture checkpoint');
            const retired = completeNativeRetirement(participant, committed.bytes);
            if (change === 'input') owner.input(); else if (change === 'remove') owner.remove(); else if (change === 'policy') c.setPolicy('private:2', () => true); else if (change === 'dispose') c.dispose(); else domain.revoke();
            const admission = await group.allocate([retired]);
            if (change === 'native') {
                // Retirement invalidates native capture without making the
                // original host-only resume witness stale by itself.
                adapter.prepare.mockRejectedValueOnce(Error('reached fresh adapter'));
                await expect(resumeCemCheckpointOwners(admission, [ticket])).rejects.toThrow('reached fresh adapter');
            } else {
                await expect(resumeCemCheckpointOwners(admission, [ticket])).rejects.toThrow('ticket'); expect(adapter.prepare).not.toHaveBeenCalled();
            }
            c.dispose();
        }
        const c = new CemCheckpointPublicationCoordinator({ runtimeId: 'host', generation: 1, policyStamp: 'p', allowOwner: () => true, seal: text => text });
        c.register('editor', () => '<template/>').published(receipt.owners[0]); await c.capture();
        expect(() => c.prepareResume()).toThrow(); c.dispose();
    });
});
