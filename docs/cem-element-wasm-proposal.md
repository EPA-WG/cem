# `cem-element` WASM Integration Proposal

**Status:** Proposal for Phase 3 runtime planning.
**Design home:** [`cem-element-design.md`](./cem-element-design.md).
**Parser/runtime basis:** [`cem-ml-stack-design.md`](./cem-ml-stack-design.md),
[`cem-ml-ac.md`](./cem-ml-ac.md), and
[`cem-ql-stack-design.md`](./cem-ql-stack-design.md).

This proposal describes how the browser-side `cem-element` runtime can use the
`cem_ml` WASM build for declaration-template parsing, CEM-QL compilation, render-plan
generation, source-map reporting, and eventually parallel processing. It is a proposal,
not a replacement for the `cem-element` design. The core model stays unchanged:

- `<cem-element>` is the declaration element. It declares a produced custom element tag
  and owns the associated declaration template source.
- Produced custom element instances own mutable data islands wrapped in WHATWG
  `<template data-cem-island="instance">` so browser rendering, layout, CSS matching,
  forms, and accessibility never consume the raw data island directly.
- DOM events update the instance data island. Data changes invalidate render scopes.
  The runtime re-renders visible light DOM from the compiled template plus current
  data.
- `<custom-element>` remains the published tag under `@epa-wg/custom-element`. Its
  monorepo migration and next-major implementation adoption happen in a follow-up
  adoption phase after the browser substrate and Edge/SSR follow-up phase are stable.

## 1. Existing Capabilities To Reuse

The `cem_ml` stack already defines the primitives that `cem-element` should consume
rather than rebuild:

- **Streaming sources.** AC-F-7 and AC-P-2 require async Rust/WASM entry points that
  accept finite source adapters or streams. The implemented `source` module already
  models chunked byte/string/file sources with absolute byte ranges.
- **Source-map-preserving parser layers.** The parser pipeline carries `SourceId`,
  byte ranges, and source-map stacks through tokenization, normalization, validation,
  AST construction, and transforms.
- **Template and machine-state model.** `cem-ml-stack-design.md` models templates as
  scoped transform resources addressed by local id, URL, URL fragment, registry entry,
  schema identity, or custom-element tag name. Machine-state data feeds template
  application and hydration.
- **Scoped URL policy.** URL-bearing fields are resolved against the owner scope's base
  URL, module or import map, substitution rules, and resource policy.
- **Scheduler shape.** AC-A-4..AC-A-6 define scope-owned CPU worker limits, bounded
  queues, and a separate external-resource I/O queue. Browser/WASM default CPU pool
  size is `min(navigator.hardwareConcurrency, 8)`, floored at 1.
- **WASM package boundary.** `packages/cem_ml` already has a `wasm32` dependency on
  `wasm-bindgen` and an observer surface. `cem-element` needs additional WASM entry
  points for template compilation and render planning.

## 2. Source Declaration Forms

The declaration source can be inline or URI-backed. Inline declarations keep one
direct-child `<template>` per the "associated WHATWG template" invariant.
URI-backed declarations use `src` on `<cem-element>` itself, matching the legacy
`<custom-element src="…">` shape — see
[`cem-element-design.md` §3.2](./cem-element-design.md) for the normative rules.

### 2.1 Inline Declaration Template

```html
<cem-element tag="cem-button">
  <template type="text/cem-ml">
    {attribute @name="label" | Save}
    {button | ${$label}}
  </template>
</cem-element>
```

The inline template is inert browser content and is not the instance data island. The
runtime reads the template source and feeds it to `cem_ml`.

For canonical CEM-ML, `type="text/cem-ml"` should be the recommended form because text
content can be streamed in deterministic chunks. XML/HTML parity syntax can also live
inside the same template, but browser parsing may normalize the original bytes before
the runtime sees them.

### 2.2 URI-Backed Declaration Template

```html
<cem-element tag="cem-button" src="@epa-wg/cem-components/button.cem#button"></cem-element>
```

The `src` value is a template specifier, not browser-rendered content. It resolves
through the active `cem-element` module-map resolver and scope URL policy. The fragment
part identifies a named template or resource-defined fragment after the resource is
parsed. When `src` is set, the declaration MUST NOT contain an inline `<template>`
child; the URI form supplies the template body.

This shape preserves one-to-one parity with the legacy
`<custom-element src="…" tag="…">` declarations used in
`~/aWork/custom-element-dist/src/material/` (for example
`<custom-element hidden src="#cem-icon" tag="cem-icon">` and
`<custom-element src="./icon-link.html#cem-icon-link" tag="cem-icon-link">`),
which makes material-parity migration mechanical.

### 2.3 Rejected Alternates

Two alternate shapes were considered and rejected:

- `<template src="…">` on the inner template. Rejected because the legacy POC puts
  URI on the declaration element, and putting it on the inner template would force
  authors to wrap every URI-backed declaration in an empty `<template></template>`
  for no benefit.
- `<cem-element template-src="…">`. Rejected because it splits source identity
  across two attribute names (`src` vs. `template-src`) for the same concept and
  does not match any legacy POC usage.

`src` on `<cem-element>` is the single URI declaration form.

## 3. Module-Map And URI Resolution

`cem-element` should own a small resolver abstraction instead of assuming direct access
to the browser's internal import-map table:

```ts
type CemTemplateResolver = (specifier: string, context: {
  baseUrl: string;
  scopeId: string;
  contentTypeHint?: string;
}) => Promise<URL>;
```

The default resolver can support:

- absolute `https:`, `http:`, and same-origin application URLs allowed by scope policy;
- relative URLs resolved against the declaration document or module base URL;
- fragment-only references such as `#button-template`;
- package-like specifiers through a user-provided module map;
- ESM-hosted resolution where available, for example a resolver backed by
  `import.meta.resolve`.

The browser import-map standard is useful for author intent, but it is not a complete
runtime API for arbitrary resource resolution. Treat "module map" as the `cem-element`
host resolver contract. A browser import map, bundler manifest, CDN map, or app-provided
resolver can feed that contract.

Resolution output must include:

- the final URL and fragment;
- resolved content type or a content-type hint;
- resource-policy identity used for cache keys;
- optional expected integrity hash;
- base URL for nested relative references.

## 4. Streaming Model

### 4.1 Remote URI Source Streaming

For URI-backed templates, the runtime should fetch and parse through a stream when the
platform supports it:

1. Resolve `template[src]` through the module-map resolver and scope policy.
2. Acquire an external-I/O queue permit. Fetching MUST NOT consume CPU worker slots.
3. Start `fetch(resolvedUrl, { signal })`.
4. Feed `response.body` chunks into a WASM `ReadableStream` or host byte-source
   adapter. If `response.body` is unavailable, fall back to `arrayBuffer()`.
5. `cem_ml` decodes, tokenizes, validates, and compiles while preserving URL-backed
   source maps.
6. Cache the compiled template artifact by content hash plus resource-policy identity.

Streaming remote source is useful even when the final render plan needs the full
template before use. It reduces peak memory, exposes diagnostics earlier, and aligns
template loading with the parser's source-map model.

### 4.2 Local Parser Streaming

Inline templates should still enter the parser through a stream-shaped adapter:

- `type="text/cem-ml"` can stream text node content in bounded UTF-8 chunks.
- XML/HTML parity content can stream a canonical serialization of `template.content`,
  but exact original byte offsets are no longer recoverable after the browser parser has
  constructed the DOM.
- For exact author-byte source maps, authors should prefer external `.cem` resources or
  a raw text inline form. DOM-derived inline source maps can still point to declaration
  nodes and synthesized offsets, but they are not equivalent to original network bytes.

This distinction matters for devtools: external and raw-text CEM-ML can trace rendered
DOM to original byte offsets; DOM-parsed inline HTML parity may only trace to a
canonicalized inline source frame.

Phase 3 source-map references carry an explicit fidelity marker:

```ts
type SourceMapFidelity =
  | "author-byte-exact"
  | "dom-canonical"
  | "declaration-only";
```

- `author-byte-exact` is required for external `.cem` resources, fetched resources, and
  raw text inline forms where the original source bytes enter the CEM parser.
- `dom-canonical` is the accepted fidelity for DOM-parsed inline XML/HTML parity
  templates. Ranges point into a canonical serialization of `template.content`, not the
  author's original file bytes.
- `declaration-only` is a fallback for diagnostics that can only identify the owning
  declaration element or template node. It is not sufficient for material parity when a
  canonicalized DOM source frame can be produced.

Material parity fixtures MAY pass with `dom-canonical` source-map fidelity for
DOM-parsed inline XML/HTML templates. They MUST NOT claim `author-byte-exact` unless the
original source bytes were available to the CEM parser.

## 5. WASM Runtime Boundary

The current WASM observer surface exposes parse/validate/transform events. `cem-element`
needs a host-facing runtime API layered above that observability. A minimal shape:

```ts
interface CemWasmTemplateEngine {
  compileTemplateSource(source: CemSourceInput, options: CompileOptions): Promise<TemplateArtifact>;
  compileTemplateStream(stream: ReadableStream<Uint8Array | string>, options: CompileOptions): Promise<TemplateArtifact>;
  compileQuery(source: string, options: QueryCompileOptions): Promise<QueryArtifact>;
  renderPlan(artifact: TemplateArtifact, data: DataIslandSnapshot, options: RenderOptions): Promise<RenderPlan>;
  diffPlan(previous: RenderPlan | null, next: RenderPlan, options: DiffOptions): Promise<DomPatchPlan>;
  diffFrames(previous: RenderPlan | null, next: RenderPlan, options: DiffOptions): ReadableStream<PatchFrame>;
  releaseArtifact(artifactId: string): void;
}
```

The browser DOM remains main-thread-owned. WASM and workers should return render plans,
diff plans, patch frames, diagnostics, and source maps. Main-thread host code applies
DOM patches, attaches event listeners, preserves focus/selection, and updates the
hidden instance data-island template.

Template artifacts should include:

- normalized declaration metadata, including declared attributes and observed attribute
  names;
- compiled CEM-QL expression artifacts for attribute values, text interpolation,
  `select`, `slice-value`, and event payload extraction;
- render-scope invalidation metadata;
- source-map tables;
- cache identity and policy stamp;
- compatibility flags for legacy `<custom-element>` bridge templates when
  `lang="custom-element-v0"` is used.

## 6. Host Runtime Support Layer

The implementation should insert a JS/TS host-runtime support layer between `cem_ml`
and `cem-element`. This layer keeps the reusable browser/worker runtime separate from
the custom-element authoring surface. The normative serializable processing boundary is
defined in [`cem-element-design.md` §4.2](./cem-element-design.md#42-serializable-processing-boundary);
this proposal keeps the remaining transport and deployment tradeoffs.

Responsibilities should split as follows:

- **`cem_ml` Rust/WASM engine:** parses, validates, and compiles CEM-ML/CEM-QL; owns
  template artifacts and render plans in WASM memory; computes render/diff output; emits
  diagnostics, source maps, `DomPatchPlan` objects, or `PatchFrame` streams. It does not
  know about `customElements`, produced element lifecycle, or browser data-island DOM
  capture.
- **Host runtime support layer:** wraps `cem_ml` for browser/Node-style hosts. It owns
  worker startup, artifact handles, module-map resolution, source streaming adapters,
  external-I/O queue integration, artifact caching, patch-frame transport, flush
  scheduling, the host-neutral `PatchApplier` contract, diagnostics forwarding, and
  scheduler traces. It may provide a generic browser patch applier, but only as a
  target-root-driven utility. It should not discover `<cem-element>` declarations or
  own produced custom element lifecycle.
- **`cem-element` layer:** owns declaration discovery, produced custom element
  lifecycle, instance data-island capture, data-island snapshots, event-to-data wiring,
  instance ids, target roots, and invocation of the browser DOM patch applier for each
  produced element instance.

Yes, the support layer is useful beyond `cem-element`. It can also serve:

- the migrated `<custom-element>` adapter in `@epa-wg/custom-element`;
- docs and playground previews that render CEM templates without registering custom
  element declarations;
- test harnesses that need deterministic render plans, patch frames, and scheduler
  traces;
- devtools or diagnostics viewers that inspect source maps, artifacts, and invalidation
  decisions;
- future framework or server adapters that want CEM template + data rendering without
  adopting the `<cem-element>` browser lifecycle.

This boundary prevents `cem_ml` from becoming browser-lifecycle code and prevents
`cem-element` from becoming the only owner of the generic worker/cache/patch protocol.

Packaging follows the staged Option D path:

- Phase 3A implements the support layer as an internal package-private module inside
  `@epa-wg/cem-elements`, for example
  `@epa-wg/cem-elements/internal/runtime-support`.
- The internal module MUST be authored as if it will later become
  `@epa-wg/cem-runtime-support`: no declaration discovery, no `customElements`
  registry ownership, no produced-element lifecycle ownership, and no direct assumption
  that the caller is `<cem-element>`.
- `@epa-wg/cem-elements` remains the only package that consumes this module during
  Phase 3A, except for local tests and fixtures.
- Extraction to a separate reusable package is deferred until material parity passes,
  patch/cache/source-map worker contracts have fixture coverage, the Edge/SSR
  follow-up phase is green, and the later `<custom-element>` adoption phase begins
  consuming the substrate.
- When extracted, the package name is reserved as `@epa-wg/cem-runtime-support`.

## 7. Deployment Topologies Enabled By The Split

The UI/processing split should be designed as a serializable host boundary. Browser
worker execution is the Phase 3 target. Edge and SSR processing are follow-up
topologies that reuse this boundary after the browser substrate is stable.

### 7.1 Browser WASM Worker Or Worker Pool

The browser mode keeps the full `cem-element` UI adapter on the main thread and moves
processing work to one or more workers:

- declaration sources, URI streams, and data-island snapshots cross into the worker;
- template artifacts and previous render plans stay in worker/WASM memory when
  possible;
- workers return `DomPatchPlan` objects, `PatchFrame` streams, diagnostics, and
  source-map references;
- the main thread performs committed DOM patches and browser lifecycle work.

This is the default production direction because it reduces main-thread work while
keeping event handling, focus, form state, and DOM mutation local to the browser.

### 7.2 Edge Processing With Stored Data And Virtual DOM

A compute-CDN or edge-worker host can run the processing layer close to the user or data
store. The host can combine:

- a template artifact or source URI;
- a serialized `DataIslandSnapshot`;
- a stored previous `RenderPlan` or virtual DOM snapshot;
- scope policy, resolver identity, cache identity, and `RenderRevision` metadata.

The edge worker can return rendered HTML for first paint, a fresh `RenderPlan`, or a
patch-frame stream for the browser UI adapter to apply. The accepted first storage
model is content-addressed cache plus revisioned pointer records: immutable blobs store
template artifacts, render plans, rendered HTML fragments, and policy-sanitized
snapshot exports by content address; a small KV/document record stores the current
`RenderRevision`, content addresses, scope/privacy policy stamps, and an ETag-like
compare value. Full data snapshot retention remains opt-in by export policy. It MUST
NOT be described as storing the live browser DOM.

This topology is useful for server-assisted first render, precomputed component
fragments, collaborative/shared state, and low-latency data-adjacent rendering. It is
not a universal replacement for local browser processing:

- every update pays network latency and serialization cost;
- edge data stores may be eventually consistent, so `RenderRevision` and data revision
  identities must be explicit;
- sensitive data-island contents require a clear policy before leaving the browser;
- transient browser state such as focus, selection, in-progress text composition, and
  form control internals cannot be reconstructed reliably at the edge;
- offline and poor-network behavior still need a local browser fallback.

### 7.3 Server-Side Rendering

The same processing layer can run in a server host to emit:

- rendered HTML for the produced custom element output;
- hydration metadata that identifies template artifact, data revision, source-map
  markers, and retained render-plan identity;
- optional initial data-island content wrapped in inert `<template>` form for the
  browser UI adapter.

SSR is out of Phase 3 execution scope. It remains a follow-up processing-host phase
built on the same serializable boundary after the browser worker substrate is stable.
In that follow-up phase, SSR remains a bootstrap path, not a separate runtime
semantics. After hydration, the browser UI adapter owns event-to-data writes and DOM
patching. The browser may continue using local worker processing or ask an edge/server
host for later render plans, but that is a deployment choice behind the same processing
contract.

## 8. Patch-Frame Stream And Flush Policy

Large render updates can distribute a `DomPatchPlan` as an ordered stream of internal
patch frames. The stream is an implementation protocol, not browser event dispatch:

```ts
type PatchFrame =
  | { type: "begin"; transactionId: string; revision: RenderRevision }
  | { type: "ops"; transactionId: string; batchIndex: number; ops: DomPatchOp[] }
  | { type: "commit"; transactionId: string; nextRenderPlan: RenderPlanIdentity }
  | { type: "abort"; transactionId: string; diagnostic: Diagnostic };
```

The default policy is **per-instance patch transactions with a batched main-thread
flush**:

- Each produced custom element instance owns its latest requested `RenderRevision`,
  previous render plan, pending patch transaction, and target root.
- WASM/worker code may stream `ops` frames for large templates or return one complete
  `DomPatchPlan` for small components.
- The main thread buffers frames by `transactionId` until `commit`.
- Stale frames whose `revision` does not equal the instance's latest requested revision
  are dropped.
- `ops` frames carry zero-based `batchIndex` values; duplicate, missing, or
  out-of-order batches abort the transaction.
- `commit` makes the instance transaction ready for the next host flush; it does not
  require immediate DOM mutation.
- The host flush batches ready instance transactions on a host-selected microtask or
  `requestAnimationFrame` policy boundary, then applies each instance transaction
  synchronously and atomically to browser DOM.
- Normal diffs target stable render-node ids. `replaceScope` is allowed only for first
  render, fallback mode, explicit policy replacement, or recovery after target
  mismatch.
- A failed or mismatched transaction aborts only that instance's pending render unless
  an ancestor scope policy explicitly escalates the failure. A target mismatch emits a
  diagnostic and permits a `replaceScope` recovery transaction; individual failed ops
  are not skipped.

`DomPatchPlan` is the one-shot equivalent of `begin + ops + commit`. The host-neutral
`PatchApplier` receives streamed frames or a one-shot plan, buffers until commit,
returns `applied`, `stale`, `aborted`, or `mismatch`, and owns the target-root-specific
node table used to map stable render-node ids to host DOM nodes.

Browser `EventTarget` / DOM `Event` dispatch MUST NOT be used as the patch transport.
Reasons:

- DOM events are observable and reentrant, so they can expose half-applied render state.
- Event dispatch is too expensive for per-node patch operations.
- DOM events blur internal render protocol with author-visible UI events.
- Worker-to-main-thread transfer still requires serialization; `MessagePort`,
  `ReadableStream`, or `postMessage` frames are the right transport boundary.
- MutationObserver timing must stay tied to committed DOM batches, not individual patch
  operations.

Browser DOM events remain user/input events. They are consumed by `cem-element` event
bindings, written into the instance data island, and then scheduled as render
invalidations.

## 9. Parallel Work Scheduling

`cem-element` can exploit parallelism without moving DOM mutation off the main thread.
Useful parallel jobs:

- compile independent `<cem-element>` declarations discovered during startup;
- fetch and compile URI-backed templates while other declarations compile;
- compile CEM-QL expressions inside a template in parallel after tokenization exposes
  expression spans;
- prepare first-render plans for multiple instances of already compiled declarations;
- process data-island invalidation batches for different instances;
- precompile external material-component templates before they are connected.

The scheduler should preserve deterministic user-visible order:

- each declaration receives a monotonically increasing sequence number at discovery;
- diagnostics are reported in declaration/source order even when work finishes out of
  order;
- a produced custom element instance waits on the artifact promise for its declaration;
- DOM patch application is serialized on the main thread per instance;
- stale jobs are discarded with `AbortSignal` when a declaration is removed or source
  changes.

The browser implementation can use two levels of parallelism:

- **Worker pool with one WASM instance per worker.** Works without
  `SharedArrayBuffer`. Artifacts are copied or transferred as structured data. This is
  the broadest browser-compatible pool.
- **Threaded WASM inside workers.** Uses `SharedArrayBuffer` and atomics when the page is
  cross-origin isolated and the build target supports it. This can reduce duplication
  for large caches, but it must have a non-threaded fallback.

External resource streams use the I/O queue and do not occupy CPU slots. CPU-bound
parse, validate, query compile, and render-plan work use the scope's worker-pool cap.

Phase 3A starts with a **single dedicated worker** as the default worker-backed mode.
The worker owns one WASM instance plus retained template artifacts and render plans.
This proves the worker/stream/cache/patch contract without introducing pool scheduling
or multi-worker artifact coherency. A scope-policy worker pool is deferred to Phase 3B
and remains an optimization behind the same host runtime API.

Fallback behavior is deterministic:

- If `Worker` is unavailable, blocked by policy, or fails during startup, the host
  runtime aborts pending worker jobs, emits a diagnostic, and retries through the
  main-thread WASM fallback.
- If `SharedArrayBuffer` is unavailable because the page is not cross-origin isolated
  or the target lacks support, Phase 3A behavior does not change; the runtime uses the
  non-threaded dedicated worker path.
- If a later threaded-WASM mode is requested but `SharedArrayBuffer` is unavailable,
  the runtime falls back to non-threaded worker message passing. If that worker path is
  also unavailable, it falls back to main-thread WASM.
- Main-thread fallback MUST preserve the same template, data, render-plan, diagnostic,
  source-map, and patch-frame semantics as the worker path.

## 10. Options

### Option A - Main-Thread WASM, Inline-First MVP

Use a single WASM instance on the main thread. Inline templates compile through
`compileTemplateSource`; URI templates may fetch the whole response before compile.

Pros:

- smallest runtime surface;
- easiest to debug;
- no worker bundling, transferable artifact, or cross-origin isolation requirement;
- good fallback path for tests and older browsers.

Cons:

- large templates can block interaction;
- remote URI streaming is limited;
- no meaningful use of the CEM scheduler model;
- material parity may pass functionally but miss the intended production architecture.

Best use: fallback mode and earliest proof of the CEM-ML/CEM-QL template compiler in
the browser.

### Option B - Worker-Backed WASM With Stream Inputs

Run template compilation in a dedicated worker or small worker pool. Inline templates
enter through local stream adapters. URI templates enter through `fetch()` response
streams. The main thread owns custom-element lifecycle and DOM patch application.

Pros:

- exercises the streaming source contract for both inline and remote templates;
- avoids main-thread parse/compile cost;
- does not require `SharedArrayBuffer`;
- maps cleanly to the current AC model: async APIs, I/O queue, bounded CPU queue, and
  deterministic diagnostics;
- enough architecture to prove material-component parity under realistic loading.

Cons:

- requires worker packaging and artifact serialization;
- duplicated WASM memory across workers;
- not as fast as a shared-memory threaded build for very large template graphs.

Best use: recommended Phase 3 MVP target, with Option A as fallback.

### Option C - Multi-Worker Pool Plus Shared Artifact Cache

Use a browser worker pool sized by scope policy, defaulting to
`min(navigator.hardwareConcurrency, 8)`. Each worker owns a WASM instance. A main-thread
or dedicated cache coordinator stores compiled artifacts keyed by content hash, URL, and
policy stamp.

Pros:

- parallelizes startup across many component declarations;
- supports prefetch/precompile of material-component template sets;
- aligns with AC-A-4 queue and worker-pool semantics;
- lets URI streams and local templates progress concurrently.

Cons:

- more scheduler complexity;
- cache invalidation must include scope policy, resolver identity, and CEM-ML/CEM-QL
  versions;
- artifact transfer cost can dominate small templates.

Best use: production path after Option B proves the API and parity fixtures.

### Option D - Threaded WASM With Shared Memory

Build `cem_ml` with threaded WASM support and run a pool behind one shared engine when
the page is cross-origin isolated.

Pros:

- strongest fit for CPU-heavy parse/validate/compile workloads;
- shared dictionaries and caches can reduce duplicated memory;
- closer to native-host worker-pool behavior.

Cons:

- requires cross-origin isolation headers for `SharedArrayBuffer`;
- increases build and deployment complexity;
- cannot be the only browser path;
- needs careful audit of Rust/WASM thread safety and deterministic report sequencing.

Best use: performance tier for controlled deployments, not the default compatibility
baseline.

### Option E - Precompiled Template Artifacts With Runtime Fallback

Add a build-time path that compiles `.cem` templates into binary or JSON template
artifacts. Runtime WASM validates cache identity and only reparses source for cache
misses, dev-mode source maps, dynamic inline templates, or policy mismatches.

Pros:

- fastest first render for packaged components;
- pairs well with content-addressed cache work;
- lets `@epa-wg/cem-components` ship stable material templates without runtime parse
  cost for the common path.

Cons:

- requires artifact versioning and source-map sidecars;
- build pipeline must stay in lockstep with runtime `cem_ml`;
- dynamic app-authored declarations still need source parsing.

Best use: production optimization after the source-driven path is already correct.

### Option F - Service-Worker Template Registry

Use a service worker to cache remote template source and compiled artifacts for
application shells that load many URI-backed declarations.

Pros:

- improves repeat visits and offline-ish demos;
- centralizes remote template caching;
- can prewarm material-template bundles.

Cons:

- service worker lifecycle adds operational complexity;
- not available in all embedding contexts;
- compiled artifact storage must be versioned and policy-aware.

Best use: optional application-level cache strategy after component parity, not core
Phase 3 substrate behavior. Phase 3 only defines the compatible cache identity and
artifact-registry hook contract.

### Option G - Edge/SSR Processing Host

Run the host runtime support layer outside the browser: in an edge worker, server
worker, or SSR process. The browser `cem-element` UI adapter sends or receives
serialized data snapshots, render-plan identities, HTML, or patch-frame streams.

Pros:

- enables SSR and server-assisted first paint;
- can render close to data stored in an edge KV/document database;
- allows expensive compilation or rendering to be precomputed and cached;
- can support non-browser consumers that need CEM template + data rendering.

Cons:

- introduces network latency and offline fallback requirements;
- requires explicit data privacy, policy, and revision handling;
- cannot own browser DOM, focus, form internals, or user input composition;
- adds cache coherency concerns between server/edge render plans and client hydration.

Best use: optional deployment topology after the browser-worker contract is stable.

## 11. Recommendation

Adopt a staged path. The Phase 3 MVP topology is now locked in
[`cem-element-design.md` §4.3](./cem-element-design.md#43-phase-3-mvp-topology):
Option B is the primary path and Option A is the required fallback. The remaining
options stay post-MVP unless a later task explicitly promotes them.

1. **Phase 3A:** implement Option B as the primary architecture and Option A as the
   fallback. This proves CEM-ML/CEM-QL compilation in WASM, local parser streaming,
   remote source streaming, the host runtime support layer, patch-frame protocol, and
   main-thread DOM patch ownership without requiring shared memory.
2. **Phase 3B:** extend Option B into Option C with a scope-policy worker pool,
   content-addressed artifact cache, deterministic scheduler traces, and parallel
   material parity fixture compilation.
3. **Phase 3C:** add Option E for packaged `@epa-wg/cem-components` templates once
   the source-driven path is stable.
4. **Later performance tier:** add Option D only for deployments that can guarantee
   cross-origin isolation. Keep Option A/B fallback permanently.
5. **Edge/SSR follow-up phase:** move Option G out of Phase 3. Add it only after the
   browser-worker processing boundary, artifact identities, data snapshot shape, and
   patch-frame protocol are stable.
6. **Post-component-parity application cache tier:** consider Option F for the CEM
   site, docs, playgrounds, or demos after the component set has proven parity. Phase 3
   keeps only the service-worker-compatible artifact identity and optional registry
   hooks.

This sequence uses the CEM-ML streaming and scheduling model early, but does not block
functional parity on browser shared-memory deployment constraints.

## 12. Compatibility With `<custom-element>`

The WASM substrate should serve both declaration surfaces:

- New code uses `<cem-element>` with CEM-ML/CEM-QL declaration templates.
- The external `@epa-wg/custom-element` package keeps publishing `<custom-element>`
  until the post-Edge/SSR adoption phase. In that later phase, `<custom-element>`
  adapts its legacy public attributes and optional `lang="custom-element-v0"` template
  body into the same internal declaration record used by `cem-element`.

The future adapter must not give legacy `<custom-element>` a separate parser/render
engine. Functional parity is achieved when both public tags share the same data-island
lifecycle, event-to-data wiring, invalidation model, and light-DOM patching path.

## 13. Verification Gates

The selected MVP should add Storybook stories as the primary browser fixtures. Each
story is an executable verification case for Storybook Test and may include
browser assertions over rendered DOM, data-island templates, event writes, focus/form
behavior, source-map metadata, and diagnostics. Pure Rust/TypeScript unit tests can
cover helpers and serialization, but user-visible runtime behavior is accepted through
Storybook. The old `@epa-wg/custom-element` test suite is used to build the parity
feature inventory: every in-scope legacy behavior becomes a named Storybook parity
story, while intentional CEM-ML/CEM-QL replacements are documented as migration
decisions.

The selected MVP should add Storybook verification cases for:

- inline `type="text/cem-ml"` declaration template compiling through WASM;
- inline XML/HTML parity declaration template with documented source-map limitations;
- URI-backed declaration template resolved through a module-map resolver;
- remote URI fetched through `ReadableStream` and canceled through `AbortSignal`;
- fragment references such as `package/button.cem#button`;
- multiple declarations compiling in parallel with deterministic diagnostic order;
- produced element instances whose raw payload is captured into
  `<template data-cem-island="instance">` before first render;
- data-island mutation causing CEM-QL-backed invalidation and light-DOM patching;
- worker unavailable fallback to main-thread WASM;
- shared-memory unavailable fallback from threaded WASM to worker message passing;
- patch-frame streaming for a large render and one-shot `DomPatchPlan` delivery for a
  small render;
- per-instance patch transactions with stale `RenderRevision` frame dropping;
- indexed `ops` batches rejecting duplicate, missing, or out-of-order frames;
- `moveBefore` preserving DOM node identity across list reorders;
- target mismatch aborting the transaction, emitting a diagnostic, and recovering
  through constrained `replaceScope`;
- assertion that normal data-island mutation uses render-node-id ops rather than
  `replaceScope`;
- batched main-thread flush across multiple ready instance transactions;
- assertion that patch transport uses internal worker/stream/message frames and never
  browser `EventTarget` / DOM `Event` dispatch;
- assertion that Phase 3 runs correctly with no service worker or Cache Storage
  registry, while artifact cache identities and optional registry hooks remain stable;
- assertion that Phase 3 does not require SSR/edge processing hosts; those fixtures
  move to the Edge/SSR follow-up phase;
- assertion that Phase 3 does not migrate or adopt `@epa-wg/custom-element`; that work
  moves to the post-Edge/SSR adoption phase.

Performance gates should measure:

- cold inline compile time;
- cold remote streaming compile time;
- warm artifact-cache render time;
- main-thread blocking time during startup;
- worker-pool queue depth and cancellation behavior;
- worker-message/frame count per render and per batch;
- DOM mutation/layout cost per batched flush;
- memory retained per compiled template artifact.

## 14. Open Decisions

- ~~Should URI source live only on the associated `<template src="...">`, or should
  `template-src` on `<cem-element>` be accepted as an alias for legacy ergonomics?~~
  Resolved: URI lives on `<cem-element src="…">`, matching the legacy
  `<custom-element src="…">` shape. See §2.2 / §2.3 and
  [`cem-element-design.md` §3.2](./cem-element-design.md).
- ~~What is the exact JS/WASM artifact wire format: JSON, binary AST, transferable
  `ArrayBuffer`, or a hybrid?~~ Resolved: Phase 3 uses the hybrid Option D wire
  format from
  [`cem-element-design.md` §4.2](./cem-element-design.md#42-serializable-processing-boundary):
  structured-clone plain-record envelopes for control, retained worker/WASM handles
  by default, JSON-compatible diagnostics, and transferable `ArrayBuffer` payloads
  for cacheable or large artifacts. The envelope is designed to be Option
  C-compatible so template artifacts, source-map sidecars, render-plan snapshots, and
  future patch-op batches can move to binary payloads without changing the semantic
  worker API.
- ~~Which source-map fidelity level is required for DOM-parsed inline HTML parity
  templates before material parity can pass?~~ Resolved: source maps carry
  `SourceMapFidelity = "author-byte-exact" | "dom-canonical" | "declaration-only"`.
  Material parity may pass with `dom-canonical` fidelity for DOM-parsed inline XML/HTML
  templates whose original browser source bytes are unrecoverable. `author-byte-exact`
  remains required for external `.cem`, fetched resources, and raw text inline forms;
  `declaration-only` is fallback-only and is not sufficient when a canonical DOM frame
  can be produced.
- ~~Does the first worker-backed implementation run one worker or a small pool by
  default?~~ Resolved: Phase 3A uses one dedicated worker by default, with
  main-thread WASM fallback when workers are unavailable or fail startup. A
  scope-policy worker pool is deferred to Phase 3B. `SharedArrayBuffer` is optional;
  threaded WASM falls back to non-threaded worker message passing and then to
  main-thread WASM if workers are unavailable.
- ~~Which cache identities are mandatory for Phase 3: source hash only, or source hash
  plus resolver identity, scope policy stamp, `cem_ml` version, and `cem_ql`
  version?~~ Resolved: Phase 3 uses the two-level Option C identity from
  [`cem-element-design.md` §4.2](./cem-element-design.md#42-serializable-processing-boundary).
  `TemplateArtifactPayloadKey` identifies portable bytes by content type, source hash,
  `cem_ml` version, `cem_ql` version, and dev/prod source-map mode.
  `TemplateArtifactIdentity` adds source ref, resolver identity, and
  `scopePolicyStamp` for host-specific use. Render plans are keyed by template
  artifact identity, `RenderRevision`, render engine version, and source-map mode.
- ~~Does the host runtime support layer start as an internal package-private module or
  as a separately published package once `<custom-element>` begins consuming it?~~
  Resolved: Phase 3A starts with an internal package-private
  `@epa-wg/cem-elements/internal/runtime-support` module authored for later extraction
  to `@epa-wg/cem-runtime-support`. Extraction waits until material parity passes,
  worker/cache/source-map contracts have fixture coverage, the Edge/SSR follow-up
  phase is green, and the later `<custom-element>` adoption phase begins consuming the
  substrate.
- ~~Is Phase 3 responsible for edge/SSR runtime delivery or SSR verification
  fixtures?~~ Resolved: no. Phase 3 keeps the serializable boundary and topology notes,
  but SSR/edge fixtures and runtime delivery move to a separate Edge/SSR follow-up
  phase after the browser worker substrate is stable.
- ~~When does `@epa-wg/custom-element` move into this monorepo and adopt the
  substrate?~~ Resolved: after the Edge/SSR follow-up phase. Phase 3 proves the browser
  `cem-element` substrate and compatibility path, but the `@epa-wg/custom-element`
  monorepo migration and next-major substrate adoption are a later adoption phase.
- ~~Which data-island fields are allowed to leave the browser for edge processing, and
  how is that policy expressed per scope?~~ Deferred: Phase 3 keeps the redaction and
  omission requirement in the serializable boundary, but the concrete field policy is
  a Phase 3.5 Edge/SSR follow-up verification fixture.
- ~~Which storage model is supported first for edge render state: content-addressed
  cache only, revisioned KV/document records, or both?~~ Deferred: the first edge
  render-state storage choice is a Phase 3.5 Edge/SSR follow-up decision, not a Phase 3
  browser-substrate gate.
- ~~Should the CEM site use a service-worker registry, or should that stay outside the
  core substrate until after component parity?~~ Resolved: Phase 3 uses Option C. It
  defines service-worker-compatible artifact identity, namespace/version metadata, and
  optional registry hooks, but does not implement a service-worker template/artifact
  registry. A concrete registry is deferred until after component parity, most likely
  in the CEM site, docs, playgrounds, or demos.
