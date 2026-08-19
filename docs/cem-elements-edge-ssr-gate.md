# CEM Elements Edge/SSR Gate

This records the accepted Phase 3.5 Edge/SSR processing gate for `cem-elements`.
It is intentionally separate from the current Phase 3C browser gate.

## Command

Run the narrow Phase 3.5 gate:

```bash
yarn nx run cem-elements:verify-edge-ssr
```

Run the current browser gate independently through:

```bash
yarn nx run cem-elements:verify
```

`cem-elements:verify` chains the Phase 3A, Phase 3B, and Phase 3C browser gates. It
does not invoke `verify-edge-ssr`; the accepted Edge/SSR evidence remains an
independent opt-in lane rather than a browser-substrate release prerequisite.

## Coverage

`cem-elements:verify-edge-ssr` aggregates:

- `cem-elements:verify-substrate` for the browser substrate roundtrip path.
- `cem-elements:test:edge-ssr-unit` for the focused processing-boundary, host-envelope,
  hybrid render-state, browser export-policy boundary, and Node-only initial SSR and
  edge-update host fixtures.
- `cem-elements:test:edge-ssr` for dedicated Storybook runtime fixtures covering SSR
  hydration, hydration rejection/fallback, edge patch frames, browser-to-edge export
  policy, and hybrid edge render-state storage.

## Handoff

Phase 3.6 `@epa-wg/custom-element` adoption can consume the `cem-elements`
processing boundary because the uncached browser and Edge/SSR lanes passed
together on 2026-08-19. The hydration contract serializes
`DataIslandSnapshot.sourceMapMode`, so SSR adoption must preserve source-fidelity
metadata for dev-mode snapshots or fail closed before client fallback.

## Accepted Evidence

The 2026-08-19 closure run passed the uncached 51-dependent-task
`cem-elements:verify` aggregate and the uncached five-dependent-task
`cem-elements:verify-edge-ssr` aggregate. The browser lane retained 114 Storybook
cases and 133 unit cases; the isolated Edge/SSR lane retained 16 focused unit/host
cases, six dedicated Storybook cases, and four substrate fixtures.
