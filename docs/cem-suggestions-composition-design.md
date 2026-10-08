# Suggestions attached to existing fields

Status: adopted composition design, 2026-10-07, under the user's instruction to
continue with recommended options. Implementation is pending. This document
completes the composition item in the [active checklist](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
It builds on the [source audit](cem-autocomplete-source-audit.md) and the current
[field contract](../packages/cem-components/docs/field-controls-contract.md).
Popup reuse is adopted in the [popup-service design](cem-suggestions-popup-design.md).
Public names, slots and filtering ownership are adopted in the
[attachment/capability design](cem-suggestions-attachment-design.md).
Keyboard, pointer, event/history and constrained-selection policies are adopted
in the [interaction design](cem-suggestions-interaction-design.md).
Static/async sources and feedback follow the adopted
[source-data lifecycle design](cem-suggestions-data-design.md).

Attach a shared suggestions session to the existing field's editor provider.
Keep that field as the sole form and value owner, and its actual native input
as the editing and focus owner. The first custom-listbox profile accepts native
text inputs and commits an option's stored string into that same editor. Rich
labels stay in the suggestion presentation. Textarea completion and additional
native input profiles are separate follow-ups.

## Composition and ownership

Add an editor-provider boundary to the shared cem-elements runtime for the
existing `form-control` capability. The suggestions capability consumes that
boundary; it does not implement a second field. Component declarations express
the attachment, suggestion data and presentation in XHTML/CEM-ML. There is no
component-, gallery- or application-local behavior module.

| Owner | Responsibilities |
| --- | --- |
| Existing field/provider | Authored defaults, live value slice, submitted name/form, validity, reset/restore, native editor identity and value updates |
| Actual native input | Text entry, caret, selection, IME, focus, native input/change behavior and constraint semantics |
| Suggestions session | Its own active/committed option handles, open state, query observation, candidate eligibility and requests to commit through the editor provider |
| Suggestion presentation | Listbox, groups, option labels/content and their produced placements; no submitted control |
| Shared popup service, selected in the popup design | Manual native listbox visibility, shared geometry/dismissal and an editor-preserving focus policy; implementation pending |

There is one attachment per editor. Several fields may consume the same retained
suggestion source, with separate active state, selection and lifecycle contexts.
A presentation session belongs to one editor; switching targets closes and
rebinds it. A shared source is never mutated into one global selected list.

Prefer attachment to the field provider, which supplies its actual editor,
rather than author knowledge of the field's internal DOM. The provider admits
exactly one owned native control. `part="control"` is its existing structural
hook, not authority to search another component's output. A nested field's
control cannot satisfy its parent's editor contract. Missing, ambiguous or
unsupported editors leave the attachment inactive with a diagnostic and the
field operable. Two consumers claiming the same editor are a conflict; mounting
order must not choose a winner.
The provider must account for authored native relationships even when an
existing declaration has not yet implemented their forwarding; an omitted
`list` binding is not permission to ignore an authored native suggestion route.

Local composition can use the supplied owner context and a unique editor
default. An explicit external attachment uses a typed node/provider relationship
under the [reference consumer contract](cem-element-reference-ids-design.md).
Cross-scope and cross-producer access requires the corresponding
[source and placement grants](cem-element-granted-placements-design.md).
An authored ID or a CSS selector neither supplies a grant nor proves identity.
The adopted [local host authorization](cem-suggestions-attachment-design.md#local-host-authorization)
policy lets an explicitly opted-in runtime prepare the local source and issue
exact editor/listbox/row grants through the shared adapter. It remains inactive
without host authorization; external bindings still need their own admissions.
The shared local adapter is implemented, while explicit host hooks remain available.
The consumer may generate DOM IDs for accessible relationships; the runtime
context and suggestion source need no authored root IDs.

Field-owned editor updates and attribute claims go through that provider's
lifecycle. A separate suggestions producer must not install template slice
bindings or patches inside the field's owned output. This preserves the
[nested ownership rule](cem-element-lifecycle-principle.md#nested-custom-element-ownership).
The editor is the popup's anchor candidate and keeps focus; it is not converted
to a button trigger. Existing popup trigger and return-focus behavior cannot be
assumed to fit this attachment.

## Value and label decisions

Preserve the current equality between the field's public `.value`, the native
input's `.value` and its submitted string, after settlement. The source option
retains its own identity and visible label, which may differ from that string.
Committing replaces the whole editor value with the option's explicit stored
value or the compatibility adapter's text fallback. It does not write rich
markup or the option's display label into the editor.

| Suggestion | List presentation | Editor and submitted value after commit |
| --- | --- | --- |
| Explicit fruit value | Rich Apple label and its `apple` value | `apple` |
| Native option with explicit value | Human-readable label and the stored value when different | The explicit value, including an explicitly empty string |
| Native option without value | Text-derived label/value according to the adapter contract | The specified text fallback |
| Numeric legacy example | One with stored `1` | A future numeric profile must write `1`, never `One`; this first profile does not attach to number inputs. |

Present the stored value alongside a different label so the user can predict
what will appear in the editor. If an application wants the human label as its
field value, it can supply that label as the stored value. If it needs a label
in the editor while submitting a different identity, that is an explicit
editor/form-owner extension, with conversion, validation, reset and restore
contracts of its own. This attachment does not create hidden submitted inputs,
override `.value` to mean a different string or borrow the frozen standalone
autocomplete's `displayValue` behavior.

An active option is a preview, not a value update. Only an explicit commit
changes the value. Typing clears the session's committed option handle even
when the resulting string equals an option value. Programmatic updates, reset
and restore also clear selection metadata; equal strings alone never select a
node. Replacing or removing suggestions may invalidate option handles but must
not rewrite the field value. Duplicate-value and source-vocabulary policies
follow the adopted suggestion-data contract; session identity cannot
be reconstructed from strings or array positions.
Provider lifecycle notifications must identify mutation causes; comparing old
and new strings alone cannot distinguish a commit, reset or equal-value setter.

Free text remains the default: suggestions add no validity requirement.
The adopted [opt-in constrained mode](cem-suggestions-interaction-design.md#free-text-and-constrained-selection)
requires explicit commit provenance through a leased field-validity contributor.
Invalid edits remain visible; close never clears text. Its provider boundary
is pending implementation, rather than inherited from the frozen product.

## Commit through the existing form owner

The [shared form-control provider](../packages/cem-elements/src/lib/form-control-capability.ts)
now implements the synchronous user-commit transaction and cause notifications.
Its leases expire on disconnect/editor replacement, and competing claims have
no mounting-order winner. Ordinary setters update live value/submission/validity
synchronously when the control is present, but emit no commit events or source
provenance. Suggestions must still integrate native source/placement admission,
preview/proof state and claimed attributes before exposing the full attachment.

The operation must:

1. Check the current attachment, field/editor revision, retained option and
   admitted placement. The editor must still be connected, editable and
   supported; the option must still be eligible. Commit uses a materialized
   candidate, not deferred I/O or reference evaluation during the key event.
2. Read the candidate's stored string at the retained CEM boundary. Check that
   the native editor can represent it unchanged, using native value
   sanitization without changing the live editor. Reject malformed values,
   such as line breaks stripped by a text input, rather than losing data.
   Native constraint invalidity is distinct: a representable value may be
   committed and leave the field invalid, just as ordinary editing can.
   For a new commit, dispatch the interaction contract's cancellable synthetic
   `beforeinput` and recheck revisions after its listeners. An exact repeat of
   the unchanged committed source is a no-op; another source with an equal
   value may establish new provenance.
3. Update the native value, field-owned live slice, submission value and committed
   source provenance together; compute effective native/contributed validity
   before reporting success. Preserve the authored host `value`/reset default.
   The field owner manages its render revision so a
   queued older render cannot restore the previous value. Generic native value
   patching rules do not change merely because suggestions are attached.
4. Remove the active preview and close the presentation without moving input
   focus. A value replacement may put the caret at the end; preview, refresh
   and unrelated renders may not
   change the user's selection.
5. Emit the defined user-commit events from the actual input after the state is
   coherent: a bubbling/composed replacement `InputEvent`, then one bubbling
   `change`, with no duplicate pair from the host or controller. Objects remain
   synthetic. A superseding edit/disconnect during a callback cancels the old
   transaction's remaining notification rather than rolling newer state back.

A user may type before committing. Manually dispatching `change` does not reset
the browser's change state. The [revision checkpoint contract](cem-suggestions-interaction-design.md#change-timing-and-observation-limits)
requires narrow reconciliation of an acknowledged commit's later native change,
while forwarding genuine later edits. Its guarantee concerns public bubbling
notifications; ancestor capture observers may still see a platform change.
Native browser fixtures must verify that distinction before implementation
claims the event contract.

When Enter requests a current eligible commit, cancel its native submission
route in the same event turn, including when the request is vetoed or becomes
stale. With no eligible active option, retain ordinary Enter behavior apart
from IME-owned presses. The
current form-control capability deliberately waits for canceled key events
before implicit submission; suggestions must coordinate with it instead of
submitting or clicking a form button independently. Opening, navigation,
refresh and dismissal emit no value events. Reset, restore, initial render and
programmatic updates retain the field's existing event-free lifecycle.

## Applicable editors

Admission is based on the actual native editor and provider contract, not its
CEM tag name. The first implementation covers `cem-field` and `cem-text-field`
with effective native `type=text`, including the native fallback for a missing
or invalid type. Other CEM controls may opt in only through the same verified
editor-provider boundary; frozen behavior code is not an automatic adapter.

| Actual editor | First custom-listbox profile | Follow-up |
| --- | --- | --- |
| Single native text input, no `list` attribute | Supported through a verified editor provider | Preserve all existing field and native editing contracts. |
| Text input with an authored native `list` relationship | Inactive with a conflict diagnostic | Keep the native route; do not run two suggestion popups or remove the author's relationship. |
| Search, email, telephone or URL input | Outside this profile | Design a native `list`/datalist profile or another conforming adapter, with its own keyboard and browser evidence. |
| Number input | Outside this profile | Preserve numeric editing and spinbutton semantics; evaluate a native datalist route separately. |
| Password, date/time, range, color, file, choice or non-editable input | Outside this profile | Preserve each editor's own semantic and value contract. |
| Textarea, including `cem-textarea` | Outside this profile | A separately requested multiline/token-completion design must preserve caret ranges, line editing and an appropriate accessible profile. |

These boundaries follow the role allowances in
[ARIA in HTML, 11 August 2026](https://www.w3.org/TR/2026/REC-html-aria-20260811/).
That Recommendation allows custom combobox semantics on text inputs without
`list`; search/email/tel/url inputs have different allowances without `list`.
Number inputs keep spinbutton semantics and textareas keep textbox semantics.
It therefore does not justify assigning combobox roles to every editor. This
is a conformance boundary, not a claim about all browsers' observed behavior.

Textarea is explicitly excluded from this whole-value combobox profile. A
multiline editor needs a separate decision about whole-value versus token/range
completion, Enter and Arrow keys, undo and selection, and accessible feedback.
Changing its role or stealing its ordinary editing keys to reuse this profile
would not preserve the existing control.

Numeric examples remain recorded compatibility intent. A future numeric
adapter must preserve the [native number value rules](https://html.spec.whatwg.org/multipage/input.html#number-state-(type=number))
and numeric constraints. Converting the input to text or writing word labels
into its value is not an implicit migration. The first profile must be labeled
as partial source-example coverage until additional profiles have evidence.

## Editing, availability and lifecycle

Keep the existing input node, accessible label, help/error relationships,
selection and form name/owner through popup changes, suggestion refresh and
theme rerenders. Native typing remains the source of text edits; observe its
original events without re-emitting them. IME, caret shortcuts and text undo
retain native behavior under the [shared interaction arbitration](cem-suggestions-interaction-design.md#ime-and-implicit-submission).
The [history policy](cem-suggestions-interaction-design.md#undo-and-redo) keeps
native commands without promising a programmatic replacement is an undo entry;
native fixtures must record results and field/submission coherence.

Disabled hosts/fieldsets and native readonly state suppress opening and commit.
Transitioning to those states ends the suggestion session without clearing the
field. `busy` is already field-owned presentation that permits editing; a
suggestion source's pending state does not overwrite it. The source-data design
specifies separate truthful suggestion readiness/announcements or an
explicit declarative binding to the existing field state.

The attachment may claim combobox role and its own popup relationships on the
editor only while its validated binding is active. Conflicting authored roles
or another semantic provider disable it rather than being overwritten. Preserve
labels, unrelated description tokens and native attributes. Release only this
session's claims when rebinding or disconnecting, without restoring stale
attributes over newer authored state. Follow the
[combobox focus model](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/): input
focus remains while an active option is represented through its relationship.

The declared attachment and retained source belong to durable CEM ownership;
DOM listeners, placement admissions, open state, active preview and native
editor handles are transient. Disconnect or authority revocation closes the
session and removes its listeners/claims. Reset/restore close it and restore
only the field-owned value. Resume must validate the current editor and obtain
fresh placement authority before enabling relationships. Suggestion refresh
does not create a new input, reset its value or replay its authored default.
If a producer really replaces the editor, invalidate old handles and rebind
only after the new editor is admitted.

## Alternatives and tradeoffs

| Option considered | Decision and reason |
| --- | --- |
| Another standalone form-associated autocomplete | Keep it as migration debt. New composition attaches to the field already responsible for submission, validation and defaults. |
| Wrapper that creates/replaces the input | Rejected for this composition: it would change focus, selection, form ownership and existing field presentation. A layout-only wrapper may not become an editor owner. |
| One capability duplicated for each field tag | Use one provider contract and one shared suggestions capability; both current fields expose equivalent native editing ownership. |
| Default editor label plus separately submitted identity | Defer to an explicit field-owner extension. Value-preserving insertion is coherent with current `.value`, submission and native validation. |
| Automatically convert unsupported types to text | Rejected: changing input type would change validation, editing and accessibility. Authors can choose a text control explicitly. |
| Native datalist for all suggestions | Evaluate as an additional native profile. Rich grouped presentation and shared active-option/popup control need their own design and evidence. |

## Delivery and verification

The checklist promotes this composition contract and the selected popup service.
The attachment/shared capability and interaction policies are now adopted.
The source-data lifecycle and status feedback are also adopted; runtime delivery
and its acceptance fixtures are the next actions.
The provider commit/lifecycle foundation is implemented. Native source/view
transport, attachment claims, suggestions surface/selection and the complete
acceptance matrix remain prerequisites to component attachment conveniences.

Both field declarations now forward native min/max/step, length/pattern,
autocomplete/inputmode/spellcheck/autocapitalize and list bindings, with removal
and native pattern fixtures. Additional profile-specific or legacy convenience
metadata still needs inventory before promising the broader surface. Suggestions
cannot emulate missing native attributes in its own validator or assume
declaration in the legacy wrapper proves current support.

The adjacent fixture actions must verify value coherence during event callbacks,
one FormData entry, original reset defaults, actual editor identity and focus,
empty edits across refresh, group/source identity, rejected attachment conflicts,
disabled/readonly transitions, IME and undo, and fresh bindings on resume.
They must include worker/fallback and source/installed-package paths where
applicable. Those are acceptance scenarios, not tests claimed by this design
change. The source audit, existing field source/contracts and reviewed standards
are the evidence used here; no suggestions runtime is implemented by this file.

## Additional native profile design

The [native-profile design](cem-suggestions-native-profiles-design.md) adopts an
explicit native-datalist follow-up for text/search/tel/url/single-email/number,
with browser-owned presentation and provider-owned list claims over retained
source publications. It does not broaden the delivered custom text-listbox
profile. Implementation and supported-browser/device acceptance remain open in
[todo.md](todo.md). Field conversion, token completion and foreign surface
adoption remain separate capabilities.
