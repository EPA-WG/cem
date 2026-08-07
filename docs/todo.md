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
    - [x] Extend the native evaluator over the retained lifecycle XML owner with
          attribute, parent, descendant, and descendant-or-self axes; path-result
          identity deduplication and document ordering; predicate item/position/
          size focus; numeric and effective-boolean-value filtering; and exact
          untyped/string general equality, without a serializer, DTO, source
          reparse, or evaluator-owned replacement tree.
    - [x] Complete retained-owner structural navigation with ancestor,
          ancestor-or-self, following, following-sibling, preceding, and
          preceding-sibling axes; reverse-axis predicate focus; postfix filter
          predicates; and native `position()`/`last()` focus functions, while
          leaving the optional namespace axis and atomic coercions explicit.
    - [x] Establish the exact native atomic comparison kernel: represent
          integers and decimals with unbounded normalized coefficient/scale
          values; atomize retained XML nodes without projection; implement
          XPath 3.1 general and value comparisons across supported string,
          URI, boolean, untyped, decimal, float, and double values; and preserve
          cardinality, coercion, promotion, NaN, and source-map semantics
          without a serializer or intermediate DTO.
    - [x] Execute XPath `and` and `or` directly on the owned expression AST:
          apply the existing effective-boolean-value kernel to each required
          operand, short-circuit deterministically from left to right, retain
          expression source maps on typed boolean results and operand errors,
          and never evaluate a skipped branch or cross a serialized boundary.
    - [x] Execute XPath node comparisons directly on retained native node
          identity: implement `is` across owners and same-owner `<<`/`>>`,
          enforce optional-singleton node operands with exact operand source
          maps, preserve empty-sequence propagation, and reject cross-owner
          ordering until the host defines a stable multi-document order.
    - [x] Execute XPath `union`/`|`, `intersect`, and `except` directly on
          retained native node sequences: enforce node-only operands, preserve
          identity deduplication and same-owner document order, retain native
          source maps, and reject only results that need undefined cross-owner
          ordering rather than introducing a serializer or synthetic tree.
    - [x] Execute XPath string concatenation (`||`) directly on the owned
          expression AST: atomize each optional-singleton operand, treat empty
          operands as zero-length strings, cast supported native atomic values
          to `xs:string` with XPath 3.1 lexical rules, retain exact result and
          operand source maps, and never introduce a serializer or DTO.
    - [x] Establish the type-preserving native arithmetic core: model unary
          signs in the owned expression AST; execute unary `+`/`-` and binary
          `+`/`-`/`*` over optional-singleton atomized operands; preserve exact
          integer/decimal values, XPath numeric promotion, IEEE float/double
          behavior, and exact source maps; and keep division and range budget
          policy as explicit follow-up work without a serializer or DTO.
    - [x] Establish the typed XPath sequence-item budget and native range
          operator: promote inherited `xpathItems` scope budgets into evaluator
          limits with no hidden default; require an explicit limit for `to`;
          apply function conversion to `xs:integer?`; materialize exact,
          inclusive integer sequences only within the limit; preserve source
          maps and deterministic diagnostics; and thread the limit through
          standalone and host invocation paths without serialization or a DTO.
    - [x] Execute XPath `idiv` and `mod` directly on typed numeric values:
          reuse optional-singleton atomization and numeric promotion; calculate
          unbounded integer/decimal quotients and remainders exactly with
          truncation toward zero; preserve float/double NaN, infinity, and
          signed-zero behavior; report FOAR0001/FOAR0002-class failures with
          exact source maps; and leave decimal `div` precision policy explicit
          without introducing serialization or an intermediate DTO.
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
    - [x] Establish the shared typed host-invocation contract and its first CEMT
          adapter: consume an already owned XPath AST, retain a separate native
          context item, bind expanded-QName keys directly to typed XPath
          sequences, and preserve resolver, safety, owner, and source-map
          identity without parsing expression strings or projecting bindings
          through JSON; leave authored CEMT call syntax explicitly undecided.
    - [x] Add the schema-owned CEMT XPath body form: fuse its typed expression
          child once into a CEMT-owned XPath AST, map explicit context and
          expanded-QName variable declarations only from native XPath host
          bindings, invoke the CEMT adapter directly, and return the typed XPath
          result artifact without a string parser, generic CEMT value bridge,
          serializer, DTO, or replacement tree at runtime.
    - [x] Add the explicit host-selected CEMT XPath function entrypoint: resolve
          one compiled XPath body by exact function name, require the host to
          populate every declared native XDM binding without implicit renderer
          aliases, and return the typed XPath artifact body without entering the
          generic CEMT evaluator or an authored call-syntax lane.
  - [x] Fuse parsed XPath streams into XSLT XPath-bearing attributes and AVT
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
    - [x] Segment schema-classified literal-result AVTs into lossless typed
          literal, expression, empty-expression, and error segments; parse each
          expression once into its directly owned XPath AST; and project escaped
          braces, nested XPath braces, comments, strings, and entity-decoded
          subranges to exact XML coordinates without a serializer or reparse.
    - [x] Cover the complete XSLT 3.0 instruction AVT matrix—59 attributes
          across 15 elements—in schema-owned contextual selectors; prove the
          selector set exactly against a normative contract fixture; and route
          every selected value through the existing directly owned AVT/XPath
          AST path while fixed, expression, pattern, and XSLT control attributes
          remain outside it.
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
