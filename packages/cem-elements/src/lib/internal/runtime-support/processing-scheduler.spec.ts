import { describe, expect, it } from 'vitest';

import {
    CemProcessingFairScheduler,
    CemProcessingSchedulingTrace,
    resolveCemProcessingPoolPolicy,
} from './processing-scheduler.js';

describe('Phase 3B processing scheduler', () => {
    it('uses the accepted bounded browser defaults and validates explicit limits', () => {
        expect(resolveCemProcessingPoolPolicy({}, 12)).toEqual({
            workerCount: 8,
            maxWorkers: 8,
            queueSize: 64,
        });
        expect(resolveCemProcessingPoolPolicy({ workerCount: 2, maxWorkers: 4, queueSize: 7 }, 16)).toEqual({
            workerCount: 2,
            maxWorkers: 4,
            queueSize: 7,
        });
        expect(() => resolveCemProcessingPoolPolicy({ workerCount: 5, maxWorkers: 4 }, 16)).toThrow(
            /workerCount=5/
        );
        expect(() => resolveCemProcessingPoolPolicy({ queueSize: 0 }, 16)).toThrow(/queueSize=0/);
    });

    it('preserves FIFO order per root and dispatches fairly across roots', () => {
        const scheduler = new CemProcessingFairScheduler<string>(8);
        scheduler.registerOwner(1);
        scheduler.registerOwner(2);
        scheduler.enqueue(1, 'root-1-a');
        scheduler.enqueue(1, 'root-1-b');
        scheduler.enqueue(2, 'root-2-a');
        scheduler.enqueue(2, 'root-2-b');

        expect([
            scheduler.dequeue()?.value,
            scheduler.dequeue()?.value,
            scheduler.dequeue()?.value,
            scheduler.dequeue()?.value,
        ]).toEqual(['root-1-a', 'root-2-a', 'root-1-b', 'root-2-b']);
    });

    it('rejects overflow, removes cancelled work, and forgets released roots', () => {
        const scheduler = new CemProcessingFairScheduler<{ jobId: number }>(3);
        scheduler.registerOwner(1);
        scheduler.registerOwner(2);
        scheduler.enqueue(1, { jobId: 1 });
        scheduler.enqueue(2, { jobId: 2 });
        scheduler.enqueue(1, { jobId: 3 });

        expect(() => scheduler.enqueue(2, { jobId: 4 })).toThrow(/queue capacity 3/);
        expect(scheduler.cancel((entry) => entry.value.jobId === 3)).toEqual([
            { ownerId: 1, value: { jobId: 3 } },
        ]);
        expect(scheduler.removeOwner(2)).toEqual([
            { ownerId: 2, value: { jobId: 2 } },
        ]);
        expect(scheduler.dequeue()).toEqual({ ownerId: 1, value: { jobId: 1 } });
        expect(scheduler.dequeue()).toBeUndefined();
    });

    it('emits clone-safe deterministic sequence traces without wall-clock fields', () => {
        const run = () => {
            const trace = new CemProcessingSchedulingTrace();
            trace.record({
                kind: 'enqueue',
                ownerScopeId: 1,
                scopePolicyStamp: 'scope-policy-v1',
                workerSlot: 1,
                jobId: 1,
                operation: 'compile',
            });
            trace.record({
                kind: 'dispatch',
                ownerScopeId: 1,
                scopePolicyStamp: 'scope-policy-v1',
                workerSlot: 1,
                jobId: 1,
                operation: 'compile',
            });
            return trace.snapshot();
        };

        expect(run()).toEqual(run());
        expect(run().map((event) => event.sequence)).toEqual([1, 2]);
        expect(structuredClone(run())).toEqual(run());
        expect(JSON.stringify(run())).not.toMatch(/time|duration|date/i);
    });
});
