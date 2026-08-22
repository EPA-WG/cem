# `@epa-wg/cem-studio`

`@epa-wg/cem-studio` is the synchronized CEM-ML browser-workbench package and
static deployment. Nx schedules its prerequisites and verification; the native
CEM-ML CLI transformation graph is the final production assembler.

The package includes the typed bootstrap/deployment boundary, installable
component shell, versioned local project repository, opt-in File System Access
provider, and graph-owned offline deployment. The project-backed workbench
executes parse, inspect, conversion, query, direct transformation, trace, and
transformation-graph commands through the same browser worker and exact-revision
repository boundary.

## Package surfaces

- `@epa-wg/cem-studio` exports `mountCemStudio`, the explicit browser-command
  loader, opt-in service-worker registration, and the IndexedDB repository
  factory.
- `@epa-wg/cem-studio/repository` exposes the same repository contract without
  importing the application bootstrap. A CEM-ML project validator is mandatory;
  import validation completes before a write transaction, and export validation
  rechecks the normalized project and every resource hash.
- `@epa-wg/cem-studio/file-system-provider` exposes the optional local file and
  portable-project directory adapter plus its deterministic import/export
  archive codec. It never prompts on status or background work; picker and
  retained-handle permission requests require an explicit caller action.
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
The `apply-command-page` command additionally parses portable authored command
resources through the shared CLI grammar, resolves project-local `studio://`
resources, validates the proposed project, and commits current/existing/new page
targets with their exact command bytes in one transaction. Shared run configs
are isolated, incompatible replacement is confirmation-gated, and the result
returns the committed project/entry/resource revisions for later Apply & Run.
The Feature Tour workbench exposes current/existing/new targets with production
CEM selects and fields, confirmation-gates incompatible replacement in a CEM
dialog, and distinguishes Apply from Apply & Run. The latter executes only the
byte-verified committed command revision and marks the result stale if the
repository or displayed command advances during execution.
Feature Tour seed `1.1.0` also includes five deterministic portable-operation
scenarios with explicit resource identities and pinned expected summaries.
Their source/result, execution, expected-result, trace, graph, copy, and download
surfaces are composed entirely from production CEM controls and remain available
after an offline reload.
Search and storage status render through CEM components when the workbench shell
is composed; persistence does not introduce application-local visible controls.

File and directory handles are structured-cloned into host-only
`providerBindings`, restored across repository reopen, and excluded from every
portable export. Pull and write-back compare the retained SHA-256 base with both
the IndexedDB revision and current external bytes before changing either side.
Directory save preflights all declared files, closes and verifies every staged
write, and only then advances the retained base. The browser API has no
cross-file transaction, so a failed multi-file close leaves the binding base
unchanged and requires review/reopen instead of claiming an atomic directory
commit. Unsupported or denied access leaves IndexedDB fully usable and exposes
the same validated deterministic project archive for upload/download recovery.

## Build and verification

```bash
yarn nx run @epa-wg/cem-studio:build
yarn nx run @epa-wg/cem-studio:test:repository
yarn nx run @epa-wg/cem-studio:test:file-system-provider
yarn nx run @epa-wg/cem-studio:test:shell
yarn nx run @epa-wg/cem-studio:check
```

The source/destination module-map pair declares every JavaScript, CSS, worker,
and WASM byte copied into the static deployment. JSON manifests and the SVG icon
are explicit typed graph imports/exports. There is no post-graph production-copy
step.
