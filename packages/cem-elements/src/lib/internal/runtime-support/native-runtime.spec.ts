import { beforeEach, describe, expect, it, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ dispose: vi.fn(), initialize: vi.fn(), call: vi.fn(), legacy: vi.fn() }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({ default: mocks.legacy, cemQlVersion: () => 'legacy' }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql_factory.js', () => ({ createCemQlWasmGeneration: () => {
    let dead = false; const memory = new WebAssembly.Memory({ initial: 1 });
    return { initialize: mocks.initialize, memory: () => dead ? undefined : memory,
        call: mocks.call, dispose: (current?: () => boolean) => { mocks.dispose(); if (current && !current()) throw new Error('ownership-stale'); dead = true; } };
} }));
beforeEach(() => { vi.resetModules(); vi.resetAllMocks(); mocks.initialize.mockResolvedValue(undefined); mocks.call.mockReturnValue('owned'); });
async function selected() {
    const api = await import('./native-runtime.js'), domains = await import('./checkpoint-domain.js');
    const generation = api.isolateCemFallbackGeneration(); await generation.ready();
    return { api, domains, generation, lease: generation.claimOwnership() };
}
describe('owned fallback generation and embedding consent', () => {
    it('selects once before use, keeps generated exports private and fences saved aliases after empty disposal', async () => {
        const { api, generation, lease } = await selected(); const alias = api.cemQlVersion;
        expect(Object.keys(generation).sort()).toEqual(['packageResourceBinding', 'claimOwnership', 'disposeEmpty', 'generationParticipant', 'ready', 'releaseOwnership', 'retirementConsent', 'runtimeId'].sort());
        expect(alias()).toBe('owned'); expect(mocks.legacy).not.toHaveBeenCalled();
        expect(() => api.isolateCemFallbackGeneration()).toThrow('already-selected');
        const disposed = await generation.disposeEmpty([lease]);
        expect(disposed).toEqual({ runtimeId: generation.runtimeId, releasedOwnedReferences: true, replacementAllowed: false });
        expect(mocks.dispose).toHaveBeenCalledOnce(); expect(() => alias()).toThrow('retired');
        await expect(generation.ready()).rejects.toThrow('retired'); expect(api.nativeRuntimeUsable()).toBe(false);
    });
    it('refuses copied, duplicate, omitted, foreign or released ownership before native disposal', async () => {
        const { generation, lease, domains } = await selected(); const other = generation.claimOwnership();
        for (const leases of [[], [lease], [{ ...lease }, other], [lease, lease], [lease, { kind: 'cem-fallback-wasm-ownership' as const }]])
            await expect(generation.disposeEmpty(leases)).rejects.toThrow('ownership');
        expect(mocks.dispose).not.toHaveBeenCalled(); expect(domains.cemMainThreadCheckpointDomain.pressureReason()).toBeUndefined();
        generation.releaseOwnership(other);
        await expect(generation.disposeEmpty([lease, other])).rejects.toThrow('ownership');
        await generation.disposeEmpty([lease]);
    });
    it('does not trust overridden array methods or iterators as original consent', async () => {
        const { generation, lease, domains } = await selected();
        const forged = [{ ...lease }]; forged.some = () => false;
        await expect(generation.disposeEmpty(forged)).rejects.toThrow('ownership');
        const iterator = vi.fn(function* () { yield lease; });
        const misleading = [{ ...lease }]; misleading[Symbol.iterator] = iterator;
        await expect(generation.disposeEmpty(misleading)).rejects.toThrow('ownership');
        expect(iterator).not.toHaveBeenCalled(); expect(mocks.dispose).not.toHaveBeenCalled();
        expect(domains.cemMainThreadCheckpointDomain.pressureReason()).toBeUndefined();
        await generation.disposeEmpty([lease]);
    });
    it('rejects accessors without invoking them and refuses inspection races or reentrancy', async () => {
        const { generation, lease, domains } = await selected();
        const accessor = vi.fn(() => lease), values = [lease];
        Object.defineProperty(values, '0', { get: accessor });
        await expect(generation.disposeEmpty(values)).rejects.toThrow('ownership');
        expect(accessor).not.toHaveBeenCalled();
        let nested: Promise<unknown> | undefined;
        const reentrant = new Proxy([lease], { getOwnPropertyDescriptor(target, property) {
            if (property === '0') nested = generation.disposeEmpty([lease]).catch(error => String(error));
            return Reflect.getOwnPropertyDescriptor(target, property);
        } });
        await generation.disposeEmpty(reentrant);
        expect(await nested).toContain('pending'); expect(mocks.dispose).toHaveBeenCalledOnce();
        expect(domains.cemMainThreadCheckpointDomain.pressureReason()).toBe('soft-pressure');
    });
    it('rejects ownership mutation during consent inspection before admission closes', async () => {
        const { generation, lease, domains } = await selected();
        let other = generation.claimOwnership();
        const racing = new Proxy([lease, other], { getOwnPropertyDescriptor(target, property) {
            if (property === '1') { generation.releaseOwnership(other); other = generation.claimOwnership(); return { ...Reflect.getOwnPropertyDescriptor(target, property), value: other }; }
            return Reflect.getOwnPropertyDescriptor(target, property);
        } });
        await expect(generation.disposeEmpty(racing)).rejects.toThrow('ownership-stale');
        expect(mocks.dispose).not.toHaveBeenCalled();
        expect(domains.cemMainThreadCheckpointDomain.pressureReason()).toBeUndefined();
        await generation.disposeEmpty([lease, other]);
    });
    it('requires all processing clients and retained helper resources to be released', async () => {
        const { generation, lease, domains } = await selected(), domain = domains.cemMainThreadCheckpointDomain;
        const peer = domain.client();
        await expect(generation.disposeEmpty([lease])).rejects.toThrow('peers');
        domain.remove(peer); domain.retain('template', 1);
        await expect(generation.disposeEmpty([lease])).rejects.toThrow('peers');
        expect(domain.pressureReason()).toBeUndefined(); expect(mocks.dispose).not.toHaveBeenCalled();
        domain.release('template', 1); await generation.disposeEmpty([lease]);
    });
    it('cancels changed consent during drain without releasing native references', async () => {
        const { generation, lease } = await selected(), other = generation.claimOwnership();
        const pending = generation.disposeEmpty([lease, other]); generation.releaseOwnership(other);
        await expect(pending).rejects.toThrow('ownership-stale'); expect(mocks.dispose).not.toHaveBeenCalled();
        await generation.disposeEmpty([lease]); expect(mocks.dispose).toHaveBeenCalledOnce();
    });
    it('rechecks original consent while the native permit is held', async () => {
        const { generation, lease, api } = await selected(), other = generation.claimOwnership();
        mocks.dispose.mockImplementationOnce(() => generation.releaseOwnership(other));
        await expect(generation.disposeEmpty([lease, other])).rejects.toThrow('ownership-stale');
        expect(api.nativeRuntimeUsable()).toBe(true);
        await generation.disposeEmpty([lease]);
    });
    it('keeps cleanup possible when the native census refuses a hidden live root', async () => {
        const { api, generation, lease } = await selected(); mocks.dispose.mockImplementationOnce(() => { throw new Error('native roots'); });
        await expect(generation.disposeEmpty([lease])).rejects.toThrow('native roots');
        expect(api.nativeRuntimeUsable()).toBe(true); expect(api.disposeTemplate(1)).toBe('owned');
        await generation.disposeEmpty([lease]); expect(() => api.cemQlVersion()).toThrow('retired');
    });
    it('does not switch a legacy generation after its functions have been used', async () => {
        const api = await import('./native-runtime.js'); expect(api.cemQlVersion()).toBe('legacy');
        expect(() => api.isolateCemFallbackGeneration()).toThrow('already-selected'); expect(mocks.initialize).not.toHaveBeenCalled();
    });
});


    async function active() {
        const state = await selected(), d = state.domains, domain = d.cemMainThreadCheckpointDomain;
        const client = domain.client(); domain.retain('template', 7); domain.adopt('template', 7, client);
        const cleanup = vi.fn(() => domain.release('template', 7));
        d.nativeCheckpointRetirementCleanup(client, cleanup);
        const consent = state.generation.retirementConsent([state.lease]);
        return { ...state, domain, client, cleanup, consent, bindings: new Map([[client, ['editor']]]) };
    }
    const handoff = () => ({ current: () => true, retain: () => new Uint8Array([4, 8]) });

describe('active owned fallback retirement', () => {
    it('refuses primitive consent even before any embedding lease has been claimed', async () => {
        const api = await import('./native-runtime.js'), d = await import('./checkpoint-domain.js');
        const generation = api.isolateCemFallbackGeneration(); await generation.ready();
        const domain = d.cemMainThreadCheckpointDomain, client = domain.client();
        d.nativeCheckpointRetirementCleanup(client, () => undefined);
        for (const token of [0, -0, false, '', null, undefined, { kind: 'cem-fallback-retirement-consent' }]) {
            await expect(d.closeNativeRenderingGeneration(client, 5000, token as object)).rejects.toThrow('ownership');
            expect(domain.pressureReason()).toBeUndefined();
        }
        expect(mocks.dispose).not.toHaveBeenCalled();
    });
    it('requires original closed consent and releases covered resources without invalidating the current handoff', async () => {
        const { api, domains: d, generation, client, cleanup, consent, bindings } = await active();
        await expect(d.closeNativeRenderingGeneration(client, 5000, { ...consent })).rejects.toThrow('ownership');
        const proof = await d.closeNativeRenderingGeneration(client, 5000, consent);
        const copy = generation.retirementConsent;
        expect(() => copy([])).toThrow();
        await expect(d.retireNativeRenderingGeneration(bindings, [], proof, handoff(), 5000, { ...consent })).rejects.toThrow('ownership');
        expect(cleanup).not.toHaveBeenCalled();
        const bytes = new Uint8Array([4, 8]);
        const retired = await d.retireNativeRenderingGeneration(bindings, [], proof, { current: () => true, retain: () => bytes }, 5000, consent);
        bytes[0] = 9; expect([...retired.checkpoint]).toEqual([4, 8]);
        expect(cleanup).toHaveBeenCalledOnce(); expect(mocks.dispose).toHaveBeenCalledOnce();
        expect(retired.replacementAllowed).toBe(false); expect(api.nativeRuntimeUsable()).toBe(false);
        expect(() => api.cemQlVersion()).toThrow('retired');
    });
    it('performs post-cleanup handoff callbacks only while the native permit is held', async () => {
        const { domains: d, domain, client, cleanup, consent, bindings } = await active();
        let cleaned = false, frozen = false;
        cleanup.mockImplementation(() => { domain.release('template', 7); cleaned = true; });
        mocks.dispose.mockImplementationOnce(() => { frozen = true; });
        const proof = await d.closeNativeRenderingGeneration(client, 5000, consent);
        await d.retireNativeRenderingGeneration(bindings, [], proof, {
            current: () => { if (cleaned && !frozen) throw new Error('callback before native freeze'); return true; },
            retain: () => new Uint8Array([1]),
        }, 5000, consent);
        expect(frozen).toBe(true);
    });
    it.each(['cleanup', 'freeze'])('refuses an expired synchronous %s deadline without releasing references', async phase => {
        const { api, domain, domains: d, client, cleanup, consent, bindings } = await active();
        let now = 0; const clock = vi.spyOn(performance, 'now').mockImplementation(() => now);
        let frozen = false;
        try {
            cleanup.mockImplementation(() => { domain.release('template', 7); if (phase === 'cleanup') now = 10; });
            mocks.dispose.mockImplementationOnce(() => { frozen = true; });
            const proof = await d.closeNativeRenderingGeneration(client, 5, consent);
            await expect(d.retireNativeRenderingGeneration(bindings, [], proof, {
                current: () => { if (phase === 'freeze' && frozen) now = 10; return true; }, retain: () => new Uint8Array([1]),
            }, 5, consent)).rejects.toThrow();
            expect(api.nativeRuntimeUsable()).toBe(false);
        } finally { clock.mockRestore(); }
    });
    it('refuses omitted peers, unregistered cleanup, copied drain and stale embedding consent before cleanup', async () => {
        const { domain, domains: d, generation, lease, client, cleanup, consent, bindings } = await active();
        const peer = domain.client();
        const proof = await d.closeNativeRenderingGeneration(client, 5000, consent);
        await expect(d.retireNativeRenderingGeneration(bindings, [], proof, handoff(), 5000, consent)).rejects.toThrow('coverage');
        const all = new Map([...bindings, [peer, ['peer']]]);
        await expect(d.retireNativeRenderingGeneration(all, [], proof, handoff(), 5000, consent)).rejects.toThrow('cleanup');
        await expect(d.retireNativeRenderingGeneration(all, [], { ...proof }, handoff(), 5000, consent)).rejects.toThrow('coverage');
        generation.releaseOwnership(lease);
        await expect(d.retireNativeRenderingGeneration(all, [], proof, handoff(), 5000, consent)).rejects.toThrow();
        expect(cleanup).not.toHaveBeenCalled(); expect(mocks.dispose).not.toHaveBeenCalled();
    });
    it.each(['input', 'ledger', 'native', 'freeze'])('fences failed %s preparation without releasing references or handoff bytes', async change => {
        const { api, domain, domains: d, generation, lease, client, cleanup, consent, bindings } = await active();
        let current = true; const bytes = new Uint8Array([6]);
        cleanup.mockImplementation(() => {
            domain.release('template', 7);
            if (change === 'input') current = false;
            if (change === 'ledger') domain.changed();
        });
        if (change === 'native') mocks.dispose.mockImplementationOnce(() => { throw new Error('native root'); });
        if (change === 'freeze') mocks.dispose.mockImplementationOnce(() => generation.releaseOwnership(lease));
        const proof = await d.closeNativeRenderingGeneration(client, 5000, consent);
        await expect(d.retireNativeRenderingGeneration(bindings, [], proof, { current: () => current, retain: () => bytes }, 5000, consent)).rejects.toThrow();
        expect([...bytes]).toEqual([6]); expect(api.nativeRuntimeUsable()).toBe(false);
        expect(d.nativeCheckpointDrainCurrent(proof)).toBe(false);
        expect(() => api.cemQlVersion()).toThrow('retired');
    });
});

describe('coordinated fresh fallback allocation', () => {
    it('admits distinct bindings after original active retirement and keeps old aliases fenced', async () => {
        const { generation, domains, api, consent, client, bindings } = await active();
        const { CemNativeGenerationGroup } = await import('./native-generation.js');
        const group = new CemNativeGenerationGroup([generation.generationParticipant()]);
        const proof = await domains.closeNativeRenderingGeneration(client, 5000, consent);
        const receipt = await domains.retireNativeRenderingGeneration(bindings, [], proof, handoff(), 5000, consent);
        mocks.call.mockImplementation((name: string) => name === 'runtimeMemoryStatus' ? '{"softPressure":false,"hardPressure":false}' : 'fresh');
        const admission = await group.allocate([receipt]);
        const fresh = admission.generations[0].runtime as import('./native-runtime.js').CemFreshFallbackGeneration;
        expect(fresh.runtimeId).not.toBe(generation.runtimeId);
        expect(fresh.invoke('cemQlVersion', [])).toBe('fresh');
        expect(() => fresh.invoke('configureRuntimeMemoryBudget' as never, [])).toThrow('entrypoint');
        expect(() => api.cemQlVersion()).toThrow('retired');
        admission.revoke(); expect(() => fresh.invoke('cemQlVersion', [])).toThrow('closed');
    });
});

it.each(['pressure', 'unavailable'])('refuses fresh fallback %s without opening admission or old aliases', async mode => {
    const { generation, domains, api, consent, client, bindings } = await active();
    const { CemNativeGenerationGroup } = await import('./native-generation.js');
    const group = new CemNativeGenerationGroup([generation.generationParticipant()]);
    const proof = await domains.closeNativeRenderingGeneration(client, 5000, consent);
    const receipt = await domains.retireNativeRenderingGeneration(bindings, [], proof, handoff(), 5000, consent);
    mocks.call.mockReturnValue(mode === 'pressure' ? '{"softPressure":true,"hardPressure":false}' : '{}');
    await expect(group.allocate([receipt])).rejects.toThrow('stale');
    expect(() => api.cemQlVersion()).toThrow('retired');
    expect([...receipt.checkpoint]).toEqual([4, 8]);
    expect(mocks.dispose).toHaveBeenCalledTimes(2);
});

it('allows only native cleanup while fresh owner revocation has fenced ordinary work', async () => {
    const { generation, domains, consent, client, bindings } = await active();
    const { CemNativeGenerationGroup, claimNativeGenerationAdmission } = await import('./native-generation.js');
    const group = new CemNativeGenerationGroup([generation.generationParticipant()]);
    const proof = await domains.closeNativeRenderingGeneration(client, 5000, consent);
    const retired = await domains.retireNativeRenderingGeneration(bindings, [], proof, handoff(), 5000, consent);
    mocks.call.mockImplementation((name: string) => name === 'runtimeMemoryStatus' ? '{"softPressure":false,"hardPressure":false}' : 'released');
    const admission = await group.allocate([retired]);
    const fresh = admission.generations[0].runtime as import('./native-runtime.js').CemFreshFallbackGeneration;
    expect(() => fresh.release('compileTemplate' as never, [])).toThrow('entrypoint');
    const packageAlias = fresh.packageResourceBinding();
    const release = vi.fn(() => {
        expect(packageAlias.current()).toBe(false);
        expect(() => packageAlias.bootstrap()).toThrow('closed');
        expect(() => fresh.invoke('cemQlVersion', [])).toThrow('closed');
        expect(fresh.release('disposeCemDocument', [77])).toBe('released');
        expect(() => fresh.release('compileTemplate' as never, [])).toThrow('entrypoint');
    });
    claimNativeGenerationAdmission(admission).onRevoke(release);
    admission.revoke(); expect(release).toHaveBeenCalledOnce();
    expect(mocks.call).toHaveBeenCalledWith('disposeCemDocument', [77]);
    expect(() => fresh.release('disposeCemDocument', [77])).toThrow('retired');
});


it('keeps fallback package grants in original setup and permits exact cleanup after closure', async () => {
    const api = await import('./native-runtime.js'), d = await import('./checkpoint-domain.js');
    const options = { packageHostConfiguration: '{"schemaPackageReplacementGrants":[]}' };
    const generation = api.isolateCemFallbackGeneration(options); options.packageHostConfiguration = 'mutated';
    await generation.ready(); const lease = generation.claimOwnership(), endpoint = generation.packageResourceBinding();
    mocks.call.mockImplementation((name: string) => name === 'schemaAssetOwnerOpenV1' ? 7 : name === 'schemaAssetOwnerDisposeV1' ? true : '{}');
    endpoint.activate(new Uint8Array([1]), 'pin', [], '[]');
    expect(mocks.call).toHaveBeenCalledWith('schemaAssetOwnerOpenV1', [new Uint8Array([1]), 'pin', '{"schemaPackageReplacementGrants":[]}']);
    expect(endpoint.identity()).toBeInstanceOf(WebAssembly.Memory);
    d.cemMainThreadCheckpointDomain.closeAdmission('soft-pressure');
    expect(endpoint.current()).toBe(false); expect(() => endpoint.bootstrap()).toThrow('closed');
    expect(endpoint.release()).toBe(true);
    expect(mocks.call).toHaveBeenCalledWith('schemaAssetOwnerDisposeV1', [7]);
    await generation.disposeEmpty([lease]);
    expect(endpoint.identity()).toBeUndefined();
    expect(() => endpoint.bootstrap()).toThrow();
});

it.each(['success', 'failure', 'credential'])('checks async fallback package release before native retirement: %s', async mode => {
    const { generation, domains: d, client, bindings, consent, cleanup, domain } = await active();
    const { registerNativeGenerationResource, CemNativeGenerationGroup } = await import('./native-generation.js');
    let released = false, admitted = true;
    registerNativeGenerationResource(generation.generationParticipant(), () => ({ current: () => admitted,
        release: async () => {
            await Promise.resolve();
            if (mode === 'failure') throw Error('package release refused');
            if (mode === 'credential') admitted = false;
            released = true;
        }, prepare: async () => { throw Error('unused'); } }), () => { /* No native package in this control fixture. */ });
    new CemNativeGenerationGroup([generation.generationParticipant()]);
    cleanup.mockImplementation(() => { expect(released).toBe(true); domain.release('template', 7); });
    const proof = await d.closeNativeRenderingGeneration(client, 5000, consent);
    const retiring = d.retireNativeRenderingGeneration(bindings, [], proof, { current: () => admitted, retain: () => new Uint8Array([1]) }, 5000, consent);
    if (mode === 'success') {
        expect((await retiring).checkpoint).toEqual(new Uint8Array([1])); expect(mocks.dispose).toHaveBeenCalledOnce();
    } else {
        await expect(retiring).rejects.toThrow(mode === 'failure' ? 'package release refused' : 'stale');
        expect(cleanup).not.toHaveBeenCalled(); expect(mocks.dispose).not.toHaveBeenCalled();
    }
});


it('keeps exact fallback package cleanup available after a failed generation is fenced', async () => {
    const api = await import('./native-runtime.js'), d = await import('./checkpoint-domain.js');
    const generation = api.isolateCemFallbackGeneration({ packageHostConfiguration: '{}' }); await generation.ready();
    const endpoint = generation.packageResourceBinding();
    mocks.call.mockImplementation((name: string) => name === 'schemaAssetOwnerOpenV1' ? 17 : name === 'schemaAssetOwnerDisposeV1' ? true : '{}');
    endpoint.activate(new Uint8Array([1]), 'pin', [], '[]');
    d.cemMainThreadCheckpointDomain.revoke();
    expect(endpoint.current()).toBe(false); expect(() => endpoint.bootstrap()).toThrow();
    expect(endpoint.release()).toBe(true);
    expect(mocks.call).toHaveBeenCalledWith('schemaAssetOwnerDisposeV1', [17]);
    expect(mocks.dispose).not.toHaveBeenCalled();
});

it('publishes package allocation pressure before another ordinary operation is admitted', async () => {
    const api = await import('./native-runtime.js'), d = await import('./checkpoint-domain.js');
    const generation = api.isolateCemFallbackGeneration({ packageHostConfiguration: '{}' }); await generation.ready();
    const endpoint = generation.packageResourceBinding(); let pressure = false;
    d.cemMainThreadCheckpointDomain.sampleWith(() => ({ softPressure: pressure, hardPressure: false }));
    mocks.call.mockImplementation((name: string) => {
        if (name === 'schemaAssetOwnerOpenV1') return 23;
        if (name === 'schemaAssetOwnerDisposeV1') return true;
        if (name === 'schemaAssetOwnerActivateV1') pressure = true;
        return '{}';
    });
    endpoint.activate(new Uint8Array([1]), 'pin', [], '[]');
    expect(endpoint.current()).toBe(false);
    expect(d.cemMainThreadCheckpointDomain.pressureReason()).toBe('soft-pressure');
    expect(endpoint.release()).toBe(true);
});
