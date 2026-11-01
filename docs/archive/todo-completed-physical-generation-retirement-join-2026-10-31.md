# Physical generation retirement join — 2026-10-31

- [x] Join original native enrollment to the actual linked worker/owned-fallback
  lifecycle, complete native census, ownership confirmation and checkpoint/
  terminal-journal handoff.
  - [x] Test a separately versioned complete census including all six original
    enrolled owner kinds and an atomic join of original drain/local revision,
    full census revision, cancellation/deadline and native freeze permit.
  - [x] Refuse legacy census/worker acknowledgments for managed physical
    retirement while preserving their inspection contract.
  - [x] Test real independent WASM memories, retained original native sources,
    stale/foreign witnesses, count-preserving cleanup, failed/reentrant
    preparation, fresh compilation and original worker/fallback handoff.
  - [x] Test final host deadline/ownership checks after synchronous handoff
    callbacks; late acknowledgments grant neither termination nor replacement.
  - [x] Verify real-WASM hard-pressure refusal of work, retained-root blocking,
    bounded cleanup/retirement and preservation of previously recorded failures.

## Delivered behavior

`runtime_admission::linked_generation()` owns one original generation per linked
instance before its first shared entry. Every managed native entry and closure
uses that actual admission object. Worker/fallback participant registration
continues to use original Worker/Memory identity; labels and DTOs do not identify
physical runtimes or restore native handles.

Schema version 2, `cem-ml-cem-ql-generation-v2`, combines the complete 30-registry
base manifest with six actual enrolled-root counters. The native collector
validates the entire base before joining its original ledger; the managed parser
requires the exact v2 manifest and permitted discardable roots. V1 collection
and inspection remain separately supported. Managed physical retirement refuses
v1 snapshots; rendering workers additionally attest the profile read from their
own fresh native census, so older acknowledgments cannot enter this path.

Final preparation recollects the actual full census and joins its exact revision
with an original sealed local drain under the admission lock. The nonportable
`GenerationRetirementPermit` freezes all work and cleanup only with zero actual
entries/enrolled roots/full-census blockers. Its original drain survives while
the permit is held. Stale/foreign/local-mutated proofs, cancellation and expiry
refuse; expiry/cancellation cannot silently reopen cleanup. Dropping the original
permit permits cleanup under permanently closed ordinary admission.

Bounded lifecycle checks retain existing cancellation, deadlines and prior
failures without applying ordinary work-memory admission. A real-WASM regression
first demonstrated that ordinary memory checking blocked empty-heap retirement
under hard pressure. The repair admits no parsing/rendering or new work and
never clears a recorded failure. Ordinary memory refusal and retained-root
blocking remain intact.

Existing host ownership/publication/source coverage and sealed checkpoint/journal
handoffs still gate physical termination or releasing owned fallback references.
Final synchronous callbacks cannot overrun the host deadline and then terminate.
Fallback factories recheck their original native confirmation after ownership
callbacks and fence reentrant abort/disposal. No external action is replayed.
Failed preparation retains host checkpoint bytes and follows bounded abort/
fencing rules; no copied confirmation grants replacement authority.

This completes the physical lifecycle infrastructure join. CEMAPPD4/structural
receivers remain native-only; fresh WASM source/code/registry transport, portable
query-owner adapters and reduced component admission remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
No application-installation endpoint or new UI capability is enabled. Shared
component/theme ownership remains unchanged. Entered passwords/OTPs block
recovery until explicitly cleared, automatic Studio recovery stays off, and
4 MiB whole-workspace checkpoints/five-second deadlines remain unchanged.

See the [design](../cem-native-schema-capsules.md#physical-generation-census-and-checkpoint-handoff),
[native join](../../packages/cem_ml/src/runtime_admission/generation.rs),
[native fixtures](../../packages/cem_ml/tests/generation_retirement.rs),
[real-memory driver](../../packages/cem_ql/tests/owned-generation-wasm.mjs),
[command worker tests](../../packages/cem-ml-cli-npm/tests/native-retirement.test.mjs)
and [host tests](../../packages/cem-elements/src/lib/internal/runtime-support/checkpoint-domain.spec.ts).

## Verification

- All 176 selected native cases passed: 19 admission/inventory/generation cases,
  18 operation-control cases, 66 envelope/module/stylesheet cases and all 73
  original datatype/application owner cases. Four new native join cases cover
  the linked gate, complete census and private barrier. The reduced component
  WASM profile also compiles; existing unused-code/tree-sitter warnings remain.
- All 150 shared host cases, 50 command-service cases and 33 lifecycle/audit/
  factory cases passed, including real Node workers and explicit GC checks.
  Failing-first probes demonstrated the synchronous deadline and legacy-census
  retirement gaps before their repairs.
- The real-WASM owned-generation and memory-budget drivers passed with optimized
  assets: independent heaps, original source retention, stale cleanup, reentry,
  old alias/initializer refusal, immutable code reuse, disposed-memory collection,
  hard-pressure work refusal and successful bounded retirement after cleanup.
- Four selected physical retirement/ownership/resume browser stories passed
  (96 filtered), plus all 18 selected shared component cases. Isolated complete
  Studio Login recovery passed in worker/fallback modes through Nx: two cases,
  six filtered. Native compilation and metadata generation were drained before
  browser acceptance; saved projects/drafts were not cleared.
- Optimized builds and runtime packaging passed for CEM-QL, the default CLI WASM
  package, CLI hosts and cem-elements. CEM-QL is 24,139,260 bytes; the default CLI
  browser/Node WASM is 24,859,636 bytes each, below 25 MiB. Optimization preserved
  import/export descriptors. Generated assets are not committed.
- Studio typechecking, source audit/check, focused Rust formatting, local links
  and diff checks passed. Selected lint reports zero errors and existing warnings.
  Screenshots and Vitest attachments remain excluded.
