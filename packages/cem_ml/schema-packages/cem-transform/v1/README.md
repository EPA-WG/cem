# CEM Transform Template Schema Package

Status: schema, examples, formatter, colorizer, README previews, and
package-local verification frame

This package defines CEMT (`.cemt`) resources. CEMT is the primary declarative
converter implementation language in the schema content registry design.

## Source Identity

Owned schema URI:

```text
https://cem.dev/ns/transform/cem/1
```

Primary content type:

```text
application/vnd.cem.transform+cem
```

CEMT source uses the CEM-ML document syntax, directive syntax, namespace
binding, text model, and Linux-style LF (`\n`) formatter output by default.
`lineEnding` is a generic formatter option; package-specific transform options
must only be added for transform-specific semantics.

CEMT reuses the CEM-native template module language. Source and target content
identity is not embedded in `.cemt`; it is declared by `package.cem` converter
edges so the same template execution surface can participate in registry
planning.

## Parser Facts And Diagnostics

CEMT parsing reuses the CEM-native template module parser and adds transform
function declarations for internal helpers, encoders, formatters, and
colorizers. The package schema declares the transform-facing element and
attribute contracts and the diagnostic codes for transform-specific policy:

- `cem.transform.converter_identity_missing`
- `cem.transform.converter_identity_mismatch`
- `cem.transform.template_base_invalid`
- `cem.transform.non_streamable_template`
- `cem.transform.function_name_unqualified`
- `cem.transform.function_identity_missing`
- `cem.transform.function_capability_missing`
- `cem.transform.function_shadowed_standard`
- `cem.transform.xpath_invocation_invalid`
- `cem.transform.xpath_binding_unresolved`
- `cem.transform_template.let_expr_invalid`

Current incomplete boundary: most transform declaration validation is still
structural schema-model validation, and several declared transform-specific
diagnostics remain target policy rather than executable schema-owned behavior.
The target shape is the same as CSV, CEM-QL, and CEM-native template: Rust
reports neutral parser/template/function facts with source ranges, and this
package's `.cem` schema owns code, severity, and structured details.

## Expression Schema Ownership

CEMT does not own a private expression language. It inherits CEM-native
template expression slots and delegates expression syntax, parse facts, type
facts, evaluator IR, and expression diagnostics to the shared CEM-QL expression
schema owned by `cem-ql/v1`. Transform-owned diagnostics remain for transform
function declaration contracts and output producer policy.

CEMT additionally owns an explicit XPath function-body form. This is a sibling
of the existing dollar-expression body, not a reinterpretation of a CEM-QL
slot:

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@ns vars = "urn:cem:variables"
@default transform

{module |
    {function @name="acme.select-by-index" @returns="any" |
        {body |
            {xpath @context="document" @sequence-type="node()" |
                {variable
                    @binding="index"
                    @namespace-uri="urn:cem:variables"
                    @local-name="index"}
                {expression | /root/n[$vars:index] }
            }
        }
    }
}
```

Each function has exactly one executable body form. The `xpath` form declares
an optional named context binding, zero or more named host bindings mapped to
expanded XPath variable names, and a required result sequence type with
optional cardinality bounds. Namespace prefixes used by the expression come
from the CEMT module's schema namespace declarations; variable declarations use
namespace URI plus local name so the runtime boundary never depends on a
prefix spelling.

The expression child is a lexical island compiled once into a CEMT-owned
`XPathExpressionAst` during template lowering. Its typed invocation descriptor
is retained in the compiled module. `invoke_transform_template_xpath` accepts
only native `XPathResultItem` context bindings and `XPathResultSequence`
variable bindings, invokes the XPath evaluator directly, and returns a typed
`XPathResultArtifact`. Runtime invocation does not read expression source,
reparse input, convert a generic CEMT value, or project bindings/results through
JSON or another DTO. Missing typed bindings fail with
`cem.transform.xpath_binding_unresolved` before evaluation.

Hosts can select an XPath-backed CEMT function explicitly by its exact compiled
function name through `invoke_transform_template_xpath_function`. The host
supplies the same native binding arena used by the lower-level invocation
adapter, and the dispatch result is a `TransformArtifactBody::XPathResult` that
retains the evaluator's typed result artifact and native node ownership. This
entrypoint does not infer aliases from renderer primary or secondary inputs,
fall back to a generic CEMT function body, or define authored CEMT call syntax.

## Output Artifacts

The package declares CEMT formatter and colorizer artifacts in `package.cem`.
The public formatter profile names are `compact`, `pretty`, and `tabular`.
The public colorizer profile names are `terminal`, `html`, and `md`.

Formatters produce formatted CEM trees, colorizers enrich those trees, and the
generic writer emits terminal, HTML, Markdown, or source bytes. Token arrays,
ANSI sequences, and HTML spans are writer-boundary implementation details.

Formatter profile behavior:

- `compact`: deterministic source-preserving CEMT output suitable for
  interchange once transform-specific compacting matures;
- `pretty`: indented review layout for transform modules and template output;
- `tabular`: currently aliases the review layout until transform declaration
  table alignment rules are implemented;
- `lineEnding=lf|crlf|preserve`: generic output line-ending control, default
  `lf`.

Colorizer profile behavior:

- `terminal`: semantic roles mapped to ANSI color output;
- `html`: semantic roles mapped to HTML color spans by the generic writer;
- `md`: reserved Markdown-oriented color role output for documentation
  pipelines.

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
content type, schema URI, category, output kind (`text`, `bytes`, `tokens`, or
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

## Safety Notes

CEMT modules describe generated output and may request imports through the
inherited template import surface. Passive validation and README preview
generation must not fetch imports, execute generated output, evaluate arbitrary
host expressions, or substitute unresolved resources without resolver-policy
approval. Output producer functions must treat raw syntax emission,
double-encoded artifacts, active content, and target-context escaping as
explicit policy boundaries.

## Formatter And Preview SDLC

When a command example, fixture, formatter, colorizer, CLI report shape, or
visible presentation output changes, regenerate the README examples with the
package `samples2readme` target. Valid UTF-8 CEMT sources remain exact fenced
source; an SVG preview is allowed only for an unfenceable fallback.

The package `verify` target checks generated README source and any referenced
fallback preview artifacts for drift.

## Verification

Focused package gates:

```bash
cargo test -p cem-ml cem_transform_package_examples_are_manifest_indexed
```

```bash
cargo test -p cem-ml-cli schema_owned_cem_transform_examples_validate_through_cli
```

```bash
yarn nx run cem_ml_schema_package_cem_transform_v1:verify
```

## Release Behavior

This package is versioned as `1.0.0`. The primary content type and schema URI
are compatibility anchors. Unsupported transform semantics should fail closed
through validation diagnostics rather than falling back to ordinary CEM or
CEM-native template behavior.

Tracked but not complete:

- schema-owned fact bindings for all transform parser and semantic diagnostics;
- distinct `compact` and `tabular` transform layout rules beyond the current
  deterministic review layout;
- HTML and Markdown output-parity checks once their transform presentation
  profiles become stable enough for executable examples.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-transform</summary>

- Source: [`examples/basic-transform.cemt`](./examples/basic-transform.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/basic-transform.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {template @name="main" |
        {body | Converted output.}
    }
}
```

<details>
<summary>module-transform</summary>

- Source: [`examples/module-transform.cemt`](./examples/module-transform.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/module-transform.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {import
        @as="shared"
        @src="shared.cemt"
        @content-type="application/vnd.cem.transform+cem"
        @schema="https://cem.dev/ns/transform/cem/1"
    }

    {param @name="items" @type="array"}

    {template @name="main" @visibility="public" |
        {body |
            {call @template="row" @with:item="current"}
            {section @class="summary" | Transform ready}
        }
    }

    {template @name="row" |
        {param @name="item" @type="object"}
        {body |
            {div @class="row" | Row output}
        }
    }
}
```

<details>
<summary>function-declarations</summary>

- Source: [`examples/function-declarations.cemt`](./examples/function-declarations.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/function-declarations.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {function
        @name="acme.normalize-callout"
        @visibility="private"
        @returns="object"
        @deterministic=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="marker" @type="string" @default="NOTE"}
        {body |
            {$ { kind: "callout", marker: $marker, value: $subject.value } }
        }
    }

    {encoding-function
        @name="html.text"
        @category="html-text"
        @subject="string"
        @produces="text"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="string" @required=true}
        {param @name="mode" @type="string" @default="canonical"}
    }

    {format-function
        @name="json.pretty"
        @category="json-document"
        @subject="object"
        @produces="tokens"
        @content-type="application/json"
        @schema="https://cem.dev/ns/data/json/1"
        @canonical=false
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="indent" @type="string" @default="    "}
    }

    {format-function
        @name="cem.format-tree"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
    }

    {color-function
        @name="cem.color-tree"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="css-custom-properties"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
    }

    {color-function
        @name="terminal.diagnostic"
        @category="terminal-color"
        @subject="tokens"
        @produces="text"
        @content-type="text/plain"
        @schema="https://cem.dev/ns/data/text/terminal/1"
        @canonical=false
        @streamable=true |
        {param @name="subject" @type="array" @required=true}
        {param @name="capability" @type="string" @default="auto"}
    }

    {encoding-function
        @name="acme.markdown.callout-block"
        @visibility="public"
        @implementation="cemt"
        @category="markdown-callout"
        @subject="object"
        @produces="tokens"
        @content-type="text/markdown"
        @schema="https://acme.test/ns/docs/markdown/1"
        @canonical=false
        @streamable=true
        @deterministic=true
        @extends="markdown-document" |
        {param @name="subject" @type="object" @required=true}
        {param @name="marker" @type="string" @default="NOTE"}
        {body |
            Markdown callout output.
        }
    }
}
```

<details>
<summary>formatter-coloring-pipeline</summary>

- Source: [`examples/formatter-coloring-pipeline.cemt`](./examples/formatter-coloring-pipeline.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `pass`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module @version="1.0.0" |
    {format-function
        @name="acme.showcase.format-node"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="slot" @type="integer" @default="0"}
        {body |
            {$ match($subject.kind, {
                element: call(acme.showcase.format-element, { subject: $subject, slot: $slot }),
                text: call(acme.showcase.format-text, { subject: $subject, slot: $slot }),
                default: $subject
            }) }
        }
    }

    {format-function
        @name="acme.showcase.format-element"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="slot" @type="integer" @default="0"}
        {body |
            {$ {
                kind: "element",
                name: $subject.name,
                sourceMap: $subject.sourceMap,
                attributes: $subject.attributes,
                children: map($subject.children, call(acme.showcase.format-node, { subject: $item, slot: $index })),
                formatLayout: {
                    kind: "format-decision",
                    formatterRole: match($subject.name, { strong: "formatter.inline-emphasis", default: "formatter.layout" }),
                    value: match($subject.name, { strong: "inline-emphasis", default: "inline" })
                }
            } }
        }
    }

    {format-function
        @name="acme.showcase.format-text"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="slot" @type="integer" @default="0"}
        {body |
            {$ {
                kind: "text",
                value: $subject.value,
                sourceMap: $subject.sourceMap
            } }
        }
    }

    {format-function
        @name="acme.showcase.format-tree"
        @visibility="public"
        @category="cem-tree"
        @subject="cem-ast-node"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @extends="cem.format-tree"
        @canonical=true
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="any" @required=true}
        {body |
            {$ appendFormatNode(
                {
                    kind: "cem-tree",
                    contentType: "application/cem",
                    schema: "https://cem.dev/ns/cem-ml/1",
                    category: "cem-tree",
                    mode: "fragment",
                    canonical: true,
                    formatterProfile: "acme.showcase.format-tree",
                    formatNodes: [{
                        kind: "format-marker",
                        name: "cem.format-tree",
                        formatterRole: "formatter.boundary",
                        formatterProfile: "acme.showcase.format-tree"
                    }],
                    nodes: match(exists($subject.kind), {
                        true: [call(acme.showcase.format-node, { subject: $subject, slot: 0 })],
                        false: map($subject, call(acme.showcase.format-node, { subject: $item, slot: $index }))
                    })
                },
                {
                    kind: "format-decision",
                    name: "showcase",
                    formatterRole: "formatter.showcase",
                    value: "formatted tree before writer"
                }
            ) }
        }
    }

    {color-function
        @name="acme.showcase.color-node"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="role" @type="string" @default="syntax.string"}
        {param @name="className" @type="string" @default="cem-color cem-color-syntax-string"}
        {body |
            {$ match($subject.kind, {
                element: call(acme.showcase.color-element, { subject: $subject }),
                text: call(acme.showcase.color-text, { subject: $subject, role: $role, className: $className }),
                default: $subject
            }) }
        }
    }

    {color-function
        @name="acme.showcase.color-element"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body |
            {$ {
                kind: "element",
                name: $subject.name,
                sourceMap: $subject.sourceMap,
                attributes: $subject.attributes,
                formatLayout: $subject.formatLayout,
                colorRole: match($subject.name, { strong: "syntax.keyword", default: "syntax.name" }),
                style: {
                    colorRole: match($subject.name, { strong: "syntax.keyword", default: "syntax.name" }),
                    colorProfile: "classes"
                },
                writerAttributeNodes: [{
                    kind: "writer-attribute",
                    name: "class",
                    value: match($subject.name, { strong: "cem-color cem-color-syntax-keyword", default: "cem-color cem-color-syntax-name" }),
                    colorizerOwned: true,
                    colorizerRole: "colorizer.writer-attribute",
                    colorProfile: "classes"
                }],
                children: map($subject.children, call(acme.showcase.color-node, {
                    subject: $item,
                    role: match($subject.name, { strong: "syntax.keyword", default: "syntax.string" }),
                    className: match($subject.name, { strong: "cem-color cem-color-syntax-keyword", default: "cem-color cem-color-syntax-string" })
                }))
            } }
        }
    }

    {color-function
        @name="acme.showcase.color-text"
        @visibility="private"
        @category="cem-tree-node"
        @subject="object"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {param @name="role" @type="string" @required=true}
        {param @name="className" @type="string" @required=true}
        {body |
            {$ {
                kind: "element",
                name: "span",
                colorRole: $role,
                style: {
                    colorRole: $role,
                    colorProfile: "classes"
                },
                writerAttributeNodes: [{
                    kind: "writer-attribute",
                    name: "class",
                    value: $className,
                    colorizerOwned: true,
                    colorizerRole: "colorizer.writer-attribute",
                    colorProfile: "classes"
                }],
                colorWrapperNodes: [
                    {
                        kind: "color-wrapper",
                        name: "span",
                        colorizerOwned: true,
                        colorizerRole: "colorizer.text-wrapper",
                        colorProfile: "classes"
                    },
                    {
                        kind: "color-decision",
                        name: "wrapped-role",
                        value: $role,
                        colorizerOwned: true,
                        colorizerRole: "colorizer.wrapped-role",
                        colorProfile: "classes"
                    }
                ],
                children: [$subject]
            } }
        }
    }

    {color-function
        @name="acme.showcase.color-tree"
        @visibility="public"
        @category="cem-tree"
        @subject="cem-tree"
        @produces="cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @extends="cem.color-tree"
        @profile="classes"
        @canonical=false
        @deterministic=true
        @streamable=true |
        {param @name="subject" @type="object" @required=true}
        {body |
            {$ applyEdits(
                appendWriterBoundary(
                    appendColorNode(
                        merge($subject, {
                            colored: true,
                            colorProfile: "classes",
                            colorNodes: [{
                                kind: "color-marker",
                                name: "cem.color-tree",
                                colorizerRole: "colorizer.boundary",
                                colorProfile: "classes"
                            }],
                            nodes: map($subject.nodes, call(acme.showcase.color-node, { subject: $item }))
                        }),
                        {
                            kind: "color-decision",
                            name: "showcase",
                            colorizerRole: "colorizer.showcase",
                            value: "colored tree before writer"
                        }
                    ),
                    {
                        kind: "writer-boundary",
                        stage: "after-color",
                        value: "writer consumes colored CEM tree"
                    }
                ),
                drainQueue(
                    defer([],
                        appendEdit(
                            "colorNodes",
                            {
                                kind: "color-decision",
                                name: "queued-edit",
                                colorizerOwned: true,
                                colorizerRole: "colorizer.queued-edit",
                                colorProfile: "classes",
                                value: "queued edit replay before writer"
                            }
                        )
                    ),
                    [],
                    append($acc, $item)
                )
            ) }
        }
    }
}
```

<details>
<summary>formatter-coloring-pipeline-fixture</summary>

- Source: [`examples/formatter-coloring-pipeline.fixture.cem`](./examples/formatter-coloring-pipeline.fixture.cem)
- Content type: `application/cem`
- Schema: `https://cem.dev/ns/cem-ml/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema.unresolved_namespace`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.fixture.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

````cem
@doc cem-ml 1
@ns showcase = "https://cem.dev/ns/showcase/1"
@default showcase

{cemt-output-pipeline-fixture
    @source="schema-packages/cem-transform/v1/examples/formatter-coloring-pipeline.cemt"
    @formatter="acme.showcase.format-tree"
    @colorizer="acme.showcase.color-tree"
    @color-profile="classes" |
    {stage
        @name="source-ast"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-fragment" |
```cem
{article |
    {text | Ready }
    {strong |
        {text | now}
    }
    {text | .}
}
```
    }

    {stage
        @name="formatted-cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-tree" |
```cem
{cem-tree @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1" @category="cem-tree" @mode="fragment" @canonical=true @formatter-profile="acme.showcase.format-tree" |
    {format-nodes |
        {format-marker @name="cem.format-tree" @formatter-role="formatter.boundary" @formatter-profile="acme.showcase.format-tree"}
        {format-decision @name="showcase" @value="formatted tree before writer" @formatter-role="formatter.showcase"}
    }
    {nodes |
        {article |
            {format-layout @kind="format-decision" @name="layout" @value="inline" @formatter-role="formatter.layout"}
            {text | Ready }
            {strong |
                {format-layout @kind="format-decision" @name="layout" @value="inline-emphasis" @formatter-role="formatter.inline-emphasis"}
                {text | now}
            }
            {text | .}
        }
    }
}
```
    }

    {stage
        @name="colored-cem-tree"
        @content-type="application/cem"
        @schema="https://cem.dev/ns/cem-ml/1"
        @category="cem-tree" |
```cem
{cem-tree @content-type="application/cem" @schema="https://cem.dev/ns/cem-ml/1" @category="cem-tree" @mode="fragment" @canonical=true @formatter-profile="acme.showcase.format-tree" @colored=true @color-profile="classes" |
    {format-nodes |
        {format-marker @name="cem.format-tree" @formatter-role="formatter.boundary" @formatter-profile="acme.showcase.format-tree"}
        {format-decision @name="showcase" @value="formatted tree before writer" @formatter-role="formatter.showcase"}
    }
    {color-nodes |
        {color-marker @name="cem.color-tree" @color-profile="classes" @colorizer-role="colorizer.boundary"}
        {color-decision @name="showcase" @value="colored tree before writer" @colorizer-role="colorizer.showcase"}
        {color-decision @name="queued-edit" @value="queued edit replay before writer" @color-profile="classes" @colorizer-role="colorizer.queued-edit"}
    }
    {writer-boundaries |
        {writer-boundary @stage="after-color" @value="writer consumes colored CEM tree"}
    }
    {nodes |
        {article @color-role="syntax.name" |
            {format-layout @kind="format-decision" @name="layout" @value="inline" @formatter-role="formatter.layout"}
            {style @color-role="syntax.name" @color-profile="classes"}
            {writer-attribute @name="class" @value="cem-color cem-color-syntax-name" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
            {span @color-role="syntax.string" |
                {style @color-role="syntax.string" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                {color-decision @name="wrapped-role" @value="syntax.string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                {text | Ready }
            }
            {strong @color-role="syntax.keyword" |
                {format-layout @kind="format-decision" @name="layout" @value="inline-emphasis" @formatter-role="formatter.inline-emphasis"}
                {style @color-role="syntax.keyword" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {span @color-role="syntax.keyword" |
                    {style @color-role="syntax.keyword" @color-profile="classes"}
                    {writer-attribute @name="class" @value="cem-color cem-color-syntax-keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                    {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                    {color-decision @name="wrapped-role" @value="syntax.keyword" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                    {text | now}
                }
            }
            {span @color-role="syntax.string" |
                {style @color-role="syntax.string" @color-profile="classes"}
                {writer-attribute @name="class" @value="cem-color cem-color-syntax-string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.writer-attribute"}
                {color-wrapper @name="span" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.text-wrapper"}
                {color-decision @name="wrapped-role" @value="syntax.string" @color-profile="classes" @colorizer-owned=true @colorizer-role="colorizer.wrapped-role"}
                {text | .}
            }
        }
    }
}
```
    }
}
````

<details>
<summary>invalid-missing-required-attribute</summary>

- Source: [`examples/invalid-missing-required-attribute.cemt`](./examples/invalid-missing-required-attribute.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_model.missing_required_attribute`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-missing-required-attribute.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {template |
        {body | Missing the required template name inherited from the native template schema.}
    }
}
```

<details>
<summary>invalid-function-missing-category</summary>

- Source: [`examples/invalid-function-missing-category.cemt`](./examples/invalid-function-missing-category.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_model.missing_required_attribute`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-function-missing-category.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {encoding-function
        @name="html.text"
        @subject="string"
        @produces="text"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1"
        @canonical=true
        @streamable=true |
        {param @name="subject" @type="string" @required=true}
    }
}
```

<details>
<summary>invalid-function-missing-contract-metadata</summary>

- Source: [`examples/invalid-function-missing-contract-metadata.cemt`](./examples/invalid-function-missing-contract-metadata.cemt)
- Content type: `application/vnd.cem.transform+cem`
- Schema: `https://cem.dev/ns/transform/cem/1`
- Expected result: `fail`
- Expected diagnostics: `cem.schema_model.missing_required_attribute`
- README rendering: fenced `cem` source

```bash
dist/target/cem_ml_cli/debug/cem-ml convert --input-spec \
  uri=packages/cem_ml/schema-packages/cem-transform/v1/examples/invalid-function-missing-contract-metadata.cemt,contentType=application/vnd.cem.transform+cem,schema=https://cem.dev/ns/transform/cem/1 \
  --to-content-type application/cem --to-schema https://cem.dev/ns/cem-ml/1 \
  --cemt-formatter-profile tabular --cemt-color-profile terminal --output-color-type \
  ansi-256
```

</details>

```cem
@doc cem-ml 1
@ns transform = "https://cem.dev/ns/transform/cem/1"
@default transform

{module |
    {encoding-function
        @name="html.text"
        @category="html-text"
        @subject="string"
        @produces="text"
        @content-type="text/html"
        @schema="https://cem.dev/ns/data/html/1" |
        {param @name="subject" @type="string" @required=true}
    }
}
```
