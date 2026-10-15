# CEM Studio Web Application Contract

Status: accepted Phase 6.5 product and persistence contract, originally
researched 2026-08-09 and promoted to the roadmap. The resolved 2026-08-20
implementation baseline and build boundary are recorded in
[`cem-studio-phase6.5-boundary-audit.md`](./cem-studio-phase6.5-boundary-audit.md);
the task-level execution state is owned by `docs/todo.md`. The 2026-08-21
repository, deterministic-search, and optional browser-AI architecture below is
also accepted: IndexedDB remains a Studio application repository behind a
generic `cem-elements` host capability, while visible search and AI states are
composed entirely from CEM controls. The read-only `repository-query` and
`storage-status` resource boundary was implemented and browser-verified on
2026-08-21. The opt-in File System Access provider, retained handle bindings,
external-conflict protection, and validated import/export fallback were
implemented and browser-verified on 2026-08-22.

The Phase 2.5 deployment, version, platform, capability, signing, and host-wire
decisions are canonical in
[`cem-ml-deployment-contract.md`](./cem-ml-deployment-contract.md). This
document remains the broader Phase 6.5 Studio product and persistence proposal.

## Outcome

CEM Studio should be an installable, offline-capable web application and its own
public `@epa-wg/cem-studio` npm package. It makes the useful `cem-ml` CLI
workflows available through a browser workbench and directly depends on the
separate `@epa-wg/cem-ml-cli` WebAssembly CLI npm package. That CLI package
depends on the generated `@epa-wg/cem-ml` WASM runtime. Studio must not
reimplement formats, validation, query, or transformation behavior in
TypeScript.

The recommended product shape is:

- a deployable `cem-studio` Progressive Web Application (PWA), published and
  versioned as `@epa-wg/cem-studio`;
- a typed browser command service in `@epa-wg/cem-ml-cli/browser`, backed by the
  separate `@epa-wg/cem-ml/wasm` runtime in a dedicated worker;
- a portable npm-installed `cem-ml` command that runs the WASM engine through a
  Node host adapter from the `@epa-wg/cem-ml-cli` npm package on supported
  operating systems;
- a deliberately small CLI platform matrix: WASM for Node, native Linux AMD64,
  native macOS ARM64 distributed through Homebrew, and native Windows AMD64;
- a separate Nx subproject and deployment package for each of those three
  native platform targets; only Linux is currently a tagged GitHub Release
  unit, while macOS and Windows remain local validation targets;
- a project/subproject explorer that owns data sets, resources, run
  configurations, conversions, queries, transformations, and transformation
  graphs;
- a bidirectional CLI Command view that serializes the active page, copies its
  command/inputs/config/output, and applies edited commands to an existing or
  newly created project page;
- editable inline sources and URL-backed sources with explicit content and
  schema identities;
- validation, data, result, report, source-map, and graph previews;
- local-first persistence in IndexedDB, with import/export as the user's durable
  backup boundary;
- opt-in individual-file and portable-directory access through retained browser
  handles, with explicit permissions and conflict-safe write-back;
- UI built from light-DOM `@epa-wg/cem-components`, with Studio-specific
  components published from `@epa-wg/cem-components/studio`; all visible UI,
  interaction bindings, and state projection are authored in XHTML/CEM-ML under
  the normative
  [`declarative UI principle`](./declarative-ui-principle.md), with no
  component- or app-local JavaScript behavior;
- Consumer Semantic Theming supplied by `@epa-wg/cem-theme`;
- optional account-backed storage adapters for S3, a NoSQL service, Git
  repositories, and GitHub Gists as a later wishlist, not an MVP dependency;
  and
- a later semi-native CLI experiment that bundles the Node host, CLI JavaScript,
  and WASM runtime into target-specific executables using Node.js Single
  Executable Applications (SEA), with deprecated `pkg` considered only as a
  comparison or migration reference.

The common `cem_ml` project owns the authoritative product version. The WASM
runtime npm package, CLI npm package, Studio npm/PWA package, and every native
platform deployment package must be built from the same release commit and
report that exact CEM-ML version. Published units use the same version; macOS
and Windows publication is deferred. Any semi-native package promoted from the
wishlist joins the same release family. They do not version independently.

The phrase “WASM version of the CLI” means portable browser and Node-hosted
parity with the CLI's typed operations and reports. In the browser it does not
mean compiling terminal formatting, `clap`, process exit behavior, or an
unrestricted operating-system filesystem into the page. In Node, a thin host
adapter may provide files, streams, environment, and exit-code projections
without creating another semantic engine.

## Implemented Phase 6.5 Baseline (2026-08-22)

The graph-built `@epa-wg/cem-studio` PWA now owns the accepted IndexedDB
repository, generated Feature Tour, offline shell, and first editable workbench.
The workbench saves and reloads exact optimistic project/resource revisions,
validates them through the dedicated browser command worker, and keeps native
diagnostics, report facts, execution identity, and source-map provenance.

The same workbench now executes CEM-ML `parse` AST/events projections and all
six browser-capable `inspect` views (`summary`, `ast`, `events`, `diagnostics`,
`source-offsets`, and `tree`). These commands are parsed and lowered by the
shared Rust-owned CLI contract, receive the exact IndexedDB bytes and durable
revision ledger, and retain the native command result plus the verified
target-native CEM-ML output artifact. The application does not reshape that
output through a JSON AST or DOM handoff. All visible selection, action, status,
output, and execution-metadata surfaces use production `cem-components` on the
shared `cem-elements` runtime. Real Chromium verifies every projection online,
identical output after offline reload, and stale-result handling when a draft or
durable revision advances during execution.

The workbench also exposes the active invocation as an editable **Studio
command**. A shared literal command-text codec in `@epa-wg/cem-ml-cli` preserves
argv without shell expansion, parses edits through the generated CLI grammar,
and serializes the normalized result canonically. Studio derives its categorized
input, identity, configuration, output, and scope change preview from the
Rust-lowered browser invocation and normalized run plan. Copy writes the exact
displayed draft only from an explicit CEM action, reports success or failure in
a CEM alert, and keeps the command textarea selectable as the fallback. Invalid
syntax/options or unresolved resources leave IndexedDB unchanged. Real Chromium
proves canonical edit/reset, semantic changes, invalid diagnostics, exact copy,
non-mutation, CEM-only controls, and the same preview after offline reload.

Transactional Apply now resolves stable current, compatible existing, or named
new pages, confirmation-gates incompatible replacement, commits the authored
command and references atomically, reloads the exact bytes, and makes Apply &
Run execute only that committed revision.

Feature Tour seed `1.1.0` adds project-owned conversion, XPath query, direct
XPath transformation, parser/validation trace, and transformation-graph
workbenches. Their commands use the same browser grammar, worker, resolver,
revision ledger, native result, artifact reader, diagnostics, and source-map
path as parse/inspect. Each scenario carries explicit input/query/template/graph
identities and a pinned expected summary. The shared CEM composition presents
operation selection, exact source/output, identity and execution metadata,
expected-result comparison, trace and graph overlays, and explicit copy/download
actions. Real Chromium executes all five online and repeats byte-identical
results after offline reload.

The local-file provider now opens an individual project resource or a portable
directory only from explicit production CEM actions. It stores structured-clone
file/directory handles and their revision/hash base snapshots in host-only
`providerBindings`, never in the project model or export. Retained permission is
queried without prompting; reconnect calls `requestPermission()` only when the
user activates the reconnect action. Native CEM-ML conversion owns exact
`project.cem` and normalized JSON projection, while the repository remains the
validation and transactional import authority. Resource and directory writes
compare current external SHA-256 snapshots before opening a writable, verify
the bytes after close, and advance the binding base only after all declared
writes succeed. Unsupported or denied access keeps the IndexedDB working copy
available and exposes a deterministic, validated download/upload archive.

## Next delivery milestone: user-owned project workspace

Defined 2026-10-12 after completion of the Studio/Site declarative UI migration.
This is the next bounded delivery slice of the accepted product contract;
implementation and fixture status belong to
[the active checklist](todo.md#cem-studio-delivery-planning). It is planned,
not an implemented capability or completion of the broader Phase 6.5 contract.

### Current implementation and the delivery gap

| Existing implementation | Work required for this milestone |
| --- | --- |
| [Repository](../packages/cem-studio/src/repository.js): validated import/export, resource saves, transactional command Apply, project trash/restore, deterministic search and revision invalidation. | Compose these services into a usable project lifecycle; retain their transaction and conflict authority. |
| [Application entry](../packages/cem-studio/src/main.js) and [workbench](../packages/cem-studio/src/workbench.js): startup selects the Feature Tour project; page descriptors and dependencies come from its seed catalog. | Select projects, pages and resources from durable repository records. Keep the Feature Tour as a starter and regression fixture. |
| [File-provider controller](../packages/cem-studio/src/shell.js): imports can change its project identity and it exposes `setProject`. | Coordinate accepted project changes with the workbench, export and provider targets; a successful import must become openable in the app. |
| [Tree](../packages/cem-components/docs/tree-contract.md): leaf activation emits `{ value }`; selection belongs to the application. The [host view](../packages/cem-elements/src/lib/host-view.ts) now delivers that scalar through an explicit tree command binding. | Prove guarded navigation and dialog/pane workflows in the shared library before explorer UI work. |

### User-visible acceptance

1. Open Home, see existing projects and storage state, and create an editable
   project from a validated bundled starter or import a valid portable archive.
   Creation allocates a fresh project identity and preserves project-local
   references. Import identity collisions produce a recoverable conflict;
   this slice does not silently replace or remap an existing project.
2. Open any admitted project and navigate its existing subprojects, pages and
   resources by stable identity. A project without executable pages still opens
   with an explicit empty state and its available resources. Inline/project-file
   text within the existing limits can be edited; unsupported or binary content
   retains its identity and a bounded read-only/download presentation.
3. Save and validate source, run parse/inspect, and execute supported persisted
   conversion/query/transformation/trace/graph pages through the existing CLI
   worker. Resolve commands, inputs and dependencies from the selected project,
   including pages created by command Apply. Ordinary projects require no seed
   catalog or expected-result sidecar; Feature Tour comparisons remain optional
   sample metadata. Unavailable commands or unresolved resources explain why
   execution is unavailable without guessing another operation or fetching data.
4. Navigate between projects/pages/resources without silently losing source or
   command drafts. Source drafts offer Save/Discard/Cancel; command drafts use
   the existing Apply preview/confirmation or Discard/Cancel. Both must be
   resolved when both are dirty. Failed persistence or rejected Apply keeps the
   draft and original selection. Saving bytes and reporting validation results
   remain distinct outcomes. Service guards prevent duplicate pending changes
   and stale asynchronous completions from switching the active project.
5. Search the repository's deterministic index and open a result through the
   same navigation guard. Reload online or offline to restore the last committed
   project/page/resource selection and exact saved bytes. Missing or trashed
   selections recover to Home; startup never overwrites an existing user copy.
   Selection preferences are local application state, outside portable exports.
6. Export the active project, move it to recoverable trash, and restore the same
   identity and bytes. The optional file provider follows the admitted active
   project/resource, preserves explicit permission/write actions and revision/hash
   checks, and retains archive fallback. Picker cancellation, denied permission,
   conflicts and quota failures leave the current working copy usable.

### Shared controls before Studio composition

Reuse the canonical controls within their documented subsets. The
[pinned Angular Material comparison](../packages/cem-components/docs/angular-material-parity.md)
is the functionality benchmark; this milestone does not promote partial rows to
full parity merely by composing them into Studio.

| User need | Shared CEM owner and boundary |
| --- | --- |
| Project list and search results | Passive `cem-list` with named `cem-action` controls; native form submission and `cem-text-field` for explicit search. Repository ranking and navigation stay in Studio services. |
| Nested explorer | `cem-tree`/`cem-tree-item`, with opaque stable keys mapped to project entities. Parent activation expands; leaf activation requests navigation. Focus alone never navigates. Only admitted navigation changes `selected-values`. Empty projects use a separate empty state. |
| Source, command and identity fields | Existing `cem-textarea`, `cem-text-field`, `cem-select` and `cem-field` contracts; preserve exact source text, caret and focus during unrelated updates. |
| Draft decisions and recoverable removal | Named `cem-dialog` with `cem-action` decisions and native-surface dismissal/focus return. Keep `cem-sheet` a focus-neutral region. |
| Pane navigation and layout | `cem-tabs`, `cem-stack`, `cem-grid` and existing surfaces. Initially use local-view tabs at compact and expanded widths, retaining pane nodes and selection. Simultaneous three-pane layout and resizing remain later work. |
| Storage, operation and result feedback | Existing `cem-alert`, `cem-badge`, `cem-progress`, `cem-table` and `cem-media-preview`; loading, empty, failure, conflict and stale states must remain understandable without color. |

The first shared slice implements `mountCemView` receiving an explicitly bound
`cem-tree-activate` command from its exact component owner, with worker/fallback
fixtures. The bridge validates the documented string `value` and delivers it
through the existing frozen scalar request. Arbitrary event-detail
objects, DOM nodes and undeclared event names must not become service input.
Preserve command allowlisting, nested-view isolation, disabled/cancelled-event
suppression, declaration revocation and disposal. Custom events retain their
actual trust metadata; they do not create browser permission authority. Native
file/install actions keep synchronous delivery on their own activation stack.
Additional component events need their own explicit contract and fixture when
a concrete workflow requires them.

The [shared project-navigation workflow](../packages/cem-components/docs/project-navigation-contract.md)
proves tree activation, guarded selection, dialog dismissal/focus return and
retained panes before Studio adoption. Its services are bounded test stubs;
the production session admission service below now provides draft authority.
Repository-owned workbench resolution and navigation wiring remain open.
Any missing reusable interaction belongs in `cem-elements`,
with its styled declaration and public examples in `cem-components` before app
use. Do not create a Studio-specific widget for functionality already owned by
the general library. Studio services own repository identities, draft decisions,
search, navigation, workers and browser APIs; XHTML/CEM-ML owns presentation and
interaction bindings. Normalized repository records remain an explicit host API
projection; document/command semantics continue through the native CEM boundary.

### Project session admission service

`createCemStudioProjectSession({ openWorkbench })` is a non-UI service exported
from the package root and `@epa-wg/cem-studio/project-session`. Its resolver must
return a fresh workbench for each requested `{ projectId, entryId?, resourceId? }`.
The session checks the loaded identity, keeps the candidate separate from the
active workbench, and commits selection only after loading and draft admission.
Project workbench snapshots expose `entryId` independently of command state;
the legacy adapter can still supply `command.application.currentEntryId`.
Selection remains local session state;
it is never added to a portable project.

`select(identity)` starts one transition. `resolveTransition(id, decision)`
accepts `save`, `apply`, `discard`, `cancel`, `confirm-replacement`, or `retry`.
Source and command decisions are independent, with source resolved first.
The snapshot exposes the pending target, transition id, stage and error while
retaining the committed selection and workbench. Native dialog dismissal must
call `cancelTransition(id)`; an old id cannot resolve a newer request.
Incompatible command replacement requires its existing explicit confirmation.

Discard records consent for the exact draft and its baseline. It removes
neither draft until the destination is admitted. Cancel, a failed open, or a
new edit during opening preserves the original workbench. Save uses
`saveSource()` to verify exact committed bytes and revisions independently of
native preview/validation availability. Apply establishes the committed command
as its new baseline and retains/rechecks any newer edit. An already-started
transaction can commit after cancellation, but its completion cannot admit
the cancelled destination.

Repeated decisions share the pending operation. A different pending target
requires cancellation first. After cancelling a write, `pendingWrite` and
`status: settling` keep further admission blocked until the write settles;
the caller then requests navigation again. Failed candidate loading supports
`retry`, including the initial open. Candidate cancellation/disposal and
unsubscribing the former workbench prevent late results from replacing the
active projection. Native command previews and background projections also
retain newer command drafts within the workbench.

`createCemStudioProjectWorkbench({ repository, validator, selection, signal })`
loads project-owned resources, page identity and dependencies from IndexedDB.
Persisted CLI command resources are decoded by the shared CLI grammar, previewed
without execution, and run from their exact bytes. The command input identity
chooses the source independently of resource order. Legacy run configurations
alone do not imply an executable operation. Explicit Parse/Inspect prepares a
command that can subsequently be applied. Optional sample metadata supplies
Feature Tour expectations and portable examples without replacing project records.

Empty projects and folders remain selectable. Local UTF-8 text within the source
limit is editable; binary, unsupported and oversized resources expose a read-only
identity and download of original local bytes. No resource is implicitly fetched.
Missing identities fail admission and retain the previous workbench.

`createCemStudioSessionWorkbench({ repository, session })` supplies the mounted
workbench with a project/page/resource selector and explicit project refresh.
The selector routes every request through admission; nested page labels retain
their parent context. The production startup now uses this adapter. Existing
`cem-select`, `cem-dialog`, `cem-action`, editor and status declarations own all
UI behavior. One navigation dialog resolves source, command and incompatible
replacement in sequence, including native dismissal and focus return. Provider
selection follows admitted identities. Home, automatic recovery of local selection,
search/explorer panes and full provider coordination remain later milestone items.
No application UI listeners, CSS, token defaults or new variation families are added.

### Completion evidence and limits

Add actionable fixture items to `todo.md` before or with every new fixture.
Deliver in this order: shared event bridge, shared workflow evidence, general
project session/workbench, Home/create/import, explorer/search, recovery/provider
coordination, and deployed acceptance. Keep the existing Feature Tour command,
artifact, source-map, offline and exact-revision checks as regressions.

The deployed acceptance fixture must exercise two independent user projects,
nested entries and a command-created page; source and command draft decisions;
failed saves and concurrent-tab conflicts; search after save/trash/restore;
active-project import/export/provider targeting; cancelled pickers; and exact
saved-byte recovery after offline reload and safe update. Build/package checks
must include new declarations and their graph-owned deployment dependencies in
an isolated consumer. Run the existing Studio `check` gate after the focused
runtime, component, repository and browser checks.

Audit against the [CEM principles](../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles):
semantic action/content intent, bounded control variants, native keyboard and
accessible names, independent focus/selection, coherent spacing/shape/type, and
theme-owned inherited tokens. Verify all five themes, forced colors, applicable
reduced motion, inherited overrides and compact layouts. Record actual browser
and device coverage; broader physical touch/pen/IME and browser/AT acceptance
remain open until measured, independently of this bounded milestone.

Explicit Save/Apply remains the project durability boundary. Local draft recovery
now protects unfinished text separately (see Local draft recovery below). Bounded
structural editing and undo/redo are implemented below. Graphical authoring,
new command-specific forms, simultaneous/resizable panes, virtualized large
trees, remote providers and AI assistance remain later slices of the broader
contract. Project creation from a starter and existing command-created pages
are included here; a general project structure editor is not required to close
this milestone. Presentation work remains deferred with its resume context in
the active todo file.

## Goals

- Let a user learn and exercise CEM-ML without installing a native executable.
- Cover the structural lifecycle: parse, validate, inspect, convert, query,
  transform, and inspect the resulting reports and source maps.
- Demonstrate every browser-capable schema package from source content types
  through queries, templates, conversions, and transformation graphs.
- Allow users to add, rename, edit, copy, reorder, and remove projects,
  subprojects, data sets, resources, configurations, and transformations.
- Let users move between structured forms and an editable `cem-ml` command
  without losing the selected inputs, effective config, outputs, or project
  destination.
- Restore user changes after reload, browser restart, PWA update, and offline
  launch, subject to browser storage guarantees.
- Keep browser and native results comparable through shared request, capability,
  diagnostic, report, and source-map contracts.
- Keep local projects private by default and require an explicit action before
  fetching a remote resource or publishing data to an external service.
- Make useful Studio components available to other CEM applications without
  turning the application shell into a component package.
- Use the Consumer Semantic Theme consistently across layout, controls,
  editors, graphs, diagnostics, previews, and all supported theme modes.

## Non-Goals for the First Release

- An arbitrary shell, arbitrary JavaScript evaluator, or emulated POSIX
  environment in the page.
- Silent access to the user's local filesystem, network credentials, Git
  credentials, or cloud credentials.
- Full native plugin, native filesystem, or benchmark parity when a capability
  is unavailable in WASM.
- Multi-user collaboration, hosted accounts, or a required CEM cloud service.
- Executing scripts from user-authored HTML or transformation output in the
  Studio origin.
- Replacing desktop IDE integration. Studio complements the LSP/DAP and
  Chromium integrations described in [`integrations.md`](./integrations.md).

## Existing Workspace Foundation and Gaps

The phase-start evidence is detailed in the
[`Phase 6.5 boundary audit`](./cem-studio-phase6.5-boundary-audit.md). The live
implementation baseline above supersedes the starting gaps recorded here:

| Existing asset                                                                                    | Value to Studio                                                                                                                                                                   | Remaining gap work                                                                                                                                                             |
| ------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `cem_ml` plus the synchronized `@epa-wg/cem-ml` `/wasm` runtime                                   | One semantic implementation reaches native, Node, and browser WASM.                                                                                                               | Studio must consume the public runtime transitively through the CLI package and never bundle a private engine.                                                                 |
| `@epa-wg/cem-ml-cli/browser` typed command service                                                | Portable `parse`, `validate`, `check`, `inspect`, `convert`, `query`, `transform`, `trace`, capability, cancellation, progress, artifact, and resolver contracts are implemented. | Compose these contracts into Studio project/run state; do not recreate command execution in application TypeScript.                                                            |
| Native/Node/browser parity reports                                                                | The portable operation fixture and normalized result/cancellation parity are already verified across all three hosts.                                                             | Add Studio-specific project, persistence, stale-result, Feature Tour, and UI integration fixtures.                                                                             |
| Schema-package registries and examples                                                            | The accepted `studio-project/v1` package owns the portable CEM/JSON project model, JSON Schema artifact, and Feature Tour/rejection fixtures.                                     | Ensure the browser capability payload carries the package versions needed to gate generated examples.                                                                          |
| `@epa-wg/cem-components`, `@epa-wg/cem-elements`, and the pinned 37-entry Angular Material matrix | The 48 public primitives provide the reusable light-DOM UI foundation.                                                                                                            | Classify each Studio composition, complete missing general parity behavior first, and add a `/studio` export only for reusable workbench behavior with no general counterpart. |
| `@epa-wg/cem-theme` Markdown specifications and generated semantic modes                          | Canonical visual system for Studio.                                                                                                                                               | Add only genuinely reusable missing semantics to the Markdown owners; keep application aliases mapped to existing semantic tokens.                                             |
| The fixed `cem-ml-platform` Nx release group                                                      | `cem_ml`, native CLI, WASM npm, CLI npm, and native target packages already share one Cargo-derived version contract.                                                             | Add the future Studio project to the release family and verify its exact CLI dependency, package artifact, app build id, and release commit.                                   |
| CEM Site CEM-ML graph/module-map production assembly                                              | Proves deterministic web assembly without making Vite or a copy script the deployment authority.                                                                                  | Extend the same boundary to Studio app modules, worker/WASM assets, manifest, service worker, and offline cache inventory.                                                     |
| No existing Studio application, IndexedDB store, local-file provider, or PWA owner                | Leaves a clean application boundary.                                                                                                                                              | Create the Studio application/package next, then implement the repository, provider, and PWA deliverables against the accepted project contract.                               |

Relevant local contracts include the
[`cem-ml` CLI feature summary](./cem-ml-cli-contract.md),
[`cem-ml` run-config contract](./cem-ml-phase2-run-config-contract.md),
[`cem-element` WASM proposal](./cem-element-wasm-proposal.md), and the
[`@epa-wg/cem-components` conventions](../packages/cem-components/docs/conventions.md).

## Recommended Architecture

```text
CEM Studio PWA
├── application shell
│   ├── routing, commands, responsive workspace layout
│   ├── @epa-wg/cem-components
│   ├── @epa-wg/cem-components/studio
│   └── @epa-wg/cem-theme CSS and theme scope
├── workspace service
│   ├── IndexedDB project records and text/blob resources
│   ├── import/export and schema migrations
│   ├── URL resource snapshots and provenance
│   └── optional remote-storage adapters
├── engine client
│   ├── typed request/response and progress protocol
│   ├── cancellation, stale-result rejection, and limits
│   └── dedicated Web Worker
│       └── @epa-wg/cem-ml-cli/browser
│           └── @epa-wg/cem-ml/wasm
│               ├── CEM-ML engine and schema packages
│               ├── CEM-QL/query/transformation engines
│               └── JS resource resolver for studio:// and allowed URLs
└── service worker
    ├── versioned app-shell/WASM/sample cache
    ├── offline navigation fallback
    └── update coordination (never the project database owner)
```

A dedicated worker keeps parsing and transformations off the UI thread. Web
Workers run in a separate global context and communicate with the page by
messages, which fits a typed request/progress/result boundary
([MDN Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)).
The first version should use one dedicated engine worker per open Studio window.
A shared worker or worker pool should be added only after measurements show a
benefit and the cache/concurrency contract is defined.

### Responsibility Boundaries

| Layer                            | Owns                                                                                                                                            | Must not own                                                                                 |
| -------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `cem_ml` Rust library            | Parsing, validation, schemas, normalized run plans, resolver policy, conversion, reports, diagnostics, source maps, transformations.            | Browser widgets, routes, terminal formatting, account services.                              |
| `@epa-wg/cem-ml/wasm`            | Generated low-level WASM bindings and wire-safe data transfer.                                                                                  | UI state or a second semantic implementation.                                                |
| `@epa-wg/cem-ml-cli/browser`     | Worker-safe typed commands, capability discovery, resolver bridge, cancellation, event streaming.                                               | DOM rendering or product-specific project storage.                                           |
| `@epa-wg/cem-components`         | Reusable light-DOM controls and functional UI patterns.                                                                                         | Studio routing, persistence, or CEM-ML execution policy.                                     |
| `@epa-wg/cem-components/studio`  | Reusable Studio workbench composites such as explorer, source editor frame, diagnostics panel, preview frame, and graph view.                   | User accounts, provider credentials, service worker lifecycle.                               |
| `@epa-wg/cem-studio` app/package | Composition, routes, project state, command orchestration, PWA lifecycle, deployment configuration, and publishable static/bootstrap artifacts. | Duplicate format/query/transform semantics or a bundled private copy of the engine contract. |

## Workspace and Package Placement

The proposed eventual workspace placement is:

```text
packages/
├── cem_ml/                       # common engine and authoritative product version
├── cem_ml_cli/                   # common native CLI source/adapter
├── cem-ml-npm/                   # @epa-wg/cem-ml: generated low-level WASM runtime
├── cem-ml-cli-npm/               # @epa-wg/cem-ml-cli: browser/Node adapters + npm bin
├── cem-components/
│   └── src/lib/studio/           # @epa-wg/cem-components/studio
├── cem-studio/                   # @epa-wg/cem-studio: deployable Nx PWA + npm package
├── cem-ml-cli-native-linux-amd64/
├── cem-ml-cli-native-brew-arm64/
└── cem-ml-cli-native-windows-amd64/
```

The exact generated npm-package folder name should be selected during
scaffolding; the public package identity is the important contract.

Studio-specific components belong in the `@epa-wg/cem-components` project and
npm package, exposed through a `./studio` export. A component should move to the
general package surface when it expresses a reusable functional pattern that
does not require CEM Studio state, for example a split pane, accessible tree,
status list, diff viewer, or bounded preview frame. Components whose vocabulary
is inherently Studio-specific, such as a transformation-graph inspector, may
remain in the Studio subpath.

Every general or Studio-specific component still follows the same physical and
testing contract: its own `src/components/<cem-tag>/<cem-tag>.xhtml` CEM-ML
declaration with an embedded static `<style>` node installed once by
`cem-elements`, consuming CEM UI theme tokens, plus a colocated CSF Next
`<cem-tag>.stories.ts` file whose HTML
string `render` cases and async `play` functions own all unit assertions through
`storybook/test`. A standalone component CSS file is forbidden. The story is
development-only and cannot implement behavior. The `/studio` subpath is not
permission to add TypeScript components, behavior callbacks, or separate unit
specs. `cem-select` is the proven physical/testing pattern.
The accepted native `@scope` target exposes only documented tags, public
`scope="name"` groups, projected `slot="name"` roots, component-owned
`part="name"` hooks, and custom properties to Studio CSS. Data-island and render
identity remain internal.
The normative ownership, specificity, and fail-closed behavior is defined by
[CEM light-DOM CSS scoping rules](./cem-ml-uid-and-scoped-css-design.md).
Studio MUST NOT assume that target behavior until its active todo gate passes.

Angular Material parity is a prerequisite gate, not merely inspiration. Before
implementing any Studio control or interaction, classify it against the pinned
[Angular Material component catalog](https://material.angular.dev/components/categories).
If Angular Material has a counterpart, first create and verify the general
`@epa-wg/cem-components` equivalent on the light-DOM `<cem-element>` substrate;
Studio then consumes that component. Neither the `/studio` subpath nor the
application may temporarily duplicate it. A Studio component or application-
local implementation may lead only when the parity matrix records no Angular
Material counterpart. If that new behavior later proves reusable, move it into
the appropriate component-package surface.

Application orchestration stays in the publishable `@epa-wg/cem-studio`
project: provider authentication, routes, IndexedDB repositories,
service-worker registration, seed-project installation, and deployment
environment configuration are not reusable UI components. Its JavaScript may
implement those non-UI services, but it may not create/mutate visible DOM or
attach UI listeners. When declarative UI would require missing or excessively
verbose CEM-ML, Studio work hard-stops until the reusable capability is
implemented and verified in `cem-elements`.

Each folder above is an independent Nx subproject with its own build/package/
verify/publish targets. This is the complete initial native target list, not an
illustrative matrix. Other native architectures, ABIs, operating systems, and
package-manager channels are outside initial support. Any later expansion must
be an explicit product decision and a new deployment subproject; one project
must not publish opaque architecture variants selected only by a shell script.

## Deployment Subprojects and Synchronized Version Model

The common engine, three npm deployment packages, and native target packages
are separate Nx subprojects and deployment units. They form one fixed-version
CEM-ML platform release.

| Subproject/deliverable    | Public package/artifact                           | Direct dependency                                                                | Runtime and use                                                          |
| ------------------------- | ------------------------------------------------- | -------------------------------------------------------------------------------- | ------------------------------------------------------------------------ |
| Common CEM-ML             | Rust crate/project `cem_ml`                       | None; owns common engine contracts                                               | Semantic engine and authoritative version source.                        |
| WASM runtime npm          | `@epa-wg/cem-ml`                                  | Generated from common `cem_ml`                                                   | Low-level WASM/types/schema-package runtime; no executable npm `bin`.    |
| Universal CLI npm         | `@epa-wg/cem-ml-cli`                              | Exact same-version `@epa-wg/cem-ml`                                              | Browser command service, Node host adapter, and npm `cem-ml` executable. |
| Studio npm/PWA            | `@epa-wg/cem-studio`                              | Exact same-version `@epa-wg/cem-ml-cli`; compatible CEM component/theme packages | Installable/hostable Studio application and static PWA deployment.       |
| Native target deployments | One archive/package set per supported Rust target | Common `cem_ml` plus common `cem_ml_cli` source                                  | Native CLI for one OS/architecture/ABI target.                           |

The required build/release graph is:

```text
cem_ml (common version authority)
├── @epa-wg/cem-ml (WASM runtime npm)
│   └── @epa-wg/cem-ml-cli (universal WASM CLI npm)
│       └── @epa-wg/cem-studio (Studio npm + PWA)
└── cem_ml_cli (common native CLI source)
    ├── cem_ml_cli_native_linux_amd64
    ├── cem_ml_cli_native_brew_arm64
    └── cem_ml_cli_native_windows_amd64
```

### Initial CLI Platform Matrix

The first supported CLI matrix is intentionally limited to these four entries:

| CLI distribution | Platform/channel                              | Runtime identity       | Deployment project/package        |
| ---------------- | --------------------------------------------- | ---------------------- | --------------------------------- |
| WASM for Node    | Supported Node.js hosts                       | `wasm-node`            | `@epa-wg/cem-ml-cli` npm package  |
| Linux AMD64      | `x86_64-unknown-linux-gnu`                    | `native-linux-amd64`   | `cem_ml_cli_native_linux_amd64`   |
| Homebrew ARM64   | macOS `aarch64-apple-darwin` through Homebrew | `native-macos-arm64`   | `cem_ml_cli_native_brew_arm64`    |
| Windows AMD64    | `x86_64-pc-windows-msvc`                      | `native-windows-amd64` | `cem_ml_cli_native_windows_amd64` |

The browser command export used by Studio remains a supported programmatic
surface, but it is not another terminal CLI platform. Additional native targets
are not implied by the portability of the Node/WASM npm package.

### Common Version Authority

The common `cem_ml` project is the only source of the CEM-ML platform version.
With the current repository layout, the authoritative field is
`packages/cem_ml/Cargo.toml` `[package].version`. If a generated neutral version
manifest is introduced later, it must live under the common project and be
generated from or replace that one source; parallel manually edited version
files are forbidden.

A release synchronizer must copy the common version exactly into:

- the common `cem_ml_cli` crate manifest;
- `@epa-wg/cem-ml/package.json`;
- `@epa-wg/cem-ml-cli/package.json` and its exact runtime dependency;
- `@epa-wg/cem-studio/package.json` and its exact CLI dependency;
- every native target package manifest, archive name, package-manager manifest,
  embedded version record, checksum/provenance/SBOM record, and release index;
- every promoted semi-native target manifest, executable metadata, embedded
  runtime version record, and provenance record;
- Studio's capability manifest, service-worker cache/build id, and About view;
  and
- the machine version/capability output fixtures for all runtimes.

The proposed Nx `cem-ml-platform` release group must use a fixed project
relationship and exact internal dependency versions. No member derives a
version from its own conventional-commit history, package-manager metadata, or
publication time. A Studio-only, npm-CLI-only, or platform-packaging change that
requires publication still advances the common `cem_ml` version and rebuilds
the complete release family from the same commit.

Release order follows the dependency graph, but all artifacts are staged and
verified before public promotion. A partial registry/package-manager failure is
retried with the same immutable version/artifacts and recorded in a release
index; it does not create a different Studio, npm CLI, or platform version.
Package-manager-specific build revisions may describe repackaging metadata, but
the embedded CEM-ML semantic version remains the exact common version.

### `@epa-wg/cem-ml`: Separate WASM Runtime NPM Package

`@epa-wg/cem-ml` is a generated deployment subproject for the low-level common
engine runtime. It contains `@epa-wg/cem-ml/wasm`, generated types,
schema-package assets, ABI/capability metadata, and integrity/version records. It
does not own the user-facing CLI argument parser, Node launcher, browser command
client, or npm `bin`.

This separation lets browser/Node libraries consume the engine without pulling
Studio or CLI presentation code and gives the CLI npm subproject one exact
runtime dependency.

### `@epa-wg/cem-ml-cli`: Separate Universal CLI NPM Package

`@epa-wg/cem-ml-cli` is its own publishable Nx subproject and npm deployment
package. It depends on the exact same-version `@epa-wg/cem-ml` runtime and owns:

| Package surface              | Purpose                                                                                                                                            |
| ---------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@epa-wg/cem-ml-cli/browser` | Dedicated-worker client, browser resolver adapters, progress/cancellation, command parsing/serialization, and capability discovery used by Studio. |
| `@epa-wg/cem-ml-cli/node`    | Node host adapters for files, standard streams, environment, signals, and process exit-code projection.                                            |
| npm `bin` named `cem-ml`     | Small Node ESM launcher over the Node adapter and shared typed command service.                                                                    |

npm supports a `package.json` `bin` mapping that installs a command shim and
makes a dependency's executable available to `npm exec` and package scripts
([npm `package.json` documentation](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#bin)).
That permits, conceptually:

```sh
npm install --global @epa-wg/cem-ml-cli
cem-ml validate data/catalog.json

# No global installation:
npm exec --package=@epa-wg/cem-ml-cli -- cem-ml validate data/catalog.json
```

Accepted 2026-08-13: command schema v1 is generated from the built native Clap
grammar and joined to the common Rust runtime-capability matrix. The npm Node
and browser APIs consume that generated schema through one parser/serializer;
project documentation and TypeScript hosts must not guess or separately
maintain flags. Exact flags therefore change only through the canonical native
grammar and a regenerated, versioned schema.

“Universal npm CLI” means this one npm deployment can run on supported Node
versions across operating systems/architectures on which the WASM and JS host
adapters work. It still requires Node and may lack native-only capabilities.
Machine version output identifies `runtime: wasm-node` and the one synchronized
CEM-ML version; capability output identifies Node, WASM ABI, and unavailable
features without inventing another package version.

The browser export must not import Node built-ins. The Node export must not
depend on DOM, service-worker, or IndexedDB APIs. Both converge on the exact
same-version `@epa-wg/cem-ml` runtime and normalized command/report contract.

If both npm and native installations expose `cem-ml`, ordinary `PATH` ordering
selects one. Documentation should recommend the explicit npm-exec form when the
WASM CLI is required. Runtime identity lets scripts reject an unintended host.

### `@epa-wg/cem-studio`: Separate Synchronized NPM Package

`@epa-wg/cem-studio` is a separate first-class publishable Nx subproject, not an
example inside the CLI, runtime, or component package. It contains:

- an application bootstrap for mounting/routing CEM Studio;
- production JS/CSS/assets plus a deployable static PWA artifact manifest;
- web app manifest, icons, worker and service-worker build inputs/outputs;
- Feature Tour/capability assets;
- types for supported bootstrap/deployment configuration; and
- package/build metadata recording the synchronized CEM-ML version and the CEM
  component/theme versions used by the build.

Its published runtime dependencies should be shaped like:

```json
{
    "version": "<cem-ml-common-version>",
    "dependencies": {
        "@epa-wg/cem-ml-cli": "<exact-cem-ml-common-version>",
        "@epa-wg/cem-components": "<tested-compatible-version>",
        "@epa-wg/cem-theme": "<tested-compatible-version>"
    }
}
```

The placeholders are generated release values, not ranges for manual editing.
Studio consumes the browser command service from `@epa-wg/cem-ml-cli/browser`;
the CLI package supplies its exact `@epa-wg/cem-ml` dependency. Studio must not
add a second independently resolved engine runtime.

The npm package and hosted PWA are two projections of one Studio deployment
build. A host may install the npm package and copy/bundle its static deployment
artifact, while the project can also publish an official hosted PWA from the
same verified output. Service-worker URLs and scope are deployment-origin
concerns; importing the npm module must not silently register a service worker
for an embedding application.

Studio has no independent release cadence or version. Even a UI-only Studio
publication uses the next common CEM-ML version and participates in the complete
fixed-version platform release.

### Native CLI Platform Subprojects and Deployment Packages

The common `cem_ml_cli` project owns platform-neutral native CLI source and
dispatch, but it is not itself a deployment package. Each of the three supported
native targets has an explicit Nx deployment subproject that cross-compiles or
natively compiles, packages, signs, and verifies only that target's artifacts.
Publication scope is narrower than the development matrix.

| Deployment subproject             | Rust target                | Required artifacts/channel inputs                                                              |
| --------------------------------- | -------------------------- | ---------------------------------------------------------------------------------------------- |
| `cem_ml_cli_native_linux_amd64`   | `x86_64-unknown-linux-gnu` | Tagged GitHub Release archive, `.deb`, APT metadata, checksum/signature, SBOM, and provenance. |
| `cem_ml_cli_native_brew_arm64`    | `aarch64-apple-darwin`     | Local archive and Homebrew projection validation; release publication deferred.                |
| `cem_ml_cli_native_windows_amd64` | `x86_64-pc-windows-msvc`   | Local ZIP/MSI and WinGet projection validation; release publication deferred.                  |

Each platform project owns or produces the inputs for its potential distribution
channel. The active APT repository is a thin aggregation project that consumes
the Linux artifact and exact common version; it must not rebuild the binary or
introduce another version. Homebrew and WinGet outputs are validation-only.

The published Linux CLI binary/archive is preserved as an asset on the tagged
GitHub Release for the common CEM-ML version. Asset names include the version,
operating system, architecture, and ABI where relevant. Checksums, signatures,
SBOMs, provenance, and a release index are uploaded beside it and the WASM/npm
units. APT resolves these immutable versioned assets, not a mutable “latest”
build URL. The complete three-unit set is staged before publication, and a
published binary is never replaced or deleted. Corrections use a new common
version. GitHub Releases are explicitly designed to package tagged software
with downloadable binary assets
([GitHub Releases documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases));
GitHub's release-management guidance describes staging assets in a draft before
making a release immutable
([GitHub release-management documentation](https://docs.github.com/en/repositories/releasing-projects-on-github/managing-releases-in-a-repository)).

Homebrew formulae are package definitions installed through `brew install`
([Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)). APT consumes
architecture-specific packages from configured repositories and manages
dependencies/upgrades
([Debian package management](https://www.debian.org/doc/manuals/debian-reference/ch02)).
`cargo install` remains a source-build fallback rather than a platform package
([Rust binary installation](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html)).

The initial native scope stops at Linux AMD64, Homebrew ARM64, and Windows AMD64.
Other targets and channels are outside initial support. A later addition requires
an explicit scope change, a new project, signing and
install/upgrade/uninstall coverage, and a release-matrix entry.

### Wishlist: Semi-Native Node/WASM Executables

A later deployment option may package the exact synchronized
`@epa-wg/cem-ml-cli` Node launcher, its `@epa-wg/cem-ml` WASM runtime, and all
required runtime assets together with Node into a self-contained executable.
This would remove the end user's Node installation requirement while retaining
the `wasm-node` implementation rather than compiling a native Rust CLI. It is
therefore **semi-native**, not a replacement for the native target packages.

The preferred experiment is Node.js Single Executable Applications (SEA).
Current Node documentation describes SEA as a way to distribute a Node
application to a system without Node installed, supports embedded assets, and
marks the facility as active development
([Node.js SEA documentation](https://nodejs.org/api/single-executable-applications.html)).
The archived Vercel `pkg` project can be evaluated for compatibility lessons or
legacy migration only: its maintainers deprecated it at version 5.8.1
([Vercel `pkg` repository](https://github.com/vercel/pkg)). It should not become
the default production packager without a maintained successor and an explicit
security/ownership decision.

| Option                                 | Advantages                                                                                                                             | Costs and risks                                                                                                                                                                      |
| -------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Native Node.js SEA                     | Maintained with Node, embeds the launcher and WASM assets, removes the user-side Node prerequisite, and exposes an official asset API. | Still in active development; requires target-specific building/signing and validation of module, snapshot, code-cache, and asset constraints.                                        |
| Vercel `pkg` or a compatible successor | Familiar single-file Node packaging model and useful prior art for dependency/asset discovery.                                         | The original project is archived and deprecated; its frozen Node/runtime assumptions and security-update ownership make it unsuitable as the default without a maintained successor. |

If promoted from the wishlist, the experiment should initially produce only
Linux AMD64, macOS/Homebrew ARM64, and Windows AMD64 executables. Each output
must have its own Nx deployment subproject, use the exact common `cem_ml`
version, identify itself as `wasm-node-sea` (or `wasm-node-pkg` for an explicit
legacy experiment), and publish checksums, SBOM, provenance, signing, and
install/uninstall tests. It must not share an artifact name or runtime identity
with the native Rust package for the same platform. Any promoted semi-native
binary is also preserved as a distinctly named asset on the matching tagged
GitHub Release.

Before adoption, measure executable size, startup and transformation
performance, WASM asset loading, temporary-file behavior, command parity,
updater behavior, antivirus/notarization impact, signing reproducibility, Node
security-update cadence, licenses, and per-target build constraints. The Node
runtime and WASM engine remain embedded dependencies that must be rebuilt and
republished when either receives a relevant security update.

### Cross-Distribution Contract

All synchronized npm/WASM, native CLI, and any promoted semi-native deployment
projects must preserve:

- exactly one version originating from common `cem_ml`;
- one command/request schema, normalized run-plan semantics, diagnostic/report
  schemas, exit policy, and source-map model;
- the same command name and option meanings for shared capabilities;
- explicit `wasm-browser`, `wasm-node`, `wasm-node-sea`, `wasm-node-pkg`, or
  native target identity in capability output without changing the common
  version;
- parity fixtures and explicit expected native-only capability differences;
- immutable checksums/provenance linking every deployment artifact to the same
  source commit and common version;
- durable tagged GitHub Release assets for every release-scoped native or
  promoted semi-native binary, with active package channels resolving those
  versioned assets;
- no automatic fallback between native, npm/WASM, and semi-native runtime
  families; and
- no use of runtime-specific output as an undocumented interchange contract.

The Studio PWA always uses its exact synchronized `@epa-wg/cem-ml-cli`
dependency. It cannot discover or execute an installed OS binary directly. A
later native bridge must be an explicit, permissioned integration such as the
native-messaging option in [`integrations.md`](./integrations.md), with
capability and common-version negotiation.

## Consumer Semantic Theme Contract

CEM Studio must consume `@epa-wg/cem-theme` from this workspace as its only
visual design-token system. The application should:

- import `@epa-wg/cem-theme/dist/lib/css/cem-combined.css` exactly once at the
  app shell;
- place one supported theme scope on the shell: `cem-theme-light`,
  `cem-theme-dark`, `cem-theme-contrast-light`, `cem-theme-contrast-dark`, or
  `cem-theme-native`;
- persist the user's selected mode as a small preference and honor the native
  mode when selected;
- use semantic action, state, surface, layering, stroke, focus, selection,
  typography, timing, coupling, control, shape, and breakpoint tokens;
- keep all CEM components and Studio composites in light DOM;
- never copy generated CEM CSS into the app or introduce another design-token
  or utility-framework vocabulary; and
- verify light, dark, both contrast modes, forced colors, keyboard focus, and
  CEM interaction-size requirements.

Graph node states, diagnostic severity, source selections, diffs, and execution
states must never be communicated by color alone. Studio-specific CSS may define
aliases such as `--cem-studio-graph-running` only by mapping them to an existing
semantic CEM token. If a missing semantic is broadly useful, add it to
`@epa-wg/cem-theme` with its specification and generation tests instead of
inventing a private raw value.

The package-local
[`CEM Theme AI Instructions`](../packages/cem-theme/src/lib/tokens/cem-theme-ai-instructions.md)
remain the styling implementation reference.

## Browser Command Service

### Typed Operations, Not a Shell Process

The browser should call a versioned request API conceptually shaped like:

```ts
import { createCemMlBrowser } from '@epa-wg/cem-ml-cli/browser';

const engine = await createCemMlBrowser({ worker: true });
const result = await engine.run({
    protocolVersion: 1,
    command: 'validate',
    inputs: [
        {
            uri: 'studio://project/catalog/data/catalog.json',
            contentType: 'application/json',
            schema: 'studio://project/catalog/schema/catalog.schema.json',
        },
    ],
    report: { projection: 'json' },
});
```

This remains a convenience-level illustration, not a second frozen TypeScript
API. The exact request/resource/result/stale contract is command-service
protocol v1 in
[`cem-ml-deployment-contract.md`](cem-ml-deployment-contract.md#command-service-protocol-v1).
The browser wrapper constructs that Rust-owned request from the normalized
library run plan and returns structured outputs, diagnostics, source maps,
artifact handles, and a report. Terminal text is only an optional presentation
generated from that result.

The app may later provide a CLI-like command bar for users who know `cem-ml`
syntax. That parser must map an allowlisted command grammar to the same typed
request. It must not invoke a shell or permit command substitution, arbitrary
JavaScript, native paths, or undeclared network access.

### Command-to-Workbench Mapping

| Native workflow          | Studio presentation                                                                                                                | Browser target                                                               |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `parse`                  | Source editor plus AST/DOM/event projection tabs and parser-stage report.                                                          | MVP when the selected parser is WASM-capable.                                |
| `validate`               | Inline ranges, diagnostics list, effective schema/identity, and source trace.                                                      | MVP.                                                                         |
| `check`                  | Combined validation and configured policy checks for a project or data set.                                                        | MVP after multi-resource run plans.                                          |
| `inspect`                | Expandable document/resource/schema summary and selected-node details.                                                             | MVP.                                                                         |
| `convert`                | Immutable input on the left, generated output on the right, identity selectors, loss/diagnostic report, and download/copy actions. | MVP after validation.                                                        |
| `query`                  | Data source, explicit query language, query editor, variable/config editor, and typed result preview.                              | Second milestone.                                                            |
| `transform`              | Input data set, transformation config/template, output preview/diff, stage trace, and graph view.                                  | Second milestone.                                                            |
| `trace`                  | Timeline/table over structured observability events with source navigation.                                                        | Second milestone.                                                            |
| `bench`                  | Repeatable browser profile with environment disclosure.                                                                            | Optional; never compare browser numbers directly with native budgets.        |
| `fixture`                | Run bundled or user project examples as tests. File globs and repository parity suites are native-only.                            | Later browser-specific example runner.                                       |
| schema/plugin management | Read-only bundled capability catalog first.                                                                                        | Mutation/installation is later and must be signed, versioned, and sandboxed. |

### Capability Discovery and Parity

The engine must expose a versioned capability manifest containing:

- engine, npm package, WASM ABI, report schema, and schema-package versions;
- available commands and command options;
- supported input, schema, query, transform, formatter, colorizer, and output
  identities;
- unavailable or degraded native capabilities and a reason code;
- resolver schemes and resource limits;
- threading/streaming support; and
- maximum recommended source, result, event, and graph sizes.

The UI must disable or label unsupported choices. It must never silently choose
a different parser, query language, schema, network resolver, or transform
implementation to make an operation appear successful.

Native/WASM parity tests should run the same portable request fixtures through
the Rust API, native CLI projection, and browser package and compare normalized
reports, diagnostics, outputs, and source maps. Environment-specific metadata
may be excluded explicitly.

### Worker Protocol

The page/worker protocol should support:

- `initialize`, capability negotiation, and schema-package preload;
- `run`, progress events, observability events, final result, and structured
  failure;
- a stable request id, project revision, and resource-version map;
- cancellation and stale-result rejection;
- lazy retrieval of large output, trace, graph, and variable chunks;
- worker restart after a fatal error or hard cancellation; and
- deterministic disposal of resource handles and large buffers.

Transfer `ArrayBuffer` payloads where useful rather than repeatedly cloning
large byte arrays. Do not expose raw Rust pointers or WASM memory views as a
public contract.

### Virtual Resources

All project resources should have logical URIs. Suggested schemes are:

| Scheme                                | Meaning                                                                      |
| ------------------------------------- | ---------------------------------------------------------------------------- |
| `studio://<project-id>/<path>`        | Editable local project resource resolved from IndexedDB.                     |
| `sample://<package>/<version>/<path>` | Immutable bundled schema-package/example resource.                           |
| `https://…`                           | Explicit user-selected remote resource, subject to CORS and resolver policy. |
| `blob:`                               | Ephemeral imported/downloaded bytes for the current session only.            |

Human paths are presentation metadata; stable ids and logical URIs own
references. Renaming a tree label must not silently break a transformation
graph. `file://` is not a portable browser resource scheme and should not be
accepted as a hidden alias.

## Project and Subproject Model

One browser origin may contain several workspaces. A workspace owns projects;
projects and subprojects form the navigation tree. Executable resources are
referenced by stable id and URI, not nested as anonymous blobs in every run
configuration.

| Entity               | Purpose                                                                                                                                               | Important fields                                                                                                        |
| -------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| Workspace            | Local collection and preference boundary.                                                                                                             | id, name, active project, storage schema version, created/updated revision.                                             |
| Project              | Top-level portable unit and export/sync boundary.                                                                                                     | id, name, description, root URI, project schema version, child ordering, provider binding.                              |
| Subproject           | Nested feature, scenario, or domain grouping.                                                                                                         | id, parent id, name, description, children, tags.                                                                       |
| Workbench page       | Persisted executable view under a project/subproject. Its kind selects validation, inspection, conversion, query, transformation, graph, or trace UI. | id, parent id, name, kind, run-config id, referenced resources, presentation state, optional latest result-snapshot id. |
| Data set             | Named ordered selection of one or more data resources with effective input defaults.                                                                  | resource ids, content/schema defaults, namespace/module map, variables, base URI.                                       |
| Resource             | Source/config/query/template/schema content.                                                                                                          | id, logical URI, role, source mode, content type, schema, encoding, text/blob or URL reference, revision, hash.         |
| Run configuration    | Shared normalized options for an operation.                                                                                                           | command, inputs, outputs, policy, budgets, formatter/colorizer/report preferences.                                      |
| Conversion           | Input/output identity scenario.                                                                                                                       | input data set, target identities, output options, expected losses/diagnostics.                                         |
| Query                | Query source and explicit query identity.                                                                                                             | language, source resource, data set, variables, expected result.                                                        |
| Transformation       | Template/module/config plus its input and output contracts.                                                                                           | config/template ids, input data set, entry point, query language, parameters, result identity.                          |
| Transformation graph | Nodes and explicit data/control edges over transformations.                                                                                           | graph config id, inputs, stages, exports, edge mapping, layout hints.                                                   |
| Result snapshot      | Optional pinned derived result for comparison or examples.                                                                                            | run-config revision, input hashes, engine/package versions, outputs, report, created time.                              |

Every mutable record needs a stable id, monotonically changing revision, and
timestamps. Every run result needs the exact project revision, resource hashes,
engine/package versions, effective identities, resolver-policy stamp, and run
config provenance required to decide whether it is stale.

### Source Modes

A resource supports these source modes:

| Mode               | Behavior                                                                                                                                       |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------- |
| Inline text        | Edited in Studio and saved transactionally to IndexedDB. Best default for examples, configs, queries, and modest data.                         |
| Inline binary/blob | Imported bytes with explicit content type; previewed through a safe projection.                                                                |
| URL reference      | Stores URL, request policy, declared identity, and optionally a last successful local snapshot. Fetch is explicit and provenance is preserved. |
| Bundled sample     | Read-only versioned app/package asset. “Edit” first clones it into an inline project resource.                                                 |
| Provider resource  | Later remote-storage reference with local working copy, base revision, sync status, and conflict metadata.                                     |

URL credentials, authorization headers, signed URLs, cookies, and bearer tokens
must never be serialized into a project export or ordinary local resource
record. Authentication belongs to a session-scoped provider adapter.

### Mutation and Recovery

- Autosave inline edits after a short debounce and on explicit save.
- Commit related record/resource changes in one IndexedDB transaction.
- Show `saving`, `saved`, `failed`, `offline`, `stale result`, and `conflict`
  states without relying only on color.
- Use a recoverable trash state for project/subproject/resource deletion before
  permanent removal.
- Preserve an undo/redo command history for the current session; later add
  lightweight local checkpoints for structural changes.
- Export a project before destructive migration or when a storage quota warning
  is reached.
- Coordinate concurrent tabs with revision checks and `BroadcastChannel`;
  same-origin browsing contexts and workers can communicate through that API
  ([MDN Broadcast Channel](https://developer.mozilla.org/en-US/docs/Web/API/Broadcast_Channel_API)).

## Initial Feature-Tour Project

On first launch, Studio should install an editable copy of a versioned,
read-only seed named **CEM-ML Feature Tour**. Updating the PWA may install a new
seed version, but must never overwrite the user's copy. “Reset” creates another
copy and names the source seed/version.

The initial generated seed baseline was completed on 2026-08-21. A deterministic
generator selects one manifest-declared passing example from each of the 31
registered schema packages, carries transitive local schema dependencies, and
records source hashes plus the exact browser capability identity. A generated
CEM-ML transformation graph—not application copy code—emits the project,
catalog, run configurations, original bytes, and sample-cache inventory. Studio
loads those resources with integrity checks and validates them through one real
browser command worker. Relative reads use the project-local
`cem-studio://validation/` URI space backed only by digest-verified inline
command resources; they never fall through to browser filesystem access.
The immutable seed ID is `cem-ml-feature-tour-seed`, the first editable copy is
`feature-tour`, and reset allocates the next unused suffix without replacing an
existing or trashed copy.

The first offline workbench slice was completed on 2026-08-21. It opens the
generated CEM-ML example through `cem-textarea`, commits with the repository's
expected project/resource revisions, and refuses to validate until the exact
bytes, hash, and advanced revisions reload from IndexedDB. Those durable
revisions are carried into the browser command ledger. The retained native
result supplies structured diagnostics, report counts, execution identity, and
origin-first source-map frames; diagnostic and provenance selection project
byte ranges back into the editor. All visible editing, status, result switching,
structured rows, and navigation use production `cem-components` on the shared
`cem-elements` runtime. Offline Chromium reloads and revalidates the same
revision, and a result is marked stale whenever its draft or durable revision
is no longer current.

```text
CEM-ML Feature Tour
├── 00 Start Here
│   ├── Parse a CEM document
│   ├── Validate and navigate diagnostics
│   └── Inspect AST, DOM, events, report, and source map
├── 01 Content Types
│   ├── CEM data and projections
│   ├── Web and structured data
│   ├── Styling and math/vector data
│   ├── Schemas and schema packages
│   └── Queries, templates, and transforms
├── 02 Validation
│   ├── Valid documents
│   ├── Syntax diagnostics
│   ├── Schema diagnostics
│   └── Multi-source and referenced-schema cases
├── 03 Conversion
│   ├── JSON ⇄ YAML
│   ├── XML/HTML/CEM projections
│   └── Loss, output identity, and source-map examples
├── 04 Query Lab
│   ├── CEM-QL
│   ├── CSS selector
│   └── XPath
├── 05 Transformations
│   ├── Native CEM template
│   ├── CEMT configuration
│   ├── XSLT compatibility example
│   └── Multiple inputs and outputs
├── 06 Transformation Graphs
│   ├── Linear graph
│   ├── Branched graph
│   ├── Collections and exports
│   └── Stage trace, diagnostics, and source provenance
└── 07 Reports and Safety
    ├── Structured report projections
    ├── Resolver policy and budgets
    ├── Remote URL/CORS behavior
    └── Safe HTML preview
```

The Content Types branch must be generated from the bundled capability/package
manifest. At the time of this proposal, the workspace package manifests include:

- CEM-ML, native templates, transform configuration, CEM-QL, CSS selectors, and
  XPath;
- HTML, XHTML, XML, JSON, YAML, CSV, Markdown, CSS, SCSS, SVG, and MathML;
- CEM schemas, schema packages, JSON Schema, Relax NG, XSLT; and
- CEM AST, DOM, and event projections.

The tree should show only capabilities in the loaded WASM build. A package that
is present in the repository but not browser-capable remains visible only as an
explanatory disabled item if that helps teach the difference. Example content
should come from schema-package manifests so samples, formatters, colorizers,
and expected diagnostics do not drift from the engine.

## Application Workbench

### Responsive Layout

An expanded desktop layout should provide:

```text
┌──────────────── command bar / run status / theme / install ────────────────┐
│ Project explorer │ editor or configuration form │ preview / inspector      │
│                  │                              │                          │
├──────────────────┴──────────────────────────────┴──────────────────────────┤
│ diagnostics / report / event trace / graph execution console              │
└─────────────────────────────────────────────────────────────────────────────┘
```

At compact widths, these become navigable panes rather than squeezed columns.
Use CEM semantic breakpoints based on available space, not device names. Preserve
the selected resource, run, diagnostic, and graph node while switching panes.

### Primary Screens

1. **Home and projects:** recent projects, create/import, Feature Tour, storage
   health, offline/update status, and optional provider status.
2. **Project explorer:** accessible tree with project, subproject, data set,
   resource, configuration, conversion, query, transformation, and graph node
   types; keyboard reordering and context actions.
3. **Resource editor:** source, identity, schema, base URI, URL/snapshot, revision,
   and validation state.
4. **Run workbench:** command-specific form generated from the typed request
   schema, with an advanced normalized-config preview.
5. **CLI Command:** editable round-trip command, resolved input/config/output
   panes, copy actions, change preview, and an Apply target for the current,
   existing, or newly named project page.
6. **Results:** structured data, rendered-safe preview, raw text/bytes,
   diagnostics, report, event trace, source trace, and download.
7. **Transformation graph:** authored nodes/edges, validation, runnable stages,
   active/completed/failed state, edge data preview, timings, and click-to-source.
8. **Project settings:** defaults, resolver/network policy, budgets, storage,
   export, provider wishlist integration, and destructive actions.

### CLI Command View and Round Trip

Every executable Workbench page should have a **CLI Command** view. It is both a
reproducibility view and an alternate editor for the page's typed run
configuration.

The view contains:

| Pane         | Contents                                                                                              | Copy behavior                                                                                                              |
| ------------ | ----------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| Command      | Canonical `cem-ml` command generated from the active page, editable as plain text.                    | **Copy command** writes the exact displayed text.                                                                          |
| Inputs       | Ordered resolved inputs with URI/path, content type, schema, revision/hash, and text or safe preview. | Copy one input or all textual inputs; binary inputs offer download, not guessed text.                                      |
| Config       | Authored config plus a separate effective normalized-config view, with inherited/default provenance.  | Copy authored config, effective config, or the selected section.                                                           |
| Output       | Current primary output(s), identity, diagnostics/report summary, and stale/current status.            | Copy textual output, selected output, report, or download binary output.                                                   |
| Reproduction | Command plus a manifest of the referenced inputs/config/output identities and versions.               | **Copy all** produces one redacted plain-text reproduction block; it does not inline secrets or unbounded/binary payloads. |

Clipboard writes must happen from an explicit user action, report success or
failure accessibly, and retain selectable text as a fallback. The asynchronous
Clipboard API is restricted to secure contexts and browsers may impose
additional user-activation/permission rules
([MDN Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API)).

#### Command Projections

The same typed run plan has two useful command projections:

1. **Studio command:** uses stable `studio://` and `sample://` logical URIs and
   can be pasted into another CEM Studio command page after its referenced
   project resources are imported or available.
2. **Exported native command:** uses project-relative paths from the portable
   project tree and is runnable after project export with a compatible native
   `cem-ml` version.

The UI must label which projection is displayed. A `studio://` command must not
be presented as directly runnable by a native CLI that has no Studio resolver.
Native command serialization should be generated from the shared command
schema, disclose its minimum CLI/capability versions, and quote arguments for a
selected target shell. Do not hand-maintain a second list of flags in the web
app.

The accepted portable authored-command resource is
`application/vnd.cem.cli-command+json` with schema
`https://cem.dev/ns/cli/command/1`, owned by
`packages/cem_ml/schema-packages/cli-command/v1`. It records its resource and
generated command-schema versions, compatible common CLI version, the fixed
`cem-ml` binary identity, and ordered literal argv excluding the binary name.
The resource is valid authored configuration for a Studio entry's existing
`run-config` role; it never stores a lowered request, effective configuration,
resolver snapshot, result, or normalized run plan. Node and browser hosts parse
that argv through the same generated native command grammar before Apply or
execution. Canonical command text is a projection of argv, so shell quoting is
not portable authority.

Studio maps both `parse` and `inspect` commands to the portable project's
existing `inspection` entry kind. Their distinct operation and projection/view
semantics remain in the authored command resource. This avoids inventing a
second project entry kind while keeping the page state lossless.

Command generation includes all semantically relevant current inputs, output
specs, transformation/conversion config, explicit content/schema/query
identities, policy/budget choices, and report options. Defaults may be omitted
only when the generated command records the compatible CLI version and omission
cannot change meaning. A “fully explicit” projection should be available for
diagnosis and long-lived reproductions.

#### Editing and Applying a Command

Editing switches the view from generated to draft mode. The browser command
parser continuously produces syntax/option diagnostics and a structured change
preview, but typing alone does not mutate project records or start a run.

The preview must show:

- the parsed command and effective run-plan change;
- inputs, configs, schemas, queries, transformations, and outputs that will be
  reused, relinked, created, or left unresolved;
- values changed from the selected page, including inherited/default values;
- capability, policy, missing-resource, type, and destructive replacement
  diagnostics; and
- whether the last output becomes stale and whether **Apply & Run** will create
  a new result snapshot.

The **Apply to** control is an editable, accessible combobox with these targets:

- **Current page**, including the current transformation or conversion;
- any compatible existing Workbench page in a selectable project/subproject;
- **New page** under the selected project/subproject; or
- a newly typed page name. If that name does not exist, Apply creates it. If it
  already exists, the UI resolves the stable existing id and requires an update
  or save-copy choice rather than creating an ambiguous duplicate.

Page creation happens only on **Apply**, **Apply & Run**, or **Save as new**.
The target parent and page name remain visible before confirmation. Applying to
an existing page updates its typed run configuration and references in one
transaction. Applying a command of a different kind, such as changing a
conversion to a transformation, should default to a new correctly typed page;
replacing an incompatible page requires explicit confirmation.

The command may create a declared output resource as part of the transaction
when the change preview identifies it. A missing input, schema, query,
transformation config, or template is not silently fabricated: the user must
select an existing resource, import it, or explicitly create the proposed
placeholder/content. Failed parsing, unresolved required resources, duplicate
target names, capability gaps, or policy errors leave the project unchanged.

After a successful apply:

- the structured form, command text, and project tree represent the same typed
  run configuration;
- the new/updated page is selected;
- old results remain visible only as clearly stale history until rerun;
- **Apply & Run** executes exactly the applied revision; and
- the whole structural update is undoable according to project recovery rules.

The MVP repository boundary is the versioned `apply-command-page` command. It
accepts the exact authored CLI-command resource bytes, expected project
revision, explicitly resolved non-command resource ids, and a `current`,
`existing`, or `new` target. The repository parses the resource with the shared
browser CLI grammar, maps `parse` and `inspect` to `inspection`, resolves every
parsed `studio://` URI within the selected project, and validates the complete
proposed portable project before opening a strict IndexedDB write transaction.
The transaction writes the entry, command resource/blob, project revision,
search documents, and change journal together. Entry revision is the enclosing
project revision because Studio project v1 entries have no independent revision.
An unshared local run config retains its stable resource id and advances its
revision; a shared or URL-backed run config is isolated into a new local command
resource before the entry is relinked. Incompatible existing kinds fail closed
with `new` as the recommended disposition and require an explicit replacement
confirmation. The result returns the exact committed command bytes and
project/entry/resource revisions so Apply & Run can execute that commit rather
than an in-memory draft.

The workbench presents that boundary entirely through shared CEM controls.
`cem-select` chooses current, existing, or new targets; `cem-text-field` owns a
new page name; `cem-alert` reports persistence and freshness; and a transient
`cem-dialog` confirmation-gates an incompatible replacement while recommending
the non-destructive new-page path. Apply serializes the checked literal command
with the shared CLI resource codec, invokes `apply-command-page`, reloads the
returned entry and command resource, and byte-compares the commit without
executing it. Apply & Run invokes the browser worker only after that reload and
passes the exact committed command bytes and project/command revisions. A
repository revision observed before or during execution rejects Apply or marks
the resulting projection stale; it is never silently promoted to current.

This round trip requires a tested invariant:

```text
page state -> typed run plan -> command -> parsed run plan -> page state
```

The two page states must be semantically equivalent after normalization, apart
from explicitly excluded presentation state and generated ids. Comments or
formatting in manually authored commands may be preserved as draft text, but
the typed run plan remains execution authority.

### Editing Options

| Editor                          | Advantages                                                                                     | Disadvantages                                                                                                           | Recommendation                                                            |
| ------------------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Native `<textarea>`             | Small, accessible baseline; dependable text value and selection offsets; no editor dependency. | No syntax highlighting, folding, inline widgets, or rich completion UI.                                                 | MVP source editor behind a reusable adapter.                              |
| `contenteditable` source editor | Can style syntax spans directly.                                                               | Complex selection/IME/clipboard/undo behavior and easy source-offset drift; unsafe if authored text is treated as HTML. | Do not use as the canonical source editor.                                |
| CodeMirror/Monaco-class editor  | Mature language UI, diagnostics, completion, and large-document ergonomics.                    | Bundle size, theming/accessibility integration, worker complexity, and another adapter contract.                        | Evaluate after the textarea workflow and language-service API are stable. |

Editable HTML means editing HTML **source text**. Rendered HTML is a separate,
read-only-by-default preview. Editor adapters must use the same versioned
document model so a richer editor can replace the textarea without changing
project storage or engine requests.

### Preview Rules

- Structured data uses an expandable tree/table plus a raw-source tab.
- Studio v1 caps a primary source and each dependency at 8 MiB, the complete
  submitted resource set at 16 MiB and 129 resources, an exact result at 16 MiB,
  inline text/markup projection at 256 KiB, and retained structured views at 100
  rows. Oversize results remain available only when the exact byte result fits
  the result cap; oversize active markup is download-only rather than truncated
  into a potentially misleading document.
- Conversion always shows input and output identities, text/bytes, diagnostics,
  loss metadata, and source mapping side by side.
- Transformation shows input data set, effective config/template/query, result,
  stage trace, and diff where meaningful.
- Transformation graphs show both the authored graph and the latest execution
  overlay; graph layout hints are presentation metadata, not execution order.
- Selecting a diagnostic, result node, graph stage, or source-map frame
  navigates to the exact resource/range.
- Large results are paged or lazily expanded. The app must not stringify an
  unbounded tree into the DOM.
- Binary data is never decoded by guessing. Use its declared identity and a
  safe registered projection, otherwise provide metadata and download.

User-authored or generated HTML must not be inserted into the Studio DOM. A
rendered preview should use a sandboxed iframe without `allow-scripts` or
`allow-same-origin` by default, a restrictive preview CSP, bounded resources,
and an explicit opt-in if a future scenario needs more capability. `srcdoc` is
an injection sink and unsandboxed content can access its parent origin
([MDN `srcdoc` security](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc)).
Plain source views must use text nodes or equivalent escaped rendering.
For the first public release, declared UTF-8 text uses this bounded raw-text
projection; HTML, XHTML, and SVG may additionally use only the isolated preview
above; binary, unknown, invalid-text, and oversize active results are
download-only. The current textarea editor is sufficient for that release.
Richer schema-aware editing remains an adapter upgrade only after profiling and
user evidence justify its bundle, memory, and accessibility cost.

## Local-First Persistence

### Storage Choice

“Preserved in local storage” should mean durable browser-local application
storage, not putting full projects into the synchronous `window.localStorage`
API.

| Browser storage            | Use in Studio                                                                                                  | Reason                                                                                                                                                                                                                                   |
| -------------------------- | -------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| IndexedDB                  | Canonical local working store for projects, records, source text, blobs, reports, and migration metadata.      | Asynchronous, transactional, indexed, and intended for significant structured data/files ([MDN IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)).                                                              |
| `localStorage`             | Tiny preferences only: last workspace id, theme mode, pane preference, dismissed update notice.                | Synchronous operations block JavaScript and Web Storage is size-limited ([MDN Web Storage](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)).                                                                           |
| Cache Storage              | Versioned app shell, WASM, bundled capability catalog, and immutable sample assets.                            | Service-worker/offline asset cache, not a transactional project database.                                                                                                                                                                |
| Origin Private File System | Test/cache substrate for large private blobs or engine scratch data; never the user's portable owner.          | Fast worker-oriented storage, but origin-private, quota-bound, and invisible to the user; it is not backup or Git integration ([MDN OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)). |
| File System Access handles | Optional user-selected file/directory provider; only the handles and conflict base live in `providerBindings`. | Direct write-back in supporting secure browsers, but permission can return to `prompt` and there is no cross-file transaction; IndexedDB plus import/export remains required.                                                            |

The accepted IndexedDB stores are `meta`, `projects`, `entries`, `resources`,
`blobs`, `runs`, `resultSnapshots`, `providerBindings`, `syncQueue`, `trash`,
`changes`, and `searchDocuments`. `projects` and `entries` deliberately match
the accepted portable `project`/`entry` vocabulary; IndexedDB does not introduce
parallel `workspace`/`node` domain names. Schema upgrades must be versioned,
transactional, tested against old fixtures, and recoverable by export when
migration fails.

### File System Access Provider

`@epa-wg/cem-studio/file-system-provider` is an optional host adapter over the
repository, not a second project model. It supports two retained binding scopes:
a file handle for one existing resource and a directory handle for a complete
portable project. Bindings record the handle, scope, display name, optimistic
binding revision, exact project/resource revision, local hash, and external hash
snapshot. Repository binding writes use the current project and binding
revisions, append to the durable `changes` journal, and remain excluded from
`export-project`.

Opening a file, selecting a directory, and requesting a retained handle's
permission are explicit user actions. Passive status and background reconnect
only call `queryPermission()` and surface `ready`, `prompt-permission`,
`denied-permission`, `unbound`, or `unsupported`; they never silently escalate.
The visible shell uses production `cem-action`, `cem-badge`, and `cem-alert`
components for open, bind, reconnect, save-back, import, export, conflict, and
fallback states. No filesystem capability is passed to the CEM-ML worker.

A portable directory owns `project.cem` plus every normalized, contained
resource path, including the retained local snapshot for a URL-backed resource.
The browser command service converts the manifest through the native
Studio-project lifecycle adapters, and repository import performs the same
schema and resource-hash validation used by every other import. The provider
retains `studio://` root/resource identity and rejects absolute, escaping,
empty, or reserved `.cem-studio/` paths. Missing declared files may be created
during explicit write-back; writing a URL snapshot does not fetch or claim that
the remote resource is current.

Before a resource write, the provider compares the current external SHA-256
with the retained base and refuses a stale write before calling
`createWritable()`. Directory write-back performs the same preflight for the
manifest and every declared resource, opens/stages all writables, closes them,
rereads every file, and advances the binding snapshot only after exact hashes
verify. The File System Access API does not provide one atomic transaction over
multiple files: a close failure can leave a subset externally replaced. In that
case the IndexedDB project and old binding base remain authoritative, so the
next operation detects the difference and requires review/reopen instead of
silently accepting a partial save.

When the API is missing or permission is denied, Studio continues to edit and
autosave in IndexedDB. Import/export uses a deterministic
`application/vnd.cem.studio-project-bundle+json` envelope containing the same
normalized project and exact base64 resource bytes. This is a lossless archive
of the accepted portable tree, not alternate project semantics; import routes
back through the repository validator, and provider handles/bases never enter
the archive.

Browser storage is best-effort by default and quotas/eviction differ by browser.
Studio should display `navigator.storage.estimate()`, request
`navigator.storage.persist()` after the user creates/imports meaningful work,
handle quota errors, and keep export prominent. Persistence reduces automatic
eviction risk but is not a backup and users can still clear site data
([MDN storage quotas and eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)).

### Repository and `cem-elements` Boundary

IndexedDB is a browser implementation detail, not template or portable-project
vocabulary. Do not expose database names, object stores, indexes,
`IDBKeyRange`, callbacks, or transaction handles to CEM-ML. `cem-elements`
instead owns a generic clone-safe repository host contract that applications
register by logical identity. Studio registers `studio-projects`; Site may
register a separately scoped, read-only search cache.

The accepted substrate surface has three responsibilities:

1. A transient `repository-query` CEM-ML resource reads or subscribes to an
   allowlisted operation on a registered repository and projects its lifecycle
   into a data slice. It is analogous to `http-request`, but its request names a
   logical repository and operation rather than physical persistence details.
2. A read-only `storage-status` resource projects repository availability,
   opening/migration state, blocked upgrades, usage/quota, persistence state,
   and recoverable diagnostics. Requesting storage persistence requires an
   explicit user action; rendering the resource cannot call
   `navigator.storage.persist()`.
3. An injected JavaScript repository port owns `query`, `execute`, `subscribe`,
   and `status`. Query results are clone-safe values with request and repository
   revisions. Mutations are explicit commands and never execute merely because
   a template rendered or a data slice changed.

The v1 conceptual boundary is:

```ts
interface CemRepositoryPort {
    query(request: RepositoryQuery, signal?: AbortSignal): Promise<QueryResult>;
    execute(command: RepositoryCommand, signal?: AbortSignal): Promise<CommandResult>;
    subscribe(cursor: ChangeCursor, notify: () => void): () => void;
    status(): Promise<RepositoryStatus>;
}
```

This is a governed runtime envelope, not a new portable content type. If a
repository request later becomes authored, persisted, or transformed CEM-ML
content, it must first gain an explicit registered content type and schema.

The implemented declaration vocabulary is:

```cem
{repository-query
    @slice=projects
    @repository=studio-projects
    @operation=list-projects
    @parameters="{$datadom.attributes.query-parameters}"
    @live=true
    @cursor=0}
{storage-status @slice=storage @repository=studio-projects @live=true @cursor=0}
```

`parameters` is optional JSON; the example interpolates an instance attribute
such as `query-parameters='{"includeTrash":false}'` so it remains one declaration
value. Request revisions remain runtime-owned. `live` subscribes from the
optional non-negative durable cursor and requeries current state when a newer
cursor arrives. Both resources are lowered out of canonical
CEM-ML render plans before DOM diffing, expose clone-safe
`scheduled`/`loaded`/`failed` envelopes, reject stale completions, and clean up
on supersession, disappearance, or disconnect. The runtime accepts the
`CemRepositoryReader` surface returned by `CemRepositoryRegistry.readOnly()`,
whose capability surface omits `execute`; no storage-manager capability enters
the render path, so it also cannot request persistence.

Application orchestration remains in `@epa-wg/cem-studio`: migrations,
autosave policy, project commands, import/export validation, search corpus and
ranking, multi-tab conflict presentation, and storage recovery are not reusable
UI components. `@epa-wg/cem-components` owns every visible control and status
surface. A raw `<indexed-db>` widget or app-local search/AI control is forbidden.

### IndexedDB Schema, Transactions, and Coordination

Use one product-scoped Studio database per origin, not one database per project
and not one database name per application release. The database version governs
structural migrations while records remain keyed by stable project and resource
identities across application updates.

Store responsibilities are:

| Store              | Authority and lifecycle                                                                                              |
| ------------------ | -------------------------------------------------------------------------------------------------------------------- |
| `meta`             | Logical schema, semantic-migration, search-index, seed, and maintenance checkpoints.                                 |
| `projects`         | Authoritative normalized project metadata plus the current project commit revision.                                  |
| `entries`          | Portable project hierarchy records keyed by project and stable entry id.                                             |
| `resources`        | Resource identity, path, content/schema identity, revision, hash, and current blob reference.                        |
| `blobs`            | Content-addressed source/result bytes keyed by SHA-256; bounded maintenance reclaims unreachable blobs.              |
| `runs`             | Bounded command run metadata and provenance.                                                                         |
| `resultSnapshots`  | Bounded immutable run outputs referenced by run identity.                                                            |
| `providerBindings` | Active host-only File System Access handles and conflict bases; revisioned/journaled, never exported.                |
| `syncQueue`        | Future idempotent remote-provider outbox with base revisions; local file write-back does not use it.                 |
| `trash`            | Tombstones, retention metadata, and restore identity; purge and blob collection are separate maintenance operations. |
| `changes`          | Durable monotonic commit journal used for subscriptions, multi-tab invalidation, and derived-index recovery.         |
| `searchDocuments`  | Rebuildable normalized search documents keyed to exact source revisions; never project authority.                    |

Atomic autosave computes source bytes and SHA-256 before opening its transaction,
then opens one read/write transaction over every affected authoritative and
derived store. Inside the transaction it verifies the expected project and
resource revisions, stores the blob/resource update, advances revisions,
replaces the search document, and appends the durable change record. Any failed
check or write aborts the whole transaction. There is no silent last-write-wins
path; stale writes return the expected/current revision and hash as a structured
conflict.

`BroadcastChannel` is only a low-latency wake-up hint. The committed `changes`
journal is the source of truth: another tab resumes after its last change cursor
and requeries. `navigator.locks` may reduce avoidable contention but cannot be
required for correctness. Every open connection handles `versionchange` by
preserving any in-memory draft, closing the connection, and exposing a reload or
retry state. IndexedDB permits multiple clients but cannot perform an upgrade
until old-version connections close
([W3C Indexed Database API 3.0](https://www.w3.org/TR/IndexedDB/)).

Keep object-store/index changes inside `onupgradeneeded`. Potentially large
semantic conversions, blob maintenance, and search reindexing use resumable
logical migrations with `meta` checkpoints. Until a derived index is rebuilt,
the application may use a bounded scan fallback; it must not expose incomplete
index results as complete.

Use strict durability where supported for project saves, imports, trash, and
restore. Derived search maintenance and disposable caches may use relaxed
durability. Normalize `blocked`, `VersionError`, `QuotaExceededError`, aborted,
corrupt, migration-failed, and revision-conflict outcomes into stable repository
diagnostics. The IndexedDB specification reserves `QuotaExceededError` for
insufficient remaining storage or denied quota
([W3C Indexed Database API 3.0](https://www.w3.org/TR/IndexedDB/)).

### Deterministic Search and Optional Browser AI

Search is deterministic and fully functional without AI. IndexedDB has no
native full-text search, so every searchable page, component, token, project,
entry, resource, diagnostic, command, schema capability, transformation stage,
and retained run projects into a versioned `searchDocuments` record containing
stable identity, scope/kind, title/path/text/tags, normalized tokens, optional
trigrams, and exact source revision. Multi-entry token/trigram indexes retrieve
candidates; application policy intersects, filters, scores, snippets, and routes
them. The index is derived, disposable, and deterministically rebuildable.

CEM Site receives its canonical search corpus as a CEM-ML build artifact. It
may install that artifact into a product-scoped IndexedDB cache keyed by build
and corpus hash, but the cache is not source authority and cannot be required
for first-load search. Studio derives its mutable corpus from authoritative
project records and merges it with versioned bundled documentation/capability
records. Deleted or unauthorized records are excluded before ranking.

Both applications compose the visible search experience from existing CEM
controls: `cem-autocomplete`, `cem-option`, `cem-option-group`, `cem-list` or
`cem-tree`, `cem-progress`/`cem-progress-spinner`, `cem-alert`, `cem-dialog`,
`cem-action`, and `cem-badge`. Site or Studio owns query state, corpus selection,
filtering/ranking, result loading, navigation, and the optional AI policy; it
does not own replacement control DOM, keyboard, accessibility, or theme behavior.

Browser AI is an optional second stage over retrieved evidence:

```text
user query
    -> deterministic lexical/trigram retrieval
    -> bounded evidence chunks with stable ids and revisions
    -> optional on-device browser model
    -> validated answer/intent plus citations to supplied ids
    -> CEM component presentation beside the unchanged result list
```

Current browser APIs are capability globals such as `LanguageModel`,
`Summarizer`, `Translator`, `LanguageDetector`, `Writer`, and `Rewriter`, not a
stable `window.ai.*` namespace. Chrome documents Translator, Language Detector,
and Summarizer from Chrome 138, and the web Prompt API from Chrome 148; Writer
and Rewriter remain trial-stage capabilities. Implementations must feature-detect
each API and call its asynchronous `availability()` with the same requested
languages/modalities used to create a session
([Chrome built-in AI API status](https://developer.chrome.com/docs/ai/built-in-apis),
[Chrome Prompt API](https://developer.chrome.com/docs/ai/prompt-api)). Microsoft
Edge documents `LanguageModel` as a Canary/Dev developer preview, so Edge support
must be capability-reported rather than promised as a production baseline
([Microsoft Edge Prompt API](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/prompt-api)).

`cem-elements` may own a browser-AI host adapter and a read-only `ai-capability`
resource so availability, model-download, streaming, cancellation, context,
language, and failure states enter normal CEM data slices. Site and Studio own
the prompts, evidence projection, result validation, and user disclosure. Model
creation/download and inference are explicit user-intent operations; they are
not triggered by rendering `ai-capability`.

The model never receives direct IndexedDB access and never becomes search
authority. It may translate a query, propose an allowlisted structured filter,
rerank the retrieved candidate set, summarize supplied excerpts, or explain a
diagnostic. It may not generate unrestricted repository commands/CEM-QL, mutate
a project, invent a citation, or fetch/crawl the web. Returned ids are validated
against the supplied candidate set and answers retain the source revision set;
unknown citations fail and changed sources mark the answer stale. Literal ranked
results stay visible when AI is unavailable, downloading, cancelled, over quota,
unsupported, or wrong.

Prompt content uses an explicit field allowlist and excludes credentials,
provider bindings, file handles, secrets, and denied data-island fields. Treat
retrieved project/page text as untrusted prompt data because it may contain
instructions. Do not persist prompt history or model answers by default. Any
later AI cache is disposable and keyed by model capability identity, prompt
contract version, query hash, locale, and complete source-revision-set hash; it
is never part of project export. Browser-managed model files are outside the
application database and must not be copied into IndexedDB.

The current Prompt and task APIs are available to top-level windows and
same-origin frames but not Web Workers. Deterministic retrieval and IndexedDB
work may run in an application worker, while the browser-AI adapter presently
runs in the document lane and streams clone-safe state back to application
orchestration
([Chrome Prompt API worker boundary](https://developer.chrome.com/docs/ai/prompt-api),
[Chrome Summarizer API](https://developer.chrome.com/docs/ai/summarizer-api)).

### PWA and Offline Contract

The application needs a web app manifest, HTTPS deployment, install icons,
standalone display metadata, and a service worker. PWA installation varies by
browser/OS, so “installable” is a capability with browser-specific presentation,
not a guaranteed custom prompt
([web.dev PWA installation](https://web.dev/learn/pwa/installation)).

The service worker should precache a versioned minimal shell, theme assets,
worker, WASM, capability manifest, and Feature Tour assets. Service workers can
intercept requests and serve cached resources offline
([web.dev service workers](https://web.dev/learn/pwa/service-workers)). Project
records remain in IndexedDB; never copy mutable projects into an opaque static
asset cache.

| Operation                                                  | Offline behavior                                                                            |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| Open/edit local project                                    | Fully available.                                                                            |
| Parse/validate/query/transform with bundled WASM resources | Available when the needed engine/package assets are cached.                                 |
| Open bundled Feature Tour                                  | Available after initial installation/cache.                                                 |
| Fetch URL-backed source                                    | Use an explicitly selected last snapshot or report offline; never pretend it is current.    |
| Sync to account provider                                   | Queue only after the provider defines idempotency, base revision, and conflict rules.       |
| Install/update                                             | Online; active work remains on the current app/DB contract until update migration succeeds. |

An update must not activate halfway through an engine run or unsaved edit. Show
that an update is ready, finish/abort active requests, persist state, then reload
under an explicit migration plan.

#### Worker-safe static module decision gate

Status: **accepted and completed 2026-08-21**.

Module-map v3 (`https://cem.dev/ns/data/module-map/3`) is the accepted graph
contract. Its typed JavaScript/JSON imports and exact `moduleImports` edges let
CEM-ML rewrite only declared static imports, export-from edges, and quoted
dynamic imports to relative deployed URLs. It rejects undeclared or unused bare
edges, copies no discovered dependency, preserves v1/v2 byte and hash
compatibility, and reports deterministic source/output digest evidence when an
edge changes emitted bytes.

Studio now declares its CLI browser client, command worker, WASM wrapper/binary,
runtime JSON, CEM components/elements, theme, and application modules through
the paired v3 maps. The destination map is also emitted as the runtime-cache
inventory authority. A Playwright fixture serves only `dist/static`, starts the
real dedicated CLI module worker, initializes the bundled WASM, and executes the
CLI `version` command online. It then reloads with Chromium network access
disabled and repeats the command from the versioned shell/runtime caches. No
Vite/Rollup production bundle or package-specific CLI deployment loader is used.

#### Installable shell and offline project-survival gate

Status: **accepted and completed 2026-08-21**.

Studio installs the production `cem-components` declarations through the
`cem-elements` runtime and composes every visible shell control from that set.
Its mode selector persists exactly the five selectors defined by the repository
theme Markdown: `cem-theme-light`, `cem-theme-dark`,
`cem-theme-contrast-light`, `cem-theme-contrast-dark`, and
`cem-theme-native`. The install action appears only when the browser raises its
own install-ready event; Studio does not manufacture or promise an install
prompt.

Cache inventory v2 separates versioned `shell`, `runtime`, and `samples`
groups. The graph-emitted samples group contains the integrity-checked Feature
Tour project, all 31 schema examples, and the five portable-operation
workbenches. Offline navigation returns the cached shell with the service-worker registration scope
as its document base, preserving deep routes without assuming a root deployment.
Mutable project content never enters these caches.

A waiting service worker does not call `skipWaiting()` on install. The shell's
update coordinator blocks activation while engine requests are active, persists
dirty state first, and keeps the current worker active if persistence fails.
Only the explicit accepted activation message releases the waiting worker, and
controller replacement then reloads the application.

Browser fixtures install and render the production controls, exercise all five
theme modes, preserve/reopen/export exact IndexedDB project bytes, and verify the
activation barrier. The static-output Playwright gate serves only the 58
module-map assets and 66 graph-emitted files, executes the real CLI worker/WASM
online, navigates to a deep route with networking disabled, recovers the theme
and project bytes, and executes the worker again offline.

## URL-Backed Data

URL resources use `fetch()` in the worker or a bounded host resolver. For a
cross-origin response to be readable, the remote server must allow the Studio
origin through CORS; `no-cors` returns an opaque response whose body is not
available to the engine
([MDN Fetch and CORS](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch#making_cross-origin_requests)).

Each fetch should apply:

- explicit user action for the first network read and clear active-network
  status;
- resolver allow/deny/substitution policy and scheme/host/port rules;
- byte, time, redirect, decompression, recursion, and dependency-count budgets;
- declared or response content type with conflict diagnostics, never silent
  extension guessing;
- requested URL, final URL, response metadata, content hash, fetch time, and
  effective policy in provenance;
- optional local snapshot with `ETag`/`Last-Modified` revalidation metadata;
- no ambient cross-origin credentials by default; and
- cache controls that avoid placing private/authenticated content in the service
  worker's shared asset cache.

A hosted fetch proxy is a later service because it introduces SSRF, credential,
privacy, bandwidth, abuse, and content-retention obligations. It must not be a
transparent workaround for CORS.

## Portable Project Format

No single serialization is ideal for IndexedDB, Git review, NoSQL queries,
single-object storage, documentation, and quick sharing. Define one versioned
**logical project model** and several lossless or generated projections.

### Accepted Canonical and Projections

The accepted v1 identities are:

- CEM: `application/vnd.cem.studio-project+cem`;
- normalized JSON: `application/vnd.cem.studio-project+json`;
- schema: `https://cem.dev/ns/studio/project/1`; and
- JSON Schema artifact:
  `https://cem.dev/schema/studio/project.schema.json`.

The owning package is
`packages/cem_ml/schema-packages/studio-project/v1`. Its native Rust model,
schema-owned fixtures, and diagnostics are the semantic authority used before
any import write.

1. `project.cem` is the preferred human-authorable portable manifest and Git
   source of truth.
2. Keep sources, schemas, queries, templates, and transformation configs as
   separate files in their native formats. Do not embed every resource into one
   huge manifest.
   Executable entries may reference authored CLI-command JSON as their
   `run-config` resource when command argv is the page authority; generic CLI
   RunConfig JSON remains appropriate when only inputs, outputs, schema
   packages, resolvers, and scheduler configuration are authored.
3. Use normalized JSON records internally for IndexedDB, NoSQL documents, API
   requests, migrations, and deterministic snapshots. JSON is a projection of
   the same project schema, not an alternate semantic model.
4. Generate `README.md` for explanation, example previews, and links. Markdown
   is not a lossless project database.
5. Offer YAML only as an optional import/export projection if there is user
   demand. YAML typing, aliases, and dialect choices make it a poor sole
   canonical store; normalize it through the CEM-ML YAML package and the Studio
   project schema.

Suggested Git/directory layout:

```text
project.cem
README.md                         # optional/generated
data/
schemas/
queries/
transforms/
graphs/
expected/                         # optional pinned result fixtures
.cem-studio/
└── project.lock.json             # generated versions/hashes; optional in Git
```

The manifest references logical relative paths plus content/schema identities.
Provider ids, OAuth details, local absolute paths, signed URLs, UI pane sizes,
and transient run output do not belong in the portable manifest.

The portable tree has `project.cem` at its root. Resource paths must be
normalized, relative, contained by the project root, and outside the reserved
`.cem-studio/` provider-state directory. The implemented deterministic JSON
download envelope, or any future ZIP projection, is only a lossless archive of
this tree and cannot add an alternate manifest or semantic model.

### Deployment Format Matrix

| Target                 | Preferred representation                                                                          | Notes                                                                                                         |
| ---------------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Browser local store    | Normalized IndexedDB records plus blobs.                                                          | Efficient mutable working copy; export required for independent backup.                                       |
| Git repository         | `project.cem` plus native resource files and optional generated `README.md`.                      | Reviewable diffs, branches, history, and CI. Keep generated JSON only when another tool requires it.          |
| GitHub Gist            | Small multi-file project with `project.cem`; optionally a single JSON export for simple examples. | Convenient sharing, but weaker hierarchy, project size, and repository workflow.                              |
| S3/object storage      | Versioned ZIP/bundle containing the directory layout, or manifest plus content-addressed objects. | A single bundle simplifies atomic save/download; object sets improve deduplication but need a commit pointer. |
| NoSQL document service | Normalized JSON metadata/documents with large source/output blobs in object storage.              | Queryable account/project indexes; transactions and object consistency need service design.                   |
| Static demo deployment | Read-only `project.cem` and resources bundled with the PWA/site.                                  | Good for Feature Tours and documentation examples.                                                            |
| Documentation          | Generated Markdown plus fenced native sources and links to an importable bundle.                  | Human presentation only.                                                                                      |
| CI interchange         | Deterministic JSON run plan/report and the repository project tree.                               | CI should not reconstruct execution semantics from Markdown.                                                  |

The export contract must include schema version, engine constraints,
capabilities, resource hashes, and migration history. Import validates before
writing, shows conflicts/unsupported capabilities, assigns new local ids where
needed, and performs the final write atomically.

## Permanent Account Storage Wishlist

Account-backed storage is a provider layer over the portable project model. The
MVP remains useful without an account.

```ts
interface StudioProjectProvider {
    capabilities(): Promise<ProviderCapabilities>;
    listProjects(): Promise<RemoteProjectSummary[]>;
    pull(ref: RemoteProjectRef): Promise<ProjectBundle>;
    push(bundle: ProjectBundle, baseRevision?: string): Promise<RemoteRevision>;
    delete?(ref: RemoteProjectRef, baseRevision: string): Promise<void>;
}
```

The actual contract also needs authentication state, optimistic concurrency,
conflicts, progress, cancellation, retry/idempotency, provider metadata, and
secret redaction.

| Provider                         | Advantages                                                                                    | Disadvantages and service requirements                                                                                                                                                | Suggested use                                                                                                                                                                                                                                                                    |
| -------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Download/upload local bundle     | No account or server; user controls the file; works across deployments.                       | Manual backup and conflict management; browser file APIs vary.                                                                                                                        | MVP durable portability baseline.                                                                                                                                                                                                                                                |
| S3-compatible object storage     | Durable object/version model; cheap static bundles; content-addressed resources are possible. | Never ship permanent S3 credentials to the page. Needs account service, authorization, conflict metadata, CORS, lifecycle policy, and cleanup.                                        | Hosted project bundles and large artifacts. Use short-lived presigned requests; AWS documents that presigned uploads avoid giving the browser AWS credentials ([AWS S3 presigned uploads](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)). |
| NoSQL database plus object store | Natural user/project indexes, metadata queries, sharing flags, quotas, and sync cursors.      | Largest custom-service commitment: schema migrations, tenancy, authorization, backup, deletion/export, cost, privacy, and consistency.                                                | Full CEM account service if product demand justifies it.                                                                                                                                                                                                                         |
| Git repository                   | Open, portable, reviewable, branchable, and CI-friendly.                                      | OAuth/Git provider integration, commit conflicts, binary/large output handling, branch UX, and non-technical-user complexity. Browser-only local Git is not uniformly portable.       | Advanced users and project-as-code deployment. GitHub's Contents API can create/update files but requires revision-aware conflict handling and repository content permissions ([GitHub repository contents API](https://docs.github.com/en/rest/repos/contents)).                |
| GitHub Gist                      | Fast account-backed sharing; multi-file API; revision history; public or secret visibility.   | GitHub-specific authentication, provider limits, flatter organization, secret gists are unlisted rather than a general authorization system, and not suited to large/binary projects. | Small examples, bug reproductions, and shareable demos. The API supports multi-file create/update and requires authentication for writes ([GitHub Gists API](https://docs.github.com/en/rest/gists/gists)).                                                                      |

Provider binding should be optional metadata outside the portable project. A
local project remains fully editable when signed out. The UI distinguishes
`saved locally` from `synced remotely`; those states must never be collapsed
into one checkmark.

### Potential CEM Studio Service

If a hosted service is introduced, keep the browser engine local by default.
The service may provide:

- OAuth/OIDC login and short-lived provider tokens;
- project metadata, sharing policy, quotas, and sync revisions;
- presigned object upload/download URLs;
- optional NoSQL indexes and bundle commit records;
- CORS-safe remote fetch only through explicit policy;
- encrypted-at-rest project storage, deletion/export, retention, and audit
  behavior; and
- server-side CI/preview execution only as a distinct opted-in service with the
  same run contract and version disclosure.

Do not send project sources, queries, transformation inputs, or results to the
service merely for telemetry, installation, or account login.

## Security and Privacy

- Treat all source text, URL responses, imported bundles, transform output,
  schema-package assets, and provider metadata as untrusted.
- Run the engine in a worker and enforce resolver and execution budgets in Rust,
  not only in disabled UI controls.
- Use a restrictive application CSP. WASM compilation may require the specific
  `wasm-unsafe-eval` CSP source expression; do not broaden this to general
  `unsafe-eval`
  ([MDN CSP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy#wasm-unsafe-eval)).
- Do not use `eval`, `Function`, injected module scripts, or event-handler
  attributes for user-authored code.
- Render source as text. Render HTML only in the isolated preview boundary.
- Keep preview origin permissions, navigation, downloads, forms, popups,
  storage, scripts, and network access denied unless a scenario explicitly
  grants the minimum capability.
- Make all network access visible and cancellable; enforce URL and redirect
  policy after every resolution step.
- Never persist provider access tokens, cloud secret keys, arbitrary request
  headers, signed URLs, or captured passwords in exports or ordinary project
  data.
- Redact sensitive values from reports, event traces, recent-project thumbnails,
  crash reports, and share operations by default.
- Sign and version bundled schema packages; a future third-party package loader
  needs provenance, integrity, capability, and trust UI.
- Provide “delete local project,” “clear all local data,” “disconnect provider,”
  and “export my projects” workflows with clear consequences.

## Accessibility and Interaction

- Meet the existing `@epa-wg/cem-components` accessibility contract for names,
  keyboard operation, focus, validation, loading, and live regions.
- Use a real accessible tree/grid/list pattern as appropriate; do not simulate
  the project hierarchy with non-semantic clickable `div` elements.
- Keep source editing, run actions, diagnostic navigation, pane switching,
  graph inspection, resize controls, and context actions keyboard reachable.
- Announce run start/completion/cancellation and diagnostic-count changes without
  flooding live regions with every parser event.
- Provide text/icon/state alternatives for graph and diagnostic colors.
- Respect reduced motion, forced colors, zoom, text reflow, and CEM coupling
  safety zones.
- Keep a native textarea fallback if a richer editor cannot meet the required
  accessibility or browser capability.

## Performance and Reliability

- Split the app shell from the engine/schema-package payload and load packages
  on demand where practical.
- Preload the minimum Feature Tour capability set; disclose downloads before
  loading unusually large packages.
- Debounce validation while typing, validate immediately on explicit run/save,
  and cancel superseded requests.
- Cache by engine/package version, normalized run config, resource hashes,
  policy stamp, and output identity. Never reuse a result across a mismatched
  resolver or schema identity.
- Bound source, graph, trace, preview, report, and retained-result sizes and
  explain limit diagnostics.
- Stream/lazily page large results and release buffers when tabs or projects
  close.
- Recover from a crashed worker without losing edits; only the current run may
  be lost.
- Surface offline, quota, migration, worker, package-load, CORS, and provider
  failures as distinct actionable states.

## Verification Strategy

1. **Rust contract tests:** command requests, normalized plans, resolver policy,
   reports, diagnostics, source maps, cancellation, and limits remain native
   tests first.
2. **WASM parity tests:** run portable cases in native and WASM and compare
   normalized structured results.
3. **Browser package tests:** worker lifecycle, transfers, progress ordering,
   cancellation, URL resolver, capability negotiation, command
   parse/serialize/normalize round trips, and native-path versus Studio-URI
   projections.
4. **Component tests:** the pinned Angular Material parity matrix verifies that
   every equivalent Studio need is supplied by a completed general
   `<cem-element>`-based CEM component. Studio-only components cover
   accessibility, keyboard, theme modes, loading/error/empty/stale/conflict
   states, and light-DOM rules, and record the absence of an Angular counterpart.
5. **Persistence tests:** IndexedDB schema creation, every migration, atomic
   autosave, quota failures, multi-tab revision conflicts, trash/restore, and
   export/import round trips. Cover Apply to current/existing/new page,
   duplicate names, incompatible command kinds, unresolved resources, atomic
   failure, stale results, and undo.
6. **PWA browser tests:** install metadata, offline reload, service-worker
   update, cached WASM/sample execution, and no project loss across update.
7. **Security tests:** active HTML, unsafe URLs, redirects, oversized/decompression
   inputs, malicious bundles, CSP, preview sandbox, resolver limits, and secret
   redaction.
8. **Feature Tour drift tests:** generate the tree from the actual bundled
   capability manifest and execute every advertised example.
9. **Npm distribution tests:** pack/install `@epa-wg/cem-ml`,
   `@epa-wg/cem-ml-cli`, and `@epa-wg/cem-studio` separately into clean
   temporary consumers. Verify the runtime exports, CLI browser/Node exports,
   npm `cem-ml` executable, Studio bootstrap/static artifact, package contents,
   exact internal dependency versions, and one resolved engine runtime on the
   supported Node/browser/OS matrix.
10. **Native target distribution tests:** build and test the Linux AMD64,
    Homebrew ARM64, and Windows AMD64 deployment subprojects independently;
    verify target identity, archive/package contents, checksums/signatures,
    machine version/capability output, and install/upgrade/uninstall on that
    exact OS/architecture/ABI. Verify the release-scoped Linux binary and its
    integrity/provenance companions appear on the matching tagged GitHub
    Release; macOS and Windows remain local validation targets.
11. **Release-family tests:** before promotion, verify that every npm package,
    native target artifact, package-manager projection, capability manifest,
    service-worker build id, checksum, SBOM, and provenance record contains the
    exact version from common `cem_ml` and points to the same release commit.

All build, test, lint, browser, and PWA verification should be exposed as Nx
targets on the eventual projects.

## Dependency-Ordered Delivery Plan

This ordering is a design proposal. It does not add work to the active roadmap
or todo list by itself.

1. **Portable project contract.** Define the logical project model, stable ids,
   CEM-ML schema, JSON projection, logical URI rules, bundle layout, migration,
   and import/export validation.
2. **Library command boundary.** Move typed CLI-independent parse, validate,
   inspect, convert, query, transform, trace, report, capability, cancellation,
   and resolver requests into `cem_ml` with native contract tests.
3. **Common version and release-family contract.** Keep the authoritative
   version in common `cem_ml`; define the fixed `cem-ml-platform` release graph,
   version synchronizer, exact internal dependency policy, staged promotion,
   release index, and cross-package/version verification gates.
4. **WASM runtime npm deployment.** Create the separate `@epa-wg/cem-ml`
   subproject with its `/wasm` runtime, generated types, schema-package assets,
   ABI/capability manifest, integrity records, and native/WASM parity fixtures.
   It has no CLI launcher or npm `bin`.
5. **Universal CLI npm deployment.** Create the separate
   `@epa-wg/cem-ml-cli` subproject with an exact same-version dependency on
   `@epa-wg/cem-ml`, `/browser` and `/node` exports, shared command parsing,
   generated command types, and the npm `cem-ml` executable.
6. **Native target deployment projects.** Establish exactly three initial Nx
   subprojects: Linux AMD64, macOS ARM64 through Homebrew, and Windows AMD64.
   Produce independently signed/checksummed artifacts and platform package
   metadata. Preserve Linux plus its integrity/provenance companions on the
   tagged GitHub Release and use thin APT aggregation; keep macOS/Homebrew and
   Windows/WinGet projections local until their distribution scope is promoted.
7. **Worker engine client.** Implement initialization, request/progress/result,
   transfers, cancellation, stale-result rejection, lazy payloads, restart, and
   JS resolver adapters in `@epa-wg/cem-ml-cli/browser`.
8. **Angular parity prerequisites and themed Studio frame.** Classify every
   required control against the pinned Angular Material catalog. Land and verify
   the general `<cem-element>`-based CEM component first wherever a counterpart
   exists; only then compose it in Studio. Add `/studio` components for
   workbench behavior with no Angular counterpart, using `@epa-wg/cem-theme`
   exclusively.
9. **Studio npm/PWA deployment.** Create the separate publishable
   `@epa-wg/cem-studio` Nx subproject with an exact same-version dependency on
   `@epa-wg/cem-ml-cli` plus tested-compatible component/theme dependencies.
   Add the responsive theme scope/mode selector, manifest, static deployment
   artifact, service worker, offline shell, update flow, and worker loading.
10. **Local project store.** Add IndexedDB repositories/migrations, autosave,
    trash/restore, multi-tab revision checks, storage persistence/health, and
    validated import/export.
11. **Generated Feature Tour.** Build the editable seed from bundled
    schema-package examples and capability manifests; add drift and execution
    tests.
12. **Parse/validate/inspect MVP.** Deliver inline/URL resources, data sets,
    source identities, diagnostics/range navigation, AST/DOM/event/report/source
    map previews, offline behavior, and the CLI Command view with copy plus
    current/existing/new-page round trip.
13. **Conversion workbench.** Add input/output identity selectors, side-by-side
    previews, losses, source trace, copy/download, and pinned expected results.
14. **Query and transformation workbench.** Add explicit query-language
    selection, variables, templates/configs, outputs, diffs, trace, and relevant
    Feature Tour scenarios.
15. **Transformation graph.** Add graph editing/validation, stage execution
    overlay, edge/scoped-data preview, source navigation, collections, and
    exports.
16. **Hardening and richer editing.** Profile bundle/memory costs, add large-data
    paging, richer editor adapter if justified, accessibility/contrast coverage,
    CSP and sandbox tests, migration recovery, supported Node/browser/OS matrix,
    npm installation/packing checks, and browser support disclosure.
17. **Semi-native executable experiment.** After the Node/WASM CLI is stable,
    prototype target-specific self-contained executables with Node SEA for the
    same Linux AMD64, macOS/Homebrew ARM64, and Windows AMD64 matrix. Compare
    deprecated `pkg` only for migration knowledge; gate adoption on size,
    performance, asset loading, signing, security-update, and installation
    results.
18. **Account-storage providers.** Start with one validated need—likely GitHub
    Gist for small sharing or Git repositories for project-as-code—then add S3
    bundles or a NoSQL account service behind the same revisioned provider
    contract.

## Decisions to Preserve

- One CEM-ML semantic engine; native CLI and browser Studio are adapters.
- Common `cem_ml` is the sole version authority for the complete fixed
  `cem-ml-platform` release family.
- `@epa-wg/cem-ml`, `@epa-wg/cem-ml-cli`, and `@epa-wg/cem-studio` are separate
  public npm deployment subprojects, but they always publish the exact common
  CEM-ML version from the same release commit.
- `@epa-wg/cem-ml` owns only the low-level WASM runtime; it has no npm CLI
  launcher or `bin`.
- `@epa-wg/cem-ml-cli` owns the browser and Node CLI adapters plus the npm
  `cem-ml` executable, and depends on the exact same-version
  `@epa-wg/cem-ml` package.
- `@epa-wg/cem-studio` depends on the exact same-version
  `@epa-wg/cem-ml-cli`; it receives the runtime transitively and must not resolve
  another copy independently.
- Common `cem_ml_cli` owns native CLI source but is not a deployment package.
  Linux AMD64, Homebrew ARM64, and Windows AMD64 each have their own Nx
  subproject and deployment package, all stamped with the exact common version.
- The initial CLI support matrix contains only WASM for Node, Linux AMD64,
  Homebrew ARM64, and Windows AMD64.
- The Linux native binary is preserved as a versioned asset on the tagged
  GitHub Release with its checksum, signature, SBOM, provenance, and target
  identity. macOS and Windows release publication is deferred, and their
  package-manager projections remain local validation output.
- Target-specific executables that bundle Node plus WASM through Node SEA are a
  wishlist item and remain `wasm-node`-derived rather than native Rust. Archived,
  deprecated `pkg` is only a comparison or migration reference.
- UI-only, npm-CLI-only, or target-packaging publication still advances the
  common version and rebuilds the complete release family; members have no
  independent release cadence.
- Every version/capability response identifies its precise WASM, semi-native, or
  native runtime and target; no distribution silently falls back to another
  runtime.
- Browser commands are typed operations, not an arbitrary shell.
- The CLI Command view and structured Workbench forms are lossless projections
  of the same normalized run plan.
- Editing a command does not mutate the tree; Apply targets a stable existing
  page or explicitly creates a named page transactionally.
- The browser exposes capability differences instead of hiding them.
- Projects are local-first and useful without an account or network.
- IndexedDB owns mutable local projects; `localStorage` owns only small
  preferences.
- Studio exposes IndexedDB through a logical repository port; CEM-ML templates
  never address physical databases, stores, indexes, or transaction handles.
- Repository queries may enter CEM data slices declaratively, but project
  mutation and persistence requests are explicit application commands and never
  render side effects.
- Deterministic lexical/trigram search is the complete baseline. Browser AI is
  optional evidence-bound assistance, never retrieval or project authority, and
  every visible search/AI surface is composed from CEM components.
- Built-in examples are versioned seeds; updates never overwrite user copies.
- Content/schema/query/transform identities are explicit and never guessed.
- Logical URIs and stable ids survive tree-label changes.
- `@epa-wg/cem-components` owns reusable functional UI;
  `@epa-wg/cem-components/studio` owns Studio-specific reusable composites; the
  PWA owns non-UI orchestration. Visible UI, interaction bindings, and state
  projection remain XHTML/CEM-ML; app-local JavaScript DOM behavior is
  forbidden by [`declarative-ui-principle.md`](./declarative-ui-principle.md).
- An Angular-Material-equivalent control is implemented and verified first as a
  general `<cem-element>`-based CEM component. Studio-first components are
  allowed only when the pinned parity matrix records no Angular counterpart.
- `@epa-wg/cem-theme` is the sole visual semantic/token system, applied once at
  the light-DOM app shell.
- Preview output is untrusted, sandboxed, bounded, and non-scriptable by default.
- URL access is explicit, policy-controlled, CORS-aware, and provenance-rich.
- `project.cem` plus native resource files is the preferred portable/Git form;
  JSON is the normalized browser/API/NoSQL projection; Markdown is generated
  presentation; YAML is optional interchange.
- Import/export precedes remote sync so users always have a provider-independent
  escape hatch.
- S3, NoSQL, Git, and Gist persistence are optional providers, not alternate
  project semantics.

## Open Contract Questions

Phase 2.5 resolved the runtime/CLI deployment roots and identities, fixed Nx
release group and Cargo-derived version synchronization, Node/host matrix,
native signing and package channels, first-release capability matrix, and
versioned host protocol. Those decisions are owned by the canonical
[`cem-ml-deployment-contract.md`](./cem-ml-deployment-contract.md), not by this
Studio proposal.

- What executable-size, startup, performance, signing, asset-loading, and
  security-update thresholds must a Node SEA semi-native package meet before it
  leaves the wishlist?
- Which first permanent provider has validated user demand and a sustainable
  authentication/service model?

## Research References

- [web.dev: Learn PWA](https://web.dev/learn/pwa/welcome)
- [web.dev: PWA installation](https://web.dev/learn/pwa/installation)
- [web.dev: Service workers](https://web.dev/learn/pwa/service-workers)
- [MDN: IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)
- [W3C: Indexed Database API 3.0](https://www.w3.org/TR/IndexedDB/)
- [MDN: Web Storage API](https://developer.mozilla.org/en-US/docs/Web/API/Web_Storage_API)
- [MDN: Storage quotas and eviction](https://developer.mozilla.org/en-US/docs/Web/API/Storage_API/Storage_quotas_and_eviction_criteria)
- [MDN: Origin Private File System](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API/Origin_private_file_system)
- [MDN: Web Workers](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers)
- [MDN: Clipboard API](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard_API)
- [MDN: Fetch and CORS](https://developer.mozilla.org/en-US/docs/Web/API/Fetch_API/Using_Fetch#making_cross-origin_requests)
- [MDN: iframe `srcdoc` security](https://developer.mozilla.org/en-US/docs/Web/API/HTMLIFrameElement/srcdoc)
- [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy)
- [Chrome: Built-in AI API status](https://developer.chrome.com/docs/ai/built-in-apis)
- [Chrome: Prompt API](https://developer.chrome.com/docs/ai/prompt-api)
- [Chrome: Summarizer API](https://developer.chrome.com/docs/ai/summarizer-api)
- [Microsoft Edge: Prompt API](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/prompt-api)
- [npm: `package.json` `bin` field](https://docs.npmjs.com/cli/v11/configuring-npm/package-json#bin)
- [Node.js: Single executable applications](https://nodejs.org/api/single-executable-applications.html)
- [Vercel: archived and deprecated `pkg`](https://github.com/vercel/pkg)
- [GitHub: About releases and binary assets](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [Angular Material: Component catalog](https://material.angular.dev/components/categories)
- [Homebrew: Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Debian: Package management](https://www.debian.org/doc/manuals/debian-reference/ch02)
- [Rust: Installing binaries with `cargo install`](https://doc.rust-lang.org/book/ch14-04-installing-binaries.html)
- [GitHub: REST API for Gists](https://docs.github.com/en/rest/gists/gists)
- [GitHub: REST API for repository contents](https://docs.github.com/en/rest/repos/contents)
- [AWS: Uploading S3 objects with presigned URLs](https://docs.aws.amazon.com/AmazonS3/latest/userguide/PresignedUrlUploadObject.html)

### Home, creation and archive import

Home is a persistent shared-control card above the retained workbench. It lists
live projects, opens them through session admission, and exposes explicit refresh,
creation and archive import. Creating or importing persists a project without
leaving the active draft; Open project requests the guarded transition separately.
The workbench selector then exposes that project's pages and resources.

Creation copies the integrity-checked bundled CEM-ML source into a fresh UUID
project with a user-supplied name. It carries one source resource and no implicit
command pages. The repository validates the normalized project through the native
boundary before its atomic create transaction. Import uses the existing archive
codec and the same create-only transaction: an existing identity is never replaced.
Boot continues to preserve existing Feature Tour copies.

Home reports empty, storage, operation-success and failure states. Picker cancellation
is silent and leaves drafts unchanged; file-read failures are reported. Creation
and import are serialized by the service. Failed or cancelled navigation does not
remove a successfully created/imported project. Recovery of the last admitted selection and active-provider coordination follow
the contract below.

### Explorer, search and retained panes

Studio composes the canonical `cem-tree` with project, nested page and resource
records. Stable URI-encoded record identities contain no whitespace or display
labels. Parent activation only expands the hierarchy; an explicit child opens a
branch's own record. Focus movement never selects or navigates. Selected state
follows committed session admission, so Save/Apply failures and Cancel retain the
previous selection and draft. Missing destinations report a navigation error.

Search submits at most 500 characters to the existing repository index and shows
up to 50 results in its deterministic score/key order. Result activation resolves
project/page/resource identity through the same guarded session. Empty, pending
and failed search states are declarative; newer searches supersede older results.
Successful saves and command Apply refresh navigation and any active search.

Navigation remains available above the retained Source and command / Results
panes at compact and expanded widths. The shared host-view service can now bind
to already-authored view elements, preserving declaration ownership and editor
identity/caret through tab reconciliation. Validated tab values cross the shared
host bridge; diagnostic and provenance commands reveal Source before applying
its range/focus request. No application event handlers, DOM
rendering, local CSS or new theme tokens implement these interactions.

The 128-resource shared tree fixture exposed repeated native binary expansion in
render-plan hashing. Binary-bearing addresses now use the versioned shared hash
contract documented in `cem-elements`; native typed artifacts remain intact.
Resizable simultaneous panes and broader device/AT acceptance remain separate work.


### Recovery and active-project providers

The session stores only its last admitted project/page/resource identity in local
browser storage (`cem-studio-selection-v1`). Admission, rather than a highlighted
row or pending transition, commits this preference. Reload reopens the exact
IndexedDB bytes through the ordinary project resolver. Missing/trashed destinations
fall back to the bundled starting page with an explanation; corrupt or unavailable
selection storage does not prevent opening projects. No draft or local preference
is embedded in exported project data. Unsaved text uses the separate local recovery journal described below.

Repository change notifications, including other tabs, refresh Home, explorer and
active search. They never overwrite a source or command draft. Changed revisions
and missing selections are reported; Save/Apply retain their optimistic revision
checks. Home moves a selected project to recoverable Trash and restores it using
its current revision. Trashing the active project keeps its drafts available;
resource writes are rejected until restore. Export remains an exact persisted
backup and does not silently save drafts.

The file provider follows the admitted project/resource. Each explicit operation
captures that target before invoking the browser API. Late status/picker results
cannot replace a newer target. Imported projects appear in Home; opening them uses
guarded navigation. Permissions are requested only by explicit actions, and
cancellation/denial leave archive import/export available.

Studio update activation waits for navigation, provider work and running/saving
operations. Unresolved source or command drafts block activation with instructions
to Save/Apply or reset explicitly. Update does not implicitly apply a command or
save a draft. The reusable update coordinator also detects new dirty state during
asynchronous persistence. Durable save status and validation results remain distinct.


### Deployed workspace acceptance

`@epa-wg/cem-studio:verify:workspace-delivery` runs the graph-emitted application
in Chromium and participates in Studio `check` and package verification. It uses
production CEM controls, a native file chooser and a browser download to create
one project and import a second, save source, create native command pages, search
and run them, cancel/discard guarded navigation, export exact UTF-8 bytes, and
Trash/Restore. The second project's save/Apply/run, export and reload run offline;
the first project's saved source remains unchanged.

Acceptance inspects exposed Chromium accessibility names and DOM ID references,
modal background inertness, Escape return focus, retained editor identity/caret,
visible keyboard focus, and page reflow at 320, 390 and 1280 CSS pixels. Native
Tab navigation may visit browser chrome while a modal is open; it must never
focus the inert page behind the modal. Existing delivery checks retain all five
themes, inherited overrides, forced colors and reduced motion.

The narrow viewport audit identified a shared select grid minimum-size defect.
`cem-select` now constrains its flexible grid column and control minimum inline
size so long selected labels use their existing ellipsis. The full option text
and accessible name remain intact. No Studio CSS or theme-token overrides are
needed. The shared `NarrowContainer` story protects option access and focus return.

`workspace-delivery.json` records the browser version, platform, lifecycle and
accessibility evidence. Release evidence references this report and its scope.
These are automated Chromium checks, not a WCAG conformance certification or
spoken screen-reader/physical-device acceptance. The independent device/AT matrix
in TODO remains open. Structural editing is implemented in the next section.


### Local draft recovery

The admitted workbench synchronously journals changed source and command text in
browser local storage, separately from IndexedDB project records. Keys use the
`cem-studio-selection-v1:draft-v1:` prefix and resolved project, page and resource
identities. This is an explicit versioned JSON storage/backup boundary, never an
AST representation or part of project export. No timer or unload callback is
required for durability after an editor update reaches the session. Each serialized
record is bounded to 1,000,000 UTF-16 code units, including its baseline text.
Browser quota and retention still apply; project exports do not include drafts.

Recovery requires exact project/resource revisions, source baseline and command
baseline matches. It restores text and the pending command target through the
ordinary workbench draft APIs, including native command validation. It never
saves source, applies commands, runs operations or restores replacement consent.
Command targets without a changed command text are not independently journaled.
Save/Apply and explicit reload/reset retire the corresponding clean drafts.
Navigation discard removes recovery only when the replacement workbench is admitted;
cancellation or a failed open retains it. Missing/trashed projects retain their
journals while selection recovery falls back to another project.

A changed baseline or corrupt record blocks automatic recovery. The shared CEM
alert explains the condition, and shared actions download a recovery backup or
explicitly clear the stored record while keeping current editor drafts. Backups
for blocked records include stored data and current text with baseline metadata;
there is no automatic merge or backup-import workflow in this slice. Storage
failure retains a downloadable in-memory snapshot of the latest editor text and
never advertises it as saved. Clearing resumes journaling of any current drafts.
Changed stored values are detected before writes/clears to avoid overwriting
observed changes from other tabs; localStorage is not a cross-tab transactional
store and simultaneous writes are outside this bounded guarantee.

The UI adds no styles, tokens, listeners or widgets: declarative CEM alerts/actions
retain native names, focus, inherited theme tokens and forced-color behavior.
The deployed two-project gate verifies an offline reload and a real recovery
backup download while persisted source bytes remain unchanged.


### Structural project editing and history

Studio exposes project and page display-name changes, subproject creation, subtree
copying, moves between subprojects/root, up/down sibling reordering, and CEM-ML
resource creation or byte-exact resource copies. These forms use existing shared
CEM cards, fields, selects, textareas, alerts and actions. Hidden native inputs carry
form command metadata only. No new component, stylesheet, theme token or app DOM
behavior is needed. The shared runtime reads form identity through attributes,
so a legitimate field named `id` cannot clobber form capture.

The existing native project model already preserves entry-array order. IndexedDB
now retains that order as private project metadata and reconstructs the ordered
portable array on export; it does not add a manifest field or schema version.
Old databases without order metadata retain their existing lexical order until
edited. Applying a command appends new pages without reordering existing pages.
Subtree copies allocate new stable page identities but keep resource references;
existing Apply copy-on-write isolates shared command resources when edited.
Resource copies allocate a new stable identity and path, preserving bytes and
content type, and become local project files. New resources begin as CEM-ML data.
Resource path rename/deletion and automatic authored-reference rebinding are not
part of these operations; display-name changes never rewrite command text.

`edit-project` reads a complete bundle, applies one bounded structural operation,
validates it through the native project import boundary and commits all records,
blobs, search documents and change notifications in one transaction. Expected
project revision is checked both before preparation and inside the transaction;
missing or trashed projects cannot be recreated or restored by a structural edit.
A stale revision, cyclic move, duplicate/escaping resource path or native validation
failure leaves the previous revision intact. Resource hashes remain byte-derived.
The replace path uses value cursors when deleting records by index.

Undo/redo holds at most 20 snapshots and 8 MiB of retained metadata/source bytes
per project in the current workbench adapter session. Snapshots never enter a
project export or local draft journal. Replay is native-validated and creates a
new monotonically increasing project revision; restored resource revisions also
advance. New edits clear redo; Save/Apply, another tab or another provider revision
invalidates history rather than overwriting that work. Reload starts fresh history.

Structural actions require clean source and command drafts and no pending
navigation/execution. After a commit the active identity reopens through normal
guarded session admission, or returns to its project if undo removed that identity.
Typing while the commit is pending triggers the existing draft guard, preserving
the new text. Explorer, project choices, search and active providers follow ordinary
repository notifications. Service-worker activation counts structural writes as
active work. The deployed two-project acceptance gate records `structuralEditing`
for shared-control rename/create/reorder, undo/redo, native invalid-path rejection
and offline reload persistence; physical-device/spoken AT coverage stays separate.


### Typed operation forms

The Command pane has an Operation form section for parse, inspect, convert, query,
transform and trace. Studio selects operation-specific options; the generated
shared CLI command schema supplies their labels, help, defaults, required flags,
boolean actions and enum choices. No grammar or enum vocabulary is copied into
Studio. Shared CEM selects expose enumerations and boolean values; text fields
and textareas retain scalar text, including multiline query/template expressions.
An option chooser exposes one active editor at a time, bounding the number of
rendered controls and keeping background projection work within the shared
scheduler budget. Switching options discards unsubmitted field input; Set commits
it to the command draft. Existing shared tabs, alerts and actions own layout,
keyboard behavior and styling.

Set edits exactly one option in the current literal command, then the shared
parser and serializer validate and canonicalize its draft. Reset removes that
option so native defaults apply, and required options cannot be reset. Positional
inputs, global options, repeated options and all other option values are retained.
Conflicting or required option groups are rejected rather than silently removing
another option; edit coupled groups together in the literal editor. Hidden,
unsupported, positional and infrastructure options are not exposed as fields.
Invalid literal commands stay repairable in the editor. Forms edit an already
prepared operation; they do not select a new operation or generate a new page.

Each form submission carries the workbench instance and draft revision it was
rendered from. A stale submission, including one from a different workbench, is
rejected with a CEM alert and leaves command bytes unchanged. Successful edits use
the existing preview, semantic-change, draft-recovery and guarded-navigation
pipeline. Preview resolves the same retained resources and applies the same
browser capability checks as literal editing. Setting a field never saves source,
applies a command or runs it. Existing Apply support remains parse/inspect;
other portable workbenches keep their existing execution and persistence limits.

The deployed lifecycle gate edits and resets format, sets source-offset retention,
checks unchanged project revision before Apply, and runs the explicit saved
command afterward. It audits accessible names, relationships, keyboard focus and
320/390/1280px reflow while the typed form is visible. The result is required in
release evidence as `typedOperationForms`; physical-device and spoken AT coverage
remain separately open.


### Resource path editing and dependency rebinding

The shared Project structure resource form supports rename and delete. Rename
preserves stable resource IDs and page references, advances the project/resource
revisions and rebinds every supported authored command input in the same transaction.
The shared CLI parser/serializer owns command decoding and quoting. Unchanged data
bytes remain exact; matching command and run-config records are canonicalized and
rehashed, while graph edits preserve bytes outside native input value ranges.
Deleting a resource requires no page or other command to reference it. Deletion
never silently removes pages, detaches dependencies or cascades to other resources.

The admitted contract remains conservative: projects may contain data sets,
subprojects, inspection and bounded opaque transformation-graph pages, with local
data/expected assets, native CLI parse/inspect or transform-with-config command
resources, explicit JSON run-configs and native transform-config graphs. Every command, including orphan command resources, must have an exact
absolute project resource URI in its input/config slot. Unsupported page kinds, configuration
formats, executable resource roles, resolver/configuration/output options,
relative/unresolved inputs and project URI values
in non-input options block the entire path edit. Data and expected payloads are
opaque leaf documents in these operations: embedded text is not searched or rewritten.
This is not an automatic refactoring of hyperlinks, schema imports or arbitrary
source text. Template/query/validation dependency rebinding and graphs with transitive loads
need additional native contracts before those projects can use path edits.

Only local project files may be renamed/deleted. Native whole-project validation
rejects invalid paths; collisions and stale revisions fail before commit. The
existing draft/navigation guard, active-provider refresh and bounded session undo/redo
apply. Undo restores prior paths and exact authored command bytes with new revisions.
The deployed gate verifies rename of an active input, blocked referenced deletion,
unreferenced deletion/undo, successful execution through the rebound command and
offline reload. Release evidence requires `resourcePathEditing`.

The first native dependency-planning layer is
`cem_ml::run_config_rebind::plan_run_config_rebind`. It accepts the existing
`RunConfigParseRequest` plus exact old/new resource URLs and returns typed reference
roles, JSON-pointer blockers, and optional replacement bytes. This is a native
library contract, exposed through the worker and used by Studio within the bounded
admission rules below.

Its admitted slots are `inputs[].uri`, `schemaPackages[].uri` and
`outputs[].inputRef`. The last is a link to a declared input and changes with that
input. Native validation runs before and after rewriting, including duplicate URI
and dangling input-link checks. Schema/namespace identifiers, policy, budgets and
other scalar metadata retain their values. The JSON representation is the explicit
run-configuration control format, not a document AST or a rendering handoff.
Changed configurations preserve authored field presence and canonicalize whitespace;
no-op configurations return no replacement so hosts retain the exact original bytes.

Planning fails closed for unknown fields and non-object records, relative or
noncanonical URLs, query/fragment/glob references, scope base overrides, resolver
bindings, module maps, custom formatter/colorizer settings and output destinations.
A blocker anywhere suppresses all replacement bytes, including otherwise valid
matches. Blocked reference inventories are incomplete for dependency admission and
must never authorize deletion. Planning does not fetch, execute, save or establish
that an entire project is safe to rename.

Relative resolution must be aligned with execution before extending this boundary.
Run configuration input URIs are retained by native normalization, while module-map
and output paths have file-base normalization; graph references have a separate
config-relative resolver. Studio sample run-config inputs also use project-root
paths despite living in a configuration subdirectory. A universal config-relative
URL join would therefore be incorrect. The next contract must carry execution's
actual acquisition base, scope overrides and resolver/map provenance, distinguish
read references from output destinations and node/format identities, and discover
transitive dependencies without executing templates. Host adoption then needs
revision-checked atomic replacement, collision checks, native whole-project validation,
exact-byte undo and deployed run-after-rebind coverage.

Graph discovery now has its own native entry point,
`cem_ml::transform_graph_dependencies::discover_transform_graph_dependencies`.
It parses the original CEM-ML graph and returns the acquired graph URI, diagnostics
and immediate references identified by node ID and native attribute name. It uses
`resolve_transform_graph_reference`, the same resolver as
`ManifestTransformGraphResourceProvider`, so `../data/input.xml` in
`studio://project/graphs/run.cem` resolves to `studio://project/data/input.xml`.
A native fixture checks this against actual manifest-provider acquisition and output
resolution. This contract targets virtual command-service execution; it does not
substitute URL joining for the CLI filesystem/resolver provider.

Reference roles distinguish import inputs, transformation templates, source/target
module maps and export destinations. Each reference retains its authored spelling.
Node links, schemas and parameter values are not treated as resource attributes.
Ordinary imported documents require document dependency inspection; templates and
module maps require their respective transitive scans. Explicit opaque imports are
leaves. Output destinations require ownership/collision checks. Globs and captured
binding patterns have no single resolved target and require resource-set expansion.
Graph errors return diagnostics without a partial inventory; callers must inspect
all diagnostics before relying on results.

This inventory performs no fetching, execution or rewriting and does not establish
a complete dependency closure. A graph edit still needs source-preserving native
rewrite locations, transitive scans, resolver provenance, output conflict checks
and host revision validation. The run-config and graph contracts remain separate:
the graph acquisition base does not establish a base for relative run-config inputs.

The bounded native source rewrite is
`cem_ml::transform_graph_rebind::plan_transform_graph_rebind`. It admits unqualified
native graph declarations with explicitly opaque inputs, exact non-pattern reads
and non-conflicting output destinations. It returns the original dependency
inventory, blockers, and optional candidate bytes. Ordinary documents, templates,
module maps and patterns block the entire rewrite; follow-ups are not silently
cleared. Unknown or duplicate attributes and graph diagnostics also block edits.
This is not a complete transitive dependency implementation.

Matches use the executor-resolved input URI. A matching relative input becomes an
absolute URI; the graph acquisition URI stays fixed. Moving/replacing the graph
itself needs a separate base-rebinding contract and is rejected. Output references
to either rename endpoint block the edit. Other output ownership and project-wide
collision checks remain the host's responsibility.

The rewrite binds AST attribute ownership to tokenizer-provided value byte ranges.
It preserves quoted values' quote style, quotes rewritten bare values, and preserves
all bytes outside those ranges, including comments, Unicode, whitespace and other
attributes. Multiple edits apply in reverse byte order. No-op and blocked plans
supply no replacement bytes. Native reparsing must produce exactly the original
typed graph with only the selected input values replaced; otherwise no candidate is
returned. A fixture also verifies acquisition through the manifest provider using
only the new resource URI. The worker and Studio adopt this bounded planner as
described below; transitive scans remain separate follow-up work.

Source module-map discovery is now available through
`cem_ml::transform_graph_request::discover_source_module_map_dependencies`.
Discovery and graph execution share the same module-map parser. Given acquired map
bytes and their canonical URI, discovery accepts explicitly declared v1/v2/v3 maps
and returns import/resource identities, authored paths, resolved source URIs,
validated content types and v3 `moduleImports` edges. Paths resolve relative to the
acquired source map using the manifest provider's resolver. Destination maps are
outside this API because their paths belong to publication, not source acquisition.

Declared edges identify both the target asset and its resolved URI. Cycles remain
explicit graph edges without recursive fetching. Missing/ambiguous targets and
non-module edge targets fail; unknown top-level fields such as import-map scopes
also fail. Identity keys and module specifiers retain their semantic roles instead
of being interpreted as file paths. The stricter discovery admission does not change
the existing execution parser's format compatibility.

This discovers declared map edges, not verified content dependencies. It does not
acquire asset bytes, validate JavaScript declarations against source imports, inspect
relative/dynamic imports, inspect document/template dependencies or authorize edits.
An empty edge set never proves an asset is a leaf. Those source checks and project
snapshot/revision validation must precede Studio admission for transitive rewrites.

V3 source maps can now be checked against supplied immutable asset snapshots with
`verify_source_module_map_declared_edges`. The native verifier reuses execution's
bare-module checks to reject undeclared, unused, escaped or invalid declared edges.
Every declared asset must be present; supplied content-type metadata must agree with
the map. Failure returns no partial evidence. Successful evidence contains the map
URI/hash, each asset's exact-byte hash and byte length, and its declared dependency
inventory. Candidate rewrite bytes are discarded; input maps and assets remain exact.

Known unresolved source mechanisms are reported per asset: non-bare imports,
dynamic import calls, template literals, and non-JavaScript contents. These checks
share the executor's lexical scanner; they do not constitute JavaScript syntax
validation or comprehensive runtime-load analysis. An empty follow-up list does not
prove dependency closure or authorize rename/delete. Document/template inspection,
runtime loader policy, host revision checks and comparison of evidence hashes with
the current snapshot remain required before broader Studio admission.

Native CEM template imports can now be inspected through
`cem_ml::real::inspect_native_template_dependencies`. The API reuses runtime module
parsing and recursive preflight, including alias/depth/cycle checks, resolver grants,
module-map policy, requested/normalized/substituted/acquired URI provenance and
content hashes. A caller-supplied import-depth cap can tighten the declared/default
limit. Reads use the supplied `EngineContext`; snapshot-only hosts must provide an
immutable snapshot resolver. The API does not install another resolver or execute
bodies, XPath invocations, functions, renderers or publication.

Results retain root identity/hash, diagnostics and successful native preflight.
Import failures return no partial preflight. Expression-valued import declarations
are rejected by native declaration parsing rather than evaluated. Every discovered
template, including the root and modules with no imports, remains listed in
`bodyAnalysisRequired`: data-dependent document loads and other body/runtime behavior
are outside module-import discovery. This is not a full document/template dependency
closure, and its inspection-purpose cache key is not execution or mutation approval.
Host snapshot/revision checks and body/document analysis remain required for Studio.

Template inspection now attaches `xpathResourceUses` for native parsed XPath slots
in the root and successfully acquired modules. Findings identify the template,
invocation and native function-name source range. The inspector traverses the typed
XPath syntax, including conditional branches, predicates, maps, arrays and nested
calls, without evaluating them. Function namespace identity distinguishes standard
resource functions from extension functions. Direct string arguments are retained
as literal URI candidates; computed arguments remain unresolved. Literal candidates
also retain a resolution blocker because base URI and resolver policy have not been
applied. No candidate document is acquired by expression inspection.

Dynamic function calls, inline functions, extension functions, unsupported syntax,
invalid syntax and exhausted traversal budgets produce explicit blockers. Traversal
is iterative with a 10,000-node budget per template XPath slot. Findings cover those
parsed slots only: other CEM expressions, generated document references and runtime
loads remain outside this contract. `bodyAnalysisRequired` is never cleared merely
because no XPath load was found. General document dependency inspection and Studio
admission remain open.


### Bounded native planning in Studio edits

`BrowserCommandServiceClient.planResourceRebind` carries supplied configuration
bytes, explicit identity, acquired base and rename endpoints to the existing worker.
The native `planResourceRebind` WASM export dispatches to the run-config or graph
planner and returns reference roles, blockers and optional candidate bytes. It
performs no acquisition, execution or mutation. Invalid control requests return the
shared worker action error envelope. No worker-local project or cached permission is
introduced.

The repository plans every declared configuration, including orphan records, against
one copied bundle before changing any stored bytes. Every non-output reference must
resolve to a local data/expected resource. Schema packages and transitive resource
roles remain blocked. Graph outputs must stay inside the project, be distinct across
all graphs, and avoid existing resources and both rename endpoints. Configuration
relocation uses the explicit native mode described below; configuration deletion
remains blocked. Ordinary command
resources continue to use the shared CLI parser, with transform admitted only through
an exact local transform-config reference and no additional transform options.

A successful edit updates matching configurations, content hashes and resource
revisions together with the path and project revision. Native project validation and
the existing transaction revision check still run after planning; a concurrent edit
invalidates the candidate. Deletion checks the native reference inventories as well
as page and command ownership. Undo restores exact prior config/data/command bytes
and paths while advancing revisions. Planner errors, blockers and cancellation before
commit leave storage untouched. The deployed `nativeRebinding` evidence covers an
opaque graph execution using the renamed input and exact exported bytes, run-config
rebinding, revision and
deletion guards, and exact-byte undo.

CEM review: the existing shared resource form and semantic feedback remain the UI;
JavaScript additions are repository and worker transport services. No component
behavior, CSS, theme defaults or public token ownership changed. Broader template,
document, module-map and relative-base admission remains explicitly unsupported.


Graph command publication preserves retained `primary_bytes` when present, matching
the existing direct-operation publication path. Previously, opaque graph exports
published their descriptive resource record as JSON and lost the asset payload. A
native non-UTF-8 regression and a real-worker artifact read cover the retained-byte
path; Studio's deployed fixture also checks the exact copied SVG content.


### Relative configuration bases and safe relocation

Configuration acquisition, initial input lookup and later resolver reads have
distinct bases. Native execution is authoritative; a host must not apply one
generic URL join to every configuration string.

| Slot | Current native execution rule |
| --- | --- |
| Run-config input `uri` and schema-package `uri` | Retained as declared lookup keys during normalization; initial command input acquisition uses the exact snapshot key. |
| Run-config output `inputRef` | Matches an input declaration key to obtain its input ID; it is not a separately resolved path. |
| Run-config scope `moduleMap` and output `destination` | Relative path-like values join the configuration's local filesystem/file-URI directory. A virtual `studio://` base does not perform this filesystem normalization. |
| Root-scope `baseUri` and invocation defaults | Separate execution context, requiring explicit provenance before admission. |
| Graph import `src` and export `out` | Both use `resolve_transform_graph_reference` against the acquired graph URI, exactly as the manifest execution provider does. |
| Later resolver reads and CLI positionals | Use their own resolver-request base or host working directory; these rules do not implicitly rebase run-config input declarations. |

A normalized output destination may still contain its authored relative value for a
virtual base. A field named `resolvedDestination` therefore does not prove an
absolute resource identity. File path normalization uses native path joining; hosts
must not substitute browser URL canonicalization.

The native `plan_run_config_relocation` and
`plan_transform_graph_relocation` functions produce candidates without fetching,
executing or storing anything. Both require canonical absolute configuration
endpoints and reject an identity base that conflicts with the acquired base.
An absent identity base remains absent; a matching explicit identity base must move
with the acquisition URI.

Run-config relocation retains exact bytes and admits absolute input/input-link
keys only. Relative keys, schema packages, resolver declarations, scope base/module
maps, pipeline dependencies, output destinations and unknown fields remain blocked.
Native normalized settings before and after the move must match under empty
invocation defaults. This does not prove external document dependency closure or
nonempty invocation defaults; those remain host admission obligations. Widening
relative inputs requires execution-owned lookup provenance, not an inferred
project-root or configuration-directory rule.

Graph relocation admits bounded opaque imports and literal outputs. It compares
each binding at the old and new acquisition URI. If the binding would change, it
replaces only that native literal value range with the original absolute key;
otherwise it retains the authored bytes. Output destinations are preserved along
with inputs. Same-directory moves and already absolute references normally need no
byte edit. Comments, Unicode, line endings and unaffected quotes remain exact.
Native reparsing checks the expected typed graph and the complete resolved binding
inventory at the new base. Patterns, selectors, transitive documents/templates/maps,
unknown or duplicate slots, and references to either configuration endpoint block
the candidate. Ordinary resource rebinding continues to reject graph relocation.

Repository adoption must plan against one immutable project snapshot, discover and
update incoming page/command/configuration references through their native owners,
check destination/resource/output collisions and local ownership, validate the
whole candidate project natively, and compare all affected revisions in the atomic
commit. A blocked plan must never partially rewrite storage. Undo must restore
exact prior bytes, paths and identities while advancing revisions. An empty
`rewritten_bytes` means retain source bytes; hosts must inspect blockers separately.
The worker's existing `planResourceRebind` control API now accepts optional
`mode: "rebind" | "relocate"` (default `rebind`). Relocation requires `fromUri` to
equal `baseUri`; mismatches and unknown modes fail at the native boundary.

Studio selects relocation only for the configuration being renamed. Other
configurations still receive ordinary rebinding plans against the same copied
snapshot. Incoming transform command `--config` slots use the existing shared CLI
parser and serializer. Page links retain stable resource IDs. The repository checks
the complete output inventory against existing resources and the proposed target,
then commits rewritten bytes, hashes, paths and revisions together. A moved resource
whose bytes also change advances its revision once. No-op candidates retain exact
bytes and still advance the moved resource revision. Configuration deletion remains
blocked.

The admitted command vocabulary has no custom resolver, module-map or global
run-config/default overrides. Graph commands permit only the existing bounded
reporting options; native graph planning requires opaque inputs without transitive
loads. Absolute run-config files retain their settings and input identities;
schema packages, scope-base overrides, module maps and destinations remain blocked.
This integration does not establish execution provenance for relative run-config
keys or admit general document/template dependency rewriting.

Real-worker fixtures execute both rebound and relocated graphs against exact
non-UTF-8 input bytes and verify the original output URI and payload. Repository
fixtures cover cancellation, stale revisions during planning, command/page links,
output collisions, no-byte-edit config moves and exact undo. Deployed acceptance
moves a graph between directories, runs its rewritten command, moves an absolute
run-config without changing its bytes, and restores both moves exactly.
`configurationRelocation` is a required lifecycle release-evidence flag.

CEM review: semantics and source edits remain native, with no JavaScript path
interpreter or document-object handoff. No UI, component behavior, styling or theme
ownership changes are required by this contract.


### Native source-map rewrite ownership

`cem_ml::source_module_map_rebind::plan_source_module_map_edit` plans v3 source-map
asset rebinding or relocation of the source map itself against a supplied immutable
asset snapshot. It uses the existing execution-aligned map inventory and declared
JavaScript edge verifier before and after the candidate change. No resource is
fetched, executed or stored.

Only `imports/<identity>/path` and `resources/<identity>/path` are owned edit slots.
Map relocation preserves resolved source locations; asset rebinding replaces each
path that resolved to the old asset URI. Changed slots use absolute URIs. Module
identities, bare specifiers, declared edge identities, cycles and asset payloads
remain unchanged. Candidate verification compares the entire expected inventory,
including resolved edge targets, content types, asset hashes and byte lengths.

The planner rejects duplicate keys at every ownership level, unknown fields,
non-v3 maps, conflicting identity metadata, nonliteral or self references, missing
assets, destination collisions in the supplied snapshot and known unresolved source
mechanisms. Non-bare imports, dynamic imports, template literals and non-JavaScript
assets still need their own analysis. A custom explicit base on a renamed asset is
also blocked. A matching explicit acquisition base must move with its resource URI;
hosts must preserve that metadata obligation as well as the path change.

These maps are an explicit JSON control boundary. Changed maps may be reformatted;
no-op maps retain their exact bytes. Before/after evidence binds map and asset bytes
to hashes. It does not bind resolver policy, resource identity metadata or revisions,
which hosts must guard separately.

Every successful candidate still lists all assets in `runtime_analysis_required`.
The lexical declared-edge check does not prove absence of `fetch`, workers, computed
URLs, generated loads or other runtime behavior. Candidates do not authorize a
project edit. Incoming graph/map references, document/template analysis, complete
snapshot ownership, identity/revision checks, atomic commit and undo remain separate
requirements. Destination maps retain publication semantics and are outside this
source-map API. Studio and the worker edit protocol do not admit source-map editing
yet.

CEM review: this step adds native control-document planning only. Document data
remains at the native import boundary; no JavaScript parser, UI behavior, component
style or theme definition was added.

### Native document and expression reference inventory

`cem_ml::document_dependencies::inspect_document_dependencies` inventories
namespace-owned HTML/SVG direct URI attributes from the retained semantic tree.
It shares native validation's direct-resource policy classification, preserves
native decoded values and attribute source maps, and retains the tree owner with
its node identities. HTML navigation links and base declarations have distinct
roles. Fragment-only and data URI values follow the existing policy exclusions.
This inventory does not resolve or acquire referenced resources; ordinary document
conversion does not itself dereference these attributes.

`cem_ql::dependency_inspection::inspect_query_resources` inspects compiled IR
without evaluation. Resolved standard-library identities distinguish `ct:read`
resource acquisition, `module:module_url` resolution and `data:read` content parsing.
Imported aliases follow their resolved module. Reserved calls follow the compiler's
existing dispatch precedence even when a declaration uses the same QName. The scan
covers the whole arena, including potential uses in unexecuted branches and function
bodies. A pipeline's first stage receives its source as the implicit argument;
later stages cannot assume that source remains unchanged. The current evaluator
rejects content-read and module-URL pipeline steps; their IR entries are conservative
candidates, not evidence that those steps execute. Only direct string
literals are recorded as candidate arguments, without folding or URI joining.

`inspect_template_expression_resources` additionally retains the template URI,
embedding byte offset and host source map separately from expression-local IR
provenance. Uncompiled expressions remain blocked instead of being reparsed.
Inspection budgets count tree nodes/attributes or IR nodes/pipeline stages;
truncation produces an explicit blocker.

Both APIs always retain `body_analysis_required`. An empty candidate inventory is
not dependency closure. Base and resolver policy, computed calls, unknown
vocabularies, unhandled CSS/presentation values, responsive-source selection,
embedded documents and runtime loads
still require analysis. Integration with native template preflight, retained CSS
resolution and immutable project snapshots remains follow-up work. These APIs do
not authorize source-map edits or widen worker/Studio edit admission.

CEM review: reference semantics and source ownership remain native. This step adds
no UI behavior, component styling, theme definitions or JavaScript document parser.

### Native preflight body inventory and explicit resolution context

`cem_ql::template_dependency_inspection::inspect_preflighted_bodies` consumes the
native template preflight result and its immutable imported bytes. It checks the
root URI/hash and every imported hash, rejecting stale bytes, absent/failed
preflight, unsupported source encoding and an exceeded body limit before returning
any body inventory. It does not reacquire imports, render templates or run queries.
Hashes establish byte correspondence; callers still own policy and revision guards.
The public preflight record is not a sealed authorization grant.

Each body is compiled independently under the supplied native compile options so
its expression inventory retains its own URI and hash. The bounded walk visits
attribute expressions/interpolation, body expressions, conditions, all choice
branches, loops, variables and payload selections. Expressions retain embedding
and query source maps. The per-body limit bounds traversal and each expression's
IR inspection separately; it is not a compilation memory or wall-time limit.
Skipped function bodies and compilation diagnostics remain explicit blockers.
This is body inspection, not linked-module execution validation.

`inspect_query_module_resolutions` resolves direct literal `module:module_url`
candidates through the execution capability with an explicit current context,
the CEM-QL purpose and the optional authored string referrer. It retains native
resolution results and policy errors. It does not substitute a template URI or
synthetic source URI for the execution context. Computed referrers, unsupported
pipeline steps, invalid arity and content reads cause no resolution call. Successful
resolution authorizes neither acquisition nor mutation; analysis blockers remain.

Authored CEM template source is not a rendered HTML/SVG document. The external-data
importer does not supply a CEM-template retained document, so this integration does
not route template bytes through that importer. Connecting output/document reference
ownership, stylesheet contexts, module-map contexts and runtime loads remains open.
Both the aggregate and its expression inventories retain body-analysis requirements;
worker and Studio edit admission is unchanged.

CEM review: analysis remains native and does not add app-local document parsing,
component behavior, CSS defaults or alternate theme ownership.

### Document and template stylesheet dependency ownership

`cem_ml::document_stylesheet_dependencies::inspect_document_stylesheets` combines
retained document reference inspection with native CSS import and URL resolution.
It inventories HTML/SVG `<style>` text and unqualified `style` attributes, preserving
the document owner, source-node identity and host source map. The CSS resource plan
retains its own imported tree and local source coordinates. Decoded document values
flow directly into the shared CSS importer; no HTML serialization, JavaScript
parser or string search for `url()` participates.

The host supplies the effective final document URL and execution resolution
capability. Relative CSS URLs use that explicit document URL; bare resource keys
use native module-map policy. Synthetic source URIs identify CSS coordinates only.
Native resolution errors and policy stamps remain attached to each candidate.
Imports are recorded with their native conditions without loading their targets.
Unknown style content types, structural/non-text style content and hard CSS import
diagnostics produce occurrence errors.

The complete bounded document inventory precedes every CSS resolution call. An
incomplete scan, import diagnostics or unresolved HTML/XML base ownership blocks
resolution, including when a `<base>` follows a style element. The API does not
infer browser base-element behavior or resolve XML base chains. CSS import/parser
limits remain separate from the document traversal budget. The resulting inventory
still requires import/runtime closure and retains the document's body-analysis flag.

Preflight body inventories now retain compiled stylesheet owners and authored module
maps. `TemplateBodyInventory::resolve_stylesheet_resources` uses the exact retained
sheet and the body's final preflight URI with the host's explicit execution
capability. The host must include the effective module maps in that capability;
retaining an authored map does not install it. Compilation errors and unknown
stylesheet indices reject analysis. Template source bytes are never treated as a
rendered document or reparsed to recover CSS.

These are context-specific inspection results, not live loader leases or mutation
proofs. Hosts must revalidate policy/revisions before use. Connecting generated
output references to source edit slots, resolving document base chains, SVG
presentation CSS, responsive-source selection and runtime loads remain open. The
bounded HTML srcset candidate extension is specified below. Studio/worker edit admission
is unchanged. CEM review: native source ownership and resolver policy remain shared;
this step introduces no component behavior, CSS defaults or theme definitions.

### XML base chains and native source correspondence

`cem_ml::document_reference_provenance::inspect_xml_reference_provenance` imports
exact XML bytes at the final acquired document URL. Direct reference candidates
reuse `RetainedCemTree::node_base_uri`, including inherited and absolute `xml:base`
values computed by native import. Results retain the original document owner,
attribute identity, effective base, source URI and opaque native source key.
Decoded attribute values are joined using the native URL implementation. This is
URL candidate resolution; resource policy and acquisition remain separate.

A complete bounded document inspection precedes resolution. Truncation, hard
import diagnostics and HTML `<base>` declarations prevent candidate results.
Malformed imported bases produce per-reference errors without fallback to the
acquisition URL. This explicit XML path does not emulate HTML browser base rules.

`bind_reference_source` correlates a retained attribute with an imported reference
slot only when native source key, source URI, expanded name and decoded value agree.
Missing, changed or ambiguous provenance rejects binding. The original source owner
is rechecked against its inventory. Returned indices identify native source slots;
they are not lexical replacement offsets. They do not establish exclusive ownership
of shared occurrences, a current project revision or a mutation authorization.
Generated output without preserved provenance cannot be edited through this API.

`inspect_xml_document_stylesheets` likewise imports XML bytes at the final document
URL and resolves inline CSS against each owner's imported base. The general retained
document stylesheet API remains conservative about unresolved XML bases, because
it does not own the original acquisition context. Both paths reject HTML base
semantics and preserve CSS policy results without fetching imports.

Safe lexical rewriting, generated-output mutation ownership, HTML base behavior
and full runtime dependency closure remain open. All body-analysis requirements and
Studio/worker admission guards remain in place. CEM review: native import owns base
semantics and source identity; no app-local parser, UI behavior or theme changes.

### Native XML reference value edit candidates

`cem_ml::import::xml_reference_edit::plan_xml_reference_edit` plans one original
XML reference occurrence selected by its native source key. Exact input bytes are
imported at the acquired document URL; stale keys, ambiguous selection, incomplete
inventories, unresolved neighboring targets and HTML base semantics reject the plan.
The new target must be an absolute URL. It is a candidate under outstanding host
resource policy, not a permission to acquire that URL.

Lexical ownership stays in the native import layer. The planner uses the retained
XML parser's attribute-value range and import-owned attribute correspondence,
checks the original lexical bytes against the retained semantic value, and replaces
only the quoted value's contents. Both quote characters and XML delimiters are
escaped; surrounding bytes and the enclosing quote style remain unchanged.
A target already resolving to the requested URL is an exact byte-preserving no-op.

The candidate is reimported at the same document URL. Reference count, native
attribute identities and inherited bases must remain stable; the selected target
must equal the requested URL and all other reference targets must remain unchanged.
Plans carry before/after content hashes and the original source key.

`plan_bound_xml_reference_edit` first binds a retained output/reference attribute
to that original source slot using the existing native correspondence check.
Missing or changed provenance fails. This does not establish that one generated
occurrence exclusively owns a shared source attribute. Plans retain explicit
runtime-closure, generated-occurrence ownership and host policy/revision/transaction
blockers. No file is written, no resource is fetched and Studio/worker edit admission
is unchanged. CSS references and other computed or embedded values are outside this
single direct-attribute editing contract.

CEM review: XML lexical work remains at native import; no application parser,
component behavior, styling or theme-token ownership changes were introduced.

### Shared XML occurrences and prepared transaction evidence

`cem_ml::xml_reference_transaction::prepare_xml_edit_transaction` binds a native
XML edit candidate to an explicit source snapshot, supplied output set and policy
identity. `Occurrence` scope admits exactly one matching output/attribute; if the
source attribute appears elsewhere in that set, it requires an explicit
`SourceWide` request. Source-wide plans list every affected supplied occurrence.
Zero matches, duplicate output IDs, absent revisions, unknown attribute provenance,
changed source correspondence and opaque reference nodes reject preparation.
The traversal budget covers the source and all supplied output nodes/attributes.

The prepared object has private state and retains immutable native output owners,
original source bytes for undo, candidate bytes and affected-occurrence evidence.
`validate_current` checks the source URI/revision/exact bytes, policy stamp and the
entire output membership, revisions and native owner identities. Reordering the
output list is harmless; adding/removing an output or replacing its native owner
invalidates the evidence even when the caller reuses a revision string.

This API proves correspondence within a host-supplied snapshot, not completeness
of all project outputs. The host must provide the complete relevant output set and
perform validation inside its atomic commit boundary. Policy stamps are host-owned
identities, not new grants. Prepared evidence continues to block on runtime
closure, output-membership completeness and the host transaction. It performs no
repository write and adds no worker/Studio edit admission. Generated attributes
without original native provenance require a separate derivation contract.

CEM review: occurrence ownership, value inspection and edit planning remain native;
no serialized document handoff, application parser, UI behavior or theme definition
was introduced.

### Bounded native HTML base resolution

`cem_ml::html_base::resolve_html_base_snapshot` resolves a fresh HTML base freeze
over a retained CEM document. The host supplies an explicit UTF-8 execution profile,
fallback context and policy identity. Ordinary documents supply their final URL;
`srcdoc` supplies its about-base URL; `about:blank` supplies its document URL and
optional about-base URL. Source-map/import URIs never supply execution bases.

The first HTML-namespace `base` with an unqualified `href` in document-tree order
selects the slot, including empty values. Its decoded value resolves against the
explicit fallback. Invalid URLs, `data:`/`javascript:` schemes and host policy denial
use that fallback; a later base cannot replace the selected slot. Target-only bases
do not select a URL base. XML `xml:base` chains are ignored in this HTML profile.
These rules follow the [HTML base-element contract](https://html.spec.whatwg.org/multipage/semantics.html#the-base-element)
and [document fallback rules](https://html.spec.whatwg.org/multipage/urls-and-fetching.html#document-base-urls).

Policy is a host callback, called once for a parseable, scheme-eligible candidate
after complete inspection. The request includes candidate/fallback URLs, selected
native slot and policy stamp. Hosts own CSP `base-uri` and other execution-policy
decisions. Unavailable policy returns an error; it cannot become an implicit allow.
The result retains the document owner, fallback context, policy identity, outcome,
effective URL and selected element/attribute, decoded value, source map, source URI
and optional native source key. Absent source provenance remains absent.

Node/attribute/diagnostic and URL-byte limits reject incomplete work. Hard import
diagnostics, opaque references, unevaluated attribute value nodes, unsupported encodings and ambiguous template
children also reject resolution before policy. The current retained representation
does not distinguish template content fragments from ordinary children. A host must
supply document topology with inert template contents excluded; a populated
`template` in the supplied snapshot fails rather than guessing that distinction.

This API consumes retained semantics; it adds no general `text/html` importer or
HTML tree-repair implementation. The shared parity corpus uses well-formed XML
imports as DOM-equivalent native snapshots and serves the original fixture markup
as UTF-8 `text/html` in Chromium. Its inert-template case explicitly supplies the
document-only native snapshot. The named JSON fixture/evidence boundary transports
test inputs and scalar results, never a parsed document AST. The CLI's Nx
`test:browser` gate compares native output directly with `document.baseURI`, including
CSP denial and real `srcdoc`/`about:blank` browsing contexts.

Chromium 148.0.7778.96 matches 22 of the 24 cases. Two explicitly version-pinned
differences remain: a first invalid href produces `about:blank` in Chromium instead
of the document fallback, and an empty href preserves the fallback's fragment
instead of removing it. Native resolution keeps the standard algorithm. The gate
asserts each exact browser value and version, reports those cases as divergences,
and requires review when either changes. These remain differences from the standard.
The explicit compatibility profile below can confirm their observed execution URL
as base evidence; general dependency completeness and edit admission remain blocked.

For a previously frozen live base, the host must preserve fallback and policy from
the actual freeze point. Supplying today's location after history or other runtime
changes does not reconstruct that history. Base resolution also does not prove
which resources were loaded earlier. All results retain `body_analysis_required`;
reference/CSS integration, runtime closure and atomic host admission remain open.
Existing XML and general stylesheet guards still reject HTML base declarations.

CEM review: semantics and source ownership stay native, with explicit host policy
and bounded work. No app-local document interpreter, UI interaction, accessibility
behavior, CSS default or theme-token ownership changed. Theme/forced-color checks
are inapplicable to this native-only API; physical AT acceptance remains separate.

### Captured HTML base evidence and browser compatibility

`cem_ml::html_base_evidence::prepare_html_base_freeze` captures an immutable native
resolution, its original fallback context, policy stamp and actual host policy
decision. Preparation accepts a retained execution snapshot, explicit compatibility
profile and a host-owned document lifetime ID, revision and nonzero base epoch.
`HrefFreeze` requires a selected href slot; `FallbackSnapshot` requires its absence.
The latter captures the current fallback without claiming that it is frozen.
The policy callback runs only when native resolution requires it, at most once;
its error prevents preparation. All existing UTF-8, topology and inspection limits
continue to apply, including rejection of unevaluated attribute values.

Preparation alone is not confirmed execution evidence. `PreparedHtmlBaseFreeze::confirm`
requires a scalar browser observation for the same document/revision/epoch, selected
native attribute and freeze policy. Browser identity and the exact serialized base
URL are checked. Private `CapturedHtmlBaseEvidence` retains both the standard native
result and the confirmed execution URL, the compatibility classification, source
owner/provenance, policy decision and observation. Confirmation never rewrites the
standard algorithm or accepts an arbitrary observed URL as an override.

Callers must explicitly select one of these profiles:

| Profile | Confirmation rule |
| --- | --- |
| `StandardAgreement` | Observed URL must exactly equal the native standard result; browser engine/version must be supplied and are retained. |
| `Chromium148` | Requires engine `chromium` and exact version `148.0.7778.96`. For ordinary HTTP(S) documents, an invalid first href expects `about:blank`; an exactly empty href that would discard a fragment expects the original fallback instead. Other cases require the standard result. |

The Chromium exceptions are limited to their native outcome and input predicates.
They do not generalize to other versions, whitespace-only hrefs, embedded-document
fallbacks or arbitrary URL mismatches. Unsupported versions and unexpected browser
values fail confirmation. The original 24-case corpus still reports 22 standard
matches and two standard divergences, while all 24 must match the explicitly chosen
Chromium profile. Actual browser observations are returned to native confirmation;
the gate does not simply compare two hardcoded expected strings.

The transition corpus also pins an unsupported Chromium behavior: after a CSP-denied
base is removed, a subsequent history change can leave the browser's base URL at
the old fallback despite having no base element. The native fallback snapshot uses
the current document URL, so confirmation rejects that observation and returns no
captured evidence. This stateful discrepancy is not included in the two bounded
compatibility exceptions. Browser updates must trigger fixture review, not silently
turn this mismatch into an execution base.

The execution host must capture fallback and policy at each real base freeze—when
a base becomes first or its href is set, including a same-value assignment. A URL
history change or later CSP change cannot replace those captured inputs. The
Chromium transition fixture records the old frozen URL after a history change,
re-resolution after resetting the same href, retained freeze policy after adding
CSP, denial at the next href assignment, selection of the next base after removal,
and current fallback after removing all bases. These tests use the
[HTML freeze triggers](https://html.spec.whatwg.org/multipage/semantics.html#the-base-element)
and [document base rules](https://html.spec.whatwg.org/multipage/urls-and-fetching.html#document-base-urls).

`CapturedHtmlBaseEvidence::validate_current` checks exact native owner identity,
document/revision/epoch, selected slot, browser identity, observed URL and current
policy stamp. It never reevaluates policy or substitutes today's document URL.
Changed read sets invalidate consumption even when the base URL string stayed the
same. Original freeze evidence remains available for inspection. The host must
advance revisions for relevant execution-context changes and epochs for new freezes
or fallback captures, and discard evidence if it missed events. Document IDs must
identify document lifetimes, not reusable URLs; a history A-B-A sequence or a new
document at the same URL must not reuse an old read set.

This API validates trusted host inputs; it does not authenticate an arbitrary
browser report or discover unobserved events. The historical transition driver is
a fixture. The bounded exclusive execution-host adapter below adds live native/DOM
correspondence and controlled event ownership; reference/CSS integration remains
separate. Test JSON is an explicit scalar
control boundary; source bytes enter native import, and no document AST is exported
to JavaScript. Runtime-load closure and the atomic host transaction remain blockers;
no resource is fetched and no Studio/worker edit admission is added.

CEM review: compatibility is explicit and bounded, semantic/source ownership remains
native, and both standard and browser results stay inspectable. No UI behavior,
theme token or inherited style changed. Physical AT acceptance remains independent.

### Exclusive HTML base execution host

`cem_ml::html_base_host::HtmlBaseExecutionHost` owns session identity, monotonic
revision/epoch counters, retained native source ownership, DOM slot identities and
pending/confirmed evidence. The shared `@epa-wg/cem-ml-cli/browser` export
`createBrowserHtmlBaseHost` connects that session through WASM to an exclusively
controlled browser document. This is browser execution infrastructure, with no
Studio-local parser, UI behavior or component styling.

The first profile accepts ordinary UTF-8 `text/html` HTTP(S) documents, explicit
browser identity/compatibility, and a retained native **execution snapshot** handle.
The snapshot must already describe the materialized document topology. The adapter
does not parse arbitrary HTML, derive native nodes from DOM, or export a JSON AST.
Native code asks synchronous predicates against its existing nodes; JavaScript
compares live DOM kind, expanded name, decoded value, parent/order and child/attribute
counts, retaining only DOM references and stable identities. Full bounded traversal
must succeed before the execution policy callback runs. Both preparation and reads
reject missing/extra/replaced slots, even in the body and even when base URLs agree.

Document topology excludes template-content and shadow fragments. Native templates
must therefore have no document children. The standard `html` doctype with empty
public/system identifiers and the root's exact inert HTML `xmlns` attribute are the
only DOM metadata omissions admitted by this initial correspondence profile, matching
native semantic import. Other parser repairs, namespace/attribute-order differences,
unsupported node kinds or ambiguous template ownership fail rather than being
silently reconstructed. Inert subtree dependencies remain outside this evidence.

`capture(nativeDocumentHandle, policy)` first checks correspondence and prepares the
native resolution using the current document URL and host-owned policy stamp/decision.
It then sets the selected href attribute to its current value, causing an actual new
freeze, and confirms the observed browser URL. With no selected base it captures a
fallback snapshot. This explicitly mutating operation must be used only where the
execution host owns base changes; attaching to an existing document does not certify
its previous frozen history. No policy is rerun by `read()`. The adapter revalidates
all native/DOM identities after the freeze and on each read, retaining the established
standard-versus-browser distinction and rejecting unsupported observations.
The packaged Chromium fixture also measures retention of the old allowed base after
adding a denying CSP and assigning the same href at the same fallback URL. Native
preparation expects the new policy to deny that freeze, so confirmation rejects and
revokes the session. This discrepancy does not expand the compatibility profile or
cause a hidden temporary href rewrite to force browser agreement.

The owner must route **every** history, policy and document change through the
synchronous `changeContext(() => { ... })` boundary. It invalidates before invoking
the callback, including changes restored to their original values. Supply the new
native execution snapshot at the next capture. Exceptions, asynchronous callbacks,
policy mutation/reentry, document replacement, page hiding and unexpected DOM
mutations revoke the session. Pending mutation records are checked synchronously
before consumption as well as at observer delivery; a DOM A-B-A sequence cannot
revive evidence. Disposal removes observation and releases the native owner.
The explicit scope follows the [DOM mutation observer contract](https://dom.spec.whatwg.org/#interface-mutationobserver)
and [HTML base freeze triggers](https://html.spec.whatwg.org/multipage/semantics.html#the-base-element).

Exclusive context ownership is a trusted host contract, not a sandbox or a claim
that MutationObserver observes history APIs. Uncoordinated same-task history A-B-A,
unreported policy changes, arbitrary scripts and other writers are outside this
profile. A changed live document URL is detected, but that check cannot reconstruct
missing historical transitions. Do not use this adapter to certify arbitrary live
pages. Embedded `srcdoc`/`about:blank` hosts and a general native HTML execution
snapshot producer still need their own integration. Existing observation-completeness,
runtime-load closure and atomic-transaction blockers remain; no Studio edit admission
or resource fetch is enabled by a confirmed URL alone.

CEM review: document interpretation and source identity stay native; shared browser
services own DOM access and bounded lifecycle control. No UI, accessibility, token,
inherited style or forced-color behavior changes; physical AT acceptance remains open.

### HTML execution reference and stylesheet inventories

`HtmlBaseExecutionHost::inspect_dependencies` now consumes a current captured base,
the actual native CSS resolution capability, an inspection budget and live DOM slot
predicates. It validates the host read set before inspecting the retained document
and again after resolution. The resulting private `HtmlExecutionInventory` retains
the exact captured base, document tree, CSS trees, reference/source identities and
resolver authority. `validate_dependencies` is required before later consumption;
an equal base URL, newly imported equal source, another host or equal resolver labels
cannot restore a stale inventory.

Direct HTML/SVG candidates from the existing document inventory resolve against the
**confirmed execution URL**. The standard base result remains inspectable through
`base()`, including any explicit Chromium compatibility difference. Neither the
document's source URI nor XML bases carried by the imported snapshot replace the
HTML execution base. Native decoded reference values pass directly to the URL
parser, preserving non-ASCII whitespace. Base declarations supply context and are
excluded from the resulting resource/navigation candidate list. Source URI, native
source key, owner/attribute IDs and original source frames remain attached.

Inline style elements and attributes reuse native CSS import and the execution
resolution capability, with the confirmed URL as their explicit referrer. Module-map
selection, blocked mappings, resolver errors and resource-policy stamps are retained
without fallback. CSS-local source coordinates remain owned by the retained CSS tree;
the occurrence also retains its document owner and host source frames. The general
stylesheet API and XML-specific API keep their existing rejection of HTML base
declarations. Only the host-validated HTML path can consume a captured base.

The complete document scan must fit the budget before any CSS resolution begins.
Resolver fingerprints must cover all resolution inputs and be nonempty; capabilities
without that guarantee cannot produce a retained inventory. The inventory binds both
the fingerprint and the exact native resolver owner/context, and checks them again
before returning or validating results. A changed DOM or resolver during analysis
prevents publication. Resolver replacement or policy drift later invalidates
consumption even if all resulting URL strings happen to agree.

These inventories remain bounded candidate analysis. The direct-slot collector's
existing coverage omits empty, fragment-only and data URI values and does not prove
every element-specific loading/navigation rule. Its raw document diagnostics remain
available; the consolidated `blockers()` discharges only the captured base requirement,
retaining resource policy, general-reference coverage, embedded/runtime content,
CSS imports, execution history and transaction requirements. Per-reference parse
errors and per-stylesheet import/resolution failures also remain explicit.

A current candidate plan does not establish which resources were previously loaded,
when CSSOM stylesheets captured their bases, or how fragment references are consumed
by a particular CSS property. Those remain execution/closure work; the
[CSS URL processing rules](https://drafts.csswg.org/css-values-4/#relative-urls)
distinguish relative and fragment URL handling. The shared Chromium fixture compares
native image/navigation candidates and inline style URLs with actual DOM properties
and computed styles after validating the live document through the packaged host.
Its cross-process JSON contains only named scalar test evidence, never a document AST.
Production native callers must provide their execution capability and revalidate the
inventory; fixture URL observations are not a browser/worker edit-admission protocol.

CEM review: source semantics, CSS parsing and resolution remain native; shared host
services own observation and invalidation. No application-local parser, UI behavior,
theme token, inherited-style or accessibility contract changed. Physical AT remains
independent and configured-project edit admission stays guarded.

### Native template and HTML dependency preparation

`cem_ql::html_template_dependencies::prepare_html_template_dependencies` combines
an existing host-validated HTML inventory with native template preflight and retained
compiled expression/CSS inventories. Construction runs the native preflight path with
an `Arc<EngineContext>` and `Arc<TemplateInput>`; caller-edited preflight reports cannot
construct the private preparation. Imported bodies retain their acquired bytes, final
URIs, hashes and native policy provenance. The root and engine owners remain retained.
Analysis does not render templates, evaluate expressions, execute content reads or
reacquire imports when validating the preparation.

The host supplies a native resolution capability for every final body URI, including
the root. Missing, extra or uncacheable contexts fail closed; duplicate body URIs with
conflicting content also fail. Capability selection is an explicit trusted execution
host input. It does not prove effective template module-map installation or historical
render context, and those requirements stay visible. Document candidates retain their
confirmed HTML base; template stylesheets use each body's final URI; expression
`module_url` calls use the supplied capability's execution context and explicit authored
referrer, when present. Source coordinates never substitute for these bases.

Expression reports retain their native compiled queries in addition to host source
frames and resource-use locations. Literal module calls resolve through the native
execution capability, retaining denied mappings and policy errors. Computed arguments,
pipeline calls, content reads and other unknown calls remain blockers. XPath findings
from native preflight remain attached and contribute their unresolved requirements.
The preparation retains distinct template and document owners; their coexistence is
not evidence that this template produced this particular document.

Every body must pass compilation and bounded traversal before expression/CSS resolution.
Per-body budgets cover template traversal, each compiled expression inspection and the
body's retained stylesheet trees. Native preflight uses its import-depth bound and
engine operation controls; the body-count limit applies after preflight and is not an
acquisition-count limit. Snapshot-only hosts use the sealed snapshot execution path
below; installing an engine resolver does not replace the ordinary file-import path.
Fingerprints are captured before preflight callbacks, checked before body resolution
and checked again with the live HTML host before publication. Later consumption requires
`validate`, which checks exact root/engine/capability owners, complete context membership,
resolver fingerprints and the current document inventory. Equal source bytes or labels
cannot replace a retained authority. Validation performs no resource reads.

This API is native candidate preparation, with immutable inspection access and no edit
or browser/worker admission grant. It preserves document coverage/policy, stylesheet
execution history, template output correspondence, loader freshness, runtime closure
and atomic host transaction blockers. An engine owner and source hash establish the
retained configuration and bytes, not the freshness of a mutable resolver's backing
store. Execution records and a complete dependency read set must establish those facts
before map rewriting or configuration relocation can consume edit evidence.

CEM review: parsing, expression semantics, source ownership and policy stay in shared
native libraries. No application UI behavior, theme token, inherited style, accessibility
or forced-color contract changes. Physical AT acceptance remains independently open.

### Recorded snapshot XHTML execution

The first execution-record profile uses `TemplateSourceSnapshot` and
`preflight_template_snapshot` in CEM-ML. A snapshot owns canonical absolute source
URIs, revisions, content types and exact bytes. Its membership and content are
immutable, with limits on source count, stored bytes, import edges and cumulative
imported bytes. Native preflight still applies declared import policy, substitutions,
cycle/depth checks and operation cancellation. Every acquisition in this profile is
a snapshot lookup, including file URLs; absent entries never fall back to filesystem,
network or registered resolvers. The private `SnapshotTemplatePreflight` retains the
snapshot, root and engine owners with the actual native import report. Consumption
requires the host's current snapshot and engine to be those exact owners. The host
must publish a replacement snapshot whenever source content or revision changes.

`cem_ql::template_execution_record::execute_snapshot_xhtml` performs a real render
from that sealed preflight. It retains the compiled closure, body inventories,
scalar input values, native resolver capability and fingerprint, output source spans,
XHTML bytes and imported output tree. This operation is separate from inventory
inspection. The profile permits scalar bindings and directly inspectable literal
module-URL calls under one shared native resolution context. An explicit renderer
entry point threads that capability into evaluation; existing render entry points
keep their previous behavior. Declared imports use the captured bytes without new
acquisition. Unknown calls, runtime content reads, computed/pipeline module resolution,
XPath invocations and authored body module maps fail before rendering. Bounds,
compilation/render errors, host-attribute updates, cancellation and policy drift
prevent publication of an execution record.

The named output boundary is XHTML: the native XML writer serializes the real render
plan, and the shared CEM-ML XML importer retains those exact output bytes. A single
HTML-namespace `html` document element is required. This is not a general HTML parser
or a DOM reconstruction protocol. The host must mount and capture this exact output
through the existing exclusive HTML execution host. Browser normalization that breaks
native/DOM correspondence still fails that host's checks. The output import's source
coordinates describe the produced XHTML; original render spans and template owners
remain separate and do not automatically establish source-edit locations.

`SnapshotTemplateExecution::prepare_dependencies` accepts only an HTML inventory whose
native tree is the execution's exact output owner. It reuses the retained preflight
and body inventories rather than acquiring or compiling them again. The resulting
private `ExecutedHtmlTemplatePreparation` requires the same execution, source snapshot,
engine, resolver and current HTML host at consumption. A second execution, equal
reimported output, replacement source snapshot, stale host or changed policy cannot
substitute for these owners. The combined blocker view discharges template/output
correspondence, template acquisition freshness and output-document inspection for
this bounded profile. The underlying candidate report retains its original findings.

The scoped extension below records per-body contexts and inline module maps.
Native data/focus inputs, runtime acquisition, stylesheet installation/history,
CSS, responsive-source selection and other document-reference coverage, map rewrite ownership and atomic
host transactions still require their own evidence. This profile enables no Studio
edit admission and exposes no browser/worker record-construction shortcut. Hosts
remain responsible for supplying their current immutable snapshot and execution
record inside the eventual transaction boundary.

CEM review: execution, resource interpretation and source retention stay in shared
native libraries; inputs are explicit scalar controls, with no JSON AST handoff.
No UI behavior, accessibility, theme-token, inherited-style or forced-color contract
changes. Physical AT acceptance remains independently open.

### Recorded lexical body contexts and module maps

`execute_snapshot_xhtml_scoped` extends the scalar snapshot profile with native
contexts for the root and each imported source body. The host capability is the
outer context. Each body appends one frame, based at its final snapshot URI, and
installs its compiled static top-level `module-map` into that frame. Imported
bodies inherit their declared import ancestry, independent of the dynamic caller.
Relative expression URLs and inline map targets use the body's URI; explicit
URL referrers still select scopes under the existing resolver contract. Declared
template imports remain the sealed native preflight acquisitions; this extension
does not reinterpret or reacquire them through the expression module maps.

The shared native resolver derives immutable child contexts, retaining ancestor
frames, blocks and allowed-scheme restrictions. Non-CSS mappings remain outer-first;
CSS mappings remain closest-first with ancestor restrictions. Map entries/scopes
use the per-body inspection budget, and native frame depth is bounded. Custom
resolvers must explicitly implement native derivation, preserving those contracts
and complete fingerprints; unsupported derivation fails. This profile has scalar
inputs and adds no node-referrer bindings or external map loading.

The closure compiler attaches the exact body capability to every compiled expression
before merging imported declarations. Named calls, match rules, expression hooks,
attributes, variables, conditionals and iteration retain that lexical capability.
Evaluation installs it for the expression and restores the caller afterward.
Authored module markers, synthetic source coordinates and template-name strings do
not select authority. These native capabilities are omitted from serialized template
artifacts; reload cannot mint execution records or recover their authority. The
ordinary compiler and shared-context execution entry point keep their existing
semantics.

One final source URI must have one import ancestry in this profile. Equal repeated
imports under the same parent reuse the context when native preflight accepts the
graph. A source reached under different parents is rejected because closure
compilation currently deduplicates by URI; occurrence-specific compilation is needed
before that case can be admitted. Nested unapplied maps also fail rather than being
treated as installed preludes.

The execution record retains every derived capability and its complete fingerprint.
Dependency preparation uses those same owners for expression and stylesheet analysis;
publication and consumption validate both host and body fingerprints. The combined
view additionally discharges effective template-map and module-resolution-context
blockers. Mapping denials, stylesheet execution/history, full document/resource
coverage, runtime closure and atomic transaction blockers remain applicable. This
native extension does not widen Studio edit or browser/worker admission.

CEM review: context construction and expression semantics remain shared native
services with explicit, bounded ownership. Native retained sources and the shared
import boundary are preserved. No UI, accessibility, public token, inherited style
or forced-color contract changes; physical AT remains independently open.

### Retained HTML image-source candidates

`cem_ml::import::srcset` parses decoded retained attributes using the
[HTML image-candidate algorithm](https://html.spec.whatwg.org/multipage/images.html#parsing-a-srcset-attribute).
The supported slots are HTML `img@srcset`, `source@srcset` and
`link@imagesrcset`, with unqualified attributes and exact retained ownership.
These are potential source lists; surrounding picture, media, type, rel/as and
sizes conditions do not establish selection or acquisition. Unsupported slots
retain their analysis blocker.

The shared importer handles ASCII whitespace, internal URL commas, descriptor
parentheses and recovery. It retains accepted URLs, density/width descriptors,
future-compatible height descriptors and structured diagnostics. Width/height
integers are positive `u32`; density uses finite `f64` with HTML float syntax,
including zero. Valid numeric syntax outside that profile makes analysis
incomplete. Parse errors discard invalid candidates while preserving recovery;
future-height and extra-comma diagnostics stay visible. Cross-candidate authoring
conformance and browser source selection remain separate from this parser.

Each retained occurrence owns the original CEM tree, element/attribute IDs, source
key and whole-attribute source map. Candidate URL and descriptor ranges are UTF-8
byte offsets into the **decoded attribute value**. Entity decoding and source
normalization mean these are not original-file edit ranges or template rewrite
locations. Candidate lists remain separate from single-URI document references.
Default limits are 65,536 decoded bytes and 4,096 candidate attempts per list,
including malformed candidates. Document inventory charges list bytes against
its shared node/attribute work budget. Exhaustion prevents host publication;
unsupported numeric values also prevent CSS callbacks and complete inventory.

`HtmlBaseExecutionHost::inspect_dependencies` resolves each accepted candidate
against the confirmed execution base and retains its source occurrence. Imported
XML bases, source URIs and module mappings cannot replace that base. A failed URL
resolution stays explicit. The existing host/DOM/capability checks apply before
publication and at consumption. Supported complete lists discharge only their
parsing and base-resolution requirements; parse diagnostics, resource policy,
source selection/load history, general coverage and atomic edit admission remain
explicit. No fetch, browser-selection emulation or source mutation is performed.

The native and Chromium fixtures consume the same XHTML bytes. Single surviving
candidates compare with actual `currentSrc` for URL commas, entity/Unicode
handling, descriptor recovery, width/height, exponent and zero-density cases.
This bounded parity evidence does not certify a complete browser selection model.
CEM review: parsing and source authority remain shared native capabilities. No
UI, public token, accessibility or inherited-style contract changes; theme and
forced-color checks are inapplicable. Physical AT acceptance remains independent.

### Immutable CSS import snapshot capture

`cem_ml::css_import_snapshot::CssSourceSnapshot` retains already acquired response
bytes, requested/final absolute URLs, content types and opaque revisions. The
trusted host supplies its current immutable snapshot at capture and consumption.
Duplicate request URLs and conflicting aliases for one final URL are rejected.
Equal copies or revision labels cannot replace the exact snapshot owner. This
records snapshot freshness; it does not monitor transport or filesystem changes.
The host must replace its snapshot when acquisition state changes.

`capture_css_import_snapshot` drives the existing native `CssImportClosure` with
only those responses. Missing imports fail with native import location evidence;
there is no filesystem, registry or network fallback. Existing shared admission
owns MIME, integrity, final-URL policy, CSS parsing, import placement, cycles and
conditions. Repeated imports remain distinct occurrences, including anonymous
layers; each occurrence retains its acquisition record and native source tree.
Relative child imports and resource URLs use the response's final URL. The root
keeps its caller-owned retained tree and explicit execution URL, including for
inline CSS whose synthetic source URI is only a coordinate identifier.

The snapshot defaults to 256 responses and 16 MiB including acquisition metadata.
Capture defaults to 64 sheet occurrences, depth 16, 65,536 retained root nodes and
the existing response/cumulative byte limits. Repeated responses count repeatedly
against capture limits. Root node bounds are checked before resolution; downloaded
CSS also keeps the shared importer's byte/event/depth bounds. Cancellation,
incomplete resolver fingerprints and observed policy drift prevent publication or
later consumption. Observed resolver drift stays invalid after an A-B-A change.
The sealed result checks exact root, base, capability and snapshot owners; its
public diagnostic views cannot construct a replacement record.

`HtmlBaseExecutionHost::inspect_css_imports` captures every inline stylesheet root
from an exact `HtmlExecutionInventory`, preserving its document/source mappings
and confirmed base. It checks all root errors and aggregate root bounds before
new resolution, then shares sheet and response-byte budgets across the forest.
`validate_css_imports` checks the current HTML host, DOM correspondence, resolver,
snapshot and cancellation state around consumption. Stale or substituted owners
fail; successful inspection discharges only the retained inline roots' import
coverage. The combined blocker view retains CSS runtime closure, condition and
installation/load history, ordinary resource-resolution failures and atomic edit
admission. No conditions are evaluated and no CSS is installed by this API.

External link stylesheet roots, recorded template-body CSS and SVG presentation
URL values use the integrations below. Responsive-source selection and general
runtime loads still require separate execution-owned evidence.
This extension does not widen
Studio/browser-worker edit admission or convert CSS-local ranges into template
rewrite locations. CEM review: shared native loading/import/source semantics and
bounded inputs; no app-local parser, UI behavior, public token or inherited-style
change. Theme/forced-color checks are inapplicable; physical AT remains open.

### Recorded template CSS import binding

`ExecutedHtmlTemplatePreparation::prepare_css_imports` binds immutable CSS
response snapshots to the exact native execution and dependency preparation.
The sealed `ExecutedTemplateCssImports` retains that preparation, the document
CSS capture and a closure for each compiled stylesheet in every recorded body.
Body and stylesheet occurrence order are preserved, including bodies with no CSS.
Public inventory fields and equal serialized artifacts cannot create this record.
It reuses retained roots, sources and capabilities without recompiling templates,
rendering again or reacquiring template imports.

Each body closure uses that body's final template URI and recorded effective
capability. Its lexical module map, ancestor restrictions and closest-first CSS
mapping policy remain authoritative. Inline document roots use the confirmed HTML
base and document capability instead. Imported sheets keep the owning body's
capability but use each response's final URL for relative references. CSS-local
source coordinates remain attached to their exact roots; synthetic CSS source
URIs, output document coordinates and authored body labels cannot select a base
or substitute an execution context.

All document and body root counts and node bounds are checked before new CSS
resolution. One aggregate budget then covers root/import sheet occurrences and
response bytes across the document and all bodies; repeated imports count again.
The existing loader retains conditions, native source maps, response revisions,
redirect/MIME/integrity/policy checks and cycle/depth limits. Missing snapshot
responses fail without fallback I/O, and execution cancellation applies to all
captures. Failed or partial captures do not publish a combined result.

Consumption validates the exact execution, engine, template snapshot, CSS
snapshot, host and capability owners. Every recorded body fingerprint is checked,
including bodies without stylesheets, before and after DOM callbacks. Observed
internal policy drift permanently invalidates the CSS preparation, even if the
policy later returns to its original value. A rejected caller-supplied replacement
owner does not invalidate otherwise current original evidence. The trusted host
must supply its current immutable acquisition snapshots and invalidate its HTML
session when execution state changes; this API does not watch external resources.

Only the combined result discharges `stylesheet-owner-context-required`,
`template-stylesheet-import-closure-required` and the retained document roots'
`css-import-and-runtime-closure-required`; the original candidate inventories
remain unchanged. Resource denials, CSS conditions and installation/load history,
template/runtime dependency coverage and atomic transaction requirements stay
explicit. These are potential dependencies across recorded bodies, including
unused declarations; capture neither proves stylesheet installation nor authorizes
source edits. External link roots and stylesheet execution observations use the
additional bindings below; broader runtime resource coverage remains separate.

CEM review: shared native import/loading semantics, explicit retained ownership
and bounded variation are preserved. No UI behavior, public token, accessibility
or inherited-style contract changes; theme/forced-color checks are inapplicable
and physical AT acceptance remains independently open.

### External stylesheet snapshot roots

`HtmlExecutionInventory::stylesheet_links` records HTML-namespace `link` occurrences
whose ASCII-case-insensitive `rel` tokens include `stylesheet`. It retains the
href attribute owner and authored media, title, alternate, disabled, type and
integrity metadata. This follows the [HTML stylesheet link contract](https://html.spec.whatwg.org/multipage/links.html#link-type-stylesheet).
Inert template descendants do not become link roots; the current HTML host also
rejects unseparated template content. Foreign namespaces and other link relations
remain outside this root inventory. Missing/empty href, untitled alternate links
and unsupported types prevent publication of this bounded capture.

`inspect_css_imports` now captures external roots alongside retained inline CSS.
The confirmed HTML base resolves the href; module maps may restrict that absolute
URL but cannot replace it. Requested and final response URLs require execution
policy approval. Root responses use the same native MIME, integrity, byte-bound
and CSS import admission as nested imports, with the link's integrity expectation
checked additionally. Exact snapshot response owners and revisions are retained.
Relative imports use final response URLs. Repeated links remain separate roots,
including disabled and alternate candidates; acquisition does not imply selection.

Inline, external and recorded body roots share occurrence, root-node and cumulative
response-byte budgets; external response bytes count as well as imported bytes.
Known retained roots are bounded before new resolution, and external roots pass
response/import bounds before traversal. Missing snapshots never trigger I/O.
Current snapshot, capability, HTML host and DOM ownership are rechecked at
consumption. The combined view clears only external stylesheet closure coverage.
Browser transport/CORS/CSP policy and actual installation require independent
host evidence; no stylesheet is fetched or installed by this capture API.

### Exclusive stylesheet execution observations

`StylesheetExecutionHost` records a bounded observation of every root and imported
sheet occurrence. `StylesheetExecutionObserver` is a trusted execution adapter:
it compares actual loaded sources and installed occurrences with the native
requests and reports selection, effective condition results, loaded source keys
and stable installation identities. Requests include native trees, final URLs,
root owners, media/title/alternate/disabled metadata and native import conditions.
The adapter must verify the actual bytes/compiled-source correspondence before
returning a source key; echoing that key is not an observation implementation.

The host requires complete observations, unique nonzero installation identities,
correct loaded source fingerprints, load/installation evidence for active sheets
and consistency with disabled roots and inactive import ancestors. Imported
conditions are observed by the execution engine; native capture does not emulate
media/supports evaluation or infer stylesheet activation from source text. Inner
rule conditions, computed values and resource loads remain runtime coverage.

Each capture supersedes the previous epoch. Its environment revision must cover
CSSOM edits, stylesheet/load replacement, selection, media/supports changes and
installation changes. The adapter must invalidate before controlled changes,
including same-value transitions; observed mismatches or callback failure revoke
the evidence permanently. Unsupported observation, untracked external writers,
shared installation handles and missing source correspondence fail this profile.
Two capture passes and later consumption probes reject changing observations.
Public diagnostic records cannot construct the sealed execution evidence.

`HtmlExecutionCssImports::record_execution` and
`ExecutedTemplateCssImports::record_execution` bind this evidence to the original
CSS preparation, source snapshots and live HTML host. The template form includes
recorded body capabilities and stylesheet scopes, including unused declarations
whose inactive status must be observed. Original candidate blockers remain intact;
only the combined result clears stylesheet/import-condition history requirements.
Runtime loads and atomic edit admission remain blocked. These native APIs record
host observations without installing CSS. The bounded shared browser
mutation/lifecycle adapter and the separate compiled-installation contract are
specified below. Flattened compiled imports have one installation per output sheet,
so they must not be reported as independently installed source imports.

The shared Chromium fixture supplies actual CSSOM/load observations for five
inline/link/import occurrences, including a disabled alternate root. It verifies
loaded bytes, active styles, disable/restore/media transitions and native stale
rejection. This bounded parity evidence does not certify arbitrary browser writers
or every conditional stylesheet grammar.

### Retained SVG presentation URL values

The shared `import::svg_presentation` boundary imports the URL-capable SVG
presentation slots `fill`, `stroke`, `filter`, `clip-path`, `mask`, `marker-start`,
`marker-mid`, `marker-end` and `cursor`. SVG presentation values use CSS property
syntax, as specified by [SVG styling](https://www.w3.org/TR/SVG/styling.html#PresentationAttributes).
Only unqualified attributes on SVG elements are admitted. SMIL elements retain
animation/runtime requirements; legacy `color-profile` and `marker` shorthand
retain explicit analysis requirements.

The importer retains the exact document, attribute and native CSS owners. It
accepts a single property value and rejects declaration injection, `!important`,
malformed CSS and bounded-work exhaustion. CSS escapes, quoted URLs, fallbacks,
local fragments and multiple cursor candidates are handled by shared native CSS
syntax, with no downstream string scanning. Offsets can be projected into the
**decoded attribute value** by subtracting the import-owned property prefix;
entity decoding means these are not source-file edit coordinates.

Document inspection charges decoded value bytes against its work budget. Complete
analysis precedes resolution callbacks. HTML execution uses its confirmed base;
explicit XML inspection uses the import-owned per-node XML base chain. Resolver
policy failures remain in native plans and combined blockers; stale HTML hosts
cannot consume them. The Chromium fixture compares SVG paint URL resolution and
local-fragment computed-style serialization with the same native source.

This is a potential dependency inventory, not CSS property conformance, computed
style, filter/pattern traversal, SVG animation execution or resource-load history.
Those runtime requirements remain visible. Across these additions, CEM review
confirms shared native import/loading services, bounded inputs and explicit source
and execution ownership. UI behavior, accessibility, public theme tokens and
inherited overrides are unaffected. Theme/forced-color checks are inapplicable;
physical device/AT acceptance remains independently open.

### Native computed SVG presentation observations

`cem_ml::svg_computed_execution::SvgComputedHost` records current computed
values for explicitly supplied retained SVG presentation attributes. Each slot
keeps the exact document, attribute and imported CSS owner. An exclusive observer
must verify the live element correspondence and supply an environment revision
covering style, animation and owner transitions, including same-value restoration.
Capture compares two complete passes bracketed by revision reads. Validation
checks native owner identity, revision and every computed value. Failure or
explicit invalidation permanently revokes that capture; recapture supersedes old
evidence even when the new attempt fails.

The profile admits at most 256 unique attributes from one document, with at most
4096 UTF-8 bytes per computed value and environment revision. Computed strings
are opaque browser observations. A cascade override can legitimately differ from
the authored presentation value; neither equality nor a computed URL establishes
which declaration won or which response bytes were consumed. Evidence retains
explicit winning-source, acquisition and general-runtime coverage blockers.
The supplied subset is not proof of complete document coverage. The caller must
also validate its document/base/style evidence before and after consumption.

Shared `BrowserRecordedOutputHost` now supplies exact native-owner scalar
computed-property reads for both synchronous and asynchronous capture. The native
session retains the SVG sources and evidence; the WASM boundary captures before
whole-output preparation and validates afterward, then validates before and after
each read. Worker-pool replacement, cancellation and disposal close the same
session. `svgComputedSlots` is a diagnostic count, never transferable evidence.

The browser profile requires static live SVG owners. It rejects document Web
Animations/CSS animations and SMIL elements when SVG slots are observed. Document
mutations, input/focus/pointer state events and watched platform preference changes
revoke the host; computed values are also reread during validation. As with CSSOM
ownership, callers must use `host.change` before controlled changes, including
same-value restoration. Polling a value cannot establish unobserved transition
history. Observations of inherited values and cascade overrides do not identify
winning declarations or acquired response bodies.

Properties without authored presentation attributes, pseudo-elements, SMIL
sampling, paint-server traversal, general CSS resource selection and response
delivery remain outside this bounded profile. Their blockers and transaction
admission restrictions remain in place.

### Local SVG paint-server dependency plans

`cem_ml::svg_paint_dependencies::SvgPaintPlan::from_inventory` defines a
conservative candidate graph over one retained HTML execution inventory. Roots
come only from native CSS resource references in SVG `fill` and `stroke`
presentation attributes. Each root retains the exact document, attribute, imported
CSS owner and CSS reference node. The full authored property value preserves any
fallback syntax; the plan does not decide whether that fallback is valid or used.
It does not parse computed CSS strings or infer which declaration won the cascade.

For literal ASCII fragment IDs, lookup spans the active document, including
foreign-namespace elements that can cause ID collisions. A unique SVG
`linearGradient`, `radialGradient` or `pattern` is a local candidate. Duplicate IDs
remain ambiguous instead of guessing browser selection. Missing IDs, unsupported
fragments and wrong element types are explicit outcomes. HTML template content is
excluded. Encoded/Unicode/SVG-view fragments and non-fragment URL aliases remain
untraversed in this initial profile.

Template edges retain their native element and attribute owners. Unqualified
`href` takes precedence over `xlink:href`, including an empty modern attribute.
Gradient chains may connect either gradient type; patterns may template only
patterns. Per-root visited sets terminate self-links and longer cycles without
mistaking shared targets across roots for cycles. The plan conservatively records
potential template links even when authored attributes or child content might
make their values unnecessary. Effective attribute inheritance and cloned-content
style evaluation are separate work. These choices follow the relevant
[SVG paint-server](https://www.w3.org/TR/SVG2/pservers.html) and
[linking contracts](https://www.w3.org/TR/SVG2/linking.html).

CSS fragment-only URLs remain local candidates. Template `href` fragments use a
stricter bounded profile: traversal requires an HTTP(S) document fallback and
confirmed HTML base to agree after removing fragments. A different base or an
embedded document yields `BaseContextRequired`. Every non-fragment edge retains
its authored URL as `UntraversedUrl`, including possible same-document aliases;
there is no fetch, module-map substitution or external-document import. XML
execution with per-element bases is outside this HTML inventory API.

Construction is atomic on budget exhaustion: at most 256 roots, 64 template links
per root, and a caller work budget capped at one million units. Document nodes,
attributes and value bytes plus graph visits are charged. The plan retains its
original inventory; live consumers must validate that inventory through its
original HTML host and resolver capability. The plan supplies no execution seal.
Rendering, paint-content dependencies, winning-source and acquired-body blockers
remain. Pattern children, gradient stops, inherited styles, animation and external
resources therefore cannot disappear from dependency requirements just because a
local template chain was found.

Recorded browser preparation now constructs the plan from the exact native
inventory passed to whole-output preparation. The plan stays inside that session;
publication follows successful preparation and a validated read. Subsequent reads
revalidate the same document/base/resolver/style read set before returning plan
diagnostics. Target replacement, ID or template-link mutation, base changes,
cancellation and worker-pool closure invalidate the session and release the plan.
Synchronous and asynchronous browser capture share this path.

`svgPaintRoots` and `svgPaintLinks` report bounded diagnostic counts only. Unresolved
target, cycle, base-context and untraversed-URL diagnostics join the existing
blockers. No native graph, source owner or evidence seal is serialized to JavaScript
or worker messages. Chromium fixtures compare DOM fragment lookup, template href
reflection and computed fragment/fallback serialization with native candidate
classifications. Duplicate IDs deliberately remain ambiguous even when Chromium
returns a first element. Different-base template references remain unresolved.
This verifies candidate correspondence and invalidation, not effective paint
inheritance, pixels, winning declarations or acquired resource bodies.

### Retained SVG paint content and source ancestry

`cem_ml::svg_paint_content::SvgPaintContentPlan` retains an `Arc<SvgPaintPlan>`
and inventories authored content for each distinct local candidate server reached
by that plan. Repeated roots and template cycles share one server entry. Unknown,
external and unresolved template targets do not create invented content. Local
and template-server entries stay separate even when a server has its own stops
or children; this is a conservative source inventory, not effective-content
selection or gradient attribute inheritance.

Entries retain native node IDs in source order, including the server, descendant
text and descriptive content. Only direct SVG `stop` children of gradient elements
are classified as gradient stops. Pattern subtrees keep nested groups, styling,
images and other content. HTML template contents remain excluded. Direct URI
attribute owners are joined from the existing native document-reference inventory;
CSS URL resolution, nested resource execution and load receipts remain separate.
The original tree keeps all attributes, source maps and imported CSS owners alive.

Each content node records its lexical source ancestors, nearest parent first.
This preserves the authored context for inherited style analysis without copying
values from the referencing graphics element. It does not evaluate `currentColor`,
custom properties, cascade rules or properties inherited by cloned template
content. SVG templates can create instances with different inherited styles;
that distinction remains an explicit blocker, following the
[SVG paint-server contract](https://www.w3.org/TR/SVG2/pservers.html#PaintServerTemplates).

Construction admits at most 256 distinct servers, 4096 node memberships across
server entries, 128 ancestors per node and a caller work budget capped at one
million units. Node/attribute values, ancestry visits and direct-reference scans
are charged. Exceeding any bound returns an error without publishing a partial
plan. Entries retain the original inventory and require its original host/resolver
validation for live use; source inspection after invalidation grants no authority.

Content evaluation, cloned-instance style provenance, rendering, winning-source
and acquired-body evidence remain blocked. Browser preparation constructs the
content plan from the same retained traversal before publishing the validated
session. Failed capture, invalidated reads, cancellation and closure release both
plans. The existing complete DOM/base/stylesheet validation owns their lifetime;
even restored stop values, pattern text/children and ancestor style/class changes
revoke the preparation.

`svgPaintServers` and `svgPaintContentNodes` expose only bounded diagnostic
counts (256 servers and 4096 node memberships). Chromium fixtures exercise both
capture paths, compare gradient/template and pattern membership, and reject
mutation or pool closure without revival. Native source ancestry is never
serialized. No component style or public theme-token default is introduced.

### Cloned paint-template style provenance

The native `cem_ml::svg_paint_style::SvgPaintStylePlan` defines structural
provenance candidates for the SVG 2 paint-template model. It retains the original
`Arc<SvgPaintContentPlan>`; no source tree, CSS owner, source map or URL base is
copied into JavaScript. This is a specification-profile plan, not browser-observed
clone evidence. The [SVG 2 template contract](https://www.w3.org/TR/SVG2/pservers.html#PaintServerTemplates)
allows cloned children to inherit differently from their source elements.

Each reached local server has a separate host context. Selection records:
authored content (`Own`), a local template source (`Template`), no relevant
content (`Empty`), or the unresolved native target classification. Under this
profile, a direct element child other than SVG `title`, `desc` or `metadata`
selects authored content. Text/comments alone do not; similarly named elements
in other namespaces are not descriptive SVG elements. Descriptive-only servers
follow the retained href chain. Own content ends content selection even if
template links remain relevant to attribute inheritance. External, ambiguous,
wrong-kind and unknown targets retain their unresolved classification.

A selected template contributes its retained descendants, including descriptive
content, without cloning its server node. Each prospective instance records its
native source node and either the receiving host or another instance's source
node as its parent. Instance identity is therefore **plan + host + source node**,
never source node alone. Direct children inherit through the receiving server;
nested children keep their internal instance-parent relationships. Indirect
templates lead directly to the final source content: intermediate servers are
recorded through href attribute IDs, not inserted as inheritance parents.
Selection detects cycles per host after testing authored content.

The source plan continues to own lexical source ancestry, declarations, resource
bases and source frames. The style plan separately records the receiving server's
lexical ancestors. Neither is replaced with the graphics consumer's ancestry.
A future style evaluator must distinguish source declaration/selector provenance
from instance inheritance, including custom properties, `currentColor`,
explicit inheritance and selector scope across the template boundary. Source
`getComputedStyle` values cannot stand in for instance styles. This API does not
apply selectors, evaluate values, inherit gradient attributes, choose rendered
resources, or materialize browser shadow trees.

The profile permits at most 256 hosts, 64 template edges per selection and 4096
aggregate prospective instance nodes; shared sources count once per receiving
host. Work is capped at one million units, with scans, edges, ancestry copying
and instance memberships charged. Limit failures publish no partial plan.
The original inventory still requires its original host/resolver validation.
Browser preparation now constructs this plan from its retained content plan before
publication. Failed capture, stale reads, cancellation and closure release it with
the owning preparation. Complete DOM/base/stylesheet validation covers source
and host ancestry, content and template links; restored mutations cannot revive
a plan. The bounded `svgPaintStyleHosts` and `svgPaintStyleInstances` diagnostics
expose counts only, never source identities, instance graphs or evidence seals.

Chromium compatibility fixtures compare native selection counts and actual
source/host computed colors, then rasterize an isolated serialization of the
fixture SVG into a canvas. These are regression probes, not pixel evidence for
the prepared execution. With a red source and blue receiving host, Chromium
148 renders inherited gradient and pattern children red; own children render
blue. A descriptive-only gradient still renders red, while a descriptive-only
pattern renders transparent. This differs from the structural SVG 2 clone
inheritance/profile expectation and is retained as an explicit compatibility
boundary. The fixtures fail if those observations change, requiring profile
review rather than automatically widening admission.

Source/host style, href and temporary own-child mutations (including restoration)
and pool closure revoke the live plan. Style evaluation, browser compatibility,
winning declarations, rendering and acquired resource bodies remain explicit
blockers. No source computed values are promoted into instance evidence.
This native structural contract changes no component behavior or theme defaults.

### Explicit clone-style browser compatibility profile

`cem_ml::svg_paint_clone_profile::SvgPaintCloneProfile` is the explicit native
compatibility contract for the six existing Chromium raster probes.
`for_browser(engine, version)` accepts only lowercase `chromium` with exact
version `148.0.7778.96`. There is no default, major-version fallback, browser
alias, caller-supplied behavior override or inference from a matching pixel.
Unsupported identities fail before profile use. This pin is independent of the
HTML base compatibility profile, even though both currently verify the same
browser build.

The profile applies only to the following isolated fixture shape: a 16-by-16 SVG
with two same-kind paint servers in `defs`, an opaque red source and opaque blue
receiving host referencing that source through local `href`. Gradient content
is one `stop` with `stop-color=currentColor`; pattern content is a 16-by-16 rect
with `fill=currentColor`, with pattern width/height 1 and viewBox `0 0 16 16`.
The host has either that own content, no children, or one SVG `title`. An outer
16-by-16 rect consumes the host. The fixture serializes the SVG as a data-URL
image, decodes it, draws it to a cleared 16-by-16 canvas and samples pixel (8,8).
Source and host computed `color` must respectively be opaque red and blue.

| Native probe | Expected sRGB RGBA pixel |
| --- | --- |
| OwnGradient | 0, 0, 255, 255 |
| InheritedGradient | 255, 0, 0, 255 |
| DescriptiveGradient | 255, 0, 0, 255 |
| OwnPattern | 0, 0, 255, 255 |
| InheritedPattern | 255, 0, 0, 255 |
| DescriptivePattern | 0, 0, 0, 0 |

`verify_probes` requires exactly one observation for each of the six named
cases, in any order. It checks both color contexts and every pixel channel.
Missing, duplicate, extra or mismatched observations fail. A previous successful
check does not latch acceptance of later reports. Work and storage are fixed
by the six-case input limit. A changed browser expectation requires explicit
profile revision and reviewed browser fixtures, not an automatic fallback.

These measurements do **not** establish a general rule that all cloned styles
use the source's computed values. Custom properties, explicit `inherit`,
selectors, indirect templates, gradients with multiple stops, animation, external
resources, nested content and other rendering environments remain outside this
profile. A transparent descriptive-pattern probe does not authorize pruning its
source dependencies. The SVG 2 structural plan remains unchanged, including its
prospective host inheritance and descriptive-content selection. Native source
ancestry and template paths continue to be retained alongside that plan.

A profile is configuration; the public observation report is compatibility data.
Verification returns no execution seal, does not authenticate the caller's
browser identity or fixture shape, and cannot discharge live DOM/CSS validation,
instance-style evaluation, winning-source or resource-acquisition requirements.
Browser preparation accepts an optional explicit `cloneProfile` with name
`chromium-148-solid-currentcolor-v1` and a synchronous `observe(index)`
callback. There is no implicit selection when it is absent. Native code requests
indices 0..5 in the table's order; each response is an array of exactly twelve
integer channels in 0..255: source RGBA, host RGBA, then sampled pixel RGBA.
This is a named scalar compatibility-report boundary, not an AST/evidence DTO.
Callers collect the isolated raster probes before capture; the callback only
delivers those measurements synchronously. Async callbacks are rejected.

After source preparation and before capture, the native session checks the
profile name and its recorded exact browser identity, verifies all six reports,
and retains the selected profile. Unknown names/identities, repeated selection,
out-of-order calls, exceptions, malformed arrays and mismatches close the
session. Browser quiet-state checks surround callback delivery, so mutations
including restoration cannot be hidden during confirmation. Both sync and async
capture use the same path. A native `svgCloneProfileConfirmed` diagnostic is
published only after successful capture; stale reads, cancellation and closure
revoke the owning preparation and clear that state.

Confirmation means only that this preparation selected the pinned configuration
and accepted its complete report. Reports are not rerun on every read and do not
authenticate the browser or prove arbitrary instance behavior. Existing live
document/base/style validation still governs reads. The browser compatibility
and style-evaluation blockers remain, and opted-in sessions additionally retain
`svg-paint-clone-execution-evidence-required`. Matching reports do not change the
SVG 2 structural plan or the historical resource-identity requirements.
No new UI behavior, public token or theme default is introduced.

### Bounded paint-instance color derivations

`cem_ml::svg_paint_colors::BoundedPaintColors::evaluate` evaluates a small
author-color subset over a retained `Arc<SvgPaintStylePlan>` and an explicitly
selected `SvgPaintCloneProfile`. It retains both. This is a native derivation
candidate, not a browser computed-style snapshot, full-cascade winner certificate
or rendering receipt. The six-probe compatibility catalog remains unchanged;
it does not automatically certify the additional value cases admitted here.

The initial content profile allows one direct SVG `stop` in a gradient or one
direct SVG `rect` in a pattern. Own content and one local same-kind template hop
are supported. Indirect links, unresolved selection, cross-kind templates,
multiple source children and nested elements fail atomically. A template pattern
whose receiving host contains only one SVG `title` records a suppressed host,
consistent with the pinned observation; the original source/template dependency
plans remain retained, and source entries are still evaluated independently.

Each admitted child requires an explicit `stop-color` or `fill` declaration
from a presentation attribute, retained inline style, or supported native
stylesheet rule as specified below. The shared native CSS tokenizer accepts a
single color token:
`red`, `blue`, `green`, `black`, `white`, `transparent`, three- or six-digit
hexadecimal sRGB, or `currentColor`. Keyword case and CSS comments follow that
tokenizer. Other named colors, color functions, variables, CSS-wide keywords,
URL paints, context paint and trailing tokens remain unsupported in this step.

A literal yields its RGBA value and tagged declaration origin. For
`currentColor`, evaluation walks the source child's SVG lexical ancestry until
it reaches a supported literal `color` declaration. A `color=currentColor`
declaration forwards that lookup to the parent, following the
[CSS Color contract](https://www.w3.org/TR/css-color/#valdef-color-currentcolor).
Every consulted ancestor and color declaration is retained nearest-first.
No implicit initial/system color is invented. The receiving host and source
child IDs remain distinct in each result; source ancestry supplies the pinned
Chromium derivation candidate for inherited content, while the separate SVG 2
instance-parent plan remains intact. Consumer graphics styles are never used as
a substitute for paint-server inheritance.

Before evaluation, the retained document is scanned conservatively for competing
author CSS or dynamic content. Unretained style elements, link/script/template/
animation elements, processing instructions and event-handler attributes reject
the entire evaluation, even when apparently unrelated. SVG inline styles and
eligible SVG style elements use the bounded retained-declaration paths below;
unretained or non-SVG owners fail. Browser preparation additionally requires
every admitted stylesheet to join its observed installation.
This deliberately narrow gate avoids silently selecting a presentation attribute
over an authored rule. User/UA styles, injected CSSOM, environment overrides, forced colors and
live execution still require separate evidence. Results preserve declaration
IDs and source maps as provenance; they do not claim a full-cascade winner.

The underlying plan bounds hosts at 256. Evaluation additionally caps ancestry
at 128 nodes and total work at one million units; document scans, attribute value
bytes, repeated lookups and inheritance steps consume the caller's work budget.
Failures return no partial result. Color values do not incorporate stop opacity,
geometry, compositing, resource acquisition or final pixels. Source plans and
suppressed-host dependencies remain subject to their original inventory's live
host/resolver validation.

Browser preparation accepts explicit `boundedPaintColors: true` only after a
complete `cloneProfile` confirmation. Unsupported evaluation fails capture
rather than silently omitting requested derivations. The native session builds
and retains the color result from the same style/content/inventory chain before
publishing preparation. Without this opt-in, existing structural preparation
retains its prior scope. Diagnostics expose only bounded `svgPaintColors` and
`svgPaintSuppressedHosts` counts; declaration/ancestor graphs remain native.

The WASM host compares each derived color to the exact source element's live
CSSOM `stop-color` or `fill`, using existing native-to-DOM correspondence.
Comparison uses the pinned Chromium serialization for supported opaque sRGB or
transparent values. It runs after capture and before/after reads. Missing owners,
observation failures, mismatches and invalidated DOM/style lifetimes close the
session and clear derivations. Source CSSOM comparison does not claim observation
of browser-private clone instances. Suppressed hosts have no color observation;
their native dependencies and complete document lifetime checks remain intact.

Chromium fixtures separately rasterize isolated SVG copies for explicit colors,
hexadecimal colors, source inheritance, currentColor forwarding, local overrides,
transparency, patterns and descriptive-pattern suppression. Restored source/host
colors, paint declarations, source replacement and pool closure revoke preparation;
unsupported inline CSS, variables and absent profile confirmation reject requested evaluation.
These bounded regression cases do not authorize general cascade or live rendered
resource closure. Color-parity, execution, winning-source, rendering and acquisition
blockers remain explicit. No theme tokens, component defaults or UI behavior change.

### Inline paint-color declarations and priority

The bounded evaluator now consumes `DocumentStylesheetOccurrence` entries from
the original inventory. Inline declarations are read from the shared importer's
retained `style-attribute` CSS tree; the evaluator does not split or reparse raw
declaration-list strings. Only SVG-owned, unnamespaced style attributes and the
properties `color`, `fill` and `stop-color` are accepted, using the existing
bounded color-value grammar. Comments do not count as declarations. Empty/comment-
only styles leave presentation-attribute selection unchanged.

Selection is local to each element and property. An admitted inline declaration
outranks its presentation attribute. Among inline duplicates, important
declarations outrank normal declarations; the last declaration of equal
importance wins. The native importer supplies decoded property names and the
`important` flag, including keyword case and intervening comments.
These rules follow the [CSS cascade](https://www.w3.org/TR/css-cascade-5/).
Inheritance begins only after local selection: a child's own normal color is
not displaced by an ancestor's important color. Inline `color=currentColor`
continues to forward to the parent, and paint `currentColor` uses that selected
color chain.

Each result carries `property` explicitly and a `PaintDeclarationWinner` with
a tagged `PaintDeclarationOrigin`: the original presentation attribute, or the
inline attribute plus retained CSS tree and declaration node. Winners carry an
important flag and zero-based declaration order excluding comments. Presentation
winners have no CSS-local declaration identity. Color inheritance records
corresponding tagged winners and the consulted ancestor chain. The original
inventory retains losing declarations and source frames/ranges, so no CSS node ID is confused with a
document node ID. Source CSSOM validation requests the result's property rather
than interpreting an inline `style` attribute as a property name.

There is an aggregate limit of 4096 inline declarations in addition to the
existing work and content bounds. Import diagnostics, unsupported properties,
nested rules, variables, CSS-wide keywords and unsupported/invalid values fail
the requested evaluation atomically, even if the offending declaration would
lose. This is a conservative supported-profile boundary, not an implementation
of browser invalid-declaration recovery. The evaluator does not discard a valid
but unsupported declaration and silently fall back to a presentation attribute.

Chromium fixtures check CSSOM and isolated raster agreement for normal/important
overrides, duplicate order, inline-only paint and currentColor inheritance. Style
and priority changes restored before read still revoke preparation. Stylesheet
rules and selector specificity are handled by the separate native extension
below. User/UA overrides, layers and general cascade winning-source evidence
remain outside these subsets; existing blockers remain.
The change enables retained author override provenance without adding component
styles, theme defaults or public tokens.

### Bounded stylesheet matching and paint cascade contract

Status: implemented in the native bounded evaluator and bound to installed
stylesheets during browser preparation and reads.
Existing evidence blockers remain. Native matching consumes retained CSS nodes
in `svg_paint_colors::cascade`; no new selector parser or browser DTO is added.

The first rule profile admits document-owned SVG `style` elements with
absent or CSS type, no title and no outer media, alongside the existing SVG
presentation attributes and inline declarations. Each style must have exactly
one successful `DocumentStylesheetOccurrence` from the same retained inventory.
HTML style elements, external sheets, imports, adopted sheets, nested rules,
conditional rules, layers, scopes and namespace declarations remain unsupported.
Reject these inputs before selecting winners, including apparently inactive or
nonmatching rules. Style order is retained document traversal order, followed
by rule and declaration order within each sheet; node IDs and source offsets
are identities/coordinates, not a replacement for that ordering. SVG style
rules have document scope; their containing gradient or defs is not an implicit
selector scope. Browser enablement needs the installation evidence below.

Consume the importer's retained `stylesheet`, `rule`, `selector-list`,
`selector`, `compound-selector` and `simple-selector` nodes. Require complete
selector analysis, top-level style rules, and explicit non-parent-dependent
specificity. Do not reparse selector strings, use JavaScript `matches`, or use
managed CSS selector emission: emitted scope/ownership selectors serve a
different contract. Decoded names, values, namespaces and source ranges must
come from the shared CSS import boundary.

The initial matching subset is a comma-separated list of single compounds made
from type/universal, ID and class selectors, for example `stop.hot`, `#accent`
and `stop, .accent`. Match exact SVG local names and exact unnamespaced ID/class
values; class membership splits on ASCII whitespace. Honor retained namespace
constraints (`*` matches any namespace, empty means no namespace); only SVG
elements are evaluation targets. Do not lowercase SVG names, IDs or classes or
borrow HTML quirks-mode matching. Repeated simple selectors all participate in
specificity. All branches must be supported, even when another branch matches.
Combinators, attribute selectors, nesting, pseudo-classes and pseudo-elements
reject this first profile. A valid selector with no matches is retained as a
nonmatch, while an unsupported selector is an error.

For a rule with several matching branches, its priority uses the greatest
specificity among those branches, not among all authored branches. Retain every
matching selector ID and the selected specificity; equal-specificity branches
do not duplicate declarations. This follows the
[selector specificity rules](https://www.w3.org/TR/selectors-4/#specificity-rules).
Matching evaluates the source element in the original document. It does not
match invented clone DOM or switch ancestry to the receiving host. Existing
flat-content limits and the explicitly selected clone profile still apply.

Within this unlayered author-only profile, compare local declarations in this
order: importance, inline attachment, specificity, then source order. Inline
attachment wins only at equal importance: an important stylesheet declaration
beats normal inline style, while important inline style beats important rules.
Presentation attributes are normal author hints with zero specificity preceding
all author rules; therefore a matching universal rule beats a presentation
attribute. These priorities follow the
[cascade sorting order](https://www.w3.org/TR/css-cascade-5/#cascade-sort).
Select the local winner before inheritance; an ancestor's importance or
specificity never competes with a child's declaration. Continue the existing
explicit `color`/`currentColor` inheritance walk. Missing explicit paint/color,
unsupported values, and unsupported properties fail as before. Validate every
CSS declaration, including nonmatching rules and losers, without implementing
browser error recovery. Only `color`, `fill` and `stop-color` and the current
bounded value grammar are admitted.

Winner provenance uses a native tagged origin: presentation attribute,
inline declaration, or stylesheet rule declaration. Each CSS origin retains its
original document occurrence, CSS tree Arc and declaration ID; a rule origin
also retains rule ID, matching selector IDs, specificity, importance and
sheet/rule/declaration ordinals. Document IDs and CSS-local IDs must remain
separate. `PaintDeclarationOrigin` replaces the attribute-only winner; redundant
`paint_declaration`/`color_declarations` fields are removed. `paint_winner` and
`color_winners` retain complete tagged origins. `document_source()` explicitly
returns a document attribute or style-element owner; `css()` and
`css_declaration()` access the optional CSS-local identity. Evaluated nonmatching
and losing rules remain retained through the same inventory, along with the
consulted element/color inheritance chain. Equal RGBA values with different winners must still preserve
different origins. Native plans own all evidence; browser summaries expose
counts only and cannot supply selector matches or winners.

The evaluator admits at most 256 style elements, 1024 rules, 4096 selector branches, 64 simple
selectors per compound and 4096 total CSS declarations across inline styles and
sheets. These are additional caps within the existing one-million-unit work
budget, 256-host and 128-ancestor bounds. Charge validation, value bytes,
selector comparisons, class scans and repeated inheritance lookups to the same
budget. Exhaustion, unsupported structure or missing provenance fails atomically
with no partial plan. Shared CSS import limits apply before these aggregate
limits; native bound fixtures spread declarations/rules across small sheets.
Reusing an immutable rule index must not bypass lifetime
validation or budget accounting for matching work.

Before publication, `RecordedOutputSession` joins every admitted native sheet
(including nonmatching/losing rules) to the existing whole-output stylesheet
observation capability. The join checks original document owner/source-node
identity, exact retained CSS tree identity, root installation metadata, matching
loaded-source key, nonzero installation identity, enabled selection and true
conditions. Missing observations or any mismatch reject capture atomically.
The existing sealed worker preparation retains stylesheet/source/DOM identities;
reads repeat the join within that validation boundary. An equal computed color
cannot excuse an inactive, replaced or tampered installation. Native plans and
all source/selector/declaration evidence remain in the owning realm.

The shared browser adapter compares actual CSSOM with parsed native-owned source,
preserves the verified owner namespace and absent/empty media distinction in
its comparison parser (including SVG default `media=all`), observes the exact
sheet on its corresponding DOM owner, and preserves existing
source-property CSSOM checks and clone-profile confirmation. Observed ID/class/
source mutations and replacement revoke, including DOM change-and-restore.
Persistent CSSOM edits or selection drift fail revalidation. CSSOM changes
restored before a read must use the exclusive-owner `host.change()` boundary,
which invalidates before the write; unreported CSSOM history is not inferred
from final equality. Rule insertion/deletion, declaration priority and disable-
restore fixtures exercise that boundary. Abort, disposal and pool closure
release the preparation and its color plans. Isolated SVG raster parity does
not establish browser-private clone identity or actual consumed resource bytes.

Native fixtures verify namespace/case/escape and
class matching; selector-list matched specificity; universal versus presentation,
inline versus rules at both importance levels, cross-sheet order and local
inheritance; retained source identities even for identical colors; rejected
selectors/rules/declarations; bounds and stale inventory. Native preparation
fixtures verify installed-source acceptance, inactive/missing installation
rejection and drift revocation. Chromium fixtures cover source CSSOM and isolated
raster parity for universal/compound/list specificity, importance versus inline,
cross-sheet order and local/source inheritance, plus DOM/CSSOM changes and
abort/pool lifetime in direct and worker-pool-owned preparation. Broader selectors, HTML ancestry, layers,
conditional/external rules and complete origin/environment handling require
separate profile extensions. Full-cascade winning-source, clone-execution,
rendering and resource-acquisition blockers remain.

CEM audit: authored semantic and theme ownership stays in retained sources;
matching never copies computed values into declarations or creates component
CSS/defaults. Explicit bounded variation preserves inherited consumer overrides
without claiming universal CSS support. No UI, public token or accessibility
behavior changes. Forced-color/user-origin and physical-device/AT acceptance
remain independent of these author-color derivations.

### Computed SVG URL bindings to acquired snapshots

`cem_ml::svg_resource_acquisition::SvgResourceRecord` is the first bounded native
byte-binding step. It joins current observed CSS URL references in SVG `fill`
and `stroke` slots to separately acquired, immutable SVG snapshots. It performs
no fetch and grants no URL access. A caller-provided acquisition snapshot is
not proof of the response the browser received or the resource it painted.
Native preparation lifetime integration and explicit browser snapshot ingress
are available. Browser response identity remains a separate next step.

Capture retains the original `Arc<SvgComputedEvidence>`, validates it against
its original `SvgComputedHost` before and after the join, and imports each
observed property/value through the shared native CSS importer. Only a single
static absolute HTTP(S) URL is admitted per slot, without credentials or a
fallback paint. Quoted/unquoted URLs and CSS escapes use imported component/
function nodes; no new CSS parser, regular expression or JavaScript AST is used.
Every captured slot must fit this profile. Relative/local-fragment references,
plain colors, lists, variables, other properties and importance reject rather
than silently dropping coverage. General CSS property selection remains open.

Each binding retains its computed-slot index, observation CSS tree/reference
node, full normalized URL and URL-serialized fragment. These coordinates belong
to the computed observation, distinct from the retained authored presentation
attribute and its CSS tree. This is not a winning-declaration attribution. The
fragment stays separate from the fragment-free request/body key; it is not yet
a decoded ID or a resolved SVG target. Multiple slots or fragments can share one
retained body without collapsing their individual references.

Native callers can opt into `SvgResourceRecord::validate_fragments` after
capture. It validates every binding atomically against the retained acquired
tree, indexing each distinct body once. The bounded profile shares local paint
traversal's literal ASCII identifier rule (letters, digits, underscore, hyphen,
dot and colon; nonempty, at most 4096 bytes). It performs no percent decoding,
Unicode normalization, SVG view-fragment or XPointer interpretation. Only a
unique unqualified `id` on an SVG-namespace `linearGradient`, `radialGradient`
or `pattern` is accepted; missing, duplicate, wrong-kind and unsupported
fragments reject the entire record. An explicit work budget, capped at one
million units, accounts for nodes, attribute values, child edges and lookups.

`fragment_target(index)` borrows the original binding and returns a target node
ID in that binding's retained body tree, preserving the computed slot, CSS
reference, original serialized fragment, source evidence, byte digest and
revision. There is no copied document or independently transferable validation
lease. Capture alone continues to permit unresolved targets. Validation checks
the original computed host before and after the operation; errors clear both
bindings and target results. Acquisition/delivery cancellation immediately hides
targets, and observation changes revoke them on record revalidation. Callers
must revalidate the record and its enclosing preparation before use. Preparation ingress can now opt into this validation as described below;
the independent acquisition API retains conservative blockers. Target identity establishes neither
template traversal, styles, nested resources, winner attribution, original
browser response identity nor consumption.

Native callers can additionally opt into `validate_paint_templates`. It validates
fragment targets and walks explicit paint-template references in the same
acquired tree. Each binding keeps its own ordered chain; shared bodies and
shared tail targets do not merge consumer slots or observation CSS references.
`paint_template_links(index)` borrows the original binding and link slice.
Each link retains owner, selected href attribute and target node coordinates;
the original href spelling, namespace, source frames and range remain in that
body's imported tree. A validated leaf returns an empty slice, whereas an absent
or revoked traversal returns no result.

The bounded traversal uses unqualified `href` before XLink `href`, including
when the unqualified value is empty or unsupported. Only literal ASCII
`#identifier` values are admitted; relative/absolute aliases, external URLs,
percent decoding and view fragments are excluded. Gradients may reference
either gradient kind; patterns may reference only patterns. Each referenced
ID must uniquely identify a compatible SVG-namespace element. XML `base`
attributes anywhere on a referencing element's ancestry reject, even for empty
or apparently equivalent bases; no URL/base equivalence is inferred.

A chain is limited to 64 edges. The shared work budget covers indexing, link
visits, href values and ancestry/attribute inspection, capped at one million
units across all bindings. Cycles, duplicate/missing IDs, incompatible kinds,
unsupported references and budget failures reject atomically and close the
record. Acquisition/delivery cancellation hides chains immediately; computed
observation drift revokes them on revalidation. Running fragment-only
validation replaces earlier traversal results, so no stale link plan survives
a narrower validation pass.

This traversal records candidate template edges only. It does not compute
inherited attributes, cloned content or styles, load nested resources or prove
browser consumption. All resource/transitive/origin blockers remain. The new
native traversal API also participates in preparation through the explicit
`validateSvgPaintTemplates: true` option. It requires snapshots and includes
fragment validation, whether or not `validateSvgFragments` is also enabled.
The WASM `enableSvgPaintTemplateValidation()` opt-in requires open snapshot
ingress and rejects repeated, closed or capture-started use. Native
`validate_svg_paint_templates` requires composed preparation with snapshots and
runs before capture publication. Read revalidates the owning observation and
preparation; acquisition cancellation, source/DOM/base/policy drift and worker
disposal revoke traversal with the enclosing record.

Only scalar `svgPaintTemplateBindings` and `svgPaintTemplateLinks` counts cross
the browser boundary. Binding counts distinguish validated leaves (zero links)
from fragment-only records (zero validated template bindings). Counts remain
zero before capture and after revocation. No chain, target coordinate or detached
proof is exported. Successful traversal does not remove the transitive-resource,
consumption, response-identity or origin/transaction blockers.

Pinned Chromium fixtures independently follow SVGAnimatedString href values in
the snapshot document and render bounded gradient/pattern chains into a canvas.
They verify gradient cross-kind links, pattern chains, XLink fallback,
unqualified href precedence and leaves. The fixture Blob URL is an isolated
rendering probe, never a replay route or proof of production response identity.
Further cases verify invalid targets/cycles/base/alias exclusions, sync/async
capture, copied ingress, combined fragment/traversal options, cancellation,
same-value DOM mutations, base/policy changes and worker disposal. These are
bounded compatibility checks, not a general SVG inheritance/style algorithm.

#### Acquired content, style provenance and nested snapshot graphs

`SvgResourceRecord::prepare_paint_content` adds bounded content selection,
attribute-source candidates, lexical style provenance and nested dependency
inventory to the same native acquisition owner. It requires an explicit
`SvgPaintCloneProfile` and the actual resolution capability with a stable cache
identity. The exact resolver owner and identity are retained and rechecked;
matching labels on a different resolver cannot bind nested resources.

The attribute candidates retain the first specified eligible source in each
chain. Gradient coordinate fields are taken only from the corresponding
gradient kind; shared gradient fields and pattern fields keep their original
owner/attribute IDs. Absent attributes have no fabricated source or default.
Each candidate now also retains a typed geometry result: finite numbers,
percentages, pixel lengths, coordinate-system/spread keywords, a four-number
`viewBox` or a bounded SVG attribute transform matrix. Negative radii/dimensions and viewBox extents are rejected in that
field's result. Missing fields remain absent; invalid or unsupported expressions
remain explicit errors. Percentages are not resolved against a fabricated
viewport or consumer bounding box. Relative units, CSS geometry winners and
binding geometry to a live consumer still require evaluation.

The plan separately records SVG 2 non-descriptive-child selection and the pinned
Chromium selection (direct stops for gradients, first element-containing pattern
content). This follows the distinction between the
[SVG 2 template model](https://www.w3.org/TR/SVG2/pservers.html#PaintServerTemplates)
and the previously measured Chromium descriptive-pattern behavior. Selected
nodes retain lexical source ancestry as well as prospective structural instance
parents. The receiving paint host and source server never become interchangeable
with the graphics consumer.

Stylesheets are imported by the existing shared retained-document CSS inventory.
Inline and presentation sources preserve their original document owners; rule
sources retain their CSS trees and coordinates. The existing bounded native
author-color cascade is reused for explicit stop/rectangle colors, literals,
`currentColor`, inline priority, importance and supported compound selectors.
Derivations retain selected declarations, selector specificity and every
currentColor source ancestor. Unsupported declarations/selectors/content retain
an explicit unresolved result; they are not treated as absent styling. Even
successful derivations are author candidates rather than all-origin browser
winner evidence. The acquired profile also derives explicit `opacity` and
`stop-opacity` winners through the same cascade, including inline and important
priority. Finite number/percentage values clamp to [0, 1]. Opacity declarations
on lexical ancestors remain separate nodes; they are not inherited or multiplied
into an invented pixel alpha. No default declaration is fabricated. Unsupported
opacity expressions remain unresolved. Filters, complex selectors, CSS transform
winners and general used-style/geometry evaluation remain open.
Pinned Chromium gradient raster probes distinguish these authored values from
pixel quantization: solid red at 50% stop opacity reads back as RGBA
`[253, 0, 0, 128]`, and at 25% as `[251, 0, 0, 64]`. Fixtures assert those exact
pixels while native derivations retain red and the separate opacity fraction.

#### Bounded transform, gradient and compositing calculations

`Matrix::parse_svg` accepts finite SVG attribute `matrix`, `translate`, `scale`,
`rotate` (including an explicit center), `skewX` and `skewY` functions. Lists
post-multiply in source order and are bounded to 4096 bytes and 64 functions.
Arguments require whitespace/comma separation; CSS units, functions, comments,
escapes, nonfinite values, malformed arity and singular 90-degree skews reject.
Matrix inversion rejects singular/nonfinite determinants. This is an attribute
profile, not a CSS transform parser or cascade winner.

`AcquiredPaintContent::resolve_gradient` borrows the original plan and takes an
explicit `GeometryContext`: viewport dimensions and the consumer's untransformed
bounding box in local user space. It resolves linear/radial coordinate candidates
and returns their paint-to-user matrix. Object-bounding-box coordinates compose
the box mapping before the authored transform. User-space percentages use viewport
axes; radial percentages use the normalized viewport diagonal. Missing gradient
values use specified initial values with `GeometryOrigin::Initial`; focal defaults
record `DerivedFrom` center fields. Authored fields keep their owner/attribute IDs.
No default gets a fabricated source location. Zero viewports, zero bounding-box
dimensions in box units, zero radial radius and singular matrices report disabled
geometry. Radial focal limiting, box-unit pixel lengths, pattern tile resolution
and competing CSS remain outside the profile; stylesheet-bearing bodies reject.

`ViewBoxMapping::resolve` implements `none` and all nine alignments with `meet` or
`slice`, retaining a disabled result for zero extents and rejecting negative or
nonfinite rectangles. Its numeric inputs are explicit assumptions. These mappings
follow the [SVG coordinate-system algorithms](https://www.w3.org/TR/SVG2/coords.html);
they do not observe browser layout or prove which viewport/bounding box applied.

`composite_pattern_sample` borrows the acquired plan and takes an explicit set of
covered rectangle source IDs. Nested `g`/`rect` content retains original paint
order regardless of coverage order. Children composite source-over into a group
before that group's opacity is applied; leaf opacity applies to its own source.
The result retains contributors and continuous premultiplied RGBA values. Empty
coverage is transparent; duplicate/unknown coverage and unsupported effects reject.
Effect checks include lexical ancestors outside the selected subtree, so inherited
fill opacity, stroke, visibility and color-interpolation cannot silently become
default behavior.
Descriptive subtrees never contribute paint. Explicit author colors and opacity
winners remain native. The calculation is bounded by the retained content limits
and an additional 8192-unit compositing work budget. Coverage is not a hit-test;
filters, masks, clips, blending modes, general shapes and CSS geometry need further
profiles. This ordering follows [source-over and group compositing](https://www.w3.org/TR/compositing-1/).

The native `svg-geometry-parity-emit` example exports only fixture source and
calculated scalars at an explicit JSON test boundary. Chromium probes compare
transform/viewBox matrices, transformed gradient pixels and overlapping group
samples. Continuous compositing values allow one 8-bit channel step for raster
rounding, which cannot conceal incorrect group-opacity flattening. This export
is not a runtime AST handoff, layout seal or rendering-consumption receipt.
Preparation binding must still establish the consumer, geometry context, coverage
and mutation lifetime before these calculations can contribute execution evidence.

The static acquired-body profile rejects scripts, animation, foreign content,
event handlers, processing instructions and XML base changes throughout the
body. Traversal and content work share a one-million-unit cap split between the
two phases; selected subtrees are limited to 4096 nodes per binding, ancestry to
128 levels, and stylesheets to 256 per body. Exhaustion revokes the whole record.
Narrower fragment/traversal validation clears prior content and nested plans.

Direct resource slots and CSS URL/import references retain the original body
URL, source coordinates and resolution policy evidence. All global stylesheet
candidates are included conservatively; local inline/presentation declarations
are restricted to selected content, template hosts and their lexical ancestors.
This inventory can overapproximate resources and is never a fetch request.

`bind_nested_snapshots` joins explicit response inputs to these references,
then recursively inventories imported CSS and SVG bodies. It retains each
reference edge and parses each distinct body only once, so CSS import cycles
terminate without merging their source coordinates. SVG processing uses the
same shared import boundary and static-context exclusions. Raster/font bodies
remain opaque bytes, not decoded images or validated fonts.

The graph accepts at most 256 direct, fragment-free, nonredirected HTTP(S)
snapshots and 4 MiB including metadata; it retains at most 4096 edges.
Missing, duplicate, unused or conflicting snapshots reject atomically.
Resolution failures, MIME hints and CSS-import MIME constraints are enforced.
Integrity-bearing mappings use the shared strict SHA-256/384/512 verifier with
strongest-algorithm selection. Every edge's expectation must pass against the
exact retained bytes, including edges reusing an earlier body or the root body.
Identical URL/expectation pairs share verification; conflicting expectations
cannot reuse another edge's success. Malformed/unsupported metadata and mismatch
revoke the whole record. Metadata is bounded to 4096 bytes per expectation and
distinct verifications to 64 MiB of hashing input.
Supported entity types are CSS, SVG, PNG, JPEG, GIF, WebP, AVIF, WOFF and WOFF2;
other types, redirects and encoded-wire metadata require later profiles.
Each body retains exact bytes, BLAKE3 digest, revision and any native imported
tree. Root-body references reuse the original acquisition. SVG fragment edges
now retain their edge index, native target tree and unique literal-ID node.
Missing/duplicate IDs and encoded/view fragment forms reject atomically; target
indexing shares the graph's native work bound. Non-SVG fragments remain URLs
without SVG target proof. Literal identity does not prove operation-specific
target compatibility, nested `use` expansion, paint recursion, resource use or
full transitive rendering closure.

Browser preparation opts in with `prepareSvgPaintContent: true`, explicit
`svgSnapshots` and the existing confirmed `cloneProfile` report. Content
preparation includes fragment and template validation. Optional
`svgNestedSnapshots: { responses, signal }` supplies the recursive snapshots.
Both ingresses copy bytes before installation. Unopened, repeated, late or
closed ingress rejects; either acquisition signal, DOM/base/policy changes,
worker disposal or native validation failure revokes the enclosing session.

Only scalar counts cross WASM: acquired content bindings/nodes/dependency
occurrences, bounded color derivations, inherited bindings, compatibility
differences, unique nested bodies, typed geometry values, explicit opacity
winners and nested literal fragment targets. Native plans, trees, source coordinates,
resolver ownership and byte digests remain native. Browser fixtures compare
supported source-color/content behavior with isolated raster probes, and test
recursive graph binding and revocation. Those probes and snapshots do not
establish original-response identity, consumption, USER/UA origin admission or
atomic alias installation.

The production origin-provider dependency remains external: this repository
has no maintained instrumented Chromium integration, and the existing CDP and
debugger probes fail the required all-origin observation and all-writer
exclusion guarantees. Production alias installation and transaction admission
remain disabled. Runtime-generated URLs, unsupported style contexts and broader
CSS/SVG coverage remain explicit blockers rather than inferred closed sets.

Acquisition inputs contain requested/final URL, revision, MIME and SVG entity
bytes. Encoded wire bytes and HTTP response headers are outside this profile.
The initial profile requires equal canonical requested/final HTTP(S) URLs,
fragment-free metadata, a nonempty revision and `image/svg+xml` without MIME
parameters. Duplicate, missing or unused responses reject atomically. Redirects
and other media types need later profiles. The native record computes a BLAKE3
digest from the retained bytes and imports those exact bytes through the shared
XML importer, requiring an SVG-namespace document root. Its imported tree and
original bytes remain together. Capture does not execute scripts, resolve fragment
targets or traverse nested dependencies. Parsing a body does not establish that
its referenced paint server exists or that a browser consumed it.

Limits are 256 responses, four MiB total body/metadata bytes, 4096-byte URLs and
revisions, and the existing computed-slot/CSS/XML import bounds. Cancellation
is checked before, throughout and after capture. Reads require explicit native
validation of computed owner/epoch/environment/values and the cancellation
signal. Failure or explicit close permanently revokes the record and clears its
body bindings; restored values cannot revive it. Retained getters are for
inspection. The acquisition owner must abort when its snapshot lifetime ends,
including changes at an unchanged URL; the record does not poll remote content.
The enclosing document/base/stylesheet/policy preparation still needs validation.

Native fixtures cover exact bytes/digest/source identity, shared bodies with
distinct fragments, CSS escapes, invalid or missing acquisition inputs, profile
rejections, byte/count bounds, foreign/stale computed hosts, restored values,
body release, pre-abort and cancellation during final observation. Browser
response identity, actual consumption, fragment-target resolution, transitive
SVG dependencies, winning sources and general runtime closure remain explicit
blockers. This partial native binding does not remove existing execution blockers
or authorize edits/transactions.

The native `RecordedOutputSession::record_svg_resources` method binds these
snapshots after computed SVG capture and before enclosing preparation capture.
It accepts raw acquisition inputs and a separate acquisition-owner cancellation
signal, retains the same computed evidence, and rejects repeated/out-of-order
calls. The record never escapes the session. `validate_svg` revalidates both
computed evidence and resource bindings; callers must bracket enclosing
capture/read with SVG validation, as with existing image/computed observations.
Every staged operation checks acquisition cancellation before and after work.
Cancellation hides diagnostic binding counts immediately; the next mutable
operation closes preparation and releases bindings. Explicit close, computed
validation failure, and enclosing DOM/base/stylesheet failure also close the
session. Closing preparation does not abort a caller's shared acquisition owner.
Only a captured live preparation exposes a binding count. All resource blockers
remain conservative, including the conservative preparation-binding blocker until response-bound
preparation is established. There is no automatic network acquisition or
response-identity claim.

Browser preparation accepts `svgSnapshots: { responses, signal }` explicitly
on both synchronous and asynchronous capture. Each response supplies
`requestedUrl`, `finalUrl`, `contentType`, `revision`, and `Uint8Array` entity
`bytes`. The required signal belongs to acquisition; aborting it disposes the
preparation without aborting other owners. Preparation cancellation, worker
closure, and DOM mutation also revoke the binding. The host pins this signal
and copies inputs into WASM during construction, before installation callbacks;
subsequent edits to caller buffers or response arrays cannot alter native bytes.
Callers must abort acquisition to indicate snapshot invalidation.

WASM `beginSvgSnapshots` / `addSvgSnapshot` accept only bounded raw source
inputs (256 responses, four MiB including metadata), before capture begins.
Repeated begin, unopened add, late ingress, and bound failures close the session.
Explicit empty ingress fails native binding capture; omitted ingress preserves
existing computed-only capture. Native import and URL matching run after SVG
computed capture, and validation brackets the enclosing preparation read.
Browser preparation additionally accepts `validateSvgFragments: true`, requiring
explicit snapshots. It opens snapshot ingress, invokes the WASM
`enableSvgFragmentValidation()` opt-in before capture, and validates targets after
native snapshot binding. Unopened, repeated or closed opt-in rejects and closes
the session. The native preparation method `validate_svg_fragments` only accepts
the composed stage with acquired resources. Capture publishes target counts
only after the enclosing DOM/base/policy and worker preparation checks pass.
Read revalidates computed selection and resource lifetimes; source/DOM/base/policy
changes, worker disposal and acquisition cancellation revoke the enclosing
record, including target evidence. Snapshot input bytes are copied at ingress;
callers abort the acquisition signal if the external snapshot lifetime changes.

The scalar summary `svgFragmentTargets` counts validated bindings, not distinct
target nodes. No target tree, node ID or transferable proof crosses into JS.
Only complete live validation removes `svg-resource-fragment-target-required`
from this preparation's blockers. Snapshot acquisition without opt-in still
reports zero targets. Browser response identity, consumption, transitive
dependencies, winner and transaction admission remain separate requirements.

Chromium fixtures compare native classification against an independently parsed
SVG document over the same bytes, exercising gradient/pattern identity, missing
and wrong-kind/namespace targets. Duplicate IDs and percent-encoded fragments
are explicit conservative exclusions: browser DOM lookup can select the first
duplicate and a decoded identifier, while this native profile rejects both.
These checks establish snapshot target identity compatibility, not external
fill/stroke rendering or browser response identity. Additional fixtures verify
sync/async capture, immutable ingress, cancellation, same-value DOM mutation,
base/policy changes and worker disposal.

The snapshot summary includes `svgResourceBindings`; no tree, digest, byte body,
or evidence handle crosses back to JavaScript. This boundary does not fetch or
replay bytes and cannot attest that supplied bytes match a browser response.
Chromium fixtures intentionally assert binding and lifetime behavior separately
from paint pixels and retain all consumption/response-identity blockers.

The native delivery-evidence prerequisite is `SvgDeliveryCapture`. A trusted
transport adapter may supply `SvgDeliveryResponse` entity bytes only while it
withholds the response pending native acceptance. The ledger admits at most
256 canonical fragment-free HTTP(S) responses and four MiB including metadata;
it rejects credentials, redirects, empty bodies and non-SVG MIME. It accepts
no caller-provided digest or revision as evidence. Duplicate request URLs reject
even for identical bytes because request multiplicity is unresolved. Any ingress
error or explicit close irreversibly clears the ledger.

`SvgResourceRecord::bind_delivery` consumes the ledger exactly once, validates
its original computed owner before and after joining, and requires exact byte
and MIME equality with every retained acquired body. Missing, extra or changed
receipts revoke the resource record. Shared fragment/slot references still point
to the same imported native body; delivery does not re-import it or replace its
source provenance. Binding retains the transport cancellation signal and
rechecks it around subsequent validation. Cancellation hides delivered-binding
counts immediately and the next validation releases resource bindings. Neither
closing the record nor consuming the ledger aborts a shared transport owner.

The asynchronous browser host connects this ledger through
`svgDelivery: 'service-worker'`, together with explicit `svgSnapshots`.
It requires an already controlling application-owned shared delivery worker;
registration and scope remain application responsibilities. Raster `imageDelivery`
and SVG delivery cannot be combined in one preparation. The SVG profile admits
only same-origin, allowlisted `image`-destination GET requests, direct status-200
responses with bare SVG MIME, bounded bytes and no redirects. URLs come from the
pinned snapshot ingress. It does not initiate fetches or rewrite authored URLs.

The worker retains each exact decoded response body, transfers an independent
copy over its private client-bound channel, and waits for synchronous native
ledger acceptance before returning that body to the browser. Duplicate requests
fail rather than merging ambiguous receipts. Preparation waits for composed SVG
image loads and transport sealing, then joins receipts to native snapshots and
revalidates before publication. `svgDeliveredBindings` reports the count only.
Missing receipts, changed bytes, receipt rejection, controller changes, late
requests, acquisition cancellation and preparation disposal fail closed.

This establishes same-session SVG image **response delivery**, not the response
chosen by an independently observed fill/stroke reference. Cached resources can
bypass the worker; absent receipts therefore reject. A delivered SVG image at
the same URL does not prove a paint reference consumed it. No caller can turn a
snapshot or diagnostic count into a transferable seal. **All existing paint
response-identity and consumption blockers remain present.** A cache-isolated,
selection-bound replay profile and Chromium paint compatibility evidence are
still required before those blockers can be removed.

The pinned compatibility profile `chromium-148-external-paint-replay-v1` is
verified against Chromium `148.0.7778.96` by
`packages/cem-ml-cli-npm/scripts/verify-svg-replay.mjs`. Twelve cases cover gradients
and patterns, each as a local control, a fresh external resource, a resource
warmed by an image consumer, and a fresh alias alongside a warmed source. Fresh and replay cases also run
without fallback paint, matching the bounded native URL profile.
External cases contain **no local paint definitions**. Local/fresh/alias paint
is green, warmed-source paint is magenta, and the standalone SVG image is blue.
The fallback-control cases use red; cases without fallback use the white
page background to distinguish missing paint from successful outcomes. Tests sample actual live-document screenshots rather than
serialize the document into an SVG image, and assert exact request URLs,
`image` fetch destinations, computed fragment references and RGBA pixels.

Chromium renders both external gradients and patterns in this bounded profile.
Warming an SVG image allows its response to be reused by an external paint
reference without another network request. A fresh alias selects its separately
served body; retargeting back to the warmed source restores magenta without a
new request. These four retargeting checks establish that the sampled paint
tracks the selected URL, rather than the visible SVG image or a local definition.
The fixtures use fixed responses at isolated server URLs; they do not implement
production source-to-alias rewriting or authenticate an arbitrary transport.

The shared browser gate records these observations under `svgReplayCompatibility`
in `dist/reports/cem-ml-platform-parity.browser.json`, with `executionSeal: false`.
A browser-version change fails the profile until its evidence is reviewed. A
separate preparation fixture rejects `svgDelivery: 'service-worker-replay'`:
compatibility alone cannot admit replay. Production replay still requires native
retained winning-source/fragment ownership, bounded alias generation and
rewriting, receipt/selection binding, and revocation on source, consumer or
transport changes. Existing response-identity/consumption blockers stay intact.
No broader CSS, filters, markers, masks, inheritance, cross-origin, animation or
physical-device compatibility is inferred from these gradient/pattern probes.

`cem_ml::svg_replay::SvgReplayPlan` now records bounded native **authored replay
candidates** before browser preparation. Inputs are retained SVG presentation
attributes, the effective base and document URL, an owner-supplied lowercase
32-hex nonce, and cancellation. Native inspection uses the original imported
CSS tree, not computed-observation CSS. Each route retains the original consumer
owner/attribute, CSS reference node (with its source frames/ranges), resolved
source URL, fragment-free source request, opaque URL-serialized fragment,
fragment-free alias request, alias URL with fragment, and a proposed whole
attribute replacement. The original source tree is never modified.

Only a single static `fill`/`stroke` URL with a nonempty external fragment is
admitted. Relative sources resolve against the retained base string; document
and source URLs must be credential-free HTTP(S), with same-origin sources.
Local references, same-document URLs, fallback paints, unsupported properties,
missing fragments, cross-origin sources and reserved replay paths reject.
Per-element/ancestor `xml:base` rejects until its resolution evidence is retained.
The plan bounds ancestry to 256, occurrences to 256, URLs and replacement values
to 4096 bytes, and retained route strings to four MiB. All sources must belong
to the same native document with distinct attribute identities.

Every occurrence receives a separate `/__cem_svg_delivery/<nonce>/<index>`
alias, even when several consumers share a source body. Fragments stay attached
to their individual aliases without being decoded or resolved as IDs. Proposed
CSS values quote and escape the serialized alias; shared-import round trips
verify opaque fragments cannot change CSS structure. Replacement coordinates
are whole decoded attributes, never source-file patch ranges. Nonce validation
checks shape only: the future preparation/transport owner must supply fresh
nonces and enforce their lifetime; construction alone proves no freshness.

Validation requires the same ordered retained document/CSS owners and source
identities. Cancellation, explicit close or mismatched validation irreversibly
revokes the plan; cancelled plans expose no routes, and close/failing validation
releases retained routes. Stored base/document strings are provenance, not live
base-policy evidence. Cascade winners, live base/policy, source-to-alias rewrite
admission, fragment target resolution, browser receipt matching and consumption
remain blockers. Stylesheets can override these candidate attributes: this API
must not rewrite them or substitute a computed winner without separate proof.
No WASM ingress, browser alias installation or transaction admission is added by
this native planning step.

Recorded preparation can now opt into `svgReplayPlan: { nonce }` through the
browser host. WASM passes only that scalar nonce to the native session;
`svgReplayCandidates` reports a diagnostic count, never routes or evidence DTOs.
`SvgReplayAuthorPlan` retains presentation-origin `PaintDeclarationWinner`
identities under a closed static author profile: an explicit inert HTML/SVG
allowlist, at most 10,000 retained nodes, and no stylesheet elements, inline
styles, scripts, event handlers, custom elements, animation, or SVG `use`.
Even unrelated/nonmatching styles reject. This proves the direct presentation
attribute wins within this bounded author universe; it does not prove precedence
over user/UA/extension origins or broader browser behavior.

Capture constructs the plan from the same retained presentation inventory and
sealed HTML base evidence used by preparation. Original source URLs must agree
with separately imported computed URLs; equality is a consistency check, never
the source of author-winner authority. The worker must validate the live DOM,
base/policy and stylesheet state before publishing the plan. Browser capture and
reads also validate the live computed host. Failed capture/read, changed DOM or
base/policy, computed disagreement, cancellation and disposal revoke the owning
session and its candidates. Source identity checks continue on native reads.

This preparation-only option never installs replacements, claims nonce freshness,
creates transport routes, or authorizes a transaction. Browser-origin proof,
controlled alias installation, original-response/alias receipt matching, fragment
targets, nested dependencies and actual consumption remain explicit obligations.
The existing full winning-source blocker remains conservative for browser origins.

CEM audit: authored source and theme ownership remain separate from observations
and acquired bytes. No copied computed value becomes a declaration, no component
behavior/default/public token changes, and no general forced-color or device/AT
acceptance is claimed. Broader selection and transport support must preserve the
same native ownership and bounded variation.

### Browser-origin admission for bounded SVG replay

This is the accepted admission contract, not an implemented browser capability.
The profile is named `svg-replay-presentation-origin-v1`; its initial compatibility
baseline is Chromium `148.0.7778.96`. The existing web host remains
**preparation-only**. Neither this profile name nor the existing
`svgReplayPlan: { nonce }` option grants replay admission. Complete native session integration and verify a browser-owned provider before
enabling writes.

SVG presentation attributes participate in the author cascade with zero
specificity ([SVG styling](https://www.w3.org/TR/SVG/styling.html#PresentationAttributes)).
Other origins and animation/transition precedence can change the result
([CSS cascade](https://www.w3.org/TR/css-cascade-5/#cascade-sort)).
`getComputedStyle` exposes resolved values rather than a declaration-origin
record ([CSSOM](https://www.w3.org/TR/cssom-1/#dom-window-getcomputedstyle)).
Consequently, CEM does not infer origin from matching URLs, a screenshot, empty
`document.styleSheets`, a browser version string, or exclusive DOM ownership.
An overriding declaration can resolve to the very same URL as the presentation
attribute and still retain the original URL after that attribute is rewritten.
That same-value override is a mandatory rejection fixture.

**Admission evidence.** The only admitted source of browser-origin evidence is a
privileged browser-owned provider bound to the execution host. It must establish
its provider identity and revision outside page-controlled JavaScript and report
complete winner provenance for each retained consumer/property. Native code owns
an opaque, non-serializable lease joining that provider to the prepared document,
worker generation, native author plan, base/policy evidence and cancellation.
An application callback returning `true`, a supplied origin label, JSON report,
profile string, or synthetic fixture provider cannot mint a production lease.
The public WASM/browser options gain no `trusted`, `ignoreUserStyles`, or
`allowUnprovenOrigins` escape hatch.

For every route, the provider must identify the exact live element, property and
winning presentation attribute, mapped through the existing native DOM slot
correspondence to the original retained source. It must account for author,
user, UA and injected declarations, animations and transitions; extension rules
are classified by their actual cascade origin, not treated as a new CSS origin.
Unknown, incomplete, inaccessible or ambiguous provenance rejects admission.
A different winner rejects even when its value equals the expected source URL.
Inherited or stylesheet winners are outside v1. Existing native URL, source,
node-count, stylesheet and static-author restrictions remain prerequisites.
The provider may inspect engine internals, but must not hand a CSS AST, copied
rule tree or caller-authored evidence DTO to the native transformation runtime.
A scalar correspondence predicate is useful only inside the authenticated lease.

**Environment boundary.** Pin the browser build and provider protocol revision
independently; the twelve paint compatibility probes remain observations with
`executionSeal: false`. A version/protocol change requires fresh review and
fixtures. Require a live rendered document and a stable screen/media context.
V1 rejects active forced-colors mode and active animation/transition effects;
unsupported media, print, navigation or rendering contexts reject. Track the
existing theme/contrast/reduced-motion observations as lease revisions, including
same-value transitions. Forced color adjustment can affect used colors
([CSS color adjustment](https://www.w3.org/TR/css-color-adjust-1/#forced-colors-properties));
a computed URL alone cannot attest to the resulting paint. Do not set
`forced-color-adjust: none`, suppress user styles, turn off extensions, disable
accessibility preferences, or add stronger declarations to obtain admission.
Admission rejection leaves normal authored rendering available without a replay guarantee.
This profile establishes selection provenance, not general visual or AT parity.

**Lifetime and ordering.** The required native states are candidate, admitted,
installing, replay-bound and revoked. They are a protocol design, not current
session states. Only the provider can establish the origin portion of admitted;
base/policy, source and computed validation must also succeed in the same lease.
Validate provider/document/generation/revision both before and after capture and
each read. Every origin-affecting change must invalidate the lease before it can
be used again, including A-to-B-to-A changes that preserve the observed URL.
Document replacement, worker replacement, source/base/policy changes, environment
changes, provider disconnect, cancellation and disposal revoke irreversibly.
Recapture creates a new lease; no old receipt or lease can be transferred.

The future installer must acquire a provider-controlled mutation interval so
there is no unchecked gap between validation and the authorized attribute writes.
During installing, reads and external writes are unavailable. Only the native
plan's exact consumer attributes and proposed alias values may change. Retain the
original source identity; the installed alias is a projection, not a replacement
source declaration. Revalidate the same presentation winner, the alias URL and
all unchanged base/policy/origin conditions before publishing replay-bound.
Unexpected mutation or loss of the interval revokes the whole attempt and its
routes; no partially installed result may be published. A later installer design
must specify safe rollback/disposal. A provider that offers only periodic polling
or cannot order invalidation against this interval is insufficient for admission.

**Authority remains separate.** A valid origin lease discharges only
`svg-replay-browser-origin-proof-required` for its own bounded routes. The broader
`svg-computed-winning-source-required` diagnostic can be removed only when every
covered slot has the corresponding complete selection proof; unhandled slots
retain it. Alias installation, fresh nonce/route ownership, response identity,
fragment targets, transitive dependencies, consumption and transaction admission
still require their own evidence. No receipt, candidate count, screenshot or
origin lease substitutes for another obligation. Current runtime blockers stay
unchanged until this protocol is implemented and verified.

The following acceptance matrix is normative for implementation; it records
required fixtures, not test results:

| Case | Required outcome |
| --- | --- |
| Ordinary web host, matching computed URL, closed author plan | Candidate only; origin blocker retained |
| Trusted provider, exact attribute winner, stable pinned environment | Origin lease only; no alias or consumption authority |
| UA/user/injected winner with the same source URL | Reject despite identical computed output |
| Author override, inherited winner, active animation/transition | Reject v1 admission |
| Missing origin coverage, forged provider/report or wrong document/generation | Reject; no production lease |
| Unsupported browser/provider revision or forced colors already active | Reject without changing user settings |
| Origin/media/policy A-to-B-to-A, including between validation and write | Revoke; no replay-bound publication |
| Provider disconnect, cancellation, navigation or worker replacement | Revoke and release the attempt's routes |
| Exact alias write within the interval, same attribute still wins | Advance only after postvalidation and separate delivery requirements |
| Unexpected write, partial install or mismatched alias observation | Revoke; no published partial result |
| Fresh recapture after revocation | New lease; old leases/receipts remain invalid |

The native foundation is implemented in `cem_ml::svg_replay::admission`.
`AdmissionOwner` binds one exact author-plan identity, retained document and
nonzero worker generation;
`AdmissionLease` consumes the author plan and holds a weak owner reference.
Candidate, admitted and revoked states are implemented. Owners invalidate
terminally before preparation changes, and dropping an owner invalidates its
leases. Cancellation is visible immediately to state inspection; validation,
explicit revocation or drop releases the retained plan. A new owner is required
for recapture. Numeric generation equality cannot transfer a provider stamp
between owners, even when they share a document.

The private provider protocol checks its opaque identity, bound owner, revision,
pinned engine/build/protocol and static screen/forced-color status before and
after every capture/read. Every route requires the original retained document,
CSS, element and attribute identities, an explicit presentation winner and a
matching natively parsed computed URL. Preparation-current checks bracket the
observations; a failure revokes and releases the plan. No public admission method,
production provider implementation, WASM ingress or session integration exists.
The only provider implementation is compiled under `cfg(test)`; it exercises the
protocol without granting production authority. A compile-fail fixture protects
the private admission boundary. This standalone foundation does not yet join
actual HTML base/policy seals or authenticate a browser adapter.

Native fixtures cover successful admission/read, out-of-order calls, foreign
sources/owners/generations, same-value wrong winners, unsupported contexts,
provider replacement/disconnect/revision changes, computed disagreement,
pre/postvalidation preparation failures, observation races, cancellation,
disposal and fresh recapture. Installation intervals, alias writes and
replay-bound publication remain future work, along with real delivery receipts.
The privileged CDP prototype below tests actual browser-origin injection and
same-value counterexamples. It does not satisfy the admission contract; a provider
with stronger engine guarantees remains the gate ahead of session integration
and alias installation. Compatibility reports remain non-authoritative.

#### Privileged CDP provider feasibility

`scripts/verify-svg-origin-provider.mjs` prototypes a Node-owned DevTools session,
separate from page JavaScript, against Chromium `148.0.7778.96`. It holds a live
CDP node/backend identity, reads presentation/matched/computed styles and tracks
CSS/DOM notification revisions. It is a test-only diagnostic observer, never a
native `OriginProvider` implementation. No CDP rule objects enter native AST or
lease ingress. All reports contain `executionSeal: false` and
`productionAdmission: false`, including baseline cases with no competing rules.
The full browser gate saves the report as `svgOriginProvider` in
`dist/reports/cem-ml-platform-parity.browser.json`; the helper is an explicit Nx
input so changes invalidate the gate's cache.

Fourteen probes exercise presentation fill/stroke, visible UA rules, real
extension AUTHOR and USER overrides, USER add/remove restoration, initial and
restored forced colors, spoofed page media observation, DOM restoration,
observation/write interleaving, navigation, explicit closure and transport loss.
The extension runs only in a temporary persistent test context with loopback
host permissions; the context and extension directory are removed afterward.
It uses `chrome.scripting.insertCSS` with explicit AUTHOR/USER origins, rather
than relabeling author CSS as user CSS. The protocol's rule-origin categories and
matched-style surface are documented in the
[DevTools CSS domain](https://chromedevtools.github.io/devtools-protocol/tot/CSS/).
Actual pinned-browser observations determine the following findings:

| Observation | Admission implication |
| --- | --- |
| Presentation values and computed URLs are visible; UA rules are listed on the SVG root | Useful correspondence data, not a complete winning-source proof |
| An extension AUTHOR rule reports `injected`; its USER rule reports `user-agent` | The CDP label cannot be treated as the actual CSS cascade origin |
| Either important override can have exactly the presentation URL; changing the attribute to an alias leaves the computed original URL | Computed equality does not establish presentation ownership |
| A same-value USER insert/remove cycle changes the matched rule list but leaves the subscribed CSS/DOM revision unchanged | This event stream cannot maintain the native lease's all-origin revision |
| A USER write succeeds between the probe read and the CDP attribute write | Observation followed by a write establishes no mutation-exclusion interval |
| Initial forced colors is observable, but page code can spoof `matchMedia` even when called through CDP Runtime.evaluate | Privileged transport does not authenticate page-derived environment data |
| DOM A-to-B-to-A increments the observed revision; navigation and detachment reject stale reads | These covered paths do not repair the missing origin/environment guarantees |

The prototype therefore retains explicit completeness, revision, mutation-interval
and environment-provenance blockers. The USER-origin event assertion deliberately
fails if a later browser begins reporting that transition, requiring review rather
than silently assuming that one improvement proves complete admission. The tests
observe a real UA rule but do not inject a UA paint override or prove complete UA
coverage. The interleaving probe disproves this observer's read/write sequence as
an exclusive interval; it does not claim that every possible privileged browser
integration is incapable of exclusion.

Five additional probes in `scripts/verify-svg-engine-primitives.mjs` test engine
primitives through independent protocol sessions. An isolated execution context
reads forced-colors correctly despite a main-world `matchMedia` replacement,
observes restoration and becomes unusable after navigation. This is a useful
bounded observation improvement, not an authenticated environment lease or
complete environment revision stream. While a debugger statement keeps the page
paused, a second session successfully changes both the SVG attribute and an
inspector stylesheet; the computed paint then reflects the competing rule.
The fixture asserts that no debugger-resumed event occurred before these writes.
Thus debugger pause cannot stand in for the required mutation interval.

These results are saved under `svgOriginProvider.enginePrimitives`, separately
from the fourteen CDP origin probes. The report retains false environment-lease,
mutation-interval, execution-seal and production-admission flags. No runtime
admission blocker is removed. Fixture pause is bounded, resumed during cleanup,
and confined to a disposable test context; it is not a production strategy for
freezing user interaction or accessibility preferences.

#### Required engine hooks and integration boundary

The following is an implementation specification, not an existing Chromium API.
Source inspection uses the pinned `148.0.7778.96` tag. In
[InspectorCSSAgent::DetectOrigin](https://chromium.googlesource.com/chromium/src/+/refs/tags/148.0.7778.96/third_party/blink/renderer/core/inspector/inspector_css_agent.cc),
the inspector label is derived from stylesheet ownership/metadata, which explains
why it is unsuitable as actual cascade-origin evidence. In
[StyleCascade](https://chromium.googlesource.com/chromium/src/+/refs/tags/148.0.7778.96/third_party/blink/renderer/core/css/resolver/style_cascade.cc),
declaration application has cascade priority, origin and matched declaration
coordinates before applying the resolved value. These are candidate internal
hook locations; neither source reading nor existing value-oriented inspector
methods establish a ready-to-use provenance API.

An instrumented engine must provide these operations to a private native bridge:

| Operation | Required ownership and behavior |
| --- | --- |
| Open owner | Bind a nontransferable engine document/frame identity, engine build/protocol revision and native preparation owner/generation; reject an ordinary page/CDP caller |
| Observe winners | Flush style under the same owner, identify the final winning source for each requested fill/stroke slot, and correlate an actual presentation declaration with the exact live element/attribute; reject unresolved fallback, inheritance, animation or declaration mapping |
| Observe environment | Read engine-owned media/rendering state and its revision directly; page or isolated-world JS values cannot mint this evidence |
| Invalidate | Advance a monotonic owner epoch before any relevant mutation can become usable; cover DOM/style/UA/user/extension writes, settings/media changes, animation transitions, navigation and provider loss, including A-to-B-to-A |
| Compare and apply | In one engine-controlled interval, compare the expected epoch and original owner/value mapping, apply only native-plan alias replacements, then validate the same winners and resulting aliases before publication |
| Revoke | Invalidate handles and pending operations on failure/disconnect; recapture requires a new owner and cannot reuse receipts |

Observation must preserve the original source identity through engine resolution;
a CSS value or a matched-rule index alone is not enough. V1 rejects unsupported
resolution paths rather than recreating Blink's cascade in JavaScript. The bridge
returns bounded scalar predicates and opaque native handles, never a copied
browser CSS tree, serialized lease or caller-defined provenance record.

The compare/apply interval must cover every writer, including other debugger
sessions and browser/extension processes. An implementation can serialize a
competing change or abort its own operation before publication; it must not drop
user settings or suppress accessibility overrides. Checking a revision after a
visible partial write is insufficient. Rendering/publication must also be held
until the internal alias operation succeeds. No network request, asynchronous
page callback or user interaction belongs inside this interval. Existing route
and byte limits bound the work; operation failure returns no admitted result.
Receipt acquisition and actual paint consumption remain separate later phases.
Rollback may restore only still-owned alias values under the same guard; otherwise
it must dispose the owned output rather than overwrite intervening user changes.

Admission remains unavailable until a maintained instrumented browser build
implements these hooks and passes the same-value origin, environment, competing
writer, partial-failure and lifecycle matrix. This workspace has no Chromium
engine checkout/build target, and the installed stock browser exposes no verified
adapter satisfying this contract. No speculative C++ patch or native mock is
presented as engine integration. Isolated-world observation is retained as a
prototype result; debugger pause is explicitly rejected as an exclusion mechanism.
Native external-fragment and dependency validation may proceed independently
while this engine integration dependency remains open.

CEM audit: native source/theme ownership remains authoritative; replay cannot
outvote user overrides to preserve a fixture result. Variation is bounded by the
named profile, environment and existing source limits. Accessibility settings are
preserved and unsupported cases keep ordinary rendering. No component behavior,
public tokens, defaults or broad browser/device/AT acceptance change.

### Shared browser stylesheet observation host

`BrowserHtmlBaseHost.captureStylesheets(responses, { ownership: 'exclusive' })`
now creates a `BrowserStylesheetHost` over the confirmed native HTML execution.
`BrowserWorkerPool.captureStylesheets(html, responses, options)` additionally
binds that host to the pool's lifetime and worker generations. The native
`HtmlStylesheetSession` retains its HTML CSS preparation, immutable acquisitions,
policy capability and sealed execution evidence. Native DOM predicates run before
and after stylesheet observations. JSON response input is an explicit acquisition
control boundary; the returned occurrence count and blockers are diagnostics.
Neither carries an AST, execution proof or edit grant between workers.

The current HTML profile covers document style elements, style attributes,
external links and readable loaded imports. Titled sets require explicit disabling
or the controlled external-link activation described below; inactive unloaded
imports use the bounded extension described below. Source-probe text comes from retained
native document values or exact acquired responses. For each occurrence the
adapter compares actual CSSOM with the browser's parsing of that native-owned
text in an inactive document, checks final link/import URLs and native import
order, and records stable CSSOM identities. A supplied source fingerprint or
refetch alone cannot pass this comparison. Media queries and import supports
conditions use the browser engine. Root selection uses actual disabled state;
import selection includes its parent's selection and conditions. Every document
stylesheet must have a recorded installation.

`change(callback)` invalidates before a synchronous CSSOM, selection or controlled
environment write, including same-value and change-and-restore operations.
`read()` requires a current capture and synchronously rechecks native ownership,
DOM correspondence, stylesheet identities, source correspondence and conditions.
A controlled change permits a new capture over matching sources; a failed
comparison or reentrant/asynchronous callback disposes the session. DOM, history
and base-policy changes use the owning HTML host's `changeContext` and require
fresh HTML preparation. Load/error, resize/orientation and media-query events
invalidate pending evidence; page hide, abort and disposal end the session.
Listeners and comparison sheets are removed on disposal. Uncontrolled synchronous
CSSOM/environment A-B-A is outside the exclusive-writer contract; MutationObserver
and event listeners cannot reconstruct it.

Pool close, error, invalid messages and replacement invalidate attached evidence
before later worker results can publish. Failed/replacing generations cannot
create fresh sessions; successful replacement restores admission only once all
unavailable generations have been replaced. The DOM and native evidence stay on
the main thread. A diagnostic summary cannot be consumed as a worker-local seal.

Bounds cover 256 sheet/response occurrences, depth 16, 65,536 native root nodes,
4 MiB of native response/source text and bounded CSSOM comparison work. Native
CSS decoding evidence must identify UTF-8. Unreadable cross-origin CSSOM, unloaded
root sheets, active unloaded imports and shadow/adopted sheets reject this profile.
There is no fallback to source-only evidence or network acquisition. Compiled CEM
output uses its own native consumer and installation contract below.

The packaged Chromium fixture exercises real inline/link/import application,
CSSOM mismatch, acquisition mismatch/missing responses, controlled A-B-A,
disable/restore, load/error, real media transitions, unowned DOM/media changes,
HTML invalidation, reentry, asynchronous callbacks, abort, pool close and forced
worker replacement. Runtime inner-rule/computed CSS/SVG resources and responsive
image acquisition remain explicit blockers, as does atomic host transaction
admission. CEM review: native semantic/source ownership and shared bounded host
capabilities are preserved; application/component UI behavior, public tokens,
inherited styles and accessibility are unchanged.


### Compiled template and body stylesheet installations

`RetainedTemplate::stylesheet_installations(consumer, indices)` captures native
compiled output and the exact leases for its consumer occurrences. The resulting
`CompiledStylesheetSet` retains source closures, native scope/context identity and
emission fragments. Replacing or releasing a consumer invalidates its leases even
when another consumer keeps the cached output alive. Each lease retains its own
incoming closure, so shared cached output cannot hide source cancellation or policy
changes in another consumer. Owner disposal also revokes those occurrence leases. Staging a pending load leaves the currently committed emission live;
successful replacement changes the lease. Scope admission continues through the
existing private/shared/instance validation, including authored shared names and
native context markers. Any emission diagnostic rejects installation evidence:
suppressed layered imports or declarations cannot certify a complete emission.

`ExecutedTemplateCssImports::compile_installations` binds every recorded body
stylesheet to its exact captured import closure, lexical capability and source
snapshot, using the same native emitter. Choices must cover all recorded bodies
and their stylesheet occurrences in order, and declare an admitted managed scope.
No body/CSS tree is serialized or reparsed. Bodies with no stylesheets still retain
their policy lifetime. The returned `RecordedBodyStylesheetInstallations` supports
bounded emitted text access, explicit release and sealed observation capture;
`RecordedBodyStylesheetEvidence::validate` checks all retained policies before
and after installation callbacks. HTML document/link observations remain a separate
requirement; this source binding does not certify the entire rendered DOM.

`CompiledStylesheetHost` compares exact native emissions against trusted host
observations. Every occurrence must have a distinct nonzero installation identity
and successful source comparison. Capture takes two consistent passes; later
reads revalidate consumer leases, source policies, selection and environment.
Mismatches, callback failure and controlled invalidation permanently stale the
old evidence. Native emission byte and occurrence limits apply before comparison
text allocation. Imported rules flattened into an output sheet share that one
real installation; the observer must not fabricate an installation for each
original import. Inner supports/media/scope applicability and computed resource
loads retain explicit runtime blockers.

`RecordedBodyStylesheetInstallations::compose_output(parents, max_nodes,
max_bytes)` now joins these declarations to a complete native render. Parent IDs
come from the original retained output and must cover every stylesheet occurrence
in order. Recorded declarations admit private/shared scopes and append to the
HTML head; instance payloads keep their separate scope/placement contract. The
composer reuses the recorded native render plan and native CSS emission fragments,
including their source frames. It does not reevaluate template expressions, acquire
sources again, infer scope ownership, or edit the original tree. Node and byte
bounds apply before cloning; the byte preflight conservatively reserves XML
escaping space. HTML raw-text style terminators reject this bounded profile.

The returned `RecordedTemplateOutput` retains the original execution and body
installation owners, the new native tree, stylesheet node bindings, XML output
and an explicit HTML delivery projection. Native XML export/import materializes
the composed semantic tree through the shared CEM-ML boundary; original template
and CSS source owners remain distinct from output XML coordinates. Delivery bytes
and source spans alone grant neither execution authority nor source rewrite rights.
An identical reimport or a second equal-byte composition is a different owner.

Capture that exact new tree in `HtmlBaseExecutionHost`, inspect its document, and
call `RecordedTemplateOutput::capture` with a `RecordedTemplateOutputHost` and
trusted stylesheet/DOM predicates. Effective/fallback base and policy must agree
with the recorded preparation. The join observes the entire document CSS forest,
then binds each compiled body occurrence to the observation for its exact native
style node. Flattened imports retain one installation each; there is no separate
caller assertion that body and document installations agree. The sealed
`RecordedTemplateOutputEvidence::validate` rechecks whole-document topology,
identities and values, native source/policy lifetimes, loaded CSSOM correspondence,
selection and environment on both sides of consumption. Callbacks that mutate
non-style content also invalidate the join. Controlled invalidation, HTML recapture,
source drift, cancellation or release cannot revive earlier evidence, including
empty body stylesheet sets. Every context/DOM and CSS/environment writer must
invalidate its respective host before writing; unobserved synchronous A-B-A
changes remain outside the exclusive-owner contract. The original execution's
`html_output()` provides the corresponding explicit HTML projection, preserving
empty template elements when delivering to a text/html document.

`RecordedPreparationWorker` owns a bounded registry (1–256 live preparations)
for one native execution realm and worker generation. Its `prepare` takes the
exact composed output and inventory, transfers the HTML host into the registry,
and captures the joined record with a privately owned stylesheet host. Only a
native `RecordedPreparationHandle` leaves the registry. The handle has no public
constructor or serialization path: an equal worker address, local ID, output
string or blocker report cannot recreate its private owner identity. Retained
records keep the source snapshots, lexical policies, output tree, source frames
and installation evidence alive without depending on caller-owned variables.

`read` revalidates the complete DOM/CSS record and checks worker, handle and
cancellation lifetimes before and after each scalar observer callback. It returns
diagnostic blockers only. A failed read removes the record permanently; callback
unwinding also releases it. `release` revokes all clones of a handle, while `close`
and worker drop revoke the generation and discard its records. A cloneable
`lifetime()` control permits immediate revocation from callbacks or host shutdown;
flagged records are reclaimed at the next registry operation or worker drop.
Capacity is checked before observations, released entries are reclaimed before
new preparation, and local IDs are never reused within a generation.

The host must close the lifetime before worker failure/termination and release
affected handles before any controlled DOM/CSS/context write, including same-value
writes. `replace` accepts only the next address for the same slot, as issued by the
shared worker coordinator; it closes the old identity before creating the new one.
Old handles and old lifetime controls cannot act on replacement records, even
when local IDs repeat. Fresh preparations are required after replacement.
Cancellation also remains bound to the original native execution and its sources;
empty stylesheet sets receive the same lifecycle and policy checks.

The production QL WASM `RecordedOutputSession(requestJson, slot, generation)`
now retains this registry in the synchronous DOM observation realm. The request
is an explicit source-acquisition envelope, bounded to 4 MiB: `rootUri`,
`outputUri`, `resolutionBase`, `policyStamp`, `browser: {engine, version}`,
`sources: [{uri, revision, source, contentType}]`,
`stylesheets: [{requestedUrl, finalUrl, revision, source, contentType}]`,
`choices: [{bodyUri, index, scope, hostPaths?}]` and optional scalar-string `bindings`.
Private scopes use `{kind: 'private', tag}`; shared scopes use
`{kind: 'shared', name}`. Native compilation validates the complete ordered
choices against recorded body occurrences. Sources enter the shared importer
once; missing acquisition responses cannot fall back to network I/O. No AST,
evidence or result handle is accepted in this envelope. The initial profile uses
immutable lexical resolution contexts and standard-agreement HTML base behavior.
Context-sensitive sheets (including imported CSS) require explicit `hostPaths`:
element-ordinal paths in the original native output, such as `[0, 1, 0]` for
the first element in an explicit HTML body. Native
`compose_output_with_hosts` validates the selected host's HTML namespace, private
tag or shared membership, and direct instance-island template before adding the
native resolver-context marker. Missing/foreign hosts, conflicting authored
markers or two different contexts for one host reject. Paths are bounded to 64
segments and 256 hosts per sheet; marker attributes count toward output bounds.
The original source/render is unchanged. JavaScript neither derives nor patches
these internal markers. Non-contextual sheets may omit host paths.

`captureBrowserRecordedOutput(document, native, install, options)` and
`pool.captureRecordedOutput(...)` own the native session, including failed
construction/capture. Supply `{ownership: 'exclusive', policy: {stamp, decide},
signal?}`. The document must be a live UTF-8 HTTP(S) document matching `outputUri`.
The synchronous `install(html, stage)` callback delivers first the `original`
render and then the `composed` output into that same document. Each delivery is
followed by full native correspondence and a real base freeze. Returning a Promise,
inexact delivery, changing the document URL, reentry or cancellation rejects and
closes the session. HTML is a named delivery artifact; the browser never reconstructs
the semantic source tree or native evidence from it.
The rendered tree must already match the browser's DOM: parser-inserted nodes
and nonempty inert template contents are outside this exact-correspondence
profile and reject capture.

`read()` returns diagnostic blockers after complete live validation. `dispose()`
releases ownership; `change(write)` revokes before a controlled synchronous
DOM/CSS/history/policy write, including A-B-A changes. A fresh native session is
required afterward. Shared DOM correspondence and document ownership prevent a
second HTML-base or recorded-output host from claiming the same document.
DOM mutation, CSSOM/installation drift, policy stamp or URL changes, load/error,
resize, page hide and abort invalidate the record. Callback failures cannot
publish a handle; WASM disposal waits until an active native call returns.
An initial browser-generated load notification for a captured inline sheet may
arrive after synchronous delivery. It preserves the record only after a complete
native reread. Later or synthetic load notifications and all errors revoke it.

For external sheets use
`await captureBrowserRecordedOutputAsync(document, native, install, options)`
or `await pool.captureRecordedOutputAsync(...)`. The installer itself remains
synchronous. The host owns the document during initial link loads, with a whole
load deadline (`timeoutMs`, default 5000, bounded 1–30000). Every link root must
load; unreadable CSSOM, missing acquisitions, failed loads, timeouts, DOM/policy
drift, reentry and cancellation reject and release ownership. Imported sheets
are compared occurrence by occurrence against the native acquired snapshot,
including final URLs and import conditions. Inactive unloaded imports retain the
existing bounded observation rules. Source text alone never proves installation.

Supply `stylesheetTitle` for explicit selection: an exact external title activates
its eligible roots; an empty string disables all titled roots. Commands come from
the existing native `HtmlStylesheetSelection` contract. Persistent sheets remain
outside those commands and authored-disabled roots cannot be activated. Selected
external links must complete a fresh browser load and install a different enabled
CSSStyleSheet. Active inline titled sheets remain unsupported; inline titled
sheets can be explicitly disabled. Omitting the option rejects titled roots.
Selection occurs before the first preparation capture: native DOM/base/source
validation runs again before publication. Changing an existing preparation,
including selecting the same title again, requires `change(write)`/disposal and a
fresh native session; an old record cannot be revived or recaptured in place.

Both hosts reuse the shared CSSOM source/selection observer. Outer media is
checked against native metadata and live matchMedia results; media changes,
orientation changes, CSSOM mutations and selection drift revoke evidence.
Shadow roots and adopted sheets remain outside the profile. Body imports remain
flattened natively into their private/shared compiled installations. No public
theme variables or inherited overrides are rewritten by the observation layer.
Pool closure, worker
failure and generation replacement revoke these main-realm dependencies before
new work can publish; fresh capture resumes only after pool recovery. Dedicated
workers never receive a DOM/evidence DTO or consume this native seal. Moving the
semantic owner off-thread would require a separate bounded observation protocol.
The live Rust/Chromium predicate fixture remains independent of this real WASM
production path.
Compiled scope/inner-condition applicability, responsive images, computed CSS/SVG
resources, other runtime loads and atomic transaction admission retain their
blockers. A joined DOM/CSS record is not complete dependency closure.

Responsive images in the whole-output async host use native `ImagePlan` and
`ImageRecord` owners. The plan retains the exact composed tree and resolves
`img@src`, imported `img@srcset`, and preceding `picture/source@srcset`
candidates against the confirmed execution base. Inert templates are excluded;
malformed/incomplete candidates, foreign URLs, unresolved or unloaded selections
and exceeded bounds reject. Browser choice remains a live engine observation:
the adapter records `currentSrc`, completion and natural dimensions after
`decode()`; it does not implement the browser's sizes/media/DPR algorithm or infer
which occurrence won when candidates share a URL. Each occurrence retains its
native owner, attribute, ordinal, resolved URL and parsed descriptor/ranges.
A selected URL matching multiple occurrences rejects with
`ambiguous selected image candidate occurrence`, including equivalent relative
URLs, different picture sources and src/srcset fallback collisions. This is a
conservative bounded profile: it does not eliminate inactive candidates by
reimplementing browser selection. Repeated URLs on different img targets are
independent; their byte reacquisitions may still be shared.

The async host then separately reacquires each distinct selected URL, with a
shared 4 MiB streaming byte limit and the same whole preparation deadline.
This initial transport profile requires same-origin HTTP(S), rejects redirects
and custom image crossorigin/referrer policies, and accepts declared raster MIME
types PNG/JPEG/GIF/WebP/AVIF. Lazy or failed images that do not decode within the
deadline reject. Opaque acquisition bytes, final/request URLs, MIME and SHA-256
revision enter the native record through an explicit byte ingress; no image AST
or execution seal is serialized. Source text/metadata are not substituted for
the retained semantic tree. Selected URLs/dimensions are checked again after
acquisition, before/after native CSS capture, and on every read. DOM mutations,
load/error, viewport/DPR/picture-media changes, cancellation and pool replacement
revoke the preparation; fresh capture requires a new native session. The
synchronous factory cannot capture documents requiring image acquisition.

`read().imageAcquisitions` is diagnostic metadata counting distinct acquired
responses. A same-URL fetch can return different bytes from the browser's original
image request: this record does **not** assert byte identity, a complete request
history, winning duplicate source occurrence, or image-specific authorization
beyond this bounded transport policy. Existing resource blockers remain and
`image-browser-response-identity-required` is explicit. A future controlled
transport/response binding must supply that missing evidence before image
dependency closure or transaction admission. SVG images, image preloads,
cross-origin/redirected acquisition and other runtime loads remain outside this
profile.

Original-response binding must begin before the initial image request. The
[HTML currentSrc contract](https://html.spec.whatwg.org/multipage/embedded-content.html#dom-img-currentsrc)
exposes the current request URL, not response bytes or candidate identity;
[Resource Timing](https://www.w3.org/TR/resource-timing/) supplies timing/size
metadata rather than a response-body handle. Neither matching dimensions,
matching timing/size, a reacquired digest, nor a server validator alone is an
original-response seal. The present post-load adapter cannot recover that proof.
The next transport must retain the actual delivered body before consumption,
bind it to the execution/document/request lifetime, cover cache and redirects,
and revoke on transport loss. A controlled replay would be a **new** delivery
with native source mapping, never proof about the earlier browser load. The
current API does not accept caller-supplied historical-identity assertions.

The shared opt-in image delivery transport now supplies that pre-consumption
byte boundary for intercepted requests. An application-owned controlling service
worker installs `installImageDeliveryTransport`; async recorded-output capture
selects `imageDelivery: 'service-worker'`. A private MessagePort associates the
worker's actual client ID with the document and native session. Before either
HTML delivery, the host opens the native ledger and worker session. For each
same-origin raster image request, the worker buffers the decoded entity body,
submits it to native byte ingress, and awaits acceptance before constructing the
browser response from those same bytes. No AST or native seal crosses realms.
The native ledger computes its own digest and binds storage to execution
cancellation, bounded URL/MIME metadata, 256 responses and 4 MiB total. Repeated
intercepted requests share a session's immutable body; conflicting native ingress
revokes. After decode, the adapter seals transport ingress, joins selected URLs
to receipts, and publishes the usual native record. Missing receipts reject;
there is no second-fetch fallback in this mode. `imageDeliveries` is diagnostic.

Registration/activation/scope remain application-owned; absence or replacement
of the controlling worker rejects. The shared transport allows at most 32 client
sessions, with a maximum 30-second preparation deadline. Failure, rejection,
abort, output invalidation or pool closure aborts pending requests and closes the
native ledger. Subsequent intercepted image loads after sealing revoke. This
initial profile rejects redirects, custom image request policy, SVG, cross-origin
responses and non-200 responses. It reconstructs a response from the decoded
entity body with raster Content-Type and no-store; encoded-wire bytes and original
HTTP headers are not claimed as evidence.

A delivery receipt proves which body this transport released. It does not prove
that every selected img consumed that response: browser available-image caches
can bypass interception, even across documents. The
`image-browser-response-identity-required` blocker therefore remains until
cache-safe consumer/request binding exists. This is distinct from the completed
pre-delivery capture/lifetime join; complete resource history and transaction
admission remain blocked independently.

The explicit `service-worker-replay` delivery profile closes selected-response
identity for a new execution. The trusted controlling worker generates a fresh
128-bit random namespace. Native `ImageReplayPlan` retains the original tree,
attribute/candidate ordinals and descriptors, resolves source URLs against the
confirmed original base, and creates at most 256 same-origin routes under
`/__cem_image_delivery/<nonce>/<ordinal>`. Native render-plan composition replaces
candidate URL spans only; it retains the replay plan on the composed output owner.
The original semantic tree is never rewritten by JavaScript. Route pairs are an
explicit transport-control boundary, not AST or evidence serialization.

The adapter installs those routes before composed HTML delivery. The worker
fetches each selected route's original source, holds the decoded body for native
acceptance, and serves the identical body under the fresh route. Repeated use of
one route within the execution refers to one immutable response; previous
executions cannot have populated the fresh namespace. Selected currentSrc values
therefore join unambiguously to both native candidate occurrences and delivery
receipts, including duplicate authored URLs. Native reads continue to revalidate
DOM/selection/lifetime. Missing receipts, foreign sources, authored reserved
routes, expired sessions and unknown reserved routes reject; there is no network
fallback for reserved URLs. Only the image response-identity blocker is removed
once native replay receipts join; all remaining resource/admission blockers stay.

Replay is an opt-in **new delivery**, not proof of an earlier image load. It
changes reflected image URLs and can affect URL-attribute selectors and the
browser's cache-dependent candidate choice. It does not preserve original HTTP
headers or encoded wire bytes. Applications requiring unchanged URL reflection
must use the existing observation profiles and retain their identity blocker.
Original source authorization, source-map rewrite ownership and complete runtime
closure remain separate requirements before transaction admission. Private/shared
CEM scope, token ownership and declarative component contracts are unchanged.

The QL WASM `CompiledStylesheetSession(artifact, consumer, indicesJson)` binds
current retained-template consumers inside that WASM realm. The shared browser
`captureBrowserCompiledStylesheets(document, nativeSession, styleNodes, options)`
accepts this opaque owner and the exact ordered installed HTML style elements.
`pool.captureCompiledStylesheets(...)` adds pool lifecycle revocation. Both factory
forms take ownership of the session, including disposal on failed capture. A CSS
string, serialized artifact or worker response cannot replace the native lease.
The compiled observer remains local to the realm holding the QL owner; it does not
transfer worker-resident preparation evidence or reconstruct recorded bodies from
worker metadata. This artifact-consumer bridge is separate from the whole-output
`RecordedOutputSession` bridge above.

The browser compares installed CSSOM against the browser's inactive parsing of
native emitted text. It requires readable native `@scope`, exact style/sheet/parent
identity, light DOM, and untitled style nodes without outer media. Native managed
scope text must match. The passed list describes the compiled set, not the entire
document stylesheet universe. Controlled CSSOM/selection writes use `change` and
require a fresh `capture`; `read`, `dispose`, abort, page hide, DOM mutation,
load/error, viewport changes and pool lifetime follow the same fail-closed rules
as HTML observations. Actual CSSOM mismatch or replacement ends the session.
Exclusive ownership is required to cover synchronous CSSOM A-B-A. No generated
component behavior, app-local parser or stylesheet installer is added.

### Explicit stylesheet sets and inactive imports

HTML observation accepts `{ titledSets: 'disabled' }` to establish inactive
preferred/alternate sets through actual `CSSStyleSheet.disabled` writes. Untitled
persistent sheets retain their selection. `host.disableTitled()` invalidates before
writing, including same-value calls, and requires a fresh capture. Native metadata,
actual titles and disabled state must agree; unowned enablement rejects consumption.

Active external titled sets use the asynchronous
`html.selectStylesheets(nativeDocument, policy, responses, title, options)` API,
or its `pool.selectStylesheets(html, ...)` counterpart. `options` requires exclusive
ownership and accepts an abort signal and a 1–30,000 ms load deadline (10,000 ms
default). An empty title disables all titled roots. Native selection matches titles
exactly, leaves persistent sheets alone, and never enables authored-disabled roots.
Active inline titled styles and initially unloaded roots remain outside this bounded
profile. The whole selected set must be supported before any writes begin.

An alternate can expose `disabled === false` while its rules remain inactive.
Flipping only CSSOM flags does not activate it. The
[CSSOM model](https://drafts.csswg.org/cssom/) also distinguishes a disabled flag
from whether a sheet applies. The controlled transition instead toggles each selected
link element's disabled state, establishing explicit enablement and waiting for a
trusted load event with a fresh enabled stylesheet installation. Every selected link
must finish. Synthetic load events, errors, timeout, abort, disposal or worker-pool
lifetime termination cannot complete a transition. Other titled roots are explicitly
disabled; persistent sheets keep their existing selection.

Before those writes the host disposes prior CSS evidence and invalidates HTML
capture, including same-title and A-B-A transitions. During loading, reads,
captures and competing changes reject; unowned DOM mutations revoke the host.
After loading, the host performs a fresh HTML base freeze and full native/DOM
correspondence check. Native rebinding requires the same retained tree, response
snapshot, HTML host, effective/fallback base and policy, with a strictly newer
HTML epoch. It then verifies each loaded root/import against native-owned source
and actual CSSOM. The private activation receipt binds the selected DOM nodes to
those loaded stylesheet objects; a title string or diagnostic summary cannot
substitute for it. Any subsequent observed load, CSSOM or environment change
invalidates the receipt and requires another controlled selection. Failure revokes
the HTML session; recovery requires a new host and capture. These observations are
current execution evidence, not historical network provenance or edit authority.

An imported sheet with an actually null `CSSImportRule.styleSheet` can be observed
as unloaded only when its effective parent selection or browser-evaluated
conditions are inactive. Its unloaded descendants remain explicit observations;
expected grammar comes from browser parsing of native-owned CSS. Every source
still needs its immutable acquisition snapshot, even when the browser did not
fetch it. Missing active imports reject, and unloaded root links remain unsupported.
A later load, condition or selection change invalidates and requires a new capture.
Imports omitted by browser CSSOM, including unsupported `supports()` imports in
Chromium, reject observation rather than being guessed as absent loads.

Chromium fixtures cover computed private/shared/instance scoping, flattened
imports, CSSOM/source tampering, replacement, selection, independent consumer
release, callback release, disposal and pool close. Additional fixtures exercise
explicit preferred/alternate disabling, same-value invalidation, unowned enablement,
the inactive alternate with a false disabled flag, a pending inactive-media
import with a null stylesheet and its unloaded descendant, its later real load,
a missing active import, and a CSSOM-omitted unsupported import. Active-transition
fixtures verify actual multi-link/import application, repeated and A-B-A selection,
pending read exclusion, synthetic events, failure/timeout/cancellation, DOM/policy/source
drift, foreign native owners, pool close and worker replacement during loading. Native fixtures
cover complete and empty body policy ownership, emission diagnostics, source/order/byte
bounds and stale evidence. These observations keep
runtime resource closure, whole-output correspondence and atomic edit admission
requirements intact. CEM review confirms shared native authority and bounded
capabilities, with no changes to UI semantics, public tokens, inherited overrides,
accessibility or theme ownership; forced-color checks are inapplicable here.
