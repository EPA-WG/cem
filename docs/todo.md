# Todo

This file is the authoritative checklist for remaining execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved in
[`archive/todo-completed.md`](archive/todo-completed.md).

## Immediate Goal

The CSS selector query and SCSS-to-CSS lifecycle slices are complete. The active
work is closing the Phase 4 component state matrix in priority order. The
component stylesheet publication, `action:hover`, and `action:active` contracts
are implemented and verified. The `input:hover` native-owner inventory is
complete, and the recommended theme taxonomy is accepted: field-like and binary
controls use separate semantic hover families. Work remains stopped at
`CEM-CSS-001` because CEM does not yet select the exact visual channels or
values for either family.

The cross-layer architecture remains serializer-free: lifecycle loading, graph
routing, joins, evaluators, CEM-QL, CEMT, and XSLT adapters must exchange
borrowed native AST streams or typed evaluator values directly. JSON and other
encodings are allowed only at explicit lifecycle parse or registered export
boundaries; no serializer, generic DTO, shape inference, or replacement tree may
mediate between internal layers.

### Immediate: CSS Selector Query Schema and Unified CLI Query Execution

- [x] Add a validated CSS selector query package and let `cem_ml_cli` execute
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
  - [x] Close the slice through Nx: run the new schema package `verify` target,
        CSS stylesheet regression verification, CEM-QL and XPath package gates,
        `cem_ml_cli` tests/e2e/converter parity, `cem_ml` lint/tests/WASM, and the
        CEM core release dry run before marking this parent item complete.

### Immediate Next: SCSS Source to CSS AST Stream

- [x] Add SCSS as a distinct schema-owned source that transforms into the
      lifecycle-owned CSS AST stream.
  - [x] Before adding fixtures, choose and document the SCSS schema/content
        identity, source extension, supported language/dialect version, module
        system baseline, and compatibility policy without claiming `text/css`.
  - [x] Create `schema-packages/scss/v1` with the normal schema-package
        subproject structure, declarative diagnostics, manifest-owned examples,
        conformance/gap matrix, CEMT profiles, preview verification, catalog
        registration, CLI dependencies, and release inputs.
  - [x] Implement SCSS parsing and transformation tests-first so native SCSS
        syntax and expansion facts lower directly into a lifecycle-owned typed
        CSS AST stream with exact origin chains. Do not emit CSS text and reparse
        it, construct a JSON/generic DTO bridge, or lose SCSS-to-CSS source maps.
  - [x] Isolate SCSS-only CLI verification in a dedicated Cargo integration test
        and Nx target so SCSS package gates do not select the broad CLI suite.
  - [x] Route imports, modules, functions, mixins, interpolation, generated
        selectors, and expansion limits through explicit resolver, safety,
        cancellation, recursion, output-size, and work-budget policies.
  - [x] Reuse the registered CSS validation, formatter, colorizer, conversion,
        and export stages after the typed AST handoff, and verify the source path
        through package-local, `cem_ml`, `cem_ml_cli`, WASM, parity, and release
        Nx gates.

### Completed: Phase 3.6 Adoption Decision

- [x] Select the next Phase 3 work track before implementation.
  - Selected the recommended Phase 3.6 adoption track. This reconciles already
    landed repository state: `@epa-wg/custom-element` is a workspace package in
    `packages/custom-element/`, preserves its npm identity and public
    `<custom-element>` tag, and delegates rendering to the parity-proven
    `CemElementRuntime` substrate.
  - Package-history/import boundary: use the local 0.0.37 checkout as the
    history source and the installed 0.0.39 package as the behavior baseline;
    keep generic legacy release tags out of this repository unless they are
    namespaced. See
    [`custom-element-migration-scope.md`](custom-element-migration-scope.md) and
    [`custom-element-package-baseline.md`](custom-element-package-baseline.md).
  - Compatibility/versioning boundary: preserve the package name, browser
    entrypoints, import side effects, and `<custom-element>` declaration surface
    in the 0.1.0 pre-1.0 major adoption release. See
    [`custom-element-adapter-boundary.md`](custom-element-adapter-boundary.md)
    and [`release-readiness-0.1.0.md`](release-readiness-0.1.0.md).
  - Bridge-retention boundary: keep explicit `custom-element-v0` support as a
    deprecated 0.1.0 migration bridge, with removal in the next major guarded by
    FF-5; legacy HTML+XSLT otherwise lowers through the CEM-owned compatibility
    compiler into CEM-ML/CEM-QL rather than restoring a browser XSLT engine. See
    [`custom-element-bridge-template-policy.md`](custom-element-bridge-template-policy.md)
    and [`custom-element-xslt-parity-decision.md`](custom-element-xslt-parity-decision.md).
  - Executable adoption evidence is owned by the package baseline, source/dist
    browser, material compatibility, theme-vendor, no-XSLT, and release-root
    checks aggregated by `yarn nx run @epa-wg/custom-element:verify`.

### Active: Phase 4 CEM Component Set

- [x] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
  - Added `packages/cem-components/tests/state-matrix-coverage.json`, keyed by
    category/state with affected or evidenced components, the required
    interaction or transition, and the exact browser test/assertion owner.
  - Added `@epa-wg/cem-components:verify-state-matrix`, which derives all 39
    requirements from `docs/component-mvp.md`, rejects missing or extra rows,
    unknown components, stale browser test/assertion references, and stale
    static-only fixture markers, then reports 19 browser-covered, 0 static-only,
    and 20 gap rows.
  - Kept the slice audit-only: the first priority gap is `content:selected`; no
    component runtime behavior or browser fixture changed in this item.
- [x] Populate the first missing state fixture or assertion from that audit,
      applying the resolved chip-state boundary before introducing a selectable
      list/table-row contract.
  - [x] Add opt-in `<cem-chip checkable [checked]>` browser coverage: keep the
        default chip a passive `<span>`, render a checkable chip as a native
        toggle `<button aria-pressed>`, and persist each click through the
        serializable `checked` slice. This closes `content:checked`;
        `content:selected` remains deferred to a future selectable row/option
        ownership model where selected, hover, and keyboard focus can be
        distinguished.
  - The focused `states.browser.spec.ts` target proves passive and checkable
    rendering, accessible naming, native focus, `aria-pressed`, and serializable
    boolean transitions in both directions. The audit now reports 20
    browser-covered, 0 static-only, and 19 gap rows.
- [x] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.
  - Re-ran the isolated state browser target from the clean implementation
    commit: all 5 tests passed. The state-matrix audit and package lint target
    also passed independently.
  - The aggregate package gate passed all 12 dependencies, including primitive,
    style, workflow, state-matrix, build, and 32 package tests across 5 files.
  - Confirmed the generated audit remains at 20 browser-covered, 0 static-only,
    and 19 gaps with `content:selected` still recommended next; no selectable
    row/option API was introduced by this verification-only item.
- [x] Decide and document the Phase 4 `content:selected` owner before adding a
      fixture or runtime behavior.
  - Accepted and documented
    [`packages/cem-components/docs/selectable-list-contract.md`](../packages/cem-components/docs/selectable-list-contract.md):
    preserve passive `cem-list` and static `cem-table` defaults, while
    `cem-list[selectable]` consumes direct declarative `cem-list-option` payload
    into a native single-select `<select size>` listbox.
  - `cem-list-option` is parent-scoped payload vocabulary, not a separately
    registered primitive, so the accepted 32-component manifest remains stable.
    It requires unique `value`, text-only content, and optional presence-only
    `selected` and `disabled` defaults.
  - Pinned host `value` precedence, last-authored selected fallback, native
    pointer/keyboard behavior, a serializable `value` slice, explicit
    `aria-selected` evidence, native-control focus, non-form participation, and
    the prohibition on nested option controls.
  - Kept table-row selection deferred to a complete interactive-grid contract;
    static rows MUST NOT gain `aria-selected` in isolation.
- [x] Implement the accepted `content:selected` contract tests-first.
  - Added the focused browser fixture red first, then preserved passive-list
    output while normalizing only direct `cem-list-option` payload into a named
    native single-select listbox in source order. The fixture covers parent
    `value` precedence, last-authored child `selected` fallback, `size`, disabled
    options, and omission of non-option or nested payload.
  - Implemented the accepted behavior entirely with existing CEM-ML/CEM-QL
    iteration, lexical expressions, conditional option materialization, and a
    native `change`-owned string `value` slice. `cem-list-option` remains
    unregistered parent-scoped vocabulary; no imperative keyboard handler or
    form participation was added.
  - Proved pointer and native keyboard selection, focus remaining on the select,
    exact native/ARIA selectedness, disabled-option skipping, and serializable
    event payloads. The component and accessibility docs now describe the
    shipped boundary.
  - Updated `content:selected` to browser-covered. The audit now has 21 covered,
    0 static-only, and 18 gap rows, with `navigation:expanded` recommended next.
- [x] Verify the `content:selected` slice with focused targets, package lint,
      the state-matrix audit, and the aggregate `@epa-wg/cem-components:verify`
      gate; then confirm the audit's next recommended gap.
  - Re-ran the uncached isolated state browser target from the clean
    implementation commit: all 6 tests passed. The uncached 32-primitive audit,
    state-matrix audit, and package lint target also passed independently.
  - The uncached aggregate package gate passed all 12 dependencies, including
    deterministic theme/token regeneration, primitive, style, workflow,
    state-matrix, build, and 33 package tests across 5 files.
  - Confirmed regeneration left the worktree unchanged and the audit remains at
    21 browser-covered, 0 static-only, and 18 gaps with
    `navigation:expanded` recommended next; no expanded-navigation behavior was
    introduced by this verification-only item.
- [x] Decide and document the Phase 4 `navigation:expanded` owner before adding
      a fixture or runtime behavior.
  - Accepted and documented
    [`packages/cem-components/docs/navigation-disclosure-contract.md`](../packages/cem-components/docs/navigation-disclosure-contract.md):
    preserve passive `cem-nav`, while `cem-nav[collapsible]` makes the whole
    labeled landmark one disclosure controlled by a native button and stable
    hidden content container.
  - Pinned `label`, presence-only `collapsible` and initial `expanded`, exact
    button-owned `aria-expanded`, native pointer/Enter/Space activation, focus
    retention, normal link tab order, a serializable boolean slice, optional
    `aria-controls` omission, progressive fallback, and non-form participation.
  - Deferred parent-scoped `cem-nav-group`, nested submenu/tree behavior, and
    menu/menubar roles because they require a separate recursive vocabulary and
    composite focus contract. Rejected `<details>/<summary>` for this slice
    because its native expanded state is implicit and current ARIA-in-HTML rules
    do not allow the explicit `aria-expanded` evidence required here.
  - Kept `cem-app-bar` and `cem-tabs` outside the state owner: banners do not own
    descendant action disclosures, and tabs own selection/panel visibility rather
    than expanded navigation groups.
- [x] Implement the accepted `navigation:expanded` contract tests-first.
  - Added a focused browser fixture with passive, closed, and initially expanded
    navigation. Its required red run failed only because the disclosure button
    was absent; after implementation all 7 focused state tests passed.
  - Added the accepted native button and stable hidden content branches using
    existing CEM-ML/CEM-QL boolean slices only. Passive `cem-nav` output remains
    unchanged, and no runtime-substrate change or imperative handler was needed.
  - Covered pointer, Enter, Space, focus retention, open/closed link tab order,
    exact ARIA/visibility agreement, stable rendered nodes, form neutrality, and
    the serialized boolean `expanded` payload.
  - Updated the component, accessibility, conventions, and accepted-contract
    docs. Promoted only `navigation:expanded` in the audit, yielding 22
    browser-covered rows, 0 static-only rows, and 17 gaps; `layout:empty` is next.
- [x] Verify the `navigation:expanded` slice with focused targets, package lint,
      the state-matrix audit, and the aggregate `@epa-wg/cem-components:verify`
      gate; then confirm the audit's next recommended gap.
  - The uncached focused state suite passed all 7 tests. Package lint, the
    32-primitive manifest audit, and the state-matrix audit also passed
    independently.
  - The uncached aggregate gate passed all 12 dependencies, including
    deterministic theme/token regeneration, style and workflow audits, and all
    34 package tests across 5 files.
  - Confirmed regeneration left tracked source unchanged and created no failed
    browser artifacts. The audit remains at 22 browser-covered, 0 static-only,
    and 17 gap rows, with `layout:empty` recommended next.
- [x] Decide and document the Phase 4 `layout:empty` owner before adding a
      fixture or runtime behavior.
  - Accepted and documented
    [`packages/cem-components/docs/layout-empty-contract.md`](../packages/cem-components/docs/layout-empty-contract.md):
    `cem-surface[empty]` explicitly marks a settled empty workflow region, while
    `cem-stack` and `cem-grid` remain formatting-only containers whose empty
    output has no inferred semantics.
  - Pinned presence-only author/data-source ownership instead of child counting,
    unchanged non-empty output, exact rendered `data-state="empty"`, and a single
    default payload containing visible contextual guidance plus a real next
    action path. No synthesized message, alternate named slot, new primitive,
    slice, state event, fetch, or routing behavior is accepted.
  - Kept the named surface as an ordinary region: no automatic live region,
    focus move, or composite role. A workflow may put `role="status"` on only a
    dedicated non-interactive message when a dynamic result meets the WCAG
    status-message definition, and it owns focus recovery if content removal
    invalidates focus.
  - Distinguished settled workflow emptiness from collection-local
    `content:empty`, pending `layout:loading`, and feedback/error semantics.
    Existing attribute observation, CEM-ML conditionals, light-DOM diffing, and
    slot projection are expected to suffice; implementation must stop if the red
    fixture disproves that boundary.
- [x] Implement the accepted `layout:empty` contract tests-first.
  - Added the focused browser fixture red first. All existing tests passed and
    the new case failed only on the absent `data-state="empty"`; after the
    primitive change all 8 focused state tests passed.
  - Added only the accepted declarative `cem-surface` branches. Ordinary output
    remains unchanged; explicit empty state adds the exact marker to the same
    stable named section while projecting the same authored payload. No child
    counting, alternate slot, slice, custom event, focus handler, or application
    behavior was added.
  - Proved pre-upgrade fallback guidance, presence-only authoring, stack/grid
    non-ownership, native next-action semantics, absence of automatic live or
    focus semantics, stable host-attribute transitions, focus retention, and no
    serialized empty state/event.
  - Updated the contract, component reference, accessibility, and boolean-state
    docs. Promoted only `layout:empty` in the audit, yielding 23 browser-covered
    rows, 0 static-only rows, and 16 gaps; `content:loading` is next.
- [x] Verify the `layout:empty` slice with focused targets, package lint, the
      state-matrix audit, and the aggregate `@epa-wg/cem-components:verify`
      gate; then confirm the audit's next recommended gap.
  - The uncached standard focused target passed all 8 state tests without
    reproducing the earlier transient Nx `env` schema abort. Package lint, the
    32-primitive manifest audit, and the state-matrix audit also passed
    independently.
  - The uncached aggregate gate passed all 12 dependencies, including
    deterministic theme/token regeneration, style and workflow audits, and all
    35 package tests across 5 files.
  - Confirmed regeneration left tracked source unchanged and created no failed
    browser artifacts. The audit remains at 23 browser-covered, 0 static-only,
    and 16 gap rows, with `content:loading` recommended next.
- [x] Decide and document the Phase 4 `content:loading` owner before adding a
      fixture or runtime behavior.
  - Accepted and documented
    [`packages/cem-components/docs/content-loading-contract.md`](../packages/cem-components/docs/content-loading-contract.md):
    `cem-card[busy]` explicitly projects the first content-loading state onto
    its stable named section as exact `data-state="loading"` and
    `aria-busy="true"`; ordinary cards remain unchanged.
  - Selected the card as the smallest shared asset, profile, discussion,
    authentication, and settings content boundary. Lists and tables retain
    collection/empty semantics, while media preview remains resource-specific;
    none infer or inherit loading in this slice.
  - Pinned presence-only workflow ownership, retained refresh payload, authored
    visible initial-loading text and layout-preserving `cem-skeleton` payload,
    optional determinate `cem-progress`, no automatic live region, stable focus,
    no inert subtree, and reduced-motion behavior.
  - Defined loading-to-content, loading-to-empty, and loading-to-error ordering.
    The card performs no fetch, timing, cancellation, payload selection, slice,
    outcome event, or error/status synthesis; those remain with the application,
    resource loader, workflow, or feedback primitive.
  - Kept deferred `layout:loading`, input/action loading, collection-specific
    placeholders, and feedback progress/status separate. Existing attribute
    observation, CEM-ML conditionals, light-DOM diffing, and slot projection are
    expected to suffice; implementation must stop if the red fixture disproves
    stable section/header/body or surviving-focus identity.
- [x] Implement the accepted `content:loading` contract tests-first.
  - Added the focused browser case first; the existing eight cases stayed green
    and the new case failed only because the card section lacked the accepted
    loading marker. After implementation, all nine focused state tests pass.
  - Added only the accepted declarative `cem-card` branches. Ordinary output is
    unchanged; presence-only `busy`, including `busy="false"`, adds exact
    `data-state="loading"` and `aria-busy="true"` to the same stable named
    section while retaining the authored header, body, and payload.
  - Proved visible initial status/skeleton fallback, retained refresh dimensions,
    stable section/header/body and surviving focus through host-attribute
    transitions, collection-owned settled empty content, and list/table/media
    preview non-ownership. No resource work, timer, slice, event, inert behavior,
    alternate slot, or synthesized status/placeholder was added.
  - Updated the contract, component reference, accessibility, and conventions
    docs. Promoted only `content:loading` in the audit, yielding 24
    browser-covered rows, 0 static-only rows, and 15 gaps; `layout:loading` is
    recommended next after this slice's verification gate.
- [x] Verify the `content:loading` slice with focused targets, package lint, the
      state-matrix audit, and the aggregate `@epa-wg/cem-components:verify`
      gate; then confirm the audit's next recommended gap.
  - The uncached focused browser target passed all 9 state tests, including the
    explicit busy-card case. Package lint, the 32-primitive manifest audit, and
    the state-matrix audit also passed independently.
  - The uncached aggregate package gate passed all 12 dependencies, including
    deterministic theme/token regeneration, style and workflow audits, and all
    36 package tests across 5 files.
  - Confirmed regeneration left the pre-existing worktree state unchanged. The
    audit remains at 24 browser-covered rows, 0 static-only rows, and 15 gaps,
    with `layout:loading` recommended next.
- [x] Decide and document the Phase 4 `layout:loading` owner before adding a
      fixture or runtime behavior.
  - Accepted and documented
    [`packages/cem-components/docs/layout-loading-contract.md`](../packages/cem-components/docs/layout-loading-contract.md):
    presence-only `cem-surface[busy]` projects whole-workflow loading onto its
    stable named section as exact `data-state="loading"` and `aria-busy="true"`.
  - Kept `cem-stack` and `cem-grid` formatting-only. They arrange authored
    loading or retained content inside a busy surface but do not infer, inherit,
    or expose loading semantics themselves.
  - Pinned retained refresh layouts, authored visible initial-loading text and
    layout-preserving skeleton/progress composition, stable dimensions, child
    placement and surviving focus, no automatic live region or inert subtree,
    and no fetch, timer, slice, event, cancellation, or outcome ownership.
  - Made `busy` deterministically precede settled `empty`: ordered transitions
    may briefly carry both host attributes, but the section exposes loading only
    until `busy` is removed, then changes directly to exact empty state without
    simultaneous markers or an ordinary-state flash.
  - Kept card-level `content:loading`, feedback, control, collection, media, and
    resource lifecycle ownership separate. Existing attribute observation,
    CEM-ML conditionals, light-DOM diffing, and slot projection are expected to
    suffice; implementation must stop if the red fixture disproves stable
    section/descendant identity, dimensions, placement, or surviving focus.
- [x] Implement the accepted `layout:loading` contract tests-first.
  - Added the focused browser case first. All 9 existing state tests passed and
    the new case failed only because the busy surface lacked its accepted
    `data-state="loading"` marker; after implementation all 10 focused tests pass.
  - Added only the accepted declarative `cem-surface` branch before the existing
    empty branch. Ordinary and empty output remain unchanged; presence-only
    `busy`, including `busy="false"`, adds exact `data-state="loading"` and
    `aria-busy="true"`, and deterministically precedes `empty` while present.
  - Proved authored progressive fallback, retained refresh layout, stable
    section/grid/child identity, dimensions, placement, and surviving focus;
    direct loading-to-empty ordering; ignored stack/grid `busy`; descendant
    non-inheritance; and absence of automatic live, inert, slice, resource,
    timer, and lifecycle-event behavior. The contract's substrate stop condition
    was not triggered.
  - Updated the implemented contract, component reference, accessibility,
    conventions, and layout-empty ordering docs. Promoted only
    `layout:loading`, yielding 25 browser-covered rows, 0 static-only rows, and
    14 gaps; appended the approved canonical successor so `action:hover` is now
    recommended next.
- [x] Verify the `layout:loading` slice with focused targets, package lint, the
      state-matrix audit, and the aggregate `@epa-wg/cem-components:verify`
      gate; then confirm `action:hover` as the audit's next recommended gap.
  - The uncached focused browser target passed all 10 state tests. Package lint,
    the 32-primitive manifest audit, and the state-matrix audit also passed
    independently.
  - The uncached aggregate package gate passed all 12 dependencies, including
    deterministic theme/token regeneration, style and workflow audits, and all
    37 package tests across 5 files.
  - Confirmed regeneration left the pre-existing worktree state unchanged and
    created no browser failure artifacts. The audit remains at 25
    browser-covered rows, 0 static-only rows, and 14 gaps, with `action:hover`
    recommended next.
- [x] Decide and document the Phase 4 `action:hover` owner and executable
      acceptance before adding a fixture or changing runtime/style behavior.
  - Accepted and documented
    [`packages/cem-components/docs/action-hover-contract.md`](../packages/cem-components/docs/action-hover-contract.md):
    each primitive's direct native `button:enabled:hover` owns the ephemeral
    pointer state; no JavaScript handler, host attribute, private state class,
    slice, ARIA state, focus change, or activation behavior is added.
  - Assigned scoped selectors and an explicit, author-imported `./styles.css`
    export to `@epa-wg/cem-components`, while `@epa-wg/cem-theme` retains the
    generated default/hover token values. The JavaScript entry remains free of
    stylesheet side effects.
  - Bound `cem-action` to the primary action token pairs and the quiet
    `cem-icon-button` plus `cem-menu-item` to contextual action pairs. Additional
    intents and arbitrary variant mappings remain separate API decisions.
  - Limited hover to paired background/text color changes, with no geometry,
    content, motion, or forced-color override. Native disabled buttons are
    excluded with `:enabled`; focus-visible treatment must survive overlapping
    hover.
  - Selected direct `:hover` without pointer/hover media-query gating because
    the media features describe device capabilities rather than the exact
    designation state. Actions remain fully operable and complete when hover
    never matches.
  - Defined a real-pointer browser acceptance owner covering token treatment,
    unhover restoration, stable rectangles/DOM/semantics/focus/runtime state,
    disabled non-treatment, event absence, style-token verification, and
    promotion of only the `action:hover` audit row.
- [x] Decide and document the `@epa-wg/cem-components/styles.css` publication
      boundary before implementing `action:hover`.
  - [x] Confirm the resolved `@epa-wg/cem-components:build` target is plain
        `tsc --build tsconfig.lib.json`: it includes only `src/**/*.ts`, declares
        only JavaScript/declaration outputs under ignored `dist`, and has no
        static-asset copy stage.
  - [x] Confirm the npm package currently publishes only `dist`, exposes only
        JavaScript/package-metadata exports, and has no established workspace
        pattern that copies a component-owned CSS source through an inferred
        TypeScript build.
  - [x] Stop before adding the red browser fixture, component CSS, package
        export, or state-matrix promotion, as required by the accepted
        [`action:hover` contract](../packages/cem-components/docs/action-hover-contract.md)
        when stylesheet publication would change the package build boundary.
  - [x] Accepted and documented
        [`packages/cem-components/docs/stylesheet-publication-contract.md`](../packages/cem-components/docs/stylesheet-publication-contract.md):
        keep the sole tracked source at `src/styles.css`, copy it byte-for-byte
        through cacheable `build:styles` into `dist/styles.css`, make the
        inferred TypeScript `build` depend on that target, and expose only
        `./styles.css: ./dist/styles.css`.
  - [x] Pinned the package-owned copy and verification script paths, exact Nx
        inputs/outputs/dependencies, clean/repeated/cache behavior, package
        `exports`/`files` boundary, source-versus-author import paths,
        CSS-side-effect prohibition, release workflow dependency, temporary
        npm-pack evidence, and target-composition stop condition.
  - Rejected a separately published package-root or `src` CSS file, JavaScript
    entry import, build replacement, checked-in `dist`, and publish-time
    generation. The existing package-root manifest continues to publish only
    verified `dist` artifacts.
- [x] Implement the accepted component stylesheet publication contract before
      adding hover behavior.
  - [x] Add the canonical minimal `src/styles.css`, package-owned deterministic
        copy script, and cacheable `build:styles` target; merge it into the
        inferred TypeScript build without changing the resolved compiler target.
  - [x] Expose only `./styles.css: ./dist/styles.css`, add the package verifier
        and aggregate gate dependency, and prove byte identity plus exact
        `npm pack --dry-run --json` inclusion/exclusion behavior.
  - [x] Verify fresh, repeated, and cached Nx builds; style/package/lint gates;
        and aggregate package verification without changing browser presentation
        or promoting a state-matrix row.
  - The resolved build retained its inferred TypeScript executor, command,
    working directory, inputs/outputs, cacheability, and sync generator. Fresh
    and repeated copies had identical source/output SHA-256; after removing only
    the generated stylesheet, Nx restored the same bytes from local cache.
  - `verify-package` packed 18 files with exactly one `dist/styles.css`, no
    source/root duplicate, and no `*.tsbuildinfo`. The uncached aggregate gate
    passed all 15 dependencies and 37 tests across five files; the audit remains
    25 covered, 0 static-only, and 14 gaps with `action:hover` recommended next.
  - Added the token-first
    [`components-css-exceptions.md`](../packages/cem-components/docs/components-css-exceptions.md)
    review queue requested for future unrepresentable component values. This
    declaration-free slice needs no exception, and the queue grants no waiver.
- [x] Implement the accepted `action:hover` contract tests-first.
  - [x] Add the focused `states.browser.spec.ts` real-pointer fixture first and
        confirm it fails only because the published component stylesheet has no
        action default/hover bindings yet.
  - [x] Publish the side-effect-free `@epa-wg/cem-components/styles.css` export
        and add the minimal enabled default/hover token bindings without runtime
        or geometry changes.
  - [x] Make the focused fixture green, update only the `action:hover` audit row
        and directly affected component/style docs, then run focused browser,
        style, state-matrix, lint, and aggregate package verification through
        Nx.
  - Added the real-pointer fixture first: all 10 existing state cases stayed
    green while the new case failed at the native button background versus the
    required primary default token. The implemented fixture passes 11/11 and
    covers all three enabled and disabled action primitives, token treatment,
    unhover restoration, focus, geometry, DOM/ARIA, runtime state, and event
    absence.
  - Added only component-scoped default and `:enabled:hover` background/text
    rules, using eight generated primary/contextual `--cem-action-*` endpoints.
    The strengthened style verifier rejects unknown/non-CEM variables, raw
    color/geometry literals, unscoped selectors, and mappings outside the
    accepted action contract. No component CSS exception is required.
  - Resolved the export-aware bundler boundary with the approved
    `@epa-wg/cem-theme/styles.css` export to generated
    `dist/lib/css/cem-combined.css`. The theme-owned package verifier passed a
    151-file npm dry run, and the component verifier retained exactly one
    `dist/styles.css` in its 18-file dry run.
  - Theme/component lint, focused browser, style, package, and state gates all
    passed. The uncached aggregate passed 16 dependencies and 38 tests across
    five files. Only `action:hover` moved to covered, yielding 26 covered,
    0 static-only, and 13 gaps with `action:active` recommended next.
- [x] Decide and document the Phase 4 `action:active` owner and executable
      acceptance before adding a fixture or changing component CSS.
  - [x] Pin enabled native-button `:active` ownership and primary/contextual
        active-token mappings for `cem-action`, `cem-icon-button`, and
        `cem-menu-item`; keep disabled buttons excluded and CEM tokens mandatory.
  - [x] Define real pointer-down/hold/release evidence that observes the
        transient active treatment without replacing `:active` with a class,
        attribute, synthetic event, or runtime slice.
  - [x] Separate presentation invariants during pointer hold from the native
        click and existing `pressed`/`selected` slice aftermath on release;
        decide the matching keyboard-activation evidence and pseudo-class
        overlap with hover/focus-visible.
  - [x] Pin paired contrast, unpressed restoration, geometry, DOM/ARIA, focus,
        disabled, forced-colors, and event expectations. Stop and promote a
        runner or token substrate issue if real held-pointer state cannot be
        observed or existing CEM active tokens cannot express the treatment.
  - Accepted
    [`packages/cem-components/docs/action-active-contract.md`](../packages/cem-components/docs/action-active-contract.md):
    use direct native `button:enabled:active` selectors; inspect computed styles
    after trusted provider `pointerdown` while delayed pointer-up is pending;
    use a retained Space keydown/keyup for keyboard parity; and keep click plus
    `pressed`/`selected` slices strictly in the release aftermath.
  - Existing generated primary/contextual active background/text pairs cover
    every declaration. Active overrides hover only while held, preserves focus
    and 4.5:1 paired contrast, restores hover/default on release/unhover, and
    leaves `forced-color-adjust: auto`; no component CSS exception is required.
- [x] Implement the accepted `action:active` contract tests-first.
  - [x] Add the focused `states.browser.spec.ts` native pointer/keyboard fixture
        first and confirm it fails at the missing active-token treatment while
        all existing state tests remain green.
  - [x] Add only the accepted `:enabled:active` background/text bindings and
        extend the exact-selector style verifier without adding a CSS exception.
  - [x] Make the focused fixture green, promote only `action:active`, update the
        directly affected component/style docs, and run focused browser, style,
        state-matrix, lint, package, and aggregate verification through Nx.
  - The tests-first run kept all 11 prior state tests green, observed the trusted
    held button matching `:active`, and failed at the hover-painted background
    versus the required generated active token. The implemented focused target
    passes all 12 state tests.
  - Added only component-scoped `:enabled:active` background/text rules after
    their hover rules. The primary/contextual bindings use four existing
    generated `--cem-action-*-active-*` endpoints, retain
    `forced-color-adjust: auto`, and need no CSS exception.
  - The fixture proves real pointer and Space holds, 4.5:1 paired contrast,
    active-over-hover priority, release/unhover restoration, geometry,
    DOM/ARIA, focus, disabled suppression, runtime timing, and native descendant
    event targeting across all three action primitives.
  - The exact style gate passed 32 primitives and 371 generated theme tokens.
    Focused browser, state, lint, package, and aggregate verification passed;
    only `action:active` moved to covered, yielding 27 covered, 0 static-only,
    and 12 gaps with `input:hover` recommended next.
- [ ] Decide and document the Phase 4 `input:hover` owners and executable
      acceptance before adding another fixture or changing component CSS.
  - [x] Inventory the actual native hover targets for `cem-field`,
        `cem-text-field`, `cem-textarea`, `cem-select`, `cem-checkbox`,
        `cem-radio`, and `cem-switch`; keep wrapper/label hover from becoming a
        substitute for the native control state.
  - [ ] Decide semantic token mappings and visual channels for text-entry,
        select, and binary controls. Search the generated CEM catalog first and
        stop with a proposed `components-css-exceptions.md` entry if no existing
        input-hover endpoints can express the treatment.
    - [x] Confirm the generated catalog exposes only action-family hover colors;
          `--cem-control-*` is geometry-only, `--cem-stroke-*` supplies generic
          width/ring geometry, and no generated input/field/form hover endpoint
          exists.
    - [x] Record proposed exception `CEM-CSS-001` without granting a waiver or
          adding component CSS. Reusing contextual action tokens or raw palette
          colors would miscategorize input semantics.
    - [x] Categorize `cem-field`, `cem-text-field`, `cem-textarea`, and
          `cem-select` as one field-like hover family, separate from the binary
          hover family shared by `cem-checkbox`, `cem-radio`, and `cem-switch`.
    - [ ] Decide the exact field-like fill/text/boundary treatment and binary
          accent/boundary/indicator treatment, including token names, formulas,
          theme-mode mappings, and forced-colors behavior. The current D0/D5
          contracts do not choose among these channels, and native binary
          controls do not expose one reliable shared painted property.
  - [ ] Pin real hover/unhover evidence, disabled and readonly boundaries,
        label/control overlap, focus-visible coexistence, restoration, geometry,
        DOM/ARIA, value/checked state, forced-colors, and event absence.
  - [ ] Keep hover presentation CSS-only and author-imported: no host state,
        synthetic pointer events, runtime slice, or JavaScript style side effect.
  - Native owners are `cem-field input`, `cem-text-field input`,
    `cem-textarea textarea`, `cem-select select`, `cem-checkbox input`,
    `cem-radio input`, and `cem-switch input`. No fixture, CSS selector, token,
    verifier binding, or `input:hover` audit promotion was added after the token
    stop condition fired. The accepted family split is taxonomy only; it does
    not authorize token names, values, component properties, or an exception.

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
