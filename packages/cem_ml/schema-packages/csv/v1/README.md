# CSV Resource Schema Package

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
