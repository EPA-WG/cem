# Custom Select Contract

**Status:** Implemented Phase 4 contract. The canonical author vocabulary is
`cem-option` with optional `cem-option-group`; an all-native `option`/`optgroup`
payload is a migration adapter.

## Why the control is custom

Native `select` owns excellent platform behavior but does not consistently
render arbitrary HTML inside its choices. `cem-select` therefore keeps the
native form and interaction contract while rendering its popup/listbox through
CEM-ML and CEM-QL. It does not claim the complete `HTMLSelectElement` IDL.

Rich descendants are carried as serialized payload nodes and materialized by
the generic `cem:project-payload @select="…"` instruction. The browser behavior
adapter owns only focus, keyboard/pointer interaction, selection state, popup
lifecycle, and `ElementInternals`; it does not create option DOM or put live DOM
identity/functions into the data snapshot.

## Author vocabulary

```html
<cem-select name="person" required>
  <span slot="label">Person</span>
  <cem-option value="">Choose a person</cem-option>
  <cem-option-group label="Engineering">
    <cem-option value="ada" selected><strong>Ada</strong> Lovelace</cem-option>
    <cem-option value="grace"><strong>Grace</strong> Hopper</cem-option>
  </cem-option-group>
</cem-select>
```

- `cem-option` is the canonical direct option. `value` is required, including
  when its value is the empty string. `label` optionally supplies the collapsed
  text label; otherwise descendant text is normalized.
- `selected` establishes default selectedness; `disabled` prevents selection.
- `cem-option-group` requires `label`; group `disabled` cascades to its options.
- Static phrasing/formatting HTML is retained. Interactive descendants,
  `tabindex`, and `contenteditable` are invalid inside an option and cause that
  option to be omitted.
- Duplicate values are invalid because values are public option identities; the
  first value wins. Canonical and native vocabularies must not be mixed.

For migration, an all-native `option`/`optgroup` payload is normalized with
native value fallback: a missing `value` derives from collapsed text. Rich
content should use `cem-option` because native option parsing remains
browser-constrained.

## Modes and interaction

| Host shape | Rendered role | Selection behavior |
| --- | --- | --- |
| no `multiple`, `size` absent/`1` | select-only `combobox` plus transient `listbox` | opening previews; Enter, Space, click, Tab, or outside blur commits; Escape restores the pre-open value |
| no `multiple`, `size > 1` | persistent single-select `listbox` | arrows, Home/End, PageUp/PageDown, typeahead, and pointer commit immediately |
| `multiple` | persistent `aria-multiselectable` listbox | arrows move active descendant; Space/click toggles; Shift extends a range; Ctrl/Cmd+A selects all enabled options |

Focus remains on the combobox/listbox and options use
`aria-activedescendant`. Disabled options are skipped. Open dropdowns expose a
truthful `aria-expanded="true"`, `aria-controls`, and active-descendant target;
closed dropdowns omit references to the absent popup.

User commits dispatch bubbling `input`, then `change`. Programmatic setters,
reset, and state restoration dispatch neither event.

## Public form surface

`cem-select` is a form-associated custom element. It exposes the focused
behavioral subset needed by forms and application code:

- `value`, `selectedIndex`, `selectedValues`, and `setSelectedValues(values)`;
- reflected `name`, `disabled`, `required`, `multiple`, `size`, and
  `autocomplete`;
- `type`, `form`, `labels`, `validity`, `validationMessage`, `willValidate`,
  `checkValidity()`, and `reportValidity()`.

Single selection contributes one string. Multiple selection contributes one
`FormData` entry per selected value under the host name. Required validity uses
the browser's localized select validation message. Disabled and nameless
controls contribute no value. Form reset restores authored `selected` defaults;
browser restore/autocomplete state is accepted through the FACE callback.

## Styling and theme ownership

The control reuses the input-indicator appearance contract: underline by
default, outline through `indicator="outline"`, focus in the focus stripe, and
open dropdown state in the selection stripe. Invalidity recolors the anchor and
does not add a fourth geometry role.

Popup and option fills consume generated `--cem-select-*` D0 state tokens. Rows
consume `--cem-list-row-height`; the transient row limit consumes
`--cem-list-popup-rows`; remaining geometry consumes existing CEM families.
Forced colors maps surface/active/selected/disabled states to system colors. No
visual theme exception is open. Physical popup draw order is separately bounded
by `CEM-CSS-002`; D4 overlay tokens remain semantic shadow recipes and are not
used as z-index values.

## Deliberate boundaries

- This is behavioral parity, not a complete `HTMLSelectElement`,
  `HTMLOptionElement`, or live `HTMLOptionsCollection` implementation.
- Option event listeners and JavaScript properties are not serialized or
  cloned. Application interaction belongs on the select, not inside an option.
- Authored option elements are an initialization payload, not a live
  `HTMLOptionsCollection`. The authoritative render consumes that payload;
  post-upgrade DOM insertion/removal is not an option-management API. Use the
  value APIs for selection changes and recreate the control when its option
  source changes.
- Mixed vocabularies, nested groups, interactive option descendants, and
  duplicate values do not receive ambiguous best-effort behavior.
