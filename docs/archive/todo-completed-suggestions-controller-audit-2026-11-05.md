# Suggestions controller fixture ownership audit — 2026-11-05

- [x] Recheck both historical suggestions-controller failures with their original
  assertions and deadlines.
- [x] Add bounded, payload-free preparation/publication and story failure capture;
  verify the capture with a temporary missing-row-map control.
- [x] Reproduce and correct controller-replacement freshness aliasing in the host
  fixture; retain the native query revision and verify the related browser gate.

## Current baseline and attribution limit

Before changing code, the two historical cases pass **2/2** in fresh Chromium,
covering both production field providers and worker/fallback paths as originally
authored. The complete unchanged controller file passes **15/15**. History review
finds no evidence establishing which change caused those old failures to stop
reproducing. The October 20 visibility and missing-map failures remain historical
failed results; a passing repeat is not their causal explanation.

A controlled late-preparation case does expose a separate, concrete ownership
bug in the shared story fixture. Its query revisions restart when a replacement
controller is admitted. The fixture formerly assigned `latest = queryRevision`
and used equality with that number to qualify native publication and row leases.
Old and replacement controllers can therefore both use revision `1`.

The new regression holds the original controller's initial preparation before
native publication, admits a trusted replacement with its own revision `1`, and
waits for three committed native rows. Releasing the old preparation formerly
publishes over the replacement and invalidates its row map. The failure capture
shows two distinct attempts with revision `1`, replacement publication and rows,
then late original publication/render and invalidation. This fails at the
unchanged-map assertion before opening the replacement.

The fixture now uses a monotonic preparation token across controller lifetimes
for all host freshness predicates. The actual native query revision remains the
controller's original value. The stale preparation is refused before it can
publish; the admitted binding remains identical and current, and the replacement
still opens normally with no controller errors. Both field providers run this
case in real worker and fallback modes inside a modal parent.

This establishes the cause and correction for the controlled fixture race. It
does not prove that this race caused either historical October failure, because
those runs lack an equivalent trace. The active checklist retains conditional
recurrence capture for both original cases.

## Failure capture

The fixture keeps the last 32 phase records, containing only phase names,
preparation/query counters and row counts. Story context records the canonical
field name, host mode and current phase. Snapshots include controller presence,
pending state, already-emitted feedback state, editor/panel connectivity and
focus/open attributes, error count and at most eight diagnostic codes. Parent
capture records native dialog connectivity/open/modal state.

Snapshots are frozen before cleanup so a failure report retains the relevant
state. Inspection reads stored observations and DOM; it does not call controller
visibility/feedback or native freshness getters, which can reconcile authority.
No query text, source bytes, row labels/values, URLs, native handles or source/
policy identities are printed. Test-only failure hooks add context to the
original runner error without racing its promise, changing deadlines or loading
Vitest hooks in ordinary Storybook.

A temporary missing-map fault deliberately fails both original stories and
produces the new bounded reports. It reports initial visibility/focus failure
with a connected focused editor, a live modal parent where applicable, and
published → render-requested → render-settled → row-map-missing → prepare-failed.
The fault is removed before the final verification; no fault injection remains
in committed source.

## Verification

- Unchanged original cases: **2/2** pass; unchanged full controller file:
  **15/15** pass.
- Temporary missing-map diagnostic control: **2 expected failures**, both with
  preserved original failures and bounded capture. These are not counted green.
- New replacement race before the freshness correction: **1 expected failure**
  at late initial preparation; after correction, all four field/host scenarios
  pass in the focused story (2.28 seconds of test work).
- Final related browser cohort: **38/38 across seven files**, 22.76 seconds total
  (17.59 seconds of test work). This covers native suggestions, editor providers,
  placement grants, manual listboxes, surfaces, platform close, all controller
  stories and the new ownership case.
- Package-wide lint passes with zero errors and the existing **818 warnings**.
- `git diff --check` passes. Frozen presentation SHA-256 remains
  `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Commands from the repository root with Node 24.16.0, `NX_DAEMON=false` and prepared
native/WASM dependencies:

```sh
yarn nx run cem-elements:test --args='src/lib/suggestions-controller.stories.ts --testNamePattern="Production Fields External Sources And Trusted Ancestor Resume|Platform Close Cancels Pending And Preserves Editor Routing" --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/suggestions-controller.stories.ts --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/suggestions-controller.stories.ts --testNamePattern="Replaced Controller Cannot Publish Late Initial Rows" --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/suggestions-controller.stories.ts src/lib/suggestions-editor.stories.ts src/lib/suggestions-placements.stories.ts src/lib/native-suggestions.stories.ts src/lib/manual-listbox.stories.ts src/lib/native-surface.stories.ts src/lib/platform-close.stories.ts --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:lint --excludeTaskDependencies --skip-nx-cache
```

Only story fixture orchestration and task/evidence documentation change.
Production controller, renderer, native publication/session/placement code,
component declarations, CSS and token ownership, source identity, privacy policy,
checkpoint budget and original test deadlines are unchanged. No runtime timing
repair, new public deployment, physical Back/touch/pen/IME or spoken AT acceptance
is claimed. The native per-controller query revision remains intact.
