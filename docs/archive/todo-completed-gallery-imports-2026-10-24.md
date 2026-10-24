# Completed gallery lifecycle and checkpoint-limit repairs — 2026-10-24

- [x] Diagnose broader demo-import verifier failures independently of canonical URLs without weakening browser-error or persisted-theme assertions.
- [x] Add a demo-element browser regression for queued connection callbacks after ancestor payload capture; defer rendering while disconnected and verify correct rendering after reinsertion.
- [x] Diagnose persisted page-theme switching with shared gallery chrome in combined source/package runs and add regression coverage at the responsible shared layer.
- [x] Add a processing-host regression for checkpoint-owner inventory overflow: refuse recovery while retaining ordinary stylesheet/render authority; preserve actual worker-loss, fresh-generation and terminal-retirement fences.

## Queued demo-element connections

The tree gallery's 15 errors came from `installStyles` in `cem-demo-element`.
Inserting a parent and child queues connection callbacks in that order. The CEM
parent captures its child into an inert payload before the child's queued
callback runs; the child then has a template document with no head. The shared
demo element now ignores connection callbacks while disconnected. Reinsertion
performs normal source/style/demo initialization. The regression reproduces the
platform reaction order and checks both capture and subsequent live rendering;
it failed on the original code.

## Recovery refusal and ordinary rendering

The icon-button page exceeded the 256-owner processing-host checkpoint
inventory. Its theme click reached the selection slice, but stylesheet
preparation then failed with `Stylesheet execution generation changed; restart
the load`. A recovery-coverage subscription incorrectly treated missing
coverage identity as physical runtime retirement. The host now distinguishes
bounded coverage refusal from client removal, terminal retirement and revoked
fresh admission. Worker transport loss retains its separate authority fence.
The owner limit remains 256, coverage stays refused after overflow, and the
credential policy and automatic-recovery setting are unchanged.

The unit regression enrolls 257 owners, verifies recovery snapshots remain
unavailable, and verifies ordinary stylesheet work completes on the same worker.
It failed before the repair. The import verifier also includes produced theme
component diagnostics in failure captures so these errors are visible directly.
No page-local JavaScript or component-specific runtime behavior was added.

## Verification

- All 13 demo-element behavior stories passed.
- All 62 processing-host and checkpoint-domain unit tests passed, including
  fresh-generation/worker-loss fences and drain/freeze control barriers.
- All eight shared gallery-page stories passed.
- Five focused theme-storage recovery and entered-credential refusal stories
  passed; entered passwords/one-time codes continue to block recovery.
- The 14-page targeted source/built import cohort passed unchanged error,
  declaration, page-theme, independent-example and canonical-viewer assertions:
  icon-button, tree gallery, menu-item, theme-switch, text property/gallery and
  tree property pages, each from authored and built outputs.
- The 22-task Studio build passed. Scoped lint reported no errors; existing
  non-null assertion warnings remain. Whitespace checks passed.

This is a targeted cohort, not a claim of a full-inventory import-verifier run.
Browser contexts were isolated; saved projects and drafts were preserved.
