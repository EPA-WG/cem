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
  origin_scope: ScopeId
  boundary_scope: ScopeId           error-boundary scope that handled the diagnostic
```

Diagnostics originate in the scope where the parser, validator, transform, or runtime
detected the error. They bubble to the nearest error-boundary scope. `Fatal`,
`Error`, and `Warning` handling is decided by that boundary scope's effective
`ScopePolicy`.

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

AsyncByteSource:
  next_chunk() -> Future<Option<ByteChunk>>
  source_hint() -> SourceHint

ByteChunk:
  bytes: Vec<u8> | String
  byte_range: Option<ByteRange>     known after source assembly or stream offset tracking

DecodeConfig:
  default_encoding: Option<Encoding> // caller/server/child-source encoding, if known
  content_type: ContentType

DecodeResult:
  chunks: Vec<DecodedChunk>
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
- Preserve raw byte slices for zero-copy diagnostic snippets.
- Inspect the first bytes of each `ByteSource` before tokenization. A supported BOM
  selects the encoding for that source, is skipped from `DecodedChunk.scalars`, and
  suppresses later encoding overrides for that source.
- If no BOM is present, use `DecodeConfig.default_encoding`. If it is absent, use UTF-8.
- Inline embedded handoffs consume the owner's decoded Unicode stream and do not run BOM
  detection. External or separately loaded resources receive their own `ByteSource` and
  `DecodeConfig`, then apply the same initiation rule.

Tier A exposes asynchronous source APIs only. In-memory byte buffers, strings, file-path
input, and WASM `ReadableStream<Uint8Array | string>` inputs are normalized through
`AsyncByteSource`. The first implementation may collect a complete `ByteSource` before
tokenization; parse-as-chunks-arrive incremental delivery is Tier B. No synchronous
public parser or WASM entry point is defined.

### 3.2 Layer 2: SchemaTokenizer (`cem_ml::tokenizer`)

The tokenizer is mode-aware and schema-guided. The HTML profile is a custom
WHATWG-state tokenizer, not a wrapper around an external tokenizer crate. The custom
implementation is required so every token
and token sub-span can preserve source-map stacks through decoded streams and nested
embedded handoff layers. It extracts source-spanned tokens and switches lexical states;
it does not construct either the initial HTML parser DOM or the WHATWG implementation
DOM.

```
RawToken:
  kind: HtmlToken | XmlToken
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
```

The `StartTag` attributes carry both the name and value ranges so the event normalizer
can emit per-attribute byte offsets into the source-map stack. `RawToken.source_map`
maps the token's local `source_id` and `byte_range` back through any decoded stream or
embedded handoff boundary that produced it. Attribute sub-ranges are interpreted within
that same token source-map context.

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
  version: String
  source_id: SourceId
  source_map: SourceMapStack

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
diagnosed, transformed, or stripped. Diagnostics and source maps always refer to the
initial decoded stream, so stripped trivia still counts for byte offsets, line/column
projection, snippets, and report events.

Diagnostic propagation walks from the origin frame toward its ancestors until it reaches
the nearest scope whose `error_boundary` is `SchemaDeclared` or `ContextRoot`. If no
schema-declared boundary is found before the context root, the context root handles the
diagnostic. The boundary scope applies its effective policy and becomes the diagnostic's
`boundary_scope`.

```
SchemaFrame:
  scope_id: ScopeId
  schema_id: SchemaId
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
  Bubble to nearest SchemaDeclared or ContextRoot error boundary.
  Evaluate boundary frame's effective ScopePolicy.
  Hide, report, push ErrorSubtree recovery frame, abort boundary scope, or abort full
  parse per policy.

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
Comments, whitespace, and processing instructions are part of the initial source stream
for reporting. Diagnostics can reference trivia byte ranges even when the active output
transform later removes those nodes.

### 3.6 CLI Projection Keys

Stack layers own data artifacts. The CLI owns projection selection, output targets, and
default stream behavior. Proposed projection layer keys:

| Key                | Stack owner                       | Projection meaning                                                                                       |
|--------------------|-----------------------------------|----------------------------------------------------------------------------------------------------------|
| `source`           | `source::ByteSource`              | Source metadata, URI, byte length, and source id; raw bytes are not emitted unless explicitly requested. |
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
  query_language: ScopedQueryLanguage
  source: SourceMapStack

CemTemplate:
  template_id: TemplateId
  match_query: ScopedQuery
  priority: i32
  body: Vec<TemplateOp>
  params: Vec<TemplateParam>
  source: SourceMapStack

ScopedQuery:
  expression: String               syntax TBD
  allowed_context: QueryContextScope
  source: SourceMapStack

ScopedQueryLanguage:
  CemScopedQuery                   XPath-like semantics, CEM scope and policy bounded

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

Scoped queries are XPath-like in capability but are evaluated only against
`QueryContextScope`: the current AST node, the active schema scope, allowed machine-state
slots, and policy-visible resources. The exact CEM template syntax and scoped query
syntax are TBD; the implementation contract is the scoped execution model.

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

In-memory inputs are wrapped as already-ready async sources. File inputs are opened and
read through async runtime adapters. Tier A may await source assembly before tokenization;
that is still an async API contract. Incremental parsing while chunks arrive is deferred
to Tier B.

---

## 4. Rust Module Map

```
cem_ml/src/
  lib.rs
  source/
    mod.rs            AsyncByteSource and assembled ByteSource traits, SourceId, ByteRange
    decode.rs         EncodingDecoder, DecodedChunk, Encoding
    line_index.rs     LineIndex - byte offset -> (line, col) projection
  tokenizer/
    mod.rs            RawToken, SchemaTokenizer trait
    html.rs           Custom WHATWG-state HTML tokenizer profile
    xml.rs            XML 1.0 tokenizer profile
  events/
    mod.rs            NormalizedEvent, EventNormalizer
  schema/
    mod.rs            SchemaMachine, SchemaFrame, FramePhase, AttributeState, ContentState, SchemaState
    compiler.rs       CEM-native schema source -> CompiledSchema
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

- The primary design defines functional behavior, tier scope, and unresolved decisions.
- This file defines implementation shapes for the behavior already present in the
  primary design.
- Acceptance criteria must be derived from resolved design decisions, not from
  speculative implementation details.
- Deferred Tier B/C interfaces may be stubbed in Tier A only when the primary design
  calls for stable seams.

---

*End of implementation design document.*
