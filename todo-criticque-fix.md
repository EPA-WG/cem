# `todo-criticque.md` Adoption Check and Fix Report

## Scope

Checked whether `docs/todo.md` actually addressed the critique in `todo-criticque.md`, using `todo-criticque-followup.md` as the claimed adoption map.

Files reviewed:

- `todo-criticque.md`
- `todo-criticque-followup.md`
- `docs/todo.md`
- Relevant token specs, especially `packages/cem-theme/src/lib/tokens/cem-shape.md` and `cem-voice-fonts-typography.md`

## Result

`docs/todo.md` had adopted the majority of the critique:

- Added transformation principles for manifest-driven generation, stable h6+table extraction, token tiers, verification, breakpoint query limits, shared generator infrastructure, and canonical design ownership.
- Reordered phases so D2 coupling precedes D3 shape and D5 stroke.
- Added D1 spacing modes and corrected D1 layout token names.
- Added D1x split output for custom properties vs literal media/container queries.
- Added D2 invariants, accessibility baseline, and proof-surface requirements.
- Added D5 WCAG/focus and no-layout-shift requirements.
- Added D4 open design blockers for output shape and perceivable channel changes.
- Expanded D6 typography coverage and cross-checks.
- Added cross-phase verification.

However, the follow-up overclaimed a few items and `docs/todo.md` still had remaining defects.

## Fixes Applied to `docs/todo.md`

### 1. Fixed R&D blocking semantics

Problem:

- `docs/todo.md` said every R&D item blocks the whole indicated phase.
- Several listed items were not true phase blockers. Some were implementation-placement questions or optional artifacts.

Fix:

- Updated Principle P7 so only absent canonical definitions block affected output.
- Clarified that ownership questions for already-canonical tokens do not block unrelated required tokens.
- Replaced the R&D table's `Blocks` column with `Impact`.

### 2. Fixed D3 `--cem-bend`

Problem:

- `todo-criticque-followup.md` claimed `--cem-bend` was "spec silent" and put it behind R-D3-1.
- `cem-shape.md` already defines `--cem-bend` as the active required bend alias.

Fix:

- Phase 9 now requires `--cem-bend` in the shape manifest.
- Phase 9 generator output now includes `--cem-bend` with the bend basis tokens.
- Removed obsolete R-D3-1.

### 3. Fixed `--cem-bend-control-round-ends` tier

Problem:

- `docs/todo.md` incorrectly marked `--cem-bend-control-round-ends` as adapter-only.
- `cem-shape.md` defines it as an optional semantic endpoint, not an adapter-only M3 parity alias.

Fix:

- Phase 9 manifest task now marks `--cem-bend-control-round-ends` as an optional semantic endpoint.
- Phase 9 generator task now emits it according to manifest tier when metadata supplies a real value.
- Adapter-only gating now applies only to M3-parity aliases such as `--cem-bend-xs`, `--cem-bend-sm`, etc.

### 4. Fixed `--cem-action-border-radius` treatment

Problem:

- `docs/todo.md` treated `--cem-action-border-radius` as optional pending R-D3-2.
- `cem-shape.md` already lists it as an existing component-binding contract, but generator ownership is still a legitimate placement question.

Fix:

- Phase 9 now records `--cem-action-border-radius` in the manifest as an existing component-binding contract.
- Renamed the open decision to `R-D3-ACTION`.
- The decision now blocks only direct emission of the action binding by `cem-shape.html`, not the required D3 bend tokens.

### 5. Removed unnecessary D1x blockers

Problem:

- The R&D table blocked Phase 7 on confirming breakpoint thresholds even though the local spec already records the canonical thresholds.
- It also blocked Phase 7 on a container wrapper question, even though the CSS generator can document containment requirements without shipping a wrapper.

Fix:

- Removed `R-D1x-1`.
- Replaced `R-D1x-2` with non-blocking `R-D1x-WRAP`, scoped only to any optional wrapper/component deliverable.

### 6. Removed unnecessary D6 blockers

Problem:

- The R&D table blocked Phase 12 on token names and dark/contrast ink projection formulas already present in `cem-voice-fonts-typography.md`.

Fix:

- Removed `R-D6-1` and `R-D6-2`.
- Phase 12 now states that generator work should mirror the current canonical D6 names unless manifest retrofit finds an actual contradiction.

## Files Changed

- `docs/todo.md`
- `todo-criticque-fix.md`

## Residual Open Decisions

Remaining open decisions are now scoped more accurately:

- `R-Schema-1` blocks Phase 4 until the manifest schema is finalized.
- `R-D7-1` blocks closure for highlighted easing.
- `R-D7-2` blocks spring output only.
- `R-D1x-WRAP` does not block Phase 7 CSS output.
- `R-D3-ACTION` does not block required D3 bend tokens.
- `R-D5-1` blocks D5 zebra geometry ownership.
- `R-D4-1` blocks D4 generator output shape.
- `R-D4-2` blocks D4 verification closure.

## Verification

- Checked `docs/todo.md` for stale references to removed blockers:
  - `R-D3-1`
  - `R-D3-2`
  - `R-D1x-1`
  - `R-D1x-2`
  - `R-D6-1`
  - `R-D6-2`
- No stale references remain.

