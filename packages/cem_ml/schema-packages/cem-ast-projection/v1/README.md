# CEM AST Projection Schema Package

Status: schema, binary/JSON examples, README previews, and package-local
verification frame

This package defines the semantic CEM AST projection layer. The projection is a
parser-owned structural view of CEM source: document nodes, element nodes,
attributes, text, token/range facts, parser diagnostics, source-map metadata,
and sealed binary chunks for runtime/cache handoff.

## Source Identity

Owned schema URI:

```text
https://cem.dev/ns/projection/ast/1
```

Primary content type:

```text
application/vnd.cem.ast+cem-bin
```

Debug/interchange view:

```text
application/vnd.cem.ast+json
```

The JSON AST export keeps the same schema identity but is a debug and
interchange view over the semantic AST projection, not the canonical runtime
artifact. The binary projection is the compatibility anchor for runtime and
cache handoff.

## Projection Facts And Diagnostics

Binary AST projection envelopes expose sealed semantic-record chunks. The root
chunk carries the binary header and links to the document node chunk; node
chunks use stable `node:{id}` root ids, parent chunk ids, ordered child links,
per-chunk hashes, and source-map deltas derived from byte source truth. Native
consumers can replay the original artifact by sorting chunks by `byteOffset`
without reserializing the JSON debug view.

The package schema declares the AST-facing element and attribute contracts and
the diagnostic codes for projection-specific policy:

- `cem.projection.ast.node_id_duplicate`
- `cem.projection.ast.token_range_invalid`
- `cem.projection.ast.chunk_hash_mismatch`
- `cem.projection.ast.source_map_missing`
- `cem.projection.ast.binary_magic`
- `cem.projection.ast.binary_truncated`
- `cem.projection.ast.binary_version`
- `cem.projection.ast.projection_mismatch`
- `cem.projection.ast.json_parse_error`
- `cem.projection.ast.json_shape`

Current incomplete boundary: binary and JSON source validation still runs
through native projection validators. The target shape is Rust extracting
neutral projection facts with byte ranges while this package's `.cem` schema
owns code, severity, and structured details.

## Output Artifacts

This package currently declares one Rust converter from
`application/vnd.cem.ast+cem-bin` to `application/vnd.cem.ast+json` for debug
inspection. It does not declare formatter or colorizer artifacts. Formatter
profiles `compact`, `pretty`, and `tabular`, colorizer profiles `terminal`,
`html`, and `md`, and generic `lineEnding=lf|crlf|preserve` behavior are
therefore not package-owned output surfaces yet.

If AST projection formatting becomes user-facing, formatter/colorizer assets
must follow the common package contract: formatters produce formatted CEM trees,
colorizers enrich those trees, and the generic writer emits target-native
terminal, HTML, Markdown, or byte output.

## Safety Notes

AST projection data may preserve source text, attribute values, diagnostics,
and source-map coordinates from user input. Tools should treat projection
artifacts as data, not executable content. Binary readers must fail closed on
invalid magic bytes, unsupported versions, truncation, hash mismatch, and
source-map inconsistency. JSON debug views should not be treated as canonical
cache input unless they are explicitly regenerated from a trusted binary
projection.

## Formatter And Preview SDLC

When a command example, fixture, converter, CLI report shape, or visible
presentation output changes, update the SVG previews in `examples/previews/`
in the same change by running
`node packages/cem_ml/schema-packages/cem-ast-projection/v1/scripts/verify-previews.mjs --update`.

The package `verify` target regenerates previews into `dist/previews/` and
fails on drift.

## Verification

Focused package gates:

```bash
cargo test -p cem-ml cem_ast_projection_package_examples_are_manifest_indexed
```

```bash
cargo test -p cem-ml-cli schema_owned_cem_ast_projection_examples_validate_through_cli
```

```bash
yarn nx run cem_ml_schema_package_cem_ast_projection_v1:verify
```

## Release Behavior

This package is versioned as `1.0.0`. The primary binary content type and schema
URI are compatibility anchors. The JSON view is a debug/interchange alias and
may gain additional derived fields as long as binary projection identity remains
stable. Unsupported binary versions, malformed chunks, and invalid JSON shapes
fail closed through projection diagnostics.

Tracked but not complete:

- schema-owned fact bindings for all binary and JSON projection diagnostics;
- canonical binary writer/reader parity fixtures beyond the current basic
  projection examples;
- package-owned formatter/colorizer profiles if AST projection presentation
  becomes user-facing.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example                                                     | Content type                      | Purpose                                                                                  | Expected result                             |
| ----------------------------------------------------------- | --------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------- |
| [`basic-ast.cem-bin`](examples/basic-ast.cem-bin)           | `application/vnd.cem.ast+cem-bin` | Canonical binary AST projection generated by `cem-ml convert --to-format ast-bin`.       | Pass                                        |
| [`basic-ast.ast.json`](examples/basic-ast.ast.json)         | `application/vnd.cem.ast+json`    | Minimal AST JSON debug view with one element and text child.                             | Pass                                        |
| [`nested-ast.ast.json`](examples/nested-ast.ast.json)       | `application/vnd.cem.ast+json`    | Nested form AST with attributes, text, null-valued boolean attribute, and source ranges. | Pass                                        |
| [`invalid-kind.ast.json`](examples/invalid-kind.ast.json)   | `application/vnd.cem.ast+json`    | AST JSON node with an unsupported node kind.                                             | Fail with `cem.projection.ast.json_shape`   |
| [`invalid-binary.cem-bin`](examples/invalid-binary.cem-bin) | `application/vnd.cem.ast+cem-bin` | Binary projection with an invalid magic header.                                          | Fail with `cem.projection.ast.binary_magic` |

Validate a binary projection explicitly:

```bash
cargo run -p cem-ml-cli -- validate \
  --format json \
  --content-type application/vnd.cem.ast+cem-bin \
  --schema https://cem.dev/ns/projection/ast/1 \
  packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.cem-bin
```

![Preview of the CEM AST binary validation JSON report](examples/previews/basic-ast-binary-validate.svg)

Validate a JSON debug view explicitly:

```bash
cargo run -p cem-ml-cli -- validate \
  --format json \
  --content-type application/vnd.cem.ast+json \
  --schema https://cem.dev/ns/projection/ast/1 \
  packages/cem_ml/schema-packages/cem-ast-projection/v1/examples/basic-ast.ast.json
```

![Preview of the CEM AST JSON validation report](examples/previews/basic-ast-json-validate.svg)
