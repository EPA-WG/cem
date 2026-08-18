# `cem-element` Design

**Status:** Design doc for the `<cem-element>` declarative custom-element substrate.
Pairs with the parser/runtime work in [`cem-ml-stack-design.md`](./cem-ml-stack-design.md),
the query/template surface in [`cem-ql-stack-design.md`](./cem-ql-stack-design.md), and the
component contracts in [`packages/cem-components/docs/`](../packages/cem-components/docs/).

This document is the source of truth for the `cem-element` substrate that
`@epa-wg/cem-components` builds on. It defines the successor substrate for the
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

`cem-element` is **not** a fork of `<custom-element>`. It is the new substrate that a
later `@epa-wg/custom-element` adoption phase can inherit from after the browser
substrate and Edge/SSR follow-up phase are stable. In that later phase,
`@epa-wg/custom-element` continues publishing the `<custom-element>` tag, but its
implementation is rebuilt on the `cem-element` substrate and published from this
monorepo.

## 2. Packages

| Package                           | Status                                           | Role                                                                                                                                                                                                                            |
|-----------------------------------|--------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `@epa-wg/cem-elements`            | Planned, this design                             | Houses the `<cem-element>` runtime and its declarative authoring surface. Plural ("elements") refers the functional components as opposite to `@epa-wg/cem-components` UI library that consumes it.                             |
| `@epa-wg/cem-components`          | Phase 3, contract docs landed                    | Declarative component primitives (`cem-button`, `cem-input`, …) authored with `<cem-element>` and the conventions in [`packages/cem-components/docs/conventions.md`](../packages/cem-components/docs/conventions.md).           |
| `@epa-wg/custom-element`          | External today, post-Edge/SSR adoption phase     | Existing POC at `~/aWork/custom-element/`. Source moves into `packages/custom-element/` only after the Edge/SSR follow-up phase; future major keeps publishing `<custom-element>` and implements it by inheriting the `cem-element` substrate. XSLT syntax preservation TBD. |
| `custom-element-dist` (reference) | External                                         | Material-style sample components at `~/aWork/custom-element-dist/src/material/` (`action`, `autocomplete`, `badge`, `dropdown`, `icon`, `icon-link`, `input`, `menu`). Used as the parity benchmark for `cem-element` (see §7). |

## 3. Authoring surface

Terminology used below:

- **Declaration element** means `<cem-element>`. It declares/registers a custom
  element tag and owns the CEM-ML template source. CEM declaration lookup is scoped
  and inherited, while the produced tag is registered once in the declaration
  document's global browser `customElements` registry under the contract below.
- **Declaration template** means the single direct-child WHATWG `<template>` inside
  `<cem-element>`. It is inert browser content, but it is not the mutable runtime
  data island.
- **Produced custom element instance** means an instance of the declared tag, such as
  `<cem-button>` or `<cem-menu>`. This is not the legacy `<custom-element>` tag.
- **Instance data island** means the produced custom element instance's inert
  `<template data-cem-island="instance">`, which stores mutable attributes, payload,
  slices, validation state, and event payloads.

### Declaration registry and name contract

Phase 3 separates two registries that have different scopes:

- The **logical CEM declaration registry** belongs to a CEM parser/runtime scope.
  Lookup checks the current scope first and then walks parent scopes. This is the
  scoped, inherited template/registry behavior required by AC-R-1 and AC-R-2.
- The **browser custom-elements registry** is
  `declarationElement.ownerDocument.defaultView.customElements`. Phase 3A treats it
  as document-global and does not require browser scoped-custom-element-registry
  support. A later browser optimization may use scoped registries only behind the
  same logical lookup and collision contract.

Every resolved declaration has a stable **registration identity** that binds the
produced tag, resolved template source identity, template language, and browser
behavior contract. Registration is decided before calling
`CustomElementRegistry#define`:

1. A second declaration for the same tag in the same logical scope is an error,
   even when both registration identities match
   (`cem-element.registry_same_scope_duplicate`).
2. A child scope may repeat an inherited tag only when its registration identity
   is identical. The child aliases the inherited declaration and does not define
   the browser tag again.
3. A different registration identity for an inherited tag is an incompatible
   shadow and fails before browser mutation
   (`cem-element.registry_inherited_collision`). Discovery-only tooling may expose
   the collision as the policy-controlled diagnostic required by AC-R-3, but the
   runtime registration gate is fail-closed.
4. A document-global browser definition may be reused only when it is owned by the
   CEM runtime and carries the identical registration identity. A different CEM
   identity, a legacy `@epa-wg/custom-element` definition, or any foreign
   constructor is a hard collision (`cem-element.browser_tag_collision`).
5. Therefore every public produced tag has one compatible definition per browser
   document. During the coexistence window, CEM and legacy declarations may share
   a document but may not claim the same produced tag.

Produced tags MUST satisfy the WHATWG custom-element name syntax. The `cem-`
prefix remains reserved for primitives published by `@epa-wg/cem-components`, as
defined by that package's conventions; the generic `cem-element` runtime does not
require third-party declarations to use that prefix and cannot infer package
ownership from a DOM declaration. Package authoring and verification gates enforce
the reserved namespace.

The executable decision core is
`CEM_DECLARATION_REGISTRATION_CONTRACT` plus
`analyzeDeclarationRegistration()` in `@epa-wg/cem-elements`. It is deliberately
pure so duplicate and collision decisions can be verified before the browser
registry is mutated. The Phase 3 substrate audit must wire runtime registration to
that accepted decision core rather than maintaining a second policy.

The logical-scope host API is `CemDeclarationScope` plus
`createCemDeclarationScope()` and `getDefaultCemDeclarationScope()`:

- A scope is an opaque object. Its object identity is its scope identity; callers do
  not provide or serialize an ID.
- `getDefaultCemDeclarationScope(document)` returns one weakly held root per
  `Document`. A disposed default root is replaced on the next request.
- `createCemDeclarationScope({ document, parent })` creates an explicit root or
  child. Parentage is immutable, the optional parent must own the same `Document`,
  and no scope relationship is inferred from declaration-element or arbitrary DOM
  ancestry.
- Inline and external declarations use the same selected runtime scope. Logical
  lookup reports a same-scope binding separately from the nearest inherited binding
  so `analyzeDeclarationRegistration()` remains the only collision-policy decision
  core. A compatible inherited declaration is committed to the child as an alias of
  the inherited declaration and constructor, not as a second compiled definition.
- `dispose()` is idempotent. It clears that scope's logical declaration ownership
  and makes the scope, plus descendants that still name it as an ancestor, invalid
  for future lookup or registration. It does not and cannot remove a constructor
  from the document-global `customElements` registry; already-defined constructors
  and upgraded instances retain normal browser lifetime.
- `scopePolicyStamp` is not a scope identifier. It remains independently versioned
  processing, resolver, privacy, and cache-policy metadata.

The construction, ancestry, lookup, and disposal contract is executable in the
pure declaration-scope tests. `CemElementRuntimeOptions.declarationScope` selects an
explicit scope; otherwise inline and external declarations select their owning
document's default root. The runtime derives a `cem-registration-v1` content address
from the produced tag, resolved template source, template language, and browser
behavior version. `CemDeclarationRegistrationOptions.behaviorIdentity` is required
and non-empty whenever `behavior` is present because function source text and object
identity are not stable across builds. Behavior-less declarations use a fixed null
behavior component and need no extra option.

After `analyzeDeclarationRegistration()` accepts the combined logical/browser state,
the runtime commits the logical binding and marks the produced constructor with the
CEM registration identity before `CustomElementRegistry#define`. Identical inherited
or independent-scope declarations alias the retained compiled declaration and the one
document-global constructor. Rejections and failed browser definitions leave no new
logical binding.

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

- Every `<cem-element>` declaration with **inline** template source MUST contain
  exactly one direct-child WHATWG `<template>` element. Declaration content outside
  that wrapper is invalid, because it would be live page content instead of
  declaration template source.
- A `<cem-element>` declaration MAY instead carry a `src="…"` attribute pointing at
  an external or in-document template (see §3.2). When `src` is set, the declaration
  MUST NOT also contain an inline `<template>` child; the URI form supplies the
  template source.
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

### 3.2 URI declaration syntax

URI-backed declarations use the `src` attribute on `<cem-element>` itself, matching
the legacy `<custom-element src="…">` shape. This keeps authoring parity with the
existing POC and with the material parity benchmark (which uses
`<custom-element src="./icon-link.html#cem-icon-link" tag="cem-icon-link">` and
`<custom-element hidden src="#cem-icon" tag="cem-icon">` patterns).

```html
<!-- External resource with fragment identifier -->
<cem-element tag="cem-icon" src="./icon-link.html#cem-icon-link"></cem-element>

<!-- Same-document fragment -->
<cem-element tag="cem-icon" hidden src="#cem-icon-template"></cem-element>

<!-- Module-map specifier resolved by the cem-element resolver (§3 of the WASM proposal) -->
<cem-element tag="cem-button" src="@epa-wg/cem-components/button.cem#button"></cem-element>
```

Rules:

- `src` on `<cem-element>` is the **only** URI declaration form. The previously
  considered alternates — `<template src="…">` on the inner template, and
  `<cem-element template-src="…">` — are **rejected**. Keeping URI on the
  declaration element preserves one-to-one parity with `<custom-element>` and avoids
  splitting source identity across two elements.
- When `src` is present, the declaration MUST NOT carry an inline `<template>`
  child. The runtime fetches and parses the resource, then treats the resolved
  fragment (or whole resource, when no fragment is given) as the declaration
  template body.
- `src` resolves through the `cem-element` module-map resolver and scope-URL policy
  documented in [`cem-element-wasm-proposal.md` §3](./cem-element-wasm-proposal.md).
  Supported forms include absolute URLs, document-relative URLs, fragment-only
  references (`#name`), and module-map specifiers (`@scope/pkg/path#fragment`).
- A `src` without a fragment loads the whole resource as the declaration template.
  A `src` with a fragment selects the named template/region inside the resolved
  resource after parse.
- `src` MAY appear on both declaration and instance usages, mirroring the legacy
  POC (`<custom-element src="../index.html#nav-head">`). On a declaration, `src`
  supplies the template body. On an instance with no matching `tag` registration
  yet, `src` is treated as an inline declaration of an anonymous tag (legacy
  behavior); the formal rules for that case land with the migration work in §6.1.
- All other declaration semantics (data-island isolation, scope policy, source
  maps, render pipeline, patch transport) are identical to the inline form. `src`
  is purely a source-acquisition shape.

### 3.3 Template engine

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
  frames. Its inputs and outputs are serializable, with CEM binary/chunk payloads
  preferred over JSON when both sides can consume them. It MUST NOT depend on live
  browser DOM nodes, `customElements`, browser event dispatch, focus state, or form
  control internals.

The processing layer may run in-process, in a browser WASM worker, in a pool of workers,
on an edge/compute worker, or in a server-side rendering host. Phase 3 implements the
browser worker substrate first; edge and SSR execution are follow-up hosts that reuse
the same serializable boundary. The UI adapter still owns the browser integration in
every client-side mode. Remote or server processing may produce rendered HTML, render
plans, or patch frames, but it cannot directly mutate browser DOM or observe
browser-only state. Focus, selection, transient input state, MutationObserver timing,
and event-to-data writes remain client UI-adapter concerns.

This makes these deployment modes valid without changing the declaration model:

- **Browser worker mode.** The processing layer runs in WASM workers for parallel
  compile/render/diff work; the main thread applies committed patch transactions.
- **Edge processing mode.** A compute-CDN/edge-worker host can render from a
  serialized data-island snapshot and a stored template/render-plan artifact. A nearby
  KV/document store may hold data snapshots and virtual/render-plan state by version,
  but not live DOM. This mode is useful for first render, precomputation, or
  server-assisted updates; it is not the default for high-frequency local interactions
  because network latency, consistency, privacy, and conflict handling become part of
  the contract. The first storage model is a hybrid: immutable, content-addressed
  blobs hold template artifacts, render plans, rendered HTML fragments, and only
  policy-sanitized snapshot exports; a small revisioned pointer record holds the
  current `RenderRevision`, content addresses, scope/privacy policy stamps, and an
  ETag-like compare value for stale-write rejection. Persistent full snapshot storage
  is opt-in by export policy and browser-only state is never stored at the edge.
- **Server-side rendering mode.** The processing layer can emit HTML plus hydration
  metadata and source-map markers. On hydration, the browser UI adapter reconstructs or
  validates the instance data island and retained render-plan identity before taking
  over local event-to-data updates. Phase 3.5 fixtures use a direct child
  `<script type="application/json" data-cem-hydration="snapshot">` containing the
  serialized `DataIslandSnapshot`, a direct instance data-island `<template>`, and the
  normal `<!--cem-render-start-->` / `<!--cem-render-end-->` render boundary comments.
  When those three pieces match the produced element, the browser runtime preserves the
  server-rendered light DOM on first connect, restores the instance/data revision state,
  and lets normal client invalidation handle later mutations.

### 4.2 Serializable processing boundary

The UI adapter and processing layer communicate through serializable records. These
records are the semantic contract. The concrete Phase 3 wire encoding is the hybrid
format selected below.

`DataIslandSnapshot` is the complete processing input for one produced custom element
instance at one render revision:

```ts
interface RenderRevision {
  instanceId: string;
  dataRevision: string;
  templateArtifactId: string;
  scopePolicyStamp: string;
  outputTarget: "light-dom";
  renderAttempt?: number;
}

interface DataIslandSnapshot {
  instanceId: string;
  producedTag: string;
  declarationTag: string;
  templateArtifactId: string;
  dataRevision: string;
  outputTarget: "light-dom";
  sourceMapMode?: SourceMapMode;
  renderAttempt?: number;
  scopePolicyStamp: string;
  privacyPolicyStamp: string;
  hostAttributes: Record<string, string | boolean | null>;
  dataset: Record<string, string>;
  payload: SerializedPayload;
  slices: Record<string, unknown>;
  formData?: Record<string, unknown>;
  validationState: Record<string, unknown>;
  eventPayloads: Record<string, unknown>;
}
```

The snapshot MUST NOT contain live `Node`, `Event`, `Element`, `DocumentFragment`,
function, class instance, or browser handle references. Payload content is serialized
from the inert instance data-island `<template>` and normalized before it crosses the
processing boundary. The UI adapter owns the conversion between live browser state and
this snapshot.

The render revision for a snapshot is the tuple `{ instanceId, dataRevision,
templateArtifactId, scopePolicyStamp, outputTarget, renderAttempt? }`. The UI adapter
owns the latest requested revision for each instance and workers echo that revision in
render plans and patch frames. `renderAttempt` is used only for retries of the same
data/template/policy revision; it is not the primary ordering model.

#### Phase 3 wire encoding decision

Phase 3 uses a **hybrid wire format** designed so the heavy payloads can migrate to a
binary-first format later without changing the semantic worker API:

- Worker messages use structured-clone plain-record envelopes for control flow,
  request correlation, artifact handles, render-plan identities, diagnostics, and
  small patch frames.
- Structured-clone payloads are restricted to a JSON-compatible subset: plain objects,
  arrays, strings, numbers, booleans, `null`, and explicitly declared transferable
  `ArrayBuffer` fields. They do not contain DOM nodes, functions, class instances,
  `Map`, `Set`, `Date`, `RegExp`, or browser handles.
- Template artifacts and full render plans stay retained in worker/WASM memory by
  default. Hosts exchange stable identities and handles rather than deep JS object
  graphs.
- Cacheable or large payloads cross as versioned transferable `ArrayBuffer` blobs:
  compiled template/cache artifacts, source-map sidecars, optional render-plan
  snapshots, and future large patch-op batches.
- Diagnostics remain structured and JSON-compatible permanently. They may carry the
  relevant `SourceMapStack` inline for author reporting, but bulk source-map tables
  cross as references or sidecars. Source-map refs and sidecars carry `fidelity` so
  devtools and parity fixtures can distinguish exact author bytes from canonicalized
  DOM or declaration-only mapping.

This is Option D from the Phase 3 wire-format options. It is intentionally Option
C-compatible: the envelope shape is the stable API, while each heavy payload can be
replaced section-by-section with the eventual binary AST/render-plan/patch-op payload
format.

The worker-crossing shapes are:

```ts
interface TemplateArtifactRef {
  artifactId: string;
  cacheKey: CacheKey;
  sourceMapMode: "dev" | "prod";
  policyStamp: string;
  declaredAttributes: string[];
  observedAttributes: string[];
  invalidationScopes: string[];
  sourceMapRef?: SourceMapRef;
  diagnostics: Diagnostic[];
}

interface ArtifactBinaryTransfer {
  kind: "template-artifact" | "query-artifact" | "cache-artifact";
  cacheKey: CacheKey;
  formatVersion: string;
  policyStamp: string;
  bytes: ArrayBuffer;
  sourceMapSidecarHash?: ContentHash;
}

interface SourceMapRef {
  hash: ContentHash;
  sourceMapMode: "dev" | "prod";
  fidelity: SourceMapFidelity;
  frameCount?: number;
}

type SourceMapFidelity =
  | "author-byte-exact"
  | "dom-canonical"
  | "declaration-only";

interface SourceMapSidecarTransfer {
  kind: "source-map-sidecar";
  hash: ContentHash;
  formatVersion: string;
  fidelity: SourceMapFidelity;
  bytes: ArrayBuffer;
}

interface RenderPlanBinaryTransfer {
  kind: "render-plan";
  identity: RenderPlanIdentity;
  formatVersion: string;
  bytes: ArrayBuffer;
}
```

`TemplateArtifactRef` is the normal compile result across the browser worker boundary.
`ArtifactBinaryTransfer` is used only when an artifact must leave retained
worker/WASM memory: cache import/export, worker migration, build-pipeline prewarm, or a
future service-worker/package artifact registry. The full template artifact MUST NOT be
exposed as a deep structured-clone object.

`RenderPlanIdentity`, defined below with the cache identity fields, names a retained
previous output that a worker, server, or edge host can diff against without receiving
the live browser DOM.

The processing layer may retain the full render plan in WASM memory, worker memory,
server memory, or a content-addressed cache. Hosts exchange the identity and only send
the full plan when the cache/retained state is missing or policy-invalid.
When the full render plan must cross the boundary, it crosses as
`RenderPlanBinaryTransfer`, not as a deep JS object graph. Small diagnostic summaries or
debug metadata may remain structured-clone records, but the runtime plan payload is
binary-versioned from the first Phase 3 implementation.

`scopePolicyStamp` is an opaque, deterministic identity for the effective scope policy
that governed parsing, resource loading, render planning, privacy export, and patch
generation. It MUST change when any of those effective rules change. Cache keys and
render-plan identities MUST include it so artifacts created under one policy are not
reused under another.

Resolver and cache identity are part of the same boundary. URI resolution and
module-map state are represented by identity stamps, not by live resolver functions
crossing the boundary.

Phase 3 uses a two-level cache identity. The portable payload key identifies reusable
artifact bytes. The load identity records how this host resolved and is allowed to use
those bytes.

```ts
type SourceMapMode = "dev" | "prod";

interface SourceRef {
  kind: "inline" | "url" | "specifier" | "fragment";
  value: string;
}

interface TemplateArtifactPayloadKey {
  contentType: "cem-template-artifact";
  sourceHash: ContentHash;
  cemMlVersion: string;
  cemQlVersion: string;
  sourceMapMode: SourceMapMode;
}

interface TemplateArtifactIdentity {
  artifactId: string;
  payloadKey: TemplateArtifactPayloadKey;
  sourceRef: SourceRef;
  resolverIdentity: string;
  scopePolicyStamp: string;
}

interface RenderPlanIdentity {
  renderPlanId: string;
  templateArtifactId: string;
  revision: RenderRevision;
  renderEngineVersion: string;
  sourceMapMode: SourceMapMode;
}

interface ArtifactRegistryNamespace {
  namespace: "cem-template-artifacts";
  registryContractVersion: "cem-artifact-registry-v1";
  artifactFormatVersion: string;
}

interface SourceMapRegistryKey {
  payloadKey: TemplateArtifactPayloadKey;
  fidelity: SourceMapFidelity;
  sourceMapHash: ContentHash;
}
```

`sourceHash` is the CEM content hash for the canonical template source or compiled
template payload, following the shared `CEM-Hash`/`cem-bin/1+blake3` transport model.
`sourceRef` is provenance and invalidation context: it records the inline slot, URL,
module-map specifier, or fragment that led to the source, but it is not the portable
payload hash. Two source refs that resolve to identical canonical bytes may share the
same `TemplateArtifactPayloadKey`, but they produce distinct `TemplateArtifactIdentity`
values when resolver identity or scope policy differs.

`resolverIdentity` is an opaque deterministic stamp for the effective module map,
base-URL rules, URL policy, and fragment selection behavior. It MUST change when a
specifier, URL, or fragment could resolve to different canonical source bytes.
`scopePolicyStamp` MUST change when parsing, resource loading, query evaluation,
privacy export, or patch-generation policy changes. A payload whose load identity does
not match the active resolver and scope policy MUST NOT be reused for rendering, even
when the payload hash matches.

Phase 3 defines a service-worker-compatible registry contract, but it does not implement
a service-worker template/artifact registry. The actual registry is deferred until after
component parity. Any later registry, whether owned by the CEM site, docs, playgrounds,
or another host, must treat artifact bytes as immutable records addressed by
`TemplateArtifactPayloadKey` plus `ArtifactRegistryNamespace`. Host-specific permission
to use those bytes is still determined by `TemplateArtifactIdentity`. Source-map
sidecars are keyed by the same payload key plus source-map mode, fidelity, and
source-map hash. Registry namespace/version fields are eviction and migration metadata;
they do not change the portable payload identity.

The Phase 3 runtime support layer may expose optional artifact-registry hooks for tests
and future hosts, but it MUST work when no hooks are supplied and MUST NOT depend on
service-worker install, activate, fetch interception, or Cache Storage lifecycle:

```ts
interface ArtifactRegistryHooks {
  getArtifact?(
    namespace: ArtifactRegistryNamespace,
    key: TemplateArtifactPayloadKey,
  ): Promise<ArtifactBinaryTransfer | undefined>;
  putArtifact?(
    namespace: ArtifactRegistryNamespace,
    artifact: ArtifactBinaryTransfer,
  ): Promise<void>;
  getSourceMap?(
    namespace: ArtifactRegistryNamespace,
    key: SourceMapRegistryKey,
  ): Promise<SourceMapBinaryTransfer | undefined>;
  putSourceMap?(
    namespace: ArtifactRegistryNamespace,
    key: SourceMapRegistryKey,
    sourceMap: SourceMapBinaryTransfer,
  ): Promise<void>;
  invalidateNamespace?(namespace: ArtifactRegistryNamespace): Promise<void>;
}
```

Render plans are keyed by template artifact identity plus `RenderRevision`, source-map
mode, and render engine version. A render plan compiled from a matching payload under a
different resolver identity or scope policy is not reusable unless a later migration
defines an explicit policy-equivalence check.

Data privacy is fail-closed. A `DataIslandSnapshot` MAY leave the browser only when the
effective scope policy allows the relevant fields to be exported to the selected host.
By default, snapshots are local-only. Sensitive fields, transient input composition,
focus/selection state, raw browser events, credentials, and policy-denied payloads MUST
remain in the UI adapter. Edge/server hosts receive redacted or omitted fields rather
than implicit access.

Patch transport uses internal frames, never browser DOM events. The normative Phase 3
contract is stable render-node-id patching with a constrained scope-replacement
fallback. Normal diffs target `renderNodeId` values from the retained render plan.
`replaceScope` is allowed only for first render, fallback mode, explicit policy
replacement, or recovery after a target mismatch.

The legacy `<custom-element>` implementation provides the useful precedent here. Its
XSLT output is not blindly assigned with `innerHTML` or wholesale fragment replacement.
Before commit, `assureUnique(fragment)` stamps generated nodes with `data-dce-id`
values that are unique inside the current parent scope; then `merge(parent,
fragment.childNodes)` maps existing children by that id, applies changed attributes,
recurses into matching children, inserts new children in id order, and removes stale
children. The browser DOM node is therefore the durable object; the rendered fragment is
the desired state used to synchronize it.

`cem-element` keeps the same principle, but moves desired-state calculation into the
serializable render-plan/WASM boundary:

- The processing layer emits a virtual render tree whose node identity is stable for
  a template/data shape. Identity is conceptually parent-scoped, as in legacy
  `data-dce-id`; an implementation may expose a globally unique `renderNodeId`, but it
  must be derivable from parent identity plus local occurrence/key information rather
  than from worker order, object identity, or browser DOM state.
- The UI adapter keeps a retained previous render plan. It does not need a permanent
  render-id lookup table for normal full-tree synchronization. The default patch walk
  compares the current browser DOM and next virtual tree sequentially, and reads the
  current browser node's render identity from `node.cemRenderNodeId`.
- SSR/debug markup cannot carry DOM properties. During hydration or first comparison,
  the UI adapter reads `node.cemRenderNodeId` first and falls back to serialized
  identity markers such as `data-cem-render-node-id` on elements or range-marker
  comments. When a fallback marker is found, the adapter mirrors it into
  `node.cemRenderNodeId` so future comparisons use the faster property path.
- A new render produces a next render plan. Diffing previous to next yields text,
  attribute, insert, move, remove, or replace operations addressed by render-node ids.
  The UI adapter applies those operations to existing browser nodes whenever the ids
  match. Temporary per-parent lookup maps are allowed for keyed sibling reorders, but
  the normal path is sequential comparison plus browser-node identity properties.
- The browser UI adapter applies rerender plans directly to the retained DOM range.
  Text and comment regions that need explicit identity use comment markers such as
  `<!--cem-start:r12-->` / `<!--cem-end:r12-->`; if retained top-level render identities
  do not match the next plan, the adapter replaces the render scope and emits a recovery
  diagnostic. After direct patching, the adapter runs live-range directive setup:
  `slice-event` bindings remove authoring metadata and install/dedupe listeners on
  retained elements, while `module-url` helpers are treated as transient nodes that are
  resolved, removed, and written back to slices without forcing visible-node
  replacement.
- If the previous and next plans are equivalent, the UI adapter performs no DOM
  mutation. Data revision advancement alone is not a reason to replace or touch the
  visible DOM tree.
- Desired render-plan attributes are authoritative by default. A browser-only
  `CemProducedElementBehavior.preserveRenderedAttribute` predicate may opt one exact
  current attribute into runtime/browser ownership when that attribute is absent from
  the desired element. The UI adapter forwards that predicate to the DOM merge without
  serializing it into a render plan; desired values still overwrite current values,
  unclaimed undeclared attributes are removed, and the predicate does not prevent owner
  replacement. Behaviors close or otherwise settle native state in `beforeRender`
  before a render that replaces its owner.
- When a parent render contains another initialized `cem-element` produced tag, the
  parent patcher owns the nested element shell and attributes, not the nested element's
  rendered body. After the nested element has a runtime-owned data island or render
  bounds, parent synchronization preserves those children and lets the nested element's
  own observer/runtime rerender from changed shell attributes. Updating parent-provided
  payload inside an already-initialized nested CEM instance requires a focused data
  island payload merge, not blind child replacement.
- `replaceScope` remains a recovery/fallback operation, not normal rerender behavior.
  It is valid on first render, template/policy incompatibility, missing retained plan,
  target mismatch, or other cases where the patcher cannot prove that existing browser
  nodes still correspond to the retained render plan.

Operational metadata must not defeat no-op rendering. Per-node debug attributes such as
`data-cem-render-node-id`, source-map markers, and creation-time render metadata may be
present in the light DOM, but the authoritative latest render revision lives in runtime
state, hydration metadata, or boundary-level metadata. A data-only rerender that produces
the same virtual tree must not rewrite every element merely to refresh debug attributes.

Dynamic text and structural regions need identities even when there is no owner element
to carry a property. Normal HTML output uses comment ranges, for example
`<!--cem-start:r12-->` and `<!--cem-end:r12-->`, around potentially changing content
created by expression insertion, conditional blocks, repeated blocks, or slot
projection. These comments have no layout or CSS box effect and can represent an empty
region.

Content-type switches change the marker syntax. Inside `<style>` and `<script>`, the
runtime must not inject HTML comment nodes into the raw text content. The equivalent
range markers use the content language's block-comment form, for example
`/*cem-start:r12*/` and `/*cem-end:r12*/`. A content-specific patcher interprets those
markers while preserving the element as a single browser node.

`<textarea>` is a special case. The visible/form value of a textarea is
`HTMLTextAreaElement.value`; child DOM appended at runtime can exist under the
textarea, but it is not rendered and does not automatically change the live value. The
browser patcher may use those hidden children as the mergeable dynamic model for
textarea internals, while still treating the textarea element itself as the durable
browser form-control node.

For dynamic textarea interiors, the UI adapter:

1. keeps dynamic text, expression, conditional, repeated, and slot-projection parts as
   hidden child nodes or range markers under the textarea;
2. reconciles those hidden children against the virtual render tree using the same
   property-first render identity rules as other DOM nodes;
3. after a successful hidden-model merge, derives the next textarea string from the
   ordered hidden model, ignoring marker comments and joining each non-marker node's
   `textContent`;
4. writes `textarea.value = nextValue` only when the value actually changed; and
5. preserves browser-owned state such as focus, selection range, selection direction,
   input composition, custom validity, and user edits according to the element's
   controlled/uncontrolled binding policy.

Using only `textarea.lastElementChild.textContent` is not sufficient because a
textarea may contain multiple static and dynamic parts. The hidden child model is patch
state; `textarea.value` remains the authoritative rendered value.

Raw SSR HTML cannot directly serialize mergeable child DOM inside a `<textarea>`,
because the HTML parser treats textarea contents as text. SSR and persisted output that
need dynamic textarea interiors must emit a loader-friendly representation, such as an
`<xsl:element name="textarea">`-style or equivalent CEM-ML construction, template
payload, or hydration marker structure. The browser loader converts that representation
into an actual textarea element, installs the hidden child model, projects the initial
`.value`, and mirrors render identities into properties before normal hydration or DOM
merge begins.

```ts
type DomPatchTarget = { kind: "render-node"; id: string };

type PatchNodePayload =
  | { encoding: "structured-node-v1"; node: SerializedNode }
  | { encoding: "binary-node-v1"; formatVersion: string; bytes: ArrayBuffer };

interface SerializedNode {
  renderNodeId: string;
  kind: "element" | "text" | "comment";
  tagName?: string;
  text?: string;
  attributes?: Record<string, string>;
  children?: SerializedNode[];
  sourceMapRef?: SourceMapRef;
}

type DomPatchOp =
  | {
      op: "insertBefore";
      parent: DomPatchTarget;
      before?: DomPatchTarget;
      node: PatchNodePayload;
    }
  | { op: "remove"; target: DomPatchTarget }
  | { op: "replace"; target: DomPatchTarget; node: PatchNodePayload }
  | {
      op: "moveBefore";
      target: DomPatchTarget;
      parent: DomPatchTarget;
      before?: DomPatchTarget;
    }
  | { op: "setText"; target: DomPatchTarget; value: string }
  | {
      op: "setAttribute";
      target: DomPatchTarget;
      name: string;
      value: string | null;
    }
  | {
      op: "replaceScope";
      scopeId: string;
      node: PatchNodePayload;
      reason: "first-render" | "fallback" | "policy" | "recovery";
    };

type PatchFrame =
  | { type: "begin"; transactionId: string; revision: RenderRevision }
  | { type: "ops"; transactionId: string; batchIndex: number; ops: DomPatchOp[] }
  | { type: "commit"; transactionId: string; nextRenderPlan: RenderPlanIdentity }
  | { type: "abort"; transactionId: string; diagnostic: Diagnostic };

interface DomPatchPlan {
  transactionId: string;
  revision: RenderRevision;
  ops: DomPatchOp[];
  nextRenderPlan: RenderPlanIdentity;
}

type PatchApplyResult =
  | { status: "applied"; transactionId: string; revision: RenderRevision }
  | { status: "stale"; transactionId: string; latestRevision: RenderRevision }
  | { status: "aborted"; transactionId: string; diagnostic: Diagnostic }
  | { status: "mismatch"; transactionId: string; diagnostic: Diagnostic };

interface PatchApplier<TTargetRoot> {
  begin(
    frame: Extract<PatchFrame, { type: "begin" }>,
    root: TTargetRoot
  ): PatchApplyResult;
  append(frame: Extract<PatchFrame, { type: "ops" }>): PatchApplyResult;
  commit(frame: Extract<PatchFrame, { type: "commit" }>): PatchApplyResult;
  abort(frame: Extract<PatchFrame, { type: "abort" }>): PatchApplyResult;
  applyPlan(plan: DomPatchPlan, root: TTargetRoot): PatchApplyResult;
}
```

`DomPatchPlan` is the one-shot equivalent of `begin + ops + commit`. Streamed `ops`
frames carry zero-based `batchIndex` values; duplicate, missing, or out-of-order
batches abort the transaction. The UI adapter buffers frames until `commit`, drops a
transaction as stale when its `revision` does not equal the latest requested revision
for that instance, and applies committed transactions synchronously and atomically
during the next host-scheduled main-thread flush.

`transactionId` is unique per render attempt. `insertBefore` and `moveBefore` append
when `before` is omitted. `setAttribute` with `value: null` removes the attribute.
`replace` preserves the target's parent position while replacing the target subtree.
`replaceScope` replaces the rendered subtree for `scopeId` and MUST NOT be emitted for
normal data-island mutation once fine-grained render-node-id diffing can represent the
change.

`PatchApplier` is host-neutral. A browser implementation owns the target root,
property-first render identity resolution, focus/selection preservation, and DOM
mutation. It MUST not mutate DOM before `commit`; for `begin` and `append`, an
`applied` result means accepted into the pending transaction buffer. If a target cannot
be found or validated through sequential comparison, mirrored SSR/debug markers, or a
temporary keyed-sibling map, it returns `mismatch`, emits a diagnostic, aborts that
transaction, and requests or permits a `replaceScope` recovery transaction. Failed ops
are not skipped.

Phase 3 sends small `DomPatchOp[]` batches and `structured-node-v1` payloads as
structured-clone records. Large batches MAY later replace node or op payloads with
transferable binary sections while preserving the same `PatchFrame`, `DomPatchPlan`,
and `PatchApplier` lifecycle.

The Phase 3.5 edge-processing fixture uses the pure `RenderPlan` diff path in
`projection.ts`: serialized template source plus `DataIslandSnapshot` projects to the
next plan, then `diffRenderPlansToPatchFrames(previous, next)` emits
`begin` / batched `ops` / `commit` frames without live DOM access. The first supported
fine-grained diff covers stable render-node-id text and attribute changes; first render,
template changes, root-count changes, or unsupported structural deltas intentionally
fall back to `replaceScope` until a fuller move/insert/remove planner lands.

### 4.3 Phase 3 MVP topology

The Phase 3 MVP topology is browser-local processing with a worker-backed primary path
and a main-thread fallback:

- **Primary path:** the host runtime support layer runs `cem_ml` WASM in one dedicated
  browser worker by default. Declaration sources and `DataIslandSnapshot` records cross
  the serializable boundary; template artifacts and retained render plans stay in
  worker/WASM memory when possible. The worker returns diagnostics, source maps,
  `DomPatchPlan` objects, or `PatchFrame` streams.
- **Fallback path:** the same host runtime API can run `cem_ml` WASM on the main thread
  when workers are unavailable, disabled by policy, or not useful in a test host. This
  fallback is a compatibility path, not the performance target, and MUST preserve the
  same template, data, render, diff, and patch semantics as the worker-backed path.
- **Pool promotion path:** a scope-policy worker pool is deferred until Phase 3B. The
  pool MUST be an optimization behind the same host runtime API, not a separate
  template/render contract.
- **UI ownership:** the main-thread `cem-element` adapter always owns custom-element
  lifecycle, browser events, instance data-island capture, focus/form behavior, and
  final browser DOM patch application.

The MVP includes the serializable processing boundary, local parser streaming, remote
source streaming where the platform provides stream bodies, retained render-plan
identity, patch-frame transport, and per-instance patch transactions with batched
main-thread flush.

The MVP does not require edge/SSR execution, threaded WASM with `SharedArrayBuffer`,
precompiled template artifacts, service-worker artifact registries, or a production
multi-worker cache. It does require cache identities and optional registry hooks to stay
compatible with a later service-worker registry. Those paths remain valid deployment
targets after the browser-worker contract is stable. `SharedArrayBuffer` availability
MUST NOT affect Phase 3A behavior: when it is unavailable, the runtime uses the same
non-threaded dedicated worker path; when workers are unavailable or fail startup, the
runtime falls back to main-thread WASM. Worker-backed and main-thread fallback modes
MUST share the same observable behavior.

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

### 6.1 `@epa-wg/custom-element` adoption sequencing

- Phase 3 does not migrate `@epa-wg/custom-element` into this monorepo and does not
  make `<custom-element>` inherit the `cem-element` substrate.
- The existing external `<custom-element>` authoring tag remains the production surface
  until the browser substrate is parity-proven, the Edge/SSR follow-up phase is green,
  and the explicit adoption phase starts.
- In the later adoption phase, `@epa-wg/custom-element` moves from
  `~/aWork/custom-element/` into `packages/custom-element/`, preserving history and
  published npm identity. Its next major keeps `<custom-element>` as the public tag and
  rebuilds the implementation on the `cem-element` substrate.

### 6.2 Co-existence window

During the bridge period (between this design landing and the post-Edge/SSR
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

Storybook is the primary browser/runtime verification surface for Phase 3. Runtime
stories under `packages/cem-elements/` are executable fixtures: each story presents a
declaration, produced instances, data-island state, rendered light DOM, and focused
interaction or mutation scenario. CI runs those stories through Storybook Test
(`@storybook/addon-vitest`) in browser mode, with assertions for DOM output,
accessibility, events, focus/form behavior, and data-island isolation. Vitest remains
available for pure helper tests, but browser behavior is accepted or rejected through
Storybook.

The old `@epa-wg/custom-element` suite is a parity source, not the primary runner. Its
tests and docs (`~/aWork/custom-element/docs/attributes.md`,
`~/aWork/custom-element/docs/rendering.md`, and the legacy test files) are mined into
a functional feature inventory. Each legacy behavior that remains in scope becomes a
named Storybook parity story for `<cem-element>` with equivalent assertions. Behaviors
that are intentionally replaced by CEM-ML/CEM-QL are recorded as migration decisions,
not silently dropped.

1. **Functional parity with `<custom-element>`.** Every in-scope public behavior from
   the old `<custom-element>` functional suite and docs reproduces under
   `<cem-element>` with a one-to-one Storybook parity story.
2. **Template and data-island isolation.** Fixtures assert that declaration template
   source and instance data-island contents are backed by `<template>` content. Raw
   declaration or data-island descendants do not render, match document selectors,
   submit form data, or enter the accessibility tree, and only the rendered
   projection affects the UI.
3. **Material parity.** Every component in
   `~/aWork/custom-element-dist/src/material/` — `action.html`, `autocomplete.html`,
   `badge.html`, `dropdown.html`, `icon.html`, `icon-link.html`, `input.html`,
   `menu.html` — is rebuilt under `<cem-element>` with paired Storybook material
   parity stories. The rendered DOM, accessibility tree, and keyboard behavior match
   the legacy versions on a documented browser matrix. The story set MUST cover
   local/external `src`, hidden declarations,
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

When (1)–(6) are green, the `cem-element` browser substrate is ready for the separate
Edge/SSR follow-up phase. The next-major `@epa-wg/custom-element` adoption waits until
that follow-up phase is complete.

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
