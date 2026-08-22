# `@epa-wg/cem-studio`

`@epa-wg/cem-studio` is the synchronized CEM-ML browser-workbench package and
static deployment. Nx schedules its prerequisites and verification; the native
CEM-ML CLI transformation graph is the final production assembler.

The package includes the typed bootstrap/deployment boundary, installable
component shell, versioned local project repository, and graph-owned offline
deployment. Workbench operations are delivered by later Phase 6.5 checklist
items.

## Package surfaces

- `@epa-wg/cem-studio` exports `mountCemStudio`, the explicit browser-command
  loader, opt-in service-worker registration, and the IndexedDB repository
  factory.
- `@epa-wg/cem-studio/repository` exposes the same repository contract without
  importing the application bootstrap. A CEM-ML project validator is mandatory;
  import validation completes before a write transaction, and export validation
  rechecks the normalized project and every resource hash.
- `@epa-wg/cem-studio/shell` exposes the production CEM-component shell, five
  theme modes, browser install state, and explicit safe-update coordinator.
- `@epa-wg/cem-studio/manifest.webmanifest` exposes the generated application
  manifest.
- `@epa-wg/cem-studio/static/*` exposes the graph-emitted deployable tree.

Importing the package never registers a service worker. Hosts must call
`registerCemStudioServiceWorker()` explicitly.

The static application entry registers that worker, while the public package
bootstrap remains side-effect free. Cache inventory v2 keeps versioned shell,
runtime, and sample assets separate; mutable projects remain in IndexedDB.

The repository implements the logical `studio-projects` port from
`@epa-wg/cem-elements`. Its physical database/stores remain private to Studio;
callers use versioned clone-safe `query` and `execute` envelopes. Project saves,
imports, trash, and restore are strict multi-store transactions with expected
revision checks, SHA-256 content addressing, a durable change journal,
`BroadcastChannel` invalidation, and derived deterministic search documents.
Search and storage status render through CEM components when the workbench shell
is composed; persistence does not introduce application-local visible controls.

## Build and verification

```bash
yarn nx run @epa-wg/cem-studio:build
yarn nx run @epa-wg/cem-studio:test:repository
yarn nx run @epa-wg/cem-studio:test:shell
yarn nx run @epa-wg/cem-studio:check
```

The source/destination module-map pair declares every JavaScript, CSS, worker,
and WASM byte copied into the static deployment. JSON manifests and the SVG icon
are explicit typed graph imports/exports. There is no post-graph production-copy
step.
