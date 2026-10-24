# Shared Studio package demand — completed 2026-10-24

- [x] Wire explicit shared demand into Studio's invocation, command-worker,
  rendering-worker and owned helper/fallback contexts. The opt-in deployment
  supplies one exact CSV replacement grant from trusted build metadata; the
  shell declares its HTTP demand. Projects, URLs, status and checkpoints cannot
  supply grants or native owners.
- [x] Add `CemSchemaAssetDemandGroup` to the shared runtime. Managed asynchronous
  work and demand use a bounded queue; readiness follows every enrolled physical
  heap. A lazy rendering/preview heap replays demand before its first managed
  job. Failed/uncertain activation fences ordinary work, while exact cleanup
  remains possible. Explicitly closed heaps leave the cohort rather than
  accumulating through preview replacement.
- [x] Preserve each original physical identity, endpoint, native inventory and
  embedding proof. Resource capture requires the complete original participant
  cohort before cleanup, including copied/omitted-peer refusal. Fresh byte
  preparation precedes complete host binding; fresh hosts reuse staged owners.
  Changed membership and entered-credential witnesses refuse recovery.
- [x] Propagate the trusted rendering admission hook and configuration into live
  previews and fresh hosts. Existing logical roots cannot change their hook;
  distinct hooks separate worker pools. Preview HTTP reads remain restricted to
  their project resources.
- [x] Import each retained body once during fresh preparation, then activate its
  native owner without a second source-import pass. Actual two-heap coordinated
  replay retains the original five-second deadline and makes no network read.
- [x] Add fixtures for complete activation, lazy enrollment, work ordering,
  partial failure, exact cleanup, copied participants, retired endpoint removal,
  complete publication, staged-owner reuse and credential refusal. Retire raw
  fixture CJS module-cache references after each case and isolate native test
  files so completed tests do not retain heaps or compete for the handoff bound.
- [x] Extend isolated online/offline Studio acceptance with real commands and
  native invalid-CSV diagnostics. Keep the standalone loader fixture on its own
  page so it cannot borrow Studio's already-bound root/configuration.
- [x] Verify deployed executable import edges without treating preserved JSDoc
  type imports as browser dependencies. All deployment bytes remain native
  graph/module-map outputs.

## Verification

- **222 focused tests pass:** 126 shared runtime/checkpoint tests, 43 actual
  optional-native schema/HTTP tests, and 53 command/native generation tests
  (`node --expose-gc --test`, no skips).
- `cem-elements:build`, Studio default and schema-assets builds, Studio
  typecheck and output verification pass. The default graph declares **218
  assets / 313 files**.
- Clean npm consumer verification passes: **363 files**, unchanged default
  **26,341,799-byte WASM**. No partition size/memory improvement is claimed.
- Studio schema-assets acceptance passes with **205 pack files**, native/factory
  pin matching, bidirectional default/profile cache isolation and independent
  offline startup. Both online and offline Studio report **four physical
  runtimes / one explicit demand**, successful native CSV commands and retained
  `cem.csv.unclosed_quote` diagnostics.
- Original owned invocation and command-worker handoffs, and two consecutive
  rendering-worker/owned-fallback handoffs, pass with distinct generations,
  checked cleanup, stale aliases, peer/embedding coverage, original budgets and
  credential refusals.
- CLI Chromium worker/callback/cancellation/artifact verification and the
  standalone cold-offline declarative package verifier also pass, including
  missing-source and invalid/incomplete deployment refusals.
- Scoped ESLint has no errors (52 existing non-null assertion warnings).
  `git diff --check` passes. Generated screenshots, Vitest attachments and
  generated deployment/report files remain excluded from Git.

Entered passwords/one-time codes continue to block recovery until cleared.
Automatic whole-Studio recovery remains disabled. The default deployment keeps
its complete embedded catalog and creates no demand group. Next: opt-in embedded
body exclusion, then raw/gzip size, cold latency and memory/headroom measurement.
