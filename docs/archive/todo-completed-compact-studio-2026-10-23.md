# Completed compact Studio work — 2026-10-23

Eleven completed execution tasks are preserved from the [active TODO](../todo.md).
The 34 remaining pre-existing unfinished tasks remain there verbatim. Earlier
completed work remains in the [prior snapshot](todo-snapshot-2026-10-23.md).

The [Studio contract](../cem-studio.md#planned-compact-ide-layout-2026-10-23)
keeps examples in the ordinary project workspace. Reusable presentation belongs
to declarative `cem-components`, shared interaction to `cem-elements`, and
Studio JavaScript to application services. The
[canonical theme](../../packages/cem-theme/src/lib/tokens/index.md#core-cem-principles)
owns compact chrome and paragraph-line super-compact trees/tables, with larger
coarse/hybrid targets. Generated project output retains its own theme.
Entered passwords or one-time codes still block recovery under the
[credential policy](../committed-checkpoints.md#credential-privacy-policy-for-live-login-recovery);
automatic whole-Studio recovery remains disabled.

## Completed tasks

- [x] Define the canonical desktop compact theme policy before component changes. Fixture: precise/coarse/hybrid input targets and guards, dense row/header/control geometry, 1em icons, project/resource kind colors, inherited overrides, wrapping/zoom, all themes and forced colors; preserve existing density profiles and project-output themes.
- [x] Add shared declarative cem-accordion and reusable leading/flush/fill expansion presentation; extend cem-tree/cem-tree-item rather than introduce a duplicate cem-treeview. Fixture: simultaneous open sections fill available height, closed headers retain leading chevrons, independent body scrolling, all-closed/short-pane behavior and accessible header actions without nested controls.
- [x] Announce and implement shared cem-elements disclosure-vs-activation tree behavior before Studio adoption. Fixture: chevron-only collapse retains selected descendants, Left/Right and Enter/Space semantics, visible focus fallback without selection changes, typed icon/kind labels, opt-in disclosure notifications, independently revocable per-event host commands and compatibility for existing consumers.
- [x] Model independent section openness, per-tree expansion/selection and globally active navigation in Studio data; compose Projects (Examples/Saved), Structure and Output files with compact heading/status. Fixture: project-to-Build, file-to-editor and folder listings, old/new URL/back/forward, distinct output-directory routes and prior-artifact reset on new source activation, one-time ancestor reveal, collapse without reopening on publication, scroll/editor retention and unsaved-draft guards.
- [x] Populate the output tree from admitted native build manifests and make the right pane follow project/source/output context. Fixture: readonly output selection, declared entry and related-output/provenance precedence, Markdown file/site HTML, ambiguous/missing matches, draft vs saved freshness, failed/cancelled rebuild retention, pinned/current runs, relative CSS/images/site links and late-result rejection without implicit Save or arbitrary Build.
- [x] Promote the existing native data:read demo viewer into a shared declarative bounded data/table component, then use it for JSON/CSV previews. Fixture: row/column/depth/cell/byte limits, quoted CSV, exact/missing/null/empty and heterogeneous/nested JSON values, native errors and structural fallback; keep parsers and source provenance native.
- [x] Pass shared source/isolated-package galleries and Studio acceptance for the new layout, keyboard/AX, fine/coarse density, narrow panes, all themes and forced colors. Preserve saved projects/drafts, editor focus/selection/highlighting, credential recovery refusal and the aggregate 4 MiB checkpoint budget; leave automatic recovery disabled.
- [x] Repair current-owner stylesheet publication cancelled by successive unrelated declaration registrations. Fixture: deterministically cancel both initial and authoritative stylesheet preparations, retain the same current render/ownership and edited DOM, then publish after registry stabilization without recording a CSS failure; real CSS errors, superseded owners, disconnect/disposal and bounded scheduling must retain their existing guards. Preserve strict gallery diagnostics and their deadlines.
- [x] Bound the source editor and expanded command inspector within the Studio center pane. Fixture: real pointer clicks can reach command tabs without editor controls overlapping them at desktop and short viewport sizes; both areas remain scrollable, the editor retains useful space, and focus, selection and drafts survive disclosure. Preserve the deployed lifecycle check rather than forcing clicks through an overlapping element.

- [x] Keep long source files bounded inside the source editor after compact pane sizing. Fixture: compare short and 200-line sources in the real short desktop shell, require stable editor and controls allocation, genuine internal editor scrolling, and retained full draft, focus and backward selection; keep canonical theme minima and avoid private child styling or JavaScript layout.

## Older helper migration completed with this delivery

- [x] Migrate the older monolithic delivery script from retired Home/dropdown/storage
  selectors to current navigator actions; retain its broader engine/provider cases
  separately from the focused `verify:navigation` layout gate. Keep this helper
  acceptance work behind the presentation functionality above.

## Verification evidence

| Gate | Recorded result |
| --- | --- |
| Native data-view contracts | `cargo test --locked -p cem-ql --lib render::data_view_tests`: **4/4 passed**. Native import/view authority remains in Rust. |
| Shared runtime and stylesheets | **782/782 runtime unit cases** and **29/29 stylesheet browser cases passed**, retaining cancellation, ownership, eventual publication and real-error guards. |
| Canonical theme geometry | All five themes passed fine/coarse probes, paragraph-line rows, `1em` glyphs, inherited overrides, wrapping/zoom and forced colors. Larger coarse/hybrid targets remain intact. |
| Shared galleries and package | All **9 affected galleries** passed at source and isolated-package locations using `--compact-studio-only`; **19 build tasks** passed, with **329 files / 59 canonical components**. |
| Helper checks | Runtime and Studio typecheck/lint passed with **0 errors**; **808 pre-existing warnings** remain warnings. |
| Studio browser cohort | **133/133 passed (638.29 s elapsed; 636.82 s tests)** across **3 files**: **107 workbench + 18 navigation + 8 recovery** (**133 cases**). |
| Recovery and privacy | **8/8 passed in the final complete cohort**. Includes live Login worker/fallback credential refusal, repeated handoffs and the aggregate **4 MiB** checkpoint budget. |
| Compact center and long-source geometry | **4/4 focused workflows passed (53.32 s)**. Pointer/disclosure cases took **4.775 / 2.666 s**; 200-line scroll/retention cases took **22.165 / 22.216 s**. The complete data, assertions and original 30-second deadlines remain. Isolated 900/1280px probes retained editor client height **126 → 126px**, internal scroll height **126 → 3935px**, and wrapper scroll height **279 → 279px**, with the full 200-line draft, focus and backward selection retained. |
| Deployed application | **Passed on the final production freeze**. Covers navigation/URL/folder actions, Create/Clone/Delete, Markdown site builds/pins/portable copies/offline, shell, lifecycle and default static-worker native/provider paths. |
| Aggregate serial release | **53 Nx check tasks passed (41 from valid cache)**. Includes deterministic output, package, release evidence and the clean seven-package consumer; static bootstrap passed, imports had no side effects, and exactly one runtime copy used CLI 0.1.0. |

The nine galleries are `cem-accordion`, `cem-data-view`, `cem-expansion`,
`cem-tree`, `cem-workspace`, `cem-table`, `cem-icon`, `cem-text` and `cem-app-bar`.
Their contracts remain in the
[accordion](../../packages/cem-components/docs/accordion-contract.md),
[data view](../../packages/cem-components/docs/data-view-contract.md) and
[component reference](../../packages/cem-components/docs/component-reference.md)
documents. The [native fixtures](../../packages/cem_ql/src/render/data_view_tests.rs),
[Studio navigation fixtures](../../packages/cem-studio/src/navigation.browser.spec.js)
and [deployed verifier](../../packages/cem-studio/scripts/verify-navigation.mjs)
retain executable assertions. No forced pointer clicks or deadline increases
replace acceptance; no JavaScript parser or private-child layout styling was
introduced. Generated screenshots and Vitest attachments remain excluded from Git.

The lifecycle gate includes Login, Supplier, graph authoring and automated
accessibility. Its optional `--presentation-only` and `--supplier-only` dispatch
paths are not claimed as separately executed. Automated evidence does not
establish physical-device or spoken assistive-technology acceptance; those
pre-existing TODOs remain open.
