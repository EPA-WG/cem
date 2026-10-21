import { describe, expect, it } from 'vitest';
import { CemNativeCheckpointDomain, nativeCheckpointAdmission, nativeCheckpointIdentity, nativeCheckpointSnapshot, nativeCheckpointRequirements, nativeCheckpointWitnessSource, nativeCheckpointSourceProof, nativeCheckpointFailureFence, nativeCheckpointWork, cemMainThreadCheckpointDomain, cemWorkerCheckpointDomain } from './checkpoint-domain.js';
import { CemCheckpointPublicationCoordinator } from '../../checkpoint-publication.js';

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
        let finish!: () => void;
        const work = nativeCheckpointWork(() => new Promise<void>(resolve => { finish = resolve; }));
        const pending = work(); expect(nativeCheckpointSnapshot(bindings)).toBeUndefined();
        finish(); await pending; expect(nativeCheckpointSnapshot(bindings)).toBeDefined();
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
