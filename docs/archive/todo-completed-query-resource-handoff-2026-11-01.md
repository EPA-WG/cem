# Shared query resource transport and checkpoint handoff — 2026-11-01

- [x] Add shared original-capability query byte endpoints, worker transport and
  generation resource bindings. Cover copied/foreign owners, closed-admission
  cleanup, cancellation/timeouts, late cleanup, source-byte accounting and stale
  privacy/policy witnesses. Refuse older mixed-vendor group constructors that
  cannot charge retained source bytes before any cleanup.
- [x] Exercise the binding with actual full-census owned WASM and worker
  retirement, native-sealed checkpoints and terminal/unknown journals. Reimport
  on fresh heaps, refuse entered credentials and preserve Studio recovery tests.

## Delivered behavior

The shared runtime's `query-source-resource` module supplies local and private
worker byte endpoints plus stable generation resource bindings. Native CEMQH1
verification and source/host/binding/request ownership remain in Rust. A trusted
physical producer supplies the original backend and ownership/epoch witness.
The helper never parses an AST, evaluates a query, fabricates a host or infers
execution rights from pins. No component/application UI behavior is added.

Endpoint owner tokens are original private map keys. Copied tokens, foreign
endpoints and mismatched physical participants fail. Workers keep their own
native owner map behind an original private MessagePort, with bounded copied
binary packets and independent control/deployment pins. The protocol has only
open, capture and release; wire IDs are not host capabilities.

Endpoint limits are 69,643 source bytes, 16,384 UTF-8 pin bytes, 16 pending
operations and 64 owners. Cancellation, unknown replies and at-most-five-second
timeouts fence the endpoint. Late import results are cleaned by the original
backend when available; late acknowledgments never renew currentness. Original
owner cleanup can use `cleanupCurrent` behind closed ordinary admission without
reopening new work. Native counters still block retirement until actual cleanup.

`QuerySourceResource.prepareRecovery` checks the caller's credential/epoch policy
before/after native capture, compares exact source bytes and prepares a read-only
witness. Group capture/release and fresh preparation recheck this witness and
the complete peer set. Evaluated values, candidates, results and request budgets
do not enter its source packet. Policy changes can refuse recovery while normal
application work continues.

Packet and UTF-8 pin bytes stay private, outside checkpoint/status DTOs, and are
charged as `retainedBytes` alongside committed checkpoint bytes within the existing
4 MiB aggregate ceiling. Capture validates counts and total sizes before cleanup;
allocation includes checkpoints in the same ceiling. A private attestation binds
budget support to the exact original group, refusing older mixed-vendor group
constructors. Existing adapters without replay-byte declarations retain their
prior accounting and independent package/cache budgets.

Resource release requires the original native/worker acknowledgment. The shared
physical owner then closes native admission, collects the complete v2 census,
holds the native freeze permit and disposes its actual heap/worker. A current
native-sealed checkpoint handoff retains terminal journal state. Unknown external
outcomes refuse handoff; source reimport replays no action. Copied retirement
records and mutated public checkpoint bytes cannot change private authority.

Fresh preparation checks the target's original endpoint, participant, runtime ID
and physical identity, then reimports source into a fresh native host and stages
new owners. The stable route changes only inside the shared binding transaction.
Stale checkpoints prevent allocation; credentials entered after allocation and
foreign targets revoke the fresh group before route publication. Old native
aliases stay retired and native diagnostics survive reimport.

The module/types ship with shared runtime assets and an internal `cem-elements`
re-export. The new acceptance target builds a private full-runtime query probe
and compiles the exact shared checkpoint store in isolation, avoiding dependency
on another package's dist directory. The full tooling probe is an unoptimized
test artifact outside published assets, not a minimal profile measurement.

Production native query endpoints, exact source-backed code/registry profiles,
imported/native rule owners and custom result-profile dependencies remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
No application admission or automatic Studio recovery is enabled. Entered
passwords/OTPs still block recovery until explicitly cleared, with unchanged
4 MiB checkpoint and five-second recovery policies. Source-route fixture islands
are inert checkpoint transport data; this fixture restores no DOM/UI owner.

See the [contract](../cem-native-schema-capsules.md#shared-query-resource-transport-and-checkpoint-handoff),
[shared module](../../packages/cem_ql/runtime/query-source-resource.mjs),
[types](../../packages/cem_ql/runtime/query-source-resource.d.mts),
[protocol fixtures](../../packages/cem_ql/tests/query-source-resource.test.mjs)
and [physical fixture](../../packages/cem_ql/tests/query-resource-wasm.mjs).

## Verification

- All 42 Node cases passed: nine new transport/resource/compatibility cases and
  33 existing lifecycle/audit/factory cases. Missing shared-module imports failed
  before implementation. Protocol unit doubles test failure mechanics only.
- All eight real physical scenarios passed through `cem_ql:test:query-resource-wasm`:
  fallback and worker success, stale checkpoint refusal, entered credentials after
  allocation and foreign targets. They use actual native source/binding/request
  owners, production owned factories, original private worker ports, complete
  36-registry v2 census/freeze, native-sealed checkpoints and terminal/unknown
  journals. Native diagnostics are compared before/after fresh reimport.
- All 292 selected native cases passed: 273 query/native-registration/datatype/
  envelope/structural cases and 19 admission/inventory/generation cases. No native
  query semantics or production WASM exports were changed.
- Shared element and CLI package builds passed, as did package verification:
  374 element files, clean package imports and 24,139,260 production WASM bytes.
  Existing owned-generation and memory-budget WASM drivers passed. Runtime assets
  were refreshed from shared source without rebuilding unchanged production WASM.
- The 59-component/24-capability source audit passed and was refreshed. The
  private full-runtime query probe compiled with pinned wasm-bindgen 0.2.122;
  existing unused-code warnings remain. Generated test fixtures stay outside git.
- All 24 selected browser checks passed: four shared retirement/ownership/resume
  cases (96 filtered), 18 component rendering/generation/attribute cases, and
  complete live Studio Login recovery in worker/fallback modes (six filtered).
  Native compilation and metadata finished before isolated browser acceptance.
  Entered-credential refusal remains covered; saved projects/drafts were not cleared.
- Node syntax, diff checks and all 342 local documentation links passed. Completed
  TODO fixture work was archived. Generated screenshots, Vitest attachments,
  private WASM probes and other build output remain excluded from the commit.
