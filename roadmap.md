# CEM Roadmap

CEM should become a complete consumer-semantics design system: tokens, documentation, schema-defined parser/runtime
tooling, web components, native adapters, Figma assets, and demos that all prove the same model from different angles.

This roadmap is intentionally higher level than `docs/todo.md`. Use this file to decide product/module order; use
`docs/todo.md` for task-level execution.

## Product Modules

| Module                        | Purpose                                                                                                                                                                                                                                                                                                     | Primary package or path                                                  |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| CEM token/theme core          | Canonical token specs, generated CSS, DTCG JSON, TypeScript metadata, and reports.                                                                                                                                                                                                                          | `packages/cem-theme`                                                     |
| Native platform adapters      | iOS Swift and Android Kotlin/Compose outputs generated from the same token spine.                                                                                                                                                                                                                           | `packages/cem-theme/dist/lib/token-platforms`                            |
| CEM parser/runtime foundation | Schema-defined streaming parser layers: byte decoding, tokenization, normalized events, validation, AST/source maps, binary AST chunks, and implementation handoff.                                                                                                                                         | `packages/cem_ml`                                                        |
| CEM structural lifecycle CLI  | Validation, load into the internal CEM AST/event model, query/transform, and export/convert across schema + content-type identities. Separate synchronized deployments provide the WASM runtime npm package, Node/WASM CLI npm package, and native Linux AMD64, Homebrew ARM64, and Windows AMD64 packages. | `packages/cem_ml`, `packages/cem_ml_cli`, future CLI deployment projects |
| CEM Studio                    | Installable local-first PWA and npm package for exercising CEM-ML validation, conversion, query, transformation, source-map, report, and graph workflows through editable projects and a bidirectional CLI Command view.                                                                                    | future `packages/cem-studio`, `@epa-wg/cem-components/studio`            |
| CEM custom-element substrate  | Declarative no-JS runtime centered on `<cem-element>`: scoped data islands, event-to-data wiring, and light-DOM re-render from CEM-ML/CEM-QL templates. Staged in `@epa-wg/cem-elements`; edge/SSR and `@epa-wg/custom-element` adoption are follow-up phases after the browser substrate is stable.        | `packages/cem-elements`, future `packages/custom-element`                |
| CEM component set             | Functional parity with the user-facing Angular Material component catalog, expressed in CEM semantics and implemented on the light-DOM `<cem-element>` substrate rather than Angular runtime code.                                                                                                          | `packages/cem-components`, `packages/cem-elements`                       |
| Figma UI Kit                  | Designer-facing components, variants, variables, usage examples, and governance workflow.                                                                                                                                                                                                                   | `examples/figma`, future design artifacts                                |
| CEM site                      | Public docs, token/component gallery, interactive examples, and release documentation wired from the repo root.                                                                                                                                                                                             | future `apps/cem-site` or static docs app                                |
| Figma site demo               | A realistic product demo: login, registration, profile, asset listing views, and threaded discussion.                                                                                                                                                                                                       | future `examples/figma-site-demo`                                        |
| Repo docs spine               | Root docs, package docs, generated API/token docs, examples index, and contribution/release docs.                                                                                                                                                                                                           | `README.md`, `docs/`, package docs                                       |

## Ordering Principles

1. Build the shared semantic spine before building demos.
2. Generate platform outputs from source-of-truth tokens; do not hand-author native or Figma values.
3. Keep parser layers explicit: byte source, decoder, tokenizer, normalized event stream, schema machine, AST builder,
   binary AST encoder, and implementation interpreter are separate contracts.
4. Prefer in-process typed structures or CEM binary/chunk streams over JSON serialization whenever the consumer can use
   them. Do not change binary format at internal/external boundaries unless an explicit converter edge requires it.
5. Carry source-map stacks and byte offsets through every parser, transform, generated node, and runtime handoff.
6. Treat embedded languages and mixed formats as scoped handoffs owned by the parent parser's return condition.
7. Prove components on the web before porting full UI examples into Figma/native.
8. Use demos as integration tests, not as the first source of component behavior.
9. Keep Angular Material as a reference benchmark for coverage and ergonomics, not as a required implementation
   dependency unless an Angular adapter is explicitly scoped later.
10. Keep the CEM-ML deployment family version-locked: common `cem_ml` owns the version; the WASM runtime npm, CLI npm,
    Studio npm/PWA, and native target packages are separate projects that publish the exact same version from one
    release commit.
11. Treat the current official Angular Material component catalog as the reusable-component parity baseline. When
    Studio needs a control or interaction that Angular Material already provides, implement and verify its
    `<cem-element>`-based `@epa-wg/cem-components` counterpart first, then consume it from Studio. A Studio-specific
    component or application-local implementation may lead only when no Angular Material counterpart exists.

## Phase 0 - Repo Spine And Docs

Goal: make the repo understandable from the root.

Deliverables:

- Replace the generated Nx sections in the root `README.md` with CEM-specific overview, quick start, package map, and
  links to docs.
- Add a root docs index that links token export docs, component docs, examples, native outputs, and release docs.
- Keep `docs/todo.md` as implementation detail and this roadmap as product sequencing.
- Add consistent package README structure for `cem-theme` and `cem-components`.

Exit criteria:

- A new contributor can start from root README and find build, docs, tokens, components, examples, and release flow.
- No important package relies on undocumented deep paths.

## Phase 1 - Token And Platform Foundation

Goal: finish the stable design-token contract before expanding the UI surface.

Current status: mostly implemented.

Deliverables:

- Canonical token extraction from markdown/XHTML into DTCG JSON.
- Generated CSS, TypeScript metadata, Figma mode files, reports, and flat per-mode JSON.
- Style Dictionary transform/filter contract.
- iOS Swift output and Android XML/Kotlin output.
- Validation for generated token modes, reports, native files, and package exports.

Remaining gates: none under Phase 1. Native toolchain compile gates (Swift, Kotlin/Compose) and the non-Figma
token-change smoke test moved to [Phase 8 - Native Platform Packages](#phase-8---native-platform-packages) where the
native artifacts they validate are owned. Figma-specific token validation moved to
[Phase 5 - Figma UI Kit](#phase-5---figma-ui-kit) so the gate lands alongside the kit it validates.

## Phase 2 - Schema-Defined Parser And Document Runtime

Goal: define the schema-driven parsing and document layer that CEM components, transforms, docs, and demos can share.

Deliverables:

- Structural data lifecycle requirement for `cem_ml` and `cem_ml_cli`: every supported format follows
  validate → load into internal AST/events → export, with format identity defined by content type plus schema/namespace.
- Root-scope run configuration shared by lib, WASM, and CLI: every document root is scope zero for its AST tree, with
  default content type, schema/version pins, default and named namespace bindings, module-map/resolver identity, base
  URI, scope policy, and budgets available as input/output options. APIs accept input/output spec arrays; CLI supports
  config files for CI/build reproducibility and repeatable CSV records for concise one-liners. Config parsing and
  normalization are owned by `cem_ml`; CLI, WASM, and Rust hosts provide raw config bytes or raw record strings plus
  config format identity and consume the same normalized `RunConfig`. Remote/custom module-map, input, and output
  URI handling is host-backed resolver behavior: local filesystem resolution is built in, while network or custom
  schemes require explicit resolver registration by scheme and operation purpose.
- Multi-document run context for build/CI validation and transformation: one invocation can process many inputs and
  outputs through a shared scheduler/thread-pool context while preserving per-document root-scope diagnostics,
  source maps, and resource accounting.
- Lifecycle adapter registry for content-type/schema-specific behavior. The generic CEM event/AST pipeline remains the
  internal spine; CEM-ML, HTML/XML parity, and XSLT 1.0 compatibility are adapters over that spine rather than separate
  command-specific engines.
- CLI format selection promoted from fixed `--from-format` / `--to-format` enums toward input/output content type and
  schema identity, while keeping the current enum flags as convenience aliases.
- XSLT 1.0 adapter implementation for the immediate custom-element compatibility profile: raw
  `custom-element-xslt`/XSLT 1.0-family input can be validated by CLI, loaded through the internal CEM AST/event model,
  and exported to canonical CEM-ML or debug projections.
- Layered runtime contract: byte source, encoding decoder, schema tokenizer, normalized event stream,
  schema-compiled state machine, interpreter AST builder, and implementation interpreter.
- CEM document schema for semantic screens, forms, navigation, lists, assets, profiles, messages, and embedded payloads.
- XML/HTML parser profile using visibly nested events, source spans, and schema frames rather than DOM construction
  inside the tokenizer.
- Scoped embedded-language handoff model for HTML `style`/`script`, XML CDATA or schema-tagged text, CSF fields, JSON
  string subdocuments, and future CSS/TypeScript/Rust-like regions.
- Typed document AST/DOM helper APIs for querying semantic roles, state, validation, relationships, source maps, and
  unresolved references.
- Source-map stack contract that preserves byte offsets as ground truth and derives line/column or UTF-16 positions as
  needed.
- Canonical CEM binary/stream formats for DOM, AST, and events, including subtree chunking for cache, transport, retry,
  query access, and parallel preprocessing. Implementation can start with an uncompressed debug encoding, but the
  roadmap target is a stable binary format that native CLI/WASM/worker/server hosts can consume without JSON
  reserialization.
- Parallel- and multicast-capable artifact streams: sealed binary chunks can be consumed by multiple downstream readers
  or workers, replayed from cache, and routed into CEM-QL/query APIs without changing their binary representation.
- Shared CEM-QL expression API and CLI runner: `cem-ql` owns the reusable expression schema used by templates,
  transforms, schema behaviors, and component bindings. The Rust API can compile and evaluate one standalone CEM-QL
  expression against a typed data/context input while preserving diagnostics, source maps, resolver-policy stamps, host
  capability policy, and typed result values. The CEM-ML CLI exposes the same engine path through the existing
  `transform` command: inline expressions use `--template-expression`, and file-backed expression transformations use
  `--template *.cem-ql` with data from content-type/schema-declared input resources.
- XSLT or transform pipeline from validated semantic documents into light-DOM custom-element markup.
- Validation reports for unknown elements, invalid state combinations, missing labels, broken references, unsafe
  content, unsupported embedded-language handoffs, and non-streamable schema features.
- Fixtures covering login, registration, profile, asset listing, and threaded discussion documents.

Exit criteria:

- `cem-ml validate --content-type custom-element-xslt <input>` validates legacy custom-element XSLT 1.0 compatibility
  input directly and reports unsupported constructs without requiring a separate conversion command.
- `cem-ml convert --content-type custom-element-xslt --to-content-type application/cem+xml <input>` loads through the
  same adapter registry and emits canonical CEM-ML with conversion diagnostics and source-map boundary information.
- A fixture CEM document can be decoded, tokenized, normalized into events, schema-validated, mapped into a typed AST,
  transformed to HTML, and rendered by the component runtime.
- Every generated node can be traced back through the source-map stack to the original source bytes or to the transform
  that generated it.
- Embedded `style`, `script`, CDATA/text, and CSF-like field payloads either validate through explicit scoped handoffs
  or produce actionable diagnostics.
- A standalone CEM-QL expression can be compiled and run against declared data through the Rust API and CEM-ML CLI, with
  the same parser, type, diagnostics, resolver-policy, source-map, and output contracts used when that expression is
  embedded in a template or schema behavior slot.
- The same fixture can feed docs/examples without copying business structure into multiple formats.

## Phase 2.5 - CEM-ML CLI Product And Deployment Foundation

Goal: turn the Phase 2 command contract into separately deployable, version-synchronized CLI packages before Studio,
IDEs, and CI integrations depend on it. The accepted package, platform, release, capability, and host-wire decisions
are canonical in [`docs/cem-ml-deployment-contract.md`](docs/cem-ml-deployment-contract.md); the broader Studio design
lives in [`docs/cem-studio.md`](docs/cem-studio.md), while editor and automation protocols live in
[`docs/integrations.md`](docs/integrations.md).

Deliverables:

- Keep typed command parsing, normalized run plans, execution, diagnostics, source maps, reports, capability discovery,
  and cancellation in common `cem_ml`; keep terminal/native host adaptation in common `cem_ml_cli`.
- Create a separate `@epa-wg/cem-ml` npm deployment project containing the generated low-level WASM runtime, types,
  schema-package assets, ABI/capability metadata, and no npm executable.
- Create a separate `@epa-wg/cem-ml-cli` npm deployment project with an exact same-version dependency on
  `@epa-wg/cem-ml`, browser and Node exports, and the npm `cem-ml` executable. The supported portable CLI target is
  WASM for Node; the browser export is a programmatic Studio/IDE surface rather than another shell platform.
- Create exactly three initial native deployment subprojects: Linux AMD64, macOS ARM64 distributed through Homebrew,
  and Windows AMD64. Each project builds, packages, signs, verifies, and publishes only its platform artifact.
- Preserve every versioned native CLI archive/binary as an asset on the matching tagged GitHub Release. Upload
  target-qualified checksums, signatures, SBOMs, and provenance alongside the binaries; Homebrew, APT, and Windows
  package metadata must resolve those version-qualified release assets rather than a mutable build URL. Stage the
  complete asset set before publication, enable immutable releases where available, and never replace or delete a
  published binary; corrections require a new common version. Promoted semi-native SEA executables follow the same
  rule.
- Make `packages/cem_ml/Cargo.toml` the authoritative version source and add a fixed `cem-ml-platform` release family.
  Synchronize the exact version into the CLI crate, npm manifests and exact internal dependencies, native package
  metadata, capability/version output, checksums, SBOMs, provenance, and release index.
- Define one machine command/capability/report contract for Node/WASM and native hosts, including explicit runtime and
  target identity, host-policy differences, source-map ranges, stable exit policy, progress, and cancellation.
- Add native/WASM parity fixtures plus clean-consumer npm pack/install tests and per-platform
  install/upgrade/uninstall smoke tests.
- Keep self-contained semi-native executables as a wishlist experiment: bundle Node, the CLI launcher, and WASM with
  native Node.js SEA for the same three platform coordinates. Treat archived/deprecated `pkg` as comparison or
  migration prior art, not the default production packager.

Exit criteria:

- The same portable fixture produces equivalent normalized results, diagnostics, reports, and source maps through the
  Rust library, Node/WASM npm CLI, and every supported native target, with documented capability differences only.
- `@epa-wg/cem-ml` and `@epa-wg/cem-ml-cli` install independently into clean consumers; only the CLI package installs
  the `cem-ml` npm executable, and it resolves exactly one same-version WASM runtime.
- Linux AMD64, Homebrew ARM64, and Windows AMD64 are separate Nx deployment projects and are the complete initial
  native support matrix.
- Every supported native CLI binary remains downloadable from its tagged GitHub Release with matching checksum,
  signature, SBOM, provenance, source commit, target identity, and common version metadata.
- Release verification fails on any version, dependency, source-commit, checksum, SBOM, provenance, or capability
  manifest drift from the common `cem_ml` version.

## Phase 3 - Custom-Element Runtime

Goal: establish the reusable declarative web runtime before building the full component catalog. Phase 3 has two
linked tracks: the **substrate** (`@epa-wg/cem-elements`) that delivers the `<cem-element>` declarative authoring tag,
and the **primitives** (`@epa-wg/cem-components`) that consume it. Design home for the substrate is
[`docs/cem-element-design.md`](docs/cem-element-design.md). WASM integration options for CEM-ML/CEM-QL template
compilation, inline and URI declaration sources, streaming, worker-pool scheduling, and post-Phase-3 edge/SSR
processing boundaries are proposed in
[`docs/cem-element-wasm-proposal.md`](docs/cem-element-wasm-proposal.md).

### 3.1 Substrate — `@epa-wg/cem-elements`

Deliverables:

- New `<cem-element>` declarative authoring tag, functional successor to `<custom-element>` from
  `@epa-wg/custom-element`. Same concept (data island, event-to-data wiring, data-to-template re-render); template
  surface lowers through `cem_ml` and expressions use CEM-QL instead of XPath.
- WHATWG `<template>`-wrapped declaration and instance data islands. Declaration content, captured author payload,
  slices, event payloads, and validation state stay associated with the component scope but are inert to the browser
  rendering engine; only the rendered projection is visible after upgrade.
- Migration-readiness contract for the future `@epa-wg/custom-element` adoption phase. Phase 3 proves the
  `cem-element` substrate and compatibility fixtures, but it does not move `@epa-wg/custom-element` into this
  monorepo or make `<custom-element>` inherit the substrate.
- Bridge-window compatibility surface: legacy `<custom-element>` templates remain supported via an opt-in
  `lang="custom-element-v0"` annotation while authors migrate.
- WASM-backed template processing path selected from
  [`docs/cem-element-wasm-proposal.md`](docs/cem-element-wasm-proposal.md), covering inline declaration templates,
  URI/module-map resolution, remote source streaming, local parser streaming, reusable host runtime support,
  patch-frame streams, worker-pool scheduling, service-worker-compatible artifact identity/hooks,
  post-Phase-3 edge/SSR boundaries, and main-thread DOM patch ownership.
- Phase 1 `<http-request>` resource primitive from
  [`docs/cem-elements-http-request-design.md`](docs/cem-elements-http-request-design.md): completed-response JSON/XML
  resource slices with scoped URL/module-map resolution, host-controlled resource loading, serializable
  request/response metadata, CEM-QL-navigable data AST/projections, stale-response abort protection, and fixture-backed
  `cem:for-each` demo parity. Progressive streaming, full data source-map UI, cache identity, and SSR/preload support
  remain later phases of that design.

Exit criteria (browser substrate production-ready trigger, not `@epa-wg/custom-element` adoption):

- Functional parity with `<custom-element>` proven by fixtures under
  `packages/cem-elements/tests/parity/legacy/`.
- Data-island isolation proven in browser fixtures: raw declaration/instance data inside `<template>` does not affect
  layout, selectors, form data, accessibility, or visible UI directly.
- Material parity with every component in `~/aWork/custom-element-dist/src/material/` (action, autocomplete, badge,
  dropdown, icon, icon-link, input, menu) proven by fixtures under `packages/cem-elements/tests/parity/material/`,
  including local/external `src`, hidden declarations, nested elements, slot projection, scoped styles, attribute
  `select`, namespaced `xhtml:*` elements, boolean attribute helper semantics, `module-url` resource slices,
  `data`/`option` payloads, slice events, and `slice-value`.
- Phase 2 verification suite (`nx run cem_ml_cli:validate-fixtures`, `cem_ml_cli:e2e`, `cem_ml:bench`) is green on
  every parity fixture.
- Accessibility contract in [`packages/cem-components/docs/accessibility.md`](packages/cem-components/docs/accessibility.md)
  passes end-to-end on the material parity set.

When the substrate hits this production-ready trigger, it is eligible for the Edge/SSR follow-up phase. The
`@epa-wg/custom-element` monorepo migration and next-major implementation adoption happen only after that follow-up
phase.

### 3.2 Primitives — `@epa-wg/cem-components`

Deliverables:

- Base CEM custom-element conventions: naming, attributes, events, form participation, validation, loading states, and
  progressive enhancement. Landed in
  [`packages/cem-components/docs/conventions.md`](packages/cem-components/docs/conventions.md).
- Light-DOM rendering rules and compatibility with the `cem-element` substrate (no shadow DOM). Landed in
  [`packages/cem-components/docs/light-dom-rendering.md`](packages/cem-components/docs/light-dom-rendering.md).
- Accessibility contract for labels, descriptions, focus, keyboard behavior, roles, and live regions. Landed in
  [`packages/cem-components/docs/accessibility.md`](packages/cem-components/docs/accessibility.md).
- Test harness for DOM rendering, events, accessibility assertions, and visual snapshots.
- Minimal primitives: action, field, surface, text, icon, stack, grid, list, nav, dialog shell.

Exit criteria:

- Primitives are authored exclusively with `<cem-element>`; no primitive depends on the legacy `<custom-element>`
  surface.
- Components can be used declaratively with no app JavaScript for common static and form flows.
- The runtime can consume validated light-DOM output from the parser/document transform layer.

## Phase 3.5 - Edge/SSR Processing Follow-Up

Goal: prove server and edge processing against the same serializable boundary after the browser worker substrate is
stable, without changing `<cem-element>` semantics.

Deliverables:

- SSR host fixture that emits initial HTML plus hydration metadata from a serialized `DataIslandSnapshot` and validates
  hydration against template artifact identity, `RenderRevision`, source-map mode, and retained render-plan identity.
- Edge processing fixture that accepts a serialized snapshot plus previous render-plan identity and produces a
  patch-frame stream without access to live browser DOM.
- Privacy/export policy fixtures proving that denied data-island fields are omitted or redacted before leaving the
  browser context.
- First render-state storage decision for edge processing: content-addressed cache only, revisioned KV/document
  records, or both.

Exit criteria:

- Edge/SSR fixtures prove the processing boundary outside the browser.
- No server or edge host can mutate live browser DOM, observe focus/selection/composition state, or bypass the
  data-export policy.
- Browser worker and main-thread fallback behavior remain the reference runtime semantics.

## Phase 3.6 - `@epa-wg/custom-element` Monorepo Adoption

Goal: move the published `@epa-wg/custom-element` package into this repository and rebuild its next-major
implementation on the parity-proven `cem-element` substrate after the Edge/SSR follow-up phase.

Deliverables:

- Migrate `@epa-wg/custom-element` from `~/aWork/custom-element/` into `packages/custom-element/`, preserving
  published npm identity and history.
- Keep `<custom-element>` as the public tag shipped by `@epa-wg/custom-element`.
- Make the next major of `@epa-wg/custom-element` inherit the `cem-element` substrate instead of maintaining a
  separate parser/render engine.
- Keep or retire `<template lang="custom-element-v0">` bridge support based on fixture evidence from the migration.

Exit criteria:

- Legacy parity, material parity, Edge/SSR follow-up fixtures, and custom-element package fixtures are green.
- `@epa-wg/cem-elements` is no longer the staging migration target once `@epa-wg/custom-element` adopts the substrate.

## Phase 4 - CEM Component Set

Goal: cover the practical Material-style UI surface in CEM terms.

Deliverables:

- Custom-element XSLT parity implemented before component expansion: define a separate legacy XSLT 1.0 + limited
  sample-used EXSLT compatibility adapter for copied component/sample templating, including bounded
  `xsl:template`, `xsl:apply-templates`, and `xsl:call-template` behavior.
- MVP component list and state matrix defined in [`docs/component-mvp.md`](docs/component-mvp.md).
- Maintain a versioned parity matrix against the
  [official Angular Material component catalog](https://material.angular.dev/components/categories). Map every
  user-facing Angular Material component to its CEM component, implementation/test status, states, keyboard and
  accessibility behavior, and any CEM-semantic extension. Angular-specific framework infrastructure may map to a CEM
  behavior rather than a public element, but it must not disappear from the parity audit.
- Actions and indicators: action/button, icon button, button toggle, menu item, badge, chips, icon, progress bar,
  progress spinner, and ripple/interaction feedback behavior.
- Inputs: autocomplete, form field, text input/textarea, select, checkbox, radio, switch/slide toggle, slider, and
  datepicker.
- Navigation: app bar/toolbar, menu, sidenav, nav, tabs, and stepper.
- Content and layout: card, divider, expansion panel, grid list, list, tree, table, paginator, sort header, avatar, and
  media preview.
- Feedback and overlays: bottom sheet, dialog, snackbar/toast, tooltip, skeleton, and alert.
- Author every parity component on the light-DOM `<cem-element>` substrate and CEM semantic/theme/accessibility
  contracts. Angular Material is the coverage and behavior benchmark, not a runtime dependency or DOM/API cloning
  requirement.
- Enforce a Studio dependency gate: if a proposed Studio control or interaction has an Angular Material counterpart,
  its general `@epa-wg/cem-components` parity implementation and tests land first. Studio-first implementation is
  reserved for capabilities absent from the Angular Material catalog.
- App workflows: auth forms, profile editor, asset browser, discussion thread, settings page.
- Component docs with examples, semantic guidance, token usage, states, and accessibility notes.

Exit criteria:

- The Figma site demo and CEM site can be built from the component set without one-off UI controls.
- The pinned Angular Material catalog has a complete, tested parity matrix, and every user-facing catalog component
  has a CEM-semantic counterpart implemented through `<cem-element>`.
- No Studio component duplicates an Angular Material capability that lacks the corresponding general CEM component;
  parity coverage is represented as CEM semantic components rather than direct Material clones.

## Phase 5 - Figma UI Kit

Goal: give designers a governed, usable design kit tied to generated tokens and component semantics.
Starts after the Phase 4 CEM Component Set has stable names, variants, states, and accessibility semantics.

Deliverables:

- Figma variables sourced from generated CEM token files through the documented pull-only workflow.
- Component variants matching the CEM component set: states, density, mode, intent, size, and validation.
- Usage pages for forms, navigation, data views, profile, assets, and discussion threads.
- Handoff annotations mapping Figma components to CEM elements and attributes.
- Governance rules for token updates, kit releases, and no write-back to source markdown.

Token-validation gates (moved from Phase 1):

- Validate native Figma library variables against the generated `figma/cem-*.tokens.json` files for every mode. The
  gate ships with the UI Kit because the validation is meaningful only against a populated kit.
- Extend the Phase 1 token-change smoke test to cover the Figma propagation path end to end (CSS / JSON / Swift /
  Android already gated in Phase 1).

Exit criteria:

- Designers can mock the major CEM demo flows without inventing colors, spacing, or unsupported component states.
- Figma names and component variants map cleanly to code names.

## Phase 6 - CEM Site

Goal: publish the system as a navigable product, not just packages.

Deliverables:

- Root-wired docs site with guides, token browser, component gallery, examples, API/reference, and release notes.
- Generated docs imported from package markdown and token reports.
- Interactive examples for tokens, components, XML fixtures, and native output snippets.
- Optional service-worker template/artifact registry for site/docs/playground caching, built from the Phase 3 artifact
  identity and registry-hook contract after component parity.
- Search and stable deep links.
- Optional Angular Material comparison page showing coverage and migration mapping.

Implementation note:

- Prefer a CEM/custom-element implementation first because the site should prove the library.
- Angular Material can be a comparison/reference or a later adapter demo, not the default dependency for the CEM site.

Exit criteria:

- The site can explain, demonstrate, and validate every public package/module from the repo root.

## Phase 6.5 - CEM Studio PWA And Browser Workbench

Goal: provide an installable, local-first browser application that exposes the synchronized CEM-ML CLI command model
through editable projects, structured workbenches, and safe previews. The detailed product and persistence contract is
[`docs/cem-studio.md`](docs/cem-studio.md).

Deliverables:

- Create a separate publishable `@epa-wg/cem-studio` Nx project that depends on the exact same-version
  `@epa-wg/cem-ml-cli` package and tested-compatible `@epa-wg/cem-components` and `@epa-wg/cem-theme` packages.
- Build an installable, offline-capable PWA shell with a dedicated CEM-ML worker, versioned app/runtime/sample caches,
  explicit update coordination, responsive layout, and Consumer Semantic Theme modes.
- Define a portable project/subproject hierarchy for data sets, inline and URL resources, validation/configuration,
  conversions, queries, transformations, and transformation graphs. Persist mutable projects in IndexedDB and provide
  validated import/export before remote providers.
- Add an opt-in local-file provider through the File System Access API. Let users open individual files or bind a
  portable project directory, edit supported resources, create files, and save changes back in place. Persist selected
  file/directory handles only as provider bindings, reconnect them through explicit permission requests, and retain
  `studio://` logical identities plus revision/hash conflict checks instead of embedding absolute OS paths. Keep the
  IndexedDB working store and validated upload/download import/export as the functional fallback when picker or write
  access is unavailable, unsupported, or denied.
- Seed an editable Feature Tour generated from actual schema-package examples and capability manifests so the initial
  hierarchy demonstrates supported content types through transformation graphs without drifting from the engine.
- Provide validation diagnostics with source ranges, data/result/report/source-map previews, conversion input/output
  views, query variables and scopes, transformation traces, graph-stage inspection, and safe sandboxed HTML previews.
- Provide a bidirectional CLI Command view that displays and copies the active command, effective inputs/config, and
  output; edited commands can transactionally update the current page, target another page, or create a named page.
- Put reusable workbench controls and composites in `@epa-wg/cem-components` or its `/studio` export; keep routing,
  persistence, provider credentials, worker lifecycle, and service-worker orchestration in `@epa-wg/cem-studio`.
- Classify every proposed Studio control against the pinned Angular Material parity matrix. If a counterpart exists,
  finish the general `<cem-element>`-based `@epa-wg/cem-components` implementation and parity tests before building
  the Studio composition. A `/studio` or application-local component may lead only when the matrix records no Angular
  Material counterpart; reusable behavior discovered there still moves into the component package.
- Keep account-backed S3, NoSQL, Git repository, and GitHub Gist storage behind one revisioned provider contract as a
  post-local-MVP wishlist.

Exit criteria:

- A user can install or open Studio, work offline, edit or fetch input, validate/convert/query/transform it, inspect
  diagnostics and source maps, and recover the project after reload without installing a native CLI.
- In a supporting browser, a user can open a local file or project directory, edit it, and explicitly save changes back
  through a retained File System Access handle. Permission loss, external changes, and unavailable API support produce
  recoverable diagnostics or the IndexedDB/import-export fallback rather than data loss or a broken project.
- Structured forms and the CLI Command view round-trip through the same normalized run plan without losing explicit
  content/schema/query identities, config, variables, inputs, outputs, or destination page.
- Studio resolves exactly one same-version `@epa-wg/cem-ml-cli`/`@epa-wg/cem-ml` engine chain and identifies its
  `wasm-browser` capabilities; it does not silently discover or execute an OS-native binary.
- The Feature Tour is generated and executed in verification, output previews remain bounded and non-scriptable by
  default, and all reusable UI passes theme, keyboard, accessibility, and light-DOM checks.
- A Studio dependency audit proves that every Angular-Material-equivalent control comes from its completed general CEM
  parity component; only controls with no catalog counterpart may originate in `/studio` or the application.

## Phase 7 - Figma Site Demo

Goal: prove CEM on a realistic product surface that designers, developers, and native consumers can all inspect.

Scope:

- Login.
- User registration.
- Profile view/edit.
- Assets listing with table, grid, card, and compact/list views.
- Asset detail.
- Discussion with message threading, replies, unread state, attachments, and moderation/status indicators.

Deliverables:

- Figma prototype built from the Figma UI Kit.
- Matching CEM XML/HTML fixtures.
- Matching web implementation using CEM components.
- Native token usage notes for iOS/Android implementations.
- Scenario tests and screenshots used as parity references.

Exit criteria:

- The same flows exist in Figma, CEM fixture form, and web-rendered form with consistent tokens and component semantics.

## Phase 8 - Native Platform Packages

Goal: move beyond generated token files into credible platform integration.

Deliverables:

- iOS package layout for generated Swift tokens and sample SwiftUI usage.
- Android package layout for XML resources, Kotlin constants, and sample Compose usage.
- Native component guidance for the CEM component set: names, state mapping, color/typography mapping, and accessibility.
- CI/toolchain validation for Swift and Kotlin outputs.
- Native visual parity checks against web/Figma references where practical.

Native validation gates (moved from Phase 1):

- Generated Swift compiles with the supported Xcode/Swift toolchain.
- Generated Kotlin/Compose compiles with the supported Gradle/Kotlin toolchain.
- Generated iOS and Android reports show zero fail-hard violations before release.
- Full token-change smoke test through CSS, JSON, Swift, and Android outputs. (The Figma propagation leg lives in
  Phase 5.)

Exit criteria:

- Native consumers can install or copy generated artifacts and pass compile checks without reading generator internals.

## Phase 9 - Release, Governance, And Compatibility

Goal: make CEM maintainable as a public design-system product.

Deliverables:

- Versioning policy for token names, component APIs, XML schema, native outputs, Figma kit releases, and the fixed
  CEM-ML runtime/CLI/Studio deployment family.
- Migration guides and deprecation reports.
- CI gates for build, lint, token reports, component tests, docs links, examples, and native compilation.
- Package export maps and published artifacts for stable public contracts.
- `cem-ml` CLI public distribution: separate `@epa-wg/cem-ml` WASM runtime and `@epa-wg/cem-ml-cli` Node/WASM CLI npm
  packages, plus separate Linux AMD64, Homebrew ARM64, and Windows AMD64 native deployment projects. Preserve native
  binaries as non-replaced assets on the matching tagged GitHub Release with checksums, signatures, SBOMs,
  provenance, install docs, and smoke tests for each install path; publish the release only after its complete asset
  set is staged and verified.
- `@epa-wg/cem-studio` npm/PWA publication from the same fixed CEM-ML version and release commit, including static
  deployment assets, capability/build metadata, service-worker update checks, and clean-consumer verification.
- Contribution guidelines for token specs, components, docs, and design kit updates.

Exit criteria:

- A release can be cut with confidence that token, web, native, Figma, docs, and demo contracts are coherent.
- Users can install `cem-ml` as WASM for Node or from Linux AMD64, Homebrew ARM64, and Windows AMD64 native packages and
  run the same portable CLI smoke test on each platform.
- Every native binary remains recoverable from the version's GitHub Release, and all CEM-ML npm, native, and Studio
  artifacts report the exact version originating from common `cem_ml`.

## Suggested Milestone Sequence

| Milestone | Focus                                                     | Why now                                                                                                                                                           |
| --------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| M1        | Root docs spine and token/native validation gates         | Current work is valuable but not yet easy to discover or verify end to end.                                                                                       |
| M2        | Schema-defined parser runtime and fixture pipeline        | It gives components, docs, and demos a shared semantic input model with source maps, validation, embedded-language handoffs, and an AST boundary.                 |
| M2.5      | Synchronized CEM-ML CLI deployment foundation             | Studio, IDE, and CI clients need stable command/report contracts plus distinct WASM npm, Node CLI npm, and per-platform native packages before depending on them. |
| M3a       | `<cem-element>` browser substrate                         | The declarative substrate must reach legacy + material parity before primitives commit to it. See [`docs/cem-element-design.md`](docs/cem-element-design.md).     |
| M3b       | Edge/SSR processing follow-up                             | Server/edge processing should prove the serializable boundary after the browser substrate is stable, not during Phase 3.                                          |
| M3c       | `@epa-wg/custom-element` monorepo adoption                | The published package adopts the substrate only after browser parity and the Edge/SSR follow-up are green.                                                        |
| M3d       | Custom-element runtime primitives                         | Components need stable behavior conventions before broad catalog work; they consume the parity-proven substrate from M3a.                                         |
| M4        | Angular Material parity through CEM components            | Completes the reusable `<cem-element>`-based control baseline before Studio or demos create equivalent one-off UI.                                                |
| M5        | Figma UI Kit MVP                                          | Designers need the same semantics once component names and states stabilize.                                                                                      |
| M6        | CEM site                                                  | Public documentation should be generated from stable package and component contracts.                                                                             |
| M6.5      | CEM Studio PWA                                            | The browser workbench composes the stable CLI/WASM contract and parity-complete components; only UI absent from Angular Material may begin as Studio-specific.    |
| M7        | Figma site demo plus matching web fixtures                | Full-flow demo proves the system across design and implementation.                                                                                                |
| M8        | Native package hardening                                  | Native artifacts become product-grade once token/component semantics are stable.                                                                                  |
| M9        | Release governance, CLI artifacts, and Studio publication | Formalize compatibility, preserve native binaries on tagged GitHub Releases, and publish the fixed-version npm/CLI/native/Studio family.                          |

## Near-Term Backlog

- Wire `roadmap.md`, `docs/todo.md`, package docs, and token export docs from the root README.
- Add a docs index under `docs/`.
- Draft the parser runtime contract: byte decoder, tokenizer, event normalizer, schema machine, AST/source-map model,
  and implementation interpreter boundary.
- Implement the accepted CLI package split, common-version synchronizer, four-entry target matrix, and GitHub Release
  asset-retention contract from
  [`docs/cem-ml-deployment-contract.md`](docs/cem-ml-deployment-contract.md) in Phase 2.5 checklist order.
- Define the first CEM XML/HTML profile and the scoped handoff rules for `style`, `script`, CDATA/text, CSF fields, and
  JSON string subdocuments.
- Create the first semantic fixture set: login, registration, profile, assets list, and message thread.
- Define the component MVP list and state matrix.
- Add the pinned Angular Material-to-CEM parity matrix and require a general `<cem-element>`-based CEM component before
  any Angular-equivalent Studio control is implemented.
- Promote the Studio project model, browser command round trip, local persistence and File System Access provider,
  themed component boundary, and PWA verification contract into task-level acceptance criteria when Phase 6.5 is
  scheduled.
- Add a Figma UI Kit plan that maps components to generated token variables.
- Add native compile validation to CI once Swift and Kotlin toolchains are available.
