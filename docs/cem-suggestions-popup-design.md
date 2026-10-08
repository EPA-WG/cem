# Popup service for attached suggestions

Status: adopted design, 2026-10-07, under the user's instruction to continue
with recommended options. This completes the popup-reuse comparison in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
Runtime implementation and browser verification remain pending.

Use shared cem-elements geometry and surface services through an explicit
suggestions profile. Its presentation is one native `popover="manual"` listbox,
with shared dismissal covering both the existing editor and that listbox.
Retain the editor's focus and the [single field/form owner](cem-suggestions-composition-design.md).
The public dropdown component and its trigger-oriented capability are not the
attachment mechanism. Public attachment syntax is now adopted in the
[attachment/capability design](cem-suggestions-attachment-design.md);
keyboard, pointer and selection policy is adopted in the
[interaction design](cem-suggestions-interaction-design.md).
Static/async data and feedback follow the [adopted source-data design](cem-suggestions-data-design.md).

## Comparison against the current runtime

This decision comes from inspecting executable source, not assuming every
popup abstraction has the same semantics.

| Candidate | Reusable behavior | Conflict or missing contract | Decision |
| --- | --- | --- | --- |
| [cem-dropdown declaration](../packages/cem-components/src/components/cem-dropdown/cem-dropdown.xhtml) with a field in its base slot | Existing panel presentation and generic projected content | Generated button/base, authored `open=true` default, trigger-oriented structure and paint; a field is not a replacement button invoker | Keep the existing component contract; do not use this composition for suggestions. |
| [popup capability](../packages/cem-elements/src/lib/popup-capability.ts), including external `trigger-for` | Visibility, geometry subscriptions and outside/focus dismissal patterns | External endpoints use the [button/link adapter](../packages/cem-elements/src/lib/interaction-reference.ts); click toggling, trigger ARIA claims, panel focus entry and Escape restoration belong to that profile | Extract reusable services while preserving its existing behavior; do not attach the capability unchanged to an input. |
| [popup controller helpers](../packages/cem-elements/src/lib/popup-controller.ts) | Native surface fitting, measured boundaries, geometry observation and style release | `showPopup`/`hidePopup` implement a legacy hidden/style route; `firstPopupControl` is focus policy, not geometry | Reuse and adapt the geometry layer. Keep legacy visibility/focus helpers out of the suggestions profile. |
| [native surface adapter](../packages/cem-elements/src/lib/native-surface.ts) | Exact native visibility owner, lifecycle observation, reference admission and cleanup | Its implemented kinds are dialog and tooltip. Dialog focus/return behavior and tooltip interest/description behavior do not define a listbox | Add an explicit semantic delegate through shared services; assigning a dialog or tooltip kind to suggestions is invalid. |
| Separate autocomplete popup implementation | Could specialize input interactions immediately | Would duplicate positioning, observers, teardown and surface coordination | Implement only the suggestions semantic delegate; share the surface machinery. |

Setting `focus-target=none` on a dropdown does not remove click toggling,
trigger eligibility, ARIA ownership or restoration handlers. Re-focusing the
input after a dropdown opens would also lose the preservation guarantee.
Existing [dropdown stories](../packages/cem-components/src/components/cem-dropdown/cem-dropdown.stories.ts)
expect menu focus entry and restoration; those remain regression contracts for
the dropdown, not acceptance criteria for suggestions.

## Native presentation and dismissal choice

Use a manual popover for top-layer presentation. The listbox stays in its
producer's owned DOM subtree; top-layer painting needs no portal or reparenting.
It is nonmodal and contains no generated trigger, native form control, dialog
owner, autofocus target or menu behavior. Prefer the listbox itself as the
native visibility owner, avoiding another panel solely to hold its role.

The [HTML popover rules](https://html.spec.whatwg.org/multipage/popover.html)
give auto/hint popovers native light dismissal and close requests; manual
popovers require an explicit close route. The `showPopover` source parameter
supplies invocation/anchor information, whereas light-dismiss target discovery
uses native target relationships. A text editor does not acquire the button
target relationship merely by being supplied as a source. Our inference is
that an auto popover cannot be assumed to preserve ordinary clicks in that
separate editor. This design selects manual mode so its own admitted editor
and listbox form one dismissal region without changing the input's native role.

| Presentation alternative | Tradeoff |
| --- | --- |
| Manual native popover with one shared dismissal policy | Selected. It avoids clipping while allowing editor caret clicks and option activation to belong to one interaction region. It requires explicit dismissal and ancestor cleanup. |
| Auto/hint native popover | Not admitted by this profile. Do not cancel or reopen native light dismissal to compensate for an editor outside its native target relationship. |
| Local absolute/fixed panel | Could reuse the old hidden/style route, but clipping, stacking and coordinate behavior would be a different contract. It is not a silent fallback for unavailable native presentation. |
| Dialog or tooltip adapter | Their focus, semantics and lifecycle rules do not match an editable combobox/listbox. |

Native popover support is a prerequisite for this profile. If unavailable,
keep the field editable and diagnose the unavailable attachment; do not expose
a false expanded state. Explicit compatibility presentation can be designed
later with its own evidence. The existing generic popup/menu/dialog/tooltip
profiles retain their own native-mode defaults.

## Shared service boundary

Refactor in the shared runtime before authoring component conveniences. The
neutral service must coordinate an exact native surface and an exclusive
session lease; a profile supplies semantic admission, dismissal regions and
focus policy. This is a required extension, not an API already supplied by
`connectCemNativeSurface`.

| Shared service | Suggestions delegate |
| --- | --- |
| Surface registration, native show/hide observation, revision and disposal | Admit one listbox and one verified text editor; reject competing controllers and incompatible roles/modes. |
| Logical fitting, bounded geometry and observer lifetime | Supply the actual editor as default anchor and choose an attached-listbox placement. |
| One dismissal coordinator with explicit regions and reason propagation | Supply editor plus listbox, selected-session ancestry and keyboard policy. |
| Owned geometry styles and native visibility transitions | Supply declaration-owned listbox paint and derive suggestion session state. |
| Native relationship/placement validation | Claim combobox relationships through the field's editor provider and consume original retained option identities. |

The service does not filter data, choose an option, write a field value, submit
a form, assign menu roles or call `focusSurface`/`restoreSurfaceFocus` for this
profile. Selection and commit remain with the suggestions session and existing
form owner. The suggestions delegate owns combobox/listbox ARIA semantics;
geometry cannot infer `aria-haspopup=menu` from descendants or label the panel
by generating a trigger ID.

One controller owns each actual surface. Registration checks producer identity,
revision and profile; registering another profile on the same node must not
silently reuse the earlier controller. Geometry subscriptions must be isolated
per active surface lease. The current host-keyed `observePopupGeometry` map
cannot simply be registered twice for different panels and assumed independent.
Release listeners and style claims when that lease ends, preserving newer
authored state rather than restoring a stale snapshot over it.

## Placement and visibility

Default to logical `block-end start`, with `block-start start` fallback and
`flip shift resize` fitting under the existing
[interaction placement contract](cem-interaction-design.md#9-placement-and-native-fitting).
Use the actual native editor rectangle, not a wrapper containing label/help
content. Minimum preferred inline size follows that editor, capped by available
bounds; content may need more space. Size the popup and its scrolling option
region without changing the editor or field's border box.

Reuse the logical [surface solver](../packages/cem-elements/src/lib/surface-position.ts)
and the native fitting/measurement helpers. Resolve writing mode and direction
from the admitted anchor. Intersect the visual viewport with an explicitly
selected boundary; mobile viewport offsets and keyboard-induced resize belong
to the same fitting path. Resize before final shift, then measure the actual
panel. Do not add another autocomplete-only collision algorithm. CSS anchors
may supply equivalent geometry when they express the contract; only one
positioning strategy may own a live lease.

Watch relevant ancestor scroll, viewport/visual-viewport changes, editor/panel/
boundary resize and relationship/visibility changes. Coalesce fitting work,
exclude the controller's own writes from observer feedback and invalidate
queued work by session/revision. An editor outside the usable boundary or an
unavailable explicit anchor/boundary closes this attached session. Do not
inherit a task dialog's frozen-last-position behavior. A zero-size/unusable
panel cannot report a successful opening.

Opening validates editor focus/eligibility, source and placement readiness,
panel semantics and geometry prerequisites before calling the native owner.
After showing, fit and recheck the same revision before exposing expanded state
and an active-option relationship. On failure, hide and release that attempt's
claims without changing the field. Reject direct native opening when those
same admission checks fail; opening `beforetoggle` is an available guard.

Actual visibility comes from `:popover-open`, not the requested state or a
delayed `toggle` event alone. Native events can be coalesced, and external
`hidePopover`, ancestor changes or a failed fit must reconcile once with that
actual owner. Clear active references promptly when hiding; `aria-expanded`
must not remain true for an invisible panel. A later render or stale data
completion cannot replay a dismissed opening request. The session records an
opening generation; automatic reopening needs a new qualifying interaction,
not another geometry callback.

Paint, scoped CSS and semantic tokens remain declaration/theme-owned. The
controller owns only geometry and visibility. Native top-layer presentation
does not require copying the legacy helper's hard-coded stacking value or
turning a theme elevation token into a stacking index.

## Focus and close routing

Keep focus on the existing input during opening, preview and commit. Exclude
autofocus and interactive descendants from this listbox profile; an option is
an active-descendant target, not a new native focus stop. The
[combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)
provides the input-focus model. Preserve label, help/error and unrelated ARIA
relationships through the editor provider; only the suggestions session claims
its controls/expanded/active-descendant relationships.

One dismissal coordinator recognizes the exact editor, listbox and admitted
session ancestry. It never treats every field descendant as an invoker. Caret
clicks and text selection in the editor proceed normally and do not toggle or
dismiss. Option activation belongs to the selection delegate, which must
coordinate focus-leave timing without a panel-wide `preventDefault` or a
refocus-after-blur workaround. Scrollbar/touch scrolling is not an option commit.

| Cause | First-profile routing |
| --- | --- |
| Escape with this session open/pending and its editor focused | The shared input route dismisses once without value changes or refocusing. It consumes the handled press's default and propagation, including held repeats, so it cannot also close an ancestor. Apply the interaction design's cancellation and IME ownership rules. |
| Tab/Shift+Tab or genuine focus leaving the session | Dismiss without an implicit commit; preserve destination focus and native change timing. |
| Outside pointer interaction | Shared pointer-sequence policy uses the editor/listbox region; cancel/drag/scroll cases cannot be reduced to two competing pointerdown/click listeners. It never steals outside focus. |
| Accepted option commit | Form-owner value/provenance update succeeds, then the session reconciles one hide before value-event observers. No generic popup listener commits or submits. |
| Disable/readonly, anchor loss, invalidated placement, disconnect or reset/restore | Close and release session claims; retain field-owned value/default behavior. |
| External native hide or ancestor surface closes | Observe and reconcile actual visibility once; do not reopen or synthesize another invocation. |

The [HTML close-request model](https://html.spec.whatwg.org/multipage/interaction.html#close-requests)
lets a handled Escape key prevent the subsequent native close request. The
first profile uses that shared key route; it does not install a competing
`CloseWatcher` for the same Escape. Non-key platform close integration requires
separate design and verification: watcher grouping can close a new child with
an existing parent without a separate activation. Do not claim parent-preserving
Back/dismiss-gesture behavior from the presence of that API.

The runtime tracks the admitted ancestor surface lifetime explicitly. Manual
popovers do not supply automatic transient-parent cleanup. Close child sessions
when their parent closes, their editor becomes inert/hidden or their authority
is revoked, even if the listbox was produced separately. Inside a modal dialog,
the listbox must remain in that dialog's interactive DOM subtree; a placement
grant does not exempt native containment. No body portal is introduced.

## Delivery gaps and verification

This selects the reuse boundary and presentation, not an immediate broad popup
migration. Extract neutral geometry/visibility/dismissal services with regression
cases for current dropdown and native dialog/tooltip behavior, then add the
suggestions delegate. Do not change the old dropdown defaults, menu focus
contracts or public surface kind names to make the new profile appear ready.
The adopted attachment API identifies provider/surface roles explicitly;
its native endpoint, provider lease and source/row view require implementation.

Implementation needs an exclusive per-surface registry, isolated geometry
leases, the editor-provider claim/commit boundary, manual-region dismissal,
ancestor invalidation and generation-safe native visibility reconciliation.
Those gaps and their scenarios are recorded beside the active TODO actions.
Existing geometry tests and dropdown/native-surface stories are source evidence
and regression inputs; they have not been rerun as proof of this new design.

Before release, verify repeated independent fields, input identity/caret/focus
through opening and refits, one commit/close route, pointer scrolling/cancellation,
nested Escape isolation, modal containment and ancestor teardown, RTL/vertical
placement, viewport/container collisions, mobile viewport changes, lost anchors,
style/ARIA cleanup, stale callbacks and fresh placement admission on hydration.
Native popover and browser/assistive-technology support must be reported by the
actual test matrix. Documentation review does not establish that interoperability.
