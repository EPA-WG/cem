# CEM Elements Edge/SSR Gate

This records the deferred Phase 3.5 Edge/SSR processing gate for `cem-elements`.
It is intentionally separate from the current Phase 3A browser gate.

## Command

Run the narrow Phase 3.5 gate:

```bash
yarn nx run cem-elements:verify-edge-ssr
```

Run the current browser gate independently through:

```bash
yarn nx run cem-elements:verify:phase3a
```

`cem-elements:verify` aliases the current Phase 3A gate and does not invoke
`verify-edge-ssr`. Edge/SSR evidence must not become a browser-substrate release
prerequisite before Phase 3.5 is active.

## Coverage

`cem-elements:verify-edge-ssr` aggregates:

- `cem-elements:verify-substrate` for the browser substrate roundtrip path.
- `cem-elements:test:edge-ssr-unit` for the focused processing-boundary, host-envelope,
  hybrid render-state, and Node-only initial SSR and edge-update host fixtures.
- `cem-elements:test:edge-ssr` for dedicated Storybook runtime fixtures covering SSR
  hydration, hydration rejection/fallback, edge patch frames, browser-to-edge export
  policy, and hybrid edge render-state storage.

## Handoff

Phase 3.6 `@epa-wg/custom-element` adoption can consume the `cem-elements`
processing boundary only after Phase 3.5 is active and both this target and the
then-current broader package gate pass. The hydration contract serializes
`DataIslandSnapshot.sourceMapMode`, so SSR adoption must preserve source-fidelity
metadata for dev-mode snapshots or fail closed before client fallback.
