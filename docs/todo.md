# Todo

This file is the authoritative checklist for remaining execution work.
Product/module sequencing lives in [`../roadmap.md`](../roadmap.md), future
wishlist work lives in [`wishlist.md`](wishlist.md), and completed execution
history is preserved in
[`archive/todo-completed.md`](archive/todo-completed.md).

## Immediate Goal

Current active slice: implement the CEM-owned XPath 3.1 compiler and evaluator
over the package AST, following completion of the strict native-AST transform
boundary.

The cross-layer architecture remains serializer-free: lifecycle loading, graph
routing, joins, evaluators, CEM-QL, CEMT, and XSLT adapters must exchange
borrowed native AST streams or typed evaluator values directly. JSON and other
encodings are allowed only at explicit lifecycle parse or registered export
boundaries; no serializer, generic DTO, shape inference, or replacement tree may
mediate between internal layers.

### Immediate: XPath 3.1 Compiler and Evaluator

- [ ] Complete XPath 3.1 execution on the existing CEM-owned typed XPath AST
      and lossless token stream.
  - [ ] Implement a CEM-owned XPath 3.1 compiler and evaluator. Treat W3C XPath
        3.1, XDM 3.1, and Functions and Operators 3.1 as normative; use the
        pinned Xee source only as a non-normative implementation reference;
        target native and WASM; and route documents, collections, unparsed text,
        environment, time, randomness, recursion, cancellation, and work budgets
        through explicit CEM resolver/safety capabilities.
    - [x] Establish the first native evaluator slice: retain the exact lifecycle
          XML AST owner and typed node handle in context/results, execute package
          AST literals, variables, context items, and child/name-test paths
          directly, reject unsupported semantics deterministically, and prove no
          serializer, projection DTO, source reparse, or replacement tree enters
          the evaluator boundary.
  - [ ] Wire the native evaluator through the `transform` command and expose
        explicit CEM-QL, CEMT, and XSLT invocation adapters without reparsing
        source text, constructing an evaluator-owned replacement XML tree, or
        projecting AST or result values through JSON.
    - [x] Register the standalone XPath transform slice: compile XPath source
          once into the package-owned AST, evaluate the primary lifecycle XML
          owner as the document context, retain native node identity in the
          typed XPath result artifact, export JSON only through the registered
          result exporter, and reject parameters, secondary inputs, and
          unsupported input AST families until their binding contracts exist.
  - [ ] Fuse parsed XPath streams into XSLT XPath-bearing attributes and AVT
        expression segments while retaining an independently addressable XPath
        AST associated with the owning XML event or subtree node.
    - [x] Fuse entity-free XPath-bearing attributes on XSLT instruction nodes
          into package-owned XPath ASTs with exact host byte ranges, owning XML
          event identity, and inherited static namespace context; keep AVT
          segmentation and entity-decoded source mapping explicit follow-up
          work rather than introducing a text, JSON, or replacement-tree bridge.
    - [x] Extend the generic `XmlAttributeAst` with its exact lexical value
          range, entity-decoded value, and monotonic decoded-byte-to-source
          spans so XML-family consumers can retain original source identity
          without a serializer or format-specific mapping overlay.
    - [x] Wrap decoded XML attribute spans in a boundary-aware typed source map
          that projects scalar-aligned ranges and zero-length positions back to
          exact original source ranges while invalid boundaries fail closed.
    - [x] Thread a shared typed source-range projector through XPath's single
          scan/parse and fuse entity-decoded whole XPath attributes directly
          into XSLT, preserving original ranges on tokens, events, syntax,
          facts, and diagnostics without a serializer or post-parse rewrite.
    - [x] Replace XSLT's hardcoded attribute-name test with schema-owned typed
          value-grammar rules that distinguish XPath expressions, XSLT
          patterns, AVTs, and literals before AVT segmentation begins.
  - [ ] Add deterministic compact/pretty/tabular and
        terminal/HTML/Markdown profiles that preserve lexical islands and source
        maps, then run package, converter-parity, CLI e2e, WASM, and core release
        gates.

Completed prerequisites include the independent XPath schema package, CEM-owned
lossless scanner and recursive-descent parser, strongly typed AST/event model,
host attachment and evaluation contracts, conformance matrix, and removal of
Xee runtime dependencies. Detailed evidence and provenance remain in the
[archived checklist](archive/todo-completed.md).

### Deferred: Phase 3 Custom-Element Runtime

- [ ] Resume Phase 3 custom-element runtime substrate expansion after the
      schema-package folder contract slice is closed.

### Deferred: Phase 4 CEM Component Set

- [ ] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
- [ ] Populate the first missing state fixture or assertion from that audit,
      prioritizing selected, expanded, empty, and loading coverage across
      navigation, content, and layout workflows.
- [ ] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.

## Recently Completed

- [x] Archive the completed execution narratives and verification evidence in
      [`archive/todo-completed.md`](archive/todo-completed.md), leaving this file
      focused on active and deferred work.

## Current Verification Commands

- `yarn nx run cem_ml_schema_package_xpath_v1:verify`
- `yarn nx run cem_ml:test:schema-package-structure`
- `yarn nx run cem_ml_cli:validate-converter-parity`
- `yarn nx run cem_ml_cli:e2e`
- `yarn nx run cem_ml:lint`
- `yarn nx run cem_ml:test`
- `yarn nx run cem_ml:build:wasm`

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
