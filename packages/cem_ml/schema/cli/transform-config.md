# CEM-ML CLI Transform Config Schema (`https://cem.dev/ns/cli/transform-config/1`)

**Status:** reserved runtime surface with active parser/lowering validation.

This is the source-of-truth schema artifact for the CEM-native CLI transform
graph config. It is separate from the CEM core document schema and separate
from CEM-native template schemas. The graph config describes import,
transformation, and export wiring; it does not define template execution
semantics. The CEM-native template declaration schema is
`https://cem.dev/ns/template/cem-native/1` and lives at
`packages/cem_ml/schema-packages/cem-native-template/v1/`.

Schema URI: `https://cem.dev/ns/cli/transform-config/1`
Namespace URI: `https://cem.dev/ns/cli/transform-config/1`

## Element Vocabulary

| Element | Required attributes | Optional attributes | Child elements |
| ------- | ------------------- | ------------------- | -------------- |
| `run` | none | none | `import` |
| `import` | `src` | `id`, `content-type`, `contentType`, `schema` | `join`, `transform`, `rewrite-importmap`, `export` |
| `join` | `mode` | `id`, `input`, `by`, `with:*` | `transform`, `rewrite-importmap`, `export` |
| `transform` | `src` | `id`, `input`, `with:*`, `entrypoint`, `template-content-type`, `templateContentType`, `template-schema`, `templateSchema` | `param`, `join`, `transform`, `rewrite-importmap`, `export` |
| `rewrite-importmap` | `target-map` | `id`, `input`, `source-map`, `sourceMap`, `targetMap`, `mode`, `missing` | `export` |
| `param` | `name`, `value` | none | none |
| `export` | `out` | `id`, `content-type`, `contentType`, `schema`, `style-policy`, `stylePolicy` | none |

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
- Graph params and entrypoints are stage-local; top-level CLI `--param` and
  `--template-entrypoint` cannot be combined with `--config`.
- `rewrite-importmap` mutates the `<script type="importmap">` JSON in a text
  HTML input artifact and leaves the rest of the HTML unchanged.
- `rewrite-importmap @source-map` optionally validates the source `imports`
  entries before rewriting.
- `rewrite-importmap @target-map` points to a browser importmap JSON file whose
  `imports` entries are written into the HTML importmap.
- `rewrite-importmap @mode` supports `replace-imports` by default, plus `merge`
  and `replace-script`.
- `rewrite-importmap @missing` supports `error` by default, plus `ignore` and
  `insert`.
- `export` creates a sink node from `@out`.
- `export @style-policy` controls HTML style handling for mixed HTML/CSS
  exports. `auto` keeps inline styles unless a sibling CSS export for the same
  graph artifact exists, then links it; `inline` keeps inline `<style>` blocks;
  `link` requires a sibling CSS export and replaces inline styles with a
  stylesheet link; `omit` removes inline styles without adding a link.
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
runtime executes supported CEM-native templates and bounded XSLT 1.0 parity
templates through registered transform-template adapters.

## Template Entrypoints And Params

`transform @entrypoint` selects a template entrypoint for adapters that expose
named execution. Child `param @name @value` records provide caller params for
that transform stage. `@value` is a string-first CLI/config value: source
bindings such as `{stem}` are expanded by the CLI host before the engine
request is built. CEM-native module execution then coerces the string according
to the selected entrypoint's param declaration. XSLT parity passes the expanded
string as an `xsl:with-param` value for named template entrypoints. Plain
`cem-ql-fragment` execution still uses the implicit entrypoint with no params.

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
