# `cem-ml` Stack Implementation Design

**Status:** Draft implementation companion derived from the primary acceptance criteria
and high-level design.
**Primary acceptance criteria:** [`cem-ml-ac.md`](cem-ml-ac.md)
**High-level design:** [`cem-ml-stack-design.md`](cem-ml-stack-design.md)
**Architectural research source:** [`parsing-algorithms-research.md`](../parsing-algorithms-research.md)
**Date:** 2026-05-08

---

## 1. Purpose And Boundary

This document contains the implementation-level contracts that support the primary
acceptance criteria in [`cem-ml-ac.md`](cem-ml-ac.md) and the high-level functional
design in [`cem-ml-stack-design.md`](cem-ml-stack-design.md). The AC owns required
behavior and tier scope. The high-level design explains layer responsibilities and
functional boundaries. This companion owns concrete data shapes, interface sketches,
projection keys, and Rust module topology.

Implementation contracts here must not introduce behavior that is absent from the AC or
high-level design. If this document conflicts with the AC, the AC wins until this
document is corrected or an unresolved ambiguity is recorded.

---

## 2. Shared Source-Map And Diagnostic Contracts

Source maps are an AST contract, not a diagnostic side table. Every AST node must carry a
traversable stack linking it back to its origin in the byte stream.

### 2.1 Coordinate System

Byte offsets in source-map frames are the stable ground truth. Line and column are
derived reporting coordinates projected on demand for a selected frame; they are never
stored permanently on AST nodes and are not parser semantics.

```
ByteRange { start: u64, len: u32 }
    - absolute byte offset from the start of the SourceId's byte stream.
    - len: u32 caps a single token at 4 GiB, which is sufficient.
```

Line/column projection is performed by a streaming `LineIndex` that records newline byte
offsets as chunks pass through each source stream. Projection is `O(log n)` via binary
search over the accumulated checkpoints for the selected frame's `SourceId`. A report
renderer chooses the frame to project: origin/input for author-facing diagnostics,
current/generated for transform debugging, or an intermediate frame for pipeline traces.
Different frames can legitimately produce different line/column values for the same
diagnostic.

For Tier B+ compressed binary content, the research specifies bit-level ranges
(`bit_start: u64, bit_len: u32`). Bit fields are reserved in the source-map schema but
not populated in Tier A.

### 2.2 Source-Map Stack

```
SourceMapStack:
  frames: [SourceMapFrame, ...]     ordered origin-first; current frame is last

SourceMapFrame:
  source_id: SourceId               which byte stream produced this context
  span: FrameSpan                   position(s) within that stream
  transform: TransformKind          what step created or modified this node

FrameSpan:
  Single(ByteRange)                  common case: one source span
  Multi(Vec<ByteRange>)              merged or split source spans
```

```
TransformKind:
  HtmlTokenizer
  XmlTokenizer
  EscapeDecoded { decoded_to_source: Vec<(ScalarRange, ByteRange)> }
  EventNormalizer
  SchemaValidation(schema_id: SchemaId)
  CemAstBuilder
  HandoffBoundary { child_content_type: ContentType }
  TemplateEmbedding { host_span: ByteRange, query_span: ByteRange }
  ReferenceInlined { ref_site: ByteRange }
  ExternalResource { ref_site: ByteRange }
  Implementation                    transform/render step
  BinaryEncoder                     [Tier B]

ScalarRange:
  start: u32                         decoded scalar offset within the frame value
  len: u32                           decoded scalar length
```

`FrameSpan::Single` is the default. `FrameSpan::Multi` is used when a node or event maps
to multiple source spans, such as merged text fragments or split source fragments. The
order of ranges in `Multi` has no semantic meaning; projection consumers must use each
range's own `SourceId`/`ByteRange` to locate the proper place in the source stream. If a
renderer wants a summary snippet, it may sort or group ranges for that projection.
Generated nodes use `Single(nearest_owner_range)` with the transform frame that generated
them. Reference inlining and external-resource resolution are modeled as boundary frames;
the referenced target carries its own source-map stack across the boundary rather than
embedding a nested stack inside `FrameSpan`.

Stacks are built by appending frames as processing advances. `frames[0]` is the origin
frame and `frames.last()` is the current frame. Code should use conceptual
`origin_frame()` and `current_frame()` accessors instead of positional indexing; whether
those methods live directly on `SourceMapStack` or on a `SourceMapView` is an
implementation API decision driven by call-site ergonomics.

### 2.3 Traversal Examples

An `aria-labelledby` reference in a parsed fixture traces as:

```
CemAnnotation { kind: Screen, value: "login" }
  frame[0]: HtmlTokenizer, StartTag("main"), byte=(0, 50), source=main.html
  frame[1]: EventNormalizer, OpenScope("main"), byte=(0, 50)
  frame[2]: SchemaValidation(cem-schema-v1), byte=(0, 50)
  frame[3]: CemAstBuilder, byte=(0, 50)
```

A CSS rule inside an inline `<style>` element traces through its handoff boundary back
to the parent HTML token:

```
CssDeclaration { property: "background-color" }
  frame[0]: HtmlTokenizer, StartTag("style"), byte=(85, 100), source=main.html
  frame[1]: HandoffBoundary(text/css), parent_byte=(85, 130), source=main.html
  frame[2]: CssParser, byte=(0, 24), source=embedded-style@main.html:88
```

External or referenced XML resources use the same source-map relationship as
content-type handoff scopes. The referenced source receives its own `SourceId`, and
nodes produced from it carry frames back through the external-resource boundary to the
originating reference.

### 2.4 Generated Nodes

Transform-generated nodes, such as custom-element output with no direct input text,
store:

- `transform: TransformKind::Implementation`
- `span`: `FrameSpan::Single(nearest owning source range)` from the CEM AST node that
  produced them.
- Prior frames: inherited from the CEM AST source chain.

### 2.5 Diagnostics

```
Diagnostic:
  uri: String                       document URI or file path
  byte_offset: u64                  projection from the selected report frame
  line: u32                         1-based projection for the selected report frame
  column: u32                       1-based projection for the selected report frame
  source_map: SourceMapStack        required for pre-AST and AST-time diagnostics
  code: DiagCode                    stable enumerated code
  severity: Severity { Fatal | Error | Warning | Info }
  message: String
  node: Option<AstNodeId>           AST node reference when available
  origin_scope: ScopeId
  boundary_scope: ScopeId           error-boundary scope that handled the diagnostic
```

Diagnostics originate in the scope where the parser, validator, transform, or runtime
detected the error. They bubble to the nearest error-boundary scope. `Fatal`,
`Error`, and `Warning` handling is decided by that boundary scope's effective
`ScopePolicy`. `source_map` is mandatory even before AST construction; tokenizer,
normalizer, and schema diagnostics create an event-time stack whose current frame is the
emitting layer. `byte_offset`, `line`, `column`, and any other scalar location
projection are renderer outputs derived from a selected frame in that stack, and
diagnostic phase is derived from the current frame's `TransformKind` rather than stored
as a separate field. This gives consumers one location shape for all phases. `node` is
only a convenience back-reference
populated when an AST node already exists; consumers must not rely on it for location.

---

## 3. Layer Interface Contracts

### 3.1 Layer 1: ByteSource And EncodingDecoder (`cem_ml::source`)

Modeled as an asynchronous source stream. Layer 1 assigns stable `SourceId` values and
absolute byte offsets, performs encoding selection, and yields decoded scalar chunks. It
does not expose parser-wide scan storage, lexer padding, or lexer-oriented byte windows.
Any buffering beyond transport chunk delivery belongs to the consuming layer and must be
bounded by that layer's purpose.

```
SourceId: opaque stable identity for a byte stream (used in source-map frames)

AsyncByteSource:
  next_chunk() -> Future<Option<ByteChunk>>
  source_hint() -> SourceHint

ByteChunk:
  bytes: Vec<u8> | String
  byte_range: Option<ByteRange>     known after adapter offset assignment or stream tracking

DecodeConfig:
  default_encoding: Option<Encoding> // caller/server/child-source encoding, if known
  content_type: ContentType

DecodeResult:
  chunk: DecodedChunk
  selected_encoding: Encoding
  selection: EncodingSelection
  bom: Option<BomInfo>

DecodedChunk:
  scalars: [(char, ByteRange)]      Unicode scalar paired with its byte span
  byte_range: ByteRange             range covered by this chunk
  encoding: Encoding                UTF-8, UTF-16LE, UTF-16BE, Latin-1, ...

EncodingSelection:
  Bom
  DefaultParameter
  Utf8Fallback

BomInfo:
  encoding: Encoding
  byte_range: ByteRange             source bytes skipped from the decoded scalar stream
```

Implementation rules:

- Keep absolute `u64` byte offsets for every token and event.
- Keep decoded scalar spans alongside scalars for Unicode-aware validation.
- Preserve current-chunk bytes only long enough to emit diagnostics for that chunk;
  source maps retain offsets, not source text.
- Inspect the first bytes of each source initiation before tokenization. A supported BOM
  selects the encoding for that source, is skipped from decoded scalars, remains
  addressable by source-map byte ranges, and suppresses later encoding overrides for that
  source.
- If no BOM is present, use `DecodeConfig.default_encoding`. Browser/server callers may
  derive it from transport metadata such as `Content-Type`; library callers pass it in
  parser configuration. If it is absent, use UTF-8.
- Inline embedded handoffs consume the owner's decoded Unicode stream and do not run BOM
  detection. External or separately loaded resources receive their own source stream and
  `DecodeConfig`, then apply the same initiation rule.
- In-band encoding declarations discovered after decoding do not re-decode the current
  source. If policy uses one to configure a later child source stream, it sets that
  stream's `DecodeConfig.default_encoding`; a child BOM still wins.
- Content-type preprocessing replacements, including HTML-specific replacement rules,
  run on decoded Unicode scalars in the owning tokenizer or transform. Replacement
  scalars retain source ranges pointing at the original bytes. An isolated UTF-8 BOM is
  accepted silently and omitted from decoded scalars while byte ranges still address the
  original source bytes.

Tier A exposes asynchronous source APIs only. Owned byte buffers, strings, file-path
input, and WASM `ReadableStream<Uint8Array | string>` inputs are normalized through
`AsyncByteSource` adapters. The parser decodes and tokenizes chunks monotonically while
the tokenizer owns any token-local buffering needed for lookahead or token assembly.
Editor-style incremental reparse and resumable chunk graphs are Tier B. No synchronous
public parser or WASM entry point is defined.

Default source limits live in `cem_ml::limits`:

- `MAX_SOURCE_BYTES_PER_SOURCE_ID = 64 MiB`;
- `MAX_SOURCE_CHUNK_BYTES = 64 KiB` recommended adapter chunk size;
- `MAX_DECODER_CARRY_BYTES = 4`;
- `MAX_TOKEN_BUFFER_BYTES = 64 KiB`;
- `MAX_DECODED_SCALARS_PER_CHUNK = 64 Ki`;
- `MAX_LINE_COUNT = 8 M`;
- `MAX_DIAGNOSTIC_SNIPPET_BYTES = 1 KiB`;
- `MAX_SOURCE_MAP_FRAMES = 32`;
- `MAX_AST_DEPTH = 1024`;
- `MAX_DIAGNOSTICS_PER_SOURCE = 10_000`.

### 3.2 Layer 2: SchemaTokenizer (`cem_ml::tokenizer`)

The tokenizer is mode-aware and schema-guided. The canonical profile is the CEM-native
curly tokenizer for `{name @attributes | content...}`, `$` expression nodes, anonymous
typed scopes, directives, comments, and rich-content enclosures. XML and HTML profiles
are secondary parity inputs that lower to the same `RawToken` and `NormalizedEvent`
contracts.

The HTML parity profile is a custom WHATWG-state tokenizer, not a wrapper around an
external tokenizer crate. The custom implementation is required so every token and token
sub-span can preserve source-map stacks through decoded streams and nested embedded
handoff layers. It extracts source-spanned tokens and switches lexical states; it does
not construct either the initial HTML parser DOM or the WHATWG implementation DOM.

Tokenizer buffering is token-local. The tokenizer may accumulate decoded scalars and
their byte spans while a token is incomplete, including format-defined lookahead, but it
must emit or fail the token before the buffer exceeds `MAX_TOKEN_BUFFER_BYTES`. After
`RawToken` emission, the accumulator is released; downstream layers receive token data
and byte ranges, not retained source text. The tokenizer does not require a specific
memory-buffer strategy.

Layer 1 decodes only byte encodings such as UTF-8, UTF-16, and Latin-1. Language-local
encodings recognized by a tokenizer, such as HTML character references, XML entity
references, CSS escapes, JSON string escapes, or CSV quoted escapes, append an
`EscapeDecoded` source-map frame. The frame maps decoded scalar ranges in the token value
back to the raw byte ranges that produced them. Multi-scalar entities record one decoded
range per emitted scalar, each pointing at the same raw source span when appropriate.
Tokens without local decoding omit the frame.

```
RawToken:
  kind: CemToken | HtmlToken | XmlToken
  byte_range: ByteRange
  source_id: SourceId
  source_map: SourceMapStack

HtmlToken:
  Doctype { name, public_id, system_id, force_quirks }
  StartTag { name: String, attributes: [(name, value, name_range, value_range)], self_closing }
  EndTag { name: String }
  Text { data: String }
  Comment { data: String }
  ProcessingInstruction { target: String, data: String }
  ParseError { code: HtmlErrorCode }

CemToken:
  NodeStart { name: String }
  NodeEnd
  Attribute { name: String, value: Option<ScalarValue> }
  ContentBoundary
  Text { data: String }
  ExpressionNode { data: String }
  AnonymousScopeStart
  Directive { name: String, data: String }
  Comment { data: String }
  RichContent { data: String, enclosure: RichContentEnclosure }
```

The `StartTag` attributes carry both the name and value ranges so the event normalizer
can emit per-attribute byte offsets into the source-map stack. `RawToken.source_map`
maps the token's local `source_id` and `byte_range` back through any decoded stream or
embedded handoff boundary that produced it. Attribute sub-ranges are interpreted within
that same token source-map context.

For CEM-native tokenization, attribute-value mode owns `{...}` cem-ql spans; content
mode treats `{name ...}` as a child node and `{$ ...}` as an expression node. Bare
`{.name}` text interpolation is rejected before event normalization.

WHATWG tokenizer states (data, RCDATA, RAWTEXT, script-data, tag-open, attribute-value,
etc.) are internal to this layer. The schema can select valid tokenizer contexts and
embedded-content boundaries, and the compiler may place schema-owned lexical/mode or
embedded-boundary diagnostics in this layer when they require only local token context.
Tokenizer-executed schema diagnostics do not make the tokenizer the semantic source of
truth and do not rewrite WHATWG lexical behavior.

XML tokenizer follows the same `RawToken` shape using an XML 1.0 profile, keeping Layers
3 and above format-agnostic. XML external resources and compatibility behavior are owned
by the XML content-type transform, not the tokenizer.

### 3.3 Layer 3: EventNormalizer (`cem_ml::events`)

```
NormalizedEvent:
  OpenScope  { name: QName, byte_range: ByteRange }
  CloseScope { name: QName, byte_range: ByteRange, synthesis: Synthesis }
  Name       { name: QName, byte_range: ByteRange }
  Value      { value: ScalarValue, byte_range: ByteRange }
  Trivia     { kind: TriviaKind, byte_range: ByteRange }
  ProcessingInstruction { target: String, data: String, byte_range: ByteRange }
  Separator  { kind: SeparatorKind, byte_range: ByteRange }
  ModeSwitch { content_type: ContentType, handoff: HandoffRecord }
  Error      { code: DiagCode, byte_range: ByteRange, severity: Severity }

ScalarValue: Text(String) | Int(i64) | Float(f64) | Bool(bool) | Null
TriviaKind: Whitespace(String) | Comment(String)
SeparatorKind: ElementBoundary | Comma | Colon | Delimiter | Newline
Synthesis:
  Real
  SelfClosing
  VoidElement
  ImpliedByStartTag
  ImpliedByAncestorClose
  ImpliedByEof
```

HTML token mapping:

| HTML token                               | Emitted events                                                                          |
|------------------------------------------|-----------------------------------------------------------------------------------------|
| `StartTag { name, attrs }`               | Close any non-embeddable current scope implied by this start tag, then `OpenScope { name }`, then for each attr: `Name { name: attr_qname }` + `Value { attr_value }`, then `Separator { kind: ElementBoundary }` |
| `StartTag { name, attrs, self_closing }` | Same as `StartTag`; if the active content-type/schema scope permits self-closing semantics, append `CloseScope { name, synthesis: SelfClosing }` after `ElementBoundary` |
| HTML void `StartTag`                     | Same as `StartTag`, then append `CloseScope { name, synthesis: VoidElement }` after `ElementBoundary` |
| HTML non-void `StartTag` with `/>`       | Same as `StartTag`, plus `cem.html.invalid_self_close`; the element remains open unless closed by another rule |
| `EndTag { name }`                        | If policy allows recovery, close any still-open descendants implied by this ancestor end tag, then `CloseScope { name, synthesis: Real }` |
| `Text { data }`                          | `Value { Text(data) }` when content; otherwise `Trivia { Whitespace(data) }` if preserved |
| `StartTag { name: "style" \| "script" }` | `OpenScope`, then `ModeSwitch { content_type }`                                         |
| `Comment`                                | `Trivia { Comment(data) }` unless the effective scope policy strips comments            |
| `ProcessingInstruction`                  | `ProcessingInstruction { target, data }`; schema/policy decides accept/diagnose/strip   |
| `ParseError`                             | `Error { ... }`                                                                         |

Each `StartTag` emits its `OpenScope` first, then one `Name`+`Value` pair per attribute,
preserving attribute source positions, then `Separator { kind: ElementBoundary }` at
the start tag closing delimiter. `ElementBoundary` closes the lexical start-tag
attribute segment only; schema-declared header/prelude child scopes may still update the
parent frame's effective attributes, namespace bindings, or other parameters before body
content begins. Immediate synthetic closes for self-closing and void elements use the
same delimiter byte range. Implied closes caused by a later start tag use that later
start tag's first byte. Implied closes caused by an ancestor end tag use the ancestor end
tag range. EOF recovery closes use the EOF range.

Synthetic close events close already-open scopes only; they do not insert missing
elements. Missing HTML container insertion such as `tbody`/`tr`, foster parenting, and
other WHATWG implementation DOM effects are content-type transform behavior.

Trivia preservation is policy-driven. The document root policy is seeded by CLI/config;
child context scopes inherit it unless their content type or schema overrides it. Context
entries such as `<pre>`, `<textarea>`, `<style>`, and `<script>` may preserve whitespace
as content or preserve comments as trivia independently from the parent HTML scope.

### 3.4 Layer 4: SchemaMachine (`cem_ml::schema`)

The CEM semantic vocabulary is defined functionally in the primary design's Layer 4
section. CEM-native declarative schema files are the source of truth. Implementation
code consumes the active compiled schema; `schema::vocab` is an implementation
convenience generated from, or kept traceable to, that compiled CEM-native schema.

```
CompiledSchema:
  schema_id: SchemaId
  namespace_uri: NamespaceUri
  version_identity: SchemaVersionIdentity
  source: CemNativeSchemaSource
  structural: StructuralSchemaIr
  semantic_rules: Vec<SemanticRule>
  transform_plans: Vec<TransformPlanRef>
  open_content: OpenContentPolicy

OpenContentPolicy:
  rules: Vec<OpenContentRule>
  defaults: OpenContentDefaults

OpenContentRule:
  content_model: Option<ContentModelId>
  namespace_uri: Option<NamespaceUri>
  open: bool
  unknown_element: OpenContentAction
  unknown_attribute: OpenContentAction
  source: SourceMapStack

OpenContentAction:
  Accept
  AcceptIgnore
  DeferToSemanticPass
  DelegateToRegisteredSchema
  Diagnostic { code: DiagCode, severity: Severity }

OpenContentDefaults:
  html_unknown_element: Diagnostic { code: cem.schema.unknown_html_element, severity: Error }
  html_unknown_attribute: Diagnostic { code: cem.schema.unknown_html_attribute, severity: Warning }
  html_custom_element: Accept
  cem_html_data_attribute: AcceptIgnore
  aria_or_role_attribute: DeferToSemanticPass
  active_cem_unknown_element: Diagnostic { code: cem.schema.unknown_cem_element, severity: Error }
  active_cem_unknown_attribute: Diagnostic { code: cem.schema.unknown_cem_attribute, severity: Error }
  other_registered_schema: DelegateToRegisteredSchema
  unbound_prefix: Diagnostic { code: cem.schema.unbound_prefix, severity: Error }
  no_namespace_open_true_element: Diagnostic { code: cem.schema.extension_element, severity: Warning }
  no_namespace_open_true_attribute: Diagnostic { code: cem.schema.extension_attribute, severity: Warning }
  no_namespace_open_false_element: Diagnostic { code: cem.schema.unknown_element, severity: Error }
  no_namespace_open_false_attribute: Diagnostic { code: cem.schema.unknown_attribute, severity: Error }
  vendor_prefixed_html_attribute: Diagnostic { code: cem.schema.unknown_html_attribute, severity: Warning }

CemNativeSchemaSource:
  uri: String
  version: String                    complete SemVer 2.0 descriptor.version
  source_id: SourceId
  source_map: SourceMapStack

SchemaVersionIdentity:
  uri: String                         stable schema identity / author constraint
  embedded_version: SemVer            authoritative complete descriptor version
  constraint: SchemaVersionConstraint
  match_rule: SchemaVersionMatchRule
  fingerprint_input: String           embedded_version including prerelease/build

SchemaVersionConstraint:
  Unconstrained
  Major(u64)
  MajorMinor(u64, u64)
  Full(SemVer)

SchemaVersionMatchRule:
  Unconstrained
  Major
  MajorMinor
  Full
  PrereleaseExact

StructuralSchemaIr:
  entry_state: SchemaState
  relax_ng_equivalent: RelaxNgEquivalentIr
  tier_a_profile: TierAValidationProfile
  states: Vec<SchemaStateDef>        DFA-ready limited structural states for Tier A
  derivative: Option<DerivativeIr>   full residual/derivative representation
  diagnostics: EngineDiagnosticProfile

EngineDiagnosticProfile:
  engine: ValidationEngineKind
  expected_content: ExpectedContentDiagnosticMode
  report_compatibility: ReportCompatibility

ValidationEngineKind:
  TierADfa
  RelaxNgDerivative

ExpectedContentDiagnosticMode:
  None
  DfaFollowSet
  DerivativeResidual

ReportCompatibility:
  EngineVersionLocal                 no compatibility guarantee across validation engines

TierAValidationProfile:
  supported_constraints: Vec<StructuralConstraintKind>
  unsupported_policy: UnsupportedConstraintPolicy

UnsupportedConstraintPolicy:
  CompileError(DiagCode)             default: cem.schema.unsupported_tier_a_constraint

SemanticRule:
  rule_id: SemanticRuleId
  phase: SemanticRulePhase           CrossReference | Contextual | Policy | Transform
  dependency_tier: ConstraintTier
  execution: RuleExecutionPlacement
  applies_to: ExpandedName
  severity: Severity
  source: SourceMapStack

ConstraintTier:
  Structural
  CrossReference
  SemanticContextual

RuleExecutionPlacement:
  Tokenizer
  EventNormalizer
  SchemaMachine
  ReferenceResolution
  AstValidation
  Transform
  Policy

ScopePolicy:
  scope_id: ScopeId
  parent: Option<ScopeId>
  content_type: ContentType
  error_boundary: ErrorBoundaryKind
  resources: ContentTypePolicy
  errors: ErrorBoundaryPolicy
  trivia: TriviaPolicy
  diagnostics: DiagnosticVisibility
  parent_override: ParentPolicyOverride

ErrorBoundaryKind:
  SchemaDeclared
  ContextRoot
  None

ErrorBoundaryPolicy:
  default_action: ErrorAction
  severity_floor: Severity
  severity_by_code: HashMap<DiagCode, Severity>
  recover_with_error_subtree: bool

ErrorAction:
  Inherit
  HideAndContinue
  ReportAndContinue
  AbortScope
  AbortParse

DiagnosticVisibility:
  Public
  ScopeLocal
  HiddenFromParent

TriviaPolicy:
  comments: TriviaDisposition
  whitespace: TriviaDisposition
  processing_instructions: ProcessingInstructionPolicy

TriviaDisposition:
  Preserve
  Strip

ProcessingInstructionPolicy:
  SchemaDriven

ParentPolicyOverride:
  force_visibility: Option<DiagnosticVisibility>
  severity_floor: Option<Severity>
  force_abort_at: Option<Severity>
  resource_ceiling: Option<ContentTypePolicy>
```

The compiler boundary is:

```
CEM-native schema source
  -> CompiledSchema
  -> StructuralSchemaIr for the SchemaMachine
  -> SemanticRule registry with dependency tier and execution placement
  -> TransformPlan metadata for the CEM template renderer
```

Only `StructuralSchemaIr` is consumed during streaming event validation. Its structural
semantics are RELAX-NG-equivalent. Tier A executes the `tier_a_profile` DFA subset and
rejects unsupported structural constraints at schema compile time; later derivative
runtimes consume `relax_ng_equivalent` / `derivative` without changing schema semantics.
Switching validation engines may change diagnostic codes, payload shapes,
expected-content sets, ordering, wording, and report snapshots; report compatibility
across the DFA and derivative engines is not required.
Cross-reference, contextual, lexical/mode, policy, and transform checks are emitted as
`SemanticRule`s with an explicit `RuleExecutionPlacement`. A rule runs at the earliest
safe layer whose input data satisfies its `ConstraintTier`: tokenizer or normalizer for
local lexical/mode constraints, SchemaMachine for structural constraints,
reference-resolution for slot/name constraints, AST validation for context-sensitive
constraints, and transform/policy placement for rendering, resource, or security
constraints.

`OpenContentPolicy` is consulted by the SchemaMachine for each unknown `OpenScope` and
attribute `Name` after `ExpandedName` resolution. Rules are keyed by content model and
namespace. HTML `data-*` attributes resolve to the synthetic `cem:html-data` namespace,
are accepted, and are ignored by schema validation unless an explicit schema rule maps
that synthetic namespace. ARIA and `role` attributes are accepted here and deferred to a
later semantic pass. WHATWG custom element names are accepted without checking the
browser registry; attributes on those elements still follow the same
namespace/open-content policy. Vendor-prefixed HTML attributes such as `x-data` use
`vendor_prefixed_html_attribute` unless a registered extension namespace or explicit
open-content rule accepts them.

`ScopePolicy` is resolved when a context scope is created. The document root creates the
initial policy. Child parser, handoff, transform, and embedded-content scopes inherit
that policy, apply any local content-type or schema-declared policy changes, and then
apply the owner scope's `ParentPolicyOverride`. The resulting `effective_policy` is
stored on the `SchemaFrame`. A child scope may hide or relax handling for its own errors
only when the parent override permits it. `ErrorBoundaryPolicy.severity_by_code` maps
stable `DiagCode`s to severity overrides for that scope; unresolved reference diagnostics
default to `Warning` when no override is present. CLI parameters or config seed the
document root `ScopePolicy` before parsing begins.

`TriviaPolicy` defaults to preserving comments and whitespace. CLI/config can set the
document-level default, and schema/content-type scopes can override it for child
contexts. Processing instructions are schema-driven: the tokenizer and normalizer
preserve their source range, and the active schema decides whether they are accepted,
diagnosed, transformed, or stripped. Diagnostics and source maps retain the event-time
source-map stack. When a report projects the input frame, stripped trivia still counts
for byte offsets, line/column projection, snippets, and report events; transform-facing
reports may project a generated or intermediate frame instead.

Diagnostic propagation walks from the origin scope frame toward its ancestors until it
reaches the nearest scope whose `error_boundary` is `SchemaDeclared` or `ContextRoot`.
`ContextRoot` means the nearest active context-root scope, not necessarily the document
root; embedded content, option/prelude, namespace, and schema-declared mid-tree contexts
can all be the applicable root for diagnostics emitted inside them. If no
schema-declared boundary is found before that context root, the context root handles the
diagnostic. The boundary scope applies its effective policy and becomes the diagnostic's
`boundary_scope`.

```
SchemaFrame:
  scope_id: ScopeId
  schema_id: SchemaId
  schema_version: SchemaVersionIdentity
  language_id: ContentType          e.g. text/html, text/css
  phase: FramePhase
  attr_state: AttributeState
  content_state: ContentState
  recovery: Option<ErrorSubtreeState>
  effective_policy: ScopePolicy
  source_span: ByteRange            range of the element that opened this frame
  source_map_stack: SourceMapStack  accumulated map at frame entry
  expected_close: Option<QName>     for element-level close validation
  namespace_ctx: Option<NsContext>
  diagnostics: Vec<Diagnostic>

FramePhase:
  Attribute
  Header
  Content
  Closed

AttributeState:
  active: HashMap<ExpandedName, AttributeOccurrence>
  pending: Option<ExpandedName>
  required_tag_remaining: HashSet<ExpandedName>
  required_header_remaining: HashSet<ExpandedName>
  header_declarations: Vec<HeaderDeclaration>

AttributeOccurrence:
  name: QName
  value_range: Option<ByteRange>
  source_map: SourceMapStack

HeaderDeclaration:
  kind: Attribute | Namespace | Parameter
  name: QName
  value_range: Option<ByteRange>
  source_map: SourceMapStack

ContentState:
  residual_or_dfa_state: SchemaState
  seen_children: Vec<(ExpandedName, ByteRange)>
  required_remaining_children: HashSet<ExpandedName>

ErrorSubtreeState:
  expected_close: Option<QName>
  taint_ast_nodes: bool
```

Schema resolution populates `SchemaVersionIdentity` before a `SchemaFrame` starts
validating. URI-tail constraints are matched against the embedded complete SemVer per
`cem-ml-ac.md` AC-V-9..AC-V-13. The schema machine emits
`cem.v.semver_resolved` into the report AST, then applies compatible-minor warning and
major-mismatch abort rules through the same diagnostic bubbling path as structural
validation.

`AttributeState.active` stores the effective attribute set by `ExpandedName`, including
lexical start-tag attributes and schema-declared header/prelude attribute forms.
Duplicate attributes are resolved after namespace binding with last-writer-wins
semantics: a later attribute with the same expanded name replaces the active value and
source range instead of emitting a duplicate-attribute diagnostic. Multi-valued
attribute semantics are validated by value-shape rules. Header/prelude declarations are
only accepted at the beginning of the element body; the first non-header event finalizes
`required_header_remaining` before being consumed as content. `ContentState.seen_children`
stores only children needed for diagnostics, in emission order.
`required_remaining_children` is a diagnostic mirror for close-time missing-child
messages; ordering and multiplicity remain enforced by
`residual_or_dfa_state`. Unordered-but-required content groups use the set tracker plus a
residual or DFA state that accepts the allowed order. Attribute-order constraints are
unsupported in Tier A and fail schema compilation with `cem.schema.unsupported_constraint`.

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

QName:
  lexical_name: String              exact source spelling after decoding
  prefix: Option<String>
  local_name: String                namespace-policy local name
  expanded_name: ExpandedName
  binding_id: Option<NamespaceBindingId>
  kind: NameKind
  source_range: ByteRange

NameKind:
  Element
  Attribute

ExpandedName:
  namespace_uri: NamespaceUri
  schema_id: Option<SchemaId>
  local_name: String

NameResolution:
  lexical_name: String
  prefix: Option<String>
  local_name: String
  expanded_name: ExpandedName
  binding_id: Option<NamespaceBindingId>
  kind: NameKind
  source_range: ByteRange
```

Resolution uses the namespace context visible at the source position being parsed.
Within a single scope, the latest binding with the same `NamespaceName` wins from its
`effective_from` position forward. Nested scopes inherit parent bindings, but an inner
binding with the same name shadows the inherited binding until the inner scope closes.

The normalizer resolves `QName` before emitting `OpenScope`, `CloseScope`, or `Name`.
HTML element names bind to the HTML namespace with ASCII-lowercased `local_name`.
Attributes are case-sensitive in every context and may use camelCase; their `local_name`
preserves the source spelling after prefix removal. XML and non-HTML child contexts
preserve element and attribute case unless that context's schema declares a different
policy. HTML `data-*` attributes bind to the synthetic `cem:html-data` namespace with a
case-sensitive local name equal to the suffix after `data-`.

Foreign content such as SVG and MathML is represented as a `ModeSwitch` to a child
content-type scope with its own `NsContext`, schema id, and name policy. The HTML parent
does not keep a special in-place foreign-content name mode; source-map frames link the
parent token range to the child context.

Previously emitted `NameResolution` records are immutable. If `screen` resolves to
`{NS1}screen`, and a later default namespace declaration changes the default namespace to
`NS2`, later `screen` names resolve to `{NS2}screen` but the earlier record still points
to the `NS1` binding id. Reports and source maps can therefore show that the same
lexical name changed schema ownership over time.

Attribute and tag collision checks use `ExpandedName`, not lexical spelling. Duplicate
attributes in the same start tag use the later `ExpandedName` occurrence as the active
value. Rendered projections may lower schema-qualified names to unqualified convenience
attributes or tags, but the renderer must retain enough mapping metadata to distinguish
generated CEM-owned names from pass-through HTML names.

State transitions:

```
open(event):
  If current frame phase = Header and event is a schema-declared header/prelude scope,
  validate the declaration, push its child frame, and arrange its close result to update
  the parent AttributeState or namespace_ctx.
  If current frame phase = Header and event is not a header/prelude scope, finalize
  required_header_remaining, transition current frame to Content, then re-dispatch event.
  Require current frame phase = Content.
  Validate OpenScope name, ordering, and multiplicity against content_state.
  If the name is unknown, consult open_content before emitting the diagnostic.
  Record diagnostic-relevant child in seen_children and update required_remaining_children.
  Push child SchemaFrame with phase = Attribute and required attr/content sets from schema.

name(event):
  Require current frame phase = Attribute.
  Use the event's already-resolved QName.
  Check unknown attribute rules against schema and open_content.
  Insert or replace attr_state.active by ExpandedName using last-writer-wins semantics.
  Set attr_state.pending to the event's ExpandedName.
  Remove required_tag_remaining entry when the effective lexical attribute is accepted.

value(event):
  In Attribute phase, validate the pending attribute's value shape and update its
  AttributeOccurrence.value_range.
  In Header phase, finalize required_header_remaining, transition to Content, then
  re-dispatch as body text.
  In Content phase, validate text content against content_state.residual_or_dfa_state.

separator(event):
  On ElementBoundary in Attribute phase, emit required lexical-attribute diagnostics,
  then transition the frame to Header phase.
  For other separators, advance sequence, record, or property pointer in current state.

handoff(event):
  Emit HandoffRecord.
  Push child frame with child content_type and child schema_id.

close(event):
  If current frame phase = Header, finalize required_header_remaining before close-time
  child requirements.
  Validate expected_close, required_remaining_children, and nullable/complete state.
  Transition phase to Closed.
  Pop frame; propagate close result to parent frame.

error(event):
  Create Diagnostic with origin_scope = current frame scope.
  Bubble to nearest SchemaDeclared or active ContextRoot error boundary.
  Evaluate boundary frame's effective ScopePolicy.
  Hide, report, push ErrorSubtree recovery frame, abort boundary scope, or abort full
  parse per policy.

transform(event):
  Append SourceMapFrame to current source_map_stack.

encode(node):          [Tier B] assign binary node ids and dictionary refs.
segment(subtree):      [Tier B] close a subtree-root chunk.
```

### 3.4.2 Schema Compiler Output Module (`cem_ml::schema::compiler`)

Implementation counterpart of
[`cem-ml-stack-design.md` §13.2](cem-ml-stack-design.md#132-schema-compiler-output-module).
Closes AC-S-2..AC-S-6, AC-ALIGN-010, and DESIGN-FOLLOW-001. Open questions
(blocking the first PR) live in
[`cem-ml-schema-compiler-open-questions.md`](cem-ml-schema-compiler-open-questions.md).

#### 3.4.2.1 Module Layout

```
cem_ml/src/schema/compiler/
  mod.rs              SchemaCompiler entry point, CompilerOptions, emit_all()
  output.rs           CompilerOutput, EmittedArtifact, ArtifactKind, PublicationManifest
  emitter.rs          SchemaEmitter trait, EmissionCursor, deterministic encoder helpers
  rng_xml.rs          RELAX NG XML mirror emitter (AC-S-2)
  rng_compact.rs      RELAX NG compact-syntax mirror emitter (AC-S-2)
  ts_dts.rs           TypeScript .d.ts emitter (AC-S-3, AC-S-6); structural + Validated<T>
  rust_hdr.rs         Rust .rs emitter (AC-S-4); behind CompilerOptions.emit_rust
  uri_publish.rs      Manifest writer, hash sidecars, URI resolution helpers (AC-S-5)
  byte_stability.rs   write_lf, write_indent, BTreeMap iteration helpers, hash sink
  error.rs            EmitError enum (IoError, UnsupportedConstraint, MissingIrField, …)
```

This subdirectory is the resolved location for schema compiler output
code and matches the §4 module map below.

#### 3.4.2.2 Public Rust Surface

```rust
pub struct SchemaCompiler;

pub struct CompilerOptions {
    pub emit_rust: bool,                    // gates rust_hdr (AC-S-4 Tier B)
    pub emit_dts: bool,                     // default true (AC-S-3)
    pub emit_rng_xml: bool,                 // default true (AC-S-2)
    pub emit_rng_compact: bool,             // default true (AC-S-2)
    pub include_validated_brand: bool,      // default true (AC-S-6)
    pub embed_source_header: bool,          // default true; emits schema URI + embedded version preamble per OQ-SC-8 (resolved). Content hash is NEVER in the header — it lives in the .hash sidecar only.
}

pub struct CompilerOutput {
    pub schema_id: SchemaId,
    pub schema_uri: String,
    pub embedded_version: SemVer,
    pub artifacts: Vec<EmittedArtifact>,
    pub manifest: PublicationManifest,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArtifactKind {
    RelaxNgXml,
    RelaxNgCompact,
    TypeScriptDts,
    RustHeader,
    Manifest,
}

pub struct EmittedArtifact {
    pub kind: ArtifactKind,
    pub relative_path: String,              // under dist/lib/schema/
    pub bytes: Vec<u8>,
    pub content_hash: ContentHash,          // cem-bin/1+blake3
    pub source_map: SourceMapStack,
}

pub struct PublicationManifest {
    pub schema_uri: String,
    pub embedded_version: SemVer,
    pub hash_scheme: &'static str,          // always "cem-bin/1+blake3"
    pub artifacts: BTreeMap<ArtifactKind, ArtifactDescriptor>,
}

pub struct ArtifactDescriptor {
    pub relative_path: String,
    pub content_hash: ContentHash,
    pub byte_length: u64,
    pub emitted_by: EmitterTag,             // crate version + emitter name; not part of hash input
    pub validated_by: Option<ValidatorTag>, // recorded only after the verification fixture passes
}

pub trait SchemaEmitter {
    const KIND: ArtifactKind;
    const EXTENSION: &'static str;
    fn emit(
        &self,
        schema: &CompiledSchema,
        options: &CompilerOptions,
        cursor: &mut EmissionCursor<'_>,
    ) -> Result<EmittedArtifact, EmitError>;
}

impl SchemaCompiler {
    pub fn emit_all(
        schema: &CompiledSchema,
        options: &CompilerOptions,
    ) -> Result<CompilerOutput, EmitError>;

    pub fn write_to_disk(
        output: &CompilerOutput,
        root_dir: &Path,                    // packages/cem_ml/dist/lib/schema/
    ) -> Result<(), EmitError>;
}
```

`EmissionCursor` walks `CompiledSchema` in a fixed order: namespace
bindings, annotations (alphabetical by local name), states, semantic rules
(by `rule_id`), open-content rules (by content model then namespace), and
finally the schema-version identity record. Cursor methods take `&mut
self` only for accumulator buffering; the visited IR is borrowed
immutably.

#### 3.4.2.3 Determinism Helpers (`byte_stability.rs`)

```rust
pub struct DeterministicWriter<'a> {
    sink: &'a mut Vec<u8>,
    indent: u16,
    hasher: blake3::Hasher,                 // updated on every byte
}

impl DeterministicWriter<'_> {
    pub fn line(&mut self, s: &str);        // appends s + b'\n', rejects '\r' and trailing spaces
    pub fn indent(&mut self);               // 2-space step
    pub fn dedent(&mut self);
    pub fn finalize(self) -> (Vec<u8>, ContentHash);
}
```

All emitters route writes through `DeterministicWriter`. The writer
rejects CR bytes, trailing whitespace, and missing final newline at debug
time (`debug_assert!`) and emits `EmitError::NonDeterministicWrite` in
release. `BTreeMap`/`BTreeSet` iteration is the only allowed source of
ordered output; ad-hoc `HashMap` iteration is forbidden.

#### 3.4.2.4 Emitter Sketches

**`rng_xml.rs`** — emits a single `<grammar>` document, one `<element>` per
annotation host, one `<attribute>` per CEM annotation in the active
namespace, and `<choice>` for enum-valued annotations. The XML preamble is
fixed (`<?xml version="1.0" encoding="UTF-8"?>`); namespace declarations
are alphabetized after the well-known `xmlns`/`xmlns:cem` preamble. Host
patterns accept pass-through non-CEM attributes while excluding unknown
attributes in the active CEM namespace. State attributes lower to RELAX NG
`list` patterns so a single `cem:state` value can carry a space-separated
state list; annotation-anchored host variants use the active annotation's
`allowed_states`, while the state-only fallback uses the global state
matrix. Non-streamable constraint markers from
`CompiledSchema.non_streamable_constraints` raise
`EmitError::UnsupportedConstraint` at emit time, mirroring the
`cem.schema.unsupported_constraint` diagnostic from the SchemaMachine.

**`rng_compact.rs`** — emits the compact syntax derived from the same
emission cursor. Round-trip with `rng_xml` through native RELAX NG tooling is
the byte-stability test (`rng_compact_roundtrip.rs`).

**`ts_dts.rs`** — emits:

```ts
// AUTO-GENERATED. CEM-native source: <schema-uri> @<embedded-version>
import type { Validated as RuntimeValidated } from "@epa-wg/cem-ml/wasm";
export { asValidated, tryValidated } from "@epa-wg/cem-ml/wasm";
declare const cemSchemaVersionBrand: unique symbol;
export type Validated<T> = RuntimeValidated<T> & {
  readonly [cemSchemaVersionBrand]: "<schema-uri>@<embedded-version>";
};

export interface Badge extends HTMLElement {
  readonly cemBadge?: "success" | "info" | "warning" | "error";
  readonly cemState?: "default";
}
```

Header carries exactly two fields — schema URI and embedded version —
per OQ-SC-8 (resolved). The content hash is **not** in the header; it
lives in the `cem-core.d.ts.hash` sidecar.

Structural interfaces inherit from the appropriate `lib.dom.d.ts` base
(`HTMLElement`, `SVGElement`, `XMLDocument`) per AC-S-V-1. `Validated<T>`
uses a `unique symbol` brand inside an intersection so the brand carries
through DOM-typed call sites unchanged (AC-S-V-3). Version-identity
discrimination (AC-S-V-4) is encoded as a per-generated-module brand keyed
by the schema URI and embedded SemVer at emit time.

Per OQ-SC-6 (resolved), the runtime side of `asValidated` /
`tryValidated` lives in the WASM build of `cem-ml` and is exposed at the
TS subpath `@epa-wg/cem-ml/wasm`. The emitter writes **re-exports** of
those runtime functions from that subpath, not local `declare function`
stubs and not a generated `.js` sibling. The local `Validated<T>` type
imports the WASM brand as `RuntimeValidated<T>` and intersects it with the
schema-version brand. AC-S-V-5 (validation-failure diagnostics
carry an AC-V-1-shaped code/severity and a source-map frame from the
caller) is satisfied at the WASM layer because that is where the inline
validation diagnostic and source-map stitching already live (AC-V-1);
re-exporting keeps the schema artifact aligned with that single
diagnostic surface instead of duplicating the runtime per schema.

Per OQ-SC-7 (resolved), each embedded schema version emits its own
`.d.ts` under the per-version on-disk path. Cross-version
discrimination (AC-S-V-4) is the *combination* of two mechanisms: each
generated `.d.ts` declares its own schema-version `unique symbol` brand,
and the package export subpath convention
`@epa-wg/cem-ml/schema/<tail>/<version>/<stem>` (see
`cem-ml-stack-design.md` §13.2.5) is what makes the two `Badge` symbols
reachable in the same TS project at once. The emitter does not write a
combined or re-export `.d.ts`; subpath isolation comes from the
package's `exports` field, written by the release tooling.

**`rust_hdr.rs`** — emits one module per schema:

```rust
//! AUTO-GENERATED. CEM-native source: <schema-uri> @<embedded-version>
#![allow(non_camel_case_types, dead_code)]

pub mod schema {
    pub const SCHEMA_URI: &str = "<schema-uri>";
    pub const EMBEDDED_VERSION: &str = "<semver>";

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Badge { Success, Info, Warning, Error }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum CemState { Default, Hover, /* ... */ }
}
```

The AC-S-4 implementation is Tier A code behind the OQ-SC-3 Tier B gate.
The current emitted surface is constants, one enum per enum-valued
annotation, and `CemState`; free-form annotations remain `&str` at call
sites and do not emit host-bound structs. The verification fixture runs
`cargo check --offline` against a generated stub crate when the Rust gate
is enabled. `rustfmt --check` integration remains a Tier B hardening item;
the Tier A byte-stability guard is the deterministic writer plus compile
fixture.

**`uri_publish.rs`** — orders artifact descriptors by
`ArtifactKind` ordinal (the enum declaration order is the canonical
write order), then serializes the manifest with a stable JSON writer
(see `byte_stability.rs`). Hash sidecars are
`{relative_path}.hash` files whose body is exactly
`cem-bin/1+blake3:<hex>\n`.

#### 3.4.2.5 Filesystem Writer

`SchemaCompiler::write_to_disk` writes artifacts under
`packages/cem_ml/dist/lib/schema/<tail>/<embedded-version>/` and the
manifest at the same level. Writes go through a `TempThenRename` adapter
so a partial publication does not leave the manifest pointing at
truncated files. The previous manifest is replaced only after every
artifact and every `.hash` sidecar are on disk.

#### 3.4.2.6 Verification Harness

Fixture root: `packages/cem_ml/tests/schema_emit/`.

| File                          | AC                                  | What runs                                                                               |
|-------------------------------|-------------------------------------|-----------------------------------------------------------------------------------------|
| `byte_stability.rs`           | AC-S-2                              | `emit_all` twice over the same `CompiledSchema`; assert byte-identical artifacts and identical content hashes. |
| `rng_xml_parity.rs`           | AC-S-2 RELAX NG mirror              | Validate canonical `examples/cem-ml/*.cem` projections against the emitted `.rng` through `xmllint --relaxng`. Skip under `CEM_ML_SCHEMA_PARITY_SKIP=1`. |
| `rng_compact_roundtrip.rs`    | AC-S-2 compact mirror               | Convert emitted `.rnc` to `.rng` through Trang; diff against `rng_xml`'s output.        |
| `ts_dts_structural.rs`        | AC-S-V-1, AC-S-V-3                   | `tsc --noEmit` against a fixture `accepts(el: HTMLElement)` call site.                  |
| `ts_dts_validated_brand.rs`   | AC-S-V-2, AC-S-V-4, AC-S-V-5 (declaration shape) | `// @ts-expect-error` fixtures for plain-literal-to-`Validated<T>` and cross-version assignment, plus an assertion that the emitted `.d.ts` re-exports `asValidated`/`tryValidated`/`Validated` from `@epa-wg/cem-ml/wasm` (no host-stub `declare function` lines). AC-S-V-5 runtime-diagnostic verification lives with the WASM build's AC-V-1 fixtures (one source of truth). |
| `rust_hdr_compiles.rs`        | AC-S-4                              | `cargo check -p cem_ml_schema_stub` against a generated stub crate that imports the emitted `.rs`. |
| `uri_manifest_resolution.rs`  | AC-S-5, AC-V-10, AC-V-13             | Resolve `/1`, `/1.2`, `/1.2.3`, prerelease URIs through `cem_ml::loader`; assert manifest match-rule events. |

Each fixture is a standalone integration test under `tests/`. The
`rust_hdr_compiles.rs` fixture spawns a child `cargo check` process and
captures the exit code; its stub crate skeleton lives under
`packages/cem_ml/tests/schema_emit/fixtures/stub-crate/`.

#### 3.4.2.7 Nx Target Wiring

`packages/cem_ml/project.json` gains:

```json
"build:schema-artifacts": {
  "executor": "nx:run-commands",
  "dependsOn": ["build:docs"],
  "cache": true,
  "options": {
    "command": "cargo run --release --bin cem-ml-schema-emit --target-dir ../../dist/target/cem_ml -- --out packages/cem_ml/dist/lib/schema",
    "cwd": "{workspaceRoot}"
  },
  "inputs": [
    "{projectRoot}/schema/**/*.md",
    "{projectRoot}/src/schema/compiler/**/*.rs"
  ],
  "outputs": [
    "{projectRoot}/dist/lib/schema/**"
  ]
}
```

The release sequence in `cem-ml-stack-design.md` §18.4 gains a step
between `lint` and `test`:

```
1. nx run cem_ml:lint
2. nx run cem_ml:build:schema-artifacts        # NEW
3. nx run cem_ml:test                          # consumes the emitted manifest
4. nx run cem_ml:build:wasm
5. nx run cem_ml:bench
```

`build:schema-artifacts` MUST run before `test` so the schema-loader and
URI-resolution tests find the on-disk manifest.

### 3.5 Report Event Model (`cem_ml::report`)

Reports are owned by `cem_ml::report`, but their canonical internal data is an
AST-associated report tree rather than a flat diagnostic list. Each parser, schema,
handoff, transform, validation, or runtime log message is captured as a report event node
attached to:

- the current input DOM/AST or CEM AST node when one exists;
- the active event-time scope context, including URI, content type, schema id, namespace
  context, effective policy, active scope, and source span;
- the source-map stack as it exists at the moment the event is emitted;
- the partial DOM/AST hierarchy visible to the emitting layer at that moment; and
- a monotonic event sequence number that preserves emission order within the report.

The report hierarchy follows the source-map/layer hierarchy, but it is event-time state:
it records the parser or transform view when the log event happened, not the final
post-transform tree. Diagnostics before AST construction attach to the active scope
context frame at the point of emission, which may be a mid-tree options, namespace,
embedded-content, or schema scope. They are later linked to AST nodes when a matching
node exists, but their canonical location remains the original event-time source-map
stack and scope context.
Comments, whitespace, and processing instructions are part of the initial source stream
for reporting. Diagnostics can reference trivia byte ranges even when the active output
transform later removes those nodes.

### 3.6 CLI Projection Keys

Stack layers own data artifacts. The CLI owns projection selection, output targets, and
default stream behavior. Proposed projection layer keys:

| Key                | Stack owner                       | Projection meaning                                                                                       |
|--------------------|-----------------------------------|----------------------------------------------------------------------------------------------------------|
| `source`           | `source::SourceRecord`            | Source metadata, URI, byte length, and source id; raw bytes are not emitted unless explicitly requested. |
| `decoded`          | `source::decode`                  | Encoding result, decoded scalar spans, replacement/encoding diagnostics, and line-index metadata.        |
| `tokens`           | `tokenizer`                       | Format-native token stream with byte ranges.                                                             |
| `events`           | `events`                          | Normalized open/close/name/value/trivia/processing-instruction/separator/mode-switch/error events.       |
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
| `report-ast`       | `report`                          | Canonical AST-associated report tree with event sequence and event-time scope context.                   |
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
  decoded_stream: DecodedStreamRef   source-mapped stream prepared by the container
  inherited_ctx: InheritedContext    parent element name, attribute name, MIME type, namespace
  child_schema_id: Option<SchemaId>
  return_condition: ReturnCondition

DecodedStreamRef:
  source_id: SourceId                synthetic or parent-backed source identity
  decoded_units: DecodedUnitStream   scalar/token units visible to the child context
  source_map: SourceMapStack         maps stream units back to container ranges

ReturnCondition:
  MatchingEndTag(QName)              e.g. </style>, </script>
  CdataEnd                           XML CDATA ]]>
  EntityEnd                          XML entity/reference boundary
  AttributeEnd                       attribute quote close
  StringEnd                          JSON string end after unescape
  JsonValueEnd                       JSON object/array/value boundary
  BlockClose                         CSS block close
  FunctionClose                      CSS function close
  TemplateEnd                        TypeScript template string end
  JsxIslandEnd                       JSX island boundary
  FieldEnd                           CSV/CSF field delimiter
  FixedLength(u64)
```

Deferred return-condition variants are listed for interface stability. Implementation
priority after Tier A is XML first, JSON second, then HTML extensions and other embedded
language cases.

The current context parser recognizes the content-type switch and decodes its owned
content before handoff when decoding is required. `ModeSwitch` is the embedded-context
creation event: it carries the `HandoffRecord`, the mapped child content type, and the
source-mapped decoded stream. The reference implementation should use the CEM framework
to map entity context type and create the child context. The child parser consumes stream
units up to the return condition and signals completion. The parent resumes with the
source position following the condition boundary.

### 3.8 Layer 6: InputDomAstBuilder / InterpreterAstBuilder (`cem_ml::parser`)

```
InputDomNode:
  Document(InputDocument)
  Element(InputElement)             native XML/(X)HTML identity plus optional CEM annotations
  Attribute(InputAttribute)
  Text(TextNode)
  Whitespace(WhitespaceNode)
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

`InputDomNode` is the generic schema-defined AST surface. Tier A preserves text,
whitespace, comments, doctypes, processing instructions accepted by schema policy,
CDATA where supported by the content type, raw text, and recovered error nodes unless
the effective scope policy strips a trivia class. The CEM projection uses `CemNode` and
can carry non-CEM source constructs by reference through `InputNode(AstNodeId)`.

`CemAnnotation` records schema-qualified transform triggers such as `cem:screen` or
`cem:action`. Multiple annotations can attach to the same `InputElement`; the source
element keeps its `ExpandedName` and native DOM meaning. If a transform rewrites or
replaces the source element, the schema-owned transform plan resolves annotation
composition, precedence, rejection, or diagnostics.

Reference slots:

```
NameSlot:
  owner_scope: ScopeId
  target_name: String
  resolved: Option<AstNodeId>
  source: SourceMapStack

LabelRef(NameSlot)          - wraps a slot; filled when id="..." element is parsed
ForRef(NameSlot)            - for/id pairing
AriaRef(NameSlot)           - aria-labelledby, aria-describedby, etc.
```

When the parser encounters an element with an `id` attribute, it looks up the slot in
the document's `id_table` and fills it. Any prior `LabelRef`/`ForRef`/`AriaRef` holding
the same slot observes the fill immediately.

Forward references are represented as unfilled slots at parse time. One pass is
sufficient: when a target id appears, the parser fills the matching slot and all prior
references observe the resolved target. When an owning context scope closes, the schema
machine inspects unfilled `NameSlot`s owned by that scope and emits unresolved-reference
diagnostics. The default severity is `Warning`; the current `ScopePolicy` may override
severity per `DiagCode`, including CLI/config-provided overrides inherited from the
document root.

Implementation TBD: `NameSlot` is a logical contract, not a required
`Arc<Mutex<Option<AstNodeId>>>` public shape. Even though public parsing APIs are async,
slot resolution can remain task-local to one parse run. A generational slot arena or
handle table is likely simpler, faster, and easier to serialize into the future binary
AST. Thread-safe shared ownership is required only if the runtime exposes slots across
threads or concurrently polled tasks.

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
  template_module: CemTemplateModule
  rules: Vec<TemplateRule>         schema-owned CEM template rules

CemTemplateModule:
  module_id: TemplateModuleId
  templates: Vec<CemTemplate>
  query_language: QueryLanguage
  source: SourceMapStack

CemTemplate:
  template_id: TemplateId
  match_query: ScopedQuery
  priority: i32
  body: Vec<TemplateOp>
  params: Vec<TemplateParam>
  source: SourceMapStack

ScopedQuery:
  expression: String               cem-ql source per docs/cem-ql-ac.md
  allowed_context: QueryContextScope
  source: SourceMapStack

QueryLanguage:
  CemQl                            CEM scope and policy bounded query language

QueryContextScope:
  current_node: AstNodeId
  schema_scope: ScopeId
  state_slots: Vec<StateSlotId>
  resource_policy: ContentTypePolicy

TransformContext:
  schema_id: SchemaId
  base_uri: Option<String>
  fail_level: FailLevel
  query_scope: QueryContextScope

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

The reference implementation stack executes schema-driven CEM template plans with a
generic Rust renderer. Rust owns the renderer, query evaluator, diagnostics, source-map
preservation, and policy checks, but transform behavior comes from `CemTemplateModule`
and `TemplateRule` data emitted by the compiled schema. The renderer should provide
XSLT-like coverage for matching, value selection, conditionals, iteration, recursive
template application, copy/pass-through rules, named/template-reference calls, parameter
or state binding, and deterministic serialization.

The reference implementation also provides a schema-independent trivia-strip transform.
It removes `Comment` and `Whitespace` input nodes when requested by CLI/config or a
context-scope policy, but it does not rewrite the source-map origin or report tree.
Diagnostics emitted before or during stripping continue to point at the initial decoded
stream and may reference stripped trivia ranges.

Scoped queries are `cem-ql` expressions evaluated only against `QueryContextScope`: the
current AST node, the active schema scope, allowed machine-state slots, and
policy-visible resources. CEM-ML template embedding uses AC-T-7's host-owned attribute
`{...}` spans, whole-expression attributes such as `select=` / `match=` / `test=`, and
explicit `$` expression nodes for content. When the template compiler extracts a query
substring from an attribute value or `$` expression node, it appends
`TransformKind::TemplateEmbedding` with both the host span and the query sub-span before
handing plain UTF-8 source to the cem-ql parser.

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
  policy: ScopePolicy

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

Machine-state slots are CEM data placeholders supplied by a call instance, caller, or
runtime adapter. They are unrelated to HTML `<slot>` distribution. If multiple
`MachineStateSlot` references use the same `StateKey` in the same effective scope, they
resolve to the same slot id and reuse the same data.

```
RenderBinding:
  visual_scope: ScopeId
  template_ref: TemplateRef
  input_dom_root: Option<AstNodeId>
  state_slots: Vec<StateSlotId>
  transform_rules: Vec<TemplateRule>

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

### 3.12 Async Engine And WASM API Contract

All public parser, loader, validator, and transform entry points are asynchronous in Rust
and WASM. The crate does not expose synchronous parser APIs.

```
CemMlEngine:
  parse(input: AsyncByteSource, config: ParserConfig) -> Future<Result<ParseOutput, CemError>>
  validate(input: AsyncByteSource, config: ParserConfig) -> Future<Result<ReportAst, CemError>>
  transform(input: AsyncByteSource, target: OutputTarget, config: ParserConfig) -> Future<Result<TransformOutput, CemError>>

WasmApi:
  parse(input: ReadableStream<Uint8Array | string> | Uint8Array | string, config) -> Promise<ParseOutput>
  validate(input: ReadableStream<Uint8Array | string> | Uint8Array | string, config) -> Promise<ReportAst>
  transform(input: ReadableStream<Uint8Array | string> | Uint8Array | string, target, config) -> Promise<TransformOutput>
```

Observability is a parallel surface on the same calls, not a blocking alternative:

```
ParserConfig:
  observers: Vec<EngineObserver>

EngineObserver:
  onParseEvent(event: ReportEvent)
  onValidate(event: ReportEvent)
  onTransform(event: ReportEvent)
```

Implementations MAY expose those observers as callbacks or async streams, but the
payload categories and names are stable per AC-O-1. The canonical storage form remains
the `ReportAst`; observers are projections emitted while the report tree is being built.

Owned byte and string inputs are wrapped as already-ready async source adapters. File
inputs are opened and read through async runtime adapters. Tier A decodes and tokenizes
chunks as they arrive; tokenizer accumulation is token-local and released after token
emission. Editor-style incremental reparse, chunk graph reuse, and resumable partial
parses are deferred to Tier B.

### 3.13 Gate Status

Gate state is recorded here per `cem-ml-ac.md` AC-G-3. Current status is intentionally
conservative because Tier A has not closed yet.

| Gate   | Status  | Reason |
|--------|---------|--------|
| G-EXT  | blocked | Tier A async, diagnostic, policy, source-map, and untrusted-input prerequisites are not yet verified. |
| G-PLUG | blocked | Depends on G-EXT plus worker-pool/queue and plugin architecture contracts. |
| G-NVDL | blocked | Depends on G-EXT and G-PLUG plus schema-version/cache prerequisites. |
| G-MUT  | blocked | Depends on G-PLUG plus async queue/cache/runtime mutation contracts. |
| G-HYD  | blocked | Depends on G-MUT and G-EXT plus render-event trace and first-paint budget verification. |

---

## 4. Rust Module Map

```
cem_ml/src/
  lib.rs
  source/
    mod.rs            AsyncByteSource, SourceId, ByteRange
    decode.rs         EncodingDecoder, DecodedChunk, Encoding
    line_index.rs     LineIndex - byte offset -> (line, col) projection
  tokenizer/
    mod.rs            RawToken, SchemaTokenizer trait
    cem.rs            Canonical curly CEM-ML tokenizer profile
    html.rs           Custom WHATWG-state HTML tokenizer profile
    xml.rs            XML 1.0 tokenizer profile
  events/
    mod.rs            NormalizedEvent, EventNormalizer
  schema/
    mod.rs            SchemaMachine, SchemaFrame, FramePhase, AttributeState, ContentState, SchemaState
    compiler/         CEM-native schema source -> CompiledSchema, plus release-artifact emission (§3.4.2)
      mod.rs          SchemaCompiler entry point, CompilerOptions, emit_all()
      output.rs       CompilerOutput, EmittedArtifact, ArtifactKind, PublicationManifest
      emitter.rs      SchemaEmitter trait, EmissionCursor, deterministic encoder helpers
      rng_xml.rs      RELAX NG XML mirror emitter (AC-S-2)
      rng_compact.rs  RELAX NG compact-syntax mirror emitter (AC-S-2)
      ts_dts.rs       TypeScript .d.ts emitter (AC-S-3, AC-S-6)
      rust_hdr.rs     Rust .rs emitter (AC-S-4) — gated by CompilerOptions.emit_rust
      uri_publish.rs  Manifest writer, hash sidecars, URI resolution helpers (AC-S-5)
      byte_stability.rs Deterministic writer, BTreeMap helpers, hash sink
      error.rs        EmitError variants
    ir.rs             CompiledSchema, StructuralSchemaIr, SemanticRule, open-content policy
    policy.rs         ScopePolicy, ErrorBoundaryKind, ErrorBoundaryPolicy, DiagnosticVisibility, parent overrides
    dfa.rs            Tier A structural validator backend
    derivative.rs     RELAX NG derivative computation
    namespace.rs      NsContext, NamespaceBinding, QName, ExpandedName, NameResolution
    vocab.rs          CEM vocabulary constants generated from compiled CEM-native schema
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
    trivia.rs         comment/whitespace stripping transform preserving reports/source maps
  interpreter/
    mod.rs            CemInterpreter trait, TransformContext, TransformOutput, RenderedOutput
    template.rs       Rust CEM template renderer and scoped query evaluator
    transform.rs      CEM semantic HTML -> custom-element template plans
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
    mod.rs            async CemMlEngine trait (I/O-independent)
    fake.rs           FakeEngine for CLI feature tests
  command/
    mod.rs            I/O-independent command orchestration and top-level ScopePolicy config
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

- The acceptance criteria define required behavior, tier scope, and open decisions.
- The primary design defines functional layer boundaries and reasoning for those AC
  items.
- This file defines implementation shapes for behavior already present in the AC and
  primary design.
- Deferred Tier B/C interfaces may be stubbed in Tier A only when the AC or primary
  design calls for stable interfaces.

---

## 7. Appendix: Implementation AC Follow-Up

Review date: 2026-05-12. The table below is the pre-alignment implementation review
that drove the current AC update. It is retained for provenance. The authoritative
follow-up list is the "Current Implementation Follow-Up" table after it.

| ID             | AC reference                  | Implementation finding                                                                                                                                                                                                                                | Missing or conflicting contract                                                                          |
|----------------|-------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------|
| IMPL-ALIGN-001 | Preamble, §6                  | The ownership rules still say acceptance criteria must be derived from the primary design, which conflicts with the AC-primary basis of this review.                                                                                                  | Update ownership rules after source-of-truth governance is decided.                                      |
| IMPL-ALIGN-002 | AC-P-1                        | `InputDomNode`, `CemNode`, WHATWG DOM projection, and UI DOM plan are separate shapes; no shared public HTML/XML DOM API is specified.                                                                                                                | Define the common node/attribute/text API required by AC-P-1 or explicitly bind it to `InputDomNode`.    |
| IMPL-ALIGN-003 | AC-P-2, AC-O-1                | `CemMlEngine::parse` returns a future for `ParseOutput`; there is no parse-event stream or named `onParseEvent`, `onValidate`, `onTransform` callback/stream API.                                                                                     | Add streaming output types and observability hooks, or revise the AC.                                    |
| IMPL-ALIGN-004 | AC-P-3                        | `Diagnostic` includes `uri`, projected `line`/`column`, and `source_map`, but no mandatory top-level `byteOffset` field.                                                                                                                              | Add `byteOffset` as a required diagnostic/report projection, derived from the selected source-map frame. |
| IMPL-ALIGN-005 | AC-P-4                        | `SchemaFrame` and `ScopePolicy` carry `schema_id`, `language_id`, namespace context, and scope ids; they do not expose the AC's exact scope identity `{ schemaUri, contentType, namespaceUri }`.                                                      | Add explicit scope identity fields or a projection that is stable across reports and APIs.               |
| IMPL-ALIGN-006 | AC-P-8, AC-S-1 through AC-S-6 | `CompiledSchema` models CEM-native source and structural IR, but there are no compiler output shapes for byte-stable XML schema mirrors, TypeScript `.d.ts`, Rust `.rs`, stable URI publication, or TS structural/branded strategy.                   | Extend `schema::compiler` outputs and module map for schema artifacts and type-header emitters.          |
| IMPL-ALIGN-007 | AC-V-2, AC-V-3                | `CemNativeSchemaSource` has `version`, but no semver compatibility algorithm, major-version failure rule, or minor-version unknown-content warning policy.                                                                                            | Add version parsing/comparison and default severity mapping for schema compatibility.                    |
| IMPL-ALIGN-008 | AC-I-1, AC-I-3                | `CemInterpreter::transform` is batch-style over `CemDocument`; no DOM `apply(transform)` API, DOM-fragment transform input, or exclusive subtree mutation ownership API exists.                                                                       | Add interpreter-owned DOM state-machine interfaces before claiming AC-I-* coverage.                      |
| IMPL-ALIGN-009 | AC-M-1 through AC-M-14        | No implementation shapes exist for `appendChildAsync`, `setAttributeAsync`, `setInnerHTMLAsync`, mutation queues, `MutationObserver` batching, `AbortSignal`, rollback, `ScopeViolationError`, `flushAsync`, or transaction policy.                   | Add a `dom_mutation`/runtime module and data contracts, or retier/remove AC-M-* from Tier A.             |
| IMPL-ALIGN-010 | AC-T-1, AC-T-4                | `TransformPlan` and `CemTemplateModule` represent schema-owned template plans, but there is no transform input abstraction for URI, stream, or DOM fragment and no concrete XSLT subset model.                                                        | Add transform-source types and supported XSLT-equivalence mapping.                                       |
| IMPL-ALIGN-011 | AC-PL-1 through AC-PL-20      | The module map has no `plugin` module and no descriptor, observe/mutate mode, chain inheritance, install/uninstall lifecycle, priority, V3/CEM source-map stitching, budget, or sandbox shapes.                                                       | Add plugin contracts or mark plugin ACs as external to this implementation design.                       |
| IMPL-ALIGN-012 | AC-A-2 through AC-A-7, AC-O-2 | The async engine uses futures, but there are no parent-node processor promises, depth-first scheduler, per-scope thread pool, bounded queue, external-resource event-stream queue, end-to-end `AbortSignal`, or deterministic scheduling trace types. | Add scheduler/cancellation/resource queue types with defaults and trace projection.                      |
| IMPL-ALIGN-013 | AC-R-1 through AC-R-3         | `TemplateRef::DceTagName` and `template-registry` projection do not model scoped DCE custom-element registrations, inherited lookup, or collision diagnostics.                                                                                        | Add scoped registry structs and lookup/collision algorithms.                                             |
| IMPL-ALIGN-014 | AC-N-1, AC-N-2                | Limits constants exist, but no benchmark harness contract, 150 ms budget, CI tolerance band, or memory proof tying retained structures to the AC's depth-bound requirement exists.                                                                    | Add benchmark and memory-budget contracts.                                                               |
| IMPL-ALIGN-015 | AC-C-1 through AC-C-3         | WASM APIs are sketched, but there is no browser latest-two/Node >= 22 compatibility policy or publishability gate for native crate, WASM, and npm wrappers.                                                                                           | Add distribution and runtime compatibility contracts.                                                    |

### 7.1 Current Implementation Follow-Up

[`cem-ml-ac.md`](cem-ml-ac.md) is now the primary decision driver. These
implementation shapes remain open:

| ID | AC reference | Implementation follow-up |
|----|--------------|--------------------------|
| IMPL-FOLLOW-001 | AC-S-2 through AC-S-6 | **Design landed (§3.4.2); all open questions resolved 2026-05-19 (OQ-SC-3, OQ-SC-5, OQ-SC-6, OQ-SC-7, OQ-SC-8 — see `cem-ml-stack-design.md` §13.2.9).** Compiler output module, emitters, byte-stability rules, URI publication, and verification harness are fully specified. Emitter implementation is unblocked; [`cem-ml-schema-compiler-open-questions.md`](cem-ml-schema-compiler-open-questions.md) is kept as the decision archive. |
| IMPL-FOLLOW-001A | AC-F-9, AC-P-1, AC-P-8 | Add concrete CEM-native tokenizer lowering tests for `{name @attributes \| content...}`, `$` expression nodes, anonymous scopes, comments, rich-content enclosures, and rejection of bare `{...}` text interpolation. |
| IMPL-FOLLOW-001B | AC-F-8 | Add `@doc` document-format identity parsing, SemVer constraint resolution, required top-level directive rejection, and AC-F-V-6 diagnostics before schema loading. |
| IMPL-FOLLOW-002 | AC-V-2, AC-V-3, AC-V-9..AC-V-13 | Schema-version structs are sketched in §3.4; add parser/comparison implementation and compatibility severity tests. |
| IMPL-FOLLOW-003 | AC-P-3, AC-O-1 | Event observer and `byte_offset` shapes are sketched in §2.5 / §3.12; add concrete Rust/WASM APIs and report projections. |
| IMPL-FOLLOW-004 | AC-F-2 | Add parser and schema-frame lowering for inline schema declarations and mid-document schema switch forms from AC-F-2 / design §13.1. |
| IMPL-FOLLOW-005 | AC-I-1, AC-M-1 through AC-M-14 | Do not add a Tier A public DOM mutation module; design it as a Tier C runtime module later. |
| IMPL-FOLLOW-006 | AC-PL-1 through AC-PL-20 | Add plugin module, descriptors, chain execution, source-map stitching, lifecycle, budgets, and sandbox types. |
| IMPL-FOLLOW-007 | AC-A-4 through AC-A-7, AC-O-2 | Add scheduler, bounded queue, cancellation, external-resource queue, and deterministic trace types. |
| IMPL-FOLLOW-008 | AC-R-1 through AC-R-3 | Add scoped template/registry lookup structs and collision diagnostics. |
| IMPL-FOLLOW-009 | AC-N-1 through AC-N-3 | Add benchmark harness contracts, memory limit tests, and CI tolerance configuration. |
| IMPL-FOLLOW-010 | AC-C-1 through AC-C-3 | Add browser/Node/WASM compatibility gates and package publication checks. |

---

*End of implementation design document.*
