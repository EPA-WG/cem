# Popup service for attached suggestions

Status: adopted design, 2026-10-07, under the user's instruction to continue
with recommended options. This completes the popup-reuse comparison in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
Shared geometry, visibility/dismissal sessions and the package-private manual
listbox delegate are implemented. Public suggestions integration remains pending.

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
per active surface lease. The implemented package-private
`createCemSurfaceGeometryLease` isolates each actual panel under a shared host,
rejects competing geometry owners and fences queued resize/mutation/viewport
work when reset or released. Legacy host observers now retain independent
registrations and their panel target sets. Release listeners and style claims
when that lease ends, preserving newer authored values and priorities.
Shorthand claims retain their individual longhands. The transient ownership
registry is shared by runtime copies in one realm; it is not durable authority.

Native dialog/tooltip adapters use those leases and retain their registered
semantic kind. A changed kind requires disconnect and fresh registration.
Geometry alone does not admit an editor or foreign placement, show/hide a
native surface, choose dismissal regions or change focus. The suggestions
manual-listbox delegate now uses the shared session service; coordinated native
bindings remain an explicit runtime action.

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
`CloseWatcher` for the same Escape. The opt-in non-key extension below retains
that route and requires explicit admission; API presence alone does not establish
parent-preserving Back/dismiss-gesture behavior.

The runtime tracks the admitted ancestor surface lifetime explicitly. Manual
popovers do not supply automatic transient-parent cleanup. Close child sessions
when their parent closes, their editor becomes inert/hidden or their authority
is revoked, even if the listbox was produced separately. Inside a modal dialog,
the listbox must remain in that dialog's interactive DOM subtree; a placement
grant does not exempt native containment. No body portal is introduced.

## Non-key platform close requests: adopted design

This opt-in extension belongs to the shared surface/session runtime. The
suggestions controller owns its pending intent before manual-listbox readiness. The host supplies a short-lived
close-request admission tied to the original editor, session generation and
explicit ancestor lifetimes. Component markup, DOM adjacency and reference
values cannot manufacture that authority. Native-datalist remains browser-owned
and does not participate in this extension.

### Browser boundary

HTML groups watchers and can close several together when opening lacks a fresh
activation. Modal dialogs and auto/hint popovers already participate. Canceling
Escape's keydown prevents subsequent watcher processing; watcher cancel events
are not always cancelable. The public API exposes no group identifier or group
reservation query. See the [HTML close-request and watcher algorithms](https://html.spec.whatwg.org/multipage/interaction.html#close-requests-and-close-watchers).

Consequently the host must coordinate watcher creation for the relevant window
and ancestor chain. A trusted event or `userActivation.isActive` alone is not a
proof of independent grouping. An unmanaged native ancestor or another watcher
created during the same activation invalidates any independence claim. Do not
probe grouping by triggering a close request or cancel parents to compensate.

### Admission and asynchronous opening

1. Keep programmatic/focus-only openings on the existing keyboard, pointer and
   focus routes. Platform admission requires an eligible, deliberate trusted
   opening interaction on the original editor and a live explicit parent chain.
2. Within that interaction, before awaiting native queries or rendering, the
   shared host coordinator may create one watcher for the pending session. It
   must know that this activation has not already been used to create another
   watcher or native surface and that the supported browser's activation/grouping
   behavior has been verified. Reserve no watcher at module load or for an
   unrequested future session; create no second watcher when rendering finishes.
3. Bind the reservation to the pending intent and its generation. A platform
   close while pending cancels that intent. Late source/query/render completion
   cannot reopen it. Failed admission leaves ordinary suggestions functional and
   reports platform integration unavailable; it never silently co-dismisses a
   parent to provide child-only behavior.
4. Keep the watcher only while that same intent is pending or visible. Source,
   authority or editor loss, parent close, unsuccessful opening, explicit
   dismissal, reset and disconnect destroy it. Destruction is cleanup, not a
   second close notification. A new opening requires fresh admission; do not
   bank activation or indefinitely retain a watcher for possible later results.

If the host cannot control the relevant watcher creation sequence, it must
decline admission. Browser grouping cannot be repaired retrospectively after an
asynchronous open. Automatic promotion of an already-open session to a platform
watcher is outside this extension; dismiss/reopen under a new deliberate action
instead. No fallback installs history entries or intercepts navigation.

### One dismissal route

The editor lease continues to own Escape, IME fencing and held-press suppression.
A handled Escape dismisses through the existing session, destroys its watcher
and prevents the key default. No watcher handler independently processes keys,
and the generic native-surface adapter must not register a second watcher for
this manual-listbox session. Enter/Tab and pointer behavior remain as specified
in the interaction design.

The watcher's `close` notification calls the same generation-checked session
`dismiss` route with a platform-close reason. It cancels pending work, clears
preview/active descendant and hides once without committing, submitting,
clearing text or moving focus. The event does not identify a specific Back key
or gesture; do not fabricate one. Suggestions do not veto `cancel` events or
try to keep a parent open by canceling its watcher. A close that cannot be
canceled must finish cleanup; it cannot be translated into an application veto.

The host's explicit parent relationships remain responsible for child cleanup
on ancestor loss. They do not authorize replacing an ancestor's native watcher
or bypassing modal containment. An independently admitted child must dismiss
before its parent on a fresh native close request; otherwise that browser/host
combination cannot advertise independent platform close support.

### Implementation and acceptance gates

Shared admission, watcher/session disposal, and editor routing are implemented.
Browser regression evidence and remaining physical-device verification are
tracked in [todo.md](todo.md). Cover pending cancellation and delayed
publication, separately activated modal/auto-popover parents, same-activation
refusal, unmanaged parents, repeated Escape/IME keys, unsupported browsers,
reentrant dismissal and fresh reopening. Verify native close processing in a
browser independently from direct `requestClose()`/`close()` method tests;
method calls alone do not establish group isolation or mobile Back behavior.
No physical platform-support claim is made until those gates pass.

The host API is `createCemPlatformCloseCoordinator(window, admit)`, exported by
`cem-elements`. Supply it as `platformClose` in `suggestionsControllerInputs`.
The callback receives the original keyboard event, editor and explicit ancestor
lifetimes and returns revocable authority (`current`/`subscribe`) only when the
host controls the relevant grouping sequence and has verified that browser.
`consume(event)` marks an interaction already used for another native surface or
watcher; consumption is shared across coordinator instances in the runtime realm.
The host must report all such use, including native surfaces it opens itself.
The runtime cannot discover arbitrary third-party watcher construction.

The first integration admits fresh, unmodified, trusted Arrow Up/Down openings
on text inputs during event dispatch. Focus, programmatic opens, input-driven
refreshes, held/IME keys, and already-open sessions do not acquire a watcher.
The host callback is not a blanket feature-detection switch. An unsupported API,
consumed interaction, denied authority or unmanaged native ancestor reports
`interaction-platform-close-unavailable` while ordinary suggestions remain
available. Native-datalist inputs are explicitly rejected by admission. Every
query replacement releases the old reservation; only a new deliberate opening
can acquire another. Native `close` notifications dismiss once; synthetic close
events confer no authority. Removal/hidden/inert changes also revoke pending
reservations, even if source preparation never completes.

## Delivery gaps and verification

This selects the reuse boundary and presentation, not an immediate broad popup
migration. Extract neutral geometry/visibility/dismissal services with regression
cases for current dropdown and native dialog/tooltip behavior, then add the
suggestions delegate. Do not change the old dropdown defaults, menu focus
contracts or public surface kind names to make the new profile appear ready.
The adopted attachment API identifies provider/surface roles explicitly;
its native endpoint, provider lease and source/row view require implementation.

Geometry leases and their registration/style cleanup are implemented and verified
with independent panels, stale callbacks, runtime copies and existing native
dialog/tooltip/dropdown contracts. The package-private
[session service](../packages/cem-elements/src/lib/surface-session.ts) shares exact
native owner registration and visibility observation with dialog/tooltip adapters.
It fences prepared openings, reconciles actual native visibility and captures
ancestor open lifetimes that cannot revive after a close or disposal. The
[manual-listbox delegate](../packages/cem-elements/src/lib/manual-listbox.ts)
adds the verified text-editor lease, focus/eligibility and native-list conflict
checks, usable geometry, modal containment and editor/listbox dismissal region.
Outside pointer sequences retain their opening generation and ignore cancellation,
drag and scroll. Escape uses the provider's existing composition/handled-press
route; Tab, focus leaving and teardown close without value writes or refocusing.

The delegate requires an explicit trusted lifecycle hook for current relationship
and source/query/row readiness. That hook is a consumer integration boundary,
not a grant created by DOM pointers, IDs, attributes or the surface registry.
Provider/source changes invalidate prepared attempts; a later geometry callback
cannot reopen them. Visibility notifications follow successful native showing,
fitting and revision rechecks, and release synchronously on native hide. The
delegate does not patch editor ARIA. Source filtering, row activation, field
commits and public suggestions declarations remain owned by the later controller.

The delegate can borrow the attachment's exact verified editor lease. The
provider's transient attribute API owns combobox relationships and preserves
newer authored writes; it requires a trusted current-publication hook and never
creates source/placement authority from endpoints or IDs. Coordinated native
reference bindings, source-to-row publication and controller integration of
those claims still need implementation. Those gaps and their scenarios are
recorded beside the active TODO actions. The session browser contracts and existing
logical geometry/dropdown/native-surface regressions establish this prerequisite;
they do not establish full suggestions or accessibility interoperability.

Before release, verify repeated independent fields, input identity/caret/focus
through opening and refits, one commit/close route, pointer scrolling/cancellation,
nested Escape isolation, modal containment and ancestor teardown, RTL/vertical
placement, viewport/container collisions, mobile viewport changes, lost anchors,
style/ARIA cleanup, stale callbacks and fresh placement admission on hydration.
Native popover and browser/assistive-technology support must be reported by the
actual test matrix. Documentation review does not establish that interoperability.
