# `cem-ml` Stack Design

**Status:** Draft — design only. No implementation code is included.  
**Primary source:** [`parsing-algorithms-research.md`](../parsing-algorithms-research.md)  
**Date:** 2026-05-08

---

## 1. Purpose And Scope

This document translates the layered parser architecture from `parsing-algorithms-research.md`
into a concrete design for the `cem-ml` Rust library and `cem-ml-cli` binary. It fixes:

- layer boundaries and data contracts between them,
- algorithm selections for each layer,
- Rust module topology,
- source-map and diagnostic shapes,
- the Tier A MVP scope, and
- open design decisions that must be resolved before implementation begins.

`parsing-algorithms-research.md` remains the primary architectural source. This
document is the active implementation/design contract derived from that research: it
defines current behavior, tiers, and layer boundaries for `cem-ml` work until the design
is revised. Other workspace documents (acceptance criteria, CLI plan, todo) are
non-authoritative projections and planning aids; they may verify this design, but they
do not introduce or override requirements.

### 1.1 Acceptance Criteria Derivation Policy

Acceptance criteria must be derived from resolved design decisions and layer contracts
in this document. They must not introduce new requirements, syntax, APIs, tiers, or
behavior that are absent from the design.

While this design is still draft, acceptance criteria that conflict with this document
are stale and must be rewritten or ignored for implementation planning. After the
design is complete, independent acceptance criteria should be rewritten from the
completed design or phased out in favor of generated/checklist-style verification
references.

---

## 2. Domain Context

CEM semantic HTML is standard HTML5 augmented with `data-cem-*` attributes that declare
semantic roles:

```html
<main data-cem-screen="login" aria-labelledby="login-title">
  <h1 id="login-title">Sign in</h1>
  <form data-cem-form="sign-in" method="post" action="/session">
    <label for="email">Email</label>
    <input id="email" name="email" type="email" required>
    <button type="submit" data-cem-action="primary">Sign in</button>
  </form>
</main>
```

The five fixtures in `examples/semantic/` (login, registration, profile, assets-list,
message-thread) are the Tier A validation surface. The pipeline must:

1. Read raw HTML bytes, detect/decode encoding, preserve byte offsets throughout.
2. Tokenize as HTML following WHATWG tokenizer states.
3. Normalize tokens into a cross-format event stream (open, close, name, value, …).
4. Validate event structure against the CEM schema using a RELAX NG derivative validator.
5. Handle embedded content (inline `<style>`, `style=""` attributes) through an explicit
   handoff stack.
6. Reconstruct the schema-defined token hierarchy as the initial input DOM/AST with
   source-map stacks and reference slots on every node.
7. Apply content-type transformations over that input DOM/AST. For HTML, WHATWG DOM
   compliance is a schema-driven transformation from the initial HTML parser DOM to the
   implementation DOM.
8. Transform the CEM AST projection to light-DOM custom-element markup (`<cem-screen>`,
   `<cem-form>`, etc.).

---

## 3. Pipeline Overview

```
ByteSource
  └─ EncodingDecoder
       └─ SchemaTokenizer              (WHATWG HTML or XML 1.0 profile)
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
For Tier A, the pipeline runs synchronously in-process against an in-memory byte buffer;
the encoder, compressor, and broadcast/cache paths are absent.

The schema machine instantiates a child pipeline (tokenizer → normalizer → schema
machine) for each embedded content region. The handoff stack controls the boundary and
return condition.

### 3.1 Resource Limit Policy

Depth and count limits are defined by the active content-type policy. The outer content
type owns the effective limits and criticality for the parse scope, including nesting
depth, attributes per element, references per document, residual cache size, chunk count,
diagnostic count, and analogous resource ceilings.

Embedded contexts inherit the outer policy and may only increase restraint: a child
content-type policy can lower a limit or raise the failure criticality, but it cannot
raise a limit or downgrade a fail-closed condition from the parent. This makes resource
behavior monotonic across handoff boundaries. A permissive outer HTML parse may allow a
child CSS parser to impose stricter CSS-specific bounds, but an embedded context cannot
weaken document-level protections.

When a limit is exceeded, the effective policy determines whether the parser records a
recoverable diagnostic and continues in degraded mode, aborts the current scope, or
aborts the full parse.

### 3.2 Unsafe Content And URL Policy

Unsafe-content diagnostics are owned by content-type transformation policies, not by the
tokenizer. The tokenizer preserves source bytes and tokens, the schema machine validates
structure, and the input DOM records URL-bearing attributes and inline content with
source maps. URL-bearing fields are then resolved by the active transformation policy
against the owner context's base URL, module or import map, and substitution rules.

After resolution and before any fetch, execution, embedding, or output materialization,
the same policy applies security restrictions such as allowed schemes, `javascript:`
rejection, same-origin requirements, inline-event handling, `srcdoc` handling,
form-action constraints, and allowed embedded content types.

Each parse or handoff scope has an effective policy inherited from its owner context.
Embedded contexts may tighten restrictions or add content-type-specific validation, but
they may not weaken the outer policy. The outer context may override inner URL mappings
and substitutions; inner scopes cannot override parent mappings in a way that relaxes
security.

---

## 4. Source-Map Model

Source maps are an AST contract, not a diagnostic side table. The research
(§ Source-Map Stack Model) requires every AST node to carry a traversable stack linking
it back to its origin in the byte stream. This model is defined here because it is
referenced by Layers 4 through 9.

### 4.1 Coordinate System

Byte offsets are the stable ground truth (research § Unicode / Design Policy). Line and
column are derived coordinates projected on demand; they are never stored permanently on
AST nodes.

```
ByteRange { start: u64, len: u32 }
    — absolute byte offset from the start of the SourceId's byte buffer.
    — `len: u32` caps a single token at 4 GiB, which is sufficient.
```

Line/column projection is performed by a `LineIndex` that records the byte offset of
each newline in the source file. Projection is `O(log n)` via binary search. The
`LineIndex` is computed once per `SourceId` and cached.

For Tier B+ compressed binary content, the research specifies bit-level ranges
(`bit_start: u64, bit_len: u32`). Bit fields are reserved in the source-map schema but
not populated in Tier A.

### 4.2 Source-Map Stack

```
SourceMapStack:
  frames: [SourceMapFrame, ...]     ordered, earliest context first

SourceMapFrame:
  source_id: SourceId               which byte buffer produced this context
  byte_range: ByteRange             position within that buffer
  transform: TransformKind          what step created or modified this node
```

```
TransformKind:
  HtmlTokenizer
  XmlTokenizer
  EventNormalizer
  SchemaValidation(schema_id: SchemaId)
  CemAstBuilder
  HandoffBoundary { child_content_type: ContentType }
  Implementation                    transform/render step
  BinaryEncoder                     [Tier B]
```

### 4.3 Traversal Example

An `aria-labelledby` reference in a parsed fixture traces as:

```
CemScreen { semantic_id: "login" }
  frame[0]: CemAstBuilder, byte=(0, 50), source=main.html
  frame[1]: SchemaValidation(cem-schema-v1), byte=(0, 50)
  frame[2]: EventNormalizer, OpenScope("main"), byte=(0, 50)
  frame[3]: HtmlTokenizer, StartTag("main"), byte=(0, 50)
```

A CSS rule inside an inline `<style>` element traces through its handoff boundary back
to the parent HTML token:

```
CssDeclaration { property: "background-color" }
  frame[0]: CssParser, byte=(0, 24), source=embedded-style@main.html:88
  frame[1]: HandoffBoundary(text/css), parent_byte=(85, 130), source=main.html
  frame[2]: HtmlTokenizer, StartTag("style"), byte=(85, 100), source=main.html
```

### 4.4 Generated Nodes

Transform-generated nodes (custom-element output with no direct input text) store:

- `transform: TransformKind::Implementation`
- `byte_range`: the nearest owning source range from the CEM AST node that produced them.
- Prior frames: inherited from the CEM AST source chain.

---

## 5. Layer 1 — ByteSource And EncodingDecoder (`cem_ml::source`)

### Purpose

Owns raw bytes, chunking, and encoding detection. Preserves absolute byte offsets through
all subsequent layers.

### Design

Modeled on LLVM `MemoryBuffer` (research § Core Architecture §1): a read-only byte slice
with a guaranteed sentinel byte after the end for fast lexing without bounds checks per
character.

```
SourceId: opaque stable identity for a byte buffer (used in source-map frames)

ByteSource:
  id() -> SourceId
  bytes() -> &[u8]                  read-only; sentinel byte guaranteed at bytes.len()
  byte_range() -> ByteRange         full range of this source

DecodedChunk:
  scalars: [(char, ByteRange)]      Unicode scalar paired with its byte span
  byte_range: ByteRange             range covered by this chunk
  encoding: Encoding                UTF-8, UTF-16LE, UTF-16BE, Latin-1, …
```

Rules from the research (§1):

- Keep absolute `u64` byte offsets for every token and event.
- Keep decoded scalar spans alongside scalars for Unicode-aware validation.
- Preserve raw byte slices for zero-copy diagnostic snippets.
- Validate UTF-8 at ingress for HTML inputs (HTML commonly requires UTF-8 in modern
  pipelines). Allow XML-style entity-specific encodings where the format requires it.

**Ambiguity 1** (see § 17) covers BOM handling differences between HTML and XML inputs.

### Tier A Scope

In-memory byte buffer, string input, and file-path input. No chunked async network
delivery. The async streaming interface (AC-P-2, `ReadableStream`) is Tier B; see
**Ambiguity 7**.

---

## 6. Layer 2 — SchemaTokenizer (`cem_ml::tokenizer`)

### Purpose

Converts decoded input into format-native tokens. The tokenizer is mode-aware and
schema-guided: it knows which lexical modes, embedded boundaries, and delimiter patterns
the schema defines. The schema also defines the token hierarchy that downstream layers
must reconstruct. Structural validation, hierarchy reconstruction, and semantic rules
remain downstream.

### Design

The research (§ HTML format notes) separates the schema-driven HTML tokenizer from the
initial HTML parser DOM and from the WHATWG implementation DOM. The tokenizer extracts
events and switches lexical states. It does not construct either DOM. The schema-defined
token hierarchy is reconstructed by the input DOM/AST builder, and WHATWG DOM
construction/update behavior is applied later as a content-type transformation over that
initial DOM.

```
RawToken:
  kind: HtmlToken | XmlToken
  byte_range: ByteRange
  source_id: SourceId

HtmlToken:
  Doctype { name, public_id, system_id, force_quirks }
  StartTag { name: String, attributes: [(name, value, name_range, value_range)], self_closing }
  EndTag { name: String }
  Text { data: String }
  Comment { data: String }
  ProcessingInstruction { target: String, data: String }
  ParseError { code: HtmlErrorCode }
```

The `StartTag` attributes carry both the name and value ranges so the event normalizer
can emit per-attribute byte offsets into the source-map stack.

WHATWG tokenizer states (data, RCDATA, RAWTEXT, script-data, tag-open, attribute-value,
etc.) are internal to this layer. The tokenizer switches states as required by WHATWG
rules. The schema can select valid tokenizer contexts and embedded-content boundaries,
but it does not rewrite WHATWG lexical behavior. The schema machine receives the
already-switched tokens and validates/reconstructs the schema-defined hierarchy that
later drives the WHATWG DOM transformation.

XML tokenizer follows the same `RawToken` shape using an XML 1.0 profile, keeping Layers
3 and above format-agnostic.

**Ambiguity 2** (see § 17) covers whether to use an existing Rust HTML5 crate or a
custom WHATWG implementation.

---

## 7. Layer 3 — EventNormalizer (`cem_ml::events`)

### Purpose

Converts format-native tokens into a small, cross-format set of normalized event
categories. This is the unification point: the schema machine consumes `NormalizedEvent`
regardless of input format.

### Normalized Event Taxonomy

From the research (§ Core Architecture §3):

```
NormalizedEvent:
  OpenScope  { name: QName, byte_range: ByteRange }
  CloseScope { name: QName, byte_range: ByteRange }
  Name       { value: String, byte_range: ByteRange }
  Value      { value: ScalarValue, byte_range: ByteRange }
  Separator  { kind: SeparatorKind, byte_range: ByteRange }
  ModeSwitch { content_type: ContentType, handoff: HandoffRecord }
  Error      { code: DiagCode, byte_range: ByteRange, severity: Severity }

ScalarValue: Text(String) | Int(i64) | Float(f64) | Bool(bool) | Null
SeparatorKind: ElementBoundary | Comma | Colon | Delimiter | Newline
```

### HTML Token → Normalized Event Mapping

| HTML token | Emitted events |
| --- | --- |
| `StartTag { name, attrs }` | `OpenScope { name }`, then for each attr: `Name { attr_name }` + `Value { attr_value }` |
| `EndTag { name }` | `CloseScope { name }` |
| `Text { data }` | `Value { Text(data) }` |
| `StartTag { name: "style" \| "script" }` | `OpenScope`, then `ModeSwitch { content_type }` |
| `Comment` | Discarded (or `Value` if schema marks comments as significant) |
| `ParseError` | `Error { … }` |

Each `StartTag` emits its `OpenScope` first, then one `Name`+`Value` pair per attribute,
preserving attribute source positions. The CEM `data-cem-*` attributes arrive as
`Name`+`Value` pairs within the element's `OpenScope` group and are handled by the schema
machine in Layer 4.

---

## 8. Layer 4 — SchemaMachine (`cem_ml::schema`)

### Purpose

Validates the normalized event stream against the CEM schema incrementally. Maintains a
push/pop stack of schema frames. The primary algorithm is a RELAX NG derivative validator
(or Tier A hand-written DFA; see **Ambiguity 9**).

### Algorithm Selection

From the research algorithm comparison table (§ Practical Algorithm Choices):

- **Nested events → visibly pushdown frame stack.** Start tags push frames, end tags pop
  them. Attributes and text update the current frame. This is the formal model the
  research recommends for streaming nested data.
- **Schema validation → RELAX NG derivatives.** After each event, compute the residual
  schema `D(event, schema)`. If the residual is the empty language, emit a hard error.
  This "gives a natural streaming algorithm and can improve diagnostics because the
  residual describes what was expected next" (research § XML format notes). This is
  preferred over DFA for CEM because CEM schemas permit unknown attributes as warnings
  (semver-compatible drift), not errors — RELAX NG derivatives handle open content models
  gracefully.

**Ambiguity 9** covers whether to use a full RELAX NG derivative engine or a Tier A
hand-written DFA for the constrained CEM vocabulary.

### Frame Stack

Directly from the research (§ Proposed Runtime Model):

```
SchemaFrame:
  schema_id: SchemaId
  language_id: ContentType          e.g. text/html, text/css
  state: SchemaState                current RELAX NG residual or DFA state
  source_span: ByteRange            range of the element that opened this frame
  source_map_stack: SourceMapStack  accumulated map at frame entry
  expected_close: Option<QName>     for element-level close validation
  namespace_ctx: Option<NsContext>
  seen_names: HashSet<String>       attribute names seen so far (for required-attr tracking)
  diagnostics: Vec<Diagnostic>
```

### State Transitions

```
open(event):
  Validate OpenScope name against current state.
  Push child SchemaFrame; compute initial residual for child schema.

value(event):
  Validate scalar type, range, pattern against current state.
  Update frame's seen_names if event is a Name.

separator(event):
  Advance sequence, record, or property pointer in current state.

handoff(event):
  Emit HandoffRecord (see Layer 5).
  Push child frame with child content_type and child schema_id.

close(event):
  Validate nullable/complete state (residual accepts empty string).
  Pop frame; propagate close result to parent frame.

error(event):
  Record Diagnostic on current frame.
  Run recovery: for non-Fatal, continue with a permissive residual.
  For Fatal, abort current scope.

transform(event):
  Append SourceMapFrame to current source_map_stack.

encode(node):          [Tier B] assign binary node ids and dictionary refs.
segment(subtree):      [Tier B] close a subtree-root chunk.
```

### CEM Vocabulary In The Schema

The schema defines which attribute names carry CEM semantic roles and what their allowed
values and nesting rules are. Based on the component surface and fixture vocabulary:

```
CEM semantic attributes:
  data-cem-screen   — screen/page root
  data-cem-form     — form boundary
  data-cem-action   — interactive action (button, link)
  data-cem-list     — data list / navigation list
  data-cem-card     — card container
  data-cem-thread   — message thread container
  data-cem-message  — individual message
  data-cem-badge    — badge / status label

Allowed state values (on any CEM-attributed element):
  data-cem-state: default | hover | focus-visible | active | selected
                | disabled | invalid | required | loading | empty
```

Nesting rules, required sibling relationships, and allowed parent elements are expressed
in the schema; the schema machine validates against them through the frame stack.

The schema source language is **Ambiguity 3**.

### Diagnostics Shape

```
Diagnostic:
  uri: String                       document URI or file path
  line: u32                         1-based, derived from byte_offset
  column: u32                       1-based, derived from byte_offset
  byte_offset: u64                  ground-truth position (see § 4)
  code: DiagCode                    stable enumerated code
  severity: Severity { Fatal | Error | Warning | Info }
  message: String
  node: Option<AstNodeId>           AST node reference when available

```

`Fatal` aborts the current scope. `Error` and `Warning` continue in diagnostic mode with
a permissive residual.

### Report Event Model

Reports are owned by `cem_ml::report`, but their canonical internal data is an
AST-associated report tree rather than a flat diagnostic list. Each parser,
schema, handoff, transform, validation, or runtime log message is captured as a
report event node attached to:

- the current input DOM/AST or CEM AST node when one exists;
- the current source module state, including URI, content type, schema id, active scope,
  and source span;
- the source-map stack as it exists at the moment the event is emitted;
- the partial DOM/AST hierarchy visible to the emitting layer at that moment; and
- a monotonic event sequence number that preserves emission order within the report.

The report hierarchy follows the source-map/layer hierarchy, but it is event-time state:
it records the parser or transform view when the log event happened, not the final
post-transform tree. Diagnostics before AST construction attach to the nearest source
module frame and are later linked to AST nodes when a matching node exists.

The canonical report tree can be projected to CEM-native, XML, JSON, or any other
supported structured format. Text and HTML reports are reference-implementation
convenience renderers over the report tree, not canonical report storage formats.

### CLI Projection And Target Ownership

Stack layers own data artifacts. The CLI owns projection selection, output targets, and
default stream behavior. A command may expose one primary output, one report output, and
zero or more side outputs that address intermediate stack layers.

Target rules:

- Primary output goes to `--out` when provided; otherwise it goes to `stdout`.
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

Proposed projection layer keys (names TBD before implementation):

| Key | Stack owner | Projection meaning |
| --- | --- | --- |
| `source` | `source::ByteSource` | Source metadata, URI, byte length, and source id; raw bytes are not emitted unless explicitly requested. |
| `decoded` | `source::decode` | Encoding result, decoded scalar spans, replacement/encoding diagnostics, and line-index metadata. |
| `tokens` | `tokenizer` | Format-native token stream with byte ranges. |
| `events` | `events` | Normalized open/close/name/value/separator/mode-switch/error events. |
| `schema-frames` | `schema` | Schema frame transitions, residual/DFA state, expected closes, and validation state. |
| `handoffs` | `handoff` | Embedded content boundaries, inherited context, child content type, and return condition. |
| `input-dom` | `parser::input_dom` | Schema-defined initial DOM/AST hierarchy reconstructed from tokens/events. |
| `whatwg-dom` | `transform::whatwg_html` | WHATWG implementation-DOM update projection from the initial HTML parser DOM. |
| `cem-ast` | `parser` | CEM semantic AST projection over the input DOM/AST. |
| `transform-output` | `interpreter` / transform modules | Rendered or transformed content for the selected output content type. |
| `source-map` | `source_map` | Source-map stacks and event-time source-map hierarchy. |
| `report-ast` | `report` | Canonical AST-associated report tree with event sequence and source module state. |
| `trace` | `engine` / `command` | Deterministic execution trace assembled from parser, validator, transform, and report events. |
| `binary-ast` | `ast::encode` | Deferred binary AST representation. |
| `chunks` | `ast::chunk` / `ast::compress` | Deferred subtree chunk and compression metadata. |

Fixture round-trip output is a CLI composition of projections, not a separate stack
layer. It records selected inputs, chosen projection layer(s), rendered outputs or hashes,
report AST summaries, and diagnostics.

---

## 9. Layer 5 — Scoped Embedded Handoff Stack (`cem_ml::handoff`)

### Purpose

Manages embedded content regions where the active parser and schema must switch to a
different content type. Makes embedded language boundaries explicit and parent-owned. A
child parser never infers the parent's return condition independently (research §5).

### Handoff Record

```
HandoffRecord:
  child_content_type: ContentType
  byte_span: Option<ByteRange>       known upfront for buffered embedded regions
  inherited_ctx: InheritedContext    parent element name, attribute name, MIME type, namespace
  child_schema_id: Option<SchemaId>
  return_condition: ReturnCondition

ReturnCondition:
  MatchingEndTag(QName)              e.g. </style>, </script>
  AttributeEnd                       attribute quote close
  StringEnd                          JSON string end after unescape
  BlockClose                         CSS block close
  FixedLength(u64)
```

### CEM Tier A Handoff Cases

| Parent context | Trigger | Child content type | Return condition |
| --- | --- | --- | --- |
| HTML document | `<style>` start tag | `text/css` | `</style>` end tag |
| HTML element | `style=""` attribute | `text/css` (declaration block) | attribute quote end |
| HTML document | `<script>` start tag | raw text (not parsed Tier A) | `</script>` end tag |

For Tier A, the CSS child parser is a stub that emits diagnostics but does not produce
a typed CSS AST. Script regions are treated as raw text. The handoff stack is implemented
fully in Tier A to keep the interface stable for Tier B content-type expansion.

### Rule: Parent Owns The Return Condition

The parent schema machine emits a `HandoffRecord` with the exact `ReturnCondition` before
yielding the byte stream to the child parser. The child parser consumes bytes up to the
return condition and signals completion. The parent resumes with the byte following the
condition boundary.

The one WHATWG-specified exception is script-data mode: `</script>` ends the script
region according to WHATWG tokenizer rules regardless of the child parser. This is
modeled as `MatchingEndTag("script")` on the `HandoffRecord` but driven by the WHATWG
tokenizer state, not the schema machine.

---

## 10. Layer 6 — InputDomAstBuilder / InterpreterAstBuilder (`cem_ml::parser`)

### Purpose

Converts the validated, normalized event stream into the schema-defined input DOM/AST.
For HTML, this is the initial HTML parser DOM: a source-preserving reconstruction of the
schema-defined token hierarchy, not the WHATWG implementation DOM. Every node carries a
`SourceMapStack`, attributes, and reference slots for unresolved `id`/`for`/`aria-*`
targets.

The typed CEM AST is a semantic projection over that input DOM/AST. It adds semantic
roles, state labels, CEM node kinds, and CEM-specific reference helpers without changing
the initial parser DOM.

### CEM AST Node Shapes

```
CemNode:
  Document(CemDocument)
  Screen(CemScreen)
  Form(CemForm)
  Action(CemAction)
  List(CemList)
  Card(CemCard)
  Thread(CemThread)
  Message(CemMessage)
  Badge(CemBadge)
  HtmlElement(HtmlElement)     non-CEM HTML elements (pass-through)
  Text(TextNode)

CemDocument:
  source_id: SourceId
  root_children: Vec<AstNodeId>
  id_table: HashMap<String, AstNodeId>   global id map for reference resolution
  diagnostics: Vec<Diagnostic>

CemScreen:
  node_id: AstNodeId
  semantic_id: String            value of data-cem-screen attribute
  label: Option<LabelRef>        resolved or pending aria-labelledby reference
  children: Vec<AstNodeId>
  source: SourceMapStack
  attrs: AttributeMap
  state: Option<CemState>
```

`CemForm`, `CemAction`, `CemList`, `CemCard`, `CemThread`, `CemMessage`, `CemBadge`
follow the same shape: `node_id`, `semantic_id` (the `data-cem-*` value), `children`,
`source`, `attrs`, `state`.

### Reference Slots (Unresolved Forward References)

From the research (§4): "Unresolved references point to mutable scoped name slots. When a
target token, declaration, or entity is defined, the interpreter updates that slot, so
existing references observe the value through the shared binding."

```
NameSlot: Arc<Mutex<Option<AstNodeId>>>

LabelRef(NameSlot)          — wraps a slot; filled when id="…" element is parsed
ForRef(NameSlot)            — for/id pairing
AriaRef(NameSlot)           — aria-labelledby, aria-describedby, etc.
```

When the parser encounters an element with an `id` attribute, it looks up the slot in
the document's `id_table` and fills it. Any prior `LabelRef`/`ForRef`/`AriaRef` holding
the same slot Arc observes the fill immediately.

Forward references (referencing an `id` that appears later in the stream) are represented
as unfilled slots at parse time. The schema machine performs a post-parse reference check
to identify remaining unfilled slots and emit `Warning` diagnostics (broken references).
See **Ambiguity 6** for the trade-off between one-pass and two-pass resolution.

### Source-Map Stack Population

The `InputDomAstBuilder` appends a source-map frame when reconstructing the
schema-defined token hierarchy. The `InterpreterAstBuilder` appends a
`TransformKind::CemAstBuilder` frame when creating each CEM projection node. The prior
frames come from the `SchemaFrame.source_map_stack` at the point the element was
validated.

For generated nodes (CEM nodes inferred from schema defaults, not directly present as
tokens), the nearest owning source range is used and the `transform` field records
`CemAstBuilder`.

---

## 11. Layers 7–8 — BinaryAstEncoder And ChunkCompressor (Deferred Tier B)

### Design Intent

The research (§ Binary AST, Compression, And Segmentation) describes the binary layer as
the internal transport and cache format because it can encode node kinds, schema ids,
scope slots, source-map stacks, string tables, and typed values without repeated textual
markup.

```
BinaryAstEncoder responsibilities:
  — Assign stable binary node ids.
  — Reference platform and app dictionaries by id.
  — Emit source-map deltas (not full frames) to compress the map chain.
  — Assign subtree chunk ownership.

ChunkCompressor responsibilities:
  — Platform dictionary: common AST node kinds, primitive encodings, schema defs, shared strings.
  — App dictionary: CEM-specific node kinds, local symbol tables, repeated literals.
  — Payload chunks: subtree AST nodes, scope slots, source-map deltas, embedded ranges.
```

Chunk boundaries align to subtree roots. Each chunk is independently decodable and
carries integrity hashes, dependency ids, and dictionary version requirements.

Compression profiles from the research:

| Profile | Algorithm | Use case |
| --- | --- | --- |
| `none` | Uncompressed binary | Debugging, tests, memory-mapped storage, environments where compression cost exceeds savings |
| `canonical-fast` | Zstandard with shared dictionary | Interactive delivery, most networked runtimes |
| `canonical-dense` | Brotli or high-level Zstandard | Cold storage, batch transfer |
| `solid-archive` | Whole-document compression | Cold storage only; no parallel decode or retry |

### Tier A Stub

For Tier A, the pipeline skips these two layers. The `InputDomAstBuilder` and
`InterpreterAstBuilder` outputs are in-memory Rust trees with no binary encoding. The
`encode` and `segment` state transitions on the `SchemaMachine` are no-ops. The module
boundaries and trait signatures are defined so that adding the real encoder does not
change the external API of the schema machine or AST builder.

### Incremental And Editor Mode (Deferred Tier B)

Incremental/editor support is classified as Tier B. Tier A remains a batch/fixture path,
but Tier A source maps and scope boundaries must preserve enough information for later
incremental reuse.

The runtime does not compute text diffs itself. An editor, version-control integration,
or document store provides changed byte ranges or an equivalent diff. The runtime maps
those ranges through source-map stacks to the smallest owning schema scopes whose prior
source spans overlap the change. Those scopes are invalidated and reparsed with their
parent-owned handoff boundaries.

Unchanged sibling scopes may be reused only when all of these remain stable:

- source-map range and source id;
- schema id and content type;
- parent-owned return condition or delimiter boundary;
- namespace/context frame;
- dependency slots that cross into or out of the changed scope.

When the change crosses a scope boundary, changes a schema or content type, modifies an
embedded return condition, or touches unresolved/cross-scope references, the runtime uses
the conservative fallback: rescan the nearest enclosing stable scope and revalidate the
partially reused tree. This slower approach is valid for all incremental cases and is the
required fallback before accepting a mixed old/new tree.

Validation after an incremental edit recomputes affected ancestors, changed scopes, and
references that cross between changed and reused scopes. Report events for incremental
passes attach to the event-time partial tree, just like batch reports.

---

## 12. Layer 9 — ImplementationInterpreter (`cem_ml::interpreter`)

### Purpose

Consumes the validated typed CEM AST and produces the target output. For CEM Tier A, the
interpreter is the transform pipeline: semantic HTML → light-DOM custom-element markup
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

Other content-type transformations use the same model: SCSS can lower to CSS, CSS can
resolve `url()`/`@import` references into parsed child ASTs or unresolved resource slots,
and CEM semantic HTML can lower to custom-element markup while preserving source-map
stacks.

### Transform Interface

```
CemInterpreter trait:
  transform(doc: &CemDocument, ctx: &TransformContext) -> Result<TransformOutput, CemError>

TransformContext:
  schema_id: SchemaId
  base_uri: Option<String>
  fail_level: FailLevel

TransformOutput:
  markup: String                 serialized light-DOM HTML
  diagnostics: Vec<Diagnostic>
  source_maps: Vec<SourceMapStack>   one per output element
```

### Transform Rules (Tier A)

Each CEM semantic node maps to a custom element. The `data-cem-*` attribute becomes a
`cem-id` attribute on the generated element. Other standard HTML attributes (class, id,
ARIA, data-*) pass through.

| CEM node | Source element (typical) | Output custom element |
| --- | --- | --- |
| `CemScreen { id }` | `<main data-cem-screen="id">` | `<cem-screen cem-id="id">` |
| `CemForm { id }` | `<form data-cem-form="id">` | `<cem-form cem-id="id">` |
| `CemAction { variant }` | `<button data-cem-action="primary">` | `<cem-action variant="primary">` |
| `CemList` | `<ul data-cem-list>` | `<cem-list>` |
| `CemCard` | `<div data-cem-card>` | `<cem-card>` |
| `CemThread` | `<ul data-cem-thread>` | `<cem-thread>` |
| `CemMessage` | `<article data-cem-message>` | `<cem-message>` |
| `CemBadge` | Any with `data-cem-badge` | `<cem-badge>` |
| `HtmlElement` | Any other element | Pass through unchanged |

Children are transformed recursively. Text nodes pass through unchanged.

### Source-Map Preservation

Each output custom-element node appends a `TransformKind::Implementation` frame. The
prior frames (inherited from the CEM AST node's `SourceMapStack`) trace back to the
original HTML token. This enables tooling to resolve from a generated `<cem-screen>` all
the way back to the raw byte range of `<main data-cem-screen="login">`.

### XSLT vs. Hand-Written Rules

The research recommends "XSLT transform helpers from semantic fixtures into light-DOM
custom-element markup." For Tier A, the transform rules are implemented as hand-written
Rust match arms. A full XSLT engine is Tier C. **Ambiguity 4** covers whether Tier A
needs a minimal XSLT-like template DSL or whether hard-coded Rust rules are sufficient
for the five-fixture surface.

---

## 13. CEM Schema Language

The schema machine requires a machine-readable CEM schema. The research establishes
RELAX NG derivatives as the validation algorithm. The language in which the schema is
*authored* is open.

### Options

**Option A — RELAX NG compact syntax (`.rnc`)**: The validation algorithm uses RELAX NG
derivatives, so authoring in RELAX NG compact is a natural match. The HTML5 validator
(`validator.nu`) uses an HTML5 RELAX NG schema; this is a proven path. Drawback: authors
must know RELAX NG compact syntax.

**Option B — Invisible XML grammar (`.ixml`)**: The research describes iXml as a
schema-driven tokenizer/parser profile for exposing non-XML text as XML events. CEM HTML
is already HTML, so iXml's primary use case is not dominant here. iXml could define a
CEM-specific DSL syntax (e.g., component shorthand), but it adds a grammar compiler
dependency. Drawback: less mature tooling than RELAX NG.

**Option C — CEM markdown tables compiled to RELAX NG**: The existing `cem-colors.md`
style (h6 + table) is already a declarative schema format. A CEM schema compiler could
read these tables and emit `.rnc` for use by the derivative runtime. This is
CEM-friendly authoring with a proven validation backend.

**Option D — CEM-native declarative format**: A purpose-built format tailored to CEM's
vocabulary (roles, states, token tiers, component names). Compiled by a dedicated schema
compiler. Requires the most up-front work but gives the most semantic precision.

All options except B ultimately feed the same RELAX NG derivative runtime. The choice
affects what the schema compiler must implement, not the schema machine itself.

This is **Ambiguity 3**.

---

## 14. Rust Module Map

```
cem_ml/src/
  lib.rs
  source/
    mod.rs            ByteSource trait, SourceId, ByteRange
    decode.rs         EncodingDecoder, DecodedChunk, Encoding
    line_index.rs     LineIndex — byte offset → (line, col) projection
  tokenizer/
    mod.rs            RawToken, SchemaTokenizer trait
    html.rs           WHATWG HTML tokenizer profile
    xml.rs            XML 1.0 tokenizer profile
  events/
    mod.rs            NormalizedEvent, EventNormalizer
  schema/
    mod.rs            SchemaMachine, SchemaFrame, SchemaState
    derivative.rs     RELAX NG derivative computation
    vocab.rs          CEM vocabulary constants (role names, state names, attr names)
  handoff/
    mod.rs            HandoffRecord, HandoffStack, ReturnCondition, InheritedContext
  parser/
    mod.rs            InputDomNode, CemNode enum, CemDocument
    input_dom.rs      schema-defined initial DOM/AST reconstruction
    nodes.rs          CemScreen, CemForm, CemAction, CemList, CemCard, CemThread, CemMessage, CemBadge
    slots.rs          NameSlot, LabelRef, ForRef, AriaRef — reference resolution
  source_map/
    mod.rs            SourceMapStack, SourceMapFrame, TransformKind
  transform/
    mod.rs            content-type transformation pipeline
    whatwg_html.rs    initial HTML parser DOM → WHATWG implementation DOM update transform
    css.rs            CSS/SCSS AST and external-reference transform hooks
  interpreter/
    mod.rs            CemInterpreter trait, TransformContext, TransformOutput
    transform.rs      CEM semantic HTML → custom-element transform rules
  diagnostic.rs       Diagnostic, Severity, DiagCode
  fail_level.rs       FailLevel enum, fail-level evaluation
  report/
    mod.rs            report model structs
    ast.rs            AST-associated report tree and event nodes
    json.rs           JSON report rendering
    xml.rs            XML report rendering
    cem.rs            CEM-native report rendering
    text.rs           reference text report rendering
    html.rs           reference HTML report rendering
    markdown.rs       Markdown report rendering
  formats.rs          parse output format names (dom-json, ast, events)
  fixture.rs          default fixture paths, report path policy
  engine/
    mod.rs            CemMlEngine trait (I/O-independent)
    fake.rs           FakeEngine for CLI feature tests
  command/
    mod.rs            I/O-independent command orchestration
  error.rs            CemError, usage/IO/schema/transform/plugin error variants
  query/
    mod.rs            role lookup, state lookup, validation messages, label resolution, source-map lookup
  ast/                [Tier B] binary AST encoding
    encode.rs         BinaryAstEncoder — node ids, dictionary refs, source-map deltas
    compress.rs       ChunkCompressor — platform/app dictionary, payload chunks
    chunk.rs          chunk metadata, integrity hash, dependency list
```

`ast/` sub-modules are reserved stubs in Tier A. Their interfaces are defined but their
bodies are no-ops.

`cem_ml_cli/src/main.rs` owns only: Clap argument parsing, cwd/workspace detection,
stdout/stderr writing, and process exit. All logic lives in `cem_ml`.

---

## 15. Tier A Scope

Nothing is implemented yet. The table below reflects **design readiness** — whether the
design in this document is complete enough to start implementation without resolving
further open questions first.

Status key:
- **Design ready** — design is complete enough to implement; open sub-questions are
  refinements, not blockers.
- **Design partial** — one or more open concerns in §17–18 must be resolved before
  clean implementation can begin. Blocker references are noted.
- **Deferred Tier B/C** — explicitly out of Tier A scope; interface stubs may be
  defined now for stability.

| Component                                                       | Design status                                                                                                                    |
|-----------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------|
| L1 ByteSource: in-memory buffer, string, file path              | Design partial — source ownership/resource bounds need decisions (§18.3.1, §18.3.3)                                              |
| L1 ByteSource: async network streaming                          | Deferred Tier B — Tier A interfaces must still preserve absolute offsets for future chunked input                                |
| L1 EncodingDecoder: UTF-8                                       | Design partial — UTF-8-only CEM profile vs WHATWG encoding detection unresolved (§18.3.2)                                        |
| L1 EncodingDecoder: UTF-16, Latin-1, BOM detection              | Design partial — required for HTML/XML profile clarity; blocked by Ambiguity 1 and §18.3.2                                       |
| L1 Sentinel-byte ownership                                      | Design partial — Rust safety model for sentinel not resolved (§18.3.1)                                                           |
| L2 SchemaTokenizer: HTML WHATWG profile                         | Design partial — crate choice and token offset behavior unresolved (Ambiguity 2)                    |
| L2 SchemaTokenizer: XML 1.0 profile                             | Design partial — namespace/name model plus DTD/entity/external-resource policy unspecified (§18.4.4, §18.10.1)                   |
| L3 EventNormalizer                                              | Design partial — attribute-list close event, void elements, name model, trivia, and ModeSwitch ownership unspecified (§18.4.1–4, §18.6.1) |
| L4 SchemaMachine: visibly pushdown frame stack                  | Design partial — recovery invariant, multiplicity/required-name state, and diagnostic propagation affect core semantics (§18.5.3–4, Ambiguity 8) |
| L4 SchemaMachine: RELAX NG derivative engine                    | Deferred Tier B — Tier A DFA must preserve a replacement path for residual diagnostics (Ambiguity 9, §18.5.1)                   |
| L4 SchemaMachine: CEM vocabulary DFA                            | Design partial — DFA state table, schema source, vocabulary ownership, and unknown-content policy unspecified (Ambiguity 3, Ambiguity 9, §18.5.1–2, §18.7.4) |
| L5 HandoffStack: struct and return-condition tracking           | Design partial — authoritative `HandoffRecord` owner and deferred return-condition variants unresolved (§18.6.1, §18.6.4)        |
| L5 Child parser: CSS (stub, diagnostic only)                    | Design partial — embedded-source byte/decoded-view model unspecified (§18.6.2)                                                    |
| L5 Child parser: Script (raw text only)                         | Design partial — script preservation policy unspecified (§18.6.3); unsafe-content validation follows content-type policy (§3.2)  |
| L6 InputDomAstBuilder: schema-defined initial DOM/AST            | Design ready — schema reconstructs token hierarchy; WHATWG DOM compliance is a downstream transformation over this initial DOM   |
| L6 InterpreterAstBuilder: typed CEM AST projection               | Design partial — multiple CEM roles per element, non-CEM construct handling, and vocabulary source unresolved (§18.7.3–5)        |
| L6 Reference slots: id/for/aria-*                               | Design partial — slot implementation model, lifecycle, override, duplicate, and cross-scope rules unresolved (Ambiguity 6, §18.7.1–2) |
| L6 Source-map stacks: byte-range + transform chain              | Design partial — frame order, multi-range nodes, escape/entity decoding, and diagnostics-before-AST mapping unresolved (§18.2.1–3, §18.2.5) |
| L6 Source-map stacks: bit-level ranges                          | Deferred Tier B — reserve representation only after source-map frame model is fixed (§18.2.1–2, §18.9.1)                         |
| L7 BinaryAstEncoder                                             | Deferred Tier B — do not freeze IDs or trait signatures until binary determinism and slot identity are resolved (§18.9.1–3)      |
| L8 ChunkCompressor                                              | Deferred Tier B — compression profiles are research-backed, but chunk determinism and cross-chunk references remain open (§18.9.2–3) |
| ContentTypeTransformPipeline: WHATWG HTML DOM                   | Design ready — schema-driven initial HTML parser DOM is transformed into WHATWG implementation DOM updates                       |
| L9 ImplementationInterpreter: hand-written Rust transform rules | Design partial — transform engine choice, attribute collision, data-cem-* pass-through, serialization, and future template seam unresolved (Ambiguity 4, §18.8.1–4) |
| L9 ImplementationInterpreter: XSLT template engine              | Deferred Tier C — minimal Tier A template abstraction still needs a decision through Ambiguity 4                                 |
| LineIndex: byte-offset → line/col projection                    | Design partial — column-unit model, newline normalization, tabs, replacement chars, and UTF-16/scalar projections unspecified (§18.2.4) |
| Diagnostics and reports                                         | Design partial — source-map ownership and diagnostics-before-AST mapping unresolved (§18.2.5)                                      |
| CLI output projections and fixture round-trip reports           | Design ready — CLI owns projection targets and side outputs; stack layers own projected artifacts                                 |
| Resource and security limits                                    | Design partial — byte/decode bounds and XML entity policy unresolved (§18.3.3, §18.10.1); depth/count and unsafe-content limits follow content-type policies (§3.1–3.2) |
| Incremental/editor parsing                                      | Deferred Tier B — caller-provided diffs map through source maps to changed scopes, with enclosing-scope rescan fallback           |
| Post-parse reference validation (unfilled slots)                | Design partial — Warning vs Error severity unresolved (Ambiguity 6 sub-question)                                                 |
| Per-scope error boundaries                                      | Deferred Tier B (Ambiguity 5)                                                                                                    |
| Async mutation API (`*Async` DOM mutations)                     | Deferred Tier B/C — outside the primary parsing research; requires separate runtime API design                                    |

---

## 16. Algorithm Selection Summary

| Layer    | Problem                      | Algorithm                                                | Reason from research                                                                           |
|----------|------------------------------|----------------------------------------------------------|------------------------------------------------------------------------------------------------|
| L2       | HTML tokenization            | WHATWG tokenizer states                                  | Browser-compatible; separates token extraction from DOM                                        |
| L2       | XML tokenization             | XML 1.0 scanner                                          | Well-defined, same `RawToken` shape as HTML                                                    |
| L3       | Cross-format event model     | Open/close/name/value taxonomy                           | Research §3: small event set lets schema validation share algorithms across formats            |
| L4       | Nested validation            | Visibly pushdown frame stack                             | Research §4, §Algorithms: "natural fit for open/close structures"                              |
| L4       | Schema validation Tier A     | Hand-written CEM DFA                                     | Simple constrained vocabulary; allows derivative upgrade without API change (Ambiguity 9)      |
| L4       | Schema validation Tier B     | RELAX NG derivatives                                     | Research §XML notes: "residual describes what was expected next" — streaming, good diagnostics |
| L5       | Embedded languages           | Parent-owned handoff with explicit return condition      | Research §5: "child parser never infers parent close condition independently"                  |
| L6       | Initial DOM/AST              | Schema-defined token hierarchy reconstruction            | Drives WHATWG HTML DOM compliance without making tokenization circular                         |
| Transform | WHATWG HTML DOM             | Content-type transform over initial HTML parser DOM       | Applies insertion modes, active formatting elements, foster parenting, and DOM updates          |
| L6       | Forward references           | Mutable scoped name slots (Arc<Mutex<Option<NodeId>>>)   | Research §4: "slot filled when defining entity arrives"                                        |
| L6       | Source location ground truth | `u64` byte offset                                        | Research Unicode policy: "byte offsets as stable storage format"                               |
| L6       | Line/column                  | On-demand projection via LineIndex                       | Research: "derived coordinates" — never stored, computed from byte offset                      |
| L9       | CEM transform Tier A         | Hand-written Rust match rules                            | Research: "XSLT transform helpers" — minimal subset acceptable                                 |
| L9       | CEM transform Tier B/C       | XSLT subset / full XSLT engine                           | Research: XSLT-equivalent transform; Tier C full XSLT 4.0                                      |
| Deferred | Binary AST transport         | Dictionary-encoded subtree chunks                        | Research §Binary AST: parallel delivery, retry, cache reuse                                    |
| Deferred | Chunk compression            | Zstandard (`canonical-fast`), Brotli (`canonical-dense`) | Research §Compression Strategy                                                                 |

---

## 17. Open Ambiguities

Each ambiguity is a design decision that must be resolved before the corresponding
implementation phase begins. They are ordered by the layer they block.

---

### Ambiguity 1 — BOM Handling For Multi-Format Sources

**Blocks:** Layer 1 implementation.

**Question:** When the same `ByteSource` + `EncodingDecoder` module handles both HTML and
XML inputs, how is the BOM treated?

- WHATWG HTML: detect encoding from BOM, then strip it before decoding.
- XML 1.0: BOM is the byte order mark for UTF-16; UTF-8 BOM is permitted, signals UTF-8.

**Impact:** `source::decode` needs either a `DecoderMode { Html, Xml }` parameter, or two
separate decoder entry points.

**Options:**  
A. Two modes on the decoder: `decode_html(bytes)` strips BOM per WHATWG; `decode_xml(bytes)` preserves BOM semantics.  
B. Unified decoder with a format hint.

**Recommendation:** Option A — keeps each format's rules unambiguous, avoids a shared code path that must know about both WHATWG and XML BOM rules.

---

### Ambiguity 2 — HTML5 Tokenizer: Existing Crate vs. Custom

**Blocks:** Layer 2 HTML profile implementation.

**Question:** Should `cem_ml::tokenizer::html` wrap an existing Rust HTML5 tokenizer
crate (e.g., `html5ever`, `lol_html`, `quick-xml`) or implement a custom WHATWG-compliant
tokenizer?

**For existing crate:** Battle-tested WHATWG recovery behavior; handles real-world HTML
edge cases; faster to integrate.  
**For custom:** Full control of token shapes; guaranteed byte-offset preservation on every
token and attribute; no coupling to external crate API evolution.

**Key constraint from research:** The tokenizer must emit byte-range-annotated tokens.
The chosen crate must preserve exact byte offsets for every attribute name, attribute
value, and text node — or the source-map stack cannot be fully populated. Verify before
committing.

---

### Ambiguity 3 — CEM Schema Source Language

**Blocks:** Layer 4 schema compilation; AC-S-1 ("CEM-native syntax as source of truth").

**Question:** What language is the CEM element/attribute schema authored in?

**Options:** RELAX NG compact (`.rnc`), Invisible XML grammar (`.ixml`), CEM markdown
tables compiled to RELAX NG (extending the existing `cem-colors.md` pattern), or a
CEM-native declarative format.

**Impact:** Determines the schema compiler pipeline and the derivative runtime's input
format. All options except iXml compile to something derivative-computable; the choice
is about authoring ergonomics and schema expressiveness.

**Key constraint:** The derivative runtime needs the schema in a form that permits
`D(event, schema)` computation. Whatever the source language, it must compile to RELAX NG
compact (or an equivalent derivative-computable representation) before the schema machine
loads it.

---

### Ambiguity 4 — Tier A Transform Engine

**Blocks:** Layer 9 implementation depth.

**Question:** Does the Tier A `ImplementationInterpreter` use:  
A. Hand-written Rust match rules (minimal, predictable, easy to snapshot-test).  
B. A minimal XSLT-like template DSL (match + value-of + apply-templates + copy).  
C. A full XSLT engine (Tier C per AC-T-3).

**Impact:** Option B requires building a template parser/evaluator but moves closer to
the Tier C target and makes transforms loadable from URI or stream (AC-T-4). Option A
is simpler but means AC-T-4 is not achievable in Tier A.

**Recommendation from research:** "XSLT transform helpers" suggests a minimal subset is
acceptable for Tier A, not a full engine.

---

### Ambiguity 5 — Scope Granularity For CEM Documents

**Blocks:** Layer 4 scope-boundary design; Tier B scope isolation (AC-P-4, AC-I-3).

**Question:** Is a CEM parse scope (error boundary) one per document, or one per
top-level `data-cem-*` element (e.g., per `data-cem-screen`)?

**Per document:** Simple; one schema machine per parse run.  
**Per `data-cem-screen`:** Aligns with AC-P-5 (nested scopes) and AC-I-3 (interpreter
owns subtree exclusively). Errors inside one screen don't corrupt others.

**Recommendation:** For Tier A, use one scope per document with named subtree anchors.
Per-screen scope isolation is Tier B; design the `SchemaFrame` to carry a `scope_id` now
so the Tier B boundary is additive.

---

### Ambiguity 6 — Forward Reference Resolution Strategy

**Blocks:** Layer 6 `slots.rs` design and post-parse validation ordering.

**Question:** One-pass with mutable `NameSlot`s, or two-pass (first build id table, then
resolve references)?

**Research position:** The research explicitly describes one-pass mutable slots: "When a
target token, declaration, or entity is defined, the interpreter updates that slot." This
is the design.

**Consequence:** AC-V-6 (broken `id`/`for`/`aria-*` references) is validated in a
post-parse step that inspects unfilled `NameSlot`s, not during streaming. The schema
machine's `close` transition on the document root triggers this check.

**Open sub-question:** Should unfilled slots on document close be `Warning` or `Error`
severity? The answer depends on whether CEM allows documents with dangling references
(e.g., `aria-labelledby` pointing to a dynamically rendered id).

---

### Ambiguity 7 — Synchronous vs. Async Rust API For Tier A

**Blocks:** Layer 1 and Layer 9 API contract.

**Question:** Does the Tier A `cem_ml` library expose a synchronous Rust API (processes
an in-memory byte slice end-to-end), or a fully async API from the start?

**AC-A-1** requires all processing to be asynchronous. **AC-P-2** requires the parser to
accept a `ReadableStream<Uint8Array | string>`.

**Research position:** The `ByteSource` model is a byte buffer; streaming is a delivery
concern. A synchronous Rust parser can be wrapped by a WASM/JS binding that returns a
`Promise`, satisfying the JS API contract.

**Recommendation:** Tier A Rust library is synchronous — takes `&[u8]` or file path.
The WASM/JS binding wraps the synchronous call in a resolved `Promise`. Full async
chunked-input delivery (parsing while bytes arrive over the network) is Tier B.

---

### Ambiguity 8 — Diagnostic Error Propagation Across Layers

**Blocks:** Layer 4 and Layer 9 error model; AC-A-8.

**Question:** Do per-node errors appear as rejected promises on that node only, or do
they bubble to the document root?

**Research position:** The schema machine records diagnostics on the current `SchemaFrame`
and runs a recovery strategy. Errors do not automatically propagate up the frame stack
unless severity is `Fatal`.

**Recommendation:** Diagnostics are collected per frame and surfaced through the
structured event stream (`onParseEvent`). Promise-level rejection applies only to `Fatal`
severity or explicit scope aborts. Non-fatal errors accumulate in the diagnostic list
and appear in the report, not as thrown exceptions.

**Open sub-question:** What is the exact boundary between `Error` severity (scope
continues with permissive residual) and `Fatal` severity (scope aborts)? This needs a
severity table in the schema machine implementation.

---

### Ambiguity 9 — RELAX NG Derivative Engine vs. Hand-Written DFA

**Blocks:** Layer 4 schema validation algorithm choice for Tier A.

**Question:** Does Tier A implement a full RELAX NG derivative engine, or a
hand-written DFA specifically for the CEM vocabulary?

**Full derivative engine:** General, handles arbitrary RELAX NG grammars including open
content models, interleave, and attribute ordering. Better diagnostics (residual
describes expected content). Significant implementation work; few mature Rust crates
available (most existing implementations are Java — Jing/Trang per the research).

**Hand-written DFA:** Purpose-built for the CEM vocabulary (eight semantic roles, ten
states, two dozen attributes). Fast to implement, deterministic, easy to test. Cannot
generalize to schemas beyond CEM without rewriting.

**Recommendation:** Option C (hybrid): Tier A uses a hand-written DFA for the CEM
vocabulary. The `SchemaState` type and `derivative.rs` module interface are designed so
that the derivative engine can replace the DFA without changing the `SchemaMachine`
external API. Full RELAX NG derivatives are required when mixed-content HTML5 schemas
or external schema loading (AC-S-5) are needed.

---

## 18. Critical Review Questions And Concerns

This section records unresolved issues found by reviewing this design against
`parsing-algorithms-research.md` as the primary source. These are not decisions. They
are follow-up questions and concerns to resolve before implementation. Other workspace
documents may provide terminology, but they should not decide the answers here.

### 18.2 Source-Map And Coordinate Model Gaps

**Concern 18.2.1 — Source-map frame order is internally inconsistent.**  
Section 4.2 says frames are "earliest context first", but the examples list
`CemAstBuilder` before `SchemaValidation`, `EventNormalizer`, and `HtmlTokenizer`, which
is latest-context first.

**Question:** Is `SourceMapStack.frames[0]` the original byte source frame or the current
AST/transformed frame? This must be fixed before traversal, compression deltas, and
generated-node inheritance are implemented.

**Concern 18.2.2 — A single `ByteRange` per frame is not enough for all research cases.**  
The research explicitly mentions merged nodes, split nodes, generated nodes, entity
expansions, and source-map stacks through transformations. A single `byte_range` cannot
represent a text node produced from multiple source regions, such as `a&amp;b`, or a node
merged from adjacent text/event fragments.

**Question:** Should `SourceMapFrame` support one range, many ranges, and generated
sentinel ranges? If not, where are entity expansion, merge, and split mappings stored?

**Concern 18.2.3 — Entity and escape decoding needs source-map ownership.**  
HTML character references, XML entity references, CSS escapes, JSON string escapes, and
CSV quoted escapes all transform raw bytes into logical scalar values. The current
`DecodedChunk` model maps scalars to byte spans, but later token/event layers do not
state how escape-produced scalars preserve their original source.

**Question:** Does each language tokenizer emit per-scalar source ranges after escape
processing, or does it append a transform frame that maps decoded values back to raw
bytes?

**Concern 18.2.4 — Line/column projection is underspecified.**  
The design says line/column are derived from byte offsets, but different consumers need
different column units: Unicode scalar index, UTF-16 code units, display columns, or
language-specific positions.

**Question:** Which coordinate projections are required in Tier A reports, and how are
CRLF, isolated CR, tabs, multi-byte UTF-8, replacement characters, and HTML preprocessing
handled?

**Concern 18.2.5 — Diagnostics before AST construction still need source-map stacks.**  
The research says source maps are not just a diagnostic side table, but parse and schema
diagnostics can occur before AST nodes exist. The current `Diagnostic` shape has
`byte_offset` and optional `node`, but no explicit `SourceMapStack`.

**Question:** Should diagnostics carry a `SourceMapStack` directly, or only a
`SourceId + ByteRange` until AST nodes exist?

### 18.3 ByteSource And Decoding Questions

**Concern 18.3.1 — Sentinel-byte semantics are unsafe unless ownership is explicit.**  
The LLVM `MemoryBuffer` model is useful, but a Rust `&[u8]` cannot guarantee a sentinel
byte after `bytes.len()` unless the runtime owns an internal padded allocation.

**Question:** Does `ByteSource.bytes()` expose the original byte slice without the
sentinel, or an internal padded buffer that includes it? How do offsets exclude the
sentinel?

**Concern 18.3.2 — HTML decoding policy conflicts with "validate UTF-8 at ingress".**  
The research says HTML uses byte-stream decoding and WHATWG compatibility behavior.
The design says to validate UTF-8 at ingress for HTML inputs, but browser-style HTML can
decode non-UTF-8 inputs or replacement characters depending on encoding detection.

**Question:** Does Tier A require UTF-8-only CEM HTML, or does it implement WHATWG
encoding detection and replacement behavior? If UTF-8-only, is that a CEM profile
restriction rather than an HTML tokenizer rule?

**Concern 18.3.3 — Resource bounds are missing from the byte and decode layer.**  
The research emphasizes streaming and bounded memory, but Tier A uses in-memory buffers.

**Question:** What are the maximum input size, maximum line index size, maximum decoded
scalar count, and maximum diagnostic snippet size for Tier A?

### 18.4 Tokenizer And Event-Normalizer Gaps

**Concern 18.4.1 — Attribute-list boundaries are not represented.**  
The normalizer emits `OpenScope`, then `Name`/`Value` pairs for attributes, then children
appear later. The schema machine needs to know when the start tag's attribute set is
complete so it can validate required attributes, duplicate attributes, and element
content separately.

**Question:** Should the normalizer emit an explicit `SeparatorKind::ElementBoundary`,
`StartTagEnd`, or `OpenScopeComplete` event after all attributes?

**Concern 18.4.2 — Self-closing and void HTML elements need explicit close semantics.**  
The mapping table handles `StartTag` and `EndTag`, but not `self_closing` or HTML void
elements such as `input`, `img`, and `br`.

**Question:** Does the normalizer synthesize `CloseScope` for self-closing and void
elements? If so, what source range does the synthetic close event use?

**Concern 18.4.3 — Comments, whitespace, and trivia policy is underspecified.**  
The design says comments are discarded unless schema marks them significant. Whitespace
text nodes may also be significant or ignorable depending on context.

**Question:** Are discarded comments/whitespace represented in source maps, diagnostics,
or binary AST trivia tables? If they are fully dropped, can source-preserving transforms
or round trips ever be supported?

**Concern 18.4.4 — QName and namespace handling is asserted but not defined.**  
Normalized events use `QName`, and frames include `namespace_ctx`, but HTML lowercasing,
XML namespaces, foreign content, prefixed attributes, and case sensitivity are not
specified.

**Question:** What is the Tier A name model for HTML elements, HTML attributes,
`data-cem-*` attributes, XML names, and future SVG/MathML foreign content?

### 18.5 Schema-Machine And Validation Questions

**Concern 18.5.1 — RELAX NG derivatives and the Tier A DFA may not produce equivalent
diagnostics.**  
The design recommends a hand-written DFA for Tier A, while the research favors
derivatives for residual-based expected-content diagnostics.

**Question:** What diagnostic quality must the Tier A DFA match so that later replacing
it with a derivative engine does not break reports or feature tests?

**Concern 18.5.2 — Unknown/extension content policy is not formalized.**  
The design mentions warnings for unknown attributes and semver-compatible drift, but does
not define open-content rules in the schema state.

**Question:** Which unknown elements and attributes are warnings, which are errors, and
which are ignored? Does the policy differ for standard HTML, ARIA, `data-*`, and
`data-cem-*`?

**Concern 18.5.3 — Recovery with a "permissive residual" needs a concrete invariant.**  
The schema machine says non-fatal errors continue with a permissive residual, but a
permissive state can hide cascaded errors or corrupt AST shape.

**Question:** After an error, does recovery skip to the next close event, accept any
child until expected close, or continue with a special "error subtree" frame?

**Concern 18.5.4 — Multiplicity, ordering, and required-name tracking need phase
boundaries.**  
The frame stores `seen_names`, but content validation also needs seen children, required
child roles, multiplicity, ordering, and possibly unordered/interleaved groups.

**Question:** What exact state is tracked on `SchemaFrame` for attributes versus child
content, and when is each constraint checked?

**Concern 18.5.5 — Schema language options mix authoring syntax with runtime semantics.**  
Section 13 states that all options except iXml feed the same RELAX NG derivative runtime.
That assumes the chosen source language can represent every required CEM constraint in
a derivative-computable form.

**Question:** Which CEM constraints are structural grammar constraints, and which are
semantic validation passes outside RELAX NG derivatives?

### 18.6 Embedded Handoff Concerns

**Concern 18.6.1 — ModeSwitch ownership is ambiguous between tokenizer, normalizer, and
schema machine.**  
The normalizer mapping emits `ModeSwitch` for `<style>` and `<script>`, while Layer 5 says
the parent schema machine emits `HandoffRecord`s. For WHATWG HTML, raw text content is
also controlled by tokenizer state.

**Question:** Which layer creates the authoritative `HandoffRecord`, and which layer
only reports that the lexical mode changed?

**Concern 18.6.2 — Embedded content byte ranges can be decoded views, not contiguous raw
bytes.**  
A `style=""` attribute child CSS source is the decoded attribute value. HTML entity
references inside that attribute mean the child parser sees a logical string whose
characters do not map one-to-one to raw bytes.

**Question:** Does the child parser consume a synthetic `SourceId` with its own decoded
text and a source-map frame back to the parent attribute, or does it consume raw parent
bytes with escape awareness?

**Concern 18.6.3 — Script handling says "raw text" but security and diagnostics still
need a policy.**  
Tier A treats scripts as raw text, but CEM semantic documents may need to reject,
warning-report, or preserve scripts.

**Question:** Are `<script>` regions always warnings, always errors, preserved raw, or
allowed only with specific `type` values?

**Concern 18.6.4 — Primary research includes JSON strings, XML CDATA, CSV/CSF fields,
TypeScript templates, JSX, and CSS functions as handoff examples.**  
The design lists Tier A HTML cases but does not clearly mark the other research cases as
deferred or unsupported.

**Question:** Should the handoff module define enum variants for all research return
conditions now, even if only HTML style/script cases are active in Tier A?

### 18.7 AST And Reference Model Concerns

**Concern 18.7.1 — `Arc<Mutex<Option<AstNodeId>>>` may overfit future concurrency.**  
The research requires mutable scoped name slots, but not necessarily `Arc<Mutex>`.
For a synchronous Tier A parser, a generational slot arena may be simpler, faster, and
easier to serialize into the binary AST.

**Question:** Is `NameSlot` a logical concept whose implementation can be an arena slot,
or is thread-safe shared ownership part of the public contract?

**Concern 18.7.2 — Slot lifecycle and override rules are not defined.**  
The research says slots are set or overridden when a referenced token/entity is defined.
The design only covers filling an empty id slot.

**Question:** What happens on duplicate IDs, shadowed scoped names, late overrides,
deleted/replaced nodes, or cross-scope references?

**Concern 18.7.3 — Multiple CEM roles on one element are not specified.**  
The AST enum implies a single CEM node kind per source element. HTML could contain more
than one `data-cem-*` attribute on the same element.

**Question:** Are multiple semantic roles invalid, does one role win by precedence, or
does the AST represent a composed role set?

**Concern 18.7.4 — CEM vocabulary in the design is asserted without source-of-truth
ownership.**  
The role and state lists in Section 8 may be correct, but the design does not say where
the authoritative vocabulary lives or how fixtures/schema update it.

**Question:** Is the vocabulary generated from schema source, hard-coded in
`schema::vocab`, derived from fixtures, or maintained manually?

**Concern 18.7.5 — Text and pass-through HTML nodes need normalization rules.**  
The AST includes `HtmlElement` and `Text`, but not comments, doctypes, processing
instructions, raw text, or recovered error nodes.

**Question:** Which non-CEM constructs are preserved in the AST, which are discarded,
and which become diagnostics-only?

### 18.8 Transform And Output Concerns

**Concern 18.8.1 — Attribute collision policy is missing.**  
Transform rules map `data-cem-*` into custom-element attributes while also passing
through standard HTML attributes.

**Question:** What happens if the source already has `cem-id`, `variant`, `state`, or
other output-reserved attributes?

**Concern 18.8.2 — Pass-through `data-*` may accidentally preserve deprecated semantic
inputs.**  
The design says ARIA and `data-*` pass through. If `data-cem-*` also passes through after
being transformed, output markup duplicates semantic source and generated semantics.

**Question:** Are source `data-cem-*` attributes removed, renamed, preserved for
debugging, or gated behind a debug/source-map mode?

**Concern 18.8.3 — Output serialization rules are not defined.**  
The transform returns `markup: String`, but deterministic snapshots require stable
attribute ordering, escaping, whitespace, void-element handling, and text serialization.

**Question:** What canonical serialization rules does `ImplementationInterpreter` use?

**Concern 18.8.4 — XSLT-equivalent behavior is not acceptance-testable yet.**  
The design says hand-written Rust rules are Tier A and XSLT is Tier C, but the research
and transform goal imply loadable transform behavior over time.

**Question:** What minimum abstraction keeps Tier A hand-written transforms from
blocking a future template/XSLT-style transform engine?

### 18.9 Binary AST And Compression Concerns

**Concern 18.9.1 — Deferred binary interfaces may prematurely freeze bad IDs.**  
The design says Tier A defines trait signatures for binary AST stubs, but the binary
format is a complex storage and transport design in the research.

**Question:** Which binary concepts must be stable in Tier A: node IDs, dictionary IDs,
scope slot IDs, chunk IDs, source-map frame IDs, or none of them?

**Concern 18.9.2 — Canonical binary representation requires determinism rules.**  
The research calls binary AST a cache/transport format. Caches and hashes require stable
ordering and canonical encoding.

**Question:** What are the ordering rules for nodes, attributes, string tables,
dictionaries, source-map deltas, and diagnostics?

**Concern 18.9.3 — Cross-chunk reference slots and AST reference slots may be the same
concept or different concepts.**  
The research uses scoped slots both for unresolved language references and for late
chunk arrival.

**Question:** Does one slot system serve AST name references and chunk dependency
placeholders, or are they separate namespaces with separate lifecycle rules?

### 18.10 Security, Recovery, And Resource Concerns

**Concern 18.10.1 — XML entity, DTD, and external-resource policy is absent.**  
The research allows XML-style encodings, but an XML profile also raises entity expansion,
external DTD, and resource-fetch questions.

**Question:** Are DTDs, external entities, XIncludes, and network fetches rejected,
ignored, or exposed as diagnostics in Tier A?

---

*End of design document. Each ambiguity and review concern above should be resolved with
a brief decision record before the corresponding implementation phase starts. Resolved
items should be struck through and replaced with the chosen option and rationale.*
