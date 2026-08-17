import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
const runtimeMetadata = readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/package.json'));

assert.equal(packageMetadata.version, runtimeMetadata.version);
assert.deepEqual(packageMetadata.dependencies, {
    '@epa-wg/cem-ml': runtimeMetadata.version,
});
assert.deepEqual(packageMetadata.bin, { 'cem-ml': './dist/bin.js' });
assert.equal(packageMetadata.exports['./browser'].import, './dist/browser.js');
assert.equal(packageMetadata.exports['./node'].import, './dist/node.js');
for (const path of [
    'bin.js',
    'bin.d.ts',
    'browser-command.js',
    'browser-command.d.ts',
    'browser-invocation.js',
    'browser-invocation.d.ts',
    'browser.js',
    'browser.d.ts',
    'browser-worker.js',
    'command.js',
    'command.d.ts',
    'generated/command-schema.js',
    'generated/command-schema.d.ts',
    'node.js',
    'node.d.ts',
    'node-command.js',
    'node-command.d.ts',
    'node-host.js',
    'node-host.d.ts',
    'node-invocation.js',
    'node-invocation.d.ts',
    'node-service.js',
    'node-service.d.ts',
    'node-worker.js',
    'operation.js',
    'operation.d.ts',
    'protocol.js',
    'protocol.d.ts',
]) {
    assert.ok(existsSync(resolve(projectRoot, 'dist', path)), `missing worker-host artifact: ${path}`);
}
const commandSource = readFileSync(resolve(projectRoot, 'dist/command.js'), 'utf8');
const generatedCommandSource = readFileSync(
    resolve(projectRoot, 'dist/generated/command-schema.js'),
    'utf8',
);
assert.match(commandSource, /generatedCommandSchema/);
assert.doesNotMatch(commandSource, /--query-file|--template-expression|--resolver-read-map/);
assert.match(generatedCommandSource, /"schemaVersion": 1/);
assert.ok(generatedCommandSource.includes(`"commonVersion": "${packageMetadata.version}"`));
assert.match(generatedCommandSource, /"long": "query-file"/);
for (const host of ['browser', 'node']) {
    const hostSource = readFileSync(resolve(projectRoot, `dist/${host}.js`), 'utf8');
    assert.match(hostSource, /command\.js/);
}
const workerSource = readFileSync(resolve(projectRoot, 'dist/node-worker.js'), 'utf8');
assert.match(workerSource, /@epa-wg\/cem-ml\/wasm/);
assert.doesNotMatch(workerSource, /cem_ml_bg\.wasm/);
const browserSource = readFileSync(resolve(projectRoot, 'dist/browser.js'), 'utf8');
const browserCommandSource = readFileSync(resolve(projectRoot, 'dist/browser-command.js'), 'utf8');
const browserInvocationSource = readFileSync(resolve(projectRoot, 'dist/browser-invocation.js'), 'utf8');
const browserWorkerSource = readFileSync(resolve(projectRoot, 'dist/browser-worker.js'), 'utf8');
assert.match(browserSource, /new Worker\(new URL\('\.\/browser-worker\.js'/);
assert.match(browserSource, /hardwareConcurrency/);
assert.match(browserWorkerSource, /browserWorkerCapabilityManifest/);
const browserClosure = `${browserSource}\n${browserCommandSource}\n${browserInvocationSource}\n${browserWorkerSource}`;
assert.doesNotMatch(browserClosure, /SharedArrayBuffer/);
assert.doesNotMatch(browserClosure, /node:/);

const runtimeManifest = readJson(resolve(projectRoot, 'dist/cem-ml-cli-runtime.json'));
assert.equal(runtimeManifest.commonVersion, packageMetadata.version);
assert.deepEqual(runtimeManifest.runtimeIdentities, ['wasm-browser-worker', 'wasm-node']);
assert.deepEqual(runtimeManifest.targetIdentities, [
    'wasm32-unknown-unknown:web',
    'wasm32-unknown-unknown:nodejs',
]);
assert.equal(runtimeManifest.runtimeDependency.name, '@epa-wg/cem-ml');
assert.equal(runtimeManifest.runtimeDependency.version, packageMetadata.version);

const integrity = readJson(resolve(projectRoot, 'dist/integrity.json'));
assert.equal(integrity.commonVersion, packageMetadata.version);
assert.equal(integrity.algorithm, 'sha256');
const actualIntegrityPaths = listFiles(resolve(projectRoot, 'dist')).filter((path) => path !== 'integrity.json');
assert.deepEqual(integrity.files.map(({ path }) => path), actualIntegrityPaths);
for (const entry of integrity.files) {
    const bytes = readFileSync(resolve(projectRoot, 'dist', entry.path));
    assert.equal(entry.byteLength, bytes.byteLength, entry.path);
    assert.equal(entry.sha256, createHash('sha256').update(bytes).digest('hex'), entry.path);
}

const typecheck = spawnSync(
    process.platform === 'win32' ? 'yarn.cmd' : 'yarn',
    [
        'tsc',
        '--ignoreConfig',
        '--noEmit',
        '--strict',
        '--target',
        'ES2022',
        '--module',
        'NodeNext',
        '--moduleResolution',
        'NodeNext',
        'tests/browser-command-types.ts',
        'tests/node-command-types.ts',
    ],
    { cwd: projectRoot, encoding: 'utf8' },
);
assert.equal(
    typecheck.status,
    0,
    `browser command-service declarations failed strict TypeScript checking:\n${typecheck.stdout}${typecheck.stderr}`,
);

console.log(
    `Verified ${packageMetadata.name}@${packageMetadata.version}: generated command schema and policy-free browser/Node worker artifacts.`,
);

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function listFiles(root) {
    const files = [];
    const visit = (directory) => {
        for (const entry of readdirSync(directory, { withFileTypes: true })) {
            const path = resolve(directory, entry.name);
            if (entry.isDirectory()) visit(path);
            else if (entry.isFile()) files.push(relative(root, path).split(sep).join('/'));
        }
    };
    visit(root);
    return files.sort();
}
