# Runtime cold-start and memory measurements

Generated with `yarn nx run cem_ql:measure:browser-startup -- --runs=5 --record=browser-readiness --write-doc`.
Committed per-run evidence: [cem-components-wasm-startup-browser-readiness.json](cem-components-wasm-startup-browser-readiness.json).
Full local observations, phase traffic, process identities and native samples:
`dist/reports/components-wasm/browser-startup-browser-readiness.json`.

## Method and scope

- The Nx target stages a separate benchmark deployment from already built bindings, canonical declarations and CSS. It refuses mismatched package/core glue and binary bytes. No demo/Studio output is rebuilt or overwritten. Each observation launches a fresh Chromium process, temporary profile and isolated context. Browser/network/code caches are fresh; OS file caches and the local fixture server can be warm. Browser launch is recorded separately from navigation-to-readiness.
- 5 observations per workload/mode; tables show median [minimum–maximum], not a percentile or performance budget. Workload/mode order rotates between runs. No CPU/network throttling is applied.
- Selected entry measures the pinned compiled cart/JSON API in one dedicated worker or one main-thread heap, with the first guarded response rendered natively. It has no DOM/control host. Full controls, JSON/CSV data view and editor use the existing complete runtime and canonical components; these are separate workloads, not a reduced whole-application comparison.
- Readiness/first use use the original control/render assertions. Timings include browser automation and local HTTP overhead. Memory snapshots run outside timed intervals; they still add diagnostic work between readiness and first use. No explicit GC, heap enumeration or production instrumentation is installed.
- Browser fixture ms is a separate renderer-clock measurement: complete fixture predicates hold for two consecutive animation frames with the same nodes and are rechecked after original assertions. Hidden/inert/disabled/incomplete controls refuse. Sampling stops before memory snapshots, and every original first-use assertion still gates publication. Frame cadence and renderer work limit precision; this is neither a paint timestamp nor general application readiness. Selected-entry timing samples its original native-initialization flag; it has no control DOM. Assertion wall ms preserves the original Node-clock measurement, including selector polling; the clock origins and confirmation criteria differ.
- Combined resident memory is the sum of Linux `smaps_rollup` **PSS** for the browser and its surviving descendants, including renderer/worker/WASM, GPU and network processes. PSS apportions shared resident pages ([kernel definition](https://docs.kernel.org/filesystems/proc.html)). Process identities and membership must match before/after each snapshot; missing/unstable counters are unavailable. Sampling spans multiple reads, not an atomic peak or an application-only allocation total. Node/Playwright/server memory, detached non-descendants and explicit huge pages outside PSS are excluded. Per-process private bytes, swap PSS and huge-page counters remain in the raw report.
- Main-thread JS heap and available native allocator/linear-memory capacity are separate diagnostics. **Do not add them to PSS** or treat reserved WASM capacity as resident/allocated bytes. Worker count means live dedicated workers observed by Playwright, separate from creation/job counters and OS processes.
- The local server uses no-store responses. Decoded and wire bodies, unique assets and repeated transfers are recorded separately against served-file/inventory hashes. No compressed-CDN, offline/all-capabilities or isolated npm-consumer claim is made.

Chromium: 148.0.7778.96; Node: v24.16.0; platform: linux 6.18.58.1-microsoft-standard-WSL2 x64.

| Artifact | SHA-256 |
| --- | --- |
| Full reference WASM | `3fc8939ca8eef749b01e9a582dc642cf098ee7d7b0ef61275036f9d6d11e2f76` |
| Selected cart WASM | `2fbe29b66fdd64c55b9b05eb0f2f8d29ef56aa6cf99444cbdb6dfab8a3d51600` |

## Repeated observations

Times are milliseconds. Worker counts are live dedicated workers, not native heaps or OS process counts.

| Workload | Mode | Samples | Browser launch ms | Assertion wall ms | Browser fixture ms | First use ms | Live workers ready / after | WASM requests |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- |
| selected | worker | 5 | 95 [77–102] | 142 [127–154] | 154 [140–162] | 45 [43–55] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| selected | fallback | 5 | 94 [63–96] | 154 [114–182] | 149 [111–178] | 49 [32–86] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| controls | worker | 5 | 101 [62–116] | 1316 [1235–1714] | 1183 [1098–1257] | 97 [75–123] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| controls | fallback | 5 | 74 [64–111] | 1103 [929–1580] | 1018 [918–1087] | 116 [89–193] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| data | worker | 5 | 71 [60–94] | 1152 [1130–1206] | 943 [852–1032] | 113 [98–117] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| data | fallback | 5 | 65 [58–66] | 1365 [874–1396] | 876 [857–899] | 172 [122–240] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| editor | worker | 5 | 74 [71–146] | 1198 [1078–1220] | 998 [825–1053] | 176 [171–206] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| editor | fallback | 5 | 73 [64–75] | 1250 [766–1471] | 789 [737–962] | 185 [167–219] | 0 [0–0] / 0 [0–0] | 1 [1–1] |

## Combined resident memory and separate native diagnostics

All sizes below are MiB. PSS includes host and native resident pages across the whole browser family. The blank-page snapshot precedes application navigation. Native counters cover only the selected entry heap or the selected fallback facade; their scope is smaller than the browser family. Worker allocator/capacity counters remain unavailable in the full host. The selected ABI exposes capacity but not allocator-live bytes.

| Workload | Mode | Blank PSS | Ready PSS | After use PSS | Main JS after use | Native capacity ready / after | Native allocator live ready / after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| selected | worker | 205.4 [204.4–205.8] | 257.8 [257.0–258.5] | 266.2 [264.0–267.9] | 1.6 [1.6–1.6] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| selected | fallback | 206.1 [205.6–206.3] | 255.0 [254.9–255.6] | 263.6 [261.4–265.0] | 2.0 [2.0–2.0] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| controls | worker | 205.3 [204.8–205.8] | 362.2 [340.0–440.1] | 345.2 [343.8–345.7] | 6.1 [6.1–6.2] | unavailable / unavailable | unavailable / unavailable |
| controls | fallback | 205.9 [204.6–206.1] | 318.6 [318.0–319.4] | 324.1 [323.0–324.3] | 5.1 [5.1–5.1] | 24.7 [24.7–25.1] / 25.6 [25.6–25.6] | 17.6 [17.6–17.6] / 17.6 [17.6–17.6] |
| data | worker | 205.6 [205.3–206.1] | 454.2 [358.6–461.9] | 388.9 [381.5–397.2] | 5.1 [5.0–5.1] | unavailable / unavailable | unavailable / unavailable |
| data | fallback | 206.1 [204.8–206.7] | 346.7 [345.3–445.2] | 375.2 [371.7–378.4] | 6.6 [6.5–6.6] | 8.1 [8.1–8.1] / 8.4 [8.4–8.4] | 2.7 [2.7–2.7] / 2.7 [2.7–2.7] |
| editor | worker | 205.0 [204.3–206.4] | 462.8 [387.2–538.3] | 397.3 [392.2–400.3] | 5.1 [5.0–5.1] | unavailable / unavailable | unavailable / unavailable |
| editor | fallback | 205.9 [205.4–206.3] | 348.8 [347.3–477.7] | 380.0 [373.4–383.7] | 5.3 [5.3–5.3] | 9.1 [9.1–9.1] / 9.1 [9.1–9.1] | 3.8 [3.8–3.8] / 3.8 [3.8–3.8] |

## Remaining measurements

Keep the [historical single-context baseline](cem-components-wasm-baseline.md) separate. These observations do not establish causation from binary-size reduction, an OOM threshold, a peak or a complete reduced live-component distribution. The measured code-sharing path acquires WASM once per workload. Measure all-capabilities/offline totals and clean npm consumers before selecting fixed budgets. Full dynamic/native link closure and optional-module ABI/loader work remain separate gates.
