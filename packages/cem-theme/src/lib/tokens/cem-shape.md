# CEM D3 Shape — Bend (Edge Softness & Corner Roundedness)

**Status:** Proposed (canonical CEM spec)
**Last updated:** December 19, 2025

**Taxonomy placement:** D3. Shape & Bend (part of the 7-dimensional CEM token framework)

**Companion specs:**
- **D0. Color (Emotional Palette)** ([`cem-colors.md`](./cem-colors.md)) — color and shape work together for emotional impact
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — provides dimension scale used by bend
- **D2. Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)) — provides control height for geometry-driven bend
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — boundaries, dividers, and focus/selection/target indicators
- **D6. Typography** ([`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md)) — related visual hierarchy
- **D7. Time & Motion** ([`cem-timing.md`](./cem-timing.md)) — animation timing for shape transitions

---

## Table of Contents

1. [Purpose and scope](#1-purpose-and-scope)
2. [Why "bend" (token naming rationale)](#2-why-bend-token-naming-rationale)
3. [CEM alignment principles applied to shape](#3-cem-alignment-principles-applied-to-shape)
   - [3.1 Semantic intent first](#31-semantic-intent-first)
   - [3.2 Bounded variation](#32-bounded-variation)
   - [3.3 Layering (basis → semantic endpoints → adapters)](#33-layering-basis--semantic-endpoints--adapters)
4. [Minimal bend basis (complexity-compliant)](#4-minimal-bend-basis-complexity-compliant)
5. [Semantic endpoints (product-facing contract)](#5-semantic-endpoints-product-facing-contract)
6. [Shape mode knob (brand expression)](#6-shape-mode-knob-brand-expression)
   - [6.1 Mode meanings](#61-mode-meanings)
   - [6.2 Mode implementation (recommended)](#62-mode-implementation-recommended)
   - [6.3 Helper classes (optional; scoped overrides)](#63-helper-classes-optional-scoped-overrides)
7. [Directional and asymmetric patterns](#7-directional-and-asymmetric-patterns)
   - [7.1 Attachment pattern](#71-attachment-pattern-one-pattern-not-many-tokens)
   - [7.2 When to use asymmetric corners](#72-when-to-use-asymmetric-corners)
   - [7.3 Implementation guidance](#73-implementation-guidance)
   - [7.4 Common recipes](#74-common-recipes)
8. [Accessibility and inclusive design requirements](#8-accessibility-and-inclusive-design-requirements)
   - [8.1 Shape must not be the only signal](#81-shape-must-not-be-the-only-signal)
   - [8.2 High-contrast and forced-colors resilience](#82-high-contrast-and-forced-colors-resilience)
   - [8.3 Focus indicators must respect the target](#83-focus-indicators-must-respect-the-target)
   - [8.4 Pointer target size and spacing](#84-pointer-target-size-and-spacing-bend--d2-size--d1-separation)
   - [8.5 Bend vs inset readability](#85-bend-vs-inset-readability-bend--d1-padding)
   - [8.6 Minimum test checklist](#86-minimum-test-checklist)
9. [Consumer vocabulary for "round ends" (capsules)](#9-consumer-vocabulary-for-round-ends-capsules)
10. [Component mapping](#10-component-mapping)
    - [10.1 Action binding (existing)](#101-action-binding-existing)
    - [10.2 Extended component mapping](#102-extended-component-mapping)
    - [10.3 Local alias pattern](#103-local-alias-pattern-preferred-for-variants)
11. [Notes on external systems (adapter-only)](#11-notes-on-external-systems-adapter-only)
12. [Quick adoption checklist](#12-quick-adoption-checklist)
13. [Governance and versioning](#13-governance-and-versioning)
    - [13.1 What counts as breaking (MAJOR)](#131-what-counts-as-breaking-major)
    - [13.2 What is non-breaking (MINOR / PATCH)](#132-what-is-non-breaking-minor--patch)
    - [13.3 Deprecation policy (recommended)](#133-deprecation-policy-recommended)
    - [13.4 Canonical token summary](#134-canonical-token-summary-contract-surface)

**Appendices**
- [Appendix A — Material 3 shape scale reference](#appendix-a--material-3-shape-scale-reference-adapter-mapping)
- [References](#references)

---

## 1. Purpose and scope

This spec defines **shape tokens beyond actions** — a consumer-semantic subsystem for **edge softness** and **corner roundedness**.

It:

- Describes **edge softness** and **corner roundedness** in terms of *perceived friendliness* rather than pixel values.
- Provides a **bounded reference basis** that can be adapter-mapped to Material 3 (and other) shape scales.
- Exposes **semantic endpoints** for surfaces and components based on their **role** in the UI.
- Supports an optional **shape mode knob** for brand expression: `sharp | smooth | round`.

Shape tokens communicate personality and help users understand **UI boundaries, groupings, and interactive affordances**.

**Out of scope:** layering/elevation (D4 — see [`cem-layering.md`](./cem-layering.md)), stroke thickness (D5), spacing/insets (D1), and operable/touch geometry (D2) — though shape must integrate with them (see §8 Accessibility section).

---

## 2. Why "bend" (token naming rationale)

In CEM terms, **bend** names the degree to which an edge curves away from a sharp corner. This framing:

- Avoids ambiguity with CSS `border-radius` (which can be a length, percentage, or elliptical).
- Emphasizes the *perceived* quality rather than the *implementation* detail.
- Supports consumer-semantic intent: "How friendly/formal does this surface feel?"

**Rule:** product code should use `--cem-bend-*` endpoints; implementation primitives (`border-radius`, elliptical radii,
corner pairs, etc.) remain an adapter/implementation detail.

---

## 3. CEM alignment principles applied to shape

### 3.1 Semantic intent first

Developers should apply tokens like:

- `--cem-bend-surface`, `--cem-bend-control`, `--cem-bend-overlay` (role semantics)
- `--cem-bend` (active bend value, mode-switchable)

…and not value-centric names like `radius-8` in component code.

This mirrors common design-token practice: values are "primitive/global," while **semantic/alias tokens** name where/how a
value is used. Keep product code semantic; keep value plumbing in the token layer.

### 3.2 Bounded variation

A small set of *distinct, recognizable* bend steps should cover most UI. The range goes from:

- `sharp` (none) → `smooth` (small bend) → `round-ends` (capsule) → `circle`

CEM applies a "bounded token budget" heuristic: avoid a long ladder of barely-distinguishable bend steps. If you need a new
step, justify it by a perceptual difference that survives dense mode, high zoom, and high-contrast overrides.

### 3.3 Layering (basis → semantic endpoints → adapters)

- **Basis tokens** define "what bend exists" (sharp / smooth / round-ends / circle).
- **Semantic endpoints** define "where bend is used" (surface / control / overlay / media).
- **Adapters** map external systems (Material / Fluent / etc.) into the basis and endpoints.

This keeps complexity low while still supporting multiple libraries.

---

## 4. Minimal bend basis (complexity-compliant)

This is the smallest stable basis that supports most UIs without overfitting.

```css
:root {
  /* Sharp = no bend */
  --cem-bend-sharp: 0;

  /* Smooth = small, friendly rounding; bind to D1 dimension scale */
  --cem-bend-smooth: var(--cem-dim-x-small); /* 0.5rem / 8px */

  /*
    Round ends (capsule / pill):
    Semicircle ends require bend = 0.5 * element height.
    Uses --cem-control-height from D2 (cem-coupling.md) as default.
    Provide --cem-shape-height where --cem-control-height is not appropriate.
  */
  --cem-bend-round: calc(var(--cem-shape-height, var(--cem-control-height)) / 2);

  /* True circle (avatars): percentage is defined relative to the element box. */
  --cem-bend-circle: 50%;

  /* Active bend used by endpoints that want the "default feel" (mode-switchable). */
  --cem-bend: var(--cem-bend-smooth);
}
```

**Cross-references:**
- `--cem-dim-x-small` is defined in [`cem-dimension.md`](./cem-dimension.md) §5
- `--cem-control-height` is defined in [`cem-coupling.md`](./cem-coupling.md) §4.2

---

## 5. Semantic endpoints (product-facing contract)

These are the tokens components should consume.

```css
:root {
  /* Core roles */
  --cem-bend-control: var(--cem-bend);
  --cem-bend-surface: var(--cem-dim-small);       /* 0.75rem / 12px — perceptual container geometry */
  --cem-bend-overlay: var(--cem-bend);            /* menus, tooltips, small popovers */

  /* Common specializations (keep minimal) */
  --cem-bend-field:              var(--cem-bend-control);
  --cem-bend-control-round-ends: var(--cem-bend-round);  /* optional capsule control variant */
  --cem-bend-modal:              calc(var(--cem-dim-large) + var(--cem-dim-xx-small)); /* ~28px — larger overlays */

  /* Media */
  --cem-bend-media:  var(--cem-bend);
  --cem-bend-avatar: var(--cem-bend-circle);
}
```

**Cross-references:**
- `--cem-dim-small`, `--cem-dim-large`, `--cem-dim-xx-small` are defined in [`cem-dimension.md`](./cem-dimension.md) §5

---

## 6. Shape mode knob (brand expression)

A shape mode knob is an **optional** product-level control that shifts bend across the UI to express a consistent brand
personality. It is intentionally coarse-grained: three modes are usually enough.

### 6.1 Mode meanings

| Mode      | Product intent                    | Typical bend policy (what changes)                                                                                                           | Perceived feel         |
|-----------|-----------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------|------------------------|
| `sharp`   | Technical, precise, enterprise    | Default bend moves toward `--cem-bend-sharp`; surfaces/overlays may remain slightly bent to preserve grouping and layer separation           | Formal, authoritative  |
| `smooth`  | Balanced, modern (default)        | Canonical defaults (base `--cem-bend-smooth`, surfaces at D1 "small")                                                                        | Friendly, professional |
| `round`   | Consumer, welcoming, approachable | Surfaces step up (e.g., D1 "medium"); optional capsule controls via `--cem-bend-control-round-ends` where the product uses them consistently | Warm, inviting         |

**Critical constraint:** shape modes must not change meaning. A user should not need to infer state or priority from
roundness alone (see §8 Accessibility section).

### 6.2 Mode implementation (recommended)

Implement modes via a theme selector (attribute or class) and override **only** the base and/or semantic endpoints.
Avoid an in-CSS "mode selector variable" (e.g., `--cem-shape-mode: round`) because CSS cannot branch on it without a
preprocessor; it also obscures where the mode is applied.

```css
/* Default / smooth */
:root,
:root[data-cem-shape="smooth"] {
  --cem-bend: var(--cem-bend-smooth);
  --cem-bend-control: var(--cem-bend);
  --cem-bend-overlay: var(--cem-bend);
  --cem-bend-surface: var(--cem-dim-small);
}

/* Sharp */
:root[data-cem-shape="sharp"] {
  --cem-bend: var(--cem-bend-sharp);

  /* Minimum resilience policy (recommended): keep containers slightly bent unless the brand explicitly wants boxy UI */
  --cem-bend-surface: var(--cem-bend-smooth);
  --cem-bend-overlay: var(--cem-bend-smooth);
  --cem-bend-control: var(--cem-bend); /* sharp controls */
}

/* Round */
:root[data-cem-shape="round"] {
  /* Keep the default control bend unless capsules are a deliberate product-wide choice */
  --cem-bend: var(--cem-bend-smooth);

  --cem-bend-surface: var(--cem-dim-medium);
  --cem-bend-overlay: var(--cem-bend-smooth);

  /* Optional: enable capsules consistently (brand decision) */
  /* --cem-bend-control: var(--cem-bend-round); */
  /* --cem-bend-field:   var(--cem-bend-round); */
}
```

**If you introduce more granular local aliases** (e.g., card vs page surface), they may also be overridden by the mode
selector, but only as pointers to existing endpoints. Do not mint a parallel "mode-specific" token family.

### 6.3 Helper classes (optional; scoped overrides)

If you need one-off local overrides (e.g., a demo page or controlled experiment), helper classes are acceptable, but should
not replace theme-level mode selectors.

```css
.cem-bend-smooth { --cem-bend: var(--cem-bend-smooth); }
.cem-bend-round  { --cem-bend: var(--cem-bend-round);  }
.cem-bend-sharp  { --cem-bend: var(--cem-bend-sharp);  }
```

Prefer overriding **semantic endpoints** (`--cem-bend-control`, `--cem-bend-surface`) rather than the global `--cem-bend`
when working inside components.

---

## 7. Directional and asymmetric patterns

### 7.1 Attachment pattern (one pattern, not many tokens)

When a surface is **attached** (sheet, drawer), do not round the attached edge.

```css
:root {
  --cem-bend-attached-edge: var(--cem-bend-sharp);
  --cem-bend-free-edge:     var(--cem-bend-modal);
}

/* Example: bottom sheet */
.cem-bottom-sheet {
  border-radius:
    var(--cem-bend-free-edge)
    var(--cem-bend-free-edge)
    var(--cem-bend-attached-edge)
    var(--cem-bend-attached-edge);
}
```

### 7.2 When to use asymmetric corners

Use asymmetric bend only when it communicates **composition**:

- **Attached elements:** popovers, sheets, or dropdowns visually "connected" to a trigger or viewport edge
- **Grouped controls:** segmented buttons / grouped toggles where only the outer corners are rounded
- **Sectioned containers:** card headers/footers that inherit top/bottom corners from the parent surface

Do **not** use asymmetric corners as the sole carrier of meaning or state (see §8 Accessibility section).

### 7.3 Implementation guidance

- Prefer **logical corner properties** so behavior is correct in RTL and other writing modes:
  `border-start-start-radius`, `border-start-end-radius`, `border-end-start-radius`, `border-end-end-radius`.
- Reuse existing semantic/basis tokens (`--cem-bend-control`, `--cem-bend-overlay`, `--cem-bend-attached-edge`,
  `--cem-bend-free-edge`) rather than minting new global "top/start/end" tokens.

### 7.4 Common recipes

**1) Popover attached to a trigger (attached on top, free on bottom)**

```css
.cem-popover--attached-top {
  border-start-start-radius: var(--cem-bend-attached-edge);
  border-start-end-radius:   var(--cem-bend-attached-edge);
  border-end-start-radius:   var(--cem-bend-overlay);
  border-end-end-radius:     var(--cem-bend-overlay);
}
```

**2) Segmented / grouped controls (outer corners only)**

```css
.cem-segment {
  border-radius: 0; /* inner corners remain sharp */
}

.cem-segment:first-child {
  border-start-start-radius: var(--cem-bend-control);
  border-end-start-radius:   var(--cem-bend-control);
}

.cem-segment:last-child {
  border-start-end-radius: var(--cem-bend-control);
  border-end-end-radius:   var(--cem-bend-control);
}
```

**3) Card header inherits top corners, body stays rectangular**

```css
.cem-card {
  border-radius: var(--cem-bend-surface);
  overflow: clip; /* ensure header background respects corners */
}

.cem-card__header {
  border-start-start-radius: var(--cem-bend-surface);
  border-start-end-radius:   var(--cem-bend-surface);
  border-end-start-radius:   0;
  border-end-end-radius:     0;
}
```

**Optional local shorthand (avoid globals):** If you need `border-radius` shorthand, keep it component-scoped and do not
encode LTR assumptions into global `--cem-bend-start/end` variables.

---

## 8. Accessibility and inclusive design requirements

Bend is a *secondary* visual cue. It must never be the only carrier of state, meaning, or instructions.

### 8.1 Shape must not be the only signal

Do not encode meaning using bend alone (e.g., "rounded = primary" / "square = secondary") unless there is a redundant cue
(color, text, icon, pattern, or stroke).

### 8.2 High-contrast and forced-colors resilience

High-contrast modes can override fills/shadows and reduce the perceptibility of subtle shape differences. Treat bend deltas
smaller than "sharp vs smooth" as **non-essential**. For critical distinctions:

- rely on **D5 stroke/outline** ([`cem-stroke.md`](./cem-stroke.md)) and **D0 color semantics**
- provide non-color redundancy (icon/text) for state/meaning

### 8.3 Focus indicators must respect the target

Focus rings and outlines must align with the target's bend, and must remain visible at high zoom and in forced-colors.

Practical guidance (cross-dimension):

- **D1:** reserve ring space via offset/inset so the ring does not collide with corners or get clipped (see [`cem-dimension.md`](./cem-dimension.md)).
- **D5:** define focus stroke width and style via `--cem-stroke-focus`; do not rely on bend to indicate focus (see [`cem-stroke.md`](./cem-stroke.md) §5).
- **D2:** ensure focus ring does not reduce effective hit size; hit targets remain compliant (see [`cem-coupling.md`](./cem-coupling.md)).

Recommended robust pattern (shape-aligned ring in normal mode, resilient in forced-colors):

```css
.cem-focusable:focus-visible {
  /* Baseline indicator that survives forced-colors */
  outline: var(--cem-stroke-focus, 2px) solid transparent;
  outline-offset: var(--cem-stroke-indicator-offset, 2px); /* D5 indicator placement */

  /* Primary ring for normal mode (shape-aligned) */
  box-shadow: 0 0 0 var(--cem-stroke-focus, 2px) currentColor;

  border-radius: var(--cem-bend-control);
}
```


### 8.4 Pointer target size and spacing (bend × D2 size × D1 separation)

Round controls can *look* smaller even when they meet minimum targets. Ensure:

- D2 control height/width targets are met (especially icon buttons and small chips) — see [`cem-coupling.md`](./cem-coupling.md)
- D1 spacing provides separation between adjacent targets — see [`cem-dimension.md`](./cem-dimension.md)

### 8.5 Bend vs inset readability (bend × D1 padding)

High bend on compact surfaces increases corner curvature and can visually crowd content. When bend increases, step D1 inset
up rather than pushing content into the curve.

### 8.6 Minimum test checklist

Validate under:

- keyboard navigation with `:focus-visible`
- forced-colors / high-contrast modes
- zoom 200–400% and text scaling (no clipping; adequate spacing)
- dense vs normal modes (D2 height changes keep round-ends correct)
- small-target components (icon buttons, chips) for hit area and separation

---

## 9. Consumer vocabulary for "round ends" (capsules)

Recommended consumer-perspective terms (descending literalness):

- **round-ends**
- **capsule**
- **pill**
- **bubble button** (marketing-leaning)
- **stadium** (designer-leaning)

**Canonical endpoint name in this spec:** `--cem-bend-control-round-ends`

If you want an alias, make it a pointer:

```css
--cem-bend-control-capsule: var(--cem-bend-control-round-ends);
```

---

## 10. Component mapping

This section provides a practical "where to bind bend" reference. It is intentionally **role-first** and maps components to
existing CEM endpoints. If a component needs an internal override, implement it as a **local alias** (component scope),
not as new global tokens.

### 10.1 Action binding (existing)

The existing implementation binds actions to the bend system:

```css
.action, button {
  --cem-action-border-radius: var(--cem-bend);
  border-radius: var(--cem-action-border-radius);
}
```

### 10.2 Extended component mapping

The mapping below is consistent with common practices across modern systems:

- Material 3 applies the corner radius scale broadly to rectangular components (e.g., cards, dialogs, menus) and uses larger
  radii for prominent overlays (dialogs are specified with 28dp container corners).
- Fluent groups shape usage into rectangular elements, flyout elements (popovers), and pill/round elements (personas), and
  recommends "none" for structural bars.
- Radix Themes treats radius as a theme-level property inherited by panels like Card/Dialog/Popover.
- Polaris documents concrete radii for cards (e.g., default 8px).
- Carbon specifies radii for certain control families (e.g., content switcher corners).

**Table: map components to CEM endpoints**

| Component family                     | Recommended token / basis                                                            | Rationale (consumer + system)                                                                                |
|--------------------------------------|--------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------|
| **Buttons**                          | `--cem-bend-control`                                                                 | Standard interactive affordance (default feel)                                                               |
| **Icon buttons (circular variants)** | Local alias → `border-radius: var(--cem-bend-circle)`                                | Icon buttons often present as circles; use basis circle via local alias (do not mint a global "icon radius") |
| **FAB**                              | Circle variant → `--cem-bend-circle`; Extended FAB → `--cem-bend-control-round-ends` | Prominent control; circular (classic FAB) or capsule (extended FAB)                                          |
| **Chips / Tags**                     | `--cem-bend-control-round-ends`                                                      | "Capsule" shape signals lightweight, removable, filter-like affordance                                       |
| **Badges / Counters**                | `--cem-bend-smooth` (or `--cem-bend-control`)                                        | Small rounding prevents harsh micro-shapes; keep subtle                                                      |
| **Text fields / Select**             | `--cem-bend-field`                                                                   | Field geometry should match the product's primary control feel                                               |
| **Switches / Toggles**               | Track: `--cem-bend-round`; Thumb: `--cem-bend-circle`                                | Track is a pill; thumb is a circle (distinct affordance)                                                     |
| **Cards / Panels**                   | `--cem-bend-surface`                                                                 | Container grouping and boundaries                                                                            |
| **Tables / Dense list rows**         | `--cem-bend-sharp` (or none)                                                         | Grid-aligned surfaces should remain formal/structural; avoid decorative rounding                             |
| **Dialogs / Sheets**                 | `--cem-bend-modal` (+ attachment pattern for sheets)                                 | High salience overlays; larger bend supports "layered" reading and separation                                |
| **Menus / Dropdowns / Popovers**     | `--cem-bend-overlay`                                                                 | Transient overlays; match default bend feel                                                                  |
| **Tooltips**                         | `--cem-bend-overlay`                                                                 | Small transient overlay; keep consistent with menus                                                          |
| **Snackbars / Toasts**               | `--cem-bend-overlay` (policy may choose sharper)                                     | Notification surfaces should not demand attention via shape alone                                            |
| **Navigation rail / App bar**        | `--cem-bend-sharp`                                                                   | Structural navigation regions read more stable with sharp edges                                              |
| **Tabs**                             | Underline tabs: `--cem-bend-sharp`; Pill tabs: `--cem-bend-control-round-ends`       | Keep minimal unless the design intentionally uses pill tabs                                                  |
| **Avatars / Personas**               | `--cem-bend-avatar` (`50%`)                                                          | Circular identity affordance                                                                                 |
| **Media thumbnails**                 | `--cem-bend-media`                                                                   | Gentle rounding to match overall feel without obscuring content edges                                        |

### 10.3 Local alias pattern (preferred for variants)

```css
.cem-icon-button {
  /* local alias: do not mint a global "icon radius" */
  border-radius: var(--cem-bend-circle);
}

.cem-chip {
  border-radius: var(--cem-bend-control-round-ends);
}
```

---

## 11. Notes on external systems (adapter-only)

Many systems use a **bounded corner scale** and recommend applying it by role:

- Material 3: "corner radius scale" across rectangular components; larger corners on prominent overlays.
- Fluent 2: "None…Circle" tokens and guidance by component area (structural bars vs flyouts vs personas).
- Radix Themes: theme-level radius inherited by panel-like components.
- Polaris and Carbon: documented radii for common container/control families.

CEM consumes these patterns at the adapter layer but keeps product code consumer-semantic.

---

## 12. Quick adoption checklist

Use this checklist to adopt the bend system with minimal churn and predictable outcomes.

1. **Confirm prerequisites (D1 + D2 are present)**
   - D1 provides the physical scale used by bend (`--cem-dim-xx-small`, `--cem-dim-x-small`, `--cem-dim-small`, etc.) — see [`cem-dimension.md`](./cem-dimension.md).
   - D2 provides height for geometry-driven bend (`--cem-control-height`), so round-ends remain correct across density/size modes — see [`cem-coupling.md`](./cem-coupling.md).

2. **Adopt the bend basis (`--cem-bend-*`)**
   - Keep the basis small: `sharp`, `smooth`, `round` (endcaps), `circle`, and the active `--cem-bend`.
   - Decide product policy for "capsule controls" (where `--cem-bend-round` is used): occasional variant vs product-wide style.

3. **Map semantic endpoints to the basis (role-first)**
   - Set `--cem-bend-control`, `--cem-bend-surface`, `--cem-bend-overlay`, and the minimal specializations already defined in this spec.
   - Avoid mapping components directly to "scale aliases" (`xs/sm/md/...`) in product code. Those are adapter-only.

4. **Bind components to semantic endpoints (no raw values)**
   - Bind `border-radius` to endpoints (e.g., `--cem-bend-control`, `--cem-bend-surface`) or to basis geometry tokens (`--cem-bend-round`, `--cem-bend-circle`) via **local aliases** for variants.
   - For asymmetric corners (groups/attachments), use the patterns in this spec and prefer **logical corner properties** to remain RTL-safe.

5. **If needed, introduce a shape mode knob (`sharp | smooth | round`)**
   - Add `data-cem-shape="..."` only if brand expression requires it.
   - Modes should override **only** `--cem-bend` and/or semantic endpoints (never introduce mode-specific component tokens).

6. **Validate in accessibility and "hard mode" rendering**
   - **Focus ring follows target shape** and is not clipped (avoid `overflow: hidden` trapping focus; validate `outline-offset` / ring space).
   - **Forced-colors / high-contrast**: affordances remain clear when shadows/fills are overridden; outline-based focus remains visible.
   - **Grouped controls**: only outer corners rounded; inner corners sharp; RTL behavior correct.
   - **Overlays** (tooltips/menus/popovers) are consistent with `--cem-bend-overlay`; **modals** use `--cem-bend-modal` (M3 dialogs are typically 28dp).
   - **Chips and badges** maintain pill/circular intent across density modes (round ends use ½-height rule).
   - **Target size & spacing** remain compliant when shapes look visually smaller (particularly icon buttons and chips). Coordinate with D2 sizing and D1 separation.
   - **Focus not obscured** by sticky headers/overlays during keyboard navigation.

7. **Lock in governance**
   - Add a code convention (or lint rule) that forbids raw `border-radius` values in components except inside adapter layers.
   - Document which endpoints are allowed for each component family (use the Component mapping table as the baseline).

---

## 13. Governance and versioning

This spec is intended to be adopted as a **stable contract** between design, engineering, and component libraries. Treat
changes using **semantic versioning** (MAJOR.MINOR.PATCH) with explicit deprecation when feasible.

### 13.1 What counts as breaking (MAJOR)

Treat as major (breaking) if you:

- Rename or remove any **canonical** bend basis token (`--cem-bend-*`) or any **canonical semantic endpoint**.
- Change the **semantic meaning** of an endpoint (e.g., what "overlay" covers; whether "round" means endcaps vs fixed value).
- Change the **geometry definition** of round-ends (`½ height`) or circle (`50%`) in a way that alters component behavior.
- Change mode names, add/remove supported modes, or change the meaning of an existing mode selector
  (e.g., `data-cem-shape="sharp|smooth|round"`).

**Business rationale:** these changes can silently alter UI boundaries, focus geometry, and density-mode behavior across many
components at once.

### 13.2 What is non-breaking (MINOR / PATCH)

Treat as minor/patch if you:

- Adjust underlying numeric values (via D1 tokens) while preserving semantics and perceptual intent.
- Add new semantic endpoints that are **optional**, clearly scoped, and do not change existing endpoint meaning.
- Add mapping guidance, adapter notes, examples, or clarify documentation.
- Expand component mapping tables without changing existing bindings.

### 13.3 Deprecation policy (recommended)

- When replacing an endpoint, keep the old name as an **alias** for at least one minor release cycle.
- Provide a migration note: "old → new," and a rationale tied to consumer semantics and accessibility.

### 13.4 Canonical token summary (contract surface)

The table below distinguishes **required CEM contract tokens** from **optional adapter aliases**.

| Token                           | Category                    |   Required     | Notes                                                 |
|---------------------------------|-----------------------------|:--------------:|-------------------------------------------------------|
| `--cem-bend-sharp`              | Bend basis                  |      Yes       | No bend (sharp corners)                               |
| `--cem-bend-smooth`             | Bend basis                  |      Yes       | Small bend; binds to D1 `--cem-dim-x-small`           |
| `--cem-bend-round`              | Bend basis (geometry)       |      Yes       | Round-ends (capsule): `½ height`                      |
| `--cem-bend-circle`             | Bend basis (geometry)       |      Yes       | Circle: `50%`                                         |
| `--cem-bend`                    | Active alias                |      Yes       | Mode-switchable default bend                          |
| `--cem-bend-control`            | Semantic endpoint           |      Yes       | Primary control bend                                  |
| `--cem-bend-surface`            | Semantic endpoint           |      Yes       | Primary container/surface bend                        |
| `--cem-bend-overlay`            | Semantic endpoint           |      Yes       | Small overlays (menus/tooltips/popovers)              |
| `--cem-bend-field`              | Semantic endpoint           |  Recommended   | Usually equals control bend                           |
| `--cem-bend-modal`              | Semantic endpoint           |  Recommended   | Prominent overlays (dialogs/sheets)                   |
| `--cem-bend-control-round-ends` | Semantic endpoint (variant) |    Optional    | Use only if capsules are a consistent product pattern |
| `--cem-bend-media`              | Semantic endpoint           |  Recommended   | Media thumbnails/previews                             |
| `--cem-bend-avatar`             | Semantic endpoint           |  Recommended   | Avatar/persona geometry (circle)                      |
| `--cem-bend-attached-edge`      | Pattern token               |  Recommended   | For asymmetric attachment patterns                    |
| `--cem-bend-free-edge`          | Pattern token               |  Recommended   | For asymmetric attachment patterns                    |
| `--cem-action-border-radius`    | Component binding           | Yes (existing) | Existing action binding contract                      |

**Adapter-only (optional) aliases**

The following "M3-parity" aliases may be exposed for compatibility, but must not be required by product component code:

- `--cem-bend-none`, `--cem-bend-xs`, `--cem-bend-sm`, `--cem-bend-md`, `--cem-bend-lg`, `--cem-bend-xl`, `--cem-bend-full`

If these aliases exist, treat them as adapter surface: renames/removals are breaking for adapter consumers, but should not
affect product components if semantic endpoints are followed.

---

## Appendix A — Material 3 shape scale reference (adapter mapping)

This appendix is **non-normative**: it exists to help adapters map Material 3 shape scales into the CEM bend basis and
semantic endpoints.

Material Design 3 defines a corner-radius scale with canonical steps (0/4/8/12/16/28 + "full"), and also introduces
additional "increased" steps for larger containers in newer guidance.

### A.1 Canonical M3 `--md-sys-shape-corner-*` steps → CEM

| M3 token                            | Typical value  | CEM mapping (adapter-only)                                       | Prefer applying via CEM endpoints                    | Perceived quality        |
|-------------------------------------|----------------|------------------------------------------------------------------|------------------------------------------------------|--------------------------|
| `--md-sys-shape-corner-none`        | 0              | `--cem-bend-sharp`                                               | attached edges, structural bars (`--cem-bend-sharp`) | Sharp, formal, technical |
| `--md-sys-shape-corner-extra-small` | 4dp            | `var(--cem-dim-xx-small)`                                        | rare micro-bend (badges, subtle containers)          | Barely rounded, subtle   |
| `--md-sys-shape-corner-small`       | 8dp            | `--cem-bend-smooth`                                              | `--cem-bend-control`, `--cem-bend-overlay`           | Slightly soft            |
| `--md-sys-shape-corner-medium`      | 12dp           | `var(--cem-dim-small)`                                           | `--cem-bend-surface`                                 | Balanced friendliness    |
| `--md-sys-shape-corner-large`       | 16dp           | `var(--cem-dim-medium)`                                          | `--cem-bend-surface-strong` (if you expose it)       | Noticeably rounded       |
| `--md-sys-shape-corner-extra-large` | 28dp           | `--cem-bend-modal`                                               | `--cem-bend-modal`                                   | Very soft, approachable  |
| `--md-sys-shape-corner-full`        | pill / 50%     | `--cem-bend-round` (capsule) and/or `--cem-bend-circle` (circle) | chips, capsule controls, avatars                     | Playful, highly rounded  |

### A.2 Optional M3-parity aliases (adapter-only)

If you want to expose M3-like short aliases for compatibility, define them as **aliases** to CEM basis and D1 tokens.
These are **not** intended for product component code.

```css
:root {
  --cem-bend-none: var(--cem-bend-sharp);
  --cem-bend-xs:   var(--cem-dim-xx-small); /* ~4dp */
  --cem-bend-sm:   var(--cem-dim-x-small);  /* ~8dp */
  --cem-bend-md:   var(--cem-dim-small);    /* ~12dp */
  --cem-bend-lg:   var(--cem-dim-medium);   /* ~16dp */
  --cem-bend-xl:   var(--cem-bend-modal);   /* ~28dp */

  /* "full" depends on geometry: capsule vs circle */
  --cem-bend-full: var(--cem-bend-round);
}
```

### A.3 Notes on validation vs "current" libraries

- Material Web uses `--md-sys-shape-corner-*` tokens in theming and leaves exact numeric assignments to the system theme.
- Angular Material may publish/ship a scale whose *numeric* values differ from the M3 "typical" table; treat Angular's
  scale as its **own source scale** and map to CEM by intent, not by exact dp numbers.
- CEM's contract is semantic endpoints. Adapters own external token mapping.

---

## References

**Primary sources**
- [Material Design 3: Shape (shape scale tokens)](https://m3.material.io/styles/shape/shape-scale-tokens)
- [Material Design 3: Corner radius scale](https://m3.material.io/styles/shape/corner-radius-scale)
- [Material Web: Theming](https://material-web.dev/theming/material-theming/)
- [Angular Material: Design Tokens](https://deepwiki.com/angular/components/5.2-material-design-tokens)
- [Google Developers: Building the Shape System](https://developers.googleblog.com/building-the-shape-system-for-material-design/)

**Additional ecosystem references (non-normative)**
- [Fluent 2: Shapes](https://fluent2.microsoft.design/shapes)
- [Radix Themes: Radius](https://www.radix-ui.com/themes/docs/theme/radius)
- [Shopify Polaris: Card](https://polaris-react.shopify.com/components/layout-and-structure/card)
- [Carbon: Content switcher style](https://carbondesignsystem.com/components/content-switcher/style/)

**Local CEM documentation**
- [D1. Space & Rhythm](./cem-dimension.md) — dimension scale tokens
- [D2. Coupling & Compactness](./cem-coupling.md) — control height and operability
- [D6. Typography](./cem-voice-fonts-typography.md) — voice and typography tokens
- [D7. Time & Motion](./cem-timing.md) — timing and easing tokens
