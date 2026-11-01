# Native-provisioned query endpoints and TODO cleanup — 2026-11-01

- [x] Preserve the previous TODO snapshot, replace stale completion status and
  flatten completed native foundations out of the active WASM checklist. Keep
  unresolved diagnostics, runtime admission and deferred work explicit.
- [x] Add native provisioned query endpoint fixtures for original deployment and
  source/profile pins, changed declarations, missing functions, cancellation,
  owner loss/retirement, session limits and closed-admission cleanup.
- [x] Exercise shared worker/fallback endpoints with original opaque native owner
  tokens, copied/foreign-token refusal, actual linked-generation provisioning,
  complete census, fresh reimport and credential-blocked checkpoint handoff.

## Delivered behavior

`QuerySourceEndpoint` is shared native implementation, separate from the test
embedding. Its trusted Rust caller supplies independent deployment pins, actual
selected functions, an original verified result profile, operation control and
the actual native generation. Packet/control metadata must agree with that
deployment; it cannot create its own authority. Original profile sources are
registered directly, and both result and diagnostic selectors must match.
No implicit catalog, JS schema parser or parallel evaluator is added.

Each session retains fresh original native source/code/profile/registry/binding/
request owners and borrows the deployment weakly. Lost/retired deployments,
cancellation and closed generations fence work. A new equal deployment cannot
renew old sessions. Failed opens release partial resources and reservations.
Source capture excludes evaluated values and request budgets.

The deployment contributes one Registry root, with seven additional roots per
session. The native endpoint admits at most 32 sessions (225 roots including
deployment) within the existing 256-root generation ceiling. Other native work
may exhaust the shared limit sooner. Retirement does not hide retained memory;
actual drop releases roots. The first capacity fixture exposed the existing
shared ceiling at 64 sessions; the endpoint was bounded to 32 without enlarging
the native budget or weakening the generation gate.

The opt-in WASM API has three byte operations: `querySourceOpenV1`,
`querySourceCaptureV1` and `querySourceReleaseV1`. Rust-only installation checks
the actual linked generation and borrows the original endpoint weakly. There is
no JS provisioning API or numeric native session ID. Open returns a frozen
original JS object; native registries require that identical token. Copies,
structured clones, foreign tokens and released tokens fail. Native registry
mutations invalidate inventory revisions, and actual leases join the existing
complete 36-registry v2 census.

The shared `createQuerySourceWasmBackend` adapter forwards bytes and original
tokens through the owning runtime, checking its actual Memory weakly. Private
worker IDs remain confined to the existing worker transport. No app-local or
component UI JavaScript is introduced. Original deployment/source epoch and
policy witnesses remain the embedding's responsibility; Memory identity alone
is not recovery authority.

Original session release and native uninstall work behind closed ordinary
admission. The embedding must remove/drop its deployment as well as sessions
before retirement can seal. Fresh worker/fallback heaps require newly provisioned
original endpoints before shared source replay and binding publication. The
test-only bootstrap supplies the shipped original schema and reviewed functions;
the library exports no fixture bootstrap and default builds remain unchanged.

Cross-heap diagnostics preserve the exact original source URI, schema selector,
ranges and complete frame stack. The extended probe exposed a previous equality
assertion on heap-local allocation addresses; those addresses are checked for the
native shape and excluded only from cross-heap equality. Original source node 14
and all other provenance fields remain asserted. Allocation addresses never act
as portable source or execution authority.

The [prior TODO snapshot](todo-snapshot-2026-11-01.md) preserves all earlier task
details with corrected relative links. The active checklist now places the
selected component-WASM workstream first, links completed category indexes and
lists remaining gates directly instead of nesting completed foundations under
unfinished parents. Runtime bootstrap generation, imported/native rule owners,
custom profile dependency transport, CEMAPPD4 transport, live reduced controls
and linked-code evidence remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
No reduced-runtime admission or automatic Studio recovery is enabled. Entered
passwords/OTPs still require explicit clearing, with unchanged 4 MiB checkpoints,
five-second recovery deadlines and external-operation reconciliation.

See the [contract](../cem-native-schema-capsules.md#native-provisioned-query-endpoints),
[native implementation](../../packages/cem_ql/src/query_source_endpoint.rs),
[WASM adapter](../../packages/cem_ql/src/query_source_endpoint/wasm.rs),
[native cases](../../packages/cem_ql/tests/query_validator_capsule/endpoint.rs),
[shared adapter](../../packages/cem_ql/runtime/query-source-resource.mjs) and
[physical handoff](../../packages/cem_ql/tests/query-resource-wasm.mjs).

## Verification

- Missing native endpoint APIs and the shared JS backend export failed their
  initial tests. All 303 selected native cases passed: 284 query/registration/
  datatype/envelope/structural cases (five new endpoint cases) and 19 generation/
  inventory/admission cases. Default, component baseline and opt-in WASM checks
  passed with existing unused-code warnings.
- Both isolated WASM targets passed after rebuilding the opt-in component/full
  probes. Independent memories and a real worker cover provisioning, foreign
  generations, original opaque tokens, stale contexts, cancellation, source
  agreement, request budgets, retained roots and scoped drains.
- All eight physical worker/fallback handoff scenarios passed with native-sealed
  journals, complete v2 census, original deployment teardown, fresh native
  provisioning and credential refusal. Retained deployment owners block a clean
  census until explicitly removed; stale checkpoints, late credential policy and
  foreign targets refuse publication.
- All 43 shared Node lifecycle/resource/audit/factory cases and existing actual
  owned-generation/memory-budget WASM drivers passed. The shared package build
  and clean import verification passed: 374 files, 24,139,260 production WASM
  bytes. Runtime JS/types were refreshed without rebuilding unchanged default
  WASM. The 59-component/24-capability source audit was refreshed and checked.
- All 24 selected browser checks passed: four shared retirement/ownership/resume
  cases, 18 component rendering/generation/attribute cases, and complete Studio
  Login recovery in worker/fallback modes (six Studio cases filtered). Native
  compilation and metadata finished before isolated browser acceptance. Entered
  credential refusal remains covered; saved projects and drafts were not cleared.
- Focused Rust formatting, Node syntax, diff checks and all 468 local
  documentation targets passed. Links to retired TODO sections now point to
  completion records; the Studio milestone table is labeled as its historical
  planning baseline. Generated screenshots, Vitest attachments, private WASM
  fixtures and build output remain excluded from the commit.
