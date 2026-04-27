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

### Other dimensions

Generator coverage by token spec.

| Markdown spec                   | Dimension                 | Generator                     | Approx tokens |
|---------------------------------|---------------------------|-------------------------------|---------------|
| `cem-dimension.md`              | Space & rhythm (D1)       | `cem-dimension.html` ✓        | 27            |
| `cem-breakpoints.md`            | Breakpoints (D1x)         | `cem-breakpoints.html` ✓      | 25            |
| `cem-coupling.md`               | Coupling safety (D2)      | `cem-coupling.html` ✓         | 3 + halo modes |
| `cem-controls.md`               | Controls geometry (D2c)   | `cem-controls.html` ✓         | 8 + modes     |
| `cem-shape.md`                  | Shape & bend (D3)         | `cem-shape.html` ✓            | 15 + brand modes |
| `cem-layering.md`               | Layering & elevation (D4) | —                             | ~14           |
| `cem-stroke.md`                 | Stroke & separation (D5)  | —                             | ~16           |
| `cem-voice-fonts-typography.md` | Typography & voice (D6)   | —                             | ~80+          |
| `cem-timing.md`                 | Timing & motion (D7)      | `cem-timing.html` ✓           | 13            |

`cem-responsive.md`, `cem-m3-parity.md`, `cem-zebra.md`, and `index.md` define no token values and are out of scope.

## Token-to-CSS Transformation Principles

These principles govern every generator (existing and new). They take precedence over any per-phase task list.

### P1. Manifest-driven contract

Every token spec MUST publish a machine-readable **token manifest** — a list of every CSS custom property the spec
defines, with required/recommended/optional/adapter-only/deprecated status, expected value type, and source rule
(formula or constant). The manifest is the single source of truth for "is this category fully generated?". Grepping the
markdown is too loose and is no longer an accepted verification.

The manifest lives inside the same `*.md` source as the prose, embedded in a stable, XPath-extractable form (see P2).

### P2. Stable extraction contract (h6 + table convention)

The current generator (`cem-colors.html`) extracts data via XPath of the form
`$xhtml//*[@id='<token-id>']/following-sibling::xhtml:table[1]/xhtml:tbody`. Every new spec MUST follow this same
convention so the generator layer stays uniform:

- A `<h6>` heading with a stable, unique `id` (e.g. `###### cem-color-hue-variant` produces
  `id="cem-color-hue-variant"`).
- The very next sibling element is a `<table>` whose `<tbody>` rows encode the data (one row per token / mapping).
- Required columns are documented in the manifest; column order is stable across edits.

Free-form `<dl data-…>` blocks are NOT a substitute. Generators do not parse arbitrary metadata shapes.

### P3. Required vs recommended vs optional vs adapter-only vs deprecated

Specs distinguish these tiers. Generators MUST honor them:

- **Required** tokens: generator emits unconditionally; missing one is a build failure.
- **Recommended** tokens: emit by default; manifest flags them so adapters can opt out.
- **Optional** tokens: emit only when the spec's metadata supplies a real value (no placeholder constants).
- **Adapter-only** aliases (e.g. `--cem-bend-xs`): emit behind an opt-in flag, not in product-facing default output.
- **Deprecated** aliases (e.g. `--cem-layout-inline-*`): emit only when an explicit "legacy" toggle is set; manifest
  flags them as deprecated.

### P4. Verification beyond presence

For each generator the build pipeline MUST check:

1. **Manifest coverage** — generated CSS contains exactly the manifest's token set (no extras, no missing).
2. **No placeholders** — no `.myClass{}` stubs, no unresolved template tokens (e.g. `{...}` AVT remnants), no empty
   declarations.
3. **CSS validity** — output parses cleanly (zero parser errors, balanced braces).
4. **Browser-level smoke** — the generator HTML opened headless via Playwright (per `CLAUDE.md` workflow) yields
   a populated `<code data-generated-css>` and the `:root` block resolves under at least light/dark/native modes.
5. **Forced-colors / accessibility checks** where the dimension affects perception (color, stroke, layering, focus).

### P5. CSS custom properties cannot drive `@media` / `@container` conditions

`var(--cem-bp-*)` is NOT usable inside `@media (min-width: …)` or `@container (…)`. Breakpoint generators MUST split
output into:

1. CSS custom properties for runtime / JS / build-tool reference.
2. Literal media-query and container-query helper rulesets for stylesheet consumption.
3. (Optional, build-time only) `@custom-media` aliases — never as production output unless a build step expands them
   first. MDN currently marks `@custom-media` as limited availability / experimental.

### P6. Generators reuse infrastructure, not duplicate it

`cem-css-loader.js` (style injection) and `cem-http-request.js` (XHTML loading) are shared utilities. New generators
MUST reuse them. Capture is performed by `capture-xpath-text.mjs` against `//code[@data-generated-css]`; each generator
MUST contain exactly one such block to avoid the current duplicate-output pathology
(`dist/lib/css/cem-colors.css` + `cem-colors-1.css`).

### P7. Canonical design ownership

When the critique surfaces missing tokens or undefined behavior, the **canonical design doc** (
`packages/cem-theme/src/lib/tokens/<spec>.md`)
is the place to fix it. Generators do NOT invent tokens. If a needed token has no canonical definition, an R&D /
decision
task lands in [R&D / Open Design Decisions](#rd--open-design-decisions) and blocks only the affected output. Questions
about which generator owns an already-canonical token are tracked as scoped implementation follow-ups; they do not block
only unrelated required tokens.

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

### Phase 1: Core Action Tokens ✓ COMPLETE

1. [x] ~~Add action intent mapping table to `cem-colors.md` as XML metadata~~ (hardcoded in generator - stable semantic
   contract)
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

### Phase 4: Metadata Schema, Token Manifest, and Pipeline Cleanup ✓ COMPLETE

Foundation phase — blocks all later phases. Establishes the contract that Principles P1–P6 require.

1. [x] Define the **token-manifest schema** (`tier` column appended to each source table; cross-product groups add tier
   to the state-axis table only). Documented in `packages/cem-theme/src/lib/tokens/index.md`.
2. [x] Add a **canonical h6+table convention** section to `index.md` (Principle P2). Specs that already deviate (most
   non-color specs use prose with embedded code-fences) get a follow-up retrofit task in their own phase.
3. [x] Backfill the `cem-colors.md` manifest as the worked example — `tier` column added to `cem-color-hue-variant`,
   `cem-palette-emotion-shift`, `cem-zebra-tokens`, and `cem-action-state-color`; lean manifest index in §14.3.
4. [x] Build a **manifest-vs-CSS validator** (`packages/cem-theme/scripts/validate-manifest.mjs`) that reads source
   tables from the compiled XHTML and asserts: every manifest token present, no extras, no `{` AVT remnants, no
   `.myClass{}` placeholders, balanced braces, PostCSS parse check.
5. [x] Wire the validator into the `build:css` target (soft mode — reports violations, exits 0). Add `--hard` flag
   to the command in `project.json` when ready to gate the build.
6. [x] **Fixed duplicate output**: added `rm -f dist/lib/css/*.css` as first step in `build:css` commands; NX cache
   no longer restores stale `cem-colors-1.css`.
7. [x] Document the new contract in `CLAUDE.md` (`## Token manifest contract` section updated; debug script moved
   to `tools/scripts/debug-cem.mjs` and referenced).

### Phase 5: D1 Dimension + Spacing Modes — `cem-dimension.html` ✓ COMPLETE

Confirmed token names from `cem-dimension.md`: `--cem-layout-stack-gap`, `--cem-layout-cluster-gap`,
`--cem-layout-gutter`/`-wide`/`-max`. `--cem-layout-inline-*` are deprecated aliases.

1. [x] Add `cem-dimension-manifest` h6+table per Phase 4 schema, marking each token's tier (deprecated for
   `--cem-layout-inline-*`).
2. [x] Add explicit h6+table metadata blocks for: dimension scale, gaps, insets, layout rhythm (correct names!), reading
   rhythm, data rhythm, **spacing modes** (`data-cem-spacing="dense|normal|sparse"`).
3. [x] Create `cem-dimension.html` generator — emit base + spacing-mode overrides (`:root[data-cem-spacing="dense"]`,
   `…="sparse"`).
4. [x] **Do NOT emit any `--cem-coupling-*` tokens here.** D1 only references coupling as normative constraints; D2 owns
   those tokens.
5. [x] Add acceptance criterion in spec prose: any consumer using D1 gaps between interactive affordances must resolve
   `gap = max(D1 gap, var(--cem-coupling-guard-min))`. Generator does NOT enforce this — it is component-author
   responsibility documented in the manifest's `notes` column.
6. [x] Reading-rhythm validation deferred to D6 cross-check (Phase 12) — D1 cannot validate rhythm in isolation.

### Phase 6: D7 Timing — `cem-timing.html` ✓ COMPLETE

1. [x] Add `cem-timing-manifest` h6+table.
2. [x] Add metadata blocks for durations, easings (with explicit per-easing declarations — R-D7-1 resolved: `highlighted`
   now uses M3 Emphasized cubic-bezier curves, visibly distinct from `smooth`).
3. [x] Create `cem-timing.html`. Emit `--cem-duration-{instant|noticeable|lingering|action|overlay}` and all 8 easing
   tokens.
4. [x] Emit `@media (prefers-reduced-motion: reduce)` overrides for the 3 core durations (0ms / 50ms / 100ms),
   preserving ordering. Alias tokens inherit automatically via `var()`.
5. [x] Springs: not emitted — R-D7-2 (value encoding) still open; reserved names have no concrete values.
6. [x] R-D7-1 resolved: `highlighted` uses `cubic-bezier(0.2, 0, 0, 1)` (M3 Emphasized); adapters SHOULD override.

### Phase 7: D1x Breakpoints — replace stub in `cem-breakpoints.html` ✓ COMPLETE

Per Principle P5, output is split.

1. [x] Add `cem-breakpoints-manifest` h6+table.
2. [x] Confirm width thresholds in `cem-breakpoints.md` align with current Material window classes (600 / 840 / 1200 /
   1600 px) and heights (480 / 900 px). Add R&D entry if spec drifts.
3. [x] Replace stub `.myClass{}` with three output blocks inside `<code data-generated-css>`:
    - **Block A — CSS custom properties (reference only)**:
      `--cem-bp-width-{compact|medium|expanded|large|xlarge}-{min|max}`, epsilon variants, height basis,
      container query reference values.
    - **Block B — literal `@media` helpers** for stylesheet use — sets `--cem-bp-active-width` and
      `--cem-bp-active-height` in each range bracket.
    - **Block C — deferred** (R-D1x-WRAP): `--cem-cq-width-*` reference values available in `:root`;
      `@container` selectors emitted by consumer.
4. [x] **Do NOT emit `@custom-media`** in production output.
5. [x] Epsilon: emit two adapter variants — `--cem-bp-epsilon-css: 0.01px` (default) and
   `--cem-bp-epsilon-mui: 0.05px` (for MUI `theme.breakpoints.step = 5` parity). Manifest documents both.
6. [x] Spec prose preserves the "not device type" rule (no `isTablet` semantics).

### Phase 8: D2 Coupling — `cem-coupling.html` ✓ COMPLETE

Reordered before D3 / D5 because stroke depends on D2 guard math. Visual control geometry split into Phase 8.5
(D2c Controls) below.

1. [x] Add `cem-coupling-minimums` h6+table marking `--cem-coupling-zone-min` and `--cem-coupling-guard-min` as
   **mode-invariant** (do not change across forgiving/balanced/compact). Tier column in source table.
2. [x] Geometry tables migrated to D2c Controls (Phase 8.5). D2 retains `cem-coupling-minimums` plus the new
   `cem-coupling-halo-overrides` table for per-mode halo policy. Token manifest index in §11.
3. [x] `cem-coupling.html` generator emits:
    - `--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo`
    - `:root[data-cem-coupling="forgiving"]` and `[compact]` overrides for **halo only** (zone/guard mode-invariant;
      visual geometry owned by D2c Controls).
4. [x] Validator wired: `validate-manifest.mjs` gets `deriveCouplingManifest` (3 tokens); `project.json` runs the
   coupling validator. 3 manifest tokens all present, no extras, CSS valid. ✓
5. [x] **Refactor for canonical Controls (Phase 8.5) complete:** visual control geometry ownership removed from D2.
   D2 Coupling now owns only `--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo`, mode
   invariants, and operability rules.
6. [ ] Manifest documents accessibility baseline: WCAG 2.2 AA target size 24×24 CSS px (with spacing exceptions); CEM
   defaults (3rem zone, 0.5rem guard) align with Android/Material 48dp+8dp guidance.
7. [ ] Manifest notes that token generation is necessary but not sufficient — components still need `min-block-size`,
   halo wrappers/pseudo-elements, and `gap = max(layout-gap, guard-min)` formulas.
8. [ ] Add proof surfaces (component examples) referenced from D2 spec: form trio (input + primary + icon), nav-list
   trailing actions, data-table row actions + selection. These are spec prose, not generator output.

### Phase 8.5: D2c Controls — `cem-controls.html` ✓ COMPLETE

Canonical decision: `cem-controls.md` is the home for visual/component control geometry. See
[`cem-controls-reasoning.md`](../cem-controls-reasoning.md).

1. [x] Created `packages/cem-theme/src/lib/tokens/cem-controls.md` as a canonical spec.
2. [x] Moved the visual control geometry source tables from `cem-coupling.md` to `cem-controls.md`:
    - baseline `--cem-control-height`, `--cem-control-padding-x`, `--cem-control-padding-y`
    - `--cem-icon-button-size`, `--cem-icon-button-icon-size`
    - `--cem-list-row-height`, `--cem-menu-row-height`, `--cem-table-row-height`
    - forgiving/compact visual geometry overrides (`cem-controls-geometry-overrides`)
3. [x] D2 Coupling now owns operability safety only:
    - `--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo` (mode-invariant minimums)
    - per-mode halo policy via `cem-coupling-halo-overrides`
    - mode-invariant safety rules and guard/halo guidance
4. [x] Created `cem-controls.html` generator using the existing h6+table extraction contract.
5. [x] `validate-manifest.mjs` adds `deriveControlsManifest`; `project.json` validates `cem-controls.css` separately.
6. [x] Cross-references updated:
    - D3 Shape (`cem-shape.md`) consumes `--cem-control-height` from `cem-controls.md` §3.1.
    - D2 Coupling references Controls for visual geometry; remains the safety constraint.
    - D1 Dimension still references D2 Coupling for adjacency guard rules.
    - Token index (`index.md`) and M3 parity (`cem-m3-parity.md`) list Controls as canonical (D2c).
7. [x] `packages/cem-theme/src/lib/css-generators/index.html` links the new Controls generator (also Coupling, which
   was previously missing from the index).
8. [x] Acceptance verified by build:
    - `cem-coupling.css` emits only safety tokens (3) plus per-mode halo overrides.
    - `cem-controls.css` emits visual control geometry (8) plus per-mode visual overrides.
    - Manifest validation passes for both specs with no duplicate token ownership.

### Phase 9: D3 Shape — `cem-shape.html` ✓ COMPLETE

Depends on Phase 8.5 Controls because `--cem-bend-round` consumes `--cem-control-height` from canonical
`cem-controls.md`.

1. [x] Manifest tables added to `cem-shape.md`: `cem-shape-basis`, `cem-shape-semantic`, `cem-shape-pattern`,
   `cem-shape-mode-sharp`, `cem-shape-mode-round`, `cem-shape-adapter-aliases` (M3-parity, tier=adapter).
   `--cem-bend-control-round-ends` is tagged `optional` (metadata supplies a real value, so it emits).
2. [x] Metadata blocks for bend basis, semantic endpoints, attachment patterns, and per-mode override tables.
3. [x] `cem-shape.html` created. Emits:
    - Basis and active alias: `--cem-bend-{sharp|smooth|round|circle}` plus required `--cem-bend`. `--cem-bend-round`
      resolves via `calc(var(--cem-shape-height, var(--cem-control-height)) / 2)` — Controls dependency satisfied by
      Phase 8.5.
    - Semantic endpoints: `--cem-bend-{control|surface|overlay|field|modal|control-round-ends|media|avatar}` (8).
    - Pattern tokens: `--cem-bend-{attached-edge|free-edge}`.
    - Existing action binding: `--cem-action-border-radius` is documented in `cem-shape.md` §14 as an existing
      component-binding contract owned by the action stylesheet (D0); not emitted here, pending R-D3-ACTION.
    - Brand mode: `data-cem-shape="sharp|smooth|round"` overrides emitted as **optional brand policy**; smooth is
      the baseline `:root` block, so only sharp/round appear as override blocks.
4. [x] Adapter-only M3-parity aliases (`cem-shape-adapter-aliases` h6+table) are NOT emitted by default.
   `deriveShapeManifest` filters them from coverage; the generator does not loop over that table. Opt-in path is
   left as a future flag.
5. [ ] Browser-level validation tasks deferred to Phase 13: focus-ring clipping with rounded corners,
   `forced-colors: active` outline behavior, 200%/400% zoom, round-end behavior under each `data-cem-coupling` mode,
   RTL logical-corner mapping.

Generated CSS: 15 tokens (5 basis + 8 semantic + 2 pattern) + sharp/round brand-mode overrides. Manifest validation
green on first build.

### Phase 10: D5 Stroke — `cem-stroke.html`

Depends on D2 (guard math) and D0 (zebra colors).

1. [ ] Resolve R&D R-D5-1 (zebra geometry ownership) BEFORE creating the generator. Outcome decides whether
   `--cem-zebra-strip-size` moves out of `cem-colors.html` into `cem-stroke.html`, or whether D0 keeps colors-only and
   D5 references them.
2. [ ] Add `cem-stroke-manifest` h6+table reflecting the R&D outcome.
3. [ ] Add metadata blocks for stroke basis, semantic endpoints, indicator-offset, ring composition recipes.
4. [ ] Create `cem-stroke.html`. Emit:
    - Basis: `--cem-stroke-{none|hair|standard|strong}`.
    - Semantic: `--cem-stroke-{boundary|divider|focus|selected|target}` (+ `-strong`, `-grid`).
    - `--cem-stroke-indicator-offset`.
    - Ring recipes: `--cem-ring-zebra-3`, `--cem-ring-zebra-4`, **each accompanied by a `forced-colors: active` outline
      fallback** (Principle P4).
5. [ ] Spec prose adds D2 guard formula: default guard MUST cover worst-case indicator outset, i.e.
   `max(4 * --cem-zebra-strip-size, --cem-stroke-indicator-offset + --cem-stroke-focus)`. This becomes a manifest note
   for D2.
6. [ ] Spec prose preserves **no-layout-shift rule**: focus/selection indicators MUST NOT mutate border-box dimensions (
   use `outline` / `box-shadow` / pseudo-elements, never `border`).
7. [ ] Spec prose notes WCAG focus-appearance caveat: external `box-shadow`/glow alone is not always counted as
   component visual presentation — `outline` fallback matters.

### Phase 11: D4 Layering — `cem-layering.html`

1. [ ] **Resolve R&D R-D4-1 (semantic-aliases-only vs adapter-hooks per channel) before generator work** — currently D4
   names rungs but doesn't specify per-channel CSS values.
2. [ ] Add `cem-layering-manifest` h6+table.
3. [ ] Add metadata blocks for elevation ladder and semantic layer endpoints; per R&D outcome, possibly per-channel (
   tone/shadow/contour/material/space/motion) tables too.
4. [ ] Create `cem-layering.html`. Emit per the R&D-decided shape:
    - Either semantic aliases only: `--cem-layer-work: var(--cem-elevation-1)`, etc.
    - Or adapter hooks per channel under a `--cem-layer-{rung}-{tone|shadow|contour|material|space|motion}` naming.
5. [ ] **NEVER emit `--cem-elevation-*` as `z-index` values** — explicitly forbidden by spec. Manifest enforces this
   with a generator unit test.
6. [ ] Acceptance criterion: every rung has at least one perceivable channel change vs its neighbors; ideally two in
   dense UIs. Manifest notes the channels each rung modifies.
7. [ ] Forced-colors validation (Phase 13): rung distinction MUST survive when contour/spacing carry the tier signal (
   subtle shadows / tonal deltas vanish).

### Phase 12: D6 Typography & Voice — `cem-voice-fonts-typography.html`

Largest category. **NOT standalone** — depends on D1 (reading rhythm), D2 (compact label safety), D5 (decoration /
underlines), and accessibility validation. The current D6 spec already defines the feature-policy, reading-ergonomics,
text-transform, and dark/contrast ink projection names; generator work should mirror those canonical names rather than
open new token-name R&D unless the manifest retrofit finds an actual contradiction.

1. [ ] Add `cem-voice-fonts-typography-manifest` h6+table covering ALL groups below.
2. [ ] Add metadata blocks for fontography families, thickness scale, size scale, line-height, letter-spacing, **feature
   policies**, **reading ergonomics**, **text-transform**, voice, semantic role endpoints, dark/contrast ink
   projections.
3. [ ] Create `cem-voice-fonts-typography.html`. Emit:
    - Fontography families: `--cem-fontography-{reading|ui|script|initialism|brand}-family`. **Quoted family stacks and
      comma-separated values MUST round-trip** (high-risk parsing — add fixture test).
    - Thickness scale (7), size scale (7).
    - Line-height + letter-spacing primitives.
    - **Feature tokens** previously missed: `--cem-typography-feature-numeric-data`,
      `--cem-typography-feature-ligatures-script`, `--cem-typography-feature-optical-sizing`.
    - **Reading-ergonomics tokens** previously missed: `--cem-typography-reading-measure-max`,
      `--cem-typography-reading-paragraph-gap`.
    - **`text-transform` role tokens** for initialism / iconized roles.
    - **Dark and contrast theme ink projections** (cross-mode, mirrors `cem-colors.html` mode pattern).
    - Voice:
      `--cem-voice-{whisper|soft|gentle|regular|firm|strong|loud}-{ink-thickness|icon-stroke-multiplier|speech-volume|speech-rate|speech-pitch|ssml-emphasis}`.
    - Semantic typography roles — output MUST include role-specific properties: data → `font-variant-numeric`, script →
      ligature policy, initialism / iconized → `text-transform`, reading → `--cem-typography-reading-measure-max` +
      `-paragraph-gap`.
4. [ ] Manifest documents: voice tokens are **CSS-exported data, not behavior**. Screen readers honor HTML/ARIA, not
   CSS. Voice tokens only feed product TTS adapters.
5. [ ] Accessibility / i18n acceptance: family stacks retain broad Unicode fallback and representative language
   coverage. Add fixture spec.
6. [ ] Cross-checks (deferred to Phase 13): D1 reading-rhythm-vs-D6-line-height, D2 compact-label legibility, D5
   underline/decoration colors.

### Phase 13: Cross-Phase Verification

Runs after every preceding phase or as periodic CI.

1. [ ] **Manifest coverage** check (Principle P4.1) green for every spec.
2. [ ] **CSS validity** check (P4.3) green; PostCSS / csstree parse with zero errors.
3. [ ] **Browser-level capture** (P4.4) via Playwright per `CLAUDE.md` workflow — every generator HTML produces a
   populated `<code data-generated-css>` block; `:root` resolves under
   `cem-theme-{native,light,dark,contrast-light,contrast-dark}`.
4. [ ] **Forced-colors / `prefers-contrast`** smoke for all dimensions that affect perception (D0, D3, D4, D5, D6).
5. [ ] **Accessibility regression suite**: Lighthouse contrast, WCAG 2.4.11 (focus not hidden), 2.5.8 (target size).
6. [ ] **Reduced-motion** check (D7): durations shorten, ordering preserved.
7. [ ] **Cross-spec semantic checks**:
    - D1 reading rhythm + D6 line-height + measure produce a usable paragraph at default size.
    - D2 guard ≥ D5 worst-case indicator outset.
    - D3 round-end + D2 dense mode does not break clip behavior.
    - D5 zebra ring fallback present under forced-colors.
8. [ ] **Adapter-only / deprecated tokens absent** from default output (assert opt-in flag default = false).

## R&D / Open Design Decisions

Only items marked "blocks phase" stop an entire phase. Other entries are scoped to optional artifacts or ownership
placement. No generator may invent or guess values absent from the canonical design docs.

| ID             | Decision needed                                                                                                                                                                                                                                                | Impact                                                                                                         |
|----------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| ~~R-Schema-1~~ | ~~Final manifest column set + h6 ID naming convention.~~ **Resolved:** tier lives in source tables (last column); cross-products add tier to the state-axis only; specs end with a lean manifest index section.                                                | Phase 4 complete.                                                                                              |
| ~~R-D7-1~~     | ~~`highlighted` easing currently aliases `smooth`.~~ **Resolved:** uses `cubic-bezier(0.2, 0, 0, 1)` (M3 Emphasized) for `-highlighted`, `cubic-bezier(0.3, 0, 0.8, 0.15)` for `-highlighted-start`, `cubic-bezier(0.05, 0.7, 0.1, 1)` for `-highlighted-end`. | Phase 6 complete.                                                                                              |
| R-D7-2         | Spring presets — define real value encoding (stiffness/damping/mass tuple) or remove from spec. Reserved names without values must not appear in the manifest.                                                                                                 | Blocks spring output only; core duration/easing output may proceed.                                            |
| R-D1x-WRAP     | Container-query helpers require consumer-provided containment. Decide whether CEM only documents that requirement or ships a wrapper component that sets `container-type`.                                                                                     | Does not block Phase 7 CSS output; blocks any wrapper/component deliverable.                                   |
| R-D3-ACTION    | Ownership of `--cem-action-border-radius` emission — D0 actions, D3 shape, or composition recipe. The token is already canonical as an existing component-binding contract.                                                                                    | Does not block required D3 bend tokens; blocks only direct emission of the action binding by `cem-shape.html`. |
| R-D5-1         | Ownership of `--cem-zebra-strip-size` and ring composition. Currently emitted by `cem-colors.html` (D0). Either move to D5 (D0 = colors only) or document the split explicitly.                                                                                | Blocks Phase 10 generator ownership decision.                                                                  |
| R-D4-1         | D4 generator output shape — semantic aliases only (`--cem-layer-work: var(--cem-elevation-1)`) vs adapter hooks per channel (`--cem-layer-work-tone`, `-shadow`, `-contour`, …). Spec must define which appearance channels are emitted at the rung level.     | Blocks Phase 11.                                                                                               |
| R-D4-2         | Per-rung minimum perceivable channel changes — formal rule (≥1 channel; ≥2 in dense UIs) and how the generator (or a unit test) verifies it.                                                                                                                   | Blocks Phase 11 verification closure.                                                                          |

## Per-generator implementation pattern

Each new generator HTML mirrors `cem-colors.html` AND honors the Token-to-CSS Transformation Principles above:

1. Source data is the compiled XHTML at `dist/lib/tokens/<name>.xhtml` (built by `build:docs`).
2. Load via `<cem-http-request url="../../../dist/lib/tokens/<name>.xhtml">`.
3. XSLT/template logic extracts data via `$xhtml//*[@id='<token-id>']/following-sibling::xhtml:table[1]/xhtml:tbody` (
   Principle P2). NO ad-hoc parsing of `<dl data-…>` or other shapes.
4. Final CSS lives inside **exactly one** `<code data-generated-css>` block (Principle P6) — captured by
   `capture-xpath-text.mjs` per the `build:css` target in `packages/cem-theme/project.json`.
5. Reuse existing `cem-css-loader.js` and `cem-http-request.js` utilities — no new infrastructure needed.
6. The generator emits ONLY tokens declared in the spec's manifest (Principle P1). Required tokens always; recommended
   tokens by default; optional/adapter/deprecated tokens behind explicit flags.

## Verification (per phase)

1. `yarn build` produces `packages/cem-theme/dist/lib/css/cem-<name>.css` (and ONLY that file — no `-1.css` duplicate).
2. Manifest validator (Phase 4 deliverable) reports green: every manifest token present, no extras, no placeholders, no
   AVT remnants, balanced braces, parses via PostCSS / csstree.
3. Open `packages/cem-theme/src/lib/css-generators/cem-<name>.html` via `yarn start` — captured
   `<code data-generated-css>` is populated and `:root` resolves under all theme modes.
4. `yarn lint` and `yarn nx affected -t lint test build typecheck` are green.
5. Phase 13 cross-phase verification suite is green.
6. Update the Token Summary table below as each phase lands.

## Token Summary

| Category                  | Defined | Generated | Gap    | Status      |
|---------------------------|---------|-----------|--------|-------------|
| Branded colors (D0)       | 29      | 29        | 0      | ✓           |
| Emotional palette (D0)    | 28      | 28        | 0      | ✓           |
| Action tokens (D0)        | 80      | 80        | 0      | ✓           |
| Zebra tokens (D0)         | 5       | 5         | 0      | ✓           |
| Dimension & rhythm (D1)   | 27      | 27        | 0      | ✓           |
| Breakpoints (D1x)         | 25      | 25        | 0      | ✓           |
| Coupling safety (D2)      | 3       | 3         | 0      | ✓           |
| Controls geometry (D2c)   | 8       | 8         | 0      | ✓           |
| Shape & bend (D3)         | 15      | 15        | 0      | ✓           |
| Layering & elevation (D4) | ~14     | 0         | ~14    | ✗ Phase 11  |
| Stroke & separation (D5)  | ~16     | 0         | ~16    | ✗ Phase 10  |
| Typography & voice (D6)   | ~95+    | 0         | ~95+   | ✗ Phase 12  |
| Timing & motion (D7)      | 13      | 13        | 0      | ✓           |
| **Total**                 | ~340+   | 228       | ~115+  | In progress |

(D6 estimate raised from ~80+ to ~95+ to include feature-policy, reading-ergonomics, and text-transform tokens that
the original plan missed; D1 +sp = spacing-mode overrides; D7 dropped from ~14–26 to ~12+ pending R-D7-2 spring
decision.)

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
