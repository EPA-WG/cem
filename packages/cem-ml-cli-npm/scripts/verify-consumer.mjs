import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

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
    assert.equal(installedCli.bin, undefined);

    const runtimeInstalls = findRuntimeInstalls(resolve(consumerRoot, 'node_modules'));
    assert.deepEqual(runtimeInstalls, [installedRuntimeRoot]);

    writeFileSync(
        resolve(consumerRoot, 'probe.mjs'),
        `import { createNodeWorkerPool } from '@epa-wg/cem-ml-cli/node';

const pool = await createNodeWorkerPool({ workerCount: 2, maxWorkers: 4 });
try {
  console.log(JSON.stringify({
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

    console.log(
        `Clean consumer verified ${installedCli.name}@${installedCli.version}: one resolved runtime copy and two isolated Node worker runtimes.`,
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
