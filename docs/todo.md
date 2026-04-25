# CSS Generation Coverage Plan

**Goal:** Generate CSS for all tokens defined in `packages/cem-theme/src/lib/tokens/cem-colors.md`

## Current State

The `cem-colors.html` generator currently produces:

- [x] Branded color tokens (`--cem-color-{hue}-{variant}`) - 29 tokens
- [x] Emotional palette (`--cem-palette-{emotion}`, `-x`, `-text`, `-text-x`) - 28 tokens
- [x] Action intent tokens (`--cem-action-{intent}-{state}-{background|text}`) - 80 tokens
- [x] Zebra outline tokens (`--cem-zebra-color-{0-3}`, `--cem-zebra-strip-size`) - 5 tokens
- [x] Native theme overrides (`.cem-theme-native`)

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

## Token Summary

| Category          | Defined | Generated | Gap | Status |
|-------------------|---------|-----------|-----|--------|
| Branded colors    | 29      | 29        | 0   | ✓      |
| Emotional palette | 28      | 28        | 0   | ✓      |
| Action tokens     | 80      | 80        | 0   | ✓      |
| Zebra tokens      | 5       | 5         | 0   | ✓      |
| **Total**         | 142     | 142       | 0   | ✓      |

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
