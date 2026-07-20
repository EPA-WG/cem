# Todo

This file tracks remaining execution tasks only. Product/module sequencing lives in
[`../roadmap.md`](../roadmap.md). Future wishlist work lives in
[`wishlist.md`](wishlist.md). Completed implementation history belongs in git
history.

## Immediate Goal

Freeze one implementable schema-owned reference-normalization contract across
[`cem-ml-reference-normalization-design.md`](cem-ml-reference-normalization-design.md),
[`cem-ml-reference-vocabulary-design.md`](cem-ml-reference-vocabulary-design.md),
and
[`cem-ml-reference-comparison-design.md`](cem-ml-reference-comparison-design.md)
before resuming implementation. High-level goals and design concepts take
priority over older concrete fields and examples: preserve declared values and
source provenance, keep normalization annotative and symmetric, make execution
placement explicit, avoid implicit canonicalization, and retain complete schema
identity.

Current active slice: design declarative CEMT body/output assertion vocabulary
for schema-owned artifact constraints currently centralized in Rust helper
checks.

### Contract And Documentation

- [x] **[C1] Define staged lookup normalization.** Specify source extraction,
      source cardinality guard, lookup-key normalization, lookup, raw-result
      cardinality guard, comparable-result normalization, normalized-result
      cardinality guard, state policy, comparison, and diagnostic projection.
      Give lookup keys and results separate envelopes, normalizers, bindings,
      and provenance. Assign operand `@binding` to the final comparable result,
      retain lookup-key bindings as provenance only, and repair the endpoint
      example accordingly.
- [x] **[C2] Restore complete schema identity.** Define pure
      `schema:schema-uri-declaration`, engine-assisted
      `schema:schema-identity`, and an explicit URI-only compatibility
      projection. Preserve declared URI, canonical descriptor identity,
      embedded full SemVer, version constraint, and match rule; never use URI
      equality as complete schema identity.
- [x] **[C3] Remove the package-validation bootstrap cycle.** Document pure
      manifest/source declaration checks, isolated provisional descriptor
      construction, validation against built-ins plus the provisional overlay,
      and catalog admission only after every required check passes.
- [x] **[C4] Unify outcome, support, and lifecycle taxonomies.** Freeze terminal
      states as `valid|missing|invalid|unresolved|unsupported`, require a reason
      for every non-valid outcome, keep availability support as
      `required|optional`, and lower any `soft` shorthand to optional support
      plus reporting policy. Define pending/deferred lookup behavior so an
      incomplete lookup is not prematurely final `unresolved`. Keep support and
      reporting policy independent from assertion success, and reject statically
      known unsupported required operations as schema/compiler errors. Keep
      `unsupported-normalizer`, `unsupported-capability`, and `policy-denied` as
      reasons rather than states.
- [x] **[C5] Separate cardinality, shape, and collection provenance.** Define
      cardinality as `one|optional|set|sequence` independently from
      `scalar|record` shape, keep candidate cardinality separate, and do not
      treat `record-set` as a fundamental shape. Either define `sequence`
      semantics or explicitly defer them from the first release. Require named
      set normalizers that declare an `itemNormalizer`, add
      `schema:namespace-uri-set`, and preserve a sorted,
      deduplicated comparison set alongside source-ordered item outcomes,
      duplicates, declared/normalized values, reasons, and source ranges.
- [x] **[C6] Define scalar/set normalization symmetry.** Require the same item
      normalization and equivalence semantics rather than identical collection
      normalizer names. Permit scalar `N` against `set-of(N)` only for an
      operator-declared compatible item type, expose collection and item
      normalizers in metadata, and reject incompatible mixed normalizers.
- [x] **[C7] Unify state-policy ownership.** Keep per-operand state policy as
      `required-valid|optional-valid|allow-unresolved|allow-unsupported`, put
      relational presence rules such as `when-present|both-or-none` on the
      comparison, remove redundant `unresolved-fails`, lower shorthands into
      explicit IR, and replace or clearly label non-parseable pseudocode.
- [x] **[C8] Separate MIME syntax from registered content identity.** Keep RFC
      media-type normalizers strict and move registered RFC and legacy aliases
      to engine-assisted content-identity normalizers. Preserve alias owner,
      routing profile, canonical identity, and declared spelling; define stable
      unknown/ambiguous reasons and required schema context. Resolve the
      `schema:media-type` datatype/normalizer symbol collision through kinded
      registries with one grammar primitive or by renaming the normalizer. Keep
      the CEM-QL `accepts` alias table scoped to CEM-QL rather than treating it
      as a global content alias registry.
- [x] **[C9] Make exact scalar normalization typed.** Define
      `schema:scalar-exact` as exact `(type, value)` comparison without
      coercion, add `schema:string-exact` for text-only contracts, and add a
      distinct lexical normalizer where spelling matters. Keep
      `declaredValue`, `sourceLexeme`, and `sourceRange` separate.
- [x] **[C10] Separate schema and namespace identity domains.** Compare manifest
      schema references with canonical schema identity or explicit URI
      declarations, and compare namespace claims only with namespace values.
      Treat schema v1's namespace-as-identity behavior as a versioned
      compatibility adapter and plan an explicit schema-identity field.
- [x] **[C11] Define identifier, profile, function, and artifact domains.** Link
      identifier tokens to an authoritative datatype grammar; define dotted,
      case-sensitive profile symbols; distinguish lexical function names from
      function identities containing module/artifact identity plus canonical
      exported name. Reserve artifact names for authored IDs and create a
      separate path-derived artifact-identity normalizer after document-URI
      resolution. Normalize every component of composite lookup keys while
      preserving authored spellings and source ranges.
- [x] **[C11.1] Define the base identifier grammar.** Link
      `schema:identifier-token` to one authoritative schema datatype grammar and
      state its exactness rules. Keep it narrow enough for schema-owned tokens,
      but do not use it as a catch-all for profiles, functions, artifacts, or
      path-derived identities.
- [x] **[C11.2] Split profile symbols from identifiers.** Define
      `schema:profile-name` as an exact, case-sensitive dotted symbol grammar
      that accepts package-qualified values such as
      `acme.showcase.format-tree`. Preserve the authored spelling and range;
      do not case-fold, segment-normalize, or treat profiles as bare
      identifiers.
- [x] **[C11.3] Split function names from function identities.** Keep
      `schema:function-name` as the authored exported lexical name. Add
      `schema:function-identity` as the compiled/registry identity record that
      includes module or artifact identity plus the canonical exported function
      name and optional function profile when the schema contract requires it.
- [x] **[C11.4] Split authored artifact names from path-derived identities.**
      Reserve `schema:artifact-name` for authored manifest artifact IDs only.
      Add a separate artifact path or artifact identity normalizer that runs
      after `schema:document-uri` resolution and records declared URI, resolved
      URI, package context, artifact kind, and provenance.
- [x] **[C11.5] Define composite function/artifact lookup keys.** Require each
      component to be normalized by its own domain normalizer: function
      name/identity, authored artifact name or resolved artifact identity,
      content-type identity, schema identity, content category, profile, and any
      subject type. Composite equality compares the normalized record fields,
      not one concatenated string.
- [x] **[C11.6] Update reference docs and current package docs in dependency
      order.** Patch normalization first, then vocabulary and comparison, then
      schema-package README/example prose. Keep shipped compatibility wording
      where current artifacts still expose only lexical names or paths.
- [x] **[C11.7] Add acceptance coverage for the split.** Document dotted
      profile symbols, authored artifact IDs versus path-derived artifact
      identities, lexical function names versus compiled function identities,
      and at least one composite lookup key with all component spellings and
      source ranges preserved.
- [x] **[C12] Correct target/current status and diagnostic projection.** Mark
      the three reference documents as accepted target design with
      implementation pending and schema-package READMEs as the current shipped
      surface. Freeze only shipped diagnostic keys, treat `unsupportedValues`
      as additive until implemented, add operand and per-item reasons, reserve
      `projection` for diagnostics, rename record extraction to `value-path` or
      `normalized-field`, and define record field-pair/state-policy syntax.
      Keep `schema:reference-resolution` as orchestration and compatibility
      behavior; it must not directly convert normalization outcomes into
      violations.
- [x] **[C13] Bound document URI normalization and lookup lifecycle.** Define
      document identity against effective base URI, resolver purpose,
      package/module-map context, and policy without fetching or asserting
      existence. Preserve declared/resolved URIs and resolver provenance, keep
      exact namespace equality separate, and leave language-specific behavior
      such as JSON Schema dynamic references in explicit capabilities.
- [x] **[C14] Decide the first-release boundary for ordered and mutable
      references.** Either scope the first vocabulary release to
      schema-package registry/artifact checks or define sequence cardinality,
      host-language ID normalizers that preserve case and punctuation,
      snapshot/revision identity, pending dependencies, and recomputation. Do
      not reuse identifier-token or set semantics for ordered ARIA IDREFs.
- [x] **Align the dependent documentation in dependency order.** Update the
      normalization, vocabulary, comparison, and registry designs first; then
      update `schema/v1`, `schema-package/v1`, the schema-package overview, and
      relevant package READMEs. Distinguish the shipped surface from the target,
      document descriptor provenance and provisional overlays, and preserve
      compatibility projection throughout the migration.
- [x] **Standardize reference terminology after the model freezes.** Use schema
      URI versus fetchable URL, canonical registry identity, `binding` versus
      domain `name`, `declaredValue`, `sourceLexeme`, `normalizedValue`, and
      `resolvedUri` consistently across the affected Markdown corpus. Define
      descriptor provenance fields for complete schema identity, raw and
      normalized content claims, namespaces, descriptor origin/source ranges,
      and CEMT output metadata.
- [x] **Run a final documentation consistency gate.** Confirm the design trio,
      registry design, resolver contracts, schema-package documentation, and
      package examples express one contract before changing `.cem` sources,
      Rust IR/evaluators, fixtures, or diagnostics.

### Acceptance Cases

- [x] Document an example of different schema URI version-tail constraints
      resolving to the same descriptor and embedded version.
- [x] Document an example of local custom package validation before registry
      admission.
- [x] Document examples of RFC media types, registered RFC aliases, bare legacy
      aliases, ambiguous aliases, and invalid media syntax.
- [x] Document scalar-to-set membership with duplicates, invalid members,
      source-ordered provenance, and deterministic comparison values.
- [x] Document `missing`, `invalid`, `unresolved`, `unsupported`, and
      pending/deferred lifecycle outcomes.
- [x] Document dotted profile symbols and composite function identities.
- [x] Document whether ordered ARIA IDREF sequences and dynamic JSON Schema
      references are supported or deferred.

## Follow-on Implementation

The immediate documentation contract and acceptance cases are complete.
Implement the declarative CEM-ML reference vocabulary behind
`schema:reference-resolution` without changing existing public diagnostic
compatibility, reconciling field names and IR shapes with the accepted design.

- [x] Add parser/schema IR coverage for `candidates`, operand role elements,
      canonical `lookup` children, `compare`, `projection`, and constraint
      execution fields.
- [x] Validate declaration errors for candidate cardinality, operand
      `@binding`, `@from` grammar, lookup-key/result cardinality and shape,
      capability names/version constraints, projection tokens, and alias
      collisions.
- [x] Implement pure candidate selection and operand source extraction using the
      constrained CEM-QL/CEM source-path profiles from the design.
- [x] Implement lookup declaration expansion: operand `@lookup` shorthand,
      canonical lookup children, pure document lookups, engine-assisted
      capability negotiation, `@support`, `@package`, provenance, and
      source-range policy.
- [x] Wire declarative operands into the existing normalized value and
      comparison evaluators, preserving explicit `missing`, `invalid`,
      `unresolved`, and `unsupported` states.
- [x] Implement projection profiles and tokens: compatibility defaults,
      structured arrays, `sourceRange`/`sourceRanges`, `comparison`,
      `provenance`, `aliases`, and candidate context.
- [x] Migrate one existing schema-package reference-resolution check to the
      declarative vocabulary as the first end-to-end fixture, keeping current
      diagnostics stable.
- [x] Verify with focused Rust tests first, then run
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
registered content identities. Similar Rust-executed reference-resolution
shapes exist for example content-type/schema compatibility and artifact CEMT
function lookup/metadata matching.

- [x] Design any further generic path-layout vocabulary beyond the current
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
