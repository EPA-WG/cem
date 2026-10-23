import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const options = Object.fromEntries(process.argv.slice(2).map(argument => {
  const match = /^--(pack|wasm)=(.+)$/.exec(argument);
  if (!match) throw new Error('usage: node verify-schema-assets.mjs [--pack=<directory>] [--wasm=<Node module>]');
  return [match[1], resolve(match[2])];
}));
const packRoot = options.pack ?? resolve(projectRoot, 'dist-schema-assets');
const emitter = resolve(workspaceRoot, 'dist/target/cem_ml_schema_assets/debug', process.platform === 'win32' ? 'cem-ml-schema-assets-emit.exe' : 'cem-ml-schema-assets-emit');
const temporaryRoot = mkdtempSync(join(tmpdir(), 'cem-schema-assets-verify-'));
try {
  // This compiled native producer is the trusted comparison, not an adjacent
  // downloaded index.sha256 file. No schema source is parsed by JavaScript.
  const canonicalRoot = resolve(temporaryRoot, 'native');
  const emitted = spawnSync(emitter, ['--out', canonicalRoot], { cwd: workspaceRoot, encoding: 'utf8' });
  assert.equal(emitted.status, 0, `native schema-asset emission failed: ${emitted.error?.message ?? emitted.stderr}`);
  const canonicalFiles = listFiles(canonicalRoot);
  assert.deepEqual(listFiles(packRoot), canonicalFiles, 'optional delivery must contain exactly the native emission');
  for (const path of canonicalFiles) assert.deepEqual(readFileSync(resolve(packRoot, path)), readFileSync(resolve(canonicalRoot, path)), `native emission drift: ${path}`);

  // A separately installed, disconnected file transport contains the full
  // immutable inventory. It never consults source trees or the network.
  const installedRoot = resolve(temporaryRoot, 'installed');
  cpSync(packRoot, installedRoot, { recursive: true });
  const indexBytes = readFileSync(resolve(installedRoot, 'index.json'));
  const expectedDigest = sha256(readFileSync(resolve(canonicalRoot, 'index.json')));
  assert.equal(readFileSync(resolve(installedRoot, 'index.sha256'), 'utf8'), `${expectedDigest}\n`);
  const index = JSON.parse(indexBytes);
  const bootstrap = readJson(resolve(installedRoot, 'bootstrap.json'));
  assert.equal(bootstrap.indexDigest, expectedDigest);
  assert.equal(bootstrap.indexBytes, indexBytes.length);
  assert.equal(bootstrap.packageCount, index.packages.length);
  assert.equal(bootstrap.sourceCount, index.sources.length);
  assert.equal(bootstrap.sourceBytes, index.sources.reduce((sum, source) => sum + source.byteLength, 0));
  const measurements = readJson(resolve(installedRoot, 'measurements.json'));
  const integrity = readJson(resolve(installedRoot, 'integrity.json'));
  assert.equal(integrity.algorithm, 'sha256');
  assert.equal(integrity.indexSha256, expectedDigest);
  assert.deepEqual(integrity.files.map(file => file.path), canonicalFiles.filter(path => path !== 'integrity.json'));
  assert.deepEqual([...integrity.offlineUrls].sort(), canonicalFiles);
  for (const file of integrity.files) {
    const bytes = offlineRead(file.path);
    assert.equal(bytes.length, file.bytes, `offline byte length drift: ${file.path}`);
    assert.equal(sha256(bytes), file.sha256, `offline digest drift: ${file.path}`);
  }
  const paths = new Set(index.sources.map(source => `sources/${source.digest}`));
  assert.equal(paths.size, measurements.digestFileCount);
  assert.equal(index.sources.length, measurements.sourceCount);
  assert.equal(index.packages.length, measurements.packageCount);
  assert.equal(indexBytes.length, measurements.indexBytes);
  assert.equal(measurements.embeddedFallbackRetained, true);
  assert.equal(measurements.binaryPartitioned, false);
  assert.equal(measurements.checkpointSavingsBytes, 0);
  for (const source of index.sources) {
    const bytes = offlineRead(`sources/${source.digest}`);
    assert.equal(bytes.length, source.byteLength);
    assert.equal(sha256(bytes), source.digest);
  }
  const missingPath = `sources/${index.sources[0].digest}`;
  const missingBytes = offlineRead(missingPath);
  rmSync(resolve(installedRoot, missingPath));
  assert.throws(() => offlineRead(missingPath), /ENOENT/);
  writeFileSync(resolve(installedRoot, missingPath), missingBytes);
  assert.throws(() => offlineRead('../index.json'), /undeclared/);

  const report = {
    formatVersion: 1,
    nativeEmissionMatches: true,
    offlineInventoryFiles: canonicalFiles.length,
    ...measurements,
    rawDownloadBytes: canonicalFiles.reduce((sum, path) => sum + offlineRead(path).length, 0),
    gzipDownloadBytes: canonicalFiles.reduce((sum, path) => sum + gzipSync(offlineRead(path), { level: 9 }).length, 0),
    nativeRuntime: options.wasm ? verifyWasm(options.wasm, indexBytes, expectedDigest, index, offlineRead) : null,
  };
  const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-ml-npm');
  // The build emitter writes immutable scalar measurements; this mutable
  // verifier report belongs in the ignored release evidence directory.
  mkdirSync(reportRoot, { recursive: true });
  writeFileSync(resolve(reportRoot, 'schema-assets.json'), `${JSON.stringify(report, null, 2)}\n`);
  console.log(`Verified ${index.sources.length} optional native schema sources, ${canonicalFiles.length} offline files${options.wasm ? ' and independent WASM reimports' : ''}.`);

  function offlineRead(path) {
    assert.ok(canonicalFiles.includes(path), `undeclared offline asset: ${path}`);
    return readFileSync(resolve(installedRoot, path));
  }
} finally {
  rmSync(temporaryRoot, { recursive: true, force: true });
}

function verifyWasm(modulePath, indexBytes, digest, index, read) {
  const defaults = createRequire(import.meta.url)(resolve(projectRoot, 'dist/wasm/node/cem_ml.js'));
  assert.equal(defaults.schemaAssetPackBootstrapV1, undefined, 'default ABI must not include optional bootstrap');
  assert.equal(defaults.schemaAssetPackSummaryV1, undefined, 'default ABI must not include optional pack probes');
  assert.equal(defaults.schemaAssetPackImportV1, undefined, 'default ABI must not include optional pack probes');
  const loadRuntime = name => {
    const root = resolve(temporaryRoot, name);
    cpSync(dirname(modulePath), root, { recursive: true });
    return createRequire(import.meta.url)(resolve(root, basename(modulePath)));
  };
  const runtimes = ['first', 'fresh'].map(loadRuntime);
  assert.notEqual(runtimes[0].runtimeMemoryStatus, runtimes[1].runtimeMemoryStatus);
  assert.ok(runtimes[0].commandNativeMemoryIdentity() instanceof WebAssembly.Memory);
  assert.notEqual(runtimes[0].commandNativeMemoryIdentity(), runtimes[1].commandNativeMemoryIdentity());
  const samples = [];
  for (const runtime of runtimes) {
    assert.equal(typeof runtime.schemaAssetPackBootstrapV1, 'function');
    assert.equal(typeof runtime.schemaAssetPackSummaryV1, 'function');
    assert.equal(typeof runtime.schemaAssetPackImportV1, 'function');
    const before = JSON.parse(runtime.runtimeMemoryStatus());
    const inventoryBefore = parseSuccess(runtime.commandNativeResourceInventory());
    const bootstrap = parseSuccess(runtime.schemaAssetPackBootstrapV1());
    assert.equal(bootstrap.indexDigest, digest, 'optional binary must independently pin the native index');
    assert.equal(bootstrap.indexBytes, indexBytes.length);
    assert.equal(bootstrap.packageCount, index.packages.length);
    assert.equal(bootstrap.sourceCount, index.sources.length);
    assert.equal(bootstrap.sourceBytes, index.sources.reduce((sum, source) => sum + source.byteLength, 0));
    const started = performance.now();
    const summary = parseSuccess(runtime.schemaAssetPackSummaryV1(indexBytes, digest));
    const summaryMs = performance.now() - started;
    assert.deepEqual(summary, bootstrap);
    const kinds = new Set();
    const imported = [];
    for (const source of index.sources) {
      if (kinds.has(source.kind)) continue;
      kinds.add(source.kind);
      const bytes = read(`sources/${source.digest}`);
      const begin = performance.now();
      const first = parseSuccess(runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, bytes));
      assert.equal(first.indexDigest, digest);
      assert.equal(first.packageId, source.packageId);
      assert.equal(first.version, source.version);
      assert.equal(first.path, source.path);
      assert.equal(first.digest, source.digest);
      assert.equal(first.sourceUri, source.path);
      assert.equal(first.sourceBytes, source.byteLength);
      assert.ok(Number.isSafeInteger(first.nodeCount) && first.nodeCount > 0);
      imported.push({ kind: source.kind, coldImportMs: performance.now() - begin, result: first });
      const repeated = parseSuccess(runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, bytes));
      assert.deepEqual(repeated, first, 'stateless original source reimport drift');
      const tampered = Uint8Array.from(bytes);
      tampered[0] ^= 1;
      refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, tampered), 'cem.schema_asset.digest');
      refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, new Uint8Array()), 'cem.schema_asset.limit');
      refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, new Uint8Array(source.byteLength + 1)), 'cem.schema_asset.limit');
    }
    refused(() => runtime.schemaAssetPackSummaryV1(indexBytes, '0'.repeat(64)), 'cem.schema_asset.digest');
    const source = index.sources[0];
    const memory = runtime.commandNativeMemoryIdentity();
    const oversizedIndex = new Uint8Array(4 * 1024 * 1024 + 1);
    const oversizedSource = new Uint8Array(16 * 1024 * 1024 + 1);
    const wrongDeclaredLength = new Uint8Array(16 * 1024 * 1024);
    assert.notEqual(wrongDeclaredLength.length, source.byteLength);
    const beforeOversize = JSON.parse(runtime.runtimeMemoryStatus());
    const oversizeMemoryBytes = memory.buffer.byteLength;
    refused(() => runtime.schemaAssetPackSummaryV1(oversizedIndex, digest), 'cem.schema_asset.limit');
    assert.equal(memory.buffer.byteLength, oversizeMemoryBytes, 'oversized index transport must refuse before WASM copying/growth');
    refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, oversizedSource), 'cem.schema_asset.limit');
    assert.equal(memory.buffer.byteLength, oversizeMemoryBytes, 'oversized source transport must refuse before WASM copying/growth');
    refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, wrongDeclaredLength), 'cem.schema_asset.limit');
    assert.equal(memory.buffer.byteLength, oversizeMemoryBytes, 'source length inconsistent with its pinned entry must refuse before WASM copying/growth');
    const afterOversize = JSON.parse(runtime.runtimeMemoryStatus());
    assert.equal(afterOversize.activeOperations, beforeOversize.activeOperations);
    assert.equal(afterOversize.reservedBytes, beforeOversize.reservedBytes);
    refused(() => runtime.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, 'missing.cem', read(`sources/${source.digest}`)), 'cem.schema_asset.missing');
    const after = JSON.parse(runtime.runtimeMemoryStatus());
    const inventoryAfter = parseSuccess(runtime.commandNativeResourceInventory());
    assert.equal(inventoryBefore.complete, true);
    assert.equal(inventoryAfter.complete, true);
    assert.equal(inventoryAfter.blockingResources, inventoryBefore.blockingResources);
    assert.deepEqual(inventoryAfter.registries, inventoryBefore.registries, 'stateless schema imports must release native roots');
    assert.equal(after.activeOperations, 0);
    assert.equal(after.reservedBytes, 0);
    samples.push({ before, after, inventoryBefore, inventoryAfter, beforeOversize, afterOversize, summaryMs, summary, imported });
  }
  assert.deepEqual(samples[0].summary, samples[1].summary);
  assert.deepEqual(samples[0].imported.map(({ result }) => result), samples[1].imported.map(({ result }) => result));
  const guarded = loadRuntime('closed-and-budgeted');
  assert.notEqual(guarded.commandNativeMemoryIdentity(), runtimes[0].commandNativeMemoryIdentity());
  const source = index.sources[0];
  const sourceBytes = read(`sources/${source.digest}`);
  const operations = [
    () => guarded.schemaAssetPackBootstrapV1(),
    () => guarded.schemaAssetPackSummaryV1(indexBytes, digest),
    () => guarded.schemaAssetPackImportV1(indexBytes, digest, source.packageId, source.version, source.path, sourceBytes),
  ];
  const guardedInitial = JSON.parse(guarded.runtimeMemoryStatus());
  const inventoryInitial = parseSuccess(guarded.commandNativeResourceInventory());
  parseSuccess(guarded.configureRuntimeMemoryBudget(JSON.stringify({ ...guardedInitial.config, softLimitBytes: 1 })));
  for (const operation of operations) refused(operation, 'cem.runtime.memory.admission');
  const budgetRefused = JSON.parse(guarded.runtimeMemoryStatus());
  const inventoryBudget = parseSuccess(guarded.commandNativeResourceInventory());
  assert.equal(budgetRefused.activeOperations, 0);
  assert.equal(budgetRefused.reservedBytes, 0);
  assert.deepEqual(inventoryBudget.registries, inventoryInitial.registries);
  assert.equal(inventoryBudget.blockingResources, inventoryInitial.blockingResources);
  parseSuccess(guarded.configureRuntimeMemoryBudget(JSON.stringify(guardedInitial.config)));
  assert.deepEqual(parseSuccess(operations[0]()), samples[0].summary, 'failed memory admission must leave the normal native route usable');
  guarded.closeCommandNativeAdmission();
  for (const operation of operations) refused(operation, 'cem.runtime.admission.closed');
  const admissionClosed = JSON.parse(guarded.runtimeMemoryStatus());
  const inventoryClosed = parseSuccess(guarded.commandNativeResourceInventory());
  assert.equal(admissionClosed.activeOperations, 0);
  assert.equal(admissionClosed.reservedBytes, 0);
  assert.deepEqual(inventoryClosed.registries, inventoryInitial.registries);
  assert.equal(inventoryClosed.blockingResources, inventoryInitial.blockingResources);
  // Closing the disposable generation does not close either earlier heap.
  assert.deepEqual(parseSuccess(runtimes[1].schemaAssetPackBootstrapV1()), samples[0].summary);
  const optionalWasm = readFileSync(resolve(dirname(modulePath), 'cem_ml_bg.wasm'));
  const defaultWasm = readFileSync(resolve(projectRoot, 'dist/wasm/node/cem_ml_bg.wasm'));
  return {
    independentGenerations: runtimes.length + 1,
    memoryAdmission: { guardedInitial, budgetRefused, admissionClosed, inventoryInitial, inventoryBudget, inventoryClosed },
    sourceKindsImported: samples[0].imported.length,
    defaultWasmBytes: defaultWasm.length,
    defaultWasmGzipBytes: gzipSync(defaultWasm, { level: 9 }).length,
    optionalWasmBytes: optionalWasm.length,
    optionalWasmGzipBytes: gzipSync(optionalWasm, { level: 9 }).length,
    embeddedFallbackRetained: true,
    samples,
  };
}

function parseSuccess(value) {
  const result = JSON.parse(value);
  assert.ok(!result.error, `optional native pack refused: ${JSON.stringify(result)}`);
  return result;
}
function refused(run, expectedCode) {
  let diagnostic;
  try {
    const result = JSON.parse(run());
    assert.ok(result.error, 'invalid optional source was admitted');
    diagnostic = typeof result.error === 'string' ? result.error : result.error.code;
  } catch (error) {
    if (error?.code === 'ERR_ASSERTION') throw error;
    diagnostic = typeof error === 'string' ? error : error?.message ?? String(error);
  }
  assert.equal(diagnostic, expectedCode);
}
function listFiles(root) {
  const files = [];
  function visit(directory) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = resolve(directory, entry.name);
      if (entry.isDirectory()) visit(path);
      else {
        assert.ok(entry.isFile(), 'schema assets must be ordinary files');
        files.push(relative(root, path).split(sep).join('/'));
      }
    }
  }
  assert.ok(existsSync(root), `missing schema assets: ${root}`);
  visit(root);
  return files.sort();
}
function sha256(bytes) { return createHash('sha256').update(bytes).digest('hex'); }
function readJson(path) { return JSON.parse(readFileSync(path, 'utf8')); }
