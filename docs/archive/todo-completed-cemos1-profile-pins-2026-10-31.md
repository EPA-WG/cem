# Versioned CEMOS1 profile pins — 2026-10-31

- [x] Add separately versioned operation-profile pins for
  dependency-aware source records; preserve CEMOP1/CEMNR1
  admission and independently check whole-source agreement.
  - [x] Add CEMOP2 profiles with exact CEMOS1 pins through
    shared role/shape, artifact and bounded manifest checks;
    test all operation modes and original dependency/constant
    sources, contexts, fresh owners and inert verification.
  - [x] Freeze CEMOP1 wire output and test old/new reader
    refusal, whole-source agreement beyond declared identity,
    missing/extra/duplicate/mistyped pins, compatibility,
    repinned malformed inputs, nested limits and cancellation.
  - [x] Run native/WASM/component and isolated Studio Login
    privacy acceptance; record explicit installation limits.

## Delivered contract

Native `CEMOP2` / `cem.native-operation-profiles/2` groups exact CEMOS1 pins
through the shared profile validator. Original dependency fields, constant slots,
reference contexts and source-backed result contracts stay in their original
arenas. Independent whole-source snapshot agreement precedes explicit metadata
mapping; matching declared identities cannot conceal changed source. Separate
verifications retain separate source owners, with deterministic manifest bytes
regardless of caller artifact order.

CEMOP1 remains closed to CEMNR1. Concrete aliases preserve its public pin inference,
including empty literals; the frozen v1 hash remains unchanged. No leaf relabelling,
magic swap, duplicate/missing/extra artifacts, role/shape mismatch or corrupt
repinned input grants admission. Both versions share bounds and ordinary checks;
CEMOP2 independently checks complete dependency snapshots before source sharing.

See the [native design](../cem-native-schema-capsules.md#versioned-cemos1-profile-pins),
[shared validator](../../packages/cem_ql/src/native_registrations/profiles/shared.rs),
[v2 entry points](../../packages/cem_ql/src/native_registrations/profiles/v2.rs),
[new fixtures](../../packages/cem_ql/tests/native_registrations/profile_pins.rs) and
[v1 wire/ownership fixtures](../../packages/cem_ql/tests/native_registrations/dependency_profiles.rs).

## Verification

- Eight new v2 cases and one frozen v1 compatibility case passed after native
  failing-first fixtures; all 57 registration cases passed.
- Final native regression: 657 distinct cases passed, comprising 459 integration
  cases and 198 core/shared traversal cases; WASM-target compilation passed.
- All 18 selected component browser cases, ten audit/factory tests and both
  independent-heap WASM drivers passed.
- Isolated complete Studio Login recovery passed through Nx in worker and
  fallback modes: two selected cases, six filtered. Native builds were drained
  before browser acceptance; saved projects and drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff
  checks passed. Existing component (11,474,094 bytes) and full (24,202,382 bytes)
  WASM assets were exercised, not rebuilt. Generated screenshots and Vitest
  attachments are excluded.

This is native-only inert metadata. Selection and executable-closure flags remain
false. No application/component JavaScript, native runtime admission, WASM endpoint
or receiver publication was added. Entered credentials still block recovery until
explicit clearing, automatic whole-Studio recovery remains disabled, and the
4 MiB checkpoint budget and five-second deadlines are unchanged.

Versioned module/profile assembly, actual-host installation, application-envelope
integration and receiver/worker transport remain in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
