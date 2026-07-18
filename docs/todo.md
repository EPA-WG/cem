# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Implement the declarative CEM-ML reference vocabulary described in
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md)
behind `schema:reference-resolution` without changing existing public
diagnostic compatibility.

- [ ] Add parser/schema IR coverage for `candidates`, operand role elements,
      canonical `lookup` children, `compare`, `projection`, and constraint
      execution fields.
- [ ] Validate declaration errors for candidate cardinality, operand
      `@binding`, `@from` grammar, lookup/result cardinality, `record-set`
      result keys, capability names/version constraints, projection tokens, and
      alias collisions.
- [ ] Implement pure candidate selection and operand source extraction using the
      constrained CEM-QL/CEM source-path profiles from the design.
- [ ] Implement lookup declaration expansion: operand `@lookup` shorthand,
      canonical lookup children, pure document lookups, engine-assisted
      capability negotiation, `@support`, `@package`, provenance, and
      source-range policy.
- [ ] Wire declarative operands into the existing normalized value and
      comparison evaluators, preserving explicit `missing`, `invalid`,
      `unresolved`, and `unsupported` states.
- [ ] Implement projection profiles and tokens: compatibility defaults,
      structured arrays, `sourceRange`/`sourceRanges`, `comparison`,
      `provenance`, `aliases`, and candidate context.
- [ ] Migrate one existing schema-package reference-resolution check to the
      declarative vocabulary as the first end-to-end fixture, keeping current
      diagnostics stable.
- [ ] Verify with focused Rust tests first, then run
      `NX_DAEMON=false yarn nx run cem_ml:test`.

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
      pure versus engine-assisted placement, prior-art comparison, symmetric
      scalar/set normalization rules, and coverage for media-type records and
      essences, schema/document URI, namespace URI, content category, profile
      name, artifact/function name, and exact scalar values.
- [x] Design comparison vocabulary for normalized reference values. The
      decision is recorded in
      [`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md):
      comparison now has operand roles, state policies, explicit operators for
      equality, membership, set coverage/overlap/disjointness, existence, and
      record-field matching, plus diagnostic projection and source-range
      ownership for expected, invalid, missing, and unresolved values.
- [x] Design the remaining richer declarative cross-node/reference vocabulary
      for schema-owned constraints currently declared in CEM-ML but executed by
      Rust. The decision is recorded in
      [`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md).
      The vocabulary lets a schema declare candidate
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
