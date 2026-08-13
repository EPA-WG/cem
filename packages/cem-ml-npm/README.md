# `@epa-wg/cem-ml`

Low-level browser and Node WebAssembly deployment of the common Rust `cem_ml`
engine. This package intentionally owns no command parser, filesystem policy,
npm executable, or UI state.

Browser code imports the generated web loader:

```js
import init, { version } from '@epa-wg/cem-ml/wasm';

await init();
console.log(version());
```

Node hosts import the generated Node loader from the same subpath:

```js
import * as cemMl from '@epa-wg/cem-ml/wasm';

console.log(cemMl.version());
```

`runtime.json` describes the package ABI and common capability projections.
`integrity.json` records SHA-256 hashes for every generated runtime and
schema-package asset. Schema assets are addressable below
`@epa-wg/cem-ml/schema-packages/`.
