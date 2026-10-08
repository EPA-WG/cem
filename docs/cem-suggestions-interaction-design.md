# Interaction and selection for attached suggestions

Status: adopted design, 2026-10-07, under the user's instruction to continue
with recommended options. This completes the keyboard/pointer/selection item in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
The shared controller implements this first text-input profile over exact native
row admissions and the existing provider/manual-listbox services. Worker/fallback
fixtures cover keyboard selection, stale-query dismissal, cancellation/provenance,
IME cleanup and native mouse activation. Production local composition and source
feedback are delivered. Chromium input-protocol fixtures also cover mouse/touch/
pen taps, normal implicit touch-capture release, canceled/dragged/revoked presses,
wheel scrolling and composition settlement on both production fields. Physical
touch-pan, operating-system IME and assistive-technology interoperability remain
in the [acceptance record](../packages/cem-components/docs/suggestions-acceptance.md);
the protocol-driven fixtures do not establish those release claims.

Use manual selection with the existing field as value/form owner and its actual
text input as focus/editing owner. Arrows preview; an explicit eligible Enter or
option activation requests a commit of the stored string. Escape, Tab and blur
never commit or clear text. Constrained selection is opt-in validity contributed
through the same field owner. Apply the [composition contract](cem-suggestions-composition-design.md),
[attachment/source adapters](cem-suggestions-attachment-design.md) and
[manual-listbox popup service](cem-suggestions-popup-design.md).

## Preview, commit and opening

An active option is one original source handle with a current admitted row
placement. It changes neither editor value nor committed identity. While the
input has focus and the popup is actually visible, exactly that row has
`aria-selected="true"` and is the editor's `aria-activedescendant`; other rows
are unselected. Selection follows preview within the listbox, as described by
the [APG combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/).
The view's `committed` flag is separate: it records an accepted field commit,
not the listbox's current preview or an authored source `selected` flag.

Focus/edit opening starts with no active option; arrow opening applies the
explicit navigation below. There is no automatic first choice,
inline completion, equal-string selection or automatic commit on blur/Tab.
The option's stored value replaces the whole input value; the label stays in
the listbox, with the existing different-value hint. Native sanitization must
preserve the stored string. Native constraint invalidity does not itself reject
a representable commit. A replacement may put the caret at the end.

Initially collapsed, the session may request opening on fresh editor focus or
a settled native text edit. Empty query uses the attachment's all-options match;
there is no minimum character threshold in this profile. Opening needs at least
one current eligible match, editor focus/editability, valid placements and usable
geometry. Ready-empty, no-match and all-disabled results close and clear preview
without changing text. Feedback follows the [source-data lifecycle design](cem-suggestions-data-design.md#loading-failure-and-accessible-feedback).
Hidden/unmatched rows and disabled options/groups are never navigable or
committable; group headings are not options.

A pending preparation can retain an opening intent from that interaction, but
cannot retain a keypress to navigate or commit later. Dismissal invalidates the
opening generation. Render, refit and source completion alone cannot reopen it.
A caret click in an already focused editor neither toggles the popup nor counts
as fresh focus after dismissal. A new edit or admitted arrow request can reopen.

## Keyboard and native editing

The shared input route checks cancellation, composition ownership and the
current editor/session before handling a key. The following commands are
unmodified keys; modified platform shortcuts keep native behavior in this first
profile. The [APG editing guidance](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
informs the focus model; clamping, opening and cancellation below are CEM choices.

| Input | Selected behavior |
| --- | --- |
| ArrowDown / ArrowUp, collapsed and ready | Open and preview first / last eligible option; cancel the native key only when that operation is accepted. |
| ArrowDown / ArrowUp, open | With no preview choose first / last; otherwise move next / previous in source order. Skip unavailable rows and clamp at the ends without wrapping. Repeated arrows may navigate. |
| Arrow with no eligible ready plan | Leave native behavior; preparation may open later without a preview, never replay navigation. |
| Enter with a current eligible active option | Claim and cancel the submission default synchronously, then request the provider commit below. A vetoed or newly stale attempt stays handled and cannot fall through to form submission. |
| Enter with no eligible active option | Preserve the field's ordinary implicit-submit route, except an IME-owned press. Never submit independently from suggestions. |
| Escape with an open session or its pending opening intent | Dismiss once, invalidate that intent and clear preview; preserve text and committed identity. Consume default and handled propagation so the same press cannot close a parent surface. |
| Escape with no session to dismiss | Leave ordinary editor/application behavior; do not implement a clear-text command. |
| Tab / Shift+Tab | Dismiss without commit, allow native focus traversal and ordinary change timing. |
| Text entry, deletion, Left/Right, Home/End and editing shortcuts | Preserve native editing. Actual edits or caret/selection movement clear preview. Do not convert text-editing keys into list navigation. |

Track an Enter/Escape press handled by this session until keyup, loss of editor
focus or binding teardown. Its held repeats cannot submit a form or dismiss a
parent after the first action closes the listbox. Already canceled keys do not
start another action. Key handlers outside this route do not gain a retroactive
veto after a synchronous commit; the cancellable `beforeinput` request below is
the explicit veto point.

Native edits update the field through its original input/change events. Typing,
paste, history operations, public programmatic setters, reset and restoration
clear committed provenance even when the resulting string is unchanged. Native
caret/selection movement alone clears preview, not committed provenance. These
rules require mutation-cause notifications from the provider; string comparison
cannot supply them. Direct writes to the component's private native control are
not an alternate public setter contract.

## Pointer activation and focus

Hover is paint only and changes neither preview nor value. The shared delegate
maps activation to the exact admitted option row, not an index, arbitrary
descendant selector or another producer's output. Primary, unmodified activation
can commit only while the original editor remains focused and editable.

Arm a pointer sequence with the candidate, editor, query, source and placement
revisions. Commit once on its native click after rechecking those revisions.
Pointerdown/up do not also commit. Pointer cancellation, scrolling, drag,
binding loss or row replacement invalidates the sequence; a later click cannot
select the new occupant of the same position. An accessibility activation
without a pointer sequence uses the same current-row/provider request.

Preserve editor focus by suppressing the focus default narrowly on an eligible
row. For a mouse, a primary row pointerdown may be canceled; no popup-wide
cancellation is allowed. Touch/pen pointerdown remains native so scrolling and
gesture recognition work. A compatibility mousedown may suppress row focus
where it occurs, but does not by itself establish portable touch support.
[Pointer Events](https://www.w3.org/TR/pointerevents4/#compatibility-mapping-with-mouse-events)
makes compatibility mouse mapping optional and permits different focus/event
ordering. The need for a verified shared focus-preserving adapter is a design
inference from that boundary, not evidence that a browser already supplies it.

Before claiming a pointer type/browser as supported, fixtures must prove tap
commit retains focus and touch/pen pan remains scrolling. If focus leaves before
an eligible activation, close and reject the stale request; do not refocus after
blur. Blank space, headings and scrollbars do not select or focus a row. Outside
interaction retains its destination focus. Pointer, focus-leave and dismissal
share one arbitration route; independent down/click/blur controllers are not
admitted. This adapter prerequisite is recorded beside implementation/fixtures.

## IME and implicit submission

The field's shared editor lifecycle owns composition state and key-press
arbitration, including its implicit-submit path. Suggestions alone cannot guard
a composition-ending Enter if a deferred form task later loses that context.

On compositionstart, hide the popup, clear preview, invalidate pending opening
and armed pointer requests. Keep native editing, caret and composition defaults.
Composing input still updates the actual field value/submission, and actual
edits clear committed provenance; it does not filter, open or select options.
Compositionstart without an edit does not itself revoke an existing commit.

Guard the composition session, `isComposing` and applicable platform composition
keys, including a terminal press captured before compositionend. An Enter,
Escape or arrow used by the IME does not become a suggestions command or a CEM
form/ancestor-surface action when composition ends. Preserve the IME's own
default; do not cancel composing beforeinput to run suggestions. The deferred
implicit-submit task retains the key's IME-owned decision rather than checking
only a later boolean. End that press guard at keyup/blur/teardown so a new
deliberate press is not swallowed.

After compositionend, one settlement task reads the actual current editor
value, not composition-event data, and prepares a fresh query revision. Final
input arriving later supersedes that revision through the same route. No fixed
millisecond debounce or fabricated input/change is added. A canceled composition
with no edit keeps provenance; a real edit remains an edit even if its final
string equals the prior value. UI Events describes
[composition/key ordering](https://w3c.github.io/uievents/#events-compositionevents),
but native browser/IME fixtures must verify the terminal-key and final-input
order; synthetic KeyboardEvents are insufficient evidence.

## Provider commit and event contract

One synchronous provider operation serves keyboard and pointer. First verify
the prepared candidate, stored string, editable editor and current authority.
An exact repeat of the same accepted source handle and stored string, with no
intervening field edit or relevant source revision/invalidation, is a no-op that
can dismiss; it emits no value events.
Selecting another source node with the same stored string is a new commit and
may establish new provenance. Value equality is not source equality.

For a new commit, dispatch a synthetic, cancellable, bubbling/composed
`beforeinput` from the actual editor, with `inputType="insertReplacementText"`,
`data` equal to the complete stored string and `isComposing=false`. It has no
native editing default; the provider applies the replacement only if uncanceled.
Recheck all revisions after listeners run: they may change the field, source or
binding. On cancellation/rejection the provider writes no value and emits no
value notifications; it preserves any listener-made changes and may keep a
still-current preview. Standard
[Input Events vocabulary](https://www.w3.org/TR/input-events-2/)
supplies these names; this synthetic request is an explicit CEM contract.

Then update native value, field slice/getter, submission and committed provenance
as one synchronous transaction before computing effective validity. Preserve
authored defaults and invalidate older render work. Clear preview and reconcile
actual popup closure before notifying observers. Dispatch one bubbling/composed
synthetic `input` with the same replacement metadata, followed by one bubbling `change`, from
the actual editor. The host/controller does not dispatch another pair. Mark
provider notifications internally so observing them does not reinterpret its
own replacement as ordinary typing and clear the new provenance.

Each notification starts with coherent field state. If an observer makes a new
edit, removes the editor or supersedes the transaction, never roll that change
back to finish the old pair; suppress the old transaction's remaining `change`.
An unchanged binding receives the defined pair. Reentrant edits have their own
normal lifecycle, not a stale success notification carrying their new value.

### Change timing and observation limits

The public field/application bubbling contract acknowledges a completed
replacement immediately.
The provider records its edit revision and change checkpoint. It must reconcile
a later native blur/commit change for that same acknowledged edit without a
duplicate public bubbling change. Suppression is narrow: no intervening real
edit, setter/reset/restore cause, editor replacement or newer transaction may
share the checkpoint. A later edit invalidates it and forwards its own native
change normally. If the provider cannot establish that the checkpoint applies,
forward the native event and report the reconciliation gap rather than suppress
speculatively. Do not use equal-value suppression or a timeout debounce.

Manual notification does not reset the browser's internal change state; HTML
defines [native commit and blur timing](https://html.spec.whatwg.org/multipage/input.html#common-input-element-apis).
The provider must validate its reconciliation in native fixtures. It cannot
promise one event to every raw DOM observer: ancestor capture listeners observe
a platform change before a target/provider can stop it, according to
[DOM event dispatch](https://dom.spec.whatwg.org/#dispatching-events).
Distinguish raw capture traces from the public bubbling trace. Do not silently
claim that no platform event occurred, replace ordinary native events, or
suppress a legitimate later edit to make an event-count assertion pass.

Opening, preview and dismissal emit no value events. Reset, restore and public
programmatic setters retain the existing event-free field contract. Suggestions
never dispatch submit, click a submitter or create a second form owner.

## Undo and redo

Keep native history commands and native typing behavior. Do not install a
suggestions-only undo stack or intercept Ctrl/Meta-Z and historyUndo/historyRedo.
Observe native history input through the provider: value, submission and validity
follow the actual editor, and restored strings do not resurrect source identity.

A programmatic option replacement is not promised to be one native undo entry,
nor to preserve all earlier browser history. Assigning value, calling a range
setter or dispatching synthetic input is not evidence of undo integration.
Record native behavior for immediate undo after commit and for later typing,
undo/redo and caret selection on every supported browser. Field/editor/submission
must remain coherent with the browser's result; no private replay restores an
older source handle. If an undoable replacement becomes a product requirement,
design a field-owned adapter before advertising it. The adjacent action records
that extension and its verification, rather than inferring a guarantee here.

### Undoable replacement assessment (2026-10-08)

Keep the current provider commit and its explicit native-history limitation.
Do not expose an undoable-replacement capability yet. A field-owned adapter is
an appropriate ownership boundary, but a wrapper around the current write does
not establish a browser undo transaction. The
[browser assessment and reproduction procedure](../packages/cem-components/docs/suggestions-acceptance.md#native-history-assessment-2026-10-08)
record the implementations actually tested and the remaining platform coverage.

The alternatives have different constraints:

| Candidate | Assessment |
| --- | --- |
| Original input `value` assignment | Current synchronous commit; retains the input and coherent field/submission state, but provides no single-entry undo guarantee. |
| Original input `setRangeText` | Supplies range and selection control. Its HTML algorithm does not specify adding a native undo transaction; replacing the setter alone cannot justify a stronger contract. |
| Synthetic `beforeinput`/`input` | Retain the existing cancellable CEM request and notifications. Event names describe an edit; dispatching them does not execute the browser's native edit/history default. |
| EditContext | Does not supply a native input undo bridge: its specification restricts attachable hosts and leaves undo to application handling. It would require a different editor model. |
| Private history, deprecated editing commands or an alternate editor | Outside the adopted contract; do not use these to emulate a stronger native guarantee. |

Sources: the [HTML range-replacement algorithm](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#dom-textarea/input-setrangetext),
[Input Events vocabulary](https://www.w3.org/TR/input-events-2/), and
[EditContext host and input handling](https://w3c.github.io/edit-context/).
These API facts support the conservative recommendation; they do not predict
identical history behavior in every browser.

If explicitly requested, reassess supported platform mechanisms before defining
a field-owned adapter. The provider must own admission and the transaction on
the original editor. Admission must cover the actual browser, input type and editing
mode; API presence alone is insufficient. Before committing, recheck editor,
source, composition and revision authority and preserve beforeinput veto and
reentrancy. An unsupported admission must leave the field editable and must not
silently claim an undoable commit. The existing commit remains available under
its existing weaker contract.

Acceptance for any future adapter must prove the replacement is one native undo
unit, prior native typing remains reachable, redo and caret/selection are
correct, and field value, FormData and validity agree after every operation.
Native history input events invalidate committed source proof even when a restored
string equals an option. Test reset, source withdrawal, disable/read-only,
composition and editor replacement too. Do not add a public flag or method until
that implementation and browser/device evidence exist.

## Free text and constrained selection

Free text is the default. In that mode suggestions add no vocabulary validity:
only the existing field's constraints apply. Add two planned host inputs to the
attachment contract:

| Input | Selected contract |
| --- | --- |
| `require-selection` | Presence opts into commit-provenance validity; omission is free text. A literal `false` value still means present. It does not proxy the field's `required`. |
| `selection-message` | Optional nonempty plain-text selection error, default `Choose a suggestion.`; contributes through the field owner. Explicit empty text is a configuration error. |

With an active admitted constrained attachment, a nonempty editor value requires
an accepted original candidate whose committed value and field edit revision
still match. Matching a current option string alone never establishes this
proof. Initial, programmatically assigned and restored nonempty strings stay
visible but are invalid until explicitly committed. No implicit initialization
by source selected flags or matching labels/values is added.

An empty value does not need a commit when the field is optional. Native
`required` still produces valueMissing, including for a committed option with
an explicitly empty stored value. Native pattern/length constraints and author
custom validity still apply to a successful commit. Tab, blur and Escape preserve
invalid text for correction; they never clear text or silently remove the field's
submission. FormData and `.value` retain the same string even while invalid;
normal validated submission uses the existing field's validity route.

Filtering a committed option out of the current display page does not alone
revoke its proof; retain its source owner. Explicit source authority loss or
authoritative invalidation may revoke it without rewriting text. The
[source-data design](cem-suggestions-data-design.md#display-pages-and-committed-provenance)
distinguishes display pages from authoritative vocabularies and defines proof
lifetime; absence from one filtered page is not invalid membership.

Add a leased validity-contributor boundary to the field provider before enabling
this mode. The selection claim supplies its own customError/message without
overwriting author `setCustomValidity` or native constraint flags. For reporting,
an author custom message takes priority, then an applicable native constraint
message, then the selection message. Releasing a claim clears only its own error.
The field's disabled/readonly/validation-barred rules still govern reporting.
Project any invalid/error presentation through the field's owned declarative
contract, preserving authored labels/help/errors; the suggestions producer must
not patch another producer's output or reinterpret an authored `invalid` string.

Attachment teardown releases its validity lease. Persistent domain validation
belongs to the field/schema owner if it must outlive suggestions. Live proof,
listener/ARIA/validity leases and editor handles are not serialized authority;
reset/restore/rebind reacquire a fresh session and do not infer a proof from the
restored string. An equal-value public setter still clears proof.

## Alternatives and verification

| Alternative | Decision and reason |
| --- | --- |
| Automatic first choice or blur/Tab acceptance | Manual selection preserves text and makes acceptance explicit. |
| Copy legacy clear-on-blur constrained behavior | Keep invalid edits visible and contribute validity through the existing owner. |
| Commit label while submitting a different value | Use the stored string for both; label/submission conversion remains a separate field contract. |
| Cancel all pointerdown events or refocus after blur | Use a verified narrow row adapter and native scrolling; reject stale activation. |
| Promise a single raw change or native undo entry from synthetic events | Specify public notification reconciliation and observed native history limits; require evidence before stronger promises. |

The adjacent [implementation and fixture actions](todo.md#autocomplete-and-suggestions-design-for-cem-inputs)
retain the verification scenarios: coherent callback values and defaults,
beforeinput veto/reentrancy, raw capture versus bubbling changes, later genuine
edits, equal-string causes, manual navigation/held presses, native Enter without
selection, nested Escape, mouse/touch/pen focus and scrolling, final IME keys,
constrained/custom/native validity, restored text, native undo/redo and independent
sessions. Data/readiness and loading/status follow the [adopted source design](cem-suggestions-data-design.md).
This document adopts policies; it supplies no browser test or runtime evidence.
