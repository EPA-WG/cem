# Worker allocation diagnostic capture — 2026-11-04

- [x] Trace the fresh-worker allocation boundary without changing its deadline.
- [x] Add bounded, payload-free phase capture and deterministic held-readiness /
  held-inspection regressions, preserving whole-group revocation and late refusal.

The historical `worker-static-change` timeout is **not reproduced or attributed**.
The original frozen cohort's 110/111 result and later passing reruns remain in the
[historical snapshot](todo-snapshot-2026-10-23.md). Passing repeats establish no
timing repair. Recurrence capture remains a conditional item in
[the active checklist](../todo.md).

## Boundary and diagnostic changes

The shared [group allocator](../../packages/cem_ql/runtime/native-generation.mjs)
uses one cumulative five-second deadline across physical participants and final
currentness checks. Each [rendering transport](../../packages/cem-elements/src/lib/internal/runtime-support/processing-host-runtime.ts)
first waits for transport readiness, then requests native headroom inspection.
The [worker's inspection](../../packages/cem-elements/src/lib/internal/runtime-support/processing-worker.ts)
awaits WASM initialization before returning native memory-pressure status.
The old generic timeout alone cannot distinguish these waits.

The [isolated media driver](../../packages/cem-elements/src/lib/retained-media.fixtures.ts)
now observes the original Worker object and forwards its original messages and
transfer arguments unchanged. It captures creation, transport readiness, headroom
request/reply, worker/message errors, termination and group start/admission/refusal.
Retention is bounded to 16 workers, 16 events per worker and 16 group events;
overflow is counted. Snapshots contain only fixed phases, relative timings and
fresh-worker flags, with detached copies and no message/source/credential bodies.
The existing heartbeat and failure report carry that snapshot. The problematic
story emits a `cem.worker-allocation` JSON trace on the browser test log channel.

| Last observed phase on a timed-out candidate | Boundary to investigate next |
| --- | --- |
| Created, no transport-ready event | Worker startup/module loading or message delivery |
| Headroom requested, no reply | Inspection round trip, including WASM loading/initialization |
| Every headroom reply present, admission absent | Remaining group members, final guards or event-loop scheduling |

These are diagnostic boundaries, not proof of a particular network, compiler or
scheduler fault. Production allocator/transport code, the original five-second
deadline, static-preview refusal assertions, privacy policy and automatic
recovery settings are unchanged.

## Verification and remaining baseline

- The unchanged isolated Static Preview Refusals story passes both scenarios
  (3.11 seconds of test work). The instrumented isolated story also passes.
- The broader invocation passes **94/104** browser cases across checkpoint,
  source-paint and editor/provider stories (258.46 seconds). Static Preview
  Refusals passes; its exact captured group window is **279 ms**.
- The ten failures are theme-storage/popup fixture checks, all reporting
  `production theme controls not rendered: 15` against the hard-coded expected
  13. The canonical switch now includes Presentation, so each switch has six
  theme choices plus its trigger; two switches plus the writer input total 15.
  The unchanged Theme Text Storage Repeated Runtime Resume fixture from
  `ad4bcb50` reproduces the same failure independently. The diagnostic edits were
  restored afterward. The broader invocation is not claimed green or identical
  to the older frozen 111-case invocation.
- All **48** focused host/diagnostic units pass, including two controlled waits:
  no settlement at 4,999 ms, timeout at 5,000 ms, blocked-worker termination,
  prepared-peer discard, closed admission and refusal of late completion/reuse.
  Observer tests verify payload/transfer identity, private data exclusion,
  detached snapshots and bounded retention.
- All **23** shared generation-lifecycle tests pass. The runtime package build
  and lint pass (warnings remain). Existing prepared WASM was used; no native
  transformation change or fresh WASM build is claimed.
- The frozen presentation archive retains its exact size and SHA-256. No public
  deployment or presentation materials changed.

Captured broader-run timings, relative to that allocation's start:

| Fresh worker | Created | Transport ready / inspect sent | Headroom ready |
| --- | ---: | ---: | ---: |
| First | 0 ms | 52 ms | 142 ms |
| Second | 142 ms | 192 ms | 279 ms |

These are observed samples, not new performance bounds.

Commands from the repository root on Linux x64 / Node 24.16.0, with
`NX_DAEMON=false` and prepared dependencies:

```sh
yarn nx run cem-elements:test --args='src/lib/checkpoint-publication.stories.ts --testNamePattern="Static Preview Refusals"' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/checkpoint-publication.stories.ts src/lib/source-paint.stories.ts src/lib/editor-provider.stories.ts' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test:unit --args='src/lib/internal/runtime-support/processing-host-runtime.spec.ts src/lib/worker-allocation-diagnostics.spec.ts' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem_ql:test:native-generation --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:build --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:lint --excludeTaskDependencies --skip-nx-cache
```
