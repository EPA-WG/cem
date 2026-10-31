# Actual-host assembly scopes and renewed selections — 2026-10-31

- [x] Bind CEMDM2 to actual fresh-host scopes/imports and
  renewed ordinary datatype/constant selections, comparing
  exact ordered original targets without fabricating a host
  or treating declared imports as crossing grants.
  - [x] Add actual-host CEMDM2 scope/occurrence registration
    and shared resolver selection; test original datatype
    and constant targets, reference chains/order/duplicates,
    explicit name imports and per-hop crossing grants.
  - [x] Test raw/copied/foreign/extra scopes, pending contexts,
    changed/intermediate/wrong-order targets, unused exports,
    stop/depth/work limits, stale/closed/retired results and
    weak callback ownership without compiler installation.
  - [x] Run native/WASM/component and isolated Studio Login
    privacy acceptance; archive the completed inspection gate.

## Delivered contract

CEMDM2 now has explicit native actual-host registration, original lexical-scope
preparation and selection entry points. They borrow the verified graph directly
and share CEMDP1/CEMDM1's actual resolver, datatype name catalog and dependency
walker. Constant-only libraries preserve their original schema scope. Captured
source identity, caller-provided source/occurrence contexts, explicit name imports
and directed per-hop grants remain mandatory.

Reachable datatype constant slots use the shared owned-reference traversal.
Each reference must select its exact original raw target sequence; each datatype
must resolve its intended flattened terminal sequence. Order, duplicates and
arena identity remain significant. Skipping an intermediate reference refuses,
even if the final constants match. Missing intermediate contexts or grants cannot
be replaced by root defaults/grants. Unused exports require no prepared selectors.

Every attempt renews mutable providers. Datatype and constant walks share a
remaining traversal work allowance; separately bounded metadata planning,
destination limits, depth and operation cancellation still apply. Sealed local
inspection counts retain immutable metadata and a weak context lease, without
retaining callbacks. Changed/closed/stopped hosts or retired results disable use.
An older live inspection never authorizes later compilation or publication.

Wire formats and portable selection/code-closure flags remain unchanged.
CEMOS1 operation owners, constant interpretation/literal admission, shared
compiler installation and versioned envelope/receiver transport remain pending.
No application/component JavaScript, new WASM endpoint or credential export path
was added. Shared UI and theme contracts are unaffected by this native-only gate.

See the [design](../cem-native-schema-capsules.md#actual-host-assembly-selection),
[actual-host adapter](../../packages/cem_ql/src/structural_receiver/dependencies/assembly.rs),
[constant adapter](../../packages/cem_ql/src/structural_receiver/dependencies/constants.rs),
[fixtures](../../packages/cem_ql/tests/datatype_assembly_runtime.rs) and
[preceding assembly stage](todo-completed-module-profile-assembly-2026-10-31.md).

## Verification

- Failing-first native fixtures preceded implementation; all 11 new actual-host
  assembly cases passed, along with the existing module and assembly cases.
- Native regression: 678 distinct cases passed, comprising 480 integration
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

Entered passwords and one-time codes continue to block recovery until explicit
clearing. Automatic whole-Studio recovery stays disabled; the existing 4 MiB
checkpoint budget and five-second deadlines are unchanged. Remaining installation
work is in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
