# Autocomplete source audit

Completed 2026-10-07 for the [autocomplete and suggestions checklist](todo.md#autocomplete-and-suggestions-design-for-cem-inputs).
This audit inventories the pinned source and recommends what the subsequent
composition design should preserve. It handed attachment syntax, filtering
ownership and value semantics to design tasks on that checklist.
The subsequent [adopted composition design](cem-suggestions-composition-design.md)
now settles editor/form ownership and first-profile value/type boundaries;
the [adopted attachment design](cem-suggestions-attachment-design.md) settles
public slots, native adapters and filtering ownership. Runtime delivery,
keyboard/event policy and data lifecycle remain pending.

The published component is a thin wrapper around `cem-input` and `cem-menu`.
Its seven examples establish useful author intent, but the inspected declarations
contain no autocomplete filtering or option-commit implementation. The next
design should preserve those intents while specifying and verifying the missing
behavior.

## Source and evidence

The primary source is
[autocomplete.html from custom-element-dist 0.0.39](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/autocomplete.html).
Its whole-file SHA-256 at inspection was
`4ec9a6dbe3aeec41b1621a95c1f3eb58e2ffa98dcf023f48f63de1507f928665`.
The complete `cem-autocomplete` declaration and all seven demo blocks are
byte-identical to the [local legacy source](../packages/custom-element/material/components/autocomplete.html).
The declaration SHA-256 is
`09364e12553fded7a55ecbd0f0640d1ec726ce905d04fcafeef17d8f52b4e512`.
The surrounding loader, styling and page metadata differ; equivalence applies
to the declaration and demos, not the whole local distribution page.

The audit also inspected the same-version
[input declaration](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/input.html)
and [menu declaration](https://unpkg.com/@epa-wg/custom-element-dist@0.0.39/src/material/components/menu.html).
Using the pinned menu matters: the current local material menu has already
been migrated to the shared runtime.

These are three distinct sources of evidence:

| Evidence | What it establishes | Limit |
| --- | --- | --- |
| Published 0.0.39 declaration and demos | Author vocabulary, native editor intent and example expectations | Static inspection does not prove interactive autocomplete behavior. |
| [Material parity fixtures](../packages/cem-elements/tests/parity/material/README.md) | A minimal legacy declaration and CEM-ML twin, with manifest markers | The twin has hard-coded apple/banana branches; it does not cover the seven examples, grouped filtering or selection transactions. |
| [Implemented autocomplete contract](../packages/cem-components/docs/autocomplete-contract.md), [legacy behavior](../packages/cem-components/src/lib/autocomplete-behavior.ts) and [option normalizer](../packages/cem-components/src/lib/choice-options.ts) | A later standalone, form-associated product with executable browser cases | Its behavior modules are frozen migration debt. Its public contract and adapters do not define the 0.0.39 wrapper or the proposed attachment to an existing field. |

## Declared API

The wrapper declares 27 attributes. They are input-oriented declarations rather
than a separate suggestions API.

| Purpose | Attributes | Recommendation for the next design |
| --- | --- | --- |
| Value and form identity | `value`, `name`, `form` | Keep the existing field/control as the single value and submission owner. Specify how a suggestion commit reaches it. |
| Editor and browser hints | `type`, `autocapitalize`, `autocomplete`, `incremental`, `list` | Respect the actual native input's applicability. Browser autofill and native datalist behavior need an explicit coexistence policy. |
| Availability and constraints | `disabled`, `readonly`, `required`, `multiple`, `min`, `max`, `step`, `minlength`, `maxlength`, `pattern` | Preserve the field's native constraints. Declaring `multiple` does not establish multi-selection suggestions. |
| Native presentation and targeting | `size`, `tabindex`, `id`, `placeholder`, `title` | Preserve authored field behavior and focus ownership. `id` is a DOM attribute, not a requirement that runtime contexts have IDs. |
| Field presentation | `label`, `supporting`, `leading`, `trailing` | Reuse the existing field's label, supporting content and adornments rather than creating another field shell. |

The `input` slot defaults to `cem-input`; the `menu` slot defaults to `cem-menu`.
They show the intent to substitute an editor and a suggestion presentation.
The fallback `cem-input` has no explicit attribute bindings in the wrapper.
Attribute declaration alone therefore does not verify forwarding through that
wrapper, or lifecycle equivalence for arbitrary slot replacements. The new
attachment design must make both ownership and handoff concrete.

The pinned input declaration authors a native input with a `selected` slice
updated by the native `input` event, and forwards input metadata and constraints.
Its value selector, also used by the wrapper, prefers an event-bearing
`selected` slice over authored `value`. This supports the intent that editing
supersedes the initial value, including an edit to empty. Parent slice
propagation and suggestion commit are not established by this selector.

The pinned menu provides a generic flex layout, an unnamed content slot and
link-oriented styling. It supplies no option mapping, filtering, selection
state, popup controller or combobox keyboard semantics. Its name and slot are
presentation history; the current checklist requires an editable combobox with
a listbox. Keeping focus on that input with `aria-activedescendant` follows the
[WAI-ARIA combobox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/).

## Map of the seven examples

The legend column identifies the source demo without introducing fixture-only
IDs. Each behavior column is a preservation recommendation for the later
design, not a claim that the legacy page implements it.

| Source legend | Source shape | Intent to preserve and verification to add |
| --- | --- | --- |
| No value | Empty labeled editor, no suggestions | Normal editing and empty initial state work without a popup. Label presentation stays with the field. |
| No value with placeholder | Number editor with a placeholder | Preserve native type and placeholder behavior when no suggestion set exists. |
| Value defined | Text editor with `value=abc` | Show the initial value; later edits survive suggestion refresh and rerender, including an empty edit. |
| No initial value with placeholder | Fruit `data` nodes with explicit values and emoji labels | Keep stored value separate from visible content; `apple` and the rendered label have different roles. |
| Number, No initial value without placeholder | Number editor with `data` values `1`, `2`, `3` and word labels | Commit an input-compatible value. The design must decide how a word label is presented without assigning it to a number input. |
| Options text as value | Native `option` nodes without explicit values, containing padded text | Provide an explicit text-as-value compatibility path and specify its whitespace and label rules. |
| Options Grouping | Labeled `optgroup` nodes containing options | Retain group boundaries and labels. The description requests filtering within groups and hiding empty groups; the inspected declarations do not implement that algorithm. |

Explicit values are demonstrated on `data` nodes. No published demo in this
file uses an explicit `option value`; that case belongs in later compatibility
verification rather than being reported as existing example coverage. The
[HTML option contract](https://html.spec.whatwg.org/multipage/form-elements.html#the-option-element)
distinguishes an explicit value from the text fallback and defines the display
label separately. The new adapter should specify those distinctions, including
empty explicit values and a `label` that differs from the text.

## Value, filtering and compatibility boundaries

The wrapper does not define a public committed option, active option,
display-value API, selection event sequence or free-text/constrained-selection
policy. Its examples show initial values and value/label separation; its
selector shows edited-state precedence. Those are the preservation inputs.
Reset/restore, native `change` timing, commit events and numeric compatibility
need a complete contract for the existing form owner.

The grouped example describes filtering, but does not choose matching against
value or label, case/locale rules, ranking, query thresholds, or what happens
to an active suggestion removed by filtering. The frozen standalone product
instead accepts an authoritative option set and leaves filtering upstream.
The next shared-capability design must record its filter boundary explicitly;
neither source settles it for reusable attachments.

The frozen product has canonical `cem-option`/`cem-option-group`, mandatory
canonical values, a native option/optgroup adapter, separate submitted and
display values, and free-text/constrained-selection behavior. Its normalizer
does not accept the legacy `data` examples. Its native fallback derives a
missing value from its computed label, which also needs review when an option's
explicit label differs from its text. These are migration inputs, not implicit
acceptance criteria for the new shared capability. This audit does not change
that product's existing contract.

Current [cem-field](../packages/cem-components/src/components/cem-field/cem-field.xhtml)
and [cem-text-field](../packages/cem-components/src/components/cem-text-field/cem-text-field.xhtml)
already use the shared [form-control capability](../packages/cem-elements/src/lib/form-control-capability.ts).
That capability owns one string value and submission on the produced host,
with its native control owning editing and focus. A suggestions attachment must
preserve that owner, native validity, reset/restore and event behavior. Adopting
the old standalone autocomplete's additional form owner would require a
different composition from the one requested on the checklist.

## Recommendations and remaining design work

Preserve the seven source intents: empty and prepopulated editors, metadata
and constraints on the existing editor, explicit stored values distinct from
labels, a documented native text fallback, grouped presentation and filtering
that can hide empty groups. Preserve the ability to compose an editor and a
suggestion presentation; whether the old slot names remain public belongs to
the attachment design.

The extensions requiring new design and verification are keyboard/pointer
navigation, IME handling, commit/cancel, active versus selected state, accessible
popup relationships, asynchronous readiness and stale-response handling,
constraint policy, and shared popup geometry/dismissal. Follow the
[declarative UI principle](declarative-ui-principle.md): implement reusable
behavior in cem-elements and declare component composition in XHTML/CEM-ML.
Suggestion data must cross the retained CEM tree boundary; the frozen record
normalizer is not a replacement runtime input model. Native reference
relationships and any required DOM IDs belong to the consumer lifecycle.

The adjacent actionable checklist items carry these decisions forward:

1. Design attachment to existing controls, including value/label handoff,
   applicable input types and an explicit textarea decision.
2. Compare direct dropdown composition with shared popup-controller reuse,
   preserving input focus and one form owner.
3. Specify attachment/slots, filtering ownership and the shared capability,
   including compatibility paths for legacy data and native options.

Later fixture tasks should map all seven examples and exercise the missing
behavior on the accepted controls. Static source comparison and the material
fixture manifest are the evidence for this audit; they do not establish legacy
browser interoperability or a completed suggestions implementation.
