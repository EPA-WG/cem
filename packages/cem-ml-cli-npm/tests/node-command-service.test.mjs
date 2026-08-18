import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import test from 'node:test';

import {
    buildNodeCommandInvocation,
    createNodeCommandHost,
    createNodeCommandService,
    createNodeCommandServiceClient,
    parseCemMlCommand,
} from '../dist/node.js';

const encoder = new TextEncoder();
const decoder = new TextDecoder();

test('Node host resolves file, HTTPS, and application stream resources', async () => {
    const directory = await mkdtemp(join(tmpdir(), 'cem-ml-node-host-'));
    try {
        const file = join(directory, 'input with space.cem');
        await writeFile(file, '{file}\n');
        const host = createNodeCommandHost({
            cwd: directory,
            fetch: async (uri) => {
                assert.equal(uri, 'https://example.test/input.cem');
                return new Response('{https}\n', {
                    status: 200,
                    headers: { 'content-length': '8' },
                });
            },
            readStreams: {
                'stream:input': (async function* () {
                    yield '{stream';
                    yield '}\n';
                })(),
            },
        });
        assert.equal(decoder.decode(await host.read(pathToFileURL(file).href)), '{file}\n');
        assert.equal(decoder.decode(await host.read('https://example.test/input.cem')), '{https}\n');
        assert.equal(decoder.decode(await host.read('stream:input')), '{stream}\n');
        await assert.rejects(host.read('http://example.test/input.cem'), /insecure HTTP/);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test('Node host file and stream writes remain prepared until commit and support rollback', async () => {
    const directory = await mkdtemp(join(tmpdir(), 'cem-ml-node-write-'));
    const stream = new FixtureWritable();
    try {
        const host = createNodeCommandHost({ cwd: directory, writeStreams: { 'stream:output': stream } });
        const destination = join(directory, 'output.cem');
        await writeFile(destination, '{original}');
        const prepared = await host.prepareWrite(writeRequest(pathToFileURL(destination).href), encoder.encode('{file}'));
        assert.equal(await readFile(destination, 'utf8'), '{original}');
        assert.equal((await host.commitWrite(prepared.token)).uri, pathToFileURL(destination).href);
        assert.equal(await readFile(destination, 'utf8'), '{file}');
        await host.rollbackWrite(prepared.token);
        assert.equal(await readFile(destination, 'utf8'), '{original}');

        const rolledBackDestination = join(directory, 'rolled-back.cem');
        const rolledBack = await host.prepareWrite(
            writeRequest(pathToFileURL(rolledBackDestination).href),
            encoder.encode('{discarded}'),
        );
        await host.rollbackWrite(rolledBack.token);
        await assert.rejects(readFile(rolledBackDestination), /ENOENT/);

        const streamToken = await host.prepareWrite(writeRequest('stream:output'), encoder.encode('{stream}'));
        assert.equal(stream.text(), '');
        await host.commitWrite(streamToken.token);
        assert.equal(stream.text(), '{stream}');

        const deferred = createNodeCommandHost({
            writeStreams: { 'stream:output': stream },
            deferStreamCommits: true,
        });
        const deferredToken = await deferred.prepareWrite(
            writeRequest('stream:output'),
            encoder.encode('{deferred}'),
        );
        await deferred.commitWrite(deferredToken.token);
        await deferred.rollbackWrite(deferredToken.token);
        assert.equal(deferred.takeCommittedStream('stream:output').byteLength, 0);
    } finally {
        await rm(directory, { recursive: true, force: true });
    }
});

test('Rust lowering and Node worker execution preserve file URI and terminal presentation ownership', async () => {
    const directory = await mkdtemp(join(tmpdir(), 'cem-ml-node-command-'));
    const stdout = new FixtureWritable();
    const stderr = new FixtureWritable();
    let service;
    try {
        const input = join(directory, 'input with space.cem');
        await writeFile(input, '{div}\n');
        const host = createNodeCommandHost({
            cwd: directory,
            stdout,
            stderr,
            deferStreamCommits: true,
        });
        const parsed = parseCemMlCommand(['parse', input], { runtime: 'wasm-node' });
        const invocation = await buildNodeCommandInvocation(parsed, host, {
            requestId: 'node-command-lowering',
        });
        const resourceUris = Object.keys(invocation.request.resourceVersions);
        assert.deepEqual(resourceUris, [pathToFileURL(input).href]);
        assert.equal(invocation.request.operation.kind, 'parse');
        assert.equal(invocation.presentation.stdoutArtifactUri, 'cem-stdio://stdout');

        service = await createNodeCommandService({ host });
        const execution = await service.run(parsed, { requestId: 'node-command-execute' });
        const progress = [];
        const unsubscribe = execution.handle.subscribe((event) => progress.push(event.sequence));
        const result = await execution.handle;
        unsubscribe();
        assert.equal(result.status, 'succeeded');
        assert.equal(result.exitCode, 0);
        assert.ok(progress.length > 0);

        const presentation = await service.publish(execution.invocation, result);
        assert.ok(presentation.writes.some((write) => write.target === 'stdout'));
        assert.ok(stdout.text().includes('div'));
        assert.equal(stderr.text(), '');
        await execution.handle.dispose();
    } finally {
        await service?.close();
        await rm(directory, { recursive: true, force: true });
    }
});

test('Node command AbortSignal produces the stable cooperative cancellation terminal', async () => {
    let resolveRevision;
    let revisionRequested;
    const requested = new Promise((resolve) => {
        revisionRequested = resolve;
    });
    const host = {
        currentRevision: ({ project }) =>
            new Promise((resolve) => {
                resolveRevision = () => resolve({ project, resourceVersions: {} });
                revisionRequested();
            }),
        readResource: unexpected('readResource'),
        prepareWrite: unexpected('prepareWrite'),
        commitWrite: unexpected('commitWrite'),
        rollbackWrite: unexpected('rollbackWrite'),
    };
    const client = await createNodeCommandServiceClient({ host });
    try {
        const abort = new AbortController();
        const handle = client.execute(versionRequest('node-command-cancel'), { signal: abort.signal });
        await requested;
        abort.abort('Node fixture cancellation');
        resolveRevision();
        const result = await handle;
        assert.equal(result.status, 'cancelled');
        assert.equal(result.exitCode, 130);
    } finally {
        await client.close();
    }
});

test('cem-ml executable preserves success, usage, and host-I/O exit policy', () => {
    const version = runExecutable(['--version']);
    assert.equal(version.status, 0);
    assert.match(
        version.stdout,
        /^cem-ml (?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?\n/,
    );
    assert.match(version.stdout, /Copyright/);

    const usage = runExecutable(['--definitely-invalid']);
    assert.equal(usage.status, 2);
    assert.match(usage.stderr, /unknown CEM-ML option/);

    const io = runExecutable(['parse', '/definitely/missing/cem-ml-input.cem']);
    assert.equal(io.status, 6);
    assert.match(io.stderr, /ENOENT/);
});

function writeRequest(uri) {
    return {
        requestId: 'node-host-write',
        project: { projectId: 'node-host', revision: 1 },
        label: 'fixture output',
        uri,
        kind: 'output',
        purpose: 'output',
        contentType: 'application/cem+xml',
        byteLength: 0,
        sha256: '0'.repeat(64),
        sourceMapId: null,
        resolverPolicyStamp: 'node-host-fixture',
    };
}

function versionRequest(requestId) {
    return {
        protocolVersion: 1,
        requestId,
        project: { projectId: 'node-fixture', revision: 1 },
        resourceVersions: {},
        operation: { kind: 'version-capabilities' },
        runPlan: null,
        resources: {},
        policyStamp: {
            resolver: 'node-fixture-resolver',
            safety: 'node-fixture-safety',
            budget: 'node-fixture-budget',
        },
    };
}

function unexpected(name) {
    return async () => {
        throw new Error(`${name} must not run`);
    };
}

function runExecutable(args) {
    return spawnSync(process.execPath, ['dist/bin.js', ...args], {
        cwd: new URL('..', import.meta.url),
        encoding: 'utf8',
    });
}

class FixtureWritable {
    chunks = [];

    write(chunk) {
        this.chunks.push(new Uint8Array(chunk));
        return true;
    }

    once(_event, listener) {
        listener();
        return this;
    }

    text() {
        const length = this.chunks.reduce((total, chunk) => total + chunk.byteLength, 0);
        const bytes = new Uint8Array(length);
        let offset = 0;
        for (const chunk of this.chunks) {
            bytes.set(chunk, offset);
            offset += chunk.byteLength;
        }
        return decoder.decode(bytes);
    }
}
