# Fallback startup phase diagnosis

Most of the previously reported 419–456 ms fallback readiness increase comes
from delayed completion of Playwright's visibility assertion. The browser-side
visibility witness in the new matched visibility-only observations shifts by
19–65 ms at the median. Assertion lag after that witness shifts from 69–109 ms
to 478–509 ms. A smaller real startup difference remains; these observations do
not establish its exact cause or a fixed budget. The verified runtime loader,
hash checks, native owners and original UI assertions are unchanged.

## Matched observations

The full binary remains 24,476,684 bytes, SHA-256
`3fc8939ca8eef749b01e9a582dc642cf098ee7d7b0ef61275036f9d6d11e2f76`.
The pre-sharing JavaScript cohort is rebuilt from `f0da8977`; the sharing cohort
uses `bf89d514`. Both use the same native bytes, canonical components, styles,
Node v24.16.0, Chromium 148.0.7778.96 and Playwright 1.60.0. No Rust rebuild or
change to the native/compiler profile is involved.

There are 72 fresh-browser observations: three repetitions of controls/data/editor
in worker/fallback modes, for both JavaScript cohorts, with full API tracing and
again with visibility-only observation. Workload/mode order rotates as before;
browser contexts, profiles and network/code caches are fresh. OS/server caches
may be warm. No other browser verification or compilation runs concurrently.
This diagnoses phases; neither mode supplies ordinary latency/PSS distributions.

The table shows fallback medians from the visibility-only control. Visible and
assertion timestamps are browser milliseconds after navigation's time origin;
the lag is their per-observation difference. The original Node wall-clock
readiness time remains in each record and is a different clock origin.

| Workload | Visible before → after ms | Assertion before → after ms | Assertion lag before → after ms |
| --- | ---: | ---: | ---: |
| Controls | 714 → 769 | 760 → 1266 | 69 → 478 |
| Data | 641 → 706 | 749 → 1213 | 108 → 508 |
| Editor | 617 → 636 | 727 → 1144 | 109 → 509 |

Medians are calculated independently; differences between median columns need
not equal the median of the per-observation differences. These are three samples
per cell, with no percentile or causal claim about small changes. API tracing
also reproduces fast/slow assertion completion within the same sharing cohort:
controls are visible at 657/662/800 ms, while assertion lag is 70/482/471 ms in
the corresponding runs. DOM visibility and assertion completion are distinct.

The installed Playwright server's `waitForSelector` calls
`retryWithProgressAndBackoff`, whose intervals are `[20, 50, 100, 100, 500]`
(with an initial immediate attempt and repetition of the last interval).
Its visibility check therefore may observe a completed render almost half a
second later. This source and version are pinned in each evidence file. The
original role, name and strict locator assertions still pass in all observations;
the supplementary DOM witness does not replace their validation.

## Native and declaration phases

In fully traced fallback observations, the full-byte SHA-256 operation takes
16–24 ms, explicit compilation 24–32 ms and instantiation 2–3 ms (rounded across
all three workloads). The byte body completes before hashing; the import/export
interface check also completes before native use. There is no second full-body
acquisition. These native spans do not contain an extra 400–500 ms stall.

The older streaming initializer combines response consumption, compilation and
instantiation in one overlapping span of 66–76 ms. Its span begins before the
body completes; it cannot be subtracted directly from the sum of the newer
post-body spans as a CPU comparison. The new path adds explicit verification and
uses a full byte buffer. Import discovery, body delivery and declaration work
also vary between observations; their exact original starts/ends and served
source hashes are retained in the evidence.

For example, traced control run 1 completes the old WASM body at 259 ms and
streaming instantiation at 278 ms; its first declaration registration completes
at 290 ms. The sharing cohort completes the body at 270 ms, hashes during
273–289 ms, compiles during 289–313 ms, instantiates during 314–317 ms, and
completes the first registration at 326 ms. Its later child registrations finish
at 516 and 553 ms; the original control DOM becomes visible at 662 ms, followed
by assertion completion at 1144 ms. The main large gap follows visibility.
This example illustrates chronology, not an additive attribution of all runtime
costs: registration spans enclose async/native work and overlap other events.

## Evidence and verification

Committed records retain original phase events, WASM resource timing, script
delivery endpoints, served-source hashes, original assertion completion,
worker success counters and traffic:

- [Pre-sharing API trace](cem-components-wasm-startup-phases-before-sharing.json)
- [Sharing API trace](cem-components-wasm-startup-phases-code-sharing.json)
- [Pre-sharing visibility-only control](cem-components-wasm-startup-phases-before-sharing-observe-only.json)
- [Sharing visibility-only control](cem-components-wasm-startup-phases-code-sharing-observe-only.json)

Each record binds the complete local raw report by SHA-256. Raw observations
remain at `dist/reports/components-wasm/browser-startup-phases-<cohort>.json`.
The publication step drops diagnostic process/PSS records rather than mixing
them into the preserved [ordinary startup](cem-components-wasm-startup.md) and
[code-sharing startup](cem-components-wasm-startup-code-sharing.md) archives.
Those archives and their memory results remain unchanged. Page-side API wrappers
do not capture worker-internal native calls; fallback keeps these calls on the
page. Only scalar phase labels, times and input sizes are captured, with a
5,000-event limit and explicit overflow refusal. No payload bodies, native
handles or authored behavior are retained or replaced.

Twenty-four serial collector/diagnostic tests pass. New cases verify original
promise identity, native compile/instantiate results, rejected compilation,
hidden-control refusal, observer/wrapper cleanup, bounded capture without
dropping original calls, and publication that excludes diagnostic memory.
The ordinary controls worker/fallback smoke also passes with diagnostic wrappers
and observers absent and with its original fixture bytes unchanged.

## Reproduction

Build the current runtime normally, or refresh its JavaScript-only code assets
and rebuild `cem-elements` from the matched native receipt. Use the same binaries
for both cohorts. For the old JavaScript build, create a detached `f0da8977`
worktree, link the installed dependencies, copy the verified full native output
to its `packages/cem_ql/dist/wasm/`, regenerate its factory with that commit's
`cemQlWasmFactory` helper, then run `cem-elements:build --excludeTaskDependencies
--skipNxCache` there. This regenerates the twelve original packaged assets.

Assemble a separate historical benchmark with `prepareComponentBenchmark`:
use that old `cem-elements` output and old factory, plus current canonical
components/theme and the unchanged core WASM/glue. The helper requires a paired
`cem_ql_code.js` in the root and vendor source directories; copy the current file
to both for this packaging check. The old runtime does not import or fetch it.
The committed source-hash evidence confirms the old native runtime/factory path;
worker mode has two original transfers, fallback one. No user's demo or Studio
deployment is overwritten. The historical deployment used here is
`dist/fixtures/component-startup-before-sharing/`.

Run these commands serially from the current project root, then publish the
phase-only evidence:

```bash
yarn nx run cem_ql:measure:startup-phases -- dist/fixtures/component-startup-before-sharing --phase-cohort=before-sharing --runs=3
yarn nx run cem_ql:measure:startup-phases -- --prepare --runs=3
yarn nx run cem_ql:measure:startup-phases -- dist/fixtures/component-startup-before-sharing --phase-cohort=before-sharing --phase-observe-only --runs=3
yarn nx run cem_ql:measure:startup-phases -- --prepare --phase-observe-only --runs=3
yarn nx run cem_ql:report:startup-phases
```

The separate [browser-side fixture timing](cem-components-browser-readiness-timing.md)
now supplements the original assertion wall time and workflow validations.
Its frame-sampled predicates refuse hidden/incomplete controls, and failed
assertions cannot publish a result. Forty new observations retain both metrics;
the phase records above remain historical diagnostic evidence.
All-capabilities/offline byte totals and clean npm-consumer measurements remain
the following distribution gates. No runtime loading change is justified by the
half-second polling gap alone, and these traces cannot retroactively assign
exact phase timings to the earlier forty observations.
