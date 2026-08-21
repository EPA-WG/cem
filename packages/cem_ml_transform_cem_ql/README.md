# cem-ml-transform-cem-ql

`cem-ml-transform-cem-ql` is the Rust adapter that connects CEM-ML's stable
transform-template contract to CEM-QL compilation, evaluation, and native CEMT
rendering.

## Public boundary

The crate sits above both [`cem-ml`](../cem_ml/README.md) and
[`cem-ql`](../cem_ql/README.md). CEM-ML retains transformation lifecycle,
artifact, schema, source-map, and adapter ownership; CEM-QL retains expression,
template, render-plan, and compiled-artifact ownership. Keeping the integration
in this crate prevents a dependency cycle and gives native hosts one explicit
adapter registration boundary.

The adapter supports CEM-native templates, standalone CEM-QL expression
templates, and the bounded legacy XSLT-parity lane. It is infrastructure for
hosts and embedders, not an application UI or an alternate query-language
implementation.

## Verification

Use Nx for the publishable crate gates:

```bash
yarn nx run cem_ml_transform_cem_ql:lint
yarn nx run cem_ml_transform_cem_ql:test
yarn nx run cem_ml_transform_cem_ql:build
```

The surrounding transformation contract is documented in the
[CEM-ML acceptance criteria](../../docs/cem-ml-ac.md) and the
[CEM-QL implementation design](../../docs/cem-ql-stack-design-impl.md).
