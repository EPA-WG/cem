import assert from 'node:assert/strict';
import test from 'node:test';

import { createNodeWorkerPool } from '../dist/node.js';

test('bounded Node pool initializes one isolated runtime per worker', async () => {
    const pool = await createNodeWorkerPool({ workerCount: 2, maxWorkers: 4 });
    try {
        assert.equal(pool.mode, 'pool');
        assert.equal(pool.size, 2);
        assert.equal(pool.capability.executorTopology, 'node-worker-pool');
        assert.equal(pool.capability.effectiveMaxWorkers, 2);
        assert.deepEqual(
            pool.workers.map(({ slot, generation }) => ({ slot, generation })),
            [
                { slot: 1, generation: 1 },
                { slot: 2, generation: 1 },
            ],
        );
        assert.equal(new Set(pool.workers.map((worker) => worker.threadId)).size, 2);
        assert.equal(new Set(pool.workers.map((worker) => worker.runtimeInstanceId)).size, 2);
        assert.equal(new Set(pool.workers.map((worker) => worker.commonVersion)).size, 1);
    } finally {
        await pool.close();
        await pool.close();
    }
});

test('explicit one-worker mode preserves the accepted fallback surface', async () => {
    const pool = await createNodeWorkerPool({ workerCount: 1 });
    try {
        assert.equal(pool.mode, 'single-worker');
        assert.equal(pool.size, 1);
        assert.equal(pool.capability.effectiveMaxWorkers, 1);
        assert.equal(pool.workers[0].slot, 1);
        assert.equal(pool.workers[0].generation, 1);
    } finally {
        await pool.close();
    }
});

test('worker and startup policy bounds fail before spawning', async () => {
    await assert.rejects(createNodeWorkerPool({ workerCount: 0 }), /workerCount=0/);
    await assert.rejects(createNodeWorkerPool({ workerCount: 5, maxWorkers: 4 }), /workerCount=5/);
    await assert.rejects(createNodeWorkerPool({ maxWorkers: 257 }), /maxWorkers=257/);
    await assert.rejects(createNodeWorkerPool({ startupTimeoutMs: 0 }), /startupTimeoutMs=0/);
});
