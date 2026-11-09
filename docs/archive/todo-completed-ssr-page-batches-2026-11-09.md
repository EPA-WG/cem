# Multi-page SSR route preflight and publication

Base commit `f7e0648a277185d9a6d730581b73ba2c00d9e2ff`.

- [x] Expand direct local page globs in sorted order before reading the
  page bodies. Keep each page's source allowance and share one original
  CPU and wall-time budget. Cap the graph at 1,024 SSR routes and 64 MiB
  of aggregate page source bytes.
- [x] Preflight every native page and effective authored base before graph
  execution. Check local HTML, readiness reports, generated maps and
  requested diagnostics report paths for aliases and source overwrites.
- [x] Stage every HTML file, readiness report and map in the existing
  publication transaction. Verify that failure on a later page leaves all
  prior output bytes intact and cancellation during commit restores
  earlier replacements.

The CLI profile requires direct, non-opaque local page imports and local
file destinations for a multi-page batch. The deterministic output
contract covers destination order, rendered HTML and render-input
identities. CPU and wall-time values in readiness reports are observations
of the particular invocation.

Verification passed 14 CLI SSR cases, 12 export cases in each adapter
profile, eight publication tests, 39 core SSR cases, 18 graph-lowering
cases and two login graph regressions. The CLI-to-Chromium suite passed
50 cases with zero external requests.
