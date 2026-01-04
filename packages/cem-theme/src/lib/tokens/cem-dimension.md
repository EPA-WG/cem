# Semantic Spacing and Layout Rhythm Tokens (CEM × M3)

**Status:** Canonical (D1 v1.0)

**Last updated:** December 19, 2025

**Audience:** Design Systems, Product Design, Front-End Engineering

**Taxonomy placement:** D1. Space & Rhythm (part of the 7-dimensional CEM token framework)

**Companion specs:**
- **D0. Color (Emotional Palette)** ([`cem-colors.md`](./cem-colors.md)) — color weight pairs with spacing rhythm
- **D2. Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)) — normative for interactive operability
- **D3. Shape & Bend** ([`cem-shape.md`](./cem-shape.md)) — bend/inset harmony rules
- **D4. Layering** ([`cem-layering.md`](./cem-layering.md)) — prominent layers "earn" breathing room
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — boundaries, dividers, focus indicators
- **D6. Typography** ([`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md)) — reading rhythm validation
- **D7. Time & Motion** ([`cem-timing.md`](./cem-timing.md)) — rhythm perception

---

## Table of Contents

1. [Scope](#1-scope)
2. [CEM alignment principles applied to spacing](#2-cem-alignment-principles-applied-to-spacing)
   - [2.1 Semantic intent first](#21-semantic-intent-first)
   - [2.2 Bounded variation](#22-bounded-variation)
   - [2.3 Accessibility and operability by construction](#23-accessibility-and-operability-by-construction)
3. [Token model](#3-token-model)
4. [Relation to Dimensional Tokens](#4-relation-to-dimensional-tokens)
   - [4.1 Dimensional Tokens: where spacing lives](#41-dimensional-tokens-where-spacing-lives)
   - [4.2 Governance rules (cross-category contracts)](#42-governance-rules-cross-category-contracts)
5. [Reference spacing scale (semantic)](#5-reference-spacing-scale-semantic)
   - [5.1 Primary dimension scale](#51-primary-dimension-scale-semanticcss)
   - [5.2 Mapping to M3 rhythm](#52-mapping-to-m3-rhythm)
6. [Consumer-semantic spacing taxonomy](#6-consumer-semantic-spacing-taxonomy)
   - [6.1 Relationship gaps (between items)](#61-relationship-gaps-between-items)
   - [6.2 Insets (inside a surface)](#62-insets-inside-a-surface)
   - [6.3 Coupling and interaction safety (D2 cross-reference)](#63-coupling-and-interaction-safety-d2-cross-reference)
   - [6.4 Reading rhythm (vertical cadence for prose)](#64-reading-rhythm-vertical-cadence-for-prose)
   - [6.5 Data/scan rhythm (dense but legible)](#65-datascan-rhythm-dense-but-legible)
7. [Layout rhythm tokens (compositional semantics)](#7-layout-rhythm-tokens-compositional-semantics)
   - [7.1 Stack rhythm (vertical composition)](#71-stack-rhythm-vertical-composition)
   - [7.2 Cluster rhythm (horizontal groups)](#72-cluster-rhythm-horizontal-groups)
   - [7.3 Page gutters (responsive breathing room)](#73-page-gutters-responsive-breathing-room)
8. [Spacing modes knob (dense / normal / sparse)](#8-spacing-modes-knob-dense--normal--sparse)
   - [8.1 Adjustment policy](#81-adjustment-policy)
   - [8.2 Concrete overrides](#82-concrete-overrides-recommended-default-mapping)
   - [8.3 Notes on component mapping](#83-notes-on-component-mapping)
   - [8.4 Layout mapping checklist](#84-layout-mapping-checklist-how-to-apply)
9. [Mapping notes: M3 ↔ CEM](#9-mapping-notes-m3--cem)
10. [Quick adoption checklist](#10-quick-adoption-checklist)

**Appendices**
- [Appendix A. D2 Coupling Mode Matrix (excerpt)](#appendix-a-d2-coupling-mode-matrix-excerpt)
- [Appendix B. Governance and versioning](#appendix-b-governance-and-versioning)

---

## 1. Scope

This document defines a **consumer-semantic** spacing and layout rhythm token set that:

- Preserves the **Material 3 rhythm** (4dp increments, common 8dp cadence) as a *reference lattice*.
- Exposes **semantic endpoints** that describe *intent* (relationship, grouping, task flow) rather than
  *implementation* (e.g., `margin: 16px`).
- Supports an optional D1 knob for **spacing modes** (`dense | normal | sparse`) **without** conflating it with D2
  operability.

This document is **D1 (Space & Rhythm)**. It intentionally does *not* define interactive minimums; it **consumes** D2
coupling minimums as hard constraints.

---

## 2. CEM alignment principles applied to spacing

### 2.1 Semantic intent first

Developers should apply tokens like:

- `gap-related`, `gap-group`, `gap-section` (relationship semantics)
- `inset-control`, `inset-container` (surface semantics)
- `coupling-guard-min` (interaction safety contract; defined in D2)

…and not value-centric names like `space-16` in component code.

### 2.2 Bounded variation

A small set of *distinct, repeatable* space steps should cover most UI. Extended values exist but are intentionally
"rare".

### 2.3 Accessibility and operability by construction

Spacing must preserve:

- **Modality-neutral operability** (pointer, touch, stylus, gaze, switch, remote) via D2 coupling minimums
- **Readability** (line/paragraph rhythm)
- **User scaling** (prefer `rem` / `em` and allow platform scaling)

---

## 3. Token model

We use three layers:

1. **Reference scale tokens** (M3-aligned numeric steps; safe to map to dp/px)
2. **System/semantic tokens** (consumer-intent endpoints; what teams apply)
3. **Component mapping** (component-specific tokens map to semantic tokens; typically owned in component adapters)

---

## 4. Relation to Dimensional Tokens

### 4.1 Dimensional Tokens: where spacing lives

In CEM, spacing/rhythm is not "just layout." It is a primary subset of **Dimensional Tokens** (the physical layer: size,
distance, bend, stroke, depth).

Use this **legend-level** dimensional taxonomy to keep token categories coherent:

- **D1. Space & Rhythm** (this document)
    - gaps, insets, gutters, composition cadence, reading cadence, scan cadence

- **D2. Coupling & Compactness** (adjacent; constrains D1) — see [`cem-coupling.md`](./cem-coupling.md)
    - operable zone minimums
    - interference/isolation minimums
    - halo/expansion policy (visual size vs operable size)
    - control geometry endpoints that affect operability (heights/paddings/row sizes)

- **D3. Shape & Bend** — see [`cem-shape.md`](./cem-shape.md)
    - bend / corner radius roles
    - bend-inset harmony (see §4.2 rule 5)

- **D4. Layering** — see [`cem-layering.md`](./cem-layering.md)
    - signed 7-tier ladder (recess → base → raised → floating → overlay → command)
    - tonal shift, shadow, contour, scrim as depth channels
    - state-driven lift (hover, drag)

- **D5. Stroke & Separation** — see [`cem-stroke.md`](./cem-stroke.md)
    - boundaries (control container edges), dividers (sibling separation)
    - focus/selection/target indicators (including zebra pattern)
    - stroke basis (`--cem-stroke-none`, `--cem-stroke-hair`, `--cem-stroke-standard`, `--cem-stroke-strong`)

- **D6. Typography Thickness** — see [`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md)
    - weight/mass roles that affect perceived density and hierarchy

- **D7. Time & Motion Timing** — see [`cem-timing.md`](./cem-timing.md)
    - durations/easing groups (not geometry, but participates in "rhythm" perception)

### 4.2 Governance rules (cross-category contracts)

These rules prevent spacing changes from degrading operability, accessibility, or visual hierarchy:

1. **Spacing modes are a D1 adjustment; never a D2 coupling regression**

- `dense | normal | sparse` may adjust *gaps/insets/gutters* (D1) selectively.
- D1 changes must not reduce the D2 safety minimums (**zone**, **guard**) and must not cause halo overlap.

2. **Normative layout contract for interactive adjacency**

When spacing occurs **between two interactive affordances**, layout must respect D2 guard:

```css
/* If the gap separates adjacent interactive affordances: */
.cem-gap-interactive {
    gap: max(var(--cem-gap-related), var(--cem-coupling-guard-min));
}
```

3. **When D1 can't grow, D5 may need to compensate**

- In tight layouts or contrast themes, increase stroke/separation to maintain scannability.

4. **Elevated/prominent surfaces "earn" breathing room**

- Higher layer prominence (D4) should generally correlate with more surrounding space (D1).

5. **Bend and inset should harmonize** (D1 × D3)

- Larger bend (D3) with very tight inset (D1) reads crowded; prefer stepping inset up.
- See [`cem-shape.md`](./cem-shape.md) §8.5 for detailed guidance on bend vs inset readability.

6. **Reading cadence is jointly governed by typography + spacing**

- Reading rhythm tokens (D1) must be validated against typescale/line-height policy (D6).
- See [`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md) for typography tokens.

---

## 5. Reference spacing scale (semantic)

The spacing scale uses semantic size names rather than pixel values, making intent clear while preserving M3 rhythm (4dp
increments).

### 5.1 Primary dimension scale (semantic.css)

Eight steps covering common UI needs:

```css
:root {
    --cem-dim-xx-small  : 0.25rem;  /*  4px — micro gaps, icon padding          */
    --cem-dim-x-small   : 0.5rem;   /*  8px — related item gaps, control insets */
    --cem-dim-small     : 0.75rem;  /* 12px — group gaps, inline rhythm         */
    --cem-dim-medium    : 1rem;     /* 16px — block gaps, container insets      */
    --cem-dim-large     : 1.5rem;   /* 24px — section gaps, surface insets      */
    --cem-dim-x-large   : 2rem;     /* 32px — page gaps, wide gutters           */
    --cem-dim-xx-large  : 4rem;     /* 64px — extended layout, hero spacing     */
    --cem-dim-xxx-large : 8rem;     /* 128px — maximum breathing room           */
}
```

**Cross-reference:** These tokens are consumed by D3 Shape ([`cem-shape.md`](./cem-shape.md)) for bend values:
- `--cem-bend-smooth` uses `--cem-dim-x-small` (8px)
- `--cem-bend-surface` uses `--cem-dim-small` (12px)
- `--cem-bend-modal` uses `--cem-dim-large` + `--cem-dim-xx-small` (~28px)

### 5.2 Mapping to M3 rhythm

| Semantic token        | Value   | M3 dp equivalent | Typical use                   |
|-----------------------|---------|------------------|-------------------------------|
| `--cem-dim-xx-small`  | 0.25rem | 4dp              | Micro gaps, tight padding     |
| `--cem-dim-x-small`   | 0.5rem  | 8dp              | Related items, control insets |
| `--cem-dim-small`     | 0.75rem | 12dp             | Group gaps, inline spacing    |
| `--cem-dim-medium`    | 1rem    | 16dp             | Block gaps, container insets  |
| `--cem-dim-large`     | 1.5rem  | 24dp             | Section gaps, surface padding |
| `--cem-dim-x-large`   | 2rem    | 32dp             | Page gaps, wide gutters       |
| `--cem-dim-xx-large`  | 4rem    | 64dp             | Extended layout spacing       |
| `--cem-dim-xxx-large` | 8rem    | 128dp            | Maximum layout spacing        |

---

## 6. Consumer-semantic spacing taxonomy

The taxonomy is organized by what the space *means to the user*.

### 6.1 Relationship gaps (between items)

Use when arranging siblings.

```css
:root {
    /* Siblings that are clearly part of the same unit */
    --cem-gap-related: var(--cem-dim-x-small); /*  8px */

    /* Items in the same group, but distinct */
    --cem-gap-group: var(--cem-dim-small); /* 12px */

    /* Between groups/blocks inside one surface */
    --cem-gap-block: var(--cem-dim-medium); /* 16px */

    /* Between major sections (cards, panels, page regions) */
    --cem-gap-section: var(--cem-dim-large); /* 24px */

    /* Between page-level regions */
    --cem-gap-page: var(--cem-dim-x-large); /* 32px */
}
```

**Guideline:** if users perceive two things as "one unit," use `gap-related`. If they perceive "these are separate
things," move up to `gap-group` or `gap-block`.

> Important: for interactive adjacency, apply the D2 guard contract (`gap = max(D1 gap, D2 guard)`).

### 6.2 Insets (inside a surface)

Use when padding content within a container.

```css
:root {
    /* Smallest safe inset for tight controls */
    --cem-inset-control: var(--cem-dim-x-small); /*  8px */

    /* Default inset for common containers */
    --cem-inset-container: var(--cem-dim-medium); /* 16px */

    /* Comfortable inset for reading surfaces and prominent cards */
    --cem-inset-surface: var(--cem-dim-large); /* 24px */
}
```

**Cross-reference:** When combining insets with bend (D3), ensure inset is large enough to prevent content crowding in
rounded corners. See [`cem-shape.md`](./cem-shape.md) §8.5 for guidance.

### 6.3 Coupling and interaction safety (D2 cross-reference)

These tokens are **defined and governed in D2** ([`cem-coupling.md`](./cem-coupling.md)). They are listed here as a **normative constraint**
because they bound how far D1 spacing modes can compress interactive adjacency. **Do not set or tune these in D1**;
treat them as sourced from the D2 theme.

```css
:root {
    /* Minimum distancing between adjacent operable zones (prevents interference). */
    --cem-coupling-guard-min: 0.5rem; /* nominally 8px */

    /* Minimum operable zone (layout-level). Keep invariant across modes. */
    --cem-coupling-zone-min: 3rem; /* nominally 48px @ 16px root */

    /* Invisible expansion beyond visuals (halo). */
    --cem-coupling-halo: 0.25rem; /* nominally 4px */

  /* Legacy aliases (deprecated): keep only while migrating older code */
  --cem-touch-separation-min: var(--cem-coupling-guard-min);
  --cem-touch-target-min: var(--cem-coupling-zone-min);
}
```

Interpretation (summary):

- **Zone**: can the user reliably couple intent to the control?
- **Guard**: will adjacent controls accidentally steal intent?
- **Halo**: can we keep visuals compact while preserving operability?

### 6.4 Reading rhythm (vertical cadence for prose)

This is distinct from UI rhythm.

```css
:root {
    /* Default paragraph spacing for prose (pairs with typography) */
    --cem-rhythm-reading-paragraph: 0.75em;

    /* Space between reading sections within one surface */
    --cem-rhythm-reading-section: var(--cem-dim-large); /* 24px */
}
```

**Cross-reference:** Reading rhythm must be validated against typography tokens. See [`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md).

### 6.5 Data/scan rhythm (dense but legible)

Optimized for scan/compare flows (tables, metrics).

```css
:root {
    /* Table-like, scan-first row padding */
    --cem-rhythm-data-row: var(--cem-dim-x-small); /*  8px */

    /* Scan grouping (subtotals, row groups, metric clusters) */
    --cem-rhythm-data-group: var(--cem-dim-medium); /* 16px */
}
```

---

## 7. Layout rhythm tokens (compositional semantics)

These describe **layout structures** without baking in specific components.

### 7.1 Stack rhythm (vertical composition)

```css
:root {
    /* Default vertical stack gap for general UI stacks */
    --cem-layout-stack-gap: var(--cem-gap-block);

  /* Optional derived endpoints (rare): keep if your system needs tight/loose variants */
  --cem-layout-stack-tight: var(--cem-gap-related);
  --cem-layout-stack-loose: var(--cem-gap-section);
}
```

### 7.2 Cluster rhythm (horizontal groups)

```css
:root {
    /* Default cluster gap for inline groups (icons + text, toolbars, button clusters) */
    --cem-layout-cluster-gap: var(--cem-gap-related);

  /* Optional legacy aliases (deprecated): */
  --cem-layout-inline-tight: var(--cem-dim-x-small);  /*  8px */
  --cem-layout-inline:       var(--cem-dim-small);    /* 12px */
  --cem-layout-inline-loose: var(--cem-dim-medium);   /* 16px */
}
```

### 7.3 Page gutters (responsive breathing room)

These are *responsive* and should be treated as semantics, not fixed numbers.

```css
:root {
    /* Default content gutter */
    --cem-layout-gutter: var(--cem-dim-medium); /* 16px */

    /* Wide screens */
    --cem-layout-gutter-wide: var(--cem-dim-x-large); /* 32px */

    /* Maximum breathing room */
    --cem-layout-gutter-max: var(--cem-dim-xx-large); /* 64px */
}
```

---

## 8. Spacing modes knob (dense / normal / sparse)

Material often frames this as "density." In CEM, treat this explicitly as a **D1 spacing mode** that shifts Space &
Rhythm while preserving D2 coupling invariants.

```css
:root {
    /* dense | normal | sparse */
    --cem-layout-spacing: normal;
}
```

### 8.1 Adjustment policy

Mode intent summary:

| D1 spacing mode | Primary goal                 | What changes                                     | What must not change                                                                                                                     |
|-----------------|------------------------------|--------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------|
| `dense`         | Increase information density | Step down gaps/insets/gutters ~1 step where safe | D2 coupling invariants (zone/guard), and any gap that separates interactive affordances must still satisfy `gap = max(D1 gap, D2 guard)` |
| `normal`        | Baseline                     | Use canonical endpoints as defined               | Same invariants                                                                                                                          |
| `sparse`        | Increase calm/readability    | Step up gaps/insets/gutters ~1 step              | Same invariants                                                                                                                          |

- **normal**: baseline values as defined in Sections 6–7.
- **dense**: reduce gaps/insets by ~1 step where it does not harm reading cadence or interactive coupling.
- **sparse**: increase gaps/insets by ~1 step to improve readability and perceived calm.

Hard constraints (never override):

- `--cem-coupling-zone-min`
- `--cem-coupling-guard-min` (may force some "gaps" to stay at or above the minimum)
- `--cem-coupling-halo` (must be considered when validating dense clusters)

### 8.2 Concrete overrides (recommended default mapping)

```css
/* NORMAL is the baseline already defined above */

:root[data-cem-spacing="dense"] {
    /* Relationship gaps — step down by one level */
    --cem-gap-related: var(--cem-dim-x-small); /*  8px */
    --cem-gap-group: var(--cem-dim-x-small); /*  8px */
    --cem-gap-block: var(--cem-dim-small); /* 12px */
    --cem-gap-section: var(--cem-dim-medium); /* 16px */
    --cem-gap-page: var(--cem-dim-large); /* 24px */

    /* Insets */
    --cem-inset-control: var(--cem-dim-x-small); /*  8px */
    --cem-inset-container: var(--cem-dim-small); /* 12px */
    --cem-inset-surface: var(--cem-dim-medium); /* 16px */

    /* Layout gutters */
    --cem-layout-gutter: var(--cem-dim-small); /* 12px */
    --cem-layout-gutter-wide: var(--cem-dim-large); /* 24px */
    --cem-layout-gutter-max: var(--cem-dim-x-large); /* 32px */

    /* Content cadence */
    --cem-rhythm-reading-paragraph: 0.6em;
    --cem-rhythm-reading-section: var(--cem-dim-medium); /* 16px */
    --cem-rhythm-data-row: var(--cem-dim-xx-small); /*  4px */
    --cem-rhythm-data-group: var(--cem-dim-small); /* 12px */
}

:root[data-cem-spacing="sparse"] {
    /* Relationship gaps — step up by one level */
    --cem-gap-related: var(--cem-dim-small); /* 12px */
    --cem-gap-group: var(--cem-dim-medium); /* 16px */
    --cem-gap-block: var(--cem-dim-large); /* 24px */
    --cem-gap-section: var(--cem-dim-x-large); /* 32px */
    --cem-gap-page: var(--cem-dim-xx-large); /* 64px */

    /* Insets */
    --cem-inset-control: var(--cem-dim-small); /* 12px */
    --cem-inset-container: var(--cem-dim-large); /* 24px */
    --cem-inset-surface: var(--cem-dim-x-large); /* 32px */

    /* Layout gutters */
    --cem-layout-gutter: var(--cem-dim-large); /* 24px */
    --cem-layout-gutter-wide: var(--cem-dim-xx-large); /* 64px */
    --cem-layout-gutter-max: var(--cem-dim-xxx-large); /* 128px */

    /* Content cadence */
    --cem-rhythm-reading-paragraph: 1em;
    --cem-rhythm-reading-section: var(--cem-dim-x-large); /* 32px */
    --cem-rhythm-data-row: var(--cem-dim-small); /* 12px */
    --cem-rhythm-data-group: var(--cem-dim-large); /* 24px */
}
```

### 8.3 Notes on component mapping

* Components should bind to semantic endpoints (`gap-*`, `inset-*`, `rhythm-*`) and inherit spacing mode automatically.

* If a component is interactive and uses layout gaps between peer affordances, resolve its gap as:
`gap = max(D1 semantic gap, D2 coupling guard)`.
* If a component must remain stable across spacing modes (rare), bind it to reference steps explicitly and document why.

### 8.4 Layout mapping checklist (how to apply)

Use these patterns so teams can implement screens without inventing spacing semantics.

**Stacks (vertical composition)**

- Default: use `--cem-layout-stack-gap` (maps to `--cem-gap-block`).
- Within a group of tightly related elements (e.g., label + control + helper): use `--cem-gap-related`.

**Clusters (horizontal composition)**

- Default: use `--cem-layout-cluster-gap` (maps to `--cem-gap-related`).
- If cluster members are independently interactive (e.g., icon button groups), resolve as:
    - `gap: max(var(--cem-layout-cluster-gap), var(--cem-coupling-guard-min))`.

**Cards / panels / surfaces**

- Container padding: `--cem-inset-container`.
- Reading- or emphasis-first surfaces: `--cem-inset-surface`.
- Tight control trays: `--cem-inset-control` (only when content remains legible).
- **Note:** When using rounded surfaces, ensure inset accommodates bend. See [`cem-shape.md`](./cem-shape.md) §8.5.

**Sections and page regions**

- Between blocks: `--cem-gap-block`.
- Between major sections: `--cem-gap-section`.
- Between page regions: `--cem-gap-page`.

**Gutters (responsive framing)**

- Default: `--cem-layout-gutter`.
- Wide screens: `--cem-layout-gutter-wide`.
- Maximum breathing room: `--cem-layout-gutter-max`.

**Reading and data cadence**

- Prose: use `--cem-rhythm-reading-paragraph` and `--cem-rhythm-reading-section`.
- Scan-heavy: use `--cem-rhythm-data-row` and `--cem-rhythm-data-group`.

---

## 9. Mapping notes: M3 ↔ CEM

### 9.1 Where this matches Material 3

- 4dp-based reference steps provide the same rhythmic lattice.
- "Micro/standard/macro/extended" groupings align with common M3 guidance.

### 9.2 Where this differs (intentionally)

- CEM exposes **consumer-intent** tokens as the *primary interface*.
- Value-centric tokens are kept as **reference** only.
- Component tokens should map to semantic tokens, not to raw numbers.

---

## 10. Quick adoption checklist

1. Use the dimension scale (`--cem-dim-*`) as the foundation.
2. Map semantic endpoints (`gap-*`, `inset-*`, `rhythm-*`) to dimension tokens.
3. Treat D2 coupling (`zone/guard/halo`) as **hard constraints** for interactive adjacency — see [`cem-coupling.md`](./cem-coupling.md).
4. Update component tokens to use semantic endpoints.
5. Add optional `data-cem-spacing="dense|normal|sparse"` only if product needs spacing modes.
6. Validate on representative surfaces:
   - interactive clusters: guard is respected and no halo overlap occurs
   - reading rhythm (articles/help)
   - scan rhythm (tables/metric panels)
   - responsive gutters
7. Validate bend-inset harmony on rounded surfaces — see [`cem-shape.md`](./cem-shape.md) §8.5.

---

## Appendix A. D2 Coupling Mode Matrix (excerpt)

This excerpt is provided for convenience when working in D1. The canonical definitions and governance live in **D2.
Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)).

| D2 coupling mode | Product intent                                            | Visual geometry trend                       | Halo (`--cem-coupling-halo`) trend  | Typical surfaces                                      |
|------------------|-----------------------------------------------------------|---------------------------------------------|-------------------------------------|-------------------------------------------------------|
| `forgiving`      | Minimize mis-coupling for imprecise input                 | Larger controls/rows; more internal padding | Smaller (visuals already meet zone) | mobile-first, kiosks, accessibility-first, gaze/dwell |
| `balanced`       | Default across modalities                                 | Baseline control heights/rows               | Baseline                            | mixed pointer + touch, general app UI                 |
| `compact`        | Increase information density without breaking operability | Smaller visuals; reduced chrome             | Larger (use halo to preserve zone)  | data grids, admin tools, scan-heavy panels            |

Normative invariants (do not override in D1):

- `--cem-coupling-zone-min` is mode-invariant.
- `--cem-coupling-guard-min` is mode-invariant.
- D1 gaps between interactive affordances must resolve as: `gap = max(D1 semantic gap, D2 coupling guard)`.

---

## Appendix B. Governance and versioning

This D1 spec is a **contract**. Changes must be intentional, reviewable, and versioned.

### B.1 What counts as breaking

Treat as **major** (breaking) if you:

- Rename or remove any canonical D1 semantic endpoint (`gap-*`, `inset-*`, `layout-*`, `rhythm-*`).
- Change the semantic meaning of an endpoint (e.g., `gap-group` stops meaning "within a group").
- Change mode names or add/remove spacing modes.
- Weaken the D1↔D2 constraint model (anything that could permit violating D2 coupling guard/zone).

### B.2 What is non-breaking

Treat as **minor/patch** if you:

- Adjust numeric values within a mode while preserving semantics and invariants.
- Add new semantic endpoints that are clearly scoped, optional, and do not redefine existing meaning.
- Clarify documentation, examples, or mapping guidance.

### B.3 Versioning guidance

- **PATCH**: documentation clarifications; no token/value changes.
- **MINOR**: additive tokens or additive examples; safe value tuning.
- **MAJOR**: renames, removals, meaning shifts, or mode changes.

### B.4 Canonical cross-spec invariants

D1 is subordinate to D2 for operability. The following must hold in every release:

- D2 coupling invariants remain enforceable from D1.
- Any D1 spacing used between interactive affordances must resolve as `gap = max(D1 semantic gap, D2 coupling guard)`.

---

## References

**Local CEM documentation**
- [D2. Coupling & Compactness](./cem-coupling.md) — interactive operability constraints
- [D3. Shape & Bend](./cem-shape.md) — bend tokens, bend-inset harmony
- [D5. Stroke & Separation](./cem-stroke.md) — boundaries, dividers, focus/selection/target indicators
- [D6. Typography](./cem-voice-fonts-typography.md) — reading rhythm validation
- [D7. Time & Motion](./cem-timing.md) — rhythm perception
