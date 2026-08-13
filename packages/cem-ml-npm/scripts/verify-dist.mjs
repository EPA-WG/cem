import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, isAbsolute, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const distRoot = resolve(projectRoot, 'dist');
const packageMetadata = readJson(resolve(projectRoot, 'package.json'));
const cargoManifest = readFileSync(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
const cargoVersion = cargoManifest.match(
  /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
)?.[1];

assert.equal(packageMetadata.version, cargoVersion, 'npm and common Cargo versions must match');
assert.equal(packageMetadata.bin, undefined, 'low-level WASM package must not expose a bin');
assert.equal(packageMetadata.main, undefined, 'low-level package has no policy-bearing root export');
assert.equal(packageMetadata.exports['./wasm'].node.import, './dist/wasm/node/cem_ml.js');
assert.equal(packageMetadata.exports['./wasm'].browser.import, './dist/wasm/browser/cem_ml.js');

const runtimeManifest = readJson(resolve(distRoot, 'cem-ml-runtime.json'));
assert.equal(runtimeManifest.featureProfile, 'debug-control');
assert.equal(runtimeManifest.package.name, packageMetadata.name);
assert.equal(runtimeManifest.package.version, cargoVersion);
assert.equal(runtimeManifest.commonVersion, cargoVersion);
assert.match(
  runtimeManifest.abi.identity,
  /^wasm-bindgen@\d+\.\d+\.\d+;profile=debug-control$/,
);
assert.equal(runtimeManifest.capabilities.node.runtime, 'wasm-node');
assert.equal(runtimeManifest.capabilities.browser.runtime, 'wasm-browser-worker');
assert.equal(runtimeManifest.capabilities.node.commonVersion, cargoVersion);
assert.equal(runtimeManifest.capabilities.browser.commonVersion, cargoVersion);
assert.equal(runtimeManifest.protocol.workerProtocolVersion, 1);
assert.equal(runtimeManifest.protocol.operationProtocolVersion, 1);
assert.equal(runtimeManifest.protocol.limits.maxWorkers, 256);
assert.equal(runtimeManifest.schemaPackages.manifestCount, 25);
assert.ok(runtimeManifest.schemaPackages.fileCount > runtimeManifest.schemaPackages.manifestCount);
verifySchemaPackageReferences(resolve(distRoot, 'schema-packages'));

for (const target of ['browser', 'node']) {
  const artifact = runtimeManifest.artifacts[target];
  for (const path of Object.values(artifact)) {
    assert.ok(existsSync(resolve(distRoot, path)), `${target} artifact is missing: ${path}`);
  }
  const declarations = readFileSync(resolve(distRoot, artifact.types), 'utf8');
  assert.match(declarations, /version\(\): string/);
  assert.match(declarations, /capabilityManifest\(request_json: string\): string/);
  assert.match(declarations, /browserWorkerCapabilityManifest\(request_json: string, effective_max_workers: number\): string/);
  assert.match(declarations, /nodeWorkerCapabilityManifest\(request_json: string, effective_max_workers: number\): string/);
  assert.match(declarations, /workerProtocolDescriptor\(\): string/);
}

const nodeRuntime = createRequire(import.meta.url)(resolve(distRoot, 'wasm/node/cem_ml.js'));
assert.equal(nodeRuntime.version(), cargoVersion);
assert.equal(typeof nodeRuntime.capabilityManifest, 'function');
assert.equal(typeof nodeRuntime.browserWorkerCapabilityManifest, 'function');
assert.equal(typeof nodeRuntime.nodeWorkerCapabilityManifest, 'function');
assert.equal(typeof nodeRuntime.workerProtocolDescriptor, 'function');

const integrity = readJson(resolve(distRoot, 'integrity.json'));
assert.equal(integrity.algorithm, 'sha256');
const recordedPaths = integrity.files.map((entry) => entry.path);
const actualPaths = listFiles(distRoot).filter((path) => path !== 'integrity.json');
assert.deepEqual(recordedPaths, actualPaths, 'integrity must cover every generated file exactly once');
for (const entry of integrity.files) {
  const bytes = readFileSync(resolve(distRoot, entry.path));
  assert.equal(entry.bytes, bytes.byteLength, `byte length drift for ${entry.path}`);
  assert.equal(
    entry.sha256,
    createHash('sha256').update(bytes).digest('hex'),
    `SHA-256 drift for ${entry.path}`,
  );
}

console.log(
  `Verified ${packageMetadata.name}@${packageMetadata.version}: browser/Node ABI, ${integrity.files.length} integrity records, and ${runtimeManifest.schemaPackages.manifestCount} schema packages.`,
);

function readJson(path) {
  return JSON.parse(readFileSync(path, 'utf8'));
}

function listFiles(root) {
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const absolute = resolve(directory, entry.name);
      if (entry.isDirectory()) visit(absolute);
      else if (entry.isFile()) files.push(relative(root, absolute).split(sep).join('/'));
    }
  };
  visit(root);
  return files.sort();
}

function verifySchemaPackageReferences(schemaRoot) {
  for (const path of listFiles(schemaRoot).filter((path) => path.endsWith('/package.cem'))) {
    const manifestPath = resolve(schemaRoot, path);
    const manifest = readFileSync(manifestPath, 'utf8');
    for (const match of manifest.matchAll(/@(source|path)="([^"]+)"/g)) {
      if (match[2].includes('://')) continue;
      const referencedPath = resolve(dirname(manifestPath), match[2]);
      const referencedRelative = relative(schemaRoot, referencedPath);
      assert.ok(
        referencedRelative !== '..' &&
          !referencedRelative.startsWith(`..${sep}`) &&
          !isAbsolute(referencedRelative),
        `${path} ${match[1]} escapes the schema-package root: ${match[2]}`,
      );
      assert.ok(existsSync(referencedPath), `${path} references a missing asset: ${match[2]}`);
    }
  }
}
