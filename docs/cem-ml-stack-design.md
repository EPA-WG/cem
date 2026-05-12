# `cem-ml` Stack Design

**Status:** Draft high-level design. Implementation contracts are split into
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md).  
**Primary source:** [`parsing-algorithms-research.md`](../parsing-algorithms-research.md)  
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

`parsing-algorithms-research.md` remains the primary architectural source. This
document is the active high-level design contract derived from that research: it defines
current behavior, tiers, module responsibilities, and layer boundaries for `cem-ml` work
until the design is revised. Concrete interface sketches, struct shapes, projection keys,
and file-level implementation ownership live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md). Other workspace documents
(acceptance criteria, CLI plan, todo) are non-authoritative projections and planning
aids; they may verify this design, but they do not introduce or override requirements.

### 1.1 Acceptance Criteria Derivation Policy

Acceptance criteria must be derived from resolved design decisions and layer contracts
in this document and its implementation companion. They must not introduce new
requirements, syntax, APIs, tiers, or behavior that are absent from the design.

While this design is still draft, acceptance criteria that conflict with this document
are stale and must be rewritten or ignored for implementation planning. After the
design is complete, independent acceptance criteria should be rewritten from the
completed design or phased out in favor of generated/checklist-style verification
references.

---

## 2. Domain Context

CEM semantic HTML is standard HTML5 plus schema-qualified CEM attributes. These
attributes are transformation annotations in the namespace associated with the active
schema, not HTML `data-*` metadata and not replacements for native HTML element meaning.
The `cem:` prefix below is illustrative; the active schema owns the concrete namespace
binding:

```html

<main cem:screen="login" aria-labelledby="login-title">
    <h1 id="login-title">Sign in</h1>
    <form cem:form="sign-in" method="post" action="/session">
        <label for="email">Email</label>
        <input id="email" name="email" type="email" required>
        <button type="submit" cem:action="primary">Sign in</button>
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

![Architectural blueprint illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-architectural-blueprint.png)

*Architectural blueprint.*

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
For Tier A, all public Rust and WASM entry points are asynchronous. The implementation
may buffer a complete input before tokenization in the first implementation phase, but
callers interact with futures/streams only; no synchronous parser API is exposed. The
encoder, compressor, and broadcast/cache paths are absent.

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
tokenizer. The tokenizer preserves source bytes and tokens, the schema machine validates
structure, and the input DOM records URL-bearing attributes and inline content with
source maps. URL-bearing fields are then resolved by the active transformation policy
against the owner context's base URL, module or import map, and substitution rules.
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
- Line and column are projections from a per-source line index; they are derived for
  reports and editor integration, not stored as node identity.
- Bit-level ranges are reserved for deferred binary/compressed content and are not
  populated in Tier A.

The stack records the processing chain: tokenizer, event normalizer, schema validation,
CEM AST builder, handoff boundaries, implementation transforms, and future binary
encoding. This lets tooling resolve from generated custom-element output back to the raw
HTML token or embedded resource that produced it.

Detailed `ByteRange`, `SourceMapStack`, `SourceMapFrame`, `TransformKind`, diagnostic,
and traversal shapes live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#2-shared-source-map-and-diagnostic-contracts).

---

## 5. Layer 1 - ByteSource And EncodingDecoder (`cem_ml::source`)

### Purpose

Owns raw bytes, chunking, and encoding detection. Preserves absolute byte offsets through
all subsequent layers.

### Functional Design

Layer 1 presents every input as an immutable source with a stable `SourceId`, a full
source byte range, decoded scalar spans, and cached line-index metadata. It is modeled on
LLVM `MemoryBuffer`: the implementation may keep a padded internal allocation with a
sentinel byte so lexers can scan safely, but offsets and public ranges always address the
real source bytes.

Rules from the research:

- Keep absolute `u64` byte offsets for every token and event.
- Keep decoded scalar spans alongside scalars for Unicode-aware validation.
- Preserve raw byte slices for zero-copy diagnostic snippets.
- Decode each byte source into a Unicode scalar stream before tokenization. Tokenizers
  consume decoded Unicode scalars, never raw encoding-specific code units.
- Treat BOM detection as byte-stream initiation. If the first bytes of a `ByteSource`
  are a supported BOM, the BOM determines the source encoding, the BOM bytes are skipped
  from the decoded scalar stream, and later encoding overrides for that source are
  ignored.
- If no BOM is present, use the explicit/default encoding parameter supplied with the
  source. Callers may derive this value from server `Content-Type` headers or other
  transport metadata. If no encoding is supplied, default to UTF-8.
- Inline embedded contexts receive source-mapped decoded streams from their owner and do
  not perform BOM detection. External or separately loaded resources are new byte-source
  initiations and apply the same BOM/default-encoding precedence independently.

Encoding resource bounds remain an open review item in §18.3.3.

### Tier A Scope

Tier A accepts in-memory byte buffers, string input, file-path input, and async byte or
string streams through asynchronous APIs. The implementation may choose to collect a
complete source before tokenization in Tier A; incremental parsing while chunks arrive is
deferred to Tier B. Tier A must still preserve absolute offsets so the streaming parser
can reuse the same source-map model.

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

For HTML, Tier A uses a custom WHATWG-state tokenizer and emits source-spanned tokens.
The custom tokenizer is selected over wrapping an existing Rust HTML5 tokenizer because
CEM requires exact source-map preservation across nested embedded contexts, decoded
handoff streams, token envelopes, attribute names, attribute values, text runs, and raw
text return boundaries. It does not construct either the source-preserving initial HTML
parser DOM or the WHATWG implementation DOM. The schema-defined token hierarchy is
reconstructed later by the input DOM/AST builder, and WHATWG DOM compliance is applied
as a content-type transform.

The schema can select valid tokenizer contexts and embedded-content boundaries, but it
does not rewrite WHATWG lexical behavior or make the tokenizer the semantic source of
truth. XML follows the same layer contract with an XML 1.0 profile so Layers 3 and above
can consume a format-agnostic event stream.

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
current context root. The boundary scope's effective policy decides whether to hide,
report, recover, abort the boundary scope, or abort the full parse. The document scope
is the topmost error boundary context; engine defaults, CLI parameters, or config seed
its policy before parsing, and descendant scopes inherit or redefine that policy within
parent override bounds. The effective policy maps stable diagnostic codes to severity
and can upgrade a recoverable warning or error to fatal/fail-fast behavior.

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
phase. Attribute validation and child-content validation use distinct trackers:
`AttributeState` stores the active effective attributes and required attributes that remain;
`ContentState` stores the residual or DFA state, diagnostic-relevant seen children in
emit order, and required children that remain. Attribute multiplicity is normalized to
0..1 per expanded name by last-writer-wins override; multi-valued attribute semantics
are value-shape checks. Child multiplicity and ordering are encoded in the residual or
DFA state, with
`required_remaining_children` kept as a diagnostic mirror for close-time messages.

Constraint checks have fixed trigger events:

| Constraint                 | Trigger                              | Frame phase |
|----------------------------|--------------------------------------|-------------|
| Duplicate attribute        | `Name`                               | Attribute; later value overrides earlier value |
| Unknown attribute          | `Name`                               | Attribute   |
| Bad attribute value        | attribute `Value`                    | Attribute   |
| Required attribute missing | `Separator { kind: ElementBoundary }` | Attribute -> Content |
| Unexpected child element   | `OpenScope`                          | Content     |
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

This is intentionally similar to repeated `var` declarations in JavaScript: the binding
name can be declared more than once in the same scope, the later declaration becomes the
active binding for subsequent uses, and earlier uses keep the binding that was active at
their source position.

Named namespaces follow the same override rule as the default namespace. The empty
namespace name represents the default namespace; it is not a special global singleton.
The parser can therefore support scoped default namespaces for CEM tags, HTML-compatible
unqualified output, and future XSLT/template-driven transformations without changing the
AST identity model.

The namespace defined as the attribute in node does not leave the ability to define multile namespacs under
same name due to unique attribute name. While it would be possible to define a namespace
attribute that lists multiple schemas, the ability to define the namespace via tag TBD.

Namespace-resolution implementation contracts are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#341-namespace-context-contracts).

### Diagnostics And Reports

Diagnostics use byte offsets as ground truth and derive line/column positions for human
outputs. The canonical report model is an AST-associated report tree, not a flat list.
Each parser, schema, handoff, transform, validation, or runtime event attaches to the
current AST node when one exists, the current source module state, the event-time
source-map stack, the originating scope, the error-boundary scope that handled it, and a
monotonic event sequence number.

The report tree can be projected to CEM-native, XML, JSON, Markdown, text, HTML, or any
other supported structured format. Text and HTML reports are reference convenience
renderers over the report tree, not canonical report storage formats.

Diagnostics and source maps always address the initial decoded source stream. Comments
and whitespace count when deriving byte offsets, line/column positions, and snippets,
even if a later transform removes those nodes. Diagnostics may refer to comments,
whitespace, or processing instructions that no longer survive in a transformed output.

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

The canonical serialization of a transformed AST tree is CEM-ML format. Canonical
snapshots, hashes, fixture round trips, and cache identities use this CEM-ML tree rather
than rendered HTML or another target projection. The CEM-ML serialization is schema-owned
and follows the same transform plan that produced the tree.

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
machine-state slots, and policy-visible resources only. The selector/query language must
provide XPath-like expressive power for the supported CEM AST and data model, but access
outside the allowed context is rejected by construction. The exact CEM template syntax
and scoped query syntax remain TBD.

Transform interface shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#310-layer-9-implementationinterpreter-cem_mlinterpreter).

---

## 13. CEM Schema Language

The schema machine requires a machine-readable CEM schema. The research establishes
RELAX NG derivatives as the **validation algorithm**. The selected schema authoring
source is a CEM-native declarative format.

The CEM-native format is the source of truth for CEM vocabulary and schema behavior:
roles, states, token tiers, component names, namespace ownership, open-content policy,
structural content models, embedded handoff declarations, and schema-owned transform
hooks. Existing token tables or external schema artifacts may be supported as import
adapters, but they are not competing canonical authoring formats for CEM schemas.

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

---

## 14. Rust Module Map

The high-level module topology keeps I/O, parsing, validation, transformation, reporting,
and CLI orchestration separate. Exact structs, traits, and file-level implementation
ownership live in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#4-rust-module-map).

```
cem_ml/src/
  source/        byte sources, decoding, line-index projection
  tokenizer/     WHATWG HTML and XML tokenization profiles
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
- **Design partial** — one or more open concerns in §17–18 must be resolved before
  clean implementation can begin. Blocker references are noted.
- **Deferred Tier B/C** — explicitly out of Tier A scope; interface stubs may be
  defined now for stability.

| Component                                                   | Design status                                                                                                                                                                           |
|-------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| L1 ByteSource: in-memory buffer, string, file path          | Design partial — source ownership/resource bounds need decisions (§18.3.1, §18.3.3); exposed through async APIs only                                                                    |
| L1 ByteSource: async byte/string streams                    | Design ready — async Rust and WASM input APIs are primary; Tier A may buffer before tokenization, while incremental parse-as-chunks-arrive remains Tier B                               |
| L1 EncodingDecoder: UTF-8                                   | Design ready — UTF-8 is the fallback when no BOM or explicit/default encoding is present (§5, §18.3.2)                                                                                  |
| L1 EncodingDecoder: UTF-16, Latin-1, BOM detection          | Design ready — byte-stream initiation, BOM precedence, BOM skipping, and caller/default encoding precedence are resolved (§5, §18.3.2)                                                   |
| L1 Sentinel-byte ownership                                  | Design partial — Rust safety model for sentinel not resolved (§18.3.1)                                                                                                                  |
| L2 SchemaTokenizer: HTML WHATWG profile                     | Design ready — custom WHATWG-state tokenizer selected for exact source-map preservation across nested embedded contexts (§6)                                                            |
| L2 SchemaTokenizer: XML 1.0 profile                         | Design partial — DTD/external-resource ownership follows transform policy (§3.2, §6)                                                                                                     |
| L3 EventNormalizer                                          | Design partial — attribute-list close event and void elements remain unspecified (§18.4.1–2); trivia preservation, `QName` resolution, and `ModeSwitch` context creation are defined (§7–9) |
| L4 SchemaMachine: visibly pushdown frame stack              | Design ready — frame phases, attribute/content trackers, recovery invariant, and diagnostic propagation boundary are resolved (§8, §3.1)                                                  |
| L4 SchemaMachine: RELAX NG derivative engine                | Deferred Tier B — CEM structural schema has RELAX NG functional parity; switching from Tier A DFA to derivatives may break report compatibility (§8, §13)                                |
| L4 SchemaMachine: CEM vocabulary DFA                        | Design partial — limited Tier A DFA profile is selected and open-content defaults are resolved (§8, §13); DFA state table remains unspecified                                             |
| L5 HandoffStack: ownership and return-condition tracking    | Design ready — current context parser recognizes `ModeSwitch`; CEM framework maps entity content type and creates child context with decoded stream (§9)                                  |
| L5 Child parser: CSS (stub, diagnostic only)                | Design ready — container content type decodes before handoff; child receives a source-mapped decoded stream (§9)                                                                        |
| L5 Child parser: Script (raw text only)                     | Design ready — parser preserves raw text; warning/error/reject/allow behavior is defined by active scope/content-type policy (§3.1–3.2, §9)                                             |
| L6 InputDomAstBuilder: schema-defined initial DOM/AST       | Design ready — schema reconstructs token hierarchy; WHATWG DOM compliance is a downstream transformation over this initial DOM                                                          |
| L6 InterpreterAstBuilder: CEM annotation projection         | Design partial — CEM attributes are transform annotations on source nodes; transform conflict policy is schema-owned; CEM comment/CDATA syntax remains TBD (§10)                         |
| L6 Reference slots: id/for/aria-*                           | Design ready — one-pass mutable slots are sufficient; unfilled slots warn on owning scope close unless scope policy overrides severity per error type (§10, §3.1)                         |
| L6 Source-map stacks: byte-range + transform chain          | Design partial — frame order, multi-range nodes, escape/entity decoding, and diagnostics-before-AST mapping unresolved (§18.2.1–3, §18.2.5)                                             |
| L6 Source-map stacks: bit-level ranges                      | Deferred Tier B — reserve representation only after source-map frame model is fixed (§18.2.1–2); no serialized binary frame ids in Tier A (§11)                                         |
| L7 BinaryAstEncoder                                         | Deferred Tier B — Tier A does not freeze serialized binary ids; canonical identity, ordering, and future id policy are scoped in §11                                                    |
| L8 ChunkCompressor                                          | Deferred Tier B — compression profiles are research-backed; canonical chunk identity, ordering, and dependency slots are scoped in §11                                                  |
| ContentTypeTransformPipeline: WHATWG HTML DOM               | Design ready — schema-driven initial HTML parser DOM is transformed into WHATWG implementation DOM updates                                                                              |
| L9 ImplementationInterpreter: schema-driven transform rules | Design ready — schema owns transform layers; namespace-qualified CEM identity resolves source collisions; canonical serialization and HTML `data-*` ownership are defined in §8 and §12 |
| L9 ImplementationInterpreter: transform execution backend   | Design partial — Rust CEM template renderer is selected; exact CEM template syntax and scoped query syntax remain TBD (§12)                                                             |
| Visual content and machine state data                       | Design partial — uniform AST role model is defined; live hydration, browser adapters, and DOM patch identity are subject to a separate design phase TBD (§12)                           |
| LineIndex: byte-offset → line/col projection                | Design partial — column-unit model, newline normalization, tabs, replacement chars, and UTF-16/scalar projections unspecified (§18.2.4)                                                 |
| Diagnostics and reports                                     | Design partial — source-map ownership and diagnostics-before-AST mapping unresolved (§18.2.5)                                                                                           |
| CLI output projections and fixture round-trip reports       | Design ready — CLI owns projection targets and side outputs; stack layers own projected artifacts                                                                                       |
| Resource and security limits                                | Design partial — byte/decode bounds remain unresolved (§18.3.3); XML external-resource limits follow context-scope policy and content-type transforms (§3.1–3.2, §6)                     |
| Incremental/editor parsing                                  | Deferred Tier B — caller-provided diffs map through source maps to changed scopes, with enclosing-scope rescan fallback                                                                 |
| Scope-close reference validation (unfilled slots)           | Design ready — unresolved references emit warnings on owning scope close by default; context-scope policy can override per diagnostic type (§10, §3.1)                                   |
| Per-scope error boundaries                                  | Design ready — each context scope owns error handling and policy; inner scopes may relax or hide own errors only within parent override bounds (§3.1–3.2)                               |
| Async mutation API (`*Async` DOM mutations)                 | Deferred Tier B/C — outside the primary parsing research; separate from the required async parse/load APIs                                                                              |

---

## 16. Algorithm Selection Summary

![Multi-format parser atlas illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-parser-atlas.png)

*Multi-format parser atlas.*

| Layer     | Problem                      | Algorithm                                                | Reason from research                                                                           |
|-----------|------------------------------|----------------------------------------------------------|------------------------------------------------------------------------------------------------|
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

## 17. Open Ambiguities

No open ambiguity entries remain in this section. Previously assigned ambiguity IDs are
omitted after resolution; related implementation concerns that still need details are
tracked in §18.

---

## 18. Critical Review Questions And Concerns

This section records unresolved issues found by reviewing this design against
`parsing-algorithms-research.md` as the primary source. These are not decisions. They
are follow-up questions and concerns to resolve before implementation. Other workspace
documents may provide terminology, but they should not decide the answers here.

### 18.2 Source-Map And Coordinate Model Gaps

**Concern 18.2.1 — Source-map frame order is internally inconsistent.**  
The implementation companion defines frames as "earliest context first", but the source
map examples list `CemAstBuilder` before `SchemaValidation`, `EventNormalizer`, and
`HtmlTokenizer`, which is latest-context first.

**Question:** Is `SourceMapStack.frames[0]` the original byte source frame or the current
AST/transformed frame? This must be fixed before traversal, compression deltas, and
generated-node inheritance are implemented.

**Answer A — `frames[0]` is the original byte-source frame; the current frame is `frames.last()`.**
This matches §2.2's literal "earliest context first" wording and the natural reading of a
"stack" appended to as transforms accrue: the bottom is the origin, the top is now. Pros:
(a) traversal back to the byte source is `frames[0]` — a stable index that does not move
when new transform frames are pushed; (b) inheritance for generated nodes is "copy
parent's stack, push a new top", which is `Vec::push`-shaped; (c) compression deltas
between sibling nodes share long common prefixes, so prefix-shared encoding is efficient.
Action: rewrite the §2.3 traversal examples in origin-first order so `frame[0]` is
`HtmlTokenizer` and the last frame is `CemAstBuilder`. Treat the current examples as the
bug, not the contract.

**Answer B — `frames[0]` is the current/topmost frame; the original byte-source frame is `frames.last()`.**
This matches the §2.3 examples literally and reads diagnostics top-down ("here is where
the node is now; here is what produced it"). Pros: (a) error rendering walks `frames[0..]`
in the order a human reads a stack trace; (b) `frame[0].byte_range` is the most-local
range, useful for snippet extraction without a length lookup. Cons: (a) every push must
shift existing frames or use a different data structure (`VecDeque::push_front` /
reverse-indexed slice); (b) the §2.2 phrase "earliest context first" must be rewritten to
"latest context first". Action: keep the examples, fix the prose, and document push as
"prepend" semantically (it can still be implemented as `Vec::push` with reversed read
order, but the contract must be explicit).

**Recommendation:** Adopt **Answer A**. It is the cheaper retrofit (prose stays, examples
get reordered), preserves O(1) origin lookup at `[0]`, makes "inherit + push" the natural
generated-node operation in §2.4, and matches how compiler source-map stacks are normally
built (lex → parse → lower, each appended). The §2.3 examples are the artifact to fix.

**Ambiguity (to be answered):** Regardless of order chosen, the contract must name two
indices explicitly — `origin_frame()` and `current_frame()` — so consumers never index
positionally. Open question: are these methods on `SourceMapStack`, or on a thin
`SourceMapView` returned from traversal?

**Concern 18.2.2 — A single `ByteRange` per frame is not enough for all research cases.**  
The research explicitly mentions merged nodes, split nodes, generated nodes,
transform-owned reference inlining, and source-map stacks through transformations. A
single `byte_range` cannot represent a text node produced from multiple source regions,
such as `a&amp;b`, or a node merged from adjacent text/event fragments.

**Question:** Should `SourceMapFrame` support one range, many ranges, generated sentinel
ranges, and transform-owned reference inlining? If not, where are escape decoding,
merge, split, and XML-compatibility entity mappings stored?

**Answer A — Promote `byte_range` to an enum `FrameSpan`.**
Replace `byte_range: ByteRange` with:

```
FrameSpan:
  Single(ByteRange)                  // common case: 1:1 source mapping
  Multi(Vec<ByteRange>)              // merged text nodes, joined fragments
  Generated { owner: ByteRange }     // transform-synthesized; carries nearest source range
  Inlined { reference: ByteRange,    // a reference site that pulled in another source
            target: SourceMapStack } //   target carries its own origin chain
```

The default constructor still takes one range (no migration of single-range call sites).
Pros: encodes all four cases in §2.2 without auxiliary side tables; `Generated` matches
§2.4's existing "owner range" notion; `Inlined` is the only shape that can faithfully
represent transform-owned reference resolution (e.g. `aria-labelledby` slot inlining)
without flattening the target's own provenance. Cons: pattern-matching cost on every
traversal; serialization size of the report grows.

**Answer B — Keep one `byte_range` and add typed sub-frames per concern.**
`SourceMapFrame` stays single-range. Instead, define explicit `TransformKind` variants
that carry the extra structure:

```
TransformKind::EscapeDecoded { decoded_to_source: Vec<(ScalarRange, ByteRange)> }
TransformKind::TextMerged    { parts: Vec<ByteRange> }
TransformKind::TextSplit     { source: ByteRange, slice: ByteRange }
TransformKind::Generated     { owner: ByteRange }
TransformKind::ReferenceInlined { ref_site: ByteRange, target_stack: SourceMapStack }
```

The frame's outer `byte_range` becomes the *summary* span (e.g. min start to max end of
the merged parts) for snippet rendering; the variant payload holds exact mapping. Pros:
common case stays cheap; consumers that only need snippet bounds touch one field; mapping
fidelity lives where the transform is named, not in a generic span container. Cons:
duplicates the "where am I" answer between the outer span and the variant payload; risk
of drift between the two.

**Answer C — Hybrid: `Single | Multi` on the frame; reference inlining as a separate frame.**
Allow only `Single` and `Multi` on `FrameSpan`. Reference inlining and external-resource
boundaries get their own dedicated frames (`TransformKind::ReferenceInlined`,
`TransformKind::ExternalResource`) whose own `source_id` switches to the target buffer —
the target's source map is then the natural continuation when traversal crosses the
frame, exactly as §2.3's CSS-in-`<style>` example already does. Pros: keeps the frame
shape uniform; reuses the existing handoff-style boundary pattern; no nested stacks
embedded in a frame. Cons: traversal of an inlined reference now requires walking out of
one stack and into another via the boundary frame's metadata.

**Recommendation:** Adopt **Answer C** as the primary contract, with `Multi(Vec<ByteRange>)`
covering escape decoding, text merge, and entity expansion. Reference inlining and
external-resource resolution are already boundary-shaped (`HandoffBoundary` is the
analogue), so reusing that pattern keeps `SourceMapFrame` shape uniform. Generated nodes
keep §2.4's "nearest owning range" rule — they are expressible as
`Single(owner_range)` plus `TransformKind::Implementation`.

**Ambiguity (to be answered):**
- For `Multi`, is the order of ranges source-order or emit-order? (For `a&amp;b`, source
  order is `[a-range, &amp;-range, b-range]`; emit order matches.) Pick source-order;
  document explicitly.
- Does a per-scalar mapping live on the frame (heavy) or on the `DecodedChunk` it
  produced (deferred lookup)? See 18.2.3.

**Concern 18.2.3 — Entity and escape decoding needs source-map ownership.**  
HTML character references, XML entity references, CSS escapes, JSON string escapes, and
CSV quoted escapes all transform raw bytes into logical scalar values. The current
`DecodedChunk` model maps scalars to byte spans, but later token/event layers do not
state how escape-produced scalars preserve their original source.

**Question:** Does each language tokenizer emit per-scalar source ranges after escape
processing, or does it append a transform frame that maps decoded values back to raw
bytes?

**Answer A — Per-scalar source ranges on `DecodedChunk` (no extra frame).**
Layer 1's `DecodedChunk { scalars: [(char, ByteRange)] }` already pairs each decoded
scalar with its origin span. Extend this to escape-producing tokenizers (HTML char refs,
XML entity refs, CSS escapes, JSON `\uXXXX`, CSV doubled-quote): the decoded scalar's
`ByteRange` is the entire raw source span that produced it (e.g. `&amp;` → one scalar
`&` with `ByteRange` covering all 5 source bytes). Pros: zero new frame types; matches
the existing layer-1 contract; per-scalar precision is available to everyone downstream
without any decoding-aware traversal logic. Cons: `Vec<(char, ByteRange)>` is heavy for
ASCII-only spans; needs a memory-efficient encoding (e.g. run-length `1:1` segments plus
sparse "decoded" entries).

**Answer B — Append a `TransformKind::EscapeDecoded` frame per escape.**
Tokenizers leave the decoded text on the token; `EventNormalizer` (or the tokenizer
itself) adds an `EscapeDecoded` frame to the resulting `Value` event's source map. The
frame carries `Vec<(decoded_offset, source_range)>` for every decoded escape inside the
value. Pros: cost is paid only when escapes occur; raw 1:1 spans need no special
encoding. Cons: every consumer that wants a per-scalar position must walk frames; two
adjacent escapes in one value still need a vector inside the frame.

**Answer C — Hybrid (recommended).**
Layer 1 emits per-scalar ranges only for byte→scalar decoding (UTF-8/UTF-16/Latin-1).
Higher-level escape processing (HTML/XML entities, CSS/JSON/CSV escapes) is owned by the
tokenizer that recognizes the escape, and the tokenizer attaches an `EscapeDecoded`
sub-mapping to the *token* (not a separate frame): `RawToken.escape_map: Option<Vec<(char_index, ByteRange)>>`.
Empty/None means 1:1. Reasoning: the frame stack records *layer transitions*, while
escape decoding is intra-layer detail of the tokenizer. Source-map traversal stays
shallow; per-scalar precision is available when the consumer asks for it.

**Recommendation:** Adopt **Answer C**. It keeps frame depth bounded by layer count
(predictable for compression deltas in 18.2.1) while still letting Tier B reports
project per-scalar offsets when needed.

**Ambiguity (to be answered):** For multi-scalar entities (e.g. `&NotEqualTilde;` →
two-scalar grapheme), does the escape map record one entry per output scalar or one
entry per input source span? Default: one entry per output scalar, both pointing at the
same source span — round-tripping the source span is the dominant query.

**Concern 18.2.4 — Line/column projection is underspecified.**  
The design says line/column are derived from byte offsets, but different consumers need
different column units: Unicode scalar index, UTF-16 code units, display columns, or
language-specific positions.

**Question:** Which coordinate projections are required in Tier A reports, and how are
CRLF, isolated CR, tabs, multi-byte UTF-8, replacement characters, and HTML preprocessing
handled?

**Answer A — Tier A ships exactly one projection: `(line, column_utf8_bytes)`.**
Lines are 1-based and counted at decoded-scalar level after WHATWG HTML preprocessing
(`CR`, `CRLF`, and isolated `LF` all collapse to `LF` for line counting; the original
byte offset still points at the raw `CR` or `CRLF` start). Columns are 1-based byte
counts within the line, not scalar counts. Tabs count as one column. Multi-byte UTF-8
sequences count as their byte length (matches `git`, `clang`, `rustc`, most editors with
"raw column" mode). Replacement characters (U+FFFD) introduced by Layer 1 are treated as
ordinary scalars with whatever byte length their producing source had. Pros: one
projection, easy to test, matches what most CLIs print. Cons: editors that report
columns in UTF-16 code units (LSP) must convert.

**Answer B — Tier A ships two projections: byte-column and UTF-16 code units.**
Add `column_utf16: u32` alongside `column_utf8_bytes` for direct LSP consumption. Pros:
no conversion shim in editor integrations; matches `textDocument/publishDiagnostics`
without a hidden re-encoding pass. Cons: doubles the projection cost; computing UTF-16
units requires a second scan or a richer `LineIndex`.

**Answer C — Single byte-column projection in Tier A; defer scalar/UTF-16/display columns to a `ProjectionService` in Tier B.**
Tier A reports always carry `(byte_offset, line, column_utf8_bytes)`. A separate
`ProjectionService` in Tier B owns scalar-index, UTF-16, display-width, and
language-specific columns and is invoked by editor adapters. Pros: keeps Tier A surface
minimal; lets editor adapters pick their own column convention without bloating the
diagnostic shape. Cons: editor adapters must always run a projection pass.

**Recommendation:** Adopt **Answer C**. Tier A diagnostics are CLI-readable with byte
columns; Tier B/editor work converts on demand. CRLF/CR/LF normalization rule:
`LineIndex` stores raw byte offsets of each `LF`; an isolated `CR` (no following `LF`)
is also a line break for index purposes but column resets *after* the `CR`'s byte. HTML
preprocessing (NUL → U+FFFD, etc.) runs in Layer 1 and never changes byte offsets —
substituted scalars keep the original byte's range.

**Resolved detail:** Byte columns exclude a skipped BOM on line 1. The BOM is byte-stream
metadata, not a line character, but byte offsets still address the original BOM bytes.

**Remaining implementation detail:** Tab stops for display columns are deferred to Tier B
`ProjectionService`; Tier A never expands tabs.

**Concern 18.2.5 — Diagnostics before AST construction still need source-map stacks.**  
The research says source maps are not just a diagnostic side table, but parse and schema
diagnostics can occur before AST nodes exist. The current `Diagnostic` shape has
`byte_offset` and optional `node`, but no explicit `SourceMapStack`.

**Question:** Should diagnostics carry a `SourceMapStack` directly, or only a
`SourceId + ByteRange` until AST nodes exist?

**Answer A — Every `Diagnostic` carries a `SourceMapStack`, always.**
Replace `byte_offset: u64` + optional `node` with a required `source_map: SourceMapStack`.
Pre-AST diagnostics emit a single-frame stack
(`[Frame { transform: HtmlTokenizer | EncodingDecoder | ..., byte_range, source_id }]`).
AST-time diagnostics inherit the node's stack. Pros: one shape; consumers never branch
on "is there a node yet?"; aligns with the design rule that "source maps are an AST
contract, not a side table" — a diagnostic *is* a source-map consumer regardless of
phase. Cons: small allocation cost per pre-AST diagnostic that previously needed only a
`u64`.

**Answer B — Tagged union: `DiagnosticLocation::PreAst { source_id, byte_range } | NodeBound { node, source_map }`.**
Pre-AST diagnostics stay cheap (no stack allocation); AST-time diagnostics carry the
node's stack. Pros: minimal allocation in the hot tokenizer/decoder path. Cons:
consumers must handle both shapes; relinking pre-AST diagnostics to nodes after the fact
needs a separate pass.

**Answer C — `SourceMapStack` always, with a `Phase` tag.**
Same as A, but add `phase: DiagnosticPhase { ByteSource | Decode | Tokenize | Normalize | Schema | AstBuild | Transform | Render }`
so consumers can filter without inspecting the topmost frame. Pros: clearer report
grouping; matches the report-event model in §3.5 of the impl doc, which already records
"source module state" at emit time. Cons: phase is largely derivable from the topmost
`TransformKind` — risks duplicating that information.

**Recommendation:** Adopt **Answer A** (with `phase` derivable from
`source_map.last().transform`, not stored separately). The hot-path allocation worry is
small — a single-frame stack is one `Vec` with capacity 1, and most diagnostics are
either rare (errors) or batched (warnings). The §3.5 report-event model already requires
a source-map stack on every event; making `Diagnostic` consistent with that avoids two
location shapes.

**Ambiguity (to be answered):**
- Does the `node: Option<AstNodeId>` field stay (as a convenience back-reference) or
  get removed in favor of "look at the topmost AST frame"? Default: keep as
  `Option<AstNodeId>`, populated when (and only when) the diagnostic was raised against
  a built AST node.

### 18.3 ByteSource And Decoding Questions

**Concern 18.3.1 — Sentinel-byte semantics are unsafe unless ownership is explicit.**  
The LLVM `MemoryBuffer` model is useful, but a Rust `&[u8]` cannot guarantee a sentinel
byte after `bytes.len()` unless the runtime owns an internal padded allocation.

**Question:** Does `ByteSource.bytes()` expose the original byte slice without the
sentinel, or an internal padded buffer that includes it? How do offsets exclude the
sentinel?

**Answer A — Public `bytes()` returns the original slice; sentinel is private.**
`ByteSource` internally owns a padded `Vec<u8>` of length `n + K` (with `K ≥ 1`
zero/sentinel bytes). `bytes()` returns `&padded[..n]`. Lexers that need the sentinel
get it via a separate, internal-only API: `bytes_with_sentinel() -> &[u8]` (crate-private,
returning `&padded[..n + K]`). All public byte ranges, line indices, and snippet ranges
are within `[0, n)`. Pros: external invariant "offset < bytes.len()" is unambiguous;
sentinel is an implementation detail of the lexer; safe Rust slice semantics enforce the
boundary. Cons: requires the runtime to own the buffer (no zero-copy from a borrowed
caller `&[u8]` without an internal copy or padding allocation).

**Answer B — Public `bytes()` returns the padded slice; the contract documents that valid offsets are `< len_unpadded`.**
`ByteSource::len_unpadded() -> u64` is the authoritative end-of-source bound. Lexers may
read `bytes()[offset]` up to and including offset = `len_unpadded()` (the sentinel) but
must never address beyond that. Pros: one slice, no second API. Cons: easier to write
buggy consumers that scan `bytes().len()` directly; the sentinel leaks into the public
contract.

**Answer C — Two-mode constructor.**
- Owned mode: `ByteSource::from_owned(bytes: Vec<u8>)` allocates `+K` padding internally
  and behaves like Answer A.
- Borrowed mode: `ByteSource::from_borrowed(bytes: &'a [u8])` does not pad; lexers get a
  `bytes_with_optional_sentinel()` that returns `Either<padded, unpadded>` and falls
  back to a per-character bounds check on the unpadded path.

Pros: zero-copy for embedders that already pad; safe default for the common owned case.
Cons: lexer hot path must handle both shapes (or always copy on ingress to normalize).

**Recommendation:** Adopt **Answer A**. It cleanly separates contract (`bytes()` is real
source bytes; offsets are `[0, bytes.len())`) from implementation (lexer scans through
sentinel via a private API). Tier A is in-memory anyway (§18.3.3), so the one-time
padding allocation is acceptable. Borrowed-mode optimization is a Tier B concern.

**Ambiguity (to be answered):**
- How many sentinel bytes? `K = 4` (largest UTF-8 sequence length) is the safe default
  so `next_char` can read up to 4 bytes past `len_unpadded` without bounds checks. The
  sentinel byte value should be `0x00` (matches LLVM `MemoryBuffer`); confirm this does
  not collide with any tokenizer's "EOF" marker semantics.
- Should `bytes()` return `&[u8]` or `Cow<[u8]>`? `&[u8]` for simplicity.

**Decision 18.3.2 — Source-stream decoding policy.**
The parser consumes a Unicode scalar stream. Layer 1 owns the byte-to-Unicode transition
before tokenization starts for a source.

Encoding selection order for each byte-source initiation:

1. **BOM wins.** If the source begins with a supported BOM, the BOM determines the
   encoding. The BOM bytes are skipped from the decoded scalar stream, remain addressable
   in the original `ByteSource`, and cause later encoding overrides for that source to be
   ignored.
2. **Explicit/default encoding parameter.** If no BOM is present, use the encoding
   supplied with the parse request. For browser/server inputs, the caller can derive this
   from transport metadata such as `Content-Type` headers. For library callers, this is a
   parser configuration parameter.
3. **UTF-8 fallback.** If neither a BOM nor a supplied encoding exists, assume UTF-8.

Inline embedded contexts are not byte-source initiations. The owning context has already
decoded the source bytes, and the handoff passes a source-mapped decoded stream to the
child context. Therefore inline embedded contexts do not perform BOM detection and cannot
contain an independent BOM header. If an external resource or explicitly byte-valued
payload is loaded as its own `ByteSource`, it starts a new source initiation and applies
the same precedence above.

In-band encoding declarations discovered after decoding, including HTML metadata or
content-type-specific encoding switches, do not force the current source to be re-decoded.
If policy allows an in-band declaration to initiate or configure a later child byte
stream, it supplies that child stream's explicit/default encoding parameter. A BOM on the
child source still wins over that parameter.

HTML preprocessing replacements such as NUL handling occur after decoding on Unicode
scalars, not on raw bytes. An isolated UTF-8 BOM is accepted silently and excluded from
the decoded scalar stream; byte ranges continue to address the original source bytes.

**Concern 18.3.3 — Resource bounds are missing from the byte and decode layer.**  
The research emphasizes streaming and bounded memory, but Tier A uses in-memory buffers.

**Question:** What are the maximum input size, maximum line index size, maximum decoded
scalar count, and maximum diagnostic snippet size for Tier A?

**Answer A — Tight, compile-time-checked Tier A limits.**
- Maximum input size: **64 MiB** per `SourceId`. Larger inputs return
  `cem.source.too_large` (`Fatal`) at `from_bytes`/`from_path`. Rationale: Tier A is
  in-memory; 64 MiB covers the largest realistic single CEM document by ≥3 orders of
  magnitude.
- Maximum line count: **8 M** lines (matches 64 MiB at average ≥8 bytes/line). The
  `LineIndex` is a `Vec<u32>` of relative offsets per line, capped at this size.
- Maximum decoded scalar count: derived (bounded by input size); no separate cap.
- Maximum diagnostic snippet size: **240 bytes** before/after the offending byte range,
  truncated at the nearest line boundary; total snippet ≤ 1 KiB.
- Maximum frames in a `SourceMapStack`: **32** (sufficient for tokenizer →
  normalizer → schema → ast-builder → handoff(×N) → impl-transform → render).
- Maximum AST depth: **1024**.
- Maximum diagnostics per source: **10 000** (further diagnostics dropped with one
  trailing `cem.diagnostics.truncated` event).

**Answer B — Configurable limits with sane defaults.**
Same numbers as Answer A become fields on a `ParserConfig` struct passed to the runtime;
tests and benchmarks can lift them. Pros: future-proofing; one place to retune. Cons:
config plumbing through every layer.

**Answer C — Defer all limits to Tier B; Tier A is best-effort.**
Layer 1 imposes only `usize::MAX` (host limit). Pros: no policy work. Cons: silent
performance cliffs and no clean error code for "this input is too big to handle".

**Recommendation:** Adopt **Answer A** for Tier A and revisit the numbers in Tier B as
**Answer B** (with the same defaults). Hard-coded constants are easier to test and
document; configurability without a use case is premature.

**Ambiguity (to be answered):**
- Does the 64 MiB cap apply to the original source, the decoded scalar buffer, or
  both? Default: applies to the *original byte buffer*; decoded scalars are derived and
  do not have a separate cap.
- Should the limits be defined in a single `cem_ml::limits` module, or as
  `pub const` items on each layer? Default: `cem_ml::limits` for discoverability and
  test override.

### 18.4 Tokenizer And Event-Normalizer Gaps

**Concern 18.4.1 — Attribute-list boundaries are not represented.**  
The normalizer emits `OpenScope`, then `Name`/`Value` pairs for attributes, then children
appear later. The schema machine needs to know when the start tag's attribute set is
complete so it can validate required attributes, resolve duplicate-attribute overrides,
and validate element content separately.

**Question:** Should the normalizer emit an explicit `SeparatorKind::ElementBoundary`,
`StartTagEnd`, or `OpenScopeComplete` event after all attributes?

**Answer A — Emit `Separator { kind: ElementBoundary, byte_range }` after the last attribute of every start tag.**
The byte range covers the `>` (or `/>` end) of the start tag. SchemaMachine treats
`ElementBoundary` as the trigger for "attribute set complete; check required-attribute
rule and duplicate-attribute rule; transition to child-content state". Pros: reuses an
existing `SeparatorKind` variant; uniform with comma/colon/delimiter separators in
JSON/CSS handoff streams; no new event variant. Cons: zero-attribute start tags must
also emit it (otherwise the trigger is implicit), which makes the event stream slightly
more chatty.

**Answer B — Add a dedicated `OpenScopeComplete { name, byte_range }` event.**
A first-class event signals end-of-attributes. Pros: explicit and self-describing; easy
to grep for in the events projection. Cons: new variant in `NormalizedEvent`; slightly
more code in every consumer.

**Answer C — Emit attribute-set boundaries via a paired sub-scope: `OpenAttributes` / `CloseAttributes` around the `Name`/`Value` pairs, then content events follow.**
Mirrors `OpenScope`/`CloseScope` for the attribute "container". Pros: schema validation
of attribute multiplicity feels symmetric to content multiplicity; eases future
attribute-shape grammars. Cons: doubles event count for the common case; deviates from
research-paper event shape and existing §3.3 mapping table.

**Answer D — No event; SchemaMachine looks ahead one event for `OpenScope` or another opening event to detect attribute-set close.**
Pros: smallest event stream. Cons: implicit; complicates streaming where lookahead is
expensive; required-attribute checks happen lazily.

**Recommendation:** Adopt **Answer A**. It reuses the existing variant, requires one
line of addition to the §3.3 mapping table ("after all attribute pairs, emit
`Separator { kind: ElementBoundary }`"), and makes the SchemaMachine's "attribute phase
→ content phase" transition explicit. Update §3.3 mapping table to include the
boundary emission for both attribute-bearing and zero-attribute start tags.

**Ambiguity (to be answered):**
- Does `ElementBoundary` carry the byte range of the `>` character only, or the entire
  start tag? Default: the closing delimiter only (`>` or `/>`), so SchemaMachine can
  attribute "missing required attribute" diagnostics to a precise position.
- For self-closing tags, does `ElementBoundary` precede or coincide with
  the synthetic `CloseScope`? Resolved in Layer 3: precede. The boundary closes the
  attribute phase; the synthetic close then closes the element scope.

### 18.5 Schema-Machine And Validation Questions

*End of design document. Each ambiguity and review concern above should be resolved with
a brief decision record before the corresponding implementation phase starts. Resolved
items should be struck through and replaced with the chosen option and rationale.*
