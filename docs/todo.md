# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Current active slice: remediate CSV format-support review findings against
[`packages/cem_ml/schema-packages/README.md`](../packages/cem_ml/schema-packages/README.md),
then resume dependency-ordered schema-package folder alignment.

### Completed Immediate Phase: CSV Formatter Review Findings

- [x] Document that CSV `pretty` and `tabular` are visual presentation formats:
      their alignment/trimming may produce strict-CSV deviations, permissive
      tools may recover by trimming leading/trailing field padding, and
      `compact` is required for non-visual data interchange.
- [x] Apply generic `lineEnding=lf|crlf|preserve` option to all CEMT formatter
      runtime bindings instead of only the CSV formatter path.

### Immediate: CSV Format-Support AC Remediation

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

### Schema Package Folder Alignment

Remaining dependency-ordered package checklist:
- [ ] `cem-ql/v1`
- [ ] `cem-native-template/v1`
- [ ] `cem-transform/v1`
- [ ] `cem-ast-projection/v1`
- [ ] `cem-events-projection/v1`
- [ ] `cem-dom-projection/v1`
- [ ] `csv/v1`
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

Resume dependency-ordered schema-package folder alignment with `cem-ql/v1`.
Review it against the common schema-package AC, align package metadata,
schema/assets/examples/docs/tests as needed, and run the smallest package-level
verification gate. After `cem-ql/v1` is green, continue with
`cem-native-template/v1`.

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
