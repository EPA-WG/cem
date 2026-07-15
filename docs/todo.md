# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Low Priority Deferred Design

Current cross-node/reference background: schema-owned constraints are declared
in CEM-ML but some are still executed by Rust. For example,
`schema-package.cem` declares converter endpoint compatibility as
`{constraint @kind="endpoint-content-type-schema" @target="from to"
@diagnostic="cem.schema_package.converter_check"
@behavior="schema:reference-resolution" ...}`, while
`SchemaPackageConverterContractRule` still reads each endpoint's `@schema`,
resolves that URI through `SchemaRegistry`, and checks whether
`content_type_essence(@content-type)` is included in the referenced schema's
registered content-type essences. Similar Rust-executed reference-resolution
shapes exist for example content-type/schema compatibility and artifact CEMT
function lookup/metadata matching.

- [x] Design normalized value vocabulary for schema-owned reference
      constraints. The decision is recorded in
      [`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md):
      normalizers now have named vocabulary terms, stable output/state shape,
      pure versus engine-assisted placement, and coverage for media-type
      essence, schema/document URI, namespace URI, content category, profile
      name, artifact/function name, and exact scalar values.
- [ ] Design comparison vocabulary for normalized reference values. Cover exact
      equality, membership in a referenced registry set, required/forbidden set
      overlap, optional profile/category matching, and missing/unresolved
      reference behavior. Specify expected/invalid value detail projection and
      source-range ownership for each comparison form.
- [ ] Design the remaining richer declarative cross-node/reference vocabulary
      for schema-owned constraints currently declared in CEM-ML but executed by
      Rust. The future vocabulary should let a schema declare candidate
      selection, reference attributes, registry/document lookup target,
      expected/invalid value detail projection, source-range propagation, and
      whether execution is pure CEM-ML/CEM-QL or engine-assisted. Reuse
      `schema:reference-resolution`, CEM-QL candidate selection, and
      schema-declared behavior functions where possible; avoid introducing
      package-specific syntax for converter endpoints.
- [ ] Design any further generic path-layout vocabulary beyond the current
      prefix, directory-name allow/forbid, extension, and basename allow/forbid
      facets. Background: `path` is always resolved in scope context, not
      document context. `./` is relative to the context root, protocol-prefixed
      values are resolved by their protocol, and bare values are resolved
      through context module maps or aliases. Future facets such as depth,
      segment count, suffix, glob/segment classes, or alias/module-map matching
      need an explicit generic CEM semantics decision before implementation so
      path layout remains schema-owned and independent of package-specific Rust
      validation branches.
- [ ] Design declarative CEMT body/output assertion vocabulary for
      schema-owned artifact constraints currently centralized in Rust helper
      checks. Immediate background: schema-package artifact declarations now
      express source readability (`artifact-source-readable`), CEMT parse
      validity (`artifact-cemt-valid`), CEMT output function lookup
      (`artifact-function-declared`), and output-function metadata matching
      (`artifact-function-contract`) as schema-owned constraints using
      `schema:resource-readable`, `schema:resource-parse`, and
      `schema:reference-resolution`. Rust still executes the CEMT-specific
      body/output inspection by parsing the CEMT module, selecting the declared
      output function by `@function-name`, and comparing function kind,
      target content type/schema/category, and optional function profile
      against manifest attributes. Future syntax should let schemas declare
      CEMT body/output selectors, function metadata projections, profile
      expectations, normalized media-type/URI comparisons, expected/invalid
      detail projection, and source-range propagation without hard-coding
      schema-package artifact semantics.

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
