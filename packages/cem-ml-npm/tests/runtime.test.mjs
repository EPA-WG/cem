import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
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

  const progress = [];
  const result = await executeVersionCommand(browserRuntime, 'wasm-browser-worker', undefined, {
    progress: (json) => {
      progress.push(JSON.parse(json));
      throw new Error('observational progress callbacks cannot change command semantics');
    },
  });
  assert.equal(result.status, 'succeeded');
  assert.equal(result.identity.runtime, 'wasm-browser-worker');
  assert.equal(result.result.value.kind, 'version-capabilities');
  assert.deepEqual(
    progress.map(({ sequence, stage, status }) => [sequence, stage, status]),
    [
      [1, 'accepted', undefined],
      [2, 'prepared', undefined],
      [3, 'executing', undefined],
      [4, 'terminal', 'succeeded'],
    ],
  );
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

test('async command-service binding owns success, stale, and callback failure projection', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  const success = await executeVersionCommand(runtime, 'wasm-node');
  assert.equal(success.protocolVersion, 1);
  assert.equal(success.requestId, 'wasm-command-version');
  assert.equal(success.operation, 'version-capabilities');
  assert.equal(success.status, 'succeeded');
  assert.equal(success.exitCode, 0);
  assert.equal(success.identity.runtime, 'wasm-node');
  assert.equal(success.result.storage, 'inline');
  assert.equal(success.result.value.value.version.commonVersion, packageMetadata.version);

  const staleProgress = [];
  const stale = await executeVersionCommand(
    runtime,
    'wasm-node',
    {
      project: { projectId: 'wasm-fixture', revision: 2 },
      resourceVersions: {},
    },
    { progress: (json) => staleProgress.push(JSON.parse(json)) },
  );
  assert.equal(stale.status, 'stale');
  assert.equal(stale.exitCode, null);
  assert.equal(stale.stale.currentProjectRevision, 2);
  assert.deepEqual(stale.stale.changedResources, []);
  assert.deepEqual(
    staleProgress.map(({ sequence, stage, status }) => [sequence, stage, status]),
    [
      [1, 'accepted', undefined],
      [2, 'terminal', 'stale'],
    ],
  );

  const failure = await executeVersionCommand(runtime, 'wasm-node', {
    error: { code: 'fixture.ledger', message: 'ledger unavailable' },
  });
  assert.equal(failure.error.code, 'cem.command_service.ledger_read');
  assert.match(failure.error.message, /fixture\.ledger: currentRevision: ledger unavailable/);
});

test('command-service registry owns duplicate admission, cooperative cancellation, progress, and cleanup', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  const progress = [];
  let resolveRevision;
  const pending = executeVersionCommand(runtime, 'wasm-node', undefined, {
    currentRevision: () =>
      new Promise((resolve) => {
        resolveRevision = resolve;
      }),
    progress: (json) => progress.push(JSON.parse(json)),
  });
  await Promise.resolve();
  assert.equal(typeof resolveRevision, 'function');

  const duplicate = await executeVersionCommand(runtime, 'wasm-node');
  assert.equal(duplicate.error.code, 'cem.command_service.request_active');

  const acknowledgement = JSON.parse(
    runtime.cancelCommandServiceV1('wasm-command-version', 'runtime fixture cancellation'),
  );
  assert.equal(acknowledgement.requestId, 'wasm-command-version');
  assert.equal(acknowledgement.disposition, 'accepted');
  assert.equal(acknowledgement.selectedScope, 0);
  resolveRevision(
    JSON.stringify({
      project: { projectId: 'wasm-fixture', revision: 1 },
      resourceVersions: {},
    }),
  );

  const cancelled = await pending;
  assert.equal(cancelled.status, 'cancelled');
  assert.equal(cancelled.exitCode, 130);
  assert.match(cancelled.diagnostics.items[0].message, /runtime fixture cancellation/);
  assert.deepEqual(
    progress.map(({ sequence, stage, status }) => [sequence, stage, status]),
    [
      [1, 'accepted', undefined],
      [2, 'terminal', 'cancelled'],
    ],
  );

  const inactive = JSON.parse(runtime.cancelCommandServiceV1('wasm-command-version'));
  assert.equal(inactive.error.code, 'cem.command_service.request_inactive');

  const reusedProgress = [];
  const reused = await executeVersionCommand(runtime, 'wasm-node', undefined, {
    progress: (json) => reusedProgress.push(JSON.parse(json)),
  });
  assert.equal(reused.status, 'succeeded');
  assert.deepEqual(
    reusedProgress.map(({ sequence, stage }) => [sequence, stage]),
    [
      [1, 'accepted'],
      [2, 'prepared'],
      [3, 'executing'],
      [4, 'terminal'],
    ],
  );
});

test('async command-service binding hydrates and transactionally publishes through host callbacks', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  const sourceUri = 'memory:fixture.css';
  const outputUri = 'memory:fixture.cem';
  const sourceBytes = new TextEncoder().encode('.card { color: red; }');
  const version = {
    revision: 1,
    sha256: createHash('sha256').update(sourceBytes).digest('hex'),
  };
  const runConfig = {
    inputs: [
      {
        uri: sourceUri,
        rootScope: {
          defaultContentType: 'text/css',
          schema: 'https://cem.dev/ns/data/css/1',
        },
      },
    ],
    outputs: [
      {
        destination: outputUri,
        rootScope: {
          defaultContentType: 'application/cem',
          schema: 'https://cem.dev/ns/cem-ml/1',
        },
      },
    ],
  };
  const runPlan = JSON.parse(
    runtime.normalizeCommandRunPlanV1(
      JSON.stringify({ configBytes: [...new TextEncoder().encode(JSON.stringify(runConfig))] }),
    ),
  );
  assert.equal(runPlan.inputs[0].inputId, 'input:0');
  assert.equal(runPlan.outputs[0].inputId, 'input:0');

  const request = {
    protocolVersion: 1,
    requestId: 'wasm-command-parse',
    project: { projectId: 'wasm-fixture', revision: 1 },
    resourceVersions: { [sourceUri]: version },
    operation: {
      kind: 'parse',
      inputId: 'input:0',
      projection: 'ast',
      preserveSourceOffsets: true,
    },
    runPlan,
    resources: {},
    policyStamp: {
      resolver: 'fixture-resolver',
      safety: 'fixture-safety',
      budget: 'fixture-budget',
    },
  };
  const events = [];
  const writes = new Map();
  const result = JSON.parse(
    await runtime.executeCommandServiceV1(
      JSON.stringify(request),
      JSON.stringify({
        runtime: 'wasm-node',
        targetIdentity: 'runtime-test:wasm-command',
        abiIdentity: 'runtime-test-v1',
        debugControlActive: false,
      }),
      async (json) => {
        events.push(['ledger', JSON.parse(json)]);
        return JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions });
      },
      async (json) => {
        const read = JSON.parse(json);
        events.push(['read', read]);
        return JSON.stringify({
          version,
          bytes: [...sourceBytes],
          identity: {
            contentType: 'text/css',
            schema: 'https://cem.dev/ns/data/css/1',
          },
        });
      },
      async (json, bytes) => {
        const write = JSON.parse(json);
        events.push(['prepare', write]);
        writes.set('write:1', { write, bytes: [...bytes], committed: false });
        return JSON.stringify({ token: 'write:1' });
      },
      async (token) => {
        events.push(['commit', token]);
        const write = writes.get(token);
        assert.ok(write);
        write.committed = true;
        return JSON.stringify({ uri: write.write.uri });
      },
      async (token) => {
        events.push(['rollback', token]);
        writes.delete(token);
      },
    ),
  );

  assert.equal(result.status, 'succeeded');
  assert.equal(result.operation, 'parse');
  assert.equal(result.result.value.kind, 'parse');
  assert.equal(result.artifacts.originalCount, 1);
  assert.equal(result.artifacts.items[0].uri, outputUri);
  assert.deepEqual(
    events.map(([kind]) => kind),
    ['ledger', 'read', 'prepare', 'ledger', 'commit'],
  );
  assert.equal(events[1][1].purposes[0], 'input');
  assert.equal(events[2][1].purpose, 'output');
  assert.equal(writes.get('write:1').committed, true);
  assert.match(new TextDecoder().decode(Uint8Array.from(writes.get('write:1').bytes)), /@doc cem-ml 1/);
});

async function executeVersionCommand(runtime, runtimeKind, ledger = undefined, options = {}) {
  const request = {
    protocolVersion: 1,
    requestId: 'wasm-command-version',
    project: { projectId: 'wasm-fixture', revision: 1 },
    resourceVersions: {},
    operation: { kind: 'version-capabilities' },
    runPlan: null,
    resources: {},
    policyStamp: {
      resolver: 'fixture-resolver',
      safety: 'fixture-safety',
      budget: 'fixture-budget',
    },
  };
  const capabilityRequest = {
    runtime: runtimeKind,
    targetIdentity: `runtime-test:${runtimeKind}`,
    abiIdentity: 'runtime-test-v1',
    debugControlActive: false,
  };
  const unexpected = async (boundary) => {
    throw new Error(`${boundary} callback must not run for version-capabilities`);
  };
  return JSON.parse(
    await runtime.executeCommandServiceV1(
      JSON.stringify(request),
      JSON.stringify(capabilityRequest),
      options.currentRevision ??
        (async () =>
          JSON.stringify(
            ledger ?? {
              project: request.project,
              resourceVersions: request.resourceVersions,
            },
          )),
      async () => unexpected('readResource'),
      async () => unexpected('prepareWrite'),
      async () => unexpected('commitWrite'),
      async () => unexpected('rollbackWrite'),
      options.progress,
    ),
  );
}
