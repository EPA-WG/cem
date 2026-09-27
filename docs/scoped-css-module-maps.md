# Scoped module maps for typed CSS

Status: native typed CSS adoption, scoped import loading, resource resolution and
browser installation are implemented through `retainedStylesheets: {}`. Both
runtime test lanes pass. The authored module-map import demonstration is verified in standalone and
source-loaded modes. Default runtime cutover remains pending; see [todo.md](todo.md).

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
- The [scoped CSS contract](cem-ml-uid-and-scoped-css-design.md) suppresses
  `@import` in the default runtime. The opt-in retained runtime loads and scopes
  imported rules; resolving their URLs alone is insufficient.

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
   still apply. The opt-in retained runtime implements this loading path;
   the default runtime continues to suppress imports.

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
Import-classified `image-set()` strings now use the same plan (see below).
Automatic local-fragment binding remains deferred; native import condition,
cycle and byte-delivery checks are implemented. Browser loader integration and
per-context style ownership remain pending; browser import suppression continues.

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
unsupported structural-selector arguments, root-level `&`, and unbound namespace prefixes produce an
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
weight. Simple `:root` and `:global` become diagnosed host aliases. Functional `:global(...)` uses the same single-compound argument profile as
`:host(...)`, with a host-alias diagnostic. Complex and list-valued arguments
remain unsupported. Ordinary selector-list functions and relative
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

## Nested style rules: native output accepted

Accepted: preserve native CSS nesting in the emitted stylesheet, consistent
with the managed stylesheet's native `@scope` target. Shared import retains the
selector structure and parent-aware selector emission produces native fragments.
Supported grouping subtree assembly is implemented; full stylesheet assembly
and browser cutover remain pending.

The [CSS Nesting specification](https://drafts.csswg.org/css-nesting-1/#nest-selector)
assigns `&` the maximum specificity of the parent selector list. Simple textual
expansion into separate parent/child combinations can change the cascade.
Additionally, [nested declaration runs](https://drafts.csswg.org/css-nesting-1/#nested-declarations-rule)
retain the parent's pseudo-element matches; substituting an `&` rule for such a
run does not preserve those matches. These distinctions make native nesting the
accepted output format. Flat output is outside this implementation plan.

Implementation contract:

- Extend shared CSS import with a typed nesting selector and explicit nested-rule
  context, including implicit descendant and leading-combinator forms. Keep
  standalone selector-query capabilities unchanged. Emission consumes retained
  CEM nodes only, with no raw-selector parsing fallback.
- Carry parent selector context through grouping rules. Preserve authored rule
  and declaration order, including declarations following a nested rule.
  Rewrite host aliases using the existing declaration/instance rules; never
  substitute `&` for `:host`. Apply the instance root prefix at the outer rule,
  not independently to every nested child.
- Evaluate the declaration/shared `0-2-1` ceiling against composed authored
  specificity, including the parent contribution, before host normalization.
  For example, `.card { & .label { color: red; } }` has child specificity
  `0-2-0`; `.card.active { & .label { color: red; } }` reaches `0-3-0` and
  should diagnose and suppress that child selector. Recompute parent context
  from admitted branches so rejected branches do not contribute to emitted
  descendants. Suppress descendants when no parent branch remains.
- Preserve existing ID and manufactured-specificity restrictions. Add explicit
  checks for repeated specificity-bearing nesting selectors in one compound and
  duplicate class/attribute weighting introduced into that compound across a
  nesting boundary. Repeating a class in separate descendant compounds remains
  distinct from manufacturing weight in one compound.
- Keep unsupported forms diagnosed with source ranges. Native output still
  requires retained selector validation and policy checks; it does not authorize
  passing arbitrary authored selector strings through to the browser.

Implemented import boundary: style rules carry `selector-context="root"` or
`"nested"`. Media, supports, layer, container and starting-style groups
carry an existing parent context through their bodies; they do not create one.
Other recognized at-rule bodies, including keyframes, start without inherited
selector context. Authored `@scope` also resets it: its body uses a
[scope-relative reference](https://drafts.csswg.org/css-nesting-1/#nesting-at-scope),
not the enclosing style selector. Retaining that additional reference kind is
deferred; `&` directly inside authored `@scope` stays unsupported. This metadata
does not authorize otherwise forbidden at-rules.

Nested lists retain explicit `simple-selector kind="nesting"` nodes with their
source ranges, leading combinators, and implicit descendant forms. Their outer
selectors carry `specificity-kind="parent-dependent"` instead of a context-free
`specificity` tuple. Functional selector arguments containing `&` receive the
same conservative marker, including `:where(&)`; downstream evaluation must
apply the pseudo-class's weight rules. Root selectors and parent-independent
functional arguments keep their existing specificity metadata. Unsupported
nested grammar remains an unsupported list without partial structure.

The context-free declaration and instance selector helpers suppress nested rules with
`cem.scoped_css.nesting_context_required`, even when its text contains no `&`.
Ordered parent-rule assembly continues to retain the child as a deferred slot.
Standalone selector queries still reject nesting.

`emit_css_nested_selectors(tree, rule, mode)` now emits native nested-selector
fragments using the actual retained ancestor chain. It admits each ancestor's
selector list using the same declaration or instance policy, then computes the
child's authored specificity with the maximum admitted parent weight. Rejected
parent branches contribute neither weight nor inherited compound tokens. A child
of an entirely suppressed parent is omitted with
`cem.scoped_css.nesting_parent_suppressed`. The result includes ancestor
diagnostics; a recursive stylesheet compiler should avoid reporting them twice.

The helper retains explicit `&` and makes implicit nesting explicit (`.label`
becomes `& .label`, `> .label` becomes `& > .label`). It evaluates functional
selector weight from retained nodes, including zero-weight `:where()`, and
applies the declaration/shared `0-2-1` ceiling before host normalization. Instance
selectors inherit their parent's generated prefix without receiving another one.
Every emitted selector now carries its retained node ID alongside source/range.

Compound checks include admitted parents' terminal class/attribute tokens and
subject intersections through `:is()` and functional `:host()`/`:global()`.
These intersection checks also apply to root compounds: `:global(.a).a`
cannot hide repeated class weighting behind its argument boundary. If any admitted
parent alternative would repeat a weighted token in the child's compound, the
whole child selector is suppressed. Descendant compounds remain separate;
`:where()` contributes no weight. Pseudo-element parent branches contribute to
the maximum specificity but not to duplicate subject tokens, since `&` cannot
match their pseudo-elements. Repeated weighted parent intersections such
as `&&` and `:is(&):is(&)` diagnose as manufactured specificity. Checked arithmetic
suppresses overflowing weights with `cem.scoped_css.specificity_overflow`.
Ancestor traversal is bounded and stops at unsupported/reference-resetting
boundaries, including authored `@scope`.

These fragments must stay nested beneath the same admitted parent selectors.
The helper does not validate grouping conditions, assemble declarations, emit
complete stylesheets, or install browser CSS. Those steps remain pending.

Before browser cutover, fixtures must establish parent-list specificity,
multi-level context, `:where()` zero specificity, host normalization, instance
prefixing, rejected parent/child branches, duplicate weighting, pseudo-elements,
and declaration order through media/supports groups. Rebuild WASM after native
fixtures pass and verify computed browser behavior for the emitted nesting,
including nested declaration runs. Automatic `url(#id)` handling stays deferred.

## Starting-style grouping: implemented natively

Shared import retains `group-starting-style` with a `syntax-valid` flag and
original prelude components. Only an empty or comment/whitespace-only prelude
is accepted. The grouping emitter diagnoses and suppresses nonempty preludes
with `cem.scoped_css.starting_style_prelude_invalid`; a missing block uses the
existing `cem.scoped_css.group_block_required` diagnostic. No raw-prelude
fallback is used.

The emitted group carries its containing style context, so direct declarations
and nested selectors retain their order and parent specificity. Top-level
declarations without a style parent remain suppressed. The browser owns the
transition lifecycle, following [CSS Transitions Level 2](https://www.w3.org/TR/css-transitions-2/#defining-before-change-style).
The public CSS schema declares the media/supports/starting-style nodes and the
existing selector-context, parent-dependent specificity and nesting vocabulary.
Keyframe compilation remains pending.

## Container grouping: implemented natively

Shared import retains `group-container` with query components and outer-syntax
validation. The admitted profile includes optional decoded container names,
name-only conditions, comma-separated conditions and boolean combinations of
parenthesized/function atoms. Empty list entries, reserved names, recovered
tokens and invalid outer boolean structure invalidate the whole group. Escapes,
comments and commas inside functions remain retained.

The emitter preserves those components and suppresses an invalid group with
`cem.scoped_css.container_condition_invalid`. It does not interpret individual
size, style, scroll-state or future feature atoms; query evaluation and feature
support remain browser-owned. The grammar follows
[CSS Conditional Rules Level 5](https://drafts.csswg.org/css-conditional-5/#container-rule).
This is outer-grammar validation, not full feature-grammar conformance.

Container groups propagate parent selector context and ordered declarations
through the native subtree composer. Chromium fixtures verify named size and
style queries, nested matching, and changes after resizing the container or
updating its custom property. Name-only and query-list forms have native
retention/emission coverage; these fixtures do not claim universal browser
support for those newer forms.

## Native rule subtree composition

`emit_css_rule_subtree` composes a top-level retained style, media, supports, container or starting-style
rule with its supported descendants. Style rules select root or parent-aware
selector emission from import-owned context. Grouping rules carry the enclosing
style context through their bodies. The result keeps ordered text fragments,
node IDs, source maps and ranges; `css()` joins those fragments for consumers.

Declarations retain their original positions before, between and after child
rules. The composer does not wrap later declaration runs in `&`, preserving
pseudo-element matches under native CSS nesting. Suppressed child rules and
empty groups disappear; valid siblings remain. Parent selector diagnostics are
reported once per source occurrence even when multiple descendants revisit them.
Traversal is bounded at 64 rule levels, alongside the shared import limit.
Detached nested entrypoints are rejected: a subtree must begin at a top-level
rule in its retained stylesheet or style block.

Unsupported descendants, including imports, other grouping rules and keyframes,
produce `cem.scoped_css.subtree_construct_unsupported` and are omitted. This
subset is not the complete stylesheet compiler and does not enable browser
runtime installation. Import closure assembly, remaining selector/grouping
support, keyframe rewriting and context lifecycle integration remain pending.

`yarn nx run cem_ml:verify:css-nesting` rebuilds WASM, exports fixture CSS from
the native composer and checks computed styles in Chromium. The fixture uses
explicit temporary CSS files; it does not serialize a retained AST or invoke a
new browser runtime API. It covers parent-list specificity, nested declaration
runs on pseudo-elements, media/supports order, rejected parents, instance scope,
host normalization, duplicate weighting and zero-weight nesting. It also
checks an entering element’s opacity transition from native `@starting-style`
output at the start, midpoint and end. This verifies
the native output's browser semantics, not WASM runtime integration.

## Functional global host aliases

Shared stylesheet import admits `:global()` with the same single-compound
argument grammar as `:host()`. Its authored specificity includes the host
pseudo-class weight plus the argument's weight. The emitter produces the same
private/instance root intersection as `:host()` and reports
`cem.scoped_css.global_alias` at the original source. Type arguments use `:is()`
to keep valid compound syntax. ID, duplicate-token and authored-ceiling checks
remain active; selector lists, complex arguments and empty arguments remain
unsupported. Standalone selector-query capabilities are unchanged.

Parent-dependent arguments participate in nesting weight and inherited subject
checks. Root compounds now apply those same subject-intersection checks, so
class/attribute repetition hidden inside host aliases or `:is()` is rejected.
Separate descendant compounds and zero-weight `:where()` arguments remain
allowed. Browser fixtures verify private and instance host matching without
matching a descendant merely because it has the same class.

## Retained keyframes: import boundary implemented

Shared CSS import now gives `@keyframes` and `@-webkit-keyframes` a typed
`keyframe-name` containing the decoded identifier or string, syntax status and
source components. Reserved identifier names and malformed/multiple name tokens
are marked invalid without inventing a usable name. Quoted names retain their
string identity. Frame blocks use `rule kind="keyframe"` and a
`keyframe-selector-list`, independently of DOM selector analysis.

The initial offset profile accepts `from`, `to` and percentage lists, preserving
source tokens/ranges and normalized offsets from zero through one. Invalid
lists expose no partial offsets. Import checks the original percentage number
before normalization so the tokenizer's f32 projection cannot round an
out-of-range value into admission. Timeline-range selectors remain explicitly
unsupported in this first profile. These forms follow the
[CSS Animations keyframe grammar](https://www.w3.org/TR/css-animations-1/#keyframes).

A keyframe body does not inherit a surrounding style selector. Its offsets
never receive host rewriting, specificity or instance-prefix policy. The definition and longhand-reference helpers below consume this representation.
Closure symbol ownership and browser installation remain pending. The subtree
composer continues to diagnose and omit keyframe constructs until that
compilation path is complete.

## Scoped keyframe definition emission

`emit_css_keyframes` emits a retained definition using a nonempty, caller-owned
stylesheet/context suffix. It exposes the original and scoped names and emits
the scoped name as an escaped CSS identifier, including for quoted source names.
The caller remains responsible for stable namespace ownership across its import
closure and for rewriting animation references to the same name.

Frame offsets bypass DOM selector rules: no host rewriting, instance prefix or
specificity policy applies. The emitter preserves authored offset tokens,
normalized offsets, frame/declaration order and source locations. Declaration
emission resolves resource URLs and suppresses important/recovered values using
the shared policy. Invalid names suppress a definition; unsupported offset lists
and body constructs diagnose at their retained sources. Missing typed structure
fails without a raw-prelude fallback.

Empty definitions and valid empty frame blocks are preserved. An empty named
animation still has a browser animation object and lifecycle; dropping its
`@keyframes` rule would change that behavior. Browser fixtures cover ordinary,
quoted-name and empty animations. They compose the definition and longhand-name helpers using an explicit native
symbol map. The general subtree composer continues to reject keyframes
until namespace and animation-reference compilation are integrated.

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

## Static animation-name references

Shared CSS import retains typed `animation-name-list` and `animation-name-slot`
nodes for both standard and prefixed longhands. `emit_css_animation_names`
consumes these slots and a native map of decoded original names to scoped names.
It preserves commas, comments, whitespace and source ranges; unmapped names
remain external. Replacements are CSS strings, so reserved words and spaces in
scoped names retain symbol identity. Bare `none` and CSS-wide keywords are never
renamed; quoted strings with the same spelling remain symbol references.

Dynamic name expressions and unsupported lists produce diagnostics without a
partial value. Missing typed slots fail without reparsing declaration text.
The helper emits a value fragment; ordinary declaration policy still applies.
The integrated declaration/subtree path below uses these slots. Closure
namespace ownership remains pending. Browser fixtures resolve original names
through the same map used to emit their keyframe definitions.

## Static animation shorthand references

The same reference helper accepts `animation` and `-webkit-animation`. Import
classifies each comma group in authored order, filling duration, delay, easing,
iteration, direction, fill and play-state slots before assigning an ambiguous
keyword to the name slot. For example, `1s linear linear` rewrites only the
second `linear`. Non-name values are retained as `animation-value-slot` nodes;
the emitter preserves their original tokens. This follows the
[CSS Animations shorthand parsing order](https://www.w3.org/TR/css-animations-1/#animation).

The supported static profile accepts literal times/counts, standard keyword slots,
`cubic-bezier()`, `steps()` and literal `linear()` stops, with bounds and
duplicate-slot checks. Unsupported functions (including `calc()`) diagnose the
whole declaration.
Any `var()`, `env()` or `attr()` substitution also diagnoses the whole shorthand:
a substitution can supply multiple tokens or comma groups, so the spelling of a
custom property alone cannot prove which slot it fills. This restriction does
not establish a dynamic-name authoring contract. Browser coverage verifies that
the rewritten second `linear` selects the scoped definition and retains linear
midpoint interpolation.

Static `linear()` admission follows the [CSS Easing Level 2 stop grammar](https://www.w3.org/TR/css-easing-2/#linear-easing-function-syntax):
at least two comma-separated stops, each with one finite literal number and an
optional contiguous run of one or two finite percentages before or after it.
Repeated, decreasing and out-of-range inputs are valid; browser easing evaluation
performs normalization. Import preserves the complete easing token range,
including comments and escapes, as an `animation-value-slot`. Only the separate
name slot is rewritten. Malformed stops, duplicate easing functions and math
expressions reject the shorthand; dynamic substitution retains its existing
separate diagnostic. The Chromium fixture verifies that a stop with two positions
holds 25% opacity at the animation midpoint using the scoped name.

## Animation references in native rule subtrees

`emit_css_rule_declarations_with_symbols` and
`emit_css_rule_subtree_with_symbols` accept a caller-owned native symbol map.
They apply name rewriting in ordinary and nested grouping declarations, while
preserving declaration order, URL resolution, source provenance and existing
selector policies. Important/recovered declaration suppression runs before
animation rewriting; unsupported animation values diagnose without suppressing
unrelated declarations. Custom-property contents retain their authored tokens.

The resource plan can be reused with different maps for different ownership
contexts. The older fragment APIs still leave names authored when no map is
supplied; the symbol-aware path validates even an empty map. Browser animation
fixtures now pass original source declarations directly through this path,
without reconstructing and reimporting emitted value fragments. The single-sheet compiler below collects symbols and assembles these parts.
Import-closure assembly and installation remain pending.

## Single-sheet assembly with keyframe collection

`emit_css_stylesheet` consumes one retained stylesheet/style-block and a
caller-owned suffix. Its first pass collects admitted keyframe definitions;
its second pass emits rules with the complete name map. References may precede
definitions. Duplicate definitions retain their authored positions and share
the same scoped name, including definitions inside supported stylesheet-level
conditional groups. The browser evaluates those conditions and applies normal
last-definition precedence. Empty definitions remain present.

Suppressed groups contribute no definitions. Keyframes nested inside style
rules remain outside this profile and diagnose; arbitrary unsupported at-rules
are not searched for symbols. Emitted definition, frame and declaration
fragments retain their source nodes/ranges. The result includes the native name
map and diagnostics, and still requires its managed scope wrapper. Imports are
explicitly diagnosed and omitted until closure compilation is connected; this
API does not authorize runtime installation. Browser fixtures now compile
references and definitions from the same retained source, including forward
references and a later conditional duplicate definition.

## Native import-closure emission

`emit_css_import_closure` compiles a ready, uncancelled `CssImportClosure` under
one explicit private, shared or instance scope. It expands each import at its
authored position, preserves supports/media wrappers, and uses each occurrence's
resource plan and final-response URL base. Layered imports remain suppressed by
the managed-CSS policy; their descendants contribute neither rules nor symbols.
No browser `@import` or downstream CSS parsing is introduced.

The caller supplies a nonempty stable owner identity covering the effective
stylesheet/declaration and resolution context. The compiler encodes its UTF-8
bytes as `cem-` followed by hexadecimal digits. This deterministic namespace is
independent of loading order or process-local handles. Callers must use distinct
identities for distinct ownership contexts; this API does not derive lifecycle
identity or register stylesheet ownership.

All admitted definitions across the import tree share one animation symbol map,
collected before rule emission. References work in either direction across file
boundaries. Repeated imports and duplicate definitions retain cascade order.
Each output fragment and diagnostic carries its sheet occurrence index, so local
node IDs remain unambiguous even when occurrences reuse the same retained tree.
Import wrapper fragments point to their original condition nodes.

Native fixtures cover repeated imports, forward references to root definitions,
final-URL resources, namespace determinism, provenance, suppressed imports and
pending/cancelled closure rejection. Chromium verifies a two-level conditional
import with a root animation reference and root cascade override. Runtime
transport, owner-identity derivation, cache/lifecycle wiring and installation
remain pending.

## Retained semantic roles in source presentation

The shared import projection refines the lossless CSS event stream with
`symbol` and `keyword` semantic roles using the retained keyframe and animation
slots. Source highlighting and schema formatter/colorizer profiles consume
those roles directly. No sample-side or CLI-side CSS parsing is added, and the
projection reuses the already-tokenized events.

Only identifier roles change: keyframe definitions/references use the existing
name palette; shorthand keywords and `from`/`to` use the existing keyword
palette. Strings, numbers, comments and functions retain their lexical roles.
Unsupported, recovered or over-limit projections retain broad source roles.
The browser sample includes `animation: 1s linear linear` to show the distinct
meanings with unchanged visible source. Nested HTML/CEM-ML/CSS source scopes and
CLI ANSI presentation use the same distinctions. Direct browser CSS export
continues to produce uncolored CSS.

Retained structural nth selectors also provide argument keyword ranges. The
prefix before their retained filter list colors identifiers such as `odd`,
`even` and `of` as keywords; identifiers inside the filter keep selector colors.
The sample `:nth-child(odd of .odd)` demonstrates the distinction. Escapes and
comments keep their original source bytes, and invalid filters keep broad roles.

The importer marks recognized conditional operators on `component-value` nodes
with `condition-role="operator"`. This includes outer and nested parenthesized
boolean expressions in media, supports and container groups, plus import
conditions. Media's `only` modifier receives the same role. A condition must
pass its outer grammar check before annotation; each media-list entry is checked
separately. Recursion follows recognized boolean expressions within the existing
64-level import bound, following [media condition grouping](https://www.w3.org/TR/mediaqueries-4/#mq-syntax).

Feature declarations, custom values and unknown functions remain opaque, so
`style(--theme: and)` and `future((x) and (y))` keep their existing colors.
Mixed boolean operators without grouping receive no nested operator metadata.
This metadata describes recognized syntax; it does not evaluate feature support
or expand the compiler's condition-admission profile. Source presentation uses
the retained roles and ranges directly, including in samples and CLI output.

## Bounded structural nth selectors

The stylesheet importer accepts An+B arguments for `:nth-child()`,
`:nth-last-child()`, `:nth-of-type()` and `:nth-last-of-type()`. Retained
`simple-selector` nodes carry numeric `nth-a` and `nth-b` attributes, and the
emitter writes a canonical expression from those fields without reparsing.
The original selector range remains available. Structural matching stays with
the browser, following [Selectors Level 4](https://www.w3.org/TR/selectors-4/#the-nth-child-pseudo)
and the [An+B grammar](https://www.w3.org/TR/css-syntax-3/#anb-microsyntax).

The initial profile bounds coefficient magnitudes to 2,147,483,647 and rejects
larger authored integers before cssparser can saturate them. It accepts odd/even,
signed coefficients and permitted comments/whitespace. Malformed expressions,
and bare nth pseudo-classes remain unsupported. Missing numeric metadata
produces a diagnostic without a text fallback.

`:nth-child()` and `:nth-last-child()` also accept `of <selector-list>`.
The importer retains the filter as a child selector list alongside the numeric
coefficients; the emitter serializes that tree. Specificity includes the
pseudo-class weight plus the maximum filter weight, with parent references
resolved through the existing nesting context. Same-subject filters participate
in repeated-class, attribute and parent-reference checks. `:where()` retains
zero weight. Invalid or relative filter lists, pseudo-elements, IDs and filters
above the declaration specificity ceiling are rejected. The type-indexed nth
forms still reject `of` filters.

Standalone selector-query capabilities are unchanged. Native fixtures cover
canonical output, source ranges, escaped keywords, nesting and policy limits.
Chromium checks all four numeric forms and both filtered child-index forms,
including a nested filter referencing its parent selector.

## Retained compiler suppression diagnostics

The retained compiler now uses the contract's diagnostic codes for authored
`@scope`, the current runtime's document-global at-rule list (`@font-face`,
`@property`, `@counter-style`, `@font-palette-values`, `@page`, `@namespace`),
and library `@layer` statements or blocks. Suppression preserves the original
source range and import occurrence identity. Sibling rules and declarations
remain in output. Unknown constructs and instance layers retain the generic
unsupported-profile diagnostic; this change grants no new syntax admission.

Declaration-list inference now checks for a top-level colon token. A colon in
an import/namespace URI, nested condition or comment cannot select that mode.
Explicit declaration-list mode and ordinary declarations remain supported.

## Shared literal pseudo-class arguments

`:dir()` and `:lang()` use the same `literal-arguments` list of `literal-argument`
nodes. Each argument retains its identifier/string kind, decoded value and source
range. The pseudo-class carries `argument-kind="literal"`; the list carries its
analysis status. One serializer emits escaped identifiers or quoted strings,
without reading source text, interpreting language/direction values or resolving
URLs. Literal arguments contribute no selector weight of their own: their
pseudo-class has ordinary specificity, including under nesting and `:where()`.
This replaces the direction-only enum and retained `direction` attribute.

The importer validates the supported grammar: one `ltr`/`rtl` identifier for
`:dir()`, or one or more comma-separated identifiers/strings for `:lang()`.
The language profile preserves case, escaped identifiers, quoted wildcards and
empty strings. It does not validate language tags or implement matching.
Direction's other identifiers remain outside the current supported profile,
though Selectors Level 4 defines them as valid and nonmatching.

The browser evaluates [directionality](https://www.w3.org/TR/selectors-4/#the-dir-pseudo)
and [language matching](https://www.w3.org/TR/selectors-4/#the-lang-pseudo), including
inherited attributes and explicit overrides. CSS `direction` does not determine
`:dir()` matching. Native selector-query capabilities remain unchanged.

The shared syntax stream consumes each literal's retained presentation role.
Direction keywords, language identifiers and quoted language strings reuse the
existing keyword, name and string palettes in source views, samples and CLI
output. Escaped source text is preserved, selector names outside argument ranges
keep their roles, and unsupported input keeps broad fallback colors.

The Chromium 148 verification browser accepts identifier language ranges but
rejects lists and quoted ranges, including wildcard and empty-string arguments.
The browser gate verifies inheritance and updates with identifier arguments,
then compares authored and emitted behavior for each advanced form and reports
syntax support. The compiler preserves those forms without emulating matching.

## Retained compiler coverage audit (2026-09-26)

The audit compared the retained compiler with the managed-CSS policy cases in
`packages/cem-elements/src/lib/processing-boundary.spec.ts`, the scope and
lifecycle stories in `css-scoping.stories.ts` and
`dom-stylesheet-adoption.stories.ts`, and the accepted scoping contract.
It is a migration-readiness audit, not a claim to support all CSS syntax.

| Area | Evidence and remaining work |
| --- | --- |
| Host aliases, nesting, specificity and suppression | `css_emission`, `css_nesting_emission` and `css_subtree` cover retained structure and diagnostics. The combined `retained_stylesheet_covers_existing_runtime_managed_css_contract` regression checks forward animation references, aliases, nested declaration order, policy suppression and source provenance together. |
| Scope boundaries | `css_rule_assembly` covers private/shared/instance wrappers and context qualification. Existing browser scope stories remain the cutover acceptance gate for projection limits, inheritance and cascade. |
| Groups and animations | `css_grouping`, `css_keyframes`, `css_animation_names` and `css_subtree` cover the admitted media/supports/container/starting-style and static animation profiles. No additional function grammar was identified as necessary for the audited runtime fixtures. |
| Imports and explicit URLs | `css_resources`, `css_resource_emission` and `css_import_closure` cover context-specific resolution, retained byte delivery, conditions, cross-sheet symbols and emitted source provenance. Single-sheet emission deliberately diagnoses imports as pending; it cannot replace closure compilation. |
| Other URL-bearing syntax | At audit time, quoted `image-set()` candidates were ordinary string components. The resolver recognized imports, URL tokens and quoted `url()` only. These candidate strings would keep the wrong relative base when an imported stylesheet is installed in the document. The subsequent [image candidate fixture](#retained-image-candidate-resources) now covers this static profile. |
| Runtime integration | `projection.ts::scopeCssText` still performs string-based compilation. `derive_css_stylesheet_identity` and the retained processing-host protocol now own context-specific identities and emitted sets. `DeclarationStyleOwnership` now admits native context-qualified sets and consumer leases. The opt-in runtime now routes declaration and inert-payload CSS through native loads, consumer leases and render/hydration readiness. Ordinary result styles remain on their separate path. |

The combined regression compares canonical retained output, rather than legacy
spacing: unqualified type selectors emit with an explicit wildcard namespace,
and rewritten animation references emit quoted names. Existing browser animation
fixtures already exercise those representations.

Known syntax limits remain explicit: functional pseudo-elements, `:state()`,
complex/list-valued host aliases, dynamic animation names/shorthands and some
static timing-function forms are outside the retained profile. They are not a
reason to add one emitter per function. Add grammar only with a concrete fixture,
using shared retained representations wherever applicable.

Audit follow-up: the static image candidate, effective identity and processing-host
fixtures below complete the first three native integration steps. Static
`linear()` easing is also admitted. A follow-up scan of the scope/adoption stories
and `demo/scoped-css.html` found no use of the deferred functional pseudo-element,
`:state()`, complex host-alias or mathematical animation profiles.

The diagnostic audit found that ready WASM output carried only a sheet index and
range. It now also carries `sourceUri` and `stylesheetUrl`: the former identifies
the retained source containing the byte range, while the latter is the resolution
base (including redirects). These differ for inline sources and may differ for
native tree delivery. Consumers must not treat either as a serialized tree.
Native owner lifetime checks cover this source projection as well as emitted CSS.
Real WASM fixtures verify root/import distinction, multiple suppression codes,
UTF-8 byte ranges and redirect provenance. Worker and fallback tests verify that
these locations survive transport. Host transition diagnostics have no source
location and remain ordinary processing diagnostics.

Failed load operations also preserve their structured native code and import
location through WASM and worker/fallback errors. MIME, integrity and response
byte failures point to the import that requested the response. The native loader
also attaches that location to transport failures.
Invalid placement in a downloaded sheet keeps that sheet's own location instead
of being overwritten by its parent import. Errors about stale handles or invalid
request IDs do not invent a source location. The host exposes structured native
failures as `CemProcessingDiagnosticError.diagnostics`; ordinary host/control
failures keep their generic processing diagnostic.

The scope/adoption browser fixtures now install native processing-host output in
test-owned styles. Real worker and forced fallback runs repeat the existing
projection, nested-host, inheritance, proximity and public-override checks. They
also compare shared membership, animation ownership and suppression diagnostics.
The boundary fixture exposed an over-budget authored host selector; wrapping its
host condition in `:where()` keeps the containment test within the accepted
`0-2-1` ceiling. Native compilation correctly diagnosed the original selector.

`DeclarationStyleOwnership.beginConsumer` now accepts complete native output for
one connected consumer generation. It reuses style elements by native cache key,
orders all context variants by source occurrence, and retains shared context
markers across declarations. Disconnect or scope disposal releases the consumer;
the last consumer removes its set. Replacing a consumer invalidates late commits,
and a remove/reinsert cycle requires a fresh lease. Native release callbacks are
supplied by the caller so they can target the exact processing-host generations.
Real worker/fallback browser fixtures check these transitions and computed CSS.
Each lease also exposes an abort signal for pending processing jobs or response
reads. Replacement publishes the new generation before notifying old handlers,
so a synchronous replacement from an abort handler cannot leave a live orphan.
Cleanup must release native load IDs individually; a delayed cleanup for a
superseded import must not release the replacement load.

The pending-import browser case also exposed an admission mismatch: shared import
produces a `style-block` tree for CSS containing `:host` or `@scope`, but downloaded
imports previously required a `stylesheet` tree. Import closure admission now
accepts both native rule containers. Declaration-list (`style-attribute`) trees
remain invalid, and authored scope suppression keeps the imported source location.

`installRetainedStylesheets` now coordinates native begin/delivery operations
with a consumer lease and one readiness promise. It captures the resolver context,
keeps native failures structured, installs independent successful occurrences,
and settles cancellation even if a reader ignores its abort signal. Cleanup uses
exact load generations and accounts for worker replacement. Browser fixtures use
real worker/fallback hosts for redirects, map snapshots, disposal, reconnect and
reuse of existing DOM content. The main runtime uses this coordinator when
`retainedStylesheets: {}` is configured; default cutover remains pending.

`CemStylesheetRegistry` now routes registered native source handles to connected
consumers. Private occurrences match effective declaration identity; shared
occurrences match public scope membership, including sources with no produced
instances. Each connection captures its consuming module context. Readiness
includes sources added or removed during loading. Source removal, disconnect,
context replacement and ancestor disposal release exact native generations;
reentrant cancellation cannot overwrite a newer connection. Real worker/fallback
browser fixtures cover these transitions and shared style reuse. In the opt-in
runtime path, declaration settlement registers admitted CEM-ML/DOM occurrences,
and render settlement waits for the consuming connection. Source scope disposal
releases registrations; disconnect and ancestor disposal abort pending renders.
Version-compatible hydration waits for imports while retaining existing DOM.
The reserved context marker is restored after mutation and excluded from snapshots.

Browser resolver configuration identities now survive reconnects. Live lookup
handles remain distinct for referrer selection, while equivalent maps, bases and
policies produce equal native context fingerprints and reuse derived styles.
Changes to mapping targets or policy still invalidate those identities. Ordinary
CEM-ML source and binary compilation both return stylesheet occurrence metadata
alongside their retained artifact handles; CSS trees stay native.

XSLT result styles are excluded from this declaration registry. The approved
[result-construction contract](xslt-runtime-lowering.md#native-output-construction)
keeps them inside their rendered branch; an XSLT component's empty declaration
stylesheet collection is intentional. Do not hoist result styles or introduce
declaration CSS handles merely to replace the XSLT render adapter's sentinel ID.

The default browser reader preserves the response URL and content type, rejects
HTTP failures and cancels body reads on abort. It buffers at most the native
16 MiB response limit plus one excess byte, then cancels the stream. Native
admission diagnoses that excess byte with `cem.css.import_byte_limit`; oversized
responses are never accepted as truncated stylesheets. Hosts may override the
reader with `retainedStylesheets: { read }`.

Next, exercise the authored CSS demos through this runtime path before enabling
the default compiler. The runtime
fixtures cover nested module maps, context replacement and hydration. Instance
styles still require their own retained ownership path;
the declaration protocol deliberately rejects instance scope. The remaining syntax
profiles stay explicitly diagnosed until a concrete fixture requires them; this audit does not
authorize raw-text fallbacks.

Automatic fragment binding remains deferred. Literal functions such as `:dir()`
and `:lang()` do not participate in URL resolution.

## Retained image candidate resources

Shared import classifies quoted image candidates in `image-set()` and its
`-webkit-image-set()` alias with `resource-role="url"` on the existing string
component. This follows the [CSS Images grammar](https://www.w3.org/TR/css-images-4/#image-set-notation).
The function retains `resource-analysis="complete"` or `"unsupported"`.
No resource interpretation is added to literal selector functions.

The initial profile admits string/URL candidates and gradient functions, with
optional static resolution and `type()` descriptors in either order. It leaves
image selection, MIME support and resolution semantics to the browser. Dynamic
candidate/descriptor substitutions, calculated resolutions and other image
function forms remain outside this profile. These diagnose as
`cem.scoped_css.resource_grammar_unsupported` and suppress their declaration
while preserving siblings. This classification does not validate gradient bodies.

The existing resource plan resolves classified strings with the nearest map and
the owning sheet's final URL. The emitter writes their resolved value as `url()`;
no candidate-specific serializer is needed. Quoted `type()` values, comments,
ordinary strings and `:lang()` arguments retain their authored text. Local
fragment candidates remain unchanged under the existing deferred-binding policy.
Blocked/failed candidate references use the existing declaration suppression.

Native fixtures cover context reuse, escaped strings, exact source ranges,
redirected/repeated imported sheets, descriptor order, the prefixed alias and
unsupported forms. The browser gate checks computed candidate URLs from native
output after rebuilding WASM. Runtime installation remains pending.

## Effective stylesheet identity (native)

`derive_css_stylesheet_identity` derives a qualified managed scope and two keys
from a ready native import closure, stable declaration identity and style
occurrence. It performs no caching or browser installation.

- **Context marker:** a versioned BLAKE3 fingerprint of the effective resolver
  configuration. The scoped resolver includes context/resolver identity, policy
  stamp, ordered frames and scoped maps, base URLs, all mapping targets and
  MIME/integrity metadata, blocking entries and allowed schemes. Map keys and
  scheme sets have deterministic order. Length-prefixed fields distinguish
  absent, empty and differently partitioned values. Lookup handles and pointers
  are excluded. Context and frame identities must themselves survive hydration.
- **Owner key:** the declaration, style occurrence, private/shared/instance scope,
  source stylesheet base and effective context marker. It is the input to the
  existing closure emitter's keyframe namespace. Editing CSS or refreshing an
  imported response preserves this owner's namespace.
- **Cache key:** the owner key plus every retained sheet's import-owned source
  fingerprint, final URL and ordered import edges. Changed root/imported content
  or response bases invalidate compiled output. Source provenance is deliberately
  part of the imported fingerprint, so cached diagnostics retain their owner.

Only styles with imports or non-fragment resource references require a context
marker. Plain styles and authored local fragments keep the single
context-independent set. Qualified private/shared roots use the existing native
scope wrapper. Instance CSS retains its implicit scope; instance callers must
supply the persisted instance identity as part of their ownership input.

`CemModuleUrlResolver::context_cache_identity` is optional. The built-in scoped
resolver hashes its immutable typed configuration directly. Custom resolvers
must fingerprint all resolution inputs before opting in; the default is
uncacheable. The closure captures the fingerprint before resolving its root.
Identity derivation rejects a changed or unavailable fingerprint, incomplete or
cancelled closures, missing source fingerprints, empty owner fields and a
caller-supplied context marker. It never substitutes a process-local handle.
These versioned hash domains must change when their encoded inputs or compiler
semantics change.

Native tests reconstruct trees and resolver handles to verify stable identity,
exercise map/policy/base/scope/content invalidation and verify cancellation and
uncacheable resolvers. The browser fixture installs two native-emitted variants,
checks their isolation from each other and unmarked hosts, then changes a host's
marker. This proves scope behavior; runtime ownership, hydration restoration and
cache lifecycle are still pending.

A cache key is not an authorization or freshness decision. The caller must
revalidate dependency responses through the loader, confirm the consuming context
is live and invalidate pending work on context changes before committing styles.
The processing-host protocol below carries these identities through the retained
artifact lifecycle. Replacing declaration-only browser ownership still requires
context-specific installation, reconnect and readiness coverage.

### Context changes during native import loading

Fingerprint-aware closures now report `cem.css.import_context_changed` when
status, identity derivation, request creation or response delivery observes a
changed resolver context. The invalid state persists even if the previous
fingerprint returns. Root construction and imported-sheet attachment check again
after invoking the resolver, before exposing the new tree. Stale tree/byte
delivery attaches no sheet or edge and does not increment received bytes.
Diagnostics preserve the outstanding import's source location when available.

Existing cancellation and failure handling remains in force. Custom resolvers
without a fingerprint still require host-driven cancellation on context changes
and cannot produce a context-dependent cache key. Browser integration must also
check context liveness before committing a marker or style set; these native
checks do not install or dispose browser styles.

## Retained template stylesheet collection (native)

`cem_ql::retained_template::RetainedTemplate` is now the owner stored in the
processing host's WASM template registry. It keeps the existing template and
reader cache alongside derived stylesheet records. Dropping a template handle
through `disposeTemplate` drops this owner and invalidates its output handles.

`retain_stylesheet` accepts a ready native closure, an admitted declaration
scope, stable declaration identity, style occurrence index and consumer identity.
The root tree must be the same retained tree owned by that template occurrence;
equal text in an unrelated tree is insufficient. Explicit authored shared scopes
must agree with the effective scope, and declaration artifacts cannot be admitted
as instance styles. Each retained record owns the closure, emitted fragments,
diagnostics and the derived identity; imported node IDs therefore retain their
native source owners.

Matching cache keys share one `Arc` record. Different contexts or occurrences
remain separate. A consumer can retain multiple occurrences. Replacing one
occurrence releases its old record only when no other registered consumer needs
it. Invalid input leaves that consumer's current binding intact. The incoming
closure and reused output must both remain active before publication.

Releasing the last registered consumer removes the collection's strong reference
and invalidates outstanding output handles. Template disposal does the same for
all records. Accessing emitted CSS checks this lifetime token and the source
closure's cancellation/context state. Existing external references can keep
native memory alive until dropped, but cannot read installable output after
release. Reconnect creates a fresh output handle from an active source closure;
release does not cancel a source closure shared with another operation.

Native fixtures cover sharing, context isolation, replacement, multiple style
occurrences, imported source/diagnostic retention, wrong roots, pending/cancelled
closures, authored scope mismatch, reconnect and disposal. WASM rendering still
uses the same artifact and reader cache through accessors.

### Processing-host stylesheet protocol

The `cem-processing-host-v10` `stylesheet` operation populates this collection
through four controls:

- `begin` supplies an artifact handle, consumer, occurrence index, admitted
  private/shared scope, source base and the existing module resolver context
  wire metadata. The engine takes declaration identity from the validated
  artifact. Native code uses that template's retained root tree and returns a
  pending import request or ready output.
- `deliver` supplies the outstanding request ID, response bytes, final URL and
  content type. Native `complete_response` applies MIME, integrity, redirect,
  response and aggregate byte checks before shared CSS import. It returns the
  next request or retains the ready closure and emits CSS, identity and diagnostics.
- `fail` supplies the outstanding request ID and a reader/transport error message.
  Native admission marks the import failed and returns `cem.css.import_load_failed`
  with the requesting import's retained source location. Stale loads and another
  consumer's handle are rejected without changing the active load.
- `release` drops all occurrences for a consumer, or only a specified load
  generation. Late cancellation of an older generation cannot release newer work.

Each template permits at most 64 pending loads. A new begin supersedes pending
work for the same consumer and occurrence. Load IDs are monotonic within the
native template; the engine qualifies them with its artifact owner's identity.
Eviction, worker loss or template disposal therefore cannot make an old host
handle refer to a new native job. Failed worker delivery requires a fresh begin;
response bytes are never replayed into another load. Begin can retry in fallback.

The host isolates consumer names per root; the engine isolates them per logical
artifact even when compilations are shared. Artifact eviction and root disposal
release their consumers. Worker and fallback cancellation both release the exact
returned generation, including replies received after host cancellation.

These controls carry named metadata and bytes; CSS trees remain native. Returned
CSS is an explicit output boundary. The resolver wire snapshot is immutable for
a load. Browser installation must supersede loads when context changes and check
context liveness before committing emitted CSS or a context marker.

`cem_ql:test:retained-stylesheets` runs native ownership/load tests and the real
WASM protocol fixture, including source-located suppression diagnostics.
Processing-engine and host tests cover shared compilation,
root disposal, stale handles, fallback and late cancellation. The installation
coordinator now drives these operations with caller-supplied byte transport,
including native failure reporting. Main runtime readiness routing and bounded
default byte transport are available through the opt-in configuration; compiler
cutover remains pending.

## Authored demo acceptance with retained runtime CSS

`cem-elements:test:retained-css` runs the existing browser suite with
`retainedStylesheets: {}` in the shared Storybook preview. The default test lane
is unchanged; stories constructing independent runtimes keep their own options.
The environment switch participates in the test cache identity.

The original 13 scoped-CSS samples use the same assertions in both lanes.
Samples 14/15 additionally verify native imports in the retained lane and import
suppression in the default lane.
Animation assertions read the browser's keyframe definition and running
animation, and check that the name survives reconnect. This permits the native
owner-key namespace documented above without depending on legacy serialization.
The repeated hex-gallery story waits on the preview runtime that owns its
registrations. Native derived styles are released with their last consumer;
remount must reproduce the same CSS and computed behavior, while the default
compiler also retains its stylesheet DOM nodes.

The wider acceptance run exposed two lifecycle issues. Late resource events
can update retained state after disconnect, but rendering now waits for a live
connection. Exact-generation stylesheet release uses the worker control path,
so bulk removal does not fill the bounded work queue and reject new renders.
A saturated-queue unit fixture and a worker/fallback disconnected-event story
cover these independently of the authored demos. The stock-cell interaction
also awaits runtime readiness around selection instead of relying on a short
DOM polling timeout.

## Native instance stylesheet owners

`RetainedTemplate::new_instance_stylesheets` adopts extracted inert-payload CSS
sources under one fixed, persisted instance identity. `adoptInstanceStylesheets`
exposes the same boundary through WASM. The source batch uses the existing
explicit CSS source/control format; retained CSS trees stay in the native owner.
The host remains responsible for admitting the inert payload envelope.

An instance owner accepts only `scope: { kind: 'instance' }`. A declaration owner
still rejects that scope. Each load must use the instance owner's fixed identity
in the existing `declarationIdentity` control field, so another identity cannot
rename or reuse the owner. Source batches carrying a shared scope are rejected.

Native output retains its implicit parent-rooted `@scope`; it does not insert a
context selector. Resource resolution still uses the supplied consuming context,
and that context contributes to the native owner/cache identity when needed.
Animation names survive reconnect under the same instance and context, while
different instance IDs have separate namespaces. Redirected imports and exact
load-generation cancellation use the existing native loader.

Processing-host protocol v10 carries `instanceStylesheetIdentity` only for CSS
source adoption. The identity is part of the native compilation cache key and
same-artifact reuse checks. Instance loads pass this fixed identity through the
WASM control boundary; declaration loads retain their registration identity.
Worker and fallback engines both reject crossing those ownership modes.
The shared installation coordinator accepts typed scope sets. Declaration leases
remain restricted to private/shared styles, while instance leases commit only to
the consuming host.

With `retainedStylesheets: {}`, the runtime extracts authored CSS from the inert
payload and adopts it into native instance owners. Direct host style children
sit outside the render range. Protocol v10 carries `payloadStylesInstalled` so
DOM and worker render plans omit payload styles without parsing their CSS;
ordinary result styles still follow their existing path.

First render and hydration wait for imports. Compatible hydration preserves
rendered DOM and animation names, and reuses the direct style nodes. The scope
policy stamp includes `retained-instance-css` to reject older hydration output
that placed payload CSS inside the render range. Payload edits and moves between
module-map contexts replace the installation while retaining the instance ID.
Disconnect and ancestor disposal abort reads and release native generations;
stale delivery cannot commit. A failed occurrence reports native source ranges
without dropping independent valid occurrences. Worker/fallback fixtures cover
these paths for DOM and CEM-ML declarations. Default compiler cutover remains
pending.


### Nested inert payload ownership

Payload CSS collection and render-plan cleanup stop at HTML templates, including
explicit XHTML namespace templates. Styles inside those templates remain authored
source for the eventual child consumer. Boundary tests cover both namespace forms
in legacy and retained modes, including imports that the parent must not process.


## Migration gate reconciliation (2026-09-26)

The import-loader and compiler implementation gates are complete for the accepted
managed-CSS profile. Their umbrella checklist entries had not been closed as the
individual fixtures landed.

| Gate | Implementation and verification |
| --- | --- |
| Byte admission and import closure | `css_import_closure` covers MIME, strongest supported integrity digests, per-response and aggregate bounds, redirects, cancellation, placement and import conditions. `retained_stylesheets` exercises load generations through the retained owner; its WASM fixture verifies the public boundary. |
| Complete scoped emission | `css_subtree`, `css_nesting_emission` and `css_rule_assembly` verify ordered declarations, grouping, nesting, managed wrappers and keyframe references. `emission_compiles_import_occurrences_with_shared_names_and_final_url_bases` combines imports, conditions, cross-sheet animation references, final URL bases and cascade order. |
| Browser installation | Worker/fallback fixtures and both full browser lanes cover declaration/shared/instance ownership, consuming contexts, readiness, hydration, cancellation and independent failures. Native source trees remain inside the processing host. |
| Override syntax | The accepted ordinary `resource` entry is implemented. `css_uses_nearest_mapping_and_preserves_non_css_precedence` verifies nearest CSS lookup and metadata while preserving non-CSS precedence; adjacent tests cover URL-scope specificity, fallback and ancestor restrictions. |

These implementation gates do not enable the default runtime. The authored example now demonstrates a stylesheet import and resource URL
overridden by a nested module map, using repository assets in standalone and
source-loaded modes. Next, verify the default-cutover trial against that example
and the existing browser lanes. The explicit syntax limits in the compiler coverage audit still
apply; broader CSS grammar remains fixture-driven work.

Verification: 85 focused native tests passed across CSS adoption, resources,
resolver precedence, import closures, subtree emission, nesting and rule assembly.
`cem_ql:test:retained-stylesheets` passed from the Nx cache, including retained-owner
and real WASM checks. The prior integration run passed 550 unit tests and 284
browser stories in each lane; this reconciliation changes documentation only.


### Authored module-map import samples

`scoped-css.html` enables retained loading explicitly. Sample 14 maps `card-theme`
and `card-icon` to a green stylesheet and smiling image. Sample 15 repeats the
outer defaults and replaces both entries in a nested DCE module map, producing
purple text and a confused image. Both stylesheets resolve their `url(card-icon)`
through the consuming context. The samples use local assets and ordinary resource
entries; no new override syntax is introduced.

Standalone and source-loaded fixture checks verify imported color, image URL,
source presentation and overflow at 1280px and 390px. The retained Storybook lane
verifies both mappings; the default lane verifies import suppression. The browser
reader requests `Accept: text/css`, so development servers return CSS bytes rather
than JavaScript module wrappers without requiring special queries in authored URLs.
