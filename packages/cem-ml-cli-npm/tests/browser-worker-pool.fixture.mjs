import { createBrowserWorkerPool } from '../dist/browser.js';

globalThis.runCemMlBrowserFixture = async (scenario) => {
    if (scenario === 'bounds') {
        const failures = [];
        for (const options of [
            { workerCount: 0 },
            { workerCount: 5, maxWorkers: 4 },
            { maxWorkers: 257 },
            { startupTimeoutMs: 0 },
        ]) {
            try {
                await createBrowserWorkerPool(options);
                failures.push('accepted');
            } catch (error) {
                failures.push(error instanceof Error ? error.message : String(error));
            }
        }
        return failures;
    }

    const NativeWorker = globalThis.Worker;
    let constructionAttempts = 0;
    if (scenario === 'single-worker-fallback') {
        globalThis.Worker = class extends NativeWorker {
            constructor(url, options) {
                constructionAttempts += 1;
                if (constructionAttempts === 1) {
                    throw new DOMException('fixture rejects the multi-worker batch', 'NotSupportedError');
                }
                super(url, options);
            }
        };
    } else if (scenario === 'main-thread-fallback' || scenario === 'operation-main-thread') {
        globalThis.Worker = class extends NativeWorker {
            constructor() {
                constructionAttempts += 1;
                throw new DOMException('fixture rejects dedicated workers', 'NotSupportedError');
            }
        };
    } else if (scenario === 'workers-unavailable') {
        globalThis.Worker = undefined;
    }

    let pool;
    try {
        pool = await createBrowserWorkerPool({
            workerCount: scenario === 'single-worker' || scenario === 'operation-single-worker' ? 1 : 2,
            maxWorkers: 4,
            hardCancelGraceMs: scenario.startsWith('operation-') ? 10 : undefined,
            onWorkerFailure: scenario.startsWith('operation-')
                ? (failure) => console.error(`operation worker failure: ${JSON.stringify(failure)}`)
                : undefined,
        });
        if (scenario.startsWith('operation-')) {
            return await runOperationFixture(pool, scenario);
        }
        return {
            mode: pool.mode,
            fallbackReason: pool.fallbackReason,
            size: pool.size,
            topology: pool.capability.executorTopology,
            effectiveMaxWorkers: pool.capability.effectiveMaxWorkers,
            commonVersion: pool.capability.commonVersion,
            workers: pool.workers.map(({ slot, generation, runtimeInstanceId, commonVersion }) => ({
                slot,
                generation,
                runtimeInstanceId,
                commonVersion,
            })),
            mainThread: pool.mainThread,
            constructionAttempts,
            sharedArrayBufferAvailable: typeof SharedArrayBuffer === 'function',
            hardCancelAvailability: pool.capability.controls.find(
                (entry) => entry.control === 'hard-cancel',
            )?.availability,
        };
    } finally {
        await pool?.close();
        globalThis.Worker = NativeWorker;
    }
};

async function runOperationFixture(pool, scenario) {
    const commits = [];
    const transformedOperation = pool.run(xpathTransformRequest());
    transformedOperation.subscribe((event) => {
        if (event.kind === 'commit') commits.push(...event.taskIds);
    });
    const transformed = await transformedOperation;
    const queryOperation = pool.run(xpathQueryRequest());
    if (scenario !== 'operation-pool') {
        await queryOperation.pause();
        await queryOperation.step();
        await queryOperation.continue();
    }
    const queried = await queryOperation;

    const cancelledOperation = pool.run(xpathQueryRequest(600, 'count(/catalog/book)'));
    const replacementEvents = [];
    let resolveExecute;
    const executeDispatched = new Promise((resolve) => {
        resolveExecute = resolve;
    });
    cancelledOperation.subscribe((event) => {
        if (event.kind === 'worker-replaced') replacementEvents.push(event);
        if (event.kind === 'dispatch' && event.stage.label === 'execute') resolveExecute();
    });
    await executeDispatched;
    const cancelled = await cancelledOperation.cancel('browser fixture cancellation');
    let awaitErrorName;
    try {
        await cancelledOperation;
    } catch (error) {
        awaitErrorName = error?.name;
    }
    return {
        mode: pool.mode,
        transformedItemCount: transformed.primary.sequence.items.length,
        queriedItemCount: queried.result.result.sequence.items.length,
        commits,
        queryTerminal: await queryOperation.result(),
        cancelled,
        awaitErrorName,
        replacementEvents,
        workers: pool.workers.map(({ slot, generation }) => ({ slot, generation })),
    };
}

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
    const books = Array.from({ length: repetitions }, (_, index) => `<book id="${index}"/>`).join('');
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
