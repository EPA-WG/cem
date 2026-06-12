# Custom-Element XSLT Parity Decision

This records the Phase 4 blocker and decision behind CEM component expansion:
copied `@epa-wg/custom-element` components/samples use more XSLT than the
current bounded custom-element fragment bridge.

## Current State

Done:

- `cem_ml::legacy_custom_element` lowers the bounded custom-element fragment
  dialect to canonical CEM-ML for `--content-type custom-element-xslt`.
- The supported fragment subset includes bare and `xsl:` `if`, `choose`,
  `when`/`otherwise`, `for-each`, `variable`, `value-of`, AVT interpolation,
  slots, declaration/resource helpers, and the fixture-derived XPath function
  subset.
- The engine converter is exported to WASM as
  `convertLegacyCustomElementTemplate`.
- The material component fixture gate reads
  `packages/custom-element/test-fixtures/legacy-compat-manifest.json` and the
  copied material component HTML files.

Partially implemented:

- `xsl:stylesheet` dispatch, root `xsl:template match="/"`, named
  `xsl:template`, `xsl:call-template`, `xsl:with-param`, `xsl:param`, and a
  bounded `xsl:apply-templates` path over inline `exsl:node-set($var)/*`
  variables now lower through `cem_ml::legacy_custom_element`. `xsl:for-each`
  also unrolls current-node selections such as `@*|*` when a template has a
  concrete current item.
- The current apply-template selector supports simple match patterns, `mode`,
  source-document child/attribute/text traversal (`*`, `@*`, `text()`, `.`),
  sample-style absolute/descendant selectors, namespace-qualified local-name
  matching, namespace wildcards such as `xhtml:*`, indexed child steps,
  parent-relative paths, current attribute/child unions,
  preceding-sibling selectors, variable-rooted current node paths such as
  `$rowNode/*[name()=$key]` and `$rowNode/@*[name()=$key]`,
  named attribute selection, union splitting, simple predicates (`*`,
  `not(*)`, `@attr`, `@attr='value'`, `@attr=$param`, `name()=$param`, and
  `name()=name(current())`), basic template `priority`, default template
  fallbacks, a
  bounded recursion guard, and child `xsl:sort` over one or more literal/numeric
  keys. It is a compatibility adapter profile, not a general XSLT stylesheet
  engine.
- The current expression subset also evaluates sample-style `count(...)` and
  `sum(...)` calls when their argument is already in the supported node
  selection subset, plus sample-style numeric `count(...) + n` expressions.
- The current construction subset supports direct `xsl:attribute` on emitted
  elements, `xsl:copy` for the current node, and bounded `xsl:copy-of` for the
  current node, current attributes, selected elements, and inline node-set
  variables. It also supports bounded `xsl:element` names when the name is
  literal or resolves through a scalar AVT such as `name="{$p}"`.
- Remaining open work is limited to richer XPath predicate/function semantics
  plus dynamic construction names outside that scalar AVT subset and any
  additional XSLT instructions or traversal cases that copied component/sample
  fixtures prove they need.
- XSLT dispatch (`AC-P-6.8`) is implemented as isolation/version-pinning
  decision-core; XSLT execution binding (`AC-P-6.9`) remains deferred.

Evidence from copied samples:

- Material components primarily need the fragment subset (`if`, `choose`,
  `for-each`, AVT, declaration helpers) plus the known `hasBoolAttribute()`
  gap, but Phase 4 compatibility must support the component templating set in
  full when copied components/samples rely on stylesheet invocation.
- Demo/reference files such as `demo/table.xsl`, `demo/tree.xsl`,
  `demo/template.xsl`, `demo/s.xslt`, `demo/html-template.xml`,
  `demo/xhtml-template.xhtml`, `demo/http-request.html`,
  `demo/location-element.html`, and `demo/local-storage.html` use
  `xsl:template`, `xsl:apply-templates`, and/or `xsl:call-template`.
- The executable inventory target
  `yarn nx run @epa-wg/custom-element:xslt:inventory` scans 41 copied
  material/demo files and writes
  `packages/custom-element/dist/reports/xslt-compat-inventory.{json,md}`.
  The current inventory finds these template features:
  `apply-templates`, `call-template`, `choose`, `for-each`, `if`,
  `match-patterns`, `param`, `sort`, `stylesheet`, `template`, `variable`,
  and `with-param`.
  It finds these XPath/EXSLT functions:
  `concat`, `contains`, `count`, `current`, `exsl:node-set`,
  `exslt:node-set`, `hasBoolAttribute`, `local-name`, `name`,
  `normalize-space`, `not`, `position`, `processing-instruction`,
  `starts-with`, `string-length`, `substring`, `substring-after`,
  `substring-before`, `sum`, `text`, and `translate`.

## Decisions

1. **Compatibility target.** Phase 4 requires full parity for the copied
   custom-element components and the sample-used templating behavior needed to
   validate those components. This includes the demo/reference stylesheets that
   exercise the component templating set; it is not limited to the already
   implemented fragment bridge.

2. **Version target.** Target XSLT 1.0 compatibility plus the limited EXSLT
   idioms used by the copied demos/components, especially `exsl:node-set`.
   "XSLT 1.1" was a typo and does not mean the abandoned XSLT 1.1 working-draft
   surface. This should be named as a legacy compatibility adapter profile, not
   as the future XSLT 3.0/4.0 peer-language engine.

3. **Template invocation subset.** `xsl:template`, `xsl:call-template`, and
   `xsl:apply-templates` are in scope. The first engine slice supports root and
   named templates, `param`/`with-param`, simple match-based template selection,
   `mode`, sample-style source traversal, namespace wildcard and indexed-child
   selection, parent-relative and preceding-sibling paths, variable-rooted
   current-node paths, scalar/current-name equality predicates, basic template
   priority, a multi-key sort subset with recursion safety, and bounded
   current-node copy/attribute/element construction. The remaining bounded
   subset must cover richer XPath predicate/function behavior and dynamic names
   outside the scalar AVT subset only where sample-used.

## Remaining Open Questions

1. **XPath/data model.** The engine now has an executable current-item data
   model for document, element, attribute, text, parent navigation, unions,
   wildcard matching, and mode-specific traversal. The remaining question is how
   far to extend predicates and functions beyond the sample-backed subset
   without turning the adapter into a general XPath runtime.

2. **Namespace recognition.** The engine should dispatch XSLT by resolved
   namespace identity (`http://www.w3.org/1999/XSL/Transform`), not only lexical
   tag prefix. Bare legacy aliases (`if`, `for-each`, `choose`, etc.) should stay
   scoped to `custom-element-xslt` compatibility input. Default HTML namespace
   handling and `xsl:` namespace declarations need an executable fixture that
   proves HTML output elements remain HTML while XSLT instructions dispatch to
   the XSLT adapter.

3. **On-demand loading boundary.** XSLT compatibility execution should not be
   loaded by default. Which activation points are required: explicit
   `custom-element-xslt` content type, CEM-ML AST initialization with an XSLT
   capability, inline `xmlns:xsl` namespace metadata, or all of these?

4. **Schema source.** Which schema is authoritative for validation/import:
   the historical XSLT 1.0 DTD-derived metadata already present in
   `packages/custom-element/ide/`, a new CEM-owned compatibility schema, or a
   later XSLT 4.0 RELAX-NG schema for the future peer-language engine?

5. **Output and diagnostics.** Should unsupported XSLT constructs remain
   warnings in the custom-element bridge, or become build-blocking diagnostics
   for Phase 4 component fixtures?

## Recommended Next Step

Define a separate `custom-element-xslt-compat` adapter profile for the
inventory-backed XSLT 1.0 + limited EXSLT subset. That profile must include a
bounded template invocation engine for `xsl:template`, `xsl:call-template`, and
`xsl:apply-templates`, while staying separate from the future XSLT 3.0/4.0
peer-language engine.
