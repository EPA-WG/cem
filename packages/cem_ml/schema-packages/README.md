# CEM-ML Schema Packages

[cem-ml-schema-content-registry-design.md](../../../docs/cem-ml-schema-content-registry-design.md)

Schema packages are versioned schema modules registered by schema URL and
content type. The first packages form the bootstrap chain for the CEM stack:

```text
CEM-ML syntax
  -> schema definition language
    -> schema package manifest schema
      -> package.cem manifest instances
```

## Bootstrap Relationship

`cem-ml/v1` defines the generic CEM-ML syntax and document model. It owns the
base `application/cem` content type, directive syntax, namespace binding,
elements, attributes, text nodes, content scopes, and handoff boundaries.

`schema/v1` defines the schema definition language expressed in CEM-ML. Schema
documents validate as CEM-ML documents first, then as instances of the schema
definition language. This package owns
`application/vnd.cem.schema+cem`.

`schema-package/v1` defines the package manifest schema for
`schema-packages/{schema-name}/{version}/package.cem`. The schema package
schema is itself authored with the schema definition language, and manifest
instances validate against it. This package owns
`application/vnd.cem.schema-package+cem`.

`cem-native-template/v1` defines the CEM-native template module language used
by template adapters. It owns `application/vnd.cem.template+cem` and also
claims current generic CEM source content types as aliases that require an
explicit schema when ambiguous.

`cem-transform/v1` defines CEMT (`.cemt`) converter-template resources. It
owns `application/vnd.cem.transform+cem` and reuses the CEM-native template
schema as its base language.

`cem-ql/v1` defines CEM-QL query source module and compiled query artifact
resource identities. It owns `application/vnd.cem.query+cem-ql`, claims
`text/cem-ql` as an authoring alias, and claims compiled artifact/cache aliases
for query binaries. CEM-QL source is not CEM-ML syntax; its parser lives in the
`cem-ql` crate.

`json/v1` defines generic JSON text resource identity. It owns
`application/json` and claims `text/json` as an alias. JSON source is not
CEM-ML syntax, and this package intentionally does not claim JSON Schema or
CEM-specific projection/vendor `+json` content types.

`json-schema/v1` defines JSON Schema document identity. It owns
`application/schema+json`, depends on `json/v1`, and models JSON Schema
dialect, vocabulary, reference, and validation-resource metadata separately
from generic JSON values.

## Validation Model

The relationship is layered validation, not broad inheritance:

- A schema document must parse as CEM-ML and validate against the schema
  definition language.
- `schema-package.cem` is a schema document, so it validates against
  `https://cem.dev/ns/schema/1`.
- `package.cem` is a manifest instance, so it validates against
  `https://cem.dev/ns/schema-package/1`.
- A package manifest does not inherit arbitrary schema-definition elements such
  as `element`, `attribute`, or `constraint` unless the package manifest schema
  explicitly permits them.

Schema dependencies should be resolved by schema URL and content type, not by
filesystem path. Filesystem layout is a distribution detail for local packages.

## Direct References

Reusable schema relationships are declared inside schema documents with
`uses/use` entries:

```cem
{uses |
    {use @schema="https://cem.dev/ns/schema/1" @as="schema"}
}
```

Downstream declarations refer to imported definitions with qualified names, for
example `schema:media-type` or `schema:uri`. This keeps package-specific schemas
small while preserving strict validation boundaries for their own instances.
