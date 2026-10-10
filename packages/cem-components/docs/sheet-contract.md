# Sheet region contract

`cem-sheet` is a named, focus-neutral `<aside role="region">`, authored in
[canonical XHTML](../src/components/cem-sheet/cem-sheet.xhtml). It needs no
imperative capability. Use [cem-dialog](./dialog-contract.md) for native task
presentation, commands, modality, dismissal and focus management.

Without `transient`, the region is persistent and ignores `expanded`. With
`transient`, `expanded` shows it; absence hides it through native `hidden`.
Both are presence flags: empty values and the text `false` count as present.
Host `hidden` independently hides the entire component.

`label` defaults to `Sheet`, supplies a visible heading and preserves the
legacy default region name. The named `label` slot replaces the visible heading
content, preserving markup. Separate `aria-label` overrides the region name;
`aria-labelledby` takes precedence and can reference a projected heading with
an authored ID. `aria-describedby` references supporting instructions. These
attributes belong on the sheet and are forwarded to the region. The default
slot is the retained body. Stable styling hooks are `part="surface"`,
`part="heading"`, `part="body"` and projected `slot="label"` roots.

Opening does not autofocus, closing does not restore focus, and Escape does
not dismiss the region. The sheet creates no tab stop, focus trap, inert
document, overlay, launcher, close control or dismissal events. Applications
own their visibility controls, any opener `aria-controls`/`aria-expanded`, and
focus recovery before hiding a focused descendant. Native controls keep their
document tab order, form values, validation, submission and reset behavior.
Visibility and label updates retain the region, body nodes and draft state.

The [property playground](../playgrounds/cem-sheet.html) and
[five-theme gallery](../playgrounds/cem-sheet-gallery.html) cover labels,
projection, ARIA references, persistent/transient combinations, application
control, forms, direction and native host visibility. Colocated
[stories](../src/components/cem-sheet/cem-sheet.stories.ts) own component tests,
including all boolean-presence combinations, native keyboard input, and
worker/fallback identity and reconnect. The shared playground gate verifies
source, bundled and packed consumers plus forced colors.
