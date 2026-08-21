# `@epa-wg/cem-studio`

`@epa-wg/cem-studio` is the synchronized CEM-ML browser-workbench package and
static deployment. Nx schedules its prerequisites and verification; the native
CEM-ML CLI transformation graph is the final production assembler.

The initial package slice deliberately contains only the typed bootstrap and
deployment boundary. Workbench controls, persistence, offline cache behavior,
and command execution are delivered by later Phase 6.5 checklist items.

## Package surfaces

- `@epa-wg/cem-studio` exports `mountCemStudio`, the explicit browser-command
  loader, and opt-in service-worker registration.
- `@epa-wg/cem-studio/manifest.webmanifest` exposes the generated application
  manifest.
- `@epa-wg/cem-studio/static/*` exposes the graph-emitted deployable tree.

Importing the package never registers a service worker. Hosts must call
`registerCemStudioServiceWorker()` explicitly.

## Build and verification

```bash
yarn nx run @epa-wg/cem-studio:build
yarn nx run @epa-wg/cem-studio:check
```

The source/destination module-map pair declares every JavaScript, CSS, worker,
and WASM byte copied into the static deployment. JSON manifests and the SVG icon
are explicit typed graph imports/exports. There is no post-graph production-copy
step.
