# CSS Generation Coverage Plan

**Goal:** Generate CSS for all tokens defined in `packages/cem-theme/src/lib/tokens/cem-colors.md`

## Current State

The `cem-colors.html` generator currently produces:

- [x] Branded color tokens (`--cem-color-{hue}-{variant}`) - 29 tokens
- [x] Emotional palette (`--cem-palette-{emotion}`, `-x`, `-text`, `-text-x`) - 28 tokens
- [x] Native theme overrides (`.cem-theme-native`)

## Missing Token Categories

### 1. Action Intent Tokens (Section 7)

**Priority:** High

Action tokens encode user-flow intent and interaction state. Required per intent:

| Intent        | Emotion Mapping | Status  |
|---------------|-----------------|---------|
| `explicit`    | creativity      | Pending |
| `primary`     | trust           | Pending |
| `contextual`  | comfort         | Pending |
| `alternate`   | enthusiasm      | Pending |
| `destructive` | danger          | Pending |

**Required state endpoints per intent:**

- `--cem-action-{intent}-default-background`
- `--cem-action-{intent}-default-text`
- `--cem-action-{intent}-hover-background`
- `--cem-action-{intent}-hover-text`
- `--cem-action-{intent}-active-background`
- `--cem-action-{intent}-active-text`
- `--cem-action-{intent}-disabled-background`
- `--cem-action-{intent}-disabled-text`
- `--cem-action-{intent}-selected-background`
- `--cem-action-{intent}-selected-text`

**State formulas (from Section 7.2.2):**

| State    | Background Formula                                                                     |
|----------|----------------------------------------------------------------------------------------|
| disabled | `color-mix(in srgb, var(--cem-palette-{emotion}) 30%, var(--cem-palette-conservative-x))` |
| default  | `var(--cem-palette-{emotion})`                                                         |
| hover    | `color-mix(in srgb, var(--cem-palette-{emotion}) 60%, var(--cem-palette-{emotion}-x))` |
| active   | `color-mix(in srgb, var(--cem-palette-{emotion}) 25%, var(--cem-palette-{emotion}-x))` |
| selected | `var(--cem-palette-{emotion})` + zebra outline                                         |

**Estimated tokens:** 5 intents × 10 endpoints = 50 tokens

### 2. Zebra Outline Colors (Section 8)

**Priority:** High

Zebra is a striped outline for focus/selection/target states.

| Token                   | Purpose                    | Status  |
|-------------------------|----------------------------|---------|
| `--cem-zebra-color-0`   | Innermost stripe (surface) | Pending |
| `--cem-zebra-color-1`   | Focus stripe               | Pending |
| `--cem-zebra-color-2`   | Selection stripe           | Pending |
| `--cem-zebra-color-3`   | Target stripe              | Pending |
| `--cem-zebra-strip-size`| Stripe thickness basis     | Pending |

**Estimated tokens:** 5 tokens

### 3. Additional Recommended States (Section 7.2)

**Priority:** Medium

Optional state endpoints for complete coverage:

- `readonly` (80% mix)
- `editable` (90% mix)
- `indeterminate` (90% mix)
- `pending` (5% mix)
- `focus` (zebra-driven)
- `target` (zebra-driven)
- `required` (marker-driven)

**Estimated tokens:** 5 intents × 14 additional endpoints = 70 tokens

## Implementation Tasks

### Phase 1: Core Action Tokens

1. [x] ~~Add action intent mapping table to `cem-colors.md` as XML metadata~~ (hardcoded in generator - stable semantic contract)
2. [x] Update `cem-colors.html` generator with action intent → emotion mapping
3. [x] Generate `--cem-action-{intent}-{state}-background` tokens using `color-mix()`
4. [x] Generate `--cem-action-{intent}-{state}-text` tokens
5. [x] Native theme action overrides inherited via `var(--cem-palette-*)` references

### Phase 2: Zebra Outline Tokens

1. [ ] Add zebra color definitions to `cem-colors.md` metadata
2. [ ] Update `cem-colors.html` generator for zebra tokens
3. [ ] Generate `--cem-zebra-color-{0-3}` tokens
4. [ ] Generate `--cem-zebra-strip-size` token
5. [ ] Add native/forced-colors zebra mappings

### Phase 3: Extended State Coverage

1. [ ] Add remaining state formulas (readonly, editable, indeterminate, pending)
2. [ ] Generate extended state tokens
3. [ ] Validate contrast ratios meet WCAG requirements (Section 11.1)

## Token Summary

| Category          | Defined | Generated | Gap |
|-------------------|---------|-----------|-----|
| Branded colors    | 29      | 29        | 0   |
| Emotional palette | 28      | 28        | 0   |
| Action tokens     | 50      | 50        | 0   |
| Zebra tokens      | 5       | 0         | 5   |
| **Total**         | 112     | 107       | 5   |

**Action tokens generated (Phase 1 complete):**
- 5 intents × 5 states × 2 attributes = 50 tokens
- Intents: explicit, primary, contextual, alternate, destructive
- States: disabled, default, hover, active, selected
- Attributes: background, text

## References

- Source: `packages/cem-theme/src/lib/tokens/cem-colors.md`
- Generator: `packages/cem-theme/src/lib/css-generators/cem-colors.html`
- Output: `packages/cem-theme/dist/lib/css/cem-colors.css`
- Build: `nx run @epa-wg/cem-theme:build:css`
