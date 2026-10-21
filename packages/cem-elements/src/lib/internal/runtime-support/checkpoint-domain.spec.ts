import { describe, expect, it, vi } from 'vitest';
import { CemNativeCheckpointDomain, nativeCheckpointAdmission, nativeCheckpointIdentity, nativeCheckpointSnapshot, nativeCheckpointRequirements, nativeCheckpointWitnessSource, nativeCheckpointSourceProof, nativeCheckpointFailureFence, nativeCheckpointWork, cemMainThreadCheckpointDomain, cemWorkerCheckpointDomain } from './checkpoint-domain.js';
import { CemCheckpointPublicationCoordinator } from '../../checkpoint-publication.js';
import { closeNativeRenderingGeneration, retireNativeRenderingGeneration, nativeCheckpointDrain, nativeCheckpointDrainCurrent, nativeCheckpointDrainRequirements, nativeCheckpointClosedCoverage, nativeCheckpointSubscribe } from './checkpoint-domain.js';

describe('pressure admission and bounded draining', () => {
    it('starts the bounded drain deadline on pressure even without a waiting caller', async () => {
        vi.useFakeTimers();
        try {
            const domain = new CemNativeCheckpointDomain('worker'), client = domain.client();
            const finish = nativeCheckpointAdmission(client); domain.closeAdmission('soft-pressure');
            await vi.advanceTimersByTimeAsync(5001); finish();
            await expect(nativeCheckpointDrain(client)).rejects.toThrow('unavailable');
            expect(nativeCheckpointSnapshot(new Map([[client, ['a']]]))).toBeUndefined();
        } finally { vi.useRealTimers(); }
    });
    it('closes the whole physical generation and drains existing work without buffering new work', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), a = domain.client(), b = domain.client();
        const finish = nativeCheckpointAdmission(a);
        domain.closeAdmission('soft-pressure');
        expect(() => nativeCheckpointAdmission(b)).toThrow('cem.runtime.memory.closed');
        expect(() => domain.client()).toThrow('cem.runtime.memory.closed');
        let drained = false;
        const draining = nativeCheckpointDrain(b).then(proof => { drained = true; return proof; });
        await Promise.resolve(); expect(drained).toBe(false);
        finish(); finish(); const proof = await draining;
        expect(nativeCheckpointDrainCurrent(proof)).toBe(true);
        expect(nativeCheckpointDrainCurrent({ ...proof })).toBe(false);
        expect(nativeCheckpointSnapshot(new Map([[a, ['a']], [b, ['b']]]))).toBeDefined();
        const cleanup = nativeCheckpointAdmission(a, true);
        expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
        cleanup(); expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
        expect(nativeCheckpointDrainCurrent(await nativeCheckpointDrain(a))).toBe(true);
    });
    it('keeps host I/O in the drain even between native jobs', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client();
        let pending = true; nativeCheckpointDrainRequirements(client, () => !pending);
        domain.closeAdmission('hard-pressure');
        let settled = false; const draining = nativeCheckpointDrain(client).then(() => { settled = true; });
        await Promise.resolve(); expect(settled).toBe(false);
        pending = false; domain.changed(); await draining; expect(settled).toBe(true);
    });
    it('fails closed at timeout, keeps work enrolled and never revives on late completion', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client(), finish = nativeCheckpointAdmission(client);
        domain.closeAdmission('soft-pressure');
        await expect(nativeCheckpointDrain(client, { timeoutMs: 1 })).rejects.toThrow('cem.runtime.drain.timeout');
        finish(); await expect(nativeCheckpointDrain(client)).rejects.toThrow('unavailable');
        expect(nativeCheckpointSnapshot(new Map([[client, ['a']]]))).toBeUndefined();
        expect(() => nativeCheckpointAdmission(client)).toThrow('closed');
        const cleanup = nativeCheckpointAdmission(client, true); cleanup();
    });
    it('cancels only the waiter and refuses revoked, open or invalid drain requests', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client(), finish = nativeCheckpointAdmission(client);
        await expect(nativeCheckpointDrain(client)).rejects.toThrow('unavailable');
        domain.closeAdmission('soft-pressure');
        await expect(nativeCheckpointDrain(client, { timeoutMs: Infinity })).rejects.toThrow('timeout-config');
        const controller = new AbortController();
        const canceled = nativeCheckpointDrain(client, { signal: controller.signal }); controller.abort();
        await expect(canceled).rejects.toThrow('aborted');
        const revoked = nativeCheckpointDrain(client); domain.revoke();
        await expect(revoked).rejects.toThrow('unavailable'); finish();
    });
    it('does not permit moving clients out of a pressure-closed live heap or into a closed heap', () => {
        const a = new CemNativeCheckpointDomain('worker'), b = new CemNativeCheckpointDomain('main-thread');
        const client = a.client(); a.closeAdmission('soft-pressure');
        expect(() => b.move(client)).toThrow('closed');
        a.revoke(); b.move(client); expect(nativeCheckpointIdentity(client)).toBe(b.identity());
        b.closeAdmission('hard-pressure');
        const c = new CemNativeCheckpointDomain('worker'), peer = c.client();
        expect(() => b.move(peer)).toThrow('closed');
    });
    it('samples native pressure before admission and never reopens after lower telemetry', () => {
        const domain = new CemNativeCheckpointDomain('main-thread'), client = domain.client();
        let pressure = false; domain.sampleWith(() => ({ softPressure: pressure, hardPressure: false }));
        pressure = true; domain.samplePressure(); pressure = false; domain.samplePressure();
        expect(domain.pressureReason()).toBe('soft-pressure');
        expect(() => nativeCheckpointAdmission(client)).toThrow('closed');
        const unknown = new CemNativeCheckpointDomain('worker'); unknown.sampleWith(() => { throw new Error('lost'); });
        expect(unknown.pressureReason()).toBe('telemetry-unavailable');
    });
    it('rechecks complete client/resource coverage independently of successful drain evidence', async () => {
        const a = new CemNativeCheckpointDomain('worker'), b = new CemNativeCheckpointDomain('main-thread');
        const first = a.client(), omitted = a.client(), second = b.client();
        a.closeAdmission('soft-pressure'); b.closeAdmission('hard-pressure');
        const proofs = await Promise.all([nativeCheckpointDrain(first), nativeCheckpointDrain(second)]);
        const bindings = new Map([[first, ['editor']], [second, ['source']]]);
        expect(nativeCheckpointClosedCoverage(bindings, [], proofs)).toBeUndefined();
        bindings.set(omitted, ['other']);
        expect(nativeCheckpointClosedCoverage(bindings, [], proofs)).toHaveLength(2);
        expect(nativeCheckpointClosedCoverage(bindings, [], [proofs[0], proofs[0]])).toBeUndefined();
        expect(nativeCheckpointClosedCoverage(bindings, [], proofs.map(proof => ({ ...proof })))).toBeUndefined();
        a.retain('unknown', 1);
        const fresh = await Promise.all([nativeCheckpointDrain(first), nativeCheckpointDrain(second)]);
        expect(nativeCheckpointClosedCoverage(bindings, [], fresh)).toBeUndefined();
        a.release('unknown', 1);
        const cleanup = nativeCheckpointAdmission(first, true);
        expect(nativeCheckpointClosedCoverage(bindings, [], fresh)).toBeUndefined(); cleanup();
    });
    it('preserves last-good bytes on a timed-out pressure drain without admitting rollback', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client();
        const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: 'logical', generation: 1, policyStamp: 'private',
            allowOwner: () => true, seal: text => text });
        const owner = coordinator.register('a', () => '<template/>', () => true, 0, () => [],
            { host: {} as HTMLElement, nativeClient: client, resolve: () => undefined });
        owner.published({ instanceId: 'a', declarationIdentity: 'source', dataRevision: '1' });
        await coordinator.capture(); const lastGood = coordinator.lastCommitted()!.bytes;
        const finish = nativeCheckpointAdmission(client); domain.closeAdmission('soft-pressure');
        await expect(nativeCheckpointDrain(client, { timeoutMs: 1 })).rejects.toThrow('timeout');
        finish(); expect(coordinator.current()).toBeUndefined();
        await expect(coordinator.capture()).rejects.toThrow('unavailable');
        expect(coordinator.lastCommitted()!.bytes).toEqual(lastGood);
        coordinator.dispose(); domain.remove(client);
    });
    it('requires original current native source witnesses when verifying closed coverage', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client(), source = {};
        let current = true;
        nativeCheckpointRequirements(client, sources => sources.has('session:1') ? [] : undefined);
        nativeCheckpointWitnessSource(source, client, 'session:1', () => current);
        const witness = nativeCheckpointSourceProof(source)!, bindings = new Map([[client, ['editor']]]);
        domain.closeAdmission('soft-pressure'); const proof = await nativeCheckpointDrain(client);
        expect(nativeCheckpointClosedCoverage(bindings, [], [proof])).toBeUndefined();
        expect(nativeCheckpointClosedCoverage(bindings, [{ ...witness }], [proof])).toBeUndefined();
        expect(nativeCheckpointClosedCoverage(bindings, [witness], [proof])).toHaveLength(1);
        current = false; expect(nativeCheckpointClosedCoverage(bindings, [witness], [proof])).toBeUndefined();
    });
});

describe('physical checkpoint domains', () => {
    it('requires every actual client, rejects structural tokens and fences pending work', () => {
        const domain = new CemNativeCheckpointDomain('worker');
        const a = domain.client(), b = domain.client();
        const bindings = new Map([[a, ['a']]]);
        expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        bindings.set(b, ['b']);
        const initial = nativeCheckpointSnapshot(bindings)!;
        expect(initial[0].clients).toHaveLength(2);
        const release = nativeCheckpointAdmission(b);
        expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        release(); release();
        expect(nativeCheckpointSnapshot(bindings)![0].revision).toBeGreaterThan(initial[0].revision);
        expect(nativeCheckpointSnapshot(new Map([[{ ...a }, ['a']]]))).toBeUndefined();
        domain.revoke(); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    });

    it('moves the exact client on fallback and retains host work until it drains', () => {
        const worker = new CemNativeCheckpointDomain('worker'), fallback = new CemNativeCheckpointDomain('main-thread');
        const client = worker.client(), peer = fallback.client();
        const old = nativeCheckpointIdentity(client);
        const release = nativeCheckpointAdmission(client);
        worker.revoke(); expect(nativeCheckpointIdentity(client)).toBeUndefined();
        fallback.move(client); worker.remove(client);
        expect(nativeCheckpointIdentity(client)).not.toBe(old);
        const bindings = new Map([[client, ['a']], [peer, ['b']]]);
        expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        release(); expect(nativeCheckpointSnapshot(bindings)![0].mode).toBe('main-thread');
        fallback.remove(peer); bindings.delete(peer);
        expect(nativeCheckpointSnapshot(bindings)![0].clients).toHaveLength(1);
        fallback.remove(client); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    });

    it('blocks direct native retention and orphaned resources rather than omitting them', () => {
        const domain = new CemNativeCheckpointDomain('main-thread'), client = domain.client();
        const bindings = new Map([[client, ['editor']]]);
        domain.retain('session', 1); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        domain.adopt('session', 1, { ...client }); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        domain.adopt('session', 1, client); expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
        domain.release('session', 1); expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
        for (let i = 0; i < 257; i++) domain.client();
        expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    });

    it('binds owner publication to its physical generation and rejects held seals on host-only admissions', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), client = domain.client();
        let finish: (() => void) | undefined;
        const c = new CemCheckpointPublicationCoordinator({ runtimeId: 'logical', generation: 99, policyStamp: 'private',
            allowOwner: () => true, seal: text => finish ? new Promise(resolve => { finish = () => resolve(text); }) : text });
        const owner = c.register('a', () => '<template/>', () => true, 0, () => [], { host: {} as HTMLElement, nativeClient: client, resolve: () => undefined });
        const metadata = { instanceId: 'a', declarationIdentity: 'source', dataRevision: '1' };
        owner.published(metadata); await c.capture();
        const first = c.current()!;
        expect(first.receipt.nativeRuntimes![0].runtimeId).not.toBe('logical');
        const peer = domain.client(); expect(c.current()).toBeUndefined();
        domain.remove(peer); await c.capture();
        const lastGood = c.lastCommitted()!.bytes;
        finish = () => undefined;
        const capture = c.capture();
        const release = nativeCheckpointAdmission(client); release(); finish();
        await expect(capture).rejects.toThrow('stale');
        expect(c.lastCommitted()!.bytes).toEqual(lastGood);
        const fallback = new CemNativeCheckpointDomain('main-thread'); fallback.move(client);
        expect(c.current()).toBeUndefined(); await expect(c.capture()).rejects.toThrow('unavailable');
        finish = undefined; owner.published(metadata); await c.capture();
        expect(c.current()!.receipt.nativeRuntimes![0].mode).toBe('main-thread');
        c.dispose(); domain.remove(client); fallback.remove(client);
    });
    it('includes an external source generation only through its current private source witness', async () => {
        const main = new CemNativeCheckpointDomain('worker'), external = new CemNativeCheckpointDomain('main-thread');
        const client = main.client(), sourceClient = external.client(), source = {};
        let live = true;
        nativeCheckpointRequirements(sourceClient, sources => sources.has('private-session') ? [] : ['unpublished-source']);
        nativeCheckpointWitnessSource(source, sourceClient, 'private-session', () => live);
        expect(nativeCheckpointSourceProof({ ...source })).toBeUndefined();
        const original = nativeCheckpointSourceProof(source)!; let proof = original;
        const c = new CemCheckpointPublicationCoordinator({ runtimeId: 'logical', generation: 1, policyStamp: 'private', allowOwner: () => true, seal: text => text });
        const owner = c.register('editor', () => '<template>source capsule</template>', () => true, 0, () => [], {
            host: {} as HTMLElement, nativeClient: client, nativeSources: () => [proof], resolve: () => undefined });
        owner.published({ instanceId: 'editor', declarationIdentity: 'source', dataRevision: '1' });
        await c.capture(); expect(c.current()!.receipt.nativeRuntimes).toHaveLength(2);
        proof = { ...original }; expect(c.current()).toBeUndefined();
        proof = original; await c.capture();
        const release = nativeCheckpointAdmission(sourceClient);
        expect(c.current()).toBeUndefined(); release(); await c.capture();
        live = false; expect(c.current()).toBeUndefined(); await expect(c.capture()).rejects.toThrow('unavailable');
        expect(c.lastCommitted()).toBeDefined();
        c.dispose(); main.remove(client); external.remove(sourceClient);
    });

    it('shares the actual Worker identity and refuses a changed initialized memory', () => {
        const worker = {};
        expect(cemWorkerCheckpointDomain(worker)).toBe(cemWorkerCheckpointDomain(worker));
        expect(cemWorkerCheckpointDomain({})).not.toBe(cemWorkerCheckpointDomain(worker));
        const domain = new CemNativeCheckpointDomain('main-thread'), client = domain.client();
        const memory = new WebAssembly.Memory({ initial: 1 }); domain.bindNativeRuntime(memory); domain.bindNativeRuntime(memory);
        const bindings = new Map([[client, ['editor']]]); expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
        domain.bindNativeRuntime(new WebAssembly.Memory({ initial: 1 })); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
    });
    it('tracks standalone asynchronous native helpers until completion and failure', async () => {
        const client = cemMainThreadCheckpointDomain.client(), bindings = new Map([[client, ['editor']]]);
        let mutations = 0;
        const stop = nativeCheckpointSubscribe(client, () => { mutations++; });
        let finish!: () => void;
        const work = nativeCheckpointWork(() => new Promise<void>(resolve => { finish = resolve; }));
        const pending = work(); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        expect(mutations).toBe(1);
        finish(); await pending; expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
        expect(mutations).toBe(2);
        stop();
        await expect(nativeCheckpointWork(async () => { throw new Error('refused'); })()).rejects.toThrow('refused');
        expect(nativeCheckpointSnapshot(bindings)).toBeDefined(); cemMainThreadCheckpointDomain.remove(client);
    });

    it('refuses duplicate ledger claims on the same native memory or worker', () => {
        for (const mode of ['worker', 'main-thread'] as const) {
            const physical = mode === 'worker' ? {} : new WebAssembly.Memory({ initial: 1 });
            const first = new CemNativeCheckpointDomain(mode), duplicate = new CemNativeCheckpointDomain(mode);
            first.bindNativeRuntime(physical);
            const client = first.client(), peer = duplicate.client();
            expect(nativeCheckpointSnapshot(new Map([[client, ['editor']]]))).toBeDefined();
            duplicate.bindNativeRuntime(physical);
            expect(nativeCheckpointIdentity(client)).toBeUndefined();
            expect(nativeCheckpointIdentity(peer)).toBeUndefined();
            expect(nativeCheckpointSnapshot(new Map([[client, ['editor']], [peer, ['peer']]]))).toBeUndefined();
        }
    });

    it('attributes failed cleanup to the admitted generation after a client moves to fallback', () => {
        const worker = new CemNativeCheckpointDomain('worker'), fallback = new CemNativeCheckpointDomain('main-thread');
        const client = worker.client(), peer = fallback.client();
        const failedOldCleanup = nativeCheckpointFailureFence(client);
        worker.revoke(); fallback.move(client); failedOldCleanup();
        expect(nativeCheckpointSnapshot(new Map([[client, ['source']], [peer, ['editor']]]))).toBeDefined();
        fallback.remove(client); fallback.remove(peer);
    });

});


describe('shared rendering retirement', () => {
    it('requires every original client and source before physical preparation', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), a = domain.client(), b = domain.client();
        const prepare = vi.fn(async () => ({ schemaVersion: 1, frozen: true, inventoryRevision: 7, frozenRevision: 8, replacementAllowed: false })), terminate = vi.fn();
        domain.retirementEndpoint({ close: async () => undefined, prepare, abort: async () => undefined, terminate });
        const proof = await closeNativeRenderingGeneration(a);
        const handoff = { current: () => true, retain: () => new Uint8Array([1, 2]) };
        await expect(retireNativeRenderingGeneration(new Map([[a, ['a']]]), [], proof, handoff)).rejects.toThrow('coverage');
        await expect(retireNativeRenderingGeneration(new Map([[a, ['a']], [b, ['b']]]), [], { ...proof }, handoff)).rejects.toThrow('coverage');
        const source = {}; nativeCheckpointWitnessSource(source, b, 'session:source', () => true);
        nativeCheckpointRequirements(b, sources => sources.has('session:source') ? ['b'] : undefined);
        await expect(retireNativeRenderingGeneration(new Map([[a, ['a']], [b, ['b']]]), [], proof, handoff)).rejects.toThrow('coverage');
        expect(prepare).not.toHaveBeenCalled(); expect(terminate).not.toHaveBeenCalled();
        await retireNativeRenderingGeneration(new Map([[a, ['a']], [b, ['b']]]), [nativeCheckpointSourceProof(source)!], proof, handoff);
        expect(prepare).toHaveBeenCalledOnce(); expect(terminate).toHaveBeenCalledOnce();
    });
    it('retains independent bytes and terminates only after a native freeze and final currentness check', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), a = domain.client();
        const terminate = vi.fn(), bytes = new Uint8Array([3, 4]);
        domain.retirementEndpoint({ close: async () => undefined, prepare: async () => ({ schemaVersion: 1, frozen: true,
            inventoryRevision: 7, frozenRevision: 8, replacementAllowed: false }), abort: async () => undefined, terminate });
        const proof = await closeNativeRenderingGeneration(a);
        const retired = await retireNativeRenderingGeneration(new Map([[a, ['a']]]), [], proof, { current: () => true, retain: () => bytes });
        bytes[0] = 9; expect([...retired.checkpoint]).toEqual([3, 4]); expect(retired.replacementAllowed).toBe(false);
        expect(terminate).toHaveBeenCalledOnce(); expect(nativeCheckpointIdentity(a)).toBeUndefined();
        expect(() => nativeCheckpointAdmission(a)).toThrow('retired');
    });
    it('cancels physical retirement on changed handoff or ownership and fences the disposed generation', async () => {
        for (const change of ['handoff', 'owner', 'malformed']) {
            const domain = new CemNativeCheckpointDomain('worker'), a = domain.client(); let current = true;
            const abort = vi.fn(async () => undefined), terminate = vi.fn();
            domain.retirementEndpoint({ close: async () => undefined, prepare: async () => {
                if (change === 'handoff') current = false;
                if (change === 'owner') domain.changed();
                return { schemaVersion: 1, frozen: change !== 'malformed', inventoryRevision: 7, frozenRevision: 8, replacementAllowed: false };
            }, abort, terminate });
            const proof = await closeNativeRenderingGeneration(a);
            await expect(retireNativeRenderingGeneration(new Map([[a, ['a']]]), [], proof, { current: () => current, retain: () => new Uint8Array([1]) })).rejects.toThrow();
            expect(abort).toHaveBeenCalledOnce(); expect(terminate).not.toHaveBeenCalled();
            expect(nativeCheckpointIdentity(a)).toBeUndefined();
        }
    });
    it('never migrates a failed explicit closure into another fallback generation', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), a = domain.client();
        domain.retirementEndpoint({ close: async () => { throw new Error('lost closure'); }, prepare: vi.fn(), abort: vi.fn(), terminate: vi.fn() });
        await expect(closeNativeRenderingGeneration(a)).rejects.toThrow('lost closure');
        const fallback = new CemNativeCheckpointDomain('main-thread');
        expect(() => fallback.move(a)).toThrow('retired'); expect(fallback.identity()).toBeDefined();
        expect(nativeCheckpointIdentity(a)).toBeUndefined();
    });
    it('bounds preparation and abort, preserves last-good bytes, and never revives timed-out authority', async () => {
        vi.useFakeTimers();
        try {
            const domain = new CemNativeCheckpointDomain('worker'), a = domain.client();
            const terminate = vi.fn(), bytes = new Uint8Array([8]);
            domain.retirementEndpoint({ close: async () => undefined, prepare: () => new Promise(() => undefined),
                abort: () => new Promise(() => undefined), terminate });
            const proof = await closeNativeRenderingGeneration(a);
            const retired = retireNativeRenderingGeneration(new Map([[a, ['a']]]), [], proof, { current: () => true, retain: () => bytes }, 10);
            const rejected = expect(retired).rejects.toThrow('timeout');
            await vi.advanceTimersByTimeAsync(21); await rejected;
            expect([...bytes]).toEqual([8]); expect(terminate).not.toHaveBeenCalled();
            expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
            expect(() => nativeCheckpointAdmission(a)).toThrow('retired');
        } finally { vi.useRealTimers(); }
    });
    it('refuses non-owned or oversized handoff storage before native cleanup', async () => {
        const domain = new CemNativeCheckpointDomain('worker'), a = domain.client();
        const prepare = vi.fn(async () => undefined);
        domain.retirementEndpoint({ close: async () => undefined, prepare, abort: async () => undefined, terminate: vi.fn() });
        const proof = await closeNativeRenderingGeneration(a);
        for (const bytes of [new Uint8Array(), new Uint8Array(1024 * 1024 + 1), new Uint8Array(new SharedArrayBuffer(1))]) {
            await expect(retireNativeRenderingGeneration(new Map([[a, ['a']]]), [], proof, { current: () => true, retain: () => bytes })).rejects.toThrow('handoff');
        }
        expect(prepare).not.toHaveBeenCalled(); expect(nativeCheckpointDrainCurrent(proof)).toBe(true); domain.revoke();
    });
    it('refuses physical fallback retirement before touching admission, bytes or owners', async () => {
        const domain = new CemNativeCheckpointDomain('main-thread'), a = domain.client();
        await expect(closeNativeRenderingGeneration(a)).rejects.toThrow('unsupported');
        expect(domain.pressureReason()).toBeUndefined(); expect(nativeCheckpointIdentity(a)).toBeDefined();
        domain.remove(a);
    });
});
