# CEM-QL Query Resource Schema Package

This package defines registry identity for CEM-QL query source modules and
compiled query artifacts.

CEM-QL source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/vnd.cem.query+cem-ql`
content type are parsed by the `cem-ql` crate.

## Owned Identities

- Schema URL: `https://cem.dev/ns/query/cem-ql/1`
- Primary source content type: `application/vnd.cem.query+cem-ql`
- Authoring alias: `text/cem-ql`
- Compiled artifact alias: `application/vnd.cem.query-artifact+cem-bin`
- Legacy/internal cache aliases: `cem-ql/1`, `cem-ql/module`

## Resource Model

The schema describes the query resource model used by loaders and caches:

- query modules declare a module URI;
- imports bind other module URIs through explicit aliases;
- declarations define variables and functions;
- expressions are compiled to typed evaluator IR;
- compiled artifacts carry hash, mode, policy stamps, import closure, and
  optional source-map sidecars.
