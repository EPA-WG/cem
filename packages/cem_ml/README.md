# cem-ml

`cem-ml` is the Rust library that owns CEM's schema-defined parsing, validation,
conversion, query, transformation, reporting, scheduling, source-map, and
operation-control semantics. Its version is the authority shared by the native
CLI and synchronized npm/WASM deployment packages.

## Public boundary

The crate exposes reusable library APIs and a WebAssembly-compatible `cdylib`.
It owns content-type and schema identities, typed lifecycle artifacts, command
services, transformation graphs, and portable diagnostics. It does not own CLI
argument policy, browser UI, component behavior, or package-specific deployment
wrappers.

The low-level npm/WASM deployment is published separately as
[`@epa-wg/cem-ml`](../cem-ml-npm/README.md). The native command host is the
[`cem-ml-cli`](../cem_ml_cli/README.md) crate, while CEM-QL evaluation is owned by
[`cem-ql`](../cem_ql/README.md).

## Verification

Use Nx as the workspace task authority:

```bash
yarn nx run cem_ml:lint
yarn nx run cem_ml:test
yarn nx run cem_ml:build:wasm
```

The acceptance criteria are documented in
[`docs/cem-ml-ac.md`](../../docs/cem-ml-ac.md), and synchronized distribution
ownership is defined in
[`docs/cem-ml-deployment-contract.md`](../../docs/cem-ml-deployment-contract.md).
