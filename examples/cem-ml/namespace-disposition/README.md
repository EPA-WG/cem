# Namespace-Disposition Fixtures

Per `docs/cem-ml-ac.md` §AC-P-V-6 / §AC-P-6.7 and `content-type-switch.md`
BR-VC-9. Each fixture exercises the **unknown-namespace disposition**: when a
region's namespace resolves to a URI with no metadata, no explicit schema, and
no rule, the effective scope policy + run mode select one defined behavior —
`reject`, `allow` (unvalidated foreign content), or `ignore` (drop with a report
event).

Driven by `packages/cem_ml/src/schema/disposition.rs` (decision-core) applied in
`packages/cem_ml/src/schema/machine.rs` (`apply_unresolved_namespace_disposition`),
and asserted by `packages/cem_ml/tests/namespace_disposition_fixtures.rs`.

| Fixture                  | Region                                | Expected outcome |
| ------------------------ | ------------------------------------- | ---------------- |
| `unknown-namespace.cem`  | `{widget:gauge}` in `urn:example:widgets:1` | application/build-SSR → `cem.schema.unresolved_namespace` (Error); development → `cem.schema.unresolved_namespace_allowed` (Info). Known-namespace siblings (`main`, `p`, `cem:screen`) are unaffected. |

## Scope

This is the **parser-side** AC-P-V-6 verifier. FF-4 covers the BR-VC-9 run-mode
disposition over unknown *optional features per governed contract* (the default
selector AC-P-6.7 references); this suite covers the *unresolved-namespace
region* in the cem_ml parser. The default mode-selected dispositions are
realized as diagnostics; deeper drop / foreign-DOM handling (the full `ignore` /
`allow` materialization) and the scope-policy override source are future work.
