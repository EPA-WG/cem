# CEM Theme AI Instructions

Use this file when applying `@epa-wg/cem-theme` styling to an existing application. These instructions are intended for
AI coding assistants and are shipped with the npm package so they match the installed package version.

## Read order

Start here, then read the relevant token specs in this same directory:

| Situation | Read |
| --------- | ---- |
| Token map and overall principles | [`index.md`](./index.md) |
| Color, action intent, theme modes, forced colors | [`cem-colors.md`](./cem-colors.md) |
| Spacing, rhythm, insets, layout gaps | [`cem-dimension.md`](./cem-dimension.md) |
| Hit targets, compactness, interaction safety | [`cem-coupling.md`](./cem-coupling.md) |
| Button/control geometry | [`cem-controls.md`](./cem-controls.md) |
| Corner radius and shape modes | [`cem-shape.md`](./cem-shape.md) |
| Borders, dividers, focus, selection, target indicators | [`cem-stroke.md`](./cem-stroke.md), [`cem-zebra.md`](./cem-zebra.md) |
| Depth, surfaces, overlays, elevation/recess | [`cem-layering.md`](./cem-layering.md) |
| Motion durations and easing | [`cem-timing.md`](./cem-timing.md) |
| Text, typography, reading rhythm | [`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md) |
| Responsive layout strategy | [`cem-breakpoints.md`](./cem-breakpoints.md), [`cem-responsive.md`](./cem-responsive.md) |
| Material/framework mapping | [`cem-m3-parity.md`](./cem-m3-parity.md) |

Use `cem.tokens.ts`, `cem.tokens.json`, and generated CSS only to confirm exact token names, tiers, specs, values, and
implementation syntax. Do not infer semantics from CSS values alone.

## Apply the package

Load the generated stylesheet once:

```js
import '@epa-wg/cem-theme/dist/lib/css/cem-combined.css';
```

For non-bundled HTML, use:

```html
<link rel="stylesheet" href="node_modules/@epa-wg/cem-theme/dist/lib/css/cem-combined.css" />
```

Add one theme scope at the app shell or document root:

```html
<main class="cem-theme-light">
  ...
</main>
```

Supported theme modes are `cem-theme-light`, `cem-theme-dark`, `cem-theme-contrast-light`,
`cem-theme-contrast-dark`, and `cem-theme-native`. If the app already has a theme selector, map it to the matching CEM
class or `data-theme="cem-theme-..."` value.

## Styling rules

- Preserve existing framework conventions, component APIs, routing, and state management.
- Do not copy generated CEM CSS into the project.
- Do not introduce a new design system, utility framework, or shadow DOM.
- Replace hardcoded visual values with semantic CEM custom properties where the docs define a matching meaning.
- Keep layout and behavior intact unless existing styling conflicts with CEM accessibility or interaction constraints.
- Use action tokens for user actions, coupling/control tokens for operable sizing, stroke/zebra tokens for focus and
  selection, shape tokens for radius, layering tokens for surfaces/depth, timing tokens for transitions, and typography
  tokens for text behavior.

## Verify after changes

- The app loads the CEM stylesheet once.
- Main screens render correctly in light and dark CEM modes.
- Focus-visible states remain obvious.
- Buttons and controls meet CEM minimum interaction sizing.
- Important product styles no longer depend on hardcoded colors where a CEM semantic token is available.
