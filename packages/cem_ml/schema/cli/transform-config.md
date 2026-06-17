# CEM-ML CLI Transform Config Schema (`https://cem.dev/ns/cli/transform-config/1`)

**Status:** reserved runtime surface with active parser/lowering validation.

This is the source-of-truth schema artifact for the CEM-native CLI transform
graph config. It is separate from the CEM core document schema and separate
from CEM-native template schemas. The graph config describes import,
transformation, and export wiring; it does not define template execution
semantics. The CEM-native template declaration schema is
`https://cem.dev/ns/template/cem-native/1` and lives at
`packages/cem_ml/schema/template/cem-native-template.md`.

Schema URI: `https://cem.dev/ns/cli/transform-config/1`
Namespace URI: `https://cem.dev/ns/cli/transform-config/1`

## Element Vocabulary

| Element | Required attributes | Optional attributes | Child elements |
| ------- | ------------------- | ------------------- | -------------- |
| `run` | none | none | `import` |
| `import` | `src` | `id`, `content-type`, `contentType`, `schema` | `join`, `transform`, `export` |
| `join` | `mode` | `id`, `input`, `by`, `with:*` | `transform`, `export` |
| `transform` | `src` | `id`, `input`, `with:*`, `entrypoint`, `template-content-type`, `templateContentType`, `template-schema`, `templateSchema` | `param`, `join`, `transform`, `export` |
| `param` | `name`, `value` | none | none |
| `export` | `out` | `id`, `content-type`, `contentType`, `schema` | none |

## Graph Semantics

- A document must contain exactly one top-level `run` element.
- `run` contains one or more import branches.
- `import` creates a source node from `@src`.
- `join @mode="collect"` creates one collection artifact from all artifacts in
  its primary input.
- `join @mode="group-by" @by="NAME"` creates one collection artifact for each
  distinct source binding value named by `@by`.
- `join @mode="match-by" @by="NAME" @with:LABEL="NODE"` creates one collection
  artifact for each primary input key and attaches same-key named secondary
  input artifacts.
- `join @mode="zip" @with:LABEL="NODE"` creates one collection artifact for
  each positional tuple across primary and named secondary input artifacts.
  Unequal input counts are a schema validation failure.
- `transform` creates a template application node from `@src`.
- `transform @entrypoint` selects a public CEM-native template entrypoint.
- `transform` child `param @name @value` records provide caller params for that transform stage.
- `export` creates a sink node from `@out`.
- Nested operation nodes create parent graph edges.
- `@input` creates an explicit primary input edge to an existing graph node.
- `@with:*` creates named side-input edges to existing graph nodes.
- `join` is the only supported cardinality-changing operation in this slice.
  Supported join modes are `collect`, source-binding `group-by`, and
  same-binding `match-by`, and positional `zip`.
- Graph node IDs are explicit `@id` values or parser-generated fallback IDs.
- Duplicate IDs, unknown references, cycles, missing required attributes, and
  duplicate output destinations are schema validation failures.
- `@out` patterns use named path placeholders such as `{stem}`. Bare `*`
  wildcards are rejected.

## Template Identity

Transform nodes must declare or imply a supported template identity through
`@template-content-type`, `@template-schema`, or a recognized `@src`
extension. The current parser classifies XSLT and CEM-native templates; the
runtime executes supported CEM-native templates and keeps XML+XSLT execution
deferred.

## CEM-Native Module Params

`transform @entrypoint` selects a public CEM-native template entrypoint. Child
`param @name @value` records provide caller params for that transform stage.
`@value` is a string-first CLI/config value: source bindings such as `{stem}`
are expanded by the CLI host, then the CEM-native module declaration coerces
the string according to the selected entrypoint's param type. These fields are
valid only for CEM-native module execution; fragment execution still uses the
implicit entrypoint with no params.

## Example

```cem
{@doc cem-ml 1}
{run |
  {import @id=book @src="inputs/*.xml" @content-type="application/xml" |
    {transform @id=html @src="templates/book.cem" |
      {export @id=page @out="book/chapters/{stem}.html" @content-type="text/html"}
    }
    {transform @id=chart @src="illustrations/chart1.cem" |
      {export @id=chart-out @out="book/chapters/{stem}/img/chart1.svg" @content-type="image/svg+xml"}
    }
  }
}
```
