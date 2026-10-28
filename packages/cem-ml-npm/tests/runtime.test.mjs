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

  const rendered = JSON.parse(
    runtime.renderCemMlToHtmlV1(
      JSON.stringify({ source: '{p @class=message | Offline preview}', sourceUrl: 'memory:demo.cem' }),
    ),
  );
  assert.equal(rendered.status, 'rendered');
  assert.equal(rendered.html, '<p class="message">Offline preview</p>');
  assert.equal(rendered.diagnostics.length, 0);
  assert.ok(rendered.outputSpans.length > 0);

  const highlighted = JSON.parse(
    runtime.highlightSourceToHtmlV1(
      JSON.stringify({ source: '<p class="message">Ready</p>', contentType: 'text/html' }),
    ),
  );
  assert.equal(highlighted.status, 'highlighted');
  assert.match(highlighted.html, /&lt;<b>p<\/b>/);
  assert.match(highlighted.html, /<var>class<\/var>=&quot;<i>message<\/i>&quot;/);
  assert.doesNotMatch(highlighted.html, /class=|data-role=/);
  verifyGraphAuthoring(runtime);
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
  assert.equal(typeof browserRuntime.readCommandArtifactV1, 'function');
  assert.equal(typeof browserRuntime.disposeCommandArtifactV1, 'function');
  assert.equal(typeof browserRuntime.disposeCommandArtifactsV1, 'function');
  assert.equal(typeof browserRuntime.renderCemMlToHtmlV1, 'function');
  assert.equal(typeof browserRuntime.highlightSourceToHtmlV1, 'function');
  verifyGraphAuthoring(browserRuntime);

  const rendered = JSON.parse(
    browserRuntime.renderCemMlToHtmlV1(JSON.stringify({ source: '{strong | Browser WASM}' })),
  );
  assert.equal(rendered.status, 'rendered');
  assert.equal(rendered.html, '<strong>Browser WASM</strong>');

  const highlighted = JSON.parse(
    browserRuntime.highlightSourceToHtmlV1(
      JSON.stringify({ source: '{strong @class="message" | Browser WASM}', contentType: 'application/cem' }),
    ),
  );
  assert.equal(highlighted.status, 'highlighted');
  assert.match(highlighted.html, /{<b>strong<\/b>/);
  assert.doesNotMatch(highlighted.html, /class=|data-role=/);

  const highlightedError = JSON.parse(
    browserRuntime.highlightSourceToHtmlV1(
      JSON.stringify({ source: '{article | Invalid: {42}}', contentType: 'application/cem' }),
    ),
  );
  assert.equal(highlightedError.diagnostics[0].code, 'cem.tokenizer.bare_brace_text');
  assert.match(highlightedError.html, /<mark>\{42\}<\/mark>/);

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

function verifyGraphAuthoring(runtime) {
  const source = '{run | {import @id=data @src=data.cem | {export @id=result @out=result.cem}}}\n';
  const describe = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source })));
  assert.equal(describe.source, source);
  assert.deepEqual(describe.nodes.map(node => node.id), ['data', 'result']);
  const edited = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source,
    edit: { node: 'result', attribute: 'out', value: 'café "literal".cem' } })));
  assert.equal(edited.nodes[1].fields.find(field => field.id === 'out').value, 'café "literal".cem');
  assert.deepEqual(edited.graph.edges, describe.graph.edges);
  assert.deepEqual(describe.parents[0], { id: '', kind: 'run', children: ['import'] });
  const created = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source,
    create: { parent: 'data', kind: 'export', id: 'copy', attributes: { out: 'copy.cem' } } })));
  assert.deepEqual(created.nodes.map(node => node.id), ['data', 'result', 'copy']);
  assert.deepEqual(created.graph.nodes.slice(0, 2), describe.graph.nodes);
  assert.ok(JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source,
    create: { parent: 'data', kind: 'export', id: 'result', attributes: { out: 'copy.cem' } } }))).error);
  const edgeSource = '{run | {import @id=other @src=other.xml} {import @id=data @src=data.xml | {transform @id=view @src=view.xpath @template-content-type=application/vnd.cem.xpath | {export @id=result @out=result.xml}}}}';
  const linked = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source: edgeSource,
    edge: { node: 'view', role: 'input', source: 'other' } })));
  assert.equal(linked.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'input').source, 'other');
  const secondary = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source: linked.source,
    edge: { node: 'view', role: 'with', source: 'other', name: 'palette' } })));
  assert.deepEqual(secondary.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'with').bindings, [{ name: 'palette', source: 'other' }]);
  const moved = JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source: secondary.source,
    edge: { node: 'view', role: 'parent', source: 'other' } })));
  assert.equal(moved.nodes.find(node => node.id === 'view').edges.find(field => field.id === 'parent').source, 'other');
  assert.match(JSON.parse(runtime.authorTransformConfigV1(JSON.stringify({ source: edgeSource,
    edge: { node: 'view', role: 'input', source: 'view' } }))).error, /cycle/);
  assert.equal(typeof JSON.parse(runtime.authorTransformConfigV1('{"source":false}')).error, 'string');
}

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

test('trusted host grant setup is explicit, strict and scoped to one WASM execution', async () => {
  const runtime = await import('@epa-wg/cem-ml/wasm');
  for (const expectedOrigin of [
    { kind: 'builtin' }, { kind: 'untracked' },
    { kind: 'manifest', uri: 'vendor://old/package.cem' },
  ]) {
    const result = await executeVersionCommand(runtime, 'wasm-node', undefined, {
      hostConfigurationJson: JSON.stringify({ schemaPackageReplacementGrants: [{
        packageId: 'vendor', expectedOrigin, replacementManifestUri: 'vendor://new/package.cem',
      }] }),
    });
    assert.equal(result.status, 'succeeded');
  }
  let callbacks = 0;
  for (const configuration of [
    null, [], { extra: true },
    { schemaPackageReplacementGrants: [['vendor', { kind: 'builtin' }, 'vendor://new']] },
    { schemaPackageReplacementGrants: [{ packageId: '', expectedOrigin: { kind: 'builtin' }, replacementManifestUri: 'vendor://new' }] },
    { schemaPackageReplacementGrants: [{ packageId: 'vendor', expectedOrigin: { kind: 'any' }, replacementManifestUri: 'vendor://new' }] },
    { schemaPackageReplacementGrants: [{ packageId: 'vendor', expectedOrigin: { kind: 'builtin', uri: 'extra' }, replacementManifestUri: 'vendor://new' }] },
  ]) {
    const result = await executeVersionCommand(runtime, 'wasm-node', undefined, {
      hostConfigurationJson: JSON.stringify(configuration),
      currentRevision: async () => { callbacks++; throw new Error('must not run'); },
    });
    assert.equal(result.error.code, 'cem.command.host_configuration_invalid');
  }
  assert.equal(callbacks, 0);
  const next = await executeVersionCommand(runtime, 'wasm-node');
  assert.equal(next.status, 'succeeded');
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
  const secondOutputUri = 'memory:fixture-secondary.cem';
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
      {
        destination: secondOutputUri,
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
        const token = `write:${writes.size + 1}`;
        writes.set(token, { write, bytes: [...bytes], committed: false });
        return JSON.stringify({ token });
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
  assert.equal(result.artifacts.originalCount, 2);
  assert.deepEqual(
    new Set(result.artifacts.items.map(({ uri }) => uri)),
    new Set([outputUri, secondOutputUri]),
  );
  assert.deepEqual(
    events.map(([kind]) => kind),
    ['ledger', 'read', 'prepare', 'prepare', 'ledger', 'commit', 'commit'],
  );
  assert.equal(events[1][1].purposes[0], 'input');
  assert.equal(events[2][1].purpose, 'output');
  assert.ok([...writes.values()].every(({ committed }) => committed));
  assert.match(
    new TextDecoder().decode(Uint8Array.from(writes.get('write:1').bytes)),
    /@doc cem-ml 1/,
  );

  const artifact = result.artifacts.items[0];
  const firstRead = runtime.readCommandArtifactV1(request.requestId, artifact.handleId, 0, 16);
  const firstMetadata = JSON.parse(firstRead.json);
  assert.ok(firstRead.bytes instanceof Uint8Array);
  assert.equal(firstMetadata.requestId, request.requestId);
  assert.deepEqual(firstMetadata.handle, artifact);
  assert.equal(firstMetadata.offset, 0);
  assert.equal(firstMetadata.byteLength, firstRead.bytes.byteLength);
  assert.equal(firstMetadata.eof, artifact.byteLength <= 16);

  const firstByte = firstRead.bytes[0];
  firstRead.bytes[0] ^= 0xff;
  const repeatedRead = runtime.readCommandArtifactV1(request.requestId, artifact.handleId, 0, 16);
  assert.equal(repeatedRead.bytes[0], firstByte, 'artifact reads must return owned byte copies');

  const chunks = [];
  let offset = 0;
  while (offset < artifact.byteLength) {
    const read = runtime.readCommandArtifactV1(request.requestId, artifact.handleId, offset, 16);
    const metadata = JSON.parse(read.json);
    assert.equal(metadata.offset, offset);
    chunks.push(...read.bytes);
    offset += read.bytes.byteLength;
    assert.equal(metadata.eof, offset === artifact.byteLength);
  }
  assert.equal(chunks.length, artifact.byteLength);
  assert.equal(createHash('sha256').update(Uint8Array.from(chunks)).digest('hex'), artifact.sha256);

  const rangeFailure = JSON.parse(
    runtime.readCommandArtifactV1(
      request.requestId,
      artifact.handleId,
      artifact.byteLength + 1,
      1,
    ).json,
  );
  assert.equal(rangeFailure.error.code, 'cem.command_service.artifact_read_range');
  const limitFailure = JSON.parse(
    runtime.readCommandArtifactV1(request.requestId, artifact.handleId, 0, 64 * 1024 * 1024 + 1)
      .json,
  );
  assert.equal(limitFailure.error.code, 'cem.command_service.artifact_read_limit');
  const unknownFailure = JSON.parse(
    runtime.readCommandArtifactV1(request.requestId, artifact.handleId + 1000, 0, 1).json,
  );
  assert.equal(unknownFailure.error.code, 'cem.command_service.artifact_handle_unknown');

  const foreignFailure = JSON.parse(
    runtime.readCommandArtifactV1('wasm-artifact-foreign', artifact.handleId, 0, 1).json,
  );
  assert.equal(foreignFailure.error.code, 'cem.command_service.artifact_handle_foreign');

  const disposed = JSON.parse(
    runtime.disposeCommandArtifactV1(request.requestId, artifact.handleId),
  );
  assert.equal(disposed.disposition, 'disposed');
  assert.equal(disposed.disposedCount, 1);
  const disposedAgain = JSON.parse(
    runtime.disposeCommandArtifactV1(request.requestId, artifact.handleId),
  );
  assert.equal(disposedAgain.disposition, 'already-disposed');
  const disposedRead = JSON.parse(
    runtime.readCommandArtifactV1(request.requestId, artifact.handleId, 0, 1).json,
  );
  assert.equal(disposedRead.error.code, 'cem.command_service.artifact_handle_disposed');

  const disposedRequest = JSON.parse(runtime.disposeCommandArtifactsV1(request.requestId));
  assert.equal(disposedRequest.disposition, 'disposed');
  assert.equal(disposedRequest.disposedCount, 1);
  assert.equal(
    JSON.parse(runtime.disposeCommandArtifactsV1(request.requestId)).disposition,
    'already-disposed',
  );
});

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`collection graph publication shares CLI manifest bytes and inline provenance in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm')
      : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') {
      await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    }
    const base = 'studio://collection/';
    const configUri = `${base}graph.cem`;
    const outputUri = `${base}out/book.json`;
    const sources = new Map([
      [configUri, `{run |
        {import @id=chapter @src=chapter.cem @content-type=text/cem-ml |
          {transform @id=page @src=view.cem @template-content-type=text/cem-ml |
            {join @id=book @mode=collect |
              {export @id=collection @out=out/book.json @content-type=application/json}
            }
          }
        }
      }`],
      [`${base}chapter.cem`, '{section | Source}'],
      [`${base}view.cem`, '{article | Café 🧬}'],
    ]);
    const resources = new Map([...sources].map(([uri, source]) => {
      const bytes = new TextEncoder().encode(source);
      return [uri, { bytes, version: {
        sha256: createHash('sha256').update(bytes).digest('hex'), revision: 1,
      } }];
    }));
    const request = {
      protocolVersion: 1,
      requestId: `wasm-collection-manifest:${runtimeKind}`,
      project: { projectId: 'collection-fixture', revision: 1 },
      resourceVersions: Object.fromEntries([...resources].map(([uri, value]) => [uri, value.version])),
      operation: { kind: 'transform', source: { kind: 'graph', configUri }, preserveSourceOffsets: true },
      runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
        ...new TextEncoder().encode(JSON.stringify({
          inputs: [{ uri: `${base}chapter.cem`,
            rootScope: { defaultContentType: 'text/cem-ml' } }],
        })),
      ] }))),
      resources: {},
      policyStamp: { resolver: 'collection-resolver', safety: 'collection-safety', budget: 'collection-budget' },
    };
    const writes = new Map();
    const committed = [];
    try {
      const result = JSON.parse(await runtime.executeCommandServiceV1(
        JSON.stringify(request),
        JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:collection',
          abiIdentity: 'runtime-test-v1', debugControlActive: false }),
        async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
        async (json) => {
          const { uri } = JSON.parse(json);
          const source = resources.get(uri);
          assert.ok(source, `known native graph resource: ${uri}`);
          return JSON.stringify({ version: source.version, bytes: [...source.bytes],
            identity: { contentType: 'text/cem-ml' } });
        },
        async (json, bytes) => {
          const write = JSON.parse(json);
          assert.equal(write.uri, outputUri, 'no implicit child or sidecar publication');
          assert.equal(writes.size, 0);
          writes.set('manifest', { write, bytes: Uint8Array.from(bytes) });
          return JSON.stringify({ token: 'manifest' });
        },
        async (token) => {
          committed.push(token);
          return JSON.stringify({ uri: writes.get(token).write.uri });
        },
        async () => assert.fail('successful graph must not roll back'),
      ));
      assert.equal(result.status, 'succeeded', JSON.stringify(result));
      assert.deepEqual(committed, ['manifest']);
      assert.equal(result.artifacts.originalCount, 1);
      const raw = result.result.value.value.value.artifacts[0].primary;
      assert.equal(raw.kind, 'collection');
      assert.equal(raw.count, 1);
      assert.ok(raw.items[0].sourceMap.frames.length > 0);
      assert.ok(raw.items[0].outputSpans.length > 0);
      const published = new TextDecoder().decode(writes.get('manifest').bytes);
      const manifest = JSON.parse(published);
      assert.equal(manifest.items[0].primary, '<article>Café 🧬</article>');
      assert.equal(manifest.items[0].sourceMap, undefined);
      assert.equal(manifest.items[0].outputSpans, undefined);
      // Compare exact native pretty-print bytes, including sorted keys and no final newline.
      const expected = {
        bindings: raw.bindings, count: raw.count, items: raw.items.map(item => ({
          artifactId: item.artifactId, bindings: item.bindings, destination: item.destination ?? null,
          identity: item.identity ?? null, input: item.input, primary: item.primary, uri: item.uri ?? null,
        })), kind: 'collection', mode: raw.mode,
      };
      assert.equal(published, JSON.stringify(expected, null, 2));
      assert.equal(result.artifacts.items[0].sha256,
        createHash('sha256').update(writes.get('manifest').bytes).digest('hex'));
    } finally {
      runtime.disposeCommandArtifactsV1(request.requestId);
    }
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`native module metadata and attributes preserve CLI bytes and provenance in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') {
      await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    }
    const base = 'studio://native-module-attributes/';
    const fixture = '../../cem_ml_transform_cem_ql/tests/fixtures/native-module-attributes/';
    const sources = new Map(await Promise.all(['asset.cem', 'build.cem', 'page.cem', 'stats.cem', 'ui.cem']
      .map(async name => [name, new Uint8Array(await readFile(new URL(`${fixture}${name}`, import.meta.url)))])));
    const expectedMain = await readFile(new URL(`${fixture}expected.html`, import.meta.url), 'utf8');
    const expectedStats = await readFile(new URL(`${fixture}expected-stats.html`, import.meta.url), 'utf8');
    for (const [index, kind] of ['asset', 'document', null].entries()) {
      const resources = Object.fromEntries([...sources].map(([name, original]) => {
        const bytes = name === 'asset.cem' ? new TextEncoder().encode(new TextDecoder().decode(original)
          .replace(' @kind=asset', kind === null ? '' : ` @kind=${kind}`)) : original;
        return [`${base}${name}`, { bytes: [...bytes], identity: { contentType: 'text/cem-ml' } }];
      }));
      const request = {
        protocolVersion: 1, requestId: `native-module-attributes:${runtimeKind}:${index}`,
        project: { projectId: 'native-module-attributes', revision: index + 1 },
        resourceVersions: Object.fromEntries(Object.entries(resources).map(([uri, value]) => [uri, {
          revision: index + 1, sha256: createHash('sha256').update(Uint8Array.from(value.bytes)).digest('hex'),
        }])),
        operation: { kind: 'transform', source: { kind: 'graph', configUri: `${base}build.cem` }, preserveSourceOffsets: true },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri: `${base}asset.cem`, rootScope: { defaultContentType: 'text/cem-ml' } }] })),
        ] }))), resources,
        policyStamp: { resolver: 'module-resolver', safety: 'module-safety', budget: 'module-budget' },
      };
      const writes = new Map();
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(
          JSON.stringify(request), JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:native-modules',
            abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('all authored module sources are inline owned resources'),
          async (json, bytes) => {
            const write = JSON.parse(json), token = `write:${writes.size}`;
            writes.set(token, { write, bytes: Uint8Array.from(bytes), committed: false });
            return JSON.stringify({ token });
          },
          async token => { const item = writes.get(token); item.committed = true; return JSON.stringify({ uri: item.write.uri }); },
          async () => assert.fail('successful native module graph must not roll back'),
        ));
        assert.equal(result.status, 'succeeded', JSON.stringify(result));
        assert.equal(result.artifacts.originalCount, 2);
        assert.equal(writes.size, 2);
        for (const [path, golden, source] of [
          ['out/asset.html', expectedMain, 'ui.cem'], ['out/stats.html', expectedStats, 'stats.cem'],
        ]) {
          const write = [...writes.values()].find(item => item.write.uri === `${base}${path}`);
          assert.equal(write?.committed, true);
          assert.equal(new TextDecoder().decode(write.bytes), golden.replaceAll('asset', kind ?? ''));
          const output = result.result.value.value.value.artifacts.find(item => item.destination === `${base}${path}`);
          assert.ok(output.outputSpans.flatMap(span => span.origin.frames).some(frame => frame.transform.uri === `${base}${source}`));
          assert.ok(output.outputSpans.every(span => span.outputRange.start + span.outputRange.len <= write.bytes.length));
          const handle = result.artifacts.items.find(item => item.uri === `${base}${path}`);
          assert.equal(handle.sha256, createHash('sha256').update(write.bytes).digest('hex'));
        }
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    }
  });
}

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
      options.hostConfigurationJson,
    ),
  );
}

test('owned browser bindings isolate memory and retire without retargeting aliases', async () => {
  const { createCemMlWasmGeneration } = await import('@epa-wg/cem-ml/wasm/factory');
  const wasm = await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url));
  const first = createCemMlWasmGeneration(), second = createCemMlWasmGeneration();
  await Promise.all([first.initialize(wasm), second.initialize(wasm)]);
  const firstMemory = first.memory(), secondMemory = second.memory();
  assert.ok(firstMemory instanceof WebAssembly.Memory);
  assert.ok(secondMemory instanceof WebAssembly.Memory);
  assert.notEqual(firstMemory, secondMemory);
  const alias = () => first.call('version', []);
  assert.equal(alias(), packageMetadata.version);
  assert.throws(() => first.dispose(() => false), /ownership-stale/);
  // Failed disposal releases only the original retirement permit; ordinary
  // admission remains closed, while the unrelated second heap stays usable.
  const confirmation = first.dispose();
  assert.equal(confirmation.frozen, true);
  assert.equal(first.memory(), undefined);
  assert.throws(alias, /generation.retired/);
  assert.equal(second.call('version', []), packageMetadata.version);
  second.dispose();
});


test('owned retirement preparation freezes native mutation and aborts only its original permit', async () => {
  const { createCemMlWasmGeneration } = await import('@epa-wg/cem-ml/wasm/factory');
  const factory = createCemMlWasmGeneration();
  await factory.initialize(await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)));
  factory.call('closeCommandNativeAdmission', []);
  const inventory = JSON.parse(factory.call('commandNativeResourceInventory', []));
  assert.throws(() => factory.prepareRetirement(inventory.revision + 1), /inventory-stale/);
  const confirmation = factory.prepareRetirement(inventory.revision);
  assert.equal(confirmation.inventoryRevision, inventory.revision);
  assert.throws(() => factory.prepareRetirement(inventory.revision), /retirement-pending/);
  factory.abortRetirement();
  assert.throws(() => factory.prepareRetirement(inventory.revision), /inventory-stale/);
  let current = JSON.parse(factory.call('commandNativeResourceInventory', [])).revision;
  factory.prepareRetirement(current);
  assert.throws(() => factory.dispose(() => false), /ownership-stale/);
  assert.ok(factory.memory() instanceof WebAssembly.Memory);
  current = JSON.parse(factory.call('commandNativeResourceInventory', [])).revision;
  factory.prepareRetirement(current);
  factory.dispose();
  assert.equal(factory.memory(), undefined);
});

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`mixed-quote inspection reimports exact literal source in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') {
      await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    }
    const literal = 'can\'t "quote" ```{#never} {$never()} ``` Café 🧬\r\n';
    const execute = async (source, contentType, show, format, sequence) => {
      const uri = `studio://mixed-quotes/source-${sequence}`;
      const bytes = new TextEncoder().encode(source);
      const version = { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') };
      const request = {
        protocolVersion: 1, requestId: `mixed-quotes:${runtimeKind}:${sequence}`,
        project: { projectId: 'mixed-quotes', revision: 1 }, resourceVersions: { [uri]: version },
        operation: { kind: 'inspect', inputId: 'input:0', show, ...(format ? { format } : {}) },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({
            inputs: [{ uri, rootScope: { defaultContentType: contentType } }],
            outputs: [{ destination: `${uri}.out`, rootScope: {
              defaultContentType: format === 'json' ? 'application/json' : 'application/cem',
            } }],
          })),
        ] }))), resources: { [uri]: { bytes: [...bytes], identity: { contentType } } },
        policyStamp: { resolver: 'literal-resolver', safety: 'literal-safety', budget: 'literal-budget' },
      };
      let published;
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(
          JSON.stringify(request), JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:mixed-quotes',
            abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('source is an inline owned resource'),
          async (json, output) => {
            assert.equal(published, undefined);
            published = { write: JSON.parse(json), bytes: Uint8Array.from(output) };
            return JSON.stringify({ token: 'inspection' });
          },
          async token => {
            assert.equal(token, 'inspection');
            return JSON.stringify({ uri: published.write.uri });
          },
          async () => assert.fail('successful inspection must not roll back'),
        ));
        assert.equal(result.status, 'succeeded', JSON.stringify(result));
        assert.equal(result.artifacts.originalCount, 1);
        assert.equal(result.artifacts.items[0].sha256, createHash('sha256').update(published.bytes).digest('hex'));
        return new TextDecoder().decode(published.bytes);
      } finally {
        runtime.disposeCommandArtifactsV1(request.requestId);
      }
    };
    for (const show of ['ast', 'tree']) {
      const cem = await execute(`<note><![CDATA[${literal}]]></note>`, 'application/xml', show, undefined, show);
      assert.ok(cem.includes(`\`\`\`\`\n${literal}\n\`\`\`\``), 'writer chooses a collision-free literal fence');
      const highlighted = JSON.parse(runtime.highlightSourceToHtmlV1(JSON.stringify({
        source: cem, contentType: 'application/cem',
      })));
      assert.equal(highlighted.status, 'highlighted');
      assert.deepEqual(highlighted.diagnostics, []);
      assert.ok(highlighted.spans.some(span => span.role === 'syntax.string'
        && new TextDecoder().decode(new TextEncoder().encode(cem).slice(span.byteOffset, span.byteOffset + span.byteLength)).includes(literal)));
      // Explicitly named JSON is a test-visible source export of native reimport,
      // never an AST handoff to an application or a replacement JavaScript parser.
      const exported = JSON.parse(await execute(cem, 'application/cem', 'ast', 'json', `${show}-reimport`));
      const pending = [exported];
      let matching = 0;
      while (pending.length) {
        const node = pending.pop();
        if (!node || typeof node !== 'object') continue;
        if (node.kind === 'attribute' && ['value', 'data'].includes(node.name) && node.value === literal) {
          matching++;
          assert.equal((node.children ?? []).length, 0, 'reference-looking payload stays literal');
        }
        pending.push(...Object.values(node));
      }
      assert.equal(matching, 1);
    }
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`structured data Parse identities and diagnostics in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    let sequence = 0;
    const execute = async (source, contentType, schema, operation) => {
      const uri = 'studio://structured-source/source';
      const bytes = new TextEncoder().encode(source);
      const version = { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') };
      const request = {
        protocolVersion: 1, requestId: `structured-source:${runtimeKind}:${++sequence}`,
        project: { projectId: 'structured-source', revision: 1 }, resourceVersions: { [uri]: version },
        operation: { ...operation, inputId: 'input:0' },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri, rootScope: { defaultContentType: contentType, schema } }],
            outputs: [{ destination: `${uri}.out`, rootScope: { defaultContentType: 'application/cem' } }] })),
        ] }))),
        resources: { [uri]: { bytes: [...bytes], identity: { contentType, schema } } },
        policyStamp: { resolver: 'data-resolver', safety: 'data-safety', budget: 'data-budget' },
      };
      let published;
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(JSON.stringify(request),
          JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:data', abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('inline owned resource must not fetch'),
          async (json, output) => { published = { write: JSON.parse(json), bytes: Uint8Array.from(output) }; return JSON.stringify({ token: 'data' }); },
          async () => JSON.stringify({ uri: published.write.uri }), async () => undefined,
        ));
        return { result, text: published ? new TextDecoder().decode(published.bytes) : undefined };
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    };
    for (const [source, contentType, schema, invalid, code] of [
      ['{"literal":"{#never} {$never()} Café 🧬"}', 'application/json', 'https://cem.dev/ns/data/json/1', '{\r\n"café":[1,}', 'cem.json.parse_error'],
      ["literal: '{#never} {$never()} Café 🧬'\r\n", 'text/x-yaml; charset=utf-8', 'https://cem.dev/ns/data/yaml/1', 'café: [1, 2\r\n', 'cem.yaml.parse_error'],
      ['{"$schema":"https://json-schema.org/draft/2020-12/schema","$defs":{"note":{"type":"string"}},"$ref":"#/$defs/note"}', 'application/json', 'https://cem.dev/ns/data/json-schema/1', '{\r\n"$defs":[}', 'cem.json_schema.parse_error'],
    ]) {
      for (const operation of [{ kind: 'parse', projection: 'ast', preserveSourceOffsets: true }, { kind: 'parse', projection: 'events', preserveSourceOffsets: true }, { kind: 'inspect', show: 'ast' }, { kind: 'inspect', show: 'diagnostics' }]) {
        const { result, text } = await execute(source, contentType, schema, operation);
        assert.equal(result.status, 'succeeded', JSON.stringify(result));
        assert.deepEqual(result.diagnostics.items, []);
        assert.ok(text.includes(schema) && text.includes(contentType), text);
        assert.ok(!text.includes('cem.tokenizer.') && !text.includes('cem.ql.'), text);
        if (operation.show !== 'diagnostics') assert.ok(text.includes('@byte-offset='), text);
      }
      const { result, text } = await execute(invalid, contentType, schema, { kind: 'parse', projection: 'ast', preserveSourceOffsets: true });
      assert.equal(result.exitCode, 1, JSON.stringify(result));
      assert.ok(result.diagnostics.items.some(d => d.code === code && d.severity === 'error'), JSON.stringify(result));
      assert.ok(!result.diagnostics.items.some(d => d.code.startsWith('cem.tokenizer.')));
      assert.equal(text, undefined, 'malformed source must not publish a partial tree');
    }
    for (const [source, contentType, schema] of [
      ["<catalog xmlns='urn:supplier'><name>🥝 {#inert}</name></catalog>", 'application/xml', 'https://cem.dev/ns/data/xml/1'],
      ["<svg xmlns='http://www.w3.org/2000/svg'><text>🥝 {#inert}</text></svg>", 'image/svg+xml', 'https://cem.dev/ns/data/svg/1'],
      ["<math xmlns='http://www.w3.org/1998/Math/MathML'><mi>x</mi></math>", 'application/mathml+xml', 'https://cem.dev/ns/data/mathml/1'],
    ]) {
      const parsed = await execute(source, contentType, schema, { kind: 'parse', projection: 'ast', preserveSourceOffsets: true });
      assert.equal(parsed.result.status, 'succeeded');
      const originalDiagnostics = parsed.result.diagnostics.items;
      if (contentType === 'image/svg+xml') {
        assert.equal(originalDiagnostics.length, 1);
        assert.equal(originalDiagnostics[0].code, 'cem.svg.accessible_name_missing');
        assert.equal(originalDiagnostics[0].severity, 'warning');
        assert.equal(originalDiagnostics[0].uri, 'studio://structured-source/source');
        assert.equal(originalDiagnostics[0].byteOffset, 0);
        assert.ok(originalDiagnostics[0].sourceMap.frames.length > 0);
      } else assert.deepEqual(originalDiagnostics, []);
      for (const show of ['ast', 'tree', 'events', 'summary', 'source-offsets', 'diagnostics']) {
        const { result, text } = await execute(source, contentType, schema, { kind: 'inspect', show });
        assert.equal(result.status, 'succeeded', JSON.stringify(result));
        assert.deepEqual(result.diagnostics.items, originalDiagnostics);
        assert.deepEqual(result.result.value.value.diagnostics, originalDiagnostics);
        assert.ok(text.includes(schema) && text.includes(contentType), text);
        assert.ok(!text.includes('cem.schema.unresolved_namespace'), text);
      }
    }
    const invalidXml = '<r>🥝\r\n<x></r>';
    const structural = await execute(invalidXml, 'application/xml', 'https://cem.dev/ns/data/xml/1', { kind: 'inspect', show: 'ast' });
    assert.equal(structural.result.status, 'failed');
    assert.equal(structural.text, undefined);
    assert.equal(structural.result.artifacts.originalCount, 0);
    const diagnostic = await execute(invalidXml, 'application/xml', 'https://cem.dev/ns/data/xml/1', { kind: 'inspect', show: 'diagnostics' });
    assert.equal(diagnostic.result.status, 'succeeded', 'diagnostic inspection is not validation');
    assert.deepEqual(diagnostic.result.diagnostics.items, structural.result.diagnostics.items);
    assert.deepEqual(diagnostic.result.result.value.value.diagnostics, structural.result.diagnostics.items);
    assert.ok(diagnostic.result.diagnostics.items.some(d => d.code === 'cem.xml.parse_error'
      && d.uri === 'studio://structured-source/source' && d.line === 2));
    assert.ok(diagnostic.text.includes('cem.xml.parse_error'));
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`CEMT package palettes and writer encodings stay separate in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    const source = '@doc cem-ml 1\n{main @title="Café & <tag>" | <script>literal</script> 🧬}\n';
    const uri = 'studio://profile-boundary/source.cem', bytes = new TextEncoder().encode(source);
    const version = { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') };
    let sequence = 0;
    const execute = async (profile, outputColorType) => {
      const request = {
        protocolVersion: 1, requestId: `profile-boundary:${runtimeKind}:${++sequence}`,
        project: { projectId: 'profile-boundary', revision: 1 }, resourceVersions: { [uri]: version },
        operation: { kind: 'convert', inputId: 'input:0', toFormat: 'cem', preserveSourceOffsets: true },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({
            inputs: [{ uri, rootScope: { defaultContentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1' } }],
            outputs: [{ destination: `${uri}.out`, rootScope: { defaultContentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1',
              cemtFormatterProfile: 'tabular', cemtColorProfile: profile, ...(outputColorType ? { outputColorType } : {}) } }],
          })),
        ] }))),
        resources: { [uri]: { bytes: [...bytes], identity: { contentType: 'application/cem', schema: 'https://cem.dev/ns/cem-ml/1' } } },
        policyStamp: { resolver: 'profile-resolver', safety: 'profile-safety', budget: 'profile-budget' },
      };
      let published;
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(JSON.stringify(request),
          JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:profiles', abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('owned inline source must not fetch'),
          async (json, output) => { assert.equal(published, undefined); published = Uint8Array.from(output); return JSON.stringify({ token: 'profile' }); },
          async () => JSON.stringify({ uri: `${uri}.out` }), async () => assert.fail('valid conversion must not roll back'),
        ));
        assert.equal(result.status, 'succeeded', JSON.stringify(result));
        assert.deepEqual(result.diagnostics.items, []);
        const artifact = result.artifacts.items[0];
        assert.equal(result.artifacts.originalCount, 1);
        assert.equal(artifact.sha256, createHash('sha256').update(published).digest('hex'));
        return { text: new TextDecoder().decode(published), contentType: artifact.contentType };
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    };
    const plain = await execute('none', 'none');
    assert.equal(plain.contentType, 'application/cem');
    for (const profile of ['classes', 'inline-style', 'css-custom-properties']) {
      assert.deepEqual(await execute(profile), plain, `${profile} is a package selector`);
    }
    assert.deepEqual(await execute('html', 'none'), plain, 'explicit none wins over the legacy preview profile');
    for (const selector of ['html-classes', 'classes', 'css-vars', 'inline-style']) {
      const output = await execute('classes', selector);
      assert.equal(output.contentType, 'text/html');
      assert.ok(output.text.startsWith('<pre') && output.text.endsWith('</pre>'), selector);
      assert.ok(output.text.includes('data-role='));
      assert.ok(output.text.includes('&lt;script&gt;') && !output.text.includes('<script>'));
      assert.ok(!output.text.includes('\u001b'));
    }
    const terminal = await execute('terminal', 'ansi-256');
    assert.equal(terminal.contentType, 'application/cem');
    assert.ok(terminal.text.includes('\u001b['));
    const ansiSequence = new RegExp(String.fromCodePoint(27) + String.raw`\[[0-?]*[ -/]*[@-~]`, 'g');
    assert.equal(terminal.text.replace(ansiSequence, ''), plain.text);
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`Login element-template validation parity in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    const source = await readFile(new URL('../../../examples/projects/login/views/login.cem', import.meta.url), 'utf8');
    const uri = 'studio://login-validation/views/login.cem';
    const contentType = 'application/vnd.cem.element-template+cem', schema = 'https://cem.dev/ns/template/cem-element/1';
    let sequence = 0;
    const execute = async (text, kind = 'validate') => {
      const bytes = new TextEncoder().encode(text);
      const request = {
        protocolVersion: 1, requestId: `login-validation:${runtimeKind}:${++sequence}`,
        project: { projectId: 'login-validation', revision: 1 },
        resourceVersions: { [uri]: { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') } },
        operation: { kind, inputIds: ['input:0'], projection: 'json', ...(kind === 'check' ? { zeroHardViolations: true } : {}) },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri, rootScope: { defaultContentType: contentType, schema } }], outputs: [] })),
        ] }))),
        resources: { [uri]: { bytes: [...bytes], identity: { contentType, schema } } },
        policyStamp: { resolver: 'login-resolver', safety: 'login-safety', budget: 'login-budget' },
      };
      let publication;
      try {
        return JSON.parse(await runtime.executeCommandServiceV1(JSON.stringify(request),
          JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:login', abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('validation must not fetch Login data or send credentials'),
          async (json) => { publication = JSON.parse(json); return JSON.stringify({ token: 'validation' }); },
          async () => JSON.stringify({ uri: publication.uri }), async () => undefined,
        ));
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    };
    const valid = await execute(source);
    assert.equal(valid.exitCode, 0, JSON.stringify(valid));
    assert.equal(valid.diagnostics.items.length, 15);
    assert.ok(valid.diagnostics.items.every(d => d.severity === 'warning'));
    const broken = source.replace('dom:text(heading)', 'dom:text(not_declared)');
    assert.notEqual(broken, source);
    for (const kind of ['validate', 'check']) {
      const result = await execute(broken, kind);
      assert.equal(result.status, 'failed', JSON.stringify(result));
      assert.equal(result.exitCode, 1);
      const errors = result.diagnostics.items.filter(d => ['error', 'fatal'].includes(d.severity));
      assert.equal(errors.length, 1);
      assert.equal(errors[0].code, 'cem.ql.render.compile_failed');
      assert.equal(errors[0].uri, uri);
      assert.equal(errors[0].byteOffset, 655);
      assert.equal(errors[0].line ?? null, null);
      assert.equal(errors[0].column ?? null, null);
      assert.ok(errors[0].message.includes('not_declared'));
      assert.ok(errors[0].sourceMap.frames.length > 0);
      assert.equal(result.diagnostics.items.filter(d => d.severity === 'warning').length, 15);
    }
    const malformed = await execute(source.slice(0, -2));
    assert.equal(malformed.exitCode, 1);
    assert.ok(malformed.diagnostics.items.some(d => d.code === 'cem.tokenizer.unterminated_node' && d.uri === uri));
    const repaired = await execute(source);
    assert.equal(repaired.exitCode, 0);
    assert.deepEqual(repaired.diagnostics.items, valid.diagnostics.items);
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`typed control source operations remain inert in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    const root = new URL('../../../examples/projects/portable-config/', import.meta.url);
    const cases = [
      ['authored/build.command.json', 'application/vnd.cem.cli-command+json', 'https://cem.dev/ns/cli/command/1', 'argv'],
      ['authored/check.run.json', 'application/json', 'https://cem.dev/ns/cli/run-config/1', 'inputs'],
      ['project/card.project.json', 'application/vnd.cem.studio-project+json', 'https://cem.dev/ns/studio/project/1', 'dispatch-card'],
      ['project/card.project.cem', 'application/vnd.cem.studio-project+cem', 'https://cem.dev/ns/studio/project/1', 'dispatch-card'],
    ];
    let sequence = 0;
    const execute = async (text, contentType, schema, kind) => {
      const uri = 'studio://control-sources/original.control', bytes = new TextEncoder().encode(text);
      const operation = ['validate', 'check'].includes(kind)
        ? { kind, inputIds: ['input:0'], projection: 'json', ...(kind === 'check' ? { zeroHardViolations: true } : {}) }
        : { kind, inputId: 'input:0', ...(kind === 'parse' ? { projection: 'ast', preserveSourceOffsets: true }
          : kind === 'inspect' ? { show: 'ast' } : { toFormat: contentType.endsWith('+cem') ? 'cem' : 'json', preserveSourceOffsets: true }) };
      const request = {
        protocolVersion: 1, requestId: `control-source:${runtimeKind}:${++sequence}`,
        project: { projectId: 'control-sources', revision: 1 },
        resourceVersions: { [uri]: { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') } }, operation,
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri, rootScope: { defaultContentType: contentType, schema } }],
            outputs: ['parse', 'inspect', 'convert'].includes(kind) ? [{ destination: `${uri}.out`, rootScope: kind === 'convert'
              ? { defaultContentType: contentType, schema, outputColorType: 'none' }
              : { defaultContentType: 'application/cem', outputColorType: 'none' } }] : [] })),
        ] }))), resources: { [uri]: { bytes: [...bytes], identity: { contentType, schema } } },
        policyStamp: { resolver: 'control-resolver', safety: 'control-safety', budget: 'control-budget' },
      };
      let published;
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(JSON.stringify(request),
          JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:controls', abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('inert control source operations must not execute argv, read resources or transport credentials'),
          async (json, output) => { published = { write: JSON.parse(json), bytes: Uint8Array.from(output) }; return JSON.stringify({ token: 'control-output' }); },
          async () => JSON.stringify({ uri: published.write.uri }), async () => undefined,
        ));
        const artifact = result.artifacts.items.find(item => item.kind === 'output');
        const output = published?.bytes ?? (artifact
          ? runtime.readCommandArtifactV1(request.requestId, artifact.handleId, 0, artifact.byteLength).bytes : undefined);
        return { result, text: output ? new TextDecoder().decode(output) : undefined };
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    };
    for (const [path, mime, schema, expected] of cases) {
      const source = await readFile(new URL(path, root), 'utf8');
      for (const kind of ['parse', 'inspect', 'validate', 'check', 'convert']) {
        const outcome = await execute(source, mime, schema, kind);
        assert.equal(outcome.result.exitCode, 0, JSON.stringify(outcome.result));
        assert.deepEqual(outcome.result.diagnostics.items, []);
        if (['parse', 'inspect'].includes(kind)) {
          assert.ok(outcome.text.includes(schema) && outcome.text.includes(mime) && outcome.text.includes(expected), outcome.text);
          assert.ok(outcome.text.includes('studio://control-sources/original.control'));
        }
        if (kind === 'convert') {
          assert.ok(outcome.text.includes(expected), outcome.text);
          assert.equal(outcome.result.artifacts.items[0].contentType, mime);
        }
      }
      const invalid = mime === 'application/json' ? await readFile(new URL('failures/duplicate.run.json', root), 'utf8')
        : mime.endsWith('+cem') ? source.replace('@schema-version=1', '@schema-version=2') : source.replace('"schemaVersion": 1', '"schemaVersion": 2');
      const code = mime === 'application/json' ? 'cem.run_config.input_uri_duplicate'
        : mime.includes('cli-command') ? 'cem.cli_command.schema_version_unsupported' : 'cem.studio_project.schema_version_unsupported';
      for (const kind of ['parse', 'inspect', 'validate', 'check', 'convert']) {
        const { result } = await execute(invalid, mime, schema, kind);
        assert.equal(result.exitCode, 1, JSON.stringify(result));
        assert.ok(result.diagnostics.items.some(d => d.code === code && d.uri === 'studio://control-sources/original.control'));
        assert.ok(!result.diagnostics.items.some(d => d.code.startsWith('cem.tokenizer.') || d.code.startsWith('cem.schema.unknown_html')));
      }
      assert.equal((await execute(source, mime, schema, 'parse')).result.exitCode, 0);
    }
  });
}

for (const runtimeKind of ['wasm-node', 'wasm-browser-worker']) {
  test(`versioned module maps and semantic AST exports share native contracts in ${runtimeKind}`, async () => {
    const runtime = runtimeKind === 'wasm-node'
      ? await import('@epa-wg/cem-ml/wasm') : await import('../dist/wasm/browser/cem_ml.js');
    if (runtimeKind !== 'wasm-node') await runtime.default({ module_or_path: await readFile(new URL('../dist/wasm/browser/cem_ml_bg.wasm', import.meta.url)) });
    let sequence = 0;
    const execute = async (text, contentType, schema, convert = false) => {
      const uri = 'studio://source-contracts/original.source', bytes = new TextEncoder().encode(text);
      const request = {
        protocolVersion: 1, requestId: `source-contracts:${runtimeKind}:${++sequence}`,
        project: { projectId: 'source-contracts', revision: 1 },
        resourceVersions: { [uri]: { revision: 1, sha256: createHash('sha256').update(bytes).digest('hex') } },
        operation: convert ? { kind: 'convert', inputId: 'input:0', toFormat: 'ast', preserveSourceOffsets: true }
          : { kind: 'validate', inputIds: ['input:0'], projection: 'json' },
        runPlan: JSON.parse(runtime.normalizeCommandRunPlanV1(JSON.stringify({ configBytes: [
          ...new TextEncoder().encode(JSON.stringify({ inputs: [{ uri, rootScope: { defaultContentType: contentType, schema } }],
            outputs: convert ? [{ destination: `${uri}.ast.json`, rootScope: { defaultContentType: 'application/vnd.cem.ast+json', schema: 'https://cem.dev/ns/projection/ast/1' } }] : [] })),
        ] }))),
        resources: { [uri]: { bytes: [...bytes], identity: { contentType, schema } } },
        policyStamp: { resolver: 'source-resolver', safety: 'source-safety', budget: 'source-budget' },
      };
      let published;
      try {
        const result = JSON.parse(await runtime.executeCommandServiceV1(JSON.stringify(request),
          JSON.stringify({ runtime: runtimeKind, targetIdentity: 'runtime-test:source-contracts', abiIdentity: 'runtime-test-v1', debugControlActive: false }),
          async () => JSON.stringify({ project: request.project, resourceVersions: request.resourceVersions }),
          async () => assert.fail('source contracts cannot read mapped assets, execute commands or transport credentials'),
          async (json, output) => { published = Uint8Array.from(output); return JSON.stringify({ token: 'source-output' }); },
          async () => JSON.stringify({ uri: `${uri}.ast.json` }), async () => undefined,
        ));
        return { result, text: published && new TextDecoder().decode(published) };
      } finally { runtime.disposeCommandArtifactsV1(request.requestId); }
    };
    for (const version of [1, 2, 3]) {
      const schema = `https://cem.dev/ns/data/module-map/${version}`;
      const root = new URL(`../../../examples/projects/module-maps/`, import.meta.url);
      const valid = await readFile(new URL(`maps/source-v${version}.json`, root), 'utf8');
      const invalid = await readFile(new URL(`failures/semantic-v${version}.json`, root), 'utf8');
      assert.equal((await execute(valid, 'application/vnd.cem.module-map+json', schema)).result.exitCode, 0);
      const refused = (await execute(invalid, 'application/vnd.cem.module-map+json', schema)).result;
      assert.equal(refused.exitCode, 1);
      const diagnostic = refused.diagnostics.items.find(d => d.code === 'cem.module_map.specifier_invalid');
      assert.equal(diagnostic.uri, 'studio://source-contracts/original.source');
      assert.equal(diagnostic.byteOffset, new TextEncoder().encode(invalid.slice(0, invalid.indexOf('"./invalid"'))).length);
      assert.ok(diagnostic.sourceMap.frames.length > 0);
      assert.equal((await execute(valid.replace(schema, 'https://cem.dev/ns/data/module-map/4'), 'application/vnd.cem.module-map+json', schema)).result.exitCode, 1);
      assert.equal((await execute(valid, 'application/vnd.cem.module-map+json', schema)).result.exitCode, 0);
    }
    const ast = await execute('{p @title="Café" | Ready}', 'application/cem', 'https://cem.dev/ns/cem-ml/1', true);
    assert.equal(ast.result.exitCode, 0, JSON.stringify(ast.result));
    const document = JSON.parse(ast.text);
    assert.equal(document.kind, 'document'); assert.ok(Array.isArray(document.children));
    assert.ok(document.children.find(n => n.kind === 'element').children.some(n => n.kind === 'text' && n.data === 'Ready'));
    assert.equal((await execute(ast.text, 'application/vnd.cem.ast+json', 'https://cem.dev/ns/projection/ast/1')).result.exitCode, 0);
    assert.equal((await execute(JSON.stringify(document.children), 'application/vnd.cem.ast+json', 'https://cem.dev/ns/projection/ast/1')).result.exitCode, 1);
  });
}
