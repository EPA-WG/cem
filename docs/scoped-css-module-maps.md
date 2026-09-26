# Scoped module maps for typed CSS

Status: native CSS URL lookup, retained-tree reference resolution, semantic tree
import, and static CEM-ML/DOM-template style adoption implemented. Scoped import loading and
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

## Retained CSS reference resolution: implemented

`cem_ml::css_resources::resolve_css_resources` walks the retained CSS tree and
resolves `import` nodes, unquoted URL components and quoted `url()` functions
through the shared CSS-purpose resolver. It keeps source order, import
layer/supports/media fields, source maps and byte ranges. The plan owns an `Arc`
to the original tree and stores each resolution or error independently, including
mapping MIME/integrity metadata. CSS strings and comments are never re-tokenized. The import boundary labels
comment components explicitly so recovered invalid tokens cannot be mistaken
for ignorable trivia inside a quoted URL.

Callers supply the owning template URL for inline styles or the final imported
stylesheet URL. The synthetic tree source URI is never a resolution base. The
same retained tree can produce distinct plans under different instance contexts;
plans must not be reused by declaration identity alone. Tests cover nearest-map
overrides, ancestor blocks, CSS-relative fallback, invalid/blocked references,
imported-sheet bases, quoted escapes, nested/custom-property URLs, empty CSS and
non-CSS rejection.

This native plan does not load imports or install browser CSS. The declaration
emitter below can consume it to rewrite explicit external URL components.
String-valued resource grammars such as `image-set()` candidates, local-fragment
semantics, empty URL handling, import condition validation/cycles, shared-loader
integration and per-context style ownership remain downstream work. The current
browser import suppression continues to apply.

## Scoped CSS fragment references: automatic binding deferred

Automatic contextual `url(#local)` handling is deferred to the
[wishlist](wishlist.md#cem-elements-runtime). The immediate approach is explicit
matching IDs in generated template content and an authored CSS custom property
containing the complete URL value, consumed through `var(--filter-url)`.
Declaration styles remain static and shared. No automatic fragment substitution
or ID/reference rewriting is required for this approach.

CSS `@scope` does not make fragment lookup private to a DCE instance. Authors
must supply distinct target IDs when resource content can differ per instance.
An ID string alone cannot be concatenated inside CSS `url()`; the custom
property must carry the entire `url("#unique-target")` value.

The [deferred binding design](scoped-css-fragment-bindings.md) preserves the
proposed future treatment of generated-DOM targets, imported template
provenance, owner identity, native manifests, shared resource slots and
hydration. Explicit external resource URLs continue to belong to the separate
module-map resolution work below.

## Unmapped CSS URLs: accepted and implemented

For CSS only, consult the closest module maps first, then resolve an unmapped
name as a URL relative to its stylesheet/template base. For a sheet at
`https://example.test/css/main.css`, unmapped `url(icons/check.svg)` becomes
`https://example.test/css/icons/check.svg`. The same fallback applies to imports
such as `@import "theme.css"`. A mapped `theme` still uses the closest resource
override. Explicit matching blocks and mapped-target errors never trigger
fallback, and the fallback target must satisfy ancestor scheme policy. Non-CSS
lookup retains strict unmapped bare-specifier errors.

A misspelled unmapped alias such as `theme` becomes a relative resource request
rather than an unresolved-alias diagnostic. Fallback results retain the authored
specifier and stylesheet referrer, use the absolute URL as the normalized
specifier, and carry no invented mapping, content-type or integrity metadata.
The behavior matches ordinary [CSS relative URL semantics](https://www.w3.org/TR/css-values-4/#relative-urls)
after CEM's module-map lookup. Native resolver and retained-tree fixtures verify
both import and resource fallback and unchanged non-CSS behavior.

## Context-dependent stylesheet ownership: accepted

The current [normative CSS ownership contract](cem-ml-uid-and-scoped-css-design.md#2-declaration-owned-css)
requires one managed stylesheet set per effective declaration, rooted at the
produced tag or public shared-scope name. `styleOwnership(compiled)` and
`installDeclarationStylesheets` implement that model. They cannot install two
resolved variants of the same declaration without both variants matching both
instances. The accepted requirement above forbids leaking one instance's module
map overrides into another. Loading imports must not proceed to installation
until ownership and root qualification cover that distinction.

Accepted extension (implementation pending):

1. Keep the immutable native source tree shared. Retain derived stylesheet sets
   per effective declaration and consuming module-map/policy context, including
   the source stylesheet's base URL in the derived artifact key.
2. Let the runtime assign produced hosts an internal
   `data-cem-css-context="<opaque-context-id>"` marker. Hosts in the same effective
   resolution context reuse the marker; it is not a unique instance identifier.
   It is reserved runtime state, not an author-facing styling hook.
3. Qualify a derived private root as
   `@scope (cem-card[data-cem-css-context="ctx"])` and a derived shared root as
   `@scope ([scope="controls"][data-cem-css-context="ctx"]:has(> template[data-cem-island="instance"]))`.
   Preserve the existing lower bounds and authored selector specificity. Do not
   use `data-cem-render-scope`, reintroduce `data-cem-instance-scope`, or copy
   declaration styles into every instance.
4. For shared rules, retain the registered shared style sources and derive the
   applicable set for each consuming context. A host with the same public scope
   but another context receives its own resolved variant; it must not lose
   shared membership merely because its context differs. Styles without resource
   references keep the existing single declaration-owned set.
5. Own the derived sets centrally, reuse them among matching connected hosts,
   and detach/release them when their context has no live consumers or is
   disposed. Context changes invalidate pending work before changing the marker
   or committing styles. Extend render/declaration readiness and hydration
   checks to cover the derived set's lifecycle.

This extends the normative one-set ownership rule for context-dependent CSS and
adds an internal CSS root qualifier. Implementation must retain the existing
single set for context-independent styles. Native import-closure and browser
ownership fixtures precede enabling imports in the scoped CSS demo.

## Native import closure: implemented

`cem_ml::css_imports::CssImportClosure` owns a root reference plan and accepts
imported retained CSS trees through one outstanding request at a time. Requests
carry native resolution metadata (including MIME hints and integrity), the import
node/source map and layer/supports/media conditions. They are loader control
records, not AST exports. A host must validate the response through shared-loader
transport policy and the shared CSS importer before delivering a tree.

The state machine walks imports depth-first in source order, preserving repeated
occurrences and anonymous-layer distinctions while allowing the loader to reuse
the same immutable tree. Each imported plan resolves its resources against the
final response URL. Requested and final URLs participate in ancestor-cycle
checks, ignoring fragments. Final URLs are rechecked against the active resolver
policy and cannot silently be replaced by another mapped URL. Default bounds are
64 sheet occurrences including the root and 16 import levels. Cancellation and
loader/validation failures make the closure terminal; stale/duplicate delivery
IDs are rejected without consuming a different outstanding request.

`Ready` describes a complete import graph only. It does not authorize installation.
The shared CSS import boundary validates condition structure as described below;
feature evaluation remains with the browser. Native byte delivery enforces
MIME/integrity/size limits. Resource rewriting, scoped emission and browser/worker
loader and ownership integration remain pending. Resource URL references are resolved but not fetched by this
state machine. No browser `@import` fallback is introduced.

## Import placement: implemented natively

The closure validates each retained sheet before queueing its imports. Imports
must be top-level and precede other rules; comments, charset nodes and layer-order
statements may appear in the import prefix. A layer statement after another rule
does not reopen that prefix. The importer retains `has-block` on semantic at-rule
wrappers so even an empty layer block ends the prefix without reparsing source.
This follows the [CSS cascade import placement rules](https://www.w3.org/TR/css-cascade-5/#at-import).
Misplaced imports produce `cem.css.import_placement_invalid`. A delivered sheet
with misplaced imports fails the closure before attaching the sheet or queueing
its dependencies. Unknown/unsupported rules conservatively end the prefix;
Condition structure is checked at the shared CSS import boundary.

## Import layer clauses: implemented natively

The shared CSS import boundary validates named import layers against the
[CSS layer-name grammar](https://drafts.csswg.org/css-cascade-5/#layer-names):
a nonempty dotted identifier path with no internal whitespace or CSS-wide
keywords. Comments and identifier escapes retain their token semantics;
`default` is a valid layer name. Bare `layer` remains anonymous, while `layer()`
is rejected rather than silently treated as anonymous.

Each layer clause retains a `css:import-layer` child with an `anonymous` boolean
and ordered `css:layer-segment` children containing decoded `value` attributes.
Segments retain source ranges and maps. This distinguishes an escaped dot within
one name from the separator between nested layers without downstream reparsing.
The existing `layer` attribute retains authored text for control metadata.
Absence of a layer clause remains distinct from an anonymous clause, whose
identity belongs to its import occurrence rather than its reusable source tree.
Malformed downloaded clauses fail byte delivery with `cem.css.import_parse_failed`
before the sheet or its dependencies enter the closure. Condition emission and
the complete scoped compiler remain pending.

## Import supports clauses: implemented natively

The CSS import boundary checks the structural
[supports grammar](https://www.w3.org/TR/css-conditional-3/#at-supports): a bare
import declaration, a single condition, negation, or a uniform `and`/`or` chain.
A bare declaration can have an empty value and final `!important`, but cannot
contain a top-level semicolon or another exclamation delimiter. Recovered bad
tokens are rejected. Invalid clauses fail import before a downloaded sheet
attaches or queues dependencies.

A `css:import-supports` child records `condition-form="declaration"` or
`condition-form="condition"` and retains typed component values, authored tokens
and source ranges. The emitter can use the form to parenthesize a bare declaration
without reparsing CSS. The existing `supports` attribute remains control metadata.
Balanced parenthesized and functional operands preserve the grammar's
future-compatible general-enclosed syntax, including unknown feature queries.
Validation does not determine browser feature support, simplify conditions, or
interpret property/selector feature semantics. Their original structure must
survive emission for the browser to evaluate. Scoped emission remains pending.

## Import media lists: implemented natively

The CSS import boundary splits media lists at top-level commas and checks each
entry against the outer media-type/boolean grammar. Nested commas remain inside
their component blocks. Media types with optional `not`/`only`, condition-only
queries, and the restricted condition after a media type are distinguished.
Unknown types, features and future-compatible enclosed syntax remain unevaluated;
this is structural validation, not a browser capability or viewport check.

A `css:import-media` child contains ordered `css:media-query` children with
`syntax-valid` boolean attributes and retained component values. Each preserves
its authored tokens and source range, including zero-width ranges for empty
entries. An omitted/empty entire media list has no `import-media` child and is
unconditional. Invalid entries retain their source for diagnostics; the emitter
**must replace each `syntax-valid="false"` entry with `not all`**, preserving valid
siblings and list order, following the
[media-query recovery rules](https://www.w3.org/TR/mediaqueries-5/#error-handling).
The raw import `media` attribute is control metadata, not normalized output.
Existing hard stylesheet parse errors and import bounds still reject the source.
The closure retains imported sheets without evaluating these conditions; native
scoped emission and browser installation remain pending.

## Import condition emission: implemented natively

`css_emission::emit_css_import_conditions` consumes the retained import node and
emits ordered supports/media wrapper openings, each with source provenance. Bare
supports declarations gain the parentheses needed by `@supports`; complete
conditions keep their structure. Invalid media entries become `not all` with
`cem.scoped_css.media_query_recovered` diagnostics at their source ranges. Output
uses retained component tokens, trimming only separate whitespace components so
escaped whitespace within an identifier cannot be lost. It never reparses the
raw `supports` or `media` metadata attributes.

Under the existing managed-CSS policy, either a named or anonymous layer clause
returns suppression with `cem.scoped_css.layer_unsupported`; the caller must omit
the complete import occurrence, including its body. Missing, duplicate or
inconsistent typed condition structure returns
`cem.scoped_css.condition_tree_invalid` rather than a raw-text fallback.

This helper expects trees produced by the shared CSS import boundary. It emits
conditional fragments, not installable stylesheets. The caller must supply the
compiled body, preserve closure order, and close each emitted wrapper. Full
native selector/host rewriting, specificity enforcement, keyframe handling,
resource-URL rewriting, managed scope wrapping and browser integration remain
pending. The retained selector profile below supplies the initial structure for
those transformations; unsupported forms must not fall back to raw parsing.

## Retained stylesheet selectors: initial profile implemented

The CSS importer projects a rule's existing native token events into a
`css:selector-list`, ordered `css:selector` and `css:compound-selector` nodes,
typed simple selectors and combinators. No selector text is retokenized by this
path. Decoded names/values, attribute operators/modifiers, nested selector lists,
relative combinators and source ranges remain in the retained tree. Authored
rule selector text and the native lexical owner remain available as provenance.

The initial stylesheet profile supports type/universal, class, ID and attribute
selectors, terminal nonfunctional pseudo-elements, ordinary simple pseudo-classes,
`:is`, `:where`, `:not`, relative `:has`, and functional `:host` with a single
compound argument. It records
specificity as `a-b-c`, including the pseudo-class plus argument weight for
[functional host selectors](https://drafts.csswg.org/css-scoping/#host-selector).
Comments do not create descendant combinators; repeated decoded selectors remain
separate within a compound for later manufactured-specificity checks.

`analysis-status="complete"` marks structure available to the compiler, not a
browser feature-support assertion. The profile leaves simple pseudo-class
matching to the browser. Forms outside the profile, including functional/chained
pseudo-elements,
`nth-*` functions, CSS nesting `&`, and unbound namespace prefixes, produce an
empty list marked `analysis-status="unsupported"`, with no partial specificity
claim. Import still retains the authored rule; unsupported analysis does not
make otherwise retained stylesheet adoption fail. The compiler must diagnose
unsupported structure rather than parse raw selector text downstream, and
coverage must expand before browser cutover.

Nonfunctional pseudo-elements such as `::before` and `::marker` are retained as
`simple-selector @kind="pseudo-element"`, with decoded lowercase names, original
source ranges and type-level specificity. The
[legacy single-colon forms](https://www.w3.org/TR/selectors-4/#pseudo-elements)
`:before`, `:after`, `:first-line` and `:first-letter` normalize to the same typed
shape and emit with two colons. Stylesheet pseudo-class names are likewise
ASCII-normalized, so uppercase host aliases and zero-weight `:WHERE` preserve
their semantics. This does not change the selector-query parser profile.

This extension accepts only a terminal pseudo-element in the outer selector
list. Functional forms, chains, trailing selectors/states, and pseudo-elements
inside selector-list functions remain outside the retained profile. Such a
list is explicitly unsupported rather than partially emitted. The terminal
restriction is this implementation's coverage limit, not a claim that all other
forms are invalid CSS. Broader coverage remains required before browser cutover.

This is an isolated stylesheet profile of the existing native selector parser.
The selector-query entry point retains its capability restrictions: `:host`,
`:host(...)`, and host-state queries remain unsupported without query host
capabilities. Declaration/shared and instance selector emission are implemented
below; full scoped rule emission remains pending.

## Declaration/shared selector fragments: implemented natively

`cem_ml::css_emission::emit_css_declaration_selectors` consumes a retained style
rule and returns accepted selector fragments plus source-aware diagnostics.
It traverses only typed CEM selector nodes; the raw rule selector is never a
fallback. Missing structure is an error, and an unsupported analysis profile
suppresses the list with `cem.scoped_css.selector_unsupported`.

The helper enforces the authored `0-2-1` ceiling before rewriting, rejects IDs
(including IDs inside `:where`), and rejects repeated decoded class/attribute
tokens within a specificity-bearing compound. Attribute identity includes its
namespace, name, operator, value and modifier, so equivalent quoting/escape
spellings cannot bypass the check. Compounds under `:where` have zero weight;
repetition there is not manufactured specificity. A failure inside a functional
selector suppresses its entire top-level selector rather than dropping a nested
branch and changing matching semantics. Other top-level selectors survive.

`:host` becomes `:where(:scope)` and functional host arguments remain attached.
When an argument contains a type/universal selector, the helper uses
`:where(:scope):is(argument)` to preserve a valid compound and the argument's
weight. Simple `:root` and `:global` become diagnosed host aliases. Functional
`:global(...)` remains outside the initial retained profile and is suppressed
until import support is added. Ordinary selector-list functions and relative
combinators emit recursively. Decoded identifiers and attribute values are
escaped when serialized; any-namespace type selectors emit an explicit `*|`.

These are declaration/shared fragments, not installable CSS. Remaining selector
grammar, complete rule bodies, keyframes, resource rewriting, scope wrappers and
complete closure emission remain pending. Browser installation continues to use
the existing path until that compiler is ready.

## Instance selector fragments: implemented natively

`cem_ml::css_emission::emit_css_instance_selectors` uses the same retained tree
and policy traversal. It rewrites `:host` and diagnosed host aliases to `:scope`,
keeps functional host arguments attached, and prefixes other top-level selectors
with `:scope `. Already-leading `:scope` is not doubled. Type/universal host
arguments use `:scope:is(argument)` to retain valid compound syntax.

Prefix selection inspects the first simple selector in the first compound,
preserving the existing runtime behavior. A host nested in a function does not
exempt the outer selector: `:is(:host, .item)` emits
`:scope :is(:scope, .item)`. Consequently its host branch cannot match the scope
root; `:where(:host)` likewise emits a selector that matches nothing. Authors
must use a leading host selector or separate top-level list entry to target the
root. This migration does not distribute selector functions or change that
matching contract. Relative selectors inside `:has` receive no extra prefix.

IDs and duplicate specificity-bearing class/attribute tokens remain forbidden,
and unsupported retained profiles remain suppressed. The authored `0-2-1`
ceiling applies only to declaration/shared CSS, so instance selectors can exceed
it. Returned specificity and source ranges describe the authored selector,
before the generated prefix. Scope wrappers, lower limits, declarations and
instance ownership still need full compiler/browser integration; these fragments
alone do not authorize installation.

## Direct declaration fragments: implemented natively

`cem_ml::css_emission::emit_css_rule_declarations` collects a retained style
rule's direct declarations in order. It applies the existing managed declaration
policy to both declaration and instance CSS: typed `important="true"` suppresses
the declaration with `cem.scoped_css.important_unsupported`. Comments, escaped
keyword spellings and case are already handled at import. The emitter does not
search value text for `!important`; literal strings and nested custom-property
values retain their meaning.

Property names serialize as escaped identifiers, and values serialize from
retained component tokens, including nested function/block tokens. The raw
`value` attribute is never a fallback. Empty custom-property values are retained;
empty ordinary values and recovered unknown components are suppressed with
`cem.scoped_css.declaration_value_unsupported`. Recovery checks include nested
components. Missing typed value structure produces
`cem.scoped_css.declaration_tree_invalid`. Ordinary property/value support remains
browser-owned; this helper is not a property grammar validator.

Each accepted declaration retains its node ID and source/range. Nested rules,
at-rules and imports are returned as explicit `deferred_children`. The complete
compiler must traverse the retained rule's original child order rather than
concatenate declarations across a deferred construct. Comments between
declarations need no emitted fragment; comments within values are preserved.

The unresolved helper retains authored resource URLs. Use the resource-aware
variant below when emitting from a resolution plan. Keyframe-reference rewriting,
nested construct policy and full scope assembly are still required before
installation; neither helper turns a ready import closure into an installable
stylesheet.

## External declaration resource emission: implemented natively

`cem_ml::css_emission::emit_css_rule_declarations_with_resources` takes a
`CssResourcePlan` and a style-rule node ID. The plan owns its retained tree and
context-specific resolutions. The helper replaces explicit external URL nodes
using import-owned byte ranges within retained component tokens, including URLs
inside nested functions, blocks and custom-property values. It does not reparse
CSS, inspect parser ASTs, or mutate the retained tree. The same tree can therefore
emit different results under different module-map contexts.

Quoted and unquoted URLs emit as `url("resolved URL")`, with quotes, backslashes
and control characters escaped as CSS string content. Non-resource token text,
ordinary strings, surrounding comments and escaped token boundaries remain
unchanged. Fragment-only references retain their authored spelling, including
escapes and trivia; this is not contextual local-ID binding or substitution.

An external resolution error suppresses its entire containing declaration with
`cem.scoped_css.resource_resolution_failed` at the original URL source range.
Valid sibling declarations remain in order. A missing, duplicate or inconsistent
plan entry fails with `cem.scoped_css.resource_plan_invalid`, rather than emitting
an unresolved URL. Existing important/recovered-value suppression and deferred
nested-rule boundaries still apply. The retained plan remains the source of
mapping metadata, policy stamps and detailed resolver errors.

This covers explicit `url()` components, not string-valued resource grammars
such as `image-set("image.png" 1x)`. Whole-stylesheet assembly, nested-rule compilation, keyframe references and
browser integration remain pending. The ordered rule and wrapper helpers below
provide the next native building blocks.

## Ordered style rules and scope wrappers: implemented natively

`cem_ml::css_emission::emit_css_style_rule` joins the retained selector and
resource-aware declaration emitters. `CssRuleMode::Declaration` applies the
library specificity ceiling and host rewriting; `CssRuleMode::Instance` applies
the existing instance prefix behavior. Valid selector siblings survive policy
suppression, and diagnostics retain their original source maps and ranges.
A rule with no accepted selector or no remaining body is omitted.

The body is an ordered sequence of `CssRuleBodyItem::Declaration` and
`CssRuleBodyItem::Deferred` entries. Deferred entries retain the original node ID,
source and range for each nested rule, at-rule or import. A declaration after a
nested construct remains after that construct; the assembler never concatenates
all declarations ahead of it. A body consisting only of deferred constructs is
retained for subsequent compilation. These are structured fragments, not a
serialized or installable stylesheet. Nested selector context, grouping-rule
conditions and keyframe references still need native compilation.

`emit_css_scope_wrapper` emits the existing managed boundaries from semantic
`CssManagedScope` inputs:

- `Private` uses the produced tag as its root.
- `Shared` uses the public scope attribute qualified by a direct instance data
  island, avoiding unrelated HTML elements that also carry `scope`.
- `Instance` omits an explicit root and uses the style element's parent.

Private and shared roots optionally include the accepted `data-cem-css-context`
qualification. All variants retain the descendant-DCE and projected-descendant
lower limits. The helper adds no instance UUID selector and never uses
`data-cem-render-scope` as a CSS hook. Names are encoded as identifiers or CSS
strings, not interpolated as raw selectors; empty names/context IDs fail with
`cem.scoped_css.scope_invalid`. Tag registration, public scope-name validation,
context assignment and lifecycle remain the owning runtime's responsibility.
The wrapper helper does not install CSS or add a context attribute to hosts.

## Media/supports grouping fragments: implemented natively

Shared CSS import now retains `group-media` and `group-supports` nodes separately
from an at-rule's body. Media groups use the same typed `media-query` entries as
import conditions; supports groups retain component tokens and a `syntax-valid`
flag. The raw `prelude` attribute remains metadata, not an emission fallback.

`emit_css_grouping_rule` emits these two grouping kinds from the retained
resource plan. It preserves valid query spelling and browser-owned feature
evaluation. Invalid media-list entries emit as `not all` with source diagnostics,
while valid siblings remain. An empty/comment-only list emits `all`, preserving
[empty media-list semantics](https://www.w3.org/TR/mediaqueries-4/).
An invalid supports condition suppresses the whole group with
`cem.scoped_css.supports_condition_invalid`. Unlike import `supports(...)`, a
bare declaration is not a valid grouping condition; see
[CSS Conditional Rules](https://www.w3.org/TR/css-conditional-3/).

The ordered body reuses resolved declaration emission and explicit deferred
child positions. `CssGroupingContext::StyleRule` permits declarations whose
selector comes from the enclosing style rule. `Stylesheet` diagnoses and omits
such declarations with `cem.scoped_css.group_declaration_unsupported`, preserving
valid child rules. The full compiler must propagate that context through nested
groups and must not move declarations across a child rule or invent a selector
for them. Empty bodies are omitted. Rules without a block diagnose as
`cem.scoped_css.group_block_required`; inconsistent or missing group structure
fails with `cem.scoped_css.group_tree_invalid` and no raw-prelude fallback.

This API returns grouping fragments with original source maps and byte ranges,
not a complete stylesheet. Deferred selector nesting, other grouping kinds,
keyframe names/references and recursive whole-stylesheet composition remain
pending before browser cutover.

## Shared-resolver byte delivery: implemented natively

`CssImportClosure::load_imports` uses `ResolverRegistry` input reads with the
closure's abort signal. `complete_response` is the corresponding byte-delivery
entry point for asynchronous host transports. Both paths send response bytes to
`import_data_bytes` only after final-URL policy/cycle checks, MIME checks, response
and aggregate byte bounds, and any declared integrity expectation. A non-CSS
mapping hint rejects the synchronous request before reading. An explicit
non-CSS response type fails even when its mapping says CSS. With no response
MIME, the mapping hint or the explicit CSS import type is used; content is never
sniffed. CSS entry modes other than a full stylesheet are rejected.

Default response bounds are the shared import limit (16 MiB per response) and
64 MiB of imported bytes per closure, excluding its already-imported root.
Repeated deliveries count separately. These checks occur before parsing buffered
responses; transports remain responsible for bounded streaming allocation,
HTTP status/access policy, and intermediate redirect handling. A final URL
recheck does not replace transport authorization.

The shared `resource_integrity::verify_resource_integrity` helper supports
space-separated `sha256`, `sha384` and `sha512` base64 digests. It selects the
strongest algorithm and accepts a match among that algorithm's supplied digests,
following [SRI's strongest-metadata rule](https://www.w3.org/TR/sri/#get-the-strongest-metadata-from-set).
This is a strict supported profile: malformed or empty expectations, unsupported
algorithms, options, and nonstandard encodings diagnose instead of silently
removing the integrity requirement. Standard padded and unpadded base64 are
accepted. Byte verification does not authorize cross-origin access.

Fixtures cover shared-resolver request order, final response bases, successful
integrity verification and tampering, stronger-algorithm mismatch, malformed
metadata, MIME rejection, byte budgets, malformed CSS, loader failures and
cancellation before imported trees are attached. Browser transport wiring and
scoped CSS installation remain pending.
