# JSON Resource Schema Package

This package defines registry identity for generic JSON text resources.

JSON source is not CEM-ML syntax. The schema package and this manifest are
authored in CEM-ML, but resources with the `application/json` content type are
parsed by a JSON parser or adapter.

## Owned Identities

- Schema URL: `https://cem.dev/ns/data/json/1`
- Primary content type: `application/json`
- Authoring/legacy alias: `text/json`

This package does not claim JSON Schema (`application/schema+json`) or
CEM-specific projection/vendor JSON types such as `application/vnd.cem.*+json`.
Those formats have their own schemas and converter rules.

## Resource Model

The schema describes JSON values as a lossless resource model:

- documents contain one root value;
- objects preserve member order and key source identity;
- arrays preserve item order;
- strings, numbers, booleans, and null preserve their JSON value kind;
- parsers should retain lexical/source-map information when available.
