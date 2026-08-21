# CEM Studio Phase 6.5 UI Classification

Status: completed 2026-08-21. This classification gates the initial Studio
shell and workbench before UI composition. Product requirements remain owned by
[`cem-studio.md`](./cem-studio.md), the pinned comparison remains owned by
[`angular-material-parity.json`](../packages/cem-components/tests/angular-material-parity.json),
and the executable classification is
[`ui-classification.json`](../packages/cem-studio/tests/ui-classification.json).

## Outcome

All 23 initial shell/workbench behaviors have an explicit owner and contract
boundary against Angular Material `22.1.1` at tag `v22.1.1`, commit
`0b67c3c38141049657b1167479accc80e455d2bd`:

- 5 behaviors reuse general CEM controls directly;
- 11 keep application state and orchestration in `@epa-wg/cem-studio` while
  rendering every visible control through CEM components;
- 7 are reusable workbench composites with no Angular Material catalog
  counterpart and are therefore reserved for a future
  `@epa-wg/cem-components/studio` export. Both view-switching behaviors now
  reuse the completed general `cem-tabs`/`cem-tab` contract, so all 23
  classifications are open and no general parity gate remains.

No application-local widget is accepted. Search uses `cem-autocomplete` with
Studio/CEM-QL-owned filtering. Source editing uses `cem-textarea` as the
canonical baseline. Data, selection, routing, persistence, workers, updates,
and commands remain application services that drive shared controls.

## Classification

| Behavior | Accepted disposition | Visible CEM owners | Composition gate |
| --- | --- | --- | --- |
| `shell-command-bar` | Reuse general controls | `cem-app-bar`, actions, select, badge | Open within the single-row app-bar and native-action subsets. |
| `responsive-workspace-layout` | Studio orchestration | `cem-grid`, `cem-stack`, `cem-surface` | Open; no grid-list, sidenav, or splitter behavior is claimed. |
| `compact-pane-navigation` | Reuse general controls | `cem-tabs`, `cem-tab` payloads | Open; Studio owns pane routes and preserved workbench selection. |
| `home-and-projects` | Studio orchestration | card, actions, badge, alert, progress | Open within the proven subsets. |
| `project-hierarchy` | Reuse general controls | `cem-tree`, `cem-tree-item` | Open; Studio owns data, selection, loading, and routes. |
| `explorer-reordering` | Future `/studio` composite | `cem-studio-project-explorer` over tree/actions | Open for component implementation; stable-id transactions stay in Studio. |
| `explorer-context-actions` | Reuse general controls | actions, tooltip, dialog | Open with visible actions; popup menus remain forbidden. |
| `project-search` | Studio orchestration | autocomplete/options plus tree/list results | Open; filtering, ranking, and routing stay in Studio/CEM-QL. |
| `source-editor-frame` | Future `/studio` composite | `cem-studio-editor-frame` over textarea/field | Open for component implementation; native textarea owns editing. |
| `resource-identity-form` | Studio orchestration | fields, text, select, autocomplete, checkbox | Open within proven field/form subsets. |
| `run-workbench-form` | Studio orchestration | schema-selected CEM inputs/actions/feedback | Open; the typed CLI schema and Studio own projection and values. |
| `cli-command-roundtrip` | Studio orchestration | textarea, actions, autocomplete, dialog, passive table | Open; parsing and transactional Apply remain CLI/application behavior. |
| `result-view-navigation` | Reuse general controls | `cem-tabs`, `cem-tab` payloads | Open; Studio owns result projections and current artifact selection. |
| `structured-data-inspector` | Future `/studio` composite | `cem-studio-data-inspector` over tree/table/sort/paginator | Open for component implementation; data loading and sorting stay in Studio. |
| `diagnostics-panel` | Future `/studio` composite | `cem-studio-diagnostics` over list/badge/actions | Open for component implementation; diagnostic data and source navigation stay in Studio. |
| `report-event-source-trace` | Future `/studio` composite | `cem-studio-trace-view` over tree/table/paginator | Open for component implementation; trace data and navigation stay in Studio. |
| `safe-preview-frame` | Future `/studio` composite | `cem-studio-preview-frame` over preview/feedback/actions | Open for component implementation; sandbox, CSP, limits, and downloads stay in Studio. |
| `transformation-graph-view` | Future `/studio` composite | `cem-studio-graph-view` over actions/status/tree/table | Open for component implementation; graph semantics remain CEM-ML/Studio. |
| `run-status-and-cancellation` | Studio orchestration | linear/circular progress, badge, alert, action | Open; worker lifecycle and cancellation stay in Studio. |
| `clipboard-download-feedback` | Studio orchestration | actions, alert, textarea | Open; persistent alert feedback replaces an unowned snack-bar lifecycle. |
| `confirmation-and-destructive-actions` | Studio orchestration | dialog shell, actions, alert | Open; transactions and recovery stay in Studio. |
| `project-settings` | Studio orchestration | cards, fields, select, checkbox/switch/slider, dialog | Open; exclusive choices use select until radio-group parity exists. |
| `storage-offline-update-status` | Studio orchestration | badge, alert, progress, action, dialog | Open; storage and service-worker lifecycle stay in Studio. |

## Accepted General Boundaries

The 19 partial Angular Material rows do not all block Studio. A partial row is
usable when Studio requires only its proven subset. The initial boundary is:

- use `cem-app-bar` as one application/context header, not as a generic or
  multi-row toolbar;
- use native `cem-action` and `cem-icon-button` variants, not FAB or
  interactive-disabled variants;
- use standalone `cem-badge`, passive `cem-table`, passive/native-single-select
  `cem-list`, and determinate/indeterminate `cem-progress` only within their
  recorded contracts;
- render explorer commands as visible actions. A popup menu waits for the
  general menu container, trigger, focus, positioning, dismissal, and nesting
  contract;
- use `cem-select` for exclusive settings until a labeled radio-group contract
  exists;
- use semantic grid layout rather than claiming Material sidenav behavior; and
- use persistent `cem-alert` feedback rather than inventing snack-bar duration,
  queue, dismissal, or focus-restoration behavior.

Changing one of these boundaries requires updating the executable
classification and completing the corresponding general parity row before
Studio composition changes.

## Reserved `/studio` Boundary

The seven planned `/studio` composites are allowed because their reusable
workbench capability has no user-facing counterpart in the pinned Material
catalog. They must compose general CEM controls and must not absorb application
services:

| Composite | Reusable component responsibility | Studio application responsibility |
| --- | --- | --- |
| `cem-studio-project-explorer` | Tree/action composition, reorder interaction, stable accessible presentation | Project records, transactional moves, conflicts, undo |
| `cem-studio-editor-frame` | Document status/range frame around the canonical textarea adapter | Resource revision, autosave, editor lifecycle |
| `cem-studio-data-inspector` | Bounded tree/table/paging presentation and source-activation events | Artifact projection, data paging, sorting, source routing |
| `cem-studio-diagnostics` | Diagnostic grouping/presentation and activation events | Diagnostics, filtering, stale/current state, source routing |
| `cem-studio-trace-view` | Bounded report/event/source-trace presentation | Trace records, paging, selected frame, source routing |
| `cem-studio-preview-frame` | Safe preview frame states and fallback presentation | Projection choice, sandbox/CSP policy, limits, downloads |
| `cem-studio-graph-view` | Graph/workbench presentation and interaction events | Authored graph, validation/execution state, source routing |

These are reservations, not implemented components. Each receives its own
contract and failing component-layer tests before runtime work when its vertical
slice becomes active.

## Executable Gate

Run:

```bash
yarn nx run @epa-wg/cem-studio:verify:ui-classification
```

The gate first verifies the pinned Angular Material inventory and CEM state
matrix. It then rejects benchmark drift, unknown components or Material rows,
status drift, missing required Studio behavior, an unrecorded composition
blocker, a `/studio` owner for a Material counterpart, or documentation that
omits an executable row. It emits deterministic JSON and Markdown reports under
`dist/reports/cem-studio/`.

## Next Gate

Implement the versioned IndexedDB project repository before composing the PWA
shell or offline vertical slice. The repository must define schema versions and
migrations, atomic autosave, trash/restore, revision/hash conflict handling,
multi-tab coordination, quota diagnostics, and validated import/export while
keeping every visible control in the general CEM component surface.
