# CEM-ML CLI Transform Config Schema (`https://cem.dev/ns/cli/transform-config/1`)

**Status:** reserved runtime surface with active parser/lowering validation.

This is the source-of-truth schema artifact for the CEM-native CLI transform
graph config. It is separate from the CEM core document schema and separate
from CEM-native template schemas. The graph config describes import,
transformation, and export wiring; it does not define template execution
semantics.

Schema URI: `https://cem.dev/ns/cli/transform-config/1`
Namespace URI: `https://cem.dev/ns/cli/transform-config/1`

## Element Vocabulary

| Element | Required attributes | Optional attributes | Child elements |
| ------- | ------------------- | ------------------- | -------------- |
| `run` | none | none | `import` |
| `import` | `src` | `id`, `content-type`, `contentType`, `schema` | `join`, `transform`, `export` |
| `join` | `mode` | `id`, `input`, `by`, `with:*` | `transform`, `export` |
| `transform` | `src` | `id`, `input`, `with:*`, `template-content-type`, `templateContentType`, `template-schema`, `templateSchema` | `join`, `transform`, `export` |
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
extension. The current parser classifies XSLT and CEM-native templates for
planning only; execution remains reserved until the runtime API and template
semantics are designed.

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
