# CEM-ML Generic Schema Package

Status: initial source package

[cem-ml-syntax.md](../../../../../docs/cem-ml-syntax.md)

This package is the first schema-package source for the generic CEM-ML document
model. It owns CEM-ML syntax and content-type identity, not domain semantics.

Owned schema URI:

```text
https://cem.dev/ns/cem-ml/1
```

Primary content type:

```text
application/cem
```

Current aliases mirror the Rust runtime's existing accepted CEM source content
types:

- `text/cem-ml`
- `text/cem`
- `application/cem+xml`

The semantic CEM annotation vocabulary remains in `packages/cem_ml/schema/cem-core.md`
under `https://cem.dev/ns/core/1`.
