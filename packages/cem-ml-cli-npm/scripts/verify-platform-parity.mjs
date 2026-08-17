import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
    createNodeCommandService,
    createNodeCommandServiceClient,
    parseCemMlCommand,
} from '../dist/node.js';
import {
    commandCases,
    fixtureFiles,
    normalizeNativeCommandResult,
    normalizeWasmCommandResult,
    portableOperationKinds,
} from '../tests/command-all-operations.fixture.mjs';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const browserEvidence = JSON.parse(
    await readFile(resolve(workspaceRoot, 'dist/reports/cem-ml-platform-parity.browser.json'), 'utf8'),
);
const nativeIdentity = currentNativeIdentity();
const nativeEvidence = emitNativeEvidence(nativeIdentity);
const fixtureRoot = await mkdtemp(resolve(tmpdir(), 'cem-ml-platform-parity-'));

try {
    await Promise.all(
        Object.entries(fixtureFiles).map(([name, source]) =>
            writeFile(resolve(fixtureRoot, name), source),
        ),
    );
    const nativeOperations = await runNativeOperations(fixtureRoot, nativeEvidence, nativeIdentity);
    const node = await runNodeOperations(fixtureRoot);
    const browserOperations = browserEvidence.operations;

    assert.equal(browserEvidence.schemaVersion, 1);
    assert.equal(browserEvidence.host, 'wasm-browser-worker');
    assert.equal(nativeEvidence.schemaVersion, 1);
    assert.equal(nativeEvidence.host, 'native');
    assertOperationMatrix(nativeOperations, 'native');
    assertOperationMatrix(node.operations, 'wasm-node');
    assertOperationMatrix(browserOperations, 'wasm-browser-worker');
    assert.deepEqual(
        node.operations.map(({ normalizedResult }) => normalizedResult),
        browserOperations.map(({ normalizedResult }) => normalizedResult),
        'Node and browser WASM normalized results drifted',
    );
    assert.deepEqual(
        nativeOperations.map(({ normalizedResult }) => normalizedResult),
        node.operations.map(({ normalizedResult }) => normalizedResult),
        'native and WASM normalized results drifted',
    );

    const capabilities = {
        native: nativeEvidence.capability,
        node: node.capability,
        browser: browserEvidence.capability,
    };
    assertCapabilities(capabilities, nativeIdentity);
    assertResultIdentities(node.operations, node.capability);
    assertResultIdentities(browserOperations, browserEvidence.capability);

    const progress = {
        native: normalizeProgress(nativeEvidence.successProgress),
        node: node.operations[0].progress,
        browser: normalizeProgress(browserEvidence.successProgress),
    };
    const expectedSuccessProgress = [
        [1, 'accepted', null],
        [2, 'prepared', null],
        [3, 'executing', null],
        [4, 'terminal', 'succeeded'],
    ];
    for (const [host, events] of Object.entries(progress)) {
        assert.deepEqual(events, expectedSuccessProgress, `${host} success progress drifted`);
    }
    for (const operation of [...node.operations, ...browserOperations]) {
        assert.deepEqual(operation.progress, expectedSuccessProgress, `${operation.runtime}/${operation.name} progress`);
    }

    const cancellation = {
        native: normalizeCancellation(nativeEvidence.cancellation),
        node: normalizeCancellation(node.cancellation),
        browser: normalizeCancellation(browserEvidence.cancellation),
    };
    const expectedCancellation = {
        acknowledgement: 'accepted',
        status: 'cancelled',
        exitCode: 130,
        progress: [
            [1, 'accepted', null],
            [2, 'terminal', 'cancelled'],
        ],
    };
    for (const [host, result] of Object.entries(cancellation)) {
        assert.deepEqual(result, expectedCancellation, `${host} cancellation drifted`);
    }

    const commonVersion = nativeEvidence.capability.commonVersion;
    assert.equal(node.capability.commonVersion, commonVersion);
    assert.equal(browserEvidence.capability.commonVersion, commonVersion);
    const report = {
        schemaVersion: 1,
        commonVersion,
        hosts: {
            native: hostIdentity(nativeEvidence.capability, nativeIdentity.deploymentIdentity),
            node: hostIdentity(node.capability, 'wasm-node'),
            browser: hostIdentity(browserEvidence.capability, 'wasm-browser-worker'),
        },
        capabilityOperations: Object.fromEntries(
            Object.entries(capabilities).map(([host, capability]) => [
                host,
                Object.fromEntries(
                    capability.operations.map(({ operation, availability }) => [operation, availability]),
                ),
            ]),
        ),
        operations: nativeOperations.map(({ name, operation, sourceKind, normalizedResult }) => ({
            name,
            operation,
            ...(sourceKind === undefined ? {} : { sourceKind }),
            normalizedResult,
        })),
        progress,
        cancellation,
    };
    const reportPath = resolve(workspaceRoot, 'dist/reports/cem-ml-platform-parity.json');
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    console.log(
        `Verified native/${nativeIdentity.target}, Node/WASM, and browser-worker parity for ${commandCases.length} command cases.`,
    );
} finally {
    await rm(fixtureRoot, { recursive: true, force: true });
}

async function runNativeOperations(directory, evidence, identity) {
    const binary = resolve(
        workspaceRoot,
        'dist/target/cem_ml_cli/debug',
        process.platform === 'win32' ? 'cem-ml.exe' : 'cem-ml',
    );
    const successProgress = normalizeProgress(evidence.successProgress);
    const operations = [];
    for (const fixtureCase of commandCases) {
        const result = spawnSync(binary, fixtureCase.argv, {
            cwd: directory,
            encoding: 'utf8',
            maxBuffer: 16 * 1024 * 1024,
        });
        assert.equal(
            result.status,
            0,
            `${fixtureCase.name} native failure: ${result.stderr || result.stdout || result.error}`,
        );
        assert.equal(result.signal, null, `${fixtureCase.name} native signal`);
        assert.equal(result.stderr, '', `${fixtureCase.name} native stderr`);
        const files = {};
        if (fixtureCase.name === 'transform-graph') {
            files['graph-output.json'] = JSON.parse(
                await readFile(resolve(directory, 'graph-output.json'), 'utf8'),
            );
        }
        const normalizedResult = normalizeNativeCommandResult(fixtureCase, result.stdout, files);
        operations.push({
            name: fixtureCase.name,
            operation: fixtureCase.operation,
            sourceKind: fixtureCase.sourceKind,
            status: 'succeeded',
            exitCode: 0,
            runtime: 'native',
            commonVersion: evidence.capability.commonVersion,
            targetIdentity: identity.target,
            abiIdentity: evidence.capability.abiIdentity,
            diagnostics: normalizedResult.diagnosticCount ?? normalizedResult.report?.diagnosticCount ?? 0,
            hasResult: true,
            hasReport: ['validate', 'check', 'trace'].includes(fixtureCase.name),
            sourceMaps: normalizedResult.sourceMaps.count,
            progress: successProgress,
            normalizedResult,
        });
    }
    return operations;
}

async function runNodeOperations(directory) {
    const stdout = fixtureWritable();
    const stderr = fixtureWritable();
    const service = await createNodeCommandService({
        cwd: directory,
        stdout,
        stderr,
        deferStreamCommits: true,
    });
    try {
        const operations = [];
        for (const fixtureCase of commandCases) {
            const progress = [];
            const parsed = parseCemMlCommand(fixtureCase.argv, { runtime: 'wasm-node' });
            const execution = await service.run(
                parsed,
                {
                    requestId: `node-platform-${fixtureCase.name}`,
                    projectId: 'node-platform-parity',
                    projectRevision: 1,
                    resourceRevision: 1,
                    cwd: directory,
                },
                { onProgress: (event) => progress.push(event) },
            );
            const result = await execution.handle;
            const presentation = await service.publish(execution.invocation, result);
            operations.push({
                name: fixtureCase.name,
                operation: result.operation,
                sourceKind:
                    execution.invocation.request.operation.kind === 'transform'
                        ? execution.invocation.request.operation.source.kind
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
                progress: normalizeProgress(progress),
                normalizedResult: normalizeWasmCommandResult(fixtureCase, result),
            });
            await execution.handle.dispose();
        }
        return {
            operations,
            capability: service.client.capability,
            cancellation: await runNodeCancellation(),
        };
    } finally {
        await service.close();
    }
}

async function runNodeCancellation() {
    let resolveRevision;
    let revisionRequested;
    const requested = new Promise((resolveRequested) => {
        revisionRequested = resolveRequested;
    });
    const unexpected = (name) => async () => {
        throw new Error(`${name} must not run for version-capabilities`);
    };
    const client = await createNodeCommandServiceClient({
        host: {
            currentRevision: ({ project }) =>
                new Promise((resolveCurrent) => {
                    resolveRevision = () => resolveCurrent({ project, resourceVersions: {} });
                    revisionRequested();
                }),
            readResource: unexpected('readResource'),
            prepareWrite: unexpected('prepareWrite'),
            commitWrite: unexpected('commitWrite'),
            rollbackWrite: unexpected('rollbackWrite'),
        },
    });
    try {
        const progress = [];
        const handle = client.execute(versionRequest('node-platform-parity-cancel'), {
            onProgress: (event) => progress.push(event),
        });
        await requested;
        const acknowledgement = await handle.cancel('Node platform parity cancellation');
        resolveRevision();
        const result = await handle;
        return {
            acknowledgement,
            status: result.status,
            exitCode: result.exitCode,
            progress: normalizeProgress(progress),
        };
    } finally {
        await client.close();
    }
}

function assertOperationMatrix(operations, runtime) {
    assert.deepEqual(
        new Set(operations.map(({ operation }) => operation)),
        new Set(portableOperationKinds),
        `${runtime} operation matrix`,
    );
    assert.deepEqual(
        operations.filter(({ operation }) => operation === 'transform').map(({ sourceKind }) => sourceKind),
        ['direct', 'graph'],
        `${runtime} transform sources`,
    );
    assert.ok(
        operations.every(
            (operation) =>
                operation.status === 'succeeded' &&
                operation.exitCode === 0 &&
                operation.runtime === runtime &&
                operation.diagnostics === 0 &&
                operation.hasResult,
        ),
        `${runtime} terminal/result matrix`,
    );
    for (const name of ['validate', 'check', 'trace']) {
        assert.equal(operations.find((operation) => operation.name === name)?.hasReport, true, `${runtime}/${name} report`);
    }
    for (const name of ['parse', 'convert', 'query', 'transform-direct', 'transform-graph']) {
        assert.ok(
            operations.find((operation) => operation.name === name)?.normalizedResult.sourceMaps.count > 0,
            `${runtime}/${name} source maps`,
        );
    }
}

function assertCapabilities(capabilities, nativeIdentity) {
    const required = new Set(portableOperationKinds);
    for (const [host, capability] of Object.entries(capabilities)) {
        const operations = new Map(
            capability.operations.map(({ operation, availability }) => [operation, availability]),
        );
        for (const operation of required) {
            assert.equal(operations.get(operation), 'available', `${host}/${operation} capability`);
        }
        assert.equal(operations.get('schema-mutation'), 'unavailable', `${host}/schema-mutation gap`);
        assert.equal(operations.get('plugin-mutation'), 'unavailable', `${host}/plugin-mutation gap`);
        assert.equal(
            capability.controls.find(({ control }) => control === 'root-cancellation')?.availability,
            'available',
            `${host}/root-cancellation capability`,
        );
    }
    assert.equal(capabilities.native.runtime, 'native');
    assert.equal(capabilities.native.targetIdentity, nativeIdentity.target);
    assert.equal(operationAvailability(capabilities.native, 'bench'), 'available');
    assert.equal(operationAvailability(capabilities.native, 'fixture'), 'development-only');
    assert.equal(capabilities.node.runtime, 'wasm-node');
    assert.equal(capabilities.node.targetIdentity, 'wasm32-unknown-unknown:nodejs');
    assert.equal(operationAvailability(capabilities.node, 'bench'), 'unavailable');
    assert.equal(operationAvailability(capabilities.node, 'fixture'), 'development-only');
    assert.equal(capabilities.browser.runtime, 'wasm-browser-worker');
    assert.equal(capabilities.browser.targetIdentity, 'wasm32-unknown-unknown:web');
    assert.equal(operationAvailability(capabilities.browser, 'bench'), 'unavailable');
    assert.equal(operationAvailability(capabilities.browser, 'fixture'), 'unavailable');
}

function assertResultIdentities(operations, capability) {
    for (const operation of operations) {
        assert.equal(operation.runtime, capability.runtime, `${operation.name} runtime identity`);
        assert.equal(operation.commonVersion, capability.commonVersion, `${operation.name} common version`);
        assert.equal(operation.targetIdentity, capability.targetIdentity, `${operation.name} target identity`);
        assert.equal(operation.abiIdentity, capability.abiIdentity, `${operation.name} ABI identity`);
    }
}

function emitNativeEvidence(identity) {
    const result = spawnSync(
        'cargo',
        [
            'run',
            '--locked',
            '--package',
            'cem-ml-cli',
            '--example',
            'native-platform-parity-emit',
            '--target-dir',
            'dist/target/cem_ml_cli',
            '--',
            identity.target,
            `cem-ml-native-cli-v1:${identity.target}`,
        ],
        { cwd: workspaceRoot, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 },
    );
    assert.equal(
        result.status,
        0,
        `native parity evidence failed: ${result.stderr || result.stdout || result.error}`,
    );
    return JSON.parse(result.stdout);
}

function currentNativeIdentity() {
    const key = `${process.platform}/${process.arch}`;
    const identities = {
        'linux/x64': {
            target: 'x86_64-unknown-linux-gnu',
            deploymentIdentity: 'native-linux-amd64',
        },
        'darwin/arm64': {
            target: 'aarch64-apple-darwin',
            deploymentIdentity: 'native-macos-arm64',
        },
        'win32/x64': {
            target: 'x86_64-pc-windows-msvc',
            deploymentIdentity: 'native-windows-amd64',
        },
    };
    const identity = identities[key];
    if (identity === undefined) throw new Error(`native platform parity is unavailable on ${key}`);
    return identity;
}

function normalizeProgress(events) {
    return events.map((event) => {
        if (Array.isArray(event)) return [event[0], event[1], event[2] ?? null];
        return [event.sequence, event.stage, event.status ?? null];
    });
}

function normalizeCancellation(result) {
    return {
        acknowledgement: result.acknowledgement.disposition,
        status: result.status,
        exitCode: result.exitCode,
        progress: Array.isArray(result.progress[0]) ? result.progress : normalizeProgress(result.progress),
    };
}

function operationAvailability(capability, selected) {
    return capability.operations.find(({ operation }) => operation === selected)?.availability;
}

function hostIdentity(capability, deploymentIdentity) {
    return {
        deploymentIdentity,
        runtime: capability.runtime,
        targetIdentity: capability.targetIdentity,
        abiIdentity: capability.abiIdentity,
    };
}

function versionRequest(requestId) {
    return {
        protocolVersion: 1,
        requestId,
        project: { projectId: 'node-platform-parity', revision: 1 },
        resourceVersions: {},
        operation: { kind: 'version-capabilities' },
        runPlan: null,
        resources: {},
        policyStamp: {
            resolver: 'node-platform-parity-resolver',
            safety: 'node-platform-parity-safety',
            budget: 'node-platform-parity-budget',
        },
    };
}

function fixtureWritable() {
    return {
        chunks: [],
        write(bytes) {
            this.chunks.push(new Uint8Array(bytes));
            return true;
        },
        once(_event, listener) {
            listener();
            return this;
        },
    };
}
