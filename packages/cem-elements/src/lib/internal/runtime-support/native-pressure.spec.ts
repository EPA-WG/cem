import { describe, expect, it, vi } from 'vitest';
const native = vi.hoisted(() => ({ pressure: false, calls: 0, reads: 0, closeAt: Infinity }));
vi.mock('../../../../../cem_ql/dist/wasm/cem_ql.js', () => ({
    default: async () => ({ memory: new WebAssembly.Memory({ initial: 1 }) }),
    runtimeMemoryStatus: () => JSON.stringify({ softPressure: native.pressure || ++native.reads >= native.closeAt, hardPressure: false }),
}));
import { ensureRuntimeReady, exportRetainedNativeSessionView, RetainedXsltComponent } from './cem-ql-render.js';
import { cemMainThreadCheckpointDomain, nativeCheckpointWork, nativeCheckpointDrain, nativeCheckpointDrainCurrent, nativeCheckpointSnapshot } from './checkpoint-domain.js';

describe('shared fallback native pressure', () => {
    it('samples the initialized native heap before support admission, drains work and admits only cleanup', async () => {
        await ensureRuntimeReady();
        const client = cemMainThreadCheckpointDomain.client();
        let finish!: () => void;
        const running = nativeCheckpointWork(() => new Promise<void>(resolve => { finish = resolve; }))();
        // Cross pressure between client enrollment and work admission, after
        // the first check has already succeeded. The refused client must retire.
        native.reads = 0; native.closeAt = 2;
        const fresh = nativeCheckpointWork(async () => { native.calls++; });
        await expect(fresh()).rejects.toThrow('cem.runtime.memory.closed');
        expect(() => exportRetainedNativeSessionView(1, { action: 'view', expression: 'input',
            handle: { instanceId: 'editor', scopePolicyStamp: 'private', sessionKey: 'source', sourceRevision: '1' } })).toThrow('cem.runtime.memory.closed');
        expect(() => new RetainedXsltComponent('source', 'https://example.test/source.xslt', '{}', '[]')).toThrow('cem.runtime.memory.closed');
        expect(native.calls).toBe(0);
        const drained = nativeCheckpointDrain(client);
        finish(); await running; const proof = await drained;
        expect(nativeCheckpointDrainCurrent(proof)).toBe(true);
        expect(nativeCheckpointSnapshot(new Map([[client, ['editor']]]))).toBeDefined();
        native.pressure = false; await expect(fresh()).rejects.toThrow('closed');
        await nativeCheckpointWork(async () => { native.calls++; }, true)();
        expect(native.calls).toBe(1); expect(nativeCheckpointDrainCurrent(proof)).toBe(false);
        cemMainThreadCheckpointDomain.remove(client);
    });
});
