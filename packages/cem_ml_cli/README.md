# cem-ml-cli

`cem-ml-cli` is the native Rust command host for the `cem-ml` binary. It applies
command-line argument, stream, exit-code, cancellation, and host-I/O policy to
the reusable services owned by the `cem-ml` library.

## Public boundary

The crate publishes the native `cem-ml` executable. Parsing, validation,
conversion, query, transform, report, and transformation-graph semantics remain
in [`cem-ml`](../cem_ml/README.md); the CEM-QL template adapter is supplied by
[`cem-ml-transform-cem-ql`](../cem_ml_transform_cem_ql/README.md).

`@epa-wg/cem-ml-cli` is a separate synchronized npm deployment for Node and
browser hosts. It does not replace this native crate or change the command
contract.

## Verification

Use Nx for native build and command verification:

```bash
yarn nx run cem_ml_cli:build
yarn nx run cem_ml_cli:test
yarn nx run cem_ml_cli:e2e
```

See the [CLI feature summary](../../docs/cem-ml-cli-contract.md) for the command
surface and the
[deployment contract](../../docs/cem-ml-deployment-contract.md) for synchronized
runtime and host ownership.
