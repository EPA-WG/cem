# cem-ql

`cem-ql` is the Rust compiler and evaluator for the CEM query language used by
schema behaviors, CEMT templates, validation, component bindings, and explicit
standalone query execution.

## Public boundary

The crate owns CEM-QL lexing, parsing, name resolution, type checking, IR,
standard-library modules, evaluation budgets, compiled artifacts, templates,
transport, and the WebAssembly boundary. It consumes CEM-ML data and projection
owners without moving general parser or lifecycle semantics out of `cem-ml`.

The public expression identity is
`application/vnd.cem.query-expression+cem-ql` with schema
`https://cem.dev/ns/query/cem-ql/1#expression`. CEM-native transform rendering
is integrated through the separate
[`cem-ml-transform-cem-ql`](../cem_ml_transform_cem_ql/README.md) adapter crate.

## Verification

Use the cached Nx targets for the native and WASM surfaces:

```bash
yarn nx run cem_ql:lint
yarn nx run cem_ql:test
yarn nx run cem_ql:build:wasm
```

See the [CEM-QL acceptance criteria](../../docs/cem-ql-ac.md),
[stack design](../../docs/cem-ql-stack-design.md), and
[implementation design](../../docs/cem-ql-stack-design-impl.md) for the complete
language and runtime contract.
