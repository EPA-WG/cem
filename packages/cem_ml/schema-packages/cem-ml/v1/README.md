# CEM-ML Generic Schema Package

Status: initial source package

[cem-ml-syntax.md](../../../../../docs/cem-ml-syntax.md)

This package is the first schema-package source for the generic CEM-ML document
model. It owns CEM-ML syntax and content-type identity, not domain semantics.

Owned schema URI:

```text
https://cem.dev/ns/cem-ml/1
```

Primary content type:

```text
application/cem
```

Current aliases mirror the Rust runtime's existing accepted CEM source content
types:

- `text/cem-ml`
- `text/cem`
- `application/cem+xml`

The semantic CEM annotation vocabulary remains in `packages/cem_ml/schema/cem-core.md`
under `https://cem.dev/ns/core/1`.

## CEMT Output Assets

Schema-local output transformations live beside the schema package:

- [`formatters/cem-format-tree.cemt`](formatters/cem-format-tree.cemt)
  declares `cem.format-tree` for `application/cem` /
  `https://cem.dev/ns/cem-ml/1`.
- [`colorizers/cem-color-tree.cemt`](colorizers/cem-color-tree.cemt)
  declares `cem.color-tree` for formatted CEM trees before the writer phase.

The package manifest declares these files as `formatter` and `colorizer`
artifacts so runtime CEMT stages are tied to schema-owned assets instead of
inline Rust template strings.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic.cem`](examples/basic.cem) | Minimal persisted CEM-ML document. | Pass |
| [`nested-handoff.cem`](examples/nested-handoff.cem) | Namespaced content with a `text/html` handoff boundary. | Pass |
| [`invalid-unclosed-scope.cem`](examples/invalid-unclosed-scope.cem) | Missing closing scope syntax diagnostic. | Fail with `cem.schema.unclosed_scope` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/cem \
  --schema https://cem.dev/ns/cem-ml/1 \
  packages/cem_ml/schema-packages/cem-ml/v1/examples/basic.cem
```

The validation harness also runs the same command shape for the invalid
example and expects a hard validation failure.
