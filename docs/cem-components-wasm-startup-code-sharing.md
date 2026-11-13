# Runtime cold-start and memory measurements

Generated with `yarn nx run cem_ql:measure:browser-startup -- --runs=5 --record=code-sharing --write-doc`.
Committed per-run evidence: [cem-components-wasm-startup-code-sharing.json](cem-components-wasm-startup-code-sharing.json).
Full local observations, phase traffic, process identities and native samples:
`dist/reports/components-wasm/browser-startup-code-sharing.json`.

## Method and scope

- The Nx target stages a separate benchmark deployment from already built bindings, canonical declarations and CSS. It refuses mismatched package/core glue and binary bytes. No demo/Studio output is rebuilt or overwritten. Each observation launches a fresh Chromium process, temporary profile and isolated context. Browser/network/code caches are fresh; OS file caches and the local fixture server can be warm. Browser launch is recorded separately from navigation-to-readiness.
- 5 observations per workload/mode; tables show median [minimum–maximum], not a percentile or performance budget. Workload/mode order rotates between runs. No CPU/network throttling is applied.
- Selected entry measures the pinned compiled cart/JSON API in one dedicated worker or one main-thread heap, with the first guarded response rendered natively. It has no DOM/control host. Full controls, JSON/CSV data view and editor use the existing complete runtime and canonical components; these are separate workloads, not a reduced whole-application comparison.
- Readiness/first use use the original control/render assertions. Timings include browser automation and local HTTP overhead. Memory snapshots run outside timed intervals; they still add diagnostic work between readiness and first use. No explicit GC, heap enumeration or production instrumentation is installed.
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

| Workload | Mode | Samples | Browser launch ms | Ready ms | First use ms | Live workers ready / after | WASM requests |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| selected | worker | 5 | 59 [55–89] | 118 [110–126] | 21 [20–28] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| selected | fallback | 5 | 55 [53–61] | 100 [95–120] | 20 [20–21] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| controls | worker | 5 | 57 [54–62] | 1070 [1034–1245] | 70 [67–110] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| controls | fallback | 5 | 61 [55–64] | 1275 [812–1285] | 81 [77–104] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| data | worker | 5 | 61 [58–74] | 1127 [1047–1234] | 100 [88–105] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| data | fallback | 5 | 65 [57–67] | 1205 [727–1220] | 79 [74–100] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| editor | worker | 5 | 61 [57–73] | 1068 [1053–1154] | 162 [156–191] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| editor | fallback | 5 | 59 [57–65] | 1159 [674–1360] | 145 [139–157] | 0 [0–0] / 0 [0–0] | 1 [1–1] |

## Combined resident memory and separate native diagnostics

All sizes below are MiB. PSS includes host and native resident pages across the whole browser family. The blank-page snapshot precedes application navigation. Native counters cover only the selected entry heap or the selected fallback facade; their scope is smaller than the browser family. Worker allocator/capacity counters remain unavailable in the full host. The selected ABI exposes capacity but not allocator-live bytes.

| Workload | Mode | Blank PSS | Ready PSS | After use PSS | Main JS after use | Native capacity ready / after | Native allocator live ready / after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| selected | worker | 203.1 [202.3–203.6] | 255.2 [255.0–256.1] | 264.6 [259.0–265.8] | 1.5 [1.5–1.5] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| selected | fallback | 203.4 [201.4–204.6] | 252.3 [250.7–252.7] | 261.0 [259.5–262.3] | 1.9 [1.9–1.9] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| controls | worker | 203.6 [202.7–204.1] | 338.4 [337.3–439.3] | 342.3 [341.8–343.2] | 6.1 [6.1–6.1] | unavailable / unavailable | unavailable / unavailable |
| controls | fallback | 203.5 [202.6–204.2] | 315.9 [315.8–317.3] | 321.6 [321.1–322.9] | 5.0 [5.0–5.1] | 24.7 [24.7–25.1] / 25.6 [25.6–25.6] | 17.6 [17.6–17.6] / 17.6 [17.6–17.6] |
| data | worker | 203.3 [202.9–204.0] | 356.1 [355.5–381.0] | 385.1 [378.7–389.4] | 5.0 [5.0–5.0] | unavailable / unavailable | unavailable / unavailable |
| data | fallback | 203.6 [203.1–204.7] | 342.3 [342.0–344.4] | 376.6 [362.6–379.9] | 6.5 [6.5–6.6] | 8.1 [8.1–8.1] / 8.4 [8.4–8.4] | 2.7 [2.7–2.7] / 2.7 [2.7–2.7] |
| editor | worker | 203.7 [202.9–204.1] | 384.1 [360.7–460.3] | 397.1 [396.7–397.5] | 5.0 [5.0–5.1] | unavailable / unavailable | unavailable / unavailable |
| editor | fallback | 203.2 [202.7–203.6] | 344.8 [344.3–346.4] | 381.4 [380.6–381.9] | 5.3 [5.3–5.3] | 9.1 [9.1–9.1] / 9.1 [9.1–9.1] | 3.8 [3.8–3.8] / 3.8 [3.8–3.8] |

## Remaining measurements

Keep the [historical single-context baseline](cem-components-wasm-baseline.md) separate. These observations do not establish causation from binary-size reduction, an OOM threshold, a peak or a complete reduced live-component distribution. The measured code-sharing path acquires WASM once per workload. Measure all-capabilities/offline totals and clean npm consumers before selecting fixed budgets. Full dynamic/native link closure and optional-module ABI/loader work remain separate gates.
