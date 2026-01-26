# CEM Design Tokens

**Consumer-Experience Model (CEM)** is a semantic design token framework that prioritizes user experience over implementation details.

## Table of Contents — Token Specifications

| Dimension  | Spec                                                                 | Description                                                   |
|------------|----------------------------------------------------------------------|---------------------------------------------------------------|
| **D0**     | [cem-colors.md](./cem-colors.md)                                     | Color — Emotional palette, action states, theme modes         |
| **D1**     | [cem-dimension.md](./cem-dimension.md)                               | Space & Rhythm — Spacing scale, layout gaps, density          |
| **D2**     | [cem-coupling.md](./cem-coupling.md)                                 | Coupling & Compactness — Interactive operability, hit targets |
| **D3**     | [cem-shape.md](./cem-shape.md)                                       | Shape & Bend — Corner roundedness, edge softness              |
| **D4**     | [cem-layering.md](./cem-layering.md)                                 | Layering — Depth, elevation, planes (recess/lift)             |
| **D5**     | [cem-stroke.md](./cem-stroke.md)                                     | Stroke & Separation — Boundaries, dividers, focus indicators  |
| **D6**     | [cem-voice-fonts-typography.md](./cem-voice-fonts-typography.md)     | Typography — Voice, fonts, reading rhythm                     |
| **D7**     | [cem-timing.md](./cem-timing.md)                                     | Time & Motion — Animation timing, transitions                 |

### Supporting Specifications

| Spec                                       | Description                                |
|--------------------------------------------|--------------------------------------------|
| [cem-breakpoints.md](./cem-breakpoints.md) | Viewport breakpoints for responsive design |
| [cem-responsive.md](./cem-responsive.md)   | Responsive adaptation strategies           |
| [cem-m3-parity.md](./cem-m3-parity.md)     | Material Design 3 compatibility mapping    |

---

## Core CEM Principles

### 1. Semantic Intent First

Tokens express **what** something means to the user, not **how** it's implemented.

```
User intent          →  Token                      →  Implementation
"primary action"     →  --cem-action-primary-*     →  blue-l / trust palette
"danger/destructive" →  --cem-action-destructive-* →  red-l / danger palette
```

### 2. Bounded Variation

All dimensions have **constrained ranges** that prevent design drift while allowing brand expression:

- **Color:** 7 emotional palettes × 4 variants (base, extreme, text, text-extreme)
- **Space:** 8-step scale from `--cem-dimension-0` to `--cem-dimension-7`
- **Shape:** 5 bend modes (sharp → pill)
- **Timing:** 4 duration steps × 3 easing curves

### 3. Accessibility by Construction

CEM tokens are designed to meet WCAG requirements automatically:

- Color contrast ratios enforced via palette construction
- Touch targets derived from `--cem-coupling-control-height`
- Focus indicators via zebra outline system (D5)
- Reduced motion support via `--cem-timing-*` tokens

### 4. Cross-Dimension Harmony

Dimensions reference each other to maintain visual coherence:

- Shape bend derives from spacing (`--cem-dimension-*`)
- Layering couples with color tonal shifts
- Stroke thickness scales with density mode
- Typography rhythm aligns with spacing scale

### 5. Theme Survivability

All tokens support multiple rendering contexts:

- Light/dark schemes via `light-dark()` function
- Native/system colors via CSS system color keywords
- Forced colors via `@media (forced-colors: active)`
- High contrast via `contrast-light` / `contrast-dark` modes

---

## Creating Customization Prompts

When using AI assistants to customize CEM tokens, structure your prompts to reference the relevant dimension specifications.

### Prompt Template

```
Context: I'm working with the CEM design token system.

Relevant specifications:
- [List the dimension specs that apply to your customization]

Current state:
- [Describe the current token values or behavior]

Goal:
- [Describe what you want to achieve]

Constraints:
- [List any brand guidelines, accessibility requirements, or technical limitations]

Please [action] while maintaining CEM principles of [relevant principles].
```

### Example Prompts by Topic

#### Color Customization

```
Context: I'm working with the CEM design token system.

Relevant specifications:
- cem-colors.md (Section 4: branded color family, Section 5: emotional palette)

Current state:
- Trust palette uses blue-l (#d7e3ff) for light mode

Goal:
- Replace the trust color with our brand blue (#1E40AF) while maintaining
  the emotional palette structure

Constraints:
- Must pass WCAG AA contrast (4.5:1) for text
- Must work in both light and dark modes
- Native theme should still use system Highlight color

Please update the cem-color-hue-variant table and verify the contrast ratios.
```

#### Action State Customization

```
Context: I'm working with the CEM design token system.

Relevant specifications:
- cem-colors.md (Section 7: action intent and state tokens)
- cem-colors.md (Section 7.2.2: state formulas)

Current state:
- Hover state uses 60% color-mix with extreme variant

Goal:
- Make hover states more subtle for our minimal brand aesthetic

Constraints:
- Must remain distinguishable from default state
- Must work across all 5 action intents

Please adjust the hover state formula in cem-action-state-color table.
```

#### Spacing Customization

```
Context: I'm working with the CEM design token system.

Relevant specifications:
- cem-dimension.md (Section 3: token model)
- cem-coupling.md (Section on density modes)

Current state:
- Base spacing uses 4px unit with 8-step scale

Goal:
- Implement a more compact UI for data-dense dashboards

Constraints:
- Touch targets must remain ≥44px
- Must support all three density modes (compact/normal/comfortable)

Please recommend which basis tokens to adjust for a compact-first design.
```

#### Shape Customization

```
Context: I'm working with the CEM design token system.

Relevant specifications:
- cem-shape.md (Section 4: minimal bend basis, Section 6: shape mode knob)

Current state:
- Using default "rounded" mode

Goal:
- Implement a "sharp" brand aesthetic with minimal corner rounding

Constraints:
- Pill shapes for tags/chips should remain fully rounded
- Must maintain visual hierarchy between containers and controls

Please update the shape mode configuration and verify semantic endpoint mappings.
```

### Tips for Effective Prompts

1. **Reference specific sections** — CEM specs are structured with numbered sections; cite them directly

2. **Include the token table IDs** — Tables have h6 ID markers (e.g., `###### cem-action-state-color`) that can be referenced in XPath queries

3. **State the dimension interactions** — If your change affects multiple dimensions, list all relevant specs

4. **Specify the output format** — Ask for markdown table updates, CSS output, or both

5. **Request validation** — Ask the assistant to verify contrast ratios, touch targets, or other accessibility requirements

---

## CSS Generation Pipeline

Token specifications (`.md` files) are transpiled to XHTML and processed by HTML generators to produce CSS:

```
*.md (source)  →  *.xhtml (transpiled)  →  *.html (generator)  →  *.css (output)
```

See [docs-generation.md](../../docs/docs-generation.md) for the full build pipeline documentation.

---

## References

- [CEM Discussion](https://github.com/AnyWhichWay/consumer-experience-model/discussions)
- [Material Design 3 Tokens](https://m3.material.io/foundations/design-tokens)
- [WCAG 2.1 Guidelines](https://www.w3.org/WAI/WCAG21/quickref/)
