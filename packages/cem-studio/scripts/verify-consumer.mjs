import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/consumer.json');
const studioArchive = resolve(workspaceRoot, 'dist/packages/cem-studio/package.tgz');
const cliArchive = resolve(workspaceRoot, 'dist/packages/cem-ml-cli-npm/package.tgz');
const runtimeArchive = resolve(workspaceRoot, 'dist/packages/cem-ml-npm/package.tgz');
const studioMetadata = readJson(resolve(projectRoot, 'package.json'));
const consumerRoot = mkdtempSync(join(tmpdir(), 'cem-studio-consumer-'));
const packRoot = resolve(consumerRoot, 'packs');

try {
    mkdirSync(packRoot, { recursive: true });
    const dependencyArchives = [
        pack(resolve(workspaceRoot, 'packages/cem-elements')),
        pack(resolve(workspaceRoot, 'packages/cem-components')),
        pack(resolve(workspaceRoot, 'packages/cem-theme')),
    ];
    writeFileSync(
        resolve(consumerRoot, 'package.json'),
        `${JSON.stringify({ name: 'cem-studio-clean-consumer', private: true, type: 'module' }, null, 2)}\n`,
    );
    run(process.platform === 'win32' ? 'npm.cmd' : 'npm', [
        'install',
        runtimeArchive,
        cliArchive,
        ...dependencyArchives,
        studioArchive,
        '--ignore-scripts',
        '--no-audit',
        '--no-fund',
    ]);

    const installedStudioRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-studio');
    const installedCliRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-ml-cli');
    const installedRuntimeRoot = resolve(consumerRoot, 'node_modules/@epa-wg/cem-ml');
    const installedStudio = readJson(resolve(installedStudioRoot, 'package.json'));
    const installedCli = readJson(resolve(installedCliRoot, 'package.json'));
    const installedRuntime = readJson(resolve(installedRuntimeRoot, 'package.json'));

    assert.equal(installedStudio.version, studioMetadata.version);
    assert.deepEqual(installedStudio.dependencies, studioMetadata.dependencies);
    assert.equal(installedCli.version, studioMetadata.version);
    assert.equal(installedCli.dependencies['@epa-wg/cem-ml'], studioMetadata.version);
    assert.equal(installedRuntime.version, studioMetadata.version);
    assert.deepEqual(findRuntimeInstalls(resolve(consumerRoot, 'node_modules')), [installedRuntimeRoot]);

    for (const path of [
        'dist/static/index.html',
        'dist/static/manifest.webmanifest',
        'dist/static/service-worker.js',
        'dist/static/cache-inventory.json',
        'dist/static/security-headers.json',
        'dist/static/assets/@epa-wg/cem-studio/bootstrap.js',
        'dist/static/assets/@epa-wg/cem-studio/repository.js',
        'dist/static/assets/@epa-wg/cem-studio/file-system-provider.js',
        'dist/static/assets/@epa-wg/cem-studio/feature-tour.js',
        'dist/static/assets/@epa-wg/cem-studio/workbench.js',
        'dist/static/assets/@epa-wg/cem-studio/preview.js',
        'dist/static/assets/@epa-wg/cem-ml-cli/dist/browser-worker.js',
        'dist/static/assets/@epa-wg/cem-ml/dist/wasm/browser/cem_ml_bg.wasm',
        'dist/static/assets/@epa-wg/cem-components/dist/index.js',
        'dist/static/assets/@epa-wg/cem-theme/dist/lib/css/cem-combined.css',
        'src/bootstrap.d.ts',
        'src/feature-tour.d.ts',
        'src/file-system-provider.d.ts',
        'src/repository.d.ts',
        'src/preview.d.ts',
        'src/workbench.d.ts',
    ]) {
        assert.ok(statSync(resolve(installedStudioRoot, path)).isFile(), `installed package is missing ${path}`);
    }

    writeFileSync(
        resolve(consumerRoot, 'probe.mjs'),
        `import {
  loadCemMlBrowser,
  mountCemStudio,
  registerCemStudioServiceWorker,
  createCemStudioProjectRepository,
  createCemStudioFileSystemProvider,
  installCemStudioFeatureTour,
  createCemStudioFeatureTourWorkbench,
  createCemStudioPreview,
} from '@epa-wg/cem-studio';

console.log(JSON.stringify({
  exports: [loadCemMlBrowser, mountCemStudio, registerCemStudioServiceWorker, createCemStudioProjectRepository, createCemStudioFileSystemProvider, installCemStudioFeatureTour, createCemStudioFeatureTourWorkbench, createCemStudioPreview].map((value) => typeof value),
  navigatorPresent: 'navigator' in globalThis,
}));
`,
    );
    const probe = JSON.parse(capture(process.execPath, ['probe.mjs']));
    assert.deepEqual(probe.exports, [
        'function',
        'function',
        'function',
        'function',
        'function',
        'function',
        'function',
        'function',
    ]);

    const report = {
        schemaVersion: 1,
        package: installedStudio.name,
        version: installedStudio.version,
        runtimeCopies: 1,
        exactCliDependency: installedStudio.dependencies['@epa-wg/cem-ml-cli'],
        staticBootstrap: true,
        importSideEffects: false,
    };
    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    console.log(
        `Clean consumer verified ${installedStudio.name}@${installedStudio.version}: static bootstrap and one runtime copy.`,
    );
} finally {
    assert.ok(consumerRoot.startsWith(`${tmpdir()}${sep}cem-studio-consumer-`));
    rmSync(consumerRoot, { recursive: true, force: true });
}

function pack(packageRoot) {
    const result = spawnSync(
        process.platform === 'win32' ? 'npm.cmd' : 'npm',
        ['pack', packageRoot, '--pack-destination', packRoot, '--json'],
        { cwd: consumerRoot, encoding: 'utf8' },
    );
    if (result.status !== 0) {
        throw new Error(`npm pack ${packageRoot} failed:\n${result.stderr || result.stdout || result.error}`);
    }
    const [{ filename }] = JSON.parse(result.stdout);
    return resolve(packRoot, filename);
}

function findRuntimeInstalls(root) {
    const matches = [];
    const visit = (directory) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            if (!entry.isDirectory() || entry.isSymbolicLink()) continue;
            const absolute = resolve(directory, entry.name);
            if (absolute.endsWith(`${sep}node_modules${sep}@epa-wg${sep}cem-ml`)) matches.push(absolute);
            visit(absolute);
        }
    };
    if (statSync(root).isDirectory()) visit(root);
    return matches.sort();
}

function run(command, args) {
    const result = spawnSync(command, args, { cwd: consumerRoot, stdio: 'inherit' });
    if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
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
