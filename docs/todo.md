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
are implemented and verified. The `input:hover` contract is also implemented:
all seven native owners share a theme-tokenized three-stripe indicator, with
underline and outline as geometry variants, not separate semantics.
`CEM-CSS-001` is closed through D0/D5 theme adoption without a component CSS
exception. Dedicated keyboard and forced-colors coverage completes
`input:focus-visible`, and explicit presence-only busy projection now completes
`input:loading` across all seven owners. The custom form-associated `cem-select`
now completes `input:expanded` with CEM-QL-owned rich option projection,
canonical `cem-option` authoring, native-option migration input, and
theme-tokenized popup/listbox states. Navigation hover, keyboard focus, held
active paint, and disabled behavior now apply only to the real nav links/buttons
and tab buttons through generated navigation-item, D5 focus, zebra semantics,
and bounded host capture. Content hover and keyboard focus now apply only to the
checkable-chip button and selectable-list composite through generated
content-interaction, D5 focus, and zebra semantics, with passive
list/chip/table wrappers excluded; the state-matrix audit now recommends
`feedback:expanded` before feedback focus so visibility, initial focus,
dismissal, restoration, and modal/non-modal ownership are truthful first. The
feedback lifecycle audit is complete, but implementation is stopped at the
public owner/model decision: the shipped dialog and sheet surfaces are static,
own no opener or close transition, and cannot truthfully receive
`aria-expanded` or focus paint on their structural wrappers.

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
- [x] Decide and document the Phase 4 `input:hover` owners and executable
      acceptance before adding another fixture or changing component CSS.
  - [x] Inventory the actual native hover targets for `cem-field`,
        `cem-text-field`, `cem-textarea`, `cem-select`, `cem-checkbox`,
        `cem-radio`, and `cem-switch`; keep wrapper/label hover from becoming a
        substitute for the native control state.
  - [x] Decide semantic token mappings and visual channels for text-entry,
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
    - [x] Accept a boundary-only field-like hover treatment: change native
          `border-color` without changing fill or text, and map the boundary to
          a system color such as `Highlight` in forced colors.
    - [x] Run a temporary Playwright Chromium paint-channel spike before
          selecting the binary treatment. Compare native-hover-normalized
          pixels for unchecked and checked checkbox, radio, and switch controls
          across accent, border, background, outline, and box-shadow channels
          in normal and forced-colors modes.
      - `accent-color` changed checked controls but produced zero additional
        pixels for unchecked controls. Native `border-color` and
        `background-color` produced zero additional painted pixels, while
        `box-shadow` disappeared in forced colors.
      - A two-pixel outline produced 208 additional pixels for every binary
        control/state combination in both modes. It is technically reliable but
        remains unaccepted because it may be confused with keyboard focus.
    - [x] Decide whether binary hover uses a dedicated single-stroke outline
          below zebra focus priority or opts into custom control appearance to
          expose another boundary/indicator channel. Then accept exact token
          names, values/formulas, theme-mode mappings, and forced-colors
          behavior for both families.
      - Accepted one three-stripe stack with anchor/state, focus, and selection
        roles. Invalidity recolors the anchor rather than adding geometry.
        Fields default to underline; binary controls default to whole-label
        outline; `indicator` or the public appearance adapter selects the other
        geometry without changing state semantics.
      - Added nine generated D0 input-indicator color endpoints and two D5
        numeric appearance selectors. All component colors and widths resolve
        through CEM tokens; no CSS exception or raw component value was added.
  - [x] Pin real hover/unhover evidence, disabled and readonly boundaries,
        label/control overlap, focus-visible coexistence, restoration, geometry,
        DOM/ARIA, value/checked state, forced-colors, and event absence.
    - [x] Add Playwright-backed input-indicator acceptance covering
          all seven native owners, both appearance geometries, token-resolved
          state colors, disabled/readonly precedence, focus and selection
          coexistence, restoration, structure/runtime stability, and the
          forced-colors fallback contract.
      - The focused state fixture passes 13 tests and proves real native hover,
        unhover, family defaults, explicit and adapter overrides, invalid and
        readonly precedence, disabled suppression, checked/focus coexistence,
        stable geometry/DOM/runtime, unchanged values/ARIA, and zero mutation
        events.
      - The component-owned forced-colors target launches Chromium with
        `forcedColors: active`, collapses the normal shadow stack, verifies
        `Highlight` hover and full `CanvasText` focus outlines, and confirms
        disabled binary controls do not acquire hover treatment.
  - [x] Keep hover presentation CSS-only and author-imported: no host state,
        synthetic pointer events, runtime slice, or JavaScript style side effect.
  - Native interaction owners remain `cem-field input`, `cem-text-field input`,
    `cem-textarea textarea`, `cem-select select`, `cem-checkbox input`,
    `cem-radio input`, and `cem-switch input`; binary paint targets are their
    wrapping labels through `:has()`. The exact style contract passes against
    32 primitives and 382 generated visual theme tokens. `input:hover` is now
    browser-covered, yielding 28 covered, 0 static-only, and 11 gaps with
    `input:focus-visible` recommended next.
- [x] Complete the Phase 4 `input:focus-visible` acceptance using the existing
      CEM input-indicator contract; stop if evidence requires a new role,
      geometry, token, or component CSS exception.
  - [x] Add a dedicated Playwright-backed fixture that tabs through every
        enabled native owner in document order and proves disabled controls are
        skipped.
  - [x] Prove the focus stripe preserves each family appearance, anchor state,
        checked/indeterminate selection, geometry, DOM/ARIA, values, runtime
        snapshots, and event absence; prove blur restores the baseline stack.
  - [x] Extend the active forced-colors gate from representative field/binary
        owners to all seven inputs with full `CanvasText` focus outlines.
  - [x] Promote only `input:focus-visible`, update directly affected docs, and
        run focused browser, forced-colors, state-matrix, lint, package, and
        aggregate verification through Nx.
  - The dedicated fixture uses trusted Tab navigation to visit `cem-field`,
    `cem-text-field`, `cem-textarea`, `cem-select`, `cem-checkbox`, `cem-radio`,
    and `cem-switch` in order while skipping disabled field and checkbox
    controls. It proves underline/outline focus geometry, anchor colors,
    checked/indeterminate stripes, hover coexistence, blur restoration, stable
    DOM/ARIA/layout/runtime/value state, and zero mutation events.
  - The forced-colors gate now repeats the seven-owner keyboard sequence and
    proves every shadow collapses to a full `CanvasText` focus outline. Existing
    CEM tokens expressed the contract completely; component CSS and the closed
    exception queue required no change.
  - Focused state tests pass 14/14. Component lint, forced-colors verification,
    the exact 32-primitive/382-token style gate, package publication, and the
    aggregate 41-test gate pass. Only `input:focus-visible` moved to covered,
    yielding 29 covered, 0 static-only, and 10 gaps with `input:loading`
    recommended next.
- [x] Define and implement the Phase 4 `input:loading` projection contract
      across all seven native input owners.
  - [x] Document presence-only host `busy`, native `data-state="loading"` and
        `aria-busy="true"` reflection, lifecycle non-ownership, unchanged
        interaction, value/focus/node/dimension preservation, and state
        precedence.
  - [x] Add a generated CEM pending-stroke endpoint so the existing anchor
        stripe changes thickness as well as color; keep component CSS free of
        raw values and leave the CSS exception queue closed.
  - [x] Add the focused browser fixture before promoting `input:loading`,
        covering all seven owners, presence semantics, transitions, tokenized
        paint, forced colors, coexistence, identity, geometry, runtime state,
        and event absence.
  - [x] Run focused browser, theme-token, style, forced-colors, state-matrix,
        lint, package, and aggregate verification through Nx; promote only
        `input:loading` after every gate passes.
  - Presence-only `busy`, including `busy="false"`, now projects exact loading
    and ARIA markers to the same native owner without creating work, slices,
    lifecycle events, status content, interaction suppression, or node/value/
    focus/dimension changes.
  - D0 pending color composes with the new generated D5
    `--cem-stroke-pending` width. Invalid and disabled keep higher anchor
    precedence; focus and selection remain independent. Forced colors use a
    full `CanvasText` pending outline and retain the stronger focus width.
    `CEM-CSS-001` remains closed and the component exception queue remains
    empty.
  - Focused state coverage passes 15/15. The uncached aggregate component gate
    passes all 17 dependencies and 42 tests across 5 files; the style contract
    verifies 32 primitives and 383 generated visual tokens. Only
    `input:loading` moved to covered, yielding 30 covered, 0 static-only, and 9
    gaps with `input:expanded` recommended next.
- [x] Implement the accepted custom `cem-select` and close `input:expanded`.
  - [x] Extend CEM-QL tests-first with generic recursive serialized-payload
        projection so rich option HTML remains on the authoritative renderer.
  - [x] Add the opt-in browser behavior/FACE seam without putting functions,
        live DOM identity, or browser objects into snapshots or render plans.
  - [x] Add the focused custom-select browser fixture before promoting the
        state: canonical `cem-option`, legacy all-native option adaptation,
        rich content, groups, dropdown preview/commit/cancel, sized and multiple
        listboxes, keyboard/typeahead/pointer behavior, form participation,
        validity/reset/restore, disabled and dynamic-source boundaries.
  - [x] Adopt missing popup/option color and list-popup geometry endpoints into
        CEM theme; update the exact style contract without opening a component
        CSS exception.
  - [x] Document the component contract, update the state matrix and affected
        examples, then run focused and aggregate Nx verification before commit
        and push.
  - CEM-QL now owns generic `cem:project-payload` materialization with recursive
    element/text/comment coverage and fail-closed diagnostics. The browser
    runtime adds an opt-in behavior/FACE seam while snapshots and render plans
    remain serializable and CEM-QL remains the authoritative DOM producer.
  - `cem-select` implements dropdown, sized single-listbox, and multiple-listbox
    modes; rich canonical options and grouped options; legacy all-native option
    adaptation; keyboard, pointer, typeahead, preview/commit/cancel, repeated
    multi-value form data, required validity, reset/restore, and disabled-state
    behavior. Authored options are explicitly initialization payload rather than
    a live options collection.
  - Twelve D0 select-state colors and the D2c list-popup row endpoint are
    generated and exported. Component CSS consumes CEM theme tokens, while
    forced colors maps to platform system colors; the component CSS exception
    queue remains empty.
  - CEM-QL tests, 93 runtime Storybook tests, and 43 component tests pass. The
    aggregate component gate verifies 34 primitives and 396 generated visual
    tokens. State coverage is 31 browser-covered, 0 static-only, and 8 gaps,
    with `navigation:hover` recommended next.
- [x] Implement the Phase 4 `navigation:hover` contract for the real interactive
      owners in `cem-nav` and `cem-tabs`.
  - [x] Add focused browser coverage for trusted pointer enter/leave,
        current/selected coexistence, disabled suppression, focus-visible
        coexistence, stable geometry/DOM/ARIA/runtime state, and event absence.
  - [x] Add a forced-colors browser gate for navigation hover, current/selected,
        disabled, focus, restoration, and structural-wrapper isolation.
  - [x] Audit generated navigation/action tokens, adopt a D0 navigation-item
        state family if action semantics cannot represent the contract, and
        bind only the interactive owners in component CSS.
  - [x] Update the exact style contract, state matrix, component docs, and
        aggregate Nx verification without opening a CSS exception unless no
        theme category can represent the accepted styling.
  - Exact owners are direct nav links/buttons, disclosed-content links/buttons,
    the native disclosure button, and direct tab buttons. Nav, content, and
    tablist wrappers receive no navigation state declarations.
  - D0 now generates ten required default/hover/current/current-hover/disabled
    navigation-item color endpoints. Current links and selected tabs retain a
    distinct hover pair; disabled wins. Normal component CSS uses only generated
    tokens, forced colors uses system colors, and the exception queue stays
    empty.
  - The focused state suite passes 17/17 with trusted pointer boundary events,
    exact token/contrast assertions, focus and selection coexistence, disabled
    suppression, restoration, stable geometry/DOM/ARIA/runtime snapshots, and
    zero mutation events. The dedicated forced-colors gate verifies system
    mappings and wrapper isolation.
  - The uncached aggregate component gate passes all 18 dependencies and 44
    tests across five files. The exact style gate verifies 34 primitives and
    406 generated visual tokens. Only `navigation:hover` moved to covered,
    yielding 32 covered, 0 static-only, and 7 gaps with
    `navigation:focus-visible` recommended next.
- [x] Implement the Phase 4 `navigation:focus-visible` contract for the same
      interactive owners in `cem-nav` and `cem-tabs`.
  - [x] Add focused keyboard traversal coverage for direct and disclosed nav
        items and tabs, including native-disabled skipping, current/selected
        coexistence, restoration, and focus/hover coexistence.
  - [x] Bind the existing D5 focus stroke/offset and zebra focus color only to
        focusable navigation owners, preserving stable geometry and leaving
        structural wrappers unstyled.
  - [x] Extend the forced-colors browser gate for focus traversal, system-color
        focus paint, disabled skipping, restoration, and event/state absence.
  - [x] Update the exact style contract, state matrix, component docs, and
        aggregate Nx verification without opening a CSS exception.
  - D5's existing `--cem-stroke-focus` and
    `--cem-stroke-indicator-offset` endpoints plus the zebra focus color fully
    represent the ring. CSS binds only the native owners; normal mode contains
    no raw/local value, forced colors uses `CanvasText`, and the exception queue
    remains empty.
  - The focused state suite passes 18/18. Real Tab traversal covers direct nav
    links, the disclosure, disclosed content, and tabs; native-disabled buttons
    are skipped, while current/selected/expanded state, hover coexistence,
    geometry, DOM/ARIA, serializable runtime state, restoration, and event
    absence remain stable.
  - The forced-colors gate repeats the full order, including the intentionally
    still-focusable ARIA-disabled boundary owned by the later disabled contract,
    and verifies exact system paint, token geometry, wrapper isolation, and
    restoration. The state matrix now reports 33 covered, 0 static-only, and 6
    gaps with `navigation:active` recommended next.
  - The uncached aggregate component gate passes all 18 dependencies and 45
    tests across five files. The exact style gate continues to verify 34
    primitives and 406 generated visual tokens.
- [x] Implement the Phase 4 `navigation:active` contract on the actual nav-item
      and tab owners after resolving the disclosure activation boundary.
  - [x] Decide scope before CSS: the disclosure button shares navigation paint
        ownership, but release legitimately mutates `expanded`. Choose whether
        this contract owns only its held `:active` paint and delegates release
        to `navigation:expanded`, or owns the full disclosure transition.
    - Accepted the held-paint boundary: `navigation:active` owns the transient
      native pseudo-class and asserts no pre-release mutation. On release it
      verifies only the already-authorized toggle, while
      `navigation:expanded` remains the canonical transition owner.
  - [x] Audit D0 navigation-item and action active endpoints before CSS. Adopt
        navigation-specific active/current-active semantics if action-intent
        tokens cannot preserve navigation and current/selected meaning.
  - [x] Add trusted pointer hold/release coverage and native keyboard activation
        coverage: Enter for links/buttons and Space for buttons only.
  - [x] Prove current/selected and focus-visible coexistence, disabled
        suppression, readable contrast, release restoration, stable geometry,
        no pre-release mutation, and only the explicitly accepted click/state
        transition after release.
  - [x] Define forced-colors active paint on the real owners and update the
        exact style contract, state matrix, docs, and aggregate Nx gate without
        opening an exception unless theme review proves one unavoidable.
  - D0 now exposes distinct navigation active and current-active background/text
    endpoints. They resolve through generated theme artifacts (183 color tokens,
    448/448 required coverage, and 410 visual tokens), so the component layer
    needs no local value and the CSS exception queue remains empty.
  - The focused state suite passes 19/19. Trusted pointer holds and native
    keyboard activation cover direct nav links, disclosure content, tabs, and
    the disclosure button; active paint coexists with focus and current/selected
    state without geometry, DOM, ARIA, or pre-release runtime mutation.
    Disclosure release performs only its accepted expanded toggle, owned by the
    later `navigation:expanded` contract.
  - The forced-colors gate verifies active system paint, disabled suppression,
    wrapper isolation, stable geometry, and release restoration. The state
    matrix now reports 34 covered, 0 static-only, and 5 gaps with
    `navigation:disabled` recommended next.
  - The uncached aggregate component gate passes all 18 dependencies and 46
    tests across five files. The exact style gate verifies 34 primitives and
    410 generated visual tokens.
- [x] Decide and implement the Phase 4 `navigation:disabled` behavior for
      `cem-nav` and `cem-tabs` before changing runtime activation.
  - [x] Resolve the owner/API policy first: native-disabled buttons already
        suppress focus and activation, while ARIA-disabled links and buttons
        remain focusable and actionable unless component behavior intervenes.
        Decide whether CEM keeps ARIA-disabled owners discoverable in tab order,
        requires authored `tabindex="-1"`, or normalizes to native `disabled`
        where that attribute is valid.
    - Accepted: native buttons use `disabled` and retain browser-owned tab/activation
      suppression. Authored `aria-disabled="true"` owners remain discoverable in
      sequential focus with their existing focus indicator; CEM does not add or
      rewrite `tabindex`.
  - [x] Define who suppresses activation on projected authored descendants:
        component event delegation or a documented author responsibility. Stop
        before adding behavior if that ownership has not been accepted.
    - Accepted component ownership: `cem-nav` and `cem-tabs` intercept activation
      at their host capture boundary for only their direct rendered navigation
      owners. Pointer/programmatic click, Enter, and native-button Space cannot
      reach the target, trigger default action, submit a form, or escape to an
      application bubble listener. Earlier ancestor capture observation follows
      normal DOM dispatch; non-activation keys and unrelated nested controls
      remain outside the behavior.
  - [x] Add trusted pointer, click, Enter, Space, and Tab coverage for the chosen
        policy, including current/selected coexistence, focus-visible behavior,
        form neutrality, stable state/geometry, and exact event suppression.
  - [x] Recheck existing disabled theme and forced-colors paint, then update the
        state matrix, component docs, and uncached aggregate Nx gate. Add a CSS
        exception only if no theme category can represent required paint.
  - One package-owned capture behavior now guards only the direct rendered
    navigation owners. It prevents default action and target/application
    propagation for pointer, programmatic, Enter, and native-button Space
    activation while preserving ARIA-disabled focus discovery, link Space,
    unrelated nested behavior, owner identity, and serializable runtime state.
  - D0's existing navigation disabled pair and D5/zebra focus semantics fully
    represent paint. Enabled nav-button hover/active selectors now exclude
    ARIA-disabled buttons symmetrically with links, and explicit
    current/selected-disabled bindings make disabled paint win. No new theme
    token or CSS exception is required.
  - The focused state suite passes 20/20 with native-disabled tab skipping,
    ARIA-disabled focus retention, current/selected coexistence, exact trusted
    and programmatic click cancellation, Enter/Space boundaries, form
    neutrality, stable geometry/DOM/ARIA/runtime state, and no mutation events.
  - The forced-colors gate includes an ARIA-disabled selected tab and verifies
    `Canvas`/`GrayText`, `CanvasText` focus, restoration, wrapper isolation, and
    native-disabled skipping. The state matrix now reports 35 covered,
    0 static-only, and 4 gaps with `content:hover` recommended next.
  - The uncached aggregate component gate passes all 18 dependencies and 47
    tests across five files. The package gate publishes 24 files; the exact
    style gate still verifies 34 primitives and 410 generated visual tokens.
- [x] Decide the Phase 4 `content:hover` owner set before adding a fixture or
      component CSS.
  - [x] Reconcile the matrix's “interactive content row or chip” wording with
        the shipped DOM: `cem-chip[checkable]` has a clear native button owner;
        `cem-list[selectable]` exposes one native `select` composite rather than
        themeable option-row hover; and `cem-table` owns only a passive table
        wrapper while projected rows remain application-authored.
  - [x] Choose whether to narrow this state to existing interactive owners
        (recommended: checkable chip plus the selectable-list composite, with
        passive chip/list/table explicitly excluded) or first accept new
        component-owned interactive table/list-row vocabulary. Stop before CSS
        or behavior if that ownership choice is not accepted.
  - [x] After owner acceptance, audit action/select/content theme categories,
        add theme semantics before CSS if needed, and cover trusted pointer
        enter/leave, selected/checked/disabled/focus coexistence, stable
        geometry/state, forced colors, exact style bindings, docs, and matrix.
  - The token audit rejected action-intent coupling for content and rejected
    custom-`cem-select` option semantics for the native list composite. D0 now
    generates ten `--cem-content-interaction-*` default, hover, selected,
    selected-hover, and disabled endpoints. The theme build reports 193 color
    tokens, 458/458 required coverage, 470 extracted tokens, and 420 visual
    tokens; no component CSS exception was needed.
  - Exact selectors bind only `cem-list[selectable] > select` and
    `cem-chip[checkable] > button`. Checked chips retain distinct selected and
    selected-hover paint; disabled native owners suppress hover through
    `:enabled`; passive chip/list/table output receives no binding. This slice
    does not introduce host-level disabled behavior or interactive table rows.
  - The focused Chromium state suite passes 21/21 and proves trusted pointer
    enter/leave, exact theme paint and contrast, selected-option and checked-chip
    coexistence, native-disabled suppression, retained focus treatment, stable
    dimensions/DOM/ARIA/runtime state, restoration, passive exclusions, and no
    mutation events.
  - The dedicated forced-colors gate maps chip hover to
    `Highlight`/`HighlightText`, checked rest to
    `SelectedItem`/`SelectedItemText`, and disabled owners to
    `Canvas`/`GrayText`. The native listbox retains its platform Canvas surface
    and uses an existing `Highlight` border for hover without changing geometry
    or its independent focus outline.
  - The state matrix now reports 36 browser-covered, 0 static-only, and 3 gaps
    with `content:focus-visible` recommended next. The exact style gate verifies
    34 primitives and 420 generated visual tokens.
  - The uncached aggregate component gate passes all 19 dependencies and 48
    tests across five files, including the new content-hover forced-colors
    verifier. The package gate still publishes 24 dist-only files with one
    canonical stylesheet.
- [x] Decide and implement the Phase 4 `content:focus-visible` owner and paint
      contract.
  - [x] Reuse the shipped interactive owner boundary unless evidence requires a
        different one (recommended: the selectable-list `<select>` and
        checkable-chip `<button>` only; keep passive list/chip/table output and
        application-authored table rows excluded).
  - [x] Audit D5 focus geometry and zebra focus color before adding CSS. Prefer
        the existing external `--cem-stroke-focus` /
        `--cem-stroke-indicator-offset` ring with `--cem-zebra-color-1`; add D0
        semantics only if checked/selected focus coexistence cannot be expressed
        independently, and record an exception only if no theme category fits.
  - [x] Prove sequential keyboard order, native-disabled skipping, exact
        `:focus-visible` ownership, selected-option and checked-chip coexistence,
        hover coexistence/restoration, stable geometry/DOM/runtime state,
        wrapper isolation, and normal/forced-colors behavior without adding
        focus handlers or mutating selection/checked state.
  - D5's existing focus width/offset and the zebra focus color fully represent
    the ring, so no D0 token or CSS exception was needed. Exact
    `:enabled:focus-visible` selectors bind only the selectable-list `<select>`
    and checkable-chip `<button>`; the chip's documented token surface now
    includes stroke semantics.
  - The focused Chromium state suite passes 22/22 and proves the exact native
    Tab order from listbox to unchecked chip to checked chip, then the end
    sentinel. Disabled native owners and passive list/chip/table output are
    skipped, the D5/zebra ring resolves exactly, hover retains the ring, and
    selected/checked/DOM/runtime state and dimensions remain stable.
  - The combined content hover/focus forced-colors gate verifies `CanvasText`
    focus width/offset on every enabled owner alongside checked
    `SelectedItem`, chip hover `Highlight`, and the listbox's independent
    `Highlight` hover border. It also proves native-disabled skipping,
    restoration, wrapper isolation, and no mutation events.
  - The state matrix now reports 37 browser-covered, 0 static-only, and 2 gaps;
    the later lifecycle audit reprioritized `feedback:expanded` before feedback
    focus. The exact style gate remains
    at 34 primitives and 420 generated visual tokens.
  - The uncached aggregate component gate passes all 19 dependencies and 49
    tests across five files. The package gate still publishes 24 dist-only files
    with one canonical stylesheet.
- [ ] Decide the Phase 4 feedback focus owner and lifecycle boundary before
      implementing `feedback:focus-visible`.
  - [x] Reconcile the matrix's “move keyboard focus into a feedback surface”
        wording with shipped output: `cem-dialog` and `cem-dialog-shell` render
        non-focusable structural `div[role="dialog"]` wrappers, `cem-sheet`
        renders a non-focusable `aside[role="region"]`, and all focusable
        descendants are application-authored.
  - [x] Choose whether to sequence `feedback:expanded` first so open state,
        initial-focus ownership, dismissal, restoration, and modal trapping are
        truthful before focus coverage (recommended), or narrow feedback focus
        to an explicitly accepted component-owned descendant contract. Do not
        add `tabindex` or `:focus-within` paint to structural wrappers merely to
        close the matrix row.
    - Accepted the recommended sequencing. The executable matrix now places
      `feedback:expanded` before `feedback:focus-visible`; no runtime, fixture,
      CSS, theme token, or exception changed in this audit-only slice.
  - [x] Decide the public `feedback:expanded` owner and lifecycle model before
        adding a fixture or runtime behavior.
    - Accepted the
      [`feedback expanded contract`](../packages/cem-components/docs/feedback-expanded-contract.md):
      preserve byte-equivalent static defaults; use presence-only `transient`
      plus current `expanded` state as the opt-in lifecycle; give `cem-dialog`
      and `cem-dialog-shell` one shared native `<dialog>` / `showModal()`
      behavior; keep `cem-sheet` a non-modal hidden/visible region; and leave
      `aria-expanded`/`aria-controls` on the application-owned opener.
    - Native dialog cancel/close, initial focus, modal containment, restoration,
      serializable post-close `cem-dismiss`, host-state synchronization,
      stable payload identity, redundant calls, disconnect/reconnect cleanup,
      and the absence of component-owned sheet focus or dismissal are pinned
      before runtime work. No theme token, component CSS, fixture, or exception
      changed in this decision slice.
  - [ ] Implement the accepted `feedback:expanded` contract tests-first.
    - [x] Exercise the native-dialog renderer stop condition with the red
          fixture before landing component behavior.
      - Both `userEvent.tab()` and a trusted keyboard Tab characterized native
        Chromium boundaries: focus may temporarily become `body`, outside page
        controls remain inert, and the next sequential move re-enters the
        dialog. The accepted contract now records this platform behavior without
        adding a custom Tab loop.
      - More importantly, changing only an open dialog host's `label` retained
        node identity but produced exact `open` mutations `"" -> null -> ""`.
        The merge removed the browser-owned attribute, the behavior reopened the
        dialog, and later close restored `body` rather than the original opener.
        Per the accepted stop condition, the experimental fixture/behavior was
        not landed and the existing component sources were restored.
    - [x] Accept and implement a generic `cem-elements` rendered-attribute
          ownership boundary before retrying the feedback fixture.
      - Recommended: add an opt-in attribute-preservation predicate to the DOM
        merge options, expose it through browser-only
        `CemProducedElementBehavior`, and have `CemElementRuntime` forward it for
        the current produced instance. Preserve only attributes explicitly
        claimed by that predicate; desired plan attributes remain authoritative
        and unrelated undeclared attributes must still be removed.
      - Prove the projection primitive first, then browser integration: an open
        native dialog must have zero `open` mutations across an unrelated host
        render, retain the original focus-restoration target, and still close
        through `close()` before an authored state change, owner replacement, or
        disconnect. Keep dialog/component names out of the generic projection
        module.
      - Implemented `preserveElementAttribute` on the projection merge options
        and browser-only `preserveRenderedAttribute` on
        `CemProducedElementBehavior`; `CemElementRuntime` forwards the predicate
        only while patching the current produced instance. The synchronizer asks
        only about omitted current attributes, so desired plan values remain
        authoritative and unclaimed extras are removed normally.
      - Added direct render-plan and produced-element Chromium stories. The red
        run observed the claimed attribute being removed and two native `open`
        mutations; the green run proves exact retention/override semantics, zero
        `open` mutations through a label render, stable modal/focused-owner
        identity, original-opener restoration, and native close before authored
        state, recovery replacement, and disconnect. No dialog or feedback name
        was added to the generic projection implementation.
    - [x] Add a focused browser fixture covering passive compatibility;
          transient initialization and host-attribute transitions; native modal
          state for both dialog tags; non-modal sheet visibility; initial focus,
          forward/reverse Tab containment, Escape/prevented cancel, native close
          return value, focus restoration; exact external trigger ARIA;
          serialized `cem-dismiss`; stable DOM/state/geometry; and
          close/disconnect/reconnect cleanup before adding behavior.
      - Added declarative `tests/feedback/expanded.html` markup plus a dedicated
        Chromium acceptance spec. Passive output compatibility is an ordinary
        passing test; the four behavior-dependent scenarios are executable
        `it.fails` cases so the accepted contract remains red without leaving
        the package gate failed before production behavior lands.
      - The initial red run passed passive compatibility and failed all four
        transient scenarios at the intended current boundaries: neither dialog
        tag rendered a native `<dialog>`, and the closed transient sheet was not
        hidden. The fixture already pins native owner/state transitions, modal
        focus and Tab boundaries, prevented/successful Escape, form return
        value, external opener ARIA, serializable dismissal, non-modal sheet
        behavior, DOM/input/geometry stability, zero `open` churn, application
        close, native-owner replacement, disconnect, and reconnect cleanup. No
        primitive behavior, CSS, theme token, or exception changed in this
        fixture-only slice.
    - [ ] Add one shared dialog behavior adapter and declarative transient
          branches without adding a custom inert sweep, Tab loop, structural
          wrapper focusability, sheet Escape/focus handling, or direct
          browser-owned `open` mutation.
    - [ ] Update component/accessibility docs and the executable matrix, then
          run the focused state suite, state-matrix audit, lint, and aggregate
          package gate before marking implementation complete.
  - [ ] After ownership acceptance, audit D5/zebra and the relevant descendant
        component token family, then cover keyboard entry/order, modal versus
        non-modal behavior, disabled skipping, focus restoration, stable
        geometry/DOM/runtime state, forced colors, exact bindings, docs, and the
        matrix. Record a CSS exception only if no theme category represents the
        accepted paint.

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
