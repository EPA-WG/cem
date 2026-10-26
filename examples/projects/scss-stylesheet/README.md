# SCSS stylesheet

Open **Examples → Content types and schemas → Documents and UI → SCSS
stylesheet**, or find the same ordinary project under **Queries and
transformations**. Clone it to edit a saved copy. This project demonstrates a
small stylesheet build with native SCSS variables, a project-owned module, a
namespaced mixin and nested selectors, followed by a shared HTML/CSS preview.

## Authored identity reference

These are resource identities, including expected artifacts. The configurations
below determine their supported uses. An identity alone does not grant an
operation.

| Resource role | Content type | Schema |
| --- | --- | --- |
| Transformation graphs | `application/cem` | `https://cem.dev/ns/cli/transform-config/1` |
| CEM-native templates | `application/vnd.cem.template+cem` | `https://cem.dev/ns/template/cem-native/1` |
| Stylesheets and expected CSS artifacts | `text/css` | `https://cem.dev/ns/data/css/1` |
| HTML source and expected artifacts | `text/html` | `https://cem.dev/ns/data/html/1` |
| SCSS sources and modules | `text/vnd.cem.scss` | `https://cem.dev/ns/data/scss/1` |

## Sources and identities

- `styles/cards.scss` declares `text/vnd.cem.scss` and schema
  `https://cem.dev/ns/data/scss/1`. The registered `text/x-scss` alias selects the
  same family; `.scss` alone does not select it.
- `styles/_tokens.scss` provides the `tokens` module's `$accent`, `$space` and
  `inset($amount)` mixin. The caller supplies `tokens.$space` explicitly.
- `data/card.html` is a separate validated HTML source with a heading and card.
  It contains no external resource reference. `templates/page.cemt` constructs
  the output wrapper and its relative link to `cards.css`.
- `graphs/styles.cem` owns the two output branches: native typed CSS generated
  from SCSS, and HTML rendered through the CEMT wrapper.

The main source uses `@use "tokens"`, `tokens.$accent`,
`@include tokens.inset(tokens.$space)`, `&__title` and a nested paragraph rule.
The output contains ordinary `.demo-card`, `.demo-card__title` and
`.demo-card p` selectors. Raw SCSS is never served as CSS.

## Configurations

| Configuration | Typical use | Expected result |
| --- | --- | --- |
| `validate-scss` — Validate passive SCSS source | Check grammar before executing a build | Zero errors and warnings, without module reads |
| `validate-unresolved-module` — Validate syntax without loading a module | Distinguish syntax validation from execution readiness | A valid `@use "not-published"` source still validates passively |
| `parse-scss-ast` — Parse expanded CSS AST with SCSS origins | Inspect the native expansion subject | CSS AST with the original SCSS source identity |
| `parse-scss-events` — Parse expanded CSS events with SCSS origins | Inspect the generated typed CSS stream | CSS events with the retained SCSS source identity |
| `expand-css` — Expand SCSS to exact browser CSS | Compile the owned variables, mixin and nested rules | Exact `expected/cards.css` bytes |
| `repair-scss-syntax` — Repair the SCSS opening brace | Diagnose and repair a saved syntax failure | One `cem.scss.parse_error`, then zero errors after repair |
| `build-html` — Build HTML and generated CSS | Publish a styled preview with an explicit entry artifact | `generated/index.html` and `generated/cards.css` |

Parse and conversion perform expansion and may resolve admitted modules. Passive
source validation does neither. The Parse AST/events describe the resulting
**CSS** subject, not a raw SCSS syntax tree or a lossless SCSS source export.
Their displayed token offsets index that expanded CSS subject; they must not be
read as original SCSS byte positions. Printed native SCSS origin frames are
inspection data; common Parse publication currently supplies no owned
artifact-source references. The build's separately published output provenance
is the source-navigation contract where provided.

## Edit, save and build

The module defines `inset($amount: $space)`, and `cards.scss` calls
`@include tokens.inset()` without an argument. The omitted amount binds the
module's `$space`; a same-named caller variable does not replace it. Explicit
arguments still evaluate in caller scope, and later defaults can use earlier
bound parameters. The CSS, module origins and definition/call-site ranges use
the shared native SCSS evaluator.

In `styles/_tokens.scss`, change `$accent: #036;` to `$accent: #630;` or change
`$space: 0.75rem;` to `$space: 1rem;`. Save, rerun **Expand SCSS to exact browser
CSS**, and choose **Build project**. The CSS output and visible card change.
The title retains its nested `font-weight: 700`; the paragraph rule remains a
selector scoped to descendants of `.demo-card`. This is ordinary selector
nesting, not an assertion that SCSS implements CEM's component scope rewriting.

`generated/index.html` is the explicit project entry. Its relative `cards.css`
reference resolves only against the published artifact tree. The shared preview
loads the generated CSS through its existing native CSS/resource policy. It
introduces no authored script, source parser, module fetcher or page-specific UI
behavior. The example's stylesheet is independent of Studio's theme and keys.

Unsaved drafts leave persisted execution inputs unchanged. An invalid source
cannot publish a new build; Studio retains the last successful output. A cloned
project owns independent source, module, template and configuration revisions.
The read-only catalog original remains unchanged.

## Failure and repair

Run **Repair the SCSS opening brace** on `failures/missing-brace.scss`. The source
omits the `{` after `.demo-card`. The native parser reports one
`cem.scss.parse_error` at line 20, column 1, original UTF-8 byte offset 273: the
remaining closing brace is unmatched. Replace `.demo-card` followed by a newline
with `.demo-card {` followed by a newline, save and revalidate. The repaired
resource then matches `styles/cards.scss` exactly. It remains separate from the
normal build inputs until deliberately used as one.

## Resolver, output and compatibility limits

Native command callers must explicitly admit the project-owned module bytes.
The CLI does not grant ambient filesystem or network module access. For example,
replace the paths below with the chosen project root; the read map grants only
that directory tree:

```sh
cem-ml convert file:///absolute/project/styles/cards.scss \
  --resolver-read-map file:///absolute/project/=/absolute/project \
  --content-type text/vnd.cem.scss --schema https://cem.dev/ns/data/scss/1 \
  --to-content-type text/css --to-schema https://cem.dev/ns/data/css/1 \
  --cemt-formatter-profile tabular --output-color-type none
```

For a native CLI graph build, also set the explicit module load-path base:

```sh
cem-ml transform --config file:///absolute/project/graphs/styles.cem \
  --resolver-read-map file:///absolute/project/=/absolute/project \
  --base-uri file:///absolute/project/styles/
```

The current CLI graph provider exposes imported local sources as filesystem
paths, so this base supplies their module-resolution context. The base itself
grants no file reads; the separate bounded read map remains required. Portable
project commands keep relative resource references, and Studio's owned virtual
source URIs supply their own module context.

Studio supplies the owned virtual resources through its existing resolver. A
file URI, MIME identity or schema identity alone grants no read capability.
`@use` candidate resolution is bounded, rejects unsafe specifiers and ambiguous
modules, and uses the engine's work, recursion and output limits. The sample uses
no external packages or remote resources.

The CEM-owned evaluator produces `CssDocumentAst` directly; the CSS package owns
browser serialization. No generated CSS text is reparsed to recreate that AST.
This project exercises a supported subset, not full Dart Sass parity. Indented
`.sass`, arbitrary Sass configuration/forwarding behavior and full language or
lexical-scope compatibility are not claimed. Module-variable and earlier-parameter
mixin defaults are covered here; function-default binding, captured callable
environments, full local-variable isolation and content-block scoping remain
outside this profile. See the native
[SCSS package](../../../packages/cem_ml/schema-packages/scss/v1/README.md).

Same-schema SCSS source export, source DomJson/Json and command HTML-colored CSS
export are unavailable. The registered passive source formatter/colorizer assets
do not establish those command operations. Generated browser CSS is uncolored
`text/css`, distinct from a syntax-colored presentation artifact. The optional
schema-asset profile must acquire and admit the retained SCSS source contract
before dependent operations; catalog registration alone does not activate it.

After the runtime and project resources are cached, saved copies can reopen and
rebuild offline using their own module bytes. No example storage key, external
font or credential belongs to this project. Entered passwords and one-time codes
continue to block recovery until explicitly cleared, and automatic whole-Studio
recovery remains disabled.
