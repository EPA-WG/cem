# Field controls

`cem-field` and `cem-text-field` are canonical XHTML/CEM-ML declarations with
colocated Storybook tests. Both render a native input in light DOM and request
the shared `form-control` capability from `cem-elements`.

Load template fragments with the capability explicitly:

~~~html
<cem-element tag="cem-field" capability="form-control"
             src="./cem-field.xhtml#cem-field"></cem-element>
~~~

## Attributes and ownership

- `label` supplies the fallback accessible label; `slot="label"` overrides it.
  `slot="help"` supplies help content. Use `describedby` to associate its ID.
- `type` defaults to `text`; use string-valued types such as `email`, `password`,
  `search`, `tel`, or `url`. File and choice controls have separate owners.
- `value` declares the initial/reset value. Changing this attribute replaces the
  live value; input events update the `value` slice without rewriting the default.
- `name` and `form` belong to the form-associated host. The native input is
  unnamed and uses `form=""`, so it neither submits twice nor validates a different
  ancestor form when the host has an explicit external form owner.
- `disabled`, `required`, `readonly`, and `busy` use presence, including an empty
  value or the text `false`. Remove the attribute to clear the state.
- `placeholder`, `invalid`, `describedby`, and `error` project to the native input.
  `invalid` is an ARIA string, not a presence boolean or native custom validity.
- `min`, `max`, `step`, `minlength`, `maxlength`, `pattern`, `autocomplete`,
  `inputmode`, `spellcheck`, `autocapitalize` and `list` forward as native attributes.
  Removing them removes the native binding; the browser applies type-specific
  constraints. An authored native `list` remains its own suggestions route.
- `indicator="underline"` is the default; `outline` selects an outline.
  Unsupported indicator values retain the default.
- Host `hidden` and `class` retain their native meaning.

The host exposes `value`, `defaultValue`, `form`, `labels`, `validity`,
`validationMessage`, `willValidate`, `checkValidity()`, `reportValidity()` and
`setCustomValidity()`. When an owned control is present, value assignment updates
its live value, slice, FormData and validity synchronously and schedules rendering;
it preserves the authored reset default and emits no synthetic value events.

Reset restores the declared default and live value slice. Disabled fieldsets
disable the control. Restored browser state updates the live value without
changing the reset default. No synthetic input/change event is emitted for
these lifecycle operations.

Enter in a single-line control uses its host form's default submit button.
Canceled key/click events and disabled submitters suppress submission. Without
a submit button, multiple text controls suppress implicit submission. This
follows the [HTML implicit-submission rules](https://html.spec.whatwg.org/multipage/form-control-infrastructure.html#implicit-submission),
counting these form-associated hosts as their native text-control equivalents.
Textarea Enter remains ordinary multiline editing.

`busy` preserves editing, focus, selection and geometry while projecting
`aria-busy="true"` and `data-state="loading"`. Indicator precedence remains
disabled, invalid-hover, invalid, pending, readonly, hover, then default.
Both native inputs remain borderless in every state, following the legacy
Material input design. Enabled, unfocused fields show the theme boundary-width
anchor indicator for both empty and populated values. Underline is the default;
outline uses the same resting boundary around the control. Hover changes its
color; focus adds its independent stripe. Forced colors use a system-color
outline without restoring a border.

This follows the filled-field resting affordance in
[Material Design text fields](https://m2.material.io/design/components/text-fields.html)
and [Google's Material 3 filled-field tokens](https://github.com/material-components/material-web/blob/main/tokens/versions/v0_192/_md-comp-filled-text-field.scss):
a 1px baseline uses the on-surface-variant color, with a stronger hover color.
CEM consumes `--cem-stroke-boundary` and `--cem-input-indicator-anchor-color`
rather than hardcoded Material values. Scoped CSS owns the indicator and
forced-color fallback; public theme tokens supply its values.

Use [the field playground](../playgrounds/cem-field.html) or
[the text-field playground](../playgrounds/cem-text-field.html) to exercise
attributes and native reset.

## Indicator examples and required markers

The field, text-field and textarea galleries each include an **Indicator
appearances** demo: default underline, explicit underline, outline and an
unsupported value falling back to underline. `indicator` controls boundary
geometry and fill, independently of the required-field star. Default underline
uses the theme-owned `--cem-input-background-color`; outline overrides that
variable to `transparent` at the native control. Changing back restores the
inherited theme fill. Both appearances preserve
native values and control dimensions when changed.

The local [theme color specification](../../cem-theme/src/lib/tokens/cem-colors.md#73-attribute-channels)
lists the required star under the marker/text channel. Its
[input stripe contract](../../cem-theme/src/lib/tokens/cem-colors.md#77-input-indicator-stripe-colors)
explicitly keeps requiredness separate from indicator stripes. These controls
project `required` to native validation and render a star beside the label.

A declarative `*` appears beside the fallback or projected label in all three
controls when `required` is present, including `required="false"`. Removing the
attribute removes the marker and validation requirement. The marker exposes
`part="required-marker"`, inherits label color, and is hidden from assistive
technology; native required semantics convey the requirement while preserving
the accessible label. Disabled, readonly and busy controls retain the marker.
The **Required-field marker** gallery demo shows optional and required controls,
both indicator appearances, projected labels and presence semantics.

Customize its text with `required-marker="(required)"`; absent attributes use `*`,
and an explicitly empty attribute leaves the marker text empty. Projected
`slot="required-marker"` content takes precedence over the attribute and preserves
markup. Both overrides appear only when `required` is present. All marker content
remains decorative and hidden from assistive technology.

```html
<cem-field label="Email" required required-marker="(required)"></cem-field>
<cem-text-field label="Account" required required-marker="Fallback">
  <strong slot="required-marker">✦</strong>
</cem-text-field>
<cem-textarea label="Notes" required required-marker="✦"></cem-textarea>
```

The **Custom required markers** gallery demos show attribute text, slotted markup,
slot precedence, and optional controls whose supplied marker remains hidden.
The property playgrounds expose `required-marker` alongside `required`.

The marker does not add an indicator stripe or change `indicator`. No new theme
token is needed for inherited text; any separate marker color would need an
explicit theme-owned contract first.
