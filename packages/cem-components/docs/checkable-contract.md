# Checkbox, radio and switch contract

The canonical `cem-checkbox`, `cem-radio` and `cem-switch` declarations each
produce one native input inside a wrapping label. Checkbox and switch use
`type="checkbox"`; switch adds `role="switch"`. Radio uses `type="radio"`.
The inputs own keyboard behavior, focus, checked state, validity and forms.
The component hosts add no tab stop or second form-associated value.

`label` supplies visible fallback text (`Checkbox`, `Radio` or `Switch`). The
default slot replaces that text and preserves markup. Separate `aria-label`
and `aria-labelledby` name the input, with `aria-labelledby` taking precedence.
`aria-describedby` and `aria-errormessage` reference authored help and errors.
When `required` is present, the visible `required-marker` defaults to `*`; its
named slot replaces the marker. The entire marker is `aria-hidden="true"`.
Required semantics come from the native input, never from adding the marker to
its accessible name. The shared test harness uses the standard accessibility
name algorithm, including hidden-marker exclusion and explicitly referenced
hidden names; workflow assertions continue to require `Email`, not `Email *`.

`checked`, `disabled`, `required` and `busy` use presence semantics: empty and
`false` values count as present. `checked` controls the native default and
updates live checked state when its presence changes. Native activation or a
direct input property edit remains intact on unrelated rerenders. Reset restores
the authored default; canceled reset leaves the live state intact. `busy` only
exposes `aria-busy` and pending paint, so it does not disable interaction.
`invalid="true"` exposes authored validation feedback separately from native
constraint validity. Native checkable inputs do not support readonly.

`name`, `value` (default `on`, including an explicit empty value), and `form`
forward to the native input. An explicit `form=""` leaves it unassociated,
even inside a form; an absent `form` uses its ancestor form. Checked, enabled, named controls contribute one
value to their form. Fieldset disabling, required validity, external form
ownership and reset use native rules. Radios with the same name and form owner
share a browser group, including ordinary native radio peers; equal names in
separate forms remain independent. Arrow keys skip disabled peers. Rerendering
an unchecked peer does not restore an obsolete selection.

Checkbox alone uses the shared `checkable-control` capability for
`indeterminate`. Presence, including the legacy spelling `indeterminate="mixed"`,
sets the native property. No conflicting `aria-checked` override is emitted.
Activation clears mixed state; unrelated renders and reconnects preserve that
edit. To reapply the mixed state, remove and readd the attribute. Radio and
switch do not implement mixed state.

Native input/change events bubble from the actual input. Applications can bind
`slice-event="change"` and `slice-value="$target.checked"` or `$target.value`
on the host/ancestor. The controls no longer bind an internal checked slice that
would rewrite form defaults or replay an obsolete radio selection. No component
events are synthesized. Host `hidden`, `class` and `dir` retain their native
meaning; public styling hooks are `part="root"`, `part="control"`,
`part="label"` and `part="required-marker"`.

Embedded token styles preserve `indicator="outline"` (default), `underline`
and the outline fallback for unsupported values. Native appearance remains
enabled. Radio paint uses a pointer-transparent circular pseudo-element around
the native dot. Focus, selection, pending and invalid indicator layers compose
without changing geometry; forced colors use system outlines. Colocated stories
cover five themes, worker/fallback forms, native keys, exact stripe geometry and
colors, projection, mixed state, native radio peers and application bindings.

Each declaration ships a property playground and five-theme gallery. The shared
playground gate verifies source and isolated-package consumers, generated bundle
metadata, native forms, accessibility names and forced colors.
