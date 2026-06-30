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

## Output Producer And Encoding Contract

CEMT is the primary output producer for schema-owned exports. Content-type
specific encoders, formatters, terminal/HTML color output helpers, writer
primitives, and output transformation helpers belong to the CEMT stack. Native
output producers exist for performance, bootstrap, and clarity, but they should
be paired with CEMT implementations and cross-checked with shared fixtures.

CEMT serializers need context-aware encoding rather than ad hoc string
concatenation. The planned standard encoding function is:

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

Schema packages may declare named encoding, formatting, and color output
functions in CEMT modules. Proposed encoding declaration shape:

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

This declaration is proposed vocabulary, not yet validated by the v1 schema.
The design intent is CEMT-first output producer edges with shared writer
primitives and paired native producer hooks while the encoder, formatter, and
terminal/HTML color output surface matures. The working proposal lives in
[`../../../docs/cemt-encoding-proposal.tmp.md`](../../../docs/cemt-encoding-proposal.tmp.md).

## Validation Examples

The schema-owned examples live in [`examples/`](examples/) and are used by the
CLI validation integration tests.

| Example | Purpose | Expected result |
| --- | --- | --- |
| [`basic-transform.cemt`](examples/basic-transform.cemt) | Minimal CEMT module with one template body. | Pass |
| [`module-transform.cemt`](examples/module-transform.cemt) | Converter template module with import metadata, params, nested output, and `with:*` data propagation. | Pass |
| [`invalid-missing-required-attribute.cemt`](examples/invalid-missing-required-attribute.cemt) | Template declaration missing the inherited required `name` attribute. | Fail with `cem.schema_model.missing_required_attribute` |

Validate an example explicitly against this schema:

```bash
cargo run -p cem-ml-cli -- validate \
  --content-type application/vnd.cem.transform+cem \
  --schema https://cem.dev/ns/transform/cem/1 \
  packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt
```
