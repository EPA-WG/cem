# Icon buttons and links

`cem-icon-button` supports the legacy `cem-icon-link` interface alongside native
button commands. Its XHTML declaration owns rendering and scoped CSS.

## Navigation and naming

- Absent `href` renders a native `button type="button"`. Its default accessible
  label is Icon action and its default icon name is circle.
- Present `href`, including an empty string, renders a native link. Relative URLs
  and fragments use the live document base. Native click and Enter navigate.
- `label` supplies the accessible name. Without it, link slot content supplies
  the name. Always label icon-only links and buttons. Icons are decorative;
  images have empty alternative text.
- `disabled` uses presence, including `disabled="false"`. A disabled link omits
  href and its activation binding, exposes aria-disabled and leaves the tab
  sequence. Removing disabled restores the authored URL.
- `expanded` forwards aria-expanded. Enabled activation records the `pressed`
  event slice; it does not create a persistent toggle state.

## Icons, content and appearance

`icon` takes precedence over the compatibility `name` attribute. An explicitly
empty icon hides the icon. Removing it restores name or the mode's default.
As in legacy icon-link, a slash selects an image URL; otherwise `fa-` selects
Font Awesome classes; any other nonempty value selects a Material Icons glyph.
The default slot supplies visible text or additional content after the icon.
Applications load their chosen icon font stylesheet; both demos include the
same Material Icons and Font Awesome providers used by the legacy demo.

`direction="row"` is the default; column stacks icon and content in link mode.
Legacy documentation advertised direction without implementing it; this
component now implements both values. Icon and control geometry consume current
CEM theme tokens. Link default, hover, active, disabled and keyboard focus paint
also use current theme tokens, with native forced-color focus outlines.

`kind` accepts the legacy normal, primary, secondary, alert and blend values.
The legacy implementation did not distinguish their colors: all links retain
primary paint. `variant` remains a compatibility class suffix (default quiet);
button mode retains contextual colors. Neither attribute changes navigation.
Host hidden and class retain their existing meaning.

See the [playground](../playgrounds/cem-icon-button.html) and
[full examples and variation matrix](../playgrounds/cem-icon-button-gallery.html).
