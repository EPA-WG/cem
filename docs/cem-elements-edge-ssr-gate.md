# CEM Elements Edge/SSR Gate

This records the Phase 3.5 Edge/SSR processing release gate for `cem-elements`.

## Command

Run the narrow Phase 3.5 gate:

```bash
yarn nx run cem-elements:verify-edge-ssr
```

The package release gate includes this target through:

```bash
yarn nx run cem-elements:verify
```

## Coverage

`cem-elements:verify-edge-ssr` aggregates:

- `cem-elements:verify-substrate` for the browser substrate roundtrip path.
- `cem-elements:test:unit` for structured-clone-safe processing-boundary fixtures.
- `cem-elements:test` for Storybook runtime fixtures covering SSR hydration, hydration rejection/fallback, edge patch
  frames, browser-to-edge export policy, and hybrid edge render-state storage.

## Handoff

Phase 3.6 `@epa-wg/custom-element` adoption can consume the `cem-elements` processing boundary when this target and
the broader `cem-elements:verify` gate pass. The hydration contract serializes `DataIslandSnapshot.sourceMapMode`, so
SSR adoption must preserve source-fidelity metadata for dev-mode snapshots or fail closed before client fallback.
