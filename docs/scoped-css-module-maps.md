# Scoped module maps for typed CSS

Status: proposal awaiting the two decisions below. The active work is tracked
in [todo.md](todo.md); this note does not change runtime contracts.

Template adoption should recognize internal `<style>` content as CSS, dispatch
through the shared `text/css` parser, and retain the resulting CEM AST and source
locations. CSS imports and resource URLs should resolve from the closest owning
module-map scope. The demonstration belongs in
[`scoped-css.html`](../packages/cem-elements/demo/scoped-css.html).

## Existing boundaries

- `cem_ml/src/schema/registry.rs` already registers `text/css` and
  `https://cem.dev/ns/data/css/1`. The remaining work is adoption and retained
  tree integration, not inventing another CSS namespace.
- `cem_ml/src/module_resolution.rs` stores frames outermost first and lets an
  outer mapping win over an inner mapping. Its tests also require outer blocks
  and resource policies to constrain inner contexts. The module URL demo
  explicitly demonstrates a wrapper overriding a component default.
- `cem_ql/src/render.rs` accepts static `import`, `resource`, and `scope`
  module-map entries. A resource already supports `specifier`, `target`,
  `content-type`, and `integrity`; there is no separate override entry kind.
- The [scoped CSS contract](cem-ml-uid-and-scoped-css-design.md) currently
  suppresses every `@import` with `cem.scoped_css.import_unsupported`.
  Resolving an import URL alone would not make its rules component-scoped.

## Decisions before implementation

1. **Recommended override representation:** use an ordinary resource entry
   with the same specifier in the nearest map as an explicit CSS override.
   Select mappings nearest-first for CSS references through the shared resolver;
   preserve existing non-CSS lookup and ancestor resource restrictions. Do not
   introduce a new `override` keyword. For example, a nested map can replace
   `{resource @specifier="component-theme" @target="./default.css"
   @content-type="text/css"}` with
   `{resource @specifier="component-theme" @target="./local.css"
   @content-type="text/css"}`. The same closest-scope lookup applies to image
   and font references inside CSS; their entries retain their actual MIME types.
   This requires approval because CSS lookup would differ from the current
   outer-first mapping contract. An alternative is a dedicated CSS override
   entry, whose syntax and interaction with ordinary mappings must be designed.
2. **Recommended import behavior:** load resolved stylesheets through the shared
   loader, parse into retained CSS CEM trees, and compile imported rules within
   the importing component's managed scope. Preserve import order, conditions
   (`media`, `supports`, `layer`), source locations, and each imported sheet's
   URL base. Diagnose unsupported conditions and cycles rather than emitting
   browser `@import` as a fallback. Existing document-global rule restrictions
   still apply. The narrower alternative is to keep imports suppressed while
   implementing adoption and ordinary CSS resource URL resolution first.

After acceptance, update the normative contracts and add focused native tests
before implementation: typed style adoption, closest-scope mapping/fallback,
ancestor blocks/policies, imported-sheet bases, import conditions/cycles, and
unresolved-reference diagnostics. Then extend the scoped CSS demo and its
source-loaded Storybook and standalone checks. Scope-dependent compiled styles
must retain their resolver context; declaration-only reuse must not cause one
instance's overrides to leak into another.
