# CEM D1x Breakpoints & Responsive Ranges — Window Size Classes (CEM × M3 × MUI)
**Status:** Canonical (v1.0)
**Last updated:** 2025-12-23
**Taxonomy placement:** D1x. Layout Framing (Space & Rhythm extension)
**Audience:** Design Systems, Product Design, Front-End Engineering

---

**CEM first rule:** If an adapter’s breakpoint names or defaults conflict with CEM semantics, **CEM semantics win**. Adapters translate to their own keyspaces; the consumer-facing vocabulary remains `compact/medium/expanded/large/xlarge`.

CEM uses **breakpoints** as a layout framing primitive that selects canonical layout archetypes (single-pane, two-pane, multi-pane) based on available space. Breakpoints express **semantic meaning about available UI space**, not device categories.

**Companion specs:**
- **D0. Color** ([`cem-colors.md`](./cem-colors.md)) — emotional palette and action states
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — spacing scale, density modes, layout gaps
- **D1y. Responsiveness Strategy** ([`cem-responsive.md`](./cem-responsive.md)) — intrinsic vs container vs breakpoint adaptation
- **D2. Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)) — zone/guard/halo invariants preserved across ranges
- **D4. Layering** ([`cem-layering.md`](./cem-layering.md)) — depth/surface hierarchy
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — separability may need reinforcement in dense layouts
- **D6. Typography** ([`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md)) — reading constraints validated in large widths
- **D7. Time & Motion** ([`cem-timing.md`](./cem-timing.md)) — layout transition timing

## Table of contents

1. [Scope](#1-scope)
2. [Design principles](#2-design-principles)
3. [Token taxonomy](#3-token-taxonomy)
4. [Window size classes](#4-window-size-classes)
5. [Token surface (CSS)](#5-token-surface-css)
6. [Adapter mapping](#6-adapter-mapping)
7. [Usage guidance](#7-usage-guidance)
8. [Container queries extension](#8-container-queries-extension)
9. [Governance and versioning](#9-governance-and-versioning)
10. [Canonical token summary](#10-canonical-token-summary)
11. [References](#11-references)

---

## 1. Scope

### 1.1 What D1x controls

D1x (Breakpoints) defines:

- **Semantic range vocabulary:** `compact`, `medium`, `expanded`, `large`, `xlarge` — stable names for layout bands.
- **Numeric basis bounds:** CSS custom properties for range boundaries.
- **Adapter guidance:** mapping to Material UI, Material Design 3, and other framework breakpoints.

### 1.2 What D1x does not define

D1x does **not** define:

- Column counts, grid systems, or gutter sizes (see **D1 Space & Rhythm**).
- Component-level responsive props (Grid, Stack, etc.).
- Container query values (optional extension in §8).

---

## 2. Design principles

### 2.0 CEM first rule: semantics before adapters

CEM breakpoint semantics are defined by **available layout space** and **consumer intent** (one-pane vs two-pane vs multi-pane), not by framework defaults (for example, `md` meaning “900px” in MUI). Treat framework breakpoints as an **adapter layer**.


### 2.1 Semantics express available space, not devices

A breakpoint expresses **available UI space** (window/container), not "tablet vs desktop".
Do not implement `isTablet`-style logic based on breakpoint names.

### 2.2 Bounded variation

Breakpoints are a small, stable vocabulary. Prefer 3–5 ranges, not dozens of product-specific micro-breakpoints.

### 2.3 Breakpoints select layouts; details are separate

Breakpoints choose **canonical layout archetypes** (one-pane → two-pane → multi-pane).
Spacing, coupling, and stroke rules stay governed by their own dimensions (D1/D2/D5).

### 2.4 Cross-dimension invariants

- **D2 coupling invariants** (zone/guard/halo) must remain enforceable in every size class.
- **D6 reading constraints** should be validated in large widths (line length, paragraph rhythm).
- **D5 separability** may need reinforcement in dense layouts regardless of width.

---

## 3. Token taxonomy

### 3.1 Two-layer model

CEM breakpoints use a two-layer model:

```
Basis bounds (numeric boundaries)
  --cem-bp-{axis}-{range}-{min|max}
        │
        ▼
Semantic ranges (consumer vocabulary)
  compact | medium | expanded | large | xlarge
```

### 3.2 Why two layers?

- **Adapter flexibility:** numeric values can be tuned without changing semantic contracts.
- **Consumer stability:** product code depends on range names, not pixel values.
- **Cross-framework mapping:** different frameworks can adjust bounds while preserving intent.

---

## 4. Window size classes

CEM adopts the **Material Design 3 window size class vocabulary** as the primary reference because it is intentionally device-agnostic and defined on available width/height.

### 4.1 Width ranges (semantic)

| CEM range  | Meaning (consumer)                                            | Width — reference      |
|------------|---------------------------------------------------------------|------------------------|
| `compact`  | single-pane default; tight navigation                         | `< 600dp`              |
| `medium`   | two-pane *possible*; avoid over-wide lines                    | `600dp ≤ w < 840dp`    |
| `expanded` | two-pane comfortable; navigation rail viable                  | `840dp ≤ w < 1200dp`   |
| `large`    | multi-pane viable; guard against sparse "stretch"             | `1200dp ≤ w < 1600dp`  |
| `xlarge`   | desktop-class; constrain reading width, increase density      | `≥ 1600dp`             |

### 4.2 Height ranges (semantic)

Height is usually secondary due to vertical scrolling, but matters for landscape phones and split-screen.

| CEM range  | Height — reference     | Device representation (informative)        |
|------------|------------------------|--------------------------------------------|
| `compact`  | `< 480dp`              | 99.78% of phones in landscape              |
| `medium`   | `480dp ≤ h < 900dp`    | 96.56% of tablets in landscape, 97.59% of phones in portrait |
| `expanded` | `≥ 900dp`              | 94.25% of tablets in portrait              |

> **Note:** Most apps adapt based on width alone. Consider height when width is `medium` while height is `compact` (e.g., phones in landscape), where two-pane layouts are often impractical.

---

## 5. Token surface (CSS)

### 5.1 Basis tokens (bounds)

These tokens are numeric bounds for ranges, consumed by build-time systems (PostCSS/@custom-media), JS breakpoints, and runtime CSS systems.

```css
:root {
  /*
   * Epsilon is used only when expressing half-open intervals `[min, nextMin)`
   * as `max-width`/`max-height` (exclusive upper bounds).
   *
   * Choose epsilon per adapter/toolchain:
   * - Generic CSS: 0.01px is typically sufficient
   * - MUI default: theme.breakpoints.step = 5 => epsilon 0.05px for down()/between()
   */
  --cem-bp-epsilon: 0.01px;

  /* Width mins (M3 reference lattice; dp thresholds reused as CSS px by convention) */
  --cem-bp-width-compact-min:  0px;
  --cem-bp-width-medium-min:   600px;
  --cem-bp-width-expanded-min: 840px;
  --cem-bp-width-large-min:    1200px;
  --cem-bp-width-xlarge-min:   1600px;

  /* Derived width max (exclusive) */
  --cem-bp-width-compact-max:  calc(var(--cem-bp-width-medium-min)   - var(--cem-bp-epsilon));
  --cem-bp-width-medium-max:   calc(var(--cem-bp-width-expanded-min) - var(--cem-bp-epsilon));
  --cem-bp-width-expanded-max: calc(var(--cem-bp-width-large-min)    - var(--cem-bp-epsilon));
  --cem-bp-width-large-max:    calc(var(--cem-bp-width-xlarge-min)   - var(--cem-bp-epsilon));

  /* Height mins */
  --cem-bp-height-compact-min:  0px;
  --cem-bp-height-medium-min:   480px;
  --cem-bp-height-expanded-min: 900px;

  /* Derived height max (exclusive) */
  --cem-bp-height-compact-max: calc(var(--cem-bp-height-medium-min)   - var(--cem-bp-epsilon));
  --cem-bp-height-medium-max:  calc(var(--cem-bp-height-expanded-min) - var(--cem-bp-epsilon));
}
```

### 5.2 Normative rules

- Bounds MUST be strictly increasing.
- Range names MUST remain stable (`compact`, `medium`, `expanded`, optionally `large`, `xlarge`).
- Ranges MUST be interpreted as **half-open intervals**: `[min, nextMin)` (lower bound inclusive, upper bound exclusive).
- If you must express an exclusive upper bound as a CSS `max-width`/`max-height`, subtract a small **epsilon**.
  - The epsilon value is **adapter/toolchain-specific**, but MUST be applied consistently to avoid overlap.
  - In Material UI, prefer the built-in mechanism: `theme.breakpoints.step` (default `5`, i.e., `0.05px`) which is used to implement exclusive `down()`/`between()` upper bounds.


### 5.3 @custom-media (where supported)

```css
@custom-media --cem-compact  (max-width: calc(600px - 0.01px));
@custom-media --cem-medium   (min-width: 600px) and (max-width: calc(840px - 0.01px));
@custom-media --cem-expanded (min-width: 840px) and (max-width: calc(1200px - 0.01px));
@custom-media --cem-large    (min-width: 1200px) and (max-width: calc(1600px - 0.01px));
@custom-media --cem-xlarge   (min-width: 1600px);

/* Height classes */
@custom-media --cem-height-compact  (max-height: calc(480px - 0.01px));
@custom-media --cem-height-medium   (min-height: 480px) and (max-height: calc(900px - 0.01px));
@custom-media --cem-height-expanded (min-height: 900px);

/*
 * NOTE:
 * The `0.01px` epsilon is illustrative. Align epsilon to your toolchain:
 * - If you use MUI default step=5, prefer 0.05px for exclusive upper bounds.
 */

```

---

## 6. Adapter mapping

### 6.1 Material Design 3 (reference)

M3 window size classes provide the reference lattice for this spec. CEM directly adopts M3's width/height boundaries.

| M3 class     | Width boundary |
|--------------|----------------|
| Compact      | < 600dp        |
| Medium       | 600–839dp      |
| Expanded     | 840–1199dp     |
| Large        | 1200–1599dp    |
| Extra-large  | ≥ 1600dp       |

### 6.2 Material UI (MUI) default breakpoints

| MUI key | Default value |
|---------|---------------|
| `xs`    | 0px           |
| `sm`    | 600px         |
| `md`    | 900px         |
| `lg`    | 1200px        |
| `xl`    | 1536px        |

**Gap analysis:** MUI lacks the **840px** boundary (M3 `expanded`) and uses 1536px instead of 1600px for `xl`.

### 6.3 Recommended MUI → CEM strategies

CEM semantics are stable; your MUI integration chooses an adapter strategy.

**Strategy A (recommended for most MUI codebases): keep MUI keys, align values to the CEM lattice.**  
This avoids custom keys and minimizes TypeScript/module-augmentation friction while matching CEM/M3 boundaries.

```ts
const theme = createTheme({
  breakpoints: {
    values: {
      xs: 0,
      sm: 600,   // medium lower bound
      md: 840,   // expanded lower bound (repurposes MUI `md`)
      lg: 1200,  // large lower bound
      xl: 1600,  // xlarge lower bound (repurposes MUI `xl`)
    },
  },
});
```

**Strategy B (semantic purity): replace MUI keys with CEM keys (`compact/medium/expanded/large/xlarge`).**  
This yields the cleanest vocabulary, but requires TypeScript module augmentation and a broader migration.

**Strategy C (bridge / mixed keyspace): add `expanded`/`xlarge` alongside the defaults.**  
Use only for incremental migration. This introduces two “large-ish” keys (`xl` vs `xlarge`)—treat the mixed period as transitional.

### 6.4 Mapping tables

**Strategy A mapping (aligned values, default keys):**

| CEM range  | MUI binding |
|------------|-------------|
| `compact`  | `down('sm')` |
| `medium`   | `between('sm', 'md')` |
| `expanded` | `between('md', 'lg')` |
| `large`    | `between('lg', 'xl')` |
| `xlarge`   | `up('xl')` |

**Strategy B mapping (CEM keys as MUI keys):**

| CEM range  | MUI binding |
|------------|-------------|
| `compact`  | `down('medium')` *(or `only('compact')` if you define it explicitly)* |
| `medium`   | `between('medium', 'expanded')` |
| `expanded` | `between('expanded', 'large')` |
| `large`    | `between('large', 'xlarge')` |
| `xlarge`   | `up('xlarge')` |

**Strategy C mapping (mixed keyspace, transitional):**

| CEM range  | MUI binding |
|------------|-------------|
| `compact`  | `down('sm')` |
| `medium`   | `between('sm', 'expanded')` |
| `expanded` | `between('expanded', 'lg')` |
| `large`    | `between('lg', 'xlarge')` |
| `xlarge`   | `up('xlarge')` |

---

## 7. Usage guidance

### 7.1 What SHOULD change at a breakpoint

Breakpoints trigger **layout archetype changes**:

- **Navigation:** bottom bar → rail → drawer
- **Content:** one-pane → two-pane → three-pane
- **Density strategy:** introduce secondary panes rather than shrinking controls below D2 invariants

### 7.2 What SHOULD NOT change at a breakpoint

- Do not reduce D2 coupling minimums.
- Do not rely on breakpoints to "fix" separability; use D5 stroke reinforcement where needed.
- Avoid large typography scale jumps; prefer continuous scaling and validate reading metrics (D6).

### 7.3 Responsive design principles

1. **Mobile-first:** start with `compact` layout, progressively enhance.
2. **Content-driven:** let content needs determine when to introduce panes.
3. **Density-aware:** larger screens can support higher information density (D2).
4. **Line-length conscious:** constrain reading width in `xlarge` to preserve D6 readability (45–75 characters optimal).


### 7.4 Orthogonal responsiveness strategy

Breakpoints (D1x) classify **available space**. Responsiveness strategy (D1y) classifies **how layout adapts**. Treat these as orthogonal axes:

- `intrinsic` — continuous adaptation via Flex wrap / intrinsic Grid / fluid constraints.
- `container` — adaptation based on container size (`@container`), for portable components.
- `breakpoint` — adaptation based on viewport/window size classes, for top-level IA shifts.
- `hybrid` — intrinsic-first, with breakpoint/container steps only where meaningfully needed.

In CEM, the default posture is **intrinsic-first**; introduce breakpoint-driven behavior primarily when the **meaning** of layout changes (e.g., one-pane → two-pane), not to compensate for rigid mechanics. See [`cem-responsive.md`](./cem-responsive.md).

### 7.5 Component mapping checklist

Use this table to choose *where* D1x breakpoints apply and *which* D1y strategy to prefer. The “Responsiveness strategy” column is normative for component recipes; other columns are guidance.

| Component family                                  | Breakpoint role (D1x)           | Preferred query context              | Responsiveness strategy (D1y)   | Notes                                                                               |
|---------------------------------------------------|---------------------------------|--------------------------------------|---------------------------------|-------------------------------------------------------------------------------------|
| App shell navigation (bottom bar / rail / drawer) | **Yes** (archetype switch)      | Viewport/window                      | `breakpoint` / `hybrid`         | Breakpoints change navigation meaning; keep D2 invariants.                          |
| Master–detail / list–detail panes                 | **Yes** (introduce second pane) | Viewport/window, sometimes container | `hybrid`                        | Use intrinsic sizing within panes; breakpoint decides when the second pane exists.  |
| Card listings / galleries                         | Usually **No**                  | Container first                      | `intrinsic` / `container`       | Prefer wrap or `auto-fit/minmax()`; avoid fixed column counts.                      |
| Forms (field groups)                              | Rare                            | Container first                      | `intrinsic`                     | Prefer flowing groups and `min()`/`max()` widths; breakpoint only for major reflow. |
| Data tables / dense comparison grids              | Sometimes                       | Container first                      | `container` / `hybrid`          | Container size often determines column visibility / stacking.                       |
| Side panels, filters, inspector panes             | Sometimes                       | Container first                      | `container` / `hybrid`          | The panel width (not viewport) is the driver in split UI.                           |
| Overlays (dialogs, popovers, tooltips)            | No (typically)                  | Container/anchor                     | `intrinsic`                     | Size to content and coupling; do not key to viewport breakpoints.                   |

### 7.6 Intrinsic layouts disclaimer (flex/grid, wrapping, and container-first responsiveness)

CEM breakpoints define **semantic ranges of available space**. They do **not** mandate a fixed grid, fixed column count, or device-class assumptions.

- **Avoid fixed-column contracts on wide screens:** ultra-wide and uncommon aspect ratios (including very wide displays such as ~8K×1K) can invalidate “N columns” assumptions even when total pixels are high.
- **Prefer intrinsic, continuously-adaptive layouts first:** use Flexbox with `flex-wrap`, and/or Grid patterns like `repeat(auto-fit, minmax(...))` so layout adapts smoothly to the *actual* space available.
- **Prefer container width over viewport width for components:** in split panes, sidebars, cards, and embedded widgets, container queries produce more reliable behavior than viewport media queries.

A well-designed intrinsic layout can span **small mobile → large desktop → ultra-wide** without introducing additional breakpoint classes. Use breakpoints primarily when the **meaning** of layout changes (e.g., introducing a second pane), not to compensate for rigid layout mechanics.

### 7.7 Content-first, component-first responsiveness patterns (informative)

- **Content-driven breakpoints:** introduce a breakpoint only when content “breaks” (wrapping, truncation, unusable density), not to match a device list.
- **Component breakpoints via container queries:** make components portable; they should adapt to the size of the region they are placed into.
- **Continuous scaling where possible:** use modern CSS constraints (e.g., `clamp()`) to avoid hard jumps in spacing/typography.

### 7.8 Should CEM introduce a token value like `responsive`?

Generally **no**: “responsive” is an implementation property (intrinsic vs breakpoint-driven), not a breakpoint range.

If you need to mark that a layout is intended to be **intrinsic-responsive** (wrapping/container-first), prefer a **documentation tag** in component mapping (e.g., `strategy: intrinsic`) or an adapter/build-time flag. Avoid encoding “responsive” as an extra breakpoint value because it conflates *method* with *available space semantics*.

---

## 8. Container queries extension

Some UIs (split panes, sidebars, embedded widgets) need size classes based on **container width**, not viewport.

### 8.1 Container bounds (optional)

```css
:root {
  /* Reuse the same semantic lattice; container queries are about measurement context. */
  --cem-cq-epsilon: var(--cem-bp-epsilon);

  --cem-cq-width-medium-min:   600px;
  --cem-cq-width-expanded-min: 840px;
  --cem-cq-width-large-min:    1200px;

  /* Derived max (exclusive) */
  --cem-cq-width-compact-max:  calc(var(--cem-cq-width-medium-min)   - var(--cem-cq-epsilon));
  --cem-cq-width-medium-max:   calc(var(--cem-cq-width-expanded-min) - var(--cem-cq-epsilon));
  --cem-cq-width-expanded-max: calc(var(--cem-cq-width-large-min)    - var(--cem-cq-epsilon));
}

```

### 8.2 Container query usage

```css
.container {
  container-type: inline-size;
  container-name: card;
}

@container card (max-width: 599.98px) {
  /* compact container layout */
}

@container card (min-width: 600px) and (max-width: 839.98px) {
  /* medium container layout */
}
```


**Material UI (optional): container query adapter and shorthand**

Material UI exposes `theme.containerQueries` and an `sx` shorthand using `@<size>` / `@<size>/<name>`.

```ts
// Theme API (breakpoint keys or unitless widths)
theme.containerQueries.up('sm');        // => '@container (min-width: 600px)'
theme.containerQueries('card').up(600); // => '@container card (min-width: 600px)'
```

```tsx
// sx shorthand (unitless sizes render as px; `@500px` is invalid syntax)
<Box
  sx={{
    '@': { p: 1 },       // 0px
    '@600': { p: 2 },    // 600px
    '@840': { p: 3 },    // 840px
  }}
/>
```


### 8.3 Normative requirements

- Container bounds MUST map to the same semantic vocabulary (`compact/medium/expanded/...`).
- Consumer meaning MUST remain consistent between viewport and container contexts.

---

## 9. Governance and versioning

### 9.1 Breaking (major)

Treat as breaking if you:

- Rename/remove any canonical range name (`compact`, `medium`, `expanded`, `large`, `xlarge`).
- Reorder ranges or violate monotonic bounds.
- Change the meaning of a range (e.g., `medium` stops being the "two-pane possible" band).

### 9.2 Non-breaking (minor/patch)

Treat as non-breaking if you:

- Tune numeric bounds while preserving semantics and ordering.
- Add optional container-query bounds that map to the same range vocabulary.
- Clarify mapping guidance and examples.

---

## 10. Canonical token summary

### 10.1 Required (width basis)

| Token                         | Category | Required | Meaning                              |
|-------------------------------|----------|----------|--------------------------------------|
| `--cem-bp-width-compact-max`  | Basis    | Yes      | upper bound for `compact` width      |
| `--cem-bp-width-medium-min`   | Basis    | Yes      | lower bound for `medium` width       |
| `--cem-bp-width-medium-max`   | Basis    | Yes      | upper bound for `medium` width       |
| `--cem-bp-width-expanded-min` | Basis    | Yes      | lower bound for `expanded` width     |

### 10.2 Recommended (extended width)

| Token                          | Category | Required | Meaning                              |
|--------------------------------|----------|----------|--------------------------------------|
| `--cem-bp-width-expanded-max`  | Basis    | Optional | upper bound for `expanded` width     |
| `--cem-bp-width-large-min`     | Basis    | Optional | lower bound for `large` width        |
| `--cem-bp-width-large-max`     | Basis    | Optional | upper bound for `large` width        |
| `--cem-bp-width-xlarge-min`    | Basis    | Optional | lower bound for `xlarge` width       |

### 10.3 Recommended (height basis)

| Token                          | Category | Required | Meaning                              |
|--------------------------------|----------|----------|--------------------------------------|
| `--cem-bp-height-compact-max`  | Basis    | Optional | upper bound for `compact` height     |
| `--cem-bp-height-medium-min`   | Basis    | Optional | lower bound for `medium` height      |
| `--cem-bp-height-medium-max`   | Basis    | Optional | upper bound for `medium` height      |
| `--cem-bp-height-expanded-min` | Basis    | Optional | lower bound for `expanded` height    |

---

## 11. References

### Internal

- **D1. Space & Rhythm** (`cem-dimension.md`) — spacing scale and layout gaps
- **D2. Coupling & Compactness** (`cem-coupling.md`) — density invariants across ranges
- **D5. Stroke & Separation** (`cem-stroke.md`) — separability reinforcement under density pressure
- **D6. Typography (Voice & Reading)** (`cem-voice-fonts-typography.md`) — reading constraints and scanning patterns
- **D1y. Responsiveness Strategy** (`cem-responsive.md`) — intrinsic/container/breakpoint strategy vocabulary

### External

- [Material Design 3 — Window size classes](https://m3.material.io/foundations/layout/applying-layout/window-size-classes)
- [Android Developers — Use window size classes](https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes)
- [Material UI — Breakpoints](https://mui.com/material-ui/customization/breakpoints/)
- [Material UI — Container queries](https://mui.com/material-ui/customization/container-queries/)
- [MDN — CSS container queries](https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries)
- [web.dev — Responsive web design basics (content-driven breakpoints)](https://web.dev/articles/responsive-web-design-basics)
- [Jen Simmons — Designing Intrinsic Layouts (intrinsic design patterns)](https://talks.jensimmons.com/15TjNW/designing-intrinsic-layouts)
- [ModernCSS — Contextual Spacing for Intrinsic Web Design (`clamp()`)](https://moderncss.dev/contextual-spacing-for-intrinsic-web-design/)

---

*This spec is the canonical D1x contract for CEM responsive breakpoints and adapter mapping.*
