# Shared host-view settlement — completed 2026-10-24

- [x] Diagnose long plaintext draft settling before changing Studio rendering or
  native source behavior. Profile the existing 200-line, 900px desktop editor
  case in an isolated Chromium session, using temporary runtime wrappers and
  CDP sampling outside tracked production files.
- [x] Replace the shared host view's repeated full-subtree search for each owner
  with a live DOM-order TreeWalker. Observe mutations while awaiting an owner;
  restart discovery after publication changes the subtree or declaration inputs,
  retain the visited-owner set, and disconnect the observer on completion or
  disposal. Perform another discovery pass to find newly activated declarations.
- [x] Add a 240-owner fixture that bounds traversal and declaration discovery
  work without a machine-dependent timing assertion. Cover a parent's updated
  child payload, inserted and replaced owners, insertion before the traversal
  cursor, and disposal during settlement.
- [x] Preserve the existing native render boundary, slice equality checks,
  declarative component composition, editor values, focus, selection and scroll
  assertions. Make no Studio code change or new UI capability; document the
  shared settlement contract in the `cem-elements` package.

## Diagnosis and temporary profiling experiment

The existing 200-line plaintext editor case produced 799 queued Studio projection
callbacks. Each callback checked eleven host views, although the runtime's slice
equality comparison suppressed almost all rendering. The trace recorded only six
`renderInstance` calls: controls, two badges, grid, source editor and expansion.
The longest measured asynchronous render stage was approximately 47ms. Native
rendering and repeated slice comparison were not the dominant delay.

The original host-view loop rebuilt a full descendant array and searched it from
the start for each unvisited owner. Across the queued callbacks, CDP attributed
15.44s of sampled inclusive time to `host-view.whenSettled()`, including 9.94s
in the function itself, 2.52s in `querySelectorAll`, and 2.79s in declaration
lookup. Slice updates accounted for approximately 0.75s. Stylesheet readiness
and connection reconciliation were another material cost.

An ephemeral Vite loader substituted a simple live TreeWalker with a final
discovery pass, without the final implementation's mutation-aware restart.
Both experiments used the unchanged 30s test deadline and the same 200-line,
900px desktop case:

| Observation | Original traversal | Temporary simple walker |
| --- | ---: | ---: |
| Entire test | 29.66s | 12.60s |
| Profiled fill and settlement interval | 24.46s | 7.81s |
| Sampled inclusive host-view settlement time | 15.44s | 0.63s |
| Native render requests scheduled | 6 | 6 |

These are diagnostic single runs with temporary instrumentation, not benchmark
distributions or measurements of the final mutation-aware implementation.
Remaining stylesheet readiness work and the browser's multiline fill behavior
were visible in the temporary-walker profile; this change does not claim to
eliminate either cost. No telemetry or profiling API was added to the runtime.

## Verification

- All fourteen shared host-view story plays pass, including the bounded traversal,
  asynchronous publication, owner replacement/insertion and disposal fixtures.
- Both unchanged Studio long-source editor cases, at 900px and 1280px, pass with
  the final mutation-aware implementation. Their combined test execution took
  20.29s; the complete focused run took 21.57s. Each case retains its 30s deadline
  and its editor geometry, value, focus, selection and scroll assertions.
- All 140 Studio workbench, navigation and recovery tests pass in one serial
  browser run (607.27s). This includes the two long-source cases again, retained
  previews, repeated worker/fallback handoffs and entered-credential refusal.
- All 801 shared runtime unit tests pass. The runtime TypeScript library build,
  Studio build and Studio typecheck pass. The final Studio output verifier
  validates 218 declared assets and 313 files. Scoped lint passes with four
  pre-existing story non-null assertion warnings; `git diff --check` passes.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery remains disabled. Checkpoint, native
allocation, transport and retirement budgets and deadlines are unchanged. The
profiling and browser fixtures used isolated test state; existing saved projects
and drafts were not opened or cleared. Generated profiles, reports, screenshots
and Vitest attachments are not part of the committed source change.
