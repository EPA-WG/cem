# Shared CEMDM2 compilation — 2026-10-31

- [x] Renew original datatype/constant selections during
  shared CEMDM2 compilation under actual contexts and owner
  leases; never consume a prior inspection as permission.
  - [x] Add shared CEMDM2 compiler admission for original
    datatype/constant selections and exact reachable profile
    owner maps; test ordered constants, inherited callbacks,
    original contexts/provenance and root-only validation.
  - [x] Exercise all operation roles/tokenizers and each
    source-backed result profile through shared compilation,
    including imported rule owners, unused exports,
    copied-source owners and late cancellation without
    retaining embedding callbacks.
  - [x] Test missing/extra/foreign/stale/retired/dropped
    owners, mutable selectors, pending contexts and grants,
    malformed literals, callback stops, bounds, unused exports
    and preservation of earlier successful compilations.
  - [x] Verify native/WASM/component regressions and isolated
    Studio Login privacy acceptance; archive this native gate
    with receiver publication and transport still pending.

## Delivered contract

`compile_datatype_assembly` uses the existing shared datatype compiler and actual
host resolver. A prior inspection grants no authority. The exact reachable
`DatatypeAssemblyOperationKey { module, datatype, role }` map supplies explicit
CEMOS1 operation/tokenizer owners, with original pin/source/declaration/scope and
actual-host lifetime checks. Unused profiles require no owner. Imported query/
native rules retain their separate explicit owner binding path.

Datatype and constant targets are renewed inside the compiler's ordinary walks.
Raw constant references and a shared native terminal-admission hook compare exact
original sequences, preserving order and duplicates before literal parsing or
interpreter callbacks. No second constant parser or replacement host is used.
The existing compiler performs literal/value/effective restriction admission;
profile shapes must match completed effective kinds/representations.

Native constant interpretation, inherited equality and imported rule validation
use their original restricted actual source contexts, with one shared query
budget. Invocation contexts are ephemeral and never stored in descriptors.
Closed native context filtering still excludes ambient inputs, callbacks and
module resolvers. Validation fetches actual source contexts afresh and does not
replay dependency selectors. A retained-vocabulary comparison limit reports a
bounded stop with original declaration provenance rather than assuming @values.

Snapshots preserve original source/result metadata and only weak operation,
tokenizer and rule owners. Retirement/drop, context changes, implementation
removal, host closure/drop and cancellation disable use. Late callback success
cannot publish a result. Failed attempts without context changes preserve prior
snapshots. Root-only typed validation remains the exposed execution API.

See the [design](../cem-native-schema-capsules.md#shared-cemdm2-compilation-with-renewed-selections),
[entry points](../../packages/cem_ql/src/structural_receiver/dependencies/assembly.rs),
[owner admission](../../packages/cem_ql/src/structural_receiver/dependencies/operations.rs),
[compiler fixtures](../../packages/cem_ql/tests/support/datatype_assembly_compilation.rs),
[profile fixtures](../../packages/cem_ql/tests/native_registrations/profile_pins.rs) and
[preceding owner-linking stage](todo-completed-cemos1-operation-owners-2026-10-31.md).

## Verification

- A failing-first native fixture preceded implementation. Thirteen new cases
  cover the compiler/owner/context boundaries, imported rules, all direct operation
  roles/tokenizers and every source-backed profile. Focused cases passed.
- Native regression: 715 distinct cases passed, comprising 517
  integration cases and 198 core/shared traversal cases. WASM-target compilation
  passed. This includes the additional datatype value-contract suite.
- All 18 selected component browser cases, ten audit/factory cases and both
  independent-heap WASM drivers passed.
- Isolated complete Studio Login recovery passed through Nx in worker/fallback
  modes: two selected cases, six filtered. Native builds were drained before
  browser acceptance; saved projects/drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff
  checks passed. Existing component (11,474,094 bytes) and full (24,202,382 bytes)
  WASM assets were exercised, not rebuilt. Generated screenshots and Vitest
  attachments are excluded.

No application/component JavaScript or WASM endpoint was added; UI/theme ownership
is unaffected. Entered passwords/one-time codes still block recovery until explicit
clearing. Automatic whole-Studio recovery stays disabled; 4 MiB checkpoints and
five-second deadlines are unchanged. Guarded root preparation/conversion/list
export, versioned application-envelope/mixed-receiver publication and runtime
transport remain in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
Portable selection and code-closure flags remain false.
