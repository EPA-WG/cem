# Native publication generation enrollment — 2026-10-31

- [x] Bind an original native generation to its actual admission object; test
  owned entries, retained roots, stale/foreign drain proofs, limits, deadlines,
  cancellation and original owner loss.
  - [x] Keep the native-export audit strict while recognizing the existing
    fallible RAII admission guards before any work.
- [x] Enroll original hosts/source scopes, compilation and registry owners,
  CEMAPPD4 publications and sealed preparation lifetimes. Test callback-time
  closure during execution/type selection and fresh original source
  reimport/recompilation without transferring old handles.

## Delivered behavior

`NativeGeneration` claims an original `Arc<NativeAdmission>` once. Its owned
entry permits use the existing lock/counter, so synchronous callbacks remain
visible during closure. `claim_current` uses the linked runtime's actual gate;
trusted native embeddings can supply their own actual gate. Neither a label
nor a serialized counter snapshot establishes generation identity. Dropping
the original generation owner closes ordinary admission permanently.

The enrolled structural host owns a retained-root lease before any source or
native implementation registration. Original datatype compilations, exact
registry selections, publication guards, preparation invocations and private
receipts also retain leases until actual destruction. Closing/retiring those
objects still fences them, but retained handles keep roots counted. Existing
guarded paths count activity across selection, compilation and field/root
execution; callback-time closure withholds late output.

The ledger is bounded to 256 roots and 256 active entries. Read-only control
metadata records counts and a monotonic local revision, never tree/credential
content or replacement authority. A pollable drain closes admission with a
caller-controlled deadline capped at five seconds. Private witnesses require
the original live drain/generation, zero active/enrolled roots and the exact
revision; cancellation, expiry, owner loss and count-preserving cleanup
invalidate them. The existing full-census validation and retirement freeze
also reject any live enrolled roots under the same admission lock.

Fresh native domains reimport the original sources, register original code and
compile again. Identical pinned package bytes do not authorize foreign old
compilations or preparation receipts. Legacy unmanaged native hosts retain
their existing behavior and are explicitly outside this enrollment contract.

This is the native prerequisite for physical worker/owned-fallback enrollment,
not that physical adapter. A caller-created native admission object is not
proof of JavaScript Worker/Memory identity. Scoped drain witnesses are not the
complete native census, private retirement permit or replacement authority.
The original 29-registry census format is unchanged. Actual runtime participant
joining, atomic full-census/ownership retirement, checkpoint/journal handoff
and fresh-heap transport remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
No new endpoint or component admission is enabled; portable closure/admission
flags remain false.

See the [contract](../cem-native-schema-capsules.md#native-publication-generation-enrollment),
[original generation](../../packages/cem_ml/src/runtime_admission/generation.rs),
[resource ledger](../../packages/cem_ml/src/runtime_admission/resources.rs),
[core fixtures](../../packages/cem_ml/tests/runtime_generation.rs) and
[publication fixtures](../../packages/cem_ql/tests/support/application_generation.rs).

## Verification

- All 817 distinct native cases passed: 591 integration cases across 19 targets,
  211 core/shared traversal cases across ten runs, and 15 admission/inventory/
  generation cases across three targets. The final assembly target passed 73
  cases. Eleven new cases cover generation/enrolled-publication lifecycles.
- WASM-target compilation passed with the existing cem-ml unused-field warnings;
  native tree-sitter retains its existing unused-label warning.
- Ten audit/factory cases and both independent-heap WASM drivers passed. Existing
  component (11,474,094 bytes) and full (24,202,382 bytes) assets were exercised,
  not rebuilt.
- All 18 selected shared component browser cases passed. Isolated complete
  Studio Login recovery passed through Nx in worker/fallback modes: two cases,
  six filtered. Native compilation/metadata was drained before browser acceptance.
  Saved projects and drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff
  checks passed. The export scanner now recognizes existing first-statement
  fallible RAII guards without adding export exemptions.

No production JavaScript, component/theme implementation or credential policy
changed. Entered passwords/OTPs block recovery until explicitly cleared;
automatic whole-Studio recovery stays off, with 4 MiB checkpoints and
five-second deadlines. Screenshots and Vitest attachments remain excluded.
