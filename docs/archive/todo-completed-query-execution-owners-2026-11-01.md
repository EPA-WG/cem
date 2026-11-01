# Source-backed query code and registry/profile ownership — 2026-11-01

- [x] Add source-backed result-profile fingerprints and original
  code/registry/profile owner guards for direct query bindings. Cover changed
  same-name schemas, missing/foreign owners, context/generation changes, weak
  lifetimes, retained roots, cancellation and atomic replacement.
- [x] Require independently emitted execution-profile pins and renewed native
  owners in the opt-in WASM worker/fallback handoff fixtures. Preserve credential
  refusal, source provenance, complete census and checkpoint budgets.

## Delivered behavior

The shared native profile builder compiles the original retained schema trees,
lexical captures and contract selectors. Its versioned source fingerprint covers
complete source snapshots, identities, namespace bindings, selected result and
diagnostic contracts and ML/QL versions. Tree/capture/schema document ownership
must match; conflicting same-URI source claims fail. Input limits are 16 sources,
16,384 aggregate nodes, 4 MiB encoded data and one million cancellation/work
checks. No synthetic schema, JSON AST transport or separate evaluator is added.

The explicit code owner retains the actual reviewed function table narrowed to
the verified query's requirements and borrows its exact original source. The
profile owner retains the compiled profile and actual registered source scopes,
checking an independent pin. The registry owner borrows those original owners
and source, checking the result contract, actual context and runtime generation.
Equal schema bytes and newly created equivalent source owners cannot substitute
those originals.

Owned bindings borrow registry authority weakly. Lost/retired owners and changed
contexts/generations fence execution and capture, including post-evaluation
checks. Replacement stages before retiring its predecessor; failed replacement
preserves the previous binding. A managed session has seven roots: one Host,
two Compilation, three Registry and one Preparation. Retirement does not hide
retained roots; only actual drop releases them. Legacy native linking remains
available without claiming this stronger ownership contract.

The test deployment sidecar is now explicitly versioned `cem-query-execution/1`:
original CEMQH1 pins and independently emitted result-profile pins. Binary
CEMQV1/CEMQD1/CEMQH1 source formats are unchanged. Fresh WASM fixtures rebuild the
original trusted shipped profile, check the pin, register original scopes and
renew source/code/profile/registry owners before linking. Shared JS only carries
bounded opaque bytes/control metadata. Native AST, compilation and execution
remain native; no Studio-specific behavior or UI JavaScript is introduced.

The private full-runtime worker/fallback fixture joins these renewed owners to
complete native census, disposal, sealed checkpoints/journals and fresh binding
publication. Entered passwords/OTPs still block recovery until explicitly
cleared. No evaluated values, credentials, request budgets or external actions
are replayed. Automatic Studio recovery remains off, with unchanged 4 MiB
checkpoint and five-second recovery policies.

Source fingerprints establish agreement, not linked-code attestation. Production
native endpoint provisioning, imported/native rule owners, custom/multi-source
result-profile dependency transport and CEMAPPD4 transport remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
The fixture's original shipped-schema parse is not a minimal-binary measurement
or proof that general parsers/schemas were removed. No component admission or
production endpoint is enabled.

See the [contract](../cem-native-schema-capsules.md#source-backed-query-execution-owners),
[profile builder](../../packages/cem_ql/src/query_validator_capsule/profiles.rs),
[native owners](../../packages/cem_ql/src/structural_receiver/query/execution.rs),
[native cases](../../packages/cem_ql/tests/query_validator_capsule/execution.rs),
[WASM probe](../../packages/cem_ql/tests/query-source-wasm.mjs) and
[physical handoff](../../packages/cem_ql/tests/query-resource-wasm.mjs).

## Verification

- Tests failed for the missing profile builder/owner APIs before implementation.
  All 298 selected native cases passed: 279 query/native-registration/datatype/
  envelope/structural cases (including six new execution-owner cases) and
  19 admission/inventory/generation cases.
- Default and `component-runtime` WASM library checks passed. The opt-in
  component/full probes rebuilt with pinned wasm-bindgen 0.2.122; independent
  heaps and a real worker passed changed-profile/version refusal, missing code,
  original provenance, retired code/profile/registry, request budgets,
  cancellation and original-root accounting.
- All eight full physical worker/fallback handoff scenarios passed with actual
  owned WASM factories, native-sealed journals and complete 36-registry v2 census.
  Stale checkpoints, late credential policy changes and foreign targets refuse
  publication. These are private fixtures, not production endpoints.
- All 42 shared Node lifecycle/resource/audit/factory cases and existing actual
  owned-generation/memory-budget WASM drivers passed. The 59-component,
  24-capability source audit was refreshed and its check passed. Production
  WASM assets remain unchanged; the full binary is 24,139,260 bytes.
- All 24 selected browser checks passed: four shared retirement/ownership/resume
  cases, 18 component rendering/generation/attribute cases, and complete Studio
  Login recovery in worker/fallback modes (six Studio cases filtered). Native
  compilation finished before isolated browser acceptance. Entered-credential
  refusal remains covered; saved projects and drafts were not cleared.
- Focused Rust formatting, Node syntax, diff checks and all 350 local
  documentation targets passed. Completed fixture work was archived. Generated
  screenshots, Vitest attachments, private WASM probes and other build output
  stay outside git.
