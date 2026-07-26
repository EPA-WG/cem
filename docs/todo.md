# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Current active slice: CSV source import into the lifecycle-owned internal AST
stream.

### Immediate: CSV Source Import To Internal AST Stream

- [x] Route `text/csv` and `https://cem.dev/ns/data/csv/1` inputs through a
      lifecycle adapter that emits a CEM-owned CSV document AST stream with
      source maps, and consume that stream from CSV convert/preview output
      instead of reparsing CSV through a direct convert bypass.

### Secondary: Generic Source Import Deviation Fixes

- [x] Move CEM-ML AST builder diagnostics behind schema-owned parse-fact
      bindings for `cem.ast.unbalanced_close`, `cem.ast.unclosed_scope`, and
      `cem.ast.unresolved_reference`, with Rust extracting neutral facts and
      `schema/cem-ml-generic.cem` owning `@fact-kind`, diagnostic code,
      severity, behavior, and policy.
- [x] Move CEM-ML document directive diagnostics behind schema-owned parse-fact
      bindings for `cem.doc.version_missing`, `cem.doc.semver_invalid`,
      `cem.doc.format_unknown`, `cem.doc.version_unsupported`,
      `cem.doc.prerelease_unmatched`, and `cem.doc.version_resolved`.
- [x] Move CEM-ML tokenizer diagnostics behind a pre-AST neutral fact stream
      interpreted by schema-owned bindings, including tokenizer scope and
      malformed lexical facts with source maps.
- [x] Audit remaining CEM-ML syntax and namespace diagnostic emitters: no
      active `cem.syntax.*` parser diagnostics are emitted today; unbound
      prefixes are currently reported as document-validation
      `cem.lint.unbound_prefix`, while unresolved namespace and scope
      diagnostics are emitted by the schema machine as `cem.schema.*`.
- [x] Decide the boundary for `cem.lint.unbound_prefix`: document-validation
      lints use a separate schema-owned semantic fact catalog rather than the
      parser fact catalog or the schema-machine diagnostic catalog.
- [x] Move active CEM-ML unbound-prefix document-validation lint behind the
      schema-owned semantic fact catalog, preserving AST source maps and
      letting `schema/cem-ml-generic.cem` own code, severity, behavior,
      fact kind, and policy for `cem.lint.unbound_prefix`.
- [x] Move remaining active CEM-ML syntax and namespace diagnostics behind
      schema-owned fact bindings after that boundary is decided, including
      invalid names if/when an active emitter exists, and schema-machine
      namespace/scope facts.
- [x] Move remaining CEM-ML schema-machine and handoff diagnostics behind the
      same neutral fact boundary, including unbalanced/unclosed schema scopes,
      unsupported/deferred handoffs, and XSLT dispatch/version diagnostics.
- [x] Move remaining CEM Core vocab and schema-scoping machine diagnostics
      behind schema-owned fact bindings, including unknown annotations,
      annotation values, state matrix violations, schema source scoping errors,
      invalid namespace directives, and non-streamable schema constraints.
- [x] Move remaining CEM-ML formatter writer-adjacent primitives for
      block-child whitespace and content-boundary construction into CEMT
      helpers, or declare the host-primitive contract explicitly as the
      tracked runtime boundary.
- [x] Add executable CEM-ML package review coverage that fails when the README
      “tracked but not complete” items are not represented as open todo
      checkitems or package-local waiver metadata.
- [ ] Add a JSON lifecycle input adapter that lowers `application/json`,
      `text/json`, and `https://cem.dev/ns/data/json/1` sources into the
      CEM-owned internal DOM/AST stream with source-map stacks instead of
      producing `cem.lifecycle.adapter_unsupported` or falling back to CEM
      syntax parsing.
- [ ] Move generic JSON/YAML data conversion off `serde_json::Value` or
      command-local document shortcuts and onto the generic source import
      DOM/AST boundary, or prove that the fast path emits identical DOM/AST,
      diagnostics, source-map, and artifact metadata.
- [x] Move CSV direct conversion and preview generation behind the same source
      import DOM/AST boundary, preserving row and field source ranges, parser
      facts, formatter/colorizer inputs, and writer source maps without a
      convert-only bypass.
- [x] Move CSV validate/check off the CLI-owned source validation collector and
      into `RealCemMlEngine` lifecycle validation, so `text/csv` validation
      consumes the lifecycle-owned `CsvDocumentAst` stream and schema-owned
      parse-fact diagnostics without falling through to CEM parsing.
- [ ] Move CLI-owned source validation collectors for JSON, YAML,
      Markdown, CSS, HTML, XML, SVG, MathML, Relax NG, XSLT, CEM-QL, and
      native-template behind engine lifecycle adapters, or prove each collector
      consumes the same imported DOM/AST and source-map model as
      `RealCemMlEngine`.
- [x] Fix the CEM-ML `ast` projection so it no longer aliases `dom_json`:
      replace `projection::ast_json` with a source-map-bearing typed CEM
      tree AST stream consumed as CEM-ML/CEMT data, not as a DOM/JSON
      projection or JSON-named internal boundary.
- [x] Replace CSV native input/output stream carriers that still use
      `serde_json::Value` with typed CSV/CEMT AST models, leaving any
      dynamic lowering isolated to explicit CEMT adapter boundaries.
- [ ] Fix or explicitly declare the reduced-fidelity contract for the CEM-ML
      `dom-json` debug projection when callers expect source-map preservation:
      `CemDocument` and the CEM tree AST stream preserve `SourceMapStack`, but
      `projection::dom_json` omits `sourceMap` and can expose collapsed byte
      ranges.
- [ ] Add regression coverage that every schema-package preview and validation
      path either uses the generic source import boundary or has an explicit
      tracked waiver with equivalent source-map and artifact metadata.

### Immediate: README Sample Preview Generation

- [x] Add a manifest-derived `samples2readme` generator that refreshes README
      example sections and SVG content previews from schema-package
      `package.cem` example metadata, using CLI tabular formatter/colorizer
      previews where executable and source snapshots where not.

### Completed Immediate Phase: CSV Formatter Review Findings

- [x] Document that CSV `pretty` and `tabular` are visual presentation formats:
      their alignment/trimming may produce strict-CSV deviations, permissive
      tools may recover by trimming leading/trailing field padding, and
      `compact` is required for non-visual data interchange.
- [x] Apply generic `lineEnding=lf|crlf|preserve` option to all CEMT formatter
      runtime bindings instead of only the CSV formatter path.

### Immediate: CSV Format-Support AC Remediation

- [x] Route CSV source projection through generic decoded-source semantics so
      UTF-8 BOM bytes are skipped from field content while raw byte length and
      absolute source ranges remain correct.
- [x] Add focused Rust and CLI coverage proving `utf8-bom.csv` compact output
      starts with `id`, not a UTF-8 BOM, and that row/field source ranges still
      point at the original byte offsets.
- [x] Promote CSV formatter/colorizer source-range metadata to the generic
      writer/source-map boundary instead of carrying it only as token `value`
      payload data.
- [x] Move the remaining Rust-owned CSV diagnostic policy dispatch behind
      schema-package behavior bindings so parser facts stay neutral and `.cem`
      owns diagnostic codes/severities.
- [x] Tighten package-local verify/build inputs so CSV package verification
      cannot pass against a stale `cem_ml_cli` binary when shared Rust code
      changes.
- [x] Replace the static schema-owned CLI validation example list with a
      `package.cem`-derived harness for package examples. The harness must load
      manifest-declared examples, preserve each example's content type, schema
      URI, expected result, and expected diagnostics, and keep any extra
      package-specific assertions layered on top instead of duplicating fixture
      registration.
- [x] Update CSV package example tests so `csv_package_examples_are_manifest_indexed`
      no longer has a stale hard-coded count. It should assert the manifest owns
      every checked-in CSV fixture and then verify the expected CSV example IDs,
      result states, content types, and diagnostic codes from the manifest data.
- [x] Prove the new manifest-derived CLI harness validates all 16 current CSV
      examples, including line-ending, BOM, spacing, tabs/empty-fields,
      formula-looking, and wide-Unicode fixtures.
- [x] Re-run focused gates after the manifest harness change:
      `cargo test -p cem-ml csv_package_examples_are_manifest_indexed`,
      `cargo test -p cem-ml-cli schema_owned_csv_examples_validate_through_cli`,
      and the smallest affected Nx target if available.
- [x] Settle the `@produces="tokens"` contract by keeping public
      formatter/colorizer output-stage assets on `@produces="cem-tree"` and
      documenting token arrays as writer-boundary implementation details, not the
      package formatter/colorizer artifact contract.
- [x] Migrate CSV formatter and colorizer CEMT assets to formatted/colored CEM
      tree output. Formatter assets now return `formatNodes` and ordered
      writer-token nodes; colorizer assets consume and return CEM trees with
      `colorNodes`; the generic writer performs final token-to-text emission.
- [x] Finish the deeper CSV formatted-tree migration by adding any missing
      schema-facing formatted-tree shape and moving Rust-owned `pretty`/`tabular`
      alignment, trimming, type inference, and display-width behavior into CEMT
      or declared host primitives.
- [x] Add package-local verify coverage for CSV examples and SVG preview drift:
      run documented README commands, compare stable stdout/rendered SVG output
      against `examples/previews/`, and fail the CSV package verify target on
      drift.
- [x] Follow through on parser data gaps after the example harness is green:
      expose row/field source ranges, quoting state, encoding/dialect facts, and
      recoverable/fatal parser facts in the schema-facing CSV table data consumed
      by formatter/colorizer stages.
- [x] Replace `csv_display_width`'s character-count implementation with a real
      terminal/display-width policy or narrow the `wide-unicode.csv` claim until
      executable coverage proves the intended behavior.

### Immediate: CEM-QL Format/HTML AC Remediation

- [x] Add CEM-QL lifecycle conversion coverage proving `convert` with
      `application/vnd.cem.query+cem-ql` enters the CEM-QL parser/AST path and
      does not emit `cem.lifecycle.adapter_unsupported`.
- [x] Wire CEM-QL formatter/colorizer package stages so operator, keyword,
      string, identifier, diagnostic, and legacy-token roles are applied instead
      of falling back to one raw `syntax.string` span.
- [x] Make `compact`, `pretty`, and `tabular` CEM-QL formatter profiles produce
      deterministic formatted CEM-tree output before the generic writer emits
      terminal or HTML bytes.
- [x] Add README command examples with adjacent SVG previews for CEM-QL terminal
      and HTML formatted output, and teach `verify-previews.mjs` to drift-check
      those outputs.
- [x] Extend `cem_ml_schema_package_cem_ql_v1:verify` so it fails when
      formatter/colorizer/HTML output regresses, not only when validation JSON
      changes.
- [x] Re-run focused gates:
      `cargo test -p cem-ml-cli schema_owned_cem_ql_examples_validate_through_cli`,
      CEM-QL formatter/colorizer conversion tests, and
      `yarn nx run cem_ml_schema_package_cem_ql_v1:verify`.

### Immediate: CEM-QL CSV-Parity Review Remediation

- [x] Move CEM-QL direct source-output conversion out of the CLI-only wrapper
      and into the engine/conversion layer or an explicit context extension so
      direct `RealCemMlEngine::convert` API users get the same parser,
      formatter/colorizer, HTML, diagnostics, metadata, and no
      `cem.lifecycle.adapter_unsupported` behavior as CLI users.
- [x] Add engine-level regression coverage proving direct
      `RealCemMlEngine::convert` handles CEM-QL source to native CEM-QL text and
      HTML with the package formatter/colorizer pipeline, not only through
      `cem_ml_cli::dispatch`.
- [x] Define the CEM-QL schema-facing parser/token fact report in
      `cem-ql/v1/README.md` and `schema/cem-ql.cem`, including source identity,
      UTF-8 status, token ranges, module URI facts, parser diagnostics, legacy
      syntax facts, recoverable/fatal disposition, and source-map preservation.
- [x] Move CEM-QL diagnostic policy toward schema-owned fact interpretation:
      keep Rust responsible for byte-accurate parsing/lexing, but make
      `cem.ql.*` code/severity ownership inspectable through schema-declared
      constraints/diagnostics rather than ad hoc bridge logic.
- [x] Expand CEM-QL README to match CSV's implemented package AC sections:
      standards/registry policy, generic `lineEnding` default and options,
      formatter profile semantics, colorizer behavior, safety/security boundary,
      formatter-and-preview SDLC, current/target parser boundary, release
      behavior, and tracked incomplete work.
- [x] Decide whether CEM-QL `compact`, `pretty`, and `tabular` should become
      distinct AST-aware layout profiles now or be documented/tested as
      intentional source-preserving aliases until layout rules exist.
- [x] Strengthen CEM-QL manifest/index tests to enumerate all expected examples,
      content types, expected results, and diagnostics like CSV does, not only
      invalid cases.
- [x] Add CEM-QL examples and manifest declarations for the non-ambiguous
      coverage cases: alias content type (`text/cem-ql`), LF/CRLF source
      fixtures, comments/whitespace, source token byte-range preservation, and
      invalid UTF-8 handling.
- [x] Decide and implement CEM-QL duplicate import/declaration semantic
      fixtures. Duplicate explicit import aliases and same-scope declarations
      now report hard schema-owned diagnostics before resolver/type/artifact
      stages.
- [x] Decide and implement the remaining CEM-QL semantic fixture:
      compiled artifact/cache identity.
  - [x] Decide denied/unresolved import semantics and CEM-ML resolver-policy
        ownership, including explicit substitution versus implicit fallback.
  - [x] Implement import-policy resolution diagnostics on CEM-QL validate and
        compile paths without making formatting or preview generation resolve
        external imports.
  - [x] Add the CEM-QL unresolved-import package example, manifest declaration,
        schema diagnostic binding, README coverage, and focused tests.
  - [x] Decide and implement type-error placeholder fixture semantics.
  - [x] Decide and implement compiled artifact/cache identity fixture semantics.
- [x] Extend package-local verify coverage so CEM-QL fails on direct engine
      conversion regressions, README/SVG drift, formatter profile drift, HTML
      wrapper/span-role drift, and manifest/example coverage gaps.
- [x] Re-run focused gates:
      CEM-QL direct engine conversion tests, CLI conversion tests,
      `cargo test -p cem-ml cem_ql_output_templates_are_schema_package_assets`,
      `cargo test -p cem-ml cem_ql_package_examples_are_manifest_indexed`,
      `cargo test -p cem-ml-cli schema_owned_cem_ql_examples_validate_through_cli`,
      and `yarn nx run cem_ml_schema_package_cem_ql_v1:verify`.

### Immediate: CEM-Native Template Package Review Remediation

- [x] Strengthen `cem-native-template/v1` README coverage for standards/source
      identity, generic LF `lineEnding` behavior, formatter/colorizer profile
      semantics, parser facts and diagnostic ownership, safety notes,
      verification gates, release behavior, and tracked incomplete work.
- [x] Add README command examples with adjacent SVG previews for the
      schema-owned validation report and colored `pretty` formatter output, and
      add package-local preview drift checking.
- [x] Strengthen the manifest-index Rust test so it enumerates every
      `cem-native-template` example ID, content type, expected result, and
      expected diagnostic code instead of only checking the invalid fixture.
- [x] Extend `cem_ml_schema_package_cem_native_template_v1:verify` so it runs
      schema-owned example validation and README/SVG drift checks, not only
      manifest validation.
- [x] Re-run focused gates:
      `cargo test -p cem-ml cem_native_template_package_examples_are_manifest_indexed`,
      `cargo test -p cem-ml-cli schema_owned_cem_native_template_examples_validate_through_cli`,
      and
      `yarn nx run cem_ml_schema_package_cem_native_template_v1:verify`.
- [x] Implement schema-owned duplicate import alias, duplicate template
      entrypoint, duplicate param, duplicate let, unknown same-module call, and
      reserved default-expression fixtures for `cem-native-template/v1`.
- [x] Map native-template source validation for those fixtures to declared
      `cem.template.*` diagnostics while preserving CEMT transform diagnostics
      on the existing `cem.transform_template.*` boundary.
- [x] Update package manifest, README example table, manifest-index assertions,
      and package-local verify coverage for the new semantic fixtures.
- [x] Decide and implement CEM-native template `{import}` denial/unresolved
      semantics for compile/preflight: source validation stays passive, local
      paths and local `file://` are default-resolvable, remote/custom schemes
      require a registered template resolver, no implicit fallback is attempted,
      template-owned `cem.template.import_denied` /
      `cem.template.import_unresolved` diagnostics preserve requested identity,
      source range, resolver code, and cache-stamp behavior, and successful
      dependency hashes include parent URI, alias, requested URI,
      content-type/schema hints, resolved URI, and content hash.
- [x] Decide and implement explicit substitution support in the shared CEM-ML
      resolver API, including substituted identity, substitution-policy stamps,
      diagnostics, and cache/dependency stamp behavior for CEM-native template
      imports.
  - [x] Add a generic resolver-policy decision boundary before resolver reads,
        keeping resource resolvers as byte adapters.
  - [x] Add default resolver-policy behavior that preserves current local and
        relative import reads while making explicit substitutions opt-in.
  - [x] Route CEM-native template import preflight through the policy decision
        before local or registered resolver reads.
  - [x] Preserve requested URI, normalized URI, substituted URI, effective URI,
        and resolver-policy stamp in diagnostics, resolved import metadata, and
        dependency hashes.
  - [x] Add focused tests for explicit substitution success and failed
        substitution resolving as `cem.template.import_unresolved`.
  - [x] Document the shared policy/resolver split in package-level and
        CEM-native template README guidance.
- [x] Decide expression-schema ownership for template expressions: CEM-QL owns
      the shared expression schema; CEM-native template owns expression slot
      context, expected type/nullability, evaluation phase, and provenance.
- [x] Document the shared expression contract in the package-level README,
      CEM-QL README, CEM-native template README, and existing schema delegation
      policy labels.
- [x] Add standalone CEM-QL expression API and CLI execution to the Phase 2
      roadmap.

### Immediate: Shared CEM-QL Expression Contract Execution

- [x] Define the schema-facing standalone CEM-QL expression resource contract:
      candidate content type, source identity, data/context input model, result
      value model, diagnostics, source maps, resolver-policy stamp, and package
      examples.
- [x] Add a `cem_ql` Rust API that can compile and evaluate one standalone
      expression against a typed data/context input without requiring a query
      module wrapper.
- [x] Add CEM-QL expression execution to the CEM-ML `transform` command:
      inline expressions use `--template-expression`, file-backed expression
      transformations use `--template *.cem-ql`, and both share transform run
      config, input content-type/schema identity, resolver policy, output
      formatting, and diagnostic reporting.
- [x] Add package-owned CEM-QL examples and tests for valid standalone
      expression execution, parse errors, type errors, source ranges, and data
      binding failures.
- [x] Wire CEM-native template expression slots to consume the shared CEM-QL
      expression fact/result contract and preserve template slot provenance.
- [x] Add invalid CEM-native template expression fixtures once the shared
      CEM-QL expression fact report is executable.
  - [x] Add an invalid template expression parse fixture that expects
        `cem.ql.use_rust_boolean_ops` with `expressionSlot` provenance.
  - [x] Add an invalid template expression type fixture that expects
        `cem.ql.type_error` with expected result type provenance.
  - [x] Add an invalid template expression data-binding fixture that expects
        `cem.ql.data_binding_missing` with available binding provenance.
  - [x] Route CLI CEM-native template validation through the embedded CEM-QL
        expression audit while keeping core CEM-ML dependency direction intact.
  - [x] Update CEM-native manifest, README, package example index tests, and
        package verify coverage for the invalid expression fixtures.

### Schema Package Folder Alignment

- [x] Align `cem-events-projection/v1` package folder with common package AC.
  - [x] Strengthen the `cem-events-projection` manifest-index Rust test so it
        enumerates every example ID, content type, schema, expected result, and
        expected diagnostic code.
  - [x] Add README SVG previews immediately after the binary and JSON debug
        validation command examples.
  - [x] Add package-local preview drift checking for
        `cem-events-projection/v1`.
  - [x] Extend `cem_ml_schema_package_cem_events_projection_v1:verify` so it
        runs manifest validation, manifest-index coverage, schema-owned CLI
        example validation, and README/SVG preview drift checks.
  - [x] Expand `cem-events-projection/v1` README coverage for source identity,
        binary/JSON event-stream contracts, formatter/colorizer absence,
        parser/diagnostic ownership, safety notes, verification gates, release
        behavior, and tracked incomplete work.

- [x] Align `cem-ast-projection/v1` package folder with common package AC.
  - [x] Strengthen the `cem-ast-projection` manifest-index Rust test so it
        enumerates every example ID, content type, schema, expected result, and
        expected diagnostic code.
  - [x] Add README SVG previews immediately after the binary and JSON debug
        validation command examples.
  - [x] Add package-local preview drift checking for `cem-ast-projection/v1`.
  - [x] Extend `cem_ml_schema_package_cem_ast_projection_v1:verify` so it runs
        manifest validation, manifest-index coverage, schema-owned CLI example
        validation, and README/SVG preview drift checks.
  - [x] Expand `cem-ast-projection/v1` README coverage for source identity,
        binary/JSON projection contracts, formatter/colorizer absence,
        parser/diagnostic ownership, safety notes, verification gates, release
        behavior, and tracked incomplete work.

- [x] Align `cem-transform/v1` package folder with common package AC.
  - [x] Strengthen the `cem-transform` manifest-index Rust test so it
        enumerates every example ID, content type, schema, expected result, and
        expected diagnostic code instead of relying on a hard-coded count and
        partial assertions.
  - [x] Add README command examples with adjacent SVG previews for the
        schema-owned validation report and colored formatter output.
  - [x] Add package-local preview drift checking for `cem-transform/v1`.
  - [x] Extend `cem_ml_schema_package_cem_transform_v1:verify` so it runs
        manifest validation, manifest-index coverage, schema-owned CLI example
        validation, and README/SVG preview drift checks.
  - [x] Expand `cem-transform/v1` README coverage for source identity, generic
        LF `lineEnding`, formatter/colorizer profile semantics, parser facts
        and diagnostic ownership, safety notes, verification gates, release
        behavior, and tracked incomplete work.

- [x] Align `cem-dom-projection/v1` package folder with common package AC.
  - [x] Strengthen the `cem-dom-projection` manifest-index Rust test so it
        enumerates every example ID, content type, schema, expected result, and
        expected diagnostic code.
  - [x] Add README SVG previews immediately after the binary and JSON debug
        validation command examples.
  - [x] Add package-local preview drift checking for
        `cem-dom-projection/v1`.
  - [x] Extend `cem_ml_schema_package_cem_dom_projection_v1:verify` so it runs
        manifest validation, manifest-index coverage, schema-owned CLI example
        validation, converter/parity coverage for the CEMT DOM HTML/XML edges,
        and README/SVG preview drift checks.
  - [x] Expand `cem-dom-projection/v1` README coverage for source identity,
        binary/JSON DOM contracts, CEMT converter/fallback semantics,
        formatter/colorizer absence, parser/diagnostic ownership, safety
        notes, verification gates, release behavior, and tracked incomplete
        work.

- [x] Align `cem-ml/v1` package folder with common package AC.
  - [x] Strengthen the `cem-ml` manifest-index Rust test so it enumerates
        every example ID, content type, schema, expected result, and expected
        diagnostic code.
  - [x] Update `cem-ml-generic.cem` diagnostic declarations so the schema names
        the current package/runtime diagnostics used by manifest examples and
        validation reports.
  - [x] Add README SVG previews immediately after CEM-ML validation and
        formatter/colorizer command examples.
  - [x] Add package-local preview drift checking for `cem-ml/v1`.
  - [x] Extend `cem_ml_schema_package_cem_ml_v1:verify` so it runs manifest
        validation, manifest-index coverage, schema-owned CLI example
        validation, formatter/colorizer package-asset coverage, and README/SVG
        preview drift checks.
  - [x] Expand `cem-ml/v1` README coverage for source identity, generic LF
        `lineEnding`, bootstrap syntax facts, parser/diagnostic ownership,
        formatter/colorizer profiles, safety notes, verification gates, release
        behavior, and tracked incomplete work.

- [x] Align `csv/v1` package folder with common package AC.
  - [x] Keep the existing explicit `csv_package_examples_are_manifest_indexed`
        coverage wired into the package-local verify target.
  - [x] Extend `cem_ml_schema_package_csv_v1:verify` so it runs manifest
        validation, manifest-index coverage, schema-owned CLI example
        validation, formatter profile coverage, formatter option coverage,
        colorizer profile coverage, HTML/terminal writer parity coverage, and
        README/SVG preview drift checks.
  - [x] Fix CSV README standards/policy table structure and make the
        `header` parameter policy readable as one row.
  - [x] Expand CSV README verification and release-behavior coverage so the
        package-local gates, strict/interchange defaults, visual formatter
        lossiness, and tracked incomplete work are explicit.
  - [x] Harden CSV preview drift checking so update mode creates both the
        checked-in preview directory and generated preview directory.

Remaining dependency-ordered package checklist:
- [x] `cem-ql/v1`
- [x] `cem-native-template/v1`
- [x] `cem-transform/v1`
- [x] `cem-ast-projection/v1`
- [x] `cem-events-projection/v1`
- [x] `cem-dom-projection/v1`
- [x] `cem-ml/v1`
- [x] `csv/v1`
- [ ] `json/v1`
- [ ] `json-schema/v1`
- [ ] `yaml/v1`
- [ ] `markdown/v1`
- [ ] `xml/v1`
- [ ] `relax-ng/v1`
- [ ] `xhtml/v1`
- [ ] `svg/v1`
- [ ] `mathml/v1`
- [ ] `xslt/v1`
- [ ] `html/v1`
- [ ] `css/v1`
- [ ] Run the final registry/package validation gate after the dependency
      checklist is green:
      `yarn nx run cem_ml:test:cli-schema-artifacts`,
      `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`,
      `yarn nx run cem_ml_cli:validate-converter-parity`,
      `yarn nx run cem_ml_cli:e2e`, then `yarn nx run cem_ml:test`.

### Deferred: Other Format Polish

- [ ] Keep JSON, YAML, XML, HTML, CSS, Markdown, SVG, MathML, XSLT, Relax NG,
      and projection-package formatter/colorizer work behind the
      schema-package folder alignment gate.

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

### Next Work Item

Review and align `json/v1` against the common schema-package AC:
manifest-derived examples, README command/SVG preview drift, package-local
verify coverage, source identity, parser/diagnostic ownership, formatter and
colorizer contracts, JSON standards/interchange boundaries, and any fixture gaps
that need explicit todo checkitems.

## Current Verification Commands

- `yarn nx run cem_ml:test:schema-package-structure`
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
