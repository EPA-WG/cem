# Native CSS-selector source export and shared editor paint

- [x] Reproduce the missing selector export with native lexical-source fixtures.
- [x] Connect original typed selector/token owners to registered formatter and colorizer assets.
- [x] Preserve native token origins and enforce CLI identity, namespace, capability and error boundaries.
- [x] Add native selector paint and shared worker/fallback source-editor coverage.
- [x] Verify ordinary Studio format/color configurations, saved copies and offline runs.

## Native source contract

The selector lifecycle now selects its own export adapter. It remains distinct
from the stylesheet adapter even though both use the existing CSS layer-format
enum. Selector-to-stylesheet and stylesheet-to-selector coercions refuse; typed
selector input no longer falls into generic CEM parsing or a compatibility JSON
projection during source export.

The conversion registry admits the existing selector package's output assets.
A borrowed `CssSelectorExpressionCemtSubjectRef` exposes original token strings,
static identity, native ranges and maps directly to CEMT. The formatter produces
the common typed materialized token tree, and the colorizer retains that native
presentation owner. No source AST is serialized or reconstructed through JSON.
The formatter's ordinal metadata uses the writer's existing `index` field.
The registered selector identity uses text syntax at the writer boundary rather
than an opaque target or a stylesheet grammar.

The `compact`, `pretty` and `tabular` profiles preserve lexical spelling,
comments, escapes and internal whitespace. They do not reflow selectors.
Plain/terminal text follows the existing final-newline rule, including CRLF
when present. An already terminated source receives no additional newline.
HTML source presentation retains the displayed source verbatim and escapes
markup in semantic spans. It is an HTML fragment rather than an invented
`pre` wrapper; publication and writer metadata identify it as HTML.

Native tests check Unicode, CRLF, comments, escaped identifiers, every profile,
terminal/HTML/Markdown coloring, original token origins and output bounds.
CLI tests also cover malformed UTF-8, invalid syntax, unbound namespaces,
unsupported pseudo-elements, missing host capabilities and unrelated target
identities. These failures retain hard diagnostics, original locations and
existing destination bytes. Formatting performs no matching or resource fetch.
Existing Parse/Inspect source-owner limits remain separate follow-ups.

## Shared editor and Studio

The existing native source-paint API recognizes
`application/vnd.cem.query-expression+css-selector` and consumes its real
selector token owner. It shares lexical role classification with native CSS,
without treating a selector as a stylesheet. Empty static context grants no
namespace or host capability. Incomplete source remains paintable while native
syntax, namespace and capability diagnostics remain available to validation.

The existing worker/fallback transport and `cem-source-editor` capability
require no application JavaScript changes. Browser checks preserve source text,
native editing markup, UTF-16 selection, readonly/sample identity and durable
drafts. The source-editor gallery includes a selector query, and public runtime,
component and Studio documentation list the new MIME.

The ordinary CSS-selector example adds `format-selector` and `color-selector`
configurations. It reuses the shared source editor and command controls; no
dedicated sample screen is introduced. Existing saved projects and their
configuration identities remain independent of catalog updates.

## Verification

- Focused native selector cohort: **11 passed**.
- CLI source-export cohort: **5 passed**.
- Selector matching integration: **10 passed**; package structure and declared
  validation examples pass.
- Native catalog: all **242 configuration receipts** pass their declared
  contracts: 178 clean commands, 29 verified Inspect outputs, 33 intentional
  diagnostic cases and two documented Parse limits. This is not a uniform
  source-conformance claim.
- Shared editor `LosslessColoring`: passed with real worker and fallback modes.
- Studio selector command and readonly/draft-paint cases: **2 passed**.
- Deployed CSS, SCSS and CSS-selector project acceptance: passed. Selector runs
  verify exact formatted source and semantic HTML coloring, saved document/query
  edits, invalid-query refusal, independent copies and offline format/color runs.
  Read-only catalog originals remain unchanged.
- Credential refusal: passed on worker/fallback; entered passwords and one-time
  codes continue to block recovery. Automatic whole-Studio recovery stays off.
- Full Studio build and 21 dependencies, Studio type checking, 23 catalog
  contract tests, catalog audit, source-only README preview policy and declarative
  architecture checks pass. The catalog has 205 authored configurations and 210
  documented workflows across 22 authored projects and 32 registered packages.

The complete selector-package verifier still fails the existing
`query_fixture_runs_all_languages_with_the_same_native_nodes_and_report_shape`
assertion at `query_cli.rs:390`: CSS-selector results have ContentTypeTransform
frames while CEM-QL also retains a SourceBoundary URI frame. The broader query
cohort passes 18 of 19 cases. The same unchanged assertion fails on a clean
`5ca43f31` snapshot with non-incremental compilation; a legacy selector-convert
probe confirms that binary predates the new export path. Original assertions,
URI frames and ranges remain intact, and the provenance repair stays active in
the TODO list. The current CLI binary was restored byte-for-byte afterward.

The scoped browser deployment uses catalog identity
`92c7e61bd790eb407a8b1bd88bdcd23fc361a396ca61442752bed783598bd084`
and deployed SHA-256
`270742a9d623ee928a3b4d44b1c5e3de830992f55a8fadad9457ae3c2df5caa3`.
The scoped browser gate passed with the same catalog and deployment bytes.
Tests use isolated browser profiles and temporary projects. Generated screenshots,
Vitest attachments, reports, native binaries and WASM outputs remain outside Git.
