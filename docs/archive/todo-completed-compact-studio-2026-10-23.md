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

## Font-relative tree spacing follow-up

- [x] Match condensed tree spacing to the IDE reference using font-relative outer inset, horizontal gaps and hierarchy indentation; extend the existing shared tree story to verify nesting and font scaling, retain paragraph-like vertical rhythm, inherited overrides and larger touch targets.

The canonical D1 theme now separates the outer tree inset (`0.25em`), horizontal
column gaps (`0.25em`) and per-level indentation (`1.4em`). The shared declarative
tree consumes these inherited tokens. Paragraph-like vertical rows, `1em` icons,
ordinary-tree geometry and coarse/hybrid target policies remain intact.

Verification: **13/13 shared tree stories**, **6/6 density tests**, the browser
density contract across five themes, precise/coarse input and forced colors,
and the **22-task Studio build** passed. Nested label offsets, enlarged fonts
and inherited spacing overrides are checked in the existing tree story.

## Tree density theme matrices and Studio geometry

- [x] Add normal, sparse and condensed tree examples to all five gallery themes; verify real computed row/coupling geometry and inherited sparse spacing rather than attributes alone.
- [x] Verify all three Studio navigation trees use condensed computed geometry, including generated output, while preserving selected descendants, keyboard behavior and the existing touch policy.

Each of the five tree gallery themes now presents normal, sparse and condensed
examples through the shared responsive `cem-grid`, with matching enabled and
disabled trees. Sparse spacing is emitted on explicit containers as well as the
document root. Normal retains the standard coupling minimum; condensed selects
`data-cem-density="super-compact"`. Studio already authors this profile on
Projects, Structure and Output files; regression checks now measure the real
computed coupling, row minimum and height on all three, after output is built.

Verification passed: source and isolated-package gallery matrices (five themes ×
three modes), two focused Studio navigation/selection cases, six density tests,
five-theme precise/coarse-input and forced-color browser checks, scoped lint,
and the 22-task Studio build. On precise desktop input, the live Projects tree
measured about 19.2px (`1lh`) versus the normal gallery row’s 48px (`3rem`).
Existing touch/hybrid policy, font size and credential recovery rules are unchanged.

## Page-level super-compact theme follow-up

- [x] Apply inherited page-level theme, spacing and super-compact density to Studio and gallery chrome; retain independent gallery density examples, explicit size profiles, credential policy and coarse/hybrid targets.
- [x] Add canonical normal-density/spacing reset boundaries and verify nested normal, sparse and super-compact theme geometry, inherited overrides and forced colors.
- [x] Verify real app-bar icon buttons and Studio header content height under page-level super-compact density, and document the public attributes and token contract.

Studio and shared gallery chrome now expose `data-cem-density="super-compact"`
and `data-cem-spacing="normal"` alongside the page theme. The existing canonical
super-compact profile supplies `1lh` control/coupling minima, zero control padding,
zero guard/halo and `1em` glyphs on precise desktop input. Explicit sizes and
consumer shape-height overrides remain supported. No application JavaScript UI
behavior or component-local token defaults were added.

Normal density and spacing boundaries restore only mode-owned values from their
canonical base tables. Gallery `part="content"` isolates ordinary samples and
property forms from page chrome; nested sparse and condensed examples retain
their selected presentation. Coarse/hybrid targets, entered-credential blocking,
disabled automatic recovery and the aggregate 4 MiB checkpoint budget remain intact.

Verification passed: **9/9 generated theme tests**, **14/14 shared app-bar/gallery
stories**, **31/31 Studio shell/navigation cases**, five-theme precise/coarse and
forced-color browser checks, all five tree-gallery density matrices from source
and isolated package archives, scoped lint, theme CSS generation and the
**22-task Studio build**. Real shared icon buttons, normal resets, independent
spacing, explicit sizes, text scaling, inherited overrides and retained editor
selection are covered. Hybrid exclusion is verified in the generated media
contract; physical-device acceptance is not claimed.

Isolated browser checks of the rebuilt Studio at port 8080 measured a **21.19px
header**, **15px theme button** and **0px block padding** on precise desktop
input. Coarse input retained the **48px button / 3rem minimum**. The gallery at
port 8081 uses the same page profile and retains independent normal/condensed
examples. Its preview server now includes the shared navigation declaration in
its narrow public-asset list. Saved projects and drafts were not modified.

## Explicit content-sized super-compact policy

- [x] Make explicit super-compact geometry content-sized with zero coupling minimum and zero block padding even when fine-pointer detection does not match; retain independent normal/compact modes, explicit sizes, inherited overrides and credential privacy.
- [x] Verify real buttons, Studio chrome and tree gallery super-compact geometry under fine/coarse/unknown pointer media, including nested normal/sparse resets, and align docs with the explicit policy.
- [x] Preserve the shared slider’s intrinsic thumb/track geometry when the super-compact coupling minimum is zero, with a colocated story for noncollapsed controls and range interaction.
- [x] Verify unconditional super-compact table/data-view row geometry, and preserve intrinsic datepicker calendar width and stepper connector alignment when the coupling minimum is zero, using colocated component stories.

This correction supersedes the earlier media-gated super-compact policy above.
Explicit `data-cem-density="super-compact"` now applies on every input modality.
Its canonical coupling minimum, generic control height and inline/block control
padding are all `0px`; tree/table/list/menu rows retain `1lh`. Only `compact`
retains the precise-input media gate. Normal density boundaries, explicit size
profiles and inherited consumer overrides remain available. Entered-credential
blocking, disabled automatic recovery and the 4 MiB budget are unchanged.

Shared controls retain their intrinsic content geometry: slider inputs and
thumbs use the larger of the coupling minimum and canonical thumb diameter;
datepicker columns and popup sizing follow text with the existing viewport cap;
stepper connectors follow each header's actual marker center. These are
component-owned declarative CSS changes, with no application UI JavaScript or
new component-local public token defaults. Table/data-view and other shared
stories now assert explicit super-compact geometry without pointer guards.

Verification passed: **9/9 generated-theme cases**, **34/34 app-bar/tree/slider/
gallery-page stories**, **23/23 datepicker/stepper/table/data-view stories**, and
**31/31 Studio shell/navigation cases** (**97 targeted cases**). The five-theme
browser contract verifies actual fine, coarse, hybrid and no-pointer media,
text scaling, explicit sizes, local overrides, normal/sparse resets and forced
colors. Scoped lint, whitespace checks, theme generation and the final
**22-task Studio build** passed. The new datepicker/stepper stories required
waiting for their existing asynchronous render/focus publication before making
the original assertions; production behavior and timeouts were not weakened.

Live isolated contexts at ports 8080 and 8081 confirmed `0px` coupling/control
minima and block padding in all four input modes. Studio's header measured
21.19px; its theme button followed its 13.33px glyph content. Condensed tree rows
measured 19.19px (`1lh`) on every input mode; normal samples retained their
`3rem` floor and normal padding. Saved projects and drafts were untouched.

The broader nine-gallery source/package gate did **not** complete: it exposed an
unchanged extensionless Canonical source URL in `cem-text` that maps to a folder
and returns HTTP 404. That independent import-map follow-up is recorded in the
active TODO. The gallery verifier now waits for nested accordion panel content
before capturing retained owners; the original identity assertion remains intact.
No complete package-gallery pass is claimed for this correction.

## Studio editor filename follow-up

- [x] Remove repeated filename labels from editable and read-only sample/output editor bodies; keep the pane heading and accessible source name, and verify the exact Login graph URL plus sample/output navigation.

Removed the editable controls' repeated path row and adjusted the grid to give
the reclaimed row to the source editor. Read-only sample and generated-output
editors now use an empty visible `label` with the resource path supplied through
`aria-label`, reusing the existing shared editor contract. The pane heading and
window/application title retain the selected file; no new UI JavaScript or
component API was introduced.

Verification: the four existing short-workspace/editor scrolling, focus and
selection cases passed, followed by two sample/output navigation regression
cases. The 22-task Studio build, scoped lint and whitespace checks passed.
An isolated browser opened the exact reported Login URL
`/?sample=login&file=login-view-graph&build=render-graph`: the pane heading was
`view.cem`, the editor body's label was empty, its accessible name remained
`graphs/view.cem`, and all 506 source characters were displayed read-only.
Saved projects and drafts were not modified.

## Shared popup surfaces and indicator clearance

- [x] Give shared dropdown/submenu surfaces an opaque theme background and define canonical menu outer inset/item gaps with stroke-derived indicator floors independent of compressed control padding.
- [x] Verify super-compact theme menus, adjacent selected/focused rings, overridden stripe sizes, forced colors and unchanged zero control geometry; rebuild and inspect Studio and the tree gallery.

Dropdowns and submenus use the opaque comfort surface, with Canvas/CanvasText
in forced colors. Canonical menu inset/gap preferences retain a component layout
floor computed from the actual zebra stripe and focus stroke tokens, including
descendant overrides. Default super-compact menus reserve 6px around items and
12px between them, while control padding and minimum height remain zero.

Validation: 14 shared dropdown/menu/theme-switch stories and nine theme density
tests passed, including all five themes and enlarged stripe sizes. Theme generation
validated all 527 tokens; the 22-task Studio build, scoped lint and whitespace
checks passed. Live gallery checks covered all five modes with mouse and touch input, opaque surfaces,
6px/12px spacing and no unnecessary scrolling; Studio light/dark checks passed.
Repeated Studio theme changes stalled before completing the contrast transition
on both the Login graph route and default workspace; this is recorded in the
active TODO and no complete Studio five-theme pass is claimed. Browser checks
used isolated contexts, preserving saved projects and drafts.

## Immediate persisted theme updates

- [x] Add a gallery regression for runtime storage writes after host storage methods are restored, then make shared runtime writes notify live storage consumers independently of method interception.
- [x] Repair shared host-command value capture for clicks on a native button's nested icon/label; verify worker/fallback command boundaries and repeated Studio theme selections.

Two shared runtime paths could save a preference without updating its owner.
Live same-document storage consumers previously depended entirely on intercepted
Storage methods. Runtime-owned writes now publish a storage change when no
intercepted notification was issued, retaining external-writer interception and
native cross-document events without double publishing an intercepted write.
Separately, Studio host commands read only the literal click target's value;
clicks on nested menu icons/labels therefore supplied null even though the
switch saved its selection. Commands now resolve the enclosing admitted native
or form-associated control, stopping at the bound command element. No app-local
UI JavaScript or theme-specific runtime workaround was added.

Both regressions failed before their fixes: the gallery stored dark while its
page stayed light, and worker/fallback host commands received null from a nested
label. After repair, 18 gallery/storage/theme-switch tests, 10 host-view tests,
13 Studio shell tests, and five focused storage recovery/credential refusal
tests passed. The Studio shell regression repeatedly clicks nested glyphs across
all five themes. The 22-task Studio build and scoped lint passed (only existing
non-null assertion warnings). Credential recovery remains blocked for entered
secrets; isolated browser contexts preserve saved projects and drafts.

Post-build live checks clicked the nested theme glyphs through dark, light,
contrast-dark, contrast-light and native in Studio and the tree gallery without
navigation or reload, with Storage method interception deliberately removed.
All page theme attributes updated; the gallery's switch and computed palettes
also matched each selection. This resolves the earlier Studio contrast-transition
failure recorded above.


## Studio accordion delivery check 2026-10-24

- [x] Investigate the reported IDE layout rollback against Git history, current
  declarations and deployed bytes. Projects, Structure and Output files still
  use the shared full-height `cem-accordion`; the navigation/layout/center and
  shared workspace/accordion/expansion sources are unchanged from compact-layout
  commit `bfe2bc3e` through `7ddb93e7`. Source and deployed declarations match.
- [x] Restore local preview availability on `http://localhost:8080/`. No server
  was listening when inspected. Restart the existing fingerprinted preview
  handler from the current build; verify its current base URL and Windows-side
  loopback HTTP 200 response. Do not unregister service workers or clear caches,
  IndexedDB, saved projects or drafts.
- [x] Strengthen the existing `verify:preview-reload` fixture. Replace its obsolete
  `layout=browser` assertion with the actual project IDE contract: three grid
  regions, a full-height flex-column accordion, three compact visible headers,
  scrollable bodies and installed declaration-owned scoped CSS. Physically
  collapse/reopen Structure and verify pane redistribution, selected file/tree
  value, editor DOM owner, bookmark, recovered dirty draft and persisted bytes.
- [x] Run the reload scenario under an older cache-first controlling worker,
  retaining a deliberately stale bootstrap entry. A changed preview fingerprint
  loads the current IDE while the stale cache entry remains intact. No app UI
  behavior or component styling workaround is introduced.

`yarn nx run @epa-wg/cem-studio:verify:preview-reload --parallel=1` passes all
24 scheduled tasks (22 from valid cache). Scoped script lint, syntax and Git
whitespace checks also pass. A fresh isolated browser measures approximately
19.2px headers/tree rows with zero vertical control padding, independent open
panes and functional disclosure. Existing responsive breakpoints still stack
navigation below 840px; the desktop three-region layout starts at 1200px.

No authored layout rollback was found. A stopped preview can leave an installed
tab using cached files, but the reported tab's exact cached bytes and viewport
were not directly inspected. The current deployment and stale-worker reload
path are verified; refresh the original Studio URL to load the current preview.
The shared component and credential/privacy policies remain unchanged.
