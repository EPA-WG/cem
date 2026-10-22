import { describe, expect, it, vi } from 'vitest';
// Test the canonical lifecycle source; all shipped copies use one realm registry.
// eslint-disable-next-line @nx/enforce-module-boundaries
import { CemNativeRecoveryScope, registerNativeGeneration, completeNativeRetirement } from '../../../../../cem_ql/runtime/native-generation.mjs';
const participant = () => registerNativeGeneration({}, crypto.randomUUID(), async () => ({
    identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, current: () => true, discard() { /* No native heap in this unit contract. */ },
}));
const member = (scope: CemNativeRecoveryScope, id: string, p = participant()) => scope.enroll({ id, kind: 'rendering',
    prepare: async () => ({ participants: [p], current: () => true }) });

describe('complete native recovery enrollment', () => {
    it('inspects without calling endpoints and snapshots original host controls', async () => {
        const scope = new CemNativeRecoveryScope(), p = participant();
        const prepare = vi.fn(async () => ({ participants: [p], current: () => true }));
        const endpoint = { id: 'preview', kind: 'live-preview' as const, prepare };
        scope.enroll(endpoint); endpoint.prepare = vi.fn(() => { throw Error('mutated'); });
        expect(scope.inspect().members).toEqual([{ id: 'preview', kind: 'live-preview' }]); expect(prepare).not.toHaveBeenCalled();
        const prepared = await scope.prepare(); expect(prepared.current()).toBe(true); expect(prepare).toHaveBeenCalledOnce();
        prepared.createGenerationGroup(); expect(() => prepared.createGenerationGroup()).toThrow('enrollment-stale');
    });
    it('allocates only the complete original participant set and never trusts copied receipts', async () => {
        const scope = new CemNativeRecoveryScope(), a = participant(), b = participant(); member(scope, 'root', a); member(scope, 'preview', b);
        const prepared = await scope.prepare(), group = prepared.createGenerationGroup();
        const ar = completeNativeRetirement(a, new Uint8Array([1])), br = completeNativeRetirement(b, new Uint8Array([2]));
        await expect(group.allocate([ar])).rejects.toThrow('coverage');
        await expect(group.allocate([{ ...ar }, br])).rejects.toThrow('receipt');
        const admission = await group.allocate([br, ar]); expect(admission.generations).toHaveLength(2); admission.revoke();
    });
    it('refuses every member when a command outcome is unknown or a heap is unsupported', async () => {
        for (const reason of ['unknown-command-outcome', 'command-invocation-singleton']) {
            const scope = new CemNativeRecoveryScope(); member(scope, 'root');
            scope.enroll({ id: 'commands', kind: 'command-service', prepare: async () => { throw Error(reason); } });
            await expect(scope.prepare()).rejects.toThrow(reason); expect(scope.inspect().members).toHaveLength(2);
        }
    });
    it('rejects overlapping physical generations and structural participant copies', async () => {
        const scope = new CemNativeRecoveryScope(), p = participant(); member(scope, 'a', p); member(scope, 'b', p);
        await expect(scope.prepare()).rejects.toThrow('overlap');
        const foreign = new CemNativeRecoveryScope(); member(foreign, 'copy', { ...p });
        await expect(foreign.prepare()).rejects.toThrow('coverage');
    });
    it('invalidates enrollment before grouping and through asynchronous allocation', async () => {
        const scope = new CemNativeRecoveryScope(); const lease = member(scope, 'root');
        const prepared = await scope.prepare(); lease.changed(); expect(prepared.current()).toBe(false);
        expect(() => prepared.createGenerationGroup()).toThrow('enrollment-stale');
        let finish!: () => void, discarded = false;
        const next = new CemNativeRecoveryScope(), p = registerNativeGeneration({}, crypto.randomUUID(), async () => {
            await new Promise<void>(resolve => { finish = resolve; });
            return { identity: {}, runtimeId: crypto.randomUUID(), runtime: {}, current: () => true, discard: () => { discarded = true; } };
        });
        const registration = member(next, 'preview', p), plan = await next.prepare(), group = plan.createGenerationGroup();
        const allocating = group.allocate([completeNativeRetirement(p, new Uint8Array([1]))]);
        await vi.waitFor(() => expect(finish).toBeTypeOf('function')); registration.remove(); finish();
        await expect(allocating).rejects.toThrow('enrollment-stale'); expect(discarded).toBe(true);
    });
    it('refuses membership changes, late witnesses and timeouts during preparation', async () => {
        const scope = new CemNativeRecoveryScope(), p = participant(); let finish!: () => void;
        scope.enroll({ id: 'preview', kind: 'live-preview', prepare: async () => {
            await new Promise<void>(resolve => { finish = resolve; }); return { participants: [p], current: () => true };
        } });
        const preparing = scope.prepare(); await expect(scope.prepare()).rejects.toThrow('enrollment'); member(scope, 'late'); finish();
        await expect(preparing).rejects.toThrow('enrollment-stale');
        const timeout = new CemNativeRecoveryScope(); let signal!: AbortSignal;
        timeout.enroll({ id: 'stalled', kind: 'command-service', prepare: async stop => {
            signal = stop; return new Promise(() => undefined);
        } });
        await expect(timeout.prepare(5)).rejects.toThrow('timeout'); expect(signal.aborted).toBe(true);
        scope.dispose(); expect(scope.inspect().members).toEqual([]); await expect(scope.prepare()).rejects.toThrow('enrollment');
    });
});
