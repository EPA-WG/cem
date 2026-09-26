# Scoped module maps for typed CSS

Status: native CSS URL lookup, semantic tree import and static CEM-ML template
and DOM-template style adoption implemented. Scoped import loading and
browser integration remain pending. The active work is tracked
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
- `cem_ql/src/render.rs::extract_static_stylesheets` adopts CSS into a
  `TemplateStylesheetArtifact` retaining both authored text and an immutable
  CSS CEM tree. Clones share the tree. Existing binary artifacts retain their
  source-CSS wire fields and reconstruct the native tree once on reload.

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
diagnostics reject import. Authorized scoped loading is the next step; an imported tree alone does not authorize resource
access. The canonical CEM-ML compile path now retains its styles as native trees.

## DOM-template readiness: accepted and implemented

DOM declarations with static styles now wait for native CSS adoption before
committing their first render. The accepted contract is:

1. Register the produced custom element and perform the existing synchronous
   lifecycle capture normally. Declarations without styles keep their current
   path and incur no CSS processing operation.
2. For declarations with styles, share one pending native adoption operation
   across instances. Send authored CSS and static type/scope metadata as named
   source/control inputs through the shared processing host. Retain native
   owners under its artifact lifecycle, including worker routing and disposal;
   do not move a CSS AST through JSON or add a browser CSS parser.
3. Delay the first DOM render commit until adoption settles. Include the work
   in `whenDeclarationSettled` and each affected instance's render-settlement
   promise. Keep any existing hydrated output intact while waiting.
4. Install successfully adopted styles once through declaration style ownership.
   A malformed style is omitted with a declaration diagnostic. A native engine
   failure diagnoses and installs no unvalidated styles; settlement must still
   complete so the DOM content can render. Styles are never installed as a raw
   text fallback after failed adoption.
5. Use the existing render generation and scope-disposal checks to prevent stale
   work from committing after disconnect, replacement or disposal. Preserve
   current instance state when the deferred render resumes.

The processing host's `css` compile operation carries a named JSON source/control
batch (`css`, optional `scope` and `contentType`) to `adoptDomStylesheets`. Native
CSS trees stay in the template artifact registry and follow its cache/disposal
lifecycle. Source-only results carry installable CSS and diagnostics back to the
host. Scope disposal settles the declaration and render waits even if an
interrupted worker request cannot reply.

Native batch tests and `dom-stylesheet-adoption.stories.ts` cover independent
validation, shared adoption, both settlement APIs, unsupported/dynamic types,
native failure, no-style declarations, reconnect/disposal, and retained hydrated
output. The stories also run with real WASM in a dedicated worker. The scoped CSS
demo extension remains pending scoped import and resource URL resolution.
