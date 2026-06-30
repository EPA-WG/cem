# RELAX NG Schema Package

This package defines registry identity for RELAX NG schema resources.

RELAX NG schema source is not CEM-ML syntax. The schema package and manifest are
authored in CEM-ML, but `.rng` resources are parsed as XML syntax and `.rnc`
resources are parsed as compact syntax text.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/relax-ng/1`
- Primary content type: `application/relax-ng+xml`
- Alias content types: `application/relax-ng-compact-syntax`
- Preferred extensions: `.rng`, `.rnc`
- XML syntax namespace: `http://relaxng.org/ns/structure/1.0`

## Resource Model

The schema describes RELAX NG resources as validation schema inputs:

- XML syntax resources preserve RELAX NG namespace identity, grammar root,
  start pattern, defines, patterns, attributes, and source offsets;
- compact syntax resources preserve namespace declarations, start definition,
  pattern definitions, operators, string literals, and source offsets;
- include and external reference declarations remain explicit, but are rejected
  unless an explicit resolver policy enables them.

## Validation Examples

Validate RELAX NG XML syntax through the CLI with the schema URL and content
type:

```bash
cem-ml validate --format json \
  --content-type application/relax-ng+xml \
  --schema https://cem.dev/ns/data/relax-ng/1 \
  packages/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rng
```

Validate RELAX NG compact syntax similarly:

```bash
cem-ml validate --format json \
  --content-type application/relax-ng-compact-syntax \
  --schema https://cem.dev/ns/data/relax-ng/1 \
  packages/cem_ml/schema-packages/relax-ng/v1/examples/basic-schema.rnc
```

Checked examples:

- [basic-schema.rng](examples/basic-schema.rng): a minimal XML syntax grammar.
- [datatype-schema.rng](examples/datatype-schema.rng): XML syntax with an XML
  Schema datatype.
- [basic-schema.rnc](examples/basic-schema.rnc): compact syntax for the same
  simple document shape.
- [invalid-missing-start.rng](examples/invalid-missing-start.rng): reports
  `cem.relax_ng.missing_start`.
- [invalid-unknown-element.rng](examples/invalid-unknown-element.rng): reports
  `cem.relax_ng.unknown_element`.
- [invalid-unclosed-compact.rnc](examples/invalid-unclosed-compact.rnc):
  reports `cem.relax_ng.compact_parse_error`.
