# Icon buttons and links

`cem-icon-button` renders native button commands and navigation links. Its XHTML declaration owns rendering and scoped CSS.

## Embedded icon and attribute compatibility

`cem-icon-button` embeds the production `cem-icon` inside its native button or
link. It loads that declaration automatically, including when loaded from the
combined component bundle. Glyph rendering and content layout belong to
`cem-icon`; navigation and interaction remain owned by `cem-icon-button`.

The icon's public attributes are relayed: `image`, `size`, `direction`, `label`,
`aria-label`, `class`, and `hidden`. Default-slot content is projected into the
embedded icon. Host CSS properties, including `--cem-icon-size`,
`--cem-icon-size-small`, and `--cem-icon-size-large`, inherit into it.

**Compatibility note:** `label` now also supplies visible fallback text, as it
does on `cem-icon`. Projected content replaces that fallback. For an icon-only
control, use `aria-label` without `label`; it names both the control and the
embedded glyph. Without `aria-label`, `label` still names the control and the
glyph remains decorative. Glyph sizing now follows `cem-icon` and its tokens;
`--cem-icon-button-icon-size` no longer sizes this embedded glyph.

## Navigation and naming

- Absent `href` renders a native button; `type` defaults to `button`. Its default accessible
  label is Icon action and its default icon name is circle.
- Present `href`, including an empty string, renders a native link. Relative URLs
  and fragments use the live document base. Native click and Enter navigate.
- `aria-label` takes precedence over `label` for the accessible control name.
  Without either, link slot content supplies the name. Always name icon-only
  links and buttons. Glyph accessibility follows `cem-icon`; image elements
  retain empty alternative text.
- `disabled` uses presence, including `disabled="false"`. A disabled link omits
  href and its activation binding, exposes aria-disabled and leaves the tab
  sequence. Removing disabled restores the authored URL.
- Enabled activation records the `pressed` event slice; it does not change
  the container-owned `selected` attribute. `expanded` is not supported.

## Action theme support

The component respects the `cem-action` state model: default, hover, active,
disabled, pending, selected and keyboard focus. Both modes use `variant="primary|explicit|contextual|alternate|destructive"`,
defaulting to `primary`, to select action tokens in all five themes.
Inherited token overrides remain effective. Selection uses the theme's zebra
outline independently of fill and focus; contrast modes use action contours.

- `selected` uses presence, including `selected="false"`. It exposes
  `aria-pressed="true"` on buttons and `aria-current="true"` on links. The
  containing application owns selection; clicks do not toggle it.
- `selectable` uses presence. An unselected selectable button exposes
  `aria-pressed="false"`; an ordinary command omits it. Links keep native
  navigation semantics and never receive `aria-pressed`.
- `pending="true"` exposes `aria-busy="true"` and animated pending paint.
  False or absence clears pending paint. Pending outranks hover, active and
  disabled paint but does not block activation. Use disabled to block it.
- Selection remains visible when disabled or pending. Keyboard focus has an
  independent theme stripe. Reduced motion freezes pending animation; forced
  colors preserve selected, pending and focus indicators with system colors.

## Icons, content and appearance

`image` is the sole glyph source attribute. An explicitly empty image hides the
glyph; removing it restores the mode's default (circle for buttons, no glyph for
links). Use `image` for glyphs; `name` identifies the native form submitter.
Source interpretation belongs to `cem-icon`: values shorter than three Unicode
code points render as text/emoji; otherwise a slash selects an image URL,
`fa-` selects Font Awesome classes, and remaining values select Material Icons.
Applications load their chosen icon font stylesheet.

`size` sets the same minimum control height as `cem-action`: small, medium,
large, x-large and xx-large select their `--cem-control-height-*` tokens.
Absent size or normal inherits control height. The same value is forwarded
unchanged to `cem-icon`: small and large select its size tokens; all other
values use its default icon size. Content and the minimum hit area can make
the control taller. `direction="row"` is the
default; column stacks the icon and content in both button and link modes.
Control geometry and interaction paint consume the action/control
CEM theme tokens. Link default, hover, active, disabled and keyboard focus paint
retain native forced-color focus outlines.

The action bend classes `cem-bend-sharp`, `cem-bend-smooth`, and
`cem-bend-round` set control corners. `size` controls both action height and the embedded icon as described above.
Host `hidden` and `class` retain their native meaning.

## Native forms

Button mode supports `type="button|submit|reset"` and forwards `name`, `value`,
`form`, `formaction`, `formenctype`, `formmethod`, and `formtarget` to its native
button. `formnovalidate` uses presence, including `formnovalidate="false"`.
Submit and reset follow native form behavior; `form` supports an external owner.
These attributes apply only in button mode; `href` selects navigation.

See the [playground](../playgrounds/cem-icon-button.html) and
[full examples and variation matrix](../playgrounds/cem-icon-button-gallery.html).
