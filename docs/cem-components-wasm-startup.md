# Runtime cold-start and memory measurements

Generated with `yarn nx run cem_ql:measure:browser-startup -- --runs=5 --write-doc`.
Committed per-run evidence: [cem-components-wasm-startup.json](cem-components-wasm-startup.json).
Full local observations, phase traffic, process identities and native samples:
`dist/reports/components-wasm/browser-startup.json`.

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
| selected | worker | 5 | 58 [54–72] | 109 [102–122] | 21 [19–26] | 1 [1–1] / 1 [1–1] | 1 [1–1] |
| selected | fallback | 5 | 54 [53–61] | 99 [87–113] | 24 [20–27] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| controls | worker | 5 | 53 [52–59] | 1096 [1047–1221] | 80 [76–126] | 1 [1–1] / 1 [1–1] | 2 [2–2] |
| controls | fallback | 5 | 58 [55–65] | 819 [795–883] | 99 [86–99] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| data | worker | 5 | 56 [53–58] | 1103 [1066–1143] | 90 [82–104] | 1 [1–1] / 1 [1–1] | 2 [2–2] |
| data | fallback | 5 | 56 [55–65] | 760 [720–1246] | 79 [69–99] | 0 [0–0] / 0 [0–0] | 1 [1–1] |
| editor | worker | 5 | 63 [55–67] | 1076 [1050–1128] | 155 [145–158] | 1 [1–1] / 1 [1–1] | 2 [2–2] |
| editor | fallback | 5 | 57 [54–61] | 740 [634–770] | 148 [137–162] | 0 [0–0] / 0 [0–0] | 1 [1–1] |

## Combined resident memory and separate native diagnostics

All sizes below are MiB. PSS includes host and native resident pages across the whole browser family. The blank-page snapshot precedes application navigation. Native counters cover only the selected entry heap or the selected fallback facade; their scope is smaller than the browser family. Worker allocator/capacity counters remain unavailable in the full host. The selected ABI exposes capacity but not allocator-live bytes.

| Workload | Mode | Blank PSS | Ready PSS | After use PSS | Main JS after use | Native capacity ready / after | Native allocator live ready / after |
| --- | --- | --- | --- | --- | --- | --- | --- |
| selected | worker | 203.7 [202.7–204.2] | 255.9 [255.5–256.1] | 264.8 [262.5–265.9] | 1.5 [1.5–1.5] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| selected | fallback | 203.9 [203.3–204.1] | 252.6 [251.8–253.0] | 261.2 [259.0–263.7] | 1.9 [1.9–1.9] | 1.8 [1.8–1.8] / 1.9 [1.9–1.9] | unavailable / unavailable |
| controls | worker | 203.3 [200.6–204.2] | 386.5 [382.7–411.3] | 388.0 [386.8–390.1] | 6.4 [6.3–6.8] | unavailable / unavailable | unavailable / unavailable |
| controls | fallback | 203.3 [202.9–203.4] | 315.9 [315.4–316.7] | 321.3 [320.8–321.7] | 5.1 [5.0–5.1] | 25.1 [24.7–25.1] / 25.6 [25.6–25.6] | 17.6 [17.6–17.6] / 17.6 [17.6–17.6] |
| data | worker | 203.4 [201.8–203.6] | 402.8 [402.0–482.3] | 434.4 [427.9–440.8] | 5.0 [5.0–6.4] | unavailable / unavailable | unavailable / unavailable |
| data | fallback | 203.7 [202.8–204.1] | 342.4 [342.1–343.1] | 373.8 [365.1–377.8] | 6.5 [6.5–6.6] | 8.1 [8.1–8.1] / 8.4 [8.4–8.4] | 2.7 [2.7–2.7] / 2.7 [2.7–2.7] |
| editor | worker | 203.5 [203.1–203.8] | 406.9 [405.3–428.8] | 443.5 [442.3–444.2] | 5.0 [5.0–5.0] | unavailable / unavailable | unavailable / unavailable |
| editor | fallback | 203.8 [202.8–204.1] | 345.3 [344.4–345.5] | 381.6 [381.3–382.6] | 5.3 [5.2–5.3] | 9.1 [9.1–9.1] / 9.1 [9.1–9.1] | 3.8 [3.8–3.8] / 3.8 [3.8–3.8] |

## Remaining measurements

Keep the [historical single-context baseline](cem-components-wasm-baseline.md) separate. These observations do not establish causation from binary-size reduction, an OOM threshold, a peak or a complete reduced live-component distribution. Diagnose repeated full-WASM acquisition, then measure all-capabilities/offline totals and clean npm consumers before selecting fixed budgets. Full dynamic/native link closure and optional-module ABI/loader work remain separate gates.
