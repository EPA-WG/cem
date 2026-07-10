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
- [`formatters/cem-format-tree-helpers.cemt`](formatters/cem-format-tree-helpers.cemt)
  declares private canonical formatter helpers used by `cem.format-tree`,
  including the private output-stage wrapper `cem.format-tree.apply-stage`
  and internal CEMT functions `cem.format-tree.build-nodes` and
  `cem.format-tree.build-envelope`.
- [`formatters/formatter-coloring-pipeline.cemt`](formatters/formatter-coloring-pipeline.cemt)
  declares `acme.showcase.format-tree` as a package-qualified formatter that
  extends `cem.format-tree`.
- [`formatters/cem-tree-helpers.cemt`](formatters/cem-tree-helpers.cemt)
  declares private `cemml.cem-tree.*` formatter helpers used by schema-local
  formatter entrypoints.
- [`colorizers/cem-color-tree.cemt`](colorizers/cem-color-tree.cemt)
  declares `cem.color-tree` for formatted CEM trees before the writer phase.
- [`colorizers/cem-color-tree-helpers.cemt`](colorizers/cem-color-tree-helpers.cemt)
  declares private canonical colorizer helpers used by `cem.color-tree`.
- [`colorizers/formatter-coloring-pipeline.cemt`](colorizers/formatter-coloring-pipeline.cemt)
  declares `acme.showcase.color-tree` as a package-qualified colorizer that
  extends `cem.color-tree`.
- [`colorizers/cem-tree-helpers.cemt`](colorizers/cem-tree-helpers.cemt)
  declares private `cemml.cem-tree.*` colorizer helpers used by schema-local
  colorizer entrypoints.

The package manifest declares entrypoint files as `formatter` and `colorizer`
artifacts and shared helper files as `formatter-helper` and `colorizer-helper`
artifacts so runtime CEMT stages are tied to schema-owned assets instead of
inline Rust template strings. The artifact entries also declare the target CEM
tree identity (`application/cem`, `https://cem.dev/ns/cem-ml/1`, `cem-tree`),
the supplied CEMT function name, the CEMT function profile when present, and
the formatter/color profiles that select the asset during output pipeline
execution. Colorizer artifacts also keep the CEMT function profile separate
from the output color profile, because one CEMT body can serve multiple output
color profiles.

The baseline package-frame selectors are declared in `package.cem`:
`compact`, `pretty`, and `tabular` select the canonical `cem.format-tree`
formatter asset and matching helper asset; `terminal`, `html`, and `md` select
the canonical `cem.color-tree` colorizer asset. The formatter profiles use one
CEMT body with profile-aware layout: `compact` keeps minimal deterministic
spacing, `pretty` expands non-text child groups into block layout, and `tabular`
also lays attributes out vertically with formatter-owned line-ending and indent
nodes. The `html` colorizer selector maps to the class-based HTML mode and
materializes HTML writer attributes, `terminal` records terminal output plus
auto capability metadata and renders ANSI/SGR-colored CEM text without HTML
writer attributes, and `md` records Markdown output metadata and renders
Markdown-safe inline HTML color spans without HTML writer attributes.

Package-qualified formatter and colorizer artifacts are selected by explicit
CEMT function name first, then by stage profile fallback. This keeps the
canonical `cem.format-tree` and `cem.color-tree` pipeline stable while allowing
showcase or schema-specific formatter/colorizer bodies to opt in through the
same manifest-declared asset path.

The canonical and showcase artifacts expose their public formatter/colorizer
functions as thin wrappers over package-owned helpers such as
`cem.format-tree.apply-stage`, `cem.format-tree.build-nodes`,
`cem.format-tree.build-envelope`, `cem.color-tree.apply-stage`,
`cemml.cem-tree.format-tree-base`, and `cemml.cem-tree.color-tree-base`. New
schema-specific formatter/colorizer functions should pass formatter decisions,
color decisions, writer boundaries, and queued edits into helper functions
instead of copying the full pipeline body. Helpers that do not represent an
output stage use internal `{function @returns=...}` declarations rather than
`format-function` or `color-function`. The runtime loads matching helper
artifacts for the selected output stage before executing the public
formatter/colorizer body, so helpers can live in dedicated package `.cemt`
files beside their entrypoints.

`cem.format-tree.build-nodes` performs canonical node traversal in CEMT with
`typeOf`, `match`, `map`, `length`, numeric depth helpers, and helper calls. It
still delegates low-level block-child whitespace and content-boundary
construction to the registered CEMT runtime primitives until those
writer-adjacent formatting primitives are also expressed as schema-owned
helpers.

`cem.color-tree.apply-stage` performs canonical coloring in CEMT over the
already formatted `cem-tree`. It reads the selected `$colorProfile`, recursively
annotates `nodes`, `formatNodes`, and `colorNodes`, materializes writer
attribute nodes and text wrappers for HTML profiles, and leaves the writer as
the final serialization phase for the colored tree.

The `formatters/` and `colorizers/` directories are part of the package
contract. Formatter, colorizer, and helper artifacts stay in those
package-relative `.cemt` locations, keeping formatting, coloring, and schema
identity in the same package hierarchy.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic.cem`](examples/basic.cem) | Minimal persisted CEM-ML document. | Pass |
| [`nested-handoff.cem`](examples/nested-handoff.cem) | Namespaced content with a `text/html` handoff boundary. | Pass |
| [`formatter-coloring-pipeline.package-artifacts.fixture.cem`](examples/formatter-coloring-pipeline.package-artifacts.fixture.cem) | Checked stage fixture generated through manifest-declared formatter/colorizer artifacts selected by explicit CEMT aliases. | Pass |
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
