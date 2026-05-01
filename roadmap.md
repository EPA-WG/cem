# CEM Roadmap

CEM should become a complete consumer-semantics design system: tokens, documentation, web components, native adapters,
Figma assets, demos, and XML/HTML tooling that all prove the same model from different angles.

This roadmap is intentionally higher level than `docs/todo.md`. Use this file to decide product/module order; use
`docs/todo.md` for task-level execution.

## Product Modules

| Module | Purpose | Primary package or path |
| ------ | ------- | ----------------------- |
| CEM token/theme core | Canonical token specs, generated CSS, DTCG JSON, TypeScript metadata, and reports. | `packages/cem-theme` |
| Native platform adapters | iOS Swift and Android Kotlin/Compose outputs generated from the same token spine. | `packages/cem-theme/dist/lib/token-platforms` |
| CEM custom-element runtime | Declarative no-JS web component primitives built on `@epa-wg/custom-element`. | `packages/cem-components` |
| CEM component set | Material-style UI coverage expressed in CEM semantics: buttons, fields, lists, nav, cards, dialogs, tables, tabs, etc. | `packages/cem-components` |
| CEM XML/HTML/XSLT library | Schema, parser, DOM helpers, transforms, and validation for declarative CEM documents. | future `packages/cem-dom` or `packages/cem-xml` |
| Figma UI Kit | Designer-facing components, variants, variables, usage examples, and governance workflow. | `examples/figma`, future design artifacts |
| CEM site | Public docs, token/component gallery, interactive examples, and release documentation wired from the repo root. | future `apps/cem-site` or static docs app |
| Figma site demo | A realistic product demo: login, registration, profile, asset listing views, and threaded discussion. | future `examples/figma-site-demo` |
| Repo docs spine | Root docs, package docs, generated API/token docs, examples index, and contribution/release docs. | `README.md`, `docs/`, package docs |

## Ordering Principles

1. Build the shared semantic spine before building demos.
2. Generate platform outputs from source-of-truth tokens; do not hand-author native or Figma values.
3. Prove components on the web before porting full UI examples into Figma/native.
4. Use demos as integration tests, not as the first source of component behavior.
5. Keep Angular Material as a reference benchmark for coverage and ergonomics, not as a required implementation
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

Remaining gates:

- Validate native Figma library variables in the CEM UI Kit.
- Compile generated Swift with a supported Xcode toolchain.
- Compile generated Kotlin/Compose with a supported Gradle toolchain.
- Run a full token-change smoke test through CSS, JSON, Figma, Swift, and Android outputs.

## Phase 2 - XML/HTML/XSLT Schema, Parser, And DOM Library

Goal: define the declarative document layer that CEM components and demos can share.

Deliverables:

- CEM document schema for semantic screens, forms, navigation, lists, assets, profiles, and messages.
- Parser from CEM XML/HTML into a typed DOM model.
- DOM helper APIs for querying semantic roles, state, validation, and relationships.
- XSLT transforms from semantic documents into light-DOM custom-element markup.
- Validation reports for unknown elements, invalid state combinations, missing labels, broken references, and unsafe
  content.
- Fixtures covering login, registration, profile, asset listing, and threaded discussion documents.

Exit criteria:

- A fixture CEM document can be parsed, validated, transformed to HTML, and rendered by the component runtime.
- The same fixture can feed docs/examples without copying business structure into multiple formats.

## Phase 3 - Custom-Element Runtime

Goal: establish the reusable declarative web runtime before building the full component catalog.

Deliverables:

- Base CEM custom-element conventions: naming, attributes, events, form participation, validation, loading states, and
  progressive enhancement.
- Light-DOM rendering rules and compatibility with `@epa-wg/custom-element`.
- Accessibility contract for labels, descriptions, focus, keyboard behavior, roles, and live regions.
- Test harness for DOM rendering, events, accessibility assertions, and visual snapshots.
- Minimal primitives: action, field, surface, text, icon, stack, grid, list, nav, dialog shell.

Exit criteria:

- Components can be used declaratively with no app JavaScript for common static and form flows.
- The runtime can consume output from the XML/HTML/XSLT transform layer.

## Phase 4 - CEM Component Set

Goal: cover the practical Material-style UI surface in CEM terms.

Deliverables:

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

Deliverables:

- Figma variables sourced from generated CEM token files through the documented pull-only workflow.
- Component variants matching the CEM component set: states, density, mode, intent, size, and validation.
- Usage pages for forms, navigation, data views, profile, assets, and discussion threads.
- Handoff annotations mapping Figma components to CEM elements and attributes.
- Governance rules for token updates, kit releases, and no write-back to source markdown.

Exit criteria:

- Designers can mock the major CEM demo flows without inventing colors, spacing, or unsupported component states.
- Figma names and component variants map cleanly to code names.

## Phase 6 - CEM Site

Goal: publish the system as a navigable product, not just packages.

Deliverables:

- Root-wired docs site with guides, token browser, component gallery, examples, API/reference, and release notes.
- Generated docs imported from package markdown and token reports.
- Interactive examples for tokens, components, XML fixtures, and native output snippets.
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

Native validation gates:

- Generated Swift compiles with the supported Xcode/Swift toolchain.
- Generated Kotlin/Compose compiles with the supported Gradle/Kotlin toolchain.
- Generated iOS and Android reports show zero fail-hard violations before release.

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
| M2 | XML/HTML/XSLT schema and fixture pipeline | It gives components, docs, and demos a shared semantic input model. |
| M3 | Custom-element runtime primitives | Components need stable behavior conventions before broad catalog work. |
| M4 | Component set MVP | Unlocks real screens and validates token semantics in UI. |
| M5 | Figma UI Kit MVP | Designers need the same semantics once component names and states stabilize. |
| M6 | CEM site | Public documentation should be generated from stable package and component contracts. |
| M7 | Figma site demo plus matching web fixtures | Full-flow demo proves the system across design and implementation. |
| M8 | Native package hardening | Native artifacts become product-grade once token/component semantics are stable. |
| M9 | Release governance | Formalize compatibility after public contracts are proven. |

## Near-Term Backlog

- Wire `roadmap.md`, `docs/todo.md`, package docs, and token export docs from the root README.
- Add a docs index under `docs/`.
- Decide the package name for the XML/HTML/XSLT DOM library.
- Create the first semantic fixture set: login, registration, profile, assets list, and message thread.
- Define the component MVP list and state matrix.
- Add a Figma UI Kit plan that maps components to generated token variables.
- Add native compile validation to CI once Swift and Kotlin toolchains are available.
