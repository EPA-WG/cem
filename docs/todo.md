# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [roadmap.md](../roadmap.md), and future
wishlist work lives in [wishlist.md](wishlist.md).

The [2026-10-07 snapshot](archive/todo-snapshot-2026-10-07.md) preserves the completed checklists,
adopted decisions and verification evidence removed in this cleanup. Its open
items are historical context; only this file governs active work. Earlier
history remains in the [2026-09-27 snapshot](archive/todo-snapshot-2026-09-27.md)
and [follow-up log](archive/todo-completed-2026-09-27.md).

## AST node reference implementation

The accepted [node references design](cem-ql-cem-ml-node-references-design.md)
governs this work. Parsing retains expressions and never executes reference
selection on load. Consumers supply runtime contexts, scope grants and bounded
evaluation at their lifecycle stage; context roots need no IDs.

Core reference syntax, retained owners, lexical capture, bounded resolution,
supported schema consumers, codecs, query/reload/transport contracts and native
host/syntax adapters are implemented. See the [consumer verification matrix](reference-consumer-verification.md)
and [archived implementation history](archive/todo-snapshot-2026-10-07.md#ast-node-reference-implementation).
General datatype/function composition and conditional typed prelude payloads
remain open below. Further cem-element consumer work is tracked in its
[consumer list](#deferred-cem-element-reference-consumption).

### 5. Integrate schema validation and construct reuse

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#5-integrate-schema-validation-and-construct-reuse).

- [ ] Adopt the general datatype compilation design and enable bounded singleton
      attribute type consumption only after its executable contracts are chosen.
      Draft: [general datatype compilation proposal](cem-datatype-compilation-proposal.md).
      Scenarios for later design verification: unsupported declarations cannot
      silently lose rules; source owners, lexical scope and budgets survive reuse.

- [ ] Define native datatype and function scalar composition contracts before
      extending their existing string-valued sites. Element `@base` is already
      implemented under the dedicated singleton `element-base` contract above;
      it is not blocked by these remaining consumer contracts.
      Scenarios for later design verification: singleton composition rejects
      multiple targets; recursive validation stays bounded; existing aliases
      remain compatible without making scalar strings implicit constructors.

- [ ] Specify typed prelude reference slots only if that source capability is
      requested. Current directive payloads are literal text; native reference and
      expression slots already belong to schema-element/host attribute forms. Keep
      this separate from the adopted enclosed host-bound child syntax.
      Scenarios for later design verification: a literal query constructor is not an
      authored AST reference slot; consumers retain original source occurrences and
      do not manufacture attributes or reparse a second CEM tree.

### Scenarios for later design verification

These scenarios are preserved from the adopted proposal for future design and
implementation verification. They do not claim executable test coverage.

- A scope default supplies the same effective property to many nodes without
  repeated references or an authored root ID.
- Two vendor scopes contain equal ID strings without accidental cross-scope
  selection; an explicit boundary contract permits an intended relationship.
- A URL selects a publicly exposed part of an external document without
  introducing URL resolution into CEM-QL.
- One retained template reference is evaluated during two transformations with
  different supplied `datadom` contexts and without mutating a shared target
  list on the authored occurrence.
- A consumer deeply resolves a chain of reference nodes within its effective
  scope policy while preserving the authored graph. A cycle or traversal limit
  stops resolution even when every link remains in the same lexical scope.
- Unresolved links follow mandatory, warning, or ignore requirements declared
  by the applicable scope schema; a resolved empty selection is handled
  separately by the schema's cardinality rule.
- Schema constructs are declared once and referenced by composition and
  validation consumers without copying them into the source tree.

## Reference adoption: dependency binding and readiness

General datatype execution, conversion, equality and enumeration authoring are a
separate workstream below. Its adopted decisions remain effective. They do not
block core reference adoption; executable native attribute `@type` specifically
remains guarded until that compiler is ready.

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#reference-adoption-dependency-binding-and-readiness).

- [ ] Integrate dependency selection outcomes with schema consumer readiness when
      the separate datatype compiler supplies its executable contracts. Add readiness
      fixtures before integration; preserve available sources for inspection and the
      last complete active package while replacement dependencies remain incomplete.
      Scenarios for later design verification: complete selection cannot activate an
      unavailable execution contract; pending lookup is distinct from a complete empty result.

## General datatype compilation design

Separate workstream: preserve all adopted contracts here, but keep execution,
equality and enumeration decisions outside the core reference implementation.
The enumeration authoring recommendation remains undecided and deferred to this
workstream; no new constant syntax is adopted by the reference work.

The [adopted datatype decisions](archive/todo-snapshot-2026-10-07.md#general-datatype-compilation-design)
remain requirements for the open work below. The [temporary proposal](cem-datatype-compilation-proposal.md)
gathers those contracts; completed registry/source/dependency foundations are
archived, while executable datatype consumption remains guarded.

- [ ] Design explicit datatype declaration overrides as separate future work:
      specify replaced declaration identity, authorization, dependency rebinding
      and coordinated activation before admitting syntax or implementation.
      Scenarios for later design verification: an application can intentionally
      replace a contract without import order silently weakening restrictions;
      existing native selections retain identity unless explicit rebinding is chosen.

- [ ] Specify list conversion adapters and explicit canonical serialization, then
      add fixture actions for ordered items, duplicates and retained item provenance
      before implementing conversion or changing scalar API result types.
      Scenarios for later design verification: typed results preserve item identity;
      serialization does not overwrite authored lexical input or clone native nodes.

- [ ] Specify a registered datatype validation
      adapter that checks retained behavior/owner identity, required input bindings
      and kind-specific representation, distinct from diagnostic-only behaviors.
      Add fixture actions before implementing signature checking or invocation.
      Scenarios for later design verification: original candidate/type handles remain
      native; unavailable execution differs from invalid data; conversion stays separate.

- [ ] Specify custom list cardinality admission and registered tokenizer contracts,
      then add fixture actions for absent versus empty input, inherited nonempty
      restrictions and invalid lexical tokens before implementing adapters.
      Scenarios for later design verification: an item base does not require an item;
      empty conversion manufactures neither a default item nor a null value.

- [ ] Specify node-kind metamodel admission, base compatibility and native sequence
      signatures; add fixture actions for retained targets, lexical-facet rejection
      and unchanged descendant reference nodes before implementing the node adapter.
      Scenarios for later design verification: native validation never stringifies
      targets or expands descendant references implicitly; scope bounds still apply.

- [ ] Specify concrete validation result bindings and an explicit compatibility
      adapter for diagnostic-only behaviors, with a declared acceptance mapping.
      Add fixture actions for acceptance/diagnostic independence, cumulative rejection,
      missing-result failures and retained source attribution before implementation.
      Scenarios for later design verification: an accepted rule cannot erase another
      restriction's rejection; lifecycle incompleteness cannot masquerade as success.

- [ ] Specify the registered implementation identity, typed input/result signature,
      package ownership checks and compilation/validation/conversion adapters.
      Scenarios for later design verification: registration grants no scope access;
      an unrelated vendor type cannot borrow a built-in implementation by name.

- [ ] Establish full validation/conversion parity for shipped datatype families;
      add fixture actions for lexical, URI/semver/media-type/path, list, grammar
      and symbolic-reference implementations before adding those implementations.
      Scenarios for later design verification: unsupported conversion stays explicit;
      a primitive predicate alone does not prove a complete conversion contract.

- [ ] Complete datatype namespace/export API and metamodel admission, registered
      tokenization policies, and grammar/reference kind contracts.
      Scenarios for later design verification: attribute restrictions cannot widen
      a base; native node contracts retain typed input without scalar extraction.

- [ ] Design whole-list inheritance syntax, dependency roles and compatibility as
      separate future work before admitting derived whole-list declarations.
      Scenarios for later design verification: list-of-list items are not inheritance;
      existing shipped item-base declarations preserve their effective item contracts.

- [ ] Specify the registered base compatibility contract and its source-attributed
      validation; add fixture actions for inherited-kind reuse, permitted shipped
      cross-kind bases, incompatible bases and unavailable registration before wiring
      the bounded dependency compiler.
      Scenarios for later design verification: a derived restriction retains the
      base implementation identity; incompatible contracts never activate by name.

- [ ] Implement effective converter selection with original implementation identity
      and registered output compatibility; add fixture actions for inherited reuse,
      explicit replacement, unavailable selected capability and validation-only types
      before wiring executable conversion. Never fall back after selected failure.
      Scenarios for later design verification: one converter produces the canonical
      value; inherited/local restrictions all inspect it without rewriting or chaining.

- [ ] Implement registered scalar equality and enumeration constant interpretation,
      preserving each inherited restriction's original contract and binding. Add
      fixture actions before adapters for numeric/string equality, unavailable equality
      and inherited restriction provenance; do not rewrite input during comparison.
      Scenarios for later design verification: a derived equality cannot widen a base
      vocabulary; unknown constant interpretation prevents readiness rather than fallback.

- [ ] Decide enumeration constant authoring: preserve whitespace-token `values` and
      defer richer retained constants (recommended), or design retained constant
      declarations now with source/reference/metamodel compatibility.
      Scenarios for later design verification: one string containing spaces is never
      silently split into a different claimed enum constant; no implicit JSON parsing.

- [ ] Implement retained datatype descriptors and lexical name binding, then a
      bounded dependency compiler and kind consumers under the adopted contracts.
      Add focused native fixture items before implementing each slice.
      Scenarios for later design verification: cycles and incomplete dependencies
      prevent activation while original declarations stay available for inspection.

- [ ] Integrate the descriptor into literal and native attribute type consumers,
      preserve compatibility through explicit parity checks, and update the
      metamodel's native datatype admission contract before enabling consumption.
      Scenarios for later design verification: referenced type owners and their
      restrictions survive package replacement and independent runtime contexts.

- [ ] Once adopted, merge the proposal into maintained designs and remove the
      temporary proposal, preserving outstanding action items and scenarios.
      Scenarios for later design verification: normative docs distinguish supported
      consumers from implementation phases and genuinely deferred decisions.

## Deferred cem-element reference consumption

Initially deferred on 2026-10-04 until the CEM-ML reference design was complete.
The [consumer mode](cem-element-reference-ids-design.md) is adopted and implemented
on 2026-10-07 for local and explicitly admitted foreign producer relationships. Generic
CEM-QL export remains separate. Explicit source lifecycle evaluation is available
through the native embedding adapter and explicit browser/worker host inputs.
Each invocation supplies original capture, context readiness, effective policies
and directed grants; document ownership and producer placement create no authority.

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#deferred-cem-element-reference-consumption).

### Next consumer actions

Execution order: continue the remaining consumer actions below while
general datatype execution awaits its separate contracts. Conditional prelude
authoring remains deferred unless explicitly requested.

Adopted 2026-10-07: allow transaction-scoped prepared placements. The host names
all producers and revisions, retains the private preparation snapshot and activates
relationships only after the whole group commits. Ordinary publication and
serialized resume hints cannot confer this readiness or authority.

- [x] Implement the bounded native placement-admission snapshot and export API
      from the [placement design](cem-element-granted-placements-design.md).
      Add native fixtures before implementation for both grant requirements,
      unique placement selection, producer-owned ID reservations, request/destination
      work bounds and stale/revoked revisions. Add real WASM worker/fallback cases.
      Scenarios for later design verification: a matching foreign ID grants
      nothing; local ambiguity stays ambiguous; an unready or revoked admission
      returns no replacement plan and does not mutate another producer's AST.
      Completed 2026-10-07: eight native admission fixtures and committed/prepared
      real-WASM cases in both heaps; the maintained gate passes 336 checks.

- [x] Implement the host placement registry/coordinator and transactional browser
      publication using native admission tokens. Add browser fixtures before
      implementation for same-root readiness, ID reservation conflicts, nested
      producer readiness, replacement races, revocation and disconnect cleanup.
      Scenarios for later design verification: changed dependencies invalidate
      preparation; coordinated producers publish all relationships together;
      revocation clears dependent routes immediately without transferring context.
      Completed 2026-10-07: five authority unit fixtures and four browser stories
      cover copied snapshots, partial revocation, activation rollback, nested
      prepared producers, conflicts, stale publication, activation-time revocation
      with safe retry and worker/fallback cleanup.

- [x] Extend placement admission to SSR and hydration with fresh host authority.
      Add retained Node/Edge/browser fixtures before implementation for coordinated
      SSR publication, profile/revision handoff, reconnect and stale resume hints.
      Scenarios for later design verification: serialized placement descriptors
      grant no authority; hydration enables routes only after live re-admission;
      stable producer identity does not make an uncommitted revision ready.
      Completed 2026-10-07: retained SSR group fixtures and worker/fallback
      hydration/reconnect stories pass; all 56 Node SSR and 12 Edge browser checks
      pass with the fixed Recommendation profile and authority-free resume hints.

- [x] Implement explicit versioned ARIA export profile selection after native and
      worker/fallback/SSR profile fixtures, using the [reviewed contract](cem-element-aria-reference-profile.md).
      Document a browser/assistive-technology compatibility matrix before enabling
      the experimental draft profile; preserve the Recommendation default.
      Scenarios for later design verification: one/many/empty/repeated targets
      obey the selected profile; profile mismatch requires rerender; unknown
      profiles fail preparation; authored reference order/identity remain intact.
      Fixture work: add native cardinality/identity cases, real WASM worker and
      fallback export cases, processing identity checks, SSR profile handoff and
      browser hydration mismatch coverage before implementing each layer.
      Completed 2026-10-07: exact pinned default/experimental options propagate
      through native export, workers, fallback, cache/revision identity and SSR;
      unknown profiles reject and hydration mismatch rerenders. Native cardinality,
      retained placement metadata, real-WASM heaps and focused browser/SSR checks
      pass. The published matrix separates DOM evidence from pending manual AT work.

- [x] Extend the accepted focus/geometry contract to native dialog/task/tooltip
      surface adapters and invocation-captured pointer/selection geometry after
      their semantic lifecycle is implemented. Add native/browser fixtures before
      each adapter; reuse typed reference consumption and shared role checks.
      Scenarios for later design verification: modal `none` preserves native
      focus entry; pointer geometry supplies no return-focus destination; missing
      invocation geometry rejects opening rather than borrowing another context.
      Adopted implementation scope 2026-10-07: shared native-surface lifecycle
      and reference adapters first; component migration remains separate.
      Fixture work: add native dialog/tooltip reference slots, immutable pointer
      and selection capture/logical fitting units, and browser plays for native modal/nonmodal/
      popover ownership, typed focus/return/boundary endpoints, context rejection,
      cancel versus forced close, missing geometry, tooltip interest and cleanup.
      Fixture work: verify that a completed Tab/outside interaction cannot set a
      later native close reason, changed overflow releases prior size constraints,
      and changing a tooltip profile releases owned description bindings.
      Completed 2026-10-07: eager `native-surface` shared capability/controller,
      native modal/nonmodal/popover ownership and tooltip interest lifecycle use
      the existing typed role adapters. Immutable invocation geometry and logical
      fitting units, native role slots and five browser stories cover the scope;
      both worker and fallback retain modal owners/focus across rerender. Further
      convenience/body lifecycle delivery and component migration stay separate.
      Verification: maintained native consumer gate 339 checks across 36 suites
      plus real WASM workers/fallback; runtime units 600, focused browser 30,
      Node SSR 58 and Edge browser 13. Build, typecheck and lint pass (two existing
      lint warnings). Manual AT interoperability is not covered by these checks.

### Scenarios for later design verification

- A consumer uses the referenced node's declared meaning to generate a native
  relationship, preserving a target element's explicit ID or generating one.
- Repeated template instances resolve references against their own supplied
  contexts and produced elements. Verify intended scope crossings and the
  specified local-name compatibility without a document-wide CEM ID lookup.

## Interaction design implementation: cem-action

The [accepted interaction design](cem-interaction-design.md) takes precedence over conflicting implementation. Deliver one component at a time; this first step owns the action invoker, not popup/menu/dialog lifecycle.

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#interaction-design-implementation-cem-action).

- [x] Follow-up fixture: preserve unrelated anonymous demo slice state when native
      popup/modal activation is followed by a theme-switch rerender. The earlier
      combined sequence reset the layout-choice demo; retain the full sequence
      as a regression check.
      Fixture work: extend the source/installed action-gallery check across the
      full native surface sequence and add a focused worker/fallback runtime
      story asserting retained demo/declaration/instance identity and slice state.
      Scenarios for later design verification: native visibility/focus changes
      do not replace unrelated anonymous owners; theme projection preserves an
      edited slice without replaying its authored default.
      Completed 2026-10-07: the current runtime passes the extended action gallery
      from source and isolated package archives without a runtime change. The
      colocated theme-switch story verifies worker/fallback slice state and exact
      demo/declaration/instance/group identity after popover and modal commands,
      then Dark/Native/Light rerenders. All 30 focused theme-switch/action/native-
      surface browser plays, source/installed action checks, typecheck and lint
      pass; the lint warnings are existing baseline warnings.

## Autocomplete and suggestions design for CEM inputs

Reference: [legacy autocomplete 0.0.39](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/autocomplete.html).

- [ ] Audit and map the source autocomplete API and examples, including its `input`/`menu` slots, explicit data/option values, text-as-value fallback, grouped suggestions, filtering and selected-value behavior; distinguish preserved contracts from extensions.

- [ ] Design reusable suggestions composition that attaches to existing `cem-field`, `cem-text-field` and applicable CEM input controls through their actual native input; preserve one form owner, naming, value/input/change events, validation, disabled/readonly behavior and normal editing. Decide textarea applicability explicitly.

- [ ] Evaluate reuse of `cem-dropdown` for autocomplete/suggestions: compare direct dropdown composition with reuse of its shared popup capability/controller for visibility, placement, collision handling and dismissal. Keep focus on the editable input, use combobox/listbox semantics rather than menu semantics, and avoid duplicate triggers, focus restoration handlers or form controls; record the selected composition and tradeoffs before implementation.

- [ ] Specify declarative attachment/slots and the shared cem-elements capability for filtering, active suggestion, commit/cancel and popup lifecycle. Reuse dropdown popup geometry/dismissal without menu roles: editable combobox + listbox/options, input focus retained with aria-activedescendant and explicit controls/expanded relationships.

- [ ] Define ArrowUp/Down, Enter commit, Escape cancel/dismiss, Tab, pointer selection, IME composition, native text editing, empty/no-match and hidden/disabled suggestion behavior; distinguish display label from committed value and define free-text versus constrained selection.

- [ ] Define static and externally loaded suggestion data at the retained CEM tree boundary, with optional asynchronous loading, stale-response handling, loading/error feedback and accessible announcements; keep UI behavior out of components, galleries and application JavaScript.

- [ ] Fixture: add shared native contract cases first, then browser stories applying the same suggestions to cem-field and cem-text-field (and any accepted additional input), covering forms, dynamic suggestions, grouped filtering, focus, IME, independent controls and cancellation.

- [ ] Fixture: plan linked property playgrounds and galleries mapping source examples, all five themes, forced colors, keyboard/pointer use and source/generated/installed-package checks; update public attribute/slot inventory, accessibility contracts, catalog and exports with implementation.

## Respect action states in cem-icon-button

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#respect-action-states-in-cem-icon-button).

- [ ] Run the new browser state/paint checks and full source/package gallery
      verification when browser execution is permitted. Restricted session
      permissions still prevent that verification.
      Follow-up 2026-10-07: browser execution is available. Dimensions, Hover and
      Active fail identically with the current and pre-change runtime: forwarded
      size is `normal` where the fixture expects absence, and baseline background
      resolves to 72 rather than 239. Diagnose the component/fixture contract before
      changing public CSS ownership; the reference placement suite is independent.

## Compose cem-icon-button from cem-icon

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#compose-cem-icon-button-from-cem-icon).

- [ ] Rerun focused icon/icon-button browser stories and source/installed-package
      playground and bundle checks when browser execution is permitted. The first
      browser run passed 12 of 15 tests; its failures exposed empty-source
      forwarding and a style assertion that included the nested declaration.
      Both are corrected; native rendering confirms empty-source preservation.
      Browser rerun is pending after the session changed to restricted permissions.
      Command: `yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-icon-button/cem-icon-button.stories.ts packages/cem-components/src/components/cem-icon/cem-icon.stories.ts`.

## Convert five icon gallery demos to CEM-ML

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#convert-five-icon-gallery-demos-to-cem-ml).

- [ ] Formatter follow-up: repeated tabular CEM formatting adds blank lines to
      text-bearing nodes. Keep the first native conversion output for this task;
      diagnose formatter idempotence separately with a minimal native fixture.

## Remove fixture-only gallery IDs

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#remove-fixture-only-gallery-ids).

- [ ] Run source/installed-package browser checks when execution is available.

## Organize cem-icon gallery examples

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#organize-cem-icon-gallery-examples).

- [ ] Run browser gallery verification when session permissions allow it.

## Fix duplicate Font Awesome glyphs in cem-icon

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#fix-duplicate-font-awesome-glyphs-in-cem-icon).

- [ ] Run icon stories and source/installed-package galleries when browser
      execution is available; current session restrictions still apply.

## Align cem-icon with the label principle

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#align-cem-icon-with-the-label-principle).

- [ ] Rerun icon stories after removing `name` (browser execution requires host permissions).

- [ ] Complete source/installed-package playground verification, then commit and push.
      The prior playground run was interrupted by the session environment change;
      `.git` is now read-only and network access is restricted. Package checks
      passed before that change; the rerun hit `spawnSync npm EPERM`. Stop the
      restricted reruns and resume these checks in a browser-capable environment
      with Git write access.

- [ ] Next: align `cem-icon-button` with the shared label principle; its current
      accessibility-only `label` remains an inconsistency outside this change.

## Browser failures observed during placement verification

- [ ] Fixture: diagnose the workflow gallery's missing `cem-text-field input`
      after its sample readiness check. It fails in focused runs with both the
      current runtime and the pre-placement runtime; preserve declaration/load
      diagnostics before changing readiness or component behavior.
      Scenarios for later design verification: a ready demo marker must not hide
      incomplete nested declarations; native input readiness must remain distinct
      from initial sample publication.

- [ ] Fixture: capture the form-isolation story's rendered-control readiness under
      aggregate browser load before replacing its single-frame wait. FormData
      lacked `visible=ok` once in the 417-case run; both focused current/baseline
      runs pass all nine isolation cases. Preserve inert island assertions.
      Scenarios for later design verification: the live control submits once after
      actual render completion; captured controls stay disconnected and absent
      from FormData. Do not attribute the recurrence to references without evidence.

## Immediate: Legacy Demo Case Coverage

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#immediate-legacy-demo-case-coverage).

- [ ] Stabilize browser startup waits under parallel Storybook load, after
      DATA-CELL-MATCH-1: the unchanged scoped-CSS and legacy icon-link stories
      intermittently exhaust their short frame/two-second waits, although
      isolated and earlier full runs pass. Prefer declaration/render readiness
      over incidental delays. Also diagnose the source-loaded cell-overrides
      stock sample's initial-warning timeout seen once in the full gallery;
      six focused standalone/source-loaded runs passed without reproducing it.
      Establish its cause before treating it as the same startup issue.
      2026-09-21: scoped-CSS, hex-grid and legacy parity helpers now wait on
      rendered output/style/image conditions within the 30-second story budget,
      replacing frame-count and two-second cutoffs. All 29 cases in the five
      affected story files pass together and in the full parallel suite. The
      table's invalid-input alert also needed its existing ten-second interaction
      budget under parallel load. Final full result: 179 passed, three separate
      CEM-QL/local-storage failures tracked below. Stock-warning checks pass in
      both focused gallery modes; its earlier intermittent cause remains open.
      After native storage migration, all 190 browser stories and the full
      gallery (28 standalone pages, 34 source-loaded documents) pass. The
      original intermittent stock-warning cause still needs investigation.
      Follow-up 2026-09-21: all 198 current stories and 24 concurrent startup
      probes (12 gallery-helper / 12 real demo-component, with three declaration
      release schedules) pass. No lost markers, missing mounts or diagnostics
      reproduce. The user selected [continued investigation](browser-stabilization-review.tmp.md#startup-follow-up-decision-continue-investigation);
      keep this item active before the authored-sample coverage inventory.
      Sequencing update 2026-09-23: the user approved beginning that inventory
      while historical timeout attribution remains open on traced recurrence.
      This supersedes the earlier stabilization-first order, not its open
      investigations. The inventory follow-up below records the accepted work.
      Follow-up 2026-09-24: the cell-overrides audit and real source-harness
      component pass their focused stories and all 31 standalone / 37
      source-loaded gallery documents. The referrer matrix's scalar-URL frame
      timeout recurred in both final aggregate runs and a dedicated full
      Storybook rerun (219/220 each), temporarily blocking aggregate sign-off.
      The specific referrer fixture below now attributes and corrects it.

  - [ ] Audit remaining frame-count readiness helpers and aggregate-count
        predicates in demo stories. Check the authored instance inventory and
        available lifecycle signals before migrating each affected fixture;
        keep historical stock-timeout attribution separate from fixture fixes.
        Prioritize the newly observed failures: external-src declaration loading
        waits 120 frames for its first button; NPM-version samples wait 200
        frames for the default selection; location samples wait 180 frames for
        both readers. Capture declaration/render/worker state before attributing
        these failures or changing waits. Normal coverage passes, but that alone
        does not establish their failure cause under combined load.
        Progress 2026-09-22: opt-in lifecycle and frame-wait traces reproduce
        external-src's 120-frame timeout during pending rendering after clean
        declaration settlement. Its render settles 49 ms later without errors;
        later assertions remain unverified. Propose lifecycle-based readiness
        within the existing story limit. NPM/location pass this repeat and still
        require captured failure state before attribution; see the attribution
        section linked above. No waits changed in this investigation.
        The hook-eligibility validation also observes two declaration-source
        retry stories failing their default 1,000 ms `waitFor` after declaration
        settlement. Their four-story isolated run and a subsequent full suite
        pass; capture render/worker state on recurrence before attributing the
        failure or migrating these waits. Do not change retry/cache behavior
        based on a missing paragraph at a polling deadline.
        Follow-up 2026-09-22: external-src and retry fixtures now await existing
        lifecycle settlement (completed item below). The broader audit remains
        open for NPM/location failure attribution and other aggregate/frame
        predicates; the thirteen-file inventory is recorded in the
        [readiness audit](browser-stabilization-review.tmp.md#declaration-and-retry-readiness-audit).
        Follow-up 2026-09-23: focused overlapping load reproduces NPM's
        premature startup wait twice. Awaiting existing initial settlement
        before the unchanged HTTP predicates passes the same workload, both
        204-test full suites and all 160 stock probes. Location passes under
        focused load; its failure and the broader inventory remain open. See
        [NPM/location evidence](browser-stabilization-review.tmp.md#npm-and-location-readiness-under-overlapping-load).

    - [ ] Capture HTTP's newly observed 300-frame initial article-count
          timeout with declaration/render/worker tracing before choosing its
          correction. It failed once in the storage diagnostic workload;
          that run did not observe HTTP lifecycle state or verify its later
          resource assertions.
          Follow-up 2026-09-23: two traced 16-file runs and a focused five-file
          startup workload pass 26/26, 26/26 and 15/15, with 96/96 stock probes.
          The timeout remains unattributed; no wait or runtime change is made.
          Permanent opt-in tracing records lifecycle state and per-card article
          counts on startup success/failure, rethrowing the original error.
          Resume attribution on a traced recurrence. The native storage
          diagnostic follow-up below is complete; see
          [HTTP observation](browser-stabilization-review.tmp.md#http-startup-observation-without-a-reproduced-failure).

      - [ ] On a traced HTTP failure, distinguish declaration/library startup,
            current rendering and future HTTP completion before changing the
            fixture. If startup polling is premature, use existing lifecycle
            settlement before checking each authored instance; preserve the
            resource assertions and 30-second story deadline.

## Declarative UI Architecture Correction

### Native-surface delivery follow-ups

- [ ] Verify both pinned ARIA profiles in the documented browser/assistive-
      technology matrix. Record accessibility relation exposure, ordered/repeated
      details targets and error-message announcements with actual versioned
      environments; keep untested combinations explicit and the draft opt-in.
      Scenarios for later design verification: single-relation accessibility APIs
      may expose only the first target; exported DOM sequences must stay intact.

- [ ] Implement shared surface body preparation and `materialize` retain/dispose
      lifecycle before using those conveniences in migrated task declarations.
      Add fixtures first for native owner stability, preparation cancellation,
      stale completion, retained drafts and disposal after closing/exit cleanup.
      Scenarios for later design verification: invisible preparation does not
      expand launchers; default-open does not replay; disposal does not lose
      separately retained application draft state.

- [ ] Implement shared nested menu-to-task focus relay and generated/slotted
      launcher/close conveniences before claiming full interaction acceptance.
      Add native/browser fixtures first for context capture before menu closure,
      one task entry, stable launcher return, outside destination preservation
      and canceled preparation. Reuse eager `native-surface` rather than adding
      component/application-local behavior.
      Scenarios for later design verification: invocation geometry survives
      ancestor hiding without transferring context authority or replaying commands.

- [ ] Inventory and migrate legacy dialog/tooltip component APIs to XHTML/CEM-ML
      declarations using shared `native-surface`; keep frozen behavior modules
      unchanged until each owner is migrated. Add colocated browser plays and
      linked property playground/gallery cases before each cutover, including
      names, forms, modal/nonmodal visibility, interest, reference roles and cleanup.
      Scenarios for later design verification: literal local-name conveniences
      remain scoped, while cross-producer targets require fresh placement grants.

### Remaining declarative UI migration

Completed prerequisites and verification: [archived checklist](archive/todo-snapshot-2026-10-07.md#remaining-declarative-ui-migration).

- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.

    - [ ] Investigate the legacy component suite failures before claiming a
          green package gate: the cutover trial reports 80 passing / 49 failing;
          empty disabled state and collapsible navigation failures reproduce
          with committed legacy sources. Establish the remaining baseline and
          repair shared semantics or migrate owners without weakening tests.
          CSS closure comparison (2026-09-27): rebuilt commit `5f8e7172` and
          the native-CSS default both produce 81 passing / 48 failing tests,
          with the same 48 test names. No new component failures; the package
          gate remains red. Rebuild `cem-elements` before comparing this suite,
          because these tests import its packaged output.
          Follow-up 2026-09-27: the unchanged suite again reports 81 passing /
          48 failing. The [failure inventory](cem-components-baseline-2026-09-27.md)
          records each failed test, the boolean-presence mismatch, and the
          separate datepicker template-brace defect. Owner repairs remain open.

- [ ] Investigate runtime suite instability: the action-cutover full run passed
      311/312 stories; `Payload Css Readiness And Hydration` expected one style
      but observed zero. Its complete 12-story file passed on isolated rerun.
      Preserve the readiness assertion and reproduce before changing behavior.

- [ ] Migrate remaining Studio and Site visible DOM construction, UI listeners,
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final [Phases 10 and 11](../roadmap.md#phase-10---figma-ui-kit). Their
checklists live only in the roadmap. Finish and verify `cem-elements` and the
declarative `cem-components` migration first, then the remaining non-Figma phases
through Phase 9 release governance.
