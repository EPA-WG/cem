# CEM Roadmap

CEM should become a complete consumer-semantics design system: tokens, documentation, schema-defined parser/runtime
tooling, web components, native adapters, Figma assets, and demos that all prove the same model from different angles.

This roadmap is intentionally higher level than `docs/todo.md`. Use this file to decide product/module order; use
`docs/todo.md` for task-level execution.

## Product Modules

| Module | Purpose | Primary package or path |
| ------ | ------- | ----------------------- |
| CEM token/theme core | Canonical token specs, generated CSS, DTCG JSON, TypeScript metadata, and reports. | `packages/cem-theme` |
| Native platform adapters | iOS Swift and Android Kotlin/Compose outputs generated from the same token spine. | `packages/cem-theme/dist/lib/token-platforms` |
| CEM parser/runtime foundation | Schema-defined streaming parser layers: byte decoding, tokenization, normalized events, validation, AST/source maps, binary AST chunks, and implementation handoff. | `packages/cem_ml` |
| CEM structural lifecycle CLI | Validation, load into the internal CEM AST/event model, and export/convert across schema + content-type identities. Built-in adapters cover CEM-ML, HTML/XML parity, and the immediate XSLT 1.0 custom-element compatibility profile; future adapters register through the plugin/content-type model. | `packages/cem_ml_cli`, `packages/cem_ml` |
| CEM custom-element substrate | Declarative no-JS runtime centered on `<cem-element>`: scoped data islands, event-to-data wiring, and light-DOM re-render from CEM-ML/CEM-QL templates. Staged in `@epa-wg/cem-elements`; edge/SSR and `@epa-wg/custom-element` adoption are follow-up phases after the browser substrate is stable. | `packages/cem-elements`, future `packages/custom-element` |
| CEM component set | Material-style UI coverage expressed in CEM semantics: buttons, fields, lists, nav, cards, dialogs, tables, tabs, etc. | `packages/cem-components` |
| Figma UI Kit | Designer-facing components, variants, variables, usage examples, and governance workflow. | `examples/figma`, future design artifacts |
| CEM site | Public docs, token/component gallery, interactive examples, and release documentation wired from the repo root. | future `apps/cem-site` or static docs app |
| Figma site demo | A realistic product demo: login, registration, profile, asset listing views, and threaded discussion. | future `examples/figma-site-demo` |
| Repo docs spine | Root docs, package docs, generated API/token docs, examples index, and contribution/release docs. | `README.md`, `docs/`, package docs |

## Ordering Principles

1. Build the shared semantic spine before building demos.
2. Generate platform outputs from source-of-truth tokens; do not hand-author native or Figma values.
3. Keep parser layers explicit: byte source, decoder, tokenizer, normalized event stream, schema machine, AST builder,
   binary AST encoder, and implementation interpreter are separate contracts.
4. Carry source-map stacks and byte offsets through every parser, transform, generated node, and runtime handoff.
5. Treat embedded languages and mixed formats as scoped handoffs owned by the parent parser's return condition.
6. Prove components on the web before porting full UI examples into Figma/native.
7. Use demos as integration tests, not as the first source of component behavior.
8. Keep Angular Material as a reference benchmark for coverage and ergonomics, not as a required implementation
   dependency unless an Angular adapter is explicitly scoped later.

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
- Binary AST and subtree chunking design for cache, transport, retry, and parallel preprocessing; implementation can
  start with an uncompressed debug encoding.
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
- The same fixture can feed docs/examples without copying business structure into multiple formats.

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
- Actions: button, icon button, split action, menu item.
- Inputs: text field, textarea, select, checkbox, radio, switch, slider, date/time affordances.
- Navigation: app bar, side nav, tabs, breadcrumbs, pagination.
- Content: card, list, table/data grid, chip, badge, avatar, media/object preview.
- Feedback: dialog, sheet, snackbar/toast, progress, skeleton, inline alert.
- App workflows: auth forms, profile editor, asset browser, discussion thread, settings page.
- Component docs with examples, semantic guidance, token usage, states, and accessibility notes.

Exit criteria:

- The Figma site demo and CEM site can be built from the component set without one-off UI controls.
- Material UI coverage is represented as CEM semantic components rather than direct Material clones.

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

- Versioning policy for token names, component APIs, XML schema, native outputs, and Figma kit releases.
- Migration guides and deprecation reports.
- CI gates for build, lint, token reports, component tests, docs links, examples, and native compilation.
- Package export maps and published artifacts for stable public contracts.
- Contribution guidelines for token specs, components, docs, and design kit updates.

Exit criteria:

- A release can be cut with confidence that token, web, native, Figma, docs, and demo contracts are coherent.

## Suggested Milestone Sequence

| Milestone | Focus | Why now |
| --------- | ----- | ------- |
| M1 | Root docs spine and token/native validation gates | Current work is valuable but not yet easy to discover or verify end to end. |
| M2 | Schema-defined parser runtime and fixture pipeline | It gives components, docs, and demos a shared semantic input model with source maps, validation, embedded-language handoffs, and an AST boundary. |
| M3a | `<cem-element>` browser substrate | The declarative substrate must reach legacy + material parity before primitives commit to it. See [`docs/cem-element-design.md`](docs/cem-element-design.md). |
| M3b | Edge/SSR processing follow-up | Server/edge processing should prove the serializable boundary after the browser substrate is stable, not during Phase 3. |
| M3c | `@epa-wg/custom-element` monorepo adoption | The published package adopts the substrate only after browser parity and the Edge/SSR follow-up are green. |
| M3d | Custom-element runtime primitives | Components need stable behavior conventions before broad catalog work; they consume the parity-proven substrate from M3a. |
| M4 | Component set MVP | Unlocks real screens and validates token semantics in UI. |
| M5 | Figma UI Kit MVP | Designers need the same semantics once component names and states stabilize. |
| M6 | CEM site | Public documentation should be generated from stable package and component contracts. |
| M7 | Figma site demo plus matching web fixtures | Full-flow demo proves the system across design and implementation. |
| M8 | Native package hardening | Native artifacts become product-grade once token/component semantics are stable. |
| M9 | Release governance | Formalize compatibility after public contracts are proven. |

## Near-Term Backlog

- Wire `roadmap.md`, `docs/todo.md`, package docs, and token export docs from the root README.
- Add a docs index under `docs/`.
- Draft the parser runtime contract: byte decoder, tokenizer, event normalizer, schema machine, AST/source-map model,
  and implementation interpreter boundary.
- Define the first CEM XML/HTML profile and the scoped handoff rules for `style`, `script`, CDATA/text, CSF fields, and
  JSON string subdocuments.
- Create the first semantic fixture set: login, registration, profile, assets list, and message thread.
- Define the component MVP list and state matrix.
- Add a Figma UI Kit plan that maps components to generated token variables.
- Add native compile validation to CI once Swift and Kotlin toolchains are available.
