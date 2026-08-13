# `@epa-wg/cem-ml-cli`

Universal host adapters for the common `@epa-wg/cem-ml` WebAssembly runtime.
The current Phase 2.5 slice exposes bounded browser dedicated-worker and Node
worker-thread hosts.

```js
import { createNodeWorkerPool } from '@epa-wg/cem-ml-cli/node';

const pool = await createNodeWorkerPool({ workerCount: 2 });
console.log(pool.capability, pool.workers);
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
require shared-memory WASM or cross-origin isolation. Operation dispatch,
hard-cancel replacement, shared commands, and the `cem-ml` npm executable remain
subsequent checklist work.
