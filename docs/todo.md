# Todo

This file is the authoritative checklist for active execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved under [`archive/`](archive/).

## Immediate Goal

### Native CEM-QL chain API (user-directed)

- [x] Implement typed native chains, Rust paths/closures, navigation, selection,
  extraction, scalar terminals and immutable stable ordering with Rust tests.
- [x] Document the [chain contract](cem-ql-chains.md) and simplify the Pokémon cell override.
- [x] Add the DOM functions demo fixture, dedicated Storybook subset, source
  contracts and standalone/source-loaded inventory; verify native and browser paths.

Validation: 168 focused Rust checks (including all 26 authored examples),
441 browser unit tests, 10 DOM/cell Storybook tests, and the complete gallery
(29 standalone pages / 35 source-loaded documents) pass. WASM/build, typecheck
and lint pass; lint retains two pre-existing non-null assertion warnings.
The DOM page has two cards per desktop row and no overflow at 390px or 320px.

Inventory and lock the Phase 6 CEM Site implementation boundary, then build the
root-wired documentation product from canonical repository sources. Live Figma
library and prototype updates are deferred to final Phases 10 and 11; Markdown
token specifications under `packages/cem-theme/src/lib/tokens/` remain canonical.

Phase 2.6 is complete. Its checklist is archived in
[`archive/todo-completed-2026-08-18.md`](archive/todo-completed-2026-08-18.md),
and the protected `0.1.0-rc.2` release, remote-byte verification, lane isolation,
and same-run recovery evidence are recorded in
[`cem-ml-release-rehearsal-0.1.0-rc.2.md`](cem-ml-release-rehearsal-0.1.0-rc.2.md).

## Immediate: XPath 3.1 Features for Existing CEM-QL Demos

User-directed findings and continuation, 2026-09-14. Improve the existing
samples' teaching points with explicitly owned XPath queries; retain CEM-QL
reactive bindings and CEMT/XSLT presentation. This is a staged subset, not a
claim of full XPath 3.1 conformance. The XSLT bundle remains separately approved
and tracked below; do not ask again for that bundle or the native-call hook.

Use the [XPath 3.1 language](https://www.w3.org/TR/xpath-31/) and
[Functions and Operators 3.1](https://www.w3.org/TR/xpath-functions-31/) contracts.
Keep XPath artifact identity separate from CEM-QL and preserve the standard
`fn:`, `map:` and `array:` namespaces within XPath. Do not silently reinterpret
existing `str:`/`seq:` calls, serialize runtime ASTs, change default importers,
or put table-specific logic in Rust. Keep each demo's original feature visible.

- [x] XPATH-DEMO-FINDINGS: map useful features to current sample sources and
      distinguish executable native support from syntax-only representation
      and missing browser integration. Paths/predicates, focus, control flow,
      arithmetic/ranges, primitive casts, string/data/number accessors and
      several numeric/formatting functions already execute natively. Most
      text/aggregate functions below are absent; map/array constructors and
      lookup plus dynamic function items remain non-executable. Artifact
      import alone does not bind an executable capability to a demo.
- [x] XPATH-DEMO-HOST-GATE: verify the existing authored CEMT XPath body,
      programmatic CEM-QL XPath slot, explicit native binding arena and browser
      artifact lifecycle before adding a new demo authoring surface. Reuse
      existing ownership checks and reject missing bindings; do not add an
      implicit CEM-QL ItemStream/JSON-to-XDM bridge. Record any authoring scope
      decision before implementation, independently of the approved XSLT route.
      Gate result 2026-09-14: all five CEM-QL XPath host-invocation tests and
      five CEMT XPath body/dispatch tests pass. The existing authored
      `{function ... {body {xpath ...}}}` form consumes a native binding arena;
      the CEM-QL slot is programmatic. Their schema contracts explicitly reject
      generic CEMT/CEM-QL/JSON value bridges. Ordinary browser rendering uses
      callback-free TemplateData, and the new XPath artifact registry neither
      installs callbacks nor supplies slice bindings. The current XSLT/native
      hook fixture supplies its own native node wrapper and registry; it is
      not direct demo authoring support. Stop at XPATH-DEMO-AUTHORING-DECISION
      below before changing this public integration contract. No runtime code
      or demo behavior changed for this gate.
- [x] XPATH-DEMO-AUTHORING-DECISION: decide whether ordinary CEMT/CEM-QL demos
      may declare and invoke reusable XPath-backed functions directly, using
      the existing CEMT XPath body as the starting point, or whether XPath
      remains accessible only through the approved XSLT bundle in this scope.
      Direct use needs an explicit scalar/native-owner binding and result
      contract plus companion-program loading; it is not supplied by the
      current CEM-QL slot API or by importing an XPath artifact. Keep default
      callback registries, generic CEMT artifacts and JSON bindings unchanged.
      Recommended: support explicit named XPath-backed functions for the
      existing CEM-QL/CEMT demos, reusing the authored CEMT XPath body and a
      separately validated companion program. Specify declared scalar/native
      bindings without an implicit generic-value/JSON-to-XDM conversion. The
      alternative is to keep this continuation on the approved XSLT-only
      delivery path and defer direct demo function invocation.
      Approved 2026-09-14: user selected direct named XPath-backed functions
      (option 1), and requested consideration of matching in CEM-QL. Reuse the
      existing authored body and keep scalar/native bindings explicit. Do not
      ask again for this authoring choice or the previously approved loaders.
- [x] Fixture XPATH-DEMO-NAMED-NATIVE: compile the existing authored XPath
      function declarations into explicitly installed CEM-QL native functions.
      Accept only declared scalar types or retained native XPath items; reject
      record/array/JSON shape inference, undeclared bindings and unsupported
      defaults/types. Test named invocation from CEM-QL and CEMT, changed
      arguments/owners, exact result types, source maps, limits and isolation.
      Keep this native fixture separate from pending browser companion loading
      and direct QName-call authoring; native:call is the existing explicit
      invocation boundary, not a claim that that later sugar is implemented.
      Completed 2026-09-14: `CemtXPathFunctions` compiles public authored bodies,
      reloads XPath-owned binary programs without source text/tokens, and
      installs named functions atomically into an explicit registry. The eight
      native fixtures in `packages/cem_ql/tests/xpath_named_functions.rs` pass,
      alongside all 11 native-hook and five existing XPath host-slot tests.
      They cover scalar/nullable/exact-decimal bindings, changed native owners,
      filtering/CEMT rules, result/cardinality rejection, source maps, private
      exports, import/declaration rejection, conflicts, IR reload, unsupported
      capabilities, cancellation and item budgets. Default importers/registries
      and low-level native-XDM adapters are unchanged. See the CEM-QL README
      for the deliberately closed host type contract and remaining limitations.
      Verification: all 316 CEM-QL tests pass through the uncached Nx test
      target, plus five existing CEMT XPath-dispatch tests. Sequential uncached
      Nx lint (existing warnings) and WASM build pass; `git diff --check` passes.
      Browser execution remains unimplemented and is not claimed by this gate.
- [x] XPATH-DEMO-MATCHING-DESIGN: expose reusable boolean XPath predicates to
      CEM-QL filtering and CEMT match conditions through the same named function
      contract. Require an actual singleton xs:boolean, not CEM-QL truthiness
      of a result sequence. Assess reuse without moving priorities, modes,
      imports or rendering into the query layer. XSLT match patterns select a
      candidate by pattern semantics, not boolean evaluation of an arbitrary
      path at that candidate; regex fn:matches is a separate text operation.
      The user requested this assessment, not an implicit full pattern engine.
      Assessment 2026-09-14: reuse named predicates through `native:call` in
      `seq:where` and CEMT `@match`; no new generic matching DSL is needed for
      this case. The native fixture proves the same predicate serves both.
      If authored XSLT patterns are later exposed to queries, compile them in
      the XSLT layer and expose candidate-to-boolean evaluation through this
      same explicit capability boundary; never evaluate an arbitrary relative
      path as though it were a candidate match. Rule selection/priority/mode
      resolution remains presentation-layer work, not a query-side registry.
- [x] Fixture XPATH-DEMO-COMPANION: persist the named-function binding contract
      with opaque XPath-owned program artifacts in a versioned CEMT companion.
      Validate hashes, source/host identities, binding names/types, source maps,
      counts and byte limits before retaining anything. Prove deterministic
      native reload and changed-input query/CEMT matching without source parsing
      or runtime AST serialization; keep generic CEMT artifacts unchanged.
      Completed 2026-09-14: `to_companion_bytes` / `from_companion_bytes` use
      `application/vnd.cem.cemt-xpath-functions+cem-bin`, version
      `cemt-xpath-functions/1`, with a bounded explicit JSON control manifest
      and opaque, independently identified XPath binaries. No XPath program or
      runtime data AST is converted to JSON. Five native integration fixtures
      cover deterministic reload, preserved namespaces/native XML owners,
      altered metadata, hashes, limits, explicit registration and stale handles;
      a host unit fixture covers retained-byte and handle-exhaustion limits.
- [x] Fixture XPATH-DEMO-COMPANION-WASM: add explicit compile/import/dispose
      companion entrypoints and a render call that selects its companion handle
      separately from data. Test native/WASM byte parity, compile-once/render-many,
      missing/stale handles, bounds and default/JSON capability isolation.
      Do not claim component demo wiring from a WASM API-only fixture.
      Completed 2026-09-14: `compileCemtXPathFunctions`,
      `retainCemtXPathFunctions`, `importCemtXPathFunctions`,
      `disposeCemtXPathFunctions` and `renderTemplateWithXPathFunctions` keep
      companion handles separate from template handles and data. Source-loaded
      retention returns compiler-generated hashes without putting programs in
      JSON. All 22 checks in `tools/scripts/verify-xpath-function-companions.mjs`
      pass, including source/binary loading, native/WASM byte equality, changed
      scalar input and shared predicates after portable-template reload.
      All 322 CEM-QL tests and sequential uncached Nx lint (existing warnings)
      and WASM build pass. The component loader, DOM demo and browser native-node
      binding path are not implemented by this API-only slice.
- [x] XPATH-DEMO-BROWSER-ATTACHMENT-DECISION: choose how a component template
      declares its companion dependency before changing browser source loading,
      module-closure cache keys, worker transport and disposal.
      Approved 2026-09-16: the user selected a separately referenced XPath
      function library with its own dependency lifecycle. Implement an explicit
      `xpath-functions` reference on the CEM-ML template, resolved through the
      declaration's module URL context. Keep library identity and retention
      separate from template artifacts and data; do not auto-enable functions
      declared in the template itself. Do not ask again for this layout choice,
      the direct-function contract, the native hook, or the XSLT bundle.
      Continuation review: the existing 13 named-function/companion Rust tests
      and 22 native/WASM companion checks pass. The library compiler rejects
      imports; resolved companion closures remain separate work. Current WASM
      rendering accepts declared scalars through JSON data only; browser native-
      owner transport is still needed for the full INVOKE fixture below.
- [x] Fixture XPATH-DEMO-LIBRARY-BROWSER: implement the approved external
      function-library reference, scoped resolution, bounded loading, independent
      source/hash/cache identity, worker/fallback retention and disposal. Prove
      shared reuse, changed scalar slices, failed loads/compiles, stale handles
      and capability isolation using native tests first, then a standalone and
      source-loaded interactive demo, source guards, inventory, lint/typecheck.
      Keep browser native-owner transport tracked in XPATH-DEMO-INVOKE.
      Include the existing data-table source-viewer card in the source-contract
      and browser inventories; its previously omitted legend blocks these gates.
      Check both gallery layouts; source-loaded document hosts must shrink
      within the demo page's flex container rather than overflow the viewport.
      Completed 2026-09-16: `xpath-functions="./library.cemt"` resolves an
      independent library snapshot through the scoped loader. Its worker/fallback
      companion cache validates source/URI/policy/compiler identity, shares
      references across templates, and releases on eviction, compile failure and
      scope disposal, including concurrent/pending work. Ordinary data cannot
      select the handle. Packaged hosts share one WASM instance for retention
      and rendering. The new `xpath-functions.html` demo covers changed string
      slices, focus/caret preservation and boolean CEMT matching.
      Verification: 14 native tests, 22 native/WASM checks, 360 component unit
      tests, all 157 Storybook tests, and the 19-page/25-document standalone/
      source-loaded inventory pass. Nx build, typecheck and lint pass; both
      1440px gallery layouts fit two cards without horizontal overflow.
      Library imports and browser native-node transport remain unsupported;
      the full INVOKE fixture is deliberately still open.
- [x] XPATH-DEMO-NODE-BINDING-REVIEW: inspect the reader, named-function
      boundary and browser transport before implementing native-node inputs.
      Reviewed 2026-09-16: all 14 named-function/companion Rust tests pass,
      including explicit rejection of `data:read("<r/>", "xml").root` as an
      XPath argument. The reader retains its parsed XML beside a CEM-tree view,
      but that view is not an `XPathQueryItem`; each read parses its source.
      Browser rendering supplies JSON data and a separate function companion,
      with no retained-document binding channel. No runtime behavior changed.
- [x] XPATH-DEMO-NODE-BINDING-DECISION: choose the public XML input route
      before implementing XPATH-DEMO-INVOKE. Either add an explicit native
      XPath view to `cem-data` / `data:read`, preserving the default CEM-tree
      view, or introduce separately retained XML documents with explicit host
      bindings and their own loading/replacement/disposal lifecycle.
      Recommended: separate retained documents, so unchanged inputs can keep
      their owner across renders. A reader-based choice must also specify
      retention for that reuse. Neither route permits generic JSON/record
      inference or DOM serialization as an AST handoff. This decision concerns
      node inputs only; the approved separate function library remains settled.
      Approved 2026-09-16: user selected the explicit XPath view on `cem-data`.
      Implement `@projection=xpath` / `data:read(source, "xml", "xpath")`,
      retaining the XML owner inside Rust and preserving the default CEM view.
      Use bounded reader retention across renders, released with the retained
      template; do not introduce separately authored document dependencies.
- [x] Fixture XPATH-DEMO-READER-VIEW: test and implement the explicit XML
      XPath reader view, unchanged-source owner reuse, changed-source isolation,
      source maps, namespace selection, malformed/non-XML/oversize rejection,
      bounded retention and disposal. Prove native CEMT rendering first, then
      WASM artifact reload and worker/fallback rendering. Add one interactive
      XML demo with matching, edit/error recovery and focus preservation, and
      update source guards, both inventories and reader documentation.
      Completed 2026-09-16: `projection=xpath` exposes a native XPath document
      through the existing reader report. `DataReaderCache` retains up to 16
      successful bounded XML reads by exact source/format; evaluation/render
      contexts share it explicitly. WASM retains it with the template, including
      imported artifacts, and releases it on template disposal. XML owners stay
      in Rust; neither JSON records nor default CEM views become XPath nodes.
      Six native reader fixtures prove namespaces/source maps, rejection/limits,
      unchanged and changed owners, cross-render reuse, eviction/clear/disposal
      and CEMT matching. The XML demo covers worker/fallback host isolation,
      malformed-input recovery, focus/caret and source-loaded execution.
      Verification: full CEM-QL suite and native adapter regression pass;
      all 37 native/WASM companion checks, 360 component unit tests, 157 browser
      tests and the 19-page/25-document fixture inventory pass. Nx builds,
      typecheck and lint pass (existing Rust warnings); both 1440px gallery
      layouts fit two cards without horizontal overflow. `git diff --check`
      passes. Library imports and non-XML XPath reader views remain unsupported.
- [x] Fixture XPATH-DEMO-INVOKE: finish the native-owner browser route after
      the verified external-library/scalar route above. Use an already
      executable XPath expression and explicit retained-owner transport.
      Prove compile-once/render-many across changed slices/native owners,
      correct host/namespace selection, result types and source maps, bounded
      work, missing-capability rejection, import/disposal and no source reparse.
      Verify native execution first, then WASM and a standalone/source-loaded
      demo. Do not wait for new functions to discover binding-contract gaps.
      Completed 2026-09-16 through the separate function library and explicit
      XML reader view above. The browser supplies source to the authored reader;
      native owners remain retained inside Rust. Existing node selection and
      boolean predicates now execute across changed slices and XML inputs.
      Compiled programs and cached XML are reused without source reparse;
      portable import, missing capabilities and disposal are verified separately
      from render data. No additional node transport or JSON-to-XDM bridge is
      required for the selected reader route.
- [x] XPATH-DEMO-TEXT-LIMIT-REVIEW: review standard signatures, native
      string conversion and runtime limits before the text-function fixture.
      Reviewed 2026-09-17: XPath 3.1 defines XML-whitespace normalization and
      one-argument tokenization, codepoint string length, and ordered joining
      of atomized values. The four functions are not yet in native dispatch.
      `XPathEvaluationLimits` currently bounds sequence items only; safe-point
      polling checks operation control, not a cumulative text-work counter.
      Existing node string values, atomization and concatenation do not charge
      text bytes. The shared operation controller has explicit memory permits,
      but XPath string construction does not use them. A one-item string result
      is therefore not byte-bounded by `xpathItems`. This limitation is already
      documented in the CEM-QL named-function contract.
      Verification: 38 native XPath tests, the controlled-range cancellation
      test and the named-function uncatchable-limit test pass. No runtime or
      demo behavior changed. Standard contracts:
      https://www.w3.org/TR/xpath-functions-31/#func-string-join and the adjacent
      string-length/normalize-space and tokenize sections.
- [x] XPATH-DEMO-TEXT-LIMIT-DECISION: choose the scope of the text-work and
      output-byte limits required by XPATH-DEMO-TEXT before implementation.
      Recommended: shared XPath evaluator limits, covering new functions and
      existing string-producing paths such as node atomization, `fn:string`,
      concatenation, casts and formatting, with host propagation, policy
      identity and uncatchable limit diagnostics. Alternatively, bound only
      the four new text functions and document that existing string operations
      keep their current resource contract. The latter is a narrower feature,
      not an evaluator-wide byte limit. Do not reinterpret sequence-item counts
      as byte counts or change CEM-QL Unicode whitespace semantics.
      Approved 2026-09-17: user selected shared evaluator limits (option 1),
      including existing string-producing paths and native node atomization.
- [x] Fixture XPATH-DEMO-TEXT-BUDGET: add native and WASM regressions for shared
      text-byte/work bounds, nested calls and intermediate results, original
      source ranges, cancellation, host propagation and uncatchable failures.
      Cover literals/bindings, node atomization, concatenation, casts and
      numeric formatting before adding the four new text functions.
      Completed 2026-09-17: shared `XPathEvaluationLimits` default to 1 MiB
      of UTF-8 atomic lexical bytes per materialized value/sequence and
      16,777,216 work units per invocation. Nested expressions use one counter;
      native XML string extraction walks the retained event owner under the
      same bounds. Item counts remain independent. Standalone/query scope
      budgets and `install_with_limits` propagate host-owned limits; result
      stamps record them. Castable expressions and CEM-QL try/catch cannot
      consume resource failures as data. This bounds lexical text and counted
      work, not total heap allocation or elapsed CPU time.
      Verification: seven dedicated native text/budget tests, 86 XPath unit
      tests, eleven XML-view tests, nine CSS query regressions, normalized
      run-config coverage, the full CEM-QL suite and 43 native/WASM companion
      checks pass. The latter includes default byte/work failures through
      retained libraries and CEM-QL try/catch. Rust lint passes with existing
      warnings.
- [x] Fixture XPATH-DEMO-TEXT: implement `fn:normalize-space`,
      `fn:string-length`, `fn:string-join` and one-argument `fn:tokenize` with
      typed arguments, exact source-located diagnostics and work/output limits.
      Then add XPath comparison cases to `dom-merge.html` and related string
      samples: repeated words, tabs/newlines, blank input, emoji/codepoints and
      focus-preserving updates. `count(tokenize($text))` is the word-count
      candidate. Preserve the distinction between XML whitespace and the
      existing CEM-QL Unicode-whitespace normalizer; do not change its behavior.
      Unsupported regex tokenize arities must fail explicitly until supported.
      Completed 2026-09-17: all four functions execute natively with the
      standard supported arities, XML-whitespace/codepoint rules, atomic
      joining, argument-range errors and shared limits. `xpath-text.cemt`
      supplies the separately loaded library for one additional DOM-merge
      counter card and two string-function comparison cards. Original CEM-QL
      behavior is preserved; an incidental visible closing brace in the input
      demo was removed. Source contracts, ordered fixture inventories, standard
      function documentation and the conformance gap description are updated;
      existing Nx `demo/*.cemt` inputs cover the new library.
      Verification: native and 43 native/WASM checks above, 363 component unit
      tests, 157 Storybook tests, all 19 standalone/25 source-loaded fixture
      documents, Nx builds, lint and typecheck pass. Native CEMT/XSLT hook
      regression also passes. Both updated galleries at 1440px fit two 704px
      cards in a row without horizontal overflow, standalone and source-loaded.
- [x] XPATH-DEMO-NODES-HTTP-REVIEW: inspect the next node fixture's input
      boundary before adding XPath over HTTP resources.
      Reviewed 2026-09-17: local XML already has the approved explicit
      `cem-data @projection=xpath` view. The browser HTTP path still buffers
      response bytes, uses DOMParser for XML, and publishes JavaScript records
      (`parseHttpResourceData` / `parseXmlHttpResourceData` in
      `cem-elements.ts`). It does not retain or expose an XPath XML owner.
      Those records cannot be passed as native XPath nodes, and rebuilding XML
      from them would violate the existing ownership boundary. No HTTP runtime
      behavior changed during this review.
- [x] XPATH-DEMO-NODES-HTTP-DECISION: choose the sequencing of the next node
      fixture. Recommended: complete native node accessors and local XML
      table/tree comparisons first, with HTTP integration explicitly deferred
      to its native resource-owner work. Alternatively, implement the native
      HTTP XML input/worker ownership and disposal path now before completing
      the combined node fixture. This is an HTTP resource lifecycle scope
      decision, not a reconsideration of the approved external function
      library or explicit XPath reader view. Approved 2026-09-17: option 1,
      complete the local XML table/tree slice first and defer HTTP integration.
      Subsequent direction is recorded under CEM-LOADER-DIRECTION below.
- [x] Fixture XPATH-DEMO-NODES: add missing `fn:local-name` and
      `fn:namespace-uri`; reuse native paths, predicates, `fn:string` and
      `fn:data` in local XML table/tree cases. Cover equal local names
      in different namespaces, attributes, mixed text/CDATA, parent/sibling
      navigation and selected-node identity after sorting. Add `fn:node-name`
      only with the required QName value/comparison contract. Non-XML owner
      support is a separate explicit view/binding task, not shape inference.
      Completed 2026-09-17: both accessors support zero/one arguments with
      native node-only typing, `xs:string`/`xs:anyURI` results, argument source
      ranges and shared text/work limits. Names come directly from retained
      XML metadata; detached handles and non-node values fail explicitly.
      `fn:node-name` and QName values/comparisons remain unsupported.
      `xpath-nodes.html` adds two anonymous table/tree cases using the separate
      `xpath-nodes.cemt` library. Rust verifies exact native owner/handle identity
      after CEM-QL sorting; the table selects unique authored IDs and preserves
      source parent/sibling navigation when reordered. The tree covers
      namespaces, attributes and coalesced text/CDATA. Browser checks cover
      live edits, focus/caret, instance isolation and reader error recovery.
      Updated related links, documentation, Nx inputs and fixture inventories.
      Verification: four accessor tests, 86 XPath unit regressions, seven text
      and 11 XML-view tests, full CEM-QL tests, 47 native/WASM checks, 365
      component unit tests, 158 Storybook tests, all 20 standalone/26
      source-loaded fixture documents, Nx builds, lint and typecheck pass.
      Both gallery modes fit two 704px cards at 1440px without overflow.
- [x] Fixture XPATH-DEMO-SEQUENCES: add the viewer-needed subset of
      `fn:distinct-values`, `fn:head`, `fn:tail`, `fn:subsequence` and
      `fn:reverse`, with empty/duplicate/type/source-map/budget cases. Derive
      table headings and cell values declaratively. Preserve first-seen heading
      order explicitly: standard distinct-values ordering is implementation-
      dependent, and map iteration must not silently define display order.
      Completed 2026-09-17: native head/tail/reverse/subsequence preserve
      original items, source maps and retained node owners, including opaque
      array/map/function items. Subsequence uses typed double bounds and
      standard one-based rounding/NaN/infinity behavior. Distinct-values
      atomizes nodes/arrays, supports the existing atomic comparison matrix,
      collapses NaNs and uses only the Unicode codepoint collation; duplicate
      scans share the work budget. Unsupported types/collations fail explicitly.
      `xpath-sequences.html` and its separate function library demonstrate
      editable word windows and XML columns derived in explicit first-seen
      order, including later attributes, namespaces, repeated/empty/missing
      cells, invalid XML recovery and focus/caret retention. Updated links,
      source guards, fixture inventories, Nx inputs and conformance notes.
      Verification: six sequence tests, 86 XPath unit regressions, four
      accessor/seven text/11 XML-view tests, full CEM-QL tests, 53 native/WASM
      checks, 367 component unit tests, 159 Storybook tests, all 21 standalone
      and 27 source-loaded fixture documents, builds, lint and typecheck pass.
      Both gallery modes fit two 704px cards at 1440px without page overflow.
- [x] Fixture XPATH-DEMO-AGGREGATES: implement the needed numeric slices of
      `fn:sum`, `fn:min`, `fn:max` and `fn:avg`; test empty input, exact decimal
      values, untyped conversion, mixed/invalid values and limits. Add a live
      basket/table case where a newly introduced fruit contributes to the
      total without a new hard-coded field expression. Do not turn invalid
      data into zero implicitly or hide unsupported standard argument types.
      Completed 2026-09-17: sum/avg/min/max atomize native nodes/arrays and
      promote the complete numeric sequence before reduction. Integer/decimal
      sums remain exact; averages reuse terminating-exact and repeating
      18-significant-digit half-even division. Empty sums honor the optional
      atomic/empty fallback; empty extrema/averages remain empty. Invalid
      untyped values and incompatible types fail explicitly, including after
      NaN; nonnumeric extrema and duration arithmetic remain unsupported.
      Added input, promotion, intermediate/result and reduction/division work
      checks. Fixed the shared decimal magnitude comparison against zero,
      which incorrectly ordered positive fractions below zero.
      `xpath-aggregates.html` and its separate library demonstrate decimal-list
      statistics and a native XML basket that includes newly added fruits.
      Authored XPath validates before casting; invalid/missing amounts display
      errors. Tests cover empty recovery, exact totals, focus/caret, isolation,
      retained native row owners and compiled/reloaded companion programs.
      Updated related links, source guards, inventories, Nx inputs and docs.
      Verification: six aggregate tests, 86 XPath unit regressions, 28 related
      sequence/accessor/text/XML-view tests, full CEM-QL tests, 69 native/WASM
      checks, 369 component unit tests, 160 Storybook tests, all 22 standalone
      and 28 source-loaded fixture documents, builds, lint and typecheck pass.
      Both gallery modes fit two 704px cards at 1440px without page overflow.
- [x] XPATH-DEMO-MAPS-ARRAYS-INPUT-REVIEW: inspect the JSON input boundary
      before the maps/arrays fixture. Reviewed 2026-09-17: native XPath map
      and array values already have opaque CEM-QL wrappers; constructors can
      be implemented using declared scalars and retained XML nodes. The existing
      `json-to-xml` reader projection retains the JSON owner and produces a
      CEM-tree view; that view is not an XPath item. `projection=xpath` accepts
      XML only, and XPath native node handles address XML owners only. The
      JSON/IP-filter examples therefore cannot pass their current reader roots
      directly into XPath. No runtime behavior changed. The approved HTTP
      native-owner deferral remains in force. Verification: all six native
      reader tests and sixteen named-function tests pass.
- [x] XPATH-DEMO-MAPS-ARRAYS-INPUT-DECISION: choose the sequencing of JSON
      integration before implementing the combined fixture below. Recommended:
      implement constructors, lookup and the listed map/array functions first
      with explicit scalar inputs and retained XML basket nodes; demonstrate
      empty-valued versus absent map entries without claiming JSON parsing,
      and track native JSON integration separately. Alternatively, add a native
      XPath view of the existing standard JSON-to-XML projection in this slice,
      retaining its JSON owner and source maps without serialization/reparse;
      specify that reader/view contract before implementation. Neither choice
      changes the existing CEM-tree projection or reopens deferred HTTP work.
      Approved 2026-09-17: option 1, complete maps/arrays using explicit scalar
      inputs and retained XML nodes first; defer native JSON integration.
- [x] Fixture XPATH-DEMO-MAPS-ARRAYS: implement constructors and lookup, then
      the required `map:contains`, `map:get`, `map:keys`, `array:size` and
      `array:get` operations. Cover nested/empty members, one-based array
      access, duplicates, key typing, absent versus empty-valued entries and
      native-owner retention. Use scalar IP-filter inputs and XML basket nodes;
      map:contains must distinguish an absent entry from an empty-valued entry.
      Do not flatten array members through CEM-QL sequences. JSON input/null
      mapping and HTTP examples were deferred at this stage; subsequent delivery
      and loader migration are tracked in the shared-tree and CEM-LOADER items below.
      Include native regressions for CEMT XPath expressions inside the existing
      rich-content fences: braces must stay literal and source ranges must
      point into the fence body, including after compiled-program reload.
      Completed 2026-09-17: native constructors, unary/postfix/wildcard lookup,
      single-key container calls and all five functions execute with exact
      supported atomic key equality, duplicate-key errors, preserved empty and
      nested members, source maps and retained XML owners. Empty member visits,
      container copies and key comparisons charge shared work/text limits.
      Typed lookup keys use program format v2; v1 programs require recompilation.
      Fixed CEMT rich-content fence ranges so constructor braces remain literal
      and diagnostics retain original body coordinates after artifact reload.
      Named functions accept opaque map/array results without CEM-QL flattening.
      Added separate IP-filter scalar-map and XML basket-array demos, with source
      guards, interactions, inventories, related links, Nx inputs and docs.
      Verification: seven new native groups, 40 related native fixtures, 86 XPath
      unit tests, five CEMT hook tests, full CEM-QL tests (19 named-function cases),
      85 native/WASM companion checks, 15 artifact checks including typed-lookup
      byte parity, 371 component unit tests, 161 Storybook tests, all 23 standalone
      and 29 source-loaded documents, builds, lint and typecheck pass. Both gallery
      modes fit two 704px cards at 1440px without page overflow. JSON and HTTP
      integration remain explicitly deferred below.
- [x] XPATH-CEM-TREE-DIRECTION: reconsider the JSON deferral as a shared
      CEM-tree capability. User direction 2026-09-17: consume the tree after
      import with the same XPath machinery as XML; confine JSON-specific code
      to import, using an XML-shaped mapping if needed. Review confirms both
      imports already produce `CemDocument`; the current XPath view instead
      addresses XML parser events. The existing standard JSON-to-XML importer
      builds CEM nodes directly, so no serialization/reparse or native JSON
      map bridge is needed. Recorded the design and parity risks in
      [cem-tree-xpath-view.md](cem-tree-xpath-view.md). Runtime is unchanged.
      This direction supersedes the JSON-specific reader/view decision below;
      keep existing import vocabularies and the separate HTTP lifecycle task.
- [x] Fixture XPATH-CEM-TREE-PARITY: before resuming the demo feature sequence,
      add native tests for one retained typed-CEM-tree XPath entry point used
      by XML and both existing JSON import projections. Assert paths, predicates,
      values, source provenance, stable identity, retained owners and unchanged
      default CEM views. Audit XML semantic values before text/reference merging;
      cover text/CDATA/entity runs, literal versus referenced CR, attributes,
      namespaces, declarations and PIs. Cover JSON null/absent/empty members,
      arbitrary/duplicate keys, array order and escaping within import. Stop
      for a concrete unresolved semantic/API decision, not to reapprove the
      shared-tree direction. Use the design note's parity cases as the baseline.
      Completed 2026-09-17: nine shared-tree fixtures cover XML, both JSON
      mappings, YAML and CSV, semantic/source parity, lifecycle coordinates,
      retained owners, limits, malformed graphs and parser-error rejection.
      All 11 existing XML-view fixtures pass; source-oriented CEM fields remain
      unchanged. The full 345-test CEM-QL suite includes five shared-tree binding,
      compiled-reload, cache/owner and authored-demo cases.
- [x] CEM-IMPORT-BOUNDARY: enforce the user's 2026-09-17 format-wide rule:
      external data formats resolve only in CEM AST import. Audit XML, JSON,
      YAML, CSV and browser external-data paths; move parser/decoding branches
      out of query/evaluation code into shared import. Document the rule in
      CLAUDE.md and a normative principle document. Add cross-format native
      fixtures and a boundary guard covering downstream query code; distinguish
      explicit API/control serialization from imported document data. Record
      existing public-binding migrations and the accepted replacement contract.
      Implemented 2026-09-17 for the data reader and XPath: shared `cem_ml::import`
      owns all four formats and semantic decoding, with one downstream node
      view and a passing source-boundary guard. The rule is normative in
      [cem-data-import-principle.md](cem-data-import-principle.md) and CLAUDE.md.
      The broader HTTP and lifecycle-query binding gaps are now migrated under
      CEM-LOADER and CEM-IMPORT-LEGACY-VIEWS below. Legacy standalone HTTP and
      its unused theme copy are retired; explicit control/export protocols remain
      separately named boundaries. Native guards reject format parsing in consumers.
- [x] XPATH-CEM-TREE-VIEW: implement the shared retained CEM-tree capability
      and XPath node view in `cem-ml`, then accept native imported CEM nodes at
      the named-function boundary. Keep JSON mapping entirely in import and
      move XML entry points onto the same view after parity passes. Preserve
      source-oriented trees through import-supplied semantic metadata where
      needed; keep navigation, atomization, ordering, identity and bounded
      traversal format-independent. Reuse the existing cache/owner lifecycle
      without record inference or per-invocation reconstruction of documents.
      Completed 2026-09-17: `RetainedCemTree` preserves source CEM nodes and
      decoded semantic metadata; ordinary imported roots bind directly to
      named XPath functions. XML compatibility constructors and XPath lifecycle
      owners delegate to the same import/view. The 16-entry reader cache now
      covers all formats by exact source, format and projection. Rust callers
      use `owner()` for the retained CEM tree and `source_owner()` for an optional
      original lifecycle owner; native CEM producers need no external owner.
- [x] Fixture XPATH-CEM-TREE-INTEGRATION: after native parity, verify compiled
      function reload, retained nodes inside maps/arrays, source maps, limits,
      cache replacement/eviction/disposal, WASM and browser wiring. Add an
      imported-JSON XPath comparison to the maps/arrays or IP-filter demo using
      ordinary imported CEM roots and the existing explicit mapping. Keep
      current XML demos working through the same query view; document the new
      capability without claiming standard JSON parsing functions are complete.
      Completed 2026-09-17: the third maps/arrays demo imports JSON directly
      into CEM and passes retained nodes through the existing XPath library;
      the XML basket now uses the ordinary CEM projection. Native fixtures
      cover both JSON mappings, YAML and CSV through the same reloaded function.
      Verification: 345 CEM-QL tests, 86 XPath unit tests, focused native tree,
      XML, text, node, sequence, aggregate and container regressions, five CEMT
      tests and the native adapter check pass. The final WASM build passes;
      97 companion and 15 artifact checks verify native/WASM parity and reload.
      All 371 browser unit and 161 Storybook tests pass, as do the 23 standalone
      and 29 source-loaded demo fixtures, lint (existing Rust warnings) and
      TypeScript typecheck. The boundary guard and its cross-package Nx cache
      inputs pass verification. Both loading modes fit two cards in a desktop
      row without page overflow. `git diff --check` passes. Legacy HTTP and
      lifecycle query bindings remain the separately recorded migration below.
- [x] CEM-LOADER-DIRECTION: user clarified the replacement on 2026-09-17.
      A loader custom element uses the `cem-ml` library to load XML and JSON
      requests into a retained CEM-ML document/AST tree. Request lifecycle
      behavior follows legacy `http-request`; loaded data is a CEM tree.
      Runtime code, samples, fixture adapters and worker messages must not
      materialize loaded documents as JavaScript JSON objects. This resolves
      the earlier keep-records/defer-versus-migrate decision. The accepted
      implementation plan is [cem-data-loader-plan.md](cem-data-loader-plan.md).
      Both HTTP binding/lifecycle and lifecycle-query migrations were explicitly
      authorized later on 2026-09-17; implementation and verification follow below.
- [x] Fixture CEM-LOADER-NATIVE: reuse CEM-ML resource loading, lifecycle and
      CEM AST import for XML/JSON. Add failing native fixtures first for loaded
      trees, equivalent queries, source ranges/identity, limits and parse errors.
      Define the loader tag, request attributes, lifecycle notifications and
      native document binding before browser wiring. Extend library capability
      where missing; never add browser format parsers or object projections.
- [x] Fixture CEM-LOADER-BROWSER: wire the loader custom element to the native
      library/WASM capability. Preserve request/response metadata, lifecycle
      updates, reload, abort on replacement/disconnect, stale-result rejection
      and instance isolation. Retain native tree owners through worker/fallback
      render paths and release references on replacement/disposal. First delivery
      publishes a complete materialized tree; byte delivery is not AST streaming.
      Completed: protocol v2 retains native documents and explicit render bindings.
      Worker/fallback, cancellation, shared-root disposal, instance isolation,
      disconnect/reconnect and versioned serialized hydration have executable coverage.
- [x] Fixture CEM-LOADER-SAMPLES: migrate HTTP, `for-each` and legacy companion
      examples, their consumers and fixture adapters to loaded CEM tree queries.
      Remove JS record bindings and browser JSON/XML document parsing from these
      paths; any retained legacy element spelling delegates to the same loader.
      Cover XML/JSON, metadata, error/recovery, changed URLs, cancellation and
      independent instances. Update inventories and source-loaded/standalone tests.
      Completed: HTTP, loop and version-picker examples use native CEM nodes and
      a separate XPath library. All 161 Chromium stories and the 23-page/29-document
      demo gate pass; HTTP/version-picker layouts fit two desktop cards without overflow.
- [x] CEM-IMPORT-LEGACY-VIEWS: migrate
      `cem_ml_transform_cem_ql::lifecycle_query_stream`, its JSON-member/XML-event
      views and encoded-document inputs to common CEM import and node capabilities.
      Update native queries/fixtures rather than preserving a second record/event
      data model. Keep explicit source inspection and export boundaries named.
      Native migration implemented: all four formats share retained CEM node
      views; encoded inputs delegate byte parsing to import. Native adapter
      consumers and DOM converter fixtures now query/pass CEM nodes. Native
      byte-owner, XPath equivalence, explicit binding and release tests pass.
- [x] Fixture CEM-LOADER-RETIREMENT: retire the standalone legacy HTTP module,
      exports, IDE declarations and unused theme copy. Migrate its examples to
      native `cem-elements`, verify source and packed examples, and retain the
      historical HTTP corpus with evidence from the shared loader tests.
      Completed: all 88 historical browser and three unit cases remain inventoried.
      The 167-file packed archive passes clean-consumer JavaScript/type/browser
      checks. Source/dist examples and incremental theme-output retirement pass.
      Package verification caches now include dependency output files.
- [x] Fixture CEM-LOADER-VERIFY: verify native/WASM parity, worker/fallback
      lifecycle and ownership, CEM-QL/XPath access, standalone/source-loaded samples,
      layout, source guards, lint and typecheck. Extend the import-boundary guard
      and Nx inputs to migrated loader/query modules. Audit remaining document-
      object bypasses before closing CEM-IMPORT-BOUNDARY.
      Completed: 347 CEM-QL tests, 95 adapter tests and 25 focused CEM-ML tests;
      378 unit/WASM tests and all 161 Chromium stories; 23 standalone pages and
      29 source-loaded documents. Native/TypeScript lint, typecheck, package
      gates, import-boundary guards and layout checks pass. Streaming stays in
      the explicitly recorded later phase below.
- [x] CEM-LOADER-STREAMING-RECORDED: persist the accepted later phase for
      progressive CEM-ML AST stream consumption in
      [the loader plan](cem-data-loader-plan.md#later-cem-ml-ast-streaming) and
      [wishlist.md](wishlist.md#cem-ml-runtime). Typed chunks/events, backpressure,
      bounded retention, partial errors, cancellation, stable identity and query
      streamability require their own fixtures after materialized-loader parity.
- [x] Fixture XPATH-DEMO-SORT: implement the required function-item/inline-
      function slice before keyed `fn:sort`; verify stable multi-key ordering,
      typed comparisons, supported collations and retained source selection.
      Explicitly author missing/invalid-last keys in the viewer; standard sort
      is not an alias for the existing `seq:sorted` policy. Keep XSLT grouping,
      xsl:sort options and presentation dispatch in their owning layer.
    - [x] Fixture XPATH-SORT-FUNCTIONS: add native inline-function closures,
          lexical capture, typed invocation and bounded dynamic calls; preserve
          retained CEM owners and source maps through compiled-program reload.
          Keep executable closures inside XPath and retain the existing CEM-QL
          rejection of function-valued arguments/results.
          Completed: lexical capture/shadowing, typed conversion, absent focus,
          expanded parameter names, retained owners, defining-source diagnostics
          and bounded recursive calls pass native and WASM checks. Portable
          programs use v3; v1/v2 programs require recompilation.
    - [x] Fixture XPATH-SORT-NATIVE: implement stable `fn:sort` with atomized
          lexicographic keys, supported typed comparisons and codepoint
          collation; verify errors, limits, cancellation and source identity.
          Completed: stable typed/multi-key sorting, callable map/array keys,
          empty/NaN ordering, explicit invalid-last policies and retained node
          identities pass native fixtures. The external-import source guard
          includes the new sorting and inline-function modules.
    - [x] Fixture XPATH-SORT-DEMO: teach authored validity and direction keys
          with retained source selection, native/WASM parity and compact
          standalone/source-loaded browser coverage.
          Completed 2026-09-17: two anonymous samples use a separate XPath
          library, preserve selected-row source navigation, and cover edits,
          malformed input, recovery and focus. Both loading modes fit two
          704px cards at 1440px without page overflow.
      Verification: 350 CEM-QL tests, 86 XPath unit tests and 32 focused native
      cases; 103 companion and 17 artifact native/WASM checks; 380 browser-host
      unit tests, all 162 Chromium stories, 24 standalone pages and 30 source-
      loaded documents. Native/TypeScript lint, typecheck, source guards and
      layout checks pass. The first broad browser run hit the existing
      data-slices startup timeout; all stories passed on the final rerun.
- [x] XPATH-REGEX-SCOPE-DECISION: clarify the initial regex contract before
      XPATH-DEMO-VALIDATION. The current Rust `regex` engine excludes
      backreferences, while XPath F&O 3.1 permits them and defines its own
      pattern/flag rules. Choose a documented standards-compatible subset that
      explicitly rejects unsupported valid constructs, or implement the full
      XPath regex language with bounded execution before enabling the demos.
      Recommended: the explicit subset first, consistent with staged XPath
      delivery; do not silently treat Rust regex syntax as XPath syntax.
      Sources: [XPath regex](https://www.w3.org/TR/xpath-functions-31/#regex-syntax)
      and [regex 1.12.3](https://docs.rs/regex/1.12.3/regex/index.html).
      Accepted 2026-09-18: option 1, the explicit standards-compatible subset.
      Reject unsupported constructs; do not silently accept the engine dialect.
- [x] Fixture XPATH-DEMO-VALIDATION: add bounded XPath regex `fn:matches` and
      regex tokenize/replace when needed, preserving standard pattern/flag/
      replacement and error semantics rather than inheriting literal CEM-QL
      split/replace behavior. Reuse `some`/`every` and supported `castable as`
      in form and IP-filter preview cases, including numeric ranges and invalid
      input; regex alone must not be presented as complete address validation.
      Date/time types and formatters remain a later separately bounded slice.
    - [x] Fixture XPATH-REGEX-NATIVE: implement and document a bounded regex
          grammar/flag subset for matches, replace and tokenize. Test standard
          errors, dialect exclusions, empty matches, replacement captures,
          Unicode/XML whitespace, limits, cancellation and compiled reload.
          Completed 2026-09-18: seven native groups cover the explicit grammar,
          s/x/q flags, numbered captures, standard errors and exclusions,
          repeated-search work charging, hard/shared limits and control checks.
          Typed programs reload without source; XML/JSON/YAML atomization uses
          retained imported trees. NFA state/edge work counts are portable.
    - [x] Fixture XPATH-REGEX-HOST: verify separate-library invocation and
          native/WASM parity, including retained imported node atomization,
          diagnostics and host budget propagation; extend import source guards.
          Completed: all 352 CEM-QL tests and 125 native/WASM companion checks
          pass, including source/reloaded libraries, explicit error diagnostics,
          and uncatchable capability/resource failures. The source guard now
          includes the regex module.
    - [x] Fixture XPATH-REGEX-DEMO: add form and IPv4/CIDR preview samples using
          authored lexical checks plus numeric ranges, with standalone/source-
          loaded interactions, recovery, focus and compact-layout coverage.
          Completed 2026-09-18: two anonymous samples invoke a separate XPath
          library for form fields and an IPv4 prefix-length allow-list. Browser
          coverage verifies boundaries, invalid input, recovery, focus/caret and
          instance isolation. Both loading modes fit two 704px cards at 1440px
          without page overflow; the preview does not claim subnet membership.
- [x] XPATH-DEMO-CONFORMANCE-DOCS: reconcile the XPath conformance matrix and
      README with the implemented arithmetic/range and accessor/numeric
      function dispatch before marking new slices complete. Distinguish native
      evaluation, host wiring, compiled representation and actual demo coverage.
      Completed 2026-09-17 alongside the sequence slice: corrected stale
      arithmetic/range and promotion gaps, listed existing accessor/numeric
      dispatch signatures and described implemented text/node/sequence slices.
      Native execution, artifact representation, host ownership and per-slice
      demo coverage remain separate; full standard-library/QT3 coverage is
      still incomplete. Keep these notes current as later slices land.
- [x] XPATH-DEMO-VERIFY: for each completed slice run focused native fixtures,
      then shared WASM and browser integration, source guards, fixture inventory,
      lint/typecheck and compact standalone/source-loaded layout checks. Keep
      original CEM-QL cases and their event/reset/focus/instance teaching points.
      Sorting-slice evidence is recorded in XPATH-DEMO-SORT above. Final regex
      verification 2026-09-18: seven native regex groups, 352 CEM-QL tests,
      86 XPath unit tests and 125 native/WASM companion checks pass; 382 host
      unit tests, all 163 Chromium stories, 25 standalone pages and 31 source-
      loaded documents pass. Import-boundary guards, native/TypeScript lint,
      typecheck and both validation-demo layout checks pass.
- [x] XPATH-DEMO-JSON-NATIVE-REPLAN: replaced the deferred JSON-specific
      reader/view decision on 2026-09-17 with XPATH-CEM-TREE-PARITY, VIEW and
      INTEGRATION above, following the user's common-tree direction. This
      closes only the obsolete planning item; runtime delivery is recorded in
      the shared capability and integration items above.
- [x] XPATH-DEMO-HTTP-NATIVE-REPLAN: the earlier HTTP deferral and pending
      CEM-IMPORT-LEGACY-BINDINGS choice are superseded by CEM-LOADER-DIRECTION.
      Track implementation under CEM-LOADER and CEM-IMPORT-LEGACY-VIEWS above.
      Progressive AST streaming remains a later phase; loaded document
      objects are not a compatibility target. This closes planning, not runtime work.

## Immediate: XSLT-Authored Data-Table Viewer

Feature list and scope gate:
[`xslt-data-table-parity.md`](xslt-data-table-parity.md). Use standard XSLT 3.0 /
XPath features when available; extensions may expose only capabilities without
a standard equivalent. Keep all table/tree/form presentation in the stylesheet.
Pause and request approval before expanding CEMT, CEM-QL, importers or browser
runtime. This is not authorization to implement a complete XSLT 3.0 processor.

- [x] XSLT-VIEW-LIST: record the bounded feature list, standards-first extension
      policy, parity criteria and explicit scope gate before implementation.
- [x] Fixture XSLT-VIEW-GATE: test the native data-model boundary before
      lowering standard XML/JSON parsing. Verify source identity, JSON object
      order, null array members and the distinction between CEM AST nodes and
      the standard JSON-to-XML result. Record any essential shared capability
      gap before changing shared implementation.
      Gate result 2026-09-13: three native characterization tests pass in
      `packages/cem_ql/tests/xslt_data_model_boundary.rs`. CEM array IR flattens
      sequence members; the native JSON projection retains null/source order
      but has CEM object/property nodes, not the standard json-to-xml tree.
      Ordinary records preserve a referenced source but do not create native
      node views. See the feature document for alternatives and evidence.
- [x] XSLT-VIEW-APPROVAL: user approved the reusable native JSON-to-XML
      projection on 2026-09-13, with ownership corrected to `cem-ml` JSON
      import. Expose it explicitly through the existing data reader; keep the
      default projection and array behavior unchanged. Standard XSLT function
      syntax/semantics still belong to the compatibility layer. Other shared
      capability expansions still require approval.
- [x] Fixture XSLT-VIEW-JSON-PROJECTION: build native CEM AST directly from
      the retained JSON AST using the standard XPath-functions namespace and
      map/array/keyed-value structure. Test scalar roots, null slots, member
      order, duplicate handling, XML-safe string escaping, source ranges and
      depth/value limits first. Add an explicit query/declarative projection
      selector, native-owner/identity tests, and default-import regression tests.
      Completed 2026-09-13: `cem-ml::validation::json_xml` owns the projection;
      `data:read(source, "json", "json-to-xml")` and the optional declarative
      `projection` attribute select it without changing default imports.
      Seven projector/presentation tests and 26 focused query/viewer/XSLT
      regression tests pass. WASM build and Clippy checks pass. Standard XSLT
      function lowering and the equivalent gallery cases remain open below.
- [x] Fixture XSLT-VIEW-ERROR-GATE: characterize parse-report failures, fatal
      diagnostic emission, coalescing after an evaluation error, and partial
      output from a failing nested template call. Check whether existing CEMT
      can supply scoped recovery for standard `xsl:try`/`xsl:catch` before
      changing shared evaluation or rendering contracts.
      Gate result 2026-09-13: four native characterization tests pass in
      `packages/cem_ql/tests/xslt_error_recovery_boundary.rs`; 13 focused
      error/data-model/legacy-XSLT tests pass together. Parse failures are
      report values, even fatal report emission is not a raised query error,
      coalescing does not recover errors, and failed nested calls keep partial
      output. No shared implementation changed. AC-QE-2 already plans query
      recovery as Tier B; it was not implemented at this gate (now completed
      in the approved recovery items below).
- [x] XSLT-VIEW-ERROR-DECISION: obtain direction before runtime lowering:
      either approve reusable query error propagation/recovery and CEMT scoped
      output recovery, or narrow the XSLT profile to an explicitly validated
      parser-only `xsl:try` form lowered through reader reports. The latter is
      not general recovery across expressions, template calls or imported
      aspects. User approved reusable query/CEMT recovery on 2026-09-13.
- [x] Fixture XSLT-VIEW-QUERY-RECOVERY: implement generic `report:raise` and
      `try { ... } catch (code, msg) { ... }`; test parsing, lexical scope,
      nested/rethrown errors, diagnostic source maps, native value identity,
      artifact reload and uncatchable cancellation/resource limits. Preserve
      diagnostic-only `report:emit` and reader-report behavior.
      Completed 2026-09-13: eight dedicated native tests pass, including
      first-error propagation and resource budgets across repeated catches.
- [x] Fixture XSLT-VIEW-TEMPLATE-RECOVERY: implement scoped CEMT try/catch;
      test nested/named/imported calls, catch selection and outer recovery,
      rollback of nodes/constructed attributes/bindings, artifact reload,
      source maps and uncatchable controls. Verify native CLI and WASM wiring
      before resuming XSLT-specific lowering.
      Completed 2026-09-13: ten dedicated native template tests and three CLI
      recovery cases pass. The complete `cem-ql` suite passes (293 tests);
      the Nx WASM build, three fresh WASM recovery checks and Nx lint pass
      (Clippy reports existing warnings). CLI module calls now execute inside
      the protected render boundary. Initial broader runs passed 92/93 adapter
      tests and 8/9 CLI XSLT tests; the loader failure is now fixed below and
      the independent stylesheet-export failure remains under XSLT-VIEW-OUTPUT.
- [x] XSLT-VIEW-LOADER-DECISION: approve the separate existing document-loader
      repair discovered by broad recovery verification. The adapter test
      `common_query_runner_executes_registered_cem_ql_runtime_with_native_owners`
      failed before evaluation: `lower_loaded_markdown_to_html_document` took
      and discarded non-Markdown `loaded.ast_stream` values. The same bug exists
      in `HEAD`. User approved preserving native owners and adding loader
      regression coverage on 2026-09-13; no parser semantics are changed.
- [x] Fixture XSLT-VIEW-LOADER-OWNERS: after approval, preserve native owners
      when the Markdown-only conversion does not apply. Test XML/JSON/CSV/YAML
      document loading, Markdown conversion/failure retention and the existing
      native query-runner test. Verify downstream WASM reads for all four data
      formats and the explicit JSON-to-XML projection. No parser semantics or
      serialized AST fallback.
      Completed 2026-09-13: the Markdown helper borrows the retained AST and
      replaces it only after successful conversion. Five format tests failed
      before the fix; all eight loader checks now pass (seven new, including
      HTML, absent ASTs and failed Markdown conversion). Native allocation
      identity, typed source metadata, bytes, format, adapter and diagnostics
      are preserved. All 204 engine tests and 93 adapter tests pass, including
      the previously failing native query-runner case. The three CLI recovery
      cases pass; the CLI XSLT suite remains 8/9 because of the separate
      stylesheet-export issue below. Both JSON-boundary audits pass. Fresh Nx
      lint (existing warnings), WASM build and five WASM data-read checks pass.
      Verification note: lint caches all of `dist/target/cem_ql`, overlapping
      WASM outputs. An initial concurrent run lost a dependency artifact;
      sequential `--skipNxCache` runs passed without build-configuration edits.
- [x] Fixture XSLT-VIEW-XML-NODE-GATE: characterize the native XML node model
      before lowering XPath paths or `fn:parse-xml`. Check adjacent text/CDATA,
      empty text, namespace declarations, XML declarations, source identity and
      native navigation. Inspect the existing native XPath owner as a reuse
      candidate; do not mistake source events for normalized XDM nodes.
      Gate result 2026-09-13: four native characterization tests pass in
      `packages/cem_ql/tests/xslt_xml_model_boundary.rs`; 17 XML/JSON/error/
      legacy-XSLT boundary tests pass together. `<r>a<![CDATA[b]]>c</r>` has
      three source-oriented children, not one XDM text node. Empty CDATA,
      whitespace and declaration/xmlns nodes also differ. The existing native
      XPath owner retains ownership but likewise returns three text events.
      No production code changed; runtime lowering remains open.
- [x] XSLT-VIEW-XML-NODE-DECISION: if those probes require a new shared native
      projection/view, request approval before implementation. Keep the default
      source-preserving CEM AST unchanged; no serialized or record-shaped
      pseudo-node workaround and no implicit restriction of XML sample input.
      Resolved 2026-09-13: user directed this work to XPath/XDM, reused by XSLT,
      and asked to continue. Normalize the existing `XPathNativeNode` view;
      leave the core XML AST/importer, CEM AST and default `data:read` unchanged.
      No new generic reader projection is authorized or required for this step.
      Stop again if runtime integration needs a shared evaluator/reader change.
- [x] Fixture XSLT-VIEW-XML-NODE-PROJECTION: after approval, test native XML
      normalization, entity/CDATA/whitespace runs and comment/element boundaries,
      namespace shadowing, PI targets, identity/order/navigation/source maps,
      safety limits and unchanged default imports. Assess the existing XPath
      owner for reuse without claiming that its current event handles are XDM.
      Implement in the XPath layer: canonical text-run handles/values/source
      maps, empty-text omission, namespace/PI metadata and axes over the logical
      nodes. Cover constructor identity, node comparisons, positions, string/
      typed values, cancellation/sequence limits and unchanged native owners.
      Update only the XPath expectation in the original boundary fixture;
      keep its default CEM-reader characterization assertions unchanged.
      Completed 2026-09-13 in `validation::xpath` only (including its typed
      parser metadata): text runs have canonical first-event identity and
      multi-span provenance, decoded entities and normalized literal line
      endings; empty/document-level text is omitted. PI names and parsed target
      tests are supported; namespace declarations cannot become attributes.
      All 11 native view tests and 86 existing XPath unit tests pass. Core XML
      AST/importer and default CEM reader are unchanged by this work.
      The 22 XSLT unit tests and 23 reader/XML/JSON/error/legacy-XSLT regression
      tests also pass. Fresh uncached Nx lint passes with existing warnings.
      The sequential uncached WASM build and five WASM reader checks pass;
      XML still exposes separate text/CDATA events, and CSV/YAML/JSON plus the
      explicit JSON-to-XML projection keep their existing shapes.
- [x] Fixture XSLT-VIEW-XML-RUNTIME-GATE: verify XSLT-owned typed XPath can
      reuse the normalized native XML view. Check whether generated CEMT can
      invoke that path, including XML parsed from runtime-bound source strings.
      Do not add generic reader projection, source reparse bridges or native
      query callbacks without a separate scope decision.
      Result 2026-09-13: an XSLT-owned select AST reuses normalized native XML
      nodes across changed runtime inputs, with identity and source maps intact.
      `XsltXPathInvocationAdapter` works, but the XSLT-to-CEMT adapter does not
      call it. CEM-QL function dispatch is fixed; `EvaluationContext` has no
      native query-function hook. The existing template-call callback returns
      rendered output/failure, not query values. `data:read` retains XML behind
      a private owner and exposes the unchanged source-oriented CEM tree.
- [x] XSLT-VIEW-XML-RUNTIME-DECISION: approve a reusable native query-function
      integration hook in CEM-QL/CEMT before wiring runtime XSLT XPath/parsing,
      or choose another native execution route with an explicit scope contract.
      Recommended: the shared hook owns only invocation, values, controls and
      diagnostics; XPath owns XML normalization/evaluation, XSLT owns lowering,
      standard function semantics and error translation. Keep core XML/CEM AST
      and default imports unchanged. No table-specific code or source/AST
      serialization bridge. Approved by the user on 2026-09-13.
- [x] Fixture XSLT-VIEW-NATIVE-CALL: implement an explicit, opt-in native
      query-function registry and generic `native:call` dispatch. Preserve
      native arguments/results and sequence boundaries, validate names/arity,
      reject duplicate registrations and missing capabilities, retain source
      diagnostics, and enforce cancellation and call/result budgets. Callbacks
      remain outside compiled artifacts and serialized/browser data bindings.
      Implemented 2026-09-13: exact identifier/arity registration is explicit,
      duplicate/missing capabilities fail, and each argument retains its native
      sequence boundaries. The evaluator owns argument diagnostics, call/item
      accounting and pre/post-call control checks; callbacks receive the active
      scope and must cooperatively bound their own work/allocations. Default
      registries are empty; no query/template artifact format change.
- [x] Fixture XSLT-VIEW-NATIVE-CEMT: carry that registry through CEMT expression
      evaluation, loops, named/module calls and scoped error recovery. Test
      compile-once/render-many, artifact reload with explicit capability
      rebinding, native ownership and uncatchable controls. Prove an XSLT-owned
      typed XPath expression executes through this hook at render time without
      expression reparsing or core AST/importer changes. Verify native/CLI
      regressions, then sequential uncached Nx lint/WASM checks.
      Native verification: 11 hook tests and all 308 CEM-QL tests pass, along
      with 93 adapter tests, the new XSLT-owned XPath hook fixture, 86 XPath
      tests and the CLI recovery fixture (three cases). The controlled XSLT
      invocation method reuses its existing host validation and evaluator.
      Verification completed 2026-09-13: sequential uncached Nx lint and WASM
      build pass (22 existing lint warnings), followed by nine WASM checks for
      native capability isolation, CEMT failure propagation and unchanged
      XML/CSV/YAML/JSON reader projections. A transient Nx project-graph cache
      read failure cleared on retry; no cache or configuration edits were needed.
      The fixture exercises the runtime connection, not production stylesheet
      lowering. Keep XSLT-VIEW-LOWER and the viewer/sample work open below.
- [x] XSLT-VIEW-DELIVERY-GATE: inspect how generated CEMT retains the typed
      XPath programs when it leaves the compiler process and enters the demo.
      Result 2026-09-14: the native hook is callable, but its registry and
      XSLT-owned expression ASTs are deliberately absent from portable CEMT
      artifacts. The browser WASM host retains only `TemplateArtifact` and
      creates callback-free `TemplateData` from JSON. Its module closure carries
      CEMT source, not native executable capabilities. Existing artifact-reload
      and native XPath fixtures establish these separate contracts; neither
      provides an XSLT-aware browser loader. Do not emit unresolved callback IDs
      as a supposedly standalone, usable viewer.
      Verification 2026-09-14: the focused CEMT binary-reload test and the
      XSLT-owned native XPath integration test both pass; `git diff --check`
      passes. This gate changes documentation only.
- [x] XSLT-VIEW-DELIVERY-DECISION: approve an XSLT-owned compiled-program bundle
      and explicit browser/WASM host loading/rebinding, or require standalone
      CEMT on the unchanged host. Recommended: allow the XSLT-owned bundle and
      scoped loader; keep generic CEMT artifacts, default callback registration,
      core ASTs/importers and JSON data bindings unchanged. Define version/hash,
      import ownership, source maps and lifetime validation before implementing
      the bundle. This is a delivery/host decision, not a repeat request for the
      already-approved native hook. Approved 2026-09-14, with the user's
      requirement that XPath own its namespace and content type, as CEM-QL does.
- [x] Fixture XPATH-COMPILED-ARTIFACT: preserve XPath's existing namespace and
      source content type; introduce a separately identified binary compiled
      program under the XPath package, not a CEM-QL or XSLT expression format.
      Test deterministic bytes, typed syntax/static-context/host provenance,
      source-free reload and changed native inputs, unsupported/truncated/
      corrupted artifacts, identity mismatch and bounded decoding. Do not add
      serialization dependencies to the source syntax AST or serialize runtime
      data nodes, callback closures, or query bindings. Keep default imports
      and generic CEMT artifacts unchanged.
      Completed 2026-09-14: XPath owns
      `application/vnd.cem.xpath-artifact+cem-bin`, with versioned/hash-checked
      typed-program encoding, source-free reload and strict decoding limits.
      Six integration tests and two adversarial codec tests pass, alongside
      all 86 existing XPath unit tests and 11 XPath XML-view tests. The source
      syntax AST remains serde-free; the binary type uses explicit artifact
      APIs, not automatic source/lifecycle importer dispatch. Arbitrary XML
      attachments cannot imply an executable language host. The uncached
      `cem_ml_schema_package_xpath_v1:verify` target also passes, including
      schema registration, source/lifecycle compatibility and CLI examples.
- [x] Fixture XPATH-WASM-ARTIFACT: expose explicitly named XPath artifact
      compile/import/dispose entry points in the shared WASM host. Preserve its
      own media type and schema, validate externally supplied hashes and host
      identity, bound retained programs and reject stale handles. Test a native
      XSLT-owned binary in WASM as well as standalone XPath compilation, without
      enabling callbacks through JSON data or changing generic CEMT handles.
      Completed 2026-09-14: `compileXPathArtifact`, `importXPathArtifact` and
      `disposeXPathArtifact` preserve XPath identity and enforce content/source
      hashes, host compatibility, size/count limits and non-reused handles.
      The uncached `cem_ml_schema_package_xpath_v1:verify:compiled-artifacts`
      Nx target passes all 13 native/WASM checks after rebuilding the shared
      WASM host. Native/WASM standalone compilation produces identical bytes;
      loading XSLT-owned programs does not install CEMT native callbacks.
      Uncached CEM-QL lint passes with warnings. XSLT bundle composition and
      explicit capability binding remain open below.
- [x] Fixture XSLT-BUNDLE-FOCUS-GATE: characterize the outer XPath focus
      available to an XSLT bundle callback before fixing its invocation
      contract. Compare per-item host calls with expression-local iteration,
      source and binary-reloaded programs, using retained imported CEM nodes.
      If host position/size need a shared XPath API expansion, stop for that
      decision under the XSLT viewer scope boundary.
      Result 2026-09-18: two native fixtures confirm that per-item XSLT-host
      calls retain CEM node ownership and source provenance through binary
      reload, but always initialize position/size to 1/1. Expression-local
      simple maps and predicates establish and restore correct sequence focus.
      `XPathDynamicContext` cannot accept an outer position or size; this is
      an invocation API gap, not a compiled-program encoding failure. See
      [the focus gate](xslt-data-table-parity.md#bundle-invocation-focus-gate-2026-09-18).
      Verification: all 97 adapter tests pass through Nx, including the two
      new probes and existing native-hook integration. Adapter lint passes
      with existing warnings; fixture formatting and `git diff --check` pass.
- [x] XSLT-BUNDLE-FOCUS-DECISION: approve a shared XPath dynamic-context
      extension for explicit host position/size before defining the bundle
      call contract, or restrict the initial bundle to singleton host focus
      and explicitly reject instructions that require unavailable outer focus.
      Recommended: extend the XPath-owned invocation context with validated
      position/size, preserving current singleton defaults, expression-local
      focus changes, absent focus in inline functions, native owners and
      source diagnostics. Runtime focus must not be serialized into programs.
      The bundle/native-hook approvals remain resolved; the new decision is
      only this shared capability expansion.
      Approved 2026-09-18: option 1, explicit host position and sequence size
      in the shared XPath dynamic context.
- [x] Fixture XPATH-HOST-FOCUS: add validated explicit host position/size with
      singleton defaults. Verify absent/invalid focus, nested predicates/maps,
      inline-function focus isolation, native owners, source diagnostics,
      limits/control and source-free reload. Replace the XSLT boundary probes
      with invocation acceptance tests; verify native and shared WASM paths.
      Completed 2026-09-18: optional `u64` context position/size are validated
      together with the item; omitted coordinates preserve singleton/absent
      focus. Seven direct fixtures and three XSLT invocation fixtures cover
      the contract, including exact 64-bit values and original source ranges.
      Verification: 86 XPath unit tests, 42 focused native cases and all 98
      adapter tests pass; shared WASM rebuild, 17 artifact and 125 companion
      checks pass. Native lint passes with existing warnings. Artifact bytes
      and versions stay unchanged; bundle/WASM focus binding remains below.
- [x] Fixture XSLT-COMPILED-BUNDLE: compose generated CEMT with independently
      identified XPath programs and stylesheet imports. Validate versions,
      hashes, source maps, capability ownership and disposal; explicitly bind
      capabilities in the browser/WASM host. Ordinary JSON data must remain
      unable to install native callbacks. Verify cross-process loading before
      claiming a deployable XSLT-authored viewer.
    - [x] Fixture XSLT-BUNDLE-NATIVE: define the bundle identity, ordered
          stylesheet closure, independently hashed CEMT/XPath members, source
          ownership, explicit focus/variable ABI and bounded native retention.
          Verify deterministic source-free reload, changed imported inputs,
          invalid closures/members, isolation, controls and disposal.
    - [x] Fixture XSLT-BUNDLE-WASM: expose explicit import/render/dispose
          entry points over the native bundle host. Verify native-produced
          bytes in a separate WASM process, retained CEM document binding,
          stylesheet diagnostics, capability isolation and stale handles.
      Implemented [the versioned bundle contract](xslt-bundle.md) and explicit
      WASM loader. Nine native cases and 42 native/WASM checks pass, including
      identical XPath over XML/JSON/YAML/CSV imports, original stylesheet
      coordinates, typed variables, shared cancellation/budgets and disposal.
      All 361 CEM-QL tests, 13 schema-package checks and the existing 17 XPath
      artifact / 125 companion WASM checks pass; native lint has existing
      warnings only.
      This fixture composes generated CEMT explicitly; typed lowering follows
      below and the executable viewer remains open.
- [x] Fixture XSLT-VIEW-LOWER: preserve XSLT instructions as runtime CEMT;
      translate the needed XPath paths, predicates, variables, context,
      sequences, conditionals and map/array operations with standard semantics.
      Reject unsupported expressions; do not unroll current source data.
    - [x] Fixture XSLT-LOWER-CORE: compile a single root template from the typed
          stylesheet AST into the approved bundle. Cover nested loops and
          restored focus, scoped native variables/maps/arrays, XPath boolean
          tests, simple text construction, changed imported documents, strict
          unsupported-instruction diagnostics, compiler limits and atomic
          failure output.
          Verify native-produced bundles in WASM before exposing source compile
          and retention entry points; keep template dispatch/imports and the
          complete viewer/output profile in their following fixtures.
      Implemented the [bounded typed runtime compiler](xslt-runtime-lowering.md)
      and WASM source compile/retention entry points. Eight native lowering
      cases and 59 native/WASM checks cover changed XML/JSON/YAML/CSV documents,
      native values, lexical/focus scope, diagnostics, output discard and
      resource limits. All 369 CEM-QL tests, 17 XPath artifact checks and
      125 companion checks pass; lint retains its 22 existing warnings.
      Template dispatch and adapter migration follow below.
- [x] Fixture XSLT-VIEW-MATCH: lower recursive named/matched templates,
      parameters, modes, priorities and imports. Preserve XSLT import precedence
      independently of CEMT priority ordering and test imported overrides.
      Migrate the older transform/CLI adapter to the typed bundle path once
      those existing entry/template contracts are covered; never fall back
      silently from unsupported typed compilation to legacy token rewriting.
    - [x] Fixture XSLT-ADAPTER-MIGRATION-GATE: characterize the legacy adapter's
          accepted version/namespace forms against the typed compiler and record
          the unresolved public compatibility choice before replacing dispatch.
          Keep runtime behavior unchanged until the user selects retirement
          with consumer migration or an explicitly separate compatibility route.
      Three native public-API characterization tests pass.
      Decision accepted: retire legacy execution and migrate affected consumers
      and tests to namespace-correct XSLT 3.0 with explicit declarations. Keep
      standalone legacy conversion tools separate; no fallback execution route.
    - [x] Fixture XSLT-MATCH-RUNTIME: compile recursive named/matched templates,
          parameters (including empty values and defaults), modes and native
          focus restoration; map exact XSLT priority/import order to existing
          CEMT dispatch. Verify source ownership, unsupported forms and limits.
          Cover cancellation, host parameter binding validation, and explicit
          preflighted module authorization without permitting
          runtime document/output URI access.
    - [x] Fixture XSLT-MATCH-ADAPTER: replace legacy execution with typed bundles,
          migrate native/CLI consumers and rejection contracts, and verify
          retained imported documents plus explicit WASM loading. Cover CLI nested
          import/include preflight and cycles, and explicit rejection of pending
          stylesheet style exports while retaining their OUTPUT acceptance case.
      Implemented the [bounded dispatch/compiler profile](xslt-runtime-lowering.md)
      and retired legacy execution. Verification: 382 CEM-QL tests, 101 adapter
      tests, 22 XSLT validation tests, 16 CLI XSLT unit tests, 10 active CLI
      integration tests, 61 native/WASM bundle checks, 17 XPath artifact checks
      and 125 companion checks pass. Both lint targets pass with existing warnings. The stylesheet style-export success case is
      explicitly ignored under OUTPUT; its rejection/no-export case is active.
- [x] Fixture XSLT-VIEW-GROUP: lower for-each-group, current-group and
      current-grouping-key to existing generic queries. Derive repeated rows
      and first-seen union headings in the stylesheet, never in Rust.
    - [x] Fixture XSLT-GROUP-PREREQUISITES: verify native grouping-key
          comparisons, first-seen keys and retained node identity/axes through
          XPath arrays. Two native tests pass; CEM identity and XPath map
          same-key must not substitute for XSLT grouping equality.
    - [x] Fixture XSLT-GROUP-CONTEXT: implement the shared XPath group context
          approved on 2026-09-18. Enable repeated-row/heading derivation,
          population/group focus, nested/called-template scope, multiple/empty
          keys, duplicate population positions and lazy absent-group errors.
          Cover numeric non-transitivity/NaN, native containers, compiler-name
          hygiene, pattern errors and explicit rejection of unsupported forms.
    - [x] Fixture XPATH-XSLT-GROUP-HOST: verify native host isolation,
          absent/present-empty values, closure clearing, focus independence,
          ownership, key validation and resource limits.
    - [x] Fixture XSLT-GROUP-WASM: verify native/source-compiled bundle equality,
          changed documents and heading unions, the same grouping stylesheet
          across XML/JSON/YAML/CSV imports, handle disposal and source-mapped
          group errors without partial output.
      The [bounded grouping profile](xslt-runtime-lowering.md#grouping) supports
      non-composite `group-by` with Unicode codepoint comparison. Verification:
      393 CEM-QL tests (11 grouping cases, none ignored), 92 shared XPath tests,
      101 adapter tests, 80 native/WASM bundle checks, 17 XPath artifact checks
      and 125 function-companion checks pass. Nx lint passes with existing
      warnings. Sorting remains the next task.
- [x] Fixture XSLT-VIEW-SORT: lower dynamic stable multi-key sorting with
      standard semantics. Author explicit missing/invalid-last keys in XSLT;
      do not equate the legacy invalid-number-as-zero behavior with parity.
    - [x] Fixture CEMQL-SORT-CALL-BUDGET: reproduce the shared evaluator's
          cumulative CallDepth bug under explicit one/four-worker policies;
          with user approval, enforce active nesting independently from cumulative
          FunctionCalls and cover sequential, recursive and recovered calls.
          Four native regressions cover named/native/lambda calls and
          uncatchable depth/total-call exhaustion without increasing limits.
    - [x] Fixture XSLT-SORT-NATIVE: prove common numeric promotion and stable
          descending sorting with existing XPath; lower sort keys with original
          focus, dynamic controls with outer focus, and sorted body focus.
          Cover multi-key ties, explicit validity keys, node/container ownership,
          template dispatch, group context, diagnostics and resource bounds.
    - [x] Fixture XSLT-SORT-WASM: reload native sorting bundles, compare native
          and WASM compilation, and verify changed inputs and sorting errors.
      The [bounded sorting profile](xslt-runtime-lowering.md#sorting) supports
      loops, template application and groups, with original key focus, dynamic
      outer-focus controls, native ownership and common numeric promotion.
      Verification: 409 CEM-QL tests (12 sorting and four call-budget cases,
      none ignored), 101 adapter tests, 109 native/WASM bundle checks, 17 XPath
      artifact checks and 125 function-companion checks pass. Both Nx lint
      targets pass with existing warnings. The shared call-depth
      correction was approved on 2026-09-18; configured limits and cumulative
      function-call accounting remain unchanged. Standard parsing is next.
- [x] Fixture XSLT-VIEW-DATA: lower standard XML/JSON parsing and error
      recovery to native data capabilities. Use CEM extensions only for
      CSV/YAML parsing and unavailable host/provenance capabilities. Preserve
      native owners; no serialized AST handoff or fake standard function names.
    - [x] Fixture XSLT-DATA-PREREQUISITES: characterize shared XPath parsing
          availability, imported-node ownership, import failure classification
          and standard error identity before requesting shared API expansion.
          Record the bounded proposal and stop for the required scope decision.
      Four native probes pass; all 413 CEM-QL tests pass with none ignored.
      The [shared parsing/error proposal](xslt-runtime-lowering.md#standard-parsing-and-recovery)
      was approved on 2026-09-18. The existing native tree route and recovery
      are reusable.
    - [x] Fixture XSLT-DATA-IMPORT: add typed string-import errors/options and
          test XML declarations, JSON BOM/escapes/duplicates, source retention,
          default-reader compatibility and bounded failures at the importer.
    - [x] Fixture XSLT-DATA-XPATH: replace missing-function probes with standard
          parsing successes and typed errors; cover lazy/nested invocation,
          native owners, base URI, empty inputs, limits and cancellation.
    - [x] Fixture XSLT-DATA-RECOVERY: lower ordered QName/wildcard catches,
          typed error variables and buffered rollback; verify nested calls,
          scope/focus, CSV/YAML extensions and uncatchable resource failures.
    - [x] Fixture XSLT-DATA-WASM: reload portable parsing/recovery bundles and
          compare native/WASM compilation and changed-input rendering.
    - [x] Fixture XSLT-DATA-BOUNDARY: repair the serialization audit's stale
          adapter end marker after strict XSLT moved into its own module;
          preserve the existing zero-serialization requirement.
      Verification: 419 CEM-QL tests, 101 adapter tests, 297 CEM-ML integration
      tests, 118 native/WASM bundle checks, 17 XPath artifact checks and 125
      function-companion checks pass. The CEM-ML library run passed 2,027 tests
      and failed the existing debugger pause/deadline timing test; that test
      passed in isolation. The stale serialization-audit marker was repaired
      without weakening its requirement. XML/JSON decoding and projection
      remain in CEM-ML import; XPath and XSLT consume retained native trees.
      All three Nx lint targets pass with existing warnings.
- [x] Fixture XSLT-VIEW-OUTPUT: preserve HTML, AVTs, whitespace, static styles
      and existing slice/event bindings. Document the bounded profile and
      source-located diagnostics without claiming full XSLT 3.0 support.
    - [x] Fixture XSLT-OUTPUT-PREREQUISITES: characterize native-node insertion,
          atomic/text distinction, attribute ordering and result-style versus
          declaration-style handling. Record any shared result-construction
          capability needed before expanding the renderer.
      The initial five probes characterized the boundary; ordinary CEMT
      compatibility probes remain active alongside the new native output tests.
      The [result-construction proposal](xslt-runtime-lowering.md#native-output-construction)
      was approved on 2026-09-18. Keep ordinary CEMT interpolation and declaration
      styles compatible while adding the explicit native result path.
    - [x] Fixture XSLT-OUTPUT-NATIVE: add portable typed result instructions;
          test retained nodes, atomic/text adjacency, attributes, namespaces,
          nested calls/recovery, limits and cancellation against ordinary CEMT.
    - [x] Fixture XSLT-OUTPUT-LOWERING: lower sequence/select-based try/catch,
          result constructors, AVTs, whitespace and slice/event attributes;
          test source locations, changed inputs and explicit unsupported forms.
    - [x] Fixture XSLT-OUTPUT-CLI: restore static-style link/inline/omit/CSS
          exports and source maps without changing component declaration styles;
          verify native XML/JSON subtree export and expanded names through the CLI.
    - [x] Fixture XSLT-OUTPUT-WASM: reload native result bundles, compare bytes
          with WASM compilation, and verify changed inputs and typed failures.
      Native constructors retain pending nodes/atomics across calls, loops and
      recovery; output AVTs, xml:space and existing slice/event attributes are
      lowered. Static style link/inline/omit/CSS exports and source maps are
      active CLI acceptance cases. XML/JSON/YAML/CSV decoding stays in CEM-ML
      import, guarded against format logic in native result construction.
      Verification: 442 CEM-QL tests, 101 adapter tests, 12 CLI parity tests,
      both import-boundary audits, 132 native/WASM bundle checks, 17 XPath
      artifact checks and 125 function-companion checks pass. The existing
      XPath functions demo remains at `packages/cem-elements/demo/xpath-functions.html`.
      Nx CEM-QL and adapter lint pass with existing warnings. Ordinary CEMT
      render-protocol attributes remain unchanged; native namespace metadata is
      emitted only for the explicitly selected result path.
- [x] Fixture XSLT-VIEW-PARITY: author data-table-view.xslt and compare its
      CLI-translated rendering with direct CEMT across XML/CSV/YAML/JSON,
      heterogeneous/nested/empty data, sorting, selection and parse failures.
    - [x] Fixture XSLT-VIEW-LIVE-VARIABLES: pass only referenced outer variables
          to each compiled XPath slot. Verify expanded names, inline/local
          shadowing, binary reload and normal viewer-sized collections without
          changing shared argument, item or function-call budgets.
    - [x] Fixture XSLT-VIEW-SORT-POPULATION: evaluate ordinary sort keys in
          native population focus instead of repeated per-row host callbacks;
          preserve cardinality errors, namespaces, original key locations,
          group-context isolation, promotion and stable multi-key ordering.
    - [x] Fixture XSLT-VIEW-BASE-NATIVE: compare the authored base stylesheet's
          semantic DOM with CEMT; verify sorting, selection, malformed input,
          namespace exclusion and all four import formats through the real CLI.
    - [x] Fixture XSLT-VIEW-BASE-WASM: compare native/WASM stylesheet compilation
          and native-produced named-entry bundles across sorting, source edits
          and selection, retaining scalar controls and CEM document handles.
          Verification: 457 CEM-QL tests, 13 real CLI parity tests and 172
          native/WASM checks pass. The base stylesheet is ready for native/CLI
          use; browser wiring, styling and imported XSLT aspects remain open.
    - [x] XSLT-VIEW-BROWSER-PARAMS-DECISION: choose the component's state-binding
          contract before adding declaration/worker integration. Recommend
          explicit scalar mappings to declared XSLT template parameters;
          the alternative introduces a native control-document node contract.
          See the [browser proposal](xslt-runtime-lowering.md#browser-state-binding-decision).
          Native bundle delivery, import and provenance approval remain settled.
          Explicit scalar parameter mappings approved on 2026-09-19.
    - [x] Fixture XSLT-VIEW-BROWSER-SCALARS: compile declared CEM-QL state
          expressions once, bind only scalar results to declared XSLT parameters,
          preserve defaults, and reject duplicate names, structured values and
          invalid expressions. Verify native and WASM option transport.
          Include imported modules/hash drift, native-property scalar selection,
          mapped empty sequences, retained-handle limits/disposal and all four
          viewer import formats through native-compiled control fixtures.
          Verification: 463 CEM-QL tests and 200 native/WASM bundle checks pass;
          CEM-QL lint passed with its existing 23 warnings. At that checkpoint,
          component authoring and worker wiring awaited the focus choice below.
    - [x] Fixture XSLT-VIEW-NAMED-FOCUS-BOUNDARY: characterized the original
          singleton-context requirement for scalar-only named templates in native
          and WASM adapters, without inventing a source tree. The approved fix
          below replaces that limitation with explicit optional focus. See the [tested focus proposal](xslt-runtime-lowering.md#named-entry-initial-focus-decision).
    - [x] XSLT-VIEW-NAMED-FOCUS-DECISION: choose absent initial focus for
          named entrypoints (recommended, following XSLT 3.0), or require a
          separate native source-document binding on every browser declaration.
          Absent initial focus approved; scalar parameter mappings remain approved.
    - [x] Fixture XSLT-VIEW-OPTIONAL-FOCUS: add an explicit optional-sequence
          bundle focus form; verify scalar-only named calls, lazy standard
          missing-focus errors, recovery, nested calls, populated/empty loops,
          imported modules, binary reload and native/WASM parity. Preserve
          strict singleton/sequence validation and shared limits.
    - [x] Fixture XSLT-VIEW-BROWSER-LIFECYCLE: retain typed XSLT in the shared
          processing engine; test cache identity, disposal, worker recovery and
          declaration state changes without the legacy converter.
          Browser lifecycle and viewer interactions pass with the approved
          environment/scope control-input policy below.
          Match worker eviction to the native 16-component retention limit;
          preserve stylesheet URI in identity and source-selection provenance.
    - [x] Fixture XSLT-VIEW-PARAM-FORWARD: forward typed bare variable
          sequences through existing CEMT bindings without allocating redundant
          XPath programs, including empty-sequence defaults; preserve native
          values, missing-focus semantics and the 128-program limit. Verify imported aspect compilation and parity.
    - [x] Fixture XSLT-VIEW-EXTERNAL-CONSUMER: migrate the existing standalone
          data-island-tree.xsl demo and named/anonymous consumers to strict XSLT
          3.0 with explicit XML source-text mapping. Verify native import, source
          contracts, source-loaded stories and standalone/source-loaded inventory.
          Checkpoint verification: the 468-test CEM-QL suite and focused consumer
          checks, 13 CLI parity tests, 209 native/WASM bundle checks, 93 unit tests
          and 81 browser stories pass. Lint/typecheck pass (23 existing QL
          warnings); the demo verifier passes 25 standalone pages and 31
          source-loaded documents. At 1440px and 390px, the migrated external
          examples have no page/card overflow; desktop keeps a two-card row.
          The interactive viewer gallery and function audit remain open below.
    - [x] Fixture XSLT-VIEW-CONTROL-BUDGET-BOUNDARY: reproduce the native
          component's 128 KiB control-input rejection after selecting a row in
          the real browser viewer; measure ordinary and 32 KiB-source envelopes
          without decoding document data in JavaScript. Keep the gallery open.
    - [x] XSLT-VIEW-CONTROL-BUDGET-DECISION: host/environment defines the
          control-input ceiling; classic CEM scope inheritance may lower it,
          never raise it. User approved this policy contract on 2026-09-19.
          Existing 32 KiB document-import limits remain separate.
          See the measured [control-envelope proposal](xslt-runtime-lowering.md#browser-control-envelope-decision).
    - [x] Fixture CONTROL-INPUT-SCOPE-POLICY: add the shared native policy cap,
          host configuration and nested constrain-only inheritance. Verify zero,
          relaxed child limits, normalization, UTF-8 boundaries, immutable
          retention/cache identity and worker/fallback policy transport.
    - [x] Fixture CONTROL-INPUT-REGISTRATION: distinguish anonymous declaration
          runtimes/policies in generated tags and reject incompatible fixed-tag
          reuse before it can retain another runtime's wider policy or owner.
    - [x] Fixture XSLT-VIEW-CONTROL-BUDGET: after the decision, enforce the
          approved control-input bound independently of native document/import
          limits. Test UTF-8 byte boundaries, worker/fallback viewer interactions
          and rejection without partial output before publishing gallery cases.
    - [x] Fixture XSLT-VIEW-ASPECTS-NATIVE: author an imported XSLT presentation
          module for notes-as-tree and the local IP-filter form; compare enabled,
          disabled and changed-draft outputs with the CEMT reference.
    - [x] Fixture XSLT-VIEW-SELECTION-BOUNDARY: verify retained row provenance
          across all four string imports, distinguish XDM node identity from
          selection keys across rerenders, and characterize stylesheet access
          to source metadata before choosing the viewer's selection contract.
          The original four native probes confirmed retained provenance and
          CEMT's stable keys, missing XPath metadata access, and CSV's
          header-profile mismatch.
          The [shared proposal](xslt-runtime-lowering.md#viewer-selection-and-csv-parity-gate)
          was approved and implemented on 2026-09-19 in the items below.
          Verification: all 446 CEM-QL tests pass; Nx lint passes with its
          existing 23 warnings. No browser runtime or demo behavior changed.
    - [x] Fixture XSLT-VIEW-SOURCE-ACCESS: expose native source
          keys and line numbers to XPath and CEM-QL; verify repeated parsing,
          same-line nodes, edits, projection separation and native/WASM parity.
        - [x] Fixture SOURCE-PROVENANCE-IMPORT: test shared tree keys/locations,
              byte/string imports, projection options and missing provenance.
        - [x] Fixture SOURCE-PROVENANCE-QUERY: compare XPath and CEM-QL metadata,
              typed errors, limits, retained ownership and binary/WASM reload.
    - [x] Fixture XSLT-VIEW-CSV-HEADERS: add explicit CSV header
          options through CEM-ML string import, preserve the one-argument profile,
          and compare native rows/headers and typed failures with CEM-QL readers.
          Verification for source access and CSV options: all 450 CEM-QL tests,
          32 focused CEM-ML integration tests (including the import-boundary
          audit), and 159 native/WASM compiled-bundle checks pass. CEM-ML and
          CEM-QL Nx lint pass with existing warnings. Source keys and original
          lines agree across native/WASM execution, sorting and binary reload;
          edits invalidate selection. No browser viewer is claimed complete.
    - [x] Fixture XSLT-VIEW-XMLNS-BOUNDARY: compare the CEMT viewer's
          source-oriented namespace-declaration attributes with XPath's
          semantic attribute axis. The original fixture confirmed CEMT emitted
          an XMLNS column while XPath selected only regular attributes; it now
          checks the approved exclusion while preserving the generic reader.
          See the [presentation decision](xslt-runtime-lowering.md#namespace-declaration-parity).
    - [x] Fixture XSLT-VIEW-XMLNS-PARITY: exclude namespace declarations from
          viewer columns, cells and tree attribute details, as approved on
          2026-09-19. Verify default and prefixed declarations, preserve real
          namespaced attributes and grouping, and keep generic `.attributes`
          and XPath semantics unchanged.
          Five source-contract tests and three source-loaded browser stories
          pass, including namespace edits/reset. The demo verifier passes all
          25 standalone pages and 31 source-loaded documents. QL/browser lint
          and typecheck pass; desktop (1440px, two-card row) and mobile (390px)
          checks show no page overflow. The XPath/CEM-QL paired demo request
          remains queued after full viewer parity and the function audit below.
- [x] Fixture XSLT-VIEW-DEMO: add an independent XSLT viewer case and source
      display to data-table.html. Add an imported XSLT aspect case for notes
      as a tree and the IP-filter local-preview form; retain the CEMT cases.
      Update teaching notes, source guards, stories, inventories and Nx inputs.
- [x] XSLT-VIEW-VERIFY: run native/CLI parity first, then WASM/browser gates,
      standalone/source-loaded demos, lint/typecheck and desktop/mobile checks.
      Verify edits/reset, sorting, selection, aspects, focus and instance state.
      Completion 2026-09-19: environment-defined control-input limits use
      constrain-only CEM scopes, immutable retained policy and worker protocol
      v4. Both independent XSLT gallery cases now cover all four formats,
      sorting/selection, recovery, focus, drafts and imported aspects.
      Verification: 469 CEM-QL tests, 67 focused shared native checks, 18
      scheduler/plugin integration checks, two generated-type checks, 13 CLI
      parity tests and 218 native/WASM bundle checks pass. Browser checks pass
      389 unit tests and 96 stories (the corrected reset expectation was
      rerun in the five-story viewer file). All 25 standalone pages and 31
      source-loaded documents pass; lint/typecheck pass with existing QL
      warnings. Layout at 1440px and 390px has no page/card overflow.
- [x] XPATH-CEMQL-PARITY-AUDIT: after the data viewer is complete, inventory
      the XPath capabilities added for it against native CEM-QL equivalents.
      Record each missing equivalent as an actionable checkitem and implement
      and verify each, as requested on 2026-09-19. Compare semantics, native
      ownership, error behavior and limits, distinguishing native functions
      from optional XPath companions. For subsequent decisions, choose the
      recommended option and continue (user instruction, 2026-09-19). Keep
      external parsing in CEM-ML import.
    - [x] Fixture CEMQL-STRING-IMPORT-PARITY: add `data:parse(source, format,
          options?)` over the shared string-import profiles, with format/options
          resolution only in CEM-ML import. Verify XML/JSON/YAML/CSV, duplicate
          and escape options, CSV headers, Unicode, retained ownership, typed
          recovery, fatal limits and binary/WASM execution; preserve `data:read`.
    - [x] Fixture CEMQL-URI-PARITY: add native `data:base_uri` and
          `data:document_uri`; compare both query languages on the same retained
          tree, inherited XML base URI, absent metadata and invalid arguments.
    - [x] Fixture CEMQL-VIEWER-FUNCTION-AUDIT: record native counterparts for
          grouping, sorting, focus, node access, parsing, provenance, recovery
          and construction. Probe whitespace semantics and add an explicit XML
          whitespace profile where the viewer needs it; preserve default string
          helper semantics.
      Completion 2026-09-19: the [audit](xpath-cem-ql-viewer-parity.md) maps
      every shared viewer capability to native CEM-QL/CEMT, distinguishing
      existing language semantics from interchangeable standard-library APIs.
      Added direct string import with options and typed failures, URI metadata,
      and explicit XML whitespace profiles. The native NBSP probe exposed and
      fixed the CEMT viewer's previous trimming mismatch. All format/option
      resolution remains in CEM-ML import; XML/JSON/YAML/CSV become retained
      native CEM trees. `data:read` and its report/cache contract are unchanged.
      Verification: 475 CEM-QL tests, five import/profile boundary checks,
      13 CLI parity tests and 229 native/WASM checks pass. QL lint passes with
      its existing 23 warnings; browser lint/typecheck pass.
- [x] Fixture XPATH-CEMQL-DEMO-PAIRS: after the function parity audit and gap
      implementations, pair XPath examples on `xpath-functions.html` with
      matching CEM-QL samples and links to detailed CEM-QL function use cases
      (requested 2026-09-19). Update source guards, source-loaded stories and
      relevant fixture inventories; verify the pairs use equivalent inputs,
      results and documented limits.
      Completion 2026-09-19: all three XPath samples have adjacent native
      CEM-QL pairs and links to detailed use-case sections. Native fixtures run
      the authored bodies with edited state and compare Unicode/empty labels,
      predicates, numeric quantities, namespaces, descendant text and errors.
      The demo explains explicit numeric conversion and language differences.
      Verification: 390 browser unit tests plus a focused 16-test source rerun,
      eight browser stories, all 25 standalone pages and 31 source-loaded
      documents, lint and typecheck pass. The demo inventory now matches the
      displayed pair order. Desktop (1440px) shows all three pairs side by side;
      mobile (390px) has no page/card overflow. Native/WASM counts are above.
      The next open viewer fixture is XML-VIEW-1, reviewing the remaining
      lossless XML inspector projection and evidence against the shared tree.

## Immediate: Legacy Demo Case Coverage

The local `~/aWork/custom-element/demo/` comparison found that current-gallery
verification is not a one-to-one legacy case audit. In particular, anonymous
whole-file XSLT loading and XSLT selected by `file.xhtml#id` initially lacked
gallery cases; the existing `embedded-xsl` fixture actually contains CEM-ML.

- [x] Map active legacy cases to current examples and executable coverage,
      distinguishing preserved teaching points, intentional format/semantic
      migrations, missing variants, and commented-out prototype examples.
- [x] Fixture: freeze the local legacy case inventory with source fingerprints
      and validate its current-case/evidence links without requiring the legacy
      checkout in CI. Include unwrapped demos and helper/experimental files.
- [x] Fixture: restore the module-URL prefix-map case that resolves an image
      and a nested same-library fragment together; verify both image URLs,
      rendered nested content, and clickable resource links in standalone and
      source-loaded demos.
- [x] Fixture: restore the active `hex-grid-dce.html` horizontal row and selected
      item teaching point, using semantic current-item state and public styling
      hooks; verify row layout, selection isolation, and keyboard navigation.
    - Completion 2026-09-12: case 9 reuses the existing grid and image-link
      helpers for a fixed-cell horizontal row. Payload `aria-current="page"`,
      a visible ✓, and public current-color properties keep the current page
      distinct from keyboard focus. Native typed-record projection, scoped-CSS
      policy, desktop/mobile geometry, sibling/instance isolation, and real
      Tab/Shift+Tab/Enter navigation are covered. All five unwrapped legacy
      gallery examples now have mappings; the standalone XML viewers remain
      a separate open migration review below.
      Verification: 47 native render tests, 339 unit tests, 152 Storybook
      tests (final full rerun with `--maxWorkers=2`), 17 standalone pages,
      23 source-loaded documents, lint/typecheck, and 1440px/390px layout
      checks pass. Run shared-output Nx build prerequisites without overlap;
      parallel cache restores briefly removed WASM imports in an earlier run.
- [x] Review the standalone `tree.xml` / `table.xml` viewers as separate native
      CEM-ML migration work, including the table inspector's sorting/selection
      lessons. These are not covered by the DCE gallery; do not restore browser
      XSLT or MSXML/EXSLT script execution.
    - [x] Fixture: guard the standalone viewer review and its open native
          migration steps in the case inventory, without promoting the
          scaffold-only legacy sort links to executable coverage.
    - Review completion 2026-09-13:
      [`xml-viewer-migration.md`](../packages/cem-elements/docs/xml-viewer-migration.md)
      records the local source evidence and native boundary. Legacy sorting
      has only a placeholder key/annotations, not an interaction handler;
      selection is independent branch-local CSS, not selectable table rows.
      First-row-only headers and omitted direct row text need deliberate
      improvements. The viewer files remain unported. Execute XML-VIEW-1
      through XML-VIEW-4 below; do not bypass the shared native prerequisites
      with a demo-local parser, grouping function or sort handler.
      Verification: seven case-map guard tests, all 341 unit tests, lint,
      four unchanged legacy fingerprints and review links pass. The resolved
      Nx unit target tracks the review document. No browser UI changed.
- [x] Fixture DATA-IMPORT-1: replace the Rust-owned table projection with a
      reusable declarative data-reading element. Import XML, CSV, YAML and
      JSON through their existing parsers into the typed CEM AST, exposing
      tree navigation, values, source identity and diagnostics to CEMT without
      a JSON/DOM AST handoff. Test native import shapes and negative cases first.
- [x] Fixture DATA-QUERY-1: add the smallest generic CEM-QL grouping and stable
      sorting capabilities needed to query imported CEM AST nodes. Preserve
      native node identity/provenance, expression-defined keys, stable ties,
      numeric/text modes, missing-last direction behavior and evaluator budgets.
      Do not bake row detection, column discovery or table presentation into Rust.
- [x] Fixture DATA-ASPECT-1: expose generic CEMT match-based presentation
      dispatch/override (like xsl:template match), reusing existing template
      matching where possible. Demonstrate additive viewer aspects selecting
      tree versus table for particular imported AST subtrees and a custom
      editable IP-filter form. Keep the base viewer reusable and unchanged by
      data-specific overrides; test match precedence, fallback, instance state
      isolation and form updates. No table- or IP-filter-specific Rust code.
- [x] Fixture DATA-MODULE-1: wire ordinary CEMT declaration imports through
      resolver preflight and the worker/fallback processing engine. Retain the
      complete closure in compilation/cache identity, install imported static
      styles, and diagnose failed imports. Source-only binary cache entries must
      not substitute for a dependency closure. Verify using the imported viewer
      aspects and generic module closure/cache tests.
- [x] Fixture DATA-TABLE-1: extend the requested table demo to XML, CSV, YAML
      and JSON with CEMT-owned transformation of the imported CEM AST. Detect
      repeated sibling/array constructs, derive columns, sort and iterate rows
      in the template; remove the initial Rust data:table implementation.
      Test heterogeneous/nested rows, namespace grouping,
      text/number stable sorting, empty/missing cells, malformed input and
      bounded resources before browser wiring. Add editable examples, selected
      source-row feedback, reset, index/related links, source-loaded stories,
      standalone inventory and desktop/mobile checks. This does not close the
      separate lossless XML tree/disclosure migration below.
      Direction corrected 2026-09-13: the point is demonstrating the CEMT
      transformation, not displaying precomputed Rust tables. The initial
      four-format browser implementation is a working interaction reference,
      not the accepted transformation architecture.
      Completed 2026-09-13: generic native import/group/sort/match primitives,
      CEMT-owned table/tree transforms, imported notes-tree/IP-filter aspects,
      and shared declaration-module loading. Native suites, 349 unit tests,
      all 155 browser stories, 18 standalone/24 source-loaded documents,
      lint/typecheck and desktop/390px/320px overflow checks pass.
- [x] Fixture DATA-TABLE-2: fix repeated-expression text patch identities in
      the shared WASM render adapter. Sorting exposed that repeated source
      frames addressed the first cell instead of each rendered occurrence.
      Preserve source provenance separately from unique text/comment patch
      IDs, retain IDs through serialized patch ingress, and verify sorting,
      sibling isolation, source edits and reset in native/browser tests.
      Also synchronize a textarea's dirty live value when its authored body
      changes, without resetting edits during unrelated renders.
- [x] Fixture XML-VIEW-1: define and verify a lossless, namespace-aware native
      XML viewer projection from the existing typed XML AST. Retain source
      identities/ranges, ordered mixed content, empty attributes, comments and
      inert processing instructions; cover malformed XML, entity rejection
      and resource/depth limits. No browser DOM-to-record or JSON handoff.
  - [x] Fixture XML-VIEW-1-BOUNDARY: verify retained source CEM nodes and typed
        writer output with namespaces, empty attributes, ordered mixed content,
        CDATA, inert processing instructions, source ranges and rejected input.
        Record any presentation gaps and the concrete shared-contract decision
        before changing the writer or source-node fields.
        Native probes preserve import/source evidence and reproduce the typed
        writer's CDATA/PI payload loss. The review's
        [presentation decision](../packages/cem-elements/docs/xml-viewer-migration.md#xml-view-1-typed-inspection--implemented)
        led to the approved shared typed inspection and import-owned PI field
        migration implemented below.
  - [x] Fixture XML-VIEW-1-INSPECT: implement the approved shared typed CEM
        inspection projection and import-owned PI field migration. Preserve
        source arena IDs, exact values, mixed-content order, ranges and owners
        through tabular/terminal writing; verify XML/JSON/YAML/CSV, inert source
        directives, native AST/tree inspection and query-consumer compatibility.
        `cem_tree_inspection` retains the original CEM owner through formatter,
        colorizer and writer artifacts; common source ranges remain distinct
        from XPath coalescing. Public AST/tree text inspection uses this path
        and rejects failed imports. PI names/data are resolved only at import;
        CEMT/XPath viewers retain matching visible output. Grouping and sorting
        are tracked by XML-VIEW-2.
- [x] Fixture XML-VIEW-2: specify and implement the smallest reusable native
      grouping and stable-sort capabilities needed by the table inspector,
      aligned with the planned Tier B sequence helpers. Cover expanded-name
      grouping, text versus numeric keys, missing cells, stable ties, direction
      changes and evaluation budgets before using them in UI declarations.
  - [x] Fixture XML-VIEW-2-AUDIT: verify the existing `seq:group_by`,
        `seq:sorted` and first-seen column union against retained CEM nodes.
        Cover parent-local expanded-name groups, attribute/element distinction,
        original ownership/provenance, both sort directions, empty/missing and
        invalid keys, lowered host-scope budgets and failures without partial
        results. Strengthen native authored-view assertions and document the
        shared contract before adding any new helper.
        Completed 2026-09-20: DATA-QUERY-1 already supplies the required shared
        helpers; no runtime extension was needed. Native tests now prove
        source-owner retention, stable text/number sorting, strict group keys,
        parent-local namespace grouping, first-seen columns and constrained
        host policies. See the [contract audit](../packages/cem-elements/docs/xml-viewer-migration.md#xml-view-2--grouping-and-stable-sorting).
        XML-VIEW-3 is next.
- [x] Fixture XML-VIEW-3: add a separate explicit-source CEM-ML tree viewer
      after XML-VIEW-1. Show editable/reloadable data, keyboard-operable
      disclosure, independent visible branch selection and parse failures;
      keep inspected content inert and CEM-ML the default structural view.
  - [x] Fixture XML-VIEW-3-BOUNDARY: verify whether CEMT can request tabular
        inspection text from its retained CEM owner. Compare the current
        `cemml:format` query surface with the shared typed inspection writer
        for XML/JSON/YAML/CSV, retaining source maps and inert values. Record
        the shared query API decision before implementing the browser viewer.
        Completed 2026-09-20: native CEMT receives the document, but
        `cemml:format(document)` silently returns an empty string. The same
        retained owner produces complete CEM-ML through the shared typed
        inspection writer. The two native probes cover all four formats and
        the existing string pass-through; they characterize the gap, not the
        future contract.
  - [x] Fixture XML-VIEW-3-INSPECTION-API: implement the recommended shared
        retained-document inspection query before browser implementation:
        `cemml:inspect(document)` returns tabular CEM-ML through the existing
        typed projection/writer, with host/scope controls and no source reparse.
        Keep source formatting separate; see the [implemented API](../packages/cem-elements/docs/xml-viewer-migration.md#xml-view-3-inspection-query--implemented).
        Completed 2026-09-20 under the user's recommended-option default.
        Native acceptance covers both retained document views, all four imports,
        escaped CEMT output, invalid input, lowered scope budgets, cancellation
        and discarded writer failures. The production WASM render adapter
        covers XML/JSON/YAML/CSV in `tree-inspection.spec.ts`. Verified 98 native
        checks, the WASM build and all 394 unit checks. Limits cover payload
        sizes; cooperative checks surround the synchronous shared writer.
        The following viewer fixtures complete source editing, disclosure,
        selection and error recovery.
  - [x] Fixture XML-VIEW-3-EDIT: native and browser checks for editable XML
        and JSON sources, typed CEM-ML output, namespace labels, empty values,
        keyboard disclosure and independent branch selections. Use source event
        revisions so edits and reloads clear selections even for identical text.
  - [x] Fixture XML-VIEW-3-CHECKED: verify conditional native checked
        attributes and shared browser patch handling. Adding/removing checked
        must update dirty live checkbox state, retain focus and leave unrelated
        rerenders and controls unchanged.
  - [x] Fixture XML-VIEW-3-ERROR: a separate malformed XML lesson, with repair
        and reload recovery and no stale tree or executable source content.
  - [x] Fixture XML-VIEW-3-REQUEST: resolve repository XML/JSON files relative
        to the declaration and display the loader's retained CEM document with
        the same helper, including lifecycle state and source-change isolation.
  - [x] Fixture XML-VIEW-3-GALLERY: add source contracts, source-loaded stories,
        standalone fixture checks, inventory links, Nx inputs and desktop/two-card
        plus 390px/320px containment checks for `data-tree.html`.
        Completed 2026-09-20: four separate lessons use retained CEM nodes and
        `cemml:inspect`, including direct HTTP document consumption. XML/JSON
        never become JavaScript document objects. Native fixtures also cover
        YAML/CSV through the identical viewer. Source/request revisions clear
        selections, and shared boolean-attribute patches handle dirty checkboxes.
        Native, source-loaded and standalone checks cover repair, independent
        selection, keyboard disclosure, relative URLs and inert processing
        instructions. Desktop two-card and 390px/320px containment checks pass.
        XML-VIEW-4's focused table-inspector lessons follow below.
- [x] Fixture XML-VIEW-4: add the native table-inspector cases after XML-VIEW-1
      and XML-VIEW-2. Union columns across heterogeneous sibling rows, retain
      text-only rows and nested groups, and show real ascending/descending
      sorting with selection keyed to source identity. Verify native results,
      standalone/source-loaded interaction, compact layout and inventory links.
  - [x] Fixture XML-VIEW-4-NATIVE: prove the opt-in CEMT inspector over common
        retained nodes: first-seen heterogeneous columns, text-only rows, nested
        groups, source-keyed multiple selection, stable text/numeric sorting and
        independent table state. Keep the existing CEMT/XSLT comparison behavior.
  - [x] Fixture XML-VIEW-4-CONTROLS: add captions, scoped headings, named sort
        buttons with active `aria-sort`, visible checkbox selection and source
        revision resets using existing declarative slice bindings.
  - [x] Fixture XML-VIEW-4-REBIND: reproduce a dirty checkbox reused for a
        different slice before its prior selection render commits. Patch and
        full reconciliation must apply the new binding's checked state, preserve
        focus and leave unchanged bindings and unrelated dirty controls alone.
  - [x] Fixture XML-VIEW-4-GALLERY: isolate the four inspector lessons on a
        linked page; verify source contracts, source-loaded stories, standalone
        keyboard interaction, sibling/instance isolation, 1440px two-card layout
        and 390px/320px containment. Update migration evidence and Nx inputs.
        Completed 2026-09-20: four lessons use the shared retained CEM table
        projection with opt-in inspector controls. Native source keys preserve
        multiple selections through stable per-table sorting; source revisions
        clear them on edits and identical-source reloads. A shared patch and
        reconciliation fix clears dirty checkbox state when its binding changes.
        All 18 focused native tests and 13 viewer stories pass, including the
        existing CEMT/XSLT parity and tree-viewer regressions. The gallery passes
        27 standalone pages and 33 source-loaded documents. Desktop two-card
        and 390px/320px containment checks, all 401 unit checks, build, lint
        and typecheck pass.
- [x] Fixture DATA-CELL-MATCH-1: add separate `cem-element` samples whose
      own CEMT modules import the existing data-view template and override
      a particular cell's data presentation through CEM-QL template matching.
      This is the immediate task, ahead of Storybook stabilization. Preserve
      the current data-table, table-inspector and XML/tree viewer behavior;
      change their implementation only for the agreed minimum reuse hook.
      Do not implement other legacy XmlView features in this increment.
  - [x] Complete retained-node parent/child navigation through `dom:parent`
        and `dom:children`; compose sibling selection with ordinary filtering.
        Preserve source versus XPath views, native identity, scope rejection,
        cancellation and inherited budgets. Pass the current source node to
        the Pokémon cell and read its sibling `id`. Keep missing/multiple
        values in one table cell. Imported views expose the whole retained
        document; host-restricted views must enforce their scope themselves.
  - [x] Fixture DATA-CELL-NAVIGATION: cover
        parent/children identity and source maps, root/attribute/text/CDATA
        behavior, namespaces, all import formats and XPath views, type errors,
        host scope rejection, budgets and cancellation. Update the earlier
        cell fixtures for node dispatch, sibling IDs, grouped-value fallback
        and the row-selection `th`.
    - [x] Repair `cell_overrides.rs` for sibling IDs and retained output; add
          grouped/missing-cell and source-node identity assertions. Add
          `retained_node_values.rs` for navigation/text/reference/clone across
          XML, JSON, YAML and CSV, XPath views and restricted host failures.
    - [x] Fixture DATA-CELL-SCOPE-CAPS: enforce execution-scope ceilings on
          the evaluator's queue-, CPU- and I/O-derived budgets. A child queue
          cap of four previously permitted twenty navigation results. Preserve
          standalone context configuration, test parent/sibling isolation and
          verify native, WASM and cell-demo consumers after the correction.
          Verified 2026-09-20: 219 focused CEM-QL tests (including 9 revised cell
          cases, 9 retained-node cases and 5 call-budget cases), 96 adapter tests,
          workspace test compilation, WASM/element build, typecheck, lint (two
          existing warnings), 408 runtime unit tests, both native-value WASM
          worker/file/fallback fixtures, and all three cell browser stories.
          All three standalone/source-loaded cell lessons pass. The broader
          viewer run passed 14 of 16 stories; its two stale test selectors are
          recorded under DATA-TABLE-VERIFIER-SELECTORS. Viewer implementations
          remain unchanged. The unit checks passed when Nx asset-dependent
          targets ran serially; overlapping cache restores caused an earlier
          generated-module lookup failure.
  - [x] Implement shared retained-node text conversion for attribute projection
        and explicit `dom:text`; the Pokémon body now reuses its node through
        `{$node}` under the accepted native-value contract below. Preserve
        node-valued selections/parameters and query operators; keep decoded
        source text distinct from XPath's coalesced text. Bound extraction and
        propagate scope, cancellation and budget failures without partial output.
  - [x] Fixture DATA-CELL-NODE-TEXT: verify explicit node text
        and attribute interpolation, nested text/CDATA/whitespace, decoded entities
        and escaping, empty nodes and sequences, selected attributes/comments/PIs,
        CEM source versus XPath text boundaries, and XML/JSON/YAML/CSV imports.
        Cover unchanged native parameter passing and retained results, unsupported
        views, denied scope, cancellation, extraction budgets and lower child-scope
        memory limits; update the Pokémon fixtures for typed `{$node}` insertion
        and explicit `dom:text` extraction.
  - [x] Implement the accepted expression/value direction recorded in
        [the proposal review](./cemt-expression-review.tmp.md), using the shared
        CEM value model and immutable retained source owners.
    - [x] Fixture CEMT-VALUES: `dom:text` explicit/implicit focus, retained
          references and repeated occurrences, explicit deep clone and empty
          element construction, source identity/provenance and all import formats.
    - [x] Fixture CEMT-HOOKS: whole-sequence content/attribute hooks, compact
          declarations, lexical scope, caller/import inheritance, precedence,
          focus restoration, empty versus missing focus and bounded recursion.
      - [x] Fixture CEMT-HOOK-FOCUS: add native cases for mixed attribute
            expression segments, lexical loop bindings, nested query focus,
            fallback while a hook is active and attribute-context restoration.
            Five native cases pass. The rich attribute-body audit exposed the
            output-context choice resolved by the fixture below.
            Verified 54 focused native hook/value, named-constraint, template
            reload/matching and error-recovery tests. This increment changes
            fixtures and documentation only; renderer/viewer behavior is unchanged.
      - [x] Fixture CEMT-HOOK-ATTRIBUTE-BODY: implement the user's selected
            attribute insertion scope (2026-09-21). Direct bodies, transparent
            controls and named/matching/imported calls now use attribute hooks;
            constructed content and nested attributes establish their own
            destinations. Native atomics, node identity, metadata, constraints
            and hook-return capture remain intact. Native result attributes
            retain their scalar/text-spacing semantics. The original probes,
            alternatives and accepted implementation are in the proposal under
            [Expression hooks in attribute bodies](./cemt-expression-review.tmp.md#expression-hooks-in-attribute-bodies-r05r07r12).
            Nine new native cases cover controls/calls, nested constructors,
            destination restoration, artifact reload, errors, recursion and
            cancellation; the real imported-module adapter also exercises a
            typed attribute body. Updated the maintained contract, package
            reference and third cell-override lesson; render engine is 1.5.2.
            Verified 242 focused CEM-QL and 96 adapter tests, workspace test
            compilation, build/typecheck/lint (two existing warnings), 408 unit
            tests, both WASM worker/file/fallback fixtures, all 3 cell-demo
            browser stories and standalone/source-loaded cell lessons. Viewer
            implementations remain unchanged. Deferred gallery/Storybook
            stabilization stays after the immediate expression/value work.
    - [x] Fixture CEMT-ATTRIBUTE-TYPES: shared conversion then validation for
          numeric, temporal, regex-constrained string and rich native values;
          preserve mixed attribute content, destination constraints, native
          component handoff and final-boundary serialization.
      - [x] Fixture CEMT-ATTRIBUTE-MATRIX: verify the documented scalar lexical
            profiles (including large integers, decimal facets, booleans and
            temporal zones), invalid conversion without partial output, and
            mixed native segments through direct and portable component handoff.
            Check final text/HTML/XML projection without changing source values.
            Three shared conversion cases and three native handoff/projection
            cases pass, alongside expanded invalid portable scalar coverage.
            Verified 8 shared-contract and 70 focused CEM-QL tests plus workspace
            test compilation. This increment changes tests and documentation;
            production code and viewer templates are unchanged.
            The query-behavior audit found the separate decision resolved by
            the large-integer fixture below.
      - [x] Fixture CEMT-LARGE-INTEGER: implement the user's selected decimal
            evaluation for integers outside `i64`, preserving integer datatype
            metadata (2026-09-21). Align direct typed values, portable values,
            receiver conversion and hook returns. The reproduction and bounded
            decimal-adapter correction are in
            [Large integer query representation](./cemt-expression-review.tmp.md#large-integer-query-representation-r06r08).
            Add type/arithmetic, signed boundary, overflow, artifact and worker
            regressions. Direct typed values and portable values now share the
            integer-to-atom mapping. Four new native cases verify both signed
            `i64` boundaries, exact decimal arithmetic, query type checks,
            receiver/hook conversion, retained datatype/source metadata,
            template reload, mixed-type errors and unchanged decimal limits.
            The WASM fixture checks direct evaluation plus larger integers
            across separate workers, saved files and fallback. Verified 175
            focused CEM-QL and 96 adapter tests, workspace test compilation,
            build/typecheck/lint (two existing warnings), 408 runtime unit tests,
            both WASM fixtures and all 3 cell-demo browser stories. Render engine
            is 1.5.3; the artifact format and viewer implementations are unchanged.
      - [x] Fixture CEMT-NAMED-CONSTRAINTS: compose inherited and local named-type
            restrictions in the shared native contract. CEMV version 3 preserves
            every constraint, retains version-1/version-2 reads, and rejects
            restrictions under old headers. Native coverage includes the two
            original regressions, accepted narrowing, conflicting/malformed
            facets, strongest whitespace normalization, final-value validation
            across host/receiver/hooks, clone/XPath/result preservation, tampered
            graphs and cancellation/metadata limits. Verified 5 shared-contract,
            162 focused CEM-QL (including 7 new constraint cases) and 96 adapter
            tests, workspace test compilation, build, typecheck, lint (two existing
            warnings), 408 runtime unit tests and 3 cell-demo browser stories.
            Both WASM fixtures pass; `native-constraints-wasm.mjs` generates
            its binary fixtures in Rust and checks saved-file, separate-worker
            and fallback validation without JavaScript document objects.
    - [x] Fixture CEMT-VALUE-PIPELINE: native intermediate output, reference
          navigation and lifetime, artifact round trips, cancellation and
          environment-defined limits with scope overrides that only lower them.
      - [x] Fixture CEMT-PIPELINE-LIFETIME: verify multiple native render stages
            and artifact round trips preserve reference targets, occurrence
            parents and typed attributes; retain live XPath results after input
            handles are released, then release memory with the last result.
            Check fresh and cached projections under lower child-scope limits,
            and cancelled/failed stages without partial output or retained permits.
            The audit reproduced a cached-depth bypass for both constructed
            and portable values. Shared CEM-ML cache admission now revalidates
            lowered limits without rebuilding the index. Five new CEM-QL cases
            and an expanded shared projection fixture cover lifetime, multi-stage
            transport, cancellation, cleanup and lower limits. Verified 2026-09-21:
            136 focused CEM-QL tests, 96 adapter tests, 2 shared projection tests,
            workspace test compilation, build/typecheck/lint (two existing
            warnings), 408 runtime unit tests, both WASM transport fixtures and
            all 3 cell-demo browser stories. Render engine is 1.5.4.
      - [x] Fixture CEMT-READER-NODE-INSERTION: migrate the two stale XPath
            reader render expectations to retained subtree insertion; preserve
            their matching, source-change and reader-owner lifetime assertions.
    - [x] Fixture CEMT-VALUE-TRANSPORT: portable native CEM value artifacts
          across workers, deterministic fallback and saved pipelines. The user
          selected this R08 direction during implementation. Preserve shared
          targets, source parents/provenance and typed attribute contracts;
          reject invalid graphs and apply host/scope limits. Keep document data
          binary/native, with JSON only for explicitly named control metadata.
          Native worker/file/fallback round trips pass in
          `packages/cem_ql/tests/native-values-wasm.mjs`; native scalar, reference,
          source-key and graph-limit cases are in `tests/portable_values.rs`.
          Browser patch and saved-envelope cases are in `native-values.spec.ts`.
    - [x] Complete explicit CEM-QL template dispatch; keep this distinct from the
          settled R08 transport and the completed contract cases below.
      - [x] Fixture CEMT-QUERY-DISPATCH: native results from
            `cemt:apply_templates(values, mode)`, shared match precedence, imported
            rules and hooks, focus restoration, artifact reload, missing host,
            error propagation and bounded recursive query/template calls.
      - [x] Fixture CEMT-HOOK-INHERITANCE: imported defaults, caller overrides,
            local priority/ties, native module calls, reload and failing predicates.
      - [x] Fixture CEMT-RECEIVER-CONTRACT: receiver declarations validate native
            and scalar inputs, preserve rich nodes, reject weaker sender contracts,
            and behave identically after worker/file/fallback transport.
      - [x] Fixture CEMT-CONSTRUCTED-XPATH: query constructed and portable values
            directly through shared CEM projection, preserving occurrence parents,
            repeated identity, namespaces, attribute values and bounded expansion.
            Migrate the native XPath callback integration to the shared node adapter.
    - [x] Finish scope/cancellation accounting for deferred text/markup projection
          and native artifact export; cover denied host views without partial text
          and account for retained XPath indexes across query scopes.
      - [x] Fixture CEMT-VALUE-CONTROL: lowered text/memory budgets, scoped denied
            text and export, cancellation without partial projections/artifacts,
            and bounded retained XPath indexes with released memory permits.
      - [x] Fixture CEMT-VALUE-METADATA: include retained source/provenance and
            attribute-contract payloads in graph/index memory estimates.
      - [x] Fixture CEMT-DEMO-VERIFIER: migrate the stale Pokémon edit fixture
            from `pokemon-id` attributes to sibling IDs and include the native
            attribute handoff lesson in standalone/source-loaded verification.
          Shared checked projection now feeds HTML/XML and browser DOM patch
          export; native intermediate values remain retained. Graph validation
          polls control, text accepts owned atomic segments, and query recovery
          cannot suppress projection control failures. The output owner retains
          only its latest query scope's XPath index; live result nodes retain its
          memory permit after eviction. `dom:element` constructs a shell directly.
    - [x] Update maintained syntax, acceptance criteria, package references and
          cell-overrides lessons; verify focused native then WASM/browser cases.
          Verified after the contract follow-up: 142 focused CEM-QL tests,
          96 transformation-adapter tests, the shared projection limit/cancellation
          fixture and native XPath callback integration; workspace test compilation,
          WASM/element build, the real worker/file/fallback fixture (including
          receiver rejection, XPath and empty sequences), 408 runtime unit tests
          and 3 cell-override browser stories.
          The follow-up adds explicit dispatch in the third cell-override lesson
          and the worker/file/fallback native-value fixture. Native checks cover
          155 focused CEM-QL tests, 96 adapter tests and workspace compilation.
          Final WASM worker/file/fallback, all three focused browser stories,
          and all three standalone/source-loaded cell-override lessons pass.
          All 408 runtime unit tests, build, typecheck and lint also pass
          (lint retains its two existing non-null-assertion warnings).
          The full gallery verifier stops at the pre-existing data-table selector
          issue recorded under DATA-TABLE-VERIFIER-SELECTORS below.
          The full expression proposal remains open at the cases listed above.
    - [x] Polish DX for text, reference, clone, element and explicit template
          dispatch operations; consider matching compact forms for other template
          types as requested during review. Do not add confusing copy/copy-of aliases.
      - [x] Fixture CEMT-CONSTRUCTOR-REFERENCES: audit `dom:clone` and
            `dom:element` with explicit reference inputs before and after CEMV
            transport. Resolve target-following versus reference-node behavior
            before changing result kind, cardinality or repeated-target identity;
            then add native and worker regressions for the selected contract.
            The 2026-09-21 audit reproduced two failures: direct `clone` returns
            two independent elements where portable `clone` returns one reference
            retaining target aliases; direct `element` accepts the reference
            where portable `element` raises a type error. The user selected
            preserving reference nodes in `clone` and requiring explicit
            `.targets` selection for element shells. Implement the selected
            [constructor contract](./cemt-expression-review.tmp.md#constructor-reference-inputs-r01r09r11),
            including scope checks without retaining source ancestors.
            Completed 2026-09-21: six native cases cover direct/constructed/CEMV
            references, aliasing, detached roots, provenance, scalar/empty/nested
            targets, all four imports, scope denial, lowered limits and cancellation.
            Both WASM worker/file/fallback fixtures pass with constructor parity.
            The operation guide documents `.targets` selection and migration;
            no aliases or new element-construction overloads were introduced.
      - [x] Implement implicit CEMT modules and default bodies, retaining explicit
            wrappers and rejecting mixed/duplicate bodies. Allow anonymous match
            declarations, align shared preflight/native/browser execution, and
            simplify the inline Pokémon sample. Approved 2026-09-21; leave test
            files unchanged for this increment and verify with existing coverage.
            Shared preflight and rendering support implicit imports and anonymous
            matches; the native adapter accepts direct module bodies. The sample
            omits both wrappers and its unused name. Existing checks pass: nine
            native cell cases, 19 renderer cases, 96 adapter cases, 413 runtime
            unit cases, five cell browser stories, WASM import/body probes,
            build, typecheck and lint (two existing warnings). The real gallery
            cards fit two columns at 1440px and have no overflow at 390px/320px.
            The full gallery verifier passes all 28 standalone pages and 34
            source-loaded documents. Test files remain unchanged.
        - [x] Fixture CEMT-IMPLICIT-MODULES: add permanent parser, renderer,
              adapter and browser coverage for implicit imports, anonymous
              matches, explicit/direct body parity and invalid mixtures in the
              next test-update increment (initially deferred at the user's request).
              Completed 2026-09-21: updated the retired mandatory-wrapper/name
              assertions and covered all implicit/explicit body forms. The new
              adapter fixture reproduced an import declaration emitted as HTML
              by explicit modules without a body wrapper. Default-entrypoint
              extraction now preserves the module boundary and excludes document
              directives. Native cases cover anonymous rule scopes, private
              imports, call source locations and rejection without partial output.
              Browser cases verify all four module/body forms, reactive switching
              between local overrides and imported fallback, and invalid layouts.
              Verification: 147 focused Rust tests (including all 97 adapter
              tests), seven cell browser stories and three source checks pass.
              WASM dependencies rebuild successfully; typecheck and lint pass
              with the same two existing lint warnings.
      - [x] Fixture CEMT-COMPACT-TEMPLATES: implement the user-approved compact
            module-body grammar (2026-09-21). Align shared module preflight and
            direct compilation, preserve parameters and visibility, and reject
            mixed/duplicate bodies. Native parser/renderer fixtures now cover
            text, comments, call/encoding collection, source positions, private
            imports and unchanged transform-function syntax. The real adapter
            uses compact imported entrypoints; the Pokémon demo uses implicit
            `node` and direct content. Downstream WASM preflight and browser
            checks pass. The selected contract is recorded in
            [the review](./cemt-expression-review.tmp.md#compact-module-template-bodies-r11r12).
        - [x] Migrate the two hand-built source-map renderer fixtures to populate
              native attribute value streams. They set only the final text field,
              so native projection emitted empty attributes. Their expected
              HTML/XML and source-map assertions are unchanged and pass.
            Completed 2026-09-21: 23 focused CEM-ML tests, 133 CEM-QL tests,
            96 adapter tests and workspace test compilation pass. Build,
            typecheck, all 408 runtime unit tests and lint pass (two existing
            warnings). The new compact-body WASM fixture and both worker/file/
            fallback fixtures pass, as do all three browser stories and focused
            standalone/source-loaded interactions. The real cards pass 1440px
            two-column and 390px/320px containment checks. Base viewer code is
            unchanged. DATA-CELL-MATCH-1 and its expression/DX work are complete;
            proceed to the deferred Storybook stabilization below.
  - [x] Inspect current dispatch and local legacy collapse code; record the
        findings in the [presentation override discussion](../packages/cem-elements/docs/xml-viewer-migration.md#cell-presentation-overrides).
        Current tables already use native disclosure. Legacy tree branches
        implement collapse, while the table caption still says `todo collapsible`.
        Child-element cells already dispatch their retained source nodes through
        `inspect`; those can demonstrate overrides without changing the base.
  - [x] Discuss small cell features before implementation. Approved: Pokémon
        image plus name, followed by a separate zero-stock warning/fallback
        lesson. The follow-up requires the first template inline and a primitive
        name match. The later approved cell-generation hook dispatches the
        retained source node and uses shared parent navigation for sibling IDs.
        Other candidate features stay deferred.
  - [x] Prove the revised import/match/delegation contract in native tests:
        the selected cell uses the local rule, unmatched and nested content
        retain base rendering, and rows stay attached to the imported CEM tree.
        Preserve default viewer output and CEMT/XSLT parity if a hook is needed.
  - [x] Author the isolated samples and their own importing templates. Verify
        the chosen cell output, match scope and fallback, source-loaded and
        standalone operation, compact layout, source contracts and inventory/cache
        inputs. Table collapse remains an alternative, not an approved refactor.
        Source contracts, source-loaded/standalone interactions and the real
        demo-card layout checks pass.
    - [x] Fixture DATA-CELL-COMPACT-LAYOUT: check the real demo cards at 1440px
          for two-column presentation and at 390px/320px for containment, in both
          standalone and source-loaded pages. Keep viewer implementations intact.
          Verified 2026-09-21: all three real demo cards render, the first two
          share a 1440px row, and 390px/320px pages have no horizontal overflow.
          All three cell stories and standalone/source-loaded interaction checks
          pass. The final native run covers 120 focused CEM-QL tests, 96 adapter
          tests and workspace test compilation. Build, typecheck, 408 runtime
          unit tests, both WASM transport fixtures and lint pass (two existing
          warnings). The full gallery's stale selectors remain deferred below.
          No base viewer implementation changes were needed.
  - [x] Fixture DATA-CELL-MATCH-POKEMON: inline the sample's template and
        match only the local name `name` in `cell` mode, without an explicit
        priority. Render an image plus name using the sibling `id` element.
        Other field names use the imported renderer. Revised native fixtures,
        image loading, sorting, node dispatch and source repair checks pass.
  - [x] Fixture DATA-CELL-MATCH-STOCK: override only zero stock in catalog
        product rows; verify positive values, unrelated fields and edited values
        use the imported fallback, without changing the retained source.
- [x] Fixture DATA-TABLE-VERIFIER-SELECTORS: during the deferred browser
      stabilization, migrate the gallery verifier's `tableInteractions` selectors
      from `td > button` to the existing `th > button` row-selection controls.
      The full gallery run stopped on this stale selector in `data-table.html`;
      both the selector and the viewer's `th` predate this follow-up. Keep the
      current data-table/XML viewer implementation unchanged.
      The 2026-09-20 value audit also ran the 16 cell/table/tree/inspector stories:
      14 passed, including all three cell stories and both native-XSLT stories.
      Fix the two additional stale row-heading assumptions in the same cleanup:
      `data-table-demo.stories.ts` / EveryAuthoredSample indexes `:scope > td`
      with an index that still counts the selection cell (reads names as IDs);
      `table-inspector-demo.stories.ts` / ColumnsAndText queries all `th` instead
      of `thead th` (includes two row-selection headings). These are fixture
      migrations, not requests to change the viewer's output.
      Completed 2026-09-21: the corrected selectors pass in Storybook and in a
      focused run of the gallery verifier covering data-table and cell-overrides
      as two standalone pages and two source-loaded documents. At that stage the
      XSLT sample retained its selection `td` through a helper exception; the
      subsequently approved heading parity work below removes that exception.
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
  - [x] Investigate failed source acquisition and recovery: HTTP 503 and
        interrupted response bodies both remain cached as rejected promises;
        original and replacement declarations make no new request after the
        source recovers. Both real/helper stock galleries reproduce failure
        after remount and recover after document reload. Record the distinction
        between this proven recovery defect and the unattributed historical timeout.
  - [x] Fixture SRC-FAILURE-RETRY: implement the user-approved explicit
        registration/reconnect retry after failed declaration acquisition. Use the
        [recovery contract](browser-stabilization-review.tmp.md#accepted-contract-retry-on-explicit-registration-or-reconnect)
        with browser coverage for original/replacement declarations, interrupted
        bodies, concurrent consumers, successful cache reuse, scope disposal and
        the authored stock gallery. Do not add automatic retries or alter source
        parsing, URL resolution or base viewer templates as part of this proposal.
        Approved 2026-09-21: preserve successful caches, share pending requests,
        retain diagnostic history, then investigate the startup issue again.
        Completed 2026-09-21: rejected loads are evicted and failed owners can
        retry. All 203 browser stories, 441 unit tests, and the full gallery
        (29 standalone pages / 35 source-loaded documents) pass. Build, types
        and lint pass; lint retains its two existing warnings.
  - [x] Investigate stock startup after SRC-FAILURE-RETRY with actual browser
        request aborts, interrupted HTTP bodies, and remount/reconnect while a
        source is pending. Compare the gallery helper and real demo component;
        capture requests, declarations, styles, diagnostics and warning output.
        Keep the historical timeout open unless its cause is established.
        Completed 2026-09-21: all eight controlled cases pass on the rebuilt
        runtime, including warning/style recovery and editing after remount.
        Pending owners share one request. The historical cause remains unknown;
        further import/worker probes are recorded in the
        [renewed investigation](browser-stabilization-review.tmp.md#renewed-startup-investigation-after-the-fix).
  - [x] Probe stock-only imported-template failures and pending loads using a
        host resolver, with the authored source unchanged. Capture processing
        traces and recovery after remount to distinguish source registration
        from module compilation and instance rendering.
        Completed 2026-09-21: four import cases and two gated worker-compilation
        cases pass with the helper and real demo component. Import failure is
        diagnosed on the defined instance; pending work recovers after remount
        and release. No worker fallback or overflow occurs. The historical
        timeout remains unattributed.
  - [x] Measure stock request, worker-queue, compile and render timing under
        parallel load using phase traces. Investigate unexplained delays without
        increasing waits or changing viewer templates; keep attribution to the
        historical timeout separate from any newly reproduced issue.
        Retain a repeatable diagnostic probe with source/build hashes and
        environment metadata; compare helper/real cards at increasing page
        concurrency and alongside the full Storybook suite.
        Completed 2026-09-21: all 122 timing probes pass across baseline,
        four/eight-page and three combined-load runs. Warning times range from
        1.36 to 11.99 seconds; critical source/runtime hashes match. The repeated
        full-suite load exposes fixture readiness/count bugs tracked below,
        without reproducing the historical 45-second stock stall. See the
        [timing report](browser-stabilization-review.tmp.md#startup-timing-under-parallel-load).
  - [x] Fixture startup settlement: investigate the combined-load failures in
        form EveryAuthoredSample (300-frame startup cutoff), table Matching
        PresentationAspects (new instance), and NativeXsltViewer (reset). Use
        declaration/render settlement before their existing assertions; preserve
        test budgets and viewer behavior, then repeat the combined-load check.
        The repeat exposes the same cutoffs in data-slices startup and additional
        table/XPath-sort interactions. Apply the same lifecycle waits to those
        transitions; do not change their expected data, ordering or time budgets.
        Settlement exposes a separate data-slices assertion bug: sixteen samples
        contain eighteen instances, so the old equality-to-sixteen condition can
        miss its transient window. Assert each legend's authored instance count.
        The synchronized repeat also exposes a 240-frame inline-substrate cutoff;
        await its own runtime's declaration/render settlement before assertions.
        Completed 2026-09-21: source-page/instance settlement replaces those
        incidental waits, and per-legend counts match all eighteen slice
        instances. All eight focused cases and the normal 203-story suite pass;
        lint passes with its two existing warnings. Full-budget stress failures
        remain a separate investigation below.
  - [x] Timestamp setup and interaction/settlement phases in the table
        EveryAuthoredSample, tree EditingSelectionAndDisclosure and inspector
        MultipleSelectionAndRecovery stories under combined load. They exhaust
        their full 30-second budgets; distinguish slow progress from unsettled
        work before changing story organization, budgets or concurrency policy.
        Fixture instrumentation: add opt-in phase/settlement observations to
        those three existing journeys, retaining incremental output on timeout;
        compare the normal full suite with synchronized eight-page probe load
        without changing waits, assertions, story budgets or runtime behavior.
        Completed 2026-09-21: opt-in observations retain setup/action/assertion,
        lifecycle settlement and connection state across timeout. Normal suite
        passes 203/203; combined runs pass 201/203 and 200/203, with all 64 stock
        probes passing. Table setup takes 24–26 seconds; inspector progresses
        through recovery near its deadline. Tree's second selection has no
        verified live result before timeout; its late settlement follows
        detachment. Post-timeout play continuation is not passing coverage.
        See [phase findings](browser-stabilization-review.tmp.md#long-story-phase-investigation).
  - [x] Fixture: trace the tree's second-selection scheduling, worker response,
        render revision and selected-count output while still connected under
        the same combined load. Distinguish delayed processing from stale live
        output before changing story organization, budgets or concurrency.
        Track timeout/detachment and post-timeout play continuation separately;
        do not treat settlement on a detached viewer as successful recovery.
        Completed 2026-09-22: opt-in worker/scheduler and DOM observations show
        a successful connected revision 3/count 2 in both combined-load runs.
        In the repeat, selection waits 5.064 seconds behind the remote-document
        viewer, then spends 4.083 seconds in the worker round trip; DOM output
        follows 17.4 ms later. Full journeys still exceed their 30-second budget.
        Normal suite passes 203/203; each stress run passes 201/203 and all 64
        stock probes pass. No lost selection or runtime fix is established.
  - [x] Decide the next response to measured rendering/queue pressure before
        changing story organization, budgets or concurrency. Recommend a bounded
        worker/native rendering profile; compare with splitting the long stories
        in [the proposal](browser-stabilization-review.tmp.md#pending-decision-rendering-profile-or-story-split).
        Accepted 2026-09-22: profile the worker/native rendering path first.
  - [x] Profiling fixture: reuse the authored XML tree and local request document
        to measure initial/warmed CEM-ML import, inspection, branch rendering,
        result conversion/diff and worker transport. Retain repeatable native
        and browser probes with correctness assertions; keep viewer templates,
        execution budgets and concurrency unchanged. Propose a measured bounded
        correction before changing shared evaluator/rendering behavior.
        Completed 2026-09-22: native and worker probes identify conversion
        registry construction inside `cemml:inspect` as the dominant cost.
        Every assembly rebuilds the schema registry thirty-six times. A
        profiling-only candidate using one local registry reduces assembly
        from 529–540 ms to 46–51 ms while preserving converter/artifact metadata
        and exact typed inspection output. Rust probe and all eighteen browser
        connected-count assertions pass; production code is unchanged.
        See [profile findings](browser-stabilization-review.tmp.md#workernative-tree-rendering-profile).
  - [x] Decide the bounded shared conversion-registry correction before
        implementation. Recommend one local schema registry per assembly,
        preserving package validation, ordering, formatter selection and owned
        registry lifetimes. Compare with caching immutable built-ins in
        [the proposal](browser-stabilization-review.tmp.md#pending-decision-built-in-conversion-registry-assembly).
        Accepted 2026-09-22: reuse one local schema registry per assembly.
  - [x] Fixture and implementation: add focused native metadata/output/isolation
        regression coverage first, implement the accepted local-registry
        correction, rebuild WASM/browser assets,
        then rerun the profiles and normal/combined-load Storybook gates without
        changing viewers, time budgets or concurrency.
        Completed 2026-09-22: one local registry resolves all eighteen packages;
        registration order, validation and independent ownership are preserved.
        All 160 conversion, five loader and ten inspection/viewer tests pass.
        Native full renders improve from 561–584 ms to 80–85 ms; refreshed
        browser round trips improve from 1.19/1.23 s to 0.34/0.74 s. Normal
        Storybook passes 203/203. Two combined-load runs pass 202/203 plus all
        64 stock probes: the tree completes within budget, but the table still
        times out. Overall stabilization remains open; see
        [the correction and remaining gate](browser-stabilization-review.tmp.md#local-schema-registry-correction).
  - [x] Profiling fixture: investigate the remaining table startup/sorting
        timeout using the authored multi-format page and native
        `data_view_templates.rs` fixtures. Separate compilation, CEM import,
        row/column selection, sorting and output from worker queue/setup time,
        including competing authored instances. Propose a measured bounded
        correction before further shared behavior changes; retain viewer
        templates, story coverage, time budgets and concurrency.
        Completed 2026-09-22: two native profiling fixtures and 98 connected
        browser checks pass. Unread host-control metadata raises an identical
        native table render from 4.953 ms to 350.921 ms. Browser probes separate
        large island bindings and repeated WASM rendering from two cold XSLT
        compilations and their worker queue. Production code remains unchanged;
        see [table findings](browser-stabilization-review.tmp.md#table-startup-and-sorting-profile).
  - [x] Decide the bounded shared binding-copy correction before implementation.
        Recommend copying only compiler-proven expression dependencies, with
        complete context retained for opaque/native callbacks. Preserve public
        `island`/`datadom.island` access, captures, focus and scope contracts; see
        [the proposal](browser-stabilization-review.tmp.md#pending-decision-expression-binding-copies).
        Accepted 2026-09-22: reduce unused binding copies with conservative
        full-context fallback.
  - [x] Fixture and implementation, after that decision: add native regression
        coverage for captures, shadowing, recovery, indirect callback access,
        scope/limits and exact output; implement the accepted dependency-copy
        correction, rebuild WASM/browser assets, then repeat profiles and normal
        and combined-load Storybook gates. Keep viewer sources, coverage,
        budgets and concurrency unchanged. The diagnostic island omission is
        not a production solution or a passing stabilization gate.
        Completed 2026-09-22: CEMT and CEM-QL derive binding dependencies from
        existing IR and retain full context for opaque calls. All 628 native
        tests, both release profile tests, 58 browser profile checks and normal
        Storybook (203/203) pass. Native loaded rendering improves 351 -> 74 ms;
        authored browser rendering improves 330–480 -> 69–99 ms with unchanged
        inputs. Combined-load gates remain open (201/203, 202/203 with late
        overlap, 200/203 synchronized); all 96 stock probes pass. See
        [implementation and remaining gates](browser-stabilization-review.tmp.md#expression-binding-selection-implementation).
  - [x] Profiling fixture: attribute remaining table startup and rendering under
        synchronized load. Separate cold XSLT compilation/worker queue time
        from copies of complete selected records and template scopes. Retain
        native import, public bindings, viewer sources, coverage, budgets and
        concurrency; propose measured bounded changes before modifying another
        shared capability. A 27.469 s passing table journey is not sufficient:
        the synchronized repeat still exceeds 30 s after 22.237 s of setup.
        Add opt-in native profiles for XSLT preflight/name resolution, bundle
        construction/reload and hook-free interpolation with unread controls;
        trace table and affected story declaration/worker readiness under the
        existing synchronized stock load, without changing their waits.
        Completed 2026-09-22: four native profiles, forty browser checks and
        all 64 stock probes pass. Normal Storybook is 203/203; synchronized load
        is 202/203 with an external-src wait expiring 49 ms before clean render
        settlement. The table takes 29.213 s; cold XSLT jobs delay cached CEMT
        compilation by up to 11.828 s in that suite. Hook-free interpolation
        still clones the whole context (0.237 -> 11.346 ms with unread controls).
        See [attribution and limits](browser-stabilization-review.tmp.md#remaining-startup-and-readiness-attribution).
  - [x] Decide the bounded no-eligible-expression-hook correction before shared
        renderer changes. Recommend returning the unchanged input stream before
        saving scope/bindings/focus when no visible destination-matching,
        non-active hook can run. Preserve evaluation of every eligible predicate;
        see [the proposal](browser-stabilization-review.tmp.md#pending-decision-expression-hooks-with-no-eligible-handler).
        Accepted 2026-09-22: skip copies when no hook can run.
  - [x] Fixture and implementation after that decision: test empty/opposite/active
        hook eligibility, complete native streams/identity, lexical captures,
        priority, typed attributes, recovery, depth and portable reload; implement
        the approved guard and repeat native/browser profiles and normal plus
        synchronized Storybook gates. Retain viewer sources, budgets, concurrency
        and public bindings. Keep selected-record copying and cold XSLT
        compilation as separate profiling tasks.
        Completed 2026-09-22: an allocation regression reproduces the old copy;
        the bounded guard preserves whole streams/context when no hook is
        eligible. All 641 native tests, four release profiles and 98 browser
        profile checks pass. Loaded interpolation improves 11.346 -> 0.287 ms;
        loaded native table rendering improves 73.496 -> 34.299 ms. Normal
        Storybook initially exposes two retry-wait failures (201/203); the retry
        subset and full repeat pass (4/4, 203/203). Synchronized Storybook passes
        203/203 with table completion at 28.264 s; all 64 stock probes pass.
        Overall stabilization remains open; see
        [implementation and validation](browser-stabilization-review.tmp.md#expression-hook-eligibility-implementation).
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
  - [x] Fixture: replace external-src's reproduced premature frame wait with
        declaration and render settlement, then assert every existing output
        within the unchanged story deadline. Add opt-in retry-attempt and
        assertion-failure snapshots with render/worker observations. After
        reproducing the retry deadline during an in-flight render, migrate the
        shared recovery assertion to render settlement too. Retain NPM and
        location waits until their failure state is captured.
        Verify focused, normal and synchronized Storybook coverage.
        Completed 2026-09-22: the original retry wait fails with a successful
        acquisition and an in-flight render; after lifecycle migration the
        focused concurrent run passes 4/4 despite a 1.327 s declaration-to-render
        gap. Final normal and synchronized suites pass 203/203 each; all 160
        stock probes pass. Viewer/runtime sources, story limits and concurrency
        are unchanged. Table completion under final load is 28.600 s, so startup
        headroom and the remaining historical failures stay open.
  - [x] Profiling fixture: separate cold XSLT lowering, XPath compilation,
        generated CEMT compilation and bundle assembly/validation before choosing
        a compiler optimization. Retain source maps, diagnostics,
        artifact validation, public bindings, native import and viewer behavior;
        propose measured bounded changes before further shared implementation.
        Current fixture: opt-in thread-local Rust test spans around the actual
        compiler stages; verify byte-identical profiled/unprofiled bundles and
        native rendering before proposing a production correction.
        Compare test-only type-surface setup candidates against complete
        signature maps, imported prefixes, existing host declarations, inferred
        types and diagnostics under strict/development configurations.
        Completed 2026-09-22: release profiles attribute 425/702 ms of 493/822 ms
        base/aspect bundle compilation to 959/1,559 CEM-QL type checks. Two
        test-only setup candidates preserve checked contracts; a per-compilation
        prepared baseline reduces the 128-setup median from 53.819 to 44.105 ms.
        No production optimization is implemented; see the
        [type-surface proposal](browser-stabilization-review.tmp.md#pending-decision-prepared-type-checking-baseline).
  - [ ] Decide the shared compiler correction: recommend one lazily initialized,
        owned built-in type-checker baseline per CEMT compilation, cloned before
        adding each expression's bindings, imports and declarations. Preserve
        the complete public function table, diagnostics/configuration and artifact bytes;
        do not share mutable checkers or introduce a process-global cache.
  - [ ] Fixture and implementation after that decision: validate checker
        isolation, import aliases, local function/variable overrides, type
        configuration, source maps, diagnostics and portable artifact equality;
        implement the accepted bounded setup reuse, repeat native stage profiles,
        rebuild WASM, and verify normal plus synchronized browser coverage.
        Measure complete compiler/browser improvement rather than projecting the
        microbenchmark percentage. Keep viewer sources, limits and concurrency.
  - [ ] Profile remaining complete selected-record and template-scope copies
        independently. Preserve native identities, all public binding access and
        opaque-callback context; propose measured bounded changes before shared
        evaluator/renderer implementation.
  - [x] Fixture: trace source-loaded stock startup with controlled declaration
        and sample-mount timing. Distinguish missing sample mounting, lost test
        selectors and failed declaration loading; retain a delayed-load
        regression and useful failure evidence without changing base viewers.
        Completed 2026-09-21: twelve controlled source-loaded runs pass, including
        delayed stock loading during Pokémon edits. A permanent story with the
        real demo cards verifies pending registration, stylesheet installation,
        initial warning, retained input and subsequent editing. All 191 browser
        stories and all 28 standalone/34 source-loaded gallery checks pass.
        Failure snapshots now include sample mount state, fixture
        markers, declaration registration/styles and runtime diagnostics. An
        injected 503 confirms that the missing-warning snapshot reports the
        actual source-load failure. The historical timeout remains unreproduced;
        no runtime/viewer fix is claimed. Evidence is recorded in the
        [stabilization review](browser-stabilization-review.tmp.md#stock-startup-follow-up).
- [x] Fixture: check declaration-owned stylesheet lifecycle across repeated
      source-loaded gallery mounts in one document. A second hex-grid story
      using the same source registration rendered unstyled links; isolate
      registration reuse and unmount/remount cleanup from startup timing.
      2026-09-21: reproduced after explicit render/declaration settlement and
      with the real source-loaded hex gallery (four styles on first mount, zero
      on second). Same-scope remounts reject as duplicates; compatible fresh
      scopes still attach styles to the detached original owner. Accepted on
      2026-09-21: restore live stylesheet ownership on identical remounts, as in
      [browser stabilization review](browser-stabilization-review.tmp.md).
      Completed 2026-09-21: one stylesheet set moves between accepted live
      owners and detaches after the last owner leaves. Six browser lifecycle
      cases cover repeated source-loaded mounts, strict node identity, compatible
      aliases, reconnect, concurrent/incompatible declarations, scope/ancestor
      disposal, delayed loading, batched manual connections and document adoption.
      Named CSS scope now participates in registration identity. Build, typecheck,
      all 410 unit tests and lint pass (two existing warnings). The 12 focused
      CSS/registration/lifecycle stories pass; the final full suite passes 185/188,
      with the same three CEM-QL/local-storage failures tracked below. Base data
      table and XML/tree viewer templates remain unchanged. The packaged hex
      gallery also passes its standalone and source-loaded interaction checks.
  - [x] Fixture: prove the identical detached-remount exception in the pure
        registration decision core; preserve concurrent duplicates, incompatible
        identities and browser collisions.
  - [x] Fixture: cover stylesheet identity and handoff on remount, original-owner
        reconnect, cross-scope aliases, last-owner removal, manual registration,
        and scope/ancestor disposal without restoring disposed processing scopes.
  - [x] Fixture: mount the source-loaded hex gallery repeatedly in one document
        and assert computed styles, a single stylesheet set, and clean diagnostics.
  - [x] Fixture: retain mount evidence when external source loading finishes
        after removal, and when a manually registered owner mounts and unmounts
        before the mutation-observer checkpoint.
  - [x] Fixture: adopting a retained declaration into another document must
        release its styles, reject cross-document ownership, and allow a valid
        identical remount in the original document.
  - [x] Fixture: include the declaration's named CSS scope in registration
        identity so a remount or cross-scope alias cannot reuse differently
        scoped styles under otherwise identical template bytes.
- [x] Fixture: during Storybook stabilization, preserve authored source in real
      source-loaded `cem-demo-element` cards. The cell layout audit shows runtime
      `data-cem-render-node-id` and artifact/revision attributes in displayed source,
      while the standalone cards show authored markup. Trace source/provenance
      capture and add a source-loaded regression without changing viewer behavior.
      Completed 2026-09-21: DOM-derived previews serialize an inert snapshot
      without the six reserved render tracking attributes, including inside
      nested templates. Input/live nodes retain metadata; explicit source strings
      remain verbatim. All 11 demo-element stories pass. The full runtime browser
      suite passes 186/189, with only the same three failures tracked below.
      Packaged standalone/source-loaded checks confirm matching previews in all
      three cards, working live examples and no browser errors. Base viewer
      templates and the CEM-ML import boundary are unchanged.
  - [x] Fixture: omit reserved render tracking attributes from DOM-derived demo
        source, including nested templates, explicit source slots, body capture
        and node-valued source input; retain authored attributes, literal strings
        and the original live/inert nodes.
  - [x] Fixture: compare all three real source-loaded cell-overrides previews
        with their authored DOM serialization and retain working live examples.
- [x] Fixture: diagnose the standalone local-storage JSON sample's missing
      list in the full gallery verifier. The 2026-09-21 run advanced past the
      repaired table selectors, then timed out on
      `cem-demo-element[legend="3e. JSON validation"] ul` expecting `b : B`.
      Isolate this from startup and stylesheet ownership before changing runtime
      behavior; preserve the CEM-ML import boundary for stored JSON.
      Diagnosed 2026-09-21 in standalone and source-loaded pages after explicit
      declaration/render settlement: the slice already contains the parsed
      record, but inserting that record in the summary reports
      `cem.ql.render.expression_type`. The invalid render is discarded, leaving
      the initial null output and no list. This is independent of startup and
      stylesheet ownership; the native renderer reproduces record rejection.
  - [x] Reproduce the JSON record insertion failure after browser resource
        settlement; document native storage read/write migration options before
        changing its public two-way binding contract.
- [x] Migrate JSON local-storage and both JSON samples to native CEM trees,
      removing JavaScript document parsing and record mutation. Native reads
      follow shared CEM-ML import. Accepted 2026-09-21: preserve two-way binding
      with native JSON export. The diagnosis, contract and tradeoffs are in
      [browser stabilization review](browser-stabilization-review.tmp.md#accepted-decision-native-json-storage-write-back).
      Cover null versus removal, invalid sources, lexical/order retention,
      native edits, stale completion, worker fallback, reconnect/disposal and
      standalone/source-loaded interactions before closing the remaining failure.
      Completed 2026-09-21: CEM-ML imports and exports opaque native values;
      native slice/event/saved transport preserves types and references. Both
      samples construct/query CEM nodes without JavaScript document records.
      Seven native storage tests, all 413 unit tests and all 190 browser stories
      pass; build/typecheck/lint pass (two existing warnings). The full packaged
      gallery passes all 28 standalone pages and 34 source-loaded documents.
      Focused storage interactions also pass in both modes. Real demo cards retain two
      desktop columns and 390px containment, with repeated edits persisted and
      no browser errors. The full native run exposes three separate existing
      XSLT/CEMT selection-cell parity failures tracked below; viewers are unchanged.
  - [x] Fixture: shared native JSON export preserves source order, duplicate keys,
        exact numeric lexemes, references, scalar/null semantics and typed native
        values; invalid shapes and resource limits fail without partial output.
  - [x] Fixture: native slice artifacts survive workers, fallback and saved-state
        transport; rendered native event values write back without string parsing.
  - [x] Fixture: storage import/export rejects stale completions and cleans up on
        disconnect, direct/ancestor scope disposal and key rebinding; invalid
        writes retain storage.
  - [x] Fixture: migrate the validation and basket samples, source contracts,
        lifecycle stories and standalone/source-loaded gallery checks together.
- [x] Fixture: reconcile the broad Storybook CEM-QL boundary/parity cases with
      native node values. The 2026-09-21 full run reports one diagnostic when
      `CemQlDataDocumentBoundary` inserts a slot bucket of JavaScript records,
      and two `cem.ql.type_error` diagnostics for the Rust-first table's
      `dom:children(cemml:parse(...))` case. Check the supported node/import
      contracts before updating the old empty-output expectations; add native
      reproductions first if runtime behavior needs repair.
      Completed 2026-09-21: explicit slot text renders and record insertion is
      rejected consistently in native/browser fixtures. Children/parent cases
      navigate imported CEM nodes and reject legacy opaque node identifiers.
      Both focused native fixtures pass. The full browser suite passes 188/189,
      leaving only the diagnosed storage migration failure; typecheck/lint pass
      with two existing lint warnings. No runtime semantics changed.
  - [x] Fixture: align native and browser structured-data cases on explicit
        slot text selection and rejection of direct record insertion.
  - [x] Fixture: exercise children/parent navigation on imported native nodes
        and assert typed rejection of opaque legacy node identifiers.
- [x] Fixture: migrate the native table-inspector column-heading checks to
      `thead th` and data-cell offsets past the selection row heading, matching
      the already corrected browser fixture; leave the viewer template unchanged.
- [x] Fixture: align the native Tier A registry check with the six already
      implemented CEMT value functions (`dom:text` at two arities, `reference`,
      `clone`, `element`, and `cemt:apply_templates`). The full native run found
      its stale count of 69 versus 75; verify their arities without changing the
      registry or evaluator.
- [x] Fixture: align the tree-inspection memory-limit assertion with the
      existing early payload rejection (`cem.ql.inspect_limit`), retaining
      no-partial-output, released-memory and cancellation checks. Do not alter
      inspection or shared budget behavior.
- [x] Fixture: compare the XPath/CEM-QL demo pairs' native text content, not
      serialized inner markup. `{$node}` now reuses the item subtree while
      the explicit CEM-QL label is a string; retain both authored examples.
- [x] Fixture: migrate the old XSLT output prerequisite from implicit node
      stringification to explicit `dom:text`, and assert ordinary expression
      insertion retains XML and JSON-to-XML subtrees under the accepted CEMT
      reference contract.
- [x] Audit the remaining `dom:descendants` and `dom:attribute` host stubs:
      replace empty-result placeholders with native traversal. The user approved
      exact unqualified strings, explicit namespace/name descriptors and scoped
      native attribute access on 2026-09-21. Keep base viewers unchanged.
      Completed 2026-09-21: scoped native helpers and their pipeline forms retain
      node identity, provenance and typed values across imported, constructed,
      XPath and portable views. Native verification passes 615 tests; only the
      three existing viewer parity failures below remain. Both WASM builds,
      runtime build, 413 unit tests, 191 browser stories, typecheck and lint pass
      (two existing warnings). The gallery passes all 28 standalone pages and
      34 source-loaded documents; the XPath demo also passes 1440px two-card
      layout and 390px/320px containment checks. See the
      [implementation review](dom-helper-native-review.tmp.md#implemented-result).
  - [x] Fixture DOM-NATIVE-AXES: implement the approved unqualified-string and
        exact `{namespace, name}` selectors with scoped native attribute access.
        Start with failing native cases for traversal order, empty/cardinality
        errors, namespaces, identity/provenance, all imports and native views,
        typed attributes, references, restricted hosts, cancellation and lowered
        limits. Cover both direct calls and pipeline forms.
  - [x] Fixture DOM-NATIVE-AXES-BROWSER: replace opaque empty-result parity rows
        with native descendant/attribute examples and invalid-input assertions;
        update the function reference and XPath/CEM-QL companion use cases.
        Verify native, WASM and browser paths and compact desktop/mobile demo
        presentation; keep base viewers unchanged.
  - [x] Fixture: reproduce both stubs on native imported nodes and record the
        attribute-name and scoped-host-access decisions before implementation.
        A Rust-native probe finds three descendants and one attribute through
        native fields, versus empty results from both helpers, without errors.
        [The accepted contract](dom-helper-native-review.tmp.md) specifies exact
        unqualified strings, explicit namespace/name descriptors and scoped
        native attribute access. The subsequent user approval covers both the
        selectors and shared host capability; empty stub results are not parity.
- [x] Resolve native XSLT/CEMT heading parity: the original full native run on
      2026-09-21 failed three `xslt_data_view` comparisons because XSLT emitted a
      selection `td` while CEMT emitted `th`. Both viewers were preserved during
      JSON storage migration. Retain strict semantic DOM parity checks.
      Approved 2026-09-21: the user requires `th` for headings in both versions.
      Completed 2026-09-21: XSLT now uses the same selection `th` as CEMT;
      browser fixtures share one heading selector and read the first data `td`.
      All 618 native CEM-QL tests pass, including the three formerly failing
      strict DOM comparisons. Verification also passes 413 runtime unit tests,
      191 browser stories, typecheck, lint (two existing warnings), all 28
      standalone/34 source-loaded gallery checks, and two-card 1440px layout
      with no overflow at 390px/320px. CEMT already used `th` and needed no edit.
  - [x] Fixture VIEWER-HEADING-PARITY: reproduce the existing native failures,
        align the XSLT selection heading with CEMT's `th`, and migrate browser
        row-value selectors and the gallery's XSLT-only `td` exception. Retain
        strict semantic DOM comparisons; verify sorting, selection and imported
        aspects in native and standalone/source-loaded browser checks, plus
        compact desktop/mobile demo layout.
- [x] Fixture: add an independent anonymous whole-file XSLT example, proving
      the existing bounded native transform path before browser wiring and
      testing rendered payload, disclosure interaction, and source resolution.
- [x] Fixture: add genuine embedded XSLT selected by an external XHTML
      `file#id`, preserving namespace identity and explicit compatibility
      dispatch; cover native lowering, missing fragments, unsupported-instruction
      diagnostics, and browser loading without `XSLTProcessor` or a demo-local
      JavaScript workaround.
- [x] Update source contracts, Storybook interactions, standalone/source-loaded
      case inventories, and Nx inputs; verify native/WASM, demo, lint/typecheck,
      and compact desktop/mobile presentation before closing these gaps.
    - Completion 2026-09-12: External Template cases 7d and 7e restore these
      variants with interactive payload trees. The fragment uses an explicit
      compatibility wrapper, not automatic XSLT detection. Three native fixture
      tests and the full `cem_ql:test` suite pass, along with 331 unit tests,
      152 Storybook tests, 17 standalone pages, 23 source-loaded documents,
      lint, and typecheck. Desktop/mobile checks pass at 1440px and 390px.
      Native test cache inputs now track both authored stylesheet fixtures.
      The initial two-case mapping and migration boundary are recorded in
      [`demo-teaching-points.md`](../packages/cem-elements/docs/demo-teaching-points.md#case-level-loader-follow-up);
      the complete legacy case inventory is recorded below.
    - Case audit completion 2026-09-12: all 32 local legacy files are fingerprinted
      in [`legacy-demo-cases.json`](../packages/cem-elements/docs/legacy-demo-cases.json).
      All 80 wrapped cards (including the static form-data illustration) and
      five unwrapped examples are classified. Four commented card prototypes
      are excluded from runnable coverage. Current-case/fixture links have a
      CI guard with explicit Nx inputs; the selected hex row and standalone
      XML viewers were still open at audit time, not a claim of full parity.
    - Mapped-resource completion 2026-09-12: module-URL case 4d resolves a
      prefix-mapped image and same-library fragment, including the library's
      own image and nested component. Both resource links and loaded images
      are verified. The focused native resolver fixture, 337 unit tests,
      152 Storybook tests (final rerun with `--maxWorkers=2`), 17 standalone
      pages, 23 source-loaded documents, lint/typecheck, and 1440px/390px layout
      checks pass. Default-run startup timing remains the separate TODO above.

## Native Markdown Documentation Builds

- [x] Replace the npm `markdown-it` documentation generator with the typed
      CEM-ML Markdown-to-HTML graph for `cem-theme` and `cem_ml`.
    - [x] Preserve heading anchors, images, link titles, XHTML page structure,
          source Markdown copies, static assets, source maps, and relative
          `.md` to `.xhtml` deployment links.
    - [x] Route generated token-coverage Markdown through the same CEMT XHTML
          page layout and remove the now-unused npm Markdown dependencies.
    - [x] Add focused native/CLI regression coverage and run the affected Nx
          documentation and theme gates.

The Phase 3 WASM topology is already decided: Option B, one dedicated browser
worker, is primary; Option A, main-thread WASM, is the required fallback. The
registration boundary is also locked: logical declaration lookup is scoped and
inherited, browser registration remains document-global, and only identical CEM
registration identities may reuse an inherited or existing definition.

## Namespace-Aware Data-Island Contract

- [x] Add the package-owned `complete-instance.cem` fixture and verify that the
      CEM-ML schema registry, CLI manifest validation, browser runtime, SSR
      hydration, and External Template Example 4 agree on the `0.1.2`
      namespace-aware context root and its complete ordered domain parts.

## Standalone External XSLT Declaration Source

- [x] Replace External Template Example 7b's HTML compatibility envelope with a
      genuine namespace-aware XSLT stylesheet and load it through an explicit
      external transform-template boundary.
    - [x] Preserve external declaration response media type, parse
          `application/xslt+xml`/`text/xsl` as XML, and fail closed for malformed
          or structurally invalid stylesheet documents.
    - [x] Route a valid standalone stylesheet through the native bounded XSLT
          lowering path without browser `XSLTProcessor` or an authored
          `<template lang="custom-element-v0">` wrapper.
    - [x] Fixture: prove standalone XSLT payload-tree rendering, MIME selection,
          XML namespace/root validation, and unchanged HTML/CEM-ML external
          declaration loading in unit, Storybook, and demo gates.
    - Completion: Example 7b is a standalone XSLT 1.0 XML document that selects
      the canonical namespace-aware payload section. The native converter keeps
      XPath `*` and `text()` node-kind semantics; 71 focused native compatibility
      tests, 184 `cem-elements` unit tests, 134 Storybook tests, and the 15-page /
      21-source demo sweep pass.

## External Declaration Relative Resource URLs

- [x] Correct External Template Example 9's self-relative module, nested
      declaration, and Smiley image URLs so the loaded `lib-dir/embed-lib.html`
      resource does not resolve a duplicated `lib-dir/lib-dir/` path.
    - [x] Fixture: require Example 9's rendered image to resolve to
          `/packages/cem-elements/demo/lib-dir/Smiley.svg`.
    - Completion: the external file resolves `./embed-lib.html` and
      `./Smiley.svg` directly from its own resource base. The source-document
      harness now also honors the runtime's `resourceBaseUrl`; all 15 standalone
      and 21 source-loaded demo fixtures pass.

## Module URL Demo Asset Ownership

- [x] Replace Module URL Example 4's external custom-element logo dependency
      with the repository-owned `cem-elements/demo/lib-dir/Smiley.svg` asset.
    - [x] Fixture: verify the standalone and source-loaded demo resolve and
          display the CEM-owned Smiley URL.
    - Completion: the symbolic module identity, page resolver, source-document
      resolver, rendered URL assertions, and filename output now use the
      CEM-owned Smiley asset; all 15 standalone and 21 source-loaded demos pass.

## Declarative UI Architecture Correction

- [x] Prove the corrected component authoring contract with `cem-select` before
      changing the normative documentation: add any reusable select capability
      to `cem-elements`, move `cem-select` to its own CEM-ML XHTML declaration
      with embedded CSS, colocate its CSF Next TypeScript story and `play`
      tests, remove its component-owned JavaScript and global CSS, and make the
      Nx architecture gate enforce that shape.
    - Completed 2026-08-23: `cem-select` now lives only in
      `src/components/cem-select/cem-select.xhtml`, uses the reusable
      `cem-elements` `choice-select` capability, embeds its own token CSS, and
      exposes its structural `<template id="cem-select">` for fragment reuse,
      and owns six colocated CSF Next interaction/unit stories. All 121 Storybook
      Chromium tests, the production Storybook build, component typecheck,
      declarative/state/style/forced-colors gates, and package publication
      checks pass.
- [x] Promote the no-JS declarative authoring rule to the primary repository,
      component, Studio, and Site contract.
- [x] Add the `cem-components:verify-declarative` hard gate with an explicit
      frozen inventory of the remaining 48 legacy registry members and 61
      authored JavaScript/TypeScript files. New component code cannot
      extend that debt; colocated `.stories.ts` test/tooling files are excluded
      from the legacy-code inventory and structurally validated instead. The
      remaining behavior implementations have per-file SHA-256 freezes, so they
      can be removed during migration but cannot be extended in place.
- [x] Implement the `cem-elements` Storybook discovery and lazy XHTML declaration
      loader for colocated CSF Next `.stories.ts` modules, including public CEM
      theme CSS loading, HTML-string `render` cases, and `storybook/test` unit
      assertions in async `play` functions.
    - Completed 2026-08-23: the preview loads each story's raw XHTML declaration
      only when needed, keeping unrelated runtime stories fast; all 121
      Storybook Chromium tests and the production build pass.
- [x] Extend the existing `cem-elements` index demo fixture to prove that an
      attribute interpolation can coalesce missing named-slot text to an
      authored placeholder default while preserving supplied slot text.
- [x] Implement native scoped `cem:variable` template bindings and use one in
      the `cem-elements` Pokémon demo to reuse its sprite base URL across the
      host image and payload loop, with Rust and browser fixture evidence.
    - Completed 2026-08-29: `cem:variable` now binds a static or data-driven
      CEM-QL expression for following content, restores shadowed bindings at
      lexical scope boundaries, and drives both Pokémon sprite URL sites.
- [x] Restore functional legacy attribute-declaration parity: compile and
      evaluate top-level `attribute @select` expressions, return and apply
      loop-safe host-attribute updates, rewrite legacy declaration XPath, and
      make the four `demo/attributes.html` samples verify default, host, slice,
      and event-derived values through both rendered bindings and host state.
    - Completed 2026-08-29: native CEM-QL/CEM-ML targets, 152 unit tests,
      132 Storybook interactions, 16 edge/SSR unit tests, and the 15-page /
      21-source-document browser fixture gate pass.
- [x] Keep declaration-selected attributes synchronized in the current
      `datadom.attributes` projection, with a three-keystroke example-4 browser
      regression proving that the final slice event is visible immediately.
    - Completed 2026-08-29: defaults and successful selects now update the
      current render projection before body evaluation; native tests, all 132
      Storybook interactions, and the 15-page / 21-source-document demo gate pass.
- [x] Add source-loaded, legend-scoped browser contracts for `cem-elements`
      `index.html` and every HTML document under `demo/`, including support
      documents, without requiring test-only hooks in the authored examples.
    - Completed 2026-08-29: the inventory-gated verifier now exercises 15
      standalone pages and 21 source-loaded documents, locates samples by legend,
      and checks each sample through scoped DOM/CSS selectors. Runtime regressions
      cover nested inert templates and loaded-document-relative declarations and
      resources; the authored demo HTML needs no test-only hooks.
- [x] Restore all 15 legacy `demo/data-slices.html` use cases as independent
      native CEM-ML samples, adding reusable default event/value, attribute-target,
      multi-directive, checkbox/radio, and event-coordinate behavior with
      Storybook and source-loaded browser contracts.
    - Completed 2026-08-29: declared slice defaults now update both named bindings
      and `datadom.slices`; form controls default to `change` with legacy-aware
      value coercion; attribute targets, nested synthetic directives, multiple
      events/targets, synthetic `slice @value` initialization, and serialized
      mouse coordinates run through the shared runtime. All 15 legacy cases have
      independent source-loaded contracts. The full 217-test `cem_ql` suite,
      152 unit tests, 133 Storybook interactions, lint,
      typecheck, and the 15-page / 21-source-document browser gate pass.
- [x] Fixture: preserve the split A1/A2 data-slice samples, making A1 prove
      legacy input-owned `?? 0` initialization without a declared slice while
      A2 retains declared-slice initialization, with independent legend-scoped
      browser contracts.
    - Completed 2026-08-29: A1 now uses the canonical CEM-QL equivalent
      `datadom.slices.clickcount ?? 0` only in the input value, with no declared
      slice and click-only increment/decrement bindings; A2 keeps the declared
      slice. Their produced tags and legend contracts are independent, and the
      16-sample data-slices inventory passes in the 15-page / 21-document gate.
- [x] Add package-owned unit coverage for every authored
      `demo/data-slices.html` sample: an exact source-contract test per legend
      in `test:unit`, plus one source-loaded Storybook interaction test that
      exercises every sample through its rendered controls and outcomes.
    - Completed 2026-08-29: 16 exact legend contracts and a no-test-hooks
      assertion bring the Node suite to 170 tests; the source-loaded interaction
      brings Storybook to 134 tests and exercises A1, A2, B, and 1-13. Demo
      selectors are structural, with no `data-role`, `data-testid`, or test-only
      `id` attributes; the 15-page / 21-source-document browser gate passes.
- [x] Fixture: load `demo/module-url.html` through a dedicated Storybook story
      using the same external `<cem-element src="…">` boundary as the data-slices
      demo, and verify its page-root import map, global mapping, URL-referrer
      scope selection, published absolute URLs, and transient-control removal.
    - Completed 2026-09-01 and expanded below: the exact five-legend source
      contract runs in `test:unit`; the Storybook interaction loads the authored
      HTML rather than copying its declarations and exercises root and local
      maps through Storybook's host-owned root configuration.
- [x] Expand `demo/module-url.html` into the executable module-resolution
      matrix: relative, bare-module, and absolute `src` values; implicit,
      relative-URL, bare-module, absolute-URL, and descendant-node referrers;
      and a component-local module map shown both naked and inside an
      outer-map wrapper whose winning mapping changes the rendered image.
    - [x] Specify and implement component-local map ownership and selector-based
          node referrers before relying on either capability in the fixture.
    - [x] Cover every new legend in the exact source contract, source-loaded
          Storybook interaction, and standalone/source demo-fixture gate.
    - Completed 2026-09-01: static `module-map` preludes compile into portable
      template-artifact metadata and install before local-map custom-element
      upgrade; clone-safe `referrer-selector` controls select exactly one live
      rendered descendant and retain shared descendant authorization. The demo
      covers the scalar 3-by-3 matrix plus all three `src` forms through a node
      referrer, and proves naked-inner, wrapped-outer override, and inner-only
      fallback images. Focused native map/artifact tests, 198 package unit tests,
      135 Storybook interactions, and the 15-page / 21-source-document browser
      gate pass.
- [x] Fix the module-URL `image-link` helper to invoke Tier A `str:shorten`
      as a text expression and render resolved URL labels at no more than 32
      codepoints while retaining the complete URL in `href`.
    - [x] Fixture: add the helper legend to the exact source and source-loaded
          Storybook inventories, covering file-relative, naked, and
          wrapper-resolved URL labels.
    - Completed 2026-09-03: `image-link` resolves its input from the declaring
      template's URL, retains that full URL for image/link attributes, and
      renders a 32-codepoint middle-shortened label. Exact source contracts,
      199 unit tests, 135 Storybook interactions, typecheck, lint, and the
      15-page / 21-source-document browser fixture gate pass.
- [x] Add a live `str:shorten` query/result matrix to the module-URL demo.
    - [x] Cover unchanged input, odd and even budgets, custom and empty
          ellipses, Unicode codepoints, and a URL-shaped value in exact source,
          Storybook, and standalone/source-loaded browser contracts.
    - Completed 2026-09-03: the seven-row matrix displays each executable
      CEM-QL query beside its evaluated result. All 200 unit tests, 135
      Storybook interactions, and the 15-page / 21-source-document browser
      fixture gate pass.
- [x] Replace direct rendered images and links in every module-URL sample with
      the shared `image-link` helper.
    - [x] Resolve explicit image and link targets independently for the
          package-logo case; same-target callers pass the resolved URL to both.
    - [x] Prove samples 4, 5, and 7 contain no direct CEM-ML `img`/`a` output
          and retain their resolved image, link, and shortened-label behavior.
    - Completed 2026-09-03: samples 4, 5, and 7 now delegate image and anchor
      rendering to `image-link`; only that helper owns CEM-ML `img`/`a` nodes.
      Its separately resolved `src` and `href` preserve the package-README
      case while same-target calls remain explicit. All 200 unit tests, 135
      Storybook interactions, and the 15-page / 21-source-document fixture
      gate pass.
- [x] Repair module-URL example 7 after its formatting refactor split the
      inner-only resource's `@target` attribute sigil from its name.
    - [x] Add an exact-source guard for the complete component-local resource
          mapping and rerun naked, wrapper-override, and node-referrer browser
          contracts in both standalone and source-loaded contexts.
    - Completed 2026-09-03: the inner-only mapping again declares one intact
      `@target` attribute, and the exact-source contract rejects the split
      `@ target` spelling. All 200 unit tests, the focused module-URL
      Storybook interaction, lint, typecheck, and the 15-page /
      21-source-document browser fixture gate pass.
- [x] Repair simplified module-URL sample 1 so its anonymous declaration passes
      the resolved package-subpath URL into `image-link`.
    - [x] Cover the package-prefix import-map entry, published slice value, and
          rendered image/link URL in exact-source and browser fixture contracts.
    - Completed 2026-09-05: executable, displayed, Storybook, and source-harness
      import maps use the valid `@epa-wg/cem-elements/` prefix entry, and the
      anonymous sample publishes its absolute URL through `image-link` to the
      rendered image and anchor.
- [x] Move the scalar URL-referrer matrix from `demo/module-url.html` into the
      standalone `demo/module-url-referrer.html` page.
    - [x] Explain scalar-referrer base and scope selection, distinguish
          `referrer-selector`, link the extracted page from the original demo,
          and cover it in exact-source, Storybook, standalone, source-loaded,
          and inventory checks.
- [x] Establish `packages/cem-elements/demo/functions/` as the executable
      CEM-QL function-demo collection.
    - [x] Move the `str:shorten` query/result matrix out of `module-url.html`
          into the standalone `functions/str.html` string-function page.
    - [x] Add exact-source, source-loaded Storybook, standalone browser, and
          inventory coverage for the function-demo page after the current demo
          restructuring is complete.
    - Completed 2026-09-05: `module-url.html` links both extracted demos without
      changing its prettified source layout. All 207 unit tests, three focused
      Storybook interactions, lint, typecheck, and the expanded 17-page /
      23-source-document browser gate pass.
- [x] Split the bundled `demo/for-each.html` fixture into one independent
      `html-demo-element` per authored use case.
    - [x] Give all nine cases declaration and instance ownership local to their
          sample, including conditional state, payload, location, and HTTP data.
    - [x] Update standalone and source-loaded fixture inventory and interaction
          checks for each new legend.
    - Completed 2026-09-05: all nine cases now render as independent samples;
      unit source contracts (217 total tests), focused Storybook interactions,
      lint, typecheck, and the 17-page / 23-source-document browser gate pass.
- [x] Apply the scoped demo-authoring contract to `demo/dom-merge.html`.
    - [x] Split the textarea and input counters into independent, anonymous,
          flush-left `html-demo-element` samples with reader-focused context.
    - [x] Cover both samples through exact-source, Storybook interaction,
          standalone, source-loaded, and Nx input contracts without test-only
          markup anchors.
    - Completed 2026-09-05: both DOM-merge samples now preserve edited control
      identity and focus while updating derived values. All 221 unit tests, two
      focused Storybook stories, lint, typecheck, the 17-page / 23-document
      browser gate, and the two-card desktop layout check pass.
- [x] Port the five form use cases and explanatory contract from
      `@epa-wg/custom-element@0.0.39/demo/form.html` to the `cem-elements` demo.
    - [x] Adapt simple validation, form lifecycle, native control messages,
          form custom messages, and a form-associated DCE into independent,
          compact, flush-left samples using current CEM-ML data paths.
    - [x] Add exact-source, Storybook interaction, standalone, source-loaded,
          inventory, and Nx input coverage without demo-local JavaScript or
          test-only markup anchors.
    - Completed 2026-09-05: all five legacy cases and their descriptive form
      contract are adapted to current `datadom` paths; the nested fruit-choice
      DCE contributes native form values through the declarative `choice-select`
      capability. All 229 unit tests, the focused five-case Storybook flow,
      lint, typecheck, the 17-page / 23-document browser gate, and the compact
      desktop layout check pass.
- [x] Port the three HTTP-request use cases from the local legacy
      `demo/http-request.html` to the `cem-elements` demo.
    - [x] Adapt runtime URL selection, a minimal image-backed GET, and request /
          response metadata inspection into independent, compact samples using
          same-origin repository fixtures.
    - [x] Add deterministic local image-backed and compact-response fixtures
          for the migrated cases.
    - [x] Add exact-source, Storybook interaction, standalone, source-loaded,
          inventory, and Nx input coverage without test-only demo markup.
    - Completed 2026-09-05: the three legacy scenarios now use current resource
      envelopes, scoped relative URLs, and deterministic local JSON/image data.
      All 235 unit tests, the focused three-case Storybook interaction, lint,
      typecheck, the 17-page / 23-document browser gate, and the two-card desktop
      layout check pass.
- [x] Restore the legacy six-Pokémon list in the simplest HTTP-request demo.
    - [x] Add a deterministic local Pokémon response fixture with self-contained
          images instead of reintroducing the remote PokeAPI dependency.
    - [x] Update source, Storybook, standalone, source-loaded, and Nx input
          contracts for the six rendered image buttons.
    - Completed 2026-09-05: the simplest case loads bulbasaur through charizard
      from `http-pokemon.json` and renders six locally backed image buttons. All
      235 unit tests, the focused Storybook interaction, lint, typecheck, and the
      17-page / 23-document browser gate pass.
- [x] Match the simplest HTTP-request case's legacy Pokémon presentation.
    - [x] Render the version-pinned Dream World Pokémon SVGs rather than text or
          placeholder-letter artwork, while retaining accessible button names.
    - [x] Cover the image-only source and resolved sprite URLs in unit,
          Storybook, standalone, and source-loaded fixture contracts.
    - Completed 2026-09-05: `Pokemon buttons from API` renders the six Dream
      World sprites without visible name text; `aria-label`, `title`, and `alt`
      retain each Pokémon name. All 235 unit tests, the focused Storybook
      interaction, lint, typecheck, and the 17-page / 23-document browser gate
      pass, with browser fixtures intercepting the pinned sprite URLs offline.
- [x] Use expressive emoji/Unicode labels across the demo gallery.
    - [x] Keep clear accessible names and tooltips, and verify the symbolic
          controls, fruit lists, and live counts in source, Storybook, standalone/source
          fixtures, and desktop/mobile layouts.
    - Completion: nine pages use symbolic controls and fruit values while
      retaining readable accessible names, tooltips, and technical teaching
      labels. Selected fruit options have an inset selection mark. Verified
      321 unit tests, 151 Storybook tests (plus focused final form reruns),
      17 standalone pages, 23 source-loaded documents, keyboard activation,
      lint, typecheck, and two-card/overflow layouts at 1440px and 390px.
      The headless host lacks an emoji font; Unicode text is browser-asserted,
      but pictogram appearance needs an emoji-capable system font.
- [x] Restore the remaining demo teaching points from `~/aWork/custom-element`.
    - [x] Add and verify event-triggered URL writes so live readers do not
          reapply stale navigation commands; cover repeated actions and drafts.
    - [x] Add a malformed JSON response fixture and verify visible failure,
          stale-result removal, and recovery through the authored GET controls.
    - [x] Audit the remaining gallery pages and supporting templates against
          the local legacy sources, recording preserved lessons and deviations.
    - [x] Restore missing reader-driven interactions and observable outcomes
          using current CEM-ML, with independent external controls where the
          demonstrated capability observes external state.
    - [x] Update affected source contracts, Storybook interactions, fixture
          inventories, related links, and Nx inputs; verify layout and browser
          behavior in standalone and source-loaded documents.
    - Completed 2026-09-12: the local prototype
      [audit](../packages/cem-elements/docs/demo-teaching-points.md) records
      preserved lessons and intentional current-contract differences. External
      History/attribute controls, actual URL/version synchronization, independent
      event-triggered URL writers, native Next validation, focus/caret retention,
      HTTP idle/failure/recovery, and split anonymous/external-template cases
      now have observable outcomes. All 312 unit tests, 151 Storybook Chromium
      tests, the 17-page / 23-source-document fixture gate, lint, and typecheck
      pass. Desktop/mobile checks confirm compact rows and no page overflow;
      Nx inputs cover the shared templates, assets, styles, and negative fixture.
- [x] Restore the local-storage demo's external-write teaching points.
    - [x] Restore direct Storage API controls, live validation and recovery,
          removal versus empty values, persistence, initial-only reads, and
          fruit/basket updates while keeping slice projection declarative.
    - [x] Update source contracts, Storybook interactions, and standalone/source
          fixture inventories; verify desktop layout, lint, and typecheck.
    - Completed 2026-09-12: twelve cases restore independent Storage API writers
      and declarative observers, typed invalid/valid transitions, JSON object and
      array projections, empty/zero/false values, persistence, initial-only reads,
      fruit totals, and a separate slice-to-storage editor. The 16 focused source
      tests, source-loaded Storybook interaction, lint, typecheck, and 17-page /
      23-document fixture gate pass, including real reload and cross-tab checks.
      Desktop cards fit two per row without source overflow; the narrow viewport
      has no page overflow. Existing Nx inputs already cover all changed fixtures.
- [x] Port the six local-storage use cases from the local legacy
      `demo/local-storage.html` to the `cem-elements` demo.
    - [x] Adapt live text synchronization, authoritative values, persisted
          defaults, typed coercion, an initial-only read, and a live JSON basket
          into independent, compact current CEM-ML samples.
    - [x] Add exact-source, Storybook interaction, standalone, source-loaded,
          inventory, and Nx input coverage without test-only demo markup.
    - Completed 2026-09-05: all six legacy scenarios are independent CEM-ML
      samples. Live instances synchronize through one key; authoritative and
      persisted-default semantics remain distinct; typed, initial-only, and
      editable JSON reads are covered. All 244 unit tests, 142 Storybook
      interactions, lint, typecheck, the 17-page / 23-document browser gate,
      and the three-card desktop layout check pass.
- [x] Add declarative lemon, cherry, apple, and banana storage controls to the
      local-storage demo.
    - [x] Keep the fruit writer independent from a live watching DCE that owns
          the four associated slices and rendered counts.
    - [x] Cover each button-to-storage-to-watcher update in source, Storybook,
          standalone, and source-loaded fixture contracts.
    - Completed 2026-09-05: an anonymous writer DCE increments the four numeric
      fruit keys, and an independent anonymous watcher uses `live` resources to
      update its matching slices, visible counts, and total from 13 to 17. All
      245 unit tests, 142 Storybook interactions, lint, typecheck, the 17-page /
      23-document browser gate, and the desktop no-overflow check pass.
- [x] Port the remaining top-level legacy demos from `location-element.html`
      through `set-url.html` into the current `cem-elements` gallery.
    - [x] Restore independent live, initial-only, and explicit-HREF location
          readers, with URL-changing controls owned by the live case.
    - [x] Add the missing legacy module-URL declaration-loading, unmapped
          specifier, and mapped-fragment scenarios without disturbing the
          current scope/referrer matrix.
    - [x] Replace the NPM-version placeholder with deterministic default,
          preselected/date, propagated-value, label-slot, and URL-sync cases.
    - [x] Map the remaining legacy scoped-CSS descendant-selector and external
          template lessons onto the current stricter CSS ownership contract.
    - [x] Restore the four independent URL-write cases for hash, method,
          conditional injection, and form-driven selection.
    - [x] Preserve `module-url-referrer.html`, which has no legacy counterpart,
          and keep bidirectional related-demo links accurate.
    - [x] Add exact-source, Storybook, standalone/source-loaded fixture,
          inventory, and Nx input coverage for every added or changed sample.
    - Completed 2026-09-06: the location reader, module URL loader, NPM-version
      picker, scoped CSS, and URL writer demos now cover the applicable legacy
      lessons as independent current CEM-ML cases. `module-url-referrer.html`
      remains the focused referrer matrix. All 282 unit tests, 146 Storybook
      interactions, lint, typecheck, and the 17-page / 23-document browser gate
      pass.
- [x] Restore the legacy framework-logo honeycomb in `demo/hex-grid.html` using
      responsive semantic-list layout principles from Web Tiki's CSS grid.
    - [x] Render the legacy framework set with deterministic local assets for
          relative-image cases and one legacy remote asset for the full-URL
          case, all inside keyboard-accessible hexagonal links.
    - [x] Preserve one reusable payload-driven DCE case with scoped CSS,
          responsive staggered row counts, and scope-aware logo URL resolution.
    - [x] Add exact-source, source-loaded Storybook, standalone/source-loaded
          fixture, inventory, Nx input, lint, and typecheck coverage.
    - [x] Accept semantic link-with-image payload and prove that both link
          destinations and image sources resolve from relative or full URLs.
    - [x] Raise the revealed label into the hexagon and size, balance, and wrap
          longer labels responsively against each tile's inline size.
    - [x] Document and demonstrate percentage and fixed-length grid `size`
          values that scale the complete cell system without separating links.
    - [x] Keep link border boxes inside standalone demo cells, make background
          alternation an opt-in parameter, and show loading/error image fallback
          with a long wrapping label.
    - Completed 2026-09-06: the payload-driven grid renders 14 framework logos
      as semantic links, resolves each payload link and image through
      `cem-module-url`, and responds from 5–4 desktop rows down to 2–1 narrow
      rows without page overflow. The source, Storybook, and standalone/source-
      loaded fixture contracts cover relative and full URL forms.
- [x] Replace the CDN-dependent `html-demo-element` with the independent,
      offline-capable `@epa-wg/cem-demo-element` package.
    - [x] Add a structured CEM-ML-to-HTML Rust/WASM render API shared by the
          demo element and future CEM Studio previews.
    - [x] Preserve inline HTML/template, named-region, external-source,
          language detection, programmatic-source, and legend/description
          behavior while adding trusted CEM-ML live rendering and explicit
          loading/error states.
    - [x] Fixture: cover every public behavior through real Chromium Storybook
          interaction tests as the package's unit-test suite, and prove the
          package demos load local workspace artifacts with all network access
          disabled.
    - [x] Replace `cem-elements` demo markup/imports and fixture inventories with
          `cem-demo-element` without adding test-only markup.
    - Completed 2026-09-07: `@epa-wg/cem-demo-element` supplies local source
      presentation and lazy CEM-ML WASM rendering, its nine Storybook Chromium
      tests cover the public and offline-demo contracts, and all 147 existing
      `cem-elements` Storybook interactions plus the 17-page standalone/23-
      document source-loaded fixture matrix pass against the real element.
- [x] Move HTML and CEM-ML source coloring into the CEM-ML formatter boundary
      and apply a Chromium DevTools-inspired semantic palette in
      `cem-demo-element`.
    - [x] Make lossless HTML formatter tokens distinguish markup delimiters,
          tag names, attribute names, equals signs, quotes, values, comments,
          and text without changing the authored source bytes.
    - [x] Expose semantic HTML and CEM-ML source highlighting through the
          shared Rust/WASM runtime so the demo element does not own a parallel
          markup grammar.
    - [x] Fixture: prove Rust token roles, Node/browser WASM transport, and real
          Chromium Storybook rendering for HTML and CEM-ML source.
    - Completed 2026-09-07: CEM-ML's HTML formatter exposes lossless semantic
      markup tokens, the versioned Node/browser WASM source formatter covers
      HTML and CEM-ML, and `cem-demo-element` applies customizable Chromium
      source-view light/dark palettes. Focused native formatter/API tests, both
      WASM runtimes, TypeScript build/lint, and all nine Storybook Chromium
      cases pass.
- [x] Derive the source-code theme from CEM's emotional palette while making
      tag names, attribute names, and values perceptually distinct.
    - [x] Map structure to trust, modifiers to enthusiasm, values to creativity,
          comments to calm, and punctuation to conservative palette endpoints;
          keep danger reserved for actual errors.
    - [x] Fixture: verify the resolved tag, attribute, and value colors in the
          real `cem-demo-element` Storybook rendering.
    - Completed 2026-09-08: native inline/CSS-variable colorizers and
      `cem-demo-element` resolve syntax roles through the CEM emotional palette,
      with standalone light/dark fallbacks. Focused Rust tests, the WASM and
      demo builds, both lint targets, and all nine Storybook Chromium cases
      pass.
- [x] Add a syntax-coloring decision page for the complete HTML and CEM-ML AST
      token/event surfaces.
    - [x] Cover valid and recovery/error variants for both content types and
          identify every semantic source-color role and its emotional token.
    - [x] Provide native, light, dark, contrast-light, and contrast-dark theme
          switching using the generated `cem-theme` CSS, without copying theme
          token values into the demo page.
    - [x] Fixture: load the complete page and source specimens offline in the
          `cem-demo-element` Storybook suite, switch every theme mode, and
          verify representative resolved HTML and CEM-ML syntax colors.
    - Completed 2026-09-08: the standalone decision page names every HTML event,
      HTML markup-token, and CEM-ML tokenizer variant, renders complete and
      recovery/error specimens, documents the emotional role mapping, and
      switches all five public CEM theme modes. The offline Chromium fixture,
      lint, component build, and production Storybook build pass.
- [x] Finalize the approved action-token hierarchy as the shared HTML/CEM-ML
      source-color contract.
    - [x] Promote tag, attribute/variable, keyword, string, and error colors
          from the decision page into the shared CSS-variable colorizer and
          `cem-demo-element` defaults.
    - [x] Make tag names and attribute/variable identifiers bold across shared
          formatter output and the browser fallback renderer.
    - [x] Replace per-token source classes/role attributes with a compact native
          tag vocabulary beneath one classified `<code>` root.
    - [x] Give tokenizer errors an explicit `diagnostic.error` source role so
          invalid source and its structured diagnostic share the error color.
    - [x] Fixture: prove exact generated CSS-variable fallbacks, native/WASM
          error-role transport, and resolved offline Storybook colors in all
          five CEM theme modes.
    - Completed 2026-09-08: source highlighting now uses one classified `<code>`
      root with semantic descendant tags, preserves structured source roles for
      consumers, applies the approved emotional action tokens and weights in
      native and browser renderers, and colors tokenizer errors consistently.
      Rust, Node/WASM, offline Storybook, lint, typecheck, component build, and
      production Storybook checks pass across all five CEM theme modes.
- [x] Apply `cem-demo-element` to the `cem-elements` and `cem-components`
      package demo pages.
    - [x] Keep the existing `cem-elements` landing-page examples on the local
          demo element and verify their source uses the semantic tag vocabulary.
    - [x] Add a `cem-components` landing-page gallery that loads each canonical
          declarative workflow fragment without modifying the fixture sources.
    - [x] Fixture the component gallery inventory, live primitive rendering,
          and source highlighting in the shared Storybook browser suite.
    - Completed 2026-09-08: both package landing pages use the local demo
      element and CEM-ML formatter. The new component gallery loads all eight
      canonical workflow fragments, while the shared Storybook suite verifies
      its exact inventory, highlighted source, live primitives, and rebased
      native resource URLs.
- [x] Restore semantic source coloring on standalone `cem-elements` demos.
    - [x] Map the local `@epa-wg/cem-ml/wasm` dependency before module scripts
          in every authored demo page that loads `cem-demo-element`.
    - [x] Fixture the shared import-map contract across the complete demo
          inventory and semantic source tokens on the standalone hex-grid page.
    - Completed 2026-09-09: all 16 standalone demo pages now resolve the local
      CEM-ML WASM formatter before module loading. Static inventory validation
      rejects missing, late, or misresolved mappings, and the real-browser
      hex-grid fixture verifies native name, attribute, and string tokens and
      their computed syntax colors.
- [x] Dispatch source formatting and coloring through nested AST content-type
      scopes.
    - [x] Treat an HTML `<template type="text/cem-ml">` body as a bounded
          CEM-ML scope, then resume HTML formatting after its closing tag.
    - [x] Treat a CEM-ML `style` rich-content body as scoped `text/css` and
          apply the CSS formatter/colorizer roles inside its fence.
    - [x] Fixture the nested HTML → CEM-ML → CSS transitions in hex-grid
          Example 7 without introducing demo-only markup.
    - Completed 2026-09-09: source highlighting now builds bounded child
      content-type scopes from the parent HTML/CEM-ML AST, dispatches each body
      to its language parser and formatter/colorizer role mapping, rebases the
      resulting spans, and resumes the parent language at the closing boundary.
      Native, unit, Storybook browser, and standalone demo fixtures cover the
      HTML → CEM-ML → scoped CSS path in hex-grid Example 7.
- [x] Distinguish CSS declaration keys and values with AST-owned source roles.
    - [x] Record selector, property-name, declaration-value, function, and
          custom-property contexts on lossless CSS AST events.
    - [x] Drive the CSS schema formatter and native source colorizer from the
          winning AST context while preserving number, string, comment, and
          diagnostic roles.
    - [x] Add compact semantic HTML hooks and independently overridable colors
          for CSS properties, values, and functions.
    - [x] Extend the syntax-coloring decision demo and fixtures with standard
          declarations, custom-property definitions/references, functions,
          hashes, nesting, and `@property` coverage.
    - Completed 2026-09-09: CSS lossless events now carry a winning semantic
      context used by both the schema formatter and native source highlighter.
      `cem-demo-element` sends standalone CSS through the local WASM formatter,
      renders properties/values/functions as compact `dfn`/`data`/`kbd` hooks,
      and exposes independently overridable IDE-inspired light/dark colors while
      retaining CEM action tokens where their light-mode meaning aligns. Native
      formatter/colorizer tests, lint and typecheck targets, all 10 demo-element
      and 148 cem-elements Storybook cases, and the 17-page/23-document browser
      fixture matrix pass.
- [x] Make every source formatter/colorizer follow AST-owned content-type
      switches through one generic syntax stream.
    - [x] Extract the HTML, CEM-ML, and CSS scope dispatcher from the browser
          source-view API into a reusable, lossless AST token stream.
    - [x] Preserve each token's active content type, semantic role, source
          range, and parent-owned return boundary across nested scopes.
    - [x] Feed the same stream to the lifecycle formatter/colorizer pipeline so
          CLI HTML presentation invokes CEM-ML and CSS roles inside typed
          embedded content, then resumes the parent formatter.
    - [x] Prove HTML → CEM-ML → scoped CSS role switching and byte-for-byte
          visible-source parity in native and CLI regression tests.
    - Completed 2026-09-09: the public lossless source-syntax AST stream now
      carries each token's active content type, formatter/colorizer identity,
      semantic role, absolute source range, and scope depth. HTML lifecycle
      presentation consumes that same stream for typed template bodies, so the
      parent HTML formatter yields to CEM-ML, CEM-ML yields to scoped CSS, and
      each parent resumes at its AST-owned boundary without changing visible
      source bytes. All 2,012 `cem-ml` library tests and the focused real CLI
      nested-content conversion regression pass.
- [x] Define CLI-driven whole-page CEM SSR compilation over the shared
      `cem-element` lifecycle.
    - [x] Separate deployed page URL, source URI, and output destination in the
          transform-graph contract, including directory fan-out.
    - [x] Define HTML, CEM-ML, and `.cem.md` ingress plus CEMT orchestration,
          shared render-plan execution, data-island serialization, hydration,
          formatter/colorizer, policy, and parity boundaries.
    - [x] Publish an implementation sequence, diagnostics contract, command and
          transform-config examples, and acceptance matrix as a proposal.
    - Completed 2026-09-09: `docs/cem-ml-cli-ssr-proposal.md` defines a
      config-first CEMT transform over a shared native lifecycle intrinsic,
      binary-first SSR page artifact, separate source/page/output identities,
      HTML/CEM-ML/`.cem.md` inputs, single-file and directory routing, DOM-safe
      HTML formatting, non-deployable colorized sidecars, and zero-rerender
      hydration parity. It also defines exact tag-registered CEMT SSR
      participants so `cem-demo-element` can transform its inert payload before
      the generic lifecycle renders the activated inner DCEs.
- [ ] Enforce one-to-one source-loaded Storybook coverage for every authored
      sample in `packages/cem-elements/index.html` and every HTML document under
      `packages/cem-elements/demo/`, following the current data-slices and
      module-url pattern.
    - [ ] Add a machine-checked page-and-legend inventory that fails when an
          authored demo page or `cem-demo-element[legend]` lacks a corresponding
          Storybook contract, or when a contract names a removed page or legend.
    - [ ] Load each authored HTML file through `<cem-element src="…">`; do not
          copy its CEM-ML declarations into the story or replace host-owned
          resource/import-map behavior with per-example resolver callbacks.
    - [ ] Exercise every sample's observable rendered outcome in an asynchronous
          Storybook `play` test, while retaining exact source-shape assertions in
          `test:unit` and avoiding test-only IDs, roles, or data attributes in the
          demo HTML.
    - [ ] Keep `verify-demo-fixtures` as the independent standalone-page and
          source-document browser gate, and require `test:unit`, Storybook
          Chromium, and demo-fixture inventory checks to pass together.
- [x] Fix sample B dynamic inline-style interpolation so mouse-event `offsetX`
      and `offsetY` values produce valid pixel lengths in `box-shadow`, with a
      native render regression and a computed-style browser assertion.
    - Completed 2026-08-30: sample B now uses canonical attribute-value CEM-QL
      expressions without the text-node `$` prefix, rendering values such as
      `157px 120px` instead of `px px`. The 218-test native suite, 170 package
      unit tests, 134 Storybook interactions, and the 15-page / 21-source-demo
      browser gate pass; both source-loaded and standalone checks require a
      nonempty computed shadow.
- [x] Add the strict CEM-QL `string + string -> string` overload without
      implicit conversion, carry it through type checking, lowering, native and
      slice-event evaluation, document the contract, and migrate binary string
      concatenation examples from `concat(...)` while retaining stream-join
      examples for `str:concat`.
    - Completed 2026-08-30: `+` now dispatches to exact string concatenation or
      same-type numeric addition in the checker, typed IR, and runtime, with
      static and dynamic mixed-type failures. Sample 5 and binary-concatenation
      fixtures use `+`; separator-based stream joins retain `str:concat`. The
      generated schema-package README, 220 native tests, 170 package unit tests,
      134 Storybook interactions, schema-package verification, typecheck, and
      the 15-page / 21-source-document browser gate pass.
- [x] Add literal `str:split(value, separator)` and simple JavaScript-inspired
      string helpers without changing existing CEM-QL string semantics.
    - [x] Specify and implement trim variants, character access, and forward/
          backward search with codepoint-safe indices and documented edge cases.
    - [x] Fixture: cover registration/arity, empty values, Unicode, bounds,
          overlapping searches, and composable word counting in native tests first.
    - [x] Publish all eight helpers in the schema-package example and regenerate
          and verify its reference documentation.
    - [x] Replace demo word-count arithmetic with split/count, and add focused
          interactive string-function samples with source and Storybook contracts.
    - [x] Update the standalone/source-loaded sample inventories and verify native,
          WASM, demo, lint/typecheck, and desktop/mobile behavior.
    - Completed 2026-09-12: eight native Tier A helpers preserve CEM's codepoint
      indexing and sequence semantics. Word-count demos use split/filter/count,
      and each new helper has an independent interactive example. Native tests
      and lint, WASM build, schema-package generation/verification, 330 unit
      tests, the 151-test Storybook rerun, 17 standalone pages, 23 source-loaded
      documents, lint/typecheck, and 1440px/390px layout checks pass.
- [x] Add Tier A `str:shorten(value, max_length, ellipsis?)` with a default
      Unicode ellipsis, codepoint-safe middle elision, suffix-favored odd
      budgets, and marker-aware minimum-length clamping.
    - [x] Fixture: publish and verify default, custom, and empty-marker examples
          in the CEM-QL schema package alongside focused registry and native
          evaluation coverage.
    - Completed 2026-09-01: `str:shorten` is registered at arities 2–3 and
      preserves both ends with the default `…`, arbitrary multi-codepoint, or
      empty markers. All 234 CEM-QL tests and the CEM-QL schema-package
      generation/verification target pass.
- [ ] Add a pure Tier A `url:` function family (`cem:stdlib/url`) for parsing,
      normalizing, assembling, and disassembling URLs according to the
      [WHATWG URL Standard](https://url.spec.whatwg.org/) and the browser
      [URL API](https://developer.mozilla.org/en-US/docs/Web/API/URL).
    - [ ] Specify the non-mutating CEM-QL surface and exact types:
        - `url:can_parse(input, base?) -> boolean` mirrors `URL.canParse()`;
        - `url:href(input, base?) -> anyURI` mirrors construction plus canonical
          `href` serialization and reports invalid input;
        - `url:parse(input, base?) -> record?` returns `href`, `origin`,
          `protocol`, `username`, `password`, `host`, `hostname`, `port`,
          `pathname`, `search`, `hash`, and ordered duplicate-preserving query
          entries, returning an empty result for invalid input;
        - `url:assemble(parts, base?) -> anyURI` and
          `url:with_parts(input, parts) -> anyURI` provide immutable equivalents
          of construction and writable URL-property setters, with specified
          precedence or diagnostics for conflicting `href`, `host`/`hostname`,
          `port`, `search`, and query-entry inputs.
    - [ ] Cover the complete pure
          [`URLSearchParams`](https://developer.mozilla.org/en-US/docs/Web/API/URLSearchParams)
          surface with an ordered `stream<record(name, value)>` parameter value:
        - `url:params(init?) -> stream<record(name, value)>` mirrors construction
          from a raw search string, record, or sequence of two-item pairs; a
          leading `?` is stripped, but a complete URL string is not parsed;
        - `url:params_size(params) -> integer`,
          `url:params_entries(params) -> stream<record(name, value)>`,
          `url:params_keys(params) -> stream<string>`, and
          `url:params_values(params) -> stream<string>` cover `size`, the default
          iterator/`entries()`, `keys()`, and `values()` while retaining source
          order and duplicate names;
        - `url:params_get(params, name) -> string?`,
          `url:params_get_all(params, name) -> stream<string>`, and
          `url:params_has(params, name, value?) -> boolean` preserve first-value,
          all-values, and optional name/value matching behavior;
        - `url:params_append(params, name, value)`,
          `url:params_delete(params, name, value?)`,
          `url:params_set(params, name, value)`, and
          `url:params_sort(params)` return new ordered parameter streams as the
          immutable equivalents of the mutating methods;
        - `url:params_string(params) -> string` mirrors `toString()` without a
          leading `?`; `forEach()` maps to ordinary `seq:map`/stream iteration
          rather than a redundant callback-specific URL function.
    - [ ] Specify `URLSearchParams` edge semantics: stable key sorting, duplicate
          insertion order, `set()` retaining the first matching position while
          deleting later matches, optional-value `has()`/`delete()`, empty value
          versus missing `=`, decoded keys and values at the function boundary,
          literal `%` double-encoding, `+`-to-space parsing, space-to-`+`
          serialization, and the WHATWG `application/x-www-form-urlencoded`
          percent-encode set. Connect `url:parse(...).query` and
          `url:with_parts(..., { query: params })` without conflating raw search
          strings with complete URLs.
    - [ ] Keep `url:` distinct from `module_url()`: it never consults import
          maps, resource policies, node scopes, or an ambient page/template
          referrer. Relative input requires an explicit base; contextual module
          and resource resolution continues to use `module_url()`.
    - [ ] Define invalid-input and setter-failure diagnostics, component-specific
          percent-encoding, Unicode/IDNA behavior, default-port elision,
          dot-segment normalization, special and non-special schemes, opaque
          paths/origins, IPv4/IPv6 hosts, credentials, and `file:` URLs. Exclude
          stateful host-only `URL.createObjectURL()` and `URL.revokeObjectURL()`
          from the pure stdlib.
    - [ ] Implement one native semantic path shared by CLI, SSR, WASM, and
          browser execution; do not delegate conformance to the embedding
          platform's JavaScript `URL` implementation.
    - [ ] Add registry/type-checking/evaluation tests plus selected
          Web Platform Test vectors for parse/serialize/setter/query behavior,
          round-trip idempotence, invalid bases, and cross-runtime parity; add
          generated schema-package reference tables and an executable
          query/result fixture matrix.
- [x] Restore zero-hard-violation fixture validation by lowering legacy
      `//slice[//slice/event]` selection to the split CEM-QL slice/event-payload
      data model instead of leaving an XPath predicate in generated CEM-QL.
    - [x] Prove the material input and autocomplete event-gated value selectors
          compile through the native converter and default fixture-validation
          corpus.
    - Completed 2026-09-01: the bridge emits an event-payload-gated conditional
      whose empty branch preserves lazy `??` fallback behavior. All 72 focused
      legacy-converter tests pass, and the rebuilt CLI validates all 55 default
      fixtures with 158 warnings, zero errors/fatals/hard violations, and exit 0.
- [x] Fixture: make external-template example 4 render the complete native
      inert `<template>` data-island tree through its external
      CEMT, including arbitrary element attributes, dataset attributes, named
      and default slots, text, comments, and nested content without a
      rendered legacy-synthetic `datadom` wrapper or attribute-name whitelist.
    - Completed 2026-08-30: example 4 covers an explicit inert payload envelope
      and ordinary live payload capture. `tree.cemt` recursively exposes the
      resulting stable island's element, text, and comment nodes plus every
      serialized attribute through the generic Tier A `record:entries` helper.
- [x] Lifecycle: separate the authored instance payload envelope from the stable
      runtime-owned data-island tree, store browser hydration data in HTML/XML DOM
      form inside the island, prevent serialized rendered output from ever being
      recaptured as payload, support both render-on-load provisional content and
      identity-matched no-rerender adoption, and update example 4 to expose the
      complete island tree.
    - Completed 2026-08-30: the normative lifecycle principle now defines one
      stable marked island with distinct DOM-native state sections. First load
      consumes either live payload or one unmarked inert envelope; resume mode is
      selected by the marker before sibling inspection. DOM hydration data restores
      render slices, loading output is replaced when bounds are absent, and matching
      version/declaration/artifact/revision/policy/source identity retains SSR output
      without a first rerender. `tree.cemt` renders all eight island sections through
      `$island`. Build, lint, typecheck, 174 unit tests, 134 browser stories, 16
      Edge/SSR unit tests, 6 Edge/SSR stories, and the 15-page / 21-source-document
      demo gate pass.
- [x] Fixture: cover declaration-owned exact SemVer across registration identity,
      data-island/Edge serialization, compatible SSR adoption, incompatible SSR
      browser rerender, and collision-safe produced-instance introspection.
    - Completed 2026-09-09: optional `<cem-element version>` values validate as
      exact SemVer and participate in registration identity. Versioned islands
      carry `declarationVersion` independently from the island schema version;
      caret-compatible declarations retain SSR output, while missing, malformed,
      or incompatible versions rerender only from an understood island. Unit,
      browser, and the seven-case isolated Edge/SSR Storybook gate cover the
      contract without claiming the produced instance's `version` attribute.
- [x] Add executable stylesheet-ownership and anonymous-declaration fixtures for
      the historical low-specificity CSS baseline, since superseded by native
      `@scope`: declaration styles installed once under
      `:where(<cem-tag>)`, matching named scopes contribute separately through
      `:where([data-cem-scope])`, payload styles retain the stronger instance
      boundary, render identity moves to `data-cem-render-scope`, and anonymous
      declarations create a deterministic UUID-shaped tag plus adjacent instance.
- [x] Expand the CSS-scoping fixtures into a complete executable contract matrix:
      make the scoped-CSS samples page, a dedicated Storybook surface, and pure
      unit tests cover private, shared shorthand, explicit shared, mixed,
      invalid/mismatched, instance-owned, specificity, static-only, identity,
      fragment-reuse, and once-per-declaration cases.
    - Completed 2026-08-24: the pure scope resolver and selector-rewrite units,
      four dedicated CSF Next browser stories, and nine-section executable
      samples page cover the full matrix. The 148 unit tests, 128 Storybook
      tests, production Storybook build, and 15-page demo-fixture gate pass.
    - Refined 2026-08-24: the first sample now compares classless buttons inside
      and outside two DCE instances, with browser verification that the dashed
      green component border does not escape its private tag boundary.

### Native CSS `@scope` migration

- [x] Implement the accepted
      [CEM light-DOM CSS scope contract](./cem-ml-uid-and-scoped-css-design.md).
    - [x] Compile private declaration styles under a produced-tag `@scope` with
          nested-DCE and projected-descendant lower limits; translate `:host` to
          `:where(:scope)` and reject authored outer `@scope`.
    - [x] Replace public `data-cem-scope` styling with one declaration-owned
          `scope="name"` reflected to produced hosts, qualified by the direct
          instance data island; validate one static CSS identifier and restore
          declaration state after instance additions, changes, or removals.
    - [x] Preserve or stamp `slot="name"` on projected element roots, including
          `slot=""` for default roots, and expose stable `part="name"` only on
          component-owned internals.
    - [x] Require a direct inert `<template>` envelope for instance payload CSS,
          move its content into the distinct instance data island, install
          managed styles under an implicit parent-rooted `@scope`, and reject
          bare or mixed payload styles fail-closed.
    - [x] Remove `data-cem-instance-scope` without adding another instance-style
          marker; keep `data-cem-render-scope` and data-island identity internal.
    - [x] Enforce the `0-2-1` authored library-specificity ceiling and reject
          IDs, manufactured specificity, generated layers, and library
          `!important`.
    - [x] Update native artifacts, WASM/browser transport, SSR, and hydration so
          declaration, shared, instance, anonymous, and restored-scope behavior
          remains deterministic.
    - [x] Replace the legacy scoped-CSS unit, Storybook, and demo matrix with
          executable cases for scope proximity, explicit public overrides,
          nested DCEs, projection limits, inheritance, `<th scope>` collision,
          invalid/mutated scopes, inert payloads, marker removal, lifecycle,
          anonymous tags, specificity, source order, SSR, and hydration.
    - [x] Pass `cem_ql:test`, `cem-elements:test:unit`, `cem-elements:test`,
          `cem-elements:verify-demo-fixtures`, and
          `@epa-wg/cem-components:verify`; then remove migration warnings and
          change the normative contract status to accepted and implemented.
    - Completed 2026-08-27: native declaration/shared/instance scopes, public
      `scope`/`slot`/`part` hooks, specificity enforcement, inert payload
      adoption, marker removal, worker/Edge/SSR/hydration transport, and the
      expanded cascade/containment evidence are implemented. The required
      `cem_ql`, 150-unit-test, 129-Storybook-test, 15-page demo, and full
      58-dependency cem-components gates pass.
    - [x] Fixture: remove `uid-seed` from scoped-CSS samples that do not expose
          deterministic render identity, and add a keyframe sample proving that
          one explicit seed stabilizes both the internal render identity and the
          rewritten animation name.
        - Completed 2026-08-28: ordinary private/shared/instance/fragment samples
          now use runtime identity, while the new animated sample verifies that
          `demo/css/keyframes` appears in `data-cem-render-scope`, the rewritten
          `@keyframes` rule, its animation reference, and the computed animation
          name. All 15 browser demo fixtures pass.

### Remaining declarative UI migration

- [ ] Add a Storybook-owned accessible theme switcher composed from production
      `cem-select`, with exact five-mode global switching on the preview root and
      per-mode interaction/token-resolution evidence. Stories must not own or
      persist global theme state.
- [ ] Migrate every legacy `cem-components` member into its own
      `src/components/<cem-tag>/<cem-tag>.xhtml` folder with embedded,
      once-per-declaration scope-contract CEM-token `<style>` and colocated
      CSF Next `<cem-tag>.stories.ts` `play` tests, moving missing reusable behavior into
      `cem-elements`, until both migration targets are zero.
- [ ] Migrate remaining Studio and Site visible DOM construction, UI listeners,
      and state projection to XHTML/CEM-ML, retaining JavaScript only for non-UI
      services and host adapters.

## Phase 3 Checklist

- [x] Lock the `<cem-element>` declaration and registration contract.
    - [x] Decide whether CEM declarations use a scoped, inherited logical template
          registry while produced browser custom elements remain registered in the
          document's global `customElements` registry.
    - [x] Decide which names are globally unique, which references may be
          scope-local, and how duplicate declarations, inherited shadowing, and
          collisions with legacy `<custom-element>` declarations fail.
    - [x] Reconcile the decision with `AC-R-1` through `AC-R-3`, the reserved
          `cem-` component namespace, and the coexistence rule that browser tag
          names must not collide.
    - [x] Promote the accepted rules into `docs/cem-element-design.md` and
          executable contract tests; remove the two blocking `TBD` statements.
    - Completed 2026-08-18: adopted scoped/inherited logical lookup with
      document-global browser registration. Same-scope duplicates and incompatible
      inherited, legacy, foreign, or existing-CEM definitions fail before browser
      mutation; identical inherited or existing CEM registration identities reuse
      the one global definition. The pure decision core and 7 focused contract
      cases pass as part of all 94 `cem-elements:test:unit` tests.

- [x] Audit the existing Phase 3 substrate against the locked contract.
    - [x] Classify current `cem-elements` declaration-shape, data-document,
          disposition, projection, processing-boundary, runtime-support, Storybook,
          legacy, material, and edge/SSR fixtures as implemented, partial,
          placeholder, or deferred.
    - [x] Map every current resolved Nx target to the Phase 3A/3B/3C roadmap and
          move edge/SSR-only acceptance out of the Phase 3 browser gate where
          necessary.
    - [x] Repair or retire the inferred
          `test-ci--src/lib/cem-elements.declaration-shape.spec.ts` target, which
          currently selects the Storybook-only Vitest project and reports no unit
          test files; keep `cem-elements:test:unit` as the accepted unit gate until
          the resolved target topology is corrected.
    - [x] Restore the `cem-elements:lint` project baseline: it currently reports
          15 pre-existing module-boundary errors in the CEMT/Storybook sources and
          two unrelated warnings, while the registration-contract source and test
          lint clean in isolation.
    - [x] Add an explicit todo checkitem before adding any new parity or browser
          fixture discovered by the audit.
    - Completed 2026-08-18: classified the substrate and every resolved target in
      [`cem-elements-phase3-substrate-audit.md`](cem-elements-phase3-substrate-audit.md),
      retired the misconfigured inferred Vitest atomics only for `cem-elements`,
      restored a warning-free project lint gate, and separated the current
      `verify:phase3a` browser aggregate from opt-in `verify-edge-ssr` evidence.
      The audit added the focused registration-scope fixture checkitem below before
      creating any new fixture.

- [x] Lock the logical declaration-scope host API required by runtime registration.
    - [x] Decide how a host creates an explicit scope and supplies its optional
          parent: opaque runtime-owned scope objects, one default root per
          `Document`, explicit same-document host/parser parents, and no inference
          from arbitrary DOM ancestry.
    - [x] Associate both inline and external runtime declarations with the selected
          explicit scope or their document's default root.
    - [x] Define scope identity, document ownership, parent compatibility, lifetime,
          and disposal without conflating the scope with `scopePolicyStamp`.
    - [x] Define how identical inherited registrations reuse the parent declaration
          and document-global constructor while same-scope duplicates and
          incompatible shadows fail before browser mutation.
    - [x] Promote the accepted API and lifecycle into
          `docs/cem-element-design.md` and focused pure contract tests.
    - [x] Lock registration-identity derivation for `CemProducedElementBehavior`:
          require a non-empty host `behaviorIdentity` when behavior is supplied,
          include it with tag/source/language in the content address, and reject
          callback-source hashing or implicit object-identity reuse.
    - [x] Add the audit-identified browser registration-scope fixture after the
          API is locked; prove same-scope failure, identical inherited reuse, and
          incompatible inherited/browser collisions without adding unrelated parity
          fixtures.
    - Completed 2026-08-18: the runtime now selects explicit/default logical scopes
      for inline and external declarations, derives `cem-registration-v1` identities
      including required host behavior versions, invokes the pure decision core, and
      marks CEM-owned constructors before document-global definition. Four scope and
      nine registration cases pass within all 100 unit tests; the focused browser
      fixture passes within all 96 Storybook tests and proves same-scope rejection,
      inherited reuse, incompatible inherited/CEM/foreign browser collisions, and
      missing behavior identity without registry mutation.

- [x] Lock the Phase 3A processing-host and worker/fallback transition API.
    - [x] Decide whether the single dedicated worker is owned per logical root scope,
          per `CemElementRuntime`, or per browser `Document`. Recommended: per logical
          root scope so explicit child scopes share retained compatible artifacts and
          independent roots remain isolated.
    - [x] Define versioned structured-clone request/response envelopes with monotonic
          job IDs, full render revisions, diagnostics, retained artifact/plan handles,
          and explicit cancel/dispose messages.
    - [x] Define the worker construction seam for bundlers, CSP hosts, and browser
          tests. Recommended: an injectable module-worker factory with a package
          default, not a public worker instance or ambient global override.
    - [x] Define deterministic startup-failure and post-handshake execution-failure
          transitions to the same main-thread host interface, including which jobs
          may be retried and how duplicate commits are prevented.
    - [x] Promote the accepted host lifecycle and transition table into the design
          before creating the worker/fallback browser fixture.
    - Completed 2026-08-18: adopted one package-private host per logical root,
      `cem-processing-host-v1` clone-safe envelopes with monotonic IDs and retained
      handles, the shared compile/render-diff/cancel/dispose interface, and an
      injectable module-worker factory with a package default. The pure transition
      core retries compile and pre-commit render work exactly once through fallback,
      aborts begun transactions under a fresh `renderAttempt`, preserves committed
      jobs without replay, and suppresses late worker results; 9 focused cases pass
      within all 109 `cem-elements:test:unit` tests.

- [x] Implement the smallest tests-first Phase 3A browser vertical slice.
    - [x] Register one inline `<cem-element>` declaration under the locked name
          rules and capture one produced instance's author payload into an inert
          WHATWG template data island.
    - [x] Package the exact generated CEM-QL ESM/declaration/WASM assets beside the
          package-private runtime support, rewrite built imports package-locally,
          declare the assets as cached Nx outputs, and verify the npm archive
          inventory before adding the module-worker entry.
          Completed 2026-08-18: `cem-elements:verify-package` packs 53 files,
          verifies the exact 31,092,853-byte WASM artifact and local ESM imports,
          includes the processing engine/host/worker entries, excludes sources/build
          metadata, and imports the tarball from a clean temporary consumer; its
          build and verification targets restore from Nx cache.
    - [x] Compile/render through the existing CEM-ML/CEM-QL WASM boundary using
          one dedicated worker, with the same semantic result through the required
          main-thread fallback.
    - [x] Apply revision-checked patch frames on the main thread while preserving
          light-DOM identity, focus/selection state, and data-island isolation.
    - [x] Prove the vertical slice in Rust-first contract tests, TypeScript unit
          tests, and one executable Storybook/browser fixture.
    - Completed 2026-08-18: canonical inline CEM-ML now compiles/renders/diffs in
      one module worker per logical root, with retained artifact/render-plan handles
      and deterministic startup/execution fallback to the same main-thread engine.
      The main thread validates complete revisions and buffered transactions before
      DOM mutation, preserves render identity/focus/selection and behavior-owned
      attributes, retries target mismatch as a fresh `replaceScope` attempt, and
      leaves URI/resource and legacy declarations on their established paths. The
      accepted evidence is Rust-first parity 3/3, TypeScript unit 111/111, Storybook
      Chromium 97/97, typecheck/lint, and the 53-file clean-consumer package probe.

- [x] Add URI declarations and the Phase 1 `<http-request>` resource slice.
    - [x] Support declaration `src` for document-relative, fragment-only, absolute,
          and module-map identities under the host resolver and scope policy.
    - [x] Add remote/local streaming, abort/stale-response protection, JSON/XML
          projections, and the fixture-backed `cem:for-each` flow.
    - [x] Preserve the same artifact identity, worker/fallback semantics, source
          maps, diagnostics, and patch protocol as inline declarations.
    - Completed 2026-08-18: canonical fragment, document-relative, absolute, and
      module-map declarations now compile as clone-safe chunked text through the
      retained root-scope worker/fallback host. URI artifact cache identity includes
      source-ref, resolver, and scope-policy state, and imported resource controls use
      the imported source URL as their base. CEM-QL-rendered `<http-request>` nodes
      lower to explicit clone-safe controls before DOM diffing; the main thread retains
      resolver/policy, multi-chunk loader, abort/stale-revision, and patch-commit
      ownership. Template-visible HTTP states now use the portable lifecycle vocabulary,
      and executable JSON/XML projections drive the same `cem:for-each` path. Accepted
      evidence is the Rust-native resource-envelope test, TypeScript unit 113/113,
      Storybook Chromium 97/97, all 15 executable demo pages, typecheck/lint, and
      the 53-file clean-consumer package probe.

      Audit correction, 2026-09-17: that delivery streams transport bytes but
      still parses HTTP bodies into JavaScript JSON/XML records. It does not
      complete the native CEM-tree or progressive AST-stream contract. The
      accepted CEM-LOADER migration above replaces that document-data path.

- [x] Complete Phase 3A/3B/3C substrate parity.
    - [x] Wire superseded processing-host render jobs to the locked `cancel`
          operation while preserving revision checks and atomic patch recovery.
          Completed 2026-08-18: each canonical instance now retains its active
          render job ID and sends `cancel(reason: "superseded")` before a newer
          revision enters the root-scope host. Worker and main-thread modes share a
          bounded active/cancelled lifecycle, reject late cancelled results, accept
          only live targets, and forget terminal IDs. The existing worker browser
          fixture holds and releases an obsolete result late, then corrupts a patch
          target and proves only the fresh revision commits through atomic recovery;
          accepted evidence is unit 114/114 and Storybook Chromium 97/97.
    - [x] Prove legacy compatibility only through opt-in
          `lang="custom-element-v0"` fixtures.
          Completed 2026-08-18: the browser selector now enters the legacy converter
          only for the exact `lang="custom-element-v0"` annotation; explicit CEM-ML
          retains precedence, while untyped XSLT-shaped markup and the native
          `custom-element-xslt` engine identity stay on the DOM path. All 12 legacy
          fixture pairs carry the annotation and the inventory gate rejects any
          unannotated legacy template. Positive and negative browser evidence passes
          within Storybook Chromium 97/97, with selector coverage in unit 123/123;
          all 15 demos and the 56-file clean-consumer package probe also pass.
    - [x] Prove the full legacy and material parity inventories plus browser
          data-island isolation and accessibility gates.
        - [x] Promote all 12 file-backed legacy/CEM fixture pairs into
              one-to-one executable Storybook browser cases.
              Completed 2026-08-18: each manifest pair is imported directly
              from its checked-in legacy and CEM HTML files by a named browser
              story. The 12 cases exercise registration, declaration-shape
              rejection, local/external sources, payload, attributes and
              invalidation, slots, slice events, datadom migration,
              conditionals, and the exact legacy bridge; Storybook Chromium
              passes all 109 tests.
        - [x] Promote all 8 material/CEM fixture pairs into one-to-one
              executable Storybook browser cases.
              Completed 2026-08-18: each checked-in material source pair now
              runs in isolated same-origin documents so both sides retain the
              real `cem-*` browser names. The legacy side uses the documented
              thin adapter and exact v0 opt-in without modifying fixture bytes;
              manifest-ordered dependencies, local/external declarations,
              module URLs, composition, slots, data payloads, and migrated
              slice interactions pass within Storybook Chromium 117/117.
        - [x] Prove raw declaration templates and captured data islands remain
              inert to layout, selectors, forms, accessibility, and visible text.
              Completed 2026-08-18: the browser isolation matrix places both
              raw sources inside a live form and proves query/tag collections,
              layout boxes, active styles, visible text, form ownership/data,
              focus, and document/accessibility exposure contain only the
              rendered projection. Worker, startup-fallback, and
              execution-fallback rerenders preserve the same island boundary;
              Storybook Chromium passes all 118 tests.
        - [x] Enforce the Phase 3 accessibility contract across the complete
              legacy and material browser inventories.
              Completed 2026-08-18: one cross-document browser audit now runs
              over all 12 legacy/CEM pairs and all 8 isolated material/CEM
              pairs (40 rendered sides), including every legacy post-mutation
              and post-event checkpoint. It enforces accessible names, native
              roles and focusability, single-tab-stop ownership, unique IDs,
              resolved label/ARIA references, valid reflected ARIA state, and
              image alternatives. Material action/disclosure cases additionally
              exercise native activation and reflected state; named authored
              inputs keep the unchanged legacy input/autocomplete fixture bytes
              conformant. Storybook Chromium passes all 118 tests.
        - [x] Include every parity fixture in the Phase 2 CLI validation,
              end-to-end, and benchmark aggregate gates.
              Completed 2026-08-18: the CLI now extracts both parity manifests,
              resolves external fragments, and lowers legacy templates through the
              Rust converter into 45 schema-profile inputs alongside the 10 Phase 2
              base fixtures. Validation reports 55 inputs with no errors, fatals, or
              hard violations; the CLI end-to-end gate passes, and the benchmark
              asserts all 40 source sides while retaining the AC-N-1 budget.
            - [x] Add package-owned pass/fail fixtures for the dedicated
                  `cem-element-template/v1` schema profile before routing the
                  parity manifests through it.
                  Completed 2026-08-18: the registered schema package owns its
                  manifest, schema, valid and invalid examples, generated README,
                  Nx verification target, Rust manifest-index test, and CLI
                  pass/fail validation coverage.
    - [x] Add the Phase 3B scope-policy worker pool, content-addressed cache, and
          deterministic scheduling traces behind the stable host API.
          Completed 2026-08-19: compatible logical roots share lazily allocated
          worker slots bounded by hardware concurrency and an eight-worker host cap,
          with a 64-operation per-slot queue, FIFO-per-root ordering, round-robin
          cross-root dispatch, pool-global job IDs, and preemptive cancel controls.
          Template compilations and render plans use 64-entry content-addressed LRU
          retention in both worker and main-thread modes; artifact aliases refresh
          before rendering and evicted plans safely degrade to `replaceScope`.
          Clone-safe sequence-only traces expose enqueue, dispatch, cancellation,
          overflow, and fallback decisions without affecting execution. Accepted
          evidence is typecheck/lint, 132/132 unit tests, 118/118 Storybook Chromium
          tests, the 62-file clean package probe, and the 48-task
          `verify:phase3b` aggregate.
    - [x] Add Phase 3C precompiled component-template artifacts without removing
          the source-driven runtime path.
          Completed 2026-08-19: deterministic `cem-template-artifact/1`
          MessagePack envelopes retain compiled CEM-ML/CEM-QL IR across native
          and WASM reloads and bind their source hash, host bindings, source-map
          mode, compiler version, and IR version. The browser transfer binds the
          active policy stamp, and all mismatches are rejected before rendering.
          The browser host supports opt-in registry reads and write-through while
          preserving source compilation as the warning-backed fallback. Two
          isolated engines prove registry
          miss/compile/store followed by byte-only import/render. Accepted
          evidence is the 4/4 native artifact fixture, all 204 `cem_ql` tests,
          135/135 unit tests, 119/119 Storybook Chromium tests, the 62-file clean
          package probe, and the uncached 50-dependency `verify:phase3c`
          aggregate.
        - [x] Add a native component-template artifact fixture covering binary
              compile/reload render parity plus hash, source, binding, mode, and
              policy rejection.

- [x] Author the Phase 3 primitive set exclusively on the accepted substrate.
    - [x] Extend the existing component test harness for substrate rendering,
          events, forms, accessibility, and visual snapshots.
        - [x] Add a Chromium action/field fixture that registers real CEM-ML
              declarations, proves render and re-render settlement, slice-driven
              events, native form data/reset/validity, accessibility assertions,
              a deterministic structural/visual baseline, and screenshot capture.
        - Completed 2026-08-19: `createSubstrateComponentHarness()` now owns
          runtime-aware declaration registration, rejects missing behavior
          identities and hard diagnostics, awaits declaration/render/re-render
          settlement, exposes data-island and native-form snapshots, and normalizes
          volatile runtime metadata out of reviewed visual baselines. The real
          action/field Chromium fixture proves light-DOM accessibility, custom
          event details, slice updates, native form data/reset/validity, focus,
          geometry/computed styles, and browser screenshot capture. The cached
          `verify-phase3-harness` target runs package lint/typecheck plus the focused
          fixture, and the full component test target sequences it before starting
          its own browser run.
    - [x] Wire action, field, surface, text, icon, stack, grid, list, nav, and
          dialog shell primitives through `<cem-element>` with no legacy runtime
          dependency.
        - [x] Give every behavior-backed primitive a stable versioned host identity
              and make primitive installation await declaration settlement before
              reporting registered/skipped tags or diagnostics.
        - [x] Extend the Chromium primitive fixture to prove the exact ten minimal
              Phase 3 tags register and render as accessible light DOM through the
              accepted substrate.
        - Completed 2026-08-19: all 16 behavior-backed declarations now supply
          stable versioned host identities, the public installer resolves only
          after every accepted declaration settles, and its ordered result reports
          only browser-registered tags plus complete diagnostics. The static gate
          verifies all 48 manifest entries, while the focused Chromium fixture's
          10 tests prove the exact minimal set renders accessible light DOM with
          inert data islands and no shadow or legacy runtime dependency.
    - [x] Run the substrate, component, CEM-ML fixture, and accessibility aggregate
          gates before closing Phase 3.
        - [x] Repair stable native-owner reconciliation for behavior-backed CEM-ML
              re-renders and migrate their browser fixtures to explicit runtime
              settlement. Truthful behavior registration currently exposes 38
              failing component assertions across native identity, focus, ARIA,
              interaction state, and nested workflow projection.
            - [x] Add a focused `cem-elements` patch-frame regression proving a
                  conditional child insertion preserves compatible native sibling
                  identity and focus while updating the surrounding render plan.
        - Completed 2026-08-19: render-engine patch transport `1.1.0` adds
          namespace-preserving child reconciliation so conditional insertions retain
          compatible native owners, focus, and state instead of replacing their
          parent. Retained render-plan directive values keep slice event bindings
          current after their visible attributes are consumed, and the component
          harness now awaits nested runtime settlement before assertions. The focused
          patch-frame unit and Storybook regressions pass; all 38 exposed component
          failures are repaired. Accepted evidence is 136/136 `cem-elements` unit
          tests, 120/120 Storybook Chromium tests, 125/125 component tests, the
          uncached 51-dependency `cem-elements:verify` aggregate, and the uncached
          41-dependency `@epa-wg/cem-components:verify` aggregate.

## Deferred Roadmap Work

The Edge/SSR host fixtures belong to Phase 3.5 after the browser substrate is
stable. Moving `@epa-wg/custom-element` into the monorepo and deciding final
legacy XSLT preservation belong to Phase 3.6. Swift/Xcode plus Kotlin/Compose
compile gates remain Phase 8. Live Figma UI Kit and prototype work is deferred
until final Phases 10 and 11, after Phase 9 release governance.

- [x] Move all remaining live Figma library and prototype updates behind the
      CEM Site, Studio, native-package, and release-governance phases.
      Completed 2026-08-19: `roadmap.md` now makes the pull-only Figma UI Kit and
      site demo final Phases 10 and 11, and this checklist activates Phase 6 while
      preserving completed credential-free Figma preparation at the end. No live
      Figma state changed; generated artifacts remain downstream projections of
      the canonical theme Markdown specifications.

- [x] Before activating Phase 3.5, split its six Storybook cases and supporting
      processing-boundary selections from the shared Phase 3A files so
      `verify-edge-ssr` has phase-specific evidence instead of relying on the broad
      browser and unit targets.
      Completed 2026-08-19: a dedicated Edge/SSR Storybook configuration now
      registers exactly the three hydration, edge-patch, privacy-export, and hybrid
      state cases (6/6), while a focused unit configuration owns the three
      structured-clone, default-deny/redaction, and host-value rejection cases plus
      the accepted hybrid storage and external host-envelope contracts, plus eleven
      Node-only initial SSR, edge-update, and browser-export boundary cases across
      four focused files (16/16). The default
      Phase 3 lanes exclude those deferred cases and pass at
      114/114 Storybook and 133/133 unit tests. The uncached five-dependency
      `cem-elements:verify-edge-ssr` aggregate passes without depending on either
      broad test target; lint and typecheck are also green.

## Phase 3.5 Checklist

- [x] Lock the external Edge/SSR host and render-state contract.
    - [x] Decide whether the existing
          `content-addressed-cache-with-revision-pointer-v1` model is the accepted
          roadmap "both" option, or whether cache-only or revisioned KV/document
          storage replaces it.
          Completed 2026-08-19: accepted the existing hybrid model without changing
          the `edge-render-state` 1.0.0 wire format. Immutable template artifacts,
          render plans, sanitized snapshots, and rendered HTML remain independently
          content-addressed and verified on read; one stable per-instance pointer
          record carries current addresses, revision/policy identity, and an ETag.
          Pointer writes use expected-ETag compare-and-swap, retain the current record
          on mismatch, and may leave unreachable immutable blobs for adapter-managed
          retention. The normative design and roadmap now reject cache-only and
          pointer-only storage for this version.
    - [x] Lock clone-safe host requests/results for initial SSR output, hydration
          metadata, previous render-plan identity, and streamed edge patch frames.
          Completed 2026-08-19: the public Edge/SSR profile now reuses the
          `cem-processing-host-v1` protocol version, monotonic job IDs, correlation,
          and diagnostic lifecycle without expanding browser-worker capabilities.
          `render-initial` returns owned-range HTML, versioned hydration metadata,
          and committed render state. `render-update` requires the previous state
          key, ETag, plan identity, and content address; it streams one progress
          envelope per patch frame and returns the next plan and pointer only at the
          terminal result. Compare-and-swap succeeds before `commit`, conflicts
          terminate without `commit`, failures are typed, and every nested value is
          checked for plain structured-clone transport. The host can accept serialized
          source, a compiled artifact transfer, or a content-addressed artifact, but
          cannot reconstruct policy-omitted snapshot fields.
- [x] Add a non-browser SSR host fixture that emits initial HTML plus hydration
      metadata from a serialized `DataIslandSnapshot` and validates template
      artifact identity, `RenderRevision`, source-map mode, and retained render-plan
      identity before hydration.
      Completed 2026-08-19: a Node-environment `render-initial` reference host now
      projects serialized template source and a complete sanitized snapshot without
      DOM globals, applies deterministic scope and dev/prod source-map policy,
      serializes escaped identity-bearing owned-range HTML, writes the hybrid state,
      and rereads the retained plan before returning `cem-ssr-hydration-v1` metadata.
      It rejects mismatched template/revision/source-map/scope identities before
      storage, fails closed when privacy policy omitted render-required fields,
      reports unresolved artifact forms without guessing, rejects unsafe raw HTML,
      and uses an atomic `ifAbsent` precondition so duplicate or concurrent initial
      renders cannot replace the existing pointer.
- [x] Add a DOM-free edge processing fixture that accepts a serialized snapshot plus
      previous render-plan identity and emits the same deterministic patch-frame
      stream as the browser reference runtime.
      Completed 2026-08-19: the Node-environment `render-update` reference host
      validates the state key, expected ETag, retained plan address, previous plan
      identity, and content-addressed template before projecting without DOM globals.
      Its async response stream emits the exact reference `begin`, `ops`, and
      `commit` patch frames only after compare-and-swap advancement and a verified
      state reread, followed by one terminal result. Focused evidence proves exact
      frame parity plus fail-closed missing-state, stale-ETag, address, identity, and
      unavailable-content outcomes; each failure emits one typed terminal response,
      no progress or commit, and leaves the pointer unchanged.
- [x] Prove privacy/export policy is applied before the serialized snapshot crosses
      the browser boundary, including omission and redaction cases in both host
      fixtures.
      Completed 2026-08-19: the public browser request factory accepts a raw
      `DataIslandSnapshot` plus export policy, applies default-deny or canonical
      redaction into an owned clone, removes the policy, and only then creates the
      structured-clone host envelope. Initial and update evidence proves omitted
      secrets never enter requests, redacted requests remain isolated from later
      browser mutations, and hosts retain or return exactly the exported snapshot
      without reconstructing denied fields. Omitted render-required input terminates
      as `privacy-policy-rejected`; initial storage remains absent and update storage
      and progress remain unchanged.
- [x] Run the Phase 3 browser reference gates and the opt-in `verify-edge-ssr`
      aggregate before closing Phase 3.5.
      Completed 2026-08-19: the uncached 51-dependent-task
      `cem-elements:verify` aggregate passed through Phase 3A, Phase 3B, and Phase
      3C with 114/114 browser cases, 133/133 unit cases, the clean 71-file npm
      package, CEM-ML fixture/e2e/benchmark coverage, parity inventories, demos,
      scheduling/cache evidence, and native template-artifact reload coverage.
      The separate uncached five-dependent-task `verify-edge-ssr` aggregate passed
      16/16 focused unit/host cases, 6/6 dedicated Storybook cases, and all four
      substrate fixtures. Neither lane depends on the other's broad test targets,
      so Phase 3.5 closes with the opt-in boundary intact.

## Phase 3.6 Checklist

- [x] Inventory the external `@epa-wg/custom-element` package before migration.
    - [x] Record `~/aWork/custom-element/` repository cleanliness, branches,
          remotes, tags, and history shape without mutating the external checkout.
    - [x] Record the published package identity, version, exports, packed files,
          build/test/release targets, and current fixture surface.
    - [x] Map runtime responsibilities to reusable `cem-element` substrate
          boundaries and identify package-specific public compatibility behavior.
    - [x] Add an explicit checklist item for every new migration or parity fixture
          discovered by the inventory before creating that fixture.
    - Completed 2026-08-19: recorded the clean but divergent source and distribution
      repositories, the npm `0.0.39` package/archive contract, the 88-story plus
      three-unit behavioral reference, and the runtime ownership map in
      [`custom-element-phase3.6-inventory.md`](custom-element-phase3.6-inventory.md).
      The audit also found that `packages/custom-element/` is an existing
      snapshot-based adapter with no imported external Git objects, so the next
      item must join two valuable histories rather than import into an empty path.
- [x] Lock the history-preserving import mechanics and monorepo package boundary
      from the inventory evidence before copying source.
    - [x] Select and document the Git history-import method, retained refs, and
          rollback/check procedure.
    - [x] Define imported source, generated-output exclusions, npm identity,
          package exports, and Nx ownership for `packages/custom-element/`.
    - Completed 2026-08-19: accepted and rehearsed the exact isolated
      `filter-branch` path rewrite plus tree-neutral three-parent `ours` join in
      [`custom-element-history-import-plan.md`](custom-element-history-import-plan.md).
      The rehearsal retained all 282 commits, produced the pinned rewritten
      main/develop/root hashes, namespaced all 32 real tags, kept the current
      monorepo tree byte-identical, and passed ancestry and Git-integrity checks.
      The boundary also locks the curated `dist/` publisher root, stable npm/tag
      surface, next-major helper policy, canonical source manifest, and resolved
      Nx build/test/release ownership. No external or product repository history
      was changed by the rehearsal.
- [x] Import `@epa-wg/custom-element` into `packages/custom-element/` with the
      accepted history and published npm identity intact.
    - Completed 2026-08-19: joined the path-prefixed external source graph through
      tree-neutral three-parent commit `dfe142be`, retaining all 282 commits,
      rewritten `main` and npm-`0.0.39` `develop` tips, 32 namespaced release tags,
      and two permanent source-tip tags without changing the active package tree.
      The Git database, parent topology, path prefix, ref targets, and first-parent
      tree identity pass the package-owned provenance gate.
- [x] Add the inventory-discovered migration and parity fixtures before claiming
      package adoption.
    - [x] Add a history-provenance gate for retained rewritten refs, namespaced
          tags, external-tip reachability, and connection to the existing adapter
          history.
        - Completed 2026-08-19: the uncached
          `@epa-wg/custom-element:verify-history` Nx target validates the immutable
          manifest's join/tree/parent identities, 282-commit graph, rewritten root
          and tips, exact 32+2 tag targets, package path prefix, current-HEAD
          reachability, and exclusion of the distribution repository graph.
    - [x] Add an external-reference manifest and verifier mapping all 88 browser
          stories and three real unit cases to accepted, package-adapter, or
          explicitly rejected bridge evidence.
        - Completed 2026-08-19: pinned the distribution `develop` tree and all 18
          contributing blob IDs in a CI-local manifest covering 87 exported
          Storybook cases, one import-map browser harness, and three real helper
          unit cases. The cached `@epa-wg/custom-element:verify-reference-corpus`
          Nx gate locks the 88+3 identities and category counts, requires every
          case to resolve to existing evidence or an explicit adapter requirement,
          and records 61 accepted, 29 package-adapter, and one rejected-bridge
          mapping without importing the distribution repository history.
    - [x] Extend the existing source/dist public-adapter browser fixture with
          multi-event/multi-slice updates, checkbox/radio coercion, form/custom
          validity, scoped-style containment, and DOM identity/focus on rerender.
        - Completed 2026-08-19: the shared source/dist browser fixture now drives
          the public settlement APIs through multi-event arithmetic, slice
          fan-out, explicit checkbox/radio values, live form-data mirrors,
          form/control custom validity, declaration and per-instance payload CSS
          containment, and retained identity/focus/selection across slice and
          host-attribute rerenders. The external-reference gate now distinguishes
          the 20 package-adapter cases proved by this matrix from nine still-open
          helper, upward-propagation, and exported-attribute policy cases.
    - [x] Add an actual packed-archive clean-consumer gate for intentional files,
          private/generated exclusions, root/subpath JS and type contracts, and
          browser loading from the packed artifact.
        - Completed 2026-08-19: replaced the broad package wildcard with a
          checked-in release-root allowlist and a 146-path SHA-256 inventory,
          stripped workspace scripts from the generated consumer manifest, and
          added cached `@epa-wg/custom-element:verify-packed-archive` ownership.
          The gate injects representative private/generated sentinels into a
          temporary copy of the actual `dist/` root, creates a real npm tarball,
          proves exact contents and dependency-free/private-vendor exports,
          installs it into a clean temporary consumer, compiles root and
          `./CustomElement` imports with TypeScript 6, and renders canonical
          CEM-ML from the installed archive in Chromium. Conditional `types`
          exports repair the root/subpath resolution defect found by that gate
          without changing either JavaScript target.
- [x] Rebuild the next-major `<custom-element>` implementation on the
      `cem-element` substrate without retaining a separate parser/render engine.
    - [x] Preserve and prove the accepted non-engine helper surface through the
          public source, built package, and clean archive consumer.
        - Completed 2026-08-19: restored and typed `cloneAs`, `deepEqual`,
          `mergeAttr`, `mix`, `obj2node`, `tagUid`, `xml2dom`, and `xmlString` on
          both public entry points. The shared browser smoke fixture proves their
          behavior against source and `dist/`, while the packed-archive gate
          compiles all eight imports in a clean TypeScript consumer. The locked
          external corpus now records 26 verified package-adapter cases and 3
          remaining policy cases without adding a parser or rendering path.
    - [x] Decide the next-major policy for upward attribute propagation and
          retained exported attributes before changing either behavior.
        - [x] Add source/dist public-adapter assertions for one-way declared,
              selected, and slice state plus both retired retention markers.
        - Completed 2026-08-19: explicitly retired implicit child-to-host
          propagation and the `dceExportedAttributes`/
          `dce-exported-attributes` retention allowlists. Host attributes remain
          author/script inputs unless an explicit substrate behavior owns a
          mutation, and `mergeAttr` now has one exact-set contract. The public
          source/dist browser gate proves the negative compatibility boundary;
          all 29 package-adapter cases are verified and none remain required.
- [x] Keep or retire `<template lang="custom-element-v0">` only after the explicit
      migration fixtures provide compatibility evidence.
    - [x] Add public source/dist evidence that an explicit deprecated selector is
          preserved and renders through the same substrate data-island path.
    - Completed 2026-08-19: retained the exact selector as deprecated through the
      next-major migration window. Twelve manifest-backed legacy/CEM-ML pairs,
      selector precedence/negative cases, and the public source/dist fixture prove
      conversion and rendering through the single CEM-ML/CEM-QL substrate; the
      browser XSLT engine and `custom-element-xslt` browser alias remain excluded.
      Removal now requires canonical replacements for retained demos, material
      components, and downstream generators plus the governed FF-5 exit gate.
      FF-5 stays blocking while narrowly allowlisting the two XSLT schema policy
      records whose text explicitly forbids browser `XSLTProcessor` delegation.
- [x] Run legacy, material, Edge/SSR, and custom-element package gates together
      before closing Phase 3.6 and retiring `@epa-wg/cem-elements` as the staging
      migration target.
    - [x] Add and execute one root `@epa-wg/cem:verify:phase3.6` Nx aggregate
          without merging the browser and isolated Edge/SSR lanes.
    - Completed 2026-08-19: the new root closure target passed with all 64
      dependencies, aggregating `cem-elements:verify`, the separately owned
      `cem-elements:verify-edge-ssr` lane, and
      `@epa-wg/custom-element:verify`. The browser lane passed 114/114 Storybook
      and 133/133 unit cases, legacy/material inventories, the clean 71-file
      substrate package, demos, CEMT pipeline, CEM-ML CLI/e2e/bench, and native
      template-artifact evidence. The isolated lane remained separate and passed
      6/6 Storybook plus 16/16 unit/host cases. The adopted custom-element package
      passed its 282-commit/32-version-tag/2-source-tip provenance gate, complete
      88-browser/3-unit reference corpus with all 29 adapter cases verified, and
      the locked 146-file packed-archive contract. The first aggregate run exposed
      a load-sensitive material-icon story wait: its 120-frame budget expired
      while 63 sibling tasks passed, but the same 114 cases passed in isolation.
      Replacing only element discovery's frame count with a bounded 10-second
      elapsed-time deadline made the focused suite and the full aggregate green;
      the final run restored 43 of 65 tasks from Nx cache. Phase 3.6 retires
      `@epa-wg/cem-elements` only as the staging migration target:
      `@epa-wg/custom-element` now adopts it, while `@epa-wg/cem-elements`
      remains the shared substrate and package rather than being deleted.

## Phase 6 Checklist

- [x] Establish graph-native Markdown-to-HTML conversion before choosing or
      scaffolding the site application.
    - [x] Declare the Markdown-to-HTML edge in the Markdown schema package so
          conversion planning selects a named, typed converter rather than an
          export-time format guess.
    - [x] Add an explicit `convert` transform-graph node that preserves glob
          variants and produces chainable typed HTML artifacts.
    - [x] Reuse the Markdown lifecycle AST and CEM-ML output pipeline without a
          JSON intermediary, with diagnostics and provenance covered by focused
          Rust and CLI tests.
    - Completed 2026-08-19: the Markdown schema package now owns the ready
      `markdown-to-html-rust` edge, explicit graph `convert` nodes resolve that
      edge by typed source/target identity, and glob variants remain distinct
      through HTML export. The native Markdown AST feeds the renderer without a
      JSON bridge, while HTML artifacts and `.html.map` sidecars retain source
      frames and output spans. Focused parser, registry, schema-package, direct
      conversion, and graph CLI tests pass; the aggregate `cem_ml:test` suite
      passes, and all 517 `cem_ml_cli:test` library cases pass (515 in the
      restricted sandbox plus the two loopback HTTP cases with socket access).
- [x] Inventory the current root, package, generated-documentation, and example
      surfaces, then lock the CEM Site project and content-ownership boundary
      before scaffolding.
    - [x] Map reusable authored Markdown, generated token/API reports, component
          examples, release notes, and existing browser entry points.
    - [x] Record the evidence-based choice between `apps/cem-site` and a static
          docs application, including build, routing, deployment, and Nx target
          ownership.
    - [x] Define authored-versus-generated content ownership so the site never
          becomes a second source for token, API, or component contracts.
    - Completed 2026-08-19: [`cem-site-phase6-inventory.md`](cem-site-phase6-inventory.md)
      records the 41-project resolved Nx graph, the absence of a site/serve
      owner, the existing Storybook and two generated-doc targets, authored
      and generated content families, 142 inspected example assets, browser
      entry points, missing API/site gates, and an eight-rule ownership
      contract. Theme Markdown specifications remain canonical; generated
      and Figma data remain read-only projections. The accepted boundary is a
      dedicated static-output `apps/cem-site` Nx application whose cached build
      target invokes the CEM-ML CLI transformation graph. CEM-ML, not Vite or an
      asset-copy script, owns site content and web dependency assembly; Vite may
      remain an optional generated-output server/test harness. No application was
      scaffolded.
- [x] Make npm/browser dependency assembly a native CEM-ML web-build graph
      before scaffolding the site shell.
    - [x] Define a dedicated CEM-ML module-map content type and schema. Its
          authored `imports` entries are the complete JavaScript dependency
          manifest: source maps resolve explicit npm files (including paths in
          `node_modules`), destination maps declare browser-facing module URLs,
          and CEM-ML does not discover dependencies by parsing JavaScript.
    - [x] Lower dedicated source/destination module maps into ordinary typed
          JavaScript graph imports and exports, copy only declared source assets
          beside the exported app HTML, and project only destination URLs into
          the browser import map.
    - [x] Reject missing destination entries, undeclared/non-JavaScript assets,
          non-bare module specifiers, escaping output paths, and destination
          collisions with stable native diagnostics.
    - [x] Prove with Rust-native and CLI graph fixtures that declared npm assets
          are byte-preserved in the clean output, no `node_modules` path leaks
          into browser output, and undeclared JavaScript is not copied.
        - Completed 2026-08-19: schema package
          `cem_ml_schema_package_module_map_v1` owns
          `application/vnd.cem.module-map+json` and
          `https://cem.dev/ns/data/module-map/1`. Paired maps now lower each
          exact declared `.js`/`.mjs` source into an opaque `text/javascript`
          graph import and a byte-preserving export relative to the HTML
          destination, while the HTML import map receives only app-relative
          destination URLs. Native negative coverage rejects mismatched keys,
          relative/URL/prefix specifiers, non-JavaScript or escaping targets,
          and destination collisions. The CLI fixture copies the declared npm
          asset, omits an undeclared sibling, and leaves no `node_modules` path
          in the emitted HTML.
    - [x] Emit a deterministic module-asset read/dependency manifest with content
          digests so Nx cache inputs and provenance auditing cover every declared
          JavaScript resource.
        - [x] Add native fixtures for stable specifier/source-map/source/target/
              destination records, byte lengths, per-asset SHA-256 digests, and a
              host-neutral aggregate cache key.
        - [x] Carry the manifest through the common transform-graph response and
              CLI JSON/CEM/Markdown reports so native, WASM, and CLI hosts expose
              the same provenance evidence.
        - [x] Add a read-only CLI cache-key projection and prove an Nx runtime
              input consumes it without parsing module maps in JavaScript or
              performing transform output writes during hashing.
        - Completed 2026-08-20: graph lowering now hashes each declared module
          asset once and emits ordered provenance records through the common
          response and every CLI report projection. The aggregate SHA-256 uses
          host-neutral specifier, target, content-type, length, and content-digest
          fields while reports retain resolved source-map, source, and destination
          URIs. `--module-asset-cache-key` stops after lowering and performs no
          output writes. The module-map Nx project consumes that projection as a
          runtime input over a real `lit` dependency; dependency-output hashing
          ensures the native CLI exists before Nx evaluates the runtime key.
- [x] Create the root-wired CEM Site shell with stable routes and generated-doc
      ingestion from public package/report boundaries.
    - [x] Add a Rust-native graph fixture for Markdown → HTML → recovered DOM →
          CEMT layout composition, including escaped-text regression coverage,
          native artifact ownership, diagnostics, and source-map provenance.
    - [x] Execute the registered `Html5RecoveryConverter` in transform graphs
          and expose its typed DOM projection directly to CEMT without raw HTML,
          JSON, or an untyped string bridge.
    - [x] Scaffold `apps/cem-site` as a cached CEM-ML CLI application with an
          explicit Hugo-like publication graph, stable root/reference routes,
          and generated documentation consumed from an upstream Nx output.
    - [x] Verify the clean static output, route links, generated-doc ownership,
          native transform report/source maps, and Nx cache replay.
    - Completed 2026-08-20: transform graphs now execute the registered HTML5
      recovery converter and retain `HtmlDocumentAst` as a native DOM projection.
      CEMT receives a borrowed hierarchical query view over that owner, including
      native event/attribute ranges, without a raw HTML, JSON, or replacement-tree
      handoff. Rust unit and adapter suites pass, the CLI's 519 pre-existing cases
      passed in the aggregate run, and the corrected end-to-end native-layout
      fixture passes as the 520th case. `apps/cem-site` now publishes explicit
      root, guides, and generated CEM-ML reference routes through the native CLI;
      its Nx verification proves the exact clean output allowlist, internal links,
      upstream generated-doc ownership, graph report, CEMT source-map spans, and a
      four-of-four local cache replay. The shell intentionally has no JavaScript;
      later interactive work must use the dedicated module-map graph contract.
- [x] Build the guides, token browser, component gallery, examples, API/reference,
      and release-notes surfaces without duplicating canonical source content.
    - [x] Extend the checked-in publication graph with an explicit route/source
          allowlist for the first authored guides and package-owned references;
          exclude archives, active planning documents, debug projections, and
          directory-wide copies.
        - Completed 2026-08-20: `apps/cem-site/site.routes.json` now records the
          exact source, route/output, source kind, owning Nx project, upstream
          generation target, and graph import/export identities for five pages.
          The first external authored surfaces are the
          `@epa-wg/cem-ml-cli` browser/Node usage guide and the
          `@epa-wg/cem-ml` WASM runtime reference; the existing transform-config
          reference remains generated only by `cem_ml:build:docs`. Nx hashes the
          package-owned Markdown directly, while the verifier rejects duplicate
          routes, missing owners/targets, graph/report drift, directory-wide or
          excluded archive/planning/temporary/Figma/debug sources, unexpected
          outputs, unpublished internal links, and missing CEMT source maps.
    - [x] Add a static token-browser surface sourced from canonical theme Markdown
          and public `@epa-wg/cem-theme:build:tokens` outputs, keeping generated
          values read-only and excluding Figma and debug token projections.
        - Completed 2026-08-20: the theme target now emits the public beta
          `cem.tokens.catalog.json` package export from the same sorted metadata
          records as `cem.tokens.ts`. The native CEM-ML site graph imports that
          JSON directly and renders 487 visual/voice records at `/tokens/`
          through CEMT with no JavaScript. The route allowlist names all ten
          canonical Markdown specifications exactly; verification maps every
          catalog record to its source-table heading, enforces the generated
          owner/target relationship, rejects Figma and intermediate/resolved
          debug inputs, and checks clean output, links, and render provenance.
          The uncached theme/token/site build and the full site verifier pass;
          a subsequent verification replay restored all 10 tasks from local Nx
          cache, and the final verifier-only edit restored 9 of 10. Focused
          `cem-site:lint` and `@epa-wg/cem-theme:lint` targets pass.
    - [x] Add a component-gallery catalog from component semantics, inventory
          reports, and separately owned Storybook/example links without copying
          executable component fixtures into site-owned content.
        - Completed 2026-08-20: the cached public
          `@epa-wg/cem-components:build:catalog` target now validates the
          canonical MVP semantics, 48 executable primitive declarations,
          package reference/conventions/accessibility guidance, and the
          generated 40-of-40 covered state-matrix report, then emits the public
          beta `cem.components.catalog.json` package export deterministically.
          The native site graph renders all 48 records at `/components/` with
          category states, token families, 48 package reference links, eight
          exact package-owned example source links, and the separately owned
          local `cem-elements:storybook`/`build-storybook` commands and source
          link. It copies no declaration or example markup, ships no JavaScript,
          consumes no Figma projection, and records canonical versus generated
          evidence sources separately. Site verification and the 66-file npm
          package inventory pass; two uncached catalog generations have the same
          SHA-256 digest, and the final Nx site replay restores all 13 tasks from
          local cache.
    - [x] Add example, API/reference, and release-note routes from their current
          owners, using an explicit authored-reference policy where no generated
          API projection exists and never presenting an absent generator as one.
        - Completed 2026-08-20: the native publication graph now adds eight
          owner-sourced pages: canonical CEM-ML and component example indexes,
          the package-authored component reference, the workspace release notes,
          and component, element, theme, and custom-element release notes. Every
          allowlisted page declares a content role and relative-link policy;
          `authored-reference` requires authored input with no upstream target,
          while `generated-reference` requires a generated input and scheduled
          Nx target. The shared CEMT layout rewrites 40 repository-relative links
          to the established canonical `develop` source tree without changing
          site-root navigation or copying executable example markup. All 15 site
          routes build and pass the Nx site verifier.
    - [x] Prove every new route records its canonical owner/upstream Nx target and
          that publication remains deterministic from a clean output directory.
        - Completed 2026-08-20: site verification now reads the resolved Nx
          project graph, assigns every source to its unique deepest project root,
          rejects owner drift, and requires each generated route's upstream target
          to exist on that same owner and be scheduled by `cem-site:build`. The
          cached `cem-site:verify:determinism` target removes the output directory,
          runs the real native publication twice, rejects any unexpected output,
          and compares per-file SHA-256 records. Both clean builds emitted the
          same 31 files for 15 routes with aggregate digest
          `c0ec818f318d7743cb4045501c1336dbc60b296e4454332404cb030c6b4bbd95`.
          Site lint passes, and the final aggregate verification restored all 14
          tasks—including ownership and determinism—from local Nx cache.
- [x] Extend the dedicated module-map deployment contract to schema v2 so the
      production CEM/custom-element runtime can publish every explicit web
      dependency without an asset-copy exception.
    - [x] Add a versioned schema package whose paired `resources` entries declare
          exact logical resource names, source files, app-relative destinations,
          and content types while preserving v1 `imports` behavior.
    - [x] Add Rust-native fixtures proving CSS, relative JavaScript sidecars,
          workers, and WASM lower as opaque graph artifacts without JavaScript
          parsing, transitive discovery, or undeclared-resource copying.
    - [x] Add a byte-capable transform-graph artifact boundary and atomic CLI
          publication path that preserve binary resource bytes and identity.
    - [x] Extend deterministic resolved-read manifests and cache keys across
          JavaScript and resource entries, with stable validation for mismatched
          keys, unsupported types, escaping destinations, and collisions.
    - [x] Verify the v1 compatibility lane, v2 schema package, native and CLI
          suites, WASM/type projections, and Nx cache replay before activating
          the interactive site fixture.
        - Completed 2026-08-20: module-map v2 adds an exact, paired `resources`
          contract for JavaScript, module workers, CSS, and WASM while keeping
          v1 `imports` behavior unchanged. Resource reads lower as declared
          opaque graph artifacts, retain their content identity, and publish
          exact text or binary bytes through the CLI's atomic multi-output
          boundary; invalid UTF-8 WASM bytes are covered explicitly. Resolved-read
          manifests and cache keys include every declared import and resource,
          and validation rejects schema/key mismatches, unsupported media types,
          unsafe destinations, and import/resource collisions. The CEM-ML build,
          its 1,968-test aggregate, and the added lifecycle regression pass; the
          full CLI target passes 521 unit tests and all integration suites,
          including both schema-owned module-map versions. A repeat of the
          aggregate restored all 37 Nx tasks from local cache, and lint plus
          scoped formatting pass.
- [x] Add interactive token, component, CEM fixture, and native-output examples
      using the production CEM/custom-element implementation.
    - [x] Add a site-owned interactive example fixture and native CEMT route that
          reference canonical token and component identities without copying their
          package-owned declarations.
    - [x] Declare the site bootstrap, production custom-element runtime,
          component sidecars, theme/component CSS, processing worker, and CEM-QL
          WASM exclusively through a paired module-map v2 source/destination map.
    - [x] Exercise token filtering, production component light-DOM rendering, an
          inline CEM declaration, and a live native-output projection in the
          browser without a Vite or asset-copy path.
    - [x] Verify the exact deployed resource allowlist, module-map rewrite,
          undeclared-resource exclusion, browser interaction contract, clean
          deterministic output, and Nx cache replay.
        - Completed 2026-08-20: `/examples/interactive/` now fuses a site-owned
          JSON fixture with native CEMT and references canonical theme-token and
          component identities. A paired module-map v2 graph owns three browser
          imports and all 34 explicit JavaScript, CSS, worker, and WASM resources;
          every published byte is compared with its declared source, and static
          verification rejects undeclared imports, module URLs, source-only paths,
          and destination drift. Chromium proves token filtering, two native
          `cem-action` clicks, `cem-field` light DOM, inline `custom-element` CEM
          compilation, native output, the exact import map, and both stylesheets
          with no runtime errors. Import-map rewriting now retains the upstream
          CEMT source-map stack and rebases unchanged output spans; the added Rust
          regression passes within all 1,970 library tests, and the regenerated
          module-map README passes all 13 schema-package structure tests. The two
          clean site publications contain the same 70 files for 16 routes with
          aggregate digest
          `61e57b1e30a2d0c15fd81cc33d0d93490da1f32919af62709ab745243ea33bcf`.
          Aggregate verification serializes determinism before Chromium to avoid
          output replacement races, lint passes, and the repeat restored all 19
          Nx tasks from local cache.
- [x] Add search, stable deep links, and root navigation.
    - [x] Generate a deterministic search index from the route manifest through
          the native CEM-ML transformation graph.
    - [x] Give rendered headings stable fragment identifiers and verify every
          declared fragment against the published HTML.
    - [x] Publish a dedicated search route/runtime through a paired module map,
          link it from the shared navigation, and cover query/deep-link behavior.
    - [x] Compose the search controls and interaction from production
          `@epa-wg/cem-components` on the `cem-elements`-backed custom-element
          runtime; keep site code limited to search orchestration.
    - [x] Verify clean deterministic output and Nx cache replay for the completed
          search/deep-link/navigation slice.
        - Completed 2026-08-20: the route manifest now owns 16 searchable
          documents and selected heading metadata, and native CEMT renders the
          semantic `/search/` index without a JSON sidecar or post-build copy.
          The search field and action are production `cem-field` and `cem-action`
          controls installed through the shared `cem-elements`-backed
          custom-element runtime; site JavaScript only filters/ranks the rendered
          index and maintains the `q` URL state. The paired module map now owns 39
          exact JavaScript, CSS, worker, and WASM declarations for both interactive
          routes, and all primary navigation exposes Search and Interactive
          examples. Native CEMT gives all 145 headings across 17 routes unique
          deterministic IDs, while static verification rejects missing/duplicate
          fragments and proves every indexed heading's level and text. Chromium
          proves initial and live search, exact-heading ranking, the stable
          `Graph Semantics` deep link, CEM light-DOM controls, stylesheet/import-map
          scope, and zero runtime errors. Two clean builds contain the same 113
          files with aggregate digest
          `c38e634aa647055b2381a367de79c46db78c83c5333e0b18cc16543a922d4fa9`;
          the repeat aggregate restored all 20 Nx tasks from local cache, and
          lint plus scoped Prettier checks pass. The CEM Site/CEM Studio shared
          component-composition principle is now durable in `CLAUDE.md` and the
          roadmap.
- [x] Add a static Angular Material coverage comparison generated from the
      pinned parity evidence, without an Angular runtime dependency.
    - [x] Make the exact pinned parity inventory a validated, cached site-build
          input and retain every catalog row.
    - [x] Publish a semantic comparison route with benchmark provenance,
          covered/partial totals, CEM owners, states, accessibility, keyboard,
          evidence, and bounded notes.
    - [x] Add the route to primary navigation and the generated search index.
    - [x] Verify source/render parity, stable fragments, zero Angular runtime
          assets, deterministic output, and production-site checks through Nx.
        - Completed 2026-08-20: native CEMT now publishes
          `/components/angular-material/` directly from the package-owned parity
          inventory pinned to Angular Material `v22.1.1` at commit
          `0b67c3c38141049657b1167479accc80e455d2bd`. The route retains all 37
          catalog rows—17 covered and 20 partial—and renders every CEM owner,
          state, keyboard contract, accessibility contract, evidence locator,
          and scope note with stable row fragments. The site build hashes the
          inventory, parity documentation, and primitive inventory and schedules
          cached `@epa-wg/cem-components:verify-material-parity` validation.
          All primary navigation and the static search projection expose the
          route; native `seq:count` now renders the exact 17-document search and
          37-row coverage totals. Static verification rejects source/render
          drift, missing package owners, scripts, `node_modules`, and Angular
          runtime imports. The full 21-task `cem-site:verify` graph passed links,
          186 stable headings across 18 routes, Chromium search and interactive
          checks, and two clean 115-file builds with aggregate SHA-256
          `04d025e2544701282f3183165493f817f2a3630aca7980c27d5f1293a0753314`;
          the repeat graph restored all 21 tasks from local cache. Site lint and
          scoped Prettier checks pass.
- [x] Close Phase 6 with complete public-surface documentation and cached
      production verification.
    - [x] Derive the public-surface inventory from every non-private workspace
          `package.json` and every Cargo crate with `publish = true`, retaining
          each deployment or crate identity even when product families overlap.
    - [x] Add canonical package documentation and root-wired site routes for
          every uncovered public npm package and Cargo crate without copying
          package content into the site application.
    - [x] Add a cached full-route Chromium gate covering responses, runtime and
          console failures, landmarks, titles, heading order, names, fragment
          targets, focus visibility, and clean production output.
    - [x] Make the public-surface inventory, static source/render verification,
          deterministic build, browser accessibility crawl, search, and
          interactive checks one cached `cem-site:verify` closure graph.
        - [x] Make `cem_ql:build:wasm` hash its Rust toolchain, lockfile,
              transitive `cem-ml` sources, embedded schemas, and build wrapper;
              make the `cem-elements` and `custom-element` packaging targets
              hash dependency outputs; prove uncached and cached site
              publication copy identical WASM bytes.
    - [x] Prove uncached execution plus all-cache replay, update root/site
          documentation, and mark the Phase 6 exit criterion complete.
    - Completed 2026-08-20: publication metadata now drives an independently
      verified inventory of all 11 public surfaces—seven non-private npm
      packages and four Cargo crates with explicit `publish = true`. Native CEMT
      publishes the static `/packages/` index plus owner-matched canonical routes
      for `cem-elements`, `custom-element`, Trang Native, and all four Rust
      crates; the new crate documentation remains package-owned. The strict
      Markdown edge remains raw-HTML-free. The cached production gate served and
      crawled all 26 routes in Chromium, checked 253 stable headings, 868 named
      keyboard stops with visible focus, 78 internal fragment links, landmarks,
      responses, ARIA references, images, and runtime/console failures, and
      reported zero errors. Search covers all 25 non-search pages, and the
      interactive component/runtime checks remain green. The closure exposed and
      repaired a stale transitive WASM cache boundary: `cem_ql:build:wasm` now
      hashes its toolchain, lockfile, `cem-ml` sources, schemas, and wrapper,
      while both packaging consumers hash dependency outputs. Fresh and cached
      publication now retain the same 34,568,929-byte WASM with SHA-256
      `0fcc18011de323e1b145e0824e4e3c1bf9221c1c2671df2189f5bc216ed934ac`.
      Two clean builds contain the same 131 files with aggregate SHA-256
      `96f6dd801da1891c29ef569140d8f1b099da90dedb8c99c0127080265ba43b7b`;
      the final replay restored all 22 Nx tasks from local cache. Site lint,
      resolved Nx configuration, JSON parsing, diff checks, and scoped Prettier
      checks pass. The Phase 6 roadmap exit criterion is complete.

## Phase 6.5 Checklist — CEM Studio PWA And Browser Workbench

This checklist executes the accepted Phase 6.5 roadmap and
[`cem-studio.md`](./cem-studio.md) contract. CEM-ML remains the production
transformation-graph authority, while all visible Site and Studio functionality,
including search, is composed from `@epa-wg/cem-components` and
`@epa-wg/cem-elements` rather than app-local substitute controls.

- [x] Audit the resolved Studio-adjacent Nx projects, browser command surface,
      component inventory, persistence/PWA foundation, and production web build
      boundary before scaffolding.
    - Completed 2026-08-20: the
      [`Phase 6.5 boundary audit`](./cem-studio-phase6.5-boundary-audit.md)
      resolved 44 Nx projects and no existing Studio app; confirmed the public
      browser command service and checked native/Node/browser parity for nine
      portable operation kinds plus cancellation; mapped the 48-component,
      37-row Material-parity foundation; and found no existing IndexedDB,
      File System Access, service-worker, or install-manifest owner. It accepts
      `packages/cem-studio`, one exact CLI/runtime chain, Nx orchestration, and
      CEM-ML graph/module-map production assembly. Its schema/content decision
      point is closed by the following completed contract slice.
- [x] Accept the Studio portable project v1 schema/content identity and prove
      its CEM and normalized JSON projections before application persistence.
    - [x] Add valid, edge, and deliberate-rejection project fixtures first,
          covering stable ids, hierarchy, resource identities, relative paths,
          run-config references, revisions/hashes, and forbidden provider/UI
          state.
    - [x] Decide and register the canonical CEM content type, JSON projection,
          versioned namespace/schema identities, schema-package ownership, and
          `project.cem` directory/bundle rules.
    - [x] Prove CEM/JSON semantic round trips, deterministic normalization,
          forward-version rejection, logical `studio://` URI derivation, and
          validation before import writes.
    - Completed 2026-08-20: `studio-project/v1` now owns the canonical
      `application/vnd.cem.studio-project+cem` projection, normalized
      `application/vnd.cem.studio-project+json` projection,
      `https://cem.dev/ns/studio/project/1` schema identity, and
      `https://cem.dev/schema/studio/project.schema.json` artifact. Six
      manifest-indexed fixtures and ten native contract tests cover projection
      equality, deterministic round trips, exact namespace selection, normalized
      JSON shape, stable hierarchy/resource references, contained paths,
      `studio://` derivation, forbidden host/UI state, duplicate ids, and
      forward-version rejection. The package is embedded in the built-in
      registry, participates in all three CLI schema-package gates, and passes
      its cached Nx verify, the 13-test schema-package structure target, and the
      CEM-ML WASM build. The broader 1,970-test CEM-ML gate passed 1,969 tests;
      its unrelated debugger deadline timing test passed immediately when
      rerun alone through the same Nx target.
- [x] Create the publishable `@epa-wg/cem-studio` Nx application/package under
      `packages/cem-studio` with exact `@epa-wg/cem-ml-cli` versioning and one
      transitive `@epa-wg/cem-ml` runtime.
    - [x] Make the CEM-ML transformation graph and source/destination module
          maps own final app, component, worker, WASM, manifest, service-worker,
          style, and cache-inventory emission with no post-graph copy exception.
    - [x] Add cached build, lint, typecheck, package, resolved-dependency,
          deterministic-output, and clean-consumer verification targets.
    - Completed 2026-08-21: `@epa-wg/cem-studio@0.1.0` is a fixed-version
      CEM-ML platform member with an exact `@epa-wg/cem-ml-cli` dependency and
      one logical transitive path to `@epa-wg/cem-ml`. Its single authoritative
      module-map rewrite emits 53 declared app/component/worker/style/WASM
      assets, including the service worker, plus seven direct graph exports:
      HTML, manifest, icon, build metadata, cache inventory, and two dependency
      metadata resources. Generic
      `import @opaque=true` config support now publishes explicitly typed raw
      resources without parsing or schema claims and rejects non-direct or
      identity-changing graph use. The cached Studio `check` covers lint,
      typecheck, dependency resolution, exact-byte graph output, two-clean-build
      determinism, npm packing, and a temporary clean consumer; the verified
      archive contains all 60 static files and exactly one installed runtime.
      The service worker and cache inventory remain intentionally
      bootstrap-only until the later PWA lifecycle item.
- [x] Classify the initial Studio shell and workbench behavior against the
      pinned Angular Material inventory before composing it.
    - [x] Reuse completed general CEM controls within their proven contracts;
          finish missing general parity in `@epa-wg/cem-components` first.
    - [x] Reserve a future `/studio` export only for reusable explorer,
          editor-frame, diagnostics, preview, trace, or graph behavior with no
          general counterpart.
    - [x] Keep application routing, state, persistence, workers, updates, and
          search orchestration in Studio while rendering every visible control
          through CEM components/elements.
    - Completed 2026-08-21: the executable
      [`Studio UI classification`](./cem-studio-phase6.5-ui-classification.md)
      maps all 23 initial shell/workbench behaviors to the pinned Angular
      Material `22.1.1` inventory and current 49-component CEM surface. Five
      behaviors directly reuse general controls, eleven keep only application
      orchestration in Studio, and seven reserve named reusable `/studio`
      composites because their workbench capability has no Material catalog
      counterpart. The completed general `cem-tabs`/`cem-tab` prerequisite now
      opens both view-switching compositions, so all 23 classifications are
      open with no remaining general parity gate. The cached
      `verify:ui-classification` target verifies the Material/state inventories,
      exact benchmark, component owners, evidence, blockers, deferred partial
      capabilities, documentation coverage, and deterministic reports. Initial
      popup menus, radio groups, sidenav behavior, and snack-bar lifecycle are
      explicitly excluded in favor of proven visible actions, select, semantic
      grid, and persistent alert contracts.
- [x] Complete the general `cem-tabs` parity required by Studio pane navigation
      before composing the shell or Results view switchers.
    - [x] Accept the authored tab/panel vocabulary, stable relationship model,
          automatic/manual activation policy, selection event, and disabled and
          dynamic-child behavior in a component contract.
    - [x] Add failing browser tests for horizontal/vertical roving focus,
          Home/End and arrow keys, activation, focus-safe panel changes,
          programmatic selection, mutation, and accessibility semantics.
    - [x] Implement the general light-DOM component behavior, state/token and
          forced-color treatment, docs, package output, and clean-consumer
          evidence without Studio-specific state.
    - [x] Promote the pinned tabs row only after the focused tests, state matrix,
          Material parity, package verification, and aggregate component gate
          pass.
    - Completed 2026-08-21: `cem-tabs` now consumes strict inert `cem-tab`
      payloads and owns stable reciprocal tab/tabpanel IDs, horizontal and
      vertical roving focus, manual Enter/Space activation, native-disabled
      skipping, silent `selectedIndex` control, one serializable `cem-tab`
      event, persistent panel state, and focus-safe removal/disable recovery.
      Token-only normal and forced-color styling uses the shared navigation
      families, with no animation, numeric stacking, or Studio-owned state.
      The focused five-test tabs suite, 22-test shared state suite, dedicated
      forced-colors gate, package verifier, 40-row state matrix, 49-primitive
      catalog/Figma projection, and full 130-test browser aggregate pass. The
      pinned Material inventory is now 18 covered / 19 partial, and Studio's
      two view switchers are open general-component compositions. Arbitrary
      in-place payload reordering remains explicitly outside v1 because it
      requires a separate CEM projection-reconciliation contract; append,
      removal, label, and disabled changes are covered. The atomized Vitest
      target default now supplies the `env` object required by local Nx 22.7.0
      without replacing inferred commands or working directories. No live
      Figma asset was changed; `cem-tab` is recorded only as a planned inert
      payload in the repository-owned projection.
- [x] Implement the versioned IndexedDB project repository with migrations,
      atomic autosave, trash/restore, revision/hash conflicts, multi-tab
      coordination, quota diagnostics, and validated import/export.
    - Completed 2026-08-21: `@epa-wg/cem-elements` now owns clone-safe
      repository protocol v1 and its logical registry, while
      `@epa-wg/cem-studio` registers the private `studio-projects` IndexedDB
      implementation without exposing database, store, index, or transaction
      vocabulary to CEM-ML. Database v1 creates all 12 accepted stores and
      indexes; strict multi-store import, resource save, trash, and restore
      transactions enforce expected revisions, content-address source bytes by
      SHA-256, advance the durable change journal, and rebuild deterministic
      search documents. Import and export both require the injected CEM-ML
      Studio-project validator and recheck all declared resource hashes before
      returning or committing a bundle. `BroadcastChannel` is only a wake-up
      hint over the durable cursor, storage status normalizes quota/persistence
      and migration diagnostics, and seven real Chromium tests cover schema,
      atomicity, validation, conflict, search, trash/restore, multi-instance
      coordination, quota, and version failures. The aggregate Studio check
      passes lint, typecheck, browser tests, deterministic graph assembly (55
      declared assets / 62 files), package verification, and a clean consumer;
      the generic repository registry is also covered by the full 138-test
      `cem-elements` unit suite.
- [x] Project logical repository reads and storage health into CEM data slices
      through transient `repository-query` and `storage-status` resources, with
      abort/stale-result handling, durable-cursor subscription cleanup, and a
      proof that rendering can never execute repository mutations or request
      persistent storage.
    - [x] Accept and document the clone-safe resource envelopes, declaration
          attributes, read-only registry injection, and canonical CEM-ML
          processing-host lowering.
    - [x] Add focused processing-engine and real-browser fixtures for query and
          status lifecycles, superseded-request abort, stale-result rejection,
          live cursor refresh, disconnect cleanup, and the no-mutation boundary.
    - [x] Integrate the resources into the package build and aggregate browser
          verification without adding app-owned visible UI.
    - Completed 2026-08-21: canonical processing-host plans and the direct DOM
      fallback now lower both declarations into clone-safe
      `scheduled`/`loaded`/`failed` data-slice envelopes. The runtime consumes
      the frozen `CemRepositoryRegistry.readOnly()` capability, aborts
      superseded queries, rejects late revisions, refreshes query and status
      slices from durable cursor hints, and releases every query/subscription on
      replacement, disappearance, or disconnect. Focused engine/registry tests
      and a real Chromium lifecycle fixture prove JSON-parameter projection,
      storage-health reads, stale-result protection, and zero calls to
      `execute`; the package and Studio module maps include the reader runtime
      without introducing application-owned visible UI. The full
      `cem-elements:verify` integration gate passes all 59 Nx tasks, and the
      Studio aggregate passes all 37 tasks with deterministic 55-asset/62-file
      graph output, package verification, and a clean consumer.
- [x] Build the installable PWA shell with semantic theme modes, a dedicated
      command worker, versioned app/runtime/sample caches, explicit update
      coordination, offline navigation, and recovery without project loss.
    - [x] Audit the graph-emitted static module chain before caching it and
          record the worker-safe deployment decision gate.
        - Completed 2026-08-21: module-map v2 byte-preserves declared JavaScript
          and rewrites only the page import map. The emitted CLI command worker
          still imports bare `@epa-wg/cem-ml/wasm`, which a module worker cannot
          resolve from the page map, while the browser client also imports
          `@epa-wg/cem-ml/runtime.json` outside the current JavaScript-only
          `imports` vocabulary. The Studio design records two valid resolutions
          and recommends a versioned, syntax-aware CEM-ML module-map extension
          over package-specific deployment-loader behavior. PWA implementation
          stops at this decision instead of caching a worker that the static
          output cannot start.
    - [x] Accept and implement the worker-safe module deployment contract, then
          prove the real graph-emitted CLI worker and bundled WASM command both
          online and offline without a production bundler.
        - [x] Add schema-owned module-map v3 source/destination examples and
              native valid/rejection fixtures first, covering JavaScript and
              JSON module entries, exact static/export/dynamic specifier
              rewrites, comments/string false positives, undeclared bare
              specifiers, mismatched rewrite edges, and unsafe destinations.
        - [x] Implement the v3 parser/lowering, exact declared-edge rewrite with
              byte preservation outside specifier spans, deterministic
              source/output digest evidence, schema
              registration, reports, and v1/v2 compatibility lane.
        - [x] Adopt v3 in Studio and add a static-output browser fixture that
              executes the real CLI worker and bundled WASM online and offline.
        - Completed 2026-08-21: schema-owned module-map v3 adds typed
          JavaScript/JSON imports and exact `moduleImports` edges, with native
          acceptance/rejection fixtures, v1/v2 compatibility, and deterministic
          source/output digest evidence in engine and CLI reports. Studio's
          paired v3 maps now own 57 assets, including the CLI worker, runtime
          JSON, WASM wrapper/binary, and complete component chain; CEM-ML emits
          64 static files and the destination map used by versioned
          shell/runtime caches. Real Chromium executes the CLI `version` command
          through the dedicated graph-emitted worker online, reloads with the
          network disabled, and executes it again from Cache Storage. No
          production bundler or package-specific deployment loader is involved.
    - [x] Compose the installable CEM-component shell, five semantic theme
          modes, evolve the accepted shell/runtime caches with a sample-cache
          policy, add explicit safe-update
          barrier, offline navigation, and IndexedDB project-survival test.
        - [x] Fixture: install the production CEM component primitives, render
              the shell with CEM controls only, and persist each of the five
              theme modes named by the repository theme Markdown.
        - [x] Fixture: expose browser-provided install readiness and block a
              waiting service-worker activation during active work or until a
              dirty project has been persisted.
        - [x] Fixture: precache separately versioned shell, runtime, and sample
              groups, then navigate to an application route with the network
              disabled.
        - [x] Fixture: import a project into IndexedDB, reload the offline app,
              and export the same project bytes from the surviving database.
        - Completed 2026-08-21: Studio now installs the production
          `cem-components` declarations through `cem-elements` and composes its
          visible controls exclusively from that set. The five semantic modes
          are the exact classes named by the theme Markdown and persist locally.
          Browser install readiness remains browser-owned. A user-visible
          update action releases a waiting worker only after active work is
          idle and dirty state persists successfully; persistence failure keeps
          the prior worker active. Cache inventory v2 owns separate versioned
          shell, runtime, and sample groups, with an empty graph-emitted sample
          catalog reserved for the next Feature Tour item. Real Chromium proves
          the 58-asset/66-file deterministic deployment, scope-safe deep-route
          fallback, exact IndexedDB project survival, and CLI worker/WASM
          execution both online and offline.
- [x] Generate an editable CEM-ML Feature Tour seed from actual schema-package
      examples and browser capabilities, then verify every advertised example
      and preserve user copies across seed upgrades.
    - [x] Fixture: generate exactly one manifest-declared passing example for
          every registered schema package when the browser capability manifest
          advertises `validate`, with deterministic identities and source
          hashes and no hand-copied example content.
    - [x] Fixture: validate the generated Studio-project manifest and every
          advertised source through native CEM-ML, then copy the original
          package-example bytes through a generated CEM-ML transformation
          graph into the versioned sample cache.
    - [x] Fixture: load and hash-check the graph-emitted seed in Chromium, use
          the real browser worker to validate every advertised example, and
          reject any catalog/runtime capability drift.
    - [x] Fixture: install an editable IndexedDB copy with an identity separate
          from the read-only seed, preserve an edited copy byte-for-byte across
          a simulated seed upgrade, and create a separately identified reset
          copy from the upgraded seed.
    - Completed 2026-08-21: the deterministic generator selects the first
      manifest-declared passing example from all 31 registered schema packages,
      records exact source hashes and browser capability identity, and discovers
      referenced local schema resources transitively. Its generated CEM-ML
      graph emits the 63-resource Studio project, run configurations, original
      example/dependency bytes, catalog, and 66-URL offline sample inventory.
      Native CEM-ML validates the project and all advertised sources. Real
      Chromium integrity-checks the seed, validates all 31 examples through one
      reusable browser command worker using the `cem-studio://` inline-resource
      resolver, and proves the cache online/offline. IndexedDB keeps the
      read-only seed identity separate from `feature-tour`, preserves edited or
      trashed copies across upgrades, and creates `feature-tour-2` on reset.
- [x] Deliver the first offline vertical slice: edit one Feature Tour CEM
      resource, persist and reload its exact revision, validate it through the
      browser worker, and navigate structured diagnostics, report data, and
      source-map provenance with CEM controls.
    - [x] Fixture: save edited CEM bytes with the expected project/resource
          revisions, reload the committed bundle, and prove exact bytes, hash,
          and monotonically advanced revisions before validation.
    - [x] Fixture: validate the committed revision through the real browser
          worker and retain its native structured diagnostics, report summary,
          execution identity, and source-map frames without a JSON reshaping
          boundary in CEM-ML.
    - [x] Fixture: compose editing, save/reload actions, validation status,
          result tabs, diagnostic navigation, report rows, and provenance
          navigation exclusively from production `cem-components` controls on
          the `cem-elements` runtime.
    - [x] Fixture: reload the edited project while offline, revalidate the
          exact persisted revision, and mark an in-flight result stale when the
          draft or durable revision advances before that result is presented.
    - Completed 2026-08-21: the first workbench opens the generated CEM-ML
      example in `cem-textarea`, saves through the IndexedDB optimistic-revision
      command, reloads and verifies the exact committed bytes/hash/revisions,
      and passes those durable project/resource revisions into the browser
      command ledger. Native validation diagnostics, report summary, execution
      identity, and origin-first source-map frames remain structured in the
      workbench. `cem-badge`, `cem-alert`, `cem-action`, `cem-tabs`, selectable
      `cem-list`, and `cem-table` own every visible interaction and status.
      Chromium proves invalid diagnostic/report/provenance navigation, exact
      offline reload and revalidation through the WASM worker, while a browser
      concurrency fixture proves results become stale when a newer draft lands
      during validation.
- [x] Add parse and inspect projections plus the lossless bidirectional CLI
      Command view for copy, edit, transactional Apply, and current/existing/new
      page targets.
    - [x] Fixture: execute CEM-ML `parse` (`ast` and `events`) and every
          browser-capable `inspect` view against exact IndexedDB bytes with the
          durable project/resource revisions carried into the command ledger.
    - [x] Fixture: retain the native command result, execution identity,
          diagnostics, source maps, and target-native CEM-ML output artifact;
          render the CEM-ML bytes without routing them through a JSON AST/DOM
          handoff.
    - [x] Fixture: reproduce browser command-service CEM events presentation
          with the native engine, preserve legal tabs and line endings in CEM
          quoted attributes, and continue rejecting non-text control bytes.
    - [x] Fixture: select and run parse/inspect projections, show status and
          read-only output, and switch result panes exclusively with production
          `cem-components` controls on the shared `cem-elements` runtime.
    - [x] Fixture: prove real-worker parse/inspect output online and after an
          offline reload, and mark a projection stale when the draft or durable
          revision advances while it is running.
    - Completed 2026-08-21: the Feature Tour workbench executes target-native
      CEM-ML `parse` AST/events projections and all six browser `inspect` views
      through the Rust-owned command grammar and real dedicated worker. Each
      request reads the exact IndexedDB resource and dependency bytes, carries
      optimistic project/resource revisions into the command ledger, retains
      the native result/identity/diagnostics/source maps, verifies the published
      output bytes by length and SHA-256, and renders those CEM-ML bytes in
      `cem-select`, `cem-action`, `cem-alert`, `cem-tabs`, `cem-textarea`, and
      `cem-table` controls. Chromium proves the projection matrix, stale draft
      handling, and identical online/offline output. The typed CEM writer now
      preserves tabs and line endings in quoted event payloads while rejecting
      non-text controls. That projection slice left the CLI Command view and
      transactional Apply target workflow to subsequent fixtures.
    - [x] Fixture: add one shared literal command-text codec beside the
          generated CLI grammar, preserve quoted/empty arguments without shell
          evaluation, and prove normalized Node/browser round trips plus stable
          lexical diagnostics.
    - [x] Fixture: generate the editable Studio command from the active
          Rust-lowered invocation and exact IndexedDB resource revision; retain
          its canonical argv, normalized parse, CLI version, and categorized
          input/identity/configuration/output/scope preview.
    - [x] Fixture: edit and reset command text, copy the exact displayed draft
          from an explicit CEM action with accessible success/failure status,
          and render semantic changes or parser/resolver diagnostics without
          mutating project records.
    - [x] Fixture: prove the CLI Command view and change table use production
          `cem-components`, execute the shared preview online and after an
          offline reload, and retain selectable command text as the clipboard
          fallback.
    - [x] Fixture: register a portable authored CLI-command JSON resource that
          stores command-schema/common versions plus literal argv, validates
          through native CEM-ML and the shared Node/browser command grammar,
          and round trips deterministically without persisting a normalized
          run plan.
    - [x] Fixture: resolve current, compatible existing, and explicitly named
          new command-page targets by stable id; atomically commit the authored
          command bytes, entry reference, project/resource revisions, search
          records, and change cursor; reject stale, unresolved, duplicate, or
          incompatible targets without mutation unless incompatible replacement
          is explicitly confirmed.
    - Completed 2026-08-22: the `apply-command-page` repository command parses
      the portable resource with the shared browser CLI grammar, resolves every
      parsed `studio://` URI to an explicitly supplied existing project
      resource, and validates the proposed portable project before opening the
      strict write transaction. Current and name/id-selected compatible pages
      update atomically; explicit new targets receive collision-safe stable ids;
      shared or URL-backed run configs are isolated into a dedicated local
      command resource. One IndexedDB commit advances project/resource/search
      revisions, stores exact command bytes by SHA-256, and appends the change
      cursor. Chromium proves exact reload, shared-resource isolation, new page
      creation, case-normalized name resolution, incompatible confirmation,
      validator rollback, unresolved-resource rejection, and stale revision
      conflicts. That repository fixture left the CEM-component target/confirm
      UI and exact-revision Apply & Run to the following fixture.
    - [x] Fixture: choose current, compatible existing, or named new command-page
          targets through production CEM controls; route incompatible replacement
          through an explicit CEM dialog whose recommended action is creating a
          new page; make Apply reload the exact committed command resource and
          make Apply & Run execute only those returned project/resource revisions,
          rejecting or marking stale every intervening repository change.
    - Completed 2026-08-22: the CLI Command view now defaults incompatible
      current pages to a named new inspection page, exposes current/existing/new
      targets through `cem-select` and `cem-text-field`, and confirmation-gates
      replacement in a transient `cem-dialog` whose primary recommendation is
      creating a new page. Apply serializes the checked draft through the shared
      CLI resource codec, commits it through `apply-command-page`, reloads and
      byte-compares the returned command resource, selects the committed stable
      entry, and never executes. Apply & Run sends only those reloaded bytes to
      the shared browser worker, carries the returned project/command revisions
      into the result, and performs a post-run repository check. Chromium covers
      new and compatible existing Apply, dialog dismissal/new/replacement paths,
      exact single execution, pre-Apply conflicts, and revisions landing during
      execution; the deployed static acceptance runs the same codec and WASM
      worker path against an isolated project copy.
    - Completed 2026-08-22: `cli-command/v1` now owns
      `application/vnd.cem.cli-command+json` and
      `https://cem.dev/ns/cli/command/1`, with a JSON Schema artifact, two
      passing authored commands, and forward-version, grammar-version,
      SemVer, binary, and control-character rejection fixtures. Native CEM-ML
      validates the resource identity and deterministic projection without a
      CEM-tokenizer fallback; the universal Node/browser package validates the
      exact argv with its generated command grammar and regenerates canonical
      resource bytes without a lowered run-plan field. Studio maps parse and
      inspect resources to the existing inspection page kind. That schema
      package fixture left repository target resolution, atomic Apply,
      incompatible replacement confirmation, and Apply & Run to following
      fixtures.
    - Completed 2026-08-21: the workbench now exposes a Studio command as a
      lossless literal-argv projection of the shared generated CLI grammar. Its
      semantic preview comes from the Rust-lowered browser invocation and
      normalized run plan rather than app-owned flag interpretation. Chromium
      proves canonical edit/parse/reset, exact explicit-action clipboard copy,
      semantic change classification, stable invalid-option diagnostics,
      project non-mutation, CEM-only controls, and offline preview. That
      workbench fixture left current/existing/new page targeting,
      incompatible-replacement confirmation, transactional Apply, and Apply &
      Run to subsequent fixtures.
- [x] Add conversion, query, transformation, trace, and transformation-graph
      workbenches without duplicating engine semantics or component behavior.
    - [x] Fixture: generate project-owned conversion, query, direct-transform,
          trace, and transformation-graph Feature Tour scenarios with explicit
          input/query/template/graph identities, pinned expected summaries, and
          deterministic cache resources.
    - [x] Fixture: execute every scenario through the shared authored browser
          command path against exact repository revisions; preserve native
          results, output artifacts, diagnostics, source maps, stage traces,
          graph overlays, stale-result checks, and command-page Apply semantics.
    - [x] Fixture: compose operation selection, identity/configuration details,
          source/result comparison, expected-result status, trace/graph views,
          and copy/download actions exclusively from production CEM controls.
    - [x] Fixture: prove the complete operation matrix through the real WASM
          worker online and after offline reload, with deterministic generated
          output, package verification, and no app-owned execution semantics.
    - Completed 2026-08-22: Feature Tour seed `1.1.0` now owns five portable
      operation workbenches, 77 project resources, and 80 cached sample URLs
      with exact shared grammar, worker, and repository revisions. Production
      CEM controls expose operation selection, source/result and expected-result
      views, trace/graph records, and exact copy/download artifacts. Twelve
      repository and eleven workbench Chromium tests pass; the 43-task Studio
      check proves the real worker online and offline, deterministic 60-asset,
      147-file output, package verification, and a clean consumer install.
- [x] Add the opt-in File System Access provider with explicit permissions,
      retained provider bindings, external-change detection, conflict-safe
      write-back, and complete IndexedDB/import-export fallback.
    - [x] Fixture: persist file and directory handles only in host-owned
          `providerBindings`, restore them without exporting provider state, and
          journal binding changes through the repository protocol.
    - [x] Fixture: open a project resource or portable project directory through
          explicit picker/permission actions, retain `studio://` identities,
          create declared files, and import exact external bytes into IndexedDB.
    - [x] Fixture: detect external hash/revision changes before pull or
          write-back, reject stale writes without truncating files, and advance
          the retained base snapshot only after every staged write succeeds.
    - [x] Fixture: compose provider availability, permission, reconnect,
          conflict, import, and export fallback states from production CEM
          controls and prove them in Chromium with unsupported/denied recovery.
    - [x] Fixture: verify the provider export, deterministic static deployment,
          package surface, and clean consumer without adding filesystem access
          to a worker or silently prompting outside explicit user actions.
    - Completed 2026-08-22: the public File System Access provider now opens or
      binds resource/file-directory handles only from explicit CEM actions,
      retains structured-clone handles and exact revision/hash bases in
      journaled host-only `providerBindings`, and restores them across IndexedDB
      reopen without leaking provider state into portable exports. Native
      Studio-project lifecycle adapters perform exact CEM/JSON manifest
      projection; directory import preserves `studio://` identities and exact
      external bytes. Resource and directory write-back preflight external
      hashes, reject stale writes before truncation, verify closed bytes, and
      advance the base only after every staged write succeeds. Unsupported and
      denied states keep IndexedDB active and expose a deterministic validated
      archive whose SHA-256 matched online/offline
      (`172f9dc59e2bda140e14038d317b20c30de8b299fd337b3099d838311d9d5374`).
      Three provider, thirteen repository, five shell, two Feature Tour, and
      eleven workbench Chromium tests passed. The 44-target Studio check also
      passed lint/typecheck, 61 declared graph assets and 148 deterministic
      files (`9e4436c6060167321267ea2d783bc24fd914a13b0f6011b3a303f13f8499deca`),
      offline worker/fallback verification, package assembly, dependency/UI
      audits, and a clean consumer install. The native Studio-project schema
      verifier passed ten contract tests plus its package-structure audit.
- [x] Close Phase 6.5 with bounded/sandboxed previews, source/result limits,
      accessibility and forced-color coverage, offline/update/security tests,
      package/install verification, dependency audit, and synchronized release
      evidence.
    - [x] Lock conservative Studio v1 source, result, inline-preview, and
          structured-row limits; enforce them before worker transfer or DOM
          projection and retain explicit truncation/oversize diagnostics.
    - [x] Add an application-owned preview boundary that renders text as text,
          treats unknown/binary results as downloads, and admits active HTML only
          through a scriptless, opaque-origin sandbox with a deny-by-default CSP.
    - [x] Prove keyboard reachability, named live status, non-color state cues,
          reflow, reduced motion, and forced-color behavior in real Chromium.
    - [x] Extend real browser evidence for offline startup, the waiting-worker
          activation barrier, dirty-project persistence, malicious preview and URL
          inputs, restrictive app/preview CSP, resolver bounds, and secret
          redaction.
    - [x] Re-run deterministic packaging, tarball installation in a clean
          consumer, and the exact/transitive dependency audit under the hardened
          application boundary.
    - [x] Emit synchronized local release evidence tying the Studio/common
          version, source revision and tree digest, build identity, deterministic
          static digest, package checksum, dependency report, and clean-consumer
          result together without advancing Phase 9 signing/publication work.
    - Completed 2026-08-22: Studio v1 now rejects primary/dependency sources over
      8 MiB, resource sets over 16 MiB or 129 resources, and exact results over
      16 MiB before submission, retained-artifact allocation, or projection; DOM
      previews retain at most 256 KiB and structured views 100 rows. The public
      preview boundary renders UTF-8 as text, keeps binary/unknown/invalid or
      oversized active output download-only, and admits HTML/XHTML/SVG only into
      an empty opaque-origin iframe sandbox with a deny-all CSP. The graph emits
      hash-based application security headers with no `unsafe-inline` or general
      `unsafe-eval`. Real Chromium proves forced colors, reduced motion, reflow,
      keyboard/live/non-color cues, dirty-update persistence before explicit
      worker activation, IndexedDB survival, all 31 examples, and online/offline
      worker/WASM execution. All 39 Studio browser tests pass; the 46-task Nx
      check also passes lint/typecheck, the exact one-runtime dependency audit,
      151-file deterministic output
      (`09675bf73c4ad68eaf8f62ba6a20075272cb79ea633a0595a887eb6cb186be3b`),
      the 11,130,846-byte clean-consumer package
      (`03ab3ec5086957a3fee013ac83d7e1c12b188a38fc8149af3b4497629f35fc63`),
      and synchronized local SBOM/provenance evidence while leaving protected
      publication/signing to Phase 9. Phase 6.5 is complete.

## Later Non-Figma Phase Gates

Expand each gate into its task-level checklist when it becomes the immediate
goal. These gates deliberately keep the deferred Figma work from becoming active
before the non-Figma roadmap is complete.

- [x] Complete Phase 8 iOS and Android token-package hardening and toolchain validation.
    - [x] Lock the native consumer contract, supported toolchain versions, generated
          package layouts, compatibility-copy policy, and host-specific CI boundary.
    - [x] Emit an installable Swift Package while preserving the standalone
          `CEMTokens.swift` copy workflow.
        - [x] Fixture: type-check the sample SwiftUI application against the
              generated package source with the supported iOS SDK.
    - [x] Emit a self-contained Android library layout with XML resources and
          Kotlin constants while preserving the standalone copy paths.
        - [x] Fixture: compile the sample Compose application as a clean Gradle
              consumer of the generated Android library.
    - [x] Publish and verify native component guidance for every public CEM
          primitive, including name, state, color, typography, and accessibility
          mappings.
        - [x] Fixture: map the shared button-and-card sample semantics and token
              usage across the web, SwiftUI, and Compose examples.
    - [x] Add credential-free package validation plus supported-host Swift/Xcode
          and Gradle/Kotlin/Compose CI compile gates.
    - [x] Prove install/copy consumption, native sample parity, zero fail-hard
          reports, and the full CSS/JSON/Swift/Android token-change smoke; record
          the evidence and close Phase 8.
    - Completed 2026-08-22: the generator now emits a Swift Package plus the
      standalone Swift compatibility copy, and a self-contained Android Gradle
      project plus the XML/Compose compatibility copies. Credential-free
      verification covers all 445 generated tokens across five modes, all 49
      public primitives, 14 component states, 19 token families, and the shared
      14-token web/SwiftUI/Compose fixture with zero fail-hard violations. The
      full token-propagation smoke passed locally through CSS, JSON, Swift, and
      Android mutation/restoration. GitHub Actions run `32612971989` proved the
      clean Swift Package and SwiftUI consumer on macOS ARM64/Xcode 16.4 (job
      `97128816724`) and the Android library plus Compose consumer on Linux AMD64
      with JDK 17, Gradle 9.4.1, AGP 9.2.0, and the versioned Android 17 SDK
      package (job `97128816568`). No live Figma parity claim was made.
- [x] Complete Phase 9 release, governance, compatibility, and credential-free
      distribution-readiness work for the code, docs, Studio, and Linux native
      surface.
    - [x] Lock one product-wide versioning, compatibility, deprecation, and
          release-family policy for token names, component APIs, XML schemas,
          native outputs, the web package family, and the fixed CEM-ML family.
    - [x] Publish the current migration/deprecation report and contribution
          guidance for token specs, components, docs/examples, generated native
          packages, Studio, and release changes.
    - [x] Add a machine-readable Phase 9 release contract plus a credential-free
          verifier for version families, exact dependencies, package exports,
          migration/deprecation ownership, workflow isolation, native gates, and
          explicit wishlist ownership of deferred publication.
        - [x] Fixture: reject version drift, a missing stable export, an
              unowned deprecation, a missing CI/native gate, and missing
              publication-wishlist ownership.
    - [x] Add Nx aggregate readiness and credential-free closure targets, wire
          serialized readiness into CI, and retain the contract preflight in both
          existing protected publish lanes.
    - [x] Prove clean package/export consumption for the fixed web family,
          CEM-ML runtime/CLI, Studio/PWA assets, token/native outputs, docs links,
          component examples, and the Linux lifecycle.
        - [x] Fixture: lock the clean custom-element archive path set after stale
              compiler outputs are removed and reject every extra or missing path.
        - [x] Fixture: require every transitive JavaScript dependency used by the
              search and interactive pages in both site runtime module maps.
    - [x] Move web-family npm publication, final CEM-ML GitHub/npm publication,
          Studio npm/static deployment, and immutable remote evidence to
          `docs/wishlist.md` without creating external state.
    - Completed 2026-08-23: the credential-free Phase 9 readiness aggregate passed
      all 187 serialized Nx tasks (the root target plus 186 dependencies). The
      machine contract verified seven public packages, five compatibility axes,
      and all five deliberate-rejection fixtures. Its underlying evidence includes
      1,982 CEM-ML tests; 522 CLI tests plus 84 schema validations (83 passing and
      one intentional ignore); all 19 atomized component browser/unit files; the
      exact 143-file custom-element archive
      (`46d67f2471ca803a70f132c7807e3286ea6a83e98b278ae4e76487d42b71f43a`);
      and the deterministic 135-file site output
      (`1f6c49eaee4ae179a19ecfbb235201647d996991a6633fcf6d9ff68860269b28`).
      On 2026-08-23, publication was deliberately removed from the active gate and
      assigned to `docs/wishlist.md`. The credential-free closure target now
      requires that wishlist ownership and passed all 188 serialized Nx tasks (the
      root closure plus 187 dependencies, 143 restored from cache) with no blockers.
      No package was published and no tag, signed release, or deployment was
      created. Phase 9 is complete.

## Phase 10 Checklist — Deferred Figma UI Kit

Phase 4 component names, variants, executable states, and accessibility semantics
are complete in the archived checklist, and the Phase 10 repository foundation
already owns the five-mode token gate and 49-primitive executable Figma inventory.
The remaining Phase 10 work is reviewed canvas work in the canonical CEM UI Kit
and must not start before Phase 9 is complete.

- [ ] Build and review the `02 Foundations` page from native CEM variables.
    - [x] Re-run the credential-free repository preflight and confirm the
          live-canvas ownership boundary before editing Figma.
        - Completed 2026-08-19: the resolved
          `@epa-wg/cem-components:verify-figma-inventory` Nx graph passed all
          eight tasks, with four restored from cache. It regenerated and
          validated 252 consistent variables across Light, Dark, Contrast Light,
          Contrast Dark, and Native, proved the representative token-propagation
          smoke, and verified all 48 public primitives (37 component sets, three
          components, four payloads, and four structural owners). The inventory
          remains truthful at zero reviewed and 48 planned canvas entries. The
          last recorded live-library review is still the 2026-04-30 230-variable
          revision, so the canonical file must be refreshed and its current
          revision captured before foundation construction. This environment has
          no approved live Figma reader/editor or credential; anonymous access is
          blocked. No canvas or publication claim was made.
    - [x] Repair the generated native Figma mode projection to use the current
          DTCG value shapes accepted by Figma Import mode before refreshing the
          live collection.
        - Completed 2026-08-19: Stage 4 now emits sRGB color objects, px
          dimension objects, and second-based duration objects while preserving
          aliases and finite number/string scalars. The offline validator rejects
          legacy scalar shapes, and the propagation smoke test compares the
          structured color values. `@epa-wg/cem-theme:test:figma` regenerated and
          validated 252 consistent tokens across all five modes, theme lint
          passed, and `@epa-wg/cem-components:verify-figma-inventory` passed its
          eight-task graph with the 48-entry inventory still at zero reviewed and
          48 planned. Shadow recipes and easing curves remained deliberately
          excluded until the following representation contract was adopted; no
          live Figma update was claimed.
    - [x] Adopt derived composite Effect Styles for canonical layering shadows
          and derived motion specimens for canonical easing curves; keep both
          families outside native Figma variable import.
        - [x] Add a checked-in `02 Foundations` composite-style and motion-review
              inventory plus deliberate-rejection fixture before live canvas
              construction.
        - [x] Add a credential-free Nx verifier that derives composite values
              from canonical tokens, rejects raw values in the inventory, and
              emits review evidence.
        - Completed 2026-08-19: canonical export now emits six real layering
          recipes as DTCG `shadow` composites and all eight easing tokens as
          `cubicBezier` arrays. Base remains the explicit string `none` because a
          valid DTCG shadow array must be non-empty; five semantic layer aliases
          preserve their owning rung's type. The checked-in 15-entry Foundations
          inventory defines six Effect Styles, one no-effect specimen, five alias
          annotations, and eight motion specimens without raw values. The new
          `@epa-wg/cem-theme:verify:figma-foundations` gate passed and emitted
          JSON/Markdown review reports, `build:token-platforms` retained all 445
          tokens across five modes, theme lint passed, and the nine-task
          `@epa-wg/cem-components:verify-figma-inventory` graph passed with eight
          tasks restored from cache. Native import remains at 252 consistent
          tokens per mode, the component inventory remains at zero reviewed and
          48 planned, and the Foundations inventory remains at zero reviewed and
          15 planned. No live canvas or publication claim was made.
    - [x] Record the starting CEM UI Kit revision and confirm that the native
          `CEM Tokens` collection has the accepted Light, Dark, Contrast Light,
          Contrast Dark, and Native modes before editing the canvas.
        - [x] Add a checked-in native-library review evidence record and
              deliberate-rejection fixture for this manual checkpoint.
        - [x] Add a credential-free Nx verifier that keeps the refresh pending
              until a real starting revision and five-mode review are recorded.
        - Completed 2026-08-19: `native-library-review.json` now separates the
          historical 2026-04-30 live review, the current 252-variable generated
          contract, and a truthful pending refresh. Its fixture defines the
          `pending` -> `started` -> `reviewed` promotion procedure and rejection
          cases. The new cached
          `@epa-wg/cem-theme:verify:figma-native-review` target derives the 57
          COLOR, 112 FLOAT, and 83 STRING expectations from all five mode files,
          requires an explicitly recorded starting revision plus the exact
          collection/modes before accepting `started`, and emits JSON/Markdown
          reports. Theme
          lint and the nine-task `test:figma` graph passed; the ten-task
          `@epa-wg/cem-components:verify-figma-inventory` aggregate passed with
          nine tasks restored from cache. The refresh remains `pending`, the
          parent live checkpoint remains open, and no external Figma change was
          claimed.
        - Completed 2026-08-23: an authenticated read-only review opened the
          canonical CEM UI Kit and recorded the `01 Tokens` page as the starting
          locator before any import or canvas edit. The live collection is
          exactly `CEM Tokens`, its modes remain Light, Dark, Contrast Light,
          Contrast Dark, and Native in order, and all 230 variables still carry
          the historical `@epa-wg/cem-theme 0.0.8` description with zero missing
          mode values and 255 alias mode values. The checked-in refresh evidence
          is now truthfully `started`; every import and live-result field remains
          null. The repository gate still expects 252 variables (57 COLOR, 112
          FLOAT, and 83 STRING), so the refresh and the parent Foundations
          checkpoint remain open. No external Figma state was changed.
    - [ ] Build color, typography, spacing, shape, stroke, layering, and motion
          guidance with variable bindings or approved composite text styles and
          no raw replacement values.
    - [ ] Review every foundation section in all five modes and record the Figma
          revision, evidence locations, and raw-value findings.
- [ ] Build and review the representative `03 Components` pilot for
      `cem-action`, `cem-text-field`, `cem-card`, `cem-nav`, and `cem-dialog`.
    - [ ] Keep variant dimensions independent, use component properties by
          semantic meaning, and test every owned state in all five modes.
    - [ ] Record the pilot fixture and review evidence before expanding to the
          remaining component inventory.
- [ ] Complete `03 Components` for every executable inventory entry, keeping
      inert payloads nested under their consuming visual owners.
- [ ] Populate `99 QA`, run the offline token/component gates, record the
      reviewed Figma revision and five-mode evidence, and publish the Phase 10
      library only after raw-value, detached-shape, state, and documentation
      checks pass.

## Phase 11 Checklist — Deferred Figma Site Demo

Phase 11 starts only after the Phase 10 UI Kit is reviewed and published.

- [ ] Build `04 Patterns` for auth, profile, assets, discussion, and settings
      entirely from library instances, then compose `05 Site Demo` from those
      patterns without detached one-off controls.
- [ ] Add matching CEM XML/HTML fixtures and a web implementation built from CEM
      components, with native iOS/Android token-usage notes.
- [ ] Record scenario tests, screenshots, and reviewed Figma evidence proving
      consistent tokens and component semantics across design and implementation.

## Theme generator CEM-ML simplification

- [x] Add portable Markdown-table row records with normalized named fields,
      ordered cells, source-table/source-row provenance, and compatible `tdN`
      aliases.
    - [x] Fixture: prove normalization, provenance, compatibility aliases, and
          diagnostics for duplicate or empty normalized headings.
- [x] Add a resolver-preflighted CEMT module-closure render boundary shared by
      browser/WASM and native CEM-ML execution.
    - [x] Fixture: render the same imported public template through browser/WASM
          and native execution and compare the render plan and diagnostics.
- [x] Introduce and adopt the shared theme token-documentation CEMT module,
      migrate generator expressions to named row fields, and retain the complete
      Markdown-driven presentation and generated CSS protocols.
    - [x] Fixture: re-run all ten source-completeness protocols and record the
          before/after source complexity measurements.
    - Completed 2026-09-12: the Markdown schema now owns normalized named row
      fields, ordered cells, source provenance, positional compatibility aliases,
      and collision diagnostics. Browser runtime support preflights resolver-backed
      static CEMT closures, records resolver-policy identity, and lets the Rust
      compiler validate hashes, root-body/parameter/runtime compatibility, nested
      imports, and public entrypoint visibility. The shared `cem-token-docs.cemt`
      supplies 18 table and 43 CSS-property call sites across the ten generators. Source size
      fell from 1,389 to 1,302 lines, loops from 137 to 76, and positional accesses
      from 497 to zero while the ten CSS outputs remained 58,527 bytes. Markdown
      package verification, 149 Storybook tests, the complete `cem-ql` suite, both
      affected lints, all ten manifest/source-completeness/browser-capture checks,
      and 479/479 token coverage passed.
