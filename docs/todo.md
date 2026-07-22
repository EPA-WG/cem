# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Phase 2 exit criteria from [`../roadmap.md`](../roadmap.md) are closed. The
active Phase 3 slice now shifts from schema-package format alignment to the
Rust-first `cem-ql` syntax decision recorded in
[`cem-ql-ac.md`](cem-ql-ac.md). CSV and sibling format work is deferred below
until the CEM-QL parser/runtime/docs/showcase gates are green.

Current active slice: make `packages/cem_ql` implement Rust-style expression
syntax and semantics as the canonical surface, while keeping XPath/XQuery and
Python strictly as functional parity references. Prove every operator and
function family from the parity list with Rust tests, schema-package examples,
documentation, and a Storybook showcase.

### Immediate Execution Phase: Rust-First CEM-QL Syntax And Parity Showcase

Observed code state:

- [x] `packages/cem_ql/src/lexer.rs` tokenized XPath operator keywords
  (`eq`, `ne`, `lt`, `div`, `mod`, `and`, `or`, `not`) and treated `&&` / `||`
  as reserved errors; it now recognizes Rust-first operators and classifies
  XPath-only words as compatibility tokens.
- [x] `packages/cem_ql/src/parser.rs` still parses XPath/XQuery forms:
  `if ... then ... else ...`, `let name := ... in ...`, `for ... return ...`,
  `some/every ... satisfies ...`, `instance of`, `cast as`, `treat as`, `/`
  paths, and quoted-only record literals. The parser now rejects those
  compatibility forms and accepts Rust-style control flow, bindings,
  records with bare/quoted keys, `/` as numeric division, type postfixes,
  `treat_as(...)`, `same_node(...)`, computed record keys, and
  `fn(...) =>` lambdas for quantified helpers.
- [x] `packages/cem_ql/src/eval.rs` already has evaluator support for boolean
  short-circuiting, numeric operators, set operations, pipelines, conditionals,
  loops, and type checks; the parser now exposes the Rust-first syntax for
  those forms, including stream difference through canonical `-`.
- [x] `-` now works as numeric subtraction and stream difference. Known stream
  operands lower to `SetOp::Difference`; unknown operands use runtime dispatch
  with a mixed numeric/stream type error.
- [x] `packages/cem_ql/tests/xpath_parity.rs` and
  `packages/cem_ql/tests/parser_recovery.rs` now use Rust-first syntax for
  in-subset passing cases; the CEM-QL schema-package validation examples now
  use the same Rust-first module syntax.
- [ ] `packages/cem-elements` has CEM-QL render-loop stories, but no dedicated
  operator/function parity showcase and no direct WASM query-evaluation
  boundary for Storybook tables.

Dependency-ordered implementation checklist:

- [x] Add the missing function inventory section referenced by
      `docs/cem-ql-ac.md` AC-QX-1 (`AC-QF-2` or an explicitly renamed
      equivalent), listing the Tier A and Tier B function families that the
      tests and Storybook showcase must cover.
- [x] Update `docs/cem-ql-stack-design.md` and
      `docs/cem-ql-stack-design-impl.md` so lexer tokens, Pratt precedence,
      parser forms, diagnostic names, and examples follow Rust-first syntax:
      `==`, `!=`, `<`, `<=`, `>`, `>=`, `+`, `-`, `*`, `/`, `%`, `&&`, `||`,
      `!`, `if condition { ... } else { ... }`, `{ let name = value; expr }`,
      `declare let name = expr`, Rust-style records, and dot pipelines.
- [x] Decide and document the `-` overload contract before parser changes:
      either lower `-` after type checking into numeric subtraction vs stream
      difference, or introduce a typed operator node that the evaluator
      dispatches by operand shape. The accepted design must preserve strict
      typed identity and deterministic stream order.
- [x] Rename the diagnostic contract from `cem.ql.use_and_or` /
      `USE_AND_OR` to `cem.ql.use_rust_boolean_ops`; update
      `packages/cem_ql/src/diagnostics.rs`, tests, and docs to report XPath
      boolean spellings as compatibility errors that suggest `&&`, `||`, and
      `!`.
- [x] Update the lexer:
      recognize `==`, `&&`, `||`, `!`, `%`, and single `=` for Rust-style
      binding syntax; stop treating `&&` / `||` as reserved; classify
      XPath-only operator words as compatibility-error tokens or ordinary
      identifiers with targeted parser diagnostics.
- [x] Update Pratt precedence in `packages/cem_ql/src/parser/pratt.rs` to
      Rust-first operator ordering, including `||`, `&&`, comparisons,
      set operators, `+ -`, `* / %`, unary `!` / unary `-`, type/cast
      postfixes, and dot calls/pipelines. Remove `/` as path syntax.
- [x] Update expression parsing in `packages/cem_ql/src/parser.rs`:
      parse Rust-style `if` blocks, expression blocks with semicolon-separated
      `let` bindings, `for name in stream { expr }`, Rust-style records with
      bare keys / quoted keys / computed keys, prefix `!`, and `expr as Type`.
- [x] Replace XPath type syntax in parser/lowering with canonical CEM-QL
      syntax: `expr is Type`, `expr as Type`, `treat_as(expr, Type)`, and
      `same_node(a, b)`. Keep XPath spellings only as diagnostic suggestions,
      not successful parses.
- [x] Convert quantified expression support from `some/every ... satisfies`
      syntax to helper calls: `any(stream, fn)` and `all(stream, fn)`.
      Retain `IrNode::Quantified` only if the helper lowering still benefits
      evaluation; otherwise lower helpers directly.
- [x] Update module-level syntax: `declare let name = expr`,
      `declare function ns:name(param as Type) { ... }`, and imports without
      `$`-prefixed variables. Remove `declare variable` from passing fixtures.
- [ ] Update IR, type checker, and evaluator naming where it leaks old syntax:
      comparison variants should reflect `==` / `!=`, numeric division and
      remainder should use `/` / `%`, and type diagnostics should talk about
      Rust-first forms.
- [ ] Define and test Rust numeric semantics explicitly: integer vs decimal vs
      double division, `%` remainder behavior, division by zero, NaN
      normalization for set identity, signed zero, and no implicit cross-type
      promotion.
- [x] Wire stream difference through the canonical `-` operator and keep
      `seq:difference(a, b)` as a named helper alias. Remove test comments that
      describe `-` as unavailable for stream difference.
- [ ] Replace `packages/cem_ql/tests/xpath_parity.rs` with a Rust-first
      functional parity table. Keep QT3/XPath category names in metadata, but
      every in-subset query must use canonical Rust-first CEM-QL syntax.
- [ ] Update parser, IR lowering, type-checking, set-operator, evaluator,
      compiled-artifact, fixture, policy-hook, and template-render tests so no
      passing test depends on XPath operator, variable, path, or clause syntax.
- [ ] Add negative parser tests for old XPath/Python syntax:
      `eq/ne/lt/le/gt/ge`, `div`, `mod`, `and`, `or`, `not(...)`,
      `if ... then ... else ...`, `let ... := ... in ...`, `for ... return`,
      `some/every ... satisfies`, `a/b`, `instance of`, `cast as`, and
      `treat as`. Each diagnostic should point to the Rust-first replacement.
- [x] Update `packages/cem_ml/schema-packages/cem-ql/v1/README.md`,
      `schema/cem-ql.cem`, `package.cem`, and examples so package-owned
      validation fixtures use Rust-first CEM-QL source.
- [ ] Add CEM-QL schema-package examples for each parity group:
      arithmetic, comparisons, boolean short-circuit, set operators,
      pipeline/current item, records/arrays/streams, blocks/let, if/else,
      for mapping, any/all, type tests/casts, stdlib sequence helpers,
      string helpers, number helpers, date/time helpers, report helpers,
      state/template helpers, read/content-type helpers, and old-syntax
      diagnostics.
- [ ] Update `packages/cem_ml/schema-packages/cem-ql/v1/formatters/` and
      `colorizers/` so formatter/colorizer examples and token roles know the
      Rust-first operators and highlight deprecated XPath/Python forms as
      diagnostics, not canonical syntax.
- [ ] Add a direct WASM query-evaluation export in
      `packages/cem_ql/src/api/wasm.rs` for Storybook demonstrations:
      compile/evaluate query source, accept JSON bindings, return items and
      diagnostics as JSON. Keep template rendering exports unchanged.
- [ ] Add a TypeScript runtime helper in `packages/cem-elements` for the direct
      CEM-QL WASM query-evaluation boundary, parallel to
      `internal/runtime-support/cem-ql-render.ts`.
- [ ] Add `packages/cem-elements/src/lib/cem-ql-rust-first-parity.stories.ts`
      with a table-driven showcase. Each row must show query source, input
      bindings, output items, diagnostics, and source category; the story's
      `play` function must assert every row.
- [ ] Storybook operator rows must cover: `==`, `!=`, `<`, `<=`, `>`, `>=`,
      `+`, `-` numeric subtraction, `*`, `/`, `%`, unary `-`, `&&`, `||`,
      `!`, `??`, stream `|`, stream `&`, stream `-`, stream `^`, `.` pipeline,
      leading `.`, `is`, `as`, `treat_as(...)`, and `same_node(...)`.
- [ ] Storybook function rows must cover the parity function inventory:
      `map`, `where`, `flat_map`, `take`, `drop`, `first`, `last`, `nth`,
      `peek`, `union`, `intersect`, `difference`, `symmetric_difference`,
      `count`, `unique`, `distinct_by`, `flatten`, `zip`, `enumerate`,
      `chunked`, `windowed`, `sliding`, `group_by`, `count_by`, `partition`,
      `take_while`, `drop_while`, `sorted`, `reversed`, `reduce`, `fold`,
      `scan`, `any`, `all`, `none`, `min`, `max`, `sum`, `avg`, plus the
      Tier A string, number, datetime, report, state, template, CEM-ML, and
      content-type helper functions exposed by `ModuleRegistry`.
- [ ] Split unimplemented Tier B helper rows into explicit pending/unsupported
      diagnostics only if their implementation is not part of the current
      slice. The story must still list them so the parity surface remains
      visible.
- [ ] Add or rename Nx targets as needed so the Rust-first parity suite can be
      run independently from legacy XPath parity. Keep the old target only as
      a compatibility harness if it no longer implies syntax parity.
- [ ] Run the Rust-first gate after implementation:
      `yarn nx run cem_ql:test`,
      `yarn nx run cem_ql:test:xpath-parity` or its renamed parity target,
      `yarn nx run cem_ql:test:set-operator-identity`,
      `yarn nx run cem_ql:test:fixtures`,
      `yarn nx run cem_ql:build:wasm`, and
      `yarn nx run cem-elements:verify`.

### Follow-On Execution Phase: Embedded CEM-QL Expression Audit

Run after the Rust-first CEM-QL parser/runtime/docs/showcase slice is green.

- [ ] Add a repository-wide extractor for CEM-QL expressions embedded in every
      checked-in `*.cem` and `*.cemt` file. It must cover host-owned template
      spans (`{...}` attributes, `select=` / `match=` / `test=`, and `{$ ...}`
      content expressions), plus CEMT expression positions in formatter,
      colorizer, converter, and validation assets.
- [ ] Preserve source provenance for every extracted expression: source file,
      byte range, host embedding kind, CEM-QL sub-span, schema-package identity
      when applicable, and whether the expression came from a formatter,
      colorizer, converter, validator, example, or documentation fixture.
- [ ] Compile every extracted expression through the Rust-first CEM-QL parser,
      resolver, and type checker. Old XPath/Python syntax must fail with the
      Rust-first diagnostics defined by the rustification slice, not pass as a
      compatibility form.
- [ ] Add functional validation fixtures for extracted expressions that need
      runtime data. Group fixtures by owning package or story so expressions are
      evaluated against representative bindings instead of only parse-checked.
- [ ] Record explicit waivers for expressions that cannot yet be functionally
      evaluated because their host bindings, external resources, or Tier B/C
      features are unavailable. Waivers must include owner, reason, and removal
      condition.
- [ ] Wire the audit as an Nx verification target, for example
      `yarn nx run cem_ql:verify-embedded-expressions`, and include it in the
      Rust-first release gate after the parser/runtime migration is complete.
- [ ] Add regression coverage proving a stale XPath-style expression in a
      `.cem` or `.cemt` asset fails the audit with the exact source file and
      byte range.

### Deferred: Schema Package Folder Alignment

Resume after the Rust-first CEM-QL syntax/showcase slice is green.

- [ ] Add a schema-package structure audit that walks every
      `schema-packages/{package-id}/v1` folder and reports `package.cem`,
      `README.md`, manifest schema source, `examples/`, package-owned CEMT
      artifact paths, baseline formatter profiles
      `compact`/`pretty`/`tabular`, baseline colorizer profiles
      `terminal`/`html`/`md`, and converter template paths only when the
      manifest declares CEMT converters.
- [ ] Decide and encode the example-reference representation: generate
      checked-in `examples/*.example.cem` sidecars from current manifest example
      entries, or update the audit to explicitly accept manifest-owned example
      entries as the equivalent reference document described by the README.
- [ ] Add focused checks for the two schema-source filename exceptions so
      `cem-ml-generic.cem` and `cem-schema.cem` are intentional, or rename them
      and update their manifests/readmes to the literal `schema/{package-id}.cem`
      shape.
- [ ] Convert `schema-packages/*/v1` folders to Nx libraries with `*.cemt`
      sources tracked for caching; CLI tests should depend on package targets
      and invoke them through Nx.
- [ ] Keep converter endpoint checks as a final registry pass because current
      manifests contain cross-package edges (`cem-ml` to projections, `xml` to
      DOM projection, and DOM projection back to HTML/XML) that should not force
      a false per-folder dependency cycle.

Deferred dependency-ordered package checklist:

- [ ] `cem-ml/v1`
- [ ] `schema/v1`
- [ ] `schema-package/v1`
- [ ] `cem-native-template/v1`
- [ ] `cem-transform/v1`
- [ ] `cem-ql/v1` after the active Rust-first syntax slice updates its
      examples, formatter, and colorizer.
- [ ] `cem-ast-projection/v1`
- [ ] `cem-events-projection/v1`
- [ ] `json/v1`
- [ ] `json-schema/v1`
- [ ] `yaml/v1`
- [ ] `csv/v1`
- [ ] `markdown/v1`
- [ ] `xml/v1`
- [ ] `relax-ng/v1`
- [ ] `xhtml/v1`
- [ ] `svg/v1`
- [ ] `mathml/v1`
- [ ] `xslt/v1`
- [ ] `html/v1`
- [ ] `cem-dom-projection/v1`
- [ ] `css/v1`
- [ ] Run the final registry/package validation gate after the dependency
      checklist is green:
      `yarn nx run cem_ml:test:cli-schema-artifacts`,
      `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`,
      `yarn nx run cem_ml_cli:validate-converter-parity`,
      `yarn nx run cem_ml_cli:e2e`, then `yarn nx run cem_ml:test`.

### Deferred: CSV And Other Format Polish

- [ ] Resume CSV-specific polish only after Rust-first CEM-QL gates are green:
      keep CSV schema-owned parsing/validation, CEMT formatter/colorizer assets,
      and package verification intact while aligning it with the final
      schema-package folder audit.
- [ ] Keep JSON, YAML, XML, HTML, CSS, Markdown, SVG, MathML, XSLT, Relax NG,
      and projection-package formatter/colorizer work behind the
      schema-package folder alignment gate.

### Deferred: Phase 3 Custom-Element Runtime

- [ ] Resume Phase 3 custom-element runtime substrate expansion after the
      Rust-first CEM-QL syntax/showcase slice and deferred schema-package
      folder contract slice are closed.

### Deferred: Phase 4 CEM Component Set

- [ ] Add a Phase 4 component state-matrix coverage audit/gate that maps
      `docs/component-mvp.md` category state requirements to the executable
      primitive, state, and workflow browser assertions.
- [ ] Populate the first missing state fixture or assertion from that audit,
      prioritizing selected, expanded, empty, and loading coverage across
      navigation, content, and layout workflows.
- [ ] Verify the state-matrix slice with focused `@epa-wg/cem-components`
      target(s), then `yarn nx run @epa-wg/cem-components:verify`.

### Next Work Item

Continue the Rust-first CEM-QL slice with the first implementation gate:

1. Update `packages/cem_ql/src/diagnostics.rs`,
   `packages/cem_ql/src/lexer.rs`, and `packages/cem_ql/src/parser/pratt.rs`
   for Rust-first operators, including renaming
   `cem.ql.use_and_or` to `cem.ql.use_rust_boolean_ops`.
2. Convert `packages/cem_ql/tests/parser_recovery.rs` and
   `packages/cem_ql/tests/xpath_parity.rs` to Rust-first syntax as the first
   executable gate before broadening evaluator/runtime changes.
3. After the Rust tests pass, update the CEM-QL schema-package examples and add
   the Storybook operator/function showcase.
4. After the Rust-first gate is green, add the embedded-expression audit for
   all checked-in `*.cem` and `*.cemt` files and functionally validate the
   extracted CEM-QL expressions against owned fixtures.

## Current Verification Commands

- `yarn nx run cem_ql:test`
- `yarn nx run cem_ql:test:xpath-parity` (or the renamed Rust-first
  functional parity target once added)
- `yarn nx run cem_ql:test:set-operator-identity`
- `yarn nx run cem_ql:test:fixtures`
- `yarn nx run cem_ql:build:wasm`
- `yarn nx run cem-elements:verify`
- `yarn nx run cem_ql:verify-embedded-expressions` (to be added after
  Rust-first syntax lands)

Deferred schema-package/format gate commands:

- `yarn nx run cem_ml:test:cli-schema-artifacts`
- `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`
- `yarn nx run cem_ml_cli:validate-converter-parity`
- `yarn nx run cem_ml_cli:e2e`
- `yarn nx run cem_ml:test`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
