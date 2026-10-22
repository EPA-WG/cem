import {
    CemNativeGenerationGroup,
    buildBrowserCommandInvocation,
    createBrowserCommandServiceClient,
    createBrowserWorkerPool,
    parseCemMlCommand,
    projectBrowserCommandPresentation,
} from '../dist/browser.js';
import parseRunPlan from './browser-command-run-plan.fixture.json' with { type: 'json' };
import {
    commandCases,
    fixtureDirectory,
    fixtureFiles,
    normalizeWasmCommandResult,
} from './command-all-operations.fixture.mjs';

globalThis.runCemMlBrowserFixture = async (scenario) => {
    if (scenario === 'command-host-configuration') return runHostConfigurationFixture();
    if (scenario === 'command-native-checkpoint') return runNativeCheckpointFixture();
    if (scenario === 'command-native-retirement') return runNativeInventoryFixture('retirement');
    if (scenario === 'command-native-closure') return runNativeInventoryFixture(true);
    if (scenario === 'command-native-inventory') return runNativeInventoryFixture();
    if (scenario === 'command-native-pressure') return runNativePressureFixture();
    if (scenario === 'command-service') return runCommandServiceFixture();
    if (scenario === 'command-all-operations') return runCommandAllOperationsFixture();
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

async function runCommandAllOperationsFixture() {
    const encoder = new TextEncoder();
    const resources = new Map(
        Object.entries(fixtureFiles).map(([name, source]) => [
            `file://${fixtureDirectory}/${name}`,
            encoder.encode(source),
        ]),
    );
    const ledgers = new Map();
    const writes = new Map();
    let nextWrite = 1;
    const client = await createBrowserCommandServiceClient({
        host: {
            currentRevision: async ({ requestId }) => {
                const ledger = ledgers.get(requestId);
                if (ledger === undefined) throw new Error(`missing fixture ledger ${requestId}`);
                return ledger;
            },
            readResource: async (request) => {
                throw new Error(`inline fixture unexpectedly read ${request.uri}`);
            },
            prepareWrite: async (request, bytes) => {
                const token = `all-operation:${nextWrite++}`;
                writes.set(token, { request, bytes: new Uint8Array(bytes), committed: false });
                return { token };
            },
            commitWrite: async (token) => {
                const write = writes.get(token);
                if (write === undefined) throw new Error(`unknown all-operation write ${token}`);
                write.committed = true;
                return { uri: write.request.uri };
            },
            rollbackWrite: async (token) => {
                writes.delete(token);
            },
        },
    });
    try {
        const summaries = [];
        for (const fixtureCase of commandCases) {
            const requestId = `browser-all-${fixtureCase.name}`;
            const parsed = parseCemMlCommand(fixtureCase.argv, {
                runtime: 'wasm-browser-worker',
            });
            const invocation = await buildBrowserCommandInvocation(
                parsed,
                async (requirement) => {
                    if (requirement.kind === 'read') {
                        const bytes = resources.get(requirement.uri);
                        if (bytes === undefined) throw new Error(`missing fixture resource ${requirement.uri}`);
                        return [{ uri: requirement.uri, bytes }];
                    }
                    return [...resources].map(([uri, bytes]) => ({ uri, bytes }));
                },
                {
                    requestId,
                    projectId: 'browser-all-operations',
                    projectRevision: 1,
                    resourceRevision: 1,
                    cwd: fixtureDirectory,
                },
            );
            ledgers.set(requestId, {
                project: invocation.request.project,
                resourceVersions: invocation.request.resourceVersions,
            });
            const progress = [];
            const handle = client.execute(invocation.request, {
                onProgress: (event) => progress.push(event),
            });
            const result = await handle;
            const presentation = projectBrowserCommandPresentation(invocation.presentation, result);
            summaries.push({
                name: fixtureCase.name,
                operation: result.operation,
                sourceKind:
                    invocation.request.operation.kind === 'transform'
                        ? invocation.request.operation.source.kind
                        : undefined,
                status: result.status,
                exitCode: result.exitCode,
                runtime: result.identity.runtime,
                commonVersion: result.identity.commonVersion,
                targetIdentity: result.identity.targetIdentity,
                abiIdentity: result.identity.abiIdentity,
                diagnostics: result.diagnostics.originalCount,
                artifacts: result.artifacts.originalCount,
                sourceMaps: result.sourceMaps.originalCount,
                hasResult: result.result != null,
                hasReport: result.report != null,
                presentationTargets: presentation.writes.map(({ target }) => target),
                progress: progress.map(({ sequence, stage, status }) => [sequence, stage, status]),
                normalizedResult: normalizeWasmCommandResult(fixtureCase, result),
            });
            await handle.dispose();
        }
        return {
            summaries,
            committedWrites: [...writes.values()].filter(({ committed }) => committed).length,
        };
    } finally {
        await client.close();
    }
}

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
            capability: client.capability,
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

async function runHostConfigurationFixture() {
    let callbacks = 0;
    const host = versionCommandHost(({ project }) => {
        callbacks++; return { project, resourceVersions: {} };
    });
    const hostConfiguration = { schemaPackageReplacementGrants: [{
        packageId: '', expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'vendor://new',
    }] };
    const options = { host, hostConfiguration };
    const starting = createBrowserCommandServiceClient(options);
    hostConfiguration.schemaPackageReplacementGrants[0].packageId = 'vendor';
    options.hostConfiguration = { schemaPackageReplacementGrants: [] };
    const original = await starting;
    const distinct = await createBrowserCommandServiceClient(options);
    try {
        const rejected = [];
        for (const requestId of ['browser-host-snapshot-1', 'browser-host-snapshot-2']) {
            try { await original.execute(versionCommandRequest(requestId)); rejected.push('accepted'); }
            catch (error) { rejected.push(error.code); }
        }
        const before = callbacks;
        const request = { ...versionCommandRequest('browser-host-distinct'),
            hostConfiguration: { schemaPackageReplacementGrants: null },
            host_configuration_json: 'null',
        };
        const result = await distinct.execute(request, { hostConfiguration: null });
        return { rejected, before, callbacks, status: result.status };
    } finally { await original.close(); await distinct.close(); }
}


async function runNativeCheckpointFixture() {
    let entered, resume;
    const requested = new Promise(resolve => { entered = resolve; });
    const client = await createBrowserCommandServiceClient({ host: versionCommandHost(({ project }) => new Promise(resolve => {
        resume = () => resolve({ project, resourceVersions: {} }); entered();
    })) });
    const other = await createBrowserCommandServiceClient({ host: versionCommandHost() });
    try {
        const handle = client.execute(versionCommandRequest('browser-native-drain'));
        await requested;
        let acknowledged = false;
        const capturing = client.checkpointNative().then(proof => { acknowledged = true; return proof; });
        let refusal;
        try { client.execute(versionCommandRequest('browser-native-refused')); } catch (error) { refusal = error.code; }
        await handle.cancel('checkpoint fixture');
        const waited = !acknowledged;
        resume();
        const result = await handle;
        const proof = await capturing;
        const current = client.isNativeCheckpointCurrent(proof);
        const copied = client.isNativeCheckpointCurrent({ ...proof });
        const foreign = other.isNativeCheckpointCurrent(proof);
        const independent = (await other.execute(versionCommandRequest('browser-native-independent'))).status;
        await client.close();
        return { waited, refusal, status: result.status, current, copied, foreign, independent,
            afterClose: client.isNativeCheckpointCurrent(proof), scope: proof.scope, replacementAllowed: proof.replacementAllowed };
    } finally { resume?.(); await client.close(); await other.close(); }
}

async function runNativePressureFixture() {
    const NativeWorker = globalThis.Worker;
    let physicalWorker;
    const workerSource = `
        const buffered = [];
        const buffer = event => { event.stopImmediatePropagation(); buffered.push(event); };
        self.addEventListener('message', buffer);
        const native = await import('${location.origin}/command-native/runtime.js');
        await native.default('${location.origin}/command-native/runtime.wasm');
        self.addEventListener('message', event => {
            if (event.data.type !== 'fixture-native-pressure') return;
            event.stopImmediatePropagation();
            try {
                const status = JSON.parse(native.runtimeMemoryStatus());
                native.configureRuntimeMemoryBudget(JSON.stringify({ ...status.config, softLimitBytes: 1 }));
                event.ports[0].postMessage({ ok: true });
            } catch (error) { event.ports[0].postMessage({ error: String(error) }); }
        });
        await import('${location.origin}/command-native/browser-worker.js');
        self.removeEventListener('message', buffer);
        for (const event of buffered) self.dispatchEvent(new MessageEvent('message', { data: event.data, ports: event.ports }));
    `;
    const url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    globalThis.Worker = class extends NativeWorker {
        constructor(_source, options) { super(url, options); physicalWorker = () => this; }
    };
    const writes = new Map();
    const bytes = new TextEncoder().encode('.card { color: red; }');
    const version = { revision: 1, sha256: await sha256(bytes) };
    const request = parseCommandRequest(version);
    const failures = [];
    let client;
    try {
        client = await createBrowserCommandServiceClient({ onWorkerFailure: failure => failures.push(failure), host: {
            currentRevision: ({ project }) => ({ project, resourceVersions: request.resourceVersions }),
            readResource: () => ({ version, bytes: [...bytes], identity: { contentType: 'text/css', schema: 'https://cem.dev/ns/data/css/1', defaultNamespace: null, namespaces: {}, baseUri: null } }),
            prepareWrite: request => { const token = request.uri; writes.set(token, request); return { token }; },
            commitWrite: token => ({ uri: writes.get(token).uri }), rollbackWrite: () => undefined,
        } });
        const handle = client.execute(request);
        const result = await handle;
        if (result.status !== 'succeeded') throw Error(JSON.stringify(result.diagnostics));
        const channel = new MessageChannel();
        await new Promise((resolve, reject) => {
            const timer = setTimeout(() => reject(Error('pressure fixture timeout')), 5000);
            channel.port1.onmessage = event => { clearTimeout(timer); if (event.data.ok) resolve(); else reject(Error(event.data.error)); };
            physicalWorker().postMessage({ type: 'fixture-native-pressure' }, [channel.port2]);
        }).finally(() => { channel.port1.close(); channel.port2.close(); });
        const rejection = await client.execute(versionCommandRequest('browser-pressure-refused')).result().then(() => null, error => error.code);
        let earlyRefusal;
        try { client.execute(versionCommandRequest('browser-pressure-early')); } catch (error) { earlyRefusal = error.code; }
        const retained = await client.checkpointNative().then(() => null, error => error.code);
        const read = await handle.readArtifact(result.artifacts.items[0], { maxBytes: 16 }).then(() => null, error => error.code);
        await handle.disposeArtifact(result.artifacts.items[0]);
        const partial = await client.checkpointNative().then(() => null, error => error.code);
        await handle.dispose();
        const proof = await client.checkpointNative();
        const current = client.isNativeCheckpointCurrent(proof);
        await handle.dispose();
        return { status: result.status, artifacts: result.artifacts.originalCount, rejection, earlyRefusal, retained, read, partial,
            current, afterCleanup: client.isNativeCheckpointCurrent(proof), failures, scope: proof.scope, replacementAllowed: proof.replacementAllowed };
    } finally { await client?.close(); globalThis.Worker = NativeWorker; URL.revokeObjectURL(url); }
}

async function runNativeInventoryFixture(coordinated = false) {
    const NativeWorker = globalThis.Worker;
    let physicalWorker;
    const workerSource = `
        const buffered = [];
        const buffer = event => { event.stopImmediatePropagation(); buffered.push(event); };
        self.addEventListener('message', buffer);
        const native = await import('${location.origin}/command-native/runtime.js');
        await native.default('${location.origin}/command-native/runtime.wasm');
        let document, freezePort;
        const post = self.postMessage.bind(self);
        self.postMessage = (...args) => {
            if (args[0]?.result?.frozen === true && freezePort) {
                let refusal; try { native.resetRetainedRenderCache(); } catch (error) { refusal = String(error); }
                const inventory = JSON.parse(native.commandNativeResourceInventory());
                freezePort.postMessage({ refusal, live: inventory.registries.find(entry => entry.name === 'native-entries').live, revision: inventory.revision,
                    frozenRevision: args[0].result.frozenRevision }); freezePort.close(); freezePort = undefined;
            }
            return post(...args);
        };
        self.addEventListener('message', async event => {
            if (event.data.type !== 'fixture-native-inventory') return;
            event.stopImmediatePropagation();
            try {
                if (event.data.retain === 'freeze-probe') { freezePort = event.ports[0]; return; }
                if (event.data.retain === 'reset') { native.resetRetainedRenderCache(); }
                else if (event.data.retain === 'probe') {
                    let document;
                    try { native.retainCemDocument(new TextEncoder().encode('<item/>'), 'application/xml', 'memory:denied'); }
                    catch (error) { document = String(error); }
                    let callbacks = 0; const forbidden = () => { callbacks++; throw Error('unexpected callback'); };
                    const command = JSON.parse(await native.executeCommandServiceV1('{}', '{}', forbidden, forbidden, forbidden, forbidden, forbidden));
                    event.ports[0].postMessage({ document, template: JSON.parse(native.compileTemplate('{p | denied}', '{}')).error.code,
                        command: command.error.code, callbacks }); return;
                }
                else if (event.data.retain) document = native.retainCemDocument(new TextEncoder().encode('<item/>'), 'application/xml', 'memory:raw-browser');
                else native.disposeCemDocument(document);
                event.ports[0].postMessage(JSON.parse(native.commandNativeResourceInventory()));
            } catch (error) { event.ports[0].postMessage({ error: String(error) }); }
        });
        await import('${location.origin}/command-native/browser-worker.js');
        self.removeEventListener('message', buffer);
        for (const event of buffered) self.dispatchEvent(new MessageEvent('message', { data: event.data, ports: event.ports }));
    `;
    const url = URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));
    globalThis.Worker = class extends NativeWorker {
        constructor(_source, options) { super(url, options); physicalWorker = () => this; }
    };
    const mutate = retain => {
        const channel = new MessageChannel();
        return new Promise((resolve, reject) => {
            const timer = setTimeout(() => reject(Error('inventory fixture timeout')), 5000);
            channel.port1.onmessage = event => { clearTimeout(timer); if (event.data.error) reject(Error(event.data.error)); else resolve(event.data); };
            physicalWorker().postMessage({ type: 'fixture-native-inventory', retain }, [channel.port2]);
        }).finally(() => { channel.port1.close(); channel.port2.close(); });
    };
    let client;
    try {
        client = await createBrowserCommandServiceClient({ host: {
            currentRevision: ({ project }) => ({ project, resourceVersions: {} }),
            readResource: () => { throw Error('unexpected read'); }, prepareWrite: () => { throw Error('unexpected write'); },
            commitWrite: () => { throw Error('unexpected commit'); }, rollbackWrite: () => undefined,
        } });
        const a = client.claimNativeEmbedding(), b = client.claimNativeEmbedding();
        const capture = owners => coordinated ? client.closeNativeAdmission(owners, 1000) : client.checkpointNative(1000, owners);
        const omitted = await capture([a]).then(() => null, error => error.code);
        const retained = await mutate(true);
        const blocked = await capture([a, b]).then(() => null, error => error.code);
        const denied = coordinated ? await mutate('probe') : undefined;
        await mutate(false);
        let proof = await capture([a, b]);
        if (coordinated === 'retirement') {
            const group = new CemNativeGenerationGroup([client.generationParticipant()]);
            const handoff = () => ({ current: () => true,
                retain: () => new TextEncoder().encode('sealed draft and terminal journal') });
            await mutate('reset');
            const stale = await client.retireNativeGeneration(proof, [a, b], handoff()).then(() => null, error => error.code);
            proof = await capture([a, b]);
            let checks = 0; const changed = handoff(); changed.current = () => ++checks < 3;
            const cancelled = await client.retireNativeGeneration(proof, [a, b], changed).then(() => null, error => error.code);
            await mutate('reset'); // original permit was dropped; cleanup works again
            proof = await capture([a, b]);
            const probe = mutate('freeze-probe');
            const retired = await client.retireNativeGeneration(proof, [a, b], handoff());
            const frozen = await probe;
            let closed; try { client.claimNativeEmbedding(); } catch (error) { closed = error.code; }
            const admission = await group.allocate([retired]);
            try {
                const item = admission.generations[0], fresh = item.runtime;
                if (item.runtimeId === item.previousRuntimeId || fresh.worker.runtimeInstanceId !== item.runtimeId) throw Error('fresh worker identity mismatch');
                const lease = fresh.claimNativeEmbedding(); fresh.releaseNativeEmbedding(lease);
            } finally { admission.revoke(); }
            let fenced = false; try { admission.generations[0].runtime.claimNativeEmbedding(); } catch (error) { fenced = String(error).includes('allocation.closed'); }
            if (!fenced) throw Error('revoked fresh worker admitted work');

            return { stale, cancelled, frozen: frozen.refusal, permit: frozen.live, unchanged: frozen.revision === frozen.frozenRevision,
                checkpoint: new TextDecoder().decode(retired.checkpoint), replacementAllowed: retired.replacementAllowed, closed };
        }
        let stale, hostCurrent;
        if (coordinated) {
            proof = await client.revalidateNativeCheckpoint(proof, [a, b], 1000);
            await mutate('reset');
            hostCurrent = client.isNativeCheckpointCurrent(proof);
            stale = await client.revalidateNativeCheckpoint(proof, [a, b], 1000).then(() => null, error => error.code);
            proof = await capture([a, b]);
        }
        const current = client.isNativeCheckpointCurrent(proof), copied = client.isNativeCheckpointCurrent({ ...proof });
        client.releaseNativeEmbedding(b);
        return { omitted, blocked, live: retained.registries.find(entry => entry.name === 'cem-documents').live,
            registries: proof.nativeInventory.registries.length, owners: proof.embeddingOwners,
            blockers: proof.nativeInventory.blockingResources, current, copied,
            afterRelease: client.isNativeCheckpointCurrent(proof), replacementAllowed: proof.replacementAllowed,
            ...(coordinated ? { denied, stale, hostCurrent, closed: proof.nativeClosure.closed } : {}) };
    } finally { await client?.close(); globalThis.Worker = NativeWorker; URL.revokeObjectURL(url); }
}
