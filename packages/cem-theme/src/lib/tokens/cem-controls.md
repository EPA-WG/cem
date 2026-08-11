# Visual Control Geometry Tokens (CEM) — Canonical Spec

**Status:** Canonical (v1.0)

**Last updated:** August 10, 2026

**Audience:** Design Systems, Product Design, Front-End Engineering

**Applies to:** Visual sizing of generic controls and component-affordance geometry (button, input, icon button, list/menu/table rows, progress and slider graphics)

**Companion specs:**
- **D1. Space & Rhythm** ([`cem-dimension.md`](./cem-dimension.md)) — gaps, insets, layout rhythm
- **D2. Coupling & Compactness** ([`cem-coupling.md`](./cem-coupling.md)) — operability safety contract (zone/guard/halo); coupling modes drive Controls geometry overrides
- **D3. Shape — Bend** ([`cem-shape.md`](./cem-shape.md)) — consumes `--cem-control-height` for round-end bend geometry
- **D5. Stroke & Separation** ([`cem-stroke.md`](./cem-stroke.md)) — focus indicator placement on controls
- **D6. Typography & Voice** ([`cem-voice-fonts-typography.md`](./cem-voice-fonts-typography.md)) — text inside controls

---

## 1. Why split Controls from Coupling

Visual control geometry (height, padding, icon-button size, row heights) and operability safety (zone/guard/halo) historically lived together in D2 Coupling. Splitting them clarifies responsibilities:

- **D2 Coupling** owns the **safety contract**: `--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo`, and the mode-invariant rules that prevent intent mis-coupling regardless of input modality.
- **D2c Controls** (this spec) owns **visual/component geometry**: how big a generic control or row looks, what padding it carries, and how those visuals scale across coupling modes.

The two contracts are coupled by policy: visual geometry MUST NOT shrink the operable zone below `--cem-coupling-zone-min`, and adjacency MUST respect `--cem-coupling-guard-min` regardless of how Controls geometry shrinks under `compact`.

See [`cem-controls-reasoning.md`](../../../../../cem-controls-reasoning.md) for the rationale behind the split.

---

## 2. Taxonomy placement

- **D2c. Controls** (this canvas)
  - generic control geometry: height, inline padding, block padding
  - icon button geometry: container size, glyph size
  - component affordance row heights: list, menu, data-table
  - component graphics: circular progress diameter/track and slider track/thumb
  - per-coupling-mode visual overrides

- **D2. Coupling & Compactness** (from [`cem-coupling.md`](./cem-coupling.md))
  - zone-min, guard-min, halo, mode-invariant operability rules

- **D1. Space & Rhythm** (from [`cem-dimension.md`](./cem-dimension.md))
  - layout gaps and insets between controls; never violates D2 guard

Boundary heuristic:

- If it changes **the visible size of a single control or row** → D2c (this spec)
- If it changes **the operable safety contract or adjacency** → D2
- If it changes **distance between items** → D1

---

## 3. Canonical control geometry tokens

### 3.1 Baseline (balanced mode)

```css
:root {
  /* Generic control geometry */
  --cem-control-height: 2.5rem;
  --cem-control-padding-x: 0.75rem;
  --cem-control-padding-y: 0.5rem;

  /* Icon buttons: typically make the visible container meet the operable zone */
  --cem-icon-button-size: var(--cem-coupling-zone-min);
  --cem-icon-button-icon-size: 1.25rem;

  /* Lists / menus */
  --cem-list-row-height: 3rem;
  --cem-list-popup-rows: 8;
  --cem-menu-row-height: 3rem;

  /* Data tables */
  --cem-table-row-height: 2.5rem;
}
```

###### cem-controls-geometry
| Token                         | Value                          | Description                                                       | tier        |
|-------------------------------|--------------------------------|-------------------------------------------------------------------|-------------|
| `--cem-control-height`        | `2.5rem`                       | Generic control height                                            | recommended |
| `--cem-control-padding-x`     | `0.75rem`                      | Generic control inline padding                                    | recommended |
| `--cem-control-padding-y`     | `0.5rem`                       | Generic control block padding                                     | recommended |
| `--cem-icon-button-size`      | `var(--cem-coupling-zone-min)` | Icon button visible container size; meets zone minimum by default | recommended |
| `--cem-icon-button-icon-size` | `1.25rem`                      | Icon glyph size within an icon button                             | recommended |
| `--cem-list-row-height`       | `3rem`                         | List row height                                                   | recommended |
| `--cem-list-popup-rows`       | `8`                            | Default maximum visible rows in a transient list popup            | recommended |
| `--cem-menu-row-height`       | `3rem`                         | Menu row height                                                   | recommended |
| `--cem-table-row-height`      | `2.5rem`                       | Data table row height                                             | recommended |

### 3.2 Progress graphic geometry

Circular progress is non-interactive, so its visual size is not an operable-zone
claim and does not grow or shrink with coupling mode. D2c still owns the
single-component geometry: the outer diameter and the visible track thickness.
D0 owns track/indicator color, and D7 owns any indeterminate cycle.

###### cem-progress-geometry
| Token | Value | Description | tier |
|---|---|---|---|
| `--cem-progress-spinner-size` | `3rem` | Default circular progress diameter | required |
| `--cem-progress-track-thickness` | `0.25rem` | Circular or linear progress track thickness | required |

### 3.3 Slider graphic geometry

Slider track thickness and visible thumb diameter are single-component visual
geometry. D2c owns those sizes; D2 continues to own the larger operable target
through `--cem-coupling-zone-min`, and D5 owns focus-outline geometry. A slider
implementation MUST keep its native thumb hit target at least zone-min even
when the visible thumb is smaller.

###### cem-slider-geometry
| Token | Value | Description | tier |
|---|---|---|---|
| `--cem-slider-track-thickness` | `0.25rem` | Visible remaining and active value-range track thickness | required |
| `--cem-slider-thumb-size` | `1.25rem` | Visible circular slider thumb diameter inside the D2 operable target | required |

### 3.4 Coupling-mode overrides (visual geometry only)

Controls geometry varies by coupling mode. The mode selector is owned by D2 Coupling
(`data-cem-coupling="forgiving|balanced|compact"`); Controls only emits the visual override values for those modes.
The safety contract (zone-min, guard-min) does NOT change.

```css
:root[data-cem-coupling="forgiving"] {
  --cem-control-height: 2.75rem;
  --cem-control-padding-x: 1rem;
  --cem-control-padding-y: 0.625rem;

  --cem-list-row-height: 3.25rem;
  --cem-menu-row-height: 3.25rem;
}

:root[data-cem-coupling="compact"] {
  --cem-control-height: 2.25rem;
  --cem-control-padding-x: 0.625rem;
  --cem-control-padding-y: 0.375rem;

  --cem-list-row-height: 2.75rem;
  --cem-menu-row-height: 2.75rem;
}
```

###### cem-controls-geometry-overrides
| Token                     | forgiving  | compact    |
|---------------------------|------------|------------|
| `--cem-control-height`    | `2.75rem`  | `2.25rem`  |
| `--cem-control-padding-x` | `1rem`     | `0.625rem` |
| `--cem-control-padding-y` | `0.625rem` | `0.375rem` |
| `--cem-list-row-height`   | `3.25rem`  | `2.75rem`  |
| `--cem-menu-row-height`   | `3.25rem`  | `2.75rem`  |

Governance rule: visual geometry shrinks under `compact` only as far as the operable contract allows. When visual size
drops below `--cem-coupling-zone-min`, components MUST use halo expansion (see D2 §6.1) so the operable zone still
meets the safety contract.

---

## 4. Cross-spec consumers

- **D3 Shape** consumes `--cem-control-height` as the default height basis for round-end bend:
  `--cem-bend-round: calc(var(--cem-shape-height, var(--cem-control-height)) / 2)`. This is why round-end geometry
  follows control sizing across coupling modes.
- **D2 Coupling** references Controls when documenting how visuals interact with halo policy. D2 itself does not
  declare `--cem-control-*` tokens.
- **D1 Dimension** references D2 Coupling (not Controls) for adjacency guard rules. Controls geometry is separate from
  layout spacing.

---

## 5. Implementation patterns

### 5.1 Generic control sizing

```css
.cem-control {
  block-size: var(--cem-control-height);
  padding-inline: var(--cem-control-padding-x);
  padding-block: var(--cem-control-padding-y);

  /* Safety: do not let visual height collapse the operable zone */
  min-block-size: var(--cem-coupling-zone-min);
}
```

### 5.2 Icon buttons

```css
.cem-icon-button {
  inline-size: var(--cem-icon-button-size);
  block-size: var(--cem-icon-button-size);
}

.cem-icon-button > svg {
  inline-size: var(--cem-icon-button-icon-size);
  block-size: var(--cem-icon-button-icon-size);
}
```

If a product wants visible icon containers below `--cem-coupling-zone-min`, wrap them with halo expansion (see
[`cem-coupling.md`](./cem-coupling.md) §6.1) so operability is preserved.

### 5.3 Rows

```css
.cem-list-row  { min-block-size: var(--cem-list-row-height); }
.cem-list-popup { max-block-size: calc(var(--cem-list-row-height) * var(--cem-list-popup-rows)); }
.cem-menu-row  { min-block-size: var(--cem-menu-row-height); }
.cem-table-row { block-size:     var(--cem-table-row-height); }
```

For data tables where row visuals fall below the safety zone, ensure interactive cells (selection, action buttons)
meet `--cem-coupling-zone-min` independently.

---

## 6. Acceptance criteria

A draft becomes "canonical" when all of the following are true:

1. **Visual-only contract:** this spec emits only `--cem-control-*`, `--cem-icon-button-*`, list/menu/table geometry,
   `--cem-progress-*` graphics, and `--cem-slider-*` visual geometry tokens.
   Safety tokens (`--cem-coupling-*`) are NOT emitted here.
2. **Mode overrides only adjust visuals:** forgiving/compact mode tables touch geometry, never zone/guard.
3. **Halo escape hatch documented:** any time visuals fall below `--cem-coupling-zone-min`, halo expansion is the
   stated remedy.
4. **D3 Shape dependency satisfied:** `--cem-control-height` is declared here so Shape's round-end formula resolves.

---

## 7. Token manifest index

| Source table | Section | Description |
|---|---|---|
| `cem-controls-geometry` | §3.1 | Baseline visual control geometry: height, padding, icon-button, list/menu/table row sizes |
| `cem-progress-geometry` | §3.2 | Circular-progress diameter and shared progress-track thickness |
| `cem-slider-geometry` | §3.3 | Slider track thickness and visible thumb diameter |
| `cem-controls-geometry-overrides` | §3.4 | Forgiving/compact override values for visual geometry (generator-only; no new tokens) |

Generator derivation rules:
- `cem-controls-geometry` → token list (tier in last column).
- `cem-progress-geometry` → token list (tier in last column).
- `cem-slider-geometry` → token list (tier in last column).
- `cem-controls-geometry-overrides` → override data only; tokens are already declared in the base `:root` block.
- Coupling minimums (`--cem-coupling-zone-min`, `--cem-coupling-guard-min`, `--cem-coupling-halo`) are NOT declared here; they belong to D2 Coupling.
