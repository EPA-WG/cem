# Autocomplete Contract

**Status:** Accepted as the first Angular Material parity implementation
priority. The primitive, browser behavior, tokenized CSS, focused fixture, and
forced-colors gate are implemented; public inventory and state-matrix promotion
remain deliberately separate.

Benchmark behavior is pinned to the Angular Material `v22.1.1`
[autocomplete guide](https://github.com/angular/components/blob/v22.1.1/src/material/autocomplete/autocomplete.md).

## Owner and boundary

`cem-autocomplete` is a new public, form-associated editable combobox. Its
native `<input>` is the only focus, text-entry, hover, and `focus-visible` owner.
Its transient listbox owns suggestion navigation and selection while focus
remains on that input through `aria-activedescendant`.

This is not an editable mode of `cem-select`. `cem-select` remains a
selection-only owner whose collapsed control is a button or persistent listbox.
The legacy `packages/cem-elements/tests/parity/material/autocomplete.*` fixtures
remain runtime/template compatibility evidence and do not define this product
contract.

## Author vocabulary

The accepted author shape reuses the canonical select option vocabulary:

```html
<cem-autocomplete name="person" required>
  <span slot="label">Person</span>
  <span slot="help">Choose a suggested person or enter another name.</span>
  <cem-option value="ada" label="Ada Lovelace">
    <strong>Ada</strong> Lovelace
  </cem-option>
  <cem-option-group label="Engineering">
    <cem-option value="grace" label="Grace Hopper">Grace Hopper</cem-option>
  </cem-option-group>
</cem-autocomplete>
```

- `cem-option` requires a string `value`. Its `label` is the text written to the
  editable input after selection; when omitted, normalized descendant text is
  used.
- `cem-option-group` requires a label and may disable all direct options.
- `selected` establishes the default committed option; `disabled` suppresses
  pointer and keyboard selection.
- Rich static option content is allowed. Interactive descendants, nested
  groups, duplicate values, and mixed canonical/native option vocabularies are
  rejected under the existing select normalization rules.
- An all-native `option`/`optgroup` payload remains a migration adapter. It is
  not the canonical vocabulary.

The autocomplete accepts live option-payload replacement while connected. This
is a deliberate difference from the current static `cem-select` payload
boundary: declarative CEM-QL or application rendering must be able to replace
the authoritative suggestion set without replacing the input, losing focus, or
mutating the committed value.

## Filtering ownership

`cem-autocomplete` does not guess a filter algorithm. The current authored
option payload is the authoritative suggestion set. Applications may filter it
with CEM-QL, another declarative transform, or application state in response to
the native `input` event. This matches Angular Material's custom-filter
boundary while keeping data transformation outside component behavior.

The component owns normalization, popup lifecycle, active-descendant movement,
selection, and form state. It does not fetch, rank, debounce, or filter data.
`busy` truthfully projects an upstream pending state without disabling text
entry or inventing a loading lifecycle.

## Value and form model

The host is a form-associated custom element. The rendered input does not carry
the submitted `name`, avoiding duplicate form entries. The public surface is:

- `value`: the submitted string. It is free text after ordinary editing or an
  option's string identity after an option commit;
- `displayValue`: the text currently displayed in the native input;
- `selectedIndex`: the committed option index, or `-1` for free text/no
  selection;
- `expanded`: the truthful popup state;
- reflected `name`, `disabled`, `required`, `readonly`, `placeholder`,
  `autocomplete`, `indicator`, `busy`, `require-selection`, and
  `auto-active-first`; and
- `form`, `labels`, `validity`, `validationMessage`, `willValidate`,
  `checkValidity()`, and `reportValidity()` through `ElementInternals`.

Free-form entry is the default. Editing clears option selection and makes
`value` equal `displayValue`. Selecting an option sets `value` to its identity
and `displayValue` to its label. A programmatic `value` matching an option uses
that option's label; an unmatched value becomes free text.

With `require-selection`, editing clears the submitted value immediately while
leaving the user's text visible for suggestion matching. Committing an enabled
option establishes the value. Closing after the user changed the text without
committing clears both value and display text; opening and closing without a
text change preserves the previous committed option. An unmatched programmatic
or restored value resolves to empty. `required` then uses native value-missing
validity. Disabled and nameless controls contribute no form value.

Form reset restores authored `value`/`selected` defaults. Browser state restore
accepts a string and resolves it using the same matching rules. Programmatic
setters, reset, restoration, option refresh, and initial render dispatch no
user-input events.

## Event contract

WHATWG `input` and `change` are sufficient; no autocomplete-specific event is
accepted.

- Native text editing updates the host model before the original bubbling,
  composed `input` event is observable. It dispatches no duplicate event.
- An option commit updates value, display text, selected state, form value, and
  validity, then dispatches exactly one bubbling/composed `input` followed by
  one bubbling `change`.
- Free-form blur retains native `change` timing. Closing an invalid
  `require-selection` edit that clears visible text dispatches the same single
  `input` then `change` pair.
- Popup open/close, active-descendant movement, pointer hover, option-payload
  refresh, and rejected disabled interaction dispatch neither event.

Event processing must use trusted native input as its source and must not
synthesize clicks, focus changes, or mutation events to drive component state.

## Popup and interaction states

| State | Accepted behavior |
| --- | --- |
| default | A labeled native text input is present; no empty listbox or stale ARIA reference is rendered. |
| hover | Only the enabled input or enabled option under the pointer receives hover paint; pointer enter/leave changes no value, active descendant, selection, events, or geometry. |
| focus-visible | Keyboard focus stays on the input and uses the existing input-indicator focus stripe. Opening the popup does not erase it. |
| expanded | Focus, input identity, input/host geometry, value, and selection remain stable. The input exposes truthful `aria-expanded`, `aria-controls`, and, when active, `aria-activedescendant`. |
| active | Exactly one enabled option may be the keyboard active descendant. Active, selected, hover, and disabled states remain independently observable. |
| selected | The committed option has `aria-selected="true"`; reopening may make it active without merging active and selected semantics. |
| disabled | Native input disablement removes focus and suppresses opening/editing. Disabled options remain visible, are skipped, and cannot commit. |
| readonly | The input remains focusable and submittable but neither editing nor option selection opens or mutates the value. |
| loading | `busy` projects `data-state="loading"` and `aria-busy="true"` onto the input without disabling it or changing dimensions. |
| invalid/required | The input exposes native/form-associated validity and stable help/error references. Invalidity, focus, open, and hover indicators coexist. |
| empty suggestions | The popup closes or remains absent; it never exposes `aria-controls`/`aria-activedescendant` to missing nodes. |

Popup open/close must not alter the input or host border box. The popup is an
anchored overlay and scrolling is confined to its option region.

## Keyboard contract

Focus always remains on the native input.

| Key | Behavior |
| --- | --- |
| `ArrowDown` | Open when suggestions exist, then move to the next enabled option. |
| `ArrowUp` | Open when suggestions exist, then move to the previous enabled option. |
| `Enter` | Commit the active enabled option and close. With no active option, do not prevent native form behavior. |
| `Escape` | Close without committing an active preview. Apply the accepted `require-selection` close rule when the text changed. |
| `Alt+ArrowDown` | Open when enabled suggestions exist. |
| `Alt+ArrowUp` | Close without committing. |
| `Tab` / `Shift+Tab` | Move focus normally and close without implicitly committing the active option. |
| `Home`, `End`, printable keys, editing shortcuts | Preserve native text-input caret and editing behavior; do not treat them as listbox navigation. |

Opening starts with the committed enabled option active. Otherwise no option is
active unless `auto-active-first` is present, in which case the first enabled
option becomes active. Disabled options are always skipped. Pointer activation
keeps focus on the input and commits exactly the clicked enabled option.

## Accessibility contract

- The native input has `role="combobox"`, `aria-autocomplete="list"`, an
  accessible label, and truthful popup relationships.
- The popup has `role="listbox"`; options use `role="option"`,
  `aria-selected`, and `aria-disabled`; labeled groups use `role="group"`.
- `aria-activedescendant` points only to a connected enabled option while the
  listbox exists. Closing removes `aria-controls` and `aria-activedescendant`.
- Interactive content inside options is invalid because focus must remain on
  the combobox.
- The committed selection remains visually identifiable. Active and hover
  feedback cannot erase selected state, and no state relies on color alone.
- Help, description, error, required, readonly, disabled, invalid, and busy
  relationships follow the existing field/select contracts.

## Theme-token audit

No missing visual theme category was found. Implementation subsequently
discovered that D4 intentionally excludes numeric physical draw order; the
bounded `CEM-CSS-002` component exception supplies that non-semantic adapter for
both select and autocomplete popups.

- The input reuses the D0 `--cem-input-indicator-*` state colors and D5
  underline/outline, pending, focus, and selection stripe geometry.
- The shared choice popup reuses the existing `--cem-select-popup-*`,
  `--cem-select-option-*`, and `--cem-select-group-text` semantic endpoints.
  Their theme specification is broadened to cover both select and autocomplete
  listbox owners before component CSS is added.
- Popup structure uses `--cem-stroke-standard`, `--cem-bend-overlay`,
  `--cem-elevation-3`, `--cem-list-row-height`,
  `--cem-list-popup-rows`, control padding, gaps, and UI typography.

The historical `--cem-select-*` prefix remains a stable public token name; its
accepted semantic category is now a shared choice-popup/listbox state family,
not a license to borrow unrelated component paint.

`--cem-layer-overlay` continues to represent semantic overlay elevation through
its shadow recipe and MUST NOT be assigned to `z-index`. Physical popup
stacking uses only the private, verifier-bounded adapter accepted in
`components-css-exceptions.md`.

## Forced-colors boundary

The focused Chromium fixture must prove:

- input shadows collapse; hover uses `Highlight`; pending and focus retain
  their D5 widths with `CanvasText`;
- popup surface/border/text use `Canvas`/`CanvasText`;
- active and pointer-hover options use `Highlight`/`HighlightText`;
- selected options retain an inset `SelectedItem` outline even when also active
  or hovered; disabled options use `GrayText`; and
- focus, expanded, invalid, selected, active, hover, and disabled precedence
  does not change geometry or component state.

## Focused fixture and assertion matrix

Implementation must first add
`packages/cem-components/tests/autocomplete/contract.html` and a focused browser
suite. The fixture must contain ordinary free-form, `require-selection`,
disabled, readonly, busy, invalid/required, grouped/rich-option, disabled-option,
preselected, empty-suggestion, and live-option-update cases.

The executable matrix must assert:

1. light-DOM/native input ownership, form association, default/reset/restore,
   accessible name, and ARIA reference integrity;
2. focus-open/input-open/Alt-open and Escape/Alt-close/outside/Tab close paths;
3. ArrowUp/ArrowDown skipping disabled options, optional first activation, and
   Enter/pointer commit with exact input/change event counts and ordering;
4. free-form versus `require-selection` value/display/validity transitions;
5. dynamic option replacement without input identity, focus, committed value,
   host/input geometry, or event mutation;
6. pointer hover ownership, selected/active/hover coexistence, disabled
   suppression, and focus-visible/open coexistence;
7. token-resolved normal-mode paint and the complete forced-colors boundary;
8. state-matrix promotion of `cem-autocomplete` across `input:default`,
   `hover`, `focus-visible`, `disabled`, `loading`, `expanded`, `invalid`,
   `required`, and `readonly`; and
9. parity inventory promotion only after every accepted assertion is executable.

## Deliberate boundaries

- This is behavioral parity, not Angular directive/service/API compatibility.
- Filtering, ranking, fetching, and debouncing remain upstream data concerns.
- Option values are strings; functions, object identity, live DOM identity, and
  `displayWith` callbacks do not cross the serializer-free CEM boundary.
- The first contract anchors the popup to its own input. Arbitrary alternate
  origins and overlay-service APIs are not semantic owners in this slice.
- No component CSS may land before the focused fixture and matrix entry. No CSS
  exception may be recorded unless implementation discovers a requirement that
  the accepted theme categories cannot represent.
