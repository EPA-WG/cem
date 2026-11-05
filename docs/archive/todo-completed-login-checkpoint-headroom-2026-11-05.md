# Login checkpoint DOM headroom — 2026-11-05

- [x] Attribute the complete live Login host-witness overflow and remove
  redundant retained view data without raising the checkpoint limits.
- [x] Add bounded recovery phase node counts and a failing-first navigation
  retention regression preserving complete service descriptors and public routes.

## Cause and repair

The [release scheduling investigation](todo-completed-release-scheduling-2026-11-05.md)
preserves the non-overlapping **243/245** source result and unchanged isolated
**2/2 failures**. Worker and fallback both refused capture 1 with
`cem.checkpoint.limit`. Their shell inventories contained **32,840 nodes**
against the **32,768-node** host-witness cap; depth **44** remained below **64**.
This was not a resume timeout or evidence of exceeding the 4 MiB byte budget.

Fixture-only census experiments compared capture, handoff, save and build. The
first handoff reduced metadata rather than doubling the DOM. The subsequent
successful build added ordinary output-navigation nodes and controls. Unused
result summary/runtime identity fields accounted for only 23 nodes, so pruning
those fields was not adopted as the repair.

`workbenchNavigationState` passed complete service tree descriptors to the
declaration. These included route objects, descriptions, source paths and media
metadata which the tree never consumed, plus a redundant aggregate expansion
string. Their typed data-island representations consumed node budget alongside
the actual controls and rendered tree. The service model also remained available
for selection, listings, source resolution and export.

The [view-state boundary](../../packages/cem-studio/src/workbench.js) now copies
only each tree item's stable route key, accessible name, kind and recursive
children. It retains per-tree selection/expansion, section state, counts, actions
and the project selection used by the warning declaration. The complete service
model is unchanged. The declaration still renders every item; no diagnostic,
preview, control, source frame or build artifact is removed. No renderer or
checkpoint traversal skips are introduced.

The [navigation regression](../../packages/cem-studio/src/navigation.browser.spec.js)
extends the existing real build/output-layout test. It fails before the repair
because the retained tree contains the extra descriptor copies. After the repair
it requires exact tree presentation, verifies the service still owns original
resource IDs/paths/content types and expansion metadata, and continues the
existing generated-output selection and invalid UTF-8 preview checks.

The [recovery fixture](../../packages/cem-studio/src/recovery.fixture.js) records
only phase, node count, depth and a truncation flag at capture, sealing, resume,
save and build boundaries. It reads at most **32,769 nodes** per observation and
retains at most **16 records**. It does not record source text, attributes or
control values. The existing exact host/editor/iframe/focus/selection/storage
assertions and real capture guards remain the acceptance authority; these counts
are diagnostics, not alternate recovery admission.

## Verification

Linux x64 / Node 24.16.0, `NX_DAEMON=false`. Browser invocations run separately,
with prepared dependencies and cache bypassed.

- The new navigation retention assertion fails against the previous view-state
  function. The original live Login node refusal also reproduced with diagnostic
  census enabled before the repair.
- Focused browser run: **3/3 pass**, comprising that navigation regression and
  complete live Login recovery on both worker and fallback. **242** unrelated
  cases were skipped in this invocation; duration **143.43 s**.
- Each live recovery case retains two credential refusals, three complete fresh
  generation handoffs, twelve edits, unknown-operation refusal, exact retained
  editor/preview identity and the original checkpoint/group byte checks.
- Complete source browser suite: **368/368 pass across all nine files**, with no
  skipped tests, duration **822.56 s**. This includes the original cold-source
  paint bounds, navigation, shell, repository, file-system provider, preview,
  Feature Tour, recovery and startup checks. This is one passing source-suite
  invocation, distinct from the focused rerun and earlier failed invocations.
- Studio lint and typecheck pass. Production determinism rebuilds successfully
  twice: **336 files**, digest
  `0e0046ecbe277f24aab4ade89cb75bccf3ef33b7e6a628b2d15fef682fd6c788`.
- The frozen presentation archive retains its exact size and SHA-256. Source,
  focused and build checks use prepared dependencies; this is not a claim of a
  complete release-verifier graph or a new public deployment.

```sh
yarn nx run @epa-wg/cem-studio:test:workbench --args='--testNamePattern="recovers the complete live Login workspace within budget|renders project Build and readonly output centers"' --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-studio:test --args='--run --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run-many --targets=lint,typecheck,verify:determinism --projects=@epa-wg/cem-studio --parallel=3 --excludeTaskDependencies --skip-nx-cache
```

Runtime/native code, declarations, theme/token ownership and CSS are unchanged.
The original 32,768-node/depth-64 witness guard, 4 MiB aggregate checkpoint/group
budget, five-second resume/allocation deadlines, credential privacy/refusal and
automatic-recovery policy remain intact. Historical deadline failures retain
their separate evidence; a node-headroom repair does not attribute them.
