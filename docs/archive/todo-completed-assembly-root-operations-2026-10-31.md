# Guarded assembly root operations — 2026-10-31

- [x] Add guarded root preparation, conversion and list
  serialization invocations for compiled assemblies; preserve
  original operation/rule contexts, shared budgets, owner
  checks and decoded span/diagnostic provenance.
  - [x] Add guarded root preparation/conversion/list export
    entry points on compiled assemblies; test root-only,
    legacy-format and missing-capability refusal without invoking other paths.
  - [x] Thread actual operation/rule contexts through shared
    inherited and checked preparation, list items, conversion
    and serialization; test decoded spans, diagnostics,
    empty lists, pre-validation and cumulative/query budgets.
  - [x] Test missing/stale/retired/dropped owners, foreign or
    closed hosts, reentry, late stops and callback retirement;
    preserve native/source-backed adapters and weak ownership.
  - [x] Run native/WASM/component and isolated Studio Login
    privacy acceptance; archive the completed native gate.

## Delivered contract

`FreshDependencyDatatypeCompilation` now exposes synchronous native
`prepare_lexical`, `convert` and `serialize_list` methods for CEMDM2 roots. A shared
root execution guard covers these APIs and validation, checking the actual host,
source epoch, exported root, compilation state and all required weak owners before
and after work. One reentry lock covers every operation and releases on failures.
A late stop or owner retirement withholds the report; stale output cannot escape.
CEMDP1/CEMDM1 retain their existing typed validation boundary.

Execution delegates to shared preparation, conversion, validation and serialization
engines. Every inherited/checked preparation step and list item's preparer uses its
original restricted source context. Conversion, serialization, rules and equality
also use their own original source contexts, with one query budget for the call.
Closed context filtering is unchanged; no runtime context is retained in a report
or descriptor. The existing sealed CEMDT1 preparation handoff remains unchanged.

Preparation preserves cumulative limits, decoded UTF-8 spans and checked
replacement compatibility. Conversion validates without invoking preparation.
Serialization validates typed list/item constraints before calling the selected
serializer and bounds its output. Empty validated lists may produce empty text.
Shared completion/stop semantics and candidate/diagnostic attribution are retained;
ordinary rejected prepared/converted values remain inspectable, while publication
still requires accepted == Some(true). No alternate operation is activated when a
capability is missing.

One call is one bounded invocation; independent calls start fresh allowances.
These reports grant no portable preparation receipt, code-closure or receiver
publication authority. Versioned application-envelope admission, atomic installation
and mixed-receiver/runtime transport remain separate gates.

See the [design](../cem-native-schema-capsules.md#guarded-assembly-root-operations),
[root entry points](../../packages/cem_ql/src/structural_receiver/dependencies/compilation/root_invocations.rs),
[fixtures](../../packages/cem_ql/tests/support/datatype_assembly_root_ops.rs),
[legacy refusal](../../packages/cem_ql/tests/datatype_module_runtime.rs),
[source-backed profile fixtures](../../packages/cem_ql/tests/native_registrations/profile_pins.rs)
and [preceding compiler stage](todo-completed-cemdm2-compilation-2026-10-31.md).

## Verification

- A failing-first native fixture preceded implementation. Nine new assembly cases
  and one legacy-format case cover source contexts, checked replacements, decoded
  spans, empty lists, original rules, diagnostic attribution, completion states,
  cumulative/query budgets, reentry and lifecycle/late-result guards. Existing
  source-backed conversion coverage now also executes the guarded root API.
- Native regression: 725 distinct cases passed, comprising 527
  integration cases and 198 core/shared traversal cases. WASM-target compilation passed.
- All 18 selected component browser cases, ten audit/factory cases and both
  independent-heap WASM drivers passed.
- Isolated complete Studio Login recovery passed through Nx in worker/fallback
  modes: two selected cases, six filtered. Native builds were drained before
  browser acceptance; saved projects/drafts were not cleared.
- Source audit, focused Rust formatting, local documentation links and diff checks
  passed. Existing component (11,474,094 bytes) and full (24,202,382 bytes) WASM
  assets were exercised, not rebuilt. Generated screenshots and Vitest attachments
  are excluded.

No application/component JavaScript or WASM endpoint was added. Shared UI/theme
ownership is unaffected. Entered passwords and one-time codes still block recovery
until explicit clearing. Automatic whole-Studio recovery remains disabled; 4 MiB
checkpoints and five-second deadlines are unchanged. Remaining publication work is
in the [active TODO](../todo.md#component-application-wasm-profile-separate-target).
