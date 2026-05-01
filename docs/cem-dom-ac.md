# `@epa-wg/cem-dom` — Acceptance Criteria

This document captures the acceptance criteria (AC) for `@epa-wg/cem-dom`. Each item is phrased as a checkable
statement so it can be referenced from `docs/todo.md` and from PR descriptions. Every requirement uses MUST / SHOULD /
MAY in the RFC 2119 sense.

> **Status legend**
>
> - **MUST** — required for the package to be considered shippable.
> - **SHOULD** — required unless an explicit waiver is recorded in this file.
> - **MAY** — explicitly optional; in scope for a later release.
> - **OPEN** — needs a decision before AC can be tested.

## Goal

`@epa-wg/cem-dom` is a parser + schema + interpreter stack that replaces, with functional parity:

- HTML / XML / XSLT / XPath / SVG / MathML / Canvas / SMIL parsing.
- XSLT template-engine evaluation.
- Virtual-DOM-style DOM update reconciliation.
- Scoped registries for Declarative Custom Elements (DCE).

It is the document layer for CEM: every CEM artifact (semantic fixtures, component templates, design docs,
transforms) flows through it.

## Conformance Tiers

The stack is large enough that one binary AC list would never finish. Three tiers:

- **Tier A — MVP.** Parses the existing five `examples/semantic/*.html` fixtures, validates them against a CEM schema,
  transforms them to light-DOM custom-element markup. Single-threaded async is acceptable. No multi-namespace
  switching.
- **Tier B — Multi-content.** Adds embedded content-type switching (CSS-in-HTML, SVG-in-HTML, JS template literal as
  HTML island, etc.), scoped registries, real thread-pool execution, and external-resource fetch.
- **Tier C — Full vision.** XSLT 4.0 / NVDL dispatch / iXml / Canvas / SMIL / MathML parity, Rust + TS type-header
  emission, streaming render-while-parsing with observable batching.

Each AC below is tagged `[A]`, `[B]`, or `[C]`. Initial release closes Tier A and explicitly lists which Tier B/C items
are deferred.

---

## 1. Parser

- **AC-P-1 [A] MUST** parse XML 1.0 well-formed documents and HTML5 documents into the same in-memory DOM model. Both
  surface the same node, attribute, and text APIs.
- **AC-P-2 [A] MUST** be a *streaming* parser: it accepts `ReadableStream<Uint8Array | string>` as input and emits
  parse events incrementally. Memory usage during parse MUST be bounded by current open-element depth, not document
  size.
- **AC-P-3 [A] MUST** report parse errors with `{ uri, line, column, byteOffset, code, severity, message }`. Errors do
  not abort the stream unless severity is `fatal`.
- **AC-P-4 [B] MUST** support **scopes** with three-tuple identity `{ schemaUri, contentType, namespaceUri }`. Each
  scope is its own error boundary: errors inside a scope MUST NOT propagate beyond it unless explicitly re-raised.
- **AC-P-5 [B] MUST** allow scopes to nest — a scope can contain child scopes of a different content type (e.g.
  `text/css` inside `text/html`).
- **AC-P-6 [C] MUST** dispatch between schemas mid-document via NVDL-style rules (see References).
- **AC-P-7 [A] SHOULD** preserve byte offsets on every node for source-map-quality diagnostics.
- **AC-P-8 [A] OPEN** — pick the canonical CEM document syntax. Options under evaluation: iXml, RELAX-NG compact, a
  CEM-native syntax. Decision blocks AC-S-1.

## 2. Schema

- **AC-S-1 [A] MUST** define schemas in **CEM-native syntax** as the source of truth.
- **AC-S-2 [A] MUST** emit, on each release, equivalent **XML schema** (XSD or RELAX-NG — pick one in `AC-S-2-OPEN`)
  alongside the native form. The two MUST be byte-stable for unchanged input.
- **AC-S-3 [A] MUST** emit type headers in **TypeScript** (`.d.ts`) that mirror schema element/attribute shapes.
- **AC-S-4 [B] SHOULD** emit type headers in **Rust** (`.rs`) for native consumers.
- **AC-S-5 [A] MUST** expose schemas at stable URIs so namespace declarations in documents can resolve them.
- **AC-S-6 [A] OPEN** — pick TS-emit strategy: structural types vs branded nominal types. Affects ergonomics and
  validation cost.

## 3. Validation & Strict Typing

- **AC-V-1 [A] MUST** validate documents against schemas and surface errors through every layer of the
  parser/transpiler stack — i.e. an XSLT consumer of a parsed tree sees the same error objects the parser produced.
- **AC-V-2 [A] MUST** be **forgiving by default within a semver-compatible namespace**. If the document declares
  schema version X.Y and the loaded schema is X.Y′ (Y′ ≥ Y), unknown elements/attributes produce **warnings, not
  errors**.
- **AC-V-3 [A] MUST** be **strict on major mismatch**. If the document pins schema X and the loaded schema is X+1,
  validation MUST fail loudly and abort the scope.
- **AC-V-4 [A] MUST** emit a machine-readable validation report (`*.report.json`) and a human-readable
  `*.report.md`, mirroring the convention in `packages/cem-theme/scripts/validate-platforms.mjs`.
- **AC-V-5 [A] MUST** validate the existing `examples/semantic/*.html` fixtures with **zero hard violations**.
- **AC-V-6 [A] SHOULD** detect: unknown elements/attrs, invalid state combinations, missing accessible names, broken
  `id`/`for`/`aria-*` references, unsafe inline content.

## 4. Interpreter & DOM State Machine

The implementation MUST split:

- **Parser** — content → events / typed nodes.
- **Interpreter** — events → state-machine transitions on a DOM subtree.

Default interpreter materializes nodes as a WHATWG-compatible DOM.

- **AC-I-1 [A] MUST** treat the interpreted DOM as a state machine with an explicit `apply(transform)` operation. The
  transform argument MUST accept a URI, a stream, or a DOM fragment.
- **AC-I-2 [B] MUST** switch interpreters when content-type changes — e.g. encountering an inline `<style>` switches to
  the CSS interpreter for that subtree, which can in turn host another content type (CSS `url(...)` referencing SVG).
- **AC-I-3 [A] MUST** allow interpreters to own a subtree exclusively. Mutations from outside the owning interpreter
  MUST be rejected or routed through the owner's `apply()`.
- **AC-I-4 [B] MUST** support render-while-parsing: an interpreter can emit visible state before EOF on its input
  stream. Examples that drive this requirement: top-level-await scripts, HTML with external images/CSS.
- **AC-I-5 [B] SHOULD** batch render updates. Default policy: flush on stream completion, OR after 100 ms since first
  pending update — whichever comes first. Policy is configurable per scope.
- **AC-I-6 [A] OPEN** — choose between WHATWG-DOM compat (full surface) vs minimal subset. Full compat is large;
  subset risks consumer rewrites.

## 5. DOM Mutation API — Async Layer Over Sync Surface

The synchronous WHATWG DOM (`appendChild`, `setAttribute`, `textContent =`, `innerHTML =`, `replaceWith`, …) remains
available for backward compatibility, but every mutation also has an **async counterpart** that participates in the
interpreter's state machine, scope thread pool, and batching policy. Reads stay synchronous — the DOM is observed as a
snapshot.

### Surface

- **AC-M-1 [A] MUST** expose an async counterpart for each mutating operation in the DOM core. Naming convention:
  suffix `Async`. Initial set (Tier A):
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
- **AC-M-2 [A] MUST** return `Promise<void>` from every `*Async` mutator. Resolution means the mutation is committed
  to the DOM **and** the owning interpreter has finished any cascading work (style recalc, scoped registry updates,
  child-scope spawning).
- **AC-M-3 [A] SHOULD** keep the sync mutators available, but route them through a fast path that is functionally
  equivalent to `await mutator()` followed by a synchronous flush. Sync mutators MUST NOT be used inside an
  interpreter that owns a subtree under active streaming — they SHOULD throw `OwnedSubtreeError` if attempted.

### Semantics

- **AC-M-4 [A] MUST** route every async mutation through the owning interpreter's queue (per AC-I-3, AC-A-4).
  Mutations from outside the owning scope SHOULD be rejected with `ScopeViolationError`.
- **AC-M-5 [A] MUST** preserve **submission order** within a single owner: if `appendChildAsync(a)` is called before
  `appendChildAsync(b)` against the same parent, `a` MUST settle before `b`. Cross-parent ordering is not guaranteed.
- **AC-M-6 [B] SHOULD** **coalesce** mutations that fall within the same batch window (per AC-I-5) into a single
  observer notification — multiple `setAttributeAsync` calls on the same node within 100 ms surface as one
  `MutationRecord`. The Promise of each call resolves only once the merged batch flushes.
- **AC-M-7 [A] MUST** support `AbortSignal` on every `*Async` mutator. Aborting before the queued mutation begins
  rejects the promise with `DOMException("Aborted", "AbortError")` and skips the mutation. Aborting after work has
  begun follows the rollback policy in AC-M-9.
- **AC-M-8 [A] MUST** dispatch `MutationObserver` callbacks **after** the batch flushes, not at promise-resolution
  time. Observer callbacks see a consistent post-batch DOM.

### Errors & Rollback

- **AC-M-9 [A] OPEN** — rollback contract on rejection. Three candidate models:
  - *Atomic*: a rejected `*Async` mutation leaves the DOM unchanged.
  - *Best-effort*: partial application is allowed; the rejection carries the post-state.
  - *Transactional*: mutations declared inside `withTransaction(async () => …)` are atomic; bare mutators are
    best-effort.
  - Recommendation: pick **Transactional** because it matches both interpreter ownership and the parser's
    error-boundary model. Confirm before implementation.
- **AC-M-10 [A] MUST** route rejected mutations through the same error stream as parser/validator errors
  (`onParseEvent`, AC-O-1) so consumers see one error pipeline.
- **AC-M-11 [B] SHOULD** validate the post-mutation tree against the active schema before commit. A schema violation
  rejects the promise with `SchemaViolationError` and rolls back per AC-M-9.

### Concurrency & Resources

- **AC-M-12 [B] MUST** count mutation work against the same per-scope thread-pool slot as parsing/transform work
  (AC-A-4). Mutations MUST NOT bypass the pool by going synchronous under the covers.
- **AC-M-13 [B] SHOULD** allow **read amid pending writes**: synchronous reads see the last committed state, never an
  in-flight intermediate. Reads inside the same microtask after an awaited mutation MUST observe that mutation.
- **AC-M-14 [B] MAY** offer `flushAsync(scope?)` to force pending mutations to commit immediately, bypassing the
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

- **AC-T-1 [A] MUST** apply XSLT-equivalent transformations to a DOM subtree. Tier A may use a subset (templates,
  match, apply-templates, value-of, copy, copy-of, attribute-value-templates).
- **AC-T-2 [A] MUST** transform every fixture in `examples/semantic/*.html` to light-DOM custom-element markup
  compatible with `@epa-wg/custom-element` and snapshot the output.
- **AC-T-3 [C] SHOULD** evaluate XSLT 4.0 surface (see `qt4cg.org` reference) for completeness.
- **AC-T-4 [A] MUST** support transforms loaded from URI, from a `ReadableStream`, or from an in-memory DOM.

## 7. Transformation Plugins

The built-in stack covers HTML / XML / CSS / SVG / XSLT (Tiers A–C). Real projects need content types the platform
does not ship — SCSS, TypeScript, JSX, Markdown variants, project-specific DSLs — and cross-cutting concerns that
augment any content type (security checks, click tracking, telemetry). Plugins extend the transformation chain
without forking the runtime.

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
- **AC-PL-11 [B] MUST** — built-in transformers (HTML parser, CSS parser, XSLT, etc.) are addressable as plugins via
  the same descriptor surface. One model, one chain.

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
  the plugin. Failure mode for the rest of the chain (skip downstream / abort scope) follows the rollback contract
  in AC-M-9.
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
- **AC-A-2 [A] MUST** model deferrable subtree work as a Promise attached to the parent node's processor. Resolving
  the parent MUST await all owned subtree promises.
- **AC-A-3 [A] MUST** resolve thread work in **depth-first order**. Tests MUST verify ordering invariants.
- **AC-A-4 [B] MUST** route processing through a **thread pool** sized per-scope, to prevent resource overbooking.
  Default size is documented and configurable.
- **AC-A-5 [B] MUST** keep the per-scope queue size bounded; overflow policy (block / reject / spill to parent) MUST
  be documented per scope.
- **AC-A-6 [B] MUST** route **external resource I/O** (network, FS) through an event-stream queue that does **not**
  consume thread-pool slots. Stream count is also scope-bounded.
- **AC-A-7 [B] SHOULD** support cancellation via `AbortSignal` end-to-end (parser, interpreter, fetch).
- **AC-A-8 [A] OPEN** — error propagation contract: do per-node errors appear as rejected promises on that node only,
  or also bubble to the root? Affects every consumer.

## 9. Scoped Custom-Element Registries

- **AC-R-1 [B] MUST** support DCE registries scoped to a parser scope. Registrations in a child scope MUST NOT leak to
  the parent.
- **AC-R-2 [B] MUST** support inherited lookup: a scope falls back to its parent registry if a tag is not found
  locally.
- **AC-R-3 [B] SHOULD** detect registry collisions across nested scopes and surface them as warnings.

## 10. Performance & Resource Budgets

- **AC-N-1 [A] MUST** parse + validate + transform any fixture in `examples/semantic/` in under **150 ms** on a
  developer-class machine (single-thread, cold cache). Benchmarked in CI with a tolerance band.
- **AC-N-2 [A] MUST** use bounded memory during streaming — peak heap during parse MUST scale with **open-element
  depth**, not document byte length. Verified by a 10 MB synthetic fixture.
- **AC-N-3 [B] SHOULD** publish a benchmark suite (`packages/cem-dom/bench/`) with regressions surfaced via Nx Cloud.

## 11. Security

- **AC-X-1 [A] MUST** treat untrusted input as untrusted: no eval, no dynamic import based on content, no fetch unless
  the host explicitly opted in.
- **AC-X-2 [B] MUST** isolate scopes — a malformed/malicious child scope MUST NOT corrupt sibling or parent state.
- **AC-X-3 [A] SHOULD** flag unsafe-content patterns (inline `<script>` in CEM semantic docs, `javascript:` URIs,
  unbounded `srcdoc`) in validation.

## 12. Observability

- **AC-O-1 [A] MUST** expose a structured event stream (`onParseEvent`, `onValidate`, `onTransform`) with stable
  shapes for tooling.
- **AC-O-2 [B] SHOULD** support a debug mode that records a deterministic trace of thread scheduling for postmortem.

## 13. Compatibility & Distribution

- **AC-C-1 [A] MUST** run in modern browsers (latest 2 of Chromium, Firefox, Safari) and Node ≥ 22, with the same
  public API.
- **AC-C-2 [B] SHOULD** ship a Rust crate that exposes the core parser/validator (compiled to native + WASM).
- **AC-C-3 [A] MUST** publish to npm as `@epa-wg/cem-dom` with ESM + types; `package.json` exports map mirrors
  `@epa-wg/cem-components` shape.

---

## Verification Plan

A release is acceptance-tested by running:

1. `yarn nx test cem-dom` — unit tests covering parser, validator, interpreter, transform.
2. `yarn nx run cem-dom:validate-fixtures` — runs validation across every `examples/semantic/*.html`. Exits 0 only if
   the report records zero hard violations.
3. `yarn nx run cem-dom:bench` — runs parse/validate/transform benchmarks. Numbers archived per release.
4. `yarn nx run cem-dom:e2e` — round-trips each fixture: parse → validate → transform → render via
   `@epa-wg/custom-element`. Snapshot compared against committed expectations.
5. Manual smoke: open the rendered fixture in a browser, confirm it renders the expected semantic surface.

Each section above contributes a concrete check to one of these scripts; AC items missing a check are not closeable.

---

## Open Questions

These must be answered before AC are testable:

1. **AC-P-8** — canonical CEM schema syntax (iXml / RELAX-NG / native).
2. **AC-S-2-OPEN** — XML schema mirror format (XSD vs RELAX-NG).
3. **AC-S-6** — TS emit strategy (structural vs branded).
4. **AC-I-6** — DOM API surface (full WHATWG vs subset).
5. **AC-A-8** — per-node vs bubbling error propagation contract.
6. **AC-M-9** — async-mutation rollback model (Atomic / Best-effort / Transactional). Recommended: Transactional.
7. **AC-PL-20** — plugin sandboxing model (host-trusted vs Worker isolation vs capability-restricted ctx vs
   out-of-process).
8. **Tier-A boundary** — explicit deferral list of Tier B/C items at v0.1 ship.
9. **Render policy default** — confirm the 100 ms batch window applies broadly or only to first paint.
10. **Thread-pool default size** — `navigator.hardwareConcurrency` vs fixed cap per scope.

---

## References

- NVDL — Namespace-based Validation Dispatching Language: <https://en.wikipedia.org/wiki/Namespace-based_Validation_Dispatching_Language>, <https://nvdl.oxygenxml.com/>
- RELAX-NG schema for XSLT 4.0: <https://qt4cg.org/specifications/xslt-40/schema-for-xslt40.rnc>
- iXml — Invisible XML: <https://invisiblexml.org/1.0/ixml.xml.html>
- HTML5 in RELAX-NG: <https://github.com/validator/validator/blob/main/schema/html5/html5.rnc>
- `@epa-wg/custom-element` — runtime target for transformed output (workspace dep).
- Companion docs: [`dom-library-plan.md`](dom-library-plan.md), [`component-mvp.md`](component-mvp.md),
  [`todo.md`](todo.md).
