# Placement journey audit — 2026-11-05

The historical data-table and set-URL `Every Authored Sample` deadlines and
suggestions `Native Pointer Taps Keep Both Editors Focused` deadline remain
unattributed. Their unchanged three-file cohort passes **20/20**, and each
critical case passes alone before instrumentation. A later passing run cannot
establish the cause of the October failures. The separate pan-readiness and
provider-story corrections remain completed work.

- [x] Record source hashes before changing the three journey files.
- [x] Run the unchanged three-file cohort and all three isolated controls.
- [x] Add passive, bounded phase/failure capture without new waits or timers.
- [x] Prove capture through the original runner's real 30-second timeout.
- [x] Remove the deliberately held control and retain every original assertion,
      readiness call, native pointer/user action and URL-restoration call.
- [x] Run both traced ten-file cohorts: **35/35 pass** in each.
- [x] Run capture unit tests: **3/3 pass**.
- [x] Run both lint targets: **0 errors**, existing 818 elements warnings and
      171 component warnings.
- [x] Pass the unchanged declarative architecture guard: **59 components**, no
      legacy components or authored legacy JavaScript/TypeScript files.
- [x] Preserve the frozen presentation archive's accepted size and SHA-256.

The development-only capture retains the last **32** monotonic phase records,
a discarded-record count and a copied metadata snapshot. It records table/card
counts, rendering/error flags, URL-reader equality/history counts and pointer
type/trust/cancellation/focus state. It excludes source, input values, row text,
actual URL/hash values, DOM nodes and native ASTs. Inspectors are released after
freezing, and later updates cannot change the saved failure. Three unit cases
verify bounded ordering, copied state and unavailable-inspection fallback.

Runner `onTestFinished`/`onTestFailed` hooks preserve global-deadline failures;
ordinary play `finally` blocks freeze before cleanup, including URL restoration.
The original runner failure remains alongside the added diagnostic error.
The held-story control actually failed after **30,010.6 ms** under the unchanged
30,000 ms configuration. Its capture contains 32 records, nine discarded marks,
the final held phase and a connected fixture. That intentional control proves
diagnostic delivery; it is not a reproduction of an historical runtime cause.
It is removed from the final source.

Capture begins when the journey's play registers its hooks. A module or loader
failure before that point cannot produce this trace; retain the existing
module/network and readiness diagnostics for that earlier boundary. Absence of
a journey trace is not evidence that native rendering never started.

Component stories obtain the helper through the existing approved Storybook
preview/setup module. An initial direct helper import was rejected by the strict
architecture guard; the import now follows that contract, without widening its
allowlist. Ordinary Storybook does not load Vitest failure hooks. No shared UI
capability, runtime behavior, declaration, CSS or theme token changes.

| Critical journey | Unchanged isolated | First traced cohort | Final approved-setup cohort |
| --- | ---: | ---: | ---: |
| Data-table authored samples | 11,204.5 ms | 21,894.3 ms | 17,197.2 ms |
| Set-URL authored samples | 9,465.2 ms | 22,830.0 ms | 18,600.0 ms |
| Both providers, native mouse/touch/pen taps | 5,207.9 ms | 7,397.1 ms | 5,988.3 ms |

The broader cohort includes data-tree, table-inspector, module-URL and referrers,
HTTP, location and local-storage stories. File parallelism stays at its default;
`STORYBOOK_CEM_STORY_TIMING=1` streams scalar phase marks. Deep tree/mutation
tracing is disabled. This is a focused 35-play workload, not a fresh full
workspace gate or physical touch/pen/IME/screen-reader acceptance.

The [receipt](placement-journey-audit-2026-11-05.json) retains source pins,
environment, test names/durations, controlled-timeout capture and passing trace
tails. Full local logs/JSON remain under `/tmp/cem-placement-*` and
`/tmp/cem-journey-*`. Build prerequisites ran with the initial normal Nx target;
later focused runs used those prepared dependencies. All verification uses the
existing Chromium/Vitest runner and existing interaction/story deadlines.

To repeat the related workload after preparing dependencies, run
`STORYBOOK_CEM_STORY_TIMING=1 yarn nx run cem-elements:test --` followed by the
ten paths in the receipt's `finalCohort.files`. A failing ordinary run also retains
the bounded snapshot without enabling phase logging. On actual recurrence,
inspect the failed phase and state before designing a cause-specific regression.
The [historical record](../browser-stabilization-review.tmp.md#form-submission-lifecycle-capture-and-recurrence-follow-up-2026-10-08)
remains unchanged. No public deployment or frozen presentation material changes.
