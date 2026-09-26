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

## CEM-QL query modules

Use `--query-content-type application/vnd.cem.query-expression+cem-ql` for a
standalone expression, or `application/vnd.cem.query+cem-ql` (`text/cem-ql`
alias) for a module. An omitted query schema is inferred; an explicit schema
must match the source kind. Module identities require a `module "URI"` header
and one final expression. Both inline `--query` and `--query-file` are supported.

```sh
cem-ml query catalog.xml --content-type application/xml \
  --query 'module "urn:example:links" import "cem:stdlib/url" as u u:href("https://example.test")' \
  --query-content-type application/vnd.cem.query+cem-ql --output json
```

Modules support local immutable bindings/functions and registered `cem:`
imports. External/plugin imports are rejected. `input` retains the native data
tree, and local declarations follow lexical shadowing rules. Ordered nonfatal
warnings reach the query result and report. See the
[module-query contract](../../docs/cem-ql-cli-module-query-design.md).
