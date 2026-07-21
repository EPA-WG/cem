# CSV Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for generic comma-separated value
resources.

CSV source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `text/csv` content type are parsed
by a CSV parser or adapter.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/csv/1`
- Primary content type: `text/csv`
- Preferred extension: `.csv`

RFC 4180 registers `text/csv` with optional `charset` and `header` parameters.
RFC 7111 defines row, column, and cell fragment identifiers for `text/csv`.
The current parser accepts UTF-8 and US-ASCII CSV sources directly. Other
declared charsets require an explicit converter instead of silent transcoding.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.
Each profile is backed by the matching design-named package asset:
`formatters/compact.cemt`, `formatters/pretty.cemt`, `formatters/tabular.cemt`,
`colorizers/terminal.cemt`, `colorizers/html.cemt`, and `colorizers/md.cemt`.

## Resource Model

The schema describes CSV resources as a tabular model:

- tables preserve row order;
- rows preserve field order;
- optional header rows provide column names;
- fields preserve lexical value, quoted state, and source location when
  available;
- parser metadata records encoding, delimiter, line-ending style, and header
  disposition.

The default delimiter for `text/csv` is comma. Other delimited text formats
should use their own schema package or an explicit converter profile instead of
being silently treated as CSV.
When the MIME `header` parameter is present, it must be `present` or `absent`;
an unknown value is reported as package-owned metadata drift while the CSV bytes
are still parsed.

## Design: Schema-Owned Parsing And Validation

The target is schema-owned CSV semantics, not a claim that bytes can be parsed
without host code. CSV is not CEM-ML syntax, so a byte reader still needs a
host parsing primitive. The ownership change is that Rust stops deciding
CSV-specific policy and diagnostics. Rust supplies generic, deterministic facts
about the byte stream; `schema/csv.cem` owns which facts are valid, which facts
are warnings or errors, and which diagnostic codes are emitted.

### Current Boundary To Remove

The current CLI path has CSV-specific validation logic in Rust. It interprets
the `text/csv` parameters, checks UTF-8 and US-ASCII byte compatibility,
parses records, detects inconsistent field counts, maps parser errors to
`cem.csv.*` diagnostics, and decides warning versus error severity.

That makes the CSV package partly declarative and partly Rust-owned. The
schema declares `cem.csv.unclosed_quote`, `cem.csv.inconsistent_field_count`,
and related diagnostics, but the Rust validator still owns the conditions that
produce them.

### Target Boundary

The target boundary has three layers:

1. Source identity normalization:
   The generic content-type reader normalizes the media-type essence and
   exposes parameters such as `charset` and `header` as structured source
   metadata. It does not decide CSV policy beyond generic media-type parsing.

2. CSV parse fact extraction:
   A generic resource behavior reads bytes and returns a schema-facing parse
   report. The report is data, not diagnostics. It includes encoding facts,
   delimiter facts, records, fields, quoted state, line endings, parser events,
   byte ranges, and recoverable or fatal parse facts such as unclosed quotes.

3. CSV schema contract evaluation:
   `schema/csv.cem` evaluates the parse report using schema-declared
   constraints, behaviors, comparisons, and diagnostics. This layer decides
   whether a fact is accepted, warned, or rejected.

Rust may still implement a reusable parser primitive for performance and byte
accuracy, but it must be policy-neutral. It should not contain CSV-specific
diagnostic-code selection, severity selection, or package-specific acceptance
rules.

### Schema-Facing Parse Report

The parser primitive should project a stable report shape that the schema can
query:

- `source`: URI, content type, media-type parameters, byte length, and charset
  declaration;
- `encoding`: decoded charset, decoder status, first invalid byte when present,
  and whether transcoding was required;
- `dialect`: delimiter, quote character, escape style, header parameter value,
  and line-ending style;
- `records`: ordered rows with record index, field count, byte range, and line
  range;
- `fields`: ordered fields with row index, field index, raw span, decoded value,
  quoted state, and escape events;
- `parseFacts`: non-diagnostic facts such as `unclosed-quote`,
  `invalid-quote-escape`, `ragged-row`, `unsupported-charset`,
  `declared-us-ascii-non-ascii-byte`, and `invalid-header-parameter`;
- `sourceMap`: a byte-offset map for rows, fields, quote tokens, separators,
  and line endings when available.

The schema owns the mapping from those facts to diagnostics. For example,
`parseFacts.kind = "unclosed-quote"` maps to `cem.csv.unclosed_quote` as an
error, while `parseFacts.kind = "ragged-row"` maps to
`cem.csv.inconsistent_field_count` as a warning unless a future schema profile
explicitly permits ragged rows.

### CSV Schema Contracts

`schema/csv.cem` should become the single place that declares these package
policies:

- `csv-source-parser`: binds the generic resource behavior that produces the
  parse report and declares the expected report shape;
- `charset-parameter-supported`: accepts `utf-8`, `utf8`, `us-ascii`, and
  `ascii`; other declared charsets require an explicit converter before CSV
  validation;
- `us-ascii-byte-compatibility`: rejects non-ASCII bytes when the source
  declares US-ASCII;
- `header-parameter-values`: accepts `present` and `absent`; unknown values are
  metadata drift and emit a warning while valid CSV bytes can still parse;
- `comma-delimiter-default`: keeps `text/csv` comma-delimited unless a future
  package profile declares a different dialect;
- `quote-escape-policy`: double quotes inside quoted fields must be represented
  by two double quote characters;
- `field-count-policy`: rows should match the first row width unless a future
  schema profile explicitly permits ragged rows;
- `source-map-preferred`: parser-provided byte ranges should be preserved on
  table, row, and field facts when available.

Diagnostics stay package-owned in this schema. Rust should pass fact kinds,
source ranges, and raw parser details upward; the schema decides whether to
emit `cem.csv.parse_error`, `cem.csv.unsupported_encoding`,
`cem.csv.unclosed_quote`, `cem.csv.invalid_quote_escape`,
`cem.csv.inconsistent_field_count`, `cem.csv.invalid_header_parameter`, or
`cem.csv.source_map_unavailable`.

### Behavior And Diagnostic Binding

The schema should use package-visible behavior declarations for diagnostic
construction. Each behavior binds a parse fact plus source range inputs and
returns a diagnostic payload with stable structured details:

- `factKind`: the parser fact being interpreted;
- `contract`: the schema constraint that interpreted the fact;
- `sourceRange`: byte and line range from the parser report;
- `mediaType`: normalized `text/csv` identity and original parameters;
- `rowIndex` and `fieldIndex` when the fact is row or field scoped;
- `expected` and `actual` for value comparisons such as field counts or header
  parameter values.

This keeps compatibility with existing CLI diagnostic codes while making the
diagnostic provenance schema-owned and inspectable.

### Migration Plan

1. Add the parse-report model to `schema/csv.cem` as schema-declared nodes,
   attributes, constraints, and behavior names.
2. Add a generic host behavior for CSV parse fact extraction. The behavior name
   should be referenced from `csv-source-parser` instead of being called
   directly by CLI CSV special cases.
3. Change CLI validation to route `text/csv` through the generic
   schema-package validation path. The CLI should provide bytes, content type,
   schema URI, and resolver context, then consume schema-produced diagnostics.
4. Move existing CSV diagnostic mapping out of
   `packages/cem_ml_cli/src/dispatch.rs`. Keep only generic source loading,
   media-type parsing, schema selection, and report projection there.
5. Expand package examples so every current Rust-owned condition has a
   schema-owned fixture: valid table, quoted fields, unclosed quote, invalid
   quote escape, ragged row, unsupported charset, US-ASCII byte mismatch, and
   invalid `header` parameter.
6. Add contract tests that mutate `schema/csv.cem` behavior bindings and prove
   CSV diagnostics change because the schema changed, not because a Rust CSV
   branch changed.

### Verification Gates

The migration is not complete until these gates pass:

- the CSV package examples validate through the same schema-package example
  harness used by other packages;
- no `cem.csv.*` diagnostic is emitted by a CSV-specific branch in
  `packages/cem_ml_cli/src/dispatch.rs`;
- source ranges for row, field, quote, and encoding diagnostics survive through
  CLI JSON output;
- CEMT formatter/colorizer assets consume the schema-facing table model rather
  than reparsing CSV bytes;
- `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture` and
  `yarn nx run cem_ml:test` pass after the Rust-specific CSV validator is
  removed.

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-table.csv`](examples/basic-table.csv) | Minimal table with a header row and scalar fields. | Pass |
| [`quoted-fields.csv`](examples/quoted-fields.csv) | Quoted fields with an embedded newline and escaped quote. | Pass |
| [`invalid-unclosed-quote.csv`](examples/invalid-unclosed-quote.csv) | Unterminated quoted field rejected by the CSV quote policy. | Fail with `cem.csv.unclosed_quote` |
| [`ragged-row.csv`](examples/ragged-row.csv) | Row width differs from the first row. | Pass with warning `cem.csv.inconsistent_field_count` |
| [`invalid-header-parameter.csv`](examples/invalid-header-parameter.csv) | CSV bytes are valid, but MIME header metadata is invalid. | Pass with warning `cem.csv.invalid_header_parameter` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type text/csv \
  --schema https://cem.dev/ns/data/csv/1 \
  packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv
```
