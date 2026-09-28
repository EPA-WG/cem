# Event render batching

2026-09-28. The user requested that browser events finish propagating before
rendering their accumulated state.

## Runtime contract

Each instance queues one render task. Slice listeners still capture values and
serialize event metadata immediately, while `currentTarget` is available.
Bubbling listeners, nested events, microtask follow-ups, behavior updates and
host attribute observation can all contribute to that task's snapshot.

A task boundary is required because native dispatch can run microtasks between
listeners. A microtask-only render queue could therefore project before an
ancestor receives the original event.

New requests immediately invalidate older render tokens and cancel superseded
processing jobs. `whenRenderSettled` waits for queued work as well as in-flight
rendering, stylesheet readiness and native storage jobs. Disconnected instances
skip queued work; reconnect projects current state. Later browser tasks and
initialization/form reconciliation may request additional passes.

Repeated clicks retain their latest payload and increment its revision even
when their slice value is unchanged. The active-state contract now checks stable
slice/form/payload/validation values after release and the updated event
revision. Held-key snapshot equality remains covered.

## Evidence

- The new bubbling regression failed before the scheduler change: renders ran
  inside propagation. It now observes one render after a trusted click, nested
  events, a microtask event, a slice update and a host attribute write.
- A second regression checks two synchronous clicks, queued settlement,
  disconnect and reconnect. Existing module-URL tests reject results from
  superseded and disconnected renders.
- Tests that depended on synchronous DOM projection or a fixed frame now await
  the relevant output or runtime settlement, including JSON storage and location
  initialization.
- All **348 browser stories** and **585 unit tests** pass.
- Runtime lint and typecheck pass (two existing lint warnings). Build and the
  uncached clean-package check pass.
- Action, select and theme-switch playgrounds pass from source and isolated
  package archives.
- The legacy state suite remains **3 passing / 18 failing**, with the same failed
  test names as the recorded baseline. A temporary test-owned disabled-binding
  probe improves to **6 passing / 15 failing**, and the complete native active
  test passes with the revised keyboard assertion. The probe was removed;
  component migration remains separate work.

Commands:

```sh
yarn nx run cem-elements:test
yarn nx run cem-elements:test:unit
yarn nx run-many -t lint typecheck -p cem-elements
yarn nx run cem-elements:verify-package
yarn nx run-many --targets=verify-package --projects=cem-elements --excludeTaskDependencies --skipNxCache
yarn nx run @epa-wg/cem-components:verify-playgrounds
```

Next: migrate `cem-icon-button` and `cem-menu-item` to canonical XHTML, including
explicit disabled presence, colocated stories and companion playgrounds, then
compare the legacy state failure inventory.
