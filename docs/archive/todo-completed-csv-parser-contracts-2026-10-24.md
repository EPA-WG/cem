# Native CSV parser contracts — 2026-10-24

- [x] Add native parser-backed package compilation contracts, starting with CSV's
  existing `csv-parse-report-fact` diagnostic behaviors. Preserve real parser
  validation and source ownership; activation continues refusing unsupported or
  pending models, without diagnostic filtering or synthetic ready models.
- [x] Add native CSV fixtures for checked fact bindings, retained source ownership,
  public activation, malformed/unknown/pending contracts, built-in incumbents and
  execution domains, owned diagnostic overrides and ordinary lifecycle/command
  consumption, including original-tree forwarding through the linked CEM-QL
  compiler. Preserve strict formatter-source refusal and atomic preparation
  retirement.

The registered native parser phase compiles CSV's existing constraint metadata
before structural diagnostic-behavior validation. It claims only the exact CSV
schema/behavior pair and validates all eight actual decoder/parser facts, unique
bindings, native targets and declared diagnostics. Encoding and syntax failures
must remain hard. Advisory severities can be upgraded; diagnostic codes, contract
names and policies remain authored. Unknown or missing facts, duplicate bindings,
incorrect targets, diagnostic execution redirection and unsafe severities cannot
produce an executable contract. Unsupported behaviors retain their diagnostics.

`compile_schema_document_model_from_retained` binds execution to the original
retained schema tree. Inspection remains unbound. The linked declaration compiler
forwards a registered source tree only when it shares the original AST allocation;
the package compiler rejects foreign owners. A native compiler cannot publish a
CSV model by omitting the required parser contract or returning an empty model.

Lifecycle document loading and source validation dispatch original input bytes to
the checked native contract. CSV uses the existing decoder, parser, parse-fact
validation and typed AST construction with its admitted diagnostic bindings.
Ordinary engine validation and formatter/colorizer conversion use public package
activation; fixtures no longer manufacture a private CSV activation binding.
Malformed quote/encoding diagnostics retain the default parser's exact metadata
and positions. Source maps and lexical captures remain paired with native owners.

Parser readiness is distinct from structural CEM-AST readiness. Namespace claims,
structural input/region validation and CEM-QL schema scope preparation do not gain
authority from a parser-only model. A CEM projection cannot replace raw CSV syntax
validation. Unauthorized replacement inspection preserves the actual default
parser incumbent, including its retained source contract.

The existing multi-package rollback fixture now supplies an explicitly pending
CSV candidate, since valid CSV is supported. Its incumbent/retirement assertions
remain intact: a later failure cannot retire earlier prepared contracts, while a
successful publication does retire replaced preparations.

## Verification

- Four native parser-contract cases and twelve activation/ordinary-consumer cases
  pass. The linked original-owner parser integration case passes.
- Native regressions pass: CSV kernel (10), CSV lifecycle selection (1), package
  contracts (60), ordinary conversions (51) and schema document models (154).
- Declaration-reference integration passes 86 cases; CEM-QL schema scope and
  structural validation integration pass 30 and 9 cases respectively.
- Optional release WASM rebuild and its 200-source admission/reimport verifier
  pass. Shared loader/HTTP tests pass 28/28. The production worker verification
  passes all 205 offline assets, cold offline loading, distinct-heap reimport,
  strict missing-source refusal and invalid/incomplete deployment refusal.
- Locked workspace checking and the linked compiler's `wasm32-unknown-unknown`
  check with `schema-asset-packs` pass.
- The broader linked package lifecycle suite passes 46/49 cases. Three existing
  preparation cases fail at the unchanged `attribute_datatypes.rs:473` source-map
  equality assertion: the actual candidate retains an additional `SourceBoundary`
  frame. All three failures reproduce independently in an isolated tracked-source
  snapshot of unchanged commit `eca54d20`: `native_pretyped_handoff_retains_restricted_candidate_and_repeated_budget`,
  `callback_context_or_publication_changes_prevent_handle_publication`, and
  `checked_preparation_replacement_preserves_defaults_sealed_values_and_publication`.
  Their assertions remain unchanged; diagnosis is recorded separately in the
  active todo list.

## Boundaries

CSV is the first registered native parser-contract provider. Other parser-backed
package providers remain unsupported until they implement equivalent checked
contracts. Low-level legacy CSV catalog constructors keep their existing behavior;
ordinary activated consumers use the checked execution path. Persistent WASM
activation endpoints, declarative demand and coordinated publication remain in
the active todo list. No application UI JavaScript or serialized native owners
were added. Credential-entry recovery refusal, the 4 MiB checkpoint budget and
the default complete release remain intact. No memory/download savings are claimed.
