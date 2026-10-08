# Suggestions acceptance evidence

The first profile attaches `cem-suggestions` to the original text input owned by
`cem-field` or `cem-text-field`. Run the colocated plays through the shared
Storybook browser target:

```sh
yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-suggestions/cem-suggestions.stories.ts packages/cem-elements/src/lib/suggestions-controller.stories.ts packages/cem-elements/src/lib/manual-listbox.stories.ts packages/cem-elements/src/lib/surface-geometry.stories.ts packages/cem-elements/src/lib/native-surface.stories.ts
```

## Automated evidence

The matrix records observed behavior; it does not establish physical device,
operating-system IME or screen-reader interoperability. The current runner is
Playwright Chromium. Its mouse, touch and pen cases inject browser input through
[CDP input](https://chromedevtools.github.io/devtools-protocol/tot/Input/), including
the browser's compatibility mouse events and implicit touch capture. Coordinates
account for the test iframe's scaling. Native touch-scroll gestures
did not scroll even a plain overflow-box control in this environment, with both
desktop and mobile emulation. The automated matrix therefore claims touch
movement cancellation and wheel scrolling separately; physical touch-pan
verification remains pending. Composition cases use Chromium's composition/input
protocol; these events are not uniformly
trusted and do not represent an operating-system candidate window.

| Area | Evidence | Boundary |
| --- | --- | --- |
| Both field providers | Original input/help identity, one FormData owner, reset, stored values and value hints | Text inputs only |
| Independent grouped filtering | Prefix filtering, unavailable rows, independent controls and preserved description | Local bounded native sources |
| Publication changes | Source replacement expires preview, old rows cannot commit, disable/read-only and conflicting inputs suspend claims | A fresh interaction is required to reopen |
| Commit transaction | Beforeinput veto, one raw input/change pair, one public pair, coherent field value and FormData at callbacks | Native undo grouping is not promised |
| Pointer taps | Mouse/touch/pen browser input commits once and leaves the original editor focused | Chromium protocol-driven input |
| Pointer cancellation | Touch movement, cancel, pen drag, outside focus and source loss cannot commit a stale press; native wheel input scrolls | No row refocus after blur |
| Composition | Composing edits update submission, close preview, settle from editor text and fence terminal Enter; a new press can select afterward | Protocol-driven composition, not physical IME |
| Native surfaces | Shared listbox/geometry/native-surface regressions plus production-template host integration with both fields and external CEMB sources: modal containment, nested Escape/held repeats, close/reopen, expired lifetime rejection, fresh admission and teardown | Worker/fallback host hooks explicitly supply ancestor authority; local adjacency supplies none |
| Accessibility structure | Listbox/group names, option active-descendant, stored-value names, localized status, disabled rows and preserved field help | DOM contracts, not spoken-output evidence |

The standalone gallery verifier also maps the seven audited source legends and
checks initial/empty edits, explicit `data` values, native text fallback,
explicit empty values, differing labels/text and non-ASCII whitespace. Keyboard
and mouse commits preserve the original input focus and FormData owner; group
filtering hides empty groups. Forced-colors mode checks a visible active-row
outline and its active-descendant relation. Run the same cases from source and
isolated package archives with:

```sh
yarn nx run @epa-wg/cem-components:verify-playgrounds -- --suggestions-only
```

The original numeric gallery examples verify ordinary field editing only. The
explicit native-datalist profile has separate evidence below. The public catalog identifies the attachment
as declarative; category-state inventory and static material parity markers do
not replace the interactive or physical acceptance records.

The row adapter cancels primary mouse pointerdown only on an admitted row. Touch
and pen pointerdown stay native. While an exact row press is armed, its
compatibility mousedown may suppress the focus default. Normal implicit capture
release after a completed touch pointerup keeps the press armed for its click;
capture loss before completion, cancellation, scrolling, drag or revision loss
invalidates it. A browser that moves focus before this suppression closes the
session and rejects the activation. It must never recover by refocusing.

Observer-driven row synchronization waits for the original parent's committed
CSS/DOM frame and fences callbacks by the active capability connection. Pending
or failed scalar readiness withdraws the prior presentation immediately, so a
later native close does not discard focused status feedback. This timing is
covered by the shared feedback play and concurrent source/installed-package
gallery verification. Explicit dismissal, blur, composition and authority loss
still clear qualifying feedback.

The host integration uses the production suggestions template under a unique
fixture tag and the unchanged production fields. It admits original external
CEMB owners through `nativeSuggestionsInputs` and exact endpoint/grant leases
through `suggestionsControllerInputs`. Captured parent lifetimes expire on
close, remain expired after reopen, and must be replaced explicitly by the host.
Source preparation settles before the deliberate opening key; a key issued
while pending is never replayed. The combined five-suite run passes 39 browser
plays. Physical release evidence below remains separate.

## Manual release record

Use the linked [property playground](../playgrounds/cem-suggestions.html) and
[gallery](../playgrounds/cem-suggestions-gallery.html) from source and an installed
package. Record browser/OS version, device/pointer, keyboard/IME or assistive
technology version, field type, exact steps and observed results. A blank record
is pending evidence. Do not infer a pass from the automated matrix.

| Pending run | Procedure and expected result |
| --- | --- |
| Physical touch and pen | On each supported browser/device, tap an enabled rich-label row on both fields. The stored value commits once, caret/focus stays in the same input, and no keyboard flicker occurs. Pan the list and cancel a gesture: scrolling stays native and neither commits. Test disabled rows, group headings, blank space and scrollbars. |
| Native IME | With a real candidate window (for example Japanese and Chinese IMEs), compose while a row is previewed. Terminal Enter/Escape/arrows belong to the IME; they neither select nor submit. Verify both possible final-input/composition-end orders, then keyup and a fresh deliberate selection. Record actual event order. |
| Screen reader | On supported browser/AT combinations, verify the field label/help, expanded state, group and active row names including stored-value hints, and localized pending/failure/empty/count announcements. Preview must not repeat unchanged status. Escape, blur and cancellation clear preview without losing the field's name/help. |
| Mobile viewport | Open the keyboard, rotate/change the visual viewport and scroll ancestors. The popup remains fitted to the original editor or closes when the anchor is unusable. It never obscures an independently focused destination or revives after dismissal. |
| Native editing/history | After commit, type, move the caret, select text and undo/redo. Record actual browser behavior and field/FormData/validity coherence. Do not require a single undo entry for scripted replacement under this first profile. |
| External/ancestor relationships | Prepare exact native endpoints and trusted ancestor lifetimes through host APIs. Verify nested Escape, parent dismissal, modal containment, placement revocation and fresh admission after resume. The simple local gallery cannot supply this authority by adjacency. |

Actionable pending runs and their scenarios remain next to the acceptance item
in [todo.md](../../../docs/todo.md). Additional editor types, full surface-provider
adoption, undoable replacement and platform close requests have separate design
items.

## Recording a physical run

Use one record per browser/device/field combination. Test both `cem-field` and
`cem-text-field` against the same options. Keep failures and inconclusive runs;
repeat a failed gesture on a plain native overflow box to distinguish runner or
platform limitations from the suggestions surface. A release run is complete
only when its actual observations are attached here or linked from this record.

```text
Date / tester:
Source revision / source or installed package:
Browser version / OS version:
Device / pointer / keyboard / IME / assistive technology version:
Field: cem-field | cem-text-field
Scenario from the pending-run table:
Steps / input event order (for IME):
Observed focus / caret / stored value / FormData:
Observed scroll / viewport fitting / spoken announcements:
Outcome: pass | fail | inconclusive
Evidence link / follow-up issue:
```

No physical device, operating-system IME or assistive-technology run has been
recorded for this revision. The template makes the remaining checks executable;
it does not close their todo item.


## Native-datalist profile: 2026-10-08

The explicit local `profile="native-datalist"` implementation is available with
both field providers. Tests retain the original input and form owner, stage
native options before claiming `list`, and preserve browser types, constraints,
labels and focus. Worker and fallback runs cover text/search/tel/url/single-email/
number, grouped-source rejection and recovery, source withdrawal, reset/resume,
policy revocation, conflicting controls and switching back to the default listbox.
Concurrent attachments use distinct runtime-owned target IDs. Publications retain
original sources and attributed omission diagnostics and offer no row commit proof.

Run the production plays together with the shared transport/lifecycle cases:

```sh
yarn nx run cem-elements:test -- packages/cem-components/src/components/cem-suggestions/cem-suggestions.stories.ts packages/cem-elements/src/lib/native-datalist-attachment.stories.ts packages/cem-elements/src/lib/native-suggestions.stories.ts packages/cem-elements/src/lib/editor-datalist.stories.ts
```

| Case | Observed evidence | Acceptance status |
| --- | --- | --- |
| Numeric values versus labels | Native options expose value `1`, label One; ordinary native typing stores/submits `1` without a synthetic replacement transaction | Automated editing contract covered; selection remains unproven |
| Actual numeric picker attempt | Linux HeadlessChrome 148.0.7778.96, original number input, browser-driven click then Arrow Down/Enter: value became `0`, with trusted input/change events | Spinbutton stepping observed; did **not** establish choosing One from a picker |
| Native constraints | `1.5` triggers step mismatch and `8` triggers range overflow for min 0/max 3/step 1; submission retains the native value | Automated contract covered |
| Email/URL sanitization | Native field value assignment trims surrounding spaces; invalid strings report native type mismatch | Automated contract covered; picker selection/filtering not established |
| Pending/failed source | Withdraws `list` and old rendered options; recovery issues a fresh admission and preserves ordinary field editing | Automated lifecycle covered |
| Semantics | Native types remain; no custom popup, active-descendant, synthetic commit or selection-proof API | DOM/event contract covered; spoken output untested |
| Native filtering and actual selection | Needs headful browser interaction and evidence of choosing displayed options, including numeric One → `1` | Open |
| Physical IME/mobile keyboard | Needs real OS candidate windows and device keyboards, including source changes while the native picker is open | Open |
| Screen readers | Needs supported browser/device/AT combinations, label/help announcements and picker navigation | Open |

These tests do not establish cross-browser native-picker acceptance. Chrome/Edge,
Firefox and Safari runs must record browser/OS versions, device and AT, exact
input type and steps, chosen visible label, resulting native value/FormData and
source-withdrawal behavior. Keep this acceptance todo open until those runs are
recorded. Unsupported native picker UI leaves ordinary editing available and
must not activate a custom popup fallback.


### Reproducible headed browser observations

The playground verifier can collect native-datalist observations with a headed
Playwright browser. It uses the production declaration and both field providers,
first from source and then from isolated npm package archives. Its numeric and
text fixtures have option values `1`, `2`, `10` and labels One, Two, Ten.

```sh
yarn nx run @epa-wg/cem-components:verify-playgrounds --skip-nx-cache -- --native-datalist-only --headed --native-browser=chromium --evidence-output=/tmp/cem-native-chromium.json
```

Repeat with `--native-browser=firefox` or `--native-browser=webkit`, a distinct
output filename and the corresponding Playwright browser installed. Run engines
sequentially so they do not compete for graphical focus. `--skip-nx-cache`
ensures this command runs the probe and writes current evidence rather than
replaying a cached log. Omit `--headed` only for explicitly headless observations.
The optional JSON output is a test report, not a runtime tree handoff.

For each field/type, the probe clicks the original input, requests `showPicker()`,
presses Arrow Down and Enter, and records native input/change events plus value
and FormData. It then enters `1`, requests the picker again, records the field's
accessibility snapshot and the still-complete native option collection, withdraws
the source, and checks that ordinary editing still works. Successful assertions
establish DOM, event and submission contracts. They do not establish which
choices the browser displayed or that a user selected a visible option.


Recorded on 2026-10-08 against runtime revision `1ced472e`, Linux WSL2
`6.6.114.1-microsoft-standard-WSL2` with WSLg display `:0`. Each completed
engine run covers eight combinations: two providers × two types × source/isolated
packages. These are Playwright browser builds, not branded Edge or Safari runs.

| Engine | Native picker attempt | Accessibility snapshot after entering `1` | Withdrawal and editing |
| --- | --- | --- | --- |
| Chromium 148.0.7778.96 | `showPicker()` accepted; Arrow Down/Enter produced numeric `0` with trusted input/change events, while text stayed empty without input/change events | Number: named spinbutton; text: named combobox | Both providers and packaging paths removed `list` and options, retained value/FormData `1`, and accepted a later edit to `2` |
| Firefox 150.0.2 | Same observed outcomes as Chromium | Number: named spinbutton; text: named combobox | Same assertions passed for both providers and packaging paths |
| Playwright WebKit build 2287 | Launch blocked before any page interaction | Not run | Not run |

Raw observations: [Chromium](evidence/native-datalist-chromium-2026-10-08.json)
and [Firefox](evidence/native-datalist-firefox-2026-10-08.json). Each report keeps
picker selection **unconfirmed**, visible filtered choices **not observed**, and
physical IME/mobile/assistive technology **not run**. A browser accessibility
snapshot is not evidence of a screen reader's spoken output. Keeping all three
DOM options after entering `1` confirms that the adapter leaves filtering to the
browser; it does not prove the visible popup's filtering behavior.

WebKit's launcher reported missing system dependencies, including GTK4,
Graphene, ICU74 and GStreamer libraries. No WebKit component assertion ran, so
this result establishes neither a product failure nor browser acceptance. Run
this probe on a compatible host, then complete branded-browser/device and
physical checks using the manual record above. The parent acceptance item stays
open; successful probe assertions alone must not close it.


## Native history assessment: 2026-10-08

The [undoable replacement assessment](../../../docs/cem-suggestions-interaction-design.md#undoable-replacement-assessment-2026-10-08)
retains the current provider contract without promising a native undo entry.
Reproduce its browser evidence with:

```sh
yarn nx run @epa-wg/cem-components:verify-playgrounds --skip-nx-cache -- --history-only --headed --browser=chromium --evidence-output=/tmp/cem-history-chromium.json
```

Repeat with Firefox and a distinct output file. Each run uses five fresh browser
contexts from source and again from isolated package archives: a plain input with
native typing, a plain input with value assignment, a plain input with
`setRangeText`, and production suggestions attached to each field provider.
Every input is required and constrained by `pattern="alpha"`; suggestions also
require selected-source proof. The fixture observes events and public state and
implements no editor or history behavior.

The probe types `al` with browser keyboard input, selects it, and replaces it with
`alpha`: by native typing, the named setter, or choosing the production suggestion
with Arrow Down/Enter. It records immediate Undo/Redo, moves to the end, types
`x`, and records a second Undo/Redo. It uses `ControlOrMeta+z` and
`ControlOrMeta+Shift+z`, without `fill()`, synthetic history input or a private
history stack. At each checkpoint it records value, selection/direction, trusted
and synthetic event traces, field getter, FormData, validity, focus and original
input identity. A plain native-typing control must demonstrate working undo.

Assertions require coherent field/submission state and native history input to
clear committed source proof, including when the string does not change. They
leave the browser's resulting string and undo grouping as observations. Original
input identity and focus must survive throughout. These checks do not require
restored strings to be valid: after a native edit, `alpha` still lacks selected
source proof under `require-selection`.


Headed Playwright Chromium 148.0.7778.96 and Firefox 150.0.2 ran on WSLg/Linux
`6.6.114.1-microsoft-standard-WSL2`, against runtime revision `adb0a04c`.
All ten source/package scenarios passed per engine (20 total). Values below were
the same in source and isolated packages; neither engine restored `al` by
immediately undoing a scripted replacement.

| Case | Chromium observation | Firefox observation |
| --- | --- | --- |
| Plain native typing | Immediate undo restored `al` with selection `[0,2]`; redo restored `alpha` | Same immediate outcome |
| Plain value assignment | Immediate undo retained `alpha`, moved caret to `0`; redo produced `alphaalal` | Immediate undo/redo retained `alpha` at caret `5` without history input events |
| Plain range replacement | Same observed result as plain value assignment | Same observed result as plain value assignment |
| Both production field commits | Immediate undo/redo retained `alpha` at caret `5`; trusted history input cleared selected-source proof | Immediate undo/redo retained `alpha` at caret `5`; no history input occurred and existing proof remained |
| Later typing after production commit | `alphax` → undo `alpha` → redo `alphax`; caret followed `6` → `5` → `6`; source proof stayed cleared | Same observed outcomes |
| Later typing in the native-typing control | Undo removed only the later `x` | Undo grouped the later edit with the earlier replacement and restored `al` |

Raw event/state reports: [Chromium](evidence/native-history-chromium-2026-10-08.json)
and [Firefox](evidence/native-history-firefox-2026-10-08.json). JSON whitespace is
compacted by checkpoint for review; the complete observations are retained.
The plain-input controls demonstrate why method presence and successful text
replacement cannot establish native history semantics. Their unusual results
are recorded without being promoted to expected behavior for other versions.

The production fields preserved their original input/focus, field getter,
FormData and effective validity throughout. `require-selection` correctly became
invalid after actual native history input or typing, even when the resulting
string was `alpha`. A shortcut which caused no edit/input event did not itself
clear the existing source proof. Neither setter nor synthetic replacement
notifications supplied the required single native undo unit in these runs.

This completes the bounded adapter assessment, not cross-platform acceptance.
WebKit history cases could not be run on this host because its browser launch
requires unavailable libraries, as recorded above. Safari/macOS/iOS, branded Edge,
physical keyboard/IME and assistive-technology evidence remain in the manual
release matrix. A future undoable adapter must meet the design's admission and
verification requirements before exposing a stronger promise.


## Platform-close integration: 2026-10-08

The shared controller now accepts explicit host admission for a CloseWatcher
reservation before a deliberate opening waits for source publication. Both
production fields preserve their original editor, text/FormData, preview cleanup,
IME ownership and Escape press handling. This API is opt-in through
`suggestionsControllerInputs.platformClose`; ordinary local suggestions and
native datalist do not acquire it.

```sh
yarn nx run cem-elements:test -- packages/cem-elements/src/lib/platform-close.stories.ts packages/cem-elements/src/lib/suggestions-controller.stories.ts packages/cem-elements/src/lib/manual-listbox.stories.ts packages/cem-elements/src/lib/native-surface.stories.ts
```

The Chromium browser runner passes 29 plays across those four suites. New checks
cover denied/synthetic/consumed interaction refusal, unsupported API, datalist
refusal, exact editor/ancestor lifetime, synchronous authority revocation,
reentrant disposal, pending cancellation, late publication, original input
preservation, held Escape, composing Escape, parent close and DOM removal.
A separate fixture leaves Escape uncanceled so the browser itself processes its
watcher stack: with a separately activated modal dialog, auto popover or hint
popover, the first native close request closes the child watcher and the next
closes its parent. These are browser-protocol observations, not device gestures.
Direct `requestClose()` tests cover lifecycle callbacks separately and do not
stand in for native grouping evidence.

The production suggestions and native-datalist attachment suites also pass all
13 checks. Runtime verification passes 608 unit tests, typecheck, lint (two
existing warnings), and package verification (222 emitted files and a clean
installed-package import).

The host admission callback in these isolated fixtures controls all relevant
watcher creation. Production hosts must independently establish that contract;
copying an always-accept callback into an uncontrolled page is not admission.
See the [host contract](../../../docs/cem-suggestions-popup-design.md#non-key-platform-close-requests-adopted-design).

Physical device run (still pending):

1. Use a host wired to the opt-in coordinator and original retained native
   source/placement grants; record browser/OS/device, source revision, field
   provider and admission status. The local property gallery supplies none of
   this platform-close authority.
2. Open a modal or auto/hint parent through one deliberate action, then open its
   child suggestions through a fresh admitted Arrow Up/Down interaction. Test
   both fields, including a hardware keyboard on mobile for this first admission
   mode. Keep the same editor focused and record its value/FormData.
3. Send the device's actual Back/dismiss gesture. Confirm child-only dismissal,
   unchanged text, cleared preview and intact parent. Repeat to close the parent,
   then verify normal navigation is no longer intercepted. Record actual outcomes
   rather than requiring identical platform navigation gestures.
4. Repeat with delayed source preparation, then finish the request: canceled
   suggestions must stay closed. Repeat parent closure, authority revocation,
   source withdrawal, same-activation refusal and fresh reopen.
5. Record IME/soft-keyboard and assistive-technology interaction separately.
   Unsupported/declined admission must retain ordinary editing and the existing
   keyboard route, without claiming Back interception.

No physical Back/dismiss-gesture run has been performed in this environment.
That todo remains open; no mobile, Firefox, Safari or screen-reader platform-close
support claim follows from the Chromium checks.
