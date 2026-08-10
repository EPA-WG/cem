# Selectable List Contract

**Status:** Implemented Phase 4 contract. This contract is promoted by
[`docs/todo.md`](../../../docs/todo.md) and is exercised by the focused
`content:selected` browser fixture in
[`states.browser.spec.ts`](../src/lib/states.browser.spec.ts).

## Scope

`cem-list` keeps its current passive native-list behavior by default. The
presence-only `selectable` attribute opts the same host into a visible,
single-select listbox. Selectable tables and multiple selection are outside v1.

Authors declare choices with direct `cem-list-option` children:

```html
<cem-list label="Asset type" selectable value="document" size="4">
  <cem-list-option value="image">Image</cem-list-option>
  <cem-list-option value="document" selected>Document</cem-list-option>
  <cem-list-option value="archive" disabled>Archive</cem-list-option>
</cem-list>
```

`cem-list-option` is parent-scoped payload vocabulary, not an independently
registered component primitive. Before upgrade its text is readable fallback
content; during `cem-list` upgrade it is captured into the inert data island and
normalized into native options.

## Public attributes

| Owner | Attribute | Contract |
| --- | --- | --- |
| `cem-list` | `selectable` | Presence enables listbox mode. Absence preserves the existing passive `<ul>` projection. |
| `cem-list` | `label` | Required non-empty accessible name for the native listbox. |
| `cem-list` | `value` | Optional initial selected option value. It does not make this content component a form participant. |
| `cem-list` | `size` | Number of visible options; integer greater than one, default `4`. |
| `cem-list-option` | `value` | Required, non-empty identity; values MUST be unique within their direct parent. |
| `cem-list-option` | `selected` | Presence marks default selectedness when the parent has no `value`. |
| `cem-list-option` | `disabled` | Presence prevents user selection and exposes native disabled-option semantics. |

Option content is a non-empty text label. Nested links, buttons, inputs, or
other interactive descendants are invalid because a listbox option exposes a
flat accessible name and cannot provide a second interaction model.

## Normalization algorithm

1. If `selectable` is absent, render the existing labeled `<ul>` and project the
   author payload unchanged.
2. If `selectable` is present, read direct `cem-list-option` payload children in
   source order and render a native `<select size>` with native `<option>`
   children. Do not register or upgrade the payload children independently.
3. Copy each option's text, `value`, and `disabled` state. Reflect current
   selectedness through both the native `selected` state and explicit
   `aria-selected="true|false"` evidence.
4. Resolve initial selectedness in this order: the parent's authored `value`;
   otherwise the last child carrying `selected`; otherwise no selection. A
   parent value with no matching child also produces no selection. Multiple
   authored defaults retain the last one, matching native single-select
   normalization, and SHOULD produce a validation warning.
5. A native `change` event writes `$target.value` to the parent's serializable
   `value` slice. Once present, the slice is authoritative for subsequent
   renders and exactly one matching option is selected.
6. Removing or replacing options preserves the current value only while a
   matching option remains. The component does not silently select a different
   option after the selected identity disappears.

The implementation MUST use declarative CEM-ML/CEM-QL data iteration and
`slice-event` wiring. It MUST NOT add component-specific imperative selection or
keyboard handlers.

## Accessibility and interaction

The normalized `<select>` owns native single-selection, focus, pointer,
Up/Down, Home/End, and type-ahead behavior. This follows the
[WHATWG select/option contract](https://html.spec.whatwg.org/multipage/form-elements.html)
and the [WAI-ARIA listbox pattern](https://www.w3.org/WAI/ARIA/apg/patterns/listbox/)
without recreating their keyboard algorithms.

Keyboard focus remains on the native control. The selected option MUST have a
selected treatment that is not merely the control's focus ring, and hover MUST
NOT mutate the value. Selection may follow native keyboard navigation. The
component does not expose `aria-activedescendant`, roving `tabindex`,
`aria-multiselectable`, or nested option controls in v1.

The package's [content hover contract](./content-hover-contract.md) styles the
native `<select>` composite, not its host or individual native options. Passive
lists remain excluded, and pointer enter/leave cannot change selectedness.

`cem-list` remains a Content component, not a form Input component. Listbox mode
does not forward `name`, `required`, `form`, or `multiple`, and it does not
contribute a value to `FormData`; authors needing form submission use
`cem-select`.

## Validation and lifecycle boundary

Schema/package validation MUST reject missing or duplicate option values,
blank labels, a `size` below two, and interactive option descendants. Runtime
normalization omits invalid non-option children rather than placing them inside
the native `<select>`.

`cem-table` remains a static table. Row selection is deferred until the package
can own the complete [WAI-ARIA grid pattern](https://www.w3.org/WAI/ARIA/apg/patterns/grid/),
including composite focus, directional navigation, and row/cell selection. It
MUST NOT be approximated by adding `aria-selected` to static table rows.

## Executable acceptance

The implementation slice is complete only when a focused browser test proves:

- passive-list output is unchanged;
- selectable payload becomes a named native listbox in source order;
- authored `value` precedence and child `selected` fallback;
- disabled option behavior and unique value identity;
- pointer and keyboard selection update the `value` slice;
- exact native selectedness and `aria-selected` reflection;
- visible focus remains distinguishable from selected state; and
- the serialized event payload contains the selected string value.
