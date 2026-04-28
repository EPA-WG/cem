# CEM Theme

**CEM Theme** is the token package for the Consumer-Experience Model design system. It defines a consumer-semantic layer
for colors, spacing, interaction safety, controls, shape, layering, stroke, typography, motion, breakpoints, and
responsive strategy.

This page is the package site entry point. During the theme build it is compiled from Markdown into
`dist/lib/README.xhtml`.

## Table of Contents

| Section                                                       | Context                                                                                                              |
|---------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------|
| [Use the Theme](#use-the-theme)                               | Install the package, load generated CSS, and apply CEM theme modes.                                                  |
| [Design Principles](#design-principles)                       | The rules that keep CEM themes semantic, bounded, accessible, and adaptable.                                         |
| [Relation to Other UX Theming](#relation-to-other-ux-theming) | How CEM works with Material Design 3, Angular Material, MUI, utility CSS, brand systems, and native platform colors. |
| [Token Categories](#token-categories)                         | The canonical D0-D7 token dimensions and supporting responsive specs.                                                |
| [CEM Extensions](#cem-extensions)                             | Token categories and contracts CEM introduces beyond common theme systems.                                           |
| [Documentation Context](#documentation-context)               | Links to the source specifications, parity notes, generated coverage, and build documentation.                       |

## Use the Theme

Install the package in an application or consume it from this workspace:

```bash
yarn add @epa-wg/cem-theme
```

The package provides generated CSS custom properties under `dist/lib/css/`. A typical page loads the token files it
needs, then scopes a theme mode with a class or `data-theme` attribute.

```html

<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-colors.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-dimension.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-coupling.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-controls.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-shape.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-layering.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-stroke.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-voice-fonts-typography.css" />
<link rel="stylesheet" href="/node_modules/@epa-wg/cem-theme/dist/lib/css/cem-timing.css" />
```

```html

<main
    class="cem-theme-light"
    data-cem-spacing="normal"
    data-cem-coupling="balanced"
    data-cem-shape="round"
>
    <button class="primary-action">Continue</button>
</main>
```

Product CSS should bind to CEM semantic endpoints instead of raw values:

```css
.primary-action {
    min-block-size: var(--cem-coupling-zone-min);
    padding-inline: var(--cem-control-padding-x);
    border-radius: var(--cem-bend-control);
    background: var(--cem-action-primary-default-background);
    color: var(--cem-action-primary-default-text);
    transition: background-color var(--cem-duration-noticeable) var(--cem-easing-smooth),
    color var(--cem-duration-noticeable) var(--cem-easing-smooth);
}

.primary-action:hover {
    background: var(--cem-action-primary-hover-background);
    color: var(--cem-action-primary-hover-text);
}

.primary-action:focus-visible {
    outline: var(--cem-stroke-focus) solid var(--cem-zebra-color-1);
    outline-offset: var(--cem-stroke-indicator-offset);
}
```

The TypeScript entry point is intentionally small and currently exposes package metadata:

```typescript
import { cemTheme } from '@epa-wg/cem-theme';

console.log(cemTheme());
```

## Design Principles

### Consumer Semantics First

CEM tokens describe what a user should perceive or do, not the implementation detail used to render it. A destructive
action is expressed through `--cem-action-destructive-*`; the selected color, outline, motion, and adapter mapping are
implementation outcomes.

### Bounded Variation

Every dimension has a governed range. Color uses emotional palettes and formulaic action states. Shape uses a bend
scale. Timing uses a small set of durations and easing curves. The ranges allow brand expression without letting each
component invent a private theme language.

### Accessibility by Construction

CEM treats contrast, operable target size, focus visibility, forced-colors support, and reduced motion as token-system
requirements. Tokens such as `--cem-coupling-zone-min`, zebra focus colors, and reduced-motion timing overrides are part
of the theme contract rather than after-the-fact component patches.

### Cross-Dimension Harmony

The dimensions are independent enough to govern, but not isolated. Shape must respect inset. Dense spacing must not
violate coupling guards. Layering can shift tone, stroke, shadow, motion, and space. Typography rhythm aligns with the
spacing scale.

### Theme Survivability

CEM is expected to survive light, dark, contrast, native/system, forced-colors, responsive, and dense UI contexts
without
changing the semantic meaning of product code.

## Relation to Other UX Theming

CEM is not an in-place replacement for Material Design 3, Angular Material, MUI, Tailwind, or a brand-token system. It
is
a canonical semantic layer that can sit above or beside those systems.

| Theming approach         | Relationship to CEM                                                                                                                                                                                                      |
|--------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Material Design 3        | CEM covers M3 color, typography, shape, elevation, motion, state layers, and window size classes, then adds CEM-only semantics such as emotional palette roles, coupling invariants, zebra indicators, and signed depth. |
| Angular Material and MUI | Use an adapter or alias layer that maps framework tokens to CEM endpoints, or maps CEM values into framework theme configuration. Product code should keep depending on CEM semantics where possible.                    |
| Utility CSS              | Utility classes can consume CEM variables, but CEM remains the source of semantic meaning. A utility like `gap-*` should resolve from `--cem-gap-*` or `--cem-layout-*` rather than bypassing the design contract.       |
| Brand token systems      | Brand hues and typography choices can feed CEM basis tokens. CEM then projects those choices into user-facing semantics such as trust, danger, reading, data, focus, and selection.                                      |
| Native platform colors   | CEM has a `native` theme mode and forced-colors guidance so authored themes can defer to system colors where accessibility or platform integration requires it.                                                          |

The practical pattern is two-layer theming:

1. Keep CEM tokens as the product-facing semantic contract.
2. Add optional compatibility aliases for framework or brand systems.
3. Validate that meaning survives theme modes, density modes, breakpoints, and forced-colors.

```css
:root {
    --button-bg: var(--cem-action-primary-default-background);
    --button-fg: var(--cem-action-primary-default-text);

    --md-sys-color-primary: var(--cem-palette-trust);
    --md-sys-color-on-primary: var(--cem-palette-trust-text);
    --mat-sys-primary: var(--cem-palette-trust);
}
```

## Token Categories

| Dimension | Spec                                                                    | Category                     | Product-facing purpose                                                                                                  |
|-----------|-------------------------------------------------------------------------|------------------------------|-------------------------------------------------------------------------------------------------------------------------|
| D0        | [cem-colors.md](./tokens/cem-colors.md)                                 | Color                        | Branded hues, emotional palettes, theme modes, action intents, state progression, zebra colors, forced-colors behavior. |
| D1        | [cem-dimension.md](./tokens/cem-dimension.md)                           | Space and Rhythm             | Spacing scale, semantic gaps, insets, reading rhythm, data rhythm, gutters, dense/normal/sparse spacing modes.          |
| D1x       | [cem-breakpoints.md](./tokens/cem-breakpoints.md)                       | Breakpoints                  | Semantic width and height ranges for window and container adaptation.                                                   |
| D1y       | [cem-responsive.md](./tokens/cem-responsive.md)                         | Responsiveness               | Strategy vocabulary for intrinsic, container, breakpoint, and hybrid layout behavior.                                   |
| D2        | [cem-coupling.md](./tokens/cem-coupling.md)                             | Coupling and Compactness     | Operable zone, guard, and halo invariants that protect interaction safety across input modalities.                      |
| D2c       | [cem-controls.md](./tokens/cem-controls.md)                             | Controls                     | Visual control geometry such as button height, icon button size, row height, and per-coupling-mode visual overrides.    |
| D3        | [cem-shape.md](./tokens/cem-shape.md)                                   | Shape and Bend               | Corner radius, edge softness, semantic bend endpoints, and shape-mode knobs.                                            |
| D4        | [cem-layering.md](./tokens/cem-layering.md)                             | Layering                     | Signed depth, planes, semantic elevation/recess endpoints, and appearance channels.                                     |
| D5        | [cem-stroke.md](./tokens/cem-stroke.md)                                 | Stroke and Separation        | Boundaries, dividers, focus rings, selected indicators, target indicators, and zebra geometry.                          |
| D6        | [cem-voice-fonts-typography.md](./tokens/cem-voice-fonts-typography.md) | Voice, Fonts, and Typography | Reading, UI, tag, script, data, initialism, iconized, and brand/display typography roles.                               |
| D7        | [cem-timing.md](./tokens/cem-timing.md)                                 | Timing and Motion            | Duration, easing, emphasized motion, adapter-local spring guidance, and reduced-motion overrides.                       |

## CEM Extensions

The core categories above cover the familiar surfaces found in M3, Angular Material, MUI, and most design-token systems.
CEM also introduces categories that are usually absent or implicit elsewhere.

| CEM category                          | Spec                                                                    | Why it is different                                                                                                                                    |
|---------------------------------------|-------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------|
| Emotional palette semantics           | [cem-colors.md](./tokens/cem-colors.md)                                 | Colors are named by user meaning such as comfort, trust, enthusiasm, creativity, danger, calm, and conservative rather than only by brand role or hue. |
| Formulaic action state progression    | [cem-colors.md](./tokens/cem-colors.md)                                 | Action states derive through semantic formulas instead of requiring every state color to be hand-authored.                                             |
| Zebra indicator system                | [cem-stroke.md](./tokens/cem-stroke.md)                                 | Focus, target, and selected states use coordinated multi-stripe indicators so combined states remain distinguishable.                                  |
| Coupling invariants                   | [cem-coupling.md](./tokens/cem-coupling.md)                             | `zone`, `guard`, and `halo` define modality-neutral safety contracts that survive compact visuals.                                                     |
| Controls split from coupling          | [cem-controls.md](./tokens/cem-controls.md)                             | Visual control geometry can become compact while D2 still protects the operable area.                                                                  |
| Signed depth and recess               | [cem-layering.md](./tokens/cem-layering.md)                             | Layering includes "behind base" semantics, not only positive elevation.                                                                                |
| Representation channels for layering  | [cem-layering.md](./tokens/cem-layering.md)                             | Depth can be expressed through tone, contour, shadow, material, space, and motion instead of a single shadow ladder.                                   |
| Voice projection in typography        | [cem-voice-fonts-typography.md](./tokens/cem-voice-fonts-typography.md) | Typography carries consumer-flow roles and voice levels that can project to ink, speech, and interaction emphasis.                                     |
| Data typography as a first-class role | [cem-voice-fonts-typography.md](./tokens/cem-voice-fonts-typography.md) | Numeric and scan-heavy content gets distinct font, rhythm, and feature expectations rather than inheriting body text defaults.                         |
| Responsiveness strategy taxonomy      | [cem-responsive.md](./tokens/cem-responsive.md)                         | Layout behavior records whether a component is intrinsic, container-driven, breakpoint-driven, or hybrid.                                              |
| Manifest tiers                        | [tokens/index.md](./tokens/index.md#token-manifest-schema)              | Token tables classify output as required, recommended, optional, adapter, or deprecated, and generators validate against the same tables.              |

## Documentation Context

| Document                                                                                                     | Context                                                                                                    |
|--------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------|
| [CEM Design Tokens](./tokens/index.md)                                                                       | Main token index, core principles, prompt guidance, manifest schema, and CSS generation overview.          |
| [D0 Color](./tokens/cem-colors.md)                                                                           | Emotional palettes, action intents, state formulas, theme modes, zebra colors, and forced-colors behavior. |
| [D1 Space and Rhythm](./tokens/cem-dimension.md)                                                             | Semantic spacing, insets, gutters, reading rhythm, data rhythm, and spacing modes.                         |
| [D1x Breakpoints](./tokens/cem-breakpoints.md)                                                               | Window and container range tokens plus M3 and MUI breakpoint mapping.                                      |
| [D1y Responsiveness](./tokens/cem-responsive.md)                                                             | Intrinsic, container, breakpoint, and hybrid responsive strategy rules.                                    |
| [D2 Coupling](./tokens/cem-coupling.md)                                                                      | Operability safety contract for zone, guard, halo, and compactness modes.                                  |
| [D2c Controls](./tokens/cem-controls.md)                                                                     | Visual control geometry and coupling-mode overrides.                                                       |
| [D3 Shape](./tokens/cem-shape.md)                                                                            | Bend basis, semantic endpoints, shape modes, and adapter mapping.                                          |
| [D4 Layering](./tokens/cem-layering.md)                                                                      | Signed depth, planes, semantic endpoints, and representation channels.                                     |
| [D5 Stroke](./tokens/cem-stroke.md)                                                                          | Boundaries, dividers, focus rings, and zebra indicator geometry.                                           |
| [D6 Typography](./tokens/cem-voice-fonts-typography.md)                                                      | Fontography, voice, semantic typography roles, data text, and internationalization.                        |
| [D7 Timing](./tokens/cem-timing.md)                                                                          | Motion durations, easing curves, reduced motion, and adapter guidance.                                     |
| [CEM to M3, Angular Material, and MUI Parity](./tokens/cem-m3-parity.md)                                     | Coverage matrix, CEM extensions beyond common systems, and alias-layer recommendations.                    |
| [Generated Token Coverage](./tokens/generated-token-coverage.md)                                             | Build-generated report showing emitted token coverage against the manifest.                                |
| [Build Documentation](https://github.com/EPA-WG/cem/blob/develop/packages/cem-theme/docs/docs-generation.md) | Markdown-to-XHTML compilation, CSS generation, manifest validation, and verification flow.                 |
| [HTML Dist Compilation](https://github.com/EPA-WG/cem/blob/develop/packages/cem-theme/docs/html-compile.md)  | How HTML templates are copied, rewritten, and published into `dist`.                                       |

## Publishing as XHTML

The source for this page is `packages/cem-theme/src/lib/README.md`. The theme documentation build compiles it to
`packages/cem-theme/dist/lib/README.xhtml`:

```bash
yarn nx run @epa-wg/cem-theme:build:docs
```

The same build also compiles token specifications from `src/lib/tokens/*.md` into `dist/lib/tokens/*.xhtml` and copies
image and CSS assets used by the documentation site.
