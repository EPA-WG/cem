import { describe, expect, it } from 'vitest';
import { CemCommittedCheckpointStore, type CemCommitReceipt } from './committed-checkpoint.js';

const receipt = (commitSequence = 1, generation = 1): CemCommitReceipt => ({
    runtimeId: 'runtime', generation, commitSequence, policyStamp: 'session',
    owners: [{ instanceId: 'editor', declarationIdentity: 'source:1', dataRevision: `draft:${commitSequence}`, acceptedEventSequence: commitSequence }],
    operations: [{ instanceId: 'editor', operationId: 'save:1', outcome: 'unknown' }],
});
const seal = (candidate: string) => candidate; // transport tests; native validation has separate fixtures
const text = (store: CemCommittedCheckpointStore) => new TextDecoder().decode(store.lastCommitted()!.bytes);

describe('host-owned committed checkpoints', () => {
    it('reserves an explicit workspace capacity without widening default stores or receipt budgets', async () => {
        const bytes = 4 * 1024 * 1024;
        for (const invalid of [0, NaN, 2 * 1024 * 1024, bytes + 1])
            expect(() => new CemCommittedCheckpointStore('runtime', invalid)).toThrow('budget');
        const normal = new CemCommittedCheckpointStore('runtime');
        expect(() => normal.acknowledge({ ...receipt(), maxBytes: bytes })).toThrow('stale');
        normal.acknowledge(receipt());
        await expect(normal.capture('é'.repeat(600_000), seal)).rejects.toThrow('limit');
        const workspace = new CemCommittedCheckpointStore('runtime', bytes);
        expect(() => workspace.acknowledge(receipt())).toThrow('stale');
        workspace.acknowledge({ ...receipt(), maxBytes: bytes });
        await workspace.capture('é'.repeat(600_000), seal);
        expect(workspace.current()!.bytes.byteLength).toBe(1_200_000);
        await expect(workspace.capture('é'.repeat(bytes / 2 + 1), seal)).rejects.toThrow('limit');
        expect(workspace.current()!.bytes.byteLength).toBe(1_200_000);
        normal.dispose(); workspace.dispose();
    });
    it('copies immutable bytes and receipts and retains no WASM memory view', async () => {
        const store = new CemCommittedCheckpointStore('runtime');
        const ack = receipt(); store.acknowledge(ack); ack.owners[0].dataRevision = 'mutated';
        await store.capture('unsaved é text', seal);
        expect(store.current()!.receipt.owners[0].dataRevision).toBe('draft:1');
        const read = store.current()!; read.bytes.fill(0); read.receipt.operations.length = 0;
        expect(text(store)).toBe('unsaved é text');
        expect(store.current()!.receipt.operations[0].outcome).toBe('unknown');
    });
    it('preserves last good bytes after failure, overflow and cancellation; blocks stale recovery', async () => {
        const store = new CemCommittedCheckpointStore('runtime'); store.acknowledge(receipt());
        await store.capture('good', seal);
        await expect(store.capture('bad', () => { throw new Error('native refused'); })).rejects.toThrow();
        await expect(store.capture('é'.repeat(600_000), seal)).rejects.toThrow('limit');
        expect(text(store)).toBe('good');
        const cancel = new AbortController();
        await expect(store.capture('cancelled', candidate => { cancel.abort(); return candidate; }, cancel.signal)).rejects.toThrow();
        expect(text(store)).toBe('good');
        store.acknowledge(receipt(2));
        expect(store.current()).toBeUndefined();
        expect(text(store)).toBe('good');
        await store.capture('new good', seal);
        expect(text(store)).toBe('new good');
    });
    it('fences pending captures against edits, owner removal, generation and session disposal', async () => {
        for (const invalidate of [(s: CemCommittedCheckpointStore) => s.invalidate(),
            (s: CemCommittedCheckpointStore) => s.acknowledge(receipt(1, 2)),
            (s: CemCommittedCheckpointStore) => s.dispose()]) {
            const store = new CemCommittedCheckpointStore('runtime'); store.acknowledge(receipt());
            await store.capture('good', seal);
            let release!: (text: string) => void;
            const pending = store.capture('pending', () => new Promise(resolve => { release = resolve; }));
            await expect(store.capture('overlap', seal)).rejects.toThrow('unavailable');
            invalidate(store); release('stale'); await expect(pending).rejects.toThrow('stale');
            expect(store.current()).toBeUndefined();
        }
    });
    it('rejects stale/wrong runtime receipts and permits a new generation', () => {
        const store = new CemCommittedCheckpointStore('runtime');
        expect(() => store.acknowledge({ ...receipt(), runtimeId: 'other' })).toThrow();
        expect(() => store.acknowledge(receipt(Number.MAX_SAFE_INTEGER + 1))).toThrow();
        store.acknowledge(receipt(4));
        expect(() => store.acknowledge(receipt(3))).toThrow();
        store.acknowledge(receipt(1, 2));
        expect(() => store.acknowledge(receipt(5, 1))).toThrow();
        store.dispose(); expect(() => store.acknowledge(receipt(2, 2))).toThrow();
    });
});


describe('checkpoint retirement handoff', () => {
    it('refuses unknown outcomes and retains independent current checkpoint bytes', async () => {
        const store = new CemCommittedCheckpointStore('runtime'); store.acknowledge(receipt());
        await store.capture('unknown action', seal);
        expect(() => store.handoff()).toThrow('operations.unknown');
        const ack = receipt(2); ack.operations[0].outcome = 'acknowledged';
        store.acknowledge(ack); await store.capture('portable checkpoint and terminal journal', seal);
        const handoff = store.handoff(); const retained = handoff.retain(); retained.fill(0);
        expect(new TextDecoder().decode(handoff.retain())).toBe('portable checkpoint and terminal journal');
        store.invalidate(); expect(handoff.current()).toBe(false);
        expect(() => handoff.retain()).toThrow('stale');
        expect(text(store)).toBe('portable checkpoint and terminal journal');
    });
    it('fences recapture, pending capture and disposal without dropping last good bytes', async () => {
        const store = new CemCommittedCheckpointStore('runtime'); const ack = receipt(); ack.operations = [];
        store.acknowledge(ack); await store.capture('good', seal);
        const handoff = store.handoff();
        let finish!: (text: string) => void;
        const pending = store.capture('next', () => new Promise(resolve => { finish = resolve; }));
        expect(handoff.current()).toBe(false); expect(() => store.handoff()).toThrow();
        finish('next'); await pending; expect(handoff.current()).toBe(false);
        const next = store.handoff(); store.dispose(); expect(next.current()).toBe(false);
    });
});
