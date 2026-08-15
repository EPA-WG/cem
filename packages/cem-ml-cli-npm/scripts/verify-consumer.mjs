import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { commandCases, fixtureFiles } from '../tests/command-all-operations.fixture.mjs';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const cliArchive = resolve(workspaceRoot, 'dist/packages/cem-ml-cli-npm/package.tgz');
const runtimeArchive = resolve(workspaceRoot, 'dist/packages/cem-ml-npm/package.tgz');
const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
const consumerRoot = mkdtempSync(join(tmpdir(), 'cem-ml-cli-consumer-'));

try {
    writeFileSync(
        resolve(consumerRoot, 'package.json'),
        `${JSON.stringify({ name: 'cem-ml-cli-clean-consumer', private: true, type: 'module' }, null, 2)}\n`,
    );
    run(process.platform === 'win32' ? 'npm.cmd' : 'npm', [
        'install',
        runtimeArchive,
        cliArchive,
        '--ignore-scripts',
        '--no-audit',
        '--no-fund',
    ]);

    const installedCliRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-ml-cli');
    const installedRuntimeRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-ml');
    const installedCli = readJson(resolve(installedCliRoot, 'package.json'));
    const installedRuntime = readJson(resolve(installedRuntimeRoot, 'package.json'));
    assert.equal(installedCli.version, packageMetadata.version);
    assert.deepEqual(installedCli.dependencies, { '@epa-wg/cem-ml': packageMetadata.version });
    assert.equal(installedRuntime.version, packageMetadata.version);
    assert.deepEqual(installedCli.bin, { 'cem-ml': './dist/bin.js' });

    const runtimeInstalls = findRuntimeInstalls(resolve(consumerRoot, 'node_modules'));
    assert.deepEqual(runtimeInstalls, [installedRuntimeRoot]);

    for (const [name, source] of Object.entries(fixtureFiles)) {
        writeFileSync(resolve(consumerRoot, name), source);
    }

    writeFileSync(
        resolve(consumerRoot, 'probe.mjs'),
        `import {
  commandSchema,
  createNodeWorkerPool,
  parseCemMlCommand,
  serializeCemMlCommand,
} from '@epa-wg/cem-ml-cli/node';

const parsed = parseCemMlCommand([
  'query',
  'data.xml',
  '--query',
  '//item',
  '--query-content-type',
  'application/vnd.cem.xpath',
], { runtime: 'wasm-node' });
const roundTrip = parseCemMlCommand(serializeCemMlCommand(parsed), { runtime: 'wasm-node' });

const pool = await createNodeWorkerPool({ workerCount: 2, maxWorkers: 4 });
try {
  console.log(JSON.stringify({
    commandSchemaVersion: commandSchema.schemaVersion,
    commandCommonVersion: commandSchema.commonVersion,
    commandPath: parsed.commandPath,
    commandRoundTrip: JSON.stringify(roundTrip) === JSON.stringify(parsed),
    mode: pool.mode,
    size: pool.size,
    commonVersion: pool.capability.commonVersion,
    topology: pool.capability.executorTopology,
    effectiveMaxWorkers: pool.capability.effectiveMaxWorkers,
    instances: pool.workers.map(({ slot, generation, runtimeInstanceId }) => ({
      slot,
      generation,
      runtimeInstanceId,
    })),
  }));
} finally {
  await pool.close();
}
`,
    );
    const probe = JSON.parse(capture(process.execPath, ['probe.mjs']));
    assert.equal(probe.commandSchemaVersion, 1);
    assert.equal(probe.commandCommonVersion, packageMetadata.version);
    assert.deepEqual(probe.commandPath, ['query']);
    assert.equal(probe.commandRoundTrip, true);
    assert.equal(probe.mode, 'pool');
    assert.equal(probe.size, 2);
    assert.equal(probe.commonVersion, packageMetadata.version);
    assert.equal(probe.topology, 'node-worker-pool');
    assert.equal(probe.effectiveMaxWorkers, 2);
    assert.deepEqual(
        probe.instances.map(({ slot, generation }) => ({ slot, generation })),
        [
            { slot: 1, generation: 1 },
            { slot: 2, generation: 1 },
        ],
    );
    assert.equal(new Set(probe.instances.map(({ runtimeInstanceId }) => runtimeInstanceId)).size, 2);

    const executable = resolve(
        consumerRoot,
        'node_modules/.bin',
        process.platform === 'win32' ? 'cem-ml.cmd' : 'cem-ml',
    );
    for (const fixtureCase of commandCases) {
        const result = spawnSync(executable, fixtureCase.argv, {
            cwd: consumerRoot,
            encoding: 'utf8',
        });
        assert.equal(
            result.status,
            0,
            `installed executable ${fixtureCase.name} failed:\n${result.stderr || result.stdout || result.error}`,
        );
        assert.equal(result.signal, null, fixtureCase.name);
        assert.equal(result.stderr, '', fixtureCase.name);
        if (fixtureCase.name !== 'transform-graph') assert.ok(result.stdout.length > 0, fixtureCase.name);
    }
    assert.equal(
        JSON.parse(readFileSync(resolve(consumerRoot, 'graph-output.json'), 'utf8')).sequence.items.length,
        2,
    );
    assert.equal(
        JSON.parse(readFileSync(resolve(consumerRoot, 'validate-report.json'), 'utf8')).summary.inputCount,
        1,
    );

    console.log(
        `Clean consumer verified ${installedCli.name}@${installedCli.version}: all portable executable operations, generated command round trip, one runtime copy, and two Node workers.`,
    );
} finally {
    assert.ok(consumerRoot.startsWith(`${tmpdir()}${sep}cem-ml-cli-consumer-`));
    rmSync(consumerRoot, { recursive: true, force: true });
}

function findRuntimeInstalls(root) {
    const matches = [];
    const visit = (directory) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            if (!entry.isDirectory() || entry.isSymbolicLink()) continue;
            const absolute = resolve(directory, entry.name);
            if (absolute.endsWith(`${sep}node_modules${sep}@epa-wg${sep}cem-ml`)) {
                matches.push(absolute);
            }
            visit(absolute);
        }
    };
    if (statSync(root).isDirectory()) visit(root);
    return matches.sort();
}

function run(command, args) {
    const result = spawnSync(command, args, { cwd: consumerRoot, stdio: 'inherit' });
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
    }
}

function capture(command, args) {
    const result = spawnSync(command, args, { cwd: consumerRoot, encoding: 'utf8' });
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`);
    }
    return result.stdout.trim();
}

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
