# Component CSS Exceptions

**Status:** No open exceptions. `CEM-CSS-001` was resolved by adopting generated
theme tokens; no component-local value or verifier waiver was authorized.

## Token-first rule

Component CSS MUST consume an existing semantic CEM token whenever one can
express the required value. Raw color, spacing, shape, stroke, typography,
timing, layering, control, or responsive values are not a shortcut around
`@epa-wg/cem-theme` ownership.

If a component requirement cannot be represented by the current token catalog,
stop and warn before adding the CSS. Add a proposed exception to this document
so it can be analyzed, categorized, and either mapped to an existing token or
adopted into the theme. This queue is not an allowlist: recording a proposal
does not authorize component CSS to bypass the style verifier.

## Review queue

None.

## Closed decisions

| ID | Status | Requirement | Resolution |
| --- | --- | --- | --- |
| CEM-CSS-001 | Closed — adopted into theme, no exception | Distinguishable default, hover, disabled, readonly, invalid, pending, focus, and selection indicators across field-like and binary input controls. | D0 owns generated semantic input-indicator colors; D5 owns pending thickness and outline/underline geometry selectors. Component CSS composes one local three-stripe shadow stack exclusively from those endpoints, with system-color forced-colors fallbacks. |

### CEM-CSS-001 discovery evidence

The exact native hover owners are `cem-field input`, `cem-text-field input`,
`cem-textarea textarea`, the `cem-select .cem-select__control`, `cem-checkbox input`,
`cem-radio input`, and `cem-switch input`. Their surrounding labels and wrappers
may also match `:hover`, but they are not substitutes for the interactive
control owner.

At discovery time the generated catalog exposed only action-family hover
colors, while controls supplied geometry only. Reusing action tokens or raw
palette values would have miscategorized input semantics, so implementation
correctly stopped at the exception queue.

Theme review accepts two semantic families. The field-like family owns
`cem-field`, `cem-text-field`, `cem-textarea`, and `cem-select`; the binary
family owns `cem-checkbox`, `cem-radio`, and `cem-switch`. A field-like control
must not borrow binary selection semantics, and an unchecked binary control
must not acquire selected semantics merely because it is hovered.

The accepted CEM direction supersedes the initial field-border-only candidate.
Both families use the same stripe-stack transform: field-like controls default
to its underline geometry, while binary controls default to its whole-label
outline geometry. Either family may select the other appearance. A public
`--cem-input-indicator-appearance` adapter accepts references to the generated
D5 appearance tokens; it does not authorize raw component geometry.

### CEM-CSS-001 binary paint evidence

The temporary spike compared each authored hover result directly with the same
native control's hovered pixels, avoiding false positives from Chromium's own
hover repaint. It covered unchecked and checked checkbox, radio, and
checkbox-backed switch controls in normal and forced-colors modes.

- `accent-color` changed checked controls but added no painted pixels for
  unchecked controls.
- Native `border-color` and `background-color` added no painted pixels in either
  checked state.
- A two-pixel `box-shadow` added 208 pixels around every normal-mode control but
  added none in forced colors.
- A two-pixel outline added 208 pixels around every control in every checked
  state and survived forced colors.

The reliable outline result is now adopted without custom native-control
appearance. The always-present anchor/state stripe uses
`--cem-stroke-boundary`; focus and selection independently add
`--cem-zebra-strip-size`. Invalidity recolors the anchor rather than adding a
fourth geometry role. This keeps hover subordinate to focus and allows invalid,
focus, and selection to coexist.

Normal rendering consumes generated `--cem-input-indicator-*`,
`--cem-stroke-*`, `--cem-zebra-*`, and `--cem-indicator-appearance-*` tokens.
Explicit input `busy` also strengthens the anchor through generated
`--cem-stroke-pending`, making pending distinguishable without hue or layout
shift while invalid and disabled keep higher anchor precedence.
In forced colors, the component removes shadows, maps hover to `Highlight`, and
uses full `CanvasText` pending and focus outlines at their semantic widths. The
focused runtime fixture and the forced-colors Chromium gate make the resolution
executable. Because no raw or component-local styling value was needed,
`CEM-CSS-001` closes as theme adoption—not as an exception.

## Review procedure

1. Search the generated token catalog and the source token specifications for a
   semantic endpoint before proposing a component-local value.
2. If no endpoint fits, stop the component change, warn that a token exception
   is required, and add one `proposed` row with the exact component, property,
   value, missing semantic category, and reason.
3. Review whether the requirement maps to an existing token, reveals a missing
   theme token/category, or is truly component-local and bounded.
4. Prefer adding and adopting a categorized theme token. A rare component-local
   exception requires an explicit accepted contract plus a narrowly scoped
   verifier rule; this document alone never suppresses a gate.
5. Close the row only after the component uses the accepted token or the
   separately approved bounded exception is executable and documented.

The implemented `action:hover` and `action:active` bindings require no
exception. Every default, hover, and active background/text declaration maps
directly to generated `--cem-action-primary-*` or
`--cem-action-contextual-*` semantic tokens, and the style verifier rejects
unknown or non-CEM variables.

The custom select likewise requires no exception. D0 owns the generated
`--cem-select-*` popup/option state colors and D2c owns
`--cem-list-popup-rows`; all remaining geometry composes existing CEM tokens.

The navigation hover contract also requires no exception. D0 owns generated
`--cem-navigation-item-*` default, hover, current, current-hover, and disabled
color pairs. Component CSS binds only those endpoints in normal modes and uses
platform system colors in forced colors. Navigation focus-visible likewise
requires no exception: D5 already owns focus thickness and external offset,
while the zebra focus category owns the mode-aware ring color.

Navigation active also requires no exception. D0 now owns distinct generated
`--cem-navigation-item-active-*` and
`--cem-navigation-item-current-active-*` pairs, normal component CSS binds only
those endpoints, and forced colors use platform system colors.

Navigation disabled requires no exception. D0's existing generated
`--cem-navigation-item-disabled-*` pair represents the paint, D5/zebra already
represent retained ARIA-disabled focus, and forced colors use platform system
colors. Activation suppression is component behavior, not CSS paint.

Content hover also requires no exception. The pre-CSS audit found that action
and custom-select option endpoints could not jointly represent checkable chips
and a native selectable-list composite, so D0 now owns generated
`--cem-content-interaction-*` default, hover, selected, selected-hover, and
disabled pairs. Component CSS binds only those endpoints in normal modes.
Forced colors use platform system colors; the native listbox retains its
platform surface and recolors its existing border without adding geometry.

Content focus-visible requires no exception. D5 already owns external focus
width and offset, and zebra owns the mode-aware focus color. Component CSS
binds those generated endpoints directly to the accepted native owners; forced
colors use `CanvasText` without changing geometry or content-state paint.
