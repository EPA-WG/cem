import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import test from 'node:test';

import {
    createNodeCommandService,
    parseCemMlCommand,
} from '../dist/node.js';
import {
    commandCases,
    fixtureFiles,
    portableOperationKinds,
} from './command-all-operations.fixture.mjs';

const executable = resolve(new URL('../dist/bin.js', import.meta.url).pathname);

test('Node command service executes every portable operation and both transform sources', async () => {
    const directory = await materializeFixture();
    const stdout = new FixtureWritable();
    const stderr = new FixtureWritable();
    const service = await createNodeCommandService({
        cwd: directory,
        stdout,
        stderr,
        deferStreamCommits: true,
    });
    try {
        const summaries = [];
        for (const fixtureCase of commandCases) {
            const parsed = parseCemMlCommand(fixtureCase.argv, { runtime: 'wasm-node' });
            const execution = await service.run(parsed, {
                requestId: `node-all-${fixtureCase.name}`,
                projectId: 'node-all-operations',
                projectRevision: 1,
                resourceRevision: 1,
                cwd: directory,
            });
            const result = await execution.handle;
            const presentation = await service.publish(execution.invocation, result);
            summaries.push({
                name: fixtureCase.name,
                operation: result.operation,
                sourceKind:
                    execution.invocation.request.operation.kind === 'transform'
                        ? execution.invocation.request.operation.source.kind
                        : undefined,
                status: result.status,
                exitCode: result.exitCode,
                runtime: result.identity.runtime,
                diagnostics: result.diagnostics.originalCount,
                artifacts: result.artifacts.originalCount,
                sourceMaps: result.sourceMaps.originalCount,
                hasResult: result.result != null,
                hasReport: result.report != null,
                presentationTargets: presentation.writes.map(({ target }) => target),
            });
            await execution.handle.dispose();
        }

        assert.deepEqual(new Set(summaries.map(({ operation }) => operation)), new Set(portableOperationKinds));
        assert.deepEqual(
            summaries.filter(({ operation }) => operation === 'transform').map(({ sourceKind }) => sourceKind),
            ['direct', 'graph'],
        );
        assert.ok(
            summaries.every(
                ({ status, exitCode, runtime, diagnostics, hasResult }) =>
                    status === 'succeeded' &&
                    exitCode === 0 &&
                    runtime === 'wasm-node' &&
                    diagnostics === 0 &&
                    hasResult,
            ),
        );
        assert.ok(summaries.some(({ hasReport }) => hasReport));
        assert.ok(summaries.some(({ sourceMaps }) => sourceMaps > 0));
        assert.ok(summaries.some(({ presentationTargets }) => presentationTargets.length > 0));
        assert.ok(stdout.byteLength > 0);
        assert.equal(stderr.byteLength, 0);
        assert.equal(
            JSON.parse(await readFile(resolve(directory, 'graph-output.json'), 'utf8')).sequence.items.length,
            2,
        );
        assert.equal(
            JSON.parse(await readFile(resolve(directory, 'validate-report.json'), 'utf8')).summary.inputCount,
            1,
        );
    } finally {
        await service.close();
        await rm(directory, { recursive: true, force: true });
    }
});

test('cem-ml executable completes the shared all-command matrix with stable exits and outputs', async () => {
    const directory = await materializeFixture();
    try {
        const outcomes = commandCases.map((fixtureCase) => ({
            fixtureCase,
            result: spawnSync(process.execPath, [executable, ...fixtureCase.argv], {
                cwd: directory,
                encoding: 'utf8',
            }),
        }));
        for (const { fixtureCase, result } of outcomes) {
            assert.equal(
                result.status,
                0,
                `${fixtureCase.name} failed: ${result.stderr || result.stdout || result.error}`,
            );
            assert.equal(result.signal, null, fixtureCase.name);
        }
        assert.ok(outcomes.every(({ result }) => result.stderr === ''));
        assert.ok(
            outcomes.every(
                ({ fixtureCase, result }) =>
                    fixtureCase.name === 'transform-graph' || result.stdout.length > 0,
            ),
        );
        assert.equal(
            JSON.parse(await readFile(resolve(directory, 'graph-output.json'), 'utf8')).sequence.items.length,
            2,
        );
        const report = JSON.parse(await readFile(resolve(directory, 'validate-report.json'), 'utf8'));
        assert.equal(report.summary.inputCount, 1);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

async function materializeFixture() {
    const directory = await mkdtemp(resolve(tmpdir(), 'cem-ml-all-operations-'));
    await Promise.all(
        Object.entries(fixtureFiles).map(([name, source]) =>
            writeFile(resolve(directory, name), source),
        ),
    );
    return directory;
}

class FixtureWritable {
    chunks = [];

    write(bytes) {
        this.chunks.push(new Uint8Array(bytes));
        return true;
    }

    get byteLength() {
        return this.chunks.reduce((total, chunk) => total + chunk.byteLength, 0);
    }
}
