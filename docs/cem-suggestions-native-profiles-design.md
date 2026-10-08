# Native suggestion profiles

Adopted design direction, 2026-10-08, under the instruction to continue with
recommended options. This completes the additional-profile design task in
[todo.md](todo.md#autocomplete-and-suggestions-design-for-cem-inputs). Delivery
remains pending. Existing `cem-suggestions` runtime behavior remains the text-only
custom listbox described in the [composition design](cem-suggestions-composition-design.md).

## Selection and native semantics

Introduce explicit `profile="native-datalist"` on the suggestions attachment.
The omitted profile continues to select the existing custom text listbox; its
explicit spelling will be `profile="listbox"`. An unknown profile suspends
attachment with a diagnostic. Never select another profile merely because the
field changes its input type or acquires an authored `list` attribute.

| Effective native editor | Planned native-datalist admission |
| --- | --- |
| text, search, tel, url | One original provider-owned input; native editing and constraints remain field-owned. |
| email without multiple | Single-address suggestions; native sanitization and type validity remain authoritative. |
| email with multiple | Deferred token-completion admission; diagnose rather than treating an address token as a whole-field value. |
| number | Preserve numeric input and spinbutton semantics; stored numeric strings supply suggestions. |
| Other input types, textarea | Outside this profile; retain the existing native control without suggestions attachment. |

HTML associates an input with a datalist through `list`; a suggestion has a
nonempty value and is not disabled. Browser choice writes its value into the
input through the native interaction. Browser filtering and presentation can
vary. These facts are established by the [datalist definition](https://html.spec.whatwg.org/multipage/form-elements.html#the-datalist-element)
and [input list rules](https://html.spec.whatwg.org/multipage/input.html#attr-input-list).
[ARIA in HTML](https://www.w3.org/TR/html-aria/) permits combobox semantics for
text/search/tel/url/email with `list`, while number retains spinbutton semantics.
This is a conformance basis, not browser/device/AT acceptance evidence.

The proposed consumer adds no custom combobox role, active-descendant,
expanded-state claim or listbox popup for the native profile. The browser owns
its native interface. Preserve field label/help/error relationships, native
constraints, autocomplete hints, focus, caret and submission. Do not install a
second input or rewrite type to text to obtain the custom popup.

## Retained source consumption

Use the same original native owners, occurrence scopes, reference evaluation,
bounds and authorized source handoff as the current suggestions consumer. Source
references are evaluated at the consumer lifecycle stage. A datalist is a DOM
projection of admitted values, not a replacement AST or serialized reference
identity. Original source handles remain available to host diagnostics.

The first native projection admits an ungrouped, complete materialized set from
one supported source family. Canonical options, HTML data, and native options
keep their existing value/text fallback rules. Reject groups and option/group
label templates: native datalist presentation cannot provide the adopted group
and rich-content contract. A caller may explicitly prepare an ungrouped set
upstream; the adapter does not silently discard group boundaries.

Project eligible nonempty values as native options with scalar value/label.
Omit disabled/unavailable rows and explicit empty values; diagnose the omitted
empty values so authors understand that Clear is not a native suggestion. An
empty field remains editable, clearable and subject to its own required rule.
Keep duplicate values in source order without inferring identity from them.
Never write a display label into the input as a substitute for its stored value:
the numeric legacy example supplies `1`, `2`, `3` with One/Two/Three labels.

Do not invent another number/email/URL parser or sanitize source values during
projection. The native input remains responsible for value rules and validity;
browsers may exclude incompatible suggestions. Native-source fixtures must
establish behavior for malformed numeric strings, range/step mismatches and
email/URL values. This policy avoids promising that every projected option will
be selectable, or that selection bypasses native constraints.

Native filtering is exclusively browser-owned. In this profile, authored
`filter`/`filter-by` and query-dependent publication inputs are unsupported and
produce a diagnostic; custom-listbox declaration defaults must not become an
implicit second filter. Start with bounded complete sets and source revision
updates. Query-dependent native-source loading needs a separate design for
browser query freshness and popup update behavior before admission.

## Provider claims and publication

Extend the field-provider lease with a dedicated datalist claim; do not broaden
the existing text-listbox ARIA claim. The proposed shape is
`lease.datalist.set({revision, current, datalist})`, with clear/release and
preservation of later author changes. The claim requires an exact connected
native datalist in the input's tree, a unique consumer-owned ID, the current
provider revision and host-issued placement authority for the `list`
relationship. Local authorization uses the existing explicit runtime policy;
foreign producers require directed grants. An ID or matching DOM adjacency
cannot establish ownership or cross-scope authority.

Any authored `list`, even unresolved or empty, conflicts with a generated
relationship. Preserve it and suspend the attachment; do not replace it, merge
its options, or infer a native-profile switch. An existing bare field/datalist
relationship remains a field/browser concern outside the adapter.

Prepare the native source publication and datalist DOM before publishing the
provider claim. The producer owns `part="datalist"` and its ID; the provider
alone writes the original input's `list`. The native profile produces no
`part="surface"` or custom row controller. Profile changes release the old
claim and controller before the new claim can activate. At most one attachment
lease and suggestion route may own an editor.

Pending/failed source readiness withdraws the owned relationship and old
options. Source replacement atomically publishes fresh options and claims.
Disconnect, provider loss, authored list/type changes, reset/resume and grant
revocation fence old callbacks; resume requires fresh admission. Clear only a
claim's own applied value and never erase a newer authored relationship.

The browser offers no shared-runtime source-row commit proof. Treat its input
and change events as ordinary native edits handled by the existing field owner;
do not synthesize another beforeinput/input/change transaction or committed
native row. Exact string equality, including after reset/restore, cannot prove
selection. Therefore `require-selection`, selection-message and vocabulary
proof policy are unavailable in this first native profile. Independently
designed lexical membership validation may validate a string, but must not be
presented as evidence of a reference selection.

No keyboard/IME, pointer, Escape, submission or close-request interception is
added by this profile. Its count, highlighted row and popup visibility are not
observable through the custom-listbox controller. Reuse only qualified focused
pending/failure source feedback; do not announce browser match counts or expanded
state. Listbox geometry, active-row messages and custom-label APIs stay specific
to the listbox profile. Removing a source cannot retract an ordinary native edit
already accepted by the browser, and a cached native popup choice never revives
source identity or a grant.

## Alternatives and deferred capabilities

| Option | Decision and consequence |
| --- | --- |
| Reuse the custom listbox for every input | Declined: retain the native semantic and value contracts. |
| Native datalist selected explicitly | Adopted direction: one browser-owned route with scalar presentation and no source-row proof. |
| Convert number/email/search to text | Declined: no implicit field type or submission conversion. |
| Field-owned label/submission conversion | Separate future capability, requiring an explicit two-value contract, validation, events, reset/restore and source provenance. |
| Multiline or email-multiple completion | Separate token/range contract, preserving selection, composition, newline/separator editing and history. |
| Full foreign suggestion surface provider | Remains separately designed: exact row mappings, one semantic/visibility owner and fresh grants; not enabled by this native profile. |

## Delivery and verification

The native Rust source projection is delivered as
`NativeCapabilitySession::datalist` and `bind_datalist_frame`, with a separate
view namespace, cached original sources, attributed omission diagnostics and
strict empty control-object parsing. The view exposes scalar option attributes
and original source links, with no selection state or implicit portable export.
WASM/worker exposure, provider claims, declaration integration and browser/device
acceptance remain pending; no additional browser profile is enabled yet.

Implementation actions and adjacent scenarios are retained in [todo.md](todo.md).
Deliver native AST/source cases before shared provider claims and declaration
integration. Prove that rejected profile configuration leaves the field usable.
Use desktop browser fixtures for DOM/claim/event invariants; record native-popup
selection, real IME, mobile keyboard and screen-reader behavior per supported
browser/device separately. A DOM `list` relationship alone is not proof that a
browser exposes its popup or supports choosing a numeric suggestion. Unsupported
native UI leaves ordinary field editing available; it never triggers a custom
popup fallback. Do not close the numeric gallery acceptance scenario until an
actual native choice of One stores/submits `1` without changing input type.
