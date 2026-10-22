import { CemNativeCheckpointDomain } from './internal/runtime-support/checkpoint-domain.js';
import { describe, expect, it } from 'vitest';
import { CemCheckpointPublicationCoordinator } from './checkpoint-publication.js';

const metadata = { instanceId: 'a', declarationIdentity: 'template:1', dataRevision: '1' };
const options = () => ({ runtimeId: 'host-session', generation: 1, policyStamp: 'private:1',
    allowOwner: () => true, seal: (candidate: string) => candidate });
const read = (coordinator: CemCheckpointPublicationCoordinator) => JSON.parse(new TextDecoder().decode(coordinator.current()!.bytes));

describe('all-owner publication checkpoint capture', () => {
    it('combines complete original domains on one heap and erases aggregate bytes on privacy withdrawal', async () => {
        for (const change of ['input', 'policy', 'dispose', 'missing']) {
            const domain = new CemNativeCheckpointDomain('main-thread'); domain.bindNativeRuntime({});
            const a = new CemCheckpointPublicationCoordinator(options()), b = a.fork();
            const first = domain.client(), second = domain.client();
            const adapter = { witness: () => () => true, prepare: async () => ({ owner: {}, current: () => true, discard: () => undefined }) };
            const one = a.register('a', () => '<template>a</template>', () => true, 0, () => [], {
                host: {} as HTMLElement, nativeClient: first, resume: adapter, resolve: () => undefined });
            const two = b.register('b', () => '<template>b</template>', () => true, 0, () => [], {
                host: {} as HTMLElement, nativeClient: second, resume: adapter, resolve: () => undefined });
            one.published(metadata); two.published({ ...metadata, instanceId: 'b' });
            await expect(a.capture()).rejects.toThrow('unavailable'); await expect(b.capture()).rejects.toThrow('unavailable');
            if (change === 'missing') {
                domain.client(); expect(() => CemCheckpointPublicationCoordinator.combine([a, b])).toThrow('coverage');
                a.dispose(); continue;
            }
            const combined = CemCheckpointPublicationCoordinator.combine([a, b]); await combined.capture();
            expect(read(combined).receipt.nativeRuntimes[0].clients).toHaveLength(2);
            expect(read(combined).owners).toHaveLength(2);
            if (change === 'input') { one.input(); expect(combined.current()).toBeUndefined(); combined.dispose(); }
            else if (change === 'policy') { a.setPolicy('withdrawn', () => false); expect(combined.lastCommitted()).toBeUndefined(); }
            else { b.dispose(); expect(combined.lastCommitted()).toBeUndefined(); }
            a.dispose();
        }
    });
    it('propagates parent privacy changes and disposal to independent preview coordinators', async () => {
        const parent = new CemCheckpointPublicationCoordinator(options()), child = parent.fork(), nested = child.fork();
        const owner = child.register('a', () => '<template>private preview source</template>');
        const nestedOwner = nested.register('a', () => '<template>private nested source</template>');
        owner.published(metadata); nestedOwner.published(metadata); await child.capture(); await nested.capture();
        expect(child.lastCommitted()).toBeDefined(); expect(nested.lastCommitted()).toBeDefined();
        parent.setPolicy('withdrawn', () => false);
        expect(child.lastCommitted()).toBeUndefined(); expect(nested.lastCommitted()).toBeUndefined();
        owner.published(metadata); nestedOwner.published(metadata);
        await expect(child.capture()).rejects.toThrow('privacy'); await expect(nested.capture()).rejects.toThrow('privacy');
        parent.dispose(); expect(child.inspectRecovery().disposed).toBe(true); expect(nested.inspectRecovery().disposed).toBe(true);
        expect(() => parent.fork()).toThrow('unavailable');
        const another = new CemCheckpointPublicationCoordinator(options()), detached = another.fork(), sibling = another.fork();
        detached.dispose(); expect(sibling.inspectRecovery().disposed).toBe(false); another.dispose();
        expect(sibling.inspectRecovery().disposed).toBe(true);
    });

    it('inspects incomplete recovery without invoking owners, native work or journal actions', async () => {
        let calls = 0;
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => { calls++; return '<template>private</template>'; }, () => { calls++; return true; });
        const operation = c.operation('a');
        const before = c.inspectRecovery();
        expect(before).toMatchObject({ disposed: false, blocked: false, capturing: false, inFlightAdmissions: 0, unknownOperations: 1,
            owners: [{ instanceId: 'a', tag: null, acknowledged: false, adapterAvailable: false }] });
        expect(calls).toBe(0); expect(c.lastCommitted()).toBeUndefined();
        expect(Object.isFrozen(before) && Object.isFrozen(before.owners) && Object.isFrozen(before.owners[0])).toBe(true);
        a.published(metadata); operation.acknowledge();
        const release = c.admit();
        expect(c.inspectRecovery()).toMatchObject({ inFlightAdmissions: 1, unknownOperations: 0, owners: [{ acknowledged: true }] });
        expect(before.unknownOperations).toBe(1); expect(before.owners[0].acknowledged).toBe(false);
        expect(calls).toBe(0); release(); a.remove(); c.dispose();
        expect(c.inspectRecovery()).toMatchObject({ disposed: true, owners: [] });
    });
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

describe('checkpoint dependency contracts', () => {
    it('pins complete endpoint revisions, copies intent, fences policy and requires explicit removal', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template/>'); a.published(metadata);
        const b = c.register('b', () => '<template/>'); b.published({ ...metadata, instanceId: 'b', dataRevision: 'b:1' });
        let current = true;
        const authority = { runtimeId: 'runtime', generation: 1, policyStamp: 'private:1' };
        const descriptor = { id: 'reference', kind: 'placement' as const,
            requester: { instanceId: 'a', dataRevision: metadata.dataRevision, nodeId: 'a.1' },
            provider: { instanceId: 'b', dataRevision: 'b:1', nodeId: 'b.1' },
            property: 'aria-controls', disposition: 'reacquire' as const };
        // Use the same trusted session authority as the fixture's options.
        authority.runtimeId = options().runtimeId; authority.policyStamp = options().policyStamp;
        const remove = c.relationship(descriptor, () => current, authority);
        descriptor.provider.nodeId = 'mutated';
        await c.capture();
        expect(read(c).receipt.relationships?.[0].provider.nodeId).toBe('b.1');
        current = false; expect(c.current()).toBeUndefined();
        await expect(c.capture()).rejects.toThrow('unavailable');
        current = true; b.published({ ...metadata, instanceId: 'b', dataRevision: 'b:2' });
        await expect(c.capture()).rejects.toThrow('unavailable');
        b.published({ ...metadata, instanceId: 'b', dataRevision: 'b:1' }); await c.capture();
        b.remove(); await expect(c.capture()).rejects.toThrow('unavailable');
        remove(); remove(); await c.capture();
        expect(read(c).receipt.relationships).toBeUndefined();
        const next = { ...descriptor, provider: { ...descriptor.provider, instanceId: 'a', dataRevision: metadata.dataRevision } };
        const release = c.relationship(next, () => true, authority);
        c.setPolicy('private:2', () => true); a.published(metadata);
        await expect(c.capture()).rejects.toThrow('unavailable');
        release(); await c.capture(); c.dispose();
    });
    it('checks dependency freshness again after asynchronous native sealing', async () => {
        let finish!: () => void, current = true;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), seal: text => new Promise(resolve => { finish = () => resolve(text); }) });
        c.register('a', () => '<template/>').published(metadata);
        c.relationship({ id: 'edge', kind: 'placement', requester: { instanceId: 'a', dataRevision: metadata.dataRevision, nodeId: 'a.1' },
            provider: { instanceId: 'a', dataRevision: metadata.dataRevision, nodeId: 'a.2' }, property: 'aria-controls', disposition: 'reacquire' },
            () => current, { runtimeId: options().runtimeId, generation: 1, policyStamp: options().policyStamp });
        const pending = c.capture(); current = false; finish();
        await expect(pending).rejects.toThrow('stale'); expect(c.current()).toBeUndefined(); c.dispose();
    });
});

describe('derived checkpoint dependencies', () => {
    it('requires every derived endpoint and rechecks graph identity after asynchronous exports', async () => {
        let release!: () => void, held = false;
        const edge = { id: 'derived', kind: 'editor-provider' as const, disposition: 'reacquire' as const, property: 'suggestions',
            requester: { instanceId: 'a', dataRevision: '1', nodeId: 'host' }, provider: { instanceId: 'b', dataRevision: '1', nodeId: 'host' } };
        let dependencies: typeof edge[] | undefined = [edge];
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => held ? new Promise(resolve => { release = () => resolve('<template/>'); }) : '<template/>', () => true, 0, () => dependencies);
        a.published(metadata); await expect(c.capture()).rejects.toThrow('unavailable');
        const b = c.register('b', () => '<template/>'); b.published({ ...metadata, instanceId: 'b' });
        await c.capture(); expect(read(c).receipt.relationships[0].id).toBe('derived'); const last = c.lastCommitted()!.bytes;
        held = true; const pending = c.capture(); edge.provider.nodeId = 'different'; release();
        await expect(pending).rejects.toThrow('stale'); expect(c.lastCommitted()!.bytes).toEqual(last); expect(c.current()).toBeUndefined();
        dependencies = undefined; expect(c.current()).toBeUndefined();
        dependencies = [edge]; held = false; await c.capture();
        b.remove(); await expect(c.capture()).rejects.toThrow('unavailable'); c.dispose();
    });
});

describe('dependent owner publication retry', () => {
    it('retries settled acknowledgements after peer publication without skipping another owner or losing input sequences', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        let refreshes = 0, peerChecks = 0;
        const a = c.register('a', () => '<template>unsaved edit</template>', () => true, 0, () => [], {
            host: {} as HTMLElement, resolve: () => undefined,
            refresh: () => { refreshes++; queueMicrotask(() => a.published(metadata)); },
        });
        const b = c.register('b', () => '<template>peer</template>', () => { peerChecks++; return true; });
        a.published(metadata); b.published({ ...metadata, instanceId: 'b' }); await c.capture();
        c.relationship({ id: 'manual-peer', kind: 'placement', property: 'return-focus', disposition: 'reacquire',
            requester: { instanceId: 'a', dataRevision: '1', nodeId: 'host' },
            provider: { instanceId: 'b', dataRevision: '1', nodeId: 'host' } }, () => true,
            { runtimeId: options().runtimeId, generation: 1, policyStamp: 'private:1' });
        await c.capture();
        const last = c.lastCommitted()!.bytes, checksBeforeInput = peerChecks; a.input();
        await expect(c.capture()).rejects.toThrow('unavailable');
        expect(refreshes).toBeGreaterThan(0); expect(peerChecks).toBeGreaterThan(checksBeforeInput);
        expect(c.lastCommitted()!.bytes).toEqual(last);
        await Promise.resolve(); await c.capture();
        expect(read(c).receipt.owners.find((owner: { instanceId: string }) => owner.instanceId === 'a').acceptedEventSequence).toBe(1);
        const beforeRevocation = refreshes;
        c.setPolicy('private:2', () => true);
        await expect(c.capture()).rejects.toThrow('unavailable'); await Promise.resolve();
        expect(refreshes).toBe(beforeRevocation); expect(a.acknowledged).toBe(false);
        a.remove(); b.remove(); c.dispose();
    });
});


it('hands off only current publications and terminal journals, fencing input and policy changes', async () => {
    const c = new CemCheckpointPublicationCoordinator(options());
    const owner = c.register('a', () => '<template>unsaved draft</template>'); owner.published(metadata);
    const operation = c.operation('a'); await c.capture();
    expect(() => c.handoff()).toThrow('operations.unknown');
    operation.acknowledge(); await c.capture();
    const handoff = c.handoff(); expect(handoff.current()).toBe(true);
    const bytes = handoff.retain(); expect(bytes.length).toBeGreaterThan(0);
    owner.input(); expect(handoff.current()).toBe(false); expect(() => handoff.retain()).toThrow('stale');
    expect(c.lastCommitted()).toBeDefined(); c.dispose();
});


describe('publication-backed rendering retirement', () => {
    it('requires the current complete sealed owner set and terminal journal before native cleanup', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client();
        let preparations = 0, terminations = 0;
        domain.retirementEndpoint({ close: async () => undefined, prepare: async () => {
            preparations++; return { schemaVersion: 1, frozen: true, inventoryRevision: 10, frozenRevision: 11, replacementAllowed: false };
        }, abort: async () => undefined, terminate: () => { terminations++; } });
        const c = new CemCheckpointPublicationCoordinator(options());
        const a = c.register('a', () => '<template>unsaved</template>', () => true, 0, () => [],
            { host: {} as HTMLElement, nativeClient: client, resolve: () => undefined });
        a.published(metadata); await c.capture();
        const proof = await c.closeRenderingGeneration();
        await expect(c.retireRenderingGeneration(proof)).rejects.toThrow();
        await c.capture();
        const unknown = c.operation('a'); await c.capture();
        await expect(c.retireRenderingGeneration(proof)).rejects.toThrow('operations.unknown');
        expect(preparations).toBe(0); expect(terminations).toBe(0);
        unknown.acknowledge(); await c.capture();
        const bytes = c.lastCommitted()!.bytes;
        const retired = await c.retireRenderingGeneration(proof);
        expect(retired.checkpoint).toEqual(bytes); expect(preparations).toBe(1); expect(terminations).toBe(1);
        expect(c.current()).toBeUndefined(); expect(c.lastCommitted()!.bytes).toEqual(bytes); c.dispose();
    });
    it('refuses multi-heap and logical-only owners before closing either heap', async () => {
        const c = new CemCheckpointPublicationCoordinator(options());
        c.register('logical', () => '<template/>').published({ ...metadata, instanceId: 'logical' });
        await expect(c.closeRenderingGeneration()).rejects.toThrow('coverage'); c.dispose();
        const d = new CemCheckpointPublicationCoordinator(options());
        const first = new CemNativeCheckpointDomain('worker'), second = new CemNativeCheckpointDomain('worker');
        const a = first.client(), b = second.client();
        for (const [id, client] of [['a', a], ['b', b]] as const)
            d.register(id, () => '<template/>', () => true, 0, () => [],
                { host: {} as HTMLElement, nativeClient: client, resolve: () => undefined }).published({ ...metadata, instanceId: id });
        await expect(d.closeRenderingGeneration()).rejects.toThrow('multiple-generations');
        expect(first.pressureReason()).toBeUndefined(); expect(second.pressureReason()).toBeUndefined();
        d.dispose(); first.remove(a); second.remove(b);
    });
});

describe('original-context native publication rebinding', () => {
    it('binds only an admitted original context from this coordinator, preserving input and changing native sealing', async () => {
        const { registerNativeGeneration, completeNativeRetirement, CemNativeGenerationGroup } = await import('./internal/runtime-support/native-generation.js');
        const { resumeCemCheckpointOwners } = await import('./checkpoint-resume.js');
        const old = new CemNativeCheckpointDomain('worker'), next = new CemNativeCheckpointDomain('worker');
        const oldClient = old.client(), freshClient = next.client();
        let retired = false, context!: import('./checkpoint-resume.js').CemCheckpointResumeContext;
        const c = new CemCheckpointPublicationCoordinator({ ...options(), seal: text => { if (retired) throw Error('retired sealer'); return text; } });
        const foreign = new CemCheckpointPublicationCoordinator(options());
        const other = foreign.register('a', () => '<template/>', () => true, 3, () => [], { host: {} as HTMLElement, nativeClient: oldClient, resolve: () => undefined });
        const owner = c.register('a', () => '<template>canonical source</template>', () => true, 3, () => [], {
            host: {} as HTMLElement, nativeClient: oldClient, resolve: () => undefined,
            resume: { witness: () => () => true, prepare: async ctx => {
                context = ctx;
                expect(() => owner.resume(ctx, freshClient, text => text)).toThrow('resume');
                return { owner: {}, current: () => true, discard: () => undefined };
            } },
        });
        owner.published(metadata); await c.capture(); const ticket = c.prepareResume(), saved = c.current()!;
        const participant = registerNativeGeneration({}, old.issuedIdentity(), async () => ({ identity: {}, runtimeId: next.issuedIdentity(),
            runtime: {}, current: () => !!next.identity(), discard: () => next.revoke() }));
        old.revoke(); retired = true;
        const admission = await new CemNativeGenerationGroup([participant]).allocate([completeNativeRetirement(participant, saved.bytes)]);
        const result = await resumeCemCheckpointOwners(admission, [ticket]);
        expect(() => owner.resume({ ...context }, freshClient, text => text)).toThrow('resume');
        expect(() => other.resume(context, freshClient, text => text)).toThrow('resume');
        expect(() => owner.resume(context, oldClient, text => text)).toThrow('resume');
        owner.input(); owner.resume(context, freshClient, text => text);
        expect(c.current()).toBeUndefined();
        expect(() => owner.resume(context, freshClient, text => text)).toThrow('resume');
        owner.published({ ...metadata, dataRevision: '2' }); await c.capture();
        expect(read(c).receipt.owners[0]).toMatchObject({ acceptedEventSequence: 4, dataRevision: '2' });
        expect(read(c).receipt.nativeRuntimes[0].runtimeId).toBe(next.issuedIdentity());
        result.revoke(); expect(c.current()).toBeUndefined();
        expect(() => owner.resume(context, freshClient, text => text)).toThrow('resume');
        c.dispose(); foreign.dispose();
    });
});
