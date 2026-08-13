import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
const runtimeMetadata = readJson(resolve(workspaceRoot, 'packages/cem-ml-npm/package.json'));

assert.equal(packageMetadata.version, runtimeMetadata.version);
assert.deepEqual(packageMetadata.dependencies, {
    '@epa-wg/cem-ml': runtimeMetadata.version,
});
assert.equal(packageMetadata.bin, undefined);
assert.equal(packageMetadata.exports['./browser'], undefined);
assert.equal(packageMetadata.exports['./node'].import, './dist/node.js');
for (const path of ['node.js', 'node.d.ts', 'node-worker.js', 'protocol.js', 'protocol.d.ts']) {
    assert.ok(existsSync(resolve(projectRoot, 'dist', path)), `missing Node host artifact: ${path}`);
}
const workerSource = readFileSync(resolve(projectRoot, 'dist/node-worker.js'), 'utf8');
assert.match(workerSource, /@epa-wg\/cem-ml\/wasm/);
assert.doesNotMatch(workerSource, /cem_ml_bg\.wasm/);

console.log(
    `Verified ${packageMetadata.name}@${packageMetadata.version}: exact runtime dependency and policy-free Node worker artifacts.`,
);

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
