# Todo

This file is the authoritative checklist for remaining execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved in
[`archive/todo-completed.md`](archive/todo-completed.md).

## Immediate Goal

The immediate work is to add CSS selectors as a schema-owned query language
peer to CEM-QL and XPath, then expose all three through one `cem_ml_cli` query
surface over lifecycle-owned data. Keep stylesheet CSS identity separate from
selector-expression identity, and keep unsupported data/query combinations
explicit rather than inferring or serializing a replacement tree.

The cross-layer architecture remains serializer-free: lifecycle loading, graph
routing, joins, evaluators, CEM-QL, CEMT, and XSLT adapters must exchange
borrowed native AST streams or typed evaluator values directly. JSON and other
encodings are allowed only at explicit lifecycle parse or registered export
boundaries; no serializer, generic DTO, shape inference, or replacement tree may
mediate between internal layers.

### Immediate: CSS Selector Query Schema and Unified CLI Query Execution

- [ ] Add a validated CSS selector query package and let `cem_ml_cli` execute
      CSS selector, CEM-QL, and XPath queries through one native-data boundary.
  - [x] Define the language identities and shared execution contract before
        adding fixtures.
    - [x] Keep the existing `schema-packages/css/v1` identity scoped to
          stylesheet data (`text/css` and `https://cem.dev/ns/data/css/1`), and
          create a distinct `schema-packages/css-selector/v1` query identity so
          selector expressions are never inferred from stylesheet text.
    - [x] Define the selector baseline, namespace behavior, matching order,
          duplicate handling, pseudo-class support, source maps, work budgets,
          and stable diagnostics for unsupported selector features.
    - [x] Define a shared typed query request/result contract for CSS selectors,
          CEM-QL, and XPath. The request must retain query-language identity,
          query AST owner, input AST owner, context/bindings, resolver and safety
          policy, and budgets; results must retain native item/node identity,
          order, source maps, and language-specific type information.
    - [x] Publish a data-compatibility matrix: CEM-QL consumes its native item
          views, XPath consumes lifecycle-owned XDM-compatible trees, and CSS
          selectors consume lifecycle-owned element-tree views. Unsupported
          input families must fail with typed diagnostics instead of converting
          through JSON, browser DOM, generic DTOs, or inferred replacement trees.
  - [x] Create `packages/cem_ml/schema-packages/css-selector/v1` as a normal
        schema-package Nx subproject named
        `cem_ml_schema_package_css_selector_v1`.
    - [x] Add `package.cem`, `README.md`, `schema/css-selector.cem`, manifest-owned
          `examples/`, a package-owned conformance matrix under `tests/`, CEMT
          `formatters/` and `colorizers/`, `scripts/verify-previews.mjs`, and
          `project.json` with `build`, `samples2readme`, and `verify` targets
          matching sibling package structure and cache inputs/outputs.
    - [x] Adopt the [schema-package shared principles and review
          protocol](../packages/cem_ml/schema-packages/README.md) together with
          the CSS package README pattern: keep domain semantics declarative in
          `.cem`; keep the built-in vocabulary primitive; make CEMT own declared
          transform/format/color stages; declare owned schema/content identities
          and status; document the lossless lifecycle AST; keep validation and
          previews passive; make schema CEM own diagnostic policy; document
          resolver/execution safety separately; expose deterministic
          compact/pretty/tabular and terminal/HTML/Markdown profiles; use fenced
          selector source when supported; and generate example sections from
          `package.cem` metadata.
    - [x] Register the package in the built-in schema catalog, schema-package
          structure audit, CLI schema-package dependencies, converter parity,
          and release inputs without weakening the existing CSS stylesheet
          package.
  - [x] Implement CSS selector parsing, validation, and evaluation tests-first.
    - [x] Add focused failing Rust tests for lossless selector tokens and ranges,
          typed selector AST shape, namespaces, combinators, attribute matching,
          selector-list ordering, invalid syntax, unsupported features, and
          evaluator work-budget failures before adding manifest examples.
    - [x] Parse selector source once into a CEM-owned typed AST plus lossless
          token stream; pin the normative selector baseline and maintain a
          schema-owned conformance/gap matrix instead of silently accepting a
          host library's unsupported or extended grammar.
    - [x] Emit neutral lexical, parse, namespace, host-association, capability,
          and evaluation facts from Rust, with `schema/css-selector.cem` owning
          diagnostic codes, severity, behavior, policy, and validation modes.
    - [x] Evaluate selectors directly over borrowed lifecycle-owned element-tree
          views with deterministic document order and native identity. Do not
          require Chromium, construct browser DOM, reparse source, or project
          the tree through JSON.
  - [x] Add one `cem-ml query` CLI surface for all three query languages.
    - [x] Lock the recommended CLI contract with red integration tests before
          implementation: `query DATA`, exactly one of `--query` or
          `--query-file`, explicit `--query-content-type` with optional
          `--query-schema`, and `--output terminal|cem|json`; cover CSS
          selector, CEM-QL, and XPath execution without checked-in fixtures.
    - [x] Accept a data input plus an explicit CSS selector, CEM-QL, or XPath
          query source by schema/content identity; support inline and file-backed
          query source, and never detect a language from query text shape.
    - [x] Route each language to its registered native evaluator adapter while
          sharing input lifecycle loading, host bindings, resolver/safety
          capabilities, cancellation, budgets, reporting, and exit semantics.
    - [x] Add explicit registered result exporters so terminal, CEM, and JSON
          output encode only at the requested export boundary; do not introduce
          a common serialized query-result data plane.
    - [x] Preserve existing CEM-QL and XPath command behavior through documented
          compatibility aliases or migration diagnostics, with the unified
          query command as the canonical interface.
  - [x] Add package and CLI evidence after the native red tests are in place.
    - [x] Add manifest-owned pass/fail CSS selector examples, validation reports,
          source-map cases, namespace cases, safety-limit cases, and README
          source/preview verification.
    - [x] Add a tree-backed data fixture queried three ways—with an equivalent
          CSS selector, CEM-QL expression, and XPath expression—and assert the
          same native node identities, document order, source maps, CLI report
          shape, and explicit exported output.
    - [x] Add negative CLI fixtures for language/content-type mismatch,
          unsupported data models, invalid queries, missing context, denied
          resolver capabilities, budget exhaustion, and unavailable exporters.
  - [ ] Close the slice through Nx: run the new schema package `verify` target,
        CSS stylesheet regression verification, CEM-QL and XPath package gates,
        `cem_ml_cli` tests/e2e/converter parity, `cem_ml` lint/tests/WASM, and the
        CEM core release dry run before marking this parent item complete.

### Immediate Next: SCSS Source to CSS AST Stream

- [ ] Add SCSS as a distinct schema-owned source that transforms into the
      lifecycle-owned CSS AST stream.
  - [ ] Before adding fixtures, choose and document the SCSS schema/content
        identity, source extension, supported language/dialect version, module
        system baseline, and compatibility policy without claiming `text/css`.
  - [ ] Create `schema-packages/scss/v1` with the normal schema-package
        subproject structure, declarative diagnostics, manifest-owned examples,
        conformance/gap matrix, CEMT profiles, preview verification, catalog
        registration, CLI dependencies, and release inputs.
  - [ ] Implement SCSS parsing and transformation tests-first so native SCSS
        syntax and expansion facts lower directly into a lifecycle-owned typed
        CSS AST stream with exact origin chains. Do not emit CSS text and reparse
        it, construct a JSON/generic DTO bridge, or lose SCSS-to-CSS source maps.
  - [ ] Route imports, modules, functions, mixins, interpolation, generated
        selectors, and expansion limits through explicit resolver, safety,
        cancellation, recursion, output-size, and work-budget policies.
  - [ ] Reuse the registered CSS validation, formatter, colorizer, conversion,
        and export stages after the typed AST handoff, and verify the source path
        through package-local, `cem_ml`, `cem_ml_cli`, WASM, parity, and release
        Nx gates.

### Deferred: Phase 3.6 Adoption Decision

- [ ] Select the next Phase 3 work track before implementation.
  - Recommended: begin Phase 3.6 by defining the migration and fixture gate for
    moving `@epa-wg/custom-element` into `packages/custom-element/`, preserving
    its npm identity and `<custom-element>` public tag while rebuilding its next
    major on the parity-proven `cem-element` substrate.
  - Alternative: reopen one or more known Phase 3.1 bridge deferrals—broad
    legacy XPath/XSLT behavior, scoped light-DOM style containment, or host
    resolution policy—only when consumer or adoption evidence requires it.
  - Decision boundary: package-history migration, compatibility/versioning, and
    bridge-retention scope must be chosen before fixtures or implementation are
    added.

### Deferred: Phase 4 CEM Component Set

- [ ] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
- [ ] Populate the first missing state fixture or assertion from that audit,
      prioritizing selected, expanded, empty, and loading coverage across
      navigation, content, and layout workflows.
- [ ] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.

## Current Verification Commands

- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`

Browser-backed targets must be run with the required host permission on their
first attempt. Chromium sandbox-host aborts under the workspace restriction are
environment failures, not product/test failures.

## Externally Gated

These are intentionally not active in the current workspace because the
required native toolchains are unavailable. Keep the existing offline platform
artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for
  `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for
  `packages/cem-theme/dist/lib/token-platforms/android/`.
