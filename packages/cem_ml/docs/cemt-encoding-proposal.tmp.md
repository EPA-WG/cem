# CEMT Encoding Proposal

Status: proposal for review

This note defines the proposed CEMT encoding capability before it is promoted
into the CEMT schema and runtime. The canonical documentation entry points are:

- [`../schema-packages/cem-transform/v1/README.md`](../schema-packages/cem-transform/v1/README.md)
- [`../schema-packages/README.md`](../schema-packages/README.md)

## Problem

CEMT can transform semantic data into target artifacts, but output serializers
need context-aware encoding that should not be hand-written in template text.
Examples include JSON string escaping, XML attribute escaping, HTML text-state
escaping, CSV field quoting, CSS identifier escaping, YAML scalar style
selection, and CEM binary chunk framing.

Encoding here means syntax/context encoding. It is separate from byte character
encoding such as UTF-8 or UTF-16 and separate from transport content encoding
such as gzip.

## Core Decision

CEMT is the primary output producer for schema-owned exports. Output production
includes transformation, encoding, formatting, terminal/HTML color output,
source-map span creation, and final artifact identity. Content-type-specific
encoders, formatters, colorizers, writer primitives, and small transformation
helpers are part of the CEMT stack, not an external post-processing layer.

```text
typed subject
  -> schema-owned CEMT output producer
    -> content-type-specific encode / format / color helpers
      -> encoded text, bytes, token stream, or chunk stream
        -> destination content type and schema
```

Native output producers remain necessary for performance, bootstrap, and clarity
for some content types. They are paired implementations, not replacements for
the CEMT contract. Every native producer should have a matching CEMT producer or
a planned CEMT producer, and shared fixtures must cross-check native output
against CEMT output. Differences must be explicit diagnostics or documented
lossiness/canonicalization choices.

The default architecture is therefore:

```text
package.cem serializer edge
  -> CEMT producer (primary)
  -> native producer (paired fallback or fast path)
  -> parity fixtures compare CEMT and native output
```

Rust fallback is allowed when a syntax profile is not yet expressible in CEMT,
when binary framing is required, or when performance requires a native writer.

## CEMT Stack Capabilities

The CEMT output stack should provide:

- encoder functions for context-specific escaping and binary framing;
- formatter functions for indentation, line endings, ordering, wrapping, scalar
  style, namespace declaration placement, and canonical output;
- color functions for semantic style roles, terminal ANSI/SGR output, HTML
  color output, no-color fallbacks, and accessibility-aware palettes;
- writer primitives for tokens, byte streams, sealed chunks, and source-map
  spans;
- schema helpers for target syntax rules, void/empty element policy, raw-text
  modes, namespace repair, identifier validity, and field/header policy;
- diagnostics for unsupported category, unsafe raw output, context mismatch,
  charset mismatch, unsupported color capability, lossy output, and
  native/CEMT parity mismatch.

These capabilities are called from CEMT templates and declared by schema package
metadata. They are not opaque host-side string filters.

## Function Call Declaration

Proposed expression-level function:

```text
encode(subject, target, options?) -> encoded-artifact
```

Minimum logical signature:

```text
encode(
  subject: any,
  target: {
    contentType: media-type,
    schema: uri,
    category: encoding-category,
    context?: encoding-context
  },
  options?: {
    mode?: "canonical" | "preserve" | "pretty" | "fragment",
    encoder?: qualified-name,
    formatter?: qualified-name,
    colorizer?: qualified-name,
    profile?: string,
    charset?: "utf-8" | "utf-16" | "utf-16be" | "utf-16le" | "us-ascii" | "other",
    lineEnding?: "lf" | "crlf" | "preserve",
    quote?: "auto" | "single" | "double" | "none",
    indent?: string,
    namespacePolicy?: "preserve" | "repair" | "canonical",
    sourceMap?: "preserve" | "generated" | "none"
  }
)
```

Expected CEMT expression form:

```cemt
{$ encode(
    $node.data,
    {
      contentType: "text/html",
      schema: "https://cem.dev/ns/data/html/1",
      category: "html-text"
    }
) }
```

The return value is not a plain string. It is an encoded artifact carrying:

- produced kind: `text`, `bytes`, `tokens`, or `chunks`;
- target content type and schema URL;
- encoding category and context;
- charset or binary framing identity;
- source-map policy and generated spans;
- a flag that prevents accidental second-pass encoding.

Template insertion must reject an encoded artifact when its target identity or
context is incompatible with the surrounding output context.

## Encoding, Formatting, And Color Function Declarations

Schema packages and shared modules should be able to declare named encoding,
formatting, and color output functions. Declaration metadata makes helpers
discoverable by the registry and validatable before execution.

Proposed CEMT declaration shape:

```cemt
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

Formatter declarations use the same pattern but produce formatting decisions or
formatted token streams:

```cemt
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
    {param @name="indent" @type="string" @default="  "}
}
```

Color declarations use the same pattern but produce target-specific styled
output from semantic style roles:

```cemt
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
    {param @name="palette" @type="string" @default="diagnostic"}
    {param @name="capability" @type="string" @default="auto"}
}
```

## Custom Encoding And Formatting Functions

CEMT should ship standard encoders, formatters, and color functions, but schema
packages and application packages must also be able to define custom functions.
Custom functions are how a package captures domain syntax rules, organization
style rules, proprietary wire formats, experimental AI context profiles, or
specialized canonicalization policies without forking the CEMT runtime.

Custom functions use the same declaration families as built-ins:
`encoding-function`, `format-function`, and `color-function`. The difference is
ownership and implementation source, not artifact semantics. A custom function
must still declare its subject type, output kind, content type, schema, category,
streamability, canonicality, params, and diagnostics. It must return the same
typed encoded artifact or formatting/color result that a standard function
returns; it must not return an untagged string that bypasses context checks.

Custom function names should be package-qualified. Standard CEM names are
reserved, and a custom declaration must not shadow a standard function unless
the import site explicitly aliases it. Registry lookup should therefore resolve
functions by `(owner package, name, contentType, schema, category, subject type,
profile)` rather than by short name alone.

Proposed custom declaration attributes:

- `@name`: package-qualified function name, such as
  `acme.markdown.callout-block`;
- `@visibility`: `public`, `package`, or `private`;
- `@implementation`: `cemt`, `native`, or `external`;
- `@profile`: optional named profile selected by `encode` options or package
  metadata;
- `@extends`: optional standard or custom function that this function wraps or
  refines;
- `@capability`: optional required host capability for native or external
  functions;
- `@deterministic`: whether the result is stable for the same input and options;
- `@trusted`: whether the function is allowed to emit raw fragments for a
  schema-gated context;
- `@fallback`: optional fallback function name when the preferred implementation
  is unavailable.

Example CEMT-authored custom encoder:

```cemt
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
        {$ write.token("blockquote-marker", "> ") }
        {$ encode($marker, {
            contentType: "text/markdown",
            schema: "https://cem.dev/ns/data/markdown/1",
            category: "markdown-text"
        }) }
        {$ write.token("text", ": ") }
        {$ encode($subject.message, {
            contentType: "text/markdown",
            schema: "https://cem.dev/ns/data/markdown/1",
            category: "markdown-text"
        }) }
    }
}
```

Example native-backed custom formatter:

```cemt
{format-function
    @name="acme.json.stable-api"
    @visibility="package"
    @implementation="native"
    @category="json-document"
    @subject="object"
    @produces="tokens"
    @content-type="application/json"
    @schema="https://acme.test/ns/api/json/1"
    @canonical=true
    @streamable=false
    @deterministic=true
    @capability="acme.native.JsonStableApiFormatter"
    @fallback="json.pretty" |
    {param @name="subject" @type="object" @required=true}
    {param @name="fieldOrder" @type="array" @required=false}
}
```

`encode` should be able to select custom functions in three ways:

- by explicit function/profile option when the caller knows the desired helper;
- by schema package metadata on a serializer edge;
- by registry resolution from content type, schema, category, context, subject
  type, and profile.

This implies extending the logical options shape with optional function selectors:

```text
options?: {
  encoder?: qualified-name,
  formatter?: qualified-name,
  colorizer?: qualified-name,
  profile?: string,
  ...
}
```

Custom function validation rules:

- The declared output identity must be compatible with the surrounding template
  output context before insertion.
- A custom function may call standard or imported custom functions, but every
  nested `encode` result must preserve its own identity and double-encoding
  guard.
- CEMT-authored functions may use writer primitives directly; native or external
  functions must declare capability, determinism, streamability, and fallback
  behavior.
- Raw output requires both a raw category and a trusted/schema-gated function.
- Public functions are versioned as package API. Breaking signature, output
  identity, canonicalization, or safety-policy changes require a package
  version boundary.
- Registry diagnostics must report ambiguous custom function resolution,
  missing capability, unavailable fallback, unsafe raw emission, non-determinism
  in a canonical profile, incompatible subject type, and incompatible produced
  kind.

For a schema-owned serializer, `package.cem` references the serializer template
edge, and the CEMT module declares or imports the encoders it uses:

```cem
{converter
    @id="ast-to-html"
    @implementation="cemt"
    @template="converters/ast-to-html.cemt"
    @template-content-type="application/vnd.cem.transform+cem"
    @template-schema="https://cem.dev/ns/transform/cem/1"
    @template-entrypoint="main"
    @streamable=true
    @lossiness="syntax-normalized"
    @readiness="planned"
    @rust-symbol="HtmlAstExportConverter"
    @fallback-reason="HTML writer primitives are not fully available in CEMT yet" |
    {from @content-type="application/vnd.cem.ast+cem-bin" @schema="https://cem.dev/ns/projection/ast/1"}
    {to @content-type="text/html" @schema="https://cem.dev/ns/data/html/1"}
}
```

## Subjects

The subject is the typed value to be encoded. It must be unencoded semantic
data, not a string that already contains target syntax unless the category is
explicitly `raw` and the caller accepts the policy risk.

Common subjects:

- scalar values: string, boolean, integer, number, null;
- names: local names, qualified names, namespace URIs, identifiers;
- structured values: arrays, maps, JSON values, YAML nodes, CSV rows;
- semantic nodes: CEM AST nodes, CEM DOM nodes, XML nodes, HTML nodes;
- token streams: already normalized parser or transform events;
- binary chunks: sealed CEM projection chunks;
- attributes and slots: name/value pairs with target-context metadata;
- fragments: document fragments whose output context is not a full document.

## Produced Values

Encoding produces a typed artifact. The produced artifact should be one of:

- `text`: UTF-8 text by default, with optional charset metadata;
- `bytes`: fully encoded byte sequence;
- `tokens`: target syntax or styled token stream for later writer composition;
- `chunks`: framed binary or streaming output chunks;
- `diagnostics`: non-output result when encoding is impossible or unsafe.

Produced artifacts must carry enough identity to validate downstream use:

- content type;
- schema URL;
- encoding category;
- formatter profile;
- color profile and capability;
- fragment/document mode;
- source-map spans;
- canonicalization mode.

## AI-Facing Context Output

AI-facing output should be a projection over the semantic CEM AST, DOM,
event-stream, schema registry, and token metadata, not a replacement for the
canonical projections. The canonical AST remains the lossless source of truth;
AI context output is a task-shaped view used for retrieval, tool calls,
summaries, and lazy expansion.

The goal is to minimize irrelevant context while preserving enough structure for
an AI consumer to act precisely. Smaller bytes are useful only when the format is
also easy for the target model or tool to interpret. A compact integer token
stream may be ideal for transport or indexing, but a model-facing surface usually
also needs a declared legend, stable names, source ranges, and expansion
references back to the canonical projection.

Useful AI-facing categories include:

- `ai-context-pack`: a bounded JSON or CEM object containing task-relevant
  nodes, summaries, diagnostics, schema identities, and source references;
- `ai-entity-graph`: named entities such as components, tokens, attributes,
  slots, converter edges, schemas, imports, and their relationships;
- `ai-semantic-tokens`: compact classified tokens with a declared legend,
  offsets/ranges, and optional source-map spans;
- `ai-context-fragment`: a subtree or event slice with neighboring context,
  source excerpt, and lazy expansion links;
- `ai-embedding-record`: normalized chunks and relationships for vector or graph
  indexes, with stable IDs back to AST/DOM/event projection nodes.

AI context encoders should support:

- budgets for nodes, tokens, characters, depth, diagnostics, and source excerpts;
- stable IDs and source ranges for exact edits and follow-up expansion;
- profile names such as `summary`, `navigation`, `refactor`, `token-authoring`,
  `diagnostic`, and `embedding`;
- lossiness metadata that distinguishes omitted detail from normalized detail;
- lazy expansion references to the canonical AST/DOM/event projection;
- host/tool metadata such as audience, priority, and cache identity when the
  encoded artifact is served through an agent protocol.

Example declaration:

```cemt
{format-function
    @name="ai.context-pack"
    @category="ai-context-pack"
    @subject="CemAstNode"
    @produces="tokens"
    @content-type="application/vnd.cem.ai-context+json"
    @schema="https://cem.dev/ns/projection/ai-context/1"
    @canonical=false
    @streamable=true |
    {param @name="subject" @type="CemAstNode" @required=true}
    {param @name="profile" @type="string" @default="summary"}
    {param @name="budget" @type="object" @required=false}
}
```

An AI-facing profile can be faster and more efficient for consumers when it
precomputes the entities and relationships the task needs, avoids full-tree
context dumps, and lets tools fetch exact subtrees on demand. It should not be
the only AST export, and it should be evaluated against representative agent
tasks because over-compressed or unfamiliar formats can cost more reasoning
tokens than they save.

## Terminal And HTML Color Output

Color output is part of CEMT output production, not a terminal-only afterthought.
CEMT should represent color semantically first, then encode it for a target
surface.

Common subjects:

- diagnostic spans and severities;
- syntax-highlight token streams;
- source excerpts with ranges;
- diff hunks and change categories;
- trace, planner, benchmark, and validation report records;
- schema element/attribute names and content-type identities.

Semantic style roles should be stable across targets:

- `diagnostic.error`, `diagnostic.warning`, `diagnostic.info`,
  `diagnostic.fatal`;
- `source.line-number`, `source.gutter`, `source.highlight`,
  `source.secondary-highlight`;
- `syntax.keyword`, `syntax.name`, `syntax.attribute`, `syntax.string`,
  `syntax.number`, `syntax.comment`, `syntax.raw`;
- `diff.add`, `diff.remove`, `diff.context`;
- `status.success`, `status.pending`, `status.muted`.

Terminal color output targets ANSI/SGR-capable text streams. The encoder must
support:

- capability modes: `none`, `ansi-16`, `ansi-256`, `truecolor`, and `auto`;
- environment policy such as no-color and forced-color;
- reset discipline so style does not leak past the produced artifact;
- optional hyperlinks only when terminal capability allows them;
- plain-text fallback that preserves meaning through labels and layout.

HTML color output targets `text/html` document or fragment output. The encoder
must support:

- class-based output for stable artifacts;
- optional inline style output only when explicitly requested;
- CSS custom-property palettes for themeable output;
- accessible contrast policy and non-color cues for diagnostics/diffs;
- escaped text content and attribute values using the same HTML encoders as
  ordinary HTML output;
- fragment-safe output that does not assume a full document wrapper.

Terminal and HTML color output share semantic style roles, but they are separate
encoding categories because their escaping, reset, accessibility, and artifact
identity rules differ.

## Native Pairing And Parity

Native producers are part of the design for performance and implementation
clarity. They should be used when:

- a content type needs a mature low-level writer before CEMT primitives exist;
- binary chunk framing needs native memory control;
- a serializer is performance-sensitive enough to justify a native fast path;
- a native writer makes edge cases clearer and can serve as an executable oracle
  for the CEMT implementation.

Native producers must be paired with CEMT producers:

- same source identity and target identity;
- same fixtures and expected diagnostics;
- same canonicalization/lossiness contract;
- comparison mode declared in package metadata: byte-exact, token-equivalent,
  parse-equivalent, or diagnostic-equivalent;
- drift reported as a parity diagnostic before a native fast path is promoted.

## Encoding Categories By Content Type Family

| Family | Content types | Encoding subject | Category examples | Produced value |
| --- | --- | --- | --- | --- |
| CEM-ML syntax | `application/cem`, CEM vendor `+cem` types | CEM AST node, name, attribute, text, directive, comment | `cem-document`, `cem-fragment`, `cem-name`, `cem-attribute-value`, `cem-text`, `cem-string-literal` | CEM text tokens or UTF-8 text |
| CEMT source | `application/vnd.cem.transform+cem` | CEMT module, template, expression text, call metadata | `cemt-module`, `cemt-template`, `cemt-expression`, `cemt-attribute-value` | CEMT source text |
| XML family | `application/xml`, `text/xml`, `application/xhtml+xml`, `image/svg+xml`, `application/mathml+xml`, `application/xslt+xml`, `application/relax-ng+xml` | XML node, QName, namespace binding, text, attribute value, comment, PI, CDATA | `xml-document`, `xml-element`, `xml-text`, `xml-attribute-value`, `xml-qname`, `xml-namespace`, `xml-comment`, `xml-cdata` | XML text tokens or bytes |
| HTML | `text/html` | HTML DOM node, text, attribute value, URL-ish attribute, raw-text/RCDATA text, foreign SVG/MathML node | `html-document`, `html-fragment`, `html-text`, `html-attribute-value`, `html-raw-text`, `html-rcdata`, `html-comment`, `html-foreign-content` | HTML text tokens or bytes |
| JSON family | `application/json`, `application/schema+json`, CEM projection `+json` debug views | JSON value, string, number, object member name, array/object | `json-document`, `json-value`, `json-string`, `json-member-name`, `json-number` | Canonical or pretty JSON text |
| YAML | `application/yaml`, `application/x-yaml`, `text/yaml`, `text/x-yaml` | YAML stream, document, scalar, sequence, mapping, tag, anchor | `yaml-stream`, `yaml-document`, `yaml-scalar`, `yaml-plain-scalar`, `yaml-quoted-scalar`, `yaml-block-scalar` | YAML text |
| CSV | `text/csv` | table, header, row, field | `csv-table`, `csv-record`, `csv-field`, `csv-header` | CSV text with configured delimiter, quote, and line ending |
| Markdown | `text/markdown` | Markdown document, inline text, code, link destination, table cell, embedded HTML policy marker | `markdown-document`, `markdown-text`, `markdown-code-span`, `markdown-fence`, `markdown-link-destination`, `markdown-table-cell` | Markdown text |
| CSS | `text/css` | stylesheet, rule, selector, declaration, identifier, string, URL token, custom property value | `css-stylesheet`, `css-identifier`, `css-string`, `css-url`, `css-declaration`, `css-selector` | CSS text |
| Terminal color text | `text/plain` with terminal color profile, future terminal-specific content type | diagnostic spans, source excerpts, syntax tokens, diff hunks, report records | `terminal-color`, `terminal-diagnostic`, `terminal-source`, `terminal-diff`, `terminal-syntax` | Plain text or ANSI/SGR text |
| HTML color output | `text/html` | diagnostic spans, source excerpts, syntax tokens, diff hunks, report records | `html-color-fragment`, `html-diagnostic`, `html-source`, `html-diff`, `html-syntax` | HTML fragment or document |
| CEM-QL | `application/vnd.cem.query+cem-ql`, `text/cem-ql` | query module, selector, string literal, identifier, parameter reference | `cem-ql-module`, `cem-ql-selector`, `cem-ql-string`, `cem-ql-identifier` | CEM-QL text |
| RELAX NG compact | `application/relax-ng-compact-syntax` | grammar, pattern, name class, literal | `rnc-document`, `rnc-pattern`, `rnc-name`, `rnc-literal` | RNC text |
| AI context projections | `application/vnd.cem.ai-context+json`, future `application/vnd.cem.ai-context+cem-bin` | CEM AST/DOM/event projection nodes, schema registry records, token metadata, diagnostics, converter edges | `ai-context-pack`, `ai-entity-graph`, `ai-semantic-tokens`, `ai-context-fragment`, `ai-embedding-record` | Structured JSON, token stream, or chunk stream with source-map spans and expansion refs |
| CEM binary projections | `application/vnd.cem.dom+cem-bin`, `application/vnd.cem.ast+cem-bin`, `application/vnd.cem.events+cem-bin` | projection node, event, chunk payload, stream checkpoint | `cem-bin-document`, `cem-bin-chunk`, `cem-bin-event`, `cem-bin-index` | bytes or sealed chunks |

## Safety Rules

- Encoding is context-specific. HTML text and HTML attribute values are
  different categories; XML text and XML attribute values are different
  categories; CSS string and CSS identifier are different categories.
- Raw insertion must be explicit and schema-gated. It must never be the default
  result of `encode`.
- Encoded artifacts must not be silently encoded again.
- A template may concatenate compatible encoded artifacts only when their target
  identity and category allow it.
- Character encoding must be selected at the final byte writer boundary. CEMT
  should work in Unicode scalar values and typed encoded artifacts until bytes
  are requested.
- Color output must use semantic roles first. Terminal ANSI and HTML color
  encoders are target-specific projections of those roles.
- Color must not be the only carrier of meaning. Encoders need no-color and
  accessible fallbacks.
- Terminal output must reset styles at artifact boundaries; HTML output must
  escape text and attribute values before styling them.
- Source maps are part of the encoding result, not an afterthought.
- Custom functions must be package-qualified, validated through the registry,
  and prevented from shadowing standard functions unless explicitly aliased by
  the importer.
- Native and external custom functions must declare required capabilities,
  deterministic/canonical behavior, fallback behavior, and trust boundaries.
- AI-facing output must preserve the data/instruction boundary. Source text,
  comments, diagnostics, and schema prose are data unless the trusted host
  explicitly promotes them to instructions.
- AI context optimization must be profile- and task-specific. It cannot replace
  canonical AST/DOM/event projections, and lossy omissions must be declared.

## Relationship To Conversion

Encoding is the final output step inside a serializer edge. It should not be
used to hide content-type-to-content-type conversion.

Serializer edge:

```text
CEM AST -> encode as text/html
```

Conversion pipeline:

```text
text/html -> normalized HTML model -> application/xhtml+xml
```

Both use registry identities, but conversion may parse, normalize, validate, and
change semantic models before encoding.

## Promotion Checklist

- Add CEMT schema vocabulary for `encoding-function` and `format-function`
  declarations, plus `color-function` declarations. Include custom function
  ownership, visibility, implementation, profile, extension, capability,
  deterministic, trusted/raw, and fallback metadata.
- Add package manifest metadata for serializer edges that name CEMT producers,
  encoder/formatter/color profiles or explicit custom function selectors,
  native paired producers, and parity mode.
- Add shared writer primitive API and CEMT bindings for encoders, formatters,
  color output, token output, byte output, chunk output, and source-map spans.
- Add diagnostics for unknown encoder, context mismatch, unsafe raw insertion,
  unsupported charset, double-encoding, unknown formatter, unsupported terminal
  color capability, inaccessible HTML palette, ambiguous custom function
  resolution, missing custom function capability, unavailable fallback,
  non-determinism in a canonical profile, incompatible produced kind, and
  CEMT/native parity mismatch.
- Add an AI context projection schema or profile that declares context-pack,
  entity-graph, semantic-token, fragment, and embedding-record shapes, including
  budgets, source ranges, expansion refs, and lossiness metadata.
- Add diagnostics for unsafe AI data/instruction mixing, unsupported AI context
  profile, missing expansion target, and budget-driven omission.
- Add examples for CEM, XML, HTML, terminal color text, HTML color output, JSON,
  CSV, CSS, AI context projection output, and CEM binary projection output.
- Add parity tests comparing CEMT producers with native paired producers.
- Add task fixtures or evals for AI context profiles so compact forms are
  accepted only when they improve retrieval, edit precision, or token budget.
