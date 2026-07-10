# CEM Transform Template Schema Package

Status: schema, examples, formatter, and colorizer package frame

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

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

## Output Producer And Encoding Contract

CEMT is the primary output producer for schema-owned exports. Output production
includes transformation, syntax/context encoding, formatting, terminal/HTML
color output, source-map span creation, final artifact identity,
content-type-specific encoders, formatters, colorizers, writer primitives, and
small transformation helpers. These capabilities are part of the CEMT stack,
not an external post-processing layer.

Encoding in this contract means syntax/context encoding: JSON string escaping,
XML attribute escaping, HTML text-state escaping, CSV field quoting, CSS
identifier escaping, YAML scalar style selection, CEM binary chunk framing, and
similar target-context operations. It is separate from byte character encoding
such as UTF-8 or UTF-16, and separate from transport content encoding such as
gzip.

The schema-owned output pipeline is:

```text
typed subject
  -> schema-owned CEMT output producer
    -> content-type-specific encode / format / color helpers
      -> encoded text, bytes, token stream, or chunk stream
        -> destination content type and schema
```

Native output producers exist for performance, bootstrap, binary framing, and
clarity where a syntax profile is not yet expressible in CEMT. They are paired
fallback or fast-path implementations, not replacements for the CEMT contract.
Every native producer should have a matching CEMT producer or a planned CEMT
producer, and shared fixtures must cross-check native output against CEMT
output. Differences must be explicit diagnostics or documented
lossiness/canonicalization choices.

CEMT serializers need context-aware encoding rather than ad hoc string
concatenation. The standard encoding function surface is:

```text
encode(subject, target, options?) -> encoded-artifact
```

`subject` is unencoded typed data: a scalar, name, attribute, AST node, DOM
node, structured value, token stream, or binary projection chunk. `target`
declares destination `contentType`, `schema`, and encoding `category`, with an
optional context such as document, fragment, text, attribute, name, string
literal, or binary chunk. `options` controls canonicalization, charset, line
ending, namespace policy, quoting, indentation, and source-map behavior.

The result is not a plain string. It is an encoded artifact carrying target
content type, schema URL, category, output kind (`text`, `bytes`, `tokens`, or
`chunks`), source-map spans, and a double-encoding guard. Template output must
reject encoded artifacts whose identity or category is incompatible with the
surrounding output context.

The CEMT output stack provides:

- encoder functions for context-specific escaping and binary framing;
- formatter functions for indentation, line endings, ordering, wrapping,
  scalar style, namespace declaration placement, and canonical output;
- color functions for semantic style roles, terminal ANSI/SGR output, HTML
  color output, no-color fallbacks, and accessibility-aware palettes;
- writer primitives for tokens, byte streams, sealed chunks, and source-map
  spans;
- schema helpers for target syntax rules, void/empty element policy, raw-text
  modes, namespace repair, identifier validity, and field/header policy;
- diagnostics for unsupported category, unsafe raw output, context mismatch,
  charset mismatch, unsupported color capability, lossy output, and
  native/CEMT parity mismatch.

CEMT output production is not a hidden content-type-to-content-type conversion
mechanism. A serializer edge writes a typed CEM subject to a destination syntax
and content identity, such as `CEM AST -> text/html`. A conversion pipeline may
parse, normalize, validate, and change semantic models between content
identities, such as `text/html -> normalized HTML model ->
application/xhtml+xml`. Both use registry identities, but callers select them
through separate planning domains.

Schema packages may declare named encoding, formatting, and color output
functions in CEMT modules. Example encoding declaration shape:

```cem
{encoding-function
    @name="html.text"
    @category="html-text"
    @subject="string"
    @produces="text"
    @content-type="text/html"
    @schema="https://cem.dev/ns/data/html/1"
    @canonical=true
    @streamable=true |
    {param @name="subject" @type="string" @required=true}
    {param @name="mode" @type="string" @default="canonical"}
}
```

The v1 schema also supports internal helper declarations:
`function @name @returns`, with typed `param` children and one executable
`body` expression. Internal functions are reusable CEMT runtime helpers; they
return JSON-compatible CEMT values and do not register as destination output
producers.

The first implementation slice validates this declaration vocabulary
structurally in the v1 schema: `function`, `encoding-function`,
`format-function`, and `color-function` declarations are distinct. Output
function declarations must carry function identity, subject, produced kind,
content type, schema, and category metadata. The design intent is CEMT-first
output producer edges with shared writer primitives and paired native producer
hooks while the encoder, formatter, and terminal/HTML color output surface
matures. The temporary proposal remains as an implementation backlog and
worked-example source in
[`../../../docs/cemt-encoding-proposal.tmp.md`](../../../docs/cemt-encoding-proposal.tmp.md).

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-transform.cemt`](examples/basic-transform.cemt) | Minimal CEMT module with one template body. | Pass |
| [`module-transform.cemt`](examples/module-transform.cemt) | Converter template module with import metadata, params, nested output, and `with:*` data propagation. | Pass |
| [`function-declarations.cemt`](examples/function-declarations.cemt) | Internal helper, encoding, formatting, color, and custom function declarations for CEMT output production. | Pass |
| [`formatter-coloring-pipeline.cemt`](examples/formatter-coloring-pipeline.cemt) | Executable CEM tree formatter and colorizer bodies that materialize formatted and colored CEM trees before the writer phase. | Pass |
| [`formatter-coloring-pipeline.fixture.cem`](examples/formatter-coloring-pipeline.fixture.cem) | CEM-native stage fixture paired with the formatter/coloring CEMT example for Storybook and fixture tests. | Pass |
| [`invalid-missing-required-attribute.cemt`](examples/invalid-missing-required-attribute.cemt) | Template declaration missing the inherited required `name` attribute. | Fail with `cem.schema_model.missing_required_attribute` |
| [`invalid-function-missing-category.cemt`](examples/invalid-function-missing-category.cemt) | Encoding function declaration missing required output category metadata. | Fail with `cem.schema_model.missing_required_attribute` |
| [`invalid-function-missing-contract-metadata.cemt`](examples/invalid-function-missing-contract-metadata.cemt) | Encoding function declaration missing required canonical and streamable contract metadata. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.transform+cem \
  --schema https://cem.dev/ns/transform/cem/1 \
  packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt
```
