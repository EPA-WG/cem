# `@epa-wg/cem-ml-cli`

Universal host adapters for the common `@epa-wg/cem-ml` WebAssembly runtime.
The current Phase 2.5 slice exposes the bounded Node worker-thread host through
`@epa-wg/cem-ml-cli/node`.

```js
import { createNodeWorkerPool } from '@epa-wg/cem-ml-cli/node';

const pool = await createNodeWorkerPool({ workerCount: 2 });
console.log(pool.capability, pool.workers);
await pool.close();
```

Every worker owns one isolated WASM runtime. Stable slot/generation identities,
strict initialization envelopes, Rust-derived protocol limits, exact runtime
versioning, bounded startup, and explicit one-worker mode are established here.
Browser workers, operation dispatch, hard-cancel replacement, and the `cem-ml`
npm executable remain subsequent checklist work.
