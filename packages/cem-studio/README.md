# `@epa-wg/cem-studio`

`@epa-wg/cem-studio` is the synchronized CEM-ML browser-workbench package and
static deployment. Nx schedules its prerequisites and verification; the native
CEM-ML CLI transformation graph is the final production assembler.

Its primary UI authoring contract is the repository's
[declarative UI principle](../../docs/declarative-ui-principle.md). All visible
markup, component composition, interaction bindings, and UI state projection
must be authored in XHTML/CEM-ML from production `cem-components`. Studio
JavaScript is limited to non-UI services and host adapters; it must not render
or mutate visible UI or attach app-local UI behavior. If CEM-ML lacks a concise
capability, Studio UI work hard-stops until that reusable capability is added to
`cem-elements`. The `cem-elements` runtime and its JavaScript are the sufficient
imperative substrate; Studio must not duplicate that machinery. Components
needed by Studio follow the proven `cem-components/src/components/cem-select/`
XHTML plus CSF Next `play`-test pattern before Studio consumes them.
The accepted CSS target installs declaration CSS once through native `@scope`.
Studio may style components only through documented tags, public `scope="name"`
groups, projected `slot="name"` roots, component-owned `part="name"` hooks, and
custom properties. It must not select data-island or render-identity markers.
Studio follows the normative
[CEM light-DOM CSS scoping rules](../../docs/cem-ml-uid-and-scoped-css-design.md).
The Studio/Site declarative UI migration is complete; its evidence is recorded
in [the active todo](../../docs/todo.md).

The package currently includes a typed bootstrap/deployment
boundary and installable component shell, plus the versioned local project
repository, opt-in File System Access
provider, and graph-owned offline deployment. The project-backed workbench
executes parse, inspect, conversion, query, direct transformation, trace, and
transformation-graph commands through the same browser worker and exact-revision
repository boundary.
The application still selects the Feature Tour project and derives workbench
descriptors from its seed catalog. General project navigation is the next
delivery milestone; existing repository/provider APIs do not yet constitute
that complete user flow.

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
- `@epa-wg/cem-studio/project-session` exposes `createCemStudioProjectSession`.
  It admits durable identities through an injected workbench resolver after
  independent source Save/Discard/Cancel and command Apply/Discard/Cancel
  decisions. See the [session contract](../../docs/cem-studio.md#project-session-admission-service).
- `@epa-wg/cem-studio/preview` exposes the bounded result policy for the shared
  `cem-media-preview` declaration. Text remains literal; HTML, XHTML, and SVG
  use an empty opaque-origin iframe sandbox plus a deny-all preview CSP;
  binary, invalid UTF-8, and oversized active markup remain download-only.
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
resources, validates the proposed project, and commits current/existing/new configuration
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
Deterministic search is available through the repository protocol; project-search
UI belongs to the next milestone below. Storage status uses the CEM shell;
persistence does not introduce application-local visible controls.

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

## Hardening and browser support

Studio v1 accepts at most 8 MiB for a primary source or one dependency, 16 MiB
for the complete resource set, 128 dependencies, and 16 MiB for an exact result.
Inline text previews are capped at 256 KiB and structured views retain at most
100 rows. These application caps sit below the engine transport ceiling and are
checked before source submission, retained-artifact reads, or DOM projection.

The static deployment requires a current browser with JavaScript modules,
import maps, WebAssembly, dedicated workers, IndexedDB, and service workers.
Direct file/directory write-back uses the File System Access API where the
browser exposes it; Studio keeps validated archive import/export available as
the fallback. The API is primarily available in Chromium-family browsers, so
direct folder binding is progressive enhancement rather than a portability
requirement ([Chrome platform guidance](https://developer.chrome.com/docs/capabilities/web-apis/file-system-access)).

Deployers must apply every response header in `static/security-headers.json`.
The graph-emitted policy is hash-based for the two static inline script blocks,
allows only same-origin application assets and the narrow `wasm-unsafe-eval`
capability, and denies framing, forms, objects, ambient network origins, and
powerful permissions. Script policy excludes `unsafe-inline` and general
`unsafe-eval`. Native CEM declarations publish their retained CSS in generated
style elements, so `style-src-elem` permits those inline elements. Style
attributes remain blocked by `style-src-attr 'none'`; preview documents retain
their separate restrictive policy. Declaration owners stay connected in the
document head so their styles have a live runtime owner.

## Build and verification

```bash
yarn nx run @epa-wg/cem-studio:build
yarn nx run @epa-wg/cem-studio:test:repository
yarn nx run @epa-wg/cem-studio:test:file-system-provider
yarn nx run @epa-wg/cem-studio:test:shell
yarn nx run @epa-wg/cem-studio:test:preview
yarn nx run @epa-wg/cem-studio:check
```

The source/destination module-map pair declares every JavaScript, CSS, worker,
and WASM byte copied into the static deployment. JSON manifests and the SVG icon
are explicit typed graph imports/exports. There is no post-graph production-copy
step.

Those JavaScript assets may own bootstrap, workers, persistence, and browser
service adapters. Visible DOM composition and UI bindings belong to the
canonical declarations and shared runtime.

The final `check` also packs the npm archive, installs it into a clean consumer,
audits the exact/transitive runtime dependency path, and writes synchronized
local release evidence to `dist/reports/cem-studio/release-evidence.json`.
Publication signing and protected-release evidence remain owned by Phase 9.


## Declarative UI migration

The application shell is now `src/shell.xhtml`, composed from production CEM
app-bar, select, action, card, badge and alert declarations. It owns theme,
install/update and file-provider presentation and all control bindings.
`src/shell.js` retains non-UI browser/repository services, declaration loading
and state delivery through `cem-elements`' `mountCemView`. The theme controller
is a state/persistence service; its former `root` DOM option is retired. Theme
classes, `data-theme` and ready state now belong to the produced
`cem-studio-shell` host, inside the bootstrap mount point. Consumers that need
the rendered state await `shell.whenSettled()`; `shell.view` exposes the shared
view handle. Slotted workbench content retains its native nodes and edits.

This migration uses Studio/Site needs to improve the shared library against the
pinned Angular Material functionality comparison. A missing useful control must
be implemented in `cem-components` before app adoption. JavaScript interaction
belongs in `cem-elements`; the styled declarative counterpart consumes canonical
CEM theme tokens. Shell controls stay within their documented single-row app-bar,
select, native-action and passive-status subsets. This slice introduces no new
widget, token defaults, theme aliases, motion or parity claims.

The host-service adapter uses an explicit command allowlist and invokes services
on the native event stack so browser install/file gestures remain valid. Repeated
pending install, update and file requests are rejected by their service owners;
rendering disabled state is not transaction authority. State projection remains
declarative. Native worker/fallback and shell browser tests cover this boundary,
theme persistence, focus/owner retention, nested-view isolation and disposal.

`verify:shell-delivery` checks the graph-emitted shell with its real deployment
headers and service worker, all five theme modes, inherited public token overrides,
forced colors and offline reload. `verify:output` checks relative JavaScript
dependencies against shipped assets. The full `verify:static-worker` gate remains
separate and still validates every Feature Tour schema example and worker workflow.

`src/workbench-controls.xhtml` now owns workbench selection, source editing,
revision/status presentation, save/reload and operation controls. Parse/inspect
use native forms and the shared host command's successful scalar values, including
programmatic CEM select values. A pending service guard prevents duplicate
operations before disabled presentation catches up. Diagnostic navigation issues
an explicit one-time selection request to the shared form-control capability;
unrelated state updates preserve caret and focus. All controls reuse canonical
CEM declarations and intents without app CSS or component-local behavior.

`src/workbench-command.xhtml` owns command editing, semantic changes, apply/run
targets and incompatible replacement confirmation. It reuses canonical textarea,
select, text-field, action, table, alert, tabs and modal dialog declarations.
Shared host commands deliver only the surface owner's dismissal; nested closes
cannot cancel the outer confirmation. The service publishes drafts synchronously,
rejects stale validation results and excludes duplicate pending operations.
The adapter forwards bounded command metadata and serialized change values;
labels, truncation, action intents and availability stay in the declaration.

`src/workbench-layout.xhtml` and `src/workbench-results.xhtml` own the remaining
result, execution, expected, trace, graph, validation and provenance views.
Navigation and exact output transfers use the same host-service boundary.
`cem-media-preview` supplies literal text, isolated documents and download
messages. Unrelated updates retain native previews and focus; pending transfers
cannot duplicate copy/download operations. The service retains exact output bytes,
UTF-8 validation, resource limits and secret redaction.
Native trace events are normalized into at most 100 path/value records for the
shared table view; output copy/download still retains the complete verified bytes.

The legacy `mountCemStudioPreview` renderer, application preview stylesheet and
`CEM_STUDIO_PREVIEW_CSP` export are retired. Hosts should render the preview model
through the canonical media-preview declaration; the component owns its fixed
sandbox/CSP. Studio workbench UI contains no application DOM event handlers or
markup generators. Its empty mounting containers and hidden download anchor are
host/bootstrap and browser API plumbing.

Site's interactive examples and search now use the same canonical declarations
and host-service boundary.

## Next delivery milestone

The planned [user-owned project workspace](../../docs/cem-studio.md#next-delivery-milestone-user-owned-project-workspace)
lets users create/import, open, navigate, edit, save, reopen and export their own
projects with guarded source/command drafts, deterministic search, recoverable
trash and an active-project file provider. The shared tree-to-host scalar command
bridge and [shared navigation workflows](../cem-components/docs/project-navigation-contract.md)
are implemented and browser-verified. The session admission service now guards
source and command drafts, retains them after failed/cancelled transitions, and
isolates late workbench results. Project-owned workbenches now load durable configurations,
resources, dependencies and native command resources without requiring a catalog.
Startup uses guarded project navigation with shared CEM dialogs; Feature Tour
metadata is optional. Empty/read-only resources remain selectable, and failed
Save/Apply or cancelled navigation retains the active workbench. Home now lists projects, creates a fresh bounded CEM-ML starter and imports archives
through create-only validated transactions. Open project uses the session guards.
Explorer and bounded repository search now use guarded record identities.
Source/command and result panes retain their declared view elements and editor
selection through shared tabs. Reload restores the last admitted identity and saved
bytes. Repository changes refresh navigation/search without replacing drafts; Home
provides recoverable Trash/Restore. Provider operations retain their admitted target,
and updates wait for explicit Save/Apply or draft reset. Deployed lifecycle acceptance
now has a dedicated `verify:workspace-delivery` gate covering two user projects,
offline Save/Apply/run and recovery, actual archive import/export, Trash/Restore,
accessible names, keyboard focus and 320/390/1280px reflow. Its versioned browser
evidence joins Studio `check`; physical-device/spoken AT coverage remains separate.
Reuse existing CEM list, tree, tabs, dialog, editor and
result controls; add missing reusable behavior in `cem-elements` and its styled
declaration in `cem-components` before Studio adoption.

The [ordered checklist](../../docs/todo.md#cem-studio-delivery-planning) owns
execution. Source and command drafts now autosave to a bounded local recovery
journal, with baseline checks, visible failures and downloadable backups. Save/Apply
remain explicit project operations. Shared CEM forms now support project/configuration rename,
subproject creation, subtree copy/move/reorder and resource creation/copy, with
revision-checked session undo/redo. The Operation form section now edits native-metadata
scalar, enum and boolean command options through shared CEM controls, with stale
submission checks and explicit Apply/Run. Parse/inspect projects also support
resource-path rename with stable native references and unreferenced deletion,
using revision checks and undo/redo. Grouped command options and native graph
scalar editing are available. Graph creation now adds named nodes under
schema-approved parents through declarative shared forms, with native validation,
draft undo/redo and explicit Save/Run. Existing IDs and resource references stay
stable. Graph edge forms now support subtree parent moves, input overrides/reset
and named secondary bindings, with cycle rejection and preservation checks.
Parameter-map forms remain a follow-up alongside broader dependency formats and
simultaneous/resizable panes. See
[graph authoring](../../docs/cem-studio.md#bounded-graph-configuration-authoring).
Broader browser/device/AT acceptance remains independently open. Studio functionality
and sample availability for the Amsterdam presentation now lead the implementation
queue; the active todo records the required login journey and remaining work.

Project tree edits preserve resource IDs and immutable `referencePath` addresses.
New resources receive an ID-based address; native import captures legacy addresses
once. Rename changes metadata, refreshes the dataset projection and never rewrites
consumer bytes. Custom Chromium and browser paint-proof research are retired.

Studio installs the shared `cem-stack`, `cem-grid` and `cem-text` declarations.
Page sections use `--cem-gap-section`, blocks use `--cem-layout-stack-gap`, and
related form controls use `--cem-gap-related`. Responsive action grids and shared
text avoid browser-default paragraph margins or application-local spacing CSS.
These compositions inherit the theme's normal/dense/sparse spacing modes.

Studio uses the shared contextual sun/moon theme menu (tooltip: “Change theme”).
It restores `cem-studio-theme` from local storage before mounting the shell,
defaulting to light. Menu choices retain the shared selected-action styling.

## Example projects

Studio serves predefined and user-created/customized projects in one workspace.
A fresh profile opens Home. Select **Login** or **Schema examples** under
**Example projects**, choose **Create project from example**, then open it from
**Projects**. **Create CEM-ML project** starts with a neutral source resource.
Startup restores an existing selection without installing a special example.

Projects own resources and configurations. Authored example definitions live in
`examples/projects/*/example.json`; Amsterdam membership is catalog metadata.
Every copy has independent persistence and retains native resource identities.
Archive updates leave saved and trashed projects intact.

Login includes parse, validation, inspection, HTML conversion and form-method
validation configurations. Open its README resource for failure/repair guidance.
**Validate persisted revision** checks saved bytes while preserving an unsaved
draft. Repairing `@method=post`, saving and validating removes the deliberate
`demo.login.method_required` error; three existing reference warnings remain.

There are no dedicated example/presentation screens. Resources, configurations,
commands, results, diagnostics and import/export use the same shared controls.
Native `entries`, `entryId` and `apply-command-page` names remain compatibility
fields; UI terminology is projects, resources and configurations. See the
[workspace design](../../docs/cem-studio.md#example-project-catalog-and-workspace)
and [Login configurations](../../docs/cem-studio.md#amsterdam-presentation-samples).
