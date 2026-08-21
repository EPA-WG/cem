# CEM Studio Phase 6.5 Boundary Audit

Status: completed 2026-08-20. This audit resolves the current workspace
baseline and the application/build boundary before the first Studio project is
created. The product and persistence requirements remain owned by
[`cem-studio.md`](./cem-studio.md), and execution progress is owned by
[`todo.md`](./todo.md).

## Outcome

Phase 6.5 can start without another browser engine, UI system, or production
web build model. The existing public CEM-ML packages expose the portable command
surface needed by Studio, CEM components cover the initial workbench controls,
and the CEM Site has already proved CEM-ML transformation-graph assembly with
schema-owned module maps.

The accepted application boundary is:

- create the publishable `@epa-wg/cem-studio` application and npm package at
  `packages/cem-studio`; the existing `packages/*` Yarn workspace includes that
  location without broadening the workspace boundary;
- let Nx schedule, cache, and verify owner builds, TypeScript compilation where
  needed, the CEM-ML CLI graph, browser tests, and package/release gates;
- keep the CEM-ML CLI transformation graph as the final production assembly
  authority. App modules, component modules, the WASM worker/runtime, manifest,
  service worker, styles, and other emitted resources enter that graph as typed
  resources or schema-owned module-map entries. There is no post-graph
  `node_modules` copy exception;
- allow Vite only as a development server or browser-test harness over authored
  or generated assets. It does not become the production bundler or dependency
  resolver;
- depend on the exact same-version `@epa-wg/cem-ml-cli`, consume its `/browser`
  entry, and receive `@epa-wg/cem-ml` transitively rather than constructing a
  second runtime chain;
- build all visible application functionality, including search, from
  `@epa-wg/cem-components` and `@epa-wg/cem-elements`. Studio owns routing,
  state, persistence, worker/service-worker lifecycle, and search orchestration,
  but not app-local substitute controls;
- keep `@epa-wg/cem-theme` Markdown token specifications authoritative and use
  only their generated semantic projections in the application; and
- make the first executable vertical slice an editable Feature Tour project
  that persists locally, reloads offline, validates through the browser command
  worker, and exposes structured diagnostics/report/source-map provenance with
  the native textarea baseline and CEM controls.

## Resolved Workspace Baseline

| Surface             | Resolved evidence                                                                                                                                                                                                                                                                                                                                                 | Phase 6.5 consequence                                                                                                                                                                                                  |
| ------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Nx project graph    | The workspace resolves 44 projects and no `cem-studio` project. `@epa-wg/cem-ml-cli`, `@epa-wg/cem-ml`, `@epa-wg/cem-components`, `@epa-wg/cem-theme`, `cem-elements`, and `cem-site` are independently resolved owners.                                                                                                                                          | Studio is a new package/application; no hidden scaffold or application state needs migration.                                                                                                                          |
| Browser engine      | `@epa-wg/cem-ml-cli/browser` exports command parsing/serialization, invocation projection, a command-service client, worker-pool operation control, cancellation, artifact reads, and disposal.                                                                                                                                                                   | Studio composes the public browser client and does not create a private WASM wrapper.                                                                                                                                  |
| Portable operations | The browser parity fixture executes `parse`, `validate`, `check`, `inspect`, `convert`, `query`, direct and graph `transform`, `trace`, and version/capability discovery. The checked report records native, Node, and browser normalized parity plus cancellation.                                                                                               | The old proposal gap claiming that typed browser operation parity still needs to be added is closed. Studio work starts at project/product composition.                                                                |
| Browser limitations | Browser capabilities truthfully mark `bench`, fixtures, schema mutation, and plugin mutation unavailable; the current browser worker topology reports one effective worker.                                                                                                                                                                                       | Unsupported features remain disabled or explanatory. They are not silently substituted and do not block the local MVP.                                                                                                 |
| UI foundation       | The component catalog has 48 public primitives and the pinned Angular Material inventory has 37 entries. The initial frame can reuse covered `cem-tree`, `cem-dialog`, `cem-select`, `cem-stepper`, `cem-tooltip`, and related elements; partial rows such as app bar, tabs, table, progress, toast, list, and button remain bounded by their recorded contracts. | Every Studio composition must classify its needed behavior against the matrix. A partial component is usable only within its proven contract; missing general behavior is completed in `@epa-wg/cem-components` first. |
| Persistence and PWA | No existing Studio IndexedDB repository, File System Access provider, service worker, Cache Storage owner, or install manifest was found.                                                                                                                                                                                                                         | These are genuine Phase 6.5 application deliverables, not wiring to an existing app. Persistence begins only after the portable project contract is accepted.                                                          |
| Web assembly        | `cem-site` already makes Nx the orchestrator and the CEM-ML graph/module-map pair the deterministic production assembly authority; Vite is present for tests but no Nx Vite application plugin owns production output.                                                                                                                                            | Studio adopts the same authority split and extends it for an installable app rather than introducing a competing build pipeline.                                                                                       |

## Ownership Boundary

```text
@epa-wg/cem-studio
├── application orchestration
│   ├── routes, pane state, commands, search, and update UI
│   ├── IndexedDB repositories, migrations, import/export, and providers
│   └── service-worker registration and worker lifecycle
├── @epa-wg/cem-components + @epa-wg/cem-elements
│   └── every visible control and reusable workbench composition
├── @epa-wg/cem-ml-cli/browser (exact same version)
│   └── typed commands, capability discovery, progress, cancellation, artifacts
│       └── @epa-wg/cem-ml/wasm (one transitive engine)
└── CEM-ML production transformation graph
    └── explicit typed resources and source/destination module maps
```

The application may own a service or state model that drives a component. It
must not reproduce the component's DOM, keyboard, accessibility, theme, or
search-control behavior with local HTML controls. Reusable diagnostics,
explorer, preview, source-editor-frame, or graph behavior belongs in a future
`@epa-wg/cem-components/studio` export after the parity classification proves
that it has no unfinished general counterpart.

## First Vertical Slice

The first executable proof is deliberately narrower than the complete Phase
6.5 exit criteria:

1. Install a versioned editable copy of one generated Feature Tour project.
2. Select one CEM source through `cem-tree` and edit its source through the
   native-textarea CEM component contract.
3. Autosave the changed resource and project revision atomically in IndexedDB.
4. Build and execute a typed `validate` request through
   `@epa-wg/cem-ml-cli/browser`, using `studio://` resources plus exact revision
   and SHA-256 metadata.
5. Render status, diagnostics, report summary, and source-map navigation through
   CEM components; do not insert generated HTML into the application origin.
6. Reload while offline and prove that the saved revision, app shell, worker,
   runtime, and seed assets remain usable.

Parse and inspect projections can join the same slice when they reuse the same
request, persistence, and result boundaries. Conversion, query,
transformation, graph editing, the bidirectional CLI Command view, local-file
write-back, and provider work stay in later checklist items rather than
expanding the first proof.

## Decision Point Reached

The portable project schema/content identity is not yet accepted. It owns the
shape persisted in IndexedDB, exported as `project.cem`, projected to JSON,
used by Feature Tour generation, and migrated across releases. Scaffolding a
store or seed before that decision would make application code an accidental
schema authority, so Phase 6.5 stops here.

The recommended next contract is:

- canonical CEM content type `application/vnd.cem.studio-project+cem`;
- normalized JSON projection `application/vnd.cem.studio-project+json`;
- shared schema identity `https://cem.dev/ns/studio/project/1`;
- JSON Schema artifact identity
  `https://cem.dev/schema/studio/project.schema.json`; and
- a versioned `studio-project/v1` schema package that owns both projections,
  deliberate rejection fixtures, conversion parity, and Feature Tour seed
  validation.

This follows the repository's existing vendor content-type and versioned
namespace vocabulary while keeping the schema independent of browser storage.
It remains a recommendation, not an accepted contract, until the next checklist
item is completed.

## Evidence

The audit used resolved Nx and checked artifacts rather than only project files:

```bash
NX_DAEMON=false yarn nx show projects --json
NX_DAEMON=false yarn nx show project @epa-wg/cem-ml-cli --json
NX_DAEMON=false yarn nx show project @epa-wg/cem-ml --json
NX_DAEMON=false yarn nx show project @epa-wg/cem-components --json
```

It also inspected:

- `packages/cem-ml-cli-npm/src/browser.ts` and the browser command/protocol
  implementation;
- `packages/cem-ml-cli-npm/tests/command-all-operations.fixture.mjs`;
- `dist/reports/cem-ml-platform-parity.browser.json` and
  `dist/reports/cem-ml-platform-parity.json`;
- `packages/cem-components/tests/angular-material-parity.json` and the public
  component source/catalog;
- the repository for IndexedDB, service-worker, Cache Storage, and File System
  Access implementations; and
- the accepted CEM Site build boundary and resolved `cem-site` targets.
