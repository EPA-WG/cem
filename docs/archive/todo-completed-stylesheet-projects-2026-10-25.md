# CSS, SCSS and selector projects, 2026-10-25

- [x] Add the ordinary [CSS stylesheet](../../examples/projects/css-stylesheet/README.md),
  [CSS selector](../../examples/projects/css-selector/README.md) and
  [SCSS stylesheet](../../examples/projects/scss-stylesheet/README.md) projects.
  Their 18 configurations cover native source inspection/validation, exact CSS
  formatting/expansion, native document selection, deliberate failure/repair and
  HTML/CSS artifact builds where supported.
- [x] Use native CEMT wrappers to construct explicit HTML entry artifacts and
  their relative stylesheet links. Export CSS from its typed native owner;
  introduce no example-specific UI JavaScript, parser or CSS text handoff.
- [x] Add failing native typed-CSS graph export cases before reusing the existing
  CSS output pipeline. Preserve target identity checks, scope, syntax/resource
  policy, presentation-color refusal and original writer spans.
- [x] Add a failing authored-CSS URI regression before attaching original source
  boundaries to native parser events. Retain SCSS module/definition/expansion
  origins without replacing them with generated-CSS offsets.
- [x] Prove SCSS module execution with explicit native host configuration and
  retain missing-grant refusal. Do not add ambient filesystem access or an
  evaluator-local filesystem fallback.
- [x] Add authored native and shared-worker configuration, exact output, native
  identity/range, repair, malformed-build refusal and independent-copy fixtures.
- [x] Retain native `ScssOrigin` module URIs and original ranges in Studio's
  existing provenance data model, with failing projection/navigation cases
  before the repair. Keep native frame kinds, bounded rows and source guards.
- [x] Regenerate ordinary project archives and update the checked operation map
  and native CSS/SCSS documentation, including unsupported source operations.
- [x] Verify deployed catalog/read-only resources, shared editor and preview,
  saved source/module/query edits, native repair/refusal, provenance, independent
  copies and offline builds/queries without clearing saved projects or drafts.

## Project contracts

`css-stylesheet` has eight authored resources and six configurations. Its CSS
contains a layer and an explicit `@scope (.catalog-card)` boundary. Native Inspect
exposes the lossless lexical representation; tree inspection exposes retained
CSS-namespace semantics. Current formatter profiles preserve conservative lexical
boundaries rather than claiming general beautification. The exact CSS output is
241 bytes; its CEMT HTML entry is 563 bytes and links `cards.css`.

`scss-stylesheet` has ten authored resources and seven configurations. It uses
an owned `_tokens.scss` module, namespaced variables, a mixin with an explicit
argument and nested selectors. Expansion constructs a typed native CSS owner;
the graph publishes 236 bytes of CSS and a 564-byte HTML entry. Original module
and expansion origins remain distinct from the generated CSS subject. Parse
AST/events describe that expanded subject, and their displayed offsets must not
be presented as raw SCSS byte offsets.

Source validation is passive: the separate unavailable-module source validates
without reading or expanding its module. Expansion requires an admitted resolver.
Native CLI acceptance supplies a bounded project-root `--resolver-read-map` and
the explicit stylesheet `--base-uri`. A base or source file URI alone grants no
read authority. Portable commands keep relative resource references; Studio uses
its owned virtual resources. The current filesystem graph provider's URI/base
normalization remains a separate follow-up, with its explicit host invocation
documented in the project README.

The SCSS example does not claim full Sass conformance, native same-schema SCSS
source export, source DomJson/Json or command HTML-colored CSS output. A
namespaced mixin default referencing its module variable remains a native
follow-up; this example uses an explicit argument. These new project acceptance
checks use the complete default deployment. Optional deployments still require
the existing trusted schema-source acquisition contract.

`css-selector` has eight authored resources and five configurations. Selector
MIME/schema identity is explicit and separate from stylesheet CSS. Its HTML query
uses attributes, a child relation, `:has()` and an overlapping selector list;
results are deduplicated by native identity and retain document order. Its XML
query supplies a static namespace binding. Exact contracts compare real native
local names, namespace URIs, document order and node IDs. Native result spans
recover the original opening elements; results expose no invented text or
attribute fields.

Selector Inspect AST currently presents a generic text envelope, not a selector
syntax tree. Registered selector formatter assets do not establish working CLI
source export: the unsupported adapter/fallback remains an explicit native
follow-up. No source is parsed or selected by browser JavaScript as a substitute
for native query execution. JSON is only the requested human-facing result.

Both stylesheet graphs copy retained native HTML body content through CEMT and
construct the packaged relative CSS link. The source HTML itself contains no
external resource reference, retaining existing native source-policy refusal.
This is explicit graph authoring, not automatic HTML resource admission or DCE
compilation. Browser preview uses the existing shared artifact tree and CSS
resource policy. The scoped example isolates the card heading from its outside
heading; example styles and storage remain independent of Studio's theme.

## Source failures and provenance

Each project has an independent failure resource and an exact documented repair.
Missing CSS/SCSS braces and an unclosed attribute selector produce original
native diagnostics; repairing a saved copy leaves the catalog original unchanged.
Malformed stylesheet inputs cannot publish a replacement graph artifact.

Graph CSS exports reuse the context-owned CSS output pipeline and activated
package reader. They reject incompatible targets and presentation coloring;
raw SCSS cannot be relabeled as CSS. Authored CSS parser events now retain their
source URI alongside exact token byte ranges. Generated SCSS events retain their
existing original module/definition/expansion maps. Writer output spans survive
into artifact provenance rather than being replaced by one whole-document range.
The CSS project verifies all 76 writer spans against original bytes.

Deployed acceptance found that Studio's existing provenance projection retained
only `SourceBoundary` frames. It now also reads the typed `ScssOrigin.module_uri`
field, preserving the native transform kind, origin kind/name and original
ranges. This changes service data projection; the existing declarative UI and
source-navigation controls are reused. The 100-row bound and project ownership,
captured revision, stale-result, draft-transition and UTF-8 selection guards
remain intact.

Expected HTML fixtures preserve the native writer's final three spaces, with
narrow Git whitespace exceptions for those two files only. No expected result is
trimmed or manufactured to hide a source/export limitation.

## Verification

The focused native cohort passes **88 tests**: 56 integration cases across the
new graph exporter, CSS import, folder builds, SCSS and JSON-boundary audit;
six adjacent inline/link/omit export cases; 14 CSS parser cases; nine SCSS CLI
cases; and three authored-project CLI cases.

The two initial typed-export cases fail before the repair. The original URI case
also fails before source-boundary attachment. Final coverage retains missing
resolver authority, unsafe module/specifier/resource refusals, incompatible
target and presentation-color refusal, exact CSS bytes and original ranges.
Authored CLI checks run every project configuration, exact source repairs,
original match identities/ranges, saved source/module edits, independent builds
and failure before publication.

All **34 Studio Feature Tour browser tests pass** against the rebuilt native
command-service and rendering WASM packages. The new five cases exercise every
stylesheet/selector configuration and both graphs through the shared worker,
including owned virtual SCSS modules, exact artifacts, original provenance,
source/module edits and malformed-build refusal. Existing sample cases pass.
Studio and shared-runtime typechecks pass using the exact native-generated CLI
schema input; the temporary input is removed afterward. Its existing standalone
preparation issue remains a separate TODO. Authored Studio lint and whitespace
checks pass; the previously documented generated-WASM package lint scope issue
is unchanged.

The new provenance cohort passes **nine browser cases** after all nine fail
before the projection repair. A real authored SCSS worker build with a Unicode
module prefix proves original module ranges, output bounds and captured
revision/hash metadata. The remaining cases exercise normalized native source
maps, renamed stable addresses, draft decisions, foreign/stale resources and
invalid UTF-8 selections. Focused authored lint passes with no warnings.
All **eight existing provenance compatibility cases also pass**, including the
original SourceBoundary, guarded draft and native site metadata paths.

Native Feature Tour verification passes all **32 schema examples** and the
authored configurations; the checked inventory now contains **87 workflow
mappings**. Output verification passes 218 declared assets and 322 files.
Two deterministic builds match SHA-256
`4bed5be8529fd67a4469e47dd08af49c1a6fe2de2c0488b7a02bdec911e922bf`.

The dedicated `@epa-wg/cem-studio:verify:stylesheets` gate passes with all 25
prerequisites. All **42 deployed evidence flags** pass (14 for each project).
CSS preserves outside-scope heading styles; saved SCSS module edits change the
visible color and padding and retain original module provenance. Failed builds
keep the last good artifacts. Selector edits preserve native ordering and
deduplication; invalid queries refuse publication and succeed again after repair.
Independent copies reopen and rebuild/rerun offline, with original catalog
resources and intentional failure samples unchanged.

## Preserved policies and next work

Acceptance uses isolated storage and temporary project copies, preserving user
saved projects and drafts. Generated screenshots, Vitest attachments, build
reports and binaries remain outside Git.

Entered passwords and one-time codes still block recovery until explicitly
cleared. Automatic whole-Studio recovery stays disabled. Studio UI composition,
theme tokens and component registration authority are unchanged.

Continue with CEM-QL/XPath data workflows, CEM/XSLT transformations, then
schema-authoring projects. Keep the documented native source-export and resolver
identity follow-ups separate from those functional sample milestones.
