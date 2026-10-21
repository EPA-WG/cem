import { beforeEach, describe, expect, it, vi } from 'vitest';
const mocks = vi.hoisted(() => ({ dispose: vi.fn(), initialize: vi.fn(), call: vi.fn(), legacy: vi.fn() }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({ default: mocks.legacy, cemQlVersion: () => 'legacy' }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql_factory.js', () => ({ createCemQlWasmGeneration: () => {
    let dead = false; const memory = new WebAssembly.Memory({ initial: 1 });
    return { initialize: mocks.initialize, memory: () => dead ? undefined : memory,
        call: mocks.call, dispose: (current?: () => boolean) => { mocks.dispose(); if (current && !current()) throw new Error('ownership-stale'); dead = true; } };
} }));
beforeEach(() => { vi.resetModules(); vi.clearAllMocks(); mocks.initialize.mockResolvedValue(undefined); mocks.call.mockReturnValue('owned'); });
async function selected() {
    const api = await import('./native-runtime.js'), domains = await import('./checkpoint-domain.js');
    const generation = api.isolateCemFallbackGeneration(); await generation.ready();
    return { api, domains, generation, lease: generation.claimOwnership() };
}
describe('owned fallback generation and embedding consent', () => {
    it('selects once before use, keeps generated exports private and fences saved aliases after empty disposal', async () => {
        const { api, generation, lease } = await selected(); const alias = api.cemQlVersion;
        expect(Object.keys(generation).sort()).toEqual(['claimOwnership', 'disposeEmpty', 'ready', 'releaseOwnership', 'runtimeId'].sort());
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
