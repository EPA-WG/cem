# Legacy material components — backward-compat reference

These are the legacy `@epa-wg/custom-element` **material suite** components (`components/*.html`),
copied verbatim from the POC (`~/aWork/custom-element-dist/src/material/components`). They are
authored as declarative **HTML + XSLT** (bare `<attribute>` / `<if>` / `<choose>` / `<when>`, `{$x}` /
`{//path}` interpolation, XPath functions such as `contains()`).

## How they render in 0.1.0

The native browser XSLT transform engine is **retired** — these samples are **not** run by a browser
XSLT processor. Instead, the `<custom-element>` adapter parses each template's DOM (HTML + XSLT
namespaces) and **transpiles it to canonical CEM-ML**, which renders on the cem_ql WASM engine — the
same engine hand-authored `type="cem-ml; version=0.0"` templates use. A legacy sample and its migrated
CEM-ML twin therefore render **identically**.

The sample markup (the `<custom-element>` declarations) is preserved **as-is**; only the
`custom-element` implementation they import changed (it now routes through the converter). See:

- Converter: [`../../cem-elements/src/lib/legacy-xslt/convert.ts`](../../cem-elements/src/lib/legacy-xslt/convert.ts)
- Engine contract/lowering: [`../../cem_ml/src/legacy_custom_element.rs`](../../cem_ml/src/legacy_custom_element.rs)
- Adapter routing: [`../custom-element.js`](../custom-element.js) (untyped templates → `legacy-xslt` mode)
- Tested parity (legacy ⇆ CEM-ML identical DOM):
  [`../../cem-elements/src/lib/legacy-xslt-parity.stories.ts`](../../cem-elements/src/lib/legacy-xslt-parity.stories.ts)
- Background: [`../README.md` §"XSLT 1.0"](../README.md), `docs/release-readiness-0.1.0.md` §4.

## Running the pages

These HTML files keep the POC's sibling-layout script/CSS paths (`../../custom-element/…`,
`../angular.css`, `../theme/…`) and the material CSS/theme tree, which is **not** copied here. The
**runnable, CI-tested** representation of each component's behavior is the twin Storybook stories
above; the raw pages are kept as authoring reference.

## Scope

Supported (Tier 1/2): the constructs the material components and demos use — see the converter doc.
A CI gate (`@epa-wg/custom-element:test`, `test-fixtures/material-convert-gate.js`) loads
`test-fixtures/legacy-compat-manifest.json`, converts every `<template>` in these files, requires the
manifest-listed primary component templates to produce non-empty CEM-ML, and fails on diagnostics that
are not explicitly allowlisted per component. The Rust CEM-ML engine gate reads the same manifest and
material files for the bounded `cem_ml::legacy_custom_element` lowering path. **Known deferred gap
(allowed only for `cem-input`):** the
legacy DCE `hasBoolAttribute()` boolean-attribute helper is not reproduced on the substrate.
**Not converted (Tier 3, deferred):** standalone full XSLT stylesheets (push-model
`<xsl:template match>` + `apply-templates`/`call-template`/`sort`, EXSLT `func:function`,
`<msxsl:script>`); these emit a conversion diagnostic. The original POC `xslt-*` test stories
(`~/aWork/custom-element-dist/src/stories`) are ported into the substrate twin stories rather than
copied verbatim (they were bound to the POC's runtime and project layout).
