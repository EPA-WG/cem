# `cem-element` Design

**Status:** Design doc for the `<cem-element>` declarative custom-element substrate.
Pairs with the parser/runtime work in [`cem-ml-stack-design.md`](./cem-ml-stack-design.md),
the query/template surface in [`cem-ql-stack-design.md`](./cem-ql-stack-design.md), and the
component contracts in [`packages/cem-components/docs/`](../packages/cem-components/docs/).

This document is the source of truth for the `cem-element` substrate that
`@epa-wg/cem-components` builds on. It supersedes the
`<custom-element>` authoring tag from `@epa-wg/custom-element` while preserving the
declarative concept that POC introduced.

## 1. Goal

`cem-element` keeps the `@epa-wg/custom-element` concept — a declaration registers a
custom element whose instances hold a **data island**, wire DOM events to data-change
updates, and re-render visible light-DOM output from template + data — and replaces
the template engine with CEM-native syntax:

- The `<cem-element>` declaration carries its template source in one associated
  WHATWG `<template>` child. That template is authored in canonical **CEM-ML**
  (curly-brace) or its XML/HTML parity surface; both lower into the same event/AST
  model owned by `cem_ml`.
- Expressions inside templates and attribute-value spans use **CEM-QL**, replacing
  XPath as the data-access language.
- A produced custom element instance owns the mutable data island. That instance data
  island is also wrapped in a WHATWG `<template>` so its contents sit in an inert
  `template.content` DocumentFragment and never reach the live render tree. Only the
  rendered output driven from that instance data island is visible.

`cem-element` is **not** a fork of `<custom-element>`. It is the new substrate that
`<custom-element>` will inherit from. The end state is that `@epa-wg/custom-element`
continues to publish the `<custom-element>` tag, but its implementation is rebuilt on
the `cem-element` substrate. The public attributes will be revised during that major
version. `@epa-wg/custom-element` will be published from this monorepo as its new
home, and https://github.com/EPA-WG/custom-element will be deprecated.

## 2. Packages

| Package                           | Status                                           | Role                                                                                                                                                                                                                            |
|-----------------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `@epa-wg/cem-elements`            | Planned, this design                             | Houses the `<cem-element>` runtime and its declarative authoring surface. Plural ("elements") refers the functional components as opposite to `@epa-wg/cem-components` UI library that consumes it.                             |
| `@epa-wg/cem-components`          | Phase 3, contract docs landed                    | Declarative component primitives (`cem-button`, `cem-input`, …) authored with `<cem-element>` and the conventions in [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md).           |
| `@epa-wg/custom-element`          | External today, scheduled for monorepo migration | Existing POC at `~/aWork/custom-element/`. Source moves into `packages/custom-element/`; future major keeps publishing `<custom-element>` and implements it by inheriting the `cem-element` substrate. XSLT syntax preservation TBD. |
| `custom-element-dist` (reference) | External                                         | Material-style sample components at `~/aWork/custom-element-dist/src/material/` (`action`, `autocomplete`, `badge`, `dropdown`, `icon`, `icon-link`, `input`, `menu`). Used as the parity benchmark for `cem-element` (see §7). |

## 3. Authoring surface

Terminology used below:

- **Declaration element** means `<cem-element>`. It declares/registers a custom
  element tag and owns the CEM-ML template source. The (scoped) custom elements registry use TBD. The use of unique tag names is TBD.
- **Declaration template** means the single direct-child WHATWG `<template>` inside
  `<cem-element>`. It is inert browser content, but it is not the mutable runtime
  data island.
- **Produced custom element instance** means an instance of the declared tag, such as
  `<cem-button>` or `<cem-menu>`. This is not the legacy `<custom-element>` tag.
- **Instance data island** means the produced custom element instance's inert
  `<template data-cem-island="instance">`, which stores mutable attributes, payload,
  slices, validation state, and event payloads.

A `<cem-element>` declaration has one direct child: the WHATWG `<template>` that
contains the declaration's CEM-ML template source. This declaration template is not
the mutable runtime data island. The custom element instances produced by the
declaration (`<cem-button>`, `<cem-menu>`, etc.) own the data island.

Before upgrade, a produced custom element instance may contain author fallback
payload. On upgrade, that payload is captured into the instance's inert data-island
`<template>`, and only the rendered projection remains visible.

```html
<cem-element tag="cem-button">
  <template>
    {attribute @name="disabled"}
    {attribute @name="busy"}
    {attribute @name="label" | Save}

    {button
      @disabled={$disabled}
      @aria-busy={$busy}
      | ${$label}
    }
  </template>
</cem-element>
```

Or the XML/HTML parity form (lowered to the same AST):

```html
<cem-element tag="cem-button">
  <template>
    <attribute name="disabled" />
    <attribute name="busy" />
    <attribute name="label">Save</attribute>

    <button disabled="{$disabled}" aria-busy="{$busy}">${$label}</button>
  </template>
</cem-element>
```

### 3.1 Declaration template vs. instance data island

- Every `<cem-element>` declaration MUST contain exactly one direct-child WHATWG
  `<template>` element. Declaration content outside that wrapper is invalid, because
  it would be live page content instead of declaration template source.
- The browser parks `<template>` content in `template.content` (a `DocumentFragment`)
  and does not render it. For the declaration template this means:
  - inner text never bleeds into the live page;
  - inner elements never affect layout;
  - inner attributes never reach selectors;
  - the declaration source is **inert by default** without any author opt-in.
- The cem-element runtime reads the declaration template's `template.content` at
  upgrade time, lowers it to the same `NormalizedEvent` stream `cem_ml` already
  produces, and runs it through the configured schema/scope policy.
- Multiple top-level concerns (attribute declarations, slices, named render templates,
  inline styles, plugin descriptors) coexist inside the single `<template>` — they are
  distinguished by element name, not by sibling position.
- For each produced custom element instance, the runtime creates or reuses a separate
  instance data island as `<template data-cem-island="instance">`. Host attributes,
  dataset, captured author payload, slice state, validation state, and event payloads
  live there. Its content is the mutable data host for that instance and MUST NOT
  participate in rendering directly.
- Author payload on the produced custom element instance (`<cem-button>Save</cem-button>`)
  is a progressive-enhancement fallback only until upgrade. During upgrade it is
  moved or cloned into the instance data-island template before the rendered output
  is installed, so the page never shows both the raw payload and rendered projection.

### 3.2 Template engine

| Concern                   | `<custom-element>` legacy                               | `<cem-element>`                                                                                                                 |
|---------------------------|---------------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------|
| Template syntax           | XSLT-shaped HTML with `<for-each>`, `<if>`, `<choose>`  | CEM-ML curly surface or XML/HTML parity; `cem-ql` template embedding (AC-T-7)                                                   |
| Expression language       | XPath 1.0, `$var` and `//path`                          | CEM-QL (see [`cem-ql-stack-design.md`](./cem-ql-stack-design.md)); `$var` for declared attributes, dotted/path forms for slices |
| Text interpolation        | `{ … }` in text and attribute values                    | `{ $expr }` in attributes (AVT spans); `${ $expr }` in text. Bare `{ … }` text is rejected per `cem-ml-syntax.md` Tier A.       |
| Attribute declarations    | `<attribute name="…">default</attribute>`               | Same shape, lowered to the same AST. Default text or `@select="{$expr}"` attribute.                                             |
| Slices and slice events   | `slice="x"` + `slice-event="…"` + `slice-value="{ … }"` | Same surface, but `slice-value` carries a CEM-QL expression.                                                                    |
| Validation / open-content | Implicit per the POC engine                             | Schema-governed; the cem-element substrate participates in `cem_ml` scope policy and Tier A semantic-validation catalog.        |

## 4. Runtime model

1. **Declaration upgrade.** When the browser upgrades `<cem-element tag="X">`, the
   runtime:
   - looks up the single child declaration `<template>`;
   - hands the declaration template's `template.content` to `cem_ml` for tokenization,
     schema scoping, and AST construction;
   - extracts declared attributes (becomes the produced custom element class's
     `observedAttributes`);
   - extracts slice declarations and event bindings (becomes the instance data-island
     state contract);
   - extracts the render template (a CEM AST projected to WHATWG light DOM via
     `cem_ml`'s `OutputTarget::LightDomCustomElements`, AC-I-6);
   - registers `tag="X"` with `customElements.define` if not already defined.
2. **Instance initialization.** When an instance of `X` connects, the runtime:
   - captures host attributes, dataset, and author child payload into
     `<template data-cem-island="instance">`;
   - records slot names, default payload, slices, validation state, and event payloads
     under that instance data island;
   - removes the captured raw payload from the live render tree before first render.
3. **Render.** On connect and on every data-island change, the runtime re-renders the
   instance's visible light-DOM output from the cached AST + the current instance
   data-island state. The data-island template itself is excluded from the diff. The
   render path goes through the same `cem_ml::interpreter::light_dom` pipeline as the
   build-time transform, so dev/runtime output is byte-identical.
4. **Events.** Declarative `slice-event="…"` bindings install DOM listeners on the
   rendered children. Listener payloads write back to the data island, which
   triggers the next render. There are no JS event handlers in the authoring
   surface.
5. **Source maps.** Every rendered node carries the AC-P-7 source-map stack back to
   its position inside the declaration template, so dev tools can trace any node in
   the live DOM to its author byte offset.

### 4.1 UI and processing layer split

The runtime MUST keep browser UI responsibilities separate from template processing.
That split is not just an implementation detail; it is the boundary that lets the same
CEM template/data engine run in different hosts.

- **UI adapter layer (`cem-element`).** Owns custom-element declaration discovery,
  produced element lifecycle, data-island capture, browser event listeners, form/focus
  behavior, target DOM roots, and final light-DOM patch application.
- **Processing layer.** Owns CEM-ML/CEM-QL parsing, template artifacts, data snapshots,
  render-plan generation, render-plan diffing, diagnostics, source maps, and patch
  frames. Its inputs and outputs are serializable. It MUST NOT depend on live browser
  DOM nodes, `customElements`, browser event dispatch, focus state, or form control
  internals.

The processing layer may run in-process, in a browser WASM worker, in a pool of workers,
on an edge/compute worker, or in a server-side rendering host. The UI adapter still owns
the browser integration in every client-side mode. Remote or server processing may
produce rendered HTML, render plans, or patch frames, but it cannot directly mutate
browser DOM or observe browser-only state. Focus, selection, transient input state,
MutationObserver timing, and event-to-data writes remain client UI-adapter concerns.

This makes these deployment modes valid without changing the declaration model:

- **Browser worker mode.** The processing layer runs in WASM workers for parallel
  compile/render/diff work; the main thread applies committed patch transactions.
- **Edge processing mode.** A compute-CDN/edge-worker host can render from a
  serialized data-island snapshot and a stored template/render-plan artifact. A nearby
  KV/document store may hold data snapshots and virtual/render-plan state by version,
  but not live DOM. This mode is useful for first render, precomputation, or
  server-assisted updates; it is not the default for high-frequency local interactions
  because network latency, consistency, privacy, and conflict handling become part of
  the contract.
- **Server-side rendering mode.** The processing layer can emit HTML plus hydration
  metadata and source-map markers. On hydration, the browser UI adapter reconstructs or
  validates the instance data island and retained render-plan identity before taking
  over local event-to-data updates.

### 4.2 Serializable processing boundary

The UI adapter and processing layer communicate through serializable records. These
records are the semantic contract; the concrete wire encoding (structured clone, JSON,
binary AST buffers, or a hybrid) is a Phase 3 implementation decision.

`DataIslandSnapshot` is the complete processing input for one produced custom element
instance at one render sequence:

```ts
interface DataIslandSnapshot {
  instanceId: string;
  producedTag: string;
  declarationTag: string;
  templateArtifactId: string;
  renderSeq: number;
  dataRevision: string;
  scopePolicyStamp: string;
  privacyPolicyStamp: string;
  hostAttributes: Record<string, string | boolean | null>;
  dataset: Record<string, string>;
  payload: SerializedPayload;
  slices: Record<string, unknown>;
  validationState: Record<string, unknown>;
  eventPayloads: Record<string, unknown>;
}
```

The snapshot MUST NOT contain live `Node`, `Event`, `Element`, `DocumentFragment`,
function, class instance, or browser handle references. Payload content is serialized
from the inert instance data-island `<template>` and normalized before it crosses the
processing boundary. The UI adapter owns the conversion between live browser state and
this snapshot.

`RenderPlanIdentity` names a retained previous output that a worker, server, or edge
host can diff against without receiving the live browser DOM:

```ts
interface RenderPlanIdentity {
  renderPlanId: string;
  instanceId: string;
  templateArtifactId: string;
  dataRevision: string;
  renderSeq: number;
  outputTarget: "light-dom";
  scopePolicyStamp: string;
  sourceMapMode: "dev" | "prod";
}
```

The processing layer may retain the full render plan in WASM memory, worker memory,
server memory, or a content-addressed cache. Hosts exchange the identity and only send
the full plan when the cache/retained state is missing or policy-invalid.

`scopePolicyStamp` is an opaque, deterministic identity for the effective scope policy
that governed parsing, resource loading, render planning, privacy export, and patch
generation. It MUST change when any of those effective rules change. Cache keys and
render-plan identities MUST include it so artifacts created under one policy are not
reused under another.

Resolver and cache identity are part of the same boundary:

- template artifacts are keyed by source/content hash, URL or specifier, resolver
  identity, `cem_ml` version, `cem_ql` version, source-map mode, and
  `scopePolicyStamp`;
- render plans are keyed by template artifact identity, `dataRevision`,
  `scopePolicyStamp`, output target, and render engine version;
- URI resolution and module-map state are represented by identity stamps, not by live
  resolver functions crossing the boundary.

Data privacy is fail-closed. A `DataIslandSnapshot` MAY leave the browser only when the
effective scope policy allows the relevant fields to be exported to the selected host.
By default, snapshots are local-only. Sensitive fields, transient input composition,
focus/selection state, raw browser events, credentials, and policy-denied payloads MUST
remain in the UI adapter. Edge/server hosts receive redacted or omitted fields rather
than implicit access.

Patch transport uses internal frames, never browser DOM events:

```ts
type PatchFrame =
  | { type: "begin"; instanceId: string; renderSeq: number }
  | { type: "ops"; instanceId: string; renderSeq: number; ops: DomPatchOp[] }
  | { type: "commit"; instanceId: string; renderSeq: number }
  | { type: "abort"; instanceId: string; renderSeq: number; diagnostic: Diagnostic };
```

Frames are ordered per `{ instanceId, renderSeq }`. The UI adapter buffers frames until
`commit`, drops stale frames whose `renderSeq` is older than the instance's current
sequence, and applies committed patches synchronously during the next batched
main-thread flush. Browser `EventTarget` / DOM `Event` dispatch is reserved for real
user and browser events, not patch transport.

### 4.3 Phase 3 MVP topology

The Phase 3 MVP topology is browser-local processing with a worker-backed primary path
and a main-thread fallback:

- **Primary path:** the host runtime support layer runs `cem_ml` WASM in a browser
  worker or worker pool. Declaration sources and `DataIslandSnapshot` records cross the
  serializable boundary; template artifacts and retained render plans stay in
  worker/WASM memory when possible. The worker returns diagnostics, source maps,
  `DomPatchPlan` objects, or `PatchFrame` streams.
- **Fallback path:** the same host runtime API can run `cem_ml` WASM on the main thread
  when workers are unavailable, disabled by policy, or not useful in a test host. This
  fallback is a compatibility path, not the performance target, and MUST preserve the
  same template, data, render, diff, and patch semantics as the worker-backed path.
- **UI ownership:** the main-thread `cem-element` adapter always owns custom-element
  lifecycle, browser events, instance data-island capture, focus/form behavior, and
  final browser DOM patch application.

The MVP includes the serializable processing boundary, local parser streaming, remote
source streaming where the platform provides stream bodies, retained render-plan
identity, patch-frame transport, and per-instance patch transactions with batched
main-thread flush.

The MVP does not require edge/SSR execution, threaded WASM with `SharedArrayBuffer`,
precompiled template artifacts, service-worker artifact registries, or a production
multi-worker cache. Those paths remain valid deployment targets after the browser-worker
contract is stable. The initial worker-pool size is a separate Phase 3 decision; this
topology only requires that worker-backed and main-thread fallback modes share the same
observable behavior.

## 5. Data-island isolation guarantees

The declaration `<template>` wrapper makes template source inert. The produced
custom element instance's data-island `<template>` wrapper makes mutable runtime data
inert. Together they make the following true without author effort:

- **Render isolation.** No child of the declaration template or instance data-island
  template participates in CSS selector matching, layout, painting, accessibility
  tree, or `getElementsByTagName` on the document.
- **Form isolation.** Form-associated descendants inside a data-island `<template>`
  are not part of the page's form data; only the rendered form controls submit.
- **Mutation isolation.** Author writes to the instance data island go through the
  runtime's scope-policy mutation API (AC-M-*); direct DOM mutations of the instance
  data-island `template.content` are allowed (it is a real `DocumentFragment`) and
  trigger a render diff.
- **Polyfill story.** When the browser does not upgrade `cem-element` (no JS, JS
  failed, lazy load pending), declaration template source remains inert. Produced
  custom element instances may show author fallback payload until upgrade; after
  upgrade that payload is captured into the instance data-island template and stops
  affecting the UI directly.

## 6. Compatibility & migration

### 6.1 `@epa-wg/custom-element` monorepo migration

- The package is migrated from its current home (`~/aWork/custom-element/`) into
  `packages/custom-element/` inside this monorepo. The migration preserves history
  and the published npm package identity.
- Until parity is reached (§7) the existing `<custom-element>` authoring tag remains
  the production surface. The package continues to publish from this monorepo, while
  `@epa-wg/cem-elements/cem-element` is the staging substrate entrypoint.
- The next major of `@epa-wg/custom-element` keeps `<custom-element>` as the
  package's public tag and rebuilds its implementation by inheriting the
  `cem-element` substrate. Existing consumers keep importing
  `@epa-wg/custom-element`; the implementation contract changes, not the package
  identity.

### 6.2 Co-existence window

During the bridge period (between this design landing and the
`@epa-wg/custom-element` implementation adoption):

- Both tags MAY appear in the same document. They share `customElements` registry
  state; tag names MUST NOT collide.
- The `cem-element` runtime understands the legacy XSLT-shaped template body as a
  compat surface only when the body is annotated `lang="custom-element-v0"` on the
  `<template>` element. New code MUST use the CEM-ML surface.

### 6.3 Cem-components contract

`@epa-wg/cem-components` authors every primitive with `<cem-element>`. The contract
docs in [`packages/cem-components/docs/`](../packages/cem-components/docs/) name
`<cem-element>` as the authoring tag and `cem-ql` as the expression language. The
host-API, attribute, event, validation, focus, and a11y rules are independent of
which substrate hosts them and remain authoritative.

## 7. Production-ready criteria

`@epa-wg/cem-elements` is **production-ready** (and the bridge window closes) only
when **all** of the following hold:

1. **Functional parity with `<custom-element>`.** Every public behavior the POC
   documents (`~/aWork/custom-element/docs/attributes.md`,
   `~/aWork/custom-element/docs/rendering.md`) reproduces under `<cem-element>` with
   a one-to-one fixture in `packages/cem-elements/tests/parity/legacy/`.
2. **Template and data-island isolation.** Fixtures assert that declaration template
   source and instance data-island contents are backed by `<template>` content. Raw
   declaration or data-island descendants do not render, match document selectors,
   submit form data, or enter the accessibility tree, and only the rendered
   projection affects the UI.
3. **Material parity.** Every component in
   `~/aWork/custom-element-dist/src/material/` — `action.html`, `autocomplete.html`,
   `badge.html`, `dropdown.html`, `icon.html`, `icon-link.html`, `input.html`,
   `menu.html` — is rebuilt under `<cem-element>` with a paired fixture in
   `packages/cem-elements/tests/parity/material/`. The rendered DOM, accessibility
   tree, and keyboard behavior match the legacy versions on a documented browser
   matrix. The fixture set MUST cover local/external `src`, hidden declarations,
   nested components, declarative slot projection, inline styles scoped to the host,
   `attribute select`, `if`/`choose` bridge constructs, namespaced `xhtml:*`
   elements, boolean attribute helper semantics, `module-url` resource slices,
   `data`/`option` payloads, slice events, and `slice-value`.
4. **Cem-ml integration.** All `<cem-element>` templates parse cleanly through
   `nx run cem_ml_cli:validate-fixtures` and round-trip through
   `nx run cem_ml_cli:e2e` cross-surface conversion. The Phase 2 semantic-validation
   catalog applies without exceptions.
5. **Performance.** AC-N-1 first-paint budgets hold on the material parity fixtures
   under the same `nx run cem_ml:bench` discipline.
6. **A11y.** The accessibility contract from
   [`packages/cem-components/docs/accessibility.md`](../packages/cem-components/docs/accessibility.md)
   is verified end-to-end on the material parity fixtures.

When (1)–(6) are green, the `cem-element` substrate becomes the implementation base
for the next major of `@epa-wg/custom-element`. The `<custom-element>` tag remains
published by that package; `@epa-wg/cem-elements` stops being the staging migration
target once the package adopts the substrate.

## 8. References

- [`docs/cem-element-wasm-proposal.md`](./cem-element-wasm-proposal.md) — host
  runtime support layer, WASM worker processing, patch-frame transport, edge
  processing, and SSR options.
- [`docs/cem-ml-syntax.md`](./cem-ml-syntax.md) — CEM-ML canonical curly surface.
- [`docs/cem-ml-ac.md`](./cem-ml-ac.md) — AC-F-2 (schema scoping), AC-F-5
  (reference slots), AC-I-6 (WHATWG DOM compliance), AC-M-* (mutation), AC-P-7
  (source-map stack), AC-T-1 / AC-T-7 (transform + template embedding).
- [`docs/cem-ql-ac.md`](./cem-ql-ac.md) — CEM-QL surface that backs template
  expressions and AVT spans.
- [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md),
  [`light-dom-rendering.md`](../packages/cem-components/docs/light-dom-rendering.md),
  [`accessibility.md`](../packages/cem-components/docs/accessibility.md) — the
  contract the substrate exists to enable.
- `~/aWork/custom-element/` — legacy POC, functional reference per
  [`CLAUDE.md`](../CLAUDE.md) §custom-element legacy info.
- `~/aWork/custom-element-dist/src/material/` — material parity benchmark.
