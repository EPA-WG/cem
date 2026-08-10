# Component CSS Exceptions

**Status:** Active review queue. One proposed exception has an accepted taxonomy
and field-like channel but still requires a binary channel and theme-token
decision; no component CSS exception is authorized.

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

| ID          | Status                                       | Component/property                                                                                                        | Proposed value                                                                                     | Missing semantic category                                                       | Analysis and theme-adoption path                                                                                                                                                                                                                                                                                 |
| ----------- | -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CEM-CSS-001 | Proposed — field channel accepted, no waiver | Field-like native controls: accepted `border-color` only. Binary native controls: exact hover property remains undecided. | No component-local value. Add generated theme-owned semantic input-hover endpoint(s) after review. | Field-like hover boundary color plus a separate binary-control hover indicator. | Do not reuse action or raw palette tokens. Add the accepted field boundary to D0 after naming/value review. Decide whether binary hover uses an outline below zebra focus priority or custom appearance, then add D0/D5 source tokens, mode/forced-colors mappings, generation coverage, and component bindings. |

### CEM-CSS-001 discovery evidence

The exact native hover owners are `cem-field input`, `cem-text-field input`,
`cem-textarea textarea`, `cem-select select`, `cem-checkbox input`,
`cem-radio input`, and `cem-switch input`. Their surrounding labels and wrappers
may also match `:hover`, but they are not substitutes for the interactive
control owner.

The generated catalog contains ten hover color endpoints, all under
`--cem-action-{intent}-hover-{background,text}`. It contains no generated
`--cem-input-*`, `--cem-field-*`, or form-control hover color endpoint. The
controls specification generates only height and padding, while D5 explicitly
rejects a canonical per-component `--cem-input-outline` thickness token in
favor of generic stroke geometry. None of those tokens defines the missing
input hover color/boundary semantics.

Theme review accepts two semantic families. The field-like family owns
`cem-field`, `cem-text-field`, `cem-textarea`, and `cem-select`; the binary
family owns `cem-checkbox`, `cem-radio`, and `cem-switch`. A field-like control
must not borrow binary selection semantics, and an unchecked binary control
must not acquire selected semantics merely because it is hovered.

The field-like paint channel is accepted as boundary-only. Hover changes the
native control's `border-color` while preserving its comfort fill and text. A
temporary repository-Playwright Chromium spike confirmed that authored boundary
color visibly repaints native text input, textarea, and select controls. With a
`Highlight` forced-colors mapping, the boundary also remained visible in that
mode. This accepts the property channel, not a component-local value or final
token name.

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

The outline result is technically reliable but creates a semantic decision: a
binary hover outline must remain unmistakably subordinate to the D5 zebra focus
ring. The alternative is custom control appearance, which would replace native
rendering and substantially enlarge the component contract. Neither direction
is authorized by this evidence alone.

Until the binary direction and both families' token names/values are accepted
and generated, component CSS and the `input:hover` browser fixture remain
blocked. The accepted taxonomy and field channel grant no verifier waiver.

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
