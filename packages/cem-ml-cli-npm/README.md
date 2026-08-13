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

`pool.run(request)` returns an awaitable operation handle with `result()`,
`subscribe()`, `cancel()`, `pause()`, `continue()`, `step()`, and `dispose()`.
The common Rust coordinator retains continuation and deterministic commit order;
workers receive bounded stateless transform/query packets. Dedicated workers are
terminated and replaced when cancellation exceeds the negotiated hard-cancel
grace. Main-thread browser fallback uses the same bounded packets and cooperative
controls, while truthfully reporting hard cancellation as unavailable.

Mapping parsed commands into the complete common operation service, Node and
browser resolver/report adapters, and the `cem-ml` npm executable remain
subsequent checklist work.
