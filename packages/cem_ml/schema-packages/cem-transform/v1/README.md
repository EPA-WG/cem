# CEM Transform Template Schema Package

Status: initial source package

This package defines CEMT (`.cemt`) resources. CEMT is the primary declarative
converter implementation language in the schema content registry design.

Owned schema URI:

```text
https://cem.dev/ns/transform/cem/1
```

Primary content type:

```text
application/vnd.cem.transform+cem
```

CEMT reuses the CEM-native template module language. Source and target content
identity is not embedded in `.cemt`; it is declared by `package.cem` converter
edges so the same template execution surface can participate in registry
planning.
