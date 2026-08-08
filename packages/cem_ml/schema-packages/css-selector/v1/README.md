# CSS Selector Schema Package

Status: schema-owned query contract, package assets, native parser, lifecycle
element-tree adapter, and evaluator implemented;
CLI query-source loading and query execution remain planned

This package owns standalone CSS selector query expressions. It is deliberately
separate from the `css/v1` stylesheet package and never treats `text/css` or a
`.css` stylesheet as a selector query.

## Owned Identities

- Schema URI: `https://cem.dev/ns/query/css-selector/1`
- Primary content type: `application/vnd.cem.query-expression+css-selector`
- Preferred extension: `.css-selector`
- Language version: `selectors-4-20260122`
- Selector baseline: [Selectors Level 4, 22 January 2026 Working Draft](https://www.w3.org/TR/2026/WD-selectors-4-20260122/)
- Tokenization baseline: [CSS Syntax Level 3](https://www.w3.org/TR/css-syntax-3/)

The extension is an authoring convention only. Schema/content identity is
authoritative. The IANA-registered `text/css` identity remains scoped to
stylesheets and owned by `schema-packages/css/v1`.

## Package Boundary

`schema/css-selector.cem` declaratively owns the typed selector resource,
lossless token stream, selector list, compound/simple selector, combinator,
namespace/static-context, request/result, native matched-node, source-map, fact,
diagnostic, and conformance structures. Native code emits neutral facts and
implements primitive parsing and matching, but it does not own severity or
package-specific validation policy.

The package declares `compact`, `pretty`, and `tabular` formatters plus
`terminal`, `html`, and `md` colorizers. Their CEMT assets are registered now as
deterministic lexical token-tree stages. Runtime invocation waits for the CLI
slice to route the package-owned selector AST/token subject; no private source
reparse or stylesheet AST substitution is permitted.

The package-owned
[`tests/selectors-4-conformance.cem`](./tests/selectors-4-conformance.cem)
matrix distinguishes required initial features, staged pseudo-classes,
capability-gated host state, and stable unsupported features. Parser acceptance
must not move ahead of this matrix.

## Native Query Semantics

The native evaluator consumes only a borrowed lifecycle-owned element-tree view.
Selector-list results eliminate duplicates by native identity and return nodes
in lifecycle document order. Explicit host namespace bindings define prefixes;
unbound prefixes fail. The evaluator must retain input/query owners, result item
types, exact query ranges, input source maps, resolver and safety policy,
cancellation, and work/result budgets.

JSON projection, browser DOM construction, generic DTO conversion, CSS
stylesheet parsing, source reparsing, and inferred replacement trees are not
compatibility mechanisms. Unsupported input views fail before evaluation.

## Diagnostics

The schema binds neutral facts to these stable codes:

- `css-selector.lexical.invalid`
- `css-selector.parse.invalid`
- `css-selector.namespace.unbound`
- `css-selector.feature.unsupported`
- `css-selector.capability.missing`
- `css-selector.budget.exceeded`
- `css-selector.input.unsupported`

Validation, formatting, coloring, README generation, and preview verification
are passive and perform no resolution or evaluation.

## Formatter And Colorizer Assets

- `formatters/compact.cemt`
- `formatters/pretty.cemt`
- `formatters/tabular.cemt`
- `formatters/css-selector-format-expression.cemt`
- `colorizers/terminal.cemt`
- `colorizers/html.cemt`
- `colorizers/md.cemt`
- `colorizers/css-selector-color-expression.cemt`

Source snapshots are used only where the current CLI cannot yet render the
package formatter/colorizer path. The checked-in selector examples use fenced
source, so they require no SVG fallback previews.

## Safety

Parsing must be deterministic, passive, and I/O-free. Relational selectors such
as `:has()` consume explicit traversal work budgets. UI, browsing, resource,
link-history, shadow-tree, and host-state pseudo-classes require advertised host
capabilities and cannot silently return false. Pseudo-elements, unknown
extensions, and the at-risk column combinator remain stable unsupported facts in
the initial slice.

## Verification

`yarn nx run cem_ml_schema_package_css_selector_v1:verify` validates
`package.cem`, checks the embedded package/catalog/profile contracts, runs the
schema-package structure gate and focused lossless syntax, schema diagnostic,
lifecycle loading, native matching, budget, and result-identity tests, and
verifies the README source-only preview policy.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-selector</summary>

- Source: [`examples/basic.css-selector`](./examples/basic.css-selector)
- Content type: `application/vnd.cem.query-expression+css-selector`
- Schema: `https://cem.dev/ns/query/css-selector/1`
- Expected result: `pass`
- README rendering: fenced `css` source

</details>

```css
main#app > article.card[data-state="ready"]
```

<details>
<summary>relational-selector</summary>

- Source: [`examples/relational.css-selector`](./examples/relational.css-selector)
- Content type: `application/vnd.cem.query-expression+css-selector`
- Schema: `https://cem.dev/ns/query/css-selector/1`
- Expected result: `pass`
- README rendering: fenced `css` source

</details>

```css
section:has(> h2):not([hidden])
```

<details>
<summary>unbound-namespace</summary>

- Source: [`examples/unbound-namespace.css-selector`](./examples/unbound-namespace.css-selector)
- Content type: `application/vnd.cem.query-expression+css-selector`
- Schema: `https://cem.dev/ns/query/css-selector/1`
- Expected result: `fail`
- Expected diagnostics: `css-selector.namespace.unbound`
- README rendering: fenced `css` source

</details>

```css
svg|svg > svg|a[href]
```
