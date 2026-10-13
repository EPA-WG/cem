# Legacy component baseline — 2026-09-27

## Result

At `4c467348`, with the packaged `cem-elements` build restored through Nx,
the unchanged legacy suite reports **81 passing / 48 failing / 129 total**.
This matches the counts recorded during native CSS closure. The command below
runs only the component suite; it does not claim the aggregate package gate is
green. The runtime build was run first because these tests import its package.

After the shared harness fixture repair, the suite reports **82 passing /
47 failing / 129 total**. Comparing full test names removes exactly the harness
failure listed below and adds no failures. The aggregate package gate remains red.

```sh
yarn nx run cem-elements:build
yarn nx run-many --targets=test --projects=@epa-wg/cem-components --excludeTaskDependencies --skipNxCache --args='--run --reporter=json --outputFile=/tmp/cem-components-baseline.json'
```

The JSON report and console log are local diagnostic outputs under `/tmp`;
the failure inventory below preserves their useful contents in the repository.

## Attribution and next work

- **Empty boolean attributes:** legacy templates use value bindings such as
  `@disabled={datadom.attributes.disabled}` or truthiness conditions such as
  `datadom.attributes.collapsible`. An empty string is omitted by the former
  and false in the latter. Canonical components must explicitly test presence
  with `seq:count(...) > 0` where the attribute contract is presence-based.
  The native `host_boolean_attribute_presence_is_distinct_from_value_truthiness`
  fixture covers absent, empty, `"false"`, and `"true"` values. This explains
  the source-level mismatch behind disabled controls and missing navigation,
  selectable-list, checkable-chip, transient-dialog, and busy/empty branches.
  It does not establish that every later assertion will pass after migration.
- **Datepicker structure:** the failure screenshot shows a literal `}` after
  the authored input/toggle. In `primitives.ts`, the `datepicker-day` template's
  `cem:otherwise` closing sequence also closes the outer module. Its following
  `datepicker-week` and main `body` are consequently outside that module, and
  the final closing sequence contains an unmatched brace. All seven datepicker
  tests fail after roughly their 15-second deadlines; the JSON reporter supplies
  `STACK_TRACE_ERROR`, not a useful inner assertion. The brace defect is a
  concrete source finding; its contribution to all seven failures still needs
  a focused native render and browser check during owner migration.
- **Separate unresolved failures:** autocomplete payload replacement, expansion
  activation, paginator focus ownership, sort-header owner identity, and input
  paint need individual attribution. Do not
  classify these from test names alone or weaken their assertions.
- **Harness required-input fixture (resolved):** the isolated harness failure
  occurs when an empty input incorrectly passes native form validation. Its
  test-owned field template bound `required` directly to the host value, so
  the empty attribute was omitted on the native input. Switching that fixture
  to the already-proven presence expression restores the existing assertion.
  The browser fixture also checks absent, empty, `"false"`, and `"true"` values,
  live removal/reintroduction, native `validity.valueMissing`, and input identity.
  No production component, shared runtime behavior, or assertion was relaxed.
  The full `verify-phase3-harness` gate passes, including lint and typechecking.

Continue with the accepted `cem-action` declarative cutover. Site's v3 XHTML
asset delivery prerequisite is complete. Preserve this baseline when comparing
consumer tests, and migrate each remaining owner with its existing contracts.
The legacy behavior modules remain frozen; this investigation changes no
production semantics or test expectations.

## Failure inventory

This is the original 48-failure snapshot; resolved cases remain here as evidence.

Counts are per test file. Each entry records the first reported failure only;
assertions after it were not exercised successfully.

| Test file | Failed |
| --- | ---: |
| `src/lib/autocomplete.browser.spec.ts` | 2 |
| `src/lib/datepicker.browser.spec.ts` | 7 |
| `src/lib/divider.browser.spec.ts` | 1 |
| `src/lib/expansion.browser.spec.ts` | 4 |
| `src/lib/feedback-expanded.browser.spec.ts` | 7 |
| `src/lib/paginator.browser.spec.ts` | 4 |
| `src/lib/sort-header.browser.spec.ts` | 2 |
| `src/lib/states.browser.spec.ts` | 18 |
| `src/lib/workflows.browser.spec.ts` | 2 |
| `src/lib/testing/component-harness.browser.spec.ts` | 1 |

### autocomplete.browser.spec.ts

- **refreshes live payload without replacing focus, input identity, committed value, or events** — `Error: Timed out waiting for replacement option renders`
- **supports native migration and projects disabled, readonly, busy, required, and invalid states safely** — `AssertionError: expected false to be true // Object.is equality`

### datepicker.browser.spec.ts

- **keeps one direct native text input and optional native toggle as the exact owners** — `Error: STACK_TRACE_ERROR`
- **keeps canonical value, locale calendar, validation, form, and reset ownership on the input** — `Error: STACK_TRACE_ERROR`
- **moves one roving grid focus owner through day, week, month, and year navigation before confirmation** — `Error: STACK_TRACE_ERROR`
- **keeps pointer drafts and cancel, Escape, and backdrop dismissal silent until Apply commits** — `Error: STACK_TRACE_ERROR`
- **suppresses every disabled route and rejects malformed owner vocabulary without substitution** — `Error: STACK_TRACE_ERROR`
- **keeps today, selected, focus, hover, and disabled paint independent and geometry-stable** — `Error: STACK_TRACE_ERROR`
- **supports a silent expanded property while live bounds update validity and calendar suppression** — `Error: STACK_TRACE_ERROR`

### divider.browser.spec.ts

- **exposes exact semantic orientation and an explicit decorative boundary** — `AssertionError: expected null to be 'true' // Object.is equality`

### expansion.browser.spec.ts

- **exposes one exact heading, button, and persistent controlled-panel relationship** — `AssertionError: expected null to be 'region' // Object.is equality`
- **uses one native click path for pointer, Enter, and Space while preserving owner identity** — `Error: Timed out waiting for expansion state true`
- **keeps disabled activation suppressed while allowing silent programmatic state control** — `AssertionError: expected false to be true // Object.is equality`
- **keeps hover, focus-visible, and active paint on the header with stable transient geometry** — `Error: Timed out waiting for expansion state true`

### feedback-expanded.browser.spec.ts

- **uses native owners for transient initialization and live state transitions** — `Error: Expected fixture to contain dialog`
- **delegates modal focus, keyboard dismissal, return value, and restoration to the native dialog** — `Error: Expected fixture to contain dialog`
- **keeps focus-visible ownership on native dialog fallbacks and authored descendants** — `Error: Expected fixture to contain dialog`
- **preserves native fallback focus, modal boundaries, and state through cancel and close** — `Error: Expected fixture to contain dialog`
- **paints only focused native dialog fallbacks with the D5 zebra outline** — `Error: Expected fixture to contain dialog`
- **keeps a transient sheet non-modal, focus-neutral, and application-controlled** — `AssertionError: expected false to be true // Object.is equality`
- **preserves open-dialog identity and state while cleaning up close, replacement, and reconnect paths** — `Error: Expected fixture to contain dialog`

### paginator.browser.spec.ts

- **renders exact landmark, page-size, range, action, normalization, and optional-control semantics** — `AssertionError: expected [ 'previous', 'next' ] to deeply equal [ 'first', 'previous', 'next', 'last' ]`
- **navigates once through pointer, Enter, and Space with focus-stable suppressed boundaries** — `Error: Missing paginator last action`
- **keeps global disabled and live programmatic control silent while retaining surviving owners** — `AssertionError: expected false to be true // Object.is equality`
- **keeps hover, focus-visible, and active paint on native controls with stable transient geometry** — `AssertionError: expected <select …(6)>…(1)</select> to be <button type="button" …(8)>…(1)</button> // Object.is equality`

### sort-header.browser.spec.ts

- **renders exact column-header, native action, direction, and accessible-name ownership** — `AssertionError: expected false to be true // Object.is equality`
- **suppresses disabled activation and keeps programmatic state silent with stable owners** — `AssertionError: expected <button type="button" …(7)>…(2)</button> not to be <button type="button" …(7)>…(2)</button> // Object.is equality`

### states.browser.spec.ts

- **reflects action, loading, disabled, expanded, selected, and focus states on native controls** — `AssertionError: expected false to be true // Object.is equality`
- **applies shared native hover treatment without changing action geometry or semantics** — `AssertionError: expected false to be true // Object.is equality`
- **styles only navigation hover owners without changing current selection or component state** — `Error: Expected harness fixture to contain cem-nav[collapsible] > nav > .cem-nav__disclosure`
- **moves keyboard focus through navigation owners without changing selection or component state** — `Error: Expected harness fixture to contain cem-nav[label="Focus workspace navigation"] > nav > .cem-nav__disclosure`
- **applies navigation active treatment during trusted pointer and native keyboard activation** — `Error: Expected harness fixture to contain cem-nav[label="Active workspace navigation"] > nav > .cem-nav__disclosure`
- **applies shared native active treatment during pointer and keyboard activation** — `AssertionError: expected false to be true // Object.is equality`
- **composes tokenized input indicators across appearance, hover, focus, validation, and selection states** — `AssertionError: expected 30 to be less than or equal to 1`
- **moves keyboard focus through every enabled input indicator without changing component state** — `AssertionError: expected false to be true // Object.is equality`
- **projects explicit busy state across every input without taking over its lifecycle** — `AssertionError: expected null to be 'loading' // Object.is equality`
- **reflects form disabled, invalid, required, readonly, checked, and indeterminate states** — `AssertionError: expected false to be true // Object.is equality`
- **toggles checkable content chips without changing passive chip semantics** — `Error: Expected state render output matching cem-chip[checkable] button`
- **styles only interactive content hover owners without changing selection or component state** — `Error: Expected harness fixture to contain #interactive-content-list > select`
- **moves keyboard focus through interactive content owners without changing selection or checked state** — `Error: Expected harness fixture to contain #focus-selectable-list > select`
- **toggles collapsible navigation without changing passive landmark semantics** — `Error: Expected state render output matching cem-nav[collapsible] button`
- **selects declarative list options without changing passive list semantics** — `Error: Expected state render output matching cem-list[selectable] select`
- **marks explicit busy cards without making nested content primitives loading owners** — `AssertionError: expected null to be 'loading' // Object.is equality`
- **marks explicit empty workflow surfaces without inferring layout emptiness** — `AssertionError: expected null to be 'empty' // Object.is equality`
- **marks explicit busy workflow surfaces without making formatting containers loading owners** — `AssertionError: expected null to be 'loading' // Object.is equality`

### workflows.browser.spec.ts

- **renders the registration auth workflow with required, invalid, and loading states** — `AssertionError: expected false to be true // Object.is equality`
- **renders the password reset workflow with help, error, and loading feedback** — `AssertionError: expected false to be true // Object.is equality`

### component-harness.browser.spec.ts

- **proves action and field contracts through real CEM-ML substrate declarations** — `AssertionError: expected true to be false // Object.is equality`

## Canonical action cutover comparison

After moving action to canonical XHTML and colocated stories, the full legacy
suite still reports **82 passing / 47 failing / 129 total**, with exactly the
same failing test names and no skipped tests. Six canonical action stories pass
separately. Workflow fixtures explicitly register the canonical declaration;
legacy action unit assertions now live in its stories. This establishes no new
legacy-suite failures, while the existing owner repairs remain open.

## Action state attribution — 2026-09-28

The unchanged state file remains **3 passing / 18 failing**. A temporary
presence-binding probe for icon-button/menu-item changes this to **5 passing /
16 failing** without altering assertions or adding failing test names. It also
exposes a masked post-Space assertion that incorrectly assumes a new native click
leaves the entire event payload unchanged. A focused runtime regression confirms
that the slice remains `"click"` while native target metadata refreshes.
See the [investigation and pending decision](legacy-action-state-investigation.md).
The probe was removed; no production migration or baseline repair is claimed.

## Checkable migration and required-marker names — 2026-10-10

Checkbox, radio and switch now have canonical declarations and colocated stories.
Their legacy-only primitive and state cases move to those stories, covering native
forms, reset, fieldsets, radio peers, mixed state, keys and indicator paint.
The shared name helper now uses `dom-accessibility-api`; regression cases prove
hidden required markers stay out of names, descriptions remain separate and
explicitly referenced hidden labels can name controls. The original registration
and reset workflow assertions remain unchanged. The focused legacy run passes
all 14 workflows, eight primitive tests and two harness tests. This is a focused
result, not a new full-suite baseline; the other failures above remain open.

## Autocomplete migration — 2026-10-10

The eight legacy autocomplete tests now live in the canonical declaration's
colocated CSF Next stories, with an additional boolean-presence/five-theme play.
Both recorded failures are resolved: live replacement targets the current
`cem-payload:payload` section, and disabled/readonly/required/busy attributes use
presence rather than string truthiness. Original assertions for live input and
focus identity, committed value, geometry and absence of events remain intact.
Render-dependent assertions await runtime settlement rather than assuming a frame
is sufficient. All nine component plays pass; two shared editable-choice plays
also cover worker/fallback forms/reset/restore, event ordering, composition/key
boundaries, active identity and nested owners. Source and isolated package pages
and the canonical bundle pass their focused interaction gate. The legacy
behavior and browser spec are removed. No new full legacy-suite baseline is
claimed; remaining owners above are still open.

## Datepicker migration — 2026-10-11

The canonical datepicker declaration fixes the module-brace defect recorded
above. A native regression compiles its actual module and renders named week/day
templates, inert boundary-year cells and boolean attributes. All seven original
browser cases now pass in colocated stories, alongside an eighth case for
presence-only attributes, years 0001/9999 and five themes. Two shared
`calendar-input` plays cover worker/fallback ownership, modal drafting, focus and
event ordering, fieldset disable, reset/reconnect and parent payload updates.

The property-page checks exposed a second, shared defect: full reconciliation
preserved a child component's generated output but skipped updates to its inert
authored payload. A fixture failed with `Arrival` after the parent changed the
label to `Departure`. Reconciliation now targets the registered child's payload
container while preserving generated output and native input identity. The
fixture also verifies consecutive name/default changes, dirty-value preservation
when the authored default is unchanged, silent updates and exact FormData.
All 22 focused calendar/input-value/select-value/prepared-patch plays pass, as do
17 datepicker/autocomplete component plays and the 22 existing workflow/primitive
regressions. The old state suite's deleted color-comparison helper was restored
verbatim so its remaining assertions typecheck.

The source/package datepicker gate covers property controls, five-theme examples,
form/reset ownership, calendar interaction, forced colors and the combined bundle.
The legacy datepicker registry entry, behavior, browser spec and global CSS are
retired; the migration inventory is 21 canonical / 32 legacy / 51 legacy authored
code files. This does not establish a green full legacy suite or physical-device
and assistive-technology acceptance. TypeScript checking currently needs the CLI
build's temporary generated command-schema source materialized; the CLI build
removes that file afterward. No generated source is retained by this migration.

## Timepicker and slider migration — 2026-10-12

Both controls now use canonical XHTML declarations, embedded scoped token CSS
and colocated CSF Next stories. All seven timepicker and six slider legacy
browser cases transfer with their assertions intact. Shared `time-input` and
`range-input` capabilities preserve the authored native inputs, labels, form
ownership and native events. Time choices close and restore focus before the
input/change pair; composition, modified editing keys, readonly and inherited
fieldset disable retain native behavior. Range bounds and non-crossing clamps
run before application listeners receive the native event. Reset and reconnect
stay silent, with canceled and stale reset work ignored. Ordered slot slices
preserve authored owner order, and repeated parent single/range mode changes
retain the surviving input and its edited value.

A pointer fixture exposed a shared scoped-CSS defect: the `[slot] > *` lower
boundary excluded Chromium's native range thumb, leaving the pointer-disabled
input without its intended pointer-enabled thumb. A failing native regression
and failing browser hit-test preceded the fix. The boundary now uses
`[slot]:not(input) > *` in native emission and compatibility projection. Inputs
are void authored elements, so the exception admits their native pseudo-elements
while preserving ordinary projected-descendant and nested-DCE exclusions.
All six native CSS rule-assembly regressions pass. The existing CSS-scoping and
stylesheet-adoption browser cases also pass.

Verification passes all 13 transferred component plays, four shared
worker/fallback plays, 46 focused declaration/processing-boundary unit checks,
and the 22 existing workflow/primitive regressions. The shared plays cover exact
owners, malformed payload rejection, native thumb hit-testing, inherited disable,
composition-ending keys, reset/reconnect and repeated parent mode changes.
The source and isolated npm-archive gate verifies both property pages,
five-theme galleries, forms, reset and the combined bundle. Style, state-matrix,
material-parity, package and forced-color gates pass. Type checking passes with
the temporary generated CLI schema described above; lint passes with existing
warnings. No generated source is retained.

The legacy behaviors, registry declarations, browser specs and global selectors
are retired. Inventory is **23 canonical / 30 legacy components / 47 legacy
authored code files**. This focused verification does not establish a green full
legacy suite or physical-device and assistive-technology acceptance. Navigation
and interactive content owners are next in the active checklist.

## Navigation and content migration — 2026-10-12

`cem-nav`, `cem-tabs`, `cem-stepper`, `cem-tree`, `cem-list` and `cem-chip`
now ship canonical XHTML declarations, scoped CEM-token CSS, colocated stories,
property playgrounds, five-theme galleries and release-bundle examples. The four
legacy navigation/tab/stepper/tree behaviors and the three separate component
spec files are retired. Inventory is **29 canonical / 24 legacy components /
40 legacy authored code files**.

The new shared capabilities are `navigation-landmark`, `tab-panels`,
`workflow-panels`, `tree-view` and `native-list`. They use public parts and exact
structural owners, retain existing payload vocabulary/event detail types, and do
not add form association. Worker/fallback evidence includes arbitrary host tags,
nested panels, modified/composition keys, reconnect and native owner retention.

Presence-only nav/list/chip attributes now accept empty and false-valued
attributes. Navigation disclosure and chip state update one stable native button;
conditional child slice directives preserve the existing boolean event snapshot
contract. Tabs/stepper retain edited panel controls and recover focus when a
programmatic update hides the focused panel. Tree collapse recovers to a visible
ancestor and cancels recovery when focus moves outside. List payload normalization
avoids the legacy native typed-attribute comparison errors and keeps native change
ownership. Scoped selectors respect the native specificity ceiling.

The fresh pre-cutover comparison had 17 passing tab/stepper/tree cases and
3 passing / 11 failing state cases. Eight state failures belonged to this set;
all their assertions now pass in canonical plays. Mixed-family assertions moved
with their owners, and the list loading-boundary assertions became a dedicated
canonical play. A snapshot comparison ignores only the hydration revision that
`snapshotInstance()` itself advances; it still compares all other island bytes,
live markup, semantics, events and geometry. Keyboard paint baselines explicitly
move the pointer off the subject before capture.

Verified after migration:

- 34 colocated component plays pass together; four shared worker/fallback plays
  and 19 declaration-contract unit checks pass.
- All five navigation/content/tabs/stepper/tree forced-color gates pass.
- `verify-navigation` passes for all six property pages and galleries in source
  and isolated npm archives, including native keyboard/selection behavior,
  live labels/hidden state, and individual/bundled declaration loading.
- Build, architecture, style, state matrix (41 covered / 0 gaps), Material parity,
  package, lint and both package typechecks pass. Lint retains existing warnings.
- The remaining primitive suite passes 3/3 and workflow suite passes 14/14 after
  loading the migrated canonical declarations. The remaining state suite has the
  same three pre-existing failures: explicit busy cards, explicit empty workflow
  surfaces, and explicit busy workflow surfaces. The full legacy gate is not green.

Next recommended migration: surface/text/stack/grid/divider/card, starting with
those loading/empty fixtures. Presentation preparation remains deferred in TODO.


## Layout and content migration — 2026-10-12

`cem-surface`, `cem-card`, `cem-text`, `cem-stack`, `cem-grid` and `cem-divider`
now use canonical per-component XHTML, scoped theme-token CSS and colocated
stories. No runtime capability or component behavior module was needed.
The inventory is **35 canonical / 18 legacy components / 38 legacy authored
code files**. The standalone state and divider specs are retired; their full
assertions live with their canonical owners. The text light-DOM assertion also
moved from the remaining primitive suite into its story. The unrelated legacy
feedback smoke fixture now uses a native wrapper, with every assertion retained.

Before changing behavior, transferred stories reproduced four failures: card
busy, surface empty, surface busy and divider decorative presence. Explicit
`seq:count(...) > 0` presence checks and single stable native elements fix them.
Busy takes precedence over empty; state changes retain section/header/body,
projected native controls, edited values and focus. Divider orientation and
decorative state update one line while preserving exact spacing/guard/stroke
geometry. Text, stack and grid remain passive. Grid auto tracks adapt using the
theme reading measure; explicit 1–4 columns remain equal flexible tracks.

Verified after migration:

- **21 layout/content plays**: seven transferred cases, 12 worker/fallback
  retention/reconnect cases and two stack/grid geometry cases. These pass with
  the **34 navigation/content plays**, for **55 passing tests in 12 files**.
- `verify-layout` passes for all six property pages, five-theme galleries,
  declarative property controls, native hidden/direction behavior, source
  fetches and bundled declarations from source and isolated npm archives.
- The dedicated divider forced-colors gate preserves every separator color,
  stroke width, inset, vertical extent and guarded margin-track assertion.
- Build, architecture, style, state matrix (41 covered / 0 gaps), Material
  parity, package, lint and both package typechecks pass. Lint reports warnings
  and no errors. Typechecking used the temporary generated CLI schema described
  above; no generated source remains in the worktree.

The refreshed **entire remaining legacy spec inventory** is **27 passing /
11 failing / 38 total** across eight files. These counts describe the smaller
remaining suite, not a direct count comparison with the original 129-test suite:

| Remaining spec | Passing | Failing | First observed failures |
| --- | ---: | ---: | --- |
| `expansion.browser.spec.ts` | 0 | 4 | Missing region presence, expansion activation timeout, disabled suppression, expansion-dependent paint lifecycle |
| `sort-header.browser.spec.ts` | 2 | 3 | Disabled semantics/focus and optional direction-cycle suppression |
| `paginator.browser.spec.ts` | 2 | 4 | Missing first/last controls, missing last action, disabled suppression and native Tab sequence |
| `progress-spinner.browser.spec.ts` | 4 | 0 | — |
| `primitives.browser.spec.ts` | 2 | 0 | — |
| `workflows.browser.spec.ts` | 14 | 0 | — |
| `testing/component-harness.browser.spec.ts` | 2 | 0 | — |
| `cem-components.spec.ts` | 1 | 0 | — |

The aggregate legacy gate remains red. Start the next canonical migration with
expansion, table, sort-header and paginator fixtures; preserve assertions that
lie beyond each first failure before attributing all failures to presence.
Presentation preparation remains deferred and its TODO context is preserved.


## CEM theme principle completion audit — 2026-10-12

The user made CEM principles a standing requirement for every decision and
completion review. `CLAUDE.md` and the active TODO preserve this requirement;
[the layout/content audit](../packages/cem-components/docs/layout-content-contract.md#cem-principle-audit--2026-10-12)
traces the latest six components to their dimension contracts.

The audit corrects the earlier grid-sizing decision recorded above: a maximum
reading-line measure must not serve as a minimum generic grid track width.
The required D1 `--cem-layout-grid-min` endpoint now owns that threshold, with
its `14rem` default from the canonical D1y example. It is generated from the
Markdown table, inherits normally, and has no component-local default. Data
text now consumes tabular lining numeral features, reading text uses optical
sizing, and surface/card separation consumes the D4 work-layer endpoint with
non-shadow forced-color contours.

Two new fixtures reproduced the grid and data-typography defects before the
fixes. All **61 plays in 12 files** now pass: the earlier 55 plus six five-theme
inheritance checks. The theme Phase 13 gate passes source-table compilation,
manifest/CSS validation, all ten browser generators, all five theme modes,
forced-color/reduced-motion checks, contrast/focus/target smoke and cross-spec
semantics. Source/packed/bundle delivery, component and theme package gates,
style checks, lint (existing warnings only) and fresh package typechecks pass.
The underlying 11 legacy behavior failures remain the next migration work;
this audit does not claim a green aggregate legacy gate or completed physical
assistive-technology acceptance.


## Disclosure and data migration — 2026-10-12

`cem-expansion`, `cem-table`, `cem-sort-header` and `cem-paginator` now ship
canonical scoped XHTML/CEM-ML declarations, colocated stories, property pages,
five-theme galleries and source/packed/bundle consumers. Inventory is
**39 canonical / 14 legacy / 32 legacy authored code files**. Three old behavior
modules and their separate specs are retired; all 15 cases and state/parity
assertions moved intact. Public parts replace class-dependent behavior through
shared disclosure-panel, sort-action and pagination capabilities.

The original 11 remaining legacy failures are resolved by these migrations.
The five remaining legacy specs pass **23/23**: workflows 14, spinner 4,
primitives 2, harness 2 and package unit 1. The 20 new component plays and three
shared worker/fallback plays pass, including presence semantics, exact native
owners, nested-table isolation, event silence, reconnect, fractional page-size
normalization and four five-mode inheritance audits. Twenty-four existing
stylesheet browser regression plays also pass. A sort fixture now waits for
peer ARIA projection after observing the active header's render; its assertion
and deadline are unchanged.

The [CEM principle audit](../packages/cem-components/docs/disclosure-data-contract.md)
corrected D2 floors: native paginator controls clamp D2c visual dimensions to
zone minimums and adjacent-control gaps to guard minimums. Header inline floors
are explicit. D6 data typography includes numeral features. All public tokens
remain inherited and theme-owned; no new token family or CSS exception was added.

Combined-bundle verification reproduced a shared worker artifact eviction:
stylesheet begin lacked the compile preflight already used by render-diff.
A one-entry-cache unit fails before and passes after the fix. Only begin
reacquires source; delivery/release preserve generation ownership. The cache
remains bounded. **68 runtime unit checks** pass, including declaration shape,
processing engine, cancellation and release contracts. Source/packed/bundle,
three forced-color contracts, architecture, styles, state/parity, package,
lint and fresh typechecks pass. Lint retains 115 component and 15 runtime
pre-existing warnings with zero errors and no new warnings in this set.

### Aggregate verification boundary

The full canonical cohort ran **240 passing / 2 failing / 242 total** across
39 files. Failures are datepicker editor-focus restoration (active element is
body) and timepicker disabled-clear readiness. The configured package gate
also failed navigation and datepicker native radio checks, the sheet's
application-controlled gallery input, and both surface-verification aliases
at the tooltip's alternate-launcher focus transition. These are open actions
in `todo.md`; no assertion was removed or deadline extended.

An isolated sheet capture shows `part=body` containing an inert template with
`cem-field` after the parent update, leaving no live input and no diagnostics.
An isolated tooltip capture confirms the authored alternate selector and exact
`aria-describedby` link after rebinding, also without diagnostics. The causal
lifecycle fixes still need focused fixtures. Focused disclosure/data success
must not be read as a green full package gate.

The aggregate also found stale offline Figma metadata (252 vs 272 generated
variables). Counts were refreshed from all five mode artifacts and the native
review verifier passes. Historical manual review evidence remains unchanged;
no external Figma update is claimed. Physical-device and assistive-technology
acceptance remains deferred as explicitly recorded in the main TODO.


## Aggregate lifecycle repairs — 2026-10-12

The captured failures now have shared runtime fixtures. Explicit HTML payloads
retain their template patch identity while parent updates reconcile the already
unwrapped inert source. Worker/fallback plays preserve the same edited input
and dialog through parent changes and optional toggle insertion/removal.

A tooltip fixture reproduces Escape dismissal while its surface has pointer
interest and the browser does not deliver `pointerleave`. Hidden surfaces now
release that interest, allowing a newly bound external launcher's focus to open
the tooltip. The previous native focus and description assertions are retained.

Radio capture shows a newer user choice being overwritten by an older child
publication. Native edits now fence live checked state from the moment an
authored render is requested, including both members of a radio group. Defaults
still update and a later authored change remains authoritative. Full plans,
worker patches, DOM fragment merges and actual worker/fallback runtimes are
covered. The navigation property page also passed 50 rapid presence cycles.

The full-cohort picker capture retained the exact native input identities.
Timepicker disabled removal waited behind option rendering; datepicker closed
with body focused, `aria-expanded=true` and no commit events yet. Native
availability and close/focus/commit now complete independently of asynchronous
publication. A matching retained time list updates active/selected state before
showing; changed option lists still await publication. Assertions and deadlines
are unchanged.

The aggregate additionally exposed stale offline component inventory and a
missing dependency on the runtime build in inferred browser-test tasks. The
inventory now maps all 53 components to their executable states and public
properties; all 53 external component reviews remain planned. The reference
parser distinguishes summary tables from detailed attribute tables and retains
duplicate rejection within summaries. Browser-test targets depend on the full
component build, preventing its clean step from removing imports during tests.

CEM principle audit: exact native focus, value, form and event owners are retained;
visibility follows semantic availability; bounded native choice models govern
transient presentation. Capability code remains shared in cem-elements and
component UI remains declarative. No CSS geometry, public tokens, theme defaults
or inheritance boundaries changed. Physical-device and assistive-technology
acceptance remains separately deferred.


Final verification: **242/242 canonical plays across 39 files**, **23/23 legacy
checks across five files**, **102 runtime unit checks**, focused regression
plays, fresh typechecks and lint pass. Lint retains 115 component and 15 runtime
warnings, with no new warnings or errors. The configured
`@epa-wg/cem-components:verify` target and all 59 dependencies pass, including
source, packed and bundle consumers and the applicable forced-color contracts.

One additional slider property-page startup timed out before root visibility in
the initial aggregate attempt. Its state was not captured then; subsequent
source/packed checks, 20 reloads under aggregate load and the final complete gate
pass. This is **unattributed**, not proof of a load-only cause or a slider fix.
The original visible-root assertion and deadline remain unchanged; failure
capture now reports host children, attributes, declaration diagnostics and page
errors. The conditional investigation remains in TODO if it recurs.


## Feedback/status migration — 2026-10-12

Migrated `cem-toast`, `cem-progress`, `cem-progress-spinner`, `cem-skeleton` and
`cem-alert` to canonical XHTML/CEM-ML, embedded scoped CSS and colocated CSF Next
plays. Inventory is now **44 canonical / 9 legacy components / 30 legacy
JavaScript/TypeScript files**. The spinner's legacy behavior and four-test suite
are retired; all of those assertions now live beside its declaration. Alert's
existing classes remain available alongside its public `part` hook.

The new reusable `progress-status` capability prepares finite/clamped circular
progress without event handlers or attribute rewriting. Its fixture first failed
without the capability, then passed in worker and fallback runtimes, including
fractional and very large finite ranges. Native linear progress retains browser
normalization and now preserves an explicitly empty value instead of treating
it as absent. Skeleton payload is both hidden from accessibility and inert.
Toast/alert retain projected content and native descendant focus behavior.

The [feedback/status contract](../packages/cem-components/docs/feedback-status-contract.md)
records the complete CEM principle audit: semantic palette/progress endpoints,
bounded alert variation and range normalization, native/read-only ownership,
D0/D2c/D7 circular geometry/paint/motion, inherited theme overrides and public
token ownership. Message styling consumes no action-state tokens. No public
component defaults, parallel token families or new token definitions were added.
Scoped animation uses a static name and token-valued longhands; the browser
fixture verifies that the compiled name selects an actual animation. Reduced
motion retains the same static incomplete arc. Linear progress keeps platform
presentation and motion policy.

Verification:

- Final standalone canonical cohort: **266/266 across 44 files**.
- Focused feedback and shared preparation: **25/25 across six files**.
- Remaining legacy suite: **18/18 across four files**.
- Configured `@epa-wg/cem-components:verify`: **all 60 tasks green**.
- Final `verify-feedback` after CSS deduplication: source and isolated package
  galleries/property controls, bundle source/metadata identity, owner retention,
  five themes, forced colors and reduced motion all pass.
- Fresh type checks pass for both packages. Lint passes with the existing
  **115 component / 15 runtime warnings**, zero errors and no new warnings.
- State matrix remains **41/41 executable-covered**; catalog and offline Figma
  inventory retain all **53 components**, with **0 external reviews completed /
  53 planned**. Token-family metadata now follows the actual feedback bindings.
- `git diff --check` passes.

Concurrent-run evidence remains separate from successful acceptance. Running the
canonical cohort alongside the full package gate produced **262/266**: the
roving-calendar and native-pointer suggestions plays timed out at their unchanged
30-second limits, followed by datepicker-open and suggestions-composition focus
failures. The later standalone cohort passed **266/266**. This does not establish
a runtime cause or justify a timeout extension; conditional diagnostic follow-up
is retained in TODO. The earlier slider readiness follow-up is also preserved.
Physical-device, spoken AT and additional supported-browser acceptance remain
open; DOM/media checks are not substitutes for those runs.


## Passive/payload migration and legacy retirement — 2026-10-12

Completed `cem-option`, `cem-option-group`, `cem-badge`, `cem-avatar`,
`cem-media-preview`, `cem-tree-item`, `cem-app-bar`, `cem-tab` and `cem-step`.
Inventory is **53 canonical / 0 legacy components / 0 legacy authored code
files**. All owners have XHTML/CEM-ML declarations, embedded once-per-declaration
CSS, colocated CSF Next plays, property pages and five-theme galleries. Passive
payload wrappers add no focus or selection owner; their consuming select, tree,
tabs and stepper preserve native interaction. Inert payload classification does
not disable interactive descendants in tab/step panels.

The registry, installer, unused option implementation, separate component specs
and harness, package-local executable tooling and obsolete TypeScript build
configuration are retired. Workspace-owned publication tools now live in
`tools/cem-components/`. The closed architecture inventory rejects new legacy
code. Package exports deliver XHTML assets, catalogs and playgrounds; the old
JavaScript root entry is removed. The stylesheet export remains a comment-only
compatibility asset. All 53 property pages and galleries are linked from both
source and published indexes.

All seven legacy workflow cases retain their assertions in card, avatar,
app-bar and sheet plays. The two generic accessible-name/native-field cases,
including their exact visual-state baseline and screenshot check, now live in
`cem-elements/src/lib/native-substrate.stories.ts`. The shared Storybook loader
loads canonical dependencies. An all-disabled listbox exposed an actual shared
choice-controller error: an empty active ID produced the invalid selector `#`.
The runtime now skips that lookup, with consumer and worker/fallback regressions.

Studio loads the XHTML bundle through the shared runtime. Its v3 module maps
classify XHTML as resources, retain all declaration dependencies, and publish
the matching bundle import-map identity. The inline-map CSP hash is refreshed.
Static schema-tour checks derive the count from the canonical manifest and keep
one example per package. This is the component dependency cutover; Studio's
visible JavaScript UI remains separately scheduled for declarative migration.

The [passive/payload contract](../packages/cem-components/docs/passive-payload-contract.md#cem-principle-audit)
records the CEM audit. Payloads consume UI typography; consuming controls own
interaction styling. Badge uses bounded semantic paired palettes and the control
bend; avatar uses D3 avatar bend and D8 initialism typography; media uses the D3
media endpoint. Spacing and boundary tokens follow their semantic owners.
Names, captions, native descendants and hidden presence remain explicit. Five
modes, inherited overrides and forced colors are verified without component
public defaults, token aliases, release-token pruning or new motion.

Aggregate verification exposed two fixture issues. Storybook's assertion API
does not implement Vitest inline snapshots; the migrated substrate uses exact
object equality over the same baseline fields. Icon-button Hover captured the
nested icon before its first render, and its exact markup diff showed that icon
render completing during the comparison. Hover and Active now use the existing
parent-plus-icon readiness helper before taking baselines. Every exact markup,
geometry, focus, event and runtime-state assertion and deadline remains intact.
The unrelated conditional timing investigations in TODO remain open.


Final verification:

- Combined component/substrate cohort: **318/318 across 54 files** (316
  canonical component plays and two generic runtime plays). The final package
  verify command passes with prerequisites excluded after their successful run.
- All **55 configured package prerequisites** passed before the cohort;
  architecture, styles, state/parity, Figma, catalog and package gates were
  refreshed after metadata updates. The final architecture check also covers
  removal of obsolete digest/registry migration guards. Architecture remains
  **53 / 0 / 0**, state coverage **41/41**, and style coverage **53 owners /
  479 consumed tokens**.
- Source, packed and bundled consumers, all new property pages/galleries,
  retention, applicable forced colors and five modes pass. Final package
  verification covers all 53 source/built index links and **296 packed files
  with zero JavaScript/TypeScript**.
- Studio: **5/5 shell tests**, static build/output verification (**98 declared
  assets / 188 files**) and all **23 UI classification entries** pass. Those
  23 visible-UI migrations remain open; classification is not completion.
- Fresh `cem-elements` production typecheck passes. Both package lint targets
  pass with the existing **115 component / 15 runtime warnings**, zero errors
  and no new warnings. The component package has no production TypeScript.
- Offline Figma metadata retains **53 components, 0 external reviews completed /
  53 planned**. External review, physical-device, spoken AT and additional
  supported-browser acceptance remain open. Presentation preparation and its
  resume context are preserved in TODO.
