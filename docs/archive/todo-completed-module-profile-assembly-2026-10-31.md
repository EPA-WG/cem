# Versioned module/profile assembly — 2026-10-31

- [x] Add separately versioned module/profile assembly for
  CEMOP2/CEMOS1, with exact original datatype/source mapping,
  bounded constant-reference dependencies, explicit imports
  and cycle/source-agreement checks; preserve CEMDM1.
  - [x] Add CEMDM2 assembly with shared datatype/leaf/order
    checks, exact profile/source mapping and original constant
    targets; test imports, ordered duplicates, chains,
    fresh ownership and inert verification.
  - [x] Freeze CEMDM1 wire behavior; test missing/extra/foreign
    profiles, source mismatches, unexported/wrong targets,
    unused imports/bindings, cycles/depth, repinned malformed
    records, aggregate/nested limits and cancellation.
  - [x] Run native/WASM/component and isolated Studio Login
    privacy acceptance; document remaining installation gates.

## Delivered contract

Native `CEMDM2` / `cem.native-datatype-modules/2` assembles original datatype
modules, exact CEMOP2 profiles and intended constant-reference targets. Shared
datatype/leaf inspection and graph ordering preserve CEMDM1's reviewed subset;
only the v2 entry point permits original constant slots. The fixed CEMDM1 wire
hash remains unchanged, and old/new readers refuse each other's input.

Imports must exactly match named datatype and constant-reference module edges.
Cross-module constants/references require explicit exports. Reference chains
retain authored order and duplicate occurrences, bounded depth/expansion and
original cycle witnesses. Complete source agreement spans modules, legacy leaves
and CEMOS1 profile requirements. Metadata mapping rechecks complete snapshots
before rebinding profiles into each module's original source arena; separate
assembly verifications allocate separate owners. Supplied profiles must all map
to reachable original datatypes exactly once.

Node-category and intended-target checks do not certify constant fields/values,
actual selector results, effective inherited kinds or code readiness. The shared
compiler alone performs those checks under actual source contexts and policies.
Selection and executable-closure flags remain false. There is no new runtime
endpoint or application/component JavaScript; actual-host installation and
application-envelope/receiver transport remain separate gates.

See the [design](../cem-native-schema-capsules.md#versioned-module-and-profile-assembly),
[assembly](../../packages/cem_ql/src/datatype_module_capsule/v2.rs),
[constant topology](../../packages/cem_ql/src/datatype_module_capsule/v2/constants.rs),
[profile inputs](../../packages/cem_ql/src/datatype_module_capsule/v2/profile_inputs.rs),
[assembly fixtures](../../packages/cem_ql/tests/datatype_module_assembly.rs) and
[v1 compatibility fixture](../../packages/cem_ql/tests/datatype_module_capsule.rs).

## Verification

- Failing-first native fixtures preceded implementation. Nine assembly cases
  and one frozen CEMDM1 wire case were added; all focused cases passed.
- Native regression: 667 distinct cases passed, comprising 469 integration
  cases and 198 core/shared traversal cases. WASM-target compilation passed.
- All 18 selected component browser cases, ten audit/factory cases and both
  independent-heap WASM drivers passed.
- Isolated complete Studio Login recovery passed through Nx in worker/fallback
  modes: two selected cases, six filtered. Native builds were drained before
  browser acceptance; saved projects/drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff
  checks passed. Existing component (11,474,094 bytes) and full (24,202,382 bytes)
  WASM assets were exercised, not rebuilt. Generated screenshots and Vitest
  attachments are excluded.

Entered credentials still block recovery until explicit clearing. Automatic
whole-Studio recovery remains disabled, with the 4 MiB checkpoint budget and
five-second deadlines unchanged. Remaining installation work is in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
