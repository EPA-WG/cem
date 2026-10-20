import { describe, expect, it } from 'vitest';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';

const metadata = { instanceId: 'a', declarationIdentity: 'template:1', dataRevision: '1' };
const options = () => ({ runtimeId: 'host-session', generation: 1, policyStamp: 'private:1',
    allowOwner: () => true, seal: (candidate: string) => candidate });
const read = (coordinator: CemCheckpointPublicationCoordinator) => JSON.parse(new TextDecoder().decode(coordinator.current()!.bytes));

describe('all-owner publication checkpoint capture', () => {
    it('requires every publication and drained admission, then exports all islands together', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template>unsaved a</template>');
        const b = c.register('b', () => '<template>unsaved b</template>');
        a.published(metadata); await expect(c.capture()).rejects.toThrow('unavailable');
        b.published({ ...metadata, instanceId: 'b' });
        const release = c.admit(); await expect(c.capture()).rejects.toThrow('unavailable'); release();
        await c.capture(); expect(read(c).owners.map((o: { island: string }) => o.island)).toEqual(['<template>unsaved a</template>', '<template>unsaved b</template>']);
        a.input(); expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined();
        a.published({ ...metadata, dataRevision: '2' }); await c.capture();
        expect(read(c).receipt.owners[0]).toMatchObject({ dataRevision: '2', acceptedEventSequence: 1 });
        b.remove(); expect(c.current()).toBeUndefined();
        await c.capture(); expect(read(c).owners).toHaveLength(1);
        a.remove();
        const reconnected = c.register('a', () => '<template>reconnected</template>', () => true, 42);
        reconnected.published(metadata); await c.capture();
        expect(read(c).receipt.owners[0].acceptedEventSequence).toBe(42);
        expect(() => a.published(metadata)).toThrow('stale');
        c.dispose();
    });
    it('fences a pending seal on input and coalesces repeated input without retaining payloads', async () => {
        let finish!: (text: string) => void;
        let held = false;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), seal: text => held ? new Promise(resolve => { finish = () => resolve(text); }) : text });
        const a = c.register('a', () => '<template>current</template>'); a.published(metadata); await c.capture();
        held = true; const pending = c.capture();
        for (let i = 0; i < 1000; i++) a.input();
        finish(''); await expect(pending).rejects.toThrow('stale');
        expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined();
        held = false; a.published({ ...metadata, dataRevision: 'latest' }); await c.capture();
        expect(read(c).receipt.owners[0].acceptedEventSequence).toBe(1000); c.dispose();
    });
    it('refuses privacy/partial exports, erases on policy changes and rejects stale owner handles', async () => {
        let allowed = false, exportCalls = 0;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), allowOwner: () => allowed });
        const a = c.register('a', () => { exportCalls++; return '<template>private</template>'; });
        a.published(metadata); await expect(c.capture()).rejects.toThrow('privacy'); expect(exportCalls).toBe(0);
        allowed = true; await c.capture();
        c.setPolicy('private:2', () => false); expect(c.lastCommitted()).toBeUndefined();
        await expect(c.capture()).rejects.toThrow('unavailable');
        a.published(metadata); await expect(c.capture()).rejects.toThrow('privacy');
        a.remove(); const replacement = c.register('a', () => '<template>new</template>');
        expect(() => a.published(metadata)).toThrow('stale'); replacement.remove(); c.dispose();
    });
    it('checks live mutations before and after sealing, and refuses oversized/missing owner exports', async () => {
        let valid = true, island: string | undefined = '<template>good</template>';
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => island, () => valid); a.published(metadata); await c.capture();
        valid = false; expect(c.current()).toBeUndefined(); await expect(c.capture()).rejects.toThrow('unavailable');
        valid = true; a.published(metadata); island = undefined; await expect(c.capture()).rejects.toThrow('unavailable');
        island = 'é'.repeat(600_000); await expect(c.capture()).rejects.toThrow('limit');
        expect(c.lastCommitted()).toBeDefined(); c.dispose();
    });
    it('coalesces a newer automatic publication after a held seal and never rolls back', async () => {
        let release!: () => void, calls = 0;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), automatic: true, seal: text => {
            calls++; return calls === 1 ? new Promise(resolve => { release = () => resolve(text); }) : text;
        } });
        const a = c.register('a', () => '<template>latest</template>');
        a.published(metadata); await Promise.resolve();
        a.input(); a.published({ ...metadata, dataRevision: 'new' }); release();
        for (let i = 0; i < 20 && !c.current(); i++) await Promise.resolve();
        expect(calls).toBe(2); expect(read(c).receipt.owners[0].dataRevision).toBe('new'); c.dispose();
    });
    it('fails closed on owner/admission overflow rather than silently omitting state', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        for (let i = 0; i < 256; i++) c.register(`owner-${i}`, () => '<template/>').published({ ...metadata, instanceId: `owner-${i}` });
        await c.capture();
        expect(() => c.register('overflow', () => '<template/>')).toThrow('owners');
        expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined(); c.dispose();
        const d = new CemCheckpointPublicationCoordinator(options());
        const release = Array.from({ length: 256 }, () => d.admit());
        expect(() => d.admit()).toThrow('limit'); release.forEach(finish => finish());
        await expect(d.capture()).rejects.toThrow('unavailable'); d.dispose();
    });

    it('records unknown external outcomes before delivery and fences stale completions without replay', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template>state</template>'); a.published(metadata); await c.capture();
        const operation = c.operation('a');
        expect(c.current()).toBeUndefined(); await c.capture();
        expect(read(c).receipt.operations).toEqual([{ instanceId: 'a', operationId: operation.operationId, outcome: 'unknown' }]);
        operation.acknowledge(); await c.capture();
        expect(read(c).receipt.operations[0].outcome).toBe('acknowledged');
        const late = c.operation('a'); a.remove(); late.acknowledge();
        const replacement = c.register('a', () => '<template>fresh</template>'); replacement.published(metadata);
        await c.capture(); expect(read(c).receipt.operations.find((o: { operationId: string }) => o.operationId === late.operationId).outcome).toBe('unknown');
        const policyLate = c.operation('a');
        c.setPolicy('private:2', () => true); policyLate.acknowledge(); replacement.published(metadata);
        await c.capture(); expect(read(c).receipt.operations.find((o: { operationId: string }) => o.operationId === policyLate.operationId).outcome).toBe('unknown');
        expect(c.operation('a').operationId).not.toBe(late.operationId); c.dispose();
    });
    it('keeps pending outcomes when an owner disappears and refuses journal overflow', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template>state</template>'); a.published(metadata);
        c.operation('a'); a.remove();
        c.register('b', () => '<template>b</template>').published({ ...metadata, instanceId: 'b' });
        await expect(c.capture()).rejects.toThrow('unavailable'); c.dispose();
        const d = new CemCheckpointPublicationCoordinator(options());
        d.register('a', () => '<template/>').published(metadata);
        for (let i = 0; i < 256; i++) d.operation('a').acknowledge();
        await d.capture(); expect(() => d.operation('a')).toThrow('limit');
        expect(d.current()).toBeUndefined(); expect(d.lastCommitted()).toBeDefined(); d.dispose();
    });

    it('pins owner exports across an asynchronous native validation boundary', async () => {
        let release!: (value: string) => void, held = false, newExports = 0;
        const c = new CemCheckpointPublicationCoordinator(options());
        c.register('a', () => held ? new Promise<string>(done => { release = done; }) : '<template>a</template>').published(metadata);
        await c.capture(); held = true; const pending = c.capture();
        c.register('new', () => { newExports++; return '<template>new</template>'; }).published({ ...metadata, instanceId: 'new' });
        release('<template>old a</template>'); await expect(pending).rejects.toThrow('stale');
        expect(newExports).toBe(0); expect(c.current()).toBeUndefined(); expect(c.lastCommitted()).toBeDefined(); c.dispose();
    });

    it('stages terminal retirement until an all-owner seal succeeds, retaining failed attempts', async () => {
        let fail=false;
        const c=new CemCheckpointPublicationCoordinator({ ...options(), seal: text => { if(fail) throw new Error('held failure'); return text; } });
        const a=c.register('a',()=>'<template>state</template>'); a.published(metadata);
        const op=c.operation('a'); op.acknowledge(); await c.capture();
        const authority={runtimeId:'host-session',generation:1,policyStamp:'private:1'};
        c.retireOperations([op.operationId], authority); expect(c.current()).toBeUndefined(); fail=true;
        await expect(c.capture()).rejects.toThrow('held failure');
        expect(c.lastCommitted()!.receipt.operations).toHaveLength(1);
        fail=false; await c.capture(); expect(read(c).receipt.operations).toHaveLength(0);
        const before=c.current(); op.acknowledge(); expect(c.current()).toEqual(before);
        expect(()=>c.retireOperations([op.operationId],authority)).toThrow(); c.dispose();
    });
    it('requires explicit current authority to reconcile orphan outcomes before retirement', async () => {
        const c=new CemCheckpointPublicationCoordinator(options());
        const a=c.register('a',()=>'<template>a</template>'); a.published(metadata);
        const op=c.operation('a'); a.remove(); op.acknowledge();
        const b=c.register('b',()=>'<template>b</template>'); b.published({...metadata,instanceId:'b'});
        const authority={runtimeId:'host-session',generation:1,policyStamp:'private:1'};
        expect(()=>c.retireOperations([op.operationId],authority)).toThrow('unknown');
        expect(()=>c.reconcileOperation(op.operationId,'not-applied',{...authority,generation:2})).toThrow('authority');
        c.setPolicy('private:2',()=>true);
        expect(()=>c.reconcileOperation(op.operationId,'not-applied',authority)).toThrow('authority');
        const current={...authority,policyStamp:'private:2'};
        c.reconcileOperation(op.operationId,'not-applied',current);
        c.retireOperations([op.operationId],current);
        c.register('active',()=>'<template>active</template>').published({...metadata,instanceId:'active'});
        // Policy changes require every surviving owner to publish anew.
        await expect(c.capture()).rejects.toThrow('unavailable');
        b.published({...metadata,instanceId:'b'}); await c.capture();
        expect(read(c).receipt.operations).toHaveLength(0); c.dispose();
    });
    it('keeps identities non-reusable through bounded repeated retirement and fences held seals', async () => {
        const c=new CemCheckpointPublicationCoordinator(options());
        const a=c.register('a',()=>'<template>a</template>'); a.published(metadata);
        const authority={runtimeId:'host-session',generation:1,policyStamp:'private:1'};
        let first='';
        for(let n=0;n<300;n++) {
            const op=c.operation('a'); if(!n) first=op.operationId;
            else expect(op.operationId).not.toBe(first);
            op.acknowledge(); c.retireOperations([op.operationId],authority); await c.capture();
            expect(read(c).receipt.operations).toHaveLength(0);
        }
        expect(()=>c.reconcileOperation(first,'acknowledged',authority)).toThrow(); c.dispose();
    });

    it('retains retirement intent and old bytes when input or cancellation fences an async seal', async () => {
        let held=false, release!:(value:string)=>void, text='';
        const c=new CemCheckpointPublicationCoordinator({...options(),seal: value => held ? new Promise<string>(done=>{release=done;text=value;}) : value});
        const a=c.register('a',()=>'<template>state</template>'); a.published(metadata);
        const op=c.operation('a'); op.acknowledge(); await c.capture();
        const authority={runtimeId:'host-session',generation:1,policyStamp:'private:1'};
        c.retireOperations([op.operationId],authority); held=true; const pending=c.capture();
        a.input(); a.published({...metadata,dataRevision:'2'}); release(text);
        await expect(pending).rejects.toThrow('stale'); expect(c.lastCommitted()!.receipt.operations).toHaveLength(1);
        const abort=new AbortController(), cancelled=c.capture(abort.signal); abort.abort(); release(text);
        await expect(cancelled).rejects.toThrow('stale'); expect(c.lastCommitted()!.receipt.operations).toHaveLength(1);
        held=false; await c.capture(); expect(read(c).receipt.operations).toHaveLength(0); c.dispose();
    });

});
