# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Implement schema-owned field contracts for every schema-declared field
      before adding more package-specific Rust validation branches. The current
      implementation compiles `required-attributes`, `optional-attributes`,
      child allow-lists, initial field contracts, and exact-one child
      occurrence from `.cem` in `packages/cem_ml/src/schema/document_model.rs`;
      remaining schema-package converter/artifact field rules and example
      schema/content-type cross-reference execution still live in
      `packages/cem_ml/src/validation/rules.rs`, and manifest descriptor
      loading still has Rust-owned `required_attr` checks in
      `packages/cem_ml/src/schema/registry.rs`.
  - [ ] Add failing tests first for the principle in `document_model.rs`,
        `rules.rs`, and the CLI schema examples: changing a field contract in a
        `.cem` schema must change validation behavior without adding or editing
        a package-specific Rust branch.
  - [ ] Expand the initial `field-contracts` vocabulary in
        `packages/cem_ml/schema-packages/schema/v1/schema/cem-schema.cem`.
        The schema language now models element-bound required/optional/
        forbidden fields, value and presence conditional selectors,
        value-specific forbidden fields, exact-one child occurrence contracts,
        package-relative path-layout contracts, diagnostic families, and
        attribute `@values` vocabularies; it still needs accepted children,
        scalar type validation beyond boolean/integer syntax, RELAX NG-style
        datatype params beyond `minInclusive` such as `maxInclusive` and
        `pattern`, defaults, richer dependent-required field groups, RELAX
        NG-style choice/case groups, and broader child occurrence ranges.
  - [ ] Extend the compiled Rust schema contract model. `SchemaDocumentModel`
        now compiles initial `field-contract` declarations and evaluates
        required/forbidden fields, attribute `@values` vocabularies, and
        `schema:boolean`/`cemml:boolean` plus `schema:integer`/
        `cemml:integer` attribute types; it still needs reusable string/path/
        URI/media-type constraints beyond path-layout checks, RELAX NG-style
        datatype params beyond integer `minInclusive`, dependent field groups
        beyond presence-gated required fields, RELAX NG-style choice/case
        groups, broader child occurrence ranges, defaults, and richer case
        grouping for all schema elements.
  - [ ] Extend structured diagnostic details beyond initial required/forbidden
        field checks. The first generic field-contract evaluator now emits
        schema URI, element, contract name, check kind, required/optional/
        forbidden fields, missing/invalid fields, actual values, condition, and
        source-map range, and attribute `@values` checks emit expected/actual
        value details; boolean and integer type checks now emit expected/
        actual details; integer `minInclusive` checks emit datatype-param
        details; value-specific forbidden checks emit forbiddenAttributeValues
        and invalidValues details; child occurrence checks emit required/
        max-one children, missing/duplicate children, and childCounts details;
        path-layout checks emit pathLayout and invalidValues details; string/
        path/URI/media-type validation beyond path-layout, datatype params
        beyond `minInclusive`, dependency, choice/case grouping, broader child
        occurrence ranges, and cross-reference checks need the same
        schema-owned detail shape.
  - [ ] Extend the generic field-contract evaluator. The first evaluator runs
        from schema URI plus content type, consumes the compiled contract
        model, preserves source-map ranges, and emits contract-declared
        diagnostic families such as `cem.schema_package.artifact_check`; it
        now emits structured details for required/forbidden field checks and
        attribute `@values` plus boolean/integer type and integer
        `minInclusive` datatype-param checks, exact-one child occurrence
        checks, and package-relative path-layout checks, and still needs
        coverage for string/path/URI/media-type validation beyond path-layout,
        datatype params beyond `minInclusive`, dependency, RELAX NG-style
        choice/case grouping, and broader child occurrence ranges.
  - [ ] Move schema-package manifest field rules from Rust conditionals into
        `packages/cem_ml/schema-packages/schema-package/v1/schema/schema-package.cem`.
        Cover `package`, `schema`, `content-type`, `namespace`, `converter`,
        `from`, `to`, `parity-fixture`, `artifact`, and `example`.
        `content-type` child `value`, `namespace` child `uri`, and
        content-type `primary` boolean validation now stay in schema-owned
        validation instead of registry descriptor extraction; `parity-fixture`
        `id`/`path` requiredness now stays in schema-owned validation instead
        of conversion descriptor extraction; artifact `kind`/`path`
        requiredness now stays in schema-owned validation instead of
        conversion artifact descriptor extraction; converter `id`/
        `implementation`, implementation value, endpoint occurrence, and
        endpoint `content-type` requiredness now stay in schema-owned
        validation instead of conversion descriptor extraction; top-level
        package `id`/`version` and schema `uri`/`source` descriptor fields now
        use schema-root/catalog fallbacks so missing manifest field diagnostics
        stay schema-owned; converter implementation value validation and CEMT
        template content-type/schema exact values now stay schema-owned through
        the generic `@values` vocabulary instead of package-specific runtime
        branches.
  - [ ] Model converter cases in `schema-package.cem`: `implementation=cemt`
        and `implementation=rust` required attribute contracts plus CEMT
        native fallback `fallback-reason` now live in schema-owned
        `field-contract` declarations and emit
        `cem.schema_package.converter_check`; `from`/`to` exact-one endpoint
        occurrence is now schema-owned; enum fields now use schema-declared
        `@values` and boolean fields now use `schema:boolean` in the generic
        document model; `cost` now uses generic integer syntax and RELAX
        NG-style `minInclusive=1`; `implicit=true` with `explicit-only=true`
        is now a schema-owned value-specific forbidden field contract; CEMT
        template `template-content-type` and `template-schema` exact values now
        use schema-declared `@values`; package-specific enum, boolean, cost,
        fallback-reason, planner-state, endpoint cardinality, and CEMT template
        identity diagnostics have been retired in favor of generic
        schema-owned codes; `parity-fixture` `id`/`path` extraction now skips
        incomplete schema-invalid rows and materializes only complete runtime
        fixture descriptors; converter descriptor extraction now skips
        incomplete schema-invalid rows missing `id`, `implementation`, known
        implementation value, `from`/`to`, or endpoint `content-type`.
  - [ ] Finish artifact cases in `schema-package.cem`. Formatter, colorizer,
        formatter-helper, and colorizer-helper required field metadata now
        lives in schema-owned `field-contract` declarations; formatter/
        colorizer stage directory and `.cemt` source-path layout now live in
        schema-owned path-layout contracts; CEMT output function target
        identity/category/profile mismatches now emit the schema-declared
        `cem.schema_package.artifact_check` family with
        `artifact-output-stage-contract` details; CEMT source readability,
        parser validity, and function lookup now also report through
        schema-declared `artifact-output-stage-contract` details while keeping
        Rust only as the execution placement; runtime conversion artifact
        extraction no longer owns artifact `kind`/`path`,
        formatter/colorizer profile required-field checks, or the `generated`
        boolean datatype check; runtime conversion selection now routes
        formatter/colorizer stage-kind-to-function-kind mapping through the
        schema-pinned CEMT stage contract groups. Remaining artifact work is to
        audit deeper CEMT body/output assertions once the schema vocabulary for
        those assertions exists.
  - [ ] Model example cases in `schema-package.cem`: required example
        metadata and failing-example `expected-diagnostics` now live in
        schema-owned `field-contract` declarations and emit
        `cem.schema_package.example_check`; content type/schema compatibility
        is declared as a schema-owned cross-reference rule and still executes
        in Rust, now emitting `example_check` with structured `checkKind`
        details; package example descriptor extraction no longer owns required
        metadata or `expected-result` value diagnostics, and the stale
        `example_content_type_mismatch` diagnostic has been retired.
  - [ ] Continue replacing one-code-per-field diagnostics with contract-family
        diagnostics declared in schema source. Artifact missing-metadata checks
        now emit `cem.schema_package.artifact_check`; converter and example
        field diagnostics now emit schema-declared `converter_check` and
        `example_check` families where the distinction is field/cross-reference
        contract detail; artifact CEMT output function metadata mismatches now
        also use `cem.schema_package.artifact_check` with contract details
        instead of a narrow mismatch code, and artifact CEMT source read, parse,
        and function lookup failures now use the same generic family with
        operational `checkKind` details; CEMT native fallback reason
        requirements now stay in schema-owned `converter_check` contracts
        instead of conversion descriptor extraction; Rust converter
        `rust-symbol` requirements now also stay in schema-owned
        `converter_check` contracts, with runtime execution retaining only the
        operational missing-symbol guard; CEMT converter `template`,
        `template-content-type`, and `template-schema` identity requirements
        now stay in schema-owned `converter-cemt-template-identity` and
        `@values` declarations, with runtime execution retaining only
        operational template source read and compile checks; converter endpoint
        schema/content-type compatibility and converter template source/compile
        failures now emit the generic `converter_check` family with structured
        `checkKind` details; converter scalar datatype/value checks for
        readiness, streamability, implicit/explicit flags, cost, output syntax,
        and parity now stay in generic schema-model validation instead of
        descriptor extraction; parity fixture `id`/`path` field checks now stay
        in generic schema-model validation instead of descriptor extraction; artifact
        `kind`/`path` field checks now stay in generic schema-model validation
        instead of descriptor extraction; converter `id`/`implementation`,
        implementation value, endpoint occurrence, and endpoint `content-type`
        field checks now stay in generic schema-model validation instead of
        descriptor extraction or schema-package validator branches; stale
        converter manifest field-specific `MissingAttribute`,
        `MissingEndpoint`, `UnknownImplementation`, and
        `converter_implementation_unknown` errors have been removed; schema
        descriptor top-level `MissingAttribute` extraction errors have been
        removed for package/schema metadata; stale converter template and
        endpoint-specific diagnostic declarations have been retired in favor of
        `cem.schema_package.converter_check`.
  - [ ] Keep Rust validators only for operational execution that cannot be
        represented as field data: resource read failures, parser failures,
        CEMT compilation, CEMT function lookup, host-hook availability, and
        source-file I/O. Those checks must still be declared as schema-owned
        constraints/rules in `.cem`, with Rust only as the execution placement.
  - [ ] Refactor `SchemaPackageConverterContractRule` toward operational-only
        checks for template readability/compilation and endpoint
        schema/content-type compatibility. CEMT template identity, Rust symbol,
        CEMT fallback reason requirements, converter planner-state conflicts,
        and converter endpoint occurrence are now schema-owned field contracts;
        the legacy package-specific enum, boolean, positive-cost,
        planner-state, and endpoint cardinality branches are now covered by
        generic `@values`, `schema:boolean`, `minInclusive`,
        value-specific forbidden field, and child occurrence checks; CEMT
        template content-type/schema exact values are now covered by generic
        `@values`, and the remaining converter operational/cross-reference
        branches emit `converter_check` with `checkKind` details.
  - [ ] Refactor `schema_descriptor_from_package_sources`,
        `collect_package_examples`, and `required_attr` in
        `packages/cem_ml/src/schema/registry.rs` so descriptor extraction runs
        after generic schema validation. Loader errors may remain typed, but
        missing/invalid manifest fields must be diagnosed by schema-owned
        contracts, not by descriptor parsing. `collect_package_examples` now
        treats invalid example field data as schema-owned and only materializes
        complete loadable example descriptors; content-type and namespace
        child extraction now skips incomplete schema-invalid rows instead of
        owning field diagnostics; top-level package/schema descriptor
        extraction now falls back to the embedded package id, schema root
        namespace/version, and embedded schema path instead of owning missing
        manifest field diagnostics, and `required_attr` has been removed.
  - [ ] Update runtime diagnostic declaration tests and CLI example coverage to
        assert generic contract-family codes plus structured details instead of
        schema-package-specific field diagnostics. A production-source audit
        now guards against reintroducing schema-package field-specific
        `*_missing`, `*_unknown`, `*_invalid`, `*_duplicate`, and `*_conflict`
        diagnostics except explicitly allowlisted operational execution
        failures, plus retired converter scalar/value/cardinality diagnostic
        names, hard-coded required-field vector helper names, and retired
        descriptor parsing helpers/errors; schema-declared enum value sets for
        conversion manifest materializers are now parity-tested against the
        Rust enum parsers, and formatter/colorizer artifact stage kind groups
        plus runtime output-function-kind mappings are parity-tested against
        schema-owned field-contract `@when-values`.
        Formatter/colorizer CEMT body metadata term checks are centralized on
        the operational stage contract type and test-pinned; moving those terms
        into schema needs a schema vocabulary for CEMT body/output assertions.
        Keep expanding that audit for narrow operational `matches!` lists as
        new declarative contracts move into schema.

- [ ] Complete the schema-package folder frame for
      `packages/cem_ml/schema-packages`: every `{package-id}/vN/` folder must be
      discoverable from `package.cem` with a `.cem` schema source, example
      references, CEMT formatter artifacts, and CEMT colorizer artifacts.
  - [ ] Extend the schema-package manifest and validators so package examples
        and formatter/colorizer artifacts are declared from `package.cem`.
        Examples must include source path, content type, schema URL, expected
        pass/fail result, and expected diagnostics. Artifacts must include
        profile, target content type/schema, target category, and CEMT function
        identity.
  - [x] Add package-folder validation that checks `package.cem`, `schema/`,
        `examples/`, `formatters/`, and `colorizers/` completeness for every
        built-in package before per-package implementation can be marked done.
        Build-time catalog checks now require every embedded built-in package
        to have matching `package.cem`, `.cem` schema source, and at least one
        example fixture; declared CEMT artifacts must exist on disk and be
        embedded in the artifact source catalog; any embedded package that
        declares `formatters/` or `colorizers/` must have those CEMT assets
        indexed by `package.cem`, and any declared output-stage profile set
        must include baseline formatter profiles `compact`, `pretty`,
        `tabular` or colorizer profiles `terminal`, `html`, and `md`.
  - [x] Require example loading to resolve the declared content type plus schema
        URL and validate the source bytes against that schema; filename
        extension inference is only a fallback hint. Declared schema-package
        examples are now read from their manifest-relative `@path`, parsed by
        declared content type/schema instead of extension, validated against
        the built-in document model when available, and checked against
        `expected-result` plus `expected-diagnostics`.
  - [x] Expand example coverage from representative constraint-kind coverage to
        finer diagnostic coverage, starting with schema-package source
        read/invalid cases and artifact source/parse/function-missing cases.
        Schema-package example fixtures now cover unreadable example sources,
        loaded example result mismatches, expected diagnostic mismatches,
        unreadable artifact CEMT sources, invalid artifact CEMT sources, and
        missing artifact CEMT function declarations through generic
        `cem.schema_package.example_check` and
        `cem.schema_package.artifact_check` diagnostics.
  - [x] Implement reusable baseline formatter profiles:
        `compact` as default, `pretty`, and `tabular`; each profile is a CEMT
        transform that preserves source-map ranges.
  - [x] Implement reusable baseline colorizer profiles: `terminal`, `html`,
        and `md`; each profile is a CEMT transform over the formatted CEM tree
        with source-map range preservation.
    - [x] Preserve literal CEM-tree colorizer profile selectors `terminal`,
          `html`, and `md` through transform binding, package-stage execution,
          generated color metadata, and writer artifact identity.
    - [x] Materialize `colorOutput`, terminal `colorCapability`, and
          profile-specific per-node style metadata from the CEMT colorizer
          helper while keeping HTML writer attributes restricted to `html`.
    - [x] Add terminal writer rendering for capability-aware ANSI/SGR output
          from colored CEM-tree ranges.
    - [x] Add Markdown-safe rendered color output forms for `md` without
          losing source-map ranges.
  - [ ] Roll the frame through the supported package scope below in order,
        keeping every content type covered before moving to lower-priority
        package families.

## Schema Package Frame Scope

Complete each supported package below only when the generic folder frame in
Immediate Tasks is satisfied for that package: `package.cem`, `.cem` schema,
explicit example content-type/schema references, `compact`/`pretty`/`tabular`
CEMT formatters, `terminal`/`html`/`md` CEMT colorizers, and package-folder
validation coverage. The order is dependency-first, then common authoring
formats, then XML/markup families, then projection/debug formats.

Bootstrap and self-hosting packages:

- [x] `cem-ml/v1` (`application/cem`; aliases: `text/cem-ml`, `text/cem`,
      `application/cem+xml`). Baseline formatter/colorizer selectors are
      declared and runtime-selectable over canonical CEMT assets; terminal and
      Markdown rendered color output preserve source-map ranges. Top-level
      examples are declared from `package.cem` with source path, content type,
      schema URL, expected result, and expected diagnostics, and catalog tests
      guard CEM-ML example manifest drift.
- [x] `schema/v1` (`application/vnd.cem.schema+cem`). Top-level schema
      examples are declared from `package.cem` with source path, content type,
      schema URL, expected result, and expected diagnostics, and catalog tests
      guard schema example manifest drift.
- [x] `schema-package/v1` (`application/vnd.cem.schema-package+cem`).
      Top-level schema-package examples are declared from `package.cem` with
      source path, content type, schema URL, expected result, and expected
      diagnostics, and catalog tests guard schema-package example manifest
      drift.
- [x] `cem-native-template/v1` (`application/vnd.cem.template+cem`; CEM
      source aliases). Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level examples are declared from `package.cem`, and
      catalog tests guard native-template artifact/example manifest drift.
- [x] `cem-transform/v1` (`application/vnd.cem.transform+cem`, `.cemt`).
      Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level examples are declared from `package.cem`, including
      the paired CEM fixture with its own CEM-ML identity, and catalog tests
      guard transform artifact/example manifest drift.
- [x] `cem-ql/v1` (`application/vnd.cem.query+cem-ql`, `text/cem-ql`, query
      artifact aliases). Baseline `compact`/`pretty`/`tabular` formatter and
      `terminal`/`html`/`md` colorizer selectors are declared over CEMT
      artifacts, top-level CEM-QL examples are declared from `package.cem`,
      and catalog tests guard CEM-QL artifact/example manifest drift.

Common structured and authoring formats:

- [x] `json/v1` (`application/json`, `text/json`). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level JSON
      examples are declared from `package.cem`, and catalog tests guard JSON
      artifact/example manifest drift.
- [x] `json-schema/v1` (`application/schema+json`). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level JSON
      Schema examples are declared from `package.cem`, and catalog tests guard
      JSON Schema artifact/example manifest drift.
- [x] `yaml/v1` (`application/yaml`, YAML aliases). Baseline
      `compact`/`pretty`/`tabular` formatter and `terminal`/`html`/`md`
      colorizer selectors are declared over CEMT artifacts, top-level YAML
      examples are declared from `package.cem`, including alias content types,
      and catalog tests guard YAML artifact/example manifest drift.
- [x] `csv/v1` (`text/csv`). Baseline `compact`/`pretty`/`tabular`
      formatter and `terminal`/`html`/`md` colorizer selectors are declared
      over CEMT artifacts, top-level CSV examples are declared from
      `package.cem`, including pass-with-warning diagnostics, and catalog tests
      guard CSV artifact/example manifest drift.
- [ ] `markdown/v1` (`text/markdown`).
- [ ] `css/v1` (`text/css`).

XML and markup family formats:

- [ ] `xml/v1` (`application/xml`, XML aliases).
- [ ] `html/v1` (`text/html`).
- [ ] `relax-ng/v1` (`application/relax-ng+xml`,
      `application/relax-ng-compact-syntax`).
- [ ] `xhtml/v1` (`application/xhtml+xml`).
- [ ] `svg/v1` (`image/svg+xml`).
- [ ] `mathml/v1` (`application/mathml+xml`, MathML aliases).
- [ ] `xslt/v1` (`application/xslt+xml`, XSLT aliases).

Projection and debug/interchange formats:

- [ ] `cem-dom-projection/v1` (`application/vnd.cem.dom+cem-bin`,
      `application/vnd.cem.dom+json`).
- [ ] `cem-ast-projection/v1` (`application/vnd.cem.ast+cem-bin`,
      `application/vnd.cem.ast+json`).
- [ ] `cem-events-projection/v1` (`application/vnd.cem.events+cem-bin`,
      `application/vnd.cem.events+json`).

# [] believes schema + registry
stop for sync up with author
## Current Verification Commands

- `yarn nx run @epa-wg/cem-theme:verify:phase13`
- `yarn nx run cem-elements:verify`
- `yarn nx run @epa-wg/cem-components:verify`
- `yarn nx run cem-elements:verify-edge-ssr`
- `yarn nx run @epa-wg/custom-element:verify`

## Externally Gated

These are intentionally not active in the current workspace because the required native toolchains are unavailable.
Keep the existing offline platform artifact validation as the release gate until supported native CI exists.

- Swift/Xcode compile gate for `packages/cem-theme/dist/lib/token-platforms/ios/CEMTokens.swift`.
- Kotlin/Compose Gradle compile gate for `packages/cem-theme/dist/lib/token-platforms/android/`.
