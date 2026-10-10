# Native tooltip contract

`cem-tooltip` is a canonical XHTML CEM-ML declaration using the shared
`native-surface` capability. One non-interactive `span[part="surface"]` with
`role="tooltip"` and `popover="manual"` supplies both the persistent accessible
description and transient visual presentation.

```html
<cem-tooltip message="Save the current document" placement="block-start center">
  <button slot="trigger" type="button">Save</button>
</cem-tooltip>
```

Use one native button, link, input, select, textarea or summary in `slot="trigger"`,
or a shared control provider. Alternatively `trigger-for` accepts a typed CEM
node reference, ID reference or nearest-scope `@name`. Native `interestfor`
relationships targeting `surface-id` also work. A slotted and external launcher
cannot both be supplied. Native controls keep their focus, value, form, keyboard,
activation and application-event ownership.

| Input | Contract |
| --- | --- |
| `message`, `slot="label"` | Supplemental description; projected non-interactive label content overrides the fallback message. Empty content suppresses presentation. |
| `show-delay` | Pointer interest delay, 500 ms by default. Keyboard focus presents immediately. |
| `hide-delay` | Interest exit delay, 100 ms by default. Escape and invalid/disabled state cancel immediately. |
| `disabled` | Presence suppresses automatic and command presentation; it does not disable the trigger or remove the persistent description. |
| `surface-id` | Optional native owner ID for native interest or command references. Otherwise the runtime supplies a stable ID. |
| `placement` | Shared logical placement; defaults to `block-start center`. |
| `anchor`, `boundary`, `fallback`, `overflow`, `anchor-lost` | Shared geometry and eligibility rules. |

Focus and hover are independent interest reasons. Pointer travel onto the
surface retains presentation. Unrelated DOM updates do not restart interest delays. Escape cancels pending presentation and
suppresses reopening until interest ends. Touch is never intercepted and does
not automatically present the tooltip; subsequent keyboard interaction can.
The native trigger's existing `aria-describedby` tokens are preserved. The
owned tooltip ID survives rerender, and rebinding/disconnect removes only that
claim. The tooltip supplies no tab stop and never moves focus.

Interactive contents are rejected by the shared surface profile. Use
[the native dialog](./dialog-contract.md) for interactive task content. Shared
commands and lifecycle events are available when an explicit controller is
needed; there is no component-local open-state machine.

## Migration from the legacy tooltip

Replace `position` with logical `placement`: `above` → `block-start center`,
`below` → `block-end center`, `before` → `inline-start center`, and `after` →
`inline-end center`. The legacy host `open` attribute is retired; use interest
or shared commands. Set explicit delays if the old zero-delay behavior is
required. Replace `.cem-tooltip__surface` styling with supported `part` hooks.
The duplicate hidden-description element and frozen behavior module are removed.

Colocated stories and shared runtime fixtures cover exact description ownership,
rerender, overlapping interest, delayed cancellation, Escape, native editor
triggers, touch, disabled state, scoped references and cleanup. Source, bundle,
isolated-package and forced-colors verification use `verify-surfaces`.
Manual assistive-technology verification remains separate browser/platform work.
