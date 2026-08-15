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
import { createBrowserCommandServiceClient } from '@epa-wg/cem-ml-cli/browser';

const client = await createBrowserCommandServiceClient({
    host: {
        currentRevision: async ({ project }) => ({ project, resourceVersions: {} }),
        readResource: async (request) => resolveApplicationResource(request),
        prepareWrite: async (request, bytes) => stageApplicationWrite(request, bytes),
        commitWrite: async (token) => commitApplicationWrite(token),
        rollbackWrite: async (token) => rollbackApplicationWrite(token),
    },
});
const operation = client.execute(commandServiceRequest, {
    signal: abortController.signal,
    onProgress: (progress) => console.log(progress),
});
const result = await operation;
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

`pool.run(request)` returns an awaitable operation handle with `result()`,
`subscribe()`, `cancel()`, `pause()`, `continue()`, `step()`, and `dispose()`.
The common Rust coordinator retains continuation and deterministic commit order;
workers receive bounded stateless transform/query packets. Dedicated workers are
terminated and replaced when cancellation exceeds the negotiated hard-cancel
grace. Main-thread browser fallback uses the same bounded packets and cooperative
controls, while truthfully reporting hard cancellation as unavailable.

Mapping parsed commands into the Node resolver/report adapters and the `cem-ml`
npm executable remains subsequent checklist work.
