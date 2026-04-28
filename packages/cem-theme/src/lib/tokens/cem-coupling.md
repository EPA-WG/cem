# Semantic Density and Coupling Tokens (CEM) — Canonical Spec

**Status:** Canonical (v1.0)

**Last updated:** December 17, 2025

**Audience:** Design Systems, Product Design, Front-End Engineering

**Applies to:** Modality-neutral interactive operability (pointer, touch, stylus, gaze, switch, remote)

**Companion specs:**
- **D0. Color (Emotional Palette)** ([`cem-colors.md`](./cem-colors.md)) — color intensity perception affected by density
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — gaps/insets/rhythm; must never violate D2 safety minimums
- **D2c. Controls** ([`cem-controls.md`](./cem-controls.md)) — visual control geometry (`--cem-control-*`, icon-button, row heights) and per-mode visual overrides
- **D3. Shape — Bend** ([`cem-shape.md`](./cem-shape.md)) — bend consumes `--cem-control-height` from D2c Controls for round-ends
- **D4. Layering** ([`cem-layering.md`](./cem-layering.md)) — overlay/modal control sizing
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — focus ring offset; D2 constrains adjacent zones so indicators have room
- **D7. Time & Motion** ([`cem-timing.md`](./cem-timing.md)) — timing for density transitions

---

## 1. Why "Coupling"

In CEM terms, **coupling** names the moment when **user intent successfully binds to an interactive affordance**—independent of input modality:

- pointer (mouse/trackpad/stylus)
- direct touch (touchscreen)
- gaze (eye tracking + dwell)
- switch scanning (highlight + select)
- remote / gamepad focus navigation

This supports the UI-space meaning you called out: proximity implies **relationship/coupling between controls**, and insufficient distancing causes **interference**.

---

## 2. Taxonomy placement

- **D2. Coupling & Compactness** (this canvas)
  - operable zone minimums
  - interference/isolation minimums
  - halo/expansion policy (visual size vs operable size)
  - mode-invariant operability rules

- **D2c. Controls** (from [`cem-controls.md`](./cem-controls.md))
  - visual control geometry (heights, paddings, icon-button size, row heights)
  - per-coupling-mode visual overrides
  - D2c geometry must never reduce operability below D2 safety minimums

- **D1. Space & Rhythm** (from [`cem-dimension.md`](./cem-dimension.md))
  - gaps/insets/rhythm
  - may *suggest* coupling semantics, but must never violate D2 safety minimums

- **D5. Stroke & Separation** (from [`cem-stroke.md`](./cem-stroke.md))
  - focus/selection/target indicator thickness and indicator placement (e.g., `--cem-stroke-focus`, `--cem-stroke-indicator-offset`, zebra ring tokens)
  - D2 does not define focus-ring offset; D2 constrains adjacent operable zones so D5 indicators have room and do not create interference

Boundary heuristic:

- If it changes **distance between items** → D1
- If it changes **the operable safety contract** (zone/guard/halo) → D2
- If it changes **the visual size of a single control or row** → D2c (see [`cem-controls.md`](./cem-controls.md))
- If it changes **indicator thickness or indicator offset** → D5 (e.g., `--cem-stroke-focus`, `--cem-stroke-indicator-offset`)

---

## 3. Coupling vocabulary (semantic + dimensional)

To keep the naming dimensional and unambiguous, use three core nouns:

- **Zone**: the operable area where intent can couple
- **Halo**: invisible expansion beyond visuals (to meet zone minimums without enlarging chrome)
- **Guard**: the minimum distancing preventing interference between adjacent zones

---

## 4. Canonical D2 coupling tokens

### 4.1 Hard safety minimums (policy anchors)

These values are **mode-invariant** — density modes MUST NOT change `zone-min` or `guard-min`.

###### cem-coupling-minimums
| Token | Value | Description | tier |
|---|---|---|---|
| `--cem-coupling-zone-min` | `3rem` | Minimum operable zone in both dimensions; nominally 48 px @ 16 px root | required |
| `--cem-coupling-guard-min` | `0.5rem` | Minimum distancing between adjacent operable zones; nominally 8 px | required |
| `--cem-coupling-halo` | `0.25rem` | Invisible expansion beyond visuals; may vary by mode (nominally 4 px) | required |

Interpretation:

- **Zone** answers: “Can the user reliably couple intent to this control?”
- **Guard** answers: “Will adjacent controls accidentally steal intent?”
- **Halo** answers: “Can we keep visuals compact while preserving operability?”

Accessibility baseline:

- WCAG 2.2 AA Success Criterion 2.5.8 (Target Size Minimum) establishes a **24 × 24 CSS px** baseline for pointer
  targets, with documented exceptions for inline text, spacing, equivalent controls, user-agent controls, essential
  presentation, and controls where target spacing prevents interference.
- CEM deliberately sets a stronger default operable zone: `--cem-coupling-zone-min: 3rem` (48 px at a 16 px root).
  This aligns with common Android / Material-style 48dp touch-target guidance and gives room for touch, stylus,
  gaze/dwell, remote focus, and switch-scanning contexts.
- `--cem-coupling-guard-min: 0.5rem` (8 px at a 16 px root) aligns with common 8dp spacing guidance and prevents
  adjacent operable zones or external focus indicators from visually or functionally colliding.
- Components may use the WCAG 24 × 24 minimum only where the WCAG spacing/exception model is deliberately validated.
  The CEM default remains 48 × 48 because it is modality-neutral and safer for mixed-input products.

### 4.1.1 D2/D5 compatibility: guard must cover indicator outset (normative)

When D5 indicators render **outside** the control edge (e.g., `outline-offset`, zebra rings),
`--cem-coupling-guard-min` SHOULD be large enough to prevent indicator collisions between adjacent controls.

Define the worst-case outward indicator extent as:

- Zebra ring (contrast): `4 * --cem-zebra-strip-size`
- Outline ring: `--cem-stroke-indicator-offset + --cem-stroke-focus`

Compatibility guideline:

`--cem-coupling-guard-min` SHOULD be ≥
`max(4 * --cem-zebra-strip-size, --cem-stroke-indicator-offset + --cem-stroke-focus)`

Default CEM values satisfy this (8px guard vs 8px zebra extent when `--cem-zebra-strip-size = 2px`).

### 4.2 Control geometry endpoints (visual sizing — D2c Controls)

Visual control geometry — `--cem-control-height`, `--cem-control-padding-x`, `--cem-control-padding-y`,
`--cem-icon-button-size`, `--cem-icon-button-icon-size`, and the list/menu/table row heights — is **defined and
governed in D2c Controls** ([`cem-controls.md`](./cem-controls.md)).

D2 references this geometry only as a normative constraint: visual sizes MUST NOT shrink the operable zone below
`--cem-coupling-zone-min`, and adjacent layouts MUST NOT shrink distancing below `--cem-coupling-guard-min`,
regardless of how Controls geometry varies across coupling modes. When visuals fall below the zone, components MUST
use halo expansion (see §6.1).

Coupling-mode overrides for visual geometry live in D2c Controls; D2 owns only the operability portions of mode
behavior (halo policy, see §5).

---

## 5. Coupling modes (consumer semantics)

### 5.1 Mode meanings

- **forgiving**
  - optimized for imprecise input: thumbs, gloves, kiosks, gaze/dwell
  - larger visual controls; halo may be smaller because visuals already provide the zone

- **balanced**
  - default for mixed mouse/touch usage

- **compact**
  - optimized for scan-heavy/data-rich screens
  - visuals may get tighter, but **coupling remains safe via halo + guard**

### 5.2 Mode switch (halo policy)

D2 owns only the halo portion of mode behavior. Visual geometry overrides live in D2c Controls
(see [`cem-controls.md`](./cem-controls.md) §3.2).

```css
:root { --cem-coupling: balanced; }

:root[data-cem-coupling="balanced"] { /* baseline halo */ }

:root[data-cem-coupling="forgiving"] {
  /* Visuals are larger (D2c); halo can be smaller because visuals already meet the zone */
  --cem-coupling-halo: 0.125rem;
}

:root[data-cem-coupling="compact"] {
  /* Visuals shrink (D2c); halo grows to preserve the operable zone */
  --cem-coupling-halo: 0.375rem;
}
```

###### cem-coupling-halo-overrides
| Token                 | forgiving  | compact    |
|-----------------------|------------|------------|
| `--cem-coupling-halo` | `0.125rem` | `0.375rem` |

Governance rule: **do not reduce** `--cem-coupling-zone-min` or `--cem-coupling-guard-min` per mode. Visual geometry
overrides are governed in D2c Controls.

---

## 6. Implementation patterns

### 6.1 Halo-based operability (compact visuals, safe coupling)

`--cem-control-padding-*` come from D2c Controls; the safety floor (`min-block-size: --cem-coupling-zone-min`) is the
D2 contribution.

Token generation is necessary but not sufficient. A component is only coupling-safe when its rendered implementation
preserves the zone, guard, and halo contract:

- Use `min-block-size` and `min-inline-size` when the visible element itself owns the operable zone.
- Use a wrapper or pseudo-element halo when visual chrome is intentionally smaller than the zone.
- Prevent halo overlap by combining D1 spacing with D2 guard rules.
- Compute dense cluster gaps as `gap = max(layout-gap, guard-min)`, not as a raw D1 spacing token.

```css
.cem-control {
  min-inline-size: var(--cem-coupling-zone-min);
  min-block-size: var(--cem-coupling-zone-min);
  padding-inline: var(--cem-control-padding-x);
  padding-block: var(--cem-control-padding-y);
}

/* Optional: halo expansion when visible chrome is smaller than the operable contract */
.cem-coupling-halo {
  position: relative;
}

.cem-coupling-halo::after {
  content: "";
  position: absolute;
  inset: calc(-1 * var(--cem-coupling-halo));
  background: transparent;
  pointer-events: auto;
}
```

### 6.2 D1/D2 enforcement: spacing must not violate coupling guard

```css
.cem-actions {
  display: flex;
  gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}
```

---

## 7. Optional: coupling semantics between controls (proximity implies relatedness)

If you want proximity to intentionally communicate relationship, add an optional D1/D2 bridge token set:

```css
:root {
  /* Within this distance, controls are perceived as a group (segmented, paired actions). */
  --cem-coupling-affinity-max: 0.375rem;

  /* Beyond this distance, controls read as independent. */
  --cem-coupling-decouple-min: 0.75rem;
}
```

Guidance:

- Keep **affinity/decouple** separate from **guard**.
- `guard` is safety; `affinity/decouple` is meaning.

---

---

## 8. Material / Angular Material mapping (implementation layer)

- Map `data-cem-coupling="forgiving|balanced|compact"` to the library’s density knob.
- Keep **CEM public semantics** stable; keep library numeric scales in the adapter.

Example mapping guidance:

- forgiving → comfortable / default density
- balanced → slightly denser
- compact → compact / denser

(Exact numeric mapping remains an implementation choice per Angular Material version.)

---

## 9. Finalization checklist (what “done” means)

### 9.1 Canonical decisions to lock

- **Category name:** D2. Coupling & Compactness (or D2. Coupling Safety)
- **Public axis name:** `coupling` (semantic), with exactly **three modes**: `forgiving | balanced | compact`
- **Core nouns:** zone / halo / guard
- **Units:** `rem` for all coupling and geometry values

### 9.2 Token set acceptance criteria

A draft becomes “canonical” when all of the following are true:

1) **Minimal core:** only these coupling policy anchors are required:
   - `--cem-coupling-zone-min`
   - `--cem-coupling-guard-min`
   - `--cem-coupling-halo`

2) **Invariants are explicit:**
   - `zone-min` and `guard-min` are **mode-invariant**
   - mode changes may adjust **visual geometry** and **halo** only

3) **Orthogonality is enforceable:**
   - a normative rule exists for layout code: `gap = max(D1 gap, D2 guard)`

4) **Component mapping is possible without special cases:**
   - for each major control family (buttons, icon buttons, chips, list rows, menu rows, table rows), you can map either:
     - visible size ≥ zone-min, or
     - visible size < zone-min with halo expansion

5) **Accessibility posture is documented:**
   - how zone-min relates to your a11y baseline
   - how halo avoids overlap and preserves guard

### 9.3 Mode matrix (canonical)

| Mode | Product intent | Visual geometry | Halo (`--cem-coupling-halo`) | Typical surfaces |
|---|---|---|---|---|
| `forgiving` | Minimize mis-coupling for imprecise input | Larger controls/rows; more internal padding | Smaller (visuals already meet zone) | mobile-first, kiosks, accessibility-first, gaze/dwell |
| `balanced` | General default across modalities | Baseline control heights/rows | Baseline | mixed pointer + touch, general app UI |
| `compact` | Increase information density without breaking operability | Smaller visual heights/rows; reduced chrome | Larger (use halo to preserve zone) | data grids, admin tools, scan-heavy panels |

Normative rule:

- `--cem-coupling-zone-min` and `--cem-coupling-guard-min` are **mode-invariant**.
- Modes may adjust **visual geometry** and **halo** only.

### 9.4 Component mapping checklist (how to apply)

Use this checklist to implement Coupling consistently. The same rules apply whether input is pointer, touch, stylus, gaze, switch scanning, or remote focus.

#### Common rules (apply to all components)

- **Zone rule:** each interactive affordance must provide an operable zone of at least `--cem-coupling-zone-min` in both dimensions *or* be wrapped/expanded so the operable zone meets the minimum.
- **Guard rule:** adjacent interactive affordances must not violate `--cem-coupling-guard-min` *between their operable zones*.
- **Halo rule:** if halo expansion is used, ensure halos do not overlap adjacent operable zones. For dense clusters, compute layout spacing with halo in mind.
- **State legibility:** focus/hover/pressed affordances must remain legible across modes (especially in `compact`).

#### Buttons

- Map `min-block-size` to `--cem-coupling-zone-min` (or set visible height ≥ zone-min).
- Use `--cem-control-height` and `--cem-control-padding-*` for visual sizing; do not shrink below operability. These
  tokens are owned by D2c Controls (see [`cem-controls.md`](./cem-controls.md)).
- For grouped buttons, enforce **guard** using layout `gap` (or dividers) that respects `--cem-coupling-guard-min`.

#### Icon buttons

- Prefer visible container size = `--cem-coupling-zone-min`.
- If the visible icon container is smaller, implement halo expansion on a wrapper.
- In clusters (toolbars), validate guard and halo non-overlap explicitly.

#### Chips / tags

- Visual chip height may be below `zone-min` in `compact`, but operability must still meet `zone-min` via:
  - a halo wrapper, or
  - additional padding on the interactive container.
- Ensure dismiss icons (×) meet the zone rule independently if separately interactive.

#### List rows / menu rows

- Ensure row **height** maps to at least `zone-min` where the row itself is the interactive affordance.
- If only a sub-region is interactive (e.g., trailing actions), each action must meet zone/guard rules.
- For trailing action clusters, apply guard via `gap` or reserved spacing to avoid interference.

#### Table rows / data grids

- It is acceptable for `--cem-table-row-height` (owned by D2c Controls) to be below `zone-min` *only if* row actions / selection / checkboxes still meet the zone rule via:
  - larger interactive containers within cells, or
  - halo expansion that does not overlap neighbors.
- Validate dense rows with:
  - selection controls (checkbox/radio)
  - row-level actions (kebab, inline icons)
  - row click targets (if the row is clickable)

### 9.5 Implementation proof points (required)

Validate each mode on at least one representative surface:

- **Forms:** text field + primary button + icon button cluster
- **Navigation list:** list rows + trailing actions
- **Data-heavy:** table with row actions and selection

Pass criteria:

- adjacent operable zones never violate `guard-min`
- no halo overlap between neighbors
- focus ring / hover affordances remain legible when visuals are compact (see [`cem-stroke.md`](./cem-stroke.md) §5 for focus indicator thickness)

#### Proof surface A: form trio

Use a text field, primary action, and icon action in one row. This validates mixed-width controls and compact visual
chrome.

```css
.cem-proof-form-trio {
  display: flex;
  align-items: center;
  gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}

.cem-proof-form-trio :is(input, button) {
  min-block-size: var(--cem-coupling-zone-min);
}

.cem-proof-form-trio .cem-icon-button {
  min-inline-size: var(--cem-coupling-zone-min);
  min-block-size: var(--cem-coupling-zone-min);
}
```

Checks:

- The input, primary button, and icon button each expose at least a 48 × 48 px operable zone by default.
- The row gap never drops below `--cem-coupling-guard-min`.
- D5 focus rings do not collide when focus moves between adjacent controls.

#### Proof surface B: navigation list with trailing actions

Use interactive rows with a trailing action cluster. This validates row affordances plus nested controls.

```css
.cem-proof-nav-list {
  display: grid;
  gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}

.cem-proof-nav-row {
  min-block-size: var(--cem-coupling-zone-min);
  display: grid;
  grid-template-columns: 1fr auto;
  align-items: center;
  column-gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}

.cem-proof-nav-actions {
  display: flex;
  gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}
```

Checks:

- The row target and each trailing action are independently operable.
- Trailing action halos do not overlap the row body or neighboring action halos.
- In `compact`, reduced row chrome does not reduce the operable zone.

#### Proof surface C: data table row actions + selection

Use a dense data row with selection and inline row actions. This validates the hardest scan-heavy layout.

```css
.cem-proof-data-row {
  min-block-size: var(--cem-table-row-height);
}

.cem-proof-data-row :is(input[type="checkbox"], button, a) {
  min-inline-size: var(--cem-coupling-zone-min);
  min-block-size: var(--cem-coupling-zone-min);
}

.cem-proof-data-actions {
  display: flex;
  gap: max(var(--cem-gap-related, 0px), var(--cem-coupling-guard-min));
}
```

Checks:

- A compact table row may be visually shorter than `--cem-coupling-zone-min`, but row selection and row actions still
  expose full operable zones.
- Selection controls, row actions, and any row-click target do not overlap in their halo regions.
- Focus/selected/target indicators remain visible over dense grid separators.

---

## 10. Change management (canonical governance)
- Treat coupling tokens as **contract-level**: changes should be versioned (SemVer or equivalent).
- Avoid renaming tokens once canonical; prefer adding new tokens or revising mode values.
- Keep the public axis semantic; keep library-specific numeric scales in adapters.

## 11. Token manifest index

| Source table                  | Section | Description                                                                                                                         |
|-------------------------------|---------|-------------------------------------------------------------------------------------------------------------------------------------|
| `cem-coupling-minimums`       | §4.1    | Hard safety minimums: `--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo` (mode-invariant safety contract) |
| `cem-coupling-halo-overrides` | §5.2    | Forgiving/compact override values for `--cem-coupling-halo` only (generator-only; no new tokens)                                    |

Generator derivation rules:
- `cem-coupling-minimums` → token list (tier in last column).
- `cem-coupling-halo-overrides` → override data only; the token is already declared in the base `:root` block.
- `zone-min` and `guard-min` are never overridden per mode (mode-invariant).
- Visual control geometry (`--cem-control-*`, `--cem-icon-button-*`, row heights) is owned by D2c Controls and is NOT
  declared by this generator. See [`cem-controls.md`](./cem-controls.md) §7.
