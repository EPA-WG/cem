# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Current active slice: strict native CEMT transform boundary.

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
- [x] Add a JSON lifecycle input adapter that lowers `application/json`,
      `text/json`, and `https://cem.dev/ns/data/json/1` sources into the
      CEM-owned internal DOM/AST stream with source-map stacks instead of
      producing `cem.lifecycle.adapter_unsupported` or falling back to CEM
      syntax parsing.
- [x] Move JSON/YAML data conversion off `serde_json::Value`,
      command-local document shortcuts, and direct format-pair bridges. JSON
      and YAML conversion now routes through source package import AST, the
      generic AST stream boundary, and target package output; future JavaScript
      object-like inputs such as JSONP must follow the same pattern rather than
      coupling to JSON directly.
- [x] Apply the same generic AST stream boundary to CSV and every future
      data-format content conversion, so new formats add source and target
      adapters around the generic boundary instead of direct pair converters.
- [x] Add focused CSV generic AST cross-format fixtures for `header=present`
      CSV to JSON and JSON object to CSV so the bridge proves target bytes, not
      only conversion metadata.
- [x] Add conversion-boundary validation coverage that fails when any
      content-type conversion directly couples two concrete data formats
      without the generic AST stream between them, including JSON, YAML, CSV,
      and future JavaScript object-like formats.
- [x] Move CSV direct conversion and preview generation behind the same source
      import DOM/AST boundary, preserving row and field source ranges, parser
      facts, formatter/colorizer inputs, and writer source maps without a
      convert-only bypass.
- [x] Move CSV validate/check off the CLI-owned source validation collector and
      into `RealCemMlEngine` lifecycle validation, so `text/csv` validation
      consumes the lifecycle-owned `CsvDocumentAst` stream and schema-owned
      parse-fact diagnostics without falling through to CEM parsing.
- [x] Prove the CLI JSON/YAML source validation collectors consume the same
      typed source import AST and schema-owned parse-fact diagnostics as
      `RealCemMlEngine`, removing CLI-local JSON/YAML parser shortcuts while
      preserving direct-report compatibility for mixed validation inputs.
- [x] Prove the CLI HTML/XML-family source validation collectors consume the
      same shared package/library validators as schema-package example
      validation: HTML, XHTML, XML, SVG, MathML, and Relax NG collector entry
      points now delegate to `cem_ml::validation::*::validate_*_source_bytes`
      with source bytes, URI, content type, and source-map diagnostics
      preserved; XSLT has no CLI-owned direct collector and already validates
      through the lifecycle/package validator path.
- [x] Move Markdown and CSS source validation out of CLI-owned collectors and
      into shared `cem_ml::validation::{markdown,css}` package/library
      validators; CLI direct validation and schema-package example validation
      now both delegate through the same bytes, URI, content type, and
      source-map diagnostic boundary.
- [x] Move the CEM-QL source validation collector out of CLI-owned parser
      dispatch and into the `cem_ml_transform_cem_ql` bridge crate, preserving
      module and expression validation through the CEM-QL parser/compiler,
      import resolution, type checking, source-map diagnostic projection, and
      the existing `cem_ml`/`cem_ql` dependency boundary.
- [x] Move the native-template source validation collector behind the
      `cem_ml_transform_cem_ql` bridge boundary: CLI dispatch now delegates
      generic template embedding validation and native-template embedded
      CEM-QL expression validation through shared source-byte, identity, URI,
      and source-map request APIs instead of directly invoking `cem_ql`.
- [x] Fix the CEM-ML `ast` projection so it no longer aliases `dom_json`:
      replace `projection::ast_json` with a source-map-bearing typed CEM
      tree AST stream consumed as CEM-ML/CEMT data, not as a DOM/JSON
      projection or JSON-named internal boundary.
- [x] Replace CSV native input/output stream carriers that still use
      `serde_json::Value` with typed CSV/CEMT AST models, leaving any
      dynamic lowering isolated to explicit CEMT adapter boundaries.
- [x] Fix the CEM-ML `dom-json` debug projection source-map contract:
      `projection::dom_json` now emits full `sourceMap` stacks for document,
      element, attribute, and leaf nodes while preserving legacy `byteRange`
      fields for compatibility.
- [x] Add regression coverage that every schema-package preview and validation
      path either uses the generic source import boundary or has an explicit
      tracked waiver with equivalent source-map and artifact metadata.
- [x] Preserve YAML comments/directives in the typed YAML AST stream and render
      them from package CEMT once parser coverage exposes those nodes; until
      then, YAML formatter/colorizer ownership covers the typed document node
      model without comment/directive presentation nodes.
      - [x] Render schema-owned YAML directives from the typed stream in the
            package CEMT formatter/colorizer path, including the required
            explicit document start after directive headers.
      - [x] Expose YAML comments as typed parser presentation nodes with byte
            ranges/source maps before rendering them; the current `yaml-rust2`
            scanner skips comments and provides no comment token/event boundary.
            - [x] Confirm the current Rust YAML parser boundary cannot expose
                  comments: `yaml-rust2` skips comments inside scanner
                  whitespace handling and has no public comment token/event.
            - [x] Choose and implement the YAML comment parser strategy before
                  adding render output: patch/upstream comment events in the
                  YAML parser, adopt a presentation parser for YAML trivia, or
                  build a schema-owned trivia lexer with fixtures covering
                  quoted scalars, block scalars, inline comments, and full-line
                  comments.
- [x] Interleave YAML comment presentation nodes by source position in package
      formatter output, including trailing inline comments and in-document
      full-line comments. Positioned comments now render by source line/indent;
      unpositioned legacy comment subjects keep the stream-level fallback.

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
- [x] `yaml/v1`
  - [x] Add schema-owned YAML parse-fact bindings for parse errors,
        unsupported UTF-8, unresolved aliases, duplicate anchors, unsafe tags,
        and source-map-unavailable facts.
  - [x] Add a typed YAML lifecycle AST stream and register a YAML input adapter
        so YAML validation no longer falls through to CEM parsing.
  - [x] Remove JSON subject/token declarations from YAML formatter/colorizer
        CEMT boundaries in favor of typed YAML document/CEM tree boundaries.
  - [x] Add YAML target/export selection only with the first-class YAML output
        layer.
  - [x] Add first-class YAML export support with a YAML layer format and route
        same-schema YAML conversion through the YAML formatter/colorizer/writer
        pipeline instead of a generic JSON-value bridge.
  - [x] Expand YAML formatter/colorizer bodies for compact, pretty, tabular,
        terminal, HTML, and Markdown profile semantics with drift tests.
  - [x] Expand `cem_ml_schema_package_yaml_v1:verify` so it runs manifest
        validation, manifest-index coverage, schema-owned CLI example
        validation, formatter/colorizer tests, README/SVG preview drift, and
        generated artifact drift.
  - [x] Expand YAML README Verification, Release Behavior, and tracked
        incomplete-work sections.
- [x] `json/v1`
  - [x] Add schema-owned JSON parse-fact bindings for parse errors,
        unsupported UTF-8, duplicate member names, and source-map-unavailable
        facts.
  - [x] Add a typed JSON lifecycle AST stream and register a JSON input adapter
        so JSON validation no longer falls through to CEM parsing.
  - [x] Add JSON target/export selection with a first-class JSON output layer.
  - [x] Route same-schema JSON conversion through the JSON lifecycle AST stream
        instead of the generic `serde_json::Value` bridge.
  - [x] Expand `cem_ml_schema_package_json_v1:verify` so it runs manifest
        validation, manifest-index coverage, lifecycle adapter/export tests,
        engine same-schema conversion coverage, and CLI same-schema coverage.
  - [x] Expand JSON README Verification, Release Behavior, and tracked
        incomplete-work sections.
  - [x] Move JSON formatter/colorizer CEMT assets from shared `json`/`tokens`
        stubs to typed JSON document/CEM-tree boundaries, covering compact,
        pretty, tabular, terminal, HTML, and Markdown semantics.
  - [x] Add JSON README/SVG preview drift checks after package-owned JSON
        formatter/colorizer bodies render the example previews.
- [x] `json-schema/v1`
  - [x] Expand package-local verification so JSON Schema runs manifest
        validation, manifest-index coverage, embedded artifact catalog tests,
        CLI validation behavior tests, and README/SVG preview drift checks.
  - [x] Move JSON Schema source validation out of CLI-owned ad hoc JSON parse
        and dialect diagnostics into an engine-reachable typed JSON Schema AST
        stream with neutral parse/dialect facts and schema-owned diagnostic
        bindings.
  - [x] Move JSON Schema formatter/colorizer CEMT assets away from raw
        `json`/`tokens` boundaries to package-owned JSON Schema document and
        formatted/colored CEM-tree boundaries.
  - [x] Route syntax-valid JSON Schema README previews through package
        formatter/colorizer output now that the typed output layer exists.
  - [x] Add a JSON Schema `nested-data` fixture with three nested object
        enclosures for tabular formatter preview coverage.
  - [x] Add a recoverable JSON Schema invalid-input preview path for parse and
        dialect-failure examples, then remove the remaining source-snapshot
        preview fallback for `json-schema/v1`.
- [x] `markdown/v1`
  - [x] Expand package-local verification so Markdown runs manifest
        validation, manifest-index coverage, embedded artifact catalog tests,
        CLI validation behavior tests, and README/SVG preview drift checks.
  - [x] Move Markdown source validation out of CLI-owned direct validation into
        an engine-reachable typed Markdown AST stream with neutral parse,
        encoding, variant, and embedded-HTML facts plus source-map ranges.
  - [x] Move Markdown formatter/colorizer CEMT assets away from raw
        `json`/`tokens` stubs to package-owned Markdown document and
        formatted/colored CEM-tree boundaries.
  - [x] Route Markdown README previews through package formatter/colorizer
        output now that the typed output layer exists; expected-fail examples
        preview schema-owned validation diagnostics instead of source
        snapshots.
  - [x] Add a Markdown-to-HTML README fixture that converts `markdown1.md`
        through the typed Markdown AST stream into HTML tabular formatter
        output, including a fenced `cem-ml svg` block rendered as inline SVG
        markup and previewed as `markdown1.md.html`.
  - [x] Replace Markdown README SVG previews with CLI-generated HTML files
        embedded as fenced `html` snippets, and stop generating
        `examples/previews/*.svg` for `markdown/v1`.
  - [x] Add regression coverage for GFM Markdown table and task-list
        conversion through the typed Markdown AST stream into HTML output.
- [x] `xml/v1`
  - [x] Move generic XML source validation into an engine-reachable typed XML
        lifecycle AST stream with source ranges, neutral parser facts, and
        schema-owned diagnostic bindings for parse, encoding, namespace,
        duplicate-attribute, DTD, external-entity, and source-map conditions.
  - [x] Move XML formatter/colorizer CEMT assets from placeholder
        `json`/`tokens` contracts to package-owned XML document and
        formatted/colored CEM-tree boundaries with compact, pretty, tabular,
        terminal, HTML, and Markdown profile coverage.
  - [x] Route same-schema XML conversion through the typed XML lifecycle AST
        and package output pipeline without CEM/HTML parser fallthrough, while
        preserving declarations, namespaces, comments, CDATA, processing
        instructions, source maps, and the default final newline.
  - [x] Expand `cem_ml_schema_package_xml_v1:verify` so it runs manifest
        validation, manifest-index coverage, schema-owned CLI example
        validation, lifecycle adapter/export tests, formatter/colorizer tests,
        and README/SVG preview drift checks.
  - [x] Route syntax-valid XML README previews through package formatter and
        colorizer output, render expected-fail examples from schema-owned
        validation diagnostics, and remove source-snapshot fallback.
  - [x] Expand the XML README for standards/registry mapping, source identity,
        parser facts and diagnostic ownership, formatter/colorizer profiles,
        resolver and entity safety, verification gates, release behavior, and
        tracked incomplete work.
- [x] `relax-ng/v1`
  - [x] Add a typed dual-syntax `RelaxNgDocumentAst` with explicit XML/compact
        syntax identity, lossless XML events or compact tokens, source ranges,
        source-map stacks, media-type parameters, and line-ending metadata.
  - [x] Move RELAX NG validation behind a dedicated lifecycle adapter and bind
        neutral parse, encoding, namespace/root, pattern/start, required
        attribute, include/external-reference, and source-map facts to
        diagnostics declared in `schema/relax-ng.cem`.
  - [x] Route same-schema XML and compact conversion through package-owned typed
        lifecycle output without CEM or generic XML fallthrough, preserving the
        input syntax and default final newline.
  - [x] Replace placeholder `json`/`tokens` artifacts with executable XML and
        compact CEMT boundaries for compact/pretty/tabular formatters and
        terminal/HTML/Markdown colorizers.
  - [x] Expand package verification for manifest/index coverage, embedded
        artifacts, validator and reject-only resolver policy, lifecycle/export,
        engine/CLI conversion, all formatter/colorizer profiles, schema-owned
        example validation, and README/SVG preview drift.
  - [x] Route passing README examples through package formatter/colorizer output,
        render expected failures from schema-owned diagnostics, remove compact
        source-snapshot fallback, and document safety/release behavior plus
        tracked limitations.
- [x] `xhtml/v1`
  - [x] Add a dedicated typed `XhtmlDocumentAst` lifecycle stream that
        preserves XHTML identity, XML lexical events, foreign-content
        boundaries, source ranges, and source maps.
  - [x] Route XHTML loading, validation, and same-schema output through the
        dedicated XHTML adapter without HTML, generic XML, or CEM fallthrough.
  - [x] Bind neutral parser, namespace, structure, profile, doctype, entity,
        foreign-content, and source-map facts to schema-owned diagnostics while
        retaining XML well-formedness and entity safety policy.
  - [x] Replace placeholder formatter/colorizer artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT profiles that
        preserve lexical XHTML and the default final newline.
  - [x] Expand manifest/index embedding, lifecycle, engine/CLI, profile,
        schema-owned example, README/SVG preview, safety, and release gates.
- [x] `svg/v1`
  - [x] Add a dedicated typed `SvgDocumentAst` lifecycle stream that preserves
        SVG identity, XML lexical events, qualified names and XLink attributes,
        foreign-content boundaries, MIME parameters, ranges, and source maps.
  - [x] Route standalone SVG content type, package schema, and namespace
        identities through the dedicated SVG adapter without HTML, generic XML,
        or CEM fallthrough while retaining embedded HTML/XHTML SVG handling.
  - [x] Bind XML safety, root/namespace, `viewBox`, accessibility, URI/resource,
        script/event-handler, foreign-content, and source-map facts to
        diagnostics declared in `schema/svg.cem`.
  - [x] Replace placeholder formatter/colorizer artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT profiles that
        preserve lexical SVG and the default final newline.
  - [x] Expand manifest/index embedding, lifecycle, engine/CLI, profile,
        schema-owned example, README/SVG preview, safety, and release gates.
- [x] `mathml/v1`
  - [x] Add a dedicated typed `MathMlDocumentAst` lifecycle stream over the
        generic XML event model, preserving all three media types, selected
        profile, MIME parameters, lexical events, ranges, and source maps.
  - [x] Route standalone MathML content type, package schema, and namespace
        identities through the dedicated adapter without HTML, generic XML, or
        CEM fallthrough while retaining embedded HTML/XHTML MathML handling.
  - [x] Bind XML safety, root/namespace, media-profile, expression,
        semantics/annotation, accessibility, external URI, foreign-content,
        and source-map facts to diagnostics declared in `schema/mathml.cem`.
  - [x] Replace placeholder formatter/colorizer artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT profiles that
        preserve lexical MathML, media-profile identity, and the final newline.
  - [x] Expand manifest/index embedding, lifecycle, engine/CLI, all-media-type,
        schema-owned example, README/SVG preview, safety, and release gates.
- [x] `xslt/v1`
  - [x] Add a dedicated typed `XsltStylesheetAst` lifecycle stream over the
        generic XML event model, preserving both standard media types, MIME
        parameters, lexical XML, stylesheet version, XPath-bearing attributes,
        ranges, and source maps.
  - [x] Route standard content type, package schema, and namespace identities
        through the dedicated adapter without generic XML, HTML, CEM, or legacy
        lowering fallthrough while retaining all four custom-element aliases on
        the bounded compatibility adapter.
  - [x] Bind XML safety, root/namespace, version, entrypoint, external URI,
        extension instruction/function, browser-engine policy, declaration,
        literal-result, XPath, and source-map facts to `schema/xslt.cem`.
  - [x] Replace placeholder formatter/colorizer artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT profiles that
        preserve lexical stylesheets and the default final newline.
  - [x] Expand manifest/index embedding, lifecycle, engine/CLI, standard and
        compatibility identity, parity, README/SVG preview, safety, and release
        gates without source-snapshot previews.
- [x] `html/v1`
  - [x] Add a dedicated typed `HtmlDocumentAst` lifecycle stream over the
        native HTML tokenizer, preserving media parameters, document/fragment
        mode, lexical events, semantic names, raw text/RCDATA, foreign-content
        namespace transitions, recovery evidence, ranges, and source maps.
  - [x] Route HTML loading, validation, and same-schema output through the
        dedicated adapter without CEM, XML, or legacy HTML-token fallthrough,
        while keeping XHTML separate and SVG/MathML islands document-owned.
  - [x] Bind neutral parser, encoding, doctype/quirks, recovery, duplicate
        attribute, script/event-handler, external-resource, custom-element,
        foreign-content, and source-map facts to `schema/html.cem`.
  - [x] Replace placeholder output artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT wrappers and
        helpers that preserve lexical HTML and the default final newline.
  - [x] Expand manifest/index embedding, lifecycle, engine/CLI, formatter and
        colorizer profile, schema-owned example, safety, release documentation,
        and README/SVG preview-drift gates without source-snapshot fallback.
- [x] `css/v1`
  - [x] Add a typed, lossless `CssDocumentAst` lifecycle stream for stylesheet,
        declaration-list/style-attribute, and scoped style-block entry modes.
        Use a standards CSS tokenizer/parser for component-value recovery and a
        presentation/trivia layer for comments, original lexemes, byte ranges,
        source maps, MIME parameters, encoding evidence, and line endings.
  - [x] Move CSS validation behind the lifecycle adapter and replace Rust-owned
        diagnostic dispatch with neutral facts bound through executable
        contracts in `schema/css.cem`, including syntax/recovery, charset,
        selector/declaration, unknown at-rule, import, URL, scope, and source-map
        facts.
  - [x] Route content type and package-schema loading plus same-schema output
        through the typed CSS stream without CEM or opaque-handoff fallthrough;
        preserve custom properties, vendor syntax, and unknown at-rules.
  - [x] Replace bodyless `json`/`tokens` artifacts with executable
        compact/pretty/tabular and terminal/HTML/Markdown CEMT wrappers and
        CEM-tree helpers, preserving compact lexical source and the default
        final newline.
  - [x] Define resolver/sanitizer capability boundaries for `@import`, `url()`,
        fonts, and other external references; validation and formatting must not
        fetch resources or interpret host-document cascade semantics.
  - [x] Route passing previews through package output profiles and expected
        failures through schema diagnostics, then expand manifest/index,
        lifecycle, engine/CLI, profile, schema-owned example, safety, release,
        and README/SVG drift verification.
- [x] Run the final registry/package validation gate after the dependency
      checklist is green.
  - [x] Run `yarn nx run cem_ml:test:cli-schema-artifacts`.
  - [x] Cover the current default YAML package indent and final newline in the
        focused generic-data runtime test, then refresh the stale native-output
        fixture expectation.
  - [x] Cover both scheduler scopes and typed YAML conversion metadata in the
        focused mixed-output report test, then refresh the stale report
        expectations.
  - [x] Allow typed package output-pipeline implementation identifiers in the
        CLI report schema.
  - [x] Run `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`.
  - [x] Run `yarn nx run cem_ml_cli:validate-converter-parity`.
  - [x] Run `yarn nx run cem_ml_cli:e2e`.
    - [x] Cover same-document fragment form actions as resolver-free HTML
          resources.
    - [x] Keep paired CEM/HTML semantic fixtures within the zero-hard resource
          policy by using local fragment form actions.
  - [x] Run `yarn nx run cem_ml:test`.
    - [x] Keep context package output-artifact tests isolated from unrelated
          built-in CEM-ML schema diagnostics.
    - [x] Declare JSON, XML, and RELAX NG helper function names in their package
          manifests.

### Other Format Polish

- [x] Keep SVG, MathML, XSLT, HTML, and CSS formatter/colorizer work behind
      the schema-package folder alignment gate.
- [x] Polish SVG formatter and colorizer profile semantics on the dedicated
      `SvgDocumentAst` path.
  - [x] Add focused Rust fixtures covering declarations, nested graphics,
        wrapped attributes, comments, CDATA, namespace-qualified attributes,
        foreign-content islands, and text-sensitive SVG elements.
  - [x] Make `compact`, `pretty`, and `tabular` produce distinct deterministic
        structural layouts without rewriting meaningful text, `style`,
        `script`, `foreignObject`, or namespace lexemes.
  - [x] Split start/end tags into delimiter, element-name, attribute-name,
        equals, and attribute-value tokens so terminal, HTML, and Markdown
        color profiles do not color an entire tag as one syntax name.
  - [x] Preserve source maps and output spans through inserted layout tokens,
        retain the configured line ending and indentation controls, and append
        exactly one final newline for text output.
  - [x] Refresh SVG package previews and run
        `yarn nx run cem_ml_schema_package_svg_v1:verify`, converter parity,
        CLI e2e, and `cem_ml:test`.
- [x] Polish MathML formatter and colorizer profile semantics on the dedicated
      `MathMlDocumentAst` path.
  - [x] Extract the SVG markup-token projection into a shared typed XML-family
        helper without moving MathML layout policy out of its schema package.
  - [x] Add focused presentation, content, semantics/annotation, namespace,
        CDATA, comment, and mixed-text fixtures before changing output.
  - [x] Define distinct deterministic `compact`, `pretty`, and `tabular`
        layouts while preserving token-sensitive math text and annotation
        payloads byte-for-byte.
  - [x] Apply delimiter, element-name, attribute-name, equals, and value roles
        consistently across terminal, HTML, and Markdown color profiles.
  - [x] Preserve lexical source maps, leave generated layout unmapped, honor
        indentation and line-ending controls, refresh previews, and run the
        MathML package, parity, CLI e2e, and core gates.
  - [x] Prefer language-tagged fenced source in generated schema-package README
        examples; use an SVG preview only when the source is binary, invalid
        UTF-8, or has no supported Markdown fence language.
- [ ] Establish XPath 3.1 as an independent schema package and typed expression
      AST stream before expanding XSLT expression-aware formatting.
  - [x] Add `xpath/v1` to the schema-package catalog with a conventional
        `package.cem`, `schema/xpath.cem`, manifest-owned fixtures, README,
        formatter/colorizer assets, preview verifier, and cacheable Nx project
        targets.
  - [x] Base XPath token boundaries on a maintained XPath 3.1 lexer while
        retaining whitespace, nested comments, exact lexemes, UTF-8 byte ranges,
        line/column coordinates, lexical errors, and source-map frames in a
        package-owned lossless token stream.
  - [x] Define a deterministic XPath AST event stream whose start/end lifecycle,
        token roles, delimiter nesting, and error facts can be consumed
        independently or fused into an owning transform stream without changing
        XPath grammar ownership.
  - [x] Define the host attachment envelope for standalone expressions, XML
        documents, and XML AST subtrees/attributes, preserving host schema,
        source identity, owner-node identity/range, expression range, namespace
        context, variable/function bindings, expected result contract,
        evaluation phase, and resolver/safety policy stamps.
  - [x] Bind decode, lexical, parse, static-context, unresolved namespace,
        external-resource, source-map, and host-association facts to
        `schema/xpath.cem` diagnostics; add fixtures for paths/axes, predicates,
        functions, variables, maps/arrays, Unicode QNames, strings, nested
        comments, malformed tokens, and incomplete delimiters.
  - [x] Route standalone XPath through the generic CEM-ML lifecycle as a typed
        `LoadedInputAstStream::XPathExpression`; validate primary, alias, and
        schema identities without CEM/XML fallback, and reject conversion until
        a typed XPath AST export adapter is registered.
  - [x] Define the schema-owned XPath evaluation request, evaluator capability,
        and ordered result-sequence artifact contracts for node, atomic, map,
        array, function, and mixed results. Preserve static context, node/source
        identity, typed lexical atomic values, evaluator-scoped function handles,
        resolver/safety policy stamps, and item-level source maps; keep the
        result media type out of the XPath source parser.
  - [x] Verify the official Xee GitHub source, license, architecture,
        conformance claims, XML ownership, and ambient resource behavior at a
        pinned commit; confirm it is suitable as a non-normative implementation
        reference but not as CEM's AST, evaluator, or security boundary.
  - [x] Accept full XPath 3.1 as the destination, delivered through staged
        conformance slices with a specification/QT3 gap matrix, stable
        unsupported-feature diagnostics, and per-file provenance for algorithms
        adapted from the pinned Xee source.
  - [x] Add a schema-owned, machine-readable CEM conformance matrix that pins
        the normative XPath/XDM/F&O/QT3 references, inventories every staged
        implementation slice, and requires an actionable gap for each slice not
        yet complete.
  - [x] Select a strongly typed W3C expression model as the primary CEM-owned
        XPath AST, retain the lossless token stream separately, and derive a
        start/end syntax event stream for XSLT, CEMT, and CEM-QL fusion rather
        than using a generic property-bag grammar tree.
  - [x] Replace the foreign `XPathSyntaxAst` payload with the first complete
        CEM-owned typed AST lowering slice for paths, predicates, variables,
        function calls, maps/arrays, source ranges, and host offsets. Permit the
        pinned Xee parser only as a temporary parser-local parity oracle; no Xee
        type or JSON projection may cross the syntax AST boundary.
  - [x] Replace the transitional `xee-xpath-lexer` and `xee-xpath-ast` runtime
        dependencies with CEM-owned XPath token and syntax AST types. Use the
        MIT-licensed Xee source pinned at commit `200b1e3356ea9d6dd2901d67bd941b779df7e5b7`
        only as a non-normative implementation reference, retain lexical/parser
        parity fixtures during migration, and record provenance for any adapted
        algorithm or copied substantial portion.
    - [x] Replace production lossless tokenization with a CEM-owned
          longest-match scanner, retain exact trivia and UTF-8 byte ranges, and
          verify lexical parity against the pinned Xee implementation.
    - [x] Replace the transitional Xee parser with a CEM-owned recursive-descent
          parser over CEM lexical tokens, then remove the remaining Xee runtime
          dependency after native, lint, and WASM gates pass.
      - [x] Add a package-owned token cursor, typed parse errors, and direct
            recursive-descent lowering with shadow AST parity coverage.
      - [x] Switch production parsing to the CEM parser, remove Xee parser
            runtime code/dependencies, and pass native, lint, and WASM gates.
  - [ ] After the strict native-AST transform boundary below is complete,
        implement a CEM-owned XPath 3.1 compiler and evaluator over the package
        AST. Treat the W3C XPath 3.1, XDM 3.1, and Functions and Operators 3.1
        specifications as normative; use Xee architecture and algorithms only
        as reference; target native and WASM; and route documents, collections,
        unparsed text, environment, time, randomness, recursion, cancellation,
        and work budgets through explicit CEM resolver/safety capabilities.
  - [ ] Wire the native evaluator through the `transform` command and expose
        explicit CEM-QL, CEMT, and XSLT invocation adapters without reparsing
        source text, constructing an evaluator-owned replacement XML tree, or
        projecting AST or result values through JSON.
  - [ ] Fuse parsed XPath streams into XSLT XPath-bearing attributes and AVT
        expression segments while retaining an independently addressable XPath
        AST associated with the owning XML event or subtree node.
  - [ ] Add deterministic compact/pretty/tabular and terminal/HTML/Markdown
        profiles that preserve lexical islands and source maps, then run package,
        converter-parity, CLI e2e, WASM, and core release gates.
- [x] Eliminate the implicit JSON transform data plane using Option C from
      `docs/transform-boundary-native-ast-decision.tmp.md`; this item is listed
      after XPath for roadmap grouping but must complete before XPath execution
      is registered.
  - [x] Add red tests for AST identity across lifecycle load and graph routing,
        duplicate JSON members, XML node/source identity, typed collection
        children, and rejection of implicit JSON projection.
    - [x] Prove JSON lexical/member identity and XML event/source identity are
          retained by `load_transform_data_artifact` as native lifecycle bodies.
    - [x] Add the dependency-neutral native, collection, extension, and encoded
          data-artifact contract without a generic JSON value variant.
    - [x] Route lifecycle load and collect joins through typed bodies, reject
          unmigrated adapter representations explicitly, and add a source audit.
    - [x] Pass focused transform tests, lint, native build/test, and WASM gates.
  - [x] Introduce a typed `TransformArtifactBody` with explicit native,
        collection, extension, and encoded variants; remove
        `serde_json::Value` from transform data and output artifact contracts.
  - [x] Retain `LoadedInputAstStream` or another package-owned native artifact
        through `load_transform_data_artifact`, graph routing, joins, and
        adapter dispatch instead of calling `projection::dom_json` or an
        equivalent serializer.
  - [x] Migrate CEM-QL, CEMT, and XSLT adapters to typed or lazy native AST
        views; remove generic `value_to_stream`, `to_cemt_subject`, and
        `to_json` tier ingress and keep JSON-to-query conversion only for
        explicitly identified JSON AST input.
    - [x] Add a CEM-QL native item-view contract whose field, member, atom,
          identity, and source-map accessors do not depend on JSON.
    - [x] Adapt native CEM documents, lifecycle JSON/XML ASTs, and typed graph
          collections to lazy CEM-QL items that retain owning `Arc` identity.
    - [x] Parse explicitly JSON-identified encoded artifacts through the JSON
          lifecycle AST before querying; remove `value_to_stream` ingress and
          add a source audit against generic JSON conversion.
    - [x] Restore native CEM-QL transform, expression, secondary-input, and
          graph behavior, then pass CEM-QL/core lint, test, and WASM gates.
    - [x] Replace the transitional CEMT value boundary with package-owned typed
          tree-envelope and writer-payload contracts.
      - [x] Add red tests for CEM tree `Arc` identity, formatter/colorizer
            metadata and source maps, ordered writer tokens/chunks, and explicit
            rejection of generic JSON-value ingress.
        - [x] Prove raw CEMT tree owner identity, lazy node access, source-map
              retention, and native formatter ingress without an encoded JSON
              artifact boundary.
        - [x] Prove formatted-envelope owner identity, ordered marker/decision
              operations, producer/profile metadata, source-map retention, and
              source-mapped-or-generated operation provenance.
        - [x] Prove stable owner paths and ordered per-node formatted overlays
              across nested nodes, attributes, preserved/elided source
              whitespace, and generated formatter fragments.
        - [x] Prove colored owner identity, generated color-operation
              provenance, and writer parity.
      - [x] Store text, byte, token, chunk, and diagnostic writer payloads as
            typed artifact variants; validate and compose them directly without
            constructor or writer-adapter JSON round trips.
      - [x] Introduce a package-owned raw CEMT tree artifact that retains the
            owning `Arc<CemTreeAstStream>` and exposes lazy node views; route
            native formatter ingress through it without an encoded JSON
            artifact boundary.
      - [x] Define and retain a typed formatted-envelope overlay over the raw
            owner, lower ordered scalar marker/decision records at adapter
            completion, and reject open object-valued decisions.
      - [x] Define structural root/child/attribute owner paths and typed
            per-node layout, boundary, attribute-spacing, close, and child-gap
            formatter operations; expose lazy owner-plus-overlay views without
            copying source AST nodes.
      - [x] Make CEMT/native output-function implementations return typed
            payloads directly; remove runtime-value classification and the
            remaining byte-encoder serialization at the evaluator boundary.
        - [x] Return native text, byte, token, chunk, and diagnostic results
              through a closed enum, validate the declared/returned kind at
              the producer boundary, and remove binary-encoder JSON
              serialization.
        - [x] Lower CEMT evaluator tree results immediately into package-owned
              typed artifacts; remove `CemtEvaluator(Value)` and
              `CemtRuntime(Value)` from cross-tier handoffs.
          - [x] Lower CEM-ML `cem.format-tree` and `cem.color-tree` results at
                adapter completion into owner-backed `CemtTreeArtifact`
                extension bodies; retain the evaluator value only as private
                scratch for the immediately following evaluator stage.
          - [x] Route the native CEM-ML writer directly from formatted and
                colored `CemtTreeArtifact` overlays, remove its
                `CemtRuntime(Value)` envelopes, and prove profile/source-map
                parity with the compatibility writer.
          - [x] Replace the native formatter/colorizer evaluator handoffs with
                lazy typed views, then remove their evaluator-value scratch
                and remaining `CemtRuntime(Value)` envelopes.
            - [x] Define a borrowed, zero-JSON evaluator view over raw
                  `CemTreeAstStream` nodes and attributes with typed scalar,
                  sequence, record, source-map, stable-path, lookup, and
                  iteration access.
            - [x] Extend the evaluator view across formatted envelope metadata,
                  owner-plus-overlay nodes, formatter operations, and generated
                  fragments without reconstructing a recursive tree.
            - [x] Migrate CEMT expression bindings and intermediates to the
                  typed evaluator value algebra, then route the native
                  formatter and colorizer through it.
              - [x] Define the owned evaluator algebra, typed binding/path
                    primitives, and persistent record/sequence overlays over
                    borrowed native records without JSON storage.
              - [x] Move runtime variable and parameter bindings plus path,
                    `exists`, `length`, and `get` evaluation onto the typed
                    algebra with independent compatibility-oracle parity.
              - [x] Move record/sequence literals, persistent `set`, `append`,
                    `extend`, and `merge`, edit constructors, and tree-patch
                    evaluation onto typed overlays with owner/source-map and
                    compatibility-oracle parity.
              - [x] Move typed function descriptors, adapter-lowered defaults,
                    `call`, `map`, `fold`, and `match` evaluation onto a typed
                    evaluator context with lexical-scope, owner/source-map,
                    contract-diagnostic, and compatibility-oracle parity.
              - [x] Move pure scalar, numeric, comparison, string, display,
                    conversion, and predicate helpers onto the typed evaluator
                    with nested-expression, Unicode/display-cell,
                    decimal-representation, and exact-diagnostic parity.
              - [x] Move functional stack/queue, source-map, diagnostic, and
                    metadata accumulator helpers onto persistent typed values
                    with owner-backed field removal, exact compatibility
                    diagnostics, and schema-expression closure audits.
              - [x] Pass focused, full, lint, native, WASM, CEMT fixture,
                    converter-parity, and CLI e2e gates for the native CEM
                    formatter/colorizer switch.
          - [x] Give non-CEM CEMT tree producers package-owned typed result
                artifacts, then remove `CemtEvaluator(Value)` globally.
            - [x] Inventory remaining producers, consumers, owner models,
                  stages, graph/secondary-input routing, and provenance gaps in
                  `docs/cemt-non-cem-typed-result-inventory.md`.
            - [x] Require direct borrowed evaluator views over the owning
                  package AST and direct construction of the result
                  `Arc<CemTreeAstStream>` with typed input provenance; prohibit
                  serializer, DTO, and re-parser boundaries between layers.
            - [x] Give materialized CEMT results a first-class
                  `TransformArtifactBody` variant so graph, join, and
                  secondary-input routing is exhaustive and preserves `Arc`
                  identity without extension downcasts.
            - [x] Introduce the recommended separate closed materialized-tree
                  artifact family after both ownership and graph decisions are
                  recorded.
            - [x] Add the serializer-free borrowed evaluator view over
                  `JsonDocumentAst`, `JsonValueAst`, ordered duplicate members,
                  exact number/string lexemes, source ranges, and source maps.
            - [x] Represent formatter-generated writer tokens as concrete
                  `CemTreeAstNode::WriterToken` nodes with typed role, style,
                  formatter metadata, source range/map, and output-span fields;
                  expose them through the borrowed evaluator view without a
                  `Value` projection.
              - [x] Add a validated materialized-tree color overlay keyed by
                    owner path, retaining the exact formatted
                    `Arc<CemTreeAstStream>` and rejecting non-token, duplicate,
                    producer-mismatched, role-mismatched, and output-mismatched
                    overlay entries.
              - [x] Route lossless `JsonDocumentAst` formatter execution
                    through the borrowed typed evaluator and lower its package
                    CEMT result directly into ordered `WriterToken` AST nodes.
              - [x] Pass the exact formatted JSON `Arc<CemTreeAstStream>` into
                    `json.color-document`, retain coloring only as the typed
                    owner-path overlay, and make the writer traverse the stream
                    plus overlay directly for plain, terminal, HTML, and
                    Markdown output.
              - [x] Remove `JsonDocumentAst::to_cemt_subject` from the JSON
                    production output path and add a source audit rejecting the
                    legacy stage executor, runtime `Value` artifact handoff,
                    composer, or compatibility-subject bridge between JSON
                    formatter, colorizer, and writer stages. The legacy public
                    tree projection is now created only after writer completion
                    for response/debug compatibility.
              - [x] Route a JSON-produced materialized body through an actual
                    graph stage, ordered join, and secondary-input edge, proving
                    both artifact and owner `Arc` identity rather than only the
                    general body-routing contract. The production JSON pipeline
                    now exposes the selected formatted/colored tree as a
                    `TransformTemplateOutputArtifact` whose body is the exact
                    `TransformArtifactBody::MaterializedCemtTree`; real graph
                    execution tests cover formatted-only and colored-overlay
                    routing through the stage, declared collection order, and
                    named secondary binding with outer artifact, materialized
                    artifact, and owner `Arc::ptr_eq` assertions.
              - [x] Replace the generic-data-to-JSON compatibility projection
                    with a borrowed/typed `GenericDataDocumentAst` evaluator
                    view so every production JSON entry path uses the
                    materialized pipeline. The view retains ordered and
                    duplicate mapping entries, generated/missing member names,
                    normalized JSON number lexemes, ranges, source maps, and
                    the original owner without constructing a
                    `JsonDocumentAst`, `Value`, DTO, or serialized document;
                    YAML scalar/sequence/mapping/missing-root/numeric and CSV
                    missing-name output cases plus source audits cover the
                    boundary.
              - [x] Migrate JSON Schema output to a borrowed
                    `JsonSchemaDocumentAst` evaluator that reuses the nested
                    lossless JSON view without cloning its owner. Source
                    metadata/parameters, dialect, parse facts, dialect facts,
                    ranges, maps, duplicate members, boolean schemas, and exact
                    lexemes remain typed; formatter and colorizer execution now
                    produce a materialized writer-token stream plus typed
                    owner-path overlay, the typed stage output retains that exact
                    artifact, and the direct writer consumes it without a
                    runtime `Value` handoff. The former JSON Schema AST
                    serializer is deleted, with its `Value` pipeline retained
                    only under `#[cfg(test)]` as a byte-parity oracle. Tabular
                    close-scope compaction now operates as a non-mutating typed
                    view over the materialized AST stream.
              - [x] Migrate native and generic-data CSV output to borrowed
                    evaluator views over `CsvDocumentAst` and
                    `GenericDataDocumentAst`. Both entry paths now preserve
                    exact field/number lexemes, row and field ordering, table
                    shape, ranges, maps, parse facts, missing cells, and the
                    original owner without a serialized CSV document or
                    runtime `Value` DTO. Formatter and colorizer stages exchange
                    the typed materialized writer-token stream and owner-path
                    overlay, the stage output retains that exact artifact, and
                    the direct writer handles plain, terminal, HTML, and
                    Markdown output. Production CSV subject composers are
                    test-only parity oracles, with source audits and native
                    same-schema conversion coverage enforcing the boundary.
              - [x] Migrate native and generic-data YAML output to borrowed
                    evaluator views over `YamlDocumentAst` and
                    `GenericDataDocumentAst`. Both paths now preserve stream
                    documents, duplicate mappings, exact scalar/number lexemes,
                    directives, comments, tags, anchors, aliases, ranges, maps,
                    and the original owner without a serialized YAML document
                    or runtime `Value` DTO. Formatter and colorizer stages
                    exchange the typed materialized writer-token stream and
                    owner-path overlay, the stage output retains that exact
                    artifact, and the direct writer handles plain, terminal,
                    HTML, and Markdown output. Production YAML subject
                    composers are test-only parity oracles; source audits and
                    native same-schema conversion coverage enforce the
                    boundary. Invalid unary `extend` expressions in the YAML
                    root scalar and alias formatter branches are also corrected.
              - [x] Migrate native Markdown output to a borrowed evaluator view
                    over `MarkdownDocumentAst`. The view preserves source and
                    encoding metadata, CommonMark/GFM variant and parse facts,
                    ordered events, optional event fields, ranges, maps, and
                    LF/CRLF policy without a runtime `Value` DTO. Formatter and
                    colorizer stages now exchange the typed materialized
                    writer-token stream and owner-path overlay, the stage output
                    retains the exact selected artifact, and the direct writer
                    handles plain, terminal, HTML, and Markdown output.
                    Production Markdown subject composers are test-only parity
                    oracles; source audits and native same-schema conversion
                    coverage enforce the boundary.
              - [x] Migrate the shared XML-family output boundary for
                    `XmlDocumentAst`, `HtmlDocumentAst`, `CssDocumentAst`,
                    `XhtmlDocumentAst`, `SvgDocumentAst`, `MathMlDocumentAst`,
                    and `XsltStylesheetAst` as one closed unit. A borrowed
                    `XmlFamilyDocumentCemtSubjectRef` retains each exact native
                    owner while reusing XML event components and preserving
                    HTML/CSS events, package facts, namespace/name data,
                    source/encoding metadata, SVG/MathML layout decisions and
                    markup tokens, ranges, maps, and line endings. All seven
                    formatter/colorizer/writer paths now exchange the typed
                    materialized writer-token stream and color overlay, retain
                    the selected artifact and owner `Arc` in the typed stage
                    body, and match the compatibility oracle. HTML, CSS,
                    XHTML, SVG, MathML, XSLT, and XML subject composers are now
                    test-only compatibility oracles.
              - [x] Migrate both XML and compact `RelaxNgDocumentAst` output
                    through `RelaxNgDocumentCemtSubjectRef`. The borrowed view
                    preserves syntax kind, source/media parameters, parse and
                    semantic facts, XML events or compact tokens, exact
                    ranges/maps, and line endings. Syntax-selected formatters
                    now materialize typed writer tokens (including typed
                    `syntaxKind` metadata), colorizers attach an owner-path
                    overlay to the same `Arc<CemTreeAstStream>`, the typed stage
                    body retains the selected artifact, and the direct writer
                    consumes it without a serialized DTO. RELAX NG and XML
                    subject composers are test-only parity oracles, enforced
                    by evaluator parity, `Arc::ptr_eq`, and source-audit tests.
              - [x] Close the native `DomProjectionParityCemtAdapter` branch.
                    It now borrows `CemtTreeSubjectRef`, retains the exact input
                    `Arc<CemTreeAstStream>` and node/attribute source maps,
                    builds typed retained-node and layout operations, returns
                    an owner-backed formatted `CemtTreeArtifact`, and enters
                    the colorizer/writer route without `Value`,
                    `CemtOutputArtifact`, or shape recovery. Converter parity
                    now exercises this typed producer; explicit JSON remains a
                    compatibility input only and never becomes the production
                    owner.
              - [x] Close the CEM-QL direct-output bridge with a package-owned
                    lexer-token AST exposed through core-owned borrowed
                    record/sequence views. The formatter now lowers directly
                    to an owned `Arc<CemTreeAstStream>`, coloring retains that
                    exact owner through a typed overlay (or is skipped for the
                    `none` profile), and the materialized writer consumes the
                    selected artifact without the former token-tree JSON DTO or
                    generic runtime pipeline. Native text, HTML, line-ending,
                    diagnostic, range/source-map identity, and source-audit
                    tests cover the boundary.
              - [x] Close the generic output-function runtime with a native or
                    typed CEM-tree result enum. Formatter and colorizer tree
                    results now lower at producer completion into declared
                    raw, formatted, colored, or materialized artifacts; the
                    exact typed artifact is passed to the next stage, and a
                    public-JSON producer is rejected as CEM-tree ingress.
              - [x] Delete `CemtOutputArtifact`,
                    `transform_template_output_cemt_subject`,
                    `CemtEvaluator(Value)`, `CemtRuntime(Value)`, the generic
                    stage fallback, and writer-boundary DTO assertions from
                    cross-tier execution. Typed tree overlays project to JSON
                    only at the explicit public/debug response boundary.
      - [x] Define typed raw, formatted, and colored CEM tree envelopes with
            ordered native nodes and lazy evaluator views over the owning AST.
      - [x] Route formatter, colorizer, writer, graph, and secondary-input
            boundaries through typed CEMT artifacts; remove
            `CemtOutputArtifact`, `transform_template_output_cemt_subject`, and
            their generic adapter DTO value conversion.
      - [x] Delete legacy `PublicJson` CEM-tree compatibility entrypoints,
            normalizers, runtime registry hooks, and value-based writer helpers;
            migrate or remove their test-only parity oracles while retaining
            `PublicJson` for actual JSON/public/debug boundaries. Formatted and
            colored continuation now accepts only typed CEMT artifacts, package
            parity fixtures enter through lifecycle ASTs, and the typed writer
            runs before the one-way public/debug projection. Source audits also
            forbid JSON projection or serialization across formatter,
            colorizer, writer, graph, join, and secondary-input handoffs.
      - [x] Add CEMT source audits and pass focused package, core, converter
            parity, CLI e2e, lint, native build/test, and WASM gates.
    - [x] Expose graph collections to CEMT as borrowed native record/sequence
          views over `TransformArtifactCollection`, retaining ordered item
          metadata, child AST identity, target identity, bindings, source maps,
          and output spans without a serializer or DTO boundary. CEM-QL uses
          the same canonical mode names and exposes `artifact` as the preferred
          child field while retaining `primary` as a compatibility alias.
  - [x] Represent JSON input internally with a lossless `JsonDocumentAst` and
        `JsonValueAst`, not `serde_json::Value`, preserving duplicate members,
        number lexemes, source ranges, diagnostics, and source maps.
    - [x] Classify the production JSON-route serde boundaries and record the
          remaining prohibited handoffs in
          `docs/json-route-serde-audit.md`. Migrate root `moduleMap` loading
          from `serde_json::Value` to one lifecycle parse plus direct ordered
          `JsonValueAst` traversal, preserving duplicate-member diagnostics,
          source positions, and last-declaration alias semantics. A source
          audit forbids serializer, public projection, or DTO conversion in
          both the loader and its AST traversal.
    - [x] Replace transform-template primary, secondary, and let JSON `Value`
          bindings with borrowed evaluator views over the owning lifecycle
          `JsonDocumentAst`/`JsonValueAst`. Encode-expression evaluation now
          uses the typed binding scope, CEM-tree inputs retain their native AST
          evaluator view, duplicate members and exact lexemes survive, and the
          data-input decoded JSON accessors plus production legacy evaluator
          are removed.
    - [x] Retain normalized transform-template parameters in an owner-backed
          typed binding scope without introducing another JSON-shaped DTO.
          The compiled artifact now owns a non-serializing typed parameter
          arena, adapters receive that arena instead of a `Value` map, render
          borrows its evaluator values before evaluating lets, and typed
          parameter identity participates in the module cache key.
    - [x] Remove the DOM-projection adapter's explicit JSON compatibility
          ingress and restrict decoded JSON output access to explicit public
          export boundaries. The adapter and its typed-envelope test now start
          from `CemTreeAstStream` directly.
  - [x] Make transform outputs typed native artifacts or explicit encoded
        artifacts, enforce encoding/content-type agreement, and require encoded
        input to pass through a lifecycle parser edge before AST consumption.
    - [x] Add red output-routing tests for native and encoded `Arc` identity,
          CEM-tree bodies, and rejection of value-shape classification.
    - [x] Replace `TransformTemplateOutputArtifact.value` with the shared typed
          body contract and transfer that body directly into graph artifacts.
    - [x] Return CEM-QL result sequences as adapter-owned native artifacts and
          register JSON encoding only for explicit JSON or `+json` exports.
    - [x] Migrate text, CEM-tree, conversion, and CEMT compatibility paths to
          typed output bodies without a generic JSON graph round trip.
    - [x] Add output-boundary source audits and pass focused, core, CLI, lint,
          native build/test, and WASM gates.
  - [x] Add source audits and native/WASM behavior gates proving that only
        registered JSON or `+json` conversion edges can serialize JSON and that
        graph collections preserve typed child order and identity.
  - [x] Register explicit DOM, event-stream, and XPath-result JSON exporters
        only after the native data-plane migration passes all gates.
- [ ] Polish XSLT formatter and colorizer profile semantics on the dedicated
      `XsltDocumentAst` path.
  - [ ] Reuse the shared typed XML-family markup-token helper while keeping
        XSLT layout and role policy package-local.
  - [ ] Characterize stylesheet/module namespaces, XPath attributes, attribute
        value templates, `xsl:text`, literal result elements, comments, CDATA,
        extension namespaces, and legacy custom-element syntax.
  - [ ] Define distinct deterministic `compact`, `pretty`, and `tabular`
        layouts without rewriting XPath, AVT, text, or foreign lexical islands.
  - [ ] Preserve token-level source maps, leave generated layout unmapped,
        honor formatter options, and verify terminal/HTML/Markdown parity.
  - [ ] Run the XSLT package, converter parity, CLI e2e, and core gates.
- [ ] cleanup completed items in todo.md

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

### Completed: Typed Evaluator-to-Encoder Boundary

- [x] Replace binding-request payload shape inference with typed evaluator
      kind, native representation, and inferred semantic type metadata.
- [x] Change registered host encoders to consume borrowed
      `CemtEvaluatorValue` directly and delete the `execute_typed` JSON
      compatibility projection.
- [x] Replace the evaluated encode response's decoded subject snapshot with
      non-payload subject metadata.
- [x] Make built-in encoders traverse typed AST/evaluator values directly.
- [x] Add source audits and behavior coverage proving duplicate JSON members,
      exact number/string lexemes, ranges, source maps, and native CEM-tree
      owner identity reach the selected encoder unchanged.

### Completed: Typed Compile-Parameter Ownership

- [x] Retain normalized parameters in the non-serializing
      `TransformTemplateParameterArena` owned by the compiled artifact.
- [x] Remove parameter `Value` maps from CEM-QL template/expression/XSLT
      adapter payloads. Those adapters enumerate arena names during compile
      and traverse borrowed evaluator values directly when constructing CEM-QL
      bindings or scalar XSLT entrypoint arguments.
- [x] Canonicalize selected-entrypoint aliases to their local binding names,
      copy defaults into the same owner, and bind parameters after data inputs
      but before typed lets.
- [x] Treat CLI/config-created values as generated evaluator values with no AST
      identity, source range, source map, or source lexeme. Strings retain
      decoded content and numbers retain their canonical typed value.
- [x] Include the arena representation and normalized typed values in module
      cache identity. Imported-module defaults and call arguments remain copied
      into their module-local owned evaluator values rather than borrowing a
      root parameter owner.

### Completed: Native Collection Evaluator Contract

- [x] Expose every collection as one borrowed record with `kind`, canonical
      `mode`, typed `count`, join `bindings`, and an ordered borrowed `items`
      sequence. Each item is a record with `inputName`, artifact identity/URI,
      destination, typed target identity, item bindings, direct borrowed child
      `artifact`, item source map, and ordered output spans.
- [x] Ratify the existing deterministic graph semantics as the public native
      contract: `collect` retains declared order; `group-by` requires keys,
      retains duplicates, and emits lexicographically ordered groups;
      `match-by` is a primary-domain grouped-left match that retains duplicates
      flat, permits unmatched primary keys, and discards secondary-only keys;
      and `zip` requires equal lengths and rejects mismatch.
- [x] Keep input aliases and join bindings as explicit record fields rather
      than ambient lexical variables. Collection, item-wrapper, and child AST
      identities remain distinct; child owners are never reconstructed, and
      source maps/output spans remain item-local rather than being falsely
      merged across sources.
- [x] Validate nested children recursively at CEMT binding time, reject encoded
      children until an explicit lifecycle parser edge creates their AST, and
      prove lossless JSON lexemes/duplicate members plus CEM-tree owner identity
      cross the collection boundary without serialization.

### Completed: JSON Serialization and Collection Boundary Gate

Close the remaining native-data-plane verification item with one auditable
allowlist and native/WASM behavior matrix:

- [x] Inventory every production `serde_json` encode/decode call reachable from
      transform load, graph routing, joins, template adapters, encoders, and
      exporters; classify each as an explicitly identified JSON/`+json`
      lifecycle or public/export boundary, and fail the source audit for any
      unregistered intermediate serialization.
- [x] Exercise `collect`, `group-by`, `match-by`, and `zip` through graph routing
      into CEMT and CEM-QL, asserting stable ordering/cardinality, collection
      and item metadata, child `Arc`/AST identity, provenance, strict zip
      mismatch, and rejection of encoded children without a parser edge.
- [x] Run focused adapter checks followed by CEM-QL/core lint, test, CLI e2e,
      converter parity, and native/WASM gates; then mark the parent source-audit
      checklist item complete.

The enforceable allowlist now distinguishes serializer-free native routes from
explicit lifecycle, registered-exporter, and public-export boundaries. The
CEM-QL cache policy and Markdown writer-token handoff no longer serialize
intermediate values. Four-mode CEMT/CEM-QL matrices prove ordering,
cardinality, native child ownership, target metadata, source maps, and output
spans survive direct graph routing.

### Completed: Explicit Native Projection Exporters

The remaining JSON projection edges are registered without reopening a generic
JSON transform data plane:

- [x] Define distinct registered exporter representations and target identities
      for DOM projections, event streams, and XPath results, including the
      exact JSON and `+json` media types each exporter accepts.
- [x] Make each exporter consume its borrowed native typed body directly and
      encode only after explicit target negotiation; do not add a generic
      `Value` fallback, shape classifier, or serializer between graph and
      exporter layers.
- [x] Preserve native owner, source-map, and output-span identity until the
      final encoding boundary. Add negative coverage for implicit JSON routes,
      mismatched non-JSON targets, and missing exporter registration.
- [x] Add focused representation/exporter tests and source-audit entries, then
      run CEM-QL/core lint and tests, CLI converter parity/e2e, and native/WASM
      gates before marking the exporter and parent migration items complete.

DOM projections, normalized event streams, and XPath results now have distinct
native body and representation identities. The default engine registry owns
their explicit exporters. DOM and event JSON is written by borrowed serializers
over the native owners, while XPath results are encoded directly from their
typed result owner. Each exporter accepts its vendor `+json` media type or
`application/json` only when paired with the matching schema; implicit targets,
non-JSON targets, schema mismatches, and absent registration are rejected.
Registry dispatch retains the exact body, source-map, and output-span references
through the final encoding call, and source audits prohibit a generic `Value`,
compatibility projection, serializer DTO, or fallback between graph and
exporter layers.

### Next Work Item — XSLT Formatter and Colorizer Profile Semantics

Polish the dedicated `XsltDocumentAst` output path without weakening its native
ownership or lexical-preservation guarantees:

- [ ] Characterize the current XSLT AST and package-owned CEMT path with red
      fixtures covering stylesheet/module namespaces, XPath-bearing
      attributes, AVTs, `xsl:text`, literal result elements, comments, CDATA,
      extension namespaces, and legacy custom-element syntax.
- [ ] Reuse the shared typed XML-family markup-token helper while keeping XSLT
      layout, role, and color policy package-local.
- [ ] Define deterministic `compact`, `pretty`, and `tabular` layouts that
      preserve XPath, AVT, text, and foreign lexical islands. Preserve
      token-level source maps and leave formatter-generated layout unmapped.
- [ ] Verify formatter options and plain, terminal, HTML, and Markdown parity,
      then run focused XSLT/package tests, core tests and lint, converter parity,
      CLI e2e, and native/WASM gates.

The first slice is characterization only. If existing package behavior and
fixtures do not determine a layout rule for an ambiguous XSLT construct, stop
at that decision point rather than inventing new profile semantics.

## Current Verification Commands

- `yarn nx run cem_ml:test:schema-package-structure`
- `yarn nx run cem_ml:test:cli-schema-artifacts`
- `yarn nx run cem_ml_cli:validate-cemt-pipeline-fixture`
- `yarn nx run cem_ml_cli:validate-converter-parity`
- `yarn nx run cem_ml_cli:e2e`
- `yarn nx run cem_ml:lint`
- `yarn nx run cem_ml:test`
- `yarn nx run cem_ml:build:wasm`
- `yarn nx run cem_ml_schema_package_json_v1:verify`
- `yarn nx run cem_ml_schema_package_json_schema_v1:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
