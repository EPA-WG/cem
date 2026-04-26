# CEM Zebra Supplemental Notes

**Status:** Non-normative companion to [`cem-colors.md`](./cem-colors.md)  
**Last updated:** 2026-04-25  
**Scope:** Supporting rationale, implementation notes, and research references for the CEM zebra outline system.

## 1. Source of Truth

[`cem-colors.md`](./cem-colors.md) is the source of truth for zebra color tokens, including the canonical
`cem-zebra-mode-mapping` table.

This document is complementary information only. It must not be used as the canonical input for token generation,
theme propagation, or compatibility decisions. If this file and `cem-colors.md` ever disagree, `cem-colors.md` wins.

Use this file to preserve:

- why CEM uses zebra rings for focus, target, and selected states
- how the current implementation is expected to be consumed
- what external accessibility and high-contrast references informed the approach
- what checks should be run before changing the canonical mapping

## 2. Current Role Model

The zebra ring is CEM's outline channel for high-salience state. It exists so focus, target, and selected states do not
depend on fill changes alone.

The canonical semantic roles are:

| Token | Role |
|-------|------|
| `--cem-zebra-color-0` | Base / surface anchor stripe |
| `--cem-zebra-color-1` | Focus stripe |
| `--cem-zebra-color-2` | Target / guided-attention stripe |
| `--cem-zebra-color-3` | Selected / chosen stripe |
| `--cem-zebra-strip-size` | Stripe thickness basis, currently `2px` in D5 |

Related local constraints:

- D0 (`cem-colors.md`) owns zebra color semantics and theme-mode mappings.
- D5 (`cem-stroke.md`) owns zebra ring geometry, width, and composition.
- D2 (`cem-coupling.md`) constrains guard spacing so the outer ring does not collide with adjacent controls.
- Contrast themes use a four-stripe ring, with inactive state stripes collapsed to `--cem-palette-comfort`.

## 3. Canonical Mapping Snapshot

The table below is copied from `cem-colors.md` for discussion convenience only. Update `cem-colors.md` first, then
refresh this snapshot if needed.

###### cem-zebra-mode-mapping-snapshot

| Theme mode       | `--cem-zebra-color-0` base | `--cem-zebra-color-1` focus | `--cem-zebra-color-2` target | `--cem-zebra-color-3` selected |
|------------------|----------------------------|-----------------------------|------------------------------|--------------------------------|
| `native`         | `Canvas`                   | `CanvasText`                | `Mark`                       | `SelectedItem`                 |
| `light`          | `--cem-palette-comfort`    | `--cem-palette-trust-x`     | `--cem-color-orange-l`       | `--cem-palette-creativity-x`   |
| `dark`           | `--cem-palette-comfort`    | `--cem-palette-comfort-x`   | `--cem-palette-creativity`   | `--cem-palette-calm-x`         |
| `contrast-light` | `--cem-palette-comfort`    | `--cem-palette-trust-x`     | `--cem-color-orange-l`       | `--cem-palette-creativity-x`   |
| `contrast-dark`  | `--cem-palette-comfort`    | `--cem-palette-comfort-x`   | `--cem-palette-enthusiasm-x` | `--cem-palette-trust`          |

The current markdown-generated zebra implementation satisfies Lighthouse accessibility checks. Treat rendered checks as
the decision gate for future changes, not isolated token-to-surface contrast math alone.

## 4. Implementation Notes

Active zebra colors should remain state-scoped. Theme blocks initialize inactive stripes to the surface anchor, then
state selectors override only the stripe for the active state.

Example pattern:

```css
.cem-theme-contrast-dark,
[data-theme="cem-theme-contrast-dark"] {
    --cem-zebra-color-0: var(--cem-palette-comfort);
    --cem-zebra-color-1: var(--cem-palette-comfort);
    --cem-zebra-color-2: var(--cem-palette-comfort);
    --cem-zebra-color-3: var(--cem-palette-comfort);
}

.cem-theme-contrast-dark :is([focus], :focus, :focus-visible, :focus-within, .focus),
[data-theme="cem-theme-contrast-dark"] :is([focus], :focus, :focus-visible, :focus-within, .focus) {
    --cem-zebra-color-1: var(--cem-palette-comfort-x);
}

.cem-theme-contrast-dark :is([target], :target, .target),
[data-theme="cem-theme-contrast-dark"] :is([target], :target, .target) {
    --cem-zebra-color-2: var(--cem-palette-enthusiasm-x);
}

.cem-theme-contrast-dark :is([selected], .selected),
[data-theme="cem-theme-contrast-dark"] :is([selected], .selected) {
    --cem-zebra-color-3: var(--cem-palette-trust);
}
```

The rendered result is multi-stripe and state-composed, so the accessibility outcome can differ from evaluating one
token in isolation.

## 5. Change Checklist

Before changing zebra mappings in `cem-colors.md`:

- Verify generated CSS still matches the intended state-scoped pattern.
- Run Lighthouse accessibility checks against rendered examples.
- Verify focus, target, and selected remain distinguishable when combined.
- Verify forced-colors behavior separately; authored contrast-light/dark values are not substitutes for OS palettes.
- Confirm D2 guard spacing still covers the maximum D5 zebra ring outset.

## 6. External References

External systems support the current CEM direction: visible focus indicators, system-color fallbacks in forced-colors,
and strong foreground/background pairs plus saturated accents for high-contrast state separation.

- W3C WCAG 2.2 Focus Appearance: https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html
- MDN `forced-colors`: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/forced-colors
- MDN system colors: https://developer.mozilla.org/en-US/docs/Web/CSS/system-color
- Carbon color overview, Focus section: https://carbondesignsystem.com/elements/color/overview/
- USWDS theme color tokens: https://designsystem.digital.gov/design-tokens/color/theme-tokens/
- Primer primitives high-contrast token docs: https://app.unpkg.com/%40primer/primitives%4011.4.0/files/dist/docs/functional/themes/light-high-contrast.json
- VS Code dark high-contrast theme source: https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/hc_black.json
- VS Code light high-contrast theme source: https://raw.githubusercontent.com/microsoft/vscode/main/extensions/theme-defaults/themes/hc_light.json
