# `cem-ml` Stack Implementation Design

**Status:** Draft implementation companion.  
**Primary design:** [`cem-ml-stack-design.md`](cem-ml-stack-design.md)  
**Primary source:** [`parsing-algorithms-research.md`](../parsing-algorithms-research.md)  
**Date:** 2026-05-08

---

## 1. Purpose And Boundary

This document contains the implementation-level contracts that support the high-level
functional design in [`cem-ml-stack-design.md`](cem-ml-stack-design.md). The primary
design owns behavior, layer responsibilities, tier scope, and unresolved decisions. This
companion owns concrete data shapes, interface sketches, projection keys, and Rust module
topology.

Implementation contracts here must not introduce behavior that is absent from the
primary design. If this document and the primary design conflict, the primary design wins
until both documents are updated by a design decision.

---

## 2. Shared Source-Map And Diagnostic Contracts

Source maps are an AST contract, not a diagnostic side table. Every AST node must carry a
traversable stack linking it back to its origin in the byte stream.

### 2.1 Coordinate System

Byte offsets are the stable ground truth. Line and column are derived coordinates
projected on demand; they are never stored permanently on AST nodes.

```
ByteRange { start: u64, len: u32 }
    - absolute byte offset from the start of the SourceId's byte buffer.
    - len: u32 caps a single token at 4 GiB, which is sufficient.
```

Line/column projection is performed by a `LineIndex` that records the byte offset of
each newline in the source file. Projection is `O(log n)` via binary search. The
`LineIndex` is computed once per `SourceId` and cached.

For Tier B+ compressed binary content, the research specifies bit-level ranges
(`bit_start: u64, bit_len: u32`). Bit fields are reserved in the source-map schema but
not populated in Tier A.

### 2.2 Source-Map Stack

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

### 2.3 Traversal Examples

An `aria-labelledby` reference in a parsed fixture traces as:

```
CemAnnotation { kind: Screen, value: "login" }
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

External or referenced XML resources use the same source-map relationship as
content-type handoff scopes. The referenced source receives its own `SourceId`, and
nodes produced from it carry frames back through the external-resource boundary to the
originating reference.

### 2.4 Generated Nodes

Transform-generated nodes, such as custom-element output with no direct input text,
store:

- `transform: TransformKind::Implementation`
- `byte_range`: the nearest owning source range from the CEM AST node that produced them.
- Prior frames: inherited from the CEM AST source chain.

### 2.5 Diagnostics

```
Diagnostic:
  uri: String                       document URI or file path
  line: u32                         1-based, derived from byte_offset
  column: u32                       1-based, derived from byte_offset
  byte_offset: u64                  ground-truth position
  code: DiagCode                    stable enumerated code
  severity: Severity { Fatal | Error | Warning | Info }
  message: String
  node: Option<AstNodeId>           AST node reference when available
```

`Fatal` aborts the current scope. `Error` and `Warning` continue in diagnostic mode with
a permissive residual, subject to the unresolved recovery invariant in the primary
design.

---

## 3. Layer Interface Contracts

### 3.1 Layer 1: ByteSource And EncodingDecoder (`cem_ml::source`)

Modeled on LLVM `MemoryBuffer`: a read-only byte slice with a guaranteed sentinel byte
after the end for fast lexing without bounds checks per character.

```
SourceId: opaque stable identity for a byte buffer (used in source-map frames)

ByteSource:
  id() -> SourceId
  bytes() -> &[u8]                  read-only; sentinel byte guaranteed at bytes.len()
  byte_range() -> ByteRange         full range of this source

DecodedChunk:
  scalars: [(char, ByteRange)]      Unicode scalar paired with its byte span
  byte_range: ByteRange             range covered by this chunk
  encoding: Encoding                UTF-8, UTF-16LE, UTF-16BE, Latin-1, ...
```

Implementation rules:

- Keep absolute `u64` byte offsets for every token and event.
- Keep decoded scalar spans alongside scalars for Unicode-aware validation.
- Preserve raw byte slices for zero-copy diagnostic snippets.
- Validate UTF-8 at ingress for HTML inputs unless the selected profile resolves the
  HTML/WHATWG encoding ambiguity differently.

Tier A supports in-memory byte buffer, string input, and file-path input. Chunked async
network delivery is Tier B.

### 3.2 Layer 2: SchemaTokenizer (`cem_ml::tokenizer`)

The tokenizer is mode-aware and schema-guided. It extracts source-spanned tokens and
switches lexical states; it does not construct either the initial HTML parser DOM or the
WHATWG implementation DOM.

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
etc.) are internal to this layer. The schema can select valid tokenizer contexts and
embedded-content boundaries, but it does not rewrite WHATWG lexical behavior.

XML tokenizer follows the same `RawToken` shape using an XML 1.0 profile, keeping Layers
3 and above format-agnostic. XML external resources and compatibility behavior are owned
by the XML content-type transform, not the tokenizer.

### 3.3 Layer 3: EventNormalizer (`cem_ml::events`)

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

HTML token mapping:

| HTML token                               | Emitted events                                                                          |
|------------------------------------------|-----------------------------------------------------------------------------------------|
| `StartTag { name, attrs }`               | `OpenScope { name }`, then for each attr: `Name { attr_name }` + `Value { attr_value }` |
| `EndTag { name }`                        | `CloseScope { name }`                                                                   |
| `Text { data }`                          | `Value { Text(data) }`                                                                  |
| `StartTag { name: "style" \| "script" }` | `OpenScope`, then `ModeSwitch { content_type }`                                         |
| `Comment`                                | Discarded, or `Value` if schema marks comments as significant                           |
| `ParseError`                             | `Error { ... }`                                                                         |

Each `StartTag` emits its `OpenScope` first, then one `Name`+`Value` pair per attribute,
preserving attribute source positions.

### 3.4 Layer 4: SchemaMachine (`cem_ml::schema`)

The CEM semantic vocabulary is defined functionally in the primary design's Layer 4
section. Implementation code consumes the active compiled schema; `schema::vocab` is an
implementation convenience generated from, or kept traceable to, that schema source.

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

### 3.4.1 Namespace Context Contracts

```
NsContext:
  scope_id: ScopeId
  parent: Option<NsContextId>
  bindings: Vec<NamespaceBinding>   ordered by declaration sequence

NamespaceBinding:
  binding_id: NamespaceBindingId
  name: NamespaceName               prefix, or "" for the default namespace
  namespace_uri: NamespaceUri
  schema_id: Option<SchemaId>
  declared_at: ByteRange
  effective_from: ByteRange         first source position where this binding applies
  source_map: SourceMapStack

NamespaceName:
  String                            "" is the default namespace name

ExpandedName:
  namespace_uri: NamespaceUri
  schema_id: Option<SchemaId>
  local_name: String

NameResolution:
  lexical_name: String
  expanded_name: ExpandedName
  binding_id: Option<NamespaceBindingId>
  source_range: ByteRange
```

Resolution uses the namespace context visible at the source position being parsed.
Within a single scope, the latest binding with the same `NamespaceName` wins from its
`effective_from` position forward. Nested scopes inherit parent bindings, but an inner
binding with the same name shadows the inherited binding until the inner scope closes.

Previously emitted `NameResolution` records are immutable. If `screen` resolves to
`{NS1}screen`, and a later default namespace declaration changes the default namespace to
`NS2`, later `screen` names resolve to `{NS2}screen` but the earlier record still points
to the `NS1` binding id. Reports and source maps can therefore show that the same
lexical name changed schema ownership over time.

Attribute and tag collision checks use `ExpandedName`, not lexical spelling. Rendered
projections may lower schema-qualified names to unqualified convenience attributes or
tags, but the renderer must retain enough mapping metadata to distinguish generated
CEM-owned names from pass-through HTML names.

State transitions:

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
  Emit HandoffRecord.
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

### 3.5 Report Event Model (`cem_ml::report`)

Reports are owned by `cem_ml::report`, but their canonical internal data is an
AST-associated report tree rather than a flat diagnostic list. Each parser, schema,
handoff, transform, validation, or runtime log message is captured as a report event node
attached to:

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

### 3.6 CLI Projection Keys

Stack layers own data artifacts. The CLI owns projection selection, output targets, and
default stream behavior. Proposed projection layer keys:

| Key                | Stack owner                       | Projection meaning                                                                                       |
|--------------------|-----------------------------------|----------------------------------------------------------------------------------------------------------|
| `source`           | `source::ByteSource`              | Source metadata, URI, byte length, and source id; raw bytes are not emitted unless explicitly requested. |
| `decoded`          | `source::decode`                  | Encoding result, decoded scalar spans, replacement/encoding diagnostics, and line-index metadata.        |
| `tokens`           | `tokenizer`                       | Format-native token stream with byte ranges.                                                             |
| `events`           | `events`                          | Normalized open/close/name/value/separator/mode-switch/error events.                                     |
| `schema-frames`    | `schema`                          | Schema frame transitions, residual/DFA state, expected closes, and validation state.                     |
| `namespace-bindings` | `schema`                        | Ordered namespace declarations, effective ranges, overrides, and lexical-to-expanded name resolutions.   |
| `handoffs`         | `handoff`                         | Embedded content boundaries, inherited context, child content type, and return condition.                |
| `input-dom`        | `parser::input_dom`               | Schema-defined initial DOM/AST hierarchy reconstructed from tokens/events.                               |
| `whatwg-dom`       | `transform::whatwg_html`          | WHATWG implementation-DOM update projection from the initial HTML parser DOM.                            |
| `cem-ast`          | `parser`                          | CEM semantic AST projection over the input DOM/AST.                                                      |
| `transform-output` | `interpreter` / transform modules | Canonical CEM-ML transform output and optional rendered projection for the selected output content type. |
| `ui-dom-plan`      | `interpreter` / `runtime`         | Virtual UI DOM plan: template references, data bindings, visual scope ownership, and patch identity.     |
| `machine-state`    | `runtime`                         | Runtime data slots from attributes, dataset, payload/slots, slices, browser adapters, or caller state.   |
| `hydration-plan`   | `runtime`                         | Event-to-state and state-to-render invalidation rules for hydrated output.                               |
| `template-registry` | `interpreter` / `runtime`        | Local, external, schema-owned, registry-owned, and DCE tag-name template references.                     |
| `source-map`       | `source_map`                      | Source-map stacks and event-time source-map hierarchy.                                                   |
| `report-ast`       | `report`                          | Canonical AST-associated report tree with event sequence and source module state.                        |
| `trace`            | `engine` / `command`              | Deterministic execution trace assembled from parser, validator, transform, and report events.            |
| `binary-ast`       | `ast::encode`                     | Deferred binary AST representation.                                                                      |
| `chunks`           | `ast::chunk` / `ast::compress`    | Deferred subtree chunk and compression metadata.                                                         |

Fixture round-trip output is a CLI composition of projections, not a separate stack
layer. It records selected inputs, chosen projection layer(s), rendered outputs or
hashes, report AST summaries, and diagnostics.

### 3.7 Layer 5: Scoped Embedded Handoff Stack (`cem_ml::handoff`)

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

The parent schema machine emits a `HandoffRecord` with the exact `ReturnCondition`
before yielding the byte stream to the child parser. The child parser consumes bytes up
to the return condition and signals completion. The parent resumes with the byte
following the condition boundary.

### 3.8 Layer 6: InputDomAstBuilder / InterpreterAstBuilder (`cem_ml::parser`)

```
InputDomNode:
  Document(InputDocument)
  Element(InputElement)             native XML/(X)HTML identity plus optional CEM annotations
  Attribute(InputAttribute)
  Text(TextNode)
  Comment(CommentNode)
  Doctype(DoctypeNode)
  ProcessingInstruction(ProcessingInstructionNode)
  Cdata(CdataNode)
  RawText(RawTextNode)
  ErrorNode(RecoveredErrorNode)
  Extension(ExtensionNode)

InputElement:
  node_id: AstNodeId
  expanded_name: ExpandedName       native tag identity, such as HTML button or SVG path
  attributes: AttributeMap
  annotations: Vec<CemAnnotationId>
  children: Vec<AstNodeId>
  source: SourceMapStack

CemAnnotation:
  annotation_id: CemAnnotationId
  source_node: AstNodeId
  name: ExpandedName                schema-qualified CEM annotation name
  kind: CemAnnotationKind
  value: Option<ScalarValue>
  source: SourceMapStack
  state: Option<CemState>

CemAnnotationKind:
  Screen
  Form
  Action
  List
  Card
  Thread
  Message
  Badge
  State
  Extension

CemNode:
  Document(CemDocument)
  AnnotatedInput(CemAnnotatedInput)
  InputNode(AstNodeId)         pass-through generic XML/(X)HTML input DOM node
  Text(TextNode)

CemAnnotatedInput:
  node_id: AstNodeId
  source_node: AstNodeId
  annotations: Vec<CemAnnotationId>
  children: Vec<AstNodeId>
  source: SourceMapStack

CemDocument:
  source_id: SourceId
  root_children: Vec<AstNodeId>
  id_table: HashMap<String, AstNodeId>   global id map for reference resolution
  diagnostics: Vec<Diagnostic>
```

`InputDomNode` is the generic schema-defined AST surface. Its minimal Tier A preserved
construct set is TBD. The CEM projection uses `CemNode` and can carry non-CEM source
constructs by reference through `InputNode(AstNodeId)`.

`CemAnnotation` records schema-qualified transform triggers such as `cem:screen` or
`cem:action`. Multiple annotations can attach to the same `InputElement`; the source
element keeps its `ExpandedName` and native DOM meaning. If a transform rewrites or
replaces the source element, the schema-owned transform plan resolves annotation
composition, precedence, rejection, or diagnostics.

Reference slots:

```
NameSlot: Arc<Mutex<Option<AstNodeId>>>

LabelRef(NameSlot)          - wraps a slot; filled when id="..." element is parsed
ForRef(NameSlot)            - for/id pairing
AriaRef(NameSlot)           - aria-labelledby, aria-describedby, etc.
```

When the parser encounters an element with an `id` attribute, it looks up the slot in
the document's `id_table` and fills it. Any prior `LabelRef`/`ForRef`/`AriaRef` holding
the same slot observes the fill immediately.

Forward references are represented as unfilled slots at parse time. The schema machine
performs a post-parse reference check to identify remaining unfilled slots and emit
diagnostics.

The `InputDomAstBuilder` appends a source-map frame when reconstructing the
schema-defined token hierarchy. The `InterpreterAstBuilder` appends a
`TransformKind::CemAstBuilder` frame when creating each CEM projection node. The prior
frames come from the `SchemaFrame.source_map_stack` at the point the element was
validated.

### 3.9 Layers 7-8: BinaryAstEncoder And ChunkCompressor (`cem_ml::ast`)

```
BinaryAstEncoder responsibilities:
  - Assign stable binary node ids after the binary format is active.
  - Reference platform and app dictionaries by id.
  - Emit source-map deltas (not full frames) to compress the map chain.
  - Assign subtree chunk ownership.

ChunkCompressor responsibilities:
  - Platform dictionary: common AST node kinds, primitive encodings, schema defs, shared strings.
  - App dictionary: CEM-specific node kinds, local symbol tables, repeated literals.
  - Payload chunks: subtree AST nodes, scope slots, source-map deltas, embedded ranges.
```

Chunk boundaries align to subtree roots. Each chunk is independently decodable and
carries integrity hashes, dependency ids, and dictionary version requirements.

Cross-chunk dependency slots and AST reference slots are separate namespaces with
separate lifecycle rules. An AST reference slot resolves a language or document reference
such as `id`/`for`/`aria-*`; a chunk dependency slot resolves transport availability for
binary subtree data.

```
ChunkRelation:
  subtree_id: BinarySubtreeId
  relation: ChunkRelationKind       primary, continuation, boundary, side_table, ...
  sequence: u32                     ordering within relation for this subtree
```

Compression profiles:

| Profile           | Algorithm                        | Use case                                                                                     |
|-------------------|----------------------------------|----------------------------------------------------------------------------------------------|
| `none`            | Uncompressed binary              | Debugging, tests, memory-mapped storage, environments where compression cost exceeds savings |
| `canonical-fast`  | Zstandard with shared dictionary | Interactive delivery, most networked runtimes                                                |
| `canonical-dense` | Brotli or high-level Zstandard   | Cold storage, batch transfer                                                                 |
| `solid-archive`   | Whole-document compression       | Cold storage only; no parallel decode or retry                                               |

Tier A must not freeze serialized binary identifiers. Node ids, dictionary ids, scope
slot ids, chunk ids, and source-map frame ids are not part of the Tier A external
contract because the binary transport/cache format is deferred. Tier A may use opaque
internal handles while building in-memory trees, but those handles must not be
serialized, hashed, exposed as stable API values, or treated as future binary ids.

### 3.10 Layer 9: ImplementationInterpreter (`cem_ml::interpreter`)

```
CemInterpreter trait:
  transform(doc: &CemDocument, plan: &TransformPlan, ctx: &TransformContext) -> Result<TransformOutput, CemError>

TransformPlan:
  schema_id: SchemaId
  target_content_type: ContentType
  rules: Vec<TransformRule>        schema-owned rules; backend-neutral

TransformContext:
  schema_id: SchemaId
  base_uri: Option<String>
  fail_level: FailLevel

TransformOutput:
  canonical: CemMlDocument            schema-owned CEM-ML AST/tree serialization
  ui_dom_plan: Option<UiDomPlan>      virtual UI DOM plan before rendered materialization
  rendered: Option<RenderedOutput>    optional target rendering, such as light-DOM HTML
  diagnostics: Vec<Diagnostic>
  source_maps: Vec<SourceMapStack>    one per output element

RenderedOutput:
  content_type: ContentType
  bytes: Vec<u8>
  encoding: Encoding
```

Each output custom-element node appends a `TransformKind::Implementation` frame. The
prior frames, inherited from the CEM AST node's `SourceMapStack`, trace back to the
original HTML token.

The reference implementation stack must execute schema-driven transform plans. A
hand-written Rust backend is allowed as developer convenience, for prototyping, and for
optimized execution of schema rules, but it must not become the essential source of
transform behavior. Any Rust implementation must be traceable back to schema-owned rules
and must preserve the same diagnostics and source-map semantics as another backend.

### 3.11 Visual Content, Machine State, And Hydration Contracts

The AST core remains uniform across content types. The implementation records content
type and functional role on scopes; it does not create separate AST families for visual,
code, or data content.

```
ContentScope:
  scope_id: ScopeId
  content_type: ContentType
  role: ContentScopeRole
  owner: Option<ScopeId>
  source: SourceMapStack
  policy: ContentTypePolicy

ContentScopeRole:
  Visual
  MachineState
  Code
  ResourceSlot
  Mixed
```

Visual scopes can produce a virtual UI DOM plan. Machine state scopes provide the data
used to parameterize that plan. Code scopes provide transform or styling behavior when
allowed by policy. Resource slots preserve unresolved external or embedded inputs for a
later transform or caller.

```
TemplateRef:
  SchemaTemplate { schema_id: SchemaId, name: String }
  LocalId { source_id: SourceId, id: String }
  Url { url: String }
  UrlFragment { url: String, fragment: String }
  RegistryEntry { registry_id: RegistryId, name: String }
  DceTagName { tag_name: String }

MachineStateSlot:
  slot_id: StateSlotId
  owner_scope: ScopeId
  key: StateKey
  source: MachineStateSource
  value: ScalarValue | AstNodeId | ResourceRef | Null
  source_map: SourceMapStack

MachineStateSource:
  Attributes
  Dataset
  PayloadSlots
  Slice
  Fetch
  Storage
  Location
  FormState
  CallerProvided
```

DCE integration is represented as ordinary template and state metadata. A DCE tag name
is a `TemplateRef`; DCE attributes, dataset, payload/slots, and slices become
`MachineStateSlot`s. Browser-facing custom-element primitives such as HTTP request,
storage, and location/route providers are runtime state sources, not AST node kinds.

```
RenderBinding:
  visual_scope: ScopeId
  template_ref: TemplateRef
  input_dom_root: Option<AstNodeId>
  state_slots: Vec<StateSlotId>
  transform_rules: Vec<TransformRule>

UiDomPlan:
  plan_id: UiDomPlanId
  root: UiDomNodeId
  binding: RenderBinding
  nodes: Vec<UiDomNode>
  hydration_rules: Vec<HydrationRule>
  patch_policy: DomPatchPolicy
  source_maps: Vec<SourceMapStack>
```

The UI DOM plan is virtual because it records how a template reference is reused with a
state binding. Materialization to browser DOM, light-DOM custom-element markup, static
HTML, or another rendered target is a projection of this plan.

```
HydrationRule:
  rule_id: HydrationRuleId
  event_source: HydrationEventSource
  event_name: String
  state_target: StateSlotId
  value_expr: Option<TransformExpr>
  invalidates: Vec<InvalidationTarget>
  policy: HydrationPolicy

InvalidationTarget:
  Scope(ScopeId)
  UiDomNode(UiDomNodeId)

HydrationEventSource:
  DomEvent { node: UiDomNodeId }
  BrowserAdapter { adapter: BrowserAdapterKind }
  CallerEvent { name: String }

BrowserAdapterKind:
  HttpRequest
  Storage
  Location
  Route
  Form

DomPatchPolicy:
  preserve_node_identity: bool
  preserve_focus_selection: bool
  preserve_runtime_resources: bool
  patch_granularity: Scope | Node | Attribute | Text
```

Hydration applies event-to-state updates, re-evaluates affected render bindings, and
patches the materialized DOM according to the patch policy. Tier A may emit the metadata
above for static or future hydrated output. The live hydration runtime, browser adapter
execution, DOM patcher, and DOM identity preservation model are subject to a separate
design phase (TBD).

---

## 4. Rust Module Map

```
cem_ml/src/
  lib.rs
  source/
    mod.rs            ByteSource trait, SourceId, ByteRange
    decode.rs         EncodingDecoder, DecodedChunk, Encoding
    line_index.rs     LineIndex - byte offset -> (line, col) projection
  tokenizer/
    mod.rs            RawToken, SchemaTokenizer trait
    html.rs           WHATWG HTML tokenizer profile
    xml.rs            XML 1.0 tokenizer profile
  events/
    mod.rs            NormalizedEvent, EventNormalizer
  schema/
    mod.rs            SchemaMachine, SchemaFrame, SchemaState
    derivative.rs     RELAX NG derivative computation
    namespace.rs      NsContext, NamespaceBinding, ExpandedName, NameResolution
    vocab.rs          CEM vocabulary constants generated from schema source
  handoff/
    mod.rs            HandoffRecord, HandoffStack, ReturnCondition, InheritedContext
  parser/
    mod.rs            InputDomNode, CemNode enum, CemDocument
    input_dom.rs      schema-defined initial DOM/AST reconstruction
    nodes.rs          InputElement, CemAnnotatedInput, CemAnnotation, pass-through input nodes
    slots.rs          NameSlot, LabelRef, ForRef, AriaRef - reference resolution
  source_map/
    mod.rs            SourceMapStack, SourceMapFrame, TransformKind
  transform/
    mod.rs            content-type transformation pipeline
    whatwg_html.rs    initial HTML parser DOM -> WHATWG implementation DOM update transform
    css.rs            CSS/SCSS AST and external-reference transform hooks
  interpreter/
    mod.rs            CemInterpreter trait, TransformContext, TransformOutput, RenderedOutput
    transform.rs      CEM semantic HTML -> custom-element transform rules
  runtime/
    mod.rs            machine state, template registry, hydration plans, DOM patch policy
    state.rs          MachineStateSlot and state source adapters
    hydration.rs      HydrationRule, event mapping, invalidation rules
    ui_dom.rs         UiDomPlan, UiDomNode, RenderBinding, patch identity
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
    encode.rs         BinaryAstEncoder - node ids, dictionary refs, source-map deltas
    compress.rs       ChunkCompressor - platform/app dictionary, payload chunks
    chunk.rs          chunk metadata, integrity hash, dependency list
```

`ast/` sub-modules are reserved stubs in Tier A. Their interfaces are defined but their
bodies are no-ops.

`cem_ml_cli/src/main.rs` owns only Clap argument parsing, cwd/workspace detection,
stdout/stderr writing, and process exit. All logic lives in `cem_ml`.

---

## 5. Incremental And Editor Mode Contract (Deferred Tier B)

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
partially reused tree.

Validation after an incremental edit recomputes affected ancestors, changed scopes, and
references that cross between changed and reused scopes. Report events for incremental
passes attach to the event-time partial tree, just like batch reports.

---

## 6. Implementation Ownership Rules

- The primary design defines functional behavior, tier scope, and unresolved decisions.
- This file defines implementation shapes for the behavior already present in the
  primary design.
- Acceptance criteria must be derived from resolved design decisions, not from
  speculative implementation details.
- Deferred Tier B/C interfaces may be stubbed in Tier A only when the primary design
  calls for stable seams.

---

*End of implementation design document.*
