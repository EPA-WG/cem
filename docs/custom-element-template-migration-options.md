# `@epa-wg/custom-element` Template Migration Options

This records the Phase 3.6 template migration choice for legacy
`@epa-wg/custom-element` templates, especially the `cem-theme` CSS generators.

## Decision Frame

There are two valid migration options. They must be chosen deliberately per
workflow; do not mix them by accident inside the default adapter.

## Option A: Keep Legacy XSLT+XPath

Keep the legacy template model:

- HTML template content remains the authoring surface;
- XSLT elements and XPath expressions keep the legacy default-namespace behavior;
- `<variable>`, `<for-each>`, `<choose>`, `<when>`, `<otherwise>`, XPath selection,
  and text helpers continue to execute with legacy semantics.

This option preserves the current CSS generators with the least authoring churn, but
it keeps a browser XSLT/XPath runtime in the migration stack. If selected, it must be
explicit:

- implemented as a named legacy runtime path, not hidden inside the substrate
  adapter;
- covered by fixtures that prove default-namespace HTML+XSLT behavior;
- excluded from Edge/SSR/export assumptions unless a separate serializable
  processing boundary is designed;
- documented as a migration/runtime compatibility path, not the target substrate
  model.

## Option B: Convert To CEM-ML+CEM-QL

Convert the legacy XSLT/XPath logic to CEM-ML templates and CEM-QL expressions.

For migration fixtures, use the reference marker:

```html
<template type="cem-ml; version=0.0">
    ...
</template>
```

`type="cem-ml; version=0.0"` is a migration marker for converted legacy templates. It should
mean: "this template is no longer XSLT/XPath, but it may still use migration-era
CEM-ML/CEM-QL compatibility affordances while generator templates are converted."

This is the planned path for `cem-theme` CSS generation after conversion:

- CSS generator templates are converted from `<variable>`/`<for-each>`/XPath to
  CEM-ML+CEM-QL;
- `capture-xpath-text.mjs` continues extracting `code[data-generated-css]` from
  rendered HTML;
- `@epa-wg/cem-theme:verify:phase13` becomes the acceptance gate after the converted
  templates emit non-empty CSS and manifest validation passes.

## Recommendation

Use Option B for `cem-theme` CSS generation.

Reasoning:

- the CSS generators are build workflows, so conversion can be fixture-driven and
  reviewed generator-by-generator;
- the converted output becomes aligned with the `cem-element` substrate and future
  Edge/SSR processing model;
- the package avoids carrying an implicit browser-only XSLT engine in the default
  publish adapter;
- the resulting fixtures exercise the target CEM-ML/CEM-QL path that future
  generators and docs should use.

Keep Option A available only as an explicit fallback if conversion of a generator is
blocked by missing CEM-QL/CEM-ML loop, binding, namespace, or XPath-parity features.
If Option A is chosen, update the package verifier and docs to name that legacy
runtime explicitly instead of treating XSLT as a regression.

## Legacy `custom-element` Compatibility Recommendation

For the copied `custom-element` demo, story, and material component modules, use a
hybrid of Option A's authoring compatibility and Option B's execution model:

- accept the old declarative HTML+XSLT syntax as input during the bridge window;
- lower the supported subset to canonical CEM-ML and `cem_ql` expressions;
- run the resulting template through the same CEM-ML render engine used by
  migrated templates;
- keep the compatibility subset narrow and fixture-derived.

The browser package does this in the `cem-elements` TypeScript converter, and
`cem_ml::legacy_custom_element` now provides the first bounded engine-side
lowering path for raw legacy fragments. That is the right ownership direction,
but the migration is not complete until the browser runtime, CLI validation,
SSR, Storybook, and the published adapter all compile the same legacy source
with the same diagnostics.

Compatibility tiers:

| Tier | Scope | Disposition |
| --- | --- | --- |
| 1 | Material component subset: declarations, AVT, `if`, `choose`, `for-each` over inline node-set variables, slots, resource slices, and the XPath functions used by those files | Supported by conversion to CEM-ML |
| 2 | Focused demos that use the same pull-style subset plus simple XPath string/sequence helpers | Supported only when covered by an executable fixture |
| 3 | `sort`, EXSLT functions beyond the inventory-backed node-set idiom, script extensions, broad DOM axes, and template invocation outside the bounded compatibility profile | Explicit handoff/deferred, not silently supported |

The adapter must remain a facade. It may normalize legacy declarations, but it
must not own a private XPath evaluator or XSLT runtime.

Phase 4 component work requires a bounded XSLT 1.0 + limited sample-used EXSLT
compatibility adapter for copied component/sample templating, including
`xsl:template`, `xsl:apply-templates`, and `xsl:call-template`. The first
engine slices support root and named templates, params, bounded
`apply-templates` over inline `exsl:node-set($var)/*` variables, sample-style
source child/attribute/text traversal, absolute/descendant selectors,
namespace wildcards, indexed child steps, parent-relative and
preceding-sibling paths, current attribute/child `for-each` unions,
variable-rooted current-node paths, static EXSLT node-set variable aliases,
filtered static node-set attribute extraction, and simple predicates including
scalar/current-name equality checks, static `if`/`when` folding for known
current-node tests, default template fallbacks, basic template priority,
multi-key `sort`, literal `count`/`sum` over supported node selections, bounded
current-node copy/copy-of/attribute construction, scalar-AVT `xsl:element`
construction, and recursion safety.
Remaining richer XPath predicate/function work and dynamic names outside that
scalar AVT subset are tracked in
[`custom-element-xslt-parity-decision.md`](custom-element-xslt-parity-decision.md).

## Immediate Plan

1. Inventory each `packages/cem-theme/src/lib/css-generators/*.html` template for
   legacy constructs: `<variable>`, `<for-each>`, conditionals, XPath selections,
   namespace-sensitive HTML/XSLT behavior, and generated CSS output.
2. Define the `type="cem-ml; version=0.0"` migration semantics required by the CSS generators.
3. Add one converted generator fixture first, preferably the smallest generator with
   table-driven CSS output.
4. Extend the browser runtime only where the converted fixture proves a missing
   CEM-ML/CEM-QL capability.
5. Convert the remaining CSS generators.
6. Rerun `@epa-wg/cem-theme:verify:phase13`; close the migrated-package gate only
   when CSS capture is non-empty and manifest validation passes.
