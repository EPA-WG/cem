# Component CSS Exceptions

**Status:** Active review queue. There are currently no component CSS
exceptions.

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

| ID  | Status | Component/property | Proposed value | Missing semantic category | Analysis and theme-adoption path      |
| --- | ------ | ------------------ | -------------- | ------------------------- | ------------------------------------- |
| —   | None   | —                  | —              | —                         | No exceptions are currently required. |

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
