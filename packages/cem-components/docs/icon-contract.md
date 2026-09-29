# cem-icon

`cem-icon` displays a passive icon and optional adjacent default-slot content.
Use `cem-icon-button` for a command or link; its button glyph size remains
`--cem-icon-button-icon-size` and it owns interaction states.

## Sources and attributes

- `image` takes precedence over `name`, including an explicitly empty value.
- Without `image`, `name` defaults to `circle`. Empty source values hide the glyph.
- Values shorter than three Unicode code points render as Unicode text/emoji.
  Otherwise, a slash selects an image URL, `fa-` selects Font Awesome classes,
  and remaining values select a Material Icons ligature.
- Load the relevant font stylesheet in the consuming document. The component
  does not fetch a font provider stylesheet itself.
- `size` accepts `normal` (default), `small`, and `large`. D2c owns
  `--cem-icon-size: 2rem`, `--cem-icon-size-small: 1rem`, and
  `--cem-icon-size-large: 3rem`; inherited overrides remain effective.
- `direction` accepts `row` (default) and `column`. The latter stacks the icon
  and projected content without a gap. Other values use the row layout.
- `class` is retained on the host. `hidden` uses native presence semantics.

## Accessibility and styling

A nonempty `label` gives the glyph wrapper one `role="img"` and an accessible
name. Otherwise it is decorative (`aria-hidden="true"`). Internal glyphs and
images are hidden from assistive technology; image `alt` is empty to avoid a
second name. Projected content is outside that wrapper and remains accessible.
An empty source removes the image role even when a label is supplied.

The icon inherits color. It has no focus, activation, disabled, pending, or
selection behavior. The containing control owns those states. Stable parts:
`content` for the layout, `icon` for the sized accessible/decorative wrapper,
and `glyph` for the image or font content. Image aspect ratio is preserved.

## Examples and verification

- [Property playground](../playgrounds/cem-icon.html)
- [Full examples and variation matrix](../playgrounds/cem-icon-gallery.html)
- [Colocated stories](../src/components/cem-icon/cem-icon.stories.ts)

This declaration replaces the frozen icon registry entry. Resolve the package
export before appending the template fragment:

```cem
{cem-module-url @slice=iconUrl @src="@epa-wg/cem-components/components/cem-icon"}
{cem:if @test="datadom.slices.iconUrl" |
  {cem-element @tag=cem-icon @src="{$datadom.slices.iconUrl}#cem-icon"}
}
```

The release bundle also exposes `components.xhtml#cem-icon`.
