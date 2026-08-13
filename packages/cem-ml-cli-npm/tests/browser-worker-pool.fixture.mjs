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
    } else if (scenario === 'main-thread-fallback') {
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
            workerCount: scenario === 'single-worker' ? 1 : 2,
            maxWorkers: 4,
        });
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
