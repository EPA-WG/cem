# CEM D1y Responsiveness Strategy — Intrinsic, Container, Breakpoint, Hybrid
**Status:** Canonical (v1.0)  
**Last updated:** 2025-12-23  
**Taxonomy placement:** D1y. Layout Adaptation Method (orthogonal to D1x Breakpoints)  
**Audience:** Design Systems, Product Design, Front-End Engineering

---

**CEM first rule:** Responsiveness strategy is a **method contract**, not a device contract. Choose the method that best preserves the consumer semantics (readability, separability, coupling invariants) across unpredictable containers and aspect ratios.

D1y defines a small vocabulary for *how* a layout adapts:
- **Intrinsic** (continuous, content-driven)
- **Container** (component adapts to the space it is placed in)
- **Breakpoint** (window size class / viewport step changes)
- **Hybrid** (intrinsic-first + targeted steps)

**Companion specs:**
- **D1x. Breakpoints** ([`cem-breakpoints.md`](./cem-breakpoints.md)) — semantic ranges of available space (compact/medium/expanded/…)
- **D1. Space & Rhythm** (`cem-dimension.md`) — spacing scale and density modes
- **D2. Coupling & Compactness** (`cem-coupling.md`) — zone/guard/halo invariants
- **D5. Stroke & Separation** (`cem-stroke.md`) — separability reinforcement patterns
- **D6. Typography (Voice & Reading)** (`cem-voice-fonts-typography.md`) — line length and reading constraints

## Table of contents

1. [Scope](#1-scope)  
2. [Design principles](#2-design-principles)  
3. [Strategy vocabulary](#3-strategy-vocabulary)  
4. [Decision rules](#4-decision-rules)  
5. [Component mapping](#5-component-mapping)  
6. [Implementation patterns](#6-implementation-patterns)  
7. [Governance](#7-governance)  
8. [References](#8-references)

---

## 1. Scope

### 1.1 What D1y controls

D1y defines:

- A **small, stable strategy vocabulary** for layout adaptation.
- A **decision contract** for when to prefer intrinsic/container-driven behavior vs viewport breakpoints.
- **Component recipe annotations** (non-CSS) that document the intended strategy.

### 1.2 What D1y does not define

D1y does **not** define:

- Breakpoint boundaries (see **D1x**).
- Exact grid column counts, track sizing, or spacing (see **D1**).
- Density minimums (see **D2**).
- Layering/overlay rules (see **D4**, if present in your system).

---

## 2. Design principles

### 2.1 Orthogonal axes: space vs method

- **D1x Breakpoints** classify **available space** (semantic ranges).
- **D1y Strategy** classifies **adaptation method** (intrinsic/container/breakpoint/hybrid).

Do not encode method as a breakpoint value (for example, avoid “responsive” as a breakpoint range).

### 2.2 Intrinsic-first posture

Prefer **intrinsic** layouts that scale smoothly across widths:
- Flexbox with wrapping
- Grid with auto-placement and minimum item sizes
- Fluid constraints (`min()`, `max()`, `clamp()`)

Reserve breakpoints for **meaningful layout archetype changes** (e.g., adding a second pane).

### 2.3 Container-first for portable components

If a component can appear in multiple regions (cards, sidebars, split panes, embedded widgets), its responsiveness should primarily be **container-driven**, not viewport-driven.

### 2.4 Preserve CEM invariants

A chosen strategy MUST preserve:
- **D2 coupling minimums** (targets and guard/halo logic)
- **D6 readability constraints** (line length and reading rhythm)
- **D5 separability** under density pressure

---

## 3. Strategy vocabulary

| Strategy | Core idea | Best for | Not ideal for |
|---|---|---|---|
| `intrinsic` | Continuous, content-driven adaptation using layout rules | Listings, cards, flowing forms, “works anywhere” components | Major IA shifts requiring explicit pane structure |
| `container` | Adapt based on container size (`@container`) | Component libraries, split panes, nested layouts | Global navigation shifts that depend on the whole window |
| `breakpoint` | Adapt based on window size class / viewport | App shell navigation, primary pane topology | Reusable components placed into unknown containers |
| `hybrid` | Intrinsic-first + targeted steps (viewport or container) | List–detail, data tables, complex dashboards | When either pure intrinsic or pure breakpoint would suffice |

---

## 4. Decision rules

### 4.1 Choose `intrinsic` when constraints are sufficient

If you can express the behavior as “items have a minimum usable width; layout wraps/reflows automatically,” choose `intrinsic`.

A practical indicator: if you are tempted to specify “N columns,” you likely want `intrinsic` with a minimum item size instead.

### 4.2 Choose `container` when the host region is the variable

If the component’s width depends on placement (sidebar vs main content vs dialog), choose `container`.

### 4.3 Choose `breakpoint` when the meaning of layout changes

If the layout change is primarily about **information architecture** (pane count, navigation model), choose `breakpoint`.

### 4.4 Choose `hybrid` for step-changes inside an intrinsic layout

If most of the layout is intrinsic, but there is a clear semantic step (e.g., “show detail pane only when it fits”), choose `hybrid`:
- `breakpoint` decides when the pane exists
- intrinsic rules decide how content fills within panes

---

## 5. Component mapping

Use this as a normative annotation target for component recipes.

| Component family | Default D1y strategy | Typical query context | Notes |
|---|---|---|---|
| App shell navigation | `breakpoint` | Viewport/window | Breakpoint chooses bottom bar vs rail vs drawer. |
| Split view (master–detail) | `hybrid` | Viewport/window (sometimes container) | Breakpoint decides pane existence; intrinsic rules govern within panes. |
| Card collections / galleries | `intrinsic` | Container first | Avoid fixed column counts; wrap/auto-place. |
| Forms and settings pages | `intrinsic` | Container first | Prefer flowing groups; breakpoints only for major regrouping. |
| Data tables / comparisons | `container` / `hybrid` | Container first | Column visibility/stacking often depends on region size. |
| Filters / inspector side panels | `container` / `hybrid` | Container first | Region width is the meaningful driver. |
| Overlays (dialogs/popovers/tooltips) | `intrinsic` | Anchor/container | Size to content and coupling; avoid viewport breakpoints. |

---

## 6. Implementation patterns

### 6.1 Intrinsic Flex wrapping (example)

```css
.listing {
  display: flex;
  flex-wrap: wrap;
  gap: var(--cem-gap-related);
}

.card {
  flex: 1 1 18rem; /* min usable width */
  max-width: 28rem;
}
```

### 6.2 Intrinsic Grid auto-placement (example)

```css
.gallery {
  display: grid;
  gap: var(--cem-gap-related);
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
}
```

### 6.3 Container query adaptation (example)

```css
.panel {
  container-type: inline-size;
  container-name: panel;
}

@container panel (min-width: 42rem) {
  .panel__filters { display: block; }
}
```

### 6.4 Breakpoint / window size class adaptation (example)

Use D1x to define when an archetype changes (for example, introducing a second pane). Keep the internal layout intrinsic where possible.

### 6.5 Strategy annotation (recommended, non-CSS)

Because CSS custom properties are not suitable for expressing enumerated strategies, record strategy as **component-recipe metadata**:

```yaml
component: CardGallery
cem:
  responsiveStrategy: intrinsic
  queryContext: container
  minItemInlineSize: 14rem
```

Adapters MAY use these annotations to:
- lint against accidental viewport-based rules in container-first components
- generate documentation
- enforce consistent strategy use across a library

---

## 7. Governance

### 7.1 Breaking changes (major)

Treat as breaking if you:

- Rename/remove strategy values (`intrinsic`, `container`, `breakpoint`, `hybrid`).
- Change the semantics of a strategy value.
- Require a strategy in places where it was previously advisory (or vice versa), without a migration plan.

### 7.2 Non-breaking changes (minor/patch)

Treat as non-breaking if you:

- Add examples, recipes, or guidance.
- Add optional annotation fields (for example, `minItemInlineSize`).
- Clarify decision rules without changing semantics.

---

## 8. References

### External

- Material Design 3 — Window size classes: https://m3.material.io/foundations/layout/applying-layout/window-size-classes  
- Android Developers — Use window size classes: https://developer.android.com/develop/ui/compose/layouts/adaptive/use-window-size-classes  
- MDN — CSS container queries: https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Containment/Container_queries  
- MDN — Common grid layouts (responsive without media queries): https://developer.mozilla.org/en-US/docs/Web/CSS/Guides/Grid_layout/Common_grid_layouts  
- web.dev — The new responsive (component-driven responsive design): https://web.dev/articles/new-responsive  
- web.dev — Responsive web design basics: https://web.dev/articles/responsive-web-design-basics  
- Material UI — Breakpoints: https://mui.com/material-ui/customization/breakpoints/  
- Material UI — Container queries: https://mui.com/material-ui/customization/container-queries/  

---

*This spec defines the canonical CEM vocabulary for responsiveness strategy as an orthogonal axis to breakpoint ranges.*
