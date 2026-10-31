# Explicit CEMOS1 operation owners — 2026-10-31

- [x] Link CEMOS1 operation requirements to explicit
  actual-host owners through shared native linker guards;
  preserve original sources, contexts and weak lifetimes.
  - [x] Add explicit CEMOS1 direct/source-backed/tokenizer
    owner linking through shared native guards; test every
    mode, exact original declarations and result adapters,
    including independently mapped CEMOP2/CEMDM2 sources.
  - [x] Test missing/wrong implementations, registry/source/
    scope mismatch, pending contexts, duplicates, stale/
    closed/stopped hosts, retirement and weak callback release;
    preserve CEMNR1 refusal and unevaluated selectors.
  - [x] Verify native, WASM and component regressions plus
    isolated Studio credential-privacy acceptance; document
    the remaining fresh-selection compilation gate.

## Delivered contract

Explicit native `link_dependency_operation`, `link_dependency_source_operation`
and `link_dependency_tokenizer` methods bind sealed CEMOS1 requirements to actual
host implementations. A private requirement view reuses CEMNR1's exact native
identity checks, original source-scope readiness, weak callback/context guards,
operation registrations, tokenizer proxy and checked result adapters. CEMOS1 is
never converted to `VerifiedNativeRequirement`; existing wire admission is unchanged.

Owners install only on exact original datatype declarations, preserving ready
versus checked-replacement modes. Wrong-source and duplicate operation installs
preserve prior selections. Source-backed profiles retain original behaviors and
shared typed result admission. `is_live_on(host)` checks actual host/context and
owner/implementation liveness. Closure/drop, changed contexts, implementation
removal, cancellation and retirement invalidate use. Shared proxies reject late
callback success; retained metadata/owners/proxies cannot retain host callbacks.

CEMOP2 and CEMDM2 requirements bind against their mapped original arenas. Linking
executes no callback, evaluates no selector, prepares no occurrence and creates
no crossing grant. It proves no dependency selection or executable/code closure.
Renewed selection and explicit owner maps within shared CEMDM2 compilation, plus
versioned application-envelope/receiver publication, remain separate gates.

See the [design](../cem-native-schema-capsules.md#explicit-cemos1-operation-owners),
[entry points](../../packages/cem_ql/src/structural_receiver/native/dependency_owners.rs),
[owner fixtures](../../packages/cem_ql/tests/native_registrations/dependency_owners.rs),
[CEMOP2 fixtures](../../packages/cem_ql/tests/native_registrations/profile_pins.rs),
[CEMDM2 fixtures](../../packages/cem_ql/tests/datatype_assembly_runtime.rs) and
[preceding selection stage](todo-completed-assembly-source-selection-2026-10-31.md).

## Verification

- A failing-first native fixture preceded implementation. Nine new cases cover
  direct/source-backed operations, tokenizers, checked replacements, exact source/
  identity/registry/context admission, lifecycle invalidation, duplicate/foreign
  installs, late tokenizer cancellation, weak callback release and mapped profiles.
- Native regression: 687 distinct cases passed, comprising 489 integration
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

No app/component JavaScript or new WASM endpoint was added. UI/theme ownership
is unaffected. Entered passwords and one-time codes continue to block recovery
until explicit clearing. Automatic whole-Studio recovery stays disabled; 4 MiB
checkpoints and five-second deadlines remain unchanged. Remaining work is in the
[active TODO](../todo.md#component-application-wasm-profile-separate-target).
