# `@epa-wg/cem-ml-cli`

Universal host adapters for the common `@epa-wg/cem-ml` WebAssembly runtime.
The current Phase 2.5 slice exposes bounded browser dedicated-worker and Node
worker-thread hosts plus one generated, versioned command grammar shared by
both APIs.

```js
import {
    commandSchema,
    parseCemMlCommand,
    serializeCemMlCommand,
} from '@epa-wg/cem-ml-cli/node';

const command = parseCemMlCommand(
    ['query', 'data.xml', '--query', '//item', '--query-content-type', 'application/vnd.cem.xpath'],
    { runtime: 'wasm-node' },
);
console.log(commandSchema.schemaVersion, command);
console.log(serializeCemMlCommand(command));
```

The schema is generated from the built native Clap command graph and joined to
the common Rust runtime-capability matrix. Options, defaults, enum values,
required groups, conflicts, and runtime availability are therefore not copied
into a separate TypeScript flag table. The same parser and serializer are
exported from `./browser` and `./node`.

```js
import { createNodeWorkerPool } from '@epa-wg/cem-ml-cli/node';

const source = (uri, contentType, schema, text) => ({
    uri,
    bytes: [...new TextEncoder().encode(text)],
    identity: { contentType, schema },
});
const xmlSource = source(
    'memory:data.xml',
    'application/xml',
    'https://cem.dev/ns/data/xml/1',
    '<root><item id="one"/><item id="two"/></root>',
);
const xpathSource = source(
    'memory:query.xpath',
    'application/vnd.cem.xpath',
    'https://cem.dev/ns/query/xpath/1',
    '//item',
);
const pool = await createNodeWorkerPool({ workerCount: 2 });
console.log(pool.capability, pool.workers);
const operation = pool.run({
    kind: 'query',
    data: xmlSource,
    query: xpathSource,
});
operation.subscribe((event) => console.log(event));
console.log(await operation);
await pool.close();
```

```js
import { createBrowserWorkerPool } from '@epa-wg/cem-ml-cli/browser';

const pool = await createBrowserWorkerPool({ workerCount: 2 });
console.log(pool.mode, pool.capability, pool.workers);
await pool.close();
```

Every worker owns one isolated WASM runtime. Stable slot/generation identities,
strict initialization envelopes, Rust-derived protocol limits, exact runtime
versioning, bounded startup, and explicit one-worker modes are established here.
The browser host falls back from the bounded pool to one dedicated worker and
then to one main-thread WASM runtime when workers cannot initialize; it does not
require shared-memory WASM or cross-origin isolation.

The browser command-service API is a separate strict boundary: it always owns
exactly one dedicated worker and never falls back to pool scheduling or
main-thread execution. Callers provide explicit asynchronous revision, read,
and transactional-write capabilities; URLs and virtual resources have no
implicit filesystem behavior.

```js
import {
    buildBrowserCommandInvocation,
    createBrowserCommandServiceClient,
    parseCemMlCommand,
    projectBrowserCommandPresentation,
} from '@epa-wg/cem-ml-cli/browser';

const parsed = parseCemMlCommand(['parse', 'document.cem'], {
    runtime: 'wasm-browser-worker',
});
const invocation = await buildBrowserCommandInvocation(
    parsed,
    async ({ uri }) => [{ uri, bytes: await readApplicationBytes(uri) }],
    { cwd: '/workspace' },
);

const client = await createBrowserCommandServiceClient({
    host: {
        currentRevision: async ({ project }) => ({ project, resourceVersions: {} }),
        readResource: async (request) => resolveApplicationResource(request),
        prepareWrite: async (request, bytes) => stageApplicationWrite(request, bytes),
        commitWrite: async (token) => commitApplicationWrite(token),
        rollbackWrite: async (token) => rollbackApplicationWrite(token),
    },
});
const operation = client.execute(invocation.request, {
    signal: abortController.signal,
    onProgress: (progress) => console.log(progress),
});
const result = await operation;
const presentation = projectBrowserCommandPresentation(invocation.presentation, result);
consumePresentation(presentation);
for (const artifact of result.artifacts.items) {
    const { metadata, bytes } = await operation.readArtifact(artifact);
    consumeCopiedArtifactChunk(metadata, bytes);
}
await operation.dispose();
await client.close();
```

The request, result, callback, progress, cancellation, and artifact types are
re-exported directly from the Rust-generated `@epa-wg/cem-ml/wasm`
declarations. Pre-terminal command errors reject with
`BrowserCommandServiceError`; canonical terminal statuses remain typed results.

The Node command service adds explicit filesystem, local `file://`, HTTPS, and
application-stream resolution plus prepared file/stream writes. Parsed command
lowering, resource discovery, canonical requests, diagnostics, reports,
terminal presentation, and exit codes remain owned by the common Rust runtime.

```js
import {
    createNodeCommandHost,
    createNodeCommandService,
    parseCemMlCommand,
} from '@epa-wg/cem-ml-cli/node';

const host = createNodeCommandHost({ stdout: process.stdout, stderr: process.stderr });
const service = await createNodeCommandService({ host });
try {
    const command = parseCemMlCommand(['parse', 'document.cem'], { runtime: 'wasm-node' });
    const { invocation, handle } = await service.run(command);
    const result = await handle;
    await service.publish(invocation, result);
    await handle.dispose();
} finally {
    await service.close();
}
```

The package also installs `cem-ml`. It uses one worker-thread runtime, maps
`SIGINT`/`SIGTERM` to cooperative cancellation, publishes stdout/stderr and
report files through the Node host, and preserves the stable common exit policy.

`pool.run(request)` returns an awaitable operation handle with `result()`,
`subscribe()`, `cancel()`, `pause()`, `continue()`, `step()`, and `dispose()`.
The common Rust coordinator retains continuation and deterministic commit order;
workers receive bounded stateless transform/query packets. Dedicated workers are
terminated and replaced when cancellation exceeds the negotiated hard-cancel
grace. Main-thread browser fallback uses the same bounded packets and cooperative
controls, while truthfully reporting hard cancellation as unavailable.

The aggregate package checks run the shared parse, validate, check, inspect,
convert, query, transform, trace, and version/capabilities matrix through the
browser and Node command-service workers and the executable. Packaging then
installs both archives into a clean consumer, proves one resolved common runtime,
and repeats the executable matrix through the installed `cem-ml` bin.

The package build writes its runtime/capability projection and complete
SHA-256 integrity manifest. Its Nx `package` target adds the version-qualified
npm tarball, SPDX 2.3 SBOM, provenance, checksums, signing state, and release
index entry, then validates those artifacts against the common platform
contract before returning success.

The uncached Nx `sign` target verifies and records a supplied
`CEM_ML_GITHUB_ATTESTATION_BUNDLE`; `CEM_ML_RELEASE_SIGNING=required` turns a
missing bundle into a hard release failure. Unsigned development packages
remain explicitly non-publishable.
