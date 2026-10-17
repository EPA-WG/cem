# Fieldset contract

`cem-fieldset` groups related controls with a native `<fieldset>` and its first
`<legend>`. Its implementation is XHTML/CEM-ML, with no JavaScript capability.
It composes existing fields, choices and actions without taking their values,
validation, events, focus or persistence ownership.

```html
<cem-element tag="cem-fieldset" src="./cem-fieldset.xhtml#cem-fieldset"></cem-element>
<form>
  <cem-fieldset label="Contact details" description="Choose where to receive updates.">
    <cem-text-field label="Email" name="email" required></cem-text-field>
    <cem-select label="Channel" name="channel" value="email">
      <option value="email">Email</option><option value="post">Post</option>
    </cem-select>
    <cem-action type="submit">Save contact</cem-action>
  </cem-fieldset>
</form>
```

Import the nested controls through their own declarations. Their intrinsic
capabilities remain necessary; preserve their source template/declaration
metadata when copying them. The fieldset adds no form-control capability of its
own and contributes no form value.

| Attribute | Contract |
| --- | --- |
| `label` | Visible legend fallback; default `Group`. |
| `description` | Plain-text group help fallback; default empty. |
| `disabled` | Presence sets native fieldset disabling; empty and `false` spellings both disable. |
| `aria-label`, `aria-labelledby` | Explicit accessible-name overrides on the native fieldset; the visible legend stays present. |
| `aria-describedby` | Explicit external description references; otherwise the component links its own stable, per-instance help ID. |
| `hidden`, `dir` | Native host visibility and inherited text direction. |
| `class` | Host classes for inherited theme and token overrides. |

The `label` slot overrides visible fallback text; the `help` slot overrides help
fallback text. The default slot contains controls. Owned hooks are `part="root"`
on the fieldset, `part="legend"`, `part="body"`, and `part="help"`. Label/help
nodes and edited controls retain identity across attribute updates and reconnects.
The group adds no role override or keyboard tab stop.

Disabled groups omit participating descendants from native form submission and
validation, prevent their native activation and remove them from keyboard order.
Values and explicit child disabled attributes remain intact. Nested enabled
groups cannot override a disabled ancestor. The browser's first-legend exemption
remains intact: a control intentionally projected into the first legend can stay
enabled and participate in its form. Put ordinary controls in the default slot.
The fieldset does not reset, save, validate, Apply or Run a command implicitly.

A group uses existing comfort palette, UI typography, boundary stroke, surface
bend, container inset and group-gap tokens. Legend inset uses related spacing.
All endpoints remain inherited; there are no component-local defaults or token families.
Long legends/help and native fieldset minimum sizing are constrained for narrow
LTR/RTL containers. Forced colors uses Canvas/CanvasText while nested controls
retain their own indicators.

A projected layout/group is a separate DCE owner. Place a `slice` event binding
on the group host when its bubbling events should update the declaring parent;
do not expect a binding authored only on a projected descendant to write the
parent's local slices. Studio retains its operation-option chooser directly in
its command view and uses the fieldset inside the native submit form.

See the [property playground](../playgrounds/cem-fieldset.html),
[full examples and variation matrix](../playgrounds/cem-fieldset-gallery.html),
and [colocated stories](../src/components/cem-fieldset/cem-fieldset.stories.ts).
The shared gallery gate supports `--fieldset-only` to verify the property page,
gallery and release bundle from both repository source and isolated package
archives. It checks retained values, native reset/submission, description
references, all five themes, inherited spacing, narrow LTR/RTL and forced colors.
Automated native form, keyboard, theme, inheritance and forced-color checks do not
establish spoken assistive-technology or physical-device acceptance.
