# cem-textarea

`cem-textarea` is the multiline counterpart of `cem-field` and `cem-text-field`.
It uses the shared `form-control` capability and renders a native textarea in
light DOM. `cem-action` supplies submit and reset commands.

## Attributes and slots

- `label` defaults to `Textarea`; `slot="label"` replaces the visible label.
- `slot="help"` supplies help content. Set `describedby` to its ID or another
  description ID; `error` supplies an `aria-errormessage` ID.
- `value` supplies the initial/reset value. The host `value` property is live;
  `defaultValue` gets/sets the authored value. Input updates the value slice.
- `name` and `form` belong to the host. It submits one value through its form
  internals; the native control has no submission name or form owner.
- `rows`, `cols`, `minlength`, `maxlength`, `autocomplete`, and `placeholder`
  forward to the native textarea. Without dimensions, browser defaults apply.
- `disabled`, `required`, `readonly`, `busy`, and `hidden` use attribute presence,
  including the string `false`. A disabled containing fieldset disables the control.
- `invalid` forwards the ARIA string; it does not impose custom validity.
- `indicator` defaults to `underline`; `outline` selects an outline indicator.
  Both appearances keep the native border removed. `class` stays on the host.

## Editing and forms

Enter inserts a newline. It does not trigger implicit form submission. Busy
preserves editing, focus, selection, and value; it reports `aria-busy="true"`
and the retained input-family `data-state="loading"` styling hook. It does not
own an async operation or introduce a status announcement.

The form capability supports native validation, `checkValidity`, `reportValidity`,
`setCustomValidity`, external form association, reset, disabled fieldsets, and
state restoration. Readonly text remains focusable and submittable. The named
host owns form submission, so the inner textarea never produces a duplicate entry.

## Styling and demos

The embedded scoped CSS consumes theme input-indicator tokens. Enabled,
unfocused textareas show a boundary-width resting underline for empty and
populated values; `indicator="outline"` uses a resting outline. Hover, focus,
readonly, invalid, busy, and disabled styles follow the canonical text field.
Forced colors replace decorative shadows with system-color outlines. Native
textarea resizing remains available. Stable parts: `root`, `label`, `control`,
`help`, and `required-marker`. Required presence also shows an accessible-name-neutral
star beside the fallback or projected label, independently of indicator geometry.

- [Property playground](../playgrounds/cem-textarea.html)
- [Full examples and variation matrix](../playgrounds/cem-textarea-gallery.html)
- [Colocated stories](../src/components/cem-textarea/cem-textarea.stories.ts)

Load `components/cem-textarea` through the module URL loader, appending
`#cem-textarea` to the resolved URL and declaring `capability="form-control"`.
The release bundle also exposes `components.xhtml#cem-textarea`. The legacy
primitive registry no longer registers this tag.

Required marker text defaults to `*`. Override it with `required-marker` or provide
markup in `slot="required-marker"`; the slot takes precedence. Visibility still
follows `required` attribute presence. These decorative overrides preserve the
accessible label and native required validation.
