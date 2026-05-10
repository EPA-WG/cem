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
For Tier A, the pipeline runs synchronously in-process against an in-memory byte buffer;
the encoder, compressor, and broadcast/cache paths are absent.

The schema machine instantiates a child pipeline (tokenizer → normalizer → schema
machine) for each embedded content region. The handoff stack controls the boundary and
return condition.

### 3.1 Resource Limit Policy

![Streaming data corridor illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-streaming-corridor.png)

*Streaming data corridor.*

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

Resource ceilings for transform-owned loading graphs use the same scope policy model.
The outer content type owns the effective policy, and child scopes may only increase
restraint by lowering limits or raising criticality. Fetch count, redirect depth, byte
count, reference depth, timeout, and allowed schemes are content-type policy fields, not
tokenizer behavior.

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
Embedded contexts may tighten restrictions or add content-type-specific validation, but
they may not weaken the outer policy. The outer context may override inner URL mappings
and substitutions; inner scopes cannot override parent mappings in a way that relaxes
security.

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
- Validate UTF-8 at ingress for the CEM HTML profile unless the HTML/WHATWG encoding
  decision resolves otherwise.

**Ambiguity 1** covers BOM handling differences between HTML and XML inputs. Encoding
policy and resource bounds remain open review items in §18.3.

### Tier A Scope

Tier A accepts in-memory byte buffers, string input, and file-path input. Chunked async
network delivery is deferred to Tier B, but Tier A must preserve absolute offsets so the
future streaming interface can reuse the same source-map model.

Implementation interfaces are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#31-layer-1-bytesource-and-encodingdecoder-cem_mlsource).

---

## 6. Layer 2 - SchemaTokenizer (`cem_ml::tokenizer`)

### Purpose

Converts decoded input into format-native tokens. The tokenizer is mode-aware and
schema-guided: it knows which lexical modes, embedded boundaries, and delimiter patterns
the schema defines. Structural validation, hierarchy reconstruction, and semantic rules
remain downstream.

### Functional Design

For HTML, the tokenizer follows WHATWG tokenizer states and emits source-spanned tokens.
It does not construct either the source-preserving initial HTML parser DOM or the WHATWG
implementation DOM. The schema-defined token hierarchy is reconstructed later by the
input DOM/AST builder, and WHATWG DOM compliance is applied as a content-type transform.

The schema can select valid tokenizer contexts and embedded-content boundaries, but it
does not rewrite WHATWG lexical behavior. XML follows the same layer contract with an
XML 1.0 profile so Layers 3 and above can consume a format-agnostic event stream.

XML constructs that require external resources or compatibility behavior, including DTDs,
entities, notation declarations, and XInclude, are delegated to the XML content-type
transform. Entity expansion is XML-specific and is not a CEM-ML primitive; CEM-ML
reference resolution uses slots and inlined references without cloning referenced content
into the originating tree.

**Ambiguity 2** covers whether Tier A wraps an existing Rust HTML5 tokenizer or uses a
custom WHATWG implementation.

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
- separator;
- mode switch for embedded content;
- error.

For HTML, each start tag emits an open scope followed by name/value events for each
attribute, preserving attribute source positions. End tags emit close scopes. Text emits
scalar text values. Comments are discarded unless the active schema marks comments as
significant. Parse errors become diagnostic events.

HTML `data-*` attributes stay in the HTML stack and may be projected to the HTML-specific
`dataset` equivalent on HTML AST nodes. CEM transform annotation attributes are
schema-qualified names, arrive as name/value events within the element's open-scope
group, and are handled by the schema machine.

The event enum and token mapping table are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#33-layer-3-eventnormalizer-cem_mlevents).

---

## 8. Layer 4 - SchemaMachine (`cem_ml::schema`)

### Purpose

Validates the normalized event stream against the CEM schema incrementally. Maintains a
push/pop stack of schema frames. The preferred algorithm is a RELAX NG derivative
validator; Tier A may use a hand-written DFA for the constrained CEM vocabulary if it
keeps the derivative replacement path open.

### Algorithm Selection

From the research algorithm comparison table:

- **Nested events -> visibly pushdown frame stack.** Start tags push frames, end tags pop
  frames, and attributes/text update the current frame.
- **Schema validation -> RELAX NG derivatives.** After each event, compute the residual
  schema `D(event, schema)`. If the residual is the empty language, emit a hard error.
  Residuals also provide expected-content diagnostics.

**Ambiguity 9** covers whether Tier A uses a full RELAX NG derivative engine or a
hand-written DFA for the initial CEM vocabulary.

### Functional Design

The schema machine validates scope opens, scalar values, separators, embedded handoffs,
and closes. It records diagnostics on the current frame, runs recovery for non-fatal
errors, aborts the current scope for fatal errors, and passes validated events to the
input DOM/AST builder.

A schema frame owns the active schema id, content type, validation state, expected close,
namespace context, source-map stack, seen names, and diagnostics. Exact frame fields and
transition sketches are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#34-layer-4-schemamachine-cem_mlschema).

### CEM Vocabulary In The Schema

The schema defines the namespace, qualified attribute names, allowed values, and nesting
rules for CEM transform annotations. CEM does not use HTML `data-cem-*` attributes for
CEM ownership. HTML `data-*` remains HTML-specific metadata and is exposed, when needed,
as the HTML `dataset` equivalent on HTML AST nodes. Based on the component
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
stack. The schema source language is **Ambiguity 3**.

### Namespace Resolution For Tags And Attributes

Names are resolved through the schema frame's namespace context before validation or
transformation. A parsed tag or attribute name has two identities:

- **lexical name:** the literal spelling in the source, such as `cem-screen`, `screen`,
  `cem:screen`, or `button`;
- **expanded name:** the resolved namespace plus local name, owned by a schema.

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
source-map stack, and a monotonic event sequence number.

The report tree can be projected to CEM-native, XML, JSON, Markdown, text, HTML, or any
other supported structured format. Text and HTML reports are reference convenience
renderers over the report tree, not canonical report storage formats.

Diagnostic and report data shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#25-diagnostics) and
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#35-report-event-model-cem_mlreport).

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
parent schema machine emits the authoritative handoff before yielding the byte stream to
the child parser. The child parser consumes through the declared return condition and
then returns control to the parent.

Tier A handoff cases:

| Parent context | Trigger              | Child content type             | Return condition    |
|----------------|----------------------|--------------------------------|---------------------|
| HTML document  | `<style>` start tag  | `text/css`                     | `</style>` end tag  |
| HTML element   | `style=""` attribute | `text/css` (declaration block) | attribute quote end |
| HTML document  | `<script>` start tag | raw text (not parsed Tier A)   | `</script>` end tag |

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
regions, recovered error nodes, and future schema-owned node kinds. These are not CEM
semantic constructs by default, but they remain part of the source-preserving input tree
when the active schema or content-type policy preserves them.

The minimal Tier A set of non-CEM constructs to preserve is TBD. CEM-specific support
and syntax for treating comments or CDATA as semantic CEM content is also TBD.

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
yet appeared binds to a mutable slot. When the parser later encounters the target id, it
fills the slot, and existing label/for/ARIA references observe the resolved target.
Remaining unfilled slots are checked at document close and reported according to the
severity decision in **Ambiguity 6**.

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
- **Code or transform content:** CSS, SCSS, XSLT or template DSL fragments, JavaScript
  when enabled by policy, and other content that can affect rendering or state but is
  not itself the rendered UI tree.
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
traversal, copy/pass-through rules, and source-map frame creation. Rust code, XSLT, or a
template DSL are execution backends for the schema-owned transform plan; none of them is
the reference source of truth.

For the CEM semantic HTML projection, schema-qualified CEM annotations drive custom
element output, wrapper generation, attribute generation, or no structural output
depending on the active transform plan. Schema-qualified CEM attributes can become
generated custom-element attributes such as `cem-id`, `variant`, or `state` according to
the active schema. Other standard HTML attributes (class, id, ARIA, and HTML `data-*`)
pass through only as HTML-owned metadata unless the active schema defines a stricter
mapping. Transformers match CEM annotations, not raw HTML `data-*` attributes or the
HTML-specific `dataset` projection.

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
- whitespace defaults to compact output unless a pretty renderer is explicitly selected.

Each output custom-element node appends an implementation transform frame. Prior frames
trace back to the original input token, enabling generated output such as `<cem-screen>`
to resolve back to `<main cem:screen="login">`.

### Transform Execution Backends

The reference implementation stack must execute schema-driven transform plans. A
hand-written Rust backend is allowed as developer convenience, for prototyping, and for
optimized execution of schema rules, but it must not become the essential source of
transform behavior. Any Rust implementation must be traceable back to schema-owned rules
and must preserve the same diagnostics and source-map semantics as another backend.

XSLT or an XSLT-like template backend is one possible execution backend for the same
schema-owned plan. Tier placement for the Rust backend, a minimal template DSL, or a full
XSLT engine is a scheduling decision and can be defined later as long as it does not
conflict with the primary principle that schema controls transform layers.

Transform interface shapes are in
[`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md#310-layer-9-implementationinterpreter-cem_mlinterpreter).

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
| L1 ByteSource: in-memory buffer, string, file path          | Design partial — source ownership/resource bounds need decisions (§18.3.1, §18.3.3)                                                                                                     |
| L1 ByteSource: async network streaming                      | Deferred Tier B — Tier A interfaces must still preserve absolute offsets for future chunked input                                                                                       |
| L1 EncodingDecoder: UTF-8                                   | Design partial — UTF-8-only CEM profile vs WHATWG encoding detection unresolved (§18.3.2)                                                                                               |
| L1 EncodingDecoder: UTF-16, Latin-1, BOM detection          | Design partial — required for HTML/XML profile clarity; blocked by Ambiguity 1 and §18.3.2                                                                                              |
| L1 Sentinel-byte ownership                                  | Design partial — Rust safety model for sentinel not resolved (§18.3.1)                                                                                                                  |
| L2 SchemaTokenizer: HTML WHATWG profile                     | Design partial — crate choice and token offset behavior unresolved (Ambiguity 2)                                                                                                        |
| L2 SchemaTokenizer: XML 1.0 profile                         | Design partial — namespace/name model remains unresolved (§18.4.4); DTD/external-resource ownership follows transform policy (§3.2, §6)                                                 |
| L3 EventNormalizer                                          | Design partial — attribute-list close event, void elements, name model, trivia, and ModeSwitch ownership unspecified (§18.4.1–4, §18.6.1)                                               |
| L4 SchemaMachine: visibly pushdown frame stack              | Design partial — recovery invariant, multiplicity/required-name state, and diagnostic propagation affect core semantics (§18.5.3–4, Ambiguity 8)                                        |
| L4 SchemaMachine: RELAX NG derivative engine                | Deferred Tier B — Tier A DFA must preserve a replacement path for residual diagnostics (Ambiguity 9, §18.5.1)                                                                           |
| L4 SchemaMachine: CEM vocabulary DFA                        | Design partial — DFA state table, schema compiler integration, and unknown-content policy remain unspecified (Ambiguity 3, Ambiguity 9, §18.5.1–2)                                      |
| L5 HandoffStack: ownership and return-condition tracking    | Design partial — authoritative handoff owner remains unresolved (§18.6.1); deferred cases are listed with XML → JSON → HTML/other implementation priority (§9)                          |
| L5 Child parser: CSS (stub, diagnostic only)                | Design partial — embedded-source byte/decoded-view model unspecified (§18.6.2)                                                                                                          |
| L5 Child parser: Script (raw text only)                     | Design ready — parser preserves raw text; warning/error/reject/allow behavior is defined by active scope/content-type policy (§3.1–3.2, §9)                                             |
| L6 InputDomAstBuilder: schema-defined initial DOM/AST       | Design ready — schema reconstructs token hierarchy; WHATWG DOM compliance is a downstream transformation over this initial DOM                                                          |
| L6 InterpreterAstBuilder: CEM annotation projection         | Design partial — CEM attributes are transform annotations on source nodes; transform conflict policy is schema-owned; Tier A non-CEM minimum and CEM comment/CDATA syntax remain TBD (§10) |
| L6 Reference slots: id/for/aria-*                           | Design partial — unfilled-slot severity remains unresolved (Ambiguity 6); concrete slot storage model is implementation TBD (§10)                                                        |
| L6 Source-map stacks: byte-range + transform chain          | Design partial — frame order, multi-range nodes, escape/entity decoding, and diagnostics-before-AST mapping unresolved (§18.2.1–3, §18.2.5)                                             |
| L6 Source-map stacks: bit-level ranges                      | Deferred Tier B — reserve representation only after source-map frame model is fixed (§18.2.1–2); no serialized binary frame ids in Tier A (§11)                                         |
| L7 BinaryAstEncoder                                         | Deferred Tier B — Tier A does not freeze serialized binary ids; canonical identity, ordering, and future id policy are scoped in §11                                                    |
| L8 ChunkCompressor                                          | Deferred Tier B — compression profiles are research-backed; canonical chunk identity, ordering, and dependency slots are scoped in §11                                                  |
| ContentTypeTransformPipeline: WHATWG HTML DOM               | Design ready — schema-driven initial HTML parser DOM is transformed into WHATWG implementation DOM updates                                                                              |
| L9 ImplementationInterpreter: schema-driven transform rules | Design ready — schema owns transform layers; namespace-qualified CEM identity resolves source collisions; canonical serialization and HTML `data-*` ownership are defined in §8 and §12 |
| L9 ImplementationInterpreter: transform execution backends  | Deferred Tier B/C — Rust, template DSL, and XSLT tier placement is a scheduling decision constrained by schema-owned transform rules (§12, Ambiguity 4)                                 |
| Visual content and machine state data                       | Design partial — uniform AST role model is defined; live hydration, browser adapters, and DOM patch identity are subject to a separate design phase TBD (§12)                           |
| LineIndex: byte-offset → line/col projection                | Design partial — column-unit model, newline normalization, tabs, replacement chars, and UTF-16/scalar projections unspecified (§18.2.4)                                                 |
| Diagnostics and reports                                     | Design partial — source-map ownership and diagnostics-before-AST mapping unresolved (§18.2.5)                                                                                           |
| CLI output projections and fixture round-trip reports       | Design ready — CLI owns projection targets and side outputs; stack layers own projected artifacts                                                                                       |
| Resource and security limits                                | Design partial — byte/decode bounds remain unresolved (§18.3.3); XML external-resource limits follow transform policy and content-type limits (§3.1–3.2, §6)                            |
| Incremental/editor parsing                                  | Deferred Tier B — caller-provided diffs map through source maps to changed scopes, with enclosing-scope rescan fallback                                                                 |
| Post-parse reference validation (unfilled slots)            | Design partial — Warning vs Error severity unresolved (Ambiguity 6 sub-question)                                                                                                        |
| Per-scope error boundaries                                  | Deferred Tier B (Ambiguity 5)                                                                                                                                                           |
| Async mutation API (`*Async` DOM mutations)                 | Deferred Tier B/C — outside the primary parsing research; requires separate runtime API design                                                                                          |

---

## 16. Algorithm Selection Summary

![Multi-format parser atlas illustration for the CEM-ML stack design.](assets/cem-ml-stack-design/announcement-parser-atlas.png)

*Multi-format parser atlas.*

| Layer     | Problem                      | Algorithm                                                | Reason from research                                                                           |
|-----------|------------------------------|----------------------------------------------------------|------------------------------------------------------------------------------------------------|
| L2        | HTML tokenization            | WHATWG tokenizer states                                  | Browser-compatible; separates token extraction from DOM                                        |
| L2        | XML tokenization             | XML 1.0 scanner                                          | Well-defined, same tokenizer contract as HTML                                                  |
| L3        | Cross-format event model     | Open/close/name/value taxonomy                           | Research §3: small event set lets schema validation share algorithms across formats            |
| L4        | Nested validation            | Visibly pushdown frame stack                             | Research §4, §Algorithms: "natural fit for open/close structures"                              |
| L4        | Schema validation Tier A     | Hand-written CEM DFA                                     | Simple constrained vocabulary; allows derivative upgrade without API change (Ambiguity 9)      |
| L4        | Schema validation Tier B     | RELAX NG derivatives                                     | Research §XML notes: "residual describes what was expected next" — streaming, good diagnostics |
| L5        | Embedded languages           | Parent-owned handoff with explicit return condition      | Research §5: "child parser never infers parent close condition independently"                  |
| L6        | Initial DOM/AST              | Schema-defined token hierarchy reconstruction            | Drives WHATWG HTML DOM compliance without making tokenization circular                         |
| Transform | WHATWG HTML DOM              | Content-type transform over initial HTML parser DOM      | Applies insertion modes, active formatting elements, foster parenting, and DOM updates         |
| L6        | Forward references           | Mutable scoped name slots                                | Research §4: "slot filled when defining entity arrives"                                        |
| L6        | Source location ground truth | `u64` byte offset                                        | Research Unicode policy: "byte offsets as stable storage format"                               |
| L6        | Line/column                  | On-demand projection via LineIndex                       | Research: "derived coordinates" — never stored, computed from byte offset                      |
| L9        | CEM transform semantics      | Schema-driven transform plan                             | Keeps schema in charge of transform layers across Rust, template, or XSLT backends             |
| L9        | UI virtualization            | Template reference + machine-state binding               | Reuses templates by reference and applies owned scope data during transformation               |
| L9        | CEM transform backends       | Rust convenience backend / template DSL / XSLT engine    | Backend tier placement is deferred; each backend executes the schema-owned plan                |
| Deferred  | Binary AST transport         | Dictionary-encoded subtree chunks                        | Research §Binary AST: parallel delivery, retry, cache reuse                                    |
| Deferred  | Chunk compression            | Zstandard (`canonical-fast`), Brotli (`canonical-dense`) | Research §Compression Strategy                                                                 |

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

**Recommendation:** Option A — keeps each format's rules unambiguous, avoids a shared code path that must know about
both WHATWG and XML BOM rules.

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

### Ambiguity 4 — Transform Backend Tier Placement

**Blocks:** Layer 9 implementation depth, not the schema-owned transform principle.

**Question:** Which execution backend is implemented in each tier for schema-owned
transform plans:  
A. Rust convenience backend generated from or traceable to schema rules.  
B. Minimal template DSL backend (match + value-of + apply-templates + copy).  
C. Full XSLT engine backend (Tier C per AC-T-3).

**Resolved principle:** Transform semantics are schema-driven. Hand-written Rust rules
are not the reference implementation source of truth; they are a developer convenience
or optimization backend only. Tier A or Tier C placement can be defined later as long as
the chosen backend executes the same schema-owned transform plan.

**Impact:** Option B requires building a template parser/evaluator but moves closer to
loadable transforms from URI or stream (AC-T-4). Option A is simpler for early execution,
but it must remain generated from or traceable to schema rules and cannot block a later
template or XSLT backend.

---

### Ambiguity 5 — Scope Granularity For CEM Documents

**Blocks:** Layer 4 scope-boundary design; Tier B scope isolation (AC-P-4, AC-I-3).

**Question:** Is a CEM parse scope (error boundary) one per document, or one per
top-level schema-qualified CEM element (e.g., per `cem:screen`)?

**Per document:** Simple; one schema machine per parse run.  
**Per `cem:screen`:** Aligns with AC-P-5 (nested scopes) and AC-I-3 (interpreter
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

**Hand-written DFA:** Purpose-built for the CEM vocabulary (eight transform annotations,
ten states, two dozen attributes). Fast to implement, deterministic, easy to test.
Cannot generalize to schemas beyond CEM without rewriting.

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
The implementation companion defines frames as "earliest context first", but the source
map examples list `CemAstBuilder` before `SchemaValidation`, `EventNormalizer`, and
`HtmlTokenizer`, which is latest-context first.

**Question:** Is `SourceMapStack.frames[0]` the original byte source frame or the current
AST/transformed frame? This must be fixed before traversal, compression deltas, and
generated-node inheritance are implemented.

**Concern 18.2.2 — A single `ByteRange` per frame is not enough for all research cases.**  
The research explicitly mentions merged nodes, split nodes, generated nodes,
transform-owned reference inlining, and source-map stacks through transformations. A
single `byte_range` cannot represent a text node produced from multiple source regions,
such as `a&amp;b`, or a node merged from adjacent text/event fragments.

**Question:** Should `SourceMapFrame` support one range, many ranges, generated sentinel
ranges, and transform-owned reference inlining? If not, where are escape decoding,
merge, split, and XML-compatibility entity mappings stored?

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

**Concern 18.4.4 — QName and namespace handling is only partially defined.**  
CEM namespace binding, scoped defaults, and ordered namespace overrides are defined in
§8. HTML lowercasing, XML namespace syntax compatibility, foreign content, prefixed
attribute parsing, and case sensitivity remain unspecified.

**Question:** What is the Tier A name model for HTML elements, HTML attributes,
schema-qualified CEM attributes, XML names, and future SVG/MathML foreign content?

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
which are ignored? Does the policy differ for standard HTML, ARIA, HTML `data-*`, and
schema-qualified CEM attributes?

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

---

*End of design document. Each ambiguity and review concern above should be resolved with
a brief decision record before the corresponding implementation phase starts. Resolved
items should be struck through and replaced with the chosen option and rationale.*
