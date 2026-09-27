# Completed work — 2026-09-27

This records follow-up work after the [TODO snapshot](todo-snapshot-2026-09-27.md).
The [active TODO](../todo.md) remains the authority for unfinished work.

- [x] Fixture: activate staged registry connections inside queued CSS/DOM
  publication; verify cleanup ordering, empty source groups, rejection and
  activation failure recovery.
  `QueuedConnectionActivation` exercises worker and fallback hosts. Publication
  adopts the connection after the DOM patch and before deferred old-load cleanup.
  Rejected candidates release preparations, reused active handles remain live,
  and activation failure or cleanup cancellation blocks the queue until recovery.
- [x] Update legacy viewer evidence checks to read completed fixture IDs from the
  archived checklist while continuing to check open IDs in the active TODO.

Validation: 585 unit tests and 304 browser cases in each of the default and
retained-CSS lanes passed. Typecheck and lint passed, with the two existing
non-null assertion warnings. All 31 existing open TODO items were preserved;
the active checklist contains no completed entries.

## Runtime payload stylesheet replacements

- [x] Fixture: retain active runtime instance CSS during replacement imports,
  failed loads and superseded updates; verify successful replacement, clearing
  and disconnect cleanup in both host modes.
  `PayloadReplacementPreservesActiveStyles` reproduced the removed live style
  node while an import was pending. Runtime replacements now use the staged
  installer, which retains active styles until a complete replacement succeeds.
  Initial installation retains its independent-source failure behavior.
  The fixture covers worker and fallback execution, node reuse, failed imports,
  superseded late results, empty payload CSS and disconnect cancellation.

Runtime CSS/DOM joint publication remains an open integration task.

Validation: 585 unit tests and 305 browser cases in each CSS lane passed,
along with typecheck and lint (two existing non-null assertion warnings).
