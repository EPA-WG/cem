# Schema profile measurements — completed 2026-10-24

- [x] Add uncached, nonparallel `@epa-wg/cem-studio:measure:schema-assets`,
  depending on existing default output/schema-assets acceptance and aggregation
  tests. Measure both deployed profiles without changing production components,
  native APIs, host admission, budgets or recovery authority.
- [x] Add isolated native CSV command-service, native HTML source-paint and real
  Studio readonly Login editor workloads. Run five samples per profile/workload
  in 30 fresh browser processes/contexts, alternating profile order. Preserve
  native source/outcome equivalence, exact cleanup and complete nonblocking
  inventories. Record startup cancellations; refuse other transport failures.
- [x] Record per-URL raw/gzip inventory, both factory binary hashes, actual
  response-body offers, raw timing distributions, allocator live/peak, linear
  capacity and existing policy headroom. Preserve failed-run evidence, without
  reusing stale success reports, timing thresholds or inferred RSS/checkpoint
  savings. Keep serialized native/control metadata distinct from source semantics.
- [x] Add aggregation/headroom fixtures: even/odd medians, invalid/nonfinite
  observations, max(live, capacity), reservations, lifetime peak separation,
  exhausted headroom and inconsistent workload stages.
- [x] Document method, measured values, limitations and the decision to keep the
  complete default. Native initial/paint capacity saves 256 KiB; CSV activation
  increases capacity by 3,342,336 bytes over complete and costs cold startup.
  Complete optional offline gzip delivery adds 575,050 bytes. Prioritize
  project/resource-driven demand before broader optional-profile adoption or
  separate rare-command ABI/heap work.

## Verification

- Four deterministic measurement tests pass through the dedicated Nx target.
- All 30 fresh-browser measurement samples pass, with matching native outcomes,
  exact selected editor text and original memory configurations/reserves.
- All 44 shared optional-native/HTTP tests pass without skips.
- Studio schema-assets online/offline, both factory pins, 205 pack assets,
  cache isolation, native formatter execution, consecutive worker/fallback
  handoffs and credential/stale/retirement refusals pass.
- Default Studio output verification passes (218 declared assets / 313 files).
  Scoped ESLint, JavaScript syntax validation and `git diff --check` pass.
- Generated reports, screenshots and Vitest attachments remain excluded from Git.

See the [method and results](../schema-asset-profile-measurements.md) for exact
bytes, latency ranges, environment and interpretation. Entered passwords/one-time
codes continue to block recovery until cleared; automatic whole-Studio recovery
remains disabled. Existing projects and drafts were never opened or cleared.
