# CEM ↔ Material Design 3 / Angular Material / MUI Token Parity

**Status:** Canonical
**Last updated:** 2025-12-23
**Audience:** Design Systems, Product Design, Front-End Engineering

---

## 1. Intent and positioning

CEM is **not** designed to be an in-place replacement for Material Design 3 (M3), Angular Material, or MUI token systems. Instead, CEM defines **consumer-semantic** system endpoints across a **7+3 dimensional framework** (D0–D7 plus D1x/D1y extensions) that are sufficient to build a **functional UI** grounded in CEM principles, while allowing selective adoption of M3/Angular Material/MUI values and conventions via a **mapping/alias layer**.

This document records a comprehensive coverage review of the complete CEM spec set:

| Dimension | Spec file | Coverage area |
|-----------|-----------|---------------|
| D0 | [`cem-colors.md`](./cem-colors.md) | Color — emotional palettes, action intents, state progression, zebra outlines |
| D1 | [`cem-dimension.md`](./cem-dimension.md) | Space & Rhythm — spacing scale, gaps, insets, gutters, layout rhythm |
| D1x | [`cem-breakpoints.md`](./cem-breakpoints.md) | Breakpoints — window size classes (compact/medium/expanded/large/xlarge) |
| D1y | [`cem-responsive.md`](./cem-responsive.md) | Responsiveness — strategy vocabulary (intrinsic/container/breakpoint/hybrid) |
| D2 | [`cem-coupling.md`](./cem-coupling.md) | Coupling & Compactness — zone/guard/halo, control geometry, density modes |
| D3 | [`cem-shape.md`](./cem-shape.md) | Shape & Bend — corner radius, bend tokens, semantic endpoints |
| D4 | [`cem-layering.md`](./cem-layering.md) | Layering — signed 7-tier depth ladder, semantic planes, representation channels |
| D5 | [`cem-stroke.md`](./cem-stroke.md) | Stroke & Separation — boundaries, dividers, focus/selection/target indicators |
| D6 | [`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md) | Typography — fontography, voice (ink/speech), semantic roles |
| D7 | [`cem-timing.md`](./cem-timing.md) | Timing & Motion — durations, easings, spring presets |

---

## 2. M3 coverage matrix

Material Design 3 exposes **system tokens** (color/typescale/shape/motion) and **state guidance** (state-layer behaviors, opacities). This section provides a detailed coverage analysis.

### 2.1 Color system

| M3 token surface | CEM coverage | CEM spec | Notes |
|------------------|--------------|----------|-------|
| `md-sys-color-primary` / `-on-primary` | ✓ **Covered** | D0 | `--cem-palette-trust` / `--cem-palette-trust-text` |
| `md-sys-color-secondary` / `-on-secondary` | ✓ **Covered** | D0 | `--cem-palette-enthusiasm` or `--cem-palette-calm` |
| `md-sys-color-tertiary` / `-on-tertiary` | ✓ **Covered** | D0 | `--cem-palette-creativity` |
| `md-sys-color-error` / `-on-error` | ✓ **Covered** | D0 | `--cem-palette-danger` / `--cem-palette-danger-text` |
| `md-sys-color-surface` / `-on-surface` | ✓ **Covered** | D0 | `--cem-palette-comfort` / `--cem-palette-comfort-text` |
| `md-sys-color-surface-container-*` (levels) | ✓ **Covered** | D0 + D4 | Surface containers via layering tonal shifts |
| `md-sys-color-inverse-*` | ✓ **Covered** | D0 | Via `--cem-palette-*-x` extreme variants |
| State layer opacities (hover/focus/pressed/drag) | ✓ **Covered** | D0 | Formulaic `color-mix()` state progression (§7.2) |

**Crosswalk example:**
```css
--md-sys-color-primary:    var(--cem-palette-trust);
--md-sys-color-on-primary: var(--cem-palette-trust-text);
--md-sys-color-error:      var(--cem-palette-danger);
--md-sys-color-surface:    var(--cem-palette-comfort);
```

### 2.2 Typography system

| M3 token surface | CEM coverage | CEM spec | Notes |
|------------------|--------------|----------|-------|
| `md-sys-typescale-display-*` (L/M/S) | ✓ **Covered** | D6 | `--cem-typography-brand-*` semantic role |
| `md-sys-typescale-headline-*` (L/M/S) | ✓ **Covered** | D6 | Mapped via D6 size scale + voice semantics |
| `md-sys-typescale-title-*` (L/M/S) | ✓ **Covered** | D6 | `--cem-typography-ui-*` role |
| `md-sys-typescale-label-*` (L/M/S) | ✓ **Covered** | D6 | `--cem-typography-tag-*` role |
| `md-sys-typescale-body-*` (L/M/S) | ✓ **Covered** | D6 | `--cem-typography-reading-*` role |
| Font weight / optical sizing | ✓ **Covered** | D6 | Voice → ink thickness projection |
| Letter spacing policies | ✓ **Covered** | D6 | `--cem-typography-letter-spacing-*` |
| Numeric/ligature features | ✓ **Covered** | D6 | `--cem-typography-feature-*` tokens |

**CEM differentiation:** CEM defines typography by **semantic role** (reading/ui/tag/data/script/initialism/iconized/brand) rather than M3's size-ladder taxonomy. This provides stronger consumer semantics while supporting M3 value mapping.

### 2.3 Shape system

| M3 token surface | CEM coverage | CEM spec | Notes |
|------------------|--------------|----------|-------|
| `md-sys-shape-corner-none` | ✓ **Covered** | D3 | `--cem-bend-sharp` (0) |
| `md-sys-shape-corner-extra-small` (4dp) | ✓ **Covered** | D3 | `var(--cem-dim-xx-small)` via adapter |
| `md-sys-shape-corner-small` (8dp) | ✓ **Covered** | D3 | `--cem-bend-smooth` |
| `md-sys-shape-corner-medium` (12dp) | ✓ **Covered** | D3 | `--cem-bend-surface` |
| `md-sys-shape-corner-large` (16dp) | ✓ **Covered** | D3 | `var(--cem-dim-medium)` via adapter |
| `md-sys-shape-corner-extra-large` (28dp) | ✓ **Covered** | D3 | `--cem-bend-modal` |
| `md-sys-shape-corner-full` (pill/50%) | ✓ **Covered** | D3 | `--cem-bend-round` (capsule) / `--cem-bend-circle` |

**Crosswalk (adapter layer):**
```css
--md-sys-shape-corner-none:        var(--cem-bend-sharp);
--md-sys-shape-corner-small:       var(--cem-bend-smooth);
--md-sys-shape-corner-medium:      var(--cem-bend-surface);
--md-sys-shape-corner-extra-large: var(--cem-bend-modal);
--md-sys-shape-corner-full:        var(--cem-bend-round);
```

### 2.4 Elevation / Layering

| M3 token surface | CEM coverage | CEM spec | Notes |
|------------------|--------------|----------|-------|
| Elevation levels 0–5 | ✓ **Covered** | D4 | Signed 7-tier ladder (recess-2 to elevation-4) |
| Tonal surface elevation | ✓ **Covered** | D4 | Tone as primary representation channel |
| Shadow elevation | ✓ **Covered** | D4 | Shadow as secondary representation channel |
| Surface container tiers | ✓ **Covered** | D4 | `--cem-layer-*` semantic endpoints |

**CEM differentiation:** CEM extends M3 elevation with **signed depth** (recess tiers for "behind" semantics) and explicit **representation channels** (tone, contour, shadow, material, space, motion).

### 2.5 Motion system

| M3 token surface | CEM coverage | CEM spec | Notes |
|------------------|--------------|----------|-------|
| Easing: standard | ✓ **Covered** | D7 | `--cem-easing-smooth` (ease-in-out) |
| Easing: accelerate (ease-in) | ✓ **Covered** | D7 | `--cem-easing-start-smooth` |
| Easing: decelerate (ease-out) | ✓ **Covered** | D7 | `--cem-easing-end-smooth` |
| Easing: emphasized | ✓ **Covered** | D7 | `--cem-easing-highlighted*` variants |
| Duration: short (50-150ms) | ✓ **Covered** | D7 | `--cem-duration-instant` (50ms) |
| Duration: medium (200-300ms) | ✓ **Covered** | D7 | `--cem-duration-noticeable` (250ms) |
| Duration: long (300-500ms) | ✓ **Covered** | D7 | `--cem-duration-lingering` (300ms) |

**Crosswalk:**
```css
--md-sys-motion-easing-standard:   var(--cem-easing-smooth);
--md-sys-motion-easing-emphasized: var(--cem-easing-highlighted);
--md-sys-motion-duration-short:    var(--cem-duration-instant);
--md-sys-motion-duration-medium:   var(--cem-duration-noticeable);
```

### 2.6 State layers

| M3 state | Opacity guidance | CEM coverage | CEM mechanism |
|----------|------------------|--------------|---------------|
| Hover | 8–16% | ✓ **Covered** | `color-mix()` 60% toward extreme variant (D0 §7.2) |
| Focus | 12% | ✓ **Covered** | `color-mix()` + zebra outline (D0 + D5) |
| Pressed | 12–16% | ✓ **Covered** | `color-mix()` 25% toward extreme variant |
| Drag | 16% | ✓ **Covered** | Pending state (5% toward extreme) |
| Disabled | 38%/12% | ✓ **Covered** | `color-mix()` 30% toward conservative-x |

**CEM differentiation:** CEM uses **formulaic derivation** via `color-mix()` rather than explicit per-state opacity tokens, reducing token surface while maintaining semantic consistency.

---

## 3. Angular Material coverage matrix

Angular Material exposes `--mat-sys-*` CSS custom properties that align with M3 tokens.

| Angular Material token area | CEM coverage | CEM spec | Notes |
|----------------------------|--------------|----------|-------|
| `--mat-sys-primary` / `-on-primary` | ✓ **Covered** | D0 | `--cem-palette-trust` / `-text` |
| `--mat-sys-surface` / `-on-surface` | ✓ **Covered** | D0 | `--cem-palette-comfort` / `-text` |
| `--mat-sys-error` / `-on-error` | ✓ **Covered** | D0 | `--cem-palette-danger` / `-text` |
| `--mat-sys-surface-container-*` | ✓ **Covered** | D0 + D4 | Layering + tonal shift |
| `--mat-sys-corner-*` | ✓ **Covered** | D3 | `--cem-bend-*` tokens |
| `--mat-sys-level*` (shadows) | ✓ **Covered** | D4 | Layering shadow channel |
| `--mat-sys-*-state-layer-opacity` | ✓ **Covered** | D0 | Formulaic `color-mix()` progression |
| Typography scale (`display/headline/body/label`) | ✓ **Covered** | D6 | Semantic typography roles |

**Adapter pattern:**
```css
/* Angular Material → CEM mapping */
--mat-sys-primary:   var(--cem-palette-trust);
--mat-sys-surface:   var(--cem-palette-comfort);
--mat-sys-corner-small: var(--cem-bend-smooth);
--mat-sys-corner-extra-large: var(--cem-bend-modal);
```

---

## 4. MUI (Material UI) coverage matrix

MUI's token surface is primarily the **theme object** with categories including `palette`, `spacing`, `shape`, `typography`, `transitions`, `zIndex`, `breakpoints`, and `shadows`.

| MUI area | MUI token surface | CEM coverage | CEM spec | Notes |
|----------|-------------------|--------------|----------|-------|
| Palette | `theme.palette.*` + action states | ✓ **Covered** | D0 | Emotional palette + action tokens |
| Spacing | `theme.spacing()` / 8px unit | ✓ **Covered** | D1 | Dimension scale (4dp increments) |
| Shape | `theme.shape.borderRadius` | ✓ **Covered** | D3 | `--cem-bend-*` endpoints |
| Transitions | `theme.transitions.duration/easing` | ✓ **Covered** | D7 | Duration + easing tokens |
| zIndex | `theme.zIndex.*` ladder | ✓ **Covered** | D4 | Semantic planes (not raw z-index) |
| Density | component props + spacing | ✓ **Covered** | D2 | Coupling modes (forgiving/balanced/compact) |
| Typography | `theme.typography.*` variants | ✓ **Covered** | D6 | Semantic roles (reading/ui/tag/data/script) |
| Shadows | `theme.shadows[]` (25 levels) | ✓ **Covered** | D4 | Layering shadow representation channel |
| Breakpoints | `theme.breakpoints.*` | ✓ **Covered** | D1x | Window size classes (compact→xlarge) |

### 4.1 MUI breakpoint mapping

| MUI key | MUI default | CEM range | CEM boundary |
|---------|-------------|-----------|--------------|
| `xs` | 0px | `compact` | 0px |
| `sm` | 600px | `medium` | 600px |
| `md` | 900px → **840px** | `expanded` | 840px (M3-aligned) |
| `lg` | 1200px | `large` | 1200px |
| `xl` | 1536px → **1600px** | `xlarge` | 1600px (M3-aligned) |

**Recommended MUI → CEM adapter:**
```ts
const theme = createTheme({
  breakpoints: {
    values: {
      xs: 0,
      sm: 600,
      md: 840,   // aligned to M3/CEM expanded
      lg: 1200,
      xl: 1600,  // aligned to M3/CEM xlarge
    },
  },
});
```

### 4.2 MUI z-index → CEM layering

| MUI zIndex | MUI value | CEM semantic plane | CEM tier |
|------------|-----------|-------------------|----------|
| `mobileStepper` | 1000 | Work | `--cem-elevation-1` |
| `fab` | 1050 | Floating | `--cem-elevation-2` |
| `appBar` | 1100 | Work | `--cem-elevation-1` |
| `drawer` | 1200 | Overlay | `--cem-elevation-3` |
| `modal` | 1300 | Command | `--cem-elevation-4` |
| `snackbar` | 1400 | Overlay | `--cem-elevation-3` |
| `tooltip` | 1500 | Overlay | `--cem-elevation-3` |

**CEM note:** D4 defines semantic planes (`back/base/work/overlay/command`) rather than raw z-index values. Implementations create z-index mappings in their adapter layer.

---

## 5. Coverage summary

### 5.1 Full coverage (no gaps)

| M3/MUI area | CEM dimension | Status |
|-------------|---------------|--------|
| Color system (primary/secondary/tertiary/error/surface) | D0 Color | ✓ Full |
| State layers (hover/focus/pressed/drag opacities) | D0 Color | ✓ Full (formulaic) |
| Typography system (typescale roles) | D6 Typography | ✓ Full |
| Shape system (corner radius scale) | D3 Shape | ✓ Full |
| Elevation / Layering | D4 Layering | ✓ Full (extended) |
| Motion (easing + duration) | D7 Timing | ✓ Full |
| Spacing / Rhythm | D1 Space | ✓ Full |
| Breakpoints / Responsive | D1x Breakpoints | ✓ Full |
| Density / Compactness | D2 Coupling | ✓ Full |
| Focus/Selection indicators | D5 Stroke | ✓ Full |

### 5.2 CEM extensions (beyond M3/MUI)

CEM provides capabilities not present in standard M3/MUI token systems:

| CEM extension | Dimension | Description |
|---------------|-----------|-------------|
| **Signed depth (recess)** | D4 | Negative elevation for "behind base" semantics |
| **Emotional palette semantics** | D0 | Intent-driven colors (comfort/trust/danger/etc.) |
| **Voice → ink/speech projection** | D6 | Cross-modal typography prominence |
| **Responsiveness strategy** | D1y | Explicit intrinsic/container/breakpoint/hybrid taxonomy |
| **Coupling invariants** | D2 | Modality-neutral zone/guard/halo contracts |
| **Zebra indicator system** | D5 | Multi-stripe focus/selection/target rings |
| **Formulaic state progression** | D0 | `color-mix()` derivation vs explicit per-state tokens |

---

## 6. Practical recommendation (compatibility without diluting CEM)

Use a **two-layer approach**:

### 6.1 CEM canonical layer

CEM tokens remain the normative contract (consumer-semantic endpoints). Product code depends on these.

```css
:root {
  /* Product code uses CEM tokens */
  --button-bg: var(--cem-action-primary-default-background);
  --card-radius: var(--cem-bend-surface);
  --transition: var(--cem-duration-noticeable) var(--cem-easing-smooth);
}
```

### 6.2 Compatibility / alias layer (optional)

Provide aliases that map M3/Angular Material/MUI token surfaces to CEM endpoints (or vice versa):

```css
:root {
  /* M3 → CEM aliases */
  --md-sys-color-primary:            var(--cem-palette-trust);
  --md-sys-color-on-primary:         var(--cem-palette-trust-text);
  --md-sys-color-surface:            var(--cem-palette-comfort);
  --md-sys-shape-corner-small:       var(--cem-bend-smooth);
  --md-sys-shape-corner-extra-large: var(--cem-bend-modal);

  /* Angular Material → CEM aliases */
  --mat-sys-primary:     var(--cem-palette-trust);
  --mat-sys-corner-small: var(--cem-bend-smooth);

  /* MUI → CEM mapping in theme */
  /* theme.transitions.duration.shortest = --cem-duration-instant */
  /* theme.shape.borderRadius = --cem-bend-smooth */
}
```

This approach preserves CEM's semantic guarantees while enabling selective adoption of M3/MUI values and ecosystem integration.

---

## 7. Quick adoption checklist

1. **Verify D0 color mapping:** emotional palette covers M3 color roles
2. **Verify D3 shape mapping:** bend endpoints map to M3 corner scale
3. **Verify D4 layering:** semantic planes cover M3/MUI elevation needs
4. **Verify D6 typography:** semantic roles cover M3 typescale taxonomy
5. **Verify D7 timing:** duration/easing tokens map to M3 motion
6. **Verify D1x breakpoints:** window size classes align with M3/MUI
7. **Verify D2 coupling:** density modes preserve operability invariants
8. **Verify D5 stroke:** focus indicators meet WCAG and M3 guidance
9. **Create adapter layer:** map framework tokens to CEM endpoints
10. **Validate in forced-colors:** system colors preserve semantics

---

## 8. References

### Internal CEM specs

- [D0. Color (Emotional Palette)](./cem-colors.md)
- [D1. Space & Rhythm](./cem-dimension.md)
- [D1x. Breakpoints](./cem-breakpoints.md)
- [D1y. Responsiveness Strategy](./cem-responsive.md)
- [D2. Coupling & Compactness](./cem-coupling.md)
- [D3. Shape & Bend](./cem-shape.md)
- [D4. Layering](./cem-layering.md)
- [D5. Stroke & Separation](./cem-stroke.md)
- [D6. Typography (Voice & Fonts)](./cem-voice-fonts-typography.md)
- [D7. Timing & Motion](./cem-timing.md)

### External references

- [Material Design 3 — Design tokens overview](https://m3.material.io/foundations/design-tokens/overview)
- [Material Design 3 — Color system](https://m3.material.io/styles/color/system/overview)
- [Material Design 3 — Typography](https://m3.material.io/styles/typography/overview)
- [Material Design 3 — Shape](https://m3.material.io/styles/shape/shape-scale-tokens)
- [Material Design 3 — Elevation](https://m3.material.io/styles/elevation/overview)
- [Material Design 3 — Motion](https://m3.material.io/styles/motion/overview)
- [Material Design 3 — State layers](https://m3.material.io/foundations/interaction/states/state-layers)
- [Material Design 3 — Window size classes](https://m3.material.io/foundations/layout/applying-layout/window-size-classes)
- [Material Web — Theming](https://material-web.dev/theming/material-theming/)
- [Angular Material — Theming guide](https://material.angular.dev/guide/theming)
- [MUI — Theming](https://mui.com/material-ui/customization/theming/)
- [MUI — Breakpoints](https://mui.com/material-ui/customization/breakpoints/)

---

*This spec documents the canonical CEM ↔ M3/Angular Material/MUI parity analysis and adapter guidance.*
