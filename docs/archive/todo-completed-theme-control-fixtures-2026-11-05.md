# Theme recovery control inventory — 2026-11-05

- [x] Replace the stale aggregate count with exact canonical theme choices,
  accessible labels/roles, trigger inventory and retained node identities.
- [x] Verify fresh Presentation selection after repeated worker/fallback handoff
  while preserving independent storage, draft, popup and refusal assertions.
- [x] Extend bounded allocation-phase capture to the text-storage/popup driver
  after an aggregate invocation exposes a `worker-method` allocation timeout.

The unchanged fixture expected 13 controls. The canonical
[theme switch](../../packages/cem-components/src/components/cem-theme-switch/cem-theme-switch.xhtml)
includes Presentation: each switch has six choices and one trigger. Two switches
plus the editor input therefore produce 15 controls. The unchanged focused story
reproduces `production theme controls not rendered: 15` before this repair.
The prior [94/104 diagnostic baseline](todo-completed-worker-allocation-diagnostics-2026-11-04.md)
remains preserved.

The [shared recovery fixture](../../packages/cem-elements/src/lib/checkpoint-publication.stories.ts)
now requires the six canonical values and labels in order, native
`menuitemradio` roles, matching declaration/control values, one labeled trigger
per switch and no extra buttons. It compares current control/editor queries to
the original node references after each unchanged handoff/refusal, retaining the
original connectivity check and intentional replaced-trigger refusal lane.

Successful lanes retain both recovery cycles, then select Presentation through
the fresh controls. When the preceding selection closes a popup, the fixture
reopens it through the retained trigger. It verifies the checked radio state,
stored value, independent second switch and original controls/draft. Existing
live subscriptions, callback/source authority, popup geometry/policy, focus and
UTF-16 selection, event-sequence and no-replay assertions remain intact.

This changes fixture code only. Runtime/component declarations, CSS, token
ownership, native semantics, privacy policy, the 4 MiB budget and the original
five-second deadlines are unchanged. The frozen presentation archive retains
its recorded size and SHA-256; there is no new deployment.

## Verification and allocation follow-up

- The focused worker/fallback repeated-resume story passes after the inventory
  correction, including Presentation selection after both recovery cycles.
- The complete ten-story theme-storage/popup matrix passes **10/10**
  (56.62 seconds), covering success, early/late storage and callback refusal,
  unowned/replaced endpoints, policy, geometry and controller changes.
- The first broader checkpoint/source-paint/editor-provider invocation passes
  **103/104** (292.03 seconds). Its sole failure is the original five-second
  `worker-method` fresh-allocation timeout, before callback mutation. It is not
  counted as green or attributed to the control inventory repair.
- The existing bounded, payload-free allocation observer now covers this driver
  as well as media recovery. Allocation has its own progress stage; the original
  error report and heartbeat retain per-worker/group phases. No retry or deadline
  extension is introduced. The isolated callback-refusal story passes both
  scenarios with this wiring (3.43 seconds of test work).
- The final broader invocation, including the text-driver phase capture, passes
  **104/104** across all three files (297.08 seconds). The earlier 103/104
  invocation remains a separate failed result; its timeout is not claimed fixed.
- Package-wide `cem-elements` lint passes with zero errors; the 818 warnings
  remain at the prior baseline.

The allocation recurrence remains conditional in [the active checklist](../todo.md).
An isolated pass or a later green cohort does not establish its cause or a
timing repair. Existing prepared native/WASM artifacts are used; no runtime
rebuild is required for this story-only change.

Commands from the repository root with Linux x64 / Node 24.16.0,
`NX_DAEMON=false` and prepared dependencies:

```sh
yarn nx run cem-elements:test --args='src/lib/checkpoint-publication.stories.ts --testNamePattern="Theme "' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/checkpoint-publication.stories.ts --testNamePattern="Theme Text Storage Callback Refusals"' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/checkpoint-publication.stories.ts src/lib/source-paint.stories.ts src/lib/editor-provider.stories.ts' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:lint --excludeTaskDependencies --skip-nx-cache
```
