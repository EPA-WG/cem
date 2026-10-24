# Native owned-package activation — 2026-10-24

- [x] Add native owned-package activation fixtures: atomic publication with exact
  replacement grants, readiness and dependency refusal, ordinary command/render
  consumption, immutable source ownership, and strict missing-body refusal with
  legacy consumers preserved.

`schema::asset_pack_activation::activate_schema_asset_packages` stages exact
package versions from an admitted immutable `Arc<SchemaAssetPack>` into an
ordinary native `EngineContext`. The existing native manifest/model compiler
keeps its authority and readiness checks. Index metadata alone cannot replace
the default catalog or activate a pending/unsupported model. Declared converter
and formatter paths/MIME must match that package's pinned index identities.
Dependencies must exist in the consumer or selected set.

The complete selected set publishes together. Failed admission, authorization,
compilation or readiness leaves incumbent registries, source bindings and
prepared contracts intact. Preparation retirement is deferred until final
publication; a fixture exercises a successful first candidate followed by a
failing second candidate, then verifies retirement only on a successful retry.
Already active models cannot disguise a rejected pending replacement.

Context clones share an immutable source owner. The final clone releases it.
Republishing can add admitted bodies but cannot mix independent index pins or
discard already owned bodies. The ordinary mutable manifest loader cannot change
a pinned package, even with an owner replacement grant. Its refusal precedes
candidate resource reads; a changed pin requires a fresh native consumer context.
Source count/bytes are embedding accounting data, never retirement authority.

Ordinary converter reads, template imports and engine output environments use
the binding. Selected package sources retain canonical URIs and refuse missing
bodies, wrong identities/MIME and filesystem escapes. A registered native
conversion fallback cannot substitute for a missing selected template body.
Per-artifact fallback policy preserves unselected packages and legacy readers.
No AST JSON handoff or application/component JavaScript was added.

The actual DOM projection package activates and serves ordinary CEM-to-HTML
conversion. CSV's existing source-only schema compilation reports unsupported
`csv-parse-report-fact` diagnostic behaviors. Public activation continues to
refuse it; the default CSV parser/formatter remains usable. An isolated native
reader fixture checks ordinary CSV formatter/colorizer admission without
claiming successful CSV schema publication or filtering its diagnostics.

## Verification

- Nine native activation/ordinary-consumer cases pass with
  `cargo test --locked -p cem-ml native_asset_ --lib`.
- Adjacent native regressions pass: `schema_package_` (60),
  `real::tests::convert_` (51), `schema::asset_pack::tests` (12) and
  `conversion::schema_asset_pack_tests` (7).
- Source and lexical bindings share the original retained native AST allocation;
  output matches the corresponding default engine output for admitted sources.
- The two runtime diagnostics are declared in the native schema-package schema;
  its diagnostic audit includes the new activation module. The native emitter
  regenerated the compiled bootstrap pin and all 205 files (32 packages,
  200 source bodies, 1,344,144 source bytes). The catalog pin test verifies the
  committed bootstrap against those exact source inputs.
- `cargo check --locked --workspace` and the linked optional WASM target check
  pass. The optional release WASM was rebuilt, then its native verifier passed
  all 200 source admissions and independent reimports.
- The shared optional loader/HTTP cohort passes 28/28 cases. The production
  worker verifier passes complete 205-file offline delivery, cold offline loads,
  distinct-heap source reimport, strict missing-source refusal and rejection of
  invalid/incomplete deployments. Browser work uses isolated fixture contexts.
- New Rust sources pass `rustfmt --check`; `git diff --check` passes. No generated
  screenshots or Vitest attachments are part of the change.

CSV's activation limitation above is superseded by the
[native CSV parser-contract follow-up](todo-completed-csv-parser-contracts-2026-10-24.md).

## Remaining integration

This increment provides the native consumer boundary. Persistent owned WASM
endpoints, shared loader/service wiring, declarative demand, inventory enrollment
and coordinated fresh-generation publication remain active work. No new global
WASM handles or automatic recovery path were introduced. The default complete
release, 4 MiB checkpoint budget and entered-credential recovery refusal remain
in place. No binary-size, checkpoint-size or memory-headroom savings are claimed.
