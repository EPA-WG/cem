# CSS Generation Coverage Plan

**Goal:** Generate CSS for all tokens defined in `packages/cem-theme/src/lib/tokens/*.md`

## Current State

### Color (D0) — `cem-colors.html` ✓

The `cem-colors.html` generator currently produces:

- [x] Branded color tokens (`--cem-color-{hue}-{variant}`) - 29 tokens
- [x] Emotional palette (`--cem-palette-{emotion}`, `-x`, `-text`, `-text-x`) - 28 tokens
- [x] Action intent tokens (`--cem-action-{intent}-{state}-{background|text}`) - 80 tokens
- [x] Zebra outline tokens (`--cem-zebra-color-{0-3}`, `--cem-zebra-strip-size`) - 5 tokens
- [x] Native theme overrides (`.cem-theme-native`)

### Other dimensions — generators missing

The remaining token-spec files have no working generator. `cem-breakpoints.html` exists as a stub.

| Markdown spec                   | Dimension                  | Generator                     | Approx tokens |
|---------------------------------|----------------------------|-------------------------------|---------------|
| `cem-dimension.md`              | Space & rhythm (D1)        | —                             | ~28           |
| `cem-breakpoints.md`            | Breakpoints (D1x)          | `cem-breakpoints.html` (stub) | ~10–18        |
| `cem-coupling.md`               | Coupling & density (D2)    | —                             | ~11 + modes   |
| `cem-shape.md`                  | Shape & bend (D3)          | —                             | ~16 + mode    |
| `cem-layering.md`               | Layering & elevation (D4)  | —                             | ~14           |
| `cem-stroke.md`                 | Stroke & separation (D5)   | —                             | ~16           |
| `cem-voice-fonts-typography.md` | Typography & voice (D6)    | —                             | ~80+          |
| `cem-timing.md`                 | Timing & motion (D7)       | —                             | ~14–26        |

`cem-responsive.md`, `cem-m3-parity.md`, `cem-zebra.md`, and `index.md` define no token values and are out of scope.

## Token Categories

### 1. Action Intent Tokens (Section 7)

**Status:** Complete

Action tokens encode user-flow intent and interaction state. Required per intent:

| Intent        | Emotion Mapping | Status   |
|---------------|-----------------|----------|
| `explicit`    | creativity      | Complete |
| `primary`     | trust           | Complete |
| `contextual`  | comfort         | Complete |
| `alternate`   | enthusiasm      | Complete |
| `destructive` | danger          | Complete |

**Generated background-driven state endpoints per intent:**

- `--cem-action-{intent}-{state}-background`
- `--cem-action-{intent}-{state}-text`

Generated states: `disabled`, `readonly`, `editable`, `default`, `indeterminate`, `hover`, `active`, `pending`.

Zebra-driven states (`focus`, `target`, `selected`) are handled through zebra outline tokens rather than counted as
background-driven action endpoints.

**State formulas (from Section 7.2.2):**

| State         | Background Formula                                                                        |
|---------------|-------------------------------------------------------------------------------------------|
| disabled      | `color-mix(in srgb, var(--cem-palette-{emotion}) 30%, var(--cem-palette-conservative-x))` |
| readonly      | `color-mix(in srgb, var(--cem-palette-{emotion}) 80%, var(--cem-palette-{emotion}-x))`    |
| editable      | `color-mix(in srgb, var(--cem-palette-{emotion}) 90%, var(--cem-palette-{emotion}-x))`    |
| default       | `var(--cem-palette-{emotion})`                                                            |
| indeterminate | `color-mix(in srgb, var(--cem-palette-{emotion}) 90%, var(--cem-palette-{emotion}-x))`    |
| hover         | `color-mix(in srgb, var(--cem-palette-{emotion}) 70%, var(--cem-palette-{emotion}-x))`    |
| active        | `color-mix(in srgb, var(--cem-palette-{emotion}) 25%, var(--cem-palette-{emotion}-x))`    |
| pending       | `color-mix(in srgb, var(--cem-palette-{emotion}) 5%, var(--cem-palette-{emotion}-x))`     |

**Generated tokens:** 5 intents × 8 states × 2 attributes = 80 tokens

### 2. Zebra Outline Colors (Section 8)

**Status:** Complete

Zebra is a striped outline for focus/selection/target states.

| Token                    | Purpose                    | Status   |
|--------------------------|----------------------------|----------|
| `--cem-zebra-color-0`    | Innermost stripe (surface) | Complete |
| `--cem-zebra-color-1`    | Focus stripe               | Complete |
| `--cem-zebra-color-2`    | Target stripe              | Complete |
| `--cem-zebra-color-3`    | Selected stripe            | Complete |
| `--cem-zebra-strip-size` | Stripe thickness basis     | Complete |

**Generated tokens:** 5 tokens

### 3. Additional Recommended States (Section 7.2)

**Status:** Complete

Extended state coverage includes:

- `readonly` (80% mix)
- `editable` (90% mix)
- `indeterminate` (90% mix)
- `pending` (5% mix)
- `focus` (zebra-driven)
- `target` (zebra-driven)
- `required` (marker-driven)

**Generated background-driven tokens:** 5 intents × 4 extended states × 2 attributes = 40 additional tokens

## Implementation Tasks

### Phase 1: Core Action Tokens

1. [x] ~~Add action intent mapping table to `cem-colors.md` as XML metadata~~ (hardcoded in generator - stable semantic contract)
2. [x] Update `cem-colors.html` generator with action intent → emotion mapping
3. [x] Generate `--cem-action-{intent}-{state}-background` tokens using `color-mix()`
4. [x] Generate `--cem-action-{intent}-{state}-text` tokens
5. [x] Native theme action overrides inherited via `var(--cem-palette-*)` references

### Phase 2: Zebra Outline Tokens ✓ COMPLETE

1. [x] ~~Add zebra color definitions to `cem-colors.md` metadata~~ (hardcoded in generator - fixed token set)
2. [x] Update `cem-colors.html` generator for zebra tokens
3. [x] Generate `--cem-zebra-color-{0-3}` tokens
4. [x] Generate `--cem-zebra-strip-size` token
5. [x] Add native/forced-colors zebra mappings (focus→CanvasText, selected/target→SelectedItem)
6. [x] Zebra tokens matrix in `cem-colors.html` visualization (all modes table-driven from XHTML)

### Phase 3: Extended State Coverage ✓ COMPLETE

1. [x] Add remaining state formulas (`readonly`, `editable`, `indeterminate`, `pending`)
2. [x] Generate extended state tokens
3. [x] Validate contrast ratios meet WCAG requirements (Section 11.1)

**Validation note:** Lighthouse contrast checks pass for `cem-colors.html`. The remaining `Highlight` /
`HighlightText` contrast issue is a browser/system-color design flaw, not a CEM theme bug.

### Phase 4: Foundation Primitives

Independent numeric/scale tokens with no cross-category dependencies. Tackling first unlocks D2/D3/D5.

#### 4.1 cem-dimension (D1) — `cem-dimension.html`

1. [ ] Add explicit metadata blocks to `cem-dimension.md` (tables / `<dl data-…>`) for the dimension scale, gaps, insets, and rhythms — mirroring the metadata pattern in `cem-colors.md`
2. [ ] Create `cem-dimension.html` generator following the `cem-colors.html` template (`<cem-http-request>` loads `dist/lib/tokens/cem-dimension.xhtml`, XSLT builds `:root{}` into `<code data-generated-css>`)
3. [ ] Generate 8-step scale: `--cem-dim-{xx-small|x-small|small|medium|large|x-large|xx-large|xxx-large}`
4. [ ] Generate semantic gaps: `--cem-gap-{related|group|block|section|page}`
5. [ ] Generate insets: `--cem-inset-{control|container|surface}`
6. [ ] Generate layout rhythm: `--cem-layout-{stack|cluster|gutter}` plus `_tight|_loose|_wide|_max` variants
7. [ ] Generate reading/data rhythm: `--cem-rhythm-{reading-paragraph|reading-section|data-row|data-group}`

#### 4.2 cem-timing (D7) — `cem-timing.html`

1. [ ] Add metadata blocks to `cem-timing.md` for durations, easings, and (optional) springs
2. [ ] Create `cem-timing.html` generator
3. [ ] Generate durations: `--cem-duration-{instant|noticeable|lingering}` (+ optional `-action`, `-overlay` aliases)
4. [ ] Generate easings: `--cem-easing-{smooth|highlighted|uniform|classic}` plus `-start`/`-end` variants
5. [ ] Generate spring presets if metadata defines them: `--cem-spring-{reposition|highlight}-{functional|delight}-{instant|noticeable|lingering}`

#### 4.3 cem-breakpoints (D1x) — replace stub in `cem-breakpoints.html`

1. [ ] Add metadata blocks to `cem-breakpoints.md` for width ranges (and optional height / container query bounds)
2. [ ] Replace stub `<code data-generated-css>` in `cem-breakpoints.html` with real generator logic
3. [ ] Generate width basis: `--cem-bp-width-{range}-{min|max}` and `--cem-bp-epsilon`
4. [ ] Generate optional height: `--cem-bp-height-{range}-{min|max}`
5. [ ] Generate optional container queries: `--cem-cq-width-{range}-{min|max}`

### Phase 5: Geometry & Structure

Layered on Phase 4 primitives. Stroke depends on zebra tokens already produced by `cem-colors.html`.

#### 5.1 cem-shape (D3) — `cem-shape.html`

1. [ ] Add metadata blocks to `cem-shape.md` for bend basis, semantic endpoints, and shape modes
2. [ ] Create `cem-shape.html` generator
3. [ ] Generate bend basis: `--cem-bend-{sharp|smooth|round|circle}`
4. [ ] Generate semantic endpoints: `--cem-bend-{control|surface|overlay|field|modal|media|avatar}`
5. [ ] Generate pattern tokens: `--cem-bend-{attached-edge|free-edge}` and optional `--cem-bend-control-round-ends`
6. [ ] Generate `data-cem-shape="{sharp|smooth|round}"` mode-selector overrides

#### 5.2 cem-stroke (D5) — `cem-stroke.html`

1. [ ] Add metadata blocks to `cem-stroke.md` for basis, semantic endpoints, and zebra ring composition
2. [ ] Create `cem-stroke.html` generator (references `--cem-zebra-*` from cem-colors)
3. [ ] Generate basis: `--cem-stroke-{none|hair|standard|strong}`
4. [ ] Generate semantic endpoints: `--cem-stroke-{boundary|divider|focus|selected|target}` plus `-strong`, `-grid`
5. [ ] Generate `--cem-stroke-indicator-offset` and `--cem-ring-zebra-{3|4}` composition recipes

#### 5.3 cem-layering (D4) — `cem-layering.html`

1. [ ] Add metadata blocks to `cem-layering.md` for the elevation ladder and semantic layer endpoints
2. [ ] Create `cem-layering.html` generator
3. [ ] Generate signed elevation ladder: `--cem-recess-{1|2}` and `--cem-elevation-{0|1|2|3|4}`
4. [ ] Generate semantic layers: `--cem-layer-{back|base|work|overlay|command}` plus optional `-back-deep`, `-work-floating`

### Phase 6: Density & Coupling

#### 6.1 cem-coupling (D2) — `cem-coupling.html`

1. [ ] Add metadata blocks to `cem-coupling.md` for minimums, control geometry, and density-mode formulas (references D1 dimension tokens)
2. [ ] Create `cem-coupling.html` generator
3. [ ] Generate minimums: `--cem-coupling-{zone-min|guard-min|halo}`
4. [ ] Generate control geometry: `--cem-control-{height|padding-x|padding-y}`, `--cem-icon-button-{size|icon-size}`, `--cem-{list|menu|table}-row-height`
5. [ ] Generate density-mode overrides via `data-cem-coupling="forgiving|balanced|compact"`

### Phase 7: Typography & Voice

Largest category and standalone — no dependencies on prior phases.

#### 7.1 cem-voice-fonts-typography (D6) — `cem-voice-fonts-typography.html`

1. [ ] Add metadata blocks to `cem-voice-fonts-typography.md` for fontography families, scales, voice, and semantic role endpoints
2. [ ] Create `cem-voice-fonts-typography.html` generator
3. [ ] Generate fontography families: `--cem-fontography-{reading|ui|script|initialism|brand}-family`
4. [ ] Generate thickness scale (7): `--cem-thickness-{xx-light|x-light|light|normal|bold|x-bold|xx-bold}`
5. [ ] Generate size scale (7): `--cem-typography-size-{xxs|xs|s|m|l|xl|xxl}`
6. [ ] Generate line-height + letter-spacing primitives: `--cem-typography-line-height-{reading|ui|script|badge}`, `--cem-typography-letter-spacing-{reading|ui|caps}`
7. [ ] Generate voice tokens: `--cem-voice-{whisper|soft|gentle|regular|firm|strong|loud}-{ink-thickness|icon-stroke-multiplier|speech-volume|speech-rate|speech-pitch|ssml-emphasis}`
8. [ ] Generate semantic typography roles: `--cem-typography-{reading|ui|tag|script|data|initialism|iconized|brand}-{font-family|font-size|line-height|letter-spacing|font-weight}`

## Per-generator implementation pattern

Each new generator HTML mirrors `cem-colors.html`:

1. Source data is the compiled XHTML at `dist/lib/tokens/<name>.xhtml` (built by `build:docs`).
2. Load via `<cem-http-request url="../../../dist/lib/tokens/<name>.xhtml">`.
3. XSLT/template logic builds `:root { … }` from metadata blocks (tables / `<dl data-…>` / `<table data-…>`) embedded in the markdown spec.
4. Final CSS lives inside `<code data-generated-css>` — captured by `capture-xpath-text.mjs` per the `build:css` target in `packages/cem-theme/project.json`.
5. Reuse existing `cem-css-loader.js` and `cem-http-request.js` utilities — no new infrastructure needed.

## Verification (per phase)

1. `yarn build` produces `packages/cem-theme/dist/lib/css/cem-<name>.css`.
2. Generated CSS contains every documented custom property for that category (grep against the spec).
3. Open `packages/cem-theme/src/lib/css-generators/cem-<name>.html` via `yarn start` — captured `<code data-generated-css>` is populated.
4. `yarn lint` and `yarn nx affected -t lint test build typecheck` are green.
5. Update the Token Summary table below as each phase lands.

## Token Summary

| Category                        | Defined | Generated | Gap     | Status        |
|---------------------------------|---------|-----------|---------|---------------|
| Branded colors (D0)             | 29      | 29        | 0       | ✓             |
| Emotional palette (D0)          | 28      | 28        | 0       | ✓             |
| Action tokens (D0)              | 80      | 80        | 0       | ✓             |
| Zebra tokens (D0)               | 5       | 5         | 0       | ✓             |
| Dimension & rhythm (D1)         | ~28     | 0         | ~28     | ✗ Phase 4.1   |
| Breakpoints (D1x)               | ~10–18  | 0         | ~10–18  | ✗ Phase 4.3   |
| Coupling & density (D2)         | ~11+    | 0         | ~11+    | ✗ Phase 6.1   |
| Shape & bend (D3)               | ~16+    | 0         | ~16+    | ✗ Phase 5.1   |
| Layering & elevation (D4)       | ~14     | 0         | ~14     | ✗ Phase 5.3   |
| Stroke & separation (D5)        | ~16     | 0         | ~16     | ✗ Phase 5.2   |
| Typography & voice (D6)         | ~80+    | 0         | ~80+    | ✗ Phase 7.1   |
| Timing & motion (D7)            | ~14–26  | 0         | ~14–26  | ✗ Phase 4.2   |
| **Total**                       | ~330+   | 142       | ~190+   | In progress   |

**Action tokens generated (Phase 3 complete):**
- 5 intents × 8 background-driven states × 2 attributes = 80 tokens
- Intents: explicit, primary, contextual, alternate, destructive
- States: disabled, readonly, editable, default, indeterminate, hover, active, pending
- Attributes: background, text
- Zebra-driven states (`focus`, `target`, `selected`) are represented by zebra outline tokens rather than counted as
  background-driven action tokens.

## References

- Source: `packages/cem-theme/src/lib/tokens/cem-colors.md`
- Generator: `packages/cem-theme/src/lib/css-generators/cem-colors.html`
- Output: `packages/cem-theme/dist/lib/css/cem-colors.css`
- Build: `nx run @epa-wg/cem-theme:build:css`
