# SCSS Schema Package v1

Status: schema identity, lossless native parsing, focused CEM-owned expansion,
explicit-policy module resolution and execution limits, direct typed CSS AST
handoff, exact expansion origins, manifest examples, and passive output-profile
assets implemented; full Sass parity remains a staged gap

This package owns SCSS source identity and the contract for expanding SCSS into
the lifecycle-owned typed CSS AST. It does not claim `text/css`, does not serve
raw SCSS as CSS, and does not declare an executable SCSS-to-CSS converter before
the evaluator exists.

## Owned Identities

- Schema URI: `https://cem.dev/ns/data/scss/1`
- Primary content type: `text/vnd.cem.scss`
- Compatibility alias: `text/x-scss`
- Source extension: `.scss`
- Encoding: UTF-8, with an optional charset normalized to `charset=utf-8`
- Dated compatibility reference: [Dart Sass 1.101.7](https://github.com/sass/dart-sass/releases/tag/1.101.7)

The indented `.sass` syntax is outside v1. Schema and content identity are
authoritative; source text shape and the `.scss` extension are not detection
mechanisms.

## Package Boundary

`schema/scss.cem` owns the lossless source resource, token and syntax AST,
module graph, neutral parser/evaluator facts, safety request, CSS handoff, and
origin-chain structures. SCSS bytes keep their SCSS identity through parsing
and expansion. The CEM-owned evaluator constructs `CssDocumentAst` under
`https://cem.dev/ns/data/css/1` directly. Generated CSS events retain source,
module, definition, call-site, interpolation, and expansion frames as
applicable.

Browser-facing serialization belongs to the CSS package and always uses
`text/css`. The pipeline must not call a Sass CSS serializer and then reparse
the generated text.

## Compatibility And Modules

The package-owned
[`tests/dart-sass-1.101.7-conformance.cem`](./tests/dart-sass-1.101.7-conformance.cem)
matrix is the only parity claim. Each required behavior begins as a named gap
and becomes supported only with focused parser/evaluator evidence.

[`@use`](https://sass-lang.com/documentation/at-rules/use/) and
[`@forward`](https://sass-lang.com/documentation/at-rules/forward/) are the
normative module system. Sass `@import` remains an accepted compatibility path
that emits `cem.scss.import_deprecated`, matching the
[Sass migration guidance](https://sass-lang.com/documentation/breaking-changes/import/).

## Parser And Evaluation Strategy

The implementation is CEM-native and has no Grass dependency. It uses
[`grass_compiler` 0.13.4](https://docs.rs/grass_compiler/0.13.4/grass_compiler/)
as a behavioral and algorithmic reference for a staged lexer, syntax parser,
lexical environments, definition registration, visitor-style evaluation, and
nested expansion. The algorithms are independently expressed over CEM-owned
tokens, statements, diagnostics, ranges, and origin frames; Grass source code,
AST types, and runtime values are not copied or linked.

Grass separates parsing, evaluation, and CSS serialization. CEM adopts the
first two architectural boundaries but deliberately replaces the serializer
boundary: expansion creates `CssDocumentAst` events directly. No Grass CSS
serializer and no generated-CSS reparse are in the data path. Dart Sass 1.101.7
remains the language compatibility reference, and only matrix rows marked
`supported` are conformance claims.

## Resolver And Evaluation Policy

Lifecycle expansion passes the engine's resolver registry, resolver policy, and
request-owned safety policy into the SCSS evaluator. Module reads use the shared
`input`/`read` purpose, retain requested, normalized, effective, and canonical
URIs in the module resolution audit, and never fall back to direct filesystem
or network access.
Relative resolution checks the importing module first and then only explicitly
provided load paths. The v1 safety policy rejects traversal, absolute paths,
backslashes, query/fragment suffixes, and HTTP(S) module specifiers before a
read. Candidate selection follows the Sass partial/index shape: `name.scss`,
`_name.scss`, `name/index.scss`, and `name/_index.scss`; multiple canonical
matches fail as ambiguous.

`@use` and `@forward` cache modules by canonical URI plus configuration text,
detect active-load cycles with the complete URI chain, and expose public
variables, mixins, and functions through the requested namespace. Legacy Sass
`@import` uses the same resolver and safety boundary while importing members
without a namespace and retaining its deprecation warning. Configuration value
application, forwarding prefixes/show/hide semantics, and full Dart Sass module
parity remain explicit gaps in the conformance matrix.

Every expansion request carries resolver and safety stamps, a cooperative abort
signal, explicit load paths, and four limits. Lifecycle defaults are 100,000
work units, recursion depth 64, 100,000 generated CSS events, and 16 MiB of CSS
output. Functions, mixins, control-flow iterations, expression interpolation,
generated selectors, candidate probes, and module loads consume the shared work
and recursion budget. A cancellation or limit breach returns no partial
`CssDocumentAst`.

## Diagnostics

The schema binds neutral facts to these initial stable codes:

- `cem.scss.identity_mismatch`
- `cem.scss.unsupported_encoding`
- `cem.scss.parse_error`
- `cem.scss.module_error`
- `cem.scss.import_deprecated`
- `cem.scss.resolver_denied`
- `cem.scss.module_cycle`
- `cem.scss.budget_exceeded`
- `cem.scss.cancelled`
- `cem.scss.origin_unavailable`
- `cem.scss.handoff_invalid`

Source validation, formatting, coloring, README generation, and preview
verification are passive and perform no module resolution. Lifecycle expansion
is explicit and hands the generated typed stream to the CSS package.

## Formatter And Colorizer Assets

- `formatters/compact.cemt`
- `formatters/pretty.cemt`
- `formatters/tabular.cemt`
- `formatters/scss-format-source.cemt`
- `colorizers/terminal.cemt`
- `colorizers/html.cemt`
- `colorizers/md.cemt`
- `colorizers/scss-color-source.cemt`

These profiles operate on a typed SCSS source subject for inspection and
authoring output. Once evaluation hands off `CssDocumentAst`, CSS validation,
formatting, coloring, conversion, and serialization come from `css/v1` rather
than being duplicated here.

## Verification

`yarn nx run cem_ml_schema_package_scss_v1:verify` validates the manifest,
schema source, conformance matrix, embedded catalog identities and assets,
schema-package structure, native SCSS parser/lowering contracts, CLI source
validation, manifest-owned examples, CLI dependency gates, and source-only
README preview policy.

For the CLI boundary alone, `yarn nx run cem_ml_cli:test:scss` runs the five
SCSS-specific integration tests without selecting the broad CLI unit or
schema-example suites.

The package `verify` target also selects only the SCSS library tests. Its native
integration suite covers lifecycle resolver plumbing, namespaced member use,
canonical single-load behavior, resolution audit fields, denials, cycles,
cancellation, recursion, work, output-node, output-byte, and exact-origin
contracts without running unrelated Rust tests.

## Examples

This section is generated from `package.cem` `{example}` metadata by the
`samples2readme` Nx target. Text examples with a recognized language and
valid UTF-8 are embedded directly as language-tagged fenced source.
All declared examples in this package support source fences, so README SVG
previews are not used.

<details>
<summary>basic-source</summary>

- Source: [`examples/basic.scss`](./examples/basic.scss)
- Content type: `text/vnd.cem.scss`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- README rendering: fenced `scss` source

</details>

```scss
$component: "card";
$accent: #036;

.#{$component} {
  color: $accent;

  &__title {
    font-weight: 700;
  }
}
```

<details>
<summary>tokens-partial</summary>

- Source: [`examples/_tokens.scss`](./examples/_tokens.scss)
- Content type: `text/vnd.cem.scss`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- README rendering: fenced `scss` source

</details>

```scss
$space: 0.75rem !default;

@mixin inset($amount: $space) {
  padding: $amount;
}
```

<details>
<summary>module-entry</summary>

- Source: [`examples/module-entry.scss`](./examples/module-entry.scss)
- Content type: `text/vnd.cem.scss; charset=utf-8`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- README rendering: fenced `scss` source

</details>

```scss
@use "tokens";

.card {
  @include tokens.inset();
}
```

<details>
<summary>forward-entry</summary>

- Source: [`examples/forward-entry.scss`](./examples/forward-entry.scss)
- Content type: `text/vnd.cem.scss`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- README rendering: fenced `scss` source

</details>

```scss
@forward "tokens";
```

<details>
<summary>deprecated-import</summary>

- Source: [`examples/deprecated-import.scss`](./examples/deprecated-import.scss)
- Content type: `text/vnd.cem.scss`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- Expected diagnostics: `cem.scss.import_deprecated`
- README rendering: fenced `scss` source

</details>

```scss
@import "tokens";

.legacy-card {
  @include inset();
}
```

<details>
<summary>compatibility-alias</summary>

- Source: [`examples/compatibility-alias.scss`](./examples/compatibility-alias.scss)
- Content type: `text/x-scss; charset=UTF-8`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `pass`
- README rendering: fenced `scss` source

</details>

```scss
.compatibility-alias {
  color: rebeccapurple;
}
```

<details>
<summary>invalid-indented-syntax</summary>

- Source: [`examples/invalid-indented-syntax.scss`](./examples/invalid-indented-syntax.scss)
- Content type: `text/vnd.cem.scss`
- Schema: `https://cem.dev/ns/data/scss/1`
- Expected result: `fail`
- Expected diagnostics: `cem.scss.parse_error`
- README rendering: fenced `scss` source

</details>

```scss
.card
  color: red
```
