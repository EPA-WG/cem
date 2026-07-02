# YAML Resource Schema Package

This package defines registry identity for generic YAML resources.

YAML source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/yaml` content type are
parsed by a YAML parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/yaml/1`
- Primary content type: `application/yaml`
- Compatibility aliases: `application/x-yaml`, `text/yaml`, `text/x-yaml`
- Preferred extension: `.yaml`
- Accepted extension: `.yml`

RFC 9512 registers `application/yaml` and identifies the older names above as
deprecated aliases that are still seen in deployed systems.

The `+yaml` structured syntax suffix is a content-type family signal for future
vendor or domain-specific YAML packages. This generic package owns only the base
YAML resource schema and common compatibility aliases.

## Resource Model

The schema describes YAML streams as a lossless resource model:

- streams contain zero or more documents;
- documents contain one root representation-graph node;
- mappings preserve entry order and key/value source identity;
- sequences preserve item order;
- scalars preserve style, lexical text, and implicit kind when available;
- anchors and aliases preserve graph identity when the parser exposes them;
- comments and directives are retained as presentation metadata where possible.

Parsers must use safe tag resolution by default. Host-object or executable tags
belong behind explicit adapter policy and runtime limits.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-document.yaml`](examples/basic-document.yaml) | Minimal mapping with scalar values and a sequence. | Pass |
| [`nested-stream.yml`](examples/nested-stream.yml) | Multi-document YAML stream with nested mappings and sequences. | Pass |
| [`invalid-parse.yaml`](examples/invalid-parse.yaml) | Unterminated flow sequence rejected by the YAML parser. | Fail with `cem.yaml.parse_error` |
| [`invalid-unsafe-tag.yaml`](examples/invalid-unsafe-tag.yaml) | Explicit non-core tag rejected by the safe default policy. | Fail with `cem.yaml.unsafe_tag` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/yaml \
  --schema https://cem.dev/ns/data/yaml/1 \
  packages/cem_ml/schema-packages/yaml/v1/examples/basic-document.yaml
```

Convert the checked-in `.yml` stream sample to JSON with ANSI color on stdout
from a built CLI binary:

```bash
dist/target/debug/cem-ml convert \
  packages/cem_ml/schema-packages/yaml/v1/examples/nested-stream.yml \
  --content-type application/yaml \
  --schema https://cem.dev/ns/data/yaml/1 \
  --to-content-type application/json \
  --to-schema https://cem.dev/ns/data/json/1 \
  --output-color-type ansi-256
```
