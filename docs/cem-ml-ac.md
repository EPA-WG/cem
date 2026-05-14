# `cem-ml` / `cem-ml-cli` — Acceptance Criteria

> **Status: Primary decision driver**
>
> This file is the acceptance-criteria source of truth for `cem-ml` and
> `cem-ml-cli`. The stack design and implementation design explain and implement these
> criteria; they do not override them.
>
> History policy: the original AC seeded the stack design, then the stack design was
> corrected through later reasoning. This revision folds the resolved later reasoning
> back into the AC. Where the original AC and later design reasoning conflict, the newer
> resolved reasoning wins here. Remaining unresolved choices are tracked in
> [Open Questions](#open-questions) and must not be implemented as implicit decisions.

This document captures the acceptance criteria (AC) for the CEM parser/runtime and CLI. Each item is phrased as a checkable
statement so it can be referenced from `docs/todo.md` and from PR descriptions. Every requirement uses MUST / SHOULD /
MAY in the RFC 2119 sense.

> **Status legend**
>
> - **MUST** — required for the package to be considered shippable.
> - **SHOULD** — required unless an explicit waiver is recorded in this file.
> - **MAY** — explicitly optional; in scope for a later release.
> - **OPEN** — needs a decision before AC can be tested.

## Goal

`cem-ml` is the CEM document layer: a parser, schema machine, AST/report model,
transformation engine, and future runtime surface for CEM artifacts. It must support
semantic fixtures, component templates, design documents, transforms, and rendered
custom-element projections through one source-mapped processing model.

The long-range parity target includes HTML, XML, SVG, MathML, CSS, JSON, YAML, CSV,
JavaScript/TypeScript islands, Rust-facing schema artifacts, CEM-ML Query, XSLT-like
template transformation, Canvas command data, SMIL-style timed content, and binary AST
transport. Parity is tiered. A feature named here is not Tier A unless its AC item says
`[A]`.

## Conformance Tiers

The stack is large enough that one binary AC list would never finish. Three tiers:

- **Tier A — Streaming schema/transform MVP.** Parses the existing five
  `examples/semantic/*.html` fixtures through async Rust/WASM public APIs, validates
  them against a CEM-native schema compiled to a RELAX-NG-equivalent structural IR,
  builds a source-preserving input DOM/AST plus CEM projection, and transforms them to
  deterministic canonical CEM-ML and rendered light-DOM custom-element markup. Tier A
  includes source-stream decoding, basic namespace resolution, CEM schema-qualified
  annotations, source-map stacks, AST-associated reports, one-pass reference slots,
  WHATWG HTML tokenization, an XML 1.0 profile, and parent-owned HTML style/script
  handoff interfaces. Tier A may be single-threaded internally, but public processing
  APIs are asynchronous.
- **Tier B — Multi-content and runtime infrastructure.** Adds fuller embedded
  content-type switching, SVG/MathML child scopes, JSON/CSS/SCSS/JS island expansion,
  external-resource loading through policy-gated queues, scoped template/registry
  lookup, plugin chains, bounded worker pools, cancellation, scheduler traces,
  incremental/editor parsing, and benchmark publication.
- **Tier C — Full document/runtime vision.** Adds NVDL-style dispatch, broad
  XSLT/XPath-equivalent CEM template/query coverage, Canvas/SMIL and additional format
  parity, live render-while-parsing, DOM patching/hydration, async DOM mutation APIs,
  binary AST transport, chunk compression, and advanced generated type artifacts.

Each AC below is tagged `[A]`, `[B]`, or `[C]`. Initial release closes Tier A and explicitly lists which Tier B/C items
are deferred.

---

## 0. Cross-Cutting Feature Requirements

- **AC-F-1 [A] MUST** define scope policy as the shared mechanism for encoding defaults,
  error-level overrides, namespace bindings, content type, schema id, resource limits,
  diagnostic visibility, and parent override bounds.
- **AC-F-2 [A] MUST** support schema loading from stable URI/file identities. Inline
  schema declarations and mid-document schema switch syntax are **OPEN** until
  Open Questions item 2 is resolved.
- **AC-F-3 [A] MUST** define tags, attributes, namespaces, open-content policy, CEM
  annotation names, state values, and transform hooks in the CEM-native schema.
- **AC-F-4 [A] MUST** model namespace-owned content-type switches as parent-owned
  handoffs with explicit return conditions. Tier A implements HTML `<style>`,
  `style=""`, and raw-text `<script>` handoff interfaces; fuller namespace/content
  switching is Tier B/C.
- **AC-F-5 [A] MUST** define CEM reference slots for `id`/`for`/`aria-*` and template or
  entity references without cloning referenced content into the source tree. XML entity
  expansion remains XML-transform-owned compatibility behavior.
- **AC-F-6 [A] MUST** expose parser, schema, validation, transform, reporting,
  resource-policy, and source-map contracts. Plugin, concurrency, runtime interpreter,
  and DOM mutation contracts are required by later-tier AC items.
- **AC-F-7 [A] MUST** keep data streaming where possible. Public Rust and WASM
  entry points are asynchronous and accept finite source adapters or streams. Internal
  Tier A processing may use task-local state and token-local buffers.

## 1. Parser

- **AC-P-1 [A] MUST** parse HTML5 documents and XML 1.0 well-formed profile documents
  into the same source-preserving input DOM/AST abstraction. The public parser tree
  surface exposes common document, element, attribute, text, trivia, processing
  instruction, raw-text, error-node, source-map, and reference-slot APIs. WHATWG
  implementation DOM and CEM AST are projections over that input tree, not competing
  parser roots.
- **AC-P-2 [A] MUST** be a streaming-first parser: it accepts async byte/string source
  adapters, including WASM `ReadableStream<Uint8Array | string>` where available, and
  emits parse/report events incrementally. Tokenizer buffering MUST be token-local and
  bounded; retained AST, report, source-map, line-index, reference, and diagnostic data
  MUST be governed by explicit resource caps instead of hidden full-source buffering.
- **AC-P-3 [A] MUST** report parse errors with `{ uri, line, column, byteOffset, code, severity, message }`. Errors do
  not abort the stream unless the effective scope policy maps severity to `fatal` or
  fail-fast behavior. `byteOffset`, `line`, and `column` are projections from the
  selected source-map frame; byte ranges are the canonical coordinate.
- **AC-P-4 [A] MUST** support **context scopes** with stable projected identity
  `{ schemaUri, contentType, namespaceUri }` plus implementation-owned scope id,
  namespace context, source-map stack, and effective scope policy. Diagnostics originate
  in the detecting scope and bubble to the nearest schema-declared or context-root
  error boundary, where policy decides hide/report/recover/abort behavior.
- **AC-P-5 [A] MUST** allow scopes to nest. A scope can contain child scopes of a
  different content type, namespace context, schema id, and policy envelope. Child scopes
  may relax or hide local diagnostics only within parent override bounds.
- **AC-P-6 [C] MUST** dispatch between schemas mid-document via NVDL-style rules (see References).
- **AC-P-7 [A] MUST** preserve source-map stacks on every source-derived node. Frames
  are ordered origin-first, the current frame is last, and each frame uses byte ranges
  as durable location identity. Line/column are report projections, not parser
  semantics.
- **AC-P-8 [A] MUST** use CEM-native schema and document syntax as the source of truth
  for CEM schema behavior. RELAX NG and other schema formats are mirrors/adapters, not
  competing canonical CEM sources.
- **AC-P-9 [A] MUST** preserve comments, whitespace, doctypes, processing
  instructions, CDATA/raw-text nodes where supported by content type, and recovered
  error nodes in the initial input DOM/AST unless the effective scope policy strips them
  through a transform that preserves report/source-map references.

## 2. Schema

- **AC-S-1 [A] MUST** define schemas in **CEM-native syntax** as the source of truth.
- **AC-S-2 [A] MUST** emit equivalent **RELAX NG** XML and compact syntax mirrors for
  validation/tooling alongside the native form. Mirrors MUST be byte-stable for
  unchanged input. XSD is a downstream adapter only when a consumer explicitly requires
  it.
- **AC-S-3 [A] MUST** emit type headers in **TypeScript** (`.d.ts`) that mirror schema element/attribute shapes.
- **AC-S-4 [B] SHOULD** emit type headers in **Rust** (`.rs`) for native consumers.
- **AC-S-5 [A] MUST** expose schemas at stable URIs so namespace declarations in documents can resolve them.
- **AC-S-6 [A] OPEN** — pick TS-emit strategy: structural types vs branded nominal types. Affects ergonomics and
  validation cost.
- **AC-S-7 [A] MUST** compile CEM-native schemas into a structural validation IR with
  RELAX-NG functional parity. Tier A MAY execute a limited DFA profile generated from
  that IR; unsupported Tier A structural constraints fail schema compilation instead of
  silently weakening validation.
- **AC-S-8 [A] MUST** emit a schema-owned rule registry for cross-reference,
  semantic/contextual, lexical/mode, tokenizer-boundary, policy, and transform rules
  with explicit execution placement.
- **AC-S-9 [A] MUST** define CEM annotations as schema-qualified names, not HTML
  `data-*` attributes. HTML `data-*` resolves to synthetic HTML-data metadata and does
  not become CEM-owned unless a schema rule maps it.

## 3. Validation & Strict Typing

- **AC-V-1 [A] MUST** validate documents against schemas and surface diagnostics through
  every parser, schema, AST, report, transform, and rendered-output layer using the same
  diagnostic identity and source-map stack. XSLT-compatible consumers are future
  adapters over the same diagnostic model; unrestricted XSLT runtime behavior is not a
  Tier A contract.
- **AC-V-2 [A] MUST** be **forgiving by default within a semver-compatible namespace**. If the document declares
  schema version X.Y and the loaded schema is X.Y′ (Y′ ≥ Y), unknown elements/attributes produce **warnings, not
  errors**.
- **AC-V-3 [A] MUST** be **strict on major mismatch**. If the document pins schema X and the loaded schema is X+1,
  validation MUST fail loudly and abort the scope.
- **AC-V-4 [A] MUST** emit validation reports from the canonical AST-associated report tree. JSON output is required
  for machine-readable validation reports. Text and HTML outputs MAY be provided by the reference implementation as
  convenience renderers; they are not canonical storage formats. The internal report is
  an event-time AST-associated tree with source-map stacks, visible partial hierarchy,
  active scope context, origin/boundary scopes, and monotonic event sequence numbers.
- **AC-V-5 [A] MUST** validate the existing `examples/semantic/*.html` fixtures with **zero hard violations**.
- **AC-V-6 [A] SHOULD** detect: unknown elements/attrs, invalid state combinations, missing accessible names, broken
  `id`/`for`/`aria-*` references, unsafe inline content.
- **AC-V-7 [A] MUST** use schema-owned open-content policy for unknown elements and
  attributes. Defaults: unknown CEM-owned names are errors, unknown HTML attributes are
  warnings, WHATWG custom elements are accepted, ARIA/`role` are deferred to semantic
  validation, and unbound prefixes are errors unless policy overrides them.
- **AC-V-8 [A] MUST** recover from non-fatal schema errors with tainted recovered
  subtrees when policy allows recovery. Recovered subtrees preserve source maps and do
  not corrupt the parent structural state.

### 3.1 Schema Version Identity

This sub-section is normative and is cited by AC-V-2, AC-V-3,
AC-CC-1, AC-CC-3, AC-S-5, the G-NVDL gate, and
`cem-ql-ac.md` AC-QT-4 / AC-QC-1. AC-V-2 and AC-V-3 hinge on a
precise definition of "schema version"; without it, two
implementations can produce subtly different validation outcomes
for the same document.

- **AC-V-9 [A] MUST** define schema version identity as a pair:
  - **URI** — stable schema identity per AC-S-5. The URI MAY end
    with a version segment as a **partial pin**, in any of the
    forms `MAJOR` (`/1`), `MAJOR.MINOR` (`/1.2`), or full
    SemVer 2.0 (`/1.2.3`, `/1.2.3-rc.1`, `/1.2.3+sha.abc`). The
    version segment, when present, expresses the document's
    **constraint** and is matched by the loader against schema
    candidates' embedded version per AC-V-10. A URI without a
    version segment means "any version of this schema."
  - **Embedded version** — every loaded schema descriptor MUST
    carry a **complete** SemVer 2.0 string in `descriptor.version`
    (e.g. `1.2.3`, `1.2.3-rc.1`, `1.2.3+sha.abc`; never `1.2`).
    Partial versions in this field are rejected at load with
    `cem.v.semver_invalid`. The embedded version is the
    authoritative version of the loaded schema; the URI tail is
    advisory.

- **AC-V-10 [A] MUST** define how URI-tail constraints match
  embedded versions during schema resolution:
  - URI without version segment matches any embedded version.
  - URI ending in `MAJOR` matches any embedded `MAJOR.*.*`.
  - URI ending in `MAJOR.MINOR` matches any embedded
    `MAJOR.MINOR.*`.
  - URI ending in full SemVer matches embedded versions that
    satisfy AC-V-2 forgiving-mode comparison against the URI's
    full version (same `MAJOR`, embedded `(MINOR, PATCH)` ≥
    URI `(MINOR, PATCH)` per SemVer §11 precedence).
  - Prereleases satisfy a non-prerelease URI segment **only when
    explicitly named** (`/1.2.3-rc.1` matches embedded
    `1.2.3-rc.1`, never embedded `1.2.3` or vice versa). This
    matches npm default behavior and avoids surprising
    rc-into-stable matches.
  - Build metadata (`+meta`) on the URI tail is matched
    case-sensitively against the embedded build metadata when
    the URI specifies it; URIs without build metadata match
    embedded versions with any build metadata.

- **AC-V-11 [A] MUST** ground AC-V-2 and AC-V-3 comparison in the
  **embedded full version** of the loaded schema, not the
  URI-tail partial version. Concretely, given declared embedded
  `D` (from the document's resolved schema reference) and loaded
  embedded `L`:
  - **AC-V-2 forgiving mode** fires when `L.MAJOR == D.MAJOR` and
    `(L.MINOR, L.PATCH) >= (D.MINOR, D.PATCH)` per SemVer §11
    precedence. Unknown elements/attributes warn under
    `cem.v.minor_skew`.
  - **AC-V-3 strict mode** fires when `L.MAJOR != D.MAJOR`, or
    when `L.MAJOR == 0` and `(L.MINOR, L.PATCH) != (D.MINOR,
    D.PATCH)` (per SemVer §4 — `0.x` is unstable, every bump may
    break). Validation aborts the scope under
    `cem.v.major_mismatch`.
  - Build metadata (`+meta`) is **ignored** for AC-V-2 and AC-V-3
    precedence per SemVer §10. It IS included in the AC-CC-1
    fingerprint (see AC-V-12) so the cache distinguishes builds
    without changing validation behavior.
  - A prerelease declaration is satisfied only by the exact
    embedded prerelease per AC-V-10. Mismatched prereleases
    abort under `cem.v.prerelease_unmatched`.

- **AC-V-12 [A] MUST** make the **embedded full version** the
  canonical input to the AC-CC-1 schema fingerprint and the
  AC-CC-3 policy stamp, including any prerelease and build
  metadata components verbatim. The URI-tail partial version
  MUST NOT enter the fingerprint — two documents declaring the
  same schema with different URI tails (`/1.2` vs `/1` vs
  unversioned) MUST hash to the **same** fingerprint when they
  resolve to the same embedded version. This decouples cache
  identity from author shorthand and keeps cross-host artifact
  reuse stable across documents that pin loosely.

- **AC-V-13 [A] MUST** record the URI-to-embedded resolution as
  a structured event in the report tree per AC-O-3 with code
  `cem.v.semver_resolved`. The event carries the URI as
  declared, the embedded full version, and which AC-V-10
  matching rule fired (`unconstrained`, `major`, `major-minor`,
  `full`, `prerelease-exact`). This makes G-NVDL dispatch traces
  and cross-host cache mismatches diagnosable without re-running
  the loader.

- **AC-V-V-S [A]** — verification (under §"Verification Plan"):
  one document per row of AC-V-10's matching table loads against
  a fixture schema descriptor with a known embedded version;
  assert the resolution event carries the documented match rule;
  assert AC-V-2 / AC-V-3 outcomes per AC-V-11 across at least one
  minor-skew case, one major-mismatch case, one `0.x` minor-skew
  case (which must trigger strict mode), one prerelease-exact
  case, and one prerelease-mismatch case; assert AC-CC-1
  fingerprints collapse across `/1`, `/1.2`, and `/1.2.3` URI
  forms when they resolve to the same embedded `1.2.3`.

#### Diagnostic codes (this sub-section)

| Code                            | Severity (default)        | Source AC      |
|---------------------------------|---------------------------|----------------|
| `cem.v.semver_invalid`          | error (rejected at load)  | AC-V-9         |
| `cem.v.minor_skew`              | warning                   | AC-V-11/AC-V-2 |
| `cem.v.major_mismatch`          | error (aborts scope)      | AC-V-11/AC-V-3 |
| `cem.v.prerelease_unmatched`    | error (aborts scope)      | AC-V-11        |
| `cem.v.semver_resolved`         | info                      | AC-V-13        |

Scope policies MAY remap severities per the host bubble-to-boundary
contract; the values above are the shipped defaults.

## 4. Interpreter & DOM State Machine

The implementation MUST split:

- **Parser** — content → events / typed nodes.
- **Interpreter** — validated AST/projection state → transform or runtime state
  transitions.

Tier A materializes a source-preserving input DOM/AST, then applies WHATWG HTML DOM
compliance as a content-type transform. A browser-style DOM state machine is a later
runtime surface.

- **AC-I-1 [A] MUST** treat parsing, validation, AST construction, content-type
  transforms, CEM projection, and rendered output as explicit state transitions with
  source-map frame creation at each transform boundary. A public DOM `apply(transform)`
  API is **OPEN** for the runtime phase, not required for Tier A.
- **AC-I-2 [A] MUST** switch parser/transform context when content type changes. Tier A
  switches for HTML `<style>`, `style=""`, and raw-text `<script>` boundaries; SVG,
  MathML, CSS `url(...)`, JSON, JS template islands, and external resources are Tier B/C.
- **AC-I-3 [A] MUST** allow parser, transform, and handoff scopes to own subtrees and
  route writes through the owning transform pipeline. Public cross-scope DOM mutation
  rejection is governed by the deferred DOM mutation ACs.
- **AC-I-4 [B] MUST** support render-while-parsing: an interpreter can emit visible state before EOF on its input
  stream. Examples that drive this requirement: top-level-await scripts, HTML with external images/CSS.
- **AC-I-5 [B] SHOULD** batch render updates. The render-batch policy
  is **host-defined**, owned by the scope policy per AC-F-1 and
  inherited per AC-A-4: the cem-ml library exposes the knob but does
  **not** dictate a numeric default. Each scope policy sets a
  `render_batch_window` (a duration after which pending updates flush
  even if the input stream has not completed) plus the implicit
  flush-on-stream-completion trigger. Consumer apps choose values
  that fit their environment:
  - **Browser / interactive hosts** typically use a short window
    (~100 ms is a common author choice that matches first-paint
    perception budgets); flush also fires on stream completion.
  - **Server-side / build-pipeline hosts** typically equate the
    window with the host's critical timeout — render flushes only
    on stream completion or when the host's request/job timeout
    fires, whichever comes first; the timeout reaching the render
    layer materializes as the host's critical-timeout error.
  - **Test / snapshot hosts** typically use stream-completion-only
    (no time-based flush) so render output is deterministic across
    runs.

  Reference implementations MAY ship a thin convenience preset per
  host environment for ergonomics, but the preset is host-side
  configuration, not a library-level default. Child scopes MAY
  shorten the inherited window but MUST NOT lengthen it, per the
  AC-A-4 cap-tightening rule.
- **AC-I-6 [A] MUST** implement WHATWG HTML DOM compliance as a schema-driven
  content-type transform over the initial HTML parser DOM. Full browser DOM API
  compatibility remains a later runtime decision.

## 5. DOM Mutation API — Async Layer Over Sync Surface

The DOM mutation API is a required runtime-phase feature, but it is not part of Tier A
parser/validation/transform shipment. Tier A must preserve enough ownership,
source-map, scope, and report metadata for this API to be added without replacing the
parser stack. The synchronous WHATWG DOM (`appendChild`, `setAttribute`,
`textContent =`, `innerHTML =`, `replaceWith`, ...) remains a compatibility target for
later browser/runtime adapters; CEM-owned mutation uses async counterparts that
participate in interpreter ownership, scope scheduling, batching, and diagnostics.

### Surface

- **AC-M-1 [C] MUST** expose an async counterpart for each mutating operation in the DOM core. Naming convention:
  suffix `Async`. Initial runtime set:
  - `appendChildAsync(node)` / `prependAsync(...nodes)`
  - `insertBeforeAsync(node, ref)`
  - `removeChildAsync(node)` / `removeAsync()`
  - `replaceChildAsync(newNode, oldNode)` / `replaceWithAsync(...nodes)`
  - `before/afterAsync(...nodes)`
  - `setAttributeAsync(name, value)` / `removeAttributeAsync(name)` / `toggleAttributeAsync(name, force?)`
  - `setTextContentAsync(text)`
  - `setInnerHTMLAsync(html)` / `setOuterHTMLAsync(html)` — these MUST run the parser+interpreter for the embedded
    content type (HTML by default).
  - `cloneNodeAsync(deep?)` — async only when `deep` triggers cross-scope clone.
- **AC-M-2 [C] MUST** return `Promise<void>` from every `*Async` mutator. Resolution means the mutation is committed
  to the DOM **and** the owning interpreter has finished any cascading work (style recalc, scoped registry updates,
  child-scope spawning).
- **AC-M-3 [C] SHOULD** keep the sync mutators available, but route them through a fast path that is functionally
  equivalent to `await mutator()` followed by a synchronous flush. Sync mutators MUST NOT be used inside an
  interpreter that owns a subtree under active streaming — they SHOULD throw `OwnedSubtreeError` if attempted.

### Semantics

- **AC-M-4 [C] MUST** route every async mutation through the owning interpreter's queue (per AC-I-3, AC-A-4).
  Mutations from outside the owning scope SHOULD be rejected with `ScopeViolationError`.
- **AC-M-5 [C] MUST** preserve **submission order** within a single owner: if `appendChildAsync(a)` is called before
  `appendChildAsync(b)` against the same parent, `a` MUST settle before `b`. Cross-parent ordering is not guaranteed.
- **AC-M-6 [C] SHOULD** **coalesce** mutations that fall within the same batch window (per AC-I-5) into a single
  observer notification — multiple `setAttributeAsync` calls on the same node within the host's configured
  `render_batch_window` surface as one `MutationRecord`. The Promise of each call resolves only once the merged batch flushes.
- **AC-M-7 [C] MUST** support `AbortSignal` on every `*Async` mutator. Aborting before the queued mutation begins
  rejects the promise with `DOMException("Aborted", "AbortError")` and skips the mutation. Aborting after work has
  begun follows the rollback policy in AC-M-9.
- **AC-M-8 [C] MUST** dispatch `MutationObserver` callbacks **after** the batch flushes, not at promise-resolution
  time. Observer callbacks see a consistent post-batch DOM.

### Errors & Rollback

- **AC-M-9 [C] OPEN** — rollback contract on rejection. Three candidate models:
  - *Atomic*: a rejected `*Async` mutation leaves the DOM unchanged.
  - *Best-effort*: partial application is allowed; the rejection carries the post-state.
  - *Transactional*: mutations declared inside `withTransaction(async () => …)` are atomic; bare mutators are
    best-effort.
  - Recommendation: pick **Transactional** because it matches both interpreter ownership and the parser's
    error-boundary model. Confirm before implementation.
- **AC-M-10 [C] MUST** route rejected mutations through the same error stream as parser/validator errors
  (`onParseEvent`, AC-O-1) so consumers see one error pipeline.
- **AC-M-11 [C] SHOULD** validate the post-mutation tree against the active schema before commit. A schema violation
  rejects the promise with `SchemaViolationError` and rolls back per AC-M-9.

### Concurrency & Resources

- **AC-M-12 [C] MUST** count mutation work against the same per-scope thread-pool slot as parsing/transform work
  (AC-A-4). Mutations MUST NOT bypass the pool by going synchronous under the covers.
- **AC-M-13 [C] SHOULD** allow **read amid pending writes**: synchronous reads see the last committed state, never an
  in-flight intermediate. Reads inside the same microtask after an awaited mutation MUST observe that mutation.
- **AC-M-14 [C] MAY** offer `flushAsync(scope?)` to force pending mutations to commit immediately, bypassing the
  batch window. Used by tests and by `apply(transform)` (AC-I-1) when transform completion requires a settled tree.

### Verification

- **AC-M-V-1** — round-trip test: mutate every fixture in `examples/semantic/*.html` via the async API, snapshot
  before/after, confirm the snapshot matches an equivalent sync-mutation reference.
- **AC-M-V-2** — ordering test: assert `await Promise.all([appendChildAsync(a), appendChildAsync(b)])` results in the
  documented order regardless of microtask scheduling.
- **AC-M-V-3** — abort test: a mutation aborted via `AbortSignal` must leave both the DOM and observers untouched
  (assuming Atomic or Transactional resolution of AC-M-9).
- **AC-M-V-4** — observer test: N mutations within the batch window produce 1 `MutationRecord`; N+1 mutations split
  across the window produce 2 records.

## 6. Transformations

- **AC-T-1 [A] MUST** apply schema-driven CEM template transformations to a DOM/AST
  subtree. Tier A covers the XSLT-like subset needed for fixture transformation:
  template matching, scoped selection, value extraction, copy/pass-through behavior,
  recursive application, generated attributes, deterministic serialization, and
  source-map frame creation. It does not adopt unrestricted XPath/XSLT execution as the
  runtime contract.
- **AC-T-2 [A] MUST** transform every fixture in `examples/semantic/*.html` to light-DOM custom-element markup
  compatible with `@epa-wg/custom-element` and snapshot the output.
- **AC-T-3 [C] SHOULD** evaluate XSLT 4.0 surface (see `qt4cg.org` reference) for
  completeness and map accepted capabilities into CEM template/query semantics.
- **AC-T-4 [A] MUST** support schema-owned transform plans loaded from the compiled
  CEM-native schema. Loading arbitrary transforms from URI, `ReadableStream`, or DOM is
  Tier B/C and must obey the same resource and source-map policy.
- **AC-T-5 [A] MUST** produce canonical CEM-ML transform output as the stable snapshot,
  hash, and cache identity. Rendered light-DOM custom-element HTML is a deterministic
  projection of that canonical output.
- **AC-T-6 [A] MUST** preserve standard HTML attributes, ARIA, class/id, and synthetic
  HTML-data metadata as pass-through input unless the active schema defines a stricter
  mapping. Transformers match schema-qualified CEM annotations, not raw `data-*`.
- **AC-T-7 [A] MUST** embed `cem-ql` expressions inside CEM-ML template bodies using
  **XSLT 3.0-style syntax**, with no CEM-specific delimiter invented:
  - **Attribute Value Templates (AVTs)** — any template attribute value may
    interpolate cem-ql expressions inside `{ … }`, e.g.
    `<cem:value text="Hello {.name}, {count(.items)} items"/>`. Multiple
    interpolations per attribute are allowed; literal `{` and `}` are
    escaped as `{{` and `}}` (XSLT rule).
  - **`select=` attributes** — template elements designed to take a single
    expression as their entire attribute value (e.g. `cem:value-of`,
    `cem:for-each`, `cem:if`, `cem:choose`/`when`, `cem:variable`) use
    `select="cem-ql-expression"`. The attribute value is the cem-ql source
    verbatim, with HTML attribute escaping applied; no surrounding `{ }`.
  - **Match / test / use / group-by attributes** — `match=`, `test=`,
    `use=`, `group-by=`, and any attribute documented as taking a query in
    a CEM template schema follow the same `select=` rule: full attribute
    value is one cem-ql expression.
  - **Text-node interpolation** — text content inside CEM-ML template
    bodies MAY use the same `{ … }` AVT form, gated by the surrounding
    template element's schema declaration (no global text interpolation
    by default; opt-in per element type per the CEM template schema).
  This binds the host side of `cem-ql-ac.md` AC-QS-6 (cem-ql is not
  embedded in HTML attributes by default; the template compiler handles
  the boundary) and is the normative CEM-ML template embedding
  contract. The cem-ql
  parser is invoked on the attribute-value or text-node substring after
  the template compiler strips AVT braces and resolves XML escapes; the
  emitted source-map frame is `TransformKind::TemplateEmbedding` and
  preserves both the host attribute span and the cem-ql sub-span per
  AC-P-7.

## 7. Transformation Plugins

The built-in stack covers HTML / XML / CSS stubs, CEM template transforms, and future
SVG/MathML/JSON/JS islands. Real projects need content types the platform does not ship
such as SCSS, TypeScript, JSX, Markdown variants, project-specific DSLs, and
cross-cutting concerns that augment any content type (security checks, click tracking,
telemetry). Plugins extend the transformation chain without forking the runtime. The
plugin API is a Tier B decision driver even though the current stack design still needs
the concrete plugin architecture section.

### Plugin Descriptor

- **AC-PL-1 [B] MUST** — every plugin is registered via a descriptor:
  `{ name, version, inputContentType, outputContentType, mode: 'observe' | 'mutate', invoke, supportsSourceMap }`.
  `inputContentType` MAY be a list (a linter that runs across CSS and SCSS, for example).
- **AC-PL-2 [B] MUST** — invoke signature: `invoke(input, ctx) => Promise<output>`. `ctx` exposes scope identity, the
  AbortSignal, and a write-channel for validation/observability events.
- **AC-PL-3 [B] MUST** — `mode: 'observe'` plugins (non-invasive) MUST NOT mutate the output. Runtime SHOULD enforce
  by passing a frozen / structural-share view; attempts to mutate raise `ObserverViolationError`.
- **AC-PL-4 [B] MUST** — `mode: 'mutate'` plugins (invasive) MUST set `supportsSourceMap: true` and emit a source map
  with every output. The runtime rejects registration of mutate-mode plugins without source-map support.
- **AC-PL-5 [B] MUST** — every plugin runs inside the scope that hosts it; it cannot reach across scopes.

### Scope Transformation Chain

- **AC-PL-6 [B] MUST** — each scope owns a chain of plugins applied in **outer → inner** order:
  `[ ancestor plugins (reverse install order) → scope-local plugins ]`. A scope sees the merged chain at apply time.
- **AC-PL-7 [B] MUST** — a parent scope can install plugins that target all descendant scopes whose
  `inputContentType` matches. Outside-down control is mandatory.
- **AC-PL-8 [B] MUST** — a descendant scope MAY add plugins on top of inherited ones, but MUST NOT remove, reorder, or
  bypass ancestor plugins. Ancestors retain control.
- **AC-PL-9 [B] MUST** — by default, all `observe` plugins for a content type run **before** any `mutate` plugin, so
  security/lint passes see pre-mutation content. Plugins MAY declare a numeric priority that the runtime honors
  within their mode.
- **AC-PL-10 [B] SHOULD** — `observe` plugins MAY run in parallel with each other; `mutate` plugins serialize so each
  one's source map references the previous output deterministically.
- **AC-PL-11 [B] MUST** — built-in transformers (HTML tokenizer/parser, CSS parser,
  CEM template transform, future XSLT-compatible adapters, etc.) are addressable as
  plugins via the same descriptor surface. One model, one chain.

### Source Maps & Debugging

- **AC-PL-12 [B] MUST** — every mutation plugin emits a source map. Format: V3, or a CEM-native equivalent that
  round-trips losslessly to V3.
- **AC-PL-13 [B] MUST** — the runtime stitches source maps across stacked mutation plugins so a debugger or
  programmatic resolver walks back through every layer to the original source. Verified by AC-PL-V-5.
- **AC-PL-14 [B] SHOULD** — source map entries include the originating scope identity so cross-scope edits remain
  attributable in the merged map.

### Examples (non-normative)

- **Non-invasive — security checker.** Registered against `text/css`, mode `observe`. Watches for `expression(...)`,
  `javascript:` URIs, and unbounded selectors; emits validation events. Output tree is byte-identical to input.
- **Invasive — click tracker.** Registered against `text/html`, mode `mutate`. Adds `data-track-id` and an inline
  click handler to every interactive element. Emits a source map so devtools can step from the injected handler back
  to the source attribute and the originating scope.
- **Invasive — SCSS → CSS.** Registered with `inputContentType: 'text/scss'`, `outputContentType: 'text/css'`,
  mode `mutate`. Converts and emits a source map; downstream CSS plugins (e.g., the security checker above) see the
  generated CSS but can resolve back to SCSS via the stitched map.

### Errors & Resource Limits

- **AC-PL-15 [B] MUST** — plugin failure is a scope error per AC-P-4. It MUST NOT cascade beyond the scope that hosts
  the plugin. Failure mode for the rest of the chain (skip downstream / abort scope)
  follows the hosting scope policy. DOM mutation rollback uses AC-M-9 only when the
  plugin participates in the Tier C mutation runtime.
- **AC-PL-16 [B] MUST** — plugin invocation consumes per-scope thread-pool slots per AC-A-4 and external-resource
  streams per AC-A-6. Plugins MUST NOT bypass the pool.
- **AC-PL-17 [B] SHOULD** — hosts MAY set per-plugin time/memory budgets; exceeding the budget rejects the plugin's
  promise with `PluginBudgetError` and triggers AC-PL-15.

### Lifecycle & Discovery

- **AC-PL-18 [B] MUST** — plugins are registered via descriptor objects, not by side-effecting imports.
- **AC-PL-19 [B] MUST** — plugins are installable / uninstallable at runtime per scope. In-flight invocations either
  drain to completion or are cancelled via `AbortSignal` (AC-A-7); the policy is host-selectable.
- **AC-PL-20 [B] OPEN** — plugin sandboxing model. For v0.1, plugins run with host privileges (host trusts what it
  registers). For untrusted/marketplace plugins, decide between Worker isolation, capability-restricted ctx, or
  out-of-process. Track separately.

### Verification

- **AC-PL-V-1** — SCSS-to-CSS plugin happy path: register, parse an `<style type="text/scss">` scope inside an HTML
  doc, confirm output is valid CSS and the source map maps every CSS rule back to a SCSS line/column.
- **AC-PL-V-2** — non-invasive security checker registered against `text/css` records violations without mutating the
  tree; tree hash before/after is identical.
- **AC-PL-V-3** — click-tracker plugin: every interactive node in `examples/semantic/login.html` gets a tracking ID;
  source map resolves each injected attribute back to the source element.
- **AC-PL-V-4** — inheritance: parent installs a plugin, descendant scope of matching content type sees it run;
  descendant attempt to disable rejects with `PluginInheritanceError`.
- **AC-PL-V-5** — source-map stitching: stack SCSS → CSS → click-tracker; resolving a final-output position returns a
  chain back to the original SCSS source.
- **AC-PL-V-6** — observer-only enforcement: a mutation attempt from an `observe`-mode plugin throws
  `ObserverViolationError` and the scope's tree is unchanged.

## 8. API Conventions

- **AC-A-1 [A] MUST** expose all processing as **asynchronous** APIs. No blocking variants.
- **AC-A-2 [A] MUST** model deferrable subtree or child-scope work as a Future/Promise
  attached to the owning scope processor. Resolving the owner MUST await all owned child
  scope work required for a complete parse/validate/transform result.
- **AC-A-3 [A] MUST** make Tier A child-scope completion deterministic. A single-threaded
  implementation may resolve owned child work depth-first. Parallel worker scheduling is
  Tier B and must preserve report event sequence determinism.
- **AC-A-4 [B] MUST** route processing through a **thread pool** sized
  per-scope, to prevent resource overbooking. Thread-pool size — and
  every other resource characteristic the host enforces (queue size
  per AC-A-5, external-I/O stream count per AC-A-6, per-scope memory
  caps, per-plugin time/memory budgets per AC-PL-17, etc.) — is owned
  by the **scope policy** per AC-F-1 and inherits down the scope tree
  per AC-P-4 / AC-P-5. The host's **root scope** carries a documented
  default thread-pool size:
  - in browser/WASM hosts, `min(navigator.hardwareConcurrency, 8)`,
    floored at 1;
  - in native hosts, `min(num_cpus, 8)`, floored at 1;
  - hosts MAY override the root default via the same scope-policy
    surface used by every other scope.

  Child scopes inherit the parent scope's resource caps and MAY
  **constrain further only** — a child scope MAY lower the
  thread-pool size, queue size, stream count, memory cap, or budget
  it offers to its own subtree, but MUST NOT raise any cap above what
  its parent allows. Attempts to relax (raise) an inherited cap are
  rejected at policy load with `cem.a.cap_relaxation_denied`. This
  matches the parent-override-bounds rule in AC-P-5 and makes
  resource governance one-directional: untrusted descendants cannot
  escape an ancestor's budget by installing a more permissive policy.
- **AC-A-5 [B] MUST** keep the per-scope queue size bounded; overflow policy (block / reject / spill to parent) MUST
  be documented per scope. Queue size obeys the same parent-bound
  inheritance as AC-A-4.
- **AC-A-6 [B] MUST** route **external resource I/O** (network, FS) through an event-stream queue that does **not**
  consume thread-pool slots. Stream count is also scope-bounded and
  obeys the same parent-bound inheritance as AC-A-4.
- **AC-A-7 [B] SHOULD** support cancellation via `AbortSignal` end-to-end (parser, interpreter, fetch).
- **AC-A-8 [A] MUST** use diagnostic bubbling rather than implicit per-node promise
  rejection as the canonical error propagation contract. Errors originate at the
  detecting scope, bubble to the nearest error-boundary scope, and are handled by that
  boundary scope's effective policy. API calls reject only when policy maps the outcome
  to fatal/abort/failed command behavior.

## 9. Scoped Custom-Element Registries

- **AC-R-1 [B] MUST** support scoped template/registry references for DCE/custom-element
  integration. DCE tag names are template references and machine-state bindings in the
  CEM model; CEM does not police the browser `customElements` registry in Tier A.
- **AC-R-2 [B] MUST** support inherited lookup for CEM template registries: a scope
  falls back to its parent registry if a template/tag reference is not found locally.
- **AC-R-3 [B] SHOULD** detect registry/template collisions across nested scopes and
  surface them as warnings or policy-controlled diagnostics.

## 10. Performance & Resource Budgets

- **AC-N-1 [A] MUST** parse + validate + transform any fixture in `examples/semantic/` in under **150 ms** on a
  developer-class machine (single-thread, cold cache). Benchmarked in CI with a tolerance band.
- **AC-N-2 [A] MUST** use bounded streaming accumulators. Tokenizer memory scales with
  current token and open-scope depth, not document byte length. Retained AST, report,
  source-map, line-index, reference-slot, and diagnostic structures are allowed but MUST
  be governed by documented caps or by the selected output/report projection. Verified
  by a 10 MB synthetic fixture and limit-breach diagnostics.
- **AC-N-3 [B] SHOULD** publish a benchmark suite in the Rust workspace with regressions surfaced via Nx.

## 11. Security

- **AC-X-1 [A] MUST** treat untrusted input as untrusted: no eval, no dynamic import based on content, no fetch unless
  the host explicitly opted in.
- **AC-X-2 [B] MUST** isolate scopes — a malformed/malicious child scope MUST NOT corrupt sibling or parent state.
- **AC-X-3 [A] SHOULD** flag unsafe-content patterns (inline `<script>` in CEM semantic docs, `javascript:` URIs,
  unbounded `srcdoc`) in validation.

## 12. Observability

- **AC-O-1 [A] MUST** expose a structured event stream (`onParseEvent`, `onValidate`, `onTransform`) with stable
  shapes for tooling. Implementations may expose callbacks, async streams, or report
  AST projections, but the event names and payload categories MUST remain stable.
- **AC-O-2 [B] SHOULD** support a debug mode that records a deterministic trace of thread scheduling for postmortem.
- **AC-O-3 [A] MUST** record report/log/diagnostic events as AST-associated report nodes. Each event node MUST include
  the current source module state, the source-map stack as it exists at emission time, the visible partial DOM/AST
  hierarchy at that moment, and a monotonic event sequence number.
- **AC-O-4 [A] MUST** make the internal report representation an AST tree that can be formatted into CEM-native, XML,
  JSON, or another supported structured format.
- **AC-O-5 [A] MAY** provide text and HTML report output in the reference implementation as convenience projections
  from the internal report AST. Text and HTML output MUST NOT be the canonical report model.
- **AC-O-6 [A] MUST** preserve comments, whitespace, and processing-instruction source
  positions for reporting even when a later transform strips those nodes from rendered
  output.

## 13. Compatibility & Distribution

- **AC-C-1 [A] MUST** run in modern browsers (latest 2 of Chromium, Firefox, Safari) and Node ≥ 22, with the same
  public API.
- **AC-C-2 [A] MUST** ship a Rust crate that exposes the core parser/validator/transform
  contracts and can compile to native and WASM-compatible targets.
- **AC-C-3 [A] MUST** keep the Rust crate and CLI package boundaries publishable. Any future npm/WASM wrapper must
  consume the Rust-owned contract instead of restoring the deprecated TypeScript package.

---

## 14. Content-Addressed Binary Cache & Transport

This section defines a **shared cache and transport protocol** used by every
parsed artifact in the stack: cem-ml documents, schemas, transform plans, and
cem-ql query modules (see [`cem-ql-ac.md`](cem-ql-ac.md) §14). The intent is
that a parse happens **at most once** for a given input across the build and
runtime sides; reload skips the parser, validator-frame construction, and
type-check, and resumes from a binary form keyed by content hash.

### Hashing & binary form

- **AC-CC-1 [B] MUST** assign every parsed top-level artifact a deterministic
  **content hash**. Hash inputs:
  - the canonical UTF-8 source bytes after BOM strip,
  - the artifact's content-type identifier,
  - a versioned hash-scheme tag (`cem-bin/1+blake3` for the initial scheme).
  The hash MUST be reproducible across hosts and platforms and MUST identify
  the artifact for cache reuse. Artifacts covered: cem-ml documents (parser
  output), schemas (`StructuralSchemaIr`), transform plans (AC-T-4),
  cem-ql modules.
- **AC-CC-2 [B] MUST** emit and accept a portable binary serialization of
  the parsed/typed artifact, keyed by AC-CC-1. Loading a binary whose hash
  matches an in-process or on-disk cache entry MUST skip parsing and proceed
  directly to validation / interpretation / evaluation. The binary format is
  owned by this AC and shared with downstream stacks (cem-ql) so a single
  loader implementation handles both.
- **AC-CC-3 [B] MUST** carry the artifact's declared **policy stamps** in the
  binary: declared schema URIs, declared plugin imports, declared external
  reads, and the scope-policy fingerprint under which it was produced. A
  binary whose policy stamps the active scope cannot satisfy MUST fail with
  `cem.cc.policy_mismatch` and fall back to the source when available; the
  cached binary MUST NOT be silently used under a less-restrictive policy.

### Development vs production mode

- **AC-CC-4 [B] MUST** support a `mode: "dev" | "prod"` cache axis. **Dev**
  binaries MUST preserve full **source-map stacks** per AC-O-3 / AC-P-7 so
  diagnostics from a reloaded binary are indistinguishable from diagnostics
  produced by re-parsing the source. **Prod** binaries MAY omit source-map
  stacks to reduce size.
- **AC-CC-5 [B] MUST** make source-map sidecars **independently
  content-addressed**: a dev binary references its source-map sidecar by
  AC-CC-1 hash, the sidecar is cached separately, and either side can be
  evicted and re-fetched. Cache scope is `(content-type, hash, mode)`; a
  dev artifact and a prod artifact with the same AST payload are distinct
  cache entries because their source-map content differs.

### Transport protocol

- **AC-CC-6 [B] MUST** define an HTTP(S) and `file://`-equivalent loader
  protocol expressed as headers on the source response:
  - **Response** MUST carry `CEM-Hash: <hash>` on every body that produces a
    parsed artifact. The engine uses `CEM-Hash` for **integrity check**
    (compare against AC-CC-1 recomputed locally on first load) and as the
    **cache key**.
  - **Request** MAY carry `If-CEM-Hash: <hash>` when the engine already
    holds a cached binary it would prefer to reuse.
  - When `If-CEM-Hash` matches the server's current `CEM-Hash`, the server
    MUST respond `304 Not Modified` with an empty body, the matching
    `CEM-Hash` header, and the same `Content-Type`. The engine satisfies
    the load from local cache and emits no `cem.cc.*` diagnostic. This is
    the cem-ml analogue of `ETag`/`If-None-Match`, scoped to the **parsed
    artifact** rather than the raw source bytes — two source files that
    canonicalize to the same AST share one cache entry.
  - Hash mismatch (recomputed AC-CC-1 hash ≠ declared `CEM-Hash`) MUST fail
    closed with `cem.cc.hash_mismatch`, discard any cached binary at that
    hash, and refuse to use the body.
- **AC-CC-7 [B] MUST** apply the same protocol to **secondary-content
  retrieval**: schemas, transform plans, plugin modules, external `read()`
  content per cem-ql AC-QA-1, and `<http-request>`-style template fetches.
  When the engine knows the expected hash for a secondary resource, it MAY
  send `If-CEM-Hash`; a confirmation-only `304` response is sufficient for
  the engine to satisfy the request from cache without re-downloading the
  body. Servers that cannot honor `If-CEM-Hash` MUST return the full body
  with `CEM-Hash` — degradation is graceful.
- **AC-CC-8 [B] SHOULD** expose the same protocol over `cem-ml-cli` and
  build-pipeline interfaces, not only HTTP, so a build emits binaries +
  hashes into a content-addressed store that a runtime loader consumes via
  the same `CEM-Hash` semantics.

### Tier C — chunked & shared substructure

- **AC-CC-9 [C] MAY** support **chunked binaries**: subtrees of a parse
  result are independently hashed and addressable, enabling partial cache
  reuse across documents that share template/schema/scope substructure.
  Aligns with the deferred chunk-compression and binary-transport scope
  in §0.
- **AC-CC-10 [C] MAY** support **cross-artifact deduplication**: schemas,
  source-map blobs, and reference closures shared by multiple cem-ml or
  cem-ql binaries are stored once in the cache and referenced by hash.

### Diagnostics

- **AC-CC-D-1 [B] MUST** route every cache/transport error through the host
  report AST per AC-O-3 with codes:
  `cem.cc.hash_mismatch`, `cem.cc.policy_mismatch`, `cem.cc.format_version`,
  `cem.cc.cache_evicted`, `cem.cc.source_map_missing` (dev only).

### Verification

- **AC-CC-V-1 [B]** — round-trip test: parse a fixture set in dev mode,
  serialize, evict in-memory cache, reload from binary, re-validate;
  diagnostics, source-map stacks, and `AstNodeId` identity MUST match the
  source-driven run byte-for-byte.
- **AC-CC-V-2 [B]** — hash-protocol test: a mock loader serves a cem-ml
  document. First request returns `200` + `CEM-Hash`. Second request sends
  `If-CEM-Hash`; server responds `304` with empty body; engine satisfies
  the AST from cache; assert the parser is **not** entered on pass two.
- **AC-CC-V-3 [B]** — policy-stamp test: a binary produced under a permissive
  scope policy is loaded under a stricter one; assert
  `cem.cc.policy_mismatch` and fallback to source.
- **AC-CC-V-4 [B]** — dev/prod cache-axis test: the same source parsed in
  dev and prod yields two binaries with different hashes; loading either
  satisfies its own request only.

---

## Verification Plan

A release is acceptance-tested by running:

1. `yarn nx run cem_ml:test` — unit tests covering parser, validator, interpreter, transform.
2. `yarn nx run cem_ml_cli:validate-fixtures` — runs validation across every `examples/semantic/*.html`. Exits 0 only if
   the report records zero hard violations.
3. `yarn nx run cem_ml_cli:bench` — runs parse/validate/transform benchmarks. Numbers archived per release.
4. `yarn nx run cem_ml_cli:e2e` — round-trips each fixture: parse → validate → transform → render via
   `@epa-wg/custom-element`. Snapshot compared against committed expectations.
5. Manual smoke: open the rendered fixture in a browser, confirm it renders the expected semantic surface.

Each section above contributes a concrete check to one of these scripts; AC items missing a check are not closeable.

---

## Open Questions

These must be answered before AC are testable:

1. **AC-S-6** — TS emit strategy: structural vs branded. Affects ergonomics and
   validation cost.
2. **AC-F-2** — inline schema declarations and mid-document schema
   switch/loading syntax. Stable URI/file schema loading is required; inline syntax is
   still undecided.
3. **AC-I-1 runtime phase** — public DOM `apply(transform)` API shape and whether it
   accepts URI, stream, DOM fragment, or a narrower transform-source abstraction.
4. **AC-M-9** — async-mutation rollback model (Atomic / Best-effort / Transactional). Recommended: Transactional.
5. **AC-PL-20** — plugin sandboxing model (host-trusted vs Worker isolation vs capability-restricted ctx vs
   out-of-process).
---

## 16. Tier Promotion Gates

Five Tier B/C features
cross major contract boundaries (security, concurrency, schema model,
network) and depend on each other; promoting any of them informally
puts pressure on Tier A ACs that have already shipped. Each gate
below names its preconditions, resolved OQs, entry and exit fixtures,
and cross-gate dependencies. The §13 verification plan stays
authoritative for *what* must pass; this section binds *when*
implementation work on a gated feature may begin and *when* the
feature is considered shippable at its tier.

### 16.0 Gate framework

- **AC-G-1 MUST** — every Tier B/C AC item that implements a feature
  named in §16 MUST cite the relevant gate identifier (`G-EXT`,
  `G-PLUG`, `G-NVDL`, `G-MUT`, `G-HYD`) in its body. Implementation
  work MUST NOT begin against a gated AC item until its gate is
  **open**. New Tier B/C ACs added later that fall inside a gate's
  scope inherit the gate by virtue of citing it.

- **AC-G-2 MUST** — a gate is **open** when *all* of the following
  hold:
  1. every AC listed in its `Required closed prior-tier ACs` field
     passes its §13 verification script with **no recorded waiver**;
  2. every Open Question listed in its `Required resolved OQs`
     field has a committed answer in §15;
  3. every upstream gate listed in its `Depends on gates` field is
     itself open;
  4. its `Entry fixture` passes.

  A gate transitions from open to **closed** when its `Exit fixture`
  passes; a closed gate is the formal "this Tier B/C feature is
  shippable at its tier" signal. Closed gates remain closed unless
  demoted per AC-G-4.

- **AC-G-3 MUST** — gate state (open / closed / demoted) is recorded
  in `cem-ml-stack-design-impl.md` once that document grows a "Gate
  status" subsection. Until then, gate state is recorded in PR
  descriptions that touch a gated AC item, and the head-of-`develop`
  state is the source of truth.

- **AC-G-4 MUST** — a gate that fails its entry or exit fixture, or
  whose `Required closed` ACs regress on a green-to-red verification
  run, MUST be **demoted** to *closed-pending*: in-flight work on
  that gate's ACs pauses, the failing condition is filed as a
  release blocker, and downstream gates that depend on it are
  demoted in turn. Demotion does not delete code already merged
  behind the gate; it stops new work and falls back to Tier A
  snapshot baselines for affected fixtures until the regression is
  fixed.

- **AC-G-5 SHOULD** — `cem-ql-ac.md` Tier B AC items that depend on
  a host gate (notably `read()` per cem-ql AC-QA-1, `AbortSignal`
  propagation per cem-ql AC-QA-3, network-scheme imports per
  cem-ql AC-QI-2 / AC-QI-4, and the §14 cache transport per
  cem-ql AC-QC-*) cite the same gate identifier rather than
  re-asserting the precondition. This keeps the two ACs from
  drifting and lets a single PR open the gate on both sides.

### 16.1 Cross-gate dependency graph

```
                [Tier A close]
                       │
                       ▼
                    G-EXT  ──────────────────┐
                       │                     │
                       ▼                     │  (also unblocks
                    G-PLUG  ─────┐           │   cem-ql AC-QA-*,
                       │         │           │   cem-ql AC-QI-2/4,
                       ▼         ▼           │   cem-ql AC-QC-*)
                    G-NVDL    G-MUT          │
                                 │           │
                                 ▼           │
                              G-HYD ◀────────┘
                              (also depends on G-EXT for
                               network-driven hydration)
```

`G-EXT` is the foundational gate; nothing else opens until it does.
`G-NVDL` and `G-MUT` are independent of each other and may open in
either order once `G-PLUG` is open. `G-HYD` is the deepest gate and
requires all others to have at least entered the open state.

### 16.2 G-EXT — External-resource loading (Tier B)

Covers the AC-A-6 I/O queue, scope-policy grant model for fetches,
content-type registry resolution at fetch time, and the cem-ql
`read()` surface (cem-ql AC-QA-1 / AC-QA-1.1).

- **Required closed prior-tier ACs**: AC-A-1, AC-A-2, AC-A-3, AC-A-8
  (async API foundation + diagnostic bubbling); AC-X-1, AC-X-3
  (untrusted-input handling); AC-O-3 (report routing); AC-F-1
  (scope policy surface); AC-P-3, AC-P-4, AC-P-7 (parser diagnostic
  / context-scope / source-map contracts the I/O queue inherits).
- **Required resolved OQs**: none. AC-A-6 is normative; this gate
  enforces its preconditions but does not need a §15 decision.
- **Entry fixture**: a single `file://` fixture loads through
  AC-A-6's I/O queue under a Tier B scope policy that grants
  `file://fixtures/`; a denied scheme on the same call site routes
  through AC-O-3 with the originating scope context attached and
  the documented `cem.ext.fetch_denied` code.
- **Exit fixture**: HTTP and `file://` fetches succeed and fail
  correctly under three documented scope policies (deny-all,
  allow-by-prefix, allow-with-budget); cancellation via
  `AbortSignal` (AC-A-7) cleanly aborts pending requests with no
  leaked queue slots; benchmark publication per AC-N-3 shows queue
  overhead within the AC-N-1 150 ms budget when no fetches are
  queued; the AC-CC-6 / AC-CC-7 transport `If-CEM-Hash` round-trip
  succeeds for a content-type-dispatched secondary artifact.
- **Depends on gates**: none (root gate after Tier A close).
- **Downstream cem-ql impact**: opens cem-ql AC-QA-1 / AC-QA-2 /
  AC-QA-3 / AC-QA-V-1, cem-ql AC-QI-2 network tier, and cem-ql
  AC-QC-4 transport participation.

### 16.3 G-PLUG — Plugin runtime (Tier B)

Covers §7 (AC-PL-1..AC-PL-19), the per-scope plugin chain, observe /
mutate mode separation, source-map stitching, and per-scope
resource budgeting for plugin invocations.

- **Required closed prior-tier ACs**: AC-PL-1..AC-PL-19 (plugin
  surface and chain semantics); AC-A-4, AC-A-5 (per-scope thread
  pool, queue overflow policy); AC-X-1, AC-X-2 (untrusted input,
  scope isolation); AC-O-1, AC-O-3 (event stream, report routing);
  AC-T-1 (transform contract that plugins compose with).
- **Required resolved OQs**: OQ 5 (AC-PL-20 plugin sandboxing
  model). Thread-pool default and per-scope cap inheritance are
  normative under AC-A-4.
- **Entry fixture**: AC-PL-V-1 (SCSS-to-CSS plugin happy path)
  passes end-to-end through the public async API with a stitched
  source map per AC-PL-12.
- **Exit fixture**: AC-PL-V-1..AC-PL-V-6 all pass; an out-of-scope
  mutation attempt from a registered plugin is rejected at the
  trust boundary with the documented diagnostic and does not
  corrupt sibling-scope state per AC-X-2; a 100-plugin chain on a
  single scope stays under per-plugin budget per AC-PL-17 without
  exhausting the scope's thread-pool slots; observer-only
  enforcement (AC-PL-V-6) holds under the chosen sandboxing model.
- **Depends on gates**: G-EXT (plugin descriptors and observe-mode
  fetches use the AC-A-6 I/O queue; observe plugins MAY emit
  external requests under their scope's policy).

### 16.4 G-NVDL — NVDL schema dispatch (Tier C)

Covers AC-P-6 (NVDL-style mid-document schema dispatch),
namespace-driven schema switching, and per-scope schema identity
under multi-content scopes.

- **Required closed prior-tier ACs**: AC-P-4, AC-P-5 (context
  scopes and nesting); AC-V-1, AC-V-2, AC-V-3, AC-V-7, AC-V-8
  (validation contracts, semver behavior, open-content policy,
  recovery model); AC-V-9..AC-V-13 (§3.1 schema version identity —
  required so namespace-dispatched schemas resolve to the same
  embedded version across hosts); AC-I-2 (content-type switching,
  the Tier B expansion of which is itself a precondition);
  AC-CC-1, AC-CC-3 (binary cache hash and policy-stamp surface so
  schema id changes invalidate cache entries correctly); AC-T-4
  (schema-owned transform plans).
- **Required resolved OQs**: none (the schema URI / version
  syntax precondition is now normative under §3.1).
- **Entry fixture**: a single fixture parses with two schemas
  dispatched by namespace inside one document; source-map stacks
  span the boundary cleanly per AC-P-7; the AC-O-3 report tree
  shows diagnostics from both schemas attached to the originating
  scope.
- **Exit fixture**: at least three documented namespace dispatches
  inside one parse; per-scope policies inherit and override
  correctly across boundaries per AC-P-5; binary-cache reuse
  (AC-CC-1) succeeds across two hosts that have the same schema
  set installed and fails with `cem.cc.policy_mismatch` when one
  host is missing a dispatched schema.
- **Depends on gates**: G-PLUG (NVDL-driven dispatch may invoke
  plugin chains owned by the dispatched schema); G-EXT (schemas
  may be loaded externally under the AC-A-6 I/O queue).

### 16.5 G-MUT — DOM mutation API (Tier C)

Covers §5 (AC-M-1..AC-M-14), the async mutation surface over the
sync DOM, queue ordering, batch coalescing, observer dispatch, and
rollback discipline.

- **Required closed prior-tier ACs**: AC-T-1, AC-T-3 (transform
  contract — mutation is layered over transforms, not under them);
  AC-P-7 (source-map stability under replay — mutations re-emit
  source frames per AC-PL-13's stitching model); AC-A-1..AC-A-7
  (async + abort + queue + I/O — every mutator is async and
  cancellable); AC-O-3 (report routing for AC-M-10 rejection
  errors); AC-CC-1, AC-CC-2, AC-CC-3 (binary cache hash and
  serialization stable across pre- and post-mutation snapshots so
  a mutated tree can re-hash deterministically); AC-I-6 (WHATWG
  DOM compliance — mutation targets the schema-driven projection,
  not a fork of it).
- **Required resolved OQs**: OQ 4 (AC-M-9 rollback model:
  Atomic / Best-effort / Transactional — recommended Transactional).
- **Entry fixture**: AC-M-V-1 (round-trip test on a single
  `examples/semantic/` fixture) passes; the read-only invariant
  cem-ql AC-Q-2 holds — queries see the mutated state but cannot
  themselves write.
- **Exit fixture**: AC-M-V-1..AC-M-V-4 all pass (round-trip,
  ordering, abort, observer batching); rollback behaves per the
  OQ-4 resolution under three induced failure modes (schema
  validation failure per AC-M-11, plugin rejection per AC-PL-15,
  cancelled mutation per AC-M-7); `flushAsync(scope?)` (AC-M-14)
  preserves submission order per AC-M-5 when used.
- **Depends on gates**: G-PLUG (mutation may be initiated by
  mutate-mode plugins per AC-PL-4 / AC-PL-15).

### 16.6 G-HYD — Live hydration / render-while-parsing (Tier C)

Covers AC-I-4 (render-while-parsing), AC-I-5 (batch policy), and
the integration of mutation queues with the rendering pipeline so
visible state can change during parse.

- **Required closed prior-tier ACs**: AC-I-4, AC-I-5 (render
  before EOF, batch flush policy); AC-M-* (every mutator —
  hydration writes through the mutation queue, not bypassing it);
  AC-O-1 (event stream so hosts can drive UI off render events);
  AC-O-2 (deterministic scheduling trace for postmortem); AC-N-1
  (150 ms first-paint budget, which hydration MUST respect under
  the `examples/semantic/` fixture set).
- **Required resolved OQs**: none (render-batch policy is
  host-defined per AC-I-5 and inherited per AC-A-4).
- **Entry fixture**: a single `examples/semantic/` fixture renders
  visible state before EOF on its input stream per AC-I-4;
  `MutationRecord` batching behaves per AC-M-V-4 across the parse
  → hydration boundary.
- **Exit fixture**: render-while-parsing on **all** five
  `examples/semantic/*.html` fixtures stays under the AC-N-1
  first-paint budget; the AC-O-2 debug trace records scheduling
  deterministically across two runs; cancellation mid-render via
  `AbortSignal` leaves the DOM in a consistent state per the
  OQ-4 rollback model and emits the documented `cem.hyd.aborted`
  diagnostic.
- **Depends on gates**: G-MUT (live hydration writes through the
  async mutation queue); G-EXT (network-driven hydration cases —
  e.g. resources fetched mid-stream — use the AC-A-6 I/O queue).

---

## References

- NVDL — Namespace-based Validation Dispatching Language: <https://en.wikipedia.org/wiki/Namespace-based_Validation_Dispatching_Language>, <https://nvdl.oxygenxml.com/>
- RELAX-NG schema for XSLT 4.0: <https://qt4cg.org/specifications/xslt-40/schema-for-xslt40.rnc>
- iXml — Invisible XML: <https://invisiblexml.org/1.0/ixml.xml.html>
- HTML5 in RELAX-NG: <https://github.com/validator/validator/blob/main/schema/html5/html5.rnc>
- `@epa-wg/custom-element` — runtime target for transformed output (workspace dep).
- Design docs: [`cem-ml-stack-design.md`](cem-ml-stack-design.md),
  [`cem-ml-stack-design-impl.md`](cem-ml-stack-design-impl.md).
- Query language AC: [`cem-ql-ac.md`](cem-ql-ac.md) — normative surface for the
  scoped query language consumed by AC-T-1 / AC-T-2 / AC-T-3 / AC-T-7 and the
  `ScopedQueryLanguage::CemScopedQuery` placeholder. AC-T-7 owns the
  CEM-ML template embedding side; cem-ql owns the expression grammar.
- Companion docs: [`cem-ml-library-plan.md`](cem-ml-library-plan.md), [`component-mvp.md`](component-mvp.md),
  [`todo.md`](todo.md).
