# CSV Resource Schema Package

Status: schema, examples, formatter, and colorizer package frame

This package defines registry identity for generic comma-separated value
resources.

CSV source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `text/csv` content type are parsed
by a CSV parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/csv/1`
- Primary content type: `text/csv`
- Preferred extension: `.csv`

RFC 4180 registers `text/csv` with optional `charset` and `header` parameters.
RFC 7111 defines row, column, and cell fragment identifiers for `text/csv`.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

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

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-table.csv`](examples/basic-table.csv) | Minimal table with a header row and scalar fields. | Pass |
| [`quoted-fields.csv`](examples/quoted-fields.csv) | Quoted fields with an embedded newline and escaped quote. | Pass |
| [`invalid-unclosed-quote.csv`](examples/invalid-unclosed-quote.csv) | Unterminated quoted field rejected by the CSV quote policy. | Fail with `cem.csv.unclosed_quote` |
| [`ragged-row.csv`](examples/ragged-row.csv) | Row width differs from the first row. | Pass with warning `cem.csv.inconsistent_field_count` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type text/csv \
  --schema https://cem.dev/ns/data/csv/1 \
  packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv
```
