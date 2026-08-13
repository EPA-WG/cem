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
        assert.equal(
            pool.capability.controls.find((entry) => entry.control === 'hard-cancel')?.availability,
            'available',
        );
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
    await assert.rejects(createNodeWorkerPool({ hardCancelGraceMs: 9 }), /hardCancelGraceMs=9/);
});

test('Node workers run native-equivalent transform and query operations through awaitable handles', async () => {
    const pool = await createNodeWorkerPool({ workerCount: 2 });
    try {
        const commits = [];
        const transform = pool.run(xpathTransformRequest());
        transform.subscribe((event) => {
            if (event.kind === 'commit') commits.push(...event.taskIds);
        });
        const transformed = await transform;
        assert.equal(transformed.primary.sequence.items.length, 2);
        assert.deepEqual(commits, ['1', '2', '3', '4']);
        assert.equal((await transform.result()).status, 'succeeded');

        const queried = await pool.run(xpathQueryRequest());
        assert.equal(queried.language, 'xpath');
        assert.equal(queried.result.result.sequence.items.length, 2);
    } finally {
        await pool.close();
    }
});

test('single Node worker pauses at all-stop, steps one packet, and continues to one result', async () => {
    const pool = await createNodeWorkerPool({ workerCount: 1 });
    try {
        const operation = pool.run(xpathQueryRequest());
        await operation.pause();
        await operation.step();
        assert.equal((await Promise.race([operation.result(), Promise.resolve('paused')])), 'paused');
        await operation.continue();
        const terminal = await operation.result();
        assert.equal(terminal.status, 'succeeded');
        assert.equal(terminal.result.result.result.sequence.items.length, 2);
    } finally {
        await pool.close();
    }
});

test('root cancellation retains exactly one cancelled terminal', async () => {
    const pool = await createNodeWorkerPool({ workerCount: 1, hardCancelGraceMs: 10 });
    try {
        const operation = pool.run(xpathQueryRequest(600, 'count(/catalog/book)'));
        const survivor = pool.run(xpathQueryRequest());
        const replacements = [];
        let resolveExecute;
        const executeDispatched = new Promise((resolve) => {
            resolveExecute = resolve;
        });
        operation.subscribe((event) => {
            if (event.kind === 'worker-replaced') replacements.push(event);
            if (event.kind === 'dispatch' && event.stage.label === 'execute') resolveExecute();
        });
        await executeDispatched;
        const first = await operation.cancel('fixture cancellation');
        const second = await operation.cancel('losing cancellation');
        assert.equal(first.status, 'cancelled');
        assert.equal(first.reason, 'fixture cancellation');
        assert.deepEqual(second, first);
        assert.ok(replacements.length > 0);
        assert.ok(pool.workers[0].generation > 1);
        await assert.rejects(Promise.resolve(operation), (error) => error?.name === 'AbortError');
        assert.equal((await survivor).result.result.sequence.items.length, 2);
    } finally {
        await pool.close();
    }
});

function xpathTransformRequest() {
    return {
        kind: 'transform',
        data: xmlSource(),
        template: xpathSource('/catalog/book'),
        target: {
            contentType: 'application/vnd.cem.xpath-result+json',
            schema: 'https://cem.dev/ns/query/xpath/1',
        },
        targetScope: {
            defaultContentType: 'application/vnd.cem.xpath-result+json',
            schema: 'https://cem.dev/ns/query/xpath/1',
        },
        preserveSourceOffsets: true,
    };
}

function xpathQueryRequest(repetitions = 2, expression = '/catalog/book') {
    const books = Array.from({ length: repetitions }, (_, index) => `<book id="${index % 2 === 0 ? 'a' : 'b'}"/>`).join('');
    return {
        kind: 'query',
        data: xmlSource(`<catalog>${books}</catalog>`),
        query: xpathSource(expression),
    };
}

function xmlSource(xml = '<catalog><book id="a"/><book id="b"/></catalog>') {
    return {
        uri: 'memory:catalog.xml',
        bytes: [...new TextEncoder().encode(xml)],
        fromFormat: 'xml',
        identity: {
            contentType: 'application/xml',
            schema: 'https://cem.dev/ns/data/xml/1',
        },
    };
}

function xpathSource(expression) {
    return {
        uri: 'memory:query.xpath',
        bytes: [...new TextEncoder().encode(expression)],
        identity: {
            contentType: 'application/vnd.cem.xpath',
            schema: 'https://cem.dev/ns/query/xpath/1',
        },
    };
}
