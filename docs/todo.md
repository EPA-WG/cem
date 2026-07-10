# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in [`wishlist.md`](wishlist.md). Completed implementation
history belongs in git history and the feature-specific docs linked below.

## Immediate Tasks

- [ ] Implement schema-owned field contracts for every schema-declared field
      before adding more package-specific Rust validation branches. The current
      implementation only compiles simple `required-attributes`,
      `optional-attributes`, and `children` from `.cem` in
      `packages/cem_ml/src/schema/document_model.rs`; schema-package
      converter, artifact, and example field rules still live in
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
        forbidden fields, conditional selectors, diagnostic families, and
        attribute `@values` vocabularies; it still needs accepted children,
        scalar type validation beyond boolean/integer syntax, RELAX NG-style
        datatype params beyond `minInclusive` such as `maxInclusive` and
        `pattern`, defaults, dependent-required fields, mutually exclusive
        fields, conditional case groups, and cardinality.
  - [ ] Extend the compiled Rust schema contract model. `SchemaDocumentModel`
        now compiles initial `field-contract` declarations and evaluates
        required/forbidden fields, attribute `@values` vocabularies, and
        `schema:boolean`/`cemml:boolean` plus `schema:integer`/
        `cemml:integer` attribute types; it still needs reusable string/path/
        URI/media-type constraints, RELAX NG-style datatype params beyond
        integer `minInclusive`, dependent fields, mutual exclusion,
        cardinality, defaults, and richer case grouping for all schema
        elements.
  - [ ] Extend structured diagnostic details beyond initial required/forbidden
        field checks. The first generic field-contract evaluator now emits
        schema URI, element, contract name, check kind, required/optional/
        forbidden fields, missing/invalid fields, actual values, condition, and
        source-map range, and attribute `@values` checks emit expected/actual
        value details; boolean and integer type checks now emit expected/
        actual details; integer `minInclusive` checks emit datatype-param
        details; string/path/URI/media-type, datatype params beyond
        `minInclusive`, dependency, mutual-exclusion, cardinality, and
        cross-reference checks need the same schema-owned detail shape.
  - [ ] Extend the generic field-contract evaluator. The first evaluator runs
        from schema URI plus content type, consumes the compiled contract
        model, preserves source-map ranges, and emits contract-declared
        diagnostic families such as `cem.schema_package.artifact_check`; it
        now emits structured details for required/forbidden field checks and
        attribute `@values` plus boolean/integer type and integer
        `minInclusive` datatype-param checks, and still needs coverage for
        string/path/URI/media-type, datatype params beyond `minInclusive`,
        dependency, mutual-exclusion, and cardinality checks.
  - [ ] Move schema-package manifest field rules from Rust conditionals into
        `packages/cem_ml/schema-packages/schema-package/v1/schema/schema-package.cem`.
        Cover `package`, `schema`, `content-type`, `namespace`, `converter`,
        `from`, `to`, `parity-fixture`, `artifact`, and `example`.
  - [ ] Model converter cases in `schema-package.cem`: `implementation=cemt`
        requires CEMT template identity fields; `implementation=rust` requires
        `rust-symbol`; CEMT native fallback requires `fallback-reason`;
        `from`/`to` endpoint cardinality is one each; enum fields now use
        schema-declared `@values` and boolean fields now use `schema:boolean`
        in the generic document model; `cost` now uses generic integer syntax
        and RELAX NG-style `minInclusive=1`; package-specific boolean/cost
        diagnostics have been retired in favor of generic schema-model codes;
        `implicit` and `explicit-only` are mutually exclusive.
  - [ ] Finish artifact cases in `schema-package.cem`. Formatter, colorizer,
        formatter-helper, and colorizer-helper required field metadata now
        lives in schema-owned `field-contract` declarations; stage directory,
        `.cemt` source-path, target identity compatibility, target category,
        function profile, and formatter/colorizer profile consistency still
        need schema-owned field/rule declarations instead of Rust conditionals.
  - [ ] Model example cases in `schema-package.cem`: examples require `id`,
        `path`, `content-type`, `schema`, and `expected-result`; failing
        examples require `expected-diagnostics`; content type/schema
        compatibility is declared as a schema-owned cross-reference rule.
  - [ ] Continue replacing one-code-per-field diagnostics with contract-family
        diagnostics declared in schema source. Artifact missing-metadata checks
        now emit `cem.schema_package.artifact_check`; next consolidate
        converter and example field diagnostics into schema-declared
        `converter_check` and `example_check` families where the only
        distinction is field contract detail.
  - [ ] Keep Rust validators only for operational execution that cannot be
        represented as field data: resource read failures, parser failures,
        CEMT compilation, CEMT function lookup, host-hook availability, and
        source-file I/O. Those checks must still be declared as schema-owned
        constraints/rules in `.cem`, with Rust only as the execution placement.
  - [ ] Refactor `SchemaPackageConverterContractRule` so it calls the generic
        field-contract evaluator before operational checks, then removes the
        Rust-owned lists and match blocks for required fields, enum values,
        dependent fields, and mutual exclusion. The legacy package-specific
        boolean and positive-cost branches are now covered by generic
        `schema:boolean` and `minInclusive` checks.
  - [ ] Refactor `schema_descriptor_from_package_sources`,
        `collect_package_examples`, and `required_attr` in
        `packages/cem_ml/src/schema/registry.rs` so descriptor extraction runs
        after generic schema validation. Loader errors may remain typed, but
        missing/invalid manifest fields must be diagnosed by schema-owned
        contracts, not by descriptor parsing.
  - [ ] Update runtime diagnostic declaration tests and CLI example coverage to
        assert generic contract-family codes plus structured details instead of
        schema-package-specific missing-field codes. Add an `rg`-based audit in
        tests or docs for remaining field-rule anti-patterns such as
        hard-coded required field vectors, enum `matches!` lists, and
        field-specific `*_missing` diagnostics.

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
  - [ ] Add package-folder validation that checks `package.cem`, `schema/`,
        `examples/`, `formatters/`, and `colorizers/` completeness for every
        built-in package before per-package implementation can be marked done.
  - [ ] Require example loading to resolve the declared content type plus schema
        URL and validate the source bytes against that schema; filename
        extension inference is only a fallback hint.
  - [ ] Expand example coverage from representative constraint-kind coverage to
        finer diagnostic coverage, starting with schema-package source
        read/invalid cases and artifact source/parse/function-missing cases.
  - [ ] Implement reusable baseline formatter profiles:
        `compact` as default, `pretty`, and `tabular`; each profile is a CEMT
        transform that preserves source-map ranges.
  - [ ] Implement reusable baseline colorizer profiles: `terminal`, `html`,
        and `md`; each profile is a CEMT transform over the formatted CEM tree
        with source-map range preservation.
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

- [ ] `cem-ml/v1` (`application/cem`; aliases: `text/cem-ml`, `text/cem`,
      `application/cem+xml`).
- [ ] `schema/v1` (`application/vnd.cem.schema+cem`).
- [ ] `schema-package/v1` (`application/vnd.cem.schema-package+cem`).
- [ ] `cem-native-template/v1` (`application/vnd.cem.template+cem`; CEM source
      aliases).
- [ ] `cem-transform/v1` (`application/vnd.cem.transform+cem`, `.cemt`).
- [ ] `cem-ql/v1` (`application/vnd.cem.query+cem-ql`, `text/cem-ql`, query
      artifact aliases).

Common structured and authoring formats:

- [ ] `json/v1` (`application/json`, `text/json`).
- [ ] `json-schema/v1` (`application/schema+json`).
- [ ] `yaml/v1` (`application/yaml`, YAML aliases).
- [ ] `csv/v1` (`text/csv`).
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
