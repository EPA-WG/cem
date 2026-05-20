# `cem-ml` Stack Design

**Status:** Draft high-level design derived from the primary acceptance criteria.
Implementation contracts are split into
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md).
**Primary acceptance criteria:** [`cem-ml-ac.md`](cem-ml-ac.md)
**Architectural research source:** [`parsing-algorithms-research.md`](../parsing-algorithms-research.md)
**Date:** 2026-05-08

---

## 1. Purpose And Scope

This document translates the layered parser architecture from `parsing-algorithms-research.md`
into a concrete design for the `cem-ml` Rust library and `cem-ml-cli` binary. It fixes:

- functional layer boundaries and module ownership,
- algorithm selections for each layer,
- Rust module topology,
- source-map, diagnostic, report, and projection responsibilities,
- the Tier A MVP scope, and
- open design decisions that must be resolved before implementation begins.

[`cem-ml-ac.md`](cem-ml-ac.md) is the primary decision driver. This document translates
those acceptance criteria and the architectural research into high-level design:
behavior, tiers, module responsibilities, and layer boundaries for `cem-ml` work.
Concrete interface sketches, struct shapes, projection keys, and file-level
implementation ownership live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md). If this design conflicts
with the AC, the AC wins until this design is corrected or an unresolved ambiguity is
recorded in both documents.

### 1.1 Acceptance Criteria Alignment Policy

Acceptance criteria are the source of truth. The design documents must explain how each
AC item is satisfied, deferred, or blocked by an explicit open question. Design updates
may propose corrections, but once the AC is aligned, implementation planning follows the
AC until a new AC revision accepts or rejects that proposal.

Resolved design reasoning has been folded back into the AC as of the current alignment
pass: CEM-native schema source, RELAX-NG structural parity, streaming-first async
public APIs, context-scope error policy, origin-first source-map stacks,
schema-qualified CEM annotations, schema-driven CEM template transforms, canonical
curly CEM-ML output with XML/HTML parity surfaces, document-format version identity,
and deferred DOM mutation runtime scope are AC decisions.

---

## 2. Domain Context

Canonical CEM-ML source uses the curly-brace surface from
[`cem-ml-syntax.md`](cem-ml-syntax.md). XML and HTML remain secondary parity surfaces:
they must lower into the same event model and AST, but they do not replace the CEM-ML
source form as the authoring canonical.

Top-level canonical CEM-ML source begins with `@doc cem-ml <version>` per AC-F-8.
This is document-format identity, not schema identity: it selects the CEM-ML language
grammar and parser compatibility family before schema loading starts. Tier A supports
embedded document-format version `1.0.0`; author-facing `@doc cem-ml 1` is the normal
shorthand. Embedded CEM-ML fragments inherit an already-established document-format
identity unless the host API supplies one explicitly. XML and HTML parity inputs get
their format identity from the selected parser/content-type profile and do not accept
`@doc`.

Schema-qualified CEM attributes are transformation annotations in the namespace
associated with the active schema. They are not HTML `data-*` metadata and not
replacements for native HTML element meaning. The `cem:` prefix below is illustrative;
the active schema owns the concrete namespace binding:

```cem
@doc cem-ml 1
@ns cem = "https://cem.dev/ns/core/1"
@ns html = "http://www.w3.org/1999/xhtml"
@default html

{main @cem:screen="login" @aria-labelledby="login-title" |
  {h1 @id="login-title" | Sign in}
  {form @cem:form="sign-in" @method=post @action="/session" |
    {label @for=email | Email}
    {input @id=email @name=email @type=email @required}
    {button @type=submit @cem:action=primary | Sign in}
  }
}
```

The canonical fixture set lives under `examples/cem-ml/` and mirrors the five existing
HTML parity fixtures in `examples/semantic/` (login, registration, profile,
assets-list, message-thread). The pipeline must:

1. Read raw source bytes, detect/decode encoding, and preserve byte offsets throughout.
2. Tokenize canonical CEM-ML curly syntax, or a selected XML/HTML parity profile, into
   the same schema event vocabulary.
3. Normalize tokens into a cross-format event stream (open, close, name, value, ...).
4. Validate event structure against the CEM schema using a RELAX NG-equivalent
   structural IR.
5. Handle embedded content (inline `<style>`, `style=""` attributes, CEM-ML typed
   scopes, and raw-text `<script>` handoffs) through an explicit handoff stack.
6. Reconstruct the schema-defined token hierarchy as the initial input DOM/AST with
   source-map stacks and reference slots on every node.
7. Apply content-type transformations over that input DOM/AST. For HTML, WHATWG DOM
   compliance is a schema-driven transformation from the initial HTML parser DOM to the
   implementation DOM.
8. Transform the CEM AST projection to canonical CEM-ML and to rendered light-DOM
   custom-element markup (`<cem-screen>`, `<cem-form>`, etc.).

---

## 3. Pipeline Overview

![Architectural blueprint illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-architectural-blueprint.png)

*Architectural blueprint.*

```
ByteSource
  └─ EncodingDecoder
       └─ SchemaTokenizer              (CEM-native curly, WHATWG HTML, or XML 1.0 profile)
            └─ EventNormalizer
                 └─ SchemaMachine      (RELAX NG derivative frame stack)
                      ├─ [HandoffStack ──> child SchemaTokenizer / SchemaMachine]
                      └─ InputDomAstBuilder
                           (schema-defined initial DOM/AST + source maps)
                           └─ ContentTypeTransformPipeline
                                (WHATWG HTML DOM update, CSS/SCSS transforms, CEM projection)
                                └─ [BinaryAstEncoder]  ← deferred Tier B
                                     └─ [ChunkCompressor]  ← deferred Tier B
                                          └─ ImplementationInterpreter
                                               (CEM transform → custom-element markup)
```

Layers in brackets are designed for stable interfaces but not implemented in Tier A.
For Tier A, all public Rust and WASM entry points are asynchronous. The implementation
parses chunk streams monotonically; tokenizer accumulation is token-local and released
after token emission. Callers interact with futures/streams only and no synchronous
parser API is exposed. The encoder, compressor, and broadcast/cache paths are absent.

The schema machine instantiates a child pipeline (tokenizer → normalizer → schema
machine) for each embedded content region. The handoff stack controls the boundary and
return condition.

### 3.1 Resource Limit Policy

![Streaming data corridor illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-streaming-corridor.png)

*Streaming data corridor.*

Depth, count limits, and error-boundary behavior are defined by the active context-scope
policy. The document top-level scope establishes the initial policy. Every parser,
handoff, transform, and embedded-content context runs inside a context scope with an
effective policy. Inner scopes may redefine their own policy downward, including relaxed
error handling, diagnostic hiding, scope-local recovery, stricter resource limits, or
content-type-specific validation behavior.

The parent/owner scope remains authoritative for the effective policy. A parent can
allow an embedded context to hide or relax its own errors, or it can override that inner
policy by enforcing severity floors, fail-closed behavior, diagnostic visibility,
resource ceilings, allowed schemes, or full-parse abort behavior. This means embedded
contexts can be locally permissive only within the envelope allowed by their owner
context.

Resource behavior is therefore policy-inherited, not globally monotonic. A child
content-type policy may lower or raise local limits and may downgrade local diagnostics
when the parent allows it. A parent policy can still impose non-relaxable ceilings or
failure criticality for document-level protections.

When a limit is exceeded, the effective policy determines whether the parser records a
recoverable diagnostic and continues in degraded mode, aborts the current scope, or
aborts the full parse.

Resource ceilings for transform-owned loading graphs use the same context-scope policy
model. Fetch count, redirect depth, byte count, reference depth, timeout, and allowed
schemes are scope-policy fields, not tokenizer behavior.

### 3.2 Unsafe Content And URL Policy

Unsafe-content diagnostics are owned by content-type transformation policies, not by the
tokenizer. The tokenizer preserves tokens and exact byte ranges, the schema machine
validates structure, and the input DOM records URL-bearing attributes and inline content
with source maps. URL-bearing fields are then resolved by the active transformation
policy against the owner context's base URL, module or import map, and substitution rules.
External resources such as XML DTDs follow the same rule: the matching content-type
transform owns fetch initialization, parsing, transformation, and application back to the
originating context. They are modeled like CSS or JavaScript loading graphs, not as
tokenizer-owned side effects.

When no transform has opted into handling an external construct, the active scope policy
determines whether the construct is rejected, diagnosed, or preserved. If the policy does
not reject it, the construct is kept as an unresolved resource slot for later use by a
matching transform or caller.

After resolution and before any fetch, execution, embedding, or output materialization,
the same policy applies security restrictions such as allowed schemes, `javascript:`
rejection, same-origin requirements, inline-event handling, `srcdoc` handling,
form-action constraints, and allowed embedded content types.

Each parse or handoff scope has an effective policy inherited from its owner context.
Embedded contexts may redefine local URL mappings, substitutions, diagnostics, and
security handling only within the envelope permitted by the owner. The outer context may
override inner URL mappings, substitutions, diagnostic visibility, error severity,
resource limits, and security restrictions; inner scopes cannot override parent mappings
in a way that the parent policy forbids.

---

## 4. Source-Map Model

Source maps are a functional contract of the AST, not a diagnostic side table. Every
source-derived node must carry a source-map stack that can trace from the current node
back to the original byte range that produced it. Transform-generated nodes inherit the
nearest owning source range and add a transform frame for the operation that created
them.

The coordinate model is byte-first:

- `ByteRange` is the durable location primitive: absolute byte offset plus byte length.
- Line and column are reporting/tooling projections from a selected source-map frame's
  `SourceId` and `ByteRange`; they are not parser semantics and are never stored as node
  identity.
- Bit-level ranges are reserved for deferred binary/compressed content and are not
  populated in Tier A.

When a source-map stack crosses tokenizer, normalizer, schema, handoff, transform, and
render frames, each frame may refer to a different source stream. Report renderers and
compiler-style tooling choose which frame to project: author-facing diagnostics usually
project the origin/input frame, while transform debugging may project the current
generated or intermediate frame. The line and column values can therefore differ across
frames without changing the canonical byte-range mapping.

The stack records the processing chain: tokenizer, event normalizer, schema validation,
CEM AST builder, handoff boundaries, implementation transforms, and future binary
encoding. This lets tooling resolve from generated custom-element output back to the raw
HTML token or embedded resource that produced it.

Source-map frames are ordered origin-first. `frames[0]` is the original source frame for
the stack, transforms append new frames as the node moves through the pipeline, and
`frames.last()` is the current frame. Generated nodes inherit the producer node's stack
and append the transform frame that created them.

Each source-map frame carries a `FrameSpan`: `Single(ByteRange)` for the common one-span
case or `Multi(Vec<ByteRange>)` for merged or split source spans. `Multi` range order has
no semantic meaning; consumers use each range directly to project to the proper location
in the source stream. Generated nodes use the nearest owning range with the transform
frame that created them. Reference inlining and external-resource resolution are modeled
as boundary frames whose target has its own source-map stack, matching embedded handoff
behavior instead of nesting source maps inside a span.

Layer 1 owns byte encoding only. Language-local encodings recognized by a tokenizer,
such as HTML character references, XML entity references, CSS escapes, JSON string
escapes, or CSV quoted escapes, append an `EscapeDecoded` source-map frame that maps
decoded scalar ranges back to the raw byte ranges that produced them. This keeps local
decoding with the language tokenizer that recognizes it while preserving exact source
positions for reports and transforms.

Detailed `ByteRange`, `SourceMapStack`, `SourceMapFrame`, `TransformKind`, diagnostic,
and traversal shapes live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#2-shared-source-map-and-diagnostic-contracts).

---

## 5. Layer 1 - ByteSource And EncodingDecoder (`cem_ml::source`)

### Purpose

Consumes raw byte chunks and owns encoding detection. Preserves absolute byte offsets
through all subsequent layers.

### Functional Design

Layer 1 presents every input as a stream initiation with a stable `SourceId`, monotonic
absolute byte offsets, decoded scalar spans, and streaming line-index metadata. It does
not own parser-wide scan storage or expose lexer-oriented storage. Retained bytes are
limited to transport chunks and the small decoder carry needed to finish an encoding unit
across chunk boundaries.

Rules from the research:

- Keep absolute `u64` byte offsets for every token and event.
- Keep decoded scalar spans alongside scalars for Unicode-aware validation.
- Preserve enough current-chunk bytes to report diagnostics emitted at that point; source
  maps retain offsets, not source text.
- Decode each byte source into a Unicode scalar stream before tokenization. Tokenizers
  consume decoded Unicode scalars, never raw encoding-specific code units.
- Treat BOM detection as byte-stream initiation. If the first bytes of a source
  initiation are a supported BOM, the BOM determines the source encoding, the BOM bytes
  are skipped from the decoded scalar stream, remain addressable by source-map byte
  ranges, and later encoding overrides for that source are ignored.
- If no BOM is present, use the explicit/default encoding parameter supplied with the
  source. Browser/server callers may derive this value from transport metadata such as
  `Content-Type`; library callers pass it through parser configuration. If neither a BOM
  nor a supplied encoding exists, default to UTF-8.
- Inline embedded contexts receive source-mapped decoded streams from their owner and do
  not perform BOM detection. External or separately loaded resources are new byte-source
  initiations and apply the same BOM/default-encoding precedence independently.
- In-band encoding declarations discovered after decoding, including HTML metadata or
  content-type-specific encoding switches, do not force the current source to be
  re-decoded. If policy allows such a declaration to initiate or configure a later child
  byte stream, it supplies that child stream's explicit/default encoding parameter; a BOM
  on the child source still wins over that parameter.
- Content-type preprocessing replacements, including HTML-specific replacement rules,
  occur after decoding on Unicode scalars in the owning tokenizer or transform, not on
  raw bytes in Layer 1. Replacement-produced scalars retain source ranges pointing at
  the original bytes. An isolated UTF-8 BOM is accepted silently and excluded from the
  decoded scalar stream while byte ranges continue to address the original bytes.

Tier A enforces resource bounds while streaming. The default limits are:

- maximum total bytes per `SourceId`: **64 MiB**, enforced cumulatively across chunks;
- maximum transport chunk: **64 KiB** recommended default for file/WASM adapters;
- maximum decoder carry: **4 bytes** for UTF-8 boundary completion, or **4 bytes** for
  UTF-16 surrogate/BOM boundary completion;
- maximum tokenizer token buffer: **64 KiB** before `cem.token.too_large`;
- maximum decoded scalar chunk: **64 Ki scalars** before downstream backpressure;
- maximum line count: **8 M** line starts, stored as streaming line-index checkpoints;
- maximum diagnostic snippet: **240 bytes** before/after the offending range, truncated
  at the nearest line boundary, total snippet <= **1 KiB**;
- maximum `SourceMapStack` frames: **32**;
- maximum AST depth: **1024**;
- maximum diagnostics per source: **10 000**, followed by one
  `cem.diagnostics.truncated` event.

Limit breaches emit fatal source/decode/token diagnostics such as
`cem.source.too_large`, `cem.decode.carry_too_large`, or `cem.token.too_large` before
downstream layers allocate unbounded state. The limits apply to the original byte stream
and per-layer streaming accumulators. Decoded scalars are chunked output, not a complete
secondary buffer.

### Tier A Scope

Tier A accepts owned byte buffers, strings, file paths, and async byte or string streams
only through asynchronous source adapters. Byte and string inputs are finite stream
adapters; file paths are opened as chunked streams; WASM inputs use
`ReadableStream<Uint8Array | string>` where available. The parser consumes chunks
monotonically through decode, tokenization, normalization, schema validation, and AST
construction. Tokenizer buffering is token-local: bytes/scalars are accumulated only
until the current token is resolved, then released after token/event emission. Source-map
ranges carry absolute offsets rather than retained source bytes. Tier B adds
editor-style incremental reparse and resumable chunk graphs; Tier A is still streaming
for a single parse pass.

Implementation interfaces are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#31-layer-1-bytesource-and-encodingdecoder-cem_mlsource).

---

## 6. Layer 2 - SchemaTokenizer (`cem_ml::tokenizer`)

### Purpose

Converts decoded input into format-native tokens. The tokenizer is mode-aware and
schema-guided: it knows which lexical modes, embedded boundaries, and delimiter patterns
the schema defines. General structural validation, hierarchy reconstruction, and
document/scope semantic checks remain downstream. Schema-owned lexical/mode,
embedded-boundary, or local token diagnostics may execute in the tokenizer when the
compiler places those rules there.

### Functional Design

For canonical CEM-ML, Tier A tokenizes the curly-brace surface from
[`cem-ml-syntax.md`](cem-ml-syntax.md): `{name @attributes | content...}` nodes, `$`
expression nodes, anonymous typed scopes, directives, comments, and rich-content
enclosures. This tokenizer is the primary authoring path.

For HTML parity, Tier A uses a custom WHATWG-state tokenizer and emits source-spanned
tokens. The custom tokenizer is selected over wrapping an existing Rust HTML5 tokenizer
because CEM requires exact source-map preservation across nested embedded contexts,
decoded handoff streams, token envelopes, attribute names, attribute values, text runs,
and raw text return boundaries. It does not construct either the source-preserving
initial HTML parser DOM or the WHATWG implementation DOM. The schema-defined token
hierarchy is reconstructed later by the input DOM/AST builder, and WHATWG DOM
compliance is applied as a content-type transform.

The tokenizer is the only layer that owns token assembly buffering. It consumes decoded
scalar chunks, keeps the partial bytes/scalars needed for the current token and any
format-defined lookahead, and releases that buffer as soon as the token is resolved and
emitted. There is no source-owned scan buffer and no tokenizer dependency on any specific
memory-buffer model. Implementations may use safe indexed access, a ring buffer, or
another bounded strategy as long as streaming order, source ranges, and token size limits
are preserved.

The schema can select valid tokenizer contexts and embedded-content boundaries, but it
does not rewrite CEM-ML, WHATWG, or XML lexical behavior or make the tokenizer the
semantic source of truth. XML follows the same layer contract with an XML 1.0 profile so
Layers 3 and above can consume a format-agnostic event stream.

XML constructs that require external resources or compatibility behavior, including DTDs,
entities, notation declarations, and XInclude, are delegated to the XML content-type
transform. Entity expansion is XML-specific and is not a CEM-ML primitive; CEM-ML
reference resolution uses slots and inlined references without cloning referenced content
into the originating tree.

Token shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#32-layer-2-schematokenizer-cem_mltokenizer).

---

## 7. Layer 3 - EventNormalizer (`cem_ml::events`)

### Purpose

Converts format-native tokens into a small, cross-format set of normalized event
categories. This is the unification point: the schema machine consumes the same event
taxonomy regardless of input format.

### Functional Design

The normalized taxonomy is:

- open scope;
- close scope;
- name;
- scalar value;
- trivia;
- processing instruction;
- separator;
- mode switch for embedded content;
- error.

For HTML, each start tag emits an open scope followed by name/value events for each
attribute, preserving attribute source positions and attribute case, then
`Separator { kind: ElementBoundary }` at the start tag closing delimiter. End tags emit
close scopes. Text emits scalar text values. Comments and whitespace are preserved by
default as source-bearing input nodes or trivia events unless the effective document or
context scope policy says to strip them. Parse errors become diagnostic events.

`ElementBoundary` closes only the lexical start-tag attribute segment. It does not mean
that every schema-owned header parameter for the element is complete. After the start
tag, the active schema may declare a leading header/prelude region whose child scopes
contribute effective attributes, namespace bindings, or other scope parameters before
real body content begins. Examples include schema-level forms such as
`<ATTRIBUTE name="abc">value</>` or `<NAMESPACE name="xyz" url="..."/>` at the beginning
of an element body. These are parsed as child scopes with source ranges of their own,
then applied to the parent frame's effective parameter state by the schema machine.
Header namespace declarations update the active namespace context from their source
position forward before later names are resolved; previously emitted name-resolution
records remain immutable.

The schema machine closes the header/prelude region when the first non-header body event
arrives, or when the element closes. That boundary is schema-state, not a tokenizer
event: the body-start event is first used to finalize header requirements and namespace
scope updates, then consumed as normal content. This keeps start-tag syntax, schema
parameters, and body content separate without requiring lookahead in the tokenizer.

The tokenizer reports lexical facts such as a `self_closing` marker and raw token
spans. It does not decide whether `/>` is meaningful in the current namespace. The
event normalizer, using the active schema frame, emits explicit `CloseScope` events when
the active content-type/schema rules determine that an element's source scope has
closed:

- XML self-closing tags in self-closing-capable contexts and HTML void elements close
  immediately after `ElementBoundary`; the synthetic close uses the start tag's closing
  delimiter range and carries `Synthesis::SelfClosing` or `Synthesis::VoidElement`.
- A self-closing marker on a non-void HTML element emits `cem.html.invalid_self_close`;
  the element remains open unless another close rule applies.
- A start tag that cannot be embedded in the currently open HTML element closes that
  existing element before the new `OpenScope`; for example, `<p><p>` emits a synthetic
  close for the first `p` before opening the second.
- When policy allows recovery and an end tag closes an ancestor while descendants are
  still open, descendant scopes are closed first using
  `Synthesis::ImpliedByAncestorClose`; strict XML-compatible profiles may instead make
  the misnested close fatal.
- EOF recovery may close remaining open scopes with `Synthesis::ImpliedByEof`.

HTML processing instructions are schema-driven. The tokenizer preserves their source
range; the active schema or context-scope policy decides whether they are accepted,
diagnosed, transformed, or stripped. Context-type entries such as `<pre>`, `<textarea>`,
`<style>`, and `<script>` create or select scopes with their own whitespace/comment
preservation policy. Whitespace in a context where it is content remains content; other
whitespace can be preserved as trivia.

HTML `data-*` attributes are normalized into the synthetic `cem:html-data` namespace and
may be projected to the HTML-specific `dataset` equivalent on HTML AST nodes. CEM
transform annotation attributes are schema-qualified names, arrive as name/value events
within the element's open-scope group, and are handled by the schema machine.

The event enum and token mapping table are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#33-layer-3-eventnormalizer-cem_mlevents).

---

## 8. Layer 4 - SchemaMachine (`cem_ml::schema`)

### Purpose

Validates the normalized event stream against the CEM schema incrementally. Maintains a
push/pop stack of schema frames. CEM structural schema semantics must have functional
parity with RELAX NG validation. Tier A may execute a limited structural subset through a
compiled DFA profile, but that profile is generated from the same RELAX-NG-equivalent
CEM schema IR and must preserve the full derivative runtime path.

### Algorithm Selection

From the research algorithm comparison table:

- **Nested events -> visibly pushdown frame stack.** Start tags push frames, end tags pop
  frames, and attributes/text update the current frame.
- **Schema validation -> RELAX NG-compatible structural IR.** Full validation semantics
  are RELAX NG-equivalent and derivative-computable. After each event, a full derivative
  runtime computes the residual schema `D(event, schema)`; Tier A may instead use a
  limited DFA transition generated from the same structural IR. If the residual or DFA
  state is the empty language, emit a hard error. Residuals and DFA follow sets provide
  engine-specific expected-content diagnostics; report compatibility between the Tier A
  DFA and a later derivative runtime is not a compatibility requirement.

### Functional Design

The schema machine validates scope opens, scalar values, separators, embedded handoffs,
and closes. It records each diagnostic at the scope where the error originates, then
bubbles the diagnostic to the nearest error-boundary scope. An error-boundary scope is
either declared explicitly by the active schema or, when no explicit boundary exists, the
nearest active context root. A context root is a scope frame, not necessarily the
document root; embedded content, option/prelude scopes, namespace-declaration scopes, and
schema-declared contexts may sit in the middle of the tree and own the effective policy
for diagnostics emitted inside them. The boundary scope's effective policy decides
whether to hide, report, recover, abort the boundary scope, or abort the full parse. The
document scope is only the topmost error-boundary context; engine defaults, CLI
parameters, or config seed its policy before parsing, and descendant scopes inherit or
redefine that policy within parent override bounds. The effective policy maps stable
diagnostic codes to severity and can upgrade a recoverable warning or error to
fatal/fail-fast behavior.

When policy allows recovery for a non-fatal schema error, the machine pushes an
`ErrorSubtree` frame. The frame accepts child events until the matching `CloseScope`
while still preserving well-formedness checks such as balanced opens/closes and scalar
value minimums. On the matching close, the machine pops the error frame and resumes the
parent frame unchanged; the parent structural state is not broadly permissive and is not
corrupted by the rejected subtree. AST nodes built inside the recovered subtree carry
`tainted: true` so downstream transforms can skip or specially handle them.
Open-scope and element-content recovery use `ErrorSubtree`; attribute-phase name/value
errors are recorded and the attribute phase continues unless policy escalates. If an
unknown `OpenScope` has no schema-provided expected close, recovery waits for the
matching lexical close; if the construct is void-like and has no close, recovery ends at
the parent close. Validated events are passed to the input DOM/AST builder.

A schema frame owns the active schema id, content type, expected close, namespace
context, source-map stack, diagnostics, effective scope policy, and explicit validation
phase. Validation phases distinguish lexical start-tag attributes, optional schema
header/prelude declarations, body content, and closed scopes. Attribute validation and
child-content validation use distinct trackers: `AttributeState` stores active effective
attributes, namespace/parameter declarations, and required header items that remain;
`ContentState` stores the residual or DFA state, diagnostic-relevant seen children in
emit order, and required children that remain. Attribute multiplicity is normalized to
0..1 per expanded name by last-writer-wins override; multi-valued attribute semantics
are value-shape checks. Child scopes that the schema marks as header/prelude declarations
are consumed before body content and update `AttributeState` or the namespace context
instead of being counted as body children. Child multiplicity and ordering are encoded
in the residual or DFA state, with
`required_remaining_children` kept as a diagnostic mirror for close-time messages.

Constraint checks have fixed trigger events:

| Constraint                 | Trigger                              | Frame phase |
|----------------------------|--------------------------------------|-------------|
| Duplicate attribute        | `Name`                               | Attribute; later value overrides earlier value |
| Unknown attribute          | `Name`                               | Attribute   |
| Bad attribute value        | attribute `Value`                    | Attribute   |
| Required lexical attribute missing | `Separator { kind: ElementBoundary }` | Attribute -> Header |
| Required header parameter missing | first non-header body event or `CloseScope` | Header -> Content/Closed |
| Unexpected child element   | `OpenScope`                          | Header or Content |
| Unexpected text content    | text `Value`                         | Content     |
| Bad child ordering         | `OpenScope`                          | Content     |
| Multiplicity exceeded      | `OpenScope`                          | Content     |
| Required child missing     | `CloseScope`                         | Content -> Closed |
| Unclosed scope             | EOF                                  | any -> Closed |

Unordered-but-required content groups use a set tracker on `ContentState` plus a
residual or DFA state that accepts the allowed order; multiplicity remains enforced by
the structural state. Schemas that declare attribute order significant are rejected at
schema compile time with `cem.schema.unsupported_constraint`. Exact frame fields and
transition sketches are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#34-layer-4-schemamachine-cem_mlschema).

### CEM Vocabulary In The Schema

The schema defines the namespace, qualified attribute names, allowed values, and nesting
rules for CEM transform annotations. CEM does not use HTML `data-cem-*` attributes for
CEM ownership. HTML `data-*` remains HTML-specific metadata, resolves into the synthetic
`cem:html-data` namespace, and is exposed, when needed, as the HTML `dataset` equivalent
on HTML AST nodes. Based on the component
surface and fixture vocabulary:

```
CEM transform annotation attributes in the active schema namespace:
  cem:screen   - screen/page root
  cem:form     - form boundary
  cem:action   - interactive action (button, link)
  cem:list     - data list / navigation list
  cem:card     - card container
  cem:thread   - message thread container
  cem:message  - individual message
  cem:badge    - badge / status label

Allowed state values (on any CEM-attributed element):
  cem:state: default | hover | focus-visible | active | selected
           | disabled | invalid | required | loading | empty
```

Nesting rules, required sibling relationships, allowed parent elements, unknown-content
policy, and state constraints are expressed in the schema and validated through the frame
stack. The schema source language is the CEM-native declarative format selected in §13.

Unknown and extension content is governed by a schema-owned open-content policy keyed on
`ExpandedName` namespace and active content model. The active schema declares
`OpenContent` per content model. If it omits a declaration, CEM-owned names default
closed, unknown HTML attributes default to warnings, and synthetic `cem:html-data`,
`role`, and `aria-*` remain orthogonal pass-through attributes.

Default unknown-content behavior:

| Family                                | Unknown element                                   | Unknown attribute                            |
|---------------------------------------|---------------------------------------------------|----------------------------------------------|
| HTML namespace, non-custom unknown name | `error: cem.schema.unknown_html_element`        | `warning: cem.schema.unknown_html_attribute` |
| WHATWG custom element name            | accepted                                          | n/a                                          |
| Synthetic `cem:html-data` (`data-*`)  | n/a                                               | accepted, ignored by schema validation       |
| ARIA (`role`, `aria-*`)               | n/a                                               | accepted; ARIA validity is a separate pass   |
| Active CEM schema namespace           | `error: cem.schema.unknown_cem_element`           | `error: cem.schema.unknown_cem_attribute`    |
| Other registered schema namespaces    | per registered schema open-content policy         | per registered schema open-content policy    |
| Unbound prefix                        | `error: cem.schema.unbound_prefix`                | `error: cem.schema.unbound_prefix`           |
| No namespace, `open: true`            | `warning: cem.schema.extension_element`           | `warning: cem.schema.extension_attribute`    |
| No namespace, `open: false` (default) | `error: cem.schema.unknown_element`               | `error: cem.schema.unknown_attribute`        |

Vendor-prefixed HTML attributes such as `x-data` are unknown HTML attributes by default
and therefore warn unless an extension namespace or open-content rule accepts them.
Custom elements such as `<my-thing>` are accepted as WHATWG-conformant author-defined
elements; CEM does not police the browser `customElements` registry. Attributes on those
elements still follow the same namespace/open-content policy. The schema machine
consults the open-content policy for each unknown `OpenScope` or attribute `Name` before
emitting diagnostics.

### Namespace Resolution For Tags And Attributes

Names are resolved through the schema frame's namespace context before validation or
transformation. A parsed tag or attribute name has two identities:

- **lexical name:** the literal spelling in the source, such as `cem-screen`, `screen`,
  `cem:screen`, or `button`;
- **expanded name:** the resolved namespace plus local name, owned by a schema.

Tier A uses one `QName { prefix, local, expanded }` model for tags and attributes. The
normalizer resolves `QName` at the event boundary before the schema machine consumes the
event. Tokenizer output keeps the original source spelling for source maps and reports.
Namespace policy decides the normalized `local` value for elements; attributes are
case-sensitive and may use camelCase. HTML element names bind to the HTML namespace with
ASCII-lowercased local names; HTML attributes keep their source case. XML and other
non-HTML contexts preserve case for both elements and attributes.

HTML `data-*` attributes resolve to the synthetic `cem:html-data` namespace, with the
local name taken from the case-sensitive suffix after `data-`. For example,
`data-userId` resolves as `{cem:html-data}userId` while the lexical name remains
`data-userId`. Prefix-less attribute names without a default schema namespace remain
HTML-owned or synthetic HTML-data names; they do not become CEM-owned by default.

Duplicate attributes are resolved by `ExpandedName` after namespace binding. If multiple
attributes in the same start tag resolve to the same expanded name, the last attribute
overrides the previous value and source range. The earlier occurrence is shadowed for
validation and transformation rather than diagnosed as a duplicate attribute.

Foreign content, including SVG and MathML, is a content-type switch rather than an
in-place HTML tokenizer name mode. The parent context emits a `ModeSwitch` and creates a
child context scope with its own content type, namespace context, schema, and case
policy. Names inside that scope are resolved by the child context; the parent source-map
stack wraps the child scope.

CEM-specific tags and attributes live in the namespace associated with their schema. They
do not collide with HTML attributes, pass-through attributes, or another schema's tags as
long as the namespace is defined. Rendered projections may choose unqualified convenience
spellings, but the internal identity remains namespace-qualified.

Namespace declarations are scoped and ordered. A namespace name is the explicit prefix
when present, or the empty name for the default namespace. A default namespace can expose
a schema's own tags without a prefix. Multiple default namespaces can coexist across
nested or sequential scopes because each declaration has an effective source range and
scope owner.

The same namespace binding name may intentionally refer to different schema namespaces
at different source positions. This includes the empty/default binding. For example, a
document can use unprefixed HTML nodes, rebind the default to SVG for an inline icon, and
then rebind the default back to HTML for following form controls. The lexical node names
remain unprefixed in all three regions, but their expanded names differ because the
active namespace binding differs.

Resolution rules:

- A namespace declaration applies from its declaration point forward within the owning
  scope, unless a later declaration with the same namespace name overrides it.
- If a namespace with the same name appears multiple times in the same scope, the latter
  declaration wins from that point forward.
- If an inner scope declares the same namespace name as an outer scope, the inner
  declaration wins inside the inner scope until that scope ends.
- If two default namespaces define the same unqualified tag in the same visible scope,
  the later effective namespace wins for subsequent unqualified uses.
- Previously resolved references keep their expanded namespace identity even if a later
  declaration changes which namespace the same lexical name resolves to.
- The runtime tracks namespace binding changes as source-mapped binding events so the
  same lexical name can resolve to different schema-owned tags at different source
  positions.

Example sequence inside one scope:

1. Declare default namespace `NS1`.
2. Use unqualified tag `screen`; it resolves to `{NS1}screen`.
3. Declare default namespace `NS2`.
4. Use unqualified tag `screen` again; it now resolves to `{NS2}screen`.
5. Declare default namespace `NS1` again.
6. Use unqualified tag `screen`; it resolves to `{NS1}screen` again.

This is intentionally similar to repeated `var` declarations in JavaScript: the binding
name can be declared more than once in the same scope, the later declaration becomes the
active binding for subsequent uses, and earlier uses keep the binding that was active at
their source position.

HTML/SVG example:

```cem
@ns html = "http://www.w3.org/1999/xhtml"
@ns svg = "http://www.w3.org/2000/svg"
@default html

{label |
  @default svg
  {svg @viewBox="0 0 16 16" |
    {path @d="M2 8h12"}
  }

  @default html
  {input @name=name}
}
```

`{label}` and `{input}` resolve to HTML expanded names. `{svg}` and `{path}` resolve to
SVG expanded names. The namespace binding events are source-mapped so diagnostics and
transforms can explain which default namespace was active for each unprefixed node.

Named namespaces follow the same override rule as the default namespace. The empty
namespace name represents the default namespace; it is not a special global singleton.
The parser can therefore support scoped default namespaces for CEM tags, HTML-compatible
unqualified output, and future XSLT/template-driven transformations without changing the
AST identity model.

Attribute-form namespace declarations remain one binding event per attribute because
source syntax requires unique attribute names. Schema switching by element is not an
open namespace-declaration question: it is the explicit AC-F-2 `<cem:schema ...>` family
described in §13.1. Any future compact syntax that lists multiple schema bindings in one
attribute must lower to the same ordered `NamespaceBinding` events before validation.

Namespace-resolution implementation contracts are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#341-namespace-context-contracts).

### Schema Version Compatibility

Schema version identity is now AC-bound rather than design-open. The normative rules
live in [`cem-ml-ac.md`](cem-ml-ac.md#31-schema-version-identity): schema identity is
`{ uri, embedded SemVer }`, URI tails are constraints, the embedded complete SemVer is
authoritative, compatible minor/patch skew warns for unknown content, major mismatch
aborts the scope, and prerelease/build metadata follow the AC-V-10..AC-V-12 rules.

The design consequence for Layer 4 is simple: schema resolution produces both a
`SchemaId` and a resolved version record before validation starts, and the schema
machine records the `cem.v.semver_resolved` report event before any compatibility
diagnostic that depends on that resolution. Cache and policy stamps consume the embedded
full version, not the URI shorthand.

### Diagnostics And Reports

Diagnostics use source-map byte ranges as ground truth. Line and column positions are
human-facing projections, like compiler or linter output, computed from the source-map
frame selected by the report renderer. The canonical report model is an AST-associated
report tree, not a flat list.
Each parser, schema, handoff, transform, validation, or runtime event attaches to the
current AST node when one exists, the active event-time scope context, the event-time
source-map stack, the originating scope, the error-boundary scope that handled it, and a
monotonic event sequence number. Diagnostics before AST construction use the same
location shape as AST-time diagnostics: they carry a `SourceMapStack` and active scope
context, while the AST-node back-reference remains empty until a node exists.

The report tree can be projected to CEM-native, XML, JSON, Markdown, text, HTML, or any
other supported structured format. Text and HTML reports are reference convenience
renderers over the report tree, not canonical report storage formats.

Diagnostics and source maps retain the event-time source-map stack. For source-facing
reports, comments and whitespace count when deriving byte offsets, line/column
positions, and snippets from the input frame, even if a later transform removes those
nodes. Transform-facing reports may instead project a generated or intermediate frame.
Diagnostics may refer to comments, whitespace, or processing instructions that no longer
survive in a transformed output.

The public diagnostic projection includes `byteOffset` alongside `uri`, `line`, and
`column`. `byteOffset` is derived from the selected report frame's `ByteRange.start`;
the underlying diagnostic location remains the full `SourceMapStack`, not the scalar
projection.

Diagnostic and report data shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#25-diagnostics) and
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#35-report-event-model-cem_mlreport).

### CLI Projection And Target Ownership

Stack layers own data artifacts. The CLI owns projection selection, output targets, and
default stream behavior. A command may expose one primary output, one report output, and
zero or more side outputs that address intermediate stack layers.

Target rules:

- Primary output goes to `--out` when provided; otherwise it goes to `stdout`.
- CLI parameters or a config file may set the document top-level context policy,
  including per-diagnostic severity overrides and comment/whitespace preservation.
  Descendant scopes inherit that policy unless the active schema/content type redefines
  it within parent override bounds.
- Validation-style operations (`validate`, `check`, `fixture validate`) have the report
  as their primary output. They render the selected report format to `stdout` by default.
  `stderr` is reserved for CLI usage errors, I/O failures, unexpected internal failures,
  and operational messages that are not part of the report AST.
- Load/save or conversion-style operations (`parse`, `convert`, future `load`/`save`)
  have converted content or selected layer data as their primary output. They write it
  to `--out` when provided, or to `stdout` by default. Reports for these operations are
  side outputs and should be written only when a report target is requested.
- When the primary output uses `stdout`, additional layer projections must not also write
  to `stdout` unless the CLI explicitly selects a multiplexed container format. Side
  outputs should use explicit file targets.
- Human text and HTML outputs are reference convenience projections. Structured report
  or layer projections should prefer CEM-native, XML, JSON, or another supported
  structured format.

Standard projection layers include source metadata, decoded scalars, tokens, normalized
events, schema frames, namespace bindings, handoffs, input DOM, WHATWG DOM, CEM AST,
transform output, UI DOM plans, machine state, hydration plans, template registries,
source-map data, report AST, trace data, and deferred binary/chunk projections. The
exact projection key table is in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#36-cli-projection-keys).

---

## 9. Layer 5 - Scoped Embedded Handoff Stack (`cem_ml::handoff`)

### Purpose

Manages embedded content regions where the active parser and schema must switch to a
different content type. Embedded language boundaries are explicit and parent-owned. A
child parser never infers the parent's return condition independently.

### Functional Design

A handoff records the child content type, the inherited parent context, the child schema
id when one is known, the source span when available, and the return condition. The
current context parser is responsible for recognizing a content-type switch and decoding
the owned body or attribute content when decoding is required. Creation of the embedded
context is the `ModeSwitch`: it yields a `HandoffRecord` plus the source-mapped decoded
stream for the child context. The reference implementation recommends using the CEM
framework to map the entity context type, create the embedded context, and attach the
decoded stream, instead of hand-constructing child contexts in each parser. The child
context receives a source-mapped decoded stream, not a plain string and not undecoded
parent bytes. The child parser consumes through the declared return condition and then
returns control to the parent.

Tier A handoff cases:

| Parent context | Trigger              | Child content type             | Return condition    |
|----------------|----------------------|--------------------------------|---------------------|
| HTML document  | `<style>` start tag  | `text/css`                     | `</style>` end tag  |
| HTML element   | `style=""` attribute | `text/css` (declaration block) | attribute quote end |
| HTML document  | `<script>` start tag | raw text (not parsed Tier A)   | `</script>` end tag |

For attribute handoffs such as `style=""`, the HTML container first decodes the attribute
value according to HTML rules. The CSS child parser receives the decoded stream with
source-map frames back to the parent attribute ranges, including entity or escape-origin
mapping where available.

For Tier A, the CSS child parser is a stub that emits diagnostics but does not produce a
typed CSS AST. Script regions are treated as raw text by the parser. Whether a script
region is preserved, warned, rejected, or allowed only for specific `type` values is
defined by the active scope/content-type policy, using the same error-level handling as
all other content types. The handoff stack is implemented fully in Tier A to keep the
interface stable for Tier B content-type expansion.

The one WHATWG-specified exception is script-data mode: `</script>` ends the script
region according to WHATWG tokenizer rules regardless of the child parser. This is
modeled as a matching-end-tag return condition but driven by the WHATWG tokenizer state,
not independently inferred by the child parser.

Deferred handoff cases should be listed in the handoff model now, but implementation
priority is explicit:

1. **XML next:** CDATA sections, entity boundaries, DTD/internal subsets, external
   resources, XInclude, and XML compatibility handoffs.
2. **JSON after XML:** JSON strings, object/array subtrees, escaped string views, and
   fixed-length or delimiter-bounded JSON payloads.
3. **HTML and other embedded cases later:** additional HTML raw-text and RCDATA cases,
   CSV/CSF fields, TypeScript template strings, JSX islands, CSS functions, and any
   other language-specific embedded boundaries from the research.

Listing a deferred case does not move it into Tier A. Tier A implements the HTML
style/script cases above and keeps the enum/interface surface broad enough to add XML,
JSON, HTML extensions, and other embedded handoffs without changing parent-owned
boundary semantics.

Handoff record and return-condition shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#37-layer-5-scoped-embedded-handoff-stack-cem_mlhandoff).

---

## 10. Layer 6 - InputDomAstBuilder / InterpreterAstBuilder (`cem_ml::parser`)

### Purpose

Converts the validated, normalized event stream into the schema-defined input DOM/AST.
For HTML, this is the initial HTML parser DOM: a source-preserving reconstruction of the
schema-defined token hierarchy, not the WHATWG implementation DOM. Every node carries a
source-map stack, attributes, and reference slots for unresolved `id`/`for`/`aria-*`
targets.

The CEM AST projection is an annotation and transformation view over that input DOM/AST.
It records schema-qualified CEM attributes, state labels, transform triggers, and
CEM-specific reference helpers without changing the initial parser DOM or replacing an
element's native HTML/XML identity.

### Functional Design

The input DOM/AST is generic and schema-defined. It must be able to represent XML and
(X)HTML grammar constructs such as elements, attributes, text, comments, doctypes,
processing instructions, CDATA sections where the content type supports them, raw-text
regions, whitespace/trivia nodes, recovered error nodes, and future schema-owned node
kinds. These are not CEM semantic constructs by default, but comments and whitespace
remain part of the source-preserving input tree unless the effective scope policy strips
them. Processing instructions are schema-driven and may be preserved, diagnosed,
transformed, or stripped by the active schema/content-type policy.

The reference implementation includes a transformer that strips comments and whitespace
from the working tree while preserving report entries and source-map references to the
initial stream. This makes compact output a transform concern rather than a tokenizer or
normalizer behavior. CEM-specific support and syntax for treating comments or CDATA as
semantic CEM content is still TBD.

The CEM projection is narrower than the generic input DOM/AST. It records CEM transform
annotations attached to source nodes, including:

- screen;
- form;
- action;
- list;
- card;
- thread;
- message;
- badge;
- state.

These schema-qualified attributes are transform triggers and transform inputs. They do
not, by themselves, replace the source element's tag meaning or native role. For example,
`<button cem:action="primary">` is still a `button`; `cem:action` supplies CEM
transformation data associated with that button.

A source element may carry zero or more CEM annotations. Transformations usually preserve
the associated source element. When a schema-owned transform changes the subtree,
including replacing or splitting the element itself, that transform plan owns
composition, precedence, rejection, or diagnostics for any incompatible annotations on
the same source node.

Each CEM annotation has an annotation id, source node id, schema-qualified annotation
name, value, source-map stack, and optional state. The document also owns the global id
table used for reference resolution.

Reference slots support unresolved forward references. A reference to an id that has not
yet appeared binds to a mutable slot in one pass. When the parser later encounters the
target id, it fills the slot, and existing label/for/ARIA references observe the
resolved target. Remaining unfilled slots are checked when the owning context scope
closes. By default, unresolved references emit warnings. The effective context-scope
policy can override the diagnostic severity per error type; that policy is inherited
from the document top-level scope unless a descendant scope redefines it within parent
override bounds.

CEM-ML reference slots inline references by binding to the resolved target; they do not
clone the referenced content into the originating tree. Content-type transforms that need
clone-like behavior, such as XML compatibility entity expansion, own that behavior within
their own transform scope.

AST node, annotation, and reference-slot shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#38-layer-6-inputdomastbuilder-interpreterastbuilder-cem_mlparser).

---

## 11. Layers 7-8 - BinaryAstEncoder And ChunkCompressor (Deferred Tier B)

### Design Intent

The binary layer is the future internal transport and cache format. It can encode node
kinds, schema ids, scope slots, source-map stacks, string tables, typed values,
dictionaries, subtree chunks, integrity hashes, and dependency ids without repeated
textual markup.

For Tier A, the pipeline skips these layers. The `InputDomAstBuilder` and
`InterpreterAstBuilder` outputs are in-memory Rust trees with no binary encoding. The
`encode` and `segment` state transitions on the schema machine are no-ops. Any IDs used
inside Tier A stubs are opaque process-local handles only and are not serialized binary
identifiers.

### Canonical Identity And Ordering

Canonical binary representation must not depend on incidental AST element order for
identity or references. Parsing preserves source order where the source format makes
order semantic. Transformations preserve that order unless the active content-type schema
permits or defines a different semantic order.

References in canonical binary form are key/value mappings by default. Node references,
attributes, dictionary entries, source-map frames, dependency slots, and chunk relations
must have stable logical keys so references remain valid if physical storage order
changes. Positional indexes are permitted only as schema-constrained optimizations over
the canonical key/value model.

Diagnostics and report-linked data preserve emission order through monotonic event
sequence numbers. Chunk continuation order is preserved through explicit chunk relation
sequence numbers.

### Deferred Broadcast, Compression, And Incremental Mode

Broadcast/cache paths are absent in Tier A. Later tiers can attach broadcast delivery to
the compressed chunk layer after subtree ownership, dependency ids, integrity hashes, and
dictionary version requirements are stable.

Incremental/editor support is also Tier B. Tier A remains a batch/fixture path, but Tier
A source maps and scope boundaries must preserve enough information for later reuse. The
future incremental runtime consumes caller-provided changed byte ranges, maps them
through source-map stacks to owning schema scopes, invalidates affected scopes, and falls
back to enclosing-scope rescan when boundaries or cross-scope references are unsafe to
reuse.

Binary, chunk, compression, id-policy, and incremental contracts are in
[
`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#39-layers-7-8-binaryastencoder-and-chunkcompressor-cem_mlast)
and
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#5-incremental-and-editor-mode-contract-deferred-tier-b).

---

## 12. Layer 9 - ImplementationInterpreter (`cem_ml::interpreter`)

### Purpose

Consumes the validated typed CEM AST and produces the target output. For CEM Tier A, the
interpreter is the transform pipeline: semantic HTML -> light-DOM custom-element markup
compatible with `@epa-wg/custom-element`.

### WHATWG HTML DOM Transformation

WHATWG HTML DOM treatment is a content-type transformation over the initial HTML parser
DOM. It is driven by the active schema because the schema defines the token hierarchy
that the input DOM/AST builder reconstructs from the token stream. The transformation
then applies WHATWG insertion modes, stack-of-open-elements behavior, active formatting
element handling, foster parenting, foreign-content behavior, and DOM update rules to
produce or update an implementation DOM.

This boundary keeps token extraction, hierarchy reconstruction, and implementation DOM
compliance separate:

- `SchemaTokenizer` follows WHATWG lexical/tokenizer rules and emits source-spanned
  tokens.
- `SchemaMachine` validates the stream and determines the schema-defined hierarchy and
  embedded-content ownership.
- `InputDomAstBuilder` reconstructs the initial HTML parser DOM from that hierarchy.
- The WHATWG HTML DOM transformation materializes the compliant implementation DOM from
  the initial DOM.

Synthetic `CloseScope` events close already-open source scopes only; they do not create
new element nodes. Missing-node insertion, reparenting, foster parenting,
active-formatting-element handling, and other WHATWG implementation DOM effects stay in
this content-type transform. For example, the normalizer may close the first `td` before
a second sibling `td`, but insertion of missing `tbody` and `tr` nodes is performed here
with transform source-map frames.

Other content-type transformations use the same model: SCSS can lower to CSS, CSS can
resolve `url()`/`@import` references into parsed child ASTs or unresolved resource slots,
and CEM semantic HTML can lower to custom-element markup while preserving source-map
stacks.

### Visual Content And Machine State Data

The internal CEM AST uses one ownership, source-map, reference-slot, and scope model
across content types. It does not create fundamentally different AST families for HTML,
SVG, MathML, canvas instructions, CSS, JavaScript, JSON, XML, or other embedded formats.
Content type is scope metadata plus transform policy. The active policy decides whether
a scope is treated as visual content, executable or transform code, machine state data,
or an inert/unresolved resource.

Functional scope roles:

- **Visual content:** HTML, SVG, MathML, canvas command data, images, video, and other
  renderable scopes that can contribute nodes or resources to the UI output.
- **Machine state data:** data islands, attributes, `dataset`, payload/slot content,
  fetched resources, route/location state, storage state, form state, call-instance
  slot data, and runtime slices that parameterize transforms.
- **Code or transform content:** CSS, SCSS, CEM template fragments, JavaScript when
  enabled by policy, and other content that can affect rendering or state but is not
  itself the rendered UI tree.
- **Unresolved resource slots:** external resources or embedded constructs preserved
  for a later transform or caller when no active policy consumes them.

The CEM engine can transform a visual scope with access to both:

- the owned scope data visible through the schema frame and AST ownership chain; and
- the HTML implementation DOM projection produced by the WHATWG DOM transformation when
  that projection is available.

The result is a CEM UI DOM plan: a virtual rendering plan that can materialize browser
DOM, light-DOM custom-element markup, or another rendered projection. Virtualization at
this layer means template reuse by reference plus data application during
transformation. A template is a scoped transform resource. It can be addressed by schema
identity, local id, URL, URL fragment, registry entry, or a DCE/custom-element tag name;
the render plan keeps the template reference stable and binds current machine state data
into that template when transforming.

This behavior is part of the CEM concept and does not require Declarative Custom Element
(DCE) markup as the template source. DCE is one runtime/authoring projection. In that
projection, a custom tag or `<custom-element>` declaration provides the template
reference and binds attributes, dataset, payload/slots, and data slices into the
transformation data. The CEM transform model must remain able to execute the same
template-reference plus data-binding concept without requiring the DCE syntax.

Hydration rules describe which runtime events can update machine state and which render
scope is invalidated by that update. A hydrated render follows this sequence:

1. Runtime event or browser data adapter updates a machine state slot.
2. The hydration rule maps that slot to one or more affected transform scopes.
3. The engine re-applies the template reference to the updated state.
4. The DOM update layer patches the rendered UI while preserving unchanged DOM nodes,
   source-map relationships, focus/selection state, and runtime-owned resources where
   policy allows.

The `<custom-element>` stack is therefore an integration target, not the definition of
the CEM render model. Its DCE implementation adds declarative interactivity by
propagating events to data slices and rerendering affected UI. Other custom-element
stack primitives expose browser data and APIs, such as HTTP request, storage, and
location/route state, as machine state providers. CEM models those providers as runtime
data adapters feeding machine state slots; they are not special AST node families.

CEM machine-state slots are data propagation placeholders supplied by a call instance or
runtime adapter. They are not HTML `<slot>` elements and do not follow HTML slot
distribution rules. Multiple CEM slots with the same name intentionally refer to the
same state slot, so the same data is reused wherever that name appears in the same
effective scope.

Tier A may emit static rendered output and enough template/state metadata for later
hydration. Live event handling, browser API adapters, DOM patching, DOM identity
preservation, and reusable template registries are subject to a separate design phase
(TBD) unless a narrower implementation phase explicitly brings one forward.

Implementation contracts for template references, machine state slots, hydration rules,
and virtual DOM patch metadata are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#311-visual-content-machine-state-and-hydration-contracts).

### Schema-Driven Transform Rules

Transform behavior is schema-driven. The schema owns the transform layers, including
source annotation matching, target element construction, attribute mapping, child
traversal, copy/pass-through rules, and source-map frame creation. The selected
execution backend is the CEM template renderer implemented in Rust. It is generic across
schema-owned transform plans and is not a set of hand-written per-component Rust rules.

For the CEM semantic HTML projection, schema-qualified CEM annotations drive custom
element output, wrapper generation, attribute generation, or no structural output
depending on the active transform plan. Schema-qualified CEM attributes can become
generated custom-element attributes such as `cem-id`, `variant`, or `state` according to
the active schema. Other standard HTML attributes (class, id, ARIA, and synthetic
`cem:html-data` names from HTML `data-*`) pass through only as HTML-owned metadata
unless the active schema defines a stricter mapping. Transformers match CEM annotations,
not raw HTML `data-*` attributes or the HTML-specific `dataset` projection.

| CEM annotation         | Source element (typical)        | Possible output projection       |
|------------------------|---------------------------------|----------------------------------|
| `cem:screen="id"`      | `<main cem:screen="id">`        | `<cem-screen cem-id="id">`       |
| `cem:form="id"`        | `<form cem:form="id">`          | `<cem-form cem-id="id">`         |
| `cem:action="primary"` | `<button cem:action="primary">` | `<cem-action variant="primary">` |
| `cem:list`             | `<ul cem:list>`                 | `<cem-list>`                     |
| `cem:card`             | `<div cem:card>`                | `<cem-card>`                     |
| `cem:thread`           | `<ul cem:thread>`               | `<cem-thread>`                   |
| `cem:message`          | `<article cem:message>`         | `<cem-message>`                  |
| `cem:badge`            | Any with `cem:badge`            | `<cem-badge>`                    |
| No CEM annotation      | Any other element               | Pass through unchanged           |

Children are transformed recursively. Text nodes pass through unchanged.

### Canonical CEM-ML Serialization

The canonical serialization of a transformed AST tree is the curly-brace CEM-ML surface
defined in [`cem-ml-syntax.md`](cem-ml-syntax.md). Canonical snapshots, hashes, fixture
round trips, and cache identities use this CEM-ML tree rather than rendered HTML, XML, or
another target projection. The CEM-ML serialization is schema-owned and follows the same
transform plan that produced the tree. XML convention serialization is a deterministic
secondary parity projection.

Canonical CEM-ML serialization rules:

- Node order follows schema-defined semantic order.
- If the schema permits source-order preservation, preserve parse/source order.
- If the schema defines a transformed order, use the schema-defined transformed order.
- Node identity and references use stable CEM-ML keys, not process-local AST ids.
- Attributes and properties serialize in schema-defined order first, then stable lexical
  key order for open or extension fields.
- Text values use the CEM-ML escaping policy for the selected CEM-ML syntax.
- Source maps are included only when the selected projection requests them; minimal
  canonical CEM-ML content does not require inline source-map payloads.
- Diagnostics are serialized through the report AST, not embedded into canonical CEM-ML
  content unless a combined report projection explicitly requests them.

### Rendered Output Projections

Light-DOM custom-element HTML is a rendered projection of the canonical CEM-ML tree, not
the canonical AST serialization. Rendered output must still be deterministic for
snapshots and fixture comparison:

- schema-owned generated attributes serialize before pass-through attributes;
- attributes within each group serialize in stable key order unless the schema defines a
  stricter order;
- text and attribute values use one renderer-specific escaping policy;
- custom elements use explicit start and end tags;
- whitespace output is controlled by the selected transform/renderer; source whitespace
  is preserved in the input tree unless policy strips it.

Each output custom-element node appends an implementation transform frame. Prior frames
trace back to the original input token, enabling generated output such as `<cem-screen>`
to resolve back to `<main cem:screen="login">`.

### Transform Execution Backend

The transform backend is resolved as a Rust implementation of the schema-driven CEM
template renderer. For most content types, schema-owned CEM templates define matching,
selection, output construction, attribute generation, copy/pass-through behavior,
recursive application, source-map frame creation, and diagnostics.

The renderer is generic: transform behavior is loaded from the compiled schema/template
plan, not encoded as bespoke Rust logic for individual CEM components. Rust owns
execution, type checking, resource limits, source-map preservation, diagnostics, and
security enforcement.

The CEM template model is expected to provide parity with the majority of XSLT-style
transformation functionality: template matching, scoped context, value selection,
conditional output, iteration, recursive template application, copy/pass-through rules,
named/template-reference calls, parameter or state binding, and deterministic output
serialization. It does not adopt unrestricted XPath/XSLT execution as the runtime
contract.

The primary differences from XSLT are context scope and selector language. CEM template
queries are evaluated against the current transform scope, schema-owned AST view,
machine-state slots, and policy-visible resources only. The selector/query language is
`cem-ql` as specified in [`cem-ql-ac.md`](cem-ql-ac.md); host-side template embedding is
the AC-T-7 contract: host-owned `{...}` spans in template-aware attributes,
whole-expression attributes such as `select=` / `match=` / `test=`, and explicit `$`
expression nodes for content. Concrete renderer grammar and evaluator IR details belong
in the future `cem-ql-stack-design.md`, but the delimiter and expression-language
decision is no longer TBD.

Transform interface shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#310-layer-9-implementationinterpreter-cem_mlinterpreter).

---

## 13. CEM Schema Language

The schema machine requires a machine-readable CEM schema. The research establishes
RELAX NG derivatives as the **validation algorithm**. The selected schema authoring
source is a CEM-native declarative format expressed through the canonical curly CEM-ML
surface, with XML/RELAX NG mirrors emitted as secondary artifacts.

The CEM-native format is the source of truth for CEM vocabulary and schema behavior:
roles, states, token tiers, component names, namespace ownership, open-content policy,
structural content models, embedded handoff declarations, and schema-owned transform
hooks. Existing token tables, XML schema files, RELAX NG compact/XML, or other external
schema artifacts may be supported as import adapters or emitted mirrors, but they are
not competing canonical authoring formats for CEM schemas.

The dedicated schema compiler emits two products:

- a structural validation IR with RELAX NG functional parity that can drive the limited
  Tier A CEM DFA profile and the full derivative runtime; and
- a schema-owned rule registry for cross-reference, semantic/contextual, lexical/mode,
  tokenizer-boundary, policy, and transform checks with explicit dependency tier and
  execution placement.

All CEM constraints are schema-owned. The compiler separates *ownership* from
*execution placement* by assigning each rule to the earliest safe layer that has enough
information to enforce it:

| Tier                                  | Examples                                                                                                              | Primary execution placement |
|---------------------------------------|-----------------------------------------------------------------------------------------------------------------------|-----------------------------|
| 1. Structural (derivative-computable) | element/attribute names, child ordering and multiplicity, attribute presence, simple value-type and pattern constraints | SchemaMachine via CEM structural IR -> Tier A DFA / later derivative runtime |
| 2. Cross-references                   | `aria-labelledby` / `for` ID resolution, `href` fragment targets, `cem:slot` slot-to-named-target binding             | Reference-resolution pass when slot/name-resolution state is available |
| 3. Semantic / contextual              | uniqueness of `id`, allowed embedding by content type, ARIA semantics, policy-sensitive validation                    | Earliest safe tokenizer, normalizer, AST, transform, or policy pass |

Tier 2 and Tier 3 checks do not have to wait for full document completion. Tokenizer or
normalizer placement is valid for schema-owned lexical/mode, delimiter,
embedded-boundary, or local token diagnostics. Tokenizer-executed diagnostics remain
schema-owned and must not make the tokenizer the semantic source of truth or allow it to
rewrite WHATWG lexical behavior.

Functional parity requirements:

- The structural IR must be RELAX-NG-equivalent for structural validation and support
  `D(event, schema)` computation. Tier A may expose only a limited DFA execution profile
  over that IR. Replacing the Tier A DFA with a derivative runtime must preserve
  structural accept/reject semantics for supported constraints, but it may change
  diagnostic codes, payload shapes, expected-content sets, ordering, wording, and report
  snapshots.
- The format must represent the CEM annotations and state values in §8, namespace
  bindings, qualified names, required attributes and children, child ordering and
  multiplicity, simple value-type and pattern constraints, unknown/open-content policy,
  embedded content handoffs, content-type transform hooks, and rendered/canonical
  projection metadata.
- The compiler must emit structural constraints into the structural IR and emit
  cross-reference, semantic/contextual, lexical/mode, tokenizer-boundary, policy, and
  transform constraints into the schema-owned rule registry with explicit execution
  placement.
- The schema machine consumes compiler output, not authoring syntax. Selecting the
  CEM-native format fixes the compiler source contract without changing the event
  processing, frame-stack, DFA, or derivative-runtime boundaries.

### 13.1 Document-Side Schema Scoping

A document references schemas through four forms: an inline body definition, an
element-level mid-document switch (self-closing or wrapping), and a scope-policy
attribute applicable to any element. All four open new scopes that follow the AC-F-1
scope-policy inheritance model; none mutates the active schema of an ancestor scope.

Canonical CEM-ML and XML convention forms are kept in parity. The XML table below is
the secondary mirror form; the canonical CEM-ML surface uses directives and `@`-prefixed
attributes:

| Canonical CEM-ML form                                      | XML convention mirror                                      |
|------------------------------------------------------------|------------------------------------------------------------|
| `@schema src="./schema.cem-schema"`                        | `<cem:schema src="./schema.cem-schema"/>` or host `cem:schema-src` |
| `{schema @name="badge" \| ...}` where schema language allows | `<cem:schema cem:name="badge">...</cem:schema>`            |
| `{schema @src="./schema.cem-schema" \| ...}`                | `<cem:schema src="./schema.cem-schema">...</cem:schema>`   |
| `{section @schema-src="./admin.cem-schema" \| ...}`         | `<cem:section cem:schema-src="./admin.cem-schema">...</cem:section>` |
| `{section @schema-select="$schemaQuery" \| ...}`            | `<cem:section cem:schema-select="$schemaQuery">...</cem:section>` |

**Form table — declaration and switching constructs:**

| Form                                                          | Self-closing | Effect                                                                                                                                                                                            |
|---------------------------------------------------------------|--------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `<cem:schema cem:name="...">…body…</cem:schema>`              | No           | Declares an inline schema with an addressable name. Does **not** switch the active schema on its parent scope. The body is available for reference via `cem:schema-select` from descendant scopes. |
| `<cem:schema src="..."/>`                                     | Yes          | Mid-document schema switch at sibling position. Opens a new scope that adopts the loaded schema for itself and subsequent siblings until the end of the parent scope.                              |
| `<cem:schema src="...">…children…</cem:schema>`               | No           | Wrapping switch. Opens a scope around its children; the loaded schema applies inside the wrapper only. The parent scope is unaffected after `</cem:schema>`.                                       |
| `<cem:schema select="..."/>` or `<cem:schema select="...">…</cem:schema>` | either | Same as the `src` forms but the schema body is resolved by cem-ql query within the document — typically resolving to an inline `<cem:schema cem:name="...">` node.                                |
| `cem:schema-src="..."` on any element                         | n/a          | The host element becomes a scope. The loaded schema applies **inside** the element only; the parent scope's active schema is unchanged. No upward propagation.                                     |
| `cem:schema-select="..."` on any element                      | n/a          | Same as above, with the schema body resolved by cem-ql query.                                                                                                                                     |

**Source-attribute table — URI vs cem-ql resolution:**

| Attribute              | Value form                  | Resolution path                                                                                          |
|------------------------|-----------------------------|----------------------------------------------------------------------------------------------------------|
| `src` (on `<cem:schema>`) / `cem:schema-src` (on any element) | URI literal (http / file / relative) | AC-T-4 transform-source loader, gated by the AC-A-6 external-resource I/O queue when not local.        |
| `select` (on `<cem:schema>`) / `cem:schema-select` (on any element) | cem-ql expression           | Evaluated against the document with scope-chain-aware semantics. Resolution returns the innermost match. |

Both `src`/`select` (and their `cem:schema-*` attribute variants) are mutually
exclusive on a single host. A host declaring neither is a schema-compilation error.

**Identifier-resolution table — `cem:name` declarations and lookups:**

| Aspect             | Behavior                                                                                                                                          |
|--------------------|---------------------------------------------------------------------------------------------------------------------------------------------------|
| Declaration        | `cem:name="..."` on `<cem:schema>` (inline form only; loaded schemas carry their own identity via §3.1 schema version identity).                  |
| Visibility         | Scope-chain — the binding is visible to the host scope and all descendants per AC-F-1 inheritance.                                                |
| Override           | A nested `<cem:schema cem:name="X">` shadows an outer `<cem:schema cem:name="X">` within the nested scope. Outer remains active outside.          |
| Uniqueness         | Names need **not** be globally unique. Intentionally differs from HTML `id` (which is required-unique) so nested-redefinition is a legal override. |
| Reference syntax   | `cem:schema-select` (or `select` on `<cem:schema>`) value resolves a cem-ql query against the scope-chain-aware document; innermost match wins.   |
| Identity for cache | An inline schema's content-addressed identity for AC-CC-1 hashing is `inline:<sha256-of-body>`; the `cem:name` is an alias, not the identity.     |

Scope opening for all four forms routes through the existing parser scope machinery
(AC-P-4, AC-P-5). The schema machine sees a new scope frame with the dispatched schema
id; source-map frames span the boundary cleanly per AC-P-7. NVDL-style namespace
dispatch (AC-P-6) remains the orthogonal mechanism for namespace-driven switching;
when both fire on the same boundary, NVDL applies first and the explicit form layers
on top within its scope.

### 13.2 Schema Compiler Output Module

This subsection closes DESIGN-FOLLOW-001 / AC-ALIGN-010 by giving the
compiler explicit *release-artifact* responsibilities in addition to the
SchemaMachine-side IR it already owns. AC mapping is direct:

| AC                         | Output                                              |
|----------------------------|-----------------------------------------------------|
| AC-S-2 [A]                 | RELAX NG XML (`*.rng`) and compact (`*.rnc`) mirror |
| AC-S-3 [A] + AC-S-6 [A]    | TypeScript `.d.ts` headers (structural + `Validated<T>`) |
| AC-S-4 [B]                 | Rust `.rs` headers for native consumers             |
| AC-S-5 [A]                 | Stable URI publication manifest + hash sidecars     |

The compiler's existing in-process output (the `CompiledSchema` consumed by
the SchemaMachine in §8) is unchanged. The new module is a *projection layer*
that takes that `CompiledSchema` and writes byte-stable artifacts under
`packages/cem_ml/dist/lib/schema/`. Concrete Rust shapes, file layout, and
test harness are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#342-schema-compiler-output-module-cem_mlschemacompiler).

#### 13.2.1 Pipeline

```
CompiledSchema (in-process IR)
  -> CompilerOutput (collected EmittedArtifact set + manifest)
  -> on-disk publication tree under packages/cem_ml/dist/lib/schema/<namespace-tail>/<embedded-version>/
```

`CompiledSchema` is the single input. The output module is *pure*: it takes
the IR plus a `CompilerOptions` flag bag and returns `CompilerOutput`. The
filesystem writer is a thin shell that copies `EmittedArtifact.bytes` to
disk in the locations declared by the embedded manifest.

#### 13.2.2 Emitter Roster

Each emitter is one trait implementation. Emitters share the same
deterministic-encoding contract (§13.2.4) and the same source-map provenance
contract: every emitted artifact carries a `SourceMapStack` frame for each
declaration so diagnostics raised against the artifact map back to the
authoring CEM-native schema source.

| Emitter           | Output extension   | AC reference  | Tier | Status of external validator                              |
|-------------------|--------------------|---------------|------|-----------------------------------------------------------|
| `rng_xml`         | `.rng`             | AC-S-2        | A    | Jing / libxml `xmllint --relaxng` oracle (see Open Question 5) |
| `rng_compact`     | `.rnc`             | AC-S-2        | A    | Trang round-trip (compact ↔ XML) cross-check              |
| `ts_dts`          | `.d.ts`            | AC-S-3, AC-S-6 | A   | `tsc --noEmit` against fixture call sites                 |
| `rust_hdr`        | `.rs`              | AC-S-4        | B    | `cargo check` against a generated stub crate              |
| `uri_publish`     | `manifest.json` (+ `*.hash` sidecars) | AC-S-5 | A | Round-trip resolution against `cem_ml::loader`            |

Emitter inputs and outputs are typed; no emitter consumes raw `CompiledSchema`
fields directly. The shared `SchemaEmitter` trait normalizes the IR into a
deterministic `EmissionCursor` that walks elements, attributes, semantic
rules, open-content rules, and the schema-version identity in a fixed order.

#### 13.2.3 Output Struct Shapes (logical)

```
CompilerOutput:
  schema_id        : SchemaId
  embedded_version : SemVer            full SemVer 2.0; AC-V-9 authoritative
  schema_uri       : String            AC-S-5 stable URI
  artifacts        : Vec<EmittedArtifact>
  manifest         : PublicationManifest
  diagnostics      : Vec<Diagnostic>   compile-time diagnostics raised during emission

EmittedArtifact:
  kind             : ArtifactKind      RelaxNgXml | RelaxNgCompact | TypeScriptDts | RustHeader | Manifest
  relative_path    : String            under dist/lib/schema/
  bytes            : Vec<u8>           UTF-8, LF-only, final newline mandatory
  content_hash     : ContentHash       AC-CC-1 cem-bin/1+blake3 over bytes
  source_map       : SourceMapStack    frame chain back to CEM-native source

PublicationManifest:
  schema_uri       : String
  embedded_version : SemVer
  artifacts        : BTreeMap<ArtifactKind, ArtifactDescriptor>   stable key order
  hash_scheme      : "cem-bin/1+blake3"  matches AC-CC-1

ArtifactDescriptor:
  relative_path    : String
  content_hash     : ContentHash
  byte_length      : u64
  emitted_by       : EmitterTag        crate version + emitter name; not part of stability hash
  validated_by     : Option<ValidatorTag>  recorded only after verification passes
```

Field ordering above is the **wire order** for the JSON projection of
`PublicationManifest`; the JSON encoder writes keys in this exact order so
the manifest is itself byte-stable.

#### 13.2.4 Byte-Stability Rules

These rules apply to every artifact and to the manifest:

1. **Encoding.** UTF-8, no BOM, LF line endings, no trailing whitespace on
   any line, single trailing newline at EOF.
2. **Indentation.** Two spaces for XML, TS, Rust, and JSON outputs.
3. **No timestamps.** No emitter writes wall-clock dates, build numbers, or
   nondeterministic identifiers. `EmitterTag` is written to the manifest
   only and is excluded from the AC-CC-1 hash input.
4. **Stable ordering.** All collections traversed by emitters use the
   declaration order recorded on `CompiledSchema`, with a `BTreeMap`/
   `BTreeSet` final sort on any name-keyed projection (annotations, allowed
   values, allowed states, namespace bindings, open-content rules).
5. **Numeric formatting.** Integer multiplicities written as base-10 with no
   leading zeros; floats are disallowed in Tier A emit output.
6. **XML attribute order.** Fixed alphabetical sort within each element
   except for a normative preamble (`xmlns`, `xmlns:cem`, `ns`, `name`) per
   RELAX NG XML §4. The same rule applies to the manifest's JSON object
   members (preamble keys first, declared field order second).
7. **Content hash.** `cem-bin/1+blake3` over the artifact bytes. The hash
   sidecar lives next to its artifact (`cem-core.rng.hash`,
   `cem-core.d.ts.hash`, …) so independent consumers can verify a single
   artifact without parsing the manifest.
8. **Hash input is the bytes.** The hash is computed *after* all rules above
   apply. Emitting the same `CompiledSchema` twice (same crate version, same
   options) MUST yield identical bytes and identical content hashes; this is
   the runnable verification surface for AC-S-2 byte stability.

#### 13.2.5 File Ownership And On-Disk Layout

```
packages/cem_ml/dist/lib/schema/
  <namespace-tail>/<embedded-major.minor.patch>/
    cem-core.rng                  (RelaxNgXml)
    cem-core.rng.hash             (sidecar; content_hash of cem-core.rng)
    cem-core.rnc                  (RelaxNgCompact)
    cem-core.rnc.hash
    cem-core.d.ts                 (TypeScriptDts)
    cem-core.d.ts.hash
    cem-core.rs                   (RustHeader; tier-B, gated by CompilerOptions.emit_rust)
    cem-core.rs.hash
    manifest.json                 (PublicationManifest; ArtifactKind::Manifest)
    manifest.json.hash
```

`<namespace-tail>` is the schema URI's path component after the well-known
`https://cem.dev/ns/` prefix, with `/` segments preserved as directories.
For `https://cem.dev/ns/core/1` the tail is `core` (the `/1` MAJOR
constraint is recorded in the URI metadata but does **not** become a
directory — see Open Question 4 for the publication-side aliasing).
`<embedded-major.minor.patch>` is the full SemVer of the loaded schema per
AC-V-9; pre-release and build-metadata suffixes are included literally.

The compiler module owns every file under this tree. No other module may
write to `dist/lib/schema/`. The CLI consumes the manifest read-only at
schema-load time.

#### 13.2.6 URI Publication Workflow (AC-S-5)

1. The compiler computes the artifact bytes and content hashes in-process.
2. `uri_publish` writes the manifest and sidecars **after** all artifacts
   have been hashed; a partial emit leaves the previous manifest in place.
3. The CLI's schema loader resolves `<schema-uri>` against
   `dist/lib/schema/` first, then against registered remote resolvers. The
   loader rejects an artifact whose recomputed hash does not match its
   sidecar with `cem.schema.artifact_hash_mismatch`.
4. AC-V-10 URI-tail matching is performed against the **manifest**, not the
   directory tree: the loader scans manifests under the namespace tail, then
   picks the embedded version that satisfies the URI tail per AC-V-10.
5. Release tooling tags the bundle in CHANGELOG and pins the manifest hash
   in `Cargo.toml` metadata so a release reproduces the same artifacts.

#### 13.2.7 Verification Fixtures

Verification fixtures land under `packages/cem_ml/tests/schema_emit/`. Each
fixture name encodes the AC it pins:

| Fixture file                          | AC pinned                          | What it asserts                                                          |
|---------------------------------------|------------------------------------|--------------------------------------------------------------------------|
| `byte_stability.rs`                   | AC-S-2 byte-stable requirement     | Emitting twice yields identical bytes and identical content hashes.      |
| `rng_xml_oracle.rs`                   | AC-S-2 RELAX NG XML mirror         | Emit `.rng`, validate canonical fixtures against it through the chosen RELAX NG oracle. Skipped (not failed) when the oracle toolchain is absent, with skip recorded in the report. |
| `rng_compact_roundtrip.rs`            | AC-S-2 compact mirror              | Emit `.rnc`, convert to `.rng` via the oracle, diff against the emitter's `.rng`. |
| `ts_dts_structural.rs`                | AC-S-V-1, AC-S-V-3                  | Compile a fixture that assigns an emitted `Badge` to `HTMLElement`.      |
| `ts_dts_validated_brand.rs`           | AC-S-V-2, AC-S-V-4, AC-S-V-5        | `// @ts-expect-error` brand fixtures + version-identity discrimination.  |
| `rust_hdr_compiles.rs`                | AC-S-4                              | Run `cargo check` against an auto-generated stub crate that imports the emitted `.rs`. |
| `uri_manifest_resolution.rs`          | AC-S-5, AC-V-10                     | Resolve `/1`, `/1.2`, `/1.2.3`, prerelease-exact URIs through the loader; assert manifest match-rule diagnostics per AC-V-13. |

The `rng_xml_oracle.rs` and `rng_compact_roundtrip.rs` fixtures honor a
`CEM_ML_SCHEMA_ORACLE_SKIP=1` escape hatch that mirrors the
`CEM_ML_PERF_SKIP` policy in §17 — skips are recorded as `info` events in
the report, not silent passes.

#### 13.2.8 Nx Target

```
nx run cem_ml:build:schema-artifacts
```

writes every emitter's output for every registered Tier A schema into
`packages/cem_ml/dist/lib/schema/`. The target is cacheable and its inputs
are the CEM-native schema source files plus the emitter module sources.
Outputs are exactly the directory tree above. Release sequencing places
`build:schema-artifacts` after `build:docs` (so the CEM-native source is
already lowered to XHTML) and before the lint/test gates in §18.4.

#### 13.2.9 Open Questions

This section lists decisions that block emitter implementation. They are
tracked in detail in
[`cem-ml-schema-compiler-open-questions.md`](cem-ml-schema-compiler-open-questions.md);
no emitter lands until each item below has a resolution or an explicit
deferral.

- **OQ-SC-1** — Module location: subdirectory `schema/compiler/` vs single
  file `schema/compiler.rs` (impl design §4 currently lists the latter).
- **OQ-SC-2** — `CompiledSchema` field gap: the existing Tier A
  `vocab::CompiledSchema` is vocabulary-only; richer fields
  (`StructuralSchemaIr`, `SemanticRule`, `OpenContentPolicy`,
  `SchemaVersionIdentity`) are designed in §3.4 but unimplemented. The
  compiler-output module can only emit what the IR carries.
- **OQ-SC-3** — Tier of `rust_hdr`: AC-S-4 is `[B] SHOULD`. Confirm whether
  the emitter ships in the Tier A release bundle (gated off by default) or
  is held back entirely until Tier B closes.
- **OQ-SC-4** — URI-tail publication aliasing: whether the on-disk tree
  carries alias directories for partial URI tails (`/1`, `/1.2`) or
  resolves them at load time from the manifest set.
- **OQ-SC-5** — External RELAX NG oracle choice: Jing (Java toolchain),
  `xmllint --relaxng` (libxml2 C toolchain), or a pure-Rust validator.
- **OQ-SC-6** — `Validated<T>` source-map frames in TS: AC-S-V-5 requires
  diagnostic frames derived from the caller's invocation site, which
  implies a TS-side runtime shim. Owner module unclear.
- **OQ-SC-7** — Cross-version `.d.ts` strategy: AC-S-V-4 requires
  `Validated<Badge@1.0>` and `Validated<Badge@2.0>` to be nominally
  distinct when both schemas are loaded in the same TS project. Single
  combined `.d.ts` vs per-version `.d.ts` plus a re-export shim.
- **OQ-SC-8** — Header-comment policy: every emitter can prefix its file
  with a CEM-native source URI + embedded version + content hash. Whether
  the header is part of the byte-stability surface (it is deterministic) or
  excluded from the AC-CC-1 hash (because it embeds the hash recursively).

---

## 14. Rust Module Map

The high-level module topology keeps I/O, parsing, validation, transformation, reporting,
and CLI orchestration separate. Exact structs, traits, and file-level implementation
ownership live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#4-rust-module-map).

```
cem_ml/src/
  source/        byte sources, decoding, line-index projection
  tokenizer/     canonical CEM-ML curly, WHATWG HTML, and XML tokenization profiles
  events/        normalized event taxonomy and token-to-event conversion
  schema/        schema machine, validation state, derivative/DFA backend
  handoff/       embedded content handoff stack and return conditions
  parser/        input DOM/AST reconstruction, CEM AST projection, reference slots
  source_map/    source-map stacks and transform frame kinds
  transform/     content-type transform pipeline, including WHATWG HTML and CSS hooks
  interpreter/   schema-driven CEM transform execution and rendered output
  runtime/       machine state slots, template registry, hydration rules, patch policy
  report/        AST-associated report tree and report renderers
  engine/        I/O-independent execution interface
  command/       I/O-independent command orchestration
  query/         lookup helpers for roles, state, diagnostics, labels, source maps
  ast/           deferred Tier B binary AST encoding and chunking stubs
```

`cem_ml_cli/src/main.rs` owns only Clap argument parsing, cwd/workspace detection,
stdout/stderr writing, and process exit. All parsing, validation, transformation,
reporting, and fixture logic lives in `cem_ml`.

---

## 15. Tier A Scope

Nothing is implemented yet. The table below reflects **design readiness** — whether the
design in this document is complete enough to start implementation without resolving
further open questions first.

Status key:

- **Design ready** — design is complete enough to implement; open sub-questions are
  refinements, not blockers.
- **Design partial** — one or more open concerns in §19–20 must be resolved before
  clean implementation can begin. Blocker references are noted.
- **Deferred Tier B/C** — explicitly out of Tier A scope; interface stubs may be
  defined now for stability.

| Component                                                   | Design status                                                                                                                                                                           |
|-------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| L1 ByteSource: owned bytes, string, file path               | Design ready — finite inputs are source adapters into the async streaming API; complete-source assembly before tokenization is not required (§5)                                          |
| L1 ByteSource: async byte/string streams                    | Design ready — async Rust and WASM input APIs are primary; Tier A parses chunks monotonically; tokenizer buffering is token-local, while editor-style incremental reparse remains Tier B  |
| L1 EncodingDecoder: UTF-8                                   | Design ready — UTF-8 is the fallback when no BOM or explicit/default encoding is present (§5)                                                                                           |
| L1 EncodingDecoder: UTF-16, Latin-1, BOM detection          | Design ready — byte-stream initiation, BOM precedence, BOM skipping, caller/default encoding precedence, in-band declaration handling, and scalar preprocessing are resolved (§5)         |
| L2 SchemaTokenizer: CEM-native curly profile                | Design ready — canonical `{name @attributes \| content...}` syntax, `$` expression nodes, anonymous scopes, rich-content enclosures, and XML parity rules are defined in `cem-ml-syntax.md` |
| L2 SchemaTokenizer: HTML WHATWG profile                     | Design ready — custom WHATWG-state tokenizer selected for exact source-map preservation across nested embedded contexts (§6)                                                            |
| L2 SchemaTokenizer: XML 1.0 profile                         | Design partial — DTD/external-resource ownership follows transform policy (§3.2, §6)                                                                                                     |
| L3 EventNormalizer                                          | Design ready — `ElementBoundary`, header/prelude parameter handling, synthesized close reasons, trivia preservation, `QName` resolution, and `ModeSwitch` context creation are defined (§7–9) |
| L4 SchemaMachine: visibly pushdown frame stack              | Design ready — frame phases, attribute/content trackers, recovery invariant, and diagnostic propagation boundary are resolved (§8, §3.1)                                                  |
| L4 SchemaMachine: RELAX NG derivative engine                | Deferred Tier B — CEM structural schema has RELAX NG functional parity; switching from Tier A DFA to derivatives may break report compatibility (§8, §13)                                |
| L4 SchemaMachine: CEM vocabulary DFA                        | Design partial — limited Tier A DFA profile is selected and open-content defaults are resolved (§8, §13); DFA state table remains unspecified                                             |
| L5 HandoffStack: ownership and return-condition tracking    | Design ready — current context parser recognizes `ModeSwitch`; CEM framework maps entity content type and creates child context with decoded stream (§9)                                  |
| L5 Child parser: CSS (stub, diagnostic only)                | Design ready — container content type decodes before handoff; child receives a source-mapped decoded stream (§9)                                                                        |
| L5 Child parser: Script (raw text only)                     | Design ready — parser preserves raw text; warning/error/reject/allow behavior is defined by active scope/content-type policy (§3.1–3.2, §9)                                             |
| L6 InputDomAstBuilder: schema-defined initial DOM/AST       | Design ready — schema reconstructs token hierarchy; WHATWG DOM compliance is a downstream transformation over this initial DOM                                                          |
| L6 InterpreterAstBuilder: CEM annotation projection         | Design partial — CEM attributes are transform annotations on source nodes; transform conflict policy is schema-owned; comment and rich-content syntax follows `cem-ml-syntax.md`           |
| L6 Reference slots: id/for/aria-*                           | Design ready — one-pass mutable slots are sufficient; unfilled slots warn on owning scope close unless scope policy overrides severity per error type (§10, §3.1)                         |
| L6 Source-map stacks: byte-range + transform chain          | Design ready — frames are origin-first, current frame is last, `FrameSpan::Multi`, boundary frames, and tokenizer-local `EscapeDecoded` frames are defined (§4, §6)                      |
| L6 Source-map stacks: bit-level ranges                      | Deferred Tier B — reserve representation only; no serialized binary frame ids in Tier A (§11)                                                                                           |
| L7 BinaryAstEncoder                                         | Deferred Tier B — Tier A does not freeze serialized binary ids; canonical identity, ordering, and future id policy are scoped in §11                                                    |
| L8 ChunkCompressor                                          | Deferred Tier B — compression profiles are research-backed; canonical chunk identity, ordering, and dependency slots are scoped in §11                                                  |
| ContentTypeTransformPipeline: WHATWG HTML DOM               | Design ready — schema-driven initial HTML parser DOM is transformed into WHATWG implementation DOM updates                                                                              |
| L9 ImplementationInterpreter: schema-driven transform rules | Design ready — schema owns transform layers; namespace-qualified CEM identity resolves source collisions; canonical serialization and HTML `data-*` ownership are defined in §8 and §12 |
| L9 ImplementationInterpreter: transform execution backend   | Design ready for Tier A — Rust CEM template renderer is selected; AC-T-7 owns template embedding and `cem-ql-ac.md` owns expression semantics. Future cem-ql stack design still needs evaluator IR (§12) |
| Visual content and machine state data                       | Design partial — uniform AST role model is defined; live hydration, browser adapters, and DOM patch identity are subject to a separate design phase TBD (§12)                           |
| LineIndex: byte-offset → line/col projection                | Design ready — line/column are report projections over a selected source-map frame; Tier A provides byte-column projection, while scalar, UTF-16, and display columns are Tier B tooling |
| Diagnostics and reports                                     | Design ready — diagnostics always carry event-time source-map stacks, active scope context, origin/boundary scopes, and optional AST-node back-references (§8)                           |
| CLI output projections and fixture round-trip reports       | Design ready — CLI owns projection targets and side outputs; stack layers own projected artifacts                                                                                       |
| Resource and security limits                                | Design ready — source byte, source-chunk, decoder-carry, token-buffer, decoded-chunk, source-map depth, AST depth, and diagnostic caps are defined in Layer 1; XML external-resource limits follow context-scope policy and content-type transforms (§3.1–3.2, §5–6) |
| Incremental/editor parsing                                  | Deferred Tier B — caller-provided diffs map through source maps to changed scopes, with enclosing-scope rescan fallback                                                                 |
| Scope-close reference validation (unfilled slots)           | Design ready — unresolved references emit warnings on owning scope close by default; context-scope policy can override per diagnostic type (§10, §3.1)                                   |
| Per-scope error boundaries                                  | Design ready — each context scope owns error handling and policy; inner scopes may relax or hide own errors only within parent override bounds (§3.1–3.2)                               |
| Async mutation API (`*Async` DOM mutations)                 | Deferred Tier C — runtime-phase surface separate from the required async parse/load/validate/transform APIs                                                                             |

---

## 16. Algorithm Selection Summary

![Multi-format parser atlas illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-parser-atlas.png)

*Multi-format parser atlas.*

| Layer     | Problem                      | Algorithm                                                | Reason from research                                                                           |
|-----------|------------------------------|----------------------------------------------------------|------------------------------------------------------------------------------------------------|
| L2        | CEM-ML tokenization          | Canonical curly CEM-ML tokenizer                         | Keeps authoring syntax canonical while lowering to the same schema event model as parity forms |
| L2        | HTML tokenization            | Custom WHATWG-state tokenizer                            | Browser-compatible; preserves exact source maps across nested embedded contexts                |
| L2        | XML tokenization             | XML 1.0 scanner                                          | Well-defined, same tokenizer contract as HTML                                                  |
| L3        | Cross-format event model     | Open/close/name/value taxonomy                           | Research §3: small event set lets schema validation share algorithms across formats            |
| L4        | Nested validation            | Visibly pushdown frame stack                             | Research §4, §Algorithms: "natural fit for open/close structures"                              |
| L4        | Schema validation Tier A     | Limited CEM DFA profile over RELAX-NG-equivalent IR      | Keeps Tier A bounded while preserving the full structural validation contract                  |
| L4        | Schema validation Tier B     | RELAX NG derivatives over the same structural IR          | Research §XML notes: "residual describes what was expected next" — streaming, good diagnostics |
| L5        | Embedded languages           | Parent-owned handoff with explicit return condition      | Research §5: "child parser never infers parent close condition independently"                  |
| L6        | Initial DOM/AST              | Schema-defined token hierarchy reconstruction            | Drives WHATWG HTML DOM compliance without making tokenization circular                         |
| Transform | WHATWG HTML DOM              | Content-type transform over initial HTML parser DOM      | Applies insertion modes, active formatting elements, foster parenting, and DOM updates         |
| L6        | Forward references           | Mutable scoped name slots                                | Research §4: "slot filled when defining entity arrives"                                        |
| L6        | Source location ground truth | `u64` byte offset                                        | Research Unicode policy: "byte offsets as stable storage format"                               |
| L6        | Line/column                  | On-demand projection via LineIndex                       | Research: "derived coordinates" — never stored, computed from byte offset                      |
| L9        | CEM transform semantics      | Schema-driven CEM template plan                          | Keeps schema in charge of transform layers while Rust executes the generic template renderer    |
| L9        | UI virtualization            | Template reference + machine-state binding               | Reuses templates by reference and applies owned scope data during transformation               |
| L9        | CEM transform backend        | Rust CEM template renderer with scoped query evaluation   | Provides XSLT-like transform coverage while enforcing CEM scope and policy boundaries          |
| Deferred  | Binary AST transport         | Dictionary-encoded subtree chunks                        | Research §Binary AST: parallel delivery, retry, cache reuse                                    |
| Deferred  | Chunk compression            | Zstandard (`canonical-fast`), Brotli (`canonical-dense`) | Research §Compression Strategy                                                                 |

---

## 17. Performance Budgets And Verification

This section resolves DESIGN-FOLLOW-011 by mapping AC-N-1 / AC-N-2 / AC-N-3 to
concrete budget ownership, CI tolerance policy, memory-limit proof fixtures, and a
runnable Nx target. The acceptance criteria remain authoritative; this section names
the data shapes, fixtures, and verification entry points that close the AC.

### 17.1 Budget Ownership

The wall-clock budget named in AC-N-1 is owned by `cem_ml::benchmark::BenchmarkBudget`.
The constructor `BenchmarkBudget::default_ac_n_1()` returns the 150 ms budget on a
developer-class machine, single-thread, cold cache. Future ACs that introduce other
budgets MUST add a sibling constructor (e.g. `default_ac_i_5_hydration_batch`) so the
budget set is enumerable from the same module and the tolerance policy stays in one
place.

The budget object carries:

- `budget: Duration` — the AC-named wall-clock target,
- `tolerance: f64` — multiplier applied before the assertion fires.

`BenchmarkRun::within(&BenchmarkBudget)` compares the **median** of the measured
samples against `budget × tolerance`. Median is the AC-aligned statistic: AC-N-1 names
a per-fixture budget, not a distribution tail, so we accept run-to-run noise as long
as the typical run lands inside the envelope. p95 / p99 are reported for trend
inspection but are not assertion gates in Tier A.

### 17.2 CI Tolerance Policy

Wall-clock budgets are not portable across hosts. The harness applies a tolerance
multiplier to absorb hardware variance:

- Default tolerance: `3.0` — passes on a CI runner that is up to 3× slower than the
  developer machine the budget was calibrated against.
- Override: `CEM_ML_PERF_TOLERANCE=<float>` in the environment. Values below `1.0` are
  clamped to `1.0` so the AC budget itself remains the hard floor.
- Skip: `CEM_ML_PERF_SKIP=1` opts the suite out entirely. Reserved for constrained
  virtualised runners (containers without performance counters, throttled shared-host
  CI) where the wall-clock budget is meaningless. A skipped suite MUST still surface
  in the test report so a release reviewer can confirm the run.
- Build mode: AC-N-1 names release wall-clock budgets. The perf suite MUST run under
  `--release`; debug builds skip automatically via `cfg!(debug_assertions)` so a
  developer running the full test suite in debug does not see spurious failures.

Tolerance is a CI ergonomics knob, not a substitute for the AC budget. A regression
that pushes the median past `budget × 1.0` on the developer machine is a regression
regardless of CI tolerance.

### 17.3 Memory-Limit Proof Fixtures (AC-N-2)

AC-N-2 requires bounded streaming accumulators: tokenizer memory scales with the
current token and open-scope depth, not document byte length. The verification path is
indirect — we cannot directly measure heap residency without instrumenting the
allocator — so the proof is two fixtures whose only difference is byte length:

- **10 MB synthetic fixture.** Built at test time from a repeated, balanced
  `{span @class=cell | x}` unit inside one outer `{main | … }` scope. Depth stays at
  2 throughout, so the tokenizer's depth-scaled state buffer is constant; only token
  count varies. The proof condition is that the per-byte wall-clock rate stays within
  10× of the small-fixture rate (with a 50 ns/byte floor on the small fixture so
  fixed-cost overhead does not create an artificially strict ratio). Super-linear
  accumulator scaling would push the 10 MB rate well past that envelope.
- **Deep-nesting fixture.** Depth = 200 with one leaf token. This isolates the
  depth-scaled component of accumulator memory and proves that the depth axis itself
  is bounded by the active scope policy, not the document body.

Both fixtures complete inside an Nx job budget (30 s envelope for the 10 MB fixture;
the deep-nesting fixture lands inside AC-N-1 directly).

Future work covered by DESIGN-FOLLOW-011 (limit-breach diagnostics): when scope-policy
caps for depth, byte count, or fetch fan-out are surfaced as documented fields, this
section MUST add a fixture per cap that exceeds it and asserts the corresponding
`cem.limit.*` diagnostic.

### 17.4 Benchmark Suite And Nx Target (AC-N-3)

The benchmark suite is `packages/cem_ml/tests/perf_budgets.rs`. Each test:

1. Skips early if `CEM_ML_PERF_SKIP=1` or `cfg!(debug_assertions)`.
2. Loads (or synthesises) the fixture.
3. Runs `cem_ml::benchmark::run_pipeline_iterations_bare` for a fixed iteration count.
4. Asserts the resulting `BenchmarkRun` against the appropriate `BenchmarkBudget`.

The Nx-reachable entry point is:

```bash
yarn nx run cem_ml:bench
```

which lifts to
`cargo test --release --test perf_budgets --target-dir ../../dist/target/cem_ml`.
CI invokes the same target. The target is `cache: false` because the inputs (host
clock, runner shape) are not Nx-tracked; caching a perf pass would mask regressions.

A regression that fails any test in this file MUST be triaged before merge: either
the change is genuinely faster-on-paper but slower in measurement (revert or tune),
or the budget itself needs to move (AC update first, then design follow-up here).

---

## 18. Compatibility And Distribution

This section resolves DESIGN-FOLLOW-012 by naming the support matrix, package
artifacts, and release checks that close AC-C-1 / AC-C-2 / AC-C-3. The acceptance
criteria remain authoritative; this section pins ownership and verification entry
points.

### 18.1 Support Matrix (AC-C-1)

AC-C-1 requires the public API to run identically in modern browsers and on Node ≥ 22:

| Surface       | Targets                                                            | Verification path |
|---------------|--------------------------------------------------------------------|-------------------|
| Browser       | Latest 2 of Chromium, Firefox, Safari (auto-tracked: floor advances when a vendor ships a new major). | Playwright matrix gated on the WASM artifact in `packages/cem_ml`. |
| Node          | Active LTS ≥ 22 (current floor: Node 22; raised when a Node LTS reaches EOL). | Node smoke runs the WASM artifact through the same `apply()` contract used by browsers. |
| Rust (native) | Stable, MSRV pinned in `packages/cem_ml/Cargo.toml`.               | `nx run cem_ml:test` + `nx run cem_ml:lint`. |
| Rust (WASM)   | `wasm32-unknown-unknown`.                                           | `nx run cem_ml:build:wasm`. |

The same public API on browser and Node MUST mean the same observable behaviour for:

- parse / validate / transform results,
- diagnostic shapes (codes, severities, source-map projections per AC-O-3),
- event stream shapes (`onParseEvent`, `onValidate`, `onTransform` per AC-O-1),
- scope-policy enforcement, including resource ceilings per §3.1.

Any surface-specific behaviour MUST be either (a) gated on an explicit host capability
(e.g. `fetch` opt-in), or (b) recorded as a divergence waiver in this section with a
target date and AC reference.

### 18.2 Crate Surface (AC-C-2)

AC-C-2 requires a Rust crate that exposes parser/validator/transform contracts and
compiles to native and WASM targets. Ownership:

- The crate is `cem-ml` in `packages/cem_ml/Cargo.toml`. `crate-type` is
  `["cdylib", "rlib"]` so the same source tree produces both the native rlib for
  downstream Rust callers and the cdylib used by the WASM build.
- Public modules at the AC-F-10 layer boundary (`source`, `tokenizer`, `events`,
  `schema`, `handoff`, `parser`, `ast`, `interpreter`, `observability`, `report`,
  `diagnostics`) are the contract. A breaking change in any of those module paths is
  a semver-major event.
- The Tier A layered-contract test
  (`lib.rs::tests::layered_runtime_contract_types_are_importable`) is the
  compile-time gate that the AC-F-10 names still resolve at the published paths;
  every release MUST run this test as part of `nx run cem_ml:test`.
- WASM compilation is verified by `nx run cem_ml:build:wasm`. A change that breaks
  WASM target compilation but passes the native build is still a release blocker.

### 18.3 CLI And Package Boundaries (AC-C-3)

AC-C-3 requires the Rust crate and CLI package boundaries to stay publishable, with
any future npm/WASM wrapper consuming the Rust-owned contract instead of restoring
the deprecated TypeScript package:

- Crate: `cem-ml` (`packages/cem_ml`). Publish path is `cargo publish` from that
  directory. `publish = true` in `Cargo.toml` is the affirmative gate.
- CLI binary: `cem-ml` (`packages/cem_ml_cli`). The CLI MUST link against the
  `cem-ml` crate by version, not by relative path, in published releases. Internal
  workspace builds use a path dependency; the release script swaps it to a version
  dependency before `cargo publish`.
- Future npm/WASM wrapper: when added, the wrapper MUST re-export the same public
  contracts (engine requests/responses, diagnostic shapes, event stream payloads).
  A wrapper that introduces its own diagnostic or event shape is a violation of this
  AC and MUST be reverted, not paved over with a translation layer.

The deprecated TypeScript package MUST NOT be reintroduced. If a TS consumer needs a
typed surface, the path is generated `.d.ts` from the Rust-owned schema/projection
contracts (AC-S-6), not a parallel TypeScript implementation.

### 18.4 Release Checks

A release of `cem-ml` (or the CLI) MUST pass, in order:

1. `nx run cem_ml:lint` — no Rust warnings or clippy errors.
2. `nx run cem_ml:test` — full Rust test suite including the layered-contract import
   test and all fixture-driven integration tests.
3. `nx run cem_ml:build:wasm` — WASM target compiles.
4. `nx run cem_ml:bench` — AC-N-* perf suite under release profile (skips honour
   `CEM_ML_PERF_SKIP` only on documented constrained runners).
5. Browser/Node smoke: Playwright + Node runners exercise the WASM artifact against
   the canonical `examples/cem-ml/` and `examples/semantic/` fixtures.
6. Manifest checks: `Cargo.toml` version bumped, CHANGELOG entry added, AC items
   closed by this release named in the entry.

A release that skips any of steps 1–4 is not a release. Steps 5–6 are required for
public releases (`crates.io`, npm wrapper) and optional for pre-release tags.

---

## 19. Open Ambiguities

No open ambiguity entries remain in this section. Previously assigned ambiguity IDs are
omitted after resolution; related implementation concerns that still need details are
tracked in §20.

---

## 20. Critical Review Questions And Concerns

This section records unresolved issues found by reviewing this design against the
primary AC and the architectural research. These are not decisions. They are follow-up
questions and concerns to resolve before implementation. Other workspace documents may
provide terminology, but they should not decide the answers here.

### 20.5 Schema-Machine And Validation Questions

---

## 21. Appendix: AC Alignment Follow-Up

Review date: 2026-05-12. The table below is the pre-alignment review that drove the
current AC update. It is retained for provenance. The authoritative follow-up list is
the "Current Design Follow-Up" table after it.

Status terms:

- **Blocker** - Tier A MUST/SHOULD behavior in the AC is contradicted, deferred, or not
  designed here.
- **Gap** - the AC requires a contract but this design has no matching decision.
- **Tier mismatch** - both documents mention the topic, but assign it to different
  tiers or phases.
- **Partial** - the design covers part of the AC but leaves required behavior
  untestable.

| ID           | AC reference                                       | Finding                                                                                                                                                                                                                                                                                                                                                                                                                                            | Required follow-up                                                                                                            |
|--------------|----------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------|
| AC-ALIGN-001 | Preamble, §1.1                                     | Authority is inconsistent. The AC file says it is derivative and this design says AC cannot override design decisions, but this review treats `cem-ml-ac.md` as primary.                                                                                                                                                                                                                                                                           | Decide and update the governance text before implementation planning uses either document as source of truth.                 |
| AC-ALIGN-002 | Goal, feature lists                                | The AC requires broad parity across HTML, XML, XSLT, XPath, SVG, MathML, Canvas, SMIL, JSON, YAML, CSV, JS/TS, Rust, and CEM-ML-Query. This design scopes many of those as conceptual future content types or omits them.                                                                                                                                                                                                                          | Add an explicit parity matrix with tier, owner layer, and "not planned" waivers.                                              |
| AC-ALIGN-003 | Tier A/B descriptions, AC-P-4, AC-P-5              | Tier boundaries conflict. The AC says Tier A has no multi-namespace switching and Tier B adds embedded content-type switching. This design puts namespace resolution, header/prelude namespace changes, and Tier A HTML style/script handoffs in the core pipeline.                                                                                                                                                                                | Retag the design features or revise the AC tier definitions.                                                                  |
| AC-ALIGN-004 | AC-P-1                                             | The AC requires XML 1.0 and HTML5 to parse into the same in-memory DOM model with the same node, attribute, and text APIs. This design defines several projections: source-preserving input DOM/AST, WHATWG implementation DOM, CEM AST, and UI DOM plan; XML 1.0 is still "design partial" in §15.                                                                                                                                                | Define the public shared DOM API or narrow AC-P-1 to the schema-defined input AST.                                            |
| AC-ALIGN-005 | AC-P-2, AC-N-2                                     | The AC requires streaming memory bounded by current open-element depth. This design also keeps line-index checkpoints, diagnostics/report events, source-map stacks, AST nodes, id/reference tables, and bounded token buffers; those are bounded by caps or document content, not only depth.                                                                                                                                                     | Replace the depth-only claim with a precise memory model, or add waivers for report/source-map/AST retention.                 |
| AC-ALIGN-006 | AC-P-3, AC-O-1                                     | The AC requires parse errors with top-level `byteOffset` and stable structured streams named `onParseEvent`, `onValidate`, and `onTransform`. This design uses source-map stacks and report AST projection, but does not define those public event stream names or a required top-level `byteOffset` projection.                                                                                                                                   | Specify the public event API and mandatory diagnostic projection fields.                                                      |
| AC-ALIGN-007 | AC-P-4                                             | The AC defines scope identity as `{ schemaUri, contentType, namespaceUri }` and says scope errors do not propagate beyond that scope unless explicitly re-raised. This design uses richer `ScopePolicy`/context roots and bubbles diagnostics to nearest error-boundary scopes, with parent overrides.                                                                                                                                             | Map the AC three-tuple onto the design's scope model and define when propagation counts as re-raise.                          |
| AC-ALIGN-008 | AC-P-6                                             | The AC requires Tier C NVDL-style schema dispatch. This design has namespace/content-type switching but no explicit NVDL dispatch contract.                                                                                                                                                                                                                                                                                                        | Add a deferred NVDL dispatch section or mark AC-P-6 out of scope.                                                             |
| AC-ALIGN-009 | AC-P-8, AC-S-1                                     | The AC is internally inconsistent: AC-P-8 leaves canonical CEM document syntax open, while AC-S-1 requires CEM-native syntax. This design chooses CEM-native syntax in §13.                                                                                                                                                                                                                                                                        | Record the syntax decision against AC-P-8 and remove the open state if CEM-native is final.                                   |
| AC-ALIGN-010 | AC-S-2, AC-S-3, AC-S-4, AC-S-5, AC-S-6             | The AC requires release emission of byte-stable XML schema mirrors, TypeScript `.d.ts` headers, Rust `.rs` headers, stable schema URIs, and a TS type strategy. This design defines CEM-native schema IR and RELAX NG functional parity but not release artifacts, type-header emission, URI publication, or TS strategy.                                                                                                                          | Add schema compiler outputs and release compatibility rules, or waive those AC items.                                         |
| AC-ALIGN-011 | AC-V-2, AC-V-3                                     | The AC requires semver-compatible namespace behavior: unknown content warns across compatible minor versions and major mismatches fail. This design defines open-content policy but no schema version compatibility model.                                                                                                                                                                                                                         | Add schema URI/version parsing, compatibility comparison, and default diagnostic severity rules.                              |
| AC-ALIGN-012 | AC-V-1, AC-V-4, AC-O-3, AC-O-4                     | The design aligns on AST-associated reports and shared diagnostics, but the AC requires the same error objects to surface through every parser/transpiler layer, including XSLT consumers. This design uses CEM template rendering and does not define XSLT consumer error identity.                                                                                                                                                               | Define error object identity across transform consumers, or narrow the AC to CEM transform consumers.                         |
| AC-ALIGN-013 | AC-V-6, AC-X-3                                     | The AC requires detection of invalid state combinations, missing accessible names, broken `id`/`for`/`aria-*` references, and unsafe inline content. The design covers reference slots and unsafe URL/content policy partially, but ARIA/accessibility validation and concrete state-combination rules are not testable.                                                                                                                           | Add schema-owned semantic checks for accessibility, ARIA, and state compatibility.                                            |
| AC-ALIGN-014 | AC-I-1, AC-I-3, AC-A-2                             | The AC requires an interpreted DOM state machine with `apply(transform)` accepting URI/stream/DOM fragment, parent-node promises for deferrable subtree work, and exclusive subtree ownership for mutations. This design defines parser/interpreter layers and transforms, but not a public DOM `apply()` state-machine API.                                                                                                                       | Design the DOM state-machine API or move these AC items to a later document.                                                  |
| AC-ALIGN-015 | AC-I-4, AC-I-5                                     | The AC requires render-while-parsing and a default 100 ms batching policy in Tier B. This design defers live hydration, DOM patching, and browser adapter behavior to a separate design phase and does not define the 100 ms default.                                                                                                                                                                                                              | Add render/update scheduling policy or retier the AC.                                                                         |
| AC-ALIGN-016 | AC-M-1 through AC-M-14                             | Major conflict: the AC makes async DOM mutation APIs Tier A/B, including `*Async` mutators, `Promise<void>`, `AbortSignal`, ordering, batching, observer timing, rollback, and `flushAsync`. Section 15 explicitly defers the async mutation API to Tier B/C as outside the primary parsing research.                                                                                                                                              | Decide whether async DOM mutation is in Tier A. If yes, add a full mutation state-machine design; if no, update AC-M-* tiers. |
| AC-ALIGN-017 | AC-T-1, AC-T-3, AC-T-4                             | The AC requires XSLT-equivalent transformations and transform loading from URI, `ReadableStream`, or DOM. This design replaces unrestricted XSLT/XPath with a schema-driven CEM template renderer and leaves template/query syntax TBD.                                                                                                                                                                                                            | Define equivalence boundaries, supported XSLT subset, and transform source loading contracts.                                 |
| AC-ALIGN-018 | AC-PL-1 through AC-PL-20                           | The AC defines a Tier B plugin runtime: descriptors, observe/mutate modes, plugin chain inheritance, priority, source-map stitching, built-in transformers as plugins, runtime install/uninstall, budgets, and sandboxing. This design has transform modules and policy concepts but no plugin descriptor, chain, lifecycle, or sandbox model.                                                                                                     | Add a plugin architecture section or mark plugin runtime as deferred outside this design.                                     |
| AC-ALIGN-019 | AC-A-3 through AC-A-7, AC-O-2                      | The AC requires depth-first thread work, per-scope thread pools, bounded queues, external-resource I/O queues, `AbortSignal`, and deterministic scheduler traces. This design defines async APIs and scope resource policy but no scheduler, queue, worker-pool, cancellation, or trace contract.                                                                                                                                                  | Add concurrency/runtime scheduling design with defaults and trace shape.                                                      |
| AC-ALIGN-020 | AC-R-1 through AC-R-3                              | The AC requires DCE registries scoped to parser scopes with inherited lookup and collision warnings. This design models DCE tag names as template references and says CEM does not police the browser `customElements` registry; scoped DCE registry behavior is absent.                                                                                                                                                                           | Add scoped registry data structures and lookup/collision rules or waive AC-R-*.                                               |
| AC-ALIGN-021 | AC-N-1, Verification Plan                          | The AC requires fixture parse/validate/transform under 150 ms and specific Nx verification targets. This design defines no benchmark budget or CI tolerance band.                                                                                                                                                                                                                                                                                  | Add performance budgets and verification ownership to the design.                                                             |
| AC-ALIGN-022 | AC-C-1, AC-C-2, AC-C-3                             | The AC requires modern browser and Node >= 22 API parity, Rust crate native+WASM exposure, and publishable package boundaries. This design mentions Rust/WASM async APIs and package boundaries but not the browser/Node compatibility matrix or distribution gates.                                                                                                                                                                               | Add runtime compatibility and packaging acceptance gates.                                                                     |
| AC-ALIGN-023 | "CEM-ML schema features" and "CEM-ML API features" | The new top-level AC feature lists require syntax for encoding/error level/namespace switching, schema switch/loading inline and by URI, validation error-level switching, template/entity references, parser/schema/validation/interpreter/DOM mutation/transformation/plugin APIs, concurrency, resource management, and source-map support. This design covers several concepts but does not define the required syntaxes or every API surface. | Turn the feature lists into traceable AC IDs and map each to a design section, tier, and owner.                               |
| AC-ALIGN-024 | Open Questions                                     | Pre-alignment finding: several open decisions were resolved or bypassed in design while others were omitted. This has been superseded by the current AC open-question list.                                                                                                                                                                                                                                                                      | Use the current AC open questions and follow-up table below.                                                                  |

### 19.1 Current Design Follow-Up

[`cem-ml-ac.md`](cem-ml-ac.md) is now the primary decision driver. These design updates
remain open:

| ID | AC reference | Design follow-up |
|----|--------------|------------------|
| DESIGN-FOLLOW-001 | AC-S-2 through AC-S-6 | **Design landed (§13.2).** Schema-compiler output module, emitter inventory, byte-stability rules, on-disk layout, URI publication workflow, and verification-fixture roster are now specified. Implementation blocked on the open questions in [`cem-ml-schema-compiler-open-questions.md`](cem-ml-schema-compiler-open-questions.md) (OQ-SC-1..OQ-SC-8). |
| DESIGN-FOLLOW-002 | AC-V-2, AC-V-3 | AC rules are resolved in `cem-ml-ac.md` §3.1 and summarized in this design; implementation structs and tests still need to be added. |
| DESIGN-FOLLOW-003 | AC-P-3, AC-O-1 | `byteOffset` and observer names are now sketched in design/impl; add concrete payload schemas, Rust/WASM API details, and tests. |
| DESIGN-FOLLOW-004 | AC-V-6, AC-X-3 | Add concrete schema-owned semantic checks for accessibility, ARIA, invalid state combinations, and unsafe inline content. |
| DESIGN-FOLLOW-005 | AC-F-2 | Resolved in §13.1; keep implementation follow-up for concrete parser/schema-frame lowering. |
| DESIGN-FOLLOW-006 | AC-I-1 | Design the later runtime `apply(transform)` API shape, or keep it explicitly open until the runtime phase. |
| DESIGN-FOLLOW-007 | AC-M-1 through AC-M-14 | Keep DOM mutation as Tier C and add a separate runtime design before implementation. |
| DESIGN-FOLLOW-008 | AC-PL-1 through AC-PL-20 | Add a plugin architecture section covering descriptor, chain, lifecycle, source-map stitching, budgets, and sandboxing. |
| DESIGN-FOLLOW-009 | AC-A-4 through AC-A-7, AC-O-2 | Add worker-pool, bounded queue, cancellation, external-resource I/O queue, and scheduler trace design. |
| DESIGN-FOLLOW-010 | AC-R-1 through AC-R-3 | Add scoped template/registry lookup and collision behavior for DCE/custom-element integration. |
| DESIGN-FOLLOW-011 | AC-N-1 through AC-N-3 | Resolved in §17. Budget ownership in `cem_ml::benchmark::BenchmarkBudget`; CI tolerance via `CEM_ML_PERF_TOLERANCE`; 10 MB and depth-200 proof fixtures live in `packages/cem_ml/tests/perf_budgets.rs`; Nx entry point `nx run cem_ml:bench`. |
| DESIGN-FOLLOW-012 | AC-C-1 through AC-C-3 | Resolved in §18. Support matrix, crate surface, CLI boundary, and release checks named. Future npm/WASM wrapper MUST consume the Rust-owned contract per §18.3. |
| DESIGN-FOLLOW-013 | AC-P-6 | Add deferred NVDL-style dispatch design for Tier C. |
| DESIGN-FOLLOW-014 | AC-T-3, AC-T-7, cem-ql AC | Resolved at AC level: AC-T-7 owns host embedding and `cem-ql-ac.md` owns expression semantics. Remaining design work is the future cem-ql grammar/evaluator IR companion, not an open CEM-ML syntax question. |

*End of design document. Each ambiguity and review concern above should be resolved with
a brief decision record before the corresponding implementation phase starts. Resolved
items should be struck through and replaced with the chosen option and rationale.*
