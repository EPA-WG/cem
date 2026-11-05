# Held-worker DOM stylesheet timeout — 2026-11-05

- [x] Attribute the existing DOM Stylesheet Adoption / Readiness And Native
  Diagnostics timeout without changing readiness assertions or deadlines.
- [x] Add bounded, payload-free phase/worker failure capture and exercise the
  attributed boundary through the existing real-worker integration story.
- [x] Verify disposal readiness before the held reply, then drain that late
  reply, require termination and refuse late rendering/style installation.

## Diagnosis

The unchanged story fails at its original 30,000 ms test deadline. With only
failure capture added, it still fails after releasing the held CSS request.
The capture reports declaration settlement, pending render/hydration, no rendered
paragraph or managed style, and this worker sequence:

```text
ready → hold declaration CSS → release → compile success → hold declaration CSS
```

The second compile is the existing processing host's stylesheet `begin`
preflight. `stylesheet()` supplies the accepted compile input to `submit()`,
which awaits that compile before sending the stylesheet request. This preserves
artifact reacquisition after bounded worker-cache eviction. It is separate from
a second authored declaration or instance stylesheet batch. Holding every CSS
compile in the fixture blocks this preflight indefinitely; calling `release()`
once only forwards requests already held at that instant.

This attributes the current reproducible timeout. The earlier ownership-work
baseline reproduced the same timeout with both the unchanged `6f1c8dca` registry
and its replacement/collection repair. Those historical results remain recorded
in the [October checklist snapshot](todo-snapshot-2026-10-23.md); this fixture
repair does not change their runtime ownership implementation.

## Repair and retained contract

`HeldCssWorker.release()` now opens its gate before forwarding the initial held
requests. Later CSS preflights pass through to the real worker and native WASM.
Release is idempotent. The original held-request count assertion still requires
exactly one initial batch, and additional assertions require later preflights
and exact equality with the original compile input. This equality check observes
fixture transport only; it neither parses CSS nor authorizes production reuse.

The original pending declaration/render/hydration checks, retained server node,
second instance, disconnect/reconnect, settled promises, rendered text, computed
`--adopted` value, single managed style and all three native/runtime diagnostic
codes remain intact. The 15-second request wait and 30-second test deadline are
unchanged. Failure/disposal/no-style and native output-parity stories remain in
the verification cohort.

The first six-file cohort after opening the gate passes 43/44 and exposes an
immediate `terminated === true` assumption in the disposal story. The production
host's scope listener starts asynchronous `dispose()`, which awaits admitted
`hostJobs` before releasing its worker lease. The fixture itself still holds
that admitted compile reply. Declaration/render readiness cancels on scope
disposal independently of physical worker cleanup. The disposal lane now first
awaits those same readiness promises and asserts no style or rendered paragraph,
then releases the late compile, waits for termination and asserts both absences
again. This explicitly tests the late-response refusal and drain boundary; it
does not require synchronous termination or change production disposal.

Failure capture uses the test runner's original failure hook and reports only
phase, readiness booleans, DOM counts and up to four workers with at most 32
transport events each. It exposes no source, payload, URLs or identity strings.
A test-finished hook disposes the scope even when the play is still awaiting a
promise at timeout; ordinary Storybook does not import the test hooks.

## Verification

- Before repair: the unchanged focused readiness case times out at 30,002 ms;
  failure capture reproduces it at the released-CSS phase with a second held
  declaration compile.
- After the gate repair: the focused readiness case passes (316 ms of test work)
  with matching preflight-input assertions.
- After aligning the disposal drain: all three DOM-adoption stories pass
  (1.25 seconds of test work).
- Final sequential stylesheet cohort: **44/44** across six files pass
  (20.24 seconds total; 16.17 seconds of test work), including real-worker and
  fallback native output, installation, registry, instance and runtime lanes.
- Package-wide lint passes with zero errors and the existing 818 warnings.
- `git diff --check` passes. The frozen presentation remains 17,321,262 bytes,
  SHA-256 `bbcb37da0a36491e5c021a5cf0c67304cc41f001a98247fc5c101ed6c45a9864`.

Commands from the repository root, Linux x64 / Node 24.16.0,
`NX_DAEMON=false`, with current prepared dependencies:

```sh
yarn nx run cem-elements:test --args='src/lib/dom-stylesheet-adoption.stories.ts --testNamePattern="Readiness And Native Diagnostics" --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:test --args='src/lib/dom-stylesheet-adoption.stories.ts src/lib/instance-stylesheet-adoption.stories.ts src/lib/stylesheet-installation.stories.ts src/lib/stylesheet-registry.stories.ts src/lib/retained-stylesheet-runtime.stories.ts src/lib/instance-stylesheet-installation.stories.ts --no-file-parallelism' --excludeTaskDependencies --skip-nx-cache
yarn nx run cem-elements:lint --excludeTaskDependencies --skip-nx-cache
```

Only the story fixture and task/evidence documentation change. Production host,
scheduler, registry, ownership, native CSS semantics, budgets and theme/token
contracts are unchanged. No component/app JavaScript or style defaults are added.
Existing prepared native/WASM artifacts are sufficient for this fixture change;
no runtime rebuild or public deployment is required. The separately tracked
fresh-allocation timeout and five Studio strict schema-definition diagnostics
are not attributed to this issue.
