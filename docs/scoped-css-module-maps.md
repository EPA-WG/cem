# Scoped module maps for typed CSS

Status: accepted design; native CSS URL lookup and semantic tree import implemented. Template adoption,
scoped import loading, and browser integration remain pending. The active work is tracked
in [todo.md](todo.md). Browser styles must not depend on these capabilities
until their implementation and integration checks are complete.

Template adoption should recognize internal `<style>` content as CSS, dispatch
through the shared `text/css` parser, and retain the resulting CEM AST and source
locations. CSS imports and resource URLs should resolve from the closest owning
module-map scope. The demonstration belongs in
[`scoped-css.html`](../packages/cem-elements/demo/scoped-css.html).

## Existing boundaries

- `cem_ml/src/schema/registry.rs` already registers `text/css` and
  `https://cem.dev/ns/data/css/1`. The remaining work is adoption and retained
  tree integration, not inventing another CSS namespace.
- `cem_ml/src/module_resolution.rs` stores frames outermost first. Non-CSS
  requests let an outer mapping win over an inner mapping; the `css` resolution
  purpose selects the closest mapping. Its tests also require outer blocks
  and resource policies to constrain inner contexts. The module URL demo
  explicitly demonstrates a wrapper overriding a component default.
- `cem_ql/src/render.rs` accepts static `import`, `resource`, and `scope`
  module-map entries. A resource already supports `specifier`, `target`,
  `content-type`, and `integrity`; there is no separate override entry kind.
- The [scoped CSS contract](cem-ml-uid-and-scoped-css-design.md) currently
  suppresses every `@import` with `cem.scoped_css.import_unsupported`.
  Resolving an import URL alone would not make its rules component-scoped.

## Accepted decisions

1. **Override representation:** use an ordinary resource entry
   with the same specifier in the nearest map as an explicit CSS override.
   Select mappings nearest-first for CSS references through the shared resolver;
   preserve existing non-CSS lookup and ancestor resource restrictions. Do not
   introduce a new `override` keyword. For example, a nested map can replace
   `{resource @specifier="component-theme" @target="./default.css"
   @content-type="text/css"}` with
   `{resource @specifier="component-theme" @target="./local.css"
   @content-type="text/css"}`. The same closest-scope lookup applies to image
   and font references inside CSS; their entries retain their actual MIME types.
   CSS lookup intentionally differs from the non-CSS outer-first contract.
   Within each frame, retain existing URL-scope and specifier specificity.
   A selected blocking entry in any applicable ancestor frame prevents an inner
   override; the final target must satisfy every ancestor's resource policy.
2. **Import behavior:** load resolved stylesheets through the shared
   loader, parse into retained CSS CEM trees, and compile imported rules within
   the importing component's managed scope. Preserve import order, conditions
   (`media`, `supports`, `layer`), source locations, and each imported sheet's
   URL base. Diagnose unsupported conditions and cycles rather than emitting
   browser `@import` as a fallback. Existing document-global and library-layer restrictions
   still apply. Until this loading path is implemented, imports remain
   suppressed by the current runtime.

Update the normative contracts and add focused native tests
before implementation: typed style adoption, closest-scope mapping/fallback,
ancestor blocks/policies, imported-sheet bases, import conditions/cycles, and
unresolved-reference diagnostics. Then extend the scoped CSS demo and its
source-loaded Storybook and standalone checks. Scope-dependent compiled styles
must retain their resolver context; declaration-only reuse must not cause one
instance's overrides to leak into another.

## Retained-tree boundary: accepted and implemented

The adoption audit identified the following boundary, now implemented in
`cem_ml::import::css`:

- `import.rs::import_content_type` now accepts `text/css`. `project_native` and
  `try_retain_lifecycle` import `LoadedInputAstStream::CssDocument` directly into
  a retained CSS-namespace CEM tree.
- `validation/css.rs::CssDocumentAst` retains token/component-value events,
  source maps and semantic roles. Shared import projects these into semantic
  `stylesheet` / `import` / `rule` / `declaration` nodes and nested component
  values, retaining the native owner as lexical provenance.
- `projection.rs::css_ast_cem_presentation_stream` uses the generic inspection
  vocabulary. Reusing it as the runtime data tree would expose inspection
  records rather than the CSS namespace and semantic elements.
- `cem_ql/src/render.rs::extract_static_stylesheets` currently extracts CSS text
  into `TemplateStylesheetArtifact`; it does not retain a CSS CEM tree.

**Accepted:** use the existing CSS schema as the runtime tree vocabulary.
Extend the native shared import boundary to build those semantic nodes directly
from the parsed CSS events, retaining the native owner, lexical source and source
maps. Resolve URLs by walking this CEM tree. Keep the inspection vocabulary at
the presentation boundary. This adds native CSS structure work before browser
adoption; registering a MIME alias alone cannot complete the feature.

For example, the semantic portion of an adopted style would be shaped as follows
(source metadata and lexical retention are omitted from this illustration):

```cem
{css:style-block @host-kind="custom-element" |
    {css:import @href="component-theme"}
    {css:rule @kind="style" @selector=":scope" |
        {css:declaration @name="background-image" @value="url(icon)" |
            {css:component-value @kind="url" @value="icon"}
        }
    }
}
```

Here `css` denotes `https://cem.dev/ns/data/css/1`. The schema's `style-block`
child model now admits `import`, covered by native fixtures. Standalone stylesheets
retain the schema's `stylesheet` root. The runtime tree must preserve enough
lexical information to distinguish URL tokens and quoted `url(...)` functions
from ordinary strings, comments and custom-property token sequences.

Unknown at-rule bodies remain balanced component values because their grammar
is not known. Quoted `url(...)` is a function containing a string; an unquoted
URL is a URL component value. Consumers do not re-tokenize strings or comments.

Native fixtures cover stylesheet, explicit style-block and declaration-list
inputs, CSS expanded names, semantic structure, source locations, quoted and
unquoted URLs, comments, escapes, empty input, hard recovery errors and bounds.
Import retains unresolved references without access: import/URL policy facts
stay on the native owner and still fail ordinary validation. Other hard
diagnostics reject import. Template adoption and authorized scoped loading are
the next steps; an imported tree alone is not an installable style artifact.
