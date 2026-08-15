import {
    createBrowserCommandServiceClient,
    createBrowserWorkerPool,
} from '../dist/browser.js';
import parseRunPlan from './browser-command-run-plan.fixture.json' with { type: 'json' };

globalThis.runCemMlBrowserFixture = async (scenario) => {
    if (scenario === 'command-service') return runCommandServiceFixture();
    if (scenario === 'command-cancellation') return runCommandCancellationFixture();
    if (scenario === 'command-close') return runCommandCloseFixture();
    if (scenario === 'command-worker-failure') return runCommandWorkerFailureFixture();
    if (scenario === 'command-worker-unavailable') return runCommandWorkerUnavailableFixture();
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

async function runCommandServiceFixture() {
    const sourceUri = 'memory:fixture.css';
    const outputUris = ['memory:fixture.cem', 'memory:fixture-secondary.cem'];
    const sourceBytes = new TextEncoder().encode('.card { color: red; }');
    const version = { revision: 1, sha256: await sha256(sourceBytes) };
    const events = [];
    const writes = new Map();
    const host = {
        currentRevision: async ({ requestId, project }) => {
            await Promise.resolve();
            events.push(['ledger', requestId]);
            if (requestId === 'browser-command-host-failure') {
                throw new Error('fixture revision ledger unavailable');
            }
            return {
                project,
                resourceVersions: requestId === 'browser-command-parse' ? { [sourceUri]: version } : {},
            };
        },
        readResource: async (request) => {
            await Promise.resolve();
            events.push(['read', request.uri]);
            return {
                version,
                bytes: [...sourceBytes],
                identity: {
                    contentType: 'text/css',
                    schema: 'https://cem.dev/ns/data/css/1',
                    defaultNamespace: null,
                    namespaces: {},
                    baseUri: null,
                },
            };
        },
        prepareWrite: async (request, bytes) => {
            await Promise.resolve();
            const token = `write:${writes.size + 1}`;
            writes.set(token, { request, bytes: new Uint8Array(bytes), committed: false });
            events.push(['prepare', request.uri]);
            return { token };
        },
        commitWrite: async (token) => {
            await Promise.resolve();
            const write = writes.get(token);
            if (write === undefined) throw new Error(`unknown fixture write ${token}`);
            write.committed = true;
            events.push(['commit', write.request.uri]);
            return { uri: write.request.uri };
        },
        rollbackWrite: async (token) => {
            await Promise.resolve();
            writes.delete(token);
            events.push(['rollback', token]);
        },
    };
    const client = await createBrowserCommandServiceClient({ host });
    try {
        const progress = [];
        const handle = client.execute(parseCommandRequest(version), {
            onProgress: (event) => progress.push(event),
        });
        const subscribedProgress = [];
        const unsubscribe = handle.subscribe((event) => subscribedProgress.push(event.sequence));
        const result = await handle;
        unsubscribe();
        const artifact = result.artifacts.items[0];
        const firstRead = await handle.readArtifact(artifact, { offset: 0, maxBytes: 16 });
        const firstByte = firstRead.bytes[0];
        firstRead.bytes[0] ^= 0xff;
        const repeatedRead = await handle.readArtifact(artifact, { offset: 0, maxBytes: 16 });
        const firstDisposed = await handle.disposeArtifact(artifact);
        const repeatedDisposed = await handle.disposeArtifact(artifact);
        const remainingDisposed = await handle.dispose();

        const concurrentProgress = new Map();
        const concurrent = ['browser-command-version-a', 'browser-command-version-b'].map((requestId) => {
            const current = [];
            concurrentProgress.set(requestId, current);
            return client.execute(versionCommandRequest(requestId), {
                onProgress: (event) => current.push(event.sequence),
            });
        });
        const concurrentResults = await Promise.all(concurrent);
        const callbackFailure = await client
            .execute(versionCommandRequest('browser-command-host-failure'))
            .result()
            .then(
                () => ({ resolved: true }),
                (error) => ({ resolved: false, code: error?.code, message: error?.message }),
            );

        return {
            worker: client.worker,
            runtime: client.capability.runtime,
            topology: client.capability.executorTopology,
            effectiveMaxWorkers: client.capability.effectiveMaxWorkers,
            abiIdentity: client.capability.abiIdentity,
            resultIdentity: result.identity,
            status: result.status,
            originalArtifactCount: result.artifacts.originalCount,
            artifactByteLength: artifact.byteLength,
            firstReadByteLength: firstRead.metadata.byteLength,
            copiedArtifactBytes: repeatedRead.bytes[0] === firstByte,
            disposeDispositions: [
                firstDisposed.disposition,
                repeatedDisposed.disposition,
                remainingDisposed.disposition,
            ],
            progress: progress.map(({ sequence, stage, status }) => [sequence, stage, status]),
            subscribedProgress,
            events,
            outputUris,
            writes: [...writes.values()].map(({ request, bytes, committed }) => ({
                uri: request.uri,
                byteLength: bytes.byteLength,
                committed,
            })),
            concurrent: concurrentResults.map(({ requestId, status }) => ({ requestId, status })),
            concurrentProgress: Object.fromEntries(concurrentProgress),
            callbackFailure,
        };
    } finally {
        await client.close();
    }
}

async function runCommandCancellationFixture() {
    let resolveRevision;
    let revisionRequested;
    const requested = new Promise((resolve) => {
        revisionRequested = resolve;
    });
    const request = versionCommandRequest('browser-command-cancel');
    const host = versionCommandHost(({ project }) =>
        new Promise((resolve) => {
            resolveRevision = () => resolve({ project, resourceVersions: {} });
            revisionRequested();
        }),
    );
    const client = await createBrowserCommandServiceClient({ host });
    try {
        const progress = [];
        const handle = client.execute(request, { onProgress: (event) => progress.push(event) });
        await requested;
        const acknowledgement = await handle.cancel('browser fixture cancellation');
        resolveRevision();
        const result = await handle.result();
        return {
            acknowledgement,
            status: result.status,
            exitCode: result.exitCode,
            progress: progress.map(({ sequence, stage, status }) => [sequence, stage, status]),
        };
    } finally {
        await client.close();
    }
}

async function runCommandCloseFixture() {
    let resolveRevision;
    let revisionRequested;
    const requested = new Promise((resolve) => {
        revisionRequested = resolve;
    });
    const host = versionCommandHost(({ project }) =>
        new Promise((resolve) => {
            resolveRevision = () => resolve({ project, resourceVersions: {} });
            revisionRequested();
        }),
    );
    const client = await createBrowserCommandServiceClient({ host });
    const handle = client.execute(versionCommandRequest('browser-command-close'));
    await requested;
    const terminal = handle.result().then(
        () => ({ resolved: true }),
        (error) => ({ resolved: false, code: error?.code, message: error?.message }),
    );
    await client.close();
    resolveRevision();
    const failure = await terminal;
    let postCloseCode;
    try {
        client.execute(versionCommandRequest('browser-command-after-close'));
    } catch (error) {
        postCloseCode = error?.code;
    }
    return { failure, postCloseCode };
}

async function runCommandWorkerFailureFixture() {
    const NativeWorker = globalThis.Worker;
    let corrupted = false;
    globalThis.Worker = class extends NativeWorker {
        postMessage(message, transfer) {
            if (!corrupted && message?.type === 'cem-command-execute') {
                corrupted = true;
                super.postMessage({ type: 'cem-fixture-invalid-command' });
                return;
            }
            super.postMessage(message, transfer);
        }
    };
    const failures = [];
    let client;
    try {
        client = await createBrowserCommandServiceClient({
            host: versionCommandHost(),
            onWorkerFailure: (failure) => failures.push(failure),
        });
        const handle = client.execute(versionCommandRequest('browser-command-worker-failure'));
        const terminal = await handle.result().then(
            () => ({ resolved: true }),
            (error) => ({ resolved: false, code: error?.code }),
        );
        return { terminal, failures, corrupted };
    } finally {
        await client?.close();
        globalThis.Worker = NativeWorker;
    }
}

async function runCommandWorkerUnavailableFixture() {
    const NativeWorker = globalThis.Worker;
    let callbacks = 0;
    globalThis.Worker = undefined;
    try {
        await createBrowserCommandServiceClient({
            host: versionCommandHost(() => {
                callbacks += 1;
                throw new Error('main-thread execution is forbidden');
            }),
        });
        return { accepted: true, callbacks };
    } catch (error) {
        return { accepted: false, code: error?.code, callbacks };
    } finally {
        globalThis.Worker = NativeWorker;
    }
}

function versionCommandHost(currentRevision) {
    const unexpected = (name) => async () => {
        throw new Error(`${name} must not run for version-capabilities`);
    };
    return {
        currentRevision:
            currentRevision ??
            (async ({ project }) => ({
                project,
                resourceVersions: {},
            })),
        readResource: unexpected('readResource'),
        prepareWrite: unexpected('prepareWrite'),
        commitWrite: unexpected('commitWrite'),
        rollbackWrite: unexpected('rollbackWrite'),
    };
}

function versionCommandRequest(requestId) {
    return {
        protocolVersion: 1,
        requestId,
        project: { projectId: 'browser-fixture', revision: 1 },
        resourceVersions: {},
        operation: { kind: 'version-capabilities' },
        runPlan: null,
        resources: {},
        policyStamp: {
            resolver: 'browser-fixture-resolver',
            safety: 'browser-fixture-safety',
            budget: 'browser-fixture-budget',
        },
    };
}

function parseCommandRequest(version) {
    return {
        protocolVersion: 1,
        requestId: 'browser-command-parse',
        project: { projectId: 'browser-fixture', revision: 1 },
        resourceVersions: { 'memory:fixture.css': version },
        operation: {
            kind: 'parse',
            inputId: 'input:0',
            projection: 'ast',
            preserveSourceOffsets: true,
        },
        runPlan: parseRunPlan,
        resources: {},
        policyStamp: {
            resolver: 'browser-fixture-resolver',
            safety: 'browser-fixture-safety',
            budget: 'browser-fixture-budget',
        },
    };
}

async function sha256(bytes) {
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    return [...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

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
