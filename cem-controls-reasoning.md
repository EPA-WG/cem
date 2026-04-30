# CEM Controls Token Reasoning

This document records the rationale for moving `--cem-control-height` and related visual control geometry tokens into
canonical `cem-controls.md`.

This is a reasoning artifact, not the canonical controls spec itself. It does not move, rename, add, or deprecate any
tokens by itself.

## Current State

- `--cem-control-height`, `--cem-control-padding-x`, and `--cem-control-padding-y` are defined in
  `packages/cem-theme/src/lib/tokens/cem-controls.md`.
- `--cem-control-height` is consumed by `cem-shape.md` as the default height basis for `--cem-bend-round`.
- D1, D2, D2c, and D3 cross-reference the split contracts:
  - D1 treats D2 coupling as a hard constraint for interactive adjacency.
  - D2 defines zone, guard, halo, and operability policy.
  - D2c Controls defines visual control geometry and per-mode visual overrides.
  - D3 uses control height so round-end geometry remains tied to actual control sizing.
- `cem-controls.html` and manifest validation source visual control geometry from `cem-controls.md`.
- `cem-coupling.html` and manifest validation source only coupling safety tokens from `cem-coupling.md`.

## Pros

- Better discoverability for component authors looking for button, input, and generic control sizing.
- Cleaner grouping for `--cem-control-*` tokens and possible future control-family aliases.
- Reduces conceptual load in `cem-coupling.md`, letting D2 focus more tightly on zone, guard, halo, and operability.
- Gives cross-category consumers such as Shape a narrower "controls geometry" contract instead of depending on broad D2
  coupling.
- Creates a natural home for component mapping guidance without crowding D1 spacing, D2 operability, or D3 shape.

## Cons

- Splits operability rules from visual control geometry, even though current coupling modes intentionally tune them
  together.
- Requires generator, validator, manifest index, token index, and cross-reference updates.
- Risks duplicating D1 spacing, D2 coupling, and D3 shape guidance.
- Adds another canonical document to maintain for a relatively small token set.
- Makes namespace boundaries less clear for related tokens such as `--cem-icon-button-*`, `--cem-list-row-height`,
  `--cem-menu-row-height`, and `--cem-table-row-height`.

## Implemented Split

Since `cem-controls.md` is canonical, visual control geometry belongs there and D2 coupling remains focused on
operability safety.

`cem-controls.md` owns:

- `--cem-control-height`
- `--cem-control-padding-x`
- `--cem-control-padding-y`
- `--cem-icon-button-size`
- `--cem-icon-button-icon-size`
- `--cem-list-row-height`, `--cem-menu-row-height`, and `--cem-table-row-height` if the controls doc owns component
  affordance sizing

`cem-coupling.md` owns:

- `--cem-coupling-zone-min`
- `--cem-coupling-guard-min`
- `--cem-coupling-halo`
- coupling mode invariants and operability rules

The clean split would be:

- Controls doc: visual/component geometry contract.
- Coupling doc: operability safety contract.
- Shape doc: consumes `--cem-control-height` from Controls instead of Coupling.
- Dimension doc: still references Coupling for interactive adjacency guard rules.

This split should be explicit and generator-backed: the controls doc owns visual control geometry, while
`cem-coupling.md` continues to own mode-invariant operability rules such as zone, guard, and halo.

## Completed Work

- Created canonical `packages/cem-theme/src/lib/tokens/cem-controls.md`.
- Moved visual control geometry source tables from `cem-coupling.md` to `cem-controls.md`.
- Added `cem-controls.html` generator and separate manifest validation.
- Updated cross-references in Shape, Coupling, Dimension, the token index, and parity docs.
- Kept coupling mode invariants and safety policy in D2 Coupling.

## Expansion Triggers

The controls doc becomes more valuable as the token family grows. Likely additions include:

- multiple size tiers for controls
- field-specific or button-specific geometry tokens
- separate control density rules independent from coupling modes
- adapter mappings that need a stable controls layer between external design systems and CEM D2
