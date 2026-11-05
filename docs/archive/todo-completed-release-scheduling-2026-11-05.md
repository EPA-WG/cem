# Studio release-verifier scheduling — 2026-11-05

- [x] Isolate Studio browser verification and deployment writers through Nx's
  scheduler while retaining normal concurrency for unrelated work.
- [x] Add a real Nx scheduler regression using resolved browser/writer task
  metadata; reject protected-workload overlap at parallelism four and require
  unrelated build probes to overlap.
- [x] Run cold Login source-paint and deployed workflows in one normally
  concurrent invocation, retaining original assertions, deadlines and failed-run
  evidence. Record the separate checkpoint refusal rather than claiming a green
  full release.

## Repair and scope

The resolved Studio browser targets, including inferred per-file Vitest CI
tasks, previously allowed Nx-level overlap. `--no-file-parallelism` serialized
files inside one Vitest process, but did not coordinate different Nx tasks.
Deployment determinism also regenerated the same output tree twice while
allowing other tasks to run alongside it.

The [Studio targets](../../packages/cem-studio/project.json) now mark these
workloads `parallelism: false`: whole-source and per-file browser tests,
browser-backed verifiers, and deployment/sample writers. Nx runs each protected
task alone within its invocation. Unrelated tasks keep normal concurrency;
the release does not require a global `--parallel=1` workaround. Commands,
dependencies, assertions and deadlines of existing targets are preserved; the
only added verification dependency is `test:release-scheduling`.

The [scheduler regression](../../packages/cem-studio/scripts/verify-release-scheduling.mjs)
reads the resolved project, identifies browser entrypoints and deployment writers,
then copies their actual scheduling settings into a private temporary Nx
workspace. Real Nx tasks record active markers and start/end intervals. Each
protected probe must run without any peer; two unrelated probes must overlap.
This tests scheduler behavior rather than just asserting Boolean configuration.
Temporary outputs and cache/metadata directories are removed after the run.

The protected set contains **50 workloads**, including the catalog-acceptance
subprocess parent. Its existing controlled internal profile concurrency remains
unchanged. The policy applies to **one Nx invocation**. It does not coordinate
separate Nx processes, raw Node/Vitest commands or distributed agents. The
[README](../../packages/cem-studio/README.md) documents this boundary and the
normal release commands.

## Verification

Linux x64, Node 24.16.0, `NX_DAEMON=false`; focused leaf commands use prepared
dependencies and bypass cache. No application, runtime or browser-spec source was
changed for this work.

- Before the scheduling overrides, the new functional regression failed with
  simultaneous protected probes. Afterward it passes at parallelism four for all
  **50** workloads, with the two unrelated probes overlapping.
- Deployment determinism passes: **336 files**, digest
  `75b13d65558001cd9cb7814bec35ecc9e2ed2b1817157a9db81d6880618548a4`.
- A single `run-many` at **parallelism three** requested `test:workbench` and
  `verify:login-navigation`. The Nx performance profile records **568.016 s**
  for source tests followed by **623.515 s** for deployed verification, with
  **no task interval overlap**. The profile is generated at
  `dist/reports/cem-studio/release-scheduling-live-profile.json`.
- The source target passed **243/245 tests** across three files. All five shared
  cold-source-paint cases passed with the original **30-second** bounds,
  including Login HTML and its live source. Complete live Login recovery failed
  in both worker and fallback at **capture 1** with `cem.checkpoint.limit`.
  This invocation failed and is not a green whole-source or whole-release result.
- An isolated unchanged rerun of those two complete live Login cases fails
  **2/2** at the same capture and node census (**243 other cases skipped**).
  Duration is **82.85 s**. The checkpoint refusal therefore also occurs without
  other source cases or deployed verification alongside it.
- Deployed verification passed example project copies, schema failure/repair,
  template output, Save/reload graph provenance, Login URL/tree selection,
  unchanged independent sources and **23 selected offline configurations**
  across Login and Supplier.
- Studio lint passes. Resolved metadata comparison confirms existing commands,
  inputs, outputs and dependency lists are unchanged except for adding the
  scheduler regression to `verify`; 48 existing targets change only scheduling.
- The frozen presentation archive retains its exact size and SHA-256. This work
  does not publish a new Site or replace accepted presentation materials.

```sh
yarn nx run @epa-wg/cem-studio:test:release-scheduling --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-studio:verify:determinism --excludeTaskDependencies --skip-nx-cache
NX_PROFILE=dist/reports/cem-studio/release-scheduling-live-profile.json yarn nx run-many --targets=test:workbench,verify:login-navigation --projects=@epa-wg/cem-studio --parallel=3 --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-studio:test:workbench --args='--testNamePattern="recovers the complete live Login workspace within budget"' --excludeTaskDependencies --skip-nx-cache
yarn nx run @epa-wg/cem-studio:lint --excludeTaskDependencies --skip-nx-cache
```

## Checkpoint blocker and retained timing evidence

The failing stack reaches `checkpointResumeHostWitness`'s node/depth traversal
guard. Both source cases report a shell inventory of **32,840 nodes**, exceeding
the existing **32,768-node** cap; maximum shell depth is **44**, below **64**.
The shell inventory alone exceeds the limit before adding the declaration's
visited nodes. This is a node-census refusal, not evidence of a checkpoint byte
limit or resume timeout. The next active task must attribute the retained DOM
growth/ownership and repair it without raising limits or omitting witness
coverage. The original **4 MiB** aggregate checkpoint budget, **five-second**
resume/allocation deadlines and credential refusal/privacy assertions remain
unchanged. Automatic whole-Studio recovery remains disabled.

Earlier evidence remains separate: the untouched serialized complete release
passed with `--parallel=1`; the overlapping **283-case** Studio source invocation
passed **276** with seven deadline failures, and all seven passed an isolated Nx
rerun, including complete live Login recovery on worker/fallback. Pending-build
cancellation passed three isolated runs after public navigation readiness was
ordered correctly. The typed control-source acceptance separately observed
fallback `cem.checkpoint.resume.timeout` at resume 0 while worker passed; its
unchanged isolated worker/fallback rerun passed **2/2**. See the
[control-source verification](todo-completed-control-source-adapters-2026-10-27.md#verification).

Those earlier invocation counts are not combined with today's results. The new
scheduler prevents overlapping tasks in its documented scope; neither the old
timing failures nor today's checkpoint refusal is relabeled as a fully repaired
runtime. If a deadline failure recurs under a verified non-overlapping schedule,
retain its phase evidence and diagnose it independently.
