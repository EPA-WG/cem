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
assert.equal(packageMetadata.exports['./browser'].import, './dist/browser.js');
assert.equal(packageMetadata.exports['./node'].import, './dist/node.js');
for (const path of [
    'browser.js',
    'browser.d.ts',
    'browser-worker.js',
    'node.js',
    'node.d.ts',
    'node-worker.js',
    'operation.js',
    'operation.d.ts',
    'protocol.js',
    'protocol.d.ts',
]) {
    assert.ok(existsSync(resolve(projectRoot, 'dist', path)), `missing worker-host artifact: ${path}`);
}
const workerSource = readFileSync(resolve(projectRoot, 'dist/node-worker.js'), 'utf8');
assert.match(workerSource, /@epa-wg\/cem-ml\/wasm/);
assert.doesNotMatch(workerSource, /cem_ml_bg\.wasm/);
const browserSource = readFileSync(resolve(projectRoot, 'dist/browser.js'), 'utf8');
const browserWorkerSource = readFileSync(resolve(projectRoot, 'dist/browser-worker.js'), 'utf8');
assert.match(browserSource, /new Worker\(new URL\('\.\/browser-worker\.js'/);
assert.match(browserSource, /hardwareConcurrency/);
assert.match(browserWorkerSource, /browserWorkerCapabilityManifest/);
assert.doesNotMatch(`${browserSource}\n${browserWorkerSource}`, /SharedArrayBuffer/);

console.log(
    `Verified ${packageMetadata.name}@${packageMetadata.version}: exact runtime dependency and policy-free browser/Node worker artifacts.`,
);

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
