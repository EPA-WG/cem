import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const packageMetadata = JSON.parse(
  await readFile(new URL('../package.json', import.meta.url), 'utf8'),
);

test('Node export initializes the common WASM runtime', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  assert.equal(runtime.version(), packageMetadata.version);

  const manifest = JSON.parse(
    runtime.capabilityManifest(
      JSON.stringify({
        runtime: 'wasm-node',
        targetIdentity: 'runtime-test:node',
        abiIdentity: 'runtime-test-v1',
        debugControlActive: false,
      }),
    ),
  );
  assert.equal(manifest.runtime, 'wasm-node');
  assert.equal(manifest.commonVersion, packageMetadata.version);
  assert.equal(manifest.executorTopology, 'sequential');
});

test('browser loader accepts bytes and initializes the same WASM ABI', async () => {
  const browserRuntime = await import('../dist/wasm/browser/cem_ml.js');
  const wasm = await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url));
  await browserRuntime.default({ module_or_path: wasm });
  assert.equal(browserRuntime.version(), packageMetadata.version);

  const manifest = JSON.parse(
    browserRuntime.capabilityManifest(
      JSON.stringify({
        runtime: 'wasm-browser-worker',
        targetIdentity: 'runtime-test:browser',
        abiIdentity: 'runtime-test-v1',
        debugControlActive: false,
      }),
    ),
  );
  assert.equal(manifest.runtime, 'wasm-browser-worker');
  assert.equal(manifest.commonVersion, packageMetadata.version);
});

test('invalid capability requests remain structured diagnostics', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  const response = JSON.parse(runtime.capabilityManifest('{}'));
  assert.equal(response.error.code, 'cem.capability.invalid_request');
});

test('worker-pool capabilities and protocol descriptors remain Rust-owned', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  const protocol = JSON.parse(runtime.workerProtocolDescriptor());
  assert.equal(protocol.workerProtocolVersion, 1);
  assert.equal(protocol.operationProtocolVersion, 1);
  assert.equal(protocol.limits.maxWorkers, 256);

  const request = JSON.stringify({
    runtime: 'wasm-node',
    targetIdentity: 'runtime-test:node-worker-pool',
    abiIdentity: 'runtime-test-v1',
    debugControlActive: false,
  });
  const capability = JSON.parse(runtime.nodeWorkerCapabilityManifest(request, 3));
  assert.equal(capability.executorTopology, 'node-worker-pool');
  assert.equal(capability.effectiveMaxWorkers, 3);
  assert.equal(capability.commonVersion, packageMetadata.version);

  const invalid = JSON.parse(runtime.nodeWorkerCapabilityManifest(request, 0));
  assert.equal(invalid.error.code, 'cem.capability.worker_count');

  const browserRequest = JSON.stringify({
    runtime: 'wasm-browser-worker',
    targetIdentity: 'runtime-test:browser-worker-pool',
    abiIdentity: 'runtime-test-v1',
    debugControlActive: false,
  });
  const browserCapability = JSON.parse(runtime.browserWorkerCapabilityManifest(browserRequest, 2));
  assert.equal(browserCapability.executorTopology, 'browser-worker-pool');
  assert.equal(browserCapability.effectiveMaxWorkers, 2);
  assert.equal(browserCapability.commonVersion, packageMetadata.version);

  const browserRuntimeMismatch = JSON.parse(runtime.browserWorkerCapabilityManifest(request, 2));
  assert.equal(browserRuntimeMismatch.error.code, 'cem.capability.runtime_mismatch');
});
